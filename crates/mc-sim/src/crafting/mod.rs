//! Crafting recipes (`CraftingManager`, `IRecipe`, `ShapedRecipes`,
//! `ShapelessRecipes`, `RecipeSorter` and the seven `Recipes*` classes;
//! WP-2.5).
//!
//! | Java | Rust |
//! |---|---|
//! | `CraftingManager` (singleton, `getInstance()`) | [`CraftingManager`], [`CraftingManager::get_instance`] |
//! | `IRecipe` | [`Recipe`] (closed enum over the two implementations) |
//! | `ShapedRecipes` | [`shaped::ShapedRecipes`] |
//! | `ShapelessRecipes` | [`shapeless::ShapelessRecipes`] |
//! | `RecipeSorter` | [`manager::compare_recipes`] |
//! | `RecipesTools`, `RecipesWeapons`, `RecipesIngots`, `RecipesFood`, `RecipesCrafting`, `RecipesArmor`, `RecipesDyes` | `recipes_*` modules, each an `add_recipes(&mut CraftingManager)` |
//! | the `Object...` arguments of `addRecipe` / `addShapelessRecipe` | [`Ingredient`] |
//!
//! The recipe list is built once, in registration order, then sorted by
//! `RecipeSorter` (stable; shaped before shapeless, larger before smaller),
//! exactly as the `CraftingManager` constructor does; 151 recipes. The
//! oracle dumps it (`tests/golden/recipe/recipe-table`, Tier A kind
//! `recipe_table`).
//!
//! `FurnaceRecipes` is data next to the furnace
//! ([`crate::tile_entity::furnace::SMELTING`], WP-1.8; verified against
//! FurnaceRecipes.java:14-25 for this package).

pub mod manager;
mod recipes_armor;
mod recipes_crafting;
mod recipes_dyes;
mod recipes_food;
mod recipes_ingots;
mod recipes_tools;
mod recipes_weapons;
pub mod shaped;
pub mod shapeless;

pub use manager::{CraftingManager, Ingredient};

use crate::inventory::crafting::InventoryCrafting;
use crate::item::ItemStack;

/// `IRecipe`: one of the two implementations.
#[derive(Clone, Debug, PartialEq)]
pub enum Recipe {
    /// `ShapedRecipes`.
    Shaped(shaped::ShapedRecipes),
    /// `ShapelessRecipes`.
    Shapeless(shapeless::ShapelessRecipes),
}

impl Recipe {
    /// `matches(inventoryCrafting)`.
    // src: IRecipe.java:4
    pub fn matches(&self, inv: &InventoryCrafting) -> bool {
        match self {
            Recipe::Shaped(r) => r.matches(inv),
            Recipe::Shapeless(r) => r.matches(inv),
        }
    }

    /// `getCraftingResult(inventoryCrafting)`: a fresh stack.
    // src: IRecipe.java:6
    pub fn get_crafting_result(&self, inv: &InventoryCrafting) -> ItemStack {
        match self {
            Recipe::Shaped(r) => r.get_crafting_result(inv),
            Recipe::Shapeless(r) => r.get_crafting_result(inv),
        }
    }

    /// `getRecipeSize()`: the grid area of a shaped recipe, the ingredient
    /// count of a shapeless one (the sort key of `RecipeSorter`).
    // src: IRecipe.java:8
    pub fn get_recipe_size(&self) -> i32 {
        match self {
            Recipe::Shaped(r) => r.get_recipe_size(),
            Recipe::Shapeless(r) => r.get_recipe_size(),
        }
    }

    /// `getRecipeOutput()`: the stored output stack (not a copy in Java).
    // src: IRecipe.java:10
    pub fn get_recipe_output(&self) -> &ItemStack {
        match self {
            Recipe::Shaped(r) => r.get_recipe_output(),
            Recipe::Shapeless(r) => r.get_recipe_output(),
        }
    }
}

