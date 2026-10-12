// disable console on windows for release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! The binary: the command line, the window, the icon, and
//! [`mc_client::ClientPlugin`] (vanilla's `Minecraft.main` /
//! `startMainThread`, Minecraft.java:1478-1529: an 854×480 window).
//!
//! ```text
//! mc_b173 [--data-dir DIR] [--prism-instance DIR] [--username NAME]
//!         [--world NAME [--seed SEED] | --import DIR] [--script FILE.json]
//!         [--vsync on|off]
//! ```
//!
//! See `crates/mc-client/README.md`.

use std::io::Cursor;
use std::path::PathBuf;

use bevy::DefaultPlugins;
use bevy::asset::AssetMetaCheck;
use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::window::{PresentMode, PrimaryWindow};
use bevy::winit::WINIT_WINDOWS;
use mc_client::paths::ClientPaths;
use mc_client::{ClientConfig, ClientPlugin, StartAction};
use winit::window::Icon;

/// The parsed command line.
#[derive(Debug, Default)]
struct Args {
    data_dir: Option<PathBuf>,
    prism_instance: Option<PathBuf>,
    username: Option<String>,
    world: Option<String>,
    seed: Option<String>,
    import: Option<PathBuf>,
    script: Option<PathBuf>,
    /// `--vsync on`: wait for the display's refresh (WP-4.3's flag; since
    /// WP-4.9 the default is off, as vanilla's, PARITY_CHECKLIST P6).
    vsync: bool,
}

const USAGE: &str =
    "usage: mc_b173 [--data-dir DIR] [--prism-instance DIR] [--username NAME]
               [--world NAME [--seed SEED] | --import DIR] [--script FILE.json]
               [--vsync on|off]

  --data-dir DIR    options, saves, screenshots, stats (default: the platform data dir + mc_b173)
  --prism-instance DIR
                    your Prism Launcher b1.7.3 instance, .../PrismLauncher/instances/b1.7.3
                    (the client jar and sounds; default: found in the standard Prism places)
  --username NAME   the player name (default: Player<millis % 1000>)
  --world NAME      open saves/NAME, creating it when missing (skips the title screen)
  --seed SEED       the new world's seed, as the Create World screen reads it
  --import DIR      import a vanilla save folder into saves/, then open it
  --script FILE     scripted-input dev mode (crates/mc-client/README.md)
  --vsync on|off    wait for the display's refresh (default off: as vanilla, the frame rate is
                    uncapped at Performance Balanced/Max and 40 fps at Power saver)";

