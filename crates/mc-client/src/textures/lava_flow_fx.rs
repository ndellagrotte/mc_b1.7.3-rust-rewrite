//! `TextureLavaFlowFX` (TextureLavaFlowFX.java): the flowing lava tile
//! (`Block.lavaMoving.blockIndexInTexture + 1`, 238, `tileSize` 2: written
//! to 238, 239, 254 and 255). The same field as `TextureLavaFX`, read
//! scrolled down one row every third tick.

use mc_core::java_random::JavaRandom;

use super::lava_fx::{lava_pixel, lava_step};
use super::texture_fx::{FxKind, LAVA_INDEX, TextureFx};

/// `TextureLavaFlowFX`'s fields.
#[derive(Clone, Debug)]
pub struct TextureLavaFlowFx {
    /// `field_1143_g`: the heat shown.
    // src: TextureLavaFlowFX.java:4
    pub heat: [f32; 256],
    /// `field_1142_h`: the next heat.
    // src: TextureLavaFlowFX.java:5
    pub heat_next: [f32; 256],
    /// `field_1141_i`: the heat sources.
    // src: TextureLavaFlowFX.java:6
    pub source: [f32; 256],
    /// `field_1140_j`: the sources' rate.
    // src: TextureLavaFlowFX.java:7
    pub source_delta: [f32; 256],
    /// `field_1139_k`: ticks so far (the scroll: one row per 3 ticks).
    // src: TextureLavaFlowFX.java:8
    pub tick_counter: i32,
}

impl TextureLavaFlowFx {
    /// `new TextureLavaFlowFX()`: `tileSize = 2`.
    // src: TextureLavaFlowFX.java:10-13
    pub fn new_fx() -> TextureFx {
        let mut fx = TextureFx::new(
            LAVA_INDEX + 1,
            FxKind::LavaFlow(Box::new(TextureLavaFlowFx {
                heat: [0.0; 256],
                heat_next: [0.0; 256],
                source: [0.0; 256],
                source_delta: [0.0; 256],
                tick_counter: 0,
            })),
        );
        fx.tile_size = 2;
        fx
    }

    /// `onTick()`.
    // src: TextureLavaFlowFX.java:15-86
    pub fn on_tick(
        &mut self,
        image: &mut [u8; 1024],
        anaglyph_enabled: bool,
        rnd: &mut JavaRandom,
    ) {
        self.tick_counter = self.tick_counter.wrapping_add(1);
        lava_step(
            &mut self.heat,
            &mut self.heat_next,
            &mut self.source,
            &mut self.source_delta,
            rnd,
        );
        for var2 in 0..256i32 {
            // src: TextureLavaFlowFX.java:59
            let src = (var2.wrapping_sub((self.tick_counter / 3).wrapping_mul(16)) & 255) as usize;
            lava_pixel(image, var2 as usize, self.heat[src], anaglyph_enabled);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hand-checked: the shown tile is the heat field shifted down by
    /// `tick_counter / 3` rows (row r shows field row r - k/3, wrapped).
    #[test]
    fn scrolls_one_row_every_third_tick() {
        let mut fx = TextureLavaFlowFx::new_fx();
        let mut rnd = JavaRandom::new(5);
        for _ in 0..7 {
            fx.on_tick(&mut rnd, None);
        }
        let FxKind::LavaFlow(l) = &fx.kind else {
            unreachable!()
        };
        assert_eq!(l.tick_counter, 7);
        // 7 / 3 = 2 rows.
        for var2 in 0..256usize {
            let mut img = [0u8; 1024];
            lava_pixel(&mut img, 0, l.heat[(var2 + 256 - 32) & 255], false);
            assert_eq!(&fx.image_data[var2 * 4..var2 * 4 + 4], &img[0..4]);
        }
    }

    /// Regression checksum of the first 100 frames (seed 99). Taken from
    /// this implementation, not from Java (no client oracle).
    #[test]
    fn checksum_regression() {
        let mut fx = TextureLavaFlowFx::new_fx();
        let mut rnd = JavaRandom::new(99);
        let mut sum = crate::textures::tests::Fnv::default();
        for _ in 0..100 {
            fx.on_tick(&mut rnd, None);
            sum.add(&fx.image_data);
        }
        assert_eq!(
            sum.0,
            crate::textures::tests::LAVA_FLOW_CHECKSUM,
            "{:#x}",
            sum.0
        );
    }
}
