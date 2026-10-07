//! The fog half of `EntityRenderer` (EntityRenderer.java): `updateFogColor`
//! (the fog and clear colour) and `setupFog` (the fog mode and range per
//! situation), published every frame as [`FogState`] for every renderer to
//! apply the same way (WP-3.3 terrain, WP-3.5 entities, WP-3.7 particles;
//! here the sky, clouds and weather through [`crate::gl_material`]).
//!
//! # What vanilla does with them
//!
//! `renderWorld` calls `updateFogColor` once per pass, which sets
//! `glClearColor` to the fog colour (so the screen starts as fog), then
//! `setupFog(-1)` before the sky and `setupFog(0)`/`setupFog(1)` around the
//! world, the clouds and the weather (EntityRenderer.java:466-566). The two
//! differ only in the normal linear mode: the sky's fog starts at 0 and ends
//! at 0.8 × `farPlaneDistance`, the world's runs from a quarter of the far
//! plane to the far plane. The far plane is `256 >> renderDistance`.
//!
//! Vanilla asks for `GL_NV_fog_distance`'s radial (eye-distance) fog where
//! the driver has it (EntityRenderer.java:907-909); this port always uses
//! the eye distance. Bevy's [`DistanceFog`] measures the same distance and
//! its linear and exponential falloffs are GL's `GL_LINEAR` and `GL_EXP`, so
//! [`apply_fog`] also keeps one on the world camera, set to the world fog,
//! for `StandardMaterial` users (its blend happens in linear colour space).
//!
//! `cloudFog` is always false: `RenderGlobal.hasCloudFog` returns false
//! (RenderGlobal.java:781-783), so `orientCamera` never sets it; its
//! branches are transcribed for completeness.

use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use mc_core::math::jmath;

use crate::camera::MainCamera;
use crate::gl_material::anaglyph;

/// `GL_FOG_MODE`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GlFogMode {
    /// `GL_LINEAR` (9729): `f = (end - d) / (end - start)`.
    #[default]
    Linear,
    /// `GL_EXP` (2048): `f = exp(-density · d)`.
    Exp,
}

/// One `setupFog` call's fog parameters.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FogParams {
    /// The mode.
    pub mode: GlFogMode,
    /// `GL_FOG_START` (linear).
    pub start: f32,
    /// `GL_FOG_END` (linear).
    pub end: f32,
    /// `GL_FOG_DENSITY` (exponential).
    pub density: f32,
}

impl FogParams {
    /// GL's fog factor at eye distance `d` (1: no fog, 0: all fog), clamped.
    pub fn factor(&self, d: f32) -> f32 {
        let f = match self.mode {
            GlFogMode::Linear => (self.end - d) / (self.end - self.start),
            GlFogMode::Exp => (-self.density * d).exp(),
        };
        f.clamp(0.0, 1.0)
    }

    /// The shader's `fog` vector (`crate::gl_material::GlParams::fog`).
    pub fn to_vec4(&self) -> Vec4 {
        let mode = match self.mode {
            GlFogMode::Linear => 1.0,
            GlFogMode::Exp => 2.0,
        };
        Vec4::new(mode, self.start, self.end, self.density)
    }
}

/// The frame's fog, for every renderer (module docs).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct FogState {
    /// Whether a world is drawn this frame (the rest is meaningless if not).
    pub active: bool,
    /// `fogColorRed/Green/Blue` after `updateFogColor`: `GL_FOG_COLOR` and
    /// the clear colour, in the Java's colour values.
    pub color: [f32; 3],
    /// `setupFog(0)` (= `setupFog(1)`): terrain, entities, particles,
    /// clouds, weather.
    pub world: FogParams,
    /// `setupFog(-1)`: the sky and the void plane.
    pub sky: FogParams,
    /// `farPlaneDistance` (`256 >> renderDistance`).
    pub far_plane_distance: f32,
}

/// What `updateFogColor` reads from the world, the view entity and the
/// settings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FogColorInputs {
    /// `gameSettings.renderDistance` (0 far … 3 tiny).
    pub render_distance: i32,
    /// `world.getSkyColor(view, partial)`.
    pub sky: [f64; 3],
    /// `world.getFogColor(partial)`.
    pub fog: [f64; 3],
    /// `world.getRainStrength(partial)`.
    pub rain: f32,
    /// `world.getWeightedThunderStrength(partial)`.
    pub thunder: f32,
    /// `world.drawClouds(partial)` when `cloudFog` (never; module docs).
    pub cloud_fog: Option<[f64; 3]>,
    /// The view entity's eye is in water.
    pub in_water: bool,
    /// The view entity's eye is in lava.
    pub in_lava: bool,
    /// `fogColor2` (last tick's brightness factor).
    pub fog_color2: f32,
    /// `fogColor1` (this tick's).
    pub fog_color1: f32,
    /// `renderPartialTicks`.
    pub partial: f32,
    /// `gameSettings.anaglyph`.
    pub anaglyph: bool,
}

