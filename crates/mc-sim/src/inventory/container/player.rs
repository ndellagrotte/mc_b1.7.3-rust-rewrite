//! `ContainerPlayer` (ContainerPlayer.java): the inventory window every
//! player has (`EntityPlayer.inventorySlots`, EntityPlayer.java:42).
//!
//! Slots: 0 the 2×2 grid's output, 1–4 the grid, 5–8 the armour (helmet
//! first: `InventoryPlayer` slots 39 down to 36), 9–35 the main inventory
//! (slots 9–35), 36–44 the hotbar (slots 0–8).

use super::{Container, ContainerKind};
use crate::entity::Entity;
use crate::inventory::InventoryRef;
use crate::inventory::crafting::{InventoryCraftResult, InventoryCrafting};
use crate::inventory::slot::Slot;
use crate::item::ItemStack;
use crate::world::World;

/// `new ContainerPlayer(inventory, isSinglePlayer)` (the one-argument
/// constructor passes true): the slots, then `onCraftMatrixChanged` (the
/// empty grid's result, null).
// src: ContainerPlayer.java:8-42
pub fn new_container_player(var2: bool) -> Container {
    let mut c = Container::empty(ContainerKind::Player {
        craft_matrix: InventoryCrafting::new(2, 2),
        craft_result: InventoryCraftResult::default(),
        is_single_player: var2,
    });
    c.add_slot(Slot::new_crafting(InventoryRef::CraftResult, 0, 144, 36));
    for var3 in 0..2 {
        for var4 in 0..2 {
            c.add_slot(Slot::new(
                InventoryRef::CraftMatrix,
                var4 + var3 * 2,
                88 + var4 * 18,
                26 + var3 * 18,
            ));
        }
    }
    for var3 in 0..4 {
        // var1.getSizeInventory() - 1 - var3: 39 - var3
        c.add_slot(Slot::new_armor(
            InventoryRef::Player,
            40 - 1 - var3,
            8,
            8 + var3 * 18,
            var3,
        ));
    }
    for var3 in 0..3 {
        for var4 in 0..9 {
            c.add_slot(Slot::new(
                InventoryRef::Player,
                var4 + (var3 + 1) * 9,
                8 + var4 * 18,
                84 + var3 * 18,
            ));
        }
    }
    for var3 in 0..9 {
        c.add_slot(Slot::new(InventoryRef::Player, var3, 8 + var3 * 18, 142));
    }
    // src: ContainerPlayer.java:41: onCraftMatrixChanged(craftMatrix); only the
    // container's own grid and result are involved.
    if let ContainerKind::Player {
        craft_matrix,
        craft_result,
        ..
    } = &mut c.kind
    {
        let r = crate::crafting::CraftingManager::get_instance().find_matching_recipe(craft_matrix);
        craft_result.set_inventory_slot_contents(0, r);
    }
    c
}

impl Container {
    /// `ContainerPlayer.getStackInSlot(i)` (shift-click): the output to the
    /// inventory and hotbar from the end, the main inventory to the
    /// hotbar, the hotbar to the main inventory, the grid and armour to
    /// both.
    // src: ContainerPlayer.java:65-95
    pub(crate) fn player_get_stack_in_slot(
        &mut self,
        world: &mut World,
        player: &mut Entity,
        var1: usize,
    ) -> Option<ItemStack> {
        let range = match var1 {
            0 => (9, 45, true),
            9..36 => (36, 45, false),
            36..45 => (9, 36, false),
            _ => (9, 45, false),
        };
        self.transfer_and_pick_up(world, player, var1, range)
    }
}
