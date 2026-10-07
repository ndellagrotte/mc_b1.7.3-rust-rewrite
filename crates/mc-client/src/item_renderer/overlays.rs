//! `ItemRenderer.renderOverlays(partial)` (ItemRenderer.java:277-409) as
//! data: the screen-filling quads `renderHand` draws after the hand with
//! the modelview reset to the identity (EntityRenderer.java:317-321), lit
//! by nothing (`disableStandardItemLighting` ended the hand), alpha test
//! off, depth test on.
//!
//! - **Fire** (`isBurning`): the fire tile and the one below it
//!   (`Block.fire.blockIndexInTexture + 16·i`), two quads at z −0.5, each
//!   shifted sideways by ∓0.24 and down by 0.3 and turned ±10° about y,
//!   blended (770, 771) at `glColor(1, 1, 1, 0.9)`.
//! - **Inside a block** (`isEntityInsideOpaqueBlock`): a quad at z −0.5
//!   with `glColor(0.1, 0.1, 0.1, 0.5)`, *not* blended, textured from
//!   `/terrain.png` around the side texture of the block at the player's
//!   position (or of the normal cube found by the eight samples around
//!   it). Vanilla bug, transcribed: the texture rectangle is computed from
//!   `index % 16 / 256` and `index / 16 / 256` without the `* 16` of a tile,
//!   so it is a 16-texel window (plus half a texel each side) whose corner
//!   is texel `(index % 16, index / 16)` near the atlas's top-left, not the
//!   block's tile. When the player's own block is a normal cube the quad is
//!   drawn twice (the first `renderInsideOfBlock` and the unconditional one
//!   after the `if`).
//! - **Water** (`isInsideOfMaterial(water)`): `/misc/water.png` repeated
//!   four times over the quad, scrolled by `-yaw / 64` and `pitch / 64`
//!   (the current rotation, not interpolated), blended (770, 771) at
//!   `glColor(b, b, b, 0.5)` with `b = getEntityBrightness(partial)`.
//!
//! The portal overlay is `GuiIngame`'s (WP-3.6b).

use crate::camera::GlMatrix;
use crate::gl_material::TessBatch;

/// `Block.fire.blockIndexInTexture` (`new BlockFire(51, 31)`).
// src: Block.java:643
pub const FIRE_TEXTURE: i32 = 31;
/// `/misc/water.png`.
// src: ItemRenderer.java:315
pub const WATER_PNG: &str = "/misc/water.png";

/// Which overlay a quad is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OverlayKind {
    /// `renderFireInFirstPerson`, `/terrain.png`, blended.
    Fire,
    /// `renderInsideOfBlock`, `/terrain.png`, opaque.
    InsideBlock,
    /// `renderWarpedTextureOverlay`, `/misc/water.png`, blended.
    Water,
}

/// One overlay draw: a Tessellator quad batch in eye space (the
/// modelview is applied), its `glColor4f`, its kind.
#[derive(Clone, Debug, PartialEq)]
pub struct OverlayDraw {
    /// What it is (texture and blend).
    pub kind: OverlayKind,
    /// `glColor4f`.
    pub color: [f32; 4],
    /// The vertices (eye space), as `addVertexWithUV` gives them.
    pub batch: TessBatch,
}

/// What `renderOverlays` reads of the player and the world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OverlayInput {
    /// `isBurning()`.
    pub burning: bool,
    /// The side-2 texture of the block `renderOverlays` picks when
    /// `isEntityInsideOpaqueBlock()` (`None` when it is not inside, or
    /// the picked id has no block); [`inside_block_texture`].
    pub inside: Option<InsideBlock>,
    /// `isInsideOfMaterial(Material.water)`.
    pub in_water: bool,
    /// `getEntityBrightness(partial)`.
    pub brightness: f32,
    /// `rotationYaw` (current, not interpolated).
    pub yaw: f32,
    /// `rotationPitch`.
    pub pitch: f32,
}

/// The inside-of-block overlay's texture and how often it is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InsideBlock {
    /// `getBlockTextureFromSide(2)`.
    pub texture: i32,
    /// 2 when the player's own block is a normal cube (module docs), else 1.
    pub draws: u32,
}

/// `renderOverlays`' choice of block (ItemRenderer.java:286-311), over the
/// world query `block(x, y, z)` = `(getBlockId, isBlockNormalCube)` and
/// `texture(id)` = `Block.blocksList[id].getBlockTextureFromSide(2)`
/// (`None` for a null block). `pos` is the player's position, `width`
/// and `height` its box.
// src: ItemRenderer.java:286-311
#[allow(clippy::identity_op)] // `(var7 >> 0) % 2`, as the Java
pub fn inside_block_texture(
    pos: [f64; 3],
    width: f32,
    height: f32,
    mut block: impl FnMut(i32, i32, i32) -> (i32, bool),
    texture: impl Fn(i32) -> Option<i32>,
) -> Option<InsideBlock> {
    use mc_core::math::{floor_double, floor_float};
    let var2 = floor_double(pos[0]);
    let var3 = floor_double(pos[1]);
    let var4 = floor_double(pos[2]);
    let (mut var6, cube) = block(var2, var3, var4);
    let mut draws = 0;
    if cube {
        if texture(var6).is_some() {
            draws += 1;
        }
    } else {
        for var7 in 0..8i32 {
            let var8 = (((var7 >> 0) % 2) as f32 - 0.5) * width * 0.9;
            let var9 = (((var7 >> 1) % 2) as f32 - 0.5) * height * 0.2;
            let var10 = (((var7 >> 2) % 2) as f32 - 0.5) * width * 0.9;
            let var11 = floor_float(var2 as f32 + var8);
            let var12 = floor_float(var3 as f32 + var9);
            let var13 = floor_float(var4 as f32 + var10);
            let (id, cube) = block(var11, var12, var13);
            if cube {
                var6 = id;
            }
        }
    }
    let t = texture(var6)?;
    draws += 1;
    Some(InsideBlock { texture: t, draws })
}

