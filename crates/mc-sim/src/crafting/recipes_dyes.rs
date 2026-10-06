//! `RecipesDyes` (RecipesDyes.java): dyed wool and the dye mixes (all
//! shapeless).

use super::ids::*;
use super::manager::{CraftingManager, Ingredient};
use crate::item::ItemStack;

/// `BlockCloth.getDyeFromBlock(i)`: `~i & 15`.
// src: BlockCloth.java:25-27
const fn get_dye_from_block(var0: i32) -> i32 {
    !var0 & 15
}

fn dye(damage: i32) -> Ingredient {
    Ingredient::Stack(ItemStack::new(DYE_POWDER, 1, damage))
}

/// `addRecipes(manager)`: wool of colour `getDyeFromBlock(d)` from dye `d`
/// and white wool (`Item.itemsList[Block.cloth.blockID]`, damage 0) for
/// `d` in 0..16, then the dyes from flowers and bone and the mixes.
// src: RecipesDyes.java:4-24
pub(super) fn add_recipes(var1: &mut CraftingManager) {
    for var2 in 0..16 {
        var1.add_shapeless_recipe(
            ItemStack::new(CLOTH, 1, get_dye_from_block(var2)),
            &[dye(var2), Ingredient::Stack(ItemStack::new(CLOTH, 1, 0))],
        );
    }
    let out = |count, damage| ItemStack::new(DYE_POWDER, count, damage);
    var1.add_shapeless_recipe(out(2, 11), &[Ingredient::Block(PLANT_YELLOW)]);
    var1.add_shapeless_recipe(out(2, 1), &[Ingredient::Block(PLANT_RED)]);
    var1.add_shapeless_recipe(out(3, 15), &[Ingredient::Item(BONE)]);
    var1.add_shapeless_recipe(out(2, 9), &[dye(1), dye(15)]);
    var1.add_shapeless_recipe(out(2, 14), &[dye(1), dye(11)]);
    var1.add_shapeless_recipe(out(2, 10), &[dye(2), dye(15)]);
    var1.add_shapeless_recipe(out(2, 8), &[dye(0), dye(15)]);
    var1.add_shapeless_recipe(out(2, 7), &[dye(8), dye(15)]);
    var1.add_shapeless_recipe(out(3, 7), &[dye(0), dye(15), dye(15)]);
    var1.add_shapeless_recipe(out(2, 12), &[dye(4), dye(15)]);
    var1.add_shapeless_recipe(out(2, 6), &[dye(4), dye(2)]);
    var1.add_shapeless_recipe(out(2, 5), &[dye(4), dye(1)]);
    var1.add_shapeless_recipe(out(2, 13), &[dye(5), dye(9)]);
    var1.add_shapeless_recipe(out(3, 13), &[dye(4), dye(1), dye(9)]);
    var1.add_shapeless_recipe(out(4, 13), &[dye(4), dye(1), dye(1), dye(15)]);
}
