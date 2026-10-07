//! `RenderGhast` (RenderGhast.java): the ghast scaled about 4.5, swelling
//! sideways and shrinking upward while it charges a fireball (its texture
//! switches to `/mob/ghast_fire.png` through `EntityGhast.texture`).

use mc_sim::entity::{Entity, EntityKindData as K};

use super::Ctx;

/// The ghast's scale `(xz, y)` for the interpolated attack counter `c`.
// src: RenderGhast.java:10-21
pub fn ghast_scale(c: f32) -> (f32, f32) {
    let mut var4 = c / 20.0;
    if var4 < 0.0 {
        var4 = 0.0;
    }
    var4 = 1.0 / (var4 * var4 * var4 * var4 * var4 * 2.0 + 1.0);
    let var5 = (8.0 + var4) / 2.0;
    let var6 = (8.0 + 1.0 / var4) / 2.0;
    (var6, var5)
}

/// `func_4014_a(ghast, partial)` (its `preRenderCallback`).
// src: RenderGhast.java:10-21
pub fn func_4014_a(cx: &mut Ctx, var1: &Entity, var2: f32) {
    let K::Ghast(_, _, g) = &var1.kind else {
        return;
    };
    let c = g.prev_attack_counter as f32 + (g.attack_counter - g.prev_attack_counter) as f32 * var2;
    let (var6, var5) = ghast_scale(c);
    cx.gl.scalef(var6, var5, var6);
    cx.gl.color4f(1.0, 1.0, 1.0, 1.0);
}
