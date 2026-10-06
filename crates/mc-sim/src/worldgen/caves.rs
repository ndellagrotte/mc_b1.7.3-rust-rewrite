//! `MapGenBase` (MapGenBase.java), `MapGenCaves` (MapGenCaves.java) and
//! `MapGenCavesHell` (MapGenCavesHell.java): caves carved into a chunk's
//! block array before it becomes a `Chunk`.
//!
//! `MapGenBase.generate` visits every chunk within `range` (8) of the target
//! in x-outer, z-inner order, reseeds `rand` from the world seed and that
//! chunk, and lets the subclass start tunnels there that may reach the
//! target. The two subclasses differ in a handful of constants and in what
//! they carve; [`CaveKind`] selects them, and each difference is cited.
//!
//! Chunk-to-block arithmetic (`chunk * 16 + …`) is Java `int` arithmetic and
//! uses `wrapping_*`: `generate` visits chunks up to 8 beyond the target, so
//! at the edge of the int range `var2 * 16` wraps in Java and must not panic
//! in a debug build.

use mc_core::JavaRandom;
use mc_core::math::{self, floor_double};

use super::ids;
use crate::world::chunk::CHUNK_VOLUME;

/// Which `MapGenBase` subclass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaveKind {
    /// `MapGenCaves` (overworld).
    Overworld,
    /// `MapGenCavesHell` (nether).
    Hell,
}

/// `MapGenBase` with its subclass.
#[derive(Clone, Debug, PartialEq)]
pub struct MapGenCaves {
    /// The subclass.
    pub kind: CaveKind,
    /// `range` (8).
    // src: MapGenBase.java:6
    pub range: i32,
    /// `rand` (`new Random()`, unseeded in vanilla; always reseeded before
    /// use, so the initial seed is unobservable).
    // src: MapGenBase.java:7
    pub rand: JavaRandom,
}

impl MapGenCaves {
    /// `new MapGenCaves()` / `new MapGenCavesHell()`.
    pub fn new(kind: CaveKind) -> Self {
        MapGenCaves {
            kind,
            range: 8,
            rand: JavaRandom::new(0),
        }
    }

    /// `MapGenBase.generate(provider, world, chunkX, chunkZ, blocks)`.
    // src: MapGenBase.java:9-22
    pub fn generate(
        &mut self,
        random_seed: i64,
        var3: i32,
        var4: i32,
        var5: &mut [u8; CHUNK_VOLUME],
    ) {
        let var6 = self.range;
        self.rand.set_seed(random_seed);
        let var7 = self.rand.next_long() / 2 * 2 + 1;
        let var9 = self.rand.next_long() / 2 * 2 + 1;
        for var11 in var3.wrapping_sub(var6)..=var3.wrapping_add(var6) {
            for var12 in var4.wrapping_sub(var6)..=var4.wrapping_add(var6) {
                self.rand.set_seed(
                    (var11 as i64)
                        .wrapping_mul(var7)
                        .wrapping_add((var12 as i64).wrapping_mul(var9))
                        ^ random_seed,
                );
                self.recursive_generate(var11, var12, var3, var4, var5);
            }
        }
    }

    /// `generateLargeCaveNode(chunkX, chunkZ, blocks, x, y, z)`: a room, a
    /// node with size `1 + rand * 6`, no direction, `-1, -1`, vertical
    /// scale 0.5.
    // src: MapGenCaves.java:6-8, MapGenCavesHell.java:6-8
    fn generate_large_cave_node(
        &mut self,
        var1: i32,
        var2: i32,
        var3: &mut [u8; CHUNK_VOLUME],
        var4: f64,
        var6: f64,
        var8: f64,
    ) {
        let size = 1.0f32 + self.rand.next_float() * 6.0f32;
        self.generate_cave_node(
            var1, var2, var3, var4, var6, var8, size, 0.0, 0.0, -1, -1, 0.5,
        );
    }

