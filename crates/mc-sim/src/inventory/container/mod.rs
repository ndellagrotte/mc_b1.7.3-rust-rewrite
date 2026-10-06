//! `Container` (Container.java) and the five concrete containers, plus the
//! `IInventory` dispatch every slot goes through.
//!
//! | Java | Rust |
//! |---|---|
//! | `ContainerPlayer` | [`player`]: [`player::new_container_player`] |
//! | `ContainerWorkbench` | [`workbench`] |
//! | `ContainerChest` | [`chest`] |
//! | `ContainerFurnace` | [`furnace`] |
//! | `ContainerDispenser` | [`dispenser`] |
//!
//! Every method that touches a slot takes the world and the player entity
//! the container was built for (`InventoryPlayer.player`): see the
//! [module docs](super) for where the inventories live and how in-place
//! `ItemStack` changes are written back.
//!
//! `getStackInSlot(int)` is overridden by four of the five containers to
//! move the slot's stack to the other part of the window (b1.7.3's
//! shift-click; later versions call it `transferStackInSlot`):
//! [`Container::get_stack_in_slot`] dispatches on the kind.

pub mod chest;
pub mod dispenser;
pub mod furnace;
pub mod player;
pub mod workbench;

use super::crafting::{InventoryCraftResult, InventoryCrafting};
use super::slot::Slot;
use super::{InventoryRef, drop_player_item, inventory_of, inventory_of_mut};
use crate::crafting::CraftingManager;
use crate::entity::Entity;
use crate::item::ItemStack;
use crate::tile_entity;
use crate::world::World;
use crate::world::tile_entity::{TileEntityKindData, TileEntityRef};

/// The concrete container class and its own fields.
#[derive(Clone, Debug, PartialEq)]
pub enum ContainerKind {
    /// `ContainerPlayer` (EntityPlayer.inventorySlots).
    Player {
        /// `craftMatrix`: 2×2.
        craft_matrix: InventoryCrafting,
        /// `craftResult`.
        craft_result: InventoryCraftResult,
        /// `isSinglePlayer`.
        is_single_player: bool,
    },
    /// `ContainerWorkbench`.
    Workbench {
        /// `craftMatrix`: 3×3.
        craft_matrix: InventoryCrafting,
        /// `craftResult`.
        craft_result: InventoryCraftResult,
        /// `posX`, `posY`, `posZ` (private): the workbench block.
        pos: (i32, i32, i32),
    },
    /// `ContainerChest`.
    Chest {
        /// `lowerChestInventory` (private): the chest's inventory.
        lower_chest_inventory: InventoryRef,
        /// `numRows` (private).
        num_rows: i32,
    },
    /// `ContainerFurnace`.
    Furnace {
        /// `furnace` (private).
        furnace: TileEntityRef,
        /// `cookTime` (private): the last value sent to the crafters.
        cook_time: i32,
        /// `burnTime` (private).
        burn_time: i32,
        /// `itemBurnTime` (private).
        item_burn_time: i32,
    },
    /// `ContainerDispenser`.
    Dispenser {
        /// `tileEntityDispenser` (private).
        tile_entity_dispenser: TileEntityRef,
    },
}

/// `Container`.
#[derive(Clone, Debug, PartialEq)]
pub struct Container {
    /// `inventoryItemStacks`: per slot, the copy last seen by
    /// `updateCraftingResults`.
    pub inventory_item_stacks: Vec<Option<ItemStack>>,
    /// `slots`.
    pub slots: Vec<Slot>,
    /// `windowId`.
    pub window_id: i32,
    /// `transactionID` (private).
    pub transaction_id: i16,
    /// The class.
    pub kind: ContainerKind,
}

/// The tile entity's slots, mutably (`None` for a class that is not an
/// inventory).
fn te_slots_mut(kind: &mut TileEntityKindData) -> Option<&mut [Option<ItemStack>]> {
    match kind {
        TileEntityKindData::Chest(d) => Some(&mut d.chest_contents),
        TileEntityKindData::Furnace(d) => Some(&mut d.furnace_item_stacks),
        TileEntityKindData::Dispenser(d) => Some(&mut d.dispenser_contents),
        _ => None,
    }
}

/// A container's tile entity that is no longer in the world's store
/// (Java would keep using the orphaned object; unreachable from the client,
/// which closes the window when `isUsableByPlayer` fails).
fn te_gone(world: &World) {
    world.note_unimplemented("Container over a tile entity no longer in the world");
}

impl Container {
    /// A container of `kind` with no slots yet (the field initialisers of
    /// Container.java:9-14).
    pub(crate) fn empty(kind: ContainerKind) -> Container {
        Container {
            inventory_item_stacks: Vec::new(),
            slots: Vec::new(),
            window_id: 0,
            transaction_id: 0,
            kind,
        }
    }

