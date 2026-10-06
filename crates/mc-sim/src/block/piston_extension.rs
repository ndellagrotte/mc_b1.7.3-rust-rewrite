//! `BlockPistonExtension` (BlockPistonExtension.java), the piston head
//! (id 34, [`super::BlockKind::PistonExtension`]). Metadata: the facing side
//! in bits 0-2 (`getDirectionMeta`), bit 3 for a sticky piston's head.
//!
//! Fully transcribed: `setHeadTexture`, `clearHeadTexture`,
//! `onBlockRemoval`, `canPlaceBlockAt`, `canPlaceBlockOnSide`,
//! `getCollidingBoundingBoxes`, `setBlockBoundsBasedOnState`,
//! `onNeighborBlockChange`, `getDirectionMeta`. Data methods
//! (`getBlockTextureFromSideAndMetadata`, `getRenderType`, `isOpaqueCube`,
//! `renderAsNormalBlock`, `quantityDropped`) are in [`crate::block`];
//! `headTexture` is [`crate::block::BlockStatics::piston_extension_head_texture`].
//!
//! A head whose metadata says side 6 or 7 (only a hand-set metadata) makes
//! Java index past `faceToSide` / the offset tables and throw
//! (`ArrayIndexOutOfBoundsException`, an exception vanilla would crash on);
//! the port returns from the method at that point instead and says so at
//! each site.

use mc_core::Aabb;

use super::piston_base::{
    OFFSETS_X_FOR_SIDE, OFFSETS_Y_FOR_SIDE, OFFSETS_Z_FOR_SIDE, PISTON_BASE_ID,
    PISTON_STICKY_BASE_ID, base_get_colliding_bounding_boxes, is_powered,
};
use super::{BlockBounds, FACE_TO_SIDE, dispatch};
use crate::world::World;

/// `setHeadTexture(texture)`: the renderer forces this texture on the head
/// face while drawing a retracting piston (TileEntityRendererPiston.java:30).
/// Client.
// src: BlockPistonExtension.java:15-17
pub fn set_head_texture(world: &mut World, var1: i32) {
    world.block_statics.piston_extension_head_texture = var1;
}

/// `clearHeadTexture()`. Client.
// src: BlockPistonExtension.java:19-21
pub fn clear_head_texture(world: &mut World) {
    world.block_statics.piston_extension_head_texture = -1;
}

/// `getDirectionMeta(meta)`: `meta & 7`.
// src: BlockPistonExtension.java:156-158
pub const fn get_direction_meta(meta: i32) -> i32 {
    meta & 7
}

/// The step toward `side`, `None` past the tables (module docs).
fn step(side: i32) -> Option<(i32, i32, i32)> {
    let s = usize::try_from(side).ok()?;
    Some((
        *OFFSETS_X_FOR_SIDE.get(s)?,
        *OFFSETS_Y_FOR_SIDE.get(s)?,
        *OFFSETS_Z_FOR_SIDE.get(s)?,
    ))
}

/// `onBlockRemoval(world, x, y, z)`: removing the head of an extended
/// piston breaks the piston (dropped as an item). `super.onBlockRemoval` is
/// `Block`'s (empty).
// src: BlockPistonExtension.java:23-39
pub fn on_block_removal(world: &mut World, x: i32, y: i32, z: i32) {
    let var5 = world.get_block_metadata(x, y, z);
    // Java throws for direction 6 or 7 (module docs).
    let Some(&var6) = FACE_TO_SIDE.get(get_direction_meta(var5) as usize) else {
        return;
    };
    let Some((dx, dy, dz)) = step(var6) else {
        return;
    };
    let (x, y, z) = (x + dx, y + dy, z + dz);
    let var7 = world.get_block_id(x, y, z);
    if var7 == PISTON_BASE_ID || var7 == PISTON_STICKY_BASE_ID {
        let var5 = world.get_block_metadata(x, y, z);
        if is_powered(var5) {
            // src: BlockPistonExtension.java:34-35
            dispatch::drop_block_as_item(world, var7, x, y, z, var5);
            world.set_block_with_notify(x, y, z, 0);
        }
    }
}

/// `canPlaceBlockAt`: false.
// src: BlockPistonExtension.java:66-68
pub fn can_place_block_at() -> bool {
    false
}

/// `canPlaceBlockOnSide`: false.
// src: BlockPistonExtension.java:70-72
pub fn can_place_block_on_side() -> bool {
    false
}

