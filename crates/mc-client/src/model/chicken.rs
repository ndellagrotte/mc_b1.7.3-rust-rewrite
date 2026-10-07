//! `ModelChicken` (ModelChicken.java).

use mc_core::math::cos;

use super::Model;
use super::part::{ModelPart, RenderMode};

/// `head`.
pub const HEAD: usize = 0;
/// `body`.
pub const BODY: usize = 1;
/// `rightLeg`.
pub const RIGHT_LEG: usize = 2;
/// `leftLeg`.
pub const LEFT_LEG: usize = 3;
/// `rightWing`.
pub const RIGHT_WING: usize = 4;
/// `leftWing`.
pub const LEFT_WING: usize = 5;
/// `bill`.
pub const BILL: usize = 6;
/// `chin` (the wattle).
pub const CHIN: usize = 7;

/// `render`'s draw list.
// src: ModelChicken.java:41-51
pub const RENDER: &[(usize, RenderMode)] = &[
    (HEAD, RenderMode::Render),
    (BILL, RenderMode::Render),
    (CHIN, RenderMode::Render),
    (BODY, RenderMode::Render),
    (RIGHT_LEG, RenderMode::Render),
    (LEFT_LEG, RenderMode::Render),
    (RIGHT_WING, RenderMode::Render),
    (LEFT_WING, RenderMode::Render),
];

/// `new ModelChicken()`.
// src: ModelChicken.java:13-39
pub fn build() -> Vec<ModelPart> {
    let var1: i32 = 16;
    let mut head = ModelPart::new("head", 0, 0);
    head.add_box_inflated(-2.0, -6.0, -2.0, 4, 6, 3, 0.0);
    head.set_rotation_point(0.0, (-1 + var1) as f32, -4.0);
    let mut bill = ModelPart::new("bill", 14, 0);
    bill.add_box_inflated(-2.0, -4.0, -4.0, 4, 2, 2, 0.0);
    bill.set_rotation_point(0.0, (-1 + var1) as f32, -4.0);
    let mut chin = ModelPart::new("chin", 14, 4);
    chin.add_box_inflated(-1.0, -2.0, -3.0, 2, 2, 2, 0.0);
    chin.set_rotation_point(0.0, (-1 + var1) as f32, -4.0);
    let mut body = ModelPart::new("body", 0, 9);
    body.add_box_inflated(-3.0, -4.0, -3.0, 6, 8, 6, 0.0);
    body.set_rotation_point(0.0, var1 as f32, 0.0);
    let mut right_leg = ModelPart::new("rightLeg", 26, 0);
    right_leg.add_box(-1.0, 0.0, -3.0, 3, 5, 3);
    right_leg.set_rotation_point(-2.0, (3 + var1) as f32, 1.0);
    let mut left_leg = ModelPart::new("leftLeg", 26, 0);
    left_leg.add_box(-1.0, 0.0, -3.0, 3, 5, 3);
    left_leg.set_rotation_point(1.0, (3 + var1) as f32, 1.0);
    let mut right_wing = ModelPart::new("rightWing", 24, 13);
    right_wing.add_box(0.0, 0.0, -3.0, 1, 4, 6);
    right_wing.set_rotation_point(-4.0, (-3 + var1) as f32, 0.0);
    let mut left_wing = ModelPart::new("leftWing", 24, 13);
    left_wing.add_box(-1.0, 0.0, -3.0, 1, 4, 6);
    left_wing.set_rotation_point(4.0, (-3 + var1) as f32, 0.0);
    vec![
        head, body, right_leg, left_leg, right_wing, left_wing, bill, chin,
    ]
}

/// `ModelChicken.setRotationAngles`: head look (pitch negated), the body
/// level, legs' walk swing, the wings flapping by `ticks`
/// (`RenderChicken.getWingRotation`).
// src: ModelChicken.java:53-65
pub fn set_rotation_angles(
    m: &mut Model,
    var1: f32,
    var2: f32,
    var3: f32,
    var4: f32,
    var5: f32,
    _var6: f32,
) {
    let p = &mut m.parts;
    p[HEAD].rotate_angle_x = -(var5 / 57.295776);
    p[HEAD].rotate_angle_y = var4 / 57.295776;
    p[BILL].rotate_angle_x = p[HEAD].rotate_angle_x;
    p[BILL].rotate_angle_y = p[HEAD].rotate_angle_y;
    p[CHIN].rotate_angle_x = p[HEAD].rotate_angle_x;
    p[CHIN].rotate_angle_y = p[HEAD].rotate_angle_y;
    p[BODY].rotate_angle_x = 1.5707964;
    p[RIGHT_LEG].rotate_angle_x = cos(var1 * 0.6662) * 1.4 * var2;
    p[LEFT_LEG].rotate_angle_x = cos(var1 * 0.6662 + 3.1415927) * 1.4 * var2;
    p[RIGHT_WING].rotate_angle_z = var3;
    p[LEFT_WING].rotate_angle_z = -var3;
}
