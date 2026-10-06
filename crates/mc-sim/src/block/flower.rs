//! `BlockFlower` (BlockFlower.java) and the subclasses whose overrides are
//! small: `BlockMushroom` (BlockMushroom.java), `BlockTallGrass`
//! (BlockTallGrass.java), `BlockDeadBush` (BlockDeadBush.java). `BlockCrops`
//! and `BlockSapling` extend `BlockFlower` too and have their own modules
//! ([`super::crops`], [`super::sapling`]); they reach the shared parts here.
//!
//! Fully transcribed: `canPlaceBlockAt`, `canThisPlantGrowOnThisBlockID`
//! (every override, by kind: [`can_this_plant_grow_on_this_block_id`]),
//! `onNeighborBlockChange`, `updateTick`, `checkFlowerChange`,
//! `canBlockStay`, `getCollisionBoundingBoxFromPool` (none);
//! `BlockMushroom.updateTick` / `canBlockStay`; `BlockTallGrass.colorMultiplier`
//! except the colormap lookup (client, see [`super::grass`]). The data
//! overrides (textures, `idDropped`, render type) are in [`crate::block`].
//!
//! Java's virtual calls inside these bodies (`this.canBlockStay`,
//! `this.dropBlockAsItem`) go through [`super::dispatch`], so a subclass's
//! override is the one that runs (`BlockMushroom.canBlockStay`,
//! `BlockCrops.dropBlockAsItemWithChance`).

use super::dispatch;
use super::{BlockKind, ids};
use crate::world::World;

/// The base `Block.canPlaceBlockAt`: the target is air or ground cover
/// (`super.canPlaceBlockAt` of `BlockFlower` and `BlockCactus`; the
/// dispatch's base arm is not reachable from here).
// src: Block.java:501-504
pub(super) fn base_can_place_block_at(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    let var5 = world.get_block_id(x, y, z);
    var5 == 0 || super::material_of(var5).get_is_ground_cover()
}

/// `canThisPlantGrowOnThisBlockID(id)` (protected) for each `BlockFlower`
/// subclass: grass, dirt or farmland for `BlockFlower` itself (and the
/// sapling and tall grass, which inherit it); farmland for crops; sand for
/// the dead bush; any `opaqueCubeLookup` block for mushrooms.
// src: BlockFlower.java:18-20, BlockCrops.java:14-16, BlockDeadBush.java:12-14,
// BlockMushroom.java:29-31
pub fn can_this_plant_grow_on_this_block_id(kind: BlockKind, var1: i32) -> bool {
    match kind {
        BlockKind::Crops => var1 == ids::TILLED_FIELD,
        BlockKind::DeadBush => var1 == ids::SAND,
        BlockKind::Mushroom => super::opaque_cube_lookup(var1),
        _ => var1 == ids::GRASS || var1 == ids::DIRT || var1 == ids::TILLED_FIELD,
    }
}

/// The kind of block `id` (a `BlockFlower` subclass).
fn kind_of(id: i32) -> BlockKind {
    super::block(id).map_or(BlockKind::Flower, |b| b.kind)
}

/// `canPlaceBlockAt(world, x, y, z)`: `super` (air or ground cover) and a
/// soil this plant grows on below.
// src: BlockFlower.java:14-16
pub fn can_place_block_at(world: &mut World, id: i32, x: i32, y: i32, z: i32) -> bool {
    base_can_place_block_at(world, x, y, z)
        && can_this_plant_grow_on_this_block_id(kind_of(id), world.get_block_id(x, y - 1, z))
}

/// `onNeighborBlockChange(world, x, y, z, neighborId)`: `super` (nothing),
/// then `checkFlowerChange`.
// src: BlockFlower.java:22-25
pub fn on_neighbor_block_change(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    check_flower_change(world, id, x, y, z);
}

/// `BlockFlower.updateTick(world, x, y, z, rand)`: `checkFlowerChange`
/// (`rand` unused). Also the `super.updateTick` of crops and saplings.
// src: BlockFlower.java:27-29
pub fn update_tick(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    check_flower_change(world, id, x, y, z);
}

/// `checkFlowerChange(world, x, y, z)` (protected final): a plant that can no
/// longer stay drops as an item and is removed.
// src: BlockFlower.java:31-37
pub fn check_flower_change(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    if !dispatch::can_block_stay(world, id, x, y, z) {
        let meta = world.get_block_metadata(x, y, z);
        dispatch::drop_block_as_item(world, id, x, y, z, meta);
        world.set_block_with_notify(x, y, z, 0);
    }
}