/// `getCollidingBoundingBoxes(world, x, y, z, mask, list)`: the head plate
/// and the arm as two boxes (each through the base body with the bounds set
/// to it), then the bounds back to a full cube.
// src: BlockPistonExtension.java:78-119
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
    let boxes = match get_direction_meta(var7) {
        // src: BlockPistonExtension.java:81-85
        0 => Some([
            BlockBounds::from_floats(0.0, 0.0, 0.0, 1.0, 0.25, 1.0),
            BlockBounds::from_floats(0.375, 0.25, 0.375, 0.625, 1.0, 0.625),
        ]),
        // src: BlockPistonExtension.java:87-91
        1 => Some([
            BlockBounds::from_floats(0.0, 0.75, 0.0, 1.0, 1.0, 1.0),
            BlockBounds::from_floats(0.375, 0.0, 0.375, 0.625, 0.75, 0.625),
        ]),
        // src: BlockPistonExtension.java:93-97
        2 => Some([
            BlockBounds::from_floats(0.0, 0.0, 0.0, 1.0, 1.0, 0.25),
            BlockBounds::from_floats(0.25, 0.375, 0.25, 0.75, 0.625, 1.0),
        ]),
        // src: BlockPistonExtension.java:99-103
        3 => Some([
            BlockBounds::from_floats(0.0, 0.0, 0.75, 1.0, 1.0, 1.0),
            BlockBounds::from_floats(0.25, 0.375, 0.0, 0.75, 0.625, 0.75),
        ]),
        // src: BlockPistonExtension.java:105-109
        4 => Some([
            BlockBounds::from_floats(0.0, 0.0, 0.0, 0.25, 1.0, 1.0),
            BlockBounds::from_floats(0.375, 0.25, 0.25, 0.625, 0.75, 1.0),
        ]),
        // src: BlockPistonExtension.java:111-115
        5 => Some([
            BlockBounds::from_floats(0.75, 0.0, 0.0, 1.0, 1.0, 1.0),
            BlockBounds::from_floats(0.0, 0.375, 0.25, 0.75, 0.625, 0.75),
        ]),
        // no default case
        _ => None,
    };
    for b in boxes.into_iter().flatten() {
        world.block_statics.bounds[(id & 255) as usize] = b;
        base_get_colliding_bounding_boxes(world, id, x, y, z, mask, list);
    }
    // src: BlockPistonExtension.java:118
    world.block_statics.bounds[(id & 255) as usize] = BlockBounds::FULL;
}

/// `setBlockBoundsBasedOnState(world, x, y, z)`: the head plate.
// src: BlockPistonExtension.java:121-143
pub fn set_block_bounds_based_on_state(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    let var5 = world.get_block_metadata(x, y, z);
    let b = match get_direction_meta(var5) {
        0 => BlockBounds::from_floats(0.0, 0.0, 0.0, 1.0, 0.25, 1.0),
        1 => BlockBounds::from_floats(0.0, 0.75, 0.0, 1.0, 1.0, 1.0),
        2 => BlockBounds::from_floats(0.0, 0.0, 0.0, 1.0, 1.0, 0.25),
        3 => BlockBounds::from_floats(0.0, 0.0, 0.75, 1.0, 1.0, 1.0),
        4 => BlockBounds::from_floats(0.0, 0.0, 0.0, 0.25, 1.0, 1.0),
        5 => BlockBounds::from_floats(0.75, 0.0, 0.0, 1.0, 1.0, 1.0),
        // no default case: bounds unchanged
        _ => return,
    };
    world.block_statics.bounds[(id & 255) as usize] = b;
}

/// `onNeighborBlockChange(world, x, y, z, neighborId)`: a head with no
/// piston behind it removes itself; otherwise the piston behind gets the
/// neighbour change (with the same `neighborId`).
// src: BlockPistonExtension.java:145-154
pub fn on_neighbor_block_change(world: &mut World, x: i32, y: i32, z: i32, var5: i32) {
    let var6 = get_direction_meta(world.get_block_metadata(x, y, z));
    // Java throws for direction 6 or 7 (module docs).
    let Some((dx, dy, dz)) = step(var6) else {
        return;
    };
    let var7 = world.get_block_id(x - dx, y - dy, z - dz);
    if var7 != PISTON_BASE_ID && var7 != PISTON_STICKY_BASE_ID {
        world.set_block_with_notify(x, y, z, 0);
    } else {
        dispatch::on_neighbor_block_change(world, var7, x - dx, y - dy, z - dz, var5);
    }
}
