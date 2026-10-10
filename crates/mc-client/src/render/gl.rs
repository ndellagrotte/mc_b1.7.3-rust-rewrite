//! [`EntityGl`]: the fixed-function GL the entity renderers draw with, as
//! data (project-owned; PLAN_v4 D7). The `Render*` classes are
//! immediate-mode code: `glPushMatrix`, `glTranslatef`, `glRotatef`,
//! `glScalef`, `glColor`, `glEnable`/`glDisable`, `glBlendFunc`,
//! `glDepthFunc`, `glNormal3f`, the `GL_TEXTURE` matrix, `bindTexture`,
//! `Tessellator` draws and `ModelBase.render` calls, with GL state that
//! carries from one call (and one entity) to the next. `EntityGl` keeps that
//! state and the matrix stack so the renderers transcribe line for line,
//! and turns each draw into data:
//!
//! - a **Tessellator draw** ([`TessDraw`]) has its vertices transformed by
//!   the current modelview, coloured as GL colours them (the Tessellator's
//!   colour when the draw set one, else `glColor`), lit on the CPU with GL's
//!   fixed-function formula when `GL_LIGHTING` is on ([`EntityGl::light`]:
//!   `RenderHelper.enableStandardItemLighting`'s two lights, the normal
//!   transformed by the inverse transpose and treated as `GL_RESCALE_NORMAL`
//!   or `GL_NORMALIZE` say), flat-shaded through GL's provoking vertex, and
//!   carries the per-fragment state ([`TessSpec`]); consecutive draws under
//!   the same state are merged;
//! - a **model draw** ([`ModelDraw`]) keeps the model's parts and the
//!   modelview at the call: [`super::apply`] draws it with WP-3.5a's shared
//!   part meshes and [`crate::model::ModelMaterial`] (which lights in its
//!   shader with the same formula).
//!
//! # Space
//!
//! The modelview starts as the identity in "GL entity space": world space
//! minus `RenderManager.renderPos` (the view entity's interpolated
//! position), axes as the world's. Vanilla's real modelview also holds the
//! camera; the lights are set under the camera's rotation
//! (EntityRenderer.java:500, RenderHelper.java:26-31), so they are fixed in
//! world space and lighting can be computed here without the camera.
//!
//! # GL semantics kept
//!
//! - `glColor3f` sets alpha to 1; a Tessellator vertex takes the
//!   Tessellator's colour field when the draw called a colour setter
//!   (`hasColor`), and that field persists between draws as in Java
//!   (Tessellator.java:20, :155); without `hasNormals` a vertex takes the
//!   current `glNormal3f`.
//! - `GL_QUADS` are drawn as the triangles `(0, 1, 2)`, `(0, 2, 3)`
//!   (`convertQuadsToTriangles`, Tessellator.java:12, :112-113); under
//!   `GL_FLAT` (`enableStandardItemLighting` sets it, RenderHelper.java:35)
//!   each triangle takes its last vertex's colour.
//! - `GL_RESCALE_NORMAL` rescales the transformed normal by the inverse's
//!   third-row length, so a normal that was not unit length stays short
//!   (`RenderArrow`'s `glNormal3f(0.05625F, …)` under `glScalef(0.05625F)`
//!   lights at 5.6 %, RenderArrow.java:24-34); `GL_NORMALIZE` makes it unit.
//!   `RenderSlime` enables `GL_NORMALIZE` and nothing disables it again
//!   (RenderSlime.java:16): **vanilla bug, transcribed** — the flag is
//!   latched for the rest of the session ([`super::RenderManager`]), which
//!   brightens arrows once a slime has been drawn.
//! - Model draws are lit in [`crate::model::ModelMaterial`]'s shader, which
//!   always normalises (WP-3.5a's approximation; model normals are unit).

use bevy::math::{Mat3, Mat4, Vec3, Vec4};
use bevy::prelude::Transform;

use super::material::EntityGlKey;
use crate::gl_material::GlBlend;
use crate::gui::batch::{TexRef, gl};
use crate::model::material::standard_light_directions;
use crate::model::{ModelKind, ModelPart, PartDraw, RenderMode};
use crate::terrain::tessellator::{Tessellator, gl_byte_normal, normal_bytes, pack_normal};

