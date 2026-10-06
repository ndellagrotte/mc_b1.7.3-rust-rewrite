//! WP-1.6 task f: unit tests for the parts of the door, bed, ladder, cake,
//! portal, fence, stairs, slab, workbench and locked-chest behaviour that no
//! golden fixture reaches yet (placement tests that only items call, WP-2.6;
//! the client-side face and texture queries; `getNearestEmptyChunkCoordinates`,
//! which only `EntityPlayer` calls, WP-2.5), against values derived by hand
//! from the Java. The world-side behaviour is pinned by `tests/golden/tick/`
//! `door-*`, `trapdoor-toggle`, `bed-*`, `ladder-placement`, `cake-eat`,
//! `portal-*`, `fence-stairs-step-bounds`, `locked-chest-vanishes`,
//! `activate-workbench-stairs` and `block-item-placement`.

use mc_core::java_hash::ChunkCoordinates;
use mc_sim::block::{BlockBounds, bed, dispatch as d};
use mc_sim::entity::player::new_entity_player_sp;
use mc_sim::save::NoSave;
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::{World, WorldEnvironment};

const STONE: i32 = 1;
const BED: i32 = 26;
const STAIR_DOUBLE: i32 = 43;
const STAIR_SINGLE: i32 = 44;
const STAIRS_WOOD: i32 = 53;
const DOOR_WOOD: i32 = 64;
const LADDER: i32 = 65;
const FENCE: i32 = 85;
const PORTAL: i32 = 90;
const CAKE: i32 = 92;
const LOCKED_CHEST: i32 = 95;
const TRAPDOOR: i32 = 96;

fn empty_world() -> World {
    let mut w = World::new(
        Box::new(NoSave),
        "harness",
        WorldProvider::new(Dimension::Surface),
        1,
        ChunkGenerator::Empty(EmptyChunkProvider),
        WorldEnvironment {
            rand_seed: 0,
            field_9437_g: 0,
            lock_timestamp: 0,
        },
    );
    w.make_deterministic(1);
    for dz in -1..=1 {
        for dx in -1..=1 {
            w.get_chunk_from_chunk_coords(dx, dz);
        }
    }
    w
}

fn platform(w: &mut World) {
    for x in -4..=4 {
        for z in -4..=4 {
            w.set_block(x, 60, z, STONE);
        }
    }
}

fn bounds(w: &World, id: i32) -> BlockBounds {
    w.block_statics.bounds[id as usize]
}

/// BlockBed.java:168-192: the 3×3 around the block, x outer, z inner, then
/// the 3×3 shifted against the direction; `skip` counts free spots.
#[test]
fn bed_nearest_empty_spot() {
    let mut w = empty_world();
    platform(&mut w);
    w.set_block_and_metadata(0, 61, 0, BED, 0);
    w.set_block_and_metadata(0, 61, 1, BED, 8);
    let at = |w: &mut World, skip| bed::get_nearest_empty_chunk_coordinates(w, 0, 61, 0, skip);
    assert_eq!(at(&mut w, 0), Some(ChunkCoordinates::new(-1, 61, -1)));
    // (-1,-1), (-1,0), (-1,1) are skipped; (0,-1) is next.
    assert_eq!(at(&mut w, 3), Some(ChunkCoordinates::new(0, 61, -1)));
    // (0,0) and (0,1) are the bed: the fourth free spot of the first square
    // after (0,-1) is (1,-1).
    assert_eq!(at(&mut w, 4), Some(ChunkCoordinates::new(1, 61, -1)));
    // Without ground nothing is free.
    let mut w = empty_world();
    w.set_block_and_metadata(0, 61, 0, BED, 0);
    assert_eq!(at(&mut w, 0), None);
}

/// BlockBed.java:157-166 and :194-199: bit 2, and the bit-8 half drops nothing.
#[test]
fn bed_occupied_bit_and_drops() {
    let mut w = empty_world();
    w.set_block_and_metadata(0, 61, 0, BED, 9);
    bed::set_bed_occupied(&mut w, 0, 61, 0, true);
    assert_eq!(w.get_block_metadata(0, 61, 0), 13);
    bed::set_bed_occupied(&mut w, 0, 61, 0, false);
    assert_eq!(w.get_block_metadata(0, 61, 0), 9);
    let before = w.rand.clone();
    d::drop_block_as_item_with_chance(&mut w, BED, 0, 61, 0, 9, 1.0);
    assert_eq!(w.rand, before, "the bit-8 half draws nothing");
    assert!(w.loaded_entity_list.is_empty());
    d::drop_block_as_item_with_chance(&mut w, BED, 0, 61, 0, 1, 1.0);
    assert_eq!(w.loaded_entity_list.len(), 1);
}

