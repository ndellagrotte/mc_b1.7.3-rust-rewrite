//! `ModelZombie` (ModelZombie.java): `ModelBiped` with both arms held
//! forward.

use mc_core::math::{cos, sin};

use super::Model;
use super::biped::{self, LEFT_ARM, RIGHT_ARM};

/// `ModelZombie.setRotationAngles`: the biped's, then the arms forward
/// (`-π/2`), lifted by the attack swing, with the idle sway.
// src: ModelZombie.java:4-26
pub fn set_rotation_angles(
    m: &mut Model,
    var1: f32,
    var2: f32,
    var3: f32,
    var4: f32,
    var5: f32,
    var6: f32,
) {
    biped::set_rotation_angles(m, var1, var2, var3, var4, var5, var6);
    let on_ground = m.on_ground;
    let p = &mut m.parts;
    let var7 = sin(on_ground * 3.1415927);
    let var8 = sin((1.0 - (1.0 - on_ground) * (1.0 - on_ground)) * 3.1415927);
    p[RIGHT_ARM].rotate_angle_z = 0.0;
    p[LEFT_ARM].rotate_angle_z = 0.0;
    p[RIGHT_ARM].rotate_angle_y = -(0.1 - var7 * 0.6);
    p[LEFT_ARM].rotate_angle_y = 0.1 - var7 * 0.6;
    p[RIGHT_ARM].rotate_angle_x = -1.5707964;
    p[LEFT_ARM].rotate_angle_x = -1.5707964;
    p[RIGHT_ARM].rotate_angle_x -= var7 * 1.2 - var8 * 0.4;
    p[LEFT_ARM].rotate_angle_x -= var7 * 1.2 - var8 * 0.4;
    p[RIGHT_ARM].rotate_angle_z += cos(var3 * 0.09) * 0.05 + 0.05;
    p[LEFT_ARM].rotate_angle_z -= cos(var3 * 0.09) * 0.05 + 0.05;
    p[RIGHT_ARM].rotate_angle_x += sin(var3 * 0.067) * 0.05;
    p[LEFT_ARM].rotate_angle_x -= sin(var3 * 0.067) * 0.05;
}
