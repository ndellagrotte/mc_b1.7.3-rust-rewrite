//! `BlockStairs` (BlockStairs.java): wooden (id 53, model block planks) and
//! cobblestone (id 67, model block cobblestone) stairs.
//!
//! Most overrides forward to `modelBlock`, and both model blocks are plain
//! `Block`s; those one-line forwards are the `BlockKind::Stairs` arms of
//! [`super::dispatch`], which call the entry point again with the model
//! block's id (so the model block's own bounds, drops and resistance apply,
//! as on Java's model instance). The overrides with a body of their own are
//! here: `onBlockAdded`, `getCollidingBoundingBoxes` (two boxes per
//! orientation) and `onBlockPlacedBy` (orientation from the placer's yaw).
//! `getCollisionBoundingBoxFromPool` and `shouldSideBeRendered` only call
//! `super`; `setBlockBoundsBasedOnState`, `canCollideCheck` and
//! `randomDisplayTick` were transcribed in WP-1.5.
//!
//! Metadata: 0..3, the side the step rises towards (0: +x, 1: -x, 2: +z,
//! 3: -z).

use mc_core::Aabb;
use mc_core::math::floor_double;

use super::{BlockBounds, dispatch};
use crate::entity::Entity;
use crate::world::World;

/// `onBlockAdded(world, x, y, z)`: `this.onNeighborBlockChange(world, x, y,
/// z, 0)` (not overridden by stairs: `Block`'s empty body), then the model
/// block's `onBlockAdded` (also `Block`'s).
// src: BlockStairs.java:137-140
pub fn on_block_added(world: &mut World, id: i32, model_block: i32, x: i32, y: i32, z: i32) {
    dispatch::on_neighbor_block_change(world, id, x, y, z, 0);
    dispatch::on_block_added(world, model_block, x, y, z);
}

/// `getCollidingBoundingBoxes(world, x, y, z, mask, list)`: the lower half
/// and the raised half as two boxes (metadata 0..3; other metadata adds
/// nothing), then the singleton's bounds back to the full cube.
// src: BlockStairs.java:42-67
#[allow(clippy::too_many_arguments)]
pub fn get_colliding_bounding_boxes(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    mask: &Aabb,
    list: &mut Vec<Aabb>,
) {
    let var7 = world.get_block_metadata(x, y, z);
    let halves = match var7 {
        0 => Some((
            BlockBounds::from_floats(0.0, 0.0, 0.0, 0.5, 0.5, 1.0),
            BlockBounds::from_floats(0.5, 0.0, 0.0, 1.0, 1.0, 1.0),
        )),
        1 => Some((
            BlockBounds::from_floats(0.0, 0.0, 0.0, 0.5, 1.0, 1.0),
            BlockBounds::from_floats(0.5, 0.0, 0.0, 1.0, 0.5, 1.0),
        )),
        2 => Some((
            BlockBounds::from_floats(0.0, 0.0, 0.0, 1.0, 0.5, 0.5),
            BlockBounds::from_floats(0.0, 0.0, 0.5, 1.0, 1.0, 1.0),
        )),
        3 => Some((
            BlockBounds::from_floats(0.0, 0.0, 0.0, 1.0, 1.0, 0.5),
            BlockBounds::from_floats(0.0, 0.0, 0.5, 1.0, 0.5, 1.0),
        )),
        _ => None,
    };
    if let Some((a, b)) = halves {
        world.block_statics.bounds[(id & 255) as usize] = a;
        super_get_colliding_bounding_boxes(world, id, x, y, z, mask, list);
        world.block_statics.bounds[(id & 255) as usize] = b;
        super_get_colliding_bounding_boxes(world, id, x, y, z, mask, list);
    }
    world.block_statics.bounds[(id & 255) as usize] = BlockBounds::FULL;
}

/// `super.getCollidingBoundingBoxes`, `Block`'s body: the (virtual)
/// collision box, which for stairs is `Block`'s from the current bounds,
/// added when it meets `mask`.
// src: Block.java:273-279
#[allow(clippy::too_many_arguments)]
fn super_get_colliding_bounding_boxes(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    mask: &Aabb,
    list: &mut Vec<Aabb>,
) {
    if let Some(var7) = dispatch::get_collision_bounding_box_from_pool(world, id, x, y, z)
        && mask.intersects_with(&var7)
    {
        list.push(var7);
    }
}

/// `onBlockPlacedBy(world, x, y, z, placer)`: the metadata from the
/// placer's yaw quadrant (0 → 2, 1 → 1, 2 → 3, 3 → 0), so the step rises
/// away from the placer.
// src: BlockStairs.java:166-184
pub fn on_block_placed_by(world: &mut World, x: i32, y: i32, z: i32, placer: &Entity) {
    // `var5.rotationYaw * 4.0F / 360.0F` in float, then `+ 0.5D`.
    let var6 = floor_double((placer.base.rotation_yaw * 4.0 / 360.0) as f64 + 0.5) & 3;
    if var6 == 0 {
        world.set_block_metadata_with_notify(x, y, z, 2);
    }
    if var6 == 1 {
        world.set_block_metadata_with_notify(x, y, z, 1);
    }
    if var6 == 2 {
        world.set_block_metadata_with_notify(x, y, z, 3);
    }
    if var6 == 3 {
        world.set_block_metadata_with_notify(x, y, z, 0);
    }
}
