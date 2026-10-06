//! `ChunkProviderHell` (ChunkProviderHell.java): the nether generator up to
//! `provideChunk` (terrain shape, surface, caves). Unlike the overworld it
//! builds the `Chunk` without `generateSkylightMap()`
//! (ChunkProviderHell.java:180-188), so height map and sky light stay zero.
//! `populate` is [`super::populate::populate_hell`] (WP-1.7), which never
//! reseeds `hellRNG` (tests/golden/README.md).

use mc_core::JavaRandom;
use mc_core::jmath;

use super::caves::{CaveKind, MapGenCaves};
use super::ids;
use super::octaves::NoiseGeneratorOctaves;
use crate::world::World;
use crate::world::chunk::{CHUNK_VOLUME, Chunk};
use crate::world::chunk_provider::ChunkProvider;

/// `ChunkProviderHell`.
#[derive(Clone, Debug, PartialEq)]
pub struct ChunkProviderHell {
    /// `hellRNG`: reseeded from the chunk coordinates only (not the world
    /// seed) by `provideChunk`.
    pub hell_rng: JavaRandom,
    /// `field_4169_i`: 16 octaves, first density noise (`field_4174_d`).
    pub field_4169_i: NoiseGeneratorOctaves,
    /// `field_4168_j`: 16 octaves, second density noise (`field_4173_e`).
    pub field_4168_j: NoiseGeneratorOctaves,
    /// `field_4167_k`: 8 octaves, the blend between them (`field_4175_c`).
    pub field_4167_k: NoiseGeneratorOctaves,
    /// `field_4166_l`: 4 octaves, soul sand and gravel noise.
    pub field_4166_l: NoiseGeneratorOctaves,
    /// `field_4165_m`: 4 octaves, surface-layer depth noise.
    pub field_4165_m: NoiseGeneratorOctaves,
    /// `field_4177_a`: 10 octaves, 2D (`field_4172_f`).
    pub field_4177_a: NoiseGeneratorOctaves,
    /// `field_4176_b`: 16 octaves, 2D (`field_4171_g`).
    pub field_4176_b: NoiseGeneratorOctaves,
    // `worldObj`: passed to each call.
    /// `field_4163_o`: the 5×17×5 density grid.
    field_4163_o: Vec<f64>,
    /// `field_4162_p`: soul-sand noise.
    field_4162_p: Vec<f64>,
    /// `gravelNoise`.
    gravel_noise: Vec<f64>,
    /// `field_4160_r`: surface-layer depth noise.
    field_4160_r: Vec<f64>,
    /// `netherCaveGenerator`.
    pub nether_cave_generator: MapGenCaves,
    /// `field_4175_c` (from `field_4167_k`).
    field_4175_c: Vec<f64>,
    /// `field_4174_d` (from `field_4169_i`).
    field_4174_d: Vec<f64>,
    /// `field_4173_e` (from `field_4168_j`).
    field_4173_e: Vec<f64>,
    /// `field_4172_f` (from `field_4177_a`).
    field_4172_f: Vec<f64>,
    /// `field_4171_g` (from `field_4176_b`).
    field_4171_g: Vec<f64>,
}

impl ChunkProviderHell {
    /// `new ChunkProviderHell(world, seed)`: seven octave generators from one
    /// `Random(seed)`, in field order.
    // src: ChunkProviderHell.java:26-36
    pub fn new(var2: i64) -> Self {
        let mut hell_rng = JavaRandom::new(var2);
        let field_4169_i = NoiseGeneratorOctaves::new(&mut hell_rng, 16);
        let field_4168_j = NoiseGeneratorOctaves::new(&mut hell_rng, 16);
        let field_4167_k = NoiseGeneratorOctaves::new(&mut hell_rng, 8);
        let field_4166_l = NoiseGeneratorOctaves::new(&mut hell_rng, 4);
        let field_4165_m = NoiseGeneratorOctaves::new(&mut hell_rng, 4);
        let field_4177_a = NoiseGeneratorOctaves::new(&mut hell_rng, 10);
        let field_4176_b = NoiseGeneratorOctaves::new(&mut hell_rng, 16);
        ChunkProviderHell {
            hell_rng,
            field_4169_i,
            field_4168_j,
            field_4167_k,
            field_4166_l,
            field_4165_m,
            field_4177_a,
            field_4176_b,
            field_4163_o: Vec::new(),
            // src: ChunkProviderHell.java:16-18 (new double[256])
            field_4162_p: vec![0.0; 256],
            gravel_noise: vec![0.0; 256],
            field_4160_r: vec![0.0; 256],
            nether_cave_generator: MapGenCaves::new(CaveKind::Hell),
            field_4175_c: Vec::new(),
            field_4174_d: Vec::new(),
            field_4173_e: Vec::new(),
            field_4172_f: Vec::new(),
            field_4171_g: Vec::new(),
        }
    }

