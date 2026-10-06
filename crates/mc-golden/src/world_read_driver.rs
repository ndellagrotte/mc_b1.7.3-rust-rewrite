//! Tier S (tests/golden/README.md "Tier S: the `save` op and
//! `kind: \"world_read\"`"; oracle: `tools/oracle/java/harness/TierS.java`).
//!
//! - [`drive_in`]: the drive closure for `kind: "world_read"`. The sidecar
//!   save directory is copied into a [`MemoryStorage`] as `world/…` (the
//!   oracle copies it to a temp directory, because opening a save writes
//!   `session.lock`), opened with the loading constructor
//!   `World(SaveOldDir, "harness", 0, null)` (`HarnessWorld.open`:
//!   [`World::new_from_save`] with no provider, then
//!   [`World::make_deterministic`] with seed 0), and [`emit_tier_s`] writes
//!   the lines at `t = 0`, unmasked.
//! - [`emit_tier_s`]: `TierS.emitFiles`, shared with the tick driver's
//!   `save` op: `level_dat` (masked after a save), `world_info`, one
//!   `region_header` per `region/*.mcr` in name order, then per chunk
//!   `chunk_nbt`, `chunk_digest` (`"loaded"`, `world_read` only, through a
//!   fresh [`McRegionChunkLoader`]) and the `tile_entity` lines.
//! - [`canonical_root`]: the README's canonical NBT tree.

use std::path::{Path, PathBuf};

use mc_sim::save::chunk_loader::discard;
use mc_sim::save::{
    ChunkLoader, FixedClock, McRegionChunkLoader, MemoryStorage, SaveOldDir, SaveStore,
    SharedSaveStore, Storage, Tag, TagType, path_join,
};
use mc_sim::tile_entity;
use mc_sim::world::info::WorldInfo;
use mc_sim::world::{World, WorldEnvironment};
use serde_json::{Map, Value, json};

use crate::scenario::ChunkPos;
use crate::trace::{ChunkArrays, Stage, TileItem, WorldInfoLine};
use crate::{
    Bits32, Bits64, GoldenError, Scenario, ScenarioKind, TraceLine, TraceWriter, hex_bytes,
};

/// The clock of the harness's save code (`System.currentTimeMillis()`):
/// fixed, non-zero in seconds so region timestamps are set
/// (`timestamps_match`). `LastPlayed` and the timestamps themselves are
/// never compared.
pub const HARNESS_CLOCK_MILLIS: i64 = 1_700_000_000_000;

/// The save directory name inside the storage (the oracle's `world/`).
pub const WORLD_DIR: &str = "world";

/// `level.dat` keys removed after a `save` op (wall clock, deflate-dependent
/// sizes; TierS.java `MASKED`).
const MASKED: [&str; 2] = ["LastPlayed", "SizeOnDisk"];

fn err(m: impl std::fmt::Display) -> GoldenError {
    GoldenError::other(m.to_string())
}

// ---- canonical tree (README "Canonical tree") -------------------------------------

/// The README's name of a tag type.
pub fn type_name(ty: TagType) -> &'static str {
    match ty {
        TagType::End => "end",
        TagType::Byte => "byte",
        TagType::Short => "short",
        TagType::Int => "int",
        TagType::Long => "long",
        TagType::Float => "float",
        TagType::Double => "double",
        TagType::ByteArray => "byte_array",
        TagType::String => "string",
        TagType::List => "list",
        TagType::Compound => "compound",
    }
}

/// A node `{"type", "value"}`.
pub fn canonical_node(tag: &Tag) -> Value {
    json!({"type": type_name(tag.tag_type()), "value": canonical_value(tag)})
}

/// A node's value: floats as bit patterns, byte arrays as hex, a list as
/// `{"elem", "items"}` (`elem` the written type: `byte` when empty).
pub fn canonical_value(tag: &Tag) -> Value {
    match tag {
        Tag::Byte(v) => (*v).into(),
        Tag::Short(v) => (*v).into(),
        Tag::Int(v) => (*v).into(),
        Tag::Long(v) => (*v).into(),
        Tag::Float(v) => Bits32::from(*v).into(),
        Tag::Double(v) => Bits64::from(*v).into(),
        Tag::ByteArray(b) => hex_bytes(b).into(),
        Tag::String(s) => s.clone().into(),
        Tag::List { elem, items } => {
            let elem = if items.is_empty() {
                TagType::Byte
            } else {
                *elem
            };
            json!({"elem": type_name(elem), "items": items.iter().map(canonical_value).collect::<Vec<_>>()})
        }
        Tag::Compound(m) => {
            let mut o = Map::new();
            for (k, v) in m {
                o.insert(k.clone(), canonical_node(v));
            }
            Value::Object(o)
        }
    }
}

