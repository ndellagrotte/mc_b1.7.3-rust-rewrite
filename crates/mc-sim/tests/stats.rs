//! WP-2.7: what the `stat/stat-table` fixture does not reach, because the
//! oracle's `HarnessPlayer` keeps `EntityPlayer`'s empty `addStat`: the
//! `EntityPlayerSP.addStat` rules (null stats, the achievement parents), the
//! call sites (a dig through `PlayerControllerSP`, a craft through
//! `SlotCrafting`, a kill, a jump and a fall), and `StatFileWriter`'s stats
//! file (format, checksum, the unsent file's read and save through
//! `Storage`).

use std::collections::BTreeMap;

use mc_sim::entity::player::{self, new_entity_player_sp};
use mc_sim::entity::{Entity, dispatch, zombie};
use mc_sim::inventory::container::workbench::new_container_workbench;
use mc_sim::item::ItemStack;
use mc_sim::player_controller::{self, PlayerControllerSp};
use mc_sim::save::{MemoryStorage, NoSave, Storage};
use mc_sim::stats::achievement_list as al;
use mc_sim::stats::file_writer::{func_27177_a, func_27185_a, md5_string};
use mc_sim::stats::stat_list as sl;
use mc_sim::stats::{AchievementMap, StatFileWriter, StatList, stat};
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::{World, WorldEnvironment};

const STONE: i32 = 1;
const PLANKS: i32 = 5;
const WORKBENCH: i32 = 58;
const SWORD_DIAMOND: i32 = 276;

fn world() -> World {
    let mut w = World::new(
        Box::new(NoSave),
        "harness",
        WorldProvider::new(Dimension::Surface),
        5,
        ChunkGenerator::Empty(EmptyChunkProvider),
        WorldEnvironment {
            rand_seed: 0,
            field_9437_g: 0,
            lock_timestamp: 0,
        },
    );
    w.make_deterministic(5);
    for dz in -1..=1 {
        for dx in -1..=1 {
            w.get_chunk_from_chunk_coords(dx, dz);
        }
    }
    for x in -8..8 {
        for z in -8..8 {
            w.set_block(x, 60, z, STONE);
        }
    }
    w
}

fn writer(p: &Entity) -> &StatFileWriter {
    &p.kind.player().unwrap().stat_file_writer
}

fn value(p: &Entity, id: i32) -> i32 {
    writer(p).write_stat(stat(id).expect("a registered stat"))
}

/// Synthetic guids (the real ones are the jar's `achievement/map.txt`).
fn guids() -> AchievementMap {
    AchievementMap::parse(
        "1000,aaaa\n2010,bbbb\n16777217,cccc\n5242880,dddd\n5242881,eeee\n16842810,ffff\n",
    )
}

// ---- EntityPlayerSP.addStat -------------------------------------------------------

/// Null stats do nothing; an achievement whose parent is locked is not
/// recorded; once the chain is unlocked from the root, each counts and is
/// queued for the toast once.
#[test]
fn achievement_needs_its_parent() {
    let mut s = StatFileWriter::default();
    s.entity_player_sp_add_stat(None, 5);
    assert!(s.field_25102_a.is_empty());
    assert!(!s.field_27189_c);

    let open = stat(al::OPEN_INVENTORY).unwrap();
    let wood = stat(al::MINE_WOOD).unwrap();
    let bench = stat(al::BUILD_WORK_BENCH).unwrap();
    assert!(!s.can_unlock_achievement(wood));
    s.entity_player_sp_add_stat(Some(bench), 1);
    s.entity_player_sp_add_stat(Some(wood), 1);
    assert!(s.field_25102_a.is_empty());

    s.entity_player_sp_add_stat(Some(open), 1);
    assert!(s.has_achievement_unlocked(open));
    assert!(s.can_unlock_achievement(wood));
    s.entity_player_sp_add_stat(Some(wood), 1);
    s.entity_player_sp_add_stat(Some(wood), 1);
    s.entity_player_sp_add_stat(Some(bench), 1);
    assert_eq!(s.write_stat(wood), 2);
    assert_eq!(s.write_stat(bench), 1);
    assert_eq!(
        s.taken_achievements,
        vec![al::OPEN_INVENTORY, al::MINE_WOOD, al::BUILD_WORK_BENCH]
    );
    assert_eq!(s.field_25101_b, s.field_25102_a);
    assert!(s.field_27189_c);

    // A plain stat needs nothing.
    s.entity_player_sp_add_stat(stat(sl::JUMP_STAT), 3);
    assert_eq!(s.write_stat(stat(sl::JUMP_STAT).unwrap()), 3);
}

