//! [`ModelMaterial`]: the GL state entity models are drawn with
//! (project-owned; PLAN_v4 D7). It is [`crate::gl_material::GlMaterial`]'s
//! arithmetic (glColor × texture, `glAlphaFunc(GL_GREATER)`, GL fog, the
//! colour conversion at the end; see that module's docs for the sRGB
//! caveats) plus what `GlMaterial` lacks and every model draw has on:
//! fixed-function lighting. `EntityRenderer.renderWorld` turns on
//! `RenderHelper.enableStandardItemLighting()` around `renderEntities`
//! (EntityRenderer.java:500-503): two directional lights fixed in the world
//! (their `GL_POSITION` is set under the camera's modelview), diffuse 0.6,
//! no light ambient, scene ambient 0.4, `GL_COLOR_MATERIAL` on
//! `GL_AMBIENT_AND_DIFFUSE`, so a face shows `clamp(c·0.4 + c·0.6·Σ max(n·L,
//! 0))` of the current colour `c` (`glColor3f(brightness)`,
//! RenderManager.java:107-108), times the texture.
//!
//! Also here: `GL_TEXTURE` matrix translation ([`ModelParams::uv_offset`],
//! the powered creeper's moving armour), `glDepthFunc(GL_EQUAL)`
//! ([`ModelKey::depth_equal`], the hurt and creeper-flash overlays), face
//! culling ([`ModelKey::cull`]: `RenderLiving` turns `GL_CULL_FACE` off,
//! minecarts and boats keep it on).
//!
//! Approximation: `GL_RESCALE_NORMAL` rescales the transformed normal by a
//! uniform factor; the shader normalises it (Bevy's
//! `mesh_normal_local_to_world`), which differs only under the non-uniform
//! `glScalef`s (the ghast's, a squished slime's) where GL's normal is not
//! unit length.

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
use mc_core::math::sqrt_double;

use crate::gl_material::GlBlend;

/// The shader (`model/model_material.wgsl`, embedded).
pub const MODEL_MATERIAL_SHADER: Handle<Shader> =
    uuid_handle!("b4f1c0d2-6a7e-4d3b-9e25-58c1a7f0e3b9");

/// `RenderHelper.enableStandardItemLighting`'s light directions,
/// `Vec3D.createVector(±0.2, 1.0, ∓0.7).normalize()` (the doubles of the
/// floats, `MathHelper.sqrt_double`), cast to float as `glLight` takes
/// them: `GL_LIGHT0` first.
// src: RenderHelper.java:25-34
pub fn standard_light_directions() -> [Vec3; 2] {
    let n = |x: f64, y: f64, z: f64| {
        let l = sqrt_double(x * x + y * y + z * z) as f64;
        if l < 1.0e-4 {
            Vec3::ZERO
        } else {
            Vec3::new((x / l) as f32, (y / l) as f32, (z / l) as f32)
        }
    };
    [
        n(0.20000000298023224, 1.0, -0.699999988079071),
        n(-0.20000000298023224, 1.0, 0.699999988079071),
    ]
}

/// The pipeline half of the state (the material's specialisation key).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ModelKey {
    /// Blending (`GlBlend::Off`: drawn in the opaque phase).
    pub blend: GlBlend,
    /// `glDepthMask`.
    pub depth_write: bool,
    /// `glDepthFunc(GL_EQUAL)` (else `GL_LEQUAL`).
    pub depth_equal: bool,
    /// `GL_CULL_FACE` (back faces, counter-clockwise front).
    pub cull: bool,
    /// With [`GlBlend::Additive`]: the source factor is `GL_ONE`
    /// (`glBlendFunc(1, 1)`, the powered creeper's pass,
    /// RenderCreeper.java:67) instead of `GL_SRC_ALPHA`, so texels are added
    /// unweighted by their alpha. Ignored for the other blends.
    pub src_one: bool,
}

