//! Light types and the lighting entry points `World` calls (WP-1.3 fills them).
//!
//! Transcribed now: [`EnumSkyBlock`], the light *reads* (`getSavedLightValue`,
//! `getBlockLightValue_do`, …, in `world/mod.rs`) and the chunk-local column
//! code in [`super::chunk`]. Stubbed, each recording its Java name in
//! [`World::skipped_systems`] whenever it is reached:
//! `World.scheduleLightingUpdate` (World.java:1686-1730, the queue of
//! `MetadataChunkBlock` updates) and `World.updatingLighting`
//! (World.java:1657-1684, which drains it). Until then the queue stays empty,
//! which is the same observable state as vanilla only when no light changes.

use super::World;

/// `EnumSkyBlock` (EnumSkyBlock.java).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnumSkyBlock {
    /// `Sky` (default light 15).
    Sky,
    /// `Block` (default light 0).
    Block,
}

impl EnumSkyBlock {
    /// `defaultLightValue`.
    // src: EnumSkyBlock.java:4-5
    pub const fn default_light_value(self) -> i32 {
        match self {
            EnumSkyBlock::Sky => 15,
            EnumSkyBlock::Block => 0,
        }
    }
}

/// `MetadataChunkBlock` (MetadataChunkBlock.java): one queued light update
/// over a box. Fields only; `func_4127_a` (the update itself) is WP-1.3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MetadataChunkBlock {
    /// `field_1299_a`: which light.
    pub field_1299_a: EnumSkyBlock,
    /// `field_1298_b`, `field_1304_c`, `field_1303_d`, `field_1302_e`,
    /// `field_1301_f`, `field_1300_g`: the box `(x0, y0, z0, x1, y1, z1)` in
    /// constructor order (MetadataChunkBlock.java:5-20; `func_4127_a` reads
    /// them as min/max, :22-25). Grouped; see `docs/mappings.md`.
    pub bounds: [i32; 6],
}

impl World {
    /// `scheduleLightingUpdate(type, x0, y0, z0, x1, y1, z1)`. **Stub (WP-1.3).**
    // src: World.java:1686-1688
    #[allow(clippy::too_many_arguments)]
    pub fn schedule_lighting_update(
        &mut self,
        kind: EnumSkyBlock,
        x0: i32,
        y0: i32,
        z0: i32,
        x1: i32,
        y1: i32,
        z1: i32,
    ) {
        let _ = (kind, x0, y0, z0, x1, y1, z1);
        self.skip("World.scheduleLightingUpdate");
    }

    /// `updatingLighting()`: drains up to 500 queued updates; the client calls
    /// it once per frame. **Stub (WP-1.3)**: returns false (nothing pending).
    // src: World.java:1657-1684
    pub fn updating_lighting(&mut self) -> bool {
        if self.lighting_updates_counter >= 50 {
            return false;
        }
        if !self.lighting_to_update.is_empty() {
            self.skip("World.updatingLighting");
        }
        false
    }

    /// `neighborLightPropagationChanged(type, x, y, z, value)`. **Stub (WP-1.3).**
    // src: World.java:624-644
    pub fn neighbor_light_propagation_changed(
        &mut self,
        kind: EnumSkyBlock,
        x: i32,
        y: i32,
        z: i32,
        value: i32,
    ) {
        let _ = (kind, x, y, z, value);
        self.skip("World.neighborLightPropagationChanged");
    }
}
