//! `BlockRedstoneTorch` (BlockRedstoneTorch.java, RedstoneUpdateInfo.java):
//! the idle (id 75) and active (id 76) redstone torch, a [`super::torch`]
//! subclass. `torchActive` is [`super::BlockKind::RedstoneTorch`]'s
//! `torch_active`.
//!
//! Fully transcribed: `checkForBurnout`, `onBlockAdded`, `onBlockRemoval`,
//! `isPoweringTo`, `isIndirectlyPowered`, `updateTick`,
//! `onNeighborBlockChange`, `isIndirectlyPoweringTo`, `canProvidePower`,
//! `randomDisplayTick` (particle events only). Data methods (`tickRate` 2,
//! `idDropped`, the texture) are in [`crate::block`].
//!
//! The static `torchUpdates` list is world state:
//! [`crate::block::BlockStatics::redstone_torch_updates`].

// The Java if/else-if chains of `return true` are kept in their shape and order.
#![allow(clippy::if_same_then_else)]

use mc_core::java_random::JavaRandom;

use super::dispatch::RandomSource;
use crate::world::World;

/// `Block.torchRedstoneIdle.blockID`.
// src: Block.java:667
const TORCH_REDSTONE_IDLE: i32 = 75;
/// `Block.torchRedstoneActive.blockID`.
// src: Block.java:668
const TORCH_REDSTONE_ACTIVE: i32 = 76;

/// `RedstoneUpdateInfo`: one entry of the burnout history.
// src: RedstoneUpdateInfo.java:3-15
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RedstoneUpdateInfo {
    /// `x`.
    pub x: i32,
    /// `y`.
    pub y: i32,
    /// `z`.
    pub z: i32,
    /// `updateTime`: the world time of the turn-off.
    pub update_time: i64,
}

/// `checkForBurnout(world, x, y, z, add)` (private): optionally records a
/// turn-off now, then whether the history holds at least 8 entries for this
/// position (counting stops at the 8th).
// src: BlockRedstoneTorch.java:15-33
fn check_for_burnout(world: &mut World, x: i32, y: i32, z: i32, add: bool) -> bool {
    if add {
        let t = world.get_world_time();
        world
            .block_statics
            .redstone_torch_updates
            .push(RedstoneUpdateInfo {
                x,
                y,
                z,
                update_time: t,
            });
    }
    let mut var6 = 0;
    for var8 in &world.block_statics.redstone_torch_updates {
        if var8.x == x && var8.y == y && var8.z == z {
            var6 += 1;
            if var6 >= 8 {
                return true;
            }
        }
    }
    false
}

/// `notifyBlocksOfNeighborChange` around the torch, in the order of
/// `onBlockAdded` / `onBlockRemoval`: below, above, -x, +x, -z, +z.
// src: BlockRedstoneTorch.java:51-56, :63-68
fn notify_all_around(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    world.notify_blocks_of_neighbor_change(x, y - 1, z, id);
    world.notify_blocks_of_neighbor_change(x, y + 1, z, id);
    world.notify_blocks_of_neighbor_change(x - 1, y, z, id);
    world.notify_blocks_of_neighbor_change(x + 1, y, z, id);
    world.notify_blocks_of_neighbor_change(x, y, z - 1, id);
    world.notify_blocks_of_neighbor_change(x, y, z + 1, id);
}

/// `onBlockAdded(world, x, y, z)`: `BlockTorch.onBlockAdded` only for a
/// torch without orientation yet (metadata 0), then an active torch
/// notifies the neighbours of all six neighbours.
// src: BlockRedstoneTorch.java:45-59
pub fn on_block_added(world: &mut World, id: i32, x: i32, y: i32, z: i32, torch_active: bool) {
    if world.get_block_metadata(x, y, z) == 0 {
        super::torch::on_block_added(world, id, x, y, z);
    }
    if torch_active {
        notify_all_around(world, id, x, y, z);
    }
}

/// `onBlockRemoval(world, x, y, z)`: an active torch notifies the neighbours
/// of all six neighbours (`super.onBlockRemoval` is not called; Block's is
/// empty anyway).
// src: BlockRedstoneTorch.java:61-71
pub fn on_block_removal(world: &mut World, id: i32, x: i32, y: i32, z: i32, torch_active: bool) {
    if torch_active {
        notify_all_around(world, id, x, y, z);
    }
}

/// `isPoweringTo(blockAccess, x, y, z, side)`: an active torch powers every
/// side but the one toward the block it hangs on.
// src: BlockRedstoneTorch.java:73-90
pub fn is_powering_to(
    world: &mut World,
    x: i32,
    y: i32,
    z: i32,
    side: i32,
    torch_active: bool,
) -> bool {
    if !torch_active {
        false
    } else {
        let var6 = world.get_block_metadata(x, y, z);
        if var6 == 5 && side == 1 {
            false
        } else if var6 == 3 && side == 3 {
            false
        } else if var6 == 4 && side == 2 {
            false
        } else if var6 == 1 && side == 5 {
            false
        } else {
            var6 != 2 || side != 4
        }
    }
}

