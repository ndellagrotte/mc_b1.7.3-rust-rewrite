//! `StatList` (StatList.java): every stat, registered by vanilla's static
//! initialisers in a fixed order, the per-block and per-item stat arrays,
//! and the id map.
//!
//! # Initialisation order
//!
//! `StatList`'s static state is filled by three entry points that the
//! `Block` and `Item` static initialisers reach in this order (the client's
//! first class to initialise is `Block`; `Item` first would fail, because
//! `AchievementList` needs `Item.book` while `Item` is still initialising):
//!
//! 1. Block.java:689 touches `Item`; Item's static initialiser ends with
//!    `StatList.initStats()` (Item.java:380), which first runs StatList's
//!    own static initialiser: the general stats (StatList.java:16-20,
//!    :175-192), the mine-block stats (`initMinableStats`, :193), the
//!    achievements (`AchievementList.func_27374_a()`, :194), and resets
//!    both "initialised" flags (:195-196). Then `initStats`'s body: the use
//!    and break stats of the items, ids 256..32000 (:57-58).
//! 2. Block.java:705: `StatList.initBreakableStats()`: the use and break
//!    stats of the blocks, ids 0..256 (:50-51); both flags are now set, so
//!    `initCraftableStats` registers the craft stats (:63-93).
//!
//! [`StatList::get`] builds the result once ([`std::sync::LazyLock`]), in
//! that order, from the block and item tables, the recipe list and the
//! smelting list.
//!
//! # `replaceAllSimilarBlocks`
//!
//! After each array is filled, eleven block pairs are merged
//! (`replaceSimilarBlocks(a, x, y)`): when only `a[x]` exists it is copied
//! to `a[y]`; otherwise `a[x]` is removed from the three lists (it stays in
//! the id map) and replaced by `a[y]`. So mining grass counts as dirt, and
//! the lit furnace, glowing redstone ore, lit pumpkin and so on count as
//! their plain block. Vanilla bug (transcribed): the lava pair names
//! `lavaStill` twice (StatList.java:147, `lavaMoving` intended), which
//! unlists `lavaStill`'s own use stat.

use std::sync::LazyLock;

use std::collections::BTreeMap;

use mc_core::java_hash::JavaHashSet;

use super::achievement_list::register_achievements;
use super::stat_base::{StatBase, StatKind, StatName, StatType};
use crate::block;
use crate::crafting::CraftingManager;
use crate::item::{self, ITEMS_LIST_LEN};
use crate::tile_entity::furnace::smelting_list;

// ---- the general stats' ids (StatList.java:16-20, :175-192) -------------------------

/// `startGameStat`.
pub const START_GAME_STAT: i32 = 1000;
/// `createWorldStat`.
pub const CREATE_WORLD_STAT: i32 = 1001;
/// `loadWorldStat`.
pub const LOAD_WORLD_STAT: i32 = 1002;
/// `joinMultiplayerStat`.
pub const JOIN_MULTIPLAYER_STAT: i32 = 1003;
/// `leaveGameStat`.
pub const LEAVE_GAME_STAT: i32 = 1004;
/// `minutesPlayedStat` (counts ticks; `StatTypeTime`).
pub const MINUTES_PLAYED_STAT: i32 = 1100;
/// `distanceWalkedStat`.
pub const DISTANCE_WALKED_STAT: i32 = 2000;
/// `distanceSwumStat`.
pub const DISTANCE_SWUM_STAT: i32 = 2001;
/// `distanceFallenStat`.
pub const DISTANCE_FALLEN_STAT: i32 = 2002;
/// `distanceClimbedStat`.
pub const DISTANCE_CLIMBED_STAT: i32 = 2003;
/// `distanceFlownStat`.
pub const DISTANCE_FLOWN_STAT: i32 = 2004;
/// `distanceDoveStat`.
pub const DISTANCE_DOVE_STAT: i32 = 2005;
/// `distanceByMinecartStat`.
pub const DISTANCE_BY_MINECART_STAT: i32 = 2006;
/// `distanceByBoatStat`.
pub const DISTANCE_BY_BOAT_STAT: i32 = 2007;
/// `distanceByPigStat`.
pub const DISTANCE_BY_PIG_STAT: i32 = 2008;
/// `jumpStat`.
pub const JUMP_STAT: i32 = 2010;
/// `dropStat`.
pub const DROP_STAT: i32 = 2011;
/// `damageDealtStat`.
pub const DAMAGE_DEALT_STAT: i32 = 2020;
/// `damageTakenStat`.
pub const DAMAGE_TAKEN_STAT: i32 = 2021;
/// `deathsStat`.
pub const DEATHS_STAT: i32 = 2022;
/// `mobKillsStat`.
pub const MOB_KILLS_STAT: i32 = 2023;
/// `playerKillsStat`.
pub const PLAYER_KILLS_STAT: i32 = 2024;
/// `fishCaughtStat`.
pub const FISH_CAUGHT_STAT: i32 = 2025;

