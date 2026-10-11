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
//! The jar and `resources/` are looked for in this order (so a packaged
//! build finds them where `docs/RUNNING.md` says to put them):
//!
//! 1. `--jar` / `--resources` (set by `mc-app`).
//! 2. Next to the bundle ([`bundle_dir`]): the `.AppImage` file's
//!    directory, else the executable's. There `minecraft.jar` by name,
//!    else the single `*.jar` containing `terrain.png` (a renamed client
//!    jar; when several such jars are there none is used, and a stderr line
//!    says so) ([`bundle_jar`]), and `resources/` ([`bundle_resources`]).
//! 3. `<data-dir>/minecraft.jar`, `<data-dir>/resources` (WP-4.3).
//! 4. The development fallbacks, each only if it exists: the jar of the
//!    vanilla release checkout in this workspace (a path compiled in from
//!    `CARGO_MANIFEST_DIR`, absent on any other machine) and the Prism
//!    Launcher (Flatpak) `b1.7.3` instance's `resources/`.
//!
//! Otherwise the data-directory path stays, and the start-up error names it
//! (D8).

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

/// The jar used without `--jar` (module docs): the one next to the bundle
/// ([`bundle_jar`]), else `<data_dir>/minecraft.jar`, else the development
/// jar if that exists, else `<data_dir>/minecraft.jar`.
pub fn jar_for(data_dir: &Path) -> PathBuf {
    jar_in(bundle_dir().as_deref(), data_dir)
}

/// The `resources/` used without `--resources` (module docs): the one next
/// to the bundle ([`bundle_resources`]), else `<data_dir>/resources`, else
/// the Prism instance's if that exists, else `<data_dir>/resources`.
pub fn resources_for(data_dir: &Path) -> PathBuf {
    resources_in(bundle_dir().as_deref(), data_dir)
}

/// [`jar_for`] with the bundle directory given.
fn jar_in(bundle: Option<&Path>, data_dir: &Path) -> PathBuf {
    bundle
        .and_then(bundle_jar)
        .unwrap_or_else(|| pick(data_dir.join(JAR_NAME), dev_jar(), Path::is_file))
}

/// [`resources_for`] with the bundle directory given.
fn resources_in(bundle: Option<&Path>, data_dir: &Path) -> PathBuf {
    bundle.and_then(bundle_resources).unwrap_or_else(|| {
        pick(
            data_dir.join(RESOURCES_NAME),
            prism_resources(),
            Path::is_dir,
        )
    })
}

/// The directory the shipped bundle sits in (module docs): the `.AppImage`
/// file's directory when running from an AppImage, else the executable's
/// (the unpacked tarball's). `None` when unknown.
pub fn bundle_dir() -> Option<PathBuf> {
    let canonical = |p: PathBuf| std::fs::canonicalize(&p).unwrap_or(p);
    bundle_dir_from(
        std::env::var_os("APPIMAGE").map(PathBuf::from),
        std::env::var_os("APPDIR").map(PathBuf::from).map(canonical),
        std::env::current_exe().ok().map(canonical),
    )
}

/// [`bundle_dir`] from its inputs. The AppImage runtime sets `APPIMAGE` (the
/// file) and `APPDIR` (the mount), and the executable runs from inside the
/// mount; `APPIMAGE` counts only when the executable is inside `APPDIR`, so
/// variables inherited from an AppImage-launched parent (an IDE's terminal)
/// do not redirect the lookup.
fn bundle_dir_from(
    appimage: Option<PathBuf>,
    appdir: Option<PathBuf>,
    exe: Option<PathBuf>,
) -> Option<PathBuf> {
    let exe = exe?;
    let file = match (&appimage, &appdir) {
        (Some(image), Some(dir)) if exe.starts_with(dir) => image.as_path(),
        _ => exe.as_path(),
    };
    file.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
}

/// The jar next to the bundle in `dir` (module docs): `dir/minecraft.jar`
/// when it is a file (not probed, like the data directory's), else the only
/// `*.jar` file there (extension in any case) that contains `terrain.png`
/// ([`crate::assets::jar_has_terrain`]). Several such jars: none, with a
/// stderr line naming two of them.
pub fn bundle_jar(dir: &Path) -> Option<PathBuf> {
    let named = dir.join(JAR_NAME);
    if named.is_file() {
        return Some(named);
    }
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("jar")))
        .collect();
    candidates.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    let found: Vec<PathBuf> = candidates
        .into_iter()
        .filter(|p| crate::assets::jar_has_terrain(p))
        .take(2)
        .collect();
    match found.as_slice() {
        [] => None,
        [one] => Some(one.clone()),
        [a, b, ..] => {
            eprintln!(
                "mc_b173: several jars with terrain.png in {}: {} and {}; using none (rename yours to minecraft.jar or pass --jar)",
                dir.display(),
                a.display(),
                b.display()
            );
            None
        }
    }
}

