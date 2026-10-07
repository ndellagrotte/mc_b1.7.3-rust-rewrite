//! WP-3.3b: `RenderBlocks.renderBlockFluids` (render type 4: water and
//! lava) and its helper `func_1224_a` (the fluid height at a corner)
//! (RenderBlocks.java:1484-1663), with the `BlockFluid` statics it calls
//! on the `IBlockAccess`: `getFlowDirection` and the private
//! `getFlowVector` / `getEffectiveFlowDecay` / `getIsBlockSolid` it runs on
//! the `Block.waterMoving` / `Block.lavaMoving` singleton
//! (BlockFluid.java:35-46, :60-69, :98-178, :226-237).
//!
//! `mc-sim`'s `block::fluid` has the same methods on `&mut World`; the
//! renderer reads a [`ChunkSnapshot`] instead, so they are transcribed again
//! here (they only read). `getPercentAir` (a pure static) is `mc-sim`'s.
//! The other `Block` methods come from [`super::blocks`]:
//! `shouldSideBeRendered` (`BlockFluid`'s: hidden against its own material
//! and ice, the top always drawn otherwise), `getBlockBrightness` (the
//! brighter of the cell and the one above), `colorMultiplier` (white),
//! `getBlockTextureFromSide(AndMetadata)` (`BlockFluid.getBlockTextureFromSide`:
//! the still tile 205/237 for the top and bottom, the flowing tile 206/238
//! for the sides).
//!
//! The faces:
//!
//! - **Top** (when `shouldSideBeRendered(…, 1)` or `renderAllFaces`): its
//!   four corners at `func_1224_a`'s heights; still (no horizontal flow,
//!   angle -1000) the still tile 205/237 unrotated, else the flowing tile
//!   206/238, whose 32×32 animation (tiles 206, 207, 222, 223) is sampled as
//!   a 16-pixel square around its centre rotated by the flow angle, so the
//!   texture runs downstream.
//! - **Bottom**: `renderBottomFace` with the still tile, shade 0.5.
//! - **Sides**: from the cell's floor up to the two top corners of that
//!   side, the flowing tile cut at the same heights; shade 0.8 (z faces) or
//!   0.6 (x faces), each with the brightness of the neighbour it faces.
//!
//! The colour multiplier is white and there is no anaglyph transform or AO
//! here (vanilla's `renderBlockFluids` has neither). Water is pass 1
//! (blended, not culled: EntityRenderer.java:515-518, so its surface is seen
//! from below too), lava pass 0 (opaque, back faces culled).
//! `renderBlockOnInventory` has no fluid case (render type 4 draws nothing
//! there, RenderBlocks.java:3187-3196).

use mc_core::{Vec3D, jmath, math};
use mc_sim::block::Material;
use mc_sim::block::fluid::get_percent_air;

use super::blocks;
use super::render_blocks::RenderBlocks;
use super::snapshot::ChunkSnapshot;

