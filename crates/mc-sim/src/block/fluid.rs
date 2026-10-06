//! The fluids: `BlockFluid` (BlockFluid.java) and its two subclasses
//! `BlockFlowing` (BlockFlowing.java, ids 8 and 10) and `BlockStationary`
//! (BlockStationary.java, ids 9 and 11), all in this file: the top level is
//! `BlockFluid`, [`flowing`] and [`stationary`] are the subclasses (inline
//! modules, so `block/mod.rs` needs no new `mod` lines).
//!
//! Fully transcribed, every method of the three classes:
//!
//! - `BlockFluid`: `colorMultiplier`, `getPercentAir`, `getFlowDecay`,
//!   `getEffectiveFlowDecay`, `getIsBlockSolid`, `shouldSideBeRendered`,
//!   `getCollisionBoundingBoxFromPool`, `getFlowVector`,
//!   `velocityToAddToEntity`, `getBlockBrightness`, `randomDisplayTick`, the
//!   static `getFlowDirection` (RenderBlocks.java:1517), `onBlockAdded`,
//!   `onNeighborBlockChange`, `checkForHarden`, `triggerLavaMixEffects`.
//!   `canCollideCheck` is in [`super::dispatch::can_collide_check`]; the
//!   constant and field-read methods (`getBlockTextureFromSide`,
//!   `renderAsNormalBlock`, `isOpaqueCube`, `getRenderType`, `idDropped`,
//!   `quantityDropped`, `tickRate`, `getRenderBlockPass`) are data methods in
//!   [`crate::block`]. `BlockFluid.updateTick` (BlockFluid.java:201-203, a
//!   `super` call to the empty base) is unreachable: both subclasses override
//!   it without calling `super`.
//! - `BlockFlowing`: `updateFlow`, `updateTick`, `flowIntoBlock`,
//!   `calculateFlowCost`, `getOptimalFlowDirections`, `blockBlocksFlow`,
//!   `getSmallestFlowDecay`, `liquidCanDisplaceBlock`, `onBlockAdded`.
//! - `BlockStationary`: `onNeighborBlockChange`, `setNotStationary`,
//!   `updateTick`, `isFlammable`.
//!
//! `this` is the block id the dispatch passes (`id`); `this.blockMaterial` is
//! that row's material ([`material`]). Java reads `this.blockID + 1` /
//! `this.blockID - 1` for the flowing/still partner; so does the port.
//!
//! # Singleton scratch state
//!
//! `BlockFlowing`'s instance fields `numAdjacentSources`,
//! `isOptimalFlowDirection` and `flowCost` belong to the `Block.waterMoving`
//! / `Block.lavaMoving` singletons and are world state
//! ([`super::BlockStatics::water_moving`] / [`super::BlockStatics::lava_moving`]).
//! Every read and write goes through them, never through a local copy:
//! `getOptimalFlowDirections` returns the singleton's own array, and
//! `updateTick` reads `var13[1..3]` after `flowIntoBlock` has run, which can
//! re-enter `updateTick` on the same singleton when
//! `World.scheduledUpdatesAreImmediate` is set (population's
//! `WorldGenLiquids` / `WorldGenHellLava`, WorldGenLiquids.java:56-58) and
//! overwrite the array (and, through a chunk load that populates, `flowCost`
//! in the middle of the cost search). Reading the live fields reproduces that.
//!
//! # Randomness
//!
//! `updateTick` draws from the `Random` the caller passes (`World.rand` in
//! the world, [`RandomSource`]); `randomDisplayTick` from the display stream
//! the dispatch passes; `triggerLavaMixEffects` draws `World.rand` (the fizz
//! pitch) and the `Math.random()` stream ([`World::math_random`], census §1
//! site BlockFluid.java:290, class D: two draws per smoke particle, eight
//! particles, whose values only reach the client).

use mc_core::java_random::JavaRandom;
use mc_core::{Aabb, Vec3D, jmath};

use super::dispatch::{self, RandomSource};
use super::{FlowingScratch, Material, block};
use crate::entity::Entity;
use crate::world::World;

