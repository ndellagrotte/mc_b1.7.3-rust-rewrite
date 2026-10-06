//! `TileEntityFurnace` (TileEntityFurnace.java) and `FurnaceRecipes`
//! (FurnaceRecipes.java, the smelting table, as data).
//!
//! Slots: 0 the input, 1 the fuel, 2 the output. `updateEntity` burns fuel
//! and cooks for 200 ticks per item; when the furnace starts or stops
//! burning it swaps the block between idle (61) and lit (62) through
//! [`crate::block::furnace::update_furnace_block_state`].

use std::collections::BTreeMap;

use mc_core::java_hash::JavaHashMap;
use mc_nbt::Tag;

use super::{compound, get_byte, get_short, get_tag_list, write_items};
use crate::block::{self, Material};
use crate::item::{ItemStack, JavaException, item};
use crate::world::World;
use crate::world::tile_entity::{TileEntityKindData, TileEntityRef};

// Item and block ids the furnace code names (`shiftedIndex` = 256 + the
// constructor's index; `blockID` the constructor's id).
// src: Item.java:283 (ingotIron 9), :284 (ingotGold 10), :282 (diamond 8),
// :337-338 (porkRaw 63, porkCooked 64), :367-368 (fishRaw 93, fishCooked 94),
// :355 (clay 81), :354 (brick 80), :369 (dyePowder 95), :281 (coal 7),
// :298 (stick 24), :345 (bucketLava 71)
const INGOT_IRON: i32 = 256 + 9;
const INGOT_GOLD: i32 = 256 + 10;
const DIAMOND: i32 = 256 + 8;
const PORK_RAW: i32 = 256 + 63;
const PORK_COOKED: i32 = 256 + 64;
const FISH_RAW: i32 = 256 + 93;
const FISH_COOKED: i32 = 256 + 94;
const CLAY: i32 = 256 + 81;
const BRICK: i32 = 256 + 80;
const DYE_POWDER: i32 = 256 + 95;
const COAL: i32 = 256 + 7;
const STICK: i32 = 256 + 24;
const BUCKET_LAVA: i32 = 256 + 71;
// src: Block.java:607 (oreIron 15), :606 (oreGold 14), :648 (oreDiamond 56),
// :604 (sand 12), :612 (glass 20), :596 (cobblestone 4), :593 (stone 1),
// :673 (cactus 81), :609 (wood 17), :598 (sapling 6)
const ORE_IRON: i32 = 15;
const ORE_GOLD: i32 = 14;
const ORE_DIAMOND: i32 = 56;
const SAND: i32 = 12;
const GLASS: i32 = 20;
const COBBLESTONE: i32 = 4;
const STONE: i32 = 1;
const CACTUS: i32 = 81;
const WOOD: i32 = 17;
const SAPLING: i32 = 6;

/// `FurnaceRecipes`' `addSmelting(input, result)` calls in constructor order:
/// `(input id, result)`. `new ItemStack(Item|Block)` is count 1, damage 0.
// src: FurnaceRecipes.java:14-25
pub const SMELTING: [(i32, ItemStack); 10] = [
    (ORE_IRON, ItemStack::new(INGOT_IRON, 1, 0)),
    (ORE_GOLD, ItemStack::new(INGOT_GOLD, 1, 0)),
    (ORE_DIAMOND, ItemStack::new(DIAMOND, 1, 0)),
    (SAND, ItemStack::new(GLASS, 1, 0)),
    (PORK_RAW, ItemStack::new(PORK_COOKED, 1, 0)),
    (FISH_RAW, ItemStack::new(FISH_COOKED, 1, 0)),
    (COBBLESTONE, ItemStack::new(STONE, 1, 0)),
    (CLAY, ItemStack::new(BRICK, 1, 0)),
    (CACTUS, ItemStack::new(DYE_POWDER, 1, 2)),
    (WOOD, ItemStack::new(COAL, 1, 1)),
];

/// `FurnaceRecipes.smelting().getSmeltingResult(id)`: the result template
/// (callers copy it), or `None`.
// src: FurnaceRecipes.java:31-33
pub fn get_smelting_result(id: i32) -> Option<ItemStack> {
    SMELTING.iter().find(|(i, _)| *i == id).map(|(_, s)| *s)
}

/// `getSmeltingList()`: the `HashMap` itself, in Java iteration order
/// (StatList.java:73 iterates its values for the smelting statistics,
/// WP-2.7).
// src: FurnaceRecipes.java:8, :27-29, :35-37
pub fn smelting_list() -> JavaHashMap<i32, ItemStack> {
    let mut m = JavaHashMap::new();
    for (id, s) in SMELTING {
        m.insert(id, s);
    }
    m
}

