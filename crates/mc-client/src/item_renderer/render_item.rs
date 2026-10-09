//! `ItemRenderer.renderItem(entity, stack)` (ItemRenderer.java:20-127): a
//! block whose render type is drawn in 3D (`RenderBlocks.renderItemIn3d`)
//! goes through `renderBlockOnInventory` (WP-3.3a's
//! [`render_block_as_item`]); anything else is the item's 16×16 icon
//! extruded into a 1/16-thick slab: a front and a back face, then 16 slabs
//! along x on each side and 16 along y on each side, every slab textured
//! with one texel column or row of the icon, so the alpha test cuts the
//! sprite's outline out of them.
//!
//! The geometry of one item is built once ([`ItemMeshCache`]) as a
//! Tessellator draw ([`crate::terrain::tessellator::Tessellator`]: its
//! normals carry `setNormal`'s byte packing, so the `+x` slabs are lit as
//! `-x`, vanilla bug). [`ItemDraw::matrix`] is `renderItem`'s own
//! `glTranslatef`/`glScalef`/`glRotatef`s; the caller puts it under its
//! matrix and draws with [`crate::model::ModelMaterial`] (lighting on,
//! alpha test, `glColor` = [`ItemDraw::gl_color`] when `renderItem` sets
//! one, else the caller's).
//!
//! Also here: the `Item` methods the renderers read that the simulation's
//! item table does not carry (`isFull3D`, `shouldRotateAroundWhenRendering`,
//! `getColorFromDamage`, `EntityLiving.getItemIcon`) and the
//! [`HeldItemHook`] for `RenderPlayer`/`RenderBiped` (WP-3.5b).

use std::collections::BTreeMap;

use bevy::math::Mat4;
use bevy::prelude::*;
use mc_sim::block::BlockStatics;
use mc_sim::entity::Entity;
use mc_sim::item::{self, ItemDef, ItemKind, ItemStack};

use crate::camera::GlMatrix;
use crate::terrain::blocks;
use crate::terrain::render_blocks::{render_block_as_item, render_item_in_3d};
use crate::terrain::tessellator::Tessellator;
use crate::textures::Colorizers;
use crate::textures::colorizer;

/// `Item.fishingRod.shiftedIndex` (`new ItemFishingRod(90)`).
// src: Item.java:316
pub const FISHING_ROD: i32 = 346;
/// `Item.mapItem.shiftedIndex` (`new ItemMap(102)`).
// src: Item.java:376
pub const MAP_ITEM: i32 = 358;

/// `Item.isFull3D()`: `bFull3D` (`setFull3D()`: the stick, the bone,
/// sugar), or `true` for the classes that override it (`ItemTool` and its
/// spade, pickaxe and axe, `ItemHoe`, `ItemSword`, `ItemFishingRod`).
// src: Item.java:221-223, ItemTool.java:43-45, ItemHoe.java:28-30, ItemSword.java:31-33, ItemFishingRod.java:10-12
pub fn is_full_3d(def: &ItemDef) -> bool {
    match def.kind {
        ItemKind::Spade(_)
        | ItemKind::Pickaxe(_)
        | ItemKind::Axe(_)
        | ItemKind::Hoe
        | ItemKind::Sword { .. }
        | ItemKind::FishingRod => true,
        _ => def.b_full_3d,
    }
}

/// `Item.shouldRotateAroundWhenRendering()`: only the fishing rod.
// src: Item.java:225-227, ItemFishingRod.java:14-16
pub fn should_rotate_around_when_rendering(def: &ItemDef) -> bool {
    matches!(def.kind, ItemKind::FishingRod)
}

/// `Item.getColorFromDamage(damage)`: white, except `ItemLeaves` (the
/// foliage colours by leaf type: spruce, birch, oak).
// src: Item.java:263-265, ItemLeaves.java:18-24
pub fn get_color_from_damage(def: &ItemDef, var1: i32) -> i32 {
    match def.kind {
        ItemKind::Leaves { .. } => {
            if (var1 & 1) == 1 {
                colorizer::get_foliage_color_pine()
            } else if (var1 & 2) == 2 {
                colorizer::get_foliage_color_birch()
            } else {
                colorizer::get_foliage_color_basic()
            }
        }
        _ => 16777215,
    }
}

