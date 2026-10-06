//! `RegionFileCache` (RegionFileCache.java): the open region files, by path.
//!
//! The Java class is all static (one `HashMap<File, SoftReference<RegionFile>>`
//! per process); here it is a value the save handler owns, so two worlds (or
//! two tests) do not share it. Differences, none observable on disk:
//!
//! - **No soft references.** The JVM may clear a `SoftReference` under memory
//!   pressure, after which `func_22193_a` reopens the file. Every write
//!   reaches [`Storage`] before returning (`region.rs`, "Storage model"), so a
//!   reopened region equals the dropped one except for its pending
//!   `sizeDelta`; the port keeps entries until
//!   [`RegionFileCache::clear_region_file_references`].
//! - **Map order.** The Java iterates the `HashMap` only to close every file
//!   (RegionFileCache.java:44-61), which has no visible order effect, so a
//!   `BTreeMap` is used (PLAN_v4 §3.4 rule 4: lookup only).
//! - **No directories.** `region/` is created with `mkdirs()`
//!   (RegionFileCache.java:31-33); [`Storage`] paths need no directories.
//!
//! Callers in the vanilla tree: `McRegionChunkLoader` (`getChunkInputStream`,
//! `getChunkOutputStream`, `getSizeDelta`, McRegionChunkLoader.java:16, :46,
//! :54) and `SaveConverterMcRegion` (`func_22193_a`, `func_22192_a`;
//! pre-McRegion conversion, deferred per D3). Nothing else calls
//! `func_22192_a` (`clearRegionFileReferences`); `ChunkProvider` and
//! `SaveOldDir` do not touch the cache.

use std::collections::BTreeMap;

use super::region::{ChunkDataOutputStream, RegionError, RegionFile, RegionWrite};
use super::storage::Storage;

/// Opening a new region when this many are cached first closes them all.
/// src: RegionFileCache.java:35-37
pub const REGION_CACHE_LIMIT: usize = 256;

/// The storage path of the region file holding chunk `(chunk_x, chunk_z)` of
/// the world saved in `world_dir`: `<world_dir>/region/r.<x>.<z>.mcr` with
/// `x = chunk_x >> 5`, `z = chunk_z >> 5` (RegionFileCache.java:20-21). An
/// empty `world_dir` is the storage root.
pub fn region_file_path(world_dir: &str, chunk_x: i32, chunk_z: i32) -> String {
    let name = format!("region/r.{}.{}.mcr", chunk_x >> 5, chunk_z >> 5);
    if world_dir.is_empty() {
        name
    } else {
        format!("{world_dir}/{name}")
    }
}

/// `RegionFileCache` (RegionFileCache.java:13): open [`RegionFile`]s by
/// storage path.
#[derive(Clone, Debug, Default)]
pub struct RegionFileCache {
    /// `cache`, keyed by the region file's path.
    cache: BTreeMap<String, RegionFile>,
}

impl RegionFileCache {
    /// An empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of open region files.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Whether no region file is open.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// The open region file at `path`, if any.
    pub fn get(&self, path: &str) -> Option<&RegionFile> {
        self.cache.get(path)
    }

    /// `func_22193_a(worldDir, chunkX, chunkZ)` (RegionFileCache.java:19-42):
    /// the cached region file holding the chunk, or a newly opened one
    /// (after closing all open ones when [`REGION_CACHE_LIMIT`] are open).
    /// Opening creates a missing file (`RegionFile::open`).
    pub fn create_or_load_region_file(
        &mut self,
        storage: &mut dyn Storage,
        world_dir: &str,
        chunk_x: i32,
        chunk_z: i32,
    ) -> Result<&mut RegionFile, RegionError> {
        let path = region_file_path(world_dir, chunk_x, chunk_z);
        // src: RegionFileCache.java:22-29
        if self.cache.contains_key(&path) {
            return Ok(self.cache.get_mut(&path).expect("checked by contains_key"));
        }
        // src: RegionFileCache.java:35-37
        if self.cache.len() >= REGION_CACHE_LIMIT {
            self.clear_region_file_references();
        }
        // src: RegionFileCache.java:39-41
        let region = RegionFile::open(storage, &path)?;
        Ok(self.cache.entry(path).or_insert(region))
    }