/// `Block.obsidian.blockID`.
// src: Block.java:641
const OBSIDIAN_ID: i32 = 49;
/// `Block.cobblestone.blockID`.
// src: Block.java:596
const COBBLESTONE_ID: i32 = 4;
/// `Block.fire.blockID`.
// src: Block.java:643
const FIRE_ID: i32 = 51;
/// `Block.waterMoving.blockID`.
// src: Block.java:600
const WATER_MOVING_ID: i32 = 8;
/// `Block.lavaMoving.blockID`.
// src: Block.java:602
const LAVA_MOVING_ID: i32 = 10;

/// `this.blockMaterial` of the fluid singleton `id` (`Material.air` for an id
/// with no block, which the dispatch never passes).
fn material(id: i32) -> Material {
    super::material_of(id)
}

/// `Math.random()` (census §1): the world's `Math.random()` stream.
fn math_random(world: &mut World) -> f64 {
    world.math_random.next_double()
}

// ---- BlockFluid -------------------------------------------------------------------

/// `BlockFluid.colorMultiplier(blockAccess, x, y, z)`: white.
// src: BlockFluid.java:14-16
pub fn color_multiplier() -> i32 {
    16777215
}

/// `BlockFluid.getPercentAir(meta)` (static): the empty fraction of a fluid
/// cell.
// src: BlockFluid.java:18-25
pub fn get_percent_air(meta: i32) -> f32 {
    let meta = if meta >= 8 { 0 } else { meta };
    (meta + 1) as f32 / 9.0
}

/// `BlockFluid.getFlowDecay(world, x, y, z)` (protected): the metadata of a
/// block of this fluid's material, -1 for anything else.
// src: BlockFluid.java:31-33
fn get_flow_decay(world: &mut World, id: i32, x: i32, y: i32, z: i32) -> i32 {
    if world.get_block_material(x, y, z) != material(id) {
        -1
    } else {
        world.get_block_metadata(x, y, z)
    }
}

/// `BlockFluid.getEffectiveFlowDecay(blockAccess, x, y, z)` (protected): as
/// [`get_flow_decay`], with falling fluid (metadata 8 and up) read as 0.
// src: BlockFluid.java:35-46
fn get_effective_flow_decay(world: &mut World, id: i32, x: i32, y: i32, z: i32) -> i32 {
    if world.get_block_material(x, y, z) != material(id) {
        -1
    } else {
        let mut var5 = world.get_block_metadata(x, y, z);
        if var5 >= 8 {
            var5 = 0;
        }
        var5
    }
}

/// `BlockFluid.getIsBlockSolid(blockAccess, x, y, z, side)`: never this
/// fluid or ice; always the top face; otherwise the base `Block` body (the
/// material at `x, y, z` is solid).
// src: BlockFluid.java:60-69
pub fn get_is_block_solid(world: &mut World, id: i32, x: i32, y: i32, z: i32, side: i32) -> bool {
    let var6 = world.get_block_material(x, y, z);
    if var6 == material(id) || var6 == Material::Ice {
        false
    } else if side == 1 {
        true
    } else {
        // src: Block.java:253-255 (super.getIsBlockSolid)
        world.get_block_material(x, y, z).is_solid()
    }
}

/// `BlockFluid.shouldSideBeRendered(blockAccess, x, y, z, side)`: never
/// against this fluid or ice; always the top face; otherwise `base`, the
/// `Block` body (`super.shouldSideBeRendered`, which the dispatch passes).
// src: BlockFluid.java:71-80
pub fn should_side_be_rendered(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    side: i32,
    base: fn(&mut World, i32, i32, i32, i32, i32) -> bool,
) -> bool {
    let var6 = world.get_block_material(x, y, z);
    if var6 == material(id) || var6 == Material::Ice {
        false
    } else if side == 1 {
        true
    } else {
        base(world, id, x, y, z, side)
    }
}

/// `BlockFluid.getCollisionBoundingBoxFromPool(world, x, y, z)`: null.
// src: BlockFluid.java:82-84
pub fn get_collision_bounding_box_from_pool() -> Option<Aabb> {
    None
}

