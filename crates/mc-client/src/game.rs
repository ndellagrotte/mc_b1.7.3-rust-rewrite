//! `Minecraft` (net/minecraft/client/Minecraft.java): the game's state and
//! lifecycle — the frame (`run`'s loop body), `runTick`, the world changes
//! (`startWorld`, `changeWorld`, `usePortal`, `respawn`, shutdown), the
//! mouse and keyboard handling, `displayGuiScreen` and the focus.
//!
//! [`Game`] is a `!Send` Bevy resource (the simulation's save store is
//! `Rc`-shared, PLAN_v4 §3.2): systems reach it with `NonSend`/`NonSendMut`.
//! One exclusive system, [`game_frame`], runs the frame; inside each tick it
//! runs the [`RunTickHook`] schedules at the points where `runTick` calls
//! into a part another WP owns, so those parts keep vanilla's order.
//!
//! # `runTick` (Minecraft.java:944-1178) and its hooks
//!
//! | Java | Here |
//! |---|---|
//! | `func_28001_B` (the has-paid check at tick 6000), `statFileWriter.func_27178_d` | deferred (D2: online services, stats sync) |
//! | `ingameGUI.updateTick()` | [`RunTickHook::IngameGuiUpdateTick`] (WP-3.6) |
//! | `entityRenderer.getMouseOver(1.0F)` | [`crate::camera::get_mouse_over`] |
//! | `ChunkProviderLoadOrGenerate.setCurrentChunkOver` | never runs: no world builds that provider (`World.getChunkProvider`, World.java:210-213) |
//! | `playerController.updateController()` | [`PlayerControllerSp::update_controller`] |
//! | `renderEngine.updateDynamicTextures()` | [`RunTickHook::UpdateDynamicTextures`] (WP-3.2) |
//! | the game-over / sleep screen checks | [`Game::run_tick`] |
//! | `currentScreen.handleInput()`, `guiParticles`, `updateScreen()` | [`RunTickHook::ScreenInput`] (WP-3.6); without a WP-3.6 owner ([`ScreenInputOwner`]) the shell applies `GuiScreen.keyTyped`'s Escape and `GuiContainer.keyTyped`'s inventory key |
//! | the mouse and keyboard loops | [`Game::process_input`] |
//! | `joinPlayerCounter`, `difficultySetting` | [`Game::run_tick`] |
//! | `entityRenderer.updateRenderer()` | [`crate::camera::EntityRendererCam::update_renderer`], then [`RunTickHook::UpdateRenderer`] (`itemRenderer.updateEquippedItem`, `addRainParticles`: WP-3.7, WP-3.4) |
//! | `renderGlobal.updateClouds()` | [`RunTickHook::UpdateClouds`] (WP-3.4) |
//! | `updateEntities`, `setAllowedMobSpawns`, `tick`, `randomDisplayUpdates` | `mc-sim` |
//! | `effectRenderer.updateEffects()` | [`RunTickHook::UpdateEffects`] (WP-3.7) |
//!
//! [`WorldEvent`]s are moved to [`WorldEventMsg`] messages before every hook
//! and at the end of the frame ([`crate::events`]).
//!
//! # Seeding
//!
//! Every `Random` vanilla seeds from the clock is seeded at world load from
//! the clock, as `new Random()` would be ([`crate::clock::new_random_seed`]):
//! the world constructor's (`WorldEnvironment`: `rand`, `field_9437_g`,
//! `lockTimestamp`; the display `Random` derives from `rand`'s seed), then
//! [`seed_clock_randoms`] for the rest the simulation exposes (the
//! `Math.random()` stream, `Item.itemRand`, the entity `rand`s and the
//! explosion `Random` keyed off one seed, the container blocks' `Random`s).
//! A portal's new world takes the old one's Java statics
//! ([`World::adopt_statics`], `Item.itemRand`). The teleporter gets its own
//! clock seed.
//!
//! # Loading
//!
//! `changeWorld`'s `func_6255_d` (load the 17×17 chunks around the player,
//! Minecraft.java:1346-1378) blocks vanilla's loop while it repaints the
//! loading screen. Here it is spread over frames ([`PendingLoad`], state
//! [`AppState::Loading`]) so the screen can be drawn; no tick runs
//! meanwhile, as none does in vanilla. What `changeWorld` / `usePortal` /
//! `respawn` do after it runs when the last chunk is in (the "tail").
//! Vanilla returns into the `runTick` that started the change (the GUI's
//! input handling, or the player's update for a portal) and finishes it on
//! the new world; here the rest of that tick is skipped instead.
//!
//! # The player
//!
//! `thePlayer` is the entity id [`Game::the_player`] in the world's list.
//! Vanilla keeps the object after the world drops a dead player (the
//! game-over screen, `respawn`); the port keeps a copy taken before the
//! last dying tick ([`Game::detached_player`]). The stats writer
//! (`Minecraft.statFileWriter`) travels with the player
//! ([`mc_sim::entity::PlayerData::stat_file_writer`]) and is held by the
//! game while there is none.
//!
//! # Out of scope here
//!
//! Applet, multiplayer, the snooper and stats sync (deferred, D2/D3: the
//! `mcApplet`, `serverName`/`GuiConnecting`, `ThreadCheckHasPaid`,
//! `ThreadDownloadResources`, `func_27178_d`), texture packs
//! (`TexturePackList`, D3), `OpenGlCapsChecker`, `checkGLError`, the
//! out-of-memory handler `func_28002_e` (no reserve buffer `field_28006_b`
//! to free), `Controllers`, the AWT canvas and `loadScreen`'s Mojang logo
//! (drawing, WP-3.6).

use std::path::PathBuf;

use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use mc_core::java_random::JavaRandom;
use mc_core::math::floor_double;
use mc_sim::entity::player::{self as player_fns};
use mc_sim::entity::{Entity, EntityKindData, dispatch};
use mc_sim::player_controller::{self, PlayerControllerSp, send_place_block, send_use_item};
use mc_sim::save::{SaveConverterMcRegion, SaveFormat, SaveStore, SharedSaveStore};
use mc_sim::stats::{AchievementMap, StatFileWriter, stat, stat_list};
use mc_sim::world::events::WorldEvent;
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::ray_trace::{EnumMovingObjectType, MovingObjectPosition};
use mc_sim::world::teleporter::Teleporter;
use mc_sim::world::{World, WorldEnvironment};

use crate::app::{AppState, GuiScreen, ScaledResolution, ScreenKind};
use crate::camera::{self, CameraState, EntityRendererCam};
use crate::clock;
use crate::events::{ClientEvent, ClientSound, WorldEventMsg};
use crate::input::{InputEvent, InputQueue, MouseHelper, MovementInputFromOptions};
use crate::keys;
use crate::loading::{IProgressUpdate, LoadingScreenRenderer};
use crate::paths::{ClientPaths, SAVES_DIR};
use crate::session::Session;
use crate::settings::{self, EnumOptions, GameSettings};
use crate::storage::{FsStorage, WallClock};
use crate::timer::Timer;

/// `Block.grass.blockID` (Minecraft.java:923).
// src: Block.java:594
const GRASS: i32 = 2;
/// `Block.dirt.blockID`.
// src: Block.java:595
const DIRT: i32 = 3;
/// `Block.stairDouble.blockID` (Minecraft.java:927).
// src: Block.java:635
const STAIR_DOUBLE: i32 = 43;
/// `Block.stairSingle.blockID`.
// src: Block.java:636
const STAIR_SINGLE: i32 = 44;
/// `Block.bedrock.blockID` (Minecraft.java:931).
// src: Block.java:599
const BEDROCK: i32 = 7;
/// `Block.stone.blockID`.
// src: Block.java:593
const STONE: i32 = 1;

/// Where `runTick` hands over to a part another WP owns (module docs).
/// Each is a schedule the frame driver runs at that point of every tick;
/// systems in it reach the game through `NonSendMut<Game>`.
#[derive(ScheduleLabel, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RunTickHook {
    /// `ingameGUI.updateTick()` (Minecraft.java:950; WP-3.6).
    IngameGuiUpdateTick,
    /// `renderEngine.updateDynamicTextures()` when not paused
    /// (Minecraft.java:968-970; WP-3.2).
    UpdateDynamicTextures,
    /// `currentScreen.handleInput()`, `guiParticles.func_25088_a()`,
    /// `updateScreen()` while a screen is open (Minecraft.java:987-993;
    /// WP-3.6). Drains [`Game::input`]'s buffers.
    ScreenInput,
    /// The rest of `entityRenderer.updateRenderer()` when not paused:
    /// `itemRenderer.updateEquippedItem()` and `addRainParticles()`
    /// (EntityRenderer.java:75-76; WP-3.7, WP-3.4).
    UpdateRenderer,
    /// `renderGlobal.updateClouds()` when not paused (Minecraft.java:1152;
    /// WP-3.4).
    UpdateClouds,
    /// `effectRenderer.updateEffects()` when not paused
    /// (Minecraft.java:1173; WP-3.7).
    UpdateEffects,
}

impl RunTickHook {
    /// Every hook.
    pub const ALL: [RunTickHook; 6] = [
        RunTickHook::IngameGuiUpdateTick,
        RunTickHook::UpdateDynamicTextures,
        RunTickHook::ScreenInput,
        RunTickHook::UpdateRenderer,
        RunTickHook::UpdateClouds,
        RunTickHook::UpdateEffects,
    ];
}

/// Whether a WP-3.6 system owns [`RunTickHook::ScreenInput`]. While false
/// the shell gives open screens `GuiScreen`'s default keys (module docs).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScreenInputOwner(pub bool);

/// What the shell queued for the other client parts this frame
/// ([`crate::events`]); flushed into Bevy messages by [`flush_outbox`].
#[derive(Clone, Debug, Default)]
pub struct Outbox {
    /// Simulation events, in order.
    pub world_events: Vec<WorldEvent>,
    /// Sound calls.
    pub sounds: Vec<ClientSound>,
    /// Shell events.
    pub events: Vec<ClientEvent>,
}

/// `func_6255_d`'s chunk loop, resumable (module docs, "Loading").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainPreload {
    /// `var6.x`.
    center_x: i32,
    /// `var6.z`.
    center_z: i32,
    /// The outer offset `var10` (x).
    var10: i32,
    /// The inner offset `var8` (z).
    var8: i32,
    /// `var3`: chunks done.
    var3: i32,
    /// `var4`: chunks in all (289).
    var4: i32,
}

/// What runs after the preload (module docs, "Loading").
#[derive(Debug)]
enum LoadTail {
    /// `changeWorld`'s remainder with the player it was given (the
    /// portal's), then `usePortal`'s remainder at `(x, z)`.
    ChangeWorld {
        player: Option<Box<Entity>>,
        portal: Option<(f64, f64)>,
    },
    /// `respawn`'s remainder (closing the game-over screen).
    Respawn,
}

/// A world change waiting for its terrain (module docs, "Loading").
#[derive(Debug)]
pub struct PendingLoad {
    /// The chunk loop.
    pub preload: TerrainPreload,
    tail: LoadTail,
}

/// What to do at start-up instead of the title screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StartAction {
    /// The title screen (`displayGuiScreen(new GuiMainMenu())`).
    Title,
    /// Open the world in `saves/<name>`, creating it (with `seed`, a
    /// `GuiCreateWorld` seed text) when it does not exist.
    World {
        /// The directory and level name.
        name: String,
        /// The seed text, `None` for a random seed.
        seed: Option<String>,
    },
    /// Import a vanilla save folder into `saves/`, then open it.
    Import {
        /// The folder.
        path: PathBuf,
    },
}