/// `replaceSimilarBlocks`: mining grass counts as dirt; the lit furnace's
/// use stat is the furnace's; lava keeps its own (unlisted) use stat (the
/// `lavaStill, lavaStill` pair, StatList.java:147).
#[test]
fn similar_blocks_share_a_stat() {
    let l = StatList::get();
    assert_eq!(
        l.mine_block_stat(2).unwrap().stat_id,
        sl::MINE_BLOCK_BASE + 3
    );
    assert_eq!(
        l.object_use_stat(62).unwrap().stat_id,
        sl::USE_ITEM_BASE + 61
    );
    assert_eq!(
        l.object_use_stat(11).unwrap().stat_id,
        sl::USE_ITEM_BASE + 11
    );
    assert!(
        l.field_25188_a()
            .all(|s| s.stat_id != sl::USE_ITEM_BASE + 11)
    );
    assert!(l.mine_block_stat(18).is_none(), "leaves: disableStats");
    assert!(l.object_break_stat(STONE).is_none());
    assert_eq!(
        l.object_break_stat(SWORD_DIAMOND).unwrap().stat_id,
        sl::BREAK_ITEM_BASE + SWORD_DIAMOND
    );
}

// ---- the call sites -----------------------------------------------------------------

/// A dig through `PlayerControllerSP.clickBlock` (an instant break of a
/// flower) and `sendBlockRemoving` (stone, finished): `harvestBlock`'s mine
/// stat; a hand-held diamond sword wears on each block
/// (`onBlockDestroyed`: true, the use stat).
#[test]
fn digging_counts_mined_blocks() {
    let mut w = world();
    w.set_block(2, 61, 4, 37);
    w.set_block(0, 61, 4, STONE);
    let mut p = new_entity_player_sp(&mut w);
    p.base.set_location_and_angles(0.5, 61.0, 0.5, 0.0, 0.0);
    let id = p.id();
    w.spawn_entity_in_world(p);
    w.with_entity_mut(id, |w, p| {
        p.base.on_ground = true;
        let mut c = PlayerControllerSp::new();
        assert!(c.click_block(w, p, 2, 61, 4, 2));
        assert_eq!(value(p, sl::MINE_BLOCK_BASE + 37), 1);

        p.kind.player_mut().unwrap().inventory.main_inventory[0] =
            Some(ItemStack::new(SWORD_DIAMOND, 1, 0));
        // A fresh controller: the first call picks the block, then it breaks.
        let mut c = PlayerControllerSp::new();
        assert!(!c.send_block_removing(w, p, 0, 61, 4, 2));
        c.cur_block_damage = 0.999;
        assert!(c.send_block_removing(w, p, 0, 61, 4, 2));
        assert_eq!(w.get_block_id(0, 61, 4), 0);
        // Stone by hand with a sword: no harvest, so no mine stat, but the
        // sword's onBlockDestroyed (true) counts a use.
        assert_eq!(value(p, sl::MINE_BLOCK_BASE + STONE), 0);
        assert_eq!(value(p, sl::USE_ITEM_BASE + SWORD_DIAMOND), 1);
    });
}

/// A workbench crafted from four planks in a workbench grid and taken from
/// the result slot: `onCrafting`'s craft stat (by the stack size) and,
/// with `mineWood` unlocked, the `buildWorkBench` achievement.
#[test]
fn crafting_counts_and_unlocks() {
    let mut w = world();
    let mut p = new_entity_player_sp(&mut w);
    {
        let s = &mut p.kind.player_mut().unwrap().stat_file_writer;
        s.entity_player_sp_add_stat(stat(al::OPEN_INVENTORY), 1);
        s.entity_player_sp_add_stat(stat(al::MINE_WOOD), 1);
    }
    let mut c = new_container_workbench(0, 0, 0);
    for k in [1, 2, 4, 5] {
        c.put_stack_in_slot(&mut w, &mut p, k, Some(ItemStack::new(PLANKS, 1, 0)));
    }
    assert_eq!(
        c.craft_result().get_stack_in_slot(0).unwrap().item_id,
        WORKBENCH
    );
    c.slot_click(&mut w, &mut p, 0, 0, false);
    assert_eq!(value(&p, sl::CRAFT_ITEM_BASE + WORKBENCH), 1);
    assert_eq!(value(&p, al::BUILD_WORK_BENCH), 1);
    assert!(
        writer(&p)
            .taken_achievements
            .contains(&al::BUILD_WORK_BENCH)
    );
}

