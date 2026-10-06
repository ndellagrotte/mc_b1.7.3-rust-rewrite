//! `Chunk` (Chunk.java): a 16×128×16 column of blocks.
//!
//! # Two kinds of methods
//!
//! Many `Chunk` methods reach back into `worldObj` (block callbacks, lighting,
//! neighbour heights). A `Chunk` lives inside the [`World`], so it cannot hold
//! a reference to it. The rule:
//!
//! - methods that only touch the chunk's own arrays are `impl Chunk`;
//! - methods that touch `worldObj` are free functions in this module taking
//!   `world: &mut World` and the chunk's coordinates `(cx, cz)`. They re-borrow
//!   the chunk from the world for each array access, so every world call in
//!   between sees the chunk exactly as Java's would (e.g. `setBlockID` writes
//!   the new id *before* calling the old block's `onBlockRemoval`).
//!
//! # Layout
//!
//! `blocks[x << 11 | z << 7 | y]` (Chunk.java:233); the three `NibbleArray`s use
//! the same index (NibbleArray.java:15); `heightMap[z << 4 | x]` (Chunk.java:61).
//! Entities are kept per 16-block vertical section as entity ids
//! (`List[] entities`, Chunk.java:23, 32), in Java list order.
//!
//! # Lighting
//!
//! The chunk-local light code (`generateHeightMap`, `generateSkylightMap`,
//! `relightBlock`, `propagateSkylightOcclusion`) is transcribed because
//! `setBlockID` runs it and it decides the height map. What it calls into the
//! world — `World.scheduleLightingUpdate` — is a recorded stub until WP-1.3
//! ([`World::schedule_lighting_update`]), so sky light outside the column
//! updates done here and all block light are not yet right.

use mc_core::java_hash::{ChunkPosition, JavaHashMap};
use mc_core::math::floor_double;

use super::World;
use super::lighting::EnumSkyBlock;
use super::nibble::NibbleArray;
use super::tile_entity::TileEntityRef;
use crate::block::{self, dispatch};
use crate::entity::EntityBase;

/// `Chunk.blocks.length`.
pub const CHUNK_VOLUME: usize = 32768;

/// The `blocks`/`NibbleArray` index of `(x, y, z)` within a chunk.
// src: Chunk.java:233
#[inline]
pub const fn block_index(x: i32, y: i32, z: i32) -> usize {
    (x << 11 | z << 7 | y) as usize
}

/// `Chunk`.
#[derive(Clone, Debug)]
pub struct Chunk {
    // `Chunk.isLit` (Chunk.java:11) is a write-only static (set in
    // getBlockLightValue, Chunk.java:344, read nowhere): not ported.
    /// `blocks`.
    pub blocks: Box<[u8; CHUNK_VOLUME]>,
    /// `isChunkLoaded`.
    pub is_chunk_loaded: bool,
    // `worldObj`: implicit (module docs).
    /// `data` (metadata).
    pub data: NibbleArray,
    /// `skylightMap`.
    pub skylight_map: NibbleArray,
    /// `blocklightMap`.
    pub blocklight_map: NibbleArray,
    /// `heightMap`.
    pub height_map: [u8; 256],
    /// `lowestBlockHeight`.
    pub lowest_block_height: i32,
    /// `xPosition`.
    pub x_position: i32,
    /// `zPosition`.
    pub z_position: i32,
    /// `chunkTileEntityMap`: iterated (`onChunkLoad`, `onChunkUnload`,
    /// Chunk.java:461, :471), so Java `HashMap` order (PLAN_v4 §3.4 rule 4).
    pub chunk_tile_entity_map: JavaHashMap<ChunkPosition, TileEntityRef>,
    /// `entities`: eight vertical sections of entity ids, in list order.
    pub entities: [Vec<i32>; 8],
    /// `isTerrainPopulated`.
    pub is_terrain_populated: bool,
    /// `isModified`.
    pub is_modified: bool,
    /// `neverSave`.
    pub never_save: bool,
    /// `hasEntities`.
    pub has_entities: bool,
    /// `lastSaveTime`.
    pub last_save_time: i64,
}

