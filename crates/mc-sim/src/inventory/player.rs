//! `InventoryPlayer` (InventoryPlayer.java): 36 main slots (0–8 the
//! hotbar), 4 armour slots, the selected hotbar slot and the stack held by
//! the mouse cursor.
//!
//! The inventory lives in the player's `PlayerData`; its back-reference
//! `player` is implicit. Methods that only touch the inventory are on
//! [`InventoryPlayer`]; those that reach the player or the world
//! (`decrementAnimations`, `damageArmor`, `dropAllItems`,
//! `canInteractWith`) are free functions taking the owning player entity.
//!
//! `writeToNBT` / `readFromNBT` (InventoryPlayer.java:199-241) are
//! [`crate::entity::nbt::inventory_write_to_nbt`] and
//! [`crate::entity::nbt::inventory_read_from_nbt`] (WP-1.9; checked against
//! the Java for this package), reachable here as
//! [`InventoryPlayer::write_to_nbt`] / [`InventoryPlayer::read_from_nbt`].

use mc_nbt::Tag;

use super::drop_player_item_with_random_choice;
use crate::block::BlockDef;
use crate::entity::Entity;
use crate::item::{ItemKind, ItemStack};
use crate::save::SaveError;
use crate::world::World;

/// `InventoryPlayer` (InventoryPlayer.java:3-13). The back-reference
/// `player` is implicit (the inventory lives in the player's entity data).
#[derive(Clone, Debug, PartialEq)]
pub struct InventoryPlayer {
    /// `mainInventory`: 36 slots, 0–8 the hotbar.
    pub main_inventory: [Option<ItemStack>; 36],
    /// `armorInventory`: 4 slots (0 boots … 3 helmet: `armorType` 3 − slot).
    pub armor_inventory: [Option<ItemStack>; 4],
    /// `currentItem`: selected hotbar slot.
    pub current_item: i32,
    /// `itemStack` (private; `getItemStack` / `setItemStack`): the stack
    /// held by the mouse cursor.
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

/// `getInventoryStackLimit()`.
// src: InventoryPlayer.java:262-264
pub const INVENTORY_STACK_LIMIT: i32 = 64;

impl InventoryPlayer {
    /// `getCurrentItem()`: the selected hotbar stack (null outside 0–8).
    // src: InventoryPlayer.java:15-17
    pub fn get_current_item(&self) -> Option<&ItemStack> {
        if self.current_item < 9 && self.current_item >= 0 {
            self.main_inventory[self.current_item as usize].as_ref()
        } else {
            None
        }
    }

    /// `getInventorySlotContainItem(id)` (private): the first main slot
    /// holding item `id`, or −1.
    // src: InventoryPlayer.java:19-27
    pub fn get_inventory_slot_contain_item(&self, var1: i32) -> i32 {
        for (var2, s) in self.main_inventory.iter().enumerate() {
            if s.is_some_and(|s| s.item_id == var1) {
                return var2 as i32;
            }
        }
        -1
    }

    /// `storeItemStack(stack)` (private): the first main slot with the same
    /// item that still has room (below both the item's and the inventory's
    /// limit; same damage for items with subtypes), or −1.
    // src: InventoryPlayer.java:29-37
    pub fn store_item_stack(&self, var1: &ItemStack) -> i32 {
        for (var2, s) in self.main_inventory.iter().enumerate() {
            if let Some(s) = s
                && s.item_id == var1.item_id
                && s.is_stackable()
                && s.stack_size < s.get_max_stack_size()
                && s.stack_size < self.get_inventory_stack_limit()
                && (!s.get_has_subtypes() || s.get_item_damage() == var1.get_item_damage())
            {
                return var2 as i32;
            }
        }
        -1
    }

    /// `getFirstEmptyStack()` (private): the first empty main slot, or −1.
    // src: InventoryPlayer.java:39-47
    pub fn get_first_empty_stack(&self) -> i32 {
        for (var1, s) in self.main_inventory.iter().enumerate() {
            if s.is_none() {
                return var1 as i32;
            }
        }
        -1
    }

    /// `setCurrentItem(id, flag)`: selects the first slot holding `id` if it
    /// is in the hotbar (`flag` is unused).
    // src: InventoryPlayer.java:49-54
    pub fn set_current_item(&mut self, var1: i32, _var2: bool) {
        let var3 = self.get_inventory_slot_contain_item(var1);
        if (0..9).contains(&var3) {
            self.current_item = var3;
        }
    }

