//! Audio (WP-3.8): `SoundManager` (SoundManager.java) as a Bevy resource,
//! its three `SoundPool`s ([`pool`]), `CodecMus` ([`mus`]), the sound half
//! of `RenderGlobal.playAuxSFX` ([`aux_sfx`]) and of `RenderGlobal.playSound`
//! / `playRecord`, on `bevy_kira_audio` through a model of the paulscode
//! library vanilla used ([`library`]).
//!
//! Inputs, each frame after [`crate::GameFrameSet`]:
//! - [`ClientEvent::Settings`]`(SoundOptionsChanged)` →
//!   [`SoundManager::on_sound_options_changed`];
//! - [`ClientSound`] (the shell's direct `sndManager` calls);
//! - [`WorldEventMsg`]: `PlaySound` (→ `RenderGlobal.playSound`),
//!   `PlayRecord` (→ `RenderGlobal.playRecord`), `PlayAuxSfx` (→
//!   [`aux_sfx::aux_sfx_sound`]).
//!
//! Other client parts call the resource directly, e.g. a GUI button's
//! `mc.sndManager.playSoundFX("random.click", 1.0F, 1.0F)` (GuiScreen.java:58)
//! is `ResMut<SoundManager>` + [`SoundManager::play_sound_fx`]. The library
//! commands are turned into kira plays in `PostUpdate`, so any system of the
//! frame may call it.
//!
//! `RenderGlobal.playRecord`'s other half, `ingameGUI.setRecordPlayingMessage`,
//! is published as [`RecordPlayingMessage`] for the HUD (WP-3.6).
//!
//! Dev aid: with `MC_SOUND_LOG=1` in the environment every play decision is
//! logged at info level (`sound: ...`), as are the misses.
//!
//! Statics: `SoundManager.sndSystem` and `loaded` are static in Java, so they
//! outlive `forceReload`'s `new SoundManager()`; here they are fields that
//! [`SoundManager::force_reload`] carries over.

pub mod aux_sfx;
pub mod library;
pub mod mus;
pub mod pool;

use std::collections::BTreeMap;
use std::io::Cursor;
use std::path::Path;

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures::check_ready};
use bevy_kira_audio::prelude::{Decibels, StaticSoundData};
use bevy_kira_audio::{
    Audio, AudioControl, AudioInstance, AudioPlugin, AudioSource, AudioSystemSet, AudioTween,
    PlaybackState,
};
use mc_core::JavaRandom;
use mc_sim::world::events::WorldEvent;

use crate::assets::{ResourcesDir, SoundPoolKind};
use crate::events::{ClientEvent, ClientSound, WorldEventMsg};
use crate::game::Game;
use crate::settings::SettingsEffect;
use crate::{GameFrameSet, clock};

use self::aux_sfx::{AuxSound, aux_sfx_sound};
use self::library::{ATTENUATION_LINEAR, Command, SoundLibrary};
use self::pool::{SoundPool, SoundPoolEntry};

/// The source name of the background music.
// src: SoundManager.java:65
pub const BG_MUSIC: &str = "BgMusic";
/// The source name of a playing record.
// src: SoundManager.java:135
pub const STREAMING: &str = "streaming";

/// What `SoundManager` reads and writes of `GameSettings`
/// (`options.soundVolume`, `options.musicVolume`, `options.saveOptions()`).
pub trait SoundOptionsHost {
    /// `(soundVolume, musicVolume)`.
    fn volumes(&self) -> (f32, f32);
    /// Sets both.
    fn set_volumes(&mut self, sound: f32, music: f32);
    /// `saveOptions()`.
    fn save_options(&mut self);
}

impl SoundOptionsHost for Game {
    fn volumes(&self) -> (f32, f32) {
        (
            self.game_settings.sound_volume,
            self.game_settings.music_volume,
        )
    }

    fn set_volumes(&mut self, sound: f32, music: f32) {
        self.game_settings.sound_volume = sound;
        self.game_settings.music_volume = music;
    }

    fn save_options(&mut self) {
        Game::save_options(self);
    }
}

/// `SoundManager` (module docs).
#[derive(Resource, Debug)]
pub struct SoundManager {
    /// `soundPoolSounds` (`sound/`, `newsound/`).
    pub sound_pool_sounds: SoundPool,
    /// `soundPoolStreaming` (`streaming/`: records).
    pub sound_pool_streaming: SoundPool,
    /// `soundPoolMusic` (`music/`, `newmusic/`).
    pub sound_pool_music: SoundPool,
    /// `field_587_e`: the counter naming the sources `sound_0` … `sound_255`.
    // src: SoundManager.java:16
    pub field_587_e: i32,
    /// `options.soundVolume`, `options.musicVolume`: a copy of the game's,
    /// refreshed every frame before the shell's calls and by
    /// [`Self::load_sound_settings`] / [`Self::on_sound_options_changed`].
    pub sound_volume: f32,
    /// See [`Self::sound_volume`].
    pub music_volume: f32,
    /// `rand`.
    rand: JavaRandom,
    /// `ticksBeforeMusic`.
    pub ticks_before_music: i32,
    /// `loaded` (static).
    pub loaded: bool,
    /// `sndSystem` (static).
    pub snd_system: Option<SoundLibrary>,
    /// Stand-in for `worldObj.rand` in `playAuxSFX` ([`aux_sfx`] docs).
    aux_world_rand: JavaRandom,
    /// The client's `Math.random()` stream for `playAuxSFX` 1003.
    aux_math_random: JavaRandom,
    /// `MC_SOUND_LOG=1`.
    pub log: bool,
}

impl SoundManager {
    /// `new SoundManager()` with the seeds of its four `new Random()`s, in
    /// field order: the sounds, streaming and music pools, then `rand`;
    /// the constructor draws `ticksBeforeMusic = rand.nextInt(12000)`.
    // src: SoundManager.java:13-24
    pub fn with_seeds(seeds: [i64; 4]) -> Self {
        let mut rand = JavaRandom::new(seeds[3]);
        let ticks_before_music = rand.next_int_bounded(12000);
        SoundManager {
            sound_pool_sounds: SoundPool::new(seeds[0]),
            sound_pool_streaming: SoundPool::new(seeds[1]),
            sound_pool_music: SoundPool::new(seeds[2]),
            field_587_e: 0,
            sound_volume: 1.0,
            music_volume: 1.0,
            rand,
            ticks_before_music,
            loaded: false,
            snd_system: None,
            aux_world_rand: JavaRandom::new(seeds[3] ^ 0x5DEECE66D),
            aux_math_random: JavaRandom::new(seeds[3].rotate_left(17)),
            log: false,
        }
    }

