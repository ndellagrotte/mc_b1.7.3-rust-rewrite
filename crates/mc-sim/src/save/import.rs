//! Import validation (PLAN_v4 D5): before a vanilla save folder is copied
//! into the saves directory, [`validate`] scans it and refuses it at the
//! first block or item id that Beta 1.7.3 does not define, naming the id and
//! where it is. Copying (and so "nothing is copied when it refuses") is the
//! client's job; this module only reads.
//!
//! Not a vanilla class: vanilla loads such a world and silently turns
//! unknown blocks into air (`Chunk.removeUnknownBlocks`, ChunkBlockMap.java)
//! or crashes on an unknown item. The scan reads what the vanilla readers
//! read, with the same codecs.
//!
//! # Order (the "first" offending id)
//!
//! 1. `level.dat` (`level.dat_old` if it is missing or does not read, as
//!    `SaveHandler.loadWorldInfo` falls back, SaveHandler.java:81-107): the player's
//!    `Inventory` item ids (InventoryPlayer.java:223-241).
//! 2. The overworld's `region/*.mcr`, then the nether's `DIM-1/region/*.mcr`,
//!    each in file-name order; in a file, the present chunks in index order
//!    `x + z * 32` (RegionFile.java:67-75). In a chunk's `Level`:
//!    - `Blocks`, in array order (`x << 11 | z << 7 | y`): a non-zero id with
//!      no `Block.blocksList` entry;
//!    - `Entities`, in list order: an `"Item"`'s `Item.id`
//!      (EntityItem.java:103-108), a `"FallingSand"`'s `Tile & 255` (a block,
//!      EntityFallingSand.java:77-79), and any entity's `Items` list (the
//!      chest minecart, EntityMinecart.java:598-616);
//!    - `TileEntities`, in list order: the `Items` lists (chest, furnace,
//!      dispenser), a record player's non-zero `Record`
//!      (TileEntityRecordPlayer.java:7-16), a moving piston's `blockId`
//!      (TileEntityPiston.java:128-143, a block).
//!
//! An item id is defined when `Item.itemsList[id]` is not null
//! ([`crate::item::item`]: the `ItemBlock`s of every block, ids 256–359 and
//! the two records); a block id when it is 0 or `Block.blocksList[id]` is not
//! null ([`crate::block::block`]).
//!
//! # Unreadable data
//!
//! A world where neither `level.dat` nor `level.dat_old` is present and
//! decodes into a `WorldInfo`, a region file vanilla's `RegionFile` cannot open, or a present
//! chunk whose stored bytes do not decode as NBT, is refused as
//! [`ImportRefusal::Unreadable`]: the scan cannot vouch for ids it cannot
//! read. (Vanilla would regenerate such a chunk.) A chunk without `Level` or
//! without `Level.Blocks` is skipped, as `McRegionChunkLoader.loadChunk`
//! skips it (McRegionChunkLoader.java:19-24).

use std::fmt;

use mc_nbt::Tag;

use super::nbt::{self as n, Compound};
use super::region::REGION_ENTRIES;
use super::save_handler::world_info_from_level_dat;
use super::{MemoryStorage, RegionFile, Storage, StorageError, path_join};
use crate::block;
use crate::item;

/// Where an offending id was found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Location {
    /// `level.dat` (the player).
    LevelDat,
    /// A chunk: `dimension` 0 for `region/`, -1 for `DIM-1/region/`.
    Chunk {
        /// Dimension of the region directory.
        dimension: i32,
        /// Chunk x.
        x: i32,
        /// Chunk z.
        z: i32,
    },
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Location::LevelDat => f.write_str("level.dat"),
            Location::Chunk { dimension: 0, x, z } => write!(f, "chunk ({x}, {z})"),
            Location::Chunk { dimension, x, z } => {
                write!(f, "chunk ({x}, {z}) of dimension {dimension}")
            }
        }
    }
}

/// Why a save is refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportRefusal {
    /// A block id Beta 1.7.3 does not define.
    UnknownBlock {
        /// The id.
        id: i32,
        /// Where.
        location: Location,
        /// What held it (e.g. `"Blocks"` with the block position).
        detail: String,
    },
    /// An item id Beta 1.7.3 does not define.
    UnknownItem {
        /// The id.
        id: i32,
        /// Where.
        location: Location,
        /// What held it (e.g. `"TileEntities[0] Chest Items slot 1"`).
        detail: String,
    },
    /// Data the scan cannot read (module docs).
    Unreadable {
        /// The storage path.
        path: String,
        /// Why.
        reason: String,
    },
}

