//! The `Model*` family (WP-3.5a; PLAN_v4 D1 rule 6 "models as data tables",
//! D7): every model the b1.7.3 renderers build, as part tables
//! ([`part::ModelPart`] = `ModelRenderer`) with their pose functions
//! (`setRotationAngles`, `setLivingAnimations`) transcribed, and the
//! machinery to draw them with Bevy: one shared [`Mesh`] per part (the
//! display list), one [`Transform`] per part per frame (the
//! `glTranslatef`/`glRotatef`s), and [`material::ModelMaterial`] (the GL
//! state, with GL lighting).
//!
//! # Shape
//!
//! `ModelBase` subclasses become [`ModelKind`] (one variant per constructor
//! call the renderers make) and one [`Model`]: the parts in the Java's field
//! order (index constants in each model's module, e.g. [`biped::HEAD`]) and
//! `ModelBase`'s/`ModelBiped`'s public state (`onGround`, `isRiding`,
//! `heldItemLeft/Right`, `isSneak`) as fields. The Java renderers keep one
//! model instance per entity class and set that state before each draw;
//! poses carry over between entities where the Java does not reset them
//! (e.g. the wolf's rotation points, `ModelMinecart`'s inner floor), so
//! keep one [`Model`] per renderer as the Java does.
//!
//! A frame's draw is [`Model::render`] (the Java `render`: it calls
//! `setRotationAngles` where the Java does and returns the parts drawn, in
//! order, with their transforms), after [`Model::set_living_animations`]
//! (only `ModelWolf` has one).
//!
//! # Model space
//!
//! Part transforms map display-list vertices (box coordinates × 0.0625,
//! model y down) into the model's space. The renderer's matrix
//! (`RenderLiving.doRenderLiving`: translate to the entity, rotate `180 -
//! renderYawOffset` about y, death tilt, `glScalef(-1, -1, 1)`,
//! `preRenderCallback`'s scale, translate `(0, -24 × 0.0625 - 0.0078125,
//! 0)`) goes on the root; see the README's "Models" section.

pub mod bed;
pub mod biped;
pub mod boat;
pub mod chicken;
pub mod cow;
pub mod creeper;
pub mod debug;
pub mod ghast;
pub mod material;
pub mod minecart;
pub mod part;
pub mod pig;
pub mod quadruped;
pub mod sheep;
pub mod skeleton;
pub mod slime;
pub mod spider;
pub mod squid;
pub mod wolf;
pub mod zombie;

use std::collections::BTreeMap;

use bevy::prelude::*;

pub use material::{ModelKey, ModelMaterial, ModelMaterialPlugin, ModelParams};
pub use part::{MODEL_SCALE, ModelPart, RenderMode};

/// Every model instance the b1.7.3 renderers construct (RenderManager.java:
/// 27-41 and the renderers' constructors).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ModelKind {
    /// `new ModelBiped()` / `ModelBiped(0.0F)`: the player
    /// (RenderPlayer.java:13) and `RenderLiving`'s fallback
    /// (RenderManager.java:41).
    Biped,
    /// `new ModelBiped(0.5F)`: `RenderPlayer.modelArmor` (leggings,
    /// RenderPlayer.java:16).
    BipedArmor,
    /// `new ModelBiped(1.0F)`: `RenderPlayer.modelArmorChestplate`
    /// (helmet, chestplate, boots; RenderPlayer.java:15).
    BipedArmorChestplate,
    /// `new ModelZombie()`: zombies, pigmen, giants (RenderManager.java:35,
    /// :38).
    Zombie,
    /// `new ModelSkeleton()` (RenderManager.java:34).
    Skeleton,
    /// `new ModelPig()` (RenderManager.java:28).
    Pig,
    /// `new ModelPig(0.5F)`: the saddle pass (RenderManager.java:28).
    PigSaddle,
    /// `new ModelCow()` (RenderManager.java:30).
    Cow,
    /// `new ModelSheep2()`: the sheared body, `RenderSheep`'s main model
    /// (RenderManager.java:29).
    Sheep,
    /// `new ModelSheep1()`: the wool pass (RenderManager.java:29).
    SheepFur,
    /// `new ModelChicken()` (RenderManager.java:32).
    Chicken,
    /// `new ModelWolf()` (RenderManager.java:31).
    Wolf,
    /// `new ModelCreeper()` (RenderCreeper.java:9).
    Creeper,
    /// `new ModelCreeper(2.0F)`: the powered creeper's armour pass
    /// (RenderCreeper.java:6).
    CreeperArmor,
    /// `new ModelSpider()`: the spider and its eyes pass
    /// (RenderSpider.java:7-8).
    Spider,
    /// `new ModelSlime(16)`: the inner body with eyes and mouth
    /// (RenderManager.java:36).
    Slime,
    /// `new ModelSlime(0)`: the translucent outer cube (RenderManager.java:36).
    SlimeOuter,
    /// `new ModelGhast()` (RenderGhast.java:7).
    Ghast,
    /// `new ModelSquid()` (RenderManager.java:40).
    Squid,
    /// `new ModelMinecart()` (RenderMinecart.java:10).
    Minecart,
    /// `new ModelBoat()` (RenderBoat.java:10).
    Boat,
}

