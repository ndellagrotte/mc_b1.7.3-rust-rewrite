//! The population features: every `WorldGenerator` subclass that is not a
//! tree (the trees are [`super::trees`]), transcribed for the providers'
//! `populate` ([`super::populate`], WP-1.7).
//!
//! # Shape
//!
//! `populate` always names the concrete class (`new WorldGenLakes(id)
//! .generate(...)`, ChunkProviderGenerate.java:327), never selects one at
//! run time, so each class is a plain struct holding its constructor
//! arguments (fieldless classes are unit structs) with the uniform
//! `generate(&self, world, rand, x, y, z) -> bool` of
//! `WorldGenerator.generate` (WorldGenerator.java:6). Only the trees need the
//! closed enum [`super::trees::WorldGen`] (a biome picks the class).
//!
//! | Class | Here |
//! |---|---|
//! | `WorldGenMinable`, `WorldGenClay` | [`minable`] |
//! | `WorldGenLakes` | [`lakes`] |
//! | `WorldGenDungeons` | [`dungeons`] |
//! | `WorldGenFlowers`, `WorldGenTallGrass`, `WorldGenDeadBush`, `WorldGenPumpkin`, `WorldGenReed`, `WorldGenCactus` | [`plants`] |
//! | `WorldGenLiquids`, `WorldGenHellLava` | [`liquids`] |
//! | `WorldGenFire`, `WorldGenGlowStone1`, `WorldGenGlowStone2` | [`nether`] |
//!
//! # Rules every feature follows
//!
//! - `rand` is the [`RandomSource`] the caller passes (population passes
//!   [`RandomSource::ChunkProviderRand`]); every draw goes through
//!   [`World::random`] at the point Java draws, so a chunk generated in the
//!   middle of a feature (a world read outside the loaded area generates and
//!   possibly populates it) reseeds the very `Random` the feature continues
//!   with, as in vanilla.
//! - World reads happen in Java's order and with Java's short-circuits: a
//!   read may load a chunk, and chunk loading is gameplay-visible (it runs
//!   generation and population).
//! - Each write uses the `World` method the Java calls: `setBlock`
//!   ([`World::set_block`]: `onBlockAdded`, no neighbour notification),
//!   `setBlockAndMetadata` ([`World::set_block_and_metadata`]) or
//!   `setBlockWithNotify` ([`World::set_block_with_notify`]: also
//!   `notifyBlockChange`).
//! - Java's argument evaluation order is kept where an argument draws
//!   (`2 + rand.nextInt(rand.nextInt(3) + 1)` draws the inner bound first).

pub mod dungeons;
pub mod lakes;
pub mod liquids;
pub mod minable;
pub mod nether;
pub mod plants;

use crate::block::dispatch::RandomSource;
use crate::world::World;

/// `rand.nextInt(n)` on the caller's `Random`.
pub(crate) fn next_int(world: &mut World, rand: RandomSource, n: i32) -> i32 {
    world.random(rand).next_int_bounded(n)
}

/// `rand.nextFloat()` on the caller's `Random`.
pub(crate) fn next_float(world: &mut World, rand: RandomSource) -> f32 {
    world.random(rand).next_float()
}

/// `rand.nextDouble()` on the caller's `Random`.
pub(crate) fn next_double(world: &mut World, rand: RandomSource) -> f64 {
    world.random(rand).next_double()
}
