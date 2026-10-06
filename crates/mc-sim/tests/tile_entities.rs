//! WP-1.8 tile entities (`mc_sim::tile_entity`) and the container blocks.
//!
//! 1. NBT shapes against the oracle: the `TileEntities` list of
//!    `tests/golden/save/tile-entities` (one tile entity of each class,
//!    written by the vanilla `writeToNBT`s through `ChunkLoader`) equals what
//!    the port's tile entities write, in `chunkTileEntityMap` order, after the
//!    same placements; and the owner's `testWorld` chest (2 planks, 4 sticks)
//!    reads with `createAndLoadEntity` and writes back to the same compound.
//! 2. `readFromNBT` semantics per class (slot byte signedness, the chest's
//!    27-slot reallocation, sign text cut, note clamp, the record player's
//!    optional key, unknown ids, `ClassCastException`s).
//! 3. Behaviour no fixture reaches (no op can fill an inventory): furnace
//!    smelting with the idle/lit block swap, the dispenser's random slot,
//!    the `IInventory` methods, chest/furnace/dispenser drops on removal.
//!
//! The Tier S fixture itself runs in `golden_save.rs`.

use std::collections::BTreeMap;

use mc_golden::{Bits32, Bits64, fixture_dir, hex_bytes, read_sidecar};
use mc_nbt::{Tag, TagType};
use mc_sim::entity::player::new_entity_player_sp;
use mc_sim::item::{ItemStack, JavaException};
use mc_sim::save::{MemoryStorage, NoSave, RegionFileCache, Storage};
use mc_sim::tile_entity::{self, furnace::TileEntityFurnace};
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::events::WorldEvent;
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::tile_entity::{TileEntity, TileEntityKindData, TileEntityPiston, TileEntityRef};
use mc_sim::world::{World, WorldEnvironment};
use serde_json::{Map, Value, json};

fn empty_world(seed: i64) -> World {
    let mut w = World::new(
        Box::new(NoSave),
        "harness",
        WorldProvider::new(Dimension::Surface),
        seed,
        ChunkGenerator::Empty(EmptyChunkProvider),
        WorldEnvironment {
            rand_seed: 0,
            field_9437_g: 0,
            lock_timestamp: 0,
        },
    );
    w.make_deterministic(seed);
    for dz in -2..=2 {
        for dx in -2..=2 {
            w.get_chunk_from_chunk_coords(dx, dz);
        }
    }
    w
}

// ---- canonical tree JSON (tests/golden/README.md; as tests/region_file.rs) ----------

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

fn expected_line(area: &str, name: &str, e: &str, pick: impl Fn(&Value) -> bool) -> Value {
    let path = fixture_dir(area, name).join("expected.jsonl");
    let text = std::fs::read_to_string(&path).unwrap();
    text.lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .find(|v| v["e"] == e && pick(v))
        .unwrap_or_else(|| panic!("no {e} line in {}", path.display()))
}

// ---- 1. shapes against the oracle --------------------------------------------------

