//! The terrain's material (project-owned, PLAN_v4 D7): the GL state of
//! `RenderGlobal.sortAndRender`'s two passes and of `drawBlockBreaking`,
//! on WP-3.4's [`GlParams`] (glColor, fog, alpha test) and the same colour
//! rules as [`crate::gl_material`] (module docs there, "Colours"), with
//! three differences that keep it from being a [`crate::gl_material::GlMaterial`]:
//!
//! - pass 0 is drawn in Bevy's opaque phase (depth-tested, written, back
//!   faces culled: `GL_CULL_FACE` is on, EntityRenderer.java:433, :467),
//!   so the sky (drawn after it at the far plane) and everything blended
//!   lands behind or over it as in vanilla;
//! - `terrain.png` comes from WP-3.2's cache, which stores sRGB images: the
//!   shader re-encodes a sample to the PNG's byte / 255 (exact for 8-bit
//!   texels, the sampler is nearest);
//! - the crack overlay blends `glBlendFunc(GL_DST_COLOR, GL_SRC_COLOR)` with
//!   `glPolygonOffset(-3, -3)` (RenderGlobal.java:1077, :1085-1086).
//!
//! The vertex colour is the Tessellator's bytes
//! ([`super::tessellator::ATTRIBUTE_TESS_COLOR`]).

use bevy::asset::{load_internal_asset, uuid_handle};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, ColorWrites,
    CompareFunction, DepthBiasState, Face, RenderPipelineDescriptor, SpecializedMeshPipelineError,
    TextureFormat,
};
use bevy::shader::{Shader, ShaderRef};

use super::tessellator::ATTRIBUTE_TESS_COLOR;
use crate::gl_material::GlParams;

/// The shader (`terrain/terrain.wgsl`, embedded).
pub const TERRAIN_SHADER: Handle<Shader> = uuid_handle!("2b7d43a0-5d6e-4a51-8c09-7e2f1d3c9b84");

/// Which draw of the terrain a material is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TerrainPhase {
    /// Pass 0: opaque, alpha-tested, culled.
    // src: EntityRenderer.java:494-498
    Opaque,
    /// Pass 1 with fancy graphics, first draw: depth only
    /// (`glColorMask(false, …)`), no culling.
    // src: EntityRenderer.java:516-526
    TranslucentDepth,
    /// Pass 1: blended `(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA)`, no culling,
    /// depth written.
    // src: EntityRenderer.java:515-545
    Translucent,
    /// The crack overlay.
    // src: RenderGlobal.java:1068-1107
    Crack,
}

/// The material's pipeline key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TerrainKey {
    /// The draw.
    pub phase: TerrainPhase,
    /// The texture is sRGB (WP-3.2's cache): re-encode samples.
    pub srgb_texture: bool,
}

/// The terrain material (module docs).
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
#[bind_group_data(TerrainKey)]
pub struct TerrainMaterial {
    /// glColor, fog, alpha test, texturing.
    #[uniform(0)]
    pub params: GlParams,
    /// `terrain.png`.
    #[texture(1)]
    #[sampler(2)]
    pub texture: Option<Handle<Image>>,
    /// The pipeline state.
    pub key: TerrainKey,
    /// Sort bias in the transparent phase (crack, pass 1).
    pub sort_bias: f32,
}

impl From<&TerrainMaterial> for TerrainKey {
    fn from(m: &TerrainMaterial) -> Self {
        m.key
    }
}

impl TerrainMaterial {
    /// A material for `phase` with `terrain.png` bound, `glColor(1, 1, 1,
    /// 1)` and `glAlphaFunc(GL_GREATER, 0.1F)` (Minecraft.java:266-267).
    pub fn new(phase: TerrainPhase, texture: Option<Handle<Image>>, sort_bias: f32) -> Self {
        TerrainMaterial {
            params: GlParams {
                textured: u32::from(texture.is_some()),
                alpha_ref: 0.1,
                ..GlParams::default()
            },
            texture,
            key: TerrainKey {
                phase,
                srgb_texture: true,
            },
            sort_bias,
        }
    }
}

impl Material for TerrainMaterial {
    fn vertex_shader() -> ShaderRef {
        ShaderRef::Handle(TERRAIN_SHADER)
    }

    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(TERRAIN_SHADER)
    }

    fn alpha_mode(&self) -> AlphaMode {
        match self.key.phase {
            TerrainPhase::Opaque => AlphaMode::Opaque,
            _ => AlphaMode::Blend,
        }
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
            ATTRIBUTE_TESS_COLOR.at_shader_location(2),
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
            }
            if k.srgb_texture {
                frag.shader_defs.push("TERRAIN_SRGB_TEXTURE".into());
            }
            if k.phase == TerrainPhase::Crack {
                frag.shader_defs.push("TERRAIN_CRACK".into());
            }
        }
        descriptor.primitive.cull_mode = match k.phase {
            TerrainPhase::Opaque | TerrainPhase::Crack => Some(Face::Back),
            TerrainPhase::TranslucentDepth | TerrainPhase::Translucent => None,
        };
        if let Some(ds) = descriptor.depth_stencil.as_mut() {
            ds.depth_write_enabled = Some(true);
            // glDepthFunc(GL_LEQUAL) (Minecraft.java:265), reverse-Z.
            ds.depth_compare = Some(CompareFunction::GreaterEqual);
            if k.phase == TerrainPhase::Crack {
                // glPolygonOffset(-3.0F, -3.0F): toward the eye, which is a
                // larger depth with reverse-Z.
                ds.bias = DepthBiasState {
                    constant: 3,
                    slope_scale: 3.0,
                    clamp: 0.0,
                };
            }
        }
        if let Some(frag) = descriptor.fragment.as_mut() {
            for t in frag.targets.iter_mut().flatten() {
                t.blend = match k.phase {
                    TerrainPhase::Opaque | TerrainPhase::TranslucentDepth => None,
                    TerrainPhase::Translucent => Some(BlendState::ALPHA_BLENDING),
                    TerrainPhase::Crack => Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::Dst,
                            dst_factor: BlendFactor::Src,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::DstAlpha,
                            dst_factor: BlendFactor::SrcAlpha,
                            operation: BlendOperation::Add,
                        },
                    }),
                };
                t.write_mask = if k.phase == TerrainPhase::TranslucentDepth {
                    ColorWrites::empty()
                } else {
                    ColorWrites::ALL
                };
            }
        }
        Ok(())
    }
}

/// Registers [`TerrainMaterial`] and its shader.
pub struct TerrainMaterialPlugin;

impl Plugin for TerrainMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, TERRAIN_SHADER, "terrain.wgsl", Shader::from_wgsl);
        app.add_plugins(MaterialPlugin::<TerrainMaterial>::default());
    }
}