/// GL enums the entity renderers use beyond [`gl`].
pub mod glx {
    /// `GL_NORMALIZE`.
    pub const NORMALIZE: u32 = 2977;
    /// `GL_LIGHT0` (enabled with `GL_LIGHTING`; tracked as one).
    pub const LIGHT0: u32 = 16384;
    /// `GL_COLOR_MATERIAL`.
    pub const COLOR_MATERIAL: u32 = 2903;
    /// `GL_TEXTURE` matrix mode.
    pub const TEXTURE: u32 = 5890;
    /// `GL_MODELVIEW` matrix mode.
    pub const MODELVIEW: u32 = 5888;
    /// `glDepthFunc(GL_EQUAL)`.
    pub const EQUAL: u32 = 514;
    /// `glDepthFunc(GL_LEQUAL)`.
    pub const LEQUAL: u32 = 515;
    /// `GL_LINE_STRIP` (`Tessellator.startDrawing(3)`).
    pub const LINE_STRIP: u32 = 3;
    /// `GL_TRIANGLE_STRIP` (`startDrawing(5)`).
    pub const TRIANGLE_STRIP: u32 = 5;
    /// `GL_QUADS` (`startDrawingQuads`).
    pub const QUADS: u32 = 7;
}

/// Who a draw belongs to (the per-entity model instances, [`super::apply`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum VisualKey {
    /// A loaded entity, by id.
    Entity(i32),
    /// A weather effect (lightning bolt), by id.
    Weather(i32),
    /// A tile entity's special renderer, by block position.
    TileEntity([i32; 3]),
    /// An `EntityPickupFX`'s entity, by index in the frame's pickups.
    Pickup(usize),
}

/// The fixed-function state (module docs).
#[derive(Clone, Debug, PartialEq)]
pub struct GlState {
    /// `GL_TEXTURE_2D`.
    pub texture_2d: bool,
    /// `GL_LIGHTING` (with `GL_LIGHT0`/`GL_LIGHT1` and `GL_COLOR_MATERIAL`,
    /// as `RenderHelper` switches them together).
    pub lighting: bool,
    /// `GL_BLEND`.
    pub blend: bool,
    /// `glBlendFunc`.
    pub blend_func: (u32, u32),
    /// `GL_ALPHA_TEST`.
    pub alpha_test: bool,
    /// `glAlphaFunc(GL_GREATER, ref)`.
    pub alpha_ref: f32,
    /// `GL_DEPTH_TEST`.
    pub depth_test: bool,
    /// `glDepthMask`.
    pub depth_mask: bool,
    /// `glDepthFunc(GL_EQUAL)` (false: `GL_LEQUAL`).
    pub depth_equal: bool,
    /// `GL_CULL_FACE`.
    pub cull_face: bool,
    /// `GL_RESCALE_NORMAL`.
    pub rescale_normal: bool,
    /// `GL_NORMALIZE`.
    pub normalize: bool,
    /// `glShadeModel(GL_SMOOTH)` (false: `GL_FLAT`).
    pub smooth: bool,
    /// `GL_FOG`.
    pub fog: bool,
    /// `glColor4f`.
    pub color: [f32; 4],
    /// `glNormal3f`.
    pub normal: [f32; 3],
    /// `glBindTexture` (a jar name or a handle).
    pub bound: Option<TexRef>,
}

