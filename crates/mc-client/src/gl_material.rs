//! The render plumbing of WP-3.4 (project-owned; PLAN_v4 D7): a Bevy
//! material, [`GlMaterial`], that reproduces the fixed-function GL state the
//! sky, clouds and weather are drawn with — `glColor4f` × the Tessellator's
//! vertex colour × an optional texture, `glAlphaFunc(GL_GREATER, ref)`,
//! `GL_FOG` in `GL_LINEAR` or `GL_EXP` mode, `glBlendFunc`, `glDepthMask`,
//! `glColorMask` — and the helpers to build Tessellator-shaped meshes.
//!
//! # Colours
//!
//! Vanilla writes its colour values straight into an 8-bit framebuffer, so a
//! value `v` shows as the byte `v * 255`. The shader does all of GL's
//! arithmetic (colour products, alpha test, fog) on the Java values.
//! Textures are uploaded as `Rgba8Unorm` (not sRGB) so sampling returns the
//! PNG's bytes / 255, as GL does.
//!
//! Blending: GL blends the bytes it stores. Every camera composites in sRGB
//! (`CompositingSpace::Srgb`, WP-3.7b: `camera::spawn_camera` and
//! `crate::ClientPlugin`), so the main pass target is `Rgba8Unorm`; the
//! shader returns the Java values unconverted (`GL_TARGET_UNORM`, chosen
//! from the pipeline's target format) and the hardware blends exactly as GL
//! does. Bevy's upscale pass decodes the result for the window, so opaque
//! pixels are the bytes vanilla shows.
//!
//! History (before WP-3.7b): on Bevy's default sRGB target the shader
//! converted to linear at the end and the hardware blended linear values,
//! which no output can make exact for every destination (that path is still
//! in the shader, for a camera without `CompositingSpace::Srgb`):
//! - [`GlBlend::Additive`] (sun, moon, stars) returns `linear(c·a) / a`, so
//!   the hardware adds `linear(c·a)`: byte-exact over a black destination
//!   (midnight stars show as byte 64, `getStarBrightness`² = 0.25 as in GL,
//!   where returning `linear(c)` gave 92), too dim over a bright one (the
//!   sun's halo by day nearly vanishes, as it did with `linear(c)`).
//! - [`GlBlend::Alpha`] returns `linear(c)`: exact where `a` is 0 or 1 or
//!   the destination equals the source. The clouds over the sky land within
//!   1-3 bytes; the selection outline (black at 0.4) is much too light
//!   (measured: blue 147 over a 185 sky where GL writes 111); the sunrise
//!   glow is off too.
//!
//! Measured with the camera switched to `CompositingSpace::Srgb` (an
//! experiment then, the live path since WP-3.7b): the outline is 111, the
//! stars 64, the sun shows its halo, and opaque pixels stay exact.
//!
//! # Draw order
//!
//! Everything here is drawn in Bevy's `Transparent3d` phase, which sorts by
//! view depth plus [`Material::depth_bias`]. The biases in [`order`] are
//! large enough to fix vanilla's order (`EntityRenderer.renderWorld`,
//! EntityRenderer.java:466-566): the sky first, then (after the opaque
//! world) the selection box when the eye is in water, then WP-3.3's
//! translucent terrain, then the selection box otherwise, the weather and
//! the clouds. The sky is drawn at the far plane ([`GlKey::far`]) without depth
//! writes, so the world covers it even though Bevy draws it after the opaque
//! pass; that reproduces vanilla drawing it first with `glDepthMask(false)`.
//!
//! Depth: vanilla uses `glDepthFunc(GL_LEQUAL)` (Minecraft.java:265); with
//! Bevy's reverse-Z that is `GreaterEqual`.

use bevy::asset::RenderAssetUsages;
use bevy::asset::{load_internal_asset, uuid_handle};
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, ColorWrites,
    CompareFunction, Face, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
    TextureFormat,
};
use bevy::shader::{Shader, ShaderRef};

use crate::assets::{AssetError, JarAssets};

