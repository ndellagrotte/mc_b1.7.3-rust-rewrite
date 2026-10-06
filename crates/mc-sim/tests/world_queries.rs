//! WP-1.5 unit tests for the `World` queries that no golden dump covers yet
//! (`getBlockDensity`, entity class queries, the spawn point, the loading
//! constructor, the player constructor), against values derived by hand from
//! the Java. Ray tracing and the power queries are pinned by
//! `tests/golden/tick/ray-trace-platform`.

use mc_core::{Aabb, Vec3D};
use mc_sim::entity::item::new_entity_item;
use mc_sim::entity::player::new_entity_player_sp;
use mc_sim::item::ItemStack;
use mc_sim::save::NoSave;
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::is_instance_of;
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::{World, WorldEnvironment};

const ENV: WorldEnvironment = WorldEnvironment {
    rand_seed: 0,
    field_9437_g: 0,
    lock_timestamp: 0,
};

fn empty_world(seed: i64) -> World {
    let mut w = World::new(
        Box::new(NoSave),
        "harness",
        WorldProvider::new(Dimension::Surface),
        seed,
        ChunkGenerator::Empty(EmptyChunkProvider),
        ENV,
    );
    w.make_deterministic(seed);
    for dz in -2..=2 {
        for dx in -2..=2 {
            w.get_chunk_from_chunk_coords(dx, dz);
        }
    }
    w
}

#[test]
fn block_density_open_and_walled() {
    let mut w = empty_world(1);
    let bb = Aabb::new(0.2, 61.0, 0.2, 0.8, 62.8, 0.8);
    let from = Vec3D::create_vector(4.5, 61.5, 0.5);
    // Nothing in between: every sample point sees the explosion.
    assert_eq!(w.get_block_density(&from, &bb), 1.0);
    // A stone wall at x = 2 covering the box's whole height: none does.
    for y in 59..66 {
        for z in -3..4 {
            w.set_block(2, y, z, 1);
        }
    }
    assert_eq!(w.get_block_density(&from, &bb), 0.0);
}

#[test]
fn ray_trace_mutates_the_start_vector() {
    let mut w = empty_world(1);
    w.set_block(0, 60, 0, 1);
    let mut start = Vec3D::create_vector(0.5, 65.0, 0.5);
    let end = Vec3D::create_vector(0.5, 55.0, 0.5);
    let hit = w
        .ray_trace_blocks(&mut start, &end)
        .expect("hits the stone");
    assert_eq!(
        (hit.block_x, hit.block_y, hit.block_z, hit.side_hit),
        (0, 60, 0, 1)
    );
    assert_eq!(hit.hit_vec.y_coord, 61.0);
    // World.java:809-811: the walk wrote the last crossed face into var1.
    assert_eq!(start.y_coord, 61.0);
}

#[test]
fn entity_class_queries() {
    let mut w = empty_world(7);
    let item = new_entity_item(&mut w, 0.5, 61.0, 0.5, ItemStack::new(4, 1, 0));
    let p = new_entity_player_sp(&mut w);
    assert!(is_instance_of(&item.kind, "EntityItem"));
    assert!(is_instance_of(&item.kind, "Entity"));
    assert!(!is_instance_of(&item.kind, "EntityLiving"));
    assert!(is_instance_of(&p.kind, "EntityPlayer"));
    assert!(is_instance_of(&p.kind, "EntityLiving"));
    assert!(!is_instance_of(&p.kind, "IMob"));
    let pid = p.id();
    assert!(w.spawn_entity_in_world(item));
    assert!(w.spawn_entity_in_world(p));
    assert_eq!(w.count_entities("Entity"), 2);
    assert_eq!(w.count_entities("EntityAnimal"), 0);
    assert_eq!(w.player_entities, vec![pid]);
    let bb = Aabb::new(-5.0, 0.0, -5.0, 5.0, 70.0, 5.0);
    let all = w.get_entities_within_aabb("Entity", &bb, None);
    assert_eq!(all.len(), 2);
    let players = w.get_entities_within_aabb("EntityPlayer", &bb, None);
    assert_eq!(players, vec![pid]);
    // An entity out of its slot for its update (a player on a pressure plate,
    // BlockPressurePlate.java:93) is found when passed in hand, at its place
    // in the chunk scan; without it the miss is recorded.
    let in_hand = w
        .with_entity_mut(pid, |w, p| {
            (
                w.get_entities_within_aabb("Entity", &bb, Some(p)),
                w.get_entities_within_aabb("EntityPlayer", &bb, None),
            )
        })
        .unwrap();
    assert_eq!(in_hand.0, all);
    assert!(in_hand.1.is_empty());
    assert_eq!(
        w.unimplemented_hits
            .borrow()
            .get("World.getEntitiesWithinAABB over a taken-out entity"),
        Some(&1)
    );
    assert_eq!(w.get_closest_player(0.0, 0.0, 0.0, -1.0), Some(pid));
    assert_eq!(w.get_player_entity_by_name("Player"), None);
}

#[test]
fn player_constructor_pure_part() {
    let mut w = empty_world(9);
    w.world_info.set_spawn(10, 64, -3);
    let before = w.math_random.clone();
    let p = new_entity_player_sp(&mut w);
    // Four Math.random() draws (EntityLiving.java:38, :63, :65, :66).
    let mut r = before;
    for _ in 0..4 {
        r.next_double();
    }
    assert_eq!(w.math_random.next_double(), r.next_double());
    // setLocationAndAngles(spawn + (0.5, 1, 0.5), 0, 0) with yOffset 1.62.
    assert_eq!(p.base.pos_x, 10.5);
    assert_eq!(p.base.pos_y, 65.0 + 1.62f32 as f64);
    assert_eq!(p.base.pos_z, -2.5);
    assert_eq!(p.base.rotation_yaw, 0.0);
    assert_eq!(p.base.bounding_box.min_y, 65.0);
    assert_eq!(p.kind.living().unwrap().health, 20);
    assert_eq!(p.base.fire_resistance, 20);
}

#[test]
fn spawn_point_helpers() {
    let mut w = empty_world(3);
    // An empty world: the first uncovered block is the air at y = 63.
    assert_eq!(w.get_first_uncovered_block(0, 0), 0);
    w.set_block(0, 63, 0, 12);
    assert_eq!(w.get_first_uncovered_block(0, 0), 12);
    w.set_block(0, 64, 0, 1);
    assert_eq!(w.get_first_uncovered_block(0, 0), 1);
    assert!(!w.can_coordinate_be_spawn(0, 0));
    w.set_block(0, 64, 0, 0);
    assert!(w.can_coordinate_be_spawn(0, 0));
    // setSpawnLocation: y defaults to 64, the walk stops at once on (0, 0).
    w.set_spawn_location();
    assert_eq!(
        (
            w.world_info.spawn_x,
            w.world_info.spawn_y,
            w.world_info.spawn_z
        ),
        (0, 64, 0)
    );
}

#[test]
fn loading_constructor_new_world_finds_sand() {
    let seed = 123;
    let w = World::new_from_save(Box::new(NoSave), "harness", seed, None, ENV);
    assert!(w.is_new_world);
    assert!(!w.finding_spawn_point);
    assert_eq!(w.world_info.level_name, "harness");
    assert_eq!(w.world_info.spawn_y, 64);
    let (x, z) = (w.world_info.spawn_x, w.world_info.spawn_z);
    let mut w = w;
    assert_eq!(w.get_first_uncovered_block(x, z), 12, "spawn at {x},{z}");
}
