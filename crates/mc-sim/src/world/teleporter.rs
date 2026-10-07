//! `Teleporter` (Teleporter.java): where an entity arriving through a nether
//! portal lands, building a portal when none is near (WP-3.1).
//!
//! The caller is the client's `Minecraft.usePortal` (Minecraft.java:1213-1258,
//! `mc-client`'s `game` module): after the dimension change it calls
//! [`Teleporter::func_4107_a`] with the new world and the player, taken out of
//! its slot.
//!
//! `field_4232_a` is `new Random()` (Teleporter.java:6), a clock-seeded
//! stream drawn once per portal build (the orientation `nextInt(4)`); the
//! caller seeds it ([`Teleporter::new`]), as the client seeds every other
//! clock-seeded `Random` at world load.
//!
//! Names: `func_4107_a` places the entity (find, else build and find),
//! `func_4106_b` finds the nearest portal block and moves the entity there,
//! `func_4108_c` builds a portal; they keep their Java names
//! (docs/mappings.md, "Kept").

use mc_core::java_random::JavaRandom;
use mc_core::math::floor_double;

use crate::block::ids::OBSIDIAN;
use crate::entity::Entity;
use crate::world::World;

/// `Block.portal.blockID`.
// src: Block.java:682
const PORTAL: i32 = 90;

/// `Teleporter`.
#[derive(Clone, Debug)]
pub struct Teleporter {
    /// `field_4232_a`: the portal builder's orientation `Random`
    /// (`new Random()`, clock-seeded in vanilla; module docs).
    // src: Teleporter.java:6
    pub field_4232_a: JavaRandom,
}

impl Teleporter {
    /// `new Teleporter()`, with the seed of its `new Random()` passed in
    /// (D9: the client reads the clock).
    // src: Teleporter.java:6
    pub fn new(seed: i64) -> Self {
        Teleporter {
            field_4232_a: JavaRandom::new(seed),
        }
    }

    /// `func_4107_a(world, entity)`: moves the entity to the nearest portal
    /// block within 128 blocks; when there is none, builds a portal near the
    /// entity and moves it there.
    // src: Teleporter.java:8-13
    pub fn func_4107_a(&mut self, var1: &mut World, var2: &mut Entity) {
        if !self.func_4106_b(var1, var2) {
            self.func_4108_c(var1, var2);
            self.func_4106_b(var1, var2);
        }
    }

    /// `func_4106_b(world, entity)`: the nearest portal block (squared
    /// distance from the entity to the block's centre at its lowest portal
    /// block of the column) within ±128 in x and z, every y from 127 down;
    /// the entity is moved to it (shifted half a block toward a neighbouring
    /// portal block on each axis), pitch 0, motion zeroed. False when none.
    /// Loads (and generates) every chunk of the 257×257 square.
    // src: Teleporter.java:15-76
    pub fn func_4106_b(&mut self, var1: &mut World, var2: &mut Entity) -> bool {
        let var3: i16 = 128;
        let mut var4 = -1.0f64;
        let mut var6 = 0i32;
        let mut var7 = 0i32;
        let mut var8 = 0i32;
        let var9 = floor_double(var2.base.pos_x);
        let var10 = floor_double(var2.base.pos_z);

        let mut var18: f64;
        let mut var11 = var9 - var3 as i32;
        while var11 <= var9 + var3 as i32 {
            let var12 = var11 as f64 + 0.5 - var2.base.pos_x;
            let mut var14 = var10 - var3 as i32;
            while var14 <= var10 + var3 as i32 {
                let var15 = var14 as f64 + 0.5 - var2.base.pos_z;
                let mut var17 = 127i32;
                while var17 >= 0 {
                    if var1.get_block_id(var11, var17, var14) == PORTAL {
                        while var1.get_block_id(var11, var17 - 1, var14) == PORTAL {
                            var17 -= 1;
                        }
                        var18 = var17 as f64 + 0.5 - var2.base.pos_y;
                        let var20 = var12 * var12 + var18 * var18 + var15 * var15;
                        if var4 < 0.0 || var20 < var4 {
                            var4 = var20;
                            var6 = var11;
                            var7 = var17;
                            var8 = var14;
                        }
                    }
                    var17 -= 1;
                }
                var14 += 1;
            }
            var11 += 1;
        }

        if var4 >= 0.0 {
            let mut var22 = var6 as f64 + 0.5;
            let var16 = var7 as f64 + 0.5;
            var18 = var8 as f64 + 0.5;
            if var1.get_block_id(var6 - 1, var7, var8) == PORTAL {
                var22 -= 0.5;
            }
            if var1.get_block_id(var6 + 1, var7, var8) == PORTAL {
                var22 += 0.5;
            }
            if var1.get_block_id(var6, var7, var8 - 1) == PORTAL {
                var18 -= 0.5;
            }
            if var1.get_block_id(var6, var7, var8 + 1) == PORTAL {
                var18 += 0.5;
            }
            let yaw = var2.base.rotation_yaw;
            var2.base
                .set_location_and_angles(var22, var16, var18, yaw, 0.0);
            var2.base.motion_x = 0.0;
            var2.base.motion_y = 0.0;
            var2.base.motion_z = 0.0;
            true
        } else {
            false
        }
    }