impl Default for ModelKey {
    /// A mob's main pass: opaque, depth writes, `GL_LEQUAL`, no culling
    /// (RenderLiving.java:21).
    fn default() -> Self {
        ModelKey {
            blend: GlBlend::Off,
            depth_write: true,
            depth_equal: false,
            cull: false,
            src_one: false,
        }
    }
}

/// The uniform half of the state (matches `ModelParams` in the shader).
#[derive(Clone, Copy, Debug, PartialEq, ShaderType)]
pub struct ModelParams {
    /// `glColor4f`, in the Java's colour values.
    pub color: Vec4,
    /// `GL_FOG_COLOR`.
    pub fog_color: Vec4,
    /// Mode (0 off, 1 linear, 2 exp), start, end, density
    /// ([`crate::fog::FogParams::to_vec4`]).
    pub fog: Vec4,
    /// `GL_LIGHT0`'s direction (world space, towards the light).
    pub light0: Vec4,
    /// `GL_LIGHT1`'s direction.
    pub light1: Vec4,
    /// `xy`: the `GL_TEXTURE` matrix's translation, added to the UVs.
    pub uv_offset: Vec4,
    /// The alpha test's reference (`GL_GREATER`); negative: off.
    pub alpha_ref: f32,
    /// 1 when `GL_TEXTURE_2D` is on.
    pub textured: u32,
    /// 1 when `GL_LIGHTING` is on.
    pub lighting: u32,
    /// 1 when the image is sRGB (the shared [`crate::textures::Textures`]
    /// cache): samples are re-encoded to the PNG's values.
    pub texture_srgb: u32,
}

impl Default for ModelParams {
    /// White, no fog, the standard lights on, no texture, alpha test off.
    fn default() -> Self {
        let [l0, l1] = standard_light_directions();
        ModelParams {
            color: Vec4::ONE,
            fog_color: Vec4::ONE,
            fog: Vec4::ZERO,
            light0: l0.extend(0.0),
            light1: l1.extend(0.0),
            uv_offset: Vec4::ZERO,
            alpha_ref: -1.0,
            textured: 0,
            lighting: 1,
            texture_srgb: 0,
        }
    }
}

/// A model draw's GL state (module docs).
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
#[bind_group_data(ModelKey)]
pub struct ModelMaterial {
    /// Colour, lights, fog, alpha test, texturing.
    #[uniform(0)]
    pub params: ModelParams,
    /// The bound texture (used when [`ModelParams::textured`] is 1).
    #[texture(1)]
    #[sampler(2)]
    pub texture: Option<Handle<Image>>,
    /// Blend, depth, culling.
    pub key: ModelKey,
    /// The draw-order bias for blended draws
    /// ([`crate::gl_material::order`]; entities and particles sit between
    /// −0.95e6 and −0.91e6).
    pub sort_bias: f32,
}

impl From<&ModelMaterial> for ModelKey {
    fn from(m: &ModelMaterial) -> Self {
        m.key
    }
}

impl ModelMaterial {
    /// A mob's main pass with `texture` (the shared cache's sRGB image):
    /// `glColor3f(brightness)`, lighting, `glEnable(GL_ALPHA_TEST)` with
    /// the client's `glAlphaFunc(GL_GREATER, 0.1F)` (Minecraft.java:267),
    /// cull off.
    // src: RenderLiving.java:21, :51, RenderManager.java:107-108
    pub fn living(texture: Handle<Image>, brightness: f32) -> Self {
        ModelMaterial {
            params: ModelParams {
                color: Vec4::new(brightness, brightness, brightness, 1.0),
                alpha_ref: 0.1,
                textured: 1,
                texture_srgb: 1,
                ..default()
            },
            texture: Some(texture),
            key: ModelKey::default(),
            sort_bias: 0.0,
        }
    }

