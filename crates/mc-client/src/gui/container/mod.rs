//! `GuiContainer` (GuiContainer.java) and the screens built on it —
//! `GuiInventory`, `GuiChest`, `GuiFurnace`, `GuiCrafting`, `GuiDispenser`
//! — plus the in-world screens `GuiEditSign`, `GuiSleepMP` and
//! `GuiGameOver`, and the opening of the screens the player asks for
//! (`EntityPlayerSP.displayGUI*`, [`open_player_gui`]).
//!
//! # The container
//!
//! The screen builds its `Container` (`inventorySlots`) from `mc-sim`'s
//! container constructors, and `initGui` hands it to the player as
//! `craftingInventory` (GuiContainer.java:16-19:
//! [`player_fns::set_crafting_inventory`]); from then on the screen works
//! on the player's object ([`GuiContainer::with_container`]), as Java's
//! screen and player share one, so:
//!
//! - `GuiInventory`'s `player.inventorySlots` is a new
//!   `ContainerPlayer(inventory, !multiplayerWorld)` per screen (the
//!   simulation keeps no persistent one, WP-2.5b). Vanilla's
//!   persists, but its grid is emptied every time the screen closes
//!   (`ContainerPlayer.onCraftGuiClosed`) and its result follows the grid,
//!   so a new one is the same container. Built with the player at the
//!   screen's first `initGui` (the registry makes screens without
//!   arguments), where vanilla's constructor also runs
//!   `addStat(openInventory, 1)`.
//! - `playerController.handleMouseClick(windowId, slot, button, shift,
//!   player)` is `player.craftingInventory.slotClick(…)`
//!   (PlayerController.java:96-98): [`Container::slot_click`] on the
//!   player's container, with the world and the player.
//! - `EntityPlayer.onUpdate`'s `craftingInventory.isUsableByPlayer` check
//!   (EntityPlayer.java:80-84: close the screen when the chest is gone or
//!   the player walked away) runs in the simulation, right after the
//!   player's `super.onUpdate()` inside the tick's `updateEntities`
//!   (`mc_sim::entity::player::on_update`, WP-4.7): the cursor stack and
//!   the grid drop there, the container leaves the player, and the
//!   screen closes when the shell drains the player's `closeScreen`
//!   request after `updateEntities`, so the next tick's input goes to the
//!   game, as in vanilla. [`GuiContainer::on_gui_closed`] then finds no
//!   container left to drop from.
//! - The slot layout (positions and numbers, fixed when a container is
//!   built) is copied for hit-testing and drawing
//!   ([`GuiContainer::slots`]).
//!
//! [`Container::slot_click`]: mc_sim::inventory::container::Container::slot_click

pub mod chest;
pub mod crafting;
pub mod dispenser;
pub mod edit_sign;
pub mod furnace;
pub mod game_over;
pub mod inventory;
pub mod sleep;

use bevy::prelude::*;
use mc_sim::entity::player::{self as player_fns};
use mc_sim::entity::{Entity, dispatch};
use mc_sim::inventory::InventoryRef;
use mc_sim::inventory::container::{
    Container, dispenser::new_container_dispenser, furnace::new_container_furnace,
    workbench::new_container_workbench,
};
use mc_sim::inventory::slot::Slot;
use mc_sim::item::ItemStack;
use mc_sim::world::World;
use mc_sim::world::client_requests::PlayerGui;

use super::batch::{GuiGl, gl};
use super::draw;
use super::font::{FontRenderer, java_trim};
use super::ingame::with_player;
use super::render_item::{ItemGl, render_item_into_gui, render_item_overlay_into_gui};
use super::screen::{
    Binder, Screen, ScreenBase, ScreenCtx, ScreenRegistry, default_draw_screen,
    default_mouse_clicked, draw_default_background,
};
use crate::app::ScreenKind;
use crate::assets::Lang;
use crate::game::Game;
use crate::keys::{KEY_ESCAPE, KEY_LSHIFT, KEY_RSHIFT};
use crate::settings::KEY_INVENTORY;

/// `/gui/items.png` (the slots' background icons).
const ITEMS_PNG: &str = super::render_item::ITEMS_PNG;

/// The hover highlight's colour, `0x80FFFFFF` (both gradient ends).
// src: GuiContainer.java:47
pub const HOVER_COLOR: i32 = -2130706433;
/// The tooltip's background, `0xC0000000`.
// src: GuiContainer.java:71
pub const TOOLTIP_COLOR: i32 = -1073741824;
/// The labels' colour (`0x404040`).
// src: GuiInventory.java:20, GuiChest.java:22
pub const LABEL_COLOR: i32 = 4210752;

