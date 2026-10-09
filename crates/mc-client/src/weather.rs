//! `EntityRenderer.renderRainSnow` (EntityRenderer.java:636-775): while it
//! rains, two crossed quads per block column around the viewer (radius 5,
//! or 10 with fancy graphics), from the column's top solid block (or the
//! viewer's level) up to 5/10 blocks above the viewer — snow
//! (`/environment/snow.png`) in snowy biomes, then rain
//! (`/environment/rain.png`) in rainy ones, each column with its own
//! scroll and offsets from the reseeded `random`, brightness from the light
//! at the column and alpha falling off with distance.
//!
//! GL state (EntityRenderer.java:645-649, :546): culling off, blending
//! (770, 771), the alpha test at 0.01, depth writes on, fog `setupFog(0)`.
//! `addRainParticles` (the splashes and the rain sound) is WP-3.7's.
//!
//! The textures are local handles; they could move to WP-3.2's texture
//! cache.

use bevy::camera::visibility::NoFrustumCulling;
use bevy::prelude::*;
use mc_core::java_random::JavaRandom;
use mc_core::math::{floor_double, sqrt_double};
use mc_sim::world::World;

use crate::assets::JarAssets;
use crate::fog::FogState;
use crate::gl_material::{GlBlend, GlKey, GlMaterial, TessBatch, jar_texture, order};
use crate::sky::ViewInfo;

/// `/environment/snow.png`.
pub const SNOW_PNG: &str = "/environment/snow.png";
/// `/environment/rain.png`.
pub const RAIN_PNG: &str = "/environment/rain.png";

/// `EntityRenderer.random` (`new Random()`): `renderRainSnow` reseeds it
/// before every use, so its first seed never shows.
// src: EntityRenderer.java:43
#[derive(Resource, Clone, Debug)]
pub struct WeatherRandom(pub JavaRandom);

impl Default for WeatherRandom {
    fn default() -> Self {
        WeatherRandom(JavaRandom::new(0))
    }
}

/// The weather of this frame.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct WeatherFrame {
    /// Whether it rains (`getRainStrength(partial) > 0`).
    pub visible: bool,
    /// The eye.
    pub anchor: Vec3,
    /// The snow quads (relative to the eye).
    pub snow: TessBatch,
    /// The rain quads.
    pub rain: TessBatch,
}

/// The per-column seed `x·x·3121 + x·45238971 + z·z·418711 + z·13761`
/// (int arithmetic, wrapping).
// src: EntityRenderer.java:696, :747-748
pub fn column_hash(x: i32, z: i32) -> i32 {
    x.wrapping_mul(x)
        .wrapping_mul(3121)
        .wrapping_add(x.wrapping_mul(45238971))
        .wrapping_add(z.wrapping_mul(z).wrapping_mul(418711))
        .wrapping_add(z.wrapping_mul(13761))
}

/// Rain's texture scroll for a column, after reseeding `random` with the
/// hash: `((rendererUpdateCount + hash & 31) + partial) / 32 · (3 +
/// random.nextFloat())`.
// src: EntityRenderer.java:747-748
pub fn rain_scroll(
    rng: &mut JavaRandom,
    renderer_update_count: i32,
    x: i32,
    z: i32,
    partial: f32,
) -> f32 {
    let h = column_hash(x, z);
    rng.set_seed(h as i64);
    (((renderer_update_count.wrapping_add(h)) & 31) as f32 + partial) / 32.0
        * (3.0 + rng.next_float())
}

/// Snow's texture offsets for a column: `(nextFloat + t·0.01·gaussian,
/// nextFloat + t·gaussian·0.001)` with `t = rendererUpdateCount + partial`,
/// after reseeding `random` with the hash; and the slow fall `((count &
/// 511) + partial) / 512`.
// src: EntityRenderer.java:696-700
pub fn snow_offsets(
    rng: &mut JavaRandom,
    renderer_update_count: i32,
    x: i32,
    z: i32,
    partial: f32,
) -> (f32, f32, f32) {
    rng.set_seed(column_hash(x, z) as i64);
    let var27 = renderer_update_count as f32 + partial;
    let var28 = ((renderer_update_count & 511) as f32 + partial) / 512.0;
    let a = rng.next_float();
    let var29 = a + var27 * 0.01 * rng.next_gaussian() as f32;
    let b = rng.next_float();
    let var30 = b + var27 * rng.next_gaussian() as f32 * 0.001;
    (var28, var29, var30)
}

