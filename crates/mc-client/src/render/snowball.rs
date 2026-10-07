//! `RenderSnowball` (RenderSnowball.java): the snowball's and the egg's
//! `items.png` icon as a camera-facing quad, half a block wide.

use super::Ctx;
use crate::gui::batch::gl;
use crate::textures::ITEMS_PNG;

/// `Item.snowball.getIconFromDamage(0)`: `setIconCoord(14, 0)`.
// src: Item.java:350, RenderManager.java:45
pub const SNOWBALL_ICON: i32 = 14;
/// `Item.egg.getIconFromDamage(0)`: `setIconCoord(12, 0)`.
// src: Item.java:362, RenderManager.java:46
pub const EGG_ICON: i32 = 12;

/// A camera-facing icon quad of `items.png` (`RenderSnowball.doRender`'s
/// and `RenderFireball`'s body): `scale` wide, its centre a quarter up.
// src: RenderSnowball.java:12-37, RenderFireball.java:6-33
pub fn billboard(cx: &mut Ctx, icon: i32, x: f64, y: f64, z: f64, scale: f32) {
    let (vy, vx) = (cx.info.player_view_y, cx.info.player_view_x);
    let g = &mut *cx.gl;
    g.push_matrix();
    g.translatef(x as f32, y as f32, z as f32);
    g.enable(gl::RESCALE_NORMAL);
    g.scalef(scale, scale, scale);
    g.bind_texture(ITEMS_PNG);
    let var11 = (icon % 16 * 16) as f32 / 256.0;
    let var12 = (icon % 16 * 16 + 16) as f32 / 256.0;
    let var13 = (icon / 16 * 16) as f32 / 256.0;
    let var14 = (icon / 16 * 16 + 16) as f32 / 256.0;
    let var15 = 1.0f32;
    let var16 = 0.5f32;
    let var17 = 0.25f32;
    g.rotatef(180.0 - vy, 0.0, 1.0, 0.0);
    g.rotatef(-vx, 1.0, 0.0, 0.0);
    g.start_drawing_quads();
    g.set_normal(0.0, 1.0, 0.0);
    g.add_vertex_with_uv(
        (0.0 - var16) as f64,
        (0.0 - var17) as f64,
        0.0,
        var11 as f64,
        var14 as f64,
    );
    g.add_vertex_with_uv(
        (var15 - var16) as f64,
        (0.0 - var17) as f64,
        0.0,
        var12 as f64,
        var14 as f64,
    );
    g.add_vertex_with_uv(
        (var15 - var16) as f64,
        (1.0 - var17) as f64,
        0.0,
        var12 as f64,
        var13 as f64,
    );
    g.add_vertex_with_uv(
        (0.0 - var16) as f64,
        (1.0 - var17) as f64,
        0.0,
        var11 as f64,
        var13 as f64,
    );
    g.draw();
    g.disable(gl::RESCALE_NORMAL);
    g.pop_matrix();
}

/// `doRender(entity, x, y, z, yaw, partial)`.
// src: RenderSnowball.java:12-37
pub fn do_render(cx: &mut Ctx, icon: i32, x: f64, y: f64, z: f64) {
    billboard(cx, icon, x, y, z, 0.5);
}