    /// `changeCurrentItem(direction)`: the mouse wheel; one slot against
    /// the sign of `direction`, wrapping within the hotbar.
    // src: InventoryPlayer.java:56-72
    pub fn change_current_item(&mut self, mut var1: i32) {
        if var1 > 0 {
            var1 = 1;
        }
        if var1 < 0 {
            var1 = -1;
        }
        self.current_item = self.current_item.wrapping_sub(var1);
        while self.current_item < 0 {
            self.current_item += 9;
        }
        while self.current_item >= 9 {
            self.current_item -= 9;
        }
    }

    /// `storePartialItemStack(stack)` (private): moves as much of `stack` as
    /// fits into one slot (a matching one with room, else the first empty
    /// one, created as `new ItemStack(id, 0, damage)`), sets that slot's
    /// `animationsToGo` to 5 and returns what is left (the whole size when
    /// nothing moved). `stack` itself is not changed.
    // src: InventoryPlayer.java:74-108
    pub fn store_partial_item_stack(&mut self, var1: &ItemStack) -> i32 {
        let var2 = var1.item_id;
        let mut var3 = var1.stack_size;
        let mut var4 = self.store_item_stack(var1);
        if var4 < 0 {
            var4 = self.get_first_empty_stack();
        }
        if var4 < 0 {
            return var3;
        }
        let limit = self.get_inventory_stack_limit();
        let slot = &mut self.main_inventory[var4 as usize];
        let s = slot.get_or_insert(ItemStack::new(var2, 0, var1.get_item_damage()));
        let mut var5 = var3;
        if var3 > s.get_max_stack_size() - s.stack_size {
            var5 = s.get_max_stack_size() - s.stack_size;
        }
        if var5 > limit - s.stack_size {
            var5 = limit - s.stack_size;
        }
        if var5 == 0 {
            var3
        } else {
            var3 -= var5;
            s.stack_size += var5;
            s.animations_to_go = 5;
            var3
        }
    }

    /// `addItemStackToInventory(stack)`: a damaged item goes whole into the
    /// first empty slot (a copy with `animationsToGo` 5; `stack` drops to
    /// size 0); anything else is stored part by part until it is gone or a
    /// round moves nothing. `stack.stackSize` is updated in place, as the
    /// caller's object is in Java (`EntityItem.onCollideWithPlayer` reads
    /// it). True when anything was taken.
    // src: InventoryPlayer.java:132-152
    pub fn add_item_stack_to_inventory(&mut self, var1: &mut ItemStack) -> bool {
        if var1.is_item_damaged() {
            let var2 = self.get_first_empty_stack();
            if var2 >= 0 {
                let mut c = var1.copy();
                c.animations_to_go = 5;
                self.main_inventory[var2 as usize] = Some(c);
                var1.stack_size = 0;
                true
            } else {
                false
            }
        } else {
            let mut var2;
            loop {
                var2 = var1.stack_size;
                var1.stack_size = self.store_partial_item_stack(var1);
                if !(var1.stack_size > 0 && var1.stack_size < var2) {
                    break;
                }
            }
            var1.stack_size < var2
        }
    }

    /// `consumeInventoryItem(id)`: one item `id` from its first slot (the
    /// slot empties at size 0 or less). False when there is none.
    // src: InventoryPlayer.java:119-130
    pub fn consume_inventory_item(&mut self, var1: i32) -> bool {
        let var2 = self.get_inventory_slot_contain_item(var1);
        if var2 < 0 {
            return false;
        }
        let slot = &mut self.main_inventory[var2 as usize];
        if let Some(s) = slot.as_mut() {
            s.stack_size = s.stack_size.wrapping_sub(1);
            if s.stack_size <= 0 {
                *slot = None;
            }
        }
        true
    }

    /// The array and index `slot` names (main 0–35, armour 36–39). Panics
    /// where Java throws `ArrayIndexOutOfBoundsException`.
    fn slot_ref(&mut self, slot: i32) -> &mut Option<ItemStack> {
        let main_len = self.main_inventory.len() as i32;
        let r = if slot >= main_len {
            self.armor_inventory.get_mut((slot - main_len) as usize)
        } else {
            usize::try_from(slot)
                .ok()
                .and_then(|i| self.main_inventory.get_mut(i))
        };
        match r {
            Some(s) => s,
            None => panic!("InventoryPlayer slot {slot}: ArrayIndexOutOfBoundsException"),
        }
    }

