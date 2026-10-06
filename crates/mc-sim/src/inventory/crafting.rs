//! `InventoryCrafting` (InventoryCrafting.java) and `InventoryCraftResult`
//! (InventoryCraftResult.java).
//!
//! Both belong to their container. `InventoryCrafting`'s `eventHandler`
//! (the container) is implicit: the methods here are the data part, and
//! the container's `IInventory` dispatch runs
//! `eventHandler.onCraftMatrixChanged(this)` after `decrStackSize` and
//! `setInventorySlotContents`, as InventoryCrafting.java:44, :52, :61 do.

use crate::item::ItemStack;

/// `InventoryCrafting`: a `width × height` grid, row-major.
#[derive(Clone, Debug, PartialEq)]
pub struct InventoryCrafting {
    /// `stackList` (private).
    pub stack_list: Vec<Option<ItemStack>>,
    /// `inventoryWidth` (private).
    pub inventory_width: i32,
}

impl InventoryCrafting {
    /// `new InventoryCrafting(container, width, height)`.
    // src: InventoryCrafting.java:8-13
    pub fn new(var2: i32, var3: i32) -> InventoryCrafting {
        let var4 = var2 * var3;
        InventoryCrafting {
            stack_list: vec![None; var4.max(0) as usize],
            inventory_width: var2,
        }
    }

    fn index(&self, slot: i32) -> usize {
        match usize::try_from(slot) {
            Ok(i) if i < self.stack_list.len() => i,
            _ => panic!("InventoryCrafting slot {slot}: ArrayIndexOutOfBoundsException"),
        }
    }

    /// `getSizeInventory()`.
    // src: InventoryCrafting.java:15-17
    pub fn get_size_inventory(&self) -> i32 {
        self.stack_list.len() as i32
    }

    /// `getStackInSlot(slot)`: null at or past the end (a negative slot
    /// throws).
    // src: InventoryCrafting.java:19-21
    pub fn get_stack_in_slot(&self, var1: i32) -> Option<ItemStack> {
        if var1 >= self.get_size_inventory() {
            None
        } else {
            self.stack_list[self.index(var1)]
        }
    }

    /// `getStackInRowAndColumn(column, row)`: null for a column outside the
    /// grid's width (and, through `getStackInSlot`, for a row below it).
    // src: InventoryCrafting.java:23-30
    pub fn get_stack_in_row_and_column(&self, var1: i32, var2: i32) -> Option<ItemStack> {
        if var1 >= 0 && var1 < self.inventory_width {
            let var3 = var1 + var2 * self.inventory_width;
            self.get_stack_in_slot(var3)
        } else {
            None
        }
    }

    /// `getInvName()`.
    // src: InventoryCrafting.java:32-34
    pub fn get_inv_name(&self) -> &'static str {
        "Crafting"
    }

    /// `decrStackSize(slot, n)`, data part (the caller then runs
    /// `onCraftMatrixChanged`): the whole stack at `n` or fewer items, else
    /// `n` split off.
    // src: InventoryCrafting.java:36-56
    pub fn decr_stack_size(&mut self, var1: i32, var2: i32) -> Option<ItemStack> {
        let i = self.index(var1);
        let cur = self.stack_list[i].as_mut()?;
        if cur.stack_size <= var2 {
            self.stack_list[i].take()
        } else {
            let var3 = cur.split_stack(var2);
            if cur.stack_size == 0 {
                self.stack_list[i] = None;
            }
            Some(var3)
        }
    }

    /// `setInventorySlotContents(slot, stack)`, data part (the caller then
    /// runs `onCraftMatrixChanged`).
    // src: InventoryCrafting.java:58-61
    pub fn set_inventory_slot_contents(&mut self, var1: i32, var2: Option<ItemStack>) {
        let i = self.index(var1);
        self.stack_list[i] = var2;
    }

    /// The stored stack, mutably (Java's in-place changes through the
    /// object `getStackInSlot` returned; no event).
    pub fn stack_mut(&mut self, slot: i32) -> &mut Option<ItemStack> {
        let i = self.index(slot);
        &mut self.stack_list[i]
    }

    /// `getInventoryStackLimit()`.
    // src: InventoryCrafting.java:63-65
    pub fn get_inventory_stack_limit(&self) -> i32 {
        64
    }

    /// `onInventoryChanged()`: empty.
    // src: InventoryCrafting.java:67-68
    pub fn on_inventory_changed(&mut self) {}

    /// `canInteractWith(player)`.
    // src: InventoryCrafting.java:70-72
    pub fn can_interact_with(&self) -> bool {
        true
    }
}

/// `InventoryCraftResult`: one slot.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InventoryCraftResult {
    /// `stackResult` (private).
    pub stack_result: [Option<ItemStack>; 1],
}

impl InventoryCraftResult {
    fn index(slot: i32) -> usize {
        match slot {
            0 => 0,
            _ => panic!("InventoryCraftResult slot {slot}: ArrayIndexOutOfBoundsException"),
        }
    }

    /// `getSizeInventory()`.
    // src: InventoryCraftResult.java:6-8
    pub fn get_size_inventory(&self) -> i32 {
        1
    }

    /// `getStackInSlot(slot)`.
    // src: InventoryCraftResult.java:10-12
    pub fn get_stack_in_slot(&self, var1: i32) -> Option<ItemStack> {
        self.stack_result[Self::index(var1)]
    }

    /// `getInvName()`.
    // src: InventoryCraftResult.java:14-16
    pub fn get_inv_name(&self) -> &'static str {
        "Result"
    }

    /// `decrStackSize(slot, n)`: always the whole stack, whatever `n`.
    // src: InventoryCraftResult.java:18-26
    pub fn decr_stack_size(&mut self, var1: i32, _var2: i32) -> Option<ItemStack> {
        self.stack_result[Self::index(var1)].take()
    }

    /// `setInventorySlotContents(slot, stack)`.
    // src: InventoryCraftResult.java:28-30
    pub fn set_inventory_slot_contents(&mut self, var1: i32, var2: Option<ItemStack>) {
        self.stack_result[Self::index(var1)] = var2;
    }

    /// The stored stack, mutably (no event).
    pub fn stack_mut(&mut self, slot: i32) -> &mut Option<ItemStack> {
        &mut self.stack_result[Self::index(slot)]
    }

    /// `getInventoryStackLimit()`.
    // src: InventoryCraftResult.java:32-34
    pub fn get_inventory_stack_limit(&self) -> i32 {
        64
    }

    /// `onInventoryChanged()`: empty.
    // src: InventoryCraftResult.java:36-37
    pub fn on_inventory_changed(&mut self) {}

    /// `canInteractWith(player)`.
    // src: InventoryCraftResult.java:39-41
    pub fn can_interact_with(&self) -> bool {
        true
    }
}