/// The two crossed quads of a column `x, z` from `y0` to `y1`, with the
/// texture's `u` offset `du` and `v = y / 4 + dv + dv2` (in that float
/// order), relative to `anchor`
/// (`setTranslationD(-x, -y, -z)`), in colour `c`.
// src: EntityRenderer.java:707-716, :755-764
#[allow(clippy::too_many_arguments)]
fn column_quads(
    b: &mut TessBatch,
    anchor: [f64; 3],
    x: i32,
    z: i32,
    y0: i32,
    y1: i32,
    du: f32,
    dv: f32,
    dv2: f32,
    c: [f32; 4],
) {
    let var26 = 1.0f32;
    let p = |px: f64, py: f64, pz: f64| {
        [
            (px - anchor[0]) as f32,
            (py - anchor[1]) as f32,
            (pz - anchor[2]) as f32,
        ]
    };
    let v = |y: i32| y as f32 * var26 / 4.0 + dv * var26 + dv2;
    let (fx, fz) = (x as f64, z as f64);
    let (fy0, fy1) = (y0 as f64, y1 as f64);
    b.vertex(p(fx, fy0, fz + 0.5), [0.0 * var26 + du, v(y0)], c);
    b.vertex(p(fx + 1.0, fy0, fz + 0.5), [1.0 * var26 + du, v(y0)], c);
    b.vertex(p(fx + 1.0, fy1, fz + 0.5), [1.0 * var26 + du, v(y1)], c);
    b.vertex(p(fx, fy1, fz + 0.5), [0.0 * var26 + du, v(y1)], c);
    b.vertex(p(fx + 0.5, fy0, fz), [0.0 * var26 + du, v(y0)], c);
    b.vertex(p(fx + 0.5, fy0, fz + 1.0), [1.0 * var26 + du, v(y0)], c);
    b.vertex(p(fx + 0.5, fy1, fz + 1.0), [1.0 * var26 + du, v(y1)], c);
    b.vertex(p(fx + 0.5, fy1, fz), [0.0 * var26 + du, v(y1)], c);
}