/// The `Block.<field>.blockID` and `Item.<field>.shiftedIndex` values the
/// recipe classes name. Blocks: `blockID` is the constructor's id, declared
/// at Block.java:(592 + id) (`cloth` 35 at :627 through its class).
/// Items: `shiftedIndex` is `256 + ` the constructor's index, declared at
/// Item.java:(274 + index) for indices 0–103.
#[allow(missing_docs)]
pub mod ids {
    // ---- blocks (Block.java:593-688) -------------------------------------------
    pub const STONE: i32 = 1; // Block.java:593
    pub const COBBLESTONE: i32 = 4; // Block.java:596
    pub const PLANKS: i32 = 5; // Block.java:597
    pub const SAND: i32 = 12; // Block.java:604
    pub const WOOD: i32 = 17; // Block.java:609
    pub const BLOCK_LAPIS: i32 = 22; // Block.java:614
    pub const DISPENSER: i32 = 23; // Block.java:615
    pub const SAND_STONE: i32 = 24; // Block.java:616
    pub const MUSIC_BLOCK: i32 = 25; // Block.java:617
    pub const RAIL_POWERED: i32 = 27; // Block.java:619
    pub const RAIL_DETECTOR: i32 = 28; // Block.java:620
    pub const PISTON_STICKY_BASE: i32 = 29; // Block.java:621
    pub const PISTON_BASE: i32 = 33; // Block.java:625
    pub const CLOTH: i32 = 35; // Block.java:627 (BlockCloth's constructor passes 35)
    pub const PLANT_YELLOW: i32 = 37; // Block.java:629
    pub const PLANT_RED: i32 = 38; // Block.java:630
    pub const MUSHROOM_BROWN: i32 = 39; // Block.java:631
    pub const MUSHROOM_RED: i32 = 40; // Block.java:632
    pub const BLOCK_GOLD: i32 = 41; // Block.java:633
    pub const BLOCK_STEEL: i32 = 42; // Block.java:634
    pub const STAIR_SINGLE: i32 = 44; // Block.java:636
    pub const BRICK: i32 = 45; // Block.java:637
    pub const TNT: i32 = 46; // Block.java:638
    pub const BOOK_SHELF: i32 = 47; // Block.java:639
    pub const TORCH_WOOD: i32 = 50; // Block.java:642
    pub const FIRE: i32 = 51; // Block.java:643
    pub const STAIR_COMPACT_PLANKS: i32 = 53; // Block.java:645
    pub const CHEST: i32 = 54; // Block.java:646
    pub const BLOCK_DIAMOND: i32 = 57; // Block.java:649
    pub const WORKBENCH: i32 = 58; // Block.java:650
    pub const STONE_OVEN_IDLE: i32 = 61; // Block.java:653
    pub const LADDER: i32 = 65; // Block.java:657
    pub const RAIL: i32 = 66; // Block.java:658
    pub const STAIR_COMPACT_COBBLESTONE: i32 = 67; // Block.java:659
    pub const LEVER: i32 = 69; // Block.java:661
    pub const PRESSURE_PLATE_STONE: i32 = 70; // Block.java:662
    pub const PRESSURE_PLATE_PLANKS: i32 = 72; // Block.java:664
    pub const TORCH_REDSTONE_ACTIVE: i32 = 76; // Block.java:668
    pub const BUTTON: i32 = 77; // Block.java:669
    pub const BLOCK_SNOW: i32 = 80; // Block.java:672
    pub const BLOCK_CLAY: i32 = 82; // Block.java:674
    pub const JUKEBOX: i32 = 84; // Block.java:676
    pub const FENCE: i32 = 85; // Block.java:677
    pub const PUMPKIN: i32 = 86; // Block.java:678
    pub const GLOW_STONE: i32 = 89; // Block.java:681
    pub const PUMPKIN_LANTERN: i32 = 91; // Block.java:683
    pub const TRAPDOOR: i32 = 96; // Block.java:688

