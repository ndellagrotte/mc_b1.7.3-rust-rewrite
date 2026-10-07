//! `TexturePortalFX` (TexturePortalFX.java): the portal tile
//! (`Block.portal.blockIndexInTexture`, 14). Its constructor precomputes 32
//! frames of two counter-rotating swirls plus noise from its own
//! `new Random(100L)`; `onTick` steps through them.

use mc_core::java_random::JavaRandom;
use mc_core::math::{self, jmath};

use super::texture_fx::{FxKind, PORTAL_INDEX, TextureFx, anaglyph};

/// `TexturePortalFX`'s fields.
#[derive(Clone, Debug)]
pub struct TexturePortalFx {
    /// `portalTickCounter`.
    // src: TexturePortalFX.java:6
    pub portal_tick_counter: i32,
    /// `portalTextureData`: 32 frames of RGBA.
    // src: TexturePortalFX.java:7
    pub portal_texture_data: Vec<[u8; 1024]>,
}

/// The swirl value of one pixel before the noise: `var5` after the
/// `var6` loop of the constructor (frame `var2`, pixel `(var3, var4)`).
// src: TexturePortalFX.java:16-45
pub fn swirl(var2: i32, var3: i32, var4: i32) -> f32 {
    let mut var5 = 0.0f32;
    for var6 in 0..2i32 {
        let var7 = (var6 * 8) as f32;
        let var8 = (var6 * 8) as f32;
        let mut var9 = (var3 as f32 - var7) / 16.0f32 * 2.0f32;
        let mut var10 = (var4 as f32 - var8) / 16.0f32 * 2.0f32;
        if var9 < -1.0 {
            var9 += 2.0;
        }
        if var9 >= 1.0 {
            var9 -= 2.0;
        }
        if var10 < -1.0 {
            var10 += 2.0;
        }
        if var10 >= 1.0 {
            var10 -= 2.0;
        }
        let var11 = var9 * var9 + var10 * var10;
        let mut var12 = jmath::atan2(var10 as f64, var9 as f64) as f32
            + (var2 as f32 / 32.0f32 * 3.1415927f32 * 2.0f32 - var11 * 10.0f32 + (var6 * 2) as f32)
                * (var6 * 2 - 1) as f32;
        var12 = (math::sin(var12) + 1.0f32) / 2.0f32;
        var12 /= var11 + 1.0f32;
        var5 += var12 * 0.5f32;
    }
    var5
}

impl TexturePortalFx {
    /// `new TexturePortalFX()`.
    // src: TexturePortalFX.java:9-61
    pub fn new_fx() -> TextureFx {
        let mut var1 = JavaRandom::new(100);
        let mut frames = vec![[0u8; 1024]; 32];
        for (var2, frame) in frames.iter_mut().enumerate() {
            for var3 in 0..16i32 {
                for var4 in 0..16i32 {
                    let mut var5 = swirl(var2 as i32, var3, var4);
                    var5 += var1.next_float() * 0.1f32;
                    let var6 = (var5 * 100.0f32 + 155.0f32) as i32;
                    let var13 = (var5 * var5 * 200.0f32 + 55.0f32) as i32;
                    let var14 = (var5 * var5 * var5 * var5 * 255.0f32) as i32;
                    let var15 = (var5 * 100.0f32 + 155.0f32) as i32;
                    let var16 = (var4 * 16 + var3) as usize;
                    frame[var16 * 4] = var13 as u8;
                    frame[var16 * 4 + 1] = var14 as u8;
                    frame[var16 * 4 + 2] = var6 as u8;
                    frame[var16 * 4 + 3] = var15 as u8;
                }
            }
        }
        TextureFx::new(
            PORTAL_INDEX,
            FxKind::Portal(Box::new(TexturePortalFx {
                portal_tick_counter: 0,
                portal_texture_data: frames,
            })),
        )
    }

