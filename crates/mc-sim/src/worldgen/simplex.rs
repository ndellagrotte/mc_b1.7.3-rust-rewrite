//! `NoiseGenerator2` (NoiseGenerator2.java): 2D simplex noise, and
//! `NoiseGeneratorOctaves2` (NoiseGeneratorOctaves2.java), its octave sum.
//! Only the biome generator ([`super::chunk_manager`]) uses them.

use mc_core::JavaRandom;
use mc_core::jmath;

/// `field_4296_d`: the twelve 3D gradient directions; only the first two
/// components are read.
// src: NoiseGenerator2.java:6
const GRAD3: [[i32; 3]; 12] = [
    [1, 1, 0],
    [-1, 1, 0],
    [1, -1, 0],
    [-1, -1, 0],
    [1, 0, 1],
    [-1, 0, 1],
    [1, 0, -1],
    [-1, 0, -1],
    [0, 1, 1],
    [0, -1, 1],
    [0, 1, -1],
    [0, -1, -1],
];

/// `field_4294_f` = `0.5 * (Math.sqrt(3.0) - 1.0)`, the 2D skew factor
/// (`sqrt` is correctly rounded, so this is the Java value).
// src: NoiseGenerator2.java:11
#[inline]
fn skew_f2() -> f64 {
    0.5 * (jmath::sqrt(3.0) - 1.0)
}

/// `field_4293_g` = `(3.0 - Math.sqrt(3.0)) / 6.0`, the 2D unskew factor.
// src: NoiseGenerator2.java:12
#[inline]
fn unskew_g2() -> f64 {
    (3.0 - jmath::sqrt(3.0)) / 6.0
}

/// `NoiseGenerator2`.
#[derive(Clone, Debug, PartialEq)]
pub struct NoiseGenerator2 {
    /// `field_4295_e`: the permutation table (shuffled 0..255 twice), as
    /// `NoiseGeneratorPerlin.permutations`.
    permutations: [i32; 512],
    /// `field_4292_a`: x origin offset.
    pub x_coord: f64,
    /// `field_4291_b`: y origin offset (the second input coordinate).
    pub y_coord: f64,
    /// `field_4297_c`: drawn but never read.
    pub z_coord: f64,
}

impl NoiseGenerator2 {
    /// `new NoiseGenerator2(rand)`: the same draws as `NoiseGeneratorPerlin`.
    ///
    /// The no-argument constructor `this(new Random())`
    /// (NoiseGenerator2.java:14-16) is not ported: its seed comes from the
    /// clock (D9), and no simulation code calls it.
    // src: NoiseGenerator2.java:18-36
    pub fn new(var1: &mut JavaRandom) -> Self {
        let mut permutations = [0i32; 512];
        let x_coord = var1.next_double() * 256.0;
        let y_coord = var1.next_double() * 256.0;
        let z_coord = var1.next_double() * 256.0;
        for (var2, p) in permutations.iter_mut().enumerate().take(256) {
            *p = var2 as i32;
        }
        for var2 in 0..256usize {
            let var3 = (var1.next_int_bounded(256 - var2 as i32) + var2 as i32) as usize;
            // `var4 = p[i]; p[i] = p[j]; p[j] = var4;`
            permutations.swap(var2, var3);
            permutations[var2 + 256] = permutations[var2];
        }
        NoiseGenerator2 {
            permutations,
            x_coord,
            y_coord,
            z_coord,
        }
    }

    /// `wrap(d)`: `d > 0 ? (int)d : (int)d - 1`. Vanilla quirk, transcribed:
    /// not a floor for non-positive integers (`wrap(0.0) == -1`,
    /// `wrap(-2.0) == -3`).
    // src: NoiseGenerator2.java:38-40
    #[inline]
    fn wrap(var0: f64) -> i32 {
        if var0 > 0.0 {
            var0 as i32
        } else {
            (var0 as i32).wrapping_sub(1)
        }
    }

    /// `func_4156_a(grad, x, y)`: `grad[0] * x + grad[1] * y`.
    // src: NoiseGenerator2.java:42-44
    #[inline]
    fn dot2(var0: &[i32; 3], var1: f64, var3: f64) -> f64 {
        var0[0] as f64 * var1 + var0[1] as f64 * var3
    }

    #[inline]
    fn p(&self, i: i32) -> i32 {
        self.permutations[i as usize]
    }

