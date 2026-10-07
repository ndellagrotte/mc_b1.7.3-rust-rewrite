//! `RenderArrow` (RenderArrow.java): two crossed back plates and four fins
//! of `/item/arrows.png` (row 0 of the arrow texture: `var11 = 0`, the only
//! row b1.7.3 uses), scaled 0.05625, along the arrow's interpolated yaw and
//! pitch, wobbling with `arrowShake` after it sticks. Nothing is drawn
//! before the arrow has a previous rotation (its first tick).
//!
//! The normals are `glNormal3f(±0.05625F, …)` under `glScalef(0.05625F)`
//! with `GL_RESCALE_NORMAL`: GL keeps their length, so the arrow is lit at
//! 5.6 % of the diffuse light ([`super::gl`] module docs) until a slime
//! latches `GL_NORMALIZE`.

use mc_core::math::sin;
use mc_sim::entity::{Entity, EntityKindData as K};

use super::Ctx;
use crate::gui::batch::gl;

/// The texture rows of arrow type `var11`: `(shaft v0, v1, back v0, v1)`
/// over a 32-pixel-tall texture, ten pixels per type.
// src: RenderArrow.java:14-22
pub fn arrow_rows(var11: i32) -> (f32, f32, f32, f32) {
    (
        (var11 * 10) as f32 / 32.0,
        (5 + var11 * 10) as f32 / 32.0,
        (5 + var11 * 10) as f32 / 32.0,
        (10 + var11 * 10) as f32 / 32.0,
    )
}

/// The shake angle at `arrowShake - partial`, if shaking.
// src: RenderArrow.java:25-29
pub fn shake_angle(arrow_shake: i32, var9: f32) -> Option<f32> {
    let var21 = arrow_shake as f32 - var9;
    (var21 > 0.0).then(|| -sin(var21 * 3.0) * var21)
}

/// `renderArrow(arrow, x, y, z, yaw, partial)`.
// src: RenderArrow.java:6-63
pub fn render_arrow(cx: &mut Ctx, var1: &Entity, var2: f64, var4: f64, var6: f64, var9: f32) {
    let K::Arrow(a) = &var1.kind else {
        return;
    };
    let b = &var1.base;
    if b.prev_rotation_yaw == 0.0 && b.prev_rotation_pitch == 0.0 {
        return;
    }
    let g = &mut *cx.gl;
    g.bind_texture("/item/arrows.png");
    g.push_matrix();
    g.translatef(var2 as f32, var4 as f32, var6 as f32);
    g.rotatef(
        b.prev_rotation_yaw + (b.rotation_yaw - b.prev_rotation_yaw) * var9 - 90.0,
        0.0,
        1.0,
        0.0,
    );
    g.rotatef(
        b.prev_rotation_pitch + (b.rotation_pitch - b.prev_rotation_pitch) * var9,
        0.0,
        0.0,
        1.0,
    );
    let var11 = 0;
    let var12 = 0.0f32;
    let var13 = 0.5f32;
    let (var14, var15, var18, var19) = arrow_rows(var11);
    let var16 = 0.0f32;
    let var17 = 0.15625f32;
    let var20 = 0.05625f32;
    g.enable(gl::RESCALE_NORMAL);
    if let Some(var22) = shake_angle(a.arrow_shake, var9) {
        g.rotatef(var22, 0.0, 0.0, 1.0);
    }
    g.rotatef(45.0, 1.0, 0.0, 0.0);
    g.scalef(var20, var20, var20);
    g.translatef(-4.0, 0.0, 0.0);
    g.normal3f(var20, 0.0, 0.0);
    g.start_drawing_quads();
    g.add_vertex_with_uv(-7.0, -2.0, -2.0, var16 as f64, var18 as f64);
    g.add_vertex_with_uv(-7.0, -2.0, 2.0, var17 as f64, var18 as f64);
    g.add_vertex_with_uv(-7.0, 2.0, 2.0, var17 as f64, var19 as f64);
    g.add_vertex_with_uv(-7.0, 2.0, -2.0, var16 as f64, var19 as f64);
    g.draw();
    g.normal3f(-var20, 0.0, 0.0);
    g.start_drawing_quads();
    g.add_vertex_with_uv(-7.0, 2.0, -2.0, var16 as f64, var18 as f64);
    g.add_vertex_with_uv(-7.0, 2.0, 2.0, var17 as f64, var18 as f64);
    g.add_vertex_with_uv(-7.0, -2.0, 2.0, var17 as f64, var19 as f64);
    g.add_vertex_with_uv(-7.0, -2.0, -2.0, var16 as f64, var19 as f64);
    g.draw();
    for _ in 0..4 {
        g.rotatef(90.0, 1.0, 0.0, 0.0);
        g.normal3f(0.0, 0.0, var20);
        g.start_drawing_quads();
        g.add_vertex_with_uv(-8.0, -2.0, 0.0, var12 as f64, var14 as f64);
        g.add_vertex_with_uv(8.0, -2.0, 0.0, var13 as f64, var14 as f64);
        g.add_vertex_with_uv(8.0, 2.0, 0.0, var13 as f64, var15 as f64);
        g.add_vertex_with_uv(-8.0, 2.0, 0.0, var12 as f64, var15 as f64);
        g.draw();
    }
    g.disable(gl::RESCALE_NORMAL);
    g.pop_matrix();
}
