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
//!   inside a directory, without the temporaries and journals; a missing
//!   directory lists empty.
//! - `delete` removes a file (a missing one is not an error) or an empty
//!   directory, then removes parent directories that became empty, up to
//!   the root: [`Storage`] has no empty directories, so `delete_tree`
//!   (SaveFormatOld.java:92-109) leaves nothing behind.
//!
//! # Write-behind and incremental region writes (WP-4.3)
//!
//! `mc-sim`'s `RegionFile` holds a whole `.mcr` file in memory and hands
//! all of it to [`Storage::write`] after every chunk it stores (its module
//! docs, "Storage model"): an autosave of 24 chunks (World.java:1772-1775,
//! ChunkProvider.java:139-145) wrote the same 1-4 MB file up to 24 times,
//! each synced, on the game thread, plus `level.dat`'s write-and-rename
//! dance (SaveHandler.java:111-131). Vanilla's `RandomAccessFile` writes the
//! chunk's sectors and two header ints in place instead
//! (RegionFile.java:165-245). The game's storage
//! ([`FsStorage::with_write_behind`]) therefore:
//!
//! - **queues** `write`, `rename` and `delete` in call order and returns;
//!   a background thread (`mc-storage-writer`) executes them in that
//!   order, so the game thread never waits for the disk. A write of a file
//!   already queued, with nothing touching that file in between, replaces
//!   the queued bytes: an autosave reaches the disk once per region file;
//! - **answers reads from the queue**: `read` returns what the queued
//!   operations make of the path (written bytes, or absent after a queued
//!   `delete` or `rename`), else the disk; `rename`'s `NotFound` is decided
//!   on that state too. `list` waits for the queue first ([`flush_writes`]);
//! - writes a region file ([`is_region_file`]) by **changed 4096-byte
//!   sectors** when they are under half of the file, with the same
//!   all-or-nothing promise as an atomic `write`: the changed sectors first
//!   go to a journal next to the file (`.mc_journal-<name>`, written
//!   atomically as above), are then written into the file in place and
//!   synced, and the journal is deleted. A crash before the journal's
//!   rename leaves the old file; after it, the next read of the file
//!   replays the journal (replaying is idempotent). Otherwise the whole
//!   file is written atomically.
//!
//! A storage made with [`FsStorage::new`] (the import, tests) waits for the
//! queue before each operation and does it at once. The game calls
//! [`flush_writes`] before it exits; a crash loses at most the queued
//! operations, each file staying either old or new. The bytes that reach
//! the disk are the ones the synchronous path writes; only the moment and
//! the number of writes change.
//!
//! **Errors.** A failed queued operation does not stop the next one, as a
//! failed `RegionFile.write` or `saveWorldInfoAndPlayer` only prints its
//! stack trace (RegionFile.java:233-234, SaveHandler.java:130-131): the
//! writer logs it and keeps the first error per storage root, which
//! [`flush_writes`] (the game's exit) and [`FsStorage::flush_writes`]
//! return. No other call returns it: every `write`, `rename`, `delete`,
//! `read` and `list` is carried out whatever failed before it, and a
//! synchronous storage at the same root (the import) waits for the queue
//! without taking the game's error, and so does the world export before it
//! reads the world folder ([`wait_writes`]).
//!
//! **Threads.** The writer is a `std::thread`. If it cannot be started,
//! the queue falls back to executing every operation at once on the
//! calling thread (logged once). `with_write_behind` is for native builds:
//! Phase 5's wasm32 client has no threads and must not use it.
//!
//! This is the one place the client touches files for the simulation;
//! the `#[allow(clippy::disallowed_*)]` attributes mark it (clippy.toml).

#![allow(clippy::disallowed_types, clippy::disallowed_methods)]

use std::fs;
use std::io::{Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock};

use mc_sim::save::{Clock, Storage, StorageError};

/// Prefix of the temporary files of an atomic write.
const TMP_PREFIX: &str = ".mc_tmp-";

/// Prefix of a region file's journal (module docs).
const JOURNAL_PREFIX: &str = ".mc_journal-";

/// The journal's magic number.
const JOURNAL_MAGIC: &[u8; 4] = b"MCJ1";

/// The unit of the incremental comparison: a region file's sector
/// (RegionFile.java:15, 4096 bytes).
const PAGE: usize = 4096;

