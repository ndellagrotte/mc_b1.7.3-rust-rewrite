//! `RecipesFood` (RecipesFood.java): the two mushroom soups and cookies.

use super::ids::*;
use super::manager::{CraftingManager, Ingredient};
use crate::item::ItemStack;

/// `addRecipes(manager)`.
// src: RecipesFood.java:4-8
pub(super) fn add_recipes(var1: &mut CraftingManager) {
    var1.add_recipe(
        ItemStack::new(BOWL_SOUP, 1, 0),
        &["Y", "X", "#"],
        &[
            ('X', Ingredient::Block(MUSHROOM_BROWN)),
            ('Y', Ingredient::Block(MUSHROOM_RED)),
            ('#', Ingredient::Item(BOWL_EMPTY)),
        ],
    );
    var1.add_recipe(
        ItemStack::new(BOWL_SOUP, 1, 0),
        &["Y", "X", "#"],
        &[
            ('X', Ingredient::Block(MUSHROOM_RED)),
            ('Y', Ingredient::Block(MUSHROOM_BROWN)),
            ('#', Ingredient::Item(BOWL_EMPTY)),
        ],
    );
    var1.add_recipe(
        ItemStack::new(COOKIE, 8, 0),
        &["#X#"],
        &[
            ('X', Ingredient::Stack(ItemStack::new(DYE_POWDER, 1, 3))),
            ('#', Ingredient::Item(WHEAT)),
        ],
    );
}