impl RenderBlocks<'_> {
    /// `renderBlockFluids(block, x, y, z)`: true when a face was drawn.
    // src: RenderBlocks.java:1484-1631
    #[allow(clippy::identity_op)] // the Java's `var2 + 0`, kept
    pub fn render_block_fluids(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let (var2, var3, var4) = (x, y, z);
        // src: RenderBlocks.java:1486-1489
        let var6 = blocks::color_multiplier(self.block_access, id, var2, var3, var4);
        let var7 = (var6 >> 16 & 255) as f32 / 255.0;
        let var8 = (var6 >> 8 & 255) as f32 / 255.0;
        let var9 = (var6 & 255) as f32 / 255.0;
        // src: RenderBlocks.java:1490-1492 (top, bottom, then -z, +z, -x, +x)
        let var10 = self.should_side(id, var2, var3 + 1, var4, 1);
        let var11 = self.should_side(id, var2, var3 - 1, var4, 0);
        let var12 = [
            self.should_side(id, var2, var3, var4 - 1, 2),
            self.should_side(id, var2, var3, var4 + 1, 3),
            self.should_side(id, var2 - 1, var3, var4, 4),
            self.should_side(id, var2 + 1, var3, var4, 5),
        ];
        if !var10 && !var11 && !var12[0] && !var12[1] && !var12[2] && !var12[3] {
            return false;
        }
        let mut var13 = false;
        // src: RenderBlocks.java:1497-1502
        let var14 = 0.5f32;
        let var15 = 1.0f32;
        let var16 = 0.8f32;
        let var17 = 0.6f32;
        let var18 = 0.0f64;
        let var20 = 1.0f64;
        // src: RenderBlocks.java:1503-1508 (the corners (x, z), (x, z+1),
        // (x+1, z+1), (x+1, z))
        let var22 = blocks::def(id).map_or(Material::Air, |b| b.block_material);
        let var23 = self.block_access.get_block_metadata(var2, var3, var4);
        let var24 = self.func_1224_a(var2, var3, var4, var22);
        let var25 = self.func_1224_a(var2, var3, var4 + 1, var22);
        let var26 = self.func_1224_a(var2 + 1, var3, var4 + 1, var22);
        let var27 = self.func_1224_a(var2 + 1, var3, var4, var22);
        let tex = |s: &Self, side: i32| {
            blocks::get_block_texture_from_side_and_metadata(&s.statics, id, side, var23)
        };

        // src: RenderBlocks.java:1514-1541 (the top face)
        if self.render_all_faces || var10 {
            var13 = true;
            let mut var28 = tex(self, 1);
            // `(float)BlockFluid.getFlowDirection(…)`; render type 4 is
            // only water and lava, so the Java's NullPointerException for
            // another material cannot happen (-1000 there: the still face).
            let mut var29 = get_flow_direction(self.block_access, var2, var3, var4, var22)
                .unwrap_or(-1000.0) as f32;
            if var29 > -999.0 {
                var28 = tex(self, 2);
            }
            let var30 = (var28 & 15) << 4;
            let var31 = var28 & 240;
            // src: RenderBlocks.java:1524-1531: the still tile's centre in
            // doubles; the flowing one's (the middle of the 32×32 tile) in
            // floats
            let mut var32 = (var30 as f64 + 8.0) / 256.0;
            let mut var34 = (var31 as f64 + 8.0) / 256.0;
            if var29 < -999.0 {
                var29 = 0.0;
            } else {
                var32 = ((var30 + 16) as f32 / 256.0) as f64;
                var34 = ((var31 + 16) as f32 / 256.0) as f64;
            }
            // src: RenderBlocks.java:1533-1534 (MathHelper's table)
            let var36 = math::sin(var29) * 8.0 / 256.0;
            let var37 = math::cos(var29) * 8.0 / 256.0;
            let var38 = self.brightness(id, var2, var3, var4);
            self.tess.set_color_opaque_f(
                var15 * var38 * var7,
                var15 * var38 * var8,
                var15 * var38 * var9,
            );
            let (s, c) = (var36 as f64, var37 as f64);
            // src: RenderBlocks.java:1537-1540
            self.tess.add_vertex_with_uv(
                (var2 + 0) as f64,
                (var3 as f32 + var24) as f64,
                (var4 + 0) as f64,
                var32 - c - s,
                var34 - c + s,
            );
            self.tess.add_vertex_with_uv(
                (var2 + 0) as f64,
                (var3 as f32 + var25) as f64,
                (var4 + 1) as f64,
                var32 - c + s,
                var34 + c + s,
            );
            self.tess.add_vertex_with_uv(
                (var2 + 1) as f64,
                (var3 as f32 + var26) as f64,
                (var4 + 1) as f64,
                var32 + c + s,
                var34 + c - s,
            );
            self.tess.add_vertex_with_uv(
                (var2 + 1) as f64,
                (var3 as f32 + var27) as f64,
                (var4 + 0) as f64,
                var32 + c - s,
                var34 - c - s,
            );
        }

        // src: RenderBlocks.java:1543-1548 (the bottom face)
        if self.render_all_faces || var11 {
            let var52 = self.brightness(id, var2, var3 - 1, var4);
            self.tess
                .set_color_opaque_f(var14 * var52, var14 * var52, var14 * var52);
            let t = blocks::get_block_texture_from_side(&self.statics, id, 0);
            self.render_bottom_face(id, var2 as f64, var3 as f64, var4 as f64, t);
            var13 = true;
        }

        // src: RenderBlocks.java:1550-1625 (the sides: -z, +z, -x, +x)
        for var28 in 0..4 {
            let mut var53 = var2;
            let mut var31 = var4;
            if var28 == 0 {
                var31 = var4 - 1;
            }
            if var28 == 1 {
                var31 += 1;
            }
            if var28 == 2 {
                var53 = var2 - 1;
            }
            if var28 == 3 {
                var53 += 1;
            }
            let var54 = tex(self, var28 + 2);
            let var33 = (var54 & 15) << 4;
            let var55 = var54 & 240;
            if self.render_all_faces || var12[var28 as usize] {
                // src: RenderBlocks.java:1576-1604: the two top corners
                // (heights var35, var36) at (var37, var38) and (var39, var40)
                let (var35, var36, var37, var39, var38, var40): (f32, f32, f32, f32, f32, f32) =
                    if var28 == 0 {
                        (
                            var24,
                            var27,
                            var2 as f32,
                            (var2 + 1) as f32,
                            var4 as f32,
                            var4 as f32,
                        )
                    } else if var28 == 1 {
                        (
                            var26,
                            var25,
                            (var2 + 1) as f32,
                            var2 as f32,
                            (var4 + 1) as f32,
                            (var4 + 1) as f32,
                        )
                    } else if var28 == 2 {
                        (
                            var25,
                            var24,
                            var2 as f32,
                            var2 as f32,
                            (var4 + 1) as f32,
                            var4 as f32,
                        )
                    } else {
                        (
                            var27,
                            var26,
                            (var2 + 1) as f32,
                            (var2 + 1) as f32,
                            var4 as f32,
                            (var4 + 1) as f32,
                        )
                    };
                var13 = true;
                // src: RenderBlocks.java:1607-1611 (the Java's mix of float
                // and double arithmetic, kept)
                let var41 = ((var33 + 0) as f32 / 256.0) as f64;
                let var43 = ((var33 + 16) as f64 - 0.01) / 256.0;
                let var45 = ((var55 as f32 + (1.0 - var35) * 16.0) / 256.0) as f64;
                let var47 = ((var55 as f32 + (1.0 - var36) * 16.0) / 256.0) as f64;
                let var49 = ((var55 + 16) as f64 - 0.01) / 256.0;
                // src: RenderBlocks.java:1612-1617
                let mut var51 = self.brightness(id, var53, var3, var31);
                if var28 < 2 {
                    var51 *= var16;
                } else {
                    var51 *= var17;
                }
                self.tess.set_color_opaque_f(
                    var15 * var51 * var7,
                    var15 * var51 * var8,
                    var15 * var51 * var9,
                );
                // src: RenderBlocks.java:1620-1623
                self.tess.add_vertex_with_uv(
                    var37 as f64,
                    (var3 as f32 + var35) as f64,
                    var38 as f64,
                    var41,
                    var45,
                );
                self.tess.add_vertex_with_uv(
                    var39 as f64,
                    (var3 as f32 + var36) as f64,
                    var40 as f64,
                    var43,
                    var47,
                );
                self.tess.add_vertex_with_uv(
                    var39 as f64,
                    (var3 + 0) as f64,
                    var40 as f64,
                    var43,
                    var49,
                );
                self.tess.add_vertex_with_uv(
                    var37 as f64,
                    (var3 + 0) as f64,
                    var38 as f64,
                    var41,
                    var49,
                );
            }
        }

        // src: RenderBlocks.java:1627-1628 (`var1.minY = 0; var1.maxY = 1`
        // on the singleton)
        let bb = &mut self.statics.bounds[(id & 255) as usize];
        bb.min_y = var18;
        bb.max_y = var20;
        var13
    }

    /// `func_1224_a(x, y, z, material)` (private; later MCP:
    /// `getFluidHeight`): the surface height at the corner `(x, z)` of the
    /// four cells `(x - 1 .. x, z - 1 .. z)` at `y`: 1 when any of them has
    /// fluid of `material` above it, else 1 minus the weighted mean of
    /// their `getPercentAir`, sources and falling fluid (metadata 0 and 8+)
    /// weighing 11, flowing fluid 1, a cell that is not solid (air, plants)
    /// counting as empty with weight 1, solid cells not counting.
    // src: RenderBlocks.java:1633-1663
    pub fn func_1224_a(&self, x: i32, y: i32, z: i32, var4: Material) -> f32 {
        let s = self.block_access;
        let mut var5 = 0i32;
        let mut var6 = 0.0f32;
        for var7 in 0..4 {
            let var8 = x - (var7 & 1);
            let var10 = z - (var7 >> 1 & 1);
            if s.get_block_material(var8, y + 1, var10) == var4 {
                return 1.0;
            }
            let var11 = s.get_block_material(var8, y, var10);
            if var11 != var4 {
                if !var11.is_solid() {
                    var6 += 1.0;
                    var5 += 1;
                }
            } else {
                let var12 = s.get_block_metadata(var8, y, var10);
                if var12 >= 8 || var12 == 0 {
                    var6 += get_percent_air(var12) * 10.0;
                    var5 += 10;
                }
                var6 += get_percent_air(var12);
                var5 += 1;
            }
        }
        1.0 - var6 / var5 as f32
    }
}

