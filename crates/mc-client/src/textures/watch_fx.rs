//! `TextureWatchFX` (TextureWatchFX.java): the clock item icon
//! (`Item.pocketSundial`, icon 70 of `gui/items.png`). The icon's magenta
//! pixels (red == blue, green 0) are a window onto `/misc/dial.png`, rotated
//! by the world's celestial angle with the compass's easing; without a world
//! the target is 0, in the Nether `Math.random()`.

use mc_core::java_random::JavaRandom;
use mc_core::math::jmath;

use super::compass_fx::ease_angle;
use super::texture_fx::{
    FxKind, FxWorldView, TILE_IMAGE_ITEMS, TextureFx, WATCH_INDEX, anaglyph, get_rgb_16,
};

/// `TextureWatchFX`'s fields (`mc` is the [`FxWorldView`] passed in).
#[derive(Clone, Debug)]
pub struct TextureWatchFx {
    /// `watchIconImageData`: the icon as `getRGB` ARGB.
    // src: TextureWatchFX.java:10
    pub watch_icon_image_data: [i32; 256],
    /// `dialImageData`: `/misc/dial.png`'s top-left 16×16, ARGB.
    // src: TextureWatchFX.java:11
    pub dial_image_data: [i32; 256],
    /// `field_4222_j`: the dial's angle (radians).
    // src: TextureWatchFX.java:12
    pub dial_angle: f64,
    /// `field_4221_k`: the dial's angular velocity.
    // src: TextureWatchFX.java:13
    pub dial_velocity: f64,
}

impl TextureWatchFx {
    /// `new TextureWatchFX(mc)`: `tileImage = 1`, the icon from the jar's
    /// `/gui/items.png` and the dial from `/misc/dial.png` (both through
    /// `Minecraft.class.getResource`). A failed read leaves zeros (Java
    /// prints the `IOException`; a failure on items.png also skips the
    /// dial).
    // src: TextureWatchFX.java:15-31
    pub fn new_fx(
        items_argb: Option<(u32, u32, &[i32])>,
        dial_argb: Option<(u32, u32, &[i32])>,
    ) -> TextureFx {
        let var3 = WATCH_INDEX % 16 * 16;
        let var4 = WATCH_INDEX / 16 * 16;
        let icon = get_rgb_16(items_argb, var3, var4);
        let dial = if items_argb.is_some() {
            get_rgb_16(dial_argb, 0, 0)
        } else {
            [0; 256]
        };
        let mut fx = TextureFx::new(
            WATCH_INDEX,
            FxKind::Watch(Box::new(TextureWatchFx {
                watch_icon_image_data: icon,
                dial_image_data: dial,
                dial_angle: 0.0,
                dial_velocity: 0.0,
            })),
        );
        fx.tile_image = TILE_IMAGE_ITEMS;
        fx
    }

    /// The dial's target angle (`var1`).
    // src: TextureWatchFX.java:34-41
    pub fn target_angle(view: Option<&FxWorldView>, rnd: &mut JavaRandom) -> f64 {
        let mut var1 = 0.0f64;
        if let Some(v) = view {
            let var3 = v.celestial_angle;
            var1 = (-var3 * 3.1415927f32 * 2.0f32) as f64;
            if v.is_nether {
                var1 = rnd.next_double() * 3.1415927410125732 * 2.0;
            }
        }
        var1
    }

    /// `onTick()`.
    // src: TextureWatchFX.java:33-98
    pub fn on_tick(
        &mut self,
        image: &mut [u8; 1024],
        anaglyph_enabled: bool,
        rnd: &mut JavaRandom,
        view: Option<&FxWorldView>,
    ) {
        let var1 = Self::target_angle(view, rnd);
        (self.dial_angle, self.dial_velocity) =
            ease_angle(self.dial_angle, self.dial_velocity, var1);
        let var5 = jmath::sin(self.dial_angle);
        let var7 = jmath::cos(self.dial_angle);

        for var9 in 0..256usize {
            let px = self.watch_icon_image_data[var9];
            let mut var10 = px >> 24 & 255;
            let mut var11 = px >> 16 & 255;
            let mut var12 = px >> 8 & 255;
            let mut var13 = px & 255;
            if var11 == var13 && var12 == 0 && var13 > 0 {
                let (var21, var18) = dial_lookup(var9 as i32, var5, var7, var11);
                let d = self.dial_image_data[var21];
                var10 = d >> 24 & 255;
                var11 = (d >> 16 & 255) * var11 / 255;
                var12 = (d >> 8 & 255) * var18 / 255;
                var13 = (d & 255) * var18 / 255;
            }
            if anaglyph_enabled {
                (var11, var12, var13) = anaglyph(var11, var12, var13);
            }
            image[var9 * 4] = var11 as u8;
            image[var9 * 4 + 1] = var12 as u8;
            image[var9 * 4 + 2] = var13 as u8;
            image[var9 * 4 + 3] = var10 as u8;
        }
    }
}