/// Which `GuiContainer` subclass, with its own fields.
#[derive(Clone, Debug)]
pub enum ContainerVariant {
    /// `GuiInventory`.
    Inventory(inventory::InventoryState),
    /// `GuiChest`.
    Chest(chest::ChestState),
    /// `GuiFurnace`.
    Furnace(furnace::FurnaceState),
    /// `GuiCrafting`.
    Crafting,
    /// `GuiDispenser`.
    Dispenser,
}

/// `GuiContainer` and its subclass (module docs).
#[derive(Clone, Debug)]
pub struct GuiContainer {
    base: ScreenBase,
    /// `xSize`.
    // src: GuiContainer.java:8
    pub x_size: i32,
    /// `ySize`.
    // src: GuiContainer.java:9
    pub y_size: i32,
    /// `inventorySlots` until `initGui` hands it to the player (module
    /// docs; `None` before `GuiInventory` builds it and after the hand-over).
    // src: GuiContainer.java:10
    pub inventory_slots: Option<Container>,
    /// `inventorySlots.slots` as built: the positions and numbers the
    /// screen hit-tests and draws (they never change after construction).
    pub slots: Vec<Slot>,
    /// The subclass.
    pub variant: ContainerVariant,
}

impl GuiContainer {
    /// `new GuiContainer(container)` for `variant`.
    // src: GuiContainer.java:12-14
    pub fn new(container: Option<Container>, variant: ContainerVariant) -> Self {
        GuiContainer {
            base: ScreenBase::default(),
            x_size: 176,
            y_size: 166,
            slots: container
                .as_ref()
                .map(|c| c.slots.clone())
                .unwrap_or_default(),
            inventory_slots: container,
            variant,
        }
    }

    /// Runs `f` on the screen's container where it lives: the player's
    /// `craftingInventory` once `initGui` has handed it over (taken out of
    /// the player for the call), else the screen's own. `None` without a
    /// player, or when the player no longer has it (the simulation closed
    /// the window, `closeScreen` restored `inventorySlots`).
    pub fn with_container<R>(
        &mut self,
        game: &mut Game,
        f: impl FnOnce(&mut World, &mut Entity, &mut Container) -> R,
    ) -> Option<R> {
        if let Some(c) = self.inventory_slots.as_mut() {
            return with_player(game, |w, p| f(w, p, c));
        }
        with_player(game, |w, p| {
            let mut c = player_fns::set_crafting_inventory(p, None)?;
            let r = f(w, p, &mut c);
            if player_fns::crafting_inventory(p).is_none() {
                player_fns::set_crafting_inventory(p, Some(c));
            }
            Some(r)
        })
        .flatten()
    }

    /// `GuiContainer.initGui`'s `mc.thePlayer.craftingInventory =
    /// this.inventorySlots`: the container moves to the player (once; a
    /// later `initGui`, e.g. on a resize, finds it there).
    // src: GuiContainer.java:16-19
    fn hand_container_to_player(&mut self, game: &mut Game) {
        let Some(c) = self.inventory_slots.take() else {
            return;
        };
        let mut c = Some(c);
        with_player(game, |_, p| {
            player_fns::set_crafting_inventory(p, c.take());
        });
        // No player to take it: the screen keeps it.
        if c.is_some() {
            self.inventory_slots = c;
        }
    }

    /// `(width - xSize) / 2`, `(height - ySize) / 2`.
    pub fn origin(&self) -> (i32, i32) {
        (
            (self.base.width - self.x_size) / 2,
            (self.base.height - self.y_size) / 2,
        )
    }

    /// `getIsMouseOverSlot(slot, x, y)`: within the slot's 16×16 square
    /// grown by one pixel on each side.
    // src: GuiContainer.java:117-123
    pub fn is_mouse_over_slot(&self, sx: i32, sy: i32, mut var2: i32, mut var3: i32) -> bool {
        let (var4, var5) = self.origin();
        var2 -= var4;
        var3 -= var5;
        var2 >= sx - 1 && var2 < sx + 16 + 1 && var3 >= sy - 1 && var3 < sy + 16 + 1
    }

