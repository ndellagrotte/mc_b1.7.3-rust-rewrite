//! `RenderItem`'s GUI half (RenderItem.java: `renderItemIntoGUI`,
//! `drawItemIntoGui`, `renderItemOverlayIntoGUI`, `renderQuad`,
//! `renderTexturedQuad`) and the GL lighting the GUI's 3D items are drawn
//! under (`RenderHelper.enableStandardItemLighting` with
//! `GL_RESCALE_NORMAL`), on WP-3.6a's [`GuiGl`].
//!
//! # 3D items in a 2D batcher
//!
//! [`GuiGl`] keeps no normals and no lights, and its material does no
//! lighting and no face culling. Block items (`RenderBlocks.renderItemIn3d`)
//! are lit by GL's fixed-function per-vertex lighting and drawn with
//! `GL_CULL_FACE` on (`renderWorld` leaves it on, EntityRenderer.java:772;
//! `drawItemIntoGui` turns it on again, RenderItem.java:162), so
//! [`ItemGl`] does both on the CPU, as GL does them, before the vertices
//! reach the batcher:
//!
//! - **Matrix:** [`ItemGl`] mirrors the modelview matrix [`GuiGl`] keeps:
//!   every `glPushMatrix`/`glTranslatef`/`glScalef`/`glRotatef` between the
//!   frame's start and an item goes through [`ItemGl`], which applies it to
//!   both. (The GUI starts every frame and every screen at the identity;
//!   the translation `(0, 0, -2000)` of `setupOverlayRendering` moves z
//!   only and changes no direction.)
//! - **Lights:** `enableStandardItemLighting` sets `GL_LIGHT0`/`GL_LIGHT1`
//!   to directions (w = 0) that GL transforms by the modelview matrix of
//!   that moment ([`ItemGl::enable_standard_item_lighting`]); diffuse 0.6,
//!   no ambient or specular of their own, the light model's ambient 0.4,
//!   `GL_COLOR_MATERIAL` with `GL_AMBIENT_AND_DIFFUSE`, flat shading.
//!   A vertex of colour `c` (the `glColor`, or the Tessellator's colour)
//!   with eye-space normal `n` is `clamp(0.4 c + Σᵢ 0.6 c max(n·Lᵢ, 0))`,
//!   alpha `c.a` ([`lit_color`]).
//! - **Normals:** the Tessellator's bytes as GL reads them
//!   ([`gl_byte_normal`]; the +x face carries the −x normal, a vanilla bug
//!   kept in [`crate::terrain::tessellator`]), transformed as GL transforms
//!   normals: `n · M⁻¹` with `M` the modelview's upper 3×3, times
//!   `GL_RESCALE_NORMAL`'s factor `1 / |third row of M⁻¹|` when that is on
//!   ([`eye_normal`]). No `GL_NORMALIZE`.
//! - **Culling:** with `GL_CULL_FACE` on (`glCullFace(GL_BACK)`,
//!   Minecraft.java:268; counter-clockwise front faces), a quad whose
//!   window-space winding is clockwise is dropped ([`is_front_facing`]).
//!   The GUI's y axis points down, so a front face is clockwise in GUI
//!   coordinates. With the depth test on (the hotbar, the containers) this
//!   changes nothing visible on a closed shape; the achievement popup draws
//!   its icon with the depth test off, where it decides which faces show.
//!
//! Colours are rounded to bytes when they reach the batcher (GL's
//! float-to-fixed conversion), which is within half a step of GL's
//! interpolated colour.
//!
//! # Clipping (the achievement map)
//!
//! `GuiAchievements` confines its icons to the map window with the depth
//! buffer (the tiles are drawn with `glDepthFunc(GL_GEQUAL)` at z −200 and
//! the icons pass only over them, GuiAchievements.java:152-200). The GUI GL
//! has no depth function, so the menus cut their quads to the window on the
//! CPU instead ([`Clip`]): with [`ItemGl::clip`] set, every quad of a 3D
//! item is transformed to GUI pixels, clipped to the rectangle
//! (Sutherland–Hodgman; positions, texture coordinates and colours
//! interpolated linearly, exact for the GUI's orthographic projection) and
//! emitted as a fan of quads under an identity matrix; a flat icon is cut
//! as the map's other quads are ([`draw_textured_rect_clipped`]).

use std::sync::OnceLock;

use bevy::math::{Mat3, Mat4, Vec3};
use mc_sim::block::BlockStatics;
use mc_sim::item::{ItemStack, item};

use super::batch::{GuiGl, gl};
use super::font::FontRenderer;
use super::menu::{Clip, draw_textured_rect_clipped};
use crate::model::material::standard_light_directions;
use crate::terrain::blocks;
use crate::terrain::render_blocks::{render_block_as_item, render_item_in_3d};
use crate::terrain::tessellator::{Tessellator, gl_byte_normal};
use crate::textures::Colorizers;

/// `/terrain.png`.
pub const TERRAIN_PNG: &str = "/terrain.png";
/// `/gui/items.png`.
pub const ITEMS_PNG: &str = "/gui/items.png";

/// `GL_LIGHT_MODEL_AMBIENT`'s 0.4 (`var0`).
// src: RenderHelper.java:22, :36
pub const LIGHT_MODEL_AMBIENT: f32 = 0.4;
/// Each light's `GL_DIFFUSE` 0.6 (`var1`).
// src: RenderHelper.java:23, :27, :32
pub const LIGHT_DIFFUSE: f32 = 0.6;

