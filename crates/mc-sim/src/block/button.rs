//! `BlockButton` (BlockButton.java): the stone button (id 77).
//!
//! Metadata: bits 0-2 the face it hangs on (1 = on the block at -x, 2 = +x,
//! 3 = -z, 4 = +z), bit 3 pressed.
//!
//! Fully transcribed: `getCollisionBoundingBoxFromPool` (null),
//! `canPlaceBlockOnSide`, `canPlaceBlockAt`, `onBlockPlaced`,
//! `getOrientation`, `onNeighborBlockChange`, `checkForDrop`,
//! `setBlockBoundsBasedOnState`, `onBlockClicked`, `blockActivated`,
//! `onBlockRemoval`, `isPoweringTo`, `isIndirectlyPoweringTo`,
//! `canProvidePower`, `updateTick`, `setBlockBoundsForItemRender`. Data
//! methods (`tickRate` 20, `isOpaqueCube`, `renderAsNormalBlock`) are in
//! [`crate::block`].

// The Java if/else-if chains of `return true` are kept in their shape and order.
#![allow(clippy::if_same_then_else)]

use super::BlockBounds;
use super::dispatch;
use crate::world::World;

/// `canPlaceBlockOnSide(world, x, y, z, side)`: a normal cube behind the
/// clicked side face (sides 2-5 only).
// src: BlockButton.java:27-37
pub fn can_place_block_on_side(world: &mut World, x: i32, y: i32, z: i32, side: i32) -> bool {
    if side == 2 && world.is_block_normal_cube(x, y, z + 1) {
        true
    } else if side == 3 && world.is_block_normal_cube(x, y, z - 1) {
        true
    } else if side == 4 && world.is_block_normal_cube(x + 1, y, z) {
        true
    } else {
        side == 5 && world.is_block_normal_cube(x - 1, y, z)
    }
}

/// `canPlaceBlockAt(world, x, y, z)`: a normal cube on one of the four
/// sides.
// src: BlockButton.java:39-49
pub fn can_place_block_at(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    if world.is_block_normal_cube(x - 1, y, z) {
        true
    } else if world.is_block_normal_cube(x + 1, y, z) {
        true
    } else if world.is_block_normal_cube(x, y, z - 1) {
        true
    } else {
        world.is_block_normal_cube(x, y, z + 1)
    }
}

/// `onBlockPlaced(world, x, y, z, side)`: faces the clicked side when it
/// can hold the button, else the first support found; keeps the pressed bit.
// src: BlockButton.java:51-68
pub fn on_block_placed(world: &mut World, x: i32, y: i32, z: i32, side: i32) {
    let var6 = world.get_block_metadata(x, y, z);
    let var7 = var6 & 8;
    // src: BlockButton.java:54: `var6 &= 7`, overwritten by every branch below
    let var6 = if side == 2 && world.is_block_normal_cube(x, y, z + 1) {
        4
    } else if side == 3 && world.is_block_normal_cube(x, y, z - 1) {
        3
    } else if side == 4 && world.is_block_normal_cube(x + 1, y, z) {
        2
    } else if side == 5 && world.is_block_normal_cube(x - 1, y, z) {
        1
    } else {
        get_orientation(world, x, y, z)
    };
    world.set_block_metadata_with_notify(x, y, z, var6 + var7);
}

/// `getOrientation(world, x, y, z)` (private): the first support (-x, +x,
/// -z, +z), or 1 when there is none.
// src: BlockButton.java:70-80
fn get_orientation(world: &mut World, x: i32, y: i32, z: i32) -> i32 {
    if world.is_block_normal_cube(x - 1, y, z) {
        1
    } else if world.is_block_normal_cube(x + 1, y, z) {
        2
    } else if world.is_block_normal_cube(x, y, z - 1) {
        3
    } else if world.is_block_normal_cube(x, y, z + 1) {
        4
    } else {
        1
    }
}

/// `onNeighborBlockChange(world, x, y, z, neighborId)`: drops the button
/// without any support, or when the block it hangs on is gone.
// src: BlockButton.java:82-108
pub fn on_neighbor_block_change(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    if check_for_drop(world, id, x, y, z) {
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
        if var7 {
            let meta = world.get_block_metadata(x, y, z);
            dispatch::drop_block_as_item(world, id, x, y, z, meta);
            world.set_block_with_notify(x, y, z, 0);
        }
    }
}

/// `checkForDrop(world, x, y, z)` (private): drops and removes the button
/// when `canPlaceBlockAt` fails; true when it stays.
// src: BlockButton.java:110-118
fn check_for_drop(world: &mut World, id: i32, x: i32, y: i32, z: i32) -> bool {
    if !can_place_block_at(world, x, y, z) {
        let meta = world.get_block_metadata(x, y, z);
        dispatch::drop_block_as_item(world, id, x, y, z, meta);
        world.set_block_with_notify(x, y, z, 0);
        false
    } else {
        true
    }
}