/// The shader (`shaders/gl_material.wgsl`, embedded).
pub const GL_MATERIAL_SHADER: Handle<Shader> = uuid_handle!("6f0b6a83-4c1e-4f63-9d0e-3b9f1c2a5e71");

/// Sort biases ([`GlMaterial::sort_bias`]): ascending is drawn first
/// (module docs, "Draw order"). Steps are far larger than any view depth a
/// camera-centred mesh's centre can have (≤ 600 blocks).
pub mod order {
    /// The sky plane (`glSkyList`), first of all.
    pub const SKY_PLANE: f32 = -1.0e6;
    /// The sunrise/sunset glow.
    pub const SUNRISE: f32 = -0.99e6;
    /// The sun.
    pub const SUN: f32 = -0.98e6;
    /// The moon.
    pub const MOON: f32 = -0.97e6;
    /// The stars.
    pub const STARS: f32 = -0.96e6;
    /// The void plane (`glSkyList2`), last of the sky.
    pub const VOID: f32 = -0.95e6;
    /// The selection box with the eye in water: after the opaque world,
    /// entities and particles, before the translucent terrain pass, which
    /// then blends over it.
    // src: EntityRenderer.java:507-513
    pub const SELECTION_IN_WATER: f32 = -0.91e6;
    /// The selection box otherwise: after the translucent terrain pass,
    /// before the weather.
    // src: EntityRenderer.java:549-555
    pub const SELECTION: f32 = 0.70e6;
    /// Snow (`renderRainSnow`'s first pass), after the world.
    pub const SNOW: f32 = 0.80e6;
    /// Rain.
    pub const RAIN: f32 = 0.81e6;
    /// The fancy clouds' depth-only pass (`glColorMask(false…)`).
    pub const CLOUDS_DEPTH: f32 = 0.90e6;
    /// The clouds (fast, or the fancy colour pass).
    pub const CLOUDS: f32 = 0.91e6;
}

/// `glBlendFunc` (with `GL_BLEND` on), or blending off.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum GlBlend {
    /// `GL_BLEND` off.
    #[default]
    Off,
    /// `glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA)` (770, 771).
    Alpha,
    /// `glBlendFunc(GL_SRC_ALPHA, GL_ONE)` (770, 1): additive.
    Additive,
}

/// The pipeline half of the GL state (the material's specialisation key).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlKey {
    /// Blending.
    pub blend: GlBlend,
    /// `glDepthMask`.
    pub depth_write: bool,
    /// `glColorMask(true, true, true, true)` (false: all four off).
    pub color_write: bool,
    /// Drawn at the far plane (the sky; module docs).
    pub far: bool,
    /// The mesh is a list of wide lines (`glLineWidth`, width in
    /// [`GlParams::line_width`]): 4 vertices per segment, built by
    /// [`wide_line_mesh`].
    pub wide_lines: bool,
}

impl Default for GlKey {
    fn default() -> Self {
        GlKey {
            blend: GlBlend::Off,
            depth_write: false,
            color_write: true,
            far: false,
            wide_lines: false,
        }
    }
}

/// The uniform half of the GL state (matches `GlParams` in the shader).
#[derive(Clone, Copy, Debug, PartialEq, ShaderType)]
pub struct GlParams {
    /// `glColor4f`, in the Java's colour values.
    pub color: Vec4,
    /// `GL_FOG_COLOR`.
    pub fog_color: Vec4,
    /// Mode (0 off, 1 linear, 2 exp), start, end, density.
    pub fog: Vec4,
    /// The alpha test's reference (`GL_GREATER`); negative: off.
    pub alpha_ref: f32,
    /// 1 when `GL_TEXTURE_2D` is on.
    pub textured: u32,
    /// `glLineWidth`, in pixels (read when [`GlKey::wide_lines`]).
    pub line_width: f32,
}

impl Default for GlParams {
    fn default() -> Self {
        GlParams {
            color: Vec4::ONE,
            fog_color: Vec4::ONE,
            fog: Vec4::ZERO,
            alpha_ref: -1.0,
            textured: 0,
            // GL's initial GL_LINE_WIDTH.
            line_width: 1.0,
        }
    }
}

