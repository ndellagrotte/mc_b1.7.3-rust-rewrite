//! `ItemRenderer` (ItemRenderer.java, WP-3.7b) and `EntityRenderer.renderHand`
//! (EntityRenderer.java:301-327): the first-person hand and held item, the
//! screen overlays (fire, inside a block, under water), the item renderer's
//! per-tick state, and `renderItem` for the entity renderers.
//!
//! | Module | Vanilla |
//! |---|---|
//! | [`render_item`] | `renderItem` (3D blocks through `renderBlockOnInventory`, the extruded icon), `Item.isFull3D` / `shouldRotateAroundWhenRendering` / `getColorFromDamage`, `getItemIcon`; [`render_item::HeldItemHook`] for WP-3.5b |
//! | [`first_person`] | `renderItemInFirstPerson` |
//! | [`overlays`] | `renderOverlays`, `renderInsideOfBlock`, `renderWarpedTextureOverlay`, `renderFireInFirstPerson` |
//! | [`map`] | `MapItemRenderer`, the map background |
//! | here | the fields, `updateEquippedItem`, `resetEquippedProgress(2)`, `renderHand`, the hand pass |
//!
//! # The frame
//!
//! `renderWorld` ends with `glClear(GL_DEPTH_BUFFER_BIT)` and `renderHand`
//! (EntityRenderer.java:567-570): the hand is drawn over the finished world
//! with its own depth, under the world's projection
//! (`gluPerspective(getFOVModifier, aspect, 0.05, farPlane·2)`; the portal
//! wobble is on the world's modelview, which `renderHand` resets) and the
//! modelview `identity · hurtCameraEffect · setupViewBobbing`. Here that is
//! a second `Camera3d` ([`HandCamera`], order 0: after the world camera,
//! before the GUI cameras) whose depth is cleared, with the identity view
//! and [`McProjection`] copied from the world camera's frame without the
//! portal distortion; the hand's meshes are on [`HAND_RENDER_LAYER`] and
//! their transforms are the Java's eye-space matrices. GL state at the
//! call: depth test (`GL_LEQUAL`) and writes on, back faces culled (the
//! clouds re-enable `GL_CULL_FACE`), blending off, alpha test 0.1, fog off.
//!
//! [`compute_hand`] reads the game in `renderHand`'s order into
//! [`HandFrame`] (data only); [`apply_hand`] turns it into entities.
//!
//! # What is drawn when
//!
//! - The hand/item: unless `thirdPersonView`, the view entity is sleeping,
//!   or `hideGUI` (F1 hides the hand too, EntityRenderer.java:313).
//! - The overlays: unless `thirdPersonView` or sleeping (also with F1).
//! - The hand's light: `enableStandardItemLighting` under the hand's
//!   modelview turned by the player's pitch and yaw
//!   ([`first_person::FirstPersonFrame::light_dirs`]); `glColor` is the
//!   light at the player's block × the item's colour.
//! - `RenderPlayer.drawFirstPersonHand` draws the right arm of
//!   `RenderPlayer.modelBipedMain`, posed by `setRotationAngles(0, …)`
//!   with `onGround` 0. That model is shared with the player's own
//!   third-person draw, whose `isRiding` is not reset afterwards: the arm
//!   keeps the riding pose of the last frame the player was drawn
//!   ([`ItemRenderer::hand_model_is_riding`]).
//!
//! # `itemToRender`
//!
//! Vanilla keeps the `ItemStack` object of the hotbar slot and compares it
//! by identity (`var2 == this.itemToRender`, ItemRenderer.java:415). The
//! simulation stores stacks by value, so identity is approximated
//! ([`same_object`]): the same slot as when it was taken, the same id, and
//! the same damage unless the item is damageable (a tool worn in place is
//! still the same object). The stack is refreshed from the slot while the
//! identity holds, as the shared object would be.

pub mod first_person;
pub mod map;
pub mod overlays;
pub mod render_item;

#[cfg(test)]
mod tests;

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::{NoFrustumCulling, RenderLayers};
use bevy::camera::{CameraOutputMode, ClearColorConfig};
use bevy::ecs::message::MessageCursor;
use bevy::math::{Mat4, Vec3, Vec4};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{BlendState, TextureFormat};
use mc_core::math::floor_double;
use mc_sim::block::{BlockStatics, Material};
use mc_sim::entity::{Entity as SimEntity, base, dispatch, living};
use mc_sim::item::{self, ItemStack};
use mc_sim::world::World as SimWorld;
use mc_sim::world::map_storage::MapCoord;

use crate::GameFrameSet;
use crate::camera::{self, CameraState, GlMatrix, McProjection};
use crate::events::ClientEvent;
use crate::game::{Game, RunTickHook};
use crate::gl_material::{GlBlend, GlKey, GlMaterial, TessBatch};
use crate::gui::batch::{GuiGl, TexRef};
use crate::gui::font::FontRenderer;
use crate::model::{
    self, MODEL_SCALE, Model, ModelKey, ModelKind, ModelMaterial, ModelMeshes, ModelParams,
};
use crate::textures::{Colorizers, TextureLoader, TexturesSet};

