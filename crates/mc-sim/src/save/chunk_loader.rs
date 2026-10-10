//! The chunk NBT codec: `ChunkLoader`'s static methods
//! `storeChunkInCompound` and `loadChunkIntoWorldFromCompound`
//! (ChunkLoader.java:108-200), which `McRegionChunkLoader` uses, and
//! `Chunk.removeUnknownBlocks` / `ChunkBlockMap` (Chunk.java:606-608,
//! ChunkBlockMap.java). The rest of `ChunkLoader` (per-chunk files,
//! pre-McRegion) is deferred (PLAN_v4 D3; [`super`] module docs).
//!
//! # The `Level` compound
//!
//! `xPos`, `zPos` (int), `LastUpdate` (long, the world time), `Blocks`
//! (32768 bytes), `Data`, `SkyLight`, `BlockLight` (16384 bytes each),
//! `HeightMap` (256 bytes), `TerrainPopulated` (byte), `Entities` (list of
//! entity compounds, `Entity.addEntityID`), `TileEntities` (list of tile
//! entity compounds, `TileEntity.writeToNBT`, in `chunkTileEntityMap`
//! `HashMap` order).
//!
//! # Differences from the Java, by necessity
//!
//! - **Array lengths.** The Java stores whatever byte arrays it reads
//!   (`getByteArray` of a missing key is `new byte[0]`, and `isValid()` /
//!   `heightMap == null` are never false, so the regeneration branches at
//!   ChunkLoader.java:161-174 are dead code) and throws
//!   `ArrayIndexOutOfBoundsException` at the first access of a short one.
//!   The port's chunk arrays have fixed sizes, so a wrong length is refused
//!   when reading ([`SaveError::Corrupt`]); `ChunkProvider` then generates
//!   the chunk, as it does after any loader exception.
//! - **Entities** are built ([`super::entity_list`]) and filed in the chunk's
//!   sections by id, and handed to the caller ([`LoadedChunk::entities`])
//!   instead of living in the chunk's lists.
//! - **Tile entities** go to [`World::tile_entities`] and the chunk's map.
//!   `Chunk.addTileEntity` reads the chunk's own block to decide whether the
//!   tile entity is kept (Chunk.java:423-446); the chunk is not in the
//!   world yet, so that is done here on the detached chunk.

use mc_core::java_hash::ChunkPosition;
use mc_nbt::{Tag, TagType};

use super::SaveError;
use super::entity_list::{self, FromNbt};
use super::nbt::{self as n, Compound};
use crate::block;
use crate::entity::{Entity, nbt as entity_nbt};
use crate::item::JavaException;
use crate::tile_entity;
use crate::world::World;
use crate::world::chunk::{CHUNK_VOLUME, Chunk};
use crate::world::nibble::NibbleArray;
use crate::world::tile_entity::TileEntityRef;

/// A chunk read by a chunk loader, before it joins the world.
#[derive(Debug)]
pub struct LoadedChunk {
    /// The chunk; its tile entities are in [`World::tile_entities`] and its
    /// `chunkTileEntityMap`, its entity ids in its sections.
    pub chunk: Chunk,
    /// The entities its sections list, in section then list order (the
    /// order `Chunk.onChunkLoad` hands them to `World.addLoadedEntities`,
    /// Chunk.java:459-467).
    pub entities: Vec<Entity>,
}