/// The placements of `save/tile-entities` up to its save at t = 5 (the note
/// block clicked twice, the spawner counting down with a player in range),
/// then the chunk's tile entities written in `chunkTileEntityMap` order.
#[test]
fn tile_entities_write_the_oracles_compounds_in_map_order() {
    let mut w = empty_world(123);
    for x in 0..16 {
        for z in 0..16 {
            w.set_block_and_metadata(x, 60, z, 1, 0);
        }
    }
    let mut p = new_entity_player_sp(&mut w);
    p.base.set_location_and_angles(8.0, 61.0, 10.0, 0.0, 0.0);
    w.spawn_entity_in_world(p);
    let set = |w: &mut World, x, z, id| w.set_block_and_metadata_with_notify(x, 61, z, id, 0);
    set(&mut w, 1, 1, 54);
    set(&mut w, 2, 1, 54);
    set(&mut w, 4, 1, 61);
    set(&mut w, 6, 1, 62);
    set(&mut w, 8, 1, 23);
    set(&mut w, 1, 4, 1);
    w.set_block_and_metadata(1, 61, 3, 68, 2);
    w.set_block_and_metadata(3, 61, 4, 63, 8);
    set(&mut w, 5, 4, 52);
    set(&mut w, 7, 4, 25);
    set(&mut w, 9, 4, 84);
    // Tick 0 (the ops), the clicks at t = 1 before that tick's update, ticks 1-4.
    w.update_entities();
    let note = w.get_block_tile_entity(7, 61, 4).unwrap();
    tile_entity::note::change_pitch(&mut w, note);
    tile_entity::note::change_pitch(&mut w, note);
    for _ in 1..5 {
        w.update_entities();
    }

    let chunk = w.loaded_chunk(0, 0).unwrap();
    let refs: Vec<TileEntityRef> = chunk.chunk_tile_entity_map.values().copied().collect();
    let ours: Vec<Value> = refs
        .iter()
        .map(|r| canonical_value(&tile_entity::write_to_nbt(&w.tile_entities[r], &w)))
        .collect();
    let line = expected_line("save", "tile-entities", "chunk_nbt", |v| {
        v["x"] == 0 && v["z"] == 0
    });
    let theirs = &line["root"]["value"]["Level"]["value"]["TileEntities"]["value"];
    assert_eq!(theirs["elem"], "compound");
    assert_eq!(json!(ours), theirs["items"]);
}

/// `testWorld`'s chest at (9, 64, -61) (PLAN_v4 §4.4 item 4) through
/// `createAndLoadEntity`, and back through `writeToNBT`.
#[test]
fn testworld_chest_round_trips() {
    let bytes = read_sidecar(
        fixture_dir("save", "testworld").join("world/region"),
        "r.0.-1.mcr",
    )
    .unwrap();
    let mut storage = MemoryStorage::default();
    storage.write("world/region/r.0.-1.mcr", &bytes).unwrap();
    let mut cache = RegionFileCache::new();
    let nbt = cache
        .get_chunk_input_stream(&mut storage, "world", 0, -4)
        .unwrap()
        .expect("chunk (0,-4) present");
    let (_, root) = mc_nbt::read_named(&nbt).unwrap();
    let Some(Tag::List { items, .. }) = root.get("Level").and_then(|l| l.get("TileEntities"))
    else {
        panic!("no Level.TileEntities");
    };
    let raw = items
        .iter()
        .find(|t| t.get("x") == Some(&Tag::Int(9)) && t.get("z") == Some(&Tag::Int(-61)))
        .expect("the chest's compound");
    let w = empty_world(0);
    let te = tile_entity::create_and_load_entity(raw, &w)
        .unwrap()
        .expect("a known id");
    assert_eq!((te.x_coord, te.y_coord, te.z_coord), (9, 64, -61));
    let TileEntityKindData::Chest(c) = &te.kind else {
        panic!("not a chest: {:?}", te.kind)
    };
    assert_eq!(c.chest_contents.len(), 27);
    assert_eq!(c.chest_contents[0], Some(ItemStack::new(5, 2, 0)));
    assert_eq!(c.chest_contents[1], Some(ItemStack::new(280, 4, 0)));
    assert!(c.chest_contents[2..].iter().all(Option::is_none));
    assert_eq!(&tile_entity::write_to_nbt(&te, &w), raw);
    // The oracle's line for it agrees.
    let line = expected_line("save", "testworld", "tile_entity", |v| v["x"] == 9);
    assert_eq!(line["id"], "Chest");
    assert_eq!(line["size"], 27);
}

// ---- 2. readFromNBT semantics ------------------------------------------------------

fn compound(entries: &[(&str, Tag)]) -> Tag {
    Tag::Compound(
        entries
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect::<BTreeMap<_, _>>(),
    )
}

fn item(slot: i8, id: i16, count: i8, damage: i16) -> Tag {
    compound(&[
        ("Slot", Tag::Byte(slot)),
        ("id", Tag::Short(id)),
        ("Count", Tag::Byte(count)),
        ("Damage", Tag::Short(damage)),
    ])
}

fn items(list: Vec<Tag>) -> Tag {
    Tag::List {
        elem: TagType::Compound,
        items: list,
    }
}

