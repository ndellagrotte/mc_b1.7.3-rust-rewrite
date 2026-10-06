//! The `Storage` trait (PLAN_v4 D9): the only way the simulation touches
//! persistent bytes. Paths are `/`-separated, relative to the storage root
//! (e.g. `"world/level.dat"`, `"world/region/r.0.0.mcr"`). The native
//! implementation (files, atomic write-then-rename) lives in `mc-client`; the
//! browser one (IndexedDB/OPFS) in Phase 5. [`MemoryStorage`] is for tests.

use std::collections::BTreeMap;
use std::fmt;

/// Why a storage operation failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StorageError {
    /// The path does not exist (for operations that require it).
    NotFound(String),
    /// The path is malformed (empty, absolute, `..`, …).
    InvalidPath(String),
    /// Any backend failure, with its message.
    Backend(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::NotFound(p) => write!(f, "not found: {p}"),
            StorageError::InvalidPath(p) => write!(f, "invalid path: {p}"),
            StorageError::Backend(m) => write!(f, "storage error: {m}"),
        }
    }
}

impl std::error::Error for StorageError {}

/// Byte-blob storage (D9).
pub trait Storage {
    /// The whole blob at `path`, or `None` if it does not exist.
    fn read(&mut self, path: &str) -> Result<Option<Vec<u8>>, StorageError>;
    /// Creates or replaces the blob at `path`.
    fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), StorageError>;
    /// The names (not paths) of the blobs and directories directly inside
    /// `dir`, sorted.
    fn list(&mut self, dir: &str) -> Result<Vec<String>, StorageError>;
    /// Moves `from` to `to`, replacing `to`.
    fn rename(&mut self, from: &str, to: &str) -> Result<(), StorageError>;
    /// Deletes the blob at `path`; deleting a missing blob is not an error.
    fn delete(&mut self, path: &str) -> Result<(), StorageError>;
}

/// In-memory [`Storage`] over a `BTreeMap`, for tests and headless runs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MemoryStorage {
    /// Blobs by full path.
    pub blobs: BTreeMap<String, Vec<u8>>,
}

fn check(path: &str) -> Result<(), StorageError> {
    if path.is_empty()
        || path.starts_with('/')
        || path.split('/').any(|c| c.is_empty() || c == "..")
    {
        Err(StorageError::InvalidPath(path.to_string()))
    } else {
        Ok(())
    }
}

impl Storage for MemoryStorage {
    fn read(&mut self, path: &str) -> Result<Option<Vec<u8>>, StorageError> {
        check(path)?;
        Ok(self.blobs.get(path).cloned())
    }

    fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), StorageError> {
        check(path)?;
        self.blobs.insert(path.to_string(), bytes.to_vec());
        Ok(())
    }

    fn list(&mut self, dir: &str) -> Result<Vec<String>, StorageError> {
        let prefix = if dir.is_empty() {
            String::new()
        } else {
            check(dir)?;
            format!("{dir}/")
        };
        let mut names: Vec<String> = self
            .blobs
            .keys()
            .filter_map(|k| k.strip_prefix(&prefix))
            .map(|rest| rest.split('/').next().unwrap_or(rest).to_string())
            .collect();
        names.dedup();
        Ok(names)
    }

    fn rename(&mut self, from: &str, to: &str) -> Result<(), StorageError> {
        check(from)?;
        check(to)?;
        let v = self
            .blobs
            .remove(from)
            .ok_or_else(|| StorageError::NotFound(from.to_string()))?;
        self.blobs.insert(to.to_string(), v);
        Ok(())
    }

    fn delete(&mut self, path: &str) -> Result<(), StorageError> {
        check(path)?;
        self.blobs.remove(path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_storage_roundtrip() {
        let mut s = MemoryStorage::default();
        assert_eq!(s.read("w/level.dat").unwrap(), None);
        s.write("w/level.dat", b"abc").unwrap();
        s.write("w/region/r.0.0.mcr", b"x").unwrap();
        assert_eq!(s.read("w/level.dat").unwrap().as_deref(), Some(&b"abc"[..]));
        assert_eq!(s.list("w").unwrap(), vec!["level.dat", "region"]);
        s.rename("w/level.dat", "w/level.dat_old").unwrap();
        assert_eq!(s.read("w/level.dat").unwrap(), None);
        s.delete("w/level.dat_old").unwrap();
        s.delete("w/level.dat_old").unwrap();
        assert!(matches!(s.read("/abs"), Err(StorageError::InvalidPath(_))));
        assert!(matches!(
            s.rename("nope", "x"),
            Err(StorageError::NotFound(_))
        ));
    }
}
