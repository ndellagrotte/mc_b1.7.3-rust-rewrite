//! `BlockPistonBase` (BlockPistonBase.java), the piston and the sticky
//! piston (ids 33 and 29, [`BlockKind::PistonBase`]), and the
//! `PistonBlockTextures` direction tables (PistonBlockTextures.java).
//!
//! Fully transcribed: `blockActivated`, `onBlockPlacedBy`,
//! `onNeighborBlockChange`, `onBlockAdded`, `updatePistonState`,
//! `isIndirectlyPowered`, `playBlock`, `setBlockBoundsBasedOnState`,
//! `setBlockBoundsForItemRender`, `getCollidingBoundingBoxes`,
//! `getOrientation`, `isPowered`, `determineOrientation`, `canPushBlock`,
//! `canExtend`, `tryExtend`, `getPistonExtensionTexture`. The data methods
//! (`getBlockTextureFromSideAndMetadata`, `getRenderType`, `isOpaqueCube`,
//! `renderAsNormalBlock`) are in [`crate::block`].
//!
//! # How a piston moves (summary of the Java)
//!
//! A neighbour change (or placement) runs `updatePistonState`: when the
//! piston is newly powered and `canExtend`, its metadata gets the powered
//! bit (8) and `World.playNoteAt(x, y, z, 0, facing)` calls `playBlock`,
//! which runs `tryExtend`: every block in the push line, from the far end
//! back to the piston, becomes a `BlockPistonMoving` (id 36) whose
//! [`TileEntityPiston`] stores the block and its metadata; the cell in front
//! of the piston stores the piston head (`BlockPistonExtension`, id 34). When
//! the piston loses power, `playBlock(…, 1, facing)` turns the piston itself
//! into a retracting moving block, removes the head, and a sticky piston
//! pulls the block two cells ahead into the head's cell. The tile entities
//! finish the move ([`crate::world::tile_entity::piston_update_entity`]).
//!
//! # Singleton state
//!
//! `ignoreUpdates` is an instance field of each of the two singletons:
//! [`crate::block::BlockStatics::piston_base_ignore_updates`] (id 33) and
//! [`crate::block::BlockStatics::piston_sticky_base_ignore_updates`] (id 29).
//! It is only ever true inside `playBlock` (which always clears it), so a new
//! world starting with false is the oracle's state too.
//!
//! [`TileEntityPiston`]: crate::world::tile_entity::TileEntityPiston

use mc_core::Aabb;
use mc_core::math::{self, floor_double};

use super::{BlockBounds, BlockKind, block, dispatch};
use crate::entity::Entity;
use crate::world::World;
use crate::world::tile_entity;

/// `Block.pistonStickyBase.blockID`.
// src: Block.java:621
pub const PISTON_STICKY_BASE_ID: i32 = 29;
/// `Block.pistonBase.blockID`.
// src: Block.java:625
pub const PISTON_BASE_ID: i32 = 33;
/// `Block.pistonExtension.blockID`.
// src: Block.java:626
pub const PISTON_EXTENSION_ID: i32 = 34;
/// `Block.obsidian.blockID`.
// src: Block.java:641
const OBSIDIAN_ID: i32 = 49;

/// `PistonBlockTextures.offsetsXForSide`: the x step toward each side.
// src: PistonBlockTextures.java:5
pub const OFFSETS_X_FOR_SIDE: [i32; 6] = [0, 0, 0, 0, -1, 1];
/// `PistonBlockTextures.offsetsYForSide`.
// src: PistonBlockTextures.java:6
pub const OFFSETS_Y_FOR_SIDE: [i32; 6] = [-1, 1, 0, 0, 0, 0];
/// `PistonBlockTextures.offsetsZForSide`.
// src: PistonBlockTextures.java:7
pub const OFFSETS_Z_FOR_SIDE: [i32; 6] = [0, 0, -1, 1, 0, 0];

/// The step `(dx, dy, dz)` toward `side`. Java indexes the three arrays and
/// throws for a side outside 0..=5 (a piston with orientation 6 or 7 never
/// reaches a lookup: `updatePistonState` skips meta 7 and `getOrientation`
/// 6 only comes from a hand-set metadata); here such a side panics the same
/// way.
fn step(side: i32) -> (i32, i32, i32) {
    let s = side as usize;
    (
        OFFSETS_X_FOR_SIDE[s],
        OFFSETS_Y_FOR_SIDE[s],
        OFFSETS_Z_FOR_SIDE[s],
    )
}

/// `isSticky` of the singleton with this id.
fn is_sticky(id: i32) -> bool {
    matches!(
        block(id).map(|b| b.kind),
        Some(BlockKind::PistonBase { is_sticky: true })
    )
}

/// `this.ignoreUpdates` of the singleton with this id.
// src: BlockPistonBase.java:7
fn ignore_updates(world: &World, id: i32) -> bool {
    world
        .block_statics
        .piston_ignore_updates(id)
        .unwrap_or(false)
}

/// `this.ignoreUpdates = v` on the singleton with this id.
// src: BlockPistonBase.java:7
fn set_ignore_updates(world: &mut World, id: i32, v: bool) {
    match id {
        PISTON_STICKY_BASE_ID => world.block_statics.piston_sticky_base_ignore_updates = v,
        PISTON_BASE_ID => world.block_statics.piston_base_ignore_updates = v,
        _ => {}
    }
}

/// `getPistonExtensionTexture()`: the head texture the renderer forces on
/// the retracting head (TileEntityRendererPiston.java:30). Client.
// src: BlockPistonBase.java:16-18
pub fn get_piston_extension_texture(id: i32) -> i32 {
    if is_sticky(id) { 106 } else { 107 }
}

/// `getOrientation(meta)`: the facing side, `meta & 7`.
// src: BlockPistonBase.java:216-218
pub const fn get_orientation(meta: i32) -> i32 {
    meta & 7
}

/// `isPowered(meta)`: the extended bit, `(meta & 8) != 0`.
// src: BlockPistonBase.java:220-222
pub const fn is_powered(meta: i32) -> bool {
    (meta & 8) != 0
}

