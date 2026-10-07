//! `SaveHandler` (SaveHandler.java) and its McRegion subclass `SaveOldDir`
//! (SaveOldDir.java): one world directory's `level.dat`, `session.lock`
//! and chunk loaders.
//!
//! The Java class `SaveHandler` is [`DirSaveHandler`] here (the name
//! `SaveHandler` is the `ISaveHandler` trait, [`super::SaveHandler`]). Its
//! own `getChunkLoader` hands out the pre-McRegion `ChunkLoader` (per-chunk
//! files), which is deferred (PLAN_v4 D3), so only [`SaveOldDir`]
//! implements the trait.
//!
//! # Files (paths relative to the save directory)
//!
//! - `session.lock`: 8 bytes, the big-endian `long` `now` the handler was
//!   created at (SaveHandler.java:17, :32-47). `checkSessionLock` throws
//!   `MinecraftException` when it holds another value (another process
//!   opened the world since) or cannot be read (:53-69).
//! - `level.dat`: gzip NBT, root `""` with one compound `Data`
//!   ([`WorldInfo`]'s fields). Written as `level.dat_new`, then
//!   `level.dat_old` deleted, `level.dat` renamed to `level.dat_old`,
//!   `level.dat_new` renamed to `level.dat` (:109-136). Read from
//!   `level.dat`, falling back to `level.dat_old` (:81-107).
//! - `region/`, `DIM-1/region/`: the chunks ([`McRegionChunkLoader`]).
//! - `data/<name>.dat`: map item data (`getMapFile`, :167-169), read and
//!   written by `MapStorage` ([`crate::world::map_storage`]) through
//!   [`SaveHandler::read_map_file`] / [`SaveHandler::write_map_file`].
//!
//! `mkdirs()` of the save directory, `players/` and `data/`
//! (SaveHandler.java:21-27) and of `DIM-1` (SaveOldDir.java:15) have no
//! counterpart: [`super::Storage`] has no empty directories.

use mc_nbt::Tag;

use super::nbt::{self as n, Compound};
use super::{ChunkLoader, McRegionChunkLoader, SaveError, SaveHandler, SharedSaveStore, path_join};
use crate::world::info::WorldInfo;
use crate::world::provider::{Dimension, WorldProvider};

/// `SaveOldDir.saveWorldInfoAndPlayer`'s save version: McRegion.
// src: SaveOldDir.java:23
pub const MCREGION_SAVE_VERSION: i32 = 19132;

/// `SaveHandler` (SaveHandler.java) without its pre-McRegion chunk loader
/// (module docs).
#[derive(Debug)]
pub struct DirSaveHandler {
    store: SharedSaveStore,
    /// `saveDirectory`: the storage path of the world directory.
    save_directory: String,
    /// `now`: `System.currentTimeMillis()` at construction, the session-lock
    /// value.
    now: i64,
}

impl DirSaveHandler {
    /// `new SaveHandler(savesDir, name, createPlayersDir)`: the directory
    /// `savesDir/name`, then the session lock written (`func_22154_d`).
    /// `Err` where the Java throws `RuntimeException("Failed to check session
    /// lock, aborting")` (the lock cannot be written).
    // src: SaveHandler.java:19-30
    pub fn new(store: SharedSaveStore, saves_dir: &str, name: &str) -> Result<Self, SaveError> {
        let now = store.borrow_mut().clock.current_time_millis(); // src: SaveHandler.java:17
        let h = DirSaveHandler {
            store,
            save_directory: path_join(saves_dir, name),
            now,
        };
        h.write_session_lock()?;
        Ok(h)
    }

    /// `func_22154_d()`: writes `now` to `session.lock` (docs/mappings.md).
    // src: SaveHandler.java:32-47
    fn write_session_lock(&self) -> Result<(), SaveError> {
        let path = path_join(&self.save_directory, "session.lock");
        self.store
            .borrow_mut()
            .storage
            .write(&path, &self.now.to_be_bytes())
            .map_err(|_| SaveError::Minecraft("Failed to check session lock, aborting".into()))
    }

    /// `getSaveDirectory()`.
    // src: SaveHandler.java:49-51
    pub fn get_save_directory(&self) -> &str {
        &self.save_directory
    }

    /// The shared store.
    pub fn store(&self) -> &SharedSaveStore {
        &self.store
    }

