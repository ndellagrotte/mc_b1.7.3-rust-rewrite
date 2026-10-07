//! The `debug_model` dev action (project-owned, not vanilla; WP-3.5a's
//! smoke test before WP-3.5b draws entities): a model posed by fixed
//! arguments, standing in front of the player, with its skin from the jar.
//!
//! The root matrix is a reduced `RenderLiving.doRenderLiving` (translate to
//! the feet, rotate `180 - yaw` about y, `glScalef(-1, -1, 1)`, the
//! renderer's `preRenderCallback` scale, translate `(0, -24 × 0.0625 -
//! 0.0078125, 0)`; RenderLiving.java:36-43), or `RenderMinecart`'s /
//! `RenderBoat`'s (translate, rotate, `glScalef(-1, -1, 1)`;
//! RenderMinecart.java:43-45, :73-74, RenderBoat.java:15-16, :31-32). It is a
//! dev aid; WP-3.5b transcribes the renderers.

use std::path::PathBuf;

use bevy::prelude::*;
use serde::Deserialize;

use super::{Model, ModelEntity, ModelKind, ModelMaterial, ModelMeshes, ModelPartEntity};
use crate::fog::FogState;
use crate::game::FrameView;
use crate::gl_material::GlBlend;
use crate::textures::TextureLoader;

/// A `debug_model` request (the script action's fields).
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct DebugModelSpec {
    /// The model ([`ModelKind::from_name`]: `"pig"`, `"sheep_fur"`, …).
    pub model: String,
    /// The texture (a jar path); default [`ModelKind::default_texture`].
    #[serde(default)]
    pub texture: Option<String>,
    /// Blocks in front of the eye (horizontally); default 3.
    #[serde(default = "default_distance")]
    pub distance: f64,
    /// Degrees added to the body yaw that faces the camera (90 shows the
    /// model's left side).
    #[serde(default)]
    pub turn: f32,
    /// `limbSwing` (`var1`).
    #[serde(default)]
    pub limb_swing: f32,
    /// `limbSwingAmount` (`var2`).
    #[serde(default)]
    pub limb_yaw: f32,
    /// `ticks` (`var3`).
    #[serde(default)]
    pub ticks: f32,
    /// `headYaw` (`var4`), degrees.
    #[serde(default)]
    pub head_yaw: f32,
    /// `headPitch` (`var5`), degrees.
    #[serde(default)]
    pub head_pitch: f32,
    /// `ModelBase.onGround` (the attack swing, 0..1).
    #[serde(default)]
    pub swing: f32,
    /// The `preRenderCallback` scale; default the renderer's at rest
    /// (ghast 4.5, others 1).
    #[serde(default)]
    pub scale: Option<f32>,
    /// `glColor3f` (brightness); default 1.
    #[serde(default = "default_brightness")]
    pub brightness: f32,
    /// Alpha-blended (the slime's outer cube) with this `glColor4f` alpha.
    #[serde(default)]
    pub blend_alpha: Option<f32>,
    /// The powered creeper's pass state (RenderCreeper.java:56-67): `GL_ONE,
    /// GL_ONE` blending, `glColor4f(0.5, 0.5, 0.5, 1)`, lighting off, the
    /// alpha test kept, the `GL_TEXTURE` matrix translated by `ticks ×
    /// 0.01` in u and v.
    #[serde(default)]
    pub additive: bool,
    /// Keep the previous debug models (to draw a pass model over its main
    /// model).
    #[serde(default)]
    pub keep: bool,
    /// Wolf only: sitting.
    #[serde(default)]
    pub sitting: bool,
    /// Where the script action saves a screenshot of it (none: no
    /// screenshot).
    #[serde(default)]
    pub path: Option<PathBuf>,
}

fn default_distance() -> f64 {
    3.0
}

fn default_brightness() -> f32 {
    1.0
}

/// The pending requests, in order ([`spawn_debug_model`] takes them).
#[derive(Resource, Default, Clone, Debug)]
pub struct DebugModelRequest(pub Vec<DebugModelSpec>);

/// A spawned debug model: its model state and arguments.
#[derive(Component, Clone, Debug)]
pub struct DebugModel {
    /// The model instance.
    pub model: Model,
    /// Its request.
    pub spec: DebugModelSpec,
    /// The material (for the fog).
    pub material: Handle<ModelMaterial>,
}

/// The root transform of a debug model at `feet` facing `yaw` degrees
/// (module docs).
pub fn root_transform(kind: ModelKind, feet: Vec3, yaw: f32, pre_scale: f32) -> Transform {
    let mut m = Mat4::from_translation(feet)
        * Mat4::from_rotation_y((180.0 - yaw).to_radians())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0));
    match kind {
        ModelKind::Minecart | ModelKind::Boat => {}
        ModelKind::Squid => {
            // RenderSquid.func_21007_a at pitch and yaw 0: up 0.5, down
            // 1.2 (before the flip), then RenderLiving's offset.
            // src: RenderSquid.java:14-22
            m = Mat4::from_translation(feet + Vec3::new(0.0, 0.5 - 1.2, 0.0))
                * Mat4::from_rotation_y((180.0 - yaw).to_radians())
                * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
                * Mat4::from_scale(Vec3::splat(pre_scale))
                * Mat4::from_translation(Vec3::new(0.0, -24.0 * 0.0625 - 0.0078125, 0.0));
        }
        _ => {
            m = m
                * Mat4::from_scale(Vec3::splat(pre_scale))
                * Mat4::from_translation(Vec3::new(0.0, -24.0 * 0.0625 - 0.0078125, 0.0));
        }
    }
    Transform::from_matrix(m)
}