    // ---- IInventory dispatch ------------------------------------------------------

    /// `inventory.getSizeInventory()`.
    // src: IInventory.java:4, InventoryPlayer.java:244-246, InventoryCrafting.java:15-17,
    // InventoryCraftResult.java:6-8, InventoryLargeChest.java:14-16
    pub fn inv_get_size_inventory(&self, world: &World, inv: &InventoryRef) -> i32 {
        match inv {
            InventoryRef::Player => 40,
            InventoryRef::CraftMatrix => self.craft_matrix().get_size_inventory(),
            InventoryRef::CraftResult => 1,
            InventoryRef::TileEntity(r) => match world.tile_entities.get(r) {
                Some(t) => tile_entity::get_size_inventory(&t.kind).unwrap_or(0),
                None => {
                    te_gone(world);
                    0
                }
            },
            InventoryRef::LargeChest(lc) => self
                .inv_get_size_inventory(world, &lc.upper_chest)
                .wrapping_add(self.inv_get_size_inventory(world, &lc.lower_chest)),
        }
    }

    /// `inventory.getStackInSlot(index)` (a copy; see the module docs).
    // src: IInventory.java:6, InventoryLargeChest.java:22-24
    pub fn inv_get_stack_in_slot(
        &self,
        world: &mut World,
        player: &mut Entity,
        inv: &InventoryRef,
        idx: i32,
    ) -> Option<ItemStack> {
        match inv {
            InventoryRef::Player => inventory_of(player).get_stack_in_slot(idx).copied(),
            InventoryRef::CraftMatrix => self.craft_matrix().get_stack_in_slot(idx),
            InventoryRef::CraftResult => self.craft_result().get_stack_in_slot(idx),
            InventoryRef::TileEntity(r) => match world.tile_entities.get(r) {
                Some(t) => tile_entity::get_stack_in_slot(&t.kind, idx),
                None => {
                    te_gone(world);
                    None
                }
            },
            InventoryRef::LargeChest(lc) => {
                let us = self.inv_get_size_inventory(world, &lc.upper_chest);
                if idx >= us {
                    self.inv_get_stack_in_slot(world, player, &lc.lower_chest, idx - us)
                } else {
                    self.inv_get_stack_in_slot(world, player, &lc.upper_chest, idx)
                }
            }
        }
    }

    /// `inventory.decrStackSize(index, n)`; the crafting grid then runs
    /// `eventHandler.onCraftMatrixChanged`.
    // src: IInventory.java:8, InventoryCrafting.java:36-56, InventoryLargeChest.java:26-28
    pub fn inv_decr_stack_size(
        &mut self,
        world: &mut World,
        player: &mut Entity,
        inv: &InventoryRef,
        idx: i32,
        n: i32,
    ) -> Option<ItemStack> {
        match inv {
            InventoryRef::Player => inventory_of_mut(player).decr_stack_size(idx, n),
            InventoryRef::CraftMatrix => {
                let r = self.craft_matrix_mut().decr_stack_size(idx, n);
                if r.is_some() {
                    // src: InventoryCrafting.java:44, :52 (not on the null path)
                    self.on_craft_matrix_changed(world, player);
                }
                r
            }
            InventoryRef::CraftResult => self.craft_result_mut().decr_stack_size(idx, n),
            InventoryRef::TileEntity(r) => {
                if world.tile_entities.contains_key(r) {
                    tile_entity::decr_stack_size(world, *r, idx, n)
                } else {
                    te_gone(world);
                    None
                }
            }
            InventoryRef::LargeChest(lc) => {
                let us = self.inv_get_size_inventory(world, &lc.upper_chest);
                if idx >= us {
                    self.inv_decr_stack_size(world, player, &lc.lower_chest, idx - us, n)
                } else {
                    self.inv_decr_stack_size(world, player, &lc.upper_chest, idx, n)
                }
            }
        }
    }

