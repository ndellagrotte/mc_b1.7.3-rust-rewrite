//! Tier S (WP-1.9b, PLAN_v4 §4.3 "S", D5): saves.
//!
//! 1. The fixtures: `save/testworld` (`world_read`, through
//!    `mc_golden::world_read_driver`), and every tick fixture with a `save`
//!    op (`save/roundtrip-seed-123`, `save/tile-entities`, through
//!    `mc_golden::tick_driver`); [`every_tier_s_fixture_has_a_test`] keeps
//!    the list complete. `tick/dungeon-chest-save` (WP-1.7) is a tick-area
//!    fixture with a `save` op, so its test is
//!    `tests/golden_tick.rs::tick_dungeon_chest_save` (listed in
//!    [`COVERED_IN_GOLDEN_TICK`]; one test per fixture, so two test binaries
//!    never write the same `actual.jsonl`).
//! 2. PLAN_v4 §4.4 item 4: `testWorld` opened through `SaveOldDir` on a
//!    `Storage`, chunk (0, −4) digests equal to the oracle's `loaded` line,
//!    the chest at (9, 64, −61) holds 2 planks and 4 sticks.
//! 3. Write / re-read: a world saved through `SaveOldDir`, opened again in a
//!    fresh `World`, compared by chunk arrays and parsed NBT trees (tile
//!    entities, entities, the player, the re-saved chunks).
//! 4. Cross-read both ways with the vanilla loaders (`harness.WorldReadDir`),
//!    when the oracle is built (`tools/oracle/build.sh`); skipped otherwise.
//! 5. Import refusal (D5): a `testWorld` copy with block id 200 in one chunk,
//!    another with item id 3000 in the chest; a clean copy passes.
//! 6. `level.dat` shapes: `WorldInfo` and the player compound against
//!    `nbt/level-dat-shape` and `nbt/testworld-level-dat`.
//! 7. Save-handler behaviour no fixture reaches: the session lock, the
//!    `level.dat` rename dance, the save list, entity classes the port
//!    cannot construct yet (kept as compounds), the relocation bug.

use std::collections::BTreeMap;
use std::path::Path;

use mc_golden::world_read_driver::{
    self, WORLD_DIR, canonical_root, emit_tier_s, harness_store, load_save_dir, open_world,
};
use mc_golden::{
    ChunkArrays, Stage, TraceLine, TraceWriter, all_fixture_dirs, compare, fixture_dir,
    parse_jsonl, read_sidecar, target_dir, tick_driver, workspace_root,
};
use mc_nbt::{Tag, TagType};
use mc_sim::entity::item::new_entity_item;
use mc_sim::entity::player::new_entity_player_sp;
use mc_sim::entity::{Entity, falling_sand::new_entity_falling_sand, nbt as entity_nbt};
use mc_sim::item::ItemStack;
use mc_sim::save::import::{ImportRefusal, Location, validate};
use mc_sim::save::{
    ChunkLoader, FixedClock, McRegionChunkLoader, MemoryStorage, SaveConverterMcRegion, SaveError,
    SaveFormat, SaveHandler, SaveOldDir, SaveStore, SharedSaveStore, Storage,
};
use mc_sim::tile_entity;
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::info::WorldInfo;
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::tile_entity::TileEntityKindData;
use mc_sim::world::{World, WorldEnvironment};
use serde_json::Value;

const ENV: WorldEnvironment = WorldEnvironment {
    rand_seed: 0,
    field_9437_g: 0,
    lock_timestamp: 0,
};

// ---- 1. fixtures -------------------------------------------------------------------

fn check_world_read(name: &str) {
    let dir = fixture_dir("save", name);
    if let Err(e) = mc_golden::run(&dir, world_read_driver::drive_in(dir.clone())) {
        panic!("save/{name}: {e}");
    }
}

fn check_tick(area: &str, name: &str) {
    if let Err(e) = mc_golden::run(fixture_dir(area, name), tick_driver::drive) {
        panic!("{area}/{name}: {e}");
    }
}

/// Every Tier S fixture: the `world_read` ones and the tick fixtures with a
/// `save` op.
const COVERED: &[(&str, &str)] = &[
    ("save", "testworld"),
    ("save", "roundtrip-seed-123"),
    ("save", "tile-entities"),
];

/// Tier S fixtures in the `tick` area: `tests/golden_tick.rs` must list them
/// (its own coverage check), so they are run there and not again here.
const COVERED_IN_GOLDEN_TICK: &[(&str, &str)] = &[("tick", "dungeon-chest-save")];

#[test]
fn save_testworld() {
    check_world_read("testworld");
}

#[test]
fn save_roundtrip_seed_123() {
    check_tick("save", "roundtrip-seed-123");
}

#[test]
// The player compound matches since WP-2.2 (EntityLiving.onUpdate: the
// player falls onto the platform as the oracle's HarnessPlayer does).
fn save_tile_entities() {
    check_tick("save", "tile-entities");
}

fn jsonl(path: &Path) -> Vec<Value> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn strip_player(line: &mut Value) {
    if line["e"] == "level_dat"
        && let Some(data) = line["root"]["value"]["Data"]["value"].as_object_mut()
    {
        assert!(data.remove("Player").is_some(), "level_dat has a Player");
    }
}

/// `save/tile-entities` with the player compound (a gap until WP-2.2) taken
/// out of both traces: everything else must match exactly. Kept beside the
/// full fixture test, which now passes, as the narrower check.
#[test]
fn save_tile_entities_matches_except_the_player() {
    let dir = fixture_dir("save", "tile-entities");
    // Runs the driver (writes actual.jsonl).
    let _ = mc_golden::run(&dir, tick_driver::drive);
    let mut expected = jsonl(&dir.join("expected.jsonl"));
    let mut actual = jsonl(&target_dir().join("golden/save/tile-entities/actual.jsonl"));
    assert_eq!(expected.len(), actual.len());
    for (e, a) in expected.iter_mut().zip(actual.iter_mut()) {
        strip_player(e);
        strip_player(a);
        if e["e"] == "region_header" {
            e.as_object_mut().unwrap().remove("dbg");
            a.as_object_mut().unwrap().remove("dbg");
        }
        assert_eq!(e, a);
    }
}

