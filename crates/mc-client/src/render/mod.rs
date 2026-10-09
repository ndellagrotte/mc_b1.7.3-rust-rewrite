//! The entity renderers (WP-3.5b): `RenderManager` and every `Render*`
//! class, the tile-entity special renderers (`TileEntityRenderer`, the sign,
//! the mob spawner, the moving piston), `RenderGlobal.renderEntities` and
//! the `EntityPickupFX` entity draws, on top of WP-3.5a's models
//! (PLAN_v4 §3.5, D7).
//!
//! | Module | Vanilla | What |
//! |---|---|---|
//! | here | `RenderManager`, `RenderGlobal.renderEntities`, `EntityRenderer.renderWorld` (the call) | the renderer map, `cacheActiveRenderInfo`, `renderEntity`, `renderEntityWithPosYaw`, the entity pass and its F3 line, [`RenderPlugin`] |
//! | [`gl`] | GL, `Tessellator`, `RenderHelper` | [`gl::EntityGl`]: the fixed-function state and matrix stack as data |
//! | [`apply`] | — | the draws as Bevy entities |
//! | [`material`] | — | [`material::EntityGlMaterial`] |
//! | [`base`], [`shadow`], [`fire`] | `Render` | `loadTexture`, `doRenderShadowAndFire`, `renderShadow`, `renderEntityOnFire`, `renderOffsetAABB`, `renderAABB` |
//! | [`living`] | `RenderLiving` | `doRenderLiving` and the hooks the subclasses override |
//! | [`player`], [`biped`], [`creeper`], [`spider`], [`sheep`], [`wolf`], [`pig`], [`cow`], [`chicken`], [`squid`], [`slime`], [`ghast`], [`giant_zombie`] | `RenderPlayer`, `RenderBiped`, … | the overrides |
//! | [`item`], [`minecart`], [`boat`], [`arrow`], [`fish`], [`tnt_primed`], [`falling_sand`], [`fireball`], [`snowball`], [`painting`], [`lightning_bolt`], [`entity`] | the non-living `Render*` | each `doRender` |
//! | [`tile_entity`], [`model_sign`], [`text3d`] | `TileEntityRenderer`, `TileEntitySpecialRenderer`, the three special renderers, `SignModel` | the tile entities; `FontRenderer` drawn in 3D |
//! | [`held_item`] | `ItemRenderer.renderItem` (the call) | [`held_item::HeldItemHook`], filled by WP-3.7b |
//!
//! # Each frame
//!
//! 1. [`render_entities`] (inside the shell's frame, after `game_frame` and
//!    before the F3 text is published, so F3's `E:` line is this frame's):
//!    `renderEntities` (RenderGlobal.java:254-307) in its order — the
//!    weather effects, the loaded entities with the range, frustum, view
//!    entity and `blockExists` checks, the tile entities of
//!    [`crate::terrain::TerrainTileEntities`] — each renderer drawing into
//!    one [`gl::EntityGl`] whose state carries from entity to entity as
//!    GL's does.
//! 2. [`render_pickups`] (after the particles' frame): `renderLitParticles`'
//!    pickups ([`crate::particles::render::PickupFxFrame`]) on the same GL
//!    state, as vanilla calls it right after `renderEntities`
//!    (EntityRenderer.java:501-502).
//! 3. [`apply::apply_draws`]: models become the per-entity model instances
//!    ([`crate::model::spawn_model`] / [`crate::model::apply_pose`], kept
//!    per sim entity id and despawned when the entity leaves the world), the
//!    Tessellator batches become pooled meshes.
//!
//! Draw order: the entities sit in the transparent band before the
//! particles ([`order`]); opaque model passes are in the opaque phase.
//!
//! # Not drawn (D2/D3)
//!
//! Skins and cloaks (`loadDownloadableImageTexture` binds the entity's own
//! texture, as it does when a download failed), `deadmau5`'s ears, other
//! players' name tags (`RenderPlayer.renderName` draws only players other
//! than `livingPlayer`: none in singleplayer).

pub mod apply;
pub mod arrow;
pub mod base;
pub mod biped;
pub mod boat;
pub mod chicken;
pub mod cow;
pub mod creeper;
pub mod dev;
pub mod entity;
pub mod falling_sand;
pub mod fire;
pub mod fireball;
pub mod fish;
pub mod ghast;
pub mod giant_zombie;
pub mod gl;
pub mod held_item;
pub mod item;
pub mod lightning_bolt;
pub mod living;
pub mod material;
pub mod minecart;
pub mod model_sign;
pub mod painting;
pub mod pig;
pub mod player;
pub mod shadow;
pub mod sheep;
pub mod slime;
pub mod snowball;
pub mod spider;
pub mod squid;
pub mod text3d;
pub mod tile_entity;
pub mod tnt_primed;
pub mod wolf;