impl GlState {
    /// The state `renderEntities` starts from: texturing on, the alpha test
    /// at 0.1 (Minecraft.java:266-267), depth test and writes with
    /// `GL_LEQUAL` (Minecraft.java:264-265), `GL_CULL_FACE` on
    /// (EntityRenderer.java:468), blending off, `GL_FOG` on with
    /// `setupFog(0)` (EntityRenderer.java:494-495), `GL_FLAT` and the
    /// standard item lighting (EntityRenderer.java:499-500), the
    /// `GL_TEXTURE_2D` binding of `/terrain.png` (EntityRenderer.java:496),
    /// `GL_NORMALIZE` as latched ([`super::RenderManager`]).
    // src: EntityRenderer.java:466-501
    pub fn render_entities_start(normalize: bool) -> Self {
        GlState {
            texture_2d: true,
            lighting: true,
            blend: false,
            blend_func: (gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA),
            alpha_test: true,
            alpha_ref: 0.1,
            depth_test: true,
            depth_mask: true,
            depth_equal: false,
            cull_face: true,
            rescale_normal: false,
            normalize,
            smooth: false,
            fog: true,
            color: [1.0; 4],
            normal: [0.0, 0.0, 1.0],
            bound: Some(TexRef::from(crate::textures::TERRAIN_PNG)),
        }
    }
}

/// The primitive topology of a [`TessDraw`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Topology {
    /// A triangle list (quads and triangle strips are expanded).
    Triangles,
    /// A line list (line strips are expanded).
    Lines,
}

/// The per-fragment state of a Tessellator draw.
#[derive(Clone, Debug, PartialEq)]
pub struct TessSpec {
    /// The bound texture with `GL_TEXTURE_2D` on.
    pub texture: Option<TexRef>,
    /// `GL_GREATER` reference with `GL_ALPHA_TEST` on.
    pub alpha_ref: Option<f32>,
    /// Blend, depth, culling.
    pub key: EntityGlKey,
    /// `GL_FOG` on (with `setupFog(0)`'s world fog).
    pub fog: bool,
    /// Lines or triangles.
    pub topology: Topology,
}

/// One merged Tessellator batch: vertices in GL entity space, colours in
/// the Java's values (lit when lighting was on).
#[derive(Clone, Debug, PartialEq)]
pub struct TessDraw {
    /// The state.
    pub spec: TessSpec,
    /// Positions.
    pub positions: Vec<[f32; 3]>,
    /// Texture coordinates.
    pub uvs: Vec<[f32; 2]>,
    /// Colours.
    pub colors: Vec<[f32; 4]>,
}

/// The GL state of a model draw ([`crate::model::ModelMaterial`]'s fields).
#[derive(Clone, Debug, PartialEq)]
pub struct ModelSpec {
    /// `glColor4f`.
    pub color: [f32; 4],
    /// The bound texture with `GL_TEXTURE_2D` on.
    pub texture: Option<TexRef>,
    /// `GL_LIGHTING`.
    pub lighting: bool,
    /// `GL_GREATER` reference with `GL_ALPHA_TEST` on.
    pub alpha_ref: Option<f32>,
    /// The blend function as the model material knows it.
    pub blend: GlBlend,
    /// `glBlendFunc(GL_ONE, GL_ONE)`.
    pub src_one: bool,
    /// `glDepthMask`.
    pub depth_write: bool,
    /// `glDepthFunc(GL_EQUAL)`.
    pub depth_equal: bool,
    /// `GL_CULL_FACE`.
    pub cull: bool,
    /// The `GL_TEXTURE` matrix's translation.
    pub uv_offset: [f32; 2],
    /// `GL_FOG`.
    pub fog: bool,
}

/// One `ModelBase.render` call: the model's drawn parts under `root`.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelDraw {
    /// The owner.
    pub key: VisualKey,
    /// Which model (its shared part meshes).
    pub kind: ModelKind,
    /// The modelview at the call, in GL entity space.
    pub root: Mat4,
    /// The drawn parts with their transforms under `root`.
    pub parts: Vec<PartDraw>,
    /// The GL state.
    pub spec: ModelSpec,
}

/// A vertex while drawing.
#[derive(Clone, Copy, Debug, PartialEq)]
struct TessVertex {
    pos: [f32; 3],
    uv: [f32; 2],
    /// The Tessellator's colour field (bytes / 255), or an exact colour.
    color: [f32; 4],
    /// The normal: the packed bytes as GL reads them, or an exact one.
    normal: [f32; 3],
}

