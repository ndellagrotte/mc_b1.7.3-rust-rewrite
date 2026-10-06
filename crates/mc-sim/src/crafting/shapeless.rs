//! `ShapelessRecipes` (ShapelessRecipes.java): a multiset of ingredients in
//! any cells.

use crate::inventory::crafting::InventoryCrafting;
use crate::item::ItemStack;

/// `ShapelessRecipes`.
#[derive(Clone, Debug, PartialEq)]
pub struct ShapelessRecipes {
    /// `recipeOutput` (private, final).
    pub recipe_output: ItemStack,
    /// `recipeItems` (private, final): the ingredients in argument order.
    pub recipe_items: Vec<ItemStack>,
}

impl ShapelessRecipes {
    /// `new ShapelessRecipes(output, items)`.
    // src: ShapelessRecipes.java:11-14
    pub fn new(output: ItemStack, items: Vec<ItemStack>) -> ShapelessRecipes {
        ShapelessRecipes {
            recipe_output: output,
            recipe_items: items,
        }
    }

    /// `getRecipeOutput()`.
    // src: ShapelessRecipes.java:16-18
    pub fn get_recipe_output(&self) -> &ItemStack {
        &self.recipe_output
    }

    /// `matches(grid)`: rows outer, columns inner; each non-empty cell takes
    /// the first remaining ingredient with its id and (unless the
    /// ingredient's damage is −1) its damage, out of a copy of the list
    /// (`ArrayList.remove(Object)`: `ItemStack` keeps `Object.equals`, so
    /// the element found is the one removed). A cell nothing takes, or an
    /// ingredient left over, fails.
    // src: ShapelessRecipes.java:20-47
    pub fn matches(&self, var1: &InventoryCrafting) -> bool {
        let mut var2: Vec<ItemStack> = self.recipe_items.clone();
        for var3 in 0..3 {
            for var4 in 0..3 {
                if let Some(var5) = var1.get_stack_in_row_and_column(var4, var3) {
                    let found = var2.iter().position(|var8| {
                        var5.item_id == var8.item_id
                            && (var8.get_item_damage() == -1
                                || var5.get_item_damage() == var8.get_item_damage())
                    });
                    match found {
                        Some(i) => {
                            var2.remove(i);
                        }
                        None => return false,
                    }
                }
            }
        }
        var2.is_empty()
    }

    /// `getCraftingResult(grid)`: `recipeOutput.copy()`.
    // src: ShapelessRecipes.java:49-51
    pub fn get_crafting_result(&self, _var1: &InventoryCrafting) -> ItemStack {
        self.recipe_output.copy()
    }

    /// `getRecipeSize()`: the ingredient count.
    // src: ShapelessRecipes.java:53-55
    pub fn get_recipe_size(&self) -> i32 {
        self.recipe_items.len() as i32
    }
}