/// The modelview matrix and lighting state of the GUI's 3D draws (module
/// docs).
#[derive(Clone, Debug, PartialEq)]
pub struct ItemGl {
    /// The modelview matrix (mirrors [`GuiGl`]'s).
    pub matrix: Mat4,
    stack: Vec<Mat4>,
    /// The two lights' eye-space directions, as the last
    /// [`ItemGl::enable_standard_item_lighting`] set them.
    pub lights: [Vec3; 2],
    /// `GL_RESCALE_NORMAL`.
    pub rescale_normal: bool,
    /// `GL_CULL_FACE`.
    pub cull_face: bool,
    /// The window the quads are cut to, in GUI pixels (module docs,
    /// "Clipping"); `None` draws them whole.
    pub clip: Option<Clip>,
}

impl Default for ItemGl {
    fn default() -> Self {
        ItemGl::new()
    }
}

impl ItemGl {
    /// The state at the start of the GUI's frame: the identity, lights not
    /// set yet (pointing along +z), no normal rescaling, culling on
    /// (EntityRenderer.java:772).
    pub fn new() -> Self {
        ItemGl {
            matrix: Mat4::IDENTITY,
            stack: Vec::new(),
            lights: [Vec3::Z; 2],
            rescale_normal: false,
            cull_face: true,
            clip: None,
        }
    }

    /// `glPushMatrix()`.
    pub fn push_matrix(&mut self, g: &mut GuiGl) {
        self.stack.push(self.matrix);
        g.push_matrix();
    }

    /// `glPopMatrix()`.
    pub fn pop_matrix(&mut self, g: &mut GuiGl) {
        if let Some(m) = self.stack.pop() {
            self.matrix = m;
        }
        g.pop_matrix();
    }

    /// `glTranslatef(x, y, z)`.
    pub fn translatef(&mut self, g: &mut GuiGl, x: f32, y: f32, z: f32) {
        self.matrix *= Mat4::from_translation(Vec3::new(x, y, z));
        g.translatef(x, y, z);
    }

    /// `glScalef(x, y, z)`.
    pub fn scalef(&mut self, g: &mut GuiGl, x: f32, y: f32, z: f32) {
        self.matrix *= Mat4::from_scale(Vec3::new(x, y, z));
        g.scalef(x, y, z);
    }

    /// `glRotatef(angle, x, y, z)` (degrees), as [`GuiGl::rotatef`]
    /// computes it.
    pub fn rotatef(&mut self, g: &mut GuiGl, angle: f32, x: f32, y: f32, z: f32) {
        let axis = Vec3::new(x, y, z).normalize_or_zero();
        if axis != Vec3::ZERO {
            self.matrix *= Mat4::from_axis_angle(axis, angle.to_radians());
        }
        g.rotatef(angle, x, y, z);
    }

    /// `RenderHelper.enableStandardItemLighting()`: lighting on, the two
    /// lights at `(±0.2, 1, ∓0.7)` normalised (`Vec3D.normalize`), as
    /// directions transformed by the current modelview matrix, flat
    /// shading.
    // src: RenderHelper.java:16-37
    pub fn enable_standard_item_lighting(&mut self, g: &mut GuiGl) {
        g.enable(gl::LIGHTING);
        let m = Mat3::from_mat4(self.matrix);
        self.lights = standard_light_directions().map(|l| (m * l).normalize_or_zero());
        g.shade_model(gl::FLAT);
    }

    /// `RenderHelper.disableStandardItemLighting()`.
    // src: RenderHelper.java:9-14
    pub fn disable_standard_item_lighting(&mut self, g: &mut GuiGl) {
        g.disable(gl::LIGHTING);
    }

    /// The eye-space normal of the model-space normal `n` under the current
    /// matrix ([`eye_normal`]).
    pub fn eye_normal(&self, n: Vec3) -> Vec3 {
        eye_normal(self.matrix, n, self.rescale_normal)
    }

    /// Draws quads (four vertices each) through the batcher as GL draws a
    /// `GL_QUADS` Tessellator draw: culled when `GL_CULL_FACE` is on, lit
    /// when `GL_LIGHTING` is on (module docs), else coloured with the
    /// Tessellator's colour or `glColor`. `colors` are the Tessellator's
    /// per-vertex colours (`None`: `hasColor` false, `glColor` used);
    /// `normals` the normals GL reads.
    pub fn draw_quads(
        &self,
        g: &mut GuiGl,
        positions: &[[f32; 3]],
        uvs: &[[f32; 2]],
        colors: Option<&[[u8; 4]]>,
        normals: &[[f32; 3]],
    ) {
        let n = positions.len() / 4 * 4;
        if n == 0 {
            return;
        }
        let lighting = g.state.lighting;
        let gl_color = g.state.color;
        if self.clip.is_some() {
            // Positions in GUI pixels: drawn under an identity matrix.
            g.push_matrix();
            g.load_identity();
        }
        g.start_drawing_quads();
        for q in (0..n).step_by(4) {
            if self.cull_face {
                let w = [0, 1, 2, 3].map(|i| {
                    let p = self.matrix.transform_point3(Vec3::from(positions[q + i]));
                    [p.x, p.y]
                });
                if !is_front_facing(&w) {
                    continue;
                }
            }
            let quad: [ClipVertex; 4] = [0, 1, 2, 3].map(|k| {
                let i = q + k;
                let base = match colors {
                    Some(c) => c[i].map(|b| b as f32 / 255.0),
                    None => gl_color,
                };
                let c = if lighting {
                    let n = self.eye_normal(Vec3::from(normals[i]));
                    lit_color(base, n, &self.lights)
                } else {
                    base
                };
                ClipVertex {
                    pos: Vec3::from(positions[i]),
                    uv: uvs[i],
                    color: c,
                }
            });
            match self.clip {
                None => {
                    for v in &quad {
                        emit_vertex(g, v);
                    }
                }
                Some(clip) => {
                    let screen = quad.map(|v| ClipVertex {
                        pos: self.matrix.transform_point3(v.pos),
                        ..v
                    });
                    for fan in fan_quads(&clip_polygon(&screen, clip)) {
                        for v in &fan {
                            emit_vertex(g, v);
                        }
                    }
                }
            }
        }
        g.draw();
        if self.clip.is_some() {
            g.pop_matrix();
        }
    }

