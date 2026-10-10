//! The GUI's immediate-mode GL, as data (project-owned; PLAN_v4 D7, §3.5):
//! [`GuiGl`] keeps the fixed-function state the `Gui*` classes set
//! (`glEnable`/`glDisable` of `GL_TEXTURE_2D`, `GL_BLEND`, `GL_ALPHA_TEST`,
//! `GL_DEPTH_TEST`; `glBlendFunc`, `glAlphaFunc`, `glDepthMask`,
//! `glShadeModel`, `glColor`, `glBindTexture`, the modelview matrix stack)
//! and a `Tessellator` (`startDrawingQuads`, `setColor*`, `addVertex*`,
//! `draw`). Each `draw()` becomes a [`GuiDraw`]: its vertices (transformed
//! by the current matrix, coloured as GL colours them) with the state they
//! were drawn under. Consecutive draws under the same state are merged, so
//! a screen is a handful of batches, one per texture or state switch, in
//! call order. [`crate::gui::render`] turns them into Bevy meshes.
//!
//! GL semantics kept:
//! - a vertex's colour is the Tessellator's colour when it set one during
//!   the draw (`hasColor`, Tessellator.java:155, :213), else the current
//!   `glColor`; `glColor3f` sets alpha to 1 (the font's colour codes);
//! - with `glShadeModel(GL_FLAT)` (GL's initial model) a quad takes the
//!   colour of its last vertex (the provoking vertex of `GL_QUADS`);
//! - the alpha test (`GL_GREATER` only, the only function vanilla uses),
//!   blending with any `glBlendFunc` pair, and the depth test (`GL_LEQUAL`,
//!   Minecraft.java:265, or `GL_EQUAL` for the inventory player's hurt
//!   overlay, [`super::entity_draws`]) and mask are kept per batch; `GL_LIGHTING` and
//!   `GL_FOG` are tracked and ignored (the 2D GUI never enables them).
//!
//! Each frame starts from [`GlState::frame_start`] (the state vanilla's GUI
//! code finds in practice: texturing on, blending off, the alpha test at
//! 0.1 from Minecraft.java:266-267, depth test and writes on, flat
//! shading, white) and an identity matrix.

use std::borrow::Cow;

use bevy::prelude::*;

use crate::gl_material::TessBatch;

/// GL enums the GUI uses (`org.lwjgl.opengl.GL11`).
pub mod gl {
    /// `GL_TEXTURE_2D`.
    pub const TEXTURE_2D: u32 = 3553;
    /// `GL_BLEND`.
    pub const BLEND: u32 = 3042;
    /// `GL_ALPHA_TEST`.
    pub const ALPHA_TEST: u32 = 3008;
    /// `GL_DEPTH_TEST`.
    pub const DEPTH_TEST: u32 = 2929;
    /// `GL_LIGHTING`.
    pub const LIGHTING: u32 = 2896;
    /// `GL_FOG`.
    pub const FOG: u32 = 2912;
    /// `GL_CULL_FACE`.
    pub const CULL_FACE: u32 = 2884;
    /// `GL_RESCALE_NORMAL`.
    pub const RESCALE_NORMAL: u32 = 32826;
    /// `GL_SMOOTH`.
    pub const SMOOTH: u32 = 7425;
    /// `GL_FLAT`.
    pub const FLAT: u32 = 7424;
    /// `GL_GREATER`.
    pub const GREATER: u32 = 516;
    /// `GL_ZERO`.
    pub const ZERO: u32 = 0;
    /// `GL_ONE`.
    pub const ONE: u32 = 1;
    /// `GL_SRC_COLOR`.
    pub const SRC_COLOR: u32 = 768;
    /// `GL_ONE_MINUS_SRC_COLOR`.
    pub const ONE_MINUS_SRC_COLOR: u32 = 769;
    /// `GL_SRC_ALPHA`.
    pub const SRC_ALPHA: u32 = 770;
    /// `GL_ONE_MINUS_SRC_ALPHA`.
    pub const ONE_MINUS_SRC_ALPHA: u32 = 771;
    /// `GL_DST_ALPHA`.
    pub const DST_ALPHA: u32 = 772;
    /// `GL_ONE_MINUS_DST_ALPHA`.
    pub const ONE_MINUS_DST_ALPHA: u32 = 773;
    /// `GL_DST_COLOR`.
    pub const DST_COLOR: u32 = 774;
    /// `GL_ONE_MINUS_DST_COLOR`.
    pub const ONE_MINUS_DST_COLOR: u32 = 775;
}

