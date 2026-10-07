//! `TextureCompassFX` (TextureCompassFX.java): the compass item icon
//! (`Item.compass`, icon 54 of `gui/items.png`). Each tick it copies the
//! icon from the jar's `items.png` and draws the needle toward the world
//! spawn, eased with a damped spring; without a world the target angle is 0,
//! in the Nether it is `Math.random()` (the needle wanders).

use mc_core::java_random::JavaRandom;
use mc_core::math::jmath;

use super::texture_fx::{
    COMPASS_INDEX, FxKind, FxWorldView, TILE_IMAGE_ITEMS, TextureFx, anaglyph, get_rgb_16,
};

/// `TextureCompassFX`'s fields (`mc` is the [`FxWorldView`] passed in).
#[derive(Clone, Debug)]
pub struct TextureCompassFx {
    /// `compassIconImageData`: the icon as `getRGB` ARGB.
    // src: TextureCompassFX.java:10
    pub compass_icon_image_data: [i32; 256],
    /// `field_4229_i`: the needle's angle (radians).
    // src: TextureCompassFX.java:11
    pub needle_angle: f64,
    /// `field_4228_j`: the needle's angular velocity.
    // src: TextureCompassFX.java:12
    pub needle_velocity: f64,
}

impl TextureCompassFx {
    /// `new TextureCompassFX(mc)`: `tileImage = 1`, and the icon read from
    /// the jar's `/gui/items.png` (`Minecraft.class.getResource`, so never
    /// a texture pack's). `items_argb` is `None` when the read failed (Java
    /// prints the exception and keeps zeros).
    // src: TextureCompassFX.java:14-28
    pub fn new_fx(items_argb: Option<(u32, u32, &[i32])>) -> TextureFx {
        let var3 = COMPASS_INDEX % 16 * 16;
        let var4 = COMPASS_INDEX / 16 * 16;
        let mut fx = TextureFx::new(
            COMPASS_INDEX,
            FxKind::Compass(Box::new(TextureCompassFx {
                compass_icon_image_data: get_rgb_16(items_argb, var3, var4),
                needle_angle: 0.0,
                needle_velocity: 0.0,
            })),
        );
        fx.tile_image = TILE_IMAGE_ITEMS;
        fx
    }

    /// The angle the needle is pulled toward (`var20`).
    // src: TextureCompassFX.java:51-60
    pub fn target_angle(view: Option<&FxWorldView>, rnd: &mut JavaRandom) -> f64 {
        let mut var20 = 0.0f64;
        if let Some(v) = view {
            let var23 = v.spawn_x as f64 - v.player_x;
            let var25 = v.spawn_z as f64 - v.player_z;
            var20 = (v.player_yaw - 90.0f32) as f64 * 3.141592653589793 / 180.0
                - jmath::atan2(var25, var23);
            if v.is_nether {
                var20 = rnd.next_double() * 3.1415927410125732 * 2.0;
            }
        }
        var20
    }

