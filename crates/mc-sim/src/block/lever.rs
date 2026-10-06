//! `BlockLever` (BlockLever.java): the lever (id 69).
//!
//! Metadata: bits 0-2 the face it is fixed to (1 = on the block at -x,
//! 2 = +x, 3 = -z, 4 = +z, 5 and 6 = on the floor, along one axis or the
//! other), bit 3 on.
//!
//! Fully transcribed: `getCollisionBoundingBoxFromPool` (null),
//! `canPlaceBlockOnSide`, `canPlaceBlockAt`, `onBlockPlaced`,
//! `onNeighborBlockChange`, `checkIfAttachedToBlock`,
//! `setBlockBoundsBasedOnState`, `onBlockClicked`, `blockActivated`,
//! `onBlockRemoval`, `isPoweringTo`, `isIndirectlyPoweringTo`,
//! `canProvidePower`. Data methods (`isOpaqueCube`, `renderAsNormalBlock`,
//! `getRenderType`) are in [`crate::block`].

// The Java if/else-if chains of `return true` are kept in their shape and order.
#![allow(clippy::if_same_then_else)]

use super::BlockBounds;
use super::button::notify_attached;
use super::dispatch;
use crate::world::World;

/// `canPlaceBlockOnSide(world, x, y, z, side)`: a normal cube behind the
/// clicked face (top or a side).
// src: BlockLever.java:24-36
pub fn can_place_block_on_side(world: &mut World, x: i32, y: i32, z: i32, side: i32) -> bool {
    if side == 1 && world.is_block_normal_cube(x, y - 1, z) {
        true
    } else if side == 2 && world.is_block_normal_cube(x, y, z + 1) {
        true
    } else if side == 3 && world.is_block_normal_cube(x, y, z - 1) {
        true
    } else if side == 4 && world.is_block_normal_cube(x + 1, y, z) {
        true
    } else {
        side == 5 && world.is_block_normal_cube(x - 1, y, z)
    }
}

/// `canPlaceBlockAt(world, x, y, z)`: a normal cube on a side (-x, +x, -z,
/// +z) or below.
// src: BlockLever.java:38-50
pub fn can_place_block_at(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    if world.is_block_normal_cube(x - 1, y, z) {
        true
    } else if world.is_block_normal_cube(x + 1, y, z) {
        true
    } else if world.is_block_normal_cube(x, y, z - 1) {
        true
    } else if world.is_block_normal_cube(x, y, z + 1) {
        true
    } else {
        world.is_block_normal_cube(x, y - 1, z)
    }
}

/// `onBlockPlaced(world, x, y, z, side)`: faces the clicked face when it can
/// hold the lever (on the floor: 5 or 6 by `world.rand.nextInt(2)`), keeping
/// the on bit; drops it when the clicked face cannot.
// src: BlockLever.java:52-83
pub fn on_block_placed(world: &mut World, id: i32, x: i32, y: i32, z: i32, side: i32) {
    let var6 = world.get_block_metadata(x, y, z);
    let var7 = var6 & 8;
    // src: BlockLever.java:55-56: `var6 &= 7; var6 = -1;`
    let mut var6 = -1;
    if side == 1 && world.is_block_normal_cube(x, y - 1, z) {
        // src: BlockLever.java:58
        var6 = 5 + world.rand.next_int_bounded(2);
    }
    if side == 2 && world.is_block_normal_cube(x, y, z + 1) {
        var6 = 4;
    }
    if side == 3 && world.is_block_normal_cube(x, y, z - 1) {
        var6 = 3;
    }
    if side == 4 && world.is_block_normal_cube(x + 1, y, z) {
        var6 = 2;
    }
    if side == 5 && world.is_block_normal_cube(x - 1, y, z) {
        var6 = 1;
    }
    if var6 == -1 {
        let meta = world.get_block_metadata(x, y, z);
        dispatch::drop_block_as_item(world, id, x, y, z, meta);
        world.set_block_with_notify(x, y, z, 0);
    } else {
        world.set_block_metadata_with_notify(x, y, z, var6 + var7);
    }
}

/// `onNeighborBlockChange(world, x, y, z, neighborId)`: drops the lever
/// without any support, or when the block it is fixed to is gone.
// src: BlockLever.java:85-119
pub fn on_neighbor_block_change(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    if check_if_attached_to_block(world, id, x, y, z) {
        let var6 = world.get_block_metadata(x, y, z) & 7;
        let mut var7 = false;
        if !world.is_block_normal_cube(x - 1, y, z) && var6 == 1 {
            var7 = true;
        }
        if !world.is_block_normal_cube(x + 1, y, z) && var6 == 2 {
            var7 = true;
        }
        if !world.is_block_normal_cube(x, y, z - 1) && var6 == 3 {
            var7 = true;
        }
        if !world.is_block_normal_cube(x, y, z + 1) && var6 == 4 {
            var7 = true;
        }
        if !world.is_block_normal_cube(x, y - 1, z) && var6 == 5 {
            var7 = true;
        }
        if !world.is_block_normal_cube(x, y - 1, z) && var6 == 6 {
            var7 = true;
        }
        if var7 {
            let meta = world.get_block_metadata(x, y, z);
            dispatch::drop_block_as_item(world, id, x, y, z, meta);
            world.set_block_with_notify(x, y, z, 0);
        }
    }
}

