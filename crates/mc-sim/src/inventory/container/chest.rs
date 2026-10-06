//! `ContainerChest` (ContainerChest.java): a chest's (or double chest's,
//! or chest minecart's) window.
//!
//! Slots: the chest's `numRows * 9` first, then the main inventory
//! (`InventoryPlayer` slots 9–35), then the hotbar (0–8).

use super::{Container, ContainerKind};
use crate::entity::Entity;
use crate::inventory::InventoryRef;
use crate::inventory::slot::Slot;
use crate::item::ItemStack;
use crate::world::World;

/// `new ContainerChest(playerInventory, chestInventory)`: `numRows` is the
/// chest inventory's size / 9; the player's rows are shifted down by
/// `(numRows − 4) * 18` pixels.
// src: ContainerChest.java:7-29
pub fn new_container_chest(world: &World, var2: InventoryRef) -> Container {
    let mut c = Container::empty(ContainerKind::Chest {
        lower_chest_inventory: var2.clone(),
        num_rows: 0,
    });
    let num_rows = c.inv_get_size_inventory(world, &var2) / 9;
    if let ContainerKind::Chest { num_rows: n, .. } = &mut c.kind {
        *n = num_rows;
    }
    let var3 = (num_rows - 4) * 18;
    for var4 in 0..num_rows {
        for var5 in 0..9 {
            c.add_slot(Slot::new(
                var2.clone(),
                var5 + var4 * 9,
                8 + var5 * 18,
                18 + var4 * 18,
            ));
        }
    }
    for var4 in 0..3 {
        for var5 in 0..9 {
            c.add_slot(Slot::new(
                InventoryRef::Player,
                var5 + var4 * 9 + 9,
                8 + var5 * 18,
                103 + var4 * 18 + var3,
            ));
        }
    }
    for var4 in 0..9 {
        c.add_slot(Slot::new(
            InventoryRef::Player,
            var4,
            8 + var4 * 18,
            161 + var3,
        ));
    }
    c
}

impl Container {
    /// `ContainerChest.getStackInSlot(i)` (shift-click): a chest slot's
    /// stack to the player's slots from the end, a player slot's to the
    /// chest; the slot is emptied or `onSlotChanged`. Unlike the other
    /// containers it returns the copy even when nothing moved and never
    /// calls `onPickupFromSlot`.
    // src: ContainerChest.java:31-56
    pub(crate) fn chest_get_stack_in_slot(
        &mut self,
        world: &mut World,
        player: &mut Entity,
        var1: usize,
    ) -> Option<ItemStack> {
        let ContainerKind::Chest { num_rows, .. } = self.kind else {
            unreachable!("chest_get_stack_in_slot on another container");
        };
        let _ = &self.slots[var1];
        let mut var2 = None;
        if self.slot_get_has_stack(world, player, var1) {
            let mut var4 = self
                .slot_get_stack(world, player, var1)
                .expect("getHasStack");
            var2 = Some(var4.copy());
            let n = self.slots.len() as i32;
            if (var1 as i32) < num_rows * 9 {
                self.merge_item_stack(world, player, &mut var4, num_rows * 9, n, true);
            } else {
                self.merge_item_stack(world, player, &mut var4, 0, num_rows * 9, false);
            }
            // var4 is the slot's object: its new size is the slot's.
            self.slot_stack_mut(world, player, var1, |s| *s = Some(var4));
            if var4.stack_size == 0 {
                self.slot_put_stack(world, player, var1, None);
            } else {
                self.slot_on_slot_changed(world, player, var1);
            }
        }
        var2
    }
}