/// `setBlockBoundsBasedOnState(blockAccess, x, y, z)`: the button's box on
/// its face, half as deep when pressed. Metadata without a face (0, 5-7)
/// leaves the singleton's bounds as they were.
// src: BlockButton.java:120-142
pub fn set_block_bounds_based_on_state(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    let var5 = world.get_block_metadata(x, y, z);
    let var6 = var5 & 7;
    let var7 = (var5 & 8) > 0;
    let var8 = 0.375f32;
    let var9 = 0.625f32;
    let var10 = 0.1875f32;
    let mut var11 = 0.125f32;
    if var7 {
        var11 = 0.0625;
    }
    let bounds = if var6 == 1 {
        BlockBounds::from_floats(0.0, var8, 0.5 - var10, var11, var9, 0.5 + var10)
    } else if var6 == 2 {
        BlockBounds::from_floats(1.0 - var11, var8, 0.5 - var10, 1.0, var9, 0.5 + var10)
    } else if var6 == 3 {
        BlockBounds::from_floats(0.5 - var10, var8, 0.0, 0.5 + var10, var9, var11)
    } else if var6 == 4 {
        BlockBounds::from_floats(0.5 - var10, var8, 1.0 - var11, 0.5 + var10, var9, 1.0)
    } else {
        return;
    };
    world.block_statics.bounds[(id & 255) as usize] = bounds;
}

/// The notifications of a button or lever that changed state: its own
/// neighbours, then the neighbours of the block it hangs on (below for any
/// face but 1-4).
// src: BlockButton.java:158-169, :179-191, :230-242; BlockLever.java:163-174, :183-195
pub(super) fn notify_attached(world: &mut World, id: i32, x: i32, y: i32, z: i32, face: i32) {
    world.notify_blocks_of_neighbor_change(x, y, z, id);
    if face == 1 {
        world.notify_blocks_of_neighbor_change(x - 1, y, z, id);
    } else if face == 2 {
        world.notify_blocks_of_neighbor_change(x + 1, y, z, id);
    } else if face == 3 {
        world.notify_blocks_of_neighbor_change(x, y, z - 1, id);
    } else if face == 4 {
        world.notify_blocks_of_neighbor_change(x, y, z + 1, id);
    } else {
        world.notify_blocks_of_neighbor_change(x, y - 1, z, id);
    }
}

/// `onBlockClicked(world, x, y, z, player)`: `blockActivated`.
// src: BlockButton.java:144-146
pub fn on_block_clicked(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    block_activated(world, id, x, y, z);
}

/// `blockActivated(world, x, y, z, player)`: presses an unpressed button
/// (metadata, render update, click sound, notifications, a tick in
/// `tickRate()` = 20 to release it); always true.
// src: BlockButton.java:148-174
pub fn block_activated(world: &mut World, id: i32, x: i32, y: i32, z: i32) -> bool {
    let var6 = world.get_block_metadata(x, y, z);
    let var7 = var6 & 7;
    let var8 = 8 - (var6 & 8);
    if var8 == 0 {
        true
    } else {
        world.set_block_metadata_with_notify(x, y, z, var7 + var8);
        world.mark_blocks_dirty(x, y, z, x, y, z);
        world.play_sound_effect(
            x as f64 + 0.5,
            y as f64 + 0.5,
            z as f64 + 0.5,
            "random.click",
            0.3,
            0.6,
        );
        notify_attached(world, id, x, y, z, var7);
        world.schedule_block_update(x, y, z, id, dispatch::tick_rate(id));
        true
    }
}

/// `onBlockRemoval(world, x, y, z)`: a pressed button notifies as on
/// release; then `super.onBlockRemoval` (empty). The metadata read is the
/// removed button's (Chunk.java:245-251 replaces the id first and the
/// metadata after this call).
// src: BlockButton.java:176-195
pub fn on_block_removal(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    let var5 = world.get_block_metadata(x, y, z);
    if (var5 & 8) > 0 {
        notify_attached(world, id, x, y, z, var5 & 7);
    }
}

/// `isPoweringTo(blockAccess, x, y, z, side)`: pressed.
// src: BlockButton.java:197-199
pub fn is_powering_to(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    (world.get_block_metadata(x, y, z) & 8) > 0
}

/// `isIndirectlyPoweringTo(world, x, y, z, side)`: pressed, toward the block
/// it hangs on only.
// src: BlockButton.java:201-219
pub fn is_indirectly_powering_to(world: &mut World, x: i32, y: i32, z: i32, side: i32) -> bool {
    let var6 = world.get_block_metadata(x, y, z);
    if (var6 & 8) == 0 {
        false
    } else {
        let var7 = var6 & 7;
        if var7 == 5 && side == 1 {
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
// src: BlockButton.java:221-223
pub fn can_provide_power() -> bool {
    true
}

/// `updateTick(world, x, y, z, rand)`: off a multiplayer client, releases a
/// pressed button (metadata, notifications, click sound, render update).
// src: BlockButton.java:225-248
pub fn update_tick(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    if !world.multiplayer_world {
        let var6 = world.get_block_metadata(x, y, z);
        if (var6 & 8) != 0 {
            world.set_block_metadata_with_notify(x, y, z, var6 & 7);
            notify_attached(world, id, x, y, z, var6 & 7);
            world.play_sound_effect(
                x as f64 + 0.5,
                y as f64 + 0.5,
                z as f64 + 0.5,
                "random.click",
                0.3,
                0.5,
            );
            world.mark_blocks_dirty(x, y, z, x, y, z);
        }
    }
}

/// `setBlockBoundsForItemRender()`: the item-render box (client only).
// src: BlockButton.java:250-255
pub fn set_block_bounds_for_item_render(world: &mut World, id: i32) {
    let var1 = 0.1875f32;
    let var2 = 0.125f32;
    let var3 = 0.125f32;
    world.block_statics.bounds[(id & 255) as usize] = BlockBounds::from_floats(
        0.5 - var1,
        0.5 - var2,
        0.5 - var3,
        0.5 + var1,
        0.5 + var2,
        0.5 + var3,
    );
}
