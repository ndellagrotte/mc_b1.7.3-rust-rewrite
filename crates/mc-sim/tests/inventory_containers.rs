//! WP-2.5 (part 2): unit checks of `InventoryPlayer`, the containers and the
//! crafting manager that the fixtures (`tests/golden/recipe/recipe-table`,
//! `tests/golden/tick/{craft-ops, inventory-slot-clicks,
//! chest-container-clicks, furnace-dispenser-clicks, workbench-crafting}`)
//! do not reach: picking items up into the inventory
//! (`addItemStackToInventory`, called by `EntityItem.onCollideWithPlayer`),
//! armour value and wear, `dropAllItems`, the drop paths of `slotClick`
//! (slot −999) and `onCraftGuiClosed` (both through `dropPlayerItem`, a
//! recorded gap until WP-2.5a), `isUsableByPlayer`, `updateCraftingResults`,
//! `updateProgressBar`, the pumpkin helmet, a map's `onCreated` gap,
//! `InventoryBasic`, and the recipe sorter.

use mc_sim::crafting::manager::compare_recipes;
use mc_sim::crafting::shaped::ShapedRecipes;
use mc_sim::crafting::shapeless::ShapelessRecipes;
use mc_sim::crafting::{CraftingManager, Ingredient, Recipe};
use mc_sim::entity::Entity;
use mc_sim::entity::player::new_entity_player_sp;
use mc_sim::inventory::basic::InventoryBasic;
use mc_sim::inventory::container::ContainerKind;
use mc_sim::inventory::container::furnace::new_container_furnace;
use mc_sim::inventory::container::player::new_container_player;
use mc_sim::inventory::container::workbench::new_container_workbench;
use mc_sim::inventory::crafting::InventoryCrafting;
use mc_sim::inventory::player::{
    can_interact_with, damage_armor, decrement_animations, drop_all_items,
};
use mc_sim::inventory::{InventoryPlayer, InventoryRef};
use mc_sim::item::ItemStack;
use mc_sim::save::NoSave;
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::tile_entity::TileEntityKindData;
use mc_sim::world::{World, WorldEnvironment};
use std::cmp::Ordering;

const COBBLESTONE: i32 = 4;
const PLANKS: i32 = 5;
const STICK: i32 = 280;
const HELMET_DIAMOND: i32 = 310;
const PLATE_STEEL: i32 = 307;
const PUMPKIN: i32 = 86;
const PICKAXE_STEEL: i32 = 257;

fn empty_world(seed: i64) -> World {
    let mut w = World::new(
        Box::new(NoSave),
        "harness",
        WorldProvider::new(Dimension::Surface),
        seed,
        ChunkGenerator::Empty(EmptyChunkProvider),
        WorldEnvironment {
            rand_seed: 0,
            field_9437_g: 0,
            lock_timestamp: 0,
        },
    );
    w.make_deterministic(seed);
    for dz in -1..=1 {
        for dx in -1..=1 {
            w.get_chunk_from_chunk_coords(dx, dz);
        }
    }
    w
}

fn inv(p: &Entity) -> &InventoryPlayer {
    &p.kind.player().unwrap().inventory
}

fn inv_mut(p: &mut Entity) -> &mut InventoryPlayer {
    &mut p.kind.player_mut().unwrap().inventory
}

/// The `EntityItem`s in `loadedEntityList` (the drops of
/// `dropPlayerItemWithRandomChoice`, WP-2.5a).
fn dropped(w: &World) -> Vec<ItemStack> {
    w.loaded_entity_list
        .iter()
        .filter_map(|s| s.entity.as_deref())
        .filter_map(|e| e.kind.item().and_then(|d| d.item))
        .collect()
}

fn hits(w: &World, what: &str) -> u64 {
    w.unimplemented_hits
        .borrow()
        .get(what)
        .copied()
        .unwrap_or(0)
}

// ---- InventoryPlayer -------------------------------------------------------------------

