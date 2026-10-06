//! `RecipesWeapons` (RecipesWeapons.java): swords of the five materials,
//! then the bow and arrows.

use super::ids::*;
use super::manager::{CraftingManager, Ingredient};
use crate::item::ItemStack;

/// `recipePatterns`: the sword.
// src: RecipesWeapons.java:4
const RECIPE_PATTERNS: [&[&str]; 1] = [&["X", "X", "#"]];

/// `recipeItems[0]`: the blade materials.
// src: RecipesWeapons.java:8
const MATERIALS: [Ingredient; 5] = [
    Ingredient::Block(PLANKS),
    Ingredient::Block(COBBLESTONE),
    Ingredient::Item(INGOT_IRON),
    Ingredient::Item(DIAMOND),
    Ingredient::Item(INGOT_GOLD),
];

/// `recipeItems[1]`: the swords.
// src: RecipesWeapons.java:8
const WEAPONS: [[i32; 5]; 1] = [[
    SWORD_WOOD,
    SWORD_STONE,
    SWORD_STEEL,
    SWORD_DIAMOND,
    SWORD_GOLD,
]];

/// `addRecipes(manager)`.
// src: RecipesWeapons.java:11-23
pub(super) fn add_recipes(var1: &mut CraftingManager) {
    for (var2, var3) in MATERIALS.iter().enumerate() {
        for (var4, row) in WEAPONS.iter().enumerate() {
            var1.add_recipe(
                ItemStack::new(row[var2], 1, 0),
                RECIPE_PATTERNS[var4],
                &[('#', Ingredient::Item(STICK)), ('X', *var3)],
            );
        }
    }
    var1.add_recipe(
        ItemStack::new(BOW, 1, 0),
        &[" #X", "# X", " #X"],
        &[
            ('X', Ingredient::Item(SILK)),
            ('#', Ingredient::Item(STICK)),
        ],
    );
    var1.add_recipe(
        ItemStack::new(ARROW, 4, 0),
        &["X", "#", "Y"],
        &[
            ('Y', Ingredient::Item(FEATHER)),
            ('X', Ingredient::Item(FLINT)),
            ('#', Ingredient::Item(STICK)),
        ],
    );
}