    /// `generateCaveNode(chunkX, chunkZ, blocks, x, y, z, size, yaw, pitch,
    /// step, length, verticalScale)`: walks one tunnel (`step == -1`: a
    /// room) from `step` to `length`, carving where it passes through the
    /// target chunk, and may split into two tunnels.
    // src: MapGenCaves.java:10-185, MapGenCavesHell.java:10-146
    #[allow(clippy::too_many_arguments)]
    fn generate_cave_node(
        &mut self,
        var1: i32,
        var2: i32,
        var3: &mut [u8; CHUNK_VOLUME],
        mut var4: f64,
        mut var6: f64,
        mut var8: f64,
        var10: f32,
        mut var11: f32,
        mut var12: f32,
        mut var13: i32,
        mut var14: i32,
        var15: f64,
    ) {
        let var17 = var1.wrapping_mul(16).wrapping_add(8) as f64;
        let var19 = var2.wrapping_mul(16).wrapping_add(8) as f64;
        let mut var21 = 0.0f32;
        let mut var22 = 0.0f32;
        let mut var23 = JavaRandom::new(self.rand.next_long());
        if var14 <= 0 {
            let var24 = self.range * 16 - 16;
            var14 = var24 - var23.next_int_bounded(var24 / 4);
        }
        let mut var52 = false;
        if var13 == -1 {
            var13 = var14 / 2;
            var52 = true;
        }
        let var25 = var23.next_int_bounded(var14 / 2) + var14 / 4;
        let var26 = var23.next_int_bounded(6) == 0;
        while var13 < var14 {
            let var27 = 1.5
                + (math::sin(var13 as f32 * 3.1415927f32 / var14 as f32) * var10 * 1.0f32) as f64;
            let var29 = var27 * var15;
            let var31 = math::cos(var12);
            let var32 = math::sin(var12);
            var4 += (math::cos(var11) * var31) as f64;
            var6 += var32 as f64;
            var8 += (math::sin(var11) * var31) as f64;
            if var26 {
                var12 *= 0.92f32;
            } else {
                var12 *= 0.7f32;
            }
            var12 += var22 * 0.1f32;
            var11 += var21 * 0.1f32;
            var22 *= 0.9f32;
            var21 *= 0.75f32;
            var22 += (var23.next_float() - var23.next_float()) * var23.next_float() * 2.0f32;
            var21 += (var23.next_float() - var23.next_float()) * var23.next_float() * 4.0f32;
            if !var52 && var13 == var25 && var10 > 1.0f32 {
                let s1 = var23.next_float() * 0.5f32 + 0.5f32;
                self.generate_cave_node(
                    var1,
                    var2,
                    var3,
                    var4,
                    var6,
                    var8,
                    s1,
                    var11 - 1.5707964f32,
                    var12 / 3.0f32,
                    var13,
                    var14,
                    1.0,
                );
                let s2 = var23.next_float() * 0.5f32 + 0.5f32;
                self.generate_cave_node(
                    var1,
                    var2,
                    var3,
                    var4,
                    var6,
                    var8,
                    s2,
                    var11 + 1.5707964f32,
                    var12 / 3.0f32,
                    var13,
                    var14,
                    1.0,
                );
                return;
            }

            if var52 || var23.next_int_bounded(4) != 0 {
                let var33 = var4 - var17;
                let var35 = var8 - var19;
                let var37 = (var14 - var13) as f64;
                let var39 = (var10 + 2.0f32 + 16.0f32) as f64;
                if var33 * var33 + var35 * var35 - var37 * var37 > var39 * var39 {
                    return;
                }

                if var4 >= var17 - 16.0 - var27 * 2.0
                    && var8 >= var19 - 16.0 - var27 * 2.0
                    && var4 <= var17 + 16.0 + var27 * 2.0
                    && var8 <= var19 + 16.0 + var27 * 2.0
                {
                    let mut var53 = floor_double(var4 - var27)
                        .wrapping_sub(var1.wrapping_mul(16))
                        .wrapping_sub(1);
                    let mut var34 = floor_double(var4 + var27)
                        .wrapping_sub(var1.wrapping_mul(16))
                        .wrapping_add(1);
                    let mut var54 = floor_double(var6 - var29) - 1;
                    let mut var36 = floor_double(var6 + var29) + 1;
                    let mut var55 = floor_double(var8 - var27)
                        .wrapping_sub(var2.wrapping_mul(16))
                        .wrapping_sub(1);
                    let mut var38 = floor_double(var8 + var27)
                        .wrapping_sub(var2.wrapping_mul(16))
                        .wrapping_add(1);
                    if var53 < 0 {
                        var53 = 0;
                    }
                    if var34 > 16 {
                        var34 = 16;
                    }
                    if var54 < 1 {
                        var54 = 1;
                    }
                    if var36 > 120 {
                        var36 = 120;
                    }
                    if var55 < 0 {
                        var55 = 0;
                    }
                    if var38 > 16 {
                        var38 = 16;
                    }

                    // Look for the fluid this generator must not breach: water
                    // in the overworld, lava in the nether. Inner columns
                    // check only their top and bottom (the y loop jumps).
                    // src: MapGenCaves.java:121-140, MapGenCavesHell.java:95-114
                    let (fluid_moving, fluid_still) = match self.kind {
                        CaveKind::Overworld => (ids::WATER_MOVING, ids::WATER_STILL),
                        CaveKind::Hell => (ids::LAVA_MOVING, ids::LAVA_STILL),
                    };
                    let mut var56 = false;
                    let mut var40 = var53;
                    while !var56 && var40 < var34 {
                        let mut var41 = var55;
                        while !var56 && var41 < var38 {
                            let mut var42 = var36 + 1;
                            while !var56 && var42 >= var54 - 1 {
                                let var43 = ((var40 * 16 + var41) * 128 + var42) as usize;
                                if (0..128).contains(&var42) {
                                    if var3[var43] == fluid_moving || var3[var43] == fluid_still {
                                        var56 = true;
                                    }
                                    if var42 != var54 - 1
                                        && var40 != var53
                                        && var40 != var34 - 1
                                        && var41 != var55
                                        && var41 != var38 - 1
                                    {
                                        var42 = var54;
                                    }
                                }
                                var42 -= 1;
                            }
                            var41 += 1;
                        }
                        var40 += 1;
                    }

                    if !var56 {
                        for var40 in var53..var34 {
                            let var57 = (var40.wrapping_add(var1.wrapping_mul(16)) as f64 + 0.5
                                - var4)
                                / var27;
                            for var43 in var55..var38 {
                                let var44 =
                                    (var43.wrapping_add(var2.wrapping_mul(16)) as f64 + 0.5 - var8)
                                        / var27;
                                // vanilla bug: the index starts at y = var36
                                // while the loop variable starts at var36 - 1,
                                // so the block tested and carved is one above
                                // the y the ellipsoid test uses.
                                let mut var46 = ((var40 * 16 + var43) * 128 + var36) as usize;
                                match self.kind {
                                    CaveKind::Overworld => {
                                        // src: MapGenCaves.java:149-173
                                        let mut var47 = false;
                                        if var57 * var57 + var44 * var44 < 1.0 {
                                            let mut var48 = var36 - 1;
                                            while var48 >= var54 {
                                                let var49 = (var48 as f64 + 0.5 - var6) / var29;
                                                if var49 > -0.7
                                                    && var57 * var57 + var49 * var49 + var44 * var44
                                                        < 1.0
                                                {
                                                    let var51 = var3[var46];
                                                    if var51 == ids::GRASS {
                                                        var47 = true;
                                                    }
                                                    if var51 == ids::STONE
                                                        || var51 == ids::DIRT
                                                        || var51 == ids::GRASS
                                                    {
                                                        if var48 < 10 {
                                                            var3[var46] = ids::LAVA_MOVING;
                                                        } else {
                                                            var3[var46] = 0;
                                                            if var47 && var3[var46 - 1] == ids::DIRT
                                                            {
                                                                var3[var46 - 1] = ids::GRASS;
                                                            }
                                                        }
                                                    }
                                                }
                                                var46 -= 1;
                                                var48 -= 1;
                                            }
                                        }
                                    }
                                    CaveKind::Hell => {
                                        // src: MapGenCavesHell.java:122-134
                                        let mut var47 = var36 - 1;
                                        while var47 >= var54 {
                                            let var48 = (var47 as f64 + 0.5 - var6) / var29;
                                            if var48 > -0.7
                                                && var57 * var57 + var48 * var48 + var44 * var44
                                                    < 1.0
                                            {
                                                let var50 = var3[var46];
                                                if var50 == ids::NETHERRACK
                                                    || var50 == ids::DIRT
                                                    || var50 == ids::GRASS
                                                {
                                                    var3[var46] = 0;
                                                }
                                            }
                                            var46 -= 1;
                                            var47 -= 1;
                                        }
                                    }
                                }
                            }
                        }
                        if var52 {
                            break;
                        }
                    }
                }
            }
            var13 += 1;
        }
    }

