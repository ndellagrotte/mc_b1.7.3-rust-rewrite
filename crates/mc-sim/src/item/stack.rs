//! `ItemStack` (ItemStack.java): the value type and its data methods.
//!
//! Vanilla's `ItemStack` is a mutable object shared by reference (an
//! `EntityItem` and an inventory slot can hold the same one); the port
//! stores stacks by value and copies where Java aliases, which every current
//! call site allows.
//!
//! The behaviour methods (`useItem`, `useItemRightClick`, `hitEntity`,
//! `onDestroyBlock`, `useItemOnEntity`, `updateAnimation`, `onCrafting`,
//! `damageItem` with its entity) call into [`super::behaviour`] (WP-2.6);
//! they take the stack as `&mut self` while it is out of its inventory slot
//! (the caller puts it back), see the module docs there. The statistics
//! they add (`EntityPlayer.addStat`) go through
//! [`crate::entity::player::add_stat`] (WP-2.7). `onItemDestroyedByUse`
//! (ItemStack.java:167-168) is empty and not ported; its callers note it.

use std::collections::BTreeMap;

use mc_nbt::Tag;

use super::behaviour::{self, RightClickResult};
use super::{ItemDef, JavaException, item};
use crate::block::BlockDef;
use crate::entity::Entity;
use crate::world::World;

/// `ItemStack` (ItemStack.java:3-7).
///
/// Java has no `equals()` override; the derived `==` compares every field
/// (including `animations_to_go`) and is a Rust convenience only. The
/// vanilla comparisons are [`ItemStack::is_item_equal`],
/// [`ItemStack::is_stack_equal`] and [`ItemStack::are_item_stacks_equal`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemStack {
    /// `stackSize`.
    pub stack_size: i32,
    /// `animationsToGo` (the pick-up bob in the hotbar; client-only in effect).
    pub animations_to_go: i32,
    /// `itemID` (block ids 0–255, items 256+).
    pub item_id: i32,
    /// `itemDamage` (private in Java; `getItemDamage()` / `setItemDamage()`).
    pub item_damage: i32,
}

/// `compound.getShort(key)` / `getByte(key)`: 0 when absent, a
/// `ClassCastException` when the tag has another type.
// src: NBTTagCompound.java:91-97
fn get_short(c: &BTreeMap<String, Tag>, key: &str) -> Result<i16, JavaException> {
    match c.get(key) {
        None => Ok(0),
        Some(Tag::Short(v)) => Ok(*v),
        Some(_) => Err(JavaException::ClassCast),
    }
}

fn get_byte(c: &BTreeMap<String, Tag>, key: &str) -> Result<i8, JavaException> {
    match c.get(key) {
        None => Ok(0),
        Some(Tag::Byte(v)) => Ok(*v),
        Some(_) => Err(JavaException::ClassCast),
    }
}

impl ItemStack {
    /// `new ItemStack(id, count, damage)`; the `Block`/`Item` constructors
    /// (ItemStack.java:9-31) pass `blockID` / `shiftedIndex` here, with
    /// count 1 and damage 0 where omitted.
    // src: ItemStack.java:33-38
    pub const fn new(item_id: i32, stack_size: i32, item_damage: i32) -> Self {
        ItemStack {
            stack_size,
            animations_to_go: 0,
            item_id,
            item_damage,
        }
    }

    /// `new ItemStack(Block, count, damage)`; the one- and two-argument
    /// overloads pass count 1 and damage 0.
    // src: ItemStack.java:9-19
    pub const fn from_block(block: &BlockDef, stack_size: i32, item_damage: i32) -> Self {
        ItemStack::new(block.block_id, stack_size, item_damage)
    }

    /// `new ItemStack(Item, count, damage)`; the one- and two-argument
    /// overloads pass count 1 and damage 0.
    // src: ItemStack.java:21-31
    pub const fn from_item(item: &ItemDef, stack_size: i32, item_damage: i32) -> Self {
        ItemStack::new(item.shifted_index, stack_size, item_damage)
    }

