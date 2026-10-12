//! Where the client's files are: the data directory (vanilla's
//! `Minecraft.getMinecraftDir()`, `~/.minecraft`), the user's client jar
//! and the `resources/` directory (D8). All three are settings
//! ([`ClientPaths`], set from the command line by `mc-app`).
//!
//! The data directory is project-owned and never the user's real
//! `.minecraft`: the platform's data-directory convention plus `mc_b173`
//! (`$XDG_DATA_HOME` or `~/.local/share` on Linux, `~/Library/Application
//! Support` on macOS, `%APPDATA%` on Windows). Vanilla's `getAppDir`
//! (Minecraft.java:359-387) picks `~/.minecraft` / `%APPDATA%\.minecraft` /
//! `~/Library/Application Support/minecraft` the same way. Inside it, as in
//! vanilla: `options.txt`, `saves/`, `screenshots/`, `stats/`, plus the
//! project's `exports/`.
//!
//! Portable mode: a folder named `data` beside the game (the `.AppImage`
//! file, the `.app` bundle on macOS, else the executable) is the data
//! directory when it exists ([`portable_data_dir`]), so a portable zip keeps
//! its settings and saves with it (tools/portable). `--data-dir` overrides
//! it; without either, the platform directory above is used.
//!
//! The jar and `resources/` come from the user's Prism Launcher `b1.7.3`
//! instance (`.../PrismLauncher/instances/b1.7.3`), given with
//! `--prism-instance` (set by `mc-app`) or found in the standard Prism data
//! directories ([`prism_roots`], [`find_prism_instance`]):
//!
//! - the jar is Prism's library copy,
//!   `<instance>/../../libraries/com/mojang/minecraft/b1.7.3/minecraft-b1.7.3-client.jar`
//!   ([`prism_jar`]), which Prism downloads on the instance's first launch;
//! - `resources/` is `<instance>/minecraft/resources` (or
//!   `.minecraft/resources` in older instances, [`prism_resources`]).
//!
//! [`ClientPaths::from_prism_instance`] checks both and says what is missing
//! ([`PrismError`]).

#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};

/// The data directory's name under the platform directory.
pub const DATA_DIR_NAME: &str = "mc_b173";

/// The saves directory inside the data directory (`new File(mcDataDir,
/// "saves")`, Minecraft.java:237), as a storage path.
pub const SAVES_DIR: &str = "saves";

/// The portable data directory's name beside the game (module docs,
/// [`portable_data_dir`]).
pub const PORTABLE_DATA_DIR: &str = "data";

/// The Prism instance's directory name looked for under `instances/`.
pub const PRISM_INSTANCE_NAME: &str = "b1.7.3";

/// The client jar under the Prism data directory.
pub const PRISM_JAR_REL: &str = "libraries/com/mojang/minecraft/b1.7.3/minecraft-b1.7.3-client.jar";

/// The client's paths (module docs).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientPaths {
    /// The data directory.
    pub data_dir: PathBuf,
    /// The user's client jar (textures, `lang/`).
    pub jar: PathBuf,
    /// The `resources/` directory (sounds, music).
    pub resources: PathBuf,
}

/// Why a Prism instance cannot supply the jar and `resources/`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrismError {
    /// No `--prism-instance` and none in the standard places.
    NotFound {
        /// The instance directories tried.
        tried: Vec<PathBuf>,
    },
    /// The directory is not a Prism instance (no `instance.cfg`).
    NotAnInstance {
        /// The directory.
        path: PathBuf,
    },
    /// The instance is there but its jar and/or `resources/` are not: it has
    /// not been launched yet.
    NotLaunched {
        /// The instance directory.
        instance: PathBuf,
        /// The jar, when missing.
        jar: Option<PathBuf>,
        /// The `resources/` directory, when missing.
        resources: Option<PathBuf>,
    },
}

