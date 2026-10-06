//! `BlockFarmland` (BlockFarmland.java): tilled soil that stays wet near
//! water, dries out and reverts to dirt, and is trampled.
//!
//! Fully transcribed: `updateTick`, `onEntityWalking`, `isCropsNearby`,
//! `isWaterNearby`, `onNeighborBlockChange`, `getCollisionBoundingBoxFromPool`.
//! The data overrides (textures, `idDropped`, opacity 255, the 15/16-high
//! bounds) are in [`crate::block`].
//!
//! Vanilla quirk, transcribed: `isCropsNearby` has radius 0 (`byte var5 = 0`,
//! BlockFarmland.java:58), so it only looks straight up.

use mc_core::Aabb;

use super::dispatch::RandomSource;
use super::{Material, ids};
use crate::entity::Entity;
use crate::world::World;

/// `getCollisionBoundingBoxFromPool(world, x, y, z)`: a full cube, although
/// the block's bounds are 15/16 high.
// src: BlockFarmland.java:14-16
pub fn get_collision_bounding_box_from_pool(x: i32, y: i32, z: i32) -> Aabb {
    // Java: `(double)(x + 0)`, …
    Aabb::new(
        x as f64,
        y as f64,
        z as f64,
        (x + 1) as f64,
        (y + 1) as f64,
        (z + 1) as f64,
    )
}

/// `updateTick(world, x, y, z, rand)`: one in five, wet (meta 7) near water
/// or under rain; otherwise dry out one step, or become dirt when dry and
/// without crops.
// src: BlockFarmland.java:34-48
pub fn update_tick(world: &mut World, x: i32, y: i32, z: i32, rand: RandomSource) {
    if world.random(rand).next_int_bounded(5) == 0 {
        if !is_water_nearby(world, x, y, z) && !world.can_block_be_rained_on(x, y + 1, z) {
            let var6 = world.get_block_metadata(x, y, z);
            if var6 > 0 {
                world.set_block_metadata_with_notify(x, y, z, var6 - 1);
            } else if !is_crops_nearby(world, x, y, z) {
                world.set_block_with_notify(x, y, z, ids::DIRT);
            }
        } else {
            world.set_block_metadata_with_notify(x, y, z, 7);
        }
    }
}

/// `onEntityWalking(world, x, y, z, entity)`: one in four (`World.rand`),
/// trampled to dirt.
// src: BlockFarmland.java:50-55
pub fn on_entity_walking(world: &mut World, x: i32, y: i32, z: i32, e: &mut Entity) {
    let _ = e;
    if world.rand.next_int_bounded(4) == 0 {
        world.set_block_with_notify(x, y, z, ids::DIRT);
    }
}

/// `isCropsNearby(world, x, y, z)` (private): crops directly above (radius 0).
// src: BlockFarmland.java:57-69
fn is_crops_nearby(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    let var5 = 0;
    for var6 in (x - var5)..=(x + var5) {
        for var7 in (z - var5)..=(z + var5) {
            if world.get_block_id(var6, y + 1, var7) == ids::CROPS {
                return true;
            }
        }
    }
    false
}

/// `isWaterNearby(world, x, y, z)` (private): water material within 4
/// horizontally, at this level or one above (x outer, y, z inner).
// src: BlockFarmland.java:71-83
fn is_water_nearby(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    for var5 in (x - 4)..=(x + 4) {
        for var6 in y..=(y + 1) {
            for var7 in (z - 4)..=(z + 4) {
                if world.get_block_material(var5, var6, var7) == Material::Water {
                    return true;
                }
            }
        }
    }
    false
}

/// `onNeighborBlockChange(world, x, y, z, neighborId)`: `super` (nothing),
/// then a solid block on top turns the farmland to dirt.
// src: BlockFarmland.java:85-92
pub fn on_neighbor_block_change(world: &mut World, x: i32, y: i32, z: i32) {
    let var6 = world.get_block_material(x, y + 1, z);
    if var6.is_solid() {
        world.set_block_with_notify(x, y, z, ids::DIRT);
    }
}
