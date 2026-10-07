//! `RenderGlobal.renderSky` (RenderGlobal.java:600-722) and the display
//! lists its constructor builds (RenderGlobal.java:76-166): the sky plane, the
//! sunrise/sunset glow, the sun and moon, the stars and the void plane; plus
//! [`SkyPlugin`], which wires all of WP-3.4 (this module, [`crate::fog`],
//! [`crate::clouds`], [`crate::weather`], [`crate::selection`]) into the
//! frame.
//!
//! # The frame
//!
//! After the shell's frame ([`crate::GameFrameSet`]: ticks, then
//! `setupCameraTransform` into [`CameraState`]), [`compute_render_frame`]
//! reads the world once, in `renderWorld`'s order (EntityRenderer.java:
//! 466-566): `updateFogColor`, `setupFog(-1)` and `renderSky` (only at the
//! far and normal render distances, `renderDistance < 2`), `setupFog(0)`,
//! the selection box, `renderRainSnow`, `renderClouds`. It fills one
//! resource per part ([`crate::fog::FogState`], [`SkyFrame`],
//! [`crate::clouds::CloudFrame`], [`crate::weather::WeatherFrame`],
//! [`crate::selection::SelectionFrame`]); the `apply_*` systems then update
//! the Bevy meshes and materials. Vanilla's second anaglyph pass is
//! not drawn (the shell renders one eye; `camera` module docs), but the
//! anaglyph colour transforms are applied.
//!
//! Positions: vanilla draws the sky, clouds and weather in eye space with
//! the camera's rotations only, so a vertex `v` lands at world `eye + v`
//! (`camera` module docs, "Matrices"); every entity here sits at
//! [`crate::camera::CameraSetup::eye`] with the Java's vertices.
//!
//! # Not here
//!
//! `drawBlockBreaking` (WP-3.3), `playAuxSFX` (WP-3.7/3.8),
//! `addRainParticles` and its rain sound (WP-3.7), the terrain half of
//! `RenderGlobal` (WP-3.3; it may read [`crate::clouds::RenderGlobalClouds`]
//! for `sortAndRender`'s occlusion stagger), the sky dimension (D3).

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::math::DVec3;
use bevy::mesh::PrimitiveTopology;
use bevy::prelude::*;
use mc_core::java_random::JavaRandom;
use mc_core::math::{self, jmath};
use mc_sim::block::Material;
use mc_sim::entity::{Entity, base};
use mc_sim::world::World;

use crate::GameFrameSet;
use crate::assets::JarAssets;
use crate::camera::CameraState;
use crate::clouds::{self, CloudFrame, RenderGlobalClouds};
use crate::fog::{self, FogColorInputs, FogState};
use crate::game::{Game, RunTickHook};
use crate::gl_material::{
    GlBlend, GlKey, GlMaterial, TessBatch, anaglyph, jar_texture, order, tess_color,
};
use crate::selection::{self, SelectionFrame};
use crate::weather::{self, WeatherFrame, WeatherRandom};

/// `/terrain/sun.png`.
pub const SUN_PNG: &str = "/terrain/sun.png";
/// `/terrain/moon.png`.
pub const MOON_PNG: &str = "/terrain/moon.png";

/// What the renderers need of the view entity (`mc.renderViewEntity`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewInfo {
    /// `posX/Y/Z`.
    pub pos: DVec3,
    /// `prevPosX/Y/Z`.
    pub prev_pos: DVec3,
    /// `lastTickPosX/Y/Z`.
    pub last_tick_pos: DVec3,
    /// `isInsideOfMaterial(Material.water)`.
    pub in_water: bool,
    /// `isInsideOfMaterial(Material.lava)`.
    pub in_lava: bool,
    /// `instanceof EntityPlayer`.
    pub is_player: bool,
}

impl ViewInfo {
    /// Reads `e` (in `w`).
    pub fn of(w: &mut World, e: &Entity) -> Self {
        let b = &e.base;
        ViewInfo {
            pos: DVec3::new(b.pos_x, b.pos_y, b.pos_z),
            prev_pos: DVec3::new(b.prev_pos_x, b.prev_pos_y, b.prev_pos_z),
            last_tick_pos: DVec3::new(b.last_tick_pos_x, b.last_tick_pos_y, b.last_tick_pos_z),
            in_water: base::is_inside_of_material(e, w, Material::Water),
            in_lava: base::is_inside_of_material(e, w, Material::Lava),
            is_player: e.kind.player().is_some(),
        }
    }

