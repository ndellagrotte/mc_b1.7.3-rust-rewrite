//! `ModelRenderer`, `TexturedQuad` and `PositionTextureVertex`
//! (ModelRenderer.java, TexturedQuad.java, PositionTextureVertex.java): one
//! box of a model, its 8 corners and 6 textured faces, its rotation point and
//! angles, and the vertices its display list holds.
//!
//! The Java compiles each part's faces into a display list once, at the
//! scale of its first `render` (always 0.0625 in the client), and then only
//! moves it with `glTranslatef`/`glRotatef`. Here the display list is a Bevy
//! [`Mesh`] built once ([`ModelPart::mesh`]) and the moves are the part's
//! [`Transform`] ([`ModelPart::transform`]).
//!
//! Normals: `TexturedQuad.draw` sets one normal per face through
//! `Tessellator.setNormal`, which packs it into three signed bytes with a
//! sign-extension bug (vanilla bug, kept: [`tess_normal`]); the mesh carries
//! the normal as GL then reads it.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use mc_core::math::sqrt_double;

/// The texture size every b1.7.3 model's UVs are divided by: `TexturedQuad`
/// hard-codes 64×32 (there is no `textureWidth`/`textureHeight` yet).
// src: TexturedQuad.java:15-18
pub const TEXTURE_WIDTH: f32 = 64.0;
/// See [`TEXTURE_WIDTH`].
// src: TexturedQuad.java:15-18
pub const TEXTURE_HEIGHT: f32 = 32.0;

/// The scale every client caller renders models at (`0.0625F`, one texel
/// of a 16-pixel block): the display lists are compiled at it.
// src: RenderLiving.java:39, RenderMinecart.java:74, RenderBoat.java:32
pub const MODEL_SCALE: f32 = 0.0625;

/// `PositionTextureVertex`: a corner and a texture coordinate. The Java
/// keeps the position as a `Vec3D` of doubles made from floats, so `f32`
/// holds it exactly.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositionTextureVertex {
    /// `vector3D`.
    pub pos: [f32; 3],
    /// `texturePositionX`.
    pub u: f32,
    /// `texturePositionY`.
    pub v: f32,
}

impl PositionTextureVertex {
    /// `new PositionTextureVertex(x, y, z, u, v)`.
    // src: PositionTextureVertex.java:8-10
    pub fn new(x: f32, y: f32, z: f32, u: f32, v: f32) -> Self {
        PositionTextureVertex {
            pos: [x, y, z],
            u,
            v,
        }
    }

    /// `setTexturePosition(u, v)`: the same corner with new coordinates.
    // src: PositionTextureVertex.java:12-14
    pub fn set_texture_position(&self, u: f32, v: f32) -> Self {
        PositionTextureVertex {
            pos: self.pos,
            u,
            v,
        }
    }
}

/// `TexturedQuad`: four vertices in drawing order. `corner` records which
/// of the box's corners each vertex is (the Java shares the `Vec3D`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TexturedQuad {
    /// `vertexPositions`.
    pub vertex_positions: [PositionTextureVertex; 4],
    /// The box corner index (0..8) of each vertex.
    pub corner: [usize; 4],
    /// `invertNormal` (never set in b1.7.3).
    pub invert_normal: bool,
}

impl TexturedQuad {
    /// `new TexturedQuad(vertices, u1, v1, u2, v2)`: the vertices get the
    /// texel rectangle's corners divided by 64×32, inset by half a texel's
    /// fraction (`0.0015625F`, `0.003125F`): `[u2, v1]`, `[u1, v1]`,
    /// `[u1, v2]`, `[u2, v2]`.
    // src: TexturedQuad.java:13-21
    pub fn new(
        v: [PositionTextureVertex; 4],
        corner: [usize; 4],
        var2: i32,
        var3: i32,
        var4: i32,
        var5: i32,
    ) -> Self {
        let var6 = 0.0015625f32;
        let var7 = 0.003125f32;
        let vertex_positions = [
            v[0].set_texture_position(
                var4 as f32 / TEXTURE_WIDTH - var6,
                var3 as f32 / TEXTURE_HEIGHT + var7,
            ),
            v[1].set_texture_position(
                var2 as f32 / TEXTURE_WIDTH + var6,
                var3 as f32 / TEXTURE_HEIGHT + var7,
            ),
            v[2].set_texture_position(
                var2 as f32 / TEXTURE_WIDTH + var6,
                var5 as f32 / TEXTURE_HEIGHT - var7,
            ),
            v[3].set_texture_position(
                var4 as f32 / TEXTURE_WIDTH - var6,
                var5 as f32 / TEXTURE_HEIGHT - var7,
            ),
        ];
        TexturedQuad {
            vertex_positions,
            corner,
            invert_normal: false,
        }
    }