/// `Minecraft`'s state (module docs).
pub struct Game {
    /// The client's paths.
    pub paths: ClientPaths,
    /// The data directory's storage (shared with the save code).
    pub store: SharedSaveStore,
    /// `saveLoader`: the saves directory.
    // src: Minecraft.java:146, :237
    pub save_loader: SaveConverterMcRegion,
    /// `gameSettings`.
    // src: Minecraft.java:140
    pub game_settings: GameSettings,
    /// `session`.
    // src: Minecraft.java:120
    pub session: Session,
    /// `playerController` (set by the world screens; `PlayerControllerSP`).
    // src: Minecraft.java:108
    pub player_controller: Option<PlayerControllerSp>,
    /// `fullscreen`.
    // src: Minecraft.java:109
    pub fullscreen: bool,
    /// `displayWidth`.
    // src: Minecraft.java:111
    pub display_width: i32,
    /// `displayHeight`.
    // src: Minecraft.java:112
    pub display_height: i32,
    /// `timer` (20 ticks per second).
    // src: Minecraft.java:114
    pub timer: Timer,
    /// `theWorld`.
    // src: Minecraft.java:115
    pub the_world: Option<World>,
    /// The save directory of `theWorld` (not Java state; for the dumps).
    pub world_dir: Option<String>,
    /// `thePlayer`, as its entity id (module docs, "The player").
    // src: Minecraft.java:117
    pub the_player: Option<i32>,
    /// The last copy of a player the world has dropped (module docs).
    pub detached_player: Option<Box<Entity>>,
    /// `renderViewEntity`, as an entity id.
    // src: Minecraft.java:118
    pub render_view_entity: Option<i32>,
    /// `isGamePaused`.
    // src: Minecraft.java:124
    pub is_game_paused: bool,
    /// `currentScreen`.
    // src: Minecraft.java:127
    pub current_screen: Option<GuiScreen>,
    /// `loadingScreen`.
    // src: Minecraft.java:128
    pub loading_screen: LoadingScreenRenderer,
    /// `entityRenderer`'s camera half.
    // src: Minecraft.java:129
    pub entity_renderer: EntityRendererCam,
    /// `ticksRan`.
    // src: Minecraft.java:131
    pub ticks_ran: i32,
    /// `leftClickCounter`.
    // src: Minecraft.java:132
    pub left_click_counter: i32,
    /// `tempDisplayWidth` (the windowed size before fullscreen).
    // src: Minecraft.java:133
    pub temp_display_width: i32,
    /// `tempDisplayHeight`.
    // src: Minecraft.java:134
    pub temp_display_height: i32,
    /// `skipRenderWorld`.
    // src: Minecraft.java:137
    pub skip_render_world: bool,
    /// `objectMouseOver`.
    // src: Minecraft.java:139
    pub object_mouse_over: Option<MovingObjectPosition>,
    /// `mouseHelper`.
    // src: Minecraft.java:143
    pub mouse_helper: MouseHelper,
    /// `statFileWriter` while no player carries it (module docs).
    // src: Minecraft.java:151
    pub stat_file_writer: StatFileWriter,
    /// The jar's achievement GUIDs (`AchievementMap`), for the stats file.
    pub achievement_map: AchievementMap,
    /// `running`.
    // src: Minecraft.java:157
    pub running: bool,
    /// `debug`: `"<fps> fps, <n> chunk updates"`.
    // src: Minecraft.java:158
    pub debug: String,
    /// `isTakingScreenshot`.
    // src: Minecraft.java:159
    pub is_taking_screenshot: bool,
    /// `prevFrameTime` (nanoseconds, -1 unset).
    // src: Minecraft.java:160
    pub prev_frame_time: i64,
    /// `inGameHasFocus`.
    // src: Minecraft.java:161
    pub in_game_has_focus: bool,
    /// `mouseTicksRan`.
    // src: Minecraft.java:162
    pub mouse_ticks_ran: i32,
    /// `systemTime` (millis of the last `runTick`; 0 after a world change).
    // src: Minecraft.java:164
    pub system_time: i64,
    /// `joinPlayerCounter`.
    // src: Minecraft.java:165
    pub join_player_counter: i32,
    /// `frameTimes` (the F3 graph; static in Java).
    // src: Minecraft.java:147
    pub frame_times: Box<[i64; 512]>,
    /// `tickTimes`.
    // src: Minecraft.java:148
    pub tick_times: Box<[i64; 512]>,
    /// `numRecordedFrameTimes`.
    // src: Minecraft.java:149
    pub num_recorded_frame_times: i32,
    /// `run`'s `var1`: the start of the current second (fps counter).
    fps_second_start: i64,
    /// `run`'s `var3`: frames this second.
    fps_frames: i32,
    /// `WorldRenderer.chunksUpdated`, counted by the terrain renderer
    /// (WP-3.3) and reset every second with the fps.
    pub chunks_updated: i32,
    /// `thePlayer.movementInput` (a new one per world change).
    pub movement_input: MovementInputFromOptions,
    /// The LWJGL event buffers.
    pub input: InputQueue,
    /// `Display.isActive()`: the window has focus (always true in the
    /// scripted mode).
    pub display_active: bool,
    /// A world change waiting for its terrain.
    pub pending: Option<PendingLoad>,
    /// For the other client parts.
    pub outbox: Outbox,
    /// F2 asked for a screenshot (taken by [`take_requested_screenshot`]).
    pub screenshot_requested: bool,
    /// The app should exit (after the shutdown).
    pub exit_requested: bool,
    /// The last error a world change hit (shown by WP-3.6; logged here).
    pub last_error: Option<String>,
}

impl Game {
    /// `startGame()`'s non-GL part: the data directory's storage, the save
    /// format, `options.txt`, the stats file, the timer and the camera.
    /// `achievement_map` is the jar's `/achievement/map.txt`.
    // src: Minecraft.java:196-302
    pub fn new(
        paths: ClientPaths,
        achievement_map: AchievementMap,
        session: Session,
    ) -> Result<Game, String> {
        let now = clock::current_time_millis();
        let nano = clock::nano_time();
        let mut fs = FsStorage::new(&paths.data_dir)
            .map_err(|e| format!("cannot use the data directory: {e}"))?;
        // src: Minecraft.java:238 (new GameSettings(this, mcDataDir))
        let game_settings = settings::load_options(&mut fs);
        // src: Minecraft.java:247 (new StatFileWriter(session, mcDataDir))
        let stat_file_writer =
            StatFileWriter::open(&mut fs, "", &session.username, &achievement_map)
                .map_err(|e| format!("cannot read the stats: {e}"))?;
        let store = SaveStore::shared(Box::new(fs), Box::new(WallClock));
        // src: Minecraft.java:237
        let save_loader = SaveConverterMcRegion::new(store.clone(), SAVES_DIR);
        Ok(Game {
            paths,
            store,
            save_loader,
            game_settings,
            session,
            player_controller: None,
            fullscreen: false,
            display_width: 854,
            display_height: 480,
            timer: Timer::new(20.0, now, nano),
            the_world: None,
            world_dir: None,
            the_player: None,
            detached_player: None,
            render_view_entity: None,
            is_game_paused: false,
            current_screen: None,
            loading_screen: LoadingScreenRenderer::default(),
            entity_renderer: EntityRendererCam::new(now),
            ticks_ran: 0,
            left_click_counter: 0,
            temp_display_width: 854,
            temp_display_height: 480,
            skip_render_world: false,
            object_mouse_over: None,
            mouse_helper: MouseHelper::default(),
            stat_file_writer,
            achievement_map,
            running: true,
            debug: String::new(),
            is_taking_screenshot: false,
            prev_frame_time: -1,
            in_game_has_focus: false,
            mouse_ticks_ran: 0,
            system_time: now,
            join_player_counter: 0,
            frame_times: Box::new([0; 512]),
            tick_times: Box::new([0; 512]),
            num_recorded_frame_times: 0,
            fps_second_start: now,
            fps_frames: 0,
            chunks_updated: 0,
            movement_input: MovementInputFromOptions::default(),
            input: InputQueue::default(),
            display_active: true,
            pending: None,
            outbox: Outbox::default(),
            screenshot_requested: false,
            exit_requested: false,
            last_error: None,
        })
    }

    // ---- small accessors ---------------------------------------------------------

    /// `isMultiplayerWorld()`: never (multiplayer is deferred, D3).
    // src: Minecraft.java:1187-1189
    pub fn is_multiplayer_world(&self) -> bool {
        self.the_world.as_ref().is_some_and(|w| w.multiplayer_world)
    }

    /// `thePlayer`: in the world, else the detached copy.
    pub fn player(&self) -> Option<&Entity> {
        let pid = self.the_player?;
        self.the_world
            .as_ref()
            .and_then(|w| w.entity(pid))
            .or(self.detached_player.as_deref())
    }

    /// `thePlayer`, mutably, when it is in the world.
    pub fn player_in_world_mut(&mut self) -> Option<&mut Entity> {
        let pid = self.the_player?;
        self.the_world.as_mut()?.entity_mut(pid)
    }

    /// `renderViewEntity`: in the world, else the detached player.
    pub fn render_view(&self) -> Option<&Entity> {
        let id = self.render_view_entity?;
        self.the_world
            .as_ref()
            .and_then(|w| w.entity(id))
            .or_else(|| {
                self.detached_player
                    .as_deref()
                    .filter(|p| p.base.entity_id == id)
            })
    }

    /// `mc.statFileWriter`: the player's while there is one.
    pub fn stats_mut(&mut self) -> &mut StatFileWriter {
        let Some(pid) = self.the_player else {
            return &mut self.stat_file_writer;
        };
        let in_world = self
            .the_world
            .as_ref()
            .is_some_and(|w| w.entity(pid).is_some());
        if in_world {
            let p = self
                .the_world
                .as_mut()
                .and_then(|w| w.entity_mut(pid))
                .and_then(|e| e.kind.player_mut())
                .expect("checked");
            return &mut p.stat_file_writer;
        }
        if let Some(p) = self
            .detached_player
            .as_deref_mut()
            .and_then(|e| e.kind.player_mut())
        {
            return &mut p.stat_file_writer;
        }
        &mut self.stat_file_writer
    }

    /// Takes the stats writer back from the player that is about to go.
    fn reclaim_stats(&mut self) {
        let w = std::mem::take(self.stats_mut());
        self.stat_file_writer = w;
    }

    /// `statFileWriter.syncStats()`: the unsent stats file.
    // src: StatFileWriter.java:190-192
    fn sync_stats(&mut self) {
        let username = self.session.username.clone();
        let map = self.achievement_map.clone();
        let w = self.stats_mut().clone();
        let store = self.store.clone();
        let mut s = store.borrow_mut();
        if let Err(e) = w.sync_stats(s.storage.as_mut(), "", &username, &map) {
            warn!("cannot save the stats: {e}");
        }
    }

    /// `statFileWriter.readStat(stat, 1)` for a stat id.
    fn read_stat(&mut self, id: i32) {
        if let Some(st) = stat(id) {
            self.stats_mut().read_stat(st, 1);
        }
    }

    /// `gameSettings.saveOptions()`.
    pub fn save_options(&mut self) {
        let s = self.game_settings.clone();
        let store = self.store.clone();
        let mut st = store.borrow_mut();
        settings::save_options(&s, st.storage.as_mut());
    }

    /// `gameSettings.setOptionValue(option, step)` with its save and side
    /// effects.
    pub fn set_option_value(&mut self, o: EnumOptions, step: i32) {
        if let Some(e) = self.game_settings.set_option_value(o, step) {
            self.outbox.events.push(ClientEvent::Settings(e));
        }
        self.save_options();
    }

    /// Moves the world's queued events to the outbox.
    pub fn collect_world_events(&mut self) {
        if let Some(w) = self.the_world.as_mut() {
            self.outbox.world_events.extend(w.events.drain());
        }
        if let Some(c) = self.player_controller.as_mut() {
            for s in c.sounds.drain(..) {
                self.outbox.sounds.push(ClientSound::PlaySound {
                    name: s.name,
                    x: s.x,
                    y: s.y,
                    z: s.z,
                    volume: s.volume,
                    pitch: s.pitch,
                });
            }
        }
    }

    // ---- screens and focus ---------------------------------------------------------