    /// `getSlotAtPosition(x, y)`: the first slot under the point, as its
    /// index.
    // src: GuiContainer.java:106-115
    pub fn slot_at_position(&self, var1: i32, var2: i32) -> Option<usize> {
        self.slots.iter().position(|s| {
            self.is_mouse_over_slot(s.x_display_position, s.y_display_position, var1, var2)
        })
    }

    /// The slot number `mouseClicked` passes to `handleMouseClick`: the
    /// slot's `slotNumber`, −999 outside the window, −1 for none.
    // src: GuiContainer.java:128-139
    pub fn click_target(&self, var1: i32, var2: i32) -> i32 {
        let var4 = self.slot_at_position(var1, var2);
        let (var5, var6) = self.origin();
        let var7 =
            var1 < var5 || var2 < var6 || var1 >= var5 + self.x_size || var2 >= var6 + self.y_size;
        let mut var8 = -1;
        if let Some(i) = var4 {
            var8 = self.slots[i].slot_number;
        }
        if var7 {
            var8 = -999;
        }
        var8
    }

    /// Every slot's stack (`slot.getStack()`), read through the container
    /// with the world and the player.
    fn slot_stacks(&mut self, game: &mut Game) -> Vec<Option<ItemStack>> {
        let n = self.slots.len();
        self.with_container(game, |w, p, c| {
            (0..n).map(|i| c.slot_get_stack(w, p, i)).collect()
        })
        .unwrap_or_else(|| vec![None; n])
    }

    /// `drawSlotInventory(slot)`: the stack and its overlay, or the slot's
    /// background icon when empty (none in this version).
    // src: GuiContainer.java:87-104
    fn draw_slot_inventory(
        &self,
        ig: &mut ItemGl,
        g: &mut GuiGl,
        font: &FontRenderer,
        statics: &mc_sim::block::BlockStatics,
        slot: &mc_sim::inventory::slot::Slot,
        var4: Option<&ItemStack>,
    ) {
        let (var2, var3) = (slot.x_display_position, slot.y_display_position);
        if var4.is_none() {
            let var5 = slot.get_background_icon_index();
            if var5 >= 0 {
                g.disable(gl::LIGHTING);
                g.bind_texture(ITEMS_PNG);
                draw::draw_textured_modal_rect(
                    g,
                    self.base.z_level,
                    var2,
                    var3,
                    var5 % 16 * 16,
                    var5 / 16 * 16,
                    16,
                    16,
                );
                g.enable(gl::LIGHTING);
                return;
            }
        }
        render_item_into_gui(ig, g, statics, var4, var2, var3);
        render_item_overlay_into_gui(g, font, var4, var2, var3);
    }

    /// `drawGuiContainerBackgroundLayer(partial)`.
    fn draw_background(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, partial: f32) {
        let (w, h, xs, ys) = (self.base.width, self.base.height, self.x_size, self.y_size);
        match &mut self.variant {
            ContainerVariant::Inventory(s) => s.draw_background(ctx, g, w, h, xs, ys, partial),
            ContainerVariant::Chest(s) => s.draw_background(g, w, h, xs, ys),
            ContainerVariant::Furnace(s) => s.draw_background(ctx, g, w, h, xs, ys),
            ContainerVariant::Crafting => crafting::draw_background(g, w, h, xs, ys),
            ContainerVariant::Dispenser => dispenser::draw_background(g, w, h, xs, ys),
        }
    }

    /// `drawGuiContainerForegroundLayer()`.
    fn draw_foreground(&self, ctx: &mut ScreenCtx, g: &mut GuiGl) {
        let f = ctx.font;
        match &self.variant {
            ContainerVariant::Inventory(_) => inventory::draw_foreground(g, f),
            ContainerVariant::Chest(s) => s.draw_foreground(g, f, self.y_size),
            ContainerVariant::Furnace(_) => furnace::draw_foreground(g, f, self.y_size),
            ContainerVariant::Crafting => crafting::draw_foreground(g, f, self.y_size),
            ContainerVariant::Dispenser => dispenser::draw_foreground(g, f, self.y_size),
        }
    }

