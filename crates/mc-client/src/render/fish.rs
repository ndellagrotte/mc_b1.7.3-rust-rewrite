//! `RenderFish` (RenderFish.java): the bobber, tile (1, 2) of
//! `/particles.png` facing the camera, and the line from the angler's hand:
//! a black `GL_LINE_STRIP` of 16 segments sagging as `(t² + t) / 2` of the
//! height difference. The hand is a point turned with the angler's view
//! and swing in first person, or beside the body in third person.

use mc_core::Vec3D;
use mc_core::math::{cos, sin, sqrt_float};
use mc_sim::entity::{Entity, EntityKindData as K, living as eliving};

use super::Ctx;
use super::gl::glx;
use crate::gui::batch::gl;

/// The line's vertex `var47` of 16, relative to the bobber's draw position
/// `(var2, var4, var6)` and the hand offset `(var40, var42, var44)`.
// src: RenderFish.java:66-71
pub fn line_vertex(var2: f64, var4: f64, var6: f64, d: [f64; 3], var47: i32) -> [f64; 3] {
    let var46 = 16;
    let var48 = var47 as f32 / var46 as f32;
    [
        var2 + d[0] * var48 as f64,
        var4 + d[1] * (var48 * var48 + var48) as f64 * 0.5 + 0.25,
        var6 + d[2] * var48 as f64,
    ]
}

/// `func_4011_a(fish, x, y, z, yaw, partial)`.
// src: RenderFish.java:6-78
pub fn do_render(cx: &mut Ctx, var1: &Entity, var2: f64, var4: f64, var6: f64, var9: f32) {
    let K::Fish(f) = &var1.kind else {
        return;
    };
    let (vy, vx) = (cx.info.player_view_y, cx.info.player_view_x);
    {
        let g = &mut *cx.gl;
        g.push_matrix();
        g.translatef(var2 as f32, var4 as f32, var6 as f32);
        g.enable(gl::RESCALE_NORMAL);
        g.scalef(0.5, 0.5, 0.5);
        let var10 = 1;
        let var11 = 2;
        g.bind_texture("/particles.png");
        let var13 = (var10 * 8) as f32 / 128.0;
        let var14 = (var10 * 8 + 8) as f32 / 128.0;
        let var15 = (var11 * 8) as f32 / 128.0;
        let var16 = (var11 * 8 + 8) as f32 / 128.0;
        let var17 = 1.0f32;
        let var18 = 0.5f32;
        let var19 = 0.5f32;
        g.rotatef(180.0 - vy, 0.0, 1.0, 0.0);
        g.rotatef(-vx, 1.0, 0.0, 0.0);
        g.start_drawing_quads();
        g.set_normal(0.0, 1.0, 0.0);
        g.add_vertex_with_uv(
            (0.0 - var18) as f64,
            (0.0 - var19) as f64,
            0.0,
            var13 as f64,
            var16 as f64,
        );
        g.add_vertex_with_uv(
            (var17 - var18) as f64,
            (0.0 - var19) as f64,
            0.0,
            var14 as f64,
            var16 as f64,
        );
        g.add_vertex_with_uv(
            (var17 - var18) as f64,
            (1.0 - var19) as f64,
            0.0,
            var14 as f64,
            var15 as f64,
        );
        g.add_vertex_with_uv(
            (0.0 - var18) as f64,
            (1.0 - var19) as f64,
            0.0,
            var13 as f64,
            var15 as f64,
        );
        g.draw();
        g.disable(gl::RESCALE_NORMAL);
        g.pop_matrix();
    }
    let Some(angler_id) = f.angler else {
        return;
    };
    let Some(a) = cx.world.entity(angler_id) else {
        return;
    };
    let ab = &a.base;
    let mut var20 = (ab.prev_rotation_yaw + (ab.rotation_yaw - ab.prev_rotation_yaw) * var9)
        * 3.1415927
        / 180.0;
    // var21 and var23 are dead stores in first person, as in the Java.
    let _var21 = sin(var20) as f64;
    let _var23 = cos(var20) as f64;
    let var25 = if a.kind.living().is_some() {
        eliving::get_swing_progress(a, var9)
    } else {
        0.0
    };
    let var26 = sin(sqrt_float(var25) * 3.1415927);
    let mut var27 = Vec3D::create_vector(-0.5, 0.03, 0.8);
    var27.rotate_around_x(
        -(ab.prev_rotation_pitch + (ab.rotation_pitch - ab.prev_rotation_pitch) * var9) * 3.1415927
            / 180.0,
    );
    var27.rotate_around_y(
        -(ab.prev_rotation_yaw + (ab.rotation_yaw - ab.prev_rotation_yaw) * var9) * 3.1415927
            / 180.0,
    );
    var27.rotate_around_y(var26 * 0.5);
    var27.rotate_around_x(-var26 * 0.7);
    let p = var9 as f64;
    let mut var28 = ab.prev_pos_x + (ab.pos_x - ab.prev_pos_x) * p + var27.x_coord;
    let mut var30 = ab.prev_pos_y + (ab.pos_y - ab.prev_pos_y) * p + var27.y_coord;
    let mut var32 = ab.prev_pos_z + (ab.pos_z - ab.prev_pos_z) * p + var27.z_coord;
    if cx.info.third_person_view {
        let l = a.kind.living();
        let (pryo, ryo) = l.map_or((0.0, 0.0), |l| {
            (l.prev_render_yaw_offset, l.render_yaw_offset)
        });
        var20 = (pryo + (ryo - pryo) * var9) * 3.1415927 / 180.0;
        let var21 = sin(var20) as f64;
        let var23 = cos(var20) as f64;
        var28 = ab.prev_pos_x + (ab.pos_x - ab.prev_pos_x) * p - var23 * 0.35 - var21 * 0.85;
        var30 = ab.prev_pos_y + (ab.pos_y - ab.prev_pos_y) * p - 0.45;
        var32 = ab.prev_pos_z + (ab.pos_z - ab.prev_pos_z) * p - var21 * 0.35 + var23 * 0.85;
    }
    let b = &var1.base;
    let var34 = b.prev_pos_x + (b.pos_x - b.prev_pos_x) * p;
    let var36 = b.prev_pos_y + (b.pos_y - b.prev_pos_y) * p + 0.25;
    let var38 = b.prev_pos_z + (b.pos_z - b.prev_pos_z) * p;
    let var40 = (var28 - var34) as f32 as f64;
    let var42 = (var30 - var36) as f32 as f64;
    let var44 = (var32 - var38) as f32 as f64;
    let g = &mut *cx.gl;
    g.disable(gl::TEXTURE_2D);
    g.disable(gl::LIGHTING);
    g.start_drawing(glx::LINE_STRIP);
    g.set_color_opaque_i(0);
    let var46 = 16;
    for var47 in 0..=var46 {
        let [x, y, z] = line_vertex(var2, var4, var6, [var40, var42, var44], var47);
        g.add_vertex(x, y, z);
    }
    g.draw();
    g.enable(gl::LIGHTING);
    g.enable(gl::TEXTURE_2D);
}