/// `Tessellator.instance` (module docs).
#[derive(Clone, Debug, PartialEq)]
struct Tess {
    drawing: bool,
    mode: u32,
    has_color: bool,
    is_color_disabled: bool,
    color: [f32; 4],
    has_normals: bool,
    normal: i32,
    uv: [f64; 2],
    offset: [f64; 3],
    verts: Vec<TessVertex>,
}

impl Default for Tess {
    fn default() -> Self {
        Tess {
            drawing: false,
            mode: glx::QUADS,
            has_color: false,
            is_color_disabled: false,
            color: [0.0; 4],
            has_normals: false,
            normal: 0,
            uv: [0.0; 2],
            offset: [0.0; 3],
            verts: Vec::new(),
        }
    }
}

/// What a frame's renderers drew.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrawList {
    /// Model draws, in call order.
    pub models: Vec<ModelDraw>,
    /// Tessellator batches, in call order.
    pub tess: Vec<TessDraw>,
}

/// The entity renderers' GL (module docs).
#[derive(Clone, Debug)]
pub struct EntityGl {
    /// The fixed-function state.
    pub state: GlState,
    matrix: Mat4,
    stack: Vec<Mat4>,
    texture_matrix: Mat4,
    texture_mode: bool,
    tess: Tess,
    lights: [Vec3; 2],
    /// The owner of the draws being made.
    pub key: VisualKey,
    /// The draws so far.
    pub out: DrawList,
}

impl Default for EntityGl {
    fn default() -> Self {
        EntityGl::new(false)
    }
}

/// `Tessellator.setColorRGBA`'s clamp of a channel to a byte.
// src: Tessellator.java:179-219
fn byte(v: i32) -> u8 {
    v.clamp(0, 255) as u8
}

impl EntityGl {
    /// A new frame at the `renderEntities` state
    /// ([`GlState::render_entities_start`]), identity modelview.
    pub fn new(normalize: bool) -> Self {
        EntityGl {
            state: GlState::render_entities_start(normalize),
            matrix: Mat4::IDENTITY,
            stack: Vec::new(),
            texture_matrix: Mat4::IDENTITY,
            texture_mode: false,
            tess: Tess::default(),
            lights: standard_light_directions(),
            key: VisualKey::Entity(0),
            out: DrawList::default(),
        }
    }

    /// The state `GuiInventory` calls `renderEntityWithPosYaw` in
    /// (GuiInventory.java:28-50): the GUI's texturing, alpha test and
    /// depth test, `GL_RESCALE_NORMAL` and the standard item lighting on,
    /// no fog, culling on (EntityRenderer.java:772), white; the modelview
    /// starts at the identity standing for the screen's matrix, and the
    /// lights are given in that space
    /// ([`crate::gui::entity_draws::lights_in_start_space`]).
    // src: GuiInventory.java:28-50, GuiContainer.java:20-33
    pub fn gui_start(normalize: bool, lights: [Vec3; 2]) -> Self {
        let mut g = EntityGl::new(normalize);
        g.state.fog = false;
        g.state.rescale_normal = true;
        g.state.bound = Some(TexRef::from(
            crate::gui::container::inventory::INVENTORY_PNG,
        ));
        g.lights = lights;
        g
    }

    /// The current modelview (GL entity space).
    pub fn matrix(&self) -> Mat4 {
        self.matrix
    }

    // ---- state ----------------------------------------------------------------------

    fn cap(&mut self, cap: u32) -> Option<&mut bool> {
        let s = &mut self.state;
        Some(match cap {
            gl::TEXTURE_2D => &mut s.texture_2d,
            gl::BLEND => &mut s.blend,
            gl::ALPHA_TEST => &mut s.alpha_test,
            gl::DEPTH_TEST => &mut s.depth_test,
            gl::LIGHTING => &mut s.lighting,
            gl::FOG => &mut s.fog,
            gl::CULL_FACE => &mut s.cull_face,
            gl::RESCALE_NORMAL => &mut s.rescale_normal,
            glx::NORMALIZE => &mut s.normalize,
            _ => return None,
        })
    }

    /// `glEnable(cap)` (others ignored).
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