    /// `inventory.setInventorySlotContents(index, stack)`; the crafting grid
    /// then runs `eventHandler.onCraftMatrixChanged`.
    // src: IInventory.java:10, InventoryCrafting.java:58-61, InventoryLargeChest.java:30-37
    pub fn inv_set_inventory_slot_contents(
        &mut self,
        world: &mut World,
        player: &mut Entity,
        inv: &InventoryRef,
        idx: i32,
        stack: Option<ItemStack>,
    ) {
        match inv {
            InventoryRef::Player => {
                inventory_of_mut(player).set_inventory_slot_contents(idx, stack)
            }
            InventoryRef::CraftMatrix => {
                self.craft_matrix_mut()
                    .set_inventory_slot_contents(idx, stack);
                self.on_craft_matrix_changed(world, player);
            }
            InventoryRef::CraftResult => self
                .craft_result_mut()
                .set_inventory_slot_contents(idx, stack),
            InventoryRef::TileEntity(r) => {
                if world.tile_entities.contains_key(r) {
                    tile_entity::set_inventory_slot_contents(world, *r, idx, stack);
                } else {
                    te_gone(world);
                }
            }
            InventoryRef::LargeChest(lc) => {
                let us = self.inv_get_size_inventory(world, &lc.upper_chest);
                if idx >= us {
                    self.inv_set_inventory_slot_contents(
                        world,
                        player,
                        &lc.lower_chest,
                        idx - us,
                        stack,
                    );
                } else {
                    self.inv_set_inventory_slot_contents(
                        world,
                        player,
                        &lc.upper_chest,
                        idx,
                        stack,
                    );
                }
            }
        }
    }

    /// `inventory.getInventoryStackLimit()`: 64 everywhere (the large
    /// chest asks its upper half).
    // src: IInventory.java:14, InventoryLargeChest.java:39-41
    pub fn inv_get_inventory_stack_limit(&self, inv: &InventoryRef) -> i32 {
        match inv {
            InventoryRef::Player => super::player::INVENTORY_STACK_LIMIT,
            InventoryRef::CraftMatrix => self.craft_matrix().get_inventory_stack_limit(),
            InventoryRef::CraftResult => self.craft_result().get_inventory_stack_limit(),
            InventoryRef::TileEntity(_) => tile_entity::INVENTORY_STACK_LIMIT,
            InventoryRef::LargeChest(lc) => self.inv_get_inventory_stack_limit(&lc.upper_chest),
        }
    }

    /// `inventory.onInventoryChanged()`: the player's sets its flag, the
    /// crafting inventories do nothing, a tile entity tells the world, the
    /// large chest calls both halves.
    // src: IInventory.java:16, InventoryPlayer.java:339-341, InventoryCrafting.java:67-68,
    // InventoryCraftResult.java:36-37, TileEntity.java:70-75, InventoryLargeChest.java:43-46
    pub fn inv_on_inventory_changed(
        &mut self,
        world: &mut World,
        player: &mut Entity,
        inv: &InventoryRef,
    ) {
        match inv {
            InventoryRef::Player => inventory_of_mut(player).on_inventory_changed(),
            InventoryRef::CraftMatrix | InventoryRef::CraftResult => {}
            InventoryRef::TileEntity(r) => tile_entity::on_inventory_changed(world, *r),
            InventoryRef::LargeChest(lc) => {
                self.inv_on_inventory_changed(world, player, &lc.upper_chest);
                self.inv_on_inventory_changed(world, player, &lc.lower_chest);
            }
        }
    }

    /// `inventory.canInteractWith(player)`: the player's own within 8
    /// blocks of a live owner (the container's player is the owner), the
    /// crafting inventories always, a tile entity still in place and within
    /// 8 blocks, the large chest both halves.
    // src: IInventory.java:18, InventoryPlayer.java:352-358, InventoryCrafting.java:70-72,
    // TileEntityChest.java:84-90, InventoryLargeChest.java:48-50
    pub fn inv_can_interact_with(
        &self,
        world: &mut World,
        player: &Entity,
        inv: &InventoryRef,
    ) -> bool {
        match inv {
            InventoryRef::Player => super::player::can_interact_with(player, player),
            InventoryRef::CraftMatrix | InventoryRef::CraftResult => true,
            InventoryRef::TileEntity(r) => tile_entity::can_interact_with(world, *r, &player.base),
            InventoryRef::LargeChest(lc) => {
                self.inv_can_interact_with(world, player, &lc.upper_chest)
                    && self.inv_can_interact_with(world, player, &lc.lower_chest)
            }
        }
    }

