//! `NoiseGeneratorOctaves` (NoiseGeneratorOctaves.java): a sum of
//! [`NoiseGeneratorPerlin`] octaves, each at twice the frequency and half the
//! amplitude of the previous one.

use mc_core::JavaRandom;

use super::perlin::NoiseGeneratorPerlin;

/// `NoiseGeneratorOctaves`.
#[derive(Clone, Debug, PartialEq)]
pub struct NoiseGeneratorOctaves {
    /// `generatorCollection`.
    generator_collection: Vec<NoiseGeneratorPerlin>,
    /// `field_1191_b`: the number of octaves (the constructor's count; every
    /// loop runs to it).
    octaves: i32,
}

impl NoiseGeneratorOctaves {
    /// `new NoiseGeneratorOctaves(rand, octaves)`: the octaves draw from
    /// `rand` in order, each as [`NoiseGeneratorPerlin::new`].
    // src: NoiseGeneratorOctaves.java:9-17
    pub fn new(var1: &mut JavaRandom, var2: i32) -> Self {
        let mut generator_collection = Vec::with_capacity(var2 as usize);
        for _ in 0..var2 {
            generator_collection.push(NoiseGeneratorPerlin::new(var1));
        }
        NoiseGeneratorOctaves {
            generator_collection,
            octaves: var2,
        }
    }

    /// `func_806_a(x, z)`: one 2D sample, `sum(noise(x*f, z*f) / f)` for
    /// `f = 1, 1/2, 1/4, ...`.
    // src: NoiseGeneratorOctaves.java:19-29
    pub fn noise_2d(&self, var1: f64, var3: f64) -> f64 {
        let mut var5 = 0.0f64;
        let mut var7 = 1.0f64;
        for var9 in 0..self.octaves as usize {
            var5 +=
                self.generator_collection[var9].generate_noise_2d(var1 * var7, var3 * var7) / var7;
            var7 /= 2.0;
        }
        var5
    }

    /// `generateNoiseOctaves(array, x, y, z, xSize, ySize, zSize, xScale,
    /// yScale, zScale)`. `array` empty stands for Java `null` (a new array of
    /// `xSize * ySize * zSize`); otherwise the **whole** array is zeroed and
    /// reused, whatever its length.
    // src: NoiseGeneratorOctaves.java:31-48
    #[allow(clippy::too_many_arguments)]
    pub fn generate_noise_octaves(
        &self,
        mut var1: Vec<f64>,
        var2: f64,
        var4: f64,
        var6: f64,
        var8: i32,
        var9: i32,
        var10: i32,
        var11: f64,
        var13: f64,
        var15: f64,
    ) -> Vec<f64> {
        if var1.is_empty() {
            var1 = vec![0.0; (var8 * var9 * var10) as usize];
        } else {
            var1.fill(0.0);
        }
        let mut var20 = 1.0f64;
        for var19 in 0..self.octaves as usize {
            self.generator_collection[var19].populate_noise_array(
                &mut var1,
                var2,
                var4,
                var6,
                var8,
                var9,
                var10,
                var11 * var20,
                var13 * var20,
                var15 * var20,
                var20,
            );
            var20 /= 2.0;
        }
        var1
    }

    /// `func_4109_a(array, x, z, xSize, zSize, xScale, zScale, unused)`:
    /// [`Self::generate_noise_octaves`] on one y layer at `y = 10.0`, y scale
    /// 1 (which the `ySize == 1` branch ignores). The last argument is never
    /// read.
    // src: NoiseGeneratorOctaves.java:50-52
    #[allow(clippy::too_many_arguments)]
    pub fn generate_noise_octaves_2d(
        &self,
        var1: Vec<f64>,
        var2: i32,
        var3: i32,
        var4: i32,
        var5: i32,
        var6: f64,
        var8: f64,
        _var10: f64,
    ) -> Vec<f64> {
        self.generate_noise_octaves(
            var1,
            var2 as f64,
            10.0,
            var3 as f64,
            var4,
            1,
            var5,
            var6,
            1.0,
            var8,
        )
    }
}
