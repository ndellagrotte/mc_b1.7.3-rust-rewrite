//! `RenderMinecart` (RenderMinecart.java): `ModelMinecart` with
//! `/item/cart.png`, placed on the rail (`getPos` and the two points 0.3
//! along the track, `getPosOffset`, give the position, the yaw from the
//! track's direction and the pitch `atan(dy) × 73`), rocking after a hit;
//! the chest or furnace cart carries its block (`renderBlockOnInventory`,
//! scaled 0.75). Face culling stays on.

use mc_core::jmath;
use mc_core::math::sin;
use mc_sim::entity::{Entity, EntityKindData as K, minecart as ecart};

use super::Ctx;
use super::base::RenderBase;
use crate::model::{Model, ModelKind};
use crate::textures::TERRAIN_PNG;

/// `Block.chest`'s id.
// src: Block.java:646
pub const CHEST: i32 = 54;
/// `Block.stoneOvenIdle`'s id (the idle furnace).
// src: Block.java:653
pub const FURNACE: i32 = 61;

/// `RenderMinecart`'s fields.
#[derive(Clone, Debug)]
pub struct RenderMinecart {
    /// `shadowSize` 0.5.
    pub base: RenderBase,
    /// `modelMinecart`.
    pub model: Model,
}

impl Default for RenderMinecart {
    // src: RenderMinecart.java:8-11
    fn default() -> Self {
        RenderMinecart {
            base: RenderBase::with_shadow(0.5),
            model: Model::new(ModelKind::Minecart),
        }
    }
}

/// The rock angle after a hit: `sin(t) × t × damage / 10 × direction`
/// with `t = timeSinceHit - partial`, `damage = currentDamage - partial`
/// (at least 0); `None` once `t <= 0`.
// src: RenderMinecart.java:46-54, RenderBoat.java:17-25
pub fn rock_angle(
    time_since_hit: i32,
    current_damage: i32,
    direction: i32,
    var9: f32,
) -> Option<f32> {
    let var23 = time_since_hit as f32 - var9;
    let mut var24 = current_damage as f32 - var9;
    if var24 < 0.0 {
        var24 = 0.0;
    }
    (var23 > 0.0).then(|| sin(var23) * var23 * var24 / 10.0 * direction as f32)
}

impl RenderMinecart {
    /// `func_152_a(cart, x, y, z, yaw, partial)`.
    // src: RenderMinecart.java:13-77
    #[allow(clippy::too_many_arguments)]
    pub fn do_render(
        &mut self,
        cx: &mut Ctx,
        var1: &Entity,
        mut var2: f64,
        mut var4: f64,
        mut var6: f64,
        mut var8: f32,
        var9: f32,
    ) {
        let K::Minecart(m) = &var1.kind else {
            return;
        };
        let b = &var1.base;
        cx.gl.push_matrix();
        let p = var9 as f64;
        let var10 = b.last_tick_pos_x + (b.pos_x - b.last_tick_pos_x) * p;
        let var12 = b.last_tick_pos_y + (b.pos_y - b.last_tick_pos_y) * p;
        let var14 = b.last_tick_pos_z + (b.pos_z - b.last_tick_pos_z) * p;
        let var16 = 0.30000001192092896f64;
        let var18 = ecart::get_pos(cx.world, var10, var12, var14);
        let mut var19 = b.prev_rotation_pitch + (b.rotation_pitch - b.prev_rotation_pitch) * var9;
        if let Some(var18) = var18 {
            let var20 =
                ecart::get_pos_offset(cx.world, var10, var12, var14, var16).unwrap_or(var18);
            let var21 =
                ecart::get_pos_offset(cx.world, var10, var12, var14, -var16).unwrap_or(var18);
            var2 += var18.x_coord - var10;
            var4 += (var20.y_coord + var21.y_coord) / 2.0 - var12;
            var6 += var18.z_coord - var14;
            let mut var22 = var21.add_vector(-var20.x_coord, -var20.y_coord, -var20.z_coord);
            if var22.length_vector() != 0.0 {
                var22 = var22.normalize();
                var8 = (jmath::atan2(var22.z_coord, var22.x_coord) * 180.0 / std::f64::consts::PI)
                    as f32;
                var19 = (jmath::atan(var22.y_coord) * 73.0) as f32;
            }
        }
        cx.gl.translatef(var2 as f32, var4 as f32, var6 as f32);
        cx.gl.rotatef(180.0 - var8, 0.0, 1.0, 0.0);
        cx.gl.rotatef(-var19, 0.0, 0.0, 1.0);
        if let Some(a) = rock_angle(
            m.minecart_time_since_hit,
            m.minecart_current_damage,
            m.minecart_rock_direction,
            var9,
        ) {
            cx.gl.rotatef(a, 1.0, 0.0, 0.0);
        }
        if m.minecart_type != 0 {
            cx.load_texture(TERRAIN_PNG);
            let var25 = 0.75f32;
            cx.gl.scalef(var25, var25, var25);
            cx.gl.translatef(0.0, 0.3125, 0.0);
            cx.gl.rotatef(90.0, 0.0, 1.0, 0.0);
            if m.minecart_type == 1 {
                let br = cx.brightness(var1, var9);
                cx.render_block_on_inventory(CHEST, 0, br);
            } else if m.minecart_type == 2 {
                let br = cx.brightness(var1, var9);
                cx.render_block_on_inventory(FURNACE, 0, br);
            }
            cx.gl.rotatef(-90.0, 0.0, 1.0, 0.0);
            cx.gl.translatef(0.0, -0.3125, 0.0);
            cx.gl.scalef(1.0 / var25, 1.0 / var25, 1.0 / var25);
        }
        cx.load_texture("/item/cart.png");
        cx.gl.scalef(-1.0, -1.0, 1.0);
        let draws = self.model.render(0.0, 0.0, -0.1, 0.0, 0.0, 0.0625);
        cx.gl.model(self.model.kind, draws);
        cx.gl.pop_matrix();
    }
}