    /// `RenderHelper.enableStandardItemLighting()` (the lights themselves
    /// are fixed in world space, module docs; it also sets `GL_FLAT`).
    // src: RenderHelper.java:16-37
    pub fn enable_standard_item_lighting(&mut self) {
        self.state.lighting = true;
        self.state.smooth = false;
    }

    /// `RenderHelper.disableStandardItemLighting()`.
    // src: RenderHelper.java:9-14
    pub fn disable_standard_item_lighting(&mut self) {
        self.state.lighting = false;
    }

    /// `glBlendFunc(src, dst)`.
    pub fn blend_func(&mut self, src: u32, dst: u32) {
        self.state.blend_func = (src, dst);
    }

    /// `glDepthMask(flag)`.
    pub fn depth_mask(&mut self, flag: bool) {
        self.state.depth_mask = flag;
    }

    /// `glDepthFunc(func)` (`GL_EQUAL` or `GL_LEQUAL`, the two vanilla
    /// uses).
    pub fn depth_func(&mut self, func: u32) {
        self.state.depth_equal = func == glx::EQUAL;
    }

    /// `glShadeModel(mode)`.
    pub fn shade_model(&mut self, mode: u32) {
        self.state.smooth = mode == gl::SMOOTH;
    }

    /// `glColor4f(r, g, b, a)`.
    pub fn color4f(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.state.color = [r, g, b, a];
    }

    /// `glColor3f(r, g, b)`: alpha 1.
    pub fn color3f(&mut self, r: f32, g: f32, b: f32) {
        self.state.color = [r, g, b, 1.0];
    }

    /// `glNormal3f(x, y, z)`.
    pub fn normal3f(&mut self, x: f32, y: f32, z: f32) {
        self.state.normal = [x, y, z];
    }

    /// `renderEngine.bindTexture(renderEngine.getTexture(name))`
    /// (`Render.loadTexture`, `TileEntitySpecialRenderer.bindTextureByName`).
    pub fn bind_texture(&mut self, t: impl Into<TexRef>) {
        self.state.bound = Some(t.into());
    }

    // ---- matrices -------------------------------------------------------------------

    /// `glMatrixMode(mode)` (`GL_MODELVIEW` or `GL_TEXTURE`).
    pub fn matrix_mode(&mut self, mode: u32) {
        self.texture_mode = mode == glx::TEXTURE;
    }

    fn current(&mut self) -> &mut Mat4 {
        if self.texture_mode {
            &mut self.texture_matrix
        } else {
            &mut self.matrix
        }
    }

    /// `glLoadIdentity()`.
    pub fn load_identity(&mut self) {
        *self.current() = Mat4::IDENTITY;
    }

