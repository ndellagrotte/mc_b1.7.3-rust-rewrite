//! `RenderChicken` (RenderChicken.java): the model's `ticks` argument is
//! the wing angle.

use mc_core::math::sin;
use mc_sim::entity::{Entity, EntityKindData as K};

/// `getWingRotation(chicken, partial)`: `(sin(flap) + 1) × flapSpeed`,
/// both interpolated.
// src: RenderChicken.java:12-16
pub fn get_wing_rotation(var1: &Entity, var2: f32) -> f32 {
    let K::Chicken(_, _, _, c) = &var1.kind else {
        return 0.0;
    };
    let var3 = c.o_flap + (c.wing_rotation - c.o_flap) * var2;
    let var4 = c.o_flap_speed + (c.dest_pos - c.o_flap_speed) * var2;
    (sin(var3) + 1.0) * var4
}