    /// `new SoundManager()` with clock-seeded `Random`s.
    pub fn new() -> Self {
        let mut m = Self::with_seeds([
            clock::new_random_seed(),
            clock::new_random_seed(),
            clock::new_random_seed(),
            clock::new_random_seed(),
        ]);
        m.aux_world_rand = JavaRandom::new(clock::new_random_seed());
        m.aux_math_random = JavaRandom::new(clock::new_random_seed());
        m
    }

    fn sync(&mut self, host: &dyn SoundOptionsHost) {
        (self.sound_volume, self.music_volume) = host.volumes();
    }

    /// `loadSoundSettings(options)`.
    // src: SoundManager.java:26-33
    pub fn load_sound_settings(&mut self, host: &mut dyn SoundOptionsHost) {
        self.sound_pool_streaming.field_1657_b = false;
        self.sync(host);
        if !self.loaded && (self.sound_volume != 0.0 || self.music_volume != 0.0) {
            self.try_to_set_library_and_codecs(host);
        }
    }

    /// `tryToSetLibraryAndCodecs()`: the volumes are saved as 0 while the
    /// library starts (a crash there leaves sound off on the next start),
    /// then restored and saved again.
    // src: SoundManager.java:35-56
    fn try_to_set_library_and_codecs(&mut self, host: &mut dyn SoundOptionsHost) {
        let (var1, var2) = host.volumes();
        host.set_volumes(0.0, 0.0);
        host.save_options();
        // SoundSystemConfig.addLibrary / setCodec("ogg"|"mus"|"wav"):
        // kira decodes OGG; .mus goes through `mus::decode_mus` first.
        self.snd_system = Some(SoundLibrary::new());
        host.set_volumes(var1, var2);
        host.save_options();
        self.sync(host);
        self.loaded = true;
    }

    /// `onSoundOptionsChanged()`: music volume 0 stops the music, anything
    /// else sets its volume. (The sound volume only affects later plays.)
    // src: SoundManager.java:58-71
    pub fn on_sound_options_changed(&mut self, host: &mut dyn SoundOptionsHost) {
        self.sync(host);
        if !self.loaded && (self.sound_volume != 0.0 || self.music_volume != 0.0) {
            self.try_to_set_library_and_codecs(host);
        }
        if self.loaded {
            let music = self.music_volume;
            if let Some(s) = self.snd_system.as_mut() {
                if music == 0.0 {
                    s.stop(BG_MUSIC);
                } else {
                    s.set_volume(BG_MUSIC, music);
                }
            }
        }
    }

    /// `closeMinecraft()`.
    // src: SoundManager.java:73-78
    pub fn close_minecraft(&mut self) {
        if self.loaded
            && let Some(s) = self.snd_system.as_mut()
        {
            s.cleanup();
        }
    }

    /// `addSound(name, file)`.
    // src: SoundManager.java:80-82
    pub fn add_sound(&mut self, name: &str, path: &Path) -> Result<(), pool::AddSoundError> {
        self.sound_pool_sounds
            .add_sound(name, path.to_path_buf())
            .map(|_| ())
    }

    /// `addStreaming(name, file)`.
    // src: SoundManager.java:84-86
    pub fn add_streaming(&mut self, name: &str, path: &Path) -> Result<(), pool::AddSoundError> {
        self.sound_pool_streaming
            .add_sound(name, path.to_path_buf())
            .map(|_| ())
    }

    /// `addMusic(name, file)`.
    // src: SoundManager.java:88-90
    pub fn add_music(&mut self, name: &str, path: &Path) -> Result<(), pool::AddSoundError> {
        self.sound_pool_music
            .add_sound(name, path.to_path_buf())
            .map(|_| ())
    }

    /// `playRandomMusicIfReady()`, once per unpaused tick: while neither
    /// music nor a record plays, count `ticksBeforeMusic` down, then start a
    /// random track and wait `12000 + nextInt(12000)` ticks after it.
    // src: SoundManager.java:92-110
    pub fn play_random_music_if_ready(&mut self) {
        if !self.loaded || self.music_volume == 0.0 {
            return;
        }
        let Some(s) = self.snd_system.as_mut() else {
            return;
        };
        if s.playing(BG_MUSIC) || s.playing(STREAMING) {
            return;
        }
        if self.ticks_before_music > 0 {
            self.ticks_before_music -= 1;
            return;
        }
        let Some(var1) = self.sound_pool_music.get_random_sound().cloned() else {
            return;
        };
        // src: SoundManager.java:102
        self.ticks_before_music = self.rand.next_int_bounded(12000) + 12000;
        s.background_music(BG_MUSIC, &var1);
        s.set_volume(BG_MUSIC, self.music_volume);
        s.play(BG_MUSIC);
        if self.log {
            info!(
                "sound: music -> {} vol {} (next in >= {} ticks)",
                var1.sound_name, self.music_volume, self.ticks_before_music
            );
        }
    }

    /// `func_338_a(player, partialTicks)`: the listener at the player's
    /// interpolated position, facing its interpolated yaw (the shell sends
    /// the interpolated pose as [`ClientSound::Listener`]).
    // src: SoundManager.java:112-131
    pub fn func_338_a(&mut self, pose: Option<([f64; 3], f32)>) {
        if !self.loaded || self.sound_volume == 0.0 {
            return;
        }
        let Some(([var4, var6, var8], var3)) = pose else {
            return;
        };
        let var10 = mc_core::math::cos(-var3 * 0.017453292 - 3.1415927);
        let var11 = mc_core::math::sin(-var3 * 0.017453292 - 3.1415927);
        let var12 = -var11;
        let var13 = 0.0;
        let var14 = -var10;
        let var15 = 0.0;
        let var16 = 1.0;
        let var17 = 0.0;
        if let Some(s) = self.snd_system.as_mut() {
            s.set_listener_position(var4 as f32, var6 as f32, var8 as f32);
            s.set_listener_orientation(var12, var13, var14, var15, var16, var17);
        }
    }

