//! `TileEntityRendererPiston` (TileEntityRendererPiston.java): the block a
//! piston moves, drawn by `RenderBlocks` on the world at its tile entity's
//! position shifted by the interpolated progress — the piston head while a
//! retracting piston's head slides back (with the head texture of its
//! piston, then the base), else the moved block with every face
//! (`func_31075_a`). Lighting off, blended, culling off; `GL_SMOOTH` with
//! smooth lighting. The base and head geometry are WP-3.3d's
//! (`func_31074_b`, `func_31080_c`).

use mc_sim::block::piston_base;
use mc_sim::world::tile_entity::{TileEntity, TileEntityPiston};

use super::super::Ctx;
use crate::gui::batch::gl;
use crate::terrain::render_blocks::RenderBlocks;
use crate::terrain::snapshot::{ChunkLoad, ChunkSnapshot, SnapshotTemplate};

/// `Block.pistonExtension`'s id.
// src: Block.java:626
pub const PISTON_EXTENSION: i32 = 34;

/// `TileEntityRendererPiston`'s fields.
#[derive(Default)]
pub struct TileEntityRendererPiston {
    /// `field_31071_b`'s world, as the block renderer's snapshot template
    /// (made on first use after `func_31069_a`).
    template: Option<SnapshotTemplate>,
}

impl TileEntityRendererPiston {
    /// `func_31069_a(world)`: a new `RenderBlocks` for the new world.
    // src: TileEntityRendererPiston.java:46-48
    pub fn func_31069_a(&mut self) {
        self.template = None;
    }

    /// `func_31070_a(piston, x, y, z, partial)`.
    // src: TileEntityRendererPiston.java:9-44
    #[allow(clippy::too_many_arguments)]
    pub fn func_31070_a(
        &mut self,
        cx: &mut Ctx,
        te: &TileEntity,
        var1: &TileEntityPiston,
        var2: f64,
        var4: f64,
        var6: f64,
        var8: f32,
    ) {
        let var9 = var1.get_stored_block_id();
        if crate::terrain::blocks::def(var9).is_none() || var1.get_progress(var8) >= 1.0 {
            return;
        }
        let (x, y, z) = (te.x_coord, te.y_coord, te.z_coord);
        cx.gl.bind_texture(crate::textures::TERRAIN_PNG);
        cx.gl.disable_standard_item_lighting();
        cx.gl.blend_func(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        cx.gl.enable(gl::BLEND);
        cx.gl.disable(gl::CULL_FACE);
        if cx.info.ambient_occlusion {
            cx.gl.shade_model(gl::SMOOTH);
        } else {
            cx.gl.shade_model(gl::FLAT);
        }
        let template = match &mut self.template {
            Some(t) => {
                t.refresh_statics(cx.world);
                t
            }
            None => self
                .template
                .insert(SnapshotTemplate::of(cx.world, cx.colorizers)),
        };
        let snap = ChunkSnapshot::build(
            cx.world,
            template,
            [x, y, z],
            [x + 1, y + 1, z + 1],
            ChunkLoad::LoadedOnly,
        );
        let mut rb = RenderBlocks::new(
            &snap,
            snap.statics.clone(),
            cx.info.ambient_occlusion,
            cx.info.anaglyph,
        );
        rb.tess.start_drawing_quads();
        rb.tess.set_translation_d(
            ((var2 as f32 - x as f32) + var1.get_offset_x(var8)) as f64,
            ((var4 as f32 - y as f32) + var1.get_offset_y(var8)) as f64,
            ((var6 as f32 - z as f32) + var1.get_offset_z(var8)) as f64,
        );
        rb.tess.set_color_opaque(1, 1, 1);
        if var9 == PISTON_EXTENSION && var1.get_progress(var8) < 0.5 {
            func_31079_a(&mut rb, var9, x, y, z, false);
        } else if var1.should_head_be_rendered() && !var1.is_extending() {
            rb.statics.piston_extension_head_texture =
                piston_base::get_piston_extension_texture(var9);
            func_31079_a(
                &mut rb,
                PISTON_EXTENSION,
                x,
                y,
                z,
                var1.get_progress(var8) < 0.5,
            );
            rb.statics.piston_extension_head_texture = -1;
            rb.tess.set_translation_d(
                (var2 as f32 - x as f32) as f64,
                (var4 as f32 - y as f32) as f64,
                (var6 as f32 - z as f32) as f64,
            );
            func_31078_d(&mut rb, var9, x, y, z);
        } else {
            rb.func_31075_a(var9, x, y, z);
        }
        rb.tess.set_translation_d(0.0, 0.0, 0.0);
        let t = std::mem::take(&mut rb.tess);
        cx.gl.draw_tessellator(&t);
        cx.gl.enable_standard_item_lighting();
    }
}

/// `RenderBlocks.func_31079_a(block, x, y, z, fullRod)`: the piston head
/// with every face.
// src: RenderBlocks.java:543-547
pub fn func_31079_a(rb: &mut RenderBlocks, id: i32, x: i32, y: i32, z: i32, var5: bool) {
    rb.render_all_faces = true;
    rb.func_31080_c(id, x, y, z, var5);
    rb.render_all_faces = false;
}

/// `RenderBlocks.func_31078_d(block, x, y, z)`: the extended piston base
/// with every face.
// src: RenderBlocks.java:381-385
pub fn func_31078_d(rb: &mut RenderBlocks, id: i32, x: i32, y: i32, z: i32) {
    rb.render_all_faces = true;
    rb.func_31074_b(id, x, y, z, true);
    rb.render_all_faces = false;
}
