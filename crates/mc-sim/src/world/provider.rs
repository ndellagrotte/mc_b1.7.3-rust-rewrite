//! `WorldProvider` and its surface/nether subclasses (WorldProvider.java,
//! WorldProviderSurface.java, WorldProviderHell.java), minus the parts that
//! belong to other systems: `worldChunkMgr` (biomes, WP-1.4), `getChunkProvider`
//! (the generator is passed to [`crate::world::World::new`]),
//! `canCoordinateBeSpawn` and the sky colour helpers (client).
//!
//! The sky dimension (`WorldProviderSky`) is out of scope (PLAN_v4 D3).

use mc_core::math::jmath;

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
        };
        if dimension == Dimension::Hell {
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