/// `getItemBurnTime(stack)` (private): fuel ticks of a stack, 0 for none.
/// Wooden blocks 300, stick 100, coal 1600, lava bucket 20000, sapling 100.
/// A block id with no block would throw in Java (unreachable: no such stack
/// exists); it counts as not wood here.
// src: TileEntityFurnace.java:185-202
pub fn get_item_burn_time(stack: Option<&ItemStack>) -> i32 {
    let Some(s) = stack else { return 0 };
    let var2 = s.item_id;
    if var2 < 256 && block::block(var2).is_some_and(|b| b.block_material == Material::Wood) {
        300
    } else if var2 == STICK {
        100
    } else if var2 == COAL {
        1600
    } else if var2 == BUCKET_LAVA {
        20000
    } else if var2 == SAPLING {
        100
    } else {
        0
    }
}

/// The fields `TileEntityFurnace` declares.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TileEntityFurnace {
    /// `furnaceItemStacks` (private): input, fuel, output.
    // src: TileEntityFurnace.java:4
    pub furnace_item_stacks: [Option<ItemStack>; 3],
    /// `furnaceBurnTime`: fuel ticks left.
    // src: TileEntityFurnace.java:5
    pub furnace_burn_time: i32,
    /// `currentItemBurnTime`: the burn time of the last fuel item (the GUI's
    /// flame scale).
    // src: TileEntityFurnace.java:6
    pub current_item_burn_time: i32,
    /// `furnaceCookTime`: ticks the current item has cooked, 0-199.
    // src: TileEntityFurnace.java:7
    pub furnace_cook_time: i32,
}

impl TileEntityFurnace {
    /// `readFromNBT(compound)` after `super.readFromNBT`: a new 3-slot array
    /// from `Items` (`Slot` read as a signed byte, so slots 128-255 are
    /// ignored), `BurnTime`, `CookTime` (shorts), and `currentItemBurnTime`
    /// recomputed from the fuel slot.
    // src: TileEntityFurnace.java:49-65
    pub fn read_from_nbt(&mut self, c: &BTreeMap<String, Tag>) -> Result<(), JavaException> {
        let var2 = get_tag_list(c, "Items")?;
        self.furnace_item_stacks = [None; 3];
        for t in var2 {
            let var4 = compound(t)?;
            let var5 = get_byte(var4, "Slot")?;
            if var5 >= 0 && (var5 as usize) < self.furnace_item_stacks.len() {
                self.furnace_item_stacks[var5 as usize] = Some(ItemStack::from_nbt(t)?);
            }
        }
        self.furnace_burn_time = get_short(c, "BurnTime")? as i32;
        self.furnace_cook_time = get_short(c, "CookTime")? as i32;
        self.current_item_burn_time = get_item_burn_time(self.furnace_item_stacks[1].as_ref());
        Ok(())
    }

    /// `writeToNBT(compound)` after `super.writeToNBT`: `BurnTime`,
    /// `CookTime` (`(short)` casts), `Items`.
    // src: TileEntityFurnace.java:67-83
    pub fn write_to_nbt(&self, c: &mut BTreeMap<String, Tag>) {
        c.insert(
            "BurnTime".to_string(),
            Tag::Short(self.furnace_burn_time as i16),
        );
        c.insert(
            "CookTime".to_string(),
            Tag::Short(self.furnace_cook_time as i16),
        );
        c.insert("Items".to_string(), write_items(&self.furnace_item_stacks));
    }

    /// `getCookProgressScaled(scale)` (the GUI arrow).
    // src: TileEntityFurnace.java:89-91
    pub fn get_cook_progress_scaled(&self, var1: i32) -> i32 {
        self.furnace_cook_time.wrapping_mul(var1) / 200
    }

    /// `getBurnTimeRemainingScaled(scale)` (the GUI flame): sets
    /// `currentItemBurnTime` to 200 when it is 0 (a side effect of a getter,
    /// vanilla, transcribed).
    // src: TileEntityFurnace.java:93-99
    pub fn get_burn_time_remaining_scaled(&mut self, var1: i32) -> i32 {
        if self.current_item_burn_time == 0 {
            self.current_item_burn_time = 200;
        }
        self.furnace_burn_time.wrapping_mul(var1) / self.current_item_burn_time
    }

