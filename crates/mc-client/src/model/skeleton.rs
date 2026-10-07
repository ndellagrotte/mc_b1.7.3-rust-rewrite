//! `ModelSkeleton` (ModelSkeleton.java): `ModelZombie` with 2×2 arms and
//! legs; its pose is the zombie's.

use super::biped::{self, LEFT_ARM, LEFT_LEG, RIGHT_ARM, RIGHT_LEG};
use super::part::ModelPart;

/// `new ModelSkeleton()`: `ModelBiped()`'s parts with the arms and legs
/// replaced.
// src: ModelSkeleton.java:4-20
pub fn build() -> Vec<ModelPart> {
    let mut p = biped::build(0.0, 0.0);
    let var1 = 0.0f32;
    let mut right_arm = ModelPart::new("bipedRightArm", 40, 16);
    right_arm.add_box_inflated(-1.0, -2.0, -1.0, 2, 12, 2, var1);
    right_arm.set_rotation_point(-5.0, 2.0, 0.0);
    let mut left_arm = ModelPart::mirrored("bipedLeftArm", 40, 16);
    left_arm.add_box_inflated(-1.0, -2.0, -1.0, 2, 12, 2, var1);
    left_arm.set_rotation_point(5.0, 2.0, 0.0);
    let mut right_leg = ModelPart::new("bipedRightLeg", 0, 16);
    right_leg.add_box_inflated(-1.0, 0.0, -1.0, 2, 12, 2, var1);
    right_leg.set_rotation_point(-2.0, 12.0, 0.0);
    let mut left_leg = ModelPart::mirrored("bipedLeftLeg", 0, 16);
    left_leg.add_box_inflated(-1.0, 0.0, -1.0, 2, 12, 2, var1);
    left_leg.set_rotation_point(2.0, 12.0, 0.0);
    p[RIGHT_ARM] = right_arm;
    p[LEFT_ARM] = left_arm;
    p[RIGHT_LEG] = right_leg;
    p[LEFT_LEG] = left_leg;
    p
}