    /// `generateNetherTerrain(x, z, blocks)`: netherrack where the
    /// interpolated density is positive, lava below y = 32, else air.
    // src: ChunkProviderHell.java:38-101
    #[allow(clippy::identity_op)] // the Java's `+ 0` offsets are kept
    pub fn generate_nether_terrain(&mut self, var1: i32, var2: i32, var3: &mut [u8; CHUNK_VOLUME]) {
        let var4: i32 = 4;
        let var5: i32 = 32;
        let var6 = var4 + 1;
        let var7: i32 = 17;
        let var8 = var4 + 1;
        let o = std::mem::take(&mut self.field_4163_o);
        self.field_4163_o = self.func_4057_a(o, var1 * var4, 0, var2 * var4, var6, var7, var8);
        let o = &self.field_4163_o;
        let at = |i: i32| o[i as usize];

        for var9 in 0..var4 {
            for var10 in 0..var4 {
                for var11 in 0..16 {
                    let var12 = 0.125;
                    let mut var14 = at(((var9 + 0) * var8 + var10 + 0) * var7 + var11 + 0);
                    let mut var16 = at(((var9 + 0) * var8 + var10 + 1) * var7 + var11 + 0);
                    let mut var18 = at(((var9 + 1) * var8 + var10 + 0) * var7 + var11 + 0);
                    let mut var20 = at(((var9 + 1) * var8 + var10 + 1) * var7 + var11 + 0);
                    let var22 =
                        (at(((var9 + 0) * var8 + var10 + 0) * var7 + var11 + 1) - var14) * var12;
                    let var24 =
                        (at(((var9 + 0) * var8 + var10 + 1) * var7 + var11 + 1) - var16) * var12;
                    let var26 =
                        (at(((var9 + 1) * var8 + var10 + 0) * var7 + var11 + 1) - var18) * var12;
                    let var28 =
                        (at(((var9 + 1) * var8 + var10 + 1) * var7 + var11 + 1) - var20) * var12;

                    for var30 in 0..8 {
                        let var31 = 0.25;
                        let mut var33 = var14;
                        let mut var35 = var16;
                        let var37 = (var18 - var14) * var31;
                        let var39 = (var20 - var16) * var31;

                        for var41 in 0..4 {
                            // src: ChunkProviderHell.java:67
                            let mut var42 =
                                (var41 + var9 * 4) << 11 | (var10 * 4) << 7 | (var11 * 8 + var30);
                            let var43: i32 = 128;
                            let var44 = 0.25;
                            let mut var46 = var33;
                            let var48 = (var35 - var33) * var44;

                            for _var50 in 0..4 {
                                let mut var51: u8 = 0;
                                if var11 * 8 + var30 < var5 {
                                    var51 = ids::LAVA_STILL;
                                }
                                if var46 > 0.0 {
                                    var51 = ids::NETHERRACK;
                                }
                                var3[var42 as usize] = var51;
                                var42 += var43;
                                var46 += var48;
                            }
                            var33 += var37;
                            var35 += var39;
                        }
                        var14 += var22;
                        var16 += var24;
                        var18 += var26;
                        var20 += var28;
                    }
                }
            }
        }
    }

