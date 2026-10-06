//! Inventories, containers and slots (`IInventory` and its implementations,
//! `Container*.java`, `Slot*.java`; WP-2.5).
//!
//! # Shape
//!
//! | Java | Rust |
//! |---|---|
//! | `InventoryPlayer` | [`player::InventoryPlayer`] (in the player's `PlayerData`); the methods that reach the owning player or the world are free functions in [`player`] |
//! | `InventoryCrafting`, `InventoryCraftResult` | [`crafting`] (owned by their container) |
//! | `InventoryLargeChest` | [`large_chest::InventoryLargeChest`] |
//! | `InventoryBasic` | [`basic::InventoryBasic`] (only the multiplayer client builds one, NetClientHandler.java:488: deferred, D3) |
//! | `TileEntityChest` / `TileEntityFurnace` / `TileEntityDispenser` as `IInventory` | [`crate::tile_entity`] (WP-1.8), reached through a [`crate::world::tile_entity::TileEntityRef`] |
//! | `IInventory` (an object reference held by a `Slot`) | [`InventoryRef`] |
//! | `Container` + `ContainerPlayer`, `ContainerWorkbench`, `ContainerChest`, `ContainerFurnace`, `ContainerDispenser` | [`container::Container`] with a [`container::ContainerKind`] |
//! | `Slot`, `SlotArmor`, `SlotCrafting`, `SlotFurnace` | [`slot::Slot`] with a [`slot::SlotKind`] |
//!
//! A `Slot` holds an `IInventory` reference. The objects behind it live in
//! three places in the port: the player's [`InventoryPlayer`] (inside the
//! player entity), the container's own crafting grid and result, and the
//! world's tile entities. Every `IInventory` call therefore goes through
//! the container ([`container::Container`]'s `inv_*` methods), which is
//! given the world and the player entity (`EntityPlayer`, the one the
//! container was built for: `InventoryPlayer.player`, `SlotCrafting` /
//! `SlotFurnace`'s `thePlayer`).
//!
//! # Aliasing
//!
//! Vanilla `ItemStack`s are shared mutable objects: `Slot.getStack()`
//! returns the stored object, and `Container.slotClick` / `mergeItemStack`
//! change `stackSize` on it in place, with no `setInventorySlotContents` and
//! so no change event. The port copies stacks by value and writes such
//! in-place changes back through `Container::inv_stack_mut`, which, like
//! Java's field write, raises no event. Each site says which Java object it
//! stands for.
//!
//! # Gaps (recorded in [`World::unimplemented_hits`])
//!
//! - `EntityPlayer.dropPlayerItemWithRandomChoice` (WP-2.5a's player code):
//!   [`drop_player_item_with_random_choice`] records it and drops nothing.
//! - `EntityPlayer.addStat` (statistics and achievements, WP-2.7): crafting
//!   and smelting pickups (`ItemStack.onCrafting`, `SlotCrafting`,
//!   `SlotFurnace`) and armour breaking record it.
//! - `ItemMap.onCreated` / `ItemMap.onUpdate` (item behaviour, WP-2.6).
//!
//! The client-side parts of `Container` are left out: `crafters`
//! (`ICrafting` listeners, which only the GUI and the network code
//! register; never any in singleplayer), `playerList` (unused in the
//! client), and the transaction-id echo methods `func_20113_a` /
//! `func_20110_b` (empty).
//!
//! [`World::unimplemented_hits`]: crate::world::World::unimplemented_hits

pub mod basic;
pub mod container;
pub mod crafting;
pub mod large_chest;
pub mod player;
pub mod slot;

pub use player::InventoryPlayer;

use crate::entity::Entity;
use crate::item::ItemStack;
use crate::world::World;
use crate::world::tile_entity::TileEntityRef;

/// An `IInventory` reference as a `Slot` (or `InventoryLargeChest`, or
/// `ContainerChest.lowerChestInventory`) holds it.
#[derive(Clone, Debug, PartialEq)]
pub enum InventoryRef {
    /// The `InventoryPlayer` of the container's player.
    Player,
    /// The container's own `craftMatrix` (`ContainerPlayer`,
    /// `ContainerWorkbench`).
    CraftMatrix,
    /// The container's own `craftResult`.
    CraftResult,
    /// A chest, furnace or dispenser tile entity.
    TileEntity(TileEntityRef),
    /// An `InventoryLargeChest` (a double chest).
    LargeChest(Box<large_chest::InventoryLargeChest>),
}

/// `player.dropPlayerItemWithRandomChoice(stack, flag)` (EntityPlayer.java:
/// 257-283): an `EntityItem` at the player's eyes with random motion.
/// `EntityPlayer`'s own methods are WP-2.5a's (`entity/player.rs`); until
/// that function exists this records `"EntityPlayer.dropPlayerItemWithRandomChoice"`
/// and the stack is lost (the caller has already taken it out of its
/// slot, as Java does). A null stack does nothing (EntityPlayer.java:258).
pub fn drop_player_item_with_random_choice(
    world: &mut World,
    player: &mut Entity,
    stack: Option<ItemStack>,
    flag: bool,
) {
    let _ = (player, flag);
    if stack.is_some() {
        world.note_unimplemented("EntityPlayer.dropPlayerItemWithRandomChoice");
    }
}

/// `player.dropPlayerItem(stack)`: `dropPlayerItemWithRandomChoice(stack,
/// false)`.
// src: EntityPlayer.java:253-255
pub fn drop_player_item(world: &mut World, player: &mut Entity, stack: Option<ItemStack>) {
    drop_player_item_with_random_choice(world, player, stack, false);
}

/// `stack.onCrafting(world, player)`: `player.addStat(objectCraftStats[id],
/// stackSize)` (WP-2.7, recorded) and `Item.onCreated` (empty in `Item`;
/// `ItemMap.onCreated` assigns a new map id: item behaviour, WP-2.6,
/// recorded and not applied).
// src: ItemStack.java:220-223, Item.java:270-271, ItemMap.java:228-235
pub fn item_stack_on_crafting(world: &mut World, player: &mut Entity, stack: &ItemStack) {
    let _ = player;
    world.note_unimplemented("EntityPlayer.addStat");
    if stack.item_id == crate::crafting::ids::MAP_ITEM {
        world.note_unimplemented("ItemMap.onCreated");
    }
}

/// The player's `InventoryPlayer`; panics where Java's `EntityPlayer`
/// cast would throw.
pub(crate) fn inventory_of(player: &Entity) -> &InventoryPlayer {
    &player
        .kind
        .player()
        .expect("an inventory's player is an EntityPlayer (Java ClassCastException)")
        .inventory
}

/// The player's `InventoryPlayer`, mutably.
pub(crate) fn inventory_of_mut(player: &mut Entity) -> &mut InventoryPlayer {
    &mut player
        .kind
        .player_mut()
        .expect("an inventory's player is an EntityPlayer (Java ClassCastException)")
        .inventory
}