    /// `GuiContainer.drawScreen(mouseX, mouseY, partial)`: the background,
    /// the slots under the item lighting with the hovered one highlighted,
    /// the held stack at the pointer, the labels, the hovered stack's name.
    // src: GuiContainer.java:21-80
    pub fn draw_container(
        &mut self,
        ctx: &mut ScreenCtx,
        g: &mut GuiGl,
        var1: i32,
        var2: i32,
        var3: f32,
    ) {
        draw_default_background(&self.base, ctx, g);
        let (var4, var5) = self.origin();
        self.draw_background(ctx, g, var3);
        let mut ig = ItemGl::new();
        ig.push_matrix(g);
        ig.rotatef(g, 120.0, 1.0, 0.0, 0.0);
        ig.enable_standard_item_lighting(g);
        ig.pop_matrix(g);
        ig.push_matrix(g);
        ig.translatef(g, var4 as f32, var5 as f32, 0.0);
        g.color4f(1.0, 1.0, 1.0, 1.0);
        ig.rescale_normal = true;
        let stacks = self.slot_stacks(ctx.game);
        let statics = ctx
            .game
            .the_world
            .as_ref()
            .map(|w| w.block_statics.clone())
            .unwrap_or_default();
        let held = ctx
            .game
            .player()
            .and_then(|p| p.kind.player())
            .and_then(|d| d.inventory.get_item_stack());
        let mut var6: Option<usize> = None;
        for (var7, var8) in self.slots.iter().enumerate() {
            let st = stacks.get(var7).copied().flatten();
            self.draw_slot_inventory(&mut ig, g, ctx.font, &statics, var8, st.as_ref());
            if self.is_mouse_over_slot(var8.x_display_position, var8.y_display_position, var1, var2)
            {
                var6 = Some(var7);
                g.disable(gl::LIGHTING);
                g.disable(gl::DEPTH_TEST);
                let (var9, var10) = (var8.x_display_position, var8.y_display_position);
                draw::draw_gradient_rect(
                    g,
                    var9,
                    var10,
                    var9 + 16,
                    var10 + 16,
                    HOVER_COLOR,
                    HOVER_COLOR,
                );
                g.enable(gl::LIGHTING);
                g.enable(gl::DEPTH_TEST);
            }
        }
        if let Some(h) = held.as_ref() {
            ig.translatef(g, 0.0, 0.0, 32.0);
            render_item_into_gui(
                &mut ig,
                g,
                &statics,
                Some(h),
                var1 - var4 - 8,
                var2 - var5 - 8,
            );
            render_item_overlay_into_gui(g, ctx.font, Some(h), var1 - var4 - 8, var2 - var5 - 8);
        }
        ig.rescale_normal = false;
        ig.disable_standard_item_lighting(g);
        g.disable(gl::LIGHTING);
        g.disable(gl::DEPTH_TEST);
        self.draw_foreground(ctx, g);
        let hovered = var6.and_then(|i| stacks.get(i).copied().flatten());
        if held.is_none()
            && let Some(st) = hovered
        {
            let key = st.get_item_name().ok().flatten();
            // "" + translateNamedKey(name): a null name is "null.name".
            let name = ctx
                .lang
                .translate_named_key(key.as_deref().unwrap_or("null"));
            let var13 = java_trim(&name).to_string();
            if !var13.is_empty() {
                let var9 = var1 - var4 + 12;
                let var10 = var2 - var5 - 12;
                let var11 = ctx.font.get_string_width(&var13);
                draw::draw_gradient_rect(
                    g,
                    var9 - 3,
                    var10 - 3,
                    var9 + var11 + 3,
                    var10 + 8 + 3,
                    TOOLTIP_COLOR,
                    TOOLTIP_COLOR,
                );
                ctx.font.draw_string_with_shadow(g, &var13, var9, var10, -1);
            }
        }
        ig.pop_matrix(g);
        default_draw_screen(&mut self.base, ctx, g, var1, var2);
        g.enable(gl::LIGHTING);
        g.enable(gl::DEPTH_TEST);
    }

    /// `thePlayer.closeScreen()` (EntityPlayerSP.java:108-111):
    /// `craftingInventory = inventorySlots` (the container stays with the
    /// player until [`GuiContainer::on_gui_closed`] takes it for its drops,
    /// which is the same order: `onGuiClosed` closes `craftingInventory`
    /// after the reset, module docs) and `displayGuiScreen(null)`.
    pub fn close_screen(ctx: &mut ScreenCtx) {
        ctx.display_gui_screen(None);
    }

    /// `playerController.handleMouseClick(windowId, slot, button, shift,
    /// player)`: `slotClick` on this screen's container (module docs).
    // src: PlayerController.java:96-98
    pub fn handle_mouse_click(&mut self, game: &mut Game, slot: i32, button: i32, shift: bool) {
        let Some(pid) = game.the_player else {
            return;
        };
        let in_world = game
            .the_world
            .as_ref()
            .is_some_and(|w| w.entity(pid).is_some());
        if in_world {
            self.with_container(game, |w, p, c| {
                c.slot_click(w, p, slot, button, shift);
            });
        }
    }
}

