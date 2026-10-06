//! WP-0.5 skeleton tests: the world, chunk, block-table, event, entity-list
//! and entity-physics plumbing on an empty world, against hand-derived values.

use mc_sim::entity::item::new_entity_item;
use mc_sim::entity::{EntityKindData, dispatch};
use mc_sim::item::ItemStack;
use mc_sim::save::NoSave;
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::events::WorldEvent;
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::{World, WorldEnvironment};

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

/// The harness's implicit preload: the 5×5 chunk square around chunk (cx, cz).
fn preload(w: &mut World, cx: i32, cz: i32) {
    for dz in -2..=2 {
        for dx in -2..=2 {
            w.get_chunk_from_chunk_coords(cx + dx, cz + dz);
        }
    }
}

#[test]
fn empty_chunk_matches_generate_skylight_map() {
    let mut w = empty_world(123);
    w.get_chunk_from_chunk_coords(0, 0);
    let a = w.chunk_arrays(0, 0).unwrap();
    assert!(a.blocks.iter().all(|&b| b == 0));
    assert!(a.heightmap.iter().all(|&h| h == 0));
    // sky light 15 for y = 127..1, 0 at y = 0 (tests/golden/README.md "World").
    let d = w.block_dump(3, 0, 5).unwrap();
    assert_eq!(d.skylight, 0);
    for y in 1..128 {
        assert_eq!(w.block_dump(3, y, 5).unwrap().skylight, 15, "y = {y}");
    }
    assert!(!w.loaded_chunk(0, 0).unwrap().is_terrain_populated);
    // Loading the +x, +z, +xz neighbours populates (0, 0) (ChunkProvider.java:52-54).
    w.get_chunk_from_chunk_coords(1, 0);
    w.get_chunk_from_chunk_coords(0, 1);
    w.get_chunk_from_chunk_coords(1, 1);
    assert!(w.loaded_chunk(0, 0).unwrap().is_terrain_populated);
    assert!(w.chunk_arrays(5, 5).is_none(), "dumps never load");
}

#[test]
fn set_block_get_block_round_trip_and_height_map() {
    let mut w = empty_world(1);
    preload(&mut w, 0, 0);
    assert!(w.set_block_and_metadata(-3, 60, 7, 1, 0));
    assert_eq!(w.get_block_id(-3, 60, 7), 1);
    assert_eq!(w.get_block_metadata(-3, 60, 7), 0);
    // Same id and metadata again: no change (Chunk.java:240-241).
    assert!(!w.set_block_and_metadata(-3, 60, 7, 1, 0));
    assert!(w.set_block_and_metadata(-3, 61, 7, 35, 14));
    assert_eq!(w.get_block_metadata(-3, 61, 7), 14);
    // relightBlock: the column's height is one above the top opaque block,
    // sky light 0 below it, 15 above (Chunk.java:158-230).
    assert_eq!(w.get_height_value(-3, 7), 62);
    assert_eq!(w.block_dump(-3, 59, 7).unwrap().skylight, 0);
    assert_eq!(w.block_dump(-3, 62, 7).unwrap().skylight, 15);
    // Out of range.
    assert!(!w.set_block(0, 128, 0, 1));
    assert_eq!(w.get_block_id(0, -1, 0), 0);
    assert_eq!(w.get_block_id(32_000_000, 64, 0), 0);
    // The light queue is a stub that records itself.
    assert!(
        w.skipped_systems
            .borrow()
            .contains_key("World.scheduleLightingUpdate")
    );
}

#[test]
fn notify_pushes_events_and_records_untranscribed_overrides() {
    let mut w = empty_world(1);
    preload(&mut w, 0, 0);
    w.events.drain();
    // Sponge: BlockSponge.onBlockAdded is not transcribed yet.
    assert!(w.set_block_with_notify(0, 61, 0, 19));
    let ev = w.events.drain();
    assert!(ev.contains(&WorldEvent::MarkBlockAndNeighborsNeedsUpdate { x: 0, y: 61, z: 0 }));
    assert!(w.events.is_empty());
    assert_eq!(
        w.unimplemented_hits
            .borrow()
            .get("BlockSponge.onBlockAdded"),
        Some(&1)
    );
    // Sand is transcribed: it schedules its fall check instead (BlockSand.java:12-14).
    assert!(w.set_block_with_notify(2, 61, 0, 12));
    assert!(
        !w.unimplemented_hits
            .borrow()
            .contains_key("BlockSand.onBlockAdded")
    );
    assert!(
        w.scheduled_ticks()
            .any(|t| (t.x_coord, t.y_coord, t.block_id) == (2, 61, 12))
    );
}

