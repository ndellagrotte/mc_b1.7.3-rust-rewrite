//! Chunk providers: `IChunkProvider` (the generator interface), the vanilla
//! caching `ChunkProvider` and the empty-world generator.
//!
//! # The two layers
//!
//! Vanilla `World.chunkProvider` is a `ChunkProvider` (ChunkProvider.java), a
//! cache that loads chunks from the `IChunkLoader`, otherwise asks a generator
//! (`ChunkProviderGenerate`, `ChunkProviderHell`, both `IChunkProvider`s too)
//! and triggers population when the 2×2 square is present. Here:
//!
//! - [`ChunkProvider`] is the generator side of `IChunkProvider`;
//!   [`ChunkGenerator`] is the closed set of generators (dispatch by `match`,
//!   PLAN_v4 D1 rule 5).
//! - [`ChunkProviderCache`] holds the cache's fields; its methods are `World`
//!   methods ([`World::get_chunk_from_chunk_coords`], [`World::chunk_exists`], …), because
//!   loading a chunk runs `Chunk.onChunkLoad` and population, which need the
//!   whole world.
//!
//! # Re-entrancy
//!
//! `provideChunk` of a generator never loads other chunks, so the cache takes
//! the generator out of the world for the call (`Option::take`) and passes it
//! `&mut World`. `populate` *does* load chunks (a populator writing next to the
//! 3×3 area generates more), and a nested `provideChunk` reseeds the same
//! generator's `Random` (the nether quirk in `tests/golden/README.md`). So
//! population is not a `&mut self` method: [`ChunkGenerator::populate`] is a
//! function over the world that borrows the generator only for each short
//! access. The empty generator's `populate` is a no-op.
//!
//! # Key aliasing (vanilla bug, transcribed)
//!
//! `chunkMap` is keyed by `ChunkCoordIntPair.chunkXZ2Int(x, z)`
//! (ChunkProvider.java:28, :45), which keeps 15 bits per axis, so chunks 32768
//! apart share a key and replace each other. The map here uses the same key.
//! `chunk_list` holds keys where Java holds `Chunk` objects
//! (ChunkProvider.java:46,133-134), so with `chunkXZ2Int` aliasing Java saves
//! both aliased chunks while this port saves the current one twice.

use std::collections::BTreeMap;

use mc_core::java_hash::{ChunkCoordIntPair, JavaHashSet};

use super::World;
use super::chunk::{self, CHUNK_VOLUME, Chunk};
use crate::save::ChunkLoader;

/// The generator half of `IChunkProvider` (IChunkProvider.java). The cache
/// half (`provideChunk` with loading, `saveChunks`, …) is on [`World`].
pub trait ChunkProvider {
    /// `provideChunk(x, z)` of a generator: a new, unpopulated chunk. The
    /// generator is outside the world during the call, so `world` is freely
    /// usable but must not load chunks.
    fn provide_chunk(&mut self, world: &mut World, x: i32, z: i32) -> Chunk;

    /// `chunkExists(x, z)` of a generator (always true for vanilla generators).
    fn chunk_exists(&self, x: i32, z: i32) -> bool;

    /// `unload100OldestChunks()` of a generator (always false for vanilla generators).
    fn unload_100_oldest_chunks(&mut self) -> bool;

    /// `canSave()`.
    fn can_save(&self) -> bool;

    /// `makeString()`.
    fn make_string(&self) -> String;
}

/// The oracle's `EmptyChunkProvider` (tools/oracle/java/harness/EmptyChunkProvider.java):
/// every chunk is air; mirrors `ChunkProviderGenerate.provideChunk` with no
/// terrain: `new Chunk(world, new byte[32768], x, z)` then
/// `generateSkylightMap()` (height map 0, sky light 15 for y = 127..1, 0 at y = 0).
#[derive(Clone, Copy, Debug, Default)]
pub struct EmptyChunkProvider;

