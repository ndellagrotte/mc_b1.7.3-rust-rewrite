//! `BlockPressurePlate` (BlockPressurePlate.java): the stone plate (id 70,
//! `EnumMobType.mobs`) and the wooden plate (id 72, `everything`).
//! `triggerMobType` is [`super::BlockKind::PressurePlate`]'s
//! `trigger_mob_type`. Metadata 1 = pressed.
//!
//! Fully transcribed: `getCollisionBoundingBoxFromPool` (null),
//! `canPlaceBlockAt`, `onBlockAdded` (empty), `onNeighborBlockChange`,
//! `updateTick`, `onEntityCollidedWithBlock`,
//! `setStateIfMobInteractsWithPlate`, `onBlockRemoval`,
//! `setBlockBoundsBasedOnState`, `isPoweringTo`, `isIndirectlyPoweringTo`,
//! `canProvidePower`, `setBlockBoundsForItemRender`. Data methods
//! (`tickRate` 20, `getMobilityFlag`, …) are in [`crate::block`].
//!
//! The entity queries run while the colliding entity is taken out of its
//! slot (`onEntityCollidedWithBlock` comes from inside its `moveEntity`), so
//! the plate passes it as `in_hand` (see
//! [`World::get_entities_within_aabb`]). `World` has no `in_hand` variant of
//! `getEntitiesWithinAABBExcludingEntity`, which the wooden plate calls, so
//! this module scans with a private copy of it that takes one
//! ([`get_entities_within_aabb_excluding_entity`]).

use mc_core::Aabb;
use mc_core::math::floor_double;

use super::dispatch;
use super::{BlockBounds, EnumMobType};
use crate::entity::Entity;
use crate::world::World;

/// `canPlaceBlockAt(world, x, y, z)`: a normal cube below.
// src: BlockPressurePlate.java:33-35
pub fn can_place_block_at(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    world.is_block_normal_cube(x, y - 1, z)
}

/// `onNeighborBlockChange(world, x, y, z, neighborId)`: drops the plate when
/// the block below is no longer a normal cube.
// src: BlockPressurePlate.java:40-51
pub fn on_neighbor_block_change(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    let mut var6 = false;
    if !world.is_block_normal_cube(x, y - 1, z) {
        var6 = true;
    }
    if var6 {
        let meta = world.get_block_metadata(x, y, z);
        dispatch::drop_block_as_item(world, id, x, y, z, meta);
        world.set_block_with_notify(x, y, z, 0);
    }
}

/// `updateTick(world, x, y, z, rand)`: off a multiplayer client, a pressed
/// plate checks again for entities (nothing in hand: a scheduled tick runs
/// outside every entity update).
// src: BlockPressurePlate.java:53-59
pub fn update_tick(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    trigger_mob_type: EnumMobType,
) {
    if !world.multiplayer_world && world.get_block_metadata(x, y, z) != 0 {
        set_state_if_mob_interacts_with_plate(world, id, x, y, z, trigger_mob_type, None);
    }
}

/// `onEntityCollidedWithBlock(world, x, y, z, entity)`: off a multiplayer
/// client, an unpressed plate checks for entities; `entity` is the one
/// moving (held out of its slot), passed on as `in_hand`.
// src: BlockPressurePlate.java:61-67
pub fn on_entity_collided_with_block(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    trigger_mob_type: EnumMobType,
    entity: &Entity,
) {
    if !world.multiplayer_world && world.get_block_metadata(x, y, z) != 1 {
        set_state_if_mob_interacts_with_plate(world, id, x, y, z, trigger_mob_type, Some(entity));
    }
}

/// `setStateIfMobInteractsWithPlate(world, x, y, z)` (private): presses the
/// plate when an entity of its trigger type is on it, releases it when none
/// is (each with metadata, notifications here and below, a render update and
/// a click), and while pressed schedules the next check in `tickRate()` = 20.
// src: BlockPressurePlate.java:69-110
fn set_state_if_mob_interacts_with_plate(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    trigger_mob_type: EnumMobType,
    in_hand: Option<&Entity>,
) {
    let var5 = world.get_block_metadata(x, y, z) == 1;
    let mut var6 = false;
    let var7 = 0.125f32;
    // src: BlockPressurePlate.java:75, :79, :83: the same box for each type
    let bb = Aabb::new(
        (x as f32 + var7) as f64,
        y as f64,
        (z as f32 + var7) as f64,
        ((x + 1) as f32 - var7) as f64,
        y as f64 + 0.25,
        ((z + 1) as f32 - var7) as f64,
    );
    let var8 = match trigger_mob_type {
        EnumMobType::Everything => get_entities_within_aabb_excluding_entity(world, &bb, in_hand),
        EnumMobType::Mobs => world.get_entities_within_aabb("EntityLiving", &bb, in_hand),
        EnumMobType::Players => world.get_entities_within_aabb("EntityPlayer", &bb, in_hand),
    };
    if !var8.is_empty() {
        var6 = true;
    }
    if var6 && !var5 {
        world.set_block_metadata_with_notify(x, y, z, 1);
        world.notify_blocks_of_neighbor_change(x, y, z, id);
        world.notify_blocks_of_neighbor_change(x, y - 1, z, id);
        world.mark_blocks_dirty(x, y, z, x, y, z);
        world.play_sound_effect(
            x as f64 + 0.5,
            y as f64 + 0.1,
            z as f64 + 0.5,
            "random.click",
            0.3,
            0.6,
        );
    }
    if !var6 && var5 {
        world.set_block_metadata_with_notify(x, y, z, 0);
        world.notify_blocks_of_neighbor_change(x, y, z, id);
        world.notify_blocks_of_neighbor_change(x, y - 1, z, id);
        world.mark_blocks_dirty(x, y, z, x, y, z);
        world.play_sound_effect(
            x as f64 + 0.5,
            y as f64 + 0.1,
            z as f64 + 0.5,
            "random.click",
            0.3,
            0.5,
        );
    }
    if var6 {
        world.schedule_block_update(x, y, z, id, dispatch::tick_rate(id));
    }
}