/// `BlockFluid.getFlowVector(blockAccess, x, y, z)` (private): the
/// horizontal decay gradient towards the four neighbours (a neighbour that
/// is not this fluid and not solid counts through the block below it), plus
/// a strong downward pull for falling fluid next to a solid side, normalised.
// src: BlockFluid.java:98-178
fn get_flow_vector(world: &mut World, id: i32, x: i32, y: i32, z: i32) -> Vec3D {
    let mut var5 = Vec3D::create_vector(0.0, 0.0, 0.0);
    let var6 = get_effective_flow_decay(world, id, x, y, z);
    // src: BlockFluid.java:102-135 (directions -x, -z, +x, +z)
    for var7 in 0..4 {
        let mut var8 = x;
        let mut var10 = z;
        if var7 == 0 {
            var8 = x - 1;
        }
        if var7 == 1 {
            var10 = z - 1;
        }
        if var7 == 2 {
            var8 += 1;
        }
        if var7 == 3 {
            var10 += 1;
        }
        let mut var11 = get_effective_flow_decay(world, id, var8, y, var10);
        if var11 < 0 {
            if !world.get_block_material(var8, y, var10).get_is_solid() {
                var11 = get_effective_flow_decay(world, id, var8, y - 1, var10);
                if var11 >= 0 {
                    // src: BlockFluid.java:127-128
                    let var12 = var11.wrapping_sub(var6 - 8);
                    var5 = var5.add_vector(
                        (var8 - x).wrapping_mul(var12) as f64,
                        0.0, // (var3 - var3) * var12
                        (var10 - z).wrapping_mul(var12) as f64,
                    );
                }
            }
        } else if var11 >= 0 {
            // src: BlockFluid.java:132-133
            let var12 = var11 - var6;
            var5 = var5.add_vector(
                (var8 - x).wrapping_mul(var12) as f64,
                0.0, // (var3 - var3) * var12
                (var10 - z).wrapping_mul(var12) as f64,
            );
        }
    }

    // src: BlockFluid.java:137-174 (`this.getIsBlockSolid` is BlockFluid's
    // own override; the `var13 ||` guards skip the remaining queries once one
    // holds)
    if world.get_block_metadata(x, y, z) >= 8 {
        let checks = [
            (x, y, z - 1, 2),
            (x, y, z + 1, 3),
            (x - 1, y, z, 4),
            (x + 1, y, z, 5),
            (x, y + 1, z - 1, 2),
            (x, y + 1, z + 1, 3),
            (x - 1, y + 1, z, 4),
            (x + 1, y + 1, z, 5),
        ];
        let mut var13 = false;
        for (cx, cy, cz, side) in checks {
            if var13 || get_is_block_solid(world, id, cx, cy, cz, side) {
                var13 = true;
            }
        }
        if var13 {
            // src: BlockFluid.java:172
            var5 = var5.normalize().add_vector(0.0, -6.0, 0.0);
        }
    }

    var5.normalize()
}

/// `BlockFluid.velocityToAddToEntity(world, x, y, z, entity, vec)`: adds the
/// flow vector to `vec` (`World.handleMaterialAcceleration`'s sum).
// src: BlockFluid.java:180-185
pub fn velocity_to_add_to_entity(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    e: &Entity,
    v: &mut Vec3D,
) {
    let _ = e;
    let var7 = get_flow_vector(world, id, x, y, z);
    v.x_coord += var7.x_coord;
    v.y_coord += var7.y_coord;
    v.z_coord += var7.z_coord;
}

/// `BlockFluid.getBlockBrightness(blockAccess, x, y, z)`: the brighter of
/// the cell and the cell above.
// src: BlockFluid.java:195-199
pub fn get_block_brightness(world: &mut World, x: i32, y: i32, z: i32) -> f32 {
    let var5 = world.get_light_brightness(x, y, z);
    let var6 = world.get_light_brightness(x, y + 1, z);
    if var5 > var6 { var5 } else { var6 }
}

/// `BlockFluid.randomDisplayTick(world, x, y, z, rand)`: the flowing-water
/// sound (1 in 64, metadata 1..7) and the lava pop particle (1 in 100, under
/// open air). Draws only from `rand`.
// src: BlockFluid.java:209-224
pub fn random_display_tick(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    rand: &mut JavaRandom,
) {
    let mat = material(id);
    if mat == Material::Water && rand.next_int_bounded(64) == 0 {
        let var6 = world.get_block_metadata(x, y, z);
        if var6 > 0 && var6 < 8 {
            // src: BlockFluid.java:213 (volume, then pitch: argument order)
            let volume = rand.next_float() * 0.25 + 0.75;
            let pitch = rand.next_float() * 1.0 + 0.5;
            world.play_sound_effect(
                (x as f32 + 0.5) as f64,
                (y as f32 + 0.5) as f64,
                (z as f32 + 0.5) as f64,
                "liquid.water",
                volume,
                pitch,
            );
        }
    }

    // src: BlockFluid.java:217 (`&&` order: material, opacity, then the draw)
    if mat == Material::Lava
        && world.get_block_material(x, y + 1, z) == Material::Air
        && !world.is_block_opaque_cube(x, y + 1, z)
        && rand.next_int_bounded(100) == 0
    {
        let var12 = (x as f32 + rand.next_float()) as f64;
        // src: BlockFluid.java:219 (`this.maxY`, the singleton's live bounds)
        let var8 = y as f64 + world.block_statics.bounds[(id & 255) as usize].max_y;
        let var10 = (z as f32 + rand.next_float()) as f64;
        world.spawn_particle("lava", var12, var8, var10, 0.0, 0.0, 0.0);
    }
}