    /// `func_4058_b(x, z, blocks)`: the nether's `replaceBlocksForBiome`:
    /// bedrock at both ends (each layer drawn per block), soul sand and
    /// gravel near y = 64.
    // src: ChunkProviderHell.java:103-174
    #[allow(clippy::if_same_then_else)] // two separate draws, kept as in Java
    pub fn func_4058_b(&mut self, var1: i32, var2: i32, var3: &mut [u8; CHUNK_VOLUME]) {
        let var4: i32 = 64;
        let var5 = 0.03125;
        self.field_4162_p = self.field_4166_l.generate_noise_octaves(
            std::mem::take(&mut self.field_4162_p),
            (var1 * 16) as f64,
            (var2 * 16) as f64,
            0.0,
            16,
            16,
            1,
            var5,
            var5,
            1.0,
        );
        self.gravel_noise = self.field_4166_l.generate_noise_octaves(
            std::mem::take(&mut self.gravel_noise),
            (var1 * 16) as f64,
            109.0134,
            (var2 * 16) as f64,
            16,
            1,
            16,
            var5,
            1.0,
            var5,
        );
        self.field_4160_r = self.field_4165_m.generate_noise_octaves(
            std::mem::take(&mut self.field_4160_r),
            (var1 * 16) as f64,
            (var2 * 16) as f64,
            0.0,
            16,
            16,
            1,
            var5 * 2.0,
            var5 * 2.0,
            var5 * 2.0,
        );

        for var7 in 0..16i32 {
            for var8 in 0..16i32 {
                let ni = (var7 + var8 * 16) as usize;
                let var9 = self.field_4162_p[ni] + self.hell_rng.next_double() * 0.2 > 0.0;
                let var10 = self.gravel_noise[ni] + self.hell_rng.next_double() * 0.2 > 0.0;
                let var11 =
                    (self.field_4160_r[ni] / 3.0 + 3.0 + self.hell_rng.next_double() * 0.25) as i32;
                let mut var12: i32 = -1;
                let mut var13: u8 = ids::NETHERRACK;
                let mut var14: u8 = ids::NETHERRACK;

                for var15 in (0..=127i32).rev() {
                    let var16 = ((var8 * 16 + var7) * 128 + var15) as usize;
                    // src: ChunkProviderHell.java:121-124: the top test draws
                    // first; the bottom test draws only when it fails.
                    if var15 >= 127 - self.hell_rng.next_int_bounded(5) {
                        var3[var16] = ids::BEDROCK;
                    } else if var15 <= self.hell_rng.next_int_bounded(5) {
                        var3[var16] = ids::BEDROCK;
                    } else {
                        let var17 = var3[var16];
                        if var17 == 0 {
                            var12 = -1;
                        } else if var17 == ids::NETHERRACK {
                            if var12 == -1 {
                                if var11 <= 0 {
                                    var13 = 0;
                                    var14 = ids::NETHERRACK;
                                } else if var15 >= var4 - 4 && var15 <= var4 + 1 {
                                    var13 = ids::NETHERRACK;
                                    var14 = ids::NETHERRACK;
                                    if var10 {
                                        var13 = ids::GRAVEL;
                                    }
                                    if var10 {
                                        var14 = ids::NETHERRACK;
                                    }
                                    if var9 {
                                        var13 = ids::SLOW_SAND;
                                    }
                                    if var9 {
                                        var14 = ids::SLOW_SAND;
                                    }
                                }
                                if var15 < var4 && var13 == 0 {
                                    var13 = ids::LAVA_STILL;
                                }
                                var12 = var11;
                                if var15 >= var4 - 1 {
                                    var3[var16] = var13;
                                } else {
                                    var3[var16] = var14;
                                }
                            } else if var12 > 0 {
                                var12 -= 1;
                                var3[var16] = var14;
                            }
                        }
                    }
                }
            }
        }
    }