/// A bound texture: a jar name as `renderEngine.getTexture(name)` takes it
/// (resolved through [`crate::textures::Textures`] when drawn), or a
/// texture already uploaded (the font's `allocateAndSetupTexture`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TexRef {
    /// `getTexture(name)`.
    Name(Cow<'static, str>),
    /// A texture handle from [`crate::textures::Textures`].
    Handle(Handle<Image>),
}

impl From<&'static str> for TexRef {
    fn from(s: &'static str) -> Self {
        TexRef::Name(Cow::Borrowed(s))
    }
}

/// Which GUI pass a draw belongs to. Vanilla clears the depth buffer before
/// each (`setupOverlayRendering`, EntityRenderer.java:779; before
/// `drawScreen`, EntityRenderer.java:422): each is a camera of its own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GuiLayer {
    /// `ingameGUI.renderGameOverlay` (WP-3.6b) and the loading screen.
    #[default]
    Overlay,
    /// `currentScreen.drawScreen`.
    Screen,
}

impl GuiLayer {
    /// Both, in drawing order.
    pub const ALL: [GuiLayer; 2] = [GuiLayer::Overlay, GuiLayer::Screen];
}

/// The pipeline half of a batch's state (the material's key).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GuiKey {
    /// `glBlendFunc(src, dst)` with `GL_BLEND` on; `None`: off.
    pub blend: Option<(u32, u32)>,
    /// `GL_DEPTH_TEST` (`GL_LEQUAL`).
    pub depth_test: bool,
    /// `glDepthMask` (GL writes depth only with the test on).
    pub depth_write: bool,
    /// `glDepthFunc(GL_EQUAL)` instead of `GL_LEQUAL` (the hurt overlay of
    /// the inventory's player, `RenderLiving.doRenderLiving`).
    pub depth_equal: bool,
}

impl Default for GuiKey {
    fn default() -> Self {
        GuiKey {
            blend: None,
            depth_test: true,
            depth_write: true,
            depth_equal: false,
        }
    }
}

/// One batch: vertices drawn under one state.
#[derive(Clone, Debug, PartialEq)]
pub struct GuiDraw {
    /// The pass.
    pub layer: GuiLayer,
    /// Blend and depth.
    pub key: GuiKey,
    /// The bound texture with `GL_TEXTURE_2D` on; `None`: texturing off.
    pub texture: Option<TexRef>,
    /// `glAlphaFunc(GL_GREATER, ref)` with `GL_ALPHA_TEST` on; `None`: off.
    pub alpha_ref: Option<f32>,
    /// Quads (4 vertices each), in GUI coordinates (x right, y down, z =
    /// the Java's z).
    pub batch: TessBatch,
}

/// The fixed-function state (module docs).
#[derive(Clone, Debug, PartialEq)]
pub struct GlState {
    /// `GL_TEXTURE_2D`.
    pub texture_2d: bool,
    /// `GL_BLEND`.
    pub blend: bool,
    /// `glBlendFunc`.
    pub blend_func: (u32, u32),
    /// `GL_ALPHA_TEST`.
    pub alpha_test: bool,
    /// `glAlphaFunc(GL_GREATER, ref)`'s reference.
    pub alpha_ref: f32,
    /// `GL_DEPTH_TEST`.
    pub depth_test: bool,
    /// `glDepthMask`.
    pub depth_mask: bool,
    /// `glDepthFunc(GL_EQUAL)` (false: `GL_LEQUAL`).
    pub depth_equal: bool,
    /// `glShadeModel(GL_SMOOTH)` (false: `GL_FLAT`).
    pub smooth: bool,
    /// `GL_LIGHTING` (tracked, not drawn).
    pub lighting: bool,
    /// `GL_FOG` (tracked, not drawn).
    pub fog: bool,
    /// `glColor4f`.
    pub color: [f32; 4],
    /// `glBindTexture`.
    pub bound: Option<TexRef>,
}

