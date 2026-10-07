//! The sound library under `SoundManager`: what vanilla's paulscode
//! `SoundSystem` + `LibraryLWJGLOpenAL` (library code bundled in
//! `minecraft.jar`, not in the spec tree) do with the calls
//! `SoundManager` makes, as far as it is audible. Pure state; the Bevy side
//! ([`super::KiraBackend`]) turns its [`Command`]s into `bevy_kira_audio`
//! plays and per-frame volume/panning updates.
//!
//! Behaviour reproduced (read from the library's bytecode, so it is
//! described, not cited):
//!
//! - **Sources by name.** `newSource` / `newStreamingSource` /
//!   `backgroundMusic` create a named source; a new source of an existing
//!   name replaces (and silences) the old one. `playing(name)` is true from
//!   `play` until the sound ends or is stopped.
//! - **Pitch** is clamped to 0.5..2.0 (`Source.setPitch`); **volume** to
//!   0..1 (`Library.setVolume`).
//! - **Attenuation.** `SoundManager` uses `ATTENUATION_LINEAR` (2) for
//!   positional sounds and `ATTENUATION_NONE` (0) for music and `playSoundFX`.
//!   Linear: OpenAL's rolloff is set to 0 and the library computes the gain
//!   itself whenever the source or the listener moves: `1 - distance /
//!   distOrRoll`, clamped to 0..1 (1 at distance 0); the source's AL gain is
//!   that times its volume. None: gain 1.
//! - **Panning.** Sources are positioned in world coordinates (never
//!   listener-relative), and OpenAL pans a mono sound by its direction from
//!   the listener; stereo sounds are not spatialised. The approximation:
//!   kira panning = the cosine between the direction to the source and the
//!   listener's right vector (`at × up`), applied only to mono sounds (the
//!   backend knows the channel count).
//!
//!   vanilla bug: `playSoundFX` and the background music use position
//!   (0, 0, 0) with no attenuation, so a *mono* FX sound is panned toward the
//!   world origin (the GUI click and the music files are stereo, so this
//!   is inaudible for them; `portal.trigger`/`portal.travel` are affected).
//!
//! Not reproduced: paulscode's channel limits (28 normal + 4 streaming
//! channels, the oldest non-priority source culled when full; kira plays up
//! to 128 sounds) and OpenAL Soft's exact pan law.

use std::collections::BTreeMap;

use super::pool::SoundPoolEntry;

/// `SoundSystemConfig.ATTENUATION_NONE`.
pub const ATTENUATION_NONE: i32 = 0;
/// `SoundSystemConfig.ATTENUATION_ROLLOFF` (not used by `SoundManager`).
pub const ATTENUATION_ROLLOFF: i32 = 1;
/// `SoundSystemConfig.ATTENUATION_LINEAR`.
pub const ATTENUATION_LINEAR: i32 = 2;

/// The listener (`ListenerData`): position and the `at`/`up` orientation
/// vectors. The library's default faces -z with +y up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Listener {
    /// Position.
    pub position: [f32; 3],
    /// Look direction.
    pub look_at: [f32; 3],
    /// Up direction.
    pub up: [f32; 3],
}

impl Default for Listener {
    fn default() -> Self {
        Listener {
            position: [0.0; 3],
            look_at: [0.0, 0.0, -1.0],
            up: [0.0, 1.0, 0.0],
        }
    }
}

/// A source's life.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceState {
    /// Created, not played yet.
    Idle,
    /// Played and not finished (also while the backend is still loading it).
    Playing,
    /// Finished or stopped.
    Stopped,
}

