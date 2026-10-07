//! WP-1.6 task g, the simple remainder: `BlockTNT`, `BlockSoulSand`,
//! `BlockWeb`, `BlockSponge`, `BlockPumpkin`, `BlockBreakable` and the drop
//! tables of the data-only classes (`BlockStone`, `BlockOre`, `BlockGravel`,
//! `BlockClay`, `BlockGlowStone`, `BlockObsidian`, `BlockGlass`,
//! `BlockBookshelf`, `BlockCloth`, `BlockOreStorage`, `BlockSandStone`,
//! `BlockNetherrack`, `BlockMobSpawner`, …).
//!
//! No golden op breaks a block: `set_block` replaces it without
//! `dropBlockAsItem` (World.java:461-468) and the harness has no harvest op,
//! so the drop paths (`dropBlockAsItemWithChance`, `quantityDropped`,
//! `idDropped`, `harvestBlock`, `onBlockDestroyedByPlayer`) are checked here
//! against an independent transcription of each class's Java ([`spec`]),
//! which in turn is checked against the oracle's per-id dump
//! (`tests/golden/block/block-table`, `idDropped(m, new Random(0))` and
//! `quantityDropped(new Random(0))`). The tick fixtures `soul-sand-web-item`,
//! `sponge-removal-notifies` and `tnt-by-fire` cover the rest.

use mc_core::java_random::JavaRandom;
use mc_sim::block::dispatch as d;
use mc_sim::entity::player::new_entity_player_sp;
use mc_sim::entity::{Entity, EntityKindData};
use mc_sim::item::ItemStack;
use mc_sim::save::NoSave;
use mc_sim::stats::StatList;
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::events::WorldEvent;
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::{World, WorldEnvironment};

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
    for dz in -1..=1 {
        for dx in -1..=1 {
            w.get_chunk_from_chunk_coords(dx, dz);
        }
    }
    w
}

// Item ids, `256 + the constructor argument` (Item.java).
const COAL: i32 = 256 + 7; // Item.java:281
const DIAMOND: i32 = 256 + 8; // Item.java:282
const SILK: i32 = 256 + 31; // Item.java:305
const FLINT: i32 = 256 + 62; // Item.java:336
const CLAY: i32 = 256 + 81; // Item.java:355
const LIGHT_STONE_DUST: i32 = 256 + 92; // Item.java:366
const DYE_POWDER: i32 = 256 + 95; // Item.java:369
const FLINT_AND_STEEL: i32 = 256 + 3; // Item.java:277
const PICKAXE_WOOD: i32 = 256 + 14; // Item.java:288
const PICKAXE_DIAMOND: i32 = 256 + 22; // Item.java:296

/// One block's drop methods, transcribed again from its class for the test.
struct Spec {
    id: i32,
    quantity: fn(&mut JavaRandom) -> i32,
    id_dropped: fn(i32, i32, &mut JavaRandom) -> i32,
    damage: fn(i32, i32) -> i32,
}

fn one(_: &mut JavaRandom) -> i32 {
    1 // Block.java:319-321
}
fn zero(_: &mut JavaRandom) -> i32 {
    0
}
fn itself(id: i32, _: i32, _: &mut JavaRandom) -> i32 {
    id // Block.java:323-325
}
fn no_damage(_: i32, _: i32) -> i32 {
    0 // Block.java:367-369
}