    /// The stored stack of `inventory[idx]`, changed in place by `f`: Java's
    /// field writes on a shared `ItemStack` (no `setInventorySlotContents`,
    /// no event, no size clamp).
    pub fn inv_stack_mut<R>(
        &mut self,
        world: &mut World,
        player: &mut Entity,
        inv: &InventoryRef,
        idx: i32,
        f: impl FnOnce(&mut Option<ItemStack>) -> R,
    ) -> R {
        match inv {
            InventoryRef::Player => f(inventory_of_mut(player).stack_mut(idx)),
            InventoryRef::CraftMatrix => f(self.craft_matrix_mut().stack_mut(idx)),
            InventoryRef::CraftResult => f(self.craft_result_mut().stack_mut(idx)),
            InventoryRef::TileEntity(r) => {
                let slots = world
                    .tile_entities
                    .get_mut(r)
                    .and_then(|t| te_slots_mut(&mut t.kind));
                match slots {
                    Some(s) => {
                        let len = s.len();
                        match usize::try_from(idx) {
                            Ok(i) if i < len => f(&mut s[i]),
                            _ => panic!("ArrayIndexOutOfBoundsException: {idx} (length {len})"),
                        }
                    }
                    None => {
                        te_gone(world);
                        f(&mut None)
                    }
                }
            }
            InventoryRef::LargeChest(lc) => {
                let us = self.inv_get_size_inventory(world, &lc.upper_chest);
                if idx >= us {
                    self.inv_stack_mut(world, player, &lc.lower_chest, idx - us, f)
                } else {
                    self.inv_stack_mut(world, player, &lc.upper_chest, idx, f)
                }
            }
        }
    }

    /// The container's `craftMatrix`; panics for a container without one
    /// (no `Slot` of such a container names it).
    pub fn craft_matrix(&self) -> &InventoryCrafting {
        match &self.kind {
            ContainerKind::Player { craft_matrix, .. }
            | ContainerKind::Workbench { craft_matrix, .. } => craft_matrix,
            _ => panic!("craftMatrix of a container without one"),
        }
    }

    fn craft_matrix_mut(&mut self) -> &mut InventoryCrafting {
        match &mut self.kind {
            ContainerKind::Player { craft_matrix, .. }
            | ContainerKind::Workbench { craft_matrix, .. } => craft_matrix,
            _ => panic!("craftMatrix of a container without one"),
        }
    }

    /// The container's `craftResult`; panics for a container without one.
    pub fn craft_result(&self) -> &InventoryCraftResult {
        match &self.kind {
            ContainerKind::Player { craft_result, .. }
            | ContainerKind::Workbench { craft_result, .. } => craft_result,
            _ => panic!("craftResult of a container without one"),
        }
    }

    fn craft_result_mut(&mut self) -> &mut InventoryCraftResult {
        match &mut self.kind {
            ContainerKind::Player { craft_result, .. }
            | ContainerKind::Workbench { craft_result, .. } => craft_result,
            _ => panic!("craftResult of a container without one"),
        }
    }

    // ---- Container ------------------------------------------------------------------

    /// `addSlot(slot)`: numbered in order, with a null entry in
    /// `inventoryItemStacks`.
    // src: Container.java:16-20
    pub fn add_slot(&mut self, mut var1: Slot) {
        var1.slot_number = self.slots.len() as i32;
        self.slots.push(var1);
        self.inventory_item_stacks.push(None);
    }

    /// `updateCraftingResults()`: refreshes `inventoryItemStacks` with
    /// copies of the slots whose stack changed (and would send them to the
    /// `crafters`, never any in singleplayer). `ContainerFurnace` then
    /// records the furnace's three timers.
    // src: Container.java:22-36, ContainerFurnace.java:26-47
    pub fn update_crafting_results(&mut self, world: &mut World, player: &mut Entity) {
        for var1 in 0..self.slots.len() {
            let var2 = self.slot_get_stack(world, player, var1);
            let var3 = self.inventory_item_stacks[var1];
            if !ItemStack::are_item_stacks_equal(var3.as_ref(), var2.as_ref()) {
                self.inventory_item_stacks[var1] = var2.map(|s| s.copy());
                // crafters.sendSlotContents(...): no crafters (module docs)
            }
        }
        if let ContainerKind::Furnace {
            furnace,
            cook_time,
            burn_time,
            item_burn_time,
        } = &mut self.kind
        {
            // crafters.updateCraftingInventoryInfo(...): no crafters
            if let Some(TileEntityKindData::Furnace(f)) =
                world.tile_entities.get(furnace).map(|t| &t.kind)
            {
                *cook_time = f.furnace_cook_time;
                *burn_time = f.furnace_burn_time;
                *item_burn_time = f.current_item_burn_time;
            }
        }
    }

    /// `getSlot(i)`.
    // src: Container.java:38-40
    pub fn get_slot(&self, var1: usize) -> &Slot {
        &self.slots[var1]
    }