#[cfg(test)]
mod tests;

use std::collections::BTreeSet;

use bevy::math::DVec3;
use bevy::prelude::*;
use mc_core::math::floor_double;
use mc_sim::block::BlockStatics;
use mc_sim::entity::{Entity, EntityKindData as K, base as ebase, living as eliving};
use mc_sim::world::World;

use self::base::RenderBase;
use self::gl::{EntityGl, VisualKey};
use self::held_item::{HeldItemArgs, HeldItemHook, HeldItemRenderer};
use self::living::RenderLiving;
use crate::GameFrameSet;
use crate::camera::CameraState;
use crate::debug::DebugSources;
use crate::events::ClientEvent;
use crate::game::Game;
use crate::gui::font::FontRenderer;
use crate::settings::SettingsEffect;
use crate::terrain::TerrainTileEntities;
use crate::terrain::frustum::{Frustrum, frustrum_of};
use crate::textures::Colorizers;

/// Sort biases of the entity draws in `Transparent3d` (the band before the
/// particles, [`crate::particles::render::order`]; `gl_material` module
/// docs, "Draw order").
pub mod order {
    /// Blended model passes (hurt overlay, spider eyes, slime, powered
    /// creeper): sorted by depth among themselves.
    pub const MODEL_BLEND: f32 = -0.948e6;
    /// The Tessellator batches: `TESS + n` for the `n`-th, in call order
    /// (their meshes sit at the GL origin, so the bias alone orders them).
    pub const TESS: f32 = -0.945e6;
}

/// `RenderManager`'s per-frame fields (`cacheActiveRenderInfo`) and the
/// settings the renderers read.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RenderInfo {
    /// `renderPosX/Y/Z`: the view entity's interpolated `lastTickPos`, the
    /// GL origin of the entity draws.
    pub render_pos: DVec3,
    /// `playerViewY`.
    pub player_view_y: f32,
    /// `playerViewX`.
    pub player_view_x: f32,
    /// `field_1222_l`, `field_1221_m`, `field_1220_n`: the view entity's
    /// interpolated position (`func_851_a`'s origin).
    pub viewer_pos: DVec3,
    /// `livingPlayer`'s id and position (labels' distance).
    pub living_player: Option<(i32, DVec3)>,
    /// `options.fancyGraphics`.
    pub fancy_graphics: bool,
    /// `options.thirdPersonView`.
    pub third_person_view: bool,
    /// `Minecraft.isDebugInfoEnabled()`.
    pub debug_info: bool,
    /// `Minecraft.isGuiEnabled()`.
    pub gui_enabled: bool,
    /// `Minecraft.isAmbientOcclusionEnabled()`.
    pub ambient_occlusion: bool,
    /// The anaglyph option (the block renderer's colours).
    pub anaglyph: bool,
}

impl RenderInfo {
    /// `func_851_a(x, y, z)`: the squared distance to the viewer.
    // src: RenderManager.java:125-130
    pub fn func_851_a(&self, var1: f64, var3: f64, var5: f64) -> f64 {
        let var7 = var1 - self.viewer_pos.x;
        let var9 = var3 - self.viewer_pos.y;
        let var11 = var5 - self.viewer_pos.z;
        var7 * var7 + var9 * var9 + var11 * var11
    }
}

/// What a renderer draws with (the `renderManager` fields it reaches, and
/// this crate's resources).
pub struct Ctx<'a> {
    /// The GL.
    pub gl: &'a mut EntityGl,
    /// `renderManager.worldObj`.
    pub world: &'a mut World,
    /// `RenderManager`'s frame fields.
    pub info: &'a RenderInfo,
    /// The world's `Block` singleton state, copied for the block renderer
    /// (`render_block_as_item` mutates its bounds).
    pub statics: &'a BlockStatics,
    /// The colour maps.
    pub colorizers: &'a Colorizers,
    /// `renderManager.getFontRenderer()`.
    pub font: &'a FontRenderer,
    /// `renderManager.itemRenderer` (WP-3.7b's hook; `None`: nothing drawn).
    pub held_item: Option<&'a dyn HeldItemRenderer>,
}

