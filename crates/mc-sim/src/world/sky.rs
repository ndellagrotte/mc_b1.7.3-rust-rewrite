//! The sky, fog and cloud colour helpers of `World` and `WorldProvider`
//! that only the client's renderer calls (WP-3.4): `getSkyColor`,
//! `drawClouds` (the cloud colour), `getFogColor`, `getStarBrightness`
//! (World.java:1017-1150), `calcSunriseSunsetColors`, `func_4096_a` (the
//! fog colour), `getCloudHeight` and `func_28112_c` (WorldProvider.java,
//! WorldProviderHell.java).
//!
//! They change no game state, with one vanilla exception: `getSkyColor`
//! asks the biome manager for the temperature and biome at the viewer, and
//! `WorldChunkManager.getTemperature` / `getBiomeGenAt` refill the manager's
//! output arrays (`temperature`, `biomeGenBase`) as they do in vanilla every
//! frame. Those arrays are scratch space every generator call refills
//! before reading, so the renderer calling it is invisible to the
//! simulation, as in vanilla. That is why [`World::get_sky_color`] takes
//! `&mut self`; the rest take `&self`.
//!
//! The sky dimension (`WorldProviderSky`'s overrides) is out of scope
//! (PLAN_v4 D3, deferred).

use mc_core::Vec3D;
use mc_core::math::{self, floor_double};

use super::World;
use super::provider::{Dimension, WorldProvider};

impl WorldProvider {
    /// `calcSunriseSunsetColors(celestialAngle, partialTick)`: the colour and
    /// alpha of the sunrise/sunset glow, `None` outside the 0.4-wide band
    /// around the horizon. The nether inherits it (its sky is never drawn).
    // src: WorldProvider.java:59-75
    pub fn calc_sunrise_sunset_colors(&self, var1: f32, _var2: f32) -> Option<[f32; 4]> {
        let var3 = 0.4f32;
        let var4 = math::cos(var1 * 3.1415927 * 2.0) - 0.0;
        let var5 = -0.0f32;
        if var4 >= var5 - var3 && var4 <= var5 + var3 {
            let var6 = (var4 - var5) / var3 * 0.5 + 0.5;
            let mut var7 = 1.0 - (1.0 - math::sin(var6 * 3.1415927)) * 0.99;
            var7 *= var7;
            Some([
                var6 * 0.3 + 0.7,
                var6 * var6 * 0.7 + 0.2,
                var6 * var6 * 0.0 + 0.2,
                var7,
            ])
        } else {
            None
        }
    }

    /// `func_4096_a(celestialAngle, partialTick)`: the fog colour, a light
    /// blue (`0xC0D8FF`) dimmed by the time of day; the nether's is a
    /// constant dark red.
    // src: WorldProvider.java:77-94, WorldProviderHell.java:12-14
    #[allow(clippy::manual_clamp)] // Java's two ifs
    pub fn func_4096_a(&self, var1: f32, _var2: f32) -> Vec3D {
        if self.dimension == Dimension::Hell {
            return Vec3D::create_vector(
                0.20000000298023224,
                0.029999999329447746,
                0.029999999329447746,
            );
        }
        let mut var3 = math::cos(var1 * 3.1415927 * 2.0) * 2.0 + 0.5;
        if var3 < 0.0 {
            var3 = 0.0;
        }
        if var3 > 1.0 {
            var3 = 1.0;
        }
        let mut var4 = 0.7529412f32;
        let mut var5 = 0.84705883f32;
        let mut var6 = 1.0f32;
        var4 *= var3 * 0.94 + 0.06;
        var5 *= var3 * 0.94 + 0.06;
        var6 *= var3 * 0.91 + 0.09;
        Vec3D::create_vector(var4 as f64, var5 as f64, var6 as f64)
    }

    /// `getCloudHeight()`: 108 (the sky dimension's 8 is deferred).
    // src: WorldProvider.java:110-112
    pub fn get_cloud_height(&self) -> f32 {
        108.0
    }

    /// `func_28112_c()`: whether the void below the horizon takes the dark
    /// blue tint (`RenderGlobal.renderSky`); true except in the (deferred)
    /// sky dimension.
    // src: WorldProvider.java:114-116
    pub fn func_28112_c(&self) -> bool {
        true
    }
}

