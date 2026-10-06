//! `BlockSapling` (BlockSapling.java): a `BlockFlower` that grows into a
//! tree after two successful growth rolls.
//!
//! Fully transcribed: `updateTick`, `growTree` (also bone meal's target,
//! `ItemDye.onItemUse`, WP-2.6). The trees are [`crate::worldgen::trees`].
//! Textures and `damageDropped` are data in [`crate::block`].
//!
//! `growTree` by sapling type (`meta & 3`): 1 spruce (`WorldGenTaiga2`),
//! 2 birch (`WorldGenForest`), otherwise oak (`WorldGenTrees`, or
//! `WorldGenBigTree` one time in ten). No biome is consulted.
//!
//! Vanilla behaviour, transcribed: `updateTick` runs `BlockFlower.updateTick`
//! first, which may remove the sapling, and then carries on with the growth
//! roll at the now empty position.

use super::dispatch::RandomSource;
use super::flower;
use crate::world::World;
use crate::worldgen::biome::WorldGenTreeKind;
use crate::worldgen::trees::WorldGen;

/// `updateTick(world, x, y, z, rand)`: `BlockFlower.updateTick`, then in
/// light 9 or more one in 30: set the "ready" bit 8, or, when set, grow.
// src: BlockSapling.java:12-25
pub fn update_tick(world: &mut World, id: i32, x: i32, y: i32, z: i32, rand: RandomSource) {
    if world.multiplayer_world {
        return;
    }
    flower::update_tick(world, id, x, y, z);
    if world.get_block_light_value(x, y + 1, z) >= 9 && world.random(rand).next_int_bounded(30) == 0
    {
        let var6 = world.get_block_metadata(x, y, z);
        if (var6 & 8) == 0 {
            world.set_block_metadata_with_notify(x, y, z, var6 | 8);
        } else {
            grow_tree(world, id, x, y, z, rand);
        }
    }
}

/// `growTree(world, x, y, z, rand)`: removes the sapling (no notification),
/// runs the generator for its type, and puts the sapling back (type only)
/// when the tree does not fit.
// src: BlockSapling.java:36-55
pub fn grow_tree(world: &mut World, id: i32, x: i32, y: i32, z: i32, rand: RandomSource) {
    let var6 = world.get_block_metadata(x, y, z) & 3;
    world.set_block(x, y, z, 0);
    let mut var7 = if var6 == 1 {
        WorldGen::new(WorldGenTreeKind::Taiga2)
    } else if var6 == 2 {
        WorldGen::new(WorldGenTreeKind::Forest)
    } else {
        let mut g = WorldGen::new(WorldGenTreeKind::Trees);
        if world.random(rand).next_int_bounded(10) == 0 {
            g = WorldGen::new(WorldGenTreeKind::BigTree);
        }
        g
    };
    if !var7.generate(world, rand, x, y, z) {
        world.set_block_and_metadata(x, y, z, id, var6);
    }
}