impl Screen for GuiContainer {
    fn base(&self) -> &ScreenBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn kind(&self) -> ScreenKind {
        match self.variant {
            ContainerVariant::Inventory(_) => ScreenKind::Inventory,
            ContainerVariant::Chest(_) => ScreenKind::Other("GuiChest".into()),
            ContainerVariant::Furnace(_) => ScreenKind::Other("GuiFurnace".into()),
            ContainerVariant::Crafting => ScreenKind::Other("GuiCrafting".into()),
            ContainerVariant::Dispenser => ScreenKind::Other("GuiDispenser".into()),
        }
    }
    // src: GuiContainer.java:168-170
    fn does_gui_pause_game(&self) -> bool {
        false
    }
    // src: GuiContainer.java:16-19 (the craftingInventory assignment: module
    // docs); GuiInventory.java:22-24 (no super call)
    fn init_gui(&mut self, ctx: &mut ScreenCtx) {
        if let ContainerVariant::Inventory(_) = self.variant {
            if self.inventory_slots.is_none() && self.slots.is_empty() {
                self.inventory_slots = inventory::new_inventory_container(ctx.game);
                if let Some(c) = &self.inventory_slots {
                    self.slots = c.slots.clone();
                }
            }
            self.base.control_list.clear();
        }
        self.hand_container_to_player(ctx.game);
    }
    // src: GuiContainer.java:21-80; GuiInventory.java:30-34
    fn draw_screen(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, mx: i32, my: i32, p: f32) {
        self.draw_container(ctx, g, mx, my, p);
        if let ContainerVariant::Inventory(s) = &mut self.variant {
            s.x_size_lo = mx as f32;
            s.y_size_lo = my as f32;
        }
    }
    // src: GuiContainer.java:125-147
    fn mouse_clicked(&mut self, ctx: &mut ScreenCtx, var1: i32, var2: i32, var3: i32) {
        default_mouse_clicked(self, ctx, var1, var2, var3);
        if var3 == 0 || var3 == 1 {
            let var8 = self.click_target(var1, var2);
            if var8 != -1 {
                let var9 = var8 != -999
                    && (ctx.game.input.is_key_down(KEY_LSHIFT)
                        || ctx.game.input.is_key_down(KEY_RSHIFT));
                self.handle_mouse_click(ctx.game, var8, var3, var9);
            }
        }
    }
    // src: GuiContainer.java:149-153 (empty for every button)
    fn mouse_moved_or_up(&mut self, _ctx: &mut ScreenCtx, _x: i32, _y: i32, _button: i32) {}
    // src: GuiContainer.java:155-160
    fn key_typed(&mut self, ctx: &mut ScreenCtx, _ch: char, var2: i32) {
        if var2 == KEY_ESCAPE || var2 == ctx.game.game_settings.key(KEY_INVENTORY) {
            GuiContainer::close_screen(ctx);
        }
    }
    // src: GuiContainer.java:162-166; PlayerController.java:100-103;
    // GuiCrafting.java:10-13; EntityPlayerSP.java:108-111;
    // EntityPlayer.java:131-133
    fn on_gui_closed(&mut self, ctx: &mut ScreenCtx) {
        // The container this screen handed to the player, taken back
        // (craftingInventory = inventorySlots); none when the simulation's
        // isUsableByPlayer close already took it and made the drops
        // (module docs).
        let installed = self.inventory_slots.is_none();
        let Some(mut c) = (if installed {
            with_player(ctx.game, |_, p| player_fns::set_crafting_inventory(p, None)).flatten()
        } else {
            self.inventory_slots.take()
        }) else {
            return;
        };
        // Every close through thePlayer.closeScreen() (Escape, the
        // inventory key, updateScreen, the isUsableByPlayer check) first
        // resets craftingInventory to inventorySlots (EntityPlayer.java:
        // 131-133) and only then calls displayGuiScreen(null)
        // (EntityPlayerSP.java:108-111). This onGuiClosed's
        // playerController.onCraftGuiClosed therefore runs
        // ContainerPlayer.onCraftGuiClosed: it drops the cursor stack
        // (Container.java:152-159) and the 2x2 grid, which is always empty
        // here (every GuiInventory close emptied it). GuiCrafting's own
        // inventorySlots.onCraftGuiClosed (GuiCrafting.java:10-13) is then
        // the only ContainerWorkbench call: the cursor is already null, the
        // 3x3 grid is dropped. One call on the screen's container gives
        // the same drops in the same order: the cursor stack, then the
        // grid.
        //
        // Deviation (README, "Known deviations"): a direct
        // displayGuiScreen(null) that skips closeScreen, the portal entry
        // with a GuiCrafting open (EntityPlayerSP.java:45-47), leaves
        // craftingInventory on the ContainerWorkbench, so vanilla drops the
        // 3x3 grid twice: the second EntityItems hold the same ItemStack
        // objects as the first, which become empty "ghosts" once one of
        // them is picked up, and take entity ids and player.rand draws.
        // The simulation's stacks are values, so a second call would
        // duplicate the items; it drops once.
        with_player(ctx.game, |w, p| c.on_craft_gui_closed(w, p));
    }
    // src: GuiContainer.java:172-178
    //
    // The isUsableByPlayer close is EntityPlayer.onUpdate's, in the
    // simulation (module docs).
    fn update_screen(&mut self, ctx: &mut ScreenCtx) {
        let close = with_player(ctx.game, |_, p| {
            !dispatch::is_entity_alive(p) || p.base.is_dead
        })
        .unwrap_or(false);
        if close {
            GuiContainer::close_screen(ctx);
        }
    }
}