impl Ctx<'_> {
    /// `Render.loadTexture(name)`.
    // src: Render.java:14-17
    pub fn load_texture(&mut self, name: &'static str) {
        self.gl.bind_texture(name);
    }

    /// `entity.getEntityBrightness(partial)`.
    pub fn brightness(&mut self, e: &Entity, partial: f32) -> f32 {
        ebase::get_entity_brightness(e, self.world, partial)
    }

    /// `renderManager.itemRenderer.renderItem(living, stack)`.
    pub fn render_item(&mut self, e: &Entity, stack: &mc_sim::item::ItemStack) {
        let Some(h) = self.held_item else {
            return;
        };
        // ItemRenderer.renderItem reads getEntityBrightness(1.0F) for blocks
        // (ItemRenderer.java:24).
        let brightness = self.brightness(e, 1.0);
        let args = HeldItemArgs {
            entity: e,
            stack,
            brightness,
            statics: self.statics,
            colorizers: self.colorizers,
        };
        h.render_item(self.gl, &args);
    }

    /// `new RenderBlocks().renderBlockOnInventory(block, meta, brightness)`
    /// under the current GL state (its `glColor4f` included).
    // src: RenderBlocks.java:3174-3365
    pub fn render_block_on_inventory(&mut self, id: i32, meta: i32, brightness: f32) {
        let Some(d) = crate::terrain::render_blocks::render_block_as_item(
            self.statics,
            self.colorizers,
            id,
            meta,
            brightness,
        ) else {
            return;
        };
        if let Some([r, g, b, a]) = d.gl_color {
            self.gl.color4f(r, g, b, a);
        }
        self.gl.draw_tessellator(&d.tess);
    }
}

/// Which renderer draws an entity: `RenderManager`'s class map with the
/// superclass fallback of `getEntityClassRenderObject` (RenderManager.java:
/// 27-54, 64-72).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RendererId {
    /// `RenderSpider`.
    Spider,
    /// `RenderPig`.
    Pig,
    /// `RenderSheep`.
    Sheep,
    /// `RenderCow`.
    Cow,
    /// `RenderWolf`.
    Wolf,
    /// `RenderChicken`.
    Chicken,
    /// `RenderCreeper`.
    Creeper,
    /// `RenderBiped(new ModelSkeleton(), 0.5F)`.
    Skeleton,
    /// `RenderBiped(new ModelZombie(), 0.5F)` (also `EntityPigZombie`).
    Zombie,
    /// `RenderSlime`.
    Slime,
    /// `RenderPlayer`.
    Player,
    /// `RenderGiantZombie`.
    GiantZombie,
    /// `RenderGhast`.
    Ghast,
    /// `RenderSquid`.
    Squid,
    /// `RenderLiving(new ModelBiped(), 0.5F)` (`EntityMob` and other
    /// `EntityLiving`s).
    Living,
    /// `RenderEntity`.
    Entity,
    /// `RenderPainting`.
    Painting,
    /// `RenderArrow`.
    Arrow,
    /// `RenderSnowball(Item.snowball…)`.
    Snowball,
    /// `RenderSnowball(Item.egg…)`.
    Egg,
    /// `RenderFireball`.
    Fireball,
    /// `RenderItem`.
    Item,
    /// `RenderTNTPrimed`.
    TntPrimed,
    /// `RenderFallingSand`.
    FallingSand,
    /// `RenderMinecart`.
    Minecart,
    /// `RenderBoat`.
    Boat,
    /// `RenderFish`.
    Fish,
    /// `RenderLightningBolt`.
    LightningBolt,
}

impl RendererId {
    /// `getEntityRenderObject(entity)`: the class's renderer, else its
    /// superclass's (`EntityPigZombie` → `EntityZombie`'s, `EntityMob` →
    /// `EntityLiving`'s, `EntityPlayerSP` → `EntityPlayer`'s).
    // src: RenderManager.java:27-54, :64-76
    pub fn of(kind: &K) -> RendererId {
        match kind {
            K::Spider(..) => RendererId::Spider,
            K::Pig(..) => RendererId::Pig,
            K::Sheep(..) => RendererId::Sheep,
            K::Cow(..) => RendererId::Cow,
            K::Wolf(..) => RendererId::Wolf,
            K::Chicken(..) => RendererId::Chicken,
            K::Creeper(..) => RendererId::Creeper,
            K::Skeleton(..) => RendererId::Skeleton,
            K::Zombie(..) | K::PigZombie(..) => RendererId::Zombie,
            K::Slime(..) => RendererId::Slime,
            K::PlayerSp(..) => RendererId::Player,
            K::GiantZombie(..) => RendererId::GiantZombie,
            K::Ghast(..) => RendererId::Ghast,
            K::Squid(..) => RendererId::Squid,
            K::Mob(..) => RendererId::Living,
            K::Painting(..) => RendererId::Painting,
            K::Arrow(..) => RendererId::Arrow,
            K::Snowball(..) => RendererId::Snowball,
            K::Egg(..) => RendererId::Egg,
            K::Fireball(..) => RendererId::Fireball,
            K::Item(..) => RendererId::Item,
            K::TntPrimed(..) => RendererId::TntPrimed,
            K::FallingSand(..) => RendererId::FallingSand,
            K::Minecart(..) => RendererId::Minecart,
            K::Boat(..) => RendererId::Boat,
            K::Fish(..) => RendererId::Fish,
            K::LightningBolt(..) => RendererId::LightningBolt,
        }
    }
}

