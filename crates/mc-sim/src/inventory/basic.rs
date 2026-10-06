//! `InventoryBasic` (InventoryBasic.java): a named, fixed-size inventory
//! with change listeners (`IInvBasic`).
//!
//! In the b1.7.3 client only the multiplayer handler builds one, for a
//! server-opened window (NetClientHandler.java:488, deferred per D3), and no
//! `IInvBasic` implementation exists in the client, so `changeListeners`
//! stays null and `onInventoryChanged` does nothing. Transcribed for the
//! `IInventory` family's completeness.

use crate::item::ItemStack;

/// `InventoryBasic`.
#[derive(Clone, Debug, PartialEq)]
pub struct InventoryBasic {
    /// `inventoryTitle` (private).
    pub inventory_title: String,
    /// `slotsCount` (private).
    pub slots_count: i32,
    /// `inventoryContents` (private).
    pub inventory_contents: Vec<Option<ItemStack>>,
}

impl InventoryBasic {
    /// `new InventoryBasic(title, slots)`.
    // src: InventoryBasic.java:11-15
    pub fn new(var1: &str, var2: i32) -> InventoryBasic {
        InventoryBasic {
            inventory_title: var1.to_string(),
            slots_count: var2,
            inventory_contents: vec![None; var2.max(0) as usize],
        }
    }

    fn index(&self, slot: i32) -> usize {
        match usize::try_from(slot) {
            Ok(i) if i < self.inventory_contents.len() => i,
            _ => panic!("InventoryBasic slot {slot}: ArrayIndexOutOfBoundsException"),
        }
    }

    /// `getStackInSlot(slot)`.
    // src: InventoryBasic.java:17-19
    pub fn get_stack_in_slot(&self, var1: i32) -> Option<ItemStack> {
        self.inventory_contents[self.index(var1)]
    }

    /// `decrStackSize(slot, n)`: the whole stack at `n` or fewer items, else
    /// `n` split off; `onInventoryChanged` either way.
    // src: InventoryBasic.java:21-41
    pub fn decr_stack_size(&mut self, var1: i32, var2: i32) -> Option<ItemStack> {
        let i = self.index(var1);
        let cur = self.inventory_contents[i].as_mut()?;
        let var3 = if cur.stack_size <= var2 {
            self.inventory_contents[i].take()
        } else {
            let var3 = cur.split_stack(var2);
            if cur.stack_size == 0 {
                self.inventory_contents[i] = None;
            }
            Some(var3)
        };
        self.on_inventory_changed();
        var3
    }

    /// `setInventorySlotContents(slot, stack)`: a stack over the limit is
    /// cut down to it; then `onInventoryChanged`.
    // src: InventoryBasic.java:43-50
    pub fn set_inventory_slot_contents(&mut self, var1: i32, mut var2: Option<ItemStack>) {
        let i = self.index(var1);
        if let Some(s) = var2.as_mut()
            && s.stack_size > self.get_inventory_stack_limit()
        {
            s.stack_size = self.get_inventory_stack_limit();
        }
        self.inventory_contents[i] = var2;
        self.on_inventory_changed();
    }

    /// `getSizeInventory()`.
    // src: InventoryBasic.java:52-54
    pub fn get_size_inventory(&self) -> i32 {
        self.slots_count
    }

    /// `getInvName()`.
    // src: InventoryBasic.java:56-58
    pub fn get_inv_name(&self) -> &str {
        &self.inventory_title
    }

    /// `getInventoryStackLimit()`.
    // src: InventoryBasic.java:60-62
    pub fn get_inventory_stack_limit(&self) -> i32 {
        64
    }

    /// `onInventoryChanged()`: notifies the `IInvBasic` listeners; the
    /// client never registers one (module docs).
    // src: InventoryBasic.java:64-71
    pub fn on_inventory_changed(&mut self) {}

    /// `canInteractWith(player)`.
    // src: InventoryBasic.java:73-75
    pub fn can_interact_with(&self) -> bool {
        true
    }
}
