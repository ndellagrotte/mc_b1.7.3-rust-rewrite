//! `FontRenderer` drawn in 3D (project-owned adapter): the sign's text and
//! the F3 entity labels call `fontRenderer.drawString` under the 3D
//! modelview. WP-3.6a's [`FontRenderer`] draws into the GUI batcher
//! ([`GuiGl`]); here it draws into a scratch `GuiGl` at the identity, and
//! each glyph quad (with the colour the batcher gave it: the call's colour
//! or a `§` code's) is replayed into [`EntityGl`] under the current
//! modelview and GL state, with the font's texture bound.

use super::gl::EntityGl;
use crate::gui::batch::GuiGl;
use crate::gui::font::FontRenderer;

/// `fontRenderer.drawString(s, x, y, color)` into `gl` (module docs).
// src: FontRenderer.java:120-181
pub fn draw_string(gl: &mut EntityGl, font: &FontRenderer, s: &str, x: i32, y: i32, color: i32) {
    let mut g = GuiGl::default();
    font.draw_string(&mut g, s, x, y, color);
    for d in g.take_draws() {
        if let Some(t) = d.texture {
            gl.bind_texture(t);
        }
        let b = &d.batch;
        gl.start_drawing_quads();
        for i in 0..b.positions.len() {
            let p = b.positions[i];
            gl.add_vertex_with_uv_color(
                [p[0] as f64, p[1] as f64, p[2] as f64],
                b.uvs[i],
                b.colors[i],
            );
        }
        gl.draw();
    }
}