impl Chunk {
    /// `new Chunk(world, blocks, x, z)`: zeroed nibble arrays and height map.
    // src: Chunk.java:30-46, Chunk.java:48-54
    pub fn new(blocks: Box<[u8; CHUNK_VOLUME]>, x: i32, z: i32) -> Self {
        Chunk {
            blocks,
            is_chunk_loaded: false,
            data: NibbleArray::new(CHUNK_VOLUME),
            skylight_map: NibbleArray::new(CHUNK_VOLUME),
            blocklight_map: NibbleArray::new(CHUNK_VOLUME),
            height_map: [0; 256],
            lowest_block_height: 0,
            x_position: x,
            z_position: z,
            chunk_tile_entity_map: JavaHashMap::new(),
            entities: Default::default(),
            is_terrain_populated: false,
            is_modified: false,
            never_save: false,
            has_entities: false,
            last_save_time: 0,
        }
    }

    /// `isAtLocation(x, z)`.
    // src: Chunk.java:56-58
    pub fn is_at_location(&self, x: i32, z: i32) -> bool {
        x == self.x_position && z == self.z_position
    }

    /// `getHeightValue(x, z)`.
    // src: Chunk.java:60-62
    pub fn get_height_value(&self, x: i32, z: i32) -> i32 {
        self.height_map[(z << 4 | x) as usize] as i32
    }

    /// `generateHeightMap()`.
    // src: Chunk.java:67-86
    pub fn generate_height_map(&mut self) {
        let mut var1 = 127;
        for var2 in 0..16 {
            for var3 in 0..16 {
                let mut var4 = 127;
                let var5 = (var2 << 11 | var3 << 7) as usize;
                while var4 > 0
                    && block::light_opacity(self.blocks[var5 + var4 as usize - 1] as i32) == 0
                {
                    var4 -= 1;
                }
                self.height_map[(var3 << 4 | var2) as usize] = var4 as u8;
                if var4 < var1 {
                    var1 = var4;
                }
            }
        }
        self.lowest_block_height = var1;
        self.is_modified = true;
    }

    /// `getBlockID(x, y, z)`.
    // src: Chunk.java:232-234
    pub fn get_block_id(&self, x: i32, y: i32, z: i32) -> i32 {
        self.blocks[block_index(x, y, z)] as i32
    }

    /// `getBlockMetadata(x, y, z)`.
    // src: Chunk.java:310-312
    pub fn get_block_metadata(&self, x: i32, y: i32, z: i32) -> i32 {
        self.data.get_nibble(x, y, z)
    }

    /// `setBlockMetadata(x, y, z, meta)`.
    // src: Chunk.java:314-317
    pub fn set_block_metadata(&mut self, x: i32, y: i32, z: i32, meta: i32) {
        self.is_modified = true;
        self.data.set_nibble(x, y, z, meta);
    }

    /// `getSavedLightValue(type, x, y, z)`.
    // src: Chunk.java:319-325
    pub fn get_saved_light_value(&self, kind: EnumSkyBlock, x: i32, y: i32, z: i32) -> i32 {
        match kind {
            EnumSkyBlock::Sky => self.skylight_map.get_nibble(x, y, z),
            EnumSkyBlock::Block => self.blocklight_map.get_nibble(x, y, z),
        }
    }

    /// `setLightValue(type, x, y, z, value)`.
    // src: Chunk.java:327-339
    pub fn set_light_value(&mut self, kind: EnumSkyBlock, x: i32, y: i32, z: i32, value: i32) {
        self.is_modified = true;
        match kind {
            EnumSkyBlock::Sky => self.skylight_map.set_nibble(x, y, z, value),
            EnumSkyBlock::Block => self.blocklight_map.set_nibble(x, y, z, value),
        }
    }

    /// `getBlockLightValue(x, y, z, skylightSubtracted)`.
    // src: Chunk.java:341-354
    pub fn get_block_light_value(&self, x: i32, y: i32, z: i32, subtracted: i32) -> i32 {
        let mut var5 = self.skylight_map.get_nibble(x, y, z);
        var5 -= subtracted;
        let var6 = self.blocklight_map.get_nibble(x, y, z);
        if var6 > var5 {
            var5 = var6;
        }
        var5
    }

