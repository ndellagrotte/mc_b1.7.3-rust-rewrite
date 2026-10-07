//! `ModelMinecart` (ModelMinecart.java): the floor, four sides and the
//! inner floor (`sideModels[5]`); `sideModels[6]` is never created.

// The Java's `0 + var4` and `-var1 / 2` shapes are kept as written.
#![allow(clippy::identity_op)]

use super::part::{ModelPart, RenderMode};

/// `sideModels[5]`, the inner floor `render` moves.
pub const INNER: usize = 5;

/// `render`'s draw list: `sideModels[0..6]`.
// src: ModelMinecart.java:36-43
pub const RENDER: &[(usize, RenderMode)] = &[
    (0, RenderMode::Render),
    (1, RenderMode::Render),
    (2, RenderMode::Render),
    (3, RenderMode::Render),
    (4, RenderMode::Render),
    (5, RenderMode::Render),
];

/// `new ModelMinecart()` (the six created parts).
// src: ModelMinecart.java:6-34
pub fn build() -> Vec<ModelPart> {
    let mut s = [
        ModelPart::new("sideModels[0]", 0, 10),
        ModelPart::new("sideModels[1]", 0, 0),
        ModelPart::new("sideModels[2]", 0, 0),
        ModelPart::new("sideModels[3]", 0, 0),
        ModelPart::new("sideModels[4]", 0, 0),
        ModelPart::new("sideModels[5]", 44, 10),
    ];
    let var1: i32 = 20;
    let var2: i32 = 8;
    let var3: i32 = 16;
    let var4: i32 = 4;
    s[0].add_box_inflated(
        (-var1 / 2) as f32,
        (-var3 / 2) as f32,
        -1.0,
        var1,
        var3,
        2,
        0.0,
    );
    s[0].set_rotation_point(0.0, (0 + var4) as f32, 0.0);
    s[5].add_box_inflated(
        (-var1 / 2 + 1) as f32,
        (-var3 / 2 + 1) as f32,
        -1.0,
        var1 - 2,
        var3 - 2,
        1,
        0.0,
    );
    s[5].set_rotation_point(0.0, (0 + var4) as f32, 0.0);
    for p in &mut s[1..5] {
        p.add_box_inflated(
            (-var1 / 2 + 2) as f32,
            (-var2 - 1) as f32,
            -1.0,
            var1 - 4,
            var2,
            2,
            0.0,
        );
    }
    s[1].set_rotation_point((-var1 / 2 + 1) as f32, (0 + var4) as f32, 0.0);
    s[2].set_rotation_point((var1 / 2 - 1) as f32, (0 + var4) as f32, 0.0);
    s[3].set_rotation_point(0.0, (0 + var4) as f32, (-var3 / 2 + 1) as f32);
    s[4].set_rotation_point(0.0, (0 + var4) as f32, (var3 / 2 - 1) as f32);
    s[0].rotate_angle_x = 1.5707964;
    s[1].rotate_angle_y = 4.712389;
    s[2].rotate_angle_y = 1.5707964;
    s[3].rotate_angle_y = 3.1415927;
    s[5].rotate_angle_x = -1.5707964;
    s.into()
}