/// `entity.getItemIcon(stack)`: `stack.getIconIndex()`, one row down (`+
/// 16`, the cast rod) for a player's fishing rod while its bobber is out.
// src: EntityLiving.java:872-874, EntityPlayer.java:802-809
pub fn get_item_icon(entity: &Entity, stack: &ItemStack) -> i32 {
    let Some(def) = item::item(stack.item_id) else {
        return 0;
    };
    let mut var2 = def.get_icon_index(stack);
    if let Some(p) = entity.kind.player()
        && stack.item_id == FISHING_ROD
        && p.fish_entity.is_some()
    {
        var2 = def.get_icon_index(stack) + 16;
    }
    var2
}

/// Whether `renderItem` draws the stack as a 3D block: `itemID < 256 &&
/// RenderBlocks.renderItemIn3d(Block.blocksList[itemID].getRenderType())`.
// src: ItemRenderer.java:22
pub fn is_block_3d(item_id: i32) -> bool {
    item_id < 256
        && blocks::def(item_id).is_some()
        && render_item_in_3d(blocks::get_render_type(item_id))
}

/// The atlas `renderItem` binds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ItemAtlas {
    /// `/terrain.png` (every id below 256).
    Terrain,
    /// `/gui/items.png`.
    Items,
}

impl ItemAtlas {
    /// The atlas of `item_id`.
    // src: ItemRenderer.java:22-30
    pub fn of(item_id: i32) -> Self {
        if item_id < 256 {
            ItemAtlas::Terrain
        } else {
            ItemAtlas::Items
        }
    }

    /// Its jar name.
    pub fn path(self) -> &'static str {
        match self {
            ItemAtlas::Terrain => crate::textures::TERRAIN_PNG,
            ItemAtlas::Items => crate::textures::ITEMS_PNG,
        }
    }
}

/// `renderItem`'s matrix for a flat item: `glTranslatef(-0, -0.3, 0)`,
/// `glScalef(1.5)`, `glRotatef(50, y)`, `glRotatef(335, z)`,
/// `glTranslatef(-0.9375, -0.0625, 0)`. Identity for a 3D block.
// src: ItemRenderer.java:38-47
pub fn flat_item_matrix() -> Mat4 {
    let mut m = GlMatrix::default();
    let var10 = 0.0f32;
    let var11 = 0.3f32;
    m.translate(-var10, -var11, 0.0);
    let var12 = 1.5f32;
    m.scale(var12, var12, var12);
    m.rotate(50.0, 0.0, 1.0, 0.0);
    m.rotate(335.0, 0.0, 0.0, 1.0);
    m.translate(-0.9375, -0.0625, 0.0);
    m.0
}

