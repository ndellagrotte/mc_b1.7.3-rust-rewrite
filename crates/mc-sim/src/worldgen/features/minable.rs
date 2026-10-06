//! `WorldGenMinable` (ore, dirt and gravel veins) and `WorldGenClay` (clay
//! patches under water): the same capsule of spheres along a random
//! horizontal segment, differing in the replaced block, the loop shape and
//! a precondition.
//!
//! Both draw, in order: `nextFloat` (the segment's angle), two `nextInt(3)`
//! (the end heights), then one `nextDouble` per step `0..=numberOfBlocks`
//! (the sphere size). The trigonometry is `MathHelper.sin`/`cos` (the `f32`
//! table, [`mc_core::math::sin`]), with Java's `float` arithmetic before the
//! widening casts.

use mc_core::math::{self, floor_double};

use super::{next_double, next_float, next_int};
use crate::block::dispatch::RandomSource;
use crate::block::{Material, ids};
use crate::world::World;

/// `(float)Math.PI` as the Java writes it (`3.1415927F`).
// src: WorldGenMinable.java:15
const PI_F: f32 = 3.1415927;

/// The segment both generators share: the two ends `(var7, var15, var11)`
/// and `(var9, var17, var13)`.
struct Segment {
    var7: f64,
    var9: f64,
    var11: f64,
    var13: f64,
    var15: f64,
    var17: f64,
}

/// The segment draws (`nextFloat`, then `nextInt(3)` twice).
// src: WorldGenMinable.java:15-21, WorldGenClay.java:18-24
fn segment(world: &mut World, rand: RandomSource, x: i32, y: i32, z: i32, n: i32) -> Segment {
    let var6 = next_float(world, rand) * PI_F;
    let nf = n as f32;
    let var7 = ((x + 8) as f32 + math::sin(var6) * nf / 8.0) as f64;
    let var9 = ((x + 8) as f32 - math::sin(var6) * nf / 8.0) as f64;
    let var11 = ((z + 8) as f32 + math::cos(var6) * nf / 8.0) as f64;
    let var13 = ((z + 8) as f32 - math::cos(var6) * nf / 8.0) as f64;
    let var15 = (y + next_int(world, rand, 3) + 2) as f64;
    let var17 = (y + next_int(world, rand, 3) + 2) as f64;
    Segment {
        var7,
        var9,
        var11,
        var13,
        var15,
        var17,
    }
}

/// One step of the segment: the centre `(var20, var22, var24)` and the
/// horizontal / vertical diameters `var28` / `var30` (equal: the Java
/// computes the same expression twice).
// src: WorldGenMinable.java:24-29, WorldGenClay.java:27-32
fn step(world: &mut World, rand: RandomSource, s: &Segment, var19: i32, n: i32) -> [f64; 5] {
    let (i, nd) = (var19 as f64, n as f64);
    let var20 = s.var7 + (s.var9 - s.var7) * i / nd;
    let var22 = s.var15 + (s.var17 - s.var15) * i / nd;
    let var24 = s.var11 + (s.var13 - s.var11) * i / nd;
    let var26 = next_double(world, rand) * nd / 16.0;
    let var28 = (math::sin(var19 as f32 * PI_F / n as f32) + 1.0) as f64 * var26 + 1.0;
    let var30 = (math::sin(var19 as f32 * PI_F / n as f32) + 1.0) as f64 * var26 + 1.0;
    [var20, var22, var24, var28, var30]
}

/// `WorldGenMinable`: replaces stone by `minableBlockId` inside the capsule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldGenMinable {
    /// `minableBlockId`.
    // src: WorldGenMinable.java:6
    pub minable_block_id: i32,
    /// `numberOfBlocks`: the segment's step count (and its length scale).
    // src: WorldGenMinable.java:7
    pub number_of_blocks: i32,
}

impl WorldGenMinable {
    /// `new WorldGenMinable(id, count)`.
    // src: WorldGenMinable.java:9-12
    pub fn new(minable_block_id: i32, number_of_blocks: i32) -> Self {
        WorldGenMinable {
            minable_block_id,
            number_of_blocks,
        }
    }

