//! `Render` (Render.java): the fields every renderer has, the shadow and
//! fire pass after `doRender` (`doRenderShadowAndFire`), and the two
//! bounding-box helpers. The shadow is [`super::shadow`], the fire
//! [`super::fire`].

use mc_core::Aabb;
use mc_sim::entity::Entity;

use super::Ctx;
use crate::gui::batch::gl;

/// `Render`'s fields.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderBase {
    /// `shadowSize`.
    // src: Render.java:8
    pub shadow_size: f32,
    /// `field_194_c`: the shadow's opacity factor (later MCP:
    /// `shadowOpaque`).
    // src: Render.java:9
    pub field_194_c: f32,
}

impl Default for RenderBase {
    // src: Render.java:8-9
    fn default() -> Self {
        RenderBase {
            shadow_size: 0.0,
            field_194_c: 1.0,
        }
    }
}

impl RenderBase {
    /// A renderer with `shadowSize = s`.
    pub fn with_shadow(s: f32) -> Self {
        RenderBase {
            shadow_size: s,
            ..RenderBase::default()
        }
    }
}

/// `doRenderShadowAndFire(entity, x, y, z, yaw, partial)`: with fancy
/// graphics and a shadow, the shadow at `(1 - d²/256) × field_194_c`
/// opacity of the squared distance to the viewer; then the fire on a
/// burning entity.
// src: Render.java:236-249
#[allow(clippy::too_many_arguments)]
pub fn do_render_shadow_and_fire(
    r: &RenderBase,
    cx: &mut Ctx,
    var1: &Entity,
    var2: f64,
    var4: f64,
    var6: f64,
    _var8: f32,
    var9: f32,
) {
    if cx.info.fancy_graphics && r.shadow_size > 0.0 {
        let var10 = cx
            .info
            .func_851_a(var1.base.pos_x, var1.base.pos_y, var1.base.pos_z);
        let var12 = shadow_opacity(var10, r.field_194_c);
        if var12 > 0.0 {
            super::shadow::render_shadow(r, cx, var1, var2, var4, var6, var12, var9);
        }
    }
    if var1.base.is_burning() {
        super::fire::render_entity_on_fire(cx, var1, var2, var4, var6, var9);
    }
}

/// `(float)((1.0D - d² / 256.0D) * (double)field_194_c)`.
// src: Render.java:238-239
pub fn shadow_opacity(dist_sq: f64, field_194_c: f32) -> f32 {
    ((1.0 - dist_sq / 256.0) * field_194_c as f64) as f32
}

/// `renderOffsetAABB(box, x, y, z)`: the box's six faces, untextured,
/// white, with their normals, offset by `(x, y, z)`.
// src: Render.java:161-200
pub fn render_offset_aabb(cx: &mut Ctx, var0: &Aabb, var1: f64, var3: f64, var5: f64) {
    let g = &mut *cx.gl;
    g.disable(gl::TEXTURE_2D);
    g.color4f(1.0, 1.0, 1.0, 1.0);
    g.start_drawing_quads();
    g.set_translation_d(var1, var3, var5);
    let b = var0;
    g.set_normal(0.0, 0.0, -1.0);
    g.add_vertex(b.min_x, b.max_y, b.min_z);
    g.add_vertex(b.max_x, b.max_y, b.min_z);
    g.add_vertex(b.max_x, b.min_y, b.min_z);
    g.add_vertex(b.min_x, b.min_y, b.min_z);
    g.set_normal(0.0, 0.0, 1.0);
    g.add_vertex(b.min_x, b.min_y, b.max_z);
    g.add_vertex(b.max_x, b.min_y, b.max_z);
    g.add_vertex(b.max_x, b.max_y, b.max_z);
    g.add_vertex(b.min_x, b.max_y, b.max_z);
    g.set_normal(0.0, -1.0, 0.0);
    g.add_vertex(b.min_x, b.min_y, b.min_z);
    g.add_vertex(b.max_x, b.min_y, b.min_z);
    g.add_vertex(b.max_x, b.min_y, b.max_z);
    g.add_vertex(b.min_x, b.min_y, b.max_z);
    g.set_normal(0.0, 1.0, 0.0);
    g.add_vertex(b.min_x, b.max_y, b.max_z);
    g.add_vertex(b.max_x, b.max_y, b.max_z);
    g.add_vertex(b.max_x, b.max_y, b.min_z);
    g.add_vertex(b.min_x, b.max_y, b.min_z);
    g.set_normal(-1.0, 0.0, 0.0);
    g.add_vertex(b.min_x, b.min_y, b.max_z);
    g.add_vertex(b.min_x, b.max_y, b.max_z);
    g.add_vertex(b.min_x, b.max_y, b.min_z);
    g.add_vertex(b.min_x, b.min_y, b.min_z);
    g.set_normal(1.0, 0.0, 0.0);
    g.add_vertex(b.max_x, b.min_y, b.min_z);
    g.add_vertex(b.max_x, b.max_y, b.min_z);
    g.add_vertex(b.max_x, b.max_y, b.max_z);
    g.add_vertex(b.max_x, b.min_y, b.max_z);
    g.set_translation_d(0.0, 0.0, 0.0);
    g.draw();
    g.enable(gl::TEXTURE_2D);
}

/// `renderAABB(box)`: the six faces without normals or colour (vanilla
/// calls it from no renderer in b1.7.3; kept for completeness).
// src: Render.java:202-230
pub fn render_aabb(cx: &mut Ctx, var0: &Aabb) {
    let g = &mut *cx.gl;
    let b = var0;
    g.start_drawing_quads();
    for (x, y, z) in [
        (b.min_x, b.max_y, b.min_z),
        (b.max_x, b.max_y, b.min_z),
        (b.max_x, b.min_y, b.min_z),
        (b.min_x, b.min_y, b.min_z),
        (b.min_x, b.min_y, b.max_z),
        (b.max_x, b.min_y, b.max_z),
        (b.max_x, b.max_y, b.max_z),
        (b.min_x, b.max_y, b.max_z),
        (b.min_x, b.min_y, b.min_z),
        (b.max_x, b.min_y, b.min_z),
        (b.max_x, b.min_y, b.max_z),
        (b.min_x, b.min_y, b.max_z),
        (b.min_x, b.max_y, b.max_z),
        (b.max_x, b.max_y, b.max_z),
        (b.max_x, b.max_y, b.min_z),
        (b.min_x, b.max_y, b.min_z),
        (b.min_x, b.min_y, b.max_z),
        (b.min_x, b.max_y, b.max_z),
        (b.min_x, b.max_y, b.min_z),
        (b.min_x, b.min_y, b.min_z),
        (b.max_x, b.min_y, b.min_z),
        (b.max_x, b.max_y, b.min_z),
        (b.max_x, b.max_y, b.max_z),
        (b.max_x, b.min_y, b.max_z),
    ] {
        g.add_vertex(x, y, z);
    }
    g.draw();
}