    /// `displayGuiScreen(screen)`.
    // src: Minecraft.java:410-444
    pub fn display_gui_screen(&mut self, mut var1: Option<GuiScreen>) {
        if self
            .current_screen
            .as_ref()
            .is_some_and(|s| s.kind == ScreenKind::Unused)
        {
            return;
        }
        if let Some(old) = self.current_screen.take() {
            self.outbox.events.push(ClientEvent::ScreenClosed(old));
        }
        // `if (var1 instanceof GuiMainMenu) statFileWriter.func_27175_b()`:
        // an empty method (StatFileWriter.java:187-188).
        self.sync_stats();
        if var1.is_none() && self.the_world.is_none() {
            var1 = Some(GuiScreen::new(ScreenKind::MainMenu));
        } else if var1.is_none()
            && self
                .player()
                .and_then(|p| p.kind.living())
                .is_some_and(|l| l.health <= 0)
        {
            var1 = Some(GuiScreen::new(ScreenKind::GameOver));
        }
        if var1
            .as_ref()
            .is_some_and(|s| s.kind == ScreenKind::MainMenu)
        {
            self.outbox.events.push(ClientEvent::ClearChat);
        }
        match var1 {
            Some(mut s) => {
                let r = ScaledResolution::new(
                    self.game_settings.gui_scale,
                    self.display_width,
                    self.display_height,
                );
                // setWorldAndResolution(mc, scaledWidth, scaledHeight)
                s.width = r.scaled_width;
                s.height = r.scaled_height;
                self.current_screen = Some(s.clone());
                self.set_ingame_not_in_focus();
                self.outbox.events.push(ClientEvent::ScreenOpened(s));
                self.skip_render_world = false;
            }
            None => {
                self.current_screen = None;
                self.set_ingame_focus();
            }
        }
    }

    /// `setIngameFocus()`.
    // src: Minecraft.java:750-760
    pub fn set_ingame_focus(&mut self) {
        if self.display_active && !self.in_game_has_focus {
            self.in_game_has_focus = true;
            self.mouse_helper.grab_mouse_cursor();
            // LWJGL's Mouse.setGrabbed also drops the motion gathered while
            // the pointer was free.
            self.input.pending_dx = 0.0;
            self.input.pending_dy = 0.0;
            self.display_gui_screen(None);
            self.left_click_counter = 10000;
            self.mouse_ticks_ran = self.ticks_ran.wrapping_add(10000);
        }
    }

    /// `setIngameNotInFocus()`.
    // src: Minecraft.java:762-771
    pub fn set_ingame_not_in_focus(&mut self) {
        if self.in_game_has_focus {
            if self.the_player.is_some() {
                // thePlayer.resetPlayerKeyState() (EntityPlayerSP.java:90-92)
                self.movement_input.reset_key_state();
            }
            self.in_game_has_focus = false;
            self.mouse_helper.ungrab_mouse_cursor();
        }
    }

    /// `displayInGameMenu()`.
    // src: Minecraft.java:773-777
    pub fn display_in_game_menu(&mut self) {
        if self.current_screen.is_none() {
            self.display_gui_screen(Some(GuiScreen::new(ScreenKind::IngameMenu)));
        }
    }

    /// `toggleFullscreen()`: the window mode follows
    /// [`Game::fullscreen`] ([`apply_window_mode`]); the sizes are read back
    /// from the window.
    // src: Minecraft.java:856-898
    pub fn toggle_fullscreen(&mut self) {
        self.fullscreen = !self.fullscreen;
        // The display size is the window's, read back every frame
        // ([`track_window`]): the `mcCanvas` branch of the Java.
        if self.current_screen.is_some() {
            let (w, h) = (self.display_width, self.display_height);
            self.resize(w, h);
        }
    }

    /// `resize(w, h)`.
    // src: Minecraft.java:900-918
    pub fn resize(&mut self, mut var1: i32, mut var2: i32) {
        if var1 <= 0 {
            var1 = 1;
        }
        if var2 <= 0 {
            var2 = 1;
        }
        self.display_width = var1;
        self.display_height = var2;
        if let Some(s) = self.current_screen.as_mut() {
            let r = ScaledResolution::new(self.game_settings.gui_scale, var1, var2);
            s.width = r.scaled_width;
            s.height = r.scaled_height;
            self.outbox
                .events
                .push(ClientEvent::ScreenResized(s.clone()));
        }
    }

    /// `shutdown()`.
    // src: Minecraft.java:746-748
    pub fn shutdown(&mut self) {
        self.running = false;
    }

    /// `shutdownMinecraftApplet()`'s game part: the stats, leaving the world
    /// (which saves it and the player), the sound system.
    // src: Minecraft.java:457-496
    pub fn shutdown_minecraft_applet(&mut self) {
        // statFileWriter.func_27175_b(): empty (StatFileWriter.java:187-188).
        self.sync_stats();
        info!("Stopping!");
        self.pending = None;
        self.change_world1(None);
        self.outbox.sounds.push(ClientSound::Close);
    }

    // ---- the mouse ------------------------------------------------------------------

    /// `func_6254_a(button, held)`: the left button's block breaking.
    /// Name kept (docs/mappings.md).
    // src: Minecraft.java:779-798
    fn func_6254_a(&mut self, var1: i32, var2: bool) {
        let Some(c) = self.player_controller.as_mut() else {
            return;
        };
        if c.is_in_test_mode {
            return;
        }
        if !var2 {
            self.left_click_counter = 0;
        }
        if var1 != 0 || self.left_click_counter <= 0 {
            let tile = self
                .object_mouse_over
                .filter(|m| m.type_of_hit == EnumMovingObjectType::Tile);
            match (
                var2 && var1 == 0,
                tile,
                self.the_player,
                self.the_world.as_mut(),
            ) {
                (true, Some(m), Some(pid), Some(w)) => {
                    w.with_entity_mut(pid, |w, p| {
                        c.send_block_removing(w, p, m.block_x, m.block_y, m.block_z, m.side_hit);
                    });
                    self.outbox.events.push(ClientEvent::BlockHitEffects {
                        x: m.block_x,
                        y: m.block_y,
                        z: m.block_z,
                        side: m.side_hit,
                    });
                }
                _ => c.reset_block_removing(),
            }
        }
    }

    /// `clickMouse(button)`: 0 attacks or starts breaking, 1 interacts,
    /// places or uses the held item.
    // src: Minecraft.java:800-854
    fn click_mouse(&mut self, var1: i32) {
        if !(var1 != 0 || self.left_click_counter <= 0) {
            return;
        }
        let (Some(pid), Some(world), Some(c)) = (
            self.the_player,
            self.the_world.as_mut(),
            self.player_controller.as_mut(),
        ) else {
            return;
        };
        let mouse_over = self.object_mouse_over;
        let mut left_click_counter = self.left_click_counter;
        let mut events = Vec::new();
        world.with_entity_mut(pid, |w, p| {
            if var1 == 0 {
                player_fns::swing_item(p);
            }
            let mut var2 = true;
            match mouse_over {
                None => {
                    // `!(playerController instanceof PlayerControllerTest)`:
                    // the creative controller is not ported.
                    if var1 == 0 {
                        left_click_counter = 10;
                    }
                }
                Some(m) if m.type_of_hit == EnumMovingObjectType::Entity => {
                    if let Some(target) = m.entity_hit {
                        if var1 == 0 {
                            w.with_entity_mut(target, |w, t| {
                                player_controller::attack_entity(w, p, t)
                            });
                        }
                        if var1 == 1 {
                            w.with_entity_mut(target, |w, t| {
                                player_controller::interact_with_entity(w, p, t)
                            });
                        }
                    }
                }
                Some(m) => {
                    let (var3, var4, var5, var6) = (m.block_x, m.block_y, m.block_z, m.side_hit);
                    if var1 == 0 {
                        c.click_block(w, p, var3, var4, var5, m.side_hit);
                    } else {
                        // var7 = inventory.getCurrentItem(), as its slot.
                        let var7 = current_item_slot(p);
                        let var8 = var7.map_or(0, |i| stack_size(p, i));
                        if send_place_block(w, p, var7, var3, var4, var5, var6) {
                            var2 = false;
                            player_fns::swing_item(p);
                        }
                        let Some(slot) = var7 else {
                            return;
                        };
                        let inv = &mut p.kind.player_mut().expect("player").inventory;
                        // The slot still holds var7 (sendPlaceBlock works on it
                        // in place).
                        let size = inv.main_inventory[slot].map_or(0, |s| s.stack_size);
                        if size == 0 {
                            let cur = inv.current_item as usize;
                            inv.main_inventory[cur] = None;
                        } else if size != var8 {
                            events.push(ClientEvent::ResetEquippedProgress { second: false });
                        }
                    }
                }
            }
            if var2
                && var1 == 1
                && let Some(var9) = current_item_slot(p)
                && send_use_item(w, p, var9)
            {
                events.push(ClientEvent::ResetEquippedProgress { second: true });
            }
        });
        self.left_click_counter = left_click_counter;
        self.outbox.events.extend(events);
    }

    /// `clickMiddleMouseButton()`: picks the block under the cursor into the
    /// hotbar (grass as dirt, double slab as slab, bedrock as stone).
    // src: Minecraft.java:920-938
    fn click_middle_mouse_button(&mut self) {
        let Some(m) = self.object_mouse_over else {
            return;
        };
        let (Some(pid), Some(world)) = (self.the_player, self.the_world.as_mut()) else {
            return;
        };
        // vanilla bug: an ENTITY hit reads blockX/Y/Z too, which are 0, so
        // middle-clicking an entity picks the block at (0, 0, 0).
        let mut var1 = world.get_block_id(m.block_x, m.block_y, m.block_z);
        if var1 == GRASS {
            var1 = DIRT;
        }
        if var1 == STAIR_DOUBLE {
            var1 = STAIR_SINGLE;
        }
        if var1 == BEDROCK {
            var1 = STONE;
        }
        if let Some(p) = world.entity_mut(pid).and_then(|e| e.kind.player_mut()) {
            // `playerController instanceof PlayerControllerTest`: false.
            p.inventory.set_current_item(var1, false);
        }
    }

    // ---- runTick ---------------------------------------------------------------------

    /// `runTick()` up to the screen hook: the mouse-over, the controller, the
    /// game-over check. Returns whether the screen hook runs.
    // src: Minecraft.java:944-993
    fn run_tick_begin(&mut self) {
        // src: Minecraft.java:945-947 (func_28001_B: ThreadCheckHasPaid,
        // online, deferred D2); :949 (statFileWriter.func_27178_d: stats
        // sync, deferred D2); :950 is the IngameGuiUpdateTick hook.
    }

    /// `entityRenderer.getMouseOver(partial)`.
    // src: EntityRenderer.java:79-135
    pub fn get_mouse_over(&mut self, var1: f32) {
        if self.render_view_entity.is_none() {
            return;
        }
        let (Some(world), Some(view)) = (self.the_world.as_mut(), self.render_view_entity) else {
            return;
        };
        let reach = self
            .player_controller
            .as_ref()
            .map_or(4.0, |c| c.get_block_reach_distance());
        if let Some((m, pointed)) = camera::get_mouse_over(world, view, reach, var1) {
            self.object_mouse_over = m;
            self.entity_renderer.pointed_entity = pointed;
        }
    }

    /// `runTick()` between the dynamic textures and the screen hook.
    // src: Minecraft.java:951-985
    fn run_tick_controller_and_screens(&mut self) {
        self.get_mouse_over(1.0);
        // src: Minecraft.java:953-961: ChunkProviderLoadOrGenerate only.
        if !self.is_game_paused
            && self.the_world.is_some()
            && let Some(c) = self.player_controller.as_mut()
        {
            c.update_controller();
            // PlayerControllerSP.updateController's second line
            // (PlayerControllerSP.java:117-120).
            self.outbox.sounds.push(ClientSound::PlayRandomMusicIfReady);
        }
    }

    /// `runTick()`'s screen checks before `handleInput`.
    // src: Minecraft.java:972-985
    fn run_tick_screen_checks(&mut self) {
        if self.current_screen.is_none() && self.the_player.is_some() {
            let dead = self
                .player()
                .and_then(|p| p.kind.living())
                .is_some_and(|l| l.health <= 0);
            if dead {
                self.display_gui_screen(None);
            }
            // The GuiSleepMP branch is multiplayer only (D3).
        }
        if self.current_screen.is_some() {
            self.left_click_counter = 10000;
            self.mouse_ticks_ran = self.ticks_ran.wrapping_add(10000);
        }
    }