/// BlockStep.java:76-90: the top always; a side next to the same slab id is
/// hidden; the bottom on stone follows Block's test.
#[test]
fn slab_faces() {
    let mut w = empty_world();
    platform(&mut w);
    w.set_block(0, 61, 0, STAIR_SINGLE);
    w.set_block(0, 61, -1, STAIR_SINGLE);
    let side = |w: &mut World, x, y, z, s| d::should_side_be_rendered(w, STAIR_SINGLE, x, y, z, s);
    assert!(side(&mut w, 0, 62, 0, 1));
    assert!(!side(&mut w, 0, 60, 0, 0));
    assert!(!side(&mut w, 0, 61, -1, 2));
    assert!(side(&mut w, 0, 61, 1, 3));
    // The double slab is an opaque cube: Block's test hides a face against it.
    w.set_block(1, 61, 0, STAIR_DOUBLE);
    assert!(!side(&mut w, 1, 61, 0, 5));
}

/// BlockStep.java:43-58: a slab on a single slab of the same metadata merges.
#[test]
fn slab_merge_needs_same_metadata() {
    let mut w = empty_world();
    platform(&mut w);
    w.set_block_and_metadata_with_notify(0, 61, 0, STAIR_SINGLE, 3);
    w.set_block_and_metadata_with_notify(0, 62, 0, STAIR_SINGLE, 2);
    assert_eq!(w.get_block_id(0, 62, 0), STAIR_SINGLE);
    w.set_block_and_metadata_with_notify(1, 61, 0, STAIR_SINGLE, 3);
    w.set_block_and_metadata_with_notify(1, 62, 0, STAIR_SINGLE, 3);
    assert_eq!(w.get_block_id(1, 62, 0), 0);
    assert_eq!(w.get_block_id(1, 61, 0), STAIR_DOUBLE);
    assert_eq!(w.get_block_metadata(1, 61, 0), 3);
}

/// BlockPortal.java:14-27, :120-140.
#[test]
fn portal_bounds_and_faces() {
    let mut w = empty_world();
    w.set_block(0, 61, 0, PORTAL);
    d::set_block_bounds_based_on_state(&mut w, PORTAL, 0, 61, 0);
    assert_eq!(
        bounds(&w, PORTAL),
        BlockBounds::from_floats(0.375, 0.0, 0.0, 0.625, 1.0, 1.0)
    );
    w.set_block(1, 61, 0, PORTAL);
    d::set_block_bounds_based_on_state(&mut w, PORTAL, 0, 61, 0);
    assert_eq!(
        bounds(&w, PORTAL),
        BlockBounds::from_floats(0.0, 0.0, 0.375, 1.0, 1.0, 0.625)
    );
    assert_eq!(
        d::get_collision_bounding_box_from_pool(&mut w, PORTAL, 0, 61, 0),
        None
    );
    // Faces: never into another portal block; across the pane only.
    assert!(!d::should_side_be_rendered(&mut w, PORTAL, 1, 61, 0, 5));
    assert!(d::should_side_be_rendered(&mut w, PORTAL, 0, 61, -1, 2));
    assert!(!d::should_side_be_rendered(&mut w, PORTAL, -1, 61, 0, 4));
}

/// BlockPortal.java:37-85: an obsidian frame with an air inside lights; one
/// with a stone in its side does not.
#[test]
fn portal_frame_check() {
    let mut w = empty_world();
    for x in 1..=2 {
        w.set_block(x, 61, 0, 49);
        w.set_block(x, 65, 0, 49);
    }
    for y in 62..=64 {
        w.set_block(0, y, 0, 49);
        w.set_block(3, y, 0, 49);
    }
    w.set_block(3, 63, 0, STONE);
    assert!(!mc_sim::block::portal::try_to_create_portal(
        &mut w, 1, 62, 0
    ));
    w.set_block(3, 63, 0, 49);
    // From the right-hand cell: (x - 1) is air, so the search shifts left.
    assert!(mc_sim::block::portal::try_to_create_portal(
        &mut w, 2, 62, 0
    ));
    for x in 1..=2 {
        for y in 62..=64 {
            assert_eq!(w.get_block_id(x, y, 0), PORTAL);
        }
    }
    assert!(!w.editing_blocks);
}

