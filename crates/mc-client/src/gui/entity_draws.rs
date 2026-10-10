//! The entity renderers' draws in the GUI (WP-4.9; project-owned, PLAN_v4
//! D7): `GuiInventory` draws the player with
//! `RenderManager.renderEntityWithPosYaw(thePlayer, 0, 0, 0, 0, 1)` under
//! the screen's matrix (GuiInventory.java:28-60), so the preview runs
//! WP-3.5b's renderers ([`crate::render::render_entity_with_pos_yaw`]:
//! `RenderPlayer` with its armour passes, held item, hurt and death
//! overlay, then `doRenderShadowAndFire`) into an [`EntityGl`] started at
//! the identity ([`EntityGl::gui_start`]), and [`emit`] turns the
//! resulting [`DrawList`] into [`GuiGl`] batches under the GUI's current
//! matrix.
//!
//! - **Lights.** The renderers light on the CPU in the space of their
//!   start matrix; GL lit in eye space with the lights `GuiInventory` set
//!   under its matrix. [`lights_in_start_space`] carries the eye-space
//!   directions back through the start matrix (its inverse, renormalised:
//!   the matrix is a uniform scale, a mirror and rotations), which gives the
//!   same dot products.
//! - **Models** are tessellated here from the parts' display lists under
//!   the draw's root and part transforms (what [`crate::render::apply`]
//!   does on the GPU with the shared part meshes), lit with the same
//!   formula as the model material (normals normalised), coloured with
//!   `glColor`, back faces dropped when `GL_CULL_FACE` was on.
//! - **Tessellator draws** keep their lit colours; triangles go to the
//!   quad batcher as degenerate quads, smooth-shaded (each vertex already
//!   holds its provoking vertex's colour where GL was flat); line draws
//!   are not drawn (none of the player's renderers makes one).
//! - **Order.** Every model draw first, then the Tessellator draws. Vanilla
//!   interleaves them (the held item comes between the armour passes and
//!   the hurt overlay; the shadow and the fire come last); the overlay only
//!   tints pixels whose depth equals the model's, so moving the held item
//!   after it changes no pixel.
//! - **State.** Blending, the alpha test, the depth mask and `GL_EQUAL`
//!   (the overlay) map to the batcher's; the depth test stays on, as in the
//!   GUI; fog is ignored (the batcher has none).

use std::collections::BTreeMap;

use bevy::math::{Mat3, Mat4, Vec3};

use super::batch::{GuiGl, TexRef, gl};
use super::render_item::{is_front_facing, lit_color};
use crate::gl_material::GlBlend;
use crate::model::{MODEL_SCALE, Model, ModelKind};
use crate::render::gl::{DrawList, EntityGl, ModelDraw, TessDraw, Topology};

/// The eye-space light directions `lights` (as
/// [`super::render_item::ItemGl::lights`] holds them) in the space of the
/// start matrix `m` of an [`EntityGl`] (module docs, "Lights").
pub fn lights_in_start_space(m: Mat4, lights: [Vec3; 2]) -> [Vec3; 2] {
    let inv = Mat3::from_mat4(m).inverse();
    lights.map(|l| (inv * l).normalize_or_zero())
}

/// A batcher state switch for one draw.
fn apply_state(
    g: &mut GuiGl,
    texture: &Option<TexRef>,
    alpha_ref: Option<f32>,
    blend: Option<(u32, u32)>,
    depth_write: bool,
    depth_equal: bool,
) {
    match texture {
        Some(t) => {
            g.enable(gl::TEXTURE_2D);
            g.bind_texture(t.clone());
        }
        None => g.disable(gl::TEXTURE_2D),
    }
    match alpha_ref {
        Some(r) => {
            g.enable(gl::ALPHA_TEST);
            g.alpha_func(gl::GREATER, r);
        }
        None => g.disable(gl::ALPHA_TEST),
    }
    match blend {
        Some((s, d)) => {
            g.enable(gl::BLEND);
            g.blend_func(s, d);
        }
        None => g.disable(gl::BLEND),
    }
    g.depth_mask(depth_write);
    g.depth_func(if depth_equal { 514 } else { 515 });
}

/// The vertex's colour as the batcher stores it (bytes).
fn put(g: &mut GuiGl, p: Vec3, uv: [f32; 2], c: [f32; 4]) {
    let b = c.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as i32);
    g.set_color_rgba(b[0], b[1], b[2], b[3]);
    g.add_vertex_with_uv(
        p.x as f64,
        p.y as f64,
        p.z as f64,
        uv[0] as f64,
        uv[1] as f64,
    );
}

/// The GL blend function of a model draw's state.
fn model_blend(d: &ModelDraw) -> Option<(u32, u32)> {
    match (d.spec.blend, d.spec.src_one) {
        (GlBlend::Off, _) => None,
        (GlBlend::Alpha, _) => Some((gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA)),
        (GlBlend::Additive, false) => Some((gl::SRC_ALPHA, gl::ONE)),
        (GlBlend::Additive, true) => Some((gl::ONE, gl::ONE)),
    }
}

