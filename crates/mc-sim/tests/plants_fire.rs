//! Paths of the WP-1.6 fire/plants/snow task that no Tier B fixture reaches
//! (no harness op breaks a block as a player, and colours, selection boxes
//! and face culling are client queries): checked here against values worked
//! out from the Java by hand. The ticking behaviour is pinned by the
//! fixtures (`tests/golden/tick/{fire-*,grass-spread,crops-growth,
//! sapling-grow,leaves-decay,snow-ice,cactus-reed-growth}`).

use mc_core::Aabb;
use mc_sim::block::{BlockBounds, dispatch as d};
use mc_sim::entity::Entity;
use mc_sim::entity::player::new_entity_player_sp;
use mc_sim::item::ItemStack;
use mc_sim::save::NoSave;
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::{World, WorldEnvironment};

/// An empty-generator overworld seeded like the oracle's, chunk (0, 0)
/// loaded, a stone floor at y = 60 over x, z in 0..16.
fn world() -> World {
    let mut w = World::new(
        Box::new(NoSave),
        "harness",
        WorldProvider::new(Dimension::Surface),
        123,
        ChunkGenerator::Empty(EmptyChunkProvider),
        WorldEnvironment {
            rand_seed: 0,
            field_9437_g: 0,
            lock_timestamp: 0,
        },
    );
    w.make_deterministic(123);
    for x in 0..16 {
        for z in 0..16 {
            w.set_block_and_metadata(x, 60, z, 1, 0);
        }
    }
    w
}

fn holding(w: &mut World, item: Option<ItemStack>) -> Entity {
    let mut p = new_entity_player_sp(w);
    if let Some(pd) = p.kind.player_mut() {
        pd.inventory.main_inventory[0] = item;
    }
    p
}

fn items(w: &World) -> Vec<ItemStack> {
    w.loaded_entity_list
        .iter()
        .filter_map(|s| s.entity.as_deref())
        .filter_map(|e| e.kind.item().and_then(|i| i.item))
        .collect()
}

#[test]
fn snow_layer_bounds_and_collision() {
    let mut w = world();
    w.set_block_and_metadata(4, 61, 4, 78, 2);
    // setBlockBoundsBasedOnState: (2 * (1 + 2)) / 16 = 0.375 (BlockSnow.java:25-29).
    d::set_block_bounds_based_on_state(&mut w, 78, 4, 61, 4);
    assert_eq!(
        w.block_statics.bounds[78],
        BlockBounds::from_floats(0.0, 0.0, 0.0, 1.0, 0.375, 1.0)
    );
    // Below three layers: no collision box (BlockSnow.java:13-14).
    assert_eq!(
        d::get_collision_bounding_box_from_pool(&mut w, 78, 4, 61, 4),
        None
    );
    w.set_block_and_metadata(4, 61, 4, 78, 3);
    assert_eq!(
        d::get_collision_bounding_box_from_pool(&mut w, 78, 4, 61, 4),
        Some(Aabb::new(4.0, 61.0, 4.0, 5.0, 61.5, 5.0))
    );
    // canPlaceBlockAt: stone yes, glass (not an opaque cube) no.
    assert!(d::can_place_block_at(&mut w, 78, 4, 61, 4));
    w.set_block_and_metadata(6, 60, 6, 20, 0);
    assert!(!d::can_place_block_at(&mut w, 78, 6, 61, 6));
}

#[test]
fn cactus_boxes_are_float_sums() {
    let mut w = world();
    let f = |v: f32| v as f64;
    let c = d::get_collision_bounding_box_from_pool(&mut w, 81, 3, 61, -2).expect("cactus box");
    assert_eq!(
        c,
        Aabb::new(
            f(3.0 + 0.0625),
            61.0,
            f(-2.0 + 0.0625),
            f(4.0 - 0.0625),
            f(62.0 - 0.0625),
            f(-1.0 - 0.0625)
        )
    );
    let s = d::get_selected_bounding_box_from_pool(&mut w, 81, 3, 61, -2);
    assert_eq!(s.max_y, 62.0);
    // Reeds, flowers and fire have none; farmland is a full cube.
    assert_eq!(
        d::get_collision_bounding_box_from_pool(&mut w, 83, 3, 61, 3),
        None
    );
    assert_eq!(
        d::get_collision_bounding_box_from_pool(&mut w, 37, 3, 61, 3),
        None
    );
    assert_eq!(
        d::get_collision_bounding_box_from_pool(&mut w, 51, 3, 61, 3),
        None
    );
    assert_eq!(
        d::get_collision_bounding_box_from_pool(&mut w, 60, 3, 60, 3),
        Some(Aabb::new(3.0, 60.0, 3.0, 4.0, 61.0, 4.0))
    );
}