#[test]
fn every_tier_s_fixture_has_a_test() {
    for dir in all_fixture_dirs().unwrap() {
        let text = std::fs::read_to_string(dir.join("scenario.json")).unwrap();
        let sc: Value = serde_json::from_str(&text).unwrap();
        let has_save = sc["ops"]
            .as_array()
            .is_some_and(|ops| ops.iter().any(|o| o["op"] == "save"));
        if sc["kind"] == "world_read" || has_save {
            let area = dir.parent().unwrap().file_name().unwrap().to_str().unwrap();
            let name = dir.file_name().unwrap().to_str().unwrap();
            assert!(
                COVERED.contains(&(area, name)) || COVERED_IN_GOLDEN_TICK.contains(&(area, name)),
                "{area}/{name} has no test in golden_save.rs"
            );
        }
    }
}

// ---- helpers -----------------------------------------------------------------------------

fn expected_line(area: &str, name: &str, e: &str, pick: impl Fn(&Value) -> bool) -> Value {
    jsonl(&fixture_dir(area, name).join("expected.jsonl"))
        .into_iter()
        .find(|v| v["e"] == e && pick(v))
        .unwrap_or_else(|| panic!("no {e} line in {area}/{name}"))
}

fn testworld_store() -> SharedSaveStore {
    harness_store(load_save_dir(&fixture_dir("save", "testworld"), "world").unwrap())
}

fn store_over(storage: MemoryStorage) -> SharedSaveStore {
    SaveStore::shared(Box::new(storage), Box::new(FixedClock(1_700_000_000_000)))
}

fn digest_line(c: &mc_sim::world::chunk::Chunk) -> Value {
    let a = mc_sim::golden::ChunkArrays::of(c);
    let line = TraceLine::chunk_digest_at(
        0,
        c.x_position,
        c.z_position,
        Stage::Loaded,
        &ChunkArrays {
            blocks: a.blocks,
            data: a.data,
            skylight: a.skylight,
            blocklight: a.blocklight,
            heightmap: a.heightmap,
        },
    );
    Value::Object(line.fields().clone())
}

fn chest_slots(w: &World, x: i32, y: i32, z: i32) -> Vec<(usize, ItemStack)> {
    let r = w
        .loaded_chunk(x >> 4, z >> 4)
        .and_then(|c| {
            c.chunk_tile_entity_map
                .get(&mc_core::java_hash::ChunkPosition::new(x & 15, y, z & 15))
                .copied()
        })
        .expect("a tile entity at the position");
    match &w.tile_entities[&r].kind {
        TileEntityKindData::Chest(c) => c
            .chest_contents
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.map(|s| (i, s)))
            .collect(),
        other => panic!("not a chest: {other:?}"),
    }
}

// ---- 2. PLAN_v4 §4.4 item 4 --------------------------------------------------------------

/// `testWorld` loads in the port with chunk digests equal to what the oracle
/// reads from the same files, and the chest at 9,64,−61 holds 2 planks and
/// 4 sticks.
#[test]
fn testworld_chunk_digest_and_chest() {
    let store = testworld_store();
    let mut w = open_world(&store).unwrap();
    assert_eq!(w.world_info.level_name, "harness"); // renamed by the loading constructor
    assert_eq!(w.world_info.random_seed, 123);
    // Through the world's own ChunkProvider and McRegionChunkLoader.
    w.get_chunk_from_chunk_coords(0, -4);
    let c = w.loaded_chunk(0, -4).unwrap();
    assert!(c.is_terrain_populated);
    let ours = digest_line(c);
    let oracle = expected_line("save", "testworld", "chunk_digest", |v| {
        v["stage"] == "loaded" && v["x"] == 0 && v["z"] == -4
    });
    for k in ["blocks", "data", "skylight", "blocklight", "heightmap"] {
        assert_eq!(ours[k], oracle[k], "chunk (0,-4) {k}");
    }
    let slots = chest_slots(&w, 9, 64, -61);
    assert_eq!(
        slots,
        vec![(0, ItemStack::new(5, 2, 0)), (1, ItemStack::new(280, 4, 0))],
        "2 planks in slot 0 and 4 sticks in slot 1"
    );
    // The chest joined loadedTileEntityList through onChunkLoad.
    assert_eq!(w.loaded_tile_entity_list.len(), 1);
    assert_eq!(w.chunk_load_errors(), 0);
}

/// `World.spawnPlayerWithLoadedChunks` restores the player from testWorld's
/// `level.dat` once (World.java:256-276).
#[test]
fn testworld_player_restored_from_level_dat() {
    let store = testworld_store();
    let mut w = open_world(&store).unwrap();
    let Some(Tag::Compound(tag)) = w.world_info.player_tag.clone() else {
        panic!("testWorld's level.dat has a Player");
    };
    let Tag::List { items: pos, .. } = &tag["Pos"] else {
        panic!("Pos");
    };
    let p = new_entity_player_sp(&mut w);
    assert!(w.spawn_player_with_loaded_chunks(p));
    assert_eq!(w.world_info.player_tag, None, "applied once");
    let pid = *w.player_entities.first().unwrap();
    let e = w.entity(pid).unwrap();
    assert_eq!(Tag::Double(e.base.pos_x), pos[0]);
    assert_eq!(Tag::Double(e.base.pos_z), pos[2]);
    assert_eq!(e.base.fire, -20);
    assert!(e.base.on_ground);
}

// ---- 3. write / re-read ------------------------------------------------------------------

