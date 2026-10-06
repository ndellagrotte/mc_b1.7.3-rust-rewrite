//! `WorldGenLakes` (WorldGenLakes.java): a water or lava lake, the union of
//! 4–7 ellipsoids in a 16×8×16 box whose corner is 8 blocks west and north
//! of the given position and 4 below the first non-air block under it.

use super::{next_double, next_int};
use crate::block::dispatch::RandomSource;
use crate::block::{self, Material, ids};
use crate::world::World;
use crate::world::lighting::EnumSkyBlock;

/// `WorldGenLakes`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldGenLakes {
    /// `field_15235_a`: the liquid's block id (`Block.waterStill` or
    /// `Block.lavaStill` in population).
    // src: WorldGenLakes.java:6
    pub field_15235_a: i32,
}

/// The index of `(x, y, z)` in the 16×16×8 shape array (`(x * 16 + z) * 8 + y`).
fn idx(x: i32, z: i32, y: i32) -> usize {
    ((x * 16 + z) * 8 + y) as usize
}

/// The border test of the two border passes: not in the lake, and a face
/// neighbour (+x, −x, +z, −z, +y, −y in that order, with Java's
/// short-circuit) is.
// src: WorldGenLakes.java:52, :91
fn is_border(var6: &[bool; 2048], var8: i32, var32: i32, var10: i32) -> bool {
    !var6[idx(var8, var32, var10)]
        && (var8 < 15 && var6[idx(var8 + 1, var32, var10)]
            || var8 > 0 && var6[idx(var8 - 1, var32, var10)]
            || var32 < 15 && var6[idx(var8, var32 + 1, var10)]
            || var32 > 0 && var6[idx(var8, var32 - 1, var10)]
            || var10 < 7 && var6[idx(var8, var32, var10 + 1)]
            || var10 > 0 && var6[idx(var8, var32, var10 - 1)])
}

impl WorldGenLakes {
    /// `new WorldGenLakes(id)`.
    // src: WorldGenLakes.java:8-10
    pub fn new(field_15235_a: i32) -> Self {
        WorldGenLakes { field_15235_a }
    }

    /// `generate(world, rand, x, y, z)`: false when the border has liquid in
    /// its upper half or a non-solid block other than the lake's liquid in
    /// its lower half; otherwise carves the lake (liquid in the lower 4
    /// layers, air above), turns dirt under open air into grass and, for a
    /// lava lake, lines the border with stone.
    // src: WorldGenLakes.java:12-101
    pub fn generate(
        &self,
        world: &mut World,
        rand: RandomSource,
        mut x: i32,
        mut y: i32,
        mut z: i32,
    ) -> bool {
        x -= 8;
        z -= 8;
        while y > 0 && world.is_air_block(x, y, z) {
            y -= 1;
        }
        y -= 4;

        // src: WorldGenLakes.java:19-44 (the shape: x 1..15, z 1..15, y 1..7)
        let mut var6 = [false; 2048];
        let var7 = next_int(world, rand, 4) + 4;
        for _var8 in 0..var7 {
            let var9 = next_double(world, rand) * 6.0 + 3.0;
            let var11 = next_double(world, rand) * 4.0 + 2.0;
            let var13 = next_double(world, rand) * 6.0 + 3.0;
            let var15 = next_double(world, rand) * (16.0 - var9 - 2.0) + 1.0 + var9 / 2.0;
            let var17 = next_double(world, rand) * (8.0 - var11 - 4.0) + 2.0 + var11 / 2.0;
            let var19 = next_double(world, rand) * (16.0 - var13 - 2.0) + 1.0 + var13 / 2.0;
            for var21 in 1..15 {
                for var22 in 1..15 {
                    for var23 in 1..7 {
                        let var24 = (var21 as f64 - var15) / (var9 / 2.0);
                        let var26 = (var23 as f64 - var17) / (var11 / 2.0);
                        let var28 = (var22 as f64 - var19) / (var13 / 2.0);
                        let var30 = var24 * var24 + var26 * var26 + var28 * var28;
                        if var30 < 1.0 {
                            var6[idx(var21, var22, var23)] = true;
                        }
                    }
                }
            }
        }

        // src: WorldGenLakes.java:49-65 (the border check)
        for var8 in 0..16 {
            for var32 in 0..16 {
                for var10 in 0..8 {
                    if is_border(&var6, var8, var32, var10) {
                        let var12 = world.get_block_material(x + var8, y + var10, z + var32);
                        if var10 >= 4 && var12.get_is_liquid() {
                            return false;
                        }
                        if var10 < 4
                            && !var12.is_solid()
                            && world.get_block_id(x + var8, y + var10, z + var32)
                                != self.field_15235_a
                        {
                            return false;
                        }
                    }
                }
            }
        }

        // src: WorldGenLakes.java:67-75
        for var8 in 0..16 {
            for var32 in 0..16 {
                for var10 in 0..8 {
                    if var6[idx(var8, var32, var10)] {
                        let id = if var10 >= 4 { 0 } else { self.field_15235_a };
                        world.set_block(x + var8, y + var10, z + var32, id);
                    }
                }
            }
        }

        // src: WorldGenLakes.java:77-85 (dirt under an air cell with sky light)
        for var8 in 0..16 {
            for var32 in 0..16 {
                for var10 in 4..8 {
                    if var6[idx(var8, var32, var10)]
                        && world.get_block_id(x + var8, y + var10 - 1, z + var32) == ids::DIRT
                        && world.get_saved_light_value(
                            EnumSkyBlock::Sky,
                            x + var8,
                            y + var10,
                            z + var32,
                        ) > 0
                    {
                        world.set_block(x + var8, y + var10 - 1, z + var32, ids::GRASS);
                    }
                }
            }
        }

        // src: WorldGenLakes.java:87-98 (lava: stone border, the upper half
        // with a coin flip; the draw comes before the material read)
        if block::material_of(self.field_15235_a) == Material::Lava {
            for var8 in 0..16 {
                for var32 in 0..16 {
                    for var10 in 0..8 {
                        if is_border(&var6, var8, var32, var10)
                            && (var10 < 4 || next_int(world, rand, 2) != 0)
                            && world
                                .get_block_material(x + var8, y + var10, z + var32)
                                .is_solid()
                        {
                            world.set_block(x + var8, y + var10, z + var32, ids::STONE);
                        }
                    }
                }
            }
        }
        true
    }
}
