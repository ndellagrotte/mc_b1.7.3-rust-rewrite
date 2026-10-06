//! `BlockRedstoneRepeater` (BlockRedstoneRepeater.java): the idle (id 93)
//! and powered (id 94) repeater. `isRepeaterPowered` is
//! [`super::BlockKind::RedstoneRepeater`]'s `is_repeater_powered`.
//!
//! Metadata: bits 0-1 the facing (the output is toward -z, +x, +z, -x for 0,
//! 1, 2, 3: `isPoweringTo`), bits 2-3 the delay setting, an index into
//! [`super::REPEATER_DELAYS`] (`field_22023_b`; the tick delay is twice the
//! entry).
//!
//! Fully transcribed: `canPlaceBlockAt`, `canBlockStay`, `updateTick`,
//! `shouldSideBeRendered`, `isIndirectlyPoweringTo`, `isPoweringTo`,
//! `onNeighborBlockChange`, `ignoreTick`, `blockActivated`,
//! `canProvidePower`, `onBlockPlacedBy`, `onBlockAdded`, `randomDisplayTick`
//! (particle events only). Data methods (textures, render type,
//! `idDropped`) are in [`crate::block`].

// The Java if/else-if chains of `return true` are kept in their shape and order.
#![allow(clippy::if_same_then_else)]

use mc_core::java_random::JavaRandom;
use mc_core::math::floor_double;

use super::{REPEATER_DELAYS, REPEATER_TORCH_OFFSET};
use crate::entity::Entity;
use crate::world::World;

/// `Block.redstoneWire.blockID`.
// src: Block.java:647
const REDSTONE_WIRE: i32 = 55;
/// `Block.redstoneRepeaterIdle.blockID`.
// src: Block.java:685
const REDSTONE_REPEATER_IDLE: i32 = 93;
/// `Block.redstoneRepeaterActive.blockID`.
// src: Block.java:686
const REDSTONE_REPEATER_ACTIVE: i32 = 94;

/// `canPlaceBlockAt(world, x, y, z)`: a normal cube below, then
/// `super.canPlaceBlockAt` (Block's: the target is air or ground cover).
// src: BlockRedstoneRepeater.java:20-22, Block.java:501-504
pub fn can_place_block_at(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    if !world.is_block_normal_cube(x, y - 1, z) {
        false
    } else {
        let var5 = world.get_block_id(x, y, z);
        var5 == 0 || super::material_of(var5).get_is_ground_cover()
    }
}

/// `canBlockStay(world, x, y, z)`: a normal cube below (`super.canBlockStay`
/// is Block's `true`).
// src: BlockRedstoneRepeater.java:24-26, Block.java:556-558
pub fn can_block_stay(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    world.is_block_normal_cube(x, y - 1, z)
}

/// `updateTick(world, x, y, z, rand)`: a powered repeater whose input is off
/// turns idle; an idle one turns powered and, if its input is already off
/// again, schedules the powered one to turn off after its delay.
// src: BlockRedstoneRepeater.java:28-41
pub fn update_tick(world: &mut World, x: i32, y: i32, z: i32, is_repeater_powered: bool) {
    let var6 = world.get_block_metadata(x, y, z);
    let var7 = ignore_tick(world, x, y, z, var6);
    if is_repeater_powered && !var7 {
        world.set_block_and_metadata_with_notify(x, y, z, REDSTONE_REPEATER_IDLE, var6);
    } else if !is_repeater_powered {
        world.set_block_and_metadata_with_notify(x, y, z, REDSTONE_REPEATER_ACTIVE, var6);
        if !var7 {
            let var8 = (var6 & 12) >> 2;
            world.schedule_block_update(
                x,
                y,
                z,
                REDSTONE_REPEATER_ACTIVE,
                REPEATER_DELAYS[var8 as usize] * 2,
            );
        }
    }
}

/// `shouldSideBeRendered(blockAccess, x, y, z, side)`: every side but the
/// bottom and the top (the renderer draws those itself).
// src: BlockRedstoneRepeater.java:53-55
pub fn should_side_be_rendered(side: i32) -> bool {
    side != 0 && side != 1
}

/// `isIndirectlyPoweringTo(world, x, y, z, side)`: `isPoweringTo`.
// src: BlockRedstoneRepeater.java:65-67
pub fn is_indirectly_powering_to(
    world: &mut World,
    x: i32,
    y: i32,
    z: i32,
    side: i32,
    is_repeater_powered: bool,
) -> bool {
    is_powering_to(world, x, y, z, side, is_repeater_powered)
}

/// `isPoweringTo(blockAccess, x, y, z, side)`: a powered repeater powers the
/// block in front of it only.
// src: BlockRedstoneRepeater.java:69-84
pub fn is_powering_to(
    world: &mut World,
    x: i32,
    y: i32,
    z: i32,
    side: i32,
    is_repeater_powered: bool,
) -> bool {
    if !is_repeater_powered {
        false
    } else {
        let var6 = world.get_block_metadata(x, y, z) & 3;
        if var6 == 0 && side == 3 {
            true
        } else if var6 == 1 && side == 4 {
            true
        } else if var6 == 2 && side == 2 {
            true
        } else {
            var6 == 3 && side == 5
        }
    }
}