/// A world on an empty generator with a `SaveOldDir` on `dir` of `store`:
/// the 5×5 chunks around the origin, a stone platform in chunk (0, 0), a
/// chest (2 planks, 4 sticks in slot 5), a lit furnace's fuel, a sign, an
/// item entity, a falling-sand entity and a player; world time 100.
fn build_saved_world(store: &SharedSaveStore, dir: &str) -> World {
    let handler = SaveOldDir::new(store.clone(), "", dir).unwrap();
    let mut w = World::new(
        Box::new(handler),
        "written",
        WorldProvider::new(Dimension::Surface),
        123,
        ChunkGenerator::Empty(EmptyChunkProvider),
        ENV,
    );
    w.make_deterministic(123);
    for dz in -2..=2 {
        for dx in -2..=2 {
            w.get_chunk_from_chunk_coords(dx, dz);
            while w.updating_lighting() {}
        }
    }
    for x in 0..16 {
        for z in 0..16 {
            w.set_block_and_metadata(x, 60, z, 1, 0);
        }
    }
    w.set_block_and_metadata_with_notify(3, 61, 3, 54, 0);
    let chest = w.get_block_tile_entity(3, 61, 3).unwrap();
    tile_entity::set_inventory_slot_contents(&mut w, chest, 0, Some(ItemStack::new(5, 2, 0)));
    tile_entity::set_inventory_slot_contents(&mut w, chest, 5, Some(ItemStack::new(280, 4, 0)));
    w.set_block_and_metadata_with_notify(5, 61, 3, 61, 0);
    let furnace = w.get_block_tile_entity(5, 61, 3).unwrap();
    tile_entity::set_inventory_slot_contents(&mut w, furnace, 1, Some(ItemStack::new(263, 3, 0)));
    w.set_block_and_metadata_with_notify(7, 61, 3, 63, 4);
    let sign = w.get_block_tile_entity(7, 61, 3).unwrap();
    if let TileEntityKindData::Sign(s) = &mut w.tile_entities.get_mut(&sign).unwrap().kind {
        s.sign_text[0] = "write".into();
        s.sign_text[3] = "re-read".into();
    }
    let item = new_entity_item(&mut w, 8.5, 61.0, 8.5, ItemStack::new(264, 2, 0));
    assert!(w.spawn_entity_in_world(item));
    let sand = new_entity_falling_sand(&mut w, 10.5, 70.5, 10.5, 12);
    assert!(w.spawn_entity_in_world(sand));
    let mut p = new_entity_player_sp(&mut w);
    p.base.set_location_and_angles(4.5, 61.0, 9.5, 400.0, 10.0);
    if let Some(pd) = p.kind.player_mut() {
        pd.inventory.main_inventory[0] = Some(ItemStack::new(278, 1, 17));
        pd.inventory.armor_inventory[3] = Some(ItemStack::new(298, 1, 5));
        pd.score = 7;
    }
    assert!(w.spawn_entity_in_world(p));
    w.set_world_time(100);
    w
}

/// The stored `Level` compound of chunk `(x, z)`.
fn stored_chunk(store: &SharedSaveStore, dir: &str, x: i32, z: i32) -> Tag {
    McRegionChunkLoader::new(store.clone(), dir)
        .read_chunk_nbt(x, z)
        .unwrap()
        .unwrap_or_else(|| panic!("chunk ({x},{z}) not saved"))
}

fn entity_trees(w: &World) -> Vec<Tag> {
    w.loaded_entity_list
        .iter()
        .filter_map(|s| s.entity.as_deref())
        .filter_map(|e| entity_nbt::add_entity_id(e, w))
        .map(Tag::Compound)
        .collect()
}

fn tile_entity_trees(w: &World, cx: i32, cz: i32) -> Vec<Tag> {
    let c = w.loaded_chunk(cx, cz).unwrap();
    c.chunk_tile_entity_map
        .values()
        .map(|r| tile_entity::write_to_nbt(&w.tile_entities[r], w))
        .collect()
}

#[test]
fn write_reread_round_trip() {
    let store = store_over(MemoryStorage::default());
    let mut a = build_saved_world(&store, "w");
    a.try_save_world(true).unwrap();
    // level.dat: version 19132 on the world's WorldInfo, the rename dance done.
    assert_eq!(a.world_info.save_version, 19132);
    {
        let mut s = store.borrow_mut();
        assert!(s.storage.read("w/level.dat").unwrap().is_some());
        assert!(s.storage.read("w/level.dat_new").unwrap().is_none());
        assert_eq!(
            s.storage.read("w/session.lock").unwrap().unwrap(),
            1_700_000_000_000i64.to_be_bytes()
        );
    }
    let chunks: Vec<(i32, i32)> = (-2..=2)
        .flat_map(|z| (-2..=2).map(move |x| (x, z)))
        .collect();
    let saved: BTreeMap<(i32, i32), Tag> = chunks
        .iter()
        .map(|&(x, z)| ((x, z), stored_chunk(&store, "w", x, z)))
        .collect();
    let a_entities = entity_trees(&a);
    assert_eq!(
        a_entities.len(),
        2,
        "the item and the sand; the player is not saved in chunks"
    );
    let a_tes = tile_entity_trees(&a, 0, 0);
    assert_eq!(a_tes.len(), 3);
    let player_written = entity_nbt::write_to_nbt(a.entity(a.player_entities[0]).unwrap(), &a);

    // A fresh world on the same files (a new SaveOldDir takes the lock).
    let handler = SaveOldDir::new(store.clone(), "", "w").unwrap();
    let mut b = World::new_from_save(Box::new(handler), "reopened", 0, None, ENV);
    assert!(!b.is_new_world);
    assert_eq!(b.world_info.random_seed, 123);
    assert_eq!(b.world_info.world_time, 100);
    assert_eq!(b.world_info.save_version, 19132);
    b.make_deterministic(123);
    for &(x, z) in &chunks {
        b.get_chunk_from_chunk_coords(x, z);
    }
    assert_eq!(b.chunk_load_errors(), 0);
    for &(x, z) in &chunks {
        let (ca, cb) = (a.loaded_chunk(x, z).unwrap(), b.loaded_chunk(x, z).unwrap());
        assert_eq!(digest_line(ca), digest_line(cb), "chunk ({x},{z}) arrays");
        assert_eq!(ca.is_terrain_populated, cb.is_terrain_populated);
    }
    assert_eq!(tile_entity_trees(&b, 0, 0), a_tes, "tile entities");
    assert_eq!(chest_slots(&b, 3, 61, 3), chest_slots(&a, 3, 61, 3));
    assert_eq!(entity_trees(&b), a_entities, "entities");
    assert_eq!(b.loaded_tile_entity_list.len(), 3);

    // The player compound in level.dat reads back into the same player.
    let tag = b
        .world_info
        .player_tag
        .clone()
        .expect("level.dat has a Player");
    let Tag::Compound(tag) = tag else { panic!() };
    assert_eq!(tag, player_written);
    let mut p = new_entity_player_sp(&mut b);
    entity_nbt::read_from_nbt(&mut p, &tag, &mut b).unwrap();
    // Yaw 400 was written as is; reading applies setRotation's % 360.
    assert_eq!(p.base.rotation_yaw, 40.0);
    let mut expect = tag.clone();
    expect.insert(
        "Rotation".into(),
        Tag::List {
            elem: TagType::Float,
            items: vec![Tag::Float(40.0), Tag::Float(10.0)],
        },
    );
    assert_eq!(entity_nbt::write_to_nbt(&p, &b), expect);
    assert_eq!(p.kind.player().unwrap().score, 7);

    // Re-saving the reopened world writes the same chunk trees.
    for &(x, z) in &chunks {
        b.get_chunk_from_chunk_coords(x, z).set_chunk_modified();
    }
    b.try_save_world(true).unwrap();
    for &(x, z) in &chunks {
        assert_eq!(
            stored_chunk(&store, "w", x, z),
            saved[&(x, z)],
            "chunk ({x},{z}) re-saved"
        );
    }
}