/// The screens [`open_player_gui`] has not opened yet (filled by
/// [`super::ingame::collect_gui_messages`]).
#[derive(Resource, Default, Debug)]
pub struct PendingPlayerGuis(pub Vec<PlayerGui>);

/// `EntityPlayerSP.displayGUIChest` / `displayWorkbenchGUI` /
/// `displayGUIFurnace` / `displayGUIDispenser` / `displayGUIEditSign`:
/// `mc.displayGuiScreen(new Gui…(…))`. A chest minecart opens a `GuiChest`
/// over the cart itself ([`InventoryRef::Entity`], WP-4.6). A request
/// vanilla would have thrown on (a null tile entity) is logged and dropped.
// src: EntityPlayerSP.java:113-131
pub fn open_player_gui(ctx: &mut ScreenCtx, gui: PlayerGui) {
    let screen: Box<dyn Screen> = match gui {
        PlayerGui::Chest {
            inventory: Some(inv),
            ..
        } => {
            let Some(world) = ctx.game.the_world.as_ref() else {
                return;
            };
            Box::new(chest::new_gui_chest(world, inv))
        }
        // src: EntityMinecart.java:774, EntityPlayerSP.java:117-119
        // (`displayGUIChest(this)`: the cart is the IInventory)
        PlayerGui::MinecartChest { entity_id } => {
            let Some(world) = ctx.game.the_world.as_ref() else {
                return;
            };
            Box::new(chest::new_gui_chest(world, InventoryRef::Entity(entity_id)))
        }
        PlayerGui::Workbench { x, y, z } => Box::new(GuiContainer::new(
            Some(new_container_workbench(x, y, z)),
            ContainerVariant::Crafting,
        )),
        PlayerGui::Furnace {
            tile_entity: Some(te),
            ..
        } => Box::new(GuiContainer::new(
            Some(new_container_furnace(te)),
            ContainerVariant::Furnace(furnace::FurnaceState {
                furnace_inventory: te,
            }),
        )),
        PlayerGui::Dispenser {
            tile_entity: Some(te),
            ..
        } => Box::new(GuiContainer::new(
            Some(new_container_dispenser(te)),
            ContainerVariant::Dispenser,
        )),
        PlayerGui::EditSign { tile_entity, .. } => {
            Box::new(edit_sign::GuiEditSign::new(tile_entity))
        }
        other => {
            warn!(
                "gui: cannot open {other:?} (vanilla would throw, or the simulation has no inventory for it)"
            );
            return;
        }
    };
    ctx.display_gui_screen(Some(screen));
}

/// Opens the pending screens (a system in every tick hook, before the
/// screen's input, and after the frame).
pub fn open_pending_player_guis(
    mut pending: ResMut<PendingPlayerGuis>,
    mut game: NonSendMut<Game>,
    mut binder: ResMut<Binder>,
    font: Res<FontRenderer>,
    lang: Res<Lang>,
    registry: Res<ScreenRegistry>,
) {
    if pending.0.is_empty() {
        return;
    }
    let guis = std::mem::take(&mut pending.0);
    let mut ctx = ScreenCtx::new(&mut game, &font, &lang.0, &mut binder, &registry, None);
    for gui in guis {
        ctx.sync();
        open_player_gui(&mut ctx, gui);
    }
}