/// Spawns the requested model in front of the player.
#[allow(clippy::too_many_arguments)]
pub fn spawn_debug_model(
    mut commands: Commands,
    mut request: ResMut<DebugModelRequest>,
    view: Res<FrameView>,
    fog: Res<FogState>,
    mut cache: ResMut<ModelMeshes>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ModelMaterial>>,
    mut textures: TextureLoader,
    old: Query<Entity, With<DebugModel>>,
) {
    if request.0.is_empty() {
        return;
    }
    let mut live: Vec<Entity> = old.iter().collect();
    for spec in std::mem::take(&mut request.0) {
        let Some(kind) = ModelKind::from_name(&spec.model) else {
            error!("debug_model: unknown model {:?}", spec.model);
            continue;
        };
        let Some((eye, yaw, _)) = view.player_eye else {
            error!("debug_model: no player");
            continue;
        };
        if !spec.keep {
            for e in live.drain(..) {
                commands.entity(e).despawn();
            }
        }
        let r = (yaw as f64).to_radians();
        let feet = Vec3::new(
            (eye[0] - r.sin() * spec.distance) as f32,
            (eye[1] - 1.62) as f32,
            (eye[2] + r.cos() * spec.distance) as f32,
        );
        let pre = spec.scale.unwrap_or(match kind {
            // RenderGhast.func_4014_a with attackCounter 0: (8 + 1) / 2.
            // src: RenderGhast.java:11-19
            ModelKind::Ghast => 4.5,
            _ => 1.0,
        });
        let root = root_transform(kind, feet, yaw + 180.0 + spec.turn, pre);
        let tex = textures.get(spec.texture.as_deref().unwrap_or(kind.default_texture()));
        let mut mat = ModelMaterial::living(tex, spec.brightness).with_fog(&fog);
        if matches!(kind, ModelKind::Minecart | ModelKind::Boat) {
            // Render (not RenderLiving): GL_CULL_FACE stays on.
            mat.key.cull = true;
        }
        if let Some(a) = spec.blend_alpha {
            mat.key.blend = GlBlend::Alpha;
            mat.params.color.w = a;
            mat.sort_bias = -0.93e6;
        }
        if spec.additive {
            // src: RenderCreeper.java:58-60, :63-67
            mat.key.blend = GlBlend::Additive;
            mat.key.src_one = true;
            mat.params.color = Vec4::new(0.5, 0.5, 0.5, 1.0);
            mat.params.lighting = 0;
            let t = spec.ticks * 0.01;
            mat.params.uv_offset = Vec4::new(t, t, 0.0, 0.0);
            mat.sort_bias = -0.93e6;
        }
        let mh = materials.add(mat);
        let id = super::spawn_model(
            &mut commands,
            &mut cache,
            &mut meshes,
            kind,
            mh.clone(),
            root,
        );
        let mut model = Model::new(kind);
        model.on_ground = spec.swing;
        info!(
            "debug_model: {} at ({:.2}, {:.2}, {:.2})",
            kind.name(),
            feet.x,
            feet.y,
            feet.z
        );
        commands.entity(id).insert(DebugModel {
            model,
            spec,
            material: mh,
        });
        live.push(id);
    }
}

/// Poses the debug models every frame (and keeps their fog current); hides
/// them while no world is drawn.
///
/// Hiding matters beyond looks: Bevy specialises the pipelines of visible
/// meshes per view, and the world camera's key changes when its
/// `DistanceFog` goes (leaving the world), so a model left visible starts a
/// pipeline compile at shutdown that can outlive the device.
pub fn update_debug_model(
    mut models: Query<(&mut DebugModel, &ModelEntity, &mut Visibility), Without<ModelPartEntity>>,
    mut parts: Query<(&mut Transform, &mut Visibility), With<ModelPartEntity>>,
    mut materials: ResMut<Assets<ModelMaterial>>,
    fog: Res<FogState>,
) {
    for (mut d, me, mut root_vis) in &mut models {
        let want = if fog.active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *root_vis != want {
            *root_vis = want;
        }
        if !fog.active {
            continue;
        }
        let s = d.spec.clone();
        if d.model.kind == ModelKind::Wolf {
            let inputs = super::LivingInputs::Wolf(super::wolf::WolfAnimInputs {
                sitting: s.sitting,
                brightness: s.brightness,
                ..Default::default()
            });
            d.model
                .set_living_animations(&inputs, s.limb_swing, s.limb_yaw, 0.0);
        }
        let draws = d.model.render(
            s.limb_swing,
            s.limb_yaw,
            s.ticks,
            s.head_yaw,
            s.head_pitch,
            super::MODEL_SCALE,
        );
        super::apply_pose(me, &draws, &mut parts);
        // Touch the material only when its fog really changes (a
        // `get_mut` re-prepares it).
        if let Some(m) = materials.get(&d.material) {
            let mut want = m.clone();
            want.set_fog(&fog);
            if want.params != m.params
                && let Some(mut m) = materials.get_mut(&d.material)
            {
                m.params = want.params;
            }
        }
    }
}