    /// The session-lock value this handler wrote.
    pub fn lock_timestamp(&self) -> i64 {
        self.now
    }

    /// `getMapFile(name)`: `new File(field_28114_d, name + ".dat")`, where
    /// `field_28114_d` is the `data` directory of the save directory
    /// (docs/mappings.md).
    // src: SaveHandler.java:23, :167-169
    pub fn get_map_file(&self, name: &str) -> String {
        path_join(
            &path_join(&self.save_directory, "data"),
            &format!("{name}.dat"),
        )
    }

    /// The bytes of [`Self::get_map_file`], `None` when it does not exist.
    pub fn read_map_file(&self, name: &str) -> Result<Option<Vec<u8>>, SaveError> {
        let path = self.get_map_file(name);
        Ok(self.store.borrow_mut().storage.read(&path)?)
    }

    /// Writes [`Self::get_map_file`].
    pub fn write_map_file(&self, name: &str, bytes: &[u8]) -> Result<(), SaveError> {
        let path = self.get_map_file(name);
        Ok(self.store.borrow_mut().storage.write(&path, bytes)?)
    }

    /// `checkSessionLock()`.
    // src: SaveHandler.java:53-69
    pub fn check_session_lock(&self) -> Result<(), SaveError> {
        let path = path_join(&self.save_directory, "session.lock");
        let bytes = self.store.borrow_mut().storage.read(&path);
        match bytes {
            // DataInputStream.readLong: the first 8 bytes, EOFException if fewer.
            Ok(Some(b)) if b.len() >= 8 => {
                let v = i64::from_be_bytes(b[..8].try_into().expect("8 bytes"));
                if v != self.now {
                    return Err(SaveError::Minecraft(
                        "The save is being accessed from another location, aborting".into(),
                    ));
                }
                Ok(())
            }
            _ => Err(SaveError::Minecraft(
                "Failed to check session lock, aborting".into(),
            )),
        }
    }

    /// One `level.dat` candidate: `loadGzippedCompoundFromOutputStream`, then
    /// `new WorldInfo(getCompoundTag("Data"))`. `Ok(None)` when the file does
    /// not exist; `Err` for what the Java catches and prints.
    fn read_world_info(&self, name: &str) -> Result<Option<WorldInfo>, SaveError> {
        let path = path_join(&self.save_directory, name);
        let bytes = self.store.borrow_mut().storage.read(&path)?;
        let Some(bytes) = bytes else {
            return Ok(None);
        };
        Ok(Some(world_info_from_level_dat(&bytes)?))
    }

    /// `loadWorldInfo()`: `level.dat`, else `level.dat_old`, else `None`.
    /// A file that fails to read is skipped (printed in Java).
    // src: SaveHandler.java:81-107
    pub fn load_world_info(&self) -> Option<WorldInfo> {
        if let Ok(Some(i)) = self.read_world_info("level.dat") {
            return Some(i);
        }
        if let Ok(Some(i)) = self.read_world_info("level.dat_old") {
            return Some(i);
        }
        None
    }

    /// `saveWorldInfoAndPlayer(info, players)`: `Data` =
    /// `info.getNBTTagCompoundWithPlayer(players)`.
    // src: SaveHandler.java:109-136
    pub fn save_world_info_and_player(
        &self,
        info: &WorldInfo,
        player: Option<Tag>,
    ) -> Result<(), SaveError> {
        let now = self.store.borrow_mut().clock.current_time_millis();
        let var3 = info.get_nbt_tag_compound_with_player(player, now);
        self.write_level_dat(var3)
    }

    /// `saveWorldInfo(info)`: `Data` = `info.getNBTTagCompound()` (with the
    /// stored player compound).
    // src: SaveHandler.java:138-165
    pub fn save_world_info(&self, info: &WorldInfo) -> Result<(), SaveError> {
        let now = self.store.borrow_mut().clock.current_time_millis();
        let var2 = info.get_nbt_tag_compound(now);
        self.write_level_dat(var2)
    }