    /// `lastTickPos + (pos - lastTickPos) * partial`, the anchor
    /// `renderWorld` and `RenderGlobal` translate the world by.
    pub fn last_tick_interp(&self, partial: f32) -> DVec3 {
        let p = partial as f64;
        DVec3::new(
            self.last_tick_pos.x + (self.pos.x - self.last_tick_pos.x) * p,
            self.last_tick_pos.y + (self.pos.y - self.last_tick_pos.y) * p,
            self.last_tick_pos.z + (self.pos.z - self.last_tick_pos.z) * p,
        )
    }

    /// `prevPos + (pos - prevPos) * partial` (the camera's).
    pub fn prev_interp(&self, partial: f32) -> DVec3 {
        let p = partial as f64;
        DVec3::new(
            self.prev_pos.x + (self.pos.x - self.prev_pos.x) * p,
            self.prev_pos.y + (self.pos.y - self.prev_pos.y) * p,
            self.prev_pos.z + (self.pos.z - self.prev_pos.z) * p,
        )
    }
}

/// The view entity of `g` (in its world, or the detached dead player).
pub fn gather_view(g: &mut Game) -> Option<ViewInfo> {
    let id = g.render_view_entity?;
    let world = g.the_world.as_mut()?;
    if let Some(v) = world.with_entity_mut(id, |w, e| ViewInfo::of(w, e)) {
        return Some(v);
    }
    let p = g
        .detached_player
        .as_deref()
        .filter(|p| p.base.entity_id == id)?;
    Some(ViewInfo::of(world, p))
}

// ---- the display lists ------------------------------------------------------------

/// `glSkyList` (y 16, facing down) or `glSkyList2` (y -16, facing up): a
/// 13 × 13 grid of 64-block quads from -384 to 448.
// src: RenderGlobal.java:83-118
pub fn sky_plane_batch(void: bool) -> TessBatch {
    let var6 = 64i32;
    let var7 = 256 / var6 + 2;
    let mut b = TessBatch::default();
    let c = [1.0; 4];
    let mut var8 = -var6 * var7;
    while var8 <= var6 * var7 {
        let mut var9 = -var6 * var7;
        while var9 <= var6 * var7 {
            let (x0, x1, z0, z1) = (
                var8 as f32,
                (var8 + var6) as f32,
                var9 as f32,
                (var9 + var6) as f32,
            );
            if !void {
                let y = 16.0f32;
                b.vertex([x0, y, z0], [0.0; 2], c);
                b.vertex([x1, y, z0], [0.0; 2], c);
                b.vertex([x1, y, z1], [0.0; 2], c);
                b.vertex([x0, y, z1], [0.0; 2], c);
            } else {
                let y = -16.0f32;
                b.vertex([x1, y, z0], [0.0; 2], c);
                b.vertex([x0, y, z0], [0.0; 2], c);
                b.vertex([x0, y, z1], [0.0; 2], c);
                b.vertex([x1, y, z1], [0.0; 2], c);
            }
            var9 += var6;
        }
        var8 += var6;
    }
    b
}

