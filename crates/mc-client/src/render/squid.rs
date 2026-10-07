//! `RenderSquid` (RenderSquid.java): the body turned by the squid's own
//! pitch and yaw about a point 0.5 up, then lowered 1.2; the tentacle
//! angle as the model's `ticks`.

use mc_sim::entity::{Entity, EntityKindData as K};

use super::Ctx;

/// `func_21007_a(squid, ticks, bodyYaw, partial)` (its `rotateCorpse`).
// src: RenderSquid.java:14-22
pub fn func_21007_a(cx: &mut Ctx, var1: &Entity, _var2: f32, var3: f32, var4: f32) {
    let K::Squid(_, _, _, s) = &var1.kind else {
        return;
    };
    let var5 = s.prev_squid_pitch + (s.squid_pitch - s.prev_squid_pitch) * var4;
    let var6 = s.prev_squid_yaw + (s.squid_yaw - s.prev_squid_yaw) * var4;
    cx.gl.translatef(0.0, 0.5, 0.0);
    cx.gl.rotatef(180.0 - var3, 0.0, 1.0, 0.0);
    cx.gl.rotatef(var5, 1.0, 0.0, 0.0);
    cx.gl.rotatef(var6, 0.0, 1.0, 0.0);
    cx.gl.translatef(0.0, -1.2, 0.0);
}

/// `func_21006_b(squid, partial)`: the interpolated tentacle angle.
// src: RenderSquid.java:27-30
pub fn func_21006_b(var1: &Entity, var2: f32) -> f32 {
    let K::Squid(_, _, _, s) = &var1.kind else {
        return 0.0;
    };
    s.last_tentacle_angle + (s.tentacle_angle - s.last_tentacle_angle) * var2
}
