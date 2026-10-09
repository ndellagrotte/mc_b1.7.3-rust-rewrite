//! World export (PLAN_v4 D5, WP-4.5; project-owned, no vanilla
//! counterpart): a world of the saves directory is copied out to a folder
//! the user names, checked so that the real Beta 1.7.3 client can open it.
//!
//! The native save format already is vanilla McRegion (D5), so an export is
//! a verified copy, nothing is converted or stripped (`session.lock` is
//! copied too; vanilla rewrites it when it opens the world,
//! SaveHandler.java:41-51). [`export_world`] does, in order:
//!
//! 1. **Target.** `<target>/<world folder>` must not exist, or be an empty
//!    directory ([`ExportError::TargetNotEmpty`]): nothing is ever
//!    overwritten or merged. A target inside the world folder is refused.
//! 2. **Read.** Every file under `saves/<world>` is read into memory
//!    (the FsStorage write temporaries `.mc_tmp-*` are skipped).
//! 3. **Validate** the in-memory copy ([`world_digest`]):
//!    - D5's import validation, [`mc_sim::save::import::validate`]: `level.dat`
//!      (or `level.dat_old`) decodes into a `WorldInfo`, and every block and
//!      item id in it and in every chunk of `region/` and `DIM-1/region/` is
//!      one Beta 1.7.3 defines ([`ExportError::Refused`]);
//!    - stricter than the import, because an export is meant to be complete:
//!      the save is McRegion (version 19132), `SaveOldDir.loadWorldInfo`
//!      reads it, every chunk the region headers list reads back (an entry
//!      past the end of a truncated file, which vanilla's reader turns into
//!      `null` and regenerates, is refused here), and every such chunk loads
//!      through the simulation's `McRegionChunkLoader` into a `Chunk`
//!      ([`ExportError::Corrupt`]).
//!
//!    The check also yields the world's digest ([`WorldDigest`]): per chunk,
//!    blake3 of the five arrays of the loaded `Chunk` (the Tier S
//!    `chunk_digest` with `"stage":"loaded"`, tests/golden/README.md) and of
//!    its stored NBT tree; the `level.dat` tree; every file's bytes.
//! 4. **Copy** through `std::fs` (here in the client, never in `mc-sim`, D9)
//!    into a hidden staging folder `<target>/.mc_export-<world>-<pid>`.
//! 5. **Re-read** the staged copy from disk and check it against the source:
//!    the same files with the same bytes, and the digest of the copy (the
//!    same loaders, step 3) equal to the source's, chunk by chunk
//!    ([`ExportError::Mismatch`]).
//! 6. **Publish**: the staging folder is renamed to `<target>/<world>`. On
//!    any failure after step 4 the staging folder is removed, so a refused or
//!    failed export leaves nothing behind.
//!
//! An open world is saved first (`World.saveWorld(true)`, by
//! [`export_from_game`]); from the world list the world is closed and its
//! files are complete. The screens are
//! [`crate::gui::menu::export_world`]; the dev action is `export_world`
//! (crates/mc-client/README.md).

#![allow(clippy::disallowed_types, clippy::disallowed_methods)]

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use mc_sim::save::chunk_loader::discard;
use mc_sim::save::import::{ImportRefusal, validate};
use mc_sim::save::save_handler::world_info_from_level_dat;
use mc_sim::save::{
    ChunkLoader, FixedClock, McRegionChunkLoader, MemoryStorage, RegionFile, SaveFormat,
    SaveHandler, SaveOldDir, SaveStore, SharedSaveStore, Storage, Tag, path_join,
};
use mc_sim::world::{World, WorldEnvironment};
use serde_json::{Value, json};

use crate::game::Game;
use crate::paths::SAVES_DIR;

/// The default target folder's name in the data directory
/// (`<data-dir>/exports`).
pub const EXPORTS_DIR: &str = "exports";

/// The storage directory a world is staged under for the checks.
const STAGE: &str = "world";

/// Prefix of the staging folder next to the target (step 4).
const STAGING_PREFIX: &str = ".mc_export-";

/// Prefix of the native storage's write temporaries (crate::storage), never
/// part of a world.
const FS_TMP_PREFIX: &str = ".mc_tmp-";

/// McRegion's save version (`SaveOldDir.saveWorldInfoAndPlayer` sets it,
/// SaveOldDir.java:22-25; `SaveConverterMcRegion.isOldMapFormat` compares
/// with it, SaveConverterMcRegion.java:57-60).
pub const MCREGION_VERSION: i32 = 19132;

/// `<data_dir>/exports`, the export screen's default folder.
pub fn default_export_dir(data_dir: &Path) -> PathBuf {
    data_dir.join(EXPORTS_DIR)
}

