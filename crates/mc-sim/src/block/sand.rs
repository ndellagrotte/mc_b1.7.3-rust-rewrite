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
//! [`EntityFallingSand`]: crate::entity::falling_sand

use super::Material;
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
