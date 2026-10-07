//! `RenderBoat` (RenderBoat.java): `ModelBoat` with `/item/boat.png`,
//! rocking after a hit. Face culling stays on.

use mc_sim::entity::{Entity, EntityKindData as K};

use super::Ctx;
use super::base::RenderBase;
use super::minecart::rock_angle;
use crate::model::{Model, ModelKind};
use crate::textures::TERRAIN_PNG;

/// `RenderBoat`'s fields.
#[derive(Clone, Debug)]
pub struct RenderBoat {
    /// `shadowSize` 0.5.
    pub base: RenderBase,
    /// `modelBoat`.
    pub model: Model,
}

impl Default for RenderBoat {
    // src: RenderBoat.java:8-11
    fn default() -> Self {
        RenderBoat {
            base: RenderBase::with_shadow(0.5),
            model: Model::new(ModelKind::Boat),
        }
    }
}

impl RenderBoat {
    /// `func_157_a(boat, x, y, z, yaw, partial)`.
    // src: RenderBoat.java:13-35
    #[allow(clippy::too_many_arguments)]
    pub fn do_render(
        &mut self,
        cx: &mut Ctx,
        var1: &Entity,
        var2: f64,
        var4: f64,
        var6: f64,
        var8: f32,
        var9: f32,
    ) {
        let K::Boat(bd) = &var1.kind else {
            return;
        };
        cx.gl.push_matrix();
        cx.gl.translatef(var2 as f32, var4 as f32, var6 as f32);
        cx.gl.rotatef(180.0 - var8, 0.0, 1.0, 0.0);
        if let Some(a) = rock_angle(
            bd.boat_time_since_hit,
            bd.boat_current_damage,
            bd.boat_rock_direction,
            var9,
        ) {
            cx.gl.rotatef(a, 1.0, 0.0, 0.0);
        }
        cx.load_texture(TERRAIN_PNG);
        let var12 = 0.75f32;
        cx.gl.scalef(var12, var12, var12);
        cx.gl.scalef(1.0 / var12, 1.0 / var12, 1.0 / var12);
        cx.load_texture("/item/boat.png");
        cx.gl.scalef(-1.0, -1.0, 1.0);
        let draws = self.model.render(0.0, 0.0, -0.1, 0.0, 0.0, 0.0625);
        cx.gl.model(self.model.kind, draws);
        cx.gl.pop_matrix();
    }
}
