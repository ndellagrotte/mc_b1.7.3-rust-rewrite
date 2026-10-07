//! WP-3.3d: the remaining render types of `RenderBlocks`:
//! `renderBlockRedstoneWire` (5), `renderBlockDoor` (7),
//! `renderBlockLadder` (8), `renderBlockMinecartTrack` (9),
//! `renderBlockStairs` (10), `renderBlockFence` (11), `renderBlockLever`
//! (12), `renderBlockBed` (14), `renderBlockRepeater` (15), the piston base
//! `func_31074_b` (16) and head `func_31080_c` (17) with their helpers
//! (`func_31078_d`, `func_31079_a`, the rod quads `func_31076_a`,
//! `func_31081_b`, `func_31077_c`) (RenderBlocks.java:134-794,
//! :984-1305, :2446-2652). `renderBlockFallingSand` (RenderFallingSand,
//! WP-3.5b) reads a `World` and belongs with it.
//!
//! Not transcribed yet: each draws nothing ([`stub_drawn`]). WP-3.3d
//! replaces the bodies below; the dispatch already calls them. The piston
//! and the bed set the `uv_rotate_*` fields and [`RenderBlocks::flip_texture`]
//! the face functions read; stairs and fence call
//! [`RenderBlocks::set_block_bounds`] and [`RenderBlocks::render_standard_block`]
//! as the Java does (the bounds live in [`RenderBlocks::statics`]). Door
//! textures (`BlockDoor.getBlockTextureFromSideAndMetadata`, negative for a
//! mirrored face, BlockDoor.java:18-36) already come through
//! [`super::blocks::get_block_texture`] from mc-sim.

use super::render_blocks::{RenderBlocks, stub_drawn};

impl RenderBlocks<'_> {
    /// `renderBlockRedstoneWire(block, x, y, z)`: render type 5.
    // src: RenderBlocks.java:984-1183
    pub fn render_block_redstone_wire(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let _ = (id, x, y, z);
        stub_drawn(5, "renderBlockRedstoneWire")
    }

    /// `renderBlockDoor(block, x, y, z)`: render type 7.
    // src: RenderBlocks.java:2542-2652
    pub fn render_block_door(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let _ = (id, x, y, z);
        stub_drawn(7, "renderBlockDoor")
    }

    /// `renderBlockLadder(block, x, y, z)`: render type 8.
    // src: RenderBlocks.java:1258-1305
    pub fn render_block_ladder(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let _ = (id, x, y, z);
        stub_drawn(8, "renderBlockLadder")
    }

    /// `renderBlockMinecartTrack(rail, x, y, z)`: render type 9.
    // src: RenderBlocks.java:1185-1256
    pub fn render_block_minecart_track(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let _ = (id, x, y, z);
        stub_drawn(9, "renderBlockMinecartTrack")
    }

    /// `renderBlockStairs(block, x, y, z)`: render type 10.
    // src: RenderBlocks.java:2509-2540
    pub fn render_block_stairs(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let _ = (id, x, y, z);
        stub_drawn(10, "renderBlockStairs")
    }

    /// `renderBlockFence(block, x, y, z)`: render type 11.
    // src: RenderBlocks.java:2446-2507
    pub fn render_block_fence(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let _ = (id, x, y, z);
        stub_drawn(11, "renderBlockFence")
    }

    /// `renderBlockLever(block, x, y, z)`: render type 12.
    // src: RenderBlocks.java:633-794
    pub fn render_block_lever(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let _ = (id, x, y, z);
        stub_drawn(12, "renderBlockLever")
    }

    /// `renderBlockBed(block, x, y, z)`: render type 14.
    // src: RenderBlocks.java:134-274
    pub fn render_block_bed(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let _ = (id, x, y, z);
        stub_drawn(14, "renderBlockBed")
    }

    /// `renderBlockRepeater(block, x, y, z)`: render type 15.
    // src: RenderBlocks.java:303-379
    pub fn render_block_repeater(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let _ = (id, x, y, z);
        stub_drawn(15, "renderBlockRepeater")
    }

    /// `func_31074_b(block, x, y, z, extended)`: the piston base, render
    /// type 16 (later MCP: `renderPistonBase`).
    // src: RenderBlocks.java:387-481
    pub fn func_31074_b(&mut self, id: i32, x: i32, y: i32, z: i32, extended: bool) -> bool {
        let _ = (id, x, y, z, extended);
        stub_drawn(16, "func_31074_b (piston base)")
    }

    /// `func_31080_c(block, x, y, z, full_rod)`: the piston head, render
    /// type 17 (later MCP: `renderPistonExtension`).
    // src: RenderBlocks.java:549-631
    pub fn func_31080_c(&mut self, id: i32, x: i32, y: i32, z: i32, full_rod: bool) -> bool {
        let _ = (id, x, y, z, full_rod);
        stub_drawn(17, "func_31080_c (piston extension)")
    }
}