/// `addItemStackToInventory`: an undamaged stack tops up the first matching
/// slot with room (animationsToGo 5), then fills empty slots; the caller's
/// size goes to what is left (InventoryPlayer.java:74-152).
#[test]
fn add_item_stack_tops_up_then_fills_empty_slots() {
    let mut i = InventoryPlayer::default();
    i.main_inventory[3] = Some(ItemStack::new(COBBLESTONE, 60, 0));
    let mut s = ItemStack::new(COBBLESTONE, 70, 0);
    assert!(i.add_item_stack_to_inventory(&mut s));
    assert_eq!(s.stack_size, 0);
    let m3 = i.main_inventory[3].unwrap();
    assert_eq!((m3.stack_size, m3.animations_to_go), (64, 5));
    // The first empty slot (0) gets the remaining 66 - capped at 64, then slot 1 the last 2.
    assert_eq!(i.main_inventory[0].unwrap().stack_size, 64);
    assert_eq!(i.main_inventory[1].unwrap().stack_size, 2);
}

/// A damaged item goes whole into the first empty slot; with no empty slot
/// nothing happens (InventoryPlayer.java:133-143).
#[test]
fn add_item_stack_damaged_needs_an_empty_slot() {
    let mut i = InventoryPlayer::default();
    let mut s = ItemStack::new(PICKAXE_STEEL, 1, 7);
    assert!(i.add_item_stack_to_inventory(&mut s));
    assert_eq!(s.stack_size, 0);
    let m0 = i.main_inventory[0].unwrap();
    assert_eq!((m0.item_damage, m0.animations_to_go), (7, 5));
    for slot in i.main_inventory.iter_mut() {
        *slot = Some(ItemStack::new(STICK, 64, 0));
    }
    let mut s = ItemStack::new(PICKAXE_STEEL, 1, 7);
    assert!(!i.add_item_stack_to_inventory(&mut s));
    assert_eq!(s.stack_size, 1);
    // A full inventory refuses an undamaged stack too.
    let mut s = ItemStack::new(COBBLESTONE, 5, 0);
    assert!(!i.add_item_stack_to_inventory(&mut s));
    assert_eq!(s.stack_size, 5);
}

#[test]
fn slot_queries_and_hotbar_selection() {
    let mut i = InventoryPlayer::default();
    i.main_inventory[12] = Some(ItemStack::new(STICK, 2, 0));
    i.main_inventory[4] = Some(ItemStack::new(STICK, 1, 0));
    assert_eq!(i.get_inventory_slot_contain_item(STICK), 4);
    // consumeInventoryItem: the first slot holding the id; it empties at 0.
    assert!(i.consume_inventory_item(STICK));
    assert_eq!(i.main_inventory[4], None);
    assert!(i.consume_inventory_item(STICK));
    assert_eq!(i.main_inventory[12].unwrap().stack_size, 1);
    assert!(!i.consume_inventory_item(COBBLESTONE));
    // setCurrentItem only selects a hotbar slot.
    i.set_current_item(STICK, false);
    assert_eq!(i.current_item, 0);
    i.main_inventory[7] = Some(ItemStack::new(PLANKS, 1, 0));
    i.set_current_item(PLANKS, false);
    assert_eq!(i.current_item, 7);
    // changeCurrentItem: one slot against the wheel's sign, wrapping.
    i.change_current_item(-5);
    assert_eq!(i.current_item, 8);
    i.change_current_item(3);
    assert_eq!(i.current_item, 7);
    i.current_item = 0;
    i.change_current_item(1);
    assert_eq!(i.current_item, 8);
    i.change_current_item(-1);
    assert_eq!(i.current_item, 0);
    // decrStackSize / setInventorySlotContents reach the armour at 36..40.
    i.set_inventory_slot_contents(39, Some(ItemStack::new(HELMET_DIAMOND, 1, 0)));
    assert_eq!(i.armor_inventory[3].unwrap().item_id, HELMET_DIAMOND);
    assert_eq!(i.decr_stack_size(39, 1).unwrap().item_id, HELMET_DIAMOND);
    assert_eq!(i.armor_inventory[3], None);
    i.main_inventory[0] = Some(ItemStack::new(COBBLESTONE, 10, 0));
    assert_eq!(i.decr_stack_size(0, 4).unwrap().stack_size, 4);
    assert_eq!(i.main_inventory[0].unwrap().stack_size, 6);
    assert!(i.has_item_stack(&ItemStack::new(COBBLESTONE, 6, 0)));
    assert!(!i.has_item_stack(&ItemStack::new(COBBLESTONE, 5, 0)));
    assert_eq!(i.get_size_inventory(), 40);
    assert_eq!(i.get_inv_name(), "Inventory");
}

