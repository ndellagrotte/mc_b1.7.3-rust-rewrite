//! `RenderPig` (RenderPig.java): the saddle (`ModelPig(0.5F)`,
//! `/mob/saddle.png`) on a saddled pig. The texture is bound for every
//! pass, saddled or not, as the Java does.

use mc_sim::entity::{Entity, pig as epig};

use super::Ctx;

/// `renderSaddledPig(pig, pass, partial)`.
// src: RenderPig.java:9-12
pub fn render_saddled_pig(cx: &mut Ctx, var1: &Entity, var2: i32) -> bool {
    cx.load_texture("/mob/saddle.png");
    var2 == 0 && epig::get_saddled(var1)
}
