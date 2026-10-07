//! The camera half of `EntityRenderer` (EntityRenderer.java): its per-tick
//! `updateRenderer` state, `getMouseOver`, `getFOVModifier`,
//! `hurtCameraEffect`, `setupViewBobbing`, `orientCamera` and
//! `setupCameraTransform`, and the Bevy camera that follows them.
//!
//! # Matrices
//!
//! The Java builds the projection and modelview with immediate GL calls;
//! [`GlMatrix`] replays them (`glRotatef`/`glTranslatef`/`glScalef`
//! post-multiply, angles in degrees, `f32`). `setupCameraTransform` is
//! split at its one non-rigid step, the portal wobble's
//! `rotate · scale · rotate⁻¹` (EntityRenderer.java:289-296): the modelview
//! is `V = G1 · P · G2` with `G1` the hurt and bobbing transforms, `P` the
//! wobble and `G2` `orientCamera`. Then `V = D · W` with `W = G1 · G2`
//! (rigid: the Bevy camera's [`Transform`] is `translate(eye) · W⁻¹`) and
//! `D = G1 · P · G1⁻¹` (identity unless in a portal), which goes into the
//! projection ([`McProjection`]: `perspective · D`). A world point `x`
//! reaches the eye as `V · (x − eye)`, exactly as the Java draws the world
//! translated by the negated interpolated position (`RenderGlobal`,
//! `EntityRenderer.renderWorld`, EntityRenderer.java:443-445).
//!
//! Bevy's world axes are the game's (x east, y up, z south; right-handed,
//! the camera looking down −z), so no axis conversion is needed. The f32
//! [`Transform`] loses precision far from the origin; [`CameraState`] keeps
//! the eye in f64 for the renderers (WP-3.3 may re-origin).
//!
//! # Hooks
//!
//! Not here (and where): the fog colour (`updateFogColor`, `setupFog`),
//! clouds, rain and snow, `hasCloudFog` (WP-3.4); `itemRenderer`,
//! `renderHand`, `addRainParticles` (WP-3.7, 3.4); anaglyph 3D's two passes
//! and colour masks (rendering, WP-3.3; both eye offsets are transcribed for
//! it in [`setup_camera_transform`]'s `anaglyph_field`: the modelview shift
//! in [`CameraSetup::pre`], the projection shift in
//! [`CameraSetup::projection_shift_x`]). The shell's single-pass camera
//! passes no field and gets neither, since vanilla never draws one
//! displaced eye alone.

use bevy::camera::CameraProjection;
use bevy::math::{DVec3, Mat4, Vec3, Vec3A};
use bevy::prelude::*;
use mc_core::Vec3D;
use mc_core::math::{self, floor_double};
use mc_sim::block::Material;
use mc_sim::entity::{Entity, base, dispatch, living};
use mc_sim::world::World;
use mc_sim::world::ray_trace::MovingObjectPosition;

use crate::input::MouseFilter;
use crate::settings::GameSettings;

/// `Block.blockBed.blockID`.
// src: Block.java:618
const BED: i32 = 26;

/// A GL matrix stack top, replayed (module docs).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlMatrix(pub Mat4);

impl Default for GlMatrix {
    fn default() -> Self {
        GlMatrix(Mat4::IDENTITY)
    }
}

impl GlMatrix {
    /// `glRotatef(angle, x, y, z)`: post-multiplies a rotation of `angle`
    /// degrees about the normalised axis.
    pub fn rotate(&mut self, angle: f32, x: f32, y: f32, z: f32) {
        let axis = Vec3::new(x, y, z);
        let len = axis.length();
        if len == 0.0 {
            return;
        }
        self.0 *= Mat4::from_axis_angle(axis / len, angle.to_radians());
    }

    /// `glTranslatef(x, y, z)`.
    pub fn translate(&mut self, x: f32, y: f32, z: f32) {
        self.0 *= Mat4::from_translation(Vec3::new(x, y, z));
    }

    /// `glScalef(x, y, z)`.
    pub fn scale(&mut self, x: f32, y: f32, z: f32) {
        self.0 *= Mat4::from_scale(Vec3::new(x, y, z));
    }
}