    /// `addEntity(entity)`: files the entity under its vertical section and
    /// records the section in the entity. The "Wrong location!" diagnostic
    /// (Chunk.java:360-363) only prints; it is not ported.
    // src: Chunk.java:356-379
    pub fn add_entity(&mut self, e: &mut EntityBase) {
        self.has_entities = true;
        let mut var4 = floor_double(e.pos_y / 16.0);
        if var4 < 0 {
            var4 = 0;
        }
        if var4 >= self.entities.len() as i32 {
            var4 = self.entities.len() as i32 - 1;
        }
        e.added_to_chunk = true;
        e.chunk_coord_x = self.x_position;
        e.chunk_coord_y = var4;
        e.chunk_coord_z = self.z_position;
        self.entities[var4 as usize].push(e.entity_id);
    }

    /// `removeEntity(entity)`: `removeEntityAtIndex(entity, entity.chunkCoordY)`.
    // src: Chunk.java:381-383
    pub fn remove_entity(&mut self, entity_id: i32, chunk_coord_y: i32) {
        self.remove_entity_at_index(entity_id, chunk_coord_y);
    }

    /// `removeEntityAtIndex(entity, section)`: `List.remove(Object)` removes the
    /// first entry equal to the entity (`Entity.equals` compares `entityId`,
    /// Entity.java:121-127).
    // src: Chunk.java:385-395
    pub fn remove_entity_at_index(&mut self, entity_id: i32, section: i32) {
        let mut var2 = section;
        if var2 < 0 {
            var2 = 0;
        }
        if var2 >= self.entities.len() as i32 {
            var2 = self.entities.len() as i32 - 1;
        }
        let list = &mut self.entities[var2 as usize];
        if let Some(i) = list.iter().position(|&id| id == entity_id) {
            list.remove(i);
        }
    }

    /// `canBlockSeeTheSky(x, y, z)`.
    // src: Chunk.java:397-399
    pub fn can_block_see_the_sky(&self, x: i32, y: i32, z: i32) -> bool {
        y >= self.height_map[(z << 4 | x) as usize] as i32
    }

    /// `setChunkModified()`.
    // src: Chunk.java:484-486
    pub fn set_chunk_modified(&mut self) {
        self.is_modified = true;
    }

    /// `needsSaving(all)`; `world_time` is `worldObj.getWorldTime()`.
    // src: Chunk.java:536-550
    pub fn needs_saving(&self, all: bool, world_time: i64) -> bool {
        if self.never_save {
            return false;
        }
        if all {
            if self.has_entities && world_time != self.last_save_time {
                return true;
            }
        } else if self.has_entities && world_time >= self.last_save_time + 600 {
            return true;
        }
        self.is_modified
    }

    /// `isEmpty()`: false (only `EmptyChunk` overrides it).
    // src: Chunk.java:602-604
    pub fn is_empty(&self) -> bool {
        false
    }

    /// The ids of the entities in sections `min_y..=max_y` of the AABB query
    /// helpers (`getEntitiesWithinAABBForEntity`, Chunk.java:488-510), clamped
    /// as Java clamps them.
    pub(crate) fn entity_sections(
        &self,
        min_y: f64,
        max_y: f64,
    ) -> std::ops::RangeInclusive<usize> {
        let mut var4 = floor_double((min_y - 2.0) / 16.0);
        let mut var5 = floor_double((max_y + 2.0) / 16.0);
        if var4 < 0 {
            var4 = 0;
        }
        if var5 >= self.entities.len() as i32 {
            var5 = self.entities.len() as i32 - 1;
        }
        // An empty range when var4 > var5, as the Java loop runs zero times.
        if var4 > var5 {
            #[allow(clippy::reversed_empty_ranges)]
            return 1..=0;
        }
        var4 as usize..=var5 as usize
    }
}

