//! `TextureWaterFX` (TextureWaterFX.java): the still water tile
//! (`Block.waterMoving.blockIndexInTexture`, 205). A wrapped field blurred
//! horizontally, fed by sources that `Math.random()` pulses.

use mc_core::java_random::JavaRandom;

use super::texture_fx::{FxKind, TextureFx, WATER_INDEX, anaglyph};

/// `TextureWaterFX`'s fields.
#[derive(Clone, Debug)]
pub struct TextureWaterFx {
    /// `field_1158_g`: the field shown.
    // src: TextureWaterFX.java:4
    pub height: [f32; 256],
    /// `field_1157_h`: the next field.
    // src: TextureWaterFX.java:5
    pub height_next: [f32; 256],
    /// `field_1156_i`: the sources.
    // src: TextureWaterFX.java:6
    pub source: [f32; 256],
    /// `field_1155_j`: the sources' rate, -0.1 a tick, reset to 0.5 at
    /// random.
    // src: TextureWaterFX.java:7
    pub source_delta: [f32; 256],
    /// `tickCounter`: counted, never read.
    // src: TextureWaterFX.java:8
    pub tick_counter: i32,
}

impl TextureWaterFx {
    /// `new TextureWaterFX()`.
    // src: TextureWaterFX.java:10-12
    pub fn new_fx() -> TextureFx {
        TextureFx::new(
            WATER_INDEX,
            FxKind::Water(Box::new(TextureWaterFx {
                height: [0.0; 256],
                height_next: [0.0; 256],
                source: [0.0; 256],
                source_delta: [0.0; 256],
                tick_counter: 0,
            })),
        )
    }

    /// `onTick()`.
    // src: TextureWaterFX.java:14-86
    pub fn on_tick(
        &mut self,
        image: &mut [u8; 1024],
        anaglyph_enabled: bool,
        rnd: &mut JavaRandom,
    ) {
        self.tick_counter = self.tick_counter.wrapping_add(1);
        for var1 in 0..16i32 {
            for var2 in 0..16i32 {
                let mut var3 = 0.0f32;
                for var4 in var1 - 1..=var1 + 1 {
                    let var5 = var4 & 15;
                    let var6 = var2 & 15;
                    var3 += self.height[(var5 + var6 * 16) as usize];
                }
                let i = (var1 + var2 * 16) as usize;
                self.height_next[i] = var3 / 3.3f32 + self.source[i] * 0.8f32;
            }
        }
        for var1 in 0..16usize {
            for var2 in 0..16usize {
                let i = var1 + var2 * 16;
                self.source[i] += self.source_delta[i] * 0.05f32;
                if self.source[i] < 0.0 {
                    self.source[i] = 0.0;
                }
                self.source_delta[i] -= 0.1f32;
                if rnd.next_double() < 0.05 {
                    self.source_delta[i] = 0.5f32;
                }
            }
        }
        std::mem::swap(&mut self.height, &mut self.height_next);
        for var2 in 0..256usize {
            water_pixel(image, var2, self.height[var2], anaglyph_enabled);
        }
    }
}

/// One pixel of a water tile: `var3` clamped to [0, 1], `var13 = var3²`;
/// red `32 + var13*32`, green `50 + var13*64`, blue 255, alpha
/// `146 + var13*50`.
// src: TextureWaterFX.java:57-83, TextureWaterFlowFX.java:58-84
#[allow(clippy::manual_clamp)] // Java's two ifs, kept
pub(super) fn water_pixel(image: &mut [u8; 1024], var2: usize, value: f32, anaglyph_enabled: bool) {
    let mut var3 = value;
    if var3 > 1.0 {
        var3 = 1.0;
    }
    if var3 < 0.0 {
        var3 = 0.0;
    }
    let var13 = var3 * var3;
    let mut var5 = (32.0f32 + var13 * 32.0f32) as i32;
    let mut var6 = (50.0f32 + var13 * 64.0f32) as i32;
    let mut var7 = 255i32;
    let var8 = (146.0f32 + var13 * 50.0f32) as i32;
    if anaglyph_enabled {
        (var5, var6, var7) = anaglyph(var5, var6, var7);
    }
    image[var2 * 4] = var5 as u8;
    image[var2 * 4 + 1] = var6 as u8;
    image[var2 * 4 + 2] = var7 as u8;
    image[var2 * 4 + 3] = var8 as u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hand-checked properties over 200 ticks: blue is always 255, red in
    /// [32, 64], green in [50, 114], and alpha follows red:
    /// `alpha = (int)(146 + v*50)` where `red = (int)(32 + v*32)`, so alpha
    /// stays in [146, 196].
    #[test]
    fn channel_ranges() {
        let mut fx = TextureWaterFx::new_fx();
        let mut rnd = JavaRandom::new(3);
        for _ in 0..200 {
            fx.on_tick(&mut rnd, None);
            for p in fx.image_data.chunks(4) {
                assert_eq!(p[2], 255);
                assert!((32..=64).contains(&p[0]), "{p:?}");
                assert!((50..=114).contains(&p[1]), "{p:?}");
                assert!((146..=196).contains(&p[3]), "{p:?}");
            }
        }
        // The first frame (all zero) is (32, 50, 255, 146).
        let mut fx = TextureWaterFx::new_fx();
        fx.on_tick(&mut JavaRandom::new(3), None);
        assert_eq!(&fx.image_data[0..4], [32, 50, 255, 146]);
    }

    /// Hand-checked: the spread is a 3-wide horizontal sum over 3.3 plus
    /// 0.8 × the source, with x wrapping.
    #[test]
    fn horizontal_blur_wraps() {
        let mut fx = TextureWaterFx::new_fx();
        if let FxKind::Water(w) = &mut fx.kind {
            w.height[0] = 1.0; // (0, 0)
        }
        fx.on_tick(&mut JavaRandom::new(0), None);
        let FxKind::Water(w) = &fx.kind else {
            unreachable!()
        };
        assert_eq!(w.height[0], 1.0 / 3.3f32);
        assert_eq!(w.height[1], 1.0 / 3.3f32);
        assert_eq!(w.height[15], 1.0 / 3.3f32);
        assert_eq!(w.height[2], 0.0);
        assert_eq!(w.height[16], 0.0);
    }

    /// Regression checksum of the first 100 frames (seed 99). Taken from
    /// this implementation, not from Java (no client oracle).
    #[test]
    fn checksum_regression() {
        let mut fx = TextureWaterFx::new_fx();
        let mut rnd = JavaRandom::new(99);
        let mut sum = crate::textures::tests::Fnv::default();
        for _ in 0..100 {
            fx.on_tick(&mut rnd, None);
            sum.add(&fx.image_data);
        }
        assert_eq!(
            sum.0,
            crate::textures::tests::WATER_CHECKSUM,
            "{:#x}",
            sum.0
        );
    }
}