fn load(t: &Tag) -> TileEntity {
    let w = empty_world(0);
    tile_entity::create_and_load_entity(t, &w)
        .unwrap()
        .expect("a known id")
}

#[test]
fn inventories_read_slots_as_their_class_does() {
    // Chest and dispenser: `getByte("Slot") & 255`; -56 is slot 200, out of range.
    let chest = load(&compound(&[
        ("id", Tag::String("Chest".into())),
        ("x", Tag::Int(-5)),
        ("y", Tag::Int(70)),
        ("z", Tag::Int(12)),
        (
            "Items",
            items(vec![
                item(26, 4, 64, 0),
                item(-56, 1, 1, 0),
                item(3, 351, 5, 4),
            ]),
        ),
    ]));
    let TileEntityKindData::Chest(c) = &chest.kind else {
        panic!()
    };
    assert_eq!(c.chest_contents.len(), 27);
    assert_eq!(c.chest_contents[26], Some(ItemStack::new(4, 64, 0)));
    assert_eq!(c.chest_contents[3], Some(ItemStack::new(351, 5, 4)));
    assert_eq!(c.chest_contents.iter().flatten().count(), 2);
    // A fresh chest has 36 slots and writes an empty list typed as TAG_Byte.
    let fresh = TileEntity::new(TileEntityKindData::Chest(Default::default()));
    let TileEntityKindData::Chest(c) = &fresh.kind else {
        panic!()
    };
    assert_eq!(c.chest_contents.len(), 36);
    let w = empty_world(0);
    assert_eq!(
        tile_entity::write_to_nbt(&fresh, &w).get("Items"),
        Some(&Tag::List {
            elem: TagType::Byte,
            items: vec![]
        })
    );

    let disp = load(&compound(&[
        ("id", Tag::String("Trap".into())),
        ("Items", items(vec![item(8, 262, 16, 0), item(9, 1, 1, 0)])),
    ]));
    let TileEntityKindData::Dispenser(d) = &disp.kind else {
        panic!()
    };
    assert_eq!(d.dispenser_contents[8], Some(ItemStack::new(262, 16, 0)));
    assert_eq!(d.dispenser_contents.iter().flatten().count(), 1);

    // Furnace: a signed slot byte (-56 < 0 is skipped), BurnTime / CookTime
    // shorts, currentItemBurnTime from the fuel slot.
    let furnace = load(&compound(&[
        ("id", Tag::String("Furnace".into())),
        ("BurnTime", Tag::Short(1234)),
        ("CookTime", Tag::Short(57)),
        (
            "Items",
            items(vec![
                item(0, 15, 3, 0),
                item(1, 263, 7, 1),
                item(-56, 1, 1, 0),
            ]),
        ),
    ]));
    let TileEntityKindData::Furnace(f) = &furnace.kind else {
        panic!()
    };
    assert_eq!(f.furnace_burn_time, 1234);
    assert_eq!(f.furnace_cook_time, 57);
    assert_eq!(f.current_item_burn_time, 1600);
    assert_eq!(f.furnace_item_stacks[0], Some(ItemStack::new(15, 3, 0)));
    assert_eq!(f.furnace_item_stacks[2], None);
    // Written back: the casts to short and the slot order.
    let back = tile_entity::write_to_nbt(&furnace, &w);
    assert_eq!(back.get("BurnTime"), Some(&Tag::Short(1234)));
    assert_eq!(back.get("id"), Some(&Tag::String("Furnace".into())));
    let Some(Tag::List { items: written, .. }) = back.get("Items") else {
        panic!()
    };
    assert_eq!(written, &vec![item(0, 15, 3, 0), item(1, 263, 7, 1)]);
}