    /// `flipFace()`: the vertex order reversed.
    // src: TexturedQuad.java:23-31
    pub fn flip_face(&mut self) {
        self.vertex_positions.reverse();
        self.corner.reverse();
    }

    /// The normal `draw` passes to `Tessellator.setNormal`: `(v1 - v2) ×
    /// (v0 - v1)` normalised (`Vec3D.subtract(o)` is `o - this`), in doubles,
    /// cast to float.
    // src: TexturedQuad.java:34-42, Vec3D.java:59-70
    pub fn face_normal(&self) -> [f32; 3] {
        let p = |i: usize| {
            let q = self.vertex_positions[i].pos;
            [q[0] as f64, q[1] as f64, q[2] as f64]
        };
        let (v0, v1, v2) = (p(0), p(1), p(2));
        // var3 = v1.subtract(v0) = v0 - v1; var4 = v1.subtract(v2) = v2 - v1.
        let var3 = [v0[0] - v1[0], v0[1] - v1[1], v0[2] - v1[2]];
        let var4 = [v2[0] - v1[0], v2[1] - v1[1], v2[2] - v1[2]];
        // var4.crossProduct(var3).
        let c = [
            var4[1] * var3[2] - var4[2] * var3[1],
            var4[2] * var3[0] - var4[0] * var3[2],
            var4[0] * var3[1] - var4[1] * var3[0],
        ];
        // Vec3D.normalize: MathHelper.sqrt_double (a float), zero below 1e-4.
        let len = sqrt_double(c[0] * c[0] + c[1] * c[1] + c[2] * c[2]) as f64;
        let n = if len < 1.0e-4 {
            [0.0, 0.0, 0.0]
        } else {
            [c[0] / len, c[1] / len, c[2] / len]
        };
        if self.invert_normal {
            [-(n[0] as f32), -(n[1] as f32), -(n[2] as f32)]
        } else {
            [n[0] as f32, n[1] as f32, n[2] as f32]
        }
    }
}

/// `Tessellator.setNormal(x, y, z)` as GL then reads it: the components go
/// into signed bytes, `x` scaled by 128 (vanilla bug: `x = 1` wraps to
/// `-128`, so a `+x` face gets the `-x` normal) and `y`, `z` by 127, packed as
/// `x | y << 8 | z << 16` with each byte sign-extended to an `int` first
/// (vanilla bug: a negative `x` sets the `y` and `z` bytes to `0xFF`, a
/// negative `y` the `z` byte). `glNormalPointer(GL_BYTE)` then reads the low
/// three bytes, each as `(2c + 1) / 255` (the signed-byte conversion of the
/// GL versions LWJGL 2 contexts get; GL 4.2 changed it to `c / 127`).
///
/// So both x faces of every box are lit as `-x`, and the `-x`/`-y` faces
/// carry small `-1/255` components.
// src: Tessellator.java:293-302, :93-98, :259-260
pub fn tess_normal(x: f32, y: f32, z: f32) -> [f32; 3] {
    let var4 = ((x * 128.0) as i32) as i8;
    let var5 = ((y * 127.0) as i32) as i8;
    let var6 = ((z * 127.0) as i32) as i8;
    let normal = (var4 as i32) | ((var5 as i32) << 8) | ((var6 as i32) << 16);
    let b = |shift: u32| ((normal >> shift) & 0xFF) as u8 as i8;
    let conv = |c: i8| (2.0 * c as f32 + 1.0) / 255.0;
    [conv(b(0)), conv(b(8)), conv(b(16))]
}

/// How a part is drawn: which of `ModelRenderer`'s draw methods the model
/// calls (they differ in the rotation order).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RenderMode {
    /// `render(scale)` (and `postRender`): translate to the rotation point,
    /// then rotate Z, Y, X.
    Render,
    /// `renderWithRotation(scale)`: translate, then rotate Y, X, Z.
    WithRotation,
}

/// The box geometry `addBox` makes: the corners and the faces.
#[derive(Clone, Debug, PartialEq)]
pub struct PartBox {
    /// `corners`.
    pub corners: [PositionTextureVertex; 8],
    /// `faces`.
    pub faces: [TexturedQuad; 6],
}