use first_person::{FirstPersonDraw, FirstPersonFrame, FirstPersonInput, HeldStack};
use overlays::{OverlayDraw, OverlayInput, OverlayKind};
use render_item::{ItemDraw, ItemMeshCache, RenderItemArgs};

/// The render layer of the hand pass. Nothing else uses it.
pub const HAND_RENDER_LAYER: usize = 29;
/// The hand camera's order: after the world camera
/// ([`camera::WORLD_CAMERA_ORDER`]), before the GUI cameras (1, 2).
pub const HAND_CAMERA_ORDER: isize = 0;
/// `/mob/char.png`: `getTextureForDownloadableImage(skinUrl,
/// getEntityTexture())` without the skin download (D2).
// src: ItemRenderer.java:258, EntityPlayer.java:51
pub const CHAR_PNG: &str = "/mob/char.png";
/// `glAlphaFunc(GL_GREATER, 0.1F)` (Minecraft.java:267; renderRainSnow
/// restores it, EntityRenderer.java:773).
pub const ALPHA_REF: f32 = 0.1;

/// Draw-order biases inside the hand pass's transparent phase (the map
/// image, then the overlays in `renderOverlays`' order).
pub mod order {
    /// The map's pixels (blended).
    pub const MAP_IMAGE: f32 = -1.0e4;
    /// `renderFireInFirstPerson`.
    pub const FIRE: f32 = 1.0e4;
    /// `renderInsideOfBlock` (not blended, but after the fire).
    pub const INSIDE_BLOCK: f32 = 2.0e4;
    /// `renderWarpedTextureOverlay`.
    pub const WATER: f32 = 3.0e4;
}

/// `ItemRenderer`'s fields (ItemRenderer.java:7-13) and the hand model's
/// leaked `isRiding` (module docs).
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct ItemRenderer {
    /// `itemToRender`.
    // src: ItemRenderer.java:8
    pub item_to_render: Option<ItemStack>,
    /// The hotbar slot `itemToRender`'s object was in when taken (the
    /// identity stand-in, module docs).
    pub item_to_render_slot: i32,
    /// `equippedProgress`.
    // src: ItemRenderer.java:9
    pub equipped_progress: f32,
    /// `prevEquippedProgress`.
    // src: ItemRenderer.java:10
    pub prev_equipped_progress: f32,
    /// `equippedItemSlot`.
    // src: ItemRenderer.java:13
    pub equipped_item_slot: i32,
    /// `RenderPlayer.modelBipedMain.isRiding`: the player's `isRiding()`
    /// the last frame `RenderPlayer` drew it (third person or asleep).
    pub hand_model_is_riding: bool,
}

impl Default for ItemRenderer {
    /// The field initialisers.
    fn default() -> Self {
        ItemRenderer {
            item_to_render: None,
            item_to_render_slot: -1,
            equipped_progress: 0.0,
            prev_equipped_progress: 0.0,
            equipped_item_slot: -1,
            hand_model_is_riding: false,
        }
    }
}

/// Whether `held` (taken from `held_slot`) is the same `ItemStack` object
/// as `stack` in hotbar slot `slot` (module docs, "`itemToRender`").
pub fn same_object(held: &ItemStack, held_slot: i32, stack: &ItemStack, slot: i32) -> bool {
    held_slot == slot
        && held.item_id == stack.item_id
        && (held.item_damage == stack.item_damage
            // isItemStackDamageable(): maxDamage > 0 (ItemStack.java:96-98)
            || item::item(stack.item_id).is_some_and(|d| d.max_damage > 0))
}

impl ItemRenderer {
    /// `updateEquippedItem()` for the hotbar slot `var3` (`currentItem`)
    /// holding `var2` (`getCurrentItem()`).
    // src: ItemRenderer.java:411-442
    pub fn update_equipped_item(&mut self, var3: i32, var2: Option<ItemStack>) {
        self.prev_equipped_progress = self.equipped_progress;
        let identical = match (&self.item_to_render, &var2) {
            (Some(t), Some(s)) => same_object(t, self.item_to_render_slot, s, var3),
            (None, None) => true,
            _ => false,
        };
        if identical {
            // The shared object: its current state.
            self.item_to_render = var2;
        }
        let mut var4 = self.equipped_item_slot == var3 && identical;
        if self.item_to_render.is_none() && var2.is_none() {
            var4 = true;
        }
        if let (Some(s), Some(t)) = (var2, self.item_to_render)
            && !identical
            && s.item_id == t.item_id
            && s.get_item_damage() == t.get_item_damage()
        {
            self.item_to_render = Some(s);
            self.item_to_render_slot = var3;
            var4 = true;
        }
        let var5 = 0.4f32;
        let var6 = if var4 { 1.0f32 } else { 0.0 };
        let mut var7 = var6 - self.equipped_progress;
        if var7 < -var5 {
            var7 = -var5;
        }
        if var7 > var5 {
            var7 = var5;
        }
        self.equipped_progress += var7;
        if self.equipped_progress < 0.1 {
            self.item_to_render = var2;
            self.item_to_render_slot = var3;
            self.equipped_item_slot = var3;
        }
    }