/// `BlockFluid.getFlowDirection(blockAccess, x, y, z, material)` (static):
/// the flow angle the renderer turns the fluid texture by
/// (RenderBlocks.java:1517), `-1000` for no horizontal flow. Computed with the
/// `Block.waterMoving` / `Block.lavaMoving` singleton. `None` for another
/// material, where Java throws a `NullPointerException` (no caller passes
/// one).
// src: BlockFluid.java:226-237
pub fn get_flow_direction(world: &mut World, x: i32, y: i32, z: i32, mat: Material) -> Option<f64> {
    let mut var5 = None;
    if mat == Material::Water {
        var5 = Some(get_flow_vector(world, WATER_MOVING_ID, x, y, z));
    }
    if mat == Material::Lava {
        var5 = Some(get_flow_vector(world, LAVA_MOVING_ID, x, y, z));
    }
    let var5 = var5?;
    Some(if var5.x_coord == 0.0 && var5.z_coord == 0.0 {
        -1000.0
    } else {
        // src: BlockFluid.java:236 (Math.atan2, PLAN_v4 §3.4 rule 5)
        jmath::atan2(var5.z_coord, var5.x_coord) - 1.5707963267948966
    })
}

/// `BlockFluid.onBlockAdded(world, x, y, z)`: `checkForHarden`.
// src: BlockFluid.java:239-241
pub fn on_block_added(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    check_for_harden(world, id, x, y, z);
}

/// `BlockFluid.onNeighborBlockChange(world, x, y, z, neighborId)`:
/// `checkForHarden`.
// src: BlockFluid.java:243-245
pub fn on_neighbor_block_change(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    check_for_harden(world, id, x, y, z);
}

/// `BlockFluid.checkForHarden(world, x, y, z)` (private): lava (this block
/// still in place) touching water on a side or from above becomes obsidian
/// (source) or cobblestone (metadata 1..4), with the fizz. Metadata 5 and up
/// (and falling lava, 8 and up) stays lava but still fizzes, every time a
/// neighbour changes (vanilla, transcribed).
// src: BlockFluid.java:247-284
fn check_for_harden(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    if world.get_block_id(x, y, z) != id {
        return;
    }
    if material(id) == Material::Lava {
        // src: BlockFluid.java:250-269 (-z, +z, -x, +x, +y; never below;
        // the `var5 ||` guards skip the remaining queries once one holds)
        let neighbours = [
            (x, y, z - 1),
            (x, y, z + 1),
            (x - 1, y, z),
            (x + 1, y, z),
            (x, y + 1, z),
        ];
        let mut var5 = false;
        for (nx, ny, nz) in neighbours {
            if var5 || world.get_block_material(nx, ny, nz) == Material::Water {
                var5 = true;
            }
        }
        if var5 {
            let var6 = world.get_block_metadata(x, y, z);
            if var6 == 0 {
                world.set_block_with_notify(x, y, z, OBSIDIAN_ID);
            } else if var6 <= 4 {
                world.set_block_with_notify(x, y, z, COBBLESTONE_ID);
            }
            trigger_lava_mix_effects(world, x, y, z);
        }
    }
}

