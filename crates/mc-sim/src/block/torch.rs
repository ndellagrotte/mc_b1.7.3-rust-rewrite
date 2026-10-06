//! `BlockTorch` (BlockTorch.java): the wooden torch (id 50) and the base of
//! `BlockRedstoneTorch` (ids 75, 76, [`super::redstone_torch`]), which
//! inherits `canPlaceBlockAt`, `onBlockPlaced`, `collisionRayTrace` and the
//! (null) collision box, and calls `onBlockAdded` / `onNeighborBlockChange`
//! here through `super`.
//!
//! Fully transcribed: `getCollisionBoundingBoxFromPool`, `canPlaceTorchOn`,
//! `canPlaceBlockAt`, `onBlockPlaced`, `updateTick`, `onBlockAdded`,
//! `onNeighborBlockChange`, `dropTorchIfCantStay`, `collisionRayTrace` (its
//! bounds; the dispatch then runs `Block.collisionRayTrace`) and
//! `randomDisplayTick` (particle events only). The data methods
//! (`isOpaqueCube`, `renderAsNormalBlock`, `getRenderType`) are in
//! [`crate::block`].
//!
//! Torch metadata is the face it hangs on: 1 = on the block at -x, 2 = +x,
//! 3 = -z, 4 = +z, 5 = standing on the block below (BlockTorch.java:79-89).
//!
//! The virtual calls inside these bodies resolve to `BlockTorch`'s own
//! methods for every torch: `this.canPlaceBlockAt` (`BlockRedstoneTorch`
//! does not override it) and `this.onBlockAdded` (reached only from
//! `updateTick`, which `BlockRedstoneTorch` overrides).

// The Java if/else-if chains of `return true` are kept in their shape and order.
#![allow(clippy::if_same_then_else)]

use mc_core::java_random::JavaRandom;

use super::BlockBounds;
use super::dispatch;
use crate::world::World;

/// `Block.fence.blockID`.
// src: Block.java:677
const FENCE_ID: i32 = 85;

/// `BlockTorch.canPlaceTorchOn(world, x, y, z)` (private): a normal cube or
/// a fence.
// src: BlockTorch.java:27-29
fn can_place_torch_on(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    world.is_block_normal_cube(x, y, z) || world.get_block_id(x, y, z) == FENCE_ID
}

/// `BlockTorch.canPlaceBlockAt(world, x, y, z)`: a normal cube on one of the
/// four sides (-x, +x, -z, +z, in that order), else a torch support below.
// src: BlockTorch.java:31-43
pub fn can_place_block_at(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    if world.is_block_normal_cube(x - 1, y, z) {
        true
    } else if world.is_block_normal_cube(x + 1, y, z) {
        true
    } else if world.is_block_normal_cube(x, y, z - 1) {
        true
    } else if world.is_block_normal_cube(x, y, z + 1) {
        true
    } else {
        can_place_torch_on(world, x, y - 1, z)
    }
}

/// `BlockTorch.onBlockPlaced(world, x, y, z, side)`: orients the torch
/// toward the face it was placed against, when that face can hold it; the
/// metadata `onBlockAdded` chose stays otherwise. Each `if` is independent
/// (only one `side` can match).
// src: BlockTorch.java:45-68
pub fn on_block_placed(world: &mut World, x: i32, y: i32, z: i32, side: i32) {
    let mut var6 = world.get_block_metadata(x, y, z);
    if side == 1 && can_place_torch_on(world, x, y - 1, z) {
        var6 = 5;
    }
    if side == 2 && world.is_block_normal_cube(x, y, z + 1) {
        var6 = 4;
    }
    if side == 3 && world.is_block_normal_cube(x, y, z - 1) {
        var6 = 3;
    }
    if side == 4 && world.is_block_normal_cube(x + 1, y, z) {
        var6 = 2;
    }
    if side == 5 && world.is_block_normal_cube(x - 1, y, z) {
        var6 = 1;
    }
    world.set_block_metadata_with_notify(x, y, z, var6);
}

/// `BlockTorch.updateTick(world, x, y, z, rand)` (the wooden torch's random
/// tick; `BlockRedstoneTorch` overrides it): `super.updateTick` (empty,
/// Block.java:297-298), then a torch with metadata 0 orients itself again.
// src: BlockTorch.java:70-76
pub fn update_tick(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    if world.get_block_metadata(x, y, z) == 0 {
        on_block_added(world, id, x, y, z);
    }
}

/// `BlockTorch.onBlockAdded(world, x, y, z)`: the metadata of the first
/// support found (-x, +x, -z, +z, below), then `dropTorchIfCantStay`.
// src: BlockTorch.java:78-92
pub fn on_block_added(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    if world.is_block_normal_cube(x - 1, y, z) {
        world.set_block_metadata_with_notify(x, y, z, 1);
    } else if world.is_block_normal_cube(x + 1, y, z) {
        world.set_block_metadata_with_notify(x, y, z, 2);
    } else if world.is_block_normal_cube(x, y, z - 1) {
        world.set_block_metadata_with_notify(x, y, z, 3);
    } else if world.is_block_normal_cube(x, y, z + 1) {
        world.set_block_metadata_with_notify(x, y, z, 4);
    } else if can_place_torch_on(world, x, y - 1, z) {
        world.set_block_metadata_with_notify(x, y, z, 5);
    }
    drop_torch_if_cant_stay(world, id, x, y, z);
}