    /// `resetEquippedProgress()` / `resetEquippedProgress2()`.
    // src: ItemRenderer.java:444-450
    pub fn reset_equipped_progress(&mut self) {
        self.equipped_progress = 0.0;
    }

    /// `prevEquippedProgress + (equippedProgress - prevEquippedProgress) *
    /// partial`.
    // src: ItemRenderer.java:130
    pub fn equipped(&self, var1: f32) -> f32 {
        self.prev_equipped_progress + (self.equipped_progress - self.prev_equipped_progress) * var1
    }
}

/// The cursor over [`ClientEvent`] for the resets.
#[derive(Resource, Default)]
pub struct ResetCursor(MessageCursor<ClientEvent>);

/// Applies `itemRenderer.resetEquippedProgress(2)` calls flushed so far
/// (Minecraft.java:841, :849; they precede the tick's `updateRenderer`).
fn drain_resets(ir: &mut ItemRenderer, cursor: &mut ResetCursor, msgs: &Messages<ClientEvent>) {
    for e in cursor.0.read(msgs) {
        if let ClientEvent::ResetEquippedProgress { .. } = e {
            ir.reset_equipped_progress();
        }
    }
}

/// The player's hotbar slot and stack.
fn current_item(p: &SimEntity) -> Option<(i32, Option<ItemStack>)> {
    let inv = &p.kind.player()?.inventory;
    Some((inv.current_item, inv.get_current_item().copied()))
}

/// `itemRenderer.updateEquippedItem()` in [`RunTickHook::UpdateRenderer`]
/// (EntityRenderer.java:75).
pub fn tick_update_equipped_item(
    game: NonSend<Game>,
    mut ir: ResMut<ItemRenderer>,
    mut cursor: ResMut<ResetCursor>,
    msgs: Res<Messages<ClientEvent>>,
) {
    drain_resets(&mut ir, &mut cursor, &msgs);
    if let Some((slot, stack)) = game.player().and_then(current_item) {
        ir.update_equipped_item(slot, stack);
    }
}

/// One lit draw of the hand pass ([`ModelMaterial`]).
#[derive(Clone, Debug, PartialEq)]
pub enum HandGeometry {
    /// The player model's right arm, `/mob/char.png`.
    Arm,
    /// `renderItem`'s geometry.
    Item(ItemRequest),
    /// The map background, `/misc/mapbg.png`.
    MapBackground,
    /// The map's pixels.
    MapImage,
    /// The map markers, `/misc/mapicons.png` (map pixels).
    MapIcons(TessBatch),
    /// The map's name (the font's quads, map pixels).
    MapText(TessBatch, Option<TexRef>),
}

/// The `renderItem` call of a frame (resolved by [`apply_hand`]).
#[derive(Clone, Debug, PartialEq)]
pub struct ItemRequest {
    /// `itemID`.
    pub item_id: i32,
    /// `getItemDamage()`.
    pub damage: i32,
    /// `getItemIcon`.
    pub icon: i32,
    /// `getEntityBrightness(1.0F)`.
    pub brightness: f32,
}

/// A lit draw: geometry, eye-space matrix, `glColor`.
#[derive(Clone, Debug, PartialEq)]
pub struct HandDraw {
    /// What.
    pub geometry: HandGeometry,
    /// The modelview at the draw (for an item, before `renderItem`'s own).
    pub matrix: Mat4,
    /// `glColor4f`.
    pub color: [f32; 4],
}

/// What [`compute_hand`] read this frame (module docs).
#[derive(Resource, Clone, Debug, Default)]
pub struct HandFrame {
    /// Whether the hand pass runs (a world is drawn).
    pub active: bool,
    /// The projection (`fov` degrees, aspect, near, far).
    pub projection: Option<[f32; 4]>,
    /// The lights in eye space.
    pub lights: [Vec3; 2],
    /// `renderItemInFirstPerson`'s draws, in order.
    pub draws: Vec<HandDraw>,
    /// The world's block state for a 3D block item (copied when one is
    /// held).
    pub statics: Option<BlockStatics>,
    /// The held map's pixels and name, when a map is held.
    pub map: Option<MapFrame>,
    /// `renderOverlays`' draws, in order.
    pub overlays: Vec<OverlayDraw>,
    /// The arm's `isRiding` ([`ItemRenderer::hand_model_is_riding`]).
    pub arm_is_riding: bool,
}

/// A held map's data.
#[derive(Clone, Debug, PartialEq)]
pub struct MapFrame {
    /// `MapData.colors`.
    pub colors: Vec<u8>,
    /// `playersVisibleOnMap`.
    pub icons: Vec<MapCoord>,
    /// `mapName`.
    pub name: String,
}

