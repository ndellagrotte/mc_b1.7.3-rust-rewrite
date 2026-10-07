//! Saves (PLAN_v4 D5): the [`Storage`] trait (D9), the McRegion container,
//! the chunk NBT codec, `level.dat`, the save handlers and the import
//! validation.
//!
//! | Module | Vanilla | |
//! |---|---|---|
//! | [`storage`] | — | [`Storage`] (D9), [`MemoryStorage`] |
//! | [`region`] | `RegionFile`, `RegionFileChunkBuffer` | McRegion container, whole file held in memory, written back through [`Storage`] on every change (WP-1.9a) |
//! | [`region_cache`] | `RegionFileCache` | open region files by path, owned by the [`SaveStore`] instead of static (WP-1.9a) |
//! | [`nbt`] | `NBTTagCompound` getters | typed reads with Java's defaults and `ClassCastException`s |
//! | [`chunk_loader`] | `ChunkLoader` (static half), `ChunkBlockMap` | the chunk NBT codec (`storeChunkInCompound`, `loadChunkIntoWorldFromCompound`, `removeUnknownBlocks`) |
//! | [`mcregion_chunk_loader`] | `McRegionChunkLoader` | [`ChunkLoader`] over the region files |
//! | [`save_handler`] | `SaveHandler`, `SaveOldDir` | `level.dat` (+ `_old`/`_new` rename dance), `session.lock` |
//! | [`save_format_old`] | `SaveFormatOld`, `SaveConverterMcRegion` (non-conversion half), `SaveFormatComparator` | the saves directory: world list, rename, delete, `getSaveLoader` |
//! | [`entity_list`] | `EntityList` | the entity name table, `createEntityFromNBT` |
//! | [`import`] | — (PLAN_v4 D5) | id validation of a copied save before it is opened |
//!
//! `WorldInfo`'s NBT form is in [`crate::world::info`]; the per-entity NBT
//! bodies are in [`crate::entity::nbt`]; the tile entities' in
//! [`crate::tile_entity`] (WP-1.8).
//!
//! # Sharing the storage
//!
//! In Java the save handler and its chunk loaders reach the disk through
//! `File`s, and `RegionFileCache` is a process-wide static. Here one
//! [`SaveStore`] (the [`Storage`], the [`RegionFileCache`] and the
//! [`Clock`]) is shared by `Rc<RefCell<..>>` ([`SharedSaveStore`]) between a
//! [`save_handler::SaveOldDir`], the loaders it hands out and the
//! [`save_format_old::SaveConverterMcRegion`] that created it. The
//! simulation is single-threaded (PLAN_v4 §3.2), so `Rc` is enough; every
//! borrow is released before control returns to the world.
//!
//! # Time (D9)
//!
//! `System.currentTimeMillis()` is read by the save code in three places:
//! the session lock (SaveHandler.java:17), `LastPlayed` (WorldInfo.java:96)
//! and the region timestamps (RegionFile.java:232). Each reads the store's
//! [`Clock`], which the client implements with the wall clock and tests with
//! [`FixedClock`].
//!
//! # Not ported
//!
//! - The pre-McRegion format: `ChunkLoader`'s per-chunk files
//!   (`chunkFileForXZ`, `loadChunk`, `saveChunk`, ChunkLoader.java:18-106),
//!   which `SaveHandler.getChunkLoader` and `SaveFormatOld.getSaveLoader`
//!   hand out, and the conversion `SaveConverterMcRegion.convertMapFormat`
//!   (SaveConverterMcRegion.java:62-171): deferred (PLAN_v4 D3).

pub mod chunk_loader;
pub mod entity_list;
pub mod import;
pub mod mcregion_chunk_loader;
pub mod nbt;
pub mod region;
pub mod region_cache;
pub mod save_format_old;
pub mod save_handler;
pub mod storage;

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

pub use chunk_loader::LoadedChunk;
pub use mc_nbt::{self, Tag, TagType};
pub use mcregion_chunk_loader::McRegionChunkLoader;
pub use region::{
    ChunkDataOutputStream, RegionError, RegionFile, RegionFileChunkBuffer, RegionWrite,
};
pub use region_cache::{RegionFileCache, region_file_path};
pub use save_format_old::{SaveConverterMcRegion, SaveFormat, SaveFormatComparator, SaveFormatOld};
pub use save_handler::SaveOldDir;
pub use storage::{MemoryStorage, Storage, StorageError};

use crate::item::JavaException;
use crate::world::World;
use crate::world::chunk::Chunk;
use crate::world::info::WorldInfo;
use crate::world::provider::WorldProvider;

