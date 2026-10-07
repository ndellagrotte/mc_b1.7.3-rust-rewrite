//! `RenderCreeper` (RenderCreeper.java): the swelling before the explosion
//! (`updateCreeperScale`), the white flash every other tenth of the fuse
//! (`updateCreeperColorMultiplier`, drawn by `RenderLiving`'s overlay), and
//! the powered creeper's moving `/armor/power.png` on `ModelCreeper(2.0F)`
//! (pass 1: `GL_ONE, GL_ONE` blending, lighting off, the `GL_TEXTURE` matrix
//! translated by `ticks × 0.01`; pass 2 restores the state).

use mc_core::math::sin;
use mc_sim::entity::{Entity, creeper as ecreeper};

use super::Ctx;
use super::gl::glx;
use super::living::RenderLiving;
use crate::gui::batch::gl;

/// The creeper's swelling scale `(xz, y)` at flash time `var4`.
// src: RenderCreeper.java:12-28
#[allow(clippy::manual_clamp)] // the Java's two ifs
pub fn creeper_scale(var4_in: f32) -> (f32, f32) {
    let mut var4 = var4_in;
    let var5 = 1.0 + sin(var4 * 100.0) * var4 * 0.01;
    if var4 < 0.0 {
        var4 = 0.0;
    }
    if var4 > 1.0 {
        var4 = 1.0;
    }
    var4 *= var4;
    var4 *= var4;
    let var6 = (1.0 + var4 * 0.4) * var5;
    let var7 = (1.0 + var4 * 0.1) / var5;
    (var6, var7)
}

/// `updateCreeperScale(creeper, partial)`.
// src: RenderCreeper.java:12-28
pub fn update_creeper_scale(cx: &mut Ctx, var1: &Entity, var2: f32) {
    let var4 = ecreeper::set_creeper_flash_time(var1, var2);
    let (var6, var7) = creeper_scale(var4);
    cx.gl.scalef(var6, var7, var6);
}

/// The flash's ARGB at flash time `var5`: none on even tenths, else white
/// at `(int)(var5 × 0.2 × 255)` alpha.
// src: RenderCreeper.java:30-49
#[allow(clippy::manual_clamp)] // the Java's two ifs
pub fn creeper_color_multiplier(var5: f32) -> i32 {
    if (var5 * 10.0) as i32 % 2 == 0 {
        0
    } else {
        let mut var6 = (var5 * 0.2 * 255.0) as i32;
        if var6 < 0 {
            var6 = 0;
        }
        if var6 > 255 {
            var6 = 255;
        }
        let (var7, var8, var9) = (255i32, 255i32, 255i32);
        var6 << 24 | var7 << 16 | var8 << 8 | var9
    }
}

/// `updateCreeperColorMultiplier(creeper, brightness, partial)`.
// src: RenderCreeper.java:30-49
pub fn update_creeper_color_multiplier(var1: &Entity, var3: f32) -> i32 {
    creeper_color_multiplier(ecreeper::set_creeper_flash_time(var1, var3))
}

/// `func_27006_a(creeper, pass, partial)`: the powered creeper's armour
/// pass.
// src: RenderCreeper.java:51-81
pub fn func_27006_a(
    r: &mut RenderLiving,
    cx: &mut Ctx,
    var1: &Entity,
    var2: i32,
    var3: f32,
) -> bool {
    if ecreeper::get_powered(var1) {
        if var2 == 1 {
            let var4 = var1.base.ticks_existed as f32 + var3;
            cx.load_texture("/armor/power.png");
            cx.gl.matrix_mode(glx::TEXTURE);
            cx.gl.load_identity();
            let var5 = var4 * 0.01;
            let var6 = var4 * 0.01;
            cx.gl.translatef(var5, var6, 0.0);
            r.set_render_pass_model(0);
            cx.gl.matrix_mode(glx::MODELVIEW);
            cx.gl.enable(gl::BLEND);
            let var7 = 0.5;
            cx.gl.color4f(var7, var7, var7, 1.0);
            cx.gl.disable(gl::LIGHTING);
            cx.gl.blend_func(gl::ONE, gl::ONE);
            return true;
        }
        if var2 == 2 {
            cx.gl.matrix_mode(glx::TEXTURE);
            cx.gl.load_identity();
            cx.gl.matrix_mode(glx::MODELVIEW);
            cx.gl.enable(gl::LIGHTING);
            cx.gl.disable(gl::BLEND);
        }
    }
    false
}
