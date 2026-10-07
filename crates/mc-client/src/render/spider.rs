//! `RenderSpider` (RenderSpider.java): a second `ModelSpider` drawn with
//! `/mob/spider_eyes.png`, alpha-blended at `(1 - brightness) × 0.5` (the
//! eyes glow in the dark), the death tilt 180.

use mc_sim::entity::{Entity, base as ebase};

use super::Ctx;
use crate::gui::batch::gl;

/// The eyes' alpha at full-tick brightness `b`.
// src: RenderSpider.java:22
pub fn eye_alpha(b: f32) -> f32 {
    (1.0 - b) * 0.5
}

/// `setSpiderEyeBrightness(spider, pass, partial)` (the doubled `var2 != 0`
/// test is the Java's).
// src: RenderSpider.java:15-29
#[allow(clippy::ifs_same_cond, clippy::if_same_then_else)] // the Java's doubled test
pub fn set_spider_eye_brightness(cx: &mut Ctx, var1: &Entity, var2: i32) -> bool {
    if var2 != 0 {
        false
    } else if var2 != 0 {
        false
    } else {
        cx.load_texture("/mob/spider_eyes.png");
        let var4 = eye_alpha(ebase::get_entity_brightness(var1, cx.world, 1.0));
        cx.gl.enable(gl::BLEND);
        cx.gl.disable(gl::ALPHA_TEST);
        cx.gl.blend_func(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        cx.gl.color4f(1.0, 1.0, 1.0, var4);
        true
    }
}
