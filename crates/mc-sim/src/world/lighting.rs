//! Light types and the light-update queue (World.java:1657-1730,
//! MetadataChunkBlock.java, EnumSkyBlock.java).
//!
//! # How vanilla lights the world
//!
//! Light lives in two nibble arrays per chunk ([`super::chunk::Chunk`]). Two
//! mechanisms write them:
//!
//! - **Column code in `Chunk`** ([`super::chunk`]): `generateSkylightMap` and
//!   `relightBlock` set sky light straight down a column when the height map
//!   moves. They run synchronously inside a block change.
//! - **The queue** (this module): every block change also queues box updates
//!   ([`World::schedule_lighting_update`], one per light type, plus sky-light
//!   boxes from the column code). [`World::updating_lighting`] pops them
//!   *last in, first out* and runs [`MetadataChunkBlock::update_light`]
//!   (`func_4127_a`) on each, at most 499 per call; each changed block queues
//!   its neighbours ([`World::neighbor_light_propagation_changed`]), so light
//!   spreads as a flood fill across calls. The client calls `updatingLighting`
//!   once per frame (Minecraft.java:553); the golden harness once per tick and
//!   to exhaustion after each preloaded chunk (tests/golden/README.md).
//!
//! The light *reads* (`getSavedLightValue`, `getBlockLightValue_do`,
//! `getFullBlockLightValue`, `canExistingBlockSeeTheSky`, `getHeightValue`,
//! `setLightValue`) are in `world/mod.rs` with the other block accessors.
//!
//! # Statics
//!
//! `World.lightingUpdatesScheduled` is a Java `static` (World.java:52); here it
//! is the per-world field [`World::lighting_updates_scheduled`] (the oracle
//! resets the static per scenario). It only counts nested
//! `scheduleLightingUpdate_do` calls, which never nest in vanilla, so its
//! `!= 50` guard is always true; it is transcribed anyway.

use super::World;
use crate::block;

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
/// over an inclusive box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MetadataChunkBlock {
    /// `field_1299_a`: which light.
    pub field_1299_a: EnumSkyBlock,
    /// `field_1298_b`: lowest x (see `docs/mappings.md`).
    pub min_x: i32,
    /// `field_1304_c`: lowest y.
    pub min_y: i32,
    /// `field_1303_d`: lowest z.
    pub min_z: i32,
    /// `field_1302_e`: highest x.
    pub max_x: i32,
    /// `field_1301_f`: highest y.
    pub max_y: i32,
    /// `field_1300_g`: highest z.
    pub max_z: i32,
}

impl MetadataChunkBlock {
    /// `new MetadataChunkBlock(type, x0, y0, z0, x1, y1, z1)`.
    // src: MetadataChunkBlock.java:12-20
    #[allow(clippy::too_many_arguments)]
    pub fn new(kind: EnumSkyBlock, x0: i32, y0: i32, z0: i32, x1: i32, y1: i32, z1: i32) -> Self {
        MetadataChunkBlock {
            field_1299_a: kind,
            min_x: x0,
            min_y: y0,
            min_z: z0,
            max_x: x1,
            max_y: y1,
            max_z: z1,
        }
    }

    /// `func_4127_a(world)`: recomputes the light of every block in the box
    /// from its six neighbours, its own emission (block light) or sky access
    /// (sky light) and its opacity; every changed block queues its neighbours.
    // src: MetadataChunkBlock.java:22-153
    pub fn update_light(&mut self, var1: &mut World) {
        let var2 = self.max_x.wrapping_sub(self.min_x).wrapping_add(1);
        let var3 = self.max_y.wrapping_sub(self.min_y).wrapping_add(1);
        let var4 = self.max_z.wrapping_sub(self.min_z).wrapping_add(1);
        let var5 = var2.wrapping_mul(var3).wrapping_mul(var4);
        if var5 > 32768 {
            // "Light too large, skipping!" (stdout only).
            return;
        }
        let mut var6 = 0;
        let mut var7 = 0;
        // Vanilla quirk (transcribed): `var8` is never set, so the per-chunk
        // cache of the neighbour check below is dead and every column runs
        // `doChunksNearChunkExist`. Same result, kept for shape.
        let var8 = false;
        let mut var9 = false;

        // The Java loops re-read the bounds every iteration; only min_y and
        // max_y change inside (clamped), and only once.
        let mut var10 = self.min_x;
        while var10 <= self.max_x {
            let mut var11 = self.min_z;
            while var11 <= self.max_z {
                let var12 = var10 >> 4;
                let var13 = var11 >> 4;
                let var14;
                if var8 && var12 == var6 && var13 == var7 {
                    var14 = var9;
                } else {
                    let mut v = var1.do_chunks_near_chunk_exist(var10, 0, var11, 1);
                    // `getChunkFromChunkCoords(..).isEmpty()`: never an
                    // `EmptyChunk` here (the port has no chunk-less world).
                    if v && var1
                        .get_chunk_from_chunk_coords(var10 >> 4, var11 >> 4)
                        .is_empty()
                    {
                        v = false;
                    }
                    var14 = v;
                    var9 = v;
                    var6 = var12;
                    var7 = var13;
                }

                if var14 {
                    if self.min_y < 0 {
                        self.min_y = 0;
                    }
                    if self.max_y >= 128 {
                        self.max_y = 127;
                    }
                    let mut var27 = self.min_y;
                    while var27 <= self.max_y {
                        self.update_one(var1, var10, var27, var11);
                        var27 += 1;
                    }
                }
                var11 += 1;
            }
            var10 += 1;
        }
    }