/// The `resources/` next to the bundle in `dir` (module docs), when it is a
/// directory.
pub fn bundle_resources(dir: &Path) -> Option<PathBuf> {
    let r = dir.join(RESOURCES_NAME);
    r.is_dir().then_some(r)
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

    use crate::storage::tests::TempDir;

    /// Writes a zip with `entries` (each holding one byte) to `path`.
    fn write_jar(path: &Path, entries: &[&str]) {
        use std::io::Write;
        let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for n in entries {
            w.start_file(*n, zip::write::SimpleFileOptions::default())
                .unwrap();
            w.write_all(b"x").unwrap();
        }
        std::fs::write(path, w.finish().unwrap().into_inner()).unwrap();
    }

    /// The data dir's own jar and resources/, so a fall-through lands there
    /// whatever development fallbacks this machine has.
    fn fill_data_dir(d: &Path) {
        std::fs::write(d.join(JAR_NAME), b"jar").unwrap();
        std::fs::create_dir_all(d.join(RESOURCES_NAME)).unwrap();
    }

    #[test]
    fn bundle_dir_prefers_the_appimage_file() {
        let p = |s: &str| Some(PathBuf::from(s));
        assert_eq!(
            bundle_dir_from(
                p("/x/y/g.AppImage"),
                p("/tmp/.mount_a"),
                p("/tmp/.mount_a/usr/bin/mc_b173")
            ),
            p("/x/y")
        );
        // Inherited AppImage variables: the executable is outside APPDIR.
        assert_eq!(
            bundle_dir_from(
                p("/x/y/g.AppImage"),
                p("/tmp/.mount_a"),
                p("/opt/game/mc_b173")
            ),
            p("/opt/game")
        );
        assert_eq!(
            bundle_dir_from(None, p("/tmp/.mount_a"), p("/tmp/.mount_a/usr/bin/mc_b173")),
            p("/tmp/.mount_a/usr/bin")
        );
        assert_eq!(bundle_dir_from(None, None, None), None);
        assert_eq!(bundle_dir_from(None, None, p("mc_b173")), None);
    }

    #[test]
    fn a_minecraft_jar_beside_the_bundle_beats_the_data_dir() {
        let b = TempDir::new("paths_b_named");
        let d = TempDir::new("paths_d_named");
        fill_data_dir(&d.0);
        assert_eq!(jar_in(Some(&b.0), &d.0), d.0.join(JAR_NAME));
        // Not probed: any file named minecraft.jar counts.
        std::fs::write(b.0.join(JAR_NAME), b"jar").unwrap();
        assert_eq!(jar_in(Some(&b.0), &d.0), b.0.join(JAR_NAME));
    }

    #[test]
    fn a_renamed_jar_is_used_when_it_is_the_only_one_with_terrain() {
        let b = TempDir::new("paths_b_renamed");
        let d = TempDir::new("paths_d_renamed");
        fill_data_dir(&d.0);
        write_jar(&b.0.join("client.jar"), &["terrain.png", "pack.png"]);
        write_jar(&b.0.join("other.jar"), &["pack.png"]);
        std::fs::write(b.0.join("notes.txt"), b"notes").unwrap();
        std::fs::write(b.0.join("fake.jar"), b"not a zip").unwrap();
        assert_eq!(jar_in(Some(&b.0), &d.0), b.0.join("client.jar"));

        let b = TempDir::new("paths_b_upper");
        write_jar(&b.0.join("B.JAR"), &["/terrain.png"]);
        assert_eq!(jar_in(Some(&b.0), &d.0), b.0.join("B.JAR"));
    }

    #[test]
    fn two_candidate_jars_fall_through() {
        let b = TempDir::new("paths_b_two");
        let d = TempDir::new("paths_d_two");
        fill_data_dir(&d.0);
        write_jar(&b.0.join("a.jar"), &["terrain.png"]);
        write_jar(&b.0.join("c.jar"), &["terrain.png"]);
        assert_eq!(bundle_jar(&b.0), None);
        assert_eq!(jar_in(Some(&b.0), &d.0), d.0.join(JAR_NAME));
    }

    #[test]
    fn minecraft_jar_beats_renamed_candidates() {
        let b = TempDir::new("paths_b_beats");
        let d = TempDir::new("paths_d_beats");
        fill_data_dir(&d.0);
        write_jar(&b.0.join("a.jar"), &["terrain.png"]);
        write_jar(&b.0.join("c.jar"), &["terrain.png"]);
        write_jar(&b.0.join(JAR_NAME), &["terrain.png"]);
        assert_eq!(jar_in(Some(&b.0), &d.0), b.0.join(JAR_NAME));
    }

    #[test]
    fn resources_beside_the_bundle() {
        let b = TempDir::new("paths_b_res");
        let d = TempDir::new("paths_d_res");
        fill_data_dir(&d.0);
        std::fs::create_dir_all(b.0.join(RESOURCES_NAME)).unwrap();
        assert_eq!(resources_in(Some(&b.0), &d.0), b.0.join(RESOURCES_NAME));
        // A file named resources is not the directory.
        let b = TempDir::new("paths_b_res_file");
        std::fs::write(b.0.join(RESOURCES_NAME), b"file").unwrap();
        assert_eq!(resources_in(Some(&b.0), &d.0), d.0.join(RESOURCES_NAME));
    }

    #[test]
    fn the_data_dir_comes_first_then_the_dev_fallbacks() {
        let d = TempDir::new("paths");
        // Nothing in the data dir: the dev fallback if it exists, else the
        // data-dir path (which the start-up error then names).
        let (jar, resources) = (jar_in(None, &d.0), resources_in(None, &d.0));
        if dev_jar().is_file() {
            assert_eq!(jar, dev_jar());
        } else {
            assert_eq!(jar, d.0.join(JAR_NAME));
        }
        if prism_resources().is_dir() {
            assert_eq!(resources, prism_resources());
        } else {
            assert_eq!(resources, d.0.join(RESOURCES_NAME));
        }
        // Present in the data dir: always used.
        fill_data_dir(&d.0);
        assert_eq!(jar_in(None, &d.0), d.0.join(JAR_NAME));
        assert_eq!(resources_in(None, &d.0), d.0.join(RESOURCES_NAME));
        // A fallback that does not exist is never picked.
        let missing = d.0.join("nope");
        assert_eq!(
            pick(missing.clone(), d.0.join("also-nope"), Path::is_file),
            missing
        );
    }
}