/// The blocks of this task, by id (Block.java:593-683).
fn spec() -> Vec<Spec> {
    let s = |id, quantity, id_dropped, damage| Spec {
        id,
        quantity,
        id_dropped,
        damage,
    };
    vec![
        // BlockStone.java:10-12: cobblestone
        s(1, one, |_, _, _| 4, no_damage),
        // BlockGravel.java:10-12: flint one time in ten (inherits BlockSand, Block's quantity)
        s(
            13,
            one,
            |id, _, r| {
                if r.next_int_bounded(10) == 0 {
                    FLINT
                } else {
                    id
                }
            },
            no_damage,
        ),
        // BlockOre.java:10-26 (gold and iron ore drop themselves)
        s(14, one, itself, no_damage),
        s(15, one, itself, no_damage),
        s(16, one, |_, _, _| COAL, no_damage),
        s(
            21,
            |r| 4 + r.next_int_bounded(5),
            |_, _, _| DYE_POWDER,
            |_, _| 4,
        ),
        s(56, one, |_, _, _| DIAMOND, no_damage),
        // BlockSponge: Block's drops
        s(19, one, itself, no_damage),
        // BlockGlass.java:10-12
        s(20, zero, itself, no_damage),
        // BlockSandStone: Block's drops
        s(24, one, itself, no_damage),
        // BlockWeb.java:30-32
        s(30, one, |_, _, _| SILK, no_damage),
        // BlockCloth.java:17-19: the colour survives
        s(35, one, itself, |_, m| m),
        // BlockOreStorage: Block's drops
        s(41, one, itself, no_damage),
        s(42, one, itself, no_damage),
        s(57, one, itself, no_damage),
        // BlockTNT.java:35-37
        s(46, zero, itself, no_damage),
        // BlockBookshelf.java:14-16
        s(47, zero, itself, no_damage),
        // BlockObsidian.java:10-16
        s(49, one, |_, _, _| 49, no_damage),
        // BlockMobSpawner.java:14-20
        s(52, zero, |_, _, _| 0, no_damage),
        // BlockClay.java:10-16
        s(82, |_| 4, |_, _, _| CLAY, no_damage),
        // BlockPumpkin, BlockNetherrack, BlockSoulSand: Block's drops
        s(86, one, itself, no_damage),
        s(87, one, itself, no_damage),
        s(88, one, itself, no_damage),
        // BlockGlowStone.java:10-16
        s(
            89,
            |r| 2 + r.next_int_bounded(3),
            |_, _, _| LIGHT_STONE_DUST,
            no_damage,
        ),
        s(91, one, itself, no_damage),
    ]
}

/// The spec agrees with the oracle's dump of every block: `idDropped(m,
/// new Random(0))` for m in 0..16 and `quantityDropped(new Random(0))`, a
/// fresh `Random(0)` per call (tests/golden/README.md, `block_table`).
#[test]
fn spec_matches_the_oracle_block_table() {
    let path = mc_golden::fixture_dir("block", "block-table").join("expected.jsonl");
    let text = std::fs::read_to_string(&path).expect("read block-table expected.jsonl");
    let mut seen = 0;
    for line in text.lines() {
        let v: serde_json::Value = serde_json::from_str(line).expect("json line");
        if v["e"] != "block_def" || v["present"] != true {
            continue;
        }
        let id = v["id"].as_i64().unwrap() as i32;
        let Some(sp) = spec().into_iter().find(|s| s.id == id) else {
            continue;
        };
        seen += 1;
        assert_eq!(
            v["quantityDropped"].as_i64().unwrap() as i32,
            (sp.quantity)(&mut JavaRandom::new(0)),
            "id {id} quantityDropped"
        );
        for m in 0..16 {
            assert_eq!(
                v["idDropped"][m as usize].as_i64().unwrap() as i32,
                (sp.id_dropped)(id, m, &mut JavaRandom::new(0)),
                "id {id} idDropped({m})"
            );
            assert_eq!(
                v["damageDropped"][m as usize].as_i64().unwrap() as i32,
                (sp.damage)(id, m),
                "id {id} damageDropped({m})"
            );
        }
    }
    assert_eq!(seen, spec().len(), "every spec id is in the block table");
}

/// What `dropBlockAsItemWithChance` (Block.java:339-365) must leave behind
/// for a block, replayed on a copy of `World.rand`: the stacks dropped with
/// their positions, and the final generator state.
fn expected_drops(
    sp: &Spec,
    rand: &JavaRandom,
    x: i32,
    y: i32,
    z: i32,
    meta: i32,
    chance: f32,
) -> (Vec<(ItemStack, [f64; 3])>, i64) {
    let mut r = rand.clone();
    let mut out = Vec::new();
    let n = (sp.quantity)(&mut r);
    for _ in 0..n {
        if r.next_float() <= chance {
            let id = (sp.id_dropped)(sp.id, meta, &mut r);
            if id > 0 {
                let mut off = || (r.next_float() * 0.7f32) as f64 + (1.0f32 - 0.7f32) as f64 * 0.5;
                let p = [x as f64 + off(), y as f64 + off(), z as f64 + off()];
                out.push((ItemStack::new(id, 1, (sp.damage)(sp.id, meta)), p));
            }
        }
    }
    (out, r.state())
}