    /// The body of `func_4127_a`'s y loop for one block.
    // src: MetadataChunkBlock.java:66-146
    fn update_one(&self, var1: &mut World, var10: i32, var27: i32, var11: i32) {
        let kind = self.field_1299_a;
        let var16 = var1.get_saved_light_value(kind, var10, var27, var11);
        let var18 = var1.get_block_id(var10, var27, var11);
        let mut var19 = block::light_opacity(var18);
        if var19 == 0 {
            var19 = 1;
        }
        let mut var20 = 0;
        match kind {
            EnumSkyBlock::Sky => {
                if var1.can_existing_block_see_the_sky(var10, var27, var11) {
                    var20 = 15;
                }
            }
            EnumSkyBlock::Block => var20 = block::light_value(var18),
        }

        let var28 = if var19 >= 15 && var20 == 0 {
            0
        } else {
            let var21 = var1.get_saved_light_value(kind, var10 - 1, var27, var11);
            let var22 = var1.get_saved_light_value(kind, var10 + 1, var27, var11);
            let var23 = var1.get_saved_light_value(kind, var10, var27 - 1, var11);
            let var24 = var1.get_saved_light_value(kind, var10, var27 + 1, var11);
            let var25 = var1.get_saved_light_value(kind, var10, var27, var11 - 1);
            let var26 = var1.get_saved_light_value(kind, var10, var27, var11 + 1);
            let mut var28 = var21;
            for v in [var22, var23, var24, var25, var26] {
                if v > var28 {
                    var28 = v;
                }
            }
            var28 -= var19;
            if var28 < 0 {
                var28 = 0;
            }
            if var20 > var28 {
                var28 = var20;
            }
            var28
        };

        if var16 != var28 {
            var1.set_light_value(kind, var10, var27, var11, var28);
            let mut var21 = var28 - 1;
            if var21 < 0 {
                var21 = 0;
            }
            // src: MetadataChunkBlock.java:132-145. The -x, -y, -z neighbours
            // always; +x, +y, +z only on the box's far faces (and beyond),
            // since the scan reaches the inner ones anyway.
            var1.neighbor_light_propagation_changed(kind, var10 - 1, var27, var11, var21);
            var1.neighbor_light_propagation_changed(kind, var10, var27 - 1, var11, var21);
            var1.neighbor_light_propagation_changed(kind, var10, var27, var11 - 1, var21);
            if var10 + 1 >= self.max_x {
                var1.neighbor_light_propagation_changed(kind, var10 + 1, var27, var11, var21);
            }
            if var27 + 1 >= self.max_y {
                var1.neighbor_light_propagation_changed(kind, var10, var27 + 1, var11, var21);
            }
            if var11 + 1 >= self.max_z {
                var1.neighbor_light_propagation_changed(kind, var10, var27, var11 + 1, var21);
            }
        }
    }