    /// `glPushMatrix()` (modelview).
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
        let m = self.current();
        *m *= Mat4::from_translation(Vec3::new(x, y, z));
    }

    /// `glScalef(x, y, z)`.
    pub fn scalef(&mut self, x: f32, y: f32, z: f32) {
        let m = self.current();
        *m *= Mat4::from_scale(Vec3::new(x, y, z));
    }

    /// `glRotatef(angle, x, y, z)`: degrees about the normalised axis.
    pub fn rotatef(&mut self, angle: f32, x: f32, y: f32, z: f32) {
        let axis = Vec3::new(x, y, z);
        if axis.length_squared() == 0.0 {
            return;
        }
        let m = self.current();
        *m *= Mat4::from_axis_angle(axis.normalize(), angle.to_radians());
    }

    /// `glMultMatrix(t)`: a part's `postRender` transform.
    pub fn mult_transform(&mut self, t: &Transform) {
        self.matrix *= t.to_matrix();
    }

    // ---- the Tessellator ----------------------------------------------------------

    /// `Tessellator.startDrawingQuads()`.
    // src: Tessellator.java:143-145
    pub fn start_drawing_quads(&mut self) {
        self.start_drawing(glx::QUADS);
    }

    /// `Tessellator.startDrawing(mode)`: no colour, normal or texture yet.
    // src: Tessellator.java:147-160
    pub fn start_drawing(&mut self, mode: u32) {
        let t = &mut self.tess;
        t.drawing = true;
        t.mode = mode;
        t.verts.clear();
        t.has_color = false;
        t.has_normals = false;
        t.is_color_disabled = false;
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

    /// `setColorOpaque(r, g, b)`.
    // src: Tessellator.java:175-177
    pub fn set_color_opaque(&mut self, r: i32, g: i32, b: i32) {
        self.set_color_rgba(r, g, b, 255);
    }

    /// `setColorRGBA(r, g, b, a)`.
    // src: Tessellator.java:179-221
    pub fn set_color_rgba(&mut self, r: i32, g: i32, b: i32, a: i32) {
        let t = &mut self.tess;
        if !t.is_color_disabled {
            t.has_color = true;
            t.color = [byte(r), byte(g), byte(b), byte(a)].map(|c| c as f32 / 255.0);
        }
    }

    /// `setColorOpaque_I(c)`.
    // src: Tessellator.java:275-280
    pub fn set_color_opaque_i(&mut self, c: i32) {
        self.set_color_opaque(c >> 16 & 255, c >> 8 & 255, c & 255);
    }

    /// `setNormal(x, y, z)` (packed as the Tessellator packs it).
    // src: Tessellator.java:293-303
    pub fn set_normal(&mut self, x: f32, y: f32, z: f32) {
        self.tess.has_normals = true;
        self.tess.normal = pack_normal(x, y, z);
    }

    /// `setTranslationD(x, y, z)`.
    // src: Tessellator.java:305-309
    pub fn set_translation_d(&mut self, x: f64, y: f64, z: f64) {
        self.tess.offset = [x, y, z];
    }

    /// `setTextureUV(u, v)`.
    // src: Tessellator.java:161-164
    pub fn set_texture_uv(&mut self, u: f64, v: f64) {
        self.tess.uv = [u, v];
    }

    /// `addVertexWithUV(x, y, z, u, v)`.
    // src: Tessellator.java:223-226
    pub fn add_vertex_with_uv(&mut self, x: f64, y: f64, z: f64, u: f64, v: f64) {
        self.set_texture_uv(u, v);
        self.add_vertex(x, y, z);
    }

    /// `addVertex(x, y, z)`: the position as `(float)(x + xOffset)`.
    // src: Tessellator.java:228-273
    pub fn add_vertex(&mut self, x: f64, y: f64, z: f64) {
        let t = &mut self.tess;
        t.verts.push(TessVertex {
            pos: [
                (x + t.offset[0]) as f32,
                (y + t.offset[1]) as f32,
                (z + t.offset[2]) as f32,
            ],
            uv: [t.uv[0] as f32, t.uv[1] as f32],
            color: t.color,
            normal: gl_byte_normal(normal_bytes(t.normal)),
        });
    }

    /// A vertex with its own exact colour (the GUI batcher's glyph
    /// colours, [`super::text3d`]): as `addVertexWithUV` after a colour
    /// setter, without the byte rounding.
    pub fn add_vertex_with_uv_color(&mut self, p: [f64; 3], uv: [f32; 2], c: [f32; 4]) {
        let t = &mut self.tess;
        t.has_color = true;
        t.verts.push(TessVertex {
            pos: [
                (p[0] + t.offset[0]) as f32,
                (p[1] + t.offset[1]) as f32,
                (p[2] + t.offset[2]) as f32,
            ],
            uv,
            color: c,
            normal: gl_byte_normal(normal_bytes(t.normal)),
        });
    }

    /// `ModelRenderer.render(scale)` drawn through the Tessellator path (a
    /// model this crate has no shared mesh for: `SignModel`): the part's
    /// transform, then its quads with their normals.
    // src: ModelRenderer.java:85-121
    pub fn render_part(&mut self, part: &ModelPart, scale: f32) {
        if !part.is_drawn() {
            return;
        }
        self.push_matrix();
        self.mult_transform(&part.transform(RenderMode::Render, scale));
        let d = part.display_list(scale);
        self.start_drawing_quads();
        self.tess.has_normals = true;
        for i in 0..d.positions.len() {
            let p = d.positions[i];
            self.tess.verts.push(TessVertex {
                pos: p,
                uv: d.uvs[i],
                color: self.tess.color,
                normal: d.normals[i],
            });
        }
        self.draw();
        self.pop_matrix();
    }

    /// `Tessellator.draw()`: the vertices as one draw (module docs).
    // src: Tessellator.java:55-131
    pub fn draw(&mut self) {
        let t = std::mem::take(&mut self.tess.verts);
        let (mode, has_color, has_normals) =
            (self.tess.mode, self.tess.has_color, self.tess.has_normals);
        self.tess.drawing = false;
        let verts: Vec<([f32; 3], [f32; 2], [f32; 4])> = t
            .iter()
            .map(|v| {
                let c = if has_color { v.color } else { self.state.color };
                let n = if has_normals {
                    Vec3::from(v.normal)
                } else {
                    Vec3::from(self.state.normal)
                };
                (v.pos, v.uv, self.vertex_color(c, n))
            })
            .collect();
        self.tess.verts = t;
        self.tess.verts.clear();
        self.emit_vertices(mode, verts);
    }

    /// A terrain [`Tessellator`]'s quads (`RenderBlocks` output: the item
    /// blocks of `renderBlockOnInventory`, the moving pistons, the falling
    /// sand) drawn with the current state, as its `draw()` would.
    pub fn draw_tessellator(&mut self, t: &Tessellator) {
        let verts: Vec<([f32; 3], [f32; 2], [f32; 4])> = (0..t.len())
            .map(|i| {
                let c = if t.has_color {
                    t.colors[i].map(|b| b as f32 / 255.0)
                } else {
                    self.state.color
                };
                let n = if t.has_normals {
                    Vec3::from(gl_byte_normal(t.normals[i]))
                } else {
                    Vec3::from(self.state.normal)
                };
                (t.positions[i], t.uvs[i], self.vertex_color(c, n))
            })
            .collect();
        self.emit_vertices(glx::QUADS, verts);
    }

    /// The colour GL gives a vertex of colour `c` and object-space normal
    /// `n` under the current state: `c` itself, or lit (module docs).
    fn vertex_color(&self, c: [f32; 4], n: Vec3) -> [f32; 4] {
        if !self.state.lighting {
            return c;
        }
        let m3 = Mat3::from_mat4(self.matrix);
        let inv = m3.inverse();
        let mut ne = inv.transpose() * n;
        if self.state.normalize {
            ne = ne.normalize_or_zero();
        } else if self.state.rescale_normal {
            let r = inv.row(2).length();
            if r > 0.0 {
                ne /= r;
            }
        }
        self.light(c, ne)
    }

    /// GL's lit colour with `RenderHelper.enableStandardItemLighting`'s
    /// setup: emission 0, scene ambient 0.4, two directional lights of
    /// diffuse 0.6 and no ambient or specular, `GL_COLOR_MATERIAL` on
    /// ambient and diffuse: `clamp(c·0.4 + Σ c·0.6·max(n·Lᵢ, 0))`, alpha the
    /// material's. `n` is the eye-space (here world-space) normal.
    // src: RenderHelper.java:16-37
    pub fn light(&self, c: [f32; 4], n: Vec3) -> [f32; 4] {
        let d0 = n.dot(self.lights[0]).max(0.0);
        let d1 = n.dot(self.lights[1]).max(0.0);
        let ch = |v: f32| (v * 0.4 + v * 0.6 * d0 + v * 0.6 * d1).clamp(0.0, 1.0);
        [ch(c[0]), ch(c[1]), ch(c[2]), c[3].clamp(0.0, 1.0)]
    }

    /// The current per-fragment state for `topology`.
    fn tess_spec(&self, topology: Topology) -> TessSpec {
        let s = &self.state;
        TessSpec {
            texture: if s.texture_2d { s.bound.clone() } else { None },
            alpha_ref: s.alpha_test.then_some(s.alpha_ref),
            key: EntityGlKey {
                blend: s.blend.then_some(s.blend_func),
                depth_test: s.depth_test,
                depth_write: s.depth_mask,
                depth_equal: s.depth_equal,
                cull: s.cull_face,
            },
            fog: s.fog,
            topology,
        }
    }

    /// Transforms and assembles `verts` drawn in `mode` and appends them
    /// (merged with the last batch when the state is the same).
    fn emit_vertices(&mut self, mode: u32, verts: Vec<([f32; 3], [f32; 2], [f32; 4])>) {
        let (topology, order): (Topology, Vec<(usize, usize)>) = match mode {
            glx::LINE_STRIP => {
                let mut o = Vec::new();
                for i in 0..verts.len().saturating_sub(1) {
                    // the segment's provoking vertex: its second
                    o.push((i, i + 1));
                    o.push((i + 1, i + 1));
                }
                (Topology::Lines, o)
            }
            glx::TRIANGLE_STRIP => {
                let mut o = Vec::new();
                for i in 0..verts.len().saturating_sub(2) {
                    let tri = if i % 2 == 0 {
                        [i, i + 1, i + 2]
                    } else {
                        [i + 1, i, i + 2]
                    };
                    for v in tri {
                        o.push((v, i + 2));
                    }
                }
                (Topology::Triangles, o)
            }
            _ => {
                let mut o = Vec::new();
                for q in 0..verts.len() / 4 {
                    let b = q * 4;
                    for v in [b, b + 1, b + 2] {
                        o.push((v, b + 2));
                    }
                    for v in [b, b + 2, b + 3] {
                        o.push((v, b + 3));
                    }
                }
                (Topology::Triangles, o)
            }
        };
        if order.is_empty() {
            return;
        }
        let spec = self.tess_spec(topology);
        let flat = !self.state.smooth;
        let m = self.matrix;
        let tm = self.texture_matrix;
        let tex_identity = tm == Mat4::IDENTITY;
        let mut positions = Vec::with_capacity(order.len());
        let mut uvs = Vec::with_capacity(order.len());
        let mut colors = Vec::with_capacity(order.len());
        for (v, provoking) in order {
            let (p, uv, c) = verts[v];
            positions.push(m.transform_point3(Vec3::from(p)).to_array());
            uvs.push(if tex_identity {
                uv
            } else {
                let t = tm * Vec4::new(uv[0], uv[1], 0.0, 1.0);
                [t.x, t.y]
            });
            colors.push(if flat { verts[provoking].2 } else { c });
        }
        if let Some(last) = self.out.tess.last_mut()
            && last.spec == spec
        {
            last.positions.extend(positions);
            last.uvs.extend(uvs);
            last.colors.extend(colors);
            return;
        }
        self.out.tess.push(TessDraw {
            spec,
            positions,
            uvs,
            colors,
        });
    }

    // ---- models ---------------------------------------------------------------------

    /// The current state as a model draw's.
    pub fn model_spec(&self) -> ModelSpec {
        let s = &self.state;
        let (blend, src_one) = if !s.blend {
            (GlBlend::Off, false)
        } else {
            match s.blend_func {
                (gl::SRC_ALPHA, gl::ONE) => (GlBlend::Additive, false),
                (gl::ONE, gl::ONE) => (GlBlend::Additive, true),
                _ => (GlBlend::Alpha, false),
            }
        };
        let t = self.texture_matrix.w_axis;
        ModelSpec {
            color: s.color,
            texture: if s.texture_2d { s.bound.clone() } else { None },
            lighting: s.lighting,
            alpha_ref: s.alpha_test.then_some(s.alpha_ref),
            blend,
            src_one,
            depth_write: s.depth_mask,
            depth_equal: s.depth_equal,
            cull: s.cull_face,
            uv_offset: [t.x, t.y],
            fog: s.fog,
        }
    }

    /// `model.render(…)`'s draws (from [`crate::model::Model::render`])
    /// under the current modelview and state.
    pub fn model(&mut self, kind: ModelKind, parts: Vec<PartDraw>) {
        if parts.is_empty() {
            return;
        }
        let spec = self.model_spec();
        self.out.models.push(ModelDraw {
            key: self.key,
            kind,
            root: self.matrix,
            parts,
            spec,
        });
    }
}
