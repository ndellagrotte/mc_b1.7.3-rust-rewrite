//! The GUI's Bevy side (project-owned; PLAN_v4 D7): the batches of
//! [`GuiGl`] drawn by two orthographic cameras after the world camera, one
//! per [`GuiLayer`] (each clears the depth buffer, as vanilla does before
//! the in-game overlay and before the screen), with [`GuiMaterial`].
//!
//! - **Projection:** `setupOverlayRendering`'s `glOrtho(0, scaledWidthD,
//!   scaledHeightD, 0, 1000, 3000)` after `glTranslatef(0, 0, -2000)`: the
//!   camera sits at z 2000 looking down -z with near 1000 and far 3000 over
//!   the area `0..scaledWidthD × -scaledHeightD..0`, and the meshes are
//!   built with y negated. A scaled pixel is `scaleFactor` window pixels.
//! - **Order:** each batch is one mesh; batches are drawn in call order
//!   (the `Transparent3d` phase sorted by [`GuiMaterial::sort_bias`] = the
//!   batch's index), with GL's depth test (`GL_LEQUAL`) and mask per batch.
//! - **Colours and blending:** [`GuiMaterial`] runs WP-3.4's shader
//!   (`gl_material.wgsl`: the Java colour values, the alpha test, converted
//!   to linear for the sRGB target) with any `glBlendFunc` pair. The
//!   caveat of [`crate::gl_material`] applies: opaque pixels are vanilla's
//!   bytes; blends with `0 < alpha < 1` (the dark gradient over the world,
//!   `drawRect` with alpha, the list shadows) happen in linear space, so a
//!   dimmed world is lighter than vanilla's; GuiIngame's crosshair
//!   inversion (`glBlendFunc(775, 769)`) inverts linear values. The
//!   framebuffer's alpha is never changed ([`blend_state`]): Bevy
//!   composites by it, GL's was never seen.
//! - **Textures:** a batch names a texture through
//!   [`crate::textures::Textures`] (`getTexture` by name, or a handle such as
//!   the font's); the GUI samples an `Rgba8Unorm` copy of that image
//!   ([`GuiTextures`]), since the shader wants the PNG's bytes and the
//!   engine's images are sRGB. The copy follows the engine's image when it
//!   changes (the animated atlases, `refreshTextures`).
//! - **Meshes** are pooled per layer and rewritten only when a batch
//!   changed.

use std::collections::BTreeMap;

use bevy::asset::AssetEvent;
use bevy::camera::visibility::{NoFrustumCulling, RenderLayers};
use bevy::camera::{ClearColorConfig, ScalingMode};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, ColorWrites,
    CompareFunction, Face, RenderPipelineDescriptor, SpecializedMeshPipelineError, TextureFormat,
};
use bevy::shader::ShaderRef;

use super::batch::{GuiDraw, GuiGl, GuiKey, GuiLayer, TexRef, gl};
use super::scaled::{ScaledResolution, ortho_size};
use crate::game::Game;
use crate::gl_material::{GL_MATERIAL_SHADER, GlParams, TessBatch};
use crate::textures::TextureLoader;

/// The render layer of the first GUI camera (the overlay); the screen's is
/// the next one. Nothing else uses these layers.
pub const GUI_RENDER_LAYER: usize = 30;

/// The GUI cameras' order after the world camera (order 0).
const GUI_CAMERA_ORDER: isize = 1;

/// A GUI batch's GL state as a material (module docs).
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug, PartialEq)]
#[bind_group_data(GuiKey)]
pub struct GuiMaterial {
    /// `glColor` (always white: the batcher bakes it into the vertices),
    /// the alpha test, texturing.
    #[uniform(0)]
    pub params: GlParams,
    /// The bound texture's `Rgba8Unorm` copy.
    #[texture(1)]
    #[sampler(2)]
    pub texture: Option<Handle<Image>>,
    /// Blend and depth.
    pub key: GuiKey,
    /// The batch's index in its layer (call order).
    pub sort_bias: f32,
}

impl From<&GuiMaterial> for GuiKey {
    fn from(m: &GuiMaterial) -> Self {
        m.key
    }
}