fn parse_args() -> Result<Args, String> {
    let mut a = Args::default();
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut value = |name: &str| it.next().ok_or_else(|| format!("{name} needs a value"));
        match arg.as_str() {
            "--data-dir" => a.data_dir = Some(value("--data-dir")?.into()),
            "--prism-instance" => a.prism_instance = Some(value("--prism-instance")?.into()),
            "--username" => a.username = Some(value("--username")?),
            "--world" => a.world = Some(value("--world")?),
            "--seed" => a.seed = Some(value("--seed")?),
            "--import" => a.import = Some(value("--import")?.into()),
            "--script" => a.script = Some(value("--script")?.into()),
            "--vsync" => {
                a.vsync = match value("--vsync")?.as_str() {
                    "on" => true,
                    "off" => false,
                    v => return Err(format!("--vsync takes on or off, not {v}")),
                }
            }
            "-h" | "--help" => return Err(String::new()),
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if a.world.is_some() && a.import.is_some() {
        return Err("--world and --import exclude each other".into());
    }
    if a.seed.is_some() && a.world.is_none() {
        return Err("--seed needs --world".into());
    }
    Ok(a)
}

/// Exits with a message (start-up errors: D8's missing Prism instance or
/// jar, an import refusal).
fn fail(msg: &str) -> ! {
    if !msg.is_empty() {
        eprintln!("mc_b173: {msg}");
    }
    eprintln!("{USAGE}");
    std::process::exit(if msg.is_empty() { 0 } else { 1 });
}

fn main() {
    mc_client::perf::mark_process_start();
    let args = parse_args().unwrap_or_else(|e| fail(&e));
    // The jar and resources/ come from the Prism b1.7.3 instance: the flag,
    // else the first one in the standard Prism places (mc_client::paths
    // docs). The lines say which were picked (CI greps them).
    let data_dir = args
        .data_dir
        .unwrap_or_else(mc_client::paths::default_data_dir);
    let instance = args
        .prism_instance
        .or_else(mc_client::paths::find_prism_instance)
        .unwrap_or_else(|| {
            fail(
                &mc_client::paths::PrismError::NotFound {
                    tried: mc_client::paths::prism_instance_candidates(),
                }
                .to_string(),
            )
        });
    eprintln!("mc_b173: prism instance: {}", instance.display());
    let paths = ClientPaths::from_prism_instance(data_dir, &instance)
        .unwrap_or_else(|e| fail(&e.to_string()));
    eprintln!("mc_b173: minecraft.jar: {}", paths.jar.display());
    eprintln!("mc_b173: resources: {}", paths.resources.display());
    let script = args.script.map(|p| {
        mc_client::script::Script::load(&p).unwrap_or_else(|e| {
            eprintln!("mc_b173: {e}");
            std::process::exit(1)
        })
    });
    let start = if let Some(src) = args.import {
        // D5: the import is validated before anything is copied; a refusal
        // names the first offending id (the dialog is WP-3.6's).
        match mc_client::import::import_world(&paths.data_dir, &src) {
            Ok(name) => {
                eprintln!("mc_b173: imported {} as saves/{name}", src.display());
                StartAction::World { name, seed: None }
            }
            Err(e) => {
                // The real file, not the staging copy (Gate 3 O4).
                eprintln!("mc_b173: {}", e.naming_folder(&src));
                std::process::exit(1);
            }
        }
    } else if let Some(name) = args.world {
        StartAction::World {
            name,
            seed: args.seed,
        }
    } else {
        StartAction::Title
    };
    let config = ClientConfig::open(paths, args.username, start, script).unwrap_or_else(|e| {
        eprintln!("mc_b173: {e}");
        std::process::exit(1)
    });

    App::new()
        .insert_resource(ClearColor(Color::linear_rgb(0.4, 0.4, 0.4)))
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Minecraft Beta 1.7.3 (Rust)".to_string(),
                        // src: Minecraft.java:1488 (the canvas's 854×480)
                        resolution: (854u32, 480u32).into(),
                        // b1.7.3 never enables LWJGL's vsync: Balanced and
                        // Max run uncapped and Power saver sleeps to 40 fps
                        // (EntityRenderer.java:364-416,
                        // terrain::power_saver_sleep). `--vsync on` (a
                        // project addition, WP-4.3) waits for the display.
                        present_mode: if args.vsync {
                            PresentMode::AutoVsync
                        } else {
                            PresentMode::AutoNoVsync
                        },
                        ..default()
                    }),
                    // The close button is `Display.isCloseRequested()`: the game
                    // saves and quits itself (Minecraft.java:521-523).
                    close_when_requested: false,
                    ..default()
                })
                .set(AssetPlugin {
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                })
                // GL compiles what a frame draws inside that frame: no
                // draw is skipped while its pipeline compiles on another
                // thread (the world's first frames after a load,
                // PARITY_CHECKLIST P8), and no compilation task is left
                // running when the process exits (PERF.md §7's segfault).
                .set(RenderPlugin {
                    synchronous_pipeline_compilation: true,
                    ..default()
                }),
        )
        .add_plugins(ClientPlugin { config })
        .add_systems(Startup, set_window_icon)
        .run();
}

// Sets the icon on windows and X11
fn set_window_icon(
    primary_window: Single<Entity, With<PrimaryWindow>>,
    _non_send_marker: NonSendMarker,
) -> Result {
    WINIT_WINDOWS.with_borrow(|windows| {
        let Some(primary) = windows.get_window(*primary_window) else {
            return Err(BevyError::from("No primary window!"));
        };
        let icon_buf = Cursor::new(include_bytes!(
            "../../../build/macos/AppIcon.iconset/icon_256x256.png"
        ));
        if let Ok(image) = image::load(icon_buf, image::ImageFormat::Png) {
            let image = image.into_rgba8();
            let (width, height) = image.dimensions();
            let rgba = image.into_raw();
            let icon = Icon::from_rgba(rgba, width, height).unwrap();
            primary.set_window_icon(Some(icon));
        };

        Ok(())
    })
}