impl ChunkProvider for EmptyChunkProvider {
    fn provide_chunk(&mut self, world: &mut World, x: i32, z: i32) -> Chunk {
        let mut c = Chunk::new(Box::new([0u8; CHUNK_VOLUME]), x, z);
        chunk::generate_skylight_map(world, &mut c);
        c
    }

    fn chunk_exists(&self, _x: i32, _z: i32) -> bool {
        true
    }

    fn unload_100_oldest_chunks(&mut self) -> bool {
        false
    }

    fn can_save(&self) -> bool {
        true
    }

    fn make_string(&self) -> String {
        "HarnessEmptyLevelSource".to_string()
    }
}

/// The generators a world can use. `ChunkProviderGenerate` and
/// `ChunkProviderHell` join in Phase 1 (WP-1.4, WP-1.7).
#[derive(Debug)]
pub enum ChunkGenerator {
    /// [`EmptyChunkProvider`].
    Empty(EmptyChunkProvider),
}

impl ChunkGenerator {
    fn as_provider(&mut self) -> &mut dyn ChunkProvider {
        match self {
            ChunkGenerator::Empty(g) => g,
        }
    }

    /// `populate(provider, x, z)` of the generator. See the module docs for
    /// why this takes the world rather than `&mut self`.
    pub fn populate(world: &mut World, x: i32, z: i32) {
        let _ = (x, z);
        match world.chunk_provider.generator.as_ref() {
            Some(ChunkGenerator::Empty(_)) => {}
            None => panic!("ChunkGenerator::populate re-entered during provide_chunk"),
        }
    }
}

/// The fields of vanilla's caching `ChunkProvider` (ChunkProvider.java:11-18).
#[derive(Debug)]
pub struct ChunkProviderCache {
    /// `droppedChunksSet`: chunk keys queued for unloading. Iterated, so Java
    /// order; nothing in the singleplayer client ever adds to it.
    pub dropped_chunks_set: JavaHashSet<i32>,
    // `emptyChunk` (ChunkProvider.java:13, used only when there is no
    // generator): every world here has a generator.
    /// `chunkProvider`: the generator; `None` only while it is lent out
    /// (module docs).
    pub generator: Option<ChunkGenerator>,
    /// `chunkLoader` (`None` for `SaveHandlerMP`).
    pub chunk_loader: Option<Box<dyn ChunkLoader>>,
    /// `chunkMap`, keyed by `chunkXZ2Int` (module docs). Lookup only, so a
    /// `BTreeMap`.
    pub chunk_map: BTreeMap<i32, Chunk>,
    /// `chunkList`: load order, used by `saveChunks`. Holds chunk keys.
    pub chunk_list: Vec<i32>,
}

impl std::fmt::Debug for dyn ChunkLoader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ChunkLoader")
    }
}

impl ChunkProviderCache {
    /// `new ChunkProvider(world, loader, generator)`.
    // src: ChunkProvider.java:20-25
    pub fn new(chunk_loader: Option<Box<dyn ChunkLoader>>, generator: ChunkGenerator) -> Self {
        ChunkProviderCache {
            dropped_chunks_set: JavaHashSet::new(),
            generator: Some(generator),
            chunk_loader,
            chunk_map: BTreeMap::new(),
            chunk_list: Vec::new(),
        }
    }
}

/// `ChunkCoordIntPair.chunkXZ2Int`, the cache key.
pub fn chunk_key(x: i32, z: i32) -> i32 {
    ChunkCoordIntPair::chunk_xz_to_int(x, z)
}

impl World {
    /// `chunkProvider.chunkExists(x, z)`: is the chunk in the cache.
    // src: ChunkProvider.java:27-29
    pub fn chunk_exists(&self, x: i32, z: i32) -> bool {
        self.chunk_provider.chunk_map.contains_key(&chunk_key(x, z))
    }

