//! The native [`Storage`] (PLAN_v4 D9): byte blobs as files under one root
//! directory (the data directory, [`crate::paths`]), and the wall
//! [`Clock`] the save code reads.
//!
//! - Paths are `/`-separated and relative, checked like
//!   [`mc_sim::save::MemoryStorage`]'s (no empty component, no `..`, not
//!   absolute).
//! - `write` is atomic: the bytes go to a temporary file in the same
//!   directory (`.mc_tmp-<name>-<pid>-<n>`, flushed to disk), which is then
//!   renamed over the target. A crash leaves either the old or the new
//!   file, never a torn one. Missing parent directories are created.
//! - `list` returns the sorted names of the files and directories directly
//!   inside a directory, without the temporaries; a missing directory lists
//!   empty.
//! - `delete` removes a file (a missing one is not an error) or an empty
//!   directory, then removes parent directories that became empty, up to
//!   the root: [`Storage`] has no empty directories, so `delete_tree`
//!   (SaveFormatOld.java:92-109) leaves nothing behind.
//!
//! This is the one place the client touches files for the simulation;
//! the `#[allow(clippy::disallowed_*)]` attributes mark it (clippy.toml).

#![allow(clippy::disallowed_types, clippy::disallowed_methods)]

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use mc_sim::save::{Clock, Storage, StorageError};

/// Prefix of the temporary files of an atomic write.
const TMP_PREFIX: &str = ".mc_tmp-";

/// Distinguishes concurrent temporaries of one process.
static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// [`Storage`] over a directory tree (module docs).
#[derive(Clone, Debug)]
pub struct FsStorage {
    root: PathBuf,
}

impl FsStorage {
    /// A storage rooted at `root` (created when missing).
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, StorageError> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(|e| backend(&root, e))?;
        Ok(FsStorage { root })
    }

    /// The root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The file-system path of a storage path (checked).
    pub fn fs_path(&self, path: &str) -> Result<PathBuf, StorageError> {
        check(path)?;
        let mut p = self.root.clone();
        for c in path.split('/') {
            p.push(c);
        }
        Ok(p)
    }

    /// Removes `dir` and its ancestors below the root while they are empty.
    fn prune_empty(&self, mut dir: PathBuf) {
        while dir != self.root && dir.starts_with(&self.root) {
            match fs::read_dir(&dir) {
                Ok(mut it) => {
                    if it.next().is_some() {
                        return;
                    }
                }
                Err(_) => return,
            }
            if fs::remove_dir(&dir).is_err() {
                return;
            }
            if !dir.pop() {
                return;
            }
        }
    }
}

fn check(path: &str) -> Result<(), StorageError> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path
            .split('/')
            .any(|c| c.is_empty() || c == ".." || c == ".")
    {
        Err(StorageError::InvalidPath(path.to_string()))
    } else {
        Ok(())
    }
}

fn backend(p: &Path, e: std::io::Error) -> StorageError {
    StorageError::Backend(format!("{}: {e}", p.display()))
}

