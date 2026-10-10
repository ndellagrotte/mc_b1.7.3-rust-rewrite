//! `BlockLeaves` (BlockLeaves.java), `BlockLeavesBase` (BlockLeavesBase.java)
//! and `BlockLog` (BlockLog.java): leaf decay, which a removed log or leaf
//! starts by marking the leaves around it.
//!
//! Fully transcribed: `BlockLeaves.updateTick` (the decay search over the
//! `adjacentTreeBlocks` scratch array of the leaves singleton,
//! [`crate::block::BlockStatics::leaves_adjacent_tree_blocks`]),
//! `removeLeaves`, `onBlockRemoval`, `harvestBlock` (shears), `onEntityWalking`,
//! `getRenderColor`, `colorMultiplier` (the biome branch's colormap lookup is
//! the client's, see [`super::grass`]); `BlockLeavesBase.shouldSideBeRendered`;
//! `BlockLog.onBlockRemoval`, `harvestBlock`. Drops, textures and
//! `isOpaqueCube` (`!graphicsLevel`) are data in [`crate::block`].
//!
//! Metadata: bits 0-1 the leaf type (oak, spruce, birch), bit 3 "check for
//! decay", set by a nearby removal and cleared when a log is found within 4
//! steps through leaves.
//!
//! Vanilla behaviour, transcribed: `adjacentTreeBlocks` is a field of the
//! singleton allocated once and never cleared; when the 11×11×11 area is not
//! loaded (`checkChunksExist` false) the decay decision reads whatever the
//! previous search left at its centre. The oracle runs every scenario in one
//! JVM, so there the array outlives a scenario; it only matters when that
//! test fails, which the fixtures avoid (reported).

use super::dispatch;
use super::ids;
use crate::entity::{Entity, player};
use crate::item::ItemStack;
use crate::stats::StatList;
use crate::world::World;

/// `Item.shears.shiftedIndex` (Item.java:377: `new ItemShears(103)`, 256 + 103).
const SHEARS: i32 = 256 + 103;

/// `ColorizerFoliage.getFoliageColorPine()`.
// src: ColorizerFoliage.java:17-19
pub const FOLIAGE_COLOR_PINE: i32 = 6396257;
/// `ColorizerFoliage.getFoliageColorBirch()`.
// src: ColorizerFoliage.java:21-23
pub const FOLIAGE_COLOR_BIRCH: i32 = 8431445;
/// `ColorizerFoliage.getFoliageColorBasic()`.
// src: ColorizerFoliage.java:25-27
pub const FOLIAGE_COLOR_BASIC: i32 = 4764952;

/// The base `Block.harvestBlock` body (`super` of leaves, logs and ice; the
/// dispatch's copy is private to it): the statistic, then the normal drops.
// src: Block.java:551-554
pub(super) fn base_harvest_block(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    player: &mut Entity,
    meta: i32,
) {
    // src: Block.java:552
    player::add_stat(player, StatList::get().mine_block_stat(id), 1);
    dispatch::drop_block_as_item(world, id, x, y, z, meta);
}

/// The base `Block.shouldSideBeRendered` body (`super` of the leaves, the
/// snow layer and ice; the dispatch's copy is private to it), reading the
/// singleton's live bounds.
// src: Block.java:235-251
pub(super) fn base_should_side_be_rendered(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    side: i32,
) -> bool {
    let bb = world.block_statics.bounds[(id & 255) as usize];
    (side == 0 && bb.min_y > 0.0)
        || (side == 1 && bb.max_y < 1.0)
        || (side == 2 && bb.min_z > 0.0)
        || (side == 3 && bb.max_z < 1.0)
        || (side == 4 && bb.min_x > 0.0)
        || (side == 5 && bb.max_x < 1.0)
        || !world.is_block_opaque_cube(x, y, z)
}

/// `BlockLeavesBase.shouldSideBeRendered(blockAccess, x, y, z, side)`: with
/// fast graphics, faces between two leaves are hidden.
// src: BlockLeavesBase.java:15-18
pub fn should_side_be_rendered(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    side: i32,
) -> bool {
    let var6 = world.get_block_id(x, y, z);
    if !world.block_statics.graphics_level && var6 == id {
        false
    } else {
        base_should_side_be_rendered(world, id, x, y, z, side)
    }
}

