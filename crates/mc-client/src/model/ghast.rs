//! `ModelGhast` (ModelGhast.java): a 16³ body and nine tentacles whose
//! lengths come from `new Random(1660L)`.

use mc_core::java_random::JavaRandom;
use mc_core::math::sin;

use super::Model;
use super::part::{ModelPart, RenderMode};

/// `body`.
pub const BODY: usize = 0;
/// `tentacles[i]` is `TENTACLE0 + i`.
pub const TENTACLE0: usize = 1;
/// `tentacles.length`.
pub const TENTACLES: usize = 9;

/// `render`'s draw list: the body, then the tentacles in order.
// src: ModelGhast.java:37-45
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
    (9, RenderMode::Render),
];

const TENTACLE_NAMES: [&str; TENTACLES] = [
    "tentacles[0]",
    "tentacles[1]",
    "tentacles[2]",
    "tentacles[3]",
    "tentacles[4]",
    "tentacles[5]",
    "tentacles[6]",
    "tentacles[7]",
    "tentacles[8]",
];

/// The tentacle lengths: `new Random(1660L).nextInt(7) + 8`, nine draws.
// src: ModelGhast.java:15, :21
pub fn tentacle_lengths() -> [i32; TENTACLES] {
    let mut var2 = JavaRandom::new(1660);
    std::array::from_fn(|_| var2.next_int_bounded(7) + 8)
}

/// `new ModelGhast()`: the tentacles on a 3×3 grid under the body, the
/// middle row shifted half a step.
// src: ModelGhast.java:9-28
pub fn build() -> Vec<ModelPart> {
    let var1: i32 = -16;
    let mut body = ModelPart::new("body", 0, 0);
    body.add_box(-8.0, -8.0, -8.0, 16, 16, 16);
    body.rotation_point_y += (24 + var1) as f32;
    let lengths = tentacle_lengths();
    let mut parts = vec![body];
    for (var3, &var6) in lengths.iter().enumerate() {
        let var3 = var3 as i32;
        let mut t = ModelPart::new(TENTACLE_NAMES[var3 as usize], 0, 0);
        let var4 =
            (((var3 % 3) as f32 - (var3 / 3 % 2) as f32 * 0.5 + 0.25) / 2.0 * 2.0 - 1.0) * 5.0;
        let var5 = ((var3 / 3) as f32 / 2.0 * 2.0 - 1.0) * 5.0;
        t.add_box(-1.0, 0.0, -1.0, 2, var6, 2);
        t.rotation_point_x = var4;
        t.rotation_point_z = var5;
        t.rotation_point_y = (31 + var1) as f32;
        parts.push(t);
    }
    parts
}

/// `ModelGhast.setRotationAngles`: each tentacle swings by `ticks`.
// src: ModelGhast.java:30-35
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
        m.parts[TENTACLE0 + var7].rotate_angle_x = 0.2 * sin(var3 * 0.3 + var7 as f32) + 0.4;
    }
}