/// The `EntityItem`s spawned since index `from`, with their positions.
fn drops_since(w: &World, from: usize) -> Vec<(ItemStack, [f64; 3])> {
    w.loaded_entity_list[from..]
        .iter()
        .map(|slot| {
            let e = slot.entity.as_ref().expect("entity in its slot");
            let it = e.kind.item().expect("an EntityItem");
            assert_eq!(it.delay_before_can_pickup, 10, "Block.java:362");
            (
                it.item.expect("a stack"),
                [e.base.pos_x, e.base.pos_y, e.base.pos_z],
            )
        })
        .collect()
}

/// `dropBlockAsItemWithChance` for every block of the task, at chance 1 and
/// at the explosion chance 0.3 (Explosion.java:158), with several metas and
/// seeds so the gravel, lapis and glowstone draws are exercised.
#[test]
fn drop_block_as_item_with_chance_per_class() {
    for seed in [1i64, 7, 42, 123, 9001] {
        for sp in spec() {
            for (meta, chance) in [(0, 1.0f32), (5, 1.0), (14, 0.3), (3, 0.3)] {
                let mut w = empty_world(seed);
                for _ in 0..(sp.id + meta) {
                    w.rand.next_int(); // vary the start state per case
                }
                let (want, state) = expected_drops(&sp, &w.rand, 2, 61, -3, meta, chance);
                let from = w.loaded_entity_list.len();
                d::drop_block_as_item_with_chance(&mut w, sp.id, 2, 61, -3, meta, chance);
                let got = drops_since(&w, from);
                assert_eq!(
                    got.len(),
                    want.len(),
                    "id {} meta {meta} seed {seed}",
                    sp.id
                );
                for ((gs, gp), (ws, wp)) in got.iter().zip(&want) {
                    assert_eq!(gs, ws, "id {} meta {meta} seed {seed}", sp.id);
                    for (g, w) in gp.iter().zip(wp) {
                        assert_eq!(g.to_bits(), w.to_bits(), "id {} pos", sp.id);
                    }
                }
                assert_eq!(w.rand.state(), state, "id {} World.rand", sp.id);
                assert!(
                    w.unimplemented_hits.borrow().is_empty(),
                    "id {}: {:?}",
                    sp.id,
                    w.unimplemented_hits.borrow()
                );
            }
        }
    }
}

/// `harvestBlock` (Block.java:551-554): the mine stat and
/// `dropBlockAsItem` with chance 1. Glowstone: 2 + nextInt(3) dust.
#[test]
fn harvest_block_drops_like_drop_block_as_item() {
    let sp = spec().into_iter().find(|s| s.id == 89).unwrap();
    let mut w = empty_world(5);
    let mut p = new_entity_player_sp(&mut w);
    let (want, state) = expected_drops(&sp, &w.rand, 0, 61, 0, 0, 1.0);
    let from = w.loaded_entity_list.len();
    d::harvest_block(&mut w, 89, 0, 61, 0, &mut p, 0);
    let got = drops_since(&w, from);
    assert!((2..=4).contains(&got.len()));
    assert_eq!(
        got.iter().map(|g| g.0).collect::<Vec<_>>(),
        want.iter().map(|g| g.0).collect::<Vec<_>>()
    );
    assert_eq!(w.rand.state(), state);
    assert!(w.unimplemented_hits.borrow().is_empty());
    let mined = StatList::get().mine_block_stat(89).unwrap();
    assert_eq!(mined.stat_id, 16777216 + 89);
    assert_eq!(
        p.kind.player().unwrap().stat_file_writer.write_stat(mined),
        1
    );
}