// ---- Chunk methods that reach the world -------------------------------------

/// `generateSkylightMap()` for a chunk that is not (yet) in the world's chunk
/// map, as generators call it right after building the block array
/// (ChunkProviderGenerate.provideChunk; the oracle's `EmptyChunkProvider`).
///
/// Because the chunk is not in the map, the `propagateSkylightOcclusion` pass
/// sees `World.getHeightValue` = 0 for its own columns
/// (`chunkExists` is false, World.java:611-621), exactly as in vanilla.
// src: Chunk.java:88-131
pub fn generate_skylight_map(world: &mut World, c: &mut Chunk) {
    let mut var1 = 127;
    for var2 in 0..16 {
        for var3 in 0..16 {
            let mut var4 = 127;
            let var5 = (var2 << 11 | var3 << 7) as usize;
            while var4 > 0 && block::light_opacity(c.blocks[var5 + var4 as usize - 1] as i32) == 0 {
                var4 -= 1;
            }
            c.height_map[(var3 << 4 | var2) as usize] = var4 as u8;
            if var4 < var1 {
                var1 = var4;
            }
            if !world.world_provider.has_no_sky {
                let mut var6 = 15;
                let mut var7: i32 = 127;
                loop {
                    var6 -= block::light_opacity(c.blocks[var5 + var7 as usize] as i32);
                    if var6 > 0 {
                        c.skylight_map.set_nibble(var2, var7, var3, var6);
                    }
                    var7 -= 1;
                    if !(var7 > 0 && var6 > 0) {
                        break;
                    }
                }
            }
        }
    }
    c.lowest_block_height = var1;
    for var2 in 0..16 {
        for var3 in 0..16 {
            propagate_skylight_occlusion_detached(world, c, var2, var3);
        }
    }
    c.is_modified = true;
}

/// `propagateSkylightOcclusion` for a chunk held outside the world map.
// src: Chunk.java:136-144
fn propagate_skylight_occlusion_detached(world: &mut World, c: &mut Chunk, x: i32, z: i32) {
    let var3 = c.get_height_value(x, z);
    let var4 = c.x_position * 16 + x;
    let var5 = c.z_position * 16 + z;
    for (nx, nz) in [
        (var4 - 1, var5),
        (var4 + 1, var5),
        (var4, var5 - 1),
        (var4, var5 + 1),
    ] {
        if check_skylight_neighbor_height(world, nx, nz, var3) {
            c.is_modified = true;
        }
    }
}

/// `propagateSkylightOcclusion(x, z)` for a chunk in the world map.
// src: Chunk.java:136-144
fn propagate_skylight_occlusion(world: &mut World, cx: i32, cz: i32, x: i32, z: i32) {
    let (var3, var4, var5) = {
        let c = world.chunk_mut(cx, cz);
        (
            c.get_height_value(x, z),
            c.x_position * 16 + x,
            c.z_position * 16 + z,
        )
    };
    for (nx, nz) in [
        (var4 - 1, var5),
        (var4 + 1, var5),
        (var4, var5 - 1),
        (var4, var5 + 1),
    ] {
        if check_skylight_neighbor_height(world, nx, nz, var3) {
            world.chunk_mut(cx, cz).is_modified = true;
        }
    }
}

/// `checkSkylightNeighborHeight(x, z, height)`; returns whether the chunk
/// must set `isModified`.
// src: Chunk.java:146-156
fn check_skylight_neighbor_height(world: &mut World, x: i32, z: i32, height: i32) -> bool {
    let var4 = world.get_height_value(x, z);
    if var4 > height {
        world.schedule_lighting_update(EnumSkyBlock::Sky, x, height, z, x, var4, z);
        true
    } else if var4 < height {
        world.schedule_lighting_update(EnumSkyBlock::Sky, x, var4, z, x, height, z);
        true
    } else {
        false
    }
}

