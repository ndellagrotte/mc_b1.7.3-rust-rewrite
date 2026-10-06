//! WP-1.7 population (`mc_sim::worldgen::populate`, `mc_sim::worldgen::features`)
//! beyond the Tier A digests of `tests/golden_worldgen.rs`:
//!
//! 1. The dungeon of chunk (1, 1), seed 123: the chests' loot
//!    (`WorldGenDungeons.pickCheckLootItem`) and the spawner's mob
//!    (`pickMobSpawner`), which reach the worldgen digest only as blocks,
//!    against the `TileEntities` the oracle saved in `tick/dungeon-chest-save`
//!    (whose own Rust test waits for the save path, WP-1.9b).
//! 2. The two `nether-seed-123-order-*` fixtures really pin the hellRNG
//!    load-order quirk (their populated digests differ).
//! 3. `BlockSand.fallInstantly` is set only during population.
//! 4. The block ids population names (`block::ids`).

use mc_golden::scenario::Dimension;
use mc_golden::terrain_driver::build_world;
use mc_golden::{Bits32, Bits64, fixture_dir, hex_bytes};
use mc_nbt::{Tag, TagType};
use mc_sim::tile_entity;
use mc_sim::world::World;
use mc_sim::world::tile_entity::TileEntityRef;
use serde_json::{Map, Value, json};

// ---- canonical tree JSON (tests/golden/README.md; as tests/tile_entities.rs) ---------

fn type_name(ty: TagType) -> &'static str {
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

fn canonical_value(tag: &Tag) -> Value {
    match tag {
        Tag::Byte(v) => (*v).into(),
        Tag::Short(v) => (*v).into(),
        Tag::Int(v) => (*v).into(),
        Tag::Long(v) => (*v).into(),
        Tag::Float(v) => Bits32::from(*v).into(),
        Tag::Double(v) => Bits64::from(*v).into(),
        Tag::ByteArray(bytes) => hex_bytes(bytes).into(),
        Tag::String(s) => s.as_str().into(),
        Tag::List { elem, items } => json!({
            "elem": type_name(*elem),
            "items": items.iter().map(canonical_value).collect::<Vec<_>>(),
        }),
        Tag::Compound(map) => Value::Object(
            map.iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        json!({"type": type_name(v.tag_type()), "value": canonical_value(v)}),
                    )
                })
                .collect::<Map<_, _>>(),
        ),
    }
}

fn expected_lines(area: &str, name: &str) -> Vec<Value> {
    let path = fixture_dir(area, name).join("expected.jsonl");
    let text = std::fs::read_to_string(&path).unwrap();
    text.lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .collect()
}

/// The tick driver's `preload` op: the square dz outer, dx inner, draining
/// the light queue after each chunk (tests/golden/README.md "Preload").
fn preload_square(world: &mut World, cx: i32, cz: i32, r: i32) {
    for dz in -r..=r {
        for dx in -r..=r {
            world.get_chunk_from_chunk_coords(cx + dx, cz + dz);
            while world.updating_lighting() {}
        }
    }
}

// ---- 1. the dungeon's tile entities ---------------------------------------------------

/// `tick/dungeon-chest-save` preloads the 3×3 around chunk (1, 1) on a seed
/// 123 world and saves at t = 0, before anything ticks: the saved
/// `TileEntities` are exactly what population left, in
/// `chunkTileEntityMap` order.
#[test]
fn dungeon_loot_and_mob_match_the_oracles_save() {
    let mut w = build_world(123, Dimension::Overworld);
    preload_square(&mut w, 1, 1, 1);

    let chunk = w.loaded_chunk(1, 1).expect("chunk (1,1) loaded");
    assert!(chunk.is_terrain_populated);
    let refs: Vec<TileEntityRef> = chunk.chunk_tile_entity_map.values().copied().collect();
    let ours: Vec<Value> = refs
        .iter()
        .map(|r| canonical_value(&tile_entity::write_to_nbt(&w.tile_entities[r], &w)))
        .collect();

    let lines = expected_lines("tick", "dungeon-chest-save");
    let nbt = lines
        .iter()
        .find(|v| v["e"] == "chunk_nbt" && v["x"] == 1 && v["z"] == 1)
        .expect("chunk_nbt 1,1");
    let theirs = &nbt["root"]["value"]["Level"]["value"]["TileEntities"]["value"];
    assert_eq!(theirs["elem"], "compound");
    assert_eq!(json!(ours), theirs["items"]);

    // The fixture is the one the module docs describe: a spawner and two
    // stocked chests.
    let tes: Vec<&Value> = lines.iter().filter(|v| v["e"] == "tile_entity").collect();
    assert_eq!(tes.len(), 3);
    assert_eq!(tes[0]["id"], "MobSpawner");
    assert!(
        tes[1..]
            .iter()
            .all(|t| t["id"] == "Chest" && !t["items"].as_array().unwrap().is_empty())
    );
}