/// `blockStrength` (Block.java:327-333) with tools from the item table:
/// `ItemPickaxe` harvest levels (ItemPickaxe.java:10-37) and efficiency
/// (`EnumToolMaterial` WOOD 2.0, EMERALD 8.0, EnumToolMaterial.java:4-7;
/// ItemTool.java:19-27, the effective list ItemPickaxe.java:40-42).
#[test]
fn block_strength_with_pickaxes() {
    let mut w = empty_world(1);
    let mut p = new_entity_player_sp(&mut w);
    p.base.on_ground = true;
    p.base.set_position(0.5, 80.0, 0.5);
    let hold = |p: &mut Entity, id: i32| {
        p.kind.player_mut().unwrap().inventory.main_inventory[0] = Some(ItemStack::new(id, 1, 0));
    };
    // obsidian (hardness 10): only harvest level 3 harvests it, and it is not
    // in the pickaxe's effective list, so the strength is 1.0.
    hold(&mut p, PICKAXE_WOOD);
    assert_eq!(d::block_strength(&mut w, 49, &p), 1.0f32 / 10.0 / 100.0);
    hold(&mut p, PICKAXE_DIAMOND);
    assert_eq!(d::block_strength(&mut w, 49, &p), 1.0f32 / 10.0 / 30.0);
    // stone (1.5): effective, any level harvests rock
    hold(&mut p, PICKAXE_WOOD);
    assert_eq!(d::block_strength(&mut w, 1, &p), 2.0f32 / 1.5 / 30.0);
    // diamond ore (3.0): level >= 2
    assert_eq!(d::block_strength(&mut w, 56, &p), 1.0f32 / 3.0 / 100.0);
    hold(&mut p, PICKAXE_DIAMOND);
    assert_eq!(d::block_strength(&mut w, 56, &p), 8.0f32 / 3.0 / 30.0);
    // glowstone is Material.rock: a pickaxe harvests it but is not effective
    assert_eq!(d::block_strength(&mut w, 89, &p), 1.0f32 / 0.3 / 30.0);
    // TNT: hardness 0 (Block.java:638), harvestable by hand: 1 / 0 / 30 = inf
    hold(&mut p, FLINT_AND_STEEL);
    assert_eq!(d::block_strength(&mut w, 46, &p), f32::INFINITY);
}

// ---- BlockTNT -----------------------------------------------------------------

/// `onBlockDestroyedByPlayer` with bit 1 clear drops one TNT item through
/// `dropBlockAsItem_do` (BlockTNT.java:47-48): three `World.rand` floats.
#[test]
fn tnt_broken_without_bit_1_drops_itself() {
    let mut w = empty_world(3);
    let mut r = w.rand.clone();
    let mut off = || (r.next_float() * 0.7f32) as f64 + (1.0f32 - 0.7f32) as f64 * 0.5;
    let pos = [4.0 + off(), 61.0 + off(), 4.0 + off()];
    let from = w.loaded_entity_list.len();
    d::on_block_destroyed_by_player(&mut w, 46, 4, 61, 4, 2);
    let got = drops_since(&w, from);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].0, ItemStack::new(46, 1, 0));
    for (g, p) in got[0].1.iter().zip(pos) {
        assert_eq!(g.to_bits(), p.to_bits());
    }
    assert_eq!(w.rand.state(), r.state());
    assert!(w.unimplemented_hits.borrow().is_empty());
}

/// With bit 1 set: an `EntityTNTPrimed` joins at the block centre with fuse
/// 80 and its `Math.random()` kick (EntityTNTPrimed.java:14-25), then the
/// fuse sound at its feet, `(float)y + 0.5F - 0.49F` (BlockTNT.java:50-52,
/// EntityTNTPrimed.java:10-11). Nothing at all on a multiplayer world.
#[test]
fn tnt_broken_with_bit_1_primes() {
    let mut w = empty_world(3);
    w.events.drain();
    let before = w.rand.state();
    let mut math = w.math_random.clone();
    let n = w.loaded_entity_list.len();
    let id = w.next_entity_id;
    d::on_block_destroyed_by_player(&mut w, 46, -2, 61, 5, 1);
    assert_eq!(w.loaded_entity_list.len(), n + 1);
    assert_eq!(w.rand.state(), before);
    assert!(w.unimplemented_hits.borrow().is_empty());
    let e = w.entity(id).unwrap();
    assert_eq!(e.kind.java_class(), "EntityTNTPrimed");
    assert_eq!(
        (e.base.pos_x, e.base.pos_y, e.base.pos_z),
        (-1.5, 61.5, 5.5)
    );
    let EntityKindData::TntPrimed(t) = &e.kind else {
        panic!()
    };
    assert_eq!(t.fuse, 80);
    let var8 = (math.next_double() * 3.1415927410125732 * 2.0) as f32;
    assert_eq!(
        e.base.motion_x,
        (-mc_core::math::sin(var8 * 3.1415927 / 180.0) * 0.02) as f64
    );
    assert_eq!(w.math_random, math);
    let ev: Vec<WorldEvent> = w
        .events
        .drain()
        .into_iter()
        .filter(|e| matches!(e, WorldEvent::PlaySound { .. }))
        .collect();
    assert_eq!(ev.len(), 1);
    match &ev[0] {
        WorldEvent::PlaySound {
            name,
            x,
            y,
            z,
            volume,
            pitch,
        } => {
            assert_eq!(name, "random.fuse");
            assert_eq!(*x, -1.5);
            assert_eq!(*y, 61.5 - 0.49f32 as f64);
            assert_eq!(*z, 5.5);
            assert_eq!((*volume, *pitch), (1.0, 1.0));
        }
        other => panic!("unexpected event {other:?}"),
    }

    let mut w = empty_world(3);
    w.multiplayer_world = true;
    w.events.drain();
    d::on_block_destroyed_by_player(&mut w, 46, 0, 61, 0, 0);
    d::on_block_destroyed_by_player(&mut w, 46, 0, 61, 0, 1);
    assert!(w.events.drain().is_empty());
    assert!(w.unimplemented_hits.borrow().is_empty());
    assert_eq!(w.loaded_entity_list.len(), n);
}