/// The renderer instances `RenderManager`'s constructor makes (one per map
/// entry; the models and their state live in them).
#[derive(Clone, Debug)]
pub struct Renderers {
    /// The `RenderLiving` subclasses, by [`RendererId`] (`living_index`).
    pub living: Vec<RenderLiving>,
    /// `RenderItem`.
    pub item: item::RenderItem,
    /// `RenderPainting`.
    pub painting: painting::RenderPainting,
    /// `RenderMinecart`.
    pub minecart: minecart::RenderMinecart,
    /// `RenderBoat`.
    pub boat: boat::RenderBoat,
}

impl Default for Renderers {
    // src: RenderManager.java:26-54
    fn default() -> Self {
        Renderers {
            living: LIVING_IDS.iter().map(|&id| RenderLiving::new(id)).collect(),
            item: item::RenderItem::default(),
            painting: painting::RenderPainting::default(),
            minecart: minecart::RenderMinecart::default(),
            boat: boat::RenderBoat::default(),
        }
    }
}

/// The living renderers, in [`Renderers::living`]'s order.
pub const LIVING_IDS: [RendererId; 15] = [
    RendererId::Spider,
    RendererId::Pig,
    RendererId::Sheep,
    RendererId::Cow,
    RendererId::Wolf,
    RendererId::Chicken,
    RendererId::Creeper,
    RendererId::Skeleton,
    RendererId::Zombie,
    RendererId::Slime,
    RendererId::Player,
    RendererId::GiantZombie,
    RendererId::Ghast,
    RendererId::Squid,
    RendererId::Living,
];

impl Renderers {
    /// The living renderer `id`, if it is one.
    pub fn living_mut(&mut self, id: RendererId) -> Option<&mut RenderLiving> {
        let i = LIVING_IDS.iter().position(|&l| l == id)?;
        self.living.get_mut(i)
    }

    /// `Render.shadowSize` and `field_194_c` of renderer `id`.
    pub fn base(&self, id: RendererId) -> RenderBase {
        if let Some(i) = LIVING_IDS.iter().position(|&l| l == id) {
            return self.living[i].base;
        }
        match id {
            RendererId::Item => self.item.base,
            RendererId::Minecart
            | RendererId::Boat
            | RendererId::TntPrimed
            | RendererId::FallingSand => {
                // src: RenderMinecart.java:9, RenderBoat.java:9, RenderTNTPrimed.java:9,
                // RenderFallingSand.java:9
                RenderBase::with_shadow(0.5)
            }
            _ => RenderBase::default(),
        }
    }
}

/// `RenderManager.instance` and `TileEntityRenderer.instance` (module docs).
/// A non-`Send` resource (the mob spawner's scratch world), as
/// [`crate::game::Game`] is.
pub struct RenderManager {
    /// The renderers.
    pub renderers: Renderers,
    /// `TileEntityRenderer`.
    pub tile_entities: tile_entity::TileEntityRenderer,
    /// This frame's `cacheActiveRenderInfo`.
    pub info: RenderInfo,
    /// `GL_NORMALIZE`, once a slime enabled it (vanilla bug kept: never
    /// disabled again; [`gl`] module docs).
    pub normalize_latched: bool,
    /// `RenderGlobal.renderEntitiesStartupCounter` (module docs of
    /// [`render_entities`]).
    pub render_entities_startup_counter: i32,
    /// The settings `loadRenderers` depends on, as last seen.
    load_renderers_key: Option<(i32, bool, bool)>,
    /// `countEntitiesTotal`, `countEntitiesRendered`, `countEntitiesHidden`.
    pub counts: (i32, i32, i32),
}

impl Default for RenderManager {
    fn default() -> Self {
        RenderManager {
            renderers: Renderers::default(),
            tile_entities: tile_entity::TileEntityRenderer::default(),
            info: RenderInfo::default(),
            normalize_latched: false,
            render_entities_startup_counter: 2,
            load_renderers_key: None,
            counts: (0, 0, 0),
        }
    }
}

impl RenderManager {
    /// `getDebugInfoEntities()`.
    // src: RenderGlobal.java:313-315
    pub fn get_debug_info_entities(&self) -> String {
        let (total, rendered, hidden) = self.counts;
        format!(
            "E: {rendered}/{total}. B: {hidden}, I: {}",
            total - hidden - rendered
        )
    }
}

