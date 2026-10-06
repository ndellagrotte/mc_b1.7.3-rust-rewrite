//! `BlockSand` + `EntityFallingSand` against `tests/golden/tick/flat-sand-falls`,
//! replayed by hand (the official `mc_golden::run` test is in
//! `golden_tick.rs`): the scheduled tick spawns the entity, it falls with the
//! oracle's exact `posY`/`motionY` bits every tick, lands on the stone floor
//! and becomes a block again.

use std::collections::BTreeMap;

use mc_golden::{TraceLine, parse_jsonl};
use mc_sim::block::sand;
use mc_sim::entity::EntityKindData;
use mc_sim::entity::falling_sand::{self, FallingSandData};
use mc_sim::save::NoSave;
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::{World, WorldEnvironment};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/golden/tick/flat-sand-falls/expected.jsonl"
);

/// An empty-generator overworld seeded like the oracle's (`"world": "empty"`).
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
    w
}

/// `fill` op: `setBlockAndMetadata` over the box, x outer, z middle, y inner.
fn fill(w: &mut World, (x0, y0, z0): (i32, i32, i32), (x1, y1, z1): (i32, i32, i32), id: i32) {
    for x in x0..=x1 {
        for z in z0..=z1 {
            for y in y0..=y1 {
                w.set_block_and_metadata(x, y, z, id, 0);
            }
        }
    }
}

/// The scenario's world after the t = 0 ops (README "Tick loop" steps 1-2),
/// with the implicit preload of every op position (r = 2 chunk squares).
fn scenario_world() -> World {
    let mut w = empty_world(123);
    let points = [
        (-8, -8),
        (8, 8),
        (-8, -8),
        (8, -8),
        (-8, 8),
        (8, 8),
        (-8, -7),
        (-8, 7),
        (8, -7),
        (8, 7),
        (0, 0),
    ];
    for (x, z) in points {
        for dz in -2..=2 {
            for dx in -2..=2 {
                w.get_chunk_from_chunk_coords((x >> 4) + dx, (z >> 4) + dz);
                while w.updating_lighting() {}
            }
        }
    }
    fill(&mut w, (-8, 60, -8), (8, 60, 8), 1);
    fill(&mut w, (-8, 61, -8), (8, 61, -8), 1);
    fill(&mut w, (-8, 61, 8), (8, 61, 8), 1);
    fill(&mut w, (-8, 61, -7), (-8, 61, 7), 1);
    fill(&mut w, (8, 61, -7), (8, 61, 7), 1);
    assert!(w.set_block_and_metadata_with_notify(0, 66, 0, 12, 0));
    w
}

/// README "Tick loop" steps 4-8 without a player, difficulty 0.
fn run_tick(w: &mut World) {
    w.difficulty_setting = 0;
    if w.field_27172_i > 0 {
        w.field_27172_i -= 1;
    }
    w.update_entities();
    w.set_allowed_mob_spawns(false, true);
    w.tick();
    w.updating_lighting();
}

/// A bit-pattern string field (`"0x…"`) as raw bits.
fn bits(l: &TraceLine, k: &str) -> u64 {
    let s = l
        .get(k)
        .and_then(|v| v.as_str())
        .expect("bit-pattern string");
    u64::from_str_radix(s.trim_start_matches("0x"), 16).unwrap()
}

fn int(l: &TraceLine, k: &str) -> i64 {
    l.get(k).and_then(|v| v.as_i64()).expect("integer field")
}

fn boolean(l: &TraceLine, k: &str) -> bool {
    l.get(k).and_then(|v| v.as_bool()).expect("boolean field")
}