/// Distinguishes concurrent temporaries of one process.
static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Process-wide counters (WP-4.3 measurements; `dump_debug`'s `storage`).
static WRITES: AtomicU64 = AtomicU64::new(0);
static WRITE_BYTES: AtomicU64 = AtomicU64::new(0);
static WRITE_NANOS: AtomicU64 = AtomicU64::new(0);
static REGION_QUEUED: AtomicU64 = AtomicU64::new(0);
static REGION_COALESCED: AtomicU64 = AtomicU64::new(0);
static REGION_FULL: AtomicU64 = AtomicU64::new(0);
static REGION_JOURNALED: AtomicU64 = AtomicU64::new(0);
static REGION_UNCHANGED: AtomicU64 = AtomicU64::new(0);
static REGION_DISK_BYTES: AtomicU64 = AtomicU64::new(0);
static WRITER_NANOS: AtomicU64 = AtomicU64::new(0);
static WRITER_FILES: AtomicU64 = AtomicU64::new(0);
static WRITER_FILE_BYTES: AtomicU64 = AtomicU64::new(0);

/// The storage counters since the process started.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StorageCounters {
    /// Files written synchronously on the calling thread (every `write` of
    /// a storage without write-behind).
    pub writes: u64,
    /// Their bytes.
    pub write_bytes: u64,
    /// Their time (creating, writing, syncing and renaming the temporary),
    /// nanoseconds, on the calling thread.
    pub write_nanos: u64,
    /// Region-file writes handed to the write-behind queue.
    pub region_queued: u64,
    /// Of those, how many replaced bytes still queued.
    pub region_coalesced: u64,
    /// Region files the writer thread wrote whole (atomically).
    pub region_full: u64,
    /// Region files it updated through a journal (changed sectors only).
    pub region_journaled: u64,
    /// Region files it found unchanged on disk.
    pub region_unchanged: u64,
    /// Bytes the writer thread wrote into region files (journals, sectors,
    /// whole files).
    pub region_disk_bytes: u64,
    /// Other files the writer thread wrote (atomically).
    pub writer_files: u64,
    /// Their bytes.
    pub writer_file_bytes: u64,
    /// The writer thread's time, nanoseconds.
    pub writer_nanos: u64,
}

impl StorageCounters {
    /// The counters now.
    pub fn now() -> Self {
        let l = |a: &AtomicU64| a.load(Ordering::Relaxed);
        StorageCounters {
            writes: l(&WRITES),
            write_bytes: l(&WRITE_BYTES),
            write_nanos: l(&WRITE_NANOS),
            region_queued: l(&REGION_QUEUED),
            region_coalesced: l(&REGION_COALESCED),
            region_full: l(&REGION_FULL),
            region_journaled: l(&REGION_JOURNALED),
            region_unchanged: l(&REGION_UNCHANGED),
            region_disk_bytes: l(&REGION_DISK_BYTES),
            writer_files: l(&WRITER_FILES),
            writer_file_bytes: l(&WRITER_FILE_BYTES),
            writer_nanos: l(&WRITER_NANOS),
        }
    }

    /// As JSON.
    pub fn to_json(self) -> serde_json::Value {
        serde_json::json!({
            "writes": self.writes,
            "write_bytes": self.write_bytes,
            "write_ms": self.write_nanos as f64 / 1e6,
            "region_queued": self.region_queued,
            "region_coalesced": self.region_coalesced,
            "region_full": self.region_full,
            "region_journaled": self.region_journaled,
            "region_unchanged": self.region_unchanged,
            "region_disk_bytes": self.region_disk_bytes,
            "writer_files": self.writer_files,
            "writer_file_bytes": self.writer_file_bytes,
            "writer_ms": self.writer_nanos as f64 / 1e6,
        })
    }
}

/// Whether `path` is a McRegion file (`r.<x>.<z>.mcr`), the files written
/// behind (module docs).
pub fn is_region_file(path: &str) -> bool {
    path.ends_with(".mcr")
}

// ---- the write-behind queue ------------------------------------------------

/// A mutating storage operation waiting for the writer thread.
#[derive(Clone, Debug)]
enum Op {
    /// `write`: the bytes, shared with readers while queued (with the
    /// storage root, which files an error).
    Write {
        root: PathBuf,
        path: PathBuf,
        bytes: Arc<Vec<u8>>,
    },
    /// `rename` (with the storage root, for pruning emptied directories).
    Rename {
        root: PathBuf,
        from: PathBuf,
        to: PathBuf,
    },
    /// `delete` (with the storage root).
    Delete { root: PathBuf, path: PathBuf },
}

impl Op {
    fn root(&self) -> &Path {
        match self {
            Op::Write { root, .. } | Op::Rename { root, .. } | Op::Delete { root, .. } => root,
        }
    }

    fn touches(&self, p: &Path) -> bool {
        match self {
            Op::Write { path, .. } | Op::Delete { path, .. } => path == p,
            Op::Rename { from, to, .. } => from == p || to == p,
        }
    }
}

/// The queue's state.
#[derive(Default)]
struct QueueState {
    /// The operations in call order; the front one is being executed while
    /// `in_flight`.
    ops: std::collections::VecDeque<Op>,
    /// The writer thread is executing `ops[0]`.
    in_flight: bool,
    /// The writer thread exists.
    started: bool,
    /// The writer thread could not be started: operations run at once on
    /// the calling thread (module docs, "Threads").
    sync_fallback: bool,
    /// The first error the writer met since the last report, by storage
    /// root.
    errors: std::collections::BTreeMap<PathBuf, String>,
}