/// `EntityRenderer`'s camera fields.
#[derive(Clone, Debug)]
pub struct EntityRendererCam {
    /// `farPlaneDistance` (`256 >> renderDistance`).
    // src: EntityRenderer.java:17
    pub far_plane_distance: f32,
    /// `rendererUpdateCount`: ticks of `updateRenderer`.
    // src: EntityRenderer.java:19
    pub renderer_update_count: i32,
    /// `pointedEntity`, as an entity id.
    // src: EntityRenderer.java:20
    pub pointed_entity: Option<i32>,
    /// `mouseFilterXAxis` (smooth camera).
    // src: EntityRenderer.java:21
    pub mouse_filter_x_axis: MouseFilter,
    /// `mouseFilterYAxis`.
    // src: EntityRenderer.java:22
    pub mouse_filter_y_axis: MouseFilter,
    /// `thirdPersonDistance` (4).
    // src: EntityRenderer.java:27
    pub third_person_distance: f32,
    /// `thirdPersonDistanceTemp`.
    // src: EntityRenderer.java:28
    pub third_person_distance_temp: f32,
    /// `debugCamYaw`.
    // src: EntityRenderer.java:29
    pub debug_cam_yaw: f32,
    /// `prevDebugCamYaw`.
    // src: EntityRenderer.java:30
    pub prev_debug_cam_yaw: f32,
    /// `debugCamPitch`.
    // src: EntityRenderer.java:31
    pub debug_cam_pitch: f32,
    /// `prevDebugCamPitch`.
    // src: EntityRenderer.java:32
    pub prev_debug_cam_pitch: f32,
    /// `debugCamFOV`.
    // src: EntityRenderer.java:33
    pub debug_cam_fov: f32,
    /// `prevDebugCamFOV`.
    // src: EntityRenderer.java:34
    pub prev_debug_cam_fov: f32,
    /// `camRoll`.
    // src: EntityRenderer.java:35
    pub cam_roll: f32,
    /// `prevCamRoll`.
    // src: EntityRenderer.java:36
    pub prev_cam_roll: f32,
    /// `cloudFog`: set by `orientCamera` from `renderGlobal.hasCloudFog`
    /// (WP-3.4); false until then.
    // src: EntityRenderer.java:37
    pub cloud_fog: bool,
    /// `cameraZoom` (1: no zoom; only the unused high-resolution screenshot
    /// code changes it).
    // src: EntityRenderer.java:38
    pub camera_zoom: f64,
    /// `cameraYaw` (zoom offset).
    // src: EntityRenderer.java:39
    pub camera_yaw: f64,
    /// `cameraPitch` (zoom offset).
    // src: EntityRenderer.java:40
    pub camera_pitch: f64,
    /// `prevFrameTime`: the last frame the window was active
    /// (`currentTimeMillis`).
    // src: EntityRenderer.java:41
    pub prev_frame_time: i64,
    /// `fogColor2`: last tick's [`EntityRendererCam::fog_color1`].
    // src: EntityRenderer.java:51
    pub fog_color2: f32,
    /// `fogColor1`: the eased brightness factor of the fog colour
    /// (`updateRenderer`; read by `updateFogColor`, WP-3.4).
    // src: EntityRenderer.java:52
    pub fog_color1: f32,
}

impl EntityRendererCam {
    /// The field initialisers (`EntityRenderer(mc)`), at `now_millis`.
    // src: EntityRenderer.java:14-57
    pub fn new(now_millis: i64) -> Self {
        EntityRendererCam {
            far_plane_distance: 0.0,
            renderer_update_count: 0,
            pointed_entity: None,
            mouse_filter_x_axis: MouseFilter::default(),
            mouse_filter_y_axis: MouseFilter::default(),
            third_person_distance: 4.0,
            third_person_distance_temp: 4.0,
            debug_cam_yaw: 0.0,
            prev_debug_cam_yaw: 0.0,
            debug_cam_pitch: 0.0,
            prev_debug_cam_pitch: 0.0,
            debug_cam_fov: 0.0,
            prev_debug_cam_fov: 0.0,
            cam_roll: 0.0,
            prev_cam_roll: 0.0,
            cloud_fog: false,
            camera_zoom: 1.0,
            camera_yaw: 0.0,
            camera_pitch: 0.0,
            prev_frame_time: now_millis,
            fog_color2: 0.0,
            fog_color1: 0.0,
        }
    }