/// `BlockFluid.getEffectiveFlowDecay(blockAccess, x, y, z)` of the
/// singleton of material `mat`: the metadata of a block of that material
/// (falling fluid, 8 and up, read as 0), -1 for anything else.
// src: BlockFluid.java:35-46
fn get_effective_flow_decay(s: &ChunkSnapshot, mat: Material, x: i32, y: i32, z: i32) -> i32 {
    if s.get_block_material(x, y, z) != mat {
        -1
    } else {
        let mut var5 = s.get_block_metadata(x, y, z);
        if var5 >= 8 {
            var5 = 0;
        }
        var5
    }
}

/// `BlockFluid.getIsBlockSolid(blockAccess, x, y, z, side)`: never this
/// fluid or ice; always the top face; else `Block.getIsBlockSolid` (the
/// material at `x, y, z` `isSolid()`).
// src: BlockFluid.java:60-69, Block.java:253-255
#[allow(clippy::if_same_then_else)] // the Java's branches, kept
fn get_is_block_solid(s: &ChunkSnapshot, mat: Material, x: i32, y: i32, z: i32, side: i32) -> bool {
    let var6 = s.get_block_material(x, y, z);
    if var6 == mat {
        false
    } else if var6 == Material::Ice {
        false
    } else if side == 1 {
        true
    } else {
        s.get_block_material(x, y, z).is_solid()
    }
}

