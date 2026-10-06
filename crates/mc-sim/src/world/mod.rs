//! The world (`World.java`): chunks, blocks, entities, tile entities,
//! scheduled ticks and the RNGs (PLAN_v4 §3.2).
//!
//! Split by topic: this file has the struct, the constructor and block
//! access; `world/entities.rs` the entity list and the take/put-back driver;
//! [`chunk_provider`] chunk loading; [`lighting`] the light stubs;
//! `crate::tick` the per-tick `World.tick`.
//!
//! # Conventions
//!
//! - Fields keep their Java names in snake_case, in Java declaration order,
//!   each with its line. Java statics that are really per-run state
//!   (`Entity.nextEntityID`, `NextTickListEntry.nextTickEntryID`,
//!   `World.lightingUpdatesScheduled`, the `Math.random()` stream) are fields
//!   here, so a scenario resets them by building a new world (PLAN_v4 §4.2).
//! - Block reads take `&mut self`: in Java, `getBlockId` and friends call
//!   `getChunkFromChunkCoords`, which **loads or generates** a missing chunk.
//!   [`World::loaded_chunk`] is the read-only, never-loading accessor for
//!   dumps and the client.
//! - Every `worldAccesses` loop becomes one push onto [`World::events`]
//!   (see [`events`]), cited at its call site.
//! - Code that is not transcribed yet records itself instead of panicking:
//!   world systems in [`World::skipped_systems`] (e.g. `"SpawnerAnimals.performSpawning"`),
//!   block/entity overrides in [`World::unimplemented_hits`]. Both count hits
//!   by Java name, so a fixture run shows exactly which gaps it touched.

pub mod chunk;
pub mod chunk_provider;
mod entities;
pub mod events;
pub mod info;
pub mod lighting;
pub mod nibble;
pub mod provider;
pub mod tick_entry;
pub mod tile_entity;

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use mc_core::java_hash::{ChunkCoordIntPair, JavaHashSet};
use mc_core::java_random::JavaRandom;
use mc_core::math::{self, floor_double};
use mc_core::{Aabb, Vec3D};

use crate::block::{self, BlockStatics, Material, dispatch as block_dispatch};
use crate::entity::{Entity, EntityBase, EntitySlot};
use crate::save::SaveHandler;

use self::chunk_provider::{ChunkGenerator, ChunkProviderCache};
use self::events::{EventQueue, WorldEvent};
use self::info::WorldInfo;
use self::lighting::{EnumSkyBlock, MetadataChunkBlock};
use self::provider::WorldProvider;
use self::tick_entry::{NextTickListEntry, TickKey};
use self::tile_entity::{TileEntity, TileEntityRef};

/// The horizontal world limit every vanilla block accessor checks:
/// `x >= -32000000 && z >= -32000000 && x < 32000000 && z <= 32000000`
/// (vanilla, transcribed: `z` includes its upper bound, `x` does not).
// src: World.java:312
#[inline]
pub fn in_world_xz(x: i32, z: i32) -> bool {
    (-32000000..32000000).contains(&x) && (-32000000..=32000000).contains(&z)
}

/// `World`.
pub struct World {
    /// `scheduledUpdatesAreImmediate`.
    // src: World.java:13
    pub scheduled_updates_are_immediate: bool,
    /// `lightingToUpdate` (stays empty until WP-1.3, see [`lighting`]).
    // src: World.java:14
    pub(crate) lighting_to_update: Vec<MetadataChunkBlock>,
    /// `loadedEntityList`, in Java list order (module docs of [`crate::entity`]).
    // src: World.java:15
    pub loaded_entity_list: Vec<EntitySlot>,
    /// `unloadedEntityList`, as entity ids.
    // src: World.java:16
    pub(crate) unloaded_entity_list: Vec<i32>,
    /// `scheduledTickTreeSet`: `TreeSet` order, see [`NextTickListEntry`].
    // src: World.java:17
    pub(crate) scheduled_tick_tree_set: BTreeSet<NextTickListEntry>,
    /// `scheduledTickSet`: membership by `equals` (never iterated).
    // src: World.java:18
    pub(crate) scheduled_tick_set: BTreeSet<TickKey>,
    /// `loadedTileEntityList`.
    // src: World.java:19
    pub loaded_tile_entity_list: Vec<TileEntityRef>,
    /// `field_30900_E`: tile entities added while `field_31055_L` is set
    /// (during the tile-entity pass), merged at its end. Name kept.
    // src: World.java:20
    pub(crate) field_30900_e: Vec<TileEntityRef>,
    /// `playerEntities`, as entity ids.
    // src: World.java:21
    pub player_entities: Vec<i32>,
    /// `weatherEffects` (lightning bolts), ticked before the entity list.
    // src: World.java:22
    pub weather_effects: Vec<EntitySlot>,
    /// `field_1019_F` (`16777215L`): the cloud colour `drawClouds` reads
    /// (World.java:1088-1090; client only). Name kept.
    // src: World.java:23
    pub field_1019_f: i64,
    /// `skylightSubtracted`.
    // src: World.java:24
    pub skylight_subtracted: i32,
    /// `field_9437_g`: the random-tick LCG state (clock-seeded in vanilla).
    /// Name kept; see `docs/mappings.md`.
    // src: World.java:25
    pub field_9437_g: i32,
    /// `field_9436_h`: `1013904223`, the LCG increment; final, and the update
    /// sites use the literal (World.java:1903, :1921, :1934, :1953). Name kept.
    // src: World.java:26
    pub field_9436_h: i32,
    /// `prevRainingStrength`.
    // src: World.java:27
    pub prev_raining_strength: f32,
    /// `rainingStrength`.
    // src: World.java:28
    pub raining_strength: f32,
    /// `prevThunderingStrength`.
    // src: World.java:29
    pub prev_thundering_strength: f32,
    /// `thunderingStrength`.
    // src: World.java:30
    pub thundering_strength: f32,
    /// `field_27168_F`: set to 2 when a bolt strikes (World.java:1928),
    /// counted down in `updateWeather`. Name kept.
    // src: World.java:31
    pub field_27168_f: i32,
    /// `field_27172_i`: sky flash timer; set to 2 by `EntityLightningBolt`
    /// (EntityLightningBolt.java:71), read by `getSkyColor` (World.java:1058),
    /// decremented by the client each tick. Name kept.
    // src: World.java:32
    pub field_27172_i: i32,
    /// `editingBlocks`: suppresses neighbour notifications.
    // src: World.java:33
    pub editing_blocks: bool,
    /// `lockTimestamp` (`System.currentTimeMillis()` in vanilla; passed in, D9).
    // src: World.java:34
    pub lock_timestamp: i64,
    /// `autosavePeriod`.
    // src: World.java:35
    pub autosave_period: i32,
    /// `difficultySetting` (set by the client every tick).
    // src: World.java:36
    pub difficulty_setting: i32,
    /// `rand`.
    // src: World.java:37
    pub rand: JavaRandom,
    /// `isNewWorld`.
    // src: World.java:38
    pub is_new_world: bool,
    /// `worldProvider`.
    // src: World.java:39
    pub world_provider: WorldProvider,
    /// `worldAccesses`, as an event queue ([`events`]).
    // src: World.java:40
    pub events: EventQueue,
    /// `chunkProvider` (the caching `ChunkProvider`).
    // src: World.java:41
    pub(crate) chunk_provider: ChunkProviderCache,
    /// `saveHandler`.
    // src: World.java:42
    pub(crate) save_handler: Box<dyn SaveHandler>,
    /// `worldInfo`.
    // src: World.java:43
    pub world_info: WorldInfo,
    /// `findingSpawnPoint`.
    // src: World.java:44
    pub finding_spawn_point: bool,
    /// `allPlayersSleeping`.
    // src: World.java:45
    pub(crate) all_players_sleeping: bool,
    // `field_28108_z` (MapStorage, World.java:46): map item data, WP-2.6.
    // `collidingBoundingBoxes` (World.java:47) and `field_1012_M` (:55) are
    // scratch lists returned by reference; here each call returns a new Vec.
    /// `field_31055_L`: set while `updateEntities` iterates the tile entities.
    // src: World.java:48
    pub(crate) field_31055_l: bool,
    /// `lightingUpdatesCounter`.
    // src: World.java:49
    pub(crate) lighting_updates_counter: i32,
    /// `spawnHostileMobs`.
    // src: World.java:50
    pub(crate) spawn_hostile_mobs: bool,
    /// `spawnPeacefulMobs`.
    // src: World.java:51
    pub(crate) spawn_peaceful_mobs: bool,
    /// `lightingUpdatesScheduled` (a static in Java).
    // src: World.java:52
    pub(crate) lighting_updates_scheduled: i32,
    /// `positionsToUpdate`: iterated by the random-tick loop, so Java
    /// `HashSet` order (PLAN_v4 §3.4 rule 4).
    // src: World.java:53
    pub(crate) positions_to_update: JavaHashSet<ChunkCoordIntPair>,
    /// `soundCounter`: ticks until the next cave-sound attempt.
    // src: World.java:54
    pub(crate) sound_counter: i32,
    /// `multiplayerWorld`.
    // src: World.java:56
    pub multiplayer_world: bool,

