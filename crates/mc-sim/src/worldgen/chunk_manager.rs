//! `WorldChunkManager` (WorldChunkManager.java) and `WorldChunkManagerHell`
//! (WorldChunkManagerHell.java): biomes, temperature and humidity.
//!
//! One struct with the shared public fields (`temperature`, `humidity`,
//! `field_4196_c`, `biomeGenBase`) and a [`ChunkManagerKind`] for the two
//! classes. Java arrays that may be `null` are `Vec`s, empty standing for
//! `null` (vanilla never creates an empty one).
//!
//! The arrays are **shared scratch**: `ChunkProviderGenerate` reads
//! `temperature` and `humidity` right after its own `loadBlockGeneratorData`
//! call, and every `getBiomeGenAt` (1×1) overwrites their first entry.
//! Callers therefore go through the one manager on the world
//! ([`crate::world::World::get_world_chunk_manager`]), as in vanilla.

use mc_core::JavaRandom;

use super::biome::{Biome, get_biome_from_lookup};
use super::simplex::NoiseGeneratorOctaves2;

/// The `WorldChunkManager` subclass.
#[derive(Clone, Debug, PartialEq)]
pub enum ChunkManagerKind {
    /// `WorldChunkManager`: three simplex octave sums.
    Noise {
        /// `field_4194_e`: temperature noise (4 octaves, seed × 9871).
        temperature_noise: Box<NoiseGeneratorOctaves2>,
        /// `field_4193_f`: humidity noise (4 octaves, seed × 39811).
        humidity_noise: Box<NoiseGeneratorOctaves2>,
        /// `field_4192_g`: the noise behind `field_4196_c` (2 octaves, seed ×
        /// 543321), blended into both temperature and humidity.
        field_4192_g: Box<NoiseGeneratorOctaves2>,
    },
    /// `WorldChunkManagerHell`: one biome, constant temperature and humidity.
    Hell {
        /// `field_4201_e`: the biome.
        biome: Biome,
        /// `field_4200_f`: the temperature.
        temperature: f64,
        /// `field_4199_g`: the humidity.
        humidity: f64,
    },
}

/// `WorldChunkManager` / `WorldChunkManagerHell`.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldChunkManager {
    /// Which class, with its private fields.
    pub kind: ChunkManagerKind,
    /// `temperature`.
    pub temperature: Vec<f64>,
    /// `humidity`.
    pub humidity: Vec<f64>,
    /// `field_4196_c`: the last `field_4192_g` noise array (the blend term).
    pub field_4196_c: Vec<f64>,
    /// `biomeGenBase`: the array `getBiomeGenBaseAt` returns.
    pub biome_gen_base: Vec<Biome>,
}

impl WorldChunkManager {
    /// `new WorldChunkManager(world)`: three generators from three fresh
    /// `Random`s keyed off the world seed.
    // src: WorldChunkManager.java:17-21
    pub fn new(random_seed: i64) -> Self {
        WorldChunkManager {
            kind: ChunkManagerKind::Noise {
                temperature_noise: Box::new(NoiseGeneratorOctaves2::new(
                    &mut JavaRandom::new(random_seed.wrapping_mul(9871)),
                    4,
                )),
                humidity_noise: Box::new(NoiseGeneratorOctaves2::new(
                    &mut JavaRandom::new(random_seed.wrapping_mul(39811)),
                    4,
                )),
                field_4192_g: Box::new(NoiseGeneratorOctaves2::new(
                    &mut JavaRandom::new(random_seed.wrapping_mul(543321)),
                    2,
                )),
            },
            temperature: Vec::new(),
            humidity: Vec::new(),
            field_4196_c: Vec::new(),
            biome_gen_base: Vec::new(),
        }
    }

    /// `new WorldChunkManagerHell(biome, temperature, humidity)`.
    // src: WorldChunkManagerHell.java:10-14
    pub fn new_hell(biome: Biome, temperature: f64, humidity: f64) -> Self {
        WorldChunkManager {
            kind: ChunkManagerKind::Hell {
                biome,
                temperature,
                humidity,
            },
            temperature: Vec::new(),
            humidity: Vec::new(),
            field_4196_c: Vec::new(),
            biome_gen_base: Vec::new(),
        }
    }

    /// `getBiomeGenAtChunkCoord(pair)`: the biome at the chunk's (0, 0) block.
    // src: WorldChunkManager.java:23-25, WorldChunkManagerHell.java:16-18
    pub fn get_biome_gen_at_chunk_coord(&mut self, chunk_x: i32, chunk_z: i32) -> Biome {
        if let ChunkManagerKind::Hell { biome, .. } = self.kind {
            return biome;
        }
        self.get_biome_gen_at(chunk_x << 4, chunk_z << 4)
    }

    /// `getBiomeGenAt(x, z)` = `getBiomeGenBaseAt(x, z, 1, 1)[0]`.
    // src: WorldChunkManager.java:27-29, WorldChunkManagerHell.java:20-22
    pub fn get_biome_gen_at(&mut self, var1: i32, var2: i32) -> Biome {
        if let ChunkManagerKind::Hell { biome, .. } = self.kind {
            return biome;
        }
        self.get_biome_gen_base_at(var1, var2, 1, 1)[0]
    }