    /// `func_4108_c(world, entity)`: builds a portal near the entity. Looks
    /// within ±16 in x and z for the closest air column top with room for a
    /// 4-wide frame (solid floor under 3×4 air, in one of four orientations
    /// starting at a random one), else for a 1-deep frame (two
    /// orientations); with neither, places it at the entity (y clamped to
    /// 70..=118) on an obsidian platform with air above it. Then sets the
    /// 4×5 frame (obsidian rim, portal inside) four times with
    /// `editingBlocks` on, notifying the neighbours after each pass.
    /// Always returns true.
    // src: Teleporter.java:78-260
    #[allow(clippy::manual_clamp)] // the Java's two `if`s, transcribed
    pub fn func_4108_c(&mut self, var1: &mut World, var2: &mut Entity) -> bool {
        let var3: i32 = 16;
        let mut var4 = -1.0f64;
        let var6 = floor_double(var2.base.pos_x);
        let var7 = floor_double(var2.base.pos_y);
        let var8 = floor_double(var2.base.pos_z);
        let mut var9 = var6;
        let mut var10 = var7;
        let mut var11 = var8;
        let mut var12 = 0i32;
        let var13 = self.field_4232_a.next_int_bounded(4);

        // src: Teleporter.java:105-152
        let mut var14 = var6 - var3;
        while var14 <= var6 + var3 {
            let var15 = var14 as f64 + 0.5 - var2.base.pos_x;
            let mut var17 = var8 - var3;
            while var17 <= var8 + var3 {
                let var18 = var17 as f64 + 0.5 - var2.base.pos_z;
                // label293: `continue label293` goes to the next var20.
                let mut var20 = 127i32;
                'label293: while var20 >= 0 {
                    if var1.is_air_block(var14, var20, var17) {
                        while var20 > 0 && var1.is_air_block(var14, var20 - 1, var17) {
                            var20 -= 1;
                        }
                        let mut var21 = var13;
                        while var21 < var13 + 4 {
                            let mut var22 = var21 % 2;
                            let mut var23 = 1 - var22;
                            if var21 % 4 >= 2 {
                                var22 = -var22;
                                var23 = -var23;
                            }
                            for var24 in 0..3 {
                                for var25 in 0..4 {
                                    for var26 in -1..4 {
                                        let var27 = var14 + (var25 - 1) * var22 + var24 * var23;
                                        let var28 = var20 + var26;
                                        let var29 = var17 + (var25 - 1) * var23 - var24 * var22;
                                        if var26 < 0
                                            && !var1
                                                .get_block_material(var27, var28, var29)
                                                .is_solid()
                                            || var26 >= 0 && !var1.is_air_block(var27, var28, var29)
                                        {
                                            var20 -= 1;
                                            continue 'label293;
                                        }
                                    }
                                }
                            }
                            let var32 = var20 as f64 + 0.5 - var2.base.pos_y;
                            let var33 = var15 * var15 + var32 * var32 + var18 * var18;
                            if var4 < 0.0 || var33 < var4 {
                                var4 = var33;
                                var9 = var14;
                                var10 = var20;
                                var11 = var17;
                                var12 = var21 % 4;
                            }
                            var21 += 1;
                        }
                    }
                    var20 -= 1;
                }
                var17 += 1;
            }
            var14 += 1;
        }