/// `saveChunks(false)` writes at most 24 chunks; `needsSaving` clears.
#[test]
fn quick_save_writes_24_chunks() {
    let store = store_over(MemoryStorage::default());
    let mut a = build_saved_world(&store, "w");
    assert!(!a.quick_save_world(0));
    assert!(a.quick_save_world(1));
    assert!(
        store
            .borrow_mut()
            .storage
            .read("w/level.dat")
            .unwrap()
            .is_some()
    );
}

// ---- 4. cross-read with the vanilla loaders -----------------------------------------------

fn oracle_ready() -> bool {
    let ok = workspace_root()
        .join("build/oracle/harness/WorldReadDir.class")
        .is_file();
    if !ok {
        eprintln!(
            "skipped: build/oracle/harness/WorldReadDir.class missing (run tools/oracle/build.sh)"
        );
    }
    ok
}

/// Runs `harness.WorldReadDir <mode> <parent>`; its JSON lines (vanilla's
/// static initialisers print a few other lines first).
fn world_read_dir(mode: &str, parent: &Path) -> Option<Vec<TraceLine>> {
    let out = std::process::Command::new("sh")
        .arg("-c")
        .arg(
            ". \"$ROOT/tools/oracle/env.sh\" && \
             java $JAVA_MATH_FLAGS -cp \"$ROOT/build/oracle:$CP\" harness.WorldReadDir \"$1\" \"$2\"",
        )
        .arg("sh")
        .arg(mode)
        .arg(parent)
        .env("ROOT", workspace_root())
        .output();
    let out = match out {
        Ok(o) => o,
        Err(e) => {
            eprintln!("skipped: cannot run sh/java: {e}");
            return None;
        }
    };
    assert!(
        out.status.success(),
        "WorldReadDir {mode} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text: String = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .filter(|l| l.starts_with('{'))
        .map(|l| format!("{l}\n"))
        .collect();
    Some(parse_jsonl(&text).unwrap())
}

/// Host directory for the JVM (test-only; D9 binds the simulation, not its
/// tests).
#[allow(clippy::disallowed_methods)]
fn write_host_tree(storage: &MemoryStorage, prefix: &str, parent: &Path) {
    let _ = std::fs::remove_dir_all(parent);
    for (path, bytes) in &storage.blobs {
        if let Some(rel) = path.strip_prefix(prefix) {
            let p = parent.join("world").join(rel.trim_start_matches('/'));
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, bytes).unwrap();
        }
    }
}

/// The Rust reading of `world/` in `storage` (as `world_read` emits it).
fn rust_world_read(storage: MemoryStorage) -> Vec<TraceLine> {
    let store = harness_store(storage);
    let mut w = open_world(&store).unwrap();
    let mut out = TraceWriter::new();
    emit_tier_s(0, &store, WORLD_DIR, None, false, &mut w, &mut out).unwrap();
    out.lines().to_vec()
}

/// Rust writes, vanilla reads: the vanilla loaders' view of a Rust-written
/// save equals the Rust loaders' view of it, line for line (level.dat tree,
/// WorldInfo, region headers, chunk trees, loaded-chunk digests, tile
/// entities).
#[test]
fn cross_read_rust_written_world_by_vanilla() {
    if !oracle_ready() {
        return;
    }
    let store = store_over(MemoryStorage::default());
    let mut a = build_saved_world(&store, WORLD_DIR);
    a.try_save_world(true).unwrap();
    drop(a);
    let storage = snapshot(&store);
    let parent = target_dir().join("cross-read/rust-written");
    write_host_tree(&storage, WORLD_DIR, &parent);
    let Some(vanilla) = world_read_dir("read", &parent) else {
        return;
    };
    let ours = rust_world_read(storage);
    assert!(vanilla.len() > 25);
    assert_eq!(vanilla.iter().filter(|l| l.e() == "tile_entity").count(), 3);
    if let Err(m) = compare(&vanilla, &ours) {
        panic!("vanilla and Rust read the Rust-written save differently:\n{m}");
    }
}

/// Vanilla writes, Rust reads: a world saved by the vanilla code
/// (`WorldReadDir write`: chest, furnace, sign, item and falling-sand
/// entities) read by both loaders gives the same lines.
#[test]
fn cross_read_vanilla_written_world_by_rust() {
    if !oracle_ready() {
        return;
    }
    let parent = target_dir().join("cross-read/vanilla-written");
    #[allow(clippy::disallowed_methods)]
    let _ = std::fs::remove_dir_all(&parent);
    if world_read_dir("write", &parent).is_none() {
        return;
    }
    let Some(vanilla) = world_read_dir("read", &parent) else {
        return;
    };
    let storage = load_save_dir(&parent, "world").unwrap();
    // The vanilla-written chunk (0, 0) holds two entities the Rust loader builds.
    let store = harness_store(storage.clone());
    let mut w = open_world(&store).unwrap();
    w.get_chunk_from_chunk_coords(0, 0);
    let kinds: Vec<&str> = w
        .loaded_entity_list
        .iter()
        .map(|s| s.entity.as_ref().unwrap().kind.java_class())
        .collect();
    assert_eq!(kinds, ["EntityItem", "EntityFallingSand"]);
    let ours = rust_world_read(storage);
    assert_eq!(vanilla.iter().filter(|l| l.e() == "tile_entity").count(), 3);
    assert_eq!(
        vanilla.iter().filter(|l| l.e() == "chunk_digest").count(),
        25
    );
    if let Err(m) = compare(&vanilla, &ours) {
        panic!("vanilla and Rust read the vanilla-written save differently:\n{m}");
    }
}

// ---- 5. import refusal (D5) ---------------------------------------------------------------

fn testworld_storage() -> MemoryStorage {
    load_save_dir(&fixture_dir("save", "testworld"), "world").unwrap()
}