    /// `playStreaming(name, x, y, z, volume, pitch)`: stops the playing
    /// record; with a name, starts that record at the position (linear
    /// attenuation over 64 blocks, volume `0.5 × soundVolume`), stopping the
    /// music. `None` only stops.
    // src: SoundManager.java:133-155
    #[allow(clippy::too_many_arguments)]
    pub fn play_streaming(
        &mut self,
        var1: Option<&str>,
        var2: f32,
        var3: f32,
        var4: f32,
        var5: f32,
        _var6: f32,
    ) {
        if !self.loaded || self.sound_volume == 0.0 {
            if self.log && var1.is_some() {
                info!("sound: record {var1:?} skipped (sound off)");
            }
            return;
        }
        let Some(s) = self.snd_system.as_mut() else {
            return;
        };
        let var7 = STREAMING;
        if s.playing(STREAMING) {
            s.stop(STREAMING);
        }
        let Some(name) = var1 else {
            return;
        };
        let var8 = self
            .sound_pool_streaming
            .get_random_sound_from_sound_pool(name)
            .cloned();
        match var8 {
            Some(var8) if var5 > 0.0 => {
                if s.playing(BG_MUSIC) {
                    s.stop(BG_MUSIC);
                }
                let var9 = 16.0_f32;
                s.new_streaming_source(
                    true,
                    var7,
                    &var8,
                    var2,
                    var3,
                    var4,
                    ATTENUATION_LINEAR,
                    var9 * 4.0,
                );
                s.set_volume(var7, 0.5 * self.sound_volume);
                s.play(var7);
                if self.log {
                    info!(
                        "sound: record {name} -> {} vol {} at ({var2}, {var3}, {var4})",
                        var8.sound_name,
                        0.5 * self.sound_volume
                    );
                }
            }
            None if self.log => info!("sound: no such record: {name}"),
            _ => {}
        }
    }

    /// `playSound(name, x, y, z, volume, pitch)`: a positional sound, linear
    /// attenuation over `16` blocks (`16 × volume` when `volume > 1`, which
    /// is then capped at 1 and marks the source as priority).
    // src: SoundManager.java:157-179
    #[allow(clippy::too_many_arguments)]
    pub fn play_sound(
        &mut self,
        var1: &str,
        var2: f32,
        var3: f32,
        var4: f32,
        var5: f32,
        var6: f32,
    ) {
        if !self.loaded || self.sound_volume == 0.0 {
            if self.log {
                info!("sound: {var1} skipped (sound off)");
            }
            return;
        }
        let var7 = self
            .sound_pool_sounds
            .get_random_sound_from_sound_pool(var1)
            .cloned();
        let Some(var7) = var7 else {
            if self.log {
                info!("sound: no such sound: {var1}");
            }
            return;
        };
        if var5 <= 0.0 {
            return;
        }
        let Some(s) = self.snd_system.as_mut() else {
            return;
        };
        self.field_587_e = (self.field_587_e + 1) % 256;
        let var8 = format!("sound_{}", self.field_587_e);
        let mut var9 = 16.0_f32;
        if var5 > 1.0 {
            var9 *= var5;
        }
        s.new_source(
            var5 > 1.0,
            &var8,
            &var7,
            var2,
            var3,
            var4,
            ATTENUATION_LINEAR,
            var9,
        );
        s.set_pitch(&var8, var6);
        let mut var5 = var5;
        if var5 > 1.0 {
            var5 = 1.0;
        }
        s.set_volume(&var8, var5 * self.sound_volume);
        s.play(&var8);
        if self.log {
            info!(
                "sound: {var1} -> {} vol {} pitch {} range {var9} at ({var2}, {var3}, {var4})",
                var7.sound_name,
                var5 * self.sound_volume,
                var6
            );
        }
    }

    /// `playSoundFX(name, volume, pitch)`: a non-positional sound (a source
    /// at the origin with no attenuation, see [`library`]), volume capped at
    /// 1 and quartered.
    // src: SoundManager.java:181-199
    pub fn play_sound_fx(&mut self, var1: &str, var2: f32, var3: f32) {
        if !self.loaded || self.sound_volume == 0.0 {
            if self.log {
                info!("sound: {var1} (fx) skipped (sound off)");
            }
            return;
        }
        let var4 = self
            .sound_pool_sounds
            .get_random_sound_from_sound_pool(var1)
            .cloned();
        let Some(var4) = var4 else {
            if self.log {
                info!("sound: no such sound: {var1}");
            }
            return;
        };
        let Some(s) = self.snd_system.as_mut() else {
            return;
        };
        self.field_587_e = (self.field_587_e + 1) % 256;
        let var5 = format!("sound_{}", self.field_587_e);
        s.new_source(false, &var5, &var4, 0.0, 0.0, 0.0, 0, 0.0);
        let mut var2 = var2;
        if var2 > 1.0 {
            var2 = 1.0;
        }
        var2 *= 0.25;
        s.set_pitch(&var5, var3);
        s.set_volume(&var5, var2 * self.sound_volume);
        s.play(&var5);
        if self.log {
            info!(
                "sound: {var1} -> {} vol {} pitch {var3} fx",
                var4.sound_name,
                var2 * self.sound_volume
            );
        }
    }