    /// `func_4157_a(array, x, y, xSize, ySize, xScale, yScale, amplitude)`:
    /// adds `70 * noise * amplitude` for every sample of the grid to `array`,
    /// x outer, y inner.
    // src: NoiseGenerator2.java:46-113
    #[allow(clippy::too_many_arguments)]
    pub fn add_noise(
        &self,
        var1: &mut [f64],
        var2: f64,
        var4: f64,
        var6: i32,
        var7: i32,
        var8: f64,
        var10: f64,
        var12: f64,
    ) {
        let f2 = skew_f2();
        let g2 = unskew_g2();
        let mut var14: usize = 0;
        for var15 in 0..var6 {
            let var16 = (var2 + var15 as f64) * var8 + self.x_coord;
            for var18 in 0..var7 {
                let var19 = (var4 + var18 as f64) * var10 + self.y_coord;
                let var27 = (var16 + var19) * f2;
                let var29 = Self::wrap(var16 + var27);
                let var30 = Self::wrap(var19 + var27);
                let var31 = var29.wrapping_add(var30) as f64 * g2;
                let var33 = var29 as f64 - var31;
                let var35 = var30 as f64 - var31;
                let var37 = var16 - var33;
                let var39 = var19 - var35;
                let (var41, var42): (i32, i32) = if var37 > var39 { (1, 0) } else { (0, 1) };
                let var43 = var37 - var41 as f64 + g2;
                let var45 = var39 - var42 as f64 + g2;
                let var47 = var37 - 1.0 + 2.0 * g2;
                let var49 = var39 - 1.0 + 2.0 * g2;
                let var51 = var29 & 255;
                let var52 = var30 & 255;
                let var53 = self.p(var51 + self.p(var52)) % 12;
                let var54 = self.p(var51 + var41 + self.p(var52 + var42)) % 12;
                let var55 = self.p(var51 + 1 + self.p(var52 + 1)) % 12;
                let mut var56 = 0.5 - var37 * var37 - var39 * var39;
                let var21 = if var56 < 0.0 {
                    0.0
                } else {
                    var56 *= var56;
                    var56 * var56 * Self::dot2(&GRAD3[var53 as usize], var37, var39)
                };
                let mut var58 = 0.5 - var43 * var43 - var45 * var45;
                let var23 = if var58 < 0.0 {
                    0.0
                } else {
                    var58 *= var58;
                    var58 * var58 * Self::dot2(&GRAD3[var54 as usize], var43, var45)
                };
                let mut var60 = 0.5 - var47 * var47 - var49 * var49;
                let var25 = if var60 < 0.0 {
                    0.0
                } else {
                    var60 *= var60;
                    var60 * var60 * Self::dot2(&GRAD3[var55 as usize], var47, var49)
                };
                var1[var14] += 70.0 * (var21 + var23 + var25) * var12;
                var14 += 1;
            }
        }
    }
}

/// `NoiseGeneratorOctaves2`.
#[derive(Clone, Debug, PartialEq)]
pub struct NoiseGeneratorOctaves2 {
    /// `field_4234_a`: the octaves.
    generators: Vec<NoiseGenerator2>,
    /// `field_4233_b`: the number of octaves.
    octaves: i32,
}

impl NoiseGeneratorOctaves2 {
    /// `new NoiseGeneratorOctaves2(rand, octaves)`.
    // src: NoiseGeneratorOctaves2.java:9-17
    pub fn new(var1: &mut JavaRandom, var2: i32) -> Self {
        let mut generators = Vec::with_capacity(var2 as usize);
        for _ in 0..var2 {
            generators.push(NoiseGenerator2::new(var1));
        }
        NoiseGeneratorOctaves2 {
            generators,
            octaves: var2,
        }
    }

    /// `func_4112_a(array, x, y, xSize, ySize, xScale, yScale, frequencyStep)`
    /// = [`Self::func_4111_a`] with amplitude step 0.5.
    // src: NoiseGeneratorOctaves2.java:19-21
    #[allow(clippy::too_many_arguments)]
    pub fn func_4112_a(
        &self,
        var1: Vec<f64>,
        var2: f64,
        var4: f64,
        var6: i32,
        var7: i32,
        var8: f64,
        var10: f64,
        var12: f64,
    ) -> Vec<f64> {
        self.func_4111_a(var1, var2, var4, var6, var7, var8, var10, var12, 0.5)
    }

    /// `func_4111_a(array, x, y, xSize, ySize, xScale, yScale, frequencyStep,
    /// amplitudeStep)`: scales divided by 1.5; octave `i` samples at
    /// `scale * frequencyStep^i` with amplitude `0.55 / amplitudeStep^i`.
    /// The array is reused (and **wholly** zeroed) when it holds at least
    /// `xSize * ySize` entries, else replaced (empty = Java `null`).
    // src: NoiseGeneratorOctaves2.java:23-44
    #[allow(clippy::too_many_arguments)]
    pub fn func_4111_a(
        &self,
        mut var1: Vec<f64>,
        var2: f64,
        var4: f64,
        var6: i32,
        var7: i32,
        mut var8: f64,
        mut var10: f64,
        var12: f64,
        var14: f64,
    ) -> Vec<f64> {
        var8 /= 1.5;
        var10 /= 1.5;
        let n = var6.wrapping_mul(var7);
        if !var1.is_empty() && var1.len() as i64 >= n as i64 {
            var1.fill(0.0);
        } else {
            var1 = vec![0.0; n as usize];
        }
        let mut var21 = 1.0f64;
        let mut var18 = 1.0f64;
        for var20 in 0..self.octaves as usize {
            self.generators[var20].add_noise(
                &mut var1,
                var2,
                var4,
                var6,
                var7,
                var8 * var18,
                var10 * var18,
                0.55 / var21,
            );
            var18 *= var12;
            var21 *= var14;
        }
        var1
    }
}