    /// `isBurning()`.
    // src: TileEntityFurnace.java:101-103
    pub fn is_burning(&self) -> bool {
        self.furnace_burn_time > 0
    }

    /// `canSmelt()` (private): an input with a recipe whose result fits the
    /// output slot. `getItem().shiftedIndex` of an input with no item would
    /// throw in Java (unreachable); it has no recipe here.
    // src: TileEntityFurnace.java:149-166
    pub fn can_smelt(&self) -> bool {
        let Some(input) = self.furnace_item_stacks[0] else {
            return false;
        };
        let Some(var1) = input
            .get_item()
            .and_then(|i| get_smelting_result(i.shifted_index))
        else {
            return false;
        };
        match self.furnace_item_stacks[2] {
            None => true,
            Some(out) if !out.is_item_equal(&var1) => false,
            Some(out)
                if out.stack_size < super::INVENTORY_STACK_LIMIT
                    && out.stack_size < out.get_max_stack_size() =>
            {
                true
            }
            Some(out) => out.stack_size < var1.get_max_stack_size(),
        }
    }

    /// `smeltItem()`: one input item into the output slot (vanilla compares
    /// only the item id when adding to an existing output; `canSmelt` has
    /// checked the damage too).
    // src: TileEntityFurnace.java:168-183
    pub fn smelt_item(&mut self) {
        if self.can_smelt() {
            let input = self.furnace_item_stacks[0].expect("canSmelt: input present");
            let var1 = item(input.item_id)
                .and_then(|i| get_smelting_result(i.shifted_index))
                .expect("canSmelt: recipe present");
            match self.furnace_item_stacks[2].as_mut() {
                None => self.furnace_item_stacks[2] = Some(var1),
                Some(out) if out.item_id == var1.item_id => out.stack_size += 1,
                Some(_) => {}
            }
            if let Some(i) = self.furnace_item_stacks[0].as_mut() {
                i.stack_size -= 1;
                if i.stack_size <= 0 {
                    self.furnace_item_stacks[0] = None;
                }
            }
        }
    }
}

fn data(world: &mut World, r: TileEntityRef) -> Option<&mut TileEntityFurnace> {
    match &mut world.tile_entities.get_mut(&r)?.kind {
        TileEntityKindData::Furnace(d) => Some(d),
        _ => None,
    }
}

