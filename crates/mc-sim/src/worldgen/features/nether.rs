//! The nether-only features: `WorldGenFire` (fire on netherrack) and the two
//! glowstone generators `WorldGenGlowStone1` / `WorldGenGlowStone2`, which
//! are the same code under two class names (population runs both,
//! ChunkProviderHell.java:334, :341).

use super::next_int;
use crate::block::dispatch::RandomSource;
use crate::block::ids;
use crate::world::World;

/// `WorldGenFire`: 64 tries of `setBlockWithNotify(fire)` on netherrack.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorldGenFire;

impl WorldGenFire {
    /// `generate(world, rand, x, y, z)`.
    // src: WorldGenFire.java:6-17
    pub fn generate(&self, world: &mut World, rand: RandomSource, x: i32, y: i32, z: i32) -> bool {
        for _var6 in 0..64 {
            let var7 = x + next_int(world, rand, 8) - next_int(world, rand, 8);
            let var8 = y + next_int(world, rand, 4) - next_int(world, rand, 4);
            let var9 = z + next_int(world, rand, 8) - next_int(world, rand, 8);
            if world.is_air_block(var7, var8, var9)
                && world.get_block_id(var7, var8 - 1, var9) == ids::NETHERRACK
            {
                world.set_block_with_notify(var7, var8, var9, ids::FIRE);
            }
        }
        true
    }
}

/// Which of the two identical glowstone classes (the class name is the
/// only difference, WorldGenGlowStone1.java vs WorldGenGlowStone2.java).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlowStoneClass {
    /// `WorldGenGlowStone1`.
    GlowStone1,
    /// `WorldGenGlowStone2`.
    GlowStone2,
}

/// `WorldGenGlowStone1` / `WorldGenGlowStone2`: a glowstone cluster hanging
/// from netherrack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldGenGlowStone {
    /// The Java class this instance stands for.
    pub class: GlowStoneClass,
}

impl WorldGenGlowStone {
    /// `new WorldGenGlowStone1()` / `new WorldGenGlowStone2()`.
    pub fn new(class: GlowStoneClass) -> Self {
        WorldGenGlowStone { class }
    }

    /// `generate(world, rand, x, y, z)`: needs air with netherrack above;
    /// places glowstone there, then 1500 tries below (down to 11 blocks) of
    /// air with exactly one glowstone face neighbour. The six neighbours are
    /// read in the order −x, +x, −y, +y, −z, +z.
    // src: WorldGenGlowStone1.java:6-60, WorldGenGlowStone2.java:6-60
    pub fn generate(&self, world: &mut World, rand: RandomSource, x: i32, y: i32, z: i32) -> bool {
        if !world.is_air_block(x, y, z) {
            return false;
        }
        if world.get_block_id(x, y + 1, z) != ids::NETHERRACK {
            return false;
        }
        world.set_block_with_notify(x, y, z, ids::GLOW_STONE);
        for _var6 in 0..1500 {
            let var7 = x + next_int(world, rand, 8) - next_int(world, rand, 8);
            let var8 = y - next_int(world, rand, 12);
            let var9 = z + next_int(world, rand, 8) - next_int(world, rand, 8);
            if world.get_block_id(var7, var8, var9) == 0 {
                let mut var10 = 0;
                for var11 in 0..6 {
                    let var12 = match var11 {
                        0 => world.get_block_id(var7 - 1, var8, var9),
                        1 => world.get_block_id(var7 + 1, var8, var9),
                        2 => world.get_block_id(var7, var8 - 1, var9),
                        3 => world.get_block_id(var7, var8 + 1, var9),
                        4 => world.get_block_id(var7, var8, var9 - 1),
                        _ => world.get_block_id(var7, var8, var9 + 1),
                    };
                    if var12 == ids::GLOW_STONE {
                        var10 += 1;
                    }
                }
                if var10 == 1 {
                    world.set_block_with_notify(var7, var8, var9, ids::GLOW_STONE);
                }
            }
        }
        true
    }
}
