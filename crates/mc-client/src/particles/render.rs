//! `EffectRenderer.renderParticles` and `renderLitParticles` as Bevy meshes
//! (D7: the quads are the Java's, the drawing is Bevy's).
//!
//! Every frame after the shell's frame, [`compute_particles`] runs
//! `renderWorld`'s two particle calls in their order (EntityRenderer.java:
//! 502-505): `renderLitParticles` (layer 3: the footsteps into a batch, the
//! pickups into [`PickupFxFrame`] for WP-3.5b), then `renderParticles`
//! (layers 0-2, one Tessellator batch each). [`apply_particles`] writes the
//! batches into one mesh per layer, drawn with [`GlMaterial`].
//!
//! # GL state (vanilla)
//!
//! `renderParticles` runs with the state `renderWorld` leaves: depth test
//! and depth writes on, `GL_BLEND` off, the alpha test `GL_GREATER 0.1`
//! (Minecraft.java:267), lighting off (`disableStandardItemLighting`), fog
//! `setupFog(0)` (EntityRenderer.java:504); each layer binds its texture
//! (EffectRenderer.java:62-74). So the particles are alpha-tested cut-outs,
//! not blended. The footstep binds `/misc/footprint.png` and draws blended
//! (770, 771) with lighting off (EntityFootStepFX.java:26-43).
//!
//! # Positions
//!
//! `renderParticles` sets `EntityFX.interpPos` to the view entity's
//! `lastTickPos + (pos - lastTickPos) · partial` and every quad is relative
//! to it; the mesh sits there. `renderLitParticles` runs *before* it in the
//! frame and reads the previous frame's `interpPos`, while the GL origin is
//! already this frame's: the footsteps and the pickup entity are drawn one
//! frame's movement off (vanilla bug, transcribed: the lit mesh sits at
//! this frame's position with last frame's offsets).
//!
//! With anaglyph 3D vanilla runs both calls once per eye
//! (EntityRenderer.java:455-575), so its second eye's lit pass reads this
//! frame's `interpPos`. The shell renders one eye (`camera` module docs):
//! these meshes are vanilla's first pass, whose lag is the one above.
//!
//! # Textures
//!
//! [`GlMaterial`] expects `Rgba8Unorm` textures (its module docs:
//! "Colours"), while `crate::textures` uploads `Rgba8UnormSrgb`. Each layer
//! therefore samples a mirror image: the `Textures` CPU copy (with the
//! animated tiles and the anaglyph transform) written into an `Rgba8Unorm`
//! image whenever it differs.
//!
//! # Draw order
//!
//! Vanilla draws the entities, the lit particles and the particles after
//! the opaque terrain and before the translucent pass (EntityRenderer.java:
//! 497-505). In `Transparent3d` that is the band −0.95e6 … −0.91e6
//! (`gl_material::order`); the biases are [`order`].

use bevy::camera::visibility::NoFrustumCulling;
use bevy::math::DVec3;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;

use super::fx::Billboard;
use super::{EffectRenderer, FxKind};
use crate::GameFrameSet;
use crate::fog::FogState;
use crate::game::Game;
use crate::gl_material::{GlBlend, GlKey, GlMaterial, TessBatch};
use crate::textures::{self, ITEMS_PNG, TERRAIN_PNG, TextureLoader, TexturesSet};

/// `/particles.png` (layer 0).
pub const PARTICLES_PNG: &str = "/particles.png";
/// `/misc/footprint.png` (the footsteps).
pub const FOOTPRINT_PNG: &str = "/misc/footprint.png";

/// The textures of layers 0-2 and of the footsteps (index 3).
// src: EffectRenderer.java:62-72, EntityFootStepFX.java:32
pub const LAYER_TEXTURES: [&str; 4] = [PARTICLES_PNG, TERRAIN_PNG, ITEMS_PNG, FOOTPRINT_PNG];

/// Sort biases (module docs, "Draw order").
pub mod order {
    /// `renderLitParticles` (the footsteps), right after the entities.
    // src: EntityRenderer.java:502
    pub const LIT_PARTICLES: f32 = -0.93e6;
    /// `renderParticles`' layer 0; layer `n` is `+ n · 1000`.
    // src: EntityRenderer.java:505
    pub const PARTICLES: f32 = -0.925e6;
}

/// `glAlphaFunc(GL_GREATER, 0.1F)`, set at start-up and restored after the
/// weather.
// src: Minecraft.java:267, EntityRenderer.java:773
pub const ALPHA_REF: f32 = 0.1;

/// What the particles draw this frame.
#[derive(Resource, Clone, Debug, Default)]
pub struct ParticlesFrame {
    /// Whether the world is drawn.
    pub visible: bool,
    /// The GL origin: this frame's `interpPos`.
    pub anchor: Vec3,
    /// Layers 0-2, then the footsteps.
    pub batches: [TessBatch; 4],
}

