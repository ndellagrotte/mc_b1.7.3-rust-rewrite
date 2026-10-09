//! The entity draws as Bevy entities (project-owned; PLAN_v4 D7).
//!
//! - **Models** ([`super::gl::ModelDraw`]): each owner ([`VisualKey`]: a sim
//!   entity id, a tile entity, a pickup) keeps a list of model instances,
//!   one per `ModelBase.render` call of its draw in call order (the main
//!   model, the pass models, the hurt overlay's copies…). An instance is a
//!   root entity with one child per part (WP-3.5a's shared part meshes from
//!   [`ModelMeshes`]; [`crate::model::spawn_model`]'s shape, spawned here
//!   already posed so a new entity shows on its first frame) and its own
//!   [`ModelMaterial`], rewritten only when the GL state changed. Each
//!   frame the root takes the modelview of the call and
//!   [`crate::model::apply_pose`] places the parts. Instances an owner did
//!   not use this frame are hidden; an owner whose entity left the world
//!   (not in [`EntityFrame::live`]), or that drew nothing for
//!   [`FORGET_FRAMES`] frames, is despawned.
//! - **Tessellator batches** ([`super::gl::TessDraw`]): pooled mesh
//!   entities with [`EntityGlMaterial`], the `n`-th batch of the frame in
//!   slot `n` (sort bias [`super::order::TESS`] `+ n`), the mesh rewritten
//!   every frame; unused slots are hidden.
//!
//! Everything is placed at [`EntityFrame::anchor`] (`renderPos`) plus the
//! GL entity space coordinates, and hidden while no world is drawn
//! (`FogState::active`; model pipelines must not be re-specialised while
//! the world camera has no fog, WP-3.5a's README note).

use std::collections::BTreeMap;

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use super::gl::{ModelDraw, ModelSpec, TessDraw, TessSpec, Topology, VisualKey};
use super::material::{EntityGlMaterial, EntityGlParams};
use super::{EntityFrame, order};
use crate::fog::FogState;
use crate::gui::batch::TexRef;
use crate::model::{
    ModelEntity, ModelKey, ModelKind, ModelMaterial, ModelMeshes, ModelParams, ModelPartEntity,
    PartDraw,
};
use crate::textures::TextureLoader;

/// Frames an owner may draw nothing before its instances are despawned.
pub const FORGET_FRAMES: u64 = 600;

/// One model instance (module docs).
struct ModelSlot {
    kind: ModelKind,
    root: Entity,
    model: ModelEntity,
    material: Handle<ModelMaterial>,
    last: Option<ModelMaterial>,
}

/// One owner's instances.
#[derive(Default)]
struct Visual {
    slots: Vec<ModelSlot>,
    last_drawn: u64,
}

/// A pooled Tessellator batch.
struct TessSlot {
    entity: Entity,
    mesh: Handle<Mesh>,
    material: Handle<EntityGlMaterial>,
    topology: Topology,
    last: Option<EntityGlMaterial>,
}

/// Marks a pooled batch entity.
#[derive(Component, Clone, Copy, Debug)]
pub struct EntityTessBatch(pub usize);

/// The Bevy side of the entity draws (module docs).
#[derive(Resource, Default)]
pub struct EntityVisuals {
    visuals: BTreeMap<VisualKey, Visual>,
    tess: Vec<TessSlot>,
    frame: u64,
}

impl EntityVisuals {
    /// The number of model instances alive (diagnostics).
    pub fn model_instances(&self) -> usize {
        self.visuals.values().map(|v| v.slots.len()).sum()
    }
}

/// A texture reference as a handle (`getTexture(name)` or the handle).
fn resolve(loader: &mut TextureLoader, t: &Option<TexRef>) -> Option<Handle<Image>> {
    match t {
        Some(TexRef::Name(n)) => Some(loader.get(n)),
        Some(TexRef::Handle(h)) => Some(h.clone()),
        None => None,
    }
}

/// The model material of `spec`.
fn model_material(
    spec: &ModelSpec,
    texture: Option<Handle<Image>>,
    fog: &FogState,
) -> ModelMaterial {
    let mut m = ModelMaterial {
        params: ModelParams {
            color: Vec4::from(spec.color),
            uv_offset: Vec4::new(spec.uv_offset[0], spec.uv_offset[1], 0.0, 0.0),
            alpha_ref: spec.alpha_ref.unwrap_or(-1.0),
            textured: texture.is_some() as u32,
            lighting: spec.lighting as u32,
            texture_srgb: 1,
            ..default()
        },
        texture,
        key: ModelKey {
            blend: spec.blend,
            depth_write: spec.depth_write,
            depth_equal: spec.depth_equal,
            cull: spec.cull,
            src_one: spec.src_one,
        },
        sort_bias: if spec.blend == crate::gl_material::GlBlend::Off {
            0.0
        } else {
            order::MODEL_BLEND
        },
    };
    if spec.fog {
        m.set_fog(fog);
    }
    m
}

