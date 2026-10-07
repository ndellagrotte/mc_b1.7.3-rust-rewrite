//! `ModelSpider` (ModelSpider.java); the eyes pass draws a second
//! `ModelSpider`.

use mc_core::math::{cos, sin};

use super::Model;
use super::part::{ModelPart, RenderMode};

/// `spiderHead`.
pub const HEAD: usize = 0;
/// `spiderNeck`.
pub const NECK: usize = 1;
/// `spiderBody`.
pub const BODY: usize = 2;
/// `spiderLeg1` … `spiderLeg8` are `LEG1 + 0` … `LEG1 + 7`.
pub const LEG1: usize = 3;

/// `render`'s draw list: head, neck, body, legs 1-8.
// src: ModelSpider.java:39-51
pub const RENDER: &[(usize, RenderMode)] = &[
    (HEAD, RenderMode::Render),
    (NECK, RenderMode::Render),
    (BODY, RenderMode::Render),
    (LEG1, RenderMode::Render),
    (LEG1 + 1, RenderMode::Render),
    (LEG1 + 2, RenderMode::Render),
    (LEG1 + 3, RenderMode::Render),
    (LEG1 + 4, RenderMode::Render),
    (LEG1 + 5, RenderMode::Render),
    (LEG1 + 6, RenderMode::Render),
    (LEG1 + 7, RenderMode::Render),
];

/// `new ModelSpider()`.
// src: ModelSpider.java:15-37
pub fn build() -> Vec<ModelPart> {
    let var1 = 0.0f32;
    let var2: i32 = 15;
    let mut head = ModelPart::new("spiderHead", 32, 4);
    head.add_box_inflated(-4.0, -4.0, -8.0, 8, 8, 8, var1);
    head.set_rotation_point(0.0, var2 as f32, -3.0);
    let mut neck = ModelPart::new("spiderNeck", 0, 0);
    neck.add_box_inflated(-3.0, -3.0, -3.0, 6, 6, 6, var1);
    neck.set_rotation_point(0.0, var2 as f32, 0.0);
    let mut body = ModelPart::new("spiderBody", 0, 12);
    body.add_box_inflated(-5.0, -4.0, -6.0, 10, 8, 12, var1);
    body.set_rotation_point(0.0, var2 as f32, 9.0);
    // Odd legs reach out to -x (box from -15), even ones to +x (from -1).
    let leg = |name: &'static str, x0: f32, rx: f32, rz: f32| {
        let mut l = ModelPart::new(name, 18, 0);
        l.add_box_inflated(x0, -1.0, -1.0, 16, 2, 2, var1);
        l.set_rotation_point(rx, var2 as f32, rz);
        l
    };
    vec![
        head,
        neck,
        body,
        leg("spiderLeg1", -15.0, -4.0, 2.0),
        leg("spiderLeg2", -1.0, 4.0, 2.0),
        leg("spiderLeg3", -15.0, -4.0, 1.0),
        leg("spiderLeg4", -1.0, 4.0, 1.0),
        leg("spiderLeg5", -15.0, -4.0, 0.0),
        leg("spiderLeg6", -1.0, 4.0, 0.0),
        leg("spiderLeg7", -15.0, -4.0, -1.0),
        leg("spiderLeg8", -1.0, 4.0, -1.0),
    ]
}

/// `ModelSpider.setRotationAngles`: head look, the legs' resting spread
/// (Z by `π/4`, the middle pairs ×0.74; Y fanned by `π/8` steps), then the
/// walk: four phases of a Y swing (`cos` at twice the rate) and a Z lift
/// (`|sin|`).
// src: ModelSpider.java:53-114
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
    let l = |i: usize| LEG1 + i - 1;
    p[HEAD].rotate_angle_y = var4 / 57.295776;
    p[HEAD].rotate_angle_x = var5 / 57.295776;
    let var7 = 0.7853982f32;
    p[l(1)].rotate_angle_z = -var7;
    p[l(2)].rotate_angle_z = var7;
    p[l(3)].rotate_angle_z = -var7 * 0.74;
    p[l(4)].rotate_angle_z = var7 * 0.74;
    p[l(5)].rotate_angle_z = -var7 * 0.74;
    p[l(6)].rotate_angle_z = var7 * 0.74;
    p[l(7)].rotate_angle_z = -var7;
    p[l(8)].rotate_angle_z = var7;
    let var8 = -0.0f32;
    let var9 = 0.3926991f32;
    p[l(1)].rotate_angle_y = var9 * 2.0 + var8;
    p[l(2)].rotate_angle_y = -var9 * 2.0 - var8;
    p[l(3)].rotate_angle_y = var9 * 1.0 + var8;
    p[l(4)].rotate_angle_y = -var9 * 1.0 - var8;
    p[l(5)].rotate_angle_y = -var9 * 1.0 + var8;
    p[l(6)].rotate_angle_y = var9 * 1.0 - var8;
    p[l(7)].rotate_angle_y = -var9 * 2.0 + var8;
    p[l(8)].rotate_angle_y = var9 * 2.0 - var8;
    let var10 = -(cos(var1 * 0.6662 * 2.0 + 0.0) * 0.4) * var2;
    let var11 = -(cos(var1 * 0.6662 * 2.0 + 3.1415927) * 0.4) * var2;
    let var12 = -(cos(var1 * 0.6662 * 2.0 + 1.5707964) * 0.4) * var2;
    let var13 = -(cos(var1 * 0.6662 * 2.0 + 4.712389) * 0.4) * var2;
    let var14 = (sin(var1 * 0.6662 + 0.0) * 0.4).abs() * var2;
    let var15 = (sin(var1 * 0.6662 + 3.1415927) * 0.4).abs() * var2;
    let var16 = (sin(var1 * 0.6662 + 1.5707964) * 0.4).abs() * var2;
    let var17 = (sin(var1 * 0.6662 + 4.712389) * 0.4).abs() * var2;
    p[l(1)].rotate_angle_y += var10;
    p[l(2)].rotate_angle_y += -var10;
    p[l(3)].rotate_angle_y += var11;
    p[l(4)].rotate_angle_y += -var11;
    p[l(5)].rotate_angle_y += var12;
    p[l(6)].rotate_angle_y += -var12;
    p[l(7)].rotate_angle_y += var13;
    p[l(8)].rotate_angle_y += -var13;
    p[l(1)].rotate_angle_z += var14;
    p[l(2)].rotate_angle_z += -var14;
    p[l(3)].rotate_angle_z += var15;
    p[l(4)].rotate_angle_z += -var15;
    p[l(5)].rotate_angle_z += var16;
    p[l(6)].rotate_angle_z += -var16;
    p[l(7)].rotate_angle_z += var17;
    p[l(8)].rotate_angle_z += -var17;
}