impl ModelKind {
    /// Every kind.
    pub const ALL: [ModelKind; 21] = [
        ModelKind::Biped,
        ModelKind::BipedArmor,
        ModelKind::BipedArmorChestplate,
        ModelKind::Zombie,
        ModelKind::Skeleton,
        ModelKind::Pig,
        ModelKind::PigSaddle,
        ModelKind::Cow,
        ModelKind::Sheep,
        ModelKind::SheepFur,
        ModelKind::Chicken,
        ModelKind::Wolf,
        ModelKind::Creeper,
        ModelKind::CreeperArmor,
        ModelKind::Spider,
        ModelKind::Slime,
        ModelKind::SlimeOuter,
        ModelKind::Ghast,
        ModelKind::Squid,
        ModelKind::Minecart,
        ModelKind::Boat,
    ];

    /// The variant's name as written (`"Biped"`, `"SheepFur"`, …).
    pub fn name(self) -> &'static str {
        match self {
            ModelKind::Biped => "Biped",
            ModelKind::BipedArmor => "BipedArmor",
            ModelKind::BipedArmorChestplate => "BipedArmorChestplate",
            ModelKind::Zombie => "Zombie",
            ModelKind::Skeleton => "Skeleton",
            ModelKind::Pig => "Pig",
            ModelKind::PigSaddle => "PigSaddle",
            ModelKind::Cow => "Cow",
            ModelKind::Sheep => "Sheep",
            ModelKind::SheepFur => "SheepFur",
            ModelKind::Chicken => "Chicken",
            ModelKind::Wolf => "Wolf",
            ModelKind::Creeper => "Creeper",
            ModelKind::CreeperArmor => "CreeperArmor",
            ModelKind::Spider => "Spider",
            ModelKind::Slime => "Slime",
            ModelKind::SlimeOuter => "SlimeOuter",
            ModelKind::Ghast => "Ghast",
            ModelKind::Squid => "Squid",
            ModelKind::Minecart => "Minecart",
            ModelKind::Boat => "Boat",
        }
    }

    /// The kind named `s` (case-insensitive, `_` ignored: `"sheep_fur"`).
    pub fn from_name(s: &str) -> Option<ModelKind> {
        let norm = |t: &str| t.replace('_', "").to_ascii_lowercase();
        let s = norm(s);
        ModelKind::ALL.into_iter().find(|k| norm(k.name()) == s)
    }

    /// The texture the renderer binds for this model in the common case
    /// (`EntityLiving.texture` / `getEntityTexture()`, or the renderer's
    /// `loadTexture`); the full table, with the alternatives, is in the
    /// README ("Models").
    pub fn default_texture(self) -> &'static str {
        match self {
            // src: EntityLiving.java:16, EntityPlayer.java:51
            ModelKind::Biped => "/mob/char.png",
            // src: RenderPlayer.java:25 (renderIndex 2, iron; pass 2 is "_2")
            ModelKind::BipedArmor => "/armor/iron_2.png",
            ModelKind::BipedArmorChestplate => "/armor/iron_1.png",
            // src: EntityZombie.java:6
            ModelKind::Zombie => "/mob/zombie.png",
            // src: EntitySkeleton.java:8
            ModelKind::Skeleton => "/mob/skeleton.png",
            // src: EntityPig.java:6
            ModelKind::Pig => "/mob/pig.png",
            // src: RenderPig.java:10
            ModelKind::PigSaddle => "/mob/saddle.png",
            // src: EntityCow.java:6
            ModelKind::Cow => "/mob/cow.png",
            // src: EntitySheep.java:10
            ModelKind::Sheep => "/mob/sheep.png",
            // src: RenderSheep.java:13
            ModelKind::SheepFur => "/mob/sheep_fur.png",
            // src: EntityChicken.java:14
            ModelKind::Chicken => "/mob/chicken.png",
            // src: EntityWolf.java:17
            ModelKind::Wolf => "/mob/wolf.png",
            // src: EntityCreeper.java:9
            ModelKind::Creeper => "/mob/creeper.png",
            // src: RenderCreeper.java:55
            ModelKind::CreeperArmor => "/armor/power.png",
            // src: EntitySpider.java:6
            ModelKind::Spider => "/mob/spider.png",
            // src: EntitySlime.java:10
            ModelKind::Slime | ModelKind::SlimeOuter => "/mob/slime.png",
            // src: EntityGhast.java:15
            ModelKind::Ghast => "/mob/ghast.png",
            // src: EntitySquid.java:21
            ModelKind::Squid => "/mob/squid.png",
            // src: RenderMinecart.java:73
            ModelKind::Minecart => "/item/cart.png",
            // src: RenderBoat.java:31
            ModelKind::Boat => "/item/boat.png",
        }
    }

    /// The parts as the constructor leaves them.
    pub fn build(self) -> Vec<ModelPart> {
        match self {
            ModelKind::Biped => biped::build(0.0, 0.0),
            ModelKind::BipedArmor => biped::build(0.5, 0.0),
            ModelKind::BipedArmorChestplate => biped::build(1.0, 0.0),
            ModelKind::Zombie => biped::build(0.0, 0.0),
            ModelKind::Skeleton => skeleton::build(),
            ModelKind::Pig => pig::build(0.0),
            ModelKind::PigSaddle => pig::build(0.5),
            ModelKind::Cow => cow::build(),
            ModelKind::Sheep => sheep::build_sheep2(),
            ModelKind::SheepFur => sheep::build_sheep1(),
            ModelKind::Chicken => chicken::build(),
            ModelKind::Wolf => wolf::build(),
            ModelKind::Creeper => creeper::build(0.0),
            ModelKind::CreeperArmor => creeper::build(2.0),
            ModelKind::Spider => spider::build(),
            ModelKind::Slime => slime::build(16),
            ModelKind::SlimeOuter => slime::build(0),
            ModelKind::Ghast => ghast::build(),
            ModelKind::Squid => squid::build(),
            ModelKind::Minecart => minecart::build(),
            ModelKind::Boat => boat::build(),
        }
    }

    /// `render`'s draw list (part index, draw method), in order.
    pub fn render_list(self) -> &'static [(usize, RenderMode)] {
        match self {
            ModelKind::Biped
            | ModelKind::BipedArmor
            | ModelKind::BipedArmorChestplate
            | ModelKind::Zombie
            | ModelKind::Skeleton => biped::RENDER,
            ModelKind::Pig | ModelKind::PigSaddle | ModelKind::Sheep | ModelKind::SheepFur => {
                quadruped::RENDER
            }
            ModelKind::Cow => cow::RENDER,
            ModelKind::Chicken => chicken::RENDER,
            ModelKind::Wolf => wolf::RENDER,
            ModelKind::Creeper | ModelKind::CreeperArmor => creeper::RENDER,
            ModelKind::Spider => spider::RENDER,
            ModelKind::Slime => slime::RENDER,
            ModelKind::SlimeOuter => &slime::RENDER[..1],
            ModelKind::Ghast => ghast::RENDER,
            ModelKind::Squid => squid::RENDER,
            ModelKind::Minecart => minecart::RENDER,
            ModelKind::Boat => boat::RENDER,
        }
    }

    /// Whether this is a `ModelBiped` (has `renderEars`/`renderCloak` and
    /// the biped part indices).
    pub fn is_biped(self) -> bool {
        matches!(
            self,
            ModelKind::Biped
                | ModelKind::BipedArmor
                | ModelKind::BipedArmorChestplate
                | ModelKind::Zombie
                | ModelKind::Skeleton
        )
    }
}