/// `ChunkLoader.storeChunkInCompound(chunk, world, compound)`: the `Level`
/// compound of `chunk` (taken out of the world's chunk map by the caller).
/// Clears and recomputes `hasEntities`. `Err` only for the session lock
/// check (`MinecraftException`).
// src: ChunkLoader.java:108-149
pub fn store_chunk_in_compound(
    chunk: &mut Chunk,
    world: &mut World,
) -> Result<Compound, SaveError> {
    world.check_session_lock()?; // src: ChunkLoader.java:109
    let mut var2 = Compound::new();
    n::set(&mut var2, "xPos", Tag::Int(chunk.x_position));
    n::set(&mut var2, "zPos", Tag::Int(chunk.z_position));
    n::set(&mut var2, "LastUpdate", Tag::Long(world.get_world_time()));
    n::set(&mut var2, "Blocks", Tag::ByteArray(chunk.blocks.to_vec()));
    n::set(&mut var2, "Data", Tag::ByteArray(chunk.data.data.to_vec()));
    n::set(
        &mut var2,
        "SkyLight",
        Tag::ByteArray(chunk.skylight_map.data.to_vec()),
    );
    n::set(
        &mut var2,
        "BlockLight",
        Tag::ByteArray(chunk.blocklight_map.data.to_vec()),
    );
    n::set(
        &mut var2,
        "HeightMap",
        Tag::ByteArray(chunk.height_map.to_vec()),
    );
    n::set_boolean(&mut var2, "TerrainPopulated", chunk.is_terrain_populated);
    chunk.has_entities = false;
    let mut var3 = Vec::new();
    // src: ChunkLoader.java:124-135
    for section in &chunk.entities {
        for &id in section {
            chunk.has_entities = true;
            match world.entity(id) {
                Some(var6) => {
                    if let Some(var7) = entity_nbt::add_entity_id(var6, world) {
                        var3.push(Tag::Compound(var7));
                    }
                }
                // Every id in a section is in loadedEntityList (dead and
                // unloaded entities leave both together, World.java:1189-1231),
                // and nothing saves during an entity's update.
                None => world.note_unimplemented(
                    "ChunkLoader.storeChunkInCompound: a section lists an entity not in loadedEntityList",
                ),
            }
        }
    }
    n::set(&mut var2, "Entities", n::list_of(TagType::Compound, var3));
    // src: ChunkLoader.java:138-148 (chunkTileEntityMap.values(), HashMap order)
    let mut var8 = Vec::new();
    for r in chunk.chunk_tile_entity_map.values() {
        if let Some(var9) = world.tile_entities.get(r) {
            var8.push(tile_entity::write_to_nbt(var9, world));
        }
    }
    n::set(
        &mut var2,
        "TileEntities",
        n::list_of(TagType::Compound, var8),
    );
    Ok(var2)
}

/// A byte array of the `Level` compound that must have exactly `len`
/// bytes (module docs, "Array lengths").
fn fixed_array<'a>(c: &'a Compound, key: &str, len: usize) -> Result<&'a [u8], SaveError> {
    let a = n::get_byte_array(c, key)?;
    if a.len() != len {
        return Err(SaveError::Corrupt(format!(
            "chunk {key} has {} bytes, not {len}",
            a.len()
        )));
    }
    Ok(a)
}

/// `ChunkLoader.loadChunkIntoWorldFromCompound(world, level)`: a new chunk
/// from the `Level` compound, its entities built (`EntityList.createEntityFromNBT`)
/// and filed in its sections, its tile entities built
/// (`TileEntity.createAndLoadEntity`) and filed in its map (`Chunk.addTileEntity`).
/// `Err` where the Java throws out of the method (a tag of the wrong type,
/// an entity list too short, a tile entity outside the chunk's arrays) or
/// the port refuses an array length (module docs).
// src: ChunkLoader.java:151-200
pub fn load_chunk_into_world_from_compound(
    world: &mut World,
    var1: &Compound,
) -> Result<LoadedChunk, SaveError> {
    let var2 = n::get_integer(var1, "xPos")?;
    let var3 = n::get_integer(var1, "zPos")?;
    let blocks = fixed_array(var1, "Blocks", CHUNK_VOLUME)?;
    let mut var4 = Chunk::new(
        Box::new(blocks.try_into().expect("length checked")),
        var2,
        var3,
    );
    var4.data = NibbleArray::from_bytes(fixed_array(var1, "Data", CHUNK_VOLUME / 2)?.into());
    var4.skylight_map =
        NibbleArray::from_bytes(fixed_array(var1, "SkyLight", CHUNK_VOLUME / 2)?.into());
    var4.blocklight_map =
        NibbleArray::from_bytes(fixed_array(var1, "BlockLight", CHUNK_VOLUME / 2)?.into());
    var4.height_map = fixed_array(var1, "HeightMap", 256)?
        .try_into()
        .expect("length checked");
    var4.is_terrain_populated = n::get_boolean(var1, "TerrainPopulated")?;
    // src: ChunkLoader.java:161-174: `isValid()` and `heightMap == null` are
    // never true for arrays read from a compound (module docs); dead code.

    let mut entities = Vec::new();
    // src: ChunkLoader.java:176-186
    let var5 = n::get_tag_list(var1, "Entities")?;
    for t in var5 {
        let var7 = n::as_compound(t)?;
        let var8 = entity_list::create_entity_from_nbt(var7, world)?;
        var4.has_entities = true;
        match var8 {
            FromNbt::Created(mut e) => {
                var4.add_entity(&mut e.base);
                entities.push(*e);
            }
            FromNbt::Skipped => {}
        }
    }
    // Section order (module docs of `LoadedChunk::entities`).
    let mut ordered = Vec::with_capacity(entities.len());
    for section in &var4.entities {
        for id in section {
            if let Some(i) = entities
                .iter()
                .position(|e: &Entity| e.base.entity_id == *id)
            {
                ordered.push(entities.swap_remove(i));
            }
        }
    }

    // src: ChunkLoader.java:188-197
    let var10 = n::get_tag_list(var1, "TileEntities")?;
    let loaded: Result<(), SaveError> = var10.iter().try_for_each(|t| {
        if let Some(var9) = tile_entity::create_and_load_entity(t, world)? {
            let r = world.new_tile_entity(var9);
            add_tile_entity_detached(&mut var4, world, r)?;
        }
        Ok(())
    });
    if let Err(e) = loaded {
        // The Java chunk (and its tile entities) becomes garbage; the port's
        // tile entities live in `World::tile_entities`, so free them.
        for r in var4.chunk_tile_entity_map.values() {
            world.tile_entities.remove(r);
        }
        return Err(e);
    }
    Ok(LoadedChunk {
        chunk: var4,
        entities: ordered,
    })
}