/// Rewrites chunk `(x, z)` of `world/` with `edit` applied to its tree.
fn patch_chunk(
    storage: MemoryStorage,
    x: i32,
    z: i32,
    edit: impl FnOnce(&mut Tag),
) -> MemoryStorage {
    let store = store_over(storage);
    let mut tree = stored_chunk(&store, WORLD_DIR, x, z);
    edit(&mut tree);
    let bytes = mc_nbt::write_named("", &tree).unwrap();
    {
        let mut s = store.borrow_mut();
        let s = &mut *s;
        s.region_cache
            .write_chunk(
                s.storage.as_mut(),
                WORLD_DIR,
                x,
                z,
                &bytes,
                1_700_000_000_000,
            )
            .unwrap();
    }
    snapshot(&store)
}

/// A copy of every blob of the store's storage.
fn snapshot(store: &SharedSaveStore) -> MemoryStorage {
    fn walk(s: &mut dyn Storage, dir: &str, out: &mut MemoryStorage) {
        for name in s.list(dir).unwrap() {
            let p = if dir.is_empty() {
                name.clone()
            } else {
                format!("{dir}/{name}")
            };
            match s.read(&p).unwrap() {
                Some(b) => out.write(&p, &b).unwrap(),
                None => walk(s, &p, out),
            }
        }
    }
    let mut out = MemoryStorage::default();
    walk(store.borrow_mut().storage.as_mut(), "", &mut out);
    out
}

fn level_mut(tree: &mut Tag) -> &mut BTreeMap<String, Tag> {
    tree.as_compound_mut()
        .unwrap()
        .get_mut("Level")
        .unwrap()
        .as_compound_mut()
        .unwrap()
}

#[test]
fn import_accepts_the_clean_testworld() {
    let mut s = testworld_storage();
    let before = s.clone();
    assert_eq!(validate(&mut s, WORLD_DIR), Ok(()));
    assert_eq!(s, before, "validation only reads");
}

#[test]
fn import_refuses_an_unknown_block_id() {
    let s = patch_chunk(testworld_storage(), 3, -5, |t| {
        let Some(Tag::ByteArray(b)) = level_mut(t).get_mut("Blocks") else {
            panic!()
        };
        // x 2, y 70, z 9
        b[2 << 11 | 9 << 7 | 70] = 200;
    });
    let mut s2 = s.clone();
    let r = validate(&mut s2, WORLD_DIR);
    assert_eq!(s2, s, "validation only reads");
    match r {
        Err(ImportRefusal::UnknownBlock {
            id: 200,
            location:
                Location::Chunk {
                    dimension: 0,
                    x: 3,
                    z: -5,
                },
            ref detail,
        }) => assert_eq!(detail, "Blocks at local x 2, y 70, z 9"),
        other => panic!("{other:?}"),
    }
    let msg = r.unwrap_err().to_string();
    assert!(
        msg.contains("200") && msg.contains("chunk (3, -5)"),
        "{msg}"
    );
}

