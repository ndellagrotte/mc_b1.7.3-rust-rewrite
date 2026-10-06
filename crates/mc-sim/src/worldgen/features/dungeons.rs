//! `WorldGenDungeons` (WorldGenDungeons.java): the mossy-cobblestone room
//! with up to two loot chests and a mob spawner.
//!
//! The loot goes through the chest's tile entity
//! ([`crate::tile_entity::set_inventory_slot_contents`]) as
//! `TileEntityChest.setInventorySlotContents` does, and the spawner's mob
//! through [`TileEntityMobSpawner::set_mob_id`]; both tile entities exist
//! because `setBlockWithNotify` ran `BlockContainer.onBlockAdded`
//! (BlockContainer.java:13-16).
//!
//! [`TileEntityMobSpawner::set_mob_id`]: crate::tile_entity::mob_spawner::TileEntityMobSpawner::set_mob_id

use super::next_int;
use crate::block::dispatch::RandomSource;
use crate::block::ids;
use crate::item::ItemStack;
use crate::tile_entity;
use crate::world::World;
use crate::world::tile_entity::TileEntityKindData;

/// `Item.<field>.shiftedIndex` (`256 + id`, Item.java:124) of the loot items.
mod item_ids {
    /// `Item.ingotIron` (Item.java:283).
    pub const INGOT_IRON: i32 = 256 + 9;
    /// `Item.silk` (Item.java:305).
    pub const SILK: i32 = crate::item::ids::SILK;
    /// `Item.gunpowder` (Item.java:307).
    pub const GUNPOWDER: i32 = 256 + 33;
    /// `Item.wheat` (Item.java:314).
    pub const WHEAT: i32 = crate::item::ids::WHEAT;
    /// `Item.bread` (Item.java:315).
    pub const BREAD: i32 = 256 + 41;
    /// `Item.appleGold` (Item.java:340).
    pub const APPLE_GOLD: i32 = 256 + 66;
    /// `Item.bucketEmpty` (Item.java:343).
    pub const BUCKET_EMPTY: i32 = 256 + 69;
    /// `Item.saddle` (Item.java:347).
    pub const SADDLE: i32 = 256 + 73;
    /// `Item.redstone` (Item.java:349).
    pub const REDSTONE: i32 = crate::item::ids::REDSTONE;
    /// `Item.dyePowder` (Item.java:369).
    pub const DYE_POWDER: i32 = crate::item::ids::DYE_POWDER;
    /// `Item.record13` (Item.java:378); `Item.recordCat` is the next index
    /// (Item.java:379).
    pub const RECORD_13: i32 = 256 + 2000;
}

/// `WorldGenDungeons` (no fields).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorldGenDungeons;