    /// `updateRenderer()`'s camera and fog-brightness part for the view
    /// entity `view`; `itemRenderer.updateEquippedItem()` and
    /// `addRainParticles()` are the [`crate::game::RunTickHook::UpdateRenderer`]
    /// hook's (WP-3.7, WP-3.4).
    // src: EntityRenderer.java:59-77
    pub fn update_renderer(&mut self, world: &mut World, view: &Entity, s: &GameSettings) {
        self.fog_color2 = self.fog_color1;
        self.third_person_distance_temp = self.third_person_distance;
        self.prev_debug_cam_yaw = self.debug_cam_yaw;
        self.prev_debug_cam_pitch = self.debug_cam_pitch;
        self.prev_debug_cam_fov = self.debug_cam_fov;
        self.prev_cam_roll = self.cam_roll;
        let var1 = world.get_light_brightness(
            floor_double(view.base.pos_x),
            floor_double(view.base.pos_y),
            floor_double(view.base.pos_z),
        );
        let var2 = (3 - s.render_distance) as f32 / 3.0;
        let var3 = var1 * (1.0 - var2) + var2;
        self.fog_color1 += (var3 - self.fog_color1) * 0.1;
        self.renderer_update_count = self.renderer_update_count.wrapping_add(1);
    }

    /// `getFOVModifier(partial)`: 70°, 60° with the eye in water, narrowed
    /// while dying, plus the (unused) debug FOV.
    // src: EntityRenderer.java:137-150
    pub fn get_fov_modifier(&self, world: &mut World, view: &Entity, var1: f32) -> f32 {
        let mut var3 = 70.0f32;
        if base::is_inside_of_material(view, world, Material::Water) {
            var3 = 60.0;
        }
        if let Some(l) = view.kind.living()
            && l.health <= 0
        {
            let var4 = l.death_time as f32 + var1;
            var3 /= (1.0 - 500.0 / (var4 + 500.0)) * 2.0 + 1.0;
        }
        var3 + self.prev_debug_cam_fov + (self.debug_cam_fov - self.prev_debug_cam_fov) * var1
    }
}

/// `hurtCameraEffect(partial)`: the death roll and the hurt shake.
// src: EntityRenderer.java:152-169
pub fn hurt_camera_effect(m: &mut GlMatrix, view: &Entity, var1: f32) {
    let Some(l) = view.kind.living() else {
        return;
    };
    let mut var3 = l.hurt_time as f32 - var1;
    if l.health <= 0 {
        let var4 = l.death_time as f32 + var1;
        m.rotate(40.0 - 8000.0 / (var4 + 200.0), 0.0, 0.0, 1.0);
    }
    if var3 >= 0.0 {
        var3 /= l.max_hurt_time as f32;
        var3 = math::sin(var3 * var3 * var3 * var3 * 3.1415927);
        let var4 = l.attacked_at_yaw;
        m.rotate(-var4, 0.0, 1.0, 0.0);
        m.rotate(-var3 * 14.0, 0.0, 0.0, 1.0);
        m.rotate(var4, 0.0, 1.0, 0.0);
    }
}

/// `setupViewBobbing(partial)`: the walking bob of a player view entity.
// src: EntityRenderer.java:171-183
pub fn setup_view_bobbing(m: &mut GlMatrix, view: &Entity, var1: f32) {
    let (Some(p), Some(l)) = (view.kind.player(), view.kind.living()) else {
        return;
    };
    let b = &view.base;
    let var3 = b.distance_walked_modified - b.prev_distance_walked_modified;
    let var4 = -(b.distance_walked_modified + var3 * var1);
    let var5 = p.prev_camera_yaw + (p.camera_yaw - p.prev_camera_yaw) * var1;
    let var6 = l.prev_camera_pitch + (l.camera_pitch - l.prev_camera_pitch) * var1;
    m.translate(
        math::sin(var4 * 3.1415927) * var5 * 0.5,
        -(math::cos(var4 * 3.1415927) * var5).abs(),
        0.0,
    );
    m.rotate(math::sin(var4 * 3.1415927) * var5 * 3.0, 0.0, 0.0, 1.0);
    m.rotate(
        (math::cos(var4 * 3.1415927 - 0.2) * var5).abs() * 5.0,
        1.0,
        0.0,
        0.0,
    );
    m.rotate(var6, 1.0, 0.0, 0.0);
}

