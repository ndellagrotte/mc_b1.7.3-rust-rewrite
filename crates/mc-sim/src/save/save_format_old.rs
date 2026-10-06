//! The saves directory: `ISaveFormat` (ISaveFormat.java) as the
//! [`SaveFormat`] trait, `SaveFormatOld` (SaveFormatOld.java), the
//! McRegion subclass `SaveConverterMcRegion` (SaveConverterMcRegion.java,
//! the one the client uses) without its conversion, and
//! `SaveFormatComparator` (SaveFormatComparator.java), a world-list row.
//!
//! Deferred (PLAN_v4 D3): converting a pre-McRegion world
//! (`SaveConverterMcRegion.convertMapFormat` and its helpers,
//! SaveConverterMcRegion.java:62-171) and `SaveFormatOld.getSaveLoader`,
//! which hands out the pre-McRegion `SaveHandler` (SaveFormatOld.java:111-113);
//! both return [`SaveError::Deferred`].
//!
//! Directory order: `File.listFiles()` (SaveConverterMcRegion.java:25) has
//! no defined order; [`super::Storage::list`] is sorted. The world-select
//! screen sorts the list itself (`Collections.sort`, by
//! [`SaveFormatComparator`]'s order).

use std::cmp::Ordering;

use mc_nbt::Tag;

use super::nbt::{self as n};
use super::save_handler::{SaveOldDir, world_info_from_level_dat};
use super::{SaveError, SaveHandler, SharedSaveStore, Storage, StorageError, path_join};
use crate::world::info::WorldInfo;

/// `ISaveFormat`.
// src: ISaveFormat.java:5-23
pub trait SaveFormat {
    /// `getWorldFormat()`.
    fn get_world_format(&self) -> &'static str;
    /// `getSaveLoader(name, createPlayersDir)`.
    fn get_save_loader(&mut self, name: &str) -> Result<Box<dyn SaveHandler>, SaveError>;
    /// `getSaveList()`.
    fn get_save_list(&mut self) -> Vec<SaveFormatComparator>;
    /// `flushCache()`.
    fn flush_cache(&mut self);
    /// `getWorldInfo(name)`.
    fn get_world_info(&mut self, name: &str) -> Option<WorldInfo>;
    /// `deleteWorldDirectory(name)`.
    fn delete_world_directory(&mut self, name: &str);
    /// `renameWorld(name, newLevelName)`.
    fn rename_world(&mut self, name: &str, new_name: &str);
    /// `isOldMapFormat(name)`.
    fn is_old_map_format(&mut self, name: &str) -> bool;
    /// `convertMapFormat(name, progress)`.
    fn convert_map_format(&mut self, name: &str) -> Result<bool, SaveError>;
}

/// `SaveFormatComparator`: one row of the world list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveFormatComparator {
    /// `fileName`: the world's directory name.
    pub file_name: String,
    /// `displayName`: the level name (the directory name when empty).
    pub display_name: String,
    /// `field_22169_c`: `lastTimePlayed` (`getLastTimePlayed()`).
    pub last_time_played: i64,
    /// `field_22168_d`: `sizeOnDisk` (`func_22159_c()`).
    pub size_on_disk: i64,
    /// `field_22167_e`: the world needs conversion (`func_22161_d()`).
    pub requires_conversion: bool,
}

// src: SaveFormatComparator.java:3-30
impl SaveFormatComparator {
    /// `new SaveFormatComparator(fileName, displayName, lastPlayed, size, convert)`.
    pub fn new(
        file_name: &str,
        display_name: &str,
        last_time_played: i64,
        size_on_disk: i64,
        requires_conversion: bool,
    ) -> Self {
        SaveFormatComparator {
            file_name: file_name.to_string(),
            display_name: display_name.to_string(),
            last_time_played,
            size_on_disk,
            requires_conversion,
        }
    }
}

impl Ord for SaveFormatComparator {
    /// `func_22160_a(other)` / `compareTo`: the most recently played first,
    /// then `fileName.compareTo` (UTF-16 code unit order).
    // src: SaveFormatComparator.java:38-44
    fn cmp(&self, other: &Self) -> Ordering {
        if self.last_time_played < other.last_time_played {
            Ordering::Greater
        } else if self.last_time_played > other.last_time_played {
            Ordering::Less
        } else {
            self.file_name
                .encode_utf16()
                .cmp(other.file_name.encode_utf16())
        }
    }
}

