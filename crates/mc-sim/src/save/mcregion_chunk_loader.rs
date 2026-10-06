//! `McRegionChunkLoader` (McRegionChunkLoader.java): the [`ChunkLoader`] of
//! McRegion saves, over the [`SaveStore`]'s region cache.
//!
//! Besides the Java's one field (`worldDir`), the port's loader keeps the
//! stored compounds of entities it could not construct, per chunk, so a
//! re-save writes them back ([`super::entity_list`]). They are replaced
//! whenever the chunk is read again.
//!
//! Errors: `loadChunk` lets the region and NBT exceptions out (the Java's
//! `IOException`s, caught by `ChunkProvider.loadChunkFromFile`,
//! ChunkProvider.java:88-91); `saveChunk` catches everything after its
//! first session-lock check and prints it (McRegionChunkLoader.java:55-57):
//! the port keeps it in [`McRegionChunkLoader::last_error`].

use std::collections::BTreeMap;

use mc_nbt::Tag;

use super::chunk_loader::{self, LoadedChunk};
use super::nbt::{self as n};
use super::{ChunkLoader, SaveError, SharedSaveStore};
use crate::world::World;
use crate::world::chunk::Chunk;

/// `McRegionChunkLoader`.
#[derive(Debug)]
pub struct McRegionChunkLoader {
    /// The shared storage, region cache and clock.
    store: SharedSaveStore,
    /// `worldDir`: the storage path of the world (or `DIM-1`) directory.
    world_dir: String,
    /// Stored compounds of entities the port cannot construct yet, by chunk
    /// position (module docs).
    preserved: BTreeMap<(i32, i32), Vec<Tag>>,
    /// The last exception `saveChunk` caught and printed.
    pub last_error: Option<SaveError>,
}

impl McRegionChunkLoader {
    /// `new McRegionChunkLoader(worldDir)`.
    // src: McRegionChunkLoader.java:11-13
    pub fn new(store: SharedSaveStore, world_dir: &str) -> Self {
        McRegionChunkLoader {
            store,
            world_dir: world_dir.to_string(),
            preserved: BTreeMap::new(),
            last_error: None,
        }
    }

    /// `worldDir`.
    pub fn world_dir(&self) -> &str {
        &self.world_dir
    }

    /// The stored entity compounds kept for chunk `(x, z)` (module docs).
    pub fn preserved_entities(&self, x: i32, z: i32) -> &[Tag] {
        self.preserved.get(&(x, z)).map_or(&[], Vec::as_slice)
    }

    /// The chunk's stored NBT (`RegionFileCache.getChunkInputStream` then
    /// `CompressedStreamTools.read`), `None` when the region has no chunk
    /// there.
    // src: McRegionChunkLoader.java:16-18
    pub fn read_chunk_nbt(&mut self, x: i32, z: i32) -> Result<Option<Tag>, SaveError> {
        Ok(self.read_chunk_nbt_named(x, z)?.map(|(_, t)| t))
    }

    /// [`McRegionChunkLoader::read_chunk_nbt`] with the root's name, which
    /// `CompressedStreamTools.read` keeps as the compound's key
    /// (CompressedStreamTools.java:38-45, `NBTBase.readTag`).
    // src: McRegionChunkLoader.java:16-18
    pub fn read_chunk_nbt_named(
        &mut self,
        x: i32,
        z: i32,
    ) -> Result<Option<(String, Tag)>, SaveError> {
        let bytes = {
            let mut s = self.store.borrow_mut();
            let s = &mut *s;
            s.region_cache
                .get_chunk_input_stream(s.storage.as_mut(), &self.world_dir, x, z)?
        };
        match bytes {
            None => Ok(None),
            Some(b) => Ok(Some(mc_nbt::read_named(&b)?)),
        }
    }