        // src: Teleporter.java:154-197
        if var4 < 0.0 {
            let mut var14 = var6 - var3;
            while var14 <= var6 + var3 {
                let var15 = var14 as f64 + 0.5 - var2.base.pos_x;
                let mut var17 = var8 - var3;
                while var17 <= var8 + var3 {
                    let var18 = var17 as f64 + 0.5 - var2.base.pos_z;
                    let mut var20 = 127i32;
                    'label231: while var20 >= 0 {
                        if var1.is_air_block(var14, var20, var17) {
                            // vanilla bug: unlike the first search (`var20 > 0 &&`,
                            // Teleporter.java:114) this descent has no floor, so an
                            // air column all the way down (`getBlockId` is 0 below
                            // y 0) never ends. Transcribed with Java's wrapping
                            // `int`.
                            // src: Teleporter.java:164-166
                            while var1.is_air_block(var14, var20.wrapping_sub(1), var17) {
                                var20 = var20.wrapping_sub(1);
                            }
                            let mut var21 = var13;
                            while var21 < var13 + 2 {
                                let var22 = var21 % 2;
                                let var23 = 1 - var22;
                                for var24 in 0..4 {
                                    for var25 in -1..4 {
                                        let var26 = var14 + (var24 - 1) * var22;
                                        let var27 = var20 + var25;
                                        let var28 = var17 + (var24 - 1) * var23;
                                        if var25 < 0
                                            && !var1
                                                .get_block_material(var26, var27, var28)
                                                .is_solid()
                                            || var25 >= 0 && !var1.is_air_block(var26, var27, var28)
                                        {
                                            var20 -= 1;
                                            continue 'label231;
                                        }
                                    }
                                }
                                let var32 = var20 as f64 + 0.5 - var2.base.pos_y;
                                let var33 = var15 * var15 + var32 * var32 + var18 * var18;
                                if var4 < 0.0 || var33 < var4 {
                                    var4 = var33;
                                    var9 = var14;
                                    var10 = var20;
                                    var11 = var17;
                                    var12 = var21 % 2;
                                }
                                var21 += 1;
                            }
                        }
                        var20 -= 1;
                    }
                    var17 += 1;
                }
                var14 += 1;
            }
        }

        // src: Teleporter.java:199-207
        let var30 = var9;
        let mut var16 = var10;
        let var17 = var11;
        let mut var31 = var12 % 2;
        let mut var19 = 1 - var31;
        if var12 % 4 >= 2 {
            var31 = -var31;
            var19 = -var19;
        }

        // src: Teleporter.java:209-232
        if var4 < 0.0 {
            if var10 < 70 {
                var10 = 70;
            }
            if var10 > 118 {
                var10 = 118;
            }
            var16 = var10;
            for var20 in -1..=1 {
                for var21 in 1..3 {
                    for var22 in -1..3 {
                        let var23 = var30 + (var21 - 1) * var31 + var20 * var19;
                        let var24 = var16 + var22;
                        let var25 = var17 + (var21 - 1) * var19 - var20 * var31;
                        let var34 = var22 < 0;
                        var1.set_block_with_notify(
                            var23,
                            var24,
                            var25,
                            if var34 { OBSIDIAN } else { 0 },
                        );
                    }
                }
            }
        }

        // src: Teleporter.java:234-257
        for _var20 in 0..4 {
            var1.editing_blocks = true;
            for var21 in 0..4 {
                for var22 in -1..4 {
                    let var23 = var30 + (var21 - 1) * var31;
                    let var24 = var16 + var22;
                    let var25 = var17 + (var21 - 1) * var19;
                    let var34 = var21 == 0 || var21 == 3 || var22 == -1 || var22 == 3;
                    var1.set_block_with_notify(
                        var23,
                        var24,
                        var25,
                        if var34 { OBSIDIAN } else { PORTAL },
                    );
                }
            }
            var1.editing_blocks = false;
            for var21 in 0..4 {
                for var22 in -1..4 {
                    let var23 = var30 + (var21 - 1) * var31;
                    let var24 = var16 + var22;
                    let var25 = var17 + (var21 - 1) * var19;
                    let id = var1.get_block_id(var23, var24, var25);
                    var1.notify_blocks_of_neighbor_change(var23, var24, var25, id);
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::player::new_entity_player_sp;
    use crate::save::NoSave;
    use crate::world::WorldEnvironment;
    use crate::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
    use crate::world::provider::{Dimension, WorldProvider};

    fn empty_world() -> World {
        let mut w = World::new(
            Box::new(NoSave),
            "t",
            WorldProvider::new(Dimension::Hell),
            1,
            ChunkGenerator::Empty(EmptyChunkProvider),
            WorldEnvironment {
                rand_seed: 0,
                field_9437_g: 0,
                lock_timestamp: 0,
            },
        );
        w.make_deterministic(1);
        w
    }

    #[test]
    fn finds_an_existing_portal_and_centres_on_a_pair() {
        let mut w = empty_world();
        // Two portal blocks side by side in x, two high.
        w.set_block(10, 64, 5, PORTAL);
        w.set_block(11, 64, 5, PORTAL);
        w.set_block(10, 65, 5, PORTAL);
        w.set_block(11, 65, 5, PORTAL);
        let mut p = new_entity_player_sp(&mut w);
        p.base.set_location_and_angles(0.5, 70.0, 0.5, 45.0, 30.0);
        p.base.motion_x = 1.0;
        let mut t = Teleporter::new(0);
        assert!(t.func_4106_b(&mut w, &mut p));
        // Nearest is the lowest block of a column: (10 or 11, 64, 5); x=10 is
        // closer to 0.5, and its +x neighbour is a portal: +0.5.
        assert_eq!(p.base.pos_x, 11.0);
        assert_eq!(p.base.pos_z, 5.5);
        assert_eq!(p.base.rotation_pitch, 0.0);
        assert_eq!(p.base.rotation_yaw, 45.0);
        assert_eq!(p.base.motion_x, 0.0);
    }

    #[test]
    fn builds_a_platform_portal_in_the_void_area_then_finds_it() {
        let mut w = empty_world();
        // Solid floor under the entity so the first search can terminate its
        // descents (an all-air column only stops at y 0 in the first search).
        for x in -20..=20 {
            for z in -20..=20 {
                w.set_block(x, 0, z, 7);
            }
        }
        let mut p = new_entity_player_sp(&mut w);
        p.base.set_location_and_angles(0.5, 40.0, 0.5, 0.0, 0.0);
        let mut t = Teleporter::new(42);
        t.func_4107_a(&mut w, &mut p);
        // The frame is obsidian-rimmed with portal blocks inside, and the
        // player ends on a portal block.
        let (x, y, z) = (
            floor_double(p.base.pos_x),
            floor_double(p.base.pos_y),
            floor_double(p.base.pos_z),
        );
        assert_eq!(w.get_block_id(x, y, z), PORTAL, "at {x} {y} {z}");
        let below = w.get_block_id(x, y - 1, z);
        assert!(below == OBSIDIAN || below == PORTAL, "below: {below}");
    }
}
