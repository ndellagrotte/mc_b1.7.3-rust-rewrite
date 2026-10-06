//! `TileEntityChest` (TileEntityChest.java): 27 inventory slots, no update.
//!
//! The `IInventory` methods are the shared ones in [`super`]
//! (`decrStackSize` and `setInventorySlotContents` call `onInventoryChanged`).

use std::collections::BTreeMap;

use mc_nbt::Tag;

use super::{compound, get_byte, get_tag_list, write_items};
use crate::item::{ItemStack, JavaException};

/// `getSizeInventory()`: always 27, although a fresh chest's array has 36
/// slots (vanilla, transcribed: slots 27-35 are never reachable, and
/// `readFromNBT` reallocates the array at 27).
// src: TileEntityChest.java:6-8
pub const SIZE_INVENTORY: i32 = 27;

/// The fields `TileEntityChest` declares.
#[derive(Clone, Debug, PartialEq)]
pub struct TileEntityChest {
    /// `chestContents` (private): 36 slots when constructed, 27 after
    /// `readFromNBT`; `writeToNBT` walks the whole array.
    // src: TileEntityChest.java:4
    pub chest_contents: Vec<Option<ItemStack>>,
}

impl Default for TileEntityChest {
    // src: TileEntityChest.java:4 (`new ItemStack[36]`)
    fn default() -> Self {
        TileEntityChest {
            chest_contents: vec![None; 36],
        }
    }
}

impl TileEntityChest {
    /// `readFromNBT(compound)` after `super.readFromNBT`: a new 27-slot
    /// array, filled from `Items` (`Slot` read as an unsigned byte; slots
    /// outside the array are ignored).
    // src: TileEntityChest.java:49-62
    pub fn read_from_nbt(&mut self, c: &BTreeMap<String, Tag>) -> Result<(), JavaException> {
        let var2 = get_tag_list(c, "Items")?;
        self.chest_contents = vec![None; SIZE_INVENTORY as usize];
        for t in var2 {
            let var4 = compound(t)?;
            let var5 = (get_byte(var4, "Slot")? as i32) & 255;
            if var5 >= 0 && (var5 as usize) < self.chest_contents.len() {
                self.chest_contents[var5 as usize] = Some(ItemStack::from_nbt(t)?);
            }
        }
        Ok(())
    }

    /// `writeToNBT(compound)` after `super.writeToNBT`: `Items`.
    // src: TileEntityChest.java:64-78
    pub fn write_to_nbt(&self, c: &mut BTreeMap<String, Tag>) {
        c.insert("Items".to_string(), write_items(&self.chest_contents));
    }
}