/// What `ModelBase.setLivingAnimations` reads of the entity (only
/// `ModelWolf` overrides it in b1.7.3).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum LivingInputs {
    /// Nothing (`ModelBase`'s empty method).
    #[default]
    None,
    /// The wolf's state.
    Wolf(wolf::WolfAnimInputs),
}

/// One part drawn by [`Model::render`]: its index and its transform in the
/// model's space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartDraw {
    /// Index into [`Model::parts`].
    pub part: usize,
    /// The draw method.
    pub mode: RenderMode,
    /// The part's transform ([`ModelPart::transform`]).
    pub transform: Transform,
}

/// A model instance: `ModelBase`'s state and its parts (module docs).
#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    /// Which model.
    pub kind: ModelKind,
    /// The `ModelRenderer`s, in field order.
    pub parts: Vec<ModelPart>,
    /// `ModelBase.onGround`: the attack swing progress
    /// (`RenderLiving.func_167_c` → `getSwingProgress`).
    pub on_ground: f32,
    /// `ModelBase.isRiding`.
    pub is_riding: bool,
    /// `ModelBiped.heldItemLeft`.
    pub held_item_left: bool,
    /// `ModelBiped.heldItemRight`.
    pub held_item_right: bool,
    /// `ModelBiped.isSneak`.
    pub is_sneak: bool,
}