#[test]
fn flat_sand_falls_matches_the_oracle_tick_by_tick() {
    let lines = parse_jsonl(&std::fs::read_to_string(FIXTURE).unwrap()).unwrap();
    let mut entity_lines = BTreeMap::new();
    let mut block_lines = BTreeMap::new();
    for l in &lines {
        match l.e() {
            "entity" => {
                assert!(entity_lines.insert(l.t(), l).is_none(), "one entity");
            }
            "block" => {
                block_lines.insert((l.t(), int(l, "y")), int(l, "id"));
            }
            _ => {}
        }
    }
    assert_eq!(entity_lines.len(), 17, "fixture has the sand in t = 2..=18");

    let mut w = scenario_world();
    let mut compared = 0;
    for t in 0..40i64 {
        run_tick(&mut w);

        // Block ids at the two dumped positions (light is WP-1.3's).
        for y in [61i64, 66] {
            let want = block_lines[&(t, y)];
            let got = w.block_dump(0, y as i32, 0).unwrap().id as i64;
            assert_eq!(got, want, "t = {t}: block id at (0, {y}, 0)");
        }

        match entity_lines.get(&t) {
            None => assert!(
                w.loaded_entity_list.is_empty(),
                "t = {t}: the oracle has no entity"
            ),
            Some(l) => {
                assert_eq!(w.loaded_entity_list.len(), 1, "t = {t}");
                let e = w.loaded_entity_list[0].entity.as_deref().unwrap();
                assert_eq!(e.kind.java_class(), "EntityFallingSand");
                let b = &e.base;
                let f = |k: &str| f64::from_bits(bits(l, k));
                let checks = [
                    ("posX", b.pos_x, f("posX")),
                    ("posY", b.pos_y, f("posY")),
                    ("posZ", b.pos_z, f("posZ")),
                    ("prevPosX", b.prev_pos_x, f("prevPosX")),
                    ("prevPosY", b.prev_pos_y, f("prevPosY")),
                    ("prevPosZ", b.prev_pos_z, f("prevPosZ")),
                    ("motionX", b.motion_x, f("motionX")),
                    ("motionY", b.motion_y, f("motionY")),
                    ("motionZ", b.motion_z, f("motionZ")),
                ];
                for (k, got, want) in checks {
                    assert_eq!(
                        got.to_bits(),
                        want.to_bits(),
                        "t = {t}: {k} {got} vs {want}"
                    );
                }
                assert_eq!(
                    b.fall_distance.to_bits(),
                    bits(l, "fallDistance") as u32,
                    "t = {t}: fallDistance"
                );
                assert_eq!(b.on_ground, boolean(l, "onGround"), "t = {t}");
                assert_eq!(b.is_dead, boolean(l, "isDead"), "t = {t}");
                assert_eq!(b.ticks_existed as i64, int(l, "ticksExisted"), "t = {t}");
                assert_eq!(b.rand.state(), int(l, "rand"), "t = {t}: rand");
                compared += 1;
            }
        }
    }
    assert_eq!(compared, 17);
    // Landed: the sand is a block on the floor (y = 60 is stone), the entity
    // gone, and the untranscribed-override counter never saw sand.
    assert_eq!(w.block_dump(0, 61, 0).unwrap().id, 12);
    assert_eq!(w.block_dump(0, 66, 0).unwrap().id, 0);
    let hits = w.unimplemented_hits.borrow();
    assert!(
        hits.keys().all(|k| !k.contains("Sand")),
        "untranscribed sand paths: {hits:?}"
    );
}

#[test]
fn falling_sand_constructor_and_nbt() {
    let mut w = empty_world(1);
    let e = falling_sand::new_entity_falling_sand(&mut w, 0.5, 66.5, 0.5, 13);
    assert_eq!(e.base.width, 0.98);
    assert_eq!(e.base.y_offset, 0.49);
    assert!(e.base.prevent_entity_spawning);
    assert_eq!(e.base.bounding_box.min_y, 66.5 - 0.49f32 as f64);
    assert_eq!((e.base.prev_pos_y, e.base.motion_y), (66.5, 0.0));
    let EntityKindData::FallingSand(d) = &e.kind else {
        panic!("kind")
    };
    assert_eq!((d.block_id, d.fall_time), (13, 0));

    let mut m = BTreeMap::new();
    falling_sand::write_entity_to_nbt(
        &FallingSandData {
            block_id: 200,
            fall_time: 7,
        },
        &mut m,
    );
    let mut back = FallingSandData::default();
    falling_sand::read_entity_from_nbt(&mut back, &m);
    assert_eq!(back.block_id, 200, "(byte) then & 255");
}

#[test]
fn fall_instantly_moves_the_block_down_at_once() {
    let mut w = empty_world(5);
    for dz in -2..=2 {
        for dx in -2..=2 {
            w.get_chunk_from_chunk_coords(dx, dz);
        }
    }
    w.set_block_and_metadata(3, 40, 3, 1, 0);
    w.set_block_and_metadata(3, 41, 3, 9, 0); // still water: sand falls through
    w.block_statics.fall_instantly = true;
    assert!(w.set_block_and_metadata(3, 50, 3, 13, 0));
    // onBlockAdded scheduled the tick; run it now.
    let at = w.get_world_time() + 3;
    w.set_world_time(at);
    w.tick_updates(false);
    assert!(w.loaded_entity_list.is_empty());
    assert_eq!(w.get_block_id(3, 50, 3), 0);
    assert_eq!(w.get_block_id(3, 41, 3), 13, "replaces the water");
    assert!(sand::can_fall_below(&mut w, 3, 42, 3));
    assert!(!sand::can_fall_below(&mut w, 3, 40, 3));
}