/// `cacheActiveRenderInfo(world, …, livingPlayer, options, partial)`: the
/// view angles (a sleeping player looks along the bed), the viewer's
/// interpolated position.
// src: RenderManager.java:78-100
pub fn cache_active_render_info(
    info: &mut RenderInfo,
    world: &mut World,
    var4: &Entity,
    var6: f32,
) {
    let b = &var4.base;
    let sleeping = var4.kind.player().is_some_and(|p| p.is_player_sleeping());
    if sleeping {
        let (x, y, z) = (
            floor_double(b.pos_x),
            floor_double(b.pos_y),
            floor_double(b.pos_z),
        );
        let var7 = world.get_block_id(x, y, z);
        // src: Block.java (blockBed = 26)
        if var7 == 26 {
            let var8 = world.get_block_metadata(x, y, z);
            let var9 = var8 & 3;
            info.player_view_y = (var9 * 90 + 180) as f32;
            info.player_view_x = 0.0;
        }
    } else {
        info.player_view_y = b.prev_rotation_yaw + (b.rotation_yaw - b.prev_rotation_yaw) * var6;
        info.player_view_x =
            b.prev_rotation_pitch + (b.rotation_pitch - b.prev_rotation_pitch) * var6;
    }
    let p = var6 as f64;
    info.viewer_pos = DVec3::new(
        b.last_tick_pos_x + (b.pos_x - b.last_tick_pos_x) * p,
        b.last_tick_pos_y + (b.pos_y - b.last_tick_pos_y) * p,
        b.last_tick_pos_z + (b.pos_z - b.last_tick_pos_z) * p,
    );
    info.living_player = Some((b.entity_id, DVec3::new(b.pos_x, b.pos_y, b.pos_z)));
}

/// `renderEntity(entity, partial)`: the interpolated position relative to
/// `renderPos`, the interpolated yaw, `glColor3f(brightness)`, then
/// `renderEntityWithPosYaw`.
// src: RenderManager.java:102-110
pub fn render_entity(r: &mut Renderers, cx: &mut Ctx, var1: &Entity, var2: f32) {
    let b = &var1.base;
    let p = var2 as f64;
    let var3 = b.last_tick_pos_x + (b.pos_x - b.last_tick_pos_x) * p;
    let var5 = b.last_tick_pos_y + (b.pos_y - b.last_tick_pos_y) * p;
    let var7 = b.last_tick_pos_z + (b.pos_z - b.last_tick_pos_z) * p;
    let var9 = interpolate_yaw(b.prev_rotation_yaw, b.rotation_yaw, var2);
    let var10 = cx.brightness(var1, var2);
    cx.gl.color3f(var10, var10, var10);
    let rp = cx.info.render_pos;
    render_entity_with_pos_yaw(
        r,
        cx,
        var1,
        var3 - rp.x,
        var5 - rp.y,
        var7 - rp.z,
        var9,
        var2,
    );
}

/// `prevRotationYaw + (rotationYaw - prevRotationYaw) * partial`: no wrap
/// at ±180 (the entities keep their yaw continuous; a jump of 360 between
/// ticks spins the draw once, as vanilla's does).
// src: RenderManager.java:106
pub fn interpolate_yaw(prev: f32, cur: f32, partial: f32) -> f32 {
    prev + (cur - prev) * partial
}

/// `renderEntityWithPosYaw(entity, x, y, z, yaw, partial)`: the renderer's
/// `doRender`, then `doRenderShadowAndFire`.
// src: RenderManager.java:112-119
#[allow(clippy::too_many_arguments)]
pub fn render_entity_with_pos_yaw(
    r: &mut Renderers,
    cx: &mut Ctx,
    var1: &Entity,
    var2: f64,
    var4: f64,
    var6: f64,
    var8: f32,
    var9: f32,
) {
    let id = RendererId::of(&var1.kind);
    do_render(r, cx, id, var1, var2, var4, var6, var8, var9);
    let base = r.base(id);
    base::do_render_shadow_and_fire(&base, cx, var1, var2, var4, var6, var8, var9);
}

/// `Render.doRender` of renderer `id`.
#[allow(clippy::too_many_arguments)]
fn do_render(
    r: &mut Renderers,
    cx: &mut Ctx,
    id: RendererId,
    e: &Entity,
    x: f64,
    y: f64,
    z: f64,
    yaw: f32,
    partial: f32,
) {
    if let Some(l) = r.living_mut(id) {
        l.do_render(cx, e, x, y, z, yaw, partial);
        return;
    }
    match id {
        RendererId::Entity => entity::do_render(cx, e, x, y, z),
        RendererId::Painting => r.painting.do_render(cx, e, x, y, z, yaw),
        RendererId::Arrow => arrow::render_arrow(cx, e, x, y, z, partial),
        RendererId::Snowball => snowball::do_render(cx, snowball::SNOWBALL_ICON, x, y, z),
        RendererId::Egg => snowball::do_render(cx, snowball::EGG_ICON, x, y, z),
        RendererId::Fireball => fireball::do_render(cx, x, y, z),
        RendererId::Item => r.item.do_render_item(cx, e, x, y, z, partial),
        RendererId::TntPrimed => tnt_primed::do_render(cx, e, x, y, z, partial),
        RendererId::FallingSand => falling_sand::do_render(cx, e, x, y, z),
        RendererId::Minecart => r.minecart.do_render(cx, e, x, y, z, yaw, partial),
        RendererId::Boat => r.boat.do_render(cx, e, x, y, z, yaw, partial),
        RendererId::Fish => fish::do_render(cx, e, x, y, z, partial),
        RendererId::LightningBolt => lightning_bolt::do_render(cx, e, x, y, z),
        _ => {}
    }
}