impl Model {
    /// The model as its constructor leaves it.
    pub fn new(kind: ModelKind) -> Self {
        Model {
            kind,
            parts: kind.build(),
            on_ground: 0.0,
            is_riding: false,
            held_item_left: false,
            held_item_right: false,
            is_sneak: false,
        }
    }

    /// `setRotationAngles(limbSwing, limbSwingAmount, ticks, headYaw,
    /// headPitch, scale)` (`var1`..`var6`; `ticks` is the renderer's
    /// `func_170_d`: age + partial, the chicken's wing angle, the wolf's
    /// tail angle, the squid's tentacle angle). `ModelSlime`, `ModelBoat`
    /// and `ModelMinecart` do nothing.
    pub fn set_rotation_angles(
        &mut self,
        var1: f32,
        var2: f32,
        var3: f32,
        var4: f32,
        var5: f32,
        var6: f32,
    ) {
        let f = match self.kind {
            ModelKind::Biped | ModelKind::BipedArmor | ModelKind::BipedArmorChestplate => {
                biped::set_rotation_angles
            }
            ModelKind::Zombie | ModelKind::Skeleton => zombie::set_rotation_angles,
            ModelKind::Pig | ModelKind::PigSaddle | ModelKind::Sheep | ModelKind::SheepFur => {
                quadruped::set_rotation_angles
            }
            ModelKind::Cow => cow::set_rotation_angles,
            ModelKind::Chicken => chicken::set_rotation_angles,
            ModelKind::Wolf => wolf::set_rotation_angles,
            ModelKind::Creeper | ModelKind::CreeperArmor => creeper::set_rotation_angles,
            ModelKind::Spider => spider::set_rotation_angles,
            ModelKind::Ghast => ghast::set_rotation_angles,
            ModelKind::Squid => squid::set_rotation_angles,
            ModelKind::Slime | ModelKind::SlimeOuter | ModelKind::Minecart | ModelKind::Boat => {
                return;
            }
        };
        f(self, var1, var2, var3, var4, var5, var6);
    }