    // ---- Java statics and singleton state owned by the world ----------------
    /// `Entity.nextEntityID` (Entity.java:7).
    pub next_entity_id: i32,
    /// `NextTickListEntry.nextTickEntryID` (NextTickListEntry.java:4).
    pub next_tick_entry_id: i64,
    /// The single stream every `Math.random()` call draws from (PLAN_v4 §3.4
    /// rule 3; the oracle's `HarnessMath`).
    pub math_random: JavaRandom,
    /// Seed of the entity-RNG protocol (PLAN_v4 §4.2): an entity's `rand` is
    /// seeded with `mix(entity_rand_seed, entityId)`
    /// ([`World::entity_rand_seed_for`]).
    pub entity_rand_seed: i64,
    /// The mutable parts of the `Block` singletons ([`BlockStatics`]).
    pub block_statics: BlockStatics,
    /// Every live tile entity, by handle ([`tile_entity`]).
    pub tile_entities: BTreeMap<TileEntityRef, TileEntity>,
    /// The next tile-entity handle.
    pub(crate) next_tile_entity_ref: u32,

    // ---- diagnostics ----------------------------------------------------------
    /// World systems reached but not transcribed yet, by Java method, with hit
    /// counts. A map rather than a log so a long run stays bounded.
    pub skipped_systems: RefCell<BTreeMap<&'static str, u64>>,
    /// Block/entity overrides reached but not transcribed yet, by
    /// `"Class.method"`, with hit counts ([`block::dispatch`],
    /// [`crate::entity::dispatch`]).
    pub unimplemented_hits: RefCell<BTreeMap<&'static str, u64>>,
}

/// The inputs `World`'s constructor takes from the environment in vanilla
/// (D9: time and unseeded randomness are passed in).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldEnvironment {
    /// Seed of `World.rand = new Random()` (World.java:82).
    pub rand_seed: i64,
    /// `field_9437_g = (new Random()).nextInt()` (World.java:75).
    pub field_9437_g: i32,
    /// `lockTimestamp = System.currentTimeMillis()` (World.java:80).
    pub lock_timestamp: i64,
}