    /// The loaded chunk at `(x, z)` (by cache key, so aliasing applies), or
    /// `None`. Never loads.
    pub fn loaded_chunk(&self, x: i32, z: i32) -> Option<&Chunk> {
        self.chunk_provider.chunk_map.get(&chunk_key(x, z))
    }

    /// The chunk at `(x, z)`, which must be loaded (used right after
    /// `provideChunk` or a `chunkExists` check).
    pub(crate) fn chunk_mut(&mut self, x: i32, z: i32) -> &mut Chunk {
        self.chunk_provider
            .chunk_map
            .get_mut(&chunk_key(x, z))
            .expect("chunk_mut on an unloaded chunk")
    }

    /// `chunkProvider.provideChunk(x, z)` = `World.getChunkFromChunkCoords`:
    /// the cached chunk, loading or generating it first if needed.
    // src: ChunkProvider.java:72-75, World.java:366-368
    pub fn get_chunk_from_chunk_coords(&mut self, x: i32, z: i32) -> &mut Chunk {
        if !self.chunk_exists(x, z) {
            self.prepare_chunk(x, z);
        }
        self.chunk_mut(x, z)
    }

    /// `getChunkFromBlockCoords(x, z)`.
    // src: World.java:362-364
    pub fn get_chunk_from_block_coords(&mut self, x: i32, z: i32) -> &mut Chunk {
        self.get_chunk_from_chunk_coords(x >> 4, z >> 4)
    }

    /// `ChunkProvider.prepareChunk(x, z)`: load or generate, register, run
    /// `onChunkLoad`, then populate this chunk and/or its -x/-z neighbours
    /// whose 2×2 square is now complete.
    // src: ChunkProvider.java:31-70
    pub fn prepare_chunk(&mut self, x: i32, z: i32) {
        let var3 = chunk_key(x, z);
        self.chunk_provider.dropped_chunks_set.remove(&var3);
        if self.chunk_provider.chunk_map.contains_key(&var3) {
            return;
        }
        let var4 = match self.load_chunk_from_file(x, z) {
            Some(c) => c,
            None => {
                let mut generator = self
                    .chunk_provider
                    .generator
                    .take()
                    .expect("provide_chunk re-entered while the generator is lent out");
                let c = generator.as_provider().provide_chunk(self, x, z);
                self.chunk_provider.generator = Some(generator);
                c
            }
        };
        self.chunk_provider.chunk_map.insert(var3, var4);
        self.chunk_provider.chunk_list.push(var3);
        // var4.func_4143_d() is empty (Chunk.java:133-134).
        chunk::on_chunk_load(self, x, z);

        if !self.chunk_mut(x, z).is_terrain_populated
            && self.chunk_exists(x + 1, z + 1)
            && self.chunk_exists(x, z + 1)
            && self.chunk_exists(x + 1, z)
        {
            self.populate_chunk(x, z);
        }
        if self.chunk_exists(x - 1, z)
            && !self
                .get_chunk_from_chunk_coords(x - 1, z)
                .is_terrain_populated
            && self.chunk_exists(x - 1, z + 1)
            && self.chunk_exists(x, z + 1)
            && self.chunk_exists(x - 1, z)
        {
            self.populate_chunk(x - 1, z);
        }
        if self.chunk_exists(x, z - 1)
            && !self
                .get_chunk_from_chunk_coords(x, z - 1)
                .is_terrain_populated
            && self.chunk_exists(x + 1, z - 1)
            && self.chunk_exists(x, z - 1)
            && self.chunk_exists(x + 1, z)
        {
            self.populate_chunk(x, z - 1);
        }
        if self.chunk_exists(x - 1, z - 1)
            && !self
                .get_chunk_from_chunk_coords(x - 1, z - 1)
                .is_terrain_populated
            && self.chunk_exists(x - 1, z - 1)
            && self.chunk_exists(x, z - 1)
            && self.chunk_exists(x - 1, z)
        {
            self.populate_chunk(x - 1, z - 1);
        }
    }