/// `isInRangeToRenderVec3D(v)` with the render-only overrides the
/// simulation's dispatch leaves to the client: the lightning bolt (while
/// `lightningState >= 0`) and the egg, fireball, fish hook and snowball
/// (four times the box's edge).
// src: Entity.java:762-774, EntityLightningBolt.java:85-87, EntityEgg.java:24-28,
// EntityFireball.java:27-31, EntityFish.java:79-83, EntitySnowball.java:24-28
pub fn is_in_range_to_render_vec3d(e: &Entity, v: DVec3) -> bool {
    if let K::LightningBolt(_, l) = &e.kind {
        return l.lightning_state >= 0;
    }
    let var2 = e.base.pos_x - v.x;
    let var4 = e.base.pos_y - v.y;
    let var6 = e.base.pos_z - v.z;
    let var8 = var2 * var2 + var4 * var4 + var6 * var6;
    match e.kind {
        K::Egg(..) | K::Fireball(..) | K::Fish(..) | K::Snowball(..) => {
            let mut var3 = e.base.bounding_box.get_average_edge_length() * 4.0;
            var3 *= 64.0;
            var8 < var3 * var3
        }
        _ => ebase::is_in_range_to_render_dist(e, var8),
    }
}

/// What the entity pass hands to [`render_pickups`] and [`apply::apply_draws`].
#[derive(Resource, Default)]
pub struct EntityFrame {
    /// Whether a world is drawn this frame.
    pub visible: bool,
    /// `renderPos`: the GL origin of the draws (world space).
    pub anchor: DVec3,
    /// The GL with this frame's draws (its state as the last renderer left
    /// it).
    pub gl: EntityGl,
    /// The keys of the loaded entities (an entity not in it has left the
    /// world: its model instances are despawned).
    pub live: BTreeSet<VisualKey>,
}

/// Keeps the startup counter in step with `loadRenderers`: the terrain
/// resets it to 2 whenever it reloads its renderers (world change, the
/// `LoadRenderers` effect, a new render distance, graphics or AO setting;
/// RenderGlobal.java:168-184, :251).
fn track_load_renderers(
    rm: &mut RenderManager,
    events: &mut MessageReader<ClientEvent>,
    game: &Game,
) {
    for e in events.read() {
        match e {
            ClientEvent::WorldChanged { .. }
            | ClientEvent::Settings(SettingsEffect::LoadRenderers) => {
                rm.render_entities_startup_counter = 2;
            }
            _ => {}
        }
    }
    let s = &game.game_settings;
    let key = (s.render_distance, s.fancy_graphics, s.ambient_occlusion);
    if rm.load_renderers_key != Some(key) {
        if rm.load_renderers_key.is_some() {
            rm.render_entities_startup_counter = 2;
        }
        rm.load_renderers_key = Some(key);
    }
}

