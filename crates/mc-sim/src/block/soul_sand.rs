//! `BlockSoulSand` (BlockSoulSand.java): a collision box 1/8 lower than the
//! cell, so entities sink into it, and a horizontal slow-down for whatever
//! touches it. (`velocityToAddToEntity` is not overridden.)

use mc_core::Aabb;

use crate::entity::Entity;

/// `BlockSoulSand.getCollisionBoundingBoxFromPool(world, x, y, z)`: the full
/// cell with the top lowered by `0.125F`, computed in `float`
/// (`(float)(y + 1) - var5`).
// src: BlockSoulSand.java:8-11
pub fn get_collision_bounding_box_from_pool(x: i32, y: i32, z: i32) -> Option<Aabb> {
    let var5 = 0.125f32;
    Some(Aabb::new(
        x as f64,
        y as f64,
        z as f64,
        x.wrapping_add(1) as f64,
        ((y.wrapping_add(1)) as f32 - var5) as f64,
        z.wrapping_add(1) as f64,
    ))
}

/// `BlockSoulSand.onEntityCollidedWithBlock(world, x, y, z, entity)`:
/// `motionX` and `motionZ` times 0.4.
// src: BlockSoulSand.java:13-16
pub fn on_entity_collided_with_block(e: &mut Entity) {
    e.base.motion_x *= 0.4;
    e.base.motion_z *= 0.4;
}
