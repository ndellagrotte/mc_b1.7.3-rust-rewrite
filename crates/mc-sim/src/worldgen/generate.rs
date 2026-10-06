//! `ChunkProviderGenerate` (ChunkProviderGenerate.java): the overworld
//! generator up to `provideChunk` (terrain shape, biome surface, caves, sky
//! light). `populate` is WP-1.7 and is recorded as skipped
//! ([`crate::world::chunk_provider::ChunkGenerator::populate`]).
//!
//! The noise arrays are fields reused across calls, as in vanilla; each use
//! zeroes the whole array first, so their history is unobservable.

use mc_core::JavaRandom;

use super::biome::Biome;
use super::caves::{CaveKind, MapGenCaves};
use super::ids;
use super::octaves::NoiseGeneratorOctaves;
use crate::world::World;
use crate::world::chunk::{self, CHUNK_VOLUME, Chunk};
use crate::world::chunk_provider::ChunkProvider;

/// `ChunkProviderGenerate`.
#[derive(Clone, Debug, PartialEq)]
pub struct ChunkProviderGenerate {
    /// `rand`: reseeded from the chunk coordinates by `provideChunk`, from the
    /// world seed and coordinates by `populate`.
    pub rand: JavaRandom,
    /// `field_912_k`: 16 octaves, the first of the two terrain-density noises
    /// (`field_4184_e`).
    pub field_912_k: NoiseGeneratorOctaves,
    /// `field_911_l`: 16 octaves, the second density noise (`field_4183_f`).
    pub field_911_l: NoiseGeneratorOctaves,
    /// `field_910_m`: 8 octaves, the blend between the two density noises
    /// (`field_4185_d`).
    pub field_910_m: NoiseGeneratorOctaves,
    /// `field_909_n`: 4 octaves, sand and gravel beach noise.
    pub field_909_n: NoiseGeneratorOctaves,
    /// `field_908_o`: 4 octaves, surface-layer depth noise.
    pub field_908_o: NoiseGeneratorOctaves,
    /// `field_922_a`: 10 octaves, 2D, scales the density falloff (`field_4182_g`).
    pub field_922_a: NoiseGeneratorOctaves,
    /// `field_921_b`: 16 octaves, 2D, shifts the terrain height (`field_4181_h`).
    pub field_921_b: NoiseGeneratorOctaves,
    /// `mobSpawnerNoise`: 8 octaves, tree-count noise of `populate`.
    pub mob_spawner_noise: NoiseGeneratorOctaves,
    // `worldObj`: passed to each call.
    /// `field_4180_q`: the 5×17×5 density grid.
    field_4180_q: Vec<f64>,
    /// `sandNoise`.
    sand_noise: Vec<f64>,
    /// `gravelNoise`.
    gravel_noise: Vec<f64>,
    /// `stoneNoise`: surface-layer depth.
    stone_noise: Vec<f64>,
    /// `field_902_u`: the cave generator.
    pub field_902_u: MapGenCaves,
    /// `biomesForGeneration`.
    biomes_for_generation: Vec<Biome>,
    /// `field_4185_d` (from `field_910_m`).
    field_4185_d: Vec<f64>,
    /// `field_4184_e` (from `field_912_k`).
    field_4184_e: Vec<f64>,
    /// `field_4183_f` (from `field_911_l`).
    field_4183_f: Vec<f64>,
    /// `field_4182_g` (from `field_922_a`).
    field_4182_g: Vec<f64>,
    /// `field_4181_h` (from `field_921_b`).
    field_4181_h: Vec<f64>,
    // `field_914_i` (`int[32][32]`, ChunkProviderGenerate.java:27): never used.
    /// `generatedTemperatures`: `populate`'s snow temperatures (WP-1.7).
    pub generated_temperatures: Vec<f64>,
}