    /// `getStackInSlot(i)`: for the player, workbench, chest and furnace
    /// containers the shift-click move (their overrides, see the kind
    /// modules); for the dispenser the base method, the slot's stack.
    /// Panics where Java's `slots.get(i)` throws `IndexOutOfBoundsException`.
    // src: Container.java:42-45
    pub fn get_stack_in_slot(
        &mut self,
        world: &mut World,
        player: &mut Entity,
        var1: usize,
    ) -> Option<ItemStack> {
        match self.kind {
            ContainerKind::Player { .. } => self.player_get_stack_in_slot(world, player, var1),
            ContainerKind::Workbench { .. } => {
                self.workbench_get_stack_in_slot(world, player, var1)
            }
            ContainerKind::Chest { .. } => self.chest_get_stack_in_slot(world, player, var1),
            ContainerKind::Furnace { .. } => self.furnace_get_stack_in_slot(world, player, var1),
            ContainerKind::Dispenser { .. } => {
                let _ = &self.slots[var1];
                self.slot_get_stack(world, player, var1)
            }
        }
    }

    /// `slotClick(slot, button, shift, player)`: the inventory window's
    /// mouse click, with `inventory.getItemStack()` as the stack on the
    /// cursor. Only buttons 0 (left) and 1 (right) do anything. Returns a
    /// copy of the clicked slot's stack from before the click (null for
    /// an empty slot and for −999).
    ///
    /// - slot −999 (outside the window): drop the held stack (left) or one
    ///   item of it (right) through `player.dropPlayerItem`.
    /// - shift: `getStackInSlot(slot)` (the move); repeated while it took
    ///   some but not all of the slot's stack.
    /// - otherwise, after `onSlotChanged`: put the held stack (left) or one
    ///   item (right) into an empty valid slot (up to the slot's limit);
    ///   pick up the slot's stack (left) or its larger half (right) into an
    ///   empty hand; add the held stack (or one) to a matching valid slot
    ///   up to the slot's and the item's limits; swap a held stack that
    ///   fits with a different one; and for an output slot (not valid),
    ///   take the slot's whole stack onto a matching held stack when it all
    ///   fits.
    ///
    /// Panics where Java's `slots.get(slot)` throws (a slot other than
    /// −999 outside the list).
    // src: Container.java:47-150
    pub fn slot_click(
        &mut self,
        world: &mut World,
        player: &mut Entity,
        var1: i32,
        var2: i32,
        var3: bool,
    ) -> Option<ItemStack> {
        let mut var5 = None;
        if var2 != 0 && var2 != 1 {
            return var5;
        }
        if var1 == -999 {
            let held = inventory_of(player).get_item_stack();
            if held.is_some() {
                if var2 == 0 {
                    // dropPlayerItem(var6.getItemStack()): the held object itself
                    drop_player_item(world, player, held);
                    inventory_of_mut(player).set_item_stack(None);
                }
                if var2 == 1 {
                    // var6.getItemStack().splitStack(1): the held object loses one
                    let part = inventory_of_mut(player)
                        .item_stack
                        .as_mut()
                        .map(|h| h.split_stack(1));
                    drop_player_item(world, player, part);
                    if inventory_of(player)
                        .get_item_stack()
                        .is_some_and(|h| h.stack_size == 0)
                    {
                        inventory_of_mut(player).set_item_stack(None);
                    }
                }
            }
            return var5;
        }
        let idx = usize::try_from(var1)
            .ok()
            .filter(|&i| i < self.slots.len())
            .unwrap_or_else(|| panic!("Container.slotClick({var1}): IndexOutOfBoundsException"));
        if var3 {
            let var7 = self.get_stack_in_slot(world, player, idx);
            if let Some(var7) = var7 {
                let var8 = var7.stack_size;
                var5 = Some(var7.copy());
                if let Some(s) = self.slot_get_stack(world, player, idx) {
                    let var10 = s.stack_size;
                    if var10 < var8 {
                        self.slot_click(world, player, var1, var2, var3);
                    }
                }
            }
            return var5;
        }
        self.slot_on_slot_changed(world, player, idx);
        let var13 = self.slot_get_stack(world, player, idx);
        let var14 = inventory_of(player).get_item_stack();
        if let Some(v13) = var13 {
            var5 = Some(v13.copy());
        }
        match (var13, var14) {
            (None, None) => {}
            (None, Some(v14)) => {
                if self.slots[idx].is_item_valid(&v14) {
                    let mut var10 = if var2 == 0 { v14.stack_size } else { 1 };
                    let lim = self.slot_get_slot_stack_limit(idx);
                    if var10 > lim {
                        var10 = lim;
                    }
                    // var12.putStack(var14.splitStack(var10)): the held object shrinks first
                    let part = inventory_of_mut(player)
                        .item_stack
                        .as_mut()
                        .map(|h| h.split_stack(var10));
                    self.slot_put_stack(world, player, idx, part);
                    if inventory_of(player)
                        .get_item_stack()
                        .is_some_and(|h| h.stack_size == 0)
                    {
                        inventory_of_mut(player).set_item_stack(None);
                    }
                }
            }
            (Some(v13), None) => {
                let var10 = if var2 == 0 {
                    v13.stack_size
                } else {
                    v13.stack_size.wrapping_add(1) / 2
                };
                let var11 = self.slot_decr_stack_size(world, player, idx, var10);
                inventory_of_mut(player).set_item_stack(var11);
                // var13 after decrStackSize: still in the slot (split off), or,
                // with the slot emptied, the very object now held.
                let var13_size = match self.slot_get_stack(world, player, idx) {
                    Some(s) => s.stack_size,
                    None => var11.map_or(0, |s| s.stack_size),
                };
                if var13_size == 0 {
                    self.slot_put_stack(world, player, idx, None);
                }
                let held = inventory_of(player).get_item_stack();
                self.slot_on_pickup_from_slot(world, player, idx, held);
            }
            (Some(v13), Some(v14)) => {
                if self.slots[idx].is_item_valid(&v14) {
                    if v13.item_id == v14.item_id
                        && (!v13.get_has_subtypes()
                            || v13.get_item_damage() == v14.get_item_damage())
                    {
                        let mut var10 = if var2 == 0 { v14.stack_size } else { 1 };
                        let lim = self.slot_get_slot_stack_limit(idx);
                        if var10 > lim - v13.stack_size {
                            var10 = lim - v13.stack_size;
                        }
                        if var10 > v14.get_max_stack_size() - v13.stack_size {
                            var10 = v14.get_max_stack_size() - v13.stack_size;
                        }
                        // var14.splitStack(var10): the held object shrinks
                        if let Some(h) = inventory_of_mut(player).item_stack.as_mut() {
                            h.split_stack(var10);
                        }
                        if inventory_of(player)
                            .get_item_stack()
                            .is_some_and(|h| h.stack_size == 0)
                        {
                            inventory_of_mut(player).set_item_stack(None);
                        }
                        // var13.stackSize += var10: the slot's object, in place
                        self.slot_stack_mut(world, player, idx, |s| {
                            if let Some(s) = s.as_mut() {
                                s.stack_size = s.stack_size.wrapping_add(var10);
                            }
                        });
                    } else if v14.stack_size <= self.slot_get_slot_stack_limit(idx) {
                        self.slot_put_stack(world, player, idx, Some(v14));
                        inventory_of_mut(player).set_item_stack(Some(v13));
                    }
                } else if v13.item_id == v14.item_id
                    && v14.get_max_stack_size() > 1
                    && (!v13.get_has_subtypes() || v13.get_item_damage() == v14.get_item_damage())
                {
                    let var10 = v13.stack_size;
                    if var10 > 0 && var10.wrapping_add(v14.stack_size) <= v14.get_max_stack_size() {
                        // var14.stackSize += var10: the held object
                        if let Some(h) = inventory_of_mut(player).item_stack.as_mut() {
                            h.stack_size = h.stack_size.wrapping_add(var10);
                        }
                        // var13.splitStack(var10): the slot's object, in place
                        let left = self.slot_stack_mut(world, player, idx, |s| {
                            s.as_mut().map(|s| {
                                s.split_stack(var10);
                                s.stack_size
                            })
                        });
                        if left == Some(0) {
                            self.slot_put_stack(world, player, idx, None);
                        }
                        // vanilla bug: onPickupFromSlot gets the whole held stack, so a
                        // crafting output's onCrafting counts the held items too
                        // (objectCraftStats, WP-2.7).
                        let held = inventory_of(player).get_item_stack();
                        self.slot_on_pickup_from_slot(world, player, idx, held);
                    }
                }
            }
        }
        var5
    }