impl WorldGenDungeons {
    /// `generate(world, rand, x, y, z)`: a room of inner half-size
    /// `nextInt(2) + 2` in x and z and height 3 at `(x, y, z)`, built only
    /// when the floor and ceiling layers are solid and the walls have 1 to 5
    /// openings (air two high at floor level).
    // src: WorldGenDungeons.java:6-105
    #[allow(clippy::if_same_then_else)] // the Java's `if / else if` with equal bodies, kept
    pub fn generate(&self, world: &mut World, rand: RandomSource, x: i32, y: i32, z: i32) -> bool {
        let var6: i32 = 3;
        let var7 = next_int(world, rand, 2) + 2;
        let var8 = next_int(world, rand, 2) + 2;
        let mut var9 = 0;

        // src: WorldGenDungeons.java:15-32 (the survey)
        for var10 in (x - var7 - 1)..=(x + var7 + 1) {
            for var11 in (y - 1)..=(y + var6 + 1) {
                for var12 in (z - var8 - 1)..=(z + var8 + 1) {
                    let var13 = world.get_block_material(var10, var11, var12);
                    if var11 == y - 1 && !var13.is_solid() {
                        return false;
                    }
                    if var11 == y + var6 + 1 && !var13.is_solid() {
                        return false;
                    }
                    if (var10 == x - var7 - 1
                        || var10 == x + var7 + 1
                        || var12 == z - var8 - 1
                        || var12 == z + var8 + 1)
                        && var11 == y
                        && world.is_air_block(var10, var11, var12)
                        && world.is_air_block(var10, var11 + 1, var12)
                    {
                        var9 += 1;
                    }
                }
            }
        }

        if !(1..=5).contains(&var9) {
            return false;
        }

        // src: WorldGenDungeons.java:35-51 (the room, top layer first)
        for var10 in (x - var7 - 1)..=(x + var7 + 1) {
            let mut var11 = y + var6;
            while var11 >= y - 1 {
                for var12 in (z - var8 - 1)..=(z + var8 + 1) {
                    if var10 != x - var7 - 1
                        && var11 != y - 1
                        && var12 != z - var8 - 1
                        && var10 != x + var7 + 1
                        && var11 != y + var6 + 1
                        && var12 != z + var8 + 1
                    {
                        world.set_block_with_notify(var10, var11, var12, 0);
                    } else if var11 >= 0
                        && !world.get_block_material(var10, var11 - 1, var12).is_solid()
                    {
                        world.set_block_with_notify(var10, var11, var12, 0);
                    } else if world.get_block_material(var10, var11, var12).is_solid() {
                        if var11 == y - 1 && next_int(world, rand, 4) != 0 {
                            world.set_block_with_notify(
                                var10,
                                var11,
                                var12,
                                ids::COBBLESTONE_MOSSY,
                            );
                        } else {
                            world.set_block_with_notify(var10, var11, var12, ids::COBBLESTONE);
                        }
                    }
                }
                var11 -= 1;
            }
        }

        // src: WorldGenDungeons.java:53-96 (two chests, three tries each; a
        // placed chest ends its tries: `continue label110`)
        'label110: for _var10 in 0..2 {
            for _var11 in 0..3 {
                let var12 = x + next_int(world, rand, var7 * 2 + 1) - var7;
                let var14 = z + next_int(world, rand, var8 * 2 + 1) - var8;
                if world.is_air_block(var12, y, var14) {
                    let mut var15 = 0;
                    if world.get_block_material(var12 - 1, y, var14).is_solid() {
                        var15 += 1;
                    }
                    if world.get_block_material(var12 + 1, y, var14).is_solid() {
                        var15 += 1;
                    }
                    if world.get_block_material(var12, y, var14 - 1).is_solid() {
                        var15 += 1;
                    }
                    if world.get_block_material(var12, y, var14 + 1).is_solid() {
                        var15 += 1;
                    }
                    if var15 == 1 {
                        world.set_block_with_notify(var12, y, var14, ids::CHEST);
                        // src: WorldGenDungeons.java:78 (the cast happens once,
                        // before any loot is drawn: a null tile entity passes it,
                        // anything but a TileEntityChest throws
                        // ClassCastException). Unreachable either way: the survey
                        // needs a solid floor and ceiling, so y is in 1..=123 and
                        // setBlockWithNotify(chest) always creates the chest.
                        let var16 = world.get_block_tile_entity(var12, y, var14);
                        if let Some(r) = var16 {
                            let t = world.tile_entities.get(&r).expect("tile entity in store");
                            assert!(
                                matches!(t.kind, TileEntityKindData::Chest(_)),
                                "WorldGenDungeons: ClassCastException (not a TileEntityChest)"
                            );
                        }
                        for _var17 in 0..8 {
                            let var18 = pick_check_loot_item(world, rand);
                            if let Some(stack) = var18 {
                                // src: WorldGenDungeons.java:88 (a null var16 is a
                                // NullPointerException here, only for a non-null item)
                                let var16 = var16.expect(
                                    "WorldGenDungeons: NullPointerException (no TileEntityChest)",
                                );
                                let size = tile_entity::get_size_inventory(
                                    &world
                                        .tile_entities
                                        .get(&var16)
                                        .expect("tile entity in store")
                                        .kind,
                                )
                                .expect("a chest is an IInventory");
                                // vanilla bug: the slot is drawn without checking
                                // it is free, so a later item can replace an
                                // earlier one.
                                let slot = next_int(world, rand, size);
                                tile_entity::set_inventory_slot_contents(
                                    world,
                                    var16,
                                    slot,
                                    Some(stack),
                                );
                            }
                        }
                        continue 'label110;
                    }
                }
            }
        }