/// `BlockFluid.triggerLavaMixEffects(world, x, y, z)` (protected): the
/// `random.fizz` sound (two `World.rand` draws for the pitch) and eight
/// `largesmoke` particles (two `Math.random()` draws each, x then z).
// src: BlockFluid.java:286-293
fn trigger_lava_mix_effects(world: &mut World, x: i32, y: i32, z: i32) {
    // src: BlockFluid.java:287 (`rand.nextFloat() - rand.nextFloat()`, left first)
    let a = world.rand.next_float();
    let b = world.rand.next_float();
    let pitch = 2.6 + (a - b) * 0.8;
    world.play_sound_effect(
        (x as f32 + 0.5) as f64,
        (y as f32 + 0.5) as f64,
        (z as f32 + 0.5) as f64,
        "random.fizz",
        0.5,
        pitch,
    );
    // src: BlockFluid.java:289-291 (argument order: x draw, then z draw)
    for _var5 in 0..8 {
        let px = x as f64 + math_random(world);
        let py = y as f64 + 1.2;
        let pz = z as f64 + math_random(world);
        world.spawn_particle("largesmoke", px, py, pz, 0.0, 0.0, 0.0);
    }
}

// ---- BlockFlowing -----------------------------------------------------------------

/// `BlockFlowing` (BlockFlowing.java): the spreading fluids, `Block.waterMoving`
/// (8) and `Block.lavaMoving` (10).
pub mod flowing {
    use super::*;

    /// The scratch fields of the singleton `id` (module docs, "Singleton
    /// scratch state").
    fn scratch(world: &World, id: i32) -> &FlowingScratch {
        world
            .block_statics
            .flowing_scratch(id)
            .expect("BlockFlowing singleton (8 or 10)")
    }

    /// The scratch fields of the singleton `id`, for writing.
    fn scratch_mut(world: &mut World, id: i32) -> &mut FlowingScratch {
        match id {
            WATER_MOVING_ID => &mut world.block_statics.water_moving,
            LAVA_MOVING_ID => &mut world.block_statics.lava_moving,
            _ => panic!("BlockFlowing singleton (8 or 10), got {id}"),
        }
    }

    /// `BlockFlowing.updateFlow(world, x, y, z)` (private): turns this block
    /// into its still partner (`blockID + 1`, same metadata) without
    /// notification.
    // src: BlockFlowing.java:14-19
    fn update_flow(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
        let var5 = world.get_block_metadata(x, y, z);
        world.set_block_and_metadata(x, y, z, id + 1, var5);
        world.mark_blocks_dirty(x, y, z, x, y, z);
        world.mark_block_needs_update(x, y, z);
    }

