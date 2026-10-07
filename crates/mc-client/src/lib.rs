//! The Bevy shell around `mc-sim` (PLAN_v4 D1, §3.5).
//!
//! Owns the window, input, rendering, UI, audio and asset loading, and drives
//! the simulation from one system. Reads textures and `lang/` from the user's
//! own `minecraft.jar` and sounds from a `resources/` directory at runtime
//! (D8). Module map and the scripted-input format: `crates/mc-client/README.md`.
//!
//! WP-3.1 (this shell): [`game`] (`Minecraft.java`'s loop and lifecycle),
//! [`timer`], [`settings`], [`input`] / [`keys`], [`camera`] (the camera half
//! of `EntityRenderer`), [`storage`] (native `Storage`), [`assets`] / [`lang`]
//! (jar and `resources/`), [`screenshot`], [`script`] (the scripted-input dev
//! mode), [`import`], [`loading`], [`debug`] (F3 text), [`events`] (messages
//! for the other client parts), [`app`] (states, the screen slot).
//!
//! WP-3.2: [`textures`] (`RenderEngine`, the `TextureFX` family, the
//! colorizers).
//!
//! WP-3.4: [`sky`] (`RenderGlobal.renderSky`; [`sky::SkyPlugin`] wires the
//! rest), [`clouds`], [`weather`] (`renderRainSnow`), [`fog`]
//! (`updateFogColor`, `setupFog`, [`fog::FogState`]), [`selection`] (the
//! block outline), [`gl_material`] (the GL-state material they draw with).
//!
//! WP-3.3a: [`terrain`] (`RenderBlocks`, `WorldRenderer`, the terrain half
//! of `RenderGlobal`, the crack overlay).
//! WP-3.5a: [`model`] (the `Model*` family, `ModelRenderer` as meshes and
//! transforms, the lit model material).
//! WP-3.6a: [`gui`] (`Gui`, `GuiScreen`, the widgets, `FontRenderer`, the
//! screen objects bound to the shell's slot, the 2D batcher and its
//! cameras).
//! WP-3.6c: [`gui::menu`] (the title, pause, options, world select /
//! create / rename / import, achievements, statistics and texture-pack
//! screens), [`gui::loading`] (the loading screen's drawing),
//! [`gui::screens_menu`] (their registration).
//! WP-3.7a: [`particles`] (`EffectRenderer`, the `EntityFX` family,
//! `RenderGlobal.spawnParticle`, `addRainParticles`).
//! WP-3.7b: [`item_renderer`] (`ItemRenderer`: the first-person hand and
//! item, the screen overlays, `renderItem`; `EntityRenderer.renderHand`).
//! WP-3.5b: [`render`] (`RenderManager`, every `Render*`, the tile-entity
//! special renderers, `RenderGlobal.renderEntities`).

pub mod app;
pub mod assets;
pub mod camera;
pub mod clock;
pub mod clouds;
pub mod debug;
pub mod events;
pub mod fog;
pub mod game;
pub mod gl_material;
pub mod gui;
pub mod import;
pub mod input;
pub mod item_renderer;
pub mod jfmt;
pub mod keys;
pub mod lang;
pub mod loading;
pub mod model;
pub mod particles;
pub mod paths;
pub mod render;
pub mod screenshot;
pub mod script;
pub mod selection;
pub mod session;
pub mod settings;
pub mod sky;
pub mod sound;
pub mod storage;
pub mod terrain;
pub mod textures;
pub mod timer;
pub mod weather;

use bevy::prelude::*;
use mc_sim::stats::AchievementMap;

pub use app::AppState;
pub use game::{Game, RunTickHook, StartAction};

/// Everything the client needs to start (built by `mc-app` from the command
/// line).
#[derive(Clone, Debug)]
pub struct ClientConfig {
    /// The data directory, jar and `resources/`.
    pub paths: paths::ClientPaths,
    /// The opened jar (D8: opening it is the start-up check).
    pub jar: assets::JarAssets,
    /// The `resources/` directory.
    pub resources: assets::ResourcesDir,
    /// The player name; `None` for `Minecraft.main`'s `"Player" + millis %
    /// 1000`.
    pub username: Option<String>,
    /// Title screen, or straight into a world.
    pub start: StartAction,
    /// The scripted-input dev mode's script.
    pub script: Option<script::Script>,
}

impl ClientConfig {
    /// Opens the jar and checks `resources/` (D8: a missing one is an error
    /// naming the path).
    pub fn open(
        paths: paths::ClientPaths,
        username: Option<String>,
        start: StartAction,
        script: Option<script::Script>,
    ) -> Result<Self, assets::AssetError> {
        let jar = assets::JarAssets::open(&paths.jar)?;
        let resources = assets::ResourcesDir::open(&paths.resources)?;
        Ok(ClientConfig {
            paths,
            jar,
            resources,
            username,
            start,
            script,
        })
    }
}

/// The client plugin.
pub struct ClientPlugin {
    /// Its configuration.
    pub config: ClientConfig,
}

