//! `ModelBiped` (ModelBiped.java): the player, the fallback `RenderLiving`
//! model, the armour layers, and the base of `ModelZombie`/`ModelSkeleton`.

use mc_core::math::{cos, sin, sqrt_float};

use super::Model;
use super::part::{ModelPart, RenderMode};

/// `bipedHead`.
pub const HEAD: usize = 0;
/// `bipedHeadwear`.
pub const HEADWEAR: usize = 1;
/// `bipedBody`.
pub const BODY: usize = 2;
/// `bipedRightArm`.
pub const RIGHT_ARM: usize = 3;
/// `bipedLeftArm`.
pub const LEFT_ARM: usize = 4;
/// `bipedRightLeg`.
pub const RIGHT_LEG: usize = 5;
/// `bipedLeftLeg`.
pub const LEFT_LEG: usize = 6;
/// `bipedEars` (drawn only by `renderEars`).
pub const EARS: usize = 7;
/// `bipedCloak` (drawn only by `renderCloak`).
pub const CLOAK: usize = 8;

/// `render`'s draw list: head, body, arms, legs, then the headwear.
// src: ModelBiped.java:58-67
pub const RENDER: &[(usize, RenderMode)] = &[
    (HEAD, RenderMode::Render),
    (BODY, RenderMode::Render),
    (RIGHT_ARM, RenderMode::Render),
    (LEFT_ARM, RenderMode::Render),
    (RIGHT_LEG, RenderMode::Render),
    (LEFT_LEG, RenderMode::Render),
    (HEADWEAR, RenderMode::Render),
];

/// `new ModelBiped(inflate, yOffset)`; the parts in field order.
// src: ModelBiped.java:25-56
pub fn build(var1: f32, var2: f32) -> Vec<ModelPart> {
    let mut cloak = ModelPart::new("bipedCloak", 0, 0);
    cloak.add_box_inflated(-5.0, 0.0, -1.0, 10, 16, 1, var1);
    let mut ears = ModelPart::new("bipedEars", 24, 0);
    ears.add_box_inflated(-3.0, -6.0, -1.0, 6, 6, 1, var1);
    let mut head = ModelPart::new("bipedHead", 0, 0);
    head.add_box_inflated(-4.0, -8.0, -4.0, 8, 8, 8, var1);
    head.set_rotation_point(0.0, 0.0 + var2, 0.0);
    let mut headwear = ModelPart::new("bipedHeadwear", 32, 0);
    headwear.add_box_inflated(-4.0, -8.0, -4.0, 8, 8, 8, var1 + 0.5);
    headwear.set_rotation_point(0.0, 0.0 + var2, 0.0);
    let mut body = ModelPart::new("bipedBody", 16, 16);
    body.add_box_inflated(-4.0, 0.0, -2.0, 8, 12, 4, var1);
    body.set_rotation_point(0.0, 0.0 + var2, 0.0);
    let mut right_arm = ModelPart::new("bipedRightArm", 40, 16);
    right_arm.add_box_inflated(-3.0, -2.0, -2.0, 4, 12, 4, var1);
    right_arm.set_rotation_point(-5.0, 2.0 + var2, 0.0);
    let mut left_arm = ModelPart::mirrored("bipedLeftArm", 40, 16);
    left_arm.add_box_inflated(-1.0, -2.0, -2.0, 4, 12, 4, var1);
    left_arm.set_rotation_point(5.0, 2.0 + var2, 0.0);
    let mut right_leg = ModelPart::new("bipedRightLeg", 0, 16);
    right_leg.add_box_inflated(-2.0, 0.0, -2.0, 4, 12, 4, var1);
    right_leg.set_rotation_point(-2.0, 12.0 + var2, 0.0);
    let mut left_leg = ModelPart::mirrored("bipedLeftLeg", 0, 16);
    left_leg.add_box_inflated(-2.0, 0.0, -2.0, 4, 12, 4, var1);
    left_leg.set_rotation_point(2.0, 12.0 + var2, 0.0);
    vec![
        head, headwear, body, right_arm, left_arm, right_leg, left_leg, ears, cloak,
    ]
}