/// `ModelRenderer`: one box of a model (b1.7.3 parts have no children).
#[derive(Clone, Debug, PartialEq)]
pub struct ModelPart {
    /// The Java field name (`bipedHead`, `tentacles[3]`, …).
    pub name: &'static str,
    /// `textureOffsetX`.
    pub texture_offset_x: i32,
    /// `textureOffsetY`.
    pub texture_offset_y: i32,
    /// `rotationPointX`.
    pub rotation_point_x: f32,
    /// `rotationPointY`.
    pub rotation_point_y: f32,
    /// `rotationPointZ`.
    pub rotation_point_z: f32,
    /// `rotateAngleX`, radians.
    pub rotate_angle_x: f32,
    /// `rotateAngleY`, radians.
    pub rotate_angle_y: f32,
    /// `rotateAngleZ`, radians.
    pub rotate_angle_z: f32,
    /// `mirror` (read by `addBox`).
    pub mirror: bool,
    /// `showModel`.
    pub show_model: bool,
    /// `isHidden`.
    pub is_hidden: bool,
    /// The box (`corners`, `faces`); `None` before `addBox`.
    pub geometry: Option<PartBox>,
}

impl ModelPart {
    /// `new ModelRenderer(textureOffsetX, textureOffsetY)`.
    // src: ModelRenderer.java:22-25
    pub fn new(name: &'static str, var1: i32, var2: i32) -> Self {
        ModelPart {
            name,
            texture_offset_x: var1,
            texture_offset_y: var2,
            rotation_point_x: 0.0,
            rotation_point_y: 0.0,
            rotation_point_z: 0.0,
            rotate_angle_x: 0.0,
            rotate_angle_y: 0.0,
            rotate_angle_z: 0.0,
            mirror: false,
            show_model: true,
            is_hidden: false,
            geometry: None,
        }
    }

    /// `new ModelRenderer(u, v)` with `mirror = true` set before `addBox`.
    pub fn mirrored(name: &'static str, var1: i32, var2: i32) -> Self {
        let mut p = ModelPart::new(name, var1, var2);
        p.mirror = true;
        p
    }

    /// `addBox(x, y, z, w, h, d)` (no inflation).
    // src: ModelRenderer.java:27-29
    pub fn add_box(&mut self, var1: f32, var2: f32, var3: f32, var4: i32, var5: i32, var6: i32) {
        self.add_box_inflated(var1, var2, var3, var4, var5, var6, 0.0);
    }