    /// `onCraftGuiClosed(player)`: the held stack is dropped
    /// (`dropPlayerItem`) and the hand emptied; then `ContainerPlayer`
    /// drops and clears its grid, and `ContainerWorkbench` (not on a
    /// multiplayer world) drops its grid's stacks without clearing it
    /// (vanilla: the container is discarded right after).
    // src: Container.java:152-159, ContainerPlayer.java:48-59, ContainerWorkbench.java:39-50
    pub fn on_craft_gui_closed(&mut self, world: &mut World, player: &mut Entity) {
        let var2 = inventory_of(player).get_item_stack();
        if var2.is_some() {
            drop_player_item(world, player, var2);
            inventory_of_mut(player).set_item_stack(None);
        }
        match self.kind {
            ContainerKind::Player { .. } => {
                for var2 in 0..4 {
                    let var3 = self.craft_matrix().get_stack_in_slot(var2);
                    if var3.is_some() {
                        drop_player_item(world, player, var3);
                        self.inv_set_inventory_slot_contents(
                            world,
                            player,
                            &InventoryRef::CraftMatrix,
                            var2,
                            None,
                        );
                    }
                }
            }
            ContainerKind::Workbench { .. } if !world.multiplayer_world => {
                for var2 in 0..9 {
                    let var3 = self.craft_matrix().get_stack_in_slot(var2);
                    if var3.is_some() {
                        drop_player_item(world, player, var3);
                    }
                }
            }
            _ => {}
        }
    }