    /// `func_4057_a(array, x, y, z, xSize, ySize, zSize)`: the nether density
    /// grid; the vertical profile is a cosine plus a cubic wall at the top
    /// and bottom four samples.
    // src: ChunkProviderHell.java:190-296
    #[allow(clippy::too_many_arguments, clippy::manual_clamp)]
    fn func_4057_a(
        &mut self,
        mut var1: Vec<f64>,
        var2: i32,
        var3: i32,
        var4: i32,
        var5: i32,
        var6: i32,
        var7: i32,
    ) -> Vec<f64> {
        if var1.is_empty() {
            var1 = vec![0.0; (var5 * var6 * var7) as usize];
        }
        let var8 = 684.412;
        let var10 = 2053.236;
        self.field_4172_f = self.field_4177_a.generate_noise_octaves(
            std::mem::take(&mut self.field_4172_f),
            var2 as f64,
            var3 as f64,
            var4 as f64,
            var5,
            1,
            var7,
            1.0,
            0.0,
            1.0,
        );
        self.field_4171_g = self.field_4176_b.generate_noise_octaves(
            std::mem::take(&mut self.field_4171_g),
            var2 as f64,
            var3 as f64,
            var4 as f64,
            var5,
            1,
            var7,
            100.0,
            0.0,
            100.0,
        );
        self.field_4175_c = self.field_4167_k.generate_noise_octaves(
            std::mem::take(&mut self.field_4175_c),
            var2 as f64,
            var3 as f64,
            var4 as f64,
            var5,
            var6,
            var7,
            var8 / 80.0,
            var10 / 60.0,
            var8 / 80.0,
        );
        self.field_4174_d = self.field_4169_i.generate_noise_octaves(
            std::mem::take(&mut self.field_4174_d),
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
        self.field_4173_e = self.field_4168_j.generate_noise_octaves(
            std::mem::take(&mut self.field_4173_e),
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
        let mut var12: usize = 0;
        let mut var13: usize = 0;
        let mut var14 = vec![0.0f64; var6 as usize];

        for var15 in 0..var6 {
            // src: ChunkProviderHell.java:208 (Math.cos, fdlibm per §3.4 rule 5)
            var14[var15 as usize] =
                jmath::cos(var15 as f64 * 3.141592653589793 * 6.0 / var6 as f64) * 2.0;
            let mut var16 = var15 as f64;
            if var15 > var6 / 2 {
                var16 = (var6 - 1 - var15) as f64;
            }
            if var16 < 4.0 {
                var16 = 4.0 - var16;
                var14[var15 as usize] -= var16 * var16 * var16 * 10.0;
            }
        }

        for _var15 in 0..var5 {
            for _var36 in 0..var7 {
                let mut var17 = (self.field_4172_f[var13] + 256.0) / 512.0;
                if var17 > 1.0 {
                    var17 = 1.0;
                }
                let var19 = 0.0;
                let mut var21 = self.field_4171_g[var13] / 8000.0;
                if var21 < 0.0 {
                    var21 = -var21;
                }
                var21 = var21 * 3.0 - 3.0;
                if var21 < 0.0 {
                    var21 /= 2.0;
                    if var21 < -1.0 {
                        var21 = -1.0;
                    }
                    var21 /= 1.4;
                    var21 /= 2.0;
                    var17 = 0.0;
                } else {
                    if var21 > 1.0 {
                        var21 = 1.0;
                    }
                    var21 /= 6.0;
                }
                var17 += 0.5;
                var21 = var21 * var6 as f64 / 16.0;
                // `var17` and `var21` are computed but never read below
                // (vanilla: the nether ignores its 2D noises).
                let _ = (var17, var21);
                var13 += 1;

                for var23 in 0..var6 {
                    let mut var24;
                    let var26 = var14[var23 as usize];
                    let var28 = self.field_4174_d[var12] / 512.0;
                    let var30 = self.field_4173_e[var12] / 512.0;
                    let var32 = (self.field_4175_c[var12] / 10.0 + 1.0) / 2.0;
                    if var32 < 0.0 {
                        var24 = var28;
                    } else if var32 > 1.0 {
                        var24 = var30;
                    } else {
                        var24 = var28 + (var30 - var28) * var32;
                    }
                    var24 -= var26;
                    if var23 > var6 - 4 {
                        // src: ChunkProviderHell.java:272 (a float division)
                        let var34 = ((var23 - (var6 - 4)) as f32 / 3.0f32) as f64;
                        var24 = var24 * (1.0 - var34) + -10.0 * var34;
                    }
                    if (var23 as f64) < var19 {
                        let mut var34 = (var19 - var23 as f64) / 4.0;
                        if var34 < 0.0 {
                            var34 = 0.0;
                        }
                        if var34 > 1.0 {
                            var34 = 1.0;
                        }
                        var24 = var24 * (1.0 - var34) + -10.0 * var34;
                    }
                    var1[var12] = var24;
                    var12 += 1;
                }
            }
        }
        var1
    }
}

impl ChunkProviderHell {
    /// `prepareChunk(x, z)`: the same as `provideChunk`. Vanilla never calls
    /// a generator's `prepareChunk` (the cache wrapper's own `prepareChunk`
    /// calls the generator's `provideChunk`), so this is not on the
    /// [`ChunkProvider`] trait.
    // src: ChunkProviderHell.java:176-178
    pub fn prepare_chunk(&mut self, world: &mut World, x: i32, z: i32) -> Chunk {
        self.provide_chunk(world, x, z)
    }

    /// `saveChunks(all, progress)`: a generator holds nothing to save. Never
    /// called by vanilla (the cache wrapper does not forward it).
    // src: ChunkProviderHell.java:361-363
    pub fn save_chunks(&mut self, _all: bool) -> bool {
        true
    }
}

impl ChunkProvider for ChunkProviderHell {
    /// `provideChunk(x, z)`: reseed `hellRNG` from the coordinates, terrain,
    /// surface, caves, `new Chunk` (no sky light pass).
    // src: ChunkProviderHell.java:180-188
    fn provide_chunk(&mut self, world: &mut World, var1: i32, var2: i32) -> Chunk {
        self.hell_rng.set_seed(
            (var1 as i64)
                .wrapping_mul(341873128712)
                .wrapping_add((var2 as i64).wrapping_mul(132897987541)),
        );
        let mut var3: Box<[u8; CHUNK_VOLUME]> = Box::new([0u8; CHUNK_VOLUME]);
        self.generate_nether_terrain(var1, var2, &mut var3);
        self.func_4058_b(var1, var2, &mut var3);
        let seed = world.get_random_seed();
        self.nether_cave_generator
            .generate(seed, var1, var2, &mut var3);
        Chunk::new(var3, var1, var2)
    }

    // src: ChunkProviderHell.java:298-300
    fn chunk_exists(&self, _x: i32, _z: i32) -> bool {
        true
    }

    // src: ChunkProviderHell.java:365-367
    fn unload_100_oldest_chunks(&mut self) -> bool {
        false
    }

    // src: ChunkProviderHell.java:369-371
    fn can_save(&self) -> bool {
        true
    }

    // src: ChunkProviderHell.java:373-375
    fn make_string(&self) -> String {
        "HellRandomLevelSource".to_string()
    }
}