impl PartialOrd for SaveFormatComparator {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// `SaveFormatOld`: the saves directory `field_22180_a`.
#[derive(Debug)]
pub struct SaveFormatOld {
    /// The shared storage.
    pub store: SharedSaveStore,
    /// `field_22180_a`: the saves directory's storage path ("" for the
    /// storage root). Name: the constructor's only argument, the parent of
    /// every world directory (SaveFormatOld.java:12-18, :42).
    pub saves_dir: String,
}

impl SaveFormatOld {
    /// `new SaveFormatOld(dir)`. `mkdirs()` has no storage counterpart.
    // src: SaveFormatOld.java:12-18
    pub fn new(store: SharedSaveStore, saves_dir: &str) -> Self {
        SaveFormatOld {
            store,
            saves_dir: saves_dir.to_string(),
        }
    }

    fn world_dir(&self, name: &str) -> String {
        path_join(&self.saves_dir, name)
    }

    /// `getWorldInfo(name)`: `name/level.dat`, else `name/level.dat_old`;
    /// `None` when the directory does not exist or neither file reads.
    // src: SaveFormatOld.java:41-72
    pub fn get_world_info(&mut self, name: &str) -> Option<WorldInfo> {
        let var2 = self.world_dir(name);
        let mut s = self.store.borrow_mut();
        if !s.dir_exists(&var2).unwrap_or(false) {
            return None;
        }
        for file in ["level.dat", "level.dat_old"] {
            if let Ok(Some(bytes)) = s.storage.read(&path_join(&var2, file))
                && let Ok(i) = world_info_from_level_dat(&bytes)
            {
                return Some(i);
            }
        }
        None
    }

    /// `getSaveList()` of the original format: worlds `World1` to `World5`.
    // src: SaveFormatOld.java:24-36
    pub fn get_save_list_old(&mut self) -> Vec<SaveFormatComparator> {
        let mut var1 = Vec::new();
        for var2 in 0..5 {
            let var3 = format!("World{}", var2 + 1);
            if let Some(var4) = self.get_world_info(&var3) {
                var1.push(SaveFormatComparator::new(
                    &var3,
                    "",
                    var4.last_time_played,
                    var4.size_on_disk,
                    false,
                ));
            }
        }
        var1
    }

    /// `renameWorld(name, newName)`: rewrites `Data.LevelName` of
    /// `name/level.dat` in place (no rename dance). Failures are printed in
    /// Java and ignored here.
    // src: SaveFormatOld.java:74-90
    pub fn rename_world(&mut self, name: &str, new_name: &str) {
        let var4 = path_join(&self.world_dir(name), "level.dat");
        let mut s = self.store.borrow_mut();
        let Ok(Some(bytes)) = s.storage.read(&var4) else {
            return;
        };
        let Ok((root_name, mut root)) = mc_nbt::read_gzip(&bytes) else {
            return;
        };
        let Some(map) = root.as_compound_mut() else {
            return;
        };
        // getCompoundTag("Data"): a missing one is a new compound that is
        // never stored, so the file is rewritten unchanged.
        if let Some(Tag::Compound(data)) = map.get_mut("Data") {
            n::set(data, "LevelName", Tag::String(new_name.to_string()));
        } else if map.contains_key("Data") {
            return; // ClassCastException, printed
        }
        if let Ok(out) = mc_nbt::write_gzip(&root_name, &root) {
            let _ = s.storage.write(&var4, &out);
        }
    }

    /// `deleteWorldDirectory(name)`: every file below `name`, then the
    /// directory (`func_22179_a`, recursive).
    // src: SaveFormatOld.java:92-109
    pub fn delete_world_directory(&mut self, name: &str) {
        let var2 = self.world_dir(name);
        let mut s = self.store.borrow_mut();
        let _ = delete_tree(s.storage.as_mut(), &var2);
    }
}

/// `func_22179_a(files)`: deletes a directory tree. Name: its only use is
/// emptying a world directory before deleting it (SaveFormatOld.java:95,
/// :100-109).
pub fn delete_tree(storage: &mut dyn Storage, dir: &str) -> Result<(), StorageError> {
    for name in storage.list(dir)? {
        let p = path_join(dir, &name);
        if storage.list(&p)?.is_empty() {
            storage.delete(&p)?;
        } else {
            delete_tree(storage, &p)?;
        }
    }
    Ok(())
}

impl SaveFormat for SaveFormatOld {
    // src: SaveFormatOld.java:20-22
    fn get_world_format(&self) -> &'static str {
        "Old Format"
    }
    /// Deferred: the pre-McRegion `SaveHandler` (module docs).
    // src: SaveFormatOld.java:111-113
    fn get_save_loader(&mut self, _name: &str) -> Result<Box<dyn SaveHandler>, SaveError> {
        Err(SaveError::Deferred(
            "SaveFormatOld.getSaveLoader: pre-McRegion SaveHandler/ChunkLoader (PLAN_v4 D3)",
        ))
    }
    // src: SaveFormatOld.java:24-36
    fn get_save_list(&mut self) -> Vec<SaveFormatComparator> {
        self.get_save_list_old()
    }
    // src: SaveFormatOld.java:38-39
    fn flush_cache(&mut self) {}
    fn get_world_info(&mut self, name: &str) -> Option<WorldInfo> {
        SaveFormatOld::get_world_info(self, name)
    }
    fn delete_world_directory(&mut self, name: &str) {
        SaveFormatOld::delete_world_directory(self, name)
    }
    fn rename_world(&mut self, name: &str, new_name: &str) {
        SaveFormatOld::rename_world(self, name, new_name)
    }
    // src: SaveFormatOld.java:115-117
    fn is_old_map_format(&mut self, _name: &str) -> bool {
        false
    }
    // src: SaveFormatOld.java:119-121
    fn convert_map_format(&mut self, _name: &str) -> Result<bool, SaveError> {
        Ok(false)
    }
}

