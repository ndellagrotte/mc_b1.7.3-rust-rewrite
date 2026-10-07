//! WP-3.3c: the plant, torch, fire, crop and cactus render types of
//! `RenderBlocks`: `renderBlockReed` (1) with `renderCrossedSquares`,
//! `renderBlockTorch` (2) with `renderTorchAtAngle`, `renderBlockFire` (3),
//! `renderBlockCrops` (6) with `func_1245_b`, `renderBlockCactus` (13) with
//! `func_1230_b` (RenderBlocks.java:276-301, :796-982, :1307-1482,
//! :2334-2444).
//!
//! # Which blocks
//!
//! | Type | Blocks (ids) | Texture |
//! |---|---|---|
//! | 1 | sapling (6), web (30), tall grass (31), dead bush (32), the two flowers (37, 38), the two mushrooms (39, 40), reed (83) | `getBlockTextureFromSideAndMetadata(0, meta)`: sapling `meta & 3` → 15/63/79 (BlockSapling.java:27-34), tall grass 0/1/2 → 55/39/56 (BlockTallGrass.java:12-20), the rest their index |
//! | 2 | torch (50), redstone torch off (75) and on (76) | `getBlockTextureFromSide(0)` |
//! | 3 | fire (51) | `getBlockTextureFromSide(0)` = 31 and the tile below it (47), both animated by `TextureFlamesFX` |
//! | 6 | crops (59) | `88 + meta` (BlockCrops.java:77-83) |
//! | 13 | cactus (81) | `getBlockTexture`: top 69, bottom 71, sides 70 (BlockCactus.java:40-46) |
//!
//! The textures and `getRenderType` come from `mc-sim`'s block table
//! through [`super::blocks`]; tall grass's `colorMultiplier` (the biome at a
//! hashed offset) is [`super::blocks::color_multiplier`]; fire's
//! `canBlockCatchFire` reads `mc-sim`'s `BlockFire.chanceToEncourageFire`
//! table ([`mc_sim::block::CHANCE_TO_ENCOURAGE_FIRE`]).
//!
//! # Arithmetic
//!
//! As the Java: `float` → `f32`, `double` → `f64`, in its order, so the
//! literal `0.45F` widened to double is `0.44999998807907104`, and the
//! fire's heights are `(double)((float)y + 1.4F + 0.0625F)`. Every quad
//! that must be seen from both sides is emitted twice with opposite
//! windings, as vanilla draws with `GL_CULL_FACE` on.
//!
//! The inventory cases (`renderBlockOnInventory`, types 1, 2, 6) call
//! [`RenderBlocks::render_crossed_squares`], [`RenderBlocks::render_torch_at_angle`]
//! and [`RenderBlocks::func_1245_b`] at (−0.5, −0.5, −0.5) from
//! [`super::render_blocks`]; the cactus's inventory case (13) is drawn
//! there from the faces.

use mc_sim::block::{BlockKind, CHANCE_TO_ENCOURAGE_FIRE};

use super::blocks::{self, bounds};
use super::render_blocks::RenderBlocks;
use super::snapshot::ChunkSnapshot;
use crate::gl_material::anaglyph;

/// `Block.fire.canBlockCatchFire(blockAccess, x, y, z)`:
/// `chanceToEncourageFire[getBlockId(x, y, z)] > 0` (planks, fence, wooden
/// stairs, logs, leaves, bookshelf, TNT, tall grass, wool).
// src: BlockFire.java:172-174, :14-24
pub fn can_block_catch_fire(s: &ChunkSnapshot, x: i32, y: i32, z: i32) -> bool {
    CHANCE_TO_ENCOURAGE_FIRE[(s.get_block_id(x, y, z) & 255) as usize] > 0
}

/// The tall-grass offset `renderBlockReed` adds to the drawing position
/// (not to the block): a hash of the position picks −0.25..+0.25 in x and z
/// and −0.2..0 in y, in sixteen steps.
// src: RenderBlocks.java:1327-1333
pub fn tall_grass_offset(x: i32, y: i32, z: i32) -> (f64, f64, f64) {
    // `(long)(var2 * 3129871)`: the int product overflows before the widening.
    let mut var17 =
        (x.wrapping_mul(3129871) as i64) ^ (z as i64).wrapping_mul(116129781) ^ (y as i64);
    var17 = var17
        .wrapping_mul(var17)
        .wrapping_mul(42317861)
        .wrapping_add(var17.wrapping_mul(11));
    let dx = (((var17 >> 16 & 15) as f32 / 15.0) as f64 - 0.5) * 0.5;
    let dy = (((var17 >> 20 & 15) as f32 / 15.0) as f64 - 1.0) * 0.2;
    let dz = (((var17 >> 24 & 15) as f32 / 15.0) as f64 - 0.5) * 0.5;
    (dx, dy, dz)
}

/// The full-tile texture rectangle the plant renderers use:
/// `(double)((float)x / 256.0F)` and `(double)(((float)x + 15.99F) / 256.0F)`
/// for `x = (tex & 15) << 4`, `y = tex & 240`; `(u0, u1, v0, v1)`.
// src: RenderBlocks.java:1405-1410
fn tile_rect(tex: i32) -> (f64, f64, f64, f64) {
    let var11 = (tex & 15) << 4;
    let var12 = tex & 240;
    (
        (var11 as f32 / 256.0) as f64,
        ((var11 as f32 + 15.99) / 256.0) as f64,
        (var12 as f32 / 256.0) as f64,
        ((var12 as f32 + 15.99) / 256.0) as f64,
    )
}