    /// The shared body of the two writers: `{"Data": data}` gzipped to
    /// `level.dat_new`, then the rename dance. Java's `File.delete` /
    /// `renameTo` return false on failure and are not checked; neither are
    /// these.
    // src: SaveHandler.java:111-131, :140-160
    fn write_level_dat(&self, data: Compound) -> Result<(), SaveError> {
        let mut var4 = Compound::new();
        n::set(&mut var4, "Data", Tag::Compound(data));
        let bytes = mc_nbt::write_gzip("", &Tag::Compound(var4))?;
        let dir = &self.save_directory;
        let var5 = path_join(dir, "level.dat_new");
        let var6 = path_join(dir, "level.dat_old");
        let var7 = path_join(dir, "level.dat");
        let mut s = self.store.borrow_mut();
        let st = s.storage.as_mut();
        st.write(&var5, &bytes)?;
        if st.read(&var6)?.is_some() {
            let _ = st.delete(&var6);
        }
        let _ = st.rename(&var7, &var6);
        if st.read(&var7)?.is_some() {
            let _ = st.delete(&var7);
        }
        let _ = st.rename(&var5, &var7);
        if st.read(&var5)?.is_some() {
            let _ = st.delete(&var5);
        }
        Ok(())
    }
}

/// A `level.dat` file's bytes → `new WorldInfo(root.getCompoundTag("Data"))`
/// (SaveHandler.java:87-89, SaveFormatOld.java:51-53).
pub fn world_info_from_level_dat(bytes: &[u8]) -> Result<WorldInfo, SaveError> {
    let (_, root) = mc_nbt::read_gzip(bytes)?;
    let root = n::as_compound(&root)?;
    let data = n::get_compound_tag(root, "Data")?;
    Ok(WorldInfo::from_nbt(&data)?)
}

/// `SaveOldDir` (SaveOldDir.java): the McRegion save handler.
#[derive(Debug)]
pub struct SaveOldDir {
    /// The `SaveHandler` part.
    pub base: DirSaveHandler,
}

impl SaveOldDir {
    /// `new SaveOldDir(savesDir, name, createPlayersDir)`: writes the session
    /// lock ([`DirSaveHandler::new`]). The players-directory flag only
    /// creates a directory (module docs) and is not taken.
    // src: SaveOldDir.java:7-9
    pub fn new(store: SharedSaveStore, saves_dir: &str, name: &str) -> Result<Self, SaveError> {
        Ok(SaveOldDir {
            base: DirSaveHandler::new(store, saves_dir, name)?,
        })
    }

    /// The world directory's storage path.
    pub fn get_save_directory(&self) -> &str {
        self.base.get_save_directory()
    }
}

impl SaveHandler for SaveOldDir {
    // src: SaveHandler.java:81-107
    fn load_world_info(&mut self) -> Option<WorldInfo> {
        self.base.load_world_info()
    }

    // src: SaveHandler.java:53-69
    fn check_session_lock(&mut self) -> Result<(), SaveError> {
        self.base.check_session_lock()
    }

    /// `getChunkLoader(provider)`: a [`McRegionChunkLoader`] on the save
    /// directory, or on its `DIM-1` for the nether.
    // src: SaveOldDir.java:11-20
    fn get_chunk_loader(&mut self, provider: &WorldProvider) -> Option<Box<dyn ChunkLoader>> {
        let var2 = self.base.get_save_directory();
        let dir = if provider.dimension == Dimension::Hell {
            path_join(var2, "DIM-1")
        } else {
            var2.to_string()
        };
        Some(Box::new(McRegionChunkLoader::new(
            self.base.store().clone(),
            &dir,
        )))
    }

    /// `saveWorldInfoAndPlayer`: sets the save version to McRegion's 19132
    /// on the world's `WorldInfo`, then the `SaveHandler` body.
    // src: SaveOldDir.java:22-25
    fn save_world_info_and_player(
        &mut self,
        info: &mut WorldInfo,
        player: Option<Tag>,
    ) -> Result<(), SaveError> {
        info.save_version = MCREGION_SAVE_VERSION;
        self.base.save_world_info_and_player(info, player)
    }

    // src: SaveHandler.java:138-165
    fn save_world_info(&mut self, info: &WorldInfo) -> Result<(), SaveError> {
        self.base.save_world_info(info)
    }

    // src: SaveHandler.java:167-169 (never null)
    fn has_map_files(&self) -> bool {
        true
    }

    // src: SaveHandler.java:167-169
    fn read_map_file(&mut self, name: &str) -> Result<Option<Vec<u8>>, SaveError> {
        self.base.read_map_file(name)
    }

    // src: SaveHandler.java:167-169
    fn write_map_file(&mut self, name: &str, bytes: &[u8]) -> Result<(), SaveError> {
        self.base.write_map_file(name, bytes)
    }
}