impl World {
    /// `getSkyColor(entity, partialTick)` for a viewer at `(pos_x, pos_z)`
    /// (the entity's `posX`/`posZ`): the biome's temperature sky colour
    /// dimmed by the time of day, greyed by rain and thunder, and brightened
    /// by a lightning flash (`field_27172_i`). `&mut self`: the biome
    /// manager's caches (module docs).
    // src: World.java:1017-1071
    #[allow(clippy::manual_clamp)] // Java's two ifs
    pub fn get_sky_color(&mut self, pos_x: f64, pos_z: f64, var2: f32) -> Vec3D {
        let var3 = self.get_celestial_angle(var2);
        let mut var4 = math::cos(var3 * 3.1415927 * 2.0) * 2.0 + 0.5;
        if var4 < 0.0 {
            var4 = 0.0;
        }
        if var4 > 1.0 {
            var4 = 1.0;
        }
        let var5 = floor_double(pos_x);
        let var6 = floor_double(pos_z);
        let mgr = self.get_world_chunk_manager();
        let var7 = mgr.get_temperature(var5, var6) as f32;
        let var8 = mgr.get_biome_gen_at(var5, var6).get_sky_color_by_temp(var7);
        let mut var9 = (var8 >> 16 & 255) as f32 / 255.0;
        let mut var10 = (var8 >> 8 & 255) as f32 / 255.0;
        let mut var11 = (var8 & 255) as f32 / 255.0;
        var9 *= var4;
        var10 *= var4;
        var11 *= var4;
        let var12 = self.get_rain_strength(var2);
        if var12 > 0.0 {
            let var13 = (var9 * 0.3 + var10 * 0.59 + var11 * 0.11) * 0.6;
            let var14 = 1.0 - var12 * 0.75;
            var9 = var9 * var14 + var13 * (1.0 - var14);
            var10 = var10 * var14 + var13 * (1.0 - var14);
            var11 = var11 * var14 + var13 * (1.0 - var14);
        }
        let var13 = self.get_weighted_thunder_strength(var2);
        if var13 > 0.0 {
            let var14 = (var9 * 0.3 + var10 * 0.59 + var11 * 0.11) * 0.2;
            let var15 = 1.0 - var13 * 0.75;
            var9 = var9 * var15 + var14 * (1.0 - var15);
            var10 = var10 * var15 + var14 * (1.0 - var15);
            var11 = var11 * var15 + var14 * (1.0 - var15);
        }
        if self.field_27172_i > 0 {
            let mut var14 = self.field_27172_i as f32 - var2;
            if var14 > 1.0 {
                var14 = 1.0;
            }
            var14 *= 0.45;
            var9 = var9 * (1.0 - var14) + 0.8 * var14;
            var10 = var10 * (1.0 - var14) + 0.8 * var14;
            var11 = var11 * (1.0 - var14) + 1.0 * var14;
        }
        Vec3D::create_vector(var9 as f64, var10 as f64, var11 as f64)
    }

    /// `drawClouds(partialTick)`: the cloud colour (`field_1019_F`, white)
    /// greyed by rain, dimmed by the time of day, darkened by thunder.
    // src: World.java:1077-1115
    #[allow(clippy::manual_clamp)] // Java's two ifs
    pub fn draw_clouds(&self, var1: f32) -> Vec3D {
        let var2 = self.get_celestial_angle(var1);
        let mut var3 = math::cos(var2 * 3.1415927 * 2.0) * 2.0 + 0.5;
        if var3 < 0.0 {
            var3 = 0.0;
        }
        if var3 > 1.0 {
            var3 = 1.0;
        }
        let mut var4 = (self.field_1019_f >> 16 & 255) as f32 / 255.0;
        let mut var5 = (self.field_1019_f >> 8 & 255) as f32 / 255.0;
        let mut var6 = (self.field_1019_f & 255) as f32 / 255.0;
        let var7 = self.get_rain_strength(var1);
        if var7 > 0.0 {
            let var8 = (var4 * 0.3 + var5 * 0.59 + var6 * 0.11) * 0.6;
            let var9 = 1.0 - var7 * 0.95;
            var4 = var4 * var9 + var8 * (1.0 - var9);
            var5 = var5 * var9 + var8 * (1.0 - var9);
            var6 = var6 * var9 + var8 * (1.0 - var9);
        }
        var4 *= var3 * 0.9 + 0.1;
        var5 *= var3 * 0.9 + 0.1;
        var6 *= var3 * 0.85 + 0.15;
        let var8 = self.get_weighted_thunder_strength(var1);
        if var8 > 0.0 {
            let var9 = (var4 * 0.3 + var5 * 0.59 + var6 * 0.11) * 0.2;
            let var10 = 1.0 - var8 * 0.95;
            var4 = var4 * var10 + var9 * (1.0 - var10);
            var5 = var5 * var10 + var9 * (1.0 - var10);
            var6 = var6 * var10 + var9 * (1.0 - var10);
        }
        Vec3D::create_vector(var4 as f64, var5 as f64, var6 as f64)
    }