/// `relightBlock(x, y, z)`: moves the column's height-map entry after a block
/// change and fixes the column's sky light.
// src: Chunk.java:158-230
fn relight_block(world: &mut World, cx: i32, cz: i32, x: i32, y: i32, z: i32) {
    let (var4, mut var5) = {
        let c = world.chunk_mut(cx, cz);
        let var4 = c.height_map[(z << 4 | x) as usize] as i32;
        let mut var5 = var4;
        if y > var4 {
            var5 = y;
        }
        let var6 = (x << 11 | z << 7) as usize;
        while var5 > 0 && block::light_opacity(c.blocks[var6 + var5 as usize - 1] as i32) == 0 {
            var5 -= 1;
        }
        (var4, var5)
    };
    if var5 == var4 {
        return;
    }
    world.mark_blocks_dirty_vertical(x, z, var5, var4);
    let (var7, var8) = {
        let c = world.chunk_mut(cx, cz);
        c.height_map[(z << 4 | x) as usize] = var5 as u8;
        if var5 < c.lowest_block_height {
            c.lowest_block_height = var5;
        } else {
            let mut var7 = 127;
            for var8 in 0..16 {
                for var9 in 0..16 {
                    let h = c.height_map[(var9 << 4 | var8) as usize] as i32;
                    if h < var7 {
                        var7 = h;
                    }
                }
            }
            c.lowest_block_height = var7;
        }
        (c.x_position * 16 + x, c.z_position * 16 + z)
    };
    if var5 < var4 {
        let c = world.chunk_mut(cx, cz);
        for var9 in var5..var4 {
            c.skylight_map.set_nibble(x, var9, z, 15);
        }
    } else {
        world.schedule_lighting_update(EnumSkyBlock::Sky, var7, var4, var8, var7, var5, var8);
        let c = world.chunk_mut(cx, cz);
        for var9 in var4..var5 {
            c.skylight_map.set_nibble(x, var9, z, 0);
        }
    }
    let var10 = var5;
    {
        let c = world.chunk_mut(cx, cz);
        let mut var9 = 15;
        while var5 > 0 && var9 > 0 {
            var5 -= 1;
            let mut var11 = block::light_opacity(c.get_block_id(x, var5, z));
            if var11 == 0 {
                var11 = 1;
            }
            var9 -= var11;
            if var9 < 0 {
                var9 = 0;
            }
            c.skylight_map.set_nibble(x, var5, z, var9);
        }
        while var5 > 0 && block::light_opacity(c.get_block_id(x, var5 - 1, z)) == 0 {
            var5 -= 1;
        }
    }
    if var5 != var10 {
        world.schedule_lighting_update(
            EnumSkyBlock::Sky,
            var7 - 1,
            var5,
            var8 - 1,
            var7 + 1,
            var10,
            var8 + 1,
        );
    }
    world.chunk_mut(cx, cz).is_modified = true;
}

/// `setBlockIDWithMetadata(x, y, z, id, meta)`.
// src: Chunk.java:236-273
#[allow(clippy::too_many_arguments)] // Java's arguments plus the chunk coordinates
pub fn set_block_id_with_metadata(
    world: &mut World,
    cx: i32,
    cz: i32,
    x: i32,
    y: i32,
    z: i32,
    id: i32,
    meta: i32,
) -> bool {
    let var6 = id as u8; // (byte)id
    let (var7, var8, var9, var10) = {
        let c = world.chunk_mut(cx, cz);
        let var7 = c.height_map[(z << 4 | x) as usize] as i32;
        let var8 = c.blocks[block_index(x, y, z)] as i32;
        if var8 == id && c.data.get_nibble(x, y, z) == meta {
            return false;
        }
        c.blocks[block_index(x, y, z)] = var6;
        (var7, var8, c.x_position * 16 + x, c.z_position * 16 + z)
    };
    if var8 != 0 && !world.multiplayer_world {
        dispatch::on_block_removal(world, var8, var9, y, var10);
    }
    world.chunk_mut(cx, cz).data.set_nibble(x, y, z, meta);
    if !world.world_provider.has_no_sky {
        if block::light_opacity(var6 as i32) != 0 {
            if y >= var7 {
                relight_block(world, cx, cz, x, y + 1, z);
            }
        } else if y == var7 - 1 {
            relight_block(world, cx, cz, x, y, z);
        }
        world.schedule_lighting_update(EnumSkyBlock::Sky, var9, y, var10, var9, y, var10);
    }
    world.schedule_lighting_update(EnumSkyBlock::Block, var9, y, var10, var9, y, var10);
    propagate_skylight_occlusion(world, cx, cz, x, z);
    world.chunk_mut(cx, cz).data.set_nibble(x, y, z, meta);
    if id != 0 {
        // Vanilla quirk (transcribed): unlike setBlockID, this path calls
        // onBlockAdded even on a multiplayer world (Chunk.java:266-268 vs :301).
        dispatch::on_block_added(world, id, var9, y, var10);
    }
    world.chunk_mut(cx, cz).is_modified = true;
    true
}