/// The held stack as [`first_person`] reads it.
fn held_stack(s: &ItemStack) -> Option<HeldStack> {
    let def = item::item(s.item_id)?;
    Some(HeldStack {
        item_id: s.item_id,
        damage: s.get_item_damage(),
        color: render_item::get_color_from_damage(def, s.get_item_damage()),
        rotate_around: render_item::should_rotate_around_when_rendering(def),
        is_map: s.item_id == render_item::MAP_ITEM,
    })
}

/// `Item.mapItem.getMapData(itemToRender, theWorld)` (ItemRenderer.java:
/// 216): the stored map, or a new one whose id `getMapData` writes into the
/// stack's damage. That stack is the hotbar slot's object in Java, so the
/// new id is written to `itemToRender` and to its slot while the slot holds
/// it ([`same_object`]). (Normally the map's `onUpdate`, run on the
/// inventory every tick, has created it already.)
// src: ItemMap.java:22-38
fn map_frame(ir: &mut ItemRenderer, world: &mut SimWorld, p: &mut SimEntity) -> Option<MapFrame> {
    use mc_sim::item::behaviour::map::{get_map_data, map_name};
    let held = ir.item_to_render?;
    let data = match world.load_item_data(&map_name(held.get_item_damage())) {
        Some(d) => d,
        None => {
            let mut s = held;
            let d = get_map_data(&mut s, world);
            let slot = ir.item_to_render_slot;
            if let Some(inv) = p.kind.player_mut().map(|d| &mut d.inventory)
                && (0..9).contains(&slot)
                && let Some(cur) = inv.main_inventory[slot as usize].as_mut()
                && same_object(&held, slot, cur, slot)
            {
                cur.item_damage = s.item_damage;
            }
            ir.item_to_render = Some(s);
            d
        }
    };
    let d = data.borrow();
    Some(MapFrame {
        colors: d.colors.clone(),
        icons: d.players_visible_on_map.clone(),
        name: d.map_name.clone(),
    })
}

/// `renderHand(partial)`'s reads (module docs), for the player `p` (also
/// the view entity) in `w`.
// src: EntityRenderer.java:301-327
#[allow(clippy::too_many_arguments)]
pub fn render_hand(
    ir: &mut ItemRenderer,
    w: &mut SimWorld,
    p: &mut SimEntity,
    third_person: bool,
    hide_gui: bool,
    view_bobbing: bool,
    var1: f32,
) -> HandFrame {
    let mut frame = HandFrame {
        active: true,
        ..default()
    };
    let sleeping = p.kind.player().is_some_and(|d| d.is_player_sleeping());
    if third_person || sleeping {
        // RenderPlayer.renderPlayer → doRenderLiving sets the shared model's
        // isRiding (RenderLiving.java:29).
        ir.hand_model_is_riding = p.base.is_riding();
    }
    frame.arm_is_riding = ir.hand_model_is_riding;
    let mut m = GlMatrix::default();
    camera::hurt_camera_effect(&mut m, p, var1);
    if view_bobbing {
        camera::setup_view_bobbing(&mut m, p, var1);
    }
    if !third_person && !sleeping && !hide_gui {
        let b = &p.base;
        let held = ir.item_to_render;
        let i = FirstPersonInput {
            equipped: ir.equipped(var1),
            pitch: b.prev_rotation_pitch + (b.rotation_pitch - b.prev_rotation_pitch) * var1,
            yaw: b.prev_rotation_yaw + (b.rotation_yaw - b.prev_rotation_yaw) * var1,
            swing: living::get_swing_progress(p, var1),
            light: w.get_light_brightness(
                floor_double(b.pos_x),
                floor_double(b.pos_y),
                floor_double(b.pos_z),
            ),
            stack: held.as_ref().and_then(held_stack),
        };
        let fp: FirstPersonFrame = first_person::render_item_in_first_person(m.0, &i);
        frame.lights = fp.light_dirs();
        for d in &fp.draws {
            match *d {
                FirstPersonDraw::Arm(mat) => frame.draws.push(HandDraw {
                    geometry: HandGeometry::Arm,
                    matrix: mat,
                    color: fp.color,
                }),
                FirstPersonDraw::Item(mat) => {
                    let s = held.expect("an item draw has a stack");
                    if render_item::is_block_3d(s.item_id) {
                        frame.statics =
                            Some(crate::terrain::snapshot::render_statics(&w.block_statics));
                    }
                    frame.draws.push(HandDraw {
                        geometry: HandGeometry::Item(ItemRequest {
                            item_id: s.item_id,
                            damage: s.get_item_damage(),
                            icon: render_item::get_item_icon(p, &s),
                            brightness: base::get_entity_brightness(p, w, 1.0),
                        }),
                        matrix: mat,
                        color: fp.color,
                    });
                }
                FirstPersonDraw::Map(mat) => {
                    let Some(mf) = map_frame(ir, w, p) else {
                        continue;
                    };
                    frame.draws.push(HandDraw {
                        geometry: HandGeometry::MapBackground,
                        matrix: mat,
                        color: fp.color,
                    });
                    frame.draws.push(HandDraw {
                        geometry: HandGeometry::MapImage,
                        matrix: mat,
                        color: fp.color,
                    });
                    frame.draws.push(HandDraw {
                        geometry: HandGeometry::MapIcons(map::icon_quads(&mf.icons)),
                        matrix: mat,
                        color: fp.color,
                    });
                    frame.draws.push(HandDraw {
                        geometry: HandGeometry::MapText(TessBatch::default(), None),
                        matrix: mat * map::text_matrix(),
                        color: fp.color,
                    });
                    frame.map = Some(mf);
                }
            }
        }
    }
    if !third_person && !sleeping {
        let b = &p.base;
        let inside = if dispatch::is_entity_inside_opaque_block(p, w) {
            let statics = crate::terrain::snapshot::render_statics(&w.block_statics);
            let pos = [b.pos_x, b.pos_y, b.pos_z];
            let (width, height) = (b.width, b.height);
            overlays::inside_block_texture(
                pos,
                width,
                height,
                |x, y, z| (w.get_block_id(x, y, z), w.is_block_normal_cube(x, y, z)),
                |id| {
                    crate::terrain::blocks::def(id).map(|_| {
                        crate::terrain::blocks::get_block_texture_from_side(&statics, id, 2)
                    })
                },
            )
        } else {
            None
        };
        let input = OverlayInput {
            burning: b.is_burning(),
            inside,
            in_water: base::is_inside_of_material(p, w, Material::Water),
            brightness: base::get_entity_brightness(p, w, var1),
            yaw: b.rotation_yaw,
            pitch: b.rotation_pitch,
        };
        frame.overlays = overlays::render_overlays(&input);
    }
    frame
}

