//! `Render.renderShadow` / `renderShadowOnBlock` (Render.java:91-160): the
//! round `/misc/shadow.png` (clamped) projected on the top faces of the
//! normal cubes under the entity, `shadowSize` around it and from its feet
//! down `shadowSize` blocks, each face's quad at `1/64` above the block,
//! blended with an alpha of `(opacity - height above the face / 2) × 0.5 ×
//! the light of the block above` (none where that block's light is 3 or
//! less), without depth writes. Drawn only with fancy graphics
//! ([`super::base::do_render_shadow_and_fire`]).

use mc_core::math::floor_double;
use mc_sim::block::{self, BlockBounds};
use mc_sim::entity::{Entity, EntityKindData as K};

use super::Ctx;
use super::base::RenderBase;
use crate::gui::batch::gl;

/// The shadow texture.
// src: Render.java:95
pub const SHADOW_PNG: &str = "%clamp%/misc/shadow.png";

/// `entity.getShadowSize()` with the render overrides: 0 for the arrow,
/// the boat, the egg, the falling sand, the fireball, the fish hook, the
/// minecart, the snowball and the primed TNT; half the height otherwise.
// src: Entity.java:869-871, EntityArrow.java:268-270, EntityBoat.java:332-334,
// EntityEgg.java:244-246, EntityFallingSand.java:81-83, EntityFireball.java:221-223,
// EntityFish.java:342-344, EntityMinecart.java:642-644, EntitySnowball.java:231-233,
// EntityTNTPrimed.java:79-81
pub fn get_shadow_size(e: &Entity) -> f32 {
    match e.kind {
        K::Arrow(..)
        | K::Boat(..)
        | K::Egg(..)
        | K::FallingSand(..)
        | K::Fireball(..)
        | K::Fish(..)
        | K::Minecart(..)
        | K::Snowball(..)
        | K::TntPrimed(..) => 0.0,
        _ => e.base.height / 2.0,
    }
}

/// `renderShadow(entity, x, y, z, opacity, partial)`.
// src: Render.java:91-129
#[allow(clippy::too_many_arguments)]
pub fn render_shadow(
    r: &RenderBase,
    cx: &mut Ctx,
    var1: &Entity,
    var2: f64,
    var4: f64,
    var6: f64,
    var8: f32,
    var9: f32,
) {
    cx.gl.enable(gl::BLEND);
    cx.gl.blend_func(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
    cx.gl.bind_texture(SHADOW_PNG);
    cx.gl.depth_mask(false);
    let var12 = r.shadow_size;
    let b = &var1.base;
    let shadow = get_shadow_size(var1) as f64;
    let p = var9 as f64;
    let var13 = b.last_tick_pos_x + (b.pos_x - b.last_tick_pos_x) * p;
    let var15 = b.last_tick_pos_y + (b.pos_y - b.last_tick_pos_y) * p + shadow;
    let var17 = b.last_tick_pos_z + (b.pos_z - b.last_tick_pos_z) * p;
    let var19 = floor_double(var13 - var12 as f64);
    let var20 = floor_double(var13 + var12 as f64);
    let var21 = floor_double(var15 - var12 as f64);
    let var22 = floor_double(var15);
    let var23 = floor_double(var17 - var12 as f64);
    let var24 = floor_double(var17 + var12 as f64);
    let var25 = var2 - var13;
    let var27 = var4 - var15;
    let var29 = var6 - var17;
    cx.gl.start_drawing_quads();
    for var32 in var19..=var20 {
        for var33 in var21..=var22 {
            for var34 in var23..=var24 {
                let var35 = cx.world.get_block_id(var32, var33 - 1, var34);
                if var35 > 0 && cx.world.get_block_light_value(var32, var33, var34) > 3 {
                    render_shadow_on_block(
                        cx,
                        var35,
                        var2,
                        var4 + shadow,
                        var6,
                        [var32, var33, var34],
                        var8,
                        var12,
                        var25,
                        var27 + shadow,
                        var29,
                    );
                }
            }
        }
    }
    cx.gl.draw();
    cx.gl.color4f(1.0, 1.0, 1.0, 1.0);
    cx.gl.disable(gl::BLEND);
    cx.gl.depth_mask(true);
}

/// The shadow's alpha on the face at height `var9` (block y): `(opacity -
/// (y - (var9 + offset)) / 2) × 0.5 × light`, `None` below 0, at most 1.
// src: Render.java:138-144
pub fn shadow_alpha(var11: f32, var4: f64, var9: i32, var15: f64, light: f32) -> Option<f64> {
    let mut var20 = (var11 as f64 - (var4 - (var9 as f64 + var15)) / 2.0) * 0.5 * light as f64;
    if var20 >= 0.0 {
        if var20 > 1.0 {
            var20 = 1.0;
        }
        Some(var20)
    } else {
        None
    }
}

/// `renderShadowOnBlock(block, x, y, z, bx, by, bz, opacity, size, dx, dy,
/// dz)`: the quad on the block's top at its current bounds (the
/// singleton's `minX` … `maxZ`).
// src: Render.java:135-160
#[allow(clippy::too_many_arguments)]
fn render_shadow_on_block(
    cx: &mut Ctx,
    id: i32,
    var2: f64,
    var4: f64,
    var6: f64,
    [var8, var9, var10]: [i32; 3],
    var11: f32,
    var12: f32,
    var13: f64,
    var15: f64,
    var17: f64,
) {
    let Some(def) = block::block(id) else {
        return;
    };
    if !block::render_as_normal_block(def.kind) {
        return;
    }
    let light = cx.world.get_light_brightness(var8, var9, var10);
    let Some(var20) = shadow_alpha(var11, var4, var9, var15, light) else {
        return;
    };
    let bb: BlockBounds = cx.world.block_statics.bounds[id as usize];
    cx.gl.set_color_rgba_f(1.0, 1.0, 1.0, var20 as f32);
    let var22 = var8 as f64 + bb.min_x + var13;
    let var24 = var8 as f64 + bb.max_x + var13;
    let var26 = var9 as f64 + bb.min_y + var15 + 0.015625;
    let var28 = var10 as f64 + bb.min_z + var17;
    let var30 = var10 as f64 + bb.max_z + var17;
    let var32 = ((var2 - var22) / 2.0 / var12 as f64 + 0.5) as f32;
    let var33 = ((var2 - var24) / 2.0 / var12 as f64 + 0.5) as f32;
    let var34 = ((var6 - var28) / 2.0 / var12 as f64 + 0.5) as f32;
    let var35 = ((var6 - var30) / 2.0 / var12 as f64 + 0.5) as f32;
    let g = &mut *cx.gl;
    g.add_vertex_with_uv(var22, var26, var28, var32 as f64, var34 as f64);
    g.add_vertex_with_uv(var22, var26, var30, var32 as f64, var35 as f64);
    g.add_vertex_with_uv(var24, var26, var30, var33 as f64, var35 as f64);
    g.add_vertex_with_uv(var24, var26, var28, var33 as f64, var34 as f64);
}