/// `renderInsideOfBlock`'s texture rectangle `(u0, u1, v0, v1)` for side
/// texture `var2` (module docs: the missing `* 16`, vanilla bug).
// src: ItemRenderer.java:334-338
pub fn inside_block_uv(var2: i32) -> (f32, f32, f32, f32) {
    let var10 = 0.0078125f32;
    let var11 = (var2 % 16) as f32 / 256.0 - var10;
    let var12 = ((var2 % 16) as f32 + 15.99) / 256.0 + var10;
    let var13 = (var2 / 16) as f32 / 256.0 - var10;
    let var14 = ((var2 / 16) as f32 + 15.99) / 256.0 + var10;
    (var11, var12, var13, var14)
}

/// `renderInsideOfBlock(partial, var2)`.
// src: ItemRenderer.java:323-347
pub fn render_inside_of_block(var2: i32) -> OverlayDraw {
    let var4 = 0.1f32;
    let (var5, var6, var7, var8, var9) = (-1.0f64, 1.0f64, -1.0f64, 1.0f64, -0.5f64);
    let (var11, var12, var13, var14) = inside_block_uv(var2);
    let mut b = TessBatch::default();
    let c = [1.0; 4];
    b.vertex([var5 as f32, var7 as f32, var9 as f32], [var12, var14], c);
    b.vertex([var6 as f32, var7 as f32, var9 as f32], [var11, var14], c);
    b.vertex([var6 as f32, var8 as f32, var9 as f32], [var11, var13], c);
    b.vertex([var5 as f32, var8 as f32, var9 as f32], [var12, var13], c);
    OverlayDraw {
        kind: OverlayKind::InsideBlock,
        color: [var4, var4, var4, 0.5],
        batch: b,
    }
}

/// `renderWarpedTextureOverlay(partial)` (the water).
// src: ItemRenderer.java:349-373
pub fn render_warped_texture_overlay(brightness: f32, yaw: f32, pitch: f32) -> OverlayDraw {
    let var3 = brightness;
    let var4 = 4.0f32;
    let (var5, var6, var7, var8, var9) = (-1.0f32, 1.0f32, -1.0f32, 1.0f32, -0.5f32);
    let var10 = -yaw / 64.0;
    let var11 = pitch / 64.0;
    let mut b = TessBatch::default();
    let c = [1.0; 4];
    b.vertex([var5, var7, var9], [var4 + var10, var4 + var11], c);
    b.vertex([var6, var7, var9], [0.0 + var10, var4 + var11], c);
    b.vertex([var6, var8, var9], [0.0 + var10, 0.0 + var11], c);
    b.vertex([var5, var8, var9], [var4 + var10, 0.0 + var11], c);
    OverlayDraw {
        kind: OverlayKind::Water,
        color: [var3, var3, var3, 0.5],
        batch: b,
    }
}

/// `renderFireInFirstPerson(partial)`: the two fire quads, each under its
/// own `glTranslatef`/`glRotatef` (applied to the vertices here).
// src: ItemRenderer.java:375-409
pub fn render_fire_in_first_person() -> OverlayDraw {
    let var3 = 1.0f32;
    let mut b = TessBatch::default();
    let c = [1.0; 4];
    for var4 in 0..2i32 {
        let var5 = FIRE_TEXTURE + var4 * 16;
        let var6 = (var5 & 15) << 4;
        let var7 = var5 & 240;
        let var8 = var6 as f32 / 256.0;
        let var9 = (var6 as f32 + 15.99) / 256.0;
        let var10 = var7 as f32 / 256.0;
        let var11 = (var7 as f32 + 15.99) / 256.0;
        let var12 = (0.0 - var3) / 2.0;
        let var13 = var12 + var3;
        let var14 = 0.0 - var3 / 2.0;
        let var15 = var14 + var3;
        let var16 = -0.5f32;
        let mut m = GlMatrix::default();
        m.translate((-(var4 * 2 - 1)) as f32 * 0.24, -0.3, 0.0);
        m.rotate((var4 * 2 - 1) as f32 * 10.0, 0.0, 1.0, 0.0);
        let p = |x: f32, y: f32, z: f32| {
            m.0.transform_point3(bevy::math::Vec3::new(x, y, z))
                .to_array()
        };
        b.vertex(p(var12, var14, var16), [var9, var11], c);
        b.vertex(p(var13, var14, var16), [var8, var11], c);
        b.vertex(p(var13, var15, var16), [var8, var10], c);
        b.vertex(p(var12, var15, var16), [var9, var10], c);
    }
    OverlayDraw {
        kind: OverlayKind::Fire,
        color: [1.0, 1.0, 1.0, 0.9],
        batch: b,
    }
}

/// `renderOverlays(partial)`: the draws in order (fire, inside a block,
/// water).
// src: ItemRenderer.java:277-321
pub fn render_overlays(i: &OverlayInput) -> Vec<OverlayDraw> {
    let mut out = Vec::new();
    if i.burning {
        out.push(render_fire_in_first_person());
    }
    if let Some(ib) = i.inside {
        for _ in 0..ib.draws {
            out.push(render_inside_of_block(ib.texture));
        }
    }
    if i.in_water {
        out.push(render_warped_texture_overlay(i.brightness, i.yaw, i.pitch));
    }
    out
}