    /// `RenderGlobal.playSound(name, x, y, z, volume, pitch)`: plays only
    /// within `16` blocks of the view entity (`16 × volume` when `volume >
    /// 1`). `view_distance_sq` is `renderViewEntity.getDistanceSq(x, y, z)`
    /// (`None` without one; Java would throw).
    // src: RenderGlobal.java:1263-1272
    #[allow(clippy::too_many_arguments)]
    pub fn render_global_play_sound(
        &mut self,
        var1: &str,
        var2: f64,
        var4: f64,
        var6: f64,
        var8: f32,
        var9: f32,
        view_distance_sq: Option<f64>,
    ) {
        let mut var10 = 16.0_f32;
        if var8 > 1.0 {
            var10 *= var8;
        }
        let Some(d) = view_distance_sq else {
            return;
        };
        if d < (var10 * var10) as f64 {
            self.play_sound(var1, var2 as f32, var4 as f32, var6 as f32, var8, var9);
        } else if self.log {
            info!("sound: {var1} out of range ({:.1} > {var10})", d.sqrt());
        }
    }

    /// `RenderGlobal.playRecord(name, x, y, z)`'s sound half (the HUD line is
    /// [`record_playing_message`]).
    // src: RenderGlobal.java:1255-1261
    pub fn render_global_play_record(&mut self, var1: Option<&str>, x: i32, y: i32, z: i32) {
        self.play_streaming(var1, x as f32, y as f32, z as f32, 1.0, 1.0);
    }

    /// `RenderGlobal.playAuxSFX`'s sound half (`player` is unused there).
    pub fn render_global_play_aux_sfx(
        &mut self,
        kind: i32,
        x: i32,
        y: i32,
        z: i32,
        data: i32,
        view_distance_sq: impl Fn(f64, f64, f64) -> Option<f64>,
    ) -> Option<String> {
        match aux_sfx_sound(
            kind,
            x,
            y,
            z,
            data,
            &mut self.aux_world_rand,
            &mut self.aux_math_random,
        )? {
            AuxSound::SoundEffect {
                name,
                x,
                y,
                z,
                volume,
                pitch,
            } => {
                let d = view_distance_sq(x, y, z);
                self.render_global_play_sound(name, x, y, z, volume, pitch, d);
                None
            }
            AuxSound::Direct {
                name,
                x,
                y,
                z,
                volume,
                pitch,
            } => {
                self.play_sound(&name, x, y, z, volume, pitch);
                None
            }
            AuxSound::Record { name, x, y, z } => {
                self.render_global_play_record(name, x, y, z);
                name.map(record_playing_message)
            }
        }
    }

    /// `Minecraft.forceReload()`'s sound part: `new SoundManager()` (the
    /// pools, counter, `rand` and music timer are new; `loaded` and the
    /// library are static and stay), `loadSoundSettings`, and the resources
    /// installed again.
    // src: Minecraft.java:1180-1185
    pub fn force_reload(&mut self, host: &mut dyn SoundOptionsHost, resources: &ResourcesDir) {
        let mut fresh = SoundManager::new();
        fresh.loaded = self.loaded;
        fresh.snd_system = self.snd_system.take();
        fresh.log = self.log;
        *self = fresh;
        self.load_sound_settings(host);
        self.install_resources(resources);
    }

    /// `ThreadDownloadResources.loadResource(resourcesFolder, "")` with
    /// `Minecraft.installResource` (downloading is deferred, D2): every file
    /// under `resources/`, its relative name routed by its first directory.
    /// `File.listFiles` order is unspecified in Java; the names are sorted
    /// here so the pools (and so the random picks) are reproducible.
    // src: ThreadDownloadResources.java:67-82, Minecraft.java:1380-1396
    pub fn install_resources(&mut self, resources: &ResourcesDir) {
        let mut files = Vec::new();
        list_files(resources.root(), "", &mut files);
        for (name, path) in files {
            let r = match name.find('/') {
                // installResource: substring(0, indexOf("/")) throws without a '/'
                None => Err(pool::AddSoundError(name.clone())),
                Some(_) => match ResourcesDir::install_resource_kind(&name) {
                    Some((SoundPoolKind::Sound, rest)) => self.add_sound(rest, &path),
                    Some((SoundPoolKind::Streaming, rest)) => self.add_streaming(rest, &path),
                    Some((SoundPoolKind::Music, rest)) => self.add_music(rest, &path),
                    None => Ok(()),
                },
            };
            if r.is_err() {
                // src: ThreadDownloadResources.java:77
                info!("Failed to add {name}");
            }
        }
        if self.log {
            info!(
                "sound: installed {} sounds ({} names), {} streaming, {} music",
                self.sound_pool_sounds.number_of_sound_pool_entries,
                self.sound_pool_sounds.name_count(),
                self.sound_pool_streaming.number_of_sound_pool_entries,
                self.sound_pool_music.number_of_sound_pool_entries
            );
        }
    }
}

impl Default for SoundManager {
    fn default() -> Self {
        Self::new()
    }
}

/// `ingameGUI.setRecordPlayingMessage("C418 - " + name)`'s text.
// src: RenderGlobal.java:1256-1258
pub fn record_playing_message(name: &str) -> String {
    format!("C418 - {name}")
}

/// `RenderGlobal.playRecord`'s HUD half: `ingameGUI.setRecordPlayingMessage`
/// with this text (WP-3.6 shows it).
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct RecordPlayingMessage(pub String);

/// `loadResource`'s recursion, names sorted (see
/// [`SoundManager::install_resources`]).
#[allow(clippy::disallowed_methods)]
fn list_files(dir: &Path, prefix: &str, out: &mut Vec<(String, std::path::PathBuf)>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = rd.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let name = e.file_name().to_string_lossy().into_owned();
        let path = e.path();
        if path.is_dir() {
            list_files(&path, &format!("{prefix}{name}/"), out);
        } else {
            out.push((format!("{prefix}{name}"), path));
        }
    }
}

// ---- Bevy side -------------------------------------------------------------------------

/// The audio plugin (registered by [`crate::ClientPlugin`]).
pub struct SoundPlugin;

impl Plugin for SoundPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<AudioPlugin>() {
            app.add_plugins(AudioPlugin);
        }
        let mut m = SoundManager::new();
        m.log = std::env::var_os("MC_SOUND_LOG").is_some_and(|v| v == "1");
        app.insert_resource(m)
            .init_resource::<KiraBackend>()
            .add_message::<RecordPlayingMessage>()
            .add_systems(Startup, sound_startup)
            .add_systems(Update, process_sound_messages.after(GameFrameSet))
            .add_systems(
                PostUpdate,
                apply_sound_commands.before(AudioSystemSet::PlayTypedChannels),
            );
    }
}