/// `setBlockID(x, y, z, id)`: metadata becomes 0.
// src: Chunk.java:275-308
pub fn set_block_id(world: &mut World, cx: i32, cz: i32, x: i32, y: i32, z: i32, id: i32) -> bool {
    let var5 = id as u8; // (byte)id
    let (var6, var7, var8, var9) = {
        let c = world.chunk_mut(cx, cz);
        let var6 = c.height_map[(z << 4 | x) as usize] as i32;
        let var7 = c.blocks[block_index(x, y, z)] as i32;
        if var7 == id {
            return false;
        }
        c.blocks[block_index(x, y, z)] = var5;
        (var6, var7, c.x_position * 16 + x, c.z_position * 16 + z)
    };
    if var7 != 0 {
        // Vanilla: no multiplayerWorld check on this path (Chunk.java:285-287).
        dispatch::on_block_removal(world, var7, var8, y, var9);
    }
    world.chunk_mut(cx, cz).data.set_nibble(x, y, z, 0);
    if block::light_opacity(var5 as i32) != 0 {
        if y >= var6 {
            relight_block(world, cx, cz, x, y + 1, z);
        }
    } else if y == var6 - 1 {
        relight_block(world, cx, cz, x, y, z);
    }
    world.schedule_lighting_update(EnumSkyBlock::Sky, var8, y, var9, var8, y, var9);
    world.schedule_lighting_update(EnumSkyBlock::Block, var8, y, var9, var8, y, var9);
    propagate_skylight_occlusion(world, cx, cz, x, z);
    if id != 0 && !world.multiplayer_world {
        dispatch::on_block_added(world, id, var8, y, var9);
    }
    world.chunk_mut(cx, cz).is_modified = true;
    true
}

/// `getChunkBlockTileEntity(x, y, z)`: creates the tile entity on demand by
/// calling the container block's `onBlockAdded`.
// src: Chunk.java:401-421
pub fn get_chunk_block_tile_entity(
    world: &mut World,
    cx: i32,
    cz: i32,
    x: i32,
    y: i32,
    z: i32,
) -> Option<TileEntityRef> {
    let var4 = ChunkPosition::new(x, y, z);
    let mut var5 = world
        .chunk_mut(cx, cz)
        .chunk_tile_entity_map
        .get(&var4)
        .copied();
    if var5.is_none() {
        let (var6, wx, wz) = {
            let c = world.chunk_mut(cx, cz);
            (
                c.get_block_id(x, y, z),
                c.x_position * 16 + x,
                c.z_position * 16 + z,
            )
        };
        if !block::is_block_container(var6) {
            return None;
        }
        dispatch::on_block_added(world, var6, wx, y, wz);
        var5 = world
            .chunk_mut(cx, cz)
            .chunk_tile_entity_map
            .get(&var4)
            .copied();
    }
    match var5 {
        Some(r) if world.tile_entities.get(&r).is_some_and(|t| t.is_invalid()) => {
            world.chunk_mut(cx, cz).chunk_tile_entity_map.remove(&var4);
            None
        }
        other => other,
    }
}

