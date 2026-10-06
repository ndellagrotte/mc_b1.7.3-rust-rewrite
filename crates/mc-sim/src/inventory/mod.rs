//! Player inventory and containers (`InventoryPlayer.java`, `Container*.java`,
//! `Slot*.java`; WP-2.5).
//!
//! Only `InventoryPlayer`'s field layout exists so far, because the
//! `entity_state` trace hashes it (tests/golden, `Dumps.inventoryBytes`),
//! plus the queries `Block.blockStrength` makes through the player
//! (`getStackInSlot`, `getStrVsBlock`, `canHarvestBlock`; WP-1.6 prep).

use crate::block::BlockDef;
use crate::item::ItemStack;

/// `InventoryPlayer` (InventoryPlayer.java:3-13). The back-reference `player`
/// is implicit (the inventory lives in the player's entity data).
#[derive(Clone, Debug, PartialEq)]
pub struct InventoryPlayer {
    /// `mainInventory`: 36 slots, 0–8 the hotbar.
    pub main_inventory: [Option<ItemStack>; 36],
    /// `armorInventory`: 4 slots.
    pub armor_inventory: [Option<ItemStack>; 4],
    /// `currentItem`: selected hotbar slot.
    pub current_item: i32,
    /// `itemStack`: the stack held by the mouse cursor.
    pub item_stack: Option<ItemStack>,
    /// `inventoryChanged`.
    pub inventory_changed: bool,
}

impl Default for InventoryPlayer {
    // src: InventoryPlayer.java:4-13
    fn default() -> Self {
        InventoryPlayer {
            main_inventory: [None; 36],
            armor_inventory: [None; 4],
            current_item: 0,
            item_stack: None,
            inventory_changed: false,
        }
    }
}

impl InventoryPlayer {
    /// `getStackInSlot(slot)`: slots 0–35 are `mainInventory`, 36–39
    /// `armorInventory`. Panics where Java throws
    /// `ArrayIndexOutOfBoundsException` (a slot outside 0..=39).
    // src: InventoryPlayer.java:248-256
    pub fn get_stack_in_slot(&self, slot: i32) -> Option<&ItemStack> {
        let main_len = self.main_inventory.len() as i32;
        let r = if slot >= main_len {
            self.armor_inventory.get((slot - main_len) as usize)
        } else {
            usize::try_from(slot)
                .ok()
                .and_then(|i| self.main_inventory.get(i))
        };
        match r {
            Some(s) => s.as_ref(),
            None => {
                panic!("InventoryPlayer.getStackInSlot({slot}): ArrayIndexOutOfBoundsException")
            }
        }
    }

    /// `getStrVsBlock(block)`: the held stack's mining speed on `block`, 1
    /// with an empty hand. Reads `mainInventory[currentItem]` directly (no
    /// hotbar range check, unlike `getCurrentItem`).
    // src: InventoryPlayer.java:190-197
    pub fn get_str_vs_block(&self, block: &BlockDef) -> f32 {
        let mut var2 = 1.0f32;
        if let Some(s) = &self.main_inventory[self.current_item as usize] {
            var2 *= s.get_str_vs_block(block);
        }
        var2
    }

    /// `canHarvestBlock(block)`: a harvestable material needs no tool;
    /// otherwise the held stack decides.
    // src: InventoryPlayer.java:271-278
    pub fn can_harvest_block(&self, block: &BlockDef) -> bool {
        if block.block_material.get_is_harvestable() {
            true
        } else {
            match self.get_stack_in_slot(self.current_item) {
                Some(var2) => var2.can_harvest_block(block),
                None => false,
            }
        }
    }

    /// `getCurrentItem()`.
    // src: InventoryPlayer.java:15-17
    pub fn get_current_item(&self) -> Option<&ItemStack> {
        if self.current_item < 9 && self.current_item >= 0 {
            self.main_inventory[self.current_item as usize].as_ref()
        } else {
            None
        }
    }
}