/// `renderStars`: up to 1500 random directions from `new Random(10842)`
/// (those inside the unit ball and not too short), each a small quad at
/// distance 100 turned by a random angle; vertices as the Tessellator's
/// `(float)` of the doubles. `Math.sin/cos/atan2/sqrt` are fdlibm
/// (PLAN_v4 §3.4 rule 5).
// src: RenderGlobal.java:121-166
pub fn star_quads() -> Vec<[[f64; 3]; 4]> {
    let mut var1 = JavaRandom::new(10842);
    let mut out = Vec::new();
    for _ in 0..1500 {
        let mut var4 = (var1.next_float() * 2.0 - 1.0) as f64;
        let mut var6 = (var1.next_float() * 2.0 - 1.0) as f64;
        let mut var8 = (var1.next_float() * 2.0 - 1.0) as f64;
        let var10 = (0.25f32 + var1.next_float() * 0.25) as f64;
        let mut var12 = var4 * var4 + var6 * var6 + var8 * var8;
        if var12 < 1.0 && var12 > 0.01 {
            var12 = 1.0 / jmath::sqrt(var12);
            var4 *= var12;
            var6 *= var12;
            var8 *= var12;
            let var14 = var4 * 100.0;
            let var16 = var6 * 100.0;
            let var18 = var8 * 100.0;
            let var20 = jmath::atan2(var4, var8);
            let var22 = jmath::sin(var20);
            let var24 = jmath::cos(var20);
            let var26 = jmath::atan2(jmath::sqrt(var4 * var4 + var8 * var8), var6);
            let var28 = jmath::sin(var26);
            let var30 = jmath::cos(var26);
            let var32 = var1.next_double() * 3.141592653589793 * 2.0;
            let var34 = jmath::sin(var32);
            let var36 = jmath::cos(var32);
            let mut q = [[0.0; 3]; 4];
            for (var38, v) in q.iter_mut().enumerate() {
                let var38 = var38 as i32;
                let var39 = 0.0f64;
                let var41 = ((var38 & 2) - 1) as f64 * var10;
                let var43 = (((var38 + 1) & 2) - 1) as f64 * var10;
                let var47 = var41 * var36 - var43 * var34;
                let var49 = var43 * var36 + var41 * var34;
                let var53 = var47 * var28 + var39 * var30;
                let var55 = var39 * var28 - var47 * var30;
                let var57 = var55 * var22 - var49 * var24;
                let var61 = var49 * var22 + var55 * var24;
                *v = [var14 + var57, var16 + var53, var18 + var61];
            }
            out.push(q);
        }
    }
    out
}

/// The star list as a batch.
pub fn star_batch() -> TessBatch {
    let mut b = TessBatch::default();
    for q in star_quads() {
        for v in q {
            b.vertex([v[0] as f32, v[1] as f32, v[2] as f32], [0.0; 2], [1.0; 4]);
        }
    }
    b
}

/// The sun's quad (±30 at y 100, facing down) or the moon's (±20 at y -100,
/// facing up), with their texture coordinates.
// src: RenderGlobal.java:683-698
pub fn sun_moon_batch(moon: bool) -> TessBatch {
    let mut b = TessBatch::default();
    let c = [1.0; 4];
    if !moon {
        let var11 = 30.0f32;
        b.vertex([-var11, 100.0, -var11], [0.0, 0.0], c);
        b.vertex([var11, 100.0, -var11], [1.0, 0.0], c);
        b.vertex([var11, 100.0, var11], [1.0, 1.0], c);
        b.vertex([-var11, 100.0, var11], [0.0, 1.0], c);
    } else {
        let var11 = 20.0f32;
        b.vertex([-var11, -100.0, var11], [1.0, 1.0], c);
        b.vertex([var11, -100.0, var11], [0.0, 1.0], c);
        b.vertex([var11, -100.0, -var11], [0.0, 0.0], c);
        b.vertex([-var11, -100.0, -var11], [1.0, 0.0], c);
    }
    b
}

/// The sunrise/sunset glow for `calcSunriseSunsetColors`' `c`: a triangle
/// fan (`startDrawing(6)`) from a centre at (0, 100, 0) in the glow's colour
/// and alpha to 17 rim points of radius 120 with alpha 0, bent back by
/// `-cos · 40 · alpha`. Only the centre colour takes the anaglyph
/// transform, as in the Java.
// src: RenderGlobal.java:641-667
pub fn sunrise_batch(c: [f32; 4], anaglyph_on: bool) -> (TessBatch, Vec<u32>) {
    let [mut var9, mut var10, mut var11] = [c[0], c[1], c[2]];
    if anaglyph_on {
        [var9, var10, var11] = anaglyph([var9, var10, var11]);
    }
    let mut b = TessBatch::default();
    b.vertex(
        [0.0, 100.0, 0.0],
        [0.0; 2],
        tess_color(var9, var10, var11, c[3]),
    );
    let var19 = 16i32;
    let rim = tess_color(c[0], c[1], c[2], 0.0);
    for var20 in 0..=var19 {
        let var14 = var20 as f32 * 3.1415927 * 2.0 / var19 as f32;
        let var15 = math::sin(var14);
        let var16 = math::cos(var14);
        b.vertex(
            [var15 * 120.0, var16 * 120.0, -var16 * 40.0 * c[3]],
            [0.0; 2],
            rim,
        );
    }
    let mut idx = Vec::new();
    for i in 1..=var19 as u32 {
        idx.extend_from_slice(&[0, i, i + 1]);
    }
    (b, idx)
}