    /// `onTick()`.
    // src: TextureCompassFX.java:30-141
    pub fn on_tick(
        &mut self,
        image: &mut [u8; 1024],
        anaglyph_enabled: bool,
        rnd: &mut JavaRandom,
        view: Option<&FxWorldView>,
    ) {
        for var1 in 0..256usize {
            let px = self.compass_icon_image_data[var1];
            let var2 = px >> 24 & 255;
            let mut var3 = px >> 16 & 255;
            let mut var4 = px >> 8 & 255;
            let mut var5 = px & 255;
            if anaglyph_enabled {
                (var3, var4, var5) = anaglyph(var3, var4, var5);
            }
            image[var1 * 4] = var3 as u8;
            image[var1 * 4 + 1] = var4 as u8;
            image[var1 * 4 + 2] = var5 as u8;
            image[var1 * 4 + 3] = var2 as u8;
        }

        let var20 = Self::target_angle(view, rnd);
        (self.needle_angle, self.needle_velocity) =
            ease_angle(self.needle_angle, self.needle_velocity, var20);
        let var24 = jmath::sin(self.needle_angle);
        let var26 = jmath::cos(self.needle_angle);

        // The short grey back of the needle.
        for var9 in -4..=4i32 {
            let var10 = (8.5 + var26 * var9 as f64 * 0.3) as i32;
            let var11 = (7.5 - var24 * var9 as f64 * 0.3 * 0.5) as i32;
            let var12 = (var11 * 16 + var10) as usize;
            let (mut var13, mut var14, mut var15) = (100, 100, 100);
            let var16 = 255;
            if anaglyph_enabled {
                (var13, var14, var15) = anaglyph(var13, var14, var15);
            }
            image[var12 * 4] = var13 as u8;
            image[var12 * 4 + 1] = var14 as u8;
            image[var12 * 4 + 2] = var15 as u8;
            image[var12 * 4 + 3] = var16 as u8;
        }

        // The needle: grey behind the pivot, red in front.
        for var9 in -8..=16i32 {
            let var10 = (8.5 + var24 * var9 as f64 * 0.3) as i32;
            let var11 = (7.5 + var26 * var9 as f64 * 0.3 * 0.5) as i32;
            let var12 = (var11 * 16 + var10) as usize;
            let mut var13 = if var9 >= 0 { 255 } else { 100 };
            let mut var14 = if var9 >= 0 { 20 } else { 100 };
            let mut var15 = if var9 >= 0 { 20 } else { 100 };
            let var16 = 255;
            if anaglyph_enabled {
                (var13, var14, var15) = anaglyph(var13, var14, var15);
            }
            image[var12 * 4] = var13 as u8;
            image[var12 * 4 + 1] = var14 as u8;
            image[var12 * 4 + 2] = var15 as u8;
            image[var12 * 4 + 3] = var16 as u8;
        }
    }
}

