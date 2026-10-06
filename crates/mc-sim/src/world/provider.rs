//! `WorldProvider` and its surface/nether subclasses (WorldProvider.java,
//! WorldProviderSurface.java, WorldProviderHell.java), minus the parts that
//! belong to other systems: `canCoordinateBeSpawn` and the sky colour helpers
//! (client).
//!
//! # The biome manager and the generator
//!
//! `registerWorld` (WorldProvider.java:13-17) runs inside the `World`
//! constructor, when the world seed is known; [`WorldProvider::new`] runs
//! before it. So:
//!
//! - `worldChunkMgr` ([`WorldProvider::world_chunk_mgr`]) is built for the
//!   nether in [`WorldProvider::new`] (it needs no seed) and for the surface
//!   on first use by [`World::get_world_chunk_manager`], from the world's
//!   `getRandomSeed()`. Building it draws only from three fresh
//!   `Random(seed * k)`s and leaves its arrays empty, so building it late is
//!   indistinguishable from building it in `registerWorld`.
//! - `getChunkProvider()` is [`WorldProvider::get_chunk_provider`], called by
//!   whoever builds the world, whose result is passed to
//!   [`crate::world::World::new`] (World.java:212).
//!
//! The sky dimension (`WorldProviderSky`) is out of scope (PLAN_v4 D3).

use mc_core::math::jmath;

use super::World;
use super::chunk_provider::ChunkGenerator;
use crate::worldgen::biome::Biome;
use crate::worldgen::chunk_manager::WorldChunkManager;
use crate::worldgen::generate::ChunkProviderGenerate;
use crate::worldgen::hell::ChunkProviderHell;

/// Which `WorldProvider` subclass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dimension {
    /// `WorldProviderSurface` (dimension 0).
    Surface,
    /// `WorldProviderHell` (dimension -1).
    Hell,
}

/// `WorldProvider`.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldProvider {
    /// The subclass.
    pub dimension: Dimension,
    /// `isNether`.
    pub is_nether: bool,
    /// `isHellWorld`.
    pub is_hell_world: bool,
    /// `hasNoSky`.
    pub has_no_sky: bool,
    /// `lightBrightnessTable`.
    pub light_brightness_table: [f32; 16],
    /// `worldType` (0 surface, -1 nether).
    pub world_type: i32,
    /// `worldChunkMgr`; `None` for the surface until
    /// [`World::get_world_chunk_manager`] builds it (module docs).
    pub world_chunk_mgr: Option<WorldChunkManager>,
}

impl WorldProvider {
    /// The provider after `registerWorld` (WorldProvider.java:13-17):
    /// `registerWorldChunkManager` sets the nether flags
    /// (WorldProviderHell.java:4-10), then `generateLightBrightnessTable`.
    pub fn new(dimension: Dimension) -> Self {
        let mut p = WorldProvider {
            dimension,
            is_nether: false,
            is_hell_world: false,
            has_no_sky: false,
            light_brightness_table: [0.0; 16],
            world_type: 0,
            world_chunk_mgr: None,
        };
        if dimension == Dimension::Hell {
            // src: WorldProviderHell.java:5
            p.world_chunk_mgr = Some(WorldChunkManager::new_hell(Biome::Hell, 1.0, 0.0));
            // src: WorldProviderHell.java:6-9
            p.is_nether = true;
            p.is_hell_world = true;
            p.has_no_sky = true;
            p.world_type = -1;
        }
        p.generate_light_brightness_table();
        p
    }

    /// `generateLightBrightnessTable` (ambient 0.05, nether 0.1).
    // src: WorldProvider.java:19-26, WorldProviderHell.java:16-23
    fn generate_light_brightness_table(&mut self) {
        let var1: f32 = match self.dimension {
            Dimension::Surface => 0.05,
            Dimension::Hell => 0.1,
        };
        for var2 in 0..=15 {
            let var3 = 1.0f32 - var2 as f32 / 15.0f32;
            self.light_brightness_table[var2] =
                (1.0 - var3) / (var3 * 3.0 + 1.0) * (1.0 - var1) + var1;
        }
    }

    /// `getChunkProvider()`: a new generator for the world seed
    /// (`ChunkProviderGenerate` for the surface, `ChunkProviderHell` for the
    /// nether).
    // src: WorldProvider.java:33-35, WorldProviderHell.java:26-28
    pub fn get_chunk_provider(&self, random_seed: i64) -> ChunkGenerator {
        match self.dimension {
            Dimension::Surface => {
                ChunkGenerator::Generate(Box::new(ChunkProviderGenerate::new(random_seed)))
            }
            Dimension::Hell => ChunkGenerator::Hell(Box::new(ChunkProviderHell::new(random_seed))),
        }
    }

    /// `calculateCelestialAngle(worldTime, partialTick)`; the nether returns 0.5.
    // src: WorldProvider.java:43-58, WorldProviderHell.java:38-40
    pub fn calculate_celestial_angle(&self, world_time: i64, partial: f32) -> f32 {
        if self.dimension == Dimension::Hell {
            return 0.5;
        }
        let var4 = (world_time % 24000) as i32;
        let mut var5 = (var4 as f32 + partial) / 24000.0f32 - 0.25f32;
        if var5 < 0.0 {
            var5 += 1.0;
        }
        if var5 > 1.0 {
            var5 -= 1.0;
        }
        let var6 = var5;
        var5 = 1.0f32 - ((jmath::cos(var5 as f64 * 3.141592653589793) + 1.0) / 2.0) as f32;
        var5 = var6 + (var5 - var6) / 3.0f32;
        var5
    }
}

impl World {
    /// `getWorldChunkManager()` = `worldProvider.worldChunkMgr`, built on first
    /// use for the surface (module docs).
    // src: World.java:58-60, WorldProvider.java:29-31
    pub fn get_world_chunk_manager(&mut self) -> &mut WorldChunkManager {
        let seed = self.get_random_seed();
        self.world_provider
            .world_chunk_mgr
            .get_or_insert_with(|| WorldChunkManager::new(seed))
    }
}