impl World {
    /// `new World(saveHandler, name, provider, seed)` (World.java:62-100):
    /// a new world with fresh `WorldInfo(seed, name)`, no chunks loaded and no
    /// spawn search. The generator that vanilla gets from
    /// `provider.getChunkProvider()` (World.java:212) is passed in.
    ///
    /// To match the oracle's worlds, call [`World::make_deterministic`] next.
    // src: World.java:62-100
    pub fn new(
        mut save_handler: Box<dyn SaveHandler>,
        name: &str,
        provider: WorldProvider,
        seed: i64,
        generator: ChunkGenerator,
        env: WorldEnvironment,
    ) -> World {
        let mut rand = JavaRandom::new(env.rand_seed);
        // src: World.java:90
        let sound_counter = rand.next_int_bounded(12000);
        // src: World.java:210-213 (getChunkProvider)
        let loader = save_handler.get_chunk_loader(&provider);
        let mut w = World {
            scheduled_updates_are_immediate: false,
            lighting_to_update: Vec::new(),
            loaded_entity_list: Vec::new(),
            unloaded_entity_list: Vec::new(),
            scheduled_tick_tree_set: BTreeSet::new(),
            scheduled_tick_set: BTreeSet::new(),
            loaded_tile_entity_list: Vec::new(),
            field_30900_e: Vec::new(),
            player_entities: Vec::new(),
            weather_effects: Vec::new(),
            field_1019_f: 16777215,
            skylight_subtracted: 0,
            field_9437_g: env.field_9437_g,
            field_9436_h: 1013904223,
            prev_raining_strength: 0.0,
            raining_strength: 0.0,
            prev_thundering_strength: 0.0,
            thundering_strength: 0.0,
            field_27168_f: 0,
            field_27172_i: 0,
            editing_blocks: false,
            lock_timestamp: env.lock_timestamp,
            autosave_period: 40,
            difficulty_setting: 0,
            rand,
            is_new_world: false,
            world_provider: provider,
            events: EventQueue::default(),
            chunk_provider: ChunkProviderCache::new(loader, generator),
            save_handler,
            world_info: WorldInfo::new(seed, name),
            finding_spawn_point: false,
            all_players_sleeping: false,
            field_31055_l: false,
            lighting_updates_counter: 0,
            spawn_hostile_mobs: true,
            spawn_peaceful_mobs: true,
            lighting_updates_scheduled: 0,
            positions_to_update: JavaHashSet::new(),
            sound_counter,
            multiplayer_world: false,
            next_entity_id: 0,
            next_tick_entry_id: 0,
            math_random: JavaRandom::new(0),
            entity_rand_seed: 0,
            block_statics: BlockStatics::default(),
            tile_entities: BTreeMap::new(),
            next_tile_entity_ref: 0,
            skipped_systems: RefCell::new(BTreeMap::new()),
            unimplemented_hits: RefCell::new(BTreeMap::new()),
        };
        // `field_28108_z = new MapStorage(saveHandler)` (World.java:96): WP-2.6.
        // `worldProvider.registerWorld(this)` (World.java:97): done by
        // `WorldProvider::new`; the biome manager is WP-1.4.
        w.calculate_initial_skylight(); // src: World.java:99
        w.func_27163_e(); // src: World.java:100
        w
    }

    /// The oracle's determinism protocol for a scenario seed (PLAN_v4 §4.2;
    /// `HarnessWorld.makeDeterministic` and `resetStatics`,
    /// tests/golden/README.md "terrain"): `rand.setSeed(seed)`;
    /// from `d = new Random(seed)`: `field_9437_g = d.nextInt()`,
    /// `soundCounter = d.nextInt(12000)`; the `Math.random()` stream reseeded
    /// with `seed`; entity ids and tick-entry ids restart at 0; entity RNGs
    /// keyed off `seed`.
    pub fn make_deterministic(&mut self, seed: i64) {
        self.rand.set_seed(seed);
        let mut d = JavaRandom::new(seed);
        self.field_9437_g = d.next_int();
        self.sound_counter = d.next_int_bounded(12000);
        self.math_random.set_seed(seed);
        self.entity_rand_seed = seed;
        self.next_entity_id = 0;
        self.next_tick_entry_id = 0;
        self.lighting_updates_scheduled = 0;
    }

    /// The seed of entity `id`'s `rand`: `mix(entity_rand_seed, id)`
    /// (`HarnessWorld.seedEntity`).
    pub fn entity_rand_seed_for(&self, id: i32) -> i64 {
        crate::entity::base::mix_seed(self.entity_rand_seed, id as i64)
    }

    // ---- diagnostics ----------------------------------------------------------

    /// Records that the untranscribed world system `what` was reached.
    pub fn skip(&self, what: &'static str) {
        *self.skipped_systems.borrow_mut().entry(what).or_insert(0) += 1;
    }

    /// Records that the untranscribed override `what` (`"Class.method"`) was
    /// reached.
    pub fn note_unimplemented(&self, what: &'static str) {
        *self
            .unimplemented_hits
            .borrow_mut()
            .entry(what)
            .or_insert(0) += 1;
    }

    /// `func_27163_E()`: start at full rain/thunder strength if the save says so.
    // src: World.java:1782-1790
    fn func_27163_e(&mut self) {
        if self.world_info.raining {
            self.raining_strength = 1.0;
            if self.world_info.thundering {
                self.thundering_strength = 1.0;
            }
        }
    }

    /// `calculateInitialSkylight()`.
    // src: World.java:1732-1738
    pub fn calculate_initial_skylight(&mut self) {
        let var1 = self.calculate_skylight_subtracted(1.0);
        if var1 != self.skylight_subtracted {
            self.skylight_subtracted = var1;
        }
    }

    /// `random(source)`: the `java.util.Random` a block callback draws from
    /// ([`block::dispatch::RandomSource`]).
    pub fn random(&mut self, src: block_dispatch::RandomSource) -> &mut JavaRandom {
        match src {
            block_dispatch::RandomSource::WorldRand => &mut self.rand,
        }
    }

    // ---- block access (World.java:311-523) ---------------------------------------

    /// `getBlockId(x, y, z)`: 0 outside ±32,000,000 horizontally or y 0..127.
    /// Loads the chunk if needed.
    // src: World.java:311-321
    pub fn get_block_id(&mut self, x: i32, y: i32, z: i32) -> i32 {
        if in_world_xz(x, z) {
            if !(0..128).contains(&y) {
                0
            } else {
                self.get_chunk_from_chunk_coords(x >> 4, z >> 4)
                    .get_block_id(x & 15, y, z & 15)
            }
        } else {
            0
        }
    }

    /// `isAirBlock(x, y, z)`.
    // src: World.java:323-325
    pub fn is_air_block(&mut self, x: i32, y: i32, z: i32) -> bool {
        self.get_block_id(x, y, z) == 0
    }

    /// `blockExists(x, y, z)`: y in range and the chunk loaded.
    // src: World.java:327-329
    pub fn block_exists(&self, x: i32, y: i32, z: i32) -> bool {
        if (0..128).contains(&y) {
            self.chunk_exists(x >> 4, z >> 4)
        } else {
            false
        }
    }

    /// `doChunksNearChunkExist(x, y, z, r)`.
    // src: World.java:331-333
    pub fn do_chunks_near_chunk_exist(&self, x: i32, y: i32, z: i32, r: i32) -> bool {
        self.check_chunks_exist(x - r, y - r, z - r, x + r, y + r, z + r)
    }

    /// `checkChunksExist(x0, y0, z0, x1, y1, z1)`: every chunk of the box loaded.
    // src: World.java:335-356
    pub fn check_chunks_exist(&self, x0: i32, y0: i32, z0: i32, x1: i32, y1: i32, z1: i32) -> bool {
        if y1 >= 0 && y0 < 128 {
            let (x0, z0, x1, z1) = (x0 >> 4, z0 >> 4, x1 >> 4, z1 >> 4);
            for var7 in x0..=x1 {
                for var8 in z0..=z1 {
                    if !self.chunk_exists(var7, var8) {
                        return false;
                    }
                }
            }
            true
        } else {
            false
        }
    }

