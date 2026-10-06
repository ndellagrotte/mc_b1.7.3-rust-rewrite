//! `RecipesIngots` (RecipesIngots.java): the four storage blocks and back.

use super::ids::*;
use super::manager::{CraftingManager, Ingredient};
use crate::item::ItemStack;

/// `recipeItems`: `{block, ingot stack}`. The ingot stack has count 9 and
/// is both the ingredient of the block recipe (copied, count 9 kept: only
/// id and damage are matched) and the output of the reverse recipe.
// src: RecipesIngots.java:6-8
const RECIPE_ITEMS: [(i32, ItemStack); 4] = [
    (BLOCK_GOLD, ItemStack::new(INGOT_GOLD, 9, 0)),
    (BLOCK_STEEL, ItemStack::new(INGOT_IRON, 9, 0)),
    (BLOCK_DIAMOND, ItemStack::new(DIAMOND, 9, 0)),
    (BLOCK_LAPIS, ItemStack::new(DYE_POWDER, 9, 4)),
];

/// `addRecipes(manager)`: per pair, the block from nine ingots, then nine
/// ingots from the block (any damage).
// src: RecipesIngots.java:10-18
pub(super) fn add_recipes(var1: &mut CraftingManager) {
    for (var3, var4) in RECIPE_ITEMS {
        var1.add_recipe(
            ItemStack::new(var3, 1, 0),
            &["###", "###", "###"],
            &[('#', Ingredient::Stack(var4))],
        );
        var1.add_recipe(var4, &["#"], &[('#', Ingredient::Block(var3))]);
    }
}