/// Killing a zombie with a diamond sword through
/// `PlayerController.attackEntity`: damage dealt and the sword's use stat.
/// `EntityLiving.onDeath` does not call the killer's `addToPlayerScore` /
/// `onKillEntity` yet (recorded gaps in entity/living.rs, reported); the
/// two are called as it would: the mob kill, `killEnemy` once
/// `buildSword` is unlocked, and a player kill.
#[test]
fn a_kill_counts() {
    let mut w = world();
    let mut p = new_entity_player_sp(&mut w);
    p.base.set_location_and_angles(0.5, 61.0, 0.5, 0.0, 0.0);
    {
        let pd = p.kind.player_mut().unwrap();
        pd.inventory.main_inventory[0] = Some(ItemStack::new(SWORD_DIAMOND, 1, 0));
        for a in [
            al::OPEN_INVENTORY,
            al::MINE_WOOD,
            al::BUILD_WORK_BENCH,
            al::BUILD_SWORD,
        ] {
            pd.stat_file_writer.entity_player_sp_add_stat(stat(a), 1);
        }
    }
    let mut z = zombie::new_entity_zombie(&mut w);
    z.base.set_location_and_angles(1.5, 61.0, 0.5, 0.0, 0.0);
    z.kind.living_mut().unwrap().health = 1;
    player_controller::attack_entity(&mut w, &mut p, &mut z);
    assert!(z.kind.living().unwrap().health <= 0);
    // Diamond sword: 4 + 3 * 2 = 10 (ItemSword.java:10).
    assert_eq!(value(&p, sl::DAMAGE_DEALT_STAT), 10);
    assert_eq!(value(&p, sl::USE_ITEM_BASE + SWORD_DIAMOND), 1);
    assert_eq!(
        w.unimplemented_hits
            .borrow()
            .get("EntityPlayer.addToPlayerScore"),
        Some(&1)
    );
    assert_eq!(
        w.unimplemented_hits
            .borrow()
            .get("EntityPlayer.onKillEntity"),
        Some(&1)
    );
    // src: EntityLiving.java:407-414 (onDeath: addToPlayerScore, onKillEntity)
    dispatch::add_to_player_score(&mut p, &mut w, &z, 0);
    dispatch::on_kill_entity(&mut p, &mut w, &z);
    assert_eq!(value(&p, sl::MOB_KILLS_STAT), 1);
    assert_eq!(value(&p, al::KILL_ENEMY), 1);
    let other = new_entity_player_sp(&mut w);
    dispatch::add_to_player_score(&mut p, &mut w, &other, 0);
    dispatch::on_kill_entity(&mut p, &mut w, &other);
    assert_eq!(value(&p, sl::PLAYER_KILLS_STAT), 1);
    assert_eq!(value(&p, al::KILL_ENEMY), 1);
}

/// `jump` and `fall` (EntityPlayer.java:721-724, :787-793): one jump; a
/// 3-block fall is 300 cm; under 2 blocks nothing.
#[test]
fn jumps_and_falls() {
    let mut w = world();
    let mut p = new_entity_player_sp(&mut w);
    p.base.set_location_and_angles(0.5, 61.0, 0.5, 0.0, 0.0);
    player::jump(&mut p);
    assert_eq!(value(&p, sl::JUMP_STAT), 1);
    player::fall(&mut p, &mut w, 1.5);
    assert!(
        !writer(&p)
            .field_25102_a
            .contains_key(&sl::DISTANCE_FALLEN_STAT)
    );
    player::fall(&mut p, &mut w, 3.0);
    assert_eq!(value(&p, sl::DISTANCE_FALLEN_STAT), 300);
}

// ---- the stats file -------------------------------------------------------------------

/// `func_27185_a`'s exact text and its parse back.
#[test]
fn stats_file_format_round_trip() {
    let g = guids();
    let mut m = BTreeMap::new();
    m.insert(1000, 3);
    m.insert(2010, 42);
    m.insert(16777217, -7);
    let text = func_27185_a(Some("Player"), Some("local"), &m, &g);
    let sum = md5_string(Some("local"), "aaaa,3,bbbb,42,cccc,-7,");
    assert_eq!(
        text,
        format!(
            "{{\r\n  \"user\":{{\r\n    \"name\":\"Player\",\r\n    \"sessionid\":\"local\"\r\n  }},\r\n  \"stats-change\":[\r\n    {{\"1000\":3}},\r\n    {{\"2010\":42}},\r\n    {{\"16777217\":-7}}\r\n  ],\r\n  \"checksum\":\"{sum}\"\r\n}}"
        )
    );
    assert_eq!(func_27177_a(&text, &g), Some(m));

    // No user block without both strings; an empty map.
    let empty = func_27185_a(None, Some("local"), &BTreeMap::new(), &g);
    assert_eq!(
        empty,
        format!(
            "{{\r\n  \"stats-change\":[\r\n  ],\r\n  \"checksum\":\"{}\"\r\n}}",
            md5_string(Some("local"), "")
        )
    );
    assert_eq!(func_27177_a(&empty, &g), Some(BTreeMap::new()));
}

