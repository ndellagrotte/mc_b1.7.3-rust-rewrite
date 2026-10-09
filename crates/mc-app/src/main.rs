// disable console on windows for release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! The binary: the command line, the window, the icon, and
//! [`mc_client::ClientPlugin`] (vanilla's `Minecraft.main` /
//! `startMainThread`, Minecraft.java:1478-1529: an 854×480 window).
//!
//! ```text
//! mc_b173 [--data-dir DIR] [--jar FILE] [--resources DIR] [--username NAME]
//!         [--world NAME [--seed SEED] | --import DIR] [--script FILE.json]
//! ```
//!
//! See `crates/mc-client/README.md`.

use std::io::Cursor;
use std::path::PathBuf;

use bevy::DefaultPlugins;
use bevy::asset::AssetMetaCheck;
use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::winit::WINIT_WINDOWS;
use mc_client::paths::ClientPaths;
use mc_client::{ClientConfig, ClientPlugin, StartAction};
use winit::window::Icon;

/// The parsed command line.
#[derive(Debug, Default)]
struct Args {
    data_dir: Option<PathBuf>,
    jar: Option<PathBuf>,
    resources: Option<PathBuf>,
    username: Option<String>,
    world: Option<String>,
    seed: Option<String>,
    import: Option<PathBuf>,
    script: Option<PathBuf>,
}

const USAGE: &str =
    "usage: mc_b173 [--data-dir DIR] [--jar FILE] [--resources DIR] [--username NAME]
               [--world NAME [--seed SEED] | --import DIR] [--script FILE.json]

  --data-dir DIR    options, saves, screenshots, stats (default: the platform data dir + mc_b173)
  --jar FILE        your Beta 1.7.3 minecraft.jar (textures, lang)
  --resources DIR   the resources/ directory (sounds, music)
  --username NAME   the player name (default: Player<millis % 1000>)
  --world NAME      open saves/NAME, creating it when missing (skips the title screen)
  --seed SEED       the new world's seed, as the Create World screen reads it
  --import DIR      import a vanilla save folder into saves/, then open it
  --script FILE     scripted-input dev mode (crates/mc-client/README.md)";

fn parse_args() -> Result<Args, String> {
    let mut a = Args::default();
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut value = |name: &str| it.next().ok_or_else(|| format!("{name} needs a value"));
        match arg.as_str() {
            "--data-dir" => a.data_dir = Some(value("--data-dir")?.into()),
            "--jar" => a.jar = Some(value("--jar")?.into()),
            "--resources" => a.resources = Some(value("--resources")?.into()),
            "--username" => a.username = Some(value("--username")?),
            "--world" => a.world = Some(value("--world")?),
            "--seed" => a.seed = Some(value("--seed")?),
            "--import" => a.import = Some(value("--import")?.into()),
            "--script" => a.script = Some(value("--script")?.into()),
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

/// Exits with a message (start-up errors: D8's missing jar, an import
/// refusal).
fn fail(msg: &str) -> ! {
    if !msg.is_empty() {
        eprintln!("mc_b173: {msg}");
    }
    eprintln!("{USAGE}");
    std::process::exit(if msg.is_empty() { 0 } else { 1 });
}

fn main() {
    let args = parse_args().unwrap_or_else(|e| fail(&e));
    let mut paths = ClientPaths::default();
    if let Some(d) = args.data_dir {
        paths.data_dir = d;
    }
    if let Some(j) = args.jar {
        paths.jar = j;
    }
    if let Some(r) = args.resources {
        paths.resources = r;
    }
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
                        // Bind to canvas included in `index.html`
                        canvas: Some("#bevy".to_owned()),
                        fit_canvas_to_parent: true,
                        // Tells wasm not to override default event handling, like F5 and Ctrl+R
                        prevent_default_event_handling: false,
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
