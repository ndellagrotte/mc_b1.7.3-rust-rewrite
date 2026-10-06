//! `BlockPistonMoving` (BlockPistonMoving.java), the block in a cell while
//! a piston moves something into it (id 36, [`super::BlockKind::PistonMoving`],
//! a `BlockContainer`). What it carries and how far it has moved is its
//! [`TileEntityPiston`] (in [`crate::world::tile_entity`], with the tile
//! entity's own methods).
//!
//! Fully transcribed: `getBlockEntity`, `onBlockAdded`, `onBlockRemoval`,
//! `canPlaceBlockAt`, `canPlaceBlockOnSide`, `blockActivated`,
//! `dropBlockAsItemWithChance`, `onNeighborBlockChange`,
//! `getNewTileEntity`, `getCollisionBoundingBoxFromPool`,
//! `setBlockBoundsBasedOnState`, `getAxisAlignedBB`,
//! `getTileEntityAtLocation`. Data methods (`getRenderType`, `isOpaqueCube`,
//! `renderAsNormalBlock`, `idDropped`) are in [`crate::block`].

use mc_core::Aabb;

use super::piston_base::{OFFSETS_X_FOR_SIDE, OFFSETS_Y_FOR_SIDE, OFFSETS_Z_FOR_SIDE};
use super::{BlockBounds, block, dispatch};
use crate::world::World;
use crate::world::tile_entity::{
    self, TileEntity, TileEntityKindData, TileEntityPiston, TileEntityRef,
};

/// `Block.pistonMoving.blockID`.
// src: Block.java:628
pub const PISTON_MOVING_ID: i32 = 36;

/// `getBlockEntity()`: null. The moving block's tile entity is never made by
/// `BlockContainer.onBlockAdded` (overridden, empty) but by the piston, with
/// [`get_new_tile_entity`].
// src: BlockPistonMoving.java:11-13
pub fn get_block_entity() -> Option<TileEntity> {
    None
}

/// `onBlockAdded`: nothing (not `BlockContainer`'s, which would add a tile
/// entity).
// src: BlockPistonMoving.java:15-16
pub fn on_block_added() {}

/// `onBlockRemoval(world, x, y, z)`: finishes the move of the tile entity
/// still at the cell ([`tile_entity::clear_piston_tile_entity`]; the cell no
/// longer holds the moving block then, so it only removes the tile entity),
/// or else runs `BlockContainer.onBlockRemoval` (`Block`'s empty body, then
/// `removeBlockTileEntity`).
// src: BlockPistonMoving.java:18-26
pub fn on_block_removal(world: &mut World, x: i32, y: i32, z: i32) {
    match get_tile_entity_at_location(world, x, y, z) {
        Some(r) => tile_entity::clear_piston_tile_entity(world, r),
        None => {
            // src: BlockContainer.java:19-22
            world.remove_block_tile_entity(x, y, z);
        }
    }
}

/// `canPlaceBlockAt`: false.
// src: BlockPistonMoving.java:28-30
pub fn can_place_block_at() -> bool {
    false
}

/// `canPlaceBlockOnSide`: false.
// src: BlockPistonMoving.java:32-34
pub fn can_place_block_on_side() -> bool {
    false
}

/// `blockActivated(world, x, y, z, player)`: right-clicking a moving block
/// that has lost its tile entity removes it.
// src: BlockPistonMoving.java:48-55
pub fn block_activated(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    if !world.multiplayer_world && world.get_block_tile_entity(x, y, z).is_none() {
        world.set_block_with_notify(x, y, z, 0);
        true
    } else {
        false
    }
}

/// `dropBlockAsItemWithChance(world, x, y, z, meta, chance)`: drops what the
/// tile entity carries (`dropBlockAsItem` of the stored block with the
/// stored metadata; `chance` unused). Java dereferences
/// `Block.blocksList[storedBlockID]` without a null check (a stored id of 0
/// would throw); the dispatch drops nothing for such an id.
// src: BlockPistonMoving.java:61-68
pub fn drop_block_as_item_with_chance(world: &mut World, x: i32, y: i32, z: i32) {
    if !world.multiplayer_world
        && let Some(r) = get_tile_entity_at_location(world, x, y, z)
        && let Some(p) = tile_entity::piston(world, r)
    {
        let (id, meta) = (p.get_stored_block_id(), p.get_block_metadata());
        dispatch::drop_block_as_item(world, id, x, y, z, meta);
    }
}

/// `onNeighborBlockChange`: an empty `if` around `getBlockTileEntity`, whose
/// lookup still runs (it can drop an invalid tile entity from the chunk map,
/// Chunk.java:415-417).
// src: BlockPistonMoving.java:70-74
pub fn on_neighbor_block_change(world: &mut World, x: i32, y: i32, z: i32) {
    if !world.multiplayer_world && world.get_block_tile_entity(x, y, z).is_none() {}
}