/// `isIndirectlyPowered(world, x, y, z)` (private): whether the block the
/// torch hangs on is powered (`isBlockIndirectlyProvidingPowerTo` toward the
/// torch).
// src: BlockRedstoneTorch.java:92-105
fn is_indirectly_powered(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    let var5 = world.get_block_metadata(x, y, z);
    if var5 == 5 && world.is_block_indirectly_providing_power_to(x, y - 1, z, 0) {
        true
    } else if var5 == 3 && world.is_block_indirectly_providing_power_to(x, y, z - 1, 2) {
        true
    } else if var5 == 4 && world.is_block_indirectly_providing_power_to(x, y, z + 1, 3) {
        true
    } else if var5 == 1 && world.is_block_indirectly_providing_power_to(x - 1, y, z, 4) {
        true
    } else {
        var5 == 2 && world.is_block_indirectly_providing_power_to(x + 1, y, z, 5)
    }
}

/// `updateTick(world, x, y, z, rand)`: drops history entries older than 100
/// ticks from the front; an active torch whose support is powered turns
/// idle (and, at the 8th turn-off within the history, fizzes: a sound and
/// five smoke particles); an idle torch whose support is unpowered turns
/// active unless burnt out. `rand` is `World.rand` (the scheduled-tick
/// caller, World.java:1960), which the fizz also reads as `var1.rand`.
// src: BlockRedstoneTorch.java:107-132
pub fn update_tick(
    world: &mut World,
    x: i32,
    y: i32,
    z: i32,
    rand: RandomSource,
    torch_active: bool,
) {
    let var6 = is_indirectly_powered(world, x, y, z);
    // src: BlockRedstoneTorch.java:110-112 (`> 100L` on the long difference)
    while !world.block_statics.redstone_torch_updates.is_empty()
        && world
            .get_world_time()
            .wrapping_sub(world.block_statics.redstone_torch_updates[0].update_time)
            > 100
    {
        world.block_statics.redstone_torch_updates.remove(0);
    }
    if torch_active {
        if var6 {
            let meta = world.get_block_metadata(x, y, z);
            world.set_block_and_metadata_with_notify(x, y, z, TORCH_REDSTONE_IDLE, meta);
            if check_for_burnout(world, x, y, z, true) {
                // src: BlockRedstoneTorch.java:118: `var1.rand`, two draws, a float sum
                let a = world.rand.next_float();
                let b = world.rand.next_float();
                let pitch = 2.6f32 + (a - b) * 0.8f32;
                world.play_sound_effect(
                    (x as f32 + 0.5) as f64,
                    (y as f32 + 0.5) as f64,
                    (z as f32 + 0.5) as f64,
                    "random.fizz",
                    0.5,
                    pitch,
                );
                // src: BlockRedstoneTorch.java:120-125 (`var5`, the tick's Random)
                for _var7 in 0..5 {
                    let r = world.random(rand);
                    let var8 = x as f64 + r.next_double() * 0.6 + 0.2;
                    let var10 = y as f64 + r.next_double() * 0.6 + 0.2;
                    let var12 = z as f64 + r.next_double() * 0.6 + 0.2;
                    world.spawn_particle("smoke", var8, var10, var12, 0.0, 0.0, 0.0);
                }
            }
        }
    } else if !var6 && !check_for_burnout(world, x, y, z, false) {
        let meta = world.get_block_metadata(x, y, z);
        world.set_block_and_metadata_with_notify(x, y, z, TORCH_REDSTONE_ACTIVE, meta);
    }
}

/// `onNeighborBlockChange(world, x, y, z, neighborId)`:
/// `BlockTorch.onNeighborBlockChange` (which may drop the torch), then a
/// tick in `tickRate()` = 2 for `this.blockID` whatever is left there.
// src: BlockRedstoneTorch.java:134-137
pub fn on_neighbor_block_change(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    super::torch::on_neighbor_block_change(world, id, x, y, z);
    // src: BlockRedstoneTorch.java:41-43 (tickRate)
    world.schedule_block_update(x, y, z, id, super::dispatch::tick_rate(id));
}

/// `isIndirectlyPoweringTo(world, x, y, z, side)`: strong power only
/// upward (side 0, into the block above), as `isPoweringTo`.
// src: BlockRedstoneTorch.java:139-141
pub fn is_indirectly_powering_to(
    world: &mut World,
    x: i32,
    y: i32,
    z: i32,
    side: i32,
    torch_active: bool,
) -> bool {
    if side == 0 {
        is_powering_to(world, x, y, z, side, torch_active)
    } else {
        false
    }
}

/// `canProvidePower()`: true (both torches).
// src: BlockRedstoneTorch.java:147-149
pub fn can_provide_power() -> bool {
    true
}

/// `randomDisplayTick(world, x, y, z, rand)`: an active torch shows a red
/// dust particle near its tip (three `rand.nextFloat()` draws; client only).
// src: BlockRedstoneTorch.java:151-172
pub fn random_display_tick(
    world: &mut World,
    x: i32,
    y: i32,
    z: i32,
    rand: &mut JavaRandom,
    torch_active: bool,
) {
    if torch_active {
        let var6 = world.get_block_metadata(x, y, z);
        let var7 = (x as f32 + 0.5) as f64 + (rand.next_float() - 0.5) as f64 * 0.2;
        let var9 = (y as f32 + 0.7) as f64 + (rand.next_float() - 0.5) as f64 * 0.2;
        let var11 = (z as f32 + 0.5) as f64 + (rand.next_float() - 0.5) as f64 * 0.2;
        let var13 = super::torch::PARTICLE_UP;
        let var15 = super::torch::PARTICLE_OUT;
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
        world.spawn_particle("reddust", px, py, pz, 0.0, 0.0, 0.0);
    }
}
