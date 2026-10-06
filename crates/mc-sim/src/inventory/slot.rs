//! `Slot` (Slot.java) and its subclasses `SlotArmor` (SlotArmor.java),
//! `SlotCrafting` (SlotCrafting.java) and `SlotFurnace` (SlotFurnace.java).
//!
//! The data is [`Slot`]; the methods that go through the slot's inventory
//! are `slot_*` methods of [`Container`] (the inventory may be the
//! container's own crafting grid, the player's inventory or a tile entity;
//! see [`super::container`]).

use super::container::Container;
use super::{InventoryRef, item_stack_on_crafting};
use crate::crafting::ids;
use crate::entity::Entity;
use crate::item::{ItemKind, ItemStack};
use crate::world::World;

/// Which `Slot` class, with its own fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlotKind {
    /// `Slot`.
    Plain,
    /// `SlotArmor` (an anonymous-like inner class of `ContainerPlayer`):
    /// `armorType` 0 helmet … 3 boots. Its `inventory` field (the
    /// `ContainerPlayer`) is never read.
    Armor {
        /// `armorType`.
        armor_type: i32,
    },
    /// `SlotCrafting`: the output of the container's `craftMatrix`
    /// (`craftMatrix` is always [`InventoryRef::CraftMatrix`]; `thePlayer`
    /// is the container's player).
    Crafting,
    /// `SlotFurnace`: a furnace's output (`thePlayer` is the container's
    /// player).
    Furnace,
}

/// `Slot`.
#[derive(Clone, Debug, PartialEq)]
pub struct Slot {
    /// `slotIndex` (private, final): the index in `inventory`.
    pub slot_index: i32,
    /// `inventory` (private, final).
    pub inventory: InventoryRef,
    /// `slotNumber`: the index in the container's `slots`.
    pub slot_number: i32,
    /// `xDisplayPosition`.
    pub x_display_position: i32,
    /// `yDisplayPosition`.
    pub y_display_position: i32,
    /// The class.
    pub kind: SlotKind,
}

impl Slot {
    /// `new Slot(inventory, index, x, y)` (`slotNumber` is set by
    /// `Container.addSlot`).
    // src: Slot.java:10-15
    pub fn new(var1: InventoryRef, var2: i32, var3: i32, var4: i32) -> Slot {
        Slot {
            slot_index: var2,
            inventory: var1,
            slot_number: 0,
            x_display_position: var3,
            y_display_position: var4,
            kind: SlotKind::Plain,
        }
    }

    /// `new SlotArmor(container, inventory, index, x, y, armorType)`.
    // src: SlotArmor.java:9-13
    pub fn new_armor(var2: InventoryRef, var3: i32, var4: i32, var5: i32, var6: i32) -> Slot {
        Slot {
            kind: SlotKind::Armor { armor_type: var6 },
            ..Slot::new(var2, var3, var4, var5)
        }
    }

    /// `new SlotCrafting(player, craftMatrix, craftResult, index, x, y)`.
    // src: SlotCrafting.java:7-11
    pub fn new_crafting(var3: InventoryRef, var4: i32, var5: i32, var6: i32) -> Slot {
        Slot {
            kind: SlotKind::Crafting,
            ..Slot::new(var3, var4, var5, var6)
        }
    }

    /// `new SlotFurnace(player, inventory, index, x, y)`.
    // src: SlotFurnace.java:6-9
    pub fn new_furnace(var2: InventoryRef, var3: i32, var4: i32, var5: i32) -> Slot {
        Slot {
            kind: SlotKind::Furnace,
            ..Slot::new(var2, var3, var4, var5)
        }
    }