#[test]
fn snow_harvest_drops_one_snowball_without_super() {
    let mut w = world();
    w.set_block_and_metadata(5, 61, 5, 78, 0);
    let mut p = holding(&mut w, None);
    let before = w.rand.state();
    d::harvest_block(&mut w, 78, 5, 61, 5, &mut p, 0);
    assert_ne!(w.rand.state(), before, "three World.rand draws");
    assert_eq!(items(&w), vec![ItemStack::new(256 + 76, 1, 0)]);
    assert_eq!(w.get_block_id(5, 61, 5), 0);
}

#[test]
fn ice_harvest_leaves_flowing_water_over_a_solid_block() {
    let mut w = world();
    w.set_block_and_metadata(5, 61, 5, 79, 0);
    let mut p = holding(&mut w, None);
    // The player controller removes the block before harvestBlock.
    w.set_block_with_notify(5, 61, 5, 0);
    d::harvest_block(&mut w, 79, 5, 61, 5, &mut p, 0);
    assert_eq!(w.get_block_id(5, 61, 5), 8);
    assert!(items(&w).is_empty(), "quantityDropped is 0");
    // Over air: nothing.
    w.set_block_and_metadata(7, 62, 7, 0, 0);
    d::harvest_block(&mut w, 79, 7, 62, 7, &mut p, 0);
    assert_eq!(w.get_block_id(7, 62, 7), 0);
}

#[test]
fn leaves_harvest_with_shears_drops_the_leaf_type() {
    let mut w = world();
    let mut p = holding(&mut w, Some(ItemStack::new(256 + 103, 1, 0)));
    d::harvest_block(&mut w, 18, 5, 61, 5, &mut p, 2 | 8);
    assert_eq!(items(&w), vec![ItemStack::new(18, 1, 2)]);
    // Without shears: the normal drop table (a sapling one time in 20).
    let mut w = world();
    let mut p = holding(&mut w, None);
    d::harvest_block(&mut w, 18, 5, 61, 5, &mut p, 0);
    assert!(items(&w).iter().all(|s| s.item_id == 6));
}

#[test]
fn render_colours() {
    let mut w = world();
    assert_eq!(d::get_render_color(&w, 18, 1), 6396257);
    assert_eq!(d::get_render_color(&w, 18, 2), 8431445);
    assert_eq!(d::get_render_color(&w, 18, 0), 4764952);
    w.set_block_and_metadata(2, 61, 2, 18, 5);
    assert_eq!(d::color_multiplier(&mut w, 18, 2, 61, 2), 6396257);
    // Dead shrub (tall grass meta 0) is white.
    w.set_block_and_metadata(3, 61, 3, 31, 0);
    assert_eq!(d::color_multiplier(&mut w, 31, 3, 61, 3), 16777215);
    // The grass and foliage colormaps are client textures: nothing installed.
    assert_eq!(d::color_multiplier(&mut w, 2, 3, 60, 3), 0);
    assert!(
        w.unimplemented_hits
            .borrow()
            .contains_key("ColorizerGrass.getGrassColor")
    );
}

#[test]
fn face_culling() {
    let mut w = world();
    // Leaves with fast graphics (the default) hide faces against leaves.
    w.set_block_and_metadata(2, 61, 2, 18, 0);
    w.set_block_and_metadata(3, 61, 2, 18, 0);
    assert!(!d::should_side_be_rendered(&mut w, 18, 3, 61, 2, 5));
    w.block_statics.set_leaves_graphics_level(true);
    assert!(d::should_side_be_rendered(&mut w, 18, 3, 61, 2, 5));
    // Ice: faces against ice hidden; the side is flipped as 1 - side before
    // the base test, so the bottom face against stone shows only through
    // that flip (side 0 -> 1, maxY < 1 is false, then the stone is opaque).
    w.set_block_and_metadata(5, 61, 5, 79, 0);
    w.set_block_and_metadata(6, 61, 5, 79, 0);
    assert!(!d::should_side_be_rendered(&mut w, 79, 6, 61, 5, 5));
    assert!(!d::should_side_be_rendered(&mut w, 79, 5, 60, 5, 0));
    // Snow layer: the top always.
    assert!(d::should_side_be_rendered(&mut w, 78, 5, 62, 5, 1));
    // Grass sides under snow show the snowy texture.
    w.set_block_and_metadata(8, 61, 8, 78, 0);
    assert_eq!(d::get_block_texture(&mut w, 2, 8, 60, 8, 2), Some(68));
    assert_eq!(d::get_block_texture(&mut w, 2, 9, 60, 9, 2), Some(3));
    assert_eq!(d::get_block_texture(&mut w, 2, 9, 60, 9, 1), Some(0));
}