impl RenderBlocks<'_> {
    /// `renderBlockReed(block, x, y, z)`: render type 1. Colour =
    /// brightness × `colorMultiplier` (with the anaglyph transform); tall
    /// grass is moved by [`tall_grass_offset`]; then the crossed squares
    /// with the cell's metadata.
    // src: RenderBlocks.java:1307-1337
    pub fn render_block_reed(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let var6 = self.brightness(id, x, y, z);
        let var7 = blocks::color_multiplier(self.block_access, id, x, y, z);
        let mut var8 = (var7 >> 16 & 255) as f32 / 255.0;
        let mut var9 = (var7 >> 8 & 255) as f32 / 255.0;
        let mut var10 = (var7 & 255) as f32 / 255.0;
        if self.anaglyph {
            [var8, var9, var10] = anaglyph([var8, var9, var10]);
        }
        self.tess
            .set_color_opaque_f(var6 * var8, var6 * var9, var6 * var10);
        let mut var19 = x as f64;
        let mut var20 = y as f64;
        let mut var15 = z as f64;
        // `var1 == Block.tallGrass` (id 31, the only `BlockTallGrass`).
        // src: RenderBlocks.java:1327, Block.java:623
        if blocks::def(id).is_some_and(|b| b.kind == BlockKind::TallGrass) {
            let (dx, dy, dz) = tall_grass_offset(x, y, z);
            var19 += dx;
            var20 += dy;
            var15 += dz;
        }
        let meta = self.block_access.get_block_metadata(x, y, z);
        self.render_crossed_squares(id, meta, var19, var20, var15);
        true
    }

    /// `renderBlockTorch(block, x, y, z)`: render type 2. Brightness 1 for a
    /// block that gives light (the torch, the lit redstone torch), else the
    /// cell's; metadata 1-4 lean the torch off the wall (moved 0.1 from the
    /// centre toward the wall and up 0.2, leaning 0.4 per block of height),
    /// any other stands upright.
    // src: RenderBlocks.java:276-301
    pub fn render_block_torch(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let var5 = self.block_access.get_block_metadata(x, y, z);
        let mut var7 = self.brightness(id, x, y, z);
        if blocks::light_value(id) > 0 {
            var7 = 1.0;
        }
        self.tess.set_color_opaque_f(var7, var7, var7);
        // `0.4D`, `0.2D` are the float literals widened.
        let var8 = 0.4000000059604645f64;
        let var10 = 0.5 - var8;
        let var12 = 0.20000000298023224f64;
        let (x, y, z) = (x as f64, y as f64, z as f64);
        if var5 == 1 {
            self.render_torch_at_angle(id, x - var10, y + var12, z, -var8, 0.0);
        } else if var5 == 2 {
            self.render_torch_at_angle(id, x + var10, y + var12, z, var8, 0.0);
        } else if var5 == 3 {
            self.render_torch_at_angle(id, x, y + var12, z - var10, 0.0, -var8);
        } else if var5 == 4 {
            self.render_torch_at_angle(id, x, y + var12, z + var10, 0.0, var8);
        } else {
            self.render_torch_at_angle(id, x, y, z, 0.0, 0.0);
        }
        true
    }

    /// `renderBlockFire(block, x, y, z)`: render type 3, colour = the
    /// brightness. Above a normal cube or a flammable block: two pairs of
    /// crossed slanted planes (1.4 high, leaning 0.2/0.3 and 0.4/0.5 off
    /// the centre), the second pair with the tile below. Otherwise a
    /// leaning flame against each flammable side neighbour (0.2 in from
    /// the cell's edge at the top, 1/16 up), and under a flammable block
    /// two planes hanging from the ceiling. The tile below (`+ 16`) and
    /// mirrored u pick alternate with the position's parity.
    // src: RenderBlocks.java:796-982
    #[allow(unused_assignments, clippy::identity_op)] // the Java's dead stores and `+ 0`s, kept
    pub fn render_block_fire(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let s = self.block_access;
        let (var2, mut var3, var4) = (x, y, z);
        let mut var6 = blocks::get_block_texture_from_side(&self.statics, id, 0);
        if self.override_block_texture >= 0 {
            var6 = self.override_block_texture;
        }
        let var7 = self.brightness(id, var2, var3, var4);
        let t = &mut self.tess;
        t.set_color_opaque_f(var7, var7, var7);
        let var8 = (var6 & 15) << 4;
        let var9 = var6 & 240;
        let mut var10 = (var8 as f32 / 256.0) as f64;
        let mut var12 = ((var8 as f32 + 15.99) / 256.0) as f64;
        let mut var14 = (var9 as f32 / 256.0) as f64;
        let mut var16 = ((var9 as f32 + 15.99) / 256.0) as f64;
        let mut var18 = 1.4f32;
        let mut var21: f64;
        let var23: f64;
        let var25: f64;
        let var27: f64;
        let var29: f64;
        let var31: f64;
        let var33: f64;
        // The second frame's rectangle: the tile below (`var9 + 16`).
        let lower = || {
            (
                (var8 as f32 / 256.0) as f64,
                ((var8 as f32 + 15.99) / 256.0) as f64,
                ((var9 + 16) as f32 / 256.0) as f64,
                ((var9 as f32 + 15.99 + 16.0) / 256.0) as f64,
            )
        };
        let upper = || {
            (
                (var8 as f32 / 256.0) as f64,
                ((var8 as f32 + 15.99) / 256.0) as f64,
                (var9 as f32 / 256.0) as f64,
                ((var9 as f32 + 15.99) / 256.0) as f64,
            )
        };
        let catch = |x, y, z| can_block_catch_fire(s, x, y, z);
        if !s.is_block_normal_cube(var2, var3 - 1, var4) && !catch(var2, var3 - 1, var4) {
            let var37 = 0.2f32;
            let var20 = 0.0625f32;
            if (var2.wrapping_add(var3).wrapping_add(var4) & 1) == 1 {
                (var10, var12, var14, var16) = lower();
            }
            if ((var2 / 2).wrapping_add(var3 / 2).wrapping_add(var4 / 2) & 1) == 1 {
                var21 = var12;
                var12 = var10;
                var10 = var21;
            }
            // The flame's top and bottom heights.
            let top = (var3 as f32 + var18 + var20) as f64;
            let bot = ((var3 + 0) as f32 + var20) as f64;
            let (xf, zf) = (var2 as f32, var4 as f32);
            let (x0, x1) = ((var2 + 0) as f64, (var2 + 1) as f64);
            let (z0, z1) = ((var4 + 0) as f64, (var4 + 1) as f64);

            if catch(var2 - 1, var3, var4) {
                let xi = (xf + var37) as f64;
                t.add_vertex_with_uv(xi, top, z1, var12, var14);
                t.add_vertex_with_uv(x0, bot, z1, var12, var16);
                t.add_vertex_with_uv(x0, bot, z0, var10, var16);
                t.add_vertex_with_uv(xi, top, z0, var10, var14);
                t.add_vertex_with_uv(xi, top, z0, var10, var14);
                t.add_vertex_with_uv(x0, bot, z0, var10, var16);
                t.add_vertex_with_uv(x0, bot, z1, var12, var16);
                t.add_vertex_with_uv(xi, top, z1, var12, var14);
            }

            if catch(var2 + 1, var3, var4) {
                let xi = ((var2 + 1) as f32 - var37) as f64;
                let xe = (var2 + 1 - 0) as f64;
                t.add_vertex_with_uv(xi, top, z0, var10, var14);
                t.add_vertex_with_uv(xe, bot, z0, var10, var16);
                t.add_vertex_with_uv(xe, bot, z1, var12, var16);
                t.add_vertex_with_uv(xi, top, z1, var12, var14);
                t.add_vertex_with_uv(xi, top, z1, var12, var14);
                t.add_vertex_with_uv(xe, bot, z1, var12, var16);
                t.add_vertex_with_uv(xe, bot, z0, var10, var16);
                t.add_vertex_with_uv(xi, top, z0, var10, var14);
            }

            if catch(var2, var3, var4 - 1) {
                let zi = (zf + var37) as f64;
                t.add_vertex_with_uv(x0, top, zi, var12, var14);
                t.add_vertex_with_uv(x0, bot, z0, var12, var16);
                t.add_vertex_with_uv(x1, bot, z0, var10, var16);
                t.add_vertex_with_uv(x1, top, zi, var10, var14);
                t.add_vertex_with_uv(x1, top, zi, var10, var14);
                t.add_vertex_with_uv(x1, bot, z0, var10, var16);
                t.add_vertex_with_uv(x0, bot, z0, var12, var16);
                t.add_vertex_with_uv(x0, top, zi, var12, var14);
            }

            if catch(var2, var3, var4 + 1) {
                let zi = ((var4 + 1) as f32 - var37) as f64;
                let ze = (var4 + 1 - 0) as f64;
                t.add_vertex_with_uv(x1, top, zi, var10, var14);
                t.add_vertex_with_uv(x1, bot, ze, var10, var16);
                t.add_vertex_with_uv(x0, bot, ze, var12, var16);
                t.add_vertex_with_uv(x0, top, zi, var12, var14);
                t.add_vertex_with_uv(x0, top, zi, var12, var14);
                t.add_vertex_with_uv(x0, bot, ze, var12, var16);
                t.add_vertex_with_uv(x1, bot, ze, var10, var16);
                t.add_vertex_with_uv(x1, top, zi, var10, var14);
            }

            if catch(var2, var3 + 1, var4) {
                var21 = var2 as f64 + 0.5 + 0.5;
                var23 = var2 as f64 + 0.5 - 0.5;
                var25 = var4 as f64 + 0.5 + 0.5;
                var27 = var4 as f64 + 0.5 - 0.5;
                var29 = var2 as f64 + 0.5 - 0.5;
                var31 = var2 as f64 + 0.5 + 0.5;
                var33 = var4 as f64 + 0.5 - 0.5;
                let var35 = var4 as f64 + 0.5 + 0.5;
                (var10, var12, var14, var16) = upper();
                var3 += 1;
                var18 = -0.2;
                let low = (var3 as f32 + var18) as f64;
                let y0 = (var3 + 0) as f64;
                if (var2.wrapping_add(var3).wrapping_add(var4) & 1) == 0 {
                    t.add_vertex_with_uv(var29, low, z0, var12, var14);
                    t.add_vertex_with_uv(var21, y0, z0, var12, var16);
                    t.add_vertex_with_uv(var21, y0, z1, var10, var16);
                    t.add_vertex_with_uv(var29, low, z1, var10, var14);
                    (var10, var12, var14, var16) = lower();
                    t.add_vertex_with_uv(var31, low, z1, var12, var14);
                    t.add_vertex_with_uv(var23, y0, z1, var12, var16);
                    t.add_vertex_with_uv(var23, y0, z0, var10, var16);
                    t.add_vertex_with_uv(var31, low, z0, var10, var14);
                } else {
                    t.add_vertex_with_uv(x0, low, var35, var12, var14);
                    t.add_vertex_with_uv(x0, y0, var27, var12, var16);
                    t.add_vertex_with_uv(x1, y0, var27, var10, var16);
                    t.add_vertex_with_uv(x1, low, var35, var10, var14);
                    (var10, var12, var14, var16) = lower();
                    t.add_vertex_with_uv(x1, low, var33, var12, var14);
                    t.add_vertex_with_uv(x1, y0, var25, var12, var16);
                    t.add_vertex_with_uv(x0, y0, var25, var10, var16);
                    t.add_vertex_with_uv(x0, low, var33, var10, var14);
                }
            }
        } else {
            let mut var19 = var2 as f64 + 0.5 + 0.2;
            let mut var21 = var2 as f64 + 0.5 - 0.2;
            let mut var23 = var4 as f64 + 0.5 + 0.2;
            let mut var25 = var4 as f64 + 0.5 - 0.2;
            let mut var27 = var2 as f64 + 0.5 - 0.3;
            let mut var29 = var2 as f64 + 0.5 + 0.3;
            let mut var31 = var4 as f64 + 0.5 - 0.3;
            let mut var33 = var4 as f64 + 0.5 + 0.3;
            let top = (var3 as f32 + var18) as f64;
            let y0 = (var3 + 0) as f64;
            let (x0, x1) = ((var2 + 0) as f64, (var2 + 1) as f64);
            let (z0, z1) = ((var4 + 0) as f64, (var4 + 1) as f64);
            t.add_vertex_with_uv(var27, top, z1, var12, var14);
            t.add_vertex_with_uv(var19, y0, z1, var12, var16);
            t.add_vertex_with_uv(var19, y0, z0, var10, var16);
            t.add_vertex_with_uv(var27, top, z0, var10, var14);
            t.add_vertex_with_uv(var29, top, z0, var12, var14);
            t.add_vertex_with_uv(var21, y0, z0, var12, var16);
            t.add_vertex_with_uv(var21, y0, z1, var10, var16);
            t.add_vertex_with_uv(var29, top, z1, var10, var14);
            (var10, var12, var14, var16) = lower();
            t.add_vertex_with_uv(x1, top, var33, var12, var14);
            t.add_vertex_with_uv(x1, y0, var25, var12, var16);
            t.add_vertex_with_uv(x0, y0, var25, var10, var16);
            t.add_vertex_with_uv(x0, top, var33, var10, var14);
            t.add_vertex_with_uv(x0, top, var31, var12, var14);
            t.add_vertex_with_uv(x0, y0, var23, var12, var16);
            t.add_vertex_with_uv(x1, y0, var23, var10, var16);
            t.add_vertex_with_uv(x1, top, var31, var10, var14);
            var19 = var2 as f64 + 0.5 - 0.5;
            var21 = var2 as f64 + 0.5 + 0.5;
            var23 = var4 as f64 + 0.5 - 0.5;
            var25 = var4 as f64 + 0.5 + 0.5;
            var27 = var2 as f64 + 0.5 - 0.4;
            var29 = var2 as f64 + 0.5 + 0.4;
            var31 = var4 as f64 + 0.5 - 0.4;
            var33 = var4 as f64 + 0.5 + 0.4;
            t.add_vertex_with_uv(var27, top, z0, var10, var14);
            t.add_vertex_with_uv(var19, y0, z0, var10, var16);
            t.add_vertex_with_uv(var19, y0, z1, var12, var16);
            t.add_vertex_with_uv(var27, top, z1, var12, var14);
            t.add_vertex_with_uv(var29, top, z1, var10, var14);
            t.add_vertex_with_uv(var21, y0, z1, var10, var16);
            t.add_vertex_with_uv(var21, y0, z0, var12, var16);
            t.add_vertex_with_uv(var29, top, z0, var12, var14);
            (var10, var12, var14, var16) = upper();
            t.add_vertex_with_uv(x0, top, var33, var10, var14);
            t.add_vertex_with_uv(x0, y0, var25, var10, var16);
            t.add_vertex_with_uv(x1, y0, var25, var12, var16);
            t.add_vertex_with_uv(x1, top, var33, var12, var14);
            t.add_vertex_with_uv(x1, top, var31, var10, var14);
            t.add_vertex_with_uv(x1, y0, var23, var10, var16);
            t.add_vertex_with_uv(x0, y0, var23, var12, var16);
            t.add_vertex_with_uv(x0, top, var31, var12, var14);
        }
        true
    }

    /// `renderBlockCrops(block, x, y, z)`: render type 6, colour = the
    /// brightness, the four planes 1/16 lower than the cell (sunk into the
    /// farmland, whose top is at 15/16).
    // src: RenderBlocks.java:1339-1345
    pub fn render_block_crops(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let var6 = self.brightness(id, x, y, z);
        self.tess.set_color_opaque_f(var6, var6, var6);
        let meta = self.block_access.get_block_metadata(x, y, z);
        self.func_1245_b(id, meta, x as f64, (y as f32 - 0.0625) as f64, z as f64);
        true
    }

    /// `renderBlockCactus(block, x, y, z)`: render type 13, the colour
    /// multiplier (white; with the anaglyph transform) into
    /// [`Self::func_1230_b`].
    // src: RenderBlocks.java:2334-2349
    pub fn render_block_cactus(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let var5 = blocks::color_multiplier(self.block_access, id, x, y, z);
        let mut var6 = (var5 >> 16 & 255) as f32 / 255.0;
        let mut var7 = (var5 >> 8 & 255) as f32 / 255.0;
        let mut var8 = (var5 & 255) as f32 / 255.0;
        if self.anaglyph {
            [var6, var7, var8] = anaglyph([var6, var7, var8]);
        }
        self.func_1230_b(id, x, y, z, var6, var7, var8)
    }

    /// `func_1230_b(block, x, y, z, r, g, b)` (later MCP:
    /// `renderBlockCactusImpl`): the six faces of the block's bounds with
    /// the plain shading (0.5 bottom, 1 top, 0.8 z sides, 0.6 x sides, each
    /// × the face neighbour's brightness, or the block's own where the
    /// bounds do not reach the cell's face), the four sides drawn 1/16 in
    /// (the Tessellator translated toward the centre around each).
    // src: RenderBlocks.java:2351-2444
    #[allow(clippy::too_many_arguments)]
    pub fn func_1230_b(
        &mut self,
        id: i32,
        x: i32,
        y: i32,
        z: i32,
        var5: f32,
        var6: f32,
        var7: f32,
    ) -> bool {
        let (var2, var3, var4) = (x, y, z);
        let mut var9 = false;
        let var10 = 0.5f32;
        let var11 = 1.0f32;
        let var12 = 0.8f32;
        let var13 = 0.6f32;
        let var14 = var10 * var5;
        let var15 = var11 * var5;
        let var16 = var12 * var5;
        let var17 = var13 * var5;
        let var18 = var10 * var6;
        let var19 = var11 * var6;
        let var20 = var12 * var6;
        let var21 = var13 * var6;
        let var22 = var10 * var7;
        let var23 = var11 * var7;
        let var24 = var12 * var7;
        let var25 = var13 * var7;
        let var26 = 0.0625f32;
        let var27 = self.brightness(id, var2, var3, var4);
        let is_liquid = blocks::def(id).is_some_and(|b| b.block_material.get_is_liquid());
        let (xd, yd, zd) = (var2 as f64, var3 as f64, var4 as f64);
        let mut var28: f32;

        if self.render_all_faces || self.should_side(id, var2, var3 - 1, var4, 0) {
            var28 = self.brightness(id, var2, var3 - 1, var4);
            self.tess
                .set_color_opaque_f(var14 * var28, var18 * var28, var22 * var28);
            let t = self.block_texture(id, var2, var3, var4, 0);
            self.render_bottom_face(id, xd, yd, zd, t);
            var9 = true;
        }

        if self.render_all_faces || self.should_side(id, var2, var3 + 1, var4, 1) {
            var28 = self.brightness(id, var2, var3 + 1, var4);
            if bounds(&self.statics, id).max_y != 1.0 && !is_liquid {
                var28 = var27;
            }
            self.tess
                .set_color_opaque_f(var15 * var28, var19 * var28, var23 * var28);
            let t = self.block_texture(id, var2, var3, var4, 1);
            self.render_top_face(id, xd, yd, zd, t);
            var9 = true;
        }

        if self.render_all_faces || self.should_side(id, var2, var3, var4 - 1, 2) {
            var28 = self.brightness(id, var2, var3, var4 - 1);
            if bounds(&self.statics, id).min_z > 0.0 {
                var28 = var27;
            }
            self.tess
                .set_color_opaque_f(var16 * var28, var20 * var28, var24 * var28);
            self.tess.set_translation_f(0.0, 0.0, var26);
            let t = self.block_texture(id, var2, var3, var4, 2);
            self.render_east_face(id, xd, yd, zd, t);
            self.tess.set_translation_f(0.0, 0.0, -var26);
            var9 = true;
        }

        if self.render_all_faces || self.should_side(id, var2, var3, var4 + 1, 3) {
            var28 = self.brightness(id, var2, var3, var4 + 1);
            if bounds(&self.statics, id).max_z < 1.0 {
                var28 = var27;
            }
            self.tess
                .set_color_opaque_f(var16 * var28, var20 * var28, var24 * var28);
            self.tess.set_translation_f(0.0, 0.0, -var26);
            let t = self.block_texture(id, var2, var3, var4, 3);
            self.render_west_face(id, xd, yd, zd, t);
            self.tess.set_translation_f(0.0, 0.0, var26);
            var9 = true;
        }

        if self.render_all_faces || self.should_side(id, var2 - 1, var3, var4, 4) {
            var28 = self.brightness(id, var2 - 1, var3, var4);
            if bounds(&self.statics, id).min_x > 0.0 {
                var28 = var27;
            }
            self.tess
                .set_color_opaque_f(var17 * var28, var21 * var28, var25 * var28);
            self.tess.set_translation_f(var26, 0.0, 0.0);
            let t = self.block_texture(id, var2, var3, var4, 4);
            self.render_north_face(id, xd, yd, zd, t);
            self.tess.set_translation_f(-var26, 0.0, 0.0);
            var9 = true;
        }

        if self.render_all_faces || self.should_side(id, var2 + 1, var3, var4, 5) {
            var28 = self.brightness(id, var2 + 1, var3, var4);
            if bounds(&self.statics, id).max_x < 1.0 {
                var28 = var27;
            }
            self.tess
                .set_color_opaque_f(var17 * var28, var21 * var28, var25 * var28);
            self.tess.set_translation_f(-var26, 0.0, 0.0);
            let t = self.block_texture(id, var2, var3, var4, 5);
            self.render_south_face(id, xd, yd, zd, t);
            self.tess.set_translation_f(var26, 0.0, 0.0);
            var9 = true;
        }

        var9
    }

    /// `renderTorchAtAngle(block, x, y, z, dx, dz)` (also the inventory's
    /// torch): the 2-pixel-wide stick as four side planes 1/16 off the
    /// centre whose bottoms are moved by `(dx, dz)`, and a top cap 0.625 up
    /// (UVs: the texture's pixels 7-9 × 6-8, the flame's tip), moved by
    /// `(dx, dz) × 0.375`. No colour is set here.
    // src: RenderBlocks.java:1347-1392
    pub fn render_torch_at_angle(&mut self, id: i32, x: f64, y: f64, z: f64, dx: f64, dz: f64) {
        let (mut var2, var4, mut var6, var8, var10) = (x, y, z, dx, dz);
        let mut var13 = blocks::get_block_texture_from_side(&self.statics, id, 0);
        if self.override_block_texture >= 0 {
            var13 = self.override_block_texture;
        }
        let var14 = (var13 & 15) << 4;
        let var15 = var13 & 240;
        let var16 = var14 as f32 / 256.0;
        let var17 = (var14 as f32 + 15.99) / 256.0;
        let var18 = var15 as f32 / 256.0;
        let var19 = (var15 as f32 + 15.99) / 256.0;
        let var20 = var16 as f64 + 0.02734375;
        let var22 = var18 as f64 + 0.0234375;
        let var24 = var16 as f64 + 0.03515625;
        let var26 = var18 as f64 + 0.03125;
        var2 += 0.5;
        var6 += 0.5;
        let var28 = var2 - 0.5;
        let var30 = var2 + 0.5;
        let var32 = var6 - 0.5;
        let var34 = var6 + 0.5;
        let var36 = 0.0625f64;
        let var38 = 0.625f64;
        let (u0, u1, v0, v1) = (var16 as f64, var17 as f64, var18 as f64, var19 as f64);
        let t = &mut self.tess;
        // The cap.
        let cx = var2 + var8 * (1.0 - var38);
        let cz = var6 + var10 * (1.0 - var38);
        t.add_vertex_with_uv(cx - var36, var4 + var38, cz - var36, var20, var22);
        t.add_vertex_with_uv(cx - var36, var4 + var38, cz + var36, var20, var26);
        t.add_vertex_with_uv(cx + var36, var4 + var38, cz + var36, var24, var26);
        t.add_vertex_with_uv(cx + var36, var4 + var38, cz - var36, var24, var22);
        // The four sides.
        t.add_vertex_with_uv(var2 - var36, var4 + 1.0, var32, u0, v0);
        t.add_vertex_with_uv(var2 - var36 + var8, var4 + 0.0, var32 + var10, u0, v1);
        t.add_vertex_with_uv(var2 - var36 + var8, var4 + 0.0, var34 + var10, u1, v1);
        t.add_vertex_with_uv(var2 - var36, var4 + 1.0, var34, u1, v0);
        t.add_vertex_with_uv(var2 + var36, var4 + 1.0, var34, u0, v0);
        t.add_vertex_with_uv(var2 + var8 + var36, var4 + 0.0, var34 + var10, u0, v1);
        t.add_vertex_with_uv(var2 + var8 + var36, var4 + 0.0, var32 + var10, u1, v1);
        t.add_vertex_with_uv(var2 + var36, var4 + 1.0, var32, u1, v0);
        t.add_vertex_with_uv(var28, var4 + 1.0, var6 + var36, u0, v0);
        t.add_vertex_with_uv(var28 + var8, var4 + 0.0, var6 + var36 + var10, u0, v1);
        t.add_vertex_with_uv(var30 + var8, var4 + 0.0, var6 + var36 + var10, u1, v1);
        t.add_vertex_with_uv(var30, var4 + 1.0, var6 + var36, u1, v0);
        t.add_vertex_with_uv(var30, var4 + 1.0, var6 - var36, u0, v0);
        t.add_vertex_with_uv(var30 + var8, var4 + 0.0, var6 - var36 + var10, u0, v1);
        t.add_vertex_with_uv(var28 + var8, var4 + 0.0, var6 - var36 + var10, u1, v1);
        t.add_vertex_with_uv(var28, var4 + 1.0, var6 - var36, u1, v0);
    }

    /// `renderCrossedSquares(block, meta, x, y, z)` (also the inventory's
    /// type 1): two diagonal planes from corner to corner of the square
    /// 0.45 around the centre, each drawn from both sides, one block high,
    /// with `getBlockTextureFromSideAndMetadata(0, meta)`. No colour is set
    /// here.
    // src: RenderBlocks.java:1394-1427
    pub fn render_crossed_squares(&mut self, id: i32, meta: i32, x: f64, y: f64, z: f64) {
        let (var3, var5, var7) = (x, y, z);
        let mut var10 =
            blocks::get_block_texture_from_side_and_metadata(&self.statics, id, 0, meta);
        if self.override_block_texture >= 0 {
            var10 = self.override_block_texture;
        }
        let (var13, var15, var17, var19) = tile_rect(var10);
        // `0.45F` widened.
        let var21 = var3 + 0.5 - 0.44999998807907104;
        let var23 = var3 + 0.5 + 0.44999998807907104;
        let var25 = var7 + 0.5 - 0.44999998807907104;
        let var27 = var7 + 0.5 + 0.44999998807907104;
        let t = &mut self.tess;
        t.add_vertex_with_uv(var21, var5 + 1.0, var25, var13, var17);
        t.add_vertex_with_uv(var21, var5 + 0.0, var25, var13, var19);
        t.add_vertex_with_uv(var23, var5 + 0.0, var27, var15, var19);
        t.add_vertex_with_uv(var23, var5 + 1.0, var27, var15, var17);
        t.add_vertex_with_uv(var23, var5 + 1.0, var27, var13, var17);
        t.add_vertex_with_uv(var23, var5 + 0.0, var27, var13, var19);
        t.add_vertex_with_uv(var21, var5 + 0.0, var25, var15, var19);
        t.add_vertex_with_uv(var21, var5 + 1.0, var25, var15, var17);
        t.add_vertex_with_uv(var21, var5 + 1.0, var27, var13, var17);
        t.add_vertex_with_uv(var21, var5 + 0.0, var27, var13, var19);
        t.add_vertex_with_uv(var23, var5 + 0.0, var25, var15, var19);
        t.add_vertex_with_uv(var23, var5 + 1.0, var25, var15, var17);
        t.add_vertex_with_uv(var23, var5 + 1.0, var25, var13, var17);
        t.add_vertex_with_uv(var23, var5 + 0.0, var25, var13, var19);
        t.add_vertex_with_uv(var21, var5 + 0.0, var27, var15, var19);
        t.add_vertex_with_uv(var21, var5 + 1.0, var27, var15, var17);
    }

    /// `func_1245_b(block, meta, x, y, z)` (later MCP:
    /// `renderBlockCropsImpl`; also the inventory's type 6): four planes
    /// in a `#`, a quarter block off the centre on each axis, each drawn
    /// from both sides, with `getBlockTextureFromSideAndMetadata(0, meta)`.
    /// No colour is set here.
    // src: RenderBlocks.java:1429-1482
    pub fn func_1245_b(&mut self, id: i32, meta: i32, x: f64, y: f64, z: f64) {
        let (var3, var5, var7) = (x, y, z);
        let mut var10 =
            blocks::get_block_texture_from_side_and_metadata(&self.statics, id, 0, meta);
        if self.override_block_texture >= 0 {
            var10 = self.override_block_texture;
        }
        let (var13, var15, var17, var19) = tile_rect(var10);
        let mut var21 = var3 + 0.5 - 0.25;
        let mut var23 = var3 + 0.5 + 0.25;
        let mut var25 = var7 + 0.5 - 0.5;
        let mut var27 = var7 + 0.5 + 0.5;
        let t = &mut self.tess;
        t.add_vertex_with_uv(var21, var5 + 1.0, var25, var13, var17);
        t.add_vertex_with_uv(var21, var5 + 0.0, var25, var13, var19);
        t.add_vertex_with_uv(var21, var5 + 0.0, var27, var15, var19);
        t.add_vertex_with_uv(var21, var5 + 1.0, var27, var15, var17);
        t.add_vertex_with_uv(var21, var5 + 1.0, var27, var13, var17);
        t.add_vertex_with_uv(var21, var5 + 0.0, var27, var13, var19);
        t.add_vertex_with_uv(var21, var5 + 0.0, var25, var15, var19);
        t.add_vertex_with_uv(var21, var5 + 1.0, var25, var15, var17);
        t.add_vertex_with_uv(var23, var5 + 1.0, var27, var13, var17);
        t.add_vertex_with_uv(var23, var5 + 0.0, var27, var13, var19);
        t.add_vertex_with_uv(var23, var5 + 0.0, var25, var15, var19);
        t.add_vertex_with_uv(var23, var5 + 1.0, var25, var15, var17);
        t.add_vertex_with_uv(var23, var5 + 1.0, var25, var13, var17);
        t.add_vertex_with_uv(var23, var5 + 0.0, var25, var13, var19);
        t.add_vertex_with_uv(var23, var5 + 0.0, var27, var15, var19);
        t.add_vertex_with_uv(var23, var5 + 1.0, var27, var15, var17);
        var21 = var3 + 0.5 - 0.5;
        var23 = var3 + 0.5 + 0.5;
        var25 = var7 + 0.5 - 0.25;
        var27 = var7 + 0.5 + 0.25;
        t.add_vertex_with_uv(var21, var5 + 1.0, var25, var13, var17);
        t.add_vertex_with_uv(var21, var5 + 0.0, var25, var13, var19);
        t.add_vertex_with_uv(var23, var5 + 0.0, var25, var15, var19);
        t.add_vertex_with_uv(var23, var5 + 1.0, var25, var15, var17);
        t.add_vertex_with_uv(var23, var5 + 1.0, var25, var13, var17);
        t.add_vertex_with_uv(var23, var5 + 0.0, var25, var13, var19);
        t.add_vertex_with_uv(var21, var5 + 0.0, var25, var15, var19);
        t.add_vertex_with_uv(var21, var5 + 1.0, var25, var15, var17);
        t.add_vertex_with_uv(var23, var5 + 1.0, var27, var13, var17);
        t.add_vertex_with_uv(var23, var5 + 0.0, var27, var13, var19);
        t.add_vertex_with_uv(var21, var5 + 0.0, var27, var15, var19);
        t.add_vertex_with_uv(var21, var5 + 1.0, var27, var15, var17);
        t.add_vertex_with_uv(var21, var5 + 1.0, var27, var13, var17);
        t.add_vertex_with_uv(var21, var5 + 0.0, var27, var13, var19);
        t.add_vertex_with_uv(var23, var5 + 0.0, var27, var15, var19);
        t.add_vertex_with_uv(var23, var5 + 1.0, var27, var15, var17);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::tessellator::Tessellator;
    use mc_sim::world::provider::{Dimension, WorldProvider};

    const SAPLING: i32 = 6;
    const STONE: i32 = 1;
    const PLANKS: i32 = 5;
    const TALL_GRASS: i32 = 31;
    const FLOWER: i32 = 37;
    const TORCH: i32 = 50;
    const FIRE: i32 = 51;
    const CROPS: i32 = 59;
    const REDSTONE_TORCH_OFF: i32 = 75;
    const CACTUS: i32 = 81;
    const REED: i32 = 83;

    fn snap() -> ChunkSnapshot {
        ChunkSnapshot::synthetic(
            [-3, -1, -3],
            [4, 6, 4],
            WorldProvider::new(Dimension::Surface).light_brightness_table,
        )
    }

    fn rb(s: &ChunkSnapshot) -> RenderBlocks<'_> {
        RenderBlocks::new(s, s.statics.clone(), false, false)
    }

    /// The four Java UV values of a full tile (`(float)x / 256F` and
    /// `((float)x + 15.99F) / 256F`), written out again.
    fn rect(i: i32) -> (f32, f32, f32, f32) {
        let x = ((i & 15) << 4) as f32;
        let y = (i & 240) as f32;
        (
            x / 256.0,
            (x + 15.99) / 256.0,
            y / 256.0,
            (y + 15.99) / 256.0,
        )
    }

    fn quad_centres(t: &Tessellator) -> Vec<[f32; 3]> {
        t.positions
            .chunks(4)
            .map(|q| {
                let mut c = [0.0f32; 3];
                for p in q {
                    for i in 0..3 {
                        c[i] += p[i] / 4.0;
                    }
                }
                c
            })
            .collect()
    }

    fn close(a: [f32; 3], b: [f64; 3]) -> bool {
        (0..3).all(|i| (a[i] as f64 - b[i]).abs() < 1e-5)
    }

    #[test]
    fn crossed_squares_at_the_origin() {
        let mut s = snap();
        s.set(0, 0, 0, FLOWER, 0, 15, 0);
        let mut r = rb(&s);
        assert!(r.render_block_by_render_type(FLOWER, 0, 0, 0));
        let (u0, u1, v0, v1) = rect(13);
        let a = (0.5 - 0.44999998807907104f64) as f32;
        let b = (0.5 + 0.44999998807907104f64) as f32;
        assert_eq!(
            r.tess.positions,
            vec![
                [a, 1.0, a],
                [a, 0.0, a],
                [b, 0.0, b],
                [b, 1.0, b],
                [b, 1.0, b],
                [b, 0.0, b],
                [a, 0.0, a],
                [a, 1.0, a],
                [a, 1.0, b],
                [a, 0.0, b],
                [b, 0.0, a],
                [b, 1.0, a],
                [b, 1.0, a],
                [b, 0.0, a],
                [a, 0.0, b],
                [a, 1.0, b],
            ]
        );
        let uv: Vec<[f32; 2]> = [[u0, v0], [u0, v1], [u1, v1], [u1, v0]].repeat(4);
        assert_eq!(r.tess.uvs, uv);
        // white × full sky light
        assert!(r.tess.colors.iter().all(|&c| c == [255; 4]));
    }

    #[test]
    fn sapling_texture_by_metadata_and_reed() {
        for (meta, tex) in [(0, 15), (1, 63), (2, 79), (3, 15), (9, 63)] {
            let mut s = snap();
            s.set(0, 0, 0, SAPLING, meta, 15, 0);
            let mut r = rb(&s);
            r.render_block_by_render_type(SAPLING, 0, 0, 0);
            let (u0, _, v0, _) = rect(tex);
            assert_eq!(r.tess.uvs[0], [u0, v0], "meta {meta}");
        }
        let mut s = snap();
        s.set(0, 0, 0, REED, 7, 15, 0);
        let mut r = rb(&s);
        r.render_block_by_render_type(REED, 0, 0, 0);
        let (u0, _, v0, _) = rect(73);
        assert_eq!(r.tess.uvs[0], [u0, v0]);
        assert_eq!(r.tess.len(), 16);
    }

    #[test]
    fn tall_grass_hash_offset() {
        // Expected values from the Java expressions evaluated with Java's
        // long wrapping and float rounding (computed independently).
        assert_eq!(tall_grass_offset(0, 0, 0), (-0.25, -0.2, -0.25));
        let (dx, dy, dz) = tall_grass_offset(5, 64, -7);
        assert_eq!(dx, 0.0833333432674408);
        assert_eq!(dy, -0.13333333134651185);
        assert_eq!(dz, -0.14999999850988388);
        let (dx, dy, dz) = tall_grass_offset(-1234, 70, 98765);
        assert_eq!(dx, 0.21666666865348816);
        assert_eq!(dy, -0.026666665077209474);
        assert_eq!(dz, 0.15000000596046448);

        // the drawn planes move by it; meta 0 (the dead shrub) is white and
        // textured 39 + 16
        let mut s = snap();
        s.set(1, 2, -2, TALL_GRASS, 0, 15, 0);
        let mut r = rb(&s);
        r.render_block_by_render_type(TALL_GRASS, 1, 2, -2);
        let (dx, dy, dz) = tall_grass_offset(1, 2, -2);
        let p = r.tess.positions[0];
        assert_eq!(p[0], (1.0 + dx + 0.5 - 0.44999998807907104) as f32);
        assert_eq!(p[1], (2.0 + dy + 1.0) as f32);
        assert_eq!(p[2], (-2.0 + dz + 0.5 - 0.44999998807907104) as f32);
        let (u0, _, v0, _) = rect(55);
        assert_eq!(r.tess.uvs[0], [u0, v0]);
        assert_eq!(r.tess.colors[0], [255; 4]);
        // a flower at the same place is not moved
        let mut s = snap();
        s.set(1, 2, -2, FLOWER, 0, 15, 0);
        let mut r = rb(&s);
        r.render_block_by_render_type(FLOWER, 1, 2, -2);
        assert_eq!(
            r.tess.positions[0],
            [
                (1.5 - 0.44999998807907104) as f32,
                3.0,
                (-1.5 - 0.44999998807907104) as f32
            ]
        );
    }

    #[test]
    fn torch_cap_centre_per_metadata() {
        // metadata → (x, y, z) of the cap's centre, the block at the origin
        // src: RenderBlocks.java:283-298, :1366-1372
        let lean = 0.5 - 0.4000000059604645;
        let tilt = 0.4000000059604645 * (1.0 - 0.625);
        for (meta, centre) in [
            (0, [0.5, 0.625, 0.5]),
            (1, [0.5 - lean - tilt, 0.20000000298023224 + 0.625, 0.5]),
            (2, [0.5 + lean + tilt, 0.20000000298023224 + 0.625, 0.5]),
            (3, [0.5, 0.20000000298023224 + 0.625, 0.5 - lean - tilt]),
            (4, [0.5, 0.20000000298023224 + 0.625, 0.5 + lean + tilt]),
            (5, [0.5, 0.625, 0.5]),
        ] {
            let mut s = snap();
            s.set(0, 0, 0, TORCH, meta, 0, 0);
            let mut r = rb(&s);
            assert!(r.render_block_by_render_type(TORCH, 0, 0, 0));
            assert_eq!(r.tess.len(), 20);
            let c = quad_centres(&r.tess);
            assert!(close(c[0], centre), "meta {meta}: {:?}", c[0]);
            // the cap is flat, the stick's bottom moved by (dx, dz)
            assert!(
                r.tess.positions[..4]
                    .iter()
                    .all(|p| p[1] == r.tess.positions[0][1])
            );
            // a lit torch is drawn at full brightness in the dark
            assert_eq!(r.tess.colors[0], [255; 4]);
        }
        // the cap's UVs: pixels 7..9 × 6..8 of tile 80
        let mut s = snap();
        s.set(0, 0, 0, TORCH, 5, 0, 0);
        let mut r = rb(&s);
        r.render_block_by_render_type(TORCH, 0, 0, 0);
        let (u0, _, v0, _) = rect(80);
        assert_eq!(
            r.tess.uvs[0],
            [
                (u0 as f64 + 0.02734375) as f32,
                (v0 as f64 + 0.0234375) as f32
            ]
        );
        assert_eq!(
            r.tess.uvs[2],
            [
                (u0 as f64 + 0.03515625) as f32,
                (v0 as f64 + 0.03125) as f32
            ]
        );
        // an unlit redstone torch takes the cell's brightness
        let mut s = snap();
        s.set(0, 0, 0, REDSTONE_TORCH_OFF, 5, 0, 0);
        let mut r = rb(&s);
        r.render_block_by_render_type(REDSTONE_TORCH_OFF, 0, 0, 0);
        let b = WorldProvider::new(Dimension::Surface).light_brightness_table[0];
        let c = (b * 255.0) as u8;
        assert_eq!(r.tess.colors[0], [c, c, c, 255]);
    }

    #[test]
    fn fire_against_one_flammable_side() {
        // fire at (0, 1, 0) over air, planks at x - 1 only: one leaning flame
        // drawn from both sides at the west edge, nothing else
        let mut s = snap();
        s.set(0, 1, 0, FIRE, 0, 15, 0);
        s.set(-1, 1, 0, PLANKS, 0, 15, 0);
        let mut r = rb(&s);
        assert!(r.render_block_by_render_type(FIRE, 0, 1, 0));
        assert_eq!(r.tess.len(), 8);
        let top = (1.0f32 + 1.4 + 0.0625) as f64 as f32;
        let bot = 1.0625f32;
        for p in &r.tess.positions {
            assert!(p[0] == 0.2 || p[0] == 0.0, "{p:?}");
            assert!(p[1] == top || p[1] == bot, "{p:?}");
            assert_eq!(p[0] == 0.2, p[1] == top);
        }
        // (0 + 1 + 0) & 1 == 1: the second frame (tile 47); 0/2 + 1/2 + 0/2
        // even: u not mirrored
        let (u0, u1, _, _) = rect(31);
        let v0 = (31 & 240) as f32 + 16.0;
        assert_eq!(r.tess.uvs[0], [u1, v0 / 256.0]);
        assert_eq!(r.tess.uvs[3], [u0, v0 / 256.0]);

        // the same with the planks on the other three sides each
        for (dx, dz, edge) in [(1, 0, 0usize), (0, -1, 2), (0, 1, 2)] {
            let mut s = snap();
            s.set(0, 1, 0, FIRE, 0, 15, 0);
            s.set(dx, 1, dz, PLANKS, 0, 15, 0);
            let mut r = rb(&s);
            r.render_block_by_render_type(FIRE, 0, 1, 0);
            assert_eq!(r.tess.len(), 8, "({dx}, {dz})");
            let want = if dx + dz > 0 { 0.8f32 } else { 0.2 };
            assert!(
                r.tess.positions.iter().any(|p| p[edge] == want),
                "({dx}, {dz})"
            );
        }

        // nothing flammable, nothing solid below: nothing drawn (the Java
        // still returns true)
        let mut s = snap();
        s.set(0, 1, 0, FIRE, 0, 15, 0);
        let mut r = rb(&s);
        assert!(r.render_block_by_render_type(FIRE, 0, 1, 0));
        assert!(r.tess.is_empty());

        // under a flammable block: the two hanging planes
        s.set(0, 2, 0, PLANKS, 0, 15, 0);
        let mut r = rb(&s);
        r.render_block_by_render_type(FIRE, 0, 1, 0);
        assert_eq!(r.tess.len(), 8);
        assert!(
            r.tess
                .positions
                .iter()
                .all(|p| p[1] == 2.0 || p[1] == (2.0f32 - 0.2))
        );
    }

    #[test]
    fn fire_on_the_ground() {
        // over stone: the four slanted planes, 1.4 high
        let mut s = snap();
        s.set(0, 0, 0, STONE, 0, 15, 0);
        s.set(0, 1, 0, FIRE, 0, 15, 0);
        let mut r = rb(&s);
        r.render_block_by_render_type(FIRE, 0, 1, 0);
        assert_eq!(r.tess.len(), 32);
        let top = (1.0f32 + 1.4) as f64 as f32;
        assert!(r.tess.positions.iter().all(|p| p[1] == 1.0 || p[1] == top));
        // the first quad: from x 0.2 at the top to 0.7 at the bottom
        assert_eq!(r.tess.positions[0], [(0.5f64 - 0.3) as f32, top, 1.0]);
        assert_eq!(r.tess.positions[1], [(0.5f64 + 0.2) as f32, 1.0, 1.0]);
        // first frame, then the second, the second, the first
        let v_first = rect(31).2;
        let v_second = rect(47).2;
        let vs: Vec<f32> = (0..4).map(|q| r.tess.uvs[q * 8][1]).collect();
        assert_eq!(vs, vec![v_first, v_second, v_second, v_first]);
        // a flammable block below counts too
        let mut s = snap();
        s.set(0, 0, 0, PLANKS, 0, 15, 0);
        s.set(0, 1, 0, FIRE, 0, 15, 0);
        let mut r = rb(&s);
        r.render_block_by_render_type(FIRE, 0, 1, 0);
        assert_eq!(r.tess.len(), 32);
    }

    #[test]
    fn crops_planes_and_textures() {
        for (meta, tex) in [(0, 88), (7, 95)] {
            let mut s = snap();
            s.set(0, 1, 0, CROPS, meta, 15, 0);
            let mut r = rb(&s);
            assert!(r.render_block_by_render_type(CROPS, 0, 1, 0));
            assert_eq!(r.tess.len(), 32);
            let (u0, u1, v0, v1) = rect(tex);
            let uv: Vec<[f32; 2]> = [[u0, v0], [u0, v1], [u1, v1], [u1, v0]].repeat(8);
            assert_eq!(r.tess.uvs, uv, "meta {meta}");
            // 1/16 down
            assert_eq!(r.tess.positions[0], [0.25, 1.9375, 0.0]);
            assert_eq!(r.tess.positions[1], [0.25, 0.9375, 0.0]);
            // the planes: x = 0.25, 0.75, then z = 0.25, 0.75
            let c = quad_centres(&r.tess);
            assert!(close(c[0], [0.25, 1.4375, 0.5]));
            assert!(close(c[2], [0.75, 1.4375, 0.5]));
            assert!(close(c[4], [0.5, 1.4375, 0.25]));
            assert!(close(c[6], [0.5, 1.4375, 0.75]));
        }
    }

    #[test]
    fn cactus_sides_inset() {
        let mut s = snap();
        s.set(0, 1, 0, CACTUS, 0, 15, 0);
        let mut r = rb(&s);
        assert!(r.render_block_by_render_type(CACTUS, 0, 1, 0));
        assert_eq!(r.tess.len(), 24);
        let c = quad_centres(&r.tess);
        // bottom, top, east (z-), west (z+), north (x-), south (x+)
        assert!(close(c[0], [0.5, 1.0, 0.5]));
        assert!(close(c[1], [0.5, 2.0, 0.5]));
        assert!(close(c[2], [0.5, 1.5, 0.0625]));
        assert!(close(c[3], [0.5, 1.5, 0.9375]));
        assert!(close(c[4], [0.0625, 1.5, 0.5]));
        assert!(close(c[5], [0.9375, 1.5, 0.5]));
        // textures: bottom 71, top 69, sides 70
        for (q, tex) in [(0, 71), (1, 69), (2, 70), (5, 70)] {
            let (u0, u1, v0, v1) = rect(tex);
            let uv = r.tess.uvs[q * 4];
            assert!(uv[0] >= u0 - 1e-6 && uv[0] <= u1 + 1e-4, "{q}");
            assert!(uv[1] >= v0 - 1e-6 && uv[1] <= v1 + 1e-4, "{q}");
        }
        // shading 0.5, 1, 0.8, 0.8, 0.6, 0.6 under full light
        let shade: Vec<u8> = (0..6).map(|q| r.tess.colors[q * 4][0]).collect();
        assert_eq!(shade, vec![127, 255, 204, 204, 153, 153]);
        // the translation is restored
        assert_eq!(r.tess.offset, [0.0; 3]);
        // a cactus on a cactus draws the shared faces (not an opaque cube);
        // stone below hides the bottom
        s.set(0, 2, 0, CACTUS, 0, 15, 0);
        let mut r = rb(&s);
        r.render_block_by_render_type(CACTUS, 0, 1, 0);
        assert_eq!(r.tess.len(), 24, "cactus is not an opaque cube");
        s.set(0, 0, 0, STONE, 0, 15, 0);
        let mut r = rb(&s);
        r.render_block_by_render_type(CACTUS, 0, 1, 0);
        assert_eq!(r.tess.len(), 20);
    }

    #[test]
    fn inventory_types_1_2_6() {
        let colorizers = crate::textures::Colorizers::default();
        let statics = mc_sim::block::BlockStatics::default();
        let render = |id, meta| {
            super::super::render_blocks::render_block_as_item(&statics, &colorizers, id, meta, 1.0)
                .expect("drawn")
        };
        let d = render(FLOWER, 0);
        assert_eq!(d.tess.len(), 16);
        let a = (-0.5 + 0.5 - 0.44999998807907104f64) as f32;
        assert_eq!(d.tess.positions[0], [a, 0.5, a]);
        let d = render(TORCH, 0);
        assert_eq!(d.tess.len(), 20);
        assert!(close(quad_centres(&d.tess)[0], [0.0, 0.125, 0.0]));
        let d = render(CROPS, 7);
        assert_eq!(d.tess.len(), 32);
        assert_eq!(d.tess.uvs[0], [rect(95).0, rect(95).2]);
        assert_eq!(d.tess.positions[0], [-0.25, 0.5, -0.5]);
    }
}