/// The world and the player for a screen's drawing or input (the world
/// first: `&mut World`, `&mut Entity`).
pub fn world_and_player<R>(
    game: &mut Game,
    f: impl FnOnce(&mut World, &mut Entity) -> R,
) -> Option<R> {
    with_player(game, f)
}

/// `inventory.getInvName()` of an `IInventory` reference.
// src: InventoryPlayer.java:258-260, InventoryCrafting.java:32-34, InventoryCraftResult.java:14-16,
// TileEntityChest.java:45-47, InventoryLargeChest.java:18-20, EntityMinecart.java:752-754
pub fn inv_name(world: &World, inv: &InventoryRef) -> String {
    match inv {
        InventoryRef::Player => "Inventory".into(),
        InventoryRef::CraftMatrix => "Crafting".into(),
        InventoryRef::CraftResult => "Result".into(),
        InventoryRef::TileEntity(r) => world
            .tile_entities
            .get(r)
            .and_then(|t| mc_sim::tile_entity::get_inv_name(&t.kind))
            .unwrap_or("")
            .into(),
        InventoryRef::LargeChest(lc) => lc.get_inv_name().into(),
        InventoryRef::Entity(id) => match world.entity(*id).map(|e| &e.kind) {
            Some(mc_sim::entity::EntityKindData::Minecart(d)) => d.get_inv_name().into(),
            _ => String::new(),
        },
    }
}