/// BlockLockedChest.java:11-40: the front faces away from the one opaque
/// neighbour.
#[test]
fn locked_chest_front() {
    let mut w = empty_world();
    w.set_block(0, 61, 0, LOCKED_CHEST);
    let tex = |w: &mut World, s| d::get_block_texture(w, LOCKED_CHEST, 0, 61, 0, s);
    assert_eq!(tex(&mut w, 1), Some(25));
    assert_eq!(tex(&mut w, 3), Some(27));
    assert_eq!(tex(&mut w, 2), Some(26));
    w.set_block(1, 61, 0, STONE);
    assert_eq!(tex(&mut w, 4), Some(27));
    assert_eq!(tex(&mut w, 3), Some(26));
    assert!(d::can_place_block_at(&mut w, LOCKED_CHEST, 5, 100, 5));
}

/// The placement tests items call (WP-2.6).
#[test]
fn placement_tests() {
    let mut w = empty_world();
    platform(&mut w);
    // BlockDoor.java:188-194: a normal cube below, two free cells, y < 127.
    assert!(d::can_place_block_at(&mut w, DOOR_WOOD, 0, 61, 0));
    assert!(!d::can_place_block_at(&mut w, DOOR_WOOD, 0, 62, 0));
    w.set_block(0, 62, 0, STONE);
    assert!(!d::can_place_block_at(&mut w, DOOR_WOOD, 0, 61, 0));
    // BlockFence.java:8-14: on a fence, or on solid material.
    w.set_block(2, 61, 0, FENCE);
    assert!(d::can_place_block_at(&mut w, FENCE, 2, 62, 0));
    assert!(!d::can_place_block_at(&mut w, FENCE, 2, 70, 0));
    // BlockCake.java:90-92, :102-104.
    assert!(d::can_place_block_at(&mut w, CAKE, 3, 61, 0));
    assert!(!d::can_place_block_at(&mut w, CAKE, 3, 70, 0));
    assert!(d::can_block_stay(&mut w, CAKE, 3, 61, 0));
    // BlockLadder.java:66-97: any horizontal normal cube; onBlockPlaced takes
    // the last supporting side when the clicked face does not hold it.
    assert!(d::can_place_block_at(&mut w, LADDER, 1, 62, 0));
    assert!(!d::can_place_block_at(&mut w, LADDER, 1, 63, 0));
    w.set_block(1, 62, 0, LADDER);
    d::on_block_placed(&mut w, LADDER, 1, 62, 0, 1);
    assert_eq!(w.get_block_metadata(1, 62, 0), 5);
    // BlockTrapDoor.java:133-178.
    assert!(!d::can_place_block_on_side(&mut w, TRAPDOOR, 0, 63, 1, 1));
    assert!(d::can_place_block_on_side(&mut w, TRAPDOOR, 0, 62, -1, 2));
    assert!(!d::can_place_block_on_side(&mut w, TRAPDOOR, 0, 62, -1, 3));
    w.set_block(0, 62, -1, TRAPDOOR);
    d::on_block_placed(&mut w, TRAPDOOR, 0, 62, -1, 4);
    assert_eq!(w.get_block_metadata(0, 62, -1), 2);
}

/// BlockStairs.java:166-184: the metadata from the placer's yaw quadrant.
#[test]
fn stairs_orientation_from_yaw() {
    let mut w = empty_world();
    let mut p = new_entity_player_sp(&mut w);
    w.set_block(0, 61, 0, STAIRS_WOOD);
    for (yaw, meta) in [
        (0.0, 2),
        (90.0, 1),
        (180.0, 3),
        (-90.0, 0),
        (44.0, 2),
        (46.0, 1),
    ] {
        p.base.rotation_yaw = yaw;
        d::on_block_placed_by(&mut w, STAIRS_WOOD, 0, 61, 0, &p);
        assert_eq!(w.get_block_metadata(0, 61, 0), meta, "yaw {yaw}");
    }
}

/// The item-render bounds (client) and the workbench's click.
#[test]
fn item_render_bounds_and_workbench() {
    let mut w = empty_world();
    d::set_block_bounds_for_item_render(&mut w, CAKE);
    assert_eq!(
        bounds(&w, CAKE),
        BlockBounds::from_floats(0.0625, 0.0, 0.0625, 0.9375, 0.5, 0.9375)
    );
    d::set_block_bounds_for_item_render(&mut w, TRAPDOOR);
    assert_eq!(
        bounds(&w, TRAPDOOR),
        BlockBounds::from_floats(0.0, 0.40625, 0.0, 1.0, 0.59375, 1.0)
    );
    let mut p = new_entity_player_sp(&mut w);
    assert!(d::block_activated(&mut w, 58, 0, 61, 0, &mut p));
}
