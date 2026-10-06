//! `BlockFence` (BlockFence.java): the fence (id 85).
//!
//! Transcribed: `canPlaceBlockAt`, `getCollisionBoundingBoxFromPool` (1.5
//! high, so nothing jumps over it). Data methods (`isOpaqueCube`,
//! `renderAsNormalBlock`, `getRenderType`) are in [`crate::block`]. The
//! singleton's bounds stay the full cube (it never sets them), which is what
//! the selection box and ray traces use.

use mc_core::Aabb;

use crate::world::World;

/// `canPlaceBlockAt(world, x, y, z)`: on another fence, or on a solid
/// material where `Block`'s test allows.
// src: BlockFence.java:8-14
pub fn can_place_block_at(world: &mut World, id: i32, x: i32, y: i32, z: i32) -> bool {
    if world.get_block_id(x, y - 1, z) == id {
        true
    } else if !world.get_block_material(x, y - 1, z).is_solid() {
        false
    } else {
        super::dispatch::base_can_place_block_at(world, x, y, z)
    }
}

/// `getCollisionBoundingBoxFromPool(world, x, y, z)`: the cell, 1.5 high
/// (`(double)((float)y + 1.5F)`).
// src: BlockFence.java:16-18
pub fn get_collision_bounding_box_from_pool(x: i32, y: i32, z: i32) -> Aabb {
    Aabb::new(
        x as f64,
        y as f64,
        z as f64,
        (x + 1) as f64,
        (y as f32 + 1.5) as f64,
        (z + 1) as f64,
    )
}