/// `updateEntity()`: burn down the fuel, take a new fuel item when the
/// input can smelt, cook, swap the block between idle and lit when the
/// burning state changed, and report a change with `onInventoryChanged`.
// src: TileEntityFurnace.java:105-147
pub fn update_entity(world: &mut World, r: TileEntityRef) {
    let multiplayer = world.multiplayer_world;
    let Some(d) = data(world, r) else { return };
    let var1 = d.furnace_burn_time > 0;
    let mut var2 = false;
    if d.furnace_burn_time > 0 {
        d.furnace_burn_time -= 1;
    }
    if !multiplayer {
        if d.furnace_burn_time == 0 && d.can_smelt() {
            // src: TileEntityFurnace.java:115 (both fields from one call)
            d.furnace_burn_time = get_item_burn_time(d.furnace_item_stacks[1].as_ref());
            d.current_item_burn_time = d.furnace_burn_time;
            if d.furnace_burn_time > 0 {
                var2 = true;
                if let Some(fuel) = d.furnace_item_stacks[1].as_mut() {
                    fuel.stack_size -= 1;
                    if fuel.stack_size == 0 {
                        d.furnace_item_stacks[1] = None;
                    }
                }
            }
        }
        if d.is_burning() && d.can_smelt() {
            d.furnace_cook_time += 1;
            if d.furnace_cook_time == 200 {
                d.furnace_cook_time = 0;
                d.smelt_item();
                var2 = true;
            }
        } else {
            d.furnace_cook_time = 0;
        }
        let burning = d.furnace_burn_time > 0;
        if var1 != burning {
            var2 = true;
            let (x, y, z) = {
                let t = &world.tile_entities[&r];
                (t.x_coord, t.y_coord, t.z_coord)
            };
            crate::block::furnace::update_furnace_block_state(burning, world, x, y, z);
        }
    }
    if var2 {
        super::on_inventory_changed(world, r);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smelting_table() {
        assert_eq!(get_smelting_result(15), Some(ItemStack::new(265, 1, 0)));
        assert_eq!(get_smelting_result(14), Some(ItemStack::new(266, 1, 0)));
        assert_eq!(get_smelting_result(56), Some(ItemStack::new(264, 1, 0)));
        assert_eq!(get_smelting_result(12), Some(ItemStack::new(20, 1, 0)));
        assert_eq!(get_smelting_result(319), Some(ItemStack::new(320, 1, 0)));
        assert_eq!(get_smelting_result(349), Some(ItemStack::new(350, 1, 0)));
        assert_eq!(get_smelting_result(4), Some(ItemStack::new(1, 1, 0)));
        assert_eq!(get_smelting_result(337), Some(ItemStack::new(336, 1, 0)));
        assert_eq!(get_smelting_result(81), Some(ItemStack::new(351, 1, 2)));
        assert_eq!(get_smelting_result(17), Some(ItemStack::new(263, 1, 1)));
        assert_eq!(get_smelting_result(16), None); // coal ore
        assert_eq!(get_smelting_result(263), None);
        // The constants name the right table rows.
        for (input, out) in SMELTING {
            if input >= 256 {
                assert!(item(input).is_some(), "input item {input}");
            } else {
                assert!(block::block(input).is_some(), "input block {input}");
            }
            assert!(item(out.item_id).is_some(), "result {}", out.item_id);
        }
        // HashMap<Integer, ItemStack> order: buckets of 16 by id & 15, then
        // insertion order within a bucket.
        let order: Vec<i32> = smelting_list().keys().copied().collect();
        assert_eq!(order, vec![337, 81, 17, 4, 56, 12, 349, 14, 15, 319]);
    }

    #[test]
    fn burn_times() {
        let s = |id| Some(ItemStack::new(id, 1, 0));
        assert_eq!(get_item_burn_time(None), 0);
        assert_eq!(get_item_burn_time(s(5).as_ref()), 300); // planks
        assert_eq!(get_item_burn_time(s(17).as_ref()), 300); // log
        assert_eq!(get_item_burn_time(s(54).as_ref()), 300); // chest (wood)
        assert_eq!(get_item_burn_time(s(58).as_ref()), 300); // workbench
        assert_eq!(get_item_burn_time(s(280).as_ref()), 100); // stick
        assert_eq!(get_item_burn_time(s(263).as_ref()), 1600); // coal (and charcoal)
        assert_eq!(
            get_item_burn_time(Some(ItemStack::new(263, 1, 1)).as_ref()),
            1600
        );
        assert_eq!(get_item_burn_time(s(327).as_ref()), 20000); // lava bucket
        assert_eq!(get_item_burn_time(s(6).as_ref()), 100); // sapling
        assert_eq!(get_item_burn_time(s(1).as_ref()), 0); // stone
        assert_eq!(get_item_burn_time(s(264).as_ref()), 0); // diamond
    }

    #[test]
    fn can_smelt_and_smelt_item() {
        let mut f = TileEntityFurnace::default();
        assert!(!f.can_smelt());
        f.furnace_item_stacks[0] = Some(ItemStack::new(4, 2, 0)); // cobblestone
        assert!(f.can_smelt());
        f.smelt_item();
        assert_eq!(f.furnace_item_stacks[0], Some(ItemStack::new(4, 1, 0)));
        assert_eq!(f.furnace_item_stacks[2], Some(ItemStack::new(1, 1, 0)));
        f.smelt_item();
        assert_eq!(f.furnace_item_stacks[0], None);
        assert_eq!(f.furnace_item_stacks[2], Some(ItemStack::new(1, 2, 0)));
        // A different output blocks smelting.
        f.furnace_item_stacks[0] = Some(ItemStack::new(12, 1, 0)); // sand -> glass
        assert!(!f.can_smelt());
        // A full output (64) blocks it too.
        f.furnace_item_stacks[2] = Some(ItemStack::new(20, 64, 0));
        assert!(!f.can_smelt());
        f.furnace_item_stacks[2] = Some(ItemStack::new(20, 63, 0));
        assert!(f.can_smelt());
        // Same id, other damage: not isItemEqual.
        f.furnace_item_stacks[0] = Some(ItemStack::new(17, 1, 0));
        f.furnace_item_stacks[2] = Some(ItemStack::new(263, 1, 0));
        assert!(!f.can_smelt());
        // Scaled getters.
        f.furnace_cook_time = 100;
        assert_eq!(f.get_cook_progress_scaled(24), 12);
        f.furnace_burn_time = 50;
        assert_eq!(f.get_burn_time_remaining_scaled(12), 3);
        assert_eq!(f.current_item_burn_time, 200);
    }
}