// ---- 2. the nether order pair ---------------------------------------------------------

/// The `populated` line of chunk (x, z) in a worldgen fixture.
fn populated(name: &str, x: i32, z: i32) -> Value {
    expected_lines("worldgen", name)
        .into_iter()
        .find(|v| {
            v["e"] == "chunk_digest" && v["stage"] == "populated" && v["x"] == x && v["z"] == z
        })
        .unwrap_or_else(|| panic!("no populated {x},{z} in worldgen/{name}"))
}

/// `ChunkProviderHell.populate` never reseeds `hellRNG`, so the same chunks
/// loaded in the other order populate differently (both fixtures pass in
/// `tests/golden_worldgen.rs`; this checks the pair is not vacuous).
#[test]
fn nether_order_fixtures_differ() {
    let ab = populated("nether-seed-123-order-ab", 0, 0);
    let ba = populated("nether-seed-123-order-ba", 0, 0);
    assert_ne!(ab["blocks"], ba["blocks"]);
}

// ---- 3. fallInstantly -------------------------------------------------------------------

/// `BlockSand.fallInstantly` is true only inside `populate` (both
/// generators set it on entry and clear it on exit).
#[test]
fn fall_instantly_is_cleared_after_population() {
    for dim in [Dimension::Overworld, Dimension::Nether] {
        let mut w = build_world(123, dim);
        assert!(!w.block_statics.fall_instantly);
        preload_square(&mut w, 0, 0, 1);
        assert!(w.loaded_chunk(0, 0).unwrap().is_terrain_populated);
        assert!(!w.block_statics.fall_instantly);
    }
}

// ---- 4. the block ids population uses -----------------------------------------------

/// The `block::ids` constants WP-1.7 added name the blocks of those names
/// (`setBlockName`, Block.java:606-681).
#[test]
fn population_block_ids_name_their_blocks() {
    use mc_sim::block::{block, ids};
    for (id, name) in [
        (ids::ORE_GOLD, "oreGold"),
        (ids::ORE_IRON, "oreIron"),
        (ids::TALL_GRASS, "tallgrass"),
        (ids::DEAD_BUSH, "deadbush"),
        (ids::PLANT_YELLOW, "flower"),
        (ids::PLANT_RED, "rose"),
        (ids::MUSHROOM_BROWN, "mushroom"),
        (ids::MUSHROOM_RED, "mushroom"),
        (ids::COBBLESTONE_MOSSY, "stoneMoss"),
        (ids::MOB_SPAWNER, "mobSpawner"),
        (ids::CHEST, "chest"),
        (ids::ORE_REDSTONE, "oreRedstone"),
        (ids::SNOW, "snow"),
        (ids::CACTUS, "cactus"),
        (ids::BLOCK_CLAY, "clay"),
        (ids::REED, "reeds"),
        (ids::PUMPKIN, "pumpkin"),
        (ids::GLOW_STONE, "lightgem"),
    ] {
        let b = block(id).unwrap_or_else(|| panic!("no block {id}"));
        assert_eq!(b.block_id, id);
        assert_eq!(b.block_name, Some(name), "block {id}");
    }
}
