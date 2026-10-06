//! `ContainerFurnace` (ContainerFurnace.java): a furnace's window.
//!
//! Slots: 0 the input, 1 the fuel, 2 the output (`SlotFurnace`), 3–29 the
//! main inventory (`InventoryPlayer` slots 9–35), 30–38 the hotbar (0–8).
//! `updateCraftingResults` and `updateProgressBar` are in
//! [`super::Container`].

use super::{Container, ContainerKind};
use crate::entity::Entity;
use crate::inventory::InventoryRef;
use crate::inventory::slot::Slot;
use crate::item::ItemStack;
use crate::world::World;
use crate::world::tile_entity::TileEntityRef;

/// `new ContainerFurnace(playerInventory, furnace)`.
// src: ContainerFurnace.java:9-26
pub fn new_container_furnace(var2: TileEntityRef) -> Container {
    let mut c = Container::empty(ContainerKind::Furnace {
        furnace: var2,
        cook_time: 0,
        burn_time: 0,
        item_burn_time: 0,
    });
    c.add_slot(Slot::new(InventoryRef::TileEntity(var2), 0, 56, 17));
    c.add_slot(Slot::new(InventoryRef::TileEntity(var2), 1, 56, 53));
    c.add_slot(Slot::new_furnace(
        InventoryRef::TileEntity(var2),
        2,
        116,
        35,
    ));
    for var3 in 0..3 {
        for var4 in 0..9 {
            c.add_slot(Slot::new(
                InventoryRef::Player,
                var4 + var3 * 9 + 9,
                8 + var4 * 18,
                84 + var3 * 18,
            ));
        }
    }
    for var3 in 0..9 {
        c.add_slot(Slot::new(InventoryRef::Player, var3, 8 + var3 * 18, 142));
    }
    c
}

impl Container {
    /// `ContainerFurnace.getStackInSlot(i)` (shift-click): the output to the
    /// player's slots from the end, the main inventory to the hotbar, the
    /// hotbar to the main inventory, input and fuel to both.
    // src: ContainerFurnace.java:70-100
    pub(crate) fn furnace_get_stack_in_slot(
        &mut self,
        world: &mut World,
        player: &mut Entity,
        var1: usize,
    ) -> Option<ItemStack> {
        let range = match var1 {
            2 => (3, 39, true),
            3..30 => (30, 39, false),
            30..39 => (3, 30, false),
            _ => (3, 39, false),
        };
        self.transfer_and_pick_up(world, player, var1, range)
    }
}