/// `Chunk.addTileEntity(tileEntity)` on a chunk that is not in the world
/// (module docs): `setChunkBlockTileEntity` with the tile entity's
/// coordinates made chunk-local; kept (validated, in the map) only when the
/// chunk's block there is a `BlockContainer`, otherwise dropped ("Attempted
/// to place a tile entity where there was no entity tile!"). The chunk is
/// not loaded yet (`isChunkLoaded` false), so `loadedTileEntityList` is not
/// touched. A position outside the arrays is Java's
/// `ArrayIndexOutOfBoundsException`.
// src: Chunk.java:423-446
fn add_tile_entity_detached(
    c: &mut Chunk,
    world: &mut World,
    r: TileEntityRef,
) -> Result<(), SaveError> {
    let (tx, ty, tz) = {
        let t = &world.tile_entities[&r];
        (t.x_coord, t.y_coord, t.z_coord)
    };
    let var2 = tx.wrapping_sub(c.x_position.wrapping_mul(16));
    let var3 = ty;
    let var4 = tz.wrapping_sub(c.z_position.wrapping_mul(16));
    // setChunkBlockTileEntity(var2, var3, var4, te), Chunk.java:434-446
    let var5 = ChunkPosition::new(var2, var3, var4);
    if let Some(t) = world.tile_entities.get_mut(&r) {
        t.x_coord = c.x_position.wrapping_mul(16).wrapping_add(var2);
        t.y_coord = var3;
        t.z_coord = c.z_position.wrapping_mul(16).wrapping_add(var4);
    }
    // getBlockID(x, y, z) = blocks[x << 11 | z << 7 | y] & 255 (Chunk.java:232-234)
    let index = var2 << 11 | var4 << 7 | var3;
    let id = match usize::try_from(index) {
        Ok(i) if i < CHUNK_VOLUME => c.blocks[i] as i32,
        _ => {
            world.tile_entities.remove(&r);
            return Err(JavaException::ArrayIndexOutOfBounds.into());
        }
    };
    // `instanceof BlockContainer` == isBlockContainer[id] for every vanilla block.
    if id != 0 && block::is_block_container(id) {
        if let Some(t) = world.tile_entities.get_mut(&r) {
            t.validate();
        }
        // `chunkTileEntityMap.put` replaces: the old tile entity becomes
        // garbage in Java, so the port frees it from `World::tile_entities`.
        if let Some(old) = c.chunk_tile_entity_map.insert(var5, r) {
            world.tile_entities.remove(&old);
        }
    } else {
        // stdout only; the tile entity is dropped.
        world.tile_entities.remove(&r);
    }
    Ok(())
}

/// `Chunk.removeUnknownBlocks()` = `ChunkBlockMap.removeUnknownBlockIDs(blocks)`:
/// every id with no `Block.blocksList` entry becomes air.
// src: Chunk.java:606-608, ChunkBlockMap.java:6-26
pub fn remove_unknown_blocks(c: &mut Chunk) {
    for b in c.blocks.iter_mut() {
        // convTable[id] = id if id == 0 or blocksList[id] != null, else 0
        if *b != 0 && block::block(*b as i32).is_none() {
            *b = 0;
        }
    }
}

/// Drops what a [`LoadedChunk`] put into the world (its tile entities), for
/// a chunk the caller discards (the first read of the relocation bug in
/// `McRegionChunkLoader.loadChunk`, or a dump that only inspects the chunk).
pub fn discard(world: &mut World, l: LoadedChunk) {
    for r in l.chunk.chunk_tile_entity_map.values() {
        world.tile_entities.remove(r);
    }
}