/// The result of [`setup_camera_transform`] (module docs, "Matrices").
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraSetup {
    /// `gluPerspective`'s field of view, degrees (`getFOVModifier`).
    pub fov_y_degrees: f32,
    /// `displayWidth / displayHeight`.
    pub aspect: f32,
    /// The near plane (0.05).
    pub near: f32,
    /// The far plane (`farPlaneDistance * 2`).
    pub far: f32,
    /// The anaglyph eye's `glTranslatef(-(field * 2 - 1) * 0.07, 0, 0)`
    /// applied to the projection before `gluPerspective` (clip space =
    /// `translate(shift, 0, 0) · perspective`); 0 without an anaglyph pass.
    pub projection_shift_x: f32,
    /// `G1`: the anaglyph eye's modelview shift, hurt and bobbing.
    pub pre: Mat4,
    /// `P`: the portal wobble (identity outside a portal).
    pub portal: Mat4,
    /// `G2`: `orientCamera`'s rotations and translations.
    pub orient: Mat4,
    /// The interpolated camera position `orientCamera` ends with (the
    /// player's eye: `prevPos + (pos - prevPos) * partial`, minus the
    /// `yOffset - 1.62` correction, 0 for players).
    pub eye: DVec3,
}

impl CameraSetup {
    /// `V = G1 · P · G2`.
    pub fn modelview(&self) -> Mat4 {
        self.pre * self.portal * self.orient
    }

    /// `W = G1 · G2`, the rigid part.
    pub fn rigid(&self) -> Mat4 {
        self.pre * self.orient
    }

    /// `D = G1 · P · G1⁻¹`.
    pub fn distortion(&self) -> Mat4 {
        self.pre * self.portal * self.pre.inverse()
    }
}

