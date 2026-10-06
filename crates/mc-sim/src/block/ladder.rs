//! `BlockLadder` (BlockLadder.java): the ladder (id 65).
//!
//! Metadata 2..5: the ladder hangs on the block at z + 1, z - 1, x + 1,
//! x - 1 respectively (any other value keeps the bounds it had).
//!
//! Transcribed: `getCollisionBoundingBoxFromPool` and
//! `getSelectedBoundingBoxFromPool` (bounds from the metadata,
//! [`set_bounds_from_metadata`], then `Block`'s body in the dispatch arms),
//! `canPlaceBlockAt`, `onBlockPlaced`, `onNeighborBlockChange`. Data methods
//! (`isOpaqueCube`, `renderAsNormalBlock`, `getRenderType`,
//! `quantityDropped`) are in [`crate::block`].

use super::{BlockBounds, dispatch};
use crate::world::World;

/// The shared head of `getCollisionBoundingBoxFromPool` and
/// `getSelectedBoundingBoxFromPool`: a 1/8 thick slab against the side the
/// metadata names. Other metadata leaves the singleton's bounds as they are.
// src: BlockLadder.java:10-30, :32-52
pub fn set_bounds_from_metadata(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    let var5 = world.get_block_metadata(x, y, z);
    let var6 = 0.125f32;
    let b = &mut world.block_statics.bounds[(id & 255) as usize];
    if var5 == 2 {
        *b = BlockBounds::from_floats(0.0, 0.0, 1.0 - var6, 1.0, 1.0, 1.0);
    }
    if var5 == 3 {
        *b = BlockBounds::from_floats(0.0, 0.0, 0.0, 1.0, 1.0, var6);
    }
    if var5 == 4 {
        *b = BlockBounds::from_floats(1.0 - var6, 0.0, 0.0, 1.0, 1.0, 1.0);
    }
    if var5 == 5 {
        *b = BlockBounds::from_floats(0.0, 0.0, 0.0, var6, 1.0, 1.0);
    }
}

/// `canPlaceBlockAt(world, x, y, z)`: a normal cube on any horizontal side
/// (-x, +x, -z, +z).
// src: BlockLadder.java:66-76
pub fn can_place_block_at(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    world.is_block_normal_cube(x - 1, y, z)
        || world.is_block_normal_cube(x + 1, y, z)
        || world.is_block_normal_cube(x, y, z - 1)
        || world.is_block_normal_cube(x, y, z + 1)
}

/// `onBlockPlaced(world, x, y, z, side)`: the metadata from the face it was
/// placed against, or the last supporting side found when that face does
/// not hold it.
// src: BlockLadder.java:78-97
pub fn on_block_placed(world: &mut World, x: i32, y: i32, z: i32, var5: i32) {
    let mut var6 = world.get_block_metadata(x, y, z);
    if (var6 == 0 || var5 == 2) && world.is_block_normal_cube(x, y, z + 1) {
        var6 = 2;
    }
    if (var6 == 0 || var5 == 3) && world.is_block_normal_cube(x, y, z - 1) {
        var6 = 3;
    }
    if (var6 == 0 || var5 == 4) && world.is_block_normal_cube(x + 1, y, z) {
        var6 = 4;
    }
    if (var6 == 0 || var5 == 5) && world.is_block_normal_cube(x - 1, y, z) {
        var6 = 5;
    }
    world.set_block_metadata_with_notify(x, y, z, var6);
}

/// `onNeighborBlockChange(world, x, y, z, neighborId)`: drops the ladder
/// (item first, then the block) when the block it hangs on is no longer a
/// normal cube; then `Block`'s body (nothing).
// src: BlockLadder.java:99-124
pub fn on_neighbor_block_change(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    let var6 = world.get_block_metadata(x, y, z);
    let mut var7 = false;
    if var6 == 2 && world.is_block_normal_cube(x, y, z + 1) {
        var7 = true;
    }
    if var6 == 3 && world.is_block_normal_cube(x, y, z - 1) {
        var7 = true;
    }
    if var6 == 4 && world.is_block_normal_cube(x + 1, y, z) {
        var7 = true;
    }
    if var6 == 5 && world.is_block_normal_cube(x - 1, y, z) {
        var7 = true;
    }
    if !var7 {
        dispatch::drop_block_as_item(world, id, x, y, z, var6);
        world.set_block_with_notify(x, y, z, 0);
    }
    // src: BlockLadder.java:123: super.onNeighborBlockChange, Block's empty
    // body (Block.java:306-307).
}
