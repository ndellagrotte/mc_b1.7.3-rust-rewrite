//! Redstone-block paths checked by hand (WP-1.6 task c): the placement
//! hooks only items call (`onBlockPlaced`, `onBlockPlacedBy`; since WP-2.6
//! they are also pinned against the oracle by `tests/golden/tick/`
//! `redstone-item-placement`, `block-item-placement` and
//! `place-stairs-slabs-doors-beds-signs`), the left-click and walking hooks
//! (no op for them), and the item-render bounds (client only). Expected values are read off the Java bodies cited per test.

use mc_core::java_random::JavaRandom;
use mc_sim::block::dispatch as d;
use mc_sim::block::{BlockBounds, block};
use mc_sim::entity::player::new_entity_player_sp;
use mc_sim::save::NoSave;
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::{World, WorldEnvironment};

const STONE: i32 = 1;
const TORCH: i32 = 50;
const LEVER: i32 = 69;
const ORE_REDSTONE: i32 = 73;
const ORE_REDSTONE_GLOWING: i32 = 74;
const BUTTON: i32 = 77;
const REPEATER_IDLE: i32 = 93;

/// An empty world with a stone floor at y = 60 over -4..4 and a stone block
/// at (0, 61, 0).
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
    for x in -4..=4 {
        for z in -4..=4 {
            w.set_block(x, 60, z, STONE);
        }
    }
    w.set_block(0, 61, 0, STONE);
    w
}

/// BlockTorch.onBlockPlaced (BlockTorch.java:45-68): the clicked face wins
/// over the orientation onBlockAdded chose, when it can hold the torch.
#[test]
fn torch_on_block_placed_faces_the_clicked_side() {
    let mut w = world();
    // On the block's +x face (side 5): onBlockAdded already picks -x (1).
    w.set_block_and_metadata_with_notify(1, 61, 0, TORCH, 0);
    assert_eq!(w.get_block_metadata(1, 61, 0), 1);
    // Clicked on the floor below (side 1): the floor is a normal cube -> 5.
    d::on_block_placed(&mut w, TORCH, 1, 61, 0, 1);
    assert_eq!(w.get_block_metadata(1, 61, 0), 5);
    // Side 2 needs a normal cube at z + 1: none, so the metadata stays.
    d::on_block_placed(&mut w, TORCH, 1, 61, 0, 2);
    assert_eq!(w.get_block_metadata(1, 61, 0), 5);
    // Side 5 needs a normal cube at x - 1: the block -> 1.
    d::on_block_placed(&mut w, TORCH, 1, 61, 0, 5);
    assert_eq!(w.get_block_metadata(1, 61, 0), 1);
}

/// BlockLever.onBlockPlaced (BlockLever.java:52-83): on the floor 5 or 6 by
/// `World.rand.nextInt(2)`, the on bit kept; a face that cannot hold it
/// drops the lever.
#[test]
fn lever_on_block_placed() {
    let mut w = world();
    w.set_block_and_metadata_with_notify(2, 61, 2, LEVER, 8);
    let mut r = w.rand.clone();
    let expected = 5 + r.next_int_bounded(2);
    d::on_block_placed(&mut w, LEVER, 2, 61, 2, 1);
    assert_eq!(w.get_block_metadata(2, 61, 2), expected + 8);
    assert_eq!(w.rand.next_long(), r.next_long(), "one nextInt(2) draw");

    // Side 3 (clicked the -z face of a block at z - 1): air there, dropped.
    w.set_block_and_metadata_with_notify(-2, 61, 2, LEVER, 0);
    d::on_block_placed(&mut w, LEVER, -2, 61, 2, 3);
    assert_eq!(w.get_block_id(-2, 61, 2), 0);
    // Side 4 on the block's -x face: needs the block at x + 1 -> 2.
    w.set_block_and_metadata_with_notify(-1, 61, 0, LEVER, 0);
    d::on_block_placed(&mut w, LEVER, -1, 61, 0, 4);
    assert_eq!(w.get_block_metadata(-1, 61, 0), 2);
}

/// BlockButton.onBlockPlaced (BlockButton.java:51-80): a refused side falls
/// back to getOrientation (first support, else 1).
#[test]
fn button_on_block_placed_falls_back_to_get_orientation() {
    let mut w = world();
    w.set_block_and_metadata_with_notify(0, 61, 1, BUTTON, 8);
    // Side 1 is never accepted: getOrientation finds the block at z - 1 -> 3.
    d::on_block_placed(&mut w, BUTTON, 0, 61, 1, 1);
    assert_eq!(w.get_block_metadata(0, 61, 1), 3 + 8);
    assert!(!d::can_place_block_on_side(&mut w, BUTTON, 0, 61, 1, 1));
    assert!(d::can_place_block_on_side(&mut w, BUTTON, 0, 61, 1, 3));
}