impl ChunkProviderGenerate {
    /// `new ChunkProviderGenerate(world, seed)`: eight octave generators from
    /// one `Random(seed)`, in field order.
    // src: ChunkProviderGenerate.java:30-41
    pub fn new(var2: i64) -> Self {
        let mut rand = JavaRandom::new(var2);
        let field_912_k = NoiseGeneratorOctaves::new(&mut rand, 16);
        let field_911_l = NoiseGeneratorOctaves::new(&mut rand, 16);
        let field_910_m = NoiseGeneratorOctaves::new(&mut rand, 8);
        let field_909_n = NoiseGeneratorOctaves::new(&mut rand, 4);
        let field_908_o = NoiseGeneratorOctaves::new(&mut rand, 4);
        let field_922_a = NoiseGeneratorOctaves::new(&mut rand, 10);
        let field_921_b = NoiseGeneratorOctaves::new(&mut rand, 16);
        let mob_spawner_noise = NoiseGeneratorOctaves::new(&mut rand, 8);
        ChunkProviderGenerate {
            rand,
            field_912_k,
            field_911_l,
            field_910_m,
            field_909_n,
            field_908_o,
            field_922_a,
            field_921_b,
            mob_spawner_noise,
            field_4180_q: Vec::new(),
            // src: ChunkProviderGenerate.java:17-19 (new double[256])
            sand_noise: vec![0.0; 256],
            gravel_noise: vec![0.0; 256],
            stone_noise: vec![0.0; 256],
            field_902_u: MapGenCaves::new(CaveKind::Overworld),
            biomes_for_generation: Vec::new(),
            field_4185_d: Vec::new(),
            field_4184_e: Vec::new(),
            field_4183_f: Vec::new(),
            field_4182_g: Vec::new(),
            field_4181_h: Vec::new(),
            generated_temperatures: Vec::new(),
        }
    }

    /// `generateTerrain(x, z, blocks, biomes, temperatures)`: stone where the
    /// trilinearly interpolated density is positive, water (ice at y = 63
    /// where the temperature is below 0.5) below sea level 64, else air.
    /// `temperature` and `humidity` are the world chunk manager's arrays
    /// right after `loadBlockGeneratorData` (`var5`, and what
    /// `func_4061_a` reads from the manager).
    // src: ChunkProviderGenerate.java:43-111
    #[allow(clippy::identity_op)] // the Java's `+ 0` offsets are kept
    pub fn generate_terrain(
        &mut self,
        var1: i32,
        var2: i32,
        var3: &mut [u8; CHUNK_VOLUME],
        temperature: &[f64],
        humidity: &[f64],
    ) {
        let var6: i32 = 4;
        let var7: i32 = 64;
        let var8 = var6 + 1;
        let var9: i32 = 17;
        let var10 = var6 + 1;
        let q = std::mem::take(&mut self.field_4180_q);
        self.field_4180_q = self.func_4061_a(
            q,
            var1 * var6,
            0,
            var2 * var6,
            var8,
            var9,
            var10,
            temperature,
            humidity,
        );
        let q = &self.field_4180_q;
        let at = |i: i32| q[i as usize];
        let var5 = temperature;

        for var11 in 0..var6 {
            for var12 in 0..var6 {
                for var13 in 0..16 {
                    let var14 = 0.125;
                    let mut var16 = at(((var11 + 0) * var10 + var12 + 0) * var9 + var13 + 0);
                    let mut var18 = at(((var11 + 0) * var10 + var12 + 1) * var9 + var13 + 0);
                    let mut var20 = at(((var11 + 1) * var10 + var12 + 0) * var9 + var13 + 0);
                    let mut var22 = at(((var11 + 1) * var10 + var12 + 1) * var9 + var13 + 0);
                    let var24 =
                        (at(((var11 + 0) * var10 + var12 + 0) * var9 + var13 + 1) - var16) * var14;
                    let var26 =
                        (at(((var11 + 0) * var10 + var12 + 1) * var9 + var13 + 1) - var18) * var14;
                    let var28 =
                        (at(((var11 + 1) * var10 + var12 + 0) * var9 + var13 + 1) - var20) * var14;
                    let var30 =
                        (at(((var11 + 1) * var10 + var12 + 1) * var9 + var13 + 1) - var22) * var14;

                    for var32 in 0..8 {
                        let var33 = 0.25;
                        let mut var35 = var16;
                        let mut var37 = var18;
                        let var39 = (var20 - var16) * var33;
                        let var41 = (var22 - var18) * var33;

                        for var43 in 0..4 {
                            // src: ChunkProviderGenerate.java:72 (`+` binds
                            // tighter than `<<`, which binds tighter than `|`)
                            let mut var44 =
                                (var43 + var11 * 4) << 11 | (var12 * 4) << 7 | (var13 * 8 + var32);
                            let var45: i32 = 128;
                            let var46 = 0.25;
                            let mut var48 = var35;
                            let var50 = (var37 - var35) * var46;

                            for var52 in 0..4 {
                                let var53 =
                                    var5[((var11 * 4 + var43) * 16 + var12 * 4 + var52) as usize];
                                let mut var55: u8 = 0;
                                if var13 * 8 + var32 < var7 {
                                    if var53 < 0.5 && var13 * 8 + var32 >= var7 - 1 {
                                        var55 = ids::ICE;
                                    } else {
                                        var55 = ids::WATER_STILL;
                                    }
                                }
                                if var48 > 0.0 {
                                    var55 = ids::STONE;
                                }
                                var3[var44 as usize] = var55;
                                var44 += var45;
                                var48 += var50;
                            }
                            var35 += var39;
                            var37 += var41;
                        }
                        var16 += var24;
                        var18 += var26;
                        var20 += var28;
                        var22 += var30;
                    }
                }
            }
        }
    }