/// The shell's frame, in order: input in, the script, window tracking, the
/// game frame, then the outputs (cursor, window mode, camera, screenshots,
/// state, published resources). Later WPs order their systems after it.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GameFrameSet;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        let c = &self.config;
        let achievement_map = AchievementMap::parse(&String::from_utf8_lossy(
            c.jar.get(assets::ACHIEVEMENT_MAP_TXT).unwrap_or_default(),
        ));
        let session = match &c.username {
            Some(u) => session::Session::new(u, "-"),
            None => session::Session::offline(clock::current_time_millis()),
        };
        let game =
            Game::new(c.paths.clone(), achievement_map, session).unwrap_or_else(|e| panic!("{e}"));
        let lang = assets::Lang(c.jar.string_translate());
        app.init_state::<AppState>()
            .add_message::<events::WorldEventMsg>()
            .add_message::<events::ClientSound>()
            .add_message::<events::ClientEvent>()
            .insert_resource(c.jar.clone())
            .insert_resource(c.resources.clone())
            .insert_resource(lang)
            .insert_resource(StartupAction(Some(c.start.clone())))
            .init_resource::<input::DeviceInput>()
            .init_resource::<camera::CameraState>()
            .init_resource::<screenshot::PendingScreenshots>()
            .init_resource::<game::FrameView>()
            .init_resource::<game::ScreenInputOwner>()
            .init_resource::<debug::DebugSources>()
            .init_resource::<debug::DebugText>()
            .insert_non_send(game);
        // Every camera composites in sRGB, as the world camera
        // (camera::spawn_camera): GL's byte-space blending (WP-3.7b).
        app.register_required_components::<Camera, bevy::camera::CompositingSpace>();
        for hook in RunTickHook::ALL {
            app.init_schedule(hook);
        }
        if let Some(s) = &c.script {
            script::install(app, s.clone());
        }
        // WP-3.2: RenderEngine (textures, the FX, the colour maps).
        app.add_plugins(textures::TexturesPlugin { jar: c.jar.clone() });
        app.add_systems(Startup, (camera::spawn_camera, run_startup_action))
            .add_systems(
                Update,
                (
                    input::collect_device_input,
                    script::run_script,
                    game::track_window,
                    game::game_frame,
                    input::apply_cursor_grab,
                    game::apply_window_mode,
                    camera::apply_camera,
                    game::take_requested_screenshot,
                    game::sync_app_state,
                    game::publish_frame_view,
                    debug::publish_debug_text,
                )
                    .chain()
                    .in_set(GameFrameSet),
            );
        // WP-3.4: sky, clouds, weather, fog, selection box.
        app.add_plugins(sky::SkyPlugin);
        // WP-3.8: SoundManager on bevy_kira_audio.
        app.add_plugins(sound::SoundPlugin);
        // WP-3.3a: the terrain (RenderBlocks, WorldRenderer, RenderGlobal).
        app.add_plugins(terrain::TerrainPlugin);
        // WP-3.5a: entity models (material, mesh cache, debug_model).
        app.add_plugins(model::ModelPlugin);
        // WP-3.6a: the GUI (screens, widgets, font, the 2D batcher).
        app.add_plugins(gui::GuiPlugin);
        // WP-3.6c: the menus, options, achievements, statistics, the world
        // screens and import, the loading screen.
        app.add_plugins(gui::screens_menu::MenuPlugin);
        // WP-3.7a: EffectRenderer and the particles.
        app.add_plugins(particles::ParticlesPlugin);
        // WP-3.7b: ItemRenderer (the hand, held items, the overlays).
        app.add_plugins(item_renderer::ItemRendererPlugin);
        // WP-3.5b: RenderManager, the entity and tile-entity renderers.
        app.add_plugins(render::RenderPlugin);
    }
}

/// What to do once at start-up ([`StartAction`]).
#[derive(Resource, Clone, Debug)]
pub struct StartupAction(pub Option<StartAction>);

/// `startGame`'s last step: the main menu, or the world the command line
/// names (as the world screens would open it). A failure is logged and
/// leaves the title screen.
// src: Minecraft.java:295-300
fn run_startup_action(mut game: NonSendMut<Game>, mut action: ResMut<StartupAction>) {
    let Some(a) = action.0.take() else {
        return;
    };
    let r = match a {
        StartAction::Title => {
            game.display_gui_screen(Some(app::GuiScreen::new(app::ScreenKind::MainMenu)));
            Ok(())
        }
        StartAction::World { name, seed } => game.open_world(&name, seed.as_deref()),
        StartAction::Import { path } => {
            match import::import_world(&game.paths.data_dir.clone(), &path) {
                Ok(name) => {
                    info!("imported {} as saves/{name}", path.display());
                    game.open_world(&name, None)
                }
                Err(e) => Err(e.to_string()),
            }
        }
    };
    if let Err(e) = r {
        error!("{e}");
        game.last_error = Some(e);
        if game.the_world.is_none() && game.current_screen.is_none() {
            game.display_gui_screen(Some(app::GuiScreen::new(app::ScreenKind::MainMenu)));
        }
    }
}