/// The process-wide queue (module docs).
struct WriteQueue {
    state: Mutex<QueueState>,
    changed: Condvar,
}

fn queue() -> &'static WriteQueue {
    static Q: OnceLock<WriteQueue> = OnceLock::new();
    Q.get_or_init(|| WriteQueue {
        state: Mutex::new(QueueState::default()),
        changed: Condvar::new(),
    })
}

fn lock(q: &WriteQueue) -> MutexGuard<'_, QueueState> {
    q.state.lock().unwrap_or_else(|e| e.into_inner())
}

/// What the queued operations make of `p`: `None` when none touches it
/// (the disk is current for it), `Some(None)` when it ends deleted or
/// renamed away, `Some(Some(bytes))` when it ends written. `Err(())` when
/// the answer is a file a queued rename has not moved yet (the caller
/// waits for the queue and reads the disk).
type Resolved = Result<Option<Option<Arc<Vec<u8>>>>, ()>;

fn resolve(ops: &std::collections::VecDeque<Op>, p: &Path, upto: usize) -> Resolved {
    for i in (0..upto).rev() {
        match &ops[i] {
            Op::Write { path, bytes, .. } if path == p => return Ok(Some(Some(bytes.clone()))),
            Op::Delete { path, .. } if path == p => return Ok(Some(None)),
            Op::Rename { from, .. } if from == p => return Ok(Some(None)),
            Op::Rename { from, to, .. } if to == p => {
                return match resolve(ops, from, i)? {
                    Some(v) => Ok(Some(v)),
                    None => Err(()),
                };
            }
            _ => {}
        }
    }
    Ok(None)
}

/// Queues an operation (coalescing a write with an earlier queued write of
/// the same file that nothing in between touches) and starts the writer
/// thread if needed. An earlier failure of the writer does not stop it
/// (module docs, "Errors"); a failure to start the writer runs the
/// operation at once instead ("Threads").
fn enqueue(op: Op) -> Result<(), StorageError> {
    let q = queue();
    let mut st = lock(q);
    let spawn = || {
        std::thread::Builder::new()
            .name("mc-storage-writer".into())
            .spawn(writer_loop)
            .map(drop)
    };
    let run_now = queue_op(&mut st, op, spawn);
    q.changed.notify_all();
    match run_now {
        // Under the lock, so the fallback keeps the call order.
        Some(op) => execute(&op),
        None => Ok(()),
    }
}

/// [`enqueue`] on a locked state, with the writer's start as `spawn`:
/// `Some(op)` when the caller must execute `op` itself (no writer).
fn queue_op(
    st: &mut QueueState,
    op: Op,
    spawn: impl FnOnce() -> std::io::Result<()>,
) -> Option<Op> {
    if st.sync_fallback {
        return Some(op);
    }
    let mut coalesced = false;
    if let Op::Write { path, bytes, .. } = &op {
        if is_region_file(&path.to_string_lossy()) {
            REGION_QUEUED.fetch_add(1, Ordering::Relaxed);
        }
        let first = usize::from(st.in_flight);
        let n = st.ops.len();
        for i in (first..n).rev() {
            if !st.ops[i].touches(path) {
                continue;
            }
            if let Op::Write {
                path: qp,
                bytes: qb,
                ..
            } = &mut st.ops[i]
                && qp == path
            {
                *qb = bytes.clone();
                REGION_COALESCED.fetch_add(1, Ordering::Relaxed);
                coalesced = true;
            }
            break;
        }
    }
    if st.started {
        if !coalesced {
            st.ops.push_back(op);
        }
        return None;
    }
    // Not started: the queue is empty (nothing was coalesced).
    match spawn() {
        Ok(()) => {
            st.started = true;
            st.ops.push_back(op);
            None
        }
        Err(e) => {
            log::warn!(
                "storage: cannot start the storage writer ({e}); writing on the game thread"
            );
            st.sync_fallback = true;
            Some(op)
        }
    }
}

/// Waits until every queued operation is done; returns (and clears) the
/// first error the writer met, of any storage. The game calls it before
/// exiting.
pub fn flush_writes() -> Result<(), StorageError> {
    let mut st = wait_idle();
    match st.errors.pop_first() {
        Some((_, e)) => Err(StorageError::Backend(e)),
        None => Ok(()),
    }
}

/// Waits until every queued operation is done, leaving the writer's errors
/// for [`flush_writes`] (module docs, "Errors"). The world export calls it
/// before reading the world folder with `std::fs` (`export::export_from_game`).
pub fn wait_writes() {
    drop(wait_idle());
}

/// Waits until every queued operation is done; returns (and clears) the
/// first error the writer met on the storage at `root`.
fn flush_root(root: &Path) -> Result<(), StorageError> {
    let mut st = wait_idle();
    match st.errors.remove(root) {
        Some(e) => Err(StorageError::Backend(e)),
        None => Ok(()),
    }
}