#[test]
fn small_tile_entities_read_and_write() {
    let w = empty_world(0);
    // Sign: each line cut to 15 characters; missing lines read as "".
    let sign = load(&compound(&[
        ("id", Tag::String("Sign".into())),
        ("Text1", Tag::String("0123456789abcdefXYZ".into())),
        ("Text3", Tag::String("hi".into())),
    ]));
    let TileEntityKindData::Sign(s) = &sign.kind else {
        panic!()
    };
    assert_eq!(
        s.sign_text,
        ["0123456789abcde", "", "hi", ""].map(String::from)
    );
    assert!(!s.field_25062_c);
    assert_eq!(s.line_being_edited, -1);
    let back = tile_entity::write_to_nbt(&sign, &w);
    assert_eq!(back.get("Text2"), Some(&Tag::String(String::new())));
    assert_eq!(
        back.get("Text1"),
        Some(&Tag::String("0123456789abcde".into()))
    );

    // Note: clamped to 0..=24.
    for (raw, want) in [(-3i8, 0i8), (30, 24), (12, 12)] {
        let n = load(&compound(&[
            ("id", Tag::String("Music".into())),
            ("note", Tag::Byte(raw)),
        ]));
        let TileEntityKindData::Note(d) = &n.kind else {
            panic!()
        };
        assert_eq!(d.note, want);
    }

    // Record player: `Record` only when a record is inside.
    let empty = load(&compound(&[("id", Tag::String("RecordPlayer".into()))]));
    assert_eq!(tile_entity::write_to_nbt(&empty, &w).get("Record"), None);
    let full = load(&compound(&[
        ("id", Tag::String("RecordPlayer".into())),
        ("Record", Tag::Int(2257)),
    ]));
    assert_eq!(
        tile_entity::write_to_nbt(&full, &w).get("Record"),
        Some(&Tag::Int(2257))
    );

    // Mob spawner: EntityId string, Delay short; a fresh one is "Pig", 20.
    let ms = load(&compound(&[
        ("id", Tag::String("MobSpawner".into())),
        ("EntityId", Tag::String("Zombie".into())),
        ("Delay", Tag::Short(-1)),
    ]));
    let TileEntityKindData::MobSpawner(d) = &ms.kind else {
        panic!()
    };
    assert_eq!((d.mob_id.as_str(), d.delay), ("Zombie", -1));
    let fresh = TileEntity::new(tile_entity::new_instance("TileEntityMobSpawner", &w).unwrap());
    let back = tile_entity::write_to_nbt(&fresh, &w);
    assert_eq!(back.get("EntityId"), Some(&Tag::String("Pig".into())));
    assert_eq!(back.get("Delay"), Some(&Tag::Short(20)));
}

#[test]
fn ids_and_exceptions() {
    let w = empty_world(0);
    // The mapping table (TileEntity.java:100-109) both ways.
    assert_eq!(tile_entity::MAPPINGS.len(), 8);
    for (id, class) in tile_entity::MAPPINGS {
        assert_eq!(tile_entity::name_to_class(id), Some(class));
        assert_eq!(tile_entity::class_to_name(class), Some(id));
    }
    // Unknown and missing ids are skipped.
    let unknown = compound(&[("id", Tag::String("Cauldron".into()))]);
    assert_eq!(tile_entity::create_and_load_entity(&unknown, &w), Ok(None));
    assert_eq!(
        tile_entity::create_and_load_entity(&compound(&[]), &w),
        Ok(None)
    );
    // A key of the wrong type throws out of createAndLoadEntity.
    let bad = compound(&[("id", Tag::String("Chest".into())), ("x", Tag::Short(1))]);
    assert_eq!(
        tile_entity::create_and_load_entity(&bad, &w),
        Err(JavaException::ClassCast)
    );
    let bad_id = compound(&[("id", Tag::Int(1))]);
    assert_eq!(
        tile_entity::create_and_load_entity(&bad_id, &w),
        Err(JavaException::ClassCast)
    );
    // The piston (pistons task, WP-1.6d): `new TileEntityPiston()` then its
    // readFromNBT (TileEntityPiston.java:17-18, :128-135); writeToNBT saves
    // the same compound back (:137-144).
    let saved = compound(&[
        ("id", Tag::String("Piston".into())),
        ("x", Tag::Int(3)),
        ("y", Tag::Int(0)),
        ("z", Tag::Int(0)),
        ("blockId", Tag::Int(1)),
        ("blockData", Tag::Int(2)),
        ("facing", Tag::Int(4)),
        ("progress", Tag::Float(0.5)),
        ("extending", Tag::Byte(1)),
    ]);
    let piston = load(&saved);
    assert_eq!(
        piston.kind,
        TileEntityKindData::Piston(TileEntityPiston {
            progress: 0.5,
            last_progress: 0.5,
            ..TileEntityPiston::new(1, 2, 4, true, false)
        })
    );
    assert_eq!(piston.x_coord, 3);
    assert_eq!(tile_entity::write_to_nbt(&piston, &w), saved);
}

