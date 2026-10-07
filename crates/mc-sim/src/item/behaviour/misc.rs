//! The remaining item classes with behaviour (WP-2.6): `ItemSaddle`
//! (ItemSaddle.java), `ItemFood` (ItemFood.java; `ItemCookie` inherits it),
//! `ItemSoup` (ItemSoup.java), `ItemRecord` (ItemRecord.java) and `ItemDye`
//! (ItemDye.java).

use super::{ItemDef, ItemStack, RightClickResult};
use crate::block::dispatch::RandomSource;
use crate::block::{crops, jukebox, sapling};
use crate::entity::{Entity, living, pig, sheep};
use crate::item::{FoodData, block_cloth_get_block_from_dye};
use crate::world::World;
use crate::world::is_instance_of;

/// `Item.bowlEmpty.shiftedIndex`.
// src: Item.java:299 (new Item(25): 256 + 25)
const BOWL_EMPTY: i32 = 281;
/// `Block.jukebox.blockID`.
// src: Block.java:676
const JUKEBOX: i32 = 84;
/// `Block.sapling.blockID`.
// src: Block.java:598
const SAPLING: i32 = 6;
/// `Block.crops.blockID`.
// src: Block.java:651
const CROPS: i32 = 59;
/// `Block.grass.blockID`.
// src: Block.java:594
const GRASS: i32 = 2;
/// `Block.tallGrass.blockID`.
// src: Block.java:623
const TALL_GRASS: i32 = 31;
/// `Block.plantYellow.blockID`.
// src: Block.java:629
const PLANT_YELLOW: i32 = 37;
/// `Block.plantRed.blockID`.
// src: Block.java:630
const PLANT_RED: i32 = 38;

/// `ItemSaddle.saddleEntity`: an unsaddled pig gets the saddle (one off the
/// stack).
// src: ItemSaddle.java:9-18
pub fn saddle_saddle_entity(var1: &mut ItemStack, var2: &mut Entity, world: &mut World) {
    let _ = world;
    if is_instance_of(&var2.kind, "EntityPig") && !pig::get_saddled(var2) {
        pig::set_saddled(var2, true);
        var1.stack_size -= 1;
    }
}

/// `ItemSaddle.hitEntity`: hitting a pig with the saddle saddles it
/// (`saddleEntity` on the target); true.
// src: ItemSaddle.java:20-23
pub fn saddle_hit_entity(var1: &mut ItemStack, var2: &mut Entity, world: &mut World) -> bool {
    saddle_saddle_entity(var1, var2, world);
    true
}

/// `ItemFood.onItemRightClick`: one off the stack and `heal(healAmount)`
/// on the player (nothing at health 0; capped at 20).
// src: ItemFood.java:14-18
pub fn food_on_item_right_click(
    f: FoodData,
    var1: &mut ItemStack,
    var3: &mut Entity,
) -> RightClickResult {
    var1.stack_size -= 1;
    living::heal(var3, f.heal_amount);
    RightClickResult::Same
}

/// `ItemSoup.onItemRightClick`: `ItemFood`'s, then an empty bowl replaces
/// the stack.
// src: ItemSoup.java:8-11
pub fn soup_on_item_right_click(
    f: FoodData,
    var1: &mut ItemStack,
    var3: &mut Entity,
) -> RightClickResult {
    food_on_item_right_click(f, var1, var3);
    RightClickResult::Replaced(ItemStack::new(BOWL_EMPTY, 1, 0))
}

/// `ItemRecord.onItemUse`: on an empty jukebox (metadata 0), the record goes
/// in (`BlockJukeBox.ejectRecord`, MCP's name for the insert), aux effect
/// 1005 plays it and one comes off the stack; true. False on anything else.
// src: ItemRecord.java:12-25
pub fn record_on_item_use(
    def: &ItemDef,
    var1: &mut ItemStack,
    var3: &mut World,
    var4: i32,
    var5: i32,
    var6: i32,
) -> bool {
    if var3.get_block_id(var4, var5, var6) == JUKEBOX
        && var3.get_block_metadata(var4, var5, var6) == 0
    {
        if var3.multiplayer_world {
            return true;
        }
        jukebox::eject_record(var3, var4, var5, var6, def.shifted_index);
        var3.play_aux_sfx_at_entity(None, 1005, var4, var5, var6, def.shifted_index);
        var1.stack_size -= 1;
        true
    } else {
        false
    }
}

