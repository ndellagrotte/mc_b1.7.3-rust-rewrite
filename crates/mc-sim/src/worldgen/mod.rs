//! Terrain generation (PLAN_v4 Phase 1).
//!
//! WP-1.4: noise ([`perlin`], [`octaves`], [`simplex`]), biomes
//! ([`biome`], [`chunk_manager`]), caves ([`caves`]) and the two generators
//! up to `provideChunk` ([`generate`] = `ChunkProviderGenerate`, [`hell`] =
//! `ChunkProviderHell`). WP-1.6b: the tree generators ([`trees`]). WP-1.7:
//! the other `WorldGen*` classes ([`features`]) and the providers'
//! `populate` ([`populate`]).
//!
//! Both generators are variants of
//! [`crate::world::chunk_provider::ChunkGenerator`], built by
//! [`crate::world::provider::WorldProvider::get_chunk_provider`]; read
//! `chunk_provider`'s module docs for why `populate` is a function over the
//! world rather than a `&mut self` method. The world's biome manager is
//! [`crate::world::World::get_world_chunk_manager`].
//!
//! Generators write a chunk's block array before the `Chunk` exists; ids
//! are the bytes in [`ids`].
//!
//! Deferred (PLAN_v4 D3): the sky dimension (`WorldProviderSky`,
//! `ChunkProviderSky`, `BiomeGenSky`).
//!
//! (`worldgen`, not the plan's `gen`: `gen` is reserved in Rust 2024.)

pub mod biome;
pub mod caves;
pub mod chunk_manager;
pub mod features;
pub mod generate;
pub mod hell;
pub mod octaves;
pub mod perlin;
pub mod populate;
pub mod simplex;
pub mod trees;

/// The block ids the generators write or test, as the `byte`s Java stores
/// (`(byte)Block.x.blockID`). Values come from [`crate::block::ids`], the
/// single source (Block.java:593-688); only the narrowing cast is here.
pub mod ids {
    use crate::block::ids as b;

    /// `(byte)` of [`b::STONE`].
    pub const STONE: u8 = b::STONE as u8;
    /// `(byte)` of [`b::GRASS`].
    pub const GRASS: u8 = b::GRASS as u8;
    /// `(byte)` of [`b::DIRT`].
    pub const DIRT: u8 = b::DIRT as u8;
    /// `(byte)` of [`b::BEDROCK`].
    pub const BEDROCK: u8 = b::BEDROCK as u8;
    /// `(byte)` of [`b::WATER_MOVING`].
    pub const WATER_MOVING: u8 = b::WATER_MOVING as u8;
    /// `(byte)` of [`b::WATER_STILL`].
    pub const WATER_STILL: u8 = b::WATER_STILL as u8;
    /// `(byte)` of [`b::LAVA_MOVING`].
    pub const LAVA_MOVING: u8 = b::LAVA_MOVING as u8;
    /// `(byte)` of [`b::LAVA_STILL`].
    pub const LAVA_STILL: u8 = b::LAVA_STILL as u8;
    /// `(byte)` of [`b::SAND`].
    pub const SAND: u8 = b::SAND as u8;
    /// `(byte)` of [`b::GRAVEL`].
    pub const GRAVEL: u8 = b::GRAVEL as u8;
    /// `(byte)` of [`b::SAND_STONE`].
    pub const SAND_STONE: u8 = b::SAND_STONE as u8;
    /// `(byte)` of [`b::ICE`].
    pub const ICE: u8 = b::ICE as u8;
    /// `(byte)` of [`b::NETHERRACK`].
    pub const NETHERRACK: u8 = b::NETHERRACK as u8;
    /// `(byte)` of [`b::SLOW_SAND`].
    pub const SLOW_SAND: u8 = b::SLOW_SAND as u8;
}

#[cfg(test)]
mod tests {
    use super::ids;

    /// The byte ids keep their old literal values and name rows of the block
    /// table; the worldgen golden fixtures
    /// (`cargo test -p mc-sim --test golden_worldgen`) prove the generators
    /// are unchanged.
    #[test]
    fn ids_unchanged_and_in_block_table() {
        let expected: [(u8, u8); 14] = [
            (ids::STONE, 1),
            (ids::GRASS, 2),
            (ids::DIRT, 3),
            (ids::BEDROCK, 7),
            (ids::WATER_MOVING, 8),
            (ids::WATER_STILL, 9),
            (ids::LAVA_MOVING, 10),
            (ids::LAVA_STILL, 11),
            (ids::SAND, 12),
            (ids::GRAVEL, 13),
            (ids::SAND_STONE, 24),
            (ids::ICE, 79),
            (ids::NETHERRACK, 87),
            (ids::SLOW_SAND, 88),
        ];
        for (id, value) in expected {
            assert_eq!(id, value);
            assert!(crate::block::block(i32::from(id)).is_some());
        }
    }
}
