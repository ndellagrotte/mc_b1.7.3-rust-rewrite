//! `RenderLightningBolt` (RenderLightningBolt.java): the bolt as triangle
//! strips of a pale blue `(0.45, 0.45, 0.5)` at alpha 0.3, added
//! (`glBlendFunc(GL_SRC_ALPHA, GL_ONE)`), untextured and unlit: a main
//! channel of eight 16-block segments jittered by `Random(boltVertex)`
//! (±5 per segment), two branches (±15), drawn four times at growing
//! widths (`0.1 + 0.2 × layer`).

use mc_core::JavaRandom;
use mc_sim::entity::{Entity, EntityKindData as K};

use super::Ctx;
use super::gl::glx;
use crate::gui::batch::gl;

/// The main channel's offsets: `(var11, var12)` per segment and the end
/// `(var13, var15)`, from `new Random(boltVertex)`.
// src: RenderLightningBolt.java:13-24
pub fn bolt_offsets(bolt_vertex: i64) -> ([f64; 8], [f64; 8], f64, f64) {
    let mut var11 = [0.0f64; 8];
    let mut var12 = [0.0f64; 8];
    let mut var13 = 0.0f64;
    let mut var15 = 0.0f64;
    let mut var17 = JavaRandom::new(bolt_vertex);
    for var18 in (0..=7).rev() {
        var11[var18] = var13;
        var12[var18] = var15;
        var13 += (var17.next_int_bounded(11) - 5) as f64;
        var15 += (var17.next_int_bounded(11) - 5) as f64;
    }
    (var11, var12, var13, var15)
}

/// One strip of the bolt: layer `var45`, branch `var19`, segment `var26`,
/// at the strip's offsets `(var22, var24)` (bottom) and `(var27, var29)`
/// (top), relative to the draw position: the ten vertices.
// src: RenderLightningBolt.java:54-91
#[allow(clippy::too_many_arguments)]
pub fn strip_vertices(
    var2: f64,
    var4: f64,
    var6: f64,
    var45: i32,
    var19: i32,
    var26: i32,
    (var22, var24): (f64, f64),
    (var27, var29): (f64, f64),
) -> Vec<[f64; 3]> {
    let mut var32 = 0.1 + var45 as f64 * 0.2;
    if var19 == 0 {
        var32 *= var26 as f64 * 0.1 + 1.0;
    }
    let mut var34 = 0.1 + var45 as f64 * 0.2;
    if var19 == 0 {
        var34 *= (var26 - 1) as f64 * 0.1 + 1.0;
    }
    let mut out = Vec::with_capacity(10);
    for var36 in 0..5 {
        let mut var37 = var2 + 0.5 - var32;
        let mut var39 = var6 + 0.5 - var32;
        if var36 == 1 || var36 == 2 {
            var37 += var32 * 2.0;
        }
        if var36 == 2 || var36 == 3 {
            var39 += var32 * 2.0;
        }
        let mut var41 = var2 + 0.5 - var34;
        let mut var43 = var6 + 0.5 - var34;
        if var36 == 1 || var36 == 2 {
            var41 += var34 * 2.0;
        }
        if var36 == 2 || var36 == 3 {
            var43 += var34 * 2.0;
        }
        out.push([var41 + var22, var4 + (var26 * 16) as f64, var43 + var24]);
        out.push([
            var37 + var27,
            var4 + ((var26 + 1) * 16) as f64,
            var39 + var29,
        ]);
    }
    out
}

/// Every strip of the bolt in draw order (layer, branch, segment), as the
/// vertices of each `startDrawing(5)` … `draw()`.
// src: RenderLightningBolt.java:26-95
pub fn bolt_strips(bolt_vertex: i64, var2: f64, var4: f64, var6: f64) -> Vec<Vec<[f64; 3]>> {
    let (var11, var12, var13, var15) = bolt_offsets(bolt_vertex);
    let mut strips = Vec::new();
    for var45 in 0..4 {
        let mut var46 = JavaRandom::new(bolt_vertex);
        for var19 in 0..3 {
            let mut var20 = 7;
            let mut var21 = 0;
            if var19 > 0 {
                var20 = 7 - var19;
            }
            if var19 > 0 {
                var21 = var20 - 2;
            }
            let mut var22 = var11[var20 as usize] - var13;
            let mut var24 = var12[var20 as usize] - var15;
            let mut var26 = var20;
            while var26 >= var21 {
                let var27 = var22;
                let var29 = var24;
                if var19 == 0 {
                    var22 += (var46.next_int_bounded(11) - 5) as f64;
                    var24 += (var46.next_int_bounded(11) - 5) as f64;
                } else {
                    var22 += (var46.next_int_bounded(31) - 15) as f64;
                    var24 += (var46.next_int_bounded(31) - 15) as f64;
                }
                strips.push(strip_vertices(
                    var2,
                    var4,
                    var6,
                    var45,
                    var19,
                    var26,
                    (var22, var24),
                    (var27, var29),
                ));
                var26 -= 1;
            }
        }
    }
    strips
}

/// `func_27002_a(bolt, x, y, z, yaw, partial)`.
// src: RenderLightningBolt.java:7-100
pub fn do_render(cx: &mut Ctx, var1: &Entity, var2: f64, var4: f64, var6: f64) {
    let K::LightningBolt(_, l) = &var1.kind else {
        return;
    };
    let g = &mut *cx.gl;
    g.disable(gl::TEXTURE_2D);
    g.disable(gl::LIGHTING);
    g.enable(gl::BLEND);
    g.blend_func(gl::SRC_ALPHA, gl::ONE);
    for strip in bolt_strips(l.bolt_vertex, var2, var4, var6) {
        g.start_drawing(glx::TRIANGLE_STRIP);
        let var31 = 0.5f32;
        g.set_color_rgba_f(0.9 * var31, 0.9 * var31, 1.0 * var31, 0.3);
        for [x, y, z] in strip {
            g.add_vertex(x, y, z);
        }
        g.draw();
    }
    g.disable(gl::BLEND);
    g.enable(gl::LIGHTING);
    g.enable(gl::TEXTURE_2D);
}