/// `SaveConverterMcRegion`: the client's save format (McRegion), without
/// the conversion (module docs).
#[derive(Debug)]
pub struct SaveConverterMcRegion {
    /// The `SaveFormatOld` part.
    pub base: SaveFormatOld,
}

impl SaveConverterMcRegion {
    /// `new SaveConverterMcRegion(dir)`.
    // src: SaveConverterMcRegion.java:15-17
    pub fn new(store: SharedSaveStore, saves_dir: &str) -> Self {
        SaveConverterMcRegion {
            base: SaveFormatOld::new(store, saves_dir),
        }
    }

    /// `getSaveLoader(name, createPlayersDir)`: a [`SaveOldDir`].
    // src: SaveConverterMcRegion.java:53-55
    pub fn get_save_old_dir(&mut self, name: &str) -> Result<SaveOldDir, SaveError> {
        SaveOldDir::new(self.base.store.clone(), &self.base.saves_dir, name)
    }
}

impl SaveFormat for SaveConverterMcRegion {
    // src: SaveConverterMcRegion.java:19-21
    fn get_world_format(&self) -> &'static str {
        "Scaevolus' McRegion"
    }
    // src: SaveConverterMcRegion.java:53-55
    fn get_save_loader(&mut self, name: &str) -> Result<Box<dyn SaveHandler>, SaveError> {
        Ok(Box::new(self.get_save_old_dir(name)?))
    }
    /// Every directory of the saves directory with a readable `level.dat`
    /// (or `level.dat_old`); `requires_conversion` when its save version is
    /// not 19132; the display name falls back to the directory name.
    // src: SaveConverterMcRegion.java:23-47
    fn get_save_list(&mut self) -> Vec<SaveFormatComparator> {
        let mut var1 = Vec::new();
        let names = {
            let mut s = self.base.store.borrow_mut();
            s.storage.list(&self.base.saves_dir).unwrap_or_default()
        };
        for var7 in names {
            let is_dir = {
                let mut s = self.base.store.borrow_mut();
                s.dir_exists(&path_join(&self.base.saves_dir, &var7))
                    .unwrap_or(false)
            };
            if !is_dir {
                continue;
            }
            if let Some(var8) = self.base.get_world_info(&var7) {
                let var9 = var8.save_version != 19132;
                let mut var10 = var8.level_name.clone();
                // MathHelper.stringNullOrLengthZero (MathHelper.java:52-54)
                if var10.is_empty() {
                    var10 = var7.clone();
                }
                var1.push(SaveFormatComparator::new(
                    &var7,
                    &var10,
                    var8.last_time_played,
                    var8.size_on_disk,
                    var9,
                ));
            }
        }
        var1
    }
    /// `RegionFileCache.func_22192_a()` on the shared cache.
    // src: SaveConverterMcRegion.java:49-51
    fn flush_cache(&mut self) {
        self.base
            .store
            .borrow_mut()
            .region_cache
            .clear_region_file_references();
    }
    fn get_world_info(&mut self, name: &str) -> Option<WorldInfo> {
        self.base.get_world_info(name)
    }
    fn delete_world_directory(&mut self, name: &str) {
        self.base.delete_world_directory(name)
    }
    fn rename_world(&mut self, name: &str, new_name: &str) {
        self.base.rename_world(name, new_name)
    }
    /// A world whose `level.dat` has save version 0 (pre-McRegion).
    // src: SaveConverterMcRegion.java:57-60
    fn is_old_map_format(&mut self, name: &str) -> bool {
        self.base
            .get_world_info(name)
            .is_some_and(|i| i.save_version == 0)
    }
    /// Deferred (module docs).
    // src: SaveConverterMcRegion.java:62-90
    fn convert_map_format(&mut self, _name: &str) -> Result<bool, SaveError> {
        Err(SaveError::Deferred(
            "SaveConverterMcRegion.convertMapFormat: pre-McRegion conversion (PLAN_v4 D3)",
        ))
    }
}
