//! Player inventory and containers (`InventoryPlayer.java`, `Container*.java`,
//! `Slot*.java`; WP-2.5).
//!
//! Only `InventoryPlayer`'s field layout exists so far, because the
//! `entity_state` trace hashes it (tests/golden, `Dumps.inventoryBytes`).

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
