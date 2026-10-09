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
    /// Dev (WP-3.7b): sets the player's `fire` ticks (`isBurning()`: the
    /// first-person fire overlay; the simulation counts it down, burns the
    /// player every 20 ticks, and water puts it out).
    SetFire {
        /// `Entity.fire`.
        ticks: i32,
    },
    /// Dev (WP-3.5b): spawns an entity (`crate::render::dev`): an
    /// `EntityList` name at `x, y, z` (or ahead of the player) with a few
    /// render-visible fields.
    SpawnEntity(crate::render::dev::SpawnEntitySpec),
    /// Dev (WP-3.3b, WP-3.3d, WP-3.5b, WP-3.7b): sets a block or a box of
    /// blocks (`crate::render::dev::SetBlockSpec`), with a sign's text:
    /// `world.setBlockAndMetadataWithNotify(x, y, z, id, meta)` (the block's
    /// `onBlockAdded` and its neighbours' `onNeighborBlockChange` run, as for
    /// a placed block; a placed fluid source flows as the simulation runs
    /// it). With `raw`, `setBlockAndMetadata` (no neighbour notification;
    /// `onBlockAdded` still runs), then the metadata forced again with
    /// `setBlockMetadata` and `markBlockNeedsUpdate`: a shape the game would
    /// not keep (a lone curved rail, a powered wire with no source). With
    /// `rel`, `x, y, z` are offsets from the player's block (`posY` is the
    /// eye), e.g. still water, 9, around the eye for the water overlay.
    SetBlock(crate::render::dev::SetBlockSpec),
    /// Dev (WP-3.5b): puts a stack into one of the player's inventory or
    /// armour slots (`crate::render::dev`).
    Equip(crate::render::dev::EquipSpec),
    /// Dev (WP-3.6b): `ingameGUI.addChatMessage(text)` (a chat line, as
    /// the screenshot key's line is added).
    Chat {
        /// The line.
        text: String,
    },
    /// Dev (WP-3.6b): writes the player's inventory as JSON: the 36 main
    /// slots, the 4 armour slots and the stack on the cursor
    /// (`InventoryPlayer.itemStack`), each `[id, count, damage]` or null.
    DumpInventory {
        /// The file.
        path: PathBuf,
    },
    /// Dev (WP-3.6b, WP-3.7b; merged in WP-3.9, `give_item` is an alias):
    /// puts `new ItemStack(id, count, damage)` into armour slot `armor` (0
    /// boots … 3 helmet) when given, else into main-inventory slot `slot`
    /// (0-8: the hotbar; replacing what is there) when given, else into the
    /// inventory as a pickup does (`InventoryPlayer.addItemStackToInventory`).
    #[serde(alias = "give_item")]
    Give {
        /// Item or block id.
        id: i32,
        /// Count.
        #[serde(default = "one")]
        count: i32,
        /// Damage.
        #[serde(default)]
        damage: i32,
        /// The main-inventory slot (0-8: the hotbar).
        #[serde(default)]
        slot: Option<i32>,
        /// Armour slot instead.
        #[serde(default)]
        armor: Option<usize>,
    },
    /// Dev (WP-3.6c): presses the button labelled `label` on the current
    /// screen: the pointer moves to its centre and a left press and
    /// release follow (vanilla's `mouseClicked` path). Waits up to 120
    /// frames for a screen with such an enabled button.
    ClickButton {
        /// The button's text (`displayString`), as drawn.
        label: String,
    },
    /// Dev (WP-3.9): clicks row `index` of the current screen's `GuiSlot`
    /// list (the world list, the statistics, the texture packs): the
    /// pointer moves to the row's centre ([`slot_row_center`]), the left
    /// button is held for two frames (`GuiSlot` polls it while drawing),
    /// and with `double` a second click follows four frames after the
    /// first (Play on the world list). `GuiSlot` takes it as a double click
    /// only when the second press comes under 250 ms of wall-clock time
    /// after the first (`System.currentTimeMillis()`, GuiSlot.java:138), so
    /// `double` needs frames under about 60 ms (above about 16 fps); on
    /// slower frames the second press is a fresh single click. Waits up to
    /// 120 frames for a list with such a row.
    ClickSlot {
        /// The row (0 = the first).
        index: i32,
        /// A double click.
        #[serde(default)]
        double: bool,
    },
    /// Dev (WP-4.5): exports the world `saves/<name>` into the folder
    /// `path` (as `<path>/<name>`) through [`crate::export::export_from_game`]:
    /// the open world is saved first when it is that world; the world is
    /// validated, copied and read back. In game this is a full
    /// `saveWorld(true)` (World.java:278-291), which vanilla runs only on
    /// quit or a world change: it resets every modified chunk's
    /// `isModified`/`lastSaveTime`, so the run's later autosaves differ
    /// from an unexported run's. With `report`, the result (the
    /// report with every file's and chunk's digest, or the error) is written
    /// there as JSON.
    ExportWorld {
        /// The world's folder in `saves/`.
        name: String,
        /// The target folder.
        path: PathBuf,
        /// The JSON report.
        #[serde(default)]
        report: Option<PathBuf>,
    },
}