    /// `func_22192_a()` (RegionFileCache.java:44-61): closes every open
    /// region file and empties the cache.
    pub fn clear_region_file_references(&mut self) {
        for (_, region) in std::mem::take(&mut self.cache) {
            region.close();
        }
    }

    /// `getSizeDelta(worldDir, chunkX, chunkZ)` (RegionFileCache.java:63-66):
    /// the growth of the chunk's region file since the last call.
    pub fn get_size_delta(
        &mut self,
        storage: &mut dyn Storage,
        world_dir: &str,
        chunk_x: i32,
        chunk_z: i32,
    ) -> Result<i32, RegionError> {
        Ok(self
            .create_or_load_region_file(storage, world_dir, chunk_x, chunk_z)?
            .take_size_delta())
    }

    /// `getChunkInputStream(worldDir, chunkX, chunkZ)`
    /// (RegionFileCache.java:68-71): the chunk's decompressed NBT bytes, or
    /// `None` (see [`RegionFile::get_chunk_data_input_stream`]). Opens (and so
    /// creates) the region file even when the chunk is absent, as the Java does.
    pub fn get_chunk_input_stream(
        &mut self,
        storage: &mut dyn Storage,
        world_dir: &str,
        chunk_x: i32,
        chunk_z: i32,
    ) -> Result<Option<Vec<u8>>, RegionError> {
        self.create_or_load_region_file(storage, world_dir, chunk_x, chunk_z)?
            .get_chunk_data_input_stream(chunk_x & 31, chunk_z & 31)
    }

    /// `getChunkOutputStream(worldDir, chunkX, chunkZ)`
    /// (RegionFileCache.java:73-76): a stream for the chunk's uncompressed
    /// NBT; store it with [`Self::close_chunk_output_stream`]. Never `None`
    /// in practice (the local coordinates are masked into range).
    pub fn get_chunk_output_stream(
        &mut self,
        storage: &mut dyn Storage,
        world_dir: &str,
        chunk_x: i32,
        chunk_z: i32,
    ) -> Result<Option<ChunkDataOutputStream>, RegionError> {
        Ok(self
            .create_or_load_region_file(storage, world_dir, chunk_x, chunk_z)?
            .get_chunk_data_output_stream(chunk_x & 31, chunk_z & 31))
    }

    /// The stream's `close()` (McRegionChunkLoader.java:52): compresses and
    /// writes the chunk into the region file the stream came from.
    /// `now_millis` is `System.currentTimeMillis()` (D9: passed in).
    pub fn close_chunk_output_stream(
        &mut self,
        storage: &mut dyn Storage,
        stream: ChunkDataOutputStream,
        now_millis: i64,
    ) -> Result<RegionWrite, RegionError> {
        match self.cache.get_mut(stream.region_path()) {
            Some(region) => stream.close(region, storage, now_millis),
            None => Err(RegionError::StaleStream(stream.region_path().to_string())),
        }
    }

    /// `getChunkOutputStream`, write `nbt` (an uncompressed NBT document),
    /// `close()`: the region part of `McRegionChunkLoader.saveChunk`
    /// (McRegionChunkLoader.java:46-52).
    pub fn write_chunk(
        &mut self,
        storage: &mut dyn Storage,
        world_dir: &str,
        chunk_x: i32,
        chunk_z: i32,
        nbt: &[u8],
        now_millis: i64,
    ) -> Result<RegionWrite, RegionError> {
        let region = self.create_or_load_region_file(storage, world_dir, chunk_x, chunk_z)?;
        let mut stream = region
            .get_chunk_data_output_stream(chunk_x & 31, chunk_z & 31)
            .expect("local chunk coordinates are masked into 0..32");
        std::io::Write::write_all(&mut stream, nbt).expect("in-memory write");
        stream.close(region, storage, now_millis)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths() {
        assert_eq!(region_file_path("w", 0, -4), "w/region/r.0.-1.mcr");
        assert_eq!(region_file_path("w", 31, 32), "w/region/r.0.1.mcr");
        assert_eq!(region_file_path("w", -33, -1), "w/region/r.-2.-1.mcr");
        assert_eq!(region_file_path("", 0, 0), "region/r.0.0.mcr");
        assert_eq!(
            region_file_path("w/DIM-1", 5, 5),
            "w/DIM-1/region/r.0.0.mcr"
        );
    }
}