    /// `setLivingAnimations(entity, limbSwing, limbSwingAmount, partial)`;
    /// returns the `glColor3f` it sets, if any (the shaking wolf). A no-op
    /// for every model but the wolf, and for the wolf without its inputs.
    // src: ModelBase.java:13-14, ModelWolf.java:72-124
    pub fn set_living_animations(
        &mut self,
        inputs: &LivingInputs,
        var2: f32,
        var3: f32,
        _var4: f32,
    ) -> Option<[f32; 3]> {
        match (self.kind, inputs) {
            (ModelKind::Wolf, LivingInputs::Wolf(w)) => {
                wolf::set_living_animations(self, w, var2, var3)
            }
            _ => None,
        }
    }

    /// `render(limbSwing, limbSwingAmount, ticks, headYaw, headPitch,
    /// scale)`: `setRotationAngles` (every model but `ModelBoat` and
    /// `ModelMinecart`, whose `render` does not call it;
    /// `ModelMinecart.render` sets the inner floor's rotation point y to
    /// `4 - ticks` instead), then the drawn parts with their transforms, in
    /// draw order. Parts that are hidden, `showModel = false` or without a
    /// box are skipped as `ModelRenderer.render` skips them.
    // src: ModelBiped.java:58-67, ModelMinecart.java:36-43, ModelBoat.java:32-37, ModelWolf.java:56-70
    pub fn render(
        &mut self,
        var1: f32,
        var2: f32,
        var3: f32,
        var4: f32,
        var5: f32,
        var6: f32,
    ) -> Vec<PartDraw> {
        match self.kind {
            ModelKind::Minecart => {
                self.parts[minecart::INNER].rotation_point_y = 4.0 - var3;
            }
            ModelKind::Boat => {}
            _ => self.set_rotation_angles(var1, var2, var3, var4, var5, var6),
        }
        self.draws(self.kind.render_list(), var6)
    }

    /// The transforms of `list`'s drawn parts, without posing.
    pub fn draws(&self, list: &[(usize, RenderMode)], scale: f32) -> Vec<PartDraw> {
        list.iter()
            .filter(|&&(i, _)| self.parts.get(i).is_some_and(ModelPart::is_drawn))
            .map(|&(i, mode)| PartDraw {
                part: i,
                mode,
                transform: self.parts[i].transform(mode, scale),
            })
            .collect()
    }

    /// `ModelBiped.renderEars(scale)`: the ears take the head's angles and
    /// are drawn (`None` if hidden or not a biped).
    // src: ModelBiped.java:164-170
    pub fn render_ears(&mut self, scale: f32) -> Option<PartDraw> {
        if !self.kind.is_biped() {
            return None;
        }
        biped::prepare_ears(self);
        self.draws(&[(biped::EARS, RenderMode::Render)], scale)
            .pop()
    }

    /// `ModelBiped.renderCloak(scale)` (`None` if hidden or not a biped).
    // src: ModelBiped.java:172-174
    pub fn render_cloak(&self, scale: f32) -> Option<PartDraw> {
        if !self.kind.is_biped() {
            return None;
        }
        self.draws(&[(biped::CLOAK, RenderMode::Render)], scale)
            .pop()
    }

    /// `part.postRender(scale)`'s transform (the held item's and the
    /// helmet block's anchor: `bipedRightArm`, `bipedHead`), or `None`
    /// when the part is hidden (`postRender` then changes nothing: the
    /// caller uses the identity and still draws the item, it does not skip
    /// it).
    // src: ModelRenderer.java:150-178, RenderBiped.java:17, RenderPlayer.java:110, :182
    pub fn post_render(&self, part: usize, scale: f32) -> Option<Transform> {
        let p = self.parts.get(part)?;
        (!p.is_hidden && p.show_model).then(|| p.transform(RenderMode::Render, scale))
    }
}