/// Fills [`HandFrame`] each frame after the shell's frame (module docs).
pub fn compute_hand(
    mut game: NonSendMut<Game>,
    mut ir: ResMut<ItemRenderer>,
    mut cursor: ResMut<ResetCursor>,
    msgs: Res<Messages<ClientEvent>>,
    cam: Res<CameraState>,
    mut frame: ResMut<HandFrame>,
    font: Option<Res<FontRenderer>>,
) {
    drain_resets(&mut ir, &mut cursor, &msgs);
    *frame = HandFrame::default();
    let Some(setup) = cam.setup else {
        return;
    };
    let g = &mut *game;
    let (Some(pid), Some(view)) = (g.the_player, g.render_view_entity) else {
        return;
    };
    // renderHand runs for the view entity; in single player it is the
    // player (Minecraft.java:1294, EntityRenderer.java:66-68).
    if pid != view {
        return;
    }
    let s = &g.game_settings;
    let (third, hide, bob) = (s.third_person_view, s.hide_gui, s.view_bobbing);
    let partial = g.timer.render_partial_ticks;
    let Some(world) = g.the_world.as_mut() else {
        return;
    };
    let Some(mut f) = world.with_entity_mut(pid, |w, p| {
        render_hand(&mut ir, w, p, third, hide, bob, partial)
    }) else {
        return;
    };
    f.projection = Some([setup.fov_y_degrees, setup.aspect, setup.near, setup.far]);
    // The map's name (MapItemRenderer.java:96-100): the font's quads.
    if let (Some(font), Some(mf)) = (font.as_deref(), f.map.as_ref()) {
        let mut gl = GuiGl::default();
        font.draw_string(&mut gl, &mf.name, 0, 0, -16777216);
        let mut batch = TessBatch::default();
        let mut tex = None;
        for d in gl.draws() {
            tex = d.texture.clone();
            for k in 0..d.batch.len() {
                batch.vertex(d.batch.positions[k], d.batch.uvs[k], d.batch.colors[k]);
            }
        }
        for d in &mut f.draws {
            if let HandGeometry::MapText(b, t) = &mut d.geometry {
                *b = batch.clone();
                *t = tex.clone();
            }
        }
    }
    *frame = f;
}

/// Marks the hand pass's camera.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct HandCamera;

/// A pooled lit draw entity.
#[derive(Component, Clone, Copy, Debug)]
pub struct HandSlot(pub usize);

/// A pooled overlay entity.
#[derive(Component, Clone, Copy, Debug)]
pub struct OverlaySlot(pub usize);

/// The hand pass's entities and the textures and meshes it owns.
#[derive(Resource, Default)]
pub struct HandPool {
    lit: Vec<(Entity, Handle<ModelMaterial>)>,
    overlays: Vec<(Entity, Handle<GlMaterial>, Handle<Mesh>)>,
    terrain_mirror: Option<Handle<Image>>,
    water: Option<Handle<Image>>,
    map_image: Option<Handle<Image>>,
    map_pixels: Vec<u8>,
    map_bg_mesh: Option<Handle<Mesh>>,
    map_image_mesh: Option<Handle<Mesh>>,
    map_icons_mesh: Option<Handle<Mesh>>,
    map_text_mesh: Option<Handle<Mesh>>,
    /// `RenderPlayer.modelBipedMain` (the arm's pose).
    model: Option<Model>,
    /// The block state a flat item's draw reads (none of it).
    default_statics: BlockStatics,
}