/// Why a save operation failed. The Java code mostly catches its exceptions
/// and prints them; the port returns them so the caller can do the same or
/// show them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveError {
    /// The [`Storage`] backend failed (Java: an `IOException` from `File` I/O).
    Storage(StorageError),
    /// The region container failed ([`RegionError`]).
    Region(RegionError),
    /// NBT decoding or encoding failed (Java: `IOException` /
    /// `ZipException` from `CompressedStreamTools`).
    Nbt(mc_nbt::Error),
    /// A Java runtime exception the vanilla code throws here: the class's
    /// simple name and a detail.
    Java {
        /// e.g. `"ClassCastException"`.
        exception: &'static str,
        /// What was being read.
        detail: String,
    },
    /// `MinecraftException` (MinecraftException.java): the session lock was
    /// taken by another process, or could not be read.
    Minecraft(String),
    /// Data the Java would accept and only fail on later (an array of the
    /// wrong length); the port refuses it when reading.
    Corrupt(String),
    /// Out of scope (PLAN_v4 D3): the pre-McRegion format; names the Java
    /// method.
    Deferred(&'static str),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveError::Storage(e) => write!(f, "{e}"),
            SaveError::Region(e) => write!(f, "{e}"),
            SaveError::Nbt(e) => write!(f, "NBT: {e:?}"),
            SaveError::Java { exception, detail } => write!(f, "{exception}: {detail}"),
            SaveError::Minecraft(m) => write!(f, "MinecraftException: {m}"),
            SaveError::Corrupt(m) => write!(f, "corrupt save: {m}"),
            SaveError::Deferred(m) => write!(f, "not supported: {m}"),
        }
    }
}

impl std::error::Error for SaveError {}

impl From<StorageError> for SaveError {
    fn from(e: StorageError) -> Self {
        SaveError::Storage(e)
    }
}

impl From<RegionError> for SaveError {
    fn from(e: RegionError) -> Self {
        SaveError::Region(e)
    }
}

impl From<mc_nbt::Error> for SaveError {
    fn from(e: mc_nbt::Error) -> Self {
        SaveError::Nbt(e)
    }
}

impl From<JavaException> for SaveError {
    fn from(e: JavaException) -> Self {
        SaveError::Java {
            exception: e.simple_name(),
            detail: String::new(),
        }
    }
}

/// `System.currentTimeMillis()` for the save code (module docs, "Time").
pub trait Clock {
    /// Milliseconds since the epoch.
    fn current_time_millis(&mut self) -> i64;
}

/// A [`Clock`] that always reads the same time (tests, headless runs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedClock(pub i64);

impl Clock for FixedClock {
    fn current_time_millis(&mut self) -> i64 {
        self.0
    }
}

/// What the save code shares (module docs, "Sharing the storage").
pub struct SaveStore {
    /// The bytes.
    pub storage: Box<dyn Storage>,
    /// `RegionFileCache`'s static map, per store.
    pub region_cache: RegionFileCache,
    /// `System.currentTimeMillis()`.
    pub clock: Box<dyn Clock>,
}

impl fmt::Debug for SaveStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SaveStore")
            .field("open_regions", &self.region_cache.len())
            .finish_non_exhaustive()
    }
}

impl SaveStore {
    /// A store over `storage` with an empty region cache.
    pub fn new(storage: Box<dyn Storage>, clock: Box<dyn Clock>) -> Self {
        SaveStore {
            storage,
            region_cache: RegionFileCache::new(),
            clock,
        }
    }

    /// The store, ready to share.
    pub fn shared(storage: Box<dyn Storage>, clock: Box<dyn Clock>) -> SharedSaveStore {
        Rc::new(RefCell::new(SaveStore::new(storage, clock)))
    }

    /// Whether a blob exists at `path` (`File.exists()` for a file).
    pub fn exists(&mut self, path: &str) -> Result<bool, StorageError> {
        Ok(self.storage.read(path)?.is_some())
    }

    /// Whether `path` is a directory with something in it (`File.exists()`
    /// for a directory; [`Storage`] has no empty directories).
    pub fn dir_exists(&mut self, path: &str) -> Result<bool, StorageError> {
        Ok(!self.storage.list(path)?.is_empty())
    }
}

/// A [`SaveStore`] shared between a save handler, its loaders and its save
/// format.
pub type SharedSaveStore = Rc<RefCell<SaveStore>>;

/// `new File(dir, name)` as a storage path: `name` alone for the root
/// (`dir` empty).
pub fn path_join(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_string()
    } else {
        format!("{dir}/{name}")
    }
}

