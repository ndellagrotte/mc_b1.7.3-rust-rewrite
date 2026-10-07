//! The scripted-input dev mode (project-owned, not vanilla): `--script
//! file.json` drives the game from a list of timed actions instead of the
//! keyboard and mouse, takes screenshots and writes state dumps, so later
//! work packages can smoke-test without a person at the window. The format
//! is documented in `crates/mc-client/README.md`.
//!
//! While a script runs, real input devices are ignored ([`DeviceInput`]),
//! the window counts as focused (`Display.isActive()`) and the desktop
//! pointer is never grabbed. Input reaches the game through the same
//! LWJGL-shaped buffers ([`crate::input::InputQueue`]) as real input, so it
//! takes vanilla's paths (key events in `runTick`, mouse motion in
//! `updateCameraAndRender`).
//!
//! Times: an action may carry `"tick": n` (not before `ticksRan` has grown
//! by `n` since the script started) and/or `"frame": n` (not before the
//! `n`-th frame); `wait` waits relatively. Actions run in order; a
//! `screenshot` holds the script until the PNG is written.

#![allow(clippy::disallowed_methods)]

use std::path::PathBuf;

use bevy::prelude::*;
use serde::Deserialize;

use crate::game::Game;
use crate::input::DeviceInput;
use crate::keys;
use crate::screenshot::{PendingScreenshots, take_screenshot};

/// A mouse button in a script.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScriptButton {
    /// LWJGL button 0.
    Left,
    /// LWJGL button 1.
    Right,
    /// LWJGL button 2.
    Middle,
}

impl ScriptButton {
    fn lwjgl(self) -> i32 {
        match self {
            ScriptButton::Left => 0,
            ScriptButton::Right => 1,
            ScriptButton::Middle => 2,
        }
    }
}