    /// `onCraftMatrixChanged(inventory)`: the player and workbench
    /// containers set their result to `CraftingManager.findMatchingRecipe`
    /// of their grid; the base method runs `updateCraftingResults`.
    // src: Container.java:161-163, ContainerPlayer.java:44-46, ContainerWorkbench.java:35-37
    pub fn on_craft_matrix_changed(&mut self, world: &mut World, player: &mut Entity) {
        match &mut self.kind {
            ContainerKind::Player {
                craft_matrix,
                craft_result,
                ..
            }
            | ContainerKind::Workbench {
                craft_matrix,
                craft_result,
                ..
            } => {
                let r = CraftingManager::get_instance().find_matching_recipe(craft_matrix);
                craft_result.set_inventory_slot_contents(0, r);
            }
            _ => self.update_crafting_results(world, player),
        }
    }

    /// `putStackInSlot(i, stack)`: `getSlot(i).putStack(stack)`.
    // src: Container.java:165-167
    pub fn put_stack_in_slot(
        &mut self,
        world: &mut World,
        player: &mut Entity,
        var1: usize,
        var2: Option<ItemStack>,
    ) {
        self.slot_put_stack(world, player, var1, var2);
    }

    /// `putStacksInSlots(stacks)`: slot `i` gets `stacks[i]`.
    // src: Container.java:169-174
    pub fn put_stacks_in_slots(
        &mut self,
        world: &mut World,
        player: &mut Entity,
        var1: &[Option<ItemStack>],
    ) {
        for (var2, s) in var1.iter().enumerate() {
            self.slot_put_stack(world, player, var2, *s);
        }
    }

    /// `updateProgressBar(id, value)`: empty, except `ContainerFurnace`,
    /// which writes the furnace's cook time (0), burn time (1) or current
    /// item burn time (2) (the network window-property packet).
    // src: Container.java:176-177, ContainerFurnace.java:49-62
    pub fn update_progress_bar(&mut self, world: &mut World, var1: i32, var2: i32) {
        if let ContainerKind::Furnace { furnace, .. } = self.kind
            && let Some(TileEntityKindData::Furnace(f)) =
                world.tile_entities.get_mut(&furnace).map(|t| &mut t.kind)
        {
            if var1 == 0 {
                f.furnace_cook_time = var2;
            }
            if var1 == 1 {
                f.furnace_burn_time = var2;
            }
            if var1 == 2 {
                f.current_item_burn_time = var2;
            }
        }
    }

    /// `getNextTransactionID(inventory)`: the transaction counter, pre-incremented.
    // src: Container.java:179-182
    pub fn get_next_transaction_id(&mut self) -> i16 {
        self.transaction_id = self.transaction_id.wrapping_add(1);
        self.transaction_id
    }

    /// `isUsableByPlayer(player)`: the player's own container always; the
    /// workbench while the block is still a workbench and the player within
    /// 8 blocks of its centre; the chest, furnace and dispenser through
    /// their inventory's `canInteractWith`.
    // src: Container.java:190, ContainerPlayer.java:61-63, ContainerWorkbench.java:52-58,
    // ContainerChest.java:27-29, ContainerFurnace.java:64-66, ContainerDispenser.java:23-25
    pub fn is_usable_by_player(&self, world: &mut World, player: &Entity) -> bool {
        match &self.kind {
            ContainerKind::Player { .. } => true,
            ContainerKind::Workbench { pos: (x, y, z), .. } => {
                if world.get_block_id(*x, *y, *z) != crate::crafting::ids::WORKBENCH {
                    false
                } else {
                    player
                        .base
                        .get_distance_sq(*x as f64 + 0.5, *y as f64 + 0.5, *z as f64 + 0.5)
                        <= 64.0
                }
            }
            ContainerKind::Chest {
                lower_chest_inventory,
                ..
            } => self.inv_can_interact_with(world, player, lower_chest_inventory),
            ContainerKind::Furnace { furnace, .. } => {
                tile_entity::can_interact_with(world, *furnace, &player.base)
            }
            ContainerKind::Dispenser {
                tile_entity_dispenser,
            } => tile_entity::can_interact_with(world, *tile_entity_dispenser, &player.base),
        }
    }

