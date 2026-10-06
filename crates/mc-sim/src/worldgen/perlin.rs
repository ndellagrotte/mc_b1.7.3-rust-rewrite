//! `NoiseGeneratorPerlin` (NoiseGeneratorPerlin.java): improved Perlin noise
//! over a 256-entry permutation, with an origin offset drawn from the
//! generator's `Random`.
//!
//! `NoiseGenerator` (NoiseGenerator.java) is an empty abstract base class and
//! has no Rust counterpart.

use mc_core::JavaRandom;

/// `NoiseGeneratorPerlin`.
#[derive(Clone, Debug, PartialEq)]
pub struct NoiseGeneratorPerlin {
    /// `permutations` (512 entries: the shuffled 0..255 twice).
    permutations: [i32; 512],
    /// `xCoord`: origin offset added to every x.
    pub x_coord: f64,
    /// `yCoord`.
    pub y_coord: f64,
    /// `zCoord`.
    pub z_coord: f64,
}

/// The quintic fade `t³(t(6t - 15) + 10)`, written out exactly as each Java
/// site writes it.
#[inline]
fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// `(int)v`, then one less if `v` was below it (a floor that keeps Java's
/// saturating cast and wrapping decrement).
#[inline]
fn java_floor(v: f64) -> i32 {
    let i = v as i32;
    if v < i as f64 { i.wrapping_sub(1) } else { i }
}

