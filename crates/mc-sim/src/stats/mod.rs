//! Statistics and achievements (WP-2.7): `StatBase` and subclasses,
//! `StatList`, `Achievement`, `AchievementList`, `AchievementMap`,
//! `StatFileWriter` and the local file part of `StatsSyncher`.
//!
//! # Audit
//!
//! | Java | Port |
//! |---|---|
//! | StatBase.java, StatBasic.java, StatCrafting.java, IStatType.java | [`StatBase`], [`StatKind`], [`StatType`] |
//! | StatTypeSimple.java, StatTypeTime.java, StatTypeDistance.java | [`StatType::format`] |
//! | StatCollector.java, StringTranslate.java (the lookup) | names are kept as keys, [`StatName`]; [`StatName::translate`] for the client |
//! | StatList.java | [`StatList`] ([`stat_list`] module docs: the initialisation order) |
//! | Achievement.java | [`Achievement`] |
//! | AchievementList.java | [`achievement_list`] (ids), registered by [`StatList`] |
//! | AchievementMap.java | [`AchievementMap`] (the client passes the jar's `map.txt`) |
//! | MD5String.java | [`file_writer::md5_string`] |
//! | StatFileWriter.java | [`StatFileWriter`], [`file_writer::func_27177_a`], [`file_writer::func_27185_a`] |
//! | StatsSyncher.java | [`file_writer::StatsSyncher`]; [`StatFileWriter::open`], [`StatFileWriter::func_27178_d`], [`StatFileWriter::sync_stats`] (local files; run synchronously, [`file_writer`] docs) |
//! | ThreadStatSyncherReceive.java, ThreadStatSyncherSend.java | their bodies, run at once in [`StatFileWriter::open`] and [`StatFileWriter::func_27178_d`] |
//! | StatStringFormatKeyInv.java, IStatStringFormat.java | client (the inventory key's name in `openInventory`'s description, Minecraft.java:248); not ported |
//! | `EntityPlayerSP.addStat` (EntityPlayerSP.java:176-192) | [`StatFileWriter::entity_player_sp_add_stat`], reached from [`crate::entity::player::add_stat`] |
//!
//! # Where stats are counted
//!
//! `EntityPlayer.addStat` is empty in Java; the local player's
//! `EntityPlayerSP.addStat` writes `Minecraft.statFileWriter`. The port's
//! only player kind is the local one, whose [`crate::entity::PlayerData`]
//! carries the writer ([`crate::entity::PlayerData::stat_file_writer`]: it
//! stands for `mc.statFileWriter`, which outlives the player entity, so the
//! client moves it along when it replaces the player). The oracle's
//! `HarnessPlayer` keeps the empty body, so stats never reach a golden
//! trace; `crates/mc-sim/tests/stats.rs` covers the call sites, and the
//! `stat/stat-table` fixture the registries.
//!
//! Client-only call sites, not in the simulation: `Minecraft.startWorld`'s
//! `createWorldStat` / `loadWorldStat` / `startGameStat`
//! (Minecraft.java:1201-1206), `NetClientHandler`'s `joinMultiplayerStat`
//! (multiplayer, D3), the GUI's `leaveGameStat`, and
//! `EntityPlayerSP`'s `openInventory` hint (EntityPlayerSP.java:35-37).

pub mod achievement;
pub mod achievement_list;
pub mod file_writer;
pub mod stat_base;
pub mod stat_list;

pub use achievement::Achievement;
pub use file_writer::{AchievementMap, StatFileWriter, StatsSyncher};
pub use stat_base::{StatBase, StatKind, StatName, StatType};
pub use stat_list::{StatList, stat};