/// One named source.
#[derive(Clone, Debug, PartialEq)]
pub struct Source {
    /// A unique id for the backend (a reused name gets a new id).
    pub id: u64,
    /// The file.
    pub entry: SoundPoolEntry,
    /// A streaming source (music, records): decoded off the main thread.
    pub streaming: bool,
    /// Position.
    pub position: [f32; 3],
    /// `ATTENUATION_*`.
    pub attenuation: i32,
    /// `distOrRoll`: the linear model's range.
    pub dist_or_roll: f32,
    /// Pitch (clamped).
    pub pitch: f32,
    /// `sourceVolume` (clamped).
    pub volume: f32,
    /// State.
    pub state: SourceState,
    /// The gain and pan last sent to the backend.
    sent: Option<(f32, f32)>,
}

impl Source {
    /// The attenuation gain at `listener` (module docs).
    pub fn attenuation_gain(&self, listener: &Listener) -> f32 {
        if self.attenuation != ATTENUATION_LINEAR {
            return 1.0;
        }
        // calculateDistance: float differences, double sqrt, back to float.
        let dx = (self.position[0] - listener.position[0]) as f64;
        let dy = (self.position[1] - listener.position[1]) as f64;
        let dz = (self.position[2] - listener.position[2]) as f64;
        let d = (dx * dx + dy * dy + dz * dz).sqrt() as f32;
        // calculateGain (linear)
        let mut gain = if d <= 0.0 {
            1.0
        } else if d >= self.dist_or_roll {
            0.0
        } else {
            1.0 - d / self.dist_or_roll
        };
        gain = gain.clamp(0.0, 1.0);
        gain
    }

    /// The output gain: attenuation × volume.
    pub fn gain(&self, listener: &Listener) -> f32 {
        self.attenuation_gain(listener) * self.volume
    }

    /// The pan of a mono sound at this position (module docs): -1 left, 1
    /// right.
    pub fn pan(&self, listener: &Listener) -> f32 {
        let d = [
            self.position[0] - listener.position[0],
            self.position[1] - listener.position[1],
            self.position[2] - listener.position[2],
        ];
        let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        let a = listener.look_at;
        let u = listener.up;
        let r = [
            a[1] * u[2] - a[2] * u[1],
            a[2] * u[0] - a[0] * u[2],
            a[0] * u[1] - a[1] * u[0],
        ];
        let rlen = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
        if len <= 1.0e-6 || rlen <= 1.0e-6 {
            return 0.0;
        }
        ((d[0] * r[0] + d[1] * r[1] + d[2] * r[2]) / (len * rlen)).clamp(-1.0, 1.0)
    }
}

/// What the backend must do.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    /// Start a sound.
    Play {
        /// Source id.
        id: u64,
        /// The file.
        entry: SoundPoolEntry,
        /// Streaming (decode off the main thread, do not cache).
        streaming: bool,
        /// Output gain (linear amplitude).
        gain: f32,
        /// Pan for a mono sound.
        pan: f32,
        /// Playback rate.
        pitch: f32,
    },
    /// New gain/pan for a playing sound.
    Update {
        /// Source id.
        id: u64,
        /// Output gain.
        gain: f32,
        /// Pan for a mono sound.
        pan: f32,
    },
    /// Stop a sound.
    Stop {
        /// Source id.
        id: u64,
    },
}

/// The library state (module docs).
#[derive(Clone, Debug, Default)]
pub struct SoundLibrary {
    /// The listener.
    pub listener: Listener,
    sources: BTreeMap<String, Source>,
    next_id: u64,
    commands: Vec<Command>,
}