    /// `BlockFlowing.updateTick(world, x, y, z, rand)`: recompute this
    /// block's level from its neighbours (decay, two-source water renewal,
    /// lava's 3-in-4 stall), then spread down, or sideways along the
    /// cheapest paths.
    // src: BlockFlowing.java:21-114
    pub fn update_tick(world: &mut World, id: i32, x: i32, y: i32, z: i32, rand: RandomSource) {
        let mat = material(id);
        let mut var6 = get_flow_decay(world, id, x, y, z);
        // src: BlockFlowing.java:23-26 (lava spreads by 2 levels per block
        // outside the Nether, by 1 in it)
        let mut var7: i32 = 1;
        if mat == Material::Lava && !world.world_provider.is_hell_world {
            var7 = 2;
        }

        let mut var8 = true;
        let mut var10: i32;
        if var6 > 0 {
            // src: BlockFlowing.java:31-36 (-x, +x, -z, +z)
            let var9: i32 = -100;
            scratch_mut(world, id).num_adjacent_sources = 0;
            let mut var12 = get_smallest_flow_decay(world, id, x - 1, y, z, var9);
            var12 = get_smallest_flow_decay(world, id, x + 1, y, z, var12);
            var12 = get_smallest_flow_decay(world, id, x, y, z - 1, var12);
            var12 = get_smallest_flow_decay(world, id, x, y, z + 1, var12);
            var10 = var12 + var7;
            if var10 >= 8 || var12 < 0 {
                var10 = -1;
            }

            // src: BlockFlowing.java:42-49 (fluid above: falling level)
            if get_flow_decay(world, id, x, y + 1, z) >= 0 {
                let var11 = get_flow_decay(world, id, x, y + 1, z);
                if var11 >= 8 {
                    var10 = var11;
                } else {
                    var10 = var11 + 8;
                }
            }

            // src: BlockFlowing.java:51-57 (infinite water)
            if scratch(world, id).num_adjacent_sources >= 2 && mat == Material::Water {
                // Java's `if / else if` with equal bodies, as one `||` (same
                // query order and short-circuit).
                if world.get_block_material(x, y - 1, z).is_solid()
                    || (world.get_block_material(x, y - 1, z) == mat
                        && world.get_block_metadata(x, y, z) == 0)
                {
                    var10 = 0;
                }
            }

            // src: BlockFlowing.java:59-62 (the draw is last in the `&&` chain)
            if mat == Material::Lava
                && var6 < 8
                && var10 < 8
                && var10 > var6
                && world.random(rand).next_int_bounded(4) != 0
            {
                var10 = var6;
                var8 = false;
            }

            if var10 != var6 {
                var6 = var10;
                if var10 < 0 {
                    world.set_block_with_notify(x, y, z, 0);
                } else {
                    world.set_block_metadata_with_notify(x, y, z, var10);
                    world.schedule_block_update(x, y, z, id, dispatch::tick_rate(id));
                    world.notify_blocks_of_neighbor_change(x, y, z, id);
                }
            } else if var8 {
                update_flow(world, id, x, y, z);
            }
        } else {
            update_flow(world, id, x, y, z);
        }

        // src: BlockFlowing.java:80-112. vanilla bug: when the block has just
        // dried up above (var6 = -1, set to air), this still runs, and with a
        // displaceable block below it places falling fluid of level
        // -1 + 8 = 7 there.
        if liquid_can_displace_block(world, id, x, y - 1, z) {
            if var6 >= 8 {
                world.set_block_and_metadata_with_notify(x, y - 1, z, id, var6);
            } else {
                world.set_block_and_metadata_with_notify(x, y - 1, z, id, var6 + 8);
            }
        } else if var6 >= 0 && (var6 == 0 || block_blocks_flow(world, x, y - 1, z)) {
            get_optimal_flow_directions(world, id, x, y, z);
            var10 = var6 + var7;
            if var6 >= 8 {
                var10 = 1;
            }
            if var10 >= 8 {
                return;
            }
            // src: BlockFlowing.java:97-111 (-x, +x, -z, +z). `var13` is the
            // singleton's `isOptimalFlowDirection`, read live after each
            // flowIntoBlock (module docs).
            let targets = [(x - 1, z), (x + 1, z), (x, z - 1), (x, z + 1)];
            for (k, (tx, tz)) in targets.into_iter().enumerate() {
                if scratch(world, id).is_optimal_flow_direction[k] {
                    flow_into_block(world, id, tx, y, tz, var10);
                }
            }
        }
    }

    /// `BlockFlowing.flowIntoBlock(world, x, y, z, level)` (private): lava
    /// fizzes over what it replaces, water drops it as items; then this
    /// fluid at `level`, with notification.
    // src: BlockFlowing.java:116-130
    fn flow_into_block(world: &mut World, id: i32, x: i32, y: i32, z: i32, var5: i32) {
        if liquid_can_displace_block(world, id, x, y, z) {
            let var6 = world.get_block_id(x, y, z);
            if var6 > 0 {
                if material(id) == Material::Lava {
                    trigger_lava_mix_effects(world, x, y, z);
                } else {
                    let meta = world.get_block_metadata(x, y, z);
                    dispatch::drop_block_as_item(world, var6, x, y, z, meta);
                }
            }
            world.set_block_and_metadata_with_notify(x, y, z, id, var5);
        }
    }

    /// `BlockFlowing.calculateFlowCost(world, x, y, z, depth, fromDir)`
    /// (private): the fewest steps (from `depth`, at most 4 deep) to a cell
    /// the fluid could fall from, 1000 if none; never steps back the way it
    /// came (`fromDir`'s opposite).
    // src: BlockFlowing.java:132-171
    fn calculate_flow_cost(
        world: &mut World,
        id: i32,
        x: i32,
        y: i32,
        z: i32,
        var5: i32,
        var6: i32,
    ) -> i32 {
        let mut var7 = 1000;
        for var8 in 0..4 {
            // src: BlockFlowing.java:136 (0/1 and 2/3 are opposites)
            if (var8 != 0 || var6 != 1)
                && (var8 != 1 || var6 != 0)
                && (var8 != 2 || var6 != 3)
                && (var8 != 3 || var6 != 2)
            {
                let mut var9 = x;
                let mut var11 = z;
                if var8 == 0 {
                    var9 = x - 1;
                }
                if var8 == 1 {
                    var9 += 1;
                }
                if var8 == 2 {
                    var11 = z - 1;
                }
                if var8 == 3 {
                    var11 += 1;
                }

                if !block_blocks_flow(world, var9, y, var11)
                    && (world.get_block_material(var9, y, var11) != material(id)
                        || world.get_block_metadata(var9, y, var11) != 0)
                {
                    if !block_blocks_flow(world, var9, y - 1, var11) {
                        return var5;
                    }
                    // src: BlockFlowing.java:160 (search depth)
                    if var5 < 4 {
                        let var12 = calculate_flow_cost(world, id, var9, y, var11, var5 + 1, var8);
                        if var12 < var7 {
                            var7 = var12;
                        }
                    }
                }
            }
        }
        var7
    }

