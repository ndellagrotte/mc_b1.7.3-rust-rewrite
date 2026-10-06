//! Terrain generation (PLAN_v4 Phase 1).
//!
//! Empty until WP-1.4 (noise, `WorldChunkManager`, biomes,
//! `ChunkProviderGenerate`/`ChunkProviderHell.provideChunk`, `MapGenCaves`)
//! and WP-1.7 (`WorldGen*`, the providers' `populate`). Both generators join
//! [`crate::world::chunk_provider::ChunkGenerator`] as new variants; read that
//! module's docs for why `populate` is a function over the world rather than
//! a `&mut self` method. Until then worlds use
//! [`crate::world::chunk_provider::EmptyChunkProvider`].
//!
//! (`worldgen`, not the plan's `gen`: `gen` is reserved in Rust 2024.)