/// `onNeighborBlockChange(world, x, y, z, neighborId)`: drops the repeater
/// without support; otherwise schedules a tick (after the delay) when the
/// input disagrees with the state.
// src: BlockRedstoneRepeater.java:86-101
pub fn on_neighbor_block_change(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    is_repeater_powered: bool,
) {
    if !can_block_stay(world, x, y, z) {
        let meta = world.get_block_metadata(x, y, z);
        super::dispatch::drop_block_as_item(world, id, x, y, z, meta);
        world.set_block_with_notify(x, y, z, 0);
    } else {
        let var6 = world.get_block_metadata(x, y, z);
        let var7 = ignore_tick(world, x, y, z, var6);
        let var8 = (var6 & 12) >> 2;
        if (is_repeater_powered && !var7) || (!is_repeater_powered && var7) {
            world.schedule_block_update(x, y, z, id, REPEATER_DELAYS[var8 as usize] * 2);
        }
    }
}

/// `ignoreTick(world, x, y, z, meta)` (private; the name is MCP's, it means
/// "the input is powered"): the block behind the repeater provides power
/// toward it, or is wire with a level above 0.
// src: BlockRedstoneRepeater.java:103-117
fn ignore_tick(world: &mut World, x: i32, y: i32, z: i32, meta: i32) -> bool {
    let input = |world: &mut World, ix: i32, iz: i32, side: i32| {
        world.is_block_indirectly_providing_power_to(ix, y, iz, side)
            || (world.get_block_id(ix, y, iz) == REDSTONE_WIRE
                && world.get_block_metadata(ix, y, iz) > 0)
    };
    match meta & 3 {
        0 => input(world, x, z + 1, 3),
        1 => input(world, x - 1, z, 4),
        2 => input(world, x, z - 1, 2),
        3 => input(world, x + 1, z, 5),
        _ => false,
    }
}

/// `blockActivated(world, x, y, z, player)`: cycles the delay setting,
/// keeping the facing; always true.
// src: BlockRedstoneRepeater.java:119-125
pub fn block_activated(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    let var6 = world.get_block_metadata(x, y, z);
    let mut var7 = (var6 & 12) >> 2;
    // src: BlockRedstoneRepeater.java:122: `var7 + 1 << 2 & 12` is ((var7 + 1) << 2) & 12
    var7 = ((var7 + 1) << 2) & 12;
    world.set_block_metadata_with_notify(x, y, z, var7 | (var6 & 3));
    true
}

/// `canProvidePower()`: false (wire connects to repeaters through the
/// repeater special case of `isPowerProviderOrWire` instead).
// src: BlockRedstoneRepeater.java:127-129
pub fn can_provide_power() -> bool {
    false
}

/// `onBlockPlacedBy(world, x, y, z, placer)`: faces away from the placer
/// (`((floor(yaw * 4 / 360 + 0.5) & 3) + 2) % 4`), and if the input is
/// already powered, ticks next tick.
// src: BlockRedstoneRepeater.java:131-139
pub fn on_block_placed_by(world: &mut World, id: i32, x: i32, y: i32, z: i32, placer: &Entity) {
    // src: BlockRedstoneRepeater.java:132: a float product and quotient, widened, + 0.5D
    let var6 = ((floor_double((placer.base.rotation_yaw * 4.0 / 360.0) as f64 + 0.5) & 3) + 2) % 4;
    world.set_block_metadata_with_notify(x, y, z, var6);
    let var7 = ignore_tick(world, x, y, z, var6);
    if var7 {
        world.schedule_block_update(x, y, z, id, 1);
    }
}

/// `onBlockAdded(world, x, y, z)`: notifies the neighbours of all six
/// neighbours (+x, -x, +z, -z, below, above).
// src: BlockRedstoneRepeater.java:141-148
pub fn on_block_added(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    world.notify_blocks_of_neighbor_change(x + 1, y, z, id);
    world.notify_blocks_of_neighbor_change(x - 1, y, z, id);
    world.notify_blocks_of_neighbor_change(x, y, z + 1, id);
    world.notify_blocks_of_neighbor_change(x, y, z - 1, id);
    world.notify_blocks_of_neighbor_change(x, y - 1, z, id);
    world.notify_blocks_of_neighbor_change(x, y + 1, z, id);
}

/// `randomDisplayTick(world, x, y, z, rand)`: a powered repeater shows a red
/// dust particle on one of its two torches (three `nextFloat` and one
/// `nextInt(2)` draws; client only).
// src: BlockRedstoneRepeater.java:158-199
pub fn random_display_tick(
    world: &mut World,
    x: i32,
    y: i32,
    z: i32,
    rand: &mut JavaRandom,
    is_repeater_powered: bool,
) {
    if is_repeater_powered {
        let var6 = world.get_block_metadata(x, y, z);
        let var7 = (x as f32 + 0.5) as f64 + (rand.next_float() - 0.5) as f64 * 0.2;
        let var9 = (y as f32 + 0.4) as f64 + (rand.next_float() - 0.5) as f64 * 0.2;
        let var11 = (z as f32 + 0.5) as f64 + (rand.next_float() - 0.5) as f64 * 0.2;
        let mut var13 = 0.0f64;
        let mut var15 = 0.0f64;
        if rand.next_int_bounded(2) == 0 {
            match var6 & 3 {
                0 => var15 = -0.3125,
                1 => var13 = 0.3125,
                2 => var15 = 0.3125,
                3 => var13 = -0.3125,
                _ => {}
            }
        } else {
            let var17 = ((var6 & 12) >> 2) as usize;
            match var6 & 3 {
                0 => var15 = REPEATER_TORCH_OFFSET[var17],
                1 => var13 = -REPEATER_TORCH_OFFSET[var17],
                2 => var15 = -REPEATER_TORCH_OFFSET[var17],
                3 => var13 = REPEATER_TORCH_OFFSET[var17],
                _ => {}
            }
        }
        world.spawn_particle("reddust", var7 + var13, var9, var11 + var15, 0.0, 0.0, 0.0);
    }
}