/// One action (module docs; README for the format).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    /// A key press: an LWJGL key name (`"W"`, `"SPACE"`, `"LSHIFT"`,
    /// `"ESCAPE"`, `"F3"`) or a binding (`"key.forward"`, `"key.jump"`, …,
    /// resolved through the current options).
    KeyDown {
        /// The key.
        key: String,
    },
    /// A key release.
    KeyUp {
        /// The key.
        key: String,
    },
    /// Mouse motion in pixels, LWJGL's sense (`dy` > 0 is up).
    MouseMove {
        /// Right.
        dx: f32,
        /// Up.
        dy: f32,
    },
    /// A button press.
    MouseDown {
        /// The button.
        button: ScriptButton,
    },
    /// A button release.
    MouseUp {
        /// The button.
        button: ScriptButton,
    },
    /// Wheel notches (positive up).
    Wheel {
        /// Notches.
        notches: i32,
    },
    /// Sets the player's yaw and pitch (and their previous values, so there
    /// is no interpolation): an absolute, reproducible look.
    Look {
        /// Degrees; 0 faces south (+z), 90 west.
        yaw: f32,
        /// Degrees; positive looks down.
        pitch: f32,
    },
    /// Waits `ticks` game ticks and/or `frames` frames, and/or until a
    /// world is open and loaded (`in_game: true`).
    Wait {
        /// Game ticks (`ticksRan`).
        #[serde(default)]
        ticks: Option<u32>,
        /// Frames.
        #[serde(default)]
        frames: Option<u32>,
        /// Until a world is loaded.
        #[serde(default)]
        in_game: bool,
    },
    /// Captures the window into a PNG at `path`.
    Screenshot {
        /// The file.
        path: PathBuf,
    },
    /// Writes a JSON of the game state to `path` ([`dump_json`]).
    Dump {
        /// The file.
        path: PathBuf,
    },
    /// Saves the world (`saveWorldIndirectly`).
    Save,
    /// Dev: `Minecraft.usePortal()` at once (nether travel without walking
    /// into a portal).
    UsePortal,
    /// Dev: `Minecraft.respawn(false, 0)`.
    Respawn,
    /// `Minecraft.shutdown()`: leaves the world (saving it and the player)
    /// and exits.
    Quit,
    /// Dev (WP-3.2): writes a texture's current pixels, FX tiles included,
    /// to a PNG ([`crate::textures::dump_texture`]).
    DumpTexture {
        /// `"terrain"`, `"items"` or a jar path such as `"/gui/gui.png"`.
        name: String,
        /// The PNG.
        path: PathBuf,
    },
    /// Dev (WP-3.4): sets `worldInfo.worldTime` (`World.setWorldTime`), so
    /// the sky can be shown at any hour (0 sunrise, 6000 noon, 12000
    /// sunset, 18000 midnight).
    SetTime {
        /// The new world time in ticks.
        world_time: i64,
    },
    /// Dev (WP-3.4): sets the weather: `rainingStrength` (and its previous
    /// value) to `strength`, `worldInfo.raining` to `strength > 0` and
    /// `thundering` (strength 1 or 0) with long timers, so `updateWeather`
    /// keeps it (it still moves the strength 0.01 a tick toward 1 or 0).
    SetRain {
        /// 0..1.
        strength: f32,
        /// Thunder as well.
        #[serde(default)]
        thunder: bool,
    },
    /// Dev (WP-3.4): moves the player to `x, y, z` (`posY` is the eye
    /// level, as the dump prints it), with no interpolation and no motion.
    SetPos {
        /// x.
        x: f64,
        /// y (`posY`).
        y: f64,
        /// z.
        z: f64,
    },
    /// Dev (WP-3.8): queues `world.playAuxSFX(kind, x, y, z, data)` as the
    /// simulation would (e.g. kind 1005 with data 2256 plays record "13" at
    /// the block; data 0 stops it).
    AuxSfx {
        /// The `playAuxSFX` type.
        kind: i32,
        /// Block.
        x: i32,
        /// Block.
        y: i32,
        /// Block.
        z: i32,
        /// The data argument.
        #[serde(default)]
        data: i32,
    },
    /// Dev (WP-3.3a): `gameSettings.setOptionValue(option, step)` with its
    /// side effects and `saveOptions`, as the options screen's button does
    /// (`option` is the translation key, e.g. `"options.ao"`,
    /// `"options.graphics"`, `"options.renderDistance"`).
    SetOption {
        /// `EnumOptions.getEnumString()`.
        option: String,
        /// The step (1 for an on/off option).
        #[serde(default = "one")]
        step: i32,
    },
    /// Dev (WP-3.3a): writes the F3 text (`DebugText`, `DebugSources`) and
    /// the terrain's counters (`terrain::TerrainStats`) as JSON.
    DumpDebug {
        /// The file.
        path: PathBuf,
    },
    /// Dev (WP-3.5a): spawns a model ([`crate::model::debug`]) at a fixed
    /// pose in front of the player with its skin from the jar, and with
    /// `path` set, takes a screenshot of it a few frames later.
    DebugModel(crate::model::debug::DebugModelSpec),
    /// Dev (WP-3.6a): opens a GUI test screen (`"widgets"`, `"list"`,
    /// `"font"`, `"yesno"`, or a registered screen name) and, with `path`,
    /// screenshots it a few frames later.
    DebugGui {
        /// The test screen.
        screen: String,
        /// The PNG.
        #[serde(default)]
        path: Option<PathBuf>,
    },
    /// Dev (WP-3.6a): `mc.displayGuiScreen(new X())` for the screen class
    /// `name` (`"GuiConflictWarning"`, `"GuiIngameMenu"`, …).
    OpenScreen {
        /// The class name.
        name: String,
    },
    /// Dev (WP-3.6a): puts the (unbuffered) pointer on scaled GUI pixel
    /// `(x, y)` (`Mouse.getX()`/`getY()`; later button events carry it).
    MouseTo {
        /// Scaled x.
        x: i32,
        /// Scaled y (down).
        y: i32,
    },
    /// Dev (WP-3.6a): types `text`: a key press and release per character
    /// carrying the character (key code 0, LWJGL's `KEY_NONE`).
    TypeText {
        /// The characters.
        text: String,
    },
    /// Dev (WP-3.7a): queues `world.spawnParticle(name, x, y, z, dx, dy,
    /// dz)` as the simulation would (`RenderGlobal.spawnParticle`'s names:
    /// `smoke`, `flame`, `reddust`, `explode`, `heart`, `note`, `bubble`,
    /// `portal`, …). With `ahead`, `x, y, z` are offsets from the point
    /// `ahead` blocks along the player's look from the eye.
    Particle {
        /// The particle name.
        name: String,
        /// x (or the offset with `ahead`).
        #[serde(default)]
        x: f64,
        /// y.
        #[serde(default)]
        y: f64,
        /// z.
        #[serde(default)]
        z: f64,
        /// The motion arguments.
        #[serde(default)]
        dx: f64,
        /// y motion.
        #[serde(default)]
        dy: f64,
        /// z motion.
        #[serde(default)]
        dz: f64,
        /// Blocks ahead of the eye.
        #[serde(default)]
        ahead: Option<f64>,
    },
}

