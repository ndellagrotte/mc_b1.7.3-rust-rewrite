//! `RecipesCrafting` (RecipesCrafting.java): chest, furnace, workbench,
//! sandstone.

use super::ids::*;
use super::manager::{CraftingManager, Ingredient};
use crate::item::ItemStack;

/// `addRecipes(manager)`.
// src: RecipesCrafting.java:4-9
pub(super) fn add_recipes(var1: &mut CraftingManager) {
    var1.add_recipe(
        ItemStack::new(CHEST, 1, 0),
        &["###", "# #", "###"],
        &[('#', Ingredient::Block(PLANKS))],
    );
    var1.add_recipe(
        ItemStack::new(STONE_OVEN_IDLE, 1, 0),
        &["###", "# #", "###"],
        &[('#', Ingredient::Block(COBBLESTONE))],
    );
    var1.add_recipe(
        ItemStack::new(WORKBENCH, 1, 0),
        &["##", "##"],
        &[('#', Ingredient::Block(PLANKS))],
    );
    var1.add_recipe(
        ItemStack::new(SAND_STONE, 1, 0),
        &["##", "##"],
        &[('#', Ingredient::Block(SAND))],
    );
}