/// `updateFogColor(partial)`: the fog colour (also the clear colour) — the
/// fog colour pulled toward the sky colour more at long render distances,
/// darkened by rain and thunder, replaced under water or lava, then scaled
/// by the eased brightness at the eye.
// src: EntityRenderer.java:788-852
pub fn update_fog_color(i: &FogColorInputs) -> [f32; 3] {
    let mut var4 = 1.0f32 / (4 - i.render_distance) as f32;
    var4 = 1.0 - jmath::pow(var4 as f64, 0.25) as f32;
    let var6 = i.sky[0] as f32;
    let var7 = i.sky[1] as f32;
    let var8 = i.sky[2] as f32;
    let mut r = i.fog[0] as f32;
    let mut g = i.fog[1] as f32;
    let mut b = i.fog[2] as f32;
    r += (var6 - r) * var4;
    g += (var7 - g) * var4;
    b += (var8 - b) * var4;
    let var10 = i.rain;
    if var10 > 0.0 {
        let var11 = 1.0 - var10 * 0.5;
        let var12 = 1.0 - var10 * 0.4;
        r *= var11;
        g *= var11;
        b *= var12;
    }
    let var11 = i.thunder;
    if var11 > 0.0 {
        let var12 = 1.0 - var11 * 0.5;
        r *= var12;
        g *= var12;
        b *= var12;
    }
    if let Some(c) = i.cloud_fog {
        r = c[0] as f32;
        g = c[1] as f32;
        b = c[2] as f32;
    } else if i.in_water {
        r = 0.02;
        g = 0.02;
        b = 0.2;
    } else if i.in_lava {
        r = 0.6;
        g = 0.1;
        b = 0.0;
    }
    let var12 = i.fog_color2 + (i.fog_color1 - i.fog_color2) * i.partial;
    r *= var12;
    g *= var12;
    b *= var12;
    if i.anaglyph {
        [r, g, b] = anaglyph([r, g, b]);
    }
    [r, g, b]
}

/// `setupFog(var1, partial)`'s mode and range: exponential in a cloud
/// (never), in water (density 0.1) or lava (2.0); otherwise linear from a
/// quarter of the far plane to the far plane, or from 0 to 0.8 of it for the
/// sky (`var1 < 0`), starting at 0 in the nether. (The anaglyph colours the
/// Java computes in the exponential branches are never used.)
// src: EntityRenderer.java:854-918
#[allow(clippy::if_same_then_else)] // the Java's branches, kept
pub fn setup_fog(
    var1: i32,
    cloud_fog: bool,
    in_water: bool,
    in_lava: bool,
    far_plane_distance: f32,
    is_nether: bool,
) -> FogParams {
    let exp = |density: f32| FogParams {
        mode: GlFogMode::Exp,
        start: 0.0,
        end: 1.0,
        density,
    };
    if cloud_fog {
        exp(0.1)
    } else if in_water {
        exp(0.1)
    } else if in_lava {
        exp(2.0)
    } else {
        let mut p = FogParams {
            mode: GlFogMode::Linear,
            start: far_plane_distance * 0.25,
            end: far_plane_distance,
            density: 1.0,
        };
        if var1 < 0 {
            p.start = 0.0;
            p.end = far_plane_distance * 0.8;
        }
        if is_nether {
            p.start = 0.0;
        }
        p
    }
}