/// `startGame`'s `sndManager.loadSoundSettings(gameSettings)` and the
/// resource thread's install (Minecraft.java:274, :289-290).
fn sound_startup(
    mut game: NonSendMut<Game>,
    mut m: ResMut<SoundManager>,
    resources: Res<ResourcesDir>,
) {
    m.load_sound_settings(&mut *game);
    m.install_resources(&resources);
}

/// The frame's sound calls (module docs).
fn process_sound_messages(
    mut game: NonSendMut<Game>,
    mut m: ResMut<SoundManager>,
    resources: Res<ResourcesDir>,
    mut client_events: MessageReader<ClientEvent>,
    mut client_sounds: MessageReader<ClientSound>,
    mut world_events: MessageReader<WorldEventMsg>,
    mut record_messages: MessageWriter<RecordPlayingMessage>,
) {
    let m = &mut *m;
    m.sync(&*game);
    for e in client_events.read() {
        if let ClientEvent::Settings(SettingsEffect::SoundOptionsChanged) = e {
            m.on_sound_options_changed(&mut *game);
        }
    }
    for s in client_sounds.read() {
        match s {
            ClientSound::PlaySound {
                name,
                x,
                y,
                z,
                volume,
                pitch,
            } => m.play_sound(name, *x, *y, *z, *volume, *pitch),
            ClientSound::PlaySoundFx {
                name,
                volume,
                pitch,
            } => m.play_sound_fx(name, *volume, *pitch),
            // src: Minecraft.java:1274
            ClientSound::StopStreaming => m.play_streaming(None, 0.0, 0.0, 0.0, 0.0, 0.0),
            ClientSound::Listener { pose } => m.func_338_a(*pose),
            ClientSound::PlayRandomMusicIfReady => m.play_random_music_if_ready(),
            ClientSound::ForceReload => m.force_reload(&mut *game, &resources),
            ClientSound::Close => m.close_minecraft(),
        }
    }
    let game = &*game;
    let dist = |x: f64, y: f64, z: f64| game.render_view().map(|e| e.base.get_distance_sq(x, y, z));
    for WorldEventMsg(e) in world_events.read() {
        match e {
            WorldEvent::PlaySound {
                name,
                x,
                y,
                z,
                volume,
                pitch,
            } => {
                m.render_global_play_sound(name, *x, *y, *z, *volume, *pitch, dist(*x, *y, *z));
            }
            WorldEvent::PlayRecord { name, x, y, z } => {
                if let Some(n) = name {
                    record_messages.write(RecordPlayingMessage(record_playing_message(n)));
                }
                m.render_global_play_record(name.as_deref(), *x, *y, *z);
            }
            WorldEvent::PlayAuxSfx {
                kind,
                x,
                y,
                z,
                data,
                ..
            } => {
                if let Some(msg) = m.render_global_play_aux_sfx(*kind, *x, *y, *z, *data, dist) {
                    record_messages.write(RecordPlayingMessage(msg));
                }
            }
            _ => {}
        }
    }
}

/// A sound the backend has started (or is decoding).
struct Live {
    instance: Option<Handle<AudioInstance>>,
    decoding: Option<Task<Result<Decoded, String>>>,
    mono: bool,
    gain: f32,
    pan: f32,
    pitch: f32,
    queued_frames: u32,
    name: String,
}

/// A decoded file.
struct Decoded {
    sound: StaticSoundData,
    mono: bool,
}

/// The kira side of [`library`]: decoded sounds (cached by `soundName`, as
/// paulscode caches its buffers by identifier), the playing instances, and
/// the output check.
#[derive(Resource, Default)]
pub struct KiraBackend {
    cache: BTreeMap<String, Option<(Handle<AudioSource>, bool)>>,
    live: BTreeMap<u64, Live>,
    /// No audio output: plays never leave the queue (kira could not open a
    /// device). Decisions are still made and logged.
    pub output_dead: bool,
}

/// After this many frames still queued, a play counts as never starting.
const DEAD_OUTPUT_FRAMES: u32 = 300;

/// Linear amplitude → kira decibels (≤ -60 dB is silence).
fn decibels(gain: f32) -> Decibels {
    if gain <= 0.001 {
        Decibels::SILENCE
    } else {
        Decibels((20.0 * gain.log10()).max(Decibels::SILENCE.0))
    }
}

/// Reads and decodes a file (`.mus` through [`mus::decode_mus`]).
#[allow(clippy::disallowed_methods)]
fn decode_file(entry: &SoundPoolEntry) -> Result<Decoded, String> {
    let mut bytes =
        std::fs::read(&entry.path).map_err(|e| format!("{}: {e}", entry.path.display()))?;
    if entry.sound_name.to_ascii_lowercase().ends_with(".mus") {
        let file = entry
            .path
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        bytes = mus::decode_mus(&file, bytes);
    }
    let mono = vorbis_channels(&bytes) == Some(1);
    let sound = StaticSoundData::from_cursor(Cursor::new(bytes))
        .map_err(|e| format!("{}: {e}", entry.path.display()))?;
    Ok(Decoded { sound, mono })
}

/// The channel count of an OGG Vorbis stream's identification header.
pub fn vorbis_channels(bytes: &[u8]) -> Option<u8> {
    let head = &bytes[..bytes.len().min(4096)];
    let i = head.windows(7).position(|w| w == b"\x01vorbis")?;
    // packet type, "vorbis", u32 version, u8 channels
    head.get(i + 11).copied()
}

fn is_playing_state(s: &PlaybackState) -> bool {
    !matches!(s, PlaybackState::Stopped | PlaybackState::Stopping { .. })
}