    /// `decrStackSize(slot, n)`: the whole stack when it has `n` or fewer
    /// items (the slot empties), else `n` split off (no event in
    /// `InventoryPlayer`).
    // src: InventoryPlayer.java:154-178
    pub fn decr_stack_size(&mut self, var1: i32, var2: i32) -> Option<ItemStack> {
        let slot = self.slot_ref(var1);
        let cur = slot.as_mut()?;
        if cur.stack_size <= var2 {
            slot.take()
        } else {
            let var4 = cur.split_stack(var2);
            if cur.stack_size == 0 {
                *slot = None;
            }
            Some(var4)
        }
    }

    /// `setInventorySlotContents(slot, stack)` (no size clamp, no event).
    // src: InventoryPlayer.java:180-188
    pub fn set_inventory_slot_contents(&mut self, var1: i32, var2: Option<ItemStack>) {
        *self.slot_ref(var1) = var2;
    }

    /// The stored stack of `slot`, mutably: what Java code changes in place
    /// through the object `getStackInSlot` returned.
    pub fn stack_mut(&mut self, slot: i32) -> &mut Option<ItemStack> {
        self.slot_ref(slot)
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

    /// `writeToNBT(new NBTTagList())`.
    // src: InventoryPlayer.java:199-221
    pub fn write_to_nbt(&self) -> Tag {
        crate::entity::nbt::inventory_write_to_nbt(self)
    }

    /// `readFromNBT(list)`.
    // src: InventoryPlayer.java:223-241
    pub fn read_from_nbt(&mut self, list: &[Tag]) -> Result<(), SaveError> {
        crate::entity::nbt::inventory_read_from_nbt(self, list)
    }

    /// `getSizeInventory()`: 36 + 4.
    // src: InventoryPlayer.java:244-246
    pub fn get_size_inventory(&self) -> i32 {
        self.main_inventory.len() as i32 + 4
    }

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

    /// `getInvName()`.
    // src: InventoryPlayer.java:258-260
    pub fn get_inv_name(&self) -> &'static str {
        "Inventory"
    }

    /// `getInventoryStackLimit()`.
    // src: InventoryPlayer.java:262-264
    pub fn get_inventory_stack_limit(&self) -> i32 {
        INVENTORY_STACK_LIMIT
    }