/// The compass's and the watch's easing: the difference to the target is
/// wrapped into [-π, π) and clamped to [-1, 1], the velocity gains a tenth
/// of it and keeps 0.8, and the angle moves by the velocity. Returns the new
/// `(angle, velocity)`.
// src: TextureCompassFX.java:62-80, TextureWatchFX.java:43-61
#[allow(clippy::manual_clamp)] // Java's two ifs, kept
pub fn ease_angle(angle: f64, velocity: f64, target: f64) -> (f64, f64) {
    let mut var22 = target - angle;
    while var22 < -3.141592653589793 {
        var22 += 6.283185307179586;
    }
    while var22 >= 3.141592653589793 {
        var22 -= 6.283185307179586;
    }
    if var22 < -1.0 {
        var22 = -1.0;
    }
    if var22 > 1.0 {
        var22 = 1.0;
    }
    let mut v = velocity + var22 * 0.1;
    v *= 0.8;
    (angle + v, v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(spawn: (i32, i32), player: (f64, f64), yaw: f32) -> FxWorldView {
        FxWorldView {
            spawn_x: spawn.0,
            spawn_z: spawn.1,
            player_x: player.0,
            player_z: player.1,
            player_yaw: yaw,
            is_nether: false,
            celestial_angle: 0.0,
        }
    }

    /// Hand-checked: spawn due south (+z) of a player facing south (yaw 0):
    /// `(0 - 90)° - atan2(10, 0) = -π/2 - π/2 = -π`. Facing north (yaw
    /// 180): `π/2 - π/2 = 0`. Spawn due east (+x), facing south:
    /// `-π/2 - atan2(0, 10) = -π/2`.
    #[test]
    fn target_angle_for_known_positions() {
        let mut rnd = JavaRandom::new(0);
        let a = TextureCompassFx::target_angle(Some(&view((0, 10), (0.0, 0.0), 0.0)), &mut rnd);
        assert!((a + std::f64::consts::PI).abs() < 1e-12, "{a}");
        let a = TextureCompassFx::target_angle(Some(&view((0, 10), (0.0, 0.0), 180.0)), &mut rnd);
        assert!(a.abs() < 1e-12, "{a}");
        let a = TextureCompassFx::target_angle(Some(&view((10, 0), (0.0, 0.0), 0.0)), &mut rnd);
        assert!((a + std::f64::consts::FRAC_PI_2).abs() < 1e-12, "{a}");
        // No world: 0, and no draw.
        let before = rnd.state();
        assert_eq!(TextureCompassFx::target_angle(None, &mut rnd), 0.0);
        assert_eq!(rnd.state(), before);
        // The Nether: one Math.random() draw.
        let mut v = view((0, 10), (0.0, 0.0), 0.0);
        v.is_nether = true;
        let mut check = JavaRandom::new(77);
        let mut rnd = JavaRandom::new(77);
        let a = TextureCompassFx::target_angle(Some(&v), &mut rnd);
        assert_eq!(a, check.next_double() * 3.1415927410125732 * 2.0);
    }

    /// Hand-checked easing: from rest, a target 0.5 away moves the needle
    /// by 0.5 * 0.1 * 0.8 = 0.04; a target 3 away is clamped to 1 (0.08);
    /// a target at 3π/2 wraps to -π/2 (then clamped to -1: -0.08).
    #[test]
    fn easing() {
        let (a, v) = ease_angle(0.0, 0.0, 0.5);
        assert!((a - 0.04).abs() < 1e-15 && (v - 0.04).abs() < 1e-15);
        let (a, _) = ease_angle(0.0, 0.0, 3.0);
        assert!((a - 0.08).abs() < 1e-15);
        let (a, _) = ease_angle(0.0, 0.0, 3.0 * std::f64::consts::FRAC_PI_2);
        assert!((a + 0.08).abs() < 1e-15);
        // Converges on the target.
        let (mut a, mut v) = (0.0, 0.0);
        for _ in 0..400 {
            (a, v) = ease_angle(a, v, 1.2);
        }
        assert!((a - 1.2).abs() < 1e-9);
    }

    /// Hand-checked needle: at angle 0 (sin 0, cos 1) the red half runs
    /// down from the pivot: `x = (int)8.5 = 8`, `y = (int)(7.5 + k*0.15)`
    /// for k = 0..16, so the tip (k = 16) is at (8, 9); the grey back
    /// (k = -8) at (8, 6); the cross bar at y 7, x 7..9.
    #[test]
    fn needle_pixels_at_angle_0() {
        let mut fx = TextureCompassFx::new_fx(None);
        let mut rnd = JavaRandom::new(0);
        // No world: the target is 0 and the needle stays at 0.
        fx.on_tick(&mut rnd, None);
        let px = |x: usize, y: usize| &fx.image_data[(y * 16 + x) * 4..(y * 16 + x) * 4 + 4];
        assert_eq!(px(8, 9), [255, 20, 20, 255]);
        assert_eq!(px(8, 8), [255, 20, 20, 255]);
        assert_eq!(px(8, 7), [255, 20, 20, 255]); // k = 0 is red, drawn last
        assert_eq!(px(8, 6), [100, 100, 100, 255]);
        assert_eq!(px(7, 7), [100, 100, 100, 255]);
        assert_eq!(px(9, 7), [100, 100, 100, 255]);
        assert_eq!(px(0, 0), [0, 0, 0, 0]);
    }

    /// Regression checksum: 60 ticks walking the player around a spawn,
    /// then 20 in the Nether (seed 11). Taken from this implementation, not
    /// from Java (no client oracle).
    #[test]
    fn checksum_regression() {
        let mut fx = TextureCompassFx::new_fx(None);
        let mut rnd = JavaRandom::new(11);
        let mut sum = crate::textures::tests::Fnv::default();
        for t in 0..80 {
            let mut v = view(
                (100, -40),
                (t as f64 * 1.5, 20.0 - t as f64),
                t as f32 * 7.0,
            );
            v.is_nether = t >= 60;
            fx.on_tick(&mut rnd, Some(&v));
            sum.add(&fx.image_data);
        }
        assert_eq!(
            sum.0,
            crate::textures::tests::COMPASS_CHECKSUM,
            "{:#x}",
            sum.0
        );
    }
}