    /// `isItemValid(stack)`: any stack for a plain slot; for armour an
    /// `ItemArmor` of the slot's `armorType`, or a pumpkin in the helmet
    /// slot; nothing for the crafting and furnace outputs.
    // src: Slot.java:21-23, SlotArmor.java:19-27, SlotCrafting.java:13-15, SlotFurnace.java:11-13
    pub fn is_item_valid(&self, var1: &ItemStack) -> bool {
        match self.kind {
            SlotKind::Plain => true,
            SlotKind::Armor { armor_type } => {
                // var1.getItem(): a null item would throw (NullPointerException).
                let item = var1.get_item().expect(
                    "SlotArmor.isItemValid on an id with no item (Java NullPointerException)",
                );
                if let ItemKind::Armor { armor_type: t, .. } = item.kind {
                    t == armor_type
                } else if item.shifted_index == crate::block::ids::PUMPKIN {
                    armor_type == 0
                } else {
                    false
                }
            }
            SlotKind::Crafting | SlotKind::Furnace => false,
        }
    }

    /// `getBackgroundIconIndex()`: −1 (no subclass overrides it in b1.7.3).
    // src: Slot.java:46-48
    pub fn get_background_icon_index(&self) -> i32 {
        -1
    }
}

impl Container {
    /// `slot.getStack()`: `inventory.getStackInSlot(slotIndex)` (the stored
    /// object in Java; a copy here).
    // src: Slot.java:25-27
    pub fn slot_get_stack(
        &mut self,
        world: &mut World,
        player: &mut Entity,
        slot: usize,
    ) -> Option<ItemStack> {
        let (inv, idx) = self.slot_target(slot);
        self.inv_get_stack_in_slot(world, player, &inv, idx)
    }

    /// `slot.getHasStack()`.
    // src: Slot.java:29-31
    pub fn slot_get_has_stack(
        &mut self,
        world: &mut World,
        player: &mut Entity,
        slot: usize,
    ) -> bool {
        self.slot_get_stack(world, player, slot).is_some()
    }

    /// `slot.putStack(stack)`: `setInventorySlotContents`, then
    /// `onSlotChanged`.
    // src: Slot.java:33-36
    pub fn slot_put_stack(
        &mut self,
        world: &mut World,
        player: &mut Entity,
        slot: usize,
        var1: Option<ItemStack>,
    ) {
        let (inv, idx) = self.slot_target(slot);
        self.inv_set_inventory_slot_contents(world, player, &inv, idx, var1);
        self.slot_on_slot_changed(world, player, slot);
    }

    /// `slot.onSlotChanged()`: `inventory.onInventoryChanged()`.
    // src: Slot.java:38-40
    pub fn slot_on_slot_changed(&mut self, world: &mut World, player: &mut Entity, slot: usize) {
        let (inv, _) = self.slot_target(slot);
        self.inv_on_inventory_changed(world, player, &inv);
    }

    /// `slot.getSlotStackLimit()`: 1 for armour, else the inventory's
    /// limit.
    // src: Slot.java:42-44, SlotArmor.java:15-17
    pub fn slot_get_slot_stack_limit(&self, slot: usize) -> i32 {
        match self.slots[slot].kind {
            SlotKind::Armor { .. } => 1,
            _ => self.inv_get_inventory_stack_limit(&self.slots[slot].inventory),
        }
    }

    /// `slot.decrStackSize(n)`: `inventory.decrStackSize(slotIndex, n)`.
    // src: Slot.java:50-52
    pub fn slot_decr_stack_size(
        &mut self,
        world: &mut World,
        player: &mut Entity,
        slot: usize,
        var1: i32,
    ) -> Option<ItemStack> {
        let (inv, idx) = self.slot_target(slot);
        self.inv_decr_stack_size(world, player, &inv, idx, var1)
    }

    /// The slot's stored stack, changed in place by `f` (Java's field
    /// writes on the object `getStack()` returned: no event).
    pub fn slot_stack_mut<R>(
        &mut self,
        world: &mut World,
        player: &mut Entity,
        slot: usize,
        f: impl FnOnce(&mut Option<ItemStack>) -> R,
    ) -> R {
        let (inv, idx) = self.slot_target(slot);
        self.inv_stack_mut(world, player, &inv, idx, f)
    }

