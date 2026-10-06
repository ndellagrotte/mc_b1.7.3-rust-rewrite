//! `CraftingManager` (CraftingManager.java) and `RecipeSorter`
//! (RecipeSorter.java).

use std::cmp::Ordering;
use std::sync::LazyLock;

use super::ids::*;
use super::shaped::ShapedRecipes;
use super::shapeless::ShapelessRecipes;
use super::{
    Recipe, recipes_armor, recipes_crafting, recipes_dyes, recipes_food, recipes_ingots,
    recipes_tools, recipes_weapons,
};
use crate::inventory::crafting::InventoryCrafting;
use crate::item::ItemStack;

/// One `Object` argument of `addRecipe` (after the pattern) or
/// `addShapelessRecipe`: what the Java code passes, which decides the
/// ingredient stack built from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ingredient {
    /// An `Item` (its `shiftedIndex`): `new ItemStack(item)`, count 1,
    /// damage 0.
    Item(i32),
    /// A `Block` (its `blockID`): in a shaped recipe
    /// `new ItemStack(block, 1, -1)` (any damage), in a shapeless one
    /// `new ItemStack(block)` (damage 0).
    Block(i32),
    /// An `ItemStack`, used as given (shaped: through the key map, then
    /// `copy()`; shapeless: `copy()`).
    Stack(ItemStack),
}

/// `CraftingManager`: the recipe list.
#[derive(Clone, Debug, PartialEq)]
pub struct CraftingManager {
    /// `recipes` (private): in `RecipeSorter` order after construction.
    recipes: Vec<Recipe>,
}

static INSTANCE: LazyLock<CraftingManager> = LazyLock::new(CraftingManager::new);

