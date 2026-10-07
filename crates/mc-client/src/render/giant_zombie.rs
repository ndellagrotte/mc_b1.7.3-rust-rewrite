//! `RenderGiantZombie` (RenderGiantZombie.java): `ModelZombie` scaled 6
//! (and its shadow 0.5 × 6).

use super::Ctx;

/// `preRenderScale(giant, partial)`.
// src: RenderGiantZombie.java:13-15
pub fn pre_render_scale(cx: &mut Ctx, scale: f32) {
    cx.gl.scalef(scale, scale, scale);
}
