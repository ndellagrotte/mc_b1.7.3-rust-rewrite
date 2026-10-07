//! `TextureFlamesFX` (TextureFlamesFX.java): the fire tiles. Two instances,
//! `TextureFlamesFX(0)` and `(1)`, animate `Block.fire.blockIndexInTexture`
//! (31) and the tile below it (47). A 16×20 heat field rises one row a
//! tick, averaged with its neighbours and cooled; the bottom row is fed
//! with `Math.random()` heat.

use mc_core::java_random::JavaRandom;

use super::texture_fx::{FIRE_INDEX, FxKind, TextureFx, anaglyph};

/// `TextureFlamesFX`'s fields.
#[derive(Clone, Debug)]
pub struct TextureFlamesFx {
    /// `field_1133_g`: the heat field shown (16 wide, 20 high).
    // src: TextureFlamesFX.java:4
    pub flame_heat: [f32; 320],
    /// `field_1132_h`: the next heat field (swapped in after each tick).
    // src: TextureFlamesFX.java:5
    pub flame_heat_next: [f32; 320],
}

impl TextureFlamesFx {
    /// `new TextureFlamesFX(var1)`: icon `Block.fire.blockIndexInTexture +
    /// var1 * 16`.
    // src: TextureFlamesFX.java:7-9
    pub fn new_fx(var1: i32) -> TextureFx {
        TextureFx::new(
            FIRE_INDEX + var1 * 16,
            FxKind::Flames(Box::new(TextureFlamesFx {
                flame_heat: [0.0; 320],
                flame_heat_next: [0.0; 320],
            })),
        )
    }