    /// The body of `saveChunk` inside its `try` (McRegionChunkLoader.java:46-54).
    fn save_chunk_body(&mut self, world: &mut World, chunk: &mut Chunk) -> Result<(), SaveError> {
        let key = (chunk.x_position, chunk.z_position);
        let preserved = self.preserved.get(&key).cloned().unwrap_or_default();
        // src: McRegionChunkLoader.java:47-50
        let var5 = chunk_loader::store_chunk_in_compound(chunk, world, &preserved)?;
        let mut var4 = n::Compound::new();
        n::set(&mut var4, "Level", Tag::Compound(var5));
        // src: McRegionChunkLoader.java:51 (CompressedStreamTools.write: the root named "")
        let bytes = mc_nbt::write_named("", &Tag::Compound(var4))?;
        let delta = {
            let mut s = self.store.borrow_mut();
            let s = &mut *s;
            let now = s.clock.current_time_millis();
            // src: McRegionChunkLoader.java:46, :52 (getChunkOutputStream, close)
            s.region_cache.write_chunk(
                s.storage.as_mut(),
                &self.world_dir,
                key.0,
                key.1,
                &bytes,
                now,
            )?;
            // src: McRegionChunkLoader.java:54
            s.region_cache
                .get_size_delta(s.storage.as_mut(), &self.world_dir, key.0, key.1)?
        };
        let info = &mut world.world_info;
        info.size_on_disk = info.size_on_disk.wrapping_add(delta as i64);
        Ok(())
    }
}

impl ChunkLoader for McRegionChunkLoader {
    /// `loadChunk(world, x, z)`.
    // src: McRegionChunkLoader.java:15-40
    fn load_chunk(
        &mut self,
        world: &mut World,
        var2: i32,
        var3: i32,
    ) -> Result<Option<LoadedChunk>, SaveError> {
        let Some(var5) = self.read_chunk_nbt(var2, var3)? else {
            return Ok(None);
        };
        let root = n::as_compound(&var5)?;
        if !n::has_key(root, "Level") {
            // "Chunk file at x,z is missing level data, skipping" (stdout)
            return Ok(None);
        }
        let level = n::get_compound_tag(root, "Level")?;
        if !n::has_key(&level, "Blocks") {
            // "Chunk file at x,z is missing block data, skipping" (stdout)
            return Ok(None);
        }
        let mut var6 = chunk_loader::load_chunk_into_world_from_compound(world, &level)?;
        if !var6.chunk.is_at_location(var2, var3) {
            // "Chunk file at x,z is in the wrong location; relocating." (stdout)
            // vanilla bug: `xPos`/`zPos` are set on the root compound, not on
            // `Level` (McRegionChunkLoader.java:29-30), so the second read
            // builds the same misplaced chunk again (constructing its
            // entities and tile entities a second time). The first copy is
            // dropped.
            let again = chunk_loader::load_chunk_into_world_from_compound(world, &level)?;
            chunk_loader::discard(world, std::mem::replace(&mut var6, again));
        }
        chunk_loader::remove_unknown_blocks(&mut var6.chunk); // src: McRegionChunkLoader.java:34
        self.preserved.insert(
            (var6.chunk.x_position, var6.chunk.z_position),
            var6.preserved.clone(),
        );
        Ok(Some(var6))
    }

    /// `saveChunk(world, chunk)`: the session lock check (outside the
    /// `try`, so its `MinecraftException` leaves the method), then the
    /// `Level` compound written through the region cache and the region's
    /// growth added to `sizeOnDisk`. Everything else that fails is caught
    /// and printed by the Java ([`McRegionChunkLoader::last_error`]).
    // src: McRegionChunkLoader.java:42-59
    fn save_chunk(&mut self, world: &mut World, chunk: &mut Chunk) -> Result<(), SaveError> {
        world.check_session_lock()?; // src: McRegionChunkLoader.java:43
        if let Err(e) = self.save_chunk_body(world, chunk) {
            self.last_error = Some(e);
        }
        Ok(())
    }

    // src: McRegionChunkLoader.java:61-62
    fn save_extra_chunk_data(
        &mut self,
        _world: &mut World,
        _chunk: &mut Chunk,
    ) -> Result<(), SaveError> {
        Ok(())
    }

    // src: McRegionChunkLoader.java:64-65
    fn chunk_tick(&mut self) {}

    // src: McRegionChunkLoader.java:67-68
    fn save_extra_data(&mut self) {}
}