impl SoundLibrary {
    /// A new library (`new SoundSystem()`).
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(clippy::too_many_arguments)]
    fn create(
        &mut self,
        streaming: bool,
        name: &str,
        entry: &SoundPoolEntry,
        x: f32,
        y: f32,
        z: f32,
        attenuation: i32,
        dist_or_roll: f32,
    ) {
        self.next_id += 1;
        let s = Source {
            id: self.next_id,
            entry: entry.clone(),
            streaming,
            position: [x, y, z],
            attenuation,
            dist_or_roll,
            pitch: 1.0,
            volume: 1.0,
            state: SourceState::Idle,
            sent: None,
        };
        if let Some(old) = self.sources.insert(name.to_string(), s)
            && old.state == SourceState::Playing
        {
            self.commands.push(Command::Stop { id: old.id });
        }
    }

    /// `newSource(priority, name, url, identifier, loop, x, y, z, att,
    /// distOrRoll)` (never looping in `SoundManager`; the priority only
    /// matters for channel culling, which is not reproduced).
    #[allow(clippy::too_many_arguments)]
    pub fn new_source(
        &mut self,
        _priority: bool,
        name: &str,
        entry: &SoundPoolEntry,
        x: f32,
        y: f32,
        z: f32,
        attenuation: i32,
        dist_or_roll: f32,
    ) {
        self.create(false, name, entry, x, y, z, attenuation, dist_or_roll);
    }

    /// `newStreamingSource(...)`.
    #[allow(clippy::too_many_arguments)]
    pub fn new_streaming_source(
        &mut self,
        _priority: bool,
        name: &str,
        entry: &SoundPoolEntry,
        x: f32,
        y: f32,
        z: f32,
        attenuation: i32,
        dist_or_roll: f32,
    ) {
        self.create(true, name, entry, x, y, z, attenuation, dist_or_roll);
    }

    /// `backgroundMusic(name, url, identifier, loop)`: a quick-play of a
    /// streaming source at (0, 0, 0) with no attenuation, then `play`.
    pub fn background_music(&mut self, name: &str, entry: &SoundPoolEntry) {
        self.create(true, name, entry, 0.0, 0.0, 0.0, ATTENUATION_NONE, 0.0);
        self.play(name);
    }

    /// `setPitch(name, pitch)`: clamped to 0.5..2.0.
    pub fn set_pitch(&mut self, name: &str, pitch: f32) {
        if let Some(s) = self.sources.get_mut(name) {
            // (NaN stays NaN, as with Java's comparisons)
            s.pitch = pitch.clamp(0.5, 2.0);
        }
    }

    /// `setVolume(name, volume)`: clamped to 0..1; a playing source's gain
    /// changes at once.
    pub fn set_volume(&mut self, name: &str, volume: f32) {
        let listener = self.listener;
        if let Some(s) = self.sources.get_mut(name) {
            s.volume = volume.clamp(0.0, 1.0);
            if s.state == SourceState::Playing {
                let gp = (s.gain(&listener), s.pan(&listener));
                s.sent = Some(gp);
                self.commands.push(Command::Update {
                    id: s.id,
                    gain: gp.0,
                    pan: gp.1,
                });
            }
        }
    }

    /// `play(name)`; playing an already playing source does nothing.
    pub fn play(&mut self, name: &str) {
        let listener = self.listener;
        if let Some(s) = self.sources.get_mut(name)
            && s.state != SourceState::Playing
        {
            s.state = SourceState::Playing;
            let gp = (s.gain(&listener), s.pan(&listener));
            s.sent = Some(gp);
            self.commands.push(Command::Play {
                id: s.id,
                entry: s.entry.clone(),
                streaming: s.streaming,
                gain: gp.0,
                pan: gp.1,
                pitch: s.pitch,
            });
        }
    }

    /// `stop(name)`.
    pub fn stop(&mut self, name: &str) {
        if let Some(s) = self.sources.get_mut(name)
            && s.state == SourceState::Playing
        {
            s.state = SourceState::Stopped;
            self.commands.push(Command::Stop { id: s.id });
        }
    }

    /// `playing(name)`.
    pub fn playing(&self, name: &str) -> bool {
        self.sources
            .get(name)
            .is_some_and(|s| s.state == SourceState::Playing)
    }

    /// `setListenerPosition(x, y, z)`.
    pub fn set_listener_position(&mut self, x: f32, y: f32, z: f32) {
        self.listener.position = [x, y, z];
    }

    /// `setListenerOrientation(lookX, lookY, lookZ, upX, upY, upZ)`.
    #[allow(clippy::too_many_arguments)]
    pub fn set_listener_orientation(
        &mut self,
        lx: f32,
        ly: f32,
        lz: f32,
        ux: f32,
        uy: f32,
        uz: f32,
    ) {
        self.listener.look_at = [lx, ly, lz];
        self.listener.up = [ux, uy, uz];
    }

    /// `cleanup()`: every source stops and is removed.
    pub fn cleanup(&mut self) {
        for s in self.sources.values() {
            if s.state == SourceState::Playing {
                self.commands.push(Command::Stop { id: s.id });
            }
        }
        self.sources.clear();
    }

    /// The backend reports that a sound ended (or could not be loaded).
    pub fn on_finished(&mut self, id: u64) {
        if let Some(s) = self.sources.values_mut().find(|s| s.id == id) {
            s.state = SourceState::Stopped;
        }
    }

    /// The per-frame listener update (`listenerMoved` → every source's
    /// `positionChanged`): new gain/pan for the playing sources whose values
    /// changed.
    pub fn frame_update(&mut self) {
        let listener = self.listener;
        for s in self.sources.values_mut() {
            if s.state != SourceState::Playing {
                continue;
            }
            let gp = (s.gain(&listener), s.pan(&listener));
            let changed = s
                .sent
                .is_none_or(|(g, p)| (g - gp.0).abs() > 1.0e-4 || (p - gp.1).abs() > 1.0e-3);
            if changed {
                s.sent = Some(gp);
                self.commands.push(Command::Update {
                    id: s.id,
                    gain: gp.0,
                    pan: gp.1,
                });
            }
        }
    }

    /// The pending commands, oldest first.
    pub fn take_commands(&mut self) -> Vec<Command> {
        std::mem::take(&mut self.commands)
    }

    /// A source by name (tests, logs).
    pub fn source(&self, name: &str) -> Option<&Source> {
        self.sources.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry() -> SoundPoolEntry {
        SoundPoolEntry {
            sound_name: "a.ogg".into(),
            path: "a.ogg".into(),
        }
    }

    #[test]
    fn linear_gain() {
        let mut l = SoundLibrary::new();
        l.new_source(
            false,
            "s",
            &entry(),
            8.0,
            0.0,
            0.0,
            ATTENUATION_LINEAR,
            16.0,
        );
        let s = l.source("s").unwrap();
        assert_eq!(s.attenuation_gain(&l.listener), 0.5);
        l.set_listener_position(8.0, 0.0, 0.0);
        assert_eq!(l.source("s").unwrap().attenuation_gain(&l.listener), 1.0);
        l.set_listener_position(-8.0, 0.0, 0.0);
        assert_eq!(l.source("s").unwrap().attenuation_gain(&l.listener), 0.0);
        l.set_listener_position(8.0, 30.0, 0.0);
        assert_eq!(l.source("s").unwrap().attenuation_gain(&l.listener), 0.0);
        // none: always 1
        l.new_source(
            false,
            "n",
            &entry(),
            1000.0,
            0.0,
            0.0,
            ATTENUATION_NONE,
            0.0,
        );
        assert_eq!(l.source("n").unwrap().attenuation_gain(&l.listener), 1.0);
    }

    #[test]
    fn clamps_and_play_commands() {
        let mut l = SoundLibrary::new();
        l.new_source(
            false,
            "s",
            &entry(),
            0.0,
            0.0,
            4.0,
            ATTENUATION_LINEAR,
            16.0,
        );
        l.set_pitch("s", 2.6);
        assert_eq!(l.source("s").unwrap().pitch, 2.0);
        l.set_pitch("s", 0.1);
        assert_eq!(l.source("s").unwrap().pitch, 0.5);
        l.set_volume("s", 3.0);
        assert_eq!(l.source("s").unwrap().volume, 1.0);
        l.set_volume("s", 0.5);
        assert!(!l.playing("s"));
        l.play("s");
        assert!(l.playing("s"));
        l.play("s");
        let c = l.take_commands();
        assert_eq!(c.len(), 1);
        // default listener at the origin facing -z; the source at +z is
        // behind it, centred
        assert_eq!(
            c[0],
            Command::Play {
                id: 1,
                entry: entry(),
                streaming: false,
                gain: 0.75 * 0.5,
                pan: 0.0,
                pitch: 0.5
            }
        );
        // a new source of the same name stops the old one
        l.new_source(
            false,
            "s",
            &entry(),
            0.0,
            0.0,
            0.0,
            ATTENUATION_LINEAR,
            16.0,
        );
        assert_eq!(l.take_commands(), vec![Command::Stop { id: 1 }]);
        assert!(!l.playing("s"));
    }

    #[test]
    fn listener_moves_update_gain() {
        let mut l = SoundLibrary::new();
        l.new_source(
            false,
            "s",
            &entry(),
            0.0,
            0.0,
            0.0,
            ATTENUATION_LINEAR,
            16.0,
        );
        l.play("s");
        l.take_commands();
        l.frame_update();
        assert!(l.take_commands().is_empty());
        l.set_listener_position(4.0, 0.0, 0.0);
        l.frame_update();
        let c = l.take_commands();
        let Command::Update { id: 1, gain, .. } = c[0] else {
            panic!("{c:?}")
        };
        assert_eq!(gain, 0.75);
        l.on_finished(1);
        assert!(!l.playing("s"));
        l.frame_update();
        assert!(l.take_commands().is_empty());
    }

    /// The listener orientation `func_338_a` sets for yaw 0 (facing +z,
    /// south): the right ear points to -x (west).
    #[test]
    fn panning_follows_the_listener() {
        let mut l = SoundLibrary::new();
        l.set_listener_orientation(0.0, 0.0, 1.0, 0.0, 1.0, 0.0);
        l.new_source(
            false,
            "w",
            &entry(),
            -5.0,
            0.0,
            0.0,
            ATTENUATION_LINEAR,
            16.0,
        );
        l.new_source(
            false,
            "e",
            &entry(),
            5.0,
            0.0,
            0.0,
            ATTENUATION_LINEAR,
            16.0,
        );
        l.new_source(
            false,
            "f",
            &entry(),
            0.0,
            0.0,
            5.0,
            ATTENUATION_LINEAR,
            16.0,
        );
        l.new_source(
            false,
            "u",
            &entry(),
            0.0,
            5.0,
            0.0,
            ATTENUATION_LINEAR,
            16.0,
        );
        let p = |n| l.source(n).unwrap().pan(&l.listener);
        assert_eq!(p("w"), 1.0);
        assert_eq!(p("e"), -1.0);
        assert_eq!(p("f"), 0.0);
        assert_eq!(p("u"), 0.0);
    }

    #[test]
    fn music_and_cleanup() {
        let mut l = SoundLibrary::new();
        l.background_music("BgMusic", &entry());
        assert!(l.playing("BgMusic"));
        let s = l.source("BgMusic").unwrap();
        assert!(s.streaming);
        assert_eq!(s.attenuation, ATTENUATION_NONE);
        l.set_volume("BgMusic", 0.25);
        let c = l.take_commands();
        assert!(matches!(c[0], Command::Play { gain, .. } if gain == 1.0));
        assert!(matches!(c[1], Command::Update { gain, .. } if gain == 0.25));
        l.stop("BgMusic");
        assert!(!l.playing("BgMusic"));
        assert_eq!(l.take_commands(), vec![Command::Stop { id: 1 }]);
        l.background_music("BgMusic", &entry());
        l.take_commands();
        l.cleanup();
        assert_eq!(l.take_commands(), vec![Command::Stop { id: 2 }]);
        assert!(l.source("BgMusic").is_none());
    }
}
