//! `ContainerDispenser` (ContainerDispenser.java): a dispenser's window.
//! No `getStackInSlot` override: a shift-click moves nothing.
//!
//! Slots: 0–8 the dispenser, 9–35 the main inventory (`InventoryPlayer`
//! slots 9–35), 36–44 the hotbar (0–8).

use super::{Container, ContainerKind};
use crate::inventory::InventoryRef;
use crate::inventory::slot::Slot;
use crate::world::tile_entity::TileEntityRef;

/// `new ContainerDispenser(playerInventory, dispenser)`.
// src: ContainerDispenser.java:3-21
pub fn new_container_dispenser(var2: TileEntityRef) -> Container {
    let mut c = Container::empty(ContainerKind::Dispenser {
        tile_entity_dispenser: var2,
    });
    for var3 in 0..3 {
        for var4 in 0..3 {
            c.add_slot(Slot::new(
                InventoryRef::TileEntity(var2),
                var4 + var3 * 3,
                62 + var4 * 18,
                17 + var3 * 18,
            ));
        }
    }
    for var3 in 0..3 {
        for var4 in 0..9 {
            c.add_slot(Slot::new(
                InventoryRef::Player,
                var4 + var3 * 9 + 9,
                8 + var4 * 18,
                84 + var3 * 18,
            ));
        }
    }
    for var3 in 0..9 {
        c.add_slot(Slot::new(InventoryRef::Player, var3, 8 + var3 * 18, 142));
    }
    c
}
