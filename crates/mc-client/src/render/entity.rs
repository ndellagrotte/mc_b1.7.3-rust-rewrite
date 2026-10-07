//! `RenderEntity` (RenderEntity.java): an entity with no renderer of its
//! own draws its bounding box, white, untextured. No b1.7.3 entity class
//! reaches it (every concrete class has a renderer).

use mc_sim::entity::Entity;

use super::Ctx;

/// `doRender(entity, x, y, z, yaw, partial)`.
// src: RenderEntity.java:6-10
pub fn do_render(cx: &mut Ctx, var1: &Entity, var2: f64, var4: f64, var6: f64) {
    cx.gl.push_matrix();
    let b = &var1.base;
    super::base::render_offset_aabb(
        cx,
        &b.bounding_box,
        var2 - b.last_tick_pos_x,
        var4 - b.last_tick_pos_y,
        var6 - b.last_tick_pos_z,
    );
    cx.gl.pop_matrix();
}