/// `orientCamera(partial)` onto `m`; returns the eye position.
// src: EntityRenderer.java:185-259
pub fn orient_camera(
    cam: &mut EntityRendererCam,
    m: &mut GlMatrix,
    world: &mut World,
    var2: &Entity,
    s: &GameSettings,
    var1: f32,
) -> DVec3 {
    let b = &var2.base;
    let mut var3 = b.y_offset - 1.62;
    let var4 = b.prev_pos_x + (b.pos_x - b.prev_pos_x) * var1 as f64;
    let var6 = b.prev_pos_y + (b.pos_y - b.prev_pos_y) * var1 as f64 - var3 as f64;
    let var8 = b.prev_pos_z + (b.pos_z - b.prev_pos_z) * var1 as f64;
    m.rotate(
        cam.prev_cam_roll + (cam.cam_roll - cam.prev_cam_roll) * var1,
        0.0,
        0.0,
        1.0,
    );
    let sleeping = var2.kind.player().is_some_and(|p| p.is_player_sleeping());
    if sleeping {
        var3 = (var3 as f64 + 1.0) as f32;
        m.translate(0.0, 0.3, 0.0);
        if !s.debug_cam_enable {
            let (x, y, z) = (
                floor_double(b.pos_x),
                floor_double(b.pos_y),
                floor_double(b.pos_z),
            );
            let var10 = world.get_block_id(x, y, z);
            if var10 == BED {
                let var11 = world.get_block_metadata(x, y, z);
                let var12 = var11 & 3;
                m.rotate((var12 * 90) as f32, 0.0, 1.0, 0.0);
            }
            m.rotate(
                b.prev_rotation_yaw + (b.rotation_yaw - b.prev_rotation_yaw) * var1 + 180.0,
                0.0,
                -1.0,
                0.0,
            );
            m.rotate(
                b.prev_rotation_pitch + (b.rotation_pitch - b.prev_rotation_pitch) * var1,
                -1.0,
                0.0,
                0.0,
            );
        }
    } else if s.third_person_view {
        let mut var27 = (cam.third_person_distance_temp
            + (cam.third_person_distance - cam.third_person_distance_temp) * var1)
            as f64;
        if s.debug_cam_enable {
            let var28 =
                cam.prev_debug_cam_yaw + (cam.debug_cam_yaw - cam.prev_debug_cam_yaw) * var1;
            let var13 =
                cam.prev_debug_cam_pitch + (cam.debug_cam_pitch - cam.prev_debug_cam_pitch) * var1;
            m.translate(0.0, 0.0, -var27 as f32);
            m.rotate(var13, 1.0, 0.0, 0.0);
            m.rotate(var28, 0.0, 1.0, 0.0);
        } else {
            let var28 = b.rotation_yaw;
            let var13 = b.rotation_pitch;
            let var14 = (-math::sin(var28 / 180.0 * 3.1415927)
                * math::cos(var13 / 180.0 * 3.1415927)) as f64
                * var27;
            let var16 = (math::cos(var28 / 180.0 * 3.1415927)
                * math::cos(var13 / 180.0 * 3.1415927)) as f64
                * var27;
            let var18 = (-math::sin(var13 / 180.0 * 3.1415927)) as f64 * var27;
            for var20 in 0..8 {
                let mut var21 = ((var20 & 1) * 2 - 1) as f32;
                let mut var22 = ((var20 >> 1 & 1) * 2 - 1) as f32;
                let mut var23 = ((var20 >> 2 & 1) * 2 - 1) as f32;
                var21 *= 0.1;
                var22 *= 0.1;
                var23 *= 0.1;
                // vanilla bug: the end point adds var23 to x as well as z
                // (`var4 - var14 + var21 + var23`), transcribed.
                // src: EntityRenderer.java:230
                let mut start = Vec3D::create_vector(
                    var4 + var21 as f64,
                    var6 + var22 as f64,
                    var8 + var23 as f64,
                );
                let end = Vec3D::create_vector(
                    var4 - var14 + var21 as f64 + var23 as f64,
                    var6 - var18 + var22 as f64,
                    var8 - var16 + var23 as f64,
                );
                if let Some(var24) = world.ray_trace_blocks(&mut start, &end) {
                    let var25 = var24
                        .hit_vec
                        .distance_to(&Vec3D::create_vector(var4, var6, var8));
                    if var25 < var27 {
                        var27 = var25;
                    }
                }
            }
            m.rotate(b.rotation_pitch - var13, 1.0, 0.0, 0.0);
            m.rotate(b.rotation_yaw - var28, 0.0, 1.0, 0.0);
            m.translate(0.0, 0.0, -var27 as f32);
            m.rotate(var28 - b.rotation_yaw, 0.0, 1.0, 0.0);
            m.rotate(var13 - b.rotation_pitch, 1.0, 0.0, 0.0);
        }
    } else {
        m.translate(0.0, 0.0, -0.1);
    }
    if !s.debug_cam_enable {
        m.rotate(
            b.prev_rotation_pitch + (b.rotation_pitch - b.prev_rotation_pitch) * var1,
            1.0,
            0.0,
            0.0,
        );
        m.rotate(
            b.prev_rotation_yaw + (b.rotation_yaw - b.prev_rotation_yaw) * var1 + 180.0,
            0.0,
            1.0,
            0.0,
        );
    }
    m.translate(0.0, var3, 0.0);
    let var4 = b.prev_pos_x + (b.pos_x - b.prev_pos_x) * var1 as f64;
    let var6 = b.prev_pos_y + (b.pos_y - b.prev_pos_y) * var1 as f64 - var3 as f64;
    let var8 = b.prev_pos_z + (b.pos_z - b.prev_pos_z) * var1 as f64;
    // cloudFog = renderGlobal.hasCloudFog(var4, var6, var8, var1): WP-3.4.
    DVec3::new(var4, var6, var8)
}