/// `setChunkBlockTileEntity(x, y, z, tileEntity)`.
// src: Chunk.java:434-446
#[allow(clippy::too_many_arguments)] // Java's arguments plus the chunk coordinates
pub fn set_chunk_block_tile_entity(
    world: &mut World,
    cx: i32,
    cz: i32,
    x: i32,
    y: i32,
    z: i32,
    te: TileEntityRef,
) {
    let var5 = ChunkPosition::new(x, y, z);
    let (wx, wz, id) = {
        let c = world.chunk_mut(cx, cz);
        (
            c.x_position * 16 + x,
            c.z_position * 16 + z,
            c.get_block_id(x, y, z),
        )
    };
    if let Some(t) = world.tile_entities.get_mut(&te) {
        t.x_coord = wx;
        t.y_coord = y;
        t.z_coord = wz;
    }
    if id != 0 && block::is_block_container(id) {
        // `instanceof BlockContainer` == isBlockContainer[id] for every vanilla block.
        if let Some(t) = world.tile_entities.get_mut(&te) {
            t.validate();
        }
        world
            .chunk_mut(cx, cz)
            .chunk_tile_entity_map
            .insert(var5, te);
    }
    // else: "Attempted to place a tile entity where there was no entity tile!" (stdout only)
}

/// `removeChunkBlockTileEntity(x, y, z)`.
// src: Chunk.java:448-457
pub fn remove_chunk_block_tile_entity(world: &mut World, cx: i32, cz: i32, x: i32, y: i32, z: i32) {
    let var4 = ChunkPosition::new(x, y, z);
    let c = world.chunk_mut(cx, cz);
    if c.is_chunk_loaded
        && let Some(r) = c.chunk_tile_entity_map.remove(&var4)
        && let Some(t) = world.tile_entities.get_mut(&r)
    {
        t.invalidate();
    }
}

/// `onChunkLoad()`.
// src: Chunk.java:459-467
pub fn on_chunk_load(world: &mut World, cx: i32, cz: i32) {
    let (tes, sections) = {
        let c = world.chunk_mut(cx, cz);
        c.is_chunk_loaded = true;
        let tes: Vec<TileEntityRef> = c.chunk_tile_entity_map.values().copied().collect();
        (tes, c.entities.clone())
    };
    world.add_tile_entity(&tes);
    for section in &sections {
        world.add_loaded_entities(section);
    }
}

/// `onChunkUnload()`.
// src: Chunk.java:469-482
pub fn on_chunk_unload(world: &mut World, cx: i32, cz: i32) {
    let (tes, sections) = {
        let c = world.chunk_mut(cx, cz);
        c.is_chunk_loaded = false;
        let tes: Vec<TileEntityRef> = c.chunk_tile_entity_map.values().copied().collect();
        (tes, c.entities.clone())
    };
    for r in tes {
        if let Some(t) = world.tile_entities.get_mut(&r) {
            t.invalidate();
        }
    }
    for section in &sections {
        world.unload_entities(section);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_formula_by_hand() {
        assert_eq!(block_index(0, 0, 0), 0);
        assert_eq!(block_index(0, 1, 0), 1);
        assert_eq!(block_index(0, 0, 1), 128);
        assert_eq!(block_index(1, 0, 0), 2048);
        assert_eq!(block_index(15, 127, 15), 32767);
        assert_eq!(block_index(3, 64, 5), 3 * 2048 + 5 * 128 + 64);
    }

    #[test]
    fn pure_accessors() {
        let mut c = Chunk::new(Box::new([0; CHUNK_VOLUME]), 2, -1);
        c.blocks[block_index(3, 64, 5)] = 1;
        assert_eq!(c.get_block_id(3, 64, 5), 1);
        c.set_block_metadata(3, 64, 5, 7);
        assert_eq!(c.get_block_metadata(3, 64, 5), 7);
        assert!(c.is_modified);
        c.generate_height_map();
        assert_eq!(c.get_height_value(3, 5), 65);
        assert_eq!(c.height_map[5 << 4 | 3], 65);
        assert_eq!(c.lowest_block_height, 0);
        assert!(c.can_block_see_the_sky(3, 65, 5));
        assert!(!c.can_block_see_the_sky(3, 64, 5));
    }
}