/// BlockRedstoneRepeater.onBlockPlacedBy (BlockRedstoneRepeater.java:131-139):
/// `((floor(yaw * 4 / 360 + 0.5) & 3) + 2) % 4`.
#[test]
fn repeater_faces_away_from_the_placer() {
    let mut w = world();
    let mut p = new_entity_player_sp(&mut w);
    for (yaw, meta) in [
        (0.0f32, 2),
        (90.0, 3),
        (180.0, 0),
        (270.0, 1),
        (-90.0, 1),
        (44.0, 2),
        (46.0, 3),
    ] {
        p.base.rotation_yaw = yaw;
        w.set_block_and_metadata_with_notify(3, 61, 3, REPEATER_IDLE, 0);
        d::on_block_placed_by(&mut w, REPEATER_IDLE, 3, 61, 3, &p);
        assert_eq!(w.get_block_metadata(3, 61, 3), meta, "yaw {yaw}");
    }
}

/// BlockRedstoneOre.onBlockClicked / onEntityWalking (BlockRedstoneOre.java:21-42):
/// sparkle (18 World.rand floats) and the plain ore glows; the glowing ore
/// only sparkles.
#[test]
fn redstone_ore_glows_when_clicked_or_walked_on() {
    let mut w = world();
    let mut p = new_entity_player_sp(&mut w);
    w.set_block(2, 60, 2, ORE_REDSTONE);
    let mut r = w.rand.clone();
    d::on_block_clicked(&mut w, ORE_REDSTONE, 2, 60, 2, &mut p);
    assert_eq!(w.get_block_id(2, 60, 2), ORE_REDSTONE_GLOWING);
    for _ in 0..18 {
        r.next_float();
    }
    assert_eq!(w.rand.next_long(), r.next_long(), "18 nextFloat draws");

    w.set_block(-2, 60, 2, ORE_REDSTONE);
    d::on_entity_walking(&mut w, ORE_REDSTONE, -2, 60, 2, &mut p);
    assert_eq!(w.get_block_id(-2, 60, 2), ORE_REDSTONE_GLOWING);
    d::on_entity_walking(&mut w, ORE_REDSTONE_GLOWING, -2, 60, 2, &mut p);
    assert_eq!(w.get_block_id(-2, 60, 2), ORE_REDSTONE_GLOWING);
    // updateTick of the glowing ore turns it back.
    d::update_tick(
        &mut w,
        ORE_REDSTONE_GLOWING,
        -2,
        60,
        2,
        d::RandomSource::WorldRand,
    );
    assert_eq!(w.get_block_id(-2, 60, 2), ORE_REDSTONE);
}

/// setBlockBoundsForItemRender of the button and the plates
/// (BlockButton.java:250-255, BlockPressurePlate.java:149-154).
#[test]
fn item_render_bounds() {
    let mut w = world();
    d::set_block_bounds_for_item_render(&mut w, BUTTON);
    assert_eq!(
        w.block_statics.bounds[BUTTON as usize],
        BlockBounds::from_floats(0.3125, 0.375, 0.375, 0.6875, 0.625, 0.625)
    );
    for plate in [70, 72] {
        d::set_block_bounds_for_item_render(&mut w, plate);
        assert_eq!(
            w.block_statics.bounds[plate as usize],
            BlockBounds::from_floats(0.0, 0.375, 0.0, 1.0, 0.625, 1.0)
        );
    }
}

/// The client-only queries: colorMultiplier of wire (BlockRedstoneWire.java:37-39),
/// shouldSideBeRendered of the repeater (BlockRedstoneRepeater.java:53-55),
/// and the particles of a powered wire (two display-random draws,
/// BlockRedstoneWire.java:339-364).
#[test]
fn client_queries() {
    let mut w = world();
    assert_eq!(d::color_multiplier(&mut w, 55, 0, 61, 0), 0x800000);
    for side in 0..6 {
        assert_eq!(
            d::should_side_be_rendered(&mut w, REPEATER_IDLE, 0, 61, 0, side),
            side > 1
        );
    }
    assert!(block(55).is_some());
    // Placed unpowered (onBlockAdded propagates level 0), then forced to 7.
    w.set_block(3, 61, -3, 55);
    w.set_block_metadata(3, 61, -3, 7);
    w.events.drain();
    let mut r = JavaRandom::new(5);
    let mut r2 = r.clone();
    d::random_display_tick(&mut w, 55, 3, 61, -3, &mut r);
    assert_eq!(w.events.drain().len(), 1);
    r2.next_float();
    r2.next_float();
    assert_eq!(r.next_long(), r2.next_long());
}