/// `BlockFlower.canBlockStay(world, x, y, z)`: light 8 or open sky, and the
/// right soil below.
// src: BlockFlower.java:39-41
pub fn can_block_stay(world: &mut World, id: i32, x: i32, y: i32, z: i32) -> bool {
    (world.get_full_block_light_value(x, y, z) >= 8 || world.can_block_see_the_sky(x, y, z))
        && can_this_plant_grow_on_this_block_id(kind_of(id), world.get_block_id(x, y - 1, z))
}

/// `getCollisionBoundingBoxFromPool`: plants have none.
// src: BlockFlower.java:43-45
pub fn get_collision_bounding_box_from_pool() -> Option<mc_core::Aabb> {
    None
}

/// `BlockMushroom.updateTick(world, x, y, z, rand)`: one in 100, try to
/// spread to a random neighbour position that is air and where a mushroom
/// can stay.
// src: BlockMushroom.java:13-27
pub fn mushroom_update_tick(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    rand: dispatch::RandomSource,
) {
    if world.random(rand).next_int_bounded(100) == 0 {
        let var6 = x + world.random(rand).next_int_bounded(3) - 1;
        let var7 =
            y + world.random(rand).next_int_bounded(2) - world.random(rand).next_int_bounded(2);
        let var8 = z + world.random(rand).next_int_bounded(3) - 1;
        if world.is_air_block(var6, var7, var8)
            && dispatch::can_block_stay(world, id, var6, var7, var8)
        {
            // src: BlockMushroom.java:19-20: two draws whose results are
            // discarded (`var10000`), then the same test again.
            let _ = x + (world.random(rand).next_int_bounded(3) - 1);
            let _ = z + (world.random(rand).next_int_bounded(3) - 1);
            if world.is_air_block(var6, var7, var8)
                && dispatch::can_block_stay(world, id, var6, var7, var8)
            {
                world.set_block_with_notify(var6, var7, var8, id);
            }
        }
    }
}

/// `BlockMushroom.canBlockStay(world, x, y, z)`: inside y 0..128, full light
/// below 13 and an opaque cube below.
// src: BlockMushroom.java:33-39
pub fn mushroom_can_block_stay(world: &mut World, id: i32, x: i32, y: i32, z: i32) -> bool {
    if (0..128).contains(&y) {
        world.get_full_block_light_value(x, y, z) < 13
            && can_this_plant_grow_on_this_block_id(kind_of(id), world.get_block_id(x, y - 1, z))
    } else {
        false
    }
}

/// `BlockTallGrass.colorMultiplier(blockAccess, x, y, z)`: white for meta 0
/// (dead shrub), else the grass colour at a position jittered by a hash of
/// the coordinates (Java `long` arithmetic).
// src: BlockTallGrass.java:22-37
pub fn tall_grass_color_multiplier(world: &mut World, x: i32, y: i32, z: i32) -> i32 {
    let var5 = world.get_block_metadata(x, y, z);
    if var5 == 0 {
        return 16777215;
    }
    // (long)(x * 3129871 + z * 6129781 + y): the sum is an int expression.
    let mut var6 = x
        .wrapping_mul(3129871)
        .wrapping_add(z.wrapping_mul(6129781))
        .wrapping_add(y) as i64;
    var6 = var6
        .wrapping_mul(var6)
        .wrapping_mul(42317861)
        .wrapping_add(var6.wrapping_mul(11));
    let x = (x as i64).wrapping_add(var6 >> 14 & 31) as i32;
    let _y = (y as i64).wrapping_add(var6 >> 19 & 31) as i32;
    let z = (z as i64).wrapping_add(var6 >> 24 & 31) as i32;
    super::grass::biome_grass_color(world, x, z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soils() {
        assert!(can_this_plant_grow_on_this_block_id(BlockKind::Flower, 2));
        assert!(can_this_plant_grow_on_this_block_id(BlockKind::Sapling, 60));
        assert!(!can_this_plant_grow_on_this_block_id(BlockKind::Crops, 3));
        assert!(can_this_plant_grow_on_this_block_id(
            BlockKind::DeadBush,
            12
        ));
        assert!(can_this_plant_grow_on_this_block_id(BlockKind::Mushroom, 1));
        // opaqueCubeLookup[18] is true (the construction-time quirk).
        assert!(can_this_plant_grow_on_this_block_id(
            BlockKind::Mushroom,
            18
        ));
        assert!(!can_this_plant_grow_on_this_block_id(
            BlockKind::Mushroom,
            20
        ));
    }
}