    /// `generate(world, rand, x, y, z)`: always true. The sphere test is
    /// pruned per axis (x, then x+y) before the innermost loop, and the
    /// block is read only inside the sphere (`&&`).
    // src: WorldGenMinable.java:14-56
    pub fn generate(&self, world: &mut World, rand: RandomSource, x: i32, y: i32, z: i32) -> bool {
        let n = self.number_of_blocks;
        let s = segment(world, rand, x, y, z, n);
        for var19 in 0..=n {
            let [var20, var22, var24, var28, var30] = step(world, rand, &s, var19, n);
            let var32 = floor_double(var20 - var28 / 2.0);
            let var33 = floor_double(var22 - var30 / 2.0);
            let var34 = floor_double(var24 - var28 / 2.0);
            let var35 = floor_double(var20 + var28 / 2.0);
            let var36 = floor_double(var22 + var30 / 2.0);
            let var37 = floor_double(var24 + var28 / 2.0);
            for var38 in var32..=var35 {
                let var39 = (var38 as f64 + 0.5 - var20) / (var28 / 2.0);
                if var39 * var39 < 1.0 {
                    for var41 in var33..=var36 {
                        let var42 = (var41 as f64 + 0.5 - var22) / (var30 / 2.0);
                        if var39 * var39 + var42 * var42 < 1.0 {
                            for var44 in var34..=var37 {
                                let var45 = (var44 as f64 + 0.5 - var24) / (var28 / 2.0);
                                if var39 * var39 + var42 * var42 + var45 * var45 < 1.0
                                    && world.get_block_id(var38, var41, var44) == ids::STONE
                                {
                                    world.set_block(var38, var41, var44, self.minable_block_id);
                                }
                            }
                        }
                    }
                }
            }
        }
        true
    }
}

/// `WorldGenClay`: replaces sand by clay inside the capsule, if the start
/// block is water.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldGenClay {
    /// `clayBlockId`: always `Block.blockClay.blockID`.
    // src: WorldGenClay.java:6, :10
    pub clay_block_id: i32,
    /// `numberOfBlocks`.
    // src: WorldGenClay.java:7
    pub number_of_blocks: i32,
}

impl WorldGenClay {
    /// `new WorldGenClay(count)`.
    // src: WorldGenClay.java:9-12
    pub fn new(number_of_blocks: i32) -> Self {
        WorldGenClay {
            clay_block_id: ids::BLOCK_CLAY,
            number_of_blocks,
        }
    }

    /// `generate(world, rand, x, y, z)`: false (and no draw) unless the
    /// start block's material is water. The loops run over the whole box,
    /// x, y, z, with the sphere test inside, and the block is read only
    /// inside the sphere.
    // src: WorldGenClay.java:14-59
    pub fn generate(&self, world: &mut World, rand: RandomSource, x: i32, y: i32, z: i32) -> bool {
        if world.get_block_material(x, y, z) != Material::Water {
            return false;
        }
        let n = self.number_of_blocks;
        let s = segment(world, rand, x, y, z, n);
        for var19 in 0..=n {
            let [var20, var22, var24, var28, var30] = step(world, rand, &s, var19, n);
            let var32 = floor_double(var20 - var28 / 2.0);
            let var33 = floor_double(var20 + var28 / 2.0);
            let var34 = floor_double(var22 - var30 / 2.0);
            let var35 = floor_double(var22 + var30 / 2.0);
            let var36 = floor_double(var24 - var28 / 2.0);
            let var37 = floor_double(var24 + var28 / 2.0);
            for var38 in var32..=var33 {
                for var39 in var34..=var35 {
                    for var40 in var36..=var37 {
                        let var41 = (var38 as f64 + 0.5 - var20) / (var28 / 2.0);
                        let var43 = (var39 as f64 + 0.5 - var22) / (var30 / 2.0);
                        let var45 = (var40 as f64 + 0.5 - var24) / (var28 / 2.0);
                        if var41 * var41 + var43 * var43 + var45 * var45 < 1.0 {
                            let var47 = world.get_block_id(var38, var39, var40);
                            if var47 == ids::SAND {
                                world.set_block(var38, var39, var40, self.clay_block_id);
                            }
                        }
                    }
                }
            }
        }
        true
    }
}