/// `blockActivated`: false (no right-click action, and it consumes nothing).
// src: BlockPistonBase.java:39-41
pub fn block_activated() -> bool {
    false
}

/// `onBlockPlacedBy(world, x, y, z, placer)`: faces the piston from the
/// placer's position and yaw ([`determine_orientation`]), then updates it.
// src: BlockPistonBase.java:43-50
pub fn on_block_placed_by(world: &mut World, x: i32, y: i32, z: i32, placer: &Entity) {
    let var6 = determine_orientation(world, x, y, z, placer);
    world.set_block_metadata_with_notify(x, y, z, var6);
    if !world.multiplayer_world {
        update_piston_state(world, x, y, z);
    }
}

/// `onNeighborBlockChange(world, x, y, z, neighborId)`: updates the piston
/// unless it is inside its own `playBlock`.
// src: BlockPistonBase.java:52-57
pub fn on_neighbor_block_change(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    if !world.multiplayer_world && !ignore_updates(world, id) {
        update_piston_state(world, x, y, z);
    }
}

/// `onBlockAdded(world, x, y, z)`: updates the piston unless a tile entity
/// is at its cell. (`getBlockTileEntity` asks the chunk, which for a
/// non-container block returns null, Chunk.java:401-421: a piston base never
/// has one, so this always updates; transcribed as written.)
// src: BlockPistonBase.java:59-64
pub fn on_block_added(world: &mut World, x: i32, y: i32, z: i32) {
    if !world.multiplayer_world && world.get_block_tile_entity(x, y, z).is_none() {
        update_piston_state(world, x, y, z);
    }
}

/// The power test `updatePistonState` asks: `World.isBlockIndirectlyProvidingPowerTo`.
fn world_power(world: &mut World, x: i32, y: i32, z: i32, side: i32) -> bool {
    world.is_block_indirectly_providing_power_to(x, y, z, side)
}

/// `updatePistonState(world, x, y, z)` (private): extends a newly powered
/// piston that can extend (metadata first, without notification, then the
/// block event 0), retracts an unpowered extended one (metadata, then block
/// event 1). Metadata 7 (the item's placement value, ItemPiston.java:8-10,
/// before `onBlockPlacedBy` sets the facing) does nothing.
// src: BlockPistonBase.java:66-82
fn update_piston_state(world: &mut World, x: i32, y: i32, z: i32) {
    update_piston_state_with(world, x, y, z, &mut world_power);
}

/// [`update_piston_state`] with the power test passed in, so unit tests can
/// power a piston before the redstone blocks exist (the only caller outside
/// the tests passes [`world_power`]).
fn update_piston_state_with(
    world: &mut World,
    x: i32,
    y: i32,
    z: i32,
    power: &mut dyn FnMut(&mut World, i32, i32, i32, i32) -> bool,
) {
    let var5 = world.get_block_metadata(x, y, z);
    let var6 = get_orientation(var5);
    let var7 = is_indirectly_powered_with(world, x, y, z, var6, power);
    // src: BlockPistonBase.java:70
    if var5 != 7 {
        if var7 && !is_powered(var5) {
            if can_extend(world, x, y, z, var6) {
                // src: BlockPistonBase.java:73-74
                world.set_block_metadata(x, y, z, var6 | 8);
                world.play_note_at(x, y, z, 0, var6);
            }
        } else if !var7 && is_powered(var5) {
            // src: BlockPistonBase.java:77-78
            world.set_block_metadata(x, y, z, var6);
            world.play_note_at(x, y, z, 1, var6);
        }
    }
}

/// `isIndirectlyPowered(world, x, y, z, facing)` (private): power into any
/// side but the front, then (the "quasi-connectivity" vanilla has) into the
/// block above and its sides, in Java's order; the first true wins.
// src: BlockPistonBase.java:84-110
fn is_indirectly_powered_with(
    world: &mut World,
    x: i32,
    y: i32,
    z: i32,
    var5: i32,
    power: &mut dyn FnMut(&mut World, i32, i32, i32, i32) -> bool,
) -> bool {
    (var5 != 0 && power(world, x, y - 1, z, 0))
        || (var5 != 1 && power(world, x, y + 1, z, 1))
        || (var5 != 2 && power(world, x, y, z - 1, 2))
        || (var5 != 3 && power(world, x, y, z + 1, 3))
        || (var5 != 5 && power(world, x + 1, y, z, 5))
        || (var5 != 4 && power(world, x - 1, y, z, 4))
        // src: BlockPistonBase.java:97-108 (no facing test)
        || power(world, x, y, z, 0)
        || power(world, x, y + 2, z, 1)
        || power(world, x, y + 1, z - 1, 2)
        || power(world, x, y + 1, z + 1, 3)
        || power(world, x - 1, y + 1, z, 4)
        || power(world, x + 1, y + 1, z, 5)
}