/// The glow's rotation: `glRotatef(90, 1, 0, 0)` then `glRotatef(angle >
/// 0.5 ? 180 : 0, 0, 0, 1)` (the evening glow faces the other way).
// src: RenderGlobal.java:638-640
pub fn sunrise_rotation(celestial_angle: f32) -> Quat {
    let z = if celestial_angle > 0.5 { 180.0f32 } else { 0.0 };
    Quat::from_axis_angle(Vec3::X, 90.0f32.to_radians())
        * Quat::from_axis_angle(Vec3::Z, z.to_radians())
}

/// The sun, moon and stars' rotation: `glRotatef(angle · 360, 1, 0, 0)`.
// src: RenderGlobal.java:682
pub fn celestial_rotation(celestial_angle: f32) -> Quat {
    Quat::from_axis_angle(Vec3::X, (celestial_angle * 360.0).to_radians())
}

// ---- the frame ------------------------------------------------------------------

/// `renderSky`'s values for this frame.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct SkyFrame {
    /// Whether the sky is drawn (a world, not the nether, `renderDistance <
    /// 2`).
    pub visible: bool,
    /// The eye (where the sky is centred).
    pub anchor: Vec3,
    /// `getSkyColor` (anaglyph applied): the sky plane's `glColor3f`.
    pub sky: [f32; 3],
    /// The void plane's colour.
    pub void: [f32; 3],
    /// `calcSunriseSunsetColors`, when in the band.
    pub sunrise: Option<[f32; 4]>,
    /// `getCelestialAngle(partial)`.
    pub celestial_angle: f32,
    /// `1 - getRainStrength(partial)`: the sun's and moon's alpha.
    pub sun_alpha: f32,
    /// `getStarBrightness(partial) · (1 - rain)`; the stars are drawn when
    /// positive.
    pub stars: f32,
    /// `gameSettings.anaglyph` (the glow's centre colour takes the
    /// transform).
    pub anaglyph: bool,
}

/// `renderSky(partial)`'s values (RenderGlobal.java:600-722): the sky
/// colour (anaglyph applied), the glow, the celestial angle, the sun's alpha
/// and the stars' brightness, and the void's tint.
// src: RenderGlobal.java:600-722
pub fn compute_sky(
    world: &mut World,
    view: &ViewInfo,
    partial: f32,
    anaglyph_on: bool,
) -> SkyFrame {
    let v = world.get_sky_color(view.pos.x, view.pos.z, partial);
    let mut sky = [v.x_coord as f32, v.y_coord as f32, v.z_coord as f32];
    if anaglyph_on {
        sky = anaglyph(sky);
    }
    let angle = world.get_celestial_angle(partial);
    let sunrise = world
        .world_provider
        .calc_sunrise_sunset_colors(angle, partial);
    let var7 = 1.0 - world.get_rain_strength(partial);
    let var12 = world.get_star_brightness(partial) * var7;
    let [var3, var4, var5] = sky;
    // src: RenderGlobal.java:711-715
    let void = if world.world_provider.func_28112_c() {
        [var3 * 0.2 + 0.04, var4 * 0.2 + 0.04, var5 * 0.6 + 0.1]
    } else {
        sky
    };
    SkyFrame {
        visible: true,
        anchor: Vec3::ZERO,
        sky,
        void,
        sunrise,
        celestial_angle: angle,
        sun_alpha: var7,
        stars: var12,
        anaglyph: anaglyph_on,
    }
}

