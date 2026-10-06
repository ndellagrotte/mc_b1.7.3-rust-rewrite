//! `TileEntityDispenser` (TileEntityDispenser.java): 9 slots and the
//! `Random` that picks which stack fires. The block is
//! [`crate::block::dispenser`].
//!
//! `dispenserRandom` is `new Random()` per tile entity (clock-seeded,
//! TileEntityDispenser.java:7) and the oracle has no hook to seed it
//! (tools/oracle/census.md §2, a gap). The port seeds every dispenser's
//! with `mix(seed, -7)` of the world's seeding protocol
//! ([`DISPENSER_RANDOM_KEY`]); the draws only matter with two or more
//! stacks in the dispenser (`nextInt(1)` is 0 whatever the state).

use std::collections::BTreeMap;

use mc_core::java_random::JavaRandom;
use mc_nbt::Tag;

use super::{compound, get_byte, get_tag_list, write_items};
use crate::entity::base::mix_seed;
use crate::item::{ItemStack, JavaException};
use crate::world::World;
use crate::world::tile_entity::{TileEntityKindData, TileEntityRef};

/// `getSizeInventory()`.
// src: TileEntityDispenser.java:9-11
pub const SIZE_INVENTORY: i32 = 9;

/// The `k` of `mix(seed, k)` that seeds a dispenser tile entity's `Random`
/// in the port (module docs; -1..-5 are the oracle's singletons, -6 the
/// explosion's, [`crate::world::explosion`]).
pub const DISPENSER_RANDOM_KEY: i64 = -7;

/// The fields `TileEntityDispenser` declares.
#[derive(Clone, Debug, PartialEq)]
pub struct TileEntityDispenser {
    /// `dispenserContents` (private).
    // src: TileEntityDispenser.java:6
    pub dispenser_contents: [Option<ItemStack>; 9],
    /// `dispenserRandom` (private).
    // src: TileEntityDispenser.java:7
    pub dispenser_random: JavaRandom,
}

impl TileEntityDispenser {
    /// `new TileEntityDispenser()`, its `Random` seeded per the module docs.
    // src: TileEntityDispenser.java:6-7
    pub fn new(world: &World) -> Self {
        TileEntityDispenser {
            dispenser_contents: [None; 9],
            dispenser_random: JavaRandom::new(mix_seed(
                world.entity_rand_seed,
                DISPENSER_RANDOM_KEY,
            )),
        }
    }

    /// `readFromNBT(compound)` after `super.readFromNBT`: a new 9-slot array
    /// from `Items` (`Slot` as an unsigned byte).
    // src: TileEntityDispenser.java:69-82
    pub fn read_from_nbt(&mut self, c: &BTreeMap<String, Tag>) -> Result<(), JavaException> {
        let var2 = get_tag_list(c, "Items")?;
        self.dispenser_contents = [None; 9];
        for t in var2 {
            let var4 = compound(t)?;
            let var5 = (get_byte(var4, "Slot")? as i32) & 255;
            if var5 >= 0 && (var5 as usize) < self.dispenser_contents.len() {
                self.dispenser_contents[var5 as usize] = Some(ItemStack::from_nbt(t)?);
            }
        }
        Ok(())
    }

    /// `writeToNBT(compound)` after `super.writeToNBT`: `Items`.
    // src: TileEntityDispenser.java:84-98
    pub fn write_to_nbt(&self, c: &mut BTreeMap<String, Tag>) {
        c.insert("Items".to_string(), write_items(&self.dispenser_contents));
    }
}

/// `getRandomStackFromInventory()`: picks one non-empty slot uniformly by
/// reservoir sampling (`nextInt(n)` for the n-th non-empty slot; the
/// counter only advances on non-empty slots) and takes one item out of it
/// with `decrStackSize`, which calls `onInventoryChanged`.
// src: TileEntityDispenser.java:39-54
pub fn get_random_stack_from_inventory(world: &mut World, r: TileEntityRef) -> Option<ItemStack> {
    let var1 = {
        let TileEntityKindData::Dispenser(d) = &mut world.tile_entities.get_mut(&r)?.kind else {
            return None;
        };
        let mut var1: i32 = -1;
        let mut var2: i32 = 1;
        for var3 in 0..d.dispenser_contents.len() {
            if d.dispenser_contents[var3].is_some() {
                // `nextInt(var2++)`: evaluated only for a non-empty slot.
                let roll = d.dispenser_random.next_int_bounded(var2);
                var2 += 1;
                if roll == 0 {
                    var1 = var3 as i32;
                }
            }
        }
        var1
    };
    if var1 >= 0 {
        super::decr_stack_size(world, r, var1, 1)
    } else {
        None
    }
}