/// `ModelBiped.setRotationAngles(limbSwing, limbSwingAmount, ticks,
/// headYaw, headPitch, scale)`: head look, the walk swing of arms and legs,
/// riding, held items, the attack swing (`onGround` is the swing progress),
/// sneaking, and the idle arm sway.
// src: ModelBiped.java:69-162
pub fn set_rotation_angles(
    m: &mut Model,
    var1: f32,
    var2: f32,
    var3: f32,
    var4: f32,
    var5: f32,
    _var6: f32,
) {
    let (on_ground, is_riding) = (m.on_ground, m.is_riding);
    let (held_left, held_right, is_sneak) = (m.held_item_left, m.held_item_right, m.is_sneak);
    let p = &mut m.parts;
    p[HEAD].rotate_angle_y = var4 / 57.295776;
    p[HEAD].rotate_angle_x = var5 / 57.295776;
    p[HEADWEAR].rotate_angle_y = p[HEAD].rotate_angle_y;
    p[HEADWEAR].rotate_angle_x = p[HEAD].rotate_angle_x;
    p[RIGHT_ARM].rotate_angle_x = cos(var1 * 0.6662 + 3.1415927) * 2.0 * var2 * 0.5;
    p[LEFT_ARM].rotate_angle_x = cos(var1 * 0.6662) * 2.0 * var2 * 0.5;
    p[RIGHT_ARM].rotate_angle_z = 0.0;
    p[LEFT_ARM].rotate_angle_z = 0.0;
    p[RIGHT_LEG].rotate_angle_x = cos(var1 * 0.6662) * 1.4 * var2;
    p[LEFT_LEG].rotate_angle_x = cos(var1 * 0.6662 + 3.1415927) * 1.4 * var2;
    p[RIGHT_LEG].rotate_angle_y = 0.0;
    p[LEFT_LEG].rotate_angle_y = 0.0;
    if is_riding {
        p[RIGHT_ARM].rotate_angle_x += -0.62831855;
        p[LEFT_ARM].rotate_angle_x += -0.62831855;
        p[RIGHT_LEG].rotate_angle_x = -1.2566371;
        p[LEFT_LEG].rotate_angle_x = -1.2566371;
        p[RIGHT_LEG].rotate_angle_y = 0.31415927;
        p[LEFT_LEG].rotate_angle_y = -0.31415927;
    }

    if held_left {
        p[LEFT_ARM].rotate_angle_x = p[LEFT_ARM].rotate_angle_x * 0.5 - 0.31415927;
    }

    if held_right {
        p[RIGHT_ARM].rotate_angle_x = p[RIGHT_ARM].rotate_angle_x * 0.5 - 0.31415927;
    }

    p[RIGHT_ARM].rotate_angle_y = 0.0;
    p[LEFT_ARM].rotate_angle_y = 0.0;
    if on_ground > -9990.0 {
        let mut var7 = on_ground;
        p[BODY].rotate_angle_y = sin(sqrt_float(var7) * 3.1415927 * 2.0) * 0.2;
        let body_y = p[BODY].rotate_angle_y;
        p[RIGHT_ARM].rotation_point_z = sin(body_y) * 5.0;
        p[RIGHT_ARM].rotation_point_x = -cos(body_y) * 5.0;
        p[LEFT_ARM].rotation_point_z = -sin(body_y) * 5.0;
        p[LEFT_ARM].rotation_point_x = cos(body_y) * 5.0;
        p[RIGHT_ARM].rotate_angle_y += body_y;
        p[LEFT_ARM].rotate_angle_y += body_y;
        p[LEFT_ARM].rotate_angle_x += body_y;
        var7 = 1.0 - on_ground;
        var7 *= var7;
        var7 *= var7;
        var7 = 1.0 - var7;
        let var8 = sin(var7 * 3.1415927);
        let var9 = sin(on_ground * 3.1415927) * -(p[HEAD].rotate_angle_x - 0.7) * 0.75;
        p[RIGHT_ARM].rotate_angle_x =
            (p[RIGHT_ARM].rotate_angle_x as f64 - (var8 as f64 * 1.2 + var9 as f64)) as f32;
        p[RIGHT_ARM].rotate_angle_y += body_y * 2.0;
        p[RIGHT_ARM].rotate_angle_z = sin(on_ground * 3.1415927) * -0.4;
    }

    if is_sneak {
        p[BODY].rotate_angle_x = 0.5;
        p[RIGHT_LEG].rotate_angle_x -= 0.0;
        p[LEFT_LEG].rotate_angle_x -= 0.0;
        p[RIGHT_ARM].rotate_angle_x += 0.4;
        p[LEFT_ARM].rotate_angle_x += 0.4;
        p[RIGHT_LEG].rotation_point_z = 4.0;
        p[LEFT_LEG].rotation_point_z = 4.0;
        p[RIGHT_LEG].rotation_point_y = 9.0;
        p[LEFT_LEG].rotation_point_y = 9.0;
        p[HEAD].rotation_point_y = 1.0;
    } else {
        p[BODY].rotate_angle_x = 0.0;
        p[RIGHT_LEG].rotation_point_z = 0.0;
        p[LEFT_LEG].rotation_point_z = 0.0;
        p[RIGHT_LEG].rotation_point_y = 12.0;
        p[LEFT_LEG].rotation_point_y = 12.0;
        p[HEAD].rotation_point_y = 0.0;
    }

    p[RIGHT_ARM].rotate_angle_z += cos(var3 * 0.09) * 0.05 + 0.05;
    p[LEFT_ARM].rotate_angle_z -= cos(var3 * 0.09) * 0.05 + 0.05;
    p[RIGHT_ARM].rotate_angle_x += sin(var3 * 0.067) * 0.05;
    p[LEFT_ARM].rotate_angle_x -= sin(var3 * 0.067) * 0.05;
}

/// `renderEars(scale)`'s state change before its `render`: the ears take
/// the head's angles, rotation point x and y zeroed (z kept).
// src: ModelBiped.java:164-170
pub fn prepare_ears(m: &mut Model) {
    let (ay, ax) = (m.parts[HEAD].rotate_angle_y, m.parts[HEAD].rotate_angle_x);
    let e = &mut m.parts[EARS];
    e.rotate_angle_y = ay;
    e.rotate_angle_x = ax;
    e.rotation_point_x = 0.0;
    e.rotation_point_y = 0.0;
}