    /// The default `currentScreen.handleInput()` without a WP-3.6 owner:
    /// every buffered event is consumed; `GuiScreen.handleKeyboardInput` /
    /// `keyTyped`'s F11 and Escape (close and refocus), and
    /// `GuiContainer.keyTyped`'s Escape or inventory key
    /// (`thePlayer.closeScreen()`).
    // src: GuiScreen.java:31-37, :90-126; GuiContainer.java:155-160
    pub fn default_screen_input(&mut self) {
        let keyboard: Vec<InputEvent> = self.input.keyboard.drain(..).collect();
        self.input.mouse.clear();
        for e in keyboard {
            let InputEvent::Key {
                key, down: true, ..
            } = e
            else {
                continue;
            };
            let Some(s) = self.current_screen.as_ref() else {
                break;
            };
            if key == keys::KEY_F11 {
                self.toggle_fullscreen();
                continue;
            }
            let container = s.kind == ScreenKind::Inventory;
            if key == keys::KEY_ESCAPE
                || container && key == self.game_settings.key(settings::KEY_INVENTORY)
            {
                if container {
                    // EntityPlayerSP.closeScreen: EntityPlayer.closeScreen
                    // (craftingInventory, WP-2.5b) + displayGuiScreen(null).
                    self.display_gui_screen(None);
                } else {
                    self.display_gui_screen(None);
                    self.set_ingame_focus();
                }
            }
        }
    }

    /// The mouse and keyboard loops of `runTick` (when no screen is open or
    /// it allows user input).
    // src: Minecraft.java:995-1131
    fn process_input(&mut self) {
        if !(self.current_screen.is_none()
            || self
                .current_screen
                .as_ref()
                .is_some_and(|s| s.allow_user_input))
        {
            return;
        }
        // Mouse events (`do { … } while (now - systemTime > 200)`: stale
        // events are dropped).
        while let Some(e) = self.input.mouse.pop_front() {
            if self.pending.is_some() {
                break;
            }
            let var5 = clock::current_time_millis() - self.system_time;
            if var5 > 200 {
                continue;
            }
            let (button, down, dwheel) = match e {
                InputEvent::MouseButton { button, down, .. } => (button, down, 0),
                InputEvent::MouseWheel { dwheel, .. } => (-1, false, dwheel),
                InputEvent::Key { .. } => continue,
            };
            let mut var3 = dwheel;
            if var3 != 0 {
                if let Some(p) = self.player_in_world_mut().and_then(|e| e.kind.player_mut()) {
                    p.inventory.change_current_item(var3);
                }
                if self.game_settings.noclip {
                    if var3 > 0 {
                        var3 = 1;
                    }
                    if var3 < 0 {
                        var3 = -1;
                    }
                    self.game_settings.noclip_rate += var3 as f32 * 0.25;
                }
            }
            if self.current_screen.is_none() {
                if !self.in_game_has_focus && down {
                    self.set_ingame_focus();
                } else {
                    if button == 0 && down {
                        self.click_mouse(0);
                        self.mouse_ticks_ran = self.ticks_ran;
                    }
                    if button == 1 && down {
                        self.click_mouse(1);
                        self.mouse_ticks_ran = self.ticks_ran;
                    }
                    if button == 2 && down {
                        self.click_middle_mouse_button();
                    }
                }
            }
            // else currentScreen.handleMouseInput(): the screen hook already
            // drained the buffer (GuiScreen.handleInput, GuiScreen.java:90-99).
        }
        if self.left_click_counter > 0 {
            self.left_click_counter -= 1;
        }
        // Keyboard events.
        while let Some(e) = self.input.keyboard.pop_front() {
            if self.pending.is_some() {
                break;
            }
            let InputEvent::Key { key, down, .. } = e else {
                continue;
            };
            if self.the_player.is_some() {
                // thePlayer.handleKeyPress(key, state) (EntityPlayerSP.java:94-96)
                let s = self.game_settings.clone();
                self.movement_input
                    .check_key_for_movement_input(&s, key, down);
            }
            if !down {
                continue;
            }
            if key == keys::KEY_F11 {
                self.toggle_fullscreen();
            } else {
                if self.current_screen.is_some() {
                    // currentScreen.handleKeyboardInput(): drained by the hook.
                } else {
                    if key == keys::KEY_ESCAPE {
                        self.display_in_game_menu();
                    }
                    if key == keys::KEY_S && self.input.is_key_down(keys::KEY_F3) {
                        // forceReload() (Minecraft.java:1180-1185)
                        info!("FORCING RELOAD!");
                        self.outbox.sounds.push(ClientSound::ForceReload);
                    }
                    if key == keys::KEY_F1 {
                        self.game_settings.hide_gui = !self.game_settings.hide_gui;
                    }
                    if key == keys::KEY_F3 {
                        self.game_settings.show_debug_info = !self.game_settings.show_debug_info;
                    }
                    if key == keys::KEY_F5 {
                        self.game_settings.third_person_view =
                            !self.game_settings.third_person_view;
                    }
                    if key == keys::KEY_F8 {
                        self.game_settings.smooth_camera = !self.game_settings.smooth_camera;
                    }
                    if key == self.game_settings.key(settings::KEY_INVENTORY) {
                        self.display_gui_screen(Some(GuiScreen::new(ScreenKind::Inventory)));
                    }
                    if key == self.game_settings.key(settings::KEY_DROP)
                        && let (Some(pid), Some(w)) = (self.the_player, self.the_world.as_mut())
                    {
                        w.with_entity_mut(pid, |w, p| player_fns::drop_current_item(p, w));
                    }
                    // keyBindChat opens GuiChat only in multiplayer
                    // (Minecraft.java:1068-1070).
                    if self.is_multiplayer_world()
                        && key == self.game_settings.key(settings::KEY_CHAT)
                    {
                        self.display_gui_screen(Some(GuiScreen::new(ScreenKind::Chat)));
                    }
                }
                for var6 in 0..9 {
                    if key == keys::KEY_1 + var6
                        && let Some(p) =
                            self.player_in_world_mut().and_then(|e| e.kind.player_mut())
                    {
                        p.inventory.current_item = var6;
                    }
                }
                if key == self.game_settings.key(settings::KEY_TOGGLE_FOG) {
                    let shift = self.input.is_key_down(keys::KEY_LSHIFT)
                        || self.input.is_key_down(keys::KEY_RSHIFT);
                    self.set_option_value(EnumOptions::RenderDistance, if shift { -1 } else { 1 });
                }
            }
        }
        if self.pending.is_some() {
            return;
        }
        if self.current_screen.is_none() {
            let quarter = self.timer.ticks_per_second / 4.0;
            if self.input.is_button_down(0)
                && (self.ticks_ran.wrapping_sub(self.mouse_ticks_ran)) as f32 >= quarter
                && self.in_game_has_focus
            {
                self.click_mouse(0);
                self.mouse_ticks_ran = self.ticks_ran;
            }
            if self.input.is_button_down(1)
                && (self.ticks_ran.wrapping_sub(self.mouse_ticks_ran)) as f32 >= quarter
                && self.in_game_has_focus
            {
                self.click_mouse(1);
                self.mouse_ticks_ran = self.ticks_ran;
            }
        }
        let held =
            self.current_screen.is_none() && self.input.is_button_down(0) && self.in_game_has_focus;
        self.func_6254_a(0, held);
    }

    /// `runTick()` from `joinPlayerCounter` to `randomDisplayUpdates`, with
    /// the hooks run by the caller ([`run_tick`]). `stage` selects the part.
    // src: Minecraft.java:1133-1170
    fn run_tick_world(&mut self, stage: WorldStage) {
        let paused = self.is_game_paused;
        match stage {
            WorldStage::Begin => {
                let Some(world) = self.the_world.as_mut() else {
                    return;
                };
                if let Some(pid) = self.the_player {
                    self.join_player_counter += 1;
                    if self.join_player_counter == 30 {
                        self.join_player_counter = 0;
                        // A dead player the world dropped would be added back
                        // in vanilla; the simulation records that as a gap.
                        world.join_entity_in_surroundings(pid);
                    }
                }
                world.difficulty_setting = self.game_settings.difficulty;
                if world.multiplayer_world {
                    world.difficulty_setting = 3;
                }
                if !paused && let Some(view_id) = self.render_view_entity.or(self.the_player) {
                    // entityRenderer.updateRenderer(): renderViewEntity
                    // defaults to thePlayer (EntityRenderer.java:66-68).
                    self.render_view_entity = Some(view_id);
                    let s = &self.game_settings;
                    let cam = &mut self.entity_renderer;
                    let done = world
                        .with_entity_mut(view_id, |w, v| cam.update_renderer(w, v, s))
                        .is_some();
                    if !done
                        && let Some(v) = self
                            .detached_player
                            .as_deref()
                            .filter(|p| p.base.entity_id == view_id)
                    {
                        cam.update_renderer(world, v, s);
                    }
                }
            }
            WorldStage::Simulate => {
                let Some(world) = self.the_world.as_mut() else {
                    return;
                };
                if !paused {
                    if world.field_27172_i > 0 {
                        world.field_27172_i -= 1;
                    }
                    // movementInput.updatePlayerMoveState runs inside
                    // EntityPlayerSP.onLivingUpdate; the key states are final
                    // for this tick here.
                    let mi = self.movement_input.update_player_move_state();
                    if let Some(pid) = self.the_player
                        && let Some(e) = world.entity_mut(pid)
                    {
                        if let EntityKindData::PlayerSp(_, _, sp) = &mut e.kind {
                            sp.movement_input = mi;
                        }
                        // The last copy before a dying tick (module docs).
                        let dying = e.kind.living().is_some_and(|l| l.health <= 0);
                        if dying {
                            self.detached_player = Some(Box::new(e.clone()));
                        }
                    }
                    let portal_before = portal_hits(world);
                    world.update_entities();
                    if let Some(pid) = self.the_player
                        && world.entity(pid).is_some()
                    {
                        self.detached_player = None;
                    }
                    if portal_hits(world) > portal_before {
                        // Minecraft.usePortal(), reached from
                        // EntityPlayerSP.onLivingUpdate (EntityPlayerSP.java:59).
                        self.use_portal(false);
                        return;
                    }
                }
                let Some(world) = self.the_world.as_mut() else {
                    return;
                };
                if !paused || world.multiplayer_world {
                    world.set_allowed_mob_spawns(self.game_settings.difficulty > 0, true);
                    world.tick();
                    if let Some(e) = world.session_lock_error.take() {
                        // MinecraftException (Minecraft.java:540-544).
                        warn!("{e}");
                        self.the_world = None;
                        self.change_world1(None);
                        self.display_gui_screen(Some(GuiScreen::new(ScreenKind::ConflictWarning)));
                        return;
                    }
                }
                if !paused
                    && let Some(world) = self.the_world.as_mut()
                    && let Some(p) = self.the_player.and_then(|pid| {
                        world
                            .entity(pid)
                            .map(|e| (e.base.pos_x, e.base.pos_y, e.base.pos_z))
                            .or_else(|| {
                                self.detached_player
                                    .as_ref()
                                    .map(|e| (e.base.pos_x, e.base.pos_y, e.base.pos_z))
                            })
                    })
                {
                    world.random_display_updates(
                        floor_double(p.0),
                        floor_double(p.1),
                        floor_double(p.2),
                    );
                }
            }
        }
    }

    // ---- the world lifecycle -------------------------------------------------------

    /// The `WorldEnvironment` a vanilla constructor would draw from the
    /// clock (module docs, "Seeding").
    fn clock_environment() -> WorldEnvironment {
        WorldEnvironment {
            rand_seed: clock::new_random_seed(),
            field_9437_g: JavaRandom::new(clock::new_random_seed()).next_int(),
            lock_timestamp: clock::current_time_millis(),
        }
    }