/// `playBlock(world, x, y, z, event, facing)`: the block event
/// `updatePistonState` sends through `World.playNoteAt`. Event 0 extends
/// (`tryExtend`, then the powered metadata with notification and the
/// `tile.piston.out` sound), event 1 retracts. `ignoreUpdates` is set for
/// the whole body and cleared around the removals that must reach this
/// piston's neighbours.
// src: BlockPistonBase.java:112-173
pub fn play_block(
    world: &mut World,
    id: i32,
    mut x: i32,
    mut y: i32,
    mut z: i32,
    var5: i32,
    var6: i32,
) {
    set_ignore_updates(world, id, true);
    if var5 == 0 {
        if try_extend(world, id, x, y, z, var6) {
            // src: BlockPistonBase.java:116-117
            world.set_block_metadata_with_notify(x, y, z, var6 | 8);
            let pitch = world.rand.next_float() * 0.25 + 0.6;
            world.play_sound_effect(
                x as f64 + 0.5,
                y as f64 + 0.5,
                z as f64 + 0.5,
                "tile.piston.out",
                0.5,
                pitch,
            );
        }
    } else if var5 == 1 {
        let (dx, dy, dz) = step(var6);
        // src: BlockPistonBase.java:120-123: a head still moving out is
        // finished first.
        if let Some(var8) =
            super::piston_moving::get_tile_entity_at_location(world, x + dx, y + dy, z + dz)
        {
            tile_entity::clear_piston_tile_entity(world, var8);
        }
        // src: BlockPistonBase.java:125-126: the piston itself retracts.
        world.set_block_and_metadata(x, y, z, super::piston_moving::PISTON_MOVING_ID, var6);
        let te = super::piston_moving::get_new_tile_entity(world, id, var6, var6, false, true);
        world.set_block_tile_entity(x, y, z, te);
        if is_sticky(id) {
            // src: BlockPistonBase.java:128-145: the block two cells ahead,
            // or what a moving block there carries when it is moving out in
            // this direction (finished at once).
            let var9 = x + dx * 2;
            let var10 = y + dy * 2;
            let var11 = z + dz * 2;
            let mut var12 = world.get_block_id(var9, var10, var11);
            let mut var13 = world.get_block_metadata(var9, var10, var11);
            let mut var14 = false;
            if var12 == super::piston_moving::PISTON_MOVING_ID
                && let Some(r) =
                    super::piston_moving::get_tile_entity_at_location(world, var9, var10, var11)
            {
                let p = tile_entity::piston(world, r)
                    .expect("checked instanceof")
                    .clone();
                if p.get_piston_orientation() == var6 && p.is_extending() {
                    tile_entity::clear_piston_tile_entity(world, r);
                    var12 = p.get_stored_block_id();
                    var13 = p.get_block_metadata();
                    var14 = true;
                }
            }

            // src: BlockPistonBase.java:147 (`&&` binds tighter than `||`)
            let not_pullable = var14
                || var12 <= 0
                || !can_push_block(var12, world, var9, var10, var11, false)
                || (block(var12).map_or(0, |b| b.get_mobility_flag()) != 0
                    && var12 != PISTON_BASE_ID
                    && var12 != PISTON_STICKY_BASE_ID);
            if not_pullable {
                // src: BlockPistonBase.java:148-152
                if !var14 {
                    set_ignore_updates(world, id, false);
                    world.set_block_with_notify(x + dx, y + dy, z + dz, 0);
                    set_ignore_updates(world, id, true);
                }
            } else {
                // src: BlockPistonBase.java:154-161
                set_ignore_updates(world, id, false);
                world.set_block_with_notify(var9, var10, var11, 0);
                set_ignore_updates(world, id, true);
                x += dx;
                y += dy;
                z += dz;
                world.set_block_and_metadata(
                    x,
                    y,
                    z,
                    super::piston_moving::PISTON_MOVING_ID,
                    var13,
                );
                let te = super::piston_moving::get_new_tile_entity(
                    world, var12, var13, var6, false, false,
                );
                world.set_block_tile_entity(x, y, z, te);
            }
        } else {
            // src: BlockPistonBase.java:164-166
            set_ignore_updates(world, id, false);
            world.set_block_with_notify(x + dx, y + dy, z + dz, 0);
            set_ignore_updates(world, id, true);
        }

        // src: BlockPistonBase.java:169: at the pulled block's cell when a
        // sticky piston pulled one (x, y, z moved above), else the piston's.
        let pitch = world.rand.next_float() * 0.15 + 0.6;
        world.play_sound_effect(
            x as f64 + 0.5,
            y as f64 + 0.5,
            z as f64 + 0.5,
            "tile.piston.in",
            0.5,
            pitch,
        );
    }
    set_ignore_updates(world, id, false);
}

/// `setBlockBoundsBasedOnState(world, x, y, z)`: an extended piston is the
/// 3/4 block behind its head; otherwise a full cube.
// src: BlockPistonBase.java:175-201
pub fn set_block_bounds_based_on_state(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    let var5 = world.get_block_metadata(x, y, z);
    let b = if is_powered(var5) {
        match get_orientation(var5) {
            0 => BlockBounds::from_floats(0.0, 0.25, 0.0, 1.0, 1.0, 1.0),
            1 => BlockBounds::from_floats(0.0, 0.0, 0.0, 1.0, 0.75, 1.0),
            2 => BlockBounds::from_floats(0.0, 0.0, 0.25, 1.0, 1.0, 1.0),
            3 => BlockBounds::from_floats(0.0, 0.0, 0.0, 1.0, 1.0, 0.75),
            4 => BlockBounds::from_floats(0.25, 0.0, 0.0, 1.0, 1.0, 1.0),
            5 => BlockBounds::from_floats(0.0, 0.0, 0.0, 0.75, 1.0, 1.0),
            // src: BlockPistonBase.java:178-196: no default case, bounds unchanged
            _ => return,
        }
    } else {
        BlockBounds::FULL
    };
    world.block_statics.bounds[(id & 255) as usize] = b;
}

/// `setBlockBoundsForItemRender()`: a full cube.
// src: BlockPistonBase.java:203-205
pub fn set_block_bounds_for_item_render(world: &mut World, id: i32) {
    world.block_statics.bounds[(id & 255) as usize] = BlockBounds::FULL;
}

/// `getCollidingBoundingBoxes(world, x, y, z, mask, list)`: the full cube,
/// whatever the state (the bounds are reset first), through the base body.
// src: BlockPistonBase.java:207-210
pub fn get_colliding_bounding_boxes(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    mask: &Aabb,
    list: &mut Vec<Aabb>,
) {
    world.block_statics.bounds[(id & 255) as usize] = BlockBounds::FULL;
    base_get_colliding_bounding_boxes(world, id, x, y, z, mask, list);
}