/// `BlockTorch.onNeighborBlockChange(world, x, y, z, neighborId)`: drops the
/// torch when nothing can hold it, or when the block its metadata hangs it
/// on is gone (the drop then reads the metadata again).
// src: BlockTorch.java:94-124
pub fn on_neighbor_block_change(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    if drop_torch_if_cant_stay(world, id, x, y, z) {
        let var6 = world.get_block_metadata(x, y, z);
        let mut var7 = false;
        if !world.is_block_normal_cube(x - 1, y, z) && var6 == 1 {
            var7 = true;
        }
        if !world.is_block_normal_cube(x + 1, y, z) && var6 == 2 {
            var7 = true;
        }
        if !world.is_block_normal_cube(x, y, z - 1) && var6 == 3 {
            var7 = true;
        }
        if !world.is_block_normal_cube(x, y, z + 1) && var6 == 4 {
            var7 = true;
        }
        if !can_place_torch_on(world, x, y - 1, z) && var6 == 5 {
            var7 = true;
        }
        if var7 {
            let meta = world.get_block_metadata(x, y, z);
            dispatch::drop_block_as_item(world, id, x, y, z, meta);
            world.set_block_with_notify(x, y, z, 0);
        }
    }
}

/// `BlockTorch.dropTorchIfCantStay(world, x, y, z)` (private): drops and
/// removes the torch when `canPlaceBlockAt` fails; true when it stays.
// src: BlockTorch.java:126-134
fn drop_torch_if_cant_stay(world: &mut World, id: i32, x: i32, y: i32, z: i32) -> bool {
    if !can_place_block_at(world, x, y, z) {
        let meta = world.get_block_metadata(x, y, z);
        dispatch::drop_block_as_item(world, id, x, y, z, meta);
        world.set_block_with_notify(x, y, z, 0);
        false
    } else {
        true
    }
}

/// The first half of `BlockTorch.collisionRayTrace`: the singleton's bounds
/// for the torch's metadata (`& 7`), which stay set afterwards (a vanilla
/// side effect on the shared singleton, transcribed). The dispatch then runs
/// `super.collisionRayTrace` (Block.java:375-464).
// src: BlockTorch.java:136-153
pub fn set_bounds_for_ray_trace(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    let var7 = world.get_block_metadata(x, y, z) & 7;
    let mut var8 = 0.15f32;
    let bounds = if var7 == 1 {
        BlockBounds::from_floats(0.0, 0.2, 0.5 - var8, var8 * 2.0, 0.8, 0.5 + var8)
    } else if var7 == 2 {
        BlockBounds::from_floats(1.0 - var8 * 2.0, 0.2, 0.5 - var8, 1.0, 0.8, 0.5 + var8)
    } else if var7 == 3 {
        BlockBounds::from_floats(0.5 - var8, 0.2, 0.0, 0.5 + var8, 0.8, var8 * 2.0)
    } else if var7 == 4 {
        BlockBounds::from_floats(0.5 - var8, 0.2, 1.0 - var8 * 2.0, 0.5 + var8, 0.8, 1.0)
    } else {
        var8 = 0.1;
        BlockBounds::from_floats(0.5 - var8, 0.0, 0.5 - var8, 0.5 + var8, 0.6, 0.5 + var8)
    };
    world.block_statics.bounds[(id & 255) as usize] = bounds;
}

/// The flame offsets of the torch particles.
// src: BlockTorch.java:160 (and BlockRedstoneTorch.java:157)
pub(super) const PARTICLE_UP: f64 = 0.2199999988079071;
// src: BlockTorch.java:161 (and BlockRedstoneTorch.java:158)
pub(super) const PARTICLE_OUT: f64 = 0.27000001072883606;

/// `BlockTorch.randomDisplayTick(world, x, y, z, rand)`: a smoke and a flame
/// particle at the flame (client only; no draws from `rand`).
// src: BlockTorch.java:155-179
pub fn random_display_tick(world: &mut World, x: i32, y: i32, z: i32, rand: &mut JavaRandom) {
    let _ = rand;
    let var6 = world.get_block_metadata(x, y, z);
    let var7 = (x as f32 + 0.5) as f64;
    let var9 = (y as f32 + 0.7) as f64;
    let var11 = (z as f32 + 0.5) as f64;
    let var13 = PARTICLE_UP;
    let var15 = PARTICLE_OUT;
    let (px, py, pz) = if var6 == 1 {
        (var7 - var15, var9 + var13, var11)
    } else if var6 == 2 {
        (var7 + var15, var9 + var13, var11)
    } else if var6 == 3 {
        (var7, var9 + var13, var11 - var15)
    } else if var6 == 4 {
        (var7, var9 + var13, var11 + var15)
    } else {
        (var7, var9, var11)
    };
    world.spawn_particle("smoke", px, py, pz, 0.0, 0.0, 0.0);
    world.spawn_particle("flame", px, py, pz, 0.0, 0.0, 0.0);
}