    // ---- items (Item.java:274-377) ---------------------------------------------
    pub const SHOVEL_STEEL: i32 = 256; // Item.java:274
    pub const PICKAXE_STEEL: i32 = 257; // Item.java:275
    pub const AXE_STEEL: i32 = 258; // Item.java:276
    pub const FLINT_AND_STEEL: i32 = 259; // Item.java:277
    pub const APPLE_RED: i32 = 260; // Item.java:278
    pub const BOW: i32 = 261; // Item.java:279
    pub const ARROW: i32 = 262; // Item.java:280
    pub const COAL: i32 = 263; // Item.java:281
    pub const DIAMOND: i32 = 264; // Item.java:282
    pub const INGOT_IRON: i32 = 265; // Item.java:283
    pub const INGOT_GOLD: i32 = 266; // Item.java:284
    pub const SWORD_STEEL: i32 = 267; // Item.java:285
    pub const SWORD_WOOD: i32 = 268; // Item.java:286
    pub const SHOVEL_WOOD: i32 = 269; // Item.java:287
    pub const PICKAXE_WOOD: i32 = 270; // Item.java:288
    pub const AXE_WOOD: i32 = 271; // Item.java:289
    pub const SWORD_STONE: i32 = 272; // Item.java:290
    pub const SHOVEL_STONE: i32 = 273; // Item.java:291
    pub const PICKAXE_STONE: i32 = 274; // Item.java:292
    pub const AXE_STONE: i32 = 275; // Item.java:293
    pub const SWORD_DIAMOND: i32 = 276; // Item.java:294
    pub const SHOVEL_DIAMOND: i32 = 277; // Item.java:295
    pub const PICKAXE_DIAMOND: i32 = 278; // Item.java:296
    pub const AXE_DIAMOND: i32 = 279; // Item.java:297
    pub const STICK: i32 = 280; // Item.java:298
    pub const BOWL_EMPTY: i32 = 281; // Item.java:299
    pub const BOWL_SOUP: i32 = 282; // Item.java:300
    pub const SWORD_GOLD: i32 = 283; // Item.java:301
    pub const SHOVEL_GOLD: i32 = 284; // Item.java:302
    pub const PICKAXE_GOLD: i32 = 285; // Item.java:303
    pub const AXE_GOLD: i32 = 286; // Item.java:304
    pub const SILK: i32 = 287; // Item.java:305
    pub const FEATHER: i32 = 288; // Item.java:306
    pub const GUNPOWDER: i32 = 289; // Item.java:307
    pub const HOE_WOOD: i32 = 290; // Item.java:308
    pub const HOE_STONE: i32 = 291; // Item.java:309
    pub const HOE_STEEL: i32 = 292; // Item.java:310
    pub const HOE_DIAMOND: i32 = 293; // Item.java:311
    pub const HOE_GOLD: i32 = 294; // Item.java:312
    pub const WHEAT: i32 = 296; // Item.java:314
    pub const BREAD: i32 = 297; // Item.java:315
    pub const HELMET_LEATHER: i32 = 298; // Item.java:316
    pub const PLATE_LEATHER: i32 = 299; // Item.java:317
    pub const LEGS_LEATHER: i32 = 300; // Item.java:318
    pub const BOOTS_LEATHER: i32 = 301; // Item.java:319
    pub const HELMET_CHAIN: i32 = 302; // Item.java:320
    pub const PLATE_CHAIN: i32 = 303; // Item.java:321
    pub const LEGS_CHAIN: i32 = 304; // Item.java:322
    pub const BOOTS_CHAIN: i32 = 305; // Item.java:323
    pub const HELMET_STEEL: i32 = 306; // Item.java:324
    pub const PLATE_STEEL: i32 = 307; // Item.java:325
    pub const LEGS_STEEL: i32 = 308; // Item.java:326
    pub const BOOTS_STEEL: i32 = 309; // Item.java:327
    pub const HELMET_DIAMOND: i32 = 310; // Item.java:328
    pub const PLATE_DIAMOND: i32 = 311; // Item.java:329
    pub const LEGS_DIAMOND: i32 = 312; // Item.java:330
    pub const BOOTS_DIAMOND: i32 = 313; // Item.java:331
    pub const HELMET_GOLD: i32 = 314; // Item.java:332
    pub const PLATE_GOLD: i32 = 315; // Item.java:333
    pub const LEGS_GOLD: i32 = 316; // Item.java:334
    pub const BOOTS_GOLD: i32 = 317; // Item.java:335
    pub const FLINT: i32 = 318; // Item.java:336
    pub const PAINTING: i32 = 321; // Item.java:339
    pub const APPLE_GOLD: i32 = 322; // Item.java:340
    pub const SIGN: i32 = 323; // Item.java:341
    pub const DOOR_WOOD: i32 = 324; // Item.java:342
    pub const BUCKET_EMPTY: i32 = 325; // Item.java:343
    pub const MINECART_EMPTY: i32 = 328; // Item.java:346
    pub const DOOR_STEEL: i32 = 330; // Item.java:348
    pub const REDSTONE: i32 = 331; // Item.java:349
    pub const SNOWBALL: i32 = 332; // Item.java:350
    pub const BOAT: i32 = 333; // Item.java:351
    pub const LEATHER: i32 = 334; // Item.java:352
    pub const BUCKET_MILK: i32 = 335; // Item.java:353
    pub const BRICK_ITEM: i32 = 336; // Item.java:354 (`Item.brick`)
    pub const CLAY: i32 = 337; // Item.java:355
    pub const REED: i32 = 338; // Item.java:356
    pub const PAPER: i32 = 339; // Item.java:357
    pub const BOOK: i32 = 340; // Item.java:358
    pub const SLIME_BALL: i32 = 341; // Item.java:359
    pub const MINECART_CRATE: i32 = 342; // Item.java:360
    pub const MINECART_POWERED: i32 = 343; // Item.java:361
    pub const EGG: i32 = 344; // Item.java:362
    pub const COMPASS: i32 = 345; // Item.java:363
    pub const FISHING_ROD: i32 = 346; // Item.java:364
    pub const POCKET_SUNDIAL: i32 = 347; // Item.java:365
    pub const LIGHT_STONE_DUST: i32 = 348; // Item.java:366
    pub const FISH_COOKED: i32 = 350; // Item.java:368
    pub const DYE_POWDER: i32 = 351; // Item.java:369
    pub const BONE: i32 = 352; // Item.java:370
    pub const SUGAR: i32 = 353; // Item.java:371
    pub const CAKE: i32 = 354; // Item.java:372
    pub const BED: i32 = 355; // Item.java:373
    pub const REDSTONE_REPEATER: i32 = 356; // Item.java:374
    pub const COOKIE: i32 = 357; // Item.java:375
    pub const MAP_ITEM: i32 = 358; // Item.java:376
    pub const SHEARS: i32 = 359; // Item.java:377
}