/// The checksum guards the values; a syntax error gives an empty map; an
/// unknown stat id is skipped and left out of the checksum; string values
/// parse; anything else fails (null).
#[test]
fn stats_file_parse_failures() {
    let g = guids();
    let mut m = BTreeMap::new();
    m.insert(2010, 42);
    let text = func_27185_a(Some("Player"), Some("local"), &m, &g);
    assert_eq!(func_27177_a(&text.replace(":42", ":43"), &g), None);
    assert_eq!(func_27177_a("{not json", &g), Some(BTreeMap::new()));
    assert_eq!(func_27177_a("17", &g), Some(BTreeMap::new()));

    let sum = md5_string(Some("local"), "bbbb,5,");
    let skipped =
        format!("{{\"stats-change\":[{{\"1\":9}},{{\"2010\":\"5\"}}],\"checksum\":\"{sum}\"}}");
    assert_eq!(
        func_27177_a(&skipped, &g),
        Some(BTreeMap::from([(2010, 5)]))
    );

    let s = |body: &str| format!("{{\"stats-change\":[{body}],\"checksum\":\"{sum}\"}}");
    assert_eq!(func_27177_a(&s("{\"2010\":5.0}"), &g), None);
    assert_eq!(func_27177_a(&s("{}"), &g), None);
    assert_eq!(func_27177_a(&s("5"), &g), None);
    assert_eq!(func_27177_a("{\"checksum\":\"x\"}", &g), None);
    assert_eq!(func_27177_a("[]", &g), None);
}

/// `syncStats` writes `stats/stats_<user>_unsent.dat` (the `.tmp` renamed,
/// the previous file kept as `.old`); `open` relocates `stats_*.dat` from
/// the data directory, renames mixed-case files to the lower-case names
/// and merges the unsent file into both maps.
#[test]
fn unsent_file_save_and_open() {
    let g = guids();
    let mut st = MemoryStorage::default();
    let mut w = StatFileWriter::default();
    w.entity_player_sp_add_stat(stat(sl::JUMP_STAT), 2);
    w.entity_player_sp_add_stat(stat(al::OPEN_INVENTORY), 1);
    w.sync_stats(&mut st, "mc", "Player", &g).unwrap();
    let dat = "mc/stats/stats_player_unsent.dat";
    let first = st.read(dat).unwrap().unwrap();
    assert_eq!(st.read("mc/stats/stats_player_unsent.tmp").unwrap(), None);
    assert_eq!(st.read("mc/stats/stats_player_unsent.old").unwrap(), None);
    w.entity_player_sp_add_stat(stat(sl::JUMP_STAT), 1);
    w.sync_stats(&mut st, "mc", "Player", &g).unwrap();
    assert_eq!(
        st.read("mc/stats/stats_player_unsent.old").unwrap(),
        Some(first)
    );

    let r = StatFileWriter::open(&mut st, "mc", "Player", &g).unwrap();
    assert_eq!(r.field_25101_b, w.field_25101_b);
    assert_eq!(r.field_25102_a, w.field_25101_b);
    assert!(r.field_27189_c);
    assert!(r.taken_achievements.is_empty());

    // Relocation and the lower-case rename.
    let mut st2 = MemoryStorage::default();
    let bytes = st.read(dat).unwrap().unwrap();
    st2.write("mc/stats_Player_unsent.dat", &bytes).unwrap();
    st2.write("mc/options.txt", b"x").unwrap();
    let r2 = StatFileWriter::open(&mut st2, "mc", "Player", &g).unwrap();
    assert_eq!(st2.read("mc/stats_Player_unsent.dat").unwrap(), None);
    assert_eq!(st2.read("mc/stats/stats_Player_unsent.dat").unwrap(), None);
    assert_eq!(st2.read(dat).unwrap(), Some(bytes));
    assert_eq!(r2.field_25101_b, w.field_25101_b);
    assert!(st2.read("mc/options.txt").unwrap().is_some());

    // No file: empty.
    let r3 = StatFileWriter::open(&mut MemoryStorage::default(), "", "Player", &g).unwrap();
    assert_eq!(r3, StatFileWriter::default());
}
