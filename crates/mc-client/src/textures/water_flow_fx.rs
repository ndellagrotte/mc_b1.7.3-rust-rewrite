//! `TextureWaterFlowFX` (TextureWaterFlowFX.java): the flowing water tile
//! (`Block.waterMoving.blockIndexInTexture + 1`, 206, `tileSize` 2: written
//! to 206, 207, 222 and 223). A field blurred vertically and read scrolled
//! down one row a tick.

use mc_core::java_random::JavaRandom;

use super::texture_fx::{FxKind, TextureFx, WATER_INDEX};
use super::water_fx::water_pixel;

/// `TextureWaterFlowFX`'s fields.
#[derive(Clone, Debug)]
pub struct TextureWaterFlowFx {
    /// `field_1138_g`: the field shown.
    // src: TextureWaterFlowFX.java:4
    pub height: [f32; 256],
    /// `field_1137_h`: the next field.
    // src: TextureWaterFlowFX.java:5
    pub height_next: [f32; 256],
    /// `field_1136_i`: the sources.
    // src: TextureWaterFlowFX.java:6
    pub source: [f32; 256],
    /// `field_1135_j`: the sources' rate, -0.3 a tick, reset to 0.5 at
    /// random.
    // src: TextureWaterFlowFX.java:7
    pub source_delta: [f32; 256],
    /// `field_1134_k`: ticks so far (the scroll: one row a tick).
    // src: TextureWaterFlowFX.java:8
    pub tick_counter: i32,
}

impl TextureWaterFlowFx {
    /// `new TextureWaterFlowFX()`: `tileSize = 2`.
    // src: TextureWaterFlowFX.java:10-13
    pub fn new_fx() -> TextureFx {
        let mut fx = TextureFx::new(
            WATER_INDEX + 1,
            FxKind::WaterFlow(Box::new(TextureWaterFlowFx {
                height: [0.0; 256],
                height_next: [0.0; 256],
                source: [0.0; 256],
                source_delta: [0.0; 256],
                tick_counter: 0,
            })),
        );
        fx.tile_size = 2;
        fx
    }

    /// `onTick()`.
    // src: TextureWaterFlowFX.java:15-87
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
                for var4 in var2 - 2..=var2 {
                    let var5 = var1 & 15;
                    let var6 = var4 & 15;
                    var3 += self.height[(var5 + var6 * 16) as usize];
                }
                let i = (var1 + var2 * 16) as usize;
                self.height_next[i] = var3 / 3.2f32 + self.source[i] * 0.8f32;
            }
        }
        for var1 in 0..16usize {
            for var2 in 0..16usize {
                let i = var1 + var2 * 16;
                self.source[i] += self.source_delta[i] * 0.05f32;
                if self.source[i] < 0.0 {
                    self.source[i] = 0.0;
                }
                self.source_delta[i] -= 0.3f32;
                if rnd.next_double() < 0.2 {
                    self.source_delta[i] = 0.5f32;
                }
            }
        }
        std::mem::swap(&mut self.height, &mut self.height_next);
        for var2 in 0..256i32 {
            // src: TextureWaterFlowFX.java:58
            let src = (var2.wrapping_sub(self.tick_counter.wrapping_mul(16)) & 255) as usize;
            water_pixel(image, var2 as usize, self.height[src], anaglyph_enabled);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hand-checked: the blur sums the cell and the two above it (y - 2 ..=
    /// y, wrapped) over 3.2.
    #[test]
    fn vertical_blur_wraps() {
        let mut fx = TextureWaterFlowFx::new_fx();
        if let FxKind::WaterFlow(w) = &mut fx.kind {
            w.height[3] = 1.0; // (3, 0)
        }
        fx.on_tick(&mut JavaRandom::new(0), None);
        let FxKind::WaterFlow(w) = &fx.kind else {
            unreachable!()
        };
        let v = 1.0 / 3.2f32;
        assert_eq!(w.height[3], v);
        assert_eq!(w.height[3 + 16], v);
        assert_eq!(w.height[3 + 32], v);
        assert_eq!(w.height[3 + 48], 0.0);
        assert_eq!(w.height[3 + 15 * 16], 0.0);
        assert_eq!(w.height[4], 0.0);
        // Shown scrolled by one row: pixel row 1 shows field row 0.
        let mut img = [0u8; 1024];
        water_pixel(&mut img, 0, v, false);
        assert_eq!(&fx.image_data[(3 + 16) * 4..(3 + 16) * 4 + 4], &img[0..4]);
    }

    /// Regression checksum of the first 100 frames (seed 99). Taken from
    /// this implementation, not from Java (no client oracle).
    #[test]
    fn checksum_regression() {
        let mut fx = TextureWaterFlowFx::new_fx();
        let mut rnd = JavaRandom::new(99);
        let mut sum = crate::textures::tests::Fnv::default();
        for _ in 0..100 {
            fx.on_tick(&mut rnd, None);
            sum.add(&fx.image_data);
        }
        assert_eq!(
            sum.0,
            crate::textures::tests::WATER_FLOW_CHECKSUM,
            "{:#x}",
            sum.0
        );
    }
}
