//! `ModelWolf` (ModelWolf.java), with `setLivingAnimations` (sitting, the
//! tail wag, the head tilt and the wet-shake) fed by [`WolfAnimInputs`].

use mc_core::math::cos;
use mc_sim::entity::{Entity, wolf};

use super::Model;
use super::part::{ModelPart, RenderMode};

/// `wolfHeadMain`.
pub const HEAD: usize = 0;
/// `wolfBody`.
pub const BODY: usize = 1;
/// `wolfLeg1`.
pub const LEG1: usize = 2;
/// `wolfLeg2`.
pub const LEG2: usize = 3;
/// `wolfLeg3`.
pub const LEG3: usize = 4;
/// `wolfLeg4`.
pub const LEG4: usize = 5;
/// `wolfRightEar`.
pub const RIGHT_EAR: usize = 6;
/// `wolfLeftEar`.
pub const LEFT_EAR: usize = 7;
/// `wolfSnout`.
pub const SNOUT: usize = 8;
/// `wolfTail`.
pub const TAIL: usize = 9;
/// `wolfMane`.
pub const MANE: usize = 10;

/// `render`'s draw list: the head, ears, snout and tail with
/// `renderWithRotation` (Y, X, Z), the rest with `render`.
// src: ModelWolf.java:56-70
pub const RENDER: &[(usize, RenderMode)] = &[
    (HEAD, RenderMode::WithRotation),
    (BODY, RenderMode::Render),
    (LEG1, RenderMode::Render),
    (LEG2, RenderMode::Render),
    (LEG3, RenderMode::Render),
    (LEG4, RenderMode::Render),
    (RIGHT_EAR, RenderMode::WithRotation),
    (LEFT_EAR, RenderMode::WithRotation),
    (SNOUT, RenderMode::WithRotation),
    (TAIL, RenderMode::WithRotation),
    (MANE, RenderMode::Render),
];

/// `new ModelWolf()`.
// src: ModelWolf.java:18-54
pub fn build() -> Vec<ModelPart> {
    let var1 = 0.0f32;
    let var2 = 13.5f32;
    let mut head = ModelPart::new("wolfHeadMain", 0, 0);
    head.add_box_inflated(-3.0, -3.0, -2.0, 6, 6, 4, var1);
    head.set_rotation_point(-1.0, var2, -7.0);
    let mut body = ModelPart::new("wolfBody", 18, 14);
    body.add_box_inflated(-4.0, -2.0, -3.0, 6, 9, 6, var1);
    body.set_rotation_point(0.0, 14.0, 2.0);
    let mut mane = ModelPart::new("wolfMane", 21, 0);
    mane.add_box_inflated(-4.0, -3.0, -3.0, 8, 6, 7, var1);
    mane.set_rotation_point(-1.0, 14.0, 2.0);
    let leg = |name: &'static str, x: f32, z: f32| {
        let mut l = ModelPart::new(name, 0, 18);
        l.add_box_inflated(-1.0, 0.0, -1.0, 2, 8, 2, var1);
        l.set_rotation_point(x, 16.0, z);
        l
    };
    let leg1 = leg("wolfLeg1", -2.5, 7.0);
    let leg2 = leg("wolfLeg2", 0.5, 7.0);
    let leg3 = leg("wolfLeg3", -2.5, -4.0);
    let leg4 = leg("wolfLeg4", 0.5, -4.0);
    let mut tail = ModelPart::new("wolfTail", 9, 18);
    tail.add_box_inflated(-1.0, 0.0, -1.0, 2, 8, 2, var1);
    tail.set_rotation_point(-1.0, 12.0, 8.0);
    let mut right_ear = ModelPart::new("wolfRightEar", 16, 14);
    right_ear.add_box_inflated(-3.0, -5.0, 0.0, 2, 2, 1, var1);
    right_ear.set_rotation_point(-1.0, var2, -7.0);
    let mut left_ear = ModelPart::new("wolfLeftEar", 16, 14);
    left_ear.add_box_inflated(1.0, -5.0, 0.0, 2, 2, 1, var1);
    left_ear.set_rotation_point(-1.0, var2, -7.0);
    let mut snout = ModelPart::new("wolfSnout", 0, 10);
    snout.add_box_inflated(-2.0, 0.0, -5.0, 3, 3, 4, var1);
    snout.set_rotation_point(-0.5, var2, -7.0);
    vec![
        head, body, leg1, leg2, leg3, leg4, right_ear, left_ear, snout, tail, mane,
    ]
}

/// What `ModelWolf.setLivingAnimations` reads of the `EntityWolf` at the
/// frame's partial tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WolfAnimInputs {
    /// `isWolfAngry()`.
    pub angry: bool,
    /// `isWolfSitting()`.
    pub sitting: bool,
    /// `getInterestedAngle(partial)`.
    pub interested_angle: f32,
    /// `getShakeAngle(partial, o)` for `o` = 0, -0.08, -0.16, -0.2.
    pub shake_angle: [f32; 4],
    /// `getWolfShaking()`.
    pub shaking: bool,
    /// `getEntityBrightness(partial)` (the caller's light lookup).
    pub brightness: f32,
    /// `getShadingWhileShaking(partial)`.
    pub shading_while_shaking: f32,
}

/// The `getShakeAngle` offsets, in [`WolfAnimInputs::shake_angle`]'s order.
// src: ModelWolf.java:111-118
pub const SHAKE_OFFSETS: [f32; 4] = [0.0, -0.08, -0.16, -0.2];