    /// `getTemperature(x, z)`. The noise version stores the **raw** 1×1
    /// noise (not the normalised temperature) in `temperature[0]`.
    // src: WorldChunkManager.java:31-34, WorldChunkManagerHell.java:24-26
    pub fn get_temperature(&mut self, var1: i32, var2: i32) -> f64 {
        match &self.kind {
            ChunkManagerKind::Hell { temperature, .. } => *temperature,
            ChunkManagerKind::Noise {
                temperature_noise, ..
            } => {
                self.temperature = temperature_noise.func_4112_a(
                    std::mem::take(&mut self.temperature),
                    var1 as f64,
                    var2 as f64,
                    1,
                    1,
                    0.02500000037252903,
                    0.02500000037252903,
                    0.5,
                );
                self.temperature[0]
            }
        }
    }

    /// `getBiomeGenBaseAt(x, z, w, h)`: [`Self::load_block_generator_data`]
    /// into the manager's own `biomeGenBase` array.
    // src: WorldChunkManager.java:36-39, WorldChunkManagerHell.java:28-31
    pub fn get_biome_gen_base_at(
        &mut self,
        var1: i32,
        var2: i32,
        var3: i32,
        var4: i32,
    ) -> &[Biome] {
        let arr = std::mem::take(&mut self.biome_gen_base);
        self.biome_gen_base = self.load_block_generator_data(arr, var1, var2, var3, var4);
        &self.biome_gen_base
    }

    /// `getTemperatures(array, x, z, w, h)`: normalised temperatures (without
    /// the humidity), into `array` (replaced if shorter than `w * h`). Updates
    /// `field_4196_c`.
    // src: WorldChunkManager.java:41-71, WorldChunkManagerHell.java:33-40
    #[allow(clippy::manual_clamp)] // Java's two ifs (NaN passes through either way)
    pub fn get_temperatures(
        &mut self,
        mut var1: Vec<f64>,
        var2: i32,
        var3: i32,
        var4: i32,
        var5: i32,
    ) -> Vec<f64> {
        let n = var4.wrapping_mul(var5);
        if var1.is_empty() || (var1.len() as i64) < n as i64 {
            var1 = vec![0.0; n as usize];
        }
        match &self.kind {
            ChunkManagerKind::Hell { temperature, .. } => {
                var1[..n as usize].fill(*temperature);
                var1
            }
            ChunkManagerKind::Noise {
                temperature_noise,
                field_4192_g,
                ..
            } => {
                var1 = temperature_noise.func_4112_a(
                    var1,
                    var2 as f64,
                    var3 as f64,
                    var4,
                    var5,
                    0.02500000037252903,
                    0.02500000037252903,
                    0.25,
                );
                self.field_4196_c = field_4192_g.func_4112_a(
                    std::mem::take(&mut self.field_4196_c),
                    var2 as f64,
                    var3 as f64,
                    var4,
                    var5,
                    0.25,
                    0.25,
                    0.5882352941176471,
                );
                let mut var6 = 0usize;
                for _var7 in 0..var4 {
                    for _var8 in 0..var5 {
                        let var9 = self.field_4196_c[var6] * 1.1 + 0.5;
                        let var11 = 0.01;
                        let var13 = 1.0 - var11;
                        let mut var15 = (var1[var6] * 0.15 + 0.7) * var13 + var9 * var11;
                        var15 = 1.0 - (1.0 - var15) * (1.0 - var15);
                        if var15 < 0.0 {
                            var15 = 0.0;
                        }
                        if var15 > 1.0 {
                            var15 = 1.0;
                        }
                        var1[var6] = var15;
                        var6 += 1;
                    }
                }
                var1
            }
        }
    }

