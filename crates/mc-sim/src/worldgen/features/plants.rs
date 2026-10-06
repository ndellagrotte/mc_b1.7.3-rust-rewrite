//! The scatter generators: `WorldGenFlowers`, `WorldGenTallGrass`,
//! `WorldGenDeadBush`, `WorldGenPumpkin`, `WorldGenReed`, `WorldGenCactus`.
//! Each tries a fixed number of random positions around the given one
//! (offsets `nextInt(n) - nextInt(n)`, x then y then z) and places where the
//! block may stay. They return true whatever they placed.
//!
//! The "may stay" tests are the block's own virtual methods through
//! [`dispatch`]: `((BlockFlower)Block.blocksList[id]).canBlockStay` is
//! `BlockMushroom.canBlockStay` for mushrooms (BlockMushroom.java:33-39) and
//! `BlockFlower.canBlockStay` with the class's `canThisPlantGrowOnThisBlockID`
//! otherwise.

use super::next_int;
use crate::block::dispatch::{self, RandomSource};
use crate::block::{Material, ids};
use crate::world::World;

/// The descent shared by `WorldGenTallGrass` and `WorldGenDeadBush`: down
/// through air and leaves while `y > 0` (the block is read before `y` is
/// tested).
// src: WorldGenTallGrass.java:16-17, WorldGenDeadBush.java:14-15
fn descend_through_air_and_leaves(world: &mut World, x: i32, mut y: i32, z: i32) -> i32 {
    loop {
        let var11 = world.get_block_id(x, y, z);
        if !((var11 == 0 || var11 == ids::LEAVES) && y > 0) {
            return y;
        }
        y -= 1;
    }
}

/// `WorldGenFlowers`: 64 tries of `setBlock` (dandelions, roses and
/// mushrooms in population).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldGenFlowers {
    /// `plantBlockId`.
    // src: WorldGenFlowers.java:6
    pub plant_block_id: i32,
}

impl WorldGenFlowers {
    /// `new WorldGenFlowers(id)`.
    // src: WorldGenFlowers.java:8-10
    pub fn new(plant_block_id: i32) -> Self {
        WorldGenFlowers { plant_block_id }
    }

    /// `generate(world, rand, x, y, z)`.
    // src: WorldGenFlowers.java:12-23
    pub fn generate(&self, world: &mut World, rand: RandomSource, x: i32, y: i32, z: i32) -> bool {
        for _var6 in 0..64 {
            let var7 = x + next_int(world, rand, 8) - next_int(world, rand, 8);
            let var8 = y + next_int(world, rand, 4) - next_int(world, rand, 4);
            let var9 = z + next_int(world, rand, 8) - next_int(world, rand, 8);
            if world.is_air_block(var7, var8, var9)
                && dispatch::can_block_stay(world, self.plant_block_id, var7, var8, var9)
            {
                world.set_block(var7, var8, var9, self.plant_block_id);
            }
        }
        true
    }
}

/// `WorldGenTallGrass`: from the first non-air, non-leaves block down, 128
/// tries of `setBlockAndMetadata` (the metadata picks grass or fern).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldGenTallGrass {
    /// `field_28060_a`: the block id (`Block.tallGrass`).
    // src: WorldGenTallGrass.java:6
    pub field_28060_a: i32,
    /// `field_28059_b`: the metadata (1 grass, 2 fern).
    // src: WorldGenTallGrass.java:7
    pub field_28059_b: i32,
}

impl WorldGenTallGrass {
    /// `new WorldGenTallGrass(id, meta)`.
    // src: WorldGenTallGrass.java:9-12
    pub fn new(field_28060_a: i32, field_28059_b: i32) -> Self {
        WorldGenTallGrass {
            field_28060_a,
            field_28059_b,
        }
    }

    /// `generate(world, rand, x, y, z)`.
    // src: WorldGenTallGrass.java:14-29
    pub fn generate(&self, world: &mut World, rand: RandomSource, x: i32, y: i32, z: i32) -> bool {
        let y = descend_through_air_and_leaves(world, x, y, z);
        for _var7 in 0..128 {
            let var8 = x + next_int(world, rand, 8) - next_int(world, rand, 8);
            let var9 = y + next_int(world, rand, 4) - next_int(world, rand, 4);
            let var10 = z + next_int(world, rand, 8) - next_int(world, rand, 8);
            if world.is_air_block(var8, var9, var10)
                && dispatch::can_block_stay(world, self.field_28060_a, var8, var9, var10)
            {
                world.set_block_and_metadata(
                    var8,
                    var9,
                    var10,
                    self.field_28060_a,
                    self.field_28059_b,
                );
            }
        }
        true
    }
}

/// `WorldGenDeadBush`: the same descent, 4 tries of `setBlock`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldGenDeadBush {
    /// `field_28058_a`: the block id (`Block.deadBush`).
    // src: WorldGenDeadBush.java:6
    pub field_28058_a: i32,
}