/// `BlockFluid.getFlowVector(blockAccess, x, y, z)` (private) of the
/// singleton of material `mat`: the decay gradient toward the four
/// neighbours (one that is neither this fluid nor solid counts through the
/// block below it, as 8 levels lower), for falling fluid next to a solid
/// side or a solid side above plus a pull of 6 down; normalised.
// src: BlockFluid.java:98-178
pub fn get_flow_vector(s: &ChunkSnapshot, mat: Material, x: i32, y: i32, z: i32) -> Vec3D {
    let (var2, var3, var4) = (x, y, z);
    let mut var5 = Vec3D::create_vector(0.0, 0.0, 0.0);
    let var6 = get_effective_flow_decay(s, mat, var2, var3, var4);
    // src: BlockFluid.java:102-135 (the neighbours -x, -z, +x, +z)
    for var7 in 0..4 {
        let mut var8 = var2;
        let mut var10 = var4;
        if var7 == 0 {
            var8 = var2 - 1;
        }
        if var7 == 1 {
            var10 = var4 - 1;
        }
        if var7 == 2 {
            var8 += 1;
        }
        if var7 == 3 {
            var10 += 1;
        }
        let mut var11 = get_effective_flow_decay(s, mat, var8, var3, var10);
        if var11 < 0 {
            if !s.get_block_material(var8, var3, var10).get_is_solid() {
                var11 = get_effective_flow_decay(s, mat, var8, var3 - 1, var10);
                if var11 >= 0 {
                    // src: BlockFluid.java:127-128
                    let var12 = var11 - (var6 - 8);
                    var5 = var5.add_vector(
                        (var8 - var2).wrapping_mul(var12) as f64,
                        0.0, // (var3 - var3) * var12
                        (var10 - var4).wrapping_mul(var12) as f64,
                    );
                }
            }
        } else if var11 >= 0 {
            // src: BlockFluid.java:131-133
            let var12 = var11 - var6;
            var5 = var5.add_vector(
                (var8 - var2).wrapping_mul(var12) as f64,
                0.0, // (var3 - var3) * var12
                (var10 - var4).wrapping_mul(var12) as f64,
            );
        }
    }
    // src: BlockFluid.java:137-174 (each `var13 ||` skips the remaining
    // queries once one holds; they only read)
    if s.get_block_metadata(var2, var3, var4) >= 8 {
        let mut var13 = false;
        for (cx, cy, cz, side) in [
            (var2, var3, var4 - 1, 2),
            (var2, var3, var4 + 1, 3),
            (var2 - 1, var3, var4, 4),
            (var2 + 1, var3, var4, 5),
            (var2, var3 + 1, var4 - 1, 2),
            (var2, var3 + 1, var4 + 1, 3),
            (var2 - 1, var3 + 1, var4, 4),
            (var2 + 1, var3 + 1, var4, 5),
        ] {
            if var13 || get_is_block_solid(s, mat, cx, cy, cz, side) {
                var13 = true;
            }
        }
        if var13 {
            // src: BlockFluid.java:172
            var5 = var5.normalize().add_vector(0.0, -6.0, 0.0);
        }
    }
    var5.normalize()
}