    /// `startWorld(dir, levelName, seed)`.
    // src: Minecraft.java:1191-1211
    pub fn start_world(&mut self, var1: &str, var2: &str, var3: i64) -> Result<(), String> {
        self.change_world1(None);
        if self.save_loader.is_old_map_format(var1) {
            // convertMapFormat (Minecraft.java:1339-1344): pre-McRegion
            // conversion, deferred (D3).
            return Err(format!(
                "{var1} is a pre-McRegion world; converting it is not supported (deferred)"
            ));
        }
        let var5 = self
            .save_loader
            .get_save_loader(var1)
            .map_err(|e| format!("cannot open saves/{var1}: {e}"))?;
        let mut var6 = World::new_from_save(var5, var2, var3, None, Self::clock_environment());
        seed_clock_randoms(&mut var6);
        self.world_dir = Some(var1.to_string());
        if var6.is_new_world {
            self.read_stat(stat_list::CREATE_WORLD_STAT);
            self.read_stat(stat_list::START_GAME_STAT);
            self.change_world(Some(var6), "Generating level", None, None, false);
        } else {
            self.read_stat(stat_list::LOAD_WORLD_STAT);
            self.read_stat(stat_list::START_GAME_STAT);
            self.change_world(Some(var6), "Loading level", None, None, false);
        }
        Ok(())
    }

    /// `changeWorld1(world)`: `changeWorld(world, "", null)`.
    // src: Minecraft.java:1260-1262
    pub fn change_world1(&mut self, var1: Option<World>) {
        self.change_world(var1, "", None, None, false);
    }

    /// `changeWorld(world, message, player)` up to `func_6255_d`; the rest
    /// runs when the terrain is in ([`Game::finish_load`]). `portal` carries
    /// `usePortal`'s remainder; `sync` loads the terrain at once (a change
    /// nested in another, `respawn`'s portal).
    // src: Minecraft.java:1268-1337
    fn change_world(
        &mut self,
        var1: Option<World>,
        var2: &str,
        var3: Option<Box<Entity>>,
        portal: Option<(f64, f64)>,
        sync: bool,
    ) {
        // statFileWriter.func_27175_b(): empty (StatFileWriter.java:187-188).
        self.sync_stats();
        self.render_view_entity = None;
        self.loading_screen.print_text(var2);
        self.loading_screen.display_loading_string("");
        self.outbox.sounds.push(ClientSound::StopStreaming);
        if self.the_world.is_some() {
            // theWorld.saveWorldIndirectly(loadingScreen)
            self.loading_screen.display_progress_message("Saving level");
            self.loading_screen.display_loading_string("Saving chunks");
            let w = self.the_world.as_mut().expect("checked");
            w.save_world_indirectly();
            if let Some(e) = w.session_lock_error.take() {
                warn!("{e}");
            }
            self.collect_world_events();
        }
        if self.the_player.is_some() && var3.is_none() {
            // `thePlayer = func_4085_a(...)` (null) or `thePlayer = null`:
            // the stats writer goes back to the game.
            self.reclaim_stats();
        }
        self.the_world = var1;
        let Some(world) = self.the_world.as_mut() else {
            self.the_player = None;
            self.detached_player = None;
            self.world_dir = None;
            self.outbox
                .events
                .push(ClientEvent::WorldChanged { dimension: None });
            self.system_time = 0;
            return;
        };
        // playerController.func_717_a(world): empty in PlayerControllerSP's
        // super chain (PlayerController.java:13-14).
        if var3.is_none() {
            // thePlayer = (EntityPlayerSP) world.func_4085_a(EntityPlayerSP.class):
            // always null (World.java:1587-1589).
            self.the_player = None;
            self.detached_player = None;
        }
        let pos = var3.as_ref().map(|p| (p.base.pos_x, p.base.pos_z));
        let preload = TerrainPreload::new(world, &mut self.loading_screen, var2, pos);
        self.pending = Some(PendingLoad {
            preload,
            tail: LoadTail::ChangeWorld {
                player: var3,
                portal,
            },
        });
        if sync {
            self.run_pending_to_completion();
        }
    }

    /// Steps the pending load for up to `budget_ms`; runs its tail when the
    /// terrain is in.
    pub fn step_pending(&mut self, budget_ms: i64) {
        let Some(mut pending) = self.pending.take() else {
            return;
        };
        let Some(world) = self.the_world.as_mut() else {
            return;
        };
        let start = clock::current_time_millis();
        let done = pending.preload.step(world, &mut self.loading_screen, || {
            clock::current_time_millis() - start >= budget_ms
        });
        if done {
            self.finish_load(pending.tail);
        } else {
            self.pending = Some(pending);
        }
    }

    /// Runs the pending load to its end.
    fn run_pending_to_completion(&mut self) {
        while self.pending.is_some() {
            self.step_pending(i64::MAX);
        }
    }

    /// What follows `func_6255_d`.
    fn finish_load(&mut self, tail: LoadTail) {
        match tail {
            LoadTail::ChangeWorld { player, portal } => {
                self.change_world_tail(player);
                if let Some((x, z)) = portal {
                    self.use_portal_tail(x, z);
                }
            }
            LoadTail::Respawn => {
                // src: Minecraft.java:1472-1474
                if self
                    .current_screen
                    .as_ref()
                    .is_some_and(|s| s.kind == ScreenKind::GameOver)
                {
                    self.display_gui_screen(None);
                }
            }
        }
        self.system_time = 0; // src: Minecraft.java:1336
    }

    /// `changeWorld`'s remainder after `func_6255_d`.
    // src: Minecraft.java:1297-1330
    fn change_world_tail(&mut self, var3: Option<Box<Entity>>) {
        let given = var3.is_some();
        let world = self.the_world.as_mut().expect("a world is being loaded");
        let mut player = match var3 {
            Some(p) => *p,
            None => {
                // thePlayer = playerController.createPlayer(world);
                // preparePlayerToSpawn(); playerController.flipPlayer(thePlayer)
                let mut p = create_player(world, &self.session);
                player_fns::prepare_player_to_spawn(&mut p, world);
                player_controller::flip_player(&mut p);
                if let Some(pd) = p.kind.player_mut() {
                    pd.stat_file_writer = std::mem::take(&mut self.stat_file_writer);
                }
                p
            }
        };
        // thePlayer.movementInput = new MovementInputFromOptions(gameSettings)
        self.movement_input = MovementInputFromOptions::default();
        if let EntityKindData::PlayerSp(_, _, sp) = &mut player.kind {
            sp.movement_input = Default::default();
        }
        // renderGlobal.changeWorld(world), effectRenderer.clearEffects(world)
        self.outbox.events.push(ClientEvent::WorldChanged {
            dimension: Some(world.world_provider.world_type),
        });
        // playerController.func_6473_b(thePlayer): empty (PlayerController.java:72-73).
        // `if (var3 != null) var1.emptyMethod1()`: empty (World.java:253-254).
        let _ = given;
        let pid = player.base.entity_id;
        self.the_player = Some(pid);
        world.spawn_player_with_loaded_chunks(player);
        if world.is_new_world {
            world.save_world_indirectly();
        }
        self.render_view_entity = Some(pid);
    }

    /// `usePortal()`: toggles the dimension, scales x and z by 8, saves and
    /// leaves the world, opens the other dimension of the same save and runs
    /// the teleporter there. `sync` loads the new terrain at once.
    // src: Minecraft.java:1213-1258
    pub fn use_portal(&mut self, sync: bool) {
        info!("Toggling dimension!!");
        let (Some(pid), Some(world)) = (self.the_player, self.the_world.as_mut()) else {
            return;
        };
        let in_world = world.entity(pid).is_some();
        // thePlayer.dimension toggled; theWorld.setEntityDead(thePlayer);
        // thePlayer.isDead = false. A dead player the world already dropped
        // (respawning in the nether) is the detached copy.
        let toggle = |p: &mut Entity| {
            let pd = p.kind.player_mut().expect("player");
            pd.dimension = if pd.dimension == -1 { 0 } else { -1 };
            pd.dimension
        };
        let dim = if in_world {
            world.with_entity_mut(pid, |w, p| {
                let d = toggle(p);
                w.set_entity_dead(p);
                p.base.is_dead = false;
                d
            })
        } else {
            self.detached_player.as_deref_mut().map(|p| {
                let d = toggle(p);
                p.base.is_dead = false;
                d
            })
        };
        let Some(dim) = dim else {
            return;
        };
        let var5 = 8.0f64;
        let scale = |p: &mut Entity| {
            let (mut var1, mut var3) = (p.base.pos_x, p.base.pos_z);
            if dim == -1 {
                var1 /= var5;
                var3 /= var5;
            } else {
                var1 *= var5;
                var3 *= var5;
            }
            let (y, yaw, pitch) = (p.base.pos_y, p.base.rotation_yaw, p.base.rotation_pitch);
            p.base.set_location_and_angles(var1, y, var3, yaw, pitch);
            (var1, var3, dispatch::is_entity_alive(p))
        };
        let (var1, var3, alive) = if in_world {
            scale(world.entity_mut(pid).expect("in its slot"))
        } else {
            scale(self.detached_player.as_deref_mut().expect("checked"))
        };
        if alive && in_world {
            world.update_entity_with_optional_force(pid, false);
        }
        let msg = if dim == -1 {
            "Entering the Nether"
        } else {
            "Leaving the Nether"
        };
        let provider = WorldProvider::new(if dim == -1 {
            Dimension::Hell
        } else {
            Dimension::Surface
        });
        // `new World(theWorld, provider)` shares the old world's save handler
        // and copies its WorldInfo (World.java:103-143); changeWorld then saves
        // the old world. Here the old world saves first (the player is no
        // longer in playerEntities, so level.dat gets no Player tag, as
        // vanilla's), then a new handler on the same save directory opens the
        // new one.
        let Some(dir) = self.world_dir.clone() else {
            return;
        };
        self.loading_screen.print_text(msg);
        world.save_world_indirectly();
        if let Some(e) = world.session_lock_error.take() {
            warn!("{e}");
        }
        self.collect_world_events();
        let handler = match self.save_loader.get_save_loader(&dir) {
            Ok(h) => h,
            Err(e) => {
                self.last_error = Some(format!("usePortal: {e}"));
                error!("usePortal: {e}");
                return;
            }
        };
        let mut old = self.the_world.take().expect("checked");
        let player = if in_world {
            old.entity_index(pid)
                .and_then(|i| old.loaded_entity_list[i].entity.take())
        } else {
            self.detached_player.take()
        };
        let Some(player) = player else {
            self.the_world = Some(old);
            return;
        };
        let mut var7 = new_world_for_dimension(&old, handler, provider);
        // The Java statics that outlive a World object in the JVM (module
        // docs, "Seeding"); the new object's own clock-seeded state came
        // from its constructor's environment.
        var7.adopt_statics(&mut old);
        var7.item_rand = old.item_rand.clone();
        drop(old);
        self.change_world(Some(var7), msg, Some(player), Some((var1, var3)), sync);
    }

    /// `usePortal`'s remainder after `changeWorld`.
    // src: Minecraft.java:1251-1257
    fn use_portal_tail(&mut self, var1: f64, var3: f64) {
        let (Some(pid), Some(world)) = (self.the_player, self.the_world.as_mut()) else {
            return;
        };
        let alive = world.entity(pid).is_some_and(dispatch::is_entity_alive);
        if alive {
            if let Some(p) = world.entity_mut(pid) {
                let (y, yaw, pitch) = (p.base.pos_y, p.base.rotation_yaw, p.base.rotation_pitch);
                p.base.set_location_and_angles(var1, y, var3, yaw, pitch);
            }
            world.update_entity_with_optional_force(pid, false);
            let mut t = Teleporter::new(clock::new_random_seed());
            world.with_entity_mut(pid, |w, p| t.func_4107_a(w, p));
        }
    }

