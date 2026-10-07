//! `Tessellator` (Tessellator.java) as the block renderer uses it: the
//! current colour, normal and translation, and the vertices of one
//! `GL_QUADS` draw. Project-owned machinery (PLAN_v4 D7) around the
//! transcribed state rules: colours are stored as the Tessellator's bytes
//! (`(int)(v * 255)` clamped to 0..255), positions as `(float)(v + offset)`,
//! texture coordinates as `(float)u`.
//!
//! Vertices without a colour (`hasColor` false, or `disableColor()`) take
//! `glColor` in GL; here they are white, and the material's colour
//! ([`crate::gl_material::GlParams::color`]) multiplies them as `glColor`
//! does.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{
    Indices, MeshVertexAttribute, PrimitiveTopology, VertexAttributeValues, VertexFormat,
};
use bevy::prelude::*;

/// The terrain mesh's colour attribute: the Tessellator's four colour bytes
/// (`Unorm8x4`), as `glColorPointer(4, GL_UNSIGNED_BYTE, …)` reads them
/// (Tessellator.java:85-88).
pub const ATTRIBUTE_TESS_COLOR: MeshVertexAttribute =
    MeshVertexAttribute::new("TessColor", 0x6d63_7433, VertexFormat::Unorm8x4);

/// `Tessellator.instance` for one draw (module docs).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tessellator {
    /// Positions (after the translation).
    pub positions: Vec<[f32; 3]>,
    /// Texture coordinates.
    pub uvs: Vec<[f32; 2]>,
    /// Colours (white where none was set).
    pub colors: Vec<[u8; 4]>,
    /// Normals (`[0; 3]` where none was set).
    pub normals: Vec<[f32; 3]>,
    /// `hasColor`.
    pub has_color: bool,
    /// `isColorDisabled`.
    pub is_color_disabled: bool,
    /// `color` (r, g, b, a bytes).
    pub color: [u8; 4],
    /// `hasNormals`.
    pub has_normals: bool,
    /// `normal`, as passed to `setNormal`.
    pub normal: [f32; 3],
    /// `xOffset`, `yOffset`, `zOffset`.
    pub offset: [f64; 3],
}

impl Tessellator {
    /// `startDrawingQuads()`: no colour, normal or texture yet, colours
    /// enabled; the translation is kept (it is not part of the draw state).
    // src: Tessellator.java:140-158
    pub fn start_drawing_quads(&mut self) {
        self.positions.clear();
        self.uvs.clear();
        self.colors.clear();
        self.normals.clear();
        self.has_color = false;
        self.has_normals = false;
        self.is_color_disabled = false;
    }

    /// `setColorOpaque_F(r, g, b)`.
    // src: Tessellator.java:166-168
    pub fn set_color_opaque_f(&mut self, r: f32, g: f32, b: f32) {
        self.set_color_opaque((r * 255.0) as i32, (g * 255.0) as i32, (b * 255.0) as i32);
    }

    /// `setColorRGBA_F(r, g, b, a)`.
    // src: Tessellator.java:170-172
    pub fn set_color_rgba_f(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.set_color_rgba(
            (r * 255.0) as i32,
            (g * 255.0) as i32,
            (b * 255.0) as i32,
            (a * 255.0) as i32,
        );
    }

    /// `setColorOpaque(r, g, b)`.
    // src: Tessellator.java:174-176
    pub fn set_color_opaque(&mut self, r: i32, g: i32, b: i32) {
        self.set_color_rgba(r, g, b, 255);
    }

    /// `setColorRGBA(r, g, b, a)`: each clamped to 0..255; ignored while the
    /// colour is disabled.
    // src: Tessellator.java:178-213
    pub fn set_color_rgba(&mut self, r: i32, g: i32, b: i32, a: i32) {
        if !self.is_color_disabled {
            let c = |v: i32| v.clamp(0, 255) as u8;
            self.has_color = true;
            self.color = [c(r), c(g), c(b), c(a)];
        }
    }

    /// `disableColor()`.
    // src: Tessellator.java:286-288
    pub fn disable_color(&mut self) {
        self.is_color_disabled = true;
    }

    /// `setNormal(x, y, z)`.
    // src: Tessellator.java:290-300
    pub fn set_normal(&mut self, x: f32, y: f32, z: f32) {
        self.has_normals = true;
        self.normal = [x, y, z];
    }

    /// `setTranslationD(x, y, z)`.
    // src: Tessellator.java:302-306
    pub fn set_translation_d(&mut self, x: f64, y: f64, z: f64) {
        self.offset = [x, y, z];
    }

    /// `setTranslationF(x, y, z)`: adds to the translation.
    // src: Tessellator.java:308-312
    pub fn set_translation_f(&mut self, x: f32, y: f32, z: f32) {
        self.offset[0] += x as f64;
        self.offset[1] += y as f64;
        self.offset[2] += z as f64;
    }

