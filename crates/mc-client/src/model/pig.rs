//! `ModelPig` (ModelPig.java): `ModelQuadruped` with 6-high legs; the
//! saddle pass is `new ModelPig(0.5F)`.

use super::part::ModelPart;
use super::quadruped;

/// `new ModelPig(inflate)` (`ModelPig()` is `inflate` 0).
// src: ModelPig.java:4-10
pub fn build(var1: f32) -> Vec<ModelPart> {
    quadruped::build(6, var1)
}