    /// `getDamageVsEntity(entity)`: the selected stack's damage, 1 with an
    /// empty hand.
    // src: InventoryPlayer.java:266-269
    pub fn get_damage_vs_entity(&self) -> i32 {
        match self.get_stack_in_slot(self.current_item) {
            Some(var2) => var2.get_damage_vs_entity(),
            None => 1,
        }
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

    /// `armorItemInSlot(i)`.
    // src: InventoryPlayer.java:280-282
    pub fn armor_item_in_slot(&self, var1: i32) -> Option<&ItemStack> {
        self.armor_inventory[var1 as usize].as_ref()
    }

    /// `getTotalArmorValue()`: the summed `damageReduceAmount` of the worn
    /// `ItemArmor` pieces, scaled by their remaining durability:
    /// `(sum − 1) * left / max + 1` (integer arithmetic), 0 with nothing
    /// worn.
    // src: InventoryPlayer.java:284-306
    pub fn get_total_armor_value(&self) -> i32 {
        let mut var1 = 0i32;
        let mut var2 = 0i32;
        let mut var3 = 0i32;
        for s in self.armor_inventory.iter().flatten() {
            if let Some(ItemKind::Armor {
                damage_reduce_amount,
                ..
            }) = s.get_item().map(|d| d.kind)
            {
                let var5 = s.get_max_damage();
                let var6 = s.get_item_damage_for_display();
                let var7 = var5 - var6;
                var2 += var7;
                var3 += var5;
                var1 += damage_reduce_amount;
            }
        }
        if var3 == 0 {
            0
        } else {
            (var1 - 1).wrapping_mul(var2) / var3 + 1
        }
    }

    /// `onInventoryChanged()`.
    // src: InventoryPlayer.java:339-341
    pub fn on_inventory_changed(&mut self) {
        self.inventory_changed = true;
    }

    /// `setItemStack(stack)`: the cursor stack; then
    /// `player.onItemStackChanged(stack)`, empty in `EntityPlayer`
    /// (EntityPlayer.java:521-522; only `EntityPlayerMP` on the server
    /// overrides it).
    // src: InventoryPlayer.java:343-346
    pub fn set_item_stack(&mut self, var1: Option<ItemStack>) {
        self.item_stack = var1;
    }

    /// `getItemStack()`.
    // src: InventoryPlayer.java:348-350
    pub fn get_item_stack(&self) -> Option<ItemStack> {
        self.item_stack
    }

    /// `hasItemStack(stack)`: an armour or main slot holding a stack equal
    /// in id, size and damage (`isStackEqual`), armour first.
    // src: InventoryPlayer.java:360-375
    pub fn has_item_stack(&self, var1: &ItemStack) -> bool {
        self.armor_inventory
            .iter()
            .chain(self.main_inventory.iter())
            .flatten()
            .any(|s| s.is_stack_equal(var1))
    }
}

/// `inventory.decrementAnimations()` of `player`: each main stack's
/// `updateAnimation(world, player, slot, slot == currentItem)`, i.e. the
/// pick-up animation counter down by one, then `Item.onUpdate` (empty in
/// `Item`; `ItemMap.onUpdate` updates map data: item behaviour, WP-2.6,
/// recorded).
// src: InventoryPlayer.java:110-117, ItemStack.java:212-218, Item.java:267-268, ItemMap.java:213-226
pub fn decrement_animations(world: &mut World, player: &mut Entity) {
    let inv = super::inventory_of_mut(player);
    for var1 in 0..inv.main_inventory.len() {
        if let Some(s) = inv.main_inventory[var1].as_mut() {
            if s.animations_to_go > 0 {
                s.animations_to_go -= 1;
            }
            if s.item_id == crate::crafting::ids::MAP_ITEM {
                world.note_unimplemented("ItemMap.onUpdate");
            }
        }
    }
}

/// `inventory.damageArmor(amount)` of `player`: `damageItem(amount,
/// player)` on every worn `ItemArmor`; a piece that breaks adds
/// `objectBreakStats` to the player (`EntityPlayer.addStat`, WP-2.7,
/// recorded) and, left at size 0, is removed (`onItemDestroyedByUse` is
/// empty).
// src: InventoryPlayer.java:308-319, ItemStack.java:124-141, :167-168
pub fn damage_armor(world: &mut World, player: &mut Entity, var1: i32) {
    let inv = super::inventory_of_mut(player);
    for var2 in 0..inv.armor_inventory.len() {
        let slot = &mut inv.armor_inventory[var2];
        let Some(s) = slot.as_mut() else { continue };
        if !matches!(s.get_item().map(|d| d.kind), Some(ItemKind::Armor { .. })) {
            continue;
        }
        if s.damage_item(var1) {
            // src: ItemStack.java:128-130 (var2 is the EntityPlayer)
            world.note_unimplemented("EntityPlayer.addStat");
        }
        if s.stack_size == 0 {
            *slot = None;
        }
    }
}

/// `inventory.dropAllItems()` of `player`: every main slot, then every
/// armour slot, through `player.dropPlayerItemWithRandomChoice(stack,
/// true)`, each slot emptied after its drop.
// src: InventoryPlayer.java:321-337
pub fn drop_all_items(world: &mut World, player: &mut Entity) {
    for var1 in 0..36 {
        if let Some(s) = super::inventory_of(player).main_inventory[var1] {
            drop_player_item_with_random_choice(world, player, Some(s), true);
            super::inventory_of_mut(player).main_inventory[var1] = None;
        }
    }
    for var1 in 0..4 {
        if let Some(s) = super::inventory_of(player).armor_inventory[var1] {
            drop_player_item_with_random_choice(world, player, Some(s), true);
            super::inventory_of_mut(player).armor_inventory[var1] = None;
        }
    }
}

/// `owner.inventory.canInteractWith(other)`: false once the owner is dead,
/// else `other` within 8 blocks of the owner.
// src: InventoryPlayer.java:352-358
pub fn can_interact_with(owner: &Entity, other: &Entity) -> bool {
    if owner.base.is_dead {
        false
    } else {
        other.base.get_distance_sq_to_entity(&owner.base) <= 64.0
    }
}