impl GlState {
    /// The state a frame's GUI starts from (module docs).
    // src: Minecraft.java:261-267 (GL_TEXTURE_2D, GL_DEPTH_TEST with
    // GL_LEQUAL, GL_ALPHA_TEST with GL_GREATER 0.1); GL_FLAT as renderWorld
    // leaves it (EntityRenderer.java:499) and Gui.drawGradientRect too
    // (Gui.java:84)
    pub fn frame_start() -> Self {
        GlState {
            texture_2d: true,
            blend: false,
            blend_func: (gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA),
            alpha_test: true,
            alpha_ref: 0.1,
            depth_test: true,
            depth_mask: true,
            depth_equal: false,
            smooth: false,
            lighting: false,
            fog: false,
            color: [1.0; 4],
            bound: None,
        }
    }
}

/// A Tessellator vertex: position, texture coordinates, colour (`None`:
/// added before the Tessellator had a colour).
type TessVertex = ([f32; 3], [f32; 2], Option<[f32; 4]>);

/// The Tessellator's state during a draw.
#[derive(Clone, Debug, Default, PartialEq)]
struct Tess {
    drawing: bool,
    /// `hasColor`.
    has_color: bool,
    /// `color` (bytes / 255).
    color: [f32; 4],
    /// `textureU`, `textureV`.
    uv: [f32; 2],
    /// Position, texture coordinates, colour (`None`: set before the
    /// Tessellator had a colour).
    verts: Vec<TessVertex>,
}

/// The GUI's GL (module docs). A resource: the systems that draw take
/// `ResMut<GuiGl>`.
#[derive(Resource, Debug)]
pub struct GuiGl {
    /// The fixed-function state.
    pub state: GlState,
    matrix: Mat4,
    stack: Vec<Mat4>,
    layer: GuiLayer,
    draws: Vec<GuiDraw>,
    tess: Tess,
}

impl Default for GuiGl {
    fn default() -> Self {
        GuiGl {
            state: GlState::frame_start(),
            matrix: Mat4::IDENTITY,
            stack: Vec::new(),
            layer: GuiLayer::Overlay,
            draws: Vec::new(),
            tess: Tess::default(),
        }
    }
}

/// `Tessellator.setColorRGBA`'s clamp of each channel to a byte, / 255.
// src: Tessellator.java:179-219
fn byte_color(r: i32, g: i32, b: i32, a: i32) -> [f32; 4] {
    let q = |v: i32| v.clamp(0, 255) as f32 / 255.0;
    [q(r), q(g), q(b), q(a)]
}

impl GuiGl {
    /// A new frame: no batches, [`GlState::frame_start`], identity matrix,
    /// the overlay pass.
    pub fn begin_frame(&mut self) {
        *self = GuiGl {
            draws: std::mem::take(&mut self.draws),
            ..GuiGl::default()
        };
        self.draws.clear();
    }

    /// Switches the pass later draws go to (module docs of
    /// [`GuiLayer`]); the depth buffer of each pass starts clear.
    pub fn set_layer(&mut self, layer: GuiLayer) {
        self.layer = layer;
    }

    /// The batches so far.
    pub fn draws(&self) -> &[GuiDraw] {
        &self.draws
    }

    /// Takes the batches.
    pub fn take_draws(&mut self) -> Vec<GuiDraw> {
        std::mem::take(&mut self.draws)
    }

    // ---- GL state -------------------------------------------------------------------