    /// With `GL_FOG` from [`crate::fog::FogState`]'s world fog.
    pub fn with_fog(mut self, fog: &crate::fog::FogState) -> Self {
        self.set_fog(fog);
        self
    }

    /// Sets `GL_FOG` from [`crate::fog::FogState`]'s world fog (`setupFog(0)`),
    /// or off when no world is drawn.
    pub fn set_fog(&mut self, fog: &crate::fog::FogState) {
        if fog.active {
            self.params.fog = fog.world.to_vec4();
            self.params.fog_color = Vec3::from(fog.color).extend(1.0);
        } else {
            self.params.fog = Vec4::ZERO;
        }
    }
}

impl Material for ModelMaterial {
    fn vertex_shader() -> ShaderRef {
        ShaderRef::Handle(MODEL_MATERIAL_SHADER)
    }

    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(MODEL_MATERIAL_SHADER)
    }

    fn alpha_mode(&self) -> AlphaMode {
        match self.key.blend {
            // The opaque phase; the alpha test discards in the shader.
            GlBlend::Off => AlphaMode::Opaque,
            // Transparent3d, sorted by depth + sort_bias.
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
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(2),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        if let Some(frag) = descriptor.fragment.as_mut() {
            // As GlMaterial: a non-sRGB main target gets the Java values.
            let unorm = frag.targets.iter().flatten().any(|t| {
                matches!(
                    t.format,
                    TextureFormat::Rgba8Unorm | TextureFormat::Bgra8Unorm
                )
            });
            // glBlendFunc(GL_ONE, GL_ONE) adds the colour as it is: the
            // plain conversion, no alpha weighting to undo.
            let src_one = k.blend == GlBlend::Additive && k.src_one;
            if unorm {
                frag.shader_defs.push("GL_TARGET_UNORM".into());
            } else if k.blend == GlBlend::Additive && !src_one {
                frag.shader_defs.push("GL_BLEND_ADDITIVE".into());
            }
            let add_src = if src_one {
                BlendFactor::One
            } else {
                BlendFactor::SrcAlpha
            };
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
                            src_factor: add_src,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: add_src,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                    }),
                };
                t.write_mask = ColorWrites::ALL;
            }
        }
        descriptor.primitive.cull_mode = if k.cull { Some(Face::Back) } else { None };
        if let Some(ds) = descriptor.depth_stencil.as_mut() {
            ds.depth_write_enabled = Some(k.depth_write);
            // GL_LEQUAL / GL_EQUAL; Bevy's depth is reverse-Z.
            ds.depth_compare = Some(if k.depth_equal {
                CompareFunction::Equal
            } else {
                CompareFunction::GreaterEqual
            });
        }
        Ok(())
    }
}

/// Registers [`ModelMaterial`] and its shader.
pub struct ModelMaterialPlugin;

impl Plugin for ModelMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            MODEL_MATERIAL_SHADER,
            "model_material.wgsl",
            Shader::from_wgsl
        );
        app.add_plugins(MaterialPlugin::<ModelMaterial>::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_directions_are_the_normalised_java_vectors() {
        let [a, b] = standard_light_directions();
        let l = (0.2f64 * 0.2 + 1.0 + 0.49).sqrt();
        assert!(
            (a - Vec3::new((0.2 / l) as f32, (1.0 / l) as f32, (-0.7 / l) as f32)).length() < 1e-6
        );
        assert_eq!(b, Vec3::new(-a.x, a.y, -a.z));
        assert!((a.length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn living_pass_defaults() {
        let m = ModelMaterial::living(Handle::default(), 0.8);
        assert_eq!(m.params.color, Vec4::new(0.8, 0.8, 0.8, 1.0));
        assert_eq!(m.params.alpha_ref, 0.1);
        assert_eq!(m.params.lighting, 1);
        assert!(!m.key.cull);
        assert!(m.key.depth_write);
        assert_eq!(m.alpha_mode(), AlphaMode::Opaque);
    }
}