/// `getNewTileEntity(storedBlockID, storedMetadata, orientation, extending,
/// shouldHeadBeRendered)`: a new `TileEntityPiston`, not yet placed
/// (`World.setBlockTileEntity` does that).
// src: BlockPistonMoving.java:76-78
pub fn get_new_tile_entity(
    world: &mut World,
    var0: i32,
    var1: i32,
    var2: i32,
    var3: bool,
    var4: bool,
) -> TileEntityRef {
    world.new_tile_entity(TileEntity {
        x_coord: 0,
        y_coord: 0,
        z_coord: 0,
        field_31007_h: false,
        kind: TileEntityKindData::Piston(TileEntityPiston::new(var0, var1, var2, var3, var4)),
    })
}

/// `getCollisionBoundingBoxFromPool(world, x, y, z)`: the stored block's
/// collision box, shifted back along the move by what is left of it
/// (`1 - progress` while extending, `progress` while retracting, at partial
/// tick 0); `None` without a tile entity.
// src: BlockPistonMoving.java:80-92
pub fn get_collision_bounding_box_from_pool(
    world: &mut World,
    x: i32,
    y: i32,
    z: i32,
) -> Option<Aabb> {
    let r = get_tile_entity_at_location(world, x, y, z)?;
    let p = tile_entity::piston(world, r)?.clone();
    let mut var6 = p.get_progress(0.0);
    if p.is_extending() {
        var6 = 1.0 - var6;
    }
    get_axis_aligned_bb(
        world,
        x,
        y,
        z,
        p.get_stored_block_id(),
        var6,
        p.get_piston_orientation(),
    )
}

/// `setBlockBoundsBasedOnState(world, x, y, z)`: the stored block's bounds
/// (after its own `setBlockBoundsBasedOnState` at this cell), shifted back
/// along the move as in [`get_collision_bounding_box_from_pool`]. Nothing
/// without a tile entity, or when the stored block is air or itself.
// src: BlockPistonMoving.java:94-117
pub fn set_block_bounds_based_on_state(world: &mut World, x: i32, y: i32, z: i32) {
    let Some(r) = get_tile_entity_at_location(world, x, y, z) else {
        return;
    };
    let Some(p) = tile_entity::piston(world, r).cloned() else {
        return;
    };
    let var6 = p.get_stored_block_id();
    if block(var6).is_none() || var6 == PISTON_MOVING_ID {
        return;
    }
    dispatch::set_block_bounds_based_on_state(world, var6, x, y, z);
    let mut var7 = p.get_progress(0.0);
    if p.is_extending() {
        var7 = 1.0 - var7;
    }
    let var8 = p.get_piston_orientation() as usize;
    // src: BlockPistonMoving.java:108-113: `(double)((float)offset * var7)`
    let ox = (OFFSETS_X_FOR_SIDE[var8] as f32 * var7) as f64;
    let oy = (OFFSETS_Y_FOR_SIDE[var8] as f32 * var7) as f64;
    let oz = (OFFSETS_Z_FOR_SIDE[var8] as f32 * var7) as f64;
    let s = world.block_statics.bounds[(var6 & 255) as usize];
    world.block_statics.bounds[PISTON_MOVING_ID as usize] = BlockBounds {
        min_x: s.min_x - ox,
        min_y: s.min_y - oy,
        min_z: s.min_z - oz,
        max_x: s.max_x - ox,
        max_y: s.max_y - oy,
        max_z: s.max_z - oz,
    };
}

/// `getAxisAlignedBB(world, x, y, z, storedBlockID, progress, orientation)`:
/// the stored block's collision box at this cell
/// (`getCollisionBoundingBoxFromPool` through the dispatch) moved back by
/// `progress` along `orientation`; `None` for air, the moving block itself,
/// or a block without a collision box.
// src: BlockPistonMoving.java:119-136
pub fn get_axis_aligned_bb(
    world: &mut World,
    x: i32,
    y: i32,
    z: i32,
    var5: i32,
    var6: f32,
    var7: i32,
) -> Option<Aabb> {
    if var5 != 0 && var5 != PISTON_MOVING_ID {
        let mut var8 = dispatch::get_collision_bounding_box_from_pool(world, var5, x, y, z)?;
        let f = var7 as usize;
        // src: BlockPistonMoving.java:125-130, in Java's order
        var8.min_x -= (OFFSETS_X_FOR_SIDE[f] as f32 * var6) as f64;
        var8.max_x -= (OFFSETS_X_FOR_SIDE[f] as f32 * var6) as f64;
        var8.min_y -= (OFFSETS_Y_FOR_SIDE[f] as f32 * var6) as f64;
        var8.max_y -= (OFFSETS_Y_FOR_SIDE[f] as f32 * var6) as f64;
        var8.min_z -= (OFFSETS_Z_FOR_SIDE[f] as f32 * var6) as f64;
        var8.max_z -= (OFFSETS_Z_FOR_SIDE[f] as f32 * var6) as f64;
        Some(var8)
    } else {
        None
    }
}

/// `getTileEntityAtLocation(world, x, y, z)` (private): the cell's tile
/// entity if it is a `TileEntityPiston`.
// src: BlockPistonMoving.java:138-141
pub(super) fn get_tile_entity_at_location(
    world: &mut World,
    x: i32,
    y: i32,
    z: i32,
) -> Option<TileEntityRef> {
    let r = world.get_block_tile_entity(x, y, z)?;
    tile_entity::piston(world, r).map(|_| r)
}
