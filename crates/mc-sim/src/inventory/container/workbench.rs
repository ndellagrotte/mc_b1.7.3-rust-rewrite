//! `ContainerWorkbench` (ContainerWorkbench.java): the crafting table's
//! window.
//!
//! Slots: 0 the output, 1–9 the 3×3 grid, 10–36 the main inventory
//! (`InventoryPlayer` slots 9–35), 37–45 the hotbar (slots 0–8).

use super::{Container, ContainerKind};
use crate::entity::Entity;
use crate::inventory::InventoryRef;
use crate::inventory::crafting::{InventoryCraftResult, InventoryCrafting};
use crate::inventory::slot::Slot;
use crate::item::ItemStack;
use crate::world::World;

/// `new ContainerWorkbench(inventory, world, x, y, z)`: the slots, then
/// `onCraftMatrixChanged` (the empty grid's result, null).
// src: ContainerWorkbench.java:3-33
pub fn new_container_workbench(var3: i32, var4: i32, var5: i32) -> Container {
    let mut c = Container::empty(ContainerKind::Workbench {
        craft_matrix: InventoryCrafting::new(3, 3),
        craft_result: InventoryCraftResult::default(),
        pos: (var3, var4, var5),
    });
    c.add_slot(Slot::new_crafting(InventoryRef::CraftResult, 0, 124, 35));
    for var6 in 0..3 {
        for var7 in 0..3 {
            c.add_slot(Slot::new(
                InventoryRef::CraftMatrix,
                var7 + var6 * 3,
                30 + var7 * 18,
                17 + var6 * 18,
            ));
        }
    }
    for var6 in 0..3 {
        for var7 in 0..9 {
            c.add_slot(Slot::new(
                InventoryRef::Player,
                var7 + var6 * 9 + 9,
                8 + var7 * 18,
                84 + var6 * 18,
            ));
        }
    }
    for var6 in 0..9 {
        c.add_slot(Slot::new(InventoryRef::Player, var6, 8 + var6 * 18, 142));
    }
    // src: ContainerWorkbench.java:32: onCraftMatrixChanged(craftMatrix)
    if let ContainerKind::Workbench {
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
    /// `ContainerWorkbench.getStackInSlot(i)` (shift-click): as the player
    /// container's with the workbench's slot numbers.
    // src: ContainerWorkbench.java:60-90
    pub(crate) fn workbench_get_stack_in_slot(
        &mut self,
        world: &mut World,
        player: &mut Entity,
        var1: usize,
    ) -> Option<ItemStack> {
        let range = match var1 {
            0 => (10, 46, true),
            10..37 => (37, 46, false),
            37..46 => (10, 37, false),
            _ => (10, 46, false),
        };
        self.transfer_and_pick_up(world, player, var1, range)
    }
}