/// The extruded icon `var4` (`renderItem`'s flat branch, the six
/// Tessellator draws in order, concatenated: front, back, the 16 `-x`
/// slabs, the 16 `+x` slabs, the 16 `+y` slabs, the 16 `-y` slabs).
// src: ItemRenderer.java:32-122
pub fn extruded_icon(var4: i32) -> Tessellator {
    let mut var3 = Tessellator::default();
    let var5 = ((var4 % 16 * 16) as f32 + 0.0) / 256.0;
    let var6 = ((var4 % 16 * 16) as f32 + 15.99) / 256.0;
    let var7 = ((var4 / 16 * 16) as f32 + 0.0) / 256.0;
    let var8 = ((var4 / 16 * 16) as f32 + 15.99) / 256.0;
    let var9 = 1.0f32;
    let var13 = 0.0625f32;
    let (u5, u6, v7, v8) = (var5 as f64, var6 as f64, var7 as f64, var8 as f64);
    let d9 = var9 as f64;
    let back = (0.0 - var13) as f64;
    var3.start_drawing_quads();
    var3.set_normal(0.0, 0.0, 1.0);
    var3.add_vertex_with_uv(0.0, 0.0, 0.0, u6, v8);
    var3.add_vertex_with_uv(d9, 0.0, 0.0, u5, v8);
    var3.add_vertex_with_uv(d9, 1.0, 0.0, u5, v7);
    var3.add_vertex_with_uv(0.0, 1.0, 0.0, u6, v7);
    var3.set_normal(0.0, 0.0, -1.0);
    var3.add_vertex_with_uv(0.0, 1.0, back, u6, v7);
    var3.add_vertex_with_uv(d9, 1.0, back, u5, v7);
    var3.add_vertex_with_uv(d9, 0.0, back, u5, v8);
    var3.add_vertex_with_uv(0.0, 0.0, back, u6, v8);
    var3.set_normal(-1.0, 0.0, 0.0);
    for var14 in 0..16 {
        let var15 = var14 as f32 / 16.0;
        let var16 = (var6 + (var5 - var6) * var15 - 0.001953125) as f64;
        let var17 = (var9 * var15) as f64;
        var3.add_vertex_with_uv(var17, 0.0, back, var16, v8);
        var3.add_vertex_with_uv(var17, 0.0, 0.0, var16, v8);
        var3.add_vertex_with_uv(var17, 1.0, 0.0, var16, v7);
        var3.add_vertex_with_uv(var17, 1.0, back, var16, v7);
    }
    var3.set_normal(1.0, 0.0, 0.0);
    for var14 in 0..16 {
        let var15 = var14 as f32 / 16.0;
        let var16 = (var6 + (var5 - var6) * var15 - 0.001953125) as f64;
        let var17 = (var9 * var15 + 0.0625) as f64;
        var3.add_vertex_with_uv(var17, 1.0, back, var16, v7);
        var3.add_vertex_with_uv(var17, 1.0, 0.0, var16, v7);
        var3.add_vertex_with_uv(var17, 0.0, 0.0, var16, v8);
        var3.add_vertex_with_uv(var17, 0.0, back, var16, v8);
    }
    var3.set_normal(0.0, 1.0, 0.0);
    for var14 in 0..16 {
        let var15 = var14 as f32 / 16.0;
        let var16 = (var8 + (var7 - var8) * var15 - 0.001953125) as f64;
        let var17 = (var9 * var15 + 0.0625) as f64;
        var3.add_vertex_with_uv(0.0, var17, 0.0, u6, var16);
        var3.add_vertex_with_uv(d9, var17, 0.0, u5, var16);
        var3.add_vertex_with_uv(d9, var17, back, u5, var16);
        var3.add_vertex_with_uv(0.0, var17, back, u6, var16);
    }
    var3.set_normal(0.0, -1.0, 0.0);
    for var14 in 0..16 {
        let var15 = var14 as f32 / 16.0;
        let var16 = (var8 + (var7 - var8) * var15 - 0.001953125) as f64;
        let var17 = (var9 * var15) as f64;
        var3.add_vertex_with_uv(d9, var17, 0.0, u5, var16);
        var3.add_vertex_with_uv(0.0, var17, 0.0, u6, var16);
        var3.add_vertex_with_uv(0.0, var17, back, u6, var16);
        var3.add_vertex_with_uv(d9, var17, back, u5, var16);
    }
    var3
}

/// What one cached item mesh is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ItemMeshKey {
    /// `renderBlockOnInventory(block, meta, …)`; `leaves` is the live
    /// `Block.leaves.blockIndexInTexture` (fancy or fast).
    Block {
        /// Block id.
        id: i32,
        /// Metadata (`getItemDamage()`).
        meta: i32,
        /// The leaves' texture index when built.
        leaves: i32,
    },
    /// The extruded icon.
    Flat {
        /// Icon index.
        icon: i32,
    },
}

/// A cached item mesh: the mesh and, for a block, `renderBlockOnInventory`'s
/// `glColor4f` at brightness 1 (its colour factors; the brightness
/// multiplies them per draw).
#[derive(Clone, Debug)]
struct CachedItem {
    mesh: Handle<Mesh>,
    block_color: Option<[f32; 4]>,
}