// ---- 3. behaviour ------------------------------------------------------------------

fn furnace_mut(w: &mut World, r: TileEntityRef) -> &mut TileEntityFurnace {
    match &mut w.tile_entities.get_mut(&r).unwrap().kind {
        TileEntityKindData::Furnace(f) => f,
        k => panic!("not a furnace: {k:?}"),
    }
}

/// `TileEntityFurnace.updateEntity` with `updateFurnaceBlockState`: the block
/// turns lit on the first update, the original tile entity is set again at
/// its position (the one the lit block's `onBlockAdded` made stays in
/// `loadedTileEntityList`, vanilla), 200 ticks cook one item, and the block
/// turns idle when the fuel is gone.
#[test]
fn furnace_smelts_and_swaps_the_block() {
    let mut w = empty_world(5);
    w.set_block_and_metadata_with_notify(0, 61, 0, 1, 0);
    w.set_block_and_metadata_with_notify(0, 61, 1, 61, 0);
    assert_eq!(
        w.get_block_metadata(0, 61, 1),
        3,
        "setDefaultDirection: stone at -z"
    );
    let r = w.get_block_tile_entity(0, 61, 1).unwrap();
    {
        let f = furnace_mut(&mut w, r);
        f.furnace_item_stacks[0] = Some(ItemStack::new(4, 2, 0)); // cobblestone
        f.furnace_item_stacks[1] = Some(ItemStack::new(280, 1, 0)); // a stick: 100 ticks
    }
    w.events.drain();
    w.update_entities();
    assert_eq!(w.get_block_id(0, 61, 1), 62);
    assert_eq!(w.get_block_metadata(0, 61, 1), 3, "metadata kept");
    assert_eq!(w.get_block_tile_entity(0, 61, 1), Some(r));
    assert_eq!(w.loaded_tile_entity_list.len(), 2);
    assert!(
        w.events
            .drain()
            .iter()
            .any(|e| matches!(e, WorldEvent::DoNothingWithTileEntity { x: 0, y: 61, z: 1 }))
    );
    {
        let f = furnace_mut(&mut w, r);
        assert_eq!(
            (
                f.furnace_burn_time,
                f.current_item_burn_time,
                f.furnace_cook_time
            ),
            (100, 100, 1)
        );
        assert_eq!(f.furnace_item_stacks[1], None);
    }
    // The stick burns out after 100 updates, before the first item is done.
    for _ in 0..100 {
        w.update_entities();
    }
    assert_eq!(w.get_block_id(0, 61, 1), 61);
    assert_eq!(w.get_block_tile_entity(0, 61, 1), Some(r));
    {
        let f = furnace_mut(&mut w, r);
        assert_eq!((f.furnace_burn_time, f.furnace_cook_time), (0, 0));
        // Coal: the next update lights it again and cooks for 200 ticks.
        f.furnace_item_stacks[1] = Some(ItemStack::new(263, 1, 0));
    }
    for _ in 0..200 {
        w.update_entities();
    }
    assert_eq!(w.get_block_id(0, 61, 1), 62);
    let f = furnace_mut(&mut w, r);
    assert_eq!(f.furnace_item_stacks[0], Some(ItemStack::new(4, 1, 0)));
    assert_eq!(f.furnace_item_stacks[2], Some(ItemStack::new(1, 1, 0)));
    assert_eq!((f.furnace_burn_time, f.furnace_cook_time), (1401, 0));
}