/// A draw call's GL state (module docs).
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
#[bind_group_data(GlKey)]
pub struct GlMaterial {
    /// Colour, fog, alpha test, texturing.
    #[uniform(0)]
    pub params: GlParams,
    /// The bound texture (used when [`GlParams::textured`] is 1).
    #[texture(1)]
    #[sampler(2)]
    pub texture: Option<Handle<Image>>,
    /// Blend, depth and colour masks.
    pub key: GlKey,
    /// The draw-order bias ([`order`]).
    pub sort_bias: f32,
}

impl From<&GlMaterial> for GlKey {
    fn from(m: &GlMaterial) -> Self {
        m.key
    }
}

impl GlMaterial {
    /// A material with `key`, sort bias `bias` and default parameters.
    pub fn new(key: GlKey, bias: f32) -> Self {
        GlMaterial {
            params: GlParams::default(),
            texture: None,
            key,
            sort_bias: bias,
        }
    }

    /// With a texture bound (`GL_TEXTURE_2D` on).
    pub fn with_texture(mut self, t: Handle<Image>) -> Self {
        self.texture = Some(t);
        self.params.textured = 1;
        self
    }

    /// With `glAlphaFunc(GL_GREATER, r)` and `GL_ALPHA_TEST` on.
    pub fn with_alpha_test(mut self, r: f32) -> Self {
        self.params.alpha_ref = r;
        self
    }
}

impl Material for GlMaterial {
    fn vertex_shader() -> ShaderRef {
        ShaderRef::Handle(GL_MATERIAL_SHADER)
    }

    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(GL_MATERIAL_SHADER)
    }

    fn alpha_mode(&self) -> AlphaMode {
        // The Transparent3d phase, sorted (module docs); the blend state
        // itself comes from the key.
        AlphaMode::Blend
    }

    fn depth_bias(&self) -> f32 {
        self.sort_bias
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let k = key.bind_group_data;
        let mut attributes = vec![
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(2),
        ];
        if k.wide_lines {
            attributes.push(Mesh::ATTRIBUTE_NORMAL.at_shader_location(3));
        }
        let vertex_layout = layout.0.get_layout(&attributes)?;
        descriptor.vertex.buffers = vec![vertex_layout];
        if k.far {
            descriptor.vertex.shader_defs.push("GL_SKY_FAR".into());
        }
        if k.wide_lines {
            descriptor.vertex.shader_defs.push("GL_LINE_WIDTH".into());
            if let Some(frag) = descriptor.fragment.as_mut() {
                frag.shader_defs.push("GL_LINE_WIDTH".into());
            }
        }
        if let Some(frag) = descriptor.fragment.as_mut() {
            // A non-sRGB main target (the camera's CompositingSpace::Srgb):
            // the Java values go out unconverted and the hardware blends
            // GL's bytes (module docs, "Colours").
            let unorm = frag.targets.iter().flatten().any(|t| {
                matches!(
                    t.format,
                    TextureFormat::Rgba8Unorm | TextureFormat::Bgra8Unorm
                )
            });
            if unorm {
                frag.shader_defs.push("GL_TARGET_UNORM".into());
            } else if k.blend == GlBlend::Additive {
                frag.shader_defs.push("GL_BLEND_ADDITIVE".into());
            }
        }
        // glDisable(GL_CULL_FACE) for clouds and weather; the sky's quads
        // face the eye anyway.
        descriptor.primitive.cull_mode = None::<Face>;
        if let Some(ds) = descriptor.depth_stencil.as_mut() {
            ds.depth_write_enabled = Some(k.depth_write);
            ds.depth_compare = Some(CompareFunction::GreaterEqual);
        }
        if let Some(frag) = descriptor.fragment.as_mut() {
            for t in frag.targets.iter_mut().flatten() {
                t.blend = match k.blend {
                    GlBlend::Off => None,
                    GlBlend::Alpha => Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                    }),
                    GlBlend::Additive => Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                    }),
                };
                t.write_mask = if k.color_write {
                    ColorWrites::ALL
                } else {
                    ColorWrites::empty()
                };
            }
        }
        Ok(())
    }
}

/// Registers [`GlMaterial`] and its shader.
pub struct GlMaterialPlugin;