    /// `func_866_a(x0, y0, z0, x1, y1, z1)`: true if the box is already
    /// covered by this update, or if this update could grow to cover it (the
    /// new box lies within one block of this one) at a cost of at most two
    /// extra volume units, in which case this update is grown in place.
    ///
    /// The volumes compared are `(max - min)` products, not block counts
    /// (vanilla, transcribed): a one-block box has volume 0.
    // src: MetadataChunkBlock.java:155-206
    #[allow(clippy::too_many_arguments)]
    pub fn try_merge(
        &mut self,
        mut var1: i32,
        mut var2: i32,
        mut var3: i32,
        mut var4: i32,
        mut var5: i32,
        mut var6: i32,
    ) -> bool {
        if var1 >= self.min_x
            && var2 >= self.min_y
            && var3 >= self.min_z
            && var4 <= self.max_x
            && var5 <= self.max_y
            && var6 <= self.max_z
        {
            return true;
        }
        let var7 = 1;
        if var1 >= self.min_x - var7
            && var2 >= self.min_y - var7
            && var3 >= self.min_z - var7
            && var4 <= self.max_x + var7
            && var5 <= self.max_y + var7
            && var6 <= self.max_z + var7
        {
            let var8 = self.max_x.wrapping_sub(self.min_x);
            let var9 = self.max_y.wrapping_sub(self.min_y);
            let var10 = self.max_z.wrapping_sub(self.min_z);
            if var1 > self.min_x {
                var1 = self.min_x;
            }
            if var2 > self.min_y {
                var2 = self.min_y;
            }
            if var3 > self.min_z {
                var3 = self.min_z;
            }
            if var4 < self.max_x {
                var4 = self.max_x;
            }
            if var5 < self.max_y {
                var5 = self.max_y;
            }
            if var6 < self.max_z {
                var6 = self.max_z;
            }
            let var11 = var4.wrapping_sub(var1);
            let var12 = var5.wrapping_sub(var2);
            let var13 = var6.wrapping_sub(var3);
            let var14 = var8.wrapping_mul(var9).wrapping_mul(var10);
            let var15 = var11.wrapping_mul(var12).wrapping_mul(var13);
            if var15.wrapping_sub(var14) <= 2 {
                self.min_x = var1;
                self.min_y = var2;
                self.min_z = var3;
                self.max_x = var4;
                self.max_y = var5;
                self.max_z = var6;
                return true;
            }
        }
        false
    }
}

/// `"More than " + 1000000 + " updates, aborting lighting updates"`: the queue
/// length above which `scheduleLightingUpdate_do` drops every pending update.
// src: World.java:1717-1721
pub const LIGHTING_QUEUE_LIMIT: usize = 1000000;

impl World {
    /// `updatingLighting()`: pops and runs queued updates, newest first, until
    /// the queue is empty (returns false) or the budget runs out (returns
    /// true). The budget is `500` decremented *before* each update and
    /// checked `<= 0`, so at most 499 updates run per call. Re-entry (an
    /// update that calls this again) is cut off at depth 50.
    // src: World.java:1657-1684
    pub fn updating_lighting(&mut self) -> bool {
        if self.lighting_updates_counter >= 50 {
            return false;
        }
        self.lighting_updates_counter += 1;
        // try/finally: nothing in the body unwinds in the port.
        let mut var1 = 500;
        let var2 = loop {
            if self.lighting_to_update.is_empty() {
                break false;
            }
            var1 -= 1;
            if var1 <= 0 {
                break true;
            }
            let mut m = self
                .lighting_to_update
                .pop()
                .expect("lightingToUpdate is not empty");
            m.update_light(self);
        };
        self.lighting_updates_counter -= 1;
        var2
    }

