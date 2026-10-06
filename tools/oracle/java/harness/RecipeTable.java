package harness;

import java.util.List;
import java.util.Map;
import net.minecraft.src.Container;
import net.minecraft.src.CraftingManager;
import net.minecraft.src.EntityPlayer;
import net.minecraft.src.IRecipe;
import net.minecraft.src.InventoryCrafting;
import net.minecraft.src.ItemStack;
import net.minecraft.src.ShapedRecipes;
import net.minecraft.src.ShapelessRecipes;

/**
 * Tier A kind {@code recipe_table} (WP-2.5; tests/golden/README.md "kind: recipe_table"): the
 * {@code CraftingManager} recipe list in its sorted order (CraftingManager.java:16-83, sorted by
 * RecipeSorter), by reflection on the private recipe fields, plus {@code findMatchingRecipe}
 * probes built from each recipe's own ingredients.
 */
final class RecipeTable {
    private RecipeTable() {}

    static Trace run(Map<String, Object> sc) throws Exception {
        // Block's static initialiser before Item's (as ItemTable), then the singleton.
        HarnessWorld.resetStatics(0L);
        List<?> recipes = CraftingManager.getInstance().getRecipeList();
        Trace t = new Trace();
        t.add(new Json.Line().put("t", 0).put("e", "recipe_count").put("count", recipes.size()));
        for (int i = 0; i < recipes.size(); i++) t.add(recipeLine(i, (IRecipe) recipes.get(i)));
        return t;
    }

    /** A grid of {@code w} x {@code w} as the craft op builds it: a container that does nothing. */
    static InventoryCrafting grid(int w) {
        Container c = new Container() {
            @Override
            public boolean isUsableByPlayer(EntityPlayer p) {
                return true;
            }
        };
        return new InventoryCrafting(c, w, w);
    }

    /** An ingredient as a stack a player could hold: count 1, damage -1 ("any") as 0. */
    private static ItemStack probe(ItemStack s) {
        if (s == null) return null;
        return new ItemStack(s.itemID, 1, s.getItemDamage() == -1 ? 0 : s.getItemDamage());
    }

    private static String result(InventoryCrafting g) {
        return Dumps.stack(CraftingManager.getInstance().findMatchingRecipe(g));
    }

    private static String stacks(ItemStack[] a) {
        StringBuilder b = new StringBuilder("[");
        for (int i = 0; i < a.length; i++) {
            if (i > 0) b.append(',');
            b.append(Dumps.stack(a[i]));
        }
        return b.append(']').toString();
    }

    private static Json.Line recipeLine(int i, IRecipe r) {
        Json.Line l = new Json.Line().put("t", 0).put("e", "recipe").put("i", i);
        if (r instanceof ShapedRecipes) {
            int w = Reflect.getInt(ShapedRecipes.class, "recipeWidth", r);
            int h = Reflect.getInt(ShapedRecipes.class, "recipeHeight", r);
            ItemStack[] items = (ItemStack[]) Reflect.get(ShapedRecipes.class, "recipeItems", r);
            l.put("kind", "shaped").put("width", w).put("height", h).put("size", r.getRecipeSize())
                    .raw("input", stacks(items)).raw("output", Dumps.stack(r.getRecipeOutput()))
                    .put("output_item_id", ((ShapedRecipes) r).recipeOutputItemID);
            InventoryCrafting g = grid(3), m = grid(3);
            for (int row = 0; row < h; row++) {
                for (int col = 0; col < w; col++) {
                    g.setInventorySlotContents(col + row * 3, probe(items[col + row * w]));
                    m.setInventorySlotContents(col + row * 3, probe(items[(w - 1 - col) + row * w]));
                }
            }
            l.raw("craft", result(g)).raw("craft_mirrored", result(m));
            if (w <= 2 && h <= 2) {
                InventoryCrafting s = grid(2);
                for (int row = 0; row < h; row++) {
                    for (int col = 0; col < w; col++) s.setInventorySlotContents(col + row * 2, probe(items[col + row * w]));
                }
                l.raw("craft_2x2", result(s));
            }
        } else if (r instanceof ShapelessRecipes) {
            @SuppressWarnings("unchecked")
            List<ItemStack> items = (List<ItemStack>) Reflect.get(ShapelessRecipes.class, "recipeItems", r);
            l.put("kind", "shapeless").put("size", r.getRecipeSize())
                    .raw("input", stacks(items.toArray(new ItemStack[0])))
                    .raw("output", Dumps.stack(r.getRecipeOutput()));
            InventoryCrafting g = grid(3);
            for (int k = 0; k < items.size(); k++) g.setInventorySlotContents(k, probe(items.get(k)));
            l.raw("craft", result(g));
            if (items.size() <= 4) {
                InventoryCrafting s = grid(2);
                for (int k = 0; k < items.size(); k++) s.setInventorySlotContents(k, probe(items.get(k)));
                l.raw("craft_2x2", result(s));
            }
        } else {
            throw new IllegalStateException("unknown IRecipe class " + r.getClass().getName());
        }
        return l;
    }
}
