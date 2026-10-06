//! The fluid methods no golden fixture reaches (WP-1.6 fluids task): the
//! renderer's queries (`getFlowDirection`, `getIsBlockSolid`,
//! `shouldSideBeRendered`, `getBlockBrightness`, `colorMultiplier`) and
//! `randomDisplayTick`'s draws, against values worked out by hand from
//! BlockFluid.java; and `BlockFlowing.updateTick` under
//! `scheduledUpdatesAreImmediate` (population's `WorldGenLiquids` mode),
//! which re-enters the flowing singleton's scratch state.

use mc_core::java_random::JavaRandom;
use mc_sim::block::fluid::{self, get_flow_direction, get_percent_air};
use mc_sim::block::{Material, dispatch as d};
use mc_sim::save::NoSave;
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::{World, WorldEnvironment};

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
    w
}

/// A stone floor at y = 60 over -10..10 (silent).
fn floor(w: &mut World) {
    for x in -10..=10 {
        for z in -10..=10 {
            w.set_block(x, 60, z, 1);
        }
    }
}

#[test]
fn percent_air() {
    // src: BlockFluid.java:18-25
    assert_eq!(get_percent_air(0), 1.0 / 9.0);
    assert_eq!(get_percent_air(7), 8.0 / 9.0);
    assert_eq!(get_percent_air(8), 1.0 / 9.0);
    assert_eq!(get_percent_air(15), 1.0 / 9.0);
}

#[test]
fn flow_direction() {
    let mut w = empty_world();
    floor(&mut w);
    w.set_block_and_metadata(0, 61, 0, 9, 0);
    w.set_block_and_metadata(1, 61, 0, 9, 1);
    // An isolated source on the floor: no gradient.
    w.set_block_and_metadata(-6, 61, -6, 9, 0);
    // (1,61,0): -x neighbour level 0, var12 = 0 - 1 = -1, (var8 - x) = -1:
    // +x flow (1, 0, 0); atan2(0, 1) - pi/2.
    assert_eq!(
        get_flow_direction(&mut w, 1, 61, 0, Material::Water),
        Some(-1.5707963267948966)
    );
    // The source: its +x neighbour is at level 1, var12 = 1 - 0 = 1,
    // (var8 - x) = +1: also +x.
    assert_eq!(
        get_flow_direction(&mut w, 0, 61, 0, Material::Water),
        Some(-1.5707963267948966)
    );
    assert_eq!(
        get_flow_direction(&mut w, -6, 61, -6, Material::Water),
        Some(-1000.0)
    );
    // Lava material reads with Block.lavaMoving: no lava here, all -1.
    assert_eq!(
        get_flow_direction(&mut w, -6, 61, -6, Material::Lava),
        Some(-1000.0)
    );
    // Java throws a NullPointerException for any other material.
    assert_eq!(get_flow_direction(&mut w, 0, 61, 0, Material::Rock), None);
}

#[test]
fn falling_water_pulls_down_next_to_a_wall() {
    let mut w = empty_world();
    floor(&mut w);
    // Falling water (meta 8) with stone on its -z side: the normalised
    // horizontal vector (zero here) plus (0, -6, 0), normalised: straight down.
    w.set_block_and_metadata(0, 61, 0, 8, 8);
    w.set_block(0, 61, -1, 1);
    let e = mc_sim::entity::player::new_entity_player_sp(&mut w);
    let mut v = mc_core::Vec3D::create_vector(0.0, 0.0, 0.0);
    d::velocity_to_add_to_entity(&mut w, 8, 0, 61, 0, &e, &mut v);
    assert_eq!((v.x_coord, v.y_coord, v.z_coord), (0.0, -1.0, 0.0));
}