/// `mineBlockStatArray`'s id base: `16777216 + blockId`.
// src: StatList.java:193
pub const MINE_BLOCK_BASE: i32 = 16777216;
/// `objectCraftStats`' id base: `16842752 + itemId`.
// src: StatList.java:87
pub const CRAFT_ITEM_BASE: i32 = 16842752;
/// `objectUseStats`' id base: `16908288 + itemId`.
// src: StatList.java:50, :57
pub const USE_ITEM_BASE: i32 = 16908288;
/// `objectBreakStats`' id base: `16973824 + itemId`.
// src: StatList.java:51, :58
pub const BREAK_ITEM_BASE: i32 = 16973824;

/// `Block.blocksList.length`.
// src: Block.java:16
const BLOCKS_LIST_LEN: i32 = 256;

/// The eleven `replaceSimilarBlocks` pairs, in order.
// src: StatList.java:145-157, block ids Block.java:594-686
const SIMILAR_BLOCKS: [(i32, i32); 11] = [
    (9, 8),   // waterStill, waterMoving
    (11, 11), // lavaStill, lavaStill (vanilla bug: lavaMoving intended)
    (91, 86), // pumpkinLantern, pumpkin
    (62, 61), // stoneOvenActive, stoneOvenIdle
    (74, 73), // oreRedstoneGlowing, oreRedstone
    (94, 93), // redstoneRepeaterActive, redstoneRepeaterIdle
    (76, 75), // torchRedstoneActive, torchRedstoneIdle
    (40, 39), // mushroomRed, mushroomBrown
    (43, 44), // stairDouble, stairSingle
    (2, 3),   // grass, dirt
    (60, 3),  // tilledField, dirt
];

/// One of the `StatBase[]` arrays: entries are indices into
/// [`StatList::stats`].
pub type StatArray = Vec<Option<u32>>;

/// `StatList`'s static state after the static initialisers.
#[derive(Clone, Debug, PartialEq)]
pub struct StatList {
    /// Every stat constructed, in construction (= registration) order.
    stats: Vec<StatBase>,
    /// `oneShotStats` (protected): `statId` to stat, every registered stat
    /// including those `replaceSimilarBlocks` later unlisted. Lookup only.
    one_shot_stats: BTreeMap<i32, u32>,
    /// `field_25188_a`: the registered stats in order, minus those
    /// `replaceSimilarBlocks` removed (the statistics screen's list; see
    /// docs/mappings.md).
    field_25188_a: Vec<u32>,
    /// `generalStats`: the `StatBasic`s.
    general_stats: Vec<u32>,
    /// `itemStats`: the use stats of items (id 256 and up).
    item_stats: Vec<u32>,
    /// `objectMineStats`: the mine-block stats.
    object_mine_stats: Vec<u32>,
    /// `mineBlockStatArray` (256 entries).
    mine_block_stat_array: StatArray,
    /// `objectCraftStats` (32000 entries).
    object_craft_stats: StatArray,
    /// `objectUseStats` (32000 entries).
    object_use_stats: StatArray,
    /// `objectBreakStats` (32000 entries).
    object_break_stats: StatArray,
    /// `AchievementList.achievementList`.
    achievement_list: Vec<u32>,
    /// `AchievementList.minDisplayColumn` (starts at 0).
    pub min_display_column: i32,
    /// `AchievementList.minDisplayRow`.
    pub min_display_row: i32,
    /// `AchievementList.maxDisplayColumn`.
    pub max_display_column: i32,
    /// `AchievementList.maxDisplayRow`.
    pub max_display_row: i32,
}