/// `checkIfAttachedToBlock(world, x, y, z)` (private): drops and removes
/// the lever when `canPlaceBlockAt` fails; true when it stays.
// src: BlockLever.java:121-129
fn check_if_attached_to_block(world: &mut World, id: i32, x: i32, y: i32, z: i32) -> bool {
    if !can_place_block_at(world, x, y, z) {
        let meta = world.get_block_metadata(x, y, z);
        dispatch::drop_block_as_item(world, id, x, y, z, meta);
        world.set_block_with_notify(x, y, z, 0);
        false
    } else {
        true
    }
}

/// `setBlockBoundsBasedOnState(blockAccess, x, y, z)`: the lever's box on
/// its face (any other face value: the floor box).
// src: BlockLever.java:131-147
pub fn set_block_bounds_based_on_state(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    let var5 = world.get_block_metadata(x, y, z) & 7;
    let mut var6 = 0.1875f32;
    let bounds = if var5 == 1 {
        BlockBounds::from_floats(0.0, 0.2, 0.5 - var6, var6 * 2.0, 0.8, 0.5 + var6)
    } else if var5 == 2 {
        BlockBounds::from_floats(1.0 - var6 * 2.0, 0.2, 0.5 - var6, 1.0, 0.8, 0.5 + var6)
    } else if var5 == 3 {
        BlockBounds::from_floats(0.5 - var6, 0.2, 0.0, 0.5 + var6, 0.8, var6 * 2.0)
    } else if var5 == 4 {
        BlockBounds::from_floats(0.5 - var6, 0.2, 1.0 - var6 * 2.0, 0.5 + var6, 0.8, 1.0)
    } else {
        var6 = 0.25;
        BlockBounds::from_floats(0.5 - var6, 0.0, 0.5 - var6, 0.5 + var6, 0.6, 0.5 + var6)
    };
    world.block_statics.bounds[(id & 255) as usize] = bounds;
}

/// `onBlockClicked(world, x, y, z, player)`: `blockActivated`.
// src: BlockLever.java:149-151
pub fn on_block_clicked(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    block_activated(world, id, x, y, z);
}

/// `blockActivated(world, x, y, z, player)`: toggles the lever (metadata,
/// render update, click sound, notifications); true (also on a multiplayer
/// client, which does nothing).
// src: BlockLever.java:153-178
pub fn block_activated(world: &mut World, id: i32, x: i32, y: i32, z: i32) -> bool {
    if world.multiplayer_world {
        true
    } else {
        let var6 = world.get_block_metadata(x, y, z);
        let var7 = var6 & 7;
        let var8 = 8 - (var6 & 8);
        world.set_block_metadata_with_notify(x, y, z, var7 + var8);
        world.mark_blocks_dirty(x, y, z, x, y, z);
        world.play_sound_effect(
            x as f64 + 0.5,
            y as f64 + 0.5,
            z as f64 + 0.5,
            "random.click",
            0.3,
            if var8 > 0 { 0.6 } else { 0.5 },
        );
        notify_attached(world, id, x, y, z, var7);
        true
    }
}

/// `onBlockRemoval(world, x, y, z)`: a lever that was on notifies as when
/// switched; then `super.onBlockRemoval` (empty). The metadata read is the
/// removed lever's (Chunk.java:245-251).
// src: BlockLever.java:180-199
pub fn on_block_removal(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    let var5 = world.get_block_metadata(x, y, z);
    if (var5 & 8) > 0 {
        notify_attached(world, id, x, y, z, var5 & 7);
    }
}

/// `isPoweringTo(blockAccess, x, y, z, side)`: on.
// src: BlockLever.java:201-203
pub fn is_powering_to(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    (world.get_block_metadata(x, y, z) & 8) > 0
}

/// `isIndirectlyPoweringTo(world, x, y, z, side)`: on, toward the block it
/// is fixed to only.
// src: BlockLever.java:205-225
pub fn is_indirectly_powering_to(world: &mut World, x: i32, y: i32, z: i32, side: i32) -> bool {
    let var6 = world.get_block_metadata(x, y, z);
    if (var6 & 8) == 0 {
        false
    } else {
        let var7 = var6 & 7;
        if var7 == 6 && side == 1 {
            true
        } else if var7 == 5 && side == 1 {
            true
        } else if var7 == 4 && side == 2 {
            true
        } else if var7 == 3 && side == 3 {
            true
        } else if var7 == 2 && side == 4 {
            true
        } else {
            var7 == 1 && side == 5
        }
    }
}

/// `canProvidePower()`: true.
// src: BlockLever.java:227-229
pub fn can_provide_power() -> bool {
    true
}
