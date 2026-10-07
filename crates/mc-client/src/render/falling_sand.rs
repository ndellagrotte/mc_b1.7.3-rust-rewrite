//! `RenderFallingSand` (RenderFallingSand.java): the falling block's six
//! faces (`RenderBlocks.renderBlockFallingSand`, RenderBlocks.java:
//! 1665-1716) around the entity, each shaded by the face factor (0.5 down,
//! 1 up, 0.8 north/south, 0.6 east/west) times the brighter of the light at
//! the entity's block and at the neighbour the face looks at; lighting off.

use mc_core::math::floor_double;
use mc_sim::entity::{Entity, EntityKindData as K};
use mc_sim::world::World;

use super::Ctx;
use crate::gui::batch::gl;
use crate::terrain::blocks;
use crate::terrain::render_blocks::RenderBlocks;
use crate::terrain::snapshot::ChunkSnapshot;
use crate::textures::TERRAIN_PNG;

/// `block.getBlockBrightness(world, x, y, z)`: `world.getBrightness(x, y,
/// z, lightValue[id])`.
// src: Block.java (getBlockBrightness), World.java:687-694
fn block_brightness(world: &mut World, id: i32, x: i32, y: i32, z: i32) -> f32 {
    world.get_brightness(x, y, z, blocks::light_value(id))
}

/// `doRenderFallingSand(entity, x, y, z, yaw, partial)`.
// src: RenderFallingSand.java:12-22
pub fn do_render(cx: &mut Ctx, var1: &Entity, var2: f64, var4: f64, var6: f64) {
    let K::FallingSand(f) = &var1.kind else {
        return;
    };
    cx.gl.push_matrix();
    cx.gl.translatef(var2 as f32, var4 as f32, var6 as f32);
    cx.load_texture(TERRAIN_PNG);
    cx.gl.disable(gl::LIGHTING);
    let b = &var1.base;
    render_block_falling_sand(
        cx,
        f.block_id,
        floor_double(b.pos_x),
        floor_double(b.pos_y),
        floor_double(b.pos_z),
    );
    cx.gl.enable(gl::LIGHTING);
    cx.gl.pop_matrix();
}

/// `RenderBlocks.renderBlockFallingSand(block, world, x, y, z)`.
// src: RenderBlocks.java:1665-1716
pub fn render_block_falling_sand(cx: &mut Ctx, id: i32, var3: i32, var4: i32, var5: i32) {
    if blocks::def(id).is_none() {
        return;
    }
    let var6 = 0.5f32;
    let var7 = 1.0f32;
    let var8 = 0.8f32;
    let var9 = 0.6f32;
    let snap = ChunkSnapshot::empty(cx.statics.clone(), cx.colorizers.clone());
    let mut rb = RenderBlocks::new(&snap, snap.statics.clone(), false, cx.info.anaglyph);
    rb.tess.start_drawing_quads();
    let w = &mut *cx.world;
    let var11 = block_brightness(w, id, var3, var4, var5);
    let tex = |rb: &RenderBlocks, side| blocks::get_block_texture_from_side(&rb.statics, id, side);
    let face = |rb: &mut RenderBlocks, w: &mut World, shade: f32, n: [i32; 3], side: i32| {
        let mut var12 = block_brightness(w, id, var3 + n[0], var4 + n[1], var5 + n[2]);
        if var12 < var11 {
            var12 = var11;
        }
        rb.tess
            .set_color_opaque_f(shade * var12, shade * var12, shade * var12);
        let t = tex(rb, side);
        match side {
            0 => rb.render_bottom_face(id, -0.5, -0.5, -0.5, t),
            1 => rb.render_top_face(id, -0.5, -0.5, -0.5, t),
            2 => rb.render_east_face(id, -0.5, -0.5, -0.5, t),
            3 => rb.render_west_face(id, -0.5, -0.5, -0.5, t),
            4 => rb.render_north_face(id, -0.5, -0.5, -0.5, t),
            _ => rb.render_south_face(id, -0.5, -0.5, -0.5, t),
        }
    };
    face(&mut rb, w, var6, [0, -1, 0], 0);
    face(&mut rb, w, var7, [0, 1, 0], 1);
    face(&mut rb, w, var8, [0, 0, -1], 2);
    face(&mut rb, w, var8, [0, 0, 1], 3);
    face(&mut rb, w, var9, [-1, 0, 0], 4);
    face(&mut rb, w, var9, [1, 0, 0], 5);
    let t = std::mem::take(&mut rb.tess);
    cx.gl.draw_tessellator(&t);
}