impl fmt::Display for ImportRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImportRefusal::UnknownBlock {
                id,
                location,
                detail,
            } => write!(
                f,
                "block id {id} is not a Beta 1.7.3 block ({location}, {detail})"
            ),
            ImportRefusal::UnknownItem {
                id,
                location,
                detail,
            } => write!(
                f,
                "item id {id} is not a Beta 1.7.3 item ({location}, {detail})"
            ),
            ImportRefusal::Unreadable { path, reason } => write!(f, "cannot read {path}: {reason}"),
        }
    }
}

impl std::error::Error for ImportRefusal {}

fn unreadable(path: &str, reason: impl fmt::Display) -> ImportRefusal {
    ImportRefusal::Unreadable {
        path: path.to_string(),
        reason: reason.to_string(),
    }
}

/// Whether `id` is a Beta 1.7.3 block id (0 is air).
pub fn is_known_block(id: i32) -> bool {
    id == 0 || block::block(id).is_some()
}

/// Whether `id` is a Beta 1.7.3 item id (`Item.itemsList[id] != null`).
pub fn is_known_item(id: i32) -> bool {
    item::item(id).is_some()
}

/// Validates the save in `dir` (a storage path; `""` for the root) as
/// described in the module docs. `Ok(())` means every id is defined.
pub fn validate(storage: &mut dyn Storage, dir: &str) -> Result<(), ImportRefusal> {
    validate_level_dat(storage, dir)?;
    for (dimension, sub) in [(0, "region"), (-1, "DIM-1/region")] {
        let rdir = path_join(dir, sub);
        // A missing region directory (no nether visited) has nothing to check.
        let names = match storage.list(&rdir) {
            Ok(n) => n,
            Err(StorageError::NotFound(_)) => Vec::new(),
            Err(e) => return Err(unreadable(&rdir, e)),
        };
        for name in names.iter().filter(|n| n.ends_with(".mcr")) {
            validate_region(storage, &rdir, name, dimension)?;
        }
    }
    Ok(())
}

/// The file `SaveHandler.loadWorldInfo` would use: `level.dat`, or
/// `level.dat_old` when `level.dat` is missing or fails to read (decoded
/// with [`world_info_from_level_dat`], as the port's loader does); refused as
/// unreadable only when neither yields a `WorldInfo`. The player compound
/// checked is that file's (`WorldInfo.playerTag`, WorldInfo.java:36-39).
// src: SaveHandler.java:81-107
fn validate_level_dat(storage: &mut dyn Storage, dir: &str) -> Result<(), ImportRefusal> {
    let mut first_failure: Option<ImportRefusal> = None;
    for name in ["level.dat", "level.dat_old"] {
        let path = path_join(dir, name);
        let bytes = match storage.read(&path) {
            Ok(Some(b)) => b,
            Ok(None) => continue, // `var1.exists()` false
            Err(e) => {
                first_failure.get_or_insert_with(|| unreadable(&path, e));
                continue;
            }
        };
        match world_info_from_level_dat(&bytes) {
            Ok(info) => {
                if let Some(Tag::Compound(player)) = &info.player_tag {
                    check_items(
                        player.get("Inventory"),
                        &Location::LevelDat,
                        "Player Inventory",
                    )?;
                }
                return Ok(());
            }
            // Printed in Java; the next file is tried.
            Err(e) => {
                first_failure.get_or_insert_with(|| unreadable(&path, e));
            }
        }
    }
    Err(first_failure.unwrap_or_else(|| unreadable(&path_join(dir, "level.dat"), "missing")))
}

/// Parses a region file read-only (on a scratch copy: opening may pad or
/// rewrite it, RegionFile.java:36-63).
fn validate_region(
    storage: &mut dyn Storage,
    rdir: &str,
    name: &str,
    dimension: i32,
) -> Result<(), ImportRefusal> {
    let path = path_join(rdir, name);
    let bytes = storage
        .read(&path)
        .map_err(|e| unreadable(&path, e))?
        .unwrap_or_default();
    // r.<x>.<z>.mcr (RegionFileCache.java:20-21)
    let parts: Vec<&str> = name.split('.').collect();
    let (rx, rz) = match parts.as_slice() {
        ["r", x, z, "mcr"] => match (x.parse::<i32>(), z.parse::<i32>()) {
            (Ok(x), Ok(z)) => (x, z),
            _ => return Err(unreadable(&path, "not a region file name")),
        },
        _ => return Err(unreadable(&path, "not a region file name")),
    };
    let mut scratch = MemoryStorage::default();
    scratch
        .write(&path, &bytes)
        .map_err(|e| unreadable(&path, e))?;
    let region = RegionFile::open(&mut scratch, &path).map_err(|e| unreadable(&path, e))?;
    for i in 0..REGION_ENTRIES as i32 {
        let (lx, lz) = (i & 31, i >> 5);
        if !region.has_chunk(lx, lz) {
            continue;
        }
        let location = Location::Chunk {
            dimension,
            x: rx.wrapping_mul(32).wrapping_add(lx),
            z: rz.wrapping_mul(32).wrapping_add(lz),
        };
        let data = region
            .get_chunk_data_input_stream(lx, lz)
            .map_err(|e| unreadable(&path, format!("{location}: {e}")))?;
        // A non-zero offset the reader returns null for (bad length or
        // version): vanilla regenerates the chunk; nothing to check.
        let Some(data) = data else {
            continue;
        };
        let (_, root) = mc_nbt::read_named(&data)
            .map_err(|e| unreadable(&path, format!("{location}: {e:?}")))?;
        validate_chunk(&root, &location)?;
    }
    Ok(())
}