/// `BlockFluid.getFlowDirection(blockAccess, x, y, z, material)` (static):
/// the flow angle in radians the top texture is turned by (`atan2(z, x) -
/// π/2`: 0 for flow toward +z, -π/2 toward +x, π/2 toward -x, π toward -z),
/// -1000 when the flow has no horizontal part. Water uses
/// `Block.waterMoving`'s `getFlowVector`, lava `Block.lavaMoving`'s; `None`
/// for another material, where the Java throws a `NullPointerException`.
// src: BlockFluid.java:226-237
pub fn get_flow_direction(
    s: &ChunkSnapshot,
    x: i32,
    y: i32,
    z: i32,
    var4: Material,
) -> Option<f64> {
    let mut var5 = None;
    if var4 == Material::Water {
        var5 = Some(get_flow_vector(s, Material::Water, x, y, z));
    }
    if var4 == Material::Lava {
        var5 = Some(get_flow_vector(s, Material::Lava, x, y, z));
    }
    let var5 = var5?;
    Some(if var5.x_coord == 0.0 && var5.z_coord == 0.0 {
        -1000.0
    } else {
        // src: BlockFluid.java:236 (Math.atan2 is fdlibm's, PLAN_v4 §3.4
        // rule 5)
        jmath::atan2(var5.z_coord, var5.x_coord) - 1.5707963267948966
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mc_sim::world::provider::{Dimension, WorldProvider};

    const STONE: i32 = 1;
    const FLOWING_WATER: i32 = 8;
    const WATER: i32 = 9;
    const FLOWING_LAVA: i32 = 10;
    const LAVA: i32 = 11;
    const GLASS: i32 = 20;
    const ICE: i32 = 79;
    const SAND: i32 = 12;

    fn table() -> [f32; 16] {
        WorldProvider::new(Dimension::Surface).light_brightness_table
    }

    /// An all-air box x, z in [-4, 5), y in [0, 6).
    fn snap() -> ChunkSnapshot {
        ChunkSnapshot::synthetic([-4, 0, -4], [5, 6, 5], table())
    }

    fn rb(s: &ChunkSnapshot) -> RenderBlocks<'_> {
        RenderBlocks::new(s, s.statics.clone(), false, false)
    }

    /// Stone at y = 0 over the whole box.
    fn floor(s: &mut ChunkSnapshot) {
        for x in -4..5 {
            for z in -4..5 {
                s.set(x, 0, z, STONE, 0, 0, 0);
            }
        }
    }

    /// Stone at y = 1 around (0, 1, 0) and the eight neighbours of it.
    fn walls(s: &mut ChunkSnapshot) {
        for x in -1..=1 {
            for z in -1..=1 {
                if (x, z) != (0, 0) {
                    s.set(x, 1, z, STONE, 0, 15, 0);
                }
            }
        }
    }

    const P: fn(i32) -> f32 = get_percent_air;

    #[test]
    fn percent_air() {
        // src: BlockFluid.java:18-25
        assert_eq!(P(0), 1.0 / 9.0);
        assert_eq!(P(7), 8.0 / 9.0);
        assert_eq!(P(8), P(0));
        assert_eq!(P(15), P(0));
    }

    /// A lone source in the open: its own cell weighs 11, the three air
    /// cells of each corner count as empty.
    #[test]
    fn corner_heights_of_a_lone_source() {
        let mut s = snap();
        floor(&mut s);
        s.set(0, 1, 0, WATER, 0, 15, 0);
        let r = rb(&s);
        // var7 = 0: the source; 1, 2, 3: air
        let mut v6 = 0.0f32;
        v6 += P(0) * 10.0;
        v6 += P(0);
        v6 += 1.0;
        v6 += 1.0;
        v6 += 1.0;
        let want = 1.0 - v6 / 14.0;
        for (x, z) in [(0, 0), (0, 1), (1, 1), (1, 0)] {
            assert_eq!(r.func_1224_a(x, 1, z, Material::Water), want, "{x},{z}");
        }
        assert!((want - (1.0 - 38.0 / 126.0)).abs() < 1e-6);
    }

    /// Walled in by stone (which does not count), a source or falling
    /// fluid stands at 8/9 and a flowing level m at 1 - (m + 1)/9; water
    /// above any of the four cells raises the corner to 1.
    #[test]
    fn corner_heights_by_metadata() {
        for meta in 0..16 {
            let mut s = snap();
            floor(&mut s);
            walls(&mut s);
            s.set(0, 1, 0, FLOWING_WATER, meta, 15, 0);
            let r = rb(&s);
            let h = r.func_1224_a(0, 1, 0, Material::Water);
            let want = if meta == 0 || meta >= 8 {
                1.0 - (P(meta) * 10.0 + P(meta)) / 11.0
            } else {
                1.0 - P(meta) / 1.0
            };
            assert_eq!(h, want, "meta {meta}");
            if (1..8).contains(&meta) {
                assert!((h - (1.0 - (meta + 1) as f32 / 9.0)).abs() < 1e-6);
            } else {
                assert!((h - 8.0 / 9.0).abs() < 1e-6, "meta {meta}: {h}");
            }
        }
        // water above the cell (-1, -1) of corner (0, 0)
        let mut s = snap();
        floor(&mut s);
        s.set(0, 1, 0, WATER, 0, 15, 0);
        s.set(-1, 2, -1, WATER, 0, 15, 0);
        let r = rb(&s);
        assert_eq!(r.func_1224_a(0, 1, 0, Material::Water), 1.0);
        assert!(r.func_1224_a(1, 1, 1, Material::Water) < 1.0);
        // lava above does not count for water
        s.set(-1, 2, -1, LAVA, 0, 15, 0);
        let r = rb(&s);
        assert!(r.func_1224_a(0, 1, 0, Material::Water) < 1.0);
    }

    /// A corner shared by a source and a flowing level 3, the other two
    /// cells stone: (11/9 + 4/9) / 12 empty.
    #[test]
    fn corner_mixes_neighbours() {
        let mut s = snap();
        floor(&mut s);
        walls(&mut s);
        s.set(0, 1, 0, WATER, 0, 15, 0);
        s.set(-1, 1, 0, FLOWING_WATER, 3, 15, 0);
        let r = rb(&s);
        let mut v6 = 0.0f32;
        v6 += P(0) * 10.0;
        v6 += P(0);
        v6 += P(3);
        assert_eq!(r.func_1224_a(0, 1, 0, Material::Water), 1.0 - v6 / 12.0);
    }

    /// A stream one block wide between stone walls, decay growing along
    /// `dir`: the flow points along it.
    fn stream(dir: (i32, i32)) -> f64 {
        let mut s = snap();
        floor(&mut s);
        for x in -3..4 {
            for z in -3..4 {
                s.set(x, 1, z, STONE, 0, 15, 0);
            }
        }
        for k in -2..=2 {
            let (x, z) = (dir.0 * k, dir.1 * k);
            s.set(x, 1, z, FLOWING_WATER, k + 2, 15, 0);
        }
        get_flow_direction(&s, 0, 1, 0, Material::Water).unwrap()
    }

    #[test]
    fn flow_direction_of_a_stream() {
        let half_pi = 1.5707963267948966f64;
        // toward +x: vector (1, 0, 0), atan2(0, 1) - π/2
        assert_eq!(stream((1, 0)), jmath::atan2(0.0, 1.0) - half_pi);
        assert_eq!(stream((1, 0)), -half_pi);
        // toward +z: atan2(1, 0) - π/2 = 0
        assert_eq!(stream((0, 1)), jmath::atan2(1.0, 0.0) - half_pi);
        assert!(stream((0, 1)).abs() < 1e-15);
        // toward -x: π/2; toward -z: -π
        assert!((stream((-1, 0)) - half_pi).abs() < 1e-15);
        assert!((stream((0, -1)) + 2.0 * half_pi).abs() < 1e-15);
    }

    #[test]
    fn flow_direction_still_and_falling() {
        // a pool of sources: no gradient
        let mut s = snap();
        floor(&mut s);
        for x in -2..=2 {
            for z in -2..=2 {
                s.set(x, 1, z, WATER, 0, 15, 0);
            }
        }
        assert_eq!(
            get_flow_direction(&s, 0, 1, 0, Material::Water),
            Some(-1000.0)
        );
        // falling water in a shaft: the -6 pull, no horizontal part
        let mut s = snap();
        walls(&mut s);
        s.set(0, 1, 0, FLOWING_WATER, 8, 15, 0);
        let v = get_flow_vector(&s, Material::Water, 0, 1, 0);
        assert_eq!((v.x_coord, v.y_coord, v.z_coord), (0.0, -1.0, 0.0));
        assert_eq!(
            get_flow_direction(&s, 0, 1, 0, Material::Water),
            Some(-1000.0)
        );
        // falling water spreading over the open edge of a ledge: the
        // neighbour that is air counts through the block below it
        let mut s = snap();
        floor(&mut s);
        s.set(0, 2, 0, FLOWING_WATER, 0, 15, 0);
        s.set(1, 1, 0, FLOWING_WATER, 1, 15, 0);
        // var6 = 0, +x is air at y 2 and water level 1 below:
        // var12 = 1 - (0 - 8) = 9 → (9, 0, 0)
        let v = get_flow_vector(&s, Material::Water, 0, 2, 0);
        assert_eq!((v.x_coord, v.y_coord, v.z_coord), (1.0, 0.0, 0.0));
        // another material is the Java's NullPointerException
        assert_eq!(get_flow_direction(&s, 0, 1, 0, Material::Rock), None);
        // lava reads lava, not water
        let mut s = snap();
        floor(&mut s);
        walls(&mut s);
        s.set(0, 1, 0, FLOWING_LAVA, 2, 15, 0);
        s.set(1, 1, 0, WATER, 0, 15, 0);
        s.set(-1, 1, 0, LAVA, 0, 15, 0);
        // -x: lava 0 → var12 = -2 → (2, 0, 0); +x water is neither lava nor
        // solid, the block below is stone
        let v = get_flow_vector(&s, Material::Lava, 0, 1, 0);
        assert_eq!((v.x_coord, v.z_coord), (1.0, 0.0));
    }

    /// The top face's UVs: still, the full tile 205; flowing toward +x,
    /// the 32×32 tile 206 sampled around (240, 208) turned by -π/2.
    #[test]
    fn top_face_uvs() {
        let f = |v: f64| v as f32;
        // still
        let mut s = snap();
        floor(&mut s);
        walls(&mut s);
        s.set(0, 1, 0, WATER, 0, 15, 0);
        let mut r = rb(&s);
        assert!(r.render_block_fluids(WATER, 0, 1, 0));
        // only the top (the walls hide the sides, the floor the bottom)
        assert_eq!(r.tess.len(), 4);
        // tile 205: u 208..224, v 192..208 (pixels); sin 0 = 0, cos 0 = 1
        let c = 8.0f32 / 256.0;
        let (cu, cv) = (216.0 / 256.0, 200.0 / 256.0);
        let want = [
            [f(cu - c as f64), f(cv - c as f64)],
            [f(cu - c as f64), f(cv + c as f64)],
            [f(cu + c as f64), f(cv + c as f64)],
            [f(cu + c as f64), f(cv - c as f64)],
        ];
        assert_eq!(r.tess.uvs, want);
        assert_eq!(r.tess.uvs[0], [208.0 / 256.0, 192.0 / 256.0]);
        assert_eq!(r.tess.uvs[2], [224.0 / 256.0, 208.0 / 256.0]);

        // flowing toward +x (angle -π/2 → float -1.5707964)
        let mut s = snap();
        floor(&mut s);
        for x in -3..4 {
            for z in -3..4 {
                s.set(x, 1, z, STONE, 0, 15, 0);
            }
        }
        for k in -2..=2 {
            s.set(k, 1, 0, FLOWING_WATER, k + 2, 15, 0);
        }
        let mut r = rb(&s);
        assert!(r.render_block_fluids(FLOWING_WATER, 0, 1, 0));
        let a = (-1.5707963267948966f64) as f32;
        let sn = math::sin(a) * 8.0 / 256.0;
        let cs = math::cos(a) * 8.0 / 256.0;
        // MathHelper's table: sin(-π/2) = -1 (index 49152), cos = the entry
        // at (int)(a × 10430.378 + 16384) = 0 → 0
        assert_eq!(math::sin(a), -1.0);
        assert_eq!(math::cos(a), 0.0);
        let (cu, cv) = (240.0f64 / 256.0, 208.0f64 / 256.0);
        let (s64, c64) = (sn as f64, cs as f64);
        let want = [
            [f(cu - c64 - s64), f(cv - c64 + s64)],
            [f(cu - c64 + s64), f(cv + c64 + s64)],
            [f(cu + c64 + s64), f(cv + c64 - s64)],
            [f(cu + c64 - s64), f(cv - c64 - s64)],
        ];
        // the top face is the first quad
        assert_eq!(&r.tess.uvs[..4], &want);
        // by hand: the corners (248, 200), (232, 200), (232, 216), (248, 216)
        // pixels: texture u runs along world -z, v along +x, so the
        // animation (which scrolls toward +v) moves toward +x, downstream
        let px = |uv: [f32; 2]| [uv[0] * 256.0, uv[1] * 256.0];
        assert_eq!(px(r.tess.uvs[0]), [248.0, 200.0]);
        assert_eq!(px(r.tess.uvs[1]), [232.0, 200.0]);
        assert_eq!(px(r.tess.uvs[2]), [232.0, 216.0]);
        assert_eq!(px(r.tess.uvs[3]), [248.0, 216.0]);
    }

    /// `BlockFluid.shouldSideBeRendered` through the renderer: the +x face
    /// of water against each neighbour, and of lava against water.
    #[test]
    fn side_visibility_table() {
        for (fluid, neighbour, drawn) in [
            (WATER, 0, true),
            (WATER, WATER, false),
            (WATER, FLOWING_WATER, false),
            (WATER, GLASS, true),
            (WATER, STONE, false),
            (WATER, ICE, false),
            (WATER, LAVA, true),
            (LAVA, WATER, true),
            (LAVA, FLOWING_LAVA, false),
            (LAVA, 0, true),
            (LAVA, STONE, false),
        ] {
            let mut s = snap();
            s.set(1, 1, 0, neighbour, 0, 15, 0);
            let r = rb(&s);
            assert_eq!(
                r.should_side(fluid, 1, 1, 0, 5),
                drawn,
                "{fluid} | {neighbour}"
            );
        }
        // the top face is drawn under anything but its own material and ice
        for (above, drawn) in [
            (0, true),
            (STONE, true),
            (SAND, true),
            (WATER, false),
            (ICE, false),
        ] {
            let mut s = snap();
            s.set(0, 2, 0, above, 0, 15, 0);
            let r = rb(&s);
            assert_eq!(r.should_side(WATER, 0, 2, 0, 1), drawn, "under {above}");
        }
        // the bottom face against sand is hidden, against air drawn
        let mut s = snap();
        s.set(0, 0, 0, SAND, 0, 15, 0);
        assert!(!rb(&s).should_side(WATER, 0, 0, 0, 0));
    }

    /// A lone source on stone: top, four sides, no bottom; the side shading
    /// and the side tops at the corner heights; the bounds reset.
    #[test]
    fn lone_source_faces() {
        let mut s = snap();
        floor(&mut s);
        s.set(0, 1, 0, WATER, 0, 15, 0);
        let mut r = rb(&s);
        assert!(r.render_block_fluids(WATER, 0, 1, 0));
        assert_eq!(r.tess.len(), 20);
        let h = r.func_1224_a(0, 1, 0, Material::Water);
        // top: all four corners at h above y = 1
        for v in &r.tess.positions[..4] {
            assert_eq!(v[1], (1.0f32 + h) as f64 as f32);
        }
        // the first side (-z): corners (0, h, 0), (1, h, 0), (1, 1, 0) floor, (0, 1, 0)
        assert_eq!(
            &r.tess.positions[4..8],
            &[
                [0.0, 1.0 + h, 0.0],
                [1.0, 1.0 + h, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0]
            ]
        );
        // its v range starts at (1 - h) × 16 pixels into tile 206
        let v0 = ((192.0f32 + (1.0 - h) * 16.0) / 256.0) as f64 as f32;
        assert_eq!(r.tess.uvs[4], [224.0 / 256.0, v0]);
        assert_eq!(r.tess.uvs[6][1], ((208.0f64 - 0.01) / 256.0) as f32);
        // shades: top 1.0, z sides 0.8, x sides 0.6, at full sky light
        let t = table();
        let shade = |v: f32| ((v * 255.0) as i32).clamp(0, 255) as u8;
        assert_eq!(r.tess.colors[0][0], shade(1.0 * t[15] * 1.0));
        assert_eq!(r.tess.colors[4][0], shade(1.0 * (t[15] * 0.8) * 1.0));
        assert_eq!(r.tess.colors[12][0], shade(1.0 * (t[15] * 0.6) * 1.0));
        assert_eq!(r.statics.bounds[WATER as usize].min_y, 0.0);
        assert_eq!(r.statics.bounds[WATER as usize].max_y, 1.0);
    }

    /// The bottom face over air, shade 0.5 of the brighter of the cell below
    /// and the fluid's own cell; nothing visible draws nothing.
    #[test]
    fn bottom_face_and_hidden_block() {
        let mut s = snap();
        walls(&mut s);
        s.set(0, 2, 0, WATER, 0, 15, 0);
        s.set(0, 1, 0, 0, 0, 4, 0);
        s.set(0, 3, 0, STONE, 0, 15, 0);
        // the walls one level up too
        for x in -1..=1 {
            for z in -1..=1 {
                if (x, z) != (0, 0) {
                    s.set(x, 2, z, STONE, 0, 15, 0);
                }
            }
        }
        let mut r = rb(&s);
        assert!(r.render_block_fluids(WATER, 0, 2, 0));
        // the top under stone and the bottom
        assert_eq!(r.tess.len(), 8);
        let t = table();
        let b = t[4].max(t[15]);
        let shade = |v: f32| ((v * 255.0) as i32).clamp(0, 255) as u8;
        assert_eq!(r.tess.colors[4][0], shade(0.5 * b));
        assert_eq!(
            r.tess.uvs[4],
            [208.0 / 256.0, ((192.0f64 + 16.0 - 0.01) / 256.0) as f32]
        );
        // the same block enclosed by water and stone: nothing
        let mut s2 = snap();
        for x in -1..=1 {
            for z in -1..=1 {
                for y in 1..=3 {
                    s2.set(x, y, z, STONE, 0, 15, 0);
                }
            }
        }
        s2.set(0, 2, 0, WATER, 0, 15, 0);
        s2.set(0, 3, 0, WATER, 0, 15, 0);
        let mut r = rb(&s2);
        assert!(!r.render_block_fluids(WATER, 0, 2, 0));
        assert!(r.tess.is_empty());
        // renderAllFaces does not help a block with no visible face: the
        // early return comes first (RenderBlocks.java:1493-1494)
        r.render_all_faces = true;
        assert!(!r.render_block_fluids(WATER, 0, 2, 0));
        assert!(r.tess.is_empty());
        // with one visible face (air at +x) it draws all six
        s2.set(1, 2, 0, 0, 0, 15, 0);
        let mut r = rb(&s2);
        r.render_all_faces = true;
        assert!(r.render_block_fluids(WATER, 0, 2, 0));
        assert_eq!(r.tess.len(), 24);
    }

    #[test]
    fn passes_and_dispatch() {
        // src: BlockFluid.java:205-207
        assert_eq!(blocks::get_render_block_pass(FLOWING_WATER), 1);
        assert_eq!(blocks::get_render_block_pass(WATER), 1);
        assert_eq!(blocks::get_render_block_pass(FLOWING_LAVA), 0);
        assert_eq!(blocks::get_render_block_pass(LAVA), 0);
        for id in [FLOWING_WATER, WATER, FLOWING_LAVA, LAVA] {
            assert_eq!(blocks::get_render_type(id), 4);
        }
        // the textures: still top/bottom 205/237, flowing sides 206/238
        let st = snap().statics;
        for (id, still) in [
            (WATER, 205),
            (FLOWING_WATER, 205),
            (LAVA, 237),
            (FLOWING_LAVA, 237),
        ] {
            for meta in [0, 3, 8] {
                for side in 0..6 {
                    let want = if side < 2 { still } else { still + 1 };
                    assert_eq!(
                        blocks::get_block_texture_from_side_and_metadata(&st, id, side, meta),
                        want
                    );
                }
            }
        }
        // the dispatch reaches the fluid renderer
        let mut s = snap();
        floor(&mut s);
        s.set(0, 1, 0, LAVA, 0, 15, 0);
        let mut r = rb(&s);
        assert!(r.render_block_by_render_type(LAVA, 0, 1, 0));
        assert_eq!(r.tess.len(), 20);
    }
}
