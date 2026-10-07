//! `RenderPainting` (RenderPainting.java): the painting's art from
//! `/art/kz.png` (`EnumArt`'s size and texture offset), one 16×16 tile per
//! block, each tile a box 1/16 deep with the frame's back and edge strips
//! from the same texture, each tile lit by the world's brightness at the
//! block it covers. Scaled 1/16, turned to the painting's yaw.

use mc_core::JavaRandom;
use mc_core::math::floor_double;
use mc_sim::entity::{Entity, EntityKindData as K};

use super::Ctx;
use crate::gui::batch::gl;

/// `RenderPainting`'s fields.
#[derive(Clone, Debug)]
pub struct RenderPainting {
    /// `rand` (seeded 187 per painting; never drawn from).
    pub rand: JavaRandom,
}

impl Default for RenderPainting {
    // src: RenderPainting.java:7
    fn default() -> Self {
        RenderPainting {
            rand: JavaRandom::new(0),
        }
    }
}

/// The front's texture rectangle of tile `(var10, var11)` of an art of
/// size `(var2, var3)` at offset `(var4, var5)`: `(u_right, u_left, v_top,
/// v_bottom)` (`var16`, `var17`, `var18`, `var19`).
// src: RenderPainting.java:37-40
pub fn tile_uv(
    var2: i32,
    var3: i32,
    var4: i32,
    var5: i32,
    var10: i32,
    var11: i32,
) -> (f32, f32, f32, f32) {
    (
        (var4 + var2 - var10 * 16) as f32 / 256.0,
        (var4 + var2 - (var10 + 1) * 16) as f32 / 256.0,
        (var5 + var3 - var11 * 16) as f32 / 256.0,
        (var5 + var3 - (var11 + 1) * 16) as f32 / 256.0,
    )
}

/// The block whose light tile `(var2, var3)` (its centre, in pixels from
/// the painting's centre) is lit by, by `direction`.
// src: RenderPainting.java:91-110
pub fn tile_light_pos(pos: [f64; 3], direction: i32, var2: f32, var3: f32) -> [i32; 3] {
    let mut var4 = floor_double(pos[0]);
    let var5 = floor_double(pos[1] + (var3 / 16.0) as f64);
    let mut var6 = floor_double(pos[2]);
    if direction == 0 {
        var4 = floor_double(pos[0] + (var2 / 16.0) as f64);
    }
    if direction == 1 {
        var6 = floor_double(pos[2] - (var2 / 16.0) as f64);
    }
    if direction == 2 {
        var4 = floor_double(pos[0] - (var2 / 16.0) as f64);
    }
    if direction == 3 {
        var6 = floor_double(pos[2] + (var2 / 16.0) as f64);
    }
    [var4, var5, var6]
}

impl RenderPainting {
    /// `func_158_a(painting, x, y, z, yaw, partial)`.
    // src: RenderPainting.java:9-22
    #[allow(clippy::too_many_arguments)]
    pub fn do_render(
        &mut self,
        cx: &mut Ctx,
        var1: &Entity,
        var2: f64,
        var4: f64,
        var6: f64,
        var8: f32,
    ) {
        let K::Painting(p) = &var1.kind else {
            return;
        };
        let Some(art) = p.art else {
            return;
        };
        self.rand.set_seed(187);
        cx.gl.push_matrix();
        cx.gl.translatef(var2 as f32, var4 as f32, var6 as f32);
        cx.gl.rotatef(var8, 0.0, 1.0, 0.0);
        cx.gl.enable(gl::RESCALE_NORMAL);
        cx.load_texture("/art/kz.png");
        let var10 = art.def();
        let var11 = 0.0625f32;
        cx.gl.scalef(var11, var11, var11);
        func_159_a(
            cx,
            var1,
            p.direction,
            var10.size_x,
            var10.size_y,
            var10.offset_x,
            var10.offset_y,
        );
        cx.gl.disable(gl::RESCALE_NORMAL);
        cx.gl.pop_matrix();
    }
}