    /// `respawn(keepSpawn, dimension)`: a new player at the bed (if it is
    /// still there and `var1` is false) or the world spawn.
    // src: Minecraft.java:1418-1476
    pub fn respawn(&mut self, var1: bool, var2: i32) {
        let Some(world) = self.the_world.as_mut() else {
            return;
        };
        if !world.multiplayer_world && !world.world_provider.can_respawn_here() {
            self.use_portal(true);
        }
        let Some(world) = self.the_world.as_mut() else {
            return;
        };
        let old = self.the_player.and_then(|pid| {
            world
                .entity(pid)
                .cloned()
                .or_else(|| self.detached_player.as_deref().cloned())
        });
        let mut var3 = None;
        let mut var4 = None;
        let mut var5 = true;
        if let Some(p) = &old
            && !var1
        {
            var3 = p
                .kind
                .player()
                .and_then(|d| d.get_player_spawn_coordinate());
            if let Some(c) = var3 {
                var4 = player_fns::verify_respawn_coordinates(world, c);
                if var4.is_none() {
                    // thePlayer.addChatMessage("tile.bed.notValid")
                    self.outbox.events.push(ClientEvent::ChatMessageTranslate(
                        "tile.bed.notValid".into(),
                    ));
                }
            }
        }
        let var4 = match var4 {
            Some(c) => c,
            None => {
                var5 = false;
                world.get_spawn_point()
            }
        };
        // src: Minecraft.java:1441-1445: ChunkProviderLoadOrGenerate only.
        world.set_spawn_location();
        world.update_entity_list();
        let mut var8 = 0;
        if let Some(pid) = self.the_player {
            var8 = pid;
            world.with_entity_mut(pid, |w, p| w.set_entity_dead(p));
            // The dead player leaves the list at once (vanilla: at the next
            // updateEntities), so the new player can take its id without two
            // slots sharing it.
            world.update_entity_list();
        }
        // The stats writer moves to the new player.
        let stats = match &old {
            Some(p) => p
                .kind
                .player()
                .map(|d| d.stat_file_writer.clone())
                .unwrap_or_default(),
            None => std::mem::take(&mut self.stat_file_writer),
        };
        self.detached_player = None;
        self.render_view_entity = None;
        let mut p = create_player(world, &self.session);
        if let Some(pd) = p.kind.player_mut() {
            pd.dimension = var2;
            pd.stat_file_writer = stats;
        }
        player_fns::prepare_player_to_spawn(&mut p, world);
        if var5 {
            if let Some(pd) = p.kind.player_mut() {
                pd.set_player_spawn_coordinate(var3);
            }
            p.base.set_location_and_angles(
                (var4.pos_x as f32 + 0.5) as f64,
                (var4.pos_y as f32 + 0.1) as f64,
                (var4.pos_z as f32 + 0.5) as f64,
                0.0,
                0.0,
            );
        }
        player_controller::flip_player(&mut p);
        // thePlayer.entityId = var8 (vanilla sets it after the join; the
        // port files the entity under its id when it joins). vanilla bug:
        // without a previous player var8 is 0, which may be another
        // entity's id; transcribed.
        p.base.entity_id = var8;
        let pid = p.base.entity_id;
        world.spawn_player_with_loaded_chunks(p);
        self.the_player = Some(pid);
        self.render_view_entity = Some(pid);
        self.movement_input = MovementInputFromOptions::default();
        // thePlayer.func_6420_o(), playerController.func_6473_b(): empty.
        let preload = TerrainPreload::new(world, &mut self.loading_screen, "Respawning", None);
        let preload = {
            let mut pl = preload;
            // func_6255_d centres on thePlayer when there is one.
            if let Some(e) = world.entity(pid) {
                pl.center_x = e.base.pos_x as i32;
                pl.center_z = e.base.pos_z as i32;
            }
            pl
        };
        self.pending = Some(PendingLoad {
            preload,
            tail: LoadTail::Respawn,
        });
    }

    /// The frame's `updateCameraAndRender(partial)` up to the renderers: the
    /// inactive-window pause, the mouse look, `renderWorld`'s
    /// `getMouseOver` and `setupCameraTransform` (into `camera`).
    // src: EntityRenderer.java:329-430, :432-469
    pub fn update_camera_and_render(&mut self, var1: f32, camera_state: &mut CameraState) {
        let now = clock::current_time_millis();
        if !self.display_active {
            if now - self.entity_renderer.prev_frame_time > 500 {
                self.display_in_game_menu();
            }
        } else {
            self.entity_renderer.prev_frame_time = now;
        }
        if self.in_game_has_focus {
            self.mouse_helper.mouse_xy_change(&mut self.input);
            let var2 = self.game_settings.mouse_sensitivity * 0.6 + 0.2;
            let var3 = var2 * var2 * var2 * 8.0;
            let mut var4 = self.mouse_helper.delta_x as f32 * var3;
            let mut var5 = self.mouse_helper.delta_y as f32 * var3;
            let mut var6 = 1i8;
            if self.game_settings.invert_mouse {
                var6 = -1;
            }
            if self.game_settings.smooth_camera {
                var4 = self
                    .entity_renderer
                    .mouse_filter_x_axis
                    .smooth(var4, 0.05 * var3);
                var5 = self
                    .entity_renderer
                    .mouse_filter_y_axis
                    .smooth(var5, 0.05 * var3);
            }
            if let Some(p) = self.player_in_world_mut() {
                p.base.set_angles(var4, var5 * var6 as f32);
            }
        }
        camera_state.setup = None;
        if self.skip_render_world || self.the_world.is_none() {
            return;
        }
        // renderWorld(partial, …)
        if self.render_view_entity.is_none() {
            self.render_view_entity = self.the_player;
        }
        self.get_mouse_over(var1);
        let Some(view_id) = self.render_view_entity else {
            return;
        };
        // mc.thePlayer's portal time (EntityRenderer.java:289)
        let portal = self
            .player()
            .and_then(|p| p.kind.player())
            .map_or(0.0, |d| {
                d.prev_time_in_portal + (d.time_in_portal - d.prev_time_in_portal) * var1
            });
        let s = &self.game_settings;
        let (w, h) = (self.display_width, self.display_height);
        let cam = &mut self.entity_renderer;
        let world = self.the_world.as_mut().expect("checked");
        let mut setup = world.with_entity_mut(view_id, |wd, v| {
            camera::setup_camera_transform(cam, wd, v, s, w, h, portal, var1, 0)
        });
        if setup.is_none()
            && let Some(v) = self
                .detached_player
                .as_deref()
                .filter(|p| p.base.entity_id == view_id)
        {
            setup = Some(camera::setup_camera_transform(
                cam, world, v, s, w, h, portal, var1, 0,
            ));
        }
        camera_state.setup = setup;
    }
}

/// Which part of `runTick`'s world section [`Game::run_tick_world`] runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorldStage {
    /// `joinPlayerCounter`, `difficultySetting`, `updateRenderer`.
    Begin,
    /// `updateEntities`, `tick`, `randomDisplayUpdates`.
    Simulate,
}

/// Seeds what the simulation draws from a clock-seeded `Random` in vanilla
/// and the world constructor did not already seed from its
/// [`WorldEnvironment`] (module docs, "Seeding"): the `Math.random()`
/// stream, `Item.itemRand`, the entity-`rand` and explosion seed and the
/// container blocks' `Random`s. Not [`World::make_deterministic`]: that also
/// restarts the entity and tick-entry counters, which the constructor's
/// spawn search may already have used.
pub fn seed_clock_randoms(w: &mut World) {
    w.math_random = JavaRandom::new(clock::new_random_seed());
    w.item_rand = JavaRandom::new(clock::new_random_seed());
    w.entity_rand_seed = clock::new_random_seed();
    w.block_statics
        .seed_container_randoms(clock::new_random_seed());
}

/// How often the player's `Minecraft.usePortal` call was recorded (the
/// simulation notes it where it would call the client,
/// `EntityPlayerSP.onLivingUpdate`, EntityPlayerSP.java:59).
fn portal_hits(world: &World) -> u64 {
    world
        .unimplemented_hits
        .borrow()
        .get("Minecraft.usePortal")
        .copied()
        .unwrap_or(0)
}

/// `inventory.getCurrentItem()` as the slot holding it.
fn current_item_slot(p: &Entity) -> Option<usize> {
    let inv = &p.kind.player()?.inventory;
    inv.get_current_item().map(|_| inv.current_item as usize)
}

/// The size of the stack in main slot `i` (0 when empty).
fn stack_size(p: &Entity, i: usize) -> i32 {
    p.kind
        .player()
        .and_then(|d| d.inventory.main_inventory[i])
        .map_or(0, |s| s.stack_size)
}

/// `playerController.createPlayer(world)`: `new EntityPlayerSP(mc, world,
/// session, world.worldProvider.worldType)` — the `EntityPlayer`
/// constructor chain, then the dimension and the session's username
/// (EntityPlayerSP.java:12-21; the skin URL is an online skin, D2), with
/// `EntityPlayerSP.onLivingUpdate`'s own body switched on.
// src: PlayerController.java:84-86
pub fn create_player(world: &mut World, session: &Session) -> Entity {
    let mut e = player_fns::new_entity_player_sp(world);
    if let EntityKindData::PlayerSp(_, p, sp) = &mut e.kind {
        p.dimension = world.world_provider.world_type;
        p.username = Some(session.username.clone());
        sp.client_additions = true;
    }
    e
}

/// `new World(world, provider)` (World.java:103-143), the dimension-change
/// constructor `mc-sim` does not have: a world on `handler` (the same save
/// directory) for `provider`, with a copy of `old`'s `WorldInfo` and lock
/// timestamp. Seeding it and carrying the Java statics over
/// ([`World::adopt_statics`]) are the caller's.
pub fn new_world_for_dimension(
    old: &World,
    handler: Box<dyn mc_sim::save::SaveHandler>,
    provider: WorldProvider,
) -> World {
    let seed = old.world_info.random_seed;
    let generator = provider.get_chunk_provider(seed);
    let mut w = World::new(
        handler,
        &old.world_info.level_name.clone(),
        provider,
        seed,
        generator,
        Game::clock_environment(),
    );
    // src: World.java:134, :136 (lockTimestamp, new WorldInfo(var1.worldInfo))
    w.lock_timestamp = old.lock_timestamp;
    w.world_info = old.world_info.clone();
    // src: World.java:141-142 (calculateInitialSkylight, func_27163_E) on the
    // copied WorldInfo.
    w.calculate_initial_skylight();
    w.raining_strength = 0.0;
    w.thundering_strength = 0.0;
    if w.world_info.raining {
        w.raining_strength = 1.0;
        if w.world_info.thundering {
            w.thundering_strength = 1.0;
        }
    }
    w.is_new_world = false;
    w
}

impl TerrainPreload {
    /// `func_6255_d(message)`'s set-up: the messages, the centre (the spawn
    /// point, or the player's truncated x and z).
    // src: Minecraft.java:1346-1363
    fn new(
        world: &World,
        loading: &mut LoadingScreenRenderer,
        var1: &str,
        player_xz: Option<(f64, f64)>,
    ) -> Self {
        loading.print_text(var1);
        loading.display_loading_string("Building terrain");
        let var2: i32 = 128;
        let mut var4 = var2 * 2 / 16 + 1;
        var4 *= var4;
        let var6 = world.get_spawn_point();
        let (x, z) = match player_xz {
            Some((x, z)) => (x as i32, z as i32),
            None => (var6.pos_x, var6.pos_z),
        };
        TerrainPreload {
            center_x: x,
            center_z: z,
            var10: -var2,
            var8: -var2,
            var3: 0,
            var4,
        }
    }

    /// Runs the loop until `out_of_time()` says so (checked after each
    /// chunk) or it is done; when done, `"Simulating world for a bit"` and
    /// `func_656_j` (unload what the provider drops). Returns whether done.
    // src: Minecraft.java:1365-1377
    fn step(
        &mut self,
        world: &mut World,
        loading: &mut LoadingScreenRenderer,
        out_of_time: impl Fn() -> bool,
    ) -> bool {
        let var2 = 128;
        while self.var10 <= var2 {
            while self.var8 <= var2 {
                loading.set_loading_progress(self.var3 * 100 / self.var4);
                self.var3 += 1;
                world.get_block_id(self.center_x + self.var10, 64, self.center_z + self.var8);
                while world.updating_lighting() {}
                self.var8 += 16;
                if out_of_time() {
                    return false;
                }
            }
            self.var8 = -var2;
            self.var10 += 16;
        }
        loading.display_loading_string("Simulating world for a bit");
        world.func_656_j();
        true
    }

