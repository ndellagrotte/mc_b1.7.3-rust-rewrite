//! `BlockDetectorRail` (BlockDetectorRail.java, id 28): a `BlockRail` with
//! `isPowered` true ([`super::rail`] has its shape and support rules) that
//! switches its metadata bit 3 on while an `EntityMinecart` is on it and
//! then powers its neighbours, strongly downward.
//!
//! Fully transcribed: `canProvidePower`, `onEntityCollidedWithBlock`,
//! `updateTick`, `isPoweringTo`, `isIndirectlyPoweringTo` and the private
//! `setStateIfMinecartInteractsWithRail`; `tickRate` (20) is data
//! ([`crate::block::tick_rate`]), as is the constructor's `setTickOnLoad`
//! (random ticks reach [`update_tick`]).
//!
//! `EntityMinecart` is WP-2.4, so no minecart can exist yet: the
//! `getEntitiesWithinAABB(EntityMinecart.class, …)` query runs in full and
//! finds nothing, which turns a powered detector rail off on its next
//! `updateTick`.

use mc_core::Aabb;

use crate::entity::Entity;
use crate::world::World;

/// `BlockDetectorRail.canProvidePower()`: true.
// src: BlockDetectorRail.java:16-18
pub const fn can_provide_power() -> bool {
    true
}

/// `BlockDetectorRail.onEntityCollidedWithBlock(world, x, y, z, entity)`:
/// any entity in the cell makes an unpowered detector rail look for a
/// minecart. `entity` is held out of its slot by `Entity.moveEntity`'s
/// caller, so the query gets it as the in-hand entity
/// ([`World::get_entities_within_aabb`]).
// src: BlockDetectorRail.java:20-27
pub fn on_entity_collided_with_block(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    entity: &Entity,
) {
    if !world.multiplayer_world {
        let var6 = world.get_block_metadata(x, y, z);
        if (var6 & 8) == 0 {
            set_state_if_minecart_interacts_with_rail(world, id, x, y, z, var6, Some(entity));
        }
    }
}

/// `BlockDetectorRail.updateTick(world, x, y, z, rand)`: a powered detector
/// rail checks again for a minecart (`rand` is unused). Reached from its own
/// scheduled tick and from random ticks (`setTickOnLoad(true)`,
/// BlockDetectorRail.java:9).
// src: BlockDetectorRail.java:29-36
pub fn update_tick(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    if !world.multiplayer_world {
        let var6 = world.get_block_metadata(x, y, z);
        if (var6 & 8) != 0 {
            set_state_if_minecart_interacts_with_rail(world, id, x, y, z, var6, None);
        }
    }
}

/// `BlockDetectorRail.isPoweringTo(blockAccess, x, y, z, side)`: weak power
/// to every side while bit 3 is set.
// src: BlockDetectorRail.java:38-40
pub fn is_powering_to(world: &mut World, x: i32, y: i32, z: i32, side: i32) -> bool {
    let _ = side;
    (world.get_block_metadata(x, y, z) & 8) != 0
}

/// `BlockDetectorRail.isIndirectlyPoweringTo(world, x, y, z, side)`: strong
/// power only into the block below (`side == 1`, the query from below) while
/// bit 3 is set.
// src: BlockDetectorRail.java:42-48
pub fn is_indirectly_powering_to(world: &mut World, x: i32, y: i32, z: i32, side: i32) -> bool {
    if (world.get_block_metadata(x, y, z) & 8) == 0 {
        false
    } else {
        side == 1
    }
}

/// `BlockDetectorRail.setStateIfMinecartInteractsWithRail(world, x, y, z,
/// meta)` (private): looks for minecarts in the lower quarter of the cell
/// inset by 1/8 on each side, switches bit 3 to match (notifying the
/// neighbours of the rail and of the block below), and while a minecart is
/// there checks again in `tickRate()` (20) ticks. `in_hand` is the entity Java
/// holds on the stack when the call comes from a collision.
// src: BlockDetectorRail.java:50-77
fn set_state_if_minecart_interacts_with_rail(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    var5: i32,
    in_hand: Option<&Entity>,
) {
    let var6 = (var5 & 8) != 0;
    let mut var7 = false;
    let var8 = 0.125f32;
    // src: BlockDetectorRail.java:54 (float sums widened to double, the y
    // bounds in double)
    let bb = Aabb::new(
        (x as f32 + var8) as f64,
        y as f64,
        (z as f32 + var8) as f64,
        ((x + 1) as f32 - var8) as f64,
        y as f64 + 0.25,
        ((z + 1) as f32 - var8) as f64,
    );
    let var9 = world.get_entities_within_aabb("EntityMinecart", &bb, in_hand);
    if !var9.is_empty() {
        var7 = true;
    }

    if var7 && !var6 {
        world.set_block_metadata_with_notify(x, y, z, var5 | 8);
        world.notify_blocks_of_neighbor_change(x, y, z, id);
        world.notify_blocks_of_neighbor_change(x, y - 1, z, id);
        world.mark_blocks_dirty(x, y, z, x, y, z);
    }

    if !var7 && var6 {
        world.set_block_metadata_with_notify(x, y, z, var5 & 7);
        world.notify_blocks_of_neighbor_change(x, y, z, id);
        world.notify_blocks_of_neighbor_change(x, y - 1, z, id);
        world.mark_blocks_dirty(x, y, z, x, y, z);
    }

    // src: BlockDetectorRail.java:73-75 (`this.tickRate()`, 20,
    // BlockDetectorRail.java:12-14)
    if var7 {
        world.schedule_block_update(x, y, z, id, super::dispatch::tick_rate(id));
    }
}