/// Why an export failed (module docs, by step).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExportError {
    /// There is no world folder of that name in the saves directory.
    NoWorld(String),
    /// Saving the open world first failed.
    Save(String),
    /// The world folder could not be read.
    Read(String),
    /// The target exists and is not an empty directory (or is inside the
    /// world).
    TargetNotEmpty(PathBuf),
    /// D5: an id Beta 1.7.3 does not define, or data the scan cannot read.
    Refused(ImportRefusal),
    /// A part of the save the simulation's loaders cannot load.
    Corrupt {
        /// The file, relative to the world folder.
        path: String,
        /// Why.
        reason: String,
    },
    /// Writing the copy failed.
    Write(String),
    /// The copy, read back, differs from the source.
    Mismatch(String),
}

/// A storage path under [`STAGE`] → the path inside the world folder.
fn rel(path: &str) -> &str {
    path.strip_prefix("world/").unwrap_or(path)
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportError::NoWorld(m) => write!(f, "no such world: {m}"),
            ExportError::Save(m) => write!(f, "cannot save the world first: {m}"),
            ExportError::Read(m) => write!(f, "cannot read the world: {m}"),
            ExportError::TargetNotEmpty(p) => {
                write!(f, "{} exists and is not empty", p.display())
            }
            ExportError::Refused(r) => write!(f, "the world cannot be exported: {r}"),
            ExportError::Corrupt { path, reason } => {
                write!(f, "the world cannot be exported: {}: {reason}", rel(path))
            }
            ExportError::Write(m) => write!(f, "cannot write the copy: {m}"),
            ExportError::Mismatch(m) => write!(f, "the copy does not match the world: {m}"),
        }
    }
}

impl std::error::Error for ExportError {}

/// One chunk's digest (module docs, step 3): blake3 (hex) of the loaded
/// `Chunk`'s arrays and of its stored NBT tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkDigest {
    /// 0 for `region/`, -1 for `DIM-1/region/`.
    pub dimension: i32,
    /// Chunk x.
    pub x: i32,
    /// Chunk z.
    pub z: i32,
    /// `Chunk.blocks`.
    pub blocks: String,
    /// `data.data`.
    pub data: String,
    /// `skylightMap.data`.
    pub skylight: String,
    /// `blocklightMap.data`.
    pub blocklight: String,
    /// `heightMap`.
    pub heightmap: String,
    /// The stored compound (`{"": {Level: …}}`), re-encoded canonically
    /// (compounds in key order), so entities and tile entities count too.
    pub nbt: String,
}

impl ChunkDigest {
    /// The first field that differs from `o`, if any.
    fn first_difference(&self, o: &ChunkDigest) -> Option<&'static str> {
        [
            ("blocks", &self.blocks, &o.blocks),
            ("data", &self.data, &o.data),
            ("skylight", &self.skylight, &o.skylight),
            ("blocklight", &self.blocklight, &o.blocklight),
            ("heightmap", &self.heightmap, &o.heightmap),
            ("nbt", &self.nbt, &o.nbt),
        ]
        .into_iter()
        .find(|(_, a, b)| a != b)
        .map(|(n, _, _)| n)
    }
}

/// A world's digest (module docs, step 3).
#[derive(Clone, Debug, PartialEq)]
pub struct WorldDigest {
    /// `LevelName`.
    pub level_name: String,
    /// `RandomSeed`.
    pub seed: i64,
    /// `Time`.
    pub time: i64,
    /// The player's `Pos`, if `level.dat` holds a player.
    pub player_pos: Option<[f64; 3]>,
    /// The player's `Dimension` (0 without a player).
    pub player_dimension: i32,
    /// blake3 of the `level.dat` tree `SaveOldDir.loadWorldInfo` reads.
    pub level_dat: String,
    /// Every file: path in the world folder, length, blake3 of the bytes.
    pub files: Vec<(String, u64, String)>,
    /// The region files checked (`region/…`, `DIM-1/region/…`).
    pub regions: Vec<String>,
    /// Every chunk, by dimension (0 then -1), region file name and index.
    pub chunks: Vec<ChunkDigest>,
}

impl WorldDigest {
    /// The digest as JSON (the `export_world` dev action's report).
    pub fn to_json(&self) -> Value {
        json!({
            "level_name": self.level_name,
            "seed": self.seed,
            "time": self.time,
            "player_pos": self.player_pos,
            "player_dimension": self.player_dimension,
            "level_dat": self.level_dat,
            "files": self.files.iter().map(|(p, n, h)| json!({"path": p, "len": n, "blake3": h})).collect::<Vec<_>>(),
            "regions": self.regions,
            "chunks": self.chunks.iter().map(|c| json!({
                "dimension": c.dimension, "x": c.x, "z": c.z,
                "blocks": c.blocks, "data": c.data, "skylight": c.skylight,
                "blocklight": c.blocklight, "heightmap": c.heightmap, "nbt": c.nbt,
            })).collect::<Vec<_>>(),
        })
    }
}

