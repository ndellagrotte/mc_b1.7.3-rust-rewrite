//! `Gui` (Gui.java): the drawing helpers every screen and widget inherits,
//! over the GUI's GL ([`GuiGl`]). Java's instance methods become free
//! functions; the one that reads `this.zLevel` takes it.

use super::batch::{GuiGl, gl};
use super::font::FontRenderer;

/// `1 / 256`: `drawTexturedModalRect`'s texel size (a 256×256 sheet).
// src: Gui.java:99-100
const TEXEL: f32 = 0.00390625;

/// `drawHorizontalLine(x1, x2, y, color)`: a 1-pixel row from `x1` to `x2`
/// inclusive.
// src: Gui.java:8-16
pub fn draw_horizontal_line(g: &mut GuiGl, mut var1: i32, mut var2: i32, var3: i32, var4: i32) {
    if var2 < var1 {
        std::mem::swap(&mut var1, &mut var2);
    }
    draw_rect(g, var1, var3, var2 + 1, var3 + 1, var4);
}

/// `drawVerticalLine(x, y1, y2, color)`: a 1-pixel column strictly between
/// `y1` and `y2` (vanilla draws from `y1 + 1` to `y2 - 1`).
// src: Gui.java:18-26
pub fn draw_vertical_line(g: &mut GuiGl, var1: i32, mut var2: i32, mut var3: i32, var4: i32) {
    if var3 < var2 {
        std::mem::swap(&mut var2, &mut var3);
    }
    draw_rect(g, var1, var2 + 1, var1 + 1, var3, var4);
}