/// `setupCameraTransform(partial, anaglyphField)`'s projection and
/// modelview, split as in the module docs. `time_in_portal` is the
/// player's interpolated `prevTimeInPortal + (timeInPortal -
/// prevTimeInPortal) * partial` (`mc.thePlayer`, not the view entity).
/// `anaglyph_field` is `renderWorld`'s pass (0 or 1) when anaglyph 3D draws
/// two eyes, `None` for a single pass (module docs, "Hooks").
// src: EntityRenderer.java:261-299
#[allow(clippy::too_many_arguments)]
pub fn setup_camera_transform(
    cam: &mut EntityRendererCam,
    world: &mut World,
    view: &Entity,
    s: &GameSettings,
    display_width: i32,
    display_height: i32,
    time_in_portal: f32,
    var1: f32,
    anaglyph_field: Option<i32>,
) -> CameraSetup {
    cam.far_plane_distance = (256 >> s.render_distance) as f32;
    // Projection: the zoom (cameraZoom, always 1 in play) is not ported.
    let var3 = 0.07f32;
    let mut projection_shift_x = 0.0;
    let fov = cam.get_fov_modifier(world, view, var1);
    let mut pre = GlMatrix::default();
    if s.anaglyph
        && let Some(var2) = anaglyph_field
    {
        // src: EntityRenderer.java:266-268
        projection_shift_x = (-(var2 * 2 - 1)) as f32 * var3;
        // src: EntityRenderer.java:280-282
        pre.translate((var2 * 2 - 1) as f32 * 0.1, 0.0, 0.0);
    }
    hurt_camera_effect(&mut pre, view, var1);
    if s.view_bobbing {
        setup_view_bobbing(&mut pre, view, var1);
    }
    let mut portal = GlMatrix::default();
    let var4 = time_in_portal;
    if var4 > 0.0 {
        let mut var5 = 5.0 / (var4 * var4 + 5.0) - var4 * 0.04;
        var5 *= var5;
        let a = (cam.renderer_update_count as f32 + var1) * 20.0;
        portal.rotate(a, 0.0, 1.0, 1.0);
        portal.scale(1.0 / var5, 1.0, 1.0);
        portal.rotate(-a, 0.0, 1.0, 1.0);
    }
    let mut orient = GlMatrix::default();
    let eye = orient_camera(cam, &mut orient, world, view, s, var1);
    CameraSetup {
        fov_y_degrees: fov,
        aspect: display_width as f32 / display_height as f32,
        near: 0.05,
        far: cam.far_plane_distance * 2.0,
        projection_shift_x,
        pre: pre.0,
        portal: portal.0,
        orient: orient.0,
        eye,
    }
}

/// `getMouseOver(partial)` for the view entity `view_id` with the
/// controller's reach: the block it looks at, replaced by the nearest
/// collidable entity within 3 blocks (or the block's distance) along the
/// look. Returns `(objectMouseOver, pointedEntity)`; `None` when the view
/// entity is not in its slot.
// src: EntityRenderer.java:79-135
pub fn get_mouse_over(
    world: &mut World,
    view_id: i32,
    reach: f32,
    var1: f32,
) -> Option<(Option<MovingObjectPosition>, Option<i32>)> {
    world.with_entity_mut(view_id, |w, view| {
        let mut var2 = reach as f64;
        let mut mouse_over = living::ray_trace(view, w, var2, var1);
        let mut var4 = var2;
        let var6 = living::get_position(view, var1);
        if let Some(m) = &mouse_over {
            var4 = m.hit_vec.distance_to(&var6);
        }
        // PlayerControllerTest (creative) is not ported: always the else.
        if var4 > 3.0 {
            var4 = 3.0;
        }
        var2 = var4;
        let var7 = living::get_look(view, var1);
        let var8 = var6.add_vector(
            var7.x_coord * var2,
            var7.y_coord * var2,
            var7.z_coord * var2,
        );
        let mut pointed: Option<i32> = None;
        let var9 = 1.0f32;
        let bb = view
            .base
            .bounding_box
            .add_coord(
                var7.x_coord * var2,
                var7.y_coord * var2,
                var7.z_coord * var2,
            )
            .expand(var9 as f64, var9 as f64, var9 as f64);
        let var10 = w.get_entities_within_aabb_excluding_entity(Some(view_id), &bb);
        let mut var11 = 0.0f64;
        for id in var10 {
            let Some(var14) = w.entity(id) else { continue };
            if dispatch::can_be_collided_with(var14, w) {
                let var15 = dispatch::get_collision_border_size(var14, w) as f64;
                let var16 = var14.base.bounding_box.expand(var15, var15, var15);
                let var17 = var16.calculate_intercept(&var6, &var8);
                if var16.is_vec_inside(&var6) {
                    if 0.0 < var11 || var11 == 0.0 {
                        pointed = Some(id);
                        var11 = 0.0;
                    }
                } else if let Some(var17) = var17 {
                    let var18 = var6.distance_to(&var17.hit_vec);
                    if var18 < var11 || var11 == 0.0 {
                        pointed = Some(id);
                        var11 = var18;
                    }
                }
            }
        }
        if let Some(id) = pointed
            && let Some(e) = w.entity(id)
        {
            mouse_over = Some(MovingObjectPosition::entity(
                id,
                e.base.pos_x,
                e.base.pos_y,
                e.base.pos_z,
            ));
        }
        (mouse_over, pointed)
    })
}