/// The item meshes built so far (module docs).
#[derive(Resource, Default)]
pub struct ItemMeshCache {
    map: BTreeMap<ItemMeshKey, Option<CachedItem>>,
}

/// What `renderItem(entity, stack)` reads.
#[derive(Clone, Copy, Debug)]
pub struct RenderItemArgs<'a> {
    /// `stack.itemID`.
    pub item_id: i32,
    /// `stack.getItemDamage()`.
    pub damage: i32,
    /// `entity.getItemIcon(stack)` ([`get_item_icon`]).
    pub icon: i32,
    /// `entity.getEntityBrightness(1.0F)` (`renderBlockOnInventory`'s
    /// colour; unused for a flat item).
    pub brightness: f32,
    /// The world's `Block` singleton state.
    pub statics: &'a BlockStatics,
    /// The colour maps.
    pub colorizers: &'a Colorizers,
}

/// One `renderItem` draw (module docs).
#[derive(Clone, Debug, PartialEq)]
pub struct ItemDraw {
    /// The geometry (position, UV, normal; Tessellator colour bytes unused).
    pub mesh: Handle<Mesh>,
    /// The bound atlas.
    pub atlas: ItemAtlas,
    /// `renderItem`'s own matrix, to apply after the caller's.
    pub matrix: Mat4,
    /// `glColor4f` set by `renderBlockOnInventory`
    /// (`getRenderColor(meta) × brightness`); `None` leaves the caller's.
    pub gl_color: Option<[f32; 4]>,
}

impl ItemMeshCache {
    /// `renderItem(entity, stack)` (module docs); `None` when nothing is
    /// drawn (an id with no block or item).
    // src: ItemRenderer.java:20-127
    pub fn render_item(
        &mut self,
        meshes: &mut Assets<Mesh>,
        a: &RenderItemArgs,
    ) -> Option<ItemDraw> {
        if is_block_3d(a.item_id) {
            let key = ItemMeshKey::Block {
                id: a.item_id,
                meta: a.damage,
                leaves: a.statics.leaves_block_index_in_texture,
            };
            let c = self
                .map
                .entry(key)
                .or_insert_with(|| {
                    let d =
                        render_block_as_item(a.statics, a.colorizers, a.item_id, a.damage, 1.0)?;
                    Some(CachedItem {
                        mesh: meshes.add(d.tess.to_mesh()),
                        block_color: d.gl_color,
                    })
                })
                .as_ref()?;
            // glColor4f(var6 * var3, var7 * var3, var8 * var3, 1.0F)
            // src: RenderBlocks.java:3179-3185
            let gl_color = c
                .block_color
                .map(|[r, g, b, _]| [r * a.brightness, g * a.brightness, b * a.brightness, 1.0]);
            Some(ItemDraw {
                mesh: c.mesh.clone(),
                atlas: ItemAtlas::Terrain,
                matrix: Mat4::IDENTITY,
                gl_color,
            })
        } else {
            if item::item(a.item_id).is_none() && blocks::def(a.item_id).is_none() {
                return None;
            }
            let key = ItemMeshKey::Flat { icon: a.icon };
            let c = self
                .map
                .entry(key)
                .or_insert_with(|| {
                    Some(CachedItem {
                        mesh: meshes.add(extruded_icon(a.icon).to_mesh()),
                        block_color: None,
                    })
                })
                .as_ref()?;
            Some(ItemDraw {
                mesh: c.mesh.clone(),
                atlas: ItemAtlas::of(a.item_id),
                matrix: flat_item_matrix(),
                gl_color: None,
            })
        }
    }
}