/// The centre of row `index` of `slot` (a list of `size` rows) in scaled
/// GUI pixels, if the row exists and its centre is inside the list
/// (`top ..= bottom`, where `GuiSlot` takes clicks): `GuiSlot`'s row top
/// `top + 4 - (int) amountScrolled + index · posZ + headerPadding`, plus
/// half the row's `posZ - 4` height; the inverse of the click's row maths
/// (GuiSlot.java:132-136, :199-207). A short list is centred (negative
/// `amountScrolled`, [`GuiSlot::bind_amount_scrolled`]).
///
/// [`GuiSlot::bind_amount_scrolled`]: crate::gui::widgets::slot::GuiSlot::bind_amount_scrolled
pub fn slot_row_center(
    slot: &crate::gui::widgets::slot::GuiSlot,
    size: i32,
    index: i32,
) -> Option<(i32, i32)> {
    if index < 0 || index >= size {
        return None;
    }
    let row_top =
        slot.top + 4 - slot.amount_scrolled as i32 + index * slot.pos_z + slot.header_padding;
    let y = row_top + (slot.pos_z - 4) / 2;
    (y >= slot.top && y <= slot.bottom).then_some((slot.width / 2, y))
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
    /// Frames a `click_button` has waited for its button (WP-3.6c).
    click_waited: u32,
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
            click_waited: 0,
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
    binder: Option<Res<crate::gui::screen::Binder>>,
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
                    // WP-4.3: the frame-time window, phases, start-up and
                    // the storage counters.
                    let perf = w
                        .get_resource::<crate::perf::PerfStats>()
                        .map(|p| p.to_json());
                    let storage = crate::storage::StorageCounters::now().to_json();
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
                        "perf": perf,
                        "storage": storage,
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
            Action::ClickButton { label } => {
                let found = binder
                    .as_deref()
                    .filter(|b| b.bound_serial() == Some(game.screen_serial))
                    .and_then(|b| b.screen())
                    .and_then(|s| {
                        s.base()
                            .control_list
                            .iter()
                            .find(|b| b.enabled && b.enabled2 && b.display_string == label)
                            .map(|b| (b.x_position + b.width / 2, b.y_position + b.height / 2))
                    });
                match found {
                    Some((x, y)) => {
                        r.click_waited = 0;
                        let (dw, dh) = (game.display_width, game.display_height);
                        let sr =
                            crate::app::ScaledResolution::new(game.game_settings.gui_scale, dw, dh);
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
                        game.input.push_button(0, true);
                        game.input.push_button(0, false);
                        info!("script: clicked {label:?}");
                    }
                    None if r.click_waited < 120 => {
                        // Not there yet: try again next frame.
                        r.click_waited += 1;
                        r.next -= 1;
                        return;
                    }
                    None => {
                        r.click_waited = 0;
                        error!("script: click_button: no enabled button {label:?}");
                    }
                }
            }
            Action::ClickSlot { index, double } => {
                let found = binder
                    .as_deref()
                    .filter(|b| b.bound_serial() == Some(game.screen_serial))
                    .and_then(|b| b.screen())
                    .and_then(|s| s.gui_slot())
                    .and_then(|(slot, size)| slot_row_center(slot, size, index));
                match found {
                    Some((x, y)) => {
                        r.click_waited = 0;
                        let (dw, dh) = (game.display_width, game.display_height);
                        let sr =
                            crate::app::ScaledResolution::new(game.game_settings.gui_scale, dw, dh);
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
                        game.input.push_button(0, true);
                        info!(
                            "script: clicking row {index} at ({x}, {y}){}",
                            if double { " twice" } else { "" }
                        );
                        let step = |action| Step {
                            tick: None,
                            frame: None,
                            action,
                        };
                        let wait = |n| Action::Wait {
                            ticks: None,
                            frames: Some(n),
                            in_game: false,
                        };
                        let left = ScriptButton::Left;
                        let mut more = vec![step(wait(2)), step(Action::MouseUp { button: left })];
                        if double {
                            more.extend([
                                step(wait(2)),
                                step(Action::MouseDown { button: left }),
                                step(wait(2)),
                                step(Action::MouseUp { button: left }),
                            ]);
                        }
                        let at = r.next;
                        r.script.steps.splice(at..at, more);
                    }
                    None if r.click_waited < 120 => {
                        r.click_waited += 1;
                        r.next -= 1;
                        return;
                    }
                    None => {
                        r.click_waited = 0;
                        error!("script: click_slot: no visible row {index}");
                    }
                }
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
            Action::SetFire { ticks } => {
                if let Some(p) = game.player_in_world_mut() {
                    p.base.fire = ticks;
                }
            }
            Action::SpawnEntity(spec) => {
                crate::render::dev::spawn_entity(&mut game, &spec);
            }
            Action::SetBlock(spec) => {
                crate::render::dev::set_block(&mut game, &spec);
            }
            Action::Equip(spec) => {
                crate::render::dev::equip(&mut game, &spec);
            }
            Action::Chat { text } => {
                commands.queue(move |w: &mut World| {
                    w.resource_scope(|w, mut ing: Mut<crate::gui::ingame::GuiIngame>| {
                        if let Some(f) = w.get_resource::<crate::gui::font::FontRenderer>() {
                            ing.add_chat_message(f, &text);
                        }
                    });
                });
            }
            Action::Give {
                id,
                count,
                damage,
                slot,
                armor,
            } => {
                if let Some(d) = game.player_in_world_mut().and_then(|e| e.kind.player_mut()) {
                    let mut s = mc_sim::item::ItemStack::new(id, count, damage);
                    match (armor, slot) {
                        (Some(i), _) if i < 4 => d.inventory.armor_inventory[i] = Some(s),
                        (Some(i), _) => error!("script: give: no armour slot {i}"),
                        (None, Some(i)) => d.inventory.set_inventory_slot_contents(i, Some(s)),
                        (None, None) => {
                            d.inventory.add_item_stack_to_inventory(&mut s);
                        }
                    }
                }
            }
            Action::ExportWorld { name, path, report } => {
                let v = match crate::export::export_from_game(&mut game, &name, &path) {
                    Ok(r) => {
                        info!(
                            "script: exported saves/{name} to {} ({} files, {} chunks checked)",
                            r.path.display(),
                            r.files,
                            r.digest.chunks.len()
                        );
                        serde_json::json!({"ok": true, "report": r.to_json()})
                    }
                    Err(e) => {
                        error!("script: export_world {name}: {e}");
                        serde_json::json!({"ok": false, "error": e.to_string()})
                    }
                };
                if let Some(report) = report {
                    if let Some(d) = report.parent() {
                        let _ = std::fs::create_dir_all(d);
                    }
                    let text = serde_json::to_string_pretty(&v).unwrap_or_default();
                    if let Err(e) = std::fs::write(&report, text + "\n") {
                        error!("script: {}: {e}", report.display());
                    }
                }
            }
            Action::DumpInventory { path } => {
                let v = inventory_json(&game);
                if let Some(d) = path.parent() {
                    let _ = std::fs::create_dir_all(d);
                }
                let text = serde_json::to_string_pretty(&v).unwrap_or_default();
                match std::fs::write(&path, text + "\n") {
                    Ok(()) => info!("script: inventory written to {}", path.display()),
                    Err(e) => error!("script: {}: {e}", path.display()),
                }
            }
        }
    }
}