#[test]
fn import_refuses_an_unknown_item_id() {
    let s = patch_chunk(testworld_storage(), 0, -4, |t| {
        let Some(Tag::List { items, .. }) = level_mut(t).get_mut("TileEntities") else {
            panic!()
        };
        let chest = items[0].as_compound_mut().unwrap();
        let Some(Tag::List { items, .. }) = chest.get_mut("Items") else {
            panic!()
        };
        items[1]
            .as_compound_mut()
            .unwrap()
            .insert("id".into(), Tag::Short(3000));
    });
    let mut s2 = s.clone();
    match validate(&mut s2, WORLD_DIR) {
        Err(ImportRefusal::UnknownItem {
            id: 3000,
            location: Location::Chunk { x: 0, z: -4, .. },
            detail,
        }) => assert_eq!(detail, "TileEntities[0] Chest slot 1"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn import_refuses_a_bad_player_item_and_a_missing_level_dat() {
    // level.dat: the player's inventory, item id 31999 (no item).
    let mut s = testworld_storage();
    let (name, mut root) = mc_nbt::read_gzip(&s.read("world/level.dat").unwrap().unwrap()).unwrap();
    let data = root.as_compound_mut().unwrap().get_mut("Data").unwrap();
    let player = data.as_compound_mut().unwrap().get_mut("Player").unwrap();
    let mut st = BTreeMap::new();
    st.insert("Slot".to_string(), Tag::Byte(3));
    st.insert("id".to_string(), Tag::Short(31999));
    st.insert("Count".to_string(), Tag::Byte(1));
    st.insert("Damage".to_string(), Tag::Short(0));
    player.as_compound_mut().unwrap().insert(
        "Inventory".into(),
        Tag::List {
            elem: TagType::Compound,
            items: vec![Tag::Compound(st)],
        },
    );
    s.write(
        "world/level.dat",
        &mc_nbt::write_gzip(&name, &root).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        validate(&mut s, WORLD_DIR),
        Err(ImportRefusal::UnknownItem {
            id: 31999,
            location: Location::LevelDat,
            ..
        })
    ));
    let mut empty = MemoryStorage::default();
    assert!(matches!(
        validate(&mut empty, WORLD_DIR),
        Err(ImportRefusal::Unreadable { .. })
    ));
}

/// `level.dat` present but unreadable: the scan uses `level.dat_old`, as
/// `SaveHandler.loadWorldInfo` does (SaveHandler.java:81-107).
#[test]
fn import_falls_back_to_level_dat_old() {
    let mut s = testworld_storage();
    let good = s.read("world/level.dat").unwrap().unwrap();
    s.write("world/level.dat_old", &good).unwrap();
    s.write("world/level.dat", b"not gzip").unwrap();
    assert_eq!(validate(&mut s.clone(), WORLD_DIR), Ok(()));
    // The fallback file's player is the one checked.
    let (name, mut root) = mc_nbt::read_gzip(&good).unwrap();
    let data = root.as_compound_mut().unwrap().get_mut("Data").unwrap();
    let player = data.as_compound_mut().unwrap().get_mut("Player").unwrap();
    let mut st = BTreeMap::new();
    st.insert("Slot".to_string(), Tag::Byte(0));
    st.insert("id".to_string(), Tag::Short(3000));
    st.insert("Count".to_string(), Tag::Byte(1));
    st.insert("Damage".to_string(), Tag::Short(0));
    player.as_compound_mut().unwrap().insert(
        "Inventory".into(),
        Tag::List {
            elem: TagType::Compound,
            items: vec![Tag::Compound(st)],
        },
    );
    let mut bad_old = s.clone();
    bad_old
        .write(
            "world/level.dat_old",
            &mc_nbt::write_gzip(&name, &root).unwrap(),
        )
        .unwrap();
    assert!(matches!(
        validate(&mut bad_old, WORLD_DIR),
        Err(ImportRefusal::UnknownItem {
            id: 3000,
            location: Location::LevelDat,
            ..
        })
    ));
    // Neither file reads: unreadable.
    s.delete("world/level.dat_old").unwrap();
    assert!(matches!(
        validate(&mut s, WORLD_DIR),
        Err(ImportRefusal::Unreadable { .. })
    ));
}

// ---- 6. level.dat shapes -------------------------------------------------------------------

fn tree_from_line(line: &Value) -> Value {
    line["root"].clone()
}

/// `nbt/level-dat-shape`'s `Data` read into a `WorldInfo` and written back
/// (`LastPlayed` from the same clock value) is the same tree, and its
/// `Player` compound reads into a player that writes it back with the yaw
/// `% 360` (Entity.java:824, :830, setRotation).
#[test]
fn world_info_level_dat_shape() {
    let oracle = tree_from_line(&expected_line("nbt", "level-dat-shape", "nbt_tree", |_| {
        true
    }));
    let root = tag_from_canonical(&oracle);
    let Tag::Compound(root) = root else { panic!() };
    let Some(Tag::Compound(data)) = root.get("Data") else {
        panic!()
    };
    let info = WorldInfo::from_nbt(data).unwrap();
    assert_eq!(info.random_seed, -4172144991579068934);
    assert_eq!(info.dimension, 0);
    assert_eq!(info.save_version, 19132);
    let Some(Tag::Long(last)) = data.get("LastPlayed") else {
        panic!()
    };
    let back = info.get_nbt_tag_compound(*last);
    assert_eq!(&back, data);
    let mut wrapped = BTreeMap::new();
    wrapped.insert("Data".to_string(), Tag::Compound(back));
    assert_eq!(canonical_root("", &Tag::Compound(wrapped)), oracle);
    check_player_round_trip(info.player_tag.as_ref().unwrap());
}

/// The same for the owner's `testWorld/level.dat` (`nbt/testworld-level-dat`),
/// read through `SaveOldDir.loadWorldInfo`.
#[test]
fn world_info_testworld_level_dat() {
    let bytes = read_sidecar(fixture_dir("nbt", "testworld-level-dat"), "level.dat").unwrap();
    let mut storage = MemoryStorage::default();
    storage.write("w/level.dat", &bytes).unwrap();
    let mut h = SaveOldDir::new(store_over(storage), "", "w").unwrap();
    let info = h.load_world_info().unwrap();
    assert_eq!(
        (
            info.random_seed,
            info.spawn_x,
            info.spawn_y,
            info.spawn_z,
            info.world_time
        ),
        (123, 21, 64, -64, 1751)
    );
    let oracle = tree_from_line(&expected_line(
        "nbt",
        "testworld-level-dat",
        "nbt_tree",
        |_| true,
    ));
    let back = info.get_nbt_tag_compound(info.last_time_played);
    let mut wrapped = BTreeMap::new();
    wrapped.insert("Data".to_string(), Tag::Compound(back));
    assert_eq!(canonical_root("", &Tag::Compound(wrapped)), oracle);
    check_player_round_trip(info.player_tag.as_ref().unwrap());
}

fn check_player_round_trip(tag: &Tag) {
    let Tag::Compound(c) = tag else { panic!() };
    let mut w = empty_world();
    let mut p = new_entity_player_sp(&mut w);
    entity_nbt::read_from_nbt(&mut p, c, &mut w).unwrap();
    let mut expect = c.clone();
    if let Some(Tag::List { items, .. }) = expect.get_mut("Rotation")
        && let Tag::Float(y) = &mut items[0]
    {
        *y %= 360.0;
    }
    assert_eq!(entity_nbt::write_to_nbt(&p, &w), expect);
}

fn empty_world() -> World {
    let mut w = World::new(
        Box::new(mc_sim::save::NoSave),
        "harness",
        WorldProvider::new(Dimension::Surface),
        0,
        ChunkGenerator::Empty(EmptyChunkProvider),
        ENV,
    );
    w.make_deterministic(0);
    w
}

/// The inverse of `canonical_root` for the shapes `level.dat` uses.
fn tag_from_canonical(node: &Value) -> Tag {
    let ty = node["type"].as_str().unwrap();
    value_to_tag(ty, &node["value"])
}

fn value_to_tag(ty: &str, v: &Value) -> Tag {
    let bits =
        |v: &Value| u64::from_str_radix(v.as_str().unwrap().trim_start_matches("0x"), 16).unwrap();
    match ty {
        "byte" => Tag::Byte(v.as_i64().unwrap() as i8),
        "short" => Tag::Short(v.as_i64().unwrap() as i16),
        "int" => Tag::Int(v.as_i64().unwrap() as i32),
        "long" => Tag::Long(v.as_i64().unwrap()),
        "float" => Tag::Float(f32::from_bits(bits(v) as u32)),
        "double" => Tag::Double(f64::from_bits(bits(v))),
        "string" => Tag::String(v.as_str().unwrap().to_string()),
        "byte_array" => Tag::ByteArray(mc_golden::parse_hex_bytes(v.as_str().unwrap()).unwrap()),
        "list" => {
            let elem = v["elem"].as_str().unwrap();
            let items = v["items"]
                .as_array()
                .unwrap()
                .iter()
                .map(|i| value_to_tag(elem, i))
                .collect();
            Tag::List {
                elem: type_from_name(elem),
                items,
            }
        }
        "compound" => Tag::Compound(
            v.as_object()
                .unwrap()
                .iter()
                .map(|(k, n)| (k.clone(), tag_from_canonical(n)))
                .collect(),
        ),
        other => panic!("type {other}"),
    }
}

fn type_from_name(n: &str) -> TagType {
    match n {
        "byte" => TagType::Byte,
        "short" => TagType::Short,
        "int" => TagType::Int,
        "long" => TagType::Long,
        "float" => TagType::Float,
        "double" => TagType::Double,
        "byte_array" => TagType::ByteArray,
        "string" => TagType::String,
        "list" => TagType::List,
        "compound" => TagType::Compound,
        other => panic!("type {other}"),
    }
}

// ---- 7. save-handler behaviour -------------------------------------------------------------

#[test]
fn session_lock_taken_by_another_handler() {
    let store = SaveStore::shared(
        Box::new(MemoryStorage::default()),
        Box::new(FixedClock(1000)),
    );
    let mut first = SaveOldDir::new(store.clone(), "saves", "w").unwrap();
    assert_eq!(first.check_session_lock(), Ok(()));
    // Another process (here: a handler created at another time) opens it.
    store.borrow_mut().clock = Box::new(FixedClock(2000));
    let mut second = SaveOldDir::new(store.clone(), "saves", "w").unwrap();
    assert_eq!(second.check_session_lock(), Ok(()));
    assert_eq!(
        first.check_session_lock(),
        Err(SaveError::Minecraft(
            "The save is being accessed from another location, aborting".into()
        ))
    );
    store
        .borrow_mut()
        .storage
        .delete("saves/w/session.lock")
        .unwrap();
    assert_eq!(
        second.check_session_lock(),
        Err(SaveError::Minecraft(
            "Failed to check session lock, aborting".into()
        ))
    );
}

#[test]
fn save_stops_at_a_stolen_lock() {
    let store = store_over(MemoryStorage::default());
    let mut a = build_saved_world(&store, "w");
    store.borrow_mut().clock = Box::new(FixedClock(5));
    let _thief = SaveOldDir::new(store.clone(), "", "w").unwrap();
    assert!(matches!(
        a.try_save_world(true),
        Err(SaveError::Minecraft(_))
    ));
    assert!(
        store
            .borrow_mut()
            .storage
            .read("w/level.dat")
            .unwrap()
            .is_none()
    );
    assert!(!a.quick_save_world(0));
}

/// The stolen lock in `World.tick`: the `MinecraftException` leaves the tick
/// at the autosave, before `setWorldTime` and the block ticks
/// (World.java:1772-1779), and reaches the client through
/// `session_lock_error` (Minecraft.java:540-544).
#[test]
fn tick_stops_at_a_stolen_lock() {
    let store = store_over(MemoryStorage::default());
    let mut held = build_saved_world(&store, "held");
    held.set_world_time(119); // 120 % autosavePeriod (40) == 0
    held.tick();
    assert_eq!(held.session_lock_error, None);
    assert_eq!(held.world_info.world_time, 120);

    let mut a = build_saved_world(&store, "w");
    store.borrow_mut().clock = Box::new(FixedClock(5));
    let _thief = SaveOldDir::new(store.clone(), "", "w").unwrap();
    a.set_world_time(119);
    a.tick();
    assert!(matches!(
        a.session_lock_error,
        Some(SaveError::Minecraft(_))
    ));
    assert_eq!(a.world_info.world_time, 119, "setWorldTime is skipped");

    let mut b = build_saved_world(&store, "w");
    store.borrow_mut().clock = Box::new(FixedClock(6));
    let _thief = SaveOldDir::new(store.clone(), "", "w").unwrap();
    b.save_world(true);
    assert!(matches!(
        b.session_lock_error,
        Some(SaveError::Minecraft(_))
    ));
}

/// `chunkTileEntityMap.put` replaces (Chunk.java:434-446) and a chunk that
/// fails to load is dropped: neither leaves a tile entity in
/// `World::tile_entities`.
#[test]
fn replaced_and_abandoned_tile_entities_leave_the_world() {
    use mc_sim::save::chunk_loader::{discard, load_chunk_into_world_from_compound};
    let store = store_over(MemoryStorage::default());
    let mut a = build_saved_world(&store, "w");
    a.try_save_world(true).unwrap();
    let tree = stored_chunk(&store, "w", 0, 0);
    let mut level = tree.as_compound().unwrap()["Level"]
        .as_compound()
        .unwrap()
        .clone();
    let Some(Tag::List { items, .. }) = level.get_mut("TileEntities") else {
        panic!("no TileEntities");
    };
    assert_eq!(items.len(), 3);
    let chest = items
        .iter()
        .find(|t| t.as_compound().unwrap()["id"] == Tag::String("Chest".into()))
        .unwrap()
        .clone();
    items.push(chest.clone());
    let before = a.tile_entities.len();
    let l = load_chunk_into_world_from_compound(&mut a, &level).unwrap();
    assert_eq!(l.chunk.chunk_tile_entity_map.len(), 3);
    assert_eq!(
        a.tile_entities.len(),
        before + 3,
        "the replaced chest is freed"
    );
    discard(&mut a, l);
    assert_eq!(a.tile_entities.len(), before);

    // A later tile entity outside the arrays: ArrayIndexOutOfBoundsException.
    let mut far = chest.as_compound().unwrap().clone();
    far.insert("x".into(), Tag::Int(1000));
    let Some(Tag::List { items, .. }) = level.get_mut("TileEntities") else {
        unreachable!()
    };
    items.push(Tag::Compound(far));
    assert!(load_chunk_into_world_from_compound(&mut a, &level).is_err());
    assert_eq!(
        a.tile_entities.len(),
        before,
        "the dropped chunk's are freed"
    );
}

#[test]
fn level_dat_rename_dance_and_fallback() {
    let store = store_over(MemoryStorage::default());
    let mut h = SaveOldDir::new(store.clone(), "", "w").unwrap();
    let mut info = WorldInfo::new(42, "first");
    h.save_world_info_and_player(&mut info, None).unwrap();
    assert_eq!(info.save_version, 19132);
    info.level_name = "second".into();
    h.save_world_info(&info).unwrap();
    {
        let mut s = store.borrow_mut();
        let names = s.storage.list("w").unwrap();
        assert_eq!(names, ["level.dat", "level.dat_old", "session.lock"]);
    }
    assert_eq!(h.load_world_info().unwrap().level_name, "second");
    // A corrupt level.dat falls back to level.dat_old (SaveHandler.java:95-104).
    store
        .borrow_mut()
        .storage
        .write("w/level.dat", b"not gzip")
        .unwrap();
    assert_eq!(h.load_world_info().unwrap().level_name, "first");
    store
        .borrow_mut()
        .storage
        .delete("w/level.dat_old")
        .unwrap();
    assert_eq!(h.load_world_info(), None);
}

#[test]
fn save_list_rename_delete() {
    let store = store_over(MemoryStorage::default());
    let mut fmt = SaveConverterMcRegion::new(store.clone(), "saves");
    let mut a = fmt.get_save_old_dir("alpha").unwrap();
    let mut info = WorldInfo::new(1, "Alpha");
    a.save_world_info_and_player(&mut info, None).unwrap();
    let mut b = fmt.get_save_old_dir("beta").unwrap();
    let mut info = WorldInfo::new(2, "");
    info.save_version = 0;
    b.save_world_info(&info).unwrap();
    let mut list = fmt.get_save_list();
    list.sort();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].file_name, "alpha");
    assert_eq!(list[0].display_name, "Alpha");
    assert!(!list[0].requires_conversion);
    assert_eq!(list[1].display_name, "beta"); // empty LevelName -> directory name
    assert!(list[1].requires_conversion);
    assert!(fmt.is_old_map_format("beta"));
    assert!(!fmt.is_old_map_format("alpha"));
    assert_eq!(fmt.get_world_format(), "Scaevolus' McRegion");
    fmt.rename_world("alpha", "Renamed");
    assert_eq!(fmt.get_world_info("alpha").unwrap().level_name, "Renamed");
    fmt.delete_world_directory("beta");
    assert_eq!(fmt.get_save_list().len(), 1);
    assert!(matches!(
        fmt.convert_map_format("beta"),
        Err(SaveError::Deferred(_))
    ));
}

