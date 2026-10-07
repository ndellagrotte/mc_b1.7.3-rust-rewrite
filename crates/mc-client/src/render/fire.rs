//! `Render.renderEntityOnFire` (Render.java:30-89): the fire tiles of
//! `/terrain.png` (`Block.fire.blockIndexInTexture`, 31, and the tile below
//! it, 47, alternating; their texture is WP-3.2's animated fire) as a column
//! of quads facing the camera, scaled to the entity's width, each step 0.45
//! higher, 0.9 times narrower and 0.03 further back.

use mc_sim::entity::Entity;

use super::Ctx;
use crate::gui::batch::gl;

/// `Block.fire.blockIndexInTexture`.
// src: Block.java:643 (Block.fire = new BlockFire(51, 31))
pub const FIRE_TEXTURE: i32 = 31;

/// The texture rectangle of step `var23`: `(u0, u1, v0, v1)` after the
/// swap of every other pair (Render.java:57-73).
// src: Render.java:57-73
pub fn fire_uv(var23: i32) -> (f32, f32, f32, f32) {
    let var9 = FIRE_TEXTURE;
    let var10 = (var9 & 15) << 4;
    let var11 = var9 & 240;
    let (mut var12, mut var13, var14, var15);
    if var23 % 2 == 0 {
        var12 = var10 as f32 / 256.0;
        var13 = (var10 as f32 + 15.99) / 256.0;
        var14 = var11 as f32 / 256.0;
        var15 = (var11 as f32 + 15.99) / 256.0;
    } else {
        var12 = var10 as f32 / 256.0;
        var13 = (var10 as f32 + 15.99) / 256.0;
        var14 = (var11 + 16) as f32 / 256.0;
        var15 = ((var11 + 16) as f32 + 15.99) / 256.0;
    }
    if var23 / 2 % 2 == 0 {
        std::mem::swap(&mut var12, &mut var13);
    }
    (var12, var13, var14, var15)
}

/// `renderEntityOnFire(entity, x, y, z, partial)`.
// src: Render.java:30-89
pub fn render_entity_on_fire(
    cx: &mut Ctx,
    var1: &Entity,
    var2: f64,
    var4: f64,
    var6: f64,
    _var8: f32,
) {
    cx.gl.disable(gl::LIGHTING);
    cx.gl.push_matrix();
    cx.gl.translatef(var2 as f32, var4 as f32, var6 as f32);
    let b = &var1.base;
    let var16 = b.width * 1.4;
    cx.gl.scalef(var16, var16, var16);
    cx.load_texture(crate::textures::TERRAIN_PNG);
    let mut var18 = 0.5f32;
    let var19 = 0.0f32;
    let mut var20 = b.height / var16;
    let mut var21 = (b.pos_y - b.bounding_box.min_y) as f32;
    let view_y = cx.info.player_view_y;
    let g = &mut *cx.gl;
    g.rotatef(-view_y, 0.0, 1.0, 0.0);
    g.translatef(0.0, 0.0, -0.3 + (var20 as i32) as f32 * 0.02);
    g.color4f(1.0, 1.0, 1.0, 1.0);
    let mut var22 = 0.0f32;
    let mut var23 = 0;
    g.start_drawing_quads();
    while var20 > 0.0 {
        let (var12, var13, var14, var15) = fire_uv(var23);
        g.add_vertex_with_uv(
            (var18 - var19) as f64,
            (0.0 - var21) as f64,
            var22 as f64,
            var13 as f64,
            var15 as f64,
        );
        g.add_vertex_with_uv(
            (-var18 - var19) as f64,
            (0.0 - var21) as f64,
            var22 as f64,
            var12 as f64,
            var15 as f64,
        );
        g.add_vertex_with_uv(
            (-var18 - var19) as f64,
            (1.4 - var21) as f64,
            var22 as f64,
            var12 as f64,
            var14 as f64,
        );
        g.add_vertex_with_uv(
            (var18 - var19) as f64,
            (1.4 - var21) as f64,
            var22 as f64,
            var13 as f64,
            var14 as f64,
        );
        var20 -= 0.45;
        var21 -= 0.45;
        var18 *= 0.9;
        var22 += 0.03;
        var23 += 1;
    }
    g.draw();
    g.pop_matrix();
    g.enable(gl::LIGHTING);
}