/// The library's commands into kira (module docs).
fn apply_sound_commands(
    mut m: ResMut<SoundManager>,
    mut be: ResMut<KiraBackend>,
    audio: Res<Audio>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut instances: ResMut<Assets<AudioInstance>>,
) {
    let log = m.log;
    let Some(lib) = m.snd_system.as_mut() else {
        return;
    };
    let be = &mut *be;
    // 1. ended sounds, the output check
    let mut ended = Vec::new();
    for (&id, l) in be.live.iter_mut() {
        let Some(h) = &l.instance else {
            continue;
        };
        let st = audio.state(h);
        if matches!(st, PlaybackState::Queued) {
            l.queued_frames += 1;
            if l.queued_frames > DEAD_OUTPUT_FRAMES && !be.output_dead {
                be.output_dead = true;
                warn!(
                    "sound: no audio output (kira could not open a device?); sounds are decided and logged but not heard"
                );
            }
        } else if !is_playing_state(&st) {
            ended.push(id);
        }
    }
    if be.output_dead {
        ended.extend(be.live.keys().copied());
    }
    for id in ended {
        be.live.remove(&id);
        lib.on_finished(id);
    }
    // 2. decodes that finished
    let mut done = Vec::new();
    for (&id, l) in be.live.iter_mut() {
        if let Some(t) = l.decoding.as_mut()
            && let Some(r) = check_ready(t)
        {
            l.decoding = None;
            done.push((id, r));
        }
    }
    for (id, r) in done {
        match r {
            Ok(d) => {
                let l = be.live.get_mut(&id).expect("live");
                l.mono = d.mono;
                let h = sources.add(AudioSource { sound: d.sound });
                l.instance = Some(start(&audio, h, l));
            }
            Err(e) => {
                warn!("sound: cannot decode {e}");
                be.live.remove(&id);
                lib.on_finished(id);
            }
        }
    }
    // 3. the listener's movement, then the queued commands
    lib.frame_update();
    for c in lib.take_commands() {
        match c {
            Command::Play {
                id,
                entry,
                streaming,
                gain,
                pan,
                pitch,
            } => {
                if be.output_dead {
                    lib.on_finished(id);
                    continue;
                }
                let mut l = Live {
                    instance: None,
                    decoding: None,
                    mono: false,
                    gain,
                    pan,
                    pitch,
                    queued_frames: 0,
                    name: entry.sound_name.clone(),
                };
                if streaming {
                    // music and records: decoded off the main thread, not cached
                    let e = entry.clone();
                    let pool = AsyncComputeTaskPool::get_or_init(Default::default);
                    l.decoding = Some(pool.spawn(async move { decode_file(&e) }));
                } else {
                    let cached =
                        be.cache.entry(entry.sound_name.clone()).or_insert_with(
                            || match decode_file(&entry) {
                                Ok(d) => {
                                    Some((sources.add(AudioSource { sound: d.sound }), d.mono))
                                }
                                Err(e) => {
                                    warn!("sound: cannot decode {e}");
                                    None
                                }
                            },
                        );
                    let Some((h, mono)) = cached.clone() else {
                        lib.on_finished(id);
                        continue;
                    };
                    l.mono = mono;
                    l.instance = Some(start(&audio, h, &l));
                }
                be.live.insert(id, l);
            }
            Command::Update { id, gain, pan } => {
                if let Some(l) = be.live.get_mut(&id) {
                    l.gain = gain;
                    l.pan = pan;
                    if let Some(h) = &l.instance
                        && let Some(mut i) = instances.get_mut(h)
                    {
                        i.set_decibels(decibels(gain), AudioTween::default());
                        if l.mono {
                            i.set_panning(pan, AudioTween::default());
                        }
                    }
                }
            }
            Command::Stop { id } => {
                if let Some(l) = be.live.remove(&id) {
                    if let Some(h) = &l.instance
                        && let Some(mut i) = instances.get_mut(h)
                    {
                        i.stop(AudioTween::default());
                    }
                    if log {
                        info!("sound: stop {}", l.name);
                    }
                }
            }
        }
    }
}

