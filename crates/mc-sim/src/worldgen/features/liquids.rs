//! `WorldGenLiquids` (springs in stone walls) and `WorldGenHellLava` (lava
//! springs in netherrack).
//!
//! Both place the flowing liquid with `setBlockWithNotify`, then run its
//! `updateTick` once with `World.scheduledUpdatesAreImmediate` set, passing
//! their own `Random` (the generator's, [`RandomSource::ChunkProviderRand`]
//! in population): with the flag set every `scheduleBlockUpdate` of the
//! spread runs its `updateTick` at once with `World.rand`
//! (World.java:1152-1163), so a whole spring flows out inside population.
//! Only the first `updateTick` draws from the generator's `Random` (lava's
//! 3-in-4 stall, BlockFlowing.java:59-62).

use crate::block::dispatch::{self, RandomSource};
use crate::block::ids;
use crate::world::World;

/// `WorldGenLiquids`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldGenLiquids {
    /// `liquidBlockId`: `Block.waterMoving` or `Block.lavaMoving`.
    // src: WorldGenLiquids.java:6
    pub liquid_block_id: i32,
}

impl WorldGenLiquids {
    /// `new WorldGenLiquids(id)`.
    // src: WorldGenLiquids.java:8-10
    pub fn new(liquid_block_id: i32) -> Self {
        WorldGenLiquids { liquid_block_id }
    }

    /// `generate(world, rand, x, y, z)`: stone above and below, air or stone
    /// at the spot, then a spring if exactly three horizontal neighbours are
    /// stone and one is air. False only for the first three tests.
    // src: WorldGenLiquids.java:12-63
    pub fn generate(&self, world: &mut World, rand: RandomSource, x: i32, y: i32, z: i32) -> bool {
        if world.get_block_id(x, y + 1, z) != ids::STONE {
            return false;
        }
        if world.get_block_id(x, y - 1, z) != ids::STONE {
            return false;
        }
        if world.get_block_id(x, y, z) != 0 && world.get_block_id(x, y, z) != ids::STONE {
            return false;
        }
        let mut var6 = 0;
        if world.get_block_id(x - 1, y, z) == ids::STONE {
            var6 += 1;
        }
        if world.get_block_id(x + 1, y, z) == ids::STONE {
            var6 += 1;
        }
        if world.get_block_id(x, y, z - 1) == ids::STONE {
            var6 += 1;
        }
        if world.get_block_id(x, y, z + 1) == ids::STONE {
            var6 += 1;
        }
        let mut var7 = 0;
        if world.is_air_block(x - 1, y, z) {
            var7 += 1;
        }
        if world.is_air_block(x + 1, y, z) {
            var7 += 1;
        }
        if world.is_air_block(x, y, z - 1) {
            var7 += 1;
        }
        if world.is_air_block(x, y, z + 1) {
            var7 += 1;
        }
        if var6 == 3 && var7 == 1 {
            spring(world, rand, self.liquid_block_id, x, y, z);
        }
        true
    }
}

/// `WorldGenHellLava`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldGenHellLava {
    /// `field_4158_a`: the liquid's id (`Block.lavaMoving`).
    // src: WorldGenHellLava.java:6
    pub field_4158_a: i32,
}

impl WorldGenHellLava {
    /// `new WorldGenHellLava(id)`.
    // src: WorldGenHellLava.java:8-10
    pub fn new(field_4158_a: i32) -> Self {
        WorldGenHellLava { field_4158_a }
    }

    /// `generate(world, rand, x, y, z)`: netherrack above, air or netherrack
    /// at the spot, then a spring if exactly four of the five neighbours
    /// (four horizontal and below) are netherrack and one is air.
    // src: WorldGenHellLava.java:12-69
    pub fn generate(&self, world: &mut World, rand: RandomSource, x: i32, y: i32, z: i32) -> bool {
        if world.get_block_id(x, y + 1, z) != ids::NETHERRACK {
            return false;
        }
        if world.get_block_id(x, y, z) != 0 && world.get_block_id(x, y, z) != ids::NETHERRACK {
            return false;
        }
        let mut var6 = 0;
        if world.get_block_id(x - 1, y, z) == ids::NETHERRACK {
            var6 += 1;
        }
        if world.get_block_id(x + 1, y, z) == ids::NETHERRACK {
            var6 += 1;
        }
        if world.get_block_id(x, y, z - 1) == ids::NETHERRACK {
            var6 += 1;
        }
        if world.get_block_id(x, y, z + 1) == ids::NETHERRACK {
            var6 += 1;
        }
        if world.get_block_id(x, y - 1, z) == ids::NETHERRACK {
            var6 += 1;
        }
        let mut var7 = 0;
        if world.is_air_block(x - 1, y, z) {
            var7 += 1;
        }
        if world.is_air_block(x + 1, y, z) {
            var7 += 1;
        }
        if world.is_air_block(x, y, z - 1) {
            var7 += 1;
        }
        if world.is_air_block(x, y, z + 1) {
            var7 += 1;
        }
        if world.is_air_block(x, y - 1, z) {
            var7 += 1;
        }
        if var6 == 4 && var7 == 1 {
            spring(world, rand, self.field_4158_a, x, y, z);
        }
        true
    }
}

/// The spring both generators make: place, then one immediate `updateTick`.
// src: WorldGenLiquids.java:55-58, WorldGenHellLava.java:61-64
fn spring(world: &mut World, rand: RandomSource, id: i32, x: i32, y: i32, z: i32) {
    world.set_block_with_notify(x, y, z, id);
    world.scheduled_updates_are_immediate = true;
    dispatch::update_tick(world, id, x, y, z, rand);
    world.scheduled_updates_are_immediate = false;
}
