//! `RecipesArmor` (RecipesArmor.java): helmets, plates, leggings and boots
//! of the five materials.
//!
//! Vanilla quirk, transcribed: the chain armour's material is `Block.fire`
//! (id 51, any damage), which no player can hold, so chain armour cannot be
//! crafted.

use super::ids::*;
use super::manager::{CraftingManager, Ingredient};
use crate::item::ItemStack;

/// `recipePatterns`: helmet, plate, leggings, boots.
// src: RecipesArmor.java:4
const RECIPE_PATTERNS: [&[&str]; 4] = [
    &["XXX", "X X"],
    &["X X", "XXX", "XXX"],
    &["XXX", "X X", "X X"],
    &["X X", "X X"],
];

/// `recipeItems[0]`: the materials.
// src: RecipesArmor.java:8
const MATERIALS: [Ingredient; 5] = [
    Ingredient::Item(LEATHER),
    Ingredient::Block(FIRE),
    Ingredient::Item(INGOT_IRON),
    Ingredient::Item(DIAMOND),
    Ingredient::Item(INGOT_GOLD),
];

/// `recipeItems[1..]`: the armour pieces, one row per pattern.
// src: RecipesArmor.java:8
const ARMOR: [[i32; 5]; 4] = [
    [
        HELMET_LEATHER,
        HELMET_CHAIN,
        HELMET_STEEL,
        HELMET_DIAMOND,
        HELMET_GOLD,
    ],
    [
        PLATE_LEATHER,
        PLATE_CHAIN,
        PLATE_STEEL,
        PLATE_DIAMOND,
        PLATE_GOLD,
    ],
    [
        LEGS_LEATHER,
        LEGS_CHAIN,
        LEGS_STEEL,
        LEGS_DIAMOND,
        LEGS_GOLD,
    ],
    [
        BOOTS_LEATHER,
        BOOTS_CHAIN,
        BOOTS_STEEL,
        BOOTS_DIAMOND,
        BOOTS_GOLD,
    ],
];

/// `addRecipes(manager)`: material outer, piece inner.
// src: RecipesArmor.java:14-24
pub(super) fn add_recipes(var1: &mut CraftingManager) {
    for (var2, var3) in MATERIALS.iter().enumerate() {
        for (var4, row) in ARMOR.iter().enumerate() {
            var1.add_recipe(
                ItemStack::new(row[var2], 1, 0),
                RECIPE_PATTERNS[var4],
                &[('X', *var3)],
            );
        }
    }
}