/// `drawRect(x1, y1, x2, y2, 0xAARRGGBB)`: an untextured, alpha-blended
/// rectangle at z 0. Leaves `GL_TEXTURE_2D` on and `GL_BLEND` off.
// src: Gui.java:28-59
pub fn draw_rect(
    g: &mut GuiGl,
    mut var1: i32,
    mut var2: i32,
    mut var3: i32,
    mut var4: i32,
    var5: i32,
) {
    if var1 < var3 {
        std::mem::swap(&mut var1, &mut var3);
    }
    if var2 < var4 {
        std::mem::swap(&mut var2, &mut var4);
    }
    let var11 = (var5 >> 24 & 255) as f32 / 255.0;
    let var7 = (var5 >> 16 & 255) as f32 / 255.0;
    let var8 = (var5 >> 8 & 255) as f32 / 255.0;
    let var9 = (var5 & 255) as f32 / 255.0;
    g.enable(gl::BLEND);
    g.disable(gl::TEXTURE_2D);
    g.blend_func(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
    g.color4f(var7, var8, var9, var11);
    g.start_drawing_quads();
    g.add_vertex(var1 as f64, var4 as f64, 0.0);
    g.add_vertex(var3 as f64, var4 as f64, 0.0);
    g.add_vertex(var3 as f64, var2 as f64, 0.0);
    g.add_vertex(var1 as f64, var2 as f64, 0.0);
    g.draw();
    g.enable(gl::TEXTURE_2D);
    g.disable(gl::BLEND);
}

/// `drawGradientRect(x1, y1, x2, y2, topColor, bottomColor)`: a vertical
/// gradient (smooth shading, no alpha test). Leaves `GL_FLAT`, blending
/// off, the alpha test and texturing on.
// src: Gui.java:61-88
pub fn draw_gradient_rect(
    g: &mut GuiGl,
    var1: i32,
    var2: i32,
    var3: i32,
    var4: i32,
    var5: i32,
    var6: i32,
) {
    let var7 = (var5 >> 24 & 255) as f32 / 255.0;
    let var8 = (var5 >> 16 & 255) as f32 / 255.0;
    let var9 = (var5 >> 8 & 255) as f32 / 255.0;
    let var10 = (var5 & 255) as f32 / 255.0;
    let var11 = (var6 >> 24 & 255) as f32 / 255.0;
    let var12 = (var6 >> 16 & 255) as f32 / 255.0;
    let var13 = (var6 >> 8 & 255) as f32 / 255.0;
    let var14 = (var6 & 255) as f32 / 255.0;
    g.disable(gl::TEXTURE_2D);
    g.enable(gl::BLEND);
    g.disable(gl::ALPHA_TEST);
    g.blend_func(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
    g.shade_model(gl::SMOOTH);
    g.start_drawing_quads();
    g.set_color_rgba_f(var8, var9, var10, var7);
    g.add_vertex(var3 as f64, var2 as f64, 0.0);
    g.add_vertex(var1 as f64, var2 as f64, 0.0);
    g.set_color_rgba_f(var12, var13, var14, var11);
    g.add_vertex(var1 as f64, var4 as f64, 0.0);
    g.add_vertex(var3 as f64, var4 as f64, 0.0);
    g.draw();
    g.shade_model(gl::FLAT);
    g.disable(gl::BLEND);
    g.enable(gl::ALPHA_TEST);
    g.enable(gl::TEXTURE_2D);
}

/// `drawCenteredString(font, s, x, y, color)`: with shadow, centred on `x`
/// (`x - width / 2`, integer division).
// src: Gui.java:90-92
pub fn draw_centered_string(
    g: &mut GuiGl,
    f: &FontRenderer,
    var2: &str,
    var3: i32,
    var4: i32,
    var5: i32,
) {
    f.draw_string_with_shadow(g, var2, var3 - f.get_string_width(var2) / 2, var4, var5);
}

/// `drawString(font, s, x, y, color)`: with shadow.
// src: Gui.java:94-96
pub fn draw_string(g: &mut GuiGl, f: &FontRenderer, var2: &str, var3: i32, var4: i32, var5: i32) {
    f.draw_string_with_shadow(g, var2, var3, var4, var5);
}

/// `drawTexturedModalRect(x, y, u, v, w, h)` at `zLevel`: a `w × h` piece
/// of the bound 256×256 sheet from texel `(u, v)`.
// src: Gui.java:98-108
#[allow(clippy::too_many_arguments, clippy::identity_op)]
pub fn draw_textured_modal_rect(
    g: &mut GuiGl,
    z_level: f32,
    var1: i32,
    var2: i32,
    var3: i32,
    var4: i32,
    var5: i32,
    var6: i32,
) {
    let var7 = TEXEL;
    let var8 = TEXEL;
    let z = z_level as f64;
    g.start_drawing_quads();
    g.add_vertex_with_uv(
        (var1 + 0) as f64,
        (var2 + var6) as f64,
        z,
        ((var3 + 0) as f32 * var7) as f64,
        ((var4 + var6) as f32 * var8) as f64,
    );
    g.add_vertex_with_uv(
        (var1 + var5) as f64,
        (var2 + var6) as f64,
        z,
        ((var3 + var5) as f32 * var7) as f64,
        ((var4 + var6) as f32 * var8) as f64,
    );
    g.add_vertex_with_uv(
        (var1 + var5) as f64,
        (var2 + 0) as f64,
        z,
        ((var3 + var5) as f32 * var7) as f64,
        ((var4 + 0) as f32 * var8) as f64,
    );
    g.add_vertex_with_uv(
        (var1 + 0) as f64,
        (var2 + 0) as f64,
        z,
        ((var3 + 0) as f32 * var7) as f64,
        ((var4 + 0) as f32 * var8) as f64,
    );
    g.draw();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_swaps_corners_and_restores_state() {
        let mut g = GuiGl::default();
        draw_rect(&mut g, 10, 20, 0, 5, 0x80ff0000u32 as i32);
        let d = &g.draws()[0];
        assert_eq!(d.texture, None);
        assert_eq!(d.key.blend, Some((gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA)));
        assert_eq!(d.batch.positions[0], [10.0, 5.0, 0.0]);
        assert_eq!(d.batch.positions[2], [0.0, 20.0, 0.0]);
        assert_eq!(d.batch.colors[0], [1.0, 0.0, 0.0, 128.0 / 255.0]);
        assert!(g.state.texture_2d && !g.state.blend);
        // Lines: horizontal inclusive, vertical between the ends.
        let mut g = GuiGl::default();
        draw_horizontal_line(&mut g, 5, 1, 3, -1);
        draw_vertical_line(&mut g, 2, 9, 4, -1);
        let p = &g.draws()[0].batch.positions;
        assert_eq!((p[0], p[2]), ([6.0, 3.0, 0.0], [1.0, 4.0, 0.0]));
        assert_eq!((p[4], p[6]), ([3.0, 5.0, 0.0], [2.0, 9.0, 0.0]));
    }

    #[test]
    fn gradient_is_smooth_without_alpha_test() {
        let mut g = GuiGl::default();
        draw_gradient_rect(&mut g, 0, 0, 4, 8, -1072689136, -804253680);
        let d = &g.draws()[0];
        assert_eq!(d.alpha_ref, None);
        // The Tessellator's bytes: (int)((b / 255F) * 255F).
        let q = |b: i32| (((b as f32 / 255.0) * 255.0) as i32) as f32 / 255.0;
        let top = [q(16), q(16), q(16), q(192)];
        let bottom = [q(16), q(16), q(16), q(208)];
        assert_eq!(d.batch.colors[0], top);
        assert_eq!(d.batch.colors[2], bottom);
        assert!(!g.state.smooth && g.state.alpha_test);
    }

    #[test]
    fn textured_modal_rect_uvs() {
        let mut g = GuiGl::default();
        g.bind_texture("/gui/gui.png");
        draw_textured_modal_rect(&mut g, -90.0, 3, 4, 0, 46, 100, 20);
        let b = &g.draws()[0].batch;
        assert_eq!(b.positions[0], [3.0, 24.0, -90.0]);
        assert_eq!(b.uvs[0], [0.0, 66.0 / 256.0]);
        assert_eq!(b.uvs[2], [100.0 / 256.0, 46.0 / 256.0]);
    }
}