/// Waits until the queue is empty, leaving the writer's errors where they
/// are (module docs, "Errors"); returns the locked state.
fn wait_idle() -> MutexGuard<'static, QueueState> {
    let q = queue();
    let mut st = lock(q);
    while !st.ops.is_empty() {
        st = q.changed.wait(st).unwrap_or_else(|e| e.into_inner());
    }
    st
}

/// The writer thread: executes the queued operations in order.
fn writer_loop() {
    let q = queue();
    loop {
        let op = {
            let mut st = lock(q);
            while st.ops.is_empty() {
                st = q.changed.wait(st).unwrap_or_else(|e| e.into_inner());
            }
            st.in_flight = true;
            st.ops[0].clone()
        };
        let t0 = crate::clock::nano_time();
        let r = execute(&op);
        let dt = (crate::clock::nano_time() - t0).max(0) as u64;
        WRITER_NANOS.fetch_add(dt, Ordering::Relaxed);
        let mut st = lock(q);
        st.ops.pop_front();
        st.in_flight = false;
        if let Err(e) = r {
            log::error!("storage: {e}");
            st.errors
                .entry(op.root().to_path_buf())
                .or_insert_with(|| e.to_string());
        }
        q.changed.notify_all();
    }
}

/// One operation on the disk (the synchronous paths).
fn execute(op: &Op) -> Result<(), StorageError> {
    match op {
        Op::Write { path, bytes, .. } => {
            if is_region_file(&path.to_string_lossy()) {
                write_region(path, bytes)
            } else {
                let r = atomic_write(path, bytes).map_err(|e| backend(path, e));
                WRITER_FILES.fetch_add(1, Ordering::Relaxed);
                WRITER_FILE_BYTES.fetch_add(bytes.len() as u64, Ordering::Relaxed);
                r
            }
        }
        Op::Rename { root, from, to } => rename_now(root, from, to),
        Op::Delete { root, path } => delete_now(root, path),
    }
}

/// `rename` on the disk.
fn rename_now(root: &Path, f: &Path, t: &Path) -> Result<(), StorageError> {
    if is_region_file(&f.to_string_lossy()) {
        replay_journal(f)?;
    }
    if is_region_file(&t.to_string_lossy()) {
        replay_journal(t)?;
    }
    if !f.exists() {
        return Err(StorageError::NotFound(f.display().to_string()));
    }
    if let Some(d) = t.parent() {
        fs::create_dir_all(d).map_err(|e| backend(d, e))?;
    }
    fs::rename(f, t).map_err(|e| backend(f, e))?;
    if let Some(d) = f.parent() {
        prune_empty(root, d.to_path_buf());
    }
    Ok(())
}

/// `delete` on the disk.
fn delete_now(root: &Path, p: &Path) -> Result<(), StorageError> {
    if is_region_file(&p.to_string_lossy()) {
        let jp = journal_path(p);
        match fs::remove_file(&jp) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                return Err(backend(&jp, e));
            }
            _ => {}
        }
    }
    match fs::metadata(p) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(backend(p, e)),
        Ok(m) if m.is_dir() => {
            // Only an empty directory goes (a blob store has no others).
            if fs::read_dir(p).map_err(|e| backend(p, e))?.next().is_none() {
                fs::remove_dir(p).map_err(|e| backend(p, e))?;
            }
        }
        Ok(_) => fs::remove_file(p).map_err(|e| backend(p, e))?,
    }
    if let Some(d) = p.parent() {
        prune_empty(root, d.to_path_buf());
    }
    Ok(())
}