/// The batch material of `spec`.
fn tess_material(
    spec: &TessSpec,
    texture: Option<Handle<Image>>,
    fog: &FogState,
    n: usize,
) -> EntityGlMaterial {
    let mut params = EntityGlParams {
        alpha_ref: spec.alpha_ref.unwrap_or(-1.0),
        textured: texture.is_some() as u32,
        ..default()
    };
    if spec.fog && fog.active {
        params.fog = fog.world.to_vec4();
        params.fog_color = Vec3::from(fog.color).extend(1.0);
    }
    EntityGlMaterial {
        params,
        texture,
        key: spec.key,
        sort_bias: order::TESS + n as f32,
    }
}

/// The batch's mesh.
fn tess_mesh(d: &TessDraw) -> Mesh {
    let topology = match d.spec.topology {
        Topology::Triangles => PrimitiveTopology::TriangleList,
        Topology::Lines => PrimitiveTopology::LineList,
    };
    let mut m = Mesh::new(topology, RenderAssetUsages::default());
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, d.positions.clone());
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, d.uvs.clone());
    m.insert_attribute(Mesh::ATTRIBUTE_COLOR, d.colors.clone());
    m.insert_indices(Indices::U32((0..d.positions.len() as u32).collect()));
    m
}

/// A root transform: the GL modelview placed at the anchor.
fn root_transform(anchor: Vec3, root: Mat4) -> Transform {
    Transform::from_matrix(Mat4::from_translation(anchor) * root)
}

/// Spawns a model instance already posed with `parts` (the shape of
/// [`crate::model::spawn_model`]).
fn spawn_posed(
    commands: &mut Commands,
    cache: &mut ModelMeshes,
    meshes: &mut Assets<Mesh>,
    kind: ModelKind,
    material: Handle<ModelMaterial>,
    root: Transform,
    parts: &[PartDraw],
) -> (Entity, ModelEntity) {
    let handles = cache.get(kind, meshes).to_vec();
    let root_id = commands.spawn((root, Visibility::Visible)).id();
    let ids = handles
        .into_iter()
        .enumerate()
        .map(|(index, h)| {
            h.map(|mesh| {
                let (t, v) = match parts.iter().find(|d| d.part == index) {
                    Some(d) => (d.transform, Visibility::Inherited),
                    None => (Transform::IDENTITY, Visibility::Hidden),
                };
                commands
                    .spawn((
                        Mesh3d(mesh),
                        MeshMaterial3d(material.clone()),
                        t,
                        v,
                        ModelPartEntity { index },
                        ChildOf(root_id),
                    ))
                    .id()
            })
        })
        .collect();
    let me = ModelEntity { kind, parts: ids };
    commands.entity(root_id).insert(me.clone());
    (root_id, me)
}