/// Spawns the hand camera (module docs).
pub fn spawn_hand_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Camera {
            order: HAND_CAMERA_ORDER,
            clear_color: ClearColorConfig::None,
            // As the GUI cameras: the shared main texture (world + hand)
            // replaces the window rather than blending over camera 0's copy
            // (gui::render module docs, "Output").
            output_mode: CameraOutputMode::Write {
                blend_state: Some(BlendState::REPLACE),
                clear_color: ClearColorConfig::None,
            },
            is_active: false,
            ..default()
        },
        Projection::custom(McProjection::default()),
        Transform::IDENTITY,
        RenderLayers::layer(HAND_RENDER_LAYER),
        HandCamera,
    ));
}

/// A mesh of `batch`'s quads with one normal for every vertex (the map's
/// `glNormal3f(0, 0, -1)`).
fn quads_with_normal(batch: &TessBatch, n: [f32; 3]) -> Mesh {
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    let mut b = batch.clone();
    if b.is_empty() {
        for _ in 0..4 {
            b.vertex([0.0; 3], [0.0; 2], [0.0; 4]);
        }
    }
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, b.positions.clone());
    m.insert_attribute(Mesh::ATTRIBUTE_UV_0, b.uvs.clone());
    m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![n; b.len()]);
    m.insert_indices(Indices::U32(b.quad_indices()));
    m
}

/// Whether an image samples as sRGB (the shared texture cache's).
fn is_srgb(images: &Assets<Image>, h: &Handle<Image>) -> bool {
    images
        .get(h)
        .is_some_and(|i| i.texture_descriptor.format.is_srgb())
}

/// The hand pass's [`ModelMaterial`] for a draw.
fn lit_material(
    texture: Handle<Image>,
    srgb: bool,
    color: [f32; 4],
    lights: [Vec3; 2],
    blend: GlBlend,
    alpha_ref: f32,
    sort_bias: f32,
) -> ModelMaterial {
    ModelMaterial {
        params: ModelParams {
            color: Vec4::from_array(color),
            light0: lights[0].extend(0.0),
            light1: lights[1].extend(0.0),
            alpha_ref,
            textured: 1,
            lighting: 1,
            texture_srgb: u32::from(srgb),
            ..default()
        },
        texture: Some(texture),
        key: ModelKey {
            blend,
            depth_write: true,
            depth_equal: false,
            // GL_CULL_FACE is on (module docs).
            cull: true,
            src_one: false,
        },
        sort_bias,
    }
}

/// A resolved lit draw: mesh, matrix, texture, `glColor`, blend, alpha
/// test reference, sort bias.
type LitDraw = (
    Handle<Mesh>,
    Mat4,
    Handle<Image>,
    [f32; 4],
    GlBlend,
    f32,
    f32,
);

/// The pooled entities' transform, visibility and mesh.
type SlotQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Transform,
        &'static mut Visibility,
        &'static mut Mesh3d,
    ),
    Or<(With<HandSlot>, With<OverlaySlot>)>,
>;