    /// A [`Tessellator`]'s quads through [`ItemGl::draw_quads`].
    pub fn draw_tessellator(&self, g: &mut GuiGl, t: &Tessellator) {
        let normals: Vec<[f32; 3]> = t
            .normals
            .iter()
            .map(|&b| {
                if t.has_normals {
                    gl_byte_normal(b)
                } else {
                    // glNormal's initial value.
                    [0.0, 0.0, 1.0]
                }
            })
            .collect();
        let colors = (t.has_color && !t.is_color_disabled).then_some(t.colors.as_slice());
        self.draw_quads(g, &t.positions, &t.uvs, colors, &normals);
    }
}

/// A vertex on its way to the batcher: position (model space, or GUI
/// pixels once clipped), texture coordinates and the colour GL gave it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClipVertex {
    /// The position.
    pub pos: Vec3,
    /// The texture coordinates.
    pub uv: [f32; 2],
    /// The colour (0..1, before the byte rounding).
    pub color: [f32; 4],
}

impl ClipVertex {
    /// The point `t` of the way from `self` to `o` (every attribute).
    fn lerp(&self, o: &ClipVertex, t: f32) -> ClipVertex {
        let l = |a: f32, b: f32| a + (b - a) * t;
        ClipVertex {
            pos: self.pos.lerp(o.pos, t),
            uv: [l(self.uv[0], o.uv[0]), l(self.uv[1], o.uv[1])],
            color: [0, 1, 2, 3].map(|i| l(self.color[i], o.color[i])),
        }
    }
}

/// One vertex into the batcher, its colour rounded to bytes.
fn emit_vertex(g: &mut GuiGl, v: &ClipVertex) {
    let b = v.color.map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as i32);
    g.set_color_rgba(b[0], b[1], b[2], b[3]);
    g.add_vertex_with_uv(
        v.pos.x as f64,
        v.pos.y as f64,
        v.pos.z as f64,
        v.uv[0] as f64,
        v.uv[1] as f64,
    );
}

/// A convex polygon (GUI pixels) cut to the clip rectangle `x0 ≤ x ≤ x1`,
/// `y0 ≤ y ≤ y1` (Sutherland–Hodgman, one edge at a time; the winding is
/// kept). Empty when nothing is inside.
pub fn clip_polygon(poly: &[ClipVertex], c: Clip) -> Vec<ClipVertex> {
    // (axis, bound, keep the side >= bound)
    let edges = [
        (0, c.x0 as f32, true),
        (0, c.x1 as f32, false),
        (1, c.y0 as f32, true),
        (1, c.y1 as f32, false),
    ];
    let mut out: Vec<ClipVertex> = poly.to_vec();
    for (axis, bound, keep_ge) in edges {
        if out.is_empty() {
            break;
        }
        let input = std::mem::take(&mut out);
        let d = |v: &ClipVertex| {
            let s = v.pos[axis] - bound;
            if keep_ge { s } else { -s }
        };
        for i in 0..input.len() {
            let a = &input[i];
            let b = &input[(i + 1) % input.len()];
            let (da, db) = (d(a), d(b));
            if da >= 0.0 {
                out.push(*a);
            }
            if (da >= 0.0) != (db >= 0.0) {
                out.push(a.lerp(b, da / (da - db)));
            }
        }
    }
    if out.len() < 3 { Vec::new() } else { out }
}

/// A convex polygon as `GL_QUADS`: a fan from its first vertex, two
/// triangles per quad, the last quad of an odd count repeating its third
/// vertex (a degenerate second triangle).
pub fn fan_quads(poly: &[ClipVertex]) -> Vec<[ClipVertex; 4]> {
    let mut out = Vec::new();
    let mut k = 1;
    while k + 1 < poly.len() {
        let d = if k + 2 < poly.len() {
            poly[k + 2]
        } else {
            poly[k + 1]
        };
        out.push([poly[0], poly[k], poly[k + 1], d]);
        k += 2;
    }
    out
}

/// GL's normal transformation: the row vector `n · M⁻¹` (`M` the upper 3×3
/// of `m`), times `1 / sqrt(m31² + m32² + m33²)` of `M⁻¹` with
/// `GL_RESCALE_NORMAL` (OpenGL 1.2, §2.10.3).
pub fn eye_normal(m: Mat4, n: Vec3, rescale: bool) -> Vec3 {
    let inv = Mat3::from_mat4(m).inverse();
    // Row vector times inv = inv^T times column vector.
    let out = inv.transpose() * n;
    if rescale {
        let row3 = Vec3::new(inv.x_axis.z, inv.y_axis.z, inv.z_axis.z);
        let len = row3.length();
        if len > 0.0 {
            return out / len;
        }
    }
    out
}

