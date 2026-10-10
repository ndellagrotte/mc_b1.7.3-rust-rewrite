//! Running a fixture (ADR-007 §7): load, drive, write `actual.jsonl`, compare.

use std::path::{Component, Path, PathBuf};

use crate::trace::parse_jsonl;
use crate::{GoldenError, Scenario, TraceWriter, compare, io};

/// The workspace root (two levels above this crate's manifest).
pub fn workspace_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(Path::parent)
        .unwrap_or(manifest)
        .to_path_buf()
}

/// `CARGO_TARGET_DIR` when set in the environment (relative values resolve
/// against the workspace root), else `<workspace>/target`.
///
/// Uses `var_os`; `std::env::var` is banned by clippy.toml (PLAN_v4 D9).
pub fn target_dir() -> PathBuf {
    match std::env::var_os("CARGO_TARGET_DIR") {
        Some(dir) if !dir.is_empty() => workspace_root().join(dir),
        _ => workspace_root().join("target"),
    }
}

/// `<workspace>/tests/golden/<area>/<name>`.
pub fn fixture_dir(area: &str, name: &str) -> PathBuf {
    workspace_root().join("tests/golden").join(area).join(name)
}

/// Every `tests/golden/<area>/<name>` directory that holds a `scenario.json`,
/// sorted.
pub fn all_fixture_dirs() -> Result<Vec<PathBuf>, GoldenError> {
    let mut out = Vec::new();
    for area in io::subdirs(&workspace_root().join("tests/golden"))? {
        for dir in io::subdirs(&area)? {
            if dir.join("scenario.json").is_file() {
                out.push(dir);
            }
        }
    }
    Ok(out)
}

/// Reads a sidecar binary named by a trace line's `"file"` field. `name` must
/// be a plain file name inside the fixture directory.
pub fn read_sidecar(dir: impl AsRef<Path>, name: &str) -> Result<Vec<u8>, GoldenError> {
    let dir = resolve(dir.as_ref());
    let mut components = Path::new(name).components();
    if !matches!(
        (components.next(), components.next()),
        (Some(Component::Normal(_)), None)
    ) {
        return Err(GoldenError::other(format!(
            "sidecar name {name:?} is not a plain file name"
        )));
    }
    io::read_bytes(&dir.join(name))
}

/// Relative fixture paths are taken from the workspace root, so tests can
/// write `run("tests/golden/math/sine-table", ..)` as in ADR-007 §7.
fn resolve(dir: &Path) -> PathBuf {
    if dir.is_absolute() {
        dir.to_path_buf()
    } else {
        workspace_root().join(dir)
    }
}

/// Runs one fixture.
///
/// Loads `<dir>/scenario.json` and `<dir>/expected.jsonl`, calls `drive` with
/// the scenario and an empty [`TraceWriter`], writes what it emitted to
/// `<target>/golden/<area>/<name>/actual.jsonl` (the last two components of
/// `dir`; written even when `drive` fails, for inspection), then compares.
/// A difference is returned as [`GoldenError::Mismatch`], whose `Display`
/// shows the first differing line and field and both file paths.
pub fn run<F>(dir: impl AsRef<Path>, drive: F) -> Result<(), GoldenError>
where
    F: FnOnce(&Scenario, &mut TraceWriter) -> Result<(), GoldenError>,
{
    let dir = resolve(dir.as_ref());
    let scenario = Scenario::load(&dir)?;
    let expected_path = dir.join("expected.jsonl");
    let expected =
        parse_jsonl(&io::read_text(&expected_path)?).map_err(|e| GoldenError::Parse {
            path: expected_path.clone(),
            line: Some(e.line),
            message: e.message,
        })?;

    let mut out = TraceWriter::new();
    let driven = drive(&scenario, &mut out);

    let actual_path = actual_dir(&dir).join("actual.jsonl");
    io::write_bytes(&actual_path, out.to_jsonl().as_bytes())?;
    // The gaps the drive reached (crate::gaps), next to actual.jsonl; a
    // stale file from an earlier run of a driver that records none goes.
    let gaps_path = actual_dir(&dir).join(GAPS_FILE);
    match out.gaps() {
        Some(g) => io::write_bytes(&gaps_path, g.to_file_text().as_bytes())?,
        None => io::remove_file_if_exists(&gaps_path)?,
    }
    driven?;

    compare(&expected, out.lines()).map_err(|mismatch| GoldenError::Mismatch {
        mismatch,
        expected: expected_path,
        actual: actual_path,
    })
}

/// The name of the per-fixture gaps file next to `actual.jsonl`
/// ([`crate::gaps`]).
pub const GAPS_FILE: &str = "gaps.json";

/// `<target>/golden/<area>/<name>`: where [`run`] writes `actual.jsonl` and
/// [`GAPS_FILE`] for the fixture directory `fixture`.
pub fn actual_dir_of(fixture: impl AsRef<Path>) -> PathBuf {
    actual_dir(&resolve(fixture.as_ref()))
}

/// `<target>/golden/<area>/<name>`.
fn actual_dir(fixture: &Path) -> PathBuf {
    let mut tail: Vec<_> = fixture.components().rev().take(2).collect();
    tail.reverse();
    let mut out = target_dir().join("golden");
    out.extend(tail);
    out
}