/// The root node: `canonical_node` plus `"name"`.
pub fn canonical_root(name: &str, tag: &Tag) -> Value {
    let mut node = canonical_node(tag);
    node.as_object_mut()
        .expect("node is an object")
        .insert("name".into(), name.into());
    node
}

// ---- storage ------------------------------------------------------------------------

/// The sidecar save directory `<fixture>/<sub>` copied into a
/// [`MemoryStorage`] under `world/`.
pub fn load_save_dir(fixture: &Path, sub: &str) -> Result<MemoryStorage, GoldenError> {
    if sub.contains('/') || sub.contains('\\') || sub.starts_with('.') {
        return Err(err("\"dir\" must be a plain name"));
    }
    let mut storage = MemoryStorage::default();
    for (rel, bytes) in crate::io::read_tree(&fixture.join(sub))? {
        storage
            .write(&path_join(WORLD_DIR, &rel), &bytes)
            .map_err(err)?;
    }
    Ok(storage)
}

/// A shared store over `storage` with the harness clock.
pub fn harness_store(storage: MemoryStorage) -> SharedSaveStore {
    SaveStore::shared(
        Box::new(storage),
        Box::new(FixedClock(HARNESS_CLOCK_MILLIS)),
    )
}

/// `HarnessWorld.open`: the loading constructor on `world/` of `store`, then
/// the determinism protocol with seed 0. An error if there is no `level.dat`.
pub fn open_world(store: &SharedSaveStore) -> Result<World, GoldenError> {
    let handler = SaveOldDir::new(store.clone(), "", WORLD_DIR).map_err(err)?;
    let mut world = World::new_from_save(
        Box::new(handler),
        "harness",
        0,
        None,
        WorldEnvironment {
            rand_seed: 0,
            field_9437_g: 0,
            lock_timestamp: 0,
        },
    );
    if world.is_new_world {
        return Err(err("no level.dat in the save"));
    }
    world.make_deterministic(0);
    Ok(world)
}

// ---- the drive closure ------------------------------------------------------------------

/// The drive closure for the `world_read` scenario in `fixture` (module
/// docs).
pub fn drive_in(
    fixture: impl Into<PathBuf>,
) -> impl FnOnce(&Scenario, &mut TraceWriter) -> Result<(), GoldenError> {
    let fixture = fixture.into();
    move |scenario, w| {
        let ScenarioKind::WorldRead { dir, chunks } = &scenario.kind else {
            return Err(err(format!(
                "world_read_driver: scenario {:?} is not kind \"world_read\"",
                scenario.name
            )));
        };
        let store = harness_store(load_save_dir(&fixture, dir)?);
        let mut world = open_world(&store)?;
        emit_tier_s(
            0,
            &store,
            WORLD_DIR,
            chunks.as_deref(),
            false,
            &mut world,
            w,
        )
    }
}

// ---- TierS.emitFiles --------------------------------------------------------------------

fn read(store: &SharedSaveStore, path: &str) -> Result<Option<Vec<u8>>, GoldenError> {
    store.borrow_mut().storage.read(path).map_err(err)
}

