//! `BlockFire` (BlockFire.java): fire burning out, spreading to flammable
//! blocks and into the air next to them, and its client particles.
//!
//! Fully transcribed: `updateTick`, `tryToCatchBlockOnFire`,
//! `canNeighborCatchFire`, `getChanceOfNeighborsEncouragingFire`,
//! `canBlockCatchFire`, `getChanceToEncourageFire`, `canPlaceBlockAt`,
//! `onNeighborBlockChange`, `onBlockAdded`, `randomDisplayTick`,
//! `getCollisionBoundingBoxFromPool` (none). The burn tables that
//! `initializeBlock` fills are data ([`super::CHANCE_TO_ENCOURAGE_FIRE`],
//! [`super::ABILITY_TO_CATCH_FIRE`]); the other data methods (`tickRate` 40,
//! `isCollidable`, `quantityDropped`, …) are in [`crate::block`].
//!
//! Gaps: `onBlockAdded` on obsidian asks `Block.portal.tryToCreatePortal`
//! (BlockFire.java:192), which is `BlockPortal`'s (the interactables task of
//! WP-1.6) and has no dispatch entry point (it is not a `Block` method): the
//! call records `"BlockPortal.tryToCreatePortal"` and answers false, so the
//! fire takes the no-portal path. Burning TNT calls
//! `Block.tnt.onBlockDestroyedByPlayer` through the dispatch, which records
//! until the TNT task lands.
//!
//! Vanilla bugs, transcribed: the spread loop tests rain at
//! `(x - 1, y', z)` with the *fire's* `z` instead of the target's
//! (BlockFire.java:97); `updateTick` keeps going after it removed the fire
//! because `canPlaceBlockAt` failed (BlockFire.java:57-59).

use mc_core::JavaRandom;

use super::dispatch::{self, RandomSource};
use super::{ABILITY_TO_CATCH_FIRE, CHANCE_TO_ENCOURAGE_FIRE};
use crate::world::World;

/// `Block.netherrack.blockID` (Block.java:679).
const NETHERRACK: i32 = 87;
/// `Block.tnt.blockID` (Block.java:638).
const TNT: i32 = 46;
/// `Block.obsidian.blockID` (Block.java:641).
const OBSIDIAN: i32 = 49;

/// `tickRate()`.
// src: BlockFire.java:51-53
const TICK_RATE: i32 = 40;

/// `getCollisionBoundingBoxFromPool`: fire has no collision box.
// src: BlockFire.java:31-33
pub fn get_collision_bounding_box_from_pool() -> Option<mc_core::Aabb> {
    None
}