/// Writes the frame's draws into the Bevy entities (module docs).
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn apply_draws(
    mut commands: Commands,
    frame: Res<EntityFrame>,
    fog: Res<FogState>,
    mut visuals: ResMut<EntityVisuals>,
    mut loader: TextureLoader,
    mut meshes: ResMut<Assets<Mesh>>,
    mut model_meshes: ResMut<ModelMeshes>,
    mut model_materials: ResMut<Assets<ModelMaterial>>,
    mut tess_materials: ResMut<Assets<EntityGlMaterial>>,
    mut roots: Query<
        (&mut Transform, &mut Visibility),
        (
            With<ModelEntity>,
            Without<ModelPartEntity>,
            Without<EntityTessBatch>,
        ),
    >,
    mut parts: Query<(&mut Transform, &mut Visibility), With<ModelPartEntity>>,
    mut batches: Query<
        (&mut Transform, &mut Visibility),
        (
            With<EntityTessBatch>,
            Without<ModelEntity>,
            Without<ModelPartEntity>,
        ),
    >,
) {
    let _perf = crate::perf::time(crate::perf::Phase::Entities);
    let v = &mut *visuals;
    v.frame += 1;
    let now = v.frame;
    let shown = frame.visible && fog.active;
    let anchor = frame.anchor.as_vec3();
    let (no_models, no_tess) = (Vec::new(), Vec::new());
    let (models, tess): (&Vec<ModelDraw>, &Vec<TessDraw>) = if shown {
        (&frame.gl.out.models, &frame.gl.out.tess)
    } else {
        (&no_models, &no_tess)
    };

    // Models, per owner in call order.
    let mut used: BTreeMap<VisualKey, usize> = BTreeMap::new();
    for d in models {
        let n = used.entry(d.key).or_insert(0);
        let i = *n;
        *n += 1;
        let vis = v.visuals.entry(d.key).or_default();
        vis.last_drawn = now;
        let tex = resolve(&mut loader, &d.spec.texture);
        let mat = model_material(&d.spec, tex, &fog);
        let rt = root_transform(anchor, d.root);
        if vis.slots.get(i).is_some_and(|s| s.kind != d.kind) {
            // a different model in this place: start the rest afresh
            for s in vis.slots.drain(i..) {
                commands.entity(s.root).despawn();
            }
        }
        if i >= vis.slots.len() {
            let h = model_materials.add(mat.clone());
            let (root, me) = spawn_posed(
                &mut commands,
                &mut model_meshes,
                &mut meshes,
                d.kind,
                h.clone(),
                rt,
                &d.parts,
            );
            vis.slots.push(ModelSlot {
                kind: d.kind,
                root,
                model: me,
                material: h,
                last: Some(mat),
            });
            continue;
        }
        let s = &mut vis.slots[i];
        if s.last
            .as_ref()
            .is_none_or(|l| !same_model_material(l, &mat))
        {
            if let Some(mut m) = model_materials.get_mut(&s.material) {
                *m = mat.clone();
            }
            s.last = Some(mat);
        }
        if let Ok((mut t, mut vv)) = roots.get_mut(s.root) {
            *t = rt;
            if *vv != Visibility::Visible {
                *vv = Visibility::Visible;
            }
        }
        crate::model::apply_pose(&s.model, &d.parts, &mut parts);
    }
    // Hide what was not used; forget what left the world.
    let live = &frame.live;
    let mut gone = Vec::new();
    for (key, vis) in v.visuals.iter_mut() {
        let n = used.get(key).copied().unwrap_or(0);
        for s in vis.slots.iter().skip(n) {
            if let Ok((_, mut vv)) = roots.get_mut(s.root)
                && *vv != Visibility::Hidden
            {
                *vv = Visibility::Hidden;
            }
        }
        let left = match key {
            VisualKey::Pickup(_) => false,
            _ => frame.visible && !live.contains(key),
        };
        if left || now.saturating_sub(vis.last_drawn) > FORGET_FRAMES {
            gone.push(*key);
        }
    }
    for key in gone {
        if let Some(vis) = v.visuals.remove(&key) {
            for s in vis.slots {
                commands.entity(s.root).despawn();
            }
        }
    }

    // Tessellator batches.
    for (n, d) in tess.iter().enumerate() {
        let tex = resolve(&mut loader, &d.spec.texture);
        let mat = tess_material(&d.spec, tex, &fog, n);
        if n >= v.tess.len() {
            let mesh = meshes.add(tess_mesh(d));
            let material = tess_materials.add(mat.clone());
            let entity = commands
                .spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material.clone()),
                    Transform::from_translation(anchor),
                    Visibility::Visible,
                    NoFrustumCulling,
                    EntityTessBatch(n),
                ))
                .id();
            v.tess.push(TessSlot {
                entity,
                mesh,
                material,
                topology: d.spec.topology,
                last: Some(mat),
            });
            continue;
        }
        let s = &mut v.tess[n];
        if let Some(mut m) = meshes.get_mut(&s.mesh) {
            *m = tess_mesh(d);
        }
        s.topology = d.spec.topology;
        if s.last.as_ref() != Some(&mat) {
            if let Some(mut m) = tess_materials.get_mut(&s.material) {
                *m = mat.clone();
            }
            s.last = Some(mat);
        }
        if let Ok((mut t, mut vv)) = batches.get_mut(s.entity) {
            t.translation = anchor;
            if *vv != Visibility::Visible {
                *vv = Visibility::Visible;
            }
        }
    }
    for s in v.tess.iter().skip(tess.len()) {
        if let Ok((_, mut vv)) = batches.get_mut(s.entity)
            && *vv != Visibility::Hidden
        {
            *vv = Visibility::Hidden;
        }
    }
}

/// Whether two model materials are the same state.
fn same_model_material(a: &ModelMaterial, b: &ModelMaterial) -> bool {
    a.params == b.params && a.texture == b.texture && a.key == b.key && a.sort_bias == b.sort_bias
}
