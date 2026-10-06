//! Terrain generation (PLAN_v4 Phase 1).
//!
//! WP-1.4 (this module today): noise ([`perlin`], [`octaves`], [`simplex`]),
//! biomes ([`biome`], [`chunk_manager`]), caves ([`caves`]) and the two
//! generators up to `provideChunk` ([`generate`] = `ChunkProviderGenerate`,
//! [`hell`] = `ChunkProviderHell`). WP-1.7 adds `WorldGen*` and the
//! providers' `populate`.
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
pub mod generate;
pub mod hell;
pub mod octaves;
pub mod perlin;
pub mod simplex;

/// The block ids the generators write or test, as the `byte`s Java stores
/// (`(byte)Block.x.blockID`).
pub mod ids {
    /// `Block.stone` (Block.java:593).
    pub const STONE: u8 = 1;
    /// `Block.grass` (Block.java:594).
    pub const GRASS: u8 = 2;
    /// `Block.dirt` (Block.java:595).
    pub const DIRT: u8 = 3;
    /// `Block.bedrock` (Block.java:599).
    pub const BEDROCK: u8 = 7;
    /// `Block.waterMoving` (Block.java:600).
    pub const WATER_MOVING: u8 = 8;
    /// `Block.waterStill` (Block.java:601).
    pub const WATER_STILL: u8 = 9;
    /// `Block.lavaMoving` (Block.java:602).
    pub const LAVA_MOVING: u8 = 10;
    /// `Block.lavaStill` (Block.java:603).
    pub const LAVA_STILL: u8 = 11;
    /// `Block.sand` (Block.java:604).
    pub const SAND: u8 = 12;
    /// `Block.gravel` (Block.java:605).
    pub const GRAVEL: u8 = 13;
    /// `Block.sandStone` (Block.java:616).
    pub const SAND_STONE: u8 = 24;
    /// `Block.ice` (Block.java:671).
    pub const ICE: u8 = 79;
    /// `Block.netherrack` (Block.java:679).
    pub const NETHERRACK: u8 = 87;
    /// `Block.slowSand` (soul sand, Block.java:680).
    pub const SLOW_SAND: u8 = 88;
}