/// `updateTick(world, x, y, z, rand)`.
// src: BlockFire.java:55-113
pub fn update_tick(world: &mut World, id: i32, x: i32, y: i32, z: i32, rand: RandomSource) {
    let var6 = world.get_block_id(x, y - 1, z) == NETHERRACK;
    if !can_place_block_at(world, x, y, z) {
        // vanilla bug (transcribed): the method does not return here.
        world.set_block_with_notify(x, y, z, 0);
    }
    if !var6
        && world.is_raining()
        && (world.can_block_be_rained_on(x, y, z)
            || world.can_block_be_rained_on(x - 1, y, z)
            || world.can_block_be_rained_on(x + 1, y, z)
            || world.can_block_be_rained_on(x, y, z - 1)
            || world.can_block_be_rained_on(x, y, z + 1))
    {
        world.set_block_with_notify(x, y, z, 0);
        return;
    }
    let var7 = world.get_block_metadata(x, y, z);
    if var7 < 15 {
        let d = world.random(rand).next_int_bounded(3) / 2;
        world.set_block_metadata(x, y, z, var7 + d);
    }
    world.schedule_block_update(x, y, z, id, TICK_RATE);
    if !var6 && !can_neighbor_catch_fire(world, x, y, z) {
        if !world.is_block_normal_cube(x, y - 1, z) || var7 > 3 {
            world.set_block_with_notify(x, y, z, 0);
        }
    } else if !var6
        && !can_block_catch_fire(world, x, y - 1, z)
        && var7 == 15
        && world.random(rand).next_int_bounded(4) == 0
    {
        world.set_block_with_notify(x, y, z, 0);
    } else {
        // src: BlockFire.java:78-83 (+x, -x, -y, +y, -z, +z)
        try_to_catch_block_on_fire(world, id, x + 1, y, z, 300, rand, var7);
        try_to_catch_block_on_fire(world, id, x - 1, y, z, 300, rand, var7);
        try_to_catch_block_on_fire(world, id, x, y - 1, z, 250, rand, var7);
        try_to_catch_block_on_fire(world, id, x, y + 1, z, 250, rand, var7);
        try_to_catch_block_on_fire(world, id, x, y, z - 1, 300, rand, var7);
        try_to_catch_block_on_fire(world, id, x, y, z + 1, 300, rand, var7);

        // src: BlockFire.java:85-109 (x outer, z middle, y inner, y up to +4)
        for var8 in (x - 1)..=(x + 1) {
            for var9 in (z - 1)..=(z + 1) {
                for var10 in (y - 1)..=(y + 4) {
                    if var8 != x || var10 != y || var9 != z {
                        let mut var11 = 100;
                        if var10 > y + 1 {
                            var11 += (var10 - (y + 1)) * 100;
                        }
                        let var12 =
                            get_chance_of_neighbors_encouraging_fire(world, var8, var10, var9);
                        if var12 > 0 {
                            let var13 = (var12 + 40) / (var7 + 30);
                            // vanilla bug (transcribed): the second rain test
                            // uses the fire's z (`var4`) where the target's
                            // (`var9`) is meant (BlockFire.java:97).
                            if var13 > 0
                                && world.random(rand).next_int_bounded(var11) <= var13
                                && (!world.is_raining()
                                    || !world.can_block_be_rained_on(var8, var10, var9))
                                && !world.can_block_be_rained_on(var8 - 1, var10, z)
                                && !world.can_block_be_rained_on(var8 + 1, var10, var9)
                                && !world.can_block_be_rained_on(var8, var10, var9 - 1)
                                && !world.can_block_be_rained_on(var8, var10, var9 + 1)
                            {
                                let mut var14 = var7 + world.random(rand).next_int_bounded(5) / 4;
                                if var14 > 15 {
                                    var14 = 15;
                                }
                                world.set_block_and_metadata_with_notify(
                                    var8, var10, var9, id, var14,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

/// `tryToCatchBlockOnFire(world, x, y, z, chance, rand, age)` (private): a
/// neighbour burns with probability `abilityToCatchFire / chance`, and is
/// replaced by fire (one in `age + 10` times five, not under rain) or air.
// src: BlockFire.java:115-135
#[allow(clippy::too_many_arguments)]
fn try_to_catch_block_on_fire(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    var5: i32,
    rand: RandomSource,
    var7: i32,
) {
    let var8 = ABILITY_TO_CATCH_FIRE[world.get_block_id(x, y, z) as usize];
    if world.random(rand).next_int_bounded(var5) < var8 {
        let var9 = world.get_block_id(x, y, z) == TNT;
        if world.random(rand).next_int_bounded(var7 + 10) < 5
            && !world.can_block_be_rained_on(x, y, z)
        {
            let mut var10 = var7 + world.random(rand).next_int_bounded(5) / 4;
            if var10 > 15 {
                var10 = 15;
            }
            world.set_block_and_metadata_with_notify(x, y, z, id, var10);
        } else {
            world.set_block_with_notify(x, y, z, 0);
        }
        if var9 {
            // src: BlockFire.java:131 (Block.tnt.onBlockDestroyedByPlayer(world, x, y, z, 1))
            dispatch::on_block_destroyed_by_player(world, TNT, x, y, z, 1);
        }
    }
}

/// `canNeighborCatchFire(world, x, y, z)` (private): any of the six
/// neighbours (+x, -x, -y, +y, -z, +z) can catch fire.
// src: BlockFire.java:137-151
fn can_neighbor_catch_fire(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    can_block_catch_fire(world, x + 1, y, z)
        || can_block_catch_fire(world, x - 1, y, z)
        || can_block_catch_fire(world, x, y - 1, z)
        || can_block_catch_fire(world, x, y + 1, z)
        || can_block_catch_fire(world, x, y, z - 1)
        || can_block_catch_fire(world, x, y, z + 1)
}

/// `getChanceOfNeighborsEncouragingFire(world, x, y, z)` (private): 0 unless
/// the block is air, else the largest `chanceToEncourageFire` of its six
/// neighbours.
// src: BlockFire.java:153-166
fn get_chance_of_neighbors_encouraging_fire(world: &mut World, x: i32, y: i32, z: i32) -> i32 {
    let var5 = 0;
    if !world.is_air_block(x, y, z) {
        return 0;
    }
    let mut var6 = get_chance_to_encourage_fire(world, x + 1, y, z, var5);
    var6 = get_chance_to_encourage_fire(world, x - 1, y, z, var6);
    var6 = get_chance_to_encourage_fire(world, x, y - 1, z, var6);
    var6 = get_chance_to_encourage_fire(world, x, y + 1, z, var6);
    var6 = get_chance_to_encourage_fire(world, x, y, z - 1, var6);
    get_chance_to_encourage_fire(world, x, y, z + 1, var6)
}

/// `canBlockCatchFire(blockAccess, x, y, z)`: the block there has a
/// `chanceToEncourageFire`.
// src: BlockFire.java:172-174
pub fn can_block_catch_fire(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    CHANCE_TO_ENCOURAGE_FIRE[world.get_block_id(x, y, z) as usize] > 0
}

/// `getChanceToEncourageFire(world, x, y, z, current)`: the larger of
/// `current` and the block's `chanceToEncourageFire`.
// src: BlockFire.java:176-179
pub fn get_chance_to_encourage_fire(world: &mut World, x: i32, y: i32, z: i32, var5: i32) -> i32 {
    let var6 = CHANCE_TO_ENCOURAGE_FIRE[world.get_block_id(x, y, z) as usize];
    if var6 > var5 { var6 } else { var5 }
}

/// `canPlaceBlockAt(world, x, y, z)`: on a normal cube, or next to
/// something flammable.
// src: BlockFire.java:181-183
pub fn can_place_block_at(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    world.is_block_normal_cube(x, y - 1, z) || can_neighbor_catch_fire(world, x, y, z)
}

/// `onNeighborBlockChange(world, x, y, z, neighborId)`.
// src: BlockFire.java:185-189
pub fn on_neighbor_block_change(world: &mut World, x: i32, y: i32, z: i32) {
    if !world.is_block_normal_cube(x, y - 1, z) && !can_neighbor_catch_fire(world, x, y, z) {
        world.set_block_with_notify(x, y, z, 0);
    }
}

/// `onBlockAdded(world, x, y, z)`: on obsidian, first try to light a portal
/// (a recorded gap, module docs); otherwise go out at once or schedule the
/// first tick.
// src: BlockFire.java:191-199
pub fn on_block_added(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    if world.get_block_id(x, y - 1, z) != OBSIDIAN || !try_to_create_portal(world, x, y, z) {
        if !world.is_block_normal_cube(x, y - 1, z) && !can_neighbor_catch_fire(world, x, y, z) {
            world.set_block_with_notify(x, y, z, 0);
        } else {
            world.schedule_block_update(x, y, z, id, TICK_RATE);
        }
    }
}

/// `Block.portal.tryToCreatePortal(world, x, y, z)` (BlockPortal.java:37):
/// not transcribed here (module docs); recorded, answers false.
fn try_to_create_portal(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    let _ = (x, y, z);
    world.note_unimplemented("BlockPortal.tryToCreatePortal");
    false
}

/// `randomDisplayTick(world, x, y, z, rand)`: the crackle sound and smoke
/// (client effects; `rand` is `randomDisplayUpdates`' own `Random`).
// src: BlockFire.java:201-264
pub fn random_display_tick(world: &mut World, x: i32, y: i32, z: i32, rand: &mut JavaRandom) {
    if rand.next_int_bounded(24) == 0 {
        // src: BlockFire.java:203 (volume, then pitch, drawn in argument order)
        let volume = 1.0f32 + rand.next_float();
        let pitch = rand.next_float() * 0.7f32 + 0.3f32;
        world.play_sound_effect(
            (x as f32 + 0.5f32) as f64,
            (y as f32 + 0.5f32) as f64,
            (z as f32 + 0.5f32) as f64,
            "fire.fire",
            volume,
            pitch,
        );
    }
    let smoke = |world: &mut World, a: f32, b: f32, c: f32| {
        world.spawn_particle("largesmoke", a as f64, b as f64, c as f64, 0.0, 0.0, 0.0);
    };
    // `Block.fire.canBlockCatchFire` is this block's own method.
    if !world.is_block_normal_cube(x, y - 1, z) && !can_block_catch_fire(world, x, y - 1, z) {
        if can_block_catch_fire(world, x - 1, y, z) {
            for _ in 0..2 {
                let var7 = x as f32 + rand.next_float() * 0.1f32;
                let var8 = y as f32 + rand.next_float();
                let var9 = z as f32 + rand.next_float();
                smoke(world, var7, var8, var9);
            }
        }
        if can_block_catch_fire(world, x + 1, y, z) {
            for _ in 0..2 {
                let var7 = (x + 1) as f32 - rand.next_float() * 0.1f32;
                let var8 = y as f32 + rand.next_float();
                let var9 = z as f32 + rand.next_float();
                smoke(world, var7, var8, var9);
            }
        }
        if can_block_catch_fire(world, x, y, z - 1) {
            for _ in 0..2 {
                let var7 = x as f32 + rand.next_float();
                let var8 = y as f32 + rand.next_float();
                let var9 = z as f32 + rand.next_float() * 0.1f32;
                smoke(world, var7, var8, var9);
            }
        }
        if can_block_catch_fire(world, x, y, z + 1) {
            for _ in 0..2 {
                let var7 = x as f32 + rand.next_float();
                let var8 = y as f32 + rand.next_float();
                let var9 = (z + 1) as f32 - rand.next_float() * 0.1f32;
                smoke(world, var7, var8, var9);
            }
        }
        if can_block_catch_fire(world, x, y + 1, z) {
            for _ in 0..2 {
                let var7 = x as f32 + rand.next_float();
                let var8 = (y + 1) as f32 - rand.next_float() * 0.1f32;
                let var9 = z as f32 + rand.next_float();
                smoke(world, var7, var8, var9);
            }
        }
    } else {
        for _ in 0..3 {
            let var7 = x as f32 + rand.next_float();
            let var8 = y as f32 + rand.next_float() * 0.5f32 + 0.5f32;
            let var9 = z as f32 + rand.next_float();
            smoke(world, var7, var8, var9);
        }
    }
}