    /// `getFogColor(partialTick)` = `worldProvider.func_4096_a(angle, partial)`.
    // src: World.java:1117-1120
    pub fn get_fog_color(&self, var1: f32) -> Vec3D {
        let var2 = self.get_celestial_angle(var1);
        self.world_provider.func_4096_a(var2, var1)
    }

    /// `getStarBrightness(partialTick)`: 0 by day, up to 0.5 at midnight.
    // src: World.java:1138-1150
    #[allow(clippy::manual_clamp)] // Java's two ifs
    pub fn get_star_brightness(&self, var1: f32) -> f32 {
        let var2 = self.get_celestial_angle(var1);
        let mut var3 = 1.0f32 - (math::cos(var2 * 3.1415927 * 2.0) * 2.0 + 0.75);
        if var3 < 0.0 {
            var3 = 0.0;
        }
        if var3 > 1.0 {
            var3 = 1.0;
        }
        var3 * var3 * 0.5
    }
}

#[cfg(test)]
mod tests {
    //! Expected values from the vanilla classes (the oracle build, intrinsics
    //! off), printed with `Double.toString`/`Float.toString`: a `World`
    //! with `WorldInfo(123, "smoke")` and a surface provider, the viewer at
    //! (0.5, 0.5) unless noted.
    use super::*;
    use crate::save::NoSave;
    use crate::world::WorldEnvironment;
    use crate::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};

    fn world(dim: Dimension) -> World {
        World::new(
            Box::new(NoSave),
            "smoke",
            WorldProvider::new(dim),
            123,
            ChunkGenerator::Empty(EmptyChunkProvider),
            WorldEnvironment {
                rand_seed: 0,
                field_9437_g: 0,
                lock_timestamp: 0,
            },
        )
    }

    fn weather(w: &mut World, rain: f32, thunder: f32) {
        w.raining_strength = rain;
        w.prev_raining_strength = rain;
        w.thundering_strength = thunder;
        w.prev_thundering_strength = thunder;
    }

    fn v(x: Vec3D) -> [f64; 3] {
        [x.x_coord, x.y_coord, x.z_coord]
    }

    #[test]
    fn colours_through_the_day() {
        let mut w = world(Dimension::Surface);
        #[allow(clippy::type_complexity)]
        let cases: [(i64, f32, f32, [f64; 3], [f64; 3], [f64; 3], f32); 8] = [
            (
                0,
                0.0,
                0.0,
                [0.40132367610931396, 0.6494147181510925, 0.9303412437438965],
                [0.9373071193695068, 0.9373071193695068, 0.940790057182312],
                [0.7036392092704773, 0.7915940880775452, 0.9366105794906616],
                0.0,
            ),
            (
                0,
                1.0,
                1.0,
                [0.15686365962028503, 0.1723693460226059, 0.18992726504802704],
                [0.1395164430141449, 0.1395164430141449, 0.13962441682815552],
                [0.7036392092704773, 0.7915940880775452, 0.9366105794906616],
                0.0,
            ),
            (
                6000,
                0.0,
                0.0,
                [0.4313725531101227, 0.6980392336845398, 1.0],
                [1.0, 1.0, 1.0],
                [0.7529411911964417, 0.8470588326454163, 1.0],
                0.0,
            ),
            (
                6000,
                1.0,
                0.0,
                [0.40090784430503845, 0.4675745368003845, 0.5430647134780884],
                [0.6200000047683716, 0.6200000047683716, 0.6200000047683716],
                [0.7529411911964417, 0.8470588326454163, 1.0],
                0.0,
            ),
            (
                12000,
                0.0,
                0.0,
                [0.401404470205307, 0.649545431137085, 0.9305285215377808],
                [0.9374756813049316, 0.9374756813049316, 0.9409493207931519],
                [0.703771710395813, 0.7917431592941284, 0.9367809295654297],
                0.0,
            ),
            (
                18000,
                0.0,
                0.0,
                [0.0, 0.0, 0.0],
                [
                    0.10000000149011612,
                    0.10000000149011612,
                    0.15000000596046448,
                ],
                [
                    0.045176468789577484,
                    0.050823528319597244,
                    0.09000000357627869,
                ],
                0.5,
            ),
            (
                18000,
                1.0,
                1.0,
                [0.0, 0.0, 0.0],
                [
                    0.01552790030837059,
                    0.01552790030837059,
                    0.01707790046930313,
                ],
                [
                    0.045176468789577484,
                    0.050823528319597244,
                    0.09000000357627869,
                ],
                0.5,
            ),
            (
                23000,
                1.0,
                0.0,
                [
                    0.15182310342788696,
                    0.17706964910030365,
                    0.20565766096115112,
                ],
                [0.27331361174583435, 0.27331361174583435, 0.2925739884376526],
                [0.313205748796463, 0.3523564636707306, 0.43461543321609497],
                0.0689325,
            ),
        ];
        for (t, rain, thunder, sky, clouds, fog, stars) in cases {
            w.set_world_time(t);
            weather(&mut w, rain, thunder);
            assert_eq!(
                v(w.get_sky_color(0.5, 0.5, 0.0)),
                sky,
                "sky t={t} {rain} {thunder}"
            );
            assert_eq!(
                v(w.draw_clouds(0.0)),
                clouds,
                "clouds t={t} {rain} {thunder}"
            );
            assert_eq!(v(w.get_fog_color(0.0)), fog, "fog t={t}");
            assert_eq!(w.get_star_brightness(0.0), stars, "stars t={t}");
        }
    }

    #[test]
    fn sky_colour_follows_the_biome_and_the_flash() {
        let mut w = world(Dimension::Surface);
        w.set_world_time(6000);
        for (x, sky) in [
            (100.5, [0.5647059082984924, 0.5960784554481506, 1.0]),
            (-250.5, [0.4117647111415863, 0.7254902124404907, 1.0]),
            (1000.5, [0.4000000059604645, 0.7411764860153198, 1.0]),
        ] {
            assert_eq!(v(w.get_sky_color(x, x * 0.5, 0.0)), sky, "x={x}");
        }
        w.field_27172_i = 2;
        assert_eq!(
            v(w.get_sky_color(0.5, 0.5, 0.5)),
            [0.5972548723220825, 0.7439215779304504, 1.0]
        );
    }

    #[test]
    fn sunrise_and_fog_colour_by_angle() {
        let p = WorldProvider::new(Dimension::Surface);
        let cases: [(f32, Option<[f32; 4]>, [f64; 3]); 6] = [
            (0.0, None, [0.7529411911964417, 0.8470588326454163, 1.0]),
            (
                0.25,
                Some([0.85, 0.375, 0.2, 1.0]),
                [0.39905881881713867, 0.448941171169281, 0.5450000166893005],
            ),
            (
                0.5,
                None,
                [
                    0.045176468789577484,
                    0.050823528319597244,
                    0.09000000357627869,
                ],
            ),
            (
                0.75,
                Some([0.85, 0.375, 0.2, 1.0]),
                [0.39905881881713867, 0.448941171169281, 0.5450000166893005],
            ),
            (
                0.23,
                Some([0.89700997, 0.50187826, 0.2, 0.7787619]),
                [0.576509416103363, 0.648573100566864, 0.7731549739837646],
            ),
            (
                0.8,
                Some([0.96585405, 0.7497206, 0.2, 0.12711751]),
                [0.7529411911964417, 0.8470588326454163, 1.0],
            ),
        ];
        for (a, sun, fog) in cases {
            assert_eq!(p.calc_sunrise_sunset_colors(a, 0.0), sun, "angle {a}");
            assert_eq!(v(p.func_4096_a(a, 0.0)), fog, "angle {a}");
        }
        let h = WorldProvider::new(Dimension::Hell);
        assert_eq!(
            v(h.func_4096_a(0.3, 0.0)),
            [
                0.20000000298023224,
                0.029999999329447746,
                0.029999999329447746
            ]
        );
        assert_eq!(p.get_cloud_height(), 108.0);
        assert!(p.func_28112_c());
    }
}