    /// `BlockFlowing.getOptimalFlowDirections(world, x, y, z)` (private):
    /// fills the singleton's `flowCost` (-x, +x, -z, +z; 1000 where the
    /// fluid cannot go) and sets `isOptimalFlowDirection` to the directions
    /// of least cost. Java returns the singleton's array; the caller reads it
    /// from [`super::super::BlockStatics`].
    // src: BlockFlowing.java:173-218
    fn get_optimal_flow_directions(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
        for var5 in 0..4 {
            // src: BlockFlowing.java:177
            scratch_mut(world, id).flow_cost[var5] = 1000;
            let mut var6 = x;
            let mut var8 = z;
            if var5 == 0 {
                var6 = x - 1;
            }
            if var5 == 1 {
                var6 += 1;
            }
            if var5 == 2 {
                var8 = z - 1;
            }
            if var5 == 3 {
                var8 += 1;
            }

            if !block_blocks_flow(world, var6, y, var8)
                && (world.get_block_material(var6, y, var8) != material(id)
                    || world.get_block_metadata(var6, y, var8) != 0)
            {
                if !block_blocks_flow(world, var6, y - 1, var8) {
                    scratch_mut(world, id).flow_cost[var5] = 0;
                } else {
                    let c = calculate_flow_cost(world, id, var6, y, var8, 1, var5 as i32);
                    scratch_mut(world, id).flow_cost[var5] = c;
                }
            }
        }

        // src: BlockFlowing.java:205-215
        let s = scratch_mut(world, id);
        let mut var5 = s.flow_cost[0];
        for var6 in 1..4 {
            if s.flow_cost[var6] < var5 {
                var5 = s.flow_cost[var6];
            }
        }
        for var6 in 0..4 {
            s.is_optimal_flow_direction[var6] = s.flow_cost[var6] == var5;
        }
    }

    /// `BlockFlowing.blockBlocksFlow(world, x, y, z)` (private): doors, sign
    /// posts, ladders and reeds, and every block of a solid material.
    // src: BlockFlowing.java:220-232
    fn block_blocks_flow(world: &mut World, x: i32, y: i32, z: i32) -> bool {
        let var5 = world.get_block_id(x, y, z);
        // src: BlockFlowing.java:222; Block.doorWood 64, doorSteel 71,
        // signPost 63, ladder 65, reed 83 (Block.java:655-675)
        if var5 != 64 && var5 != 71 && var5 != 63 && var5 != 65 && var5 != 83 {
            if var5 == 0 {
                false
            } else {
                material(var5).get_is_solid()
            }
        } else {
            true
        }
    }

    /// `BlockFlowing.getSmallestFlowDecay(world, x, y, z, smallest)`
    /// (protected): the smaller of `smallest` (ignored while negative) and
    /// the effective level at `x, y, z`; counts source neighbours into the
    /// singleton's `numAdjacentSources`.
    // src: BlockFlowing.java:234-249
    fn get_smallest_flow_decay(
        world: &mut World,
        id: i32,
        x: i32,
        y: i32,
        z: i32,
        var5: i32,
    ) -> i32 {
        let mut var6 = get_flow_decay(world, id, x, y, z);
        if var6 < 0 {
            var5
        } else {
            if var6 == 0 {
                scratch_mut(world, id).num_adjacent_sources += 1;
            }
            if var6 >= 8 {
                var6 = 0;
            }
            if var5 >= 0 && var6 >= var5 {
                var5
            } else {
                var6
            }
        }
    }

