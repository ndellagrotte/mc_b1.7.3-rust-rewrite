//! `RecipesTools` (RecipesTools.java): pickaxes, shovels, axes and hoes of
//! the five materials, then shears.

use super::ids::*;
use super::manager::{CraftingManager, Ingredient};
use crate::item::ItemStack;

/// `recipePatterns`: pickaxe, shovel, axe, hoe.
// src: RecipesTools.java:4
const RECIPE_PATTERNS: [&[&str]; 4] = [
    &["XXX", " # ", " # "],
    &["X", "#", "#"],
    &["XX", "X#", " #"],
    &["XX", " #", " #"],
];

/// `recipeItems[0]`: the head materials (blocks match any damage).
// src: RecipesTools.java:8
const MATERIALS: [Ingredient; 5] = [
    Ingredient::Block(PLANKS),
    Ingredient::Block(COBBLESTONE),
    Ingredient::Item(INGOT_IRON),
    Ingredient::Item(DIAMOND),
    Ingredient::Item(INGOT_GOLD),
];

/// `recipeItems[1..]`: the tools, one row per pattern, one column per
/// material.
// src: RecipesTools.java:8
const TOOLS: [[i32; 5]; 4] = [
    [
        PICKAXE_WOOD,
        PICKAXE_STONE,
        PICKAXE_STEEL,
        PICKAXE_DIAMOND,
        PICKAXE_GOLD,
    ],
    [
        SHOVEL_WOOD,
        SHOVEL_STONE,
        SHOVEL_STEEL,
        SHOVEL_DIAMOND,
        SHOVEL_GOLD,
    ],
    [AXE_WOOD, AXE_STONE, AXE_STEEL, AXE_DIAMOND, AXE_GOLD],
    [HOE_WOOD, HOE_STONE, HOE_STEEL, HOE_DIAMOND, HOE_GOLD],
];

/// `addRecipes(manager)`: material outer, tool inner.
// src: RecipesTools.java:11-22
pub(super) fn add_recipes(var1: &mut CraftingManager) {
    for (var2, var3) in MATERIALS.iter().enumerate() {
        for (var4, row) in TOOLS.iter().enumerate() {
            let var5 = row[var2];
            var1.add_recipe(
                ItemStack::new(var5, 1, 0),
                RECIPE_PATTERNS[var4],
                &[('#', Ingredient::Item(STICK)), ('X', *var3)],
            );
        }
    }
    var1.add_recipe(
        ItemStack::new(SHEARS, 1, 0),
        &[" #", "# "],
        &[('#', Ingredient::Item(INGOT_IRON))],
    );
}
