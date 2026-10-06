//! `BlockPumpkin` (BlockPumpkin.java): pumpkin and jack-o'-lantern. The face
//! direction is the metadata, set from the placer's yaw; placing needs a
//! normal cube below.
//!
//! Data methods (both texture methods) are in [`crate::block`].
//! `onBlockAdded` only calls `super` (empty, Block.java:313-314) and the
//! class sets `tickOnLoad` without overriding `updateTick`, so neither has a
//! body here.

use mc_core::math::floor_double;

use crate::entity::Entity;
use crate::world::World;

/// `BlockPumpkin.canPlaceBlockAt(world, x, y, z)`: the base test (air or
/// ground cover at the spot) and a normal cube below.
// src: BlockPumpkin.java:50-53
pub fn can_place_block_at(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    let var5 = world.get_block_id(x, y, z);
    (var5 == 0 || super::material_of(var5).get_is_ground_cover())
        && world.is_block_normal_cube(x, y - 1, z)
}

/// `BlockPumpkin.onBlockPlacedBy(world, x, y, z, placer)`: metadata
/// `floor(yaw * 4 / 360 + 2.5) & 3` (the product and quotient in `float`,
/// the sum in `double`), with notification.
// src: BlockPumpkin.java:55-58
pub fn on_block_placed_by(world: &mut World, x: i32, y: i32, z: i32, placer: &Entity) {
    let var6 = floor_double((placer.base.rotation_yaw * 4.0f32 / 360.0f32) as f64 + 2.5) & 3;
    world.set_block_metadata_with_notify(x, y, z, var6);
}