/// `Block.getCollidingBoundingBoxes` (the `super` call): the collision box
/// (`getCollisionBoundingBoxFromPool`, through the dispatch) when it
/// intersects `mask`. Private copy of the base body of
/// [`dispatch::get_colliding_bounding_boxes`] for the `super` calls of the
/// piston classes.
// src: Block.java:273-279
pub(super) fn base_get_colliding_bounding_boxes(
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

/// `determineOrientation(world, x, y, z, player)` (private static): up or
/// down when the player is within 2 blocks horizontally and well above or
/// below (eye height `posY + 1.82 - yOffset`), else the side facing the
/// player by yaw. Java casts the placer to `EntityPlayer`; only its position,
/// `yOffset` and yaw are read.
// src: BlockPistonBase.java:224-246
fn determine_orientation(world: &World, x: i32, y: i32, z: i32, var4: &Entity) -> i32 {
    let _ = world;
    let b = &var4.base;
    if math::abs(b.pos_x as f32 - x as f32) < 2.0 && math::abs(b.pos_z as f32 - z as f32) < 2.0 {
        let var5 = b.pos_y + 1.82 - b.y_offset as f64;
        if var5 - y as f64 > 2.0 {
            return 1;
        }
        if y as f64 - var5 > 0.0 {
            return 0;
        }
    }
    // src: BlockPistonBase.java:236
    let var7 = floor_double((b.rotation_yaw * 4.0 / 360.0) as f64 + 0.5) & 3;
    match var7 {
        0 => 2,
        1 => 5,
        2 => 3,
        3 => 4,
        // src: BlockPistonBase.java:244 (unreachable after `& 3`)
        _ => 0,
    }
}

/// `canPushBlock(id, world, x, y, z, allowDestroy)` (private static): never
/// obsidian; a piston base only when retracted; any other block not when
/// unbreakable (hardness -1: bedrock, the moving block), immovable
/// (mobility 2), or destroyed-by-push (mobility 1) unless `allowDestroy`;
/// and in every case not when the cell has a tile entity.
// src: BlockPistonBase.java:248-271
fn can_push_block(var0: i32, world: &mut World, x: i32, y: i32, z: i32, var5: bool) -> bool {
    if var0 == OBSIDIAN_ID {
        return false;
    }
    if var0 != PISTON_BASE_ID && var0 != PISTON_STICKY_BASE_ID {
        // Java: Block.blocksList[var0] (a NullPointerException for an id with
        // no block, which the callers never pass: they read ids from the world).
        let Some(b) = block(var0) else { return false };
        if b.get_hardness() == -1.0 {
            return false;
        }
        if b.get_mobility_flag() == 2 {
            return false;
        }
        if !var5 && b.get_mobility_flag() == 1 {
            return false;
        }
    } else if is_powered(world.get_block_metadata(x, y, z)) {
        return false;
    }
    // src: BlockPistonBase.java:268-269
    world.get_block_tile_entity(x, y, z).is_none()
}

/// `canExtend(world, x, y, z, facing)` (private static): walks the push line
/// from the cell in front: true at the first air or destroy-by-push block
/// within 13 cells, false at an unpushable block, the world's y limits
/// (`y <= 0` or `y >= 127`), or a 13th pushable block.
// src: BlockPistonBase.java:273-307
fn can_extend(world: &mut World, x: i32, y: i32, z: i32, var4: i32) -> bool {
    let (dx, dy, dz) = step(var4);
    let mut var5 = x + dx;
    let mut var6 = y + dy;
    let mut var7 = z + dz;
    let mut var8 = 0;
    while var8 < 13 {
        if var6 <= 0 || var6 >= 127 {
            return false;
        }
        let var9 = world.get_block_id(var5, var6, var7);
        if var9 == 0 {
            break;
        }
        if !can_push_block(var9, world, var5, var6, var7, true) {
            return false;
        }
        if block(var9).map_or(0, |b| b.get_mobility_flag()) == 1 {
            break;
        }
        if var8 == 12 {
            return false;
        }
        var5 += dx;
        var6 += dy;
        var7 += dz;
        var8 += 1;
    }
    true
}

/// `tryExtend(world, x, y, z, facing)` (private): `canExtend`'s walk again,
/// this time dropping (as items) and removing a destroy-by-push block at the
/// end of the line; then, from the far end back to the piston, each cell
/// becomes a moving block carrying the block behind it, the cell in front of
/// the piston carrying the head (`facing | 8` for a sticky piston).
// src: BlockPistonBase.java:309-365
fn try_extend(world: &mut World, id: i32, x: i32, y: i32, z: i32, var5: i32) -> bool {
    let (dx, dy, dz) = step(var5);
    let mut var6 = x + dx;
    let mut var7 = y + dy;
    let mut var8 = z + dz;
    let mut var9 = 0;
    // src: BlockPistonBase.java:316-343
    while var9 < 13 {
        if !(var7 > 0 && var7 < 127) {
            return false;
        }
        let var10 = world.get_block_id(var6, var7, var8);
        if var10 != 0 {
            if !can_push_block(var10, world, var6, var7, var8, true) {
                return false;
            }
            if block(var10).map_or(0, |b| b.get_mobility_flag()) != 1 {
                if var9 == 12 {
                    return false;
                }
                var6 += dx;
                var7 += dy;
                var8 += dz;
                var9 += 1;
                continue;
            }
            // src: BlockPistonBase.java:336-337
            let meta = world.get_block_metadata(var6, var7, var8);
            dispatch::drop_block_as_item(world, var10, var6, var7, var8, meta);
            world.set_block_with_notify(var6, var7, var8, 0);
        }
        break;
    }

    // src: BlockPistonBase.java:345-362
    let head_meta = var5 | if is_sticky(id) { 8 } else { 0 };
    while var6 != x || var7 != y || var8 != z {
        let var9 = var6 - dx;
        let var10 = var7 - dy;
        let var11 = var8 - dz;
        let var12 = world.get_block_id(var9, var10, var11);
        let var13 = world.get_block_metadata(var9, var10, var11);
        if var12 == id && var9 == x && var10 == y && var11 == z {
            world.set_block_and_metadata(
                var6,
                var7,
                var8,
                super::piston_moving::PISTON_MOVING_ID,
                head_meta,
            );
            let te = super::piston_moving::get_new_tile_entity(
                world,
                PISTON_EXTENSION_ID,
                head_meta,
                var5,
                true,
                false,
            );
            world.set_block_tile_entity(var6, var7, var8, te);
        } else {
            world.set_block_and_metadata(
                var6,
                var7,
                var8,
                super::piston_moving::PISTON_MOVING_ID,
                var13,
            );
            let te =
                super::piston_moving::get_new_tile_entity(world, var12, var13, var5, true, false);
            world.set_block_tile_entity(var6, var7, var8, te);
        }
        var6 = var9;
        var7 = var10;
        var8 = var11;
    }
    true
}

#[cfg(test)]
mod tests {
    //! Pistons without redstone: no power source is transcribed in this
    //! package (the redstone task of WP-1.6), so the extension tests drive
    //! [`update_piston_state_with`] with a power test that says "powered"
    //! (test-only; the game path always asks the world). Retraction and the
    //! moving block need no power.

    use super::*;
    use crate::block::piston_moving::{self, PISTON_MOVING_ID};
    use crate::item::ItemStack;
    use crate::save::NoSave;
    use crate::world::WorldEnvironment;
    use crate::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
    use crate::world::events::WorldEvent;
    use crate::world::provider::{Dimension, WorldProvider};
    use crate::world::tile_entity::{TileEntityPiston, TileEntityRef, piston};

    const STONE: i32 = 1;
    const BEDROCK: i32 = 7;
    const WOOL: i32 = 35;
    const PUMPKIN: i32 = 86;

    /// An empty world (the oracle's `"world": "empty"`) with a stone floor
    /// at y = 60 over -4..=20 x -4..=4.
    fn world() -> World {
        let mut w = World::new(
            Box::new(NoSave),
            "harness",
            WorldProvider::new(Dimension::Surface),
            123,
            ChunkGenerator::Empty(EmptyChunkProvider),
            WorldEnvironment {
                rand_seed: 0,
                field_9437_g: 0,
                lock_timestamp: 0,
            },
        );
        w.make_deterministic(123);
        for x in -4..=20 {
            for z in -4..=4 {
                w.set_block_and_metadata(x, 60, z, STONE, 0);
            }
        }
        w
    }

    fn id_meta(w: &mut World, x: i32, y: i32, z: i32) -> (i32, i32) {
        (w.get_block_id(x, y, z), w.get_block_metadata(x, y, z))
    }

    fn te_at(w: &mut World, x: i32, y: i32, z: i32) -> Option<TileEntityPiston> {
        let r = w.get_block_tile_entity(x, y, z)?;
        piston(w, r).cloned()
    }

    fn powered(_: &mut World, _: i32, _: i32, _: i32, _: i32) -> bool {
        true
    }

    #[test]
    fn indirectly_powered_asks_in_java_order_and_skips_the_front() {
        let mut w = world();
        for facing in 0..6 {
            let mut asked = Vec::new();
            let mut q = |_: &mut World, x: i32, y: i32, z: i32, s: i32| {
                asked.push((x, y, z, s));
                false
            };
            assert!(!is_indirectly_powered_with(
                &mut w, 10, 64, 20, facing, &mut q
            ));
            // src: BlockPistonBase.java:85-108
            let mut expected = vec![
                (0, (10, 63, 20, 0)),
                (1, (10, 65, 20, 1)),
                (2, (10, 64, 19, 2)),
                (3, (10, 64, 21, 3)),
                (5, (11, 64, 20, 5)),
                (4, (9, 64, 20, 4)),
            ]
            .into_iter()
            .filter(|(f, _)| *f != facing)
            .map(|(_, q)| q)
            .collect::<Vec<_>>();
            expected.extend([
                (10, 64, 20, 0),
                (10, 66, 20, 1),
                (10, 65, 19, 2),
                (10, 65, 21, 3),
                (9, 65, 20, 4),
                (11, 65, 20, 5),
            ]);
            assert_eq!(asked, expected, "facing {facing}");
        }
        // The first true stops the chain.
        let mut n = 0;
        let mut q = |_: &mut World, _: i32, _: i32, _: i32, _: i32| {
            n += 1;
            n == 3
        };
        assert!(is_indirectly_powered_with(&mut w, 0, 64, 0, 1, &mut q));
        assert_eq!(n, 3);
        // Without redstone the world powers nothing.
        assert!(!is_indirectly_powered_with(
            &mut w,
            0,
            61,
            0,
            5,
            &mut world_power
        ));
    }

    #[test]
    fn can_extend_counts_twelve_blocks_and_stops_at_immovables() {
        let mut w = world();
        let row = |w: &mut World, n: i32, last: i32| {
            for x in 1..=20 {
                w.set_block_and_metadata(x, 61, 0, 0, 0);
            }
            for x in 1..=n {
                w.set_block_and_metadata(x, 61, 0, STONE, 0);
            }
            if last != 0 {
                w.set_block_and_metadata(n + 1, 61, 0, last, 0);
            }
        };
        w.set_block_and_metadata(0, 61, 0, PISTON_BASE_ID, 5);
        row(&mut w, 0, 0);
        assert!(can_extend(&mut w, 0, 61, 0, 5), "nothing in front");
        row(&mut w, 12, 0);
        assert!(can_extend(&mut w, 0, 61, 0, 5), "12 blocks");
        row(&mut w, 13, 0);
        assert!(!can_extend(&mut w, 0, 61, 0, 5), "13 blocks");
        // A destroy-by-push block (mobility 1) ends the line, even 13th.
        row(&mut w, 12, PUMPKIN);
        assert!(can_extend(&mut w, 0, 61, 0, 5), "12 blocks and a pumpkin");
        for imm in [OBSIDIAN_ID, BEDROCK, PISTON_EXTENSION_ID, PISTON_MOVING_ID] {
            row(&mut w, 0, imm);
            assert!(!can_extend(&mut w, 0, 61, 0, 5), "id {imm} in front");
            row(&mut w, 3, imm);
            assert!(!can_extend(&mut w, 0, 61, 0, 5), "id {imm} behind 3");
        }
        // A retracted piston is pushable, an extended one is not.
        row(&mut w, 0, PISTON_STICKY_BASE_ID);
        assert!(can_extend(&mut w, 0, 61, 0, 5));
        w.set_block_metadata(1, 61, 0, 8 | 5);
        assert!(!can_extend(&mut w, 0, 61, 0, 5));
        // The world's limits: the line may not reach y <= 0 or y >= 127.
        assert!(!can_extend(&mut w, 0, 126, 0, 1));
        assert!(can_extend(&mut w, 0, 125, 0, 1));
        assert!(!can_extend(&mut w, 0, 1, 0, 0));
        assert!(can_extend(&mut w, 0, 2, 0, 0));
    }

    #[test]
    fn extension_moves_blocks_and_metadata_then_the_unpowered_piston_retracts() {
        let mut w = world();
        // src: BlockPistonBase.java:43-50 is not used: placed facing +x (5).
        w.set_block_and_metadata(0, 61, 0, PISTON_BASE_ID, 5);
        w.set_block_and_metadata(1, 61, 0, STONE, 0);
        w.set_block_and_metadata(2, 61, 0, WOOL, 14);
        w.events.drain();
        let rand0 = w.rand.clone();
        update_piston_state_with(&mut w, 0, 61, 0, &mut powered);

        assert_eq!(id_meta(&mut w, 0, 61, 0), (PISTON_BASE_ID, 13));
        // Far end first; the head is the plain piston's (no bit 8).
        let head = TileEntityPiston::new(PISTON_EXTENSION_ID, 5, 5, true, false);
        let stone = TileEntityPiston::new(STONE, 0, 5, true, false);
        let wool = TileEntityPiston::new(WOOL, 14, 5, true, false);
        assert_eq!(id_meta(&mut w, 1, 61, 0), (PISTON_MOVING_ID, 5));
        assert_eq!(id_meta(&mut w, 2, 61, 0), (PISTON_MOVING_ID, 0));
        assert_eq!(id_meta(&mut w, 3, 61, 0), (PISTON_MOVING_ID, 14));
        assert_eq!(te_at(&mut w, 1, 61, 0), Some(head));
        assert_eq!(te_at(&mut w, 2, 61, 0), Some(stone));
        assert_eq!(te_at(&mut w, 3, 61, 0), Some(wool));
        let order: Vec<(i32, i32, i32)> = w
            .loaded_tile_entity_list
            .iter()
            .map(|r| {
                let t = &w.tile_entities[r];
                (t.x_coord, t.y_coord, t.z_coord)
            })
            .collect();
        assert_eq!(order, [(3, 61, 0), (2, 61, 0), (1, 61, 0)]);
        // One draw for the pitch of tile.piston.out.
        let mut r = rand0;
        let pitch = r.next_float() * 0.25 + 0.6;
        assert_eq!(w.rand.state(), r.state());
        assert!(w.events.drain().into_iter().any(|e| matches!(
            e,
            WorldEvent::PlaySound { ref name, x, volume, pitch: p, .. }
                if name == "tile.piston.out" && x == 0.5 && volume == 0.5 && p == pitch
        )));

        // The moving stone collides one cell back at progress 0.
        let bb = crate::block::dispatch::get_collision_bounding_box_from_pool(
            &mut w,
            PISTON_MOVING_ID,
            2,
            61,
            0,
        )
        .unwrap();
        assert_eq!(
            (bb.min_x, bb.max_x, bb.min_y, bb.max_y),
            (1.0, 2.0, 61.0, 62.0)
        );

        // Progress 0 -> 0.5 -> 1.0, then the blocks land.
        w.update_entities();
        let t = te_at(&mut w, 2, 61, 0).unwrap();
        assert_eq!((t.last_progress, t.progress), (0.0, 0.5));
        assert_eq!(t.get_progress(0.5), 0.25);
        assert_eq!(t.get_progress(2.0), 0.5);
        assert_eq!(t.get_offset_x(0.0), -1.0);
        assert_eq!(t.get_offset_x(1.0), -0.5);
        assert_eq!(t.get_offset_y(1.0), 0.0);
        w.update_entities();
        let t = te_at(&mut w, 2, 61, 0).unwrap();
        assert_eq!((t.last_progress, t.progress), (0.5, 1.0));
        w.update_entities();
        assert_eq!(id_meta(&mut w, 3, 61, 0), (WOOL, 14));
        assert_eq!(id_meta(&mut w, 2, 61, 0), (STONE, 0));
        // The head landed and notified the piston, which nothing powers in
        // this world: it retracted at once (the head cell emptied, the piston
        // is a moving block carrying itself back, facing kept).
        assert_eq!(id_meta(&mut w, 1, 61, 0), (0, 0));
        assert_eq!(id_meta(&mut w, 0, 61, 0), (PISTON_MOVING_ID, 5));
        // Created during the tile-entity pass: pending, so not updated in
        // that pass, and filed in the chunk at its end (World.java:1254-1274).
        assert_eq!(
            te_at(&mut w, 0, 61, 0),
            Some(TileEntityPiston::new(PISTON_BASE_ID, 5, 5, false, true))
        );
        w.update_entities();
        assert_eq!(
            te_at(&mut w, 0, 61, 0),
            Some(TileEntityPiston {
                progress: 0.5,
                ..TileEntityPiston::new(PISTON_BASE_ID, 5, 5, false, true)
            })
        );
        w.update_entities();
        w.update_entities();
        assert_eq!(id_meta(&mut w, 0, 61, 0), (PISTON_BASE_ID, 5));
        assert!(w.loaded_tile_entity_list.is_empty());
    }

    #[test]
    fn extension_drops_a_destroy_by_push_block_and_refuses_thirteen() {
        let mut w = world();
        w.set_block_and_metadata(0, 61, 0, PISTON_STICKY_BASE_ID, 5);
        w.set_block_and_metadata(1, 61, 0, STONE, 0);
        w.set_block_and_metadata(2, 61, 0, PUMPKIN, 0);
        let entities = w.loaded_entity_list.len();
        update_piston_state_with(&mut w, 0, 61, 0, &mut powered);
        assert_eq!(id_meta(&mut w, 0, 61, 0), (PISTON_STICKY_BASE_ID, 13));
        // The pumpkin dropped as an item; the stone moves into its cell; the
        // sticky head carries bit 8.
        assert_eq!(w.loaded_entity_list.len(), entities + 1);
        assert_eq!(id_meta(&mut w, 2, 61, 0), (PISTON_MOVING_ID, 0));
        assert_eq!(id_meta(&mut w, 1, 61, 0), (PISTON_MOVING_ID, 13));
        assert_eq!(
            te_at(&mut w, 1, 61, 0),
            Some(TileEntityPiston::new(
                PISTON_EXTENSION_ID,
                13,
                5,
                true,
                false
            ))
        );

        let mut w = world();
        w.set_block_and_metadata(0, 61, 0, PISTON_BASE_ID, 5);
        for x in 1..=13 {
            w.set_block_and_metadata(x, 61, 0, STONE, 0);
        }
        let rand0 = w.rand.state();
        update_piston_state_with(&mut w, 0, 61, 0, &mut powered);
        assert_eq!(id_meta(&mut w, 0, 61, 0), (PISTON_BASE_ID, 5));
        assert_eq!(w.rand.state(), rand0);
        w.set_block_and_metadata(13, 61, 0, 0, 0);
        update_piston_state_with(&mut w, 0, 61, 0, &mut powered);
        assert_eq!(id_meta(&mut w, 0, 61, 0), (PISTON_BASE_ID, 13));
        assert_eq!(id_meta(&mut w, 13, 61, 0), (PISTON_MOVING_ID, 0));
    }

    #[test]
    fn sticky_retraction_pulls_a_block_and_leaves_an_immovable_one() {
        // The extended sticky piston is placed last: its onBlockAdded runs
        // updatePistonState, and without power it retracts at once.
        let mut w = world();
        w.set_block_and_metadata(1, 61, 0, PISTON_EXTENSION_ID, 13);
        w.set_block_and_metadata(2, 61, 0, WOOL, 3);
        let rand0 = w.rand.clone();
        w.set_block_and_metadata(0, 61, 0, PISTON_STICKY_BASE_ID, 13);
        assert_eq!(id_meta(&mut w, 0, 61, 0), (PISTON_MOVING_ID, 5));
        assert_eq!(
            te_at(&mut w, 0, 61, 0),
            Some(TileEntityPiston::new(
                PISTON_STICKY_BASE_ID,
                5,
                5,
                false,
                true
            ))
        );
        assert_eq!(id_meta(&mut w, 1, 61, 0), (PISTON_MOVING_ID, 3));
        assert_eq!(
            te_at(&mut w, 1, 61, 0),
            Some(TileEntityPiston::new(WOOL, 3, 5, false, false))
        );
        assert_eq!(id_meta(&mut w, 2, 61, 0), (0, 0));
        let mut r = rand0;
        r.next_float();
        assert_eq!(w.rand.state(), r.state());
        // Retracting moving blocks collide where they are going.
        let bb = piston_moving::get_collision_bounding_box_from_pool(&mut w, 1, 61, 0).unwrap();
        assert_eq!((bb.min_x, bb.max_x), (1.0, 2.0));
        for _ in 0..3 {
            w.update_entities();
        }
        assert_eq!(id_meta(&mut w, 0, 61, 0), (PISTON_STICKY_BASE_ID, 5));
        assert_eq!(id_meta(&mut w, 1, 61, 0), (WOOL, 3));
        assert!(!w.block_statics.piston_sticky_base_ignore_updates);

        let mut w = world();
        w.set_block_and_metadata(1, 61, 0, PISTON_EXTENSION_ID, 13);
        w.set_block_and_metadata(2, 61, 0, OBSIDIAN_ID, 0);
        w.set_block_and_metadata(0, 61, 0, PISTON_STICKY_BASE_ID, 13);
        assert_eq!(id_meta(&mut w, 1, 61, 0), (0, 0));
        assert_eq!(id_meta(&mut w, 2, 61, 0), (OBSIDIAN_ID, 0));
    }

    #[test]
    fn retraction_finishes_a_moving_head_first() {
        // A head still moving out (the tile entity in front) is placed at
        // once by clearPistonTileEntity, then removed by the retraction.
        let mut w = world();
        w.set_block_and_metadata(0, 61, 0, PISTON_BASE_ID, 5);
        update_piston_state_with(&mut w, 0, 61, 0, &mut powered);
        assert_eq!(id_meta(&mut w, 1, 61, 0), (PISTON_MOVING_ID, 5));
        let head: TileEntityRef = w.get_block_tile_entity(1, 61, 0).unwrap();
        // Unpowered now: retract (BlockPistonBase.java:76-79).
        update_piston_state(&mut w, 0, 61, 0);
        assert!(w.tile_entities[&head].is_invalid());
        assert_eq!(piston(&w, head).unwrap().last_progress, 1.0);
        assert_eq!(id_meta(&mut w, 1, 61, 0), (0, 0));
        assert_eq!(id_meta(&mut w, 0, 61, 0), (PISTON_MOVING_ID, 5));
    }

    #[test]
    fn moving_block_without_tile_entity_and_orphan_head() {
        let mut w = world();
        let mut p = crate::entity::player::new_entity_player_sp(&mut w);
        w.set_block_and_metadata_with_notify(0, 61, 0, PISTON_MOVING_ID, 0);
        assert_eq!(w.get_block_tile_entity(0, 61, 0), None);
        assert!(crate::block::dispatch::block_activated(
            &mut w,
            PISTON_MOVING_ID,
            0,
            61,
            0,
            &mut p
        ));
        assert_eq!(id_meta(&mut w, 0, 61, 0), (0, 0));
        assert!(!crate::block::dispatch::block_activated(
            &mut w,
            PISTON_BASE_ID,
            0,
            61,
            0,
            &mut p
        ));

        // A head with no piston behind it goes at the next neighbour change.
        w.set_block_and_metadata_with_notify(3, 61, 0, PISTON_EXTENSION_ID, 1);
        assert_eq!(id_meta(&mut w, 3, 61, 0), (PISTON_EXTENSION_ID, 1));
        w.set_block_and_metadata_with_notify(4, 61, 0, STONE, 0);
        assert_eq!(id_meta(&mut w, 3, 61, 0), (0, 0));
    }

    #[test]
    fn moving_block_drops_what_it_carries() {
        let mut w = world();
        w.set_block_and_metadata(0, 61, 0, PISTON_BASE_ID, 5);
        w.set_block_and_metadata(1, 61, 0, WOOL, 14);
        update_piston_state_with(&mut w, 0, 61, 0, &mut powered);
        let n = w.loaded_entity_list.len();
        crate::block::dispatch::drop_block_as_item_with_chance(
            &mut w,
            PISTON_MOVING_ID,
            2,
            61,
            0,
            0,
            0.0,
        );
        assert_eq!(w.loaded_entity_list.len(), n + 1);
        let e = w.loaded_entity_list[n].entity.as_deref().unwrap();
        let stack = e.kind.item().unwrap().item.unwrap();
        assert_eq!(stack, ItemStack::new(WOOL, 1, 14));
    }

    #[test]
    fn bounds_follow_state() {
        let mut w = world();
        w.set_block_and_metadata(0, 61, 0, PISTON_BASE_ID, 5);
        w.set_block_metadata(0, 61, 0, 13);
        set_block_bounds_based_on_state(&mut w, PISTON_BASE_ID, 0, 61, 0);
        assert_eq!(
            w.block_statics.bounds[PISTON_BASE_ID as usize],
            BlockBounds::from_floats(0.0, 0.0, 0.0, 0.75, 1.0, 1.0)
        );
        let mut list = Vec::new();
        let mask = Aabb::new(-1.0, 60.0, -1.0, 2.0, 63.0, 2.0);
        get_colliding_bounding_boxes(&mut w, PISTON_BASE_ID, 0, 61, 0, &mask, &mut list);
        assert_eq!(list.len(), 1);
        assert_eq!((list[0].min_x, list[0].max_x), (0.0, 1.0));
        set_block_bounds_for_item_render(&mut w, PISTON_BASE_ID);
        assert_eq!(
            w.block_statics.bounds[PISTON_BASE_ID as usize],
            BlockBounds::FULL
        );

        // The head: plate and arm, then a full cube again.
        w.set_block_and_metadata(1, 61, 0, PISTON_EXTENSION_ID, 5);
        let mut list = Vec::new();
        crate::block::dispatch::get_colliding_bounding_boxes(
            &mut w,
            PISTON_EXTENSION_ID,
            1,
            61,
            0,
            &mask,
            &mut list,
        );
        assert_eq!(list.len(), 2);
        assert_eq!((list[0].min_x, list[0].max_x), (1.75, 2.0));
        assert_eq!(
            (list[1].min_x, list[1].min_y, list[1].max_y),
            (1.0, 61.375, 61.625)
        );
        assert_eq!(
            w.block_statics.bounds[PISTON_EXTENSION_ID as usize],
            BlockBounds::FULL
        );
    }

    #[test]
    fn orientation_from_the_placer() {
        let mut w = world();
        let mut p = crate::entity::player::new_entity_player_sp(&mut w);
        let mut at = |x: f64, y: f64, z: f64, yaw: f32| {
            p.base.pos_x = x;
            p.base.pos_y = y;
            p.base.pos_z = z;
            p.base.rotation_yaw = yaw;
            determine_orientation(&w, 0, 61, 0, &p)
        };
        // Far away: by yaw (south 0 -> 2, west 90 -> 5, north 180 -> 3, east 270 -> 4).
        assert_eq!(at(10.0, 61.0, 10.0, 0.0), 2);
        assert_eq!(at(10.0, 61.0, 10.0, 90.0), 5);
        assert_eq!(at(10.0, 61.0, 10.0, 180.0), 3);
        assert_eq!(at(10.0, 61.0, 10.0, 270.0), 4);
        assert_eq!(at(10.0, 61.0, 10.0, -45.0), 2);
        assert_eq!(at(10.0, 61.0, 10.0, 44.0), 2);
        assert_eq!(at(10.0, 61.0, 10.0, 46.0), 5);
        // Close: eyes (posY + 1.82 - yOffset) more than 2 above -> up; below -> down.
        let off = p_y_offset();
        assert_eq!(at(0.5, 62.0 + off, 0.5, 0.0), 1);
        assert_eq!(at(0.5, 58.0 + off, 0.5, 0.0), 0);
        assert_eq!(at(0.5, 61.0 + off, 0.5, 0.0), 2);
    }

    /// The player's `yOffset` (1.62, EntityPlayerSP), as f64.
    fn p_y_offset() -> f64 {
        let mut w = world();
        crate::entity::player::new_entity_player_sp(&mut w)
            .base
            .y_offset as f64
    }

    #[test]
    fn tile_entity_piston_nbt_round_trip() {
        let mut t = TileEntityPiston::new(STONE, 3, 4, true, false);
        t.last_progress = 0.5;
        t.progress = 1.0;
        let mut c = std::collections::BTreeMap::new();
        t.write_to_nbt(&mut c);
        assert_eq!(c["progress"], mc_nbt::Tag::Float(0.5));
        assert_eq!(c["extending"], mc_nbt::Tag::Byte(1));
        let mut u = TileEntityPiston::new(0, 0, 0, false, false);
        u.read_from_nbt(&c);
        assert_eq!(
            u,
            TileEntityPiston {
                progress: 0.5,
                last_progress: 0.5,
                ..TileEntityPiston::new(STONE, 3, 4, true, false)
            }
        );
    }
}
