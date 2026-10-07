//! `RenderTNTPrimed` (RenderTNTPrimed.java): the TNT block
//! (`renderBlockOnInventory`) swelling by up to 30 % over its last ten
//! ticks, and every other five ticks of the fuse drawn again untextured
//! and white, blended `(GL_SRC_ALPHA, GL_DST_ALPHA)` with `glColor4f(1, 1,
//! 1, (1 - (fuse - partial + 1) / 100) × 0.8)` — which the block renderer's
//! own `glColor4f(…, 1.0F)` replaces (vanilla bug, transcribed: the flash
//! is opaque white).

use mc_sim::entity::{Entity, EntityKindData as K};

use super::Ctx;
use crate::gui::batch::gl;
use crate::textures::TERRAIN_PNG;

/// `Block.tnt`'s id.
// src: Block.java:638
pub const TNT: i32 = 46;

/// The swell scale at `fuse - partial + 1`, if swelling.
// src: RenderTNTPrimed.java:16-30
pub fn tnt_scale(fuse: i32, var9: f32) -> Option<f32> {
    if (fuse as f32 - var9 + 1.0) < 10.0 {
        let mut var10 = 1.0 - (fuse as f32 - var9 + 1.0) / 10.0;
        var10 = var10.clamp(0.0, 1.0);
        var10 *= var10;
        var10 *= var10;
        Some(1.0 + var10 * 0.3)
    } else {
        None
    }
}

/// The flash's alpha.
// src: RenderTNTPrimed.java:32
pub fn tnt_flash_alpha(fuse: i32, var9: f32) -> f32 {
    (1.0 - (fuse as f32 - var9 + 1.0) / 100.0) * 0.8
}

/// `func_153_a(tnt, x, y, z, yaw, partial)`.
// src: RenderTNTPrimed.java:12-49
pub fn do_render(cx: &mut Ctx, var1: &Entity, var2: f64, var4: f64, var6: f64, var9: f32) {
    let K::TntPrimed(t) = &var1.kind else {
        return;
    };
    cx.gl.push_matrix();
    cx.gl.translatef(var2 as f32, var4 as f32, var6 as f32);
    if let Some(var11) = tnt_scale(t.fuse, var9) {
        cx.gl.scalef(var11, var11, var11);
    }
    let var10 = tnt_flash_alpha(t.fuse, var9);
    cx.load_texture(TERRAIN_PNG);
    let br = cx.brightness(var1, var9);
    cx.render_block_on_inventory(TNT, 0, br);
    if t.fuse / 5 % 2 == 0 {
        cx.gl.disable(gl::TEXTURE_2D);
        cx.gl.disable(gl::LIGHTING);
        cx.gl.enable(gl::BLEND);
        cx.gl.blend_func(gl::SRC_ALPHA, gl::DST_ALPHA);
        cx.gl.color4f(1.0, 1.0, 1.0, var10);
        // vanilla bug: renderBlockOnInventory sets glColor4f(…, 1.0F) itself
        // (RenderBlocks.java:3176-3181), replacing var10's alpha: the flash
        // is opaque white.
        cx.render_block_on_inventory(TNT, 0, 1.0);
        cx.gl.color4f(1.0, 1.0, 1.0, 1.0);
        cx.gl.disable(gl::BLEND);
        cx.gl.enable(gl::LIGHTING);
        cx.gl.enable(gl::TEXTURE_2D);
    }
    cx.gl.pop_matrix();
}