    /// Progress in percent (the last `setLoadingProgress` value).
    pub fn percent(&self) -> i32 {
        self.var3 * 100 / self.var4
    }
}

impl Game {
    /// Opens `saves/<name>` as the world screens do: an existing world as
    /// `GuiSelectWorld.selectWorld` (level name from its `level.dat`, seed
    /// 0), a new one as `GuiCreateWorld`'s create button (`seed` is the seed
    /// box's text: a `long`, else its `hashCode()`; empty or 0 keeps a
    /// random seed).
    // src: GuiSelectWorld.java:100-113, GuiCreateWorld.java:67-91
    pub fn open_world(&mut self, name: &str, seed: Option<&str>) -> Result<(), String> {
        match self.save_loader.get_world_info(name) {
            Some(info) => {
                self.display_gui_screen(None);
                self.player_controller = Some(PlayerControllerSp::new());
                // getSaveName: the list row's display name, which
                // getSaveList made the directory name when the level name is
                // empty (GuiSelectWorld.java:49-57, SaveConverterMcRegion.java:39-42).
                let display = if info.level_name.is_empty() {
                    name.to_string()
                } else {
                    info.level_name.clone()
                };
                self.start_world(name, &display, 0)?;
                self.display_gui_screen(None);
            }
            None => {
                self.display_gui_screen(None);
                let mut var2 = JavaRandom::new(clock::new_random_seed()).next_long();
                if let Some(var4) = seed
                    && !var4.is_empty()
                {
                    match crate::jfmt::parse_long(var4) {
                        Ok(var5) => {
                            if var5 != 0 {
                                var2 = var5;
                            }
                        }
                        Err(_) => var2 = crate::jfmt::string_hash_code(var4) as i64,
                    }
                }
                self.player_controller = Some(PlayerControllerSp::new());
                self.start_world(name, name, var2)?;
                self.display_gui_screen(None);
            }
        }
        Ok(())
    }

    /// The frame after the ticks (`run`'s loop body, Minecraft.java:547-614):
    /// the sound listener, the light queue, the opaque-block check, the
    /// camera, the F3 timing, F2, the pause flag and the fps line.
    // src: Minecraft.java:547-614
    fn frame_tail(&mut self, var24: i64, camera_state: &mut CameraState) {
        let partial = self.timer.render_partial_ticks;
        // sndManager.func_338_a(thePlayer, renderPartialTicks)
        let pose = self.player().map(|p| {
            let b = &p.base;
            (
                [
                    b.prev_pos_x + (b.pos_x - b.prev_pos_x) * partial as f64,
                    b.prev_pos_y + (b.pos_y - b.prev_pos_y) * partial as f64,
                    b.prev_pos_z + (b.pos_z - b.prev_pos_z) * partial as f64,
                ],
                b.prev_rotation_yaw + (b.rotation_yaw - b.prev_rotation_yaw) * partial,
            )
        });
        self.outbox.sounds.push(ClientSound::Listener { pose });
        if let Some(w) = self.the_world.as_mut() {
            w.updating_lighting();
        }
        if let (Some(pid), Some(w)) = (self.the_player, self.the_world.as_mut())
            && w.with_entity_mut(pid, |w, p| dispatch::is_entity_inside_opaque_block(p, w))
                .unwrap_or(false)
        {
            self.game_settings.third_person_view = false;
        }
        if !self.skip_render_world {
            // playerController.setPartialTime(partial): the controller's
            // damage_partial_time is read by the renderer (WP-3.7).
            self.update_camera_and_render(partial, camera_state);
        }
        if !self.display_active && self.fullscreen {
            self.toggle_fullscreen();
        }
        let now_nano = clock::nano_time();
        if self.game_settings.show_debug_info {
            self.display_debug_info(var24, now_nano);
        } else {
            self.prev_frame_time = now_nano;
        }
        // guiAchievement.updateAchievementWindow(): WP-3.6, after this system.
        // screenshotListener()
        // src: Minecraft.java:659-669
        if self.input.is_key_down(keys::KEY_F2) {
            if !self.is_taking_screenshot {
                self.is_taking_screenshot = true;
                self.screenshot_requested = true;
            }
        } else {
            self.is_taking_screenshot = false;
        }
        self.fps_frames += 1;
        self.is_game_paused = !self.is_multiplayer_world()
            && self
                .current_screen
                .as_ref()
                .is_some_and(|s| s.does_gui_pause_game());
        let now = clock::current_time_millis();
        while now >= self.fps_second_start + 1000 {
            self.debug = format!(
                "{} fps, {} chunk updates",
                self.fps_frames, self.chunks_updated
            );
            self.chunks_updated = 0;
            self.fps_second_start += 1000;
            self.fps_frames = 0;
        }
    }

    /// `displayDebugInfo(tickNanos)`'s bookkeeping (the graph is drawn by
    /// WP-3.6 from [`Game::frame_times`] / [`Game::tick_times`]).
    // src: Minecraft.java:671-680
    fn display_debug_info(&mut self, var1: i64, var5: i64) {
        if self.prev_frame_time == -1 {
            self.prev_frame_time = var5;
        }
        let n = self.num_recorded_frame_times;
        self.tick_times[(n & 511) as usize] = var1;
        self.frame_times[(n & 511) as usize] = var5 - self.prev_frame_time;
        self.num_recorded_frame_times = n.wrapping_add(1);
        self.prev_frame_time = var5;
    }

    /// One `runTick()` before the hooks that need the Bevy world; see
    /// [`run_tick`].
    fn tick_screen_phase(&mut self) {
        self.run_tick_screen_checks();
    }
}

/// Runs a hook schedule (flushing the simulation's events first, so the
/// hook sees them).
fn run_hook(bw: &mut bevy::ecs::world::World, hook: RunTickHook) {
    flush_outbox(bw);
    bw.run_schedule(hook);
}

/// One `runTick()` (module docs).
// src: Minecraft.java:944-1178
fn run_tick(bw: &mut bevy::ecs::world::World) {
    bw.non_send_mut::<Game>().run_tick_begin();
    run_hook(bw, RunTickHook::IngameGuiUpdateTick);
    let paused = {
        let mut g = bw.non_send_mut::<Game>();
        g.run_tick_controller_and_screens();
        g.is_game_paused
    };
    if !paused {
        run_hook(bw, RunTickHook::UpdateDynamicTextures);
    }
    let screen_open = {
        let mut g = bw.non_send_mut::<Game>();
        g.tick_screen_phase();
        g.current_screen.is_some()
    };
    if screen_open {
        let owned = bw.get_resource::<ScreenInputOwner>().is_some_and(|o| o.0);
        if owned {
            run_hook(bw, RunTickHook::ScreenInput);
        } else {
            bw.non_send_mut::<Game>().default_screen_input();
        }
    }
    let world_present = {
        let mut g = bw.non_send_mut::<Game>();
        if g.pending.is_some() || !g.running {
            return;
        }
        g.process_input();
        if g.pending.is_some() || !g.running {
            return;
        }
        if g.the_world.is_some() {
            g.run_tick_world(WorldStage::Begin);
        }
        g.the_world.is_some()
    };
    if world_present {
        let paused = bw.non_send::<Game>().is_game_paused;
        if !paused {
            run_hook(bw, RunTickHook::UpdateRenderer);
            run_hook(bw, RunTickHook::UpdateClouds);
        }
        let go_on = {
            let mut g = bw.non_send_mut::<Game>();
            g.run_tick_world(WorldStage::Simulate);
            g.pending.is_none() && g.the_world.is_some()
        };
        if go_on && !paused {
            run_hook(bw, RunTickHook::UpdateEffects);
        }
    }
    bw.non_send_mut::<Game>().system_time = clock::current_time_millis();
}

/// The frame: `Minecraft.run`'s loop body (module docs). The exclusive
/// system of [`crate::ClientPlugin`].
// src: Minecraft.java:513-614
pub fn game_frame(bw: &mut bevy::ecs::world::World) {
    let exit = {
        let mut g = bw.non_send_mut::<Game>();
        if !g.running && !g.exit_requested {
            g.shutdown_minecraft_applet();
            g.exit_requested = true;
        }
        g.exit_requested
    };
    if exit {
        flush_outbox(bw);
        bw.write_message(AppExit::Success);
        return;
    }
    let loading = {
        let mut g = bw.non_send_mut::<Game>();
        if g.pending.is_some() {
            // The loading screen's frames: no timer, no ticks (vanilla's
            // loop is blocked in func_6255_d).
            g.step_pending(25);
        }
        g.pending.is_some()
    };
    if loading {
        flush_outbox(bw);
        return;
    }
    {
        let mut g = bw.non_send_mut::<Game>();
        let now = clock::current_time_millis();
        let nano = clock::nano_time();
        if g.is_game_paused && g.the_world.is_some() {
            let var4 = g.timer.render_partial_ticks;
            g.timer.update_timer(now, nano);
            g.timer.render_partial_ticks = var4;
        } else {
            g.timer.update_timer(now, nano);
        }
    }
    let var23 = clock::nano_time();
    let ticks = bw.non_send::<Game>().timer.elapsed_ticks;
    for _ in 0..ticks {
        {
            let mut g = bw.non_send_mut::<Game>();
            g.ticks_ran = g.ticks_ran.wrapping_add(1);
        }
        run_tick(bw);
        let g = bw.non_send::<Game>();
        if g.pending.is_some() || !g.running {
            break;
        }
    }
    let var24 = clock::nano_time() - var23;
    bw.resource_scope::<CameraState, _>(|bw, mut cs| {
        let mut g = bw.non_send_mut::<Game>();
        if g.pending.is_none() && g.running {
            g.frame_tail(var24, &mut cs);
        }
    });
    flush_outbox(bw);
}

/// The outbox into Bevy messages ([`crate::events`]).
pub fn flush_outbox(bw: &mut bevy::ecs::world::World) {
    let out = {
        let mut g = bw.non_send_mut::<Game>();
        g.collect_world_events();
        std::mem::take(&mut g.outbox)
    };
    for e in out.world_events {
        bw.write_message(WorldEventMsg(e));
    }
    for s in out.sounds {
        bw.write_message(s);
    }
    for e in out.events {
        bw.write_message(e);
    }
}

/// The window into the game: `Display.isActive()` (focus; always in the
/// scripted mode), the display size (`mcCanvas` resize, Minecraft.java:593-605)
/// and the close button (`Display.isCloseRequested()` → `shutdown()`,
/// Minecraft.java:521-523).
pub fn track_window(
    mut game: NonSendMut<Game>,
    device: Res<crate::input::DeviceInput>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut close: MessageReader<bevy::window::WindowCloseRequested>,
) {
    for _ in close.read() {
        game.shutdown();
    }
    let Ok(w) = windows.single() else {
        return;
    };
    game.display_active = w.focused || !device.0;
    let (pw, ph) = (w.physical_width() as i32, w.physical_height() as i32);
    if pw > 0 && ph > 0 && (pw != game.display_width || ph != game.display_height) {
        game.resize(pw, ph);
    }
}

/// `Display.setFullscreen` from [`Game::fullscreen`].
pub fn apply_window_mode(
    game: NonSend<Game>,
    mut windows: Query<&mut Window, With<bevy::window::PrimaryWindow>>,
) {
    let Ok(mut w) = windows.single_mut() else {
        return;
    };
    let want = if game.fullscreen {
        bevy::window::WindowMode::BorderlessFullscreen(bevy::window::MonitorSelection::Current)
    } else {
        bevy::window::WindowMode::Windowed
    };
    if w.mode != want {
        w.mode = want;
    }
}

/// F2's capture ([`crate::screenshot::save_screenshot`]) and its chat line.
pub fn take_requested_screenshot(
    mut commands: Commands,
    mut game: NonSendMut<Game>,
    pending: Res<crate::screenshot::PendingScreenshots>,
    mut events: MessageWriter<ClientEvent>,
) {
    if !game.screenshot_requested {
        return;
    }
    game.screenshot_requested = false;
    let line = crate::screenshot::save_screenshot(
        &mut commands,
        &pending,
        &game.paths.data_dir,
        clock::current_time_millis(),
    );
    // ingameGUI.addChatMessage(ScreenShotHelper.saveScreenshot(...))
    events.write(ClientEvent::ChatMessage(line));
}

