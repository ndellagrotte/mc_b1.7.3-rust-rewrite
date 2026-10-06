//! `ShapedRecipes` (ShapedRecipes.java): a pattern of up to 3×3 stacks that
//! matches anywhere in the grid, plain or mirrored left to right.

use crate::inventory::crafting::InventoryCrafting;
use crate::item::ItemStack;

/// `ShapedRecipes`.
#[derive(Clone, Debug, PartialEq)]
pub struct ShapedRecipes {
    /// `recipeWidth` (private).
    pub recipe_width: i32,
    /// `recipeHeight` (private).
    pub recipe_height: i32,
    /// `recipeItems` (private): `width * height` entries, row-major; null
    /// for a blank, damage −1 for "any damage" (a `Block` ingredient).
    pub recipe_items: Vec<Option<ItemStack>>,
    /// `recipeOutput` (private).
    pub recipe_output: ItemStack,
    /// `recipeOutputItemID`.
    pub recipe_output_item_id: i32,
}

impl ShapedRecipes {
    /// `new ShapedRecipes(width, height, items, output)`.
    // src: ShapedRecipes.java:10-16
    pub fn new(
        width: i32,
        height: i32,
        items: Vec<Option<ItemStack>>,
        output: ItemStack,
    ) -> ShapedRecipes {
        ShapedRecipes {
            recipe_output_item_id: output.item_id,
            recipe_width: width,
            recipe_height: height,
            recipe_items: items,
            recipe_output: output,
        }
    }

    /// `getRecipeOutput()`.
    // src: ShapedRecipes.java:18-20
    pub fn get_recipe_output(&self) -> &ItemStack {
        &self.recipe_output
    }

    /// `matches(grid)`: every offset at which the pattern fits in a 3×3
    /// grid, x outer, y inner, the mirrored pattern tried first at each.
    /// A 2×2 grid reads as null outside its slots.
    // src: ShapedRecipes.java:22-36
    pub fn matches(&self, var1: &InventoryCrafting) -> bool {
        let mut var2 = 0;
        while var2 <= 3 - self.recipe_width {
            let mut var3 = 0;
            while var3 <= 3 - self.recipe_height {
                if self.func_21137_a(var1, var2, var3, true) {
                    return true;
                }
                if self.func_21137_a(var1, var2, var3, false) {
                    return true;
                }
                var3 += 1;
            }
            var2 += 1;
        }
        false
    }

    /// `func_21137_a(grid, x, y, mirrored)`: the pattern at offset `(x, y)`
    /// against all nine cells; a cell outside the pattern must be empty. An
    /// ingredient of damage −1 matches any damage.
    // src: ShapedRecipes.java:38-70
    fn func_21137_a(&self, var1: &InventoryCrafting, var2: i32, var3: i32, var4: bool) -> bool {
        for var5 in 0..3 {
            for var6 in 0..3 {
                let var7 = var5 - var2;
                let var8 = var6 - var3;
                let mut var9: Option<ItemStack> = None;
                if var7 >= 0 && var8 >= 0 && var7 < self.recipe_width && var8 < self.recipe_height {
                    var9 = if var4 {
                        self.recipe_items
                            [(self.recipe_width - var7 - 1 + var8 * self.recipe_width) as usize]
                    } else {
                        self.recipe_items[(var7 + var8 * self.recipe_width) as usize]
                    };
                }
                let var10 = var1.get_stack_in_row_and_column(var5, var6);
                if var10.is_some() || var9.is_some() {
                    let (Some(var9), Some(var10)) = (var9, var10) else {
                        return false;
                    };
                    if var9.item_id != var10.item_id {
                        return false;
                    }
                    if var9.get_item_damage() != -1
                        && var9.get_item_damage() != var10.get_item_damage()
                    {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// `getCraftingResult(grid)`: a new stack of the output's id, size and
    /// damage.
    // src: ShapedRecipes.java:72-74
    pub fn get_crafting_result(&self, _var1: &InventoryCrafting) -> ItemStack {
        ItemStack::new(
            self.recipe_output.item_id,
            self.recipe_output.stack_size,
            self.recipe_output.get_item_damage(),
        )
    }

    /// `getRecipeSize()`: `width * height`.
    // src: ShapedRecipes.java:76-78
    pub fn get_recipe_size(&self) -> i32 {
        self.recipe_width * self.recipe_height
    }
}