/// `onBlockDestroyedByExplosion` primes a TNT whose fuse is drawn from
/// `World.rand`, `nextInt(80 / 4) + 80 / 8` (BlockTNT.java:39-43).
#[test]
fn tnt_destroyed_by_explosion_draws_the_fuse() {
    let mut w = empty_world(11);
    let mut r = w.rand.clone();
    let fuse = r.next_int_bounded(20) + 10;
    let id = w.next_entity_id;
    d::on_block_destroyed_by_explosion(&mut w, 46, 0, 61, 0);
    assert_eq!(w.rand.state(), r.state());
    assert!(w.unimplemented_hits.borrow().is_empty());
    let Some(EntityKindData::TntPrimed(t)) = w.entity(id).map(|e| &e.kind) else {
        panic!()
    };
    assert_eq!(t.fuse, fuse);
}

/// Left-click with flint and steel sets metadata 1 without notification
/// (BlockTNT.java:58-64); anything else leaves it; right-click is false.
#[test]
fn tnt_clicked_with_flint_and_steel() {
    let mut w = empty_world(1);
    let mut p = new_entity_player_sp(&mut w);
    w.set_block(1, 61, 1, 46);
    w.set_block(1, 61, 2, 12); // sand next to it: a notification would schedule it
    let ticks = w.scheduled_ticks().count();
    d::on_block_clicked(&mut w, 46, 1, 61, 1, &mut p);
    assert_eq!(w.get_block_metadata(1, 61, 1), 0);
    p.kind.player_mut().unwrap().inventory.main_inventory[0] = Some(ItemStack::new(1, 1, 0));
    d::on_block_clicked(&mut w, 46, 1, 61, 1, &mut p);
    assert_eq!(w.get_block_metadata(1, 61, 1), 0);
    p.kind.player_mut().unwrap().inventory.main_inventory[0] =
        Some(ItemStack::new(FLINT_AND_STEEL, 1, 0));
    d::on_block_clicked(&mut w, 46, 1, 61, 1, &mut p);
    assert_eq!(w.get_block_metadata(1, 61, 1), 1);
    assert_eq!(w.scheduled_ticks().count(), ticks);
    assert!(!d::block_activated(&mut w, 46, 1, 61, 1, &mut p));
    assert!(w.unimplemented_hits.borrow().is_empty());
}

/// Unpowered TNT does nothing when placed or when a non-source neighbour
/// changes (BlockTNT.java:18-33). The powered paths need a power source
/// (redstone, WP-1.6c) and are not covered here.
#[test]
fn tnt_unpowered_stays() {
    let mut w = empty_world(1);
    w.set_block(0, 61, 0, 46);
    let before = w.rand.state();
    d::on_block_added(&mut w, 46, 0, 61, 0);
    d::on_neighbor_block_change(&mut w, 46, 0, 61, 0, 0);
    d::on_neighbor_block_change(&mut w, 46, 0, 61, 0, 1);
    assert_eq!(w.get_block_id(0, 61, 0), 46);
    assert_eq!(w.rand.state(), before);
    assert!(w.unimplemented_hits.borrow().is_empty());
}

// ---- BlockSoulSand, BlockWeb ---------------------------------------------------

