//! `GuiContainer` (GuiContainer.java) and the screens built on it —
//! `GuiInventory`, `GuiChest`, `GuiFurnace`, `GuiCrafting`, `GuiDispenser`
//! — plus the in-world screens `GuiEditSign`, `GuiSleepMP` and
//! `GuiGameOver`, and the opening of the screens the player asks for
//! (`EntityPlayerSP.displayGUI*`, [`open_player_gui`]).
//!
//! # The container
//!
//! The screen owns its `Container` (`inventorySlots`), built from
//! `mc-sim`'s container constructors; the simulation keeps no
//! `EntityPlayer.inventorySlots` / `craftingInventory` (WP-2.5b), so:
//!
//! - `GuiInventory`'s `player.inventorySlots` is a new
//!   `ContainerPlayer(inventory, !multiplayerWorld)` per screen. Vanilla's
//!   persists, but its grid is emptied every time the screen closes
//!   (`ContainerPlayer.onCraftGuiClosed`) and its result follows the grid,
//!   so a new one is the same container. Built with the player at the
//!   screen's first `initGui` (the registry makes screens without
//!   arguments), where vanilla's constructor also runs
//!   `addStat(openInventory, 1)`.
//! - `playerController.handleMouseClick(windowId, slot, button, shift,
//!   player)` is `player.craftingInventory.slotClick(…)`
//!   (PlayerController.java:96-98), and `craftingInventory` is the
//!   screen's container (`GuiContainer.initGui`, GuiContainer.java:16-19):
//!   [`Container::slot_click`] on it, with the world and the player.
//! - `EntityPlayer.onUpdate`'s `craftingInventory.isUsableByPlayer` check
//!   (EntityPlayer.java:80-84: close the screen when the chest is gone or
//!   the player walked away) runs in the screen's `updateScreen`
//!   ([`GuiContainer::update_screen`]), at the start of tick N+1, instead
//!   of right after the player's `super.onUpdate()` inside tick N's
//!   `updateEntities` (a known deviation, README "Known deviations"; the
//!   fix needs the player's `craftingInventory` in `mc-sim` or a hook
//!   after the player's update, outside this module).
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
use mc_sim::entity::{Entity, dispatch};
use mc_sim::inventory::InventoryRef;
use mc_sim::inventory::container::{
    Container, dispenser::new_container_dispenser, furnace::new_container_furnace,
    workbench::new_container_workbench,
};
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
    /// `inventorySlots` (`None` until `GuiInventory` builds it).
    // src: GuiContainer.java:10
    pub inventory_slots: Option<Container>,
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
            inventory_slots: container,
            variant,
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
        let c = self.inventory_slots.as_ref()?;
        c.slots.iter().position(|s| {
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
        if let (Some(i), Some(c)) = (var4, self.inventory_slots.as_ref()) {
            var8 = c.slots[i].slot_number;
        }
        if var7 {
            var8 = -999;
        }
        var8
    }

    /// Every slot's stack (`slot.getStack()`), read through the container
    /// with the world and the player.
    fn slot_stacks(&mut self, game: &mut Game) -> Vec<Option<ItemStack>> {
        let Some(c) = self.inventory_slots.as_mut() else {
            return Vec::new();
        };
        let n = c.slots.len();
        with_player(game, |w, p| {
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
        if let Some(c) = self.inventory_slots.as_ref() {
            for (var7, var8) in c.slots.iter().enumerate() {
                let st = stacks.get(var7).copied().flatten();
                self.draw_slot_inventory(&mut ig, g, ctx.font, &statics, var8, st.as_ref());
                if self.is_mouse_over_slot(
                    var8.x_display_position,
                    var8.y_display_position,
                    var1,
                    var2,
                ) {
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
    /// `craftingInventory = inventorySlots` (nothing to keep here) and
    /// `displayGuiScreen(null)`.
    pub fn close_screen(ctx: &mut ScreenCtx) {
        ctx.display_gui_screen(None);
    }

    /// `playerController.handleMouseClick(windowId, slot, button, shift,
    /// player)`: `slotClick` on this screen's container (module docs).
    // src: PlayerController.java:96-98
    pub fn handle_mouse_click(&mut self, game: &mut Game, slot: i32, button: i32, shift: bool) {
        let Some(c) = self.inventory_slots.as_mut() else {
            return;
        };
        let Some(pid) = game.the_player else {
            return;
        };
        if let Some(w) = game.the_world.as_mut() {
            w.with_entity_mut(pid, |w, p| {
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
            if self.inventory_slots.is_none() {
                self.inventory_slots = inventory::new_inventory_container(ctx.game);
            }
            self.base.control_list.clear();
        }
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
        let Some(c) = self.inventory_slots.as_mut() else {
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
    // src: GuiContainer.java:172-178; EntityPlayer.java:80-84
    //
    // Only the isEntityAlive/isDead test is GuiContainer.updateScreen's.
    // The isUsableByPlayer test is EntityPlayer.onUpdate's, right after
    // the player's super.onUpdate() inside tick N's updateEntities; here
    // it runs at the start of tick N+1, so the dropped cursor stack and
    // workbench grid join the world before tick N+1's updateEntities
    // (one update fewer for those EntityItems, and their entity ids follow
    // the spawns of the rest of tick N). Known deviation (module docs):
    // it moves when mc-sim keeps the player's craftingInventory or the
    // shell offers a hook after the player's update.
    fn update_screen(&mut self, ctx: &mut ScreenCtx) {
        let close = {
            let game = &mut *ctx.game;
            let multiplayer = game.is_multiplayer_world();
            let c = self.inventory_slots.as_ref();
            with_player(game, |w, p| {
                let unusable = !multiplayer && c.is_some_and(|c| !c.is_usable_by_player(w, p));
                unusable || !dispatch::is_entity_alive(p) || p.base.is_dead
            })
            .unwrap_or(false)
        };
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
/// `mc.displayGuiScreen(new Gui…(…))`. A request vanilla would have
/// thrown on (a null tile entity) or that the simulation cannot serve yet
/// (the chest minecart: `InventoryRef` has no entity variant) is logged
/// and dropped.
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
// TileEntityChest.java:45-47, InventoryLargeChest.java:18-20
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

    #[test]
    fn chest_height() {
        // GuiChest: 222 - 108 + rows * 18.
        assert_eq!(chest::chest_y_size(3), 168);
        assert_eq!(chest::chest_y_size(6), 222);
    }
}
