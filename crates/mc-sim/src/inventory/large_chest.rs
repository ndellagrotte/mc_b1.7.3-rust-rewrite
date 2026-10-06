//! `InventoryLargeChest` (InventoryLargeChest.java): two inventories (the
//! halves of a double chest) seen as one, the upper half's slots first.
//!
//! The halves are [`InventoryRef`]s; every method's routing (an index at or
//! past the upper half's size goes to the lower half, shifted down) is in
//! the container's `IInventory` dispatch
//! ([`crate::inventory::container::Container`]), which can reach the tile
//! entities behind them. [`chest_inventory`] builds the inventory
//! `BlockChest.blockActivated` opens.

use super::InventoryRef;
use crate::world::World;

/// `InventoryLargeChest`.
#[derive(Clone, Debug, PartialEq)]
pub struct InventoryLargeChest {
    /// `name` (private): `"Large chest"` from `BlockChest`.
    pub name: String,
    /// `upperChest` (private).
    pub upper_chest: InventoryRef,
    /// `lowerChest` (private).
    pub lower_chest: InventoryRef,
}

impl InventoryLargeChest {
    /// `new InventoryLargeChest(name, upper, lower)`.
    // src: InventoryLargeChest.java:8-12
    pub fn new(var1: &str, var2: InventoryRef, var3: InventoryRef) -> InventoryLargeChest {
        InventoryLargeChest {
            name: var1.to_string(),
            upper_chest: var2,
            lower_chest: var3,
        }
    }

    /// `getInvName()`.
    // src: InventoryLargeChest.java:18-20
    pub fn get_inv_name(&self) -> &str {
        &self.name
    }
}

/// The `IInventory` `BlockChest.blockActivated(world, x, y, z, player)`
/// passes to `displayGUIChest` once it has decided to open (the
/// normal-cube checks above it are the caller's): the chest's own tile
/// entity, wrapped in an `InventoryLargeChest` with each neighbouring chest
/// in Java's order (−x as the upper half, +x as the lower, −z upper, +z
/// lower). `None` where Java's tile entity would be null (no chest tile
/// entity at a position looked up).
// src: BlockChest.java:186, :198-212
pub fn chest_inventory(world: &mut World, x: i32, y: i32, z: i32) -> Option<InventoryRef> {
    let chest = crate::block::ids::CHEST;
    let te = |w: &mut World, x, y, z| {
        w.get_block_tile_entity(x, y, z)
            .map(InventoryRef::TileEntity)
    };
    let mut var6 = te(world, x, y, z)?;
    let large =
        |a, b| InventoryRef::LargeChest(Box::new(InventoryLargeChest::new("Large chest", a, b)));
    if world.get_block_id(x - 1, y, z) == chest {
        var6 = large(te(world, x - 1, y, z)?, var6);
    }
    if world.get_block_id(x + 1, y, z) == chest {
        var6 = large(var6, te(world, x + 1, y, z)?);
    }
    if world.get_block_id(x, y, z - 1) == chest {
        var6 = large(te(world, x, y, z - 1)?, var6);
    }
    if world.get_block_id(x, y, z + 1) == chest {
        var6 = large(var6, te(world, x, y, z + 1)?);
    }
    Some(var6)
}
