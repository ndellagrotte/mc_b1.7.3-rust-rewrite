//! `RenderFireball` (RenderFireball.java): the ghast's fireball as the
//! snowball's icon (`Item.snowball.getIconFromDamage(0)`) scaled 2, facing
//! the camera.

use super::Ctx;
use super::snowball::{SNOWBALL_ICON, billboard};

/// `func_4012_a(fireball, x, y, z, yaw, partial)`: `glScalef(2 / 1)`.
// src: RenderFireball.java:6-33
pub fn do_render(cx: &mut Ctx, x: f64, y: f64, z: f64) {
    let var10 = 2.0f32;
    billboard(cx, SNOWBALL_ICON, x, y, z, var10 / 1.0);
}