/// Reads the world for every WP-3.4 part (module docs, "The frame").
#[allow(clippy::too_many_arguments)]
pub fn compute_render_frame(
    mut game: NonSendMut<Game>,
    camera: Res<CameraState>,
    clouds_state: Res<RenderGlobalClouds>,
    mut rng: ResMut<WeatherRandom>,
    mut fog: ResMut<FogState>,
    mut sky: ResMut<SkyFrame>,
    mut cloud: ResMut<CloudFrame>,
    mut weather_frame: ResMut<WeatherFrame>,
    mut sel: ResMut<SelectionFrame>,
) {
    let g = &mut *game;
    let setup = camera.setup;
    let view = gather_view(g);
    let (Some(setup), Some(view), true) = (setup, view, g.the_world.is_some()) else {
        fog.active = false;
        sky.visible = false;
        cloud.visible = false;
        weather_frame.visible = false;
        sel.r#box = None;
        return;
    };
    let partial = g.timer.render_partial_ticks;
    let s = &g.game_settings;
    let (rd, fancy, anaglyph_on) = (s.render_distance, s.fancy_graphics, s.anaglyph);
    let cam = &g.entity_renderer;
    let (cloud_fog, fog2, fog1, far, ruc) = (
        cam.cloud_fog,
        cam.fog_color2,
        cam.fog_color1,
        cam.far_plane_distance,
        cam.renderer_update_count,
    );
    let mouse_over = g.object_mouse_over;
    let world = g.the_world.as_mut().expect("checked");
    let anchor = setup.eye.as_vec3();
    let nether = world.world_provider.is_nether;

    // updateFogColor(partial)
    let skyc = world.get_sky_color(view.pos.x, view.pos.z, partial);
    let fogc = world.get_fog_color(partial);
    let cloud_fog_colour = cloud_fog.then(|| {
        let c = world.draw_clouds(partial);
        [c.x_coord, c.y_coord, c.z_coord]
    });
    let color = fog::update_fog_color(&FogColorInputs {
        render_distance: rd,
        sky: [skyc.x_coord, skyc.y_coord, skyc.z_coord],
        fog: [fogc.x_coord, fogc.y_coord, fogc.z_coord],
        rain: world.get_rain_strength(partial),
        thunder: world.get_weighted_thunder_strength(partial),
        cloud_fog: cloud_fog_colour,
        in_water: view.in_water,
        in_lava: view.in_lava,
        fog_color2: fog2,
        fog_color1: fog1,
        partial,
        anaglyph: anaglyph_on,
    });
    *fog = FogState {
        active: true,
        color,
        world: fog::setup_fog(0, cloud_fog, view.in_water, view.in_lava, far, nether),
        sky: fog::setup_fog(-1, cloud_fog, view.in_water, view.in_lava, far, nether),
        far_plane_distance: far,
    };

    // renderSky, only when renderDistance < 2 (EntityRenderer.java:471-474),
    // and not in the nether (RenderGlobal.java:601).
    // src: EntityRenderer.java:471-474
    if rd < 2 && !nether {
        *sky = compute_sky(world, &view, partial, anaglyph_on);
        sky.anchor = anchor;
    } else {
        sky.visible = false;
    }

    // The selection box: either drawSelectionBox call site (in water before
    // the translucent pass, else after it); cameraZoom is always 1.
    // src: EntityRenderer.java:507-513, :549-555
    *sel = selection::compute(world, &view, mouse_over.as_ref(), partial);
    sel.anchor = anchor;

    // renderRainSnow(partial)
    // src: EntityRenderer.java:557
    *weather_frame = weather::compute(world, &view, fancy, ruc, partial, &mut rng.0);
    weather_frame.anchor = anchor;

    // renderClouds(partial)
    // src: EntityRenderer.java:564
    *cloud = clouds::compute(
        world,
        &view,
        fancy,
        anaglyph_on,
        clouds_state.cloud_offset_x,
        partial,
    );
    cloud.anchor = anchor;
}

/// One sky entity.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkyPart {
    /// `glSkyList`.
    Plane,
    /// The sunrise/sunset glow.
    Sunrise,
    /// The sun.
    Sun,
    /// The moon.
    Moon,
    /// The stars.
    Stars,
    /// `glSkyList2`.
    Void,
}

/// Handles of the sky's meshes and materials.
#[derive(Resource, Clone, Debug)]
pub struct SkyHandles {
    /// The glow's mesh (rebuilt every frame).
    pub sunrise_mesh: Handle<Mesh>,
    /// Per part, its material.
    pub materials: Vec<(SkyPart, Handle<GlMaterial>)>,
}

