//! `BlockSign` (BlockSign.java): the sign post (63, `isFreestanding`) and
//! the wall sign (68). Both hold a `TileEntitySign`
//! ([`crate::tile_entity::sign`]); the text is edited by the client.

use mc_core::Aabb;

use super::{BlockBounds, BlockKind, block, dispatch};
use crate::world::World;

/// `isFreestanding` of the singleton `id`.
fn is_freestanding(id: i32) -> bool {
    matches!(
        block(id).map(|b| b.kind),
        Some(BlockKind::Sign {
            is_freestanding: true
        })
    )
}

/// `getCollisionBoundingBoxFromPool(world, x, y, z)`: none, signs do not
/// collide.
// src: BlockSign.java:19-21
pub fn get_collision_bounding_box_from_pool() -> Option<Aabb> {
    None
}

/// `setBlockBoundsBasedOnState(blockAccess, x, y, z)`: a wall sign takes
/// the thin box against the wall its metadata names (2 against +z, 3 -z, 4
/// +x, 5 -x; the full cube for any other value); the sign post keeps its
/// constructor bounds.
// src: BlockSign.java:28-54
pub fn set_block_bounds_based_on_state(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    if !is_freestanding(id) {
        let var5 = world.get_block_metadata(x, y, z);
        let var6 = 0.28125f32;
        let var7 = 0.78125f32;
        let var8 = 0.0f32;
        let var9 = 1.0f32;
        let var10 = 0.125f32;
        let b = &mut world.block_statics.bounds[(id & 255) as usize];
        *b = BlockBounds::from_floats(0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
        if var5 == 2 {
            *b = BlockBounds::from_floats(var8, var6, 1.0 - var10, var9, var7, 1.0);
        }
        if var5 == 3 {
            *b = BlockBounds::from_floats(var8, var6, 0.0, var9, var7, var10);
        }
        if var5 == 4 {
            *b = BlockBounds::from_floats(1.0 - var10, var6, var8, 1.0, var7, var9);
        }
        if var5 == 5 {
            *b = BlockBounds::from_floats(0.0, var6, var8, var10, var7, var9);
        }
    }
}

/// `onNeighborBlockChange(world, x, y, z, neighborId)`: a sign post
/// without a solid block below, or a wall sign without a solid block
/// behind it (any metadata but 2-5 always counts as unsupported), drops as
/// an item and is removed; then `super.onNeighborBlockChange` (Block's,
/// empty).
// src: BlockSign.java:80-111
pub fn on_neighbor_block_change(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    let mut var6 = false;
    if is_freestanding(id) {
        if !world.get_block_material(x, y - 1, z).is_solid() {
            var6 = true;
        }
    } else {
        let var7 = world.get_block_metadata(x, y, z);
        var6 = true;
        if var7 == 2 && world.get_block_material(x, y, z + 1).is_solid() {
            var6 = false;
        }
        if var7 == 3 && world.get_block_material(x, y, z - 1).is_solid() {
            var6 = false;
        }
        if var7 == 4 && world.get_block_material(x + 1, y, z).is_solid() {
            var6 = false;
        }
        if var7 == 5 && world.get_block_material(x - 1, y, z).is_solid() {
            var6 = false;
        }
    }
    if var6 {
        let meta = world.get_block_metadata(x, y, z);
        dispatch::drop_block_as_item(world, id, x, y, z, meta);
        world.set_block_with_notify(x, y, z, 0);
    }
}