#[test]
fn soul_sand_and_web_collision() {
    let mut w = empty_world(1);
    // BlockSoulSand.java:8-11: the top at (float)(y + 1) - 0.125F
    let bb = d::get_collision_bounding_box_from_pool(&mut w, 88, 3, 60, -4).unwrap();
    assert_eq!(
        (bb.min_x, bb.min_y, bb.min_z, bb.max_x, bb.max_y, bb.max_z),
        (3.0, 60.0, -4.0, 4.0, 60.875, -3.0)
    );
    // BlockWeb.java:18-20
    assert!(d::get_collision_bounding_box_from_pool(&mut w, 30, 3, 60, -4).is_none());

    let mut p = new_entity_player_sp(&mut w);
    p.base.motion_x = 1.0;
    p.base.motion_y = 1.0;
    p.base.motion_z = -0.5;
    d::on_entity_collided_with_block(&mut w, 88, 0, 60, 0, &mut p);
    assert_eq!(
        (p.base.motion_x, p.base.motion_y, p.base.motion_z),
        (0.4, 1.0, -0.2)
    );
    assert!(!p.base.is_in_web);
    d::on_entity_collided_with_block(&mut w, 30, 0, 60, 0, &mut p);
    assert!(p.base.is_in_web);
    assert!(w.unimplemented_hits.borrow().is_empty());
}

// ---- BlockPumpkin ----------------------------------------------------------------

/// BlockPumpkin.java:50-53: air or ground cover, over a normal cube.
#[test]
fn pumpkin_can_place_block_at() {
    let mut w = empty_world(1);
    w.set_block(0, 60, 0, 1);
    w.set_block(1, 60, 0, 20); // glass: not a normal cube
    w.set_block(2, 60, 0, 1);
    w.set_block(2, 61, 0, 78); // snow layer: ground cover (Material.java:129)
    w.set_block(3, 60, 0, 1);
    w.set_block(3, 61, 0, 1);
    assert!(d::can_place_block_at(&mut w, 86, 0, 61, 0));
    assert!(!d::can_place_block_at(&mut w, 86, 1, 61, 0));
    assert!(d::can_place_block_at(&mut w, 91, 2, 61, 0));
    assert!(!d::can_place_block_at(&mut w, 86, 3, 61, 0));
    assert!(!d::can_place_block_at(&mut w, 86, 4, 61, 0)); // air below
    assert!(w.unimplemented_hits.borrow().is_empty());
}

/// BlockPumpkin.java:55-58: `floor(yaw * 4 / 360 + 2.5) & 3`.
#[test]
fn pumpkin_faces_the_placer() {
    let mut w = empty_world(1);
    let mut p = new_entity_player_sp(&mut w);
    for (yaw, meta) in [
        (0.0f32, 2),
        (44.9, 2),
        (45.0, 3),
        (90.0, 3),
        (180.0, 0),
        (270.0, 1),
        (-90.0, 1),
        (-45.0, 2),
        (-45.1, 1),
        (720.0, 2),
    ] {
        w.set_block(0, 61, 0, 86);
        p.base.rotation_yaw = yaw;
        d::on_block_placed_by(&mut w, 86, 0, 61, 0, &p);
        assert_eq!(w.get_block_metadata(0, 61, 0), meta, "yaw {yaw}");
    }
    assert!(w.unimplemented_hits.borrow().is_empty());
}

// ---- BlockBreakable (glass), BlockSponge -----------------------------------------

/// BlockBreakable.java:15-18 with `localFlag` false (glass, Block.java:612):
/// no face against glass, else the `Block` rule.
#[test]
fn glass_hides_faces_against_glass() {
    let mut w = empty_world(1);
    w.set_block(0, 61, 0, 20);
    w.set_block(1, 61, 0, 20);
    w.set_block(0, 62, 0, 1);
    assert!(!d::should_side_be_rendered(&mut w, 20, 1, 61, 0, 5));
    assert!(!d::should_side_be_rendered(&mut w, 20, 0, 62, 0, 1));
    assert!(d::should_side_be_rendered(&mut w, 20, -1, 61, 0, 4));
    assert!(w.unimplemented_hits.borrow().is_empty());
}

/// BlockSponge.java:9-21: the water scan changes nothing and draws nothing.
#[test]
fn sponge_added_does_nothing() {
    let mut w = empty_world(1);
    w.set_block(1, 61, 0, 9); // still water (its onBlockAdded is the fluids task's)
    w.set_block(0, 61, 0, 19);
    w.unimplemented_hits.borrow_mut().clear();
    let ticks = w.scheduled_ticks().count();
    let before = w.rand.state();
    d::on_block_added(&mut w, 19, 0, 61, 0);
    assert_eq!(w.get_block_id(1, 61, 0), 9);
    assert_eq!(w.rand.state(), before);
    assert_eq!(w.scheduled_ticks().count(), ticks);
    assert!(w.unimplemented_hits.borrow().is_empty());
}