/// One `EntityPickupFX` to draw this frame (for WP-3.5b): the particle at
/// `EffectRenderer.fx_layers[3][layer_index]` (its
/// `FxKind::Pickup(data).entity_to_pick_up` is the entity), drawn by
/// `RenderManager.renderEntityWithPosYaw(entity, pos, yaw, partial)` with
/// `glColor4f(brightness, brightness, brightness, 1)` and the standard item
/// lighting on, relative to [`ParticlesFrame::anchor`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickupDraw {
    /// Index in layer 3.
    pub layer_index: usize,
    /// The picked entity's id.
    pub entity_id: i32,
    /// The position relative to the anchor.
    pub pos: DVec3,
    /// `rotationYaw`.
    pub yaw: f32,
    /// The partial tick.
    pub partial: f32,
    /// `glColor4f`'s grey.
    pub brightness: f32,
}

/// The pickups of this frame (module docs; WP-3.5b draws them).
#[derive(Resource, Clone, Debug, Default)]
pub struct PickupFxFrame {
    /// In layer order.
    pub draws: Vec<PickupDraw>,
}

/// One particle mesh (0-2: the layers; 3: the footsteps).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParticleLayer(pub usize);

/// The meshes, materials and texture mirrors.
#[derive(Resource, Debug)]
pub struct ParticleHandles {
    /// Per layer: mesh, material.
    pub parts: [(Handle<Mesh>, Handle<GlMaterial>); 4],
    /// Per layer: the `Rgba8Unorm` mirror of its texture (module docs).
    pub mirrors: [Option<Handle<Image>>; 4],
}

/// Spawns the four particle meshes (hidden until used).
pub fn spawn_particles(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GlMaterial>>,
) {
    let parts = std::array::from_fn(|i| {
        let (blend, bias) = if i == 3 {
            (GlBlend::Alpha, order::LIT_PARTICLES)
        } else {
            (GlBlend::Off, order::PARTICLES + i as f32 * 1000.0)
        };
        let m = GlMaterial::new(
            GlKey {
                blend,
                depth_write: true,
                color_write: true,
                far: false,
                wide_lines: false,
            },
            bias,
        )
        .with_alpha_test(ALPHA_REF);
        let mh = materials.add(m);
        let mesh = meshes.add(TessBatch::placeholder_mesh());
        commands.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mh.clone()),
            Transform::IDENTITY,
            Visibility::Hidden,
            NoFrustumCulling,
            ParticleLayer(i),
        ));
        (mesh, mh)
    });
    commands.insert_resource(ParticleHandles {
        parts,
        mirrors: Default::default(),
    });
}

/// `renderLitParticles` then `renderParticles` for this frame (module
/// docs). F3's particle count is [`super::publish_statistics`].
// src: EntityRenderer.java:501-505, EffectRenderer.java:49-100
pub fn compute_particles(
    mut game: NonSendMut<Game>,
    mut er: ResMut<EffectRenderer>,
    mut frame: ResMut<ParticlesFrame>,
    mut pickups: ResMut<PickupFxFrame>,
) {
    let _perf = crate::perf::time(crate::perf::Phase::Particles);
    pickups.draws.clear();
    for b in &mut frame.batches {
        *b = TessBatch::default();
    }
    let partial = game.timer.render_partial_ticks;
    let Some(view) = game.render_view().map(|e| {
        let b = &e.base;
        (
            DVec3::new(b.last_tick_pos_x, b.last_tick_pos_y, b.last_tick_pos_z),
            DVec3::new(b.pos_x, b.pos_y, b.pos_z),
            b.rotation_yaw,
            b.rotation_pitch,
        )
    }) else {
        frame.visible = false;
        return;
    };
    let (last, pos, yaw, pitch) = view;
    // The pickers' positions, read before the world is borrowed mutably.
    let pickers: Vec<Option<(DVec3, DVec3)>> = er.fx_layers[3]
        .iter()
        .map(|fx| match &fx.kind {
            FxKind::Pickup(p) => entity_track(&game, p.entity_picking_up),
            _ => None,
        })
        .collect();
    let Some(world) = game.the_world.as_mut() else {
        frame.visible = false;
        return;
    };
    let er = &mut *er;

    // renderLitParticles(view, partial): last frame's interpPos (module docs).
    // src: EffectRenderer.java:89-100
    let old_interp = er.interp_pos;
    for (i, fx) in er.fx_layers[3].iter().enumerate() {
        match &fx.kind {
            FxKind::FootStep { .. } => {
                fx.render_footstep(world, &mut frame.batches[3], partial, old_interp);
            }
            FxKind::Pickup(p) => {
                let Some(picker) = pickers.get(i).copied().flatten() else {
                    continue;
                };
                if let Some(pose) = fx.pickup_pose(world, picker, partial, old_interp) {
                    pickups.draws.push(PickupDraw {
                        layer_index: i,
                        entity_id: p.entity_to_pick_up.base.entity_id,
                        pos: pose.pos,
                        yaw: pose.yaw,
                        partial,
                        brightness: pose.brightness,
                    });
                }
            }
            _ => {}
        }
    }

    // renderParticles(view, partial)
    // src: EffectRenderer.java:49-87
    let bb = Billboard::from_rotation(yaw, pitch);
    let p = partial as f64;
    let interp = DVec3::new(
        last.x + (pos.x - last.x) * p,
        last.y + (pos.y - last.y) * p,
        last.z + (pos.z - last.z) * p,
    );
    er.interp_pos = interp;
    for var8 in 0..3 {
        let batch = &mut frame.batches[var8];
        for fx in &mut er.fx_layers[var8] {
            fx.render_particle(world, batch, partial, &bb, interp);
        }
    }
    frame.visible = true;
    frame.anchor = interp.as_vec3();
}