impl WorldGenDeadBush {
    /// `new WorldGenDeadBush(id)`.
    // src: WorldGenDeadBush.java:8-10
    pub fn new(field_28058_a: i32) -> Self {
        WorldGenDeadBush { field_28058_a }
    }

    /// `generate(world, rand, x, y, z)`.
    // src: WorldGenDeadBush.java:12-27
    pub fn generate(&self, world: &mut World, rand: RandomSource, x: i32, y: i32, z: i32) -> bool {
        let y = descend_through_air_and_leaves(world, x, y, z);
        for _var7 in 0..4 {
            let var8 = x + next_int(world, rand, 8) - next_int(world, rand, 8);
            let var9 = y + next_int(world, rand, 4) - next_int(world, rand, 4);
            let var10 = z + next_int(world, rand, 8) - next_int(world, rand, 8);
            if world.is_air_block(var8, var9, var10)
                && dispatch::can_block_stay(world, self.field_28058_a, var8, var9, var10)
            {
                world.set_block(var8, var9, var10, self.field_28058_a);
            }
        }
        true
    }
}

/// `WorldGenPumpkin`: 64 tries on grass, facing `nextInt(4)` (drawn only
/// when placing).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorldGenPumpkin;

impl WorldGenPumpkin {
    /// `generate(world, rand, x, y, z)`.
    // src: WorldGenPumpkin.java:6-17
    pub fn generate(&self, world: &mut World, rand: RandomSource, x: i32, y: i32, z: i32) -> bool {
        for _var6 in 0..64 {
            let var7 = x + next_int(world, rand, 8) - next_int(world, rand, 8);
            let var8 = y + next_int(world, rand, 4) - next_int(world, rand, 4);
            let var9 = z + next_int(world, rand, 8) - next_int(world, rand, 8);
            if world.is_air_block(var7, var8, var9)
                && world.get_block_id(var7, var8 - 1, var9) == ids::GRASS
                && dispatch::can_place_block_at(world, ids::PUMPKIN, var7, var8, var9)
            {
                let meta = next_int(world, rand, 4);
                world.set_block_and_metadata(var7, var8, var9, ids::PUMPKIN, meta);
            }
        }
        true
    }
}

/// `WorldGenReed`: 20 tries at the given height next to water; a column of
/// `2 + nextInt(nextInt(3) + 1)` reeds, each placed only if it may stay.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorldGenReed;

impl WorldGenReed {
    /// `generate(world, rand, x, y, z)`.
    // src: WorldGenReed.java:6-23
    pub fn generate(&self, world: &mut World, rand: RandomSource, x: i32, y: i32, z: i32) -> bool {
        for _var6 in 0..20 {
            let var7 = x + next_int(world, rand, 4) - next_int(world, rand, 4);
            let var8 = y;
            let var9 = z + next_int(world, rand, 4) - next_int(world, rand, 4);
            if world.is_air_block(var7, y, var9)
                && (world.get_block_material(var7 - 1, y - 1, var9) == Material::Water
                    || world.get_block_material(var7 + 1, y - 1, var9) == Material::Water
                    || world.get_block_material(var7, y - 1, var9 - 1) == Material::Water
                    || world.get_block_material(var7, y - 1, var9 + 1) == Material::Water)
            {
                // `2 + rand.nextInt(rand.nextInt(3) + 1)`: the bound first.
                let bound = next_int(world, rand, 3) + 1;
                let var10 = 2 + next_int(world, rand, bound);
                for var11 in 0..var10 {
                    if dispatch::can_block_stay(world, ids::REED, var7, var8 + var11, var9) {
                        world.set_block(var7, var8 + var11, var9, ids::REED);
                    }
                }
            }
        }
        true
    }
}

/// `WorldGenCactus`: 10 tries; a column of `1 + nextInt(nextInt(3) + 1)`
/// cacti, each placed only if it may stay.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorldGenCactus;

impl WorldGenCactus {
    /// `generate(world, rand, x, y, z)`.
    // src: WorldGenCactus.java:6-23
    pub fn generate(&self, world: &mut World, rand: RandomSource, x: i32, y: i32, z: i32) -> bool {
        for _var6 in 0..10 {
            let var7 = x + next_int(world, rand, 8) - next_int(world, rand, 8);
            let var8 = y + next_int(world, rand, 4) - next_int(world, rand, 4);
            let var9 = z + next_int(world, rand, 8) - next_int(world, rand, 8);
            if world.is_air_block(var7, var8, var9) {
                // `1 + rand.nextInt(rand.nextInt(3) + 1)`: the bound first.
                let bound = next_int(world, rand, 3) + 1;
                let var10 = 1 + next_int(world, rand, bound);
                for var11 in 0..var10 {
                    if dispatch::can_block_stay(world, ids::CACTUS, var7, var8 + var11, var9) {
                        world.set_block(var7, var8 + var11, var9, ids::CACTUS);
                    }
                }
            }
        }
        true
    }
}
