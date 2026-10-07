//! `ModelQuadruped` (ModelQuadruped.java): the four-legged base of the pig,
//! cow and sheep models.

use mc_core::math::cos;

use super::Model;
use super::part::{ModelPart, RenderMode};

/// `head`.
pub const HEAD: usize = 0;
/// `body`.
pub const BODY: usize = 1;
/// `leg1` (back right).
pub const LEG1: usize = 2;
/// `leg2` (back left).
pub const LEG2: usize = 3;
/// `leg3` (front right).
pub const LEG3: usize = 4;
/// `leg4` (front left).
pub const LEG4: usize = 5;

/// `render`'s draw list.
// src: ModelQuadruped.java:31-39
pub const RENDER: &[(usize, RenderMode)] = &[
    (HEAD, RenderMode::Render),
    (BODY, RenderMode::Render),
    (LEG1, RenderMode::Render),
    (LEG2, RenderMode::Render),
    (LEG3, RenderMode::Render),
    (LEG4, RenderMode::Render),
];

/// `new ModelQuadruped(legHeight, inflate)`.
// src: ModelQuadruped.java:4-29
pub fn build(var1: i32, var2: f32) -> Vec<ModelPart> {
    let mut head = ModelPart::new("head", 0, 0);
    head.add_box_inflated(-4.0, -4.0, -8.0, 8, 8, 8, var2);
    head.set_rotation_point(0.0, (18 - var1) as f32, -6.0);
    let mut body = ModelPart::new("body", 28, 8);
    body.add_box_inflated(-5.0, -10.0, -7.0, 10, 16, 8, var2);
    body.set_rotation_point(0.0, (17 - var1) as f32, 2.0);
    let leg = |name: &'static str, x: f32, z: f32| {
        let mut l = ModelPart::new(name, 0, 16);
        l.add_box_inflated(-2.0, 0.0, -2.0, 4, var1, 4, var2);
        l.set_rotation_point(x, (24 - var1) as f32, z);
        l
    };
    vec![
        head,
        body,
        leg("leg1", -3.0, 7.0),
        leg("leg2", 3.0, 7.0),
        leg("leg3", -3.0, -5.0),
        leg("leg4", 3.0, -5.0),
    ]
}

/// `ModelQuadruped.setRotationAngles`: head look, the body turned level
/// (`π/2`), the legs' walk swing in diagonal pairs.
// src: ModelQuadruped.java:41-49
pub fn set_rotation_angles(
    m: &mut Model,
    var1: f32,
    var2: f32,
    _var3: f32,
    var4: f32,
    var5: f32,
    _var6: f32,
) {
    let p = &mut m.parts;
    p[HEAD].rotate_angle_x = var5 / 57.295776;
    p[HEAD].rotate_angle_y = var4 / 57.295776;
    p[BODY].rotate_angle_x = 1.5707964;
    p[LEG1].rotate_angle_x = cos(var1 * 0.6662) * 1.4 * var2;
    p[LEG2].rotate_angle_x = cos(var1 * 0.6662 + 3.1415927) * 1.4 * var2;
    p[LEG3].rotate_angle_x = cos(var1 * 0.6662 + 3.1415927) * 1.4 * var2;
    p[LEG4].rotate_angle_x = cos(var1 * 0.6662) * 1.4 * var2;
}