impl Plugin for GlMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            GL_MATERIAL_SHADER,
            "shaders/gl_material.wgsl",
            Shader::from_wgsl
        );
        app.add_plugins(MaterialPlugin::<GlMaterial>::default());
    }
}

/// `Tessellator.setColorRGBA_F(r, g, b, a)`: each channel `(int)(v * 255)`
/// clamped to 0..255, as the Tessellator stores bytes; returned / 255.
// src: Tessellator.java:171-173, :179-212
pub fn tess_color(r: f32, g: f32, b: f32, a: f32) -> [f32; 4] {
    let q = |v: f32| ((v * 255.0) as i32).clamp(0, 255) as f32 / 255.0;
    [q(r), q(g), q(b), q(a)]
}

/// A Tessellator batch: positions, texture coordinates and colours per
/// vertex, as `addVertexWithUV` emits them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TessBatch {
    /// Positions.
    pub positions: Vec<[f32; 3]>,
    /// Texture coordinates.
    pub uvs: Vec<[f32; 2]>,
    /// Colours (`[1; 4]` where the Java sets none and `glColor` applies).
    pub colors: Vec<[f32; 4]>,
}

impl TessBatch {
    /// `addVertexWithUV(x, y, z, u, v)` with the current colour `c`.
    pub fn vertex(&mut self, p: [f32; 3], uv: [f32; 2], c: [f32; 4]) {
        self.positions.push(p);
        self.uvs.push(uv);
        self.colors.push(c);
    }

    /// The vertex count.
    pub fn len(&self) -> usize {
        self.positions.len()
    }

    /// Whether no vertex was added.
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    /// Indices drawing every 4 vertices as a `GL_QUADS` quad (two triangles,
    /// as the Tessellator's `convertQuadsToTriangles` does).
    // src: Tessellator.java:112-113
    pub fn quad_indices(&self) -> Vec<u32> {
        let n = self.positions.len() as u32 / 4;
        let mut idx = Vec::with_capacity(n as usize * 6);
        for q in 0..n {
            let b = q * 4;
            idx.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
        }
        idx
    }

    /// Writes the batch into `mesh` with `indices`.
    pub fn write_into(&self, mesh: &mut Mesh, indices: Vec<u32>) {
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors.clone());
        mesh.insert_indices(Indices::U32(indices));
    }

    /// A mesh to fill later: one degenerate quad (Bevy's mesh allocator
    /// rejects empty meshes).
    pub fn placeholder_mesh() -> Mesh {
        let mut b = TessBatch::default();
        for _ in 0..4 {
            b.vertex([0.0; 3], [0.0; 2], [0.0; 4]);
        }
        b.quads_mesh()
    }

    /// A new triangle-list mesh of the batch drawn as quads.
    pub fn quads_mesh(&self) -> Mesh {
        let mut m = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        self.write_into(&mut m, self.quad_indices());
        m
    }
}

/// A wide-line mesh ([`GlKey::wide_lines`]) of `segments`: per segment
/// `(a, b)` four vertices `a-, a+, b+, b-` (position = this end, normal = the
/// other end, `uv.x` = the side ∓1) and two triangles; the shader moves each
/// vertex half the line width off the projected segment along its minor
/// screen axis, as GL rasterises aliased wide lines (a `width`-pixel column
/// for an x-major line, a row for a y-major one).
pub fn wide_line_mesh(segments: &[([f32; 3], [f32; 3])]) -> Mesh {
    let n = segments.len();
    let mut pos = Vec::with_capacity(n * 4);
    let mut other = Vec::with_capacity(n * 4);
    let mut uv = Vec::with_capacity(n * 4);
    let mut idx = Vec::with_capacity(n * 6);
    for (i, &(a, b)) in segments.iter().enumerate() {
        for (p, o, side) in [(a, b, -1.0f32), (a, b, 1.0), (b, a, 1.0), (b, a, -1.0)] {
            pos.push(p);
            other.push(o);
            uv.push([side, 0.0]);
        }
        let q = i as u32 * 4;
        idx.extend_from_slice(&[q, q + 1, q + 2, q, q + 2, q + 3]);
    }
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, other);
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
    m.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![[1.0f32; 4]; n * 4]);
    m.insert_indices(Indices::U32(idx));
    m
}