/// Removes `dir` and its ancestors below `root` while they are empty.
fn prune_empty(root: &Path, mut dir: PathBuf) {
    while dir != root && dir.starts_with(root) {
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

/// Reads a file from the disk (`None` if missing or a directory), first
/// finishing an interrupted journal of a region file.
fn read_now(p: &Path) -> Result<Option<Vec<u8>>, StorageError> {
    if is_region_file(&p.to_string_lossy()) {
        replay_journal(p)?;
    }
    match fs::metadata(p) {
        Ok(m) if m.is_dir() => return Ok(None),
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(backend(p, e)),
    }
    match fs::read(p) {
        Ok(b) => Ok(Some(b)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(backend(p, e)),
    }
}

// ---- writing a region file --------------------------------------------------

/// Writes `tmp` with `bytes`, syncs it and renames it over `target`
/// (module docs).
fn atomic_write(target: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let dir = target.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(dir)?;
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
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
        fs::rename(&tmp, target)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// The journal of region file `p`.
fn journal_path(p: &Path) -> PathBuf {
    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    p.with_file_name(format!("{JOURNAL_PREFIX}{name}"))
}

/// The byte ranges (sector-aligned, merged) where `new` differs from `old`,
/// including bytes past either's end.
fn changed_ranges(old: &[u8], new: &[u8]) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    let n = new.len().div_ceil(PAGE);
    for i in 0..n {
        let a = i * PAGE;
        let b = (a + PAGE).min(new.len());
        let same = old.get(a..b).is_some_and(|o| o == &new[a..b]);
        if !same {
            match out.last_mut() {
                Some((start, len)) if *start + *len == a => *len += b - a,
                _ => out.push((a, b - a)),
            }
        }
    }
    out
}

/// The journal's bytes: magic, the file's new length, the ranges.
fn encode_journal(new: &[u8], ranges: &[(usize, usize)]) -> Vec<u8> {
    let body: usize = ranges.iter().map(|r| 12 + r.1).sum();
    let mut j = Vec::with_capacity(16 + body);
    j.extend_from_slice(JOURNAL_MAGIC);
    j.extend_from_slice(&(new.len() as u64).to_be_bytes());
    j.extend_from_slice(&(ranges.len() as u32).to_be_bytes());
    for &(a, len) in ranges {
        j.extend_from_slice(&(a as u64).to_be_bytes());
        j.extend_from_slice(&(len as u32).to_be_bytes());
        j.extend_from_slice(&new[a..a + len]);
    }
    j
}

/// A decoded journal: the new length and the ranges with their bytes.
type Journal<'a> = (u64, Vec<(u64, &'a [u8])>);

fn decode_journal(j: &[u8]) -> Option<Journal<'_>> {
    let rest = j.strip_prefix(JOURNAL_MAGIC)?;
    let len = u64::from_be_bytes(rest.get(..8)?.try_into().ok()?);
    let n = u32::from_be_bytes(rest.get(8..12)?.try_into().ok()?);
    let mut at = 12;
    let mut ranges = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let off = u64::from_be_bytes(rest.get(at..at + 8)?.try_into().ok()?);
        let l = u32::from_be_bytes(rest.get(at + 8..at + 12)?.try_into().ok()?) as usize;
        let bytes = rest.get(at + 12..at + 12 + l)?;
        ranges.push((off, bytes));
        at += 12 + l;
    }
    (at == rest.len()).then_some((len, ranges))
}

/// Applies a journal to its file (in place, then synced).
fn apply_journal(p: &Path, journal: &Journal<'_>) -> std::io::Result<()> {
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(p)?;
    for &(off, bytes) in &journal.1 {
        f.seek(SeekFrom::Start(off))?;
        f.write_all(bytes)?;
    }
    f.set_len(journal.0)?;
    f.sync_data()
}

/// Finishes an interrupted journaled write of `p`, if there is one
/// (module docs). A journal that does not decode was never renamed into
/// place whole, which cannot happen; it is dropped with a warning.
fn replay_journal(p: &Path) -> Result<(), StorageError> {
    let jp = journal_path(p);
    let j = match fs::read(&jp) {
        Ok(j) => j,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(backend(&jp, e)),
    };
    match decode_journal(&j) {
        Some(journal) => {
            apply_journal(p, &journal).map_err(|e| backend(p, e))?;
            log::warn!("storage: replayed the journal of {}", p.display());
        }
        None => log::warn!("storage: dropping a damaged journal {}", jp.display()),
    }
    fs::remove_file(&jp).map_err(|e| backend(&jp, e))
}

/// Writes region file `p` with `new` (module docs): nothing if unchanged,
/// the changed sectors through a journal if they are under half the file,
/// else the whole file atomically.
fn write_region(p: &Path, new: &[u8]) -> Result<(), StorageError> {
    replay_journal(p)?;
    let old = match fs::read(p) {
        Ok(o) => Some(o),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(backend(p, e)),
    };
    let Some(old) = old else {
        atomic_write(p, new).map_err(|e| backend(p, e))?;
        REGION_FULL.fetch_add(1, Ordering::Relaxed);
        REGION_DISK_BYTES.fetch_add(new.len() as u64, Ordering::Relaxed);
        return Ok(());
    };
    let ranges = changed_ranges(&old, new);
    if ranges.is_empty() && old.len() == new.len() {
        REGION_UNCHANGED.fetch_add(1, Ordering::Relaxed);
        return Ok(());
    }
    let changed: usize = ranges.iter().map(|r| r.1).sum();
    if changed * 2 > new.len() {
        atomic_write(p, new).map_err(|e| backend(p, e))?;
        REGION_FULL.fetch_add(1, Ordering::Relaxed);
        REGION_DISK_BYTES.fetch_add(new.len() as u64, Ordering::Relaxed);
        return Ok(());
    }
    let jbytes = encode_journal(new, &ranges);
    let jp = journal_path(p);
    atomic_write(&jp, &jbytes).map_err(|e| backend(&jp, e))?;
    let journal = decode_journal(&jbytes).expect("just encoded");
    apply_journal(p, &journal).map_err(|e| backend(p, e))?;
    fs::remove_file(&jp).map_err(|e| backend(&jp, e))?;
    REGION_JOURNALED.fetch_add(1, Ordering::Relaxed);
    REGION_DISK_BYTES.fetch_add((jbytes.len() + changed) as u64, Ordering::Relaxed);
    Ok(())
}