fn be_i32(b: &[u8], i: usize) -> i32 {
    i32::from_be_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

/// `TierS.emitFiles` (module docs). `mask` is true after a `save` op.
pub fn emit_tier_s(
    t: i64,
    store: &SharedSaveStore,
    dir: &str,
    chunks: Option<&[ChunkPos]>,
    mask: bool,
    world: &mut World,
    w: &mut TraceWriter,
) -> Result<(), GoldenError> {
    // 1. level_dat (SaveHandler.java:85: the gzip reader)
    let level_path = path_join(dir, "level.dat");
    let bytes = read(store, &level_path)?.ok_or_else(|| err("no level.dat"))?;
    let (root_name, root) =
        mc_sim::save::mc_nbt::read_gzip(&bytes).map_err(|e| err(format!("{e:?}")))?;
    let data = match root.get("Data") {
        Some(Tag::Compound(m)) => m.clone(),
        None => Default::default(),
        Some(_) => return Err(err("level.dat Data is not a compound")),
    };
    let mut masked: Vec<String> = Vec::new();
    let shown = if mask {
        let mut copy = data.clone();
        for k in MASKED {
            if copy.remove(k).is_some() {
                masked.push(format!("Data.{k}"));
            }
        }
        masked.sort();
        let mut r = std::collections::BTreeMap::new();
        r.insert("Data".to_string(), Tag::Compound(copy));
        Tag::Compound(r)
    } else {
        root.clone()
    };
    let masked_refs: Vec<&str> = masked.iter().map(String::as_str).collect();
    w.push(TraceLine::level_dat(
        t,
        canonical_root(&root_name, &shown),
        &masked_refs,
    ));

    // 2. world_info (WorldInfo.java:22-40, read back through its getters)
    let wi = WorldInfo::from_nbt(&data).map_err(|e| err(format!("WorldInfo: {e:?}")))?;
    w.push(TraceLine::world_info(
        t,
        &WorldInfoLine {
            random_seed: wi.random_seed,
            spawn: [wi.spawn_x, wi.spawn_y, wi.spawn_z],
            time: wi.world_time,
            level_name: &wi.level_name,
            version: wi.save_version,
            rain_time: wi.rain_time,
            raining: wi.raining,
            thunder_time: wi.thunder_time,
            thundering: wi.thundering,
            dimension: wi.dimension,
            player: wi.player_tag.is_some(),
        },
    ));

    // 3. region headers, file-name order
    let region_dir = path_join(dir, "region");
    let mut names: Vec<String> = store
        .borrow_mut()
        .storage
        .list(&region_dir)
        .map_err(err)?
        .into_iter()
        .filter(|n| n.ends_with(".mcr"))
        .collect();
    names.sort();
    let mut present_chunks: Vec<ChunkPos> = Vec::new();
    for name in &names {
        let b = read(store, &path_join(&region_dir, name))?.unwrap_or_default();
        if b.len() < 8192 {
            return Err(err(format!("{name}: shorter than its header")));
        }
        let parts: Vec<&str> = name.split('.').collect();
        let (rx, rz): (i32, i32) = match parts.as_slice() {
            [_, x, z, _] => (x.parse().map_err(err)?, z.parse().map_err(err)?),
            _ => return Err(err(format!("bad region name {name}"))),
        };
        let mut present = Vec::new();
        let mut locations = Vec::with_capacity(1024);
        let mut consistent = true;
        for i in 0..1024usize {
            let loc = be_i32(&b, i * 4);
            let ts = be_i32(&b, 4096 + i * 4);
            locations.push(loc);
            if loc != 0 {
                present.push(i as u32);
                present_chunks.push(ChunkPos {
                    x: rx * 32 + (i as i32 & 31),
                    z: rz * 32 + (i as i32 >> 5),
                });
            }
            if (loc != 0) != (ts != 0) {
                consistent = false;
            }
        }
        w.push(TraceLine::region_header(
            t,
            name,
            &present,
            consistent,
            b.len() as u64,
            &locations,
        ));
    }

    // 4. per chunk
    let chunks: Vec<ChunkPos> = match chunks {
        Some(c) => c.to_vec(),
        None => present_chunks,
    };
    let mut loader = McRegionChunkLoader::new(store.clone(), dir);
    for c in chunks {
        let Some(tree) = loader.read_chunk_nbt(c.x, c.z).map_err(err)? else {
            w.push(TraceLine::chunk_absent(t, c.x, c.z));
            continue;
        };
        // CompressedStreamTools.read keeps the root name; McRegion writes "".
        w.push(TraceLine::chunk_nbt(t, c.x, c.z, canonical_root("", &tree)));
        if !mask {
            let loaded = loader
                .load_chunk(world, c.x, c.z)
                .map_err(err)?
                .ok_or_else(|| err(format!("chunk ({}, {}) did not load", c.x, c.z)))?;
            let a = mc_sim::golden::ChunkArrays::of(&loaded.chunk);
            w.push(TraceLine::chunk_digest_at(
                t,
                c.x,
                c.z,
                Stage::Loaded,
                &ChunkArrays {
                    blocks: a.blocks,
                    data: a.data,
                    skylight: a.skylight,
                    blocklight: a.blocklight,
                    heightmap: a.heightmap,
                },
            ));
            discard(world, loaded);
        }
        let tes: &[Tag] = match tree.get("Level").and_then(|l| l.get("TileEntities")) {
            Some(Tag::List { items, .. }) => items,
            _ => &[],
        };
        for (i, te_nbt) in tes.iter().enumerate() {
            let id = match te_nbt.get("id") {
                Some(Tag::String(s)) => s.clone(),
                _ => String::new(),
            };
            match tile_entity::create_and_load_entity(te_nbt, world)
                .map_err(|e| err(format!("tile entity {i}: {e:?}")))?
            {
                None => w.push(TraceLine::tile_entity_unknown(t, i as u32, &id)),
                Some(te) => {
                    let inv = tile_entity::get_size_inventory(&te.kind).map(|size| {
                        let items: Vec<TileItem> = (0..size)
                            .filter_map(|s| {
                                tile_entity::get_stack_in_slot(&te.kind, s).map(|st| TileItem {
                                    slot: s,
                                    id: st.item_id,
                                    count: st.stack_size,
                                    damage: st.item_damage,
                                })
                            })
                            .collect();
                        (size as u32, items)
                    });
                    w.push(TraceLine::tile_entity(
                        t,
                        i as u32,
                        &id,
                        te.kind.java_class(),
                        [te.x_coord, te.y_coord, te.z_coord],
                        inv.as_ref().map(|(s, items)| (*s, items.as_slice())),
                    ));
                }
            }
        }
    }
    Ok(())
}
