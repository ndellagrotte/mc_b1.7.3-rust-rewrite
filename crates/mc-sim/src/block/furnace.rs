//! `BlockFurnace` (BlockFurnace.java): the idle (61) and lit (62) furnace.
//!
//! The two ids are two singletons with their own `furnaceRand`
//! ([`super::BlockStatics::furnace_rand_mut`]). The lit/idle swap,
//! `updateFurnaceBlockState`, keeps the tile entity across the block change
//! with the static `keepFurnaceInventory`
//! ([`super::BlockStatics::keep_furnace_inventory`]).

use mc_core::java_random::JavaRandom;
use mc_core::math::floor_double;

use super::{block, container, opaque_cube_lookup};
use crate::entity::Entity;
use crate::world::World;

/// `Block.stoneOvenIdle.blockID` / `Block.stoneOvenActive.blockID`.
// src: Block.java:653-654
const STONE_OVEN_IDLE: i32 = 61;
const STONE_OVEN_ACTIVE: i32 = 62;

/// `isActive` of the singleton `id` (62 is the lit one).
fn is_active(id: i32) -> bool {
    matches!(
        block(id).map(|b| b.kind),
        Some(super::BlockKind::Furnace { is_active: true })
    )
}

/// `blockIndexInTexture` (45, BlockFurnace.java:13).
fn tex(id: i32) -> i32 {
    block(id).map_or(45, |b| b.block_index_in_texture)
}

/// `onBlockAdded(world, x, y, z)`: `BlockContainer.onBlockAdded` (the tile
/// entity), then `setDefaultDirection`.
// src: BlockFurnace.java:20-23
pub fn on_block_added(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    container::on_block_added(world, id, x, y, z);
    set_default_direction(world, x, y, z);
}

/// `setDefaultDirection(world, x, y, z)` (private): faces away from an
/// opaque neighbour (metadata 2-5 = the side the front looks at; 3 when
/// no single side is blocked, x neighbours deciding over z ones), with
/// notification.
// src: BlockFurnace.java:25-50
fn set_default_direction(world: &mut World, x: i32, y: i32, z: i32) {
    if !world.multiplayer_world {
        let var5 = world.get_block_id(x, y, z - 1);
        let var6 = world.get_block_id(x, y, z + 1);
        let var7 = world.get_block_id(x - 1, y, z);
        let var8 = world.get_block_id(x + 1, y, z);
        let o = opaque_cube_lookup;
        let mut var9 = 3;
        if o(var5) && !o(var6) {
            var9 = 3;
        }
        if o(var6) && !o(var5) {
            var9 = 2;
        }
        if o(var7) && !o(var8) {
            var9 = 5;
        }
        if o(var8) && !o(var7) {
            var9 = 4;
        }
        world.set_block_metadata_with_notify(x, y, z, var9);
    }
}

/// `getBlockTexture(blockAccess, x, y, z, side)`: top and bottom 62, the
/// front (the side equal to the metadata) 44 idle / 61 lit, else 45.
// src: BlockFurnace.java:52-65
pub fn get_block_texture(world: &mut World, id: i32, x: i32, y: i32, z: i32, side: i32) -> i32 {
    let bi = tex(id);
    if side == 1 || side == 0 {
        bi + 17
    } else {
        let var6 = world.get_block_metadata(x, y, z);
        if side != var6 {
            bi
        } else if is_active(id) {
            bi + 16
        } else {
            bi - 1
        }
    }
}

/// `randomDisplayTick(world, x, y, z, rand)`: a lit furnace puts smoke and
/// flame in front of its face (client particles; `rand` is
/// `randomDisplayUpdates`' local `Random`).
// src: BlockFurnace.java:67-90
pub fn random_display_tick(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    rand: &mut JavaRandom,
) {
    if is_active(id) {
        let var6 = world.get_block_metadata(x, y, z);
        let var7 = x as f32 + 0.5;
        let var8 = y as f32 + 0.0 + rand.next_float() * 6.0 / 16.0;
        let var9 = z as f32 + 0.5;
        let var10 = 0.52f32;
        let var11 = rand.next_float() * 0.6 - 0.3;
        let at = match var6 {
            4 => Some((var7 - var10, var9 + var11)),
            5 => Some((var7 + var10, var9 + var11)),
            2 => Some((var7 + var11, var9 - var10)),
            3 => Some((var7 + var11, var9 + var10)),
            _ => None,
        };
        if let Some((px, pz)) = at {
            world.spawn_particle("smoke", px as f64, var8 as f64, pz as f64, 0.0, 0.0, 0.0);
            world.spawn_particle("flame", px as f64, var8 as f64, pz as f64, 0.0, 0.0, 0.0);
        }
    }
}

