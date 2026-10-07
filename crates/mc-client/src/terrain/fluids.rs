//! WP-3.3b: `RenderBlocks.renderBlockFluids` (render type 4: water and
//! lava) and its helper `func_1224_a` (the fluid height at a corner)
//! (RenderBlocks.java:1484-1663).
//!
//! Not transcribed yet: the entry point draws nothing ([`stub_drawn`]).
//! WP-3.3b replaces the bodies below; [`RenderBlocks`]'s dispatch already
//! calls them. Available: [`RenderBlocks::block_access`] (the
//! `IBlockAccess`, including `get_block_material`, `get_light_brightness`),
//! [`RenderBlocks::brightness`] (`BlockFluid.getBlockBrightness`, max of the
//! cell and the one above), [`super::blocks::should_side_be_rendered`]
//! (`BlockFluid.shouldSideBeRendered`), [`super::blocks::color_multiplier`]
//! (white for fluids), the face functions (`renderBottomFace`) and the
//! tessellator. The water pass (1) is drawn blended by
//! [`super::world_renderer`].

use super::render_blocks::{RenderBlocks, stub_drawn};

impl RenderBlocks<'_> {
    /// `renderBlockFluids(block, x, y, z)`.
    // src: RenderBlocks.java:1484-1631
    pub fn render_block_fluids(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let _ = (id, x, y, z);
        stub_drawn(4, "renderBlockFluids")
    }
}