    /// `recursiveGenerate(world, chunkX, chunkZ, targetX, targetZ, blocks)`:
    /// how many cave systems start in chunk `(var2, var3)`, and each one's
    /// rooms and tunnels.
    // src: MapGenCaves.java:187-211, MapGenCavesHell.java:148-172
    fn recursive_generate(
        &mut self,
        var2: i32,
        var3: i32,
        var4: i32,
        var5: i32,
        var6: &mut [u8; CHUNK_VOLUME],
    ) {
        let hell = self.kind == CaveKind::Hell;
        // src: MapGenCaves.java:188-191 (40, 15), MapGenCavesHell.java:149-152 (10, 5)
        let (count_bound, chance) = if hell { (10, 5) } else { (40, 15) };
        let a = self.rand.next_int_bounded(count_bound) + 1;
        let b = self.rand.next_int_bounded(a) + 1;
        let mut var7 = self.rand.next_int_bounded(b);
        if self.rand.next_int_bounded(chance) != 0 {
            var7 = 0;
        }
        for _var8 in 0..var7 {
            let var9 = var2
                .wrapping_mul(16)
                .wrapping_add(self.rand.next_int_bounded(16)) as f64;
            // src: MapGenCaves.java:195 (nextInt(nextInt(120) + 8)),
            // MapGenCavesHell.java:156 (nextInt(128))
            let var11 = if hell {
                self.rand.next_int_bounded(128) as f64
            } else {
                let bound = self.rand.next_int_bounded(120) + 8;
                self.rand.next_int_bounded(bound) as f64
            };
            let var13 = var3
                .wrapping_mul(16)
                .wrapping_add(self.rand.next_int_bounded(16)) as f64;
            let mut var15 = 1;
            if self.rand.next_int_bounded(4) == 0 {
                self.generate_large_cave_node(var4, var5, var6, var9, var11, var13);
                var15 += self.rand.next_int_bounded(4);
            }
            for _var16 in 0..var15 {
                let var17 = self.rand.next_float() * 3.1415927f32 * 2.0f32;
                let var18 = (self.rand.next_float() - 0.5f32) * 2.0f32 / 8.0f32;
                let var19 = self.rand.next_float() * 2.0f32 + self.rand.next_float();
                // src: MapGenCaves.java:207 (size var19, scale 1.0),
                // MapGenCavesHell.java:168 (size var19 * 2, scale 0.5)
                if hell {
                    self.generate_cave_node(
                        var4,
                        var5,
                        var6,
                        var9,
                        var11,
                        var13,
                        var19 * 2.0f32,
                        var17,
                        var18,
                        0,
                        0,
                        0.5,
                    );
                } else {
                    self.generate_cave_node(
                        var4, var5, var6, var9, var11, var13, var19, var17, var18, 0, 0, 1.0,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The last chunk whose blocks fit in an `int` (`2^27 - 1`): its
    /// neighbours up to 8 chunks further out have `chunk * 16` past
    /// `i32::MAX`, which Java wraps. Must not panic with overflow checks on.
    #[test]
    fn generate_at_int_edge_wraps_like_java() {
        for kind in [CaveKind::Overworld, CaveKind::Hell] {
            let mut blocks = Box::new([1u8; CHUNK_VOLUME]);
            let mut caves = MapGenCaves::new(kind);
            caves.generate(42, (1 << 27) - 1, 0, &mut blocks);
            caves.generate(42, -(1 << 27), -(1 << 27), &mut blocks);
        }
    }
}