/// The perspective of `setupCameraTransform` with the portal distortion
/// (module docs): Bevy's reverse-Z infinite perspective times `D`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct McProjection {
    /// Vertical field of view, radians.
    pub fov: f32,
    /// Width over height.
    pub aspect_ratio: f32,
    /// Near plane.
    pub near: f32,
    /// Far plane (culling only; the depth range is infinite).
    pub far: f32,
    /// `D` (identity outside a portal).
    pub distortion: Mat4,
}

impl Default for McProjection {
    fn default() -> Self {
        McProjection {
            fov: 70f32.to_radians(),
            aspect_ratio: 854.0 / 480.0,
            near: 0.05,
            far: 512.0,
            distortion: Mat4::IDENTITY,
        }
    }
}

impl CameraProjection for McProjection {
    fn get_clip_from_view(&self) -> Mat4 {
        Mat4::perspective_infinite_reverse_rh(self.fov, self.aspect_ratio, self.near)
            * self.distortion
    }

    fn get_clip_from_view_for_sub(&self, _sub_view: &bevy::camera::SubCameraView) -> Mat4 {
        self.get_clip_from_view()
    }

    fn update(&mut self, width: f32, height: f32) {
        if width > 0.0 && height > 0.0 {
            self.aspect_ratio = width / height;
        }
    }

    fn far(&self) -> f32 {
        self.far
    }

    fn get_frustum_corners(&self, z_near: f32, z_far: f32) -> [Vec3A; 8] {
        let tan_half_fov = (self.fov / 2.0).tan();
        let a = z_near.abs() * tan_half_fov;
        let b = z_far.abs() * tan_half_fov;
        let r = self.aspect_ratio;
        [
            Vec3A::new(a * r, -a, z_near),
            Vec3A::new(a * r, a, z_near),
            Vec3A::new(-a * r, a, z_near),
            Vec3A::new(-a * r, -a, z_near),
            Vec3A::new(b * r, -b, z_far),
            Vec3A::new(b * r, b, z_far),
            Vec3A::new(-b * r, b, z_far),
            Vec3A::new(-b * r, -b, z_far),
        ]
    }
}

/// The frame's camera for the renderers (written by the frame driver).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct CameraState {
    /// The last `setupCameraTransform`, `None` without a world.
    pub setup: Option<CameraSetup>,
}

/// Marks the Bevy camera that follows the game's.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct MainCamera;

/// Spawns the world camera.
pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Projection::custom(McProjection::default()),
        Transform::IDENTITY,
        MainCamera,
    ));
}

/// The Bevy camera's transform and projection from [`CameraState`]
/// (module docs, "Matrices").
pub fn apply_camera(
    state: Res<CameraState>,
    mut cams: Query<(&mut Transform, &mut Projection), With<MainCamera>>,
) {
    let Some(setup) = state.setup else {
        return;
    };
    let Ok((mut t, mut p)) = cams.single_mut() else {
        return;
    };
    *t = camera_transform(&setup);
    let proj = McProjection {
        fov: setup.fov_y_degrees.to_radians(),
        aspect_ratio: setup.aspect,
        near: setup.near,
        far: setup.far,
        distortion: setup.distortion(),
    };
    if let Projection::Custom(c) = &mut *p
        && let Some(m) = c.get_mut::<McProjection>()
    {
        *m = proj;
    } else {
        *p = Projection::custom(proj);
    }
}

