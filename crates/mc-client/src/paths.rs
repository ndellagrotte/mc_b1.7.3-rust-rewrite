//! Where the client's files are: the data directory (vanilla's
//! `Minecraft.getMinecraftDir()`, `~/.minecraft`), the user's `minecraft.jar`
//! and the `resources/` directory (D8). All three are settings
//! ([`ClientPaths`], set from the command line by `mc-app`).
//!
//! The data directory is project-owned and never the user's real
//! `.minecraft`: the platform's data-directory convention plus `mc_b173`
//! (`$XDG_DATA_HOME` or `~/.local/share` on Linux, `~/Library/Application
//! Support` on macOS, `%APPDATA%` on Windows). Vanilla's `getAppDir`
//! (Minecraft.java:359-387) picks `~/.minecraft` / `%APPDATA%\.minecraft` /
//! `~/Library/Application Support/minecraft` the same way. Inside it, as in
//! vanilla: `options.txt`, `saves/`, `screenshots/`, `stats/`.
//!
//! The jar and `resources/` default to the data directory too
//! (`<data-dir>/minecraft.jar`, `<data-dir>/resources`; WP-4.3, so a
//! packaged build finds them where `docs/RUNNING.md` says to put them).
//! Only when that file or directory does not exist does a development
//! fallback apply, and only if it exists itself: the jar of the vanilla
//! release checkout in this workspace (a path compiled in from
//! `CARGO_MANIFEST_DIR`, absent on any other machine) and the Prism
//! Launcher (Flatpak) `b1.7.3` instance's `resources/`. Otherwise the
//! data-directory path stays, and the start-up error names it (D8).

#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};

/// The data directory's name under the platform directory.
pub const DATA_DIR_NAME: &str = "mc_b173";

/// The saves directory inside the data directory (`new File(mcDataDir,
/// "saves")`, Minecraft.java:237), as a storage path.
pub const SAVES_DIR: &str = "saves";

/// The jar's file name in the data directory.
pub const JAR_NAME: &str = "minecraft.jar";

/// The `resources/` directory's name in the data directory.
pub const RESOURCES_NAME: &str = "resources";

/// The client's paths (module docs).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientPaths {
    /// The data directory.
    pub data_dir: PathBuf,
    /// The user's `minecraft.jar` (textures, `lang/`).
    pub jar: PathBuf,
    /// The `resources/` directory (sounds, music).
    pub resources: PathBuf,
}

impl Default for ClientPaths {
    fn default() -> Self {
        ClientPaths::for_data_dir(default_data_dir())
    }
}

impl ClientPaths {
    /// The paths for `data_dir`: its jar and `resources/` (module docs).
    pub fn for_data_dir(data_dir: PathBuf) -> Self {
        ClientPaths {
            jar: jar_for(&data_dir),
            resources: resources_for(&data_dir),
            data_dir,
        }
    }
}

/// The jar used without `--jar` (module docs): `<data_dir>/minecraft.jar`,
/// else the development jar if that exists, else `<data_dir>/minecraft.jar`.
pub fn jar_for(data_dir: &Path) -> PathBuf {
    pick(data_dir.join(JAR_NAME), dev_jar(), Path::is_file)
}

/// The `resources/` used without `--resources` (module docs):
/// `<data_dir>/resources`, else the Prism instance's if that exists, else
/// `<data_dir>/resources`.
pub fn resources_for(data_dir: &Path) -> PathBuf {
    pick(
        data_dir.join(RESOURCES_NAME),
        prism_resources(),
        Path::is_dir,
    )
}

/// `own` when it exists, else `fallback` when that exists, else `own`.
fn pick(own: PathBuf, fallback: PathBuf, exists: fn(&Path) -> bool) -> PathBuf {
    if !exists(&own) && exists(&fallback) {
        fallback
    } else {
        own
    }
}

/// The workspace root, from this crate's manifest directory at build time
/// (`crates/mc-client/../..`).
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The development jar: the vanilla release checkout's
/// `.reference/mc_b1.7.3_release/1.7.3-LTS/jars/bin/minecraft.jar` in the
/// workspace this binary was built from.
pub fn dev_jar() -> PathBuf {
    workspace_root().join(".reference/mc_b1.7.3_release/1.7.3-LTS/jars/bin/minecraft.jar")
}

/// The development `resources/`: the Prism Launcher (Flatpak) `b1.7.3`
/// instance's.
pub fn prism_resources() -> PathBuf {
    home_dir().join(
        ".var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher/instances/b1.7.3/minecraft/resources",
    )
}

/// The jar for the default data directory ([`jar_for`]).
pub fn default_jar() -> PathBuf {
    jar_for(&default_data_dir())
}

/// The `resources/` for the default data directory ([`resources_for`]).
pub fn default_resources() -> PathBuf {
    resources_for(&default_data_dir())
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// The platform data directory plus [`DATA_DIR_NAME`] (module docs).
pub fn default_data_dir() -> PathBuf {
    let base = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(home_dir)
    } else if cfg!(target_os = "macos") {
        home_dir().join("Library/Application Support")
    } else {
        match std::env::var_os("XDG_DATA_HOME") {
            Some(d) if Path::new(&d).is_absolute() => PathBuf::from(d),
            _ => home_dir().join(".local/share"),
        }
    };
    base.join(DATA_DIR_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_project_owned() {
        let p = ClientPaths::default();
        assert!(p.data_dir.ends_with(DATA_DIR_NAME));
        assert!(!p.data_dir.to_string_lossy().contains(".minecraft"));
        assert!(p.jar.ends_with(JAR_NAME));
        assert!(p.resources.ends_with(RESOURCES_NAME));
    }

    #[test]
    fn the_data_dir_comes_first_then_the_dev_fallbacks() {
        let d = crate::storage::tests::TempDir::new("paths");
        // Nothing in the data dir: the dev fallback if it exists, else the
        // data-dir path (which the start-up error then names).
        let p = ClientPaths::for_data_dir(d.0.clone());
        if dev_jar().is_file() {
            assert_eq!(p.jar, dev_jar());
        } else {
            assert_eq!(p.jar, d.0.join(JAR_NAME));
        }
        if prism_resources().is_dir() {
            assert_eq!(p.resources, prism_resources());
        } else {
            assert_eq!(p.resources, d.0.join(RESOURCES_NAME));
        }
        // Present in the data dir: always used.
        std::fs::write(d.0.join(JAR_NAME), b"jar").unwrap();
        std::fs::create_dir_all(d.0.join(RESOURCES_NAME)).unwrap();
        let p = ClientPaths::for_data_dir(d.0.clone());
        assert_eq!(p.jar, d.0.join(JAR_NAME));
        assert_eq!(p.resources, d.0.join(RESOURCES_NAME));
        // A fallback that does not exist is never picked.
        let missing = d.0.join("nope");
        assert_eq!(
            pick(missing.clone(), d.0.join("also-nope"), Path::is_file),
            missing
        );
    }
}