/// What a successful export did.
#[derive(Clone, Debug, PartialEq)]
pub struct ExportReport {
    /// The exported world folder (`<target>/<world>`).
    pub path: PathBuf,
    /// Files copied.
    pub files: usize,
    /// Bytes copied.
    pub bytes: u64,
    /// The checked digest (equal for the source and the copy).
    pub digest: WorldDigest,
}

impl ExportReport {
    /// The report as JSON.
    pub fn to_json(&self) -> Value {
        json!({
            "path": self.path.display().to_string(),
            "files": self.files,
            "bytes": self.bytes,
            "regions": self.digest.regions.len(),
            "chunks": self.digest.chunks.len(),
            "digest": self.digest.to_json(),
        })
    }
}

fn hex(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

/// The files under `root`, as `/`-separated paths relative to it, sorted
/// (links to files followed, a link to a directory refused; the storage
/// temporaries skipped).
pub fn read_tree(root: &Path) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut out = Vec::new();
    let mut stack = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, prefix)) = stack.pop() {
        let it = fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        for entry in it {
            let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with(FS_TMP_PREFIX) {
                continue;
            }
            let path = entry.path();
            let r = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{prefix}/{name}")
            };
            let meta = fs::metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let link = fs::symlink_metadata(&path)
                .map_err(|e| format!("{}: {e}", path.display()))?
                .file_type()
                .is_symlink();
            if meta.is_dir() && link {
                // Never followed: a link to a parent would never end.
                return Err(format!("{}: a link to a directory", path.display()));
            } else if meta.is_dir() {
                stack.push((path, r));
            } else if meta.is_file() {
                let bytes = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                out.push((r, bytes));
            } else {
                return Err(format!("{}: not a file or a directory", path.display()));
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

/// Writes `files` under `root` (created), each flushed to disk.
fn write_tree(root: &Path, files: &[(String, Vec<u8>)]) -> Result<(), String> {
    fs::create_dir_all(root).map_err(|e| format!("{}: {e}", root.display()))?;
    for (r, bytes) in files {
        let mut p = root.to_path_buf();
        for c in r.split('/') {
            p.push(c);
        }
        if let Some(d) = p.parent() {
            fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
        }
        let mut f = fs::File::create(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        f.write_all(bytes)
            .and_then(|()| f.sync_all())
            .map_err(|e| format!("{}: {e}", p.display()))?;
    }
    Ok(())
}

/// The `level.dat` tree `SaveHandler.loadWorldInfo` uses: `level.dat`, or
/// `level.dat_old` when that is missing or does not decode
/// (SaveHandler.java:81-107).
fn level_dat_tree(storage: &mut dyn Storage) -> Option<(String, Tag)> {
    for name in ["level.dat", "level.dat_old"] {
        let Ok(Some(bytes)) = storage.read(&path_join(STAGE, name)) else {
            continue;
        };
        if world_info_from_level_dat(&bytes).is_ok()
            && let Ok(t) = mc_sim::save::mc_nbt::read_gzip(&bytes)
        {
            return Some(t);
        }
    }
    None
}

fn corrupt(path: impl Into<String>, reason: impl std::fmt::Display) -> ExportError {
    ExportError::Corrupt {
        path: path.into(),
        reason: reason.to_string(),
    }
}

/// Validates a world given as its files (step 3 of the module docs) and
/// returns its digest.
pub fn world_digest(files: &[(String, Vec<u8>)]) -> Result<WorldDigest, ExportError> {
    let mut st = MemoryStorage::default();
    for (r, bytes) in files {
        st.write(&path_join(STAGE, r), bytes)
            .map_err(|e| ExportError::Read(format!("{r}: {e}")))?;
    }
    // D5: level.dat and every id (the import's scan).
    validate(&mut st, STAGE).map_err(|r| {
        ExportError::Refused(match r {
            // The path inside the world folder, not the staging prefix.
            ImportRefusal::Unreadable { path, reason } => ImportRefusal::Unreadable {
                path: rel(&path).to_string(),
                reason,
            },
            other => other,
        })
    })?;
    let (root_name, root) =
        level_dat_tree(&mut st).ok_or_else(|| corrupt("world/level.dat", "does not decode"))?;
    let level_dat = hex(&mc_sim::save::mc_nbt::write_named(&root_name, &root)
        .map_err(|e| corrupt("world/level.dat", format!("{e:?}")))?);

    let store: SharedSaveStore = SaveStore::shared(Box::new(st), Box::new(FixedClock(0)));
    // SaveOldDir writes session.lock: into the in-memory copy only.
    let mut handler =
        SaveOldDir::new(store.clone(), "", STAGE).map_err(|e| corrupt("world/session.lock", e))?;
    let info = handler
        .load_world_info()
        .ok_or_else(|| corrupt("world/level.dat", "SaveOldDir.loadWorldInfo returns null"))?;
    if info.save_version != MCREGION_VERSION {
        return Err(corrupt(
            "world/level.dat",
            format!(
                "save version {} is not McRegion ({MCREGION_VERSION}); pre-McRegion worlds are not supported (deferred)",
                info.save_version
            ),
        ));
    }
    let player_pos = match &info.player_tag {
        Some(Tag::Compound(p)) => match p.get("Pos") {
            Some(Tag::List { items, .. }) if items.len() == 3 => match items.as_slice() {
                [Tag::Double(x), Tag::Double(y), Tag::Double(z)] => Some([*x, *y, *z]),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    };
    let mut digest = WorldDigest {
        level_name: info.level_name.clone(),
        seed: info.random_seed,
        time: info.world_time,
        player_pos,
        player_dimension: info.dimension,
        level_dat,
        files: files
            .iter()
            .map(|(p, b)| (p.clone(), b.len() as u64, hex(b)))
            .collect(),
        regions: Vec::new(),
        chunks: Vec::new(),
    };

    // The loading constructor (World.java:149-208), as the Tier S
    // `world_read` opens a save; the chunks are loaded one by one and
    // dropped again.
    let mut world = World::new_from_save(
        Box::new(handler),
        &info.level_name,
        0,
        None,
        WorldEnvironment {
            rand_seed: 0,
            field_9437_g: 0,
            lock_timestamp: 0,
        },
    );
    for (dimension, sub, chunk_dir) in [
        (0, "region", STAGE.to_string()),
        (-1, "DIM-1/region", path_join(STAGE, "DIM-1")),
    ] {
        let rdir = path_join(STAGE, sub);
        let mut names: Vec<String> = store
            .borrow_mut()
            .storage
            .list(&rdir)
            .unwrap_or_default()
            .into_iter()
            .filter(|n| n.ends_with(".mcr"))
            .collect();
        names.sort();
        let mut loader = McRegionChunkLoader::new(store.clone(), &chunk_dir);
        for name in names {
            let path = path_join(&rdir, &name);
            digest.regions.push(rel(&path).to_string());
            let bytes = store
                .borrow_mut()
                .storage
                .read(&path)
                .map_err(|e| corrupt(path.as_str(), e))?
                .unwrap_or_default();
            // r.<x>.<z>.mcr (RegionFileCache.java:20-21); `validate` refused
            // any other name.
            let parts: Vec<&str> = name.split('.').collect();
            let (rx, rz) = match parts.as_slice() {
                ["r", x, z, "mcr"] => (
                    x.parse::<i32>().map_err(|e| corrupt(path.as_str(), e))?,
                    z.parse::<i32>().map_err(|e| corrupt(path.as_str(), e))?,
                ),
                _ => return Err(corrupt(path.as_str(), "not a region file name")),
            };
            // Read-only, on a scratch copy (opening may pad the file,
            // RegionFile.java:36-52).
            let mut scratch = MemoryStorage::default();
            scratch
                .write(&path, &bytes)
                .map_err(|e| corrupt(path.as_str(), e))?;
            let region =
                RegionFile::open(&mut scratch, &path).map_err(|e| corrupt(path.as_str(), e))?;
            for i in 0..1024 {
                let (lx, lz) = (i & 31, i >> 5);
                if !region.has_chunk(lx, lz) {
                    continue;
                }
                let (x, z) = (rx * 32 + lx, rz * 32 + lz);
                let at = |m: &dyn std::fmt::Display| format!("chunk ({x}, {z}): {m}");
                let raw = region
                    .get_chunk_data_input_stream(lx, lz)
                    .map_err(|e| corrupt(path.as_str(), at(&e)))?
                    .ok_or_else(|| {
                        corrupt(
                            path.as_str(),
                            at(&"listed in the header but unreadable (past the end of the file, or a bad length or version)"),
                        )
                    })?;
                let (nbt_name, tree) = mc_sim::save::mc_nbt::read_named(&raw)
                    .map_err(|e| corrupt(path.as_str(), at(&format!("{e:?}"))))?;
                let nbt = hex(&mc_sim::save::mc_nbt::write_named(&nbt_name, &tree)
                    .map_err(|e| corrupt(path.as_str(), at(&format!("{e:?}"))))?);
                let loaded = loader
                    .load_chunk(&mut world, x, z)
                    .map_err(|e| corrupt(path.as_str(), at(&e)))?
                    .ok_or_else(|| {
                        corrupt(
                            path.as_str(),
                            at(&"McRegionChunkLoader.loadChunk returns null (no Level or Blocks)"),
                        )
                    })?;
                let a = mc_sim::golden::ChunkArrays::of(&loaded.chunk);
                digest.chunks.push(ChunkDigest {
                    dimension,
                    x,
                    z,
                    blocks: hex(a.blocks),
                    data: hex(a.data),
                    skylight: hex(a.skylight),
                    blocklight: hex(a.blocklight),
                    heightmap: hex(a.heightmap),
                    nbt,
                });
                discard(&mut world, loaded);
            }
        }
    }
    Ok(digest)
}

/// Step 5: the copy against the source.
pub fn compare(source: &WorldDigest, copy: &WorldDigest) -> Result<(), ExportError> {
    let mm = |m: String| Err(ExportError::Mismatch(m));
    for (a, b) in source.files.iter().zip(&copy.files) {
        if a.0 != b.0 {
            return mm(format!("file {} where the world has {}", b.0, a.0));
        }
        if a.1 != b.1 || a.2 != b.2 {
            return mm(format!("{} differs", a.0));
        }
    }
    if source.files.len() != copy.files.len() {
        return mm(format!(
            "{} files where the world has {}",
            copy.files.len(),
            source.files.len()
        ));
    }
    if source.level_dat != copy.level_dat {
        return mm("level.dat reads differently".into());
    }
    for (a, b) in source.chunks.iter().zip(&copy.chunks) {
        if (a.dimension, a.x, a.z) != (b.dimension, b.x, b.z) {
            return mm(format!(
                "chunk ({}, {}) where the world has chunk ({}, {})",
                b.x, b.z, a.x, a.z
            ));
        }
        if let Some(f) = a.first_difference(b) {
            return mm(format!(
                "chunk ({}, {}) of dimension {}: {f} differs",
                a.x, a.z, a.dimension
            ));
        }
    }
    if source.chunks.len() != copy.chunks.len() {
        return mm(format!(
            "{} chunks where the world has {}",
            copy.chunks.len(),
            source.chunks.len()
        ));
    }
    if source != copy {
        return mm("the world's header reads differently".into());
    }
    Ok(())
}

/// Whether `dir` is absent or an empty directory.
fn target_free(dir: &Path) -> bool {
    match fs::symlink_metadata(dir) {
        Err(e) => e.kind() == std::io::ErrorKind::NotFound,
        Ok(m) => m.is_dir() && fs::read_dir(dir).is_ok_and(|mut it| it.next().is_none()),
    }
}

/// Whether a world folder name is a plain name.
fn plain_name(name: &str) -> bool {
    !(name.is_empty()
        || name == "."
        || name == ".."
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0'))
}

/// Exports `saves_dir/<name>` to `target/<name>` (module docs, steps 1-6).
pub fn export_world(
    saves_dir: &Path,
    name: &str,
    target: &Path,
) -> Result<ExportReport, ExportError> {
    if !plain_name(name) {
        return Err(ExportError::NoWorld(name.to_string()));
    }
    let src = saves_dir.join(name);
    if !src.is_dir() {
        return Err(ExportError::NoWorld(format!(
            "{} is not a world folder",
            src.display()
        )));
    }
    // 1. the target
    let dest = target.join(name);
    if !target_free(&dest) {
        return Err(ExportError::TargetNotEmpty(dest));
    }
    if let (Ok(s), Ok(t)) = (src.canonicalize(), absolute_existing_prefix(target))
        && t.starts_with(&s)
    {
        return Err(ExportError::TargetNotEmpty(dest));
    }
    // 2. read, 3. validate
    let files = read_tree(&src).map_err(ExportError::Read)?;
    if files.is_empty() {
        return Err(ExportError::Read(format!("{} is empty", src.display())));
    }
    let source = world_digest(&files)?;
    // 4. copy into the staging folder
    fs::create_dir_all(target)
        .map_err(|e| ExportError::Write(format!("{}: {e}", target.display())))?;
    let staging = target.join(format!("{STAGING_PREFIX}{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&staging);
    let checked = (|| {
        write_tree(&staging, &files).map_err(ExportError::Write)?;
        // 5. re-read and compare
        let back = read_tree(&staging)
            .map_err(|e| ExportError::Mismatch(format!("reading the copy: {e}")))?;
        let copy = world_digest(&back)
            .map_err(|e| ExportError::Mismatch(format!("the copy does not load: {e}")))?;
        compare(&source, &copy)?;
        // 6. publish (the target may have appeared meanwhile)
        if !target_free(&dest) {
            return Err(ExportError::TargetNotEmpty(dest.clone()));
        }
        if dest.exists() {
            fs::remove_dir(&dest)
                .map_err(|e| ExportError::Write(format!("{}: {e}", dest.display())))?;
        }
        fs::rename(&staging, &dest)
            .map_err(|e| ExportError::Write(format!("{}: {e}", dest.display())))?;
        Ok(copy)
    })();
    match checked {
        Ok(digest) => Ok(ExportReport {
            path: dest,
            files: files.len(),
            bytes: files.iter().map(|(_, b)| b.len() as u64).sum(),
            digest,
        }),
        Err(e) => {
            let _ = fs::remove_dir_all(&staging);
            Err(e)
        }
    }
}

/// The longest existing ancestor of `p` (itself included), canonicalized,
/// with the rest appended: where `p` would be once created.
fn absolute_existing_prefix(p: &Path) -> std::io::Result<PathBuf> {
    let mut rest = Vec::new();
    let mut cur = p.to_path_buf();
    loop {
        match cur.canonicalize() {
            Ok(c) => {
                let mut out = c;
                for r in rest.iter().rev() {
                    out.push(r);
                }
                return Ok(out);
            }
            Err(e) => {
                let Some(name) = cur.file_name().map(|n| n.to_os_string()) else {
                    return Err(e);
                };
                rest.push(name);
                if !cur.pop() {
                    return Err(e);
                }
                if cur.as_os_str().is_empty() {
                    cur = PathBuf::from(".");
                }
            }
        }
    }
}

/// The export as the game runs it: the open world is saved first when it is
/// the one exported (`World.saveWorld(true, null)`, World.java:277-290), the
/// region-file cache is flushed (`SaveConverterMcRegion.flushCache`,
/// SaveConverterMcRegion.java:49-51), then [`export_world`] from the data
/// directory's `saves/`.
pub fn export_from_game(
    game: &mut Game,
    name: &str,
    target: &Path,
) -> Result<ExportReport, ExportError> {
    if game.world_dir.as_deref() == Some(name)
        && let Some(w) = game.the_world.as_mut()
    {
        w.try_save_world(true)
            .map_err(|e| ExportError::Save(e.to_string()))?;
    }
    game.save_loader.flush_cache();
    export_world(&game.paths.data_dir.join(SAVES_DIR), name, target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::tests::TempDir;
    use mc_sim::save::{RegionFileCache, TagType};
    use std::collections::BTreeMap;

    /// A gzip `level.dat` with a player at `pos`.
    fn level_dat(name: &str, pos: [f64; 3]) -> Vec<u8> {
        let mut player = BTreeMap::new();
        player.insert(
            "Pos".to_string(),
            Tag::List {
                elem: TagType::Double,
                items: pos.iter().map(|&v| Tag::Double(v)).collect(),
            },
        );
        let mut info = mc_sim::world::info::WorldInfo::new(123, name);
        info.save_version = MCREGION_VERSION;
        let data = info.get_nbt_tag_compound_with_player(Some(Tag::Compound(player)), 0);
        let mut root = BTreeMap::new();
        root.insert("Data".to_string(), Tag::Compound(data));
        mc_sim::save::mc_nbt::write_gzip("", &Tag::Compound(root)).unwrap()
    }

    /// A chunk compound at `(x, z)` with stone at the bottom and block
    /// `id` at local (1, 64, 1).
    fn chunk_nbt(x: i32, z: i32, id: u8) -> Vec<u8> {
        let mut blocks = vec![0u8; 32768];
        for i in 0..256usize {
            blocks[(i >> 4) << 11 | (i & 15) << 7] = 1;
        }
        blocks[1 << 11 | 1 << 7 | 64] = id;
        let mut level = BTreeMap::new();
        level.insert("xPos".to_string(), Tag::Int(x));
        level.insert("zPos".to_string(), Tag::Int(z));
        level.insert("LastUpdate".to_string(), Tag::Long(7));
        level.insert("Blocks".to_string(), Tag::ByteArray(blocks));
        level.insert("Data".to_string(), Tag::ByteArray(vec![0; 16384]));
        level.insert("SkyLight".to_string(), Tag::ByteArray(vec![0xff; 16384]));
        level.insert("BlockLight".to_string(), Tag::ByteArray(vec![0; 16384]));
        level.insert("HeightMap".to_string(), Tag::ByteArray(vec![1; 256]));
        level.insert("TerrainPopulated".to_string(), Tag::Byte(1));
        for k in ["Entities", "TileEntities"] {
            level.insert(
                k.to_string(),
                Tag::List {
                    elem: TagType::Compound,
                    items: vec![],
                },
            );
        }
        let mut root = BTreeMap::new();
        root.insert("Level".to_string(), Tag::Compound(level));
        mc_sim::save::mc_nbt::write_named("", &Tag::Compound(root)).unwrap()
    }

    /// A world folder `saves/<name>` under `data` with chunks (0, 0) and
    /// (1, 0) in `r.0.0.mcr`, (−1, −1) of the nether in `DIM-1`; `bad` is
    /// the block id at (1, 64, 1) of chunk (1, 0).
    fn make_world(data: &Path, name: &str, bad: u8) -> PathBuf {
        let mut st = MemoryStorage::default();
        st.write(
            &format!("{name}/level.dat"),
            &level_dat(name, [8.5, 66.62, -3.5]),
        )
        .unwrap();
        st.write(&format!("{name}/session.lock"), &[0, 0, 0, 0, 0, 0, 0, 1])
            .unwrap();
        let mut cache = RegionFileCache::new();
        let dir = name.to_string();
        cache
            .write_chunk(&mut st, &dir, 0, 0, &chunk_nbt(0, 0, 4), 1)
            .unwrap();
        cache
            .write_chunk(&mut st, &dir, 1, 0, &chunk_nbt(1, 0, bad), 1)
            .unwrap();
        let nether = format!("{name}/DIM-1");
        cache
            .write_chunk(&mut st, &nether, -1, -1, &chunk_nbt(-1, -1, 87), 1)
            .unwrap();
        let saves = data.join("saves");
        for (p, b) in &st.blobs {
            let to = saves.join(p);
            fs::create_dir_all(to.parent().unwrap()).unwrap();
            fs::write(&to, b).unwrap();
        }
        saves.join(name)
    }

    #[test]
    fn exports_a_good_world_and_reads_it_back() {
        let d = TempDir::new("export_good");
        let src = make_world(&d.0, "Good", 4);
        let out = d.0.join("out");
        let r = export_world(&d.0.join("saves"), "Good", &out).unwrap();
        assert_eq!(r.path, out.join("Good"));
        assert_eq!(r.digest.chunks.len(), 3);
        assert_eq!(
            r.digest.regions,
            vec!["region/r.0.0.mcr", "DIM-1/region/r.-1.-1.mcr"]
        );
        assert_eq!(r.digest.player_pos, Some([8.5, 66.62, -3.5]));
        assert_eq!(r.digest.seed, 123);
        assert_eq!(r.files, 4);
        // The copy has the source's bytes, and nothing else is left behind.
        assert_eq!(read_tree(&src).unwrap(), read_tree(&r.path).unwrap());
        let left: Vec<_> = fs::read_dir(&out)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(left, vec![std::ffi::OsString::from("Good")]);
        // The digest of the export, computed again, equals the report's.
        let again = world_digest(&read_tree(&r.path).unwrap()).unwrap();
        assert_eq!(again, r.digest);
        // The loaded arrays: block 4 at (1, 64, 1) of chunk (0, 0).
        let c00 = &r.digest.chunks[0];
        assert_eq!((c00.dimension, c00.x, c00.z), (0, 0, 0));
        let mut blocks = vec![0u8; 32768];
        for i in 0..256usize {
            blocks[(i >> 4) << 11 | (i & 15) << 7] = 1;
        }
        blocks[1 << 11 | 1 << 7 | 64] = 4;
        assert_eq!(c00.blocks, hex(&blocks));
        assert_eq!(r.digest.chunks[2].dimension, -1);
    }

    #[test]
    fn refuses_an_unknown_block_id_and_copies_nothing() {
        let d = TempDir::new("export_bad_id");
        make_world(&d.0, "Modded", 200);
        let out = d.0.join("out");
        let e = export_world(&d.0.join("saves"), "Modded", &out).unwrap_err();
        let ExportError::Refused(ImportRefusal::UnknownBlock { id, location, .. }) = &e else {
            panic!("{e}");
        };
        assert_eq!(*id, 200);
        assert_eq!(location.to_string(), "chunk (1, 0)");
        assert!(!out.join("Modded").exists());
        assert!(!out.exists() || fs::read_dir(&out).unwrap().next().is_none());
    }

    #[test]
    fn refuses_a_truncated_region_file() {
        let d = TempDir::new("export_trunc");
        let src = make_world(&d.0, "Trunc", 4);
        let r = src.join("region/r.0.0.mcr");
        let bytes = fs::read(&r).unwrap();
        // Header plus the first chunk's sector only: chunk (1, 0) now lies
        // past the end (vanilla's reader returns null; the import's scan
        // skips it).
        fs::write(&r, &bytes[..3 * 4096]).unwrap();
        let mut st = MemoryStorage::default();
        for (p, b) in read_tree(&src).unwrap() {
            st.write(&format!("w/{p}"), &b).unwrap();
        }
        assert_eq!(validate(&mut st, "w"), Ok(()));
        let out = d.0.join("out");
        let e = export_world(&d.0.join("saves"), "Trunc", &out).unwrap_err();
        let ExportError::Corrupt { path, reason } = &e else {
            panic!("{e}");
        };
        assert_eq!(path, "world/region/r.0.0.mcr");
        assert!(
            reason.starts_with("chunk (1, 0): listed in the header"),
            "{reason}"
        );
        assert!(
            e.to_string().contains("region/r.0.0.mcr: chunk (1, 0)"),
            "{e}"
        );
        assert!(!out.join("Trunc").exists());
    }

    #[test]
    fn refuses_a_missing_level_dat() {
        let d = TempDir::new("export_nolevel");
        let src = make_world(&d.0, "NoLevel", 4);
        fs::remove_file(src.join("level.dat")).unwrap();
        let e = export_world(&d.0.join("saves"), "NoLevel", &d.0.join("out")).unwrap_err();
        assert_eq!(
            e,
            ExportError::Refused(ImportRefusal::Unreadable {
                path: "level.dat".into(),
                reason: "missing".into()
            })
        );
        assert!(!d.0.join("out").exists());
    }

    /// A link to a directory is refused (a link to a parent would never
    /// end); nothing is copied.
    #[cfg(unix)]
    #[test]
    fn refuses_a_link_to_a_directory() {
        let d = TempDir::new("export_link");
        let src = make_world(&d.0, "Linked", 4);
        std::os::unix::fs::symlink(&src, src.join("loop")).unwrap();
        let e = export_world(&d.0.join("saves"), "Linked", &d.0.join("out")).unwrap_err();
        assert!(
            matches!(&e, ExportError::Read(m) if m.ends_with("loop: a link to a directory")),
            "{e}"
        );
        assert!(!d.0.join("out").exists());
    }

    #[test]
    fn never_overwrites_a_non_empty_target() {
        let d = TempDir::new("export_exists");
        make_world(&d.0, "W", 4);
        let out = d.0.join("out");
        fs::create_dir_all(out.join("W")).unwrap();
        fs::write(out.join("W/keep.txt"), b"mine").unwrap();
        let e = export_world(&d.0.join("saves"), "W", &out).unwrap_err();
        assert_eq!(e, ExportError::TargetNotEmpty(out.join("W")));
        assert_eq!(fs::read(out.join("W/keep.txt")).unwrap(), b"mine");
        assert_eq!(fs::read_dir(out.join("W")).unwrap().count(), 1);
        // A file in the way is refused too; an empty folder is used.
        fs::remove_dir_all(out.join("W")).unwrap();
        fs::write(out.join("W"), b"x").unwrap();
        assert!(matches!(
            export_world(&d.0.join("saves"), "W", &out),
            Err(ExportError::TargetNotEmpty(_))
        ));
        fs::remove_file(out.join("W")).unwrap();
        fs::create_dir(out.join("W")).unwrap();
        let r = export_world(&d.0.join("saves"), "W", &out).unwrap();
        assert!(r.path.join("level.dat").exists());
        // Exporting again: the first export is now in the way.
        assert!(matches!(
            export_world(&d.0.join("saves"), "W", &out),
            Err(ExportError::TargetNotEmpty(_))
        ));
    }

    #[test]
    fn refuses_a_target_inside_the_world_and_unknown_worlds() {
        let d = TempDir::new("export_inside");
        let src = make_world(&d.0, "W", 4);
        assert!(matches!(
            export_world(&d.0.join("saves"), "W", &src.join("sub")),
            Err(ExportError::TargetNotEmpty(_))
        ));
        assert!(!src.join("sub").exists());
        assert!(matches!(
            export_world(&d.0.join("saves"), "Nope", &d.0.join("out")),
            Err(ExportError::NoWorld(_))
        ));
        assert!(matches!(
            export_world(&d.0.join("saves"), "../W", &d.0.join("out")),
            Err(ExportError::NoWorld(_))
        ));
    }

    #[test]
    fn the_comparison_names_the_first_difference() {
        let d = TempDir::new("export_cmp");
        let src = make_world(&d.0, "A", 4);
        let a = world_digest(&read_tree(&src).unwrap()).unwrap();
        assert_eq!(compare(&a, &a), Ok(()));
        // The same world with another block in chunk (1, 0): its file and
        // that chunk's blocks and NBT differ.
        let other = make_world(&d.0, "B", 5);
        let mut b = world_digest(&read_tree(&other).unwrap()).unwrap();
        b.level_name = a.level_name.clone();
        let e = compare(&a, &b).unwrap_err();
        assert_eq!(e, ExportError::Mismatch("level.dat differs".into()));
        b.files = a.files.clone();
        b.level_dat = a.level_dat.clone();
        assert_eq!(
            compare(&a, &b),
            Err(ExportError::Mismatch(
                "chunk (1, 0) of dimension 0: blocks differs".into()
            ))
        );
        b.chunks[1].blocks = a.chunks[1].blocks.clone();
        assert_eq!(
            compare(&a, &b),
            Err(ExportError::Mismatch(
                "chunk (1, 0) of dimension 0: nbt differs".into()
            ))
        );
        b.chunks.pop();
        b.chunks[1] = a.chunks[1].clone();
        assert_eq!(
            compare(&a, &b),
            Err(ExportError::Mismatch(
                "2 chunks where the world has 3".into()
            ))
        );
    }

    /// The reference testWorld when present (never committed, D8).
    #[test]
    fn the_reference_test_world_when_present() {
        let tw = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.reference/mc_b1.7.3-world/testWorld");
        if !tw.exists() {
            return;
        }
        let d = TempDir::new("export_ref");
        let saves = d.0.join("saves");
        write_tree(&saves.join("testWorld"), &read_tree(&tw).unwrap()).unwrap();
        let r = export_world(&saves, "testWorld", &d.0.join("out")).unwrap();
        assert_eq!(r.digest.seed, 123);
        assert!(r.digest.chunks.len() > 100, "{}", r.digest.chunks.len());
        assert_eq!(read_tree(&tw).unwrap(), read_tree(&r.path).unwrap());
    }
}