/// `IChunkLoader` (IChunkLoader.java): reads and writes chunks.
///
/// The loader lives inside the world (`ChunkProvider.chunkLoader`); the
/// world's chunk provider takes it out for each call, so the loader gets the
/// whole world, as the Java's `World` parameter gives it.
pub trait ChunkLoader {
    /// `loadChunk(world, x, z)`: the stored chunk and the entities read with
    /// it (in the order the chunk's sections list them), or `None`. The chunk
    /// is not in the world's chunk map yet; its tile entities are in
    /// [`World::tile_entities`] and its map, its entities are not in
    /// `loadedEntityList` (the caller adds them, `World.addLoadedEntities`
    /// via `Chunk.onChunkLoad`). `Err` where the Java throws out of the
    /// method (`ChunkProvider.loadChunkFromFile` prints it and returns null,
    /// ChunkProvider.java:88-91).
    fn load_chunk(
        &mut self,
        world: &mut World,
        x: i32,
        z: i32,
    ) -> Result<Option<LoadedChunk>, SaveError>;
    /// `saveChunk(world, chunk)`. The chunk is out of the world's chunk map
    /// during the call. `Err` only where the Java throws out of the method.
    fn save_chunk(&mut self, world: &mut World, chunk: &mut Chunk) -> Result<(), SaveError>;
    /// `saveExtraChunkData(world, chunk)`.
    fn save_extra_chunk_data(
        &mut self,
        world: &mut World,
        chunk: &mut Chunk,
    ) -> Result<(), SaveError>;
    /// `chunkTick()`.
    fn chunk_tick(&mut self);
    /// `saveExtraData()`.
    fn save_extra_data(&mut self);
}

/// `ISaveHandler` (ISaveHandler.java). `getMapFile(name)` (the map item
/// data file, read and written only by `MapStorage`) is the three
/// `*map_file*` methods: the file itself is never handed out, its bytes go
/// through the handler's [`Storage`] (WP-2.6, [`crate::world::map_storage`]).
pub trait SaveHandler {
    /// `loadWorldInfo()`: `None` for a new world.
    fn load_world_info(&mut self) -> Option<WorldInfo>;
    /// `checkSessionLock()`. Vanilla throws `MinecraftException`
    /// ([`SaveError::Minecraft`]) when another process took the lock.
    fn check_session_lock(&mut self) -> Result<(), SaveError>;
    /// `getChunkLoader(provider)`.
    fn get_chunk_loader(&mut self, provider: &WorldProvider) -> Option<Box<dyn ChunkLoader>>;
    /// `saveWorldInfoAndPlayer(info, players)`. The player list is reduced to
    /// the first player's `writeToNBT` compound, which is all
    /// `WorldInfo.getNBTTagCompoundWithPlayer` uses (WorldInfo.java:72-87).
    /// `info` is mutable because `SaveOldDir` sets its save version
    /// (SaveOldDir.java:22-25). Vanilla prints failures; they are returned.
    fn save_world_info_and_player(
        &mut self,
        info: &mut WorldInfo,
        player: Option<Tag>,
    ) -> Result<(), SaveError>;
    /// `saveWorldInfo(info)`.
    fn save_world_info(&mut self, info: &WorldInfo) -> Result<(), SaveError>;
    /// `getMapFile(name) != null`: whether the handler has a map data
    /// directory. False (`SaveHandlerMP`, SaveHandlerMP.java:24-26) means
    /// `MapStorage` reads and writes nothing.
    ///
    /// The three map-file methods have no default body, so a wrapping
    /// handler (the golden harness's) cannot silently lose them.
    fn has_map_files(&self) -> bool;
    /// The bytes of `getMapFile(name)` (`data/<name>.dat`), `Ok(None)` when
    /// the file does not exist (`File.exists()` false) or there is no map
    /// directory.
    fn read_map_file(&mut self, name: &str) -> Result<Option<Vec<u8>>, SaveError>;
    /// Writes `getMapFile(name)` (a `FileOutputStream` over it); nothing
    /// without a map directory.
    fn write_map_file(&mut self, name: &str, bytes: &[u8]) -> Result<(), SaveError>;
}

/// `SaveHandlerMP` (SaveHandlerMP.java): saves nothing, loads nothing.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoSave;

impl SaveHandler for NoSave {
    // src: SaveHandlerMP.java:6-8
    fn load_world_info(&mut self) -> Option<WorldInfo> {
        None
    }
    // src: SaveHandlerMP.java:10-11
    fn check_session_lock(&mut self) -> Result<(), SaveError> {
        Ok(())
    }
    // src: SaveHandlerMP.java:13-15
    fn get_chunk_loader(&mut self, _provider: &WorldProvider) -> Option<Box<dyn ChunkLoader>> {
        None
    }
    // src: SaveHandlerMP.java:17-18
    fn save_world_info_and_player(
        &mut self,
        _info: &mut WorldInfo,
        _player: Option<Tag>,
    ) -> Result<(), SaveError> {
        Ok(())
    }
    // src: SaveHandlerMP.java:20-21
    fn save_world_info(&mut self, _info: &WorldInfo) -> Result<(), SaveError> {
        Ok(())
    }
    // src: SaveHandlerMP.java:24-26 (getMapFile returns null)
    fn has_map_files(&self) -> bool {
        false
    }
    // src: SaveHandlerMP.java:24-26
    fn read_map_file(&mut self, _name: &str) -> Result<Option<Vec<u8>>, SaveError> {
        Ok(None)
    }
    // src: SaveHandlerMP.java:24-26
    fn write_map_file(&mut self, _name: &str, _bytes: &[u8]) -> Result<(), SaveError> {
        Ok(())
    }
}