    /// `new ItemStack(NBTTagCompound)`. `Err` where Java would throw
    /// (`compound` is not a compound, or a field has the wrong tag type).
    // src: ItemStack.java:40-43
    pub fn from_nbt(compound: &Tag) -> Result<Self, JavaException> {
        let mut s = ItemStack::new(0, 0, 0);
        s.read_from_nbt(compound)?;
        Ok(s)
    }

    /// `splitStack(n)`: takes `n` off this stack into a new one (no bounds
    /// check: `n` may exceed the size or be negative).
    // src: ItemStack.java:45-48
    pub fn split_stack(&mut self, n: i32) -> ItemStack {
        self.stack_size = self.stack_size.wrapping_sub(n);
        ItemStack::new(self.item_id, n, self.item_damage)
    }

    /// `getItem()`: `Item.itemsList[itemID]`.
    // src: ItemStack.java:50-52
    pub fn get_item(&self) -> Option<&'static ItemDef> {
        item(self.item_id)
    }

    /// `getItem()` where the Java code dereferences it; panics where Java
    /// would throw a `NullPointerException` (an id with no item, which a
    /// validated world never holds, PLAN_v4 D5).
    fn item(&self) -> &'static ItemDef {
        match self.get_item() {
            Some(i) => i,
            None => panic!(
                "Item.itemsList[{}] is null (Java NullPointerException)",
                self.item_id
            ),
        }
    }

    /// `getIconIndex()`.
    // src: ItemStack.java:54-56
    pub fn get_icon_index(&self) -> i32 {
        self.item().get_icon_index(self)
    }

    /// `getStrVsBlock(block)`.
    // src: ItemStack.java:67-69
    pub fn get_str_vs_block(&self, block: &BlockDef) -> f32 {
        self.item().get_str_vs_block(block)
    }

    /// `writeToNBT(compound)`: `id` short, `Count` byte, `Damage` short
    /// (Java narrowing casts). Writes into `compound`, which must be a
    /// compound.
    // src: ItemStack.java:75-80
    pub fn write_to_nbt_into(&self, compound: &mut BTreeMap<String, Tag>) {
        compound.insert("id".to_string(), Tag::Short(self.item_id as i16));
        compound.insert("Count".to_string(), Tag::Byte(self.stack_size as i8));
        compound.insert("Damage".to_string(), Tag::Short(self.item_damage as i16));
    }

    /// `writeToNBT(new NBTTagCompound())`.
    // src: ItemStack.java:75-80
    pub fn write_to_nbt(&self) -> Tag {
        let mut m = BTreeMap::new();
        self.write_to_nbt_into(&mut m);
        Tag::Compound(m)
    }

    /// `readFromNBT(compound)`: missing keys read as 0 (the `getShort` /
    /// `getByte` defaults). `animationsToGo` is left alone.
    // src: ItemStack.java:82-86
    pub fn read_from_nbt(&mut self, compound: &Tag) -> Result<(), JavaException> {
        let c = compound.as_compound().ok_or(JavaException::ClassCast)?;
        self.item_id = i32::from(get_short(c, "id")?);
        self.stack_size = i32::from(get_byte(c, "Count")?);
        self.item_damage = i32::from(get_short(c, "Damage")?);
        Ok(())
    }

    /// `getMaxStackSize()`: the item's `getItemStackLimit()`.
    // src: ItemStack.java:88-90
    pub fn get_max_stack_size(&self) -> i32 {
        self.item().get_item_stack_limit()
    }

    /// `isStackable()`.
    // src: ItemStack.java:92-94
    pub fn is_stackable(&self) -> bool {
        self.get_max_stack_size() > 1
            && (!self.is_item_stack_damageable() || !self.is_item_damaged())
    }

    /// `isItemStackDamageable()`: `maxDamage > 0` (unlike
    /// `Item.isDamagable`, ignores `hasSubtypes`).
    // src: ItemStack.java:96-98
    pub fn is_item_stack_damageable(&self) -> bool {
        self.item().get_max_damage() > 0
    }

    /// `getHasSubtypes()`.
    // src: ItemStack.java:100-102
    pub fn get_has_subtypes(&self) -> bool {
        self.item().get_has_subtypes()
    }

    /// `isItemDamaged()`.
    // src: ItemStack.java:104-106
    pub fn is_item_damaged(&self) -> bool {
        self.is_item_stack_damageable() && self.item_damage > 0
    }

    /// `getItemDamageForDisplay()`.
    // src: ItemStack.java:108-110
    pub const fn get_item_damage_for_display(&self) -> i32 {
        self.item_damage
    }

    /// `getItemDamage()`.
    // src: ItemStack.java:112-114
    pub const fn get_item_damage(&self) -> i32 {
        self.item_damage
    }

    /// `setItemDamage(v)`.
    // src: ItemStack.java:116-118
    pub fn set_item_damage(&mut self, v: i32) {
        self.item_damage = v;
    }

    /// `getMaxDamage()`.
    // src: ItemStack.java:120-122
    pub fn get_max_damage(&self) -> i32 {
        self.item().get_max_damage()
    }

    /// `damageItem(n, entity)`, data part: adds `n` to the damage; past the
    /// maximum the stack loses one item (not below 0) and the damage resets.
    /// Returns true when the item broke, where Java adds
    /// `StatList.objectBreakStats[itemID]` if `entity` is an `EntityPlayer`
    /// (the caller's job, WP-2.7).
    // src: ItemStack.java:124-141
    pub fn damage_item(&mut self, n: i32) -> bool {
        if self.is_item_stack_damageable() {
            self.item_damage = self.item_damage.wrapping_add(n);
            if self.item_damage > self.get_max_damage() {
                self.stack_size = self.stack_size.wrapping_sub(1);
                if self.stack_size < 0 {
                    self.stack_size = 0;
                }
                self.item_damage = 0;
                return true;
            }
        }
        false
    }

    /// `damageItem(n, entity)`: the wear ([`ItemStack::damage_item`]), and
    /// when the stack breaks, `objectBreakStats` for a player `entity`.
    // src: ItemStack.java:124-141
    pub fn damage_item_by(&mut self, n: i32, entity: &Entity) {
        if self.damage_item(n) {
            // src: ItemStack.java:128-130 (before the decrement in Java; the
            // stat does not read the stack)
            behaviour::add_stat_if_player(entity, "StatList.objectBreakStats[itemID]", 1);
        }
    }

    /// `useItem(player, world, x, y, z, side)`: `Item.onItemUse`, and on
    /// success the player's `objectUseStats`.
    // src: ItemStack.java:58-65
    #[allow(clippy::too_many_arguments)]
    pub fn use_item(
        &mut self,
        player: &mut Entity,
        world: &mut World,
        x: i32,
        y: i32,
        z: i32,
        side: i32,
    ) -> bool {
        let var7 = behaviour::on_item_use(self, player, world, x, y, z, side);
        if var7 {
            crate::entity::player::add_stat(player, "StatList.objectUseStats[itemID]", 1);
        }
        var7
    }

    /// `useItemRightClick(world, player)`: `Item.onItemRightClick`.
    // src: ItemStack.java:71-73
    pub fn use_item_right_click(
        &mut self,
        world: &mut World,
        player: &mut Entity,
    ) -> RightClickResult {
        behaviour::on_item_right_click(self, world, player)
    }

    /// `hitEntity(target, player)`: `Item.hitEntity(this, target, player)`,
    /// and when true the player's `objectUseStats`.
    // src: ItemStack.java:143-149
    pub fn hit_entity(&mut self, target: &mut Entity, player: &mut Entity, world: &mut World) {
        let var3 = behaviour::hit_entity(self, target, player, world);
        if var3 {
            crate::entity::player::add_stat(player, "StatList.objectUseStats[itemID]", 1);
        }
    }

    /// `onDestroyBlock(blockId, x, y, z, player)`: `Item.onBlockDestroyed`,
    /// and when true the player's `objectUseStats`.
    // src: ItemStack.java:151-157
    #[allow(clippy::too_many_arguments)]
    pub fn on_destroy_block(
        &mut self,
        block_id: i32,
        x: i32,
        y: i32,
        z: i32,
        player: &mut Entity,
        world: &mut World,
    ) {
        let var6 = behaviour::on_block_destroyed(self, block_id, x, y, z, player, world);
        if var6 {
            crate::entity::player::add_stat(player, "StatList.objectUseStats[itemID]", 1);
        }
    }

    /// `useItemOnEntity(target)`: `Item.saddleEntity(this, target)`.
    // src: ItemStack.java:170-172
    pub fn use_item_on_entity(&mut self, target: &mut Entity, world: &mut World) {
        behaviour::saddle_entity(self, target, world);
    }

    /// `updateAnimation(world, entity, slot, isCurrentItem)`: the pick-up
    /// animation counter down by one, then `Item.onUpdate`.
    // src: ItemStack.java:212-218
    pub fn update_animation(
        &mut self,
        world: &mut World,
        entity: &mut Entity,
        slot: i32,
        is_current: bool,
    ) {
        if self.animations_to_go > 0 {
            self.animations_to_go -= 1;
        }
        behaviour::on_update(self, world, entity, slot, is_current);
    }

    /// `onCrafting(world, player)`: the player's `objectCraftStats` (by the
    /// stack size), then `Item.onCreated`.
    // src: ItemStack.java:220-223
    pub fn on_crafting(&mut self, world: &mut World, player: &mut Entity) {
        crate::entity::player::add_stat(
            player,
            "StatList.objectCraftStats[itemID]",
            self.stack_size,
        );
        behaviour::on_created(self, world, player);
    }

    /// `getDamageVsEntity(entity)`.
    // src: ItemStack.java:159-161
    pub fn get_damage_vs_entity(&self) -> i32 {
        self.item().get_damage_vs_entity()
    }

    /// `canHarvestBlock(block)`.
    // src: ItemStack.java:163-165
    pub fn can_harvest_block(&self, block: &BlockDef) -> bool {
        self.item().can_harvest_block(block)
    }

    /// `copy()`: same id, size and damage; `animationsToGo` starts at 0.
    // src: ItemStack.java:174-176
    pub const fn copy(&self) -> ItemStack {
        ItemStack::new(self.item_id, self.stack_size, self.item_damage)
    }

    /// `ItemStack.areItemStacksEqual(a, b)`: both null, or equal size, id
    /// and damage.
    // src: ItemStack.java:178-194
    pub fn are_item_stacks_equal(a: Option<&ItemStack>, b: Option<&ItemStack>) -> bool {
        match (a, b) {
            (None, None) => true,
            (Some(a), Some(b)) => a.is_item_stack_equal(b),
            _ => false,
        }
    }

    /// `isItemStackEqual(other)` (private in Java).
    // src: ItemStack.java:186-194
    fn is_item_stack_equal(&self, other: &ItemStack) -> bool {
        self.stack_size == other.stack_size
            && self.item_id == other.item_id
            && self.item_damage == other.item_damage
    }

    /// `isItemEqual(other)`: same id and damage.
    // src: ItemStack.java:196-198
    pub const fn is_item_equal(&self, other: &ItemStack) -> bool {
        self.item_id == other.item_id && self.item_damage == other.item_damage
    }

    /// `getItemName()`: the item's `getItemNameIS(this)`.
    // src: ItemStack.java:200-202
    pub fn get_item_name(&self) -> Result<Option<String>, JavaException> {
        self.item().get_item_name_is(self)
    }

    /// `ItemStack.copyItemStack(stack)`.
    // src: ItemStack.java:204-206
    pub fn copy_item_stack(stack: Option<&ItemStack>) -> Option<ItemStack> {
        stack.map(ItemStack::copy)
    }

    /// `toString()`: `"<size>x<item.getItemName()>@<damage>"`.
    // src: ItemStack.java:208-210
    pub fn java_to_string(&self) -> String {
        format!(
            "{}x{}@{}",
            self.stack_size,
            self.item().get_item_name().as_deref().unwrap_or("null"),
            self.item_damage
        )
    }

    /// `isStackEqual(other)`: same id, size and damage.
    // src: ItemStack.java:225-227
    pub const fn is_stack_equal(&self, other: &ItemStack) -> bool {
        self.item_id == other.item_id
            && self.stack_size == other.stack_size
            && self.item_damage == other.item_damage
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nbt_round_trip_with_java_casts() {
        // Count 200 is byte -56, damage 70000 is short 4464 (ItemStack.java:77-78).
        let s = ItemStack::new(35, 200, 70000);
        let tag = s.write_to_nbt();
        let c = tag.as_compound().unwrap();
        assert_eq!(c.get("id"), Some(&Tag::Short(35)));
        assert_eq!(c.get("Count"), Some(&Tag::Byte(-56)));
        assert_eq!(c.get("Damage"), Some(&Tag::Short(4464)));
        assert_eq!(ItemStack::from_nbt(&tag), Ok(ItemStack::new(35, -56, 4464)));
    }

    #[test]
    fn nbt_read_defaults_and_type_errors() {
        // Missing keys read as 0 (NBTTagCompound.java:91-97).
        assert_eq!(
            ItemStack::from_nbt(&Tag::compound()),
            Ok(ItemStack::new(0, 0, 0))
        );
        let mut m = BTreeMap::new();
        m.insert("id".to_string(), Tag::Int(1));
        assert_eq!(
            ItemStack::from_nbt(&Tag::Compound(m)),
            Err(JavaException::ClassCast)
        );
        assert_eq!(
            ItemStack::from_nbt(&Tag::Byte(0)),
            Err(JavaException::ClassCast)
        );
    }

    #[test]
    fn read_keeps_animations_to_go() {
        let mut s = ItemStack::new(1, 1, 0);
        s.animations_to_go = 5;
        s.read_from_nbt(&ItemStack::new(4, 2, 0).write_to_nbt())
            .unwrap();
        assert_eq!(s.animations_to_go, 5);
        assert_eq!(s.copy().animations_to_go, 0);
    }

    #[test]
    fn block_and_item_constructors() {
        // Block.sand = 12 (Block.java:604); Item.stick = 280 (Item.java:298).
        let sand = crate::block::block(12).unwrap();
        assert_eq!(ItemStack::from_block(sand, 3, 0), ItemStack::new(12, 3, 0));
        let stick = item(280).unwrap();
        assert_eq!(ItemStack::from_item(stick, 1, 0), ItemStack::new(280, 1, 0));
    }

    #[test]
    fn equality_helpers() {
        let a = ItemStack::new(1, 2, 3);
        let b = ItemStack::new(1, 5, 3);
        assert!(a.is_item_equal(&b));
        assert!(!a.is_stack_equal(&b));
        assert!(ItemStack::are_item_stacks_equal(None, None));
        assert!(!ItemStack::are_item_stacks_equal(Some(&a), None));
        assert!(!ItemStack::are_item_stacks_equal(Some(&a), Some(&b)));
        assert!(ItemStack::are_item_stacks_equal(Some(&a), Some(&a.copy())));
        assert_eq!(ItemStack::copy_item_stack(None), None);
    }

    #[test]
    fn java_to_string() {
        assert_eq!(
            ItemStack::new(276, 1, 7).java_to_string(),
            "1xitem.swordDiamond@7"
        );
        assert_eq!(ItemStack::new(1, 64, 0).java_to_string(), "64xtile.stone@0");
        // The piston extension's ItemBlock has no name (Block.java:626).
        assert_eq!(ItemStack::new(34, 1, 0).java_to_string(), "1xnull@0");
    }
}
