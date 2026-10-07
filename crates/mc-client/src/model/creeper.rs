//! `ModelCreeper` (ModelCreeper.java); the powered creeper's armour is
//! `new ModelCreeper(2.0F)`.

use mc_core::math::cos;

use super::Model;
use super::part::{ModelPart, RenderMode};

/// `head`.
pub const HEAD: usize = 0;
/// `unusedCreeperHeadwear` (built, never drawn).
pub const UNUSED_HEADWEAR: usize = 1;
/// `body`.
pub const BODY: usize = 2;
/// `leg1`.
pub const LEG1: usize = 3;
/// `leg2`.
pub const LEG2: usize = 4;
/// `leg3`.
pub const LEG3: usize = 5;
/// `leg4`.
pub const LEG4: usize = 6;

/// `render`'s draw list (no headwear).
// src: ModelCreeper.java:41-49
pub const RENDER: &[(usize, RenderMode)] = &[
    (HEAD, RenderMode::Render),
    (BODY, RenderMode::Render),
    (LEG1, RenderMode::Render),
    (LEG2, RenderMode::Render),
    (LEG3, RenderMode::Render),
    (LEG4, RenderMode::Render),
];

/// `new ModelCreeper(inflate)`.
// src: ModelCreeper.java:16-39
pub fn build(var1: f32) -> Vec<ModelPart> {
    let var2: i32 = 4;
    let mut head = ModelPart::new("head", 0, 0);
    head.add_box_inflated(-4.0, -8.0, -4.0, 8, 8, 8, var1);
    head.set_rotation_point(0.0, var2 as f32, 0.0);
    let mut headwear = ModelPart::new("unusedCreeperHeadwear", 32, 0);
    headwear.add_box_inflated(-4.0, -8.0, -4.0, 8, 8, 8, var1 + 0.5);
    headwear.set_rotation_point(0.0, var2 as f32, 0.0);
    let mut body = ModelPart::new("body", 16, 16);
    body.add_box_inflated(-4.0, 0.0, -2.0, 8, 12, 4, var1);
    body.set_rotation_point(0.0, var2 as f32, 0.0);
    let leg = |name: &'static str, x: f32, z: f32| {
        let mut l = ModelPart::new(name, 0, 16);
        l.add_box_inflated(-2.0, 0.0, -2.0, 4, 6, 4, var1);
        l.set_rotation_point(x, (12 + var2) as f32, z);
        l
    };
    vec![
        head,
        headwear,
        body,
        leg("leg1", -2.0, 4.0),
        leg("leg2", 2.0, 4.0),
        leg("leg3", -2.0, -4.0),
        leg("leg4", 2.0, -4.0),
    ]
}

/// `ModelCreeper.setRotationAngles`: head look, the legs' walk swing.
// src: ModelCreeper.java:51-58
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
    p[HEAD].rotate_angle_y = var4 / 57.295776;
    p[HEAD].rotate_angle_x = var5 / 57.295776;
    p[LEG1].rotate_angle_x = cos(var1 * 0.6662) * 1.4 * var2;
    p[LEG2].rotate_angle_x = cos(var1 * 0.6662 + 3.1415927) * 1.4 * var2;
    p[LEG3].rotate_angle_x = cos(var1 * 0.6662 + 3.1415927) * 1.4 * var2;
    p[LEG4].rotate_angle_x = cos(var1 * 0.6662) * 1.4 * var2;
}