/// `renderRainSnow(partial)`'s quads (module docs). The `glColor4f` of each
/// column becomes its vertex colour.
// src: EntityRenderer.java:636-775
pub fn compute(
    world: &mut World,
    view: &ViewInfo,
    fancy: bool,
    renderer_update_count: i32,
    partial: f32,
    rng: &mut JavaRandom,
) -> WeatherFrame {
    let var2 = world.get_rain_strength(partial);
    if var2 <= 0.0 {
        return WeatherFrame::default();
    }
    let var5 = floor_double(view.pos.x);
    let var6 = floor_double(view.pos.y);
    let var7 = floor_double(view.pos.z);
    let a = view.last_tick_interp(partial);
    let anchor = [a.x, a.y, a.z];
    let var15 = floor_double(a.y);
    let mut var16 = 5i32;
    if fancy {
        var16 = 10;
    }
    let n = var16 * 2 + 1;
    let var17: Vec<_> =
        world
            .get_world_chunk_manager()
            .get_biome_gen_base_at(var5 - var16, var7 - var16, n, n)[..(n * n) as usize]
            .to_vec();
    let mut snow = TessBatch::default();
    let mut var18 = 0usize;
    for var19 in (var5 - var16)..=(var5 + var16) {
        for var20 in (var7 - var16)..=(var7 + var16) {
            let var21 = var17[var18];
            var18 += 1;
            if var21.get_enable_snow() {
                let mut var22 = world.find_top_solid_block(var19, var20);
                if var22 < 0 {
                    var22 = 0;
                }
                let mut var23 = var22;
                if var22 < var15 {
                    var23 = var15;
                }
                let mut var24 = var6 - var16;
                let mut var25 = var6 + var16;
                if var24 < var22 {
                    var24 = var22;
                }
                if var25 < var22 {
                    var25 = var22;
                }
                if var24 != var25 {
                    let (var28, var29, var30) =
                        snow_offsets(rng, renderer_update_count, var19, var20, partial);
                    let var31 = ((var19 as f32 + 0.5) as f64) - view.pos.x;
                    let var33 = ((var20 as f32 + 0.5) as f64) - view.pos.z;
                    let var35 = sqrt_double(var31 * var31 + var33 * var33) / var16 as f32;
                    let var36 = world.get_light_brightness(var19, var23, var20);
                    let alpha = ((1.0 - var35 * var35) * 0.3 + 0.5) * var2;
                    column_quads(
                        &mut snow,
                        anchor,
                        var19,
                        var20,
                        var24,
                        var25,
                        var29,
                        var28,
                        var30,
                        [var36, var36, var36, alpha],
                    );
                }
            }
        }
    }
    if fancy {
        var16 = 10;
    }
    let mut rain = TessBatch::default();
    var18 = 0;
    for var19 in (var5 - var16)..=(var5 + var16) {
        for var20 in (var7 - var16)..=(var7 + var16) {
            let var21 = var17[var18];
            var18 += 1;
            if var21.can_spawn_lightning_bolt() {
                let var22 = world.find_top_solid_block(var19, var20);
                let mut var23 = var6 - var16;
                let mut var24 = var6 + var16;
                if var23 < var22 {
                    var23 = var22;
                }
                if var24 < var22 {
                    var24 = var22;
                }
                if var23 != var24 {
                    let var26 = rain_scroll(rng, renderer_update_count, var19, var20, partial);
                    let var38 = ((var19 as f32 + 0.5) as f64) - view.pos.x;
                    let var39 = ((var20 as f32 + 0.5) as f64) - view.pos.z;
                    let var40 = sqrt_double(var38 * var38 + var39 * var39) / var16 as f32;
                    let var32 = world.get_light_brightness(var19, 128, var20) * 0.85 + 0.15;
                    let alpha = ((1.0 - var40 * var40) * 0.5 + 0.5) * var2;
                    column_quads(
                        &mut rain,
                        anchor,
                        var19,
                        var20,
                        var23,
                        var24,
                        0.0,
                        var26,
                        0.0,
                        [var32, var32, var32, alpha],
                    );
                }
            }
        }
    }
    WeatherFrame {
        visible: true,
        anchor: Vec3::ZERO,
        snow,
        rain,
    }
}

/// Which weather entity.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeatherPart {
    /// Snow.
    Snow,
    /// Rain.
    Rain,
}

/// The weather's meshes and materials.
#[derive(Resource, Clone, Debug)]
pub struct WeatherHandles {
    /// Per part: mesh, material.
    pub parts: Vec<(WeatherPart, Handle<Mesh>, Handle<GlMaterial>)>,
}

/// Spawns the weather entities (hidden until it rains).
pub fn spawn_weather(
    mut commands: Commands,
    jar: Res<JarAssets>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GlMaterial>>,
) {
    let mut parts = Vec::new();
    for (part, png, bias) in [
        (WeatherPart::Snow, SNOW_PNG, order::SNOW),
        (WeatherPart::Rain, RAIN_PNG, order::RAIN),
    ] {
        let mut m = GlMaterial::new(
            GlKey {
                blend: GlBlend::Alpha,
                depth_write: true,
                color_write: true,
                far: false,
                wide_lines: false,
            },
            bias,
        )
        // glAlphaFunc(GL_GREATER, 0.01F) (EntityRenderer.java:649)
        .with_alpha_test(0.01);
        match jar_texture(&jar, png, false) {
            Ok(i) => m = m.with_texture(images.add(i)),
            Err(e) => error!("{e}"),
        }
        let mh = materials.add(m);
        let mesh = meshes.add(TessBatch::placeholder_mesh());
        parts.push((part, mesh.clone(), mh.clone()));
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(mh),
            Transform::IDENTITY,
            Visibility::Hidden,
            NoFrustumCulling,
            part,
        ));
    }
    commands.insert_resource(WeatherHandles { parts });
}