/// `blockActivated(world, x, y, z, player)`: true; off a multiplayer client
/// `player.displayGUIFurnace(tileEntity)` (client GUI and `ContainerFurnace`,
/// WP-2.5/Phase 3; recorded as `"EntityPlayerSP.displayGUIFurnace"`, after
/// the `getBlockTileEntity` lookup, which can create the tile entity).
// src: BlockFurnace.java:102-110
pub fn block_activated(world: &mut World, x: i32, y: i32, z: i32, player: &mut Entity) -> bool {
    let _ = player;
    if !world.multiplayer_world {
        let _var6 = world.get_block_tile_entity(x, y, z);
        world.note_unimplemented("EntityPlayerSP.displayGUIFurnace");
    }
    true
}

/// `updateFurnaceBlockState(burning, world, x, y, z)` (public static): swaps
/// the block between idle and lit, keeping the metadata and the tile entity
/// (`keepFurnaceInventory` stops the removal from scattering the items; the
/// old tile entity is validated and set again after the new block's
/// `onBlockAdded` made a fresh one, which stays in `loadedTileEntityList`
/// without a chunk slot: vanilla, transcribed).
// src: BlockFurnace.java:112-126
pub fn update_furnace_block_state(var0: bool, world: &mut World, x: i32, y: i32, z: i32) {
    let var5 = world.get_block_metadata(x, y, z);
    let var6 = world.get_block_tile_entity(x, y, z);
    world.block_statics.keep_furnace_inventory = true;
    if var0 {
        world.set_block_with_notify(x, y, z, STONE_OVEN_ACTIVE);
    } else {
        world.set_block_with_notify(x, y, z, STONE_OVEN_IDLE);
    }
    world.block_statics.keep_furnace_inventory = false;
    world.set_block_metadata_with_notify(x, y, z, var5);
    // `var6.validate()`: a missing tile entity would throw here in Java.
    if let Some(r) = var6 {
        if let Some(t) = world.tile_entities.get_mut(&r) {
            t.validate();
        }
        world.set_block_tile_entity(x, y, z, r);
    }
}

/// `onBlockPlacedBy(world, x, y, z, placer)`: the front faces the placer
/// (`floor(yaw * 4 / 360 + 0.5) & 3` → metadata 2, 5, 3, 4).
// src: BlockFurnace.java:132-150
pub fn on_block_placed_by(world: &mut World, x: i32, y: i32, z: i32, placer: &Entity) {
    let var6 = floor_double((placer.base.rotation_yaw * 4.0 / 360.0) as f64 + 0.5) & 3;
    if var6 == 0 {
        world.set_block_metadata_with_notify(x, y, z, 2);
    }
    if var6 == 1 {
        world.set_block_metadata_with_notify(x, y, z, 5);
    }
    if var6 == 2 {
        world.set_block_metadata_with_notify(x, y, z, 3);
    }
    if var6 == 3 {
        world.set_block_metadata_with_notify(x, y, z, 4);
    }
}

/// `onBlockRemoval(world, x, y, z)`: unless `keepFurnaceInventory`, scatters
/// the items with this singleton's `furnaceRand`; then
/// `BlockContainer.onBlockRemoval`.
// src: BlockFurnace.java:152-182
pub fn on_block_removal(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    if !world.block_statics.keep_furnace_inventory {
        let rand: fn(&mut World) -> &mut JavaRandom = if id == STONE_OVEN_ACTIVE {
            |w| &mut w.block_statics.stone_oven_active_furnace_rand
        } else {
            |w| &mut w.block_statics.stone_oven_idle_furnace_rand
        };
        container::scatter_inventory(world, x, y, z, rand);
    }
    container::on_block_removal(world, x, y, z);
}
