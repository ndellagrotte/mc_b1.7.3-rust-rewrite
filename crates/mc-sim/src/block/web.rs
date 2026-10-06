//! `BlockWeb` (BlockWeb.java): no collision box, and anything inside it is
//! flagged `isInWeb`, which the next `Entity.moveEntity` turns into a
//! slow-down (Entity.java:303-311, `crate::entity::base::move_entity`).
//!
//! Data methods (`isOpaqueCube`, `getRenderType`, `renderAsNormalBlock`,
//! `idDropped` string) are in [`crate::block`].

use mc_core::Aabb;

use crate::entity::Entity;

/// `BlockWeb.onEntityCollidedWithBlock(world, x, y, z, entity)`.
// src: BlockWeb.java:10-12
pub fn on_entity_collided_with_block(e: &mut Entity) {
    e.base.is_in_web = true;
}

/// `BlockWeb.getCollisionBoundingBoxFromPool(world, x, y, z)`: `null`.
// src: BlockWeb.java:18-20
pub const fn get_collision_bounding_box_from_pool() -> Option<Aabb> {
    None
}