/// `getRenderColor(meta)`: the item tint by leaf type.
// src: BlockLeaves.java:15-21
pub fn get_render_color(meta: i32) -> i32 {
    if (meta & 1) == 1 {
        FOLIAGE_COLOR_PINE
    } else if (meta & 2) == 2 {
        FOLIAGE_COLOR_BIRCH
    } else {
        FOLIAGE_COLOR_BASIC
    }
}

/// `colorMultiplier(blockAccess, x, y, z)`: spruce and birch have fixed
/// tints; oak takes the biome's foliage colour (`ColorizerFoliage`, a
/// client colormap like the grass one: [`super::grass`] module docs).
// src: BlockLeaves.java:23-35
pub fn color_multiplier(world: &mut World, x: i32, y: i32, z: i32) -> i32 {
    let var5 = world.get_block_metadata(x, y, z);
    if (var5 & 1) == 1 {
        FOLIAGE_COLOR_PINE
    } else if (var5 & 2) == 2 {
        FOLIAGE_COLOR_BIRCH
    } else {
        let m = world.get_world_chunk_manager();
        m.get_biome_gen_base_at(x, z, 1, 1);
        let var6 = m.temperature[0];
        let var8 = m.humidity[0];
        let index = super::grass::colormap_index(var6, var8);
        // The headless colormap, as BlockGrass (super::grass module docs).
        super::grass::colormap_lookup(index)
    }
}

/// `BlockLeaves.onBlockRemoval(world, x, y, z)`: marks every leaf block in
/// the 3×3×3 around for a decay check (when the 5×5×5 area is loaded).
// src: BlockLeaves.java:37-54
pub fn on_block_removal(world: &mut World, x: i32, y: i32, z: i32) {
    let var5 = 1;
    let var6 = var5 + 1;
    if world.check_chunks_exist(x - var6, y - var6, z - var6, x + var6, y + var6, z + var6) {
        for var7 in -var5..=var5 {
            for var8 in -var5..=var5 {
                for var9 in -var5..=var5 {
                    let var10 = world.get_block_id(x + var7, y + var8, z + var9);
                    if var10 == ids::LEAVES {
                        let var11 = world.get_block_metadata(x + var7, y + var8, z + var9);
                        world.set_block_metadata(x + var7, y + var8, z + var9, var11 | 8);
                    }
                }
            }
        }
    }
}