fn validate_chunk(root: &Tag, location: &Location) -> Result<(), ImportRefusal> {
    let Some(Tag::Compound(level)) = root.get("Level") else {
        return Ok(());
    };
    let Some(Tag::ByteArray(blocks)) = level.get("Blocks") else {
        return Ok(());
    };
    for (i, &b) in blocks.iter().enumerate() {
        if !is_known_block(b as i32) {
            return Err(ImportRefusal::UnknownBlock {
                id: b as i32,
                location: location.clone(),
                detail: format!(
                    "Blocks at local x {}, y {}, z {}",
                    i >> 11,
                    i & 127,
                    (i >> 7) & 15
                ),
            });
        }
    }
    if let Some(Tag::List { items, .. }) = level.get("Entities") {
        for (k, e) in items.iter().enumerate() {
            let Tag::Compound(e) = e else { continue };
            let name = n::get_string(e, "id").unwrap_or_default();
            let what = format!("Entities[{k}] {name}");
            if name == "Item"
                && let Some(Tag::Compound(st)) = e.get("Item")
            {
                check_item_compound(st, location, &format!("{what} Item"))?;
            }
            if name == "FallingSand"
                && let Ok(t) = n::get_byte(e, "Tile")
            {
                let id = t as i32 & 255;
                if !is_known_block(id) {
                    return Err(ImportRefusal::UnknownBlock {
                        id,
                        location: location.clone(),
                        detail: format!("{what} Tile"),
                    });
                }
            }
            check_items(e.get("Items"), location, &what)?;
        }
    }
    if let Some(Tag::List { items, .. }) = level.get("TileEntities") {
        for (k, te) in items.iter().enumerate() {
            let Tag::Compound(te) = te else { continue };
            let name = n::get_string(te, "id").unwrap_or_default();
            let what = format!("TileEntities[{k}] {name}");
            check_items(te.get("Items"), location, &what)?;
            if name == "RecordPlayer"
                && let Ok(r) = n::get_integer(te, "Record")
                && r != 0
                && !is_known_item(r)
            {
                return Err(ImportRefusal::UnknownItem {
                    id: r,
                    location: location.clone(),
                    detail: format!("{what} Record"),
                });
            }
            if name == "Piston"
                && let Ok(id) = n::get_integer(te, "blockId")
                && !is_known_block(id)
            {
                return Err(ImportRefusal::UnknownBlock {
                    id,
                    location: location.clone(),
                    detail: format!("{what} blockId"),
                });
            }
        }
    }
    Ok(())
}

/// An `Items`-style list: every compound's `id`.
fn check_items(list: Option<&Tag>, location: &Location, what: &str) -> Result<(), ImportRefusal> {
    let Some(Tag::List { items, .. }) = list else {
        return Ok(());
    };
    for st in items {
        if let Tag::Compound(st) = st {
            let slot = n::get_byte(st, "Slot").unwrap_or(0) as i32 & 255;
            check_item_compound(st, location, &format!("{what} slot {slot}"))?;
        }
    }
    Ok(())
}

/// One item stack compound (`ItemStack.readFromNBT`, ItemStack.java:82-86:
/// `id` is a short).
fn check_item_compound(
    st: &Compound,
    location: &Location,
    what: &str,
) -> Result<(), ImportRefusal> {
    let id = n::get_short(st, "id").unwrap_or(0) as i32;
    if !is_known_item(id) {
        return Err(ImportRefusal::UnknownItem {
            id,
            location: location.clone(),
            detail: what.to_string(),
        });
    }
    Ok(())
}