    /// `BlockFlowing.liquidCanDisplaceBlock(world, x, y, z)` (private): not
    /// this fluid, not lava, and not a flow-blocking block.
    // src: BlockFlowing.java:251-260
    fn liquid_can_displace_block(world: &mut World, id: i32, x: i32, y: i32, z: i32) -> bool {
        let var5 = world.get_block_material(x, y, z);
        if var5 == material(id) || var5 == Material::Lava {
            false
        } else {
            !block_blocks_flow(world, x, y, z)
        }
    }

    /// `BlockFlowing.onBlockAdded(world, x, y, z)`: `BlockFluid`'s
    /// (`checkForHarden`), then, if the block is still this one, its first
    /// tick.
    // src: BlockFlowing.java:262-268
    pub fn on_block_added(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
        super::on_block_added(world, id, x, y, z);
        if world.get_block_id(x, y, z) == id {
            world.schedule_block_update(x, y, z, id, dispatch::tick_rate(id));
        }
    }
}

// ---- BlockStationary --------------------------------------------------------------

/// `BlockStationary` (BlockStationary.java): the still fluids,
/// `Block.waterStill` (9) and `Block.lavaStill` (11; random ticks, since its
/// constructor sets `tickOnLoad`).
pub mod stationary {
    use super::*;

    /// `BlockStationary.onNeighborBlockChange(world, x, y, z, neighborId)`:
    /// `BlockFluid`'s (`checkForHarden`), then, if the block is still this
    /// one, `setNotStationary`.
    // src: BlockStationary.java:15-21
    pub fn on_neighbor_block_change(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
        super::on_neighbor_block_change(world, id, x, y, z);
        if world.get_block_id(x, y, z) == id {
            set_not_stationary(world, id, x, y, z);
        }
    }

    /// `BlockStationary.setNotStationary(world, x, y, z)` (private): turns
    /// this block into its flowing partner (`blockID - 1`, same metadata)
    /// with neighbour notification suppressed (`editingBlocks`), and
    /// schedules its tick. `editingBlocks` ends false whatever it was before
    /// (vanilla, transcribed).
    // src: BlockStationary.java:23-30
    fn set_not_stationary(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
        let var5 = world.get_block_metadata(x, y, z);
        world.editing_blocks = true;
        world.set_block_and_metadata(x, y, z, id - 1, var5);
        world.mark_blocks_dirty(x, y, z, x, y, z);
        // src: BlockStationary.java:28 (`this.tickRate()`: the still block's,
        // equal to the flowing one's)
        world.schedule_block_update(x, y, z, id - 1, dispatch::tick_rate(id));
        world.editing_blocks = false;
    }

    /// `BlockStationary.updateTick(world, x, y, z, rand)`: still lava's
    /// random tick tries up to two steps up and sideways for an air block
    /// next to something flammable and sets it on fire. Water does nothing.
    // src: BlockStationary.java:32-52
    pub fn update_tick(
        world: &mut World,
        id: i32,
        mut x: i32,
        mut y: i32,
        mut z: i32,
        rand: RandomSource,
    ) {
        if material(id) == Material::Lava {
            let var6 = world.random(rand).next_int_bounded(3);
            for _var7 in 0..var6 {
                // src: BlockStationary.java:37-39 (x draw, then z draw)
                x += world.random(rand).next_int_bounded(3) - 1;
                y += 1;
                z += world.random(rand).next_int_bounded(3) - 1;
                let var8 = world.get_block_id(x, y, z);
                if var8 == 0 {
                    // src: BlockStationary.java:42 (-x, +x, -z, +z, -y, +y,
                    // short-circuit)
                    if is_flammable(world, x - 1, y, z)
                        || is_flammable(world, x + 1, y, z)
                        || is_flammable(world, x, y, z - 1)
                        || is_flammable(world, x, y, z + 1)
                        || is_flammable(world, x, y - 1, z)
                        || is_flammable(world, x, y + 1, z)
                    {
                        world.set_block_with_notify(x, y, z, FIRE_ID);
                        return;
                    }
                } else if block(var8).is_some_and(|b| b.block_material.get_is_solid()) {
                    return;
                }
            }
        }
    }

    /// `BlockStationary.isFlammable(world, x, y, z)` (private): the material
    /// there burns.
    // src: BlockStationary.java:54-56
    fn is_flammable(world: &mut World, x: i32, y: i32, z: i32) -> bool {
        world.get_block_material(x, y, z).get_burning()
    }
}