    /// `scheduleLightingUpdate(type, x0, y0, z0, x1, y1, z1)`:
    /// `scheduleLightingUpdate_do(…, true)`.
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
        self.schedule_lighting_update_do(kind, x0, y0, z0, x1, y1, z1, true);
    }

    /// `scheduleLightingUpdate_do(type, x0, y0, z0, x1, y1, z1, merge)`.
    ///
    /// Dropped when the box's centre column `((x0 + x1) / 2, (z0 + z1) / 2)`
    /// is not loaded (`blockExists` at y = 64). With `merge`, the five newest
    /// queued updates of the same type are offered the box first
    /// ([`MetadataChunkBlock::try_merge`]); the first that takes it ends the
    /// call. Otherwise the box is appended; a queue longer than 1,000,000 is
    /// cleared.
    // src: World.java:1690-1730
    #[allow(clippy::too_many_arguments)]
    pub fn schedule_lighting_update_do(
        &mut self,
        var1: EnumSkyBlock,
        var2: i32,
        var3: i32,
        var4: i32,
        var5: i32,
        var6: i32,
        var7: i32,
        var8: bool,
    ) {
        if self.world_provider.has_no_sky && var1 == EnumSkyBlock::Sky {
            return;
        }
        self.lighting_updates_scheduled += 1;
        // try/finally around the body: the decrement runs on every exit.
        self.schedule_lighting_update_body(var1, var2, var3, var4, var5, var6, var7, var8);
        self.lighting_updates_scheduled -= 1;
    }

    /// The `try` block of `scheduleLightingUpdate_do`.
    // src: World.java:1695-1724
    #[allow(clippy::too_many_arguments)]
    fn schedule_lighting_update_body(
        &mut self,
        var1: EnumSkyBlock,
        var2: i32,
        var3: i32,
        var4: i32,
        var5: i32,
        var6: i32,
        var7: i32,
        var8: bool,
    ) {
        if self.lighting_updates_scheduled == 50 {
            return;
        }
        // Java int division truncates toward zero, as Rust's does.
        let var9 = var5.wrapping_add(var2) / 2;
        let var10 = var7.wrapping_add(var4) / 2;
        if !self.block_exists(var9, 64, var10) {
            return;
        }
        if self.get_chunk_from_block_coords(var9, var10).is_empty() {
            return;
        }
        let var11 = self.lighting_to_update.len();
        if var8 {
            let var12 = var11.min(5);
            for var13 in 0..var12 {
                let i = self.lighting_to_update.len() - var13 - 1;
                let var14 = &mut self.lighting_to_update[i];
                if var14.field_1299_a == var1 && var14.try_merge(var2, var3, var4, var5, var6, var7)
                {
                    return;
                }
            }
        }
        self.lighting_to_update.push(MetadataChunkBlock::new(
            var1, var2, var3, var4, var5, var6, var7,
        ));
        if self.lighting_to_update.len() > LIGHTING_QUEUE_LIMIT {
            // "More than 1000000 updates, aborting lighting updates" (stdout only).
            self.lighting_to_update.clear();
        }
    }

    /// `neighborLightPropagationChanged(type, x, y, z, value)`: a neighbour
    /// of a changed block should have light `value` (at least its own
    /// emission, or 15 under open sky); if its stored light differs, queue a
    /// one-block update for it.
    // src: World.java:624-644
    pub fn neighbor_light_propagation_changed(
        &mut self,
        var1: EnumSkyBlock,
        var2: i32,
        var3: i32,
        var4: i32,
        mut var5: i32,
    ) {
        if self.world_provider.has_no_sky && var1 == EnumSkyBlock::Sky {
            return;
        }
        if !self.block_exists(var2, var3, var4) {
            return;
        }
        match var1 {
            EnumSkyBlock::Sky => {
                if self.can_existing_block_see_the_sky(var2, var3, var4) {
                    var5 = 15;
                }
            }
            EnumSkyBlock::Block => {
                let var6 = self.get_block_id(var2, var3, var4);
                if block::light_value(var6) > var5 {
                    var5 = block::light_value(var6);
                }
            }
        }
        if self.get_saved_light_value(var1, var2, var3, var4) != var5 {
            self.schedule_lighting_update(var1, var2, var3, var4, var2, var3, var4);
        }
    }

    /// The pending light updates (`lightingToUpdate`), oldest first; the
    /// next one [`World::updating_lighting`] runs is the last.
    pub fn lighting_to_update(&self) -> &[MetadataChunkBlock] {
        &self.lighting_to_update
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(b: [i32; 6]) -> MetadataChunkBlock {
        MetadataChunkBlock::new(EnumSkyBlock::Block, b[0], b[1], b[2], b[3], b[4], b[5])
    }

    fn bounds(m: &MetadataChunkBlock) -> [i32; 6] {
        [m.min_x, m.min_y, m.min_z, m.max_x, m.max_y, m.max_z]
    }

    #[test]
    fn try_merge_by_hand() {
        // Contained: true, unchanged.
        let mut a = m([0, 0, 0, 4, 4, 4]);
        assert!(a.try_merge(1, 1, 1, 2, 2, 2));
        assert_eq!(bounds(&a), [0, 0, 0, 4, 4, 4]);

        // One block (volume 0) next to another one-block box: the merged box
        // 0..1 x 0..0 x 0..0 still has (max-min) volume 0, so it merges.
        let mut b = m([0, 0, 0, 0, 0, 0]);
        assert!(b.try_merge(1, 0, 0, 1, 0, 0));
        assert_eq!(bounds(&b), [0, 0, 0, 1, 0, 0]);

        // 2x2x2 (volume 1) grown by one in x: 2*1*1 = 2, 2 - 1 = 1 <= 2: merges.
        let mut c = m([0, 0, 0, 1, 1, 1]);
        assert!(c.try_merge(2, 0, 0, 2, 0, 0));
        assert_eq!(bounds(&c), [0, 0, 0, 2, 1, 1]);

        // 3x3x3 (volume 8) grown by one in x: 3*2*2 = 12, 12 - 8 = 4 > 2: no.
        let mut d = m([0, 0, 0, 2, 2, 2]);
        assert!(!d.try_merge(3, 0, 0, 3, 0, 0));
        assert_eq!(bounds(&d), [0, 0, 0, 2, 2, 2]);

        // Two blocks away: no.
        let mut e = m([0, 0, 0, 0, 0, 0]);
        assert!(!e.try_merge(2, 0, 0, 2, 0, 0));
    }
}