    /// `replaceBlocksForBiome(x, z, blocks, biomes)`: bedrock at the bottom,
    /// then per column from the top down the biome's top and filler blocks
    /// (sand, gravel, sandstone variants near sea level).
    // src: ChunkProviderGenerate.java:113-187
    pub fn replace_blocks_for_biome(
        &mut self,
        var1: i32,
        var2: i32,
        var3: &mut [u8; CHUNK_VOLUME],
        var4: &[Biome],
    ) {
        let var5: i32 = 64;
        let var6 = 0.03125;
        self.sand_noise = self.field_909_n.generate_noise_octaves(
            std::mem::take(&mut self.sand_noise),
            (var1 * 16) as f64,
            (var2 * 16) as f64,
            0.0,
            16,
            16,
            1,
            var6,
            var6,
            1.0,
        );
        self.gravel_noise = self.field_909_n.generate_noise_octaves(
            std::mem::take(&mut self.gravel_noise),
            (var1 * 16) as f64,
            109.0134,
            (var2 * 16) as f64,
            16,
            1,
            16,
            var6,
            1.0,
            var6,
        );
        self.stone_noise = self.field_908_o.generate_noise_octaves(
            std::mem::take(&mut self.stone_noise),
            (var1 * 16) as f64,
            (var2 * 16) as f64,
            0.0,
            16,
            16,
            1,
            var6 * 2.0,
            var6 * 2.0,
            var6 * 2.0,
        );

        for var8 in 0..16i32 {
            for var9 in 0..16i32 {
                let var10 = var4[(var8 + var9 * 16) as usize].data();
                let ni = (var8 + var9 * 16) as usize;
                let var11 = self.sand_noise[ni] + self.rand.next_double() * 0.2 > 0.0;
                let var12 = self.gravel_noise[ni] + self.rand.next_double() * 0.2 > 3.0;
                let var13 =
                    (self.stone_noise[ni] / 3.0 + 3.0 + self.rand.next_double() * 0.25) as i32;
                let mut var14: i32 = -1;
                let mut var15: u8 = var10.top_block;
                let mut var16: u8 = var10.filler_block;

                for var17 in (0..=127i32).rev() {
                    let var18 = ((var9 * 16 + var8) * 128 + var17) as usize;
                    if var17 <= self.rand.next_int_bounded(5) {
                        var3[var18] = ids::BEDROCK;
                    } else {
                        let var19 = var3[var18];
                        if var19 == 0 {
                            var14 = -1;
                        } else if var19 == ids::STONE {
                            if var14 == -1 {
                                if var13 <= 0 {
                                    var15 = 0;
                                    var16 = ids::STONE;
                                } else if var17 >= var5 - 4 && var17 <= var5 + 1 {
                                    var15 = var10.top_block;
                                    var16 = var10.filler_block;
                                    if var12 {
                                        var15 = 0;
                                    }
                                    if var12 {
                                        var16 = ids::GRAVEL;
                                    }
                                    if var11 {
                                        var15 = ids::SAND;
                                    }
                                    if var11 {
                                        var16 = ids::SAND;
                                    }
                                }
                                if var17 < var5 && var15 == 0 {
                                    var15 = ids::WATER_STILL;
                                }
                                var14 = var13;
                                if var17 >= var5 - 1 {
                                    var3[var18] = var15;
                                } else {
                                    var3[var18] = var16;
                                }
                            } else if var14 > 0 {
                                var14 -= 1;
                                var3[var18] = var16;
                                if var14 == 0 && var16 == ids::SAND {
                                    var14 = self.rand.next_int_bounded(4);
                                    var16 = ids::SAND_STONE;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// `func_4061_a(array, x, y, z, xSize, ySize, zSize)`: the terrain density
    /// grid (5×17×5 for a chunk), from the 2D height noises, the biome
    /// temperature × humidity, and the blended 3D density noises.
    /// `var12`/`var13` are the world chunk manager's `temperature` and
    /// `humidity` (ChunkProviderGenerate.java:213-214).
    // src: ChunkProviderGenerate.java:206-304
    #[allow(clippy::too_many_arguments)]
    fn func_4061_a(
        &mut self,
        mut var1: Vec<f64>,
        var2: i32,
        var3: i32,
        var4: i32,
        var5: i32,
        var6: i32,
        var7: i32,
        var12: &[f64],
        var13: &[f64],
    ) -> Vec<f64> {
        if var1.is_empty() {
            var1 = vec![0.0; (var5 * var6 * var7) as usize];
        }
        let var8 = 684.412;
        let var10 = 684.412;
        self.field_4182_g = self.field_922_a.generate_noise_octaves_2d(
            std::mem::take(&mut self.field_4182_g),
            var2,
            var4,
            var5,
            var7,
            1.121,
            1.121,
            0.5,
        );
        self.field_4181_h = self.field_921_b.generate_noise_octaves_2d(
            std::mem::take(&mut self.field_4181_h),
            var2,
            var4,
            var5,
            var7,
            200.0,
            200.0,
            0.5,
        );
        self.field_4185_d = self.field_910_m.generate_noise_octaves(
            std::mem::take(&mut self.field_4185_d),
            var2 as f64,
            var3 as f64,
            var4 as f64,
            var5,
            var6,
            var7,
            var8 / 80.0,
            var10 / 160.0,
            var8 / 80.0,
        );
        self.field_4184_e = self.field_912_k.generate_noise_octaves(
            std::mem::take(&mut self.field_4184_e),
            var2 as f64,
            var3 as f64,
            var4 as f64,
            var5,
            var6,
            var7,
            var8,
            var10,
            var8,
        );
        self.field_4183_f = self.field_911_l.generate_noise_octaves(
            std::mem::take(&mut self.field_4183_f),
            var2 as f64,
            var3 as f64,
            var4 as f64,
            var5,
            var6,
            var7,
            var8,
            var10,
            var8,
        );
        let mut var14: usize = 0;
        let mut var15: usize = 0;
        let var16 = 16 / var5;

        for var17 in 0..var5 {
            let var18 = var17 * var16 + var16 / 2;
            for var19 in 0..var7 {
                let var20 = var19 * var16 + var16 / 2;
                let var21 = var12[(var18 * 16 + var20) as usize];
                let var23 = var13[(var18 * 16 + var20) as usize] * var21;
                let mut var25 = 1.0 - var23;
                var25 *= var25;
                var25 *= var25;
                var25 = 1.0 - var25;
                let mut var27 = (self.field_4182_g[var15] + 256.0) / 512.0;
                var27 *= var25;
                if var27 > 1.0 {
                    var27 = 1.0;
                }
                let mut var29 = self.field_4181_h[var15] / 8000.0;
                if var29 < 0.0 {
                    var29 = -var29 * 0.3;
                }
                var29 = var29 * 3.0 - 2.0;
                if var29 < 0.0 {
                    var29 /= 2.0;
                    if var29 < -1.0 {
                        var29 = -1.0;
                    }
                    var29 /= 1.4;
                    var29 /= 2.0;
                    var27 = 0.0;
                } else {
                    if var29 > 1.0 {
                        var29 = 1.0;
                    }
                    var29 /= 8.0;
                }
                if var27 < 0.0 {
                    var27 = 0.0;
                }
                var27 += 0.5;
                var29 = var29 * var6 as f64 / 16.0;
                let var31 = var6 as f64 / 2.0 + var29 * 4.0;
                var15 += 1;

                for var33 in 0..var6 {
                    let mut var34;
                    let mut var36 = (var33 as f64 - var31) * 12.0 / var27;
                    if var36 < 0.0 {
                        var36 *= 4.0;
                    }
                    let var38 = self.field_4184_e[var14] / 512.0;
                    let var40 = self.field_4183_f[var14] / 512.0;
                    let var42 = (self.field_4185_d[var14] / 10.0 + 1.0) / 2.0;
                    if var42 < 0.0 {
                        var34 = var38;
                    } else if var42 > 1.0 {
                        var34 = var40;
                    } else {
                        var34 = var38 + (var40 - var38) * var42;
                    }
                    var34 -= var36;
                    if var33 > var6 - 4 {
                        // src: ChunkProviderGenerate.java:293 (a float division)
                        let var44 = ((var33 - (var6 - 4)) as f32 / 3.0f32) as f64;
                        var34 = var34 * (1.0 - var44) + -10.0 * var44;
                    }
                    var1[var14] = var34;
                    var14 += 1;
                }
            }
        }
        var1
    }
}

impl ChunkProvider for ChunkProviderGenerate {
    /// `provideChunk(x, z)`: reseed `rand` from the coordinates, biomes,
    /// terrain, surface, caves, then `new Chunk` and `generateSkylightMap()`.
    // src: ChunkProviderGenerate.java:193-204
    fn provide_chunk(&mut self, world: &mut World, var1: i32, var2: i32) -> Chunk {
        self.rand.set_seed(
            (var1 as i64)
                .wrapping_mul(341873128712)
                .wrapping_add((var2 as i64).wrapping_mul(132897987541)),
        );
        let mut var3: Box<[u8; CHUNK_VOLUME]> = Box::new([0u8; CHUNK_VOLUME]);
        // `new Chunk(world, var3, x, z)` (:196) reads nothing from the array,
        // so it is built after the array is filled.
        let seed = world.get_random_seed();
        let wcm = world.get_world_chunk_manager();
        let biomes = std::mem::take(&mut self.biomes_for_generation);
        self.biomes_for_generation =
            wcm.load_block_generator_data(biomes, var1 * 16, var2 * 16, 16, 16);
        // src: ChunkProviderGenerate.java:198 (var5 = manager.temperature)
        self.generate_terrain(var1, var2, &mut var3, &wcm.temperature, &wcm.humidity);
        let biomes = std::mem::take(&mut self.biomes_for_generation);
        self.replace_blocks_for_biome(var1, var2, &mut var3, &biomes);
        self.biomes_for_generation = biomes;
        self.field_902_u.generate(seed, var1, var2, &mut var3);
        let mut var4 = Chunk::new(var3, var1, var2);
        chunk::generate_skylight_map(world, &mut var4);
        var4
    }

    // src: ChunkProviderGenerate.java:306-308
    fn chunk_exists(&self, _x: i32, _z: i32) -> bool {
        true
    }

    // src: ChunkProviderGenerate.java:609-611
    fn unload_100_oldest_chunks(&mut self) -> bool {
        false
    }

    // src: ChunkProviderGenerate.java:613-615
    fn can_save(&self) -> bool {
        true
    }

    // src: ChunkProviderGenerate.java:617-619
    fn make_string(&self) -> String {
        "RandomLevelSource".to_string()
    }
}
