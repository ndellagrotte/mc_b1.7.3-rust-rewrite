//! `tests/golden/recipe/recipe-table` (Tier A kind `recipe_table`, WP-2.5):
//! the Rust `CraftingManager` recipe list, in its sorted order, replayed
//! against the oracle's reflection dump, with the `findMatchingRecipe`
//! probes built from each recipe's own ingredients (format:
//! tests/golden/README.md "`kind: \"recipe_table\"`"; oracle:
//! tools/oracle/java/harness/RecipeTable.java).

use mc_golden::{GoldenError, Scenario, TraceLine, TraceWriter, fixture_dir};
use mc_sim::crafting::{CraftingManager, Recipe};
use mc_sim::inventory::crafting::InventoryCrafting;
use mc_sim::item::ItemStack;
use serde_json::{Value, json};

/// `Dumps.stack`: `{id, count, damage}` or null.
fn stack(s: Option<&ItemStack>) -> Value {
    match s {
        None => Value::Null,
        Some(s) => json!({ "id": s.item_id, "count": s.stack_size, "damage": s.get_item_damage() }),
    }
}

/// `RecipeTable.probe`: count 1, damage −1 as 0.
fn probe(s: Option<&ItemStack>) -> Option<ItemStack> {
    s.map(|s| {
        let d = s.get_item_damage();
        ItemStack::new(s.item_id, 1, if d == -1 { 0 } else { d })
    })
}

fn result(g: &InventoryCrafting) -> Value {
    stack(
        CraftingManager::get_instance()
            .find_matching_recipe(g)
            .as_ref(),
    )
}

fn recipe_line(i: usize, r: &Recipe) -> TraceLine {
    let l = TraceLine::new(0, "recipe").with("i", i);
    match r {
        Recipe::Shaped(s) => {
            let (w, h) = (s.recipe_width, s.recipe_height);
            let items = &s.recipe_items;
            let at = |c: i32, row: i32| items[(c + row * w) as usize].as_ref();
            let mut g = InventoryCrafting::new(3, 3);
            let mut m = InventoryCrafting::new(3, 3);
            for row in 0..h {
                for col in 0..w {
                    g.set_inventory_slot_contents(col + row * 3, probe(at(col, row)));
                    m.set_inventory_slot_contents(col + row * 3, probe(at(w - 1 - col, row)));
                }
            }
            let mut l = l
                .with("kind", "shaped")
                .with("width", w)
                .with("height", h)
                .with("size", r.get_recipe_size())
                .with(
                    "input",
                    Value::Array(items.iter().map(|s| stack(s.as_ref())).collect()),
                )
                .with("output", stack(Some(r.get_recipe_output())))
                .with("output_item_id", s.recipe_output_item_id)
                .with("craft", result(&g))
                .with("craft_mirrored", result(&m));
            if w <= 2 && h <= 2 {
                let mut g2 = InventoryCrafting::new(2, 2);
                for row in 0..h {
                    for col in 0..w {
                        g2.set_inventory_slot_contents(col + row * 2, probe(at(col, row)));
                    }
                }
                l = l.with("craft_2x2", result(&g2));
            }
            l
        }
        Recipe::Shapeless(s) => {
            let items = &s.recipe_items;
            let mut g = InventoryCrafting::new(3, 3);
            for (k, it) in items.iter().enumerate() {
                g.set_inventory_slot_contents(k as i32, probe(Some(it)));
            }
            let mut l = l
                .with("kind", "shapeless")
                .with("size", r.get_recipe_size())
                .with(
                    "input",
                    Value::Array(items.iter().map(|s| stack(Some(s))).collect()),
                )
                .with("output", stack(Some(r.get_recipe_output())))
                .with("craft", result(&g));
            if items.len() <= 4 {
                let mut g2 = InventoryCrafting::new(2, 2);
                for (k, it) in items.iter().enumerate() {
                    g2.set_inventory_slot_contents(k as i32, probe(Some(it)));
                }
                l = l.with("craft_2x2", result(&g2));
            }
            l
        }
    }
}

fn drive(scenario: &Scenario, out: &mut TraceWriter) -> Result<(), GoldenError> {
    if !matches!(scenario.kind, mc_golden::ScenarioKind::RecipeTable) {
        return Err(GoldenError::other("not a recipe_table scenario"));
    }
    let recipes = CraftingManager::get_instance().get_recipe_list();
    out.push(TraceLine::new(0, "recipe_count").with("count", recipes.len()));
    for (i, r) in recipes.iter().enumerate() {
        out.push(recipe_line(i, r));
    }
    Ok(())
}

#[test]
fn recipe_recipe_table() {
    if let Err(e) = mc_golden::run(fixture_dir("recipe", "recipe-table"), drive) {
        panic!("recipe/recipe-table: {e}");
    }
}

/// The count `CraftingManager`'s constructor prints (CraftingManager.java:82).
#[test]
fn recipe_count_is_151() {
    assert_eq!(CraftingManager::get_instance().get_recipe_list().len(), 151);
}