/// Applies [`WeatherFrame`].
pub fn apply_weather(
    frame: Res<WeatherFrame>,
    fog: Res<FogState>,
    handles: Option<Res<WeatherHandles>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GlMaterial>>,
    mut q: Query<(&WeatherPart, &mut Transform, &mut Visibility)>,
) {
    let _perf = crate::perf::time(crate::perf::Phase::Sky);
    let Some(h) = handles else {
        return;
    };
    let batch = |p: WeatherPart| match p {
        WeatherPart::Snow => &frame.snow,
        WeatherPart::Rain => &frame.rain,
    };
    for (part, mut t, mut vis) in &mut q {
        let shown = frame.visible && !batch(*part).is_empty();
        let want = if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
        t.translation = frame.anchor;
    }
    if !frame.visible {
        return;
    }
    for (part, mesh, mh) in &h.parts {
        let b = batch(*part);
        if b.is_empty() {
            continue;
        }
        if let Some(mut m) = meshes.get_mut(mesh) {
            b.write_into(&mut m, b.quad_indices());
        }
        if let Some(mut m) = materials.get_mut(mh) {
            m.params.fog = fog.world.to_vec4();
            m.params.fog_color = Vec4::new(fog.color[0], fog.color[1], fog.color[2], 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_hash_wraps_like_java_ints() {
        assert_eq!(column_hash(0, 0), 0);
        assert_eq!(column_hash(1, 1), 3121 + 45238971 + 418711 + 13761);
        // 100·100·3121 + 100·45238971 overflows an int.
        let java = (100i64 * 100 * 3121 + 100 * 45238971 + 10000 * 418711 + 100 * 13761) as i32;
        assert_eq!(column_hash(100, 100), java);
    }

    #[test]
    fn rain_scroll_cycles_every_32_ticks() {
        let mut r = JavaRandom::new(0);
        let a = rain_scroll(&mut r, 5, 3, 4, 0.25);
        let b = rain_scroll(&mut r, 5 + 32, 3, 4, 0.25);
        assert_eq!(a, b);
        // ((5 + h & 31) + 0.25) / 32 · (3 + nextFloat of Random(h)).
        let h = column_hash(3, 4);
        let f = JavaRandom::new(h as i64).next_float();
        assert_eq!(a, (((5 + h) & 31) as f32 + 0.25) / 32.0 * (3.0 + f));
        // One tick later the texture moved 1/32 · (3 + f) (mod the cycle).
        let c = rain_scroll(&mut r, 6, 3, 4, 0.25);
        if (5 + h) & 31 != 31 {
            assert!((c - a - (3.0 + f) / 32.0).abs() < 1e-5);
        }
    }

    #[test]
    fn snow_offsets_draw_in_java_order() {
        let mut r = JavaRandom::new(0);
        let (fall, du, dv) = snow_offsets(&mut r, 1000, -7, 12, 0.5);
        assert_eq!(fall, ((1000 & 511) as f32 + 0.5) / 512.0);
        let mut j = JavaRandom::new(column_hash(-7, 12) as i64);
        let t = 1000.5f32;
        let a = j.next_float();
        let ga = j.next_gaussian() as f32;
        let b = j.next_float();
        let gb = j.next_gaussian() as f32;
        assert_eq!(du, a + t * 0.01 * ga);
        assert_eq!(dv, b + t * gb * 0.001);
    }

    #[test]
    fn column_quads_cross_at_the_centre() {
        let mut b = TessBatch::default();
        column_quads(
            &mut b,
            [0.5, 70.0, 0.5],
            2,
            3,
            64,
            80,
            0.0,
            0.5,
            0.0,
            [1.0; 4],
        );
        assert_eq!(b.len(), 8);
        assert_eq!(b.positions[0], [1.5, -6.0, 3.0]);
        assert_eq!(b.positions[2], [2.5, 10.0, 3.0]);
        assert_eq!(b.positions[4], [2.0, -6.0, 2.5]);
        assert_eq!(b.uvs[0], [0.0, 64.0 / 4.0 + 0.5]);
        assert_eq!(b.uvs[2], [1.0, 80.0 / 4.0 + 0.5]);
    }
}
