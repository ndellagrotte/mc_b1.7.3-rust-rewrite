//! The held item: `RenderPlayer` and `RenderBiped` call
//! `renderManager.itemRenderer.renderItem(entity, stack)` under the hand's
//! matrix (RenderPlayer.java:118, :215, RenderBiped.java:42).
//! `ItemRenderer` is WP-3.7b's; it plugs in here.
//!
//! # Filled by WP-3.7b's item renderer (WP-3.9)
//!
//! `item_renderer::ItemRendererPlugin` inserts
//! `HeldItemHook(Some(Arc::new(item_renderer::render_item::EntityGlItemRenderer)))`:
//! `render_item(gl, args)` is `ItemRenderer.renderItem(args.entity,
//! args.stack)` transcribed on [`EntityGl`]
//! (`item_renderer::render_item::render_item_on_entity_gl`) — a 3D block
//! through `terrain::render_blocks::render_block_as_item(args.statics,
//! args.colorizers, id, meta, args.brightness)` with `gl.color4f` of its
//! `gl_color` and `gl.draw_tessellator`, anything else as the extruded icon
//! (`item_renderer::render_item::extruded_icon`, the geometry the
//! first-person hand uses) under `GL_RESCALE_NORMAL` and `renderItem`'s
//! `glTranslatef`/`glScalef`/`glRotatef`s. The current modelview is the
//! hand's (the caller's `postRender` and placement, [`super::player::
//! held_item_transform`]), the current `glColor` the entity's; lighting is
//! computed per vertex as GL does. `args.brightness` is
//! `entity.getEntityBrightness(1.0F)` (ItemRenderer.java:24). Without a
//! hook nothing is drawn (the unit tests' renderers).

use std::sync::Arc;

use bevy::prelude::*;
use mc_sim::block::BlockStatics;
use mc_sim::entity::Entity;
use mc_sim::item::ItemStack;

use super::gl::EntityGl;
use crate::textures::Colorizers;

/// What `ItemRenderer.renderItem(entity, stack)` reads.
pub struct HeldItemArgs<'a> {
    /// The holder.
    pub entity: &'a Entity,
    /// The stack (`new ItemStack(Item.stick)` while fishing).
    pub stack: &'a ItemStack,
    /// `entity.getEntityBrightness(1.0F)`.
    pub brightness: f32,
    /// The world's `Block` singleton state (for the block renderer).
    pub statics: &'a BlockStatics,
    /// The colour maps.
    pub colorizers: &'a Colorizers,
}

/// `ItemRenderer.renderItem` (module docs).
pub trait HeldItemRenderer: Send + Sync + 'static {
    /// Draws `args.stack` held by `args.entity` into `gl`.
    fn render_item(&self, gl: &mut EntityGl, args: &HeldItemArgs);
}

/// `renderManager.itemRenderer` (module docs); `None` draws nothing.
#[derive(Resource, Default, Clone)]
pub struct HeldItemHook(pub Option<Arc<dyn HeldItemRenderer>>);