// ---- the storage ------------------------------------------------------------

/// [`Storage`] over a directory tree (module docs).
#[derive(Clone, Debug)]
pub struct FsStorage {
    root: PathBuf,
    /// Mutations go through the write-behind queue (module docs).
    write_behind: bool,
}

impl FsStorage {
    /// A storage rooted at `root` (created when missing) whose operations
    /// reach the disk before they return.
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, StorageError> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(|e| backend(&root, e))?;
        Ok(FsStorage {
            root,
            write_behind: false,
        })
    }

    /// This storage, writing behind (module docs).
    pub fn with_write_behind(mut self) -> Self {
        self.write_behind = true;
        self
    }

    /// The root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Waits for the write-behind queue; returns (and clears) the first
    /// error the writer met on this storage's root since the last report
    /// (module docs, "Errors").
    pub fn flush_writes(&self) -> Result<(), StorageError> {
        flush_root(&self.root)
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

    /// What the queue makes of `p` (see [`resolve`]); waits for the queue
    /// when only the disk can tell.
    fn queued_state(&self, p: &Path) -> Result<Option<Option<Arc<Vec<u8>>>>, StorageError> {
        let r = {
            let st = lock(queue());
            let n = st.ops.len();
            resolve(&st.ops, p, n)
        };
        match r {
            Ok(v) => Ok(v),
            Err(()) => {
                drop(wait_idle());
                Ok(None)
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
        if self.write_behind {
            if let Some(v) = self.queued_state(&p)? {
                return Ok(v.map(|b| b.as_ref().clone()));
            }
        } else {
            drop(wait_idle());
        }
        read_now(&p)
    }

    fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), StorageError> {
        let p = self.fs_path(path)?;
        if self.write_behind {
            return enqueue(Op::Write {
                root: self.root.clone(),
                path: p,
                bytes: Arc::new(bytes.to_vec()),
            });
        }
        drop(wait_idle());
        let t0 = crate::clock::nano_time();
        let r = atomic_write(&p, bytes).map_err(|e| backend(&p, e));
        WRITES.fetch_add(1, Ordering::Relaxed);
        WRITE_BYTES.fetch_add(bytes.len() as u64, Ordering::Relaxed);
        WRITE_NANOS.fetch_add(
            (crate::clock::nano_time() - t0).max(0) as u64,
            Ordering::Relaxed,
        );
        r
    }

    fn list(&mut self, dir: &str) -> Result<Vec<String>, StorageError> {
        drop(wait_idle());
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
            if !name.starts_with(TMP_PREFIX) && !name.starts_with(JOURNAL_PREFIX) {
                names.push(name);
            }
        }
        names.sort();
        Ok(names)
    }

    fn rename(&mut self, from: &str, to: &str) -> Result<(), StorageError> {
        let f = self.fs_path(from)?;
        let t = self.fs_path(to)?;
        if self.write_behind {
            // `NotFound` is decided now, on the state the queue leads to.
            let exists = match self.queued_state(&f)? {
                Some(v) => v.is_some(),
                None => f.exists(),
            };
            if !exists {
                return Err(StorageError::NotFound(from.to_string()));
            }
            return enqueue(Op::Rename {
                root: self.root.clone(),
                from: f,
                to: t,
            });
        }
        drop(wait_idle());
        rename_now(&self.root, &f, &t).map_err(|e| match e {
            StorageError::NotFound(_) => StorageError::NotFound(from.to_string()),
            e => e,
        })
    }

    fn delete(&mut self, path: &str) -> Result<(), StorageError> {
        let p = self.fs_path(path)?;
        if self.write_behind {
            return enqueue(Op::Delete {
                root: self.root.clone(),
                path: p,
            });
        }
        drop(wait_idle());
        delete_now(&self.root, &p)
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
            // A write-behind storage may still have operations queued here
            // (waiting leaves other tests' errors alone).
            drop(wait_idle());
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

    fn region_bytes(len: usize, seed: u8) -> Vec<u8> {
        (0..len)
            .map(|i| (i as u8).wrapping_mul(31).wrapping_add(seed))
            .collect()
    }

    #[test]
    fn changed_ranges_are_sector_aligned_and_merged() {
        let old = region_bytes(5 * PAGE, 1);
        assert!(changed_ranges(&old, &old).is_empty());
        let mut new = old.clone();
        new[10] ^= 1;
        new[PAGE + 5] ^= 1;
        new[3 * PAGE + 7] ^= 1;
        assert_eq!(
            changed_ranges(&old, &new),
            vec![(0, 2 * PAGE), (3 * PAGE, PAGE)]
        );
        // Growth: the new sectors are changed; a short last sector too.
        let mut grown = old.clone();
        grown.extend_from_slice(&[9; PAGE + 100]);
        assert_eq!(changed_ranges(&old, &grown), vec![(5 * PAGE, PAGE + 100)]);
    }

    #[test]
    fn journal_round_trip() {
        let new = region_bytes(3 * PAGE, 7);
        let ranges = vec![(0, PAGE), (2 * PAGE, PAGE)];
        let j = encode_journal(&new, &ranges);
        let (len, r) = decode_journal(&j).unwrap();
        assert_eq!(len, new.len() as u64);
        assert_eq!(r.len(), 2);
        assert_eq!(r[1].0, 2 * PAGE as u64);
        assert_eq!(r[1].1, &new[2 * PAGE..]);
        assert!(decode_journal(&j[..j.len() - 1]).is_none());
        assert!(decode_journal(b"XXXX").is_none());
    }

    #[test]
    fn region_files_are_written_behind_and_incrementally() {
        let d = TempDir::new("region_wb");
        let mut s = FsStorage::new(&d.0).unwrap().with_write_behind();
        let path = "w/region/r.0.0.mcr";
        let disk = d.0.join("w/region/r.0.0.mcr");
        // A new file: whole; read back from the queue or the disk alike.
        let a = region_bytes(64 * PAGE, 3);
        s.write(path, &a).unwrap();
        assert_eq!(s.read(path).unwrap().as_deref(), Some(&a[..]));
        s.flush_writes().unwrap();
        assert_eq!(fs::read(&disk).unwrap(), a);
        // Two writes in a row: the second replaces the first in the queue;
        // a few changed sectors go through the journal.
        let mut b = a.clone();
        b[5] = 0xEE;
        let mut c = b.clone();
        c[40 * PAGE + 1] = 0xDD;
        c.extend_from_slice(&[1; PAGE]);
        s.write(path, &b).unwrap();
        s.write(path, &c).unwrap();
        assert_eq!(s.read(path).unwrap().as_deref(), Some(&c[..]));
        s.flush_writes().unwrap();
        assert_eq!(fs::read(&disk).unwrap(), c);
        assert!(!journal_path(&disk).exists());
        // list waits for the queue and hides journals and temporaries.
        s.write(path, &a).unwrap();
        fs::write(d.0.join("w/region/.mc_journal-x.mcr"), b"MCJ1").unwrap();
        assert_eq!(s.list("w/region").unwrap(), vec!["r.0.0.mcr"]);
        assert_eq!(fs::read(&disk).unwrap(), a);
        // delete is queued too and removes a journal as well.
        s.write(path, &c).unwrap();
        s.delete(path).unwrap();
        assert_eq!(s.read(path).unwrap(), None);
        s.flush_writes().unwrap();
        assert!(!disk.exists());
    }

    #[test]
    fn the_level_dat_dance_reads_the_queued_state() {
        // SaveHandler.write_level_dat's sequence on a write-behind storage
        // gives the files the synchronous storage gives.
        fn dance(s: &mut FsStorage, bytes: &[u8]) {
            s.write("w/level.dat_new", bytes).unwrap();
            if s.read("w/level.dat_old").unwrap().is_some() {
                s.delete("w/level.dat_old").unwrap();
            }
            let _ = s.rename("w/level.dat", "w/level.dat_old");
            if s.read("w/level.dat").unwrap().is_some() {
                s.delete("w/level.dat").unwrap();
            }
            s.rename("w/level.dat_new", "w/level.dat").unwrap();
            assert_eq!(s.read("w/level.dat_new").unwrap(), None);
        }
        let a = TempDir::new("dance_sync");
        let b = TempDir::new("dance_wb");
        let mut sync = FsStorage::new(&a.0).unwrap();
        let mut wb = FsStorage::new(&b.0).unwrap().with_write_behind();
        for (i, bytes) in [&b"one"[..], b"two", b"three"].into_iter().enumerate() {
            dance(&mut sync, bytes);
            dance(&mut wb, bytes);
            assert_eq!(wb.read("w/level.dat").unwrap().as_deref(), Some(bytes));
            if i > 0 {
                assert!(wb.read("w/level.dat_old").unwrap().is_some());
            }
        }
        // A rename of a file that is neither queued nor on disk fails now.
        assert!(matches!(
            wb.rename("w/nope", "w/x"),
            Err(StorageError::NotFound(_))
        ));
        wb.flush_writes().unwrap();
        for name in ["level.dat", "level.dat_old", "level.dat_new"] {
            assert_eq!(
                fs::read(a.0.join("w").join(name)).ok(),
                fs::read(b.0.join("w").join(name)).ok(),
                "{name}"
            );
        }
        assert_eq!(wb.list("w").unwrap(), sync.list("w").unwrap());
    }

    #[test]
    fn an_interrupted_journal_is_replayed_on_read() {
        let d = TempDir::new("region_journal");
        let mut s = FsStorage::new(&d.0).unwrap();
        let path = "w/region/r.1.0.mcr";
        let disk = d.0.join("w/region/r.1.0.mcr");
        let old = region_bytes(8 * PAGE, 5);
        s.write(path, &old).unwrap();
        let mut new = old.clone();
        new[3 * PAGE] = 0;
        new.extend_from_slice(&[4; 10]);
        // The crash: the journal was renamed into place, the file was not
        // touched yet.
        let ranges = changed_ranges(&old, &new);
        atomic_write(&journal_path(&disk), &encode_journal(&new, &ranges)).unwrap();
        assert_eq!(s.read(path).unwrap().as_deref(), Some(&new[..]));
        assert!(!journal_path(&disk).exists());
        assert_eq!(fs::read(&disk).unwrap(), new);
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
        for write_behind in [false, true] {
            use mc_sim::save::{SaveHandler, SaveOldDir, SaveStore};
            use mc_sim::world::info::WorldInfo;
            let d = TempDir::new("handler");
            let mut fs = FsStorage::new(&d.0).unwrap();
            if write_behind {
                fs = fs.with_write_behind();
            }
            let store = SaveStore::shared(Box::new(fs), Box::new(mc_sim::save::FixedClock(1234)));
            let mut h = SaveOldDir::new(store, "saves", "W").unwrap();
            h.check_session_lock().unwrap();
            let mut info = WorldInfo::new(42, "W");
            h.save_world_info_and_player(&mut info, None).unwrap();
            // Twice: the second goes through the level.dat_old rename dance.
            h.save_world_info_and_player(&mut info, None).unwrap();
            let back = h.load_world_info().unwrap();
            assert_eq!(back.random_seed, 42);
            flush_root(&d.0).unwrap();
            assert!(d.0.join("saves/W/level.dat_old").exists());
            assert!(d.0.join("saves/W/level.dat").exists());
            assert!(!d.0.join("saves/W/level.dat_new").exists());
            assert!(d.0.join("saves/W/session.lock").exists());
        }
    }

    #[test]
    fn a_failed_queued_write_does_not_stop_the_next() {
        // RegionFile.java:233-234: a failed write prints its stack trace;
        // the next write is carried out.
        let d = TempDir::new("wb_error");
        let mut s = FsStorage::new(&d.0).unwrap().with_write_behind();
        // "w/blocker" is a file, so "w/blocker/region/r.0.0.mcr" cannot be
        // created (its parent is not a directory).
        s.write("w/blocker", b"file").unwrap();
        s.write("w/blocker/region/r.0.0.mcr", b"lost").unwrap();
        drop(wait_idle());
        // The writer's error does not fail or drop the next operations.
        let a = region_bytes(4 * PAGE, 9);
        s.write("w/region/r.1.0.mcr", &a).unwrap();
        s.write("w/level.dat", b"next").unwrap();
        s.rename("w/level.dat", "w/level.dat_old").unwrap();
        // A synchronous storage at the same root works and leaves the
        // error to the write-behind one.
        let mut sync = FsStorage::new(&d.0).unwrap();
        sync.write("w/other", b"sync").unwrap();
        assert_eq!(sync.read("w/other").unwrap().as_deref(), Some(&b"sync"[..]));
        assert_eq!(fs::read(d.0.join("w/region/r.1.0.mcr")).unwrap(), a);
        assert_eq!(fs::read(d.0.join("w/level.dat_old")).unwrap(), b"next");
        assert!(!d.0.join("w/level.dat").exists());
        // The error is reported once, by the explicit flush.
        assert!(matches!(s.flush_writes(), Err(StorageError::Backend(_))));
        s.flush_writes().unwrap();
    }

    #[test]
    fn a_writer_that_cannot_start_runs_operations_at_once() {
        let op = |n: &str| Op::Write {
            root: PathBuf::from("/r"),
            path: PathBuf::from(format!("/r/{n}")),
            bytes: Arc::new(vec![1]),
        };
        // The spawn fails: the operation comes back to run at once, the
        // queue stays empty and not started, and no later spawn is tried.
        let mut st = QueueState::default();
        let back = queue_op(&mut st, op("a"), || {
            Err(std::io::Error::other("no threads"))
        });
        assert!(matches!(back, Some(Op::Write { ref path, .. }) if path == Path::new("/r/a")));
        assert!(!st.started && st.sync_fallback && st.ops.is_empty());
        let back = queue_op(&mut st, op("b"), || panic!("no second spawn"));
        assert!(back.is_some() && st.ops.is_empty());
        // The spawn succeeds: queued, started.
        let mut st = QueueState::default();
        assert!(queue_op(&mut st, op("a"), || Ok(())).is_none());
        assert!(st.started && !st.sync_fallback && st.ops.len() == 1);
        assert!(queue_op(&mut st, op("b"), || panic!("already started")).is_none());
        assert_eq!(st.ops.len(), 2);
    }
}
