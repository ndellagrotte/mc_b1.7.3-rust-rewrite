//! WP-3.3d: the remaining render types of `RenderBlocks`:
//! `renderBlockRedstoneWire` (5), `renderBlockDoor` (7),
//! `renderBlockLadder` (8), `renderBlockMinecartTrack` (9),
//! `renderBlockStairs` (10), `renderBlockFence` (11), `renderBlockLever`
//! (12), `renderBlockBed` (14), `renderBlockRepeater` (15), the piston base
//! `func_31074_b` (16) and head `func_31080_c` (17) with their helpers
//! (`func_31078_d`, `func_31079_a`, the rod quads `func_31076_a`,
//! `func_31081_b`, `func_31077_c`) (RenderBlocks.java:134-274, :303-794,
//! :984-1305, :2446-2652), plus `BlockRedstoneWire.isPowerProviderOrWire`
//! on the snapshot. `renderBlockFallingSand` (RenderFallingSand, WP-3.5b)
//! reads a `World` and belongs with it.
//!
//! Every other block `RenderBlocks` draws is a standard cube with bounds
//! (render type 0, [`RenderBlocks::render_standard_block`]): the portal,
//! snow layer, pressure plates, buttons, farmland, slabs, glass, cake,
//! trapdoor; the sign and the moving piston (type -1) are drawn by their
//! tile-entity renderers (WP-3.5b). The inventory cases of these types
//! (10, 11, 16) are in [`RenderBlocks::render_block_on_inventory`]; the
//! others are not drawn as 3D items (`renderItemIn3d`).
//!
//! # Conventions
//!
//! The Java mutates the `Block` singleton's bounds (`setBlockBounds`) and
//! the renderer's `flipTexture` / `uvRotate*` fields around calls to the
//! face functions; here the bounds are [`RenderBlocks::statics`] (the job's
//! copy) and the fields are [`RenderBlocks`]'s. The bounds a renderer leaves
//! behind stay, as in vanilla (the lever's box): the next
//! `setBlockBoundsBasedOnState` of a block of that id resets them.
//! `setBlockBoundsBasedOnState` itself (doors, beds, rails, pistons,
//! levers, stairs) is computed by `mc-sim` on the world and carried by the
//! snapshot ([`super::snapshot`]); the textures come from `mc-sim`'s
//! `getBlockTextureFromSideAndMetadata` through [`super::blocks`] (the
//! door's mirrored faces are negative, BlockDoor.java:18-36; the piston
//! base's front reads the singleton's current bounds,
//! BlockPistonBase.java:20-29). Arithmetic is the Java's: `float` → `f32`,
//! `double` → `f64`, in its order.
//!
//! The repeater's two torches are drawn by `renderTorchAtAngle`
//! ([`RenderBlocks::render_torch_at_angle`], WP-3.3c's `plants.rs`).

// The Java's `(float)(var2 + 0)` and friends, kept.
#![allow(clippy::identity_op)]

use mc_core::vec3d::Vec3D;
use mc_sim::block::{self, BlockKind, REPEATER_TORCH_OFFSET, bed, piston_base, piston_extension};

use super::blocks::{self, bounds};
use super::render_blocks::RenderBlocks;
use super::snapshot::ChunkSnapshot;
use crate::model::bed::{FOOT_INVISIBLE_FACE_REMAP, HEAD_INVISIBLE_FACE};

/// `Block.redstoneWire.blockID`.
// src: Block.java:647
pub const REDSTONE_WIRE: i32 = 55;
/// `Block.redstoneRepeaterIdle.blockID`.
// src: Block.java:685
pub const REDSTONE_REPEATER_IDLE: i32 = 93;
/// `Block.redstoneRepeaterActive.blockID`.
// src: Block.java:686
pub const REDSTONE_REPEATER_ACTIVE: i32 = 94;
/// `Block.cobblestone.blockIndexInTexture`: the lever's base.
// src: Block.java:596, RenderBlocks.java:640
pub const COBBLESTONE_TEXTURE: i32 = 16;
/// The piston rod's texture (`func_31076_a`'s `var16`).
// src: RenderBlocks.java:484
pub const PISTON_ROD_TEXTURE: i32 = 108;

/// `Block.blocksList[id].canProvidePower()` as the renderer sees it:
/// detector rail, lever, pressure plates, redstone torches and buttons
/// answer true, the repeater false; the wire answers its
/// `wiresProvidePower`, which is `true` except inside its own power update
/// (never while a frame is drawn). Every class's answer is constant, so
/// this needs no world (mc-sim's `dispatch::can_provide_power` reads the
/// wire's flag from it).
// src: Block.java:537-539 and the overrides: BlockDetectorRail.java:16-18,
// BlockRedstoneWire.java:9, :335-337, BlockLever.java:227-229,
// BlockPressurePlate.java:145-147, BlockRedstoneTorch.java:147-149,
// BlockButton.java:221-223, BlockRedstoneRepeater.java:127-129
pub fn can_provide_power(id: i32) -> bool {
    let Some(b) = blocks::def(id) else {
        return false;
    };
    match b.kind {
        BlockKind::DetectorRail => block::detector_rail::can_provide_power(),
        BlockKind::RedstoneWire => true,
        BlockKind::Lever => block::lever::can_provide_power(),
        BlockKind::PressurePlate { .. } => block::pressure_plate::can_provide_power(),
        BlockKind::RedstoneTorch { .. } => block::redstone_torch::can_provide_power(),
        BlockKind::Button => block::button::can_provide_power(),
        BlockKind::RedstoneRepeater { .. } => block::repeater::can_provide_power(),
        _ => false,
    }
}

/// `BlockRedstoneWire.isPowerProviderOrWire(blockAccess, x, y, z,
/// direction)` on the snapshot: wire, a power provider, or a repeater whose
/// axis is `direction` (`footInvisibleFaceRemap[meta & 3]`; the diagonal
/// checks pass -1, which never matches).
// src: BlockRedstoneWire.java:366-380
pub fn is_power_provider_or_wire(s: &ChunkSnapshot, x: i32, y: i32, z: i32, var4: i32) -> bool {
    let var5 = s.get_block_id(x, y, z);
    if var5 == REDSTONE_WIRE {
        true
    } else if var5 == 0 {
        false
    } else if can_provide_power(var5) {
        true
    } else if var5 != REDSTONE_REPEATER_IDLE && var5 != REDSTONE_REPEATER_ACTIVE {
        false
    } else {
        let var6 = s.get_block_metadata(x, y, z);
        var4 == FOOT_INVISIBLE_FACE_REMAP[(var6 & 3) as usize]
    }
}

/// The wire's four connections `[-x, +x, -z, +z]` (`var26`..`var29`): a
/// neighbour that is wire or a power provider, the wire one block down
/// past a non-solid neighbour, and (when the block above is not solid) the
/// wire one block up on a solid neighbour.
// src: RenderBlocks.java:1016-1036
pub fn wire_connections(s: &ChunkSnapshot, x: i32, y: i32, z: i32) -> [bool; 4] {
    let (var2, var3, var4) = (x, y, z);
    let p = |x, y, z, d| is_power_provider_or_wire(s, x, y, z, d);
    let n = |x, y, z| s.is_block_normal_cube(x, y, z);
    let mut var26 =
        p(var2 - 1, var3, var4, 1) || !n(var2 - 1, var3, var4) && p(var2 - 1, var3 - 1, var4, -1);
    let mut var27 =
        p(var2 + 1, var3, var4, 3) || !n(var2 + 1, var3, var4) && p(var2 + 1, var3 - 1, var4, -1);
    let mut var28 =
        p(var2, var3, var4 - 1, 2) || !n(var2, var3, var4 - 1) && p(var2, var3 - 1, var4 - 1, -1);
    let mut var29 =
        p(var2, var3, var4 + 1, 0) || !n(var2, var3, var4 + 1) && p(var2, var3 - 1, var4 + 1, -1);
    if !n(var2, var3 + 1, var4) {
        if n(var2 - 1, var3, var4) && p(var2 - 1, var3 + 1, var4, -1) {
            var26 = true;
        }
        if n(var2 + 1, var3, var4) && p(var2 + 1, var3 + 1, var4, -1) {
            var27 = true;
        }
        if n(var2, var3, var4 - 1) && p(var2, var3 + 1, var4 - 1, -1) {
            var28 = true;
        }
        if n(var2, var3, var4 + 1) && p(var2, var3 + 1, var4 + 1, -1) {
            var29 = true;
        }
    }
    [var26, var27, var28, var29]
}

/// The wire's colour factors `(var10, var11, var12)` for power `meta`
/// (multiplied by the brightness): red `meta/15 × 0.6 + 0.4` (0.3 when
/// unpowered), green `p² × 0.7 − 0.5` and blue `p² × 0.6 − 0.7`, each at
/// least 0.
// src: RenderBlocks.java:993-1007
pub fn wire_color(meta: i32) -> [f32; 3] {
    let var9 = meta as f32 / 15.0;
    let mut var10 = var9 * 0.6 + 0.4;
    if meta == 0 {
        var10 = 0.3;
    }
    let mut var11 = var9 * var9 * 0.7 - 0.5;
    let mut var12 = var9 * var9 * 0.6 - 0.7;
    if var11 < 0.0 {
        var11 = 0.0;
    }
    if var12 < 0.0 {
        var12 = 0.0;
    }
    [var10, var11, var12]
}

/// The wire's flat texture: 0 the cross (with the arms of the missing
/// connections cut), 1 the line along x, 2 the line along z (`var35`).
// src: RenderBlocks.java:1042-1049
pub fn wire_shape(c: [bool; 4]) -> i32 {
    let [var26, var27, var28, var29] = c;
    let mut var35 = 0;
    if (var26 || var27) && !var28 && !var29 {
        var35 = 1;
    }
    if (var28 || var29) && !var27 && !var26 {
        var35 = 2;
    }
    var35
}

/// The texture rotations a piston base or head facing `facing` gives the
/// six faces: `[east (field_31087_g), west (_h), south (_i), north (_j),
/// top (_k), bottom (_l)]`. The two renderers set the same values.
// src: RenderBlocks.java:392-428, :439-469, :555-621
pub const fn piston_uv_rotations(facing: i32) -> [i32; 6] {
    match facing {
        0 => [3, 3, 3, 3, 0, 0],
        2 => [0, 0, 1, 2, 0, 0],
        3 => [0, 0, 2, 1, 3, 3],
        4 => [1, 2, 0, 0, 2, 1],
        5 => [2, 1, 0, 0, 1, 2],
        _ => [0; 6],
    }
}