    fn cap(&mut self, cap: u32) -> Option<&mut bool> {
        let s = &mut self.state;
        Some(match cap {
            gl::TEXTURE_2D => &mut s.texture_2d,
            gl::BLEND => &mut s.blend,
            gl::ALPHA_TEST => &mut s.alpha_test,
            gl::DEPTH_TEST => &mut s.depth_test,
            gl::LIGHTING => &mut s.lighting,
            gl::FOG => &mut s.fog,
            _ => return None,
        })
    }

    /// `glEnable(cap)` (unknown capabilities are ignored).
    pub fn enable(&mut self, cap: u32) {
        if let Some(c) = self.cap(cap) {
            *c = true;
        }
    }

    /// `glDisable(cap)`.
    pub fn disable(&mut self, cap: u32) {
        if let Some(c) = self.cap(cap) {
            *c = false;
        }
    }

    /// `glBlendFunc(src, dst)`.
    pub fn blend_func(&mut self, src: u32, dst: u32) {
        self.state.blend_func = (src, dst);
    }

    /// `glAlphaFunc(GL_GREATER, ref)` (vanilla uses no other function).
    pub fn alpha_func(&mut self, func: u32, r: f32) {
        debug_assert_eq!(func, gl::GREATER);
        self.state.alpha_ref = r;
    }

    /// `glDepthFunc(func)`: `GL_EQUAL` (514) or `GL_LEQUAL` (515, the
    /// default).
    pub fn depth_func(&mut self, func: u32) {
        self.state.depth_equal = func == 514;
    }

    /// `glDepthMask(flag)`.
    pub fn depth_mask(&mut self, flag: bool) {
        self.state.depth_mask = flag;
    }

    /// `glShadeModel(mode)`.
    pub fn shade_model(&mut self, mode: u32) {
        self.state.smooth = mode == gl::SMOOTH;
    }

    /// `glColor4f(r, g, b, a)`.
    pub fn color4f(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.state.color = [r, g, b, a];
    }

    /// `glColor3f(r, g, b)`: alpha becomes 1.
    pub fn color3f(&mut self, r: f32, g: f32, b: f32) {
        self.state.color = [r, g, b, 1.0];
    }

    /// `glBindTexture(GL_TEXTURE_2D, t)`.
    pub fn bind_texture(&mut self, t: impl Into<TexRef>) {
        self.state.bound = Some(t.into());
    }

    /// `glPushMatrix()`.
    pub fn push_matrix(&mut self) {
        self.stack.push(self.matrix);
    }

    /// `glPopMatrix()`.
    pub fn pop_matrix(&mut self) {
        if let Some(m) = self.stack.pop() {
            self.matrix = m;
        }
    }

    /// `glTranslatef(x, y, z)`.
    pub fn translatef(&mut self, x: f32, y: f32, z: f32) {
        self.matrix *= Mat4::from_translation(Vec3::new(x, y, z));
    }

    /// `glScalef(x, y, z)`.
    pub fn scalef(&mut self, x: f32, y: f32, z: f32) {
        self.matrix *= Mat4::from_scale(Vec3::new(x, y, z));
    }

    /// `glRotatef(angle, x, y, z)` (degrees).
    pub fn rotatef(&mut self, angle: f32, x: f32, y: f32, z: f32) {
        let axis = Vec3::new(x, y, z).normalize_or_zero();
        if axis != Vec3::ZERO {
            self.matrix *= Mat4::from_axis_angle(axis, angle.to_radians());
        }
    }

    /// `glLoadIdentity()` on the modelview matrix.
    pub fn load_identity(&mut self) {
        self.matrix = Mat4::IDENTITY;
    }

    // ---- Tessellator ----------------------------------------------------------------

    /// `startDrawingQuads()`.
    // src: Tessellator.java:143-159
    pub fn start_drawing_quads(&mut self) {
        debug_assert!(!self.tess.drawing, "Already tesselating!");
        self.tess = Tess {
            drawing: true,
            verts: std::mem::take(&mut self.tess.verts),
            ..Tess::default()
        };
        self.tess.verts.clear();
    }

