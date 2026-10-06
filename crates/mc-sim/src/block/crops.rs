//! `BlockCrops` (BlockCrops.java): wheat, a `BlockFlower` that grows on
//! farmland and drops seeds.
//!
//! Fully transcribed: `updateTick`, `fertilize` (bone meal's target, called
//! by `ItemDye.onItemUse` in WP-2.6), `getGrowthRate`,
//! `dropBlockAsItemWithChance`. `canThisPlantGrowOnThisBlockID` is in
//! [`super::flower`]; textures, render type, `idDropped`, `quantityDropped`
//! are data in [`crate::block`].
//!
//! Vanilla behaviour, transcribed: `updateTick` runs `BlockFlower.updateTick`
//! first, which may remove the crop, and then carries on with the growth
//! roll on whatever is there now (metadata 0 of air passes `< 7`).

use mc_core::java_random::JavaRandom;

use super::dispatch::{self, RandomSource};
use super::flower;
use super::ids::TILLED_FIELD;
use crate::entity::item::new_entity_item;
use crate::item::{ItemStack, ids as items};
use crate::world::World;

/// `updateTick(world, x, y, z, rand)`: `BlockFlower.updateTick`, then in
/// light 9 or more a one in `(int)(100 / growthRate)` chance to grow a stage.
// src: BlockCrops.java:18-31
pub fn update_tick(world: &mut World, id: i32, x: i32, y: i32, z: i32, rand: RandomSource) {
    flower::update_tick(world, id, x, y, z);
    if world.get_block_light_value(x, y + 1, z) >= 9 {
        let mut var6 = world.get_block_metadata(x, y, z);
        if var6 < 7 {
            let var7 = get_growth_rate(world, id, x, y, z);
            if world
                .random(rand)
                .next_int_bounded((100.0f32 / var7) as i32)
                == 0
            {
                var6 += 1;
                world.set_block_metadata_with_notify(x, y, z, var6);
            }
        }
    }
}

/// `fertilize(world, x, y, z)`: straight to the last stage.
// src: BlockCrops.java:33-35
pub fn fertilize(world: &mut World, x: i32, y: i32, z: i32) {
    world.set_block_metadata_with_notify(x, y, z, 7);
}

/// `getGrowthRate(world, x, y, z)` (private): 1, plus 1 (dry) or 3 (wet) for
/// the farmland below and a quarter of that for each of the 8 around it;
/// halved when crops of this kind stand diagonally, or on both an x and a z
/// side.
// src: BlockCrops.java:37-75
fn get_growth_rate(world: &mut World, id: i32, x: i32, y: i32, z: i32) -> f32 {
    let mut var5 = 1.0f32;
    let var6 = world.get_block_id(x, y, z - 1);
    let var7 = world.get_block_id(x, y, z + 1);
    let var8 = world.get_block_id(x - 1, y, z);
    let var9 = world.get_block_id(x + 1, y, z);
    let var10 = world.get_block_id(x - 1, y, z - 1);
    let var11 = world.get_block_id(x + 1, y, z - 1);
    let var12 = world.get_block_id(x + 1, y, z + 1);
    let var13 = world.get_block_id(x - 1, y, z + 1);
    let var14 = var8 == id || var9 == id;
    let var15 = var6 == id || var7 == id;
    let var16 = var10 == id || var11 == id || var12 == id || var13 == id;
    for var17 in (x - 1)..=(x + 1) {
        for var18 in (z - 1)..=(z + 1) {
            let var19 = world.get_block_id(var17, y - 1, var18);
            let mut var20 = 0.0f32;
            if var19 == TILLED_FIELD {
                var20 = 1.0;
                if world.get_block_metadata(var17, y - 1, var18) > 0 {
                    var20 = 3.0;
                }
            }
            if var17 != x || var18 != z {
                var20 /= 4.0;
            }
            var5 += var20;
        }
    }
    if var16 || var14 && var15 {
        var5 /= 2.0;
    }
    var5
}

/// The base `Block.dropBlockAsItemWithChance` body (`super` here; the
/// dispatch's copy is private to it): `quantityDropped` rolls of
/// `nextFloat() <= chance`, each dropping `idDropped(meta)` when positive.
// src: Block.java:339-353
fn base_drop_block_as_item_with_chance(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    meta: i32,
    chance: f32,
) {
    let Some(b) = super::block(id) else { return };
    if !world.multiplayer_world {
        let var7 = super::quantity_dropped(b, &mut world.rand);
        for _var8 in 0..var7 {
            if world.rand.next_float() <= chance {
                let var9 = super::id_dropped(b, meta, &mut world.rand);
                if var9 > 0 {
                    let stack = ItemStack::new(var9, 1, super::damage_dropped(b, meta));
                    dispatch::drop_block_as_item_do(world, id, x, y, z, stack);
                }
            }
        }
    }
}

/// `dropBlockAsItemWithChance(world, x, y, z, meta, chance)`: `super` (the
/// wheat at stage 7), then three rolls of `nextInt(15) <= meta`, each
/// dropping one seed (`chance` is not applied to them).
// src: BlockCrops.java:89-105
#[allow(clippy::too_many_arguments)]
pub fn drop_block_as_item_with_chance(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    meta: i32,
    chance: f32,
) {
    base_drop_block_as_item_with_chance(world, id, x, y, z, meta, chance);
    if !world.multiplayer_world {
        for _var7 in 0..3 {
            if world.rand.next_int_bounded(15) <= meta {
                let var8 = 0.7f32;
                let r: &mut JavaRandom = &mut world.rand;
                let var9 = r.next_float() * var8 + (1.0f32 - var8) * 0.5f32;
                let var10 = r.next_float() * var8 + (1.0f32 - var8) * 0.5f32;
                let var11 = r.next_float() * var8 + (1.0f32 - var8) * 0.5f32;
                // src: BlockCrops.java:98: `(double)((float)x + var9)`, a float sum;
                // `new ItemStack(Item.seeds)` is one seed, damage 0.
                let mut var12 = new_entity_item(
                    world,
                    (x as f32 + var9) as f64,
                    (y as f32 + var10) as f64,
                    (z as f32 + var11) as f64,
                    ItemStack::new(items::SEEDS, 1, 0),
                );
                if let Some(d) = var12.kind.item_mut() {
                    d.delay_before_can_pickup = 10;
                }
                world.spawn_entity_in_world(var12);
            }
        }
    }
}
