//! The base `Block` bodies added to the dispatch in WP-1.6 prep, against
//! values derived by hand from Block.java (no golden op reaches them yet:
//! block breaking and explosions arrive with WP-1.8 and WP-2.5).

use mc_core::java_random::JavaRandom;
use mc_sim::block::dispatch as d;
use mc_sim::entity::player::new_entity_player_sp;
use mc_sim::item::ItemStack;
use mc_sim::save::NoSave;
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
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

/// Block.java:339-365 for cobblestone (id 4: one roll, drops itself, no
/// draw in `idDropped`): `nextFloat() <= 1.0`, then three `nextFloat()` for
/// the offset `rand * 0.7F + 0.15`, an `EntityItem` with pick-up delay 10.
#[test]
fn drop_block_as_item_draws_and_places_like_block_java() {
    let mut w = empty_world(7);
    let mut r: JavaRandom = w.rand.clone();
    let _chance = r.next_float();
    let mut off = || (r.next_float() * 0.7f32) as f64 + (1.0f32 - 0.7f32) as f64 * 0.5;
    let (ox, oy, oz) = (off(), off(), off());
    let n = w.loaded_entity_list.len();
    d::drop_block_as_item(&mut w, 4, 3, 61, -2, 0);
    assert_eq!(w.loaded_entity_list.len(), n + 1);
    let e = w
        .loaded_entity_list
        .last()
        .unwrap()
        .entity
        .as_ref()
        .unwrap();
    let it = e.kind.item().expect("an EntityItem");
    assert_eq!(it.item, Some(ItemStack::new(4, 1, 0)));
    assert_eq!(it.delay_before_can_pickup, 10);
    assert_eq!(e.base.pos_x.to_bits(), (3.0 + ox).to_bits());
    assert_eq!(e.base.pos_y.to_bits(), (61.0 + oy).to_bits());
    assert_eq!(e.base.pos_z.to_bits(), (-2.0 + oz).to_bits());
    assert_eq!(w.rand.state(), r.state());
}

/// A roll above the chance drops nothing but still draws; a multiplayer
/// client world draws nothing.
#[test]
fn drop_chance_and_multiplayer() {
    let mut w = empty_world(7);
    let n = w.loaded_entity_list.len();
    let mut r: JavaRandom = w.rand.clone();
    r.next_float();
    d::drop_block_as_item_with_chance(&mut w, 4, 0, 61, 0, 0, -1.0);
    assert_eq!(w.loaded_entity_list.len(), n);
    assert_eq!(w.rand.state(), r.state());

    w.multiplayer_world = true;
    let before = w.rand.state();
    d::drop_block_as_item(&mut w, 4, 0, 61, 0, 0);
    assert_eq!(w.rand.state(), before);
    assert_eq!(w.loaded_entity_list.len(), n);
}

/// Block.java:327-333: stone (rock, not harvestable by hand) and dirt
/// (harvestable) with an empty hand, off the ground and on it.
#[test]
fn block_strength_empty_hand() {
    let mut w = empty_world(1);
    let mut p = new_entity_player_sp(&mut w);
    p.base.set_position(0.5, 70.0, 0.5);
    p.base.on_ground = false;
    // stone: hardness 1.5, cannot harvest: 1 / 1.5 / 100
    assert_eq!(d::block_strength(&mut w, 1, &p), 1.0f32 / 1.5 / 100.0);
    // dirt: hardness 0.5, harvestable; strength 1, off the ground / 5
    assert_eq!(d::block_strength(&mut w, 3, &p), 1.0f32 / 5.0 / 0.5 / 30.0);
    p.base.on_ground = true;
    assert_eq!(d::block_strength(&mut w, 3, &p), 1.0f32 / 0.5 / 30.0);
    // bedrock: unbreakable
    assert_eq!(d::block_strength(&mut w, 7, &p), 0.0);
}

/// Block.java:235-251 with full bounds: a face is drawn unless the
/// neighbour is an opaque cube.
#[test]
fn should_side_be_rendered_full_cube() {
    let mut w = empty_world(1);
    w.set_block(0, 61, 0, 1);
    assert!(!d::should_side_be_rendered(&mut w, 1, 0, 61, 0, 1));
    assert!(d::should_side_be_rendered(&mut w, 1, 0, 62, 0, 1));
}

/// The constant bases: white tint, resistance / 5, `canBlockStay` true.
#[test]
fn constant_bases() {
    let mut w = empty_world(1);
    assert_eq!(d::get_render_color(&w, 1, 0), 0xFFFFFF);
    assert_eq!(d::color_multiplier(&mut w, 1, 0, 61, 0), 0xFFFFFF);
    // stone: setResistance(10) stores 30, / 5 = 6
    assert_eq!(d::get_explosion_resistance(&w, 1, None), 6.0);
    assert!(d::can_block_stay(&mut w, 1, 0, 61, 0));
    assert!(!d::can_provide_power(&w, 1));
    assert!(w.unimplemented_hits.borrow().is_empty());
}