    /// `setBlockAndMetadata(x, y, z, id, meta)`: no notification.
    // src: World.java:370-383
    pub fn set_block_and_metadata(&mut self, x: i32, y: i32, z: i32, id: i32, meta: i32) -> bool {
        if in_world_xz(x, z) {
            if !(0..128).contains(&y) {
                false
            } else {
                self.get_chunk_from_chunk_coords(x >> 4, z >> 4);
                chunk::set_block_id_with_metadata(self, x >> 4, z >> 4, x & 15, y, z & 15, id, meta)
            }
        } else {
            false
        }
    }

    /// `setBlock(x, y, z, id)`: metadata 0, no notification.
    // src: World.java:385-398
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, id: i32) -> bool {
        if in_world_xz(x, z) {
            if !(0..128).contains(&y) {
                false
            } else {
                self.get_chunk_from_chunk_coords(x >> 4, z >> 4);
                chunk::set_block_id(self, x >> 4, z >> 4, x & 15, y, z & 15, id)
            }
        } else {
            false
        }
    }

    /// `getBlockMaterial(x, y, z)`.
    // src: World.java:400-403
    pub fn get_block_material(&mut self, x: i32, y: i32, z: i32) -> Material {
        let var4 = self.get_block_id(x, y, z);
        block::material_of(var4)
    }

    /// `getBlockMetadata(x, y, z)`.
    // src: World.java:405-420
    pub fn get_block_metadata(&mut self, x: i32, y: i32, z: i32) -> i32 {
        if in_world_xz(x, z) {
            if !(0..128).contains(&y) {
                0
            } else {
                self.get_chunk_from_chunk_coords(x >> 4, z >> 4)
                    .get_block_metadata(x & 15, y, z & 15)
            }
        } else {
            0
        }
    }

    /// `setBlockMetadataWithNotify(x, y, z, meta)`.
    // src: World.java:422-432
    pub fn set_block_metadata_with_notify(&mut self, x: i32, y: i32, z: i32, meta: i32) {
        if self.set_block_metadata(x, y, z, meta) {
            let var5 = self.get_block_id(x, y, z);
            if block::set_neighbor_notify_on_metadata_change(var5 & 255) {
                self.notify_block_change(x, y, z, var5);
            } else {
                self.notify_blocks_of_neighbor_change(x, y, z, var5);
            }
        }
    }

    /// `setBlockMetadata(x, y, z, meta)`.
    // src: World.java:434-450
    pub fn set_block_metadata(&mut self, x: i32, y: i32, z: i32, meta: i32) -> bool {
        if in_world_xz(x, z) {
            if !(0..128).contains(&y) {
                false
            } else {
                self.get_chunk_from_chunk_coords(x >> 4, z >> 4)
                    .set_block_metadata(x & 15, y, z & 15, meta);
                true
            }
        } else {
            false
        }
    }

    /// `setBlockWithNotify(x, y, z, id)`.
    // src: World.java:452-459
    pub fn set_block_with_notify(&mut self, x: i32, y: i32, z: i32, id: i32) -> bool {
        if self.set_block(x, y, z, id) {
            self.notify_block_change(x, y, z, id);
            true
        } else {
            false
        }
    }

    /// `setBlockAndMetadataWithNotify(x, y, z, id, meta)`.
    // src: World.java:461-468
    pub fn set_block_and_metadata_with_notify(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        id: i32,
        meta: i32,
    ) -> bool {
        if self.set_block_and_metadata(x, y, z, id, meta) {
            self.notify_block_change(x, y, z, id);
            true
        } else {
            false
        }
    }

    /// `markBlockNeedsUpdate(x, y, z)`.
    // src: World.java:470-475 (IWorldAccess.markBlockAndNeighborsNeedsUpdate)
    pub fn mark_block_needs_update(&mut self, x: i32, y: i32, z: i32) {
        self.events
            .push(WorldEvent::MarkBlockAndNeighborsNeedsUpdate { x, y, z });
    }

    /// `notifyBlockChange(x, y, z, id)`.
    // src: World.java:477-480
    pub fn notify_block_change(&mut self, x: i32, y: i32, z: i32, id: i32) {
        self.mark_block_needs_update(x, y, z);
        self.notify_blocks_of_neighbor_change(x, y, z, id);
    }

    /// `markBlocksDirtyVertical(x, z, y0, y1)`.
    // src: World.java:482-490
    pub fn mark_blocks_dirty_vertical(&mut self, x: i32, z: i32, mut y0: i32, mut y1: i32) {
        if y0 > y1 {
            std::mem::swap(&mut y0, &mut y1);
        }
        self.mark_blocks_dirty(x, y0, z, x, y1, z);
    }

    /// `markBlockAsNeedsUpdate(x, y, z)`.
    // src: World.java:492-497 (IWorldAccess.markBlockRangeNeedsUpdate)
    pub fn mark_block_as_needs_update(&mut self, x: i32, y: i32, z: i32) {
        self.events.push(WorldEvent::MarkBlockRangeNeedsUpdate {
            x0: x,
            y0: y,
            z0: z,
            x1: x,
            y1: y,
            z1: z,
        });
    }

    /// `markBlocksDirty(x0, y0, z0, x1, y1, z1)`.
    // src: World.java:499-504 (IWorldAccess.markBlockRangeNeedsUpdate)
    pub fn mark_blocks_dirty(&mut self, x0: i32, y0: i32, z0: i32, x1: i32, y1: i32, z1: i32) {
        self.events.push(WorldEvent::MarkBlockRangeNeedsUpdate {
            x0,
            y0,
            z0,
            x1,
            y1,
            z1,
        });
    }

    /// `notifyBlocksOfNeighborChange(x, y, z, id)`: -x, +x, -y, +y, -z, +z.
    // src: World.java:506-513
    pub fn notify_blocks_of_neighbor_change(&mut self, x: i32, y: i32, z: i32, id: i32) {
        self.notify_block_of_neighbor_change(x - 1, y, z, id);
        self.notify_block_of_neighbor_change(x + 1, y, z, id);
        self.notify_block_of_neighbor_change(x, y - 1, z, id);
        self.notify_block_of_neighbor_change(x, y + 1, z, id);
        self.notify_block_of_neighbor_change(x, y, z - 1, id);
        self.notify_block_of_neighbor_change(x, y, z + 1, id);
    }

    /// `notifyBlockOfNeighborChange(x, y, z, id)`.
    // src: World.java:515-523
    fn notify_block_of_neighbor_change(&mut self, x: i32, y: i32, z: i32, id: i32) {
        if !self.editing_blocks && !self.multiplayer_world {
            let var5 = self.get_block_id(x, y, z);
            if block::block(var5).is_some() {
                block_dispatch::on_neighbor_block_change(self, var5, x, y, z, id);
            }
        }
    }

    /// `canBlockSeeTheSky(x, y, z)`.
    // src: World.java:525-527
    pub fn can_block_see_the_sky(&mut self, x: i32, y: i32, z: i32) -> bool {
        self.get_chunk_from_chunk_coords(x >> 4, z >> 4)
            .can_block_see_the_sky(x & 15, y, z & 15)
    }

    /// `getFullBlockLightValue(x, y, z)`: light ignoring the sky darkening.
    // src: World.java:529-539
    pub fn get_full_block_light_value(&mut self, x: i32, mut y: i32, z: i32) -> i32 {
        if y < 0 {
            return 0;
        }
        if y >= 128 {
            y = 127;
        }
        self.get_chunk_from_chunk_coords(x >> 4, z >> 4)
            .get_block_light_value(x & 15, y, z & 15, 0)
    }

    /// `getBlockLightValue(x, y, z)`.
    // src: World.java:541-543
    pub fn get_block_light_value(&mut self, x: i32, y: i32, z: i32) -> i32 {
        self.get_block_light_value_do(x, y, z, true)
    }

    /// `getBlockLightValue_do(x, y, z, useNeighbours)`: slabs, farmland and
    /// stairs take the brightest neighbour.
    // src: World.java:545-590
    pub fn get_block_light_value_do(&mut self, x: i32, mut y: i32, z: i32, var4: bool) -> i32 {
        if !(in_world_xz(x, z)) {
            return 15;
        }
        if var4 {
            let var5 = self.get_block_id(x, y, z);
            // stairSingle 44, tilledField 60, stairCompactCobblestone 67, stairCompactPlanks 53
            if var5 == 44 || var5 == 60 || var5 == 67 || var5 == 53 {
                let mut var6 = self.get_block_light_value_do(x, y + 1, z, false);
                let var7 = self.get_block_light_value_do(x + 1, y, z, false);
                let var8 = self.get_block_light_value_do(x - 1, y, z, false);
                let var9 = self.get_block_light_value_do(x, y, z + 1, false);
                let var10 = self.get_block_light_value_do(x, y, z - 1, false);
                for v in [var7, var8, var9, var10] {
                    if v > var6 {
                        var6 = v;
                    }
                }
                return var6;
            }
        }
        if y < 0 {
            return 0;
        }
        if y >= 128 {
            y = 127;
        }
        let sub = self.skylight_subtracted;
        self.get_chunk_from_chunk_coords(x >> 4, z >> 4)
            .get_block_light_value(x & 15, y, z & 15, sub)
    }

    /// `canExistingBlockSeeTheSky(x, y, z)`: never loads.
    // src: World.java:592-609
    pub fn can_existing_block_see_the_sky(&self, x: i32, y: i32, z: i32) -> bool {
        if !(in_world_xz(x, z)) {
            return false;
        }
        if y < 0 {
            false
        } else if y >= 128 {
            true
        } else {
            match self.loaded_chunk(x >> 4, z >> 4) {
                None => false,
                Some(c) => c.can_block_see_the_sky(x & 15, y, z & 15),
            }
        }
    }

    /// `getHeightValue(x, z)`: 0 for an unloaded chunk; never loads.
    // src: World.java:611-622
    pub fn get_height_value(&self, x: i32, z: i32) -> i32 {
        if in_world_xz(x, z) {
            match self.loaded_chunk(x >> 4, z >> 4) {
                None => 0,
                Some(c) => c.get_height_value(x & 15, z & 15),
            }
        } else {
            0
        }
    }

    /// `getSavedLightValue(type, x, y, z)`: never loads.
    // src: World.java:646-667
    pub fn get_saved_light_value(&self, kind: EnumSkyBlock, x: i32, mut y: i32, z: i32) -> i32 {
        if y < 0 {
            y = 0;
        }
        if y >= 128 {
            y = 127;
        }
        if (0..128).contains(&y) && in_world_xz(x, z) {
            match self.loaded_chunk(x >> 4, z >> 4) {
                None => 0,
                Some(c) => c.get_saved_light_value(kind, x & 15, y, z & 15),
            }
        } else {
            kind.default_light_value()
        }
    }

    /// `setLightValue(type, x, y, z, value)`.
    // src: World.java:669-685 (IWorldAccess.markBlockAndNeighborsNeedsUpdate)
    pub fn set_light_value(&mut self, kind: EnumSkyBlock, x: i32, y: i32, z: i32, value: i32) {
        if in_world_xz(x, z) && (0..128).contains(&y) && self.chunk_exists(x >> 4, z >> 4) {
            self.chunk_mut(x >> 4, z >> 4)
                .set_light_value(kind, x & 15, y, z & 15, value);
            self.events
                .push(WorldEvent::MarkBlockAndNeighborsNeedsUpdate { x, y, z });
        }
    }

    /// `getBrightness(x, y, z, minLight)`.
    // src: World.java:687-694
    pub fn get_brightness(&mut self, x: i32, y: i32, z: i32, min: i32) -> f32 {
        let mut var5 = self.get_block_light_value(x, y, z);
        if var5 < min {
            var5 = min;
        }
        self.world_provider.light_brightness_table[var5 as usize]
    }

    /// `getLightBrightness(x, y, z)`.
    // src: World.java:696-698
    pub fn get_light_brightness(&mut self, x: i32, y: i32, z: i32) -> f32 {
        let v = self.get_block_light_value(x, y, z);
        self.world_provider.light_brightness_table[v as usize]
    }

    /// `isDaytime()`: `skylightSubtracted < 4`.
    // src: World.java:700-702
    pub fn is_daytime(&self) -> bool {
        self.skylight_subtracted < 4
    }

    // ---- sounds and particles (IWorldAccess) ----------------------------------------

    /// `playSoundAtEntity(entity, name, volume, pitch)`: at the entity's feet.
    // src: World.java:863-868 (IWorldAccess.playSound)
    pub fn play_sound_at_entity(&mut self, e: &EntityBase, name: &str, volume: f32, pitch: f32) {
        self.events.push(WorldEvent::PlaySound {
            name: Cow::Owned(name.to_string()),
            x: e.pos_x,
            y: e.pos_y - e.y_offset as f64,
            z: e.pos_z,
            volume,
            pitch,
        });
    }

    /// `playSoundEffect(x, y, z, name, volume, pitch)`.
    // src: World.java:870-875 (IWorldAccess.playSound)
    pub fn play_sound_effect(
        &mut self,
        x: f64,
        y: f64,
        z: f64,
        name: &str,
        volume: f32,
        pitch: f32,
    ) {
        self.events.push(WorldEvent::PlaySound {
            name: Cow::Owned(name.to_string()),
            x,
            y,
            z,
            volume,
            pitch,
        });
    }

    /// `playRecord(name, x, y, z)`.
    // src: World.java:877-882 (IWorldAccess.playRecord)
    pub fn play_record(&mut self, name: Option<&str>, x: i32, y: i32, z: i32) {
        self.events.push(WorldEvent::PlayRecord {
            name: name.map(|n| Cow::Owned(n.to_string())),
            x,
            y,
            z,
        });
    }

    /// `spawnParticle(name, x, y, z, vx, vy, vz)`.
    // src: World.java:884-889 (IWorldAccess.spawnParticle)
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_particle(
        &mut self,
        name: &'static str,
        x: f64,
        y: f64,
        z: f64,
        vx: f64,
        vy: f64,
        vz: f64,
    ) {
        self.events.push(WorldEvent::SpawnParticle {
            name: Cow::Borrowed(name),
            x,
            y,
            z,
            vx,
            vy,
            vz,
        });
    }

    /// `playAuxSFX(type, x, y, z, data)`: `playAuxSFXAtEntity(null, …)`.
    // src: World.java:2474-2476
    pub fn play_aux_sfx(&mut self, kind: i32, x: i32, y: i32, z: i32, data: i32) {
        self.play_aux_sfx_at_entity(None, kind, x, y, z, data);
    }

    /// `playAuxSFXAtEntity(player, type, x, y, z, data)`.
    // src: World.java:2478-2484 (IWorldAccess.playAuxSFX)
    pub fn play_aux_sfx_at_entity(
        &mut self,
        player: Option<i32>,
        kind: i32,
        x: i32,
        y: i32,
        z: i32,
        data: i32,
    ) {
        self.events.push(WorldEvent::PlayAuxSfx {
            player,
            kind,
            x,
            y,
            z,
            data,
        });
    }

    // ---- AABB queries (World.java:1357-1516) ---------------------------------------

    /// `getIsAnyLiquid(box)`: note the extra `--` on negative minimums
    /// (vanilla, transcribed).
    // src: World.java:1370-1401
    pub fn get_is_any_liquid(&mut self, bb: &Aabb) -> bool {
        let mut var2 = floor_double(bb.min_x);
        let var3 = floor_double(bb.max_x + 1.0);
        let mut var4 = floor_double(bb.min_y);
        let var5 = floor_double(bb.max_y + 1.0);
        let mut var6 = floor_double(bb.min_z);
        let var7 = floor_double(bb.max_z + 1.0);
        if bb.min_x < 0.0 {
            var2 -= 1;
        }
        if bb.min_y < 0.0 {
            var4 -= 1;
        }
        if bb.min_z < 0.0 {
            var6 -= 1;
        }
        for var8 in var2..var3 {
            for var9 in var4..var5 {
                for var10 in var6..var7 {
                    let id = self.get_block_id(var8, var9, var10);
                    if block::block(id).is_some_and(|b| b.block_material.get_is_liquid()) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// `isBoundingBoxBurning(box)`: fire or lava inside.
    // src: World.java:1403-1424
    pub fn is_bounding_box_burning(&mut self, bb: &Aabb) -> bool {
        let var2 = floor_double(bb.min_x);
        let var3 = floor_double(bb.max_x + 1.0);
        let var4 = floor_double(bb.min_y);
        let var5 = floor_double(bb.max_y + 1.0);
        let var6 = floor_double(bb.min_z);
        let var7 = floor_double(bb.max_z + 1.0);
        if self.check_chunks_exist(var2, var4, var6, var3, var5, var7) {
            for var8 in var2..var3 {
                for var9 in var4..var5 {
                    for var10 in var6..var7 {
                        let var11 = self.get_block_id(var8, var9, var10);
                        // fire 51, lavaMoving 10, lavaStill 11
                        if var11 == 51 || var11 == 10 || var11 == 11 {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// `handleMaterialAcceleration(box, material, entity)`: pushes the entity
    /// with the fluid's flow; true if it is in the fluid.
    // src: World.java:1426-1464
    pub fn handle_material_acceleration(
        &mut self,
        bb: &Aabb,
        material: Material,
        e: &mut Entity,
    ) -> bool {
        let var4 = floor_double(bb.min_x);
        let var5 = floor_double(bb.max_x + 1.0);
        let var6 = floor_double(bb.min_y);
        let var7 = floor_double(bb.max_y + 1.0);
        let var8 = floor_double(bb.min_z);
        let var9 = floor_double(bb.max_z + 1.0);
        if !self.check_chunks_exist(var4, var6, var8, var5, var7, var9) {
            return false;
        }
        let mut var10 = false;
        let mut var11 = Vec3D::create_vector(0.0, 0.0, 0.0);
        for var12 in var4..var5 {
            for var13 in var6..var7 {
                for var14 in var8..var9 {
                    let id = self.get_block_id(var12, var13, var14);
                    if block::block(id).is_some_and(|b| b.block_material == material) {
                        let meta = self.get_block_metadata(var12, var13, var14);
                        let var16 =
                            ((var13 + 1) as f32 - block::fluid::get_percent_air(meta)) as f64;
                        if var7 as f64 >= var16 {
                            var10 = true;
                            block_dispatch::velocity_to_add_to_entity(
                                self, id, var12, var13, var14, e, &mut var11,
                            );
                        }
                    }
                }
            }
        }
        if var11.length_vector() > 0.0 {
            var11 = var11.normalize();
            let var18 = 0.014;
            e.base.motion_x += var11.x_coord * var18;
            e.base.motion_y += var11.y_coord * var18;
            e.base.motion_z += var11.z_coord * var18;
        }
        var10
    }

    /// `isMaterialInBB(box, material)`.
    // src: World.java:1466-1486
    pub fn is_material_in_bb(&mut self, bb: &Aabb, material: Material) -> bool {
        let var3 = floor_double(bb.min_x);
        let var4 = floor_double(bb.max_x + 1.0);
        let var5 = floor_double(bb.min_y);
        let var6 = floor_double(bb.max_y + 1.0);
        let var7 = floor_double(bb.min_z);
        let var8 = floor_double(bb.max_z + 1.0);
        for var9 in var3..var4 {
            for var10 in var5..var6 {
                for var11 in var7..var8 {
                    let id = self.get_block_id(var9, var10, var11);
                    if block::block(id).is_some_and(|b| b.block_material == material) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// `isBlockOpaqueCube(x, y, z)`.
    // src: World.java:1639-1642
    pub fn is_block_opaque_cube(&mut self, x: i32, y: i32, z: i32) -> bool {
        let id = self.get_block_id(x, y, z);
        block_dispatch::is_opaque_cube(self, id)
    }

    /// `isBlockNormalCube(x, y, z)`: translucent-material (i.e. solid, not
    /// "opaque" in the MCP sense) and a normal block render.
    // src: World.java:1644-1651
    pub fn is_block_normal_cube(&mut self, x: i32, y: i32, z: i32) -> bool {
        let id = self.get_block_id(x, y, z);
        match block::block(id) {
            None => false,
            Some(b) => {
                b.block_material.get_is_translucent() && block_dispatch::render_as_normal_block(id)
            }
        }
    }

    /// `findTopSolidBlock(x, z)`: one above the highest solid or liquid block,
    /// -1 if none.
    // src: World.java:1122-1136
    pub fn find_top_solid_block(&mut self, x: i32, z: i32) -> i32 {
        let (cx, cz) = (x >> 4, z >> 4);
        self.get_chunk_from_chunk_coords(cx, cz);
        let c = self.chunk_mut(cx, cz);
        let (x, z) = (x & 15, z & 15);
        let mut var4 = 127;
        while var4 > 0 {
            let var6 = block::material_of(c.get_block_id(x, var4, z));
            if var6.get_is_solid() || var6.get_is_liquid() {
                return var4 + 1;
            }
            var4 -= 1;
        }
        -1
    }

    /// `canBlockBeRainedOn(x, y, z)`. The final biome test is a recorded stub
    /// until biomes exist (WP-1.4): it answers false.
    // src: World.java:2449-2460
    #[allow(clippy::if_same_then_else)] // the Java shape, kept
    pub fn can_block_be_rained_on(&mut self, x: i32, y: i32, z: i32) -> bool {
        if !self.is_raining() {
            false
        } else if !self.can_block_see_the_sky(x, y, z) {
            false
        } else if self.find_top_solid_block(x, z) > y {
            false
        } else {
            self.skip("BiomeGenBase (World.canBlockBeRainedOn)");
            false
        }
    }

    // ---- weather and time getters ---------------------------------------------------

    /// `calculateSkylightSubtracted(partialTick)`.
    // src: World.java:999-1015
    #[allow(clippy::manual_clamp)] // Java's two ifs, kept
    pub fn calculate_skylight_subtracted(&self, var1: f32) -> i32 {
        let var2 = self.get_celestial_angle(var1);
        let mut var3 = 1.0f32 - (math::cos(var2 * 3.1415927 * 2.0) * 2.0 + 0.5);
        if var3 < 0.0 {
            var3 = 0.0;
        }
        if var3 > 1.0 {
            var3 = 1.0;
        }
        var3 = 1.0 - var3;
        var3 = (var3 as f64 * (1.0 - (self.get_rain_strength(var1) * 5.0) as f64 / 16.0)) as f32;
        var3 = (var3 as f64
            * (1.0 - (self.get_weighted_thunder_strength(var1) * 5.0) as f64 / 16.0))
            as f32;
        var3 = 1.0 - var3;
        (var3 * 11.0) as i32
    }

    /// `getCelestialAngle(partialTick)`.
    // src: World.java:1073-1075
    pub fn get_celestial_angle(&self, var1: f32) -> f32 {
        self.world_provider
            .calculate_celestial_angle(self.world_info.world_time, var1)
    }

    /// `getWeightedThunderStrength(partialTick)`.
    // src: World.java:2428-2430
    pub fn get_weighted_thunder_strength(&self, var1: f32) -> f32 {
        (self.prev_thundering_strength
            + (self.thundering_strength - self.prev_thundering_strength) * var1)
            * self.get_rain_strength(var1)
    }

    /// `getRainStrength(partialTick)`.
    // src: World.java:2432-2434
    pub fn get_rain_strength(&self, var1: f32) -> f32 {
        self.prev_raining_strength + (self.raining_strength - self.prev_raining_strength) * var1
    }

    /// `setRainStrength(strength)`.
    // src: World.java:2436-2439
    pub fn set_rain_strength(&mut self, var1: f32) {
        self.prev_raining_strength = var1;
        self.raining_strength = var1;
    }

    /// `getIsThundering()`.
    // src: World.java:2441-2443
    pub fn get_is_thundering(&self) -> bool {
        self.get_weighted_thunder_strength(1.0) as f64 > 0.9
    }

    /// `isRaining()`.
    // src: World.java:2445-2447
    pub fn is_raining(&self) -> bool {
        self.get_rain_strength(1.0) as f64 > 0.2
    }

    /// `getWorldTime()`.
    // src: World.java:2282-2284
    pub fn get_world_time(&self) -> i64 {
        self.world_info.world_time
    }

    /// `setWorldTime(t)`.
    // src: World.java:2274-2276
    pub fn set_world_time(&mut self, t: i64) {
        self.world_info.world_time = t;
    }

    /// `getRandomSeed()`.
    // src: World.java:2278-2280
    pub fn get_random_seed(&self) -> i64 {
        self.world_info.random_seed
    }

    /// `setAllowedMobSpawns(hostile, peaceful)`.
    // src: World.java:1740-1743
    pub fn set_allowed_mob_spawns(&mut self, hostile: bool, peaceful: bool) {
        self.spawn_hostile_mobs = hostile;
        self.spawn_peaceful_mobs = peaceful;
    }

    // ---- scheduled ticks ----------------------------------------------------------------

    /// `scheduleBlockUpdate(x, y, z, blockID, delay)`.
    ///
    /// Membership is checked by `equals` (position and id) in
    /// `scheduledTickSet`, so a second request for a pending block is
    /// dropped even with an earlier time. The entry takes a `tickEntryID`
    /// even when it is then dropped (the constructor runs first).
    // src: World.java:1152-1176
    pub fn schedule_block_update(&mut self, x: i32, y: i32, z: i32, block_id: i32, delay: i32) {
        let mut var6 = NextTickListEntry::new(x, y, z, block_id, self.next_tick_entry_id);
        self.next_tick_entry_id += 1;
        let var7 = 8;
        if self.scheduled_updates_are_immediate {
            if self.check_chunks_exist(
                var6.x_coord - var7,
                var6.y_coord - var7,
                var6.z_coord - var7,
                var6.x_coord + var7,
                var6.y_coord + var7,
                var6.z_coord + var7,
            ) {
                let var8 = self.get_block_id(var6.x_coord, var6.y_coord, var6.z_coord);
                if var8 == var6.block_id && var8 > 0 {
                    block_dispatch::update_tick(
                        self,
                        var8,
                        var6.x_coord,
                        var6.y_coord,
                        var6.z_coord,
                        block_dispatch::RandomSource::WorldRand,
                    );
                }
            }
        } else if self.check_chunks_exist(
            x - var7,
            y - var7,
            z - var7,
            x + var7,
            y + var7,
            z + var7,
        ) {
            if block_id > 0 {
                var6.set_scheduled_time(delay as i64 + self.world_info.world_time);
            }
            if !self.scheduled_tick_set.contains(&var6.key()) {
                self.scheduled_tick_set.insert(var6.key());
                self.scheduled_tick_tree_set.insert(var6);
            }
        }
    }

    /// `scheduledTickTreeSet` in iteration (`TreeSet`) order, for dumps.
    pub fn scheduled_ticks(&self) -> impl Iterator<Item = &NextTickListEntry> {
        self.scheduled_tick_tree_set.iter()
    }

    // ---- tile entities -------------------------------------------------------------------

    /// Stores a new tile entity and returns its handle (vanilla's `new
    /// TileEntityXxx()`).
    pub fn new_tile_entity(&mut self, te: TileEntity) -> TileEntityRef {
        let r = TileEntityRef(self.next_tile_entity_ref);
        self.next_tile_entity_ref += 1;
        self.tile_entities.insert(r, te);
        r
    }

    /// `addTileEntity(collection)`: deferred while the tile-entity pass runs.
    // src: World.java:1278-1285
    pub fn add_tile_entity(&mut self, list: &[TileEntityRef]) {
        if self.field_31055_l {
            self.field_30900_e.extend_from_slice(list);
        } else {
            self.loaded_tile_entity_list.extend_from_slice(list);
        }
    }

    /// `getBlockTileEntity(x, y, z)`.
    // src: World.java:1599-1602
    pub fn get_block_tile_entity(&mut self, x: i32, y: i32, z: i32) -> Option<TileEntityRef> {
        self.get_chunk_from_chunk_coords(x >> 4, z >> 4);
        chunk::get_chunk_block_tile_entity(self, x >> 4, z >> 4, x & 15, y, z & 15)
    }

    /// `setBlockTileEntity(x, y, z, tileEntity)`.
    // src: World.java:1604-1620
    pub fn set_block_tile_entity(&mut self, x: i32, y: i32, z: i32, te: TileEntityRef) {
        let invalid = self.tile_entities.get(&te).is_none_or(|t| t.is_invalid());
        if invalid {
            return;
        }
        if self.field_31055_l {
            if let Some(t) = self.tile_entities.get_mut(&te) {
                t.x_coord = x;
                t.y_coord = y;
                t.z_coord = z;
            }
            self.field_30900_e.push(te);
        } else {
            self.loaded_tile_entity_list.push(te);
            self.get_chunk_from_chunk_coords(x >> 4, z >> 4);
            chunk::set_chunk_block_tile_entity(self, x >> 4, z >> 4, x & 15, y, z & 15, te);
        }
    }

    /// `removeBlockTileEntity(x, y, z)`.
    // src: World.java:1622-1637
    pub fn remove_block_tile_entity(&mut self, x: i32, y: i32, z: i32) {
        let var4 = self.get_block_tile_entity(x, y, z);
        if let (Some(r), true) = (var4, self.field_31055_l) {
            if let Some(t) = self.tile_entities.get_mut(&r) {
                t.invalidate();
            }
        } else {
            if let Some(r) = var4
                && let Some(i) = self.loaded_tile_entity_list.iter().position(|t| *t == r)
            {
                self.loaded_tile_entity_list.remove(i);
            }
            chunk::remove_chunk_block_tile_entity(self, x >> 4, z >> 4, x & 15, y, z & 15);
        }
    }

    /// `func_698_b(x, y, z, tileEntity)`: a tile entity's inventory changed.
    // src: World.java:2053-2062 (IWorldAccess.doNothingWithTileEntity)
    pub fn func_698_b(&mut self, x: i32, y: i32, z: i32) {
        if self.block_exists(x, y, z) {
            self.get_chunk_from_block_coords(x, z).set_chunk_modified();
        }
        self.events
            .push(WorldEvent::DoNothingWithTileEntity { x, y, z });
    }

    // ---- saving ----------------------------------------------------------------------------

    /// `saveWorld(all, progress)`.
    // src: World.java:278-291
    pub fn save_world(&mut self, all: bool) {
        if self.chunk_provider_can_save() {
            self.save_level();
            self.save_chunks(all);
        }
    }

    /// `saveLevel()`. The player NBT is not built yet (Phase 2); with
    /// [`crate::save::NoSave`] that is unobservable.
    // src: World.java:293-297
    fn save_level(&mut self) {
        let _ = self.save_handler.check_session_lock();
        if !self.player_entities.is_empty() {
            self.skip("EntityPlayer.writeToNBT (World.saveLevel)");
        }
        let info = self.world_info.clone();
        self.save_handler.save_world_info_and_player(&info, None);
        // field_28108_z.saveAllData() (MapStorage): WP-2.6, nothing to save yet.
    }
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("world_time", &self.world_info.world_time)
            .field("loaded_chunks", &self.chunk_provider.chunk_map.len())
            .field("loaded_entities", &self.loaded_entity_list.len())
            .finish_non_exhaustive()
    }
}
