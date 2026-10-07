//! `RenderSlime` (RenderSlime.java): the inner body (`ModelSlime(16)`), then
//! the translucent outer cube (`ModelSlime(0)`, alpha-blended), squashed by
//! the jump. Pass 0 enables `GL_NORMALIZE`, which nothing disables again
//! (vanilla bug, transcribed: [`super::gl`] module docs).

use mc_sim::entity::{Entity, EntityKindData as K, slime as eslime};

use super::Ctx;
use super::gl::glx;
use super::living::RenderLiving;
use crate::gui::batch::gl;

/// `renderSlimePassModel(slime, pass, partial)`.
// src: RenderSlime.java:13-28
pub fn render_slime_pass_model(r: &mut RenderLiving, cx: &mut Ctx, var2: i32) -> bool {
    if var2 == 0 {
        r.set_render_pass_model(0);
        cx.gl.enable(glx::NORMALIZE);
        cx.gl.enable(gl::BLEND);
        cx.gl.blend_func(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        true
    } else {
        if var2 == 1 {
            cx.gl.disable(gl::BLEND);
            cx.gl.color4f(1.0, 1.0, 1.0, 1.0);
        }
        false
    }
}

/// The slime's scale `(xz, y)` for size `var3` and the interpolated squish.
// src: RenderSlime.java:30-36
pub fn slime_scale(var3: i32, squish: f32) -> (f32, f32) {
    let var4 = squish / (var3 as f32 * 0.5 + 1.0);
    let var5 = 1.0 / (var4 + 1.0);
    let var6 = var3 as f32;
    (var5 * var6, 1.0 / var5 * var6)
}

/// `scaleSlime(slime, partial)`.
// src: RenderSlime.java:30-36
pub fn scale_slime(cx: &mut Ctx, var1: &Entity, var2: f32) {
    let K::Slime(_, s) = &var1.kind else {
        return;
    };
    let var3 = eslime::get_slime_size(var1);
    let squish = s.squish_factor + (s.squish_amount - s.squish_factor) * var2;
    let (xz, y) = slime_scale(var3, squish);
    cx.gl.scalef(xz, y, xz);
}