/// `getRandomStackFromInventory`: `nextInt(n)` for the n-th non-empty slot of
/// the tile entity's own `Random`, then `decrStackSize(slot, 1)`.
#[test]
fn dispenser_random_stack() {
    let mut w = empty_world(9);
    w.set_block_and_metadata_with_notify(0, 61, 0, 23, 0);
    let r = w.get_block_tile_entity(0, 61, 0).unwrap();
    let seed = {
        let TileEntityKindData::Dispenser(d) = &mut w.tile_entities.get_mut(&r).unwrap().kind
        else {
            panic!()
        };
        d.dispenser_contents[2] = Some(ItemStack::new(4, 3, 0));
        d.dispenser_contents[5] = Some(ItemStack::new(12, 1, 0));
        d.dispenser_contents[7] = Some(ItemStack::new(3, 2, 0));
        d.dispenser_random.clone()
    };
    // The same choice by hand.
    let mut rnd = seed;
    let mut pick = -1;
    for (n, slot) in [(1, 2), (2, 5), (3, 7)] {
        if rnd.next_int_bounded(n) == 0 {
            pick = slot;
        }
    }
    let got = tile_entity::dispenser::get_random_stack_from_inventory(&mut w, r).unwrap();
    let want_id = [(2, 4), (5, 12), (7, 3)]
        .iter()
        .find(|(s, _)| *s == pick)
        .unwrap()
        .1;
    assert_eq!(got, ItemStack::new(want_id, 1, 0));
    let TileEntityKindData::Dispenser(d) = &w.tile_entities[&r].kind else {
        panic!()
    };
    assert_eq!(
        d.dispenser_contents
            .iter()
            .flatten()
            .map(|s| s.stack_size)
            .sum::<i32>(),
        5
    );
    // Empty: no draw, nothing.
    let mut w2 = empty_world(9);
    w2.set_block_and_metadata_with_notify(0, 61, 0, 23, 0);
    let r2 = w2.get_block_tile_entity(0, 61, 0).unwrap();
    assert_eq!(
        tile_entity::dispenser::get_random_stack_from_inventory(&mut w2, r2),
        None
    );
}

#[test]
fn iinventory_methods() {
    let mut w = empty_world(1);
    w.set_block_and_metadata_with_notify(0, 61, 0, 54, 0);
    let r = w.get_block_tile_entity(0, 61, 0).unwrap();
    w.events.drain();
    // setInventorySlotContents caps at 64 and notifies (chest).
    tile_entity::set_inventory_slot_contents(&mut w, r, 4, Some(ItemStack::new(1, 100, 0)));
    let kind = &w.tile_entities[&r].kind;
    assert_eq!(
        tile_entity::get_stack_in_slot(kind, 4),
        Some(ItemStack::new(1, 64, 0))
    );
    assert_eq!(tile_entity::get_size_inventory(kind), Some(27));
    assert_eq!(tile_entity::get_inv_name(kind), Some("Chest"));
    assert_eq!(w.events.drain().len(), 1);
    // decrStackSize: a split, then the rest.
    assert_eq!(
        tile_entity::decr_stack_size(&mut w, r, 4, 60),
        Some(ItemStack::new(1, 60, 0))
    );
    assert_eq!(
        tile_entity::decr_stack_size(&mut w, r, 4, 10),
        Some(ItemStack::new(1, 4, 0))
    );
    assert_eq!(tile_entity::decr_stack_size(&mut w, r, 4, 1), None);
    // canInteractWith: the same tile entity at its position, within 8 blocks.
    let mut p = new_entity_player_sp(&mut w);
    p.base.set_position(0.5, 61.5, 8.4);
    assert!(tile_entity::can_interact_with(&mut w, r, &p.base));
    p.base.set_position(0.5, 61.5, 8.6);
    assert!(!tile_entity::can_interact_with(&mut w, r, &p.base));
    // The furnace's set/decr do not notify.
    w.set_block_and_metadata_with_notify(3, 61, 0, 61, 0);
    let f = w.get_block_tile_entity(3, 61, 0).unwrap();
    w.events.drain();
    tile_entity::set_inventory_slot_contents(&mut w, f, 0, Some(ItemStack::new(15, 2, 0)));
    tile_entity::decr_stack_size(&mut w, f, 0, 1);
    assert!(w.events.drain().is_empty());
}

