//! WP-3.3c: the plant, torch, fire, crop and cactus render types of
//! `RenderBlocks`: `renderBlockReed` (1), `renderBlockTorch` (2) with
//! `renderTorchAtAngle`, `renderBlockFire` (3), `renderBlockCrops` (6) with
//! `func_1245_b`, `renderBlockCactus` (13) with `func_1230_b`, and
//! `renderCrossedSquares` (RenderBlocks.java:276-301, :796-982,
//! :1307-1482, :2334-2444).
//!
//! Not transcribed yet: each draws nothing ([`stub_drawn`]). WP-3.3c
//! replaces the bodies below; [`RenderBlocks`]'s dispatch and
//! `renderBlockOnInventory` (types 1, 2, 6) already call them with the
//! Java's signatures. Tall grass's `colorMultiplier` (the hashed biome
//! offset) is in [`super::blocks::color_multiplier`].

use super::render_blocks::{RenderBlocks, stub_drawn};

impl RenderBlocks<'_> {
    /// `renderBlockReed(block, x, y, z)`: render type 1.
    // src: RenderBlocks.java:1307-1337
    pub fn render_block_reed(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let _ = (id, x, y, z);
        stub_drawn(1, "renderBlockReed")
    }

    /// `renderBlockTorch(block, x, y, z)`: render type 2.
    // src: RenderBlocks.java:276-301
    pub fn render_block_torch(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let _ = (id, x, y, z);
        stub_drawn(2, "renderBlockTorch")
    }

    /// `renderBlockFire(block, x, y, z)`: render type 3.
    // src: RenderBlocks.java:796-982
    pub fn render_block_fire(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let _ = (id, x, y, z);
        stub_drawn(3, "renderBlockFire")
    }

    /// `renderBlockCrops(block, x, y, z)`: render type 6.
    // src: RenderBlocks.java:1339-1345
    pub fn render_block_crops(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let _ = (id, x, y, z);
        stub_drawn(6, "renderBlockCrops")
    }

    /// `renderBlockCactus(block, x, y, z)`: render type 13.
    // src: RenderBlocks.java:2334-2349
    pub fn render_block_cactus(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let _ = (id, x, y, z);
        stub_drawn(13, "renderBlockCactus")
    }

    /// `renderTorchAtAngle(block, x, y, z, dx, dz)` (also the inventory's
    /// torch).
    // src: RenderBlocks.java:1347-1392
    pub fn render_torch_at_angle(&mut self, id: i32, x: f64, y: f64, z: f64, dx: f64, dz: f64) {
        let _ = (id, x, y, z, dx, dz);
        stub_drawn(2, "renderTorchAtAngle");
    }

    /// `renderCrossedSquares(block, meta, x, y, z)` (also the inventory's
    /// type 1).
    // src: RenderBlocks.java:1394-1427
    pub fn render_crossed_squares(&mut self, id: i32, meta: i32, x: f64, y: f64, z: f64) {
        let _ = (id, meta, x, y, z);
        stub_drawn(1, "renderCrossedSquares");
    }

    /// `func_1245_b(block, meta, x, y, z)`: the crops' four planes (also the
    /// inventory's type 6).
    // src: RenderBlocks.java:1429-1482
    pub fn func_1245_b(&mut self, id: i32, meta: i32, x: f64, y: f64, z: f64) {
        let _ = (id, meta, x, y, z);
        stub_drawn(6, "func_1245_b");
    }
}