/// The dial pixel a magenta icon pixel `var9` shows at rotation
/// `(sin, cos) = (var5, var7)`: `(var21, var18)`, the dial index and the
/// icon's own red (the shade).
// src: TextureWatchFX.java:71-76
pub fn dial_lookup(var9: i32, var5: f64, var7: f64, var11: i32) -> (usize, i32) {
    let var14 = -((var9 % 16) as f64 / 15.0 - 0.5);
    let var16 = (var9 / 16) as f64 / 15.0 - 0.5;
    let var18 = var11;
    let var19 = ((var14 * var7 + var16 * var5 + 0.5) * 16.0) as i32;
    let var20 = ((var16 * var7 - var14 * var5 + 0.5) * 16.0) as i32;
    let var21 = (var19 & 15) + (var20 & 15) * 16;
    (var21 as usize, var18)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(angle: f32) -> FxWorldView {
        FxWorldView {
            spawn_x: 0,
            spawn_z: 0,
            player_x: 0.0,
            player_z: 0.0,
            player_yaw: 0.0,
            is_nether: false,
            celestial_angle: angle,
        }
    }

    /// Hand-checked: the target is `-angle * 2π` in float: noon (celestial
    /// angle 0) is 0, sunset (0.25) is -π/2, midnight (0.5) is -π.
    #[test]
    fn target_for_known_celestial_angles() {
        let mut rnd = JavaRandom::new(0);
        assert_eq!(
            TextureWatchFx::target_angle(Some(&view(0.0)), &mut rnd),
            0.0
        );
        let a = TextureWatchFx::target_angle(Some(&view(0.25)), &mut rnd);
        assert_eq!(a, (-0.25f32 * 3.1415927f32 * 2.0f32) as f64);
        assert!((a + std::f64::consts::FRAC_PI_2).abs() < 1e-6);
        let a = TextureWatchFx::target_angle(Some(&view(0.5)), &mut rnd);
        assert!((a + std::f64::consts::PI).abs() < 1e-6);
        assert_eq!(TextureWatchFx::target_angle(None, &mut rnd), 0.0);
    }

    /// Hand-checked: at rotation 0 the dial is not turned: icon pixel
    /// (x, y) shows dial pixel `((int)((0.5 - (x/15 - 0.5)) * 16) & 15,
    /// (int)(y/15 * 16) & 15)`, i.e. mirrored in x: x = 0 → 16 & 15 = 0,
    /// x = 15 → 0; x = 5 → (int)(10.667) = 10; y = 5 → 5.
    #[test]
    fn dial_lookup_unrotated() {
        assert_eq!(dial_lookup(0, 0.0, 1.0, 200), (0, 200));
        assert_eq!(dial_lookup(15, 0.0, 1.0, 200), (0, 200));
        assert_eq!(dial_lookup(5 + 5 * 16, 0.0, 1.0, 200), (10 + 5 * 16, 200));
        // A half turn (sin 0, cos -1): x' = (int)((-(x/15-0.5)*-1 + 0.5)*16)
        // = (int)(x/15*16): x = 5 → 5; y' = (int)((-(y/15-0.5) + 0.5)*16):
        // y = 5 → 10.
        assert_eq!(dial_lookup(5 + 5 * 16, 0.0, -1.0, 9), (5 + 10 * 16, 9));
    }

    /// Hand-checked shading: a magenta icon pixel (r = b = 128, g = 0)
    /// takes the dial pixel's alpha and its colour scaled by 128/255; other
    /// pixels pass through.
    #[test]
    fn magenta_pixels_show_the_dial() {
        let mut fx = TextureWatchFx::new_fx(None, None);
        if let FxKind::Watch(w) = &mut fx.kind {
            w.watch_icon_image_data[0] = 0xff80_0080_u32 as i32;
            w.watch_icon_image_data[1] = 0xff10_2030_u32 as i32;
            w.dial_image_data = [0x7f40_c0ff; 256];
        }
        fx.on_tick(&mut JavaRandom::new(0), None);
        assert_eq!(
            &fx.image_data[0..4],
            [
                (0x40 * 128 / 255) as u8,
                (0xc0 * 128 / 255) as u8,
                (0xff * 128 / 255) as u8,
                0x7f
            ]
        );
        assert_eq!(&fx.image_data[4..8], [0x10, 0x20, 0x30, 0xff]);
    }

    /// Regression checksum: a day's worth of celestial angles in 100 ticks,
    /// then 20 Nether ticks (seed 21), with a synthetic icon and dial.
    /// Taken from this implementation, not from Java (no client oracle).
    #[test]
    fn checksum_regression() {
        let icon: Vec<i32> = (0..256)
            .map(|i| {
                if i % 3 == 0 {
                    0xff90_0090_u32 as i32
                } else {
                    0xff22_3344_u32 as i32
                }
            })
            .collect();
        let dial: Vec<i32> = (0..256)
            .map(|i| (0xff00_0000_u32 as i32) | (i * 0x010203))
            .collect();
        let mut fx = TextureWatchFx::new_fx(Some((16, 16, &icon)), Some((16, 16, &dial)));
        // The icon read at (96, 64) is outside the 16×16 stand-in: zeros.
        if let FxKind::Watch(w) = &mut fx.kind {
            w.watch_icon_image_data.copy_from_slice(&icon);
        }
        let mut rnd = JavaRandom::new(21);
        let mut sum = crate::textures::tests::Fnv::default();
        for t in 0..120 {
            let mut v = view(t as f32 / 100.0);
            v.is_nether = t >= 100;
            fx.on_tick(&mut rnd, Some(&v));
            sum.add(&fx.image_data);
        }
        assert_eq!(
            sum.0,
            crate::textures::tests::WATCH_CHECKSUM,
            "{:#x}",
            sum.0
        );
    }
}