/// `World.getEntitiesWithinAABBExcludingEntity(null, box)` as the wooden
/// plate calls it, with the entity that is running its update (`in_hand`,
/// out of its slot) visible where the scan meets its id, as
/// [`World::get_entities_within_aabb`] does for the class query: the chunks
/// within 2 blocks, x outer, z inner, each chunk's sections bottom-up, in
/// list order (the same scan as
/// [`World::get_entities_within_aabb_excluding_entity`], whose signature has
/// no `in_hand`). Any other taken-out entity is recorded.
// src: World.java:2013-2029, Chunk.java:488-510
fn get_entities_within_aabb_excluding_entity(
    world: &World,
    bb: &Aabb,
    in_hand: Option<&Entity>,
) -> Vec<i32> {
    let mut out = Vec::new();
    let var3 = floor_double((bb.min_x - 2.0) / 16.0);
    let var4 = floor_double((bb.max_x + 2.0) / 16.0);
    let var5 = floor_double((bb.min_z - 2.0) / 16.0);
    let var6 = floor_double((bb.max_z + 2.0) / 16.0);
    for var7 in var3..=var4 {
        for var8 in var5..=var6 {
            let Some(c) = world.loaded_chunk(var7, var8) else {
                continue;
            };
            for section in c.entity_sections(bb.min_y, bb.max_y) {
                for &id in &c.entities[section] {
                    // `var1` (the excluded entity) is null: nothing is skipped.
                    let held = in_hand.filter(|h| h.base.entity_id == id);
                    match held.or_else(|| world.entity(id)) {
                        Some(o) => {
                            if o.base.bounding_box.intersects_with(bb) {
                                out.push(id);
                            }
                        }
                        None => {
                            if world.entity_index(id).is_none() {
                                world.note_unimplemented(
                                    "Chunk entity list holds an entity not in loadedEntityList",
                                );
                            } else {
                                world.note_unimplemented(
                                    "World.getEntitiesWithinAABBExcludingEntity over a taken-out entity",
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

/// `onBlockRemoval(world, x, y, z)`: a pressed plate notifies here and
/// below; then `super.onBlockRemoval` (empty). The metadata read is the
/// removed plate's (Chunk.java:245-251).
// src: BlockPressurePlate.java:112-120
pub fn on_block_removal(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    let var5 = world.get_block_metadata(x, y, z);
    if var5 > 0 {
        world.notify_blocks_of_neighbor_change(x, y, z, id);
        world.notify_blocks_of_neighbor_change(x, y - 1, z, id);
    }
}

/// `setBlockBoundsBasedOnState(blockAccess, x, y, z)`: 1/32 high when
/// pressed, 1/16 otherwise.
// src: BlockPressurePlate.java:122-131
pub fn set_block_bounds_based_on_state(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    let var5 = world.get_block_metadata(x, y, z) == 1;
    let var6 = 0.0625f32;
    let bounds = if var5 {
        BlockBounds::from_floats(var6, 0.0, var6, 1.0 - var6, 0.03125, 1.0 - var6)
    } else {
        BlockBounds::from_floats(var6, 0.0, var6, 1.0 - var6, 0.0625, 1.0 - var6)
    };
    world.block_statics.bounds[(id & 255) as usize] = bounds;
}

/// `isPoweringTo(blockAccess, x, y, z, side)`: pressed.
// src: BlockPressurePlate.java:133-135
pub fn is_powering_to(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    world.get_block_metadata(x, y, z) > 0
}

/// `isIndirectlyPoweringTo(world, x, y, z, side)`: pressed, into the block
/// below only (side 1).
// src: BlockPressurePlate.java:137-143
pub fn is_indirectly_powering_to(world: &mut World, x: i32, y: i32, z: i32, side: i32) -> bool {
    if world.get_block_metadata(x, y, z) == 0 {
        false
    } else {
        side == 1
    }
}

/// `canProvidePower()`: true.
// src: BlockPressurePlate.java:145-147
pub fn can_provide_power() -> bool {
    true
}

/// `setBlockBoundsForItemRender()`: the item-render box (client only).
// src: BlockPressurePlate.java:149-154
pub fn set_block_bounds_for_item_render(world: &mut World, id: i32) {
    let var1 = 0.5f32;
    let var2 = 0.125f32;
    let var3 = 0.5f32;
    world.block_statics.bounds[(id & 255) as usize] = BlockBounds::from_floats(
        0.5 - var1,
        0.5 - var2,
        0.5 - var3,
        0.5 + var1,
        0.5 + var2,
        0.5 + var3,
    );
}
