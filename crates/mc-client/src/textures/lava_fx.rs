//! `TextureLavaFX` (TextureLavaFX.java): the still lava tile
//! (`Block.lavaMoving.blockIndexInTexture`, 237). A wrapped 16×16 heat field
//! blurred along a sine-shifted 3×3 kernel, fed by a decaying source field
//! that `Math.random()` re-ignites.

use mc_core::java_random::JavaRandom;
use mc_core::math;

use super::texture_fx::{FxKind, LAVA_INDEX, TextureFx, anaglyph};

/// `TextureLavaFX`'s fields.
#[derive(Clone, Debug)]
pub struct TextureLavaFx {
    /// `field_1147_g`: the heat shown.
    // src: TextureLavaFX.java:4
    pub heat: [f32; 256],
    /// `field_1146_h`: the next heat (swapped in after each tick).
    // src: TextureLavaFX.java:5
    pub heat_next: [f32; 256],
    /// `field_1145_i`: the heat sources, fed by `heat_delta`.
    // src: TextureLavaFX.java:6
    pub source: [f32; 256],
    /// `field_1144_j`: the sources' rate, -0.06 a tick, reset to 1.5 at
    /// random.
    // src: TextureLavaFX.java:7
    pub source_delta: [f32; 256],
}

impl TextureLavaFx {
    /// `new TextureLavaFX()`.
    // src: TextureLavaFX.java:9-11
    pub fn new_fx() -> TextureFx {
        TextureFx::new(
            LAVA_INDEX,
            FxKind::Lava(Box::new(TextureLavaFx {
                heat: [0.0; 256],
                heat_next: [0.0; 256],
                source: [0.0; 256],
                source_delta: [0.0; 256],
            })),
        )
    }

    /// `onTick()`.
    // src: TextureLavaFX.java:13-82
    pub fn on_tick(
        &mut self,
        image: &mut [u8; 1024],
        anaglyph_enabled: bool,
        rnd: &mut JavaRandom,
    ) {
        lava_step(
            &mut self.heat,
            &mut self.heat_next,
            &mut self.source,
            &mut self.source_delta,
            rnd,
        );
        for var2 in 0..256usize {
            lava_pixel(image, var2, self.heat[var2], anaglyph_enabled);
        }
    }
}

/// The field update shared, line for line, by `TextureLavaFX` (:21-52) and
/// `TextureLavaFlowFX` (:25-56); the arrays are swapped at the end.
// src: TextureLavaFX.java:21-52, TextureLavaFlowFX.java:25-56
pub(super) fn lava_step(
    heat: &mut [f32; 256],
    heat_next: &mut [f32; 256],
    source: &mut [f32; 256],
    source_delta: &mut [f32; 256],
    rnd: &mut JavaRandom,
) {
    for var1 in 0..16i32 {
        for var2 in 0..16i32 {
            let mut var3 = 0.0f32;
            let var4 = (math::sin(var2 as f32 * 3.1415927f32 * 2.0f32 / 16.0f32) * 1.2f32) as i32;
            let var5 = (math::sin(var1 as f32 * 3.1415927f32 * 2.0f32 / 16.0f32) * 1.2f32) as i32;
            for var6 in var1 - 1..=var1 + 1 {
                for var7 in var2 - 1..=var2 + 1 {
                    let var8 = (var6 + var4) & 15;
                    let var9 = (var7 + var5) & 15;
                    var3 += heat[(var8 + var9 * 16) as usize];
                }
            }
            let at = |x: i32, y: i32| ((x & 15) + (y & 15) * 16) as usize;
            let i = (var1 + var2 * 16) as usize;
            heat_next[i] = var3 / 10.0f32
                + (source[at(var1, var2)]
                    + source[at(var1 + 1, var2)]
                    + source[at(var1 + 1, var2 + 1)]
                    + source[at(var1, var2 + 1)])
                    / 4.0f32
                    * 0.8f32;
            source[i] += source_delta[i] * 0.01f32;
            if source[i] < 0.0 {
                source[i] = 0.0;
            }
            source_delta[i] -= 0.06f32;
            if rnd.next_double() < 0.005 {
                source_delta[i] = 1.5f32;
            }
        }
    }
    std::mem::swap(heat, heat_next);
}