/// GL's lighting of a vertex of colour `c` and eye normal `n` under the two
/// standard lights (module docs): `0.4 c + Σ 0.6 c max(n·L, 0)` per
/// channel, clamped to 1; alpha `c.a`.
// src: RenderHelper.java:16-37
pub fn lit_color(c: [f32; 4], n: Vec3, lights: &[Vec3; 2]) -> [f32; 4] {
    let mut f = LIGHT_MODEL_AMBIENT;
    for l in lights {
        f += LIGHT_DIFFUSE * n.dot(*l).max(0.0);
    }
    [
        (c[0] * f).clamp(0.0, 1.0),
        (c[1] * f).clamp(0.0, 1.0),
        (c[2] * f).clamp(0.0, 1.0),
        c[3],
    ]
}

/// Whether a quad given in GUI coordinates (y down) faces the viewer:
/// counter-clockwise in window coordinates (y up), GL's default front
/// face, is clockwise here, a negative shoelace sum.
pub fn is_front_facing(w: &[[f32; 2]; 4]) -> bool {
    let mut s = 0.0f32;
    for i in 0..4 {
        let a = w[i];
        let b = w[(i + 1) % 4];
        s += a[0] * b[1] - b[0] * a[1];
    }
    s < 0.0
}

/// Colour maps for `render_block_as_item` (the inventory faces read none;
/// one empty set, made once).
pub(crate) fn empty_colorizers() -> &'static Colorizers {
    static C: OnceLock<Colorizers> = OnceLock::new();
    C.get_or_init(Colorizers::default)
}

/// `RenderItem.field_27004_a`: the GUI's item renderer colours items with
/// `getColorFromDamage` (true for every `RenderItem` the GUI makes).
// src: RenderItem.java:9
pub const FIELD_27004_A: bool = true;

/// `renderItemIntoGUI(font, renderEngine, stack, x, y)`: nothing for a
/// null stack.
// src: RenderItem.java:165-169
pub fn render_item_into_gui(
    ig: &mut ItemGl,
    g: &mut GuiGl,
    statics: &BlockStatics,
    var3: Option<&ItemStack>,
    var4: i32,
    var5: i32,
) {
    if let Some(s) = var3 {
        draw_item_into_gui(
            ig,
            g,
            statics,
            s.item_id,
            s.get_item_damage(),
            s.get_icon_index(),
            var4,
            var5,
        );
    }
}

/// The matrix `drawItemIntoGui` builds for a block item at `(x, y)`, after
/// the caller's (the `glTranslatef`/`glScalef`/`glRotatef` sequence, the
/// last `glRotatef(-90, 0, 1, 0)` included).
// src: RenderItem.java:123-137
pub fn block_item_matrix(var6: i32, var7: i32) -> Mat4 {
    let mut m = Mat4::IDENTITY;
    m *= Mat4::from_translation(Vec3::new((var6 - 2) as f32, (var7 + 3) as f32, -3.0));
    m *= Mat4::from_scale(Vec3::new(10.0, 10.0, 10.0));
    m *= Mat4::from_translation(Vec3::new(1.0, 0.5, 1.0));
    m *= Mat4::from_scale(Vec3::new(1.0, 1.0, -1.0));
    m *= Mat4::from_axis_angle(Vec3::X, 210.0f32.to_radians());
    m *= Mat4::from_axis_angle(Vec3::Y, 45.0f32.to_radians());
    m *= Mat4::from_axis_angle(Vec3::Y, (-90.0f32).to_radians());
    m
}

/// `drawItemIntoGui(font, renderEngine, id, damage, icon, x, y)` of a GUI
/// `RenderItem` (`field_27004_a` true, no clip): see
/// [`draw_item_into_gui_with`].
// src: RenderItem.java:117-163
#[allow(clippy::too_many_arguments)]
pub fn draw_item_into_gui(
    ig: &mut ItemGl,
    g: &mut GuiGl,
    statics: &BlockStatics,
    var3: i32,
    var4: i32,
    var5: i32,
    var6: i32,
    var7: i32,
) {
    draw_item_into_gui_with(
        ig,
        g,
        statics,
        FIELD_27004_A,
        None,
        var3,
        var4,
        var5,
        var6,
        var7,
    );
}