/// `glBlendFunc`'s factor as wgpu's colour factor.
fn blend_factor(f: u32) -> BlendFactor {
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

/// The destination's alpha, kept: vanilla's framebuffer alpha is never
/// seen, but Bevy composites a camera's output by it, so a GUI draw with
/// `0 < alpha < 1` would let the window behind show through (measured:
/// `GuiSlot`'s shadows turned sky-blue).
const KEEP_ALPHA: BlendComponent = BlendComponent {
    src_factor: BlendFactor::Zero,
    dst_factor: BlendFactor::One,
    operation: BlendOperation::Add,
};

/// The wgpu blend state of `glBlendFunc(src, dst)` with `GL_BLEND` on, or
/// of blending off (`None`): the colour as GL computes it, the
/// destination alpha kept ([`KEEP_ALPHA`]).
pub fn blend_state(blend: Option<(u32, u32)>) -> BlendState {
    let color = match blend {
        Some((src, dst)) => BlendComponent {
            src_factor: blend_factor(src),
            dst_factor: blend_factor(dst),
            operation: BlendOperation::Add,
        },
        None => BlendComponent::REPLACE,
    };
    BlendState {
        color,
        alpha: KEEP_ALPHA,
    }
}

impl Material for GuiMaterial {
    fn vertex_shader() -> ShaderRef {
        ShaderRef::Handle(GL_MATERIAL_SHADER)
    }

    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(GL_MATERIAL_SHADER)
    }

    fn alpha_mode(&self) -> AlphaMode {
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
                t.blend = Some(blend_state(k.blend));
                t.write_mask = ColorWrites::ALL;
            }
        }
        descriptor.primitive.cull_mode = None::<Face>;
        if let Some(ds) = descriptor.depth_stencil.as_mut() {
            ds.depth_write_enabled = Some(k.depth_write);
            // GL_LEQUAL under Bevy's reverse-Z.
            ds.depth_compare = Some(if k.depth_test {
                CompareFunction::GreaterEqual
            } else {
                CompareFunction::Always
            });
        }
        Ok(())
    }
}

/// Marks a GUI camera.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuiCamera(pub GuiLayer);

/// One pooled batch mesh.
#[derive(Debug)]
struct PoolEntry {
    entity: Entity,
    mesh: Handle<Mesh>,
    material: Handle<GuiMaterial>,
    /// What the mesh holds now.
    batch: Option<TessBatch>,
}

/// The pooled meshes of each layer.
#[derive(Resource, Debug, Default)]
pub struct GuiMeshPool {
    layers: BTreeMap<GuiLayer, Vec<PoolEntry>>,
}

/// The `Rgba8Unorm` copies of the engine's images the GUI samples (module
/// docs), by the engine image's id.
#[derive(Resource, Debug, Default)]
pub struct GuiTextures {
    copies: BTreeMap<AssetId<Image>, Handle<Image>>,
}

impl GuiTextures {
    /// The copy of `src`, made now if needed.
    pub fn copy_of(
        &mut self,
        src: &Handle<Image>,
        images: &mut Assets<Image>,
    ) -> Option<Handle<Image>> {
        if let Some(h) = self.copies.get(&src.id()) {
            return Some(h.clone());
        }
        let img = unorm_copy(images.get(src)?);
        let h = images.add(img);
        self.copies.insert(src.id(), h.clone());
        Some(h)
    }

    /// Writes the copy of `src` again (the engine's image changed).
    pub fn refresh(&mut self, src: AssetId<Image>, images: &mut Assets<Image>) {
        let Some(h) = self.copies.get(&src).cloned() else {
            return;
        };
        if let Some(img) = images.get(src).map(unorm_copy) {
            let _ = images.insert(&h, img);
        }
    }
}

/// An image's pixels re-labelled `Rgba8Unorm` (sampling returns the bytes /
/// 255, as GL does).
fn unorm_copy(src: &Image) -> Image {
    let mut img = src.clone();
    img.texture_descriptor.format = TextureFormat::Rgba8Unorm;
    img.texture_descriptor.view_formats = &[];
    img
}

/// Spawns the two GUI cameras (after the world camera; module docs).
pub fn spawn_gui_cameras(mut commands: Commands) {
    for (i, layer) in GuiLayer::ALL.into_iter().enumerate() {
        commands.spawn((
            Camera3d::default(),
            Camera {
                order: GUI_CAMERA_ORDER + i as isize,
                clear_color: ClearColorConfig::None,
                ..default()
            },
            Projection::Orthographic(OrthographicProjection {
                near: 1000.0,
                far: 3000.0,
                viewport_origin: Vec2::new(0.0, 1.0),
                scaling_mode: ScalingMode::Fixed {
                    width: 427.0,
                    height: 240.0,
                },
                ..OrthographicProjection::default_3d()
            }),
            Transform::from_xyz(0.0, 0.0, 2000.0),
            RenderLayers::layer(GUI_RENDER_LAYER + i),
            GuiCamera(layer),
        ));
    }
}

