//! `BlockSand` (BlockSand.java), also the behaviour of `BlockGravel`, which
//! `extends BlockSand` and overrides only `idDropped` (BlockGravel.java:10-12,
//! a drop-table matter that falling never reaches: [`EntityFallingSand`]
//! drops its `blockID` directly).
//!
//! Fully transcribed: `onBlockAdded`, `onNeighborBlockChange`, `updateTick`,
//! `tryToFall`, `tickRate`, `canFallBelow`. The static `fallInstantly` is
//! world state ([`crate::block::BlockStatics::fall_instantly`]): the terrain
//! generators set it around population (ChunkProviderGenerate.java:311, :602)
//! and a new world starts with it false, which is how a scenario resets it.
//!
//! Also here, until `world/mod.rs` is free to take them: the two `World`
//! methods `EntityFallingSand` needs, [`World::can_block_be_placed_at`] and
//! [`World::check_if_aabb_is_clear`].
//!
//! [`EntityFallingSand`]: crate::entity::falling_sand

use mc_core::Aabb;

use super::{Material, block, dispatch};
use crate::entity::falling_sand;
use crate::world::World;

/// `Block.fire.blockID`.
// src: Block.java:643
const FIRE_ID: i32 = 51;

/// `BlockSand.onBlockAdded(world, x, y, z)`: schedules the first fall check.
// src: BlockSand.java:12-14
pub fn on_block_added(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    world.schedule_block_update(x, y, z, id, tick_rate());
}

/// `BlockSand.onNeighborBlockChange(world, x, y, z, neighborId)`.
// src: BlockSand.java:16-18
pub fn on_neighbor_block_change(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    world.schedule_block_update(x, y, z, id, tick_rate());
}

/// `BlockSand.updateTick(world, x, y, z, rand)`: `rand` is unused.
// src: BlockSand.java:20-22
pub fn update_tick(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    try_to_fall(world, id, x, y, z);
}

/// `BlockSand.tryToFall(world, x, y, z)` (private): spawns an
/// `EntityFallingSand`, or, with `fallInstantly` or missing chunks, moves the
/// block straight down at once.
// src: BlockSand.java:24-43
fn try_to_fall(world: &mut World, id: i32, x: i32, mut y: i32, z: i32) {
    // src: BlockSand.java:25 (the `y >= 0` test runs after canFallBelow)
    if can_fall_below(world, x, y - 1, z) && y >= 0 {
        let var8 = 32;
        // src: BlockSand.java:27
        if !world.block_statics.fall_instantly
            && world.check_chunks_exist(x - var8, y - var8, z - var8, x + var8, y + var8, z + var8)
        {
            // src: BlockSand.java:28: `(double)((float)x + 0.5F)`, a float sum
            let var9 = falling_sand::new_entity_falling_sand(
                world,
                (x as f32 + 0.5) as f64,
                (y as f32 + 0.5) as f64,
                (z as f32 + 0.5) as f64,
                id,
            );
            // src: BlockSand.java:29 (`entityJoinedWorld`)
            world.spawn_entity_in_world(var9);
        } else {
            // src: BlockSand.java:31
            world.set_block_with_notify(x, y, z, 0);
            // src: BlockSand.java:33-35
            while can_fall_below(world, x, y - 1, z) && y > 0 {
                y -= 1;
            }
            // src: BlockSand.java:37-39
            if y > 0 {
                world.set_block_with_notify(x, y, z, id);
            }
        }
    }
}

/// `BlockSand.tickRate()`: 3 (also [`super::tick_rate`]'s table entry).
// src: BlockSand.java:45-47
pub const fn tick_rate() -> i32 {
    3
}

/// `BlockSand.canFallBelow(world, x, y, z)` (public static): air, fire, or a
/// block of water or lava material. Loads the chunk (`getBlockId`).
// src: BlockSand.java:49-63
pub fn can_fall_below(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    let var4 = world.get_block_id(x, y, z);
    // src: BlockSand.java:51-55 (air, then `Block.fire.blockID`)
    if var4 == 0 || var4 == FIRE_ID {
        return true;
    }
    // src: BlockSand.java:56-60; Material.water / Material.lava
    // (Material.java:116-117) are the `Material::Water` / `Lava` singletons.
    let var5 = super::material_of(var4);
    var5 == Material::Water || var5 == Material::Lava
}

impl World {
    /// `canBlockBePlacedAt(blockId, x, y, z, ignoreCollision, side)`: the
    /// block's collision box (unless `ignoreCollision`) is free of
    /// spawn-preventing entities, the target is air or a replaceable block
    /// (water, lava, fire, snow layer), and the block's
    /// `canPlaceBlockOnSide` agrees.
    ///
    /// The collision box is computed even when `ignoreCollision` discards it,
    /// as in Java (it can record an untranscribed override).
    // src: World.java:2096-2114
    pub fn can_block_be_placed_at(
        &mut self,
        block_id: i32,
        x: i32,
        y: i32,
        z: i32,
        ignore_collision: bool,
        side: i32,
    ) -> bool {
        let var7 = self.get_block_id(x, y, z);
        let mut var8 = block(var7);
        // `Block.blocksList[var1]` must exist in Java (a null would throw);
        // an unknown id here is answered false.
        if block(block_id).is_none() {
            return false;
        }
        let mut var10 = dispatch::get_collision_bounding_box_from_pool(self, block_id, x, y, z);
        if ignore_collision {
            var10 = None;
        }
        if let Some(bb) = var10
            && !self.check_if_aabb_is_clear(&bb)
        {
            return false;
        }
        // src: World.java:2108-2110 (waterMoving 8, waterStill 9, lavaMoving
        // 10, lavaStill 11, fire 51, snow 78)
        if matches!(var7, 8..=11 | FIRE_ID | 78) {
            var8 = None;
        }
        block_id > 0
            && var8.is_none()
            && dispatch::can_place_block_on_side(self, block_id, x, y, z, side)
    }

    /// `checkIfAABBIsClear(box)`: no live entity with `preventEntitySpawning`
    /// intersects `box`. Entities taken out for an update are not seen
    /// ([`crate::entity`] module docs).
    // src: World.java:1357-1368
    pub fn check_if_aabb_is_clear(&self, bb: &Aabb) -> bool {
        let var2 = self.get_entities_within_aabb_excluding_entity(None, bb);
        for id in var2 {
            if let Some(var4) = self.entity(id)
                && !var4.base.is_dead
                && var4.base.prevent_entity_spawning
            {
                return false;
            }
        }
        true
    }
}