/// Spawns the sky's entities (hidden until a frame shows them). The sun and
/// moon textures are local handles (they could move to WP-3.2's texture
/// cache).
pub fn spawn_sky(
    mut commands: Commands,
    jar: Res<JarAssets>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GlMaterial>>,
) {
    let tex = |images: &mut Assets<Image>, name: &str| match jar_texture(&jar, name, false) {
        Ok(i) => Some(images.add(i)),
        Err(e) => {
            error!("{e}");
            None
        }
    };
    let sun_tex = tex(&mut images, SUN_PNG);
    let moon_tex = tex(&mut images, MOON_PNG);
    let sky_key = |blend| GlKey {
        blend,
        depth_write: false,
        color_write: true,
        far: true,
        wide_lines: false,
    };
    let sunrise_mesh = meshes.add(Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    ));
    {
        let (b, idx) = sunrise_batch([0.0; 4], false);
        if let Some(mut m) = meshes.get_mut(&sunrise_mesh) {
            b.write_into(&mut m, idx);
        }
    }
    let with_tex = |m: GlMaterial, t: &Option<Handle<Image>>| match t {
        Some(t) => m.with_texture(t.clone()),
        None => m,
    };
    let parts: Vec<(SkyPart, Handle<Mesh>, GlMaterial)> = vec![
        (
            SkyPart::Plane,
            meshes.add(sky_plane_batch(false).quads_mesh()),
            GlMaterial::new(sky_key(GlBlend::Off), order::SKY_PLANE),
        ),
        (
            SkyPart::Sunrise,
            sunrise_mesh.clone(),
            GlMaterial::new(sky_key(GlBlend::Alpha), order::SUNRISE),
        ),
        (
            SkyPart::Sun,
            meshes.add(sun_moon_batch(false).quads_mesh()),
            with_tex(
                GlMaterial::new(sky_key(GlBlend::Additive), order::SUN),
                &sun_tex,
            ),
        ),
        (
            SkyPart::Moon,
            meshes.add(sun_moon_batch(true).quads_mesh()),
            with_tex(
                GlMaterial::new(sky_key(GlBlend::Additive), order::MOON),
                &moon_tex,
            ),
        ),
        (
            SkyPart::Stars,
            meshes.add(star_batch().quads_mesh()),
            GlMaterial::new(sky_key(GlBlend::Additive), order::STARS),
        ),
        (
            SkyPart::Void,
            meshes.add(sky_plane_batch(true).quads_mesh()),
            GlMaterial::new(sky_key(GlBlend::Off), order::VOID),
        ),
    ];
    let mut handles = Vec::new();
    for (part, mesh, mat) in parts {
        let mh = materials.add(mat);
        handles.push((part, mh.clone()));
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(mh),
            Transform::IDENTITY,
            Visibility::Hidden,
            NoFrustumCulling,
            part,
        ));
    }
    commands.insert_resource(SkyHandles {
        sunrise_mesh,
        materials: handles,
    });
}

/// Applies [`SkyFrame`] to the sky's entities and materials.
pub fn apply_sky(
    frame: Res<SkyFrame>,
    fog: Res<FogState>,
    handles: Option<Res<SkyHandles>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GlMaterial>>,
    mut parts: Query<(&SkyPart, &mut Transform, &mut Visibility)>,
) {
    let Some(h) = handles else {
        return;
    };
    for (part, mut t, mut vis) in &mut parts {
        let shown = frame.visible
            && match part {
                SkyPart::Sunrise => frame.sunrise.is_some(),
                SkyPart::Stars => frame.stars > 0.0,
                _ => true,
            };
        let want = if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
        if !shown {
            continue;
        }
        t.translation = frame.anchor;
        t.rotation = match part {
            SkyPart::Sunrise => sunrise_rotation(frame.celestial_angle),
            SkyPart::Sun | SkyPart::Moon | SkyPart::Stars => {
                celestial_rotation(frame.celestial_angle)
            }
            _ => Quat::IDENTITY,
        };
    }
    if !frame.visible {
        return;
    }
    if let Some(c) = frame.sunrise
        && let Some(mut m) = meshes.get_mut(&h.sunrise_mesh)
    {
        let (b, idx) = sunrise_batch(c, frame.anaglyph);
        b.write_into(&mut m, idx);
    }
    let fog_color = Vec4::new(fog.color[0], fog.color[1], fog.color[2], 1.0);
    for (part, mh) in &h.materials {
        let Some(mut m) = materials.get_mut(mh) else {
            continue;
        };
        let p = &mut m.params;
        p.fog_color = fog_color;
        match part {
            // glEnable(GL_FOG) with setupFog(-1)'s values.
            // src: RenderGlobal.java:618-623
            SkyPart::Plane => {
                p.color = Vec4::new(frame.sky[0], frame.sky[1], frame.sky[2], 1.0);
                p.fog = fog.sky.to_vec4();
            }
            // src: RenderGlobal.java:709-718
            SkyPart::Void => {
                p.color = Vec4::new(frame.void[0], frame.void[1], frame.void[2], 1.0);
                p.fog = fog.sky.to_vec4();
            }
            SkyPart::Sunrise => {
                p.color = Vec4::ONE;
                p.fog = Vec4::ZERO;
            }
            // glColor4f(1, 1, 1, 1 - rain), fog off.
            // src: RenderGlobal.java:675-679
            SkyPart::Sun | SkyPart::Moon => {
                p.color = Vec4::new(1.0, 1.0, 1.0, frame.sun_alpha);
                p.fog = Vec4::ZERO;
            }
            // glColor4f(b, b, b, b).
            // src: RenderGlobal.java:700-704
            SkyPart::Stars => {
                p.color = Vec4::splat(frame.stars);
                p.fog = Vec4::ZERO;
            }
        }
    }
}