/// The GUI batches into the pooled meshes, and the cameras' projection
/// from this frame's `ScaledResolution` (module docs).
#[allow(clippy::too_many_arguments)]
pub fn flush_gui(
    mut commands: Commands,
    game: NonSend<Game>,
    mut gl_: ResMut<GuiGl>,
    mut pool: ResMut<GuiMeshPool>,
    mut copies: ResMut<GuiTextures>,
    mut loader: TextureLoader,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GuiMaterial>>,
    mut image_events: MessageReader<AssetEvent<Image>>,
    mut cams: Query<(&GuiCamera, &mut Projection)>,
    mut vis: Query<&mut Visibility>,
) {
    let r = ScaledResolution::new(
        game.game_settings.gui_scale,
        game.display_width,
        game.display_height,
    );
    let (w, h) = ortho_size(&r);
    for (_, mut p) in &mut cams {
        if let Projection::Orthographic(o) = &mut *p {
            let same = matches!(
                o.scaling_mode,
                ScalingMode::Fixed { width, height } if width == w && height == h
            );
            if !same {
                o.scaling_mode = ScalingMode::Fixed {
                    width: w,
                    height: h,
                };
            }
        }
    }
    let changed: Vec<AssetId<Image>> = image_events
        .read()
        .filter_map(|e| match e {
            AssetEvent::Modified { id } => Some(*id),
            _ => None,
        })
        .collect();
    for id in changed {
        copies.refresh(id, &mut loader.images);
    }
    let draws = gl_.take_draws();
    for (i, layer) in GuiLayer::ALL.into_iter().enumerate() {
        let entries = pool.layers.entry(layer).or_default();
        let mine: Vec<&GuiDraw> = draws.iter().filter(|d| d.layer == layer).collect();
        for (n, d) in mine.iter().enumerate() {
            if n == entries.len() {
                let mesh = meshes.add(TessBatch::placeholder_mesh());
                let material = materials.add(GuiMaterial {
                    params: GlParams::default(),
                    texture: None,
                    key: GuiKey::default(),
                    sort_bias: 0.0,
                });
                let entity = commands
                    .spawn((
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(material.clone()),
                        Transform::IDENTITY,
                        Visibility::Hidden,
                        NoFrustumCulling,
                        RenderLayers::layer(GUI_RENDER_LAYER + i),
                    ))
                    .id();
                entries.push(PoolEntry {
                    entity,
                    mesh,
                    material,
                    batch: None,
                });
            }
            let e = &mut entries[n];
            let texture = match &d.texture {
                None => None,
                Some(t) => {
                    let src = match t {
                        TexRef::Name(name) => loader.get(name),
                        TexRef::Handle(h) => h.clone(),
                    };
                    copies.copy_of(&src, &mut loader.images)
                }
            };
            let want = GuiMaterial {
                params: GlParams {
                    alpha_ref: d.alpha_ref.unwrap_or(-1.0),
                    textured: texture.is_some() as u32,
                    ..GlParams::default()
                },
                texture,
                key: d.key,
                sort_bias: n as f32,
            };
            if materials.get(&e.material) != Some(&want)
                && let Some(mut m) = materials.get_mut(&e.material)
            {
                *m = want;
            }
            if e.batch.as_ref() != Some(&d.batch) {
                let flipped = flip_y(&d.batch);
                let _ = meshes.insert(&e.mesh, flipped.quads_mesh());
                e.batch = Some(d.batch.clone());
            }
            if let Ok(mut v) = vis.get_mut(e.entity) {
                v.set_if_neq(Visibility::Inherited);
            }
        }
        for e in entries.iter().skip(mine.len()) {
            if let Ok(mut v) = vis.get_mut(e.entity) {
                v.set_if_neq(Visibility::Hidden);
            }
        }
    }
}

/// The batch with y negated (module docs, "Projection").
fn flip_y(b: &TessBatch) -> TessBatch {
    let mut out = b.clone();
    for p in &mut out.positions {
        p[1] = -p[1];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_pairs() {
        let a = blend_state(Some((gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA)));
        assert_eq!(a.color, BlendState::ALPHA_BLENDING.color);
        assert_eq!(a.alpha, KEEP_ALPHA);
        // The crosshair's inversion (GuiIngame.java:56).
        let inv = blend_state(Some((gl::ONE_MINUS_DST_COLOR, gl::ONE_MINUS_SRC_COLOR)));
        assert_eq!(inv.color.src_factor, BlendFactor::OneMinusDst);
        assert_eq!(inv.color.dst_factor, BlendFactor::OneMinusSrc);
        assert_eq!(blend_state(None).color, BlendComponent::REPLACE);
    }

    #[test]
    fn flip() {
        let mut b = TessBatch::default();
        b.vertex([1.0, 2.0, 3.0], [0.0; 2], [1.0; 4]);
        assert_eq!(flip_y(&b).positions[0], [1.0, -2.0, 3.0]);
    }
}