        // src: WorldGenDungeons.java:98-101
        world.set_block_with_notify(x, y, z, ids::MOB_SPAWNER);
        let var19 = world
            .get_block_tile_entity(x, y, z)
            .expect("WorldGenDungeons: the mob spawner has no tile entity");
        let mob = pick_mob_spawner(world, rand);
        match world.tile_entities.get_mut(&var19).map(|t| &mut t.kind) {
            Some(TileEntityKindData::MobSpawner(s)) => s.set_mob_id(mob.to_string()),
            _ => panic!("WorldGenDungeons: ClassCastException (not a TileEntityMobSpawner)"),
        }
        true
    }
}

/// `pickCheckLootItem(rand)`: one of eleven outcomes, some with a further
/// chance draw (`null` = nothing).
// src: WorldGenDungeons.java:107-132
fn pick_check_loot_item(world: &mut World, rand: RandomSource) -> Option<ItemStack> {
    use item_ids as i;
    let var2 = next_int(world, rand, 11);
    if var2 == 0 {
        Some(ItemStack::new(i::SADDLE, 1, 0))
    } else if var2 == 1 {
        Some(ItemStack::new(
            i::INGOT_IRON,
            next_int(world, rand, 4) + 1,
            0,
        ))
    } else if var2 == 2 {
        Some(ItemStack::new(i::BREAD, 1, 0))
    } else if var2 == 3 {
        Some(ItemStack::new(i::WHEAT, next_int(world, rand, 4) + 1, 0))
    } else if var2 == 4 {
        Some(ItemStack::new(
            i::GUNPOWDER,
            next_int(world, rand, 4) + 1,
            0,
        ))
    } else if var2 == 5 {
        Some(ItemStack::new(i::SILK, next_int(world, rand, 4) + 1, 0))
    } else if var2 == 6 {
        Some(ItemStack::new(i::BUCKET_EMPTY, 1, 0))
    } else if var2 == 7 && next_int(world, rand, 100) == 0 {
        Some(ItemStack::new(i::APPLE_GOLD, 1, 0))
    } else if var2 == 8 && next_int(world, rand, 2) == 0 {
        Some(ItemStack::new(i::REDSTONE, next_int(world, rand, 4) + 1, 0))
    } else if var2 == 9 && next_int(world, rand, 10) == 0 {
        // `Item.itemsList[Item.record13.shiftedIndex + rand.nextInt(2)]`:
        // record 13 or cat.
        Some(ItemStack::new(
            i::RECORD_13 + next_int(world, rand, 2),
            1,
            0,
        ))
    } else if var2 == 10 {
        // `new ItemStack(Item.dyePowder, 1, 3)`: cocoa beans.
        Some(ItemStack::new(i::DYE_POWDER, 1, 3))
    } else {
        None
    }
}

/// `pickMobSpawner(rand)`: skeleton 1/4, zombie 2/4, spider 1/4.
// src: WorldGenDungeons.java:134-145
fn pick_mob_spawner(world: &mut World, rand: RandomSource) -> &'static str {
    let var2 = next_int(world, rand, 4);
    if var2 == 0 {
        "Skeleton"
    } else if var2 == 1 || var2 == 2 {
        "Zombie"
    } else if var2 == 3 {
        "Spider"
    } else {
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::item_ids;
    use crate::item::item;

    /// The loot ids name the right rows of the item table.
    #[test]
    fn loot_ids_are_item_rows() {
        for id in [
            item_ids::INGOT_IRON,
            item_ids::SILK,
            item_ids::GUNPOWDER,
            item_ids::WHEAT,
            item_ids::BREAD,
            item_ids::APPLE_GOLD,
            item_ids::BUCKET_EMPTY,
            item_ids::SADDLE,
            item_ids::REDSTONE,
            item_ids::DYE_POWDER,
            item_ids::RECORD_13,
            item_ids::RECORD_13 + 1,
        ] {
            assert_eq!(item(id).map(|d| d.shifted_index), Some(id), "item {id}");
        }
        assert_eq!(
            item(item_ids::SADDLE).and_then(|d| d.item_name),
            Some("saddle")
        );
        assert_eq!(
            item(item_ids::RECORD_13).and_then(|d| d.item_name),
            Some("record")
        );
        assert_eq!(
            item(item_ids::APPLE_GOLD).and_then(|d| d.item_name),
            Some("appleGold")
        );
    }
}
