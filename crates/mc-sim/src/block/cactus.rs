//! `BlockCactus` (BlockCactus.java) and `BlockReed` (BlockReed.java, sugar
//! cane): plants that grow upward to three blocks, one step per random tick
//! through metadata 0..15.
//!
//! Fully transcribed: `BlockCactus.updateTick`,
//! `getCollisionBoundingBoxFromPool`, `getSelectedBoundingBoxFromPool`,
//! `canPlaceBlockAt`, `onNeighborBlockChange`, `canBlockStay`,
//! `onEntityCollidedWithBlock`; `BlockReed.updateTick`, `canPlaceBlockAt`,
//! `onNeighborBlockChange`, `checkBlockCoordValid`, `canBlockStay`,
//! `getCollisionBoundingBoxFromPool` (none). Textures, render types and
//! drops are data in [`crate::block`].
//!
//! Vanilla behaviour, transcribed: neither `updateTick` checks that the
//! plant may stay; only a neighbour change does.

use mc_core::Aabb;

use super::{Material, dispatch, ids};
use crate::entity::Entity;
use crate::world::World;

/// The shared growth step of `BlockCactus.updateTick` and
/// `BlockReed.updateTick` (the two bodies are identical): with air above and
/// fewer than three of this block in the column, count metadata up; at 15
/// place a new block on top and restart at 0.
// src: BlockCactus.java:11-28, BlockReed.java:14-31
pub fn update_tick(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    if world.is_air_block(x, y + 1, z) {
        let mut var6 = 1;
        while world.get_block_id(x, y - var6, z) == id {
            var6 += 1;
        }
        if var6 < 3 {
            let var7 = world.get_block_metadata(x, y, z);
            if var7 == 15 {
                world.set_block_with_notify(x, y + 1, z, id);
                world.set_block_metadata_with_notify(x, y, z, 0);
            } else {
                world.set_block_metadata_with_notify(x, y, z, var7 + 1);
            }
        }
    }
}

/// `BlockCactus.getCollisionBoundingBoxFromPool(world, x, y, z)`: inset by
/// 1/16 on the sides and the top (float sums widened).
// src: BlockCactus.java:30-33
pub fn get_collision_bounding_box_from_pool(x: i32, y: i32, z: i32) -> Aabb {
    let var5 = 0.0625f32;
    Aabb::new(
        (x as f32 + var5) as f64,
        y as f64,
        (z as f32 + var5) as f64,
        ((x + 1) as f32 - var5) as f64,
        ((y + 1) as f32 - var5) as f64,
        ((z + 1) as f32 - var5) as f64,
    )
}

/// `BlockCactus.getSelectedBoundingBoxFromPool(world, x, y, z)`: inset on the
/// sides only.
// src: BlockCactus.java:35-38
pub fn get_selected_bounding_box_from_pool(x: i32, y: i32, z: i32) -> Aabb {
    let var5 = 0.0625f32;
    Aabb::new(
        (x as f32 + var5) as f64,
        y as f64,
        (z as f32 + var5) as f64,
        ((x + 1) as f32 - var5) as f64,
        (y + 1) as f64,
        ((z + 1) as f32 - var5) as f64,
    )
}

/// `BlockCactus.canPlaceBlockAt(world, x, y, z)`: `super` (air or ground
/// cover), then `canBlockStay`.
// src: BlockCactus.java:60-62
pub fn can_place_block_at(world: &mut World, id: i32, x: i32, y: i32, z: i32) -> bool {
    if !super::dispatch::base_can_place_block_at(world, x, y, z) {
        false
    } else {
        dispatch::can_block_stay(world, id, x, y, z)
    }
}

/// `BlockCactus.onNeighborBlockChange(world, x, y, z, neighborId)`: drop and
/// remove when it can no longer stay.
// src: BlockCactus.java:64-70
pub fn on_neighbor_block_change(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    if !dispatch::can_block_stay(world, id, x, y, z) {
        let meta = world.get_block_metadata(x, y, z);
        dispatch::drop_block_as_item(world, id, x, y, z, meta);
        world.set_block_with_notify(x, y, z, 0);
    }
}

/// `BlockCactus.canBlockStay(world, x, y, z)`: no solid block on the four
/// sides (-x, +x, -z, +z), and cactus or sand below.
// src: BlockCactus.java:72-85
pub fn can_block_stay(world: &mut World, id: i32, x: i32, y: i32, z: i32) -> bool {
    if world.get_block_material(x - 1, y, z).is_solid()
        || world.get_block_material(x + 1, y, z).is_solid()
        || world.get_block_material(x, y, z - 1).is_solid()
        || world.get_block_material(x, y, z + 1).is_solid()
    {
        false
    } else {
        let var5 = world.get_block_id(x, y - 1, z);
        var5 == id || var5 == ids::SAND
    }
}

/// `BlockCactus.onEntityCollidedWithBlock(world, x, y, z, entity)`: one
/// point of damage from no attacker.
// src: BlockCactus.java:87-89
pub fn on_entity_collided_with_block(world: &mut World, e: &mut Entity) {
    crate::entity::dispatch::attack_entity_from(e, world, None, 1);
}

/// `BlockReed.canPlaceBlockAt(world, x, y, z)`: on reed, or on grass or dirt
/// with water beside that block (-x, +x, -z, +z).
// src: BlockReed.java:33-48
pub fn reed_can_place_block_at(world: &mut World, id: i32, x: i32, y: i32, z: i32) -> bool {
    let var5 = world.get_block_id(x, y - 1, z);
    if var5 == id {
        true
    } else if var5 != ids::GRASS && var5 != ids::DIRT {
        false
    } else {
        world.get_block_material(x - 1, y - 1, z) == Material::Water
            || world.get_block_material(x + 1, y - 1, z) == Material::Water
            || world.get_block_material(x, y - 1, z - 1) == Material::Water
            || world.get_block_material(x, y - 1, z + 1) == Material::Water
    }
}

/// `BlockReed.onNeighborBlockChange`: `checkBlockCoordValid`.
// src: BlockReed.java:50-52
pub fn reed_on_neighbor_block_change(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    check_block_coord_valid(world, id, x, y, z);
}

/// `checkBlockCoordValid(world, x, y, z)` (protected final): drop and remove
/// when it can no longer stay.
// src: BlockReed.java:54-60
fn check_block_coord_valid(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    if !dispatch::can_block_stay(world, id, x, y, z) {
        let meta = world.get_block_metadata(x, y, z);
        dispatch::drop_block_as_item(world, id, x, y, z, meta);
        world.set_block_with_notify(x, y, z, 0);
    }
}

/// `BlockReed.canBlockStay(world, x, y, z)`: `this.canPlaceBlockAt` (the
/// reed's own).
// src: BlockReed.java:62-64
pub fn reed_can_block_stay(world: &mut World, id: i32, x: i32, y: i32, z: i32) -> bool {
    dispatch::can_place_block_at(world, id, x, y, z)
}