impl CraftingManager {
    /// `CraftingManager.getInstance()`: the singleton, built on first use.
    // src: CraftingManager.java:9, :12-14
    pub fn get_instance() -> &'static CraftingManager {
        &INSTANCE
    }

    /// The private constructor: the seven `Recipes*` classes in order, then
    /// the miscellaneous recipes, then `Collections.sort(recipes, new
    /// RecipeSorter(this))` (a stable sort). The `System.out.println` of
    /// the count (151) is left out.
    // src: CraftingManager.java:16-83
    pub fn new() -> CraftingManager {
        use Ingredient::{Block as B, Item as I, Stack as S};
        let mut m = CraftingManager {
            recipes: Vec::new(),
        };
        recipes_tools::add_recipes(&mut m);
        recipes_weapons::add_recipes(&mut m);
        recipes_ingots::add_recipes(&mut m);
        recipes_food::add_recipes(&mut m);
        recipes_crafting::add_recipes(&mut m);
        recipes_armor::add_recipes(&mut m);
        recipes_dyes::add_recipes(&mut m);
        let st = ItemStack::new;
        // src: CraftingManager.java:24-80
        m.add_recipe(st(PAPER, 3, 0), &["###"], &[('#', I(REED))]);
        m.add_recipe(st(BOOK, 1, 0), &["#", "#", "#"], &[('#', I(PAPER))]);
        m.add_recipe(st(FENCE, 2, 0), &["###", "###"], &[('#', I(STICK))]);
        m.add_recipe(
            st(JUKEBOX, 1, 0),
            &["###", "#X#", "###"],
            &[('#', B(PLANKS)), ('X', I(DIAMOND))],
        );
        m.add_recipe(
            st(MUSIC_BLOCK, 1, 0),
            &["###", "#X#", "###"],
            &[('#', B(PLANKS)), ('X', I(REDSTONE))],
        );
        m.add_recipe(
            st(BOOK_SHELF, 1, 0),
            &["###", "XXX", "###"],
            &[('#', B(PLANKS)), ('X', I(BOOK))],
        );
        m.add_recipe(st(BLOCK_SNOW, 1, 0), &["##", "##"], &[('#', I(SNOWBALL))]);
        m.add_recipe(st(BLOCK_CLAY, 1, 0), &["##", "##"], &[('#', I(CLAY))]);
        m.add_recipe(st(BRICK, 1, 0), &["##", "##"], &[('#', I(BRICK_ITEM))]);
        m.add_recipe(
            st(GLOW_STONE, 1, 0),
            &["##", "##"],
            &[('#', I(LIGHT_STONE_DUST))],
        );
        m.add_recipe(st(CLOTH, 1, 0), &["##", "##"], &[('#', I(SILK))]);
        m.add_recipe(
            st(TNT, 1, 0),
            &["X#X", "#X#", "X#X"],
            &[('X', I(GUNPOWDER)), ('#', B(SAND))],
        );
        m.add_recipe(st(STAIR_SINGLE, 3, 3), &["###"], &[('#', B(COBBLESTONE))]);
        m.add_recipe(st(STAIR_SINGLE, 3, 0), &["###"], &[('#', B(STONE))]);
        m.add_recipe(st(STAIR_SINGLE, 3, 1), &["###"], &[('#', B(SAND_STONE))]);
        m.add_recipe(st(STAIR_SINGLE, 3, 2), &["###"], &[('#', B(PLANKS))]);
        m.add_recipe(st(LADDER, 2, 0), &["# #", "###", "# #"], &[('#', I(STICK))]);
        m.add_recipe(
            st(DOOR_WOOD, 1, 0),
            &["##", "##", "##"],
            &[('#', B(PLANKS))],
        );
        m.add_recipe(st(TRAPDOOR, 2, 0), &["###", "###"], &[('#', B(PLANKS))]);
        m.add_recipe(
            st(DOOR_STEEL, 1, 0),
            &["##", "##", "##"],
            &[('#', I(INGOT_IRON))],
        );
        m.add_recipe(
            st(SIGN, 1, 0),
            &["###", "###", " X "],
            &[('#', B(PLANKS)), ('X', I(STICK))],
        );
        m.add_recipe(
            st(CAKE, 1, 0),
            &["AAA", "BEB", "CCC"],
            &[
                ('A', I(BUCKET_MILK)),
                ('B', I(SUGAR)),
                ('C', I(WHEAT)),
                ('E', I(EGG)),
            ],
        );
        m.add_recipe(st(SUGAR, 1, 0), &["#"], &[('#', I(REED))]);
        m.add_recipe(st(PLANKS, 4, 0), &["#"], &[('#', B(WOOD))]);
        m.add_recipe(st(STICK, 4, 0), &["#", "#"], &[('#', B(PLANKS))]);
        m.add_recipe(
            st(TORCH_WOOD, 4, 0),
            &["X", "#"],
            &[('X', I(COAL)), ('#', I(STICK))],
        );
        m.add_recipe(
            st(TORCH_WOOD, 4, 0),
            &["X", "#"],
            &[('X', S(st(COAL, 1, 1))), ('#', I(STICK))],
        );
        m.add_recipe(st(BOWL_EMPTY, 4, 0), &["# #", " # "], &[('#', B(PLANKS))]);
        m.add_recipe(
            st(RAIL, 16, 0),
            &["X X", "X#X", "X X"],
            &[('X', I(INGOT_IRON)), ('#', I(STICK))],
        );
        m.add_recipe(
            st(RAIL_POWERED, 6, 0),
            &["X X", "X#X", "XRX"],
            &[('X', I(INGOT_GOLD)), ('R', I(REDSTONE)), ('#', I(STICK))],
        );
        m.add_recipe(
            st(RAIL_DETECTOR, 6, 0),
            &["X X", "X#X", "XRX"],
            &[
                ('X', I(INGOT_IRON)),
                ('R', I(REDSTONE)),
                ('#', B(PRESSURE_PLATE_STONE)),
            ],
        );
        m.add_recipe(
            st(MINECART_EMPTY, 1, 0),
            &["# #", "###"],
            &[('#', I(INGOT_IRON))],
        );
        m.add_recipe(
            st(PUMPKIN_LANTERN, 1, 0),
            &["A", "B"],
            &[('A', B(PUMPKIN)), ('B', B(TORCH_WOOD))],
        );
        m.add_recipe(
            st(MINECART_CRATE, 1, 0),
            &["A", "B"],
            &[('A', B(CHEST)), ('B', I(MINECART_EMPTY))],
        );
        m.add_recipe(
            st(MINECART_POWERED, 1, 0),
            &["A", "B"],
            &[('A', B(STONE_OVEN_IDLE)), ('B', I(MINECART_EMPTY))],
        );
        m.add_recipe(st(BOAT, 1, 0), &["# #", "###"], &[('#', B(PLANKS))]);
        m.add_recipe(
            st(BUCKET_EMPTY, 1, 0),
            &["# #", " # "],
            &[('#', I(INGOT_IRON))],
        );
        m.add_recipe(
            st(FLINT_AND_STEEL, 1, 0),
            &["A ", " B"],
            &[('A', I(INGOT_IRON)), ('B', I(FLINT))],
        );
        m.add_recipe(st(BREAD, 1, 0), &["###"], &[('#', I(WHEAT))]);
        m.add_recipe(
            st(STAIR_COMPACT_PLANKS, 4, 0),
            &["#  ", "## ", "###"],
            &[('#', B(PLANKS))],
        );
        m.add_recipe(
            st(FISHING_ROD, 1, 0),
            &["  #", " #X", "# X"],
            &[('#', I(STICK)), ('X', I(SILK))],
        );
        m.add_recipe(
            st(STAIR_COMPACT_COBBLESTONE, 4, 0),
            &["#  ", "## ", "###"],
            &[('#', B(COBBLESTONE))],
        );
        m.add_recipe(
            st(PAINTING, 1, 0),
            &["###", "#X#", "###"],
            &[('#', I(STICK)), ('X', B(CLOTH))],
        );
        m.add_recipe(
            st(APPLE_GOLD, 1, 0),
            &["###", "#X#", "###"],
            &[('#', B(BLOCK_GOLD)), ('X', I(APPLE_RED))],
        );
        m.add_recipe(
            st(LEVER, 1, 0),
            &["X", "#"],
            &[('#', B(COBBLESTONE)), ('X', I(STICK))],
        );
        m.add_recipe(
            st(TORCH_REDSTONE_ACTIVE, 1, 0),
            &["X", "#"],
            &[('#', I(STICK)), ('X', I(REDSTONE))],
        );
        m.add_recipe(
            st(REDSTONE_REPEATER, 1, 0),
            &["#X#", "III"],
            &[
                ('#', B(TORCH_REDSTONE_ACTIVE)),
                ('X', I(REDSTONE)),
                ('I', B(STONE)),
            ],
        );
        m.add_recipe(
            st(POCKET_SUNDIAL, 1, 0),
            &[" # ", "#X#", " # "],
            &[('#', I(INGOT_GOLD)), ('X', I(REDSTONE))],
        );
        m.add_recipe(
            st(COMPASS, 1, 0),
            &[" # ", "#X#", " # "],
            &[('#', I(INGOT_IRON)), ('X', I(REDSTONE))],
        );
        m.add_recipe(
            st(MAP_ITEM, 1, 0),
            &["###", "#X#", "###"],
            &[('#', I(PAPER)), ('X', I(COMPASS))],
        );
        m.add_recipe(st(BUTTON, 1, 0), &["#", "#"], &[('#', B(STONE))]);
        m.add_recipe(st(PRESSURE_PLATE_STONE, 1, 0), &["##"], &[('#', B(STONE))]);
        m.add_recipe(
            st(PRESSURE_PLATE_PLANKS, 1, 0),
            &["##"],
            &[('#', B(PLANKS))],
        );
        m.add_recipe(
            st(DISPENSER, 1, 0),
            &["###", "#X#", "#R#"],
            &[('#', B(COBBLESTONE)), ('X', I(BOW)), ('R', I(REDSTONE))],
        );
        m.add_recipe(
            st(PISTON_BASE, 1, 0),
            &["TTT", "#X#", "#R#"],
            &[
                ('#', B(COBBLESTONE)),
                ('X', I(INGOT_IRON)),
                ('R', I(REDSTONE)),
                ('T', B(PLANKS)),
            ],
        );
        m.add_recipe(
            st(PISTON_STICKY_BASE, 1, 0),
            &["S", "P"],
            &[('S', I(SLIME_BALL)), ('P', B(PISTON_BASE))],
        );
        m.add_recipe(
            st(BED, 1, 0),
            &["###", "XXX"],
            &[('#', B(CLOTH)), ('X', B(PLANKS))],
        );
        // src: CraftingManager.java:81 (Collections.sort is stable)
        m.recipes.sort_by(compare_recipes);
        // src: CraftingManager.java:82: System.out.println(size + " recipes") (stdout only)
        m
    }

    /// `addRecipe(output, pattern..., key, ingredient, ...)`: the pattern
    /// rows are concatenated, the width is the last row's length and the
    /// height the row count (`String[]` or separate `String` arguments give
    /// the same result); each key maps to its ingredient stack
    /// (`HashMap.put`, so a repeated key keeps its last ingredient; used for
    /// lookups only). Cells whose character has no key are null.
    ///
    /// Panics where Java throws `StringIndexOutOfBoundsException` (rows of
    /// unequal lengths that leave the concatenation shorter than
    /// `width * height`; never in vanilla).
    // src: CraftingManager.java:85-135
    pub fn add_recipe(&mut self, var1: ItemStack, rows: &[&str], keys: &[(char, Ingredient)]) {
        let mut var3: Vec<char> = Vec::new();
        let mut var5 = 0i32;
        let mut var6 = 0i32;
        for var9 in rows {
            var6 += 1;
            var5 = var9.chars().count() as i32;
            var3.extend(var9.chars());
        }
        // The key map: Item → new ItemStack(item); Block → new ItemStack(block, 1, -1);
        // ItemStack → itself.
        let lookup = |c: char| -> Option<ItemStack> {
            keys.iter()
                .rev()
                .find(|(k, _)| *k == c)
                .map(|(_, v)| match *v {
                    Ingredient::Item(id) => ItemStack::new(id, 1, 0),
                    Ingredient::Block(id) => ItemStack::new(id, 1, -1),
                    Ingredient::Stack(s) => s,
                })
        };
        let n = var5 * var6;
        let mut var14: Vec<Option<ItemStack>> = Vec::with_capacity(n.max(0) as usize);
        for var16 in 0..n {
            let Some(&var10) = var3.get(var16 as usize) else {
                panic!("StringIndexOutOfBoundsException: index {var16} in addRecipe's pattern");
            };
            var14.push(lookup(var10).map(|s| s.copy()));
        }
        self.recipes
            .push(Recipe::Shaped(ShapedRecipes::new(var5, var6, var14, var1)));
    }

    /// `addShapelessRecipe(output, ingredients...)`: `ItemStack` → `copy()`,
    /// `Item` → `new ItemStack(item)`, `Block` → `new ItemStack(block)`
    /// (damage 0, unlike a shaped recipe's block).
    // src: CraftingManager.java:137-158
    pub fn add_shapeless_recipe(&mut self, var1: ItemStack, var2: &[Ingredient]) {
        let var3: Vec<ItemStack> = var2
            .iter()
            .map(|var7| match *var7 {
                Ingredient::Stack(s) => s.copy(),
                Ingredient::Item(id) => ItemStack::new(id, 1, 0),
                Ingredient::Block(id) => ItemStack::new(id, 1, 0),
            })
            .collect();
        self.recipes
            .push(Recipe::Shapeless(ShapelessRecipes::new(var1, var3)));
    }

    /// `findMatchingRecipe(grid)`: the first recipe in list order that
    /// matches, its `getCraftingResult`; null when none does.
    // src: CraftingManager.java:160-169
    pub fn find_matching_recipe(&self, var1: &InventoryCrafting) -> Option<ItemStack> {
        self.recipes
            .iter()
            .find(|r| r.matches(var1))
            .map(|r| r.get_crafting_result(var1))
    }

    /// `getRecipeList()`.
    // src: CraftingManager.java:171-173
    pub fn get_recipe_list(&self) -> &[Recipe] {
        &self.recipes
    }
}

impl Default for CraftingManager {
    fn default() -> Self {
        CraftingManager::new()
    }
}

/// `RecipeSorter.compareRecipes(a, b)`: shaped before shapeless, then the
/// larger `getRecipeSize()` first; equal otherwise (the stable sort keeps
/// registration order).
// src: RecipeSorter.java:13-23
#[expect(
    clippy::if_same_then_else,
    reason = "the branches of RecipeSorter.compareRecipes, transcribed"
)]
pub fn compare_recipes(var1: &Recipe, var2: &Recipe) -> Ordering {
    let shapeless = |r: &Recipe| matches!(r, Recipe::Shapeless(_));
    if shapeless(var1) && !shapeless(var2) {
        Ordering::Greater
    } else if shapeless(var2) && !shapeless(var1) {
        Ordering::Less
    } else if var2.get_recipe_size() < var1.get_recipe_size() {
        Ordering::Less
    } else if var2.get_recipe_size() > var1.get_recipe_size() {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}