/// The extended piston base's bounds per facing (the head side cut by a
/// quarter): `(x0, y0, z0, x1, y1, z1)`; full for a facing above 5.
// src: RenderBlocks.java:392-428
pub const fn piston_base_extended_bounds(facing: i32) -> [f32; 6] {
    match facing {
        0 => [0.0, 0.25, 0.0, 1.0, 1.0, 1.0],
        1 => [0.0, 0.0, 0.0, 1.0, 0.75, 1.0],
        2 => [0.0, 0.0, 0.25, 1.0, 1.0, 1.0],
        3 => [0.0, 0.0, 0.0, 1.0, 1.0, 0.75],
        4 => [0.25, 0.0, 0.0, 1.0, 1.0, 1.0],
        5 => [0.0, 0.0, 0.0, 0.75, 1.0, 1.0],
        _ => [0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
    }
}

/// The repeater's torch offsets for metadata `meta`: `[(dx, dz) of the
/// moving torch (var12, var14), (dx, dz) of the fixed one (var16,
/// var18)]`; both stand 0.1875 below the block (`var10`).
// src: RenderBlocks.java:305-336
pub fn repeater_torch_offsets(meta: i32) -> [[f64; 2]; 2] {
    let var6 = meta & 3;
    let var7 = ((meta & 12) >> 2) as usize;
    let mut var12 = 0.0f64;
    let mut var14 = 0.0f64;
    let mut var16 = 0.0f64;
    let mut var18 = 0.0f64;
    match var6 {
        0 => {
            var18 = -0.3125;
            var14 = REPEATER_TORCH_OFFSET[var7];
        }
        1 => {
            var16 = 0.3125;
            var12 = -REPEATER_TORCH_OFFSET[var7];
        }
        2 => {
            var18 = 0.3125;
            var14 = -REPEATER_TORCH_OFFSET[var7];
        }
        3 => {
            var16 = -0.3125;
            var12 = REPEATER_TORCH_OFFSET[var7];
        }
        _ => {}
    }
    [[var12, var14], [var16, var18]]
}

/// The lever stick's eight corners for metadata `meta` at block
/// `(x, y, z)`: a 0.125 × 0.625 × 0.125 box tilted ±40° about x (`(meta &
/// 8)` is on), turned per orientation (1-4 on a wall, 5 and 6 on the
/// floor), in `Vec3D`'s double arithmetic with the `MathHelper` table.
// src: RenderBlocks.java:682-735
pub fn lever_vertices(meta: i32, x: i32, y: i32, z: i32) -> [Vec3D; 8] {
    let var6 = meta & 7;
    let var7 = (meta & 8) > 0;
    let var22 = 0.0625f32;
    let var23 = 0.0625f32;
    let var24 = 0.625f32;
    let mut var21 = [
        Vec3D::create_vector(-var22 as f64, 0.0, -var23 as f64),
        Vec3D::create_vector(var22 as f64, 0.0, -var23 as f64),
        Vec3D::create_vector(var22 as f64, 0.0, var23 as f64),
        Vec3D::create_vector(-var22 as f64, 0.0, var23 as f64),
        Vec3D::create_vector(-var22 as f64, var24 as f64, -var23 as f64),
        Vec3D::create_vector(var22 as f64, var24 as f64, -var23 as f64),
        Vec3D::create_vector(var22 as f64, var24 as f64, var23 as f64),
        Vec3D::create_vector(-var22 as f64, var24 as f64, var23 as f64),
    ];
    for v in &mut var21 {
        if var7 {
            v.z_coord -= 0.0625;
            v.rotate_around_x(0.69813174);
        } else {
            v.z_coord += 0.0625;
            v.rotate_around_x(-0.69813174);
        }
        if var6 == 6 {
            v.rotate_around_y(1.5707964);
        }
        if var6 < 5 {
            v.y_coord -= 0.375;
            v.rotate_around_x(1.5707964);
            if var6 == 4 {
                v.rotate_around_y(0.0);
            }
            if var6 == 3 {
                v.rotate_around_y(3.1415927);
            }
            if var6 == 2 {
                v.rotate_around_y(1.5707964);
            }
            if var6 == 1 {
                v.rotate_around_y(-1.5707964);
            }
            v.x_coord += x as f64 + 0.5;
            v.y_coord += (y as f32 + 0.5) as f64;
            v.z_coord += z as f64 + 0.5;
        } else {
            v.x_coord += x as f64 + 0.5;
            v.y_coord += (y as f32 + 0.125) as f64;
            v.z_coord += z as f64 + 0.5;
        }
    }
    var21
}

impl RenderBlocks<'_> {
    /// `renderBlockRedstoneWire(block, x, y, z)`: render type 5. The flat
    /// quad (cross or line) tinted by power, then the same quad untinted
    /// with the texture one row down (the dust's overlay, v + 0.0625), and
    /// a pair of such quads up each solid side that has wire on top.
    // src: RenderBlocks.java:984-1183
    pub fn render_block_redstone_wire(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let (var2, var3, var4) = (x, y, z);
        let s = self.block_access;
        let var6 = s.get_block_metadata(var2, var3, var4);
        let mut var7 = blocks::get_block_texture_from_side_and_metadata(&self.statics, id, 1, var6);
        if self.override_block_texture >= 0 {
            var7 = self.override_block_texture;
        }
        let var8 = self.brightness(id, var2, var3, var4);
        let [var10, var11, var12] = wire_color(var6);
        self.tess
            .set_color_opaque_f(var8 * var10, var8 * var11, var8 * var12);
        let var13 = (var7 & 15) << 4;
        let var14 = var7 & 240;
        let mut var15 = (var13 as f32 / 256.0) as f64;
        let mut var17 = ((var13 as f32 + 15.99) / 256.0) as f64;
        let mut var19 = (var14 as f32 / 256.0) as f64;
        let mut var21 = ((var14 as f32 + 15.99) / 256.0) as f64;
        let c = wire_connections(s, var2, var3, var4);
        let [var26, var27, var28, var29] = c;
        let mut var31 = (var2 + 0) as f32;
        let mut var32 = (var2 + 1) as f32;
        let mut var33 = (var4 + 0) as f32;
        let mut var34 = (var4 + 1) as f32;
        let var35 = wire_shape(c);
        if var35 != 0 {
            var15 = ((var13 + 16) as f32 / 256.0) as f64;
            var17 = (((var13 + 16) as f32 + 15.99) / 256.0) as f64;
            var19 = (var14 as f32 / 256.0) as f64;
            var21 = ((var14 as f32 + 15.99) / 256.0) as f64;
        }
        let yy = (var3 as f32 + 0.015625) as f64;
        let (x0, x1, z0, z1);
        let t = &mut self.tess;
        if var35 == 0 {
            if var27 || var28 || var29 || var26 {
                if !var26 {
                    var31 += 0.3125;
                }
                if !var26 {
                    var15 += 0.01953125;
                }
                if !var27 {
                    var32 -= 0.3125;
                }
                if !var27 {
                    var17 -= 0.01953125;
                }
                if !var28 {
                    var33 += 0.3125;
                }
                if !var28 {
                    var19 += 0.01953125;
                }
                if !var29 {
                    var34 -= 0.3125;
                }
                if !var29 {
                    var21 -= 0.01953125;
                }
            }
            (x0, x1, z0, z1) = (var31 as f64, var32 as f64, var33 as f64, var34 as f64);
            t.add_vertex_with_uv(x1, yy, z1, var17, var21);
            t.add_vertex_with_uv(x1, yy, z0, var17, var19);
            t.add_vertex_with_uv(x0, yy, z0, var15, var19);
            t.add_vertex_with_uv(x0, yy, z1, var15, var21);
            t.set_color_opaque_f(var8, var8, var8);
            t.add_vertex_with_uv(x1, yy, z1, var17, var21 + 0.0625);
            t.add_vertex_with_uv(x1, yy, z0, var17, var19 + 0.0625);
            t.add_vertex_with_uv(x0, yy, z0, var15, var19 + 0.0625);
            t.add_vertex_with_uv(x0, yy, z1, var15, var21 + 0.0625);
        } else if var35 == 1 {
            (x0, x1, z0, z1) = (var31 as f64, var32 as f64, var33 as f64, var34 as f64);
            t.add_vertex_with_uv(x1, yy, z1, var17, var21);
            t.add_vertex_with_uv(x1, yy, z0, var17, var19);
            t.add_vertex_with_uv(x0, yy, z0, var15, var19);
            t.add_vertex_with_uv(x0, yy, z1, var15, var21);
            t.set_color_opaque_f(var8, var8, var8);
            t.add_vertex_with_uv(x1, yy, z1, var17, var21 + 0.0625);
            t.add_vertex_with_uv(x1, yy, z0, var17, var19 + 0.0625);
            t.add_vertex_with_uv(x0, yy, z0, var15, var19 + 0.0625);
            t.add_vertex_with_uv(x0, yy, z1, var15, var21 + 0.0625);
        } else if var35 == 2 {
            (x0, x1, z0, z1) = (var31 as f64, var32 as f64, var33 as f64, var34 as f64);
            t.add_vertex_with_uv(x1, yy, z1, var17, var21);
            t.add_vertex_with_uv(x1, yy, z0, var15, var21);
            t.add_vertex_with_uv(x0, yy, z0, var15, var19);
            t.add_vertex_with_uv(x0, yy, z1, var17, var19);
            t.set_color_opaque_f(var8, var8, var8);
            t.add_vertex_with_uv(x1, yy, z1, var17, var21 + 0.0625);
            t.add_vertex_with_uv(x1, yy, z0, var15, var21 + 0.0625);
            t.add_vertex_with_uv(x0, yy, z0, var15, var19 + 0.0625);
            t.add_vertex_with_uv(x0, yy, z1, var17, var19 + 0.0625);
        }
        if !s.is_block_normal_cube(var2, var3 + 1, var4) {
            var15 = ((var13 + 16) as f32 / 256.0) as f64;
            var17 = (((var13 + 16) as f32 + 15.99) / 256.0) as f64;
            var19 = (var14 as f32 / 256.0) as f64;
            var21 = ((var14 as f32 + 15.99) / 256.0) as f64;
            let top = ((var3 + 1) as f32 + 0.021875) as f64;
            let bot = (var3 + 0) as f64;
            let tinted = [var8 * var10, var8 * var11, var8 * var12];
            // src: RenderBlocks.java:1129-1140 (the -x wall)
            if s.is_block_normal_cube(var2 - 1, var3, var4)
                && s.get_block_id(var2 - 1, var3 + 1, var4) == REDSTONE_WIRE
            {
                let xx = (var2 as f32 + 0.015625) as f64;
                let (za, zb) = ((var4 + 1) as f64, (var4 + 0) as f64);
                t.set_color_opaque_f(tinted[0], tinted[1], tinted[2]);
                t.add_vertex_with_uv(xx, top, za, var17, var19);
                t.add_vertex_with_uv(xx, bot, za, var15, var19);
                t.add_vertex_with_uv(xx, bot, zb, var15, var21);
                t.add_vertex_with_uv(xx, top, zb, var17, var21);
                t.set_color_opaque_f(var8, var8, var8);
                t.add_vertex_with_uv(xx, top, za, var17, var19 + 0.0625);
                t.add_vertex_with_uv(xx, bot, za, var15, var19 + 0.0625);
                t.add_vertex_with_uv(xx, bot, zb, var15, var21 + 0.0625);
                t.add_vertex_with_uv(xx, top, zb, var17, var21 + 0.0625);
            }
            // src: RenderBlocks.java:1142-1153 (the +x wall)
            if s.is_block_normal_cube(var2 + 1, var3, var4)
                && s.get_block_id(var2 + 1, var3 + 1, var4) == REDSTONE_WIRE
            {
                let xx = ((var2 + 1) as f32 - 0.015625) as f64;
                let (za, zb) = ((var4 + 1) as f64, (var4 + 0) as f64);
                t.set_color_opaque_f(tinted[0], tinted[1], tinted[2]);
                t.add_vertex_with_uv(xx, bot, za, var15, var21);
                t.add_vertex_with_uv(xx, top, za, var17, var21);
                t.add_vertex_with_uv(xx, top, zb, var17, var19);
                t.add_vertex_with_uv(xx, bot, zb, var15, var19);
                t.set_color_opaque_f(var8, var8, var8);
                t.add_vertex_with_uv(xx, bot, za, var15, var21 + 0.0625);
                t.add_vertex_with_uv(xx, top, za, var17, var21 + 0.0625);
                t.add_vertex_with_uv(xx, top, zb, var17, var19 + 0.0625);
                t.add_vertex_with_uv(xx, bot, zb, var15, var19 + 0.0625);
            }
            // src: RenderBlocks.java:1155-1166 (the -z wall)
            if s.is_block_normal_cube(var2, var3, var4 - 1)
                && s.get_block_id(var2, var3 + 1, var4 - 1) == REDSTONE_WIRE
            {
                let zz = (var4 as f32 + 0.015625) as f64;
                let (xa, xb) = ((var2 + 1) as f64, (var2 + 0) as f64);
                t.set_color_opaque_f(tinted[0], tinted[1], tinted[2]);
                t.add_vertex_with_uv(xa, bot, zz, var15, var21);
                t.add_vertex_with_uv(xa, top, zz, var17, var21);
                t.add_vertex_with_uv(xb, top, zz, var17, var19);
                t.add_vertex_with_uv(xb, bot, zz, var15, var19);
                t.set_color_opaque_f(var8, var8, var8);
                t.add_vertex_with_uv(xa, bot, zz, var15, var21 + 0.0625);
                t.add_vertex_with_uv(xa, top, zz, var17, var21 + 0.0625);
                t.add_vertex_with_uv(xb, top, zz, var17, var19 + 0.0625);
                t.add_vertex_with_uv(xb, bot, zz, var15, var19 + 0.0625);
            }
            // src: RenderBlocks.java:1168-1179 (the +z wall)
            if s.is_block_normal_cube(var2, var3, var4 + 1)
                && s.get_block_id(var2, var3 + 1, var4 + 1) == REDSTONE_WIRE
            {
                let zz = ((var4 + 1) as f32 - 0.015625) as f64;
                let (xa, xb) = ((var2 + 1) as f64, (var2 + 0) as f64);
                t.set_color_opaque_f(tinted[0], tinted[1], tinted[2]);
                t.add_vertex_with_uv(xa, top, zz, var17, var19);
                t.add_vertex_with_uv(xa, bot, zz, var15, var19);
                t.add_vertex_with_uv(xb, bot, zz, var15, var21);
                t.add_vertex_with_uv(xb, top, zz, var17, var21);
                t.set_color_opaque_f(var8, var8, var8);
                t.add_vertex_with_uv(xa, top, zz, var17, var19 + 0.0625);
                t.add_vertex_with_uv(xa, bot, zz, var15, var19 + 0.0625);
                t.add_vertex_with_uv(xb, bot, zz, var15, var21 + 0.0625);
                t.add_vertex_with_uv(xb, top, zz, var17, var21 + 0.0625);
            }
        }
        true
    }

    /// `renderBlockDoor(block, x, y, z)`: render type 7. All six faces of
    /// the door's slab (bounds from `setBlockBoundsBasedOnState`), each lit
    /// by its neighbour (the door's own light for a face inside the cell);
    /// a negative side texture is the mirrored panel (`flipTexture`).
    // src: RenderBlocks.java:2542-2652
    pub fn render_block_door(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let (var2, var3, var4) = (x, y, z);
        let var8 = 0.5f32;
        let var9 = 1.0f32;
        let var10 = 0.8f32;
        let var11 = 0.6f32;
        let lit = blocks::light_value(id) > 0;
        let bb = bounds(&self.statics, id);
        let var12 = self.brightness(id, var2, var3, var4);
        let mut var13 = self.brightness(id, var2, var3 - 1, var4);
        if bb.min_y > 0.0 {
            var13 = var12;
        }
        if lit {
            var13 = 1.0;
        }
        self.tess
            .set_color_opaque_f(var8 * var13, var8 * var13, var8 * var13);
        let t = self.block_texture(id, var2, var3, var4, 0);
        self.render_bottom_face(id, var2 as f64, var3 as f64, var4 as f64, t);
        var13 = self.brightness(id, var2, var3 + 1, var4);
        if bb.max_y < 1.0 {
            var13 = var12;
        }
        if lit {
            var13 = 1.0;
        }
        self.tess
            .set_color_opaque_f(var9 * var13, var9 * var13, var9 * var13);
        let t = self.block_texture(id, var2, var3, var4, 1);
        self.render_top_face(id, var2 as f64, var3 as f64, var4 as f64, t);
        // the four sides: (side, neighbour, face inside the cell, shade)
        for side in 2..6 {
            let (nx, nz, inside, shade) = match side {
                2 => (var2, var4 - 1, bb.min_z > 0.0, var10),
                3 => (var2, var4 + 1, bb.max_z < 1.0, var10),
                4 => (var2 - 1, var4, bb.min_x > 0.0, var11),
                _ => (var2 + 1, var4, bb.max_x < 1.0, var11),
            };
            var13 = self.brightness(id, nx, var3, nz);
            if inside {
                var13 = var12;
            }
            if lit {
                var13 = 1.0;
            }
            self.tess
                .set_color_opaque_f(shade * var13, shade * var13, shade * var13);
            let mut var14 = self.block_texture(id, var2, var3, var4, side);
            if var14 < 0 {
                self.flip_texture = true;
                var14 = -var14;
            }
            let (fx, fy, fz) = (var2 as f64, var3 as f64, var4 as f64);
            match side {
                2 => self.render_east_face(id, fx, fy, fz, var14),
                3 => self.render_west_face(id, fx, fy, fz, var14),
                4 => self.render_north_face(id, fx, fy, fz, var14),
                _ => self.render_south_face(id, fx, fy, fz, var14),
            }
            self.flip_texture = false;
        }
        true
    }

    /// `renderBlockLadder(block, x, y, z)`: render type 8. One quad 0.05
    /// from the wall the metadata names (2 on +z's wall … 5 on −x's).
    // src: RenderBlocks.java:1258-1305
    pub fn render_block_ladder(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let (var2, var3, var4) = (x, y, z);
        let mut var6 = blocks::get_block_texture_from_side(&self.statics, id, 0);
        if self.override_block_texture >= 0 {
            var6 = self.override_block_texture;
        }
        let var7 = self.brightness(id, var2, var3, var4);
        self.tess.set_color_opaque_f(var7, var7, var7);
        let var8 = (var6 & 15) << 4;
        let var9 = var6 & 240;
        let var10 = (var8 as f32 / 256.0) as f64;
        let var12 = ((var8 as f32 + 15.99) / 256.0) as f64;
        let var14 = (var9 as f32 / 256.0) as f64;
        let var16 = ((var9 as f32 + 15.99) / 256.0) as f64;
        let var18 = self.block_access.get_block_metadata(var2, var3, var4);
        let var19 = 0.0f32;
        let var20 = 0.05f32;
        // the corner coordinates, as the Java's float casts
        let x0 = ((var2 + 0) as f32 - var19) as f64;
        let x1 = ((var2 + 1) as f32 + var19) as f64;
        let y0 = ((var3 + 0) as f32 - var19) as f64;
        let y1 = ((var3 + 1) as f32 + var19) as f64;
        let z0 = ((var4 + 0) as f32 - var19) as f64;
        let z1 = ((var4 + 1) as f32 + var19) as f64;
        let t = &mut self.tess;
        if var18 == 5 {
            let xx = (var2 as f32 + var20) as f64;
            t.add_vertex_with_uv(xx, y1, z1, var10, var14);
            t.add_vertex_with_uv(xx, y0, z1, var10, var16);
            t.add_vertex_with_uv(xx, y0, z0, var12, var16);
            t.add_vertex_with_uv(xx, y1, z0, var12, var14);
        }
        if var18 == 4 {
            let xx = ((var2 + 1) as f32 - var20) as f64;
            t.add_vertex_with_uv(xx, y0, z1, var12, var16);
            t.add_vertex_with_uv(xx, y1, z1, var12, var14);
            t.add_vertex_with_uv(xx, y1, z0, var10, var14);
            t.add_vertex_with_uv(xx, y0, z0, var10, var16);
        }
        if var18 == 3 {
            let zz = (var4 as f32 + var20) as f64;
            t.add_vertex_with_uv(x1, y0, zz, var12, var16);
            t.add_vertex_with_uv(x1, y1, zz, var12, var14);
            t.add_vertex_with_uv(x0, y1, zz, var10, var14);
            t.add_vertex_with_uv(x0, y0, zz, var10, var16);
        }
        if var18 == 2 {
            let zz = ((var4 + 1) as f32 - var20) as f64;
            t.add_vertex_with_uv(x1, y1, zz, var10, var14);
            t.add_vertex_with_uv(x1, y0, zz, var10, var16);
            t.add_vertex_with_uv(x0, y0, zz, var12, var16);
            t.add_vertex_with_uv(x0, y1, zz, var12, var14);
        }
        true
    }

    /// `renderBlockMinecartTrack(rail, x, y, z)`: render type 9. One quad
    /// 1/16 above the block, both sides (the second copy reversed): the
    /// straight texture turned for the x-axis shapes (1, 2, 3; 7 is a curve
    /// on the plain rail, whose turned copy it shares), the slopes (2-5)
    /// raising one edge by a block, the curves (6-9) on the texture one row
    /// up (`getBlockTextureFromSideAndMetadata`). The powered and detector
    /// rails drop the powered bit (8) before the shape.
    // src: RenderBlocks.java:1185-1256
    pub fn render_block_minecart_track(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let (var2, var3, var4) = (x, y, z);
        let mut var6 = self.block_access.get_block_metadata(var2, var3, var4);
        let mut var7 = blocks::get_block_texture_from_side_and_metadata(&self.statics, id, 0, var6);
        if self.override_block_texture >= 0 {
            var7 = self.override_block_texture;
        }
        if block::rail::get_is_powered(id) {
            var6 &= 7;
        }
        let var8 = self.brightness(id, var2, var3, var4);
        self.tess.set_color_opaque_f(var8, var8, var8);
        let var9 = (var7 & 15) << 4;
        let var10 = var7 & 240;
        let var11 = (var9 as f32 / 256.0) as f64;
        let var13 = ((var9 as f32 + 15.99) / 256.0) as f64;
        let var15 = (var10 as f32 / 256.0) as f64;
        let var17 = ((var10 as f32 + 15.99) / 256.0) as f64;
        let [px, py, pz] = rail_corners(var6, var2, var3, var4);
        let t = &mut self.tess;
        let v = |i: usize| (px[i] as f64, py[i] as f64, pz[i] as f64);
        let (a, b, c, d) = (v(0), v(1), v(2), v(3));
        t.add_vertex_with_uv(a.0, a.1, a.2, var13, var15);
        t.add_vertex_with_uv(b.0, b.1, b.2, var13, var17);
        t.add_vertex_with_uv(c.0, c.1, c.2, var11, var17);
        t.add_vertex_with_uv(d.0, d.1, d.2, var11, var15);
        t.add_vertex_with_uv(d.0, d.1, d.2, var11, var15);
        t.add_vertex_with_uv(c.0, c.1, c.2, var11, var17);
        t.add_vertex_with_uv(b.0, b.1, b.2, var13, var17);
        t.add_vertex_with_uv(a.0, a.1, a.2, var13, var15);
        true
    }

    /// `renderBlockStairs(block, x, y, z)`: render type 10. Two boxes per
    /// metadata (the low half and the high half, ascending toward +x, −x,
    /// +z, −z for 0-3), each a `renderStandardBlock`; then the full cube
    /// again.
    // src: RenderBlocks.java:2509-2540
    pub fn render_block_stairs(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let mut var5 = false;
        let var6 = self.block_access.get_block_metadata(x, y, z);
        if let Some([lo, hi]) = stairs_boxes(var6) {
            for b in [lo, hi] {
                self.set_block_bounds(id, b[0], b[1], b[2], b[3], b[4], b[5]);
                self.render_standard_block(id, x, y, z);
            }
            var5 = true;
        }
        self.set_block_bounds(id, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
        var5
    }

    /// `renderBlockFence(block, x, y, z)`: render type 11. The post, then
    /// the upper and lower bars along x and/or z toward each neighbour of
    /// the same block id (b1.7.3 fences join only fences); a lone post gets
    /// short bars along x.
    // src: RenderBlocks.java:2446-2507
    pub fn render_block_fence(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let (var2, var3, var4) = (x, y, z);
        let mut var5;
        let mut var6 = 0.375f32;
        let mut var7 = 0.625f32;
        self.set_block_bounds(id, var6, 0.0, var6, var7, 1.0, var7);
        self.render_standard_block(id, var2, var3, var4);
        var5 = true;
        let mut var8 = false;
        let mut var9 = false;
        let s = self.block_access;
        if s.get_block_id(var2 - 1, var3, var4) == id || s.get_block_id(var2 + 1, var3, var4) == id
        {
            var8 = true;
        }
        if s.get_block_id(var2, var3, var4 - 1) == id || s.get_block_id(var2, var3, var4 + 1) == id
        {
            var9 = true;
        }
        let var10 = s.get_block_id(var2 - 1, var3, var4) == id;
        let var11 = s.get_block_id(var2 + 1, var3, var4) == id;
        let var12 = s.get_block_id(var2, var3, var4 - 1) == id;
        let var13 = s.get_block_id(var2, var3, var4 + 1) == id;
        if !var8 && !var9 {
            var8 = true;
        }
        var6 = 0.4375;
        var7 = 0.5625;
        let mut var14 = 0.75f32;
        let mut var15 = 0.9375f32;
        let var16 = if var10 { 0.0 } else { var6 };
        let var17 = if var11 { 1.0 } else { var7 };
        let var18 = if var12 { 0.0 } else { var6 };
        let var19 = if var13 { 1.0 } else { var7 };
        for _ in 0..2 {
            if var8 {
                self.set_block_bounds(id, var16, var14, var6, var17, var15, var7);
                self.render_standard_block(id, var2, var3, var4);
                var5 = true;
            }
            if var9 {
                self.set_block_bounds(id, var6, var14, var18, var7, var15, var19);
                self.render_standard_block(id, var2, var3, var4);
                var5 = true;
            }
            // src: RenderBlocks.java:2491-2492 (the lower bars)
            var14 = 0.375;
            var15 = 0.5625;
        }
        self.set_block_bounds(id, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
        var5
    }

    /// `renderBlockLever(block, x, y, z)`: render type 12. The cobblestone
    /// base (bounds per orientation) through `renderStandardBlock`, then the
    /// stick's six faces ([`lever_vertices`]) from the lever texture's middle
    /// columns.
    // src: RenderBlocks.java:633-794
    pub fn render_block_lever(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let (var2, var3, var4) = (x, y, z);
        let var5 = self.block_access.get_block_metadata(var2, var3, var4);
        let var6 = var5 & 7;
        let var9 = self.override_block_texture >= 0;
        if !var9 {
            self.override_block_texture = COBBLESTONE_TEXTURE;
        }
        let var10 = 0.25f32;
        let var11 = 0.1875f32;
        let var12 = 0.1875f32;
        if var6 == 5 {
            self.set_block_bounds(
                id,
                0.5 - var11,
                0.0,
                0.5 - var10,
                0.5 + var11,
                var12,
                0.5 + var10,
            );
        } else if var6 == 6 {
            self.set_block_bounds(
                id,
                0.5 - var10,
                0.0,
                0.5 - var11,
                0.5 + var10,
                var12,
                0.5 + var11,
            );
        } else if var6 == 4 {
            self.set_block_bounds(
                id,
                0.5 - var11,
                0.5 - var10,
                1.0 - var12,
                0.5 + var11,
                0.5 + var10,
                1.0,
            );
        } else if var6 == 3 {
            self.set_block_bounds(
                id,
                0.5 - var11,
                0.5 - var10,
                0.0,
                0.5 + var11,
                0.5 + var10,
                var12,
            );
        } else if var6 == 2 {
            self.set_block_bounds(
                id,
                1.0 - var12,
                0.5 - var10,
                0.5 - var11,
                1.0,
                0.5 + var10,
                0.5 + var11,
            );
        } else if var6 == 1 {
            self.set_block_bounds(
                id,
                0.0,
                0.5 - var10,
                0.5 - var11,
                var12,
                0.5 + var10,
                0.5 + var11,
            );
        }
        self.render_standard_block(id, var2, var3, var4);
        if !var9 {
            self.override_block_texture = -1;
        }
        let mut var13 = self.brightness(id, var2, var3, var4);
        if blocks::light_value(id) > 0 {
            var13 = 1.0;
        }
        self.tess.set_color_opaque_f(var13, var13, var13);
        let mut var14 = blocks::get_block_texture_from_side(&self.statics, id, 0);
        if self.override_block_texture >= 0 {
            var14 = self.override_block_texture;
        }
        let var15 = (var14 & 15) << 4;
        let var16 = var14 & 240;
        let mut var17 = var15 as f32 / 256.0;
        let mut var18 = (var15 as f32 + 15.99) / 256.0;
        let mut var19 = var16 as f32 / 256.0;
        let mut var20 = (var16 as f32 + 15.99) / 256.0;
        let var21 = lever_vertices(var5, var2, var3, var4);
        for var29 in 0..6 {
            if var29 == 0 {
                var17 = (var15 + 7) as f32 / 256.0;
                var18 = ((var15 + 9) as f32 - 0.01) / 256.0;
                var19 = (var16 + 6) as f32 / 256.0;
                var20 = ((var16 + 8) as f32 - 0.01) / 256.0;
            } else if var29 == 2 {
                var17 = (var15 + 7) as f32 / 256.0;
                var18 = ((var15 + 9) as f32 - 0.01) / 256.0;
                var19 = (var16 + 6) as f32 / 256.0;
                var20 = ((var16 + 16) as f32 - 0.01) / 256.0;
            }
            let [var30, var26, var27, var28] = match var29 {
                0 => [0, 1, 2, 3],
                1 => [7, 6, 5, 4],
                2 => [1, 0, 4, 5],
                3 => [2, 1, 5, 6],
                4 => [3, 2, 6, 7],
                _ => [0, 3, 7, 4],
            }
            .map(|i| var21[i]);
            let t = &mut self.tess;
            let (u0, u1, v0, v1) = (var17 as f64, var18 as f64, var19 as f64, var20 as f64);
            t.add_vertex_with_uv(var30.x_coord, var30.y_coord, var30.z_coord, u0, v1);
            t.add_vertex_with_uv(var26.x_coord, var26.y_coord, var26.z_coord, u1, v1);
            t.add_vertex_with_uv(var27.x_coord, var27.y_coord, var27.z_coord, u1, v0);
            t.add_vertex_with_uv(var28.x_coord, var28.y_coord, var28.z_coord, u0, v0);
        }
        true
    }

    /// `renderBlockBed(block, x, y, z)`: render type 14. The bottom 3/16
    /// above the floor in planks, the top turned per direction, and the
    /// four sides except the one that meets the other half
    /// (`ModelBed.headInvisibleFace`; for the half with bit 8, the
    /// opposite one), one long side drawn mirrored (`flipTexture`,
    /// [`bed_flipped_side`]) so both long sides run the same way.
    // src: RenderBlocks.java:134-274
    pub fn render_block_bed(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let (var2, var3, var4) = (x, y, z);
        let var6 = self.block_access.get_block_metadata(var2, var3, var4);
        let var7 = bed::get_direction_from_metadata(var6);
        let var8 = bed::is_block_foot_of_bed(var6);
        let var9 = 0.5f32;
        let var10 = 1.0f32;
        let var11 = 0.8f32;
        let var12 = 0.6f32;
        let bb = bounds(&self.statics, id);
        let var25 = self.brightness(id, var2, var3, var4);
        self.tess
            .set_color_opaque_f(var9 * var25, var9 * var25, var9 * var25);
        let var26 = self.block_texture(id, var2, var3, var4, 0);
        let var27 = (var26 & 15) << 4;
        let var28 = var26 & 240;
        let var29 = (var27 as f32 / 256.0) as f64;
        let var31 = ((var27 + 16) as f64 - 0.01) / 256.0;
        let var33 = (var28 as f32 / 256.0) as f64;
        let var35 = ((var28 + 16) as f64 - 0.01) / 256.0;
        let var37 = var2 as f64 + bb.min_x;
        let var39 = var2 as f64 + bb.max_x;
        let var41 = var3 as f64 + bb.min_y + 0.1875;
        let var43 = var4 as f64 + bb.min_z;
        let var45 = var4 as f64 + bb.max_z;
        let t = &mut self.tess;
        t.add_vertex_with_uv(var37, var41, var45, var29, var35);
        t.add_vertex_with_uv(var37, var41, var43, var29, var33);
        t.add_vertex_with_uv(var39, var41, var43, var31, var33);
        t.add_vertex_with_uv(var39, var41, var45, var31, var35);
        let var64 = self.brightness(id, var2, var3 + 1, var4);
        self.tess
            .set_color_opaque_f(var10 * var64, var10 * var64, var10 * var64);
        let var27 = self.block_texture(id, var2, var3, var4, 1);
        let [c0, c1, c2, c3] = bed_top_uvs(var27, var7);
        let var54 = var2 as f64 + bb.min_x;
        let var56 = var2 as f64 + bb.max_x;
        let var58 = var3 as f64 + bb.max_y;
        let var60 = var4 as f64 + bb.min_z;
        let var62 = var4 as f64 + bb.max_z;
        let t = &mut self.tess;
        t.add_vertex_with_uv(var56, var58, var62, c0[0], c0[1]);
        t.add_vertex_with_uv(var56, var58, var60, c1[0], c1[1]);
        t.add_vertex_with_uv(var54, var58, var60, c2[0], c2[1]);
        t.add_vertex_with_uv(var54, var58, var62, c3[0], c3[1]);
        let mut var26 = HEAD_INVISIBLE_FACE[var7 as usize];
        if var8 {
            var26 = HEAD_INVISIBLE_FACE[FOOT_INVISIBLE_FACE_REMAP[var7 as usize] as usize];
        }
        let var65 = bed_flipped_side(var7);
        let (fx, fy, fz) = (var2 as f64, var3 as f64, var4 as f64);
        if var26 != 2 && (self.render_all_faces || self.should_side(id, var2, var3, var4 - 1, 2)) {
            let mut var66 = self.brightness(id, var2, var3, var4 - 1);
            if bb.min_z > 0.0 {
                var66 = var25;
            }
            self.tess
                .set_color_opaque_f(var11 * var66, var11 * var66, var11 * var66);
            self.flip_texture = var65 == 2;
            let t = self.block_texture(id, var2, var3, var4, 2);
            self.render_east_face(id, fx, fy, fz, t);
        }
        if var26 != 3 && (self.render_all_faces || self.should_side(id, var2, var3, var4 + 1, 3)) {
            let mut var66 = self.brightness(id, var2, var3, var4 + 1);
            if bb.max_z < 1.0 {
                var66 = var25;
            }
            self.tess
                .set_color_opaque_f(var11 * var66, var11 * var66, var11 * var66);
            self.flip_texture = var65 == 3;
            let t = self.block_texture(id, var2, var3, var4, 3);
            self.render_west_face(id, fx, fy, fz, t);
        }
        if var26 != 4 && (self.render_all_faces || self.should_side(id, var2 - 1, var3, var4, 4)) {
            let mut var66 = self.brightness(id, var2 - 1, var3, var4);
            if bb.min_x > 0.0 {
                var66 = var25;
            }
            self.tess
                .set_color_opaque_f(var12 * var66, var12 * var66, var12 * var66);
            self.flip_texture = var65 == 4;
            let t = self.block_texture(id, var2, var3, var4, 4);
            self.render_north_face(id, fx, fy, fz, t);
        }
        if var26 != 5 && (self.render_all_faces || self.should_side(id, var2 + 1, var3, var4, 5)) {
            let mut var66 = self.brightness(id, var2 + 1, var3, var4);
            if bb.max_x < 1.0 {
                var66 = var25;
            }
            self.tess
                .set_color_opaque_f(var12 * var66, var12 * var66, var12 * var66);
            self.flip_texture = var65 == 5;
            let t = self.block_texture(id, var2, var3, var4, 5);
            self.render_south_face(id, fx, fy, fz, t);
        }
        self.flip_texture = false;
        true
    }

    /// `renderBlockRepeater(block, x, y, z)`: render type 15. The slab
    /// (`renderStandardBlock`: its sides only, `shouldSideBeRendered`), the
    /// two torches ([`repeater_torch_offsets`]) through
    /// `renderTorchAtAngle`, and the top turned to face the metadata's
    /// direction.
    // src: RenderBlocks.java:303-379
    pub fn render_block_repeater(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let (var2, var3, var4) = (x, y, z);
        let var5 = self.block_access.get_block_metadata(var2, var3, var4);
        let var6 = var5 & 3;
        self.render_standard_block(id, var2, var3, var4);
        let mut var9 = self.brightness(id, var2, var3, var4);
        if blocks::light_value(id) > 0 {
            var9 = (var9 + 1.0) * 0.5;
        }
        self.tess.set_color_opaque_f(var9, var9, var9);
        let var10 = -0.1875f64;
        let [[var12, var14], [var16, var18]] = repeater_torch_offsets(var5);
        self.render_torch_at_angle(
            id,
            var2 as f64 + var12,
            var3 as f64 + var10,
            var4 as f64 + var14,
            0.0,
            0.0,
        );
        self.render_torch_at_angle(
            id,
            var2 as f64 + var16,
            var3 as f64 + var10,
            var4 as f64 + var18,
            0.0,
            0.0,
        );
        let var20 = blocks::get_block_texture_from_side(&self.statics, id, 1);
        let var21 = (var20 & 15) << 4;
        let var22 = var20 & 240;
        let var23 = (var21 as f32 / 256.0) as f64;
        let var25 = ((var21 as f32 + 15.99) / 256.0) as f64;
        let var27 = (var22 as f32 / 256.0) as f64;
        let var29 = ((var22 as f32 + 15.99) / 256.0) as f64;
        let var31 = 0.125f32;
        let mut var32 = (var2 + 1) as f32;
        let mut var33 = (var2 + 1) as f32;
        let mut var34 = (var2 + 0) as f32;
        let mut var35 = (var2 + 0) as f32;
        let mut var36 = (var4 + 0) as f32;
        let mut var37 = (var4 + 1) as f32;
        let mut var38 = (var4 + 1) as f32;
        let mut var39 = (var4 + 0) as f32;
        let var40 = var3 as f32 + var31;
        if var6 == 2 {
            var32 = (var2 + 0) as f32;
            var33 = var32;
            var34 = (var2 + 1) as f32;
            var35 = var34;
            var36 = (var4 + 1) as f32;
            var39 = var36;
            var37 = (var4 + 0) as f32;
            var38 = var37;
        } else if var6 == 3 {
            var32 = (var2 + 0) as f32;
            var35 = var32;
            var33 = (var2 + 1) as f32;
            var34 = var33;
            var36 = (var4 + 0) as f32;
            var37 = var36;
            var38 = (var4 + 1) as f32;
            var39 = var38;
        } else if var6 == 1 {
            var32 = (var2 + 1) as f32;
            var35 = var32;
            var33 = (var2 + 0) as f32;
            var34 = var33;
            var36 = (var4 + 1) as f32;
            var37 = var36;
            var38 = (var4 + 0) as f32;
            var39 = var38;
        }
        let t = &mut self.tess;
        let y = var40 as f64;
        t.add_vertex_with_uv(var35 as f64, y, var39 as f64, var23, var27);
        t.add_vertex_with_uv(var34 as f64, y, var38 as f64, var23, var29);
        t.add_vertex_with_uv(var33 as f64, y, var37 as f64, var25, var29);
        t.add_vertex_with_uv(var32 as f64, y, var36 as f64, var25, var27);
        true
    }

    /// Sets the six `uv_rotate_*` fields ([`piston_uv_rotations`]).
    fn set_uv_rotations(&mut self, r: [i32; 6]) {
        self.uv_rotate_east = r[0];
        self.uv_rotate_west = r[1];
        self.uv_rotate_south = r[2];
        self.uv_rotate_north = r[3];
        self.uv_rotate_top = r[4];
        self.uv_rotate_bottom = r[5];
    }

    /// `func_31078_d(block, x, y, z)`: the piston base drawn extended with
    /// every face (the moving piston's renderer, WP-3.5b; later MCP:
    /// `renderPistonBaseAllFaces`).
    // src: RenderBlocks.java:381-385
    pub fn func_31078_d(&mut self, id: i32, x: i32, y: i32, z: i32) {
        self.render_all_faces = true;
        self.func_31074_b(id, x, y, z, true);
        self.render_all_faces = false;
    }

    /// `func_31074_b(block, x, y, z, extended)`: the piston base, render
    /// type 16 (later MCP: `renderPistonBase`). The faces turned to the
    /// facing; extended (the metadata's bit 8, or `extended`), the base is
    /// cut by a quarter on the head side, whose face then shows the inner
    /// texture (110, BlockPistonBase.java:24).
    // src: RenderBlocks.java:387-481
    pub fn func_31074_b(&mut self, id: i32, x: i32, y: i32, z: i32, extended: bool) -> bool {
        let var6 = self.block_access.get_block_metadata(x, y, z);
        let var7 = extended || (var6 & 8) != 0;
        let var8 = piston_base::get_orientation(var6);
        self.set_uv_rotations(piston_uv_rotations(var8));
        if var7 {
            if (0..6).contains(&var8) {
                let b = piston_base_extended_bounds(var8);
                self.set_block_bounds(id, b[0], b[1], b[2], b[3], b[4], b[5]);
            }
            self.render_standard_block(id, x, y, z);
            self.set_uv_rotations([0; 6]);
            self.set_block_bounds(id, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
        } else {
            self.render_standard_block(id, x, y, z);
            self.set_uv_rotations([0; 6]);
        }
        true
    }

    /// The rod quads: one face of the 0.25-thick rod, textured from the
    /// first 4 rows of tile 108 over `len` texels (`var14`: 16 for a full
    /// rod, 8 for the moving piston's half). `func_31076_a` (rods along y),
    /// `func_31081_b` (along z), `func_31077_c` (along x) differ in which
    /// arguments make the four corners.
    // src: RenderBlocks.java:483-541
    fn piston_rod(&mut self, kind: u8, a: [f64; 6], var13: f32, var14: f64) {
        let [var1, var3, var5, var7, var9, var11] = a;
        let mut var16 = PISTON_ROD_TEXTURE;
        if self.override_block_texture >= 0 {
            var16 = self.override_block_texture;
        }
        let var17 = (var16 & 15) << 4;
        let var18 = var16 & 240;
        let var20 = ((var17 + 0) as f32 / 256.0) as f64;
        let var22 = ((var18 + 0) as f32 / 256.0) as f64;
        let var24 = (var17 as f64 + var14 - 0.01) / 256.0;
        let var26 = ((var18 as f32 + 4.0) as f64 - 0.01) / 256.0;
        let t = &mut self.tess;
        t.set_color_opaque_f(var13, var13, var13);
        match kind {
            // src: RenderBlocks.java:497-500 (func_31076_a)
            0 => {
                t.add_vertex_with_uv(var1, var7, var9, var24, var22);
                t.add_vertex_with_uv(var1, var5, var9, var20, var22);
                t.add_vertex_with_uv(var3, var5, var11, var20, var26);
                t.add_vertex_with_uv(var3, var7, var11, var24, var26);
            }
            // src: RenderBlocks.java:517-520 (func_31081_b)
            1 => {
                t.add_vertex_with_uv(var1, var5, var11, var24, var22);
                t.add_vertex_with_uv(var1, var5, var9, var20, var22);
                t.add_vertex_with_uv(var3, var7, var9, var20, var26);
                t.add_vertex_with_uv(var3, var7, var11, var24, var26);
            }
            // src: RenderBlocks.java:537-540 (func_31077_c)
            _ => {
                t.add_vertex_with_uv(var3, var5, var9, var24, var22);
                t.add_vertex_with_uv(var1, var5, var9, var20, var22);
                t.add_vertex_with_uv(var1, var7, var11, var20, var26);
                t.add_vertex_with_uv(var3, var7, var11, var24, var26);
            }
        }
    }

    /// `func_31076_a(x0, x1, y0, y1, z0, z1, shade, len)`: a rod face along y.
    // src: RenderBlocks.java:483-501
    #[allow(clippy::too_many_arguments)]
    pub fn func_31076_a(
        &mut self,
        var1: f64,
        var3: f64,
        var5: f64,
        var7: f64,
        var9: f64,
        var11: f64,
        var13: f32,
        var14: f64,
    ) {
        self.piston_rod(0, [var1, var3, var5, var7, var9, var11], var13, var14);
    }

    /// `func_31081_b(x0, x1, y0, y1, z0, z1, shade, len)`: a rod face along z.
    // src: RenderBlocks.java:503-521
    #[allow(clippy::too_many_arguments)]
    pub fn func_31081_b(
        &mut self,
        var1: f64,
        var3: f64,
        var5: f64,
        var7: f64,
        var9: f64,
        var11: f64,
        var13: f32,
        var14: f64,
    ) {
        self.piston_rod(1, [var1, var3, var5, var7, var9, var11], var13, var14);
    }

    /// `func_31077_c(x0, x1, y0, y1, z0, z1, shade, len)`: a rod face along x.
    // src: RenderBlocks.java:523-541
    #[allow(clippy::too_many_arguments)]
    pub fn func_31077_c(
        &mut self,
        var1: f64,
        var3: f64,
        var5: f64,
        var7: f64,
        var9: f64,
        var11: f64,
        var13: f32,
        var14: f64,
    ) {
        self.piston_rod(2, [var1, var3, var5, var7, var9, var11], var13, var14);
    }

    /// `func_31079_a(block, x, y, z, full_rod)`: the piston head drawn with
    /// every face (the moving piston's renderer, WP-3.5b; later MCP:
    /// `renderPistonExtensionAllFaces`).
    // src: RenderBlocks.java:543-547
    pub fn func_31079_a(&mut self, id: i32, x: i32, y: i32, z: i32, full_rod: bool) {
        self.render_all_faces = true;
        self.func_31080_c(id, x, y, z, full_rod);
        self.render_all_faces = false;
    }

    /// `func_31080_c(block, x, y, z, full_rod)`: the piston head, render
    /// type 17 (later MCP: `renderPistonExtension`). The 0.25-thick head
    /// plate (`renderStandardBlock` with the facing's bounds and texture
    /// rotations; the face texture, normal or sticky, from
    /// `BlockPistonExtension.getBlockTextureFromSideAndMetadata`) and the
    /// four faces of the rod reaching back into the base (a full block with
    /// `full_rod`, half of one for the moving piston).
    // src: RenderBlocks.java:549-631
    pub fn func_31080_c(&mut self, id: i32, x: i32, y: i32, z: i32, full_rod: bool) -> bool {
        let (var2, var3, var4) = (x, y, z);
        let var6 = self.block_access.get_block_metadata(var2, var3, var4);
        let var7 = piston_extension::get_direction_meta(var6);
        let var11 = self.brightness(id, var2, var3, var4);
        let var12: f32 = if full_rod { 1.0 } else { 0.5 };
        let var13: f64 = if full_rod { 16.0 } else { 8.0 };
        let (x, y, z) = (var2 as f32, var3 as f32, var4 as f32);
        let d = |v: f32| v as f64;
        self.set_uv_rotations(piston_uv_rotations(var7));
        match var7 {
            0 => {
                self.set_block_bounds(id, 0.0, 0.0, 0.0, 1.0, 0.25, 1.0);
                self.render_standard_block(id, var2, var3, var4);
                let (y0, y1) = (d(y + 0.25), d(y + 0.25 + var12));
                let (a, b) = (d(x + 0.375), d(x + 0.625));
                let (c, e) = (d(z + 0.375), d(z + 0.625));
                self.func_31076_a(a, b, y0, y1, e, e, var11 * 0.8, var13);
                self.func_31076_a(b, a, y0, y1, c, c, var11 * 0.8, var13);
                self.func_31076_a(a, a, y0, y1, c, e, var11 * 0.6, var13);
                self.func_31076_a(b, b, y0, y1, e, c, var11 * 0.6, var13);
            }
            1 => {
                self.set_block_bounds(id, 0.0, 0.75, 0.0, 1.0, 1.0, 1.0);
                self.render_standard_block(id, var2, var3, var4);
                let (y0, y1) = (d(y - 0.25 + 1.0 - var12), d(y - 0.25 + 1.0));
                let (a, b) = (d(x + 0.375), d(x + 0.625));
                let (c, e) = (d(z + 0.375), d(z + 0.625));
                self.func_31076_a(a, b, y0, y1, e, e, var11 * 0.8, var13);
                self.func_31076_a(b, a, y0, y1, c, c, var11 * 0.8, var13);
                self.func_31076_a(a, a, y0, y1, c, e, var11 * 0.6, var13);
                self.func_31076_a(b, b, y0, y1, e, c, var11 * 0.6, var13);
            }
            2 => {
                self.set_block_bounds(id, 0.0, 0.0, 0.0, 1.0, 1.0, 0.25);
                self.render_standard_block(id, var2, var3, var4);
                let (z0, z1) = (d(z + 0.25), d(z + 0.25 + var12));
                let (a, b) = (d(x + 0.375), d(x + 0.625));
                let (c, e) = (d(y + 0.375), d(y + 0.625));
                self.func_31081_b(a, a, e, c, z0, z1, var11 * 0.6, var13);
                self.func_31081_b(b, b, c, e, z0, z1, var11 * 0.6, var13);
                self.func_31081_b(a, b, c, c, z0, z1, var11 * 0.5, var13);
                self.func_31081_b(b, a, e, e, z0, z1, var11, var13);
            }
            3 => {
                self.set_block_bounds(id, 0.0, 0.0, 0.75, 1.0, 1.0, 1.0);
                self.render_standard_block(id, var2, var3, var4);
                let (z0, z1) = (d(z - 0.25 + 1.0 - var12), d(z - 0.25 + 1.0));
                let (a, b) = (d(x + 0.375), d(x + 0.625));
                let (c, e) = (d(y + 0.375), d(y + 0.625));
                self.func_31081_b(a, a, e, c, z0, z1, var11 * 0.6, var13);
                self.func_31081_b(b, b, c, e, z0, z1, var11 * 0.6, var13);
                self.func_31081_b(a, b, c, c, z0, z1, var11 * 0.5, var13);
                self.func_31081_b(b, a, e, e, z0, z1, var11, var13);
            }
            4 => {
                self.set_block_bounds(id, 0.0, 0.0, 0.0, 0.25, 1.0, 1.0);
                self.render_standard_block(id, var2, var3, var4);
                let (x0, x1) = (d(x + 0.25), d(x + 0.25 + var12));
                let (a, b) = (d(y + 0.375), d(y + 0.625));
                let (c, e) = (d(z + 0.375), d(z + 0.625));
                self.func_31077_c(x0, x1, a, a, e, c, var11 * 0.5, var13);
                self.func_31077_c(x0, x1, b, b, c, e, var11, var13);
                self.func_31077_c(x0, x1, a, b, c, c, var11 * 0.6, var13);
                self.func_31077_c(x0, x1, b, a, e, e, var11 * 0.6, var13);
            }
            5 => {
                self.set_block_bounds(id, 0.75, 0.0, 0.0, 1.0, 1.0, 1.0);
                self.render_standard_block(id, var2, var3, var4);
                let (x0, x1) = (d(x - 0.25 + 1.0 - var12), d(x - 0.25 + 1.0));
                let (a, b) = (d(y + 0.375), d(y + 0.625));
                let (c, e) = (d(z + 0.375), d(z + 0.625));
                self.func_31077_c(x0, x1, a, a, e, c, var11 * 0.5, var13);
                self.func_31077_c(x0, x1, b, b, c, e, var11, var13);
                self.func_31077_c(x0, x1, a, b, c, c, var11 * 0.6, var13);
                self.func_31077_c(x0, x1, b, a, e, e, var11 * 0.6, var13);
            }
            _ => {}
        }
        self.set_uv_rotations([0; 6]);
        self.set_block_bounds(id, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
        true
    }
}

/// The stairs' two boxes `[(x0, y0, z0, x1, y1, z1); 2]` for metadata
/// `meta` (0-3; `None` for any other: nothing drawn).
// src: RenderBlocks.java:2512-2536
pub const fn stairs_boxes(meta: i32) -> Option<[[f32; 6]; 2]> {
    match meta {
        0 => Some([
            [0.0, 0.0, 0.0, 0.5, 0.5, 1.0],
            [0.5, 0.0, 0.0, 1.0, 1.0, 1.0],
        ]),
        1 => Some([
            [0.0, 0.0, 0.0, 0.5, 1.0, 1.0],
            [0.5, 0.0, 0.0, 1.0, 0.5, 1.0],
        ]),
        2 => Some([
            [0.0, 0.0, 0.0, 1.0, 0.5, 0.5],
            [0.0, 0.0, 0.5, 1.0, 1.0, 1.0],
        ]),
        3 => Some([
            [0.0, 0.0, 0.0, 1.0, 1.0, 0.5],
            [0.0, 0.0, 0.5, 1.0, 0.5, 1.0],
        ]),
        _ => None,
    }
}

/// The rail quad's four corners for shape `meta` (after the powered rails'
/// `& 7`) at block `(x, y, z)`: `[xs, ys, zs]` of `var20..23`, `var28..31`,
/// `var24..27`. Shapes 1, 2, 3 and 7 turn the texture a quarter, 8 and 9
/// the other two ways (6 and 0 keep it); 2 and 4 raise corners 0 and 3, 3
/// and 5 corners 1 and 2, by a block.
// src: RenderBlocks.java:1205-1245
pub fn rail_corners(meta: i32, x: i32, y: i32, z: i32) -> [[f32; 4]; 3] {
    let var6 = meta;
    let var19 = 0.0625f32;
    let mut var20 = (x + 1) as f32;
    let mut var21 = (x + 1) as f32;
    let mut var22 = (x + 0) as f32;
    let mut var23 = (x + 0) as f32;
    let mut var24 = (z + 0) as f32;
    let mut var25 = (z + 1) as f32;
    let mut var26 = (z + 1) as f32;
    let mut var27 = (z + 0) as f32;
    let mut var28 = y as f32 + var19;
    let mut var29 = y as f32 + var19;
    let mut var30 = y as f32 + var19;
    let mut var31 = y as f32 + var19;
    if var6 != 1 && var6 != 2 && var6 != 3 && var6 != 7 {
        if var6 == 8 {
            var20 = (x + 0) as f32;
            var21 = var20;
            var22 = (x + 1) as f32;
            var23 = var22;
            var24 = (z + 1) as f32;
            var27 = var24;
            var25 = (z + 0) as f32;
            var26 = var25;
        } else if var6 == 9 {
            var20 = (x + 0) as f32;
            var23 = var20;
            var21 = (x + 1) as f32;
            var22 = var21;
            var24 = (z + 0) as f32;
            var25 = var24;
            var26 = (z + 1) as f32;
            var27 = var26;
        }
    } else {
        var20 = (x + 1) as f32;
        var23 = var20;
        var21 = (x + 0) as f32;
        var22 = var21;
        var24 = (z + 1) as f32;
        var25 = var24;
        var26 = (z + 0) as f32;
        var27 = var26;
    }
    if var6 != 2 && var6 != 4 {
        if var6 == 3 || var6 == 5 {
            var29 += 1.0;
            var30 += 1.0;
        }
    } else {
        var28 += 1.0;
        var31 += 1.0;
    }
    [
        [var20, var21, var22, var23],
        [var28, var29, var30, var31],
        [var24, var25, var26, var27],
    ]
}

/// The bed top's four vertex UVs for texture `tex` and direction `var7`
/// (`var46/50`, `var38/42`, `var40/44`, `var48/52`, in drawing order): the
/// tile turned so the pillow lies at the head.
// src: RenderBlocks.java:163-207
pub fn bed_top_uvs(tex: i32, var7: i32) -> [[f64; 2]; 4] {
    let var27 = tex;
    let var28 = (var27 & 15) << 4;
    let var67 = var27 & 240;
    let var30 = (var28 as f32 / 256.0) as f64;
    let var32 = ((var28 + 16) as f64 - 0.01) / 256.0;
    let var34 = (var67 as f32 / 256.0) as f64;
    let var36 = ((var67 + 16) as f64 - 0.01) / 256.0;
    let mut var38 = var30;
    let mut var40 = var32;
    let mut var42 = var34;
    let mut var44 = var34;
    let mut var46 = var30;
    let mut var48 = var32;
    let mut var50 = var36;
    let mut var52 = var36;
    if var7 == 0 {
        var40 = var30;
        var42 = var36;
        var46 = var32;
        var52 = var34;
    } else if var7 == 2 {
        var38 = var32;
        var44 = var36;
        var48 = var30;
        var50 = var34;
    } else if var7 == 3 {
        var38 = var32;
        var44 = var36;
        var48 = var30;
        var50 = var34;
        var40 = var30;
        var42 = var36;
        var46 = var32;
        var52 = var34;
    }
    [
        [var46, var50],
        [var38, var42],
        [var40, var44],
        [var48, var52],
    ]
}

/// The bed side whose texture is mirrored (`var65`): a long side, 5, 3, 4
/// or 2 for directions 0-3.
// src: RenderBlocks.java:213-225
pub const fn bed_flipped_side(var7: i32) -> i32 {
    // vanilla's switch falls through from case 1 into case 2/default,
    // which only breaks: 1 → 3.
    match var7 {
        0 => 5,
        1 => 3,
        3 => 2,
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::tessellator::Tessellator;
    use mc_sim::block::BlockBounds;
    use mc_sim::world::provider::{Dimension, WorldProvider};

    const STONE: i32 = 1;
    const DOOR: i32 = 64;
    const LADDER: i32 = 65;
    const RAIL: i32 = 66;
    const RAIL_POWERED: i32 = 27;
    const RAIL_DETECTOR: i32 = 28;
    const STAIRS: i32 = 53;
    const FENCE: i32 = 85;
    const LEVER: i32 = 69;
    const BED: i32 = 26;
    const PISTON: i32 = 33;
    const STICKY: i32 = 29;
    const HEAD: i32 = 34;
    const TORCH_ON: i32 = 76;

    fn snap() -> ChunkSnapshot {
        ChunkSnapshot::synthetic(
            [-3, 0, -3],
            [5, 8, 5],
            WorldProvider::new(Dimension::Surface).light_brightness_table,
        )
    }

    fn rb(s: &ChunkSnapshot) -> RenderBlocks<'_> {
        RenderBlocks::new(s, s.statics.clone(), false, false)
    }

    fn quads(t: &Tessellator) -> usize {
        t.len() / 4
    }

    // ---- redstone wire -------------------------------------------------------------

    /// The wire's connections over a 3×3 pattern of wires on a stone floor
    /// (and against walls), with the texture choice they give.
    #[test]
    fn redstone_wire_connection_table() {
        // (neighbour wires relative to (1,1,1), expected [-x,+x,-z,+z], shape)
        type Case<'a> = (&'a [(i32, i32, i32)], [bool; 4], i32);
        let cases: [Case; 7] = [
            (&[], [false; 4], 0),
            (
                &[(-1, 0, 0), (1, 0, 0), (0, 0, -1), (0, 0, 1)],
                [true; 4],
                0,
            ),
            (&[(0, 0, -1), (0, 0, 1)], [false, false, true, true], 2),
            (&[(-1, 0, 0), (1, 0, 0)], [true, true, false, false], 1),
            (&[(1, 0, 0)], [false, true, false, false], 1),
            (&[(1, 0, 0), (0, 0, 1)], [false, true, false, true], 0),
            // one block down past an air gap
            (&[(0, -1, 1)], [false, false, false, true], 2),
        ];
        for (wires, expect, shape) in cases {
            let mut s = snap();
            for x in -1..4 {
                for z in -1..4 {
                    s.set(x, 0, z, STONE, 0, 15, 0);
                }
            }
            s.set(1, 0, 2, 0, 0, 15, 0); // the hole for the down case
            s.set(1, 1, 1, REDSTONE_WIRE, 0, 15, 0);
            for &(dx, dy, dz) in wires {
                s.set(1 + dx, 1 + dy, 1 + dz, REDSTONE_WIRE, 0, 15, 0);
            }
            let c = wire_connections(&s, 1, 1, 1);
            assert_eq!(c, expect, "{wires:?}");
            assert_eq!(wire_shape(c), shape, "{wires:?}");
        }
        // up the wall: a stone block at +x with wire on top, air above
        let mut s = snap();
        s.set(1, 0, 1, STONE, 0, 15, 0);
        s.set(1, 1, 1, REDSTONE_WIRE, 0, 15, 0);
        s.set(2, 1, 1, STONE, 0, 15, 0);
        s.set(2, 2, 1, REDSTONE_WIRE, 0, 15, 0);
        assert_eq!(wire_connections(&s, 1, 1, 1), [false, true, false, false]);
        let mut r = rb(&s);
        r.render_block_redstone_wire(REDSTONE_WIRE, 1, 1, 1);
        // the flat quad and its overlay, then the +x wall's two quads
        assert_eq!(quads(&r.tess), 4);
        let xs: Vec<f32> = r.tess.positions[8..16].iter().map(|p| p[0]).collect();
        assert!(xs.iter().all(|&x| x == 2.0 - 0.015625), "{xs:?}");
        assert_eq!(r.tess.positions[9][1], 2.0f32 + 0.021875);
        // a solid block above cuts the wall piece and the upward link
        s.set(1, 2, 1, STONE, 0, 15, 0);
        assert_eq!(wire_connections(&s, 1, 1, 1), [false; 4]);
        let mut r = rb(&s);
        r.render_block_redstone_wire(REDSTONE_WIRE, 1, 1, 1);
        assert_eq!(quads(&r.tess), 2);
        // a repeater connects only along its axis; a torch always
        let mut s = snap();
        s.set(1, 1, 1, REDSTONE_WIRE, 0, 15, 0);
        s.set(2, 1, 1, REDSTONE_REPEATER_IDLE, 1, 15, 0); // facing ±x: remap[1] = 3
        s.set(1, 1, 2, REDSTONE_REPEATER_IDLE, 1, 15, 0); // wants 0, gets 3
        s.set(0, 1, 1, TORCH_ON, 5, 15, 0);
        assert_eq!(wire_connections(&s, 1, 1, 1), [true, true, false, false]);
    }

    #[test]
    fn wire_colour_at_power_0_and_15() {
        assert_eq!(wire_color(0), [0.3, 0.0, 0.0]);
        let [r, g, b] = wire_color(15);
        // 15/15 = 1: 0.6 + 0.4, 0.7 - 0.5, 0.6 - 0.7 → 0
        assert_eq!(r, 1.0f32 * 0.6 + 0.4);
        assert_eq!(g, 1.0f32 * 1.0 * 0.7 - 0.5);
        assert_eq!(b, 0.0);
        // power 10: green still 0 (100/225 × 0.7 < 0.5)
        let p = 10.0f32 / 15.0;
        assert_eq!(wire_color(10), [p * 0.6 + 0.4, 0.0, 0.0]);
        // the vertex bytes at full light and power 15
        let mut s = snap();
        s.set(0, 1, 0, REDSTONE_WIRE, 15, 15, 0);
        let mut r = rb(&s);
        r.render_block_redstone_wire(REDSTONE_WIRE, 0, 1, 0);
        // 1.0 × 0.7 - 0.5 is 0.19999999 in float: 50, not 51
        assert_eq!(r.tess.colors[0], [255, 50, 0, 255]);
        assert_eq!(r.tess.colors[4], [255, 255, 255, 255]);
        // the overlay is one tile row down
        assert_eq!(r.tess.uvs[4][1], (r.tess.uvs[0][1] as f64 + 0.0625) as f32);
    }

    #[test]
    fn wire_cross_arms_are_cut() {
        let mut s = snap();
        s.set(0, 1, 0, REDSTONE_WIRE, 0, 15, 0);
        s.set(1, 1, 0, REDSTONE_WIRE, 0, 15, 0);
        s.set(0, 1, 1, REDSTONE_WIRE, 0, 15, 0);
        let mut r = rb(&s);
        r.render_block_redstone_wire(REDSTONE_WIRE, 0, 1, 0);
        // a corner: x from 0.3125 to 1, z from 0.3125 to 1
        let p = &r.tess.positions;
        assert_eq!(p[0], [1.0, 1.015625, 1.0]);
        assert_eq!(p[2], [0.3125, 1.015625, 0.3125]);
        // tile 164: u 64/256 + 0.01953125 (5 texels) at the cut sides
        assert_eq!(r.tess.uvs[2][0], (64.0f64 / 256.0 + 0.01953125) as f32);
    }

    // ---- door ----------------------------------------------------------------------

    /// The door's side textures for the eight bottom-half metadatas, as
    /// `BlockDoor.getBlockTextureFromSideAndMetadata` computes them
    /// (`getState`, BlockDoor.java:18-36, :184-186): one side per metadata
    /// is the mirrored panel (negative), and the renderer draws it with the
    /// u range swapped (`flipTexture`).
    #[test]
    fn door_flip_table() {
        // the mirrored side per metadata, worked out by hand from the Java
        // expression; every other side is +97
        let mirrored = [5, 3, 4, 2, 2, 5, 3, 4];
        let st = snap().statics;
        for meta in 0..8 {
            for side in 0..6 {
                let t = blocks::get_block_texture_from_side_and_metadata(&st, DOOR, side, meta);
                let want = if side == mirrored[meta as usize] {
                    -97
                } else {
                    97
                };
                assert_eq!(t, want, "meta {meta} side {side}");
            }
            // the top half (meta | 8): the same sides, tile 97 - 16
            let t = blocks::get_block_texture_from_side_and_metadata(
                &st,
                DOOR,
                mirrored[meta as usize],
                meta | 8,
            );
            assert_eq!(t, -81, "meta {meta} top");
        }
        // drawn: the east face (side 2, the third quad) is mirrored for
        // meta 3 and not for meta 0 (the bounds do not matter here)
        let east = |meta| {
            let mut s = snap();
            s.set(0, 1, 0, DOOR, meta, 15, 0);
            let mut r = rb(&s);
            r.set_block_bounds(DOOR, 0.0, 0.0, 0.0, 1.0, 1.0, 0.1875);
            r.render_block_door(DOOR, 0, 1, 0);
            assert_eq!(quads(&r.tess), 6);
            assert!(!r.flip_texture);
            r.tess.uvs[8..12].to_vec()
        };
        let (plain, flipped) = (east(0), east(3));
        let (u0, u1) = (
            (16.0f64 / 256.0) as f32,
            ((16.0 + 16.0 - 0.01) / 256.0) as f32,
        );
        // renderEastFace's first vertex takes the far u (var20 = var14)
        assert_eq!([plain[0][0], plain[1][0]], [u1, u0]);
        assert_eq!([flipped[0][0], flipped[1][0]], [u0, u1]);
    }

    // ---- ladder ---------------------------------------------------------------------

    #[test]
    fn ladder_orientations() {
        for (meta, axis, at) in [
            (2, 2, 2.0 - 0.05),
            (3, 2, 1.05),
            (4, 0, 1.0 - 0.05),
            (5, 0, 0.05),
        ] {
            let mut s = snap();
            s.set(0, 1, 1, LADDER, meta, 15, 0);
            let mut r = rb(&s);
            r.render_block_ladder(LADDER, 0, 1, 1);
            assert_eq!(quads(&r.tess), 1, "meta {meta}");
            for p in &r.tess.positions {
                assert_eq!(p[axis], at as f32, "meta {meta}");
            }
        }
        let mut s = snap();
        s.set(0, 1, 1, LADDER, 0, 15, 0);
        let mut r = rb(&s);
        r.render_block_ladder(LADDER, 0, 1, 1);
        assert!(r.tess.is_empty());
    }

    // ---- rails ----------------------------------------------------------------------

    #[test]
    fn rail_geometry_straight_curved_sloped() {
        let y = 1.0625f32;
        // straight along z (0): texture upright
        assert_eq!(
            rail_corners(0, 0, 1, 0),
            [[1.0, 1.0, 0.0, 0.0], [y; 4], [0.0, 1.0, 1.0, 0.0]]
        );
        // straight along x (1): turned
        assert_eq!(
            rail_corners(1, 0, 1, 0),
            [[1.0, 0.0, 0.0, 1.0], [y; 4], [1.0, 1.0, 0.0, 0.0]]
        );
        // ascending +x (2): corners 0 and 3 (x = 1) raised
        assert_eq!(
            rail_corners(2, 0, 1, 0),
            [
                [1.0, 0.0, 0.0, 1.0],
                [y + 1.0, y, y, y + 1.0],
                [1.0, 1.0, 0.0, 0.0]
            ]
        );
        // ascending -x (3): corners 1 and 2 (x = 0) raised
        assert_eq!(rail_corners(3, 0, 1, 0)[1], [y, y + 1.0, y + 1.0, y]);
        // ascending -z (4) and +z (5)
        assert_eq!(
            rail_corners(4, 0, 1, 0),
            [
                [1.0, 1.0, 0.0, 0.0],
                [y + 1.0, y, y, y + 1.0],
                [0.0, 1.0, 1.0, 0.0]
            ]
        );
        assert_eq!(rail_corners(5, 0, 1, 0)[1], [y, y + 1.0, y + 1.0, y]);
        // curves: 6 as the plain layout, 7 turned, 8 and 9 the other ways
        assert_eq!(rail_corners(6, 0, 1, 0), rail_corners(0, 0, 1, 0));
        assert_eq!(rail_corners(7, 0, 1, 0), rail_corners(1, 0, 1, 0));
        assert_eq!(
            rail_corners(8, 0, 1, 0),
            [[0.0, 0.0, 1.0, 1.0], [y; 4], [1.0, 0.0, 0.0, 1.0]]
        );
        assert_eq!(
            rail_corners(9, 0, 1, 0),
            [[0.0, 1.0, 1.0, 0.0], [y; 4], [0.0, 0.0, 1.0, 1.0]]
        );
    }

    #[test]
    fn rail_textures_and_the_powered_bit() {
        // plain rail: curves (>= 6) on the row above (128 - 16 = 112)
        let mut s = snap();
        s.set(0, 1, 0, RAIL, 6, 15, 0);
        s.set(1, 1, 0, RAIL_POWERED, 1 | 8, 15, 0);
        s.set(2, 1, 0, RAIL_POWERED, 1, 15, 0);
        s.set(3, 1, 0, RAIL_DETECTOR, 9, 15, 0);
        let mut r = rb(&s);
        r.render_block_minecart_track(RAIL, 0, 1, 0);
        let tile = |t: &Tessellator| {
            let u = (t.uvs[3][0] * 256.0).round() as i32;
            let v = (t.uvs[0][1] * 256.0).round() as i32;
            (v & 240) | (u >> 4)
        };
        assert_eq!(tile(&r.tess), 112);
        assert_eq!(quads(&r.tess), 2);
        // powered and on: 179, the shape is meta & 7 = 1 (turned)
        let mut r = rb(&s);
        r.render_block_minecart_track(RAIL_POWERED, 1, 1, 0);
        assert_eq!(tile(&r.tess), 179);
        assert_eq!(r.tess.positions[0], [2.0, 1.0625, 1.0]);
        // powered and off: 163
        let mut r = rb(&s);
        r.render_block_minecart_track(RAIL_POWERED, 2, 1, 0);
        assert_eq!(tile(&r.tess), 163);
        // detector, meta 9 (pressed, shape 1): 195 always
        let mut r = rb(&s);
        r.render_block_minecart_track(RAIL_DETECTOR, 3, 1, 0);
        assert_eq!(tile(&r.tess), 195);
        assert_eq!(r.tess.positions[0], [4.0, 1.0625, 1.0]);
        // both sides: the second quad is the first reversed
        assert_eq!(r.tess.positions[0], r.tess.positions[7]);
        assert_eq!(r.tess.positions[1], r.tess.positions[6]);
    }

    // ---- stairs, fence ---------------------------------------------------------------

    #[test]
    fn stairs_bounds_per_metadata() {
        for meta in 0..4 {
            let [lo, hi] = stairs_boxes(meta).unwrap();
            // the low box is a half-height half, the high box full height
            let low = if meta == 0 || meta == 2 { lo } else { hi };
            let high = if meta == 0 || meta == 2 { hi } else { lo };
            assert_eq!(low[4], 0.5, "{meta}");
            assert_eq!(high[4], 1.0, "{meta}");
        }
        // 0 ascends toward +x, 1 toward -x, 2 toward +z, 3 toward -z
        assert_eq!(stairs_boxes(0).unwrap()[1][0], 0.5);
        assert_eq!(stairs_boxes(1).unwrap()[0][3], 0.5);
        assert_eq!(stairs_boxes(2).unwrap()[1][2], 0.5);
        assert_eq!(stairs_boxes(3).unwrap()[0][5], 0.5);
        assert_eq!(stairs_boxes(4), None);
        // the renderer: two boxes, then full bounds again
        let mut s = snap();
        s.set(0, 1, 0, STAIRS, 0, 15, 0);
        let mut r = rb(&s);
        assert!(r.render_block_stairs(STAIRS, 0, 1, 0));
        assert_eq!(bounds(&r.statics, STAIRS), BlockBounds::FULL);
        // box 1: 6 faces (all inside or against air); box 2: 6
        assert_eq!(quads(&r.tess), 12);
        let mut s = snap();
        s.set(0, 1, 0, STAIRS, 5, 15, 0);
        let mut r = rb(&s);
        assert!(!r.render_block_stairs(STAIRS, 0, 1, 0));
        assert!(r.tess.is_empty());
    }

    #[test]
    fn fence_connections() {
        // a lone post: the post (6 faces) and short x bars (2 × 6)
        let mut s = snap();
        s.set(1, 1, 1, FENCE, 0, 15, 0);
        let mut r = rb(&s);
        r.render_block_fence(FENCE, 1, 1, 1);
        assert_eq!(quads(&r.tess), 18);
        assert_eq!(bounds(&r.statics, FENCE), BlockBounds::FULL);
        let min_x = r.tess.positions.iter().map(|p| p[0]).fold(9.0, f32::min);
        assert_eq!(min_x, 1.375); // the bars stay inside the post's x range
        // a neighbour at -x: bars reach x = 0 (the cell edge), along x only
        s.set(0, 1, 1, FENCE, 0, 15, 0);
        let mut r = rb(&s);
        r.render_block_fence(FENCE, 1, 1, 1);
        let min_x = r.tess.positions.iter().map(|p| p[0]).fold(9.0, f32::min);
        let max_x = r.tess.positions.iter().map(|p| p[0]).fold(-9.0, f32::max);
        let max_z = r.tess.positions.iter().map(|p| p[2]).fold(-9.0, f32::max);
        assert_eq!((min_x, max_x, max_z), (1.0, 1.625, 1.625));
        // neighbours on x and z: four bars (2 heights × 2 axes)
        s.set(1, 1, 2, FENCE, 0, 15, 0);
        let mut r = rb(&s);
        r.render_block_fence(FENCE, 1, 1, 1);
        let max_z = r.tess.positions.iter().map(|p| p[2]).fold(-9.0, f32::max);
        assert_eq!(max_z, 2.0);
        // the bars' heights: 0.75-0.9375 and 0.375-0.5625
        let ys: std::collections::BTreeSet<u32> = r
            .tess
            .positions
            .iter()
            .map(|p| (p[1] * 10000.0) as u32)
            .collect();
        for y in [1.75f32, 1.9375, 1.375, 1.5625] {
            assert!(ys.contains(&((y * 10000.0) as u32)), "{y}");
        }
        // a stone neighbour does not connect
        let mut s = snap();
        s.set(1, 1, 1, FENCE, 0, 15, 0);
        s.set(1, 1, 2, STONE, 0, 15, 0);
        let mut r = rb(&s);
        r.render_block_fence(FENCE, 1, 1, 1);
        let max_z = r.tess.positions.iter().map(|p| p[2]).fold(-9.0, f32::max);
        assert_eq!(max_z, 1.625);
    }

    // ---- lever ----------------------------------------------------------------------

    #[test]
    fn lever_positions_per_metadata() {
        // the stick's base centre ((v0 + v2) / 2) and top centre ((v4 + v6) / 2)
        let centre = |v: &[Vec3D; 8], a: usize, b: usize| {
            [
                (v[a].x_coord + v[b].x_coord) / 2.0,
                (v[a].y_coord + v[b].y_coord) / 2.0,
                (v[a].z_coord + v[b].z_coord) / 2.0,
            ]
        };
        let close = |a: [f64; 3], b: [f64; 3]| (0..3).all(|i| (a[i] - b[i]).abs() < 1e-6);
        // floor, off (5): the base at (0.5, 0.125, 0.5 + 0.0625 rotated)
        for meta in [5, 6, 13, 14] {
            let v = lever_vertices(meta, 0, 0, 0);
            let base = centre(&v, 0, 2);
            let top = centre(&v, 4, 6);
            assert!(base[1] < 0.2 && top[1] > 0.5, "{meta}: {base:?} {top:?}");
            // tilted ±40° toward z (5) or x (6); on flips it
            let lean = if meta & 7 == 5 {
                top[2] - base[2]
            } else {
                top[0] - base[0]
            };
            let on = meta & 8 != 0;
            assert!(
                (lean > 0.0) == (meta & 7 == 5) ^ on ^ (meta & 7 == 6),
                "{meta}: {lean}"
            );
        }
        // wall levers: the base sits at the wall named by the metadata
        // (1: -x wall, 2: +x, 3: -z, 4: +z), centred at y 0.5
        for (meta, axis, wall) in [(1, 0, 0.0), (2, 0, 1.0), (3, 2, 0.0), (4, 2, 1.0)] {
            let v = lever_vertices(meta, 0, 0, 0);
            let base = centre(&v, 0, 2);
            let top = centre(&v, 4, 6);
            assert!((base[axis] - wall).abs() < 0.2, "{meta}: {base:?}");
            assert!(
                (top[axis] - wall).abs() > (base[axis] - wall).abs(),
                "{meta}"
            );
            assert!((base[1] - 0.5).abs() < 0.2, "{meta}: {base:?}");
        }
        // exact: meta 5 off, vertex 0 = (-1/16, 0, -1/16 + 1/16) rotated by
        // -0.69813174 about x with the table, then moved to (0.5, 0.125, 0.5)
        let v = lever_vertices(5, 0, 0, 0);
        let mut e = Vec3D::create_vector(-0.0625, 0.0, 0.0);
        e.rotate_around_x(-0.69813174);
        assert!(close(
            [v[0].x_coord, v[0].y_coord, v[0].z_coord],
            [e.x_coord + 0.5, e.y_coord + 0.125, e.z_coord + 0.5]
        ));
        // drawn: the cobblestone base (bounds for 5; its bottom on the stone
        // is culled) and 6 stick faces
        let mut s = snap();
        s.set(0, 1, 0, STONE, 0, 15, 0);
        s.set(0, 2, 0, LEVER, 5, 15, 0);
        let mut r = rb(&s);
        r.render_block_lever(LEVER, 0, 2, 0);
        assert_eq!(quads(&r.tess), 5 + 6);
        assert_eq!(r.override_block_texture, -1);
        let bb = bounds(&r.statics, LEVER);
        assert_eq!(bb.max_y, 0.1875f32 as f64);
        assert_eq!(bb.min_z, (0.5f32 - 0.25) as f64);
    }

    // ---- bed ------------------------------------------------------------------------

    #[test]
    fn bed_textures_per_direction() {
        // metadata → textures of sides 1 (top), 2, 3, 4, 5. MCP's
        // `isBlockFootOfBed` (bit 8) is the pillow half: `ItemBed` places the
        // block with `dir` where clicked and `dir + 8` one step along the
        // direction (ItemBed.java:14-35); the renderer only asks the bit.
        let st = snap().statics;
        let tex = |meta: i32| -> Vec<i32> {
            (1..6)
                .map(|side| blocks::get_block_texture_from_side_and_metadata(&st, BED, side, meta))
                .collect()
        };
        // bit 8 clear, direction 0: bedDirection[0] = {1, 0, 3, 2, 5, 4}:
        // side 2 → 3 (the end with the legs, 134 - 1 + 16 = 149), side 3 →
        // 2 (134, hidden against the other half), sides 4/5 → 5/4 (150)
        assert_eq!(tex(0), vec![134, 149, 134, 150, 150]);
        // bit 8 set (the pillow), direction 0: side 3 → 2 (the end, 134 +
        // 2 + 16 = 152)
        assert_eq!(tex(8), vec![135, 135, 152, 151, 151]);
        assert_eq!(tex(1), vec![134, 150, 150, 134, 149]);
        assert_eq!(tex(2), vec![134, 134, 149, 150, 150]);
        assert_eq!(tex(3), vec![134, 150, 150, 149, 134]);
        // the hidden side (toward the other half) and the mirrored end
        assert_eq!(HEAD_INVISIBLE_FACE, [3, 4, 2, 5]);
        assert_eq!(
            [0, 1, 2, 3].map(|d| HEAD_INVISIBLE_FACE[FOOT_INVISIBLE_FACE_REMAP[d] as usize]),
            [2, 5, 3, 4]
        );
        assert_eq!([0, 1, 2, 3].map(bed_flipped_side), [5, 3, 4, 2]);
        // the top's UVs: direction 2 keeps the tile upright, 0 turns it
        let t = tile_uv(134);
        assert_eq!(
            bed_top_uvs(134, 1),
            [[t.0, t.3], [t.0, t.2], [t.1, t.2], [t.1, t.3]]
        );
        assert_eq!(
            bed_top_uvs(134, 0),
            [[t.1, t.3], [t.0, t.3], [t.0, t.2], [t.1, t.2]]
        );
        assert_eq!(
            bed_top_uvs(134, 2),
            [[t.0, t.2], [t.1, t.2], [t.1, t.3], [t.0, t.3]]
        );
        assert_eq!(
            bed_top_uvs(134, 3),
            [[t.1, t.2], [t.1, t.3], [t.0, t.3], [t.0, t.2]]
        );
        // drawn: bottom, top and three sides (+z, side 3, meets the other
        // half: headInvisibleFace[0])
        let mut s = snap();
        s.set(0, 1, 0, BED, 0, 15, 0);
        s.set(0, 1, 1, BED, 8, 15, 0);
        let mut r = rb(&s);
        r.set_block_bounds(BED, 0.0, 0.0, 0.0, 1.0, 0.5625, 1.0);
        r.render_block_bed(BED, 0, 1, 0);
        assert_eq!(quads(&r.tess), 5);
        assert_eq!(r.tess.positions[0][1], 1.1875);
        assert_eq!(r.tess.positions[4][1], 1.5625);
        assert!(!r.flip_texture);
    }

    /// `(u0, u1, v0, v1)` of a whole tile as the bed computes it.
    fn tile_uv(i: i32) -> (f64, f64, f64, f64) {
        let x = (i & 15) << 4;
        let y = i & 240;
        (
            (x as f32 / 256.0) as f64,
            ((x + 16) as f64 - 0.01) / 256.0,
            (y as f32 / 256.0) as f64,
            ((y + 16) as f64 - 0.01) / 256.0,
        )
    }

    // ---- repeater -------------------------------------------------------------------

    #[test]
    fn repeater_torch_positions_for_delays_1_to_4() {
        // facing 0 (meta & 3): the fixed torch at z - 0.3125, the moving one
        // at z + offset[delay]
        let offsets = [-0.0625, 0.0625, 0.1875, 0.3125];
        for delay in 0..4 {
            let meta = delay << 2;
            assert_eq!(
                repeater_torch_offsets(meta),
                [[0.0, offsets[delay as usize]], [0.0, -0.3125]],
                "delay {}",
                delay + 1
            );
            assert_eq!(
                repeater_torch_offsets(meta | 1),
                [[-offsets[delay as usize], 0.0], [0.3125, 0.0]]
            );
            assert_eq!(
                repeater_torch_offsets(meta | 2),
                [[0.0, -offsets[delay as usize]], [0.0, 0.3125]]
            );
            assert_eq!(
                repeater_torch_offsets(meta | 3),
                [[offsets[delay as usize], 0.0], [-0.3125, 0.0]]
            );
        }
        // the slab's four sides and the top (torches: WP-3.3c)
        let mut s = snap();
        s.set(0, 1, 0, REDSTONE_REPEATER_IDLE, 0, 15, 0);
        let mut r = rb(&s);
        r.render_block_repeater(REDSTONE_REPEATER_IDLE, 0, 1, 0);
        let top = r.tess.len() - 4;
        assert!(r.tess.positions[top..].iter().all(|p| p[1] == 1.125));
        // facing 0: the top's first vertex at (x, z) = (0, 0) with (u0, v0)
        assert_eq!(r.tess.positions[top], [0.0, 1.125, 0.0]);
        assert_eq!(
            r.tess.uvs[top],
            [(131 & 15) as f32 * 16.0 / 256.0, 128.0 / 256.0]
        );
    }

    // ---- pistons --------------------------------------------------------------------

    #[test]
    fn piston_head_uv_rotation_per_facing() {
        // [east, west, south, north, top, bottom]
        assert_eq!(piston_uv_rotations(0), [3, 3, 3, 3, 0, 0]);
        assert_eq!(piston_uv_rotations(1), [0; 6]);
        assert_eq!(piston_uv_rotations(2), [0, 0, 1, 2, 0, 0]);
        assert_eq!(piston_uv_rotations(3), [0, 0, 2, 1, 3, 3]);
        assert_eq!(piston_uv_rotations(4), [1, 2, 0, 0, 2, 1]);
        assert_eq!(piston_uv_rotations(5), [2, 1, 0, 0, 1, 2]);
        // the head, each facing: the plate (6 faces) and 4 rod faces, and
        // every field back to 0 with full bounds
        for facing in 0..6 {
            let mut s = snap();
            s.set(1, 2, 1, HEAD, facing, 15, 0);
            let mut r = rb(&s);
            r.func_31080_c(HEAD, 1, 2, 1, true);
            assert_eq!(quads(&r.tess), 10, "facing {facing}");
            assert_eq!(
                [
                    r.uv_rotate_east,
                    r.uv_rotate_west,
                    r.uv_rotate_south,
                    r.uv_rotate_north,
                    r.uv_rotate_top,
                    r.uv_rotate_bottom
                ],
                [0; 6]
            );
            assert_eq!(bounds(&r.statics, HEAD), BlockBounds::FULL);
            // the rod: 0.25 wide, from the plate a full block back
            let rod = &r.tess.positions[24..];
            let along = [1, 1, 2, 2, 0, 0][facing as usize];
            let lo = rod.iter().map(|p| p[along]).fold(9.0, f32::min);
            let hi = rod.iter().map(|p| p[along]).fold(-9.0, f32::max);
            assert_eq!(hi - lo, 1.0, "facing {facing}");
        }
        // the head's face texture: normal 107, sticky (meta 8) 106, the
        // rest 108 / 107 (the back)
        let st = snap().statics;
        let t =
            |side, meta| blocks::get_block_texture_from_side_and_metadata(&st, HEAD, side, meta);
        assert_eq!((t(1, 1), t(1, 9), t(0, 1), t(2, 1)), (107, 106, 107, 108));
        // the moving piston's half rod: texture 8 texels long
        let mut s = snap();
        s.set(1, 2, 1, HEAD, 1, 15, 0);
        let mut r = rb(&s);
        r.func_31080_c(HEAD, 1, 2, 1, false);
        let u = r.tess.uvs[24][0] as f64;
        assert_eq!(
            u,
            ((((108 & 15) << 4) as f64 + 8.0 - 0.01) / 256.0) as f32 as f64
        );
    }

    #[test]
    fn piston_base_retracted_and_extended() {
        // retracted facing up: a full cube, the front (top) 107
        let mut s = snap();
        s.set(1, 1, 1, PISTON, 1, 15, 0);
        let mut r = rb(&s);
        assert!(r.func_31074_b(PISTON, 1, 1, 1, false));
        assert_eq!(quads(&r.tess), 6);
        // extended facing up: the top at 0.75, textured 110 (inner face)
        let mut s = snap();
        s.set(1, 1, 1, STICKY, 1 | 8, 15, 0);
        s.set(1, 2, 1, HEAD, 1 | 8, 15, 0);
        let mut r = rb(&s);
        r.func_31074_b(STICKY, 1, 1, 1, false);
        let top: Vec<_> = r.tess.positions[4..8].to_vec();
        assert!(top.iter().all(|p| p[1] == 1.75), "{top:?}");
        let u = (r.tess.uvs[4..8].iter().map(|u| u[0]).fold(9.0, f32::min) * 256.0).round();
        assert_eq!(u as i32, (110 & 15) << 4);
        assert_eq!(bounds(&r.statics, STICKY), BlockBounds::FULL);
        for facing in 0..6 {
            let b = piston_base_extended_bounds(facing);
            let cut = b.iter().filter(|&&v| v == 0.25 || v == 0.75).count();
            assert_eq!(cut, 1, "facing {facing}");
        }
        // func_31078_d: drawn extended regardless of the metadata
        let mut s = snap();
        s.set(1, 1, 1, PISTON, 3, 15, 0);
        let mut r = rb(&s);
        r.func_31078_d(PISTON, 1, 1, 1);
        let max_z = r.tess.positions.iter().map(|p| p[2]).fold(-9.0, f32::max);
        assert_eq!(max_z, 1.75);
        assert!(!r.render_all_faces);
    }
}