    /// `slot.onPickupFromSlot(stack)`: `stack` is what was taken (the held
    /// stack, or the slot's own object after a shift-click).
    ///
    /// - `Slot`, `SlotArmor`: `onSlotChanged`.
    /// - `SlotCrafting`: `stack.onCrafting` and the crafting achievements
    ///   (workbench, wooden pickaxe, furnace, wooden hoe, bread, cake,
    ///   stone pickaxe, wooden sword: `EntityPlayer.addStat`, WP-2.7,
    ///   recorded), then one item off every non-empty grid cell (each
    ///   `decrStackSize` re-runs `onCraftMatrixChanged`), a cell whose item
    ///   has a container item (the milk bucket) getting a new stack of it.
    ///   Not `super`: no `onSlotChanged`.
    /// - `SlotFurnace`: `stack.onCrafting`, the iron and fish achievements
    ///   (recorded), then `super`.
    // src: Slot.java:17-19, SlotCrafting.java:17-47, SlotFurnace.java:15-26
    pub fn slot_on_pickup_from_slot(
        &mut self,
        world: &mut World,
        player: &mut Entity,
        slot: usize,
        var1: Option<ItemStack>,
    ) {
        match self.slots[slot].kind {
            SlotKind::Plain | SlotKind::Armor { .. } => {
                self.slot_on_slot_changed(world, player, slot);
            }
            SlotKind::Crafting => {
                let var1 =
                    var1.expect("SlotCrafting.onPickupFromSlot(null): Java NullPointerException");
                item_stack_on_crafting(world, player, &var1);
                // src: SlotCrafting.java:19-35 (AchievementList.buildWorkBench,
                // buildPickaxe, buildFurnace, buildHoe, makeBread, bakeCake,
                // buildBetterPickaxe, buildSword)
                if [
                    ids::WORKBENCH,
                    ids::PICKAXE_WOOD,
                    ids::STONE_OVEN_IDLE,
                    ids::HOE_WOOD,
                    ids::BREAD,
                    ids::CAKE,
                    ids::PICKAXE_STONE,
                    ids::SWORD_WOOD,
                ]
                .contains(&var1.item_id)
                {
                    world.note_unimplemented("EntityPlayer.addStat");
                }
                let matrix = InventoryRef::CraftMatrix;
                let size = self.inv_get_size_inventory(world, &matrix);
                for var2 in 0..size {
                    let var3 = self.inv_get_stack_in_slot(world, player, &matrix, var2);
                    if let Some(var3) = var3 {
                        self.inv_decr_stack_size(world, player, &matrix, var2, 1);
                        let item = var3.get_item().expect(
                            "SlotCrafting: a grid stack with no item (Java NullPointerException)",
                        );
                        if item.has_container_item() {
                            let c = item
                                .get_container_item()
                                .expect("hasContainerItem implies a container item");
                            // new ItemStack(Item): count 1, damage 0
                            self.inv_set_inventory_slot_contents(
                                world,
                                player,
                                &matrix,
                                var2,
                                Some(ItemStack::new(c.shifted_index, 1, 0)),
                            );
                        }
                    }
                }
            }
            SlotKind::Furnace => {
                let var1 =
                    var1.expect("SlotFurnace.onPickupFromSlot(null): Java NullPointerException");
                item_stack_on_crafting(world, player, &var1);
                // src: SlotFurnace.java:17-23 (AchievementList.acquireIron, cookFish)
                if var1.item_id == ids::INGOT_IRON {
                    world.note_unimplemented("EntityPlayer.addStat");
                }
                if var1.item_id == ids::FISH_COOKED {
                    world.note_unimplemented("EntityPlayer.addStat");
                }
                self.slot_on_slot_changed(world, player, slot);
            }
        }
    }

    /// The slot's inventory and index (cloned, so the container can be
    /// borrowed mutably while it is used).
    pub(crate) fn slot_target(&self, slot: usize) -> (InventoryRef, i32) {
        let s = &self.slots[slot];
        (s.inventory.clone(), s.slot_index)
    }
}
