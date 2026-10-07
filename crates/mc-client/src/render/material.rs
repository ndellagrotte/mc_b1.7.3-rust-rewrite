//! [`EntityGlMaterial`]: the GL state the entity renderers' Tessellator
//! draws are made with (project-owned; PLAN_v4 D7). The vertices arrive
//! already transformed (the matrix stack of [`super::gl::EntityGl`]) and
//! coloured (the Tessellator's colour or `glColor`, lit on the CPU by GL's
//! fixed-function formula when `GL_LIGHTING` was on), so the material only
//! keeps what GL does per fragment: the texture (the shared cache's sRGB
//! images, re-encoded to the PNG's values as [`crate::model::ModelMaterial`]
//! does), the alpha test, the fog, any `glBlendFunc` pair, the depth test,
//! mask and function, and face culling.
//!
//! Colours and blending follow [`crate::gl_material`] (its module docs,
//! "Colours"): opaque pixels are vanilla's bytes on the sRGB target, blends
//! with `0 < alpha < 1` happen in linear space (the shadows are lighter than
//! GL's), additive `(770, 1)` draws are exact over black.

use bevy::asset::{load_internal_asset, uuid_handle};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, ColorWrites,
    CompareFunction, Face, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
    TextureFormat,
};
use bevy::shader::{Shader, ShaderRef};

use crate::gui::batch::gl;

/// The shader (`render/entity_gl.wgsl`, embedded).
pub const ENTITY_GL_SHADER: Handle<Shader> = uuid_handle!("3f2c8d41-7b6a-4e19-a2d5-90c4e6f1b873");

/// The pipeline half of the state (the material's specialisation key).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EntityGlKey {
    /// `glBlendFunc(src, dst)` with `GL_BLEND` on; `None`: off.
    pub blend: Option<(u32, u32)>,
    /// `GL_DEPTH_TEST`.
    pub depth_test: bool,
    /// `glDepthMask`.
    pub depth_write: bool,
    /// `glDepthFunc(GL_EQUAL)` (else `GL_LEQUAL`).
    pub depth_equal: bool,
    /// `GL_CULL_FACE` (back faces; counter-clockwise front).
    pub cull: bool,
}

impl Default for EntityGlKey {
    fn default() -> Self {
        EntityGlKey {
            blend: None,
            depth_test: true,
            depth_write: true,
            depth_equal: false,
            cull: true,
        }
    }
}

/// The uniform half (matches `EntityGlParams` in the shader).
#[derive(Clone, Copy, Debug, PartialEq, ShaderType)]
pub struct EntityGlParams {
    /// `GL_FOG_COLOR`.
    pub fog_color: Vec4,
    /// Mode (0 off, 1 linear, 2 exp), start, end, density.
    pub fog: Vec4,
    /// The alpha test's reference (`GL_GREATER`); negative: off.
    pub alpha_ref: f32,
    /// 1 when `GL_TEXTURE_2D` is on.
    pub textured: u32,
    /// 1 when the image is sRGB (the shared texture cache).
    pub texture_srgb: u32,
    /// Alignment.
    pub pad: u32,
}

impl Default for EntityGlParams {
    fn default() -> Self {
        EntityGlParams {
            fog_color: Vec4::ONE,
            fog: Vec4::ZERO,
            alpha_ref: -1.0,
            textured: 0,
            texture_srgb: 1,
            pad: 0,
        }
    }
}

/// A Tessellator draw's GL state (module docs).
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug, PartialEq)]
#[bind_group_data(EntityGlKey)]
pub struct EntityGlMaterial {
    /// Fog, alpha test, texturing.
    #[uniform(0)]
    pub params: EntityGlParams,
    /// The bound texture.
    #[texture(1)]
    #[sampler(2)]
    pub texture: Option<Handle<Image>>,
    /// Blend, depth, culling.
    pub key: EntityGlKey,
    /// The draw-order bias (`super::order`).
    pub sort_bias: f32,
}

impl From<&EntityGlMaterial> for EntityGlKey {
    fn from(m: &EntityGlMaterial) -> Self {
        m.key
    }
}

/// `glBlendFunc`'s factor as wgpu's.
pub fn blend_factor(f: u32) -> BlendFactor {
    match f {
        gl::ZERO => BlendFactor::Zero,
        gl::ONE => BlendFactor::One,
        gl::SRC_COLOR => BlendFactor::Src,
        gl::ONE_MINUS_SRC_COLOR => BlendFactor::OneMinusSrc,
        gl::SRC_ALPHA => BlendFactor::SrcAlpha,
        gl::ONE_MINUS_SRC_ALPHA => BlendFactor::OneMinusSrcAlpha,
        gl::DST_COLOR => BlendFactor::Dst,
        gl::ONE_MINUS_DST_COLOR => BlendFactor::OneMinusDst,
        gl::DST_ALPHA => BlendFactor::DstAlpha,
        gl::ONE_MINUS_DST_ALPHA => BlendFactor::OneMinusDstAlpha,
        _ => BlendFactor::One,
    }
}

impl Material for EntityGlMaterial {
    fn vertex_shader() -> ShaderRef {
        ShaderRef::Handle(ENTITY_GL_SHADER)
    }

    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(ENTITY_GL_SHADER)
    }

    fn alpha_mode(&self) -> AlphaMode {
        // Transparent3d, sorted by the bias (draw order); the blend state
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
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(2),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        if let Some(frag) = descriptor.fragment.as_mut() {
            let unorm = frag.targets.iter().flatten().any(|t| {
                matches!(
                    t.format,
                    TextureFormat::Rgba8Unorm | TextureFormat::Bgra8Unorm
                )
            });
            if unorm {
                frag.shader_defs.push("GL_TARGET_UNORM".into());
            } else if k.blend == Some((gl::SRC_ALPHA, gl::ONE)) {
                frag.shader_defs.push("GL_BLEND_ADDITIVE".into());
            }
            for t in frag.targets.iter_mut().flatten() {
                t.blend = k.blend.map(|(src, dst)| {
                    let c = BlendComponent {
                        src_factor: blend_factor(src),
                        dst_factor: blend_factor(dst),
                        operation: BlendOperation::Add,
                    };
                    BlendState { color: c, alpha: c }
                });
                t.write_mask = ColorWrites::ALL;
            }
        }
        descriptor.primitive.cull_mode = if k.cull { Some(Face::Back) } else { None };
        if let Some(ds) = descriptor.depth_stencil.as_mut() {
            // GL writes depth only with the test on.
            ds.depth_write_enabled = Some(k.depth_write && k.depth_test);
            // GL_LEQUAL / GL_EQUAL; Bevy's depth is reverse-Z.
            ds.depth_compare = Some(if !k.depth_test {
                CompareFunction::Always
            } else if k.depth_equal {
                CompareFunction::Equal
            } else {
                CompareFunction::GreaterEqual
            });
        }
        Ok(())
    }
}

/// Registers [`EntityGlMaterial`] and its shader.
pub struct EntityGlMaterialPlugin;

impl Plugin for EntityGlMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, ENTITY_GL_SHADER, "entity_gl.wgsl", Shader::from_wgsl);
        app.add_plugins(MaterialPlugin::<EntityGlMaterial>::default());
    }
}
