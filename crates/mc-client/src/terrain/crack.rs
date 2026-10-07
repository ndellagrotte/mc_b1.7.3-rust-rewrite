//! `RenderGlobal.drawBlockBreaking` (RenderGlobal.java:1068-1144) with
//! `var3 == 0`, the only way `renderWorld` calls it: while the player is
//! breaking a block (`damagePartialTime > 0`), the block under the cursor
//! is drawn again with the crack tile `240 + (int)(damagePartialTime * 10)`
//! on every face (`renderBlockUsingTexture`), colours disabled, under
//! `glColor4f(1, 1, 1, 0.5F)`, blended `(GL_DST_COLOR, GL_SRC_COLOR)`
//! (the overlay's grey doubles against the block: 0.5 leaves it as is,
//! darker texels darken it), alpha-tested, with `glPolygonOffset(-3, -3)`
//! so it wins the depth test against the block itself.
//!
//! The block is drawn with its own render type (an untranscribed type
//! shows no crack until WP-3.3b-d fill it); air (a mouse-over that is not a
//! block hit still has block coordinates) draws as stone, as the Java does
//! (`var9 == null → Block.stone`, :1090-1092). `globalRenderBlocks` reads
//! the `World` directly; here a snapshot of the block's neighbourhood
//! stands in for it ([`super::snapshot::ChunkSnapshot`], loaded chunks only).
//!
//! When: `renderWorld` draws it for a player view with `objectMouseOver`
//! set, before the translucent terrain with the eye in water, after it
//! otherwise (EntityRenderer.java:507-513, :549-555), and before the
//! selection box at each site ([`super::order`]).

use bevy::prelude::*;
use mc_sim::world::World;
use mc_sim::world::ray_trace::MovingObjectPosition;

use super::render_blocks::RenderBlocks;
use super::snapshot::{ChunkLoad, ChunkSnapshot, SnapshotTemplate};
use super::tessellator::Tessellator;

/// `Block.stone.blockID`.
// src: Block.java:593
const STONE: i32 = 1;

/// The crack tile for a damage: `240 + (int)(damagePartialTime * 10.0F)`.
// src: RenderGlobal.java:1098
pub fn crack_texture(damage_partial_time: f32) -> i32 {
    240 + (damage_partial_time * 10.0) as i32
}

/// The overlay's quads for the block `m` points at, relative to the
/// block's minimum corner; `None` when nothing is being broken
/// (`damagePartialTime <= 0`).
// src: RenderGlobal.java:1075-1107
pub fn crack_quads(
    world: &mut World,
    template: &SnapshotTemplate,
    m: &MovingObjectPosition,
    damage_partial_time: f32,
    ambient_occlusion: bool,
    anaglyph: bool,
) -> Option<Tessellator> {
    if damage_partial_time <= 0.0 {
        return None;
    }
    let (x, y, z) = (m.block_x, m.block_y, m.block_z);
    let var8 = world.get_block_id(x, y, z);
    let var9 = if var8 > 0 && mc_sim::block::block(var8).is_some() {
        var8
    } else {
        STONE
    };
    let snap = ChunkSnapshot::build(
        world,
        template,
        [x, y, z],
        [x + 1, y + 1, z + 1],
        ChunkLoad::LoadedOnly,
    );
    let mut rb = RenderBlocks::new(&snap, snap.statics.clone(), ambient_occlusion, anaglyph);
    rb.tess.start_drawing_quads();
    rb.tess
        .set_translation_d(-(x as f64), -(y as f64), -(z as f64));
    rb.tess.disable_color();
    rb.render_block_using_texture(var9, x, y, z, crack_texture(damage_partial_time));
    rb.tess.set_translation_d(0.0, 0.0, 0.0);
    (!rb.tess.is_empty()).then_some(rb.tess)
}

/// What the overlay's quads depend on: the block, the stage, the settings,
/// the leaves' graphics level and the 3×3×3 blocks around (face culling,
/// neighbour-dependent bounds and textures).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrackKey {
    pos: [i32; 3],
    stage: i32,
    settings: [bool; 3],
    blocks: [(i32, i32); 27],
}

impl CrackKey {
    /// The key of the overlay `crack_quads` would draw now (`stage` is -1
    /// when nothing is being broken).
    pub fn of(
        world: &mut World,
        template: &SnapshotTemplate,
        m: &MovingObjectPosition,
        damage_partial_time: f32,
        ambient_occlusion: bool,
        anaglyph: bool,
    ) -> Self {
        let (x, y, z) = (m.block_x, m.block_y, m.block_z);
        let mut blocks = [(0, 0); 27];
        let mut i = 0;
        for dy in -1..=1 {
            for dz in -1..=1 {
                for dx in -1..=1 {
                    blocks[i] = (
                        world.get_block_id(x + dx, y + dy, z + dz),
                        world.get_block_metadata(x + dx, y + dy, z + dz),
                    );
                    i += 1;
                }
            }
        }
        CrackKey {
            pos: [x, y, z],
            stage: if damage_partial_time > 0.0 {
                crack_texture(damage_partial_time)
            } else {
                -1
            },
            settings: [ambient_occlusion, anaglyph, template.statics.graphics_level],
            blocks,
        }
    }
}

/// Marks the crack overlay's entity.
#[derive(Component, Clone, Copy, Debug)]
pub struct CrackOverlay;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crack_stages() {
        assert_eq!(crack_texture(0.0), 240);
        assert_eq!(crack_texture(0.05), 240);
        assert_eq!(crack_texture(0.15), 241);
        assert_eq!(crack_texture(0.99), 249);
    }
}