    /// `setColorRGBA(r, g, b, a)` (bytes).
    // src: Tessellator.java:179-219
    pub fn set_color_rgba(&mut self, r: i32, g: i32, b: i32, a: i32) {
        self.tess.has_color = true;
        self.tess.color = byte_color(r, g, b, a);
    }

    /// `setColorRGBA_F(r, g, b, a)`.
    // src: Tessellator.java:171-173
    pub fn set_color_rgba_f(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.set_color_rgba(
            (r * 255.0) as i32,
            (g * 255.0) as i32,
            (b * 255.0) as i32,
            (a * 255.0) as i32,
        );
    }

    /// `setColorOpaque_F(r, g, b)`.
    // src: Tessellator.java:167-169
    pub fn set_color_opaque_f(&mut self, r: f32, g: f32, b: f32) {
        self.set_color_rgba(
            (r * 255.0) as i32,
            (g * 255.0) as i32,
            (b * 255.0) as i32,
            255,
        );
    }

    /// `setColorOpaque_I(0xRRGGBB)`.
    // src: Tessellator.java:275-280
    pub fn set_color_opaque_i(&mut self, c: i32) {
        self.set_color_rgba(c >> 16 & 255, c >> 8 & 255, c & 255, 255);
    }

    /// `setColorRGBA_I(0xRRGGBB, alpha)`.
    // src: Tessellator.java:282-287
    pub fn set_color_rgba_i(&mut self, c: i32, a: i32) {
        self.set_color_rgba(c >> 16 & 255, c >> 8 & 255, c & 255, a);
    }

    /// `setTextureUV(u, v)`.
    pub fn set_texture_uv(&mut self, u: f64, v: f64) {
        self.tess.uv = [u as f32, v as f32];
    }

    /// `addVertex(x, y, z)`.
    pub fn add_vertex(&mut self, x: f64, y: f64, z: f64) {
        let p = self
            .matrix
            .transform_point3(Vec3::new(x as f32, y as f32, z as f32));
        let c = self.tess.has_color.then_some(self.tess.color);
        self.tess.verts.push((p.to_array(), self.tess.uv, c));
    }

    /// `addVertexWithUV(x, y, z, u, v)`.
    // src: Tessellator.java:223-226
    pub fn add_vertex_with_uv(&mut self, x: f64, y: f64, z: f64, u: f64, v: f64) {
        self.set_texture_uv(u, v);
        self.add_vertex(x, y, z);
    }