/// `func_159_a(painting, sizeX, sizeY, offsetX, offsetY)`: the tiles.
// src: RenderPainting.java:24-89
#[allow(clippy::too_many_arguments)]
fn func_159_a(
    cx: &mut Ctx,
    var1: &Entity,
    direction: i32,
    var2: i32,
    var3: i32,
    var4: i32,
    var5: i32,
) {
    let var6 = (-var2) as f32 / 2.0;
    let var7 = (-var3) as f32 / 2.0;
    let var8 = -0.5f32;
    let var9 = 0.5f32;
    let pos = [var1.base.pos_x, var1.base.pos_y, var1.base.pos_z];
    for var10 in 0..var2 / 16 {
        for var11 in 0..var3 / 16 {
            let var12 = var6 + ((var10 + 1) * 16) as f32;
            let var13 = var6 + (var10 * 16) as f32;
            let var14 = var7 + ((var11 + 1) * 16) as f32;
            let var15 = var7 + (var11 * 16) as f32;
            // func_160_a: glColor3f(world.getLightBrightness(the tile's block))
            let [lx, ly, lz] =
                tile_light_pos(pos, direction, (var12 + var13) / 2.0, (var14 + var15) / 2.0);
            let var7l = cx.world.get_light_brightness(lx, ly, lz);
            cx.gl.color3f(var7l, var7l, var7l);
            let (var16, var17, var18, var19) = tile_uv(var2, var3, var4, var5, var10, var11);
            let var20 = 0.75f32;
            let var21 = 0.8125f32;
            let var22 = 0.0f32;
            let var23 = 0.0625f32;
            let var24 = 0.75f32;
            let var25 = 0.8125f32;
            let var26 = 0.001953125f32;
            let var27 = 0.001953125f32;
            let var28 = 0.7519531f32;
            let var29 = 0.7519531f32;
            let var30 = 0.0f32;
            let var31 = 0.0625f32;
            let (a, b, c, d) = (var12 as f64, var13 as f64, var14 as f64, var15 as f64);
            let (z8, z9) = (var8 as f64, var9 as f64);
            let f = |v: f32| v as f64;
            let g = &mut *cx.gl;
            g.start_drawing_quads();
            g.set_normal(0.0, 0.0, -1.0);
            g.add_vertex_with_uv(a, d, z8, f(var17), f(var18));
            g.add_vertex_with_uv(b, d, z8, f(var16), f(var18));
            g.add_vertex_with_uv(b, c, z8, f(var16), f(var19));
            g.add_vertex_with_uv(a, c, z8, f(var17), f(var19));
            g.set_normal(0.0, 0.0, 1.0);
            g.add_vertex_with_uv(a, c, z9, f(var20), f(var22));
            g.add_vertex_with_uv(b, c, z9, f(var21), f(var22));
            g.add_vertex_with_uv(b, d, z9, f(var21), f(var23));
            g.add_vertex_with_uv(a, d, z9, f(var20), f(var23));
            g.set_normal(0.0, -1.0, 0.0);
            g.add_vertex_with_uv(a, c, z8, f(var24), f(var26));
            g.add_vertex_with_uv(b, c, z8, f(var25), f(var26));
            g.add_vertex_with_uv(b, c, z9, f(var25), f(var27));
            g.add_vertex_with_uv(a, c, z9, f(var24), f(var27));
            g.set_normal(0.0, 1.0, 0.0);
            g.add_vertex_with_uv(a, d, z9, f(var24), f(var26));
            g.add_vertex_with_uv(b, d, z9, f(var25), f(var26));
            g.add_vertex_with_uv(b, d, z8, f(var25), f(var27));
            g.add_vertex_with_uv(a, d, z8, f(var24), f(var27));
            g.set_normal(-1.0, 0.0, 0.0);
            g.add_vertex_with_uv(a, c, z9, f(var29), f(var30));
            g.add_vertex_with_uv(a, d, z9, f(var29), f(var31));
            g.add_vertex_with_uv(a, d, z8, f(var28), f(var31));
            g.add_vertex_with_uv(a, c, z8, f(var28), f(var30));
            g.set_normal(1.0, 0.0, 0.0);
            g.add_vertex_with_uv(b, c, z8, f(var29), f(var30));
            g.add_vertex_with_uv(b, d, z8, f(var29), f(var31));
            g.add_vertex_with_uv(b, d, z9, f(var28), f(var31));
            g.add_vertex_with_uv(b, c, z9, f(var28), f(var30));
            g.draw();
        }
    }
}