/// Queues the kira play of a sound with its current gain, pan and pitch.
fn start(audio: &Audio, h: Handle<AudioSource>, l: &Live) -> Handle<AudioInstance> {
    let mut cmd = audio.play(h);
    cmd.with_volume(decibels(l.gain))
        .with_playback_rate(l.pitch as f64);
    if l.mono {
        cmd.with_panning(l.pan);
    }
    cmd.handle()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[derive(Default)]
    struct Host {
        sound: f32,
        music: f32,
        saves: Vec<(f32, f32)>,
    }

    impl SoundOptionsHost for Host {
        fn volumes(&self) -> (f32, f32) {
            (self.sound, self.music)
        }
        fn set_volumes(&mut self, sound: f32, music: f32) {
            self.sound = sound;
            self.music = music;
        }
        fn save_options(&mut self) {
            self.saves.push((self.sound, self.music));
        }
    }

    fn host(sound: f32, music: f32) -> Host {
        Host {
            sound,
            music,
            ..Default::default()
        }
    }

    fn manager(h: &mut Host) -> SoundManager {
        let mut m = SoundManager::with_seeds([1, 2, 3, 4]);
        m.load_sound_settings(h);
        for f in ["step/grass1.ogg", "step/grass2.ogg", "random/click.ogg"] {
            m.add_sound(f, &PathBuf::from(f)).unwrap();
        }
        m.add_streaming("13.mus", &PathBuf::from("13.mus")).unwrap();
        m.add_streaming("cat.ogg", &PathBuf::from("cat.ogg"))
            .unwrap();
        m.add_music("calm1.ogg", &PathBuf::from("calm1.ogg"))
            .unwrap();
        m
    }

    fn lib(m: &mut SoundManager) -> &mut SoundLibrary {
        m.snd_system.as_mut().unwrap()
    }

    #[test]
    fn load_settings_saves_zero_then_restores() {
        let mut h = host(0.7, 0.3);
        let m = manager(&mut h);
        assert!(m.loaded);
        assert_eq!(h.saves, vec![(0.0, 0.0), (0.7, 0.3)]);
        assert!(!m.sound_pool_streaming.field_1657_b);
        // both off: nothing loads until a volume is raised
        let mut h = host(0.0, 0.0);
        let mut m = SoundManager::with_seeds([1, 2, 3, 4]);
        m.load_sound_settings(&mut h);
        assert!(!m.loaded && h.saves.is_empty());
        h.music = 0.5;
        m.on_sound_options_changed(&mut h);
        assert!(m.loaded);
    }

    #[test]
    fn ticks_before_music_initial_draw() {
        // new Random(7).nextInt(12000) = 8236, then 9164 (printed by a JVM)
        let m = SoundManager::with_seeds([0, 0, 0, 7]);
        assert_eq!(m.ticks_before_music, 8236);
        let mut r = JavaRandom::new(7);
        r.next_int_bounded(12000);
        assert_eq!(r.next_int_bounded(12000), 9164);
    }

    #[test]
    fn play_sound_arithmetic() {
        let mut h = host(0.5, 1.0);
        let mut m = manager(&mut h);
        m.play_sound("step.grass", 1.5, 64.5, 2.5, 0.25, 0.8);
        let s = lib(&mut m).source("sound_1").unwrap().clone();
        assert_eq!(s.position, [1.5, 64.5, 2.5]);
        assert_eq!(s.attenuation, ATTENUATION_LINEAR);
        assert_eq!(s.dist_or_roll, 16.0);
        assert_eq!(s.pitch, 0.8);
        assert_eq!(s.volume, 0.25 * 0.5);
        // volume > 1: range 16 × volume, volume capped at 1
        m.play_sound("step.grass", 0.0, 0.0, 0.0, 4.0, 3.0);
        let s = lib(&mut m).source("sound_2").unwrap().clone();
        assert_eq!(s.dist_or_roll, 64.0);
        assert_eq!(s.volume, 0.5);
        assert_eq!(s.pitch, 2.0);
        // volume 0 and misses: no source, no counter step
        m.play_sound("step.grass", 0.0, 0.0, 0.0, 0.0, 1.0);
        m.play_sound("step.nothing", 0.0, 0.0, 0.0, 1.0, 1.0);
        assert_eq!(m.field_587_e, 2);
        // the counter wraps at 256
        m.field_587_e = 255;
        m.play_sound("step.grass", 0.0, 0.0, 0.0, 1.0, 1.0);
        assert!(lib(&mut m).playing("sound_0"));
    }

    /// The variant pick draws on the pool's own `Random`, even for a
    /// volume-0 call (the pick comes before the `var5 > 0` check).
    #[test]
    fn variant_pick_order() {
        let mut h = host(1.0, 1.0);
        let mut m = manager(&mut h);
        let mut r = JavaRandom::new(1);
        for i in 0..20 {
            let want = ["step/grass1.ogg", "step/grass2.ogg"][r.next_int_bounded(2) as usize];
            let vol = if i % 3 == 0 { 0.0 } else { 1.0 };
            m.play_sound("step.grass", 0.0, 0.0, 0.0, vol, 1.0);
            if vol > 0.0 {
                let n = format!("sound_{}", m.field_587_e);
                assert_eq!(lib(&mut m).source(&n).unwrap().entry.sound_name, want);
            }
        }
    }

    #[test]
    fn sound_off_returns_early() {
        let mut h = host(1.0, 1.0);
        let mut m = manager(&mut h);
        h.sound = 0.0;
        m.sync(&h);
        m.play_sound("step.grass", 0.0, 0.0, 0.0, 1.0, 1.0);
        m.play_sound_fx("random.click", 1.0, 1.0);
        m.play_streaming(Some("13"), 0.0, 0.0, 0.0, 1.0, 1.0);
        assert_eq!(m.field_587_e, 0);
        assert!(lib(&mut m).take_commands().is_empty());
    }

    #[test]
    fn play_sound_fx_arithmetic() {
        let mut h = host(0.8, 1.0);
        let mut m = manager(&mut h);
        m.play_sound_fx("random.click", 3.0, 1.0);
        let s = lib(&mut m).source("sound_1").unwrap().clone();
        assert_eq!(s.attenuation, 0);
        assert_eq!(s.position, [0.0; 3]);
        assert_eq!(s.volume, 0.25 * 0.8);
        m.play_sound_fx("random.click", 0.5, 0.2);
        let s = lib(&mut m).source("sound_2").unwrap().clone();
        assert_eq!(s.volume, 0.5 * 0.25 * 0.8);
        assert_eq!(s.pitch, 0.5);
    }

    #[test]
    fn render_global_range_check() {
        let mut h = host(1.0, 1.0);
        let mut m = manager(&mut h);
        m.render_global_play_sound("step.grass", 0.0, 0.0, 0.0, 1.0, 1.0, Some(255.9));
        assert_eq!(m.field_587_e, 1);
        m.render_global_play_sound("step.grass", 0.0, 0.0, 0.0, 1.0, 1.0, Some(256.0));
        assert_eq!(m.field_587_e, 1);
        // volume 2: range 32
        m.render_global_play_sound("step.grass", 0.0, 0.0, 0.0, 2.0, 1.0, Some(1023.0));
        assert_eq!(m.field_587_e, 2);
        m.render_global_play_sound("step.grass", 0.0, 0.0, 0.0, 1.0, 1.0, None);
        assert_eq!(m.field_587_e, 2);
    }

    #[test]
    fn records_and_music() {
        let mut h = host(0.6, 0.4);
        let mut m = manager(&mut h);
        // music waits for the timer
        m.ticks_before_music = 2;
        m.play_random_music_if_ready();
        m.play_random_music_if_ready();
        assert_eq!(m.ticks_before_music, 0);
        assert!(!lib(&mut m).playing(BG_MUSIC));
        m.play_random_music_if_ready();
        assert!(lib(&mut m).playing(BG_MUSIC));
        assert!((12000..24000).contains(&m.ticks_before_music));
        assert_eq!(lib(&mut m).source(BG_MUSIC).unwrap().volume, 0.4);
        // while music plays, the timer stands still
        let t = m.ticks_before_music;
        m.play_random_music_if_ready();
        assert_eq!(m.ticks_before_music, t);
        // a record stops the music
        m.play_streaming(Some("13"), 9.0, 64.0, -61.0, 1.0, 1.0);
        assert!(!lib(&mut m).playing(BG_MUSIC));
        let s = lib(&mut m).source(STREAMING).unwrap().clone();
        assert_eq!(s.position, [9.0, 64.0, -61.0]);
        assert_eq!(s.dist_or_roll, 64.0);
        assert_eq!(s.attenuation, ATTENUATION_LINEAR);
        assert_eq!(s.volume, 0.5 * 0.6);
        assert!(s.streaming);
        // StopStreaming
        m.play_streaming(None, 0.0, 0.0, 0.0, 0.0, 0.0);
        assert!(!lib(&mut m).playing(STREAMING));
        // music volume 0 stops the music
        m.play_random_music_if_ready();
        m.ticks_before_music = 0;
        m.play_random_music_if_ready();
        assert!(lib(&mut m).playing(BG_MUSIC));
        h.music = 0.0;
        m.on_sound_options_changed(&mut h);
        assert!(!lib(&mut m).playing(BG_MUSIC));
        // and stops the timer
        let t = m.ticks_before_music;
        m.play_random_music_if_ready();
        assert_eq!(m.ticks_before_music, t);
        // other volume: live update
        h.music = 1.0;
        m.on_sound_options_changed(&mut h);
        m.ticks_before_music = 0;
        m.play_random_music_if_ready();
        lib(&mut m).take_commands();
        h.music = 0.5;
        m.on_sound_options_changed(&mut h);
        let c = lib(&mut m).take_commands();
        assert!(matches!(c[..], [Command::Update { gain, .. }] if gain == 0.5));
    }

    /// The listener orientation of `func_338_a` (MathHelper's table).
    #[test]
    fn listener_orientation() {
        let mut h = host(1.0, 1.0);
        let mut m = manager(&mut h);
        m.func_338_a(Some(([1.0, 2.0, 3.0], 0.0)));
        let l = lib(&mut m).listener;
        assert_eq!(l.position, [1.0, 2.0, 3.0]);
        // yaw 0: facing +z
        assert!(l.look_at[0].abs() < 1e-6 && (l.look_at[2] - 1.0).abs() < 1e-6);
        assert_eq!(l.up, [0.0, 1.0, 0.0]);
        m.func_338_a(Some(([0.0; 3], 90.0)));
        // yaw 90: facing -x (west)
        let l = lib(&mut m).listener;
        assert!((l.look_at[0] + 1.0).abs() < 1e-4 && l.look_at[2].abs() < 1e-4);
        // no player: unchanged
        m.func_338_a(None);
        assert_eq!(lib(&mut m).listener, l);
    }

    #[test]
    fn aux_sfx_through_the_manager() {
        let mut h = host(1.0, 1.0);
        let mut m = manager(&mut h);
        let far = |_, _, _| Some(1.0e6);
        let near = |_, _, _| Some(0.0);
        assert_eq!(m.render_global_play_aux_sfx(1000, 0, 0, 0, 0, far), None);
        assert_eq!(m.field_587_e, 0);
        m.render_global_play_aux_sfx(1000, 0, 0, 0, 0, near);
        assert_eq!(m.field_587_e, 1);
        // 2001 has no range check
        m.render_global_play_aux_sfx(2001, 0, 0, 0, 2, far);
        assert_eq!(m.field_587_e, 2);
        assert_eq!(
            m.render_global_play_aux_sfx(1005, 1, 2, 3, 2256, far),
            Some("C418 - 13".to_string())
        );
        assert!(lib(&mut m).playing(STREAMING));
        assert_eq!(m.render_global_play_aux_sfx(1005, 1, 2, 3, 0, far), None);
        assert!(!lib(&mut m).playing(STREAMING));
    }

    #[test]
    fn force_reload_keeps_the_library() {
        let mut h = host(1.0, 1.0);
        let mut m = manager(&mut h);
        m.ticks_before_music = 0;
        m.play_random_music_if_ready();
        let dir = std::env::temp_dir().join("mc_b173_sound_test_empty");
        let _ = std::fs::create_dir_all(&dir);
        let res = ResourcesDir::open(&dir).unwrap();
        m.force_reload(&mut h, &res);
        assert!(m.loaded);
        assert!(lib(&mut m).playing(BG_MUSIC));
        assert_eq!(m.sound_pool_sounds.number_of_sound_pool_entries, 0);
        assert_eq!(m.field_587_e, 0);
    }

    #[test]
    fn decibel_mapping() {
        assert_eq!(decibels(1.0).0, 0.0);
        assert!((decibels(0.5).0 + 6.0206).abs() < 1e-3);
        assert_eq!(decibels(0.0), Decibels::SILENCE);
    }

    #[test]
    fn vorbis_channel_count() {
        let mut b = b"OggS\0\x02xxxxxxxxxxxxxxxxxxxxx\x01vorbis\0\0\0\0\x01\x44\xac".to_vec();
        assert_eq!(vorbis_channels(&b), Some(1));
        let i = b.len() - 3;
        b[i] = 2;
        assert_eq!(vorbis_channels(&b), Some(2));
        assert_eq!(vorbis_channels(b"nothing"), None);
    }

    /// The user's `resources/` when present (D8): the pools install, and the
    /// names the game uses resolve.
    #[test]
    fn real_resources_when_present() {
        let Ok(res) = ResourcesDir::open(&crate::paths::default_resources()) else {
            return;
        };
        let mut h = host(1.0, 1.0);
        let mut m = SoundManager::with_seeds([1, 2, 3, 4]);
        m.load_sound_settings(&mut h);
        m.install_resources(&res);
        for n in [
            "step.grass",
            "step.stone",
            "random.click",
            "random.door_open",
            "random.fizz",
            "random.glass",
        ] {
            assert!(
                !m.sound_pool_sounds.variants(n).is_empty(),
                "{n} not installed"
            );
        }
        // sound/ and newsound/ both feed step.grass
        assert_eq!(m.sound_pool_sounds.variants("step.grass").len(), 8);
        assert_eq!(m.sound_pool_streaming.variants("13").len(), 2);
        assert!(m.sound_pool_music.number_of_sound_pool_entries > 0);
        // a click decodes
        let e = m.sound_pool_sounds.variants("random.click")[0].clone();
        assert!(decode_file(&e).is_ok());
    }
}