/// One pixel of a lava tile from its heat: `var3 = heat * 2` clamped to
/// [0, 1]; red `var3*100 + 155`, green `var3² * 255`, blue `var3⁴ * 128`,
/// opaque.
// src: TextureLavaFX.java:55-79, TextureLavaFlowFX.java:59-83
#[allow(clippy::manual_clamp)] // Java's two ifs, kept
pub(super) fn lava_pixel(image: &mut [u8; 1024], var2: usize, heat: f32, anaglyph_enabled: bool) {
    let mut var3 = heat * 2.0f32;
    if var3 > 1.0 {
        var3 = 1.0;
    }
    if var3 < 0.0 {
        var3 = 0.0;
    }
    let mut var5 = (var3 * 100.0f32 + 155.0f32) as i32;
    let mut var6 = (var3 * var3 * 255.0f32) as i32;
    let mut var7 = (var3 * var3 * var3 * var3 * 128.0f32) as i32;
    if anaglyph_enabled {
        (var5, var6, var7) = anaglyph(var5, var6, var7);
    }
    image[var2 * 4] = var5 as u8;
    image[var2 * 4 + 1] = var6 as u8;
    image[var2 * 4 + 2] = var7 as u8;
    image[var2 * 4 + 3] = 255; // (byte)-1
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hand-checked: `(int)(MathHelper.sin(i * π * 2 / 16) * 1.2F)` is the
    /// kernel shift: 1 where sin > 1/1.2 (i = 3, 4, 5), -1 where sin <
    /// -1/1.2 (11, 12, 13), else 0.
    #[test]
    fn kernel_shift() {
        let shift: Vec<i32> = (0..16)
            .map(|i| (math::sin(i as f32 * 3.1415927f32 * 2.0f32 / 16.0f32) * 1.2f32) as i32)
            .collect();
        assert_eq!(shift, [0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, -1, -1, -1, 0, 0]);
    }

    /// Hand-checked: an all-zero field gives the darkest lava,
    /// `(155, 0, 0, 255)`, and each cell draws once from the stream, in
    /// x-major order (the Java loops are `var1` = x outer, `var2` = y inner).
    #[test]
    fn first_tick_colour_and_draws() {
        let mut fx = TextureLavaFx::new_fx();
        let mut rnd = JavaRandom::new(7);
        fx.on_tick(&mut rnd, None);
        for p in fx.image_data.chunks(4) {
            assert_eq!(p, [155, 0, 0, 255]);
        }
        let mut check = JavaRandom::new(7);
        let FxKind::Lava(l) = &fx.kind else {
            unreachable!()
        };
        for var1 in 0..16 {
            for var2 in 0..16 {
                let want = if check.next_double() < 0.005 {
                    1.5
                } else {
                    -0.06f32
                };
                assert_eq!(l.source_delta[var1 + var2 * 16], want);
            }
        }
        assert_eq!(rnd.state(), check.state());
    }

    /// Hand-checked: full heat gives `(255, 255, 128, 255)`; half heat
    /// (`var3` = 0.5 after doubling 0.25) gives `(205, 63, 8, 255)`.
    #[test]
    fn pixel_formula() {
        let mut img = [0u8; 1024];
        lava_pixel(&mut img, 0, 1.0, false);
        assert_eq!(&img[0..4], [255, 255, 128, 255]);
        lava_pixel(&mut img, 1, 0.25, false);
        assert_eq!(&img[4..8], [205, 63, 8, 255]);
    }

    /// Regression checksum of the first 100 frames (seed 99). Taken from
    /// this implementation, not from Java (no client oracle).
    #[test]
    fn checksum_regression() {
        let mut fx = TextureLavaFx::new_fx();
        let mut rnd = JavaRandom::new(99);
        let mut sum = crate::textures::tests::Fnv::default();
        for _ in 0..100 {
            fx.on_tick(&mut rnd, None);
            sum.add(&fx.image_data);
        }
        assert_eq!(sum.0, crate::textures::tests::LAVA_CHECKSUM, "{:#x}", sum.0);
    }
}