/// A jar texture as `RenderEngine.getTexture` sets it up: nearest filtering
/// (`useMipmaps` is false), `GL_REPEAT` unless `clamp` (vanilla's
/// `%clamp%` prefix), uploaded as `Rgba8Unorm` (module docs, "Colours").
// src: RenderEngine.java:162-186
pub fn jar_texture(jar: &JarAssets, name: &str, clamp: bool) -> Result<Image, AssetError> {
    let mut img = jar.bevy_image(name)?;
    img.texture_descriptor.format = TextureFormat::Rgba8Unorm;
    let mode = if clamp {
        ImageAddressMode::ClampToEdge
    } else {
        ImageAddressMode::Repeat
    };
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: mode,
        address_mode_v: mode,
        address_mode_w: mode,
        mag_filter: ImageFilterMode::Nearest,
        min_filter: ImageFilterMode::Nearest,
        mipmap_filter: ImageFilterMode::Nearest,
        ..Default::default()
    });
    Ok(img)
}

/// The anaglyph colour transform vanilla applies to every colour it sets
/// with anaglyph 3D on: `(r·30 + g·59 + b·11)/100, (r·30 + g·70)/100,
/// (r·30 + b·70)/100`.
// src: RenderGlobal.java:609-616, EntityRenderer.java:842-849
pub fn anaglyph(c: [f32; 3]) -> [f32; 3] {
    let [r, g, b] = c;
    [
        (r * 30.0 + g * 59.0 + b * 11.0) / 100.0,
        (r * 30.0 + g * 70.0) / 100.0,
        (r * 30.0 + b * 70.0) / 100.0,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tessellator_colours_truncate_to_bytes() {
        assert_eq!(
            tess_color(1.0, 0.5, 0.0, 0.8),
            [1.0, 127.0 / 255.0, 0.0, 204.0 / 255.0]
        );
        assert_eq!(
            tess_color(1.5, -0.2, 0.999, 2.0),
            [1.0, 0.0, 254.0 / 255.0, 1.0]
        );
    }

    #[test]
    fn quads_become_two_triangles() {
        let mut b = TessBatch::default();
        for i in 0..8 {
            b.vertex([i as f32, 0.0, 0.0], [0.0, 0.0], [1.0; 4]);
        }
        assert_eq!(b.quad_indices(), vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
    }

    #[test]
    fn wide_lines_are_quads_with_the_other_end_and_side() {
        let m = wide_line_mesh(&[([0.0, 0.0, 0.0], [1.0, 2.0, 3.0])]);
        let pos = m.attribute(Mesh::ATTRIBUTE_POSITION).unwrap().as_float3();
        let other = m.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap().as_float3();
        assert_eq!(
            pos.unwrap(),
            &[[0.0; 3], [0.0; 3], [1.0, 2.0, 3.0], [1.0, 2.0, 3.0]]
        );
        assert_eq!(
            other.unwrap(),
            &[[1.0, 2.0, 3.0], [1.0, 2.0, 3.0], [0.0; 3], [0.0; 3]]
        );
        let Some(bevy::mesh::VertexAttributeValues::Float32x2(uv)) =
            m.attribute(Mesh::ATTRIBUTE_UV_0)
        else {
            panic!("uv");
        };
        let sides: Vec<f32> = uv.iter().map(|u| u[0]).collect();
        assert_eq!(sides, vec![-1.0, 1.0, 1.0, -1.0]);
        let Some(Indices::U32(idx)) = m.indices() else {
            panic!("indices");
        };
        assert_eq!(idx, &vec![0, 1, 2, 0, 2, 3]);
    }

    #[test]
    fn anaglyph_weights() {
        let a = anaglyph([1.0, 0.5, 0.25]);
        assert_eq!(
            a,
            [
                (30.0 + 29.5 + 2.75) / 100.0,
                (30.0 + 35.0) / 100.0,
                (30.0 + 17.5) / 100.0
            ]
        );
    }
}
