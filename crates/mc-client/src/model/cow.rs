//! `ModelCow` (ModelCow.java): `ModelQuadruped(12, 0)` with its own head and
//! body, horns, udders, and the legs moved out.

use super::Model;
use super::part::{ModelPart, RenderMode};
use super::quadruped::{self, BODY, HEAD, LEG1, LEG2, LEG3, LEG4};

/// `udders`.
pub const UDDERS: usize = 6;
/// `horn1`.
pub const HORN1: usize = 7;
/// `horn2`.
pub const HORN2: usize = 8;

/// `render`'s draw list: the quadruped's, then the horns and udders.
// src: ModelCow.java:36-41
pub const RENDER: &[(usize, RenderMode)] = &[
    (HEAD, RenderMode::Render),
    (BODY, RenderMode::Render),
    (LEG1, RenderMode::Render),
    (LEG2, RenderMode::Render),
    (LEG3, RenderMode::Render),
    (LEG4, RenderMode::Render),
    (HORN1, RenderMode::Render),
    (HORN2, RenderMode::Render),
    (UDDERS, RenderMode::Render),
];

/// `new ModelCow()`.
// src: ModelCow.java:8-34
pub fn build() -> Vec<ModelPart> {
    let mut p = quadruped::build(12, 0.0);
    let mut head = ModelPart::new("head", 0, 0);
    head.add_box_inflated(-4.0, -4.0, -6.0, 8, 8, 6, 0.0);
    head.set_rotation_point(0.0, 4.0, -8.0);
    let mut horn1 = ModelPart::new("horn1", 22, 0);
    horn1.add_box_inflated(-4.0, -5.0, -4.0, 1, 3, 1, 0.0);
    horn1.set_rotation_point(0.0, 3.0, -7.0);
    let mut horn2 = ModelPart::new("horn2", 22, 0);
    horn2.add_box_inflated(3.0, -5.0, -4.0, 1, 3, 1, 0.0);
    horn2.set_rotation_point(0.0, 3.0, -7.0);
    let mut udders = ModelPart::new("udders", 52, 0);
    udders.add_box_inflated(-2.0, -3.0, 0.0, 4, 6, 2, 0.0);
    udders.set_rotation_point(0.0, 14.0, 6.0);
    udders.rotate_angle_x = 1.5707964;
    let mut body = ModelPart::new("body", 18, 4);
    body.add_box_inflated(-6.0, -10.0, -7.0, 12, 18, 10, 0.0);
    body.set_rotation_point(0.0, 5.0, 2.0);
    p[HEAD] = head;
    p[BODY] = body;
    p[LEG1].rotation_point_x -= 1.0;
    p[LEG2].rotation_point_x += 1.0;
    p[LEG1].rotation_point_z += 0.0;
    p[LEG2].rotation_point_z += 0.0;
    p[LEG3].rotation_point_x -= 1.0;
    p[LEG4].rotation_point_x += 1.0;
    p[LEG3].rotation_point_z -= 1.0;
    p[LEG4].rotation_point_z -= 1.0;
    p.extend([udders, horn1, horn2]);
    p
}

/// `ModelCow.setRotationAngles`: the quadruped's, the horns following the
/// head.
// src: ModelCow.java:43-50
pub fn set_rotation_angles(
    m: &mut Model,
    var1: f32,
    var2: f32,
    var3: f32,
    var4: f32,
    var5: f32,
    var6: f32,
) {
    quadruped::set_rotation_angles(m, var1, var2, var3, var4, var5, var6);
    let p = &mut m.parts;
    let (hy, hx) = (p[HEAD].rotate_angle_y, p[HEAD].rotate_angle_x);
    p[HORN1].rotate_angle_y = hy;
    p[HORN1].rotate_angle_x = hx;
    p[HORN2].rotate_angle_y = hy;
    p[HORN2].rotate_angle_x = hx;
}
