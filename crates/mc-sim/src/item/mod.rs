//! Items (`Item.java`, `ItemStack.java`).
//!
//! Only [`ItemStack`] and an [`ItemDef`] skeleton exist so far. `Item.java`'s
//! static table (every item id, max stack size, max damage, icon, name) is
//! WP-1.2; item behaviour (`onItemUse`, `onItemRightClick`, …) is WP-2.6.

/// `ItemStack` (ItemStack.java:3-7). Vanilla's `ItemStack` is a mutable
/// object shared by reference (an `EntityItem` and an inventory slot can hold
/// the same one); the port stores stacks by value and copies where Java
/// aliases, which every current call site allows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemStack {
    /// `stackSize`.
    pub stack_size: i32,
    /// `animationsToGo` (the pick-up bob in the hotbar; client-only in effect).
    pub animations_to_go: i32,
    /// `itemID` (block ids 0–255, items 256+).
    pub item_id: i32,
    /// `itemDamage` (private in Java; `getItemDamage()`).
    pub item_damage: i32,
}

impl ItemStack {
    /// `new ItemStack(id, count, damage)`.
    // src: ItemStack.java:33-38
    pub const fn new(item_id: i32, stack_size: i32, item_damage: i32) -> Self {
        ItemStack {
            stack_size,
            animations_to_go: 0,
            item_id,
            item_damage,
        }
    }

    /// `splitStack(n)`: takes `n` off this stack into a new one.
    // src: ItemStack.java:45-48
    pub fn split_stack(&mut self, n: i32) -> ItemStack {
        self.stack_size -= n;
        ItemStack::new(self.item_id, n, self.item_damage)
    }

    /// `getItemDamage()`.
    // src: ItemStack.java:112-114
    pub const fn get_item_damage(&self) -> i32 {
        self.item_damage
    }

    /// `writeToNBT(compound)`: `id` short, `Count` byte, `Damage` short.
    // src: ItemStack.java:75-80
    pub fn write_to_nbt(&self) -> mc_nbt::Tag {
        let mut m = std::collections::BTreeMap::new();
        m.insert("id".to_string(), mc_nbt::Tag::Short(self.item_id as i16));
        m.insert(
            "Count".to_string(),
            mc_nbt::Tag::Byte(self.stack_size as i8),
        );
        m.insert(
            "Damage".to_string(),
            mc_nbt::Tag::Short(self.item_damage as i16),
        );
        mc_nbt::Tag::Compound(m)
    }
}

/// The per-item data `Item.java`'s static initialiser fills in. Skeleton: only
/// what `EntityItem` and inventories need first. WP-1.2 completes it from
/// Item.java and the tool/armour enums.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemDef {
    /// `shiftedIndex`: the item id (`256 + constructor id`).
    pub shifted_index: i32,
    /// `maxStackSize` (64 unless set).
    pub max_stack_size: i32,
    /// `maxDamage`.
    pub max_damage: i32,
}
