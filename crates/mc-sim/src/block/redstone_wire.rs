//! `BlockRedstoneWire` (BlockRedstoneWire.java): redstone dust (id 55).
//! Metadata is the power level 0..15.
//!
//! Fully transcribed: `getCollisionBoundingBoxFromPool`, `colorMultiplier`,
//! `canPlaceBlockAt`, `updateAndPropagateCurrentStrength`,
//! `calculateCurrentChanges`, `notifyWireNeighborsOfNeighborChange`,
//! `onBlockAdded`, `onBlockRemoval`, `getMaxCurrentStrength`,
//! `onNeighborBlockChange`, `isIndirectlyPoweringTo`, `isPoweringTo`,
//! `canProvidePower`, `randomDisplayTick` (particle events only) and the
//! static `isPowerProviderOrWire`. Data methods (`idDropped`, texture,
//! render type) are in [`crate::block`].
//!
//! The singleton's two mutable fields are world state:
//! `wiresProvidePower` is
//! [`crate::block::BlockStatics::redstone_wire_wires_provide_power`] and
//! `blocksNeedingUpdate`, a `HashSet<ChunkPosition>` whose iteration order
//! decides the order of the neighbour notifications, is
//! [`crate::block::BlockStatics::redstone_wire_blocks_needing_update`]
//! (a [`mc_core::java_hash::JavaHashSet`], PLAN_v4 §3.4 rule 4).

// The Java if/else-if chains of `return true` are kept in their shape and order.
#![allow(clippy::if_same_then_else)]

use mc_core::java_hash::ChunkPosition;
use mc_core::java_random::JavaRandom;

use super::dispatch;
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

/// `ModelBed.footInvisibleFaceRemap`: for a repeater's facing (`meta & 3`),
/// the `isPowerProviderOrWire` direction code it connects on.
// src: ModelBed.java:5
const FOOT_INVISIBLE_FACE_REMAP: [i32; 4] = [2, 3, 0, 1];

/// `colorMultiplier(blockAccess, x, y, z)`: dark red, `0x800000`.
// src: BlockRedstoneWire.java:37-39
pub fn color_multiplier() -> i32 {
    8388608
}

/// `canPlaceBlockAt(world, x, y, z)`: a normal cube below.
// src: BlockRedstoneWire.java:41-43
pub fn can_place_block_at(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    world.is_block_normal_cube(x, y - 1, z)
}

/// `updateAndPropagateCurrentStrength(world, x, y, z)` (private): recompute
/// this wire and, recursively, the wires around it, then notify the
/// neighbours of every position collected in `blocksNeedingUpdate`, in the
/// set's iteration order (`new ArrayList(set)`), after clearing the set.
// src: BlockRedstoneWire.java:45-55
fn update_and_propagate_current_strength(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    calculate_current_changes(world, x, y, z, x, y, z);
    let var5: Vec<ChunkPosition> = world
        .block_statics
        .redstone_wire_blocks_needing_update
        .iter()
        .copied()
        .collect();
    world
        .block_statics
        .redstone_wire_blocks_needing_update
        .clear();
    for var7 in var5 {
        world.notify_blocks_of_neighbor_change(var7.x, var7.y, var7.z, id);
    }
}

