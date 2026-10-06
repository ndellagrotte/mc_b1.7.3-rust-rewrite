//! Saves: the [`Storage`] trait (D9) and the save-handler/chunk-loader
//! interfaces in the shape of `ISaveHandler` and `IChunkLoader`.
//!
//! Implementations come with WP-1.9 (`SaveOldDir`, `SaveFormatOld`,
//! `McRegionChunkLoader`, `RegionFile`, `ChunkLoader` chunk NBT, `WorldInfo`
//! NBT via `mc-nbt`, the D5 id validation for import). Until then the only
//! save handler is [`NoSave`], the port of `SaveHandlerMP`, which is what the
//! oracle's Tier B/C worlds use.

pub mod storage;

pub use storage::{MemoryStorage, Storage, StorageError};

use crate::world::chunk::Chunk;
use crate::world::info::WorldInfo;
use crate::world::provider::WorldProvider;

/// `IChunkLoader` (IChunkLoader.java): reads and writes chunks.
pub trait ChunkLoader {
    /// `loadChunk(world, x, z)`: the stored chunk, or `None`.
    fn load_chunk(&mut self, x: i32, z: i32) -> Result<Option<Chunk>, StorageError>;
    /// `saveChunk(world, chunk)`.
    fn save_chunk(&mut self, chunk: &Chunk) -> Result<(), StorageError>;
    /// `saveExtraChunkData(world, chunk)`.
    fn save_extra_chunk_data(&mut self, chunk: &Chunk) -> Result<(), StorageError>;
    /// `chunkTick()`.
    fn chunk_tick(&mut self);
    /// `saveExtraData()`.
    fn save_extra_data(&mut self);
}

/// `ISaveHandler` (ISaveHandler.java). `getMapFile` (map item data) is left
/// out until `MapStorage` is ported (WP-2.6).
pub trait SaveHandler {
    /// `loadWorldInfo()`: `None` for a new world.
    fn load_world_info(&mut self) -> Option<WorldInfo>;
    /// `checkSessionLock()`. Vanilla throws `MinecraftException` when another
    /// process took the lock.
    fn check_session_lock(&mut self) -> Result<(), StorageError>;
    /// `getChunkLoader(provider)`.
    fn get_chunk_loader(&mut self, provider: &WorldProvider) -> Option<Box<dyn ChunkLoader>>;
    /// `saveWorldInfoAndPlayer(info, players)`. The player list is reduced to
    /// the first player's NBT, which is all `WorldInfo.getNBTTagCompoundWithPlayer`
    /// uses (WorldInfo.java:72-87).
    fn save_world_info_and_player(&mut self, info: &WorldInfo, player: Option<mc_nbt::Tag>);
    /// `saveWorldInfo(info)`.
    fn save_world_info(&mut self, info: &WorldInfo);
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
    fn check_session_lock(&mut self) -> Result<(), StorageError> {
        Ok(())
    }
    // src: SaveHandlerMP.java:13-15
    fn get_chunk_loader(&mut self, _provider: &WorldProvider) -> Option<Box<dyn ChunkLoader>> {
        None
    }
    // src: SaveHandlerMP.java:17-18
    fn save_world_info_and_player(&mut self, _info: &WorldInfo, _player: Option<mc_nbt::Tag>) {}
    // src: SaveHandlerMP.java:20-21
    fn save_world_info(&mut self, _info: &WorldInfo) {}
}