/// `BlockLeaves.updateTick(world, x, y, z, rand)`: a leaf marked for a
/// check (bit 3) searches up to 4 steps through leaves for a log; found:
/// the mark is cleared, not found: the leaf decays (`removeLeaves`).
/// `rand` is unused.
// src: BlockLeaves.java:56-148
pub fn update_tick(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    if world.multiplayer_world {
        return;
    }
    let var6 = world.get_block_metadata(x, y, z);
    if (var6 & 8) == 0 {
        return;
    }
    let var7: i32 = 4;
    let var8 = var7 + 1;
    let var9: i32 = 32;
    let var10 = var9 * var9;
    let var11 = var9 / 2;
    // src: BlockLeaves.java:65-67: allocated once, by the first check.
    let mut adj = world
        .block_statics
        .leaves_adjacent_tree_blocks
        .take()
        .unwrap_or_else(|| vec![0i32; (var9 * var9 * var9) as usize].into_boxed_slice());
    let idx = |a: i32, b: i32, c: i32| (a * var10 + b * var9 + c) as usize;
    if world.check_chunks_exist(x - var8, y - var8, z - var8, x + var8, y + var8, z + var8) {
        // src: BlockLeaves.java:122-136: classify the 9×9×9 cube (log 0,
        // leaves -2, anything else -1).
        for var12 in -var7..=var7 {
            for var13 in -var7..=var7 {
                for var14 in -var7..=var7 {
                    let var15 = world.get_block_id(x + var12, y + var13, z + var14);
                    adj[idx(var12 + var11, var13 + var11, var14 + var11)] = if var15 == ids::WOOD {
                        0
                    } else if var15 == ids::LEAVES {
                        -2
                    } else {
                        -1
                    };
                }
            }
        }
        // src: BlockLeaves.java:79-119: four rounds; a leaf (-2) next to a
        // cell of distance `var12 - 1` gets distance `var12`. Each round
        // updates in place, x outer, y, z inner, neighbours -x, +x, -y, +y,
        // -z, +z.
        for var12 in 1..=4 {
            for var13 in -var7..=var7 {
                for var14 in -var7..=var7 {
                    for var15 in -var7..=var7 {
                        let (a, b, c) = (var13 + var11, var14 + var11, var15 + var11);
                        if adj[idx(a, b, c)] == var12 - 1 {
                            for (na, nb, nc) in [
                                (a - 1, b, c),
                                (a + 1, b, c),
                                (a, b - 1, c),
                                (a, b + 1, c),
                                (a, b, c - 1),
                                (a, b, c + 1),
                            ] {
                                if adj[idx(na, nb, nc)] == -2 {
                                    adj[idx(na, nb, nc)] = var12;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    // src: BlockLeaves.java:139-144
    let var12 = adj[idx(var11, var11, var11)];
    world.block_statics.leaves_adjacent_tree_blocks = Some(adj);
    if var12 >= 0 {
        world.set_block_metadata(x, y, z, var6 & -9);
    } else {
        remove_leaves(world, id, x, y, z);
    }
}

/// `removeLeaves(world, x, y, z)` (private): drop and remove.
// src: BlockLeaves.java:150-153
fn remove_leaves(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    let meta = world.get_block_metadata(x, y, z);
    dispatch::drop_block_as_item(world, id, x, y, z, meta);
    world.set_block_with_notify(x, y, z, 0);
}

/// `BlockLeaves.harvestBlock(world, player, x, y, z, meta)`: with shears in
/// hand the leaf block itself drops (type only); otherwise `super`.
// src: BlockLeaves.java:163-171
pub fn harvest_block(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    player: &mut Entity,
    meta: i32,
) {
    let shears = player
        .kind
        .player()
        .and_then(|p| p.inventory.get_current_item())
        .is_some_and(|s| s.item_id == SHEARS);
    if !world.multiplayer_world && shears {
        // src: BlockLeaves.java:165 (leaves have no mine stat: disableStats, Block.java:610)
        player::add_stat(player, StatList::get().mine_block_stat(id), 1);
        dispatch::drop_block_as_item_do(
            world,
            id,
            x,
            y,
            z,
            ItemStack::new(ids::LEAVES, 1, meta & 3),
        );
    } else {
        base_harvest_block(world, id, x, y, z, player, meta);
    }
}

/// `BlockLeaves.onEntityWalking`: only `super` (nothing).
// src: BlockLeaves.java:190-192
pub fn on_entity_walking() {}

/// `BlockLog.onBlockRemoval(world, x, y, z)`: marks every unmarked leaf block
/// in the 9×9×9 around for a decay check (when the 11×11×11 area is loaded).
// src: BlockLog.java:23-42
pub fn log_on_block_removal(world: &mut World, x: i32, y: i32, z: i32) {
    let var5 = 4;
    let var6 = var5 + 1;
    if world.check_chunks_exist(x - var6, y - var6, z - var6, x + var6, y + var6, z + var6) {
        for var7 in -var5..=var5 {
            for var8 in -var5..=var5 {
                for var9 in -var5..=var5 {
                    let var10 = world.get_block_id(x + var7, y + var8, z + var9);
                    if var10 == ids::LEAVES {
                        let var11 = world.get_block_metadata(x + var7, y + var8, z + var9);
                        if (var11 & 8) == 0 {
                            world.set_block_metadata(x + var7, y + var8, z + var9, var11 | 8);
                        }
                    }
                }
            }
        }
    }
}

/// `BlockLog.harvestBlock`: only `super`.
// src: BlockLog.java:19-21
pub fn log_harvest_block(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    player: &mut Entity,
    meta: i32,
) {
    base_harvest_block(world, id, x, y, z, player, meta);
}