#[test]
fn scheduled_ticks_run_in_tree_set_order() {
    let mut w = empty_world(7);
    preload(&mut w, 0, 0);
    w.set_block_and_metadata(0, 61, 0, 1, 0);
    w.set_block_and_metadata(1, 61, 0, 1, 0);
    w.schedule_block_update(0, 61, 0, 1, 5);
    w.schedule_block_update(1, 61, 0, 1, 2);
    // Same position and id: dropped (scheduledTickSet.contains, World.java:1169),
    // but the entry id was still consumed.
    w.schedule_block_update(1, 61, 0, 1, 1);
    let order: Vec<(i32, i64)> = w
        .scheduled_ticks()
        .map(|e| (e.x_coord, e.scheduled_time))
        .collect();
    assert_eq!(order, vec![(1, 2), (0, 5)]);
    assert_eq!(w.next_tick_entry_id, 3);
    assert_eq!(w.scheduled_ticks_bytes().len(), 48);
    // Nothing is due at time 0.
    assert!(w.tick_updates(false));
    w.set_world_time(2);
    assert!(w.tick_updates(false));
    assert_eq!(w.scheduled_ticks().count(), 1);
    w.set_world_time(5);
    assert!(!w.tick_updates(false));
}

#[test]
fn spawn_assigns_ids_in_construction_order_and_keeps_list_order() {
    let mut w = empty_world(42);
    preload(&mut w, 0, 0);
    let a = new_entity_item(&mut w, 0.5, 70.0, 0.5, ItemStack::new(4, 1, 0));
    let b = new_entity_item(&mut w, 1.5, 70.0, 0.5, ItemStack::new(4, 2, 0));
    // A chunk that is not loaded: refused, and the id is still used.
    let lost = new_entity_item(&mut w, 1000.5, 70.0, 0.5, ItemStack::new(4, 3, 0));
    let c = new_entity_item(&mut w, 2.5, 70.0, 0.5, ItemStack::new(4, 4, 0));
    assert_eq!((a.id(), b.id(), lost.id(), c.id()), (0, 1, 2, 3));
    assert!(w.spawn_entity_in_world(b));
    assert!(w.spawn_entity_in_world(a));
    assert!(!w.spawn_entity_in_world(lost));
    assert!(w.spawn_entity_in_world(c));
    let ids: Vec<i32> = w.loaded_entity_list.iter().map(|s| s.entity_id).collect();
    assert_eq!(ids, vec![1, 0, 3]);
    // Joining reseeds rand with mix(seed, id) (tests/golden/README.md "Seeding").
    let e = w.entity(3).unwrap();
    assert_eq!(
        e.base.rand.state(),
        mc_core::JavaRandom::new(w.entity_rand_seed_for(3)).state()
    );
    assert!(e.base.added_to_chunk);
    assert_eq!(w.loaded_chunk(0, 0).unwrap().entities[4], vec![1, 0, 3]);
    let joined = w
        .events
        .drain()
        .into_iter()
        .filter(|e| matches!(e, WorldEvent::ObtainEntitySkin { .. }))
        .count();
    assert_eq!(joined, 3);
}