    /// `loadBlockGeneratorData(array, x, z, w, h)`: biomes into `array`
    /// (replaced if shorter than `w * h`; empty = `null`), normalised
    /// temperature and humidity into the manager's `temperature` and
    /// `humidity`, index `i * h + j` for `i` over x and `j` over z.
    ///
    /// Vanilla bug, transcribed: the three noise arrays are generated as
    /// `w × w` (the width passed twice, WorldChunkManager.java:78-80) while
    /// the loop runs over `w × h`; every vanilla caller passes a square.
    // src: WorldChunkManager.java:73-116, WorldChunkManagerHell.java:42-56
    #[allow(clippy::manual_clamp)] // Java's ifs (NaN passes through either way)
    pub fn load_block_generator_data(
        &mut self,
        mut var1: Vec<Biome>,
        var2: i32,
        var3: i32,
        var4: i32,
        var5: i32,
    ) -> Vec<Biome> {
        let n = var4.wrapping_mul(var5);
        if var1.is_empty() || (var1.len() as i64) < n as i64 {
            // Java fills a new array with nulls; every entry below n is set.
            var1 = vec![Biome::Tundra; n as usize];
        }
        match &self.kind {
            ChunkManagerKind::Hell {
                biome,
                temperature,
                humidity,
            } => {
                if self.temperature.is_empty() || (self.temperature.len() as i64) < n as i64 {
                    self.temperature = vec![0.0; n as usize];
                    self.humidity = vec![0.0; n as usize];
                }
                var1[..n as usize].fill(*biome);
                self.humidity[..n as usize].fill(*humidity);
                self.temperature[..n as usize].fill(*temperature);
                var1
            }
            ChunkManagerKind::Noise {
                temperature_noise,
                humidity_noise,
                field_4192_g,
            } => {
                // vanilla bug: `var4, var4` (width twice), see the doc comment.
                // src: WorldChunkManager.java:78-80
                self.temperature = temperature_noise.func_4112_a(
                    std::mem::take(&mut self.temperature),
                    var2 as f64,
                    var3 as f64,
                    var4,
                    var4,
                    0.02500000037252903,
                    0.02500000037252903,
                    0.25,
                );
                self.humidity = humidity_noise.func_4112_a(
                    std::mem::take(&mut self.humidity),
                    var2 as f64,
                    var3 as f64,
                    var4,
                    var4,
                    0.05000000074505806,
                    0.05000000074505806,
                    0.3333333333333333,
                );
                self.field_4196_c = field_4192_g.func_4112_a(
                    std::mem::take(&mut self.field_4196_c),
                    var2 as f64,
                    var3 as f64,
                    var4,
                    var4,
                    0.25,
                    0.25,
                    0.5882352941176471,
                );
                let mut var6 = 0usize;
                for _var7 in 0..var4 {
                    for _var8 in 0..var5 {
                        // src: WorldChunkManager.java:85-111
                        let var9 = self.field_4196_c[var6] * 1.1 + 0.5;
                        let mut var11 = 0.01;
                        let mut var13 = 1.0 - var11;
                        let mut var15 =
                            (self.temperature[var6] * 0.15 + 0.7) * var13 + var9 * var11;
                        var11 = 0.002;
                        var13 = 1.0 - var11;
                        let mut var17 = (self.humidity[var6] * 0.15 + 0.5) * var13 + var9 * var11;
                        var15 = 1.0 - (1.0 - var15) * (1.0 - var15);
                        if var15 < 0.0 {
                            var15 = 0.0;
                        }
                        if var17 < 0.0 {
                            var17 = 0.0;
                        }
                        if var15 > 1.0 {
                            var15 = 1.0;
                        }
                        if var17 > 1.0 {
                            var17 = 1.0;
                        }
                        self.temperature[var6] = var15;
                        self.humidity[var6] = var17;
                        var1[var6] = get_biome_from_lookup(var15, var17);
                        var6 += 1;
                    }
                }
                var1
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The noise is pointwise (`(x0 + i) * scale` with integers exact in f64),
    /// so the 1×1 queries `populate` makes must agree with the 16×16 array
    /// the `biome` fixtures check against the oracle.
    #[test]
    fn point_queries_agree_with_the_chunk_array() {
        let seed = -4172144991579068934;
        let (x0, z0) = (-48, 112);
        let mut grid = WorldChunkManager::new(seed);
        let biomes = grid.load_block_generator_data(Vec::new(), x0, z0, 16, 16);
        let temps = grid.temperature.clone();
        let mut point = WorldChunkManager::new(seed);
        for i in 0..16 {
            for j in 0..16 {
                let k = (i * 16 + j) as usize;
                assert_eq!(point.get_biome_gen_at(x0 + i, z0 + j), biomes[k]);
                assert_eq!(point.temperature[0].to_bits(), temps[k].to_bits());
            }
        }
        // getTemperatures is loadBlockGeneratorData's temperature without
        // the humidity.
        let t = point.get_temperatures(Vec::new(), x0, z0, 16, 16);
        assert!(
            t.iter()
                .zip(&temps)
                .all(|(a, b)| a.to_bits() == b.to_bits())
        );
        // A longer array is reused, not replaced; `func_4111_a` zeroes all of
        // it and only the first 256 entries are written.
        let t = point.get_temperatures(vec![7.0; 300], x0, z0, 16, 16);
        assert_eq!(t.len(), 300);
        assert!(t[256..].iter().all(|v| *v == 0.0));
    }

    #[test]
    fn hell_manager_is_constant() {
        let mut m = WorldChunkManager::new_hell(Biome::Hell, 1.0, 0.0);
        assert_eq!(m.get_biome_gen_at(123, -456), Biome::Hell);
        assert_eq!(m.get_temperature(0, 0), 1.0);
        let b = m.load_block_generator_data(Vec::new(), 0, 0, 16, 16);
        assert!(b.iter().all(|b| *b == Biome::Hell));
        assert!(m.temperature.iter().all(|t| *t == 1.0));
        assert!(m.humidity.iter().all(|h| *h == 0.0));
        let t = m.get_temperatures(Vec::new(), 5, 5, 4, 4);
        assert_eq!(t, vec![1.0; 16]);
    }
}