/// The default `step` of [`Action::SetOption`].
fn one() -> i32 {
    1
}

/// One script line: an action and its optional time.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Step {
    /// Not before `ticksRan` grew by this much since the start.
    #[serde(default)]
    pub tick: Option<u64>,
    /// Not before this frame.
    #[serde(default)]
    pub frame: Option<u64>,
    /// The action.
    #[serde(flatten)]
    pub action: Action,
}

/// A script: `{"actions": [...]}` or a bare array.
#[derive(Clone, Debug, PartialEq)]
pub struct Script {
    /// The steps, in order.
    pub steps: Vec<Step>,
}

impl Script {
    /// Parses a script's JSON text.
    pub fn parse(text: &str) -> Result<Script, String> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum File {
            Object { actions: Vec<Step> },
            Array(Vec<Step>),
        }
        let f: File = serde_json::from_str(text).map_err(|e| format!("script: {e}"))?;
        let steps = match f {
            File::Object { actions } => actions,
            File::Array(a) => a,
        };
        for (i, s) in steps.iter().enumerate() {
            if let Action::KeyDown { key } | Action::KeyUp { key } = &s.action
                && !key.starts_with("key.")
                && keys::key_index(key).is_none()
            {
                return Err(format!("script step {i}: unknown key {key:?}"));
            }
        }
        Ok(Script { steps })
    }

    /// Reads and parses a script file.
    pub fn load(path: &std::path::Path) -> Result<Script, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Script::parse(&text)
    }
}

/// The running script.
#[derive(Resource, Clone, Debug)]
pub struct ScriptRunner {
    /// The script.
    pub script: Script,
    /// The next step.
    pub next: usize,
    /// Frames since the start.
    pub frames: u64,
    /// `ticksRan` at the start (set on the first frame).
    pub start_ticks: Option<i32>,
    /// A relative wait in progress: (until ticks, until frame, in game).
    waiting: Option<(Option<u64>, Option<u64>, bool)>,
}

impl ScriptRunner {
    /// A runner at the first step.
    pub fn new(script: Script) -> Self {
        ScriptRunner {
            script,
            next: 0,
            frames: 0,
            start_ticks: None,
            waiting: None,
        }
    }

    /// Whether every step ran.
    pub fn finished(&self) -> bool {
        self.next >= self.script.steps.len() && self.waiting.is_none()
    }
}

/// The LWJGL code of a script key (a name or a binding).
fn key_code(g: &Game, key: &str) -> Option<i32> {
    if key.starts_with("key.") {
        g.game_settings
            .key_bindings
            .iter()
            .find(|b| b.key_description == key)
            .map(|b| b.key_code)
    } else {
        keys::key_index(key)
    }
}