    /// `onTick()`.
    // src: TexturePortalFX.java:63-87
    pub fn on_tick(&mut self, image: &mut [u8; 1024], anaglyph_enabled: bool) {
        self.portal_tick_counter = self.portal_tick_counter.wrapping_add(1);
        let var1 = &self.portal_texture_data[(self.portal_tick_counter & 31) as usize];
        for var2 in 0..256usize {
            let mut var3 = var1[var2 * 4] as i32;
            let mut var4 = var1[var2 * 4 + 1] as i32;
            let mut var5 = var1[var2 * 4 + 2] as i32;
            let var6 = var1[var2 * 4 + 3] as i32;
            if anaglyph_enabled {
                (var3, var4, var5) = anaglyph(var3, var4, var5);
            }
            image[var2 * 4] = var3 as u8;
            image[var2 * 4 + 1] = var4 as u8;
            image[var2 * 4 + 2] = var5 as u8;
            image[var2 * 4 + 3] = var6 as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hand-checked: pixel (0, 0) of frame 0. Swirl 0 (centre (0, 0)):
    /// `var9 = var10 = 0`, `var11 = 0`, `atan2(0, 0) = 0`, angle
    /// `(0 - 0 + 0) * -1 = 0`, `(sin 0 + 1) / 2 = 0.5`, contributes 0.25.
    /// Swirl 1 (centre (8, 8)): `var9 = var10 = -1` (not < -1, so not
    /// wrapped), `var11 = 2`, `atan2(-1, -1) = -3π/4`, angle `-3π/4 + (0 -
    /// 20 + 2) * 1`, value `(sin + 1) / 2 / 3 * 0.5`.
    #[test]
    fn swirl_at_origin_frame_0() {
        let a = jmath::atan2(-1.0, -1.0) as f32 + (0.0f32 - 2.0f32 * 10.0f32 + 2.0f32);
        let s1 = (math::sin(a) + 1.0f32) / 2.0f32 / 3.0f32 * 0.5f32;
        let want = 0.0f32 + 0.25f32 + s1;
        assert_eq!(swirl(0, 0, 0), want);
        // Sanity on the magnitudes: 0.25 + something in [0, 1/6].
        assert!((0.25..=0.25 + 1.0 / 6.0).contains(&want));
    }

    /// Hand-checked: the constructor's first `nextFloat` of `Random(100)`
    /// adds to pixel (0, 0) of frame 0, packed as (G², G⁴, B, A) =
    /// `(v²*200+55, v⁴*255, v*100+155, v*100+155)`; frame `k & 31` is shown
    /// on the k-th tick.
    #[test]
    fn frame_layout_and_cycle() {
        let mut fx = TexturePortalFx::new_fx();
        let v = swirl(0, 0, 0) + JavaRandom::new(100).next_float() * 0.1f32;
        let want = [
            (v * v * 200.0f32 + 55.0f32) as i32 as u8,
            (v * v * v * v * 255.0f32) as i32 as u8,
            (v * 100.0f32 + 155.0f32) as i32 as u8,
            (v * 100.0f32 + 155.0f32) as i32 as u8,
        ];
        let FxKind::Portal(p) = &fx.kind else {
            unreachable!()
        };
        assert_eq!(&p.portal_texture_data[0][0..4], want);
        let frame1 = p.portal_texture_data[1];
        let frame0 = p.portal_texture_data[0];
        fx.on_tick(&mut JavaRandom::new(0), None);
        assert_eq!(fx.image_data, frame1);
        for _ in 0..31 {
            fx.on_tick(&mut JavaRandom::new(0), None);
        }
        assert_eq!(fx.image_data, frame0);
    }

    /// Regression checksum of all 32 precomputed frames (fully determined
    /// by `Random(100)`). Taken from this implementation, not from Java (no
    /// client oracle).
    #[test]
    fn checksum_regression() {
        let fx = TexturePortalFx::new_fx();
        let FxKind::Portal(p) = &fx.kind else {
            unreachable!()
        };
        let mut sum = crate::textures::tests::Fnv::default();
        for f in &p.portal_texture_data {
            sum.add(f);
        }
        assert_eq!(
            sum.0,
            crate::textures::tests::PORTAL_CHECKSUM,
            "{:#x}",
            sum.0
        );
    }
}