impl NoiseGeneratorPerlin {
    /// `new NoiseGeneratorPerlin(rand)`: three `nextDouble`s for the origin,
    /// then 256 `nextInt(256 - i)` for the Fisher-Yates shuffle.
    ///
    /// The no-argument constructor `this(new Random())`
    /// (NoiseGeneratorPerlin.java:11-13) is not ported: its seed comes from
    /// the clock (D9), and no simulation code calls it.
    // src: NoiseGeneratorPerlin.java:15-33
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
        NoiseGeneratorPerlin {
            permutations,
            x_coord,
            y_coord,
            z_coord,
        }
    }

    #[inline]
    fn p(&self, i: i32) -> i32 {
        self.permutations[i as usize]
    }

    /// `generateNoise(x, y, z)`: one 3D sample.
    // src: NoiseGeneratorPerlin.java:35-70
    pub fn generate_noise(&self, var1: f64, var3: f64, var5: f64) -> f64 {
        let mut var7 = var1 + self.x_coord;
        let mut var9 = var3 + self.y_coord;
        let mut var11 = var5 + self.z_coord;
        let var13 = java_floor(var7);
        let var14 = java_floor(var9);
        let var15 = java_floor(var11);
        let var16 = var13 & 255;
        let var17 = var14 & 255;
        let var18 = var15 & 255;
        var7 -= var13 as f64;
        var9 -= var14 as f64;
        var11 -= var15 as f64;
        let var19 = fade(var7);
        let var21 = fade(var9);
        let var23 = fade(var11);
        let var25 = self.p(var16) + var17;
        let var26 = self.p(var25) + var18;
        let var27 = self.p(var25 + 1) + var18;
        let var28 = self.p(var16 + 1) + var17;
        let var29 = self.p(var28) + var18;
        let var30 = self.p(var28 + 1) + var18;
        // src: NoiseGeneratorPerlin.java:69 (argument order kept)
        self.lerp(
            var23,
            self.lerp(
                var21,
                self.lerp(
                    var19,
                    self.grad(self.p(var26), var7, var9, var11),
                    self.grad(self.p(var29), var7 - 1.0, var9, var11),
                ),
                self.lerp(
                    var19,
                    self.grad(self.p(var27), var7, var9 - 1.0, var11),
                    self.grad(self.p(var30), var7 - 1.0, var9 - 1.0, var11),
                ),
            ),
            self.lerp(
                var21,
                self.lerp(
                    var19,
                    self.grad(self.p(var26 + 1), var7, var9, var11 - 1.0),
                    self.grad(self.p(var29 + 1), var7 - 1.0, var9, var11 - 1.0),
                ),
                self.lerp(
                    var19,
                    self.grad(self.p(var27 + 1), var7, var9 - 1.0, var11 - 1.0),
                    self.grad(self.p(var30 + 1), var7 - 1.0, var9 - 1.0, var11 - 1.0),
                ),
            ),
        )
    }

    /// `lerp(t, a, b)` = `a + t * (b - a)`.
    // src: NoiseGeneratorPerlin.java:72-74
    #[inline]
    pub fn lerp(&self, var1: f64, var3: f64, var5: f64) -> f64 {
        var3 + var1 * (var5 - var3)
    }

    /// `func_4110_a(hash, x, z)`: the gradient for a sample in the y = 0
    /// plane (the 2D branch of [`Self::populate_noise_array`] uses it in place
    /// of `grad(hash, x, 0, z)` for one of its eight corners). Note
    /// `(1 - ((h & 8) >> 3)) * x` is `0.0 * x`, i.e. `-0.0` for negative `x`.
    // src: NoiseGeneratorPerlin.java:76-81
    #[inline]
    pub fn grad_2d(&self, var1: i32, var2: f64, var4: f64) -> f64 {
        let var6 = var1 & 15;
        let var7 = (1 - ((var6 & 8) >> 3)) as f64 * var2;
        let var9 = if var6 < 4 {
            0.0
        } else if var6 != 12 && var6 != 14 {
            var4
        } else {
            var2
        };
        (if (var6 & 1) == 0 { var7 } else { -var7 }) + (if (var6 & 2) == 0 { var9 } else { -var9 })
    }

    /// `grad(hash, x, y, z)`.
    // src: NoiseGeneratorPerlin.java:83-88
    #[inline]
    pub fn grad(&self, var1: i32, var2: f64, var4: f64, var6: f64) -> f64 {
        let var8 = var1 & 15;
        let var9 = if var8 < 8 { var2 } else { var4 };
        let var11 = if var8 < 4 {
            var4
        } else if var8 != 12 && var8 != 14 {
            var6
        } else {
            var2
        };
        (if (var8 & 1) == 0 { var9 } else { -var9 })
            + (if (var8 & 2) == 0 { var11 } else { -var11 })
    }

    /// `func_801_a(x, y)` = `generateNoise(x, y, 0)`.
    // src: NoiseGeneratorPerlin.java:90-92
    pub fn generate_noise_2d(&self, var1: f64, var3: f64) -> f64 {
        self.generate_noise(var1, var3, 0.0)
    }

    /// `func_805_a(array, x, y, z, xSize, ySize, zSize, xScale, yScale,
    /// zScale, amplitude)`: adds `noise / amplitude` for every sample of the
    /// grid to `array`, in x-outer, z-middle, y-inner order. With
    /// `ySize == 1` it samples the y = 0 plane (ignoring `y` and `yScale`).
    // src: NoiseGeneratorPerlin.java:94-220
    #[allow(clippy::too_many_arguments)]
    pub fn populate_noise_array(
        &self,
        var1: &mut [f64],
        var2: f64,
        var4: f64,
        var6: f64,
        var8: i32,
        var9: i32,
        var10: i32,
        var11: f64,
        var13: f64,
        var15: f64,
        var17: f64,
    ) {
        if var9 == 1 {
            // src: NoiseGeneratorPerlin.java:106-148
            let mut var75: usize = 0;
            let var77 = 1.0 / var17;
            for var30 in 0..var8 {
                let mut var31 = (var2 + var30 as f64) * var11 + self.x_coord;
                let var78 = java_floor(var31);
                let var34 = var78 & 255;
                var31 -= var78 as f64;
                let var35 = fade(var31);
                for var37 in 0..var10 {
                    let mut var38 = (var6 + var37 as f64) * var15 + self.z_coord;
                    let var40 = java_floor(var38);
                    let var41 = var40 & 255;
                    var38 -= var40 as f64;
                    let var42 = fade(var38);
                    let var19 = self.p(var34);
                    let var66 = self.p(var19) + var41;
                    let var67 = self.p(var34 + 1);
                    let var22 = self.p(var67) + var41;
                    let var70 = self.lerp(
                        var35,
                        self.grad_2d(self.p(var66), var31, var38),
                        self.grad(self.p(var22), var31 - 1.0, 0.0, var38),
                    );
                    let var73 = self.lerp(
                        var35,
                        self.grad(self.p(var66 + 1), var31, 0.0, var38 - 1.0),
                        self.grad(self.p(var22 + 1), var31 - 1.0, 0.0, var38 - 1.0),
                    );
                    let var79 = self.lerp(var42, var70, var73);
                    var1[var75] += var79 * var77;
                    var75 += 1;
                }
            }
        } else {
            // src: NoiseGeneratorPerlin.java:149-219
            let mut var19: usize = 0;
            let var20 = 1.0 / var17;
            let mut var22: i32 = -1;
            let mut var29 = 0.0f64;
            let mut var31 = 0.0f64;
            let mut var33 = 0.0f64;
            let mut var35 = 0.0f64;
            for var37 in 0..var8 {
                let mut var38 = (var2 + var37 as f64) * var11 + self.x_coord;
                let var40 = java_floor(var38);
                let var41 = var40 & 255;
                var38 -= var40 as f64;
                let var42 = fade(var38);
                for var44 in 0..var10 {
                    let mut var45 = (var6 + var44 as f64) * var15 + self.z_coord;
                    let var47 = java_floor(var45);
                    let var48 = var47 & 255;
                    var45 -= var47 as f64;
                    let var49 = fade(var45);
                    for var51 in 0..var9 {
                        let mut var52 = (var4 + var51 as f64) * var13 + self.y_coord;
                        let var54 = java_floor(var52);
                        let var55 = var54 & 255;
                        var52 -= var54 as f64;
                        let var56 = fade(var52);
                        // The four x-lerps are cached while the y lattice cell
                        // stays the same (and always recomputed at y index 0).
                        // src: NoiseGeneratorPerlin.java:196-208
                        if var51 == 0 || var55 != var22 {
                            var22 = var55;
                            let var69 = self.p(var41) + var55;
                            let var71 = self.p(var69) + var48;
                            let var72 = self.p(var69 + 1) + var48;
                            let var74 = self.p(var41 + 1) + var55;
                            let var75 = self.p(var74) + var48;
                            let var76 = self.p(var74 + 1) + var48;
                            var29 = self.lerp(
                                var42,
                                self.grad(self.p(var71), var38, var52, var45),
                                self.grad(self.p(var75), var38 - 1.0, var52, var45),
                            );
                            var31 = self.lerp(
                                var42,
                                self.grad(self.p(var72), var38, var52 - 1.0, var45),
                                self.grad(self.p(var76), var38 - 1.0, var52 - 1.0, var45),
                            );
                            var33 = self.lerp(
                                var42,
                                self.grad(self.p(var71 + 1), var38, var52, var45 - 1.0),
                                self.grad(self.p(var75 + 1), var38 - 1.0, var52, var45 - 1.0),
                            );
                            var35 = self.lerp(
                                var42,
                                self.grad(self.p(var72 + 1), var38, var52 - 1.0, var45 - 1.0),
                                self.grad(self.p(var76 + 1), var38 - 1.0, var52 - 1.0, var45 - 1.0),
                            );
                        }
                        let var58 = self.lerp(var56, var29, var31);
                        let var60 = self.lerp(var56, var33, var35);
                        let var62 = self.lerp(var49, var58, var60);
                        var1[var19] += var62 * var20;
                        var19 += 1;
                    }
                }
            }
        }
    }
}