/// A JSON snapshot of the game for the `dump` action: the state, the world
/// (directory, level name, seed, dimension, time, loaded chunks, entities)
/// and the player (position, rotation, motion, health, dimension).
pub fn dump_json(g: &Game, state: &str) -> serde_json::Value {
    use serde_json::json;
    let world = g.the_world.as_ref().map(|w| {
        json!({
            "dir": g.world_dir,
            "level_name": w.world_info.level_name,
            "seed": w.world_info.random_seed,
            "dimension": w.world_provider.world_type,
            "world_time": w.get_world_time(),
            "loaded_chunks": w.loaded_chunk_count(),
            "entities": w.loaded_entity_list.len(),
            "spawn": [w.world_info.spawn_x, w.world_info.spawn_y, w.world_info.spawn_z],
            "raining": w.world_info.raining,
        })
    });
    let player = g.player().map(|p| {
        let b = &p.base;
        json!({
            "id": b.entity_id,
            "pos": [b.pos_x, b.pos_y, b.pos_z],
            "yaw": b.rotation_yaw,
            "pitch": b.rotation_pitch,
            "motion": [b.motion_x, b.motion_y, b.motion_z],
            "on_ground": b.on_ground,
            "health": p.kind.living().map(|l| l.health),
            "dimension": p.kind.player().map(|d| d.dimension),
            "current_item": p.kind.player().map(|d| d.inventory.current_item),
            "username": p.kind.player().and_then(|d| d.username.clone()),
            "in_portal": p.kind.player().map(|d| d.in_portal),
            "time_in_portal": p.kind.player().map(|d| d.time_in_portal),
            "time_until_portal": p.kind.player().map(|d| d.time_until_portal),
            "y_size": b.y_size,
        })
    });
    json!({
        "state": state,
        "ticks_ran": g.ticks_ran,
        "screen": g.current_screen.as_ref().map(|s| format!("{:?}", s.kind)),
        "in_game_has_focus": g.in_game_has_focus,
        "paused": g.is_game_paused,
        "third_person": g.game_settings.third_person_view,
        "show_debug_info": g.game_settings.show_debug_info,
        "object_mouse_over": g.object_mouse_over.map(|m| json!({
            "type": format!("{:?}", m.type_of_hit),
            "block": [m.block_x, m.block_y, m.block_z],
            "side": m.side_hit,
            "entity": m.entity_hit,
        })),
        "world": world,
        "player": player,
    })
}