    /// `mergeItemStack(stack, from, to, backwards)`: first tops up the slots
    /// `from..to` (in reverse with `backwards`) holding the same item (and
    /// damage, with subtypes) up to the item's limit, if the stack is
    /// stackable; then puts what is left, as a copy, into the first empty
    /// slot. `var1` is the source slot's object: its size drops in place.
    /// The targets change in place (no event, no `isItemValid` check),
    /// except the empty slot, which gets `putStack`.
    // src: Container.java:192-250
    pub fn merge_item_stack(
        &mut self,
        world: &mut World,
        player: &mut Entity,
        var1: &mut ItemStack,
        var2: i32,
        var3: i32,
        var4: bool,
    ) {
        let mut var5 = if var4 { var3 - 1 } else { var2 };
        if var1.is_stackable() {
            while var1.stack_size > 0 && (!var4 && var5 < var3 || var4 && var5 >= var2) {
                let var6 = var5 as usize;
                let var7 = self.slot_get_stack(world, player, var6);
                if let Some(var7) = var7
                    && var7.item_id == var1.item_id
                    && (!var1.get_has_subtypes()
                        || var1.get_item_damage() == var7.get_item_damage())
                {
                    let var8 = var7.stack_size.wrapping_add(var1.stack_size);
                    if var8 <= var1.get_max_stack_size() {
                        var1.stack_size = 0;
                        self.slot_stack_mut(world, player, var6, |s| {
                            if let Some(s) = s.as_mut() {
                                s.stack_size = var8;
                            }
                        });
                        self.slot_on_slot_changed(world, player, var6);
                    } else if var7.stack_size < var1.get_max_stack_size() {
                        var1.stack_size = var1
                            .stack_size
                            .wrapping_sub(var1.get_max_stack_size() - var7.stack_size);
                        let max = var1.get_max_stack_size();
                        self.slot_stack_mut(world, player, var6, |s| {
                            if let Some(s) = s.as_mut() {
                                s.stack_size = max;
                            }
                        });
                        self.slot_on_slot_changed(world, player, var6);
                    }
                }
                if var4 {
                    var5 -= 1;
                } else {
                    var5 += 1;
                }
            }
        }
        if var1.stack_size > 0 {
            var5 = if var4 { var3 - 1 } else { var2 };
            while !var4 && var5 < var3 || var4 && var5 >= var2 {
                let var6 = var5 as usize;
                if self.slot_get_stack(world, player, var6).is_none() {
                    self.slot_put_stack(world, player, var6, Some(var1.copy()));
                    self.slot_on_slot_changed(world, player, var6);
                    var1.stack_size = 0;
                    break;
                }
                if var4 {
                    var5 -= 1;
                } else {
                    var5 += 1;
                }
            }
        }
    }

    /// The shared body of the `getStackInSlot` overrides of the player,
    /// workbench and furnace containers (ContainerPlayer.java:65-95,
    /// ContainerWorkbench.java:60-90, ContainerFurnace.java:68-98): `range`
    /// picks `mergeItemStack`'s arguments from the slot number. The slot's
    /// object is moved by `mergeItemStack`; emptied, the slot gets
    /// `putStack(null)`, else `onSlotChanged`; nothing moved returns null,
    /// otherwise `onPickupFromSlot` with the slot's object and the copy from
    /// before is returned.
    pub(crate) fn transfer_and_pick_up(
        &mut self,
        world: &mut World,
        player: &mut Entity,
        var1: usize,
        range: (i32, i32, bool),
    ) -> Option<ItemStack> {
        let _ = &self.slots[var1];
        let mut var2 = None;
        if self.slot_get_has_stack(world, player, var1) {
            let mut var4 = self
                .slot_get_stack(world, player, var1)
                .expect("getHasStack");
            var2 = Some(var4.copy());
            self.merge_item_stack(world, player, &mut var4, range.0, range.1, range.2);
            // var4 is the slot's object: its new size is the slot's.
            self.slot_stack_mut(world, player, var1, |s| *s = Some(var4));
            if var4.stack_size == 0 {
                self.slot_put_stack(world, player, var1, None);
            } else {
                self.slot_on_slot_changed(world, player, var1);
            }
            if var2.is_some_and(|v| var4.stack_size == v.stack_size) {
                return None;
            }
            // vanilla bug: when the inventory takes only part of a crafting output,
            // the rest stays in the output slot until SlotCrafting.onPickupFromSlot
            // consumes the grid, whose onCraftMatrixChanged then overwrites the output
            // with a fresh result: the untaken part is lost.
            self.slot_on_pickup_from_slot(world, player, var1, Some(var4));
        }
        var2
    }
}