/// Turns [`HandFrame`] into the hand pass's entities (module docs).
#[allow(clippy::too_many_arguments)]
pub fn apply_hand(
    mut commands: Commands,
    frame: Res<HandFrame>,
    game: NonSend<Game>,
    mut pool: ResMut<HandPool>,
    mut cache: ResMut<ItemMeshCache>,
    mut model_meshes: ResMut<ModelMeshes>,
    hook: Res<render_item::HeldItemHook>,
    colorizers: Option<Res<Colorizers>>,
    mut loader: TextureLoader,
    mut meshes: ResMut<Assets<Mesh>>,
    mut lit_mats: ResMut<Assets<ModelMaterial>>,
    mut gl_mats: ResMut<Assets<GlMaterial>>,
    mut cams: Query<(&mut Camera, &mut Projection), With<HandCamera>>,
    mut slots: SlotQuery,
) {
    let pool = &mut *pool;
    if let Ok((mut cam, mut proj)) = cams.single_mut() {
        let active = frame.active && frame.projection.is_some();
        if cam.is_active != active {
            cam.is_active = active;
        }
        if let Some([fov, aspect, near, far]) = frame.projection {
            // renderHand keeps setupCameraTransform's projection; the
            // portal wobble was on the world's modelview (camera module
            // docs), so it is not here.
            let p = McProjection {
                fov: fov.to_radians(),
                aspect_ratio: aspect,
                near,
                far,
                distortion: Mat4::IDENTITY,
            };
            if let Projection::Custom(c) = &mut *proj
                && let Some(m) = c.get_mut::<McProjection>()
            {
                if *m != p {
                    *m = p;
                }
            } else {
                *proj = Projection::custom(p);
            }
        }
    }
    let layer = RenderLayers::layer(HAND_RENDER_LAYER);
    // ---- lit draws ----
    let mut n_lit = 0usize;
    let model = pool
        .model
        .get_or_insert_with(|| Model::new(ModelKind::Biped));
    for d in &frame.draws {
        let resolved: Option<LitDraw> = match &d.geometry {
            HandGeometry::Arm => {
                // RenderPlayer.drawFirstPersonHand (RenderPlayer.java:226-230).
                model.on_ground = 0.0;
                model.is_riding = frame.arm_is_riding;
                model.held_item_right = false;
                model.held_item_left = false;
                model.is_sneak = false;
                model.set_rotation_angles(0.0, 0.0, 0.0, 0.0, 0.0, MODEL_SCALE);
                let part = &model.parts[model::biped::RIGHT_ARM];
                let mesh = model_meshes.get(ModelKind::Biped, &mut meshes)[model::biped::RIGHT_ARM]
                    .clone();
                mesh.map(|mesh| {
                    (
                        mesh,
                        d.matrix * model::part_transform(part).to_matrix(),
                        loader.get(CHAR_PNG),
                        d.color,
                        GlBlend::Off,
                        ALPHA_REF,
                        0.0,
                    )
                })
            }
            HandGeometry::Item(r) => {
                let Some(col) = colorizers.as_deref() else {
                    continue;
                };
                let args = RenderItemArgs {
                    item_id: r.item_id,
                    damage: r.damage,
                    icon: r.icon,
                    brightness: r.brightness,
                    statics: frame.statics.as_ref().unwrap_or(&pool.default_statics),
                    colorizers: col,
                };
                (hook.render_item)(&mut cache, &mut meshes, &args).map(
                    |ItemDraw {
                         mesh,
                         atlas,
                         matrix,
                         gl_color,
                     }| {
                        (
                            mesh,
                            d.matrix * matrix,
                            loader.get(atlas.path()),
                            gl_color.unwrap_or(d.color),
                            GlBlend::Off,
                            ALPHA_REF,
                            0.0,
                        )
                    },
                )
            }
            HandGeometry::MapBackground => {
                let mesh = pool
                    .map_bg_mesh
                    .get_or_insert_with(|| {
                        meshes.add(quads_with_normal(&map::background_quad(), [0.0, 0.0, -1.0]))
                    })
                    .clone();
                Some((
                    mesh,
                    d.matrix,
                    loader.get(map::MAPBG_PNG),
                    d.color,
                    GlBlend::Off,
                    ALPHA_REF,
                    0.0,
                ))
            }
            HandGeometry::MapImage => {
                let Some(mf) = frame.map.as_ref() else {
                    continue;
                };
                let px = map::map_pixels(&mf.colors, game.game_settings.anaglyph);
                let img = match pool.map_image.clone() {
                    Some(h) => {
                        if pool.map_pixels != px
                            && let Some(mut i) = loader.images.get_mut(&h)
                        {
                            i.data = Some(px.clone());
                        }
                        h
                    }
                    None => {
                        // allocateAndSetupTexture(new BufferedImage(128, 128, 2)):
                        // nearest, repeat (RenderEngine.java:162-186).
                        let i = crate::textures::setup_texture(128, 128, px.clone(), false, false);
                        let mut i = i;
                        i.texture_descriptor.format = TextureFormat::Rgba8Unorm;
                        let h = loader.images.add(i);
                        pool.map_image = Some(h.clone());
                        h
                    }
                };
                pool.map_pixels = px;
                let mesh = pool
                    .map_image_mesh
                    .get_or_insert_with(|| {
                        meshes.add(quads_with_normal(&map::image_quad(), [0.0, 0.0, -1.0]))
                    })
                    .clone();
                // GL_BLEND on, GL_ALPHA_TEST off (MapItemRenderer.java:63-64).
                Some((
                    mesh,
                    d.matrix,
                    img,
                    d.color,
                    GlBlend::Alpha,
                    -1.0,
                    order::MAP_IMAGE,
                ))
            }
            HandGeometry::MapIcons(b) => {
                let m = quads_with_normal(b, [0.0, 0.0, -1.0]);
                let mesh = match pool.map_icons_mesh.clone() {
                    Some(h) => {
                        let _ = meshes.insert(&h, m);
                        h
                    }
                    None => {
                        let h = meshes.add(m);
                        pool.map_icons_mesh = Some(h.clone());
                        h
                    }
                };
                Some((
                    mesh,
                    d.matrix,
                    loader.get(map::MAPICONS_PNG),
                    d.color,
                    GlBlend::Off,
                    ALPHA_REF,
                    0.0,
                ))
            }
            HandGeometry::MapText(b, tex) => {
                let Some(tex) = tex.clone() else {
                    continue;
                };
                let handle = match tex {
                    TexRef::Name(n) => loader.get(&n),
                    TexRef::Handle(h) => h,
                };
                let color = b.colors.first().copied().unwrap_or([0.0, 0.0, 0.0, 1.0]);
                let m = quads_with_normal(b, [0.0, 0.0, -1.0]);
                let mesh = match pool.map_text_mesh.clone() {
                    Some(h) => {
                        let _ = meshes.insert(&h, m);
                        h
                    }
                    None => {
                        let h = meshes.add(m);
                        pool.map_text_mesh = Some(h.clone());
                        h
                    }
                };
                Some((mesh, d.matrix, handle, color, GlBlend::Off, ALPHA_REF, 0.0))
            }
        };
        let Some((mesh, matrix, texture, color, blend, alpha_ref, bias)) = resolved else {
            continue;
        };
        let srgb = is_srgb(&loader.images, &texture);
        let mat = lit_material(texture, srgb, color, frame.lights, blend, alpha_ref, bias);
        if n_lit == pool.lit.len() {
            let h = lit_mats.add(mat.clone());
            let e = commands
                .spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(h.clone()),
                    Transform::from_matrix(matrix),
                    Visibility::Visible,
                    layer.clone(),
                    NoFrustumCulling,
                    HandSlot(n_lit),
                ))
                .id();
            pool.lit.push((e, h));
        } else {
            let (e, h) = pool.lit[n_lit].clone();
            if let Some(mut m) = lit_mats.get_mut(&h) {
                *m = mat;
            }
            if let Ok((mut t, mut v, mut m3)) = slots.get_mut(e) {
                *t = Transform::from_matrix(matrix);
                *v = Visibility::Visible;
                if m3.0 != mesh {
                    m3.0 = mesh;
                }
            }
        }
        n_lit += 1;
    }
    for (e, _) in &pool.lit[n_lit..] {
        if let Ok((_, mut v, _)) = slots.get_mut(*e) {
            *v = Visibility::Hidden;
        }
    }
    // ---- overlays ----
    let mut n_ov = 0usize;
    for o in &frame.overlays {
        let (texture, blend, bias) = match o.kind {
            OverlayKind::Fire | OverlayKind::InsideBlock => {
                let Some(h) = crate::particles::render::sync_mirror(
                    &loader.textures,
                    &mut loader.images,
                    crate::textures::TERRAIN_PNG,
                    pool.terrain_mirror.clone(),
                ) else {
                    continue;
                };
                pool.terrain_mirror = Some(h.clone());
                if o.kind == OverlayKind::Fire {
                    (h, GlBlend::Alpha, order::FIRE)
                } else {
                    (h, GlBlend::Off, order::INSIDE_BLOCK)
                }
            }
            OverlayKind::Water => {
                let h = match pool.water.clone() {
                    Some(h) => h,
                    None => {
                        let Ok(img) = crate::gl_material::jar_texture(
                            &loader.jar,
                            overlays::WATER_PNG,
                            false,
                        ) else {
                            continue;
                        };
                        let h = loader.images.add(img);
                        pool.water = Some(h.clone());
                        h
                    }
                };
                (h, GlBlend::Alpha, order::WATER)
            }
        };
        let key = GlKey {
            blend,
            depth_write: true,
            ..default()
        };
        let mut mat = GlMaterial::new(key, bias).with_texture(texture);
        // GL_ALPHA_TEST off (ItemRenderer.java:278).
        mat.params.color = Vec4::from_array(o.color);
        let mesh_data = o.batch.quads_mesh();
        if n_ov == pool.overlays.len() {
            let mesh = meshes.add(mesh_data);
            let h = gl_mats.add(mat);
            let e = commands
                .spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(h.clone()),
                    Transform::IDENTITY,
                    Visibility::Visible,
                    layer.clone(),
                    NoFrustumCulling,
                    OverlaySlot(n_ov),
                ))
                .id();
            pool.overlays.push((e, h, mesh));
        } else {
            let (e, h, mesh) = pool.overlays[n_ov].clone();
            let _ = meshes.insert(&mesh, mesh_data);
            if let Some(mut m) = gl_mats.get_mut(&h) {
                *m = mat;
            }
            if let Ok((_, mut v, _)) = slots.get_mut(e) {
                *v = Visibility::Visible;
            }
        }
        n_ov += 1;
    }
    for (e, _, _) in &pool.overlays[n_ov..] {
        if let Ok((_, mut v, _)) = slots.get_mut(*e) {
            *v = Visibility::Hidden;
        }
    }
}

/// The WP-3.7b plugin (module docs).
pub struct ItemRendererPlugin;

impl Plugin for ItemRendererPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ItemRenderer>()
            .init_resource::<ResetCursor>()
            .init_resource::<HandFrame>()
            .init_resource::<HandPool>()
            .init_resource::<ItemMeshCache>()
            .init_resource::<render_item::HeldItemHook>()
            .add_systems(Startup, spawn_hand_camera)
            .add_systems(RunTickHook::UpdateRenderer, tick_update_equipped_item)
            .add_systems(
                Update,
                (compute_hand, apply_hand)
                    .chain()
                    .after(GameFrameSet)
                    .after(TexturesSet),
            );
    }
}