    /// `onTick()`.
    // src: TextureFlamesFX.java:11-76
    #[allow(clippy::manual_clamp)] // Java's two ifs, kept
    pub fn on_tick(
        &mut self,
        image: &mut [u8; 1024],
        anaglyph_enabled: bool,
        rnd: &mut JavaRandom,
    ) {
        for var1 in 0..16i32 {
            for var2 in 0..20i32 {
                let mut var3 = 18i32;
                let mut var4 =
                    self.flame_heat[(var1 + (var2 + 1) % 20 * 16) as usize] * var3 as f32;
                for var5 in var1 - 1..=var1 + 1 {
                    for var6 in var2..=var2 + 1 {
                        if var5 >= 0 && var6 >= 0 && var5 < 16 && var6 < 20 {
                            var4 += self.flame_heat[(var5 + var6 * 16) as usize];
                        }
                        var3 += 1;
                    }
                }
                let i = (var1 + var2 * 16) as usize;
                self.flame_heat_next[i] = var4 / (var3 as f32 * 1.06f32);
                if var2 >= 19 {
                    // Java evaluates the four Math.random() calls left to
                    // right: r1 * r2 * r3 * 4.0 + r4 * 0.1… + 0.2….
                    let r1 = rnd.next_double();
                    let r2 = rnd.next_double();
                    let r3 = rnd.next_double();
                    let r4 = rnd.next_double();
                    self.flame_heat_next[i] = (r1 * r2 * r3 * 4.0
                        + r4 * 0.10000000149011612
                        + 0.20000000298023224) as f32;
                }
            }
        }

        std::mem::swap(&mut self.flame_heat, &mut self.flame_heat_next);

        for var2 in 0..256usize {
            let mut var13 = self.flame_heat[var2] * 1.8f32;
            if var13 > 1.0 {
                var13 = 1.0;
            }
            if var13 < 0.0 {
                var13 = 0.0;
            }
            let mut var5 = (var13 * 155.0f32 + 100.0f32) as i32;
            let mut var6 = (var13 * var13 * 255.0f32) as i32;
            let mut var7 = (var13
                * var13
                * var13
                * var13
                * var13
                * var13
                * var13
                * var13
                * var13
                * var13
                * 255.0f32) as i32;
            let mut var8: i32 = 255;
            if var13 < 0.5 {
                var8 = 0;
            }
            // `var4 = (var13 - 0.5F) * 2.0F` (TextureFlamesFX.java:60) is
            // computed and never read.
            if anaglyph_enabled {
                (var5, var6, var7) = anaglyph(var5, var6, var7);
            }
            image[var2 * 4] = var5 as u8;
            image[var2 * 4 + 1] = var6 as u8;
            image[var2 * 4 + 2] = var7 as u8;
            image[var2 * 4 + 3] = var8 as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::textures::texture_fx::FxKind;

    fn flames(fx: &TextureFx) -> &TextureFlamesFx {
        match &fx.kind {
            FxKind::Flames(f) => f,
            _ => unreachable!(),
        }
    }

    /// Hand-checked: from an all-zero field, the first tick leaves rows
    /// 0-18 at zero (no heat to spread) and fills row 19 with
    /// `r1*r2*r3*4 + r4*0.1 + 0.2` from the stream, in x order; the shown
    /// tile (rows 0-15) is then all `(100, 0, 0, 0)`: transparent dark red.
    #[test]
    fn first_tick_feeds_only_the_bottom_row() {
        let mut fx = TextureFlamesFx::new_fx(0);
        let mut rnd = JavaRandom::new(42);
        fx.on_tick(&mut rnd, None);
        let mut check = JavaRandom::new(42);
        let f = flames(&fx);
        for x in 0..16 {
            for y in 0..19 {
                assert_eq!(f.flame_heat[x + y * 16], 0.0);
            }
            let (r1, r2, r3, r4) = (
                check.next_double(),
                check.next_double(),
                check.next_double(),
                check.next_double(),
            );
            let want = (r1 * r2 * r3 * 4.0 + r4 * 0.10000000149011612 + 0.20000000298023224) as f32;
            assert_eq!(f.flame_heat[x + 19 * 16], want);
        }
        // 64 draws in all.
        assert_eq!(rnd.state(), check.state());
        for p in fx.image_data.chunks(4) {
            assert_eq!(p, [100, 0, 0, 0]);
        }
    }

    /// Hand-checked: the spread divides by `var3 * 1.06` with `var3 = 18 +
    /// 6 = 24` (the counter runs for all six neighbour cells, inside the
    /// field or not), and the cell below counts 18 times plus its own
    /// neighbour visit.
    #[test]
    fn spread_weights() {
        let mut fx = TextureFlamesFx::new_fx(0);
        if let FxKind::Flames(f) = &mut fx.kind {
            // One hot cell at (5, 10).
            f.flame_heat[5 + 10 * 16] = 1.0;
        }
        let mut rnd = JavaRandom::new(1);
        fx.on_tick(&mut rnd, None);
        let f = flames(&fx);
        let d = 24.0f32 * 1.06f32;
        // (5, 9): the cell below (5, 10) weighs 18 and (5, 10) is also a
        // neighbour of row 9 (var6 = 9..=10): (18 + 1) / d.
        assert_eq!(f.flame_heat[5 + 9 * 16], (1.0f32 * 18.0 + 1.0) / d);
        // (4, 9) and (6, 9): neighbour only.
        assert_eq!(f.flame_heat[4 + 9 * 16], 1.0 / d);
        assert_eq!(f.flame_heat[6 + 9 * 16], 1.0 / d);
        // (5, 10): itself as a neighbour (var6 = 10).
        assert_eq!(f.flame_heat[5 + 10 * 16], 1.0 / d);
        // (5, 8): nothing.
        assert_eq!(f.flame_heat[5 + 8 * 16], 0.0);
        // Its pixel: 19/25.44 * 1.8 > 1 → clamped to 1: (255, 255, 255, 255).
        assert_eq!(
            &fx.image_data[(5 + 9 * 16) * 4..(5 + 9 * 16) * 4 + 4],
            [255, 255, 255, 255]
        );
    }

    /// Regression checksum of the first 40 frames of both flame FX sharing
    /// one stream (seed 1234). The value was taken from this
    /// implementation, not from Java: the client has no oracle.
    #[test]
    fn checksum_regression() {
        let mut a = TextureFlamesFx::new_fx(0);
        let mut b = TextureFlamesFx::new_fx(1);
        let mut rnd = JavaRandom::new(1234);
        let mut sum = crate::textures::tests::Fnv::default();
        for _ in 0..40 {
            a.on_tick(&mut rnd, None);
            b.on_tick(&mut rnd, None);
            sum.add(&a.image_data);
            sum.add(&b.image_data);
        }
        assert_eq!(
            sum.0,
            crate::textures::tests::FLAMES_CHECKSUM,
            "{:#x}",
            sum.0
        );
    }
}