impl std::fmt::Display for PrismError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrismError::NotFound { tried } => {
                write!(
                    f,
                    "no Prism Launcher {PRISM_INSTANCE_NAME} instance found (tried "
                )?;
                for (i, p) in tried.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p.display())?;
                }
                write!(
                    f,
                    "); create a {PRISM_INSTANCE_NAME} instance in Prism Launcher, launch it once, and pass --prism-instance <.../PrismLauncher/instances/{PRISM_INSTANCE_NAME}> if it is elsewhere (docs/RUNNING.md)"
                )
            }
            PrismError::NotAnInstance { path } => write!(
                f,
                "{} is not a Prism Launcher instance directory (no instance.cfg); pass --prism-instance <.../PrismLauncher/instances/{PRISM_INSTANCE_NAME}> (docs/RUNNING.md)",
                path.display()
            ),
            PrismError::NotLaunched {
                instance,
                jar,
                resources,
            } => {
                write!(
                    f,
                    "found the Prism instance at {} but not ",
                    instance.display()
                )?;
                match (jar, resources) {
                    (Some(j), Some(r)) => write!(
                        f,
                        "the client jar at {} nor resources/ at {}",
                        j.display(),
                        r.display()
                    )?,
                    (Some(j), None) => write!(f, "the client jar at {}", j.display())?,
                    (None, Some(r)) => write!(f, "resources/ at {}", r.display())?,
                    (None, None) => write!(f, "everything")?,
                }
                write!(
                    f,
                    "; launch the {PRISM_INSTANCE_NAME} instance from Prism Launcher at least once so it downloads them, then try again"
                )
            }
        }
    }
}

impl std::error::Error for PrismError {}

impl ClientPaths {
    /// The paths for `data_dir` with the jar and `resources/` of the Prism
    /// instance at `instance` (module docs), checked.
    pub fn from_prism_instance(data_dir: PathBuf, instance: &Path) -> Result<Self, PrismError> {
        if !instance.join("instance.cfg").is_file() {
            return Err(PrismError::NotAnInstance {
                path: instance.to_path_buf(),
            });
        }
        let jar = prism_jar(instance);
        let resources = prism_resources(instance);
        let missing_jar = (!jar.is_file()).then(|| jar.clone());
        let missing_resources = (!resources.is_dir()).then(|| resources.clone());
        if missing_jar.is_some() || missing_resources.is_some() {
            return Err(PrismError::NotLaunched {
                instance: instance.to_path_buf(),
                jar: missing_jar,
                resources: missing_resources,
            });
        }
        Ok(ClientPaths {
            data_dir,
            jar,
            resources,
        })
    }
}

/// The client jar of the instance at `instance`: [`PRISM_JAR_REL`] under the
/// Prism data directory two levels up (`<root>/instances/<name>`).
pub fn prism_jar(instance: &Path) -> PathBuf {
    let instance = std::fs::canonicalize(instance).unwrap_or_else(|_| instance.to_path_buf());
    match instance.ancestors().nth(2) {
        Some(root) if !root.as_os_str().is_empty() => root.join(PRISM_JAR_REL),
        _ => instance.join("../..").join(PRISM_JAR_REL),
    }
}

/// The `resources/` of the instance at `instance`: `minecraft/resources`,
/// else `.minecraft/resources` when only that exists.
pub fn prism_resources(instance: &Path) -> PathBuf {
    let r = instance.join("minecraft/resources");
    let dot = instance.join(".minecraft/resources");
    if !r.is_dir() && dot.is_dir() { dot } else { r }
}

/// The standard Prism Launcher data directories, in lookup order: on Linux
/// `$XDG_DATA_HOME/PrismLauncher` (or `~/.local/share/PrismLauncher`), then
/// the Flatpak's; on macOS `~/Library/Application Support/PrismLauncher`; on
/// Windows `%APPDATA%\PrismLauncher`.
pub fn prism_roots() -> Vec<PathBuf> {
    if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA")
            .map(|d| vec![PathBuf::from(d).join("PrismLauncher")])
            .unwrap_or_default()
    } else if cfg!(target_os = "macos") {
        vec![home_dir().join("Library/Application Support/PrismLauncher")]
    } else {
        vec![
            xdg_data_home().join("PrismLauncher"),
            home_dir().join(".var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher"),
        ]
    }
}