    /// `draw()`: the quads become a batch under the current state (module
    /// docs).
    pub fn draw(&mut self) {
        debug_assert!(self.tess.drawing, "Not tesselating!");
        self.tess.drawing = false;
        let verts = std::mem::take(&mut self.tess.verts);
        let n = verts.len() / 4 * 4;
        if n == 0 {
            self.tess.verts = verts;
            return;
        }
        let gl_color = self.state.color;
        let s = &self.state;
        let key = GuiKey {
            blend: s.blend.then_some(s.blend_func),
            depth_test: s.depth_test,
            depth_write: s.depth_test && s.depth_mask,
            depth_equal: s.depth_test && s.depth_equal,
        };
        let texture = if s.texture_2d {
            // An unbound texture samples as GL's texture 0 would: the
            // missing texture stands in (TexRef resolution).
            Some(s.bound.clone().unwrap_or(TexRef::Name(Cow::Borrowed(""))))
        } else {
            None
        };
        let alpha_ref = s.alpha_test.then_some(s.alpha_ref);
        let layer = self.layer;
        let smooth = s.smooth;
        let same = self.draws.last().is_some_and(|d| {
            d.layer == layer && d.key == key && d.texture == texture && d.alpha_ref == alpha_ref
        });
        if !same {
            self.draws.push(GuiDraw {
                layer,
                key,
                texture,
                alpha_ref,
                batch: TessBatch::default(),
            });
        }
        let batch = &mut self.draws.last_mut().expect("pushed").batch;
        for q in verts[..n].as_chunks::<4>().0 {
            // GL_FLAT: the quad's last vertex colours it.
            let flat = q[3].2.unwrap_or(gl_color);
            for &(p, uv, c) in q {
                let c = if smooth { c.unwrap_or(gl_color) } else { flat };
                batch.vertex(p, uv, c);
            }
        }
        self.tess.verts = verts;
        self.tess.verts.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad(g: &mut GuiGl, x: f64) {
        g.start_drawing_quads();
        g.add_vertex_with_uv(x, 1.0, 0.0, 0.0, 1.0);
        g.add_vertex_with_uv(x + 1.0, 1.0, 0.0, 1.0, 1.0);
        g.add_vertex_with_uv(x + 1.0, 0.0, 0.0, 1.0, 0.0);
        g.add_vertex_with_uv(x, 0.0, 0.0, 0.0, 0.0);
        g.draw();
    }

    #[test]
    fn same_state_merges_and_a_switch_splits() {
        let mut g = GuiGl::default();
        g.bind_texture("/gui/gui.png");
        quad(&mut g, 0.0);
        g.color4f(1.0, 0.0, 0.0, 1.0);
        quad(&mut g, 2.0);
        assert_eq!(g.draws().len(), 1);
        assert_eq!(g.draws()[0].batch.len(), 8);
        assert_eq!(g.draws()[0].batch.colors[4], [1.0, 0.0, 0.0, 1.0]);
        g.bind_texture("/gui/background.png");
        quad(&mut g, 4.0);
        g.disable(gl::TEXTURE_2D);
        quad(&mut g, 6.0);
        g.enable(gl::BLEND);
        quad(&mut g, 8.0);
        g.set_layer(GuiLayer::Screen);
        quad(&mut g, 10.0);
        let d = g.draws();
        assert_eq!(d.len(), 5);
        assert_eq!(d[2].texture, None);
        assert_eq!(
            d[3].key.blend,
            Some((gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA))
        );
        assert_eq!(d[4].layer, GuiLayer::Screen);
    }

    #[test]
    fn flat_shading_takes_the_last_vertex_and_smooth_interpolates() {
        let mut g = GuiGl::default();
        for smooth in [false, true] {
            g.shade_model(if smooth { gl::SMOOTH } else { gl::FLAT });
            g.start_drawing_quads();
            g.set_color_rgba_i(0, 0);
            g.add_vertex(0.0, 0.0, 0.0);
            g.add_vertex(1.0, 0.0, 0.0);
            g.set_color_rgba_i(0xff0000, 255);
            g.add_vertex(1.0, 1.0, 0.0);
            g.add_vertex(0.0, 1.0, 0.0);
            g.draw();
        }
        let b = &g.draws()[0].batch;
        assert_eq!(b.colors[0], [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(b.colors[4], [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(b.colors[7], [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn matrix_and_tessellator_colours() {
        let mut g = GuiGl::default();
        g.push_matrix();
        g.translatef(10.0, 20.0, 0.0);
        g.color3f(0.5, 0.5, 0.5);
        quad(&mut g, 0.0);
        g.pop_matrix();
        quad(&mut g, 0.0);
        let b = &g.draws()[0].batch;
        assert_eq!(b.positions[0], [10.0, 21.0, 0.0]);
        assert_eq!(b.positions[4], [0.0, 1.0, 0.0]);
        assert_eq!(b.colors[0], [0.5, 0.5, 0.5, 1.0]);
        // setColorRGBA clamps to bytes; setColorRGBA_F truncates.
        g.start_drawing_quads();
        g.set_color_rgba_f(1.5, 0.5, -1.0, 0.999);
        for _ in 0..4 {
            g.add_vertex(0.0, 0.0, 0.0);
        }
        g.draw();
        let c = *g.draws().last().unwrap().batch.colors.last().unwrap();
        assert_eq!(c, [1.0, 127.0 / 255.0, 0.0, 254.0 / 255.0]);
    }
}