/// `getTotalArmorValue`: `(sum of damageReduceAmount − 1) * durability left
/// / max durability + 1` over the worn `ItemArmor` (InventoryPlayer.java:284-306).
#[test]
fn total_armor_value_scales_with_durability() {
    let mut i = InventoryPlayer::default();
    assert_eq!(i.get_total_armor_value(), 0);
    // Diamond helmet: reduce 3, max 11*3 << 3 = 264; iron plate: reduce 8,
    // max 16*3 << 2 = 192 (ItemArmor.java:4-17).
    i.armor_inventory[3] = Some(ItemStack::new(HELMET_DIAMOND, 1, 0));
    i.armor_inventory[2] = Some(ItemStack::new(PLATE_STEEL, 1, 92));
    // (11 - 1) * (264 + 100) / (264 + 192) + 1 = 3640 / 456 + 1 = 8
    assert_eq!(i.get_total_armor_value(), 8);
    // A pumpkin is worn but not ItemArmor.
    i.armor_inventory[3] = Some(ItemStack::new(PUMPKIN, 1, 0));
    assert_eq!(i.get_total_armor_value(), (8 - 1) * 100 / 192 + 1);
}

#[test]
fn armor_wears_out_and_the_owner_drops_everything() {
    let mut w = empty_world(1);
    let mut p = new_entity_player_sp(&mut w);
    inv_mut(&mut p).armor_inventory[3] = Some(ItemStack::new(HELMET_DIAMOND, 1, 263));
    inv_mut(&mut p).armor_inventory[0] = Some(ItemStack::new(PUMPKIN, 1, 0));
    damage_armor(&mut w, &mut p, 1);
    assert_eq!(inv(&p).armor_inventory[3].unwrap().item_damage, 264);
    assert_eq!(hits(&w, "EntityPlayer.addStat"), 0);
    damage_armor(&mut w, &mut p, 1);
    // Broken: objectBreakStats (recorded) and the slot emptied; the pumpkin is no ItemArmor.
    assert_eq!(inv(&p).armor_inventory[3], None);
    assert_eq!(hits(&w, "EntityPlayer.addStat"), 1);
    assert!(inv(&p).armor_inventory[0].is_some());

    inv_mut(&mut p).main_inventory[5] = Some(ItemStack::new(STICK, 3, 0));
    inv_mut(&mut p).main_inventory[5]
        .as_mut()
        .unwrap()
        .animations_to_go = 2;
    decrement_animations(&mut w, &mut p);
    assert_eq!(inv(&p).main_inventory[5].unwrap().animations_to_go, 1);

    drop_all_items(&mut w, &mut p);
    assert!(inv(&p).main_inventory.iter().all(Option::is_none));
    assert!(inv(&p).armor_inventory.iter().all(Option::is_none));
    assert_eq!(
        dropped(&w).len(),
        2,
        "one per stack (the stick, the pumpkin)"
    );

    let mut other = new_entity_player_sp(&mut w);
    assert!(can_interact_with(&p, &other));
    other.base.pos_x += 9.0;
    assert!(!can_interact_with(&p, &other));
    p.base.is_dead = true;
    assert!(!can_interact_with(&p, &p.clone()));
}

