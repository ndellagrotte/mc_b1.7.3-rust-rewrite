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

#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};

/// The data directory's name under the platform directory.
pub const DATA_DIR_NAME: &str = "mc_b173";

/// The saves directory inside the data directory (`new File(mcDataDir,
/// "saves")`, Minecraft.java:237), as a storage path.
pub const SAVES_DIR: &str = "saves";

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
        ClientPaths {
            data_dir: default_data_dir(),
            jar: default_jar(),
            resources: default_resources(),
        }
    }
}

/// The workspace root, from this crate's manifest directory at build time
/// (`crates/mc-client/../..`).
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// D8's default jar: the vanilla release checkout's
/// `.reference/mc_b1.7.3_release/1.7.3-LTS/jars/bin/minecraft.jar` in the
/// workspace.
pub fn default_jar() -> PathBuf {
    workspace_root().join(".reference/mc_b1.7.3_release/1.7.3-LTS/jars/bin/minecraft.jar")
}

/// D8's default `resources/`: the Prism Launcher (Flatpak) instance's.
pub fn default_resources() -> PathBuf {
    home_dir().join(
        ".var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher/instances/b1.7.3/minecraft/resources",
    )
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
        assert!(p.jar.ends_with("jars/bin/minecraft.jar"));
        assert!(p.resources.ends_with("minecraft/resources"));
    }
}