    /// `addBox(x, y, z, w, h, d, inflate)`: the box from `(x, y, z)` to
    /// `(x + w, y + h, z + d)`, grown by `inflate` on every side, its x
    /// extremes swapped when `mirror` is set; the six faces with the texture
    /// layout (`d` = depth, `w` = width, `h` = height, from the texture
    /// offset `(u, v)`):
    ///
    /// ```text
    ///            u+d     u+d+w   u+d+2w
    ///   v        | top   | bottom|
    ///   v+d  left| front | right | back
    ///   v+d+h
    /// ```
    ///
    /// (face 0 is `+x` at `u+d+w`, face 1 `-x` at `u`, face 2 `-y` (top in
    /// model space, y down) at `u+d`, face 3 `+y` at `u+d+w`, face 4 `-z`
    /// (front) at `u+d`, face 5 `+z` at `u+2d+w`); mirrored boxes get every
    /// face's vertex order reversed.
    #[allow(clippy::too_many_arguments)]
    // src: ModelRenderer.java:31-77
    pub fn add_box_inflated(
        &mut self,
        mut var1: f32,
        mut var2: f32,
        mut var3: f32,
        var4: i32,
        var5: i32,
        var6: i32,
        var7: f32,
    ) {
        let mut var8 = var1 + var4 as f32;
        let mut var9 = var2 + var5 as f32;
        let mut var10 = var3 + var6 as f32;
        var1 -= var7;
        var2 -= var7;
        var3 -= var7;
        var8 += var7;
        var9 += var7;
        var10 += var7;
        if self.mirror {
            std::mem::swap(&mut var8, &mut var1);
        }

        let var20 = PositionTextureVertex::new(var1, var2, var3, 0.0, 0.0);
        let var12 = PositionTextureVertex::new(var8, var2, var3, 0.0, 8.0);
        let var13 = PositionTextureVertex::new(var8, var9, var3, 8.0, 8.0);
        let var14 = PositionTextureVertex::new(var1, var9, var3, 8.0, 0.0);
        let var15 = PositionTextureVertex::new(var1, var2, var10, 0.0, 0.0);
        let var16 = PositionTextureVertex::new(var8, var2, var10, 0.0, 8.0);
        let var17 = PositionTextureVertex::new(var8, var9, var10, 8.0, 8.0);
        let var18 = PositionTextureVertex::new(var1, var9, var10, 8.0, 0.0);
        let corners = [var20, var12, var13, var14, var15, var16, var17, var18];
        let (tx, ty) = (self.texture_offset_x, self.texture_offset_y);
        // Corner indices: var20 0, var12 1, var13 2, var14 3, var15 4,
        // var16 5, var17 6, var18 7.
        let q = |i: [usize; 4], u1: i32, v1: i32, u2: i32, v2: i32| {
            TexturedQuad::new(
                [corners[i[0]], corners[i[1]], corners[i[2]], corners[i[3]]],
                i,
                u1,
                v1,
                u2,
                v2,
            )
        };
        let mut faces = [
            q(
                [5, 1, 2, 6],
                tx + var6 + var4,
                ty + var6,
                tx + var6 + var4 + var6,
                ty + var6 + var5,
            ),
            q([0, 4, 7, 3], tx, ty + var6, tx + var6, ty + var6 + var5),
            q([5, 4, 0, 1], tx + var6, ty, tx + var6 + var4, ty + var6),
            q(
                [2, 3, 7, 6],
                tx + var6 + var4,
                ty,
                tx + var6 + var4 + var4,
                ty + var6,
            ),
            q(
                [1, 0, 3, 2],
                tx + var6,
                ty + var6,
                tx + var6 + var4,
                ty + var6 + var5,
            ),
            q(
                [4, 5, 6, 7],
                tx + var6 + var4 + var6,
                ty + var6,
                tx + var6 + var4 + var6 + var4,
                ty + var6 + var5,
            ),
        ];
        if self.mirror {
            for f in &mut faces {
                f.flip_face();
            }
        }
        self.geometry = Some(PartBox { corners, faces });
    }

    /// `setRotationPoint(x, y, z)`.
    // src: ModelRenderer.java:79-83
    pub fn set_rotation_point(&mut self, var1: f32, var2: f32, var3: f32) {
        self.rotation_point_x = var1;
        self.rotation_point_y = var2;
        self.rotation_point_z = var3;
    }

    /// Whether `render` draws anything: not `isHidden`, `showModel`, and
    /// a box was added.
    // src: ModelRenderer.java:86-87
    pub fn is_drawn(&self) -> bool {
        !self.is_hidden && self.show_model && self.geometry.is_some()
    }

    /// The display list's vertices: per face (in order) its four vertices
    /// at `corner * scale` (`(float)x * scale`), its texture coordinates and
    /// the normal GL reads ([`tess_normal`]).
    // src: ModelRenderer.java:180-191, TexturedQuad.java:33-50
    pub fn display_list(&self, scale: f32) -> PartVertices {
        let mut out = PartVertices::default();
        let Some(g) = &self.geometry else {
            return out;
        };
        for f in &g.faces {
            let n = f.face_normal();
            let n = tess_normal(n[0], n[1], n[2]);
            for v in &f.vertex_positions {
                out.positions
                    .push([v.pos[0] * scale, v.pos[1] * scale, v.pos[2] * scale]);
                out.uvs.push([v.u, v.v]);
                out.normals.push(n);
            }
        }
        out
    }

    /// The part's display list as a Bevy mesh ([`ModelPart::display_list`]):
    /// position, UV and normal per vertex, each face's quad as the triangles
    /// `0 1 2`, `0 2 3`. `None` without a box.
    pub fn mesh(&self, scale: f32) -> Option<Mesh> {
        self.geometry.as_ref()?;
        Some(self.display_list(scale).into_mesh())
    }