/// `drawItemIntoGui(font, renderEngine, id, damage, icon, x, y)`: a block
/// whose render type is drawn in 3D (`RenderBlocks.renderItemIn3d`) as a
/// small lit cube from `/terrain.png`; any other item with an icon as a
/// 16×16 tile of `/terrain.png` (ids below 256) or `/gui/items.png`, lighting
/// off. Leaves `GL_CULL_FACE` on.
///
/// `colorize` is the renderer's `field_27004_a`: when set, `glColor` is
/// `getColorFromDamage(damage)` and the block's `renderBlockOnInventory`
/// sets its own `getRenderColor(meta)` (`RenderBlocks.field_31088_b`);
/// when clear (`GuiAchievements` for an achievement that cannot be taken
/// yet) both keep the caller's `glColor`, the 0.1 grey
/// (GuiAchievements.java:268-280). `clip` cuts the drawing to a window in
/// GUI pixels (module docs, "Clipping"; `None` draws it whole).
// src: RenderItem.java:117-163
#[allow(clippy::too_many_arguments)]
pub fn draw_item_into_gui_with(
    ig: &mut ItemGl,
    g: &mut GuiGl,
    statics: &BlockStatics,
    colorize: bool,
    clip: Option<Clip>,
    var3: i32,
    var4: i32,
    var5: i32,
    var6: i32,
    var7: i32,
) {
    let color = item(var3).map_or(16777215, |i| i.get_color_from_damage(var4));
    let rgb = |c: i32| {
        (
            (c >> 16 & 255) as f32 / 255.0,
            (c >> 8 & 255) as f32 / 255.0,
            (c & 255) as f32 / 255.0,
        )
    };
    if var3 < 256 && render_item_in_3d(blocks::get_render_type(var3)) {
        g.bind_texture(TERRAIN_PNG);
        ig.push_matrix(g);
        ig.translatef(g, (var6 - 2) as f32, (var7 + 3) as f32, -3.0);
        ig.scalef(g, 10.0, 10.0, 10.0);
        ig.translatef(g, 1.0, 0.5, 1.0);
        ig.scalef(g, 1.0, 1.0, -1.0);
        ig.rotatef(g, 210.0, 1.0, 0.0, 0.0);
        ig.rotatef(g, 45.0, 0.0, 1.0, 0.0);
        let (var11, var12, var13) = rgb(color);
        if colorize {
            g.color4f(var11, var12, var13, 1.0);
        }
        ig.rotatef(g, -90.0, 0.0, 1.0, 0.0);
        // renderBlocks.field_31088_b = field_27004_a: renderBlockOnInventory
        // sets glColor from getRenderColor(meta) × brightness 1 only then;
        // field_31088_b = true again afterwards (RenderItem.java:138-140).
        if let Some(d) = render_block_as_item(statics, empty_colorizers(), var3, var4, 1.0) {
            if colorize && let Some(c) = d.gl_color {
                g.color4f(c[0], c[1], c[2], c[3]);
            }
            let saved = ig.clip;
            ig.clip = clip;
            ig.draw_tessellator(g, &d.tess);
            ig.clip = saved;
        }
        ig.pop_matrix(g);
    } else if var5 >= 0 {
        g.disable(gl::LIGHTING);
        if var3 < 256 {
            g.bind_texture(TERRAIN_PNG);
        } else {
            g.bind_texture(ITEMS_PNG);
        }
        let (var9, var10, var11) = rgb(color);
        if colorize {
            g.color4f(var9, var10, var11, 1.0);
        }
        let (u, v) = (var5 % 16 * 16, var5 / 16 * 16);
        match clip {
            None => render_textured_quad(g, var6, var7, u, v, 16, 16),
            // renderTexturedQuad's quad (z 0, texel 1/256), cut.
            Some(_) => {
                draw_textured_rect_clipped(g, 0.0, clip, var6, var7, u, v, 16, 16, 0.00390625)
            }
        }
        g.enable(gl::LIGHTING);
    }
    ig.cull_face = true;
}

/// Java's `Math.round(double)` as `(int)` of the `long`: `floor(x + 0.5)`.
pub fn java_round(x: f64) -> i32 {
    (x + 0.5).floor() as i64 as i32
}

/// The damage bar's widths and colours for a damaged stack: the green
/// part's width (`var11`, of 13), the bar's colour (`var9`) and the
/// shadow's (`var10`).
// src: RenderItem.java:183-190
pub fn damage_bar(damage: i32, max_damage: i32) -> (i32, i32, i32) {
    let var11 = java_round(13.0 - damage as f64 * 13.0 / max_damage as f64);
    let var7 = java_round(255.0 - damage as f64 * 255.0 / max_damage as f64);
    let var9 = (255 - var7) << 16 | var7 << 8;
    let var10 = ((255 - var7) / 4) << 16 | 16128;
    (var11, var9, var10)
}

/// `renderItemOverlayIntoGUI(font, renderEngine, stack, x, y)`: the stack
/// size (above 1) at the slot's bottom right, with shadow and no depth
/// test; for a damaged item the damage bar (a black 13×2 bar, the shadow
/// 12×1 and the coloured part) at the bottom.
// src: RenderItem.java:171-201
pub fn render_item_overlay_into_gui(
    g: &mut GuiGl,
    var1: &FontRenderer,
    var3: Option<&ItemStack>,
    var4: i32,
    var5: i32,
) {
    let Some(var3) = var3 else {
        return;
    };
    if var3.stack_size > 1 {
        let var6 = var3.stack_size.to_string();
        g.disable(gl::LIGHTING);
        g.disable(gl::DEPTH_TEST);
        var1.draw_string_with_shadow(
            g,
            &var6,
            var4 + 19 - 2 - var1.get_string_width(&var6),
            var5 + 6 + 3,
            16777215,
        );
        g.enable(gl::LIGHTING);
        g.enable(gl::DEPTH_TEST);
    }
    if var3.is_item_damaged() {
        let (var11, var9, var10) =
            damage_bar(var3.get_item_damage_for_display(), var3.get_max_damage());
        g.disable(gl::LIGHTING);
        g.disable(gl::DEPTH_TEST);
        g.disable(gl::TEXTURE_2D);
        render_quad(g, var4 + 2, var5 + 13, 13, 2, 0);
        render_quad(g, var4 + 2, var5 + 13, 12, 1, var10);
        render_quad(g, var4 + 2, var5 + 13, var11, 1, var9);
        g.enable(gl::TEXTURE_2D);
        g.enable(gl::LIGHTING);
        g.enable(gl::DEPTH_TEST);
        g.color4f(1.0, 1.0, 1.0, 1.0);
    }
}

/// `renderQuad(tessellator, x, y, w, h, color)`: an opaque untextured
/// rectangle at z 0.
// src: RenderItem.java:203-211
#[allow(clippy::identity_op)]
pub fn render_quad(g: &mut GuiGl, var2: i32, var3: i32, var4: i32, var5: i32, var6: i32) {
    g.start_drawing_quads();
    g.set_color_opaque_i(var6);
    g.add_vertex((var2 + 0) as f64, (var3 + 0) as f64, 0.0);
    g.add_vertex((var2 + 0) as f64, (var3 + var5) as f64, 0.0);
    g.add_vertex((var2 + var4) as f64, (var3 + var5) as f64, 0.0);
    g.add_vertex((var2 + var4) as f64, (var3 + 0) as f64, 0.0);
    g.draw();
}

