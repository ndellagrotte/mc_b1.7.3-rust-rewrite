//! `AchievementList` (AchievementList.java): the sixteen achievements in
//! registration order. Their registration runs inside `StatList`'s static
//! initialiser (`AchievementList.func_27374_a()`, StatList.java:194), so
//! [`super::StatList`] builds them through [`register_achievements`].
//!
//! The `System.out.println(size + " achievements")` (AchievementList.java:49)
//! is left out.

use super::achievement::{ACHIEVEMENT_ID_BASE, Achievement, set_special};
use super::stat_base::StatBase;
use crate::item::ItemStack;

/// `openInventory`'s `statId`.
pub const OPEN_INVENTORY: i32 = ACHIEVEMENT_ID_BASE;
/// `mineWood`'s `statId`.
pub const MINE_WOOD: i32 = ACHIEVEMENT_ID_BASE + 1;
/// `buildWorkBench`'s `statId`.
pub const BUILD_WORK_BENCH: i32 = ACHIEVEMENT_ID_BASE + 2;
/// `buildPickaxe`'s `statId`.
pub const BUILD_PICKAXE: i32 = ACHIEVEMENT_ID_BASE + 3;
/// `buildFurnace`'s `statId`.
pub const BUILD_FURNACE: i32 = ACHIEVEMENT_ID_BASE + 4;
/// `acquireIron`'s `statId`.
pub const ACQUIRE_IRON: i32 = ACHIEVEMENT_ID_BASE + 5;
/// `buildHoe`'s `statId`.
pub const BUILD_HOE: i32 = ACHIEVEMENT_ID_BASE + 6;
/// `makeBread`'s `statId`.
pub const MAKE_BREAD: i32 = ACHIEVEMENT_ID_BASE + 7;
/// `bakeCake`'s `statId`.
pub const BAKE_CAKE: i32 = ACHIEVEMENT_ID_BASE + 8;
/// `buildBetterPickaxe`'s `statId`.
pub const BUILD_BETTER_PICKAXE: i32 = ACHIEVEMENT_ID_BASE + 9;
/// `cookFish`'s `statId`.
pub const COOK_FISH: i32 = ACHIEVEMENT_ID_BASE + 10;
/// `onARail`'s `statId`.
pub const ON_A_RAIL: i32 = ACHIEVEMENT_ID_BASE + 11;
/// `buildSword`'s `statId`.
pub const BUILD_SWORD: i32 = ACHIEVEMENT_ID_BASE + 12;
/// `killEnemy`'s `statId`.
pub const KILL_ENEMY: i32 = ACHIEVEMENT_ID_BASE + 13;
/// `killCow`'s `statId`.
pub const KILL_COW: i32 = ACHIEVEMENT_ID_BASE + 14;
/// `flyPig`'s `statId`.
pub const FLY_PIG: i32 = ACHIEVEMENT_ID_BASE + 15;

/// `new ItemStack(item)` / `new ItemStack(block)`: count 1, damage 0.
// src: ItemStack.java:13-23
const fn icon(id: i32) -> ItemStack {
    ItemStack::new(id, 1, 0)
}

/// The sixteen `Achievement` constructions of AchievementList's static
/// initialiser, in order, each followed by its `setIndependent()` /
/// `setSpecial()`; `register` is `registerAchievement()` (StatBase's
/// `registerStat()`, then `achievementList.add`, Achievement.java:55-59).
/// Icon ids: `Item.x.shiftedIndex` (`256 +` the constructor id) or
/// `Block.x.blockID` as cited.
// src: AchievementList.java:33-48
pub(super) fn register_achievements(register: &mut dyn FnMut(StatBase)) {
    let a = Achievement::new_stat;
    // Item.book (Item.java:358: new Item(84)), no parent, independent.
    register(a(0, "openInventory", 0, 0, icon(256 + 84), None).init_independent_stat());
    // Block.wood (Block.java:609, id 17).
    register(a(1, "mineWood", 2, 1, icon(17), Some(OPEN_INVENTORY)));
    // Block.workbench (Block.java:650, id 58).
    register(a(2, "buildWorkBench", 4, -1, icon(58), Some(MINE_WOOD)));
    // Item.pickaxeWood (Item.java:288: ItemPickaxe(14)).
    register(a(
        3,
        "buildPickaxe",
        4,
        2,
        icon(256 + 14),
        Some(BUILD_WORK_BENCH),
    ));
    // Block.stoneOvenActive (Block.java:654, id 62).
    register(a(4, "buildFurnace", 3, 4, icon(62), Some(BUILD_PICKAXE)));
    // Item.ingotIron (Item.java:283: new Item(9)).
    register(a(
        5,
        "acquireIron",
        1,
        4,
        icon(256 + 9),
        Some(BUILD_FURNACE),
    ));
    // Item.hoeWood (Item.java:308: ItemHoe(34)).
    register(a(
        6,
        "buildHoe",
        2,
        -3,
        icon(256 + 34),
        Some(BUILD_WORK_BENCH),
    ));
    // Item.bread (Item.java:315: ItemFood(41, ...)).
    register(a(7, "makeBread", -1, -3, icon(256 + 41), Some(BUILD_HOE)));
    // Item.cake (Item.java:372: ItemReed(98, Block.cake)).
    register(a(8, "bakeCake", 0, -5, icon(256 + 98), Some(BUILD_HOE)));
    // Item.pickaxeStone (Item.java:292: ItemPickaxe(18)).
    register(a(
        9,
        "buildBetterPickaxe",
        6,
        2,
        icon(256 + 18),
        Some(BUILD_PICKAXE),
    ));
    // Item.fishCooked (Item.java:368: ItemFood(94, ...)).
    register(a(10, "cookFish", 2, 6, icon(256 + 94), Some(BUILD_FURNACE)));
    // Block.rail (Block.java:658, id 66), special.
    register(set_special(a(
        11,
        "onARail",
        2,
        3,
        icon(66),
        Some(ACQUIRE_IRON),
    )));
    // Item.swordWood (Item.java:286: ItemSword(12)).
    register(a(
        12,
        "buildSword",
        6,
        -1,
        icon(256 + 12),
        Some(BUILD_WORK_BENCH),
    ));
    // Item.bone (Item.java:370: new Item(96)).
    register(a(13, "killEnemy", 8, -1, icon(256 + 96), Some(BUILD_SWORD)));
    // Item.leather (Item.java:352: new Item(78)).
    register(a(14, "killCow", 7, -3, icon(256 + 78), Some(BUILD_SWORD)));
    // Item.saddle (Item.java:347: ItemSaddle(73)), special.
    register(set_special(a(
        15,
        "flyPig",
        8,
        -4,
        icon(256 + 73),
        Some(KILL_COW),
    )));
}