/// `calculateCurrentChanges(world, x, y, z, fromX, fromY, fromZ)` (private):
/// the wire's new power (15 when its own position is powered by something
/// other than wire, else the strongest connected wire minus one, ignoring
/// the wire at `from`); on a change, writes it without notification,
/// recurses into the neighbouring wires whose level disagrees, and, when the
/// wire switched on or off (old or new level 0), queues its position and
/// its six neighbours for notification.
// src: BlockRedstoneWire.java:57-171
fn calculate_current_changes(
    world: &mut World,
    x: i32,
    y: i32,
    z: i32,
    from_x: i32,
    from_y: i32,
    from_z: i32,
) {
    let var8 = world.get_block_metadata(x, y, z);
    let mut var9 = 0;
    // src: BlockRedstoneWire.java:60-62: wires stop providing power while the
    // wire asks whether anything else powers it.
    world.block_statics.redstone_wire_wires_provide_power = false;
    let var10 = world.is_block_indirectly_getting_powered(x, y, z);
    world.block_statics.redstone_wire_wires_provide_power = true;
    if var10 {
        var9 = 15;
    } else {
        for var11 in 0..4 {
            let mut var12 = x;
            let mut var13 = z;
            if var11 == 0 {
                var12 = x - 1;
            }
            if var11 == 1 {
                var12 += 1;
            }
            if var11 == 2 {
                var13 = z - 1;
            }
            if var11 == 3 {
                var13 += 1;
            }
            if var12 != from_x || y != from_y || var13 != from_z {
                var9 = get_max_current_strength(world, var12, y, var13, var9);
            }
            if world.is_block_normal_cube(var12, y, var13)
                && !world.is_block_normal_cube(x, y + 1, z)
            {
                if var12 != from_x || y + 1 != from_y || var13 != from_z {
                    var9 = get_max_current_strength(world, var12, y + 1, var13, var9);
                }
            } else if !world.is_block_normal_cube(var12, y, var13)
                && (var12 != from_x || y - 1 != from_y || var13 != from_z)
            {
                var9 = get_max_current_strength(world, var12, y - 1, var13, var9);
            }
        }
        if var9 > 0 {
            var9 -= 1;
        } else {
            var9 = 0;
        }
    }

    if var8 != var9 {
        // src: BlockRedstoneWire.java:109-112
        world.editing_blocks = true;
        world.set_block_metadata_with_notify(x, y, z, var9);
        world.mark_blocks_dirty(x, y, z, x, y, z);
        world.editing_blocks = false;

        for var11 in 0..4 {
            let mut var12 = x;
            let mut var13 = z;
            let mut var14 = y - 1;
            if var11 == 0 {
                var12 = x - 1;
            }
            if var11 == 1 {
                var12 += 1;
            }
            if var11 == 2 {
                var13 = z - 1;
            }
            if var11 == 3 {
                var13 += 1;
            }
            if world.is_block_normal_cube(var12, y, var13) {
                var14 += 2;
            }
            let mut var16 = get_max_current_strength(world, var12, y, var13, -1);
            var9 = world.get_block_metadata(x, y, z);
            if var9 > 0 {
                var9 -= 1;
            }
            if var16 >= 0 && var16 != var9 {
                calculate_current_changes(world, var12, y, var13, x, y, z);
            }
            var16 = get_max_current_strength(world, var12, var14, var13, -1);
            var9 = world.get_block_metadata(x, y, z);
            if var9 > 0 {
                var9 -= 1;
            }
            if var16 >= 0 && var16 != var9 {
                calculate_current_changes(world, var12, var14, var13, x, y, z);
            }
        }

        // src: BlockRedstoneWire.java:160-168 (var9 is the last re-read level minus one)
        if var8 == 0 || var9 == 0 {
            let set = &mut world.block_statics.redstone_wire_blocks_needing_update;
            set.insert(ChunkPosition { x, y, z });
            set.insert(ChunkPosition { x: x - 1, y, z });
            set.insert(ChunkPosition { x: x + 1, y, z });
            set.insert(ChunkPosition { x, y: y - 1, z });
            set.insert(ChunkPosition { x, y: y + 1, z });
            set.insert(ChunkPosition { x, y, z: z - 1 });
            set.insert(ChunkPosition { x, y, z: z + 1 });
        }
    }
}

/// `notifyWireNeighborsOfNeighborChange(world, x, y, z)` (private): if there
/// is wire at the position, notify its neighbours and the neighbours of its
/// six neighbours (-x, +x, -z, +z, below, above).
// src: BlockRedstoneWire.java:173-183
fn notify_wire_neighbors_of_neighbor_change(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    if world.get_block_id(x, y, z) == id {
        world.notify_blocks_of_neighbor_change(x, y, z, id);
        world.notify_blocks_of_neighbor_change(x - 1, y, z, id);
        world.notify_blocks_of_neighbor_change(x + 1, y, z, id);
        world.notify_blocks_of_neighbor_change(x, y, z - 1, id);
        world.notify_blocks_of_neighbor_change(x, y, z + 1, id);
        world.notify_blocks_of_neighbor_change(x, y - 1, z, id);
        world.notify_blocks_of_neighbor_change(x, y + 1, z, id);
    }
}

