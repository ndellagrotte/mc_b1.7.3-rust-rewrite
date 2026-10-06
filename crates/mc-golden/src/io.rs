//! Host file access, the only place in the crate that touches `std::fs`.
//!
//! `clippy.toml` bans `std::fs::{File, read, write, read_dir}` workspace-wide
//! to keep the simulation crates browser-ready (PLAN_v4 D9). `mc-golden` is a
//! host-only test-support crate (ADR-007 §7) and never ships in the sim or the
//! web build, so `lib.rs` allows those lints for this module alone.

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::GoldenError;

fn io_err(path: &Path) -> impl FnOnce(std::io::Error) -> GoldenError + '_ {
    move |source| GoldenError::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// Reads a whole file.
pub(crate) fn read_bytes(path: &Path) -> Result<Vec<u8>, GoldenError> {
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|mut f| f.read_to_end(&mut bytes))
        .map_err(io_err(path))?;
    Ok(bytes)
}

/// Reads a whole UTF-8 file.
pub(crate) fn read_text(path: &Path) -> Result<String, GoldenError> {
    let bytes = read_bytes(path)?;
    String::from_utf8(bytes).map_err(|e| GoldenError::Parse {
        path: path.to_path_buf(),
        line: None,
        message: format!("not UTF-8: {e}"),
    })
}

/// Writes a file, creating parent directories.
pub(crate) fn write_bytes(path: &Path, bytes: &[u8]) -> Result<(), GoldenError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(io_err(parent))?;
    }
    File::create(path)
        .and_then(|mut f| f.write_all(bytes))
        .map_err(io_err(path))
}

/// Sorted subdirectories of `path`; empty when `path` does not exist.
pub(crate) fn subdirs(path: &Path) -> Result<Vec<PathBuf>, GoldenError> {
    if !path.is_dir() {
        return Ok(Vec::new());
    }
    let mut dirs = Vec::new();
    for entry in std::fs::read_dir(path).map_err(io_err(path))? {
        let entry = entry.map_err(io_err(path))?;
        if entry.path().is_dir() {
            dirs.push(entry.path());
        }
    }
    dirs.sort();
    Ok(dirs)
}

/// Every file below `root`, as (`/`-separated path relative to `root`,
/// bytes), sorted by path. For sidecar directories (Tier S `world_read`).
pub(crate) fn read_tree(root: &Path) -> Result<Vec<(String, Vec<u8>)>, GoldenError> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) -> Result<(), GoldenError> {
        for entry in std::fs::read_dir(dir).map_err(io_err(dir))? {
            let path = entry.map_err(io_err(dir))?.path();
            if path.is_dir() {
                walk(root, &path, out)?;
            } else {
                let rel = path
                    .strip_prefix(root)
                    .expect("walked below root")
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                out.push((rel, read_bytes(&path)?));
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(root, root, &mut out)?;
    out.sort();
    Ok(out)
}