/// An entity's `lastTickPos` and `pos` by id (in the world, else the
/// detached player).
fn entity_track(game: &Game, id: i32) -> Option<(DVec3, DVec3)> {
    let e = game
        .the_world
        .as_ref()
        .and_then(|w| w.entity(id))
        .or_else(|| {
            game.detached_player
                .as_deref()
                .filter(|p| p.base.entity_id == id)
        })?;
    let b = &e.base;
    Some((
        DVec3::new(b.last_tick_pos_x, b.last_tick_pos_y, b.last_tick_pos_z),
        DVec3::new(b.pos_x, b.pos_y, b.pos_z),
    ))
}

/// Writes [`ParticlesFrame`] into the meshes, binds each layer's texture
/// (its `Rgba8Unorm` mirror) and the fog.
#[allow(clippy::too_many_arguments)]
pub fn apply_particles(
    frame: Res<ParticlesFrame>,
    fog: Res<FogState>,
    handles: Option<ResMut<ParticleHandles>>,
    mut loader: TextureLoader,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GlMaterial>>,
    mut q: Query<(&ParticleLayer, &mut Transform, &mut Visibility)>,
) {
    let _perf = crate::perf::time(crate::perf::Phase::Particles);
    let Some(mut h) = handles else {
        return;
    };
    let shown = |i: usize| frame.visible && fog.active && !frame.batches[i].is_empty();
    for (layer, mut t, mut vis) in &mut q {
        let want = if shown(layer.0) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
        t.translation = frame.anchor;
    }
    for (i, name) in LAYER_TEXTURES.iter().enumerate() {
        if !shown(i) {
            continue;
        }
        let b = &frame.batches[i];
        let (mesh, mh) = h.parts[i].clone();
        if let Some(mut m) = meshes.get_mut(&mesh) {
            b.write_into(&mut m, b.quad_indices());
        }
        // renderEngine.getTexture(name) then the mirror (module docs).
        loader.get(name);
        let mirror = sync_mirror(
            &loader.textures,
            &mut loader.images,
            name,
            h.mirrors[i].take(),
        );
        h.mirrors[i] = mirror.clone();
        if let Some(mut m) = materials.get_mut(&mh) {
            if let Some(img) = mirror
                && m.texture.as_ref() != Some(&img)
            {
                m.texture = Some(img);
                m.params.textured = 1;
            }
            m.params.fog = fog.world.to_vec4();
            m.params.fog_color = Vec4::new(fog.color[0], fog.color[1], fog.color[2], 1.0);
        }
    }
}

/// The `Rgba8Unorm` mirror of the texture `name` (module docs): created
/// from, or rewritten with, the `Textures` CPU copy when it differs.
pub fn sync_mirror(
    tex: &textures::Textures,
    images: &mut Assets<Image>,
    name: &str,
    mirror: Option<Handle<Image>>,
) -> Option<Handle<Image>> {
    let e = tex.entry(name)?;
    if let Some(h) = mirror {
        let same = images.get(&h).is_some_and(|img| {
            img.data.as_deref() == Some(&e.rgba[..])
                && img.texture_descriptor.size.width == e.width
                && img.texture_descriptor.size.height == e.height
        });
        if !same && let Some(mut img) = images.get_mut(&h) {
            if img.texture_descriptor.size.width == e.width
                && img.texture_descriptor.size.height == e.height
            {
                img.data = Some(e.rgba.clone());
            } else {
                *img = unorm_image(e);
            }
        }
        return Some(h);
    }
    Some(images.add(unorm_image(e)))
}

/// The CPU copy of `e` as an `Rgba8Unorm` image with its sampler.
fn unorm_image(e: &textures::TextureEntry) -> Image {
    let mut img = textures::setup_texture(e.width, e.height, e.rgba.clone(), e.blur, e.clamp);
    img.texture_descriptor.format = TextureFormat::Rgba8Unorm;
    img
}

/// Adds the render systems (called by [`super::ParticlesPlugin`]).
pub fn build(app: &mut App) {
    app.init_resource::<ParticlesFrame>()
        .add_systems(Startup, spawn_particles)
        .add_systems(
            Update,
            (compute_particles, apply_particles)
                .chain()
                .after(GameFrameSet)
                .after(TexturesSet)
                .after(crate::sky::compute_render_frame)
                .after(super::apply_pending_clear),
        );
}