/// The wire-neighbour notifications shared by `onBlockAdded` and
/// `onBlockRemoval`: the four sides, then for each side the wire one up
/// (side block a normal cube) or one down (otherwise).
// src: BlockRedstoneWire.java:191-217, :228-254
fn notify_wire_neighbors_around(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    notify_wire_neighbors_of_neighbor_change(world, id, x - 1, y, z);
    notify_wire_neighbors_of_neighbor_change(world, id, x + 1, y, z);
    notify_wire_neighbors_of_neighbor_change(world, id, x, y, z - 1);
    notify_wire_neighbors_of_neighbor_change(world, id, x, y, z + 1);
    if world.is_block_normal_cube(x - 1, y, z) {
        notify_wire_neighbors_of_neighbor_change(world, id, x - 1, y + 1, z);
    } else {
        notify_wire_neighbors_of_neighbor_change(world, id, x - 1, y - 1, z);
    }
    if world.is_block_normal_cube(x + 1, y, z) {
        notify_wire_neighbors_of_neighbor_change(world, id, x + 1, y + 1, z);
    } else {
        notify_wire_neighbors_of_neighbor_change(world, id, x + 1, y - 1, z);
    }
    if world.is_block_normal_cube(x, y, z - 1) {
        notify_wire_neighbors_of_neighbor_change(world, id, x, y + 1, z - 1);
    } else {
        notify_wire_neighbors_of_neighbor_change(world, id, x, y - 1, z - 1);
    }
    if world.is_block_normal_cube(x, y, z + 1) {
        notify_wire_neighbors_of_neighbor_change(world, id, x, y + 1, z + 1);
    } else {
        notify_wire_neighbors_of_neighbor_change(world, id, x, y - 1, z + 1);
    }
}

/// `onBlockAdded(world, x, y, z)`: `super.onBlockAdded` (empty), then, off a
/// multiplayer client: propagate, notify above and below, notify the
/// surrounding wires.
// src: BlockRedstoneWire.java:185-220
pub fn on_block_added(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    if !world.multiplayer_world {
        update_and_propagate_current_strength(world, id, x, y, z);
        world.notify_blocks_of_neighbor_change(x, y + 1, z, id);
        world.notify_blocks_of_neighbor_change(x, y - 1, z, id);
        notify_wire_neighbors_around(world, id, x, y, z);
    }
}

/// `onBlockRemoval(world, x, y, z)`: `super.onBlockRemoval` (empty), then, off
/// a multiplayer client: notify above and below, propagate (the wire is
/// already gone: the block id was replaced before this call, Chunk.java:245-248),
/// notify the surrounding wires.
// src: BlockRedstoneWire.java:222-257
pub fn on_block_removal(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    if !world.multiplayer_world {
        world.notify_blocks_of_neighbor_change(x, y + 1, z, id);
        world.notify_blocks_of_neighbor_change(x, y - 1, z, id);
        update_and_propagate_current_strength(world, id, x, y, z);
        notify_wire_neighbors_around(world, id, x, y, z);
    }
}

/// `getMaxCurrentStrength(world, x, y, z, current)` (private): the larger of
/// `current` and the level of a wire at the position (`current` when there
/// is no wire).
// src: BlockRedstoneWire.java:259-266
fn get_max_current_strength(world: &mut World, x: i32, y: i32, z: i32, current: i32) -> i32 {
    if world.get_block_id(x, y, z) != REDSTONE_WIRE {
        current
    } else {
        let var6 = world.get_block_metadata(x, y, z);
        if var6 > current { var6 } else { current }
    }
}

/// `onNeighborBlockChange(world, x, y, z, neighborId)`: off a multiplayer
/// client, drop the wire when it lost its support, else propagate; then
/// `super.onNeighborBlockChange` (empty).
// src: BlockRedstoneWire.java:268-281
pub fn on_neighbor_block_change(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    if !world.multiplayer_world {
        let var6 = world.get_block_metadata(x, y, z);
        let var7 = can_place_block_at(world, x, y, z);
        if !var7 {
            dispatch::drop_block_as_item(world, id, x, y, z, var6);
            world.set_block_with_notify(x, y, z, 0);
        } else {
            update_and_propagate_current_strength(world, id, x, y, z);
        }
    }
}

/// `isIndirectlyPoweringTo(world, x, y, z, side)`: as `isPoweringTo` while
/// wires provide power.
// src: BlockRedstoneWire.java:287-289
pub fn is_indirectly_powering_to(world: &mut World, x: i32, y: i32, z: i32, side: i32) -> bool {
    if !world.block_statics.redstone_wire_wires_provide_power {
        false
    } else {
        is_powering_to(world, x, y, z, side)
    }
}