    /// `ChunkProvider.loadChunkFromFile(x, z)`.
    // src: ChunkProvider.java:77-93
    fn load_chunk_from_file(&mut self, x: i32, z: i32) -> Option<Chunk> {
        let loader = self.chunk_provider.chunk_loader.as_mut()?;
        // Vanilla prints the exception and returns null on failure.
        let mut c = loader.load_chunk(x, z).ok().flatten()?;
        c.last_save_time = self.world_info.world_time;
        Some(c)
    }

    /// `ChunkProvider.populate(provider, x, z)`: marks the chunk populated, then
    /// runs the generator's population.
    // src: ChunkProvider.java:118-128
    pub fn populate_chunk(&mut self, x: i32, z: i32) {
        let c = self.get_chunk_from_chunk_coords(x, z);
        if !c.is_terrain_populated {
            c.is_terrain_populated = true;
            ChunkGenerator::populate(self, x, z);
            self.chunk_mut(x, z).set_chunk_modified();
        }
    }

    /// `ChunkProvider.saveChunks(all, progress)`.
    // src: ChunkProvider.java:130-157
    pub fn save_chunks(&mut self, all: bool) -> bool {
        let mut var3 = 0;
        let keys = self.chunk_provider.chunk_list.clone();
        let world_time = self.world_info.world_time;
        for key in keys {
            let Some(c) = self.chunk_provider.chunk_map.get_mut(&key) else {
                continue;
            };
            if all
                && !c.never_save
                && let Some(l) = self.chunk_provider.chunk_loader.as_mut()
            {
                let _ = l.save_extra_chunk_data(c);
            }
            if c.needs_saving(all, world_time) {
                if let Some(l) = self.chunk_provider.chunk_loader.as_mut() {
                    c.last_save_time = world_time;
                    let _ = l.save_chunk(c);
                }
                c.is_modified = false;
                var3 += 1;
                if var3 == 24 && !all {
                    return false;
                }
            }
        }
        if all {
            match self.chunk_provider.chunk_loader.as_mut() {
                None => return true,
                Some(l) => l.save_extra_data(),
            }
        }
        true
    }

    /// `ChunkProvider.unload100OldestChunks()`.
    // src: ChunkProvider.java:159-178
    pub fn unload_100_oldest_chunks(&mut self) -> bool {
        for _ in 0..100 {
            let next = self
                .chunk_provider
                .dropped_chunks_set
                .iter()
                .next()
                .copied();
            if let Some(var2) = next {
                if let Some((cx, cz)) = self
                    .chunk_provider
                    .chunk_map
                    .get(&var2)
                    .map(|c| (c.x_position, c.z_position))
                {
                    chunk::on_chunk_unload(self, cx, cz);
                    let world_time = self.world_info.world_time;
                    if let Some(c) = self.chunk_provider.chunk_map.get_mut(&var2)
                        && let Some(l) = self.chunk_provider.chunk_loader.as_mut()
                    {
                        c.last_save_time = world_time;
                        let _ = l.save_chunk(c);
                        let _ = l.save_extra_chunk_data(c);
                    }
                }
                self.chunk_provider.dropped_chunks_set.remove(&var2);
                self.chunk_provider.chunk_map.remove(&var2);
                // `chunkList.remove(chunk)`: the first occurrence.
                if let Some(i) = self
                    .chunk_provider
                    .chunk_list
                    .iter()
                    .position(|k| *k == var2)
                {
                    self.chunk_provider.chunk_list.remove(i);
                }
            }
        }
        if let Some(l) = self.chunk_provider.chunk_loader.as_mut() {
            l.chunk_tick();
        }
        match self.chunk_provider.generator.as_mut() {
            Some(g) => g.as_provider().unload_100_oldest_chunks(),
            None => false,
        }
    }

    /// `ChunkProvider.canSave()`: always true.
    // src: ChunkProvider.java:180-182
    pub fn chunk_provider_can_save(&self) -> bool {
        true
    }
}