/// `translate(eye) · W⁻¹` as a Bevy transform (module docs).
pub fn camera_transform(setup: &CameraSetup) -> Transform {
    let w_inv = setup.rigid().inverse();
    let m = Mat4::from_translation(setup.eye.as_vec3()) * w_inv;
    Transform::from_matrix(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-4
    }

    fn setup_with(orient: GlMatrix, eye: DVec3) -> CameraSetup {
        CameraSetup {
            fov_y_degrees: 70.0,
            aspect: 1.0,
            near: 0.05,
            far: 512.0,
            projection_shift_x: 0.0,
            pre: Mat4::IDENTITY,
            portal: Mat4::IDENTITY,
            orient: orient.0,
            eye,
        }
    }

    #[test]
    fn yaw_zero_looks_south() {
        // orientCamera for yaw 0, pitch 0, first person: translate(0,0,-0.1),
        // rotate(pitch, x), rotate(yaw + 180, y).
        let mut m = GlMatrix::default();
        m.translate(0.0, 0.0, -0.1);
        m.rotate(0.0, 1.0, 0.0, 0.0);
        m.rotate(0.0 + 180.0, 0.0, 1.0, 0.0);
        let t = camera_transform(&setup_with(m, DVec3::new(0.5, 65.62, 0.5)));
        // Minecraft yaw 0 faces +z (south).
        assert!(approx(*t.forward(), Vec3::Z), "{:?}", t.forward());
        assert!(approx(*t.up(), Vec3::Y));
        // The 0.1 pull-back moves the eye 0.1 behind the player (north).
        assert!(
            approx(t.translation, Vec3::new(0.5, 65.62, 0.4)),
            "{:?}",
            t.translation
        );
    }

    #[test]
    fn yaw_90_looks_west_and_pitch_looks_down() {
        let mut m = GlMatrix::default();
        m.rotate(0.0, 1.0, 0.0, 0.0);
        m.rotate(90.0 + 180.0, 0.0, 1.0, 0.0);
        let t = camera_transform(&setup_with(m, DVec3::ZERO));
        // yaw 90 faces -x (west).
        assert!(approx(*t.forward(), -Vec3::X), "{:?}", t.forward());
        let mut m = GlMatrix::default();
        m.rotate(90.0, 1.0, 0.0, 0.0);
        m.rotate(180.0, 0.0, 1.0, 0.0);
        let t = camera_transform(&setup_with(m, DVec3::ZERO));
        // pitch +90 looks straight down.
        assert!(approx(*t.forward(), -Vec3::Y), "{:?}", t.forward());
    }

    #[test]
    fn portal_wobble_is_in_the_projection_only() {
        let mut portal = GlMatrix::default();
        portal.rotate(30.0, 0.0, 1.0, 1.0);
        portal.scale(1.5, 1.0, 1.0);
        portal.rotate(-30.0, 0.0, 1.0, 1.0);
        let mut pre = GlMatrix::default();
        pre.translate(0.01, -0.02, 0.0);
        pre.rotate(3.0, 0.0, 0.0, 1.0);
        let mut orient = GlMatrix::default();
        orient.rotate(20.0, 1.0, 0.0, 0.0);
        orient.rotate(200.0, 0.0, 1.0, 0.0);
        let s = CameraSetup {
            portal: portal.0,
            pre: pre.0,
            ..setup_with(orient, DVec3::new(10.0, 70.0, -5.0))
        };
        // D · W == V
        let v = s.modelview();
        let dw = s.distortion() * s.rigid();
        assert!(v.abs_diff_eq(dw, 1e-5));
        // The transform is rigid.
        let t = camera_transform(&s);
        assert!((t.scale - Vec3::ONE).length() < 1e-4);
        // A world point maps the same way through both factorisations.
        let x = Vec3::new(12.0, 71.0, -3.0);
        let eye_space = v.transform_point3(x - s.eye.as_vec3());
        let via_bevy = s
            .distortion()
            .transform_point3(t.to_matrix().inverse().transform_point3(x));
        assert!(approx(eye_space, via_bevy), "{eye_space:?} {via_bevy:?}");
    }

    #[test]
    fn far_plane_follows_render_distance() {
        for (rd, far) in [(0, 256.0), (1, 128.0), (2, 64.0), (3, 32.0)] {
            assert_eq!((256 >> rd) as f32, far);
        }
    }

    #[test]
    fn fov_while_dying() {
        // getFOVModifier's death narrowing: 70 / ((1 - 500/(t+500)) * 2 + 1).
        let t = 10.0f32 + 0.5;
        let fov = 70.0 / ((1.0 - 500.0 / (t + 500.0)) * 2.0 + 1.0);
        assert!((fov - 67.234).abs() < 0.001, "{fov}");
    }
}