/// `isPoweringTo(blockAccess, x, y, z, side)`: a powered wire powers the
/// block below (side 1) always, and a side block when it points at it: when
/// it connects to nothing (all four sides), or when it runs straight toward
/// that side (connected on that axis only).
// src: BlockRedstoneWire.java:291-333
pub fn is_powering_to(world: &mut World, x: i32, y: i32, z: i32, side: i32) -> bool {
    if !world.block_statics.redstone_wire_wires_provide_power {
        return false;
    } else if world.get_block_metadata(x, y, z) == 0 {
        return false;
    } else if side == 1 {
        return true;
    }
    let mut var6 = is_power_provider_or_wire(world, x - 1, y, z, 1)
        || (!world.is_block_normal_cube(x - 1, y, z)
            && is_power_provider_or_wire(world, x - 1, y - 1, z, -1));
    let mut var7 = is_power_provider_or_wire(world, x + 1, y, z, 3)
        || (!world.is_block_normal_cube(x + 1, y, z)
            && is_power_provider_or_wire(world, x + 1, y - 1, z, -1));
    let mut var8 = is_power_provider_or_wire(world, x, y, z - 1, 2)
        || (!world.is_block_normal_cube(x, y, z - 1)
            && is_power_provider_or_wire(world, x, y - 1, z - 1, -1));
    let mut var9 = is_power_provider_or_wire(world, x, y, z + 1, 0)
        || (!world.is_block_normal_cube(x, y, z + 1)
            && is_power_provider_or_wire(world, x, y - 1, z + 1, -1));
    if !world.is_block_normal_cube(x, y + 1, z) {
        if world.is_block_normal_cube(x - 1, y, z)
            && is_power_provider_or_wire(world, x - 1, y + 1, z, -1)
        {
            var6 = true;
        }
        if world.is_block_normal_cube(x + 1, y, z)
            && is_power_provider_or_wire(world, x + 1, y + 1, z, -1)
        {
            var7 = true;
        }
        if world.is_block_normal_cube(x, y, z - 1)
            && is_power_provider_or_wire(world, x, y + 1, z - 1, -1)
        {
            var8 = true;
        }
        if world.is_block_normal_cube(x, y, z + 1)
            && is_power_provider_or_wire(world, x, y + 1, z + 1, -1)
        {
            var9 = true;
        }
    }
    if !var8 && !var7 && !var6 && !var9 && (2..=5).contains(&side) {
        true
    } else if side == 2 && var8 && !var6 && !var7 {
        true
    } else if side == 3 && var9 && !var6 && !var7 {
        true
    } else if side == 4 && var6 && !var8 && !var9 {
        true
    } else {
        side == 5 && var7 && !var8 && !var9
    }
}

/// `canProvidePower()`: `wiresProvidePower`.
// src: BlockRedstoneWire.java:335-337
pub fn can_provide_power(world: &World) -> bool {
    world.block_statics.redstone_wire_wires_provide_power
}

/// `randomDisplayTick(world, x, y, z, rand)`: a powered wire shows a red
/// dust particle tinted by its level (two `rand.nextFloat()` draws; client
/// only).
// src: BlockRedstoneWire.java:339-364
pub fn random_display_tick(world: &mut World, x: i32, y: i32, z: i32, rand: &mut JavaRandom) {
    let var6 = world.get_block_metadata(x, y, z);
    if var6 > 0 {
        let var7 = x as f64 + 0.5 + (rand.next_float() as f64 - 0.5) * 0.2;
        let var9 = (y as f32 + 0.0625) as f64;
        let var11 = z as f64 + 0.5 + (rand.next_float() as f64 - 0.5) * 0.2;
        let var13 = var6 as f32 / 15.0;
        let mut var14 = var13 * 0.6 + 0.4;
        if var6 == 0 {
            var14 = 0.0;
        }
        let mut var15 = var13 * var13 * 0.7 - 0.5;
        let mut var16 = var13 * var13 * 0.6 - 0.7;
        if var15 < 0.0 {
            var15 = 0.0;
        }
        if var16 < 0.0 {
            var16 = 0.0;
        }
        world.spawn_particle(
            "reddust",
            var7,
            var9,
            var11,
            var14 as f64,
            var15 as f64,
            var16 as f64,
        );
    }
}

/// `isPowerProviderOrWire(blockAccess, x, y, z, direction)` (static; also
/// read by the renderer): wire, any block whose `canProvidePower()` holds,
/// or a repeater with `footInvisibleFaceRemap[meta & 3] == direction`
/// (`isPoweringTo` passes 1, 3, 2, 0 for the -x, +x, -z, +z neighbours, so a
/// repeater connects along its axis, and -1 for the diagonal ones, which
/// never match).
// src: BlockRedstoneWire.java:366-380
pub fn is_power_provider_or_wire(
    world: &mut World,
    x: i32,
    y: i32,
    z: i32,
    direction: i32,
) -> bool {
    let var5 = world.get_block_id(x, y, z);
    if var5 == REDSTONE_WIRE {
        true
    } else if var5 == 0 {
        false
    } else if dispatch::can_provide_power(world, var5) {
        true
    } else if var5 != REDSTONE_REPEATER_IDLE && var5 != REDSTONE_REPEATER_ACTIVE {
        false
    } else {
        let var6 = world.get_block_metadata(x, y, z);
        direction == FOOT_INVISIBLE_FACE_REMAP[(var6 & 3) as usize]
    }
}