// ---- containers ------------------------------------------------------------------------

/// Slot −999: the left button drops the held stack, the right button one
/// item of it (Container.java:51-64), each through `dropPlayerItem`.
#[test]
fn clicking_outside_the_window_drops() {
    let mut w = empty_world(2);
    let mut p = new_entity_player_sp(&mut w);
    let mut c = new_container_player(true);
    inv_mut(&mut p).item_stack = Some(ItemStack::new(COBBLESTONE, 3, 0));
    assert_eq!(c.slot_click(&mut w, &mut p, -999, 1, false), None);
    assert_eq!(inv(&p).item_stack.unwrap().stack_size, 2);
    c.slot_click(&mut w, &mut p, -999, 1, false);
    c.slot_click(&mut w, &mut p, -999, 1, false);
    assert_eq!(inv(&p).item_stack, None, "emptied at size 0");
    inv_mut(&mut p).item_stack = Some(ItemStack::new(COBBLESTONE, 3, 0));
    c.slot_click(&mut w, &mut p, -999, 0, false);
    assert_eq!(inv(&p).item_stack, None);
    assert_eq!(
        dropped(&w),
        [1, 1, 1, 3].map(|n| ItemStack::new(COBBLESTONE, n, 0))
    );
    // Other buttons do nothing.
    inv_mut(&mut p).item_stack = Some(ItemStack::new(COBBLESTONE, 3, 0));
    c.slot_click(&mut w, &mut p, -999, 2, false);
    assert_eq!(inv(&p).item_stack.unwrap().stack_size, 3);
}

/// `onCraftGuiClosed`: the held stack and (player container) the grid are
/// dropped and the grid cleared, which recomputes the result
/// (ContainerPlayer.java:48-59); the workbench drops its grid but keeps it
/// (ContainerWorkbench.java:43-54).
#[test]
fn closing_drops_the_grid() {
    let mut w = empty_world(3);
    let mut p = new_entity_player_sp(&mut w);
    let mut c = new_container_player(true);
    c.put_stack_in_slot(&mut w, &mut p, 1, Some(ItemStack::new(17, 1, 0)));
    assert_eq!(
        c.craft_result().get_stack_in_slot(0).unwrap().item_id,
        PLANKS
    );
    inv_mut(&mut p).item_stack = Some(ItemStack::new(STICK, 1, 0));
    c.on_craft_gui_closed(&mut w, &mut p);
    assert_eq!(inv(&p).item_stack, None);
    assert_eq!(c.craft_matrix().get_stack_in_slot(0), None);
    assert_eq!(c.craft_result().get_stack_in_slot(0), None);
    assert_eq!(dropped(&w).len(), 2);

    let mut wb = new_container_workbench(0, 61, 0);
    wb.put_stack_in_slot(&mut w, &mut p, 5, Some(ItemStack::new(17, 1, 0)));
    wb.on_craft_gui_closed(&mut w, &mut p);
    assert_eq!(dropped(&w).len(), 3);
    assert!(
        wb.craft_matrix().get_stack_in_slot(4).is_some(),
        "not cleared"
    );
}

#[test]
fn usable_by_player() {
    let mut w = empty_world(4);
    let p = new_entity_player_sp(&mut w);
    // The player stands at the spawn point + (0.5, 1, 0.5) = (0.5, 1, 0.5).
    let wb = new_container_workbench(1, 1, 1);
    assert!(!wb.is_usable_by_player(&mut w, &p), "no workbench there");
    w.set_block(1, 1, 1, 58);
    assert!(wb.is_usable_by_player(&mut w, &p));
    let far = new_container_workbench(9, 1, 0);
    w.set_block(9, 1, 0, 58);
    assert!(!far.is_usable_by_player(&mut w, &p), "8.5 blocks away");
    assert!(new_container_player(true).is_usable_by_player(&mut w, &p));
}