/// The player's inventory for the `dump_inventory` action.
pub fn inventory_json(g: &Game) -> serde_json::Value {
    use serde_json::json;
    let st = |s: &Option<mc_sim::item::ItemStack>| {
        s.map(|s| json!([s.item_id, s.stack_size, s.item_damage]))
    };
    match g.player().and_then(|p| p.kind.player()) {
        Some(d) => json!({
            "main": d.inventory.main_inventory.iter().map(st).collect::<Vec<_>>(),
            "armor": d.inventory.armor_inventory.iter().map(st).collect::<Vec<_>>(),
            "cursor": st(&d.inventory.item_stack),
            "current_item": d.inventory.current_item,
        }),
        None => serde_json::Value::Null,
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
    fn parses_the_wp_3_3b_dev_action() {
        let s = Script::parse(
            r#"[{"action": "set_block", "x": 1, "y": 70, "z": -2, "id": 9},
                {"action": "set_block", "x": 1, "y": 70, "z": -2, "id": 8, "meta": 3}]"#,
        )
        .unwrap();
        assert_eq!(
            s.steps[0].action,
            Action::SetBlock(crate::render::dev::SetBlockSpec {
                x: 1.0,
                y: 70.0,
                z: -2.0,
                id: 9,
                meta: 0,
                raw: false,
                rel: false,
                ..Default::default()
            })
        );
        assert_eq!(
            s.steps[1].action,
            Action::SetBlock(crate::render::dev::SetBlockSpec {
                x: 1.0,
                y: 70.0,
                z: -2.0,
                id: 8,
                meta: 3,
                raw: false,
                rel: false,
                ..Default::default()
            })
        );
    }

    #[test]
    fn parses_the_wp_3_3d_dev_action() {
        let s = Script::parse(
            r#"[{"action": "set_block", "x": 1, "y": 64, "z": -2, "id": 66, "meta": 7, "raw": true},
                {"action": "set_block", "x": 0, "y": 65, "z": 0, "id": 85}]"#,
        )
        .unwrap();
        assert_eq!(
            s.steps[0].action,
            Action::SetBlock(crate::render::dev::SetBlockSpec {
                x: 1.0,
                y: 64.0,
                z: -2.0,
                id: 66,
                meta: 7,
                raw: true,
                rel: false,
                ..Default::default()
            })
        );
        assert_eq!(
            s.steps[1].action,
            Action::SetBlock(crate::render::dev::SetBlockSpec {
                x: 0.0,
                y: 65.0,
                z: 0.0,
                id: 85,
                meta: 0,
                raw: false,
                rel: false,
                ..Default::default()
            })
        );
    }

    #[test]
    fn parses_the_wp_3_7b_dev_actions() {
        let s = Script::parse(
            r#"[{"action": "give_item", "id": 3, "count": 5, "slot": 1},
                {"action": "give_item", "id": 280},
                {"action": "set_fire", "ticks": 300},
                {"action": "set_block", "x": 0, "y": 0, "z": 0, "id": 9, "rel": true}]"#,
        )
        .unwrap();
        // give_item is an alias of give (WP-3.9).
        assert_eq!(
            s.steps[0].action,
            Action::Give {
                id: 3,
                count: 5,
                damage: 0,
                slot: Some(1),
                armor: None,
            }
        );
        assert_eq!(
            s.steps[1].action,
            Action::Give {
                id: 280,
                count: 1,
                damage: 0,
                slot: None,
                armor: None,
            }
        );
        assert_eq!(s.steps[2].action, Action::SetFire { ticks: 300 });
        assert_eq!(
            s.steps[3].action,
            Action::SetBlock(crate::render::dev::SetBlockSpec {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                id: 9,
                meta: 0,
                raw: false,
                rel: true,
                ..Default::default()
            })
        );
    }

    #[test]
    fn parses_the_wp_4_5_export_action() {
        let s = Script::parse(
            r#"[{"action": "export_world", "name": "w", "path": "/tmp/out"},
                {"action": "export_world", "name": "w", "path": "o", "report": "r.json"}]"#,
        )
        .unwrap();
        assert_eq!(
            s.steps[0].action,
            Action::ExportWorld {
                name: "w".into(),
                path: "/tmp/out".into(),
                report: None
            }
        );
        assert_eq!(
            s.steps[1].action,
            Action::ExportWorld {
                name: "w".into(),
                path: "o".into(),
                report: Some("r.json".into())
            }
        );
        assert!(Script::parse(r#"[{"action": "export_world", "name": "w"}]"#).is_err());
    }

    #[test]
    fn bare_arrays_and_errors() {
        let s = Script::parse(r#"[{"action": "quit"}]"#).unwrap();
        assert_eq!(s.steps.len(), 1);
        assert!(Script::parse(r#"[{"action": "fly"}]"#).is_err());
        assert!(Script::parse(r#"[{"action": "key_down", "key": "WW"}]"#).is_err());
        assert!(Script::parse(r#"[{"action": "key_down"}]"#).is_err());
    }

    #[test]
    fn give_and_click_slot_parse() {
        let s = Script::parse(
            r#"[{"action": "give", "id": 298, "armor": 3},
                {"action": "give", "id": 276, "slot": 0},
                {"action": "click_slot", "index": 0},
                {"action": "click_slot", "index": 2, "double": true}]"#,
        )
        .unwrap();
        assert_eq!(
            s.steps[0].action,
            Action::Give {
                id: 298,
                count: 1,
                damage: 0,
                slot: None,
                armor: Some(3),
            }
        );
        assert!(matches!(
            s.steps[1].action,
            Action::Give { slot: Some(0), .. }
        ));
        assert_eq!(
            s.steps[2].action,
            Action::ClickSlot {
                index: 0,
                double: false
            }
        );
        assert_eq!(
            s.steps[3].action,
            Action::ClickSlot {
                index: 2,
                double: true
            }
        );
    }

    #[test]
    fn slot_rows_are_found_where_guislot_takes_the_click() {
        use crate::gui::widgets::slot::GuiSlot;
        // The world list at 854 × 480, scale 2 (427 × 240): GuiSlot(…, 32,
        // height - 64, 36). One world: the short list is centred.
        let mut slot = GuiSlot::new(427, 240, 32, 240 - 64, 36);
        let content = 36;
        slot.bind_amount_scrolled(content);
        // (36 - (176 - 32 - 4)) / 2 = -52
        assert_eq!(slot.amount_scrolled, -52.0);
        let (x, y) = slot_row_center(&slot, 1, 0).unwrap();
        assert_eq!((x, y), (213, 32 + 4 + 52 + 16));
        // The O1 case: the first row is at y ≈ 92-124, not at the top.
        assert!((88..=124).contains(&y));
        // GuiSlot's own row maths picks the same row back.
        assert_eq!(slot.get_slot_index_from_screen_coords(1, x, y), 0);
        assert_eq!(slot_row_center(&slot, 1, 1), None);
        assert_eq!(slot_row_center(&slot, 1, -1), None);
        // A long, scrolled list with a header (the statistics tables).
        let mut slot = GuiSlot::new(427, 240, 32, 176, 20);
        slot.set_has_list_header(true, 20);
        slot.amount_scrolled = 30.0;
        for i in 0..6 {
            if let Some((x, y)) = slot_row_center(&slot, 40, i) {
                assert_eq!(slot.get_slot_index_from_screen_coords(40, x, y), i);
            }
        }
        // Row 0 scrolled above the list: not clickable.
        slot.amount_scrolled = 100.0;
        assert_eq!(slot_row_center(&slot, 40, 0), None);
        assert!(slot_row_center(&slot, 40, 6).is_some());
    }
}