impl WolfAnimInputs {
    /// Reads them from a wolf through `mc_sim::entity::wolf`, with the
    /// entity's brightness passed in (`getEntityBrightness` needs the
    /// world).
    pub fn from_entity(e: &Entity, partial: f32, brightness: f32) -> Self {
        WolfAnimInputs {
            angry: wolf::is_wolf_angry(e),
            sitting: wolf::is_wolf_sitting(e),
            interested_angle: wolf::get_interested_angle(e, partial),
            shake_angle: SHAKE_OFFSETS.map(|o| wolf::get_shake_angle(e, partial, o)),
            shaking: wolf::get_wolf_shaking(e),
            brightness,
            shading_while_shaking: wolf::get_shading_while_shaking(e, partial),
        }
    }
}

/// `ModelWolf.setLivingAnimations(wolf, limbSwing, limbSwingAmount,
/// partial)`: the tail wag (still when angry), the sitting or standing pose
/// (legs' walk swing when standing), the head tilt plus shake on the head,
/// ears and snout, the shake on mane, body and tail. Returns the
/// `glColor3f` it sets while the wolf shakes (brightness × shading).
// src: ModelWolf.java:72-124
pub fn set_living_animations(
    m: &mut Model,
    var5: &WolfAnimInputs,
    var2: f32,
    var3: f32,
) -> Option<[f32; 3]> {
    let p = &mut m.parts;
    if var5.angry {
        p[TAIL].rotate_angle_y = 0.0;
    } else {
        p[TAIL].rotate_angle_y = cos(var2 * 0.6662) * 1.4 * var3;
    }

    if var5.sitting {
        p[MANE].set_rotation_point(-1.0, 16.0, -3.0);
        p[MANE].rotate_angle_x = 1.2566371;
        p[MANE].rotate_angle_y = 0.0;
        p[BODY].set_rotation_point(0.0, 18.0, 0.0);
        p[BODY].rotate_angle_x = 0.7853982;
        p[TAIL].set_rotation_point(-1.0, 21.0, 6.0);
        p[LEG1].set_rotation_point(-2.5, 22.0, 2.0);
        p[LEG1].rotate_angle_x = 4.712389;
        p[LEG2].set_rotation_point(0.5, 22.0, 2.0);
        p[LEG2].rotate_angle_x = 4.712389;
        p[LEG3].rotate_angle_x = 5.811947;
        p[LEG3].set_rotation_point(-2.49, 17.0, -4.0);
        p[LEG4].rotate_angle_x = 5.811947;
        p[LEG4].set_rotation_point(0.51, 17.0, -4.0);
    } else {
        p[BODY].set_rotation_point(0.0, 14.0, 2.0);
        p[BODY].rotate_angle_x = 1.5707964;
        p[MANE].set_rotation_point(-1.0, 14.0, -3.0);
        p[MANE].rotate_angle_x = p[BODY].rotate_angle_x;
        p[TAIL].set_rotation_point(-1.0, 12.0, 8.0);
        p[LEG1].set_rotation_point(-2.5, 16.0, 7.0);
        p[LEG2].set_rotation_point(0.5, 16.0, 7.0);
        p[LEG3].set_rotation_point(-2.5, 16.0, -4.0);
        p[LEG4].set_rotation_point(0.5, 16.0, -4.0);
        p[LEG1].rotate_angle_x = cos(var2 * 0.6662) * 1.4 * var3;
        p[LEG2].rotate_angle_x = cos(var2 * 0.6662 + 3.1415927) * 1.4 * var3;
        p[LEG3].rotate_angle_x = cos(var2 * 0.6662 + 3.1415927) * 1.4 * var3;
        p[LEG4].rotate_angle_x = cos(var2 * 0.6662) * 1.4 * var3;
    }

    let var6 = var5.interested_angle + var5.shake_angle[0];
    p[HEAD].rotate_angle_z = var6;
    p[RIGHT_EAR].rotate_angle_z = var6;
    p[LEFT_EAR].rotate_angle_z = var6;
    p[SNOUT].rotate_angle_z = var6;
    p[MANE].rotate_angle_z = var5.shake_angle[1];
    p[BODY].rotate_angle_z = var5.shake_angle[2];
    p[TAIL].rotate_angle_z = var5.shake_angle[3];
    if var5.shaking {
        let var7 = var5.brightness * var5.shading_while_shaking;
        return Some([var7, var7, var7]);
    }
    None
}

/// `ModelWolf.setRotationAngles`: head look copied to the ears and snout;
/// the tail's X angle is `ticks` (`EntityWolf.setTailRotation()` through
/// `RenderWolf.func_25004_a`).
// src: ModelWolf.java:126-137
pub fn set_rotation_angles(
    m: &mut Model,
    _var1: f32,
    _var2: f32,
    var3: f32,
    var4: f32,
    var5: f32,
    _var6: f32,
) {
    let p = &mut m.parts;
    p[HEAD].rotate_angle_x = var5 / 57.295776;
    p[HEAD].rotate_angle_y = var4 / 57.295776;
    let (hy, hx) = (p[HEAD].rotate_angle_y, p[HEAD].rotate_angle_x);
    for i in [RIGHT_EAR, LEFT_EAR, SNOUT] {
        p[i].rotate_angle_y = hy;
        p[i].rotate_angle_x = hx;
    }
    p[TAIL].rotate_angle_x = var3;
}