static INSTANCE: LazyLock<StatList> = LazyLock::new(StatList::build);

/// What `registerStat` adds the stat to besides `field_25188_a` and the id
/// map.
enum Register {
    /// `StatBase.registerStat()`.
    Base,
    /// `StatBasic.registerStat()`: also `generalStats`.
    Basic,
    /// `Achievement.registerAchievement()`: also `achievementList`.
    Achievement,
}

impl StatList {
    /// The static state, built on first use.
    pub fn get() -> &'static StatList {
        &INSTANCE
    }

    fn empty() -> StatList {
        StatList {
            stats: Vec::new(),
            one_shot_stats: BTreeMap::new(),
            field_25188_a: Vec::new(),
            general_stats: Vec::new(),
            item_stats: Vec::new(),
            object_mine_stats: Vec::new(),
            mine_block_stat_array: Vec::new(),
            object_craft_stats: Vec::new(),
            object_use_stats: Vec::new(),
            object_break_stats: Vec::new(),
            achievement_list: Vec::new(),
            min_display_column: 0,
            min_display_row: 0,
            max_display_column: 0,
            max_display_row: 0,
        }
    }

    /// The static initialisers in vanilla order (module docs).
    // src: StatList.java:11-20, :174-197, :56-61, :49-54, Item.java:380, Block.java:705
    fn build() -> StatList {
        use StatType::{Distance, Simple, Time};
        let mut l = StatList::empty();
        // StatList's static initialiser, from Item.java:380.
        // src: StatList.java:16-20
        for (id, key) in [
            (START_GAME_STAT, "stat.startGame"),
            (CREATE_WORLD_STAT, "stat.createWorld"),
            (LOAD_WORLD_STAT, "stat.loadWorld"),
            (JOIN_MULTIPLAYER_STAT, "stat.joinMultiplayer"),
            (LEAVE_GAME_STAT, "stat.leaveGame"),
        ] {
            l.register(
                StatBase::basic(id, key, Simple).init_independent_stat(),
                Register::Basic,
            );
        }
        // src: StatList.java:175-192
        for (id, key, ty, independent) in [
            (MINUTES_PLAYED_STAT, "stat.playOneMinute", Time, true),
            (DISTANCE_WALKED_STAT, "stat.walkOneCm", Distance, true),
            (DISTANCE_SWUM_STAT, "stat.swimOneCm", Distance, true),
            (DISTANCE_FALLEN_STAT, "stat.fallOneCm", Distance, true),
            (DISTANCE_CLIMBED_STAT, "stat.climbOneCm", Distance, true),
            (DISTANCE_FLOWN_STAT, "stat.flyOneCm", Distance, true),
            (DISTANCE_DOVE_STAT, "stat.diveOneCm", Distance, true),
            (
                DISTANCE_BY_MINECART_STAT,
                "stat.minecartOneCm",
                Distance,
                true,
            ),
            (DISTANCE_BY_BOAT_STAT, "stat.boatOneCm", Distance, true),
            (DISTANCE_BY_PIG_STAT, "stat.pigOneCm", Distance, true),
            (JUMP_STAT, "stat.jump", Simple, true),
            (DROP_STAT, "stat.drop", Simple, true),
            (DAMAGE_DEALT_STAT, "stat.damageDealt", Simple, false),
            (DAMAGE_TAKEN_STAT, "stat.damageTaken", Simple, false),
            (DEATHS_STAT, "stat.deaths", Simple, false),
            (MOB_KILLS_STAT, "stat.mobKills", Simple, false),
            (PLAYER_KILLS_STAT, "stat.playerKills", Simple, false),
            (FISH_CAUGHT_STAT, "stat.fishCaught", Simple, false),
        ] {
            let s = StatBase::basic(id, key, ty);
            let s = if independent {
                s.init_independent_stat()
            } else {
                s
            };
            l.register(s, Register::Basic);
        }
        // src: StatList.java:193
        l.mine_block_stat_array = l.init_minable_stats("stat.mineBlock", MINE_BLOCK_BASE);
        // src: StatList.java:194 (AchievementList's static initialiser)
        register_achievements(&mut |s| {
            l.register(s, Register::Achievement);
        });
        // StatList.java:195-196 reset both flags; initStats's body follows.
        // src: StatList.java:56-61 (initCraftableStats does nothing: the block flag is false)
        let mut use_stats = std::mem::take(&mut l.object_use_stats);
        l.init_usable_stats(
            &mut use_stats,
            "stat.useItem",
            USE_ITEM_BASE,
            BLOCKS_LIST_LEN,
            ITEMS_LIST_LEN,
        );
        let mut break_stats = std::mem::take(&mut l.object_break_stats);
        l.func_25149_b(
            &mut break_stats,
            "stat.breakItem",
            BREAK_ITEM_BASE,
            BLOCKS_LIST_LEN,
            ITEMS_LIST_LEN,
        );
        // Block.java:705: initBreakableStats.
        // src: StatList.java:49-54
        l.init_usable_stats(
            &mut use_stats,
            "stat.useItem",
            USE_ITEM_BASE,
            0,
            BLOCKS_LIST_LEN,
        );
        l.func_25149_b(
            &mut break_stats,
            "stat.breakItem",
            BREAK_ITEM_BASE,
            0,
            BLOCKS_LIST_LEN,
        );
        l.object_use_stats = use_stats;
        l.object_break_stats = break_stats;
        l.init_craftable_stats();
        l
    }

    /// `registerStat()` and its overrides: the duplicate check (a panic
    /// where Java throws; never reached), `field_25188_a.add`,
    /// `oneShotStats.put`; `StatBasic` adds to `generalStats`,
    /// `Achievement` to `achievementList` (and the `Achievement`
    /// constructor widens the display bounds).
    // src: StatBase.java:35-44, StatBasic.java:12-16, Achievement.java:26-40, :55-59
    fn register(&mut self, s: StatBase, how: Register) -> u32 {
        assert!(
            !self.one_shot_stats.contains_key(&s.stat_id),
            "Duplicate stat id {}",
            s.stat_id
        );
        if let StatKind::Achievement(a) = &s.kind {
            // src: Achievement.java:26-40 (in the constructor)
            self.min_display_column = self.min_display_column.min(a.display_column);
            self.min_display_row = self.min_display_row.min(a.display_row);
            self.max_display_column = self.max_display_column.max(a.display_column);
            self.max_display_row = self.max_display_row.max(a.display_row);
        }
        let i = self.stats.len() as u32;
        self.one_shot_stats.insert(s.stat_id, i);
        self.stats.push(s);
        self.field_25188_a.push(i);
        match how {
            Register::Base => {}
            Register::Basic => self.general_stats.push(i),
            Register::Achievement => self.achievement_list.push(i),
        }
        i
    }

    /// `initMinableStats(key, base)` (private): a `StatCrafting` per block
    /// with `getEnableStats()`, named after `translateBlockName()`, also
    /// added to `objectMineStats`; then `replaceAllSimilarBlocks`.
    // src: StatList.java:95-108
    fn init_minable_stats(&mut self, var0: &str, var1: i32) -> StatArray {
        let mut var2: StatArray = vec![None; 256];
        for var3 in 0..256 {
            if let Some(b) = block::block(var3)
                && b.enable_stats
            {
                // Block.translateBlockName(): getBlockName() + ".name" (Block.java:568-570)
                let var4 = StatName::formatted(var0, java_name_key(b.get_block_name()));
                let i = self.register(StatBase::crafting(var1 + var3, var4, var3), Register::Base);
                var2[var3 as usize] = Some(i);
                self.object_mine_stats.push(i);
            }
        }
        self.replace_all_similar_blocks(&mut var2);
        var2
    }

    /// `initUsableStats(array, key, base, from, to)` (private): a
    /// `StatCrafting` per item in `from..to` (array created on first use),
    /// named after `Item.getStatName()`; items (id 256 and up) also go to
    /// `itemStats`. Then `replaceAllSimilarBlocks`.
    // src: StatList.java:110-127
    fn init_usable_stats(
        &mut self,
        var0: &mut StatArray,
        var1: &str,
        var2: i32,
        var3: i32,
        var4: i32,
    ) {
        if var0.is_empty() {
            *var0 = vec![None; ITEMS_LIST_LEN as usize];
        }
        for var5 in var3..var4 {
            if let Some(it) = item::item(var5) {
                let var6 = StatName::formatted(var1, item_stat_name(it));
                let i = self.register(StatBase::crafting(var2 + var5, var6, var5), Register::Base);
                var0[var5 as usize] = Some(i);
                if var5 >= BLOCKS_LIST_LEN {
                    self.item_stats.push(i);
                }
            }
        }
        self.replace_all_similar_blocks(var0);
    }

    /// `func_25149_b(array, key, base, from, to)` (private): as
    /// [`Self::init_usable_stats`] for the items with `isDamagable()`, and
    /// no list besides `field_25188_a`: the break ("depleted") stats.
    // src: StatList.java:129-143
    fn func_25149_b(&mut self, var0: &mut StatArray, var1: &str, var2: i32, var3: i32, var4: i32) {
        if var0.is_empty() {
            *var0 = vec![None; ITEMS_LIST_LEN as usize];
        }
        for var5 in var3..var4 {
            if let Some(it) = item::item(var5)
                && it.is_damagable()
            {
                let var6 = StatName::formatted(var1, item_stat_name(it));
                let i = self.register(StatBase::crafting(var2 + var5, var6, var5), Register::Base);
                var0[var5 as usize] = Some(i);
            }
        }
        self.replace_all_similar_blocks(var0);
    }

    /// `initCraftableStats()`, its second call (both flags set): the
    /// output ids of every recipe (list order) and every smelting result
    /// (the smelting `HashMap`'s value order) go into a `HashSet<Integer>`,
    /// whose iteration order (§3.4 rule 4: [`JavaHashSet`]) is the
    /// registration order of the craft stats. Then
    /// `replaceAllSimilarBlocks`.
    // src: StatList.java:63-93
    fn init_craftable_stats(&mut self) {
        let mut var0: JavaHashSet<i32> = JavaHashSet::new();
        for var2 in CraftingManager::get_instance().get_recipe_list() {
            var0.insert(var2.get_recipe_output().item_id);
        }
        for var4 in smelting_list().values() {
            var0.insert(var4.item_id);
        }
        let mut craft: StatArray = vec![None; ITEMS_LIST_LEN as usize];
        let ids: Vec<i32> = var0.iter().copied().collect();
        for var5 in ids {
            if let Some(it) = item::item(var5) {
                let var3 = StatName::formatted("stat.craftItem", item_stat_name(it));
                let i = self.register(
                    StatBase::crafting(CRAFT_ITEM_BASE + var5, var3, var5),
                    Register::Base,
                );
                craft[var5 as usize] = Some(i);
            }
        }
        self.replace_all_similar_blocks(&mut craft);
        self.object_craft_stats = craft;
    }

    /// `replaceAllSimilarBlocks(array)` (private), the eleven pairs.
    // src: StatList.java:145-157
    fn replace_all_similar_blocks(&mut self, var0: &mut StatArray) {
        for (a, b) in SIMILAR_BLOCKS {
            self.replace_similar_blocks(var0, a as usize, b as usize);
        }
    }

    /// `replaceSimilarBlocks(array, x, y)` (private): module docs. Java's
    /// `List.remove(null)` removes nothing (no list holds null).
    // src: StatList.java:159-168
    fn replace_similar_blocks(&mut self, var0: &mut StatArray, var1: usize, var2: usize) {
        if var0[var1].is_some() && var0[var2].is_none() {
            var0[var2] = var0[var1];
        } else {
            if let Some(s) = var0[var1] {
                for list in [
                    &mut self.field_25188_a,
                    &mut self.object_mine_stats,
                    &mut self.general_stats,
                ] {
                    if let Some(p) = list.iter().position(|&x| x == s) {
                        list.remove(p);
                    }
                }
            }
            var0[var1] = var0[var2];
        }
    }

    // ---- lookups ------------------------------------------------------------------

    fn at(&'static self, i: u32) -> &'static StatBase {
        &self.stats[i as usize]
    }

    fn array(&'static self, a: &'static StatArray, id: i32) -> Option<&'static StatBase> {
        let i = usize::try_from(id).ok()?;
        a.get(i).copied().flatten().map(|i| self.at(i))
    }

    /// `func_27361_a(id)`: `oneShotStats.get(id)`, the stat with this
    /// `statId` (including unlisted ones), or null.
    // src: StatList.java:170-172
    pub fn func_27361_a(&'static self, var0: i32) -> Option<&'static StatBase> {
        self.one_shot_stats.get(&var0).map(|&i| self.at(i))
    }

    /// `mineBlockStatArray[blockId]`; `None` for a null entry or an index
    /// outside the array (where Java would throw).
    pub fn mine_block_stat(&'static self, block_id: i32) -> Option<&'static StatBase> {
        self.array(&self.mine_block_stat_array, block_id)
    }

    /// `objectCraftStats[itemId]`.
    pub fn object_craft_stat(&'static self, item_id: i32) -> Option<&'static StatBase> {
        self.array(&self.object_craft_stats, item_id)
    }

    /// `objectUseStats[itemId]`.
    pub fn object_use_stat(&'static self, item_id: i32) -> Option<&'static StatBase> {
        self.array(&self.object_use_stats, item_id)
    }

    /// `objectBreakStats[itemId]`.
    pub fn object_break_stat(&'static self, item_id: i32) -> Option<&'static StatBase> {
        self.array(&self.object_break_stats, item_id)
    }

    /// `field_25188_a` in order.
    pub fn field_25188_a(&'static self) -> impl Iterator<Item = &'static StatBase> {
        self.field_25188_a.iter().map(|&i| self.at(i))
    }

    /// Every registered stat (`oneShotStats`' values) in `statId` order.
    pub fn one_shot_stats(&'static self) -> impl Iterator<Item = &'static StatBase> {
        self.one_shot_stats.values().map(|&i| self.at(i))
    }

    /// `generalStats` in order.
    pub fn general_stats(&'static self) -> impl Iterator<Item = &'static StatBase> {
        self.general_stats.iter().map(|&i| self.at(i))
    }

    /// `itemStats` in order.
    pub fn item_stats(&'static self) -> impl Iterator<Item = &'static StatBase> {
        self.item_stats.iter().map(|&i| self.at(i))
    }

    /// `objectMineStats` in order.
    pub fn object_mine_stats(&'static self) -> impl Iterator<Item = &'static StatBase> {
        self.object_mine_stats.iter().map(|&i| self.at(i))
    }

    /// `AchievementList.achievementList` in order.
    pub fn achievement_list(&'static self) -> impl Iterator<Item = &'static StatBase> {
        self.achievement_list.iter().map(|&i| self.at(i))
    }

    /// One of the four arrays by its Java name (`"mineBlockStatArray"`,
    /// `"objectUseStats"`, `"objectBreakStats"`, `"objectCraftStats"`):
    /// its length and the non-null entries as `(index, stat)` in index
    /// order.
    pub fn array_entries(
        &'static self,
        name: &str,
    ) -> Option<(usize, Vec<(usize, &'static StatBase)>)> {
        let a = match name {
            "mineBlockStatArray" => &self.mine_block_stat_array,
            "objectUseStats" => &self.object_use_stats,
            "objectBreakStats" => &self.object_break_stats,
            "objectCraftStats" => &self.object_craft_stats,
            _ => return None,
        };
        let entries = a
            .iter()
            .enumerate()
            .filter_map(|(k, e)| e.map(|i| (k, self.at(i))))
            .collect();
        Some((a.len(), entries))
    }
}

/// `x + ".name"` where `x` may be Java's null (`"null.name"`).
fn java_name_key(name: Option<String>) -> String {
    format!("{}.name", name.as_deref().unwrap_or("null"))
}

/// `Item.getStatName()`: `getItemName() + ".name"`, translated.
// src: Item.java:259-261
fn item_stat_name(it: &item::ItemDef) -> String {
    java_name_key(it.get_item_name())
}

/// Shorthand for [`StatList::get`]`().func_27361_a(id)`.
pub fn stat(id: i32) -> Option<&'static StatBase> {
    StatList::get().func_27361_a(id)
}