/// Registers the screens the shell opens by kind and the dev ones by name:
/// `GuiInventory`, `GuiGameOver`, `GuiChat`, `GuiSleepMP`.
pub fn register(registry: &mut ScreenRegistry) {
    // src: GuiInventory.java:9-13 (allowUserInput = true)
    registry.register("GuiInventory", || {
        let mut g = GuiContainer::new(
            None,
            ContainerVariant::Inventory(inventory::InventoryState::default()),
        );
        g.base.allow_user_input = true;
        Box::new(g)
    });
    registry.register(
        "GuiGameOver",
        || Box::new(game_over::GuiGameOver::default()),
    );
    registry.register("GuiChat", || Box::new(super::chat::GuiChat::default()));
    registry.register("GuiSleepMP", || Box::new(sleep::GuiSleepMp::default()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use mc_sim::inventory::container::player::new_container_player;

    fn inv_screen() -> GuiContainer {
        let mut s = GuiContainer::new(
            Some(new_container_player(true)),
            ContainerVariant::Inventory(inventory::InventoryState::default()),
        );
        s.base.width = 427;
        s.base.height = 240;
        s
    }

    #[test]
    fn slot_hit_testing() {
        let s = inv_screen();
        // Origin: ((427 - 176) / 2, (240 - 166) / 2) = (125, 37).
        assert_eq!(s.origin(), (125, 37));
        // ContainerPlayer slot 0 (the result) at (144, 36): its square
        // grown by one pixel, 143..161 × 35..53 in window coordinates
        // shifted by the origin.
        assert_eq!(s.slot_at_position(125 + 143, 37 + 35), Some(0));
        assert_eq!(s.slot_at_position(125 + 160, 37 + 52), Some(0));
        assert_eq!(s.slot_at_position(125 + 161, 37 + 36), None);
        assert_eq!(s.slot_at_position(125 + 142, 37 + 36), None);
        // The hotbar's first slot (36) at (8, 142).
        assert_eq!(s.slot_at_position(125 + 8, 37 + 142), Some(36));
        assert_eq!(s.click_target(125 + 8, 37 + 142), 36);
        // Inside the window, no slot: -1; outside: -999.
        assert_eq!(s.click_target(125 + 2, 37 + 2), -1);
        assert_eq!(s.click_target(10, 10), -999);
        assert_eq!(s.click_target(125 + 176, 37 + 50), -999);
        assert_eq!(s.click_target(125 + 175, 37 + 165), -1);
    }

    /// WP-4.7: the container moves to the player at `initGui`
    /// (GuiContainer.java:16-19); a chest out of reach is closed by the
    /// simulation inside the player's update (EntityPlayer.java:80-84),
    /// which drops the cursor stack there, so the screen's own close finds
    /// nothing left to drop; an Escape close drops the cursor stack once.
    #[test]
    fn the_chest_window_closes_in_the_players_update() {
        use crate::gui::screen::Binder;
        use crate::lang::StringTranslate;
        use crate::paths::ClientPaths;
        use crate::session::Session;
        use crate::storage::tests::TempDir;
        use crate::textures::ArgbImage;
        use mc_sim::entity::EntityKindData;
        use mc_sim::stats::AchievementMap;

        let dir = TempDir::new("container_close");
        let paths = ClientPaths {
            data_dir: dir.0.clone(),
            jar: "/nonexistent.jar".into(),
            resources: "/nonexistent".into(),
        };
        let mut game = Game::new(paths, AchievementMap::default(), Session::new("T", "-")).unwrap();
        game.open_world("C", Some("5")).unwrap();
        while game.pending.is_some() {
            game.step_pending(i64::MAX);
        }
        let mut registry = ScreenRegistry::default();
        crate::gui::register_screens(&mut registry);
        let font = FontRenderer::new(
            &ArgbImage::new(128, 128),
            false,
            super::super::font::ChatAllowedCharacters::from_chars("abc".chars().collect()),
        );
        let lang = StringTranslate::default();
        let mut binder = Binder::default();
        let items = |g: &Game| {
            g.the_world
                .as_ref()
                .unwrap()
                .loaded_entity_list
                .iter()
                .filter(|s| {
                    matches!(
                        s.entity.as_deref().map(|e| &e.kind),
                        Some(EntityKindData::Item(_))
                    )
                })
                .count()
        };
        let (px, py, pz) = {
            let p = game.player().unwrap();
            (
                p.base.pos_x.floor() as i32,
                p.base.bounding_box.min_y.floor() as i32,
                p.base.pos_z.floor() as i32,
            )
        };
        let chest_at = |g: &mut Game| {
            let w = g.the_world.as_mut().unwrap();
            w.set_block(px + 2, py, pz, 54);
            let te = w.get_block_tile_entity(px + 2, py, pz).unwrap();
            PlayerGui::Chest {
                x: px + 2,
                y: py,
                z: pz,
                inventory: Some(InventoryRef::TileEntity(te)),
            }
        };
        let set_cursor = |g: &mut Game, n: i32| {
            let p = g.player_in_world_mut().unwrap();
            p.kind.player_mut().unwrap().inventory.item_stack = Some(ItemStack::new(4, n, 0));
        };

        // Walked away: closed by the simulation.
        let gui = chest_at(&mut game);
        {
            let mut ctx = ScreenCtx::new(&mut game, &font, &lang, &mut binder, &registry, None);
            open_player_gui(&mut ctx, gui);
        }
        assert!(player_fns::crafting_inventory(game.player().unwrap()).is_some());
        set_cursor(&mut game, 3);
        let before = items(&game);
        game.player_in_world_mut().unwrap().base.set_position(
            px as f64 + 20.5,
            py as f64 + 1.62,
            pz as f64 + 0.5,
        );
        game.the_world.as_mut().unwrap().update_entities();
        game.collect_world_events();
        assert!(game.current_screen.is_none());
        assert_eq!(items(&game), before + 1);
        ScreenCtx::new(&mut game, &font, &lang, &mut binder, &registry, None).sync();
        assert!(binder.screen().is_none());
        assert_eq!(items(&game), before + 1);
        assert!(player_fns::crafting_inventory(game.player().unwrap()).is_none());

        // Escape: the screen takes the container back and drops once.
        game.player_in_world_mut().unwrap().base.set_position(
            px as f64 + 0.5,
            py as f64 + 1.62,
            pz as f64 + 0.5,
        );
        let gui = chest_at(&mut game);
        {
            let mut ctx = ScreenCtx::new(&mut game, &font, &lang, &mut binder, &registry, None);
            open_player_gui(&mut ctx, gui);
        }
        set_cursor(&mut game, 2);
        let before = items(&game);
        {
            let mut ctx = ScreenCtx::new(&mut game, &font, &lang, &mut binder, &registry, None);
            GuiContainer::close_screen(&mut ctx);
        }
        assert_eq!(items(&game), before + 1);
        assert!(player_fns::crafting_inventory(game.player().unwrap()).is_none());
    }

    #[test]
    fn chest_height() {
        // GuiChest: 222 - 108 + rows * 18.
        assert_eq!(chest::chest_y_size(3), 168);
        assert_eq!(chest::chest_y_size(6), 222);
    }
}
