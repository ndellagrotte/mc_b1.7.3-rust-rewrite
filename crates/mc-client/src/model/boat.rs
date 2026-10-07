//! `ModelBoat` (ModelBoat.java): the bottom and four sides.

// The Java's `0 + var4` and `-var1 / 2` shapes are kept as written.
#![allow(clippy::identity_op)]

use super::part::{ModelPart, RenderMode};

/// `render`'s draw list: `boatSides[0..5]`.
// src: ModelBoat.java:28-33
pub const RENDER: &[(usize, RenderMode)] = &[
    (0, RenderMode::Render),
    (1, RenderMode::Render),
    (2, RenderMode::Render),
    (3, RenderMode::Render),
    (4, RenderMode::Render),
];

/// `new ModelBoat()`.
// src: ModelBoat.java:6-26
pub fn build() -> Vec<ModelPart> {
    let mut s = [
        ModelPart::new("boatSides[0]", 0, 8),
        ModelPart::new("boatSides[1]", 0, 0),
        ModelPart::new("boatSides[2]", 0, 0),
        ModelPart::new("boatSides[3]", 0, 0),
        ModelPart::new("boatSides[4]", 0, 0),
    ];
    let var1: i32 = 24;
    let var2: i32 = 6;
    let var3: i32 = 20;
    let var4: i32 = 4;
    s[0].add_box_inflated(
        (-var1 / 2) as f32,
        (-var3 / 2 + 2) as f32,
        -3.0,
        var1,
        var3 - 4,
        4,
        0.0,
    );
    s[0].set_rotation_point(0.0, (0 + var4) as f32, 0.0);
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
    s.into()
}
