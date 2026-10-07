//! `ModelSlime` (ModelSlime.java): `new ModelSlime(16)` is the inner body
//! with eyes and mouth (RenderSlime's main model), `new ModelSlime(0)` the
//! translucent outer cube (its render-pass model).

use super::part::{ModelPart, RenderMode};

/// `slimeBodies`.
pub const BODY: usize = 0;
/// `slimeRightEye` (only with a texture row above 0).
pub const RIGHT_EYE: usize = 1;
/// `slimeLeftEye`.
pub const LEFT_EYE: usize = 2;
/// `slimeMouth`.
pub const MOUTH: usize = 3;

/// `render`'s draw list (the eyes and mouth only when built).
// src: ModelSlime.java:28-37
pub const RENDER: &[(usize, RenderMode)] = &[
    (BODY, RenderMode::Render),
    (RIGHT_EYE, RenderMode::Render),
    (LEFT_EYE, RenderMode::Render),
    (MOUTH, RenderMode::Render),
];

/// `new ModelSlime(textureRow)`: the 8×8×8 cube at texture row `var1`; with
/// `var1 > 0` it is replaced by a 6×6×6 cube and the eyes and mouth are
/// added. `setRotationAngles` does nothing.
// src: ModelSlime.java:9-23
pub fn build(var1: i32) -> Vec<ModelPart> {
    let mut body = ModelPart::new("slimeBodies", 0, var1);
    body.add_box(-4.0, 16.0, -4.0, 8, 8, 8);
    if var1 > 0 {
        let mut body = ModelPart::new("slimeBodies", 0, var1);
        body.add_box(-3.0, 17.0, -3.0, 6, 6, 6);
        let mut right_eye = ModelPart::new("slimeRightEye", 32, 0);
        right_eye.add_box(-3.25, 18.0, -3.5, 2, 2, 2);
        let mut left_eye = ModelPart::new("slimeLeftEye", 32, 4);
        left_eye.add_box(1.25, 18.0, -3.5, 2, 2, 2);
        let mut mouth = ModelPart::new("slimeMouth", 32, 8);
        mouth.add_box(0.0, 21.0, -3.5, 1, 1, 1);
        return vec![body, right_eye, left_eye, mouth];
    }
    vec![body]
}