/// `RenderGlobal.renderEntities(viewPos, frustum, partial)` (module docs).
// src: RenderGlobal.java:254-307, EntityRenderer.java:497-501
#[allow(clippy::too_many_arguments)]
pub fn render_entities(
    mut game: NonSendMut<Game>,
    mut rm: NonSendMut<RenderManager>,
    mut frame: ResMut<EntityFrame>,
    tile_entities: Res<TerrainTileEntities>,
    camera: Res<CameraState>,
    font: Res<FontRenderer>,
    hook: Res<HeldItemHook>,
    colorizers: Res<Colorizers>,
    mut debug: ResMut<DebugSources>,
    mut events: MessageReader<ClientEvent>,
    mut item_renderer: Option<ResMut<crate::item_renderer::ItemRenderer>>,
) {
    let _perf = crate::perf::time(crate::perf::Phase::Entities);
    let rm = &mut *rm;
    track_load_renderers(rm, &mut events, &game);
    frame.visible = false;
    frame.live.clear();
    frame.gl = EntityGl::new(rm.normalize_latched);
    let g = &mut *game;
    let partial = g.timer.render_partial_ticks;
    let (Some(setup), Some(view_id)) = (camera.setup, g.render_view_entity) else {
        return;
    };
    if g.skip_render_world {
        return;
    }
    let settings = g.game_settings.clone();
    let Some(world) = g.the_world.as_mut() else {
        return;
    };
    for s in &world.loaded_entity_list {
        frame.live.insert(VisualKey::Entity(s.entity_id));
    }
    for s in &world.weather_effects {
        frame.live.insert(VisualKey::Weather(s.entity_id));
    }
    for p in &tile_entities.tile_entities {
        frame.live.insert(VisualKey::TileEntity(*p));
    }
    // The view entity, as the pass reads it (out of the world only between
    // a portal's two halves: nothing is drawn then).
    let Some((render_pos, var1, view_sleeping)) = world.entity(view_id).map(|view| {
        let vb = &view.base;
        let p = partial as f64;
        let render_pos = DVec3::new(
            vb.last_tick_pos_x + (vb.pos_x - vb.last_tick_pos_x) * p,
            vb.last_tick_pos_y + (vb.pos_y - vb.last_tick_pos_y) * p,
            vb.last_tick_pos_z + (vb.pos_z - vb.last_tick_pos_z) * p,
        );
        // var4.getPosition(partial) (EntityRenderer.java:501)
        let v = eliving::get_position(view, partial);
        let sleeping = view.kind.player().is_some_and(|p| p.is_player_sleeping());
        (
            render_pos,
            DVec3::new(v.x_coord, v.y_coord, v.z_coord),
            sleeping,
        )
    }) else {
        return;
    };
    frame.visible = true;
    frame.anchor = render_pos;
    if rm.render_entities_startup_counter > 0 {
        // src: RenderGlobal.java:255-257
        rm.render_entities_startup_counter -= 1;
        // With anaglyph on, vanilla calls renderEntities once per eye
        // (EntityRenderer.java:454, :501, :572-574), so the counter goes
        // down twice a frame. The port draws the first eye only; the second
        // eye's call is this decrement.
        if settings.anaglyph && rm.render_entities_startup_counter > 0 {
            rm.render_entities_startup_counter -= 1;
        }
        return;
    }
    // TileEntityRenderer.cacheActiveRenderInfo, RenderManager.cacheActiveRenderInfo
    // src: RenderGlobal.java:258-259
    world.with_entity_mut(view_id, |w, view| {
        rm.tile_entities.cache_active_render_info(w, view, partial);
        cache_active_render_info(&mut rm.info, w, view, partial);
    });
    rm.info.render_pos = render_pos;
    rm.info.fancy_graphics = settings.fancy_graphics;
    rm.info.third_person_view = settings.third_person_view;
    rm.info.debug_info = settings.show_debug_info;
    rm.info.gui_enabled = !settings.hide_gui;
    rm.info.ambient_occlusion = settings.ambient_occlusion;
    rm.info.anaglyph = settings.anaglyph;
    rm.counts = (0, 0, 0);
    // the frustum at the interpolated position (EntityRenderer.java:482-483)
    let frustum: Frustrum = frustrum_of(&setup, render_pos.to_array());
    let statics = crate::terrain::snapshot::render_statics(&world.block_statics);
    let info = rm.info.clone();
    let mut gl = std::mem::take(&mut frame.gl);
    let mut counts = (world.loaded_entity_list.len() as i32, 0, 0);
    {
        let renderers = &mut rm.renderers;
        // weather effects
        // src: RenderGlobal.java:274-281
        for i in 0..world.weather_effects.len() {
            counts.1 += 1;
            let Some(e) = world.weather_effects[i].entity.take() else {
                continue;
            };
            if is_in_range_to_render_vec3d(&e, var1) {
                gl.key = VisualKey::Weather(e.base.entity_id);
                let mut cx = Ctx {
                    gl: &mut gl,
                    world: &mut *world,
                    info: &info,
                    statics: &statics,
                    colorizers: &colorizers,
                    font: &font,
                    held_item: hook.0.as_deref(),
                };
                render_entity(renderers, &mut cx, &e, partial);
            }
            if let Some(s) = world.weather_effects.get_mut(i) {
                s.entity = Some(e);
            }
        }
        // the loaded entities
        // src: RenderGlobal.java:283-299
        for i in 0..world.loaded_entity_list.len() {
            let Some(e) = world.loaded_entity_list[i].entity.take() else {
                continue;
            };
            let b = &e.base;
            if is_in_range_to_render_vec3d(&e, var1)
                && (b.ignore_frustum_check || frustum.is_bounding_box_in_frustum(&b.bounding_box))
                && (b.entity_id != view_id || settings.third_person_view || view_sleeping)
            {
                let var8 = floor_double(b.pos_y).clamp(0, 127);
                if world.block_exists(floor_double(b.pos_x), var8, floor_double(b.pos_z)) {
                    counts.1 += 1;
                    gl.key = VisualKey::Entity(b.entity_id);
                    let mut cx = Ctx {
                        gl: &mut gl,
                        world: &mut *world,
                        info: &info,
                        statics: &statics,
                        colorizers: &colorizers,
                        font: &font,
                        held_item: hook.0.as_deref(),
                    };
                    render_entity(renderers, &mut cx, &e, partial);
                    // RenderPlayer's doRenderLiving set the shared
                    // modelBipedMain.isRiding, which the first-person arm
                    // keeps (RenderLiving.java:26; WP-3.7b's ItemRenderer).
                    if RendererId::of(&e.kind) == RendererId::Player
                        && let Some(ir) = item_renderer.as_deref_mut()
                    {
                        ir.player_model_drawn(e.base.is_riding());
                    }
                }
            }
            world.loaded_entity_list[i].entity = Some(e);
        }
        // the tile entities
        // src: RenderGlobal.java:301-303
        for &pos in &tile_entities.tile_entities {
            gl.key = VisualKey::TileEntity(pos);
            let mut cx = Ctx {
                gl: &mut gl,
                world: &mut *world,
                info: &info,
                statics: &statics,
                colorizers: &colorizers,
                font: &font,
                held_item: hook.0.as_deref(),
            };
            rm.tile_entities
                .render_tile_entity(renderers, &mut cx, pos, partial);
        }
    }
    rm.counts = counts;
    rm.normalize_latched |= gl.state.normalize;
    frame.gl = gl;
    let line = rm.get_debug_info_entities();
    if debug.entities != line {
        debug.entities = line;
    }
}