#[test]
fn entity_item_falls_one_tick_in_empty_air() {
    let mut w = empty_world(5);
    preload(&mut w, 0, 0);
    let mut e = new_entity_item(&mut w, 0.5, 70.0, 0.5, ItemStack::new(4, 1, 0));
    // The `set_motion` op.
    let (mx0, my0, mz0) = (0.1, 0.2, 0.05);
    e.base.set_velocity(mx0, my0, mz0);
    assert!(w.spawn_entity_in_world(e));

    w.update_entities();

    let e = w.entity(0).unwrap();
    // EntityItem.onUpdate (EntityItem.java:45, :64-66), moveEntity with no
    // collision: the box moves by the motion and posY is rebuilt from it
    // (Entity.java:453-455; yOffset = height / 2 = 0.125, ySize = 0).
    let dy = my0 - 0.03999999910593033;
    let y_offset = 0.25f32 / 2.0;
    let min_y = 70.0 - y_offset as f64 + 0.0 + dy;
    let half = 0.25f32 / 2.0;
    let min_x = 0.5 - half as f64 + mx0;
    let max_x = 0.5 + half as f64 + mx0;
    assert_eq!(e.base.pos_y, min_y + y_offset as f64 - 0.0);
    assert_eq!(e.base.pos_x, (min_x + max_x) / 2.0);
    assert_eq!(e.base.motion_y, dy * 0.9800000190734863);
    assert_eq!(e.base.motion_x, mx0 * 0.98f32 as f64);
    assert_eq!(e.base.motion_z, mz0 * 0.98f32 as f64);
    assert!(!e.base.on_ground);
    // Moving up (dy > 0): updateFallState leaves fallDistance alone (Entity.java:552).
    assert!(dy > 0.0);
    assert_eq!(e.base.fall_distance, 0.0);
    assert_eq!(e.base.ticks_existed, 1);
    assert_eq!(e.base.prev_pos_y, 70.0);
    assert_eq!(e.base.fire, -1);
    match &e.kind {
        EntityKindData::Item(d) => {
            assert_eq!(d.age, 1);
            assert_eq!(d.delay_before_can_pickup, 0);
        }
        _ => unreachable!(),
    }
    assert!(
        w.unimplemented_hits.borrow().is_empty(),
        "{:?}",
        w.unimplemented_hits.borrow()
    );
}

#[test]
fn entity_item_lands_on_stone_and_bounces() {
    let mut w = empty_world(9);
    preload(&mut w, 0, 0);
    for x in -2..=2 {
        for z in -2..=2 {
            w.set_block(x, 60, z, 1);
        }
    }
    // posY = 61.125: the box bottom (posY - yOffset) rests on the stone top.
    let mut e = new_entity_item(&mut w, 0.5, 61.125, 0.5, ItemStack::new(4, 1, 0));
    e.base.set_velocity(0.1, -0.5, 0.0);
    assert!(w.spawn_entity_in_world(e));
    w.update_entities();
    let e = w.entity(0).unwrap();
    // The stone top (y = 61) stops the box: minY = 61, posY = 61.125.
    assert!(e.base.on_ground);
    assert!(e.base.is_collided_vertically);
    assert_eq!(e.base.pos_y, 61.0 + 0.125);
    // motionY was zeroed by the collision (Entity.java:465-467), then
    // 0.0 * 0.98 * -0.5 = -0.0 (EntityItem.java:65-69).
    assert_eq!(e.base.motion_y.to_bits(), (-0.0f64).to_bits());
    assert_eq!(e.base.fall_distance, 0.0);
    // Ground friction: stone's slipperiness 0.6F * 0.98F (EntityItem.java:57-61).
    assert_eq!(e.base.motion_x, 0.1 * (0.6f32 * 0.98f32) as f64);
    assert!(!dispatch::can_trigger_walking(e));
}

#[test]
fn world_tick_advances_time_and_draws_weather_timers() {
    let mut w = empty_world(123);
    preload(&mut w, 0, 0);
    let before = w.rng_state();
    w.update_entities();
    w.tick();
    assert_eq!(w.get_world_time(), 1);
    // updateWeather on a new world draws thunder then rain timers
    // (World.java:1799-1823): two nextInt calls on World.rand.
    let mut r = mc_core::JavaRandom::new(123);
    let thunder = r.next_int_bounded(168000) + 12000;
    let rain = r.next_int_bounded(168000) + 12000;
    assert_eq!(w.world_info.thunder_time, thunder);
    assert_eq!(w.world_info.rain_time, rain);
    // No player: no random-tick chunks, so the LCG did not move.
    assert_eq!(w.rng_state().lcg, before.lcg);
    assert_eq!(w.rng_state().world_rand, r.state());
}