/// `renderManager.itemRenderer.renderItem(entity, stack)` for the entity
/// renderers (WP-3.5b's `RenderPlayer.renderSpecials` and
/// `RenderBiped.renderEquippedItems`, RenderPlayer.java:118, :215,
/// RenderBiped.java:42): call `(hook.render_item)(&mut cache, &mut meshes,
/// &args)` under the arm's `postRender` matrix and the held-item
/// transform, then draw [`ItemDraw::mesh`] at `that · draw.matrix` with
/// [`crate::model::ModelMaterial`] on [`ItemAtlas::path`]'s texture
/// (`texture_srgb` 1, lighting on, alpha test 0.1, cull off as in
/// `RenderLiving`), `glColor` = `draw.gl_color` or the entity's brightness.
#[derive(Resource, Clone, Copy)]
pub struct HeldItemHook {
    /// `ItemRenderer.renderItem`.
    pub render_item: fn(&mut ItemMeshCache, &mut Assets<Mesh>, &RenderItemArgs) -> Option<ItemDraw>,
}

impl Default for HeldItemHook {
    fn default() -> Self {
        HeldItemHook {
            render_item: |c, m, a| c.render_item(m, a),
        }
    }
}

/// `renderManager.itemRenderer` for the entity renderers: fills WP-3.5b's
/// [`crate::render::held_item::HeldItemHook`] (inserted by
/// [`super::ItemRendererPlugin`]), so `RenderPlayer.renderSpecials` and
/// `RenderBiped.renderEquippedItems` draw the held item and the helmet
/// block in third person ([`render_item_on_entity_gl`]).
#[derive(Clone, Copy, Debug, Default)]
pub struct EntityGlItemRenderer;

impl crate::render::held_item::HeldItemRenderer for EntityGlItemRenderer {
    fn render_item(
        &self,
        gl: &mut crate::render::gl::EntityGl,
        args: &crate::render::held_item::HeldItemArgs,
    ) {
        render_item_on_entity_gl(gl, args);
    }
}

/// `ItemRenderer.renderItem(entity, stack)` transcribed on the entity
/// renderers' GL ([`crate::render::gl::EntityGl`]), under the caller's
/// matrix and `glColor` (the arm's `postRender` and the held-item
/// transform, `RenderLiving`'s brightness): a 3D block through
/// `renderBlockOnInventory` (its `glColor4f` of `getRenderColor(meta) ×
/// getEntityBrightness(1.0F)`), anything else as the extruded icon of
/// `getItemIcon` ([`extruded_icon`]) under `GL_RESCALE_NORMAL` and
/// [`flat_item_matrix`]'s calls, lit per vertex as GL lights it. The same
/// geometry the first-person hand draws from [`ItemMeshCache`].
// src: ItemRenderer.java:20-127
pub fn render_item_on_entity_gl(
    gl: &mut crate::render::gl::EntityGl,
    a: &crate::render::held_item::HeldItemArgs,
) {
    use crate::gui::batch::gl as glc;
    let var1 = a.entity;
    let var2 = a.stack;
    if item::item(var2.item_id).is_none() && blocks::def(var2.item_id).is_none() {
        // Item.itemsList[id] is null: vanilla would throw; nothing drawn.
        return;
    }
    gl.push_matrix();
    if is_block_3d(var2.item_id) {
        gl.bind_texture(crate::textures::TERRAIN_PNG);
        if let Some(d) = render_block_as_item(
            a.statics,
            a.colorizers,
            var2.item_id,
            var2.get_item_damage(),
            a.brightness,
        ) {
            if let Some([r, g, b, al]) = d.gl_color {
                gl.color4f(r, g, b, al);
            }
            gl.draw_tessellator(&d.tess);
        }
    } else {
        gl.bind_texture(ItemAtlas::of(var2.item_id).path());
        let var4 = get_item_icon(var1, var2);
        let var10 = 0.0f32;
        let var11 = 0.3f32;
        gl.enable(glc::RESCALE_NORMAL);
        gl.translatef(-var10, -var11, 0.0);
        let var12 = 1.5f32;
        gl.scalef(var12, var12, var12);
        gl.rotatef(50.0, 0.0, 1.0, 0.0);
        gl.rotatef(335.0, 0.0, 0.0, 1.0);
        gl.translatef(-0.9375, -0.0625, 0.0);
        // The six Tessellator draws (front, back, the x and y slabs) under
        // one state: drawn as one.
        gl.draw_tessellator(&extruded_icon(var4));
        gl.disable(glc::RESCALE_NORMAL);
    }
    gl.pop_matrix();
}