/// `fn part_transform(part) -> Transform` for the common draw method.
pub fn part_transform(part: &ModelPart) -> Transform {
    part.transform(RenderMode::Render, MODEL_SCALE)
}

/// The meshes of every part of every model, built once (the display lists,
/// compiled at [`MODEL_SCALE`]), keyed by kind; `None` for a part without a
/// box.
#[derive(Resource, Default)]
pub struct ModelMeshes {
    map: BTreeMap<ModelKind, Vec<Option<Handle<Mesh>>>>,
}

impl ModelMeshes {
    /// The part meshes of `kind`, built on first use.
    pub fn get(&mut self, kind: ModelKind, meshes: &mut Assets<Mesh>) -> &[Option<Handle<Mesh>>] {
        self.map.entry(kind).or_insert_with(|| {
            kind.build()
                .iter()
                .map(|p| p.mesh(MODEL_SCALE).map(|m| meshes.add(m)))
                .collect()
        })
    }
}

/// A model drawn as Bevy entities: the root (carrying the renderer's
/// matrix) and one child per part with a box (`parts[i]`, the part's
/// display list; `None` for a part without one).
#[derive(Component, Clone, Debug)]
pub struct ModelEntity {
    /// The model.
    pub kind: ModelKind,
    /// The part entities, by part index.
    pub parts: Vec<Option<Entity>>,
}

/// A part entity of a [`ModelEntity`].
#[derive(Component, Clone, Copy, Debug)]
pub struct ModelPartEntity {
    /// The part index.
    pub index: usize,
}

/// Spawns `kind` as a root entity at `root` with one hidden child per part
/// (its shared mesh, `material`); [`apply_pose`] then shows and places the
/// drawn ones each frame.
pub fn spawn_model(
    commands: &mut Commands,
    cache: &mut ModelMeshes,
    meshes: &mut Assets<Mesh>,
    kind: ModelKind,
    material: Handle<ModelMaterial>,
    root: Transform,
) -> Entity {
    let handles = cache.get(kind, meshes).to_vec();
    let root_id = commands.spawn((root, Visibility::Visible)).id();
    let parts = handles
        .into_iter()
        .enumerate()
        .map(|(index, h)| {
            h.map(|mesh| {
                commands
                    .spawn((
                        Mesh3d(mesh),
                        MeshMaterial3d(material.clone()),
                        Transform::IDENTITY,
                        Visibility::Hidden,
                        ModelPartEntity { index },
                        ChildOf(root_id),
                    ))
                    .id()
            })
        })
        .collect();
    commands.entity(root_id).insert(ModelEntity { kind, parts });
    root_id
}

/// Shows the parts in `draws` with their transforms and hides the rest.
pub fn apply_pose(
    model: &ModelEntity,
    draws: &[PartDraw],
    parts: &mut Query<(&mut Transform, &mut Visibility), With<ModelPartEntity>>,
) {
    for (i, e) in model.parts.iter().enumerate() {
        let Some(e) = e else {
            continue;
        };
        let Ok((mut t, mut v)) = parts.get_mut(*e) else {
            continue;
        };
        match draws.iter().find(|d| d.part == i) {
            Some(d) => {
                *t = d.transform;
                *v = Visibility::Inherited;
            }
            None => *v = Visibility::Hidden,
        }
    }
}

/// The WP-3.5a plugin: the material, the mesh cache, the `debug_model`
/// dev action.
pub struct ModelPlugin;

impl Plugin for ModelPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ModelMaterialPlugin)
            .init_resource::<ModelMeshes>()
            .init_resource::<debug::DebugModelRequest>()
            .add_systems(
                Update,
                (debug::spawn_debug_model, debug::update_debug_model)
                    .chain()
                    .after(crate::GameFrameSet)
                    .after(crate::textures::TexturesSet),
            );
    }
}

#[cfg(test)]
mod tests;
