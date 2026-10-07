//! `ModelSquid` (ModelSquid.java): a 12×16×12 body and eight tentacles in a
//! ring.

use mc_core::math::jmath;

use super::Model;
use super::part::{ModelPart, RenderMode};

/// `squidBody`.
pub const BODY: usize = 0;
/// `squidTentacles[i]` is `TENTACLE0 + i`.
pub const TENTACLE0: usize = 1;
/// `squidTentacles.length`.
pub const TENTACLES: usize = 8;

/// `render`'s draw list: the body, then the tentacles.
// src: ModelSquid.java:36-44
pub const RENDER: &[(usize, RenderMode)] = &[
    (BODY, RenderMode::Render),
    (1, RenderMode::Render),
    (2, RenderMode::Render),
    (3, RenderMode::Render),
    (4, RenderMode::Render),
    (5, RenderMode::Render),
    (6, RenderMode::Render),
    (7, RenderMode::Render),
    (8, RenderMode::Render),
];

const TENTACLE_NAMES: [&str; TENTACLES] = [
    "squidTentacles[0]",
    "squidTentacles[1]",
    "squidTentacles[2]",
    "squidTentacles[3]",
    "squidTentacles[4]",
    "squidTentacles[5]",
    "squidTentacles[6]",
    "squidTentacles[7]",
];

/// `new ModelSquid()`: the tentacles on a circle of radius 5 under the
/// body (`Math.cos`/`Math.sin` in doubles), each turned to face outwards.
// src: ModelSquid.java:7-27
pub fn build() -> Vec<ModelPart> {
    let var1: i32 = -16;
    let mut body = ModelPart::new("squidBody", 0, 0);
    body.add_box(-6.0, -8.0, -6.0, 12, 16, 12);
    body.rotation_point_y += (24 + var1) as f32;
    let mut parts = vec![body];
    for (var2, name) in TENTACLE_NAMES.iter().enumerate() {
        let mut t = ModelPart::new(name, 48, 0);
        let mut var3 = var2 as f64 * 3.141592653589793 * 2.0 / TENTACLES as f64;
        let var5 = jmath::cos(var3) as f32 * 5.0;
        let var6 = jmath::sin(var3) as f32 * 5.0;
        t.add_box(-1.0, 0.0, -1.0, 2, 18, 2);
        t.rotation_point_x = var5;
        t.rotation_point_z = var6;
        t.rotation_point_y = (31 + var1) as f32;
        var3 = var2 as f64 * 3.141592653589793 * -2.0 / TENTACLES as f64 + 1.5707963267948966;
        t.rotate_angle_y = var3 as f32;
        parts.push(t);
    }
    parts
}

/// `ModelSquid.setRotationAngles`: every tentacle's X angle is `ticks`
/// (`RenderSquid` passes the interpolated tentacle angle there).
// src: ModelSquid.java:29-34
pub fn set_rotation_angles(
    m: &mut Model,
    _var1: f32,
    _var2: f32,
    var3: f32,
    _var4: f32,
    _var5: f32,
    _var6: f32,
) {
    for var7 in 0..TENTACLES {
        m.parts[TENTACLE0 + var7].rotate_angle_x = var3;
    }
}