/// A stored entity of a class the port cannot construct (the first living
/// `EntityList` class without a constructor yet: the cow was until WP-2.3):
/// its id and `EntityLiving`'s `Math.random()` draws are consumed, the gap
/// recorded, and a re-save writes the compound back. Vacuous once every
/// mob is constructible.
#[test]
fn unconstructible_entities_are_kept() {
    let Some(name) = mc_sim::save::entity_list::MAPPINGS
        .iter()
        .filter(|m| m.concrete && m.living)
        .map(|m| m.name)
        .find(|n| {
            mc_sim::save::entity_list::create_entity_in_world(n, &mut empty_world()).is_none()
        })
    else {
        return;
    };
    let store = store_over(MemoryStorage::default());
    let mut a = build_saved_world(&store, "w");
    a.try_save_world(true).unwrap();
    drop(a);
    let mut cow = BTreeMap::new();
    cow.insert("id".to_string(), Tag::String(name.into()));
    cow.insert(
        "Pos".to_string(),
        Tag::List {
            elem: TagType::Double,
            items: vec![Tag::Double(1.5), Tag::Double(61.0), Tag::Double(1.5)],
        },
    );
    cow.insert("Health".to_string(), Tag::Short(7));
    let mut mob = BTreeMap::new();
    mob.insert("id".to_string(), Tag::String("Mob".into()));
    let tree = {
        let mut t = stored_chunk(&store, "w", 0, 0);
        let Some(Tag::List { items, .. }) = level_mut(&mut t).get_mut("Entities") else {
            panic!()
        };
        items.push(Tag::Compound(cow.clone()));
        items.push(Tag::Compound(mob));
        t
    };
    let bytes = mc_nbt::write_named("", &tree).unwrap();
    {
        let mut s = store.borrow_mut();
        let s = &mut *s;
        s.region_cache
            .write_chunk(s.storage.as_mut(), "w", 0, 0, &bytes, 1)
            .unwrap();
    }
    let handler = SaveOldDir::new(store.clone(), "", "w").unwrap();
    let mut b = World::new_from_save(Box::new(handler), "x", 0, None, ENV);
    b.make_deterministic(5);
    let math_before = b.math_random.state();
    b.get_chunk_from_chunk_coords(0, 0);
    // Item, sand (both built), the kept mob (id 2 consumed); "Mob" is
    // abstract: skipped.
    assert_eq!(b.next_entity_id, 3);
    assert_eq!(b.loaded_entity_list.len(), 2);
    assert_ne!(b.math_random.state(), math_before);
    assert!(
        b.unimplemented_hits
            .borrow()
            .keys()
            .any(|k| k.starts_with("EntityList.createEntityFromNBT"))
    );
    b.get_chunk_from_chunk_coords(0, 0).set_chunk_modified();
    b.try_save_world(true).unwrap();
    let t = stored_chunk(&store, "w", 0, 0);
    let Some(Tag::List { items, .. }) = t.get("Level").unwrap().get("Entities") else {
        panic!()
    };
    assert_eq!(
        items.len(),
        3,
        "item, sand, then the kept mob; the Mob is dropped"
    );
    assert_eq!(items[2], Tag::Compound(cow));
}