/// `renderLitParticles`' pickups: each `EntityPickupFX` draws its entity
/// with `RenderManager.renderEntityWithPosYaw` at the particle's position
/// (relative to its anchor), `glColor4f(light, light, light, 1)`, on the
/// state the entity pass left.
// src: EffectRenderer.java:89-100, EntityPickupFX.java:20-40
#[allow(clippy::too_many_arguments)]
pub fn render_pickups(
    mut game: NonSendMut<Game>,
    mut rm: NonSendMut<RenderManager>,
    mut frame: ResMut<EntityFrame>,
    pickups: Res<crate::particles::render::PickupFxFrame>,
    particles: Res<crate::particles::render::ParticlesFrame>,
    er: Res<crate::particles::EffectRenderer>,
    font: Res<FontRenderer>,
    hook: Res<HeldItemHook>,
    colorizers: Res<Colorizers>,
) {
    let _perf = crate::perf::time(crate::perf::Phase::Entities);
    if !frame.visible || pickups.draws.is_empty() {
        return;
    }
    let rm = &mut *rm;
    let Some(world) = game.the_world.as_mut() else {
        return;
    };
    let statics = crate::terrain::snapshot::render_statics(&world.block_statics);
    let info = rm.info.clone();
    let anchor = frame.anchor;
    let mut gl = std::mem::take(&mut frame.gl);
    for (n, d) in pickups.draws.iter().enumerate() {
        let Some(fx) = er.fx_layers[3].get(d.layer_index) else {
            continue;
        };
        let crate::particles::FxKind::Pickup(p) = &fx.kind else {
            continue;
        };
        let e = &p.entity_to_pick_up;
        // The particle's anchor (this frame's interpPos) to the entity GL
        // origin (renderPos, the same point).
        let off = particles.anchor.as_dvec3() - anchor;
        gl.key = VisualKey::Pickup(n);
        gl.color4f(d.brightness, d.brightness, d.brightness, 1.0);
        let mut cx = Ctx {
            gl: &mut gl,
            world: &mut *world,
            info: &info,
            statics: &statics,
            colorizers: &colorizers,
            font: &font,
            held_item: hook.0.as_deref(),
        };
        // (double)(float) of the offsets, as EntityPickupFX passes them.
        let pos = (d.pos + off).as_vec3().as_dvec3();
        render_entity_with_pos_yaw(
            &mut rm.renderers,
            &mut cx,
            e,
            pos.x,
            pos.y,
            pos.z,
            e.base.rotation_yaw,
            d.partial,
        );
    }
    rm.normalize_latched |= gl.state.normalize;
    frame.gl = gl;
}

/// The WP-3.5b plugin.
pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(material::EntityGlMaterialPlugin)
            .insert_non_send(RenderManager::default())
            .init_resource::<EntityFrame>()
            .init_resource::<HeldItemHook>()
            .init_resource::<apply::EntityVisuals>()
            .add_systems(
                Update,
                render_entities
                    .in_set(GameFrameSet)
                    .after(crate::game::game_frame)
                    .before(crate::debug::publish_debug_text),
            )
            .add_systems(
                Update,
                (render_pickups, apply::apply_draws)
                    .chain()
                    .after(GameFrameSet)
                    .after(crate::particles::render::compute_particles)
                    .after(crate::sky::compute_render_frame)
                    .after(crate::textures::TexturesSet),
            );
    }
}
