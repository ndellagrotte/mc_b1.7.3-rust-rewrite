//! The light-update queue (`World.scheduleLightingUpdate_do`,
//! `updatingLighting`, World.java:1657-1730) beyond what the golden fixtures
//! show directly: the 499-updates-per-call budget, the merge window, the
//! dropped updates (no-sky dimension, unloaded centre column) and the
//! 1,000,000-entry overflow rule. The light values themselves are checked by
//! the `tests/golden/light/*` fixtures (`golden_tick.rs`).

use mc_golden::{TraceLine, parse_jsonl};
use mc_sim::save::NoSave;
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::lighting::{EnumSkyBlock, LIGHTING_QUEUE_LIMIT};
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::{World, WorldEnvironment};

fn empty_world(dim: Dimension, seed: i64) -> World {
    let mut w = World::new(
        Box::new(NoSave),
        "harness",
        WorldProvider::new(dim),
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

/// The implicit preload of one position: the r = 2 chunk square, draining
/// the light queue after each chunk.
fn preload(w: &mut World, x: i32, z: i32) {
    for dz in -2..=2 {
        for dx in -2..=2 {
            w.get_chunk_from_chunk_coords((x >> 4) + dx, (z >> 4) + dz);
            while w.updating_lighting() {}
        }
    }
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

/// `n` one-block sky updates in chunk (0, 0) at positions two apart, so no
/// two can merge, high in open air where they change nothing.
fn queue_isolated(w: &mut World, n: usize) {
    for i in 0..n {
        let (x, z, y) = (
            2 * (i % 8) as i32,
            2 * (i / 8 % 8) as i32,
            64 + 2 * (i / 64) as i32,
        );
        assert!(y < 128);
        w.schedule_lighting_update(EnumSkyBlock::Sky, x, y, z, x, y, z);
    }
}

#[test]
fn updating_lighting_runs_at_most_499_per_call() {
    let mut w = empty_world(Dimension::Surface, 123);
    preload(&mut w, 0, 0);
    assert!(w.lighting_to_update().is_empty());
    queue_isolated(&mut w, 600);
    assert_eq!(w.lighting_to_update().len(), 600);
    // `--var1` before the check `var1 <= 0` with var1 = 500: 499 updates.
    assert!(w.updating_lighting(), "budget ran out");
    assert_eq!(w.lighting_to_update().len(), 101);
    // LIFO: the oldest 101 remain, oldest first.
    assert_eq!(w.lighting_to_update()[0].min_x, 0);
    assert_eq!(w.lighting_to_update()[0].min_y, 64);
    assert!(!w.updating_lighting(), "drained");
    assert!(w.lighting_to_update().is_empty());

    // Exactly 499 pending: the 499th update runs, then the empty queue ends
    // the loop before the budget check: false.
    queue_isolated(&mut w, 499);
    assert!(!w.updating_lighting());
    assert!(w.lighting_to_update().is_empty());
    // 500 pending: one is left over and the call reports more work.
    queue_isolated(&mut w, 500);
    assert!(w.updating_lighting());
    assert_eq!(w.lighting_to_update().len(), 1);
}

#[test]
fn merge_window_is_the_five_newest_of_the_same_type() {
    let mut w = empty_world(Dimension::Surface, 123);
    preload(&mut w, 0, 0);
    w.schedule_lighting_update(EnumSkyBlock::Block, 4, 70, 4, 4, 70, 4);
    // Same box, other type: not merged.
    w.schedule_lighting_update(EnumSkyBlock::Sky, 4, 70, 4, 4, 70, 4);
    assert_eq!(w.lighting_to_update().len(), 2);
    // Adjacent block of the same type, behind one newer entry: grown in place.
    w.schedule_lighting_update(EnumSkyBlock::Block, 5, 70, 4, 5, 70, 4);
    assert_eq!(w.lighting_to_update().len(), 2);
    let m = w.lighting_to_update()[0];
    assert_eq!(
        (m.min_x, m.min_y, m.min_z, m.max_x, m.max_y, m.max_z),
        (4, 70, 4, 5, 70, 4)
    );
    // Five isolated Sky updates push the Block box out of the window.
    queue_isolated(&mut w, 5);
    w.schedule_lighting_update(EnumSkyBlock::Block, 4, 70, 4, 4, 70, 4);
    assert_eq!(w.lighting_to_update().len(), 8);
    // With merge = false nothing is offered.
    w.schedule_lighting_update_do(EnumSkyBlock::Block, 4, 70, 4, 4, 70, 4, false);
    assert_eq!(w.lighting_to_update().len(), 9);
}

#[test]
fn dropped_updates() {
    // No sky: Sky updates and neighbour checks are ignored, Block ones kept.
    let mut w = empty_world(Dimension::Hell, 123);
    preload(&mut w, 0, 0);
    w.schedule_lighting_update(EnumSkyBlock::Sky, 1, 70, 1, 1, 70, 1);
    assert!(w.lighting_to_update().is_empty());
    w.schedule_lighting_update(EnumSkyBlock::Block, 1, 70, 1, 1, 70, 1);
    assert_eq!(w.lighting_to_update().len(), 1);

    // Centre column ((x0 + x1) / 2, (z0 + z1) / 2) not loaded: dropped, even
    // when a corner is loaded. Chunks -2..2 are loaded around (0, 0).
    let mut w = empty_world(Dimension::Surface, 123);
    preload(&mut w, 0, 0);
    w.schedule_lighting_update(EnumSkyBlock::Block, 40, 70, 0, 60, 70, 0);
    assert!(w.lighting_to_update().is_empty());
    w.schedule_lighting_update(EnumSkyBlock::Block, 30, 70, 0, 60, 70, 0);
    assert_eq!(w.lighting_to_update().len(), 1);
    // Java division truncates toward zero: (-65 + 0) / 2 = -32 is chunk -2
    // (loaded), where a floor would give -33, chunk -3 (not loaded).
    w.schedule_lighting_update(EnumSkyBlock::Block, -65, 70, 0, 0, 70, 0);
    assert_eq!(w.lighting_to_update().len(), 2);
    // (-67 + 0) / 2 = -33: chunk -3, dropped.
    w.schedule_lighting_update(EnumSkyBlock::Block, -67, 70, 0, 0, 70, 0);
    assert_eq!(w.lighting_to_update().len(), 2);
}

#[test]
fn more_than_a_million_updates_clear_the_queue() {
    let mut w = empty_world(Dimension::Surface, 123);
    preload(&mut w, 0, 0);
    // A cycle of six isolated positions: each new box is only compared with
    // the five newest, which are the other five.
    for i in 0..LIGHTING_QUEUE_LIMIT {
        let x = 2 * (i % 6) as i32;
        w.schedule_lighting_update(EnumSkyBlock::Block, x, 70, 0, x, 70, 0);
    }
    assert_eq!(w.lighting_to_update().len(), LIGHTING_QUEUE_LIMIT);
    w.schedule_lighting_update(EnumSkyBlock::Block, 12, 70, 0, 12, 70, 0);
    assert!(w.lighting_to_update().is_empty(), "size > 1000000 clears");
}

/// `tests/golden/light/skylight-column` shows that the light queue does not
/// drain in one tick (it does not pin the exact budget: 498 or 500 updates
/// per call still match it; `light/blocklight-glowstone` is the fixture that
/// fails at either). Replay the ops and check the queue after each tick; the
/// fixture's t = 0 and t = 1 sky light dumps differ.
#[test]
fn skylight_column_fill_does_not_drain_in_one_tick() {
    let mut w = empty_world(Dimension::Surface, 123);
    for (x, z) in [(-4, -4), (4, 4)] {
        preload(&mut w, x, z);
    }
    // The driver preloads around each op position; the eight pillar ops all
    // sit at (2, y, 2), so one preload covers them.
    preload(&mut w, 2, 2);
    for x in -4..=4 {
        for z in -4..=4 {
            w.set_block_and_metadata(x, 60, z, 1, 0);
        }
    }
    let queued = w.lighting_to_update().len();
    run_tick(&mut w);
    let after0 = w.lighting_to_update().len();
    assert!(after0 > 0, "t=0 left nothing (queued {queued})");
    let mut drained_at = None;
    for t in 1..5 {
        run_tick(&mut w);
        if w.lighting_to_update().is_empty() {
            drained_at = Some(t);
            break;
        }
    }
    let drained_at = drained_at.expect("drained before the pillar at t=5");
    eprintln!(
        "skylight-column: {queued} queued by the fill, {after0} left after t=0, drained at t={drained_at}"
    );

    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/golden/light/skylight-column/expected.jsonl"
    ))
    .expect("read expected.jsonl");
    let lines: Vec<TraceLine> = parse_jsonl(&text).expect("parse");
    let sky_at = |t: i64| {
        lines
            .iter()
            .find(|l| l.t() == t && l.get("e").and_then(|v| v.as_str()) == Some("blocks_full"))
            .and_then(|l| l.get("skylight"))
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .expect("blocks_full line")
    };
    assert_ne!(
        sky_at(0),
        sky_at(1),
        "the oracle also needed more than one tick"
    );
}
