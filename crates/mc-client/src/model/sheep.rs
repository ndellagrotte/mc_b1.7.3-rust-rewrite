//! `ModelSheep2` (the sheared body, RenderSheep's main model) and
//! `ModelSheep1` (the wool, its render-pass model): `ModelQuadruped(12, 0)`
//! with their own head and body (ModelSheep1.java, ModelSheep2.java).

use super::part::ModelPart;
use super::quadruped::{self, BODY, HEAD, LEG1, LEG2, LEG3, LEG4};

/// `new ModelSheep2()`.
// src: ModelSheep2.java:4-11
pub fn build_sheep2() -> Vec<ModelPart> {
    let mut p = quadruped::build(12, 0.0);
    let mut head = ModelPart::new("head", 0, 0);
    head.add_box_inflated(-3.0, -4.0, -6.0, 6, 6, 8, 0.0);
    head.set_rotation_point(0.0, 6.0, -8.0);
    let mut body = ModelPart::new("body", 28, 8);
    body.add_box_inflated(-4.0, -10.0, -7.0, 8, 16, 6, 0.0);
    body.set_rotation_point(0.0, 5.0, 2.0);
    p[HEAD] = head;
    p[BODY] = body;
    p
}

/// `new ModelSheep1()`: the wool, every box inflated (head 0.6, body 1.75,
/// legs 0.5, the legs 6 high).
// src: ModelSheep1.java:4-24
pub fn build_sheep1() -> Vec<ModelPart> {
    let mut p = quadruped::build(12, 0.0);
    let mut head = ModelPart::new("head", 0, 0);
    head.add_box_inflated(-3.0, -4.0, -4.0, 6, 6, 6, 0.6);
    head.set_rotation_point(0.0, 6.0, -8.0);
    let mut body = ModelPart::new("body", 28, 8);
    body.add_box_inflated(-4.0, -10.0, -7.0, 8, 16, 6, 1.75);
    body.set_rotation_point(0.0, 5.0, 2.0);
    let var1 = 0.5f32;
    let leg = |name: &'static str, x: f32, z: f32| {
        let mut l = ModelPart::new(name, 0, 16);
        l.add_box_inflated(-2.0, 0.0, -2.0, 4, 6, 4, var1);
        l.set_rotation_point(x, 12.0, z);
        l
    };
    p[HEAD] = head;
    p[BODY] = body;
    p[LEG1] = leg("leg1", -3.0, 7.0);
    p[LEG2] = leg("leg2", 3.0, 7.0);
    p[LEG3] = leg("leg3", -3.0, -5.0);
    p[LEG4] = leg("leg4", 3.0, -5.0);
    p
}