/// The instance directories looked for: `<root>/instances/b1.7.3` for each
/// of [`prism_roots`].
pub fn prism_instance_candidates() -> Vec<PathBuf> {
    instance_candidates(&prism_roots())
}

fn instance_candidates(roots: &[PathBuf]) -> Vec<PathBuf> {
    roots
        .iter()
        .map(|r| r.join("instances").join(PRISM_INSTANCE_NAME))
        .collect()
}

/// The first of [`prism_instance_candidates`] that is a directory.
pub fn find_prism_instance() -> Option<PathBuf> {
    find_instance_in(&prism_roots())
}

fn find_instance_in(roots: &[PathBuf]) -> Option<PathBuf> {
    instance_candidates(roots).into_iter().find(|p| p.is_dir())
}

/// The jar of the auto-detected instance ([`find_prism_instance`]), or a
/// path that does not exist (tests against the real jar skip then).
pub fn default_jar() -> PathBuf {
    find_prism_instance()
        .map(|i| prism_jar(&i))
        .unwrap_or_else(|| default_data_dir().join("no-prism-instance/client.jar"))
}

/// The `resources/` of the auto-detected instance, or a path that does not
/// exist ([`default_jar`]).
pub fn default_resources() -> PathBuf {
    find_prism_instance()
        .map(|i| prism_resources(&i))
        .unwrap_or_else(|| default_data_dir().join("no-prism-instance/resources"))
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// `$XDG_DATA_HOME` when absolute, else `~/.local/share`.
fn xdg_data_home() -> PathBuf {
    match std::env::var_os("XDG_DATA_HOME") {
        Some(d) if Path::new(&d).is_absolute() => PathBuf::from(d),
        _ => home_dir().join(".local/share"),
    }
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
        xdg_data_home()
    };
    base.join(DATA_DIR_NAME)
}

/// The directory a portable build's [`PORTABLE_DATA_DIR`] sits in, from the
/// executable's path: beside the `.AppImage` file when `exe` runs from inside
/// the mounted image (`appimage` and `appdir` are the AppImage runtime's
/// `$APPIMAGE` and `$APPDIR`), beside the `.app` bundle when `exe` is in
/// `<X>.app/Contents/MacOS/`, else beside `exe`. Pure: the caller supplies
/// the environment ([`portable_data_dir`]).
pub fn portable_base(
    exe: &Path,
    appimage: Option<&Path>,
    appdir: Option<&Path>,
) -> Option<PathBuf> {
    let exe_dir = exe.parent()?;
    if let (Some(image), Some(dir)) = (appimage, appdir)
        && !dir.as_os_str().is_empty()
        && exe.starts_with(dir)
    {
        return image.parent().map(Path::to_path_buf);
    }
    if exe_dir.ends_with("Contents/MacOS")
        && let Some(app) = exe_dir.parent().and_then(Path::parent)
        && app.extension().is_some_and(|e| e == "app")
    {
        return app.parent().map(Path::to_path_buf);
    }
    Some(exe_dir.to_path_buf())
}

/// `<base>/data` when it is a directory.
fn existing_portable_dir(base: &Path) -> Option<PathBuf> {
    let d = base.join(PORTABLE_DATA_DIR);
    d.is_dir().then_some(d)
}

