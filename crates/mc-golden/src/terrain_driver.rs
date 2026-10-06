//! The `kind: "terrain"` driver: generates the scenario's chunks on an
//! [`mc_sim::world::World`] and emits the trace the Java oracle emits
//! (tests/golden/README.md "`kind: "terrain"`"; oracle:
//! `tools/oracle/java/harness/TierA.java`, `terrain`).
//!
//! Use it as the drive closure of [`crate::run`]:
//!
//! ```no_run
//! mc_golden::run("tests/golden/worldgen/overworld-seed-123", mc_golden::terrain_driver::drive).unwrap();
//! ```
//!
//! # Sequence (mirrors `TierA.terrain`)
//!
//! 1. `new World(new SaveHandlerMP(), "harness", provider, seed)` with
//!    `WorldProviderSurface` / `WorldProviderHell` and the provider's
//!    generator ([`WorldProvider::get_chunk_provider`]), then
//!    [`World::make_deterministic`] (`HarnessWorld.makeDeterministic`).
//! 2. For each chunk in list order:
//!    - **generated**: [`World::get_chunk_from_chunk_coords`], then
//!      `chunk_digest` (+ `chunk_full`) with `"stage":"generated"`;
//!    - **populated**: the 3×3 neighbourhood loaded dz outer, dx inner; the
//!      target must then be `isTerrainPopulated` (the oracle throws
//!      otherwise, so this is an error here too); `chunk_digest` (+
//!      `chunk_full`) with `"stage":"populated"`;
//!    - **biome** (overworld only): a fresh
//!      [`WorldChunkManager::new`]`(seed)`'s `loadBlockGeneratorData(null,
//!      x*16, z*16, 16, 16)`, hashed over the first 256 entries.
//!
//! Population of the two generators is WP-1.7 (a recorded no-op until then),
//! so the `populated` lines cannot match yet; `tests/golden_worldgen.rs`
//! filters them by stage.

use mc_sim::save::NoSave;
use mc_sim::world::provider::{Dimension as SimDimension, WorldProvider};
use mc_sim::world::{World, WorldEnvironment};
use mc_sim::worldgen::chunk_manager::WorldChunkManager;

use crate::scenario::{ChunkSel, Dimension};
use crate::trace::{ChunkArrays, Stage};
use crate::{GoldenError, Scenario, ScenarioKind, TraceLine, TraceWriter};

/// Drives a `kind: "terrain"` scenario (module docs). Other kinds are an
/// error.
pub fn drive(scenario: &Scenario, w: &mut TraceWriter) -> Result<(), GoldenError> {
    let ScenarioKind::Terrain {
        seed,
        dimension,
        chunks,
    } = &scenario.kind
    else {
        return Err(GoldenError::other(format!(
            "terrain_driver: scenario {:?} is not kind \"terrain\"",
            scenario.name
        )));
    };
    let seed = *seed;
    let mut world = build_world(seed, *dimension);
    for c in chunks {
        chunk(&mut world, *dimension, c, w)?;
    }
    Ok(())
}

/// `new World(new SaveHandlerMP(), "harness", provider, seed)` +
/// `HarnessWorld.makeDeterministic(world, seed)`.
pub fn build_world(seed: i64, dimension: Dimension) -> World {
    let provider = WorldProvider::new(match dimension {
        Dimension::Overworld => SimDimension::Surface,
        Dimension::Nether => SimDimension::Hell,
    });
    // src: World.java:212 (getChunkProvider with the world seed)
    let generator = provider.get_chunk_provider(seed);
    let mut world = World::new(
        Box::new(NoSave),
        "harness",
        provider,
        seed,
        generator,
        // Overwritten by make_deterministic; vanilla draws these unseeded.
        WorldEnvironment {
            rand_seed: 0,
            field_9437_g: 0,
            lock_timestamp: 0,
        },
    );
    world.make_deterministic(seed);
    world
}

/// The three stages of one chunk entry.
fn chunk(
    world: &mut World,
    dimension: Dimension,
    c: &ChunkSel,
    w: &mut TraceWriter,
) -> Result<(), GoldenError> {
    let (x, z) = (c.x, c.z);

    // 1. generated
    world.get_chunk_from_chunk_coords(x, z);
    digest(world, x, z, Stage::Generated, c.full, w)?;

    // 2. populated: dz outer, dx inner.
    for dz in -1..=1 {
        for dx in -1..=1 {
            world.get_chunk_from_chunk_coords(x + dx, z + dz);
        }
    }
    let populated = world
        .loaded_chunk(x, z)
        .is_some_and(|ch| ch.is_terrain_populated);
    if !populated {
        return Err(GoldenError::other(format!(
            "chunk {x},{z} not populated after loading its neighbours"
        )));
    }
    digest(world, x, z, Stage::Populated, c.full, w)?;

    // 3. biome (overworld only), on a fresh manager.
    if dimension == Dimension::Overworld {
        let mut wcm = WorldChunkManager::new(world.get_random_seed());
        let biomes = wcm.load_block_generator_data(Vec::new(), x * 16, z * 16, 16, 16);
        let names: Vec<&str> = biomes[..256].iter().map(|b| b.biome_name()).collect();
        w.push(TraceLine::biome(
            x,
            z,
            &names,
            &wcm.temperature[..256],
            &wcm.humidity[..256],
        ));
    }
    Ok(())
}

/// `TierA.chunkDigest`: `chunk_digest` and, when `full`, `chunk_full`.
fn digest(
    world: &World,
    x: i32,
    z: i32,
    stage: Stage,
    full: bool,
    w: &mut TraceWriter,
) -> Result<(), GoldenError> {
    let a = world
        .chunk_arrays(x, z)
        .ok_or_else(|| GoldenError::other(format!("chunk {x},{z} not loaded")))?;
    let arrays = ChunkArrays {
        blocks: a.blocks,
        data: a.data,
        skylight: a.skylight,
        blocklight: a.blocklight,
        heightmap: a.heightmap,
    };
    w.push(TraceLine::chunk_digest(x, z, stage, &arrays));
    if full {
        w.push(TraceLine::chunk_full(x, z, stage, &arrays));
    }
    Ok(())
}