    /// The part's `glTranslatef`/`glRotatef`s as a transform from the
    /// part's display list to the model's space: translate by the rotation
    /// point × `scale`, then rotate Z, Y, X ([`RenderMode::Render`], also
    /// `postRender`) or Y, X, Z ([`RenderMode::WithRotation`]). The Java
    /// skips the zero rotations and translation, which changes nothing.
    /// Angles go to `glRotatef` as `angle * 57.295776F` degrees; the
    /// radians are used here directly.
    // src: ModelRenderer.java:85-121 (render), :123-148 (renderWithRotation), :150-178 (postRender)
    pub fn transform(&self, mode: RenderMode, scale: f32) -> Transform {
        let t = Vec3::new(
            self.rotation_point_x * scale,
            self.rotation_point_y * scale,
            self.rotation_point_z * scale,
        );
        let (x, y, z) = (
            Quat::from_rotation_x(self.rotate_angle_x),
            Quat::from_rotation_y(self.rotate_angle_y),
            Quat::from_rotation_z(self.rotate_angle_z),
        );
        let r = match mode {
            RenderMode::Render => z * y * x,
            RenderMode::WithRotation => y * x * z,
        };
        Transform::from_translation(t).with_rotation(r)
    }
}

/// A part's display list ([`ModelPart::display_list`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PartVertices {
    /// Positions (model units × scale).
    pub positions: Vec<[f32; 3]>,
    /// Texture coordinates (0..1 over 64×32).
    pub uvs: Vec<[f32; 2]>,
    /// Normals as GL reads them.
    pub normals: Vec<[f32; 3]>,
}