/// Runs the due steps of the script, before the frame (module docs).
pub fn run_script(
    mut commands: Commands,
    runner: Option<ResMut<ScriptRunner>>,
    mut game: NonSendMut<Game>,
    pending: Res<PendingScreenshots>,
    state: Res<State<crate::app::AppState>>,
) {
    let Some(mut r) = runner else {
        return;
    };
    r.frames += 1;
    let start = *r.start_ticks.get_or_insert(game.ticks_ran);
    let ticks = game.ticks_ran.wrapping_sub(start).max(0) as u64;
    let in_game = game.the_world.is_some() && game.pending.is_none() && game.the_player.is_some();
    loop {
        if pending.count() > 0 {
            return;
        }
        if let Some((t, f, need_world)) = r.waiting {
            if t.is_some_and(|t| ticks < t)
                || f.is_some_and(|f| r.frames < f)
                || need_world && !in_game
            {
                return;
            }
            r.waiting = None;
        }
        let Some(step) = r.script.steps.get(r.next).cloned() else {
            return;
        };
        if step.tick.is_some_and(|t| ticks < t) || step.frame.is_some_and(|f| r.frames < f) {
            return;
        }
        r.next += 1;
        match step.action {
            Action::KeyDown { key } => {
                if let Some(code) = key_code(&game, &key) {
                    game.input.push_key(code, true, '\0');
                }
            }
            Action::KeyUp { key } => {
                if let Some(code) = key_code(&game, &key) {
                    game.input.push_key(code, false, '\0');
                }
            }
            Action::MouseMove { dx, dy } => {
                game.input.pending_dx += dx;
                game.input.pending_dy += dy;
            }
            Action::MouseDown { button } => game.input.push_button(button.lwjgl(), true),
            Action::MouseUp { button } => game.input.push_button(button.lwjgl(), false),
            Action::Wheel { notches } => game.input.push_wheel(notches * 120),
            Action::Look { yaw, pitch } => {
                if let Some(p) = game.player_in_world_mut() {
                    p.base.rotation_yaw = yaw;
                    p.base.rotation_pitch = pitch;
                    p.base.prev_rotation_yaw = yaw;
                    p.base.prev_rotation_pitch = pitch;
                }
            }
            Action::Wait {
                ticks: t,
                frames: f,
                in_game: w,
            } => {
                r.waiting = Some((
                    t.map(|t| ticks + t as u64),
                    f.map(|f| r.frames + f as u64),
                    w,
                ));
            }
            Action::Screenshot { path } => {
                take_screenshot(&mut commands, &pending, path);
                return;
            }
            Action::Dump { path } => {
                let v = dump_json(&game, &format!("{:?}", state.get()));
                if let Some(d) = path.parent() {
                    let _ = std::fs::create_dir_all(d);
                }
                let text = serde_json::to_string_pretty(&v).unwrap_or_default();
                match std::fs::write(&path, text + "\n") {
                    Ok(()) => info!("script: dump written to {}", path.display()),
                    Err(e) => error!("script: {}: {e}", path.display()),
                }
            }
            Action::Save => {
                if let Some(w) = game.the_world.as_mut() {
                    w.save_world_indirectly();
                }
            }
            Action::UsePortal => game.use_portal(false),
            Action::Respawn => game.respawn(false, 0),
            Action::Quit => {
                game.shutdown();
                return;
            }
            Action::DumpTexture { name, path } => {
                commands.queue(move |w: &mut World| {
                    match crate::textures::dump_texture(w, &name, &path) {
                        Ok(()) => info!("script: texture {name} written to {}", path.display()),
                        Err(e) => error!("script: dump_texture: {e}"),
                    }
                });
            }
            Action::SetTime { world_time } => {
                if let Some(w) = game.the_world.as_mut() {
                    w.set_world_time(world_time);
                }
            }
            Action::SetRain { strength, thunder } => {
                if let Some(w) = game.the_world.as_mut() {
                    w.set_rain_strength(strength);
                    w.world_info.raining = strength > 0.0;
                    w.world_info.rain_time = 1_000_000;
                    let t = if thunder { 1.0 } else { 0.0 };
                    w.thundering_strength = t;
                    w.prev_thundering_strength = t;
                    w.world_info.thundering = thunder;
                    w.world_info.thunder_time = 1_000_000;
                }
            }
            Action::SetPos { x, y, z } => {
                if let Some(p) = game.player_in_world_mut() {
                    let b = &mut p.base;
                    b.set_position(x, y, z);
                    (b.prev_pos_x, b.prev_pos_y, b.prev_pos_z) = (x, y, z);
                    (b.last_tick_pos_x, b.last_tick_pos_y, b.last_tick_pos_z) = (x, y, z);
                    (b.motion_x, b.motion_y, b.motion_z) = (0.0, 0.0, 0.0);
                    b.fall_distance = 0.0;
                }
            }
            Action::AuxSfx {
                kind,
                x,
                y,
                z,
                data,
            } => {
                if let Some(w) = game.the_world.as_mut() {
                    w.play_aux_sfx(kind, x, y, z, data);
                }
            }
            Action::SetOption { option, step } => {
                match crate::settings::EnumOptions::VALUES
                    .into_iter()
                    .find(|o| o.get_enum_string() == option)
                {
                    Some(o) => game.set_option_value(o, step),
                    None => error!("script: set_option: no option {option:?}"),
                }
            }
            Action::DumpDebug { path } => {
                commands.queue(move |w: &mut World| {
                    use serde_json::json;
                    let text = w.get_resource::<crate::debug::DebugText>().cloned();
                    let src = w.get_resource::<crate::debug::DebugSources>().cloned();
                    let stats = w.get_resource::<crate::terrain::TerrainStats>().copied();
                    let fps_line = w.get_non_send::<Game>().map(|g| g.debug.clone());
                    let v = json!({
                        "debug": fps_line,
                        "left": text.as_ref().map(|t| t.left.clone()),
                        "renders": src.as_ref().map(|s| s.renders.clone()),
                        "terrain": stats.map(|s| json!({
                            "rebuilds_this_frame": s.rebuilds_this_frame,
                            "rebuilds_total": s.rebuilds_total,
                            "pending_jobs": s.pending_jobs,
                            "drawn": s.drawn,
                            "to_update": s.to_update,
                            "snapshot_ms_total": s.snapshot_nanos_total as f64 / 1e6,
                            "mesh_ms_total": s.mesh_nanos_total as f64 / 1e6,
                            "terrain_frame_ms": s.frame_nanos as f64 / 1e6,
                        })),
                    });
                    if let Some(d) = path.parent() {
                        let _ = std::fs::create_dir_all(d);
                    }
                    let text = serde_json::to_string_pretty(&v).unwrap_or_default();
                    match std::fs::write(&path, text + "\n") {
                        Ok(()) => info!("script: debug dump written to {}", path.display()),
                        Err(e) => error!("script: {}: {e}", path.display()),
                    }
                });
            }
            Action::DebugModel(spec) => {
                let path = spec.path.clone();
                commands.queue(move |w: &mut World| {
                    w.get_resource_or_init::<crate::model::debug::DebugModelRequest>()
                        .0
                        .push(spec);
                });
                if let Some(path) = path {
                    // Give the new meshes, material, skin and pipeline
                    // (compiled asynchronously) 20 frames to reach the GPU,
                    // then capture.
                    let at = r.next;
                    r.script.steps.insert(
                        at,
                        Step {
                            tick: None,
                            frame: None,
                            action: Action::Wait {
                                ticks: None,
                                frames: Some(20),
                                in_game: false,
                            },
                        },
                    );
                    r.script.steps.insert(
                        at + 1,
                        Step {
                            tick: None,
                            frame: None,
                            action: Action::Screenshot { path },
                        },
                    );
                }
            }
            Action::DebugGui { screen, path } => {
                let name = crate::gui::debug_screens::debug_screen_name(&screen);
                commands.queue(move |w: &mut World| crate::gui::open_screen_by_name(w, &name));
                if let Some(path) = path {
                    // Drawn on the next frames, then captured.
                    let at = r.next;
                    r.script.steps.splice(
                        at..at,
                        [
                            Step {
                                tick: None,
                                frame: None,
                                action: Action::Wait {
                                    ticks: None,
                                    frames: Some(3),
                                    in_game: false,
                                },
                            },
                            Step {
                                tick: None,
                                frame: None,
                                action: Action::Screenshot { path },
                            },
                        ],
                    );
                }
                return;
            }
            Action::OpenScreen { name } => {
                commands.queue(move |w: &mut World| crate::gui::open_screen_by_name(w, &name));
                return;
            }
            Action::MouseTo { x, y } => {
                let (dw, dh) = (game.display_width, game.display_height);
                let sr = crate::app::ScaledResolution::new(game.game_settings.gui_scale, dw, dh);
                let (mx, my) = crate::gui::scaled::gui_to_mouse(
                    x,
                    y,
                    sr.scaled_width,
                    sr.scaled_height,
                    dw,
                    dh,
                );
                game.input.mouse_x = mx;
                game.input.mouse_y = my;
            }
            Action::TypeText { text } => {
                for ch in text.chars() {
                    game.input.push_key(0, true, ch);
                    game.input.push_key(0, false, ch);
                }
            }
            Action::Particle {
                name,
                x,
                y,
                z,
                dx,
                dy,
                dz,
                ahead,
            } => {
                let base = match ahead {
                    Some(d) => game.player().map(|p| {
                        let b = &p.base;
                        // EntityLiving.getLook(1.0F)'s direction (EntityLiving.java:813-823).
                        let (yaw, pitch) = (
                            (b.rotation_yaw as f64).to_radians(),
                            (b.rotation_pitch as f64).to_radians(),
                        );
                        let (lx, ly, lz) = (
                            -yaw.sin() * pitch.cos(),
                            -pitch.sin(),
                            yaw.cos() * pitch.cos(),
                        );
                        (b.pos_x + lx * d, b.pos_y + ly * d, b.pos_z + lz * d)
                    }),
                    None => Some((0.0, 0.0, 0.0)),
                };
                if let (Some((bx, by, bz)), Some(w)) = (base, game.the_world.as_mut()) {
                    w.events
                        .push(mc_sim::world::events::WorldEvent::SpawnParticle {
                            name: name.into(),
                            x: bx + x,
                            y: by + y,
                            z: bz + z,
                            vx: dx,
                            vy: dy,
                            vz: dz,
                        });
                }
            }
        }
    }
}

