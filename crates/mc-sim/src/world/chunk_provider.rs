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

use mc_core::JavaRandom;
use mc_core::java_hash::{ChunkCoordIntPair, JavaHashSet};

use super::World;
use super::chunk::{self, CHUNK_VOLUME, Chunk};
use crate::entity::EntitySlot;
use crate::save::{ChunkLoader, LoadedChunk, SaveError};
use crate::worldgen::generate::ChunkProviderGenerate;
use crate::worldgen::hell::ChunkProviderHell;
use crate::worldgen::populate;

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

/// The generators a world can use (built by
/// [`super::provider::WorldProvider::get_chunk_provider`] or, for the oracle's
/// empty worlds, directly).
#[derive(Debug)]
pub enum ChunkGenerator {
    /// [`EmptyChunkProvider`].
    Empty(EmptyChunkProvider),
    /// `ChunkProviderGenerate` (the overworld).
    Generate(Box<ChunkProviderGenerate>),
    /// `ChunkProviderHell` (the nether).
    Hell(Box<ChunkProviderHell>),
}

impl ChunkGenerator {
    fn as_provider(&mut self) -> &mut dyn ChunkProvider {
        match self {
            ChunkGenerator::Empty(g) => g,
            ChunkGenerator::Generate(g) => g.as_mut(),
            ChunkGenerator::Hell(g) => g.as_mut(),
        }
    }

    /// `populate(provider, x, z)` of the generator. See the module docs for
    /// why this takes the world rather than `&mut self`: the features draw
    /// from the generator's `Random` through
    /// [`RandomSource::ChunkProviderRand`](crate::block::dispatch::RandomSource::ChunkProviderRand)
    /// ([`ChunkProviderCache::generator_rand`]) one draw at a time, so a chunk
    /// generated in the middle reseeds the same instance.
    // src: ChunkProviderGenerate.java:310-603, ChunkProviderHell.java:302-359
    pub fn populate(world: &mut World, x: i32, z: i32) {
        match world.chunk_provider.generator.as_ref() {
            Some(ChunkGenerator::Empty(_)) => {}
            Some(ChunkGenerator::Generate(_)) => populate::populate_generate(world, x, z),
            Some(ChunkGenerator::Hell(_)) => populate::populate_hell(world, x, z),
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
    /// How many `loadChunkFromFile` calls failed (Java prints the exception
    /// and generates the chunk, ChunkProvider.java:88-91).
    pub load_errors: u64,
}

impl std::fmt::Debug for dyn ChunkLoader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ChunkLoader")
    }
}

impl ChunkProviderCache {
    /// The terrain generator's own `Random` (`ChunkProviderGenerate.rand`,
    /// `ChunkProviderHell.hellRNG`), which
    /// [`World::random`]`(RandomSource::ChunkProviderRand)` returns. Only
    /// population draws it, and population never runs while the generator is
    /// lent out to `provide_chunk`; the empty generator has none (its
    /// `populate` does nothing).
    // src: ChunkProviderGenerate.java:6, ChunkProviderHell.java:6
    pub fn generator_rand(&mut self) -> &mut JavaRandom {
        match self.generator.as_mut() {
            Some(ChunkGenerator::Generate(g)) => &mut g.rand,
            Some(ChunkGenerator::Hell(g)) => &mut g.hell_rng,
            Some(ChunkGenerator::Empty(_)) => {
                panic!("the empty chunk generator has no Random")
            }
            None => panic!("generator_rand while the generator is lent out to provide_chunk"),
        }
    }