#[test]
fn renderer_side_queries() {
    let mut w = empty_world();
    floor(&mut w);
    w.set_block(0, 61, 0, 9);
    w.set_block(1, 61, 0, 9);
    w.set_block(0, 61, 1, 79); // ice
    w.set_block(-1, 61, 0, 1);
    // getIsBlockSolid: same fluid, ice: false; top face: true; else the
    // material's isSolid (stone yes, air no).
    assert!(!d::get_is_block_solid(&mut w, 9, 1, 61, 0, 5));
    assert!(!d::get_is_block_solid(&mut w, 9, 0, 61, 1, 3));
    assert!(d::get_is_block_solid(&mut w, 9, 0, 61, -1, 1));
    assert!(!d::get_is_block_solid(&mut w, 9, 0, 61, -1, 2));
    assert!(d::get_is_block_solid(&mut w, 9, -1, 61, 0, 4));
    // Still lava against water: another material, so the base answers.
    assert!(!d::get_is_block_solid(&mut w, 11, 1, 61, 0, 5));
    // shouldSideBeRendered: never against the same fluid or ice; the top
    // always; else the base (the neighbour is not an opaque cube).
    assert!(!d::should_side_be_rendered(&mut w, 9, 1, 61, 0, 5));
    assert!(!d::should_side_be_rendered(&mut w, 9, 0, 61, 1, 3));
    assert!(d::should_side_be_rendered(&mut w, 9, 0, 62, 0, 1));
    assert!(d::should_side_be_rendered(&mut w, 9, 0, 61, -1, 2));
    assert!(!d::should_side_be_rendered(&mut w, 9, -1, 61, 0, 4));
    // colorMultiplier: white; no collision box.
    assert_eq!(d::color_multiplier(&mut w, 8, 0, 61, 0), 16777215);
    assert_eq!(fluid::color_multiplier(), 16777215);
    assert_eq!(
        d::get_collision_bounding_box_from_pool(&mut w, 9, 0, 61, 0),
        None
    );
    // getBlockBrightness: the brighter of the cell and the one above.
    let here = w.get_light_brightness(0, 61, 0);
    let above = w.get_light_brightness(0, 62, 0);
    assert_eq!(
        d::get_block_brightness(&mut w, 9, 0, 61, 0),
        if here > above { here } else { above }
    );
    assert!(w.unimplemented_hits.borrow().is_empty());
}

#[test]
fn random_display_tick_draws() {
    let mut w = empty_world();
    floor(&mut w);
    w.set_block_and_metadata(0, 61, 0, 8, 3);
    w.set_block(2, 61, 0, 11);
    for seed in 0..200 {
        // Water: nextInt(64), then two floats when it hits and 0 < meta < 8.
        let mut r = JavaRandom::new(seed);
        let mut expect = r.clone();
        if expect.next_int_bounded(64) == 0 {
            expect.next_float();
            expect.next_float();
        }
        d::random_display_tick(&mut w, 8, 0, 61, 0, &mut r);
        assert_eq!(r.state(), expect.state(), "water, seed {seed}");
        // Lava under open air: nextInt(100), then two floats on a hit.
        let mut r = JavaRandom::new(seed);
        let mut expect = r.clone();
        if expect.next_int_bounded(100) == 0 {
            expect.next_float();
            expect.next_float();
        }
        d::random_display_tick(&mut w, 11, 2, 61, 0, &mut r);
        assert_eq!(r.state(), expect.state(), "lava, seed {seed}");
    }
    // Lava under a block: no draw at all (the material test comes first).
    w.set_block(2, 62, 0, 1);
    let mut r = JavaRandom::new(5);
    let before = r.state();
    d::random_display_tick(&mut w, 11, 2, 61, 0, &mut r);
    assert_eq!(r.state(), before);
}

/// Population's mode (WorldGenLiquids.java:56-58): every scheduled tick runs
/// at once, so `flowIntoBlock` re-enters `updateTick` on the same singleton.
/// The flood fill terminates and reaches the same 7-block diamond as the
/// ticked spread.
#[test]
fn immediate_updates_flood_fill() {
    let mut w = empty_world();
    floor(&mut w);
    // Load every chunk within 8 of the area so scheduleBlockUpdate runs the
    // tick (World.java:1155-1163).
    for x in (-24..=24).step_by(16) {
        for z in (-24..=24).step_by(16) {
            w.get_block_id(x, 64, z);
        }
    }
    w.scheduled_updates_are_immediate = true;
    w.set_block_and_metadata_with_notify(0, 61, 0, 8, 0);
    w.scheduled_updates_are_immediate = false;
    for x in -10i32..=10 {
        for z in -10i32..=10 {
            let id = w.get_block_id(x, 61, z);
            let dist = x.abs() + z.abs();
            if dist <= 7 {
                assert!(id == 8 || id == 9, "({x},{z}) id {id}");
                assert_eq!(w.get_block_metadata(x, 61, z), dist, "({x},{z})");
            } else {
                assert_eq!(id, 0, "({x},{z})");
            }
        }
    }
}