/// `ItemDye.onItemUse`: only bone meal (damage 15) does anything. On a
/// sapling it grows the tree (`BlockSapling.growTree` with `World.rand`);
/// on crops it ripens them (`BlockCrops.fertilize`); on grass it scatters
/// up to 128 plants: for attempt `i`, a walk of `i / 16` random steps from
/// the cell above (abandoned when it leaves grass or meets a normal cube),
/// then on air tall grass (9 in 10), else a dandelion (2 in 3) or a rose,
/// all drawn from `Item.itemRand`. One item off in each case (not on a
/// multiplayer client); true for those three blocks, false otherwise.
// src: ItemDye.java:21-78
pub fn dye_on_item_use(
    var1: &mut ItemStack,
    var3: &mut World,
    var4: i32,
    var5: i32,
    var6: i32,
) -> bool {
    if var1.get_item_damage() != 15 {
        return false;
    }
    let var8 = var3.get_block_id(var4, var5, var6);
    if var8 == SAPLING {
        if !var3.multiplayer_world {
            sapling::grow_tree(var3, SAPLING, var4, var5, var6, RandomSource::WorldRand);
            var1.stack_size -= 1;
        }
        return true;
    }
    if var8 == CROPS {
        if !var3.multiplayer_world {
            crops::fertilize(var3, var4, var5, var6);
            var1.stack_size -= 1;
        }
        return true;
    }
    if var8 == GRASS {
        if !var3.multiplayer_world {
            var1.stack_size -= 1;
            'label53: for var9 in 0..128 {
                let mut var10 = var4;
                let mut var11 = var5 + 1;
                let mut var12 = var6;
                for _var13 in 0..var9 / 16 {
                    var10 += var3.item_rand.next_int_bounded(3) - 1;
                    // Java evaluates left to right: (a - 1) * b / 2.
                    let a = var3.item_rand.next_int_bounded(3) - 1;
                    let b = var3.item_rand.next_int_bounded(3);
                    var11 += a * b / 2;
                    var12 += var3.item_rand.next_int_bounded(3) - 1;
                    if var3.get_block_id(var10, var11 - 1, var12) != GRASS
                        || var3.is_block_normal_cube(var10, var11, var12)
                    {
                        continue 'label53;
                    }
                }
                if var3.get_block_id(var10, var11, var12) == 0 {
                    if var3.item_rand.next_int_bounded(10) != 0 {
                        var3.set_block_and_metadata_with_notify(var10, var11, var12, TALL_GRASS, 1);
                    } else if var3.item_rand.next_int_bounded(3) != 0 {
                        var3.set_block_with_notify(var10, var11, var12, PLANT_YELLOW);
                    } else {
                        var3.set_block_with_notify(var10, var11, var12, PLANT_RED);
                    }
                }
            }
        }
        return true;
    }
    false
}

/// `ItemDye.saddleEntity`: a dye used on an unsheared sheep of another
/// colour dyes its fleece (`BlockCloth.getBlockFromDye(damage)`, one off the
/// stack).
// src: ItemDye.java:80-90
pub fn dye_saddle_entity(var1: &mut ItemStack, var2: &mut Entity, world: &mut World) {
    let _ = world;
    if is_instance_of(&var2.kind, "EntitySheep") {
        let var4 = block_cloth_get_block_from_dye(var1.get_item_damage());
        if !sheep::get_sheared(var2) && sheep::get_fleece_color(var2) != var4 {
            sheep::set_fleece_color(var2, var4);
            var1.stack_size -= 1;
        }
    }
}