/// `SlotArmor.isItemValid`: armour of the slot's type, or a pumpkin in the
/// helmet slot (SlotArmor.java:19-27).
#[test]
fn armor_slots_take_a_pumpkin_as_a_helmet() {
    let c = new_container_player(true);
    let pumpkin = ItemStack::new(PUMPKIN, 1, 0);
    let helmet = ItemStack::new(HELMET_DIAMOND, 1, 0);
    assert!(c.get_slot(5).is_item_valid(&pumpkin));
    assert!(!c.get_slot(6).is_item_valid(&pumpkin));
    assert!(c.get_slot(5).is_item_valid(&helmet));
    assert!(!c.get_slot(8).is_item_valid(&helmet));
    assert!(!c.get_slot(0).is_item_valid(&helmet), "the output");
    assert_eq!(c.slot_get_slot_stack_limit(5), 1);
    assert_eq!(c.slot_get_slot_stack_limit(9), 64);
    assert_eq!(c.slots.len(), 45);
    assert_eq!(c.get_slot(5).slot_index, 39);
    assert_eq!(c.get_slot(36).slot_index, 0);
}

/// A map from the workbench: `ItemMap.onCreated` gives the cursor's map a
/// fresh id and a `MapData` centred on the player (ItemMap.java:228-238);
/// the crafting stat is recorded (WP-2.7).
#[test]
fn crafting_a_map_runs_on_created() {
    let mut w = empty_world(5);
    let mut p = new_entity_player_sp(&mut w);
    let mut c = new_container_workbench(0, 0, 0);
    for k in 1..=9 {
        let s = if k == 5 { (345, 1) } else { (339, 1) };
        c.put_stack_in_slot(&mut w, &mut p, k, Some(ItemStack::new(s.0, s.1, 0)));
    }
    assert_eq!(c.craft_result().get_stack_in_slot(0).unwrap().item_id, 358);
    w.get_unique_data_id("map");
    c.slot_click(&mut w, &mut p, 0, 0, false);
    let held = inv(&p).item_stack.unwrap();
    assert_eq!((held.item_id, held.item_damage), (358, 1));
    assert!(w.load_item_data("map_1").is_some());
    assert_eq!(hits(&w, "ItemMap.onCreated"), 0);
    assert_eq!(hits(&w, "EntityPlayer.addStat"), 1);
    assert!((1..=9).all(|k| c.slot_get_stack(&mut w, &mut p, k).is_none()));
}

#[test]
fn crafting_results_sync_and_furnace_progress() {
    let mut w = empty_world(6);
    let mut p = new_entity_player_sp(&mut w);
    let mut c = new_container_player(true);
    inv_mut(&mut p).main_inventory[0] = Some(ItemStack::new(STICK, 2, 0));
    c.update_crafting_results(&mut w, &mut p);
    assert_eq!(
        c.inventory_item_stacks[36],
        Some(ItemStack::new(STICK, 2, 0))
    );
    assert_eq!(c.get_next_transaction_id(), 1);
    assert_eq!(c.get_next_transaction_id(), 2);

    w.set_block(0, 1, 0, 61);
    let r = w
        .get_block_tile_entity(0, 1, 0)
        .expect("furnace tile entity");
    let mut f = new_container_furnace(r);
    f.update_progress_bar(&mut w, 0, 77);
    f.update_progress_bar(&mut w, 2, 1600);
    let TileEntityKindData::Furnace(d) = &w.tile_entities[&r].kind else {
        panic!("a furnace");
    };
    assert_eq!((d.furnace_cook_time, d.current_item_burn_time), (77, 1600));
    f.update_crafting_results(&mut w, &mut p);
    let ContainerKind::Furnace { cook_time, .. } = f.kind else {
        panic!()
    };
    assert_eq!(cook_time, 77);
    assert_eq!(f.get_slot(2).inventory, InventoryRef::TileEntity(r));
}