#[cfg(test)]
mod tests {
    use super::ids::*;
    use crate::item::item;

    /// Every item constant is the `shiftedIndex` of an item with that name.
    #[test]
    fn item_ids_name_the_right_items() {
        for (id, name) in [
            (SHOVEL_STEEL, "shovelIron"),
            (STICK, "stick"),
            (INGOT_IRON, "ingotIron"),
            (BRICK_ITEM, "brick"),
            (DYE_POWDER, "dyePowder"),
            (MAP_ITEM, "map"),
            (SHEARS, "shears"),
            (COOKIE, "cookie"),
            (HELMET_CHAIN, "helmetChain"),
            (BUCKET_MILK, "milk"),
        ] {
            assert_eq!(item(id).and_then(|d| d.item_name), Some(name), "item {id}");
        }
    }

    #[test]
    fn block_ids_name_the_right_blocks() {
        for (id, name) in [
            (CLOTH, "cloth"),
            (WORKBENCH, "workbench"),
            (STONE_OVEN_IDLE, "furnace"),
            (TRAPDOOR, "trapdoor"),
            (PUMPKIN_LANTERN, "litpumpkin"),
            (BLOCK_LAPIS, "blockLapis"),
        ] {
            assert_eq!(
                crate::block::block(id).and_then(|b| b.block_name),
                Some(name),
                "block {id}"
            );
        }
    }
}