/// One model draw's parts as lit quads (module docs, "Models"); `m` is the
/// GUI matrix (for the facing test), `lights` in the start space.
fn emit_model(
    g: &mut GuiGl,
    m: Mat4,
    d: &ModelDraw,
    lights: &[Vec3; 2],
    models: &mut BTreeMap<ModelKind, Model>,
) {
    let model = models.entry(d.kind).or_insert_with(|| Model::new(d.kind));
    apply_state(
        g,
        &d.spec.texture,
        d.spec.alpha_ref,
        model_blend(d),
        d.spec.depth_write,
        d.spec.depth_equal,
    );
    g.start_drawing_quads();
    for pd in &d.parts {
        let Some(part) = model.parts.get(pd.part) else {
            continue;
        };
        let v = part.display_list(MODEL_SCALE);
        let t = d.root * pd.transform.to_matrix();
        let nm = Mat3::from_mat4(t).inverse().transpose();
        for q in (0..v.positions.len() / 4 * 4).step_by(4) {
            let ps: [Vec3; 4] =
                [0, 1, 2, 3].map(|k| t.transform_point3(Vec3::from(v.positions[q + k])));
            if d.spec.cull {
                let w = ps.map(|p| {
                    let s = m.transform_point3(p);
                    [s.x, s.y]
                });
                if !is_front_facing(&w) {
                    continue;
                }
            }
            for (k, &p) in ps.iter().enumerate() {
                let c = if d.spec.lighting {
                    let n = (nm * Vec3::from(v.normals[q + k])).normalize_or_zero();
                    lit_color(d.spec.color, n, lights)
                } else {
                    d.spec.color
                };
                let uv = v.uvs[q + k];
                put(
                    g,
                    p,
                    [uv[0] + d.spec.uv_offset[0], uv[1] + d.spec.uv_offset[1]],
                    c,
                );
            }
        }
    }
    g.draw();
}

/// One Tessellator batch (module docs, "Tessellator draws").
fn emit_tess(g: &mut GuiGl, m: Mat4, d: &TessDraw) {
    if d.spec.topology != Topology::Triangles {
        return;
    }
    let k = &d.spec.key;
    apply_state(
        g,
        &d.spec.texture,
        d.spec.alpha_ref,
        k.blend,
        k.depth_write,
        k.depth_equal,
    );
    g.start_drawing_quads();
    for t in (0..d.positions.len() / 3 * 3).step_by(3) {
        let ps = [0, 1, 2].map(|i| Vec3::from(d.positions[t + i]));
        if k.cull {
            let w = [ps[0], ps[1], ps[2], ps[2]].map(|p| {
                let s = m.transform_point3(p);
                [s.x, s.y]
            });
            if !is_front_facing(&w) {
                continue;
            }
        }
        for i in [0, 1, 2, 2] {
            put(g, ps[i], d.uvs[t + i], d.colors[t + i]);
        }
    }
    g.draw();
}

/// Draws `list` (made by an [`EntityGl`] started at the identity) under
/// the batcher's current matrix `m`, with `lights` in the start space
/// (module docs). Leaves the batcher smooth-shaded as it found it, with
/// texturing, the alpha test and `GL_LEQUAL` on, blending off and depth
/// writes on.
pub fn emit(g: &mut GuiGl, m: Mat4, list: &DrawList, lights: [Vec3; 2]) {
    let smooth = g.state.smooth;
    g.shade_model(gl::SMOOTH);
    let mut models = BTreeMap::new();
    for d in &list.models {
        emit_model(g, m, d, &lights, &mut models);
    }
    for d in &list.tess {
        emit_tess(g, m, d);
    }
    g.shade_model(if smooth { gl::SMOOTH } else { gl::FLAT });
    g.enable(gl::TEXTURE_2D);
    g.enable(gl::ALPHA_TEST);
    g.disable(gl::BLEND);
    g.depth_mask(true);
    g.depth_func(515);
}

/// The [`EntityGl`] the preview draws with ([`EntityGl::gui_start`]).
pub fn preview_gl(normalize: bool, lights: [Vec3; 2]) -> EntityGl {
    EntityGl::gui_start(normalize, lights)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::material::standard_light_directions;

    #[test]
    fn lights_come_back_through_the_start_matrix() {
        // GuiInventory's matrix: translate, scale(-30, 30, 30), rotate 180
        // about z, 135 about y (the lights), back -135, a tilt.
        let ml = Mat4::from_translation(Vec3::new(100.0, 80.0, 50.0))
            * Mat4::from_scale(Vec3::new(-30.0, 30.0, 30.0))
            * Mat4::from_axis_angle(Vec3::Z, 180f32.to_radians())
            * Mat4::from_axis_angle(Vec3::Y, 135f32.to_radians());
        let eye = standard_light_directions().map(|l| (Mat3::from_mat4(ml) * l).normalize());
        let me = ml
            * Mat4::from_axis_angle(Vec3::Y, (-135f32).to_radians())
            * Mat4::from_axis_angle(Vec3::X, 10f32.to_radians());
        let start = lights_in_start_space(me, eye);
        // A start-space normal lights the same as its eye-space image.
        let n = Vec3::new(0.3, -0.5, 0.8).normalize();
        let n_eye = (Mat3::from_mat4(me).inverse().transpose() * n).normalize();
        for i in 0..2 {
            assert!((n.dot(start[i]) - n_eye.dot(eye[i])).abs() < 1e-5);
        }
    }

    #[test]
    fn a_tess_triangle_becomes_a_degenerate_quad() {
        use crate::render::gl::TessSpec;
        use crate::render::material::EntityGlKey;
        let d = TessDraw {
            spec: TessSpec {
                texture: None,
                alpha_ref: None,
                key: EntityGlKey {
                    cull: false,
                    ..EntityGlKey::default()
                },
                fog: false,
                topology: Topology::Triangles,
            },
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]],
            uvs: vec![[0.0; 2]; 3],
            colors: vec![[1.0, 0.0, 0.0, 1.0]; 3],
        };
        let mut g = GuiGl::default();
        let list = DrawList {
            models: Vec::new(),
            tess: vec![d],
        };
        emit(&mut g, Mat4::IDENTITY, &list, standard_light_directions());
        let draws = g.take_draws();
        assert_eq!(draws.len(), 1);
        assert_eq!(draws[0].batch.len(), 4);
        assert_eq!(draws[0].texture, None);
    }
}