    /// `new ChunkProvider(world, loader, generator)`.
    // src: ChunkProvider.java:20-25
    pub fn new(chunk_loader: Option<Box<dyn ChunkLoader>>, generator: ChunkGenerator) -> Self {
        ChunkProviderCache {
            dropped_chunks_set: JavaHashSet::new(),
            generator: Some(generator),
            chunk_loader,
            chunk_map: BTreeMap::new(),
            chunk_list: Vec::new(),
            load_errors: 0,
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

    /// How many chunk loads failed and fell back to the generator
    /// ([`ChunkProviderCache::load_errors`]).
    pub fn chunk_load_errors(&self) -> u64 {
        self.chunk_provider.load_errors
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
        let (var4, loaded_entities) = match self.load_chunk_from_file(x, z) {
            Some(l) => (l.chunk, l.entities),
            None => {
                let mut generator = self
                    .chunk_provider
                    .generator
                    .take()
                    .expect("provide_chunk re-entered while the generator is lent out");
                let c = generator.as_provider().provide_chunk(self, x, z);
                self.chunk_provider.generator = Some(generator);
                (c, Vec::new())
            }
        };
        self.chunk_provider.chunk_map.insert(var3, var4);
        self.chunk_provider.chunk_list.push(var3);
        // var4.func_4143_d() is empty (Chunk.java:133-134).
        // The entities read with the chunk join `loadedEntityList` here, in
        // the chunk's section order: this is the `loadedEntityList.addAll`
        // half of `World.addLoadedEntities` (World.java:2077-2084), which
        // `onChunkLoad` reaches after `addTileEntity` (Chunk.java:459-467);
        // the port's entities have to be owned by the list before
        // `add_loaded_entities` (the `obtainEntitySkin` half) sees their ids.
        // The two lists are separate, so doing it first is not observable.
        for e in loaded_entities {
            self.loaded_entity_list.push(EntitySlot {
                entity_id: e.base.entity_id,
                entity: Some(Box::new(e)),
            });
        }
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

    /// `ChunkProvider.loadChunkFromFile(x, z)`: the loader's chunk (with
    /// its entities), `lastSaveTime` set to the world time. The loader is
    /// taken out of the provider for the call (it gets the whole world,
    /// [`ChunkLoader`]); a loader error is printed in Java and means "no
    /// chunk".
    // src: ChunkProvider.java:77-93
    fn load_chunk_from_file(&mut self, x: i32, z: i32) -> Option<LoadedChunk> {
        let mut loader = self.chunk_provider.chunk_loader.take()?;
        let r = loader.load_chunk(self, x, z);
        self.chunk_provider.chunk_loader = Some(loader);
        match r {
            Ok(Some(mut l)) => {
                l.chunk.last_save_time = self.world_info.world_time;
                Some(l)
            }
            Ok(None) => None,
            Err(_) => {
                // src: ChunkProvider.java:88-91 (printStackTrace, null)
                self.chunk_provider.load_errors += 1;
                None
            }
        }
    }

    /// Runs `f` with the chunk at `key` taken out of the chunk map and the
    /// loader taken out of the provider (both put back after), so the loader
    /// gets `&mut World` and `&mut Chunk` at once ([`ChunkLoader`]).
    fn with_loader_and_chunk<R>(
        &mut self,
        loader: &mut Box<dyn ChunkLoader>,
        key: i32,
        f: impl FnOnce(&mut Box<dyn ChunkLoader>, &mut World, &mut Chunk) -> R,
    ) -> Option<R> {
        let mut c = self.chunk_provider.chunk_map.remove(&key)?;
        let r = f(loader, self, &mut c);
        self.chunk_provider.chunk_map.insert(key, c);
        Some(r)
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

    /// `ChunkProvider.saveChunks(all, progress)`; see
    /// [`World::try_save_chunks`]. A session-lock failure (an exception in
    /// Java) returns false and is kept in [`World::session_lock_error`].
    // src: ChunkProvider.java:130-158
    pub fn save_chunks(&mut self, all: bool) -> bool {
        match self.try_save_chunks(all) {
            Ok(b) => b,
            Err(e) => {
                self.session_lock_error = Some(e);
                false
            }
        }
    }

    /// `ChunkProvider.saveChunks(all, progress)`: every chunk in load order
    /// whose `needsSaving(all)` holds (at most 24 unless `all`), through the
    /// loader (`unloadAndSaveChunk`; `unloadAndSaveChunkData` first when
    /// `all`). `Err` for the loader's `MinecraftException` (the session
    /// lock), which leaves the Java method too (only `IOException` is caught,
    /// ChunkProvider.java:111-113).
    // src: ChunkProvider.java:95-116, :130-158
    pub fn try_save_chunks(&mut self, all: bool) -> Result<bool, SaveError> {
        let mut loader = self.chunk_provider.chunk_loader.take();
        let r = self.save_chunks_with(&mut loader, all);
        self.chunk_provider.chunk_loader = loader;
        r
    }

    fn save_chunks_with(
        &mut self,
        loader: &mut Option<Box<dyn ChunkLoader>>,
        all: bool,
    ) -> Result<bool, SaveError> {
        let mut var3 = 0;
        let keys = self.chunk_provider.chunk_list.clone();
        for key in keys {
            let Some(c) = self.chunk_provider.chunk_map.get(&key) else {
                continue;
            };
            let never_save = c.never_save;
            // src: ChunkProvider.java:135-137, :95-104 (exceptions printed)
            if all
                && !never_save
                && let Some(l) = loader.as_mut()
            {
                let _ = self.with_loader_and_chunk(l, key, |l, w, c| l.save_extra_chunk_data(w, c));
            }
            let world_time = self.world_info.world_time;
            let needs = self
                .chunk_provider
                .chunk_map
                .get(&key)
                .is_some_and(|c| c.needs_saving(all, world_time));
            if needs {
                // src: ChunkProvider.java:139-140, :106-116
                if let Some(l) = loader.as_mut() {
                    let r = self.with_loader_and_chunk(l, key, |l, w, c| {
                        c.last_save_time = w.world_info.world_time;
                        l.save_chunk(w, c)
                    });
                    if let Some(Err(e)) = r {
                        return Err(e);
                    }
                }
                if let Some(c) = self.chunk_provider.chunk_map.get_mut(&key) {
                    c.is_modified = false; // src: ChunkProvider.java:141
                }
                var3 += 1;
                if var3 == 24 && !all {
                    return Ok(false);
                }
            }
        }
        if all {
            match loader.as_mut() {
                None => return Ok(true),
                Some(l) => l.save_extra_data(),
            }
        }
        Ok(true)
    }

    /// `ChunkProvider.unload100OldestChunks()`; see
    /// [`World::try_unload_100_oldest_chunks`]. A session-lock failure (an
    /// exception in Java) returns false and is kept in
    /// [`World::session_lock_error`].
    // src: ChunkProvider.java:159-178
    pub fn unload_100_oldest_chunks(&mut self) -> bool {
        match self.try_unload_100_oldest_chunks() {
            Ok(b) => b,
            Err(e) => {
                self.session_lock_error = Some(e);
                false
            }
        }
    }

    /// `ChunkProvider.unload100OldestChunks()`: up to 100 dropped chunks
    /// unloaded and saved, then the loader's `chunkTick` and the generator's
    /// own `unload100OldestChunks`. `Err` for the loader's
    /// `MinecraftException` (the session lock), which leaves the Java method
    /// (`unloadAndSaveChunk` catches only `IOException`,
    /// ChunkProvider.java:111-113): the chunk stays in `droppedChunksSet`,
    /// `chunkMap` and `chunkList`, already `onChunkUnload`ed.
    // src: ChunkProvider.java:159-178
    pub fn try_unload_100_oldest_chunks(&mut self) -> Result<bool, SaveError> {
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
                    // src: ChunkProvider.java:166-167 (unloadAndSaveChunk,
                    // unloadAndSaveChunkData; their exceptions are printed,
                    // a MinecraftException leaves the method: the port stops)
                    if let Some(mut l) = self.chunk_provider.chunk_loader.take() {
                        let r = self.with_loader_and_chunk(&mut l, var2, |l, w, c| {
                            c.last_save_time = w.world_info.world_time;
                            let r = l.save_chunk(w, c);
                            // The exception skips unloadAndSaveChunkData
                            // (ChunkProvider.java:166-167).
                            if r.is_ok() {
                                let _ = l.save_extra_chunk_data(w, c);
                            }
                            r
                        });
                        self.chunk_provider.chunk_loader = Some(l);
                        if let Some(Err(e)) = r {
                            return Err(e);
                        }
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
        Ok(match self.chunk_provider.generator.as_mut() {
            Some(g) => g.as_provider().unload_100_oldest_chunks(),
            None => false,
        })
    }

    /// `ChunkProvider.canSave()`: always true.
    // src: ChunkProvider.java:180-182
    pub fn chunk_provider_can_save(&self) -> bool {
        true
    }
}