impl Storage for FsStorage {
    fn read(&mut self, path: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let p = self.fs_path(path)?;
        match fs::metadata(&p) {
            Ok(m) if m.is_dir() => return Ok(None),
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(backend(&p, e)),
        }
        match fs::read(&p) {
            Ok(b) => Ok(Some(b)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(backend(&p, e)),
        }
    }

    fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), StorageError> {
        let p = self.fs_path(path)?;
        let dir = p
            .parent()
            .expect("a checked path has a parent")
            .to_path_buf();
        fs::create_dir_all(&dir).map_err(|e| backend(&dir, e))?;
        let name = p
            .file_name()
            .expect("a checked path has a file name")
            .to_string_lossy();
        let tmp = dir.join(format!(
            "{TMP_PREFIX}{name}-{}-{}",
            std::process::id(),
            TMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let result = (|| {
            let mut f = fs::File::create(&tmp)?;
            f.write_all(bytes)?;
            f.sync_all()?;
            drop(f);
            fs::rename(&tmp, &p)
        })();
        if let Err(e) = result {
            let _ = fs::remove_file(&tmp);
            return Err(backend(&p, e));
        }
        Ok(())
    }

    fn list(&mut self, dir: &str) -> Result<Vec<String>, StorageError> {
        let p = if dir.is_empty() {
            self.root.clone()
        } else {
            self.fs_path(dir)?
        };
        let it = match fs::read_dir(&p) {
            Ok(it) => it,
            Err(e)
                if e.kind() == std::io::ErrorKind::NotFound
                    || e.kind() == std::io::ErrorKind::NotADirectory =>
            {
                return Ok(Vec::new());
            }
            Err(e) => return Err(backend(&p, e)),
        };
        let mut names = Vec::new();
        for entry in it {
            let entry = entry.map_err(|e| backend(&p, e))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.starts_with(TMP_PREFIX) {
                names.push(name);
            }
        }
        names.sort();
        Ok(names)
    }

    fn rename(&mut self, from: &str, to: &str) -> Result<(), StorageError> {
        let f = self.fs_path(from)?;
        let t = self.fs_path(to)?;
        if !f.exists() {
            return Err(StorageError::NotFound(from.to_string()));
        }
        if let Some(d) = t.parent() {
            fs::create_dir_all(d).map_err(|e| backend(d, e))?;
        }
        fs::rename(&f, &t).map_err(|e| backend(&f, e))?;
        if let Some(d) = f.parent() {
            self.prune_empty(d.to_path_buf());
        }
        Ok(())
    }

    fn delete(&mut self, path: &str) -> Result<(), StorageError> {
        let p = self.fs_path(path)?;
        match fs::metadata(&p) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(backend(&p, e)),
            Ok(m) if m.is_dir() => {
                // Only an empty directory goes (a blob store has no others).
                if fs::read_dir(&p)
                    .map_err(|e| backend(&p, e))?
                    .next()
                    .is_none()
                {
                    fs::remove_dir(&p).map_err(|e| backend(&p, e))?;
                }
            }
            Ok(_) => fs::remove_file(&p).map_err(|e| backend(&p, e))?,
        }
        if let Some(d) = p.parent() {
            self.prune_empty(d.to_path_buf());
        }
        Ok(())
    }
}

/// `System.currentTimeMillis()` from the system clock: the session lock,
/// `LastPlayed` and the region timestamps ([`mc_sim::save`] "Time").
#[derive(Clone, Copy, Debug, Default)]
pub struct WallClock;

impl Clock for WallClock {
    fn current_time_millis(&mut self) -> i64 {
        crate::clock::current_time_millis()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A fresh directory under the system temp dir, removed on drop.
    pub(crate) struct TempDir(pub PathBuf);

    impl TempDir {
        pub(crate) fn new(tag: &str) -> Self {
            let p = std::env::temp_dir().join(format!(
                "mc_client_test_{tag}_{}_{}",
                std::process::id(),
                TMP_COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = fs::remove_dir_all(&p);
            fs::create_dir_all(&p).unwrap();
            TempDir(p)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn read_write_list_rename_delete() {
        let d = TempDir::new("storage");
        let mut s = FsStorage::new(&d.0).unwrap();
        assert_eq!(s.read("w/level.dat").unwrap(), None);
        s.write("w/level.dat", b"abc").unwrap();
        s.write("w/region/r.0.0.mcr", b"x").unwrap();
        assert_eq!(s.read("w/level.dat").unwrap().as_deref(), Some(&b"abc"[..]));
        assert_eq!(s.list("w").unwrap(), vec!["level.dat", "region"]);
        assert_eq!(s.list("").unwrap(), vec!["w"]);
        assert_eq!(s.list("nope").unwrap(), Vec::<String>::new());
        // A directory reads as absent.
        assert_eq!(s.read("w/region").unwrap(), None);
        // Overwrite is atomic and leaves no temporaries.
        s.write("w/level.dat", b"defg").unwrap();
        assert_eq!(
            s.read("w/level.dat").unwrap().as_deref(),
            Some(&b"defg"[..])
        );
        let raw: Vec<_> = fs::read_dir(d.0.join("w"))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(raw.iter().all(|n| !n.starts_with(TMP_PREFIX)), "{raw:?}");
        s.rename("w/level.dat", "w/level.dat_old").unwrap();
        assert_eq!(s.read("w/level.dat").unwrap(), None);
        assert_eq!(
            s.read("w/level.dat_old").unwrap().as_deref(),
            Some(&b"defg"[..])
        );
        assert!(matches!(
            s.rename("w/nope", "w/x"),
            Err(StorageError::NotFound(_))
        ));
        s.delete("w/level.dat_old").unwrap();
        s.delete("w/level.dat_old").unwrap();
        // Deleting the last file of a directory removes the directory.
        s.delete("w/region/r.0.0.mcr").unwrap();
        assert!(!d.0.join("w/region").exists());
        assert!(!d.0.join("w").exists());
        assert!(d.0.exists());
        assert!(matches!(s.read("/abs"), Err(StorageError::InvalidPath(_))));
        assert!(matches!(
            s.read("a/../b"),
            Err(StorageError::InvalidPath(_))
        ));
        assert!(matches!(
            s.write("", b""),
            Err(StorageError::InvalidPath(_))
        ));
    }

    #[test]
    fn delete_tree_of_the_save_format_empties_a_world() {
        let d = TempDir::new("tree");
        let mut s = FsStorage::new(&d.0).unwrap();
        s.write("saves/W/level.dat", b"1").unwrap();
        s.write("saves/W/region/r.0.0.mcr", b"2").unwrap();
        s.write("saves/W/DIM-1/region/r.0.0.mcr", b"3").unwrap();
        s.write("saves/Other/level.dat", b"4").unwrap();
        mc_sim::save::save_format_old::delete_tree(&mut s, "saves/W").unwrap();
        assert!(!d.0.join("saves/W").exists());
        assert_eq!(s.list("saves").unwrap(), vec!["Other"]);
    }

    #[test]
    fn the_save_handler_runs_on_it() {
        use mc_sim::save::{SaveHandler, SaveOldDir, SaveStore};
        use mc_sim::world::info::WorldInfo;
        let d = TempDir::new("handler");
        let store = SaveStore::shared(
            Box::new(FsStorage::new(&d.0).unwrap()),
            Box::new(mc_sim::save::FixedClock(1234)),
        );
        let mut h = SaveOldDir::new(store, "saves", "W").unwrap();
        h.check_session_lock().unwrap();
        let mut info = WorldInfo::new(42, "W");
        h.save_world_info_and_player(&mut info, None).unwrap();
        // Twice: the second goes through the level.dat_old rename dance.
        h.save_world_info_and_player(&mut info, None).unwrap();
        let back = h.load_world_info().unwrap();
        assert_eq!(back.random_seed, 42);
        assert!(d.0.join("saves/W/level.dat_old").exists());
        assert!(d.0.join("saves/W/session.lock").exists());
    }
}