/// Inserts the runner and turns real devices off.
pub fn install(app: &mut App, script: Script) {
    app.insert_resource(ScriptRunner::new(script))
        .insert_resource(DeviceInput(false));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_documented_format() {
        let s = Script::parse(
            r#"{"actions": [
                {"action": "wait", "in_game": true},
                {"action": "wait", "ticks": 60},
                {"action": "look", "yaw": 90, "pitch": 10.5},
                {"action": "key_down", "key": "key.forward", "tick": 70},
                {"action": "key_up", "key": "W", "frame": 300},
                {"action": "mouse_move", "dx": 10, "dy": -5},
                {"action": "mouse_down", "button": "left"},
                {"action": "mouse_up", "button": "left"},
                {"action": "wheel", "notches": -1},
                {"action": "screenshot", "path": "build/screens/a.png"},
                {"action": "dump", "path": "build/screens/a.json"},
                {"action": "save"},
                {"action": "quit"}
            ]}"#,
        )
        .unwrap();
        assert_eq!(s.steps.len(), 13);
        assert_eq!(
            s.steps[2].action,
            Action::Look {
                yaw: 90.0,
                pitch: 10.5
            }
        );
        assert_eq!(s.steps[3].tick, Some(70));
        assert_eq!(s.steps[4].frame, Some(300));
        assert_eq!(
            s.steps[6].action,
            Action::MouseDown {
                button: ScriptButton::Left
            }
        );
        assert_eq!(s.steps[12].action, Action::Quit);
    }

    #[test]
    fn parses_the_wp_3_4_dev_actions() {
        let s = Script::parse(
            r#"[{"action": "set_time", "world_time": 18000},
                {"action": "set_rain", "strength": 1, "thunder": true},
                {"action": "set_rain", "strength": 0.5},
                {"action": "set_pos", "x": 0.5, "y": 200, "z": -3}]"#,
        )
        .unwrap();
        assert_eq!(s.steps[0].action, Action::SetTime { world_time: 18000 });
        assert_eq!(
            s.steps[1].action,
            Action::SetRain {
                strength: 1.0,
                thunder: true
            }
        );
        assert_eq!(
            s.steps[2].action,
            Action::SetRain {
                strength: 0.5,
                thunder: false
            }
        );
        assert_eq!(
            s.steps[3].action,
            Action::SetPos {
                x: 0.5,
                y: 200.0,
                z: -3.0
            }
        );
    }

    #[test]
    fn parses_the_wp_3_3a_dev_actions() {
        let s = Script::parse(
            r#"[{"action": "set_option", "option": "options.ao"},
                {"action": "set_option", "option": "options.renderDistance", "step": 2},
                {"action": "dump_debug", "path": "build/screens/d.json"}]"#,
        )
        .unwrap();
        assert_eq!(
            s.steps[0].action,
            Action::SetOption {
                option: "options.ao".into(),
                step: 1
            }
        );
        assert_eq!(
            s.steps[1].action,
            Action::SetOption {
                option: "options.renderDistance".into(),
                step: 2
            }
        );
        assert_eq!(
            s.steps[2].action,
            Action::DumpDebug {
                path: "build/screens/d.json".into()
            }
        );
    }

    #[test]
    fn parses_the_wp_3_6a_dev_actions() {
        let s = Script::parse(
            r#"[{"action": "debug_gui", "screen": "widgets", "path": "a.png"},
                {"action": "debug_gui", "screen": "font"},
                {"action": "open_screen", "name": "GuiConflictWarning"},
                {"action": "mouse_to", "x": 10, "y": 20},
                {"action": "type_text", "text": "ab"}]"#,
        )
        .unwrap();
        assert_eq!(
            s.steps[0].action,
            Action::DebugGui {
                screen: "widgets".into(),
                path: Some("a.png".into())
            }
        );
        assert_eq!(
            s.steps[1].action,
            Action::DebugGui {
                screen: "font".into(),
                path: None
            }
        );
        assert_eq!(s.steps[3].action, Action::MouseTo { x: 10, y: 20 });
        assert_eq!(s.steps[4].action, Action::TypeText { text: "ab".into() });
    }

    #[test]
    fn parses_the_wp_3_5a_debug_model_action() {
        let s = Script::parse(
            r#"[{"action": "debug_model", "model": "pig", "path": "build/screens/pig.png"},
                {"action": "debug_model", "model": "sheep_fur", "turn": 90, "keep": true}]"#,
        )
        .unwrap();
        let Action::DebugModel(a) = &s.steps[0].action else {
            panic!("{:?}", s.steps[0].action);
        };
        assert_eq!(a.model, "pig");
        assert_eq!(a.path, Some(PathBuf::from("build/screens/pig.png")));
        assert_eq!(a.distance, 3.0);
        let Action::DebugModel(b) = &s.steps[1].action else {
            panic!("{:?}", s.steps[1].action);
        };
        assert_eq!((b.turn, b.keep, b.path.is_none()), (90.0, true, true));
    }

    #[test]
    fn parses_the_wp_3_7a_particle_action() {
        let s = Script::parse(
            r#"[{"action": "particle", "name": "smoke", "x": 1, "y": 64.5, "z": -2},
                {"action": "particle", "name": "reddust", "ahead": 2, "dx": 0.5}]"#,
        )
        .unwrap();
        assert_eq!(
            s.steps[0].action,
            Action::Particle {
                name: "smoke".into(),
                x: 1.0,
                y: 64.5,
                z: -2.0,
                dx: 0.0,
                dy: 0.0,
                dz: 0.0,
                ahead: None
            }
        );
        assert_eq!(
            s.steps[1].action,
            Action::Particle {
                name: "reddust".into(),
                x: 0.0,
                y: 0.0,
                z: 0.0,
                dx: 0.5,
                dy: 0.0,
                dz: 0.0,
                ahead: Some(2.0)
            }
        );
    }

    #[test]
    fn bare_arrays_and_errors() {
        let s = Script::parse(r#"[{"action": "quit"}]"#).unwrap();
        assert_eq!(s.steps.len(), 1);
        assert!(Script::parse(r#"[{"action": "fly"}]"#).is_err());
        assert!(Script::parse(r#"[{"action": "key_down", "key": "WW"}]"#).is_err());
        assert!(Script::parse(r#"[{"action": "key_down"}]"#).is_err());
    }
}
