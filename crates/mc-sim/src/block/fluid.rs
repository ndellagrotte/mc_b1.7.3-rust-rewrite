//! `BlockFluid` statics (BlockFluid.java), the part other systems call today.
//!
//! The fluid behaviour (`BlockFlowing`, `BlockStationary`, `velocityToAddToEntity`,
//! `getFlowDirection`, …) is the fluids task of WP-1.6 and lands here.

/// `BlockFluid.getPercentAir(meta)`: the empty fraction of a fluid cell.
// src: BlockFluid.java:18-25
pub fn get_percent_air(meta: i32) -> f32 {
    let meta = if meta >= 8 { 0 } else { meta };
    (meta + 1) as f32 / 9.0
}