/// `InventoryBasic`: a stack over the limit is cut down; `decrStackSize`
/// splits or takes (InventoryBasic.java:21-50).
#[test]
fn inventory_basic() {
    let mut b = InventoryBasic::new("Trap", 3);
    b.set_inventory_slot_contents(1, Some(ItemStack::new(COBBLESTONE, 70, 0)));
    assert_eq!(b.get_stack_in_slot(1).unwrap().stack_size, 64);
    assert_eq!(b.decr_stack_size(1, 10).unwrap().stack_size, 10);
    assert_eq!(b.decr_stack_size(1, 60).unwrap().stack_size, 54);
    assert_eq!(b.get_stack_in_slot(1), None);
    assert_eq!(b.decr_stack_size(0, 1), None);
    assert_eq!((b.get_size_inventory(), b.get_inv_name()), (3, "Trap"));
}

// ---- crafting --------------------------------------------------------------------------

/// `RecipeSorter`: shaped before shapeless, then the larger recipe first
/// (RecipeSorter.java:13-23).
#[test]
fn recipe_sorter_order() {
    let shaped = |w, h| {
        Recipe::Shaped(ShapedRecipes::new(
            w,
            h,
            vec![None; (w * h) as usize],
            ItemStack::new(1, 1, 0),
        ))
    };
    let shapeless = |n| {
        Recipe::Shapeless(ShapelessRecipes::new(
            ItemStack::new(1, 1, 0),
            vec![ItemStack::new(1, 1, 0); n],
        ))
    };
    assert_eq!(
        compare_recipes(&shapeless(9), &shaped(1, 1)),
        Ordering::Greater
    );
    assert_eq!(
        compare_recipes(&shaped(1, 1), &shapeless(9)),
        Ordering::Less
    );
    assert_eq!(
        compare_recipes(&shaped(3, 3), &shaped(2, 2)),
        Ordering::Less
    );
    assert_eq!(
        compare_recipes(&shaped(1, 2), &shaped(2, 1)),
        Ordering::Equal
    );
    assert_eq!(
        compare_recipes(&shapeless(2), &shapeless(3)),
        Ordering::Greater
    );
    let list = CraftingManager::get_instance().get_recipe_list();
    let first_shapeless = list
        .iter()
        .position(|r| matches!(r, Recipe::Shapeless(_)))
        .unwrap();
    assert_eq!(
        first_shapeless,
        151 - 31,
        "the 31 dye recipes are the shapeless ones"
    );
    assert!(
        list.windows(2)
            .all(|p| compare_recipes(&p[0], &p[1]) != Ordering::Greater)
    );
}

/// `addRecipe`: a repeated key keeps its last ingredient (`HashMap.put`), a
/// `Block` matches any damage; `addShapelessRecipe`'s `Block` is damage 0
/// (CraftingManager.java:85-158).
#[test]
fn add_recipe_key_map_and_block_damage() {
    let mut m = CraftingManager::get_instance().clone();
    let before = m.get_recipe_list().len();
    m.add_recipe(
        ItemStack::new(1, 1, 0),
        &["#"],
        &[
            ('#', Ingredient::Item(STICK)),
            ('#', Ingredient::Block(PLANKS)),
        ],
    );
    m.add_shapeless_recipe(ItemStack::new(2, 1, 0), &[Ingredient::Block(PLANKS)]);
    let list = m.get_recipe_list();
    let Recipe::Shaped(s) = &list[before] else {
        panic!()
    };
    assert_eq!(s.recipe_items, vec![Some(ItemStack::new(PLANKS, 1, -1))]);
    let Recipe::Shapeless(s) = &list[before + 1] else {
        panic!()
    };
    assert_eq!(s.recipe_items, vec![ItemStack::new(PLANKS, 1, 0)]);
    // A planks cell of damage 3: the shaped wildcard matches, the shapeless one does not.
    let mut g = InventoryCrafting::new(3, 3);
    g.set_inventory_slot_contents(4, Some(ItemStack::new(PLANKS, 1, 3)));
    assert!(list[before].matches(&g));
    assert!(!list[before + 1].matches(&g));
}