/// `BlockChest/Furnace/Dispenser.onBlockRemoval`: the inventory scattered as
/// `EntityItem`s, stacks of `nextInt(21) + 10`, from the block's own
/// `Random` (`mix(seed, -2)` for the chest); `keepFurnaceInventory` stops it.
#[test]
fn removal_scatters_the_inventory() {
    let mut w = empty_world(77);
    w.set_block_and_metadata_with_notify(0, 61, 0, 54, 0);
    let r = w.get_block_tile_entity(0, 61, 0).unwrap();
    tile_entity::set_inventory_slot_contents(&mut w, r, 0, Some(ItemStack::new(4, 64, 0)));
    tile_entity::set_inventory_slot_contents(&mut w, r, 9, Some(ItemStack::new(280, 3, 0)));
    let mut rand = w.block_statics.chest_random.clone();
    // Slot 0: three offsets, then stacks until 64 are used up; slot 9 likewise.
    let mut sizes = Vec::new();
    for total in [64, 3] {
        for _ in 0..3 {
            rand.next_float();
        }
        let mut left = total;
        while left > 0 {
            let n = (rand.next_int_bounded(21) + 10).min(left);
            left -= n;
            sizes.push(n);
            for _ in 0..3 {
                rand.next_gaussian();
            }
        }
    }
    w.set_block_with_notify(0, 61, 0, 0);
    let drops: Vec<i32> = w
        .loaded_entity_list
        .iter()
        .filter_map(|s| s.entity.as_ref()?.kind.item()?.item.map(|i| i.stack_size))
        .collect();
    assert_eq!(drops, sizes);
    assert_eq!(w.block_statics.chest_random, rand);
    assert_eq!(w.get_block_tile_entity(0, 61, 0), None);

    // A furnace swapped by updateFurnaceBlockState keeps its items.
    w.set_block_and_metadata_with_notify(3, 61, 0, 61, 0);
    let f = w.get_block_tile_entity(3, 61, 0).unwrap();
    tile_entity::set_inventory_slot_contents(&mut w, f, 0, Some(ItemStack::new(15, 2, 0)));
    let before = w.loaded_entity_list.len();
    mc_sim::block::furnace::update_furnace_block_state(true, &mut w, 3, 61, 0);
    assert_eq!(w.loaded_entity_list.len(), before);
    assert_eq!(w.get_block_id(3, 61, 0), 62);
    assert_eq!(w.get_block_tile_entity(3, 61, 0), Some(f));
    assert!(!w.block_statics.keep_furnace_inventory);
    // Broken for real, the lit furnace scatters with stoneOvenActive's Random.
    let lit = w.block_statics.stone_oven_active_furnace_rand.clone();
    w.set_block_with_notify(3, 61, 0, 0);
    assert_eq!(w.loaded_entity_list.len(), before + 1);
    assert_ne!(w.block_statics.stone_oven_active_furnace_rand, lit);
}

#[test]
fn chest_placement_rule() {
    use mc_sim::block::dispatch::can_place_block_at;
    let mut w = empty_world(1);
    w.set_block(0, 61, 0, 54);
    assert!(
        can_place_block_at(&mut w, 54, 1, 61, 0),
        "beside a single chest"
    );
    w.set_block(1, 61, 0, 54);
    assert!(!can_place_block_at(&mut w, 54, 2, 61, 0), "beside a pair");
    assert!(!can_place_block_at(&mut w, 54, 0, 61, 1), "beside a pair");
    assert!(!can_place_block_at(&mut w, 54, 1, 61, -1), "beside a pair");
    assert!(can_place_block_at(&mut w, 54, 3, 61, 0), "two away");
    w.set_block(5, 61, 0, 54);
    w.set_block(5, 61, 2, 54);
    assert!(
        !can_place_block_at(&mut w, 54, 5, 61, 1),
        "between two singles"
    );
    // No air check of its own: the target cell is not read.
    w.set_block(9, 61, 9, 1);
    assert!(can_place_block_at(&mut w, 54, 9, 61, 9));
}
