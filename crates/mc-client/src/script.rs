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
    fn bare_arrays_and_errors() {
        let s = Script::parse(r#"[{"action": "quit"}]"#).unwrap();
        assert_eq!(s.steps.len(), 1);
        assert!(Script::parse(r#"[{"action": "fly"}]"#).is_err());
        assert!(Script::parse(r#"[{"action": "key_down", "key": "WW"}]"#).is_err());
        assert!(Script::parse(r#"[{"action": "key_down"}]"#).is_err());
    }
}