/// The portable data directory (module docs): [`PORTABLE_DATA_DIR`] in the
/// [`portable_base`] of `std::env::current_exe` (already canonical on Linux,
/// via `/proc/self/exe`; canonicalized on macOS, where a symlinked launcher
/// would otherwise hide the bundle; left alone on Windows, where
/// canonicalizing adds a `\\?\` prefix), `$APPIMAGE` and `$APPDIR`, when that
/// folder exists. `$APPDIR` is canonicalized: in extract-and-run mode it
/// lives under `$TMPDIR`, which may hold symlinks. `mc-app` prefers this to
/// [`default_data_dir`]; `--data-dir` overrides both.
pub fn portable_data_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    #[cfg(target_os = "macos")]
    let exe = std::fs::canonicalize(&exe).unwrap_or(exe);
    let appimage = std::env::var_os("APPIMAGE").map(PathBuf::from);
    let appdir = std::env::var_os("APPDIR").map(|d| {
        let d = PathBuf::from(d);
        std::fs::canonicalize(&d).unwrap_or(d)
    });
    existing_portable_dir(&portable_base(
        &exe,
        appimage.as_deref(),
        appdir.as_deref(),
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::tests::TempDir;

    #[test]
    fn data_dir_is_project_owned() {
        let d = default_data_dir();
        assert!(d.ends_with(DATA_DIR_NAME));
        assert!(!d.to_string_lossy().contains(".minecraft"));
    }

    /// A Prism data directory under `root` with a `b1.7.3` instance: its
    /// `instance.cfg`, and the jar and `<mc>/resources` as asked.
    fn fake_prism(root: &Path, jar: bool, mc: Option<&str>) -> PathBuf {
        let inst = root.join("instances").join(PRISM_INSTANCE_NAME);
        std::fs::create_dir_all(&inst).unwrap();
        std::fs::write(inst.join("instance.cfg"), b"[General]\n").unwrap();
        if jar {
            let j = root.join(PRISM_JAR_REL);
            std::fs::create_dir_all(j.parent().unwrap()).unwrap();
            std::fs::write(&j, b"jar").unwrap();
        }
        if let Some(mc) = mc {
            std::fs::create_dir_all(inst.join(mc).join("resources")).unwrap();
        }
        inst
    }

    #[test]
    fn a_launched_instance_gives_the_jar_and_resources() {
        let r = TempDir::new("prism_ok");
        let inst = fake_prism(&r.0, true, Some("minecraft"));
        let p = ClientPaths::from_prism_instance(PathBuf::from("/data"), &inst).unwrap();
        assert_eq!(p.data_dir, PathBuf::from("/data"));
        assert!(p.jar.is_file());
        assert!(p.jar.ends_with("minecraft-b1.7.3-client.jar"));
        assert_eq!(p.resources, inst.join("minecraft/resources"));
    }

    #[test]
    fn dot_minecraft_resources_in_older_instances() {
        let r = TempDir::new("prism_dot");
        let inst = fake_prism(&r.0, true, Some(".minecraft"));
        let p = ClientPaths::from_prism_instance(PathBuf::from("/data"), &inst).unwrap();
        assert_eq!(p.resources, inst.join(".minecraft/resources"));
    }

    #[test]
    fn a_missing_jar_says_to_launch_the_instance() {
        let r = TempDir::new("prism_nojar");
        let inst = fake_prism(&r.0, false, Some("minecraft"));
        let e = ClientPaths::from_prism_instance(PathBuf::from("/data"), &inst).unwrap_err();
        assert!(matches!(
            &e,
            PrismError::NotLaunched {
                jar: Some(_),
                resources: None,
                ..
            }
        ));
        let s = e.to_string();
        assert!(s.contains("minecraft-b1.7.3-client.jar"), "{s}");
        assert!(s.contains("at least once"), "{s}");
        // Nothing downloaded yet: both named.
        let r = TempDir::new("prism_empty");
        let inst = fake_prism(&r.0, false, None);
        let e = ClientPaths::from_prism_instance(PathBuf::from("/data"), &inst).unwrap_err();
        assert!(matches!(
            e,
            PrismError::NotLaunched {
                jar: Some(_),
                resources: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn a_directory_without_instance_cfg_is_refused() {
        let r = TempDir::new("prism_notinst");
        let e = ClientPaths::from_prism_instance(PathBuf::from("/data"), &r.0).unwrap_err();
        assert_eq!(e, PrismError::NotAnInstance { path: r.0.clone() });
        assert!(e.to_string().contains("--prism-instance"));
    }

    #[test]
    fn the_first_root_with_the_instance_is_found() {
        let a = TempDir::new("prism_root_a");
        let b = TempDir::new("prism_root_b");
        let roots = [a.0.join("PrismLauncher"), b.0.join("PrismLauncher")];
        assert_eq!(find_instance_in(&roots), None);
        let inst_b = fake_prism(&roots[1], true, Some("minecraft"));
        assert_eq!(find_instance_in(&roots), Some(inst_b));
        let inst_a = fake_prism(&roots[0], false, None);
        assert_eq!(find_instance_in(&roots), Some(inst_a));
        assert_eq!(
            instance_candidates(&roots)[0],
            roots[0].join("instances/b1.7.3")
        );
    }

    fn base(exe: &str, appimage: Option<&str>, appdir: Option<&str>) -> Option<PathBuf> {
        portable_base(
            Path::new(exe),
            appimage.map(Path::new),
            appdir.map(Path::new),
        )
    }

    #[test]
    fn appimage_in_appdir_uses_appimage_parent() {
        assert_eq!(
            base(
                "/tmp/.mount_ab/usr/bin/mc_b173",
                Some("/home/u/Games/mc_b173.AppImage"),
                Some("/tmp/.mount_ab"),
            ),
            Some(PathBuf::from("/home/u/Games"))
        );
    }

    #[test]
    fn appimage_env_ignored_when_exe_outside_appdir() {
        let image = Some("/home/u/Games/mc_b173.AppImage");
        let exe = "/opt/mc/mc_b173";
        let opt = Some(PathBuf::from("/opt/mc"));
        assert_eq!(base(exe, image, Some("/tmp/.mount_ab")), opt);
        assert_eq!(base(exe, image, None), opt);
        // An empty $APPDIR would be a prefix of every path.
        assert_eq!(base(exe, image, Some("")), opt);
    }

    #[test]
    fn mac_bundle_uses_bundle_parent() {
        assert_eq!(
            base("/Users/u/mc/mc_b173.app/Contents/MacOS/mc_b173", None, None),
            Some(PathBuf::from("/Users/u/mc"))
        );
        // A `Contents/MacOS/` outside a `.app` is a plain directory.
        assert_eq!(
            base("/Users/u/bin/Contents/MacOS/mc_b173", None, None),
            Some(PathBuf::from("/Users/u/bin/Contents/MacOS"))
        );
    }

    #[test]
    fn plain_exe_uses_exe_dir() {
        assert_eq!(
            base("/games/mc/mc_b173.exe", None, None),
            Some(PathBuf::from("/games/mc"))
        );
    }

    #[test]
    fn portable_needs_existing_data_dir() {
        let r = TempDir::new("portable");
        let exe = r.0.join("mc_b173");
        let b = portable_base(&exe, None, None).unwrap();
        assert_eq!(b, r.0);
        assert_eq!(existing_portable_dir(&b), None);
        // A file named data is not the data directory.
        std::fs::write(b.join(PORTABLE_DATA_DIR), b"").unwrap();
        assert_eq!(existing_portable_dir(&b), None);
        std::fs::remove_file(b.join(PORTABLE_DATA_DIR)).unwrap();
        std::fs::create_dir(b.join(PORTABLE_DATA_DIR)).unwrap();
        assert_eq!(existing_portable_dir(&b), Some(r.0.join("data")));
    }

    #[test]
    fn not_found_names_the_places_tried() {
        let e = PrismError::NotFound {
            tried: vec![PathBuf::from("/a/instances/b1.7.3"), PathBuf::from("/b")],
        };
        let s = e.to_string();
        assert!(s.contains("/a/instances/b1.7.3, /b"), "{s}");
        assert!(s.contains("--prism-instance"), "{s}");
    }
}