    /// `addVertexWithUV(x, y, z, u, v)`.
    // src: Tessellator.java:215-218, :220-265
    pub fn add_vertex_with_uv(&mut self, x: f64, y: f64, z: f64, u: f64, v: f64) {
        self.positions.push([
            (x + self.offset[0]) as f32,
            (y + self.offset[1]) as f32,
            (z + self.offset[2]) as f32,
        ]);
        self.uvs.push([u as f32, v as f32]);
        self.colors
            .push(if self.has_color { self.color } else { [255; 4] });
        self.normals.push(if self.has_normals {
            self.normal
        } else {
            [0.0; 3]
        });
    }

    /// The vertex count.
    pub fn len(&self) -> usize {
        self.positions.len()
    }

    /// Whether no vertex was added.
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    /// Index list drawing every 4 vertices as two triangles `(0, 1, 2)`,
    /// `(0, 2, 3)`, as `convertQuadsToTriangles` emits them.
    // src: Tessellator.java:12, :222-239
    pub fn quad_indices(&self) -> Vec<u32> {
        let n = self.positions.len() as u32 / 4;
        let mut idx = Vec::with_capacity(n as usize * 6);
        for q in 0..n {
            let b = q * 4;
            idx.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
        }
        idx
    }

    /// The draw as a mesh (position, UV, [`ATTRIBUTE_TESS_COLOR`]; normals
    /// when any vertex had one).
    pub fn to_mesh(&self) -> Mesh {
        let mut m = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        m.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions.clone());
        m.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs.clone());
        m.insert_attribute(
            ATTRIBUTE_TESS_COLOR,
            VertexAttributeValues::Unorm8x4(self.colors.clone()),
        );
        if self.normals.iter().any(|n| *n != [0.0; 3]) {
            m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals.clone());
        }
        m.insert_indices(Indices::U32(self.quad_indices()));
        m
    }

    /// The draw as a flat-shaded mesh: each triangle's vertices take the
    /// colour of its last vertex, GL's provoking vertex under
    /// `glShadeModel(GL_FLAT)` (the quads become `(0, 1, 2)` and `(0, 2, 3)`,
    /// so vertices 2 and 3). Vertices are not shared.
    pub fn to_flat_mesh(&self) -> Mesh {
        let idx = self.quad_indices();
        let mut pos = Vec::with_capacity(idx.len());
        let mut uv = Vec::with_capacity(idx.len());
        let mut col = Vec::with_capacity(idx.len());
        for tri in idx.as_chunks::<3>().0 {
            let c = self.colors[tri[2] as usize];
            for &i in tri {
                pos.push(self.positions[i as usize]);
                uv.push(self.uvs[i as usize]);
                col.push(c);
            }
        }
        let n = pos.len() as u32;
        let mut m = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
        m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
        m.insert_attribute(ATTRIBUTE_TESS_COLOR, VertexAttributeValues::Unorm8x4(col));
        m.insert_indices(Indices::U32((0..n).collect()));
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_truncate_and_clamp_like_the_java() {
        let mut t = Tessellator::default();
        t.set_color_opaque_f(1.0, 0.5, 0.8 * 0.6);
        assert_eq!(t.color, [255, 127, 122, 255]);
        t.set_color_opaque_f(1.2, -0.1, 0.999);
        assert_eq!(t.color, [255, 0, 254, 255]);
        t.disable_color();
        t.set_color_opaque_f(0.0, 0.0, 0.0);
        assert_eq!(t.color, [255, 0, 254, 255]);
        t.start_drawing_quads();
        assert!(!t.is_color_disabled && !t.has_color);
    }

    #[test]
    fn vertices_are_translated_in_double_then_cast() {
        let mut t = Tessellator::default();
        t.set_translation_d(-16.0, -32.0, -48.0);
        t.add_vertex_with_uv(17.5, 33.0, 48.25, 0.5, 0.25);
        assert_eq!(t.positions[0], [1.5, 1.0, 0.25]);
        assert_eq!(t.uvs[0], [0.5, 0.25]);
        assert_eq!(t.colors[0], [255; 4]);
        t.set_translation_f(0.0, 0.0, 0.0625);
        assert_eq!(t.offset, [-16.0, -32.0, -47.9375]);
    }

    #[test]
    fn flat_shading_uses_the_provoking_vertex() {
        let mut t = Tessellator::default();
        for i in 0..4u8 {
            t.set_color_opaque(i as i32 * 10, 0, 0);
            t.add_vertex_with_uv(i as f64, 0.0, 0.0, 0.0, 0.0);
        }
        let m = t.to_flat_mesh();
        let Some(VertexAttributeValues::Unorm8x4(c)) = m.attribute(ATTRIBUTE_TESS_COLOR) else {
            panic!("colours");
        };
        let reds: Vec<u8> = c.iter().map(|c| c[0]).collect();
        assert_eq!(reds, vec![20, 20, 20, 30, 30, 30]);
    }
}