/// vanilla bug: a chunk stored under the wrong position stays wrong
/// (McRegionChunkLoader.java:27-31 sets xPos/zPos on the root compound,
/// not on `Level`, before reading `Level` again), and its contents are built
/// twice; a misplaced chunk with tile entities throws
/// (`ArrayIndexOutOfBoundsException` in `Chunk.setChunkBlockTileEntity`), as
/// in vanilla.
#[test]
fn relocation_bug() {
    let store = store_over(MemoryStorage::default());
    let mut a = build_saved_world(&store, "w");
    a.try_save_world(true).unwrap();
    drop(a);
    let rewrite = |x: i32, z: i32| {
        let mut t = stored_chunk(&store, "w", x, z);
        level_mut(&mut t).insert("xPos".into(), Tag::Int(7));
        let bytes = mc_nbt::write_named("", &t).unwrap();
        let mut s = store.borrow_mut();
        let s = &mut *s;
        s.region_cache
            .write_chunk(s.storage.as_mut(), "w", x, z, &bytes, 1)
            .unwrap();
    };
    rewrite(1, 1);
    rewrite(0, 0);
    let mut w = empty_world();
    let mut loader = McRegionChunkLoader::new(store.clone(), "w");
    let l = loader.load_chunk(&mut w, 1, 1).unwrap().unwrap();
    assert_eq!((l.chunk.x_position, l.chunk.z_position), (7, 1));
    // Chunk (0, 0): two entities built (ids 0 and 1), then the chest at
    // world x 3 is local x 3 - 7 * 16 < 0.
    let e = loader.load_chunk(&mut w, 0, 0).unwrap_err();
    assert!(matches!(
        e,
        SaveError::Java {
            exception: "ArrayIndexOutOfBoundsException",
            ..
        }
    ));
    assert_eq!(w.next_entity_id, 2);
    let _: Option<&Entity> = l.entities.first();
}