impl PartVertices {
    /// A triangle-list mesh: every four vertices one `GL_QUADS` quad.
    pub fn into_mesh(self) -> Mesh {
        let n = self.positions.len() as u32 / 4;
        let mut idx = Vec::with_capacity(n as usize * 6);
        for q in 0..n {
            let b = q * 4;
            idx.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
        }
        let mut m = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        m.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        m.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        m.insert_indices(Indices::U32(idx));
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tessellator_normal_bytes() {
        let a = 1.0 / 255.0;
        // +y and +z survive.
        assert_eq!(tess_normal(0.0, 1.0, 0.0), [a, 255.0 / 255.0, a]);
        assert_eq!(tess_normal(0.0, 0.0, 1.0), [a, a, 1.0]);
        // -z: the z byte only.
        assert_eq!(tess_normal(0.0, 0.0, -1.0), [a, a, -253.0 / 255.0]);
        // -y: the sign extension fills the z byte.
        assert_eq!(tess_normal(0.0, -1.0, 0.0), [a, -253.0 / 255.0, -a]);
        // +x wraps to -128, and both x faces fill y and z with 0xFF.
        assert_eq!(tess_normal(1.0, 0.0, 0.0), [-1.0, -a, -a]);
        assert_eq!(tess_normal(-1.0, 0.0, 0.0), [-1.0, -a, -a]);
    }

    /// A known box, mirror off and on, against the Java's corners, face
    /// corner order and UVs (ModelRenderer.java:31-77, TexturedQuad.java).
    #[test]
    fn add_box_layout_mirror_off_and_on() {
        let mut p = ModelPart::new("t", 40, 16);
        p.add_box_inflated(-3.0, -2.0, -2.0, 4, 12, 4, 0.0);
        let g = p.geometry.clone().unwrap();
        assert_eq!(g.corners[0].pos, [-3.0, -2.0, -2.0]);
        assert_eq!(g.corners[6].pos, [1.0, 10.0, 2.0]);
        let order: Vec<[usize; 4]> = g.faces.iter().map(|f| f.corner).collect();
        assert_eq!(
            order,
            vec![
                [5, 1, 2, 6],
                [0, 4, 7, 3],
                [5, 4, 0, 1],
                [2, 3, 7, 6],
                [1, 0, 3, 2],
                [4, 5, 6, 7]
            ]
        );
        // Face 4 (front, -z): texels u 44..48, v 20..32.
        let f = &g.faces[4].vertex_positions;
        assert_eq!(
            (f[0].u, f[0].v),
            (48.0 / 64.0 - 0.0015625, 20.0 / 32.0 + 0.003125)
        );
        assert_eq!(
            (f[1].u, f[1].v),
            (44.0 / 64.0 + 0.0015625, 20.0 / 32.0 + 0.003125)
        );
        assert_eq!(
            (f[2].u, f[2].v),
            (44.0 / 64.0 + 0.0015625, 32.0 / 32.0 - 0.003125)
        );
        assert_eq!(
            (f[3].u, f[3].v),
            (48.0 / 64.0 - 0.0015625, 32.0 / 32.0 - 0.003125)
        );
        assert_eq!(f[0].pos, [1.0, -2.0, -2.0]);
        assert_eq!(f[1].pos, [-3.0, -2.0, -2.0]);

        let mut m = ModelPart::mirrored("m", 40, 16);
        m.add_box_inflated(-1.0, -2.0, -2.0, 4, 12, 4, 0.0);
        let g = m.geometry.clone().unwrap();
        // x extremes swapped.
        assert_eq!(g.corners[0].pos, [3.0, -2.0, -2.0]);
        assert_eq!(g.corners[1].pos, [-1.0, -2.0, -2.0]);
        // Reversed vertex order, same UV rectangle.
        assert_eq!(g.faces[4].corner, [2, 3, 0, 1]);
        let f = &g.faces[4].vertex_positions;
        assert_eq!(
            (f[3].u, f[3].v),
            (48.0 / 64.0 - 0.0015625, 20.0 / 32.0 + 0.003125)
        );
        assert_eq!(
            (f[0].u, f[0].v),
            (48.0 / 64.0 - 0.0015625, 32.0 / 32.0 - 0.003125)
        );
    }

    #[test]
    fn inflation_grows_every_side() {
        let mut p = ModelPart::new("h", 32, 0);
        p.add_box_inflated(-4.0, -8.0, -4.0, 8, 8, 8, 0.5);
        let g = p.geometry.unwrap();
        assert_eq!(g.corners[0].pos, [-4.5, -8.5, -4.5]);
        assert_eq!(g.corners[6].pos, [4.5, 0.5, 4.5]);
    }

    /// Every face's computed normal points out of the box (mirror off and
    /// on), and the faces wind counter-clockwise seen from outside (GL's
    /// front face).
    #[test]
    fn faces_point_outwards_and_wind_ccw() {
        for mirror in [false, true] {
            let mut p = ModelPart::new("b", 0, 0);
            p.mirror = mirror;
            p.add_box_inflated(-1.0, -2.0, -3.0, 2, 4, 6, 0.0);
            let g = p.geometry.unwrap();
            let want = [
                [1.0, 0.0, 0.0],
                [-1.0, 0.0, 0.0],
                [0.0, -1.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, -1.0],
                [0.0, 0.0, 1.0],
            ];
            for (i, f) in g.faces.iter().enumerate() {
                let n = f.face_normal();
                // Mirrored boxes swap the x faces' places.
                let w = if mirror && i < 2 {
                    want[1 - i]
                } else {
                    want[i]
                };
                assert_eq!(n, w, "mirror {mirror} face {i}");
                let p0 = Vec3::from(f.vertex_positions[0].pos);
                let p1 = Vec3::from(f.vertex_positions[1].pos);
                let p2 = Vec3::from(f.vertex_positions[2].pos);
                let ccw = (p1 - p0).cross(p2 - p0);
                assert!(ccw.dot(Vec3::from(w)) > 0.0, "mirror {mirror} face {i}");
            }
        }
    }

    #[test]
    fn display_list_and_mesh() {
        let mut p = ModelPart::new("b", 0, 0);
        p.add_box(-4.0, -8.0, -4.0, 8, 8, 8);
        let d = p.display_list(MODEL_SCALE);
        assert_eq!(d.positions.len(), 24);
        assert_eq!(d.positions[0], [0.25, -0.5, 0.25]);
        assert_eq!(d.normals[0], tess_normal(1.0, 0.0, 0.0));
        let m = p.mesh(MODEL_SCALE).unwrap();
        assert_eq!(m.count_vertices(), 24);
        assert_eq!(m.indices().unwrap().len(), 36);
        assert!(ModelPart::new("e", 0, 0).mesh(MODEL_SCALE).is_none());
    }

    #[test]
    fn transform_orders() {
        let mut p = ModelPart::new("b", 0, 0);
        p.set_rotation_point(16.0, 0.0, 0.0);
        p.rotate_angle_x = std::f32::consts::FRAC_PI_2;
        p.rotate_angle_z = std::f32::consts::FRAC_PI_2;
        // render: T · Rz · Ry · Rx; a +y point goes to +z under Rx, stays.
        let t = p.transform(RenderMode::Render, MODEL_SCALE);
        let v = t.transform_point(Vec3::Y);
        assert!((v - Vec3::new(1.0, 0.0, 1.0)).length() < 1e-6, "{v}");
        // renderWithRotation: T · Ry · Rx · Rz; +y goes to -x under Rz,
        // which Rx keeps.
        let t = p.transform(RenderMode::WithRotation, MODEL_SCALE);
        let v = t.transform_point(Vec3::Y);
        assert!((v - Vec3::new(0.0, 0.0, 0.0)).length() < 1e-6, "{v}");
    }
}