/// `renderTexturedQuad(x, y, u, v, w, h)`: a `w × h` piece of the bound
/// 256×256 sheet at z 0.
// src: RenderItem.java:213-224
#[allow(clippy::identity_op)]
pub fn render_textured_quad(
    g: &mut GuiGl,
    var1: i32,
    var2: i32,
    var3: i32,
    var4: i32,
    var5: i32,
    var6: i32,
) {
    let var7 = 0.0f64;
    let var8 = 0.00390625f32;
    let var9 = 0.00390625f32;
    g.start_drawing_quads();
    g.add_vertex_with_uv(
        (var1 + 0) as f64,
        (var2 + var6) as f64,
        var7,
        ((var3 + 0) as f32 * var8) as f64,
        ((var4 + var6) as f32 * var9) as f64,
    );
    g.add_vertex_with_uv(
        (var1 + var5) as f64,
        (var2 + var6) as f64,
        var7,
        ((var3 + var5) as f32 * var8) as f64,
        ((var4 + var6) as f32 * var9) as f64,
    );
    g.add_vertex_with_uv(
        (var1 + var5) as f64,
        (var2 + 0) as f64,
        var7,
        ((var3 + var5) as f32 * var8) as f64,
        ((var4 + 0) as f32 * var9) as f64,
    );
    g.add_vertex_with_uv(
        (var1 + 0) as f64,
        (var2 + 0) as f64,
        var7,
        ((var3 + 0) as f32 * var8) as f64,
        ((var4 + 0) as f32 * var9) as f64,
    );
    g.draw();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::font::ChatAllowedCharacters;
    use crate::textures::ArgbImage;

    fn approx(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-5
    }

    #[test]
    fn block_item_matrix_matches_the_gl_sequence() {
        // The unit cube's centre (renderBlockOnInventory draws it around the
        // origin) lands at the translate's (x - 2 + 10, y + 3 + 5, -3 + 10).
        let m = block_item_matrix(8, 4);
        let c = m.transform_point3(Vec3::ZERO);
        assert!(approx(c, Vec3::new(16.0, 12.0, 7.0)), "{c}");
        // Scale 10 with the z mirror: determinant -1000.
        assert!((m.determinant() + 1000.0).abs() < 1e-2);
        // The top face (+y) points up the screen (−y) and toward the
        // viewer (+z) after the 210° tilt.
        let up = Mat3::from_mat4(m) * Vec3::Y;
        assert!(up.y < 0.0 && up.z > 0.0, "{up}");
        // The ItemGl sequence builds the same matrix as GuiGl.
        let mut ig = ItemGl::new();
        let mut g = GuiGl::default();
        ig.translatef(&mut g, 6.0, 7.0, -3.0);
        ig.scalef(&mut g, 10.0, 10.0, 10.0);
        ig.translatef(&mut g, 1.0, 0.5, 1.0);
        ig.scalef(&mut g, 1.0, 1.0, -1.0);
        ig.rotatef(&mut g, 210.0, 1.0, 0.0, 0.0);
        ig.rotatef(&mut g, 45.0, 0.0, 1.0, 0.0);
        ig.rotatef(&mut g, -90.0, 0.0, 1.0, 0.0);
        assert!(ig.matrix.abs_diff_eq(m, 1e-5));
        g.start_drawing_quads();
        for _ in 0..4 {
            g.add_vertex(0.0, 0.0, 0.0);
        }
        g.draw();
        let p = g.draws()[0].batch.positions[0];
        assert!(approx(Vec3::from(p), Vec3::new(16.0, 12.0, 7.0)));
    }

    #[test]
    fn rescaled_normals_are_unit_under_uniform_scale() {
        let m = block_item_matrix(0, 0);
        let n = eye_normal(m, Vec3::Y, true);
        assert!((n.length() - 1.0).abs() < 1e-5);
        // Without rescaling a scale of 10 shrinks the normal to 1/10.
        let n = eye_normal(m, Vec3::Y, false);
        assert!((n.length() - 0.1).abs() < 1e-5);
    }

    #[test]
    fn lighting_formula() {
        // Lights from the hotbar's glRotatef(120, 1, 0, 0).
        let mut ig = ItemGl::new();
        let mut g = GuiGl::default();
        ig.push_matrix(&mut g);
        ig.rotatef(&mut g, 120.0, 1.0, 0.0, 0.0);
        ig.enable_standard_item_lighting(&mut g);
        ig.pop_matrix(&mut g);
        assert!(g.state.lighting && !g.state.smooth);
        for l in ig.lights {
            assert!((l.length() - 1.0).abs() < 1e-5);
        }
        // A normal facing away from both lights gets the ambient only.
        let away = -(ig.lights[0] + ig.lights[1]).normalize();
        let c = lit_color([1.0, 0.5, 0.25, 0.8], away, &ig.lights);
        assert_eq!(c, [0.4, 0.2, 0.1, 0.8]);
        // Facing one light exactly: 0.4 + 0.6 + 0.6 max(L0·L1, 0).
        let d = ig.lights[0].dot(ig.lights[1]).max(0.0);
        let c = lit_color([1.0; 4], ig.lights[0], &ig.lights);
        assert_eq!(c[0], (0.4 + 0.6 + 0.6 * d).min(1.0));
    }

    #[test]
    fn winding() {
        // drawTexturedModalRect's order (bottom-left, bottom-right,
        // top-right, top-left in y-down coordinates) is GL's front face.
        assert!(is_front_facing(&[
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
            [0.0, 0.0]
        ]));
        assert!(!is_front_facing(&[
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
            [0.0, 1.0]
        ]));
    }

    #[test]
    fn damage_bar_widths() {
        // A fresh-ish pickaxe (max 59 for wood, Item.java): damage 0 is
        // not drawn; damage 30 of 59: round(13 - 6.61) = 6 (6.389 → 6),
        // green 255 - 129.66 = round(125.34) = 125.
        let (w, c, s) = damage_bar(30, 59);
        assert_eq!(w, 6);
        assert_eq!(c, (255 - 125) << 16 | 125 << 8);
        assert_eq!(s, (130 / 4) << 16 | 16128);
        assert_eq!(damage_bar(1, 59).0, 13);
        assert_eq!(damage_bar(59, 59).0, 0);
        // Math.round(x.5) rounds up.
        assert_eq!(java_round(2.5), 3);
        assert_eq!(java_round(-2.5), -2);
    }

    #[test]
    fn overlay_count_position_and_bar() {
        let mut chars = vec![' '; 96];
        for (i, c) in chars.iter_mut().enumerate() {
            *c = char::from(32 + i as u8);
        }
        let font = FontRenderer::new(
            &ArgbImage::new(128, 128),
            false,
            ChatAllowedCharacters::from_chars(chars),
        );
        let mut g = GuiGl::default();
        let s = ItemStack::new(4, 12, 0);
        render_item_overlay_into_gui(&mut g, &font, Some(&s), 100, 50);
        // Two strings (shadow and text), no depth test.
        assert!(g.draws().iter().all(|d| !d.key.depth_test));
        assert!(!g.draws().is_empty());
        assert!(g.state.depth_test && g.state.lighting);
        // A single item: nothing.
        let mut g = GuiGl::default();
        render_item_overlay_into_gui(&mut g, &font, Some(&ItemStack::new(4, 1, 0)), 0, 0);
        assert!(g.draws().is_empty());
        // A damaged pickaxe (270): three quads at (x + 2, y + 13).
        let mut g = GuiGl::default();
        render_item_overlay_into_gui(&mut g, &font, Some(&ItemStack::new(270, 1, 30)), 0, 0);
        let b = &g.draws()[0].batch;
        assert_eq!(g.draws()[0].texture, None);
        assert_eq!(b.positions.len(), 12);
        assert_eq!(b.positions[0], [2.0, 13.0, 0.0]);
        assert_eq!(b.positions[2], [15.0, 15.0, 0.0]);
        assert_eq!(b.positions[10], [8.0, 14.0, 0.0]);
    }

    #[test]
    fn flat_item_icon_uvs_and_tint() {
        let statics = BlockStatics::default();
        let mut ig = ItemGl::new();
        let mut g = GuiGl::default();
        // The stick (280): icon 53 from items.png, white.
        let stick = ItemStack::new(280, 4, 0);
        render_item_into_gui(&mut ig, &mut g, &statics, Some(&stick), 10, 20);
        let d = &g.draws()[0];
        assert_eq!(d.texture, Some(ITEMS_PNG.into()));
        let i = stick.get_icon_index();
        let (u, v) = ((i % 16 * 16) as f32 / 256.0, (i / 16 * 16) as f32 / 256.0);
        assert_eq!(d.batch.uvs[3], [u, v]);
        assert_eq!(d.batch.positions[0], [10.0, 36.0, 0.0]);
        assert!(g.state.lighting);
    }

    #[test]
    fn block_item_draws_only_front_faces_lit() {
        let statics = BlockStatics::default();
        let mut ig = ItemGl::new();
        let mut g = GuiGl::default();
        ig.push_matrix(&mut g);
        ig.rotatef(&mut g, 120.0, 1.0, 0.0, 0.0);
        ig.enable_standard_item_lighting(&mut g);
        ig.pop_matrix(&mut g);
        ig.rescale_normal = true;
        // Planks (5): a cube, three faces visible.
        render_item_into_gui(
            &mut ig,
            &mut g,
            &statics,
            Some(&ItemStack::new(5, 1, 0)),
            0,
            0,
        );
        let d = &g.draws()[0];
        assert_eq!(d.texture, Some(TERRAIN_PNG.into()));
        assert_eq!(d.batch.positions.len(), 12);
        // Within the 16-pixel cell (plus the cube's corners' reach).
        for p in &d.batch.positions {
            assert!(
                p[0] > -1.0 && p[0] < 17.0 && p[1] > -1.0 && p[1] < 18.0,
                "{p:?}"
            );
        }
        // The three faces are shaded differently.
        let mut shades: Vec<_> = (0..3).map(|f| d.batch.colors[f * 4 + 3][0]).collect();
        shades.dedup();
        assert_eq!(shades.len(), 3, "{shades:?}");
    }

    fn lit_planks(colorize: bool, clip: Option<Clip>) -> GuiGl {
        let statics = BlockStatics::default();
        let mut ig = ItemGl::new();
        let mut g = GuiGl::default();
        ig.push_matrix(&mut g);
        ig.rotatef(&mut g, 180.0, 1.0, 0.0, 0.0);
        ig.enable_standard_item_lighting(&mut g);
        ig.pop_matrix(&mut g);
        ig.rescale_normal = true;
        // GuiAchievements' grey for an achievement that cannot be taken.
        g.color4f(0.1, 0.1, 0.1, 1.0);
        draw_item_into_gui_with(&mut ig, &mut g, &statics, colorize, clip, 5, 0, 4, 20, 30);
        assert_eq!(ig.clip, None, "the clip is the call's only");
        g
    }

    #[test]
    fn colorize_off_keeps_the_callers_grey() {
        // field_27004_a true: renderBlockOnInventory's white × the light.
        let lit = lit_planks(true, None);
        let on = lit.draws()[0].batch.colors.clone();
        // false: the 0.1 grey × the same light (GuiAchievements.java:268-280).
        let grey = lit_planks(false, None);
        let off = grey.draws()[0].batch.colors.clone();
        assert_eq!(on.len(), off.len());
        for (a, b) in on.iter().zip(&off) {
            // 0.1 × (0.4 + 0.6 Σ max(n·L, 0)), at most 0.1 × 1.6; the
            // colorized face is that factor on white, clamped to 1.
            assert!(b[0] >= 0.04 - 1e-3 && b[0] <= 0.16 + 1e-3, "{b:?}");
            assert!(
                (b[0] - (a[0] * 0.1)).abs() <= 1.5 / 255.0 || a[0] == 1.0,
                "{a:?} {b:?}"
            );
            assert_eq!(b[3], 1.0);
        }
        assert!(on.iter().all(|c| c[0] >= 0.4));
        // A flat item (the stick) keeps the grey too, and with
        // colorize its getColorFromDamage white.
        let statics = BlockStatics::default();
        for (colorize, want) in [(false, 0.1f32), (true, 1.0)] {
            let mut ig = ItemGl::new();
            let mut g = GuiGl::default();
            g.color4f(0.1, 0.1, 0.1, 1.0);
            draw_item_into_gui_with(&mut ig, &mut g, &statics, colorize, None, 280, 0, 53, 0, 0);
            assert_eq!(g.state.color, [want, want, want, 1.0]);
            assert!(g.state.lighting);
        }
    }

    #[test]
    fn clipped_block_icons_stay_in_the_window() {
        let whole = lit_planks(true, None);
        let wb = &whole.draws()[0].batch;
        let (mut x0, mut x1) = (f32::MAX, f32::MIN);
        for p in &wb.positions {
            x0 = x0.min(p[0]);
            x1 = x1.max(p[0]);
        }
        // Cut through the middle of the cube.
        let mid = ((x0 + x1) / 2.0).floor() as i32;
        let c = Clip {
            x0: mid,
            y0: -100,
            x1: 1000,
            y1: 1000,
        };
        let cut = lit_planks(true, Some(c));
        let cb = &cut.draws()[0].batch;
        assert!(!cb.positions.is_empty());
        assert!(cb.positions.iter().all(|p| p[0] >= mid as f32 - 1e-3));
        assert!(cb.positions.iter().any(|p| (p[0] - x1).abs() < 1e-3));
        // Outside the window: nothing.
        let away = Clip {
            x0: 500,
            y0: 500,
            x1: 600,
            y1: 600,
        };
        assert!(lit_planks(true, Some(away)).draws().is_empty());
        // A clip around everything draws the same triangles' area.
        let all = Clip {
            x0: -1000,
            y0: -1000,
            x1: 1000,
            y1: 1000,
        };
        let same = lit_planks(true, Some(all));
        let tri = |a: [f32; 3], b: [f32; 3], c: [f32; 3]| {
            ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])).abs() / 2.0
        };
        // Quads of four vertices, drawn as (0, 1, 2) and (0, 2, 3).
        let area = |b: &crate::gl_material::TessBatch| -> f32 {
            b.positions
                .chunks(4)
                .map(|q| tri(q[0], q[1], q[2]) + tri(q[0], q[2], q[3]))
                .sum()
        };
        assert!((area(&same.draws()[0].batch) - area(wb)).abs() < 1e-2);
    }

    #[test]
    fn polygon_clipping_and_fans() {
        let v = |x: f32, y: f32| ClipVertex {
            pos: Vec3::new(x, y, 0.0),
            uv: [x / 10.0, y / 10.0],
            color: [1.0; 4],
        };
        let sq = [v(0.0, 0.0), v(10.0, 0.0), v(10.0, 10.0), v(0.0, 10.0)];
        let c = Clip {
            x0: 5,
            y0: -5,
            x1: 20,
            y1: 20,
        };
        let out = clip_polygon(&sq, c);
        assert_eq!(out.len(), 4);
        for p in &out {
            assert!(p.pos.x >= 5.0);
            // uv follows the position
            assert!((p.uv[0] - p.pos.x / 10.0).abs() < 1e-6);
        }
        // The hypotenuse cut off: five vertices, two fan quads.
        let c = Clip {
            x0: -5,
            y0: -5,
            x1: 8,
            y1: 8,
        };
        let tri = [v(0.0, 0.0), v(10.0, 0.0), v(0.0, 10.0)];
        let out = clip_polygon(&tri, c);
        // (0, 0), (8, 0), (8, 2), (2, 8), (0, 8)
        assert_eq!(out.len(), 5);
        assert_eq!(fan_quads(&out).len(), 2);
        let fans = fan_quads(&clip_polygon(&sq, c));
        assert_eq!(fans.len(), 1);
        let pent = [
            v(0.0, 0.0),
            v(4.0, 0.0),
            v(6.0, 3.0),
            v(2.0, 6.0),
            v(-2.0, 3.0),
        ];
        let fans = fan_quads(&pent);
        assert_eq!(fans.len(), 2);
        assert_eq!(fans[1][2], fans[1][3]);
        assert!(
            clip_polygon(
                &sq,
                Clip {
                    x0: 20,
                    y0: 20,
                    x1: 30,
                    y1: 30
                }
            )
            .is_empty()
        );
    }
}