/// `glClearColor` and the world camera's [`DistanceFog`] from [`FogState`]
/// (module docs). Without a world the clear colour is
/// `startGame`'s black (Minecraft.java:314).
pub fn apply_fog(
    mut commands: Commands,
    fog: Res<FogState>,
    mut clear: ResMut<ClearColor>,
    cams: Query<Entity, With<MainCamera>>,
) {
    let c = if fog.active { fog.color } else { [0.0; 3] };
    let want = Color::srgb(c[0], c[1], c[2]);
    if clear.0 != want {
        clear.0 = want;
    }
    for e in &cams {
        if fog.active {
            let falloff = match fog.world.mode {
                GlFogMode::Linear => FogFalloff::Linear {
                    start: fog.world.start,
                    end: fog.world.end,
                },
                GlFogMode::Exp => FogFalloff::Exponential {
                    density: fog.world.density,
                },
            };
            commands.entity(e).insert(DistanceFog {
                color: want,
                directional_light_color: Color::NONE,
                directional_light_exponent: 8.0,
                falloff,
            });
        } else {
            commands.entity(e).remove::<DistanceFog>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs() -> FogColorInputs {
        FogColorInputs {
            render_distance: 0,
            sky: [0.4313725531101227, 0.6980392336845398, 1.0],
            fog: [0.7529411911964417, 0.8470588326454163, 1.0],
            rain: 0.0,
            thunder: 0.0,
            cloud_fog: None,
            in_water: false,
            in_lava: false,
            fog_color2: 1.0,
            fog_color1: 1.0,
            partial: 0.0,
            anaglyph: false,
        }
    }

    #[test]
    fn fog_colour_pulls_toward_the_sky_with_distance() {
        // var4 = 1 - (1 / (4 - rd))^0.25: far 0.29289323, tiny 0.
        let i = inputs();
        let c = update_fog_color(&i);
        let k = 1.0f32 - (0.25f64).powf(0.25) as f32;
        assert!((k - 0.29289323).abs() < 1e-7);
        let fr = 0.7529411911964417f64 as f32;
        assert_eq!(c[0], fr + (0.4313725531101227f64 as f32 - fr) * k);
        let tiny = update_fog_color(&FogColorInputs {
            render_distance: 3,
            ..i
        });
        assert_eq!(tiny, [0.7529412, 0.84705883, 1.0]);
    }

    #[test]
    fn fog_colour_rain_thunder_water_lava_brightness() {
        let i = FogColorInputs {
            render_distance: 3,
            ..inputs()
        };
        let rain = update_fog_color(&FogColorInputs { rain: 1.0, ..i });
        assert_eq!(rain, [0.7529412 * 0.5, 0.84705883 * 0.5, 0.6]);
        let storm = update_fog_color(&FogColorInputs {
            rain: 1.0,
            thunder: 1.0,
            ..i
        });
        assert_eq!(storm, [0.7529412 * 0.5 * 0.5, 0.84705883 * 0.5 * 0.5, 0.3]);
        let water = update_fog_color(&FogColorInputs {
            in_water: true,
            fog_color1: 0.5,
            fog_color2: 0.25,
            partial: 0.5,
            ..i
        });
        assert_eq!(water, [0.02 * 0.375, 0.02 * 0.375, 0.2 * 0.375]);
        let lava = update_fog_color(&FogColorInputs { in_lava: true, ..i });
        assert_eq!(lava, [0.6, 0.1, 0.0]);
    }

    #[test]
    fn fog_ranges_per_mode_and_render_distance() {
        for (rd, far) in [(0, 256.0f32), (1, 128.0), (2, 64.0), (3, 32.0)] {
            assert_eq!((256 >> rd) as f32, far);
            let w = setup_fog(0, false, false, false, far, false);
            assert_eq!(
                (w.mode, w.start, w.end),
                (GlFogMode::Linear, far * 0.25, far)
            );
            let s = setup_fog(-1, false, false, false, far, false);
            assert_eq!(
                (s.mode, s.start, s.end),
                (GlFogMode::Linear, 0.0, far * 0.8)
            );
            let n = setup_fog(0, false, false, false, far, true);
            assert_eq!((n.start, n.end), (0.0, far));
        }
        let w = setup_fog(0, false, true, false, 256.0, false);
        assert_eq!((w.mode, w.density), (GlFogMode::Exp, 0.1));
        let l = setup_fog(-1, false, false, true, 256.0, false);
        assert_eq!((l.mode, l.density), (GlFogMode::Exp, 2.0));
        // Water wins over lava, as the Java's else-if chain.
        let both = setup_fog(0, false, true, true, 256.0, false);
        assert_eq!(both.density, 0.1);
    }

    #[test]
    fn fog_factor_matches_gl() {
        let s = setup_fog(-1, false, false, false, 256.0, false);
        // Looking 30 degrees up at the sky plane (y = 16): d = 32.
        assert!((s.factor(32.0) - (204.8 - 32.0) / 204.8).abs() < 1e-6);
        assert_eq!(s.factor(300.0), 0.0);
        let e = setup_fog(0, false, true, false, 256.0, false);
        assert!((e.factor(10.0) - (-1.0f32).exp()).abs() < 1e-6);
    }
}