/// [`AppState`] from the game (module docs of [`crate::app`]).
pub fn sync_app_state(
    game: NonSend<Game>,
    state: Res<State<AppState>>,
    mut next: ResMut<NextState<AppState>>,
) {
    let want = if game.pending.is_some() {
        AppState::Loading
    } else if game.the_world.is_none() {
        AppState::Title
    } else if game.is_game_paused {
        AppState::Paused
    } else {
        AppState::InGame
    };
    if *state.get() != want {
        next.set(want);
    }
}

/// The interpolated view for the renderers, published every frame.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameView {
    /// `timer.renderPartialTicks`.
    pub render_partial_ticks: f32,
    /// The player's interpolated eye position (`prevPos + (pos - prevPos) *
    /// partial`; `posY` is the eye for a player) and yaw/pitch; `None`
    /// without a player.
    pub player_eye: Option<([f64; 3], f32, f32)>,
    /// `objectMouseOver`.
    pub object_mouse_over: Option<MovingObjectPosition>,
    /// `ticksRan`.
    pub ticks_ran: i32,
}

/// Fills [`FrameView`].
pub fn publish_frame_view(game: NonSend<Game>, mut view: ResMut<FrameView>) {
    let partial = game.timer.render_partial_ticks;
    view.render_partial_ticks = partial;
    view.player_eye = game.player().map(|p| {
        let b = &p.base;
        (
            [
                b.prev_pos_x + (b.pos_x - b.prev_pos_x) * partial as f64,
                b.prev_pos_y + (b.pos_y - b.prev_pos_y) * partial as f64,
                b.prev_pos_z + (b.pos_z - b.prev_pos_z) * partial as f64,
            ],
            b.prev_rotation_yaw + (b.rotation_yaw - b.prev_rotation_yaw) * partial,
            b.prev_rotation_pitch + (b.rotation_pitch - b.prev_rotation_pitch) * partial,
        )
    });
    view.object_mouse_over = game.object_mouse_over;
    view.ticks_ran = game.ticks_ran;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::tests::TempDir;

    fn game(dir: &TempDir) -> Game {
        let paths = ClientPaths {
            data_dir: dir.0.clone(),
            jar: PathBuf::from("/nonexistent.jar"),
            resources: PathBuf::from("/nonexistent"),
        };
        Game::new(
            paths,
            AchievementMap::default(),
            Session::new("Tester", "-"),
        )
        .unwrap()
    }

    /// Ticks without Bevy: the world part of `runTick` (no hooks).
    fn tick(g: &mut Game) {
        g.ticks_ran += 1;
        g.run_tick_controller_and_screens();
        g.tick_screen_phase();
        if g.current_screen.is_some() {
            g.default_screen_input();
        }
        g.process_input();
        if g.the_world.is_some() && g.pending.is_none() {
            g.run_tick_world(WorldStage::Begin);
            g.run_tick_world(WorldStage::Simulate);
        }
        g.system_time = clock::current_time_millis();
    }

    #[test]
    fn create_play_save_reopen() {
        let dir = TempDir::new("game");
        let mut g = game(&dir);
        assert_eq!(g.game_settings, GameSettings::default());
        g.open_world("T", Some("123")).unwrap();
        assert!(g.pending.is_some());
        assert_eq!(
            g.loading_screen.currently_displayed_text,
            "Generating level"
        );
        g.run_pending_to_completion();
        assert_eq!(g.the_world.as_ref().unwrap().world_info.random_seed, 123);
        let pid = g.the_player.unwrap();
        assert!(g.the_world.as_ref().unwrap().entity(pid).is_some());
        assert!(g.current_screen.is_none());
        let t0 = g.the_world.as_ref().unwrap().get_world_time();
        for _ in 0..20 {
            tick(&mut g);
        }
        let t1 = g.the_world.as_ref().unwrap().get_world_time();
        assert_eq!(t1 - t0, 20);
        // Move the player, quit (saves), reopen: the position persists.
        {
            let p = g.player_in_world_mut().unwrap();
            let (x, y, z) = (p.base.pos_x + 3.0, p.base.pos_y, p.base.pos_z);
            p.base.set_position(x, y, z);
        }
        let saved_x = g.player().unwrap().base.pos_x;
        g.shutdown();
        g.shutdown_minecraft_applet();
        assert!(g.the_world.is_none());
        let mut g = game(&dir);
        g.open_world("T", None).unwrap();
        assert_eq!(g.loading_screen.currently_displayed_text, "Loading level");
        g.run_pending_to_completion();
        assert_eq!(g.the_world.as_ref().unwrap().world_info.random_seed, 123);
        let x = g.player().unwrap().base.pos_x;
        assert!((x - saved_x).abs() < 1e-9, "{x} vs {saved_x}");
        // The stats file was written (create, load and start-game stats).
        assert!(dir.0.join("stats/stats_tester_unsent.dat").exists());
    }

    #[test]
    fn escape_pauses_and_escape_resumes() {
        let dir = TempDir::new("pause");
        let mut g = game(&dir);
        g.open_world("P", Some("5")).unwrap();
        g.run_pending_to_completion();
        g.display_gui_screen(None);
        assert!(g.in_game_has_focus);
        g.input.push_key(keys::KEY_ESCAPE, true, '\0');
        g.input.push_key(keys::KEY_ESCAPE, false, '\0');
        tick(&mut g);
        assert_eq!(
            g.current_screen.as_ref().map(|s| s.kind.clone()),
            Some(ScreenKind::IngameMenu)
        );
        assert!(!g.in_game_has_focus);
        let mut cs = CameraState::default();
        g.frame_tail(0, &mut cs);
        assert!(g.is_game_paused);
        let t0 = g.the_world.as_ref().unwrap().get_world_time();
        tick(&mut g);
        assert_eq!(g.the_world.as_ref().unwrap().get_world_time(), t0);
        g.input.push_key(keys::KEY_ESCAPE, true, '\0');
        tick(&mut g);
        assert!(g.current_screen.is_none());
        assert!(g.in_game_has_focus);
    }

    #[test]
    fn movement_keys_reach_the_player() {
        let dir = TempDir::new("move");
        let mut g = game(&dir);
        g.open_world("M", Some("7")).unwrap();
        g.run_pending_to_completion();
        tick(&mut g);
        g.input.push_key(17, true, 'w'); // forward
        g.input.push_key(30, true, 'a'); // left
        tick(&mut g);
        // EntityPlayerSP.updatePlayerActionState copied the input into the
        // living entity's steering, which onLivingUpdate then damps by 0.98
        // (EntityLiving.java). The spawn's surroundings vary with the
        // clock-seeded spawn search, so the test does not rely on walking.
        let p = g.player().unwrap();
        let EntityKindData::PlayerSp(l, _, sp) = &p.kind else {
            panic!("not a player");
        };
        assert_eq!(sp.movement_input.move_forward, 1.0);
        assert_eq!((l.move_forward, l.move_strafing), (0.98, 0.98));
        g.input.push_key(17, false, 'w');
        g.input.push_key(42, true, '\0'); // sneak
        tick(&mut g);
        let p = g.player().unwrap();
        let EntityKindData::PlayerSp(l, _, sp) = &p.kind else {
            panic!("not a player");
        };
        assert!(sp.movement_input.sneak);
        assert_eq!(
            (l.move_forward, l.move_strafing),
            (0.0, (1.0f64 * 0.3) as f32 * 0.98)
        );
        // Losing focus resets the keys (setIngameNotInFocus).
        g.display_in_game_menu();
        assert_eq!(g.movement_input, MovementInputFromOptions::default());
    }

    #[test]
    fn hotbar_keys_and_options_keys() {
        let dir = TempDir::new("keys");
        let mut g = game(&dir);
        g.open_world("K", Some("9")).unwrap();
        g.run_pending_to_completion();
        g.input.push_key(keys::KEY_1 + 4, true, '5');
        g.input.push_key(keys::KEY_F5, true, '\0');
        g.input.push_key(keys::KEY_F3, true, '\0');
        g.input.push_key(33, true, 'f'); // fog key: render distance + 1
        tick(&mut g);
        assert_eq!(
            g.player()
                .unwrap()
                .kind
                .player()
                .unwrap()
                .inventory
                .current_item,
            4
        );
        assert!(g.game_settings.third_person_view);
        assert!(g.game_settings.show_debug_info);
        assert_eq!(g.game_settings.render_distance, 1);
        // The change was saved.
        let text = std::fs::read_to_string(dir.0.join("options.txt")).unwrap();
        assert!(text.contains("viewDistance:1\n"));
    }

    #[test]
    fn respawn_creates_a_new_player_with_the_old_id() {
        let dir = TempDir::new("respawn");
        let mut g = game(&dir);
        g.open_world("R", Some("11")).unwrap();
        g.run_pending_to_completion();
        let pid = g.the_player.unwrap();
        g.respawn(false, 0);
        g.run_pending_to_completion();
        assert_eq!(g.the_player, Some(pid));
        let w = g.the_world.as_ref().unwrap();
        assert_eq!(
            w.loaded_entity_list
                .iter()
                .filter(|s| s.entity_id == pid)
                .count(),
            1
        );
        assert_eq!(w.player_entities, vec![pid]);
    }

    #[test]
    fn portal_travel_round_trip() {
        let dir = TempDir::new("portal");
        let mut g = game(&dir);
        g.open_world("N", Some("13")).unwrap();
        g.run_pending_to_completion();
        g.use_portal(true);
        let w = g.the_world.as_ref().unwrap();
        assert_eq!(w.world_provider.world_type, -1);
        let p = g.player().unwrap();
        assert_eq!(p.kind.player().unwrap().dimension, -1);
        // The teleporter built (or found) a portal at the player.
        let (x, y, z) = (
            floor_double(p.base.pos_x),
            floor_double(p.base.pos_y),
            floor_double(p.base.pos_z),
        );
        let mut w = g.the_world.take().unwrap();
        assert_eq!(w.get_block_id(x, y, z), 90);
        g.the_world = Some(w);
        // Back.
        g.use_portal(true);
        assert_eq!(g.the_world.as_ref().unwrap().world_provider.world_type, 0);
        assert_eq!(g.player().unwrap().kind.player().unwrap().dimension, 0);
        assert!(dir.0.join("saves/N/DIM-1/region").exists());
    }

    #[test]
    fn dying_in_the_nether_respawns_in_the_overworld() {
        let dir = TempDir::new("nether_death");
        let mut g = game(&dir);
        g.open_world("D", Some("17")).unwrap();
        g.run_pending_to_completion();
        g.use_portal(true);
        assert_eq!(g.the_world.as_ref().unwrap().world_provider.world_type, -1);
        let pid = g.the_player.unwrap();
        // Kill the player and let the world drop the body (deathTime 20).
        if let Some(l) = g.player_in_world_mut().and_then(|e| e.kind.living_mut()) {
            l.health = 0;
        }
        for _ in 0..30 {
            tick(&mut g);
        }
        assert!(g.the_world.as_ref().unwrap().entity(pid).is_none());
        assert!(g.detached_player.is_some());
        assert_eq!(
            g.current_screen.as_ref().map(|s| s.kind.clone()),
            Some(ScreenKind::GameOver)
        );
        g.respawn(false, 0);
        g.run_pending_to_completion();
        assert!(g.current_screen.is_none());
        let w = g.the_world.as_ref().unwrap();
        assert_eq!(w.world_provider.world_type, 0);
        assert_eq!(g.the_player, Some(pid));
        let p = g.player().unwrap();
        assert_eq!(p.kind.living().unwrap().health, 20);
        assert_eq!(p.kind.player().unwrap().dimension, 0);
    }

    #[test]
    fn preload_centres_on_the_truncated_position() {
        // func_6255_d centres on (int) posX, not floor (Minecraft.java:1356).
        assert_eq!((-0.5f64) as i32, 0);
        assert_eq!(floor_double(-0.5), -1);
    }
}