/// Registers WP-3.4: the material, the resources, the hook, the systems.
pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(crate::gl_material::GlMaterialPlugin)
            .init_resource::<FogState>()
            .init_resource::<SkyFrame>()
            .init_resource::<CloudFrame>()
            .init_resource::<WeatherFrame>()
            .init_resource::<WeatherRandom>()
            .init_resource::<SelectionFrame>()
            .init_resource::<RenderGlobalClouds>()
            .add_systems(
                Startup,
                (
                    spawn_sky,
                    clouds::spawn_clouds,
                    weather::spawn_weather,
                    selection::spawn_selection,
                ),
            )
            .add_systems(RunTickHook::UpdateClouds, clouds::update_clouds)
            .add_systems(
                Update,
                (
                    compute_render_frame,
                    (
                        fog::apply_fog,
                        apply_sky,
                        clouds::apply_clouds,
                        weather::apply_weather,
                        selection::apply_selection,
                    ),
                )
                    .chain()
                    .after(GameFrameSet),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The first three star quads and the count, from the Java loop run on
    /// the oracle JVM (intrinsics off), `Double.toString`.
    #[test]
    fn stars_match_the_java_loop() {
        let q = star_quads();
        assert_eq!(q.len(), 780);
        assert_eq!(
            q[0],
            [
                [-53.544398699078286, 69.795714043366, 47.557135428659606],
                [-53.209925774125004, 70.14301575326797, 47.42137283122404],
                [-52.94933451454261, 70.05576598216113, 47.84018277733463],
                [-53.283807439495895, 69.70846427225916, 47.9759453747702],
            ]
        );
        assert_eq!(
            q[1][0],
            [-97.02699410207538, -18.360653829219196, 15.784285795741413]
        );
        assert_eq!(
            q[2][3],
            [-98.74391377982806, 9.577010029704908, 12.577451410892985]
        );
    }

    #[test]
    fn sky_planes_cover_the_grid() {
        let p = sky_plane_batch(false);
        assert_eq!(p.len(), 13 * 13 * 4);
        assert_eq!(p.positions[0], [-384.0, 16.0, -384.0]);
        assert_eq!(p.positions[2], [-320.0, 16.0, -320.0]);
        assert_eq!(p.positions.last(), Some(&[384.0, 16.0, 448.0]));
        let v = sky_plane_batch(true);
        assert_eq!(v.positions[0], [-320.0, -16.0, -384.0]);
        assert_eq!(v.positions[1], [-384.0, -16.0, -384.0]);
    }

    #[test]
    fn sunrise_fan() {
        let (b, idx) = sunrise_batch([0.85, 0.375, 0.2, 1.0], false);
        assert_eq!(b.len(), 18);
        assert_eq!(idx.len(), 16 * 3);
        assert_eq!(b.positions[0], [0.0, 100.0, 0.0]);
        // First rim point: sin 0 = 0, cos 0 = 1 → (0, 120, -40·a).
        assert_eq!(b.positions[1], [0.0, 120.0, -40.0]);
        assert_eq!(b.colors[0], tess_color(0.85, 0.375, 0.2, 1.0));
        assert_eq!(b.colors[1][3], 0.0);
        // The rotation lays the fan's +y on the horizon toward the sun: +z
        // at sunset (angle 0.25, where the sun is), -z at sunrise (0.75).
        let r = sunrise_rotation(0.25) * Vec3::Y;
        assert!((r - Vec3::Z).length() < 1e-6, "{r:?}");
        assert!((celestial_rotation(0.25) * Vec3::Y - Vec3::Z).length() < 1e-5);
        let r = sunrise_rotation(0.75) * Vec3::Y;
        assert!((r + Vec3::Z).length() < 1e-6, "{r:?}");
        assert!((celestial_rotation(0.75) * Vec3::Y + Vec3::Z).length() < 1e-5);
        let r = sunrise_rotation(0.75) * Vec3::X;
        assert!((r + Vec3::X).length() < 1e-6, "{r:?}");
    }

    #[test]
    fn sun_is_overhead_at_noon_and_below_at_midnight() {
        // Celestial angle 0 (noon): the sun's quad at +y stays up.
        assert!((celestial_rotation(0.0) * Vec3::new(0.0, 100.0, 0.0)).y > 99.9);
        // 0.5 (midnight): it is below, the moon (-y) above.
        assert!((celestial_rotation(0.5) * Vec3::new(0.0, 100.0, 0.0)).y < -99.9);
        assert!((celestial_rotation(0.5) * Vec3::new(0.0, -100.0, 0.0)).y > 99.9);
        // 0.25 (sunset): glRotatef(90, 1, 0, 0) takes +y to +z, so the sun
        // travels from -z to +z over the day (Beta's sun sets in the
        // south, +z, by the game's own compass).
        let r = celestial_rotation(0.25) * Vec3::Y;
        assert!((r - Vec3::Z).length() < 1e-5, "{r:?}");
    }

    fn world_at(time: i64, rain: f32) -> World {
        use mc_sim::save::NoSave;
        use mc_sim::world::WorldEnvironment;
        use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
        use mc_sim::world::provider::{Dimension, WorldProvider};
        let mut w = World::new(
            Box::new(NoSave),
            "smoke",
            WorldProvider::new(Dimension::Surface),
            123,
            ChunkGenerator::Empty(EmptyChunkProvider),
            WorldEnvironment {
                rand_seed: 0,
                field_9437_g: 0,
                lock_timestamp: 0,
            },
        );
        w.set_world_time(time);
        w.set_rain_strength(rain);
        w
    }

    fn view() -> ViewInfo {
        let p = DVec3::new(0.5, 70.0, 0.5);
        ViewInfo {
            pos: p,
            prev_pos: p,
            last_tick_pos: p,
            in_water: false,
            in_lava: false,
            is_player: true,
        }
    }

    #[test]
    fn sky_frame_through_the_day() {
        // Noon (time 6000, angle 0): getSkyColor at (0.5, 0.5) on seed 123
        // (the oracle's value), no glow, no stars, full sun.
        let f = compute_sky(&mut world_at(6000, 0.0), &view(), 0.0, false);
        assert_eq!(f.celestial_angle, 0.0);
        assert_eq!(
            f.sky,
            [
                0.4313725531101227f64 as f32,
                0.6980392336845398f64 as f32,
                1.0
            ]
        );
        assert_eq!(f.sunrise, None);
        assert_eq!((f.sun_alpha, f.stars), (1.0, 0.0));
        let [r, g, b] = f.sky;
        assert_eq!(f.void, [r * 0.2 + 0.04, g * 0.2 + 0.04, b * 0.6 + 0.1]);
        // Midnight (18000, angle 0.5): black sky, stars at 0.5, the void
        // keeps its constant part.
        let f = compute_sky(&mut world_at(18000, 0.0), &view(), 0.0, false);
        assert_eq!(f.sky, [0.0; 3]);
        assert_eq!(f.void, [0.04, 0.04, 0.1]);
        assert_eq!(f.stars, 0.5);
        // Rain dims the stars and the sun: (1 - rain).
        let f = compute_sky(&mut world_at(18000, 0.5), &view(), 0.0, false);
        assert_eq!((f.sun_alpha, f.stars), (0.5, 0.25));
        // Sunset (12000): in the glow's band.
        let f = compute_sky(&mut world_at(12000, 0.0), &view(), 0.0, false);
        assert!(f.sunrise.is_some());
    }
}
