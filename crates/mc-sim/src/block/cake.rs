//! `BlockCake` (BlockCake.java): the cake (id 92). Metadata 0..5 is the
//! number of slices eaten.
//!
//! Transcribed: `setBlockBoundsBasedOnState`, `setBlockBoundsForItemRender`,
//! `getCollisionBoundingBoxFromPool`, `getSelectedBoundingBoxFromPool`,
//! `blockActivated`, `onBlockClicked`, `eatCakeSlice`, `canPlaceBlockAt`,
//! `onNeighborBlockChange`, `canBlockStay`. Data methods (`quantityDropped`,
//! `idDropped`, the texture methods, …) are in [`crate::block`].
//!
//! `eatCakeSlice` calls `EntityLiving.heal` on the player (no subclass in
//! singleplayer overrides it), [`crate::entity::living::heal`], which also
//! restarts half the damage-resistance window (`heartsLife`): a hit soon
//! after a slice deals full damage again (WP-4.6; Phase 1 had a private copy
//! without that field).

use mc_core::Aabb;

use super::BlockBounds;
use crate::entity::Entity;
use crate::world::World;

/// `setBlockBoundsBasedOnState(world, x, y, z)`: half high, 1/16 inset, the
/// west edge moving east by 2/16 per slice eaten.
// src: BlockCake.java:11-17
pub fn set_block_bounds_based_on_state(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    let var5 = world.get_block_metadata(x, y, z);
    let var6 = 0.0625f32;
    let var7 = (1 + var5 * 2) as f32 / 16.0;
    let var8 = 0.5f32;
    world.block_statics.bounds[(id & 255) as usize] =
        BlockBounds::from_floats(var7, 0.0, var6, 1.0 - var6, var8, 1.0 - var6);
}

/// `setBlockBoundsForItemRender()`: the whole cake.
// src: BlockCake.java:19-23
pub fn set_block_bounds_for_item_render(world: &mut World, id: i32) {
    let var1 = 0.0625f32;
    let var2 = 0.5f32;
    world.block_statics.bounds[(id & 255) as usize] =
        BlockBounds::from_floats(var1, 0.0, var1, 1.0 - var1, var2, 1.0 - var1);
}

/// `getCollisionBoundingBoxFromPool(world, x, y, z)`: built directly (the
/// singleton's bounds are not used), 1/16 lower than the cake, in `float`
/// arithmetic widened to `double`.
// src: BlockCake.java:25-31
pub fn get_collision_bounding_box_from_pool(world: &mut World, x: i32, y: i32, z: i32) -> Aabb {
    let var5 = world.get_block_metadata(x, y, z);
    let var6 = 0.0625f32;
    let var7 = (1 + var5 * 2) as f32 / 16.0;
    let var8 = 0.5f32;
    Aabb::new(
        (x as f32 + var7) as f64,
        y as f64,
        (z as f32 + var6) as f64,
        ((x + 1) as f32 - var6) as f64,
        (y as f32 + var8 - var6) as f64,
        ((z + 1) as f32 - var6) as f64,
    )
}

/// `getSelectedBoundingBoxFromPool(world, x, y, z)`: as the collision box,
/// but the full half height.
// src: BlockCake.java:33-39
pub fn get_selected_bounding_box_from_pool(world: &mut World, x: i32, y: i32, z: i32) -> Aabb {
    let var5 = world.get_block_metadata(x, y, z);
    let var6 = 0.0625f32;
    let var7 = (1 + var5 * 2) as f32 / 16.0;
    let var8 = 0.5f32;
    Aabb::new(
        (x as f32 + var7) as f64,
        y as f64,
        (z as f32 + var6) as f64,
        ((x + 1) as f32 - var6) as f64,
        (y as f32 + var8) as f64,
        ((z + 1) as f32 - var6) as f64,
    )
}

/// `blockActivated(world, x, y, z, player)`: eats a slice; always true.
// src: BlockCake.java:67-70
pub fn block_activated(world: &mut World, x: i32, y: i32, z: i32, player: &mut Entity) -> bool {
    eat_cake_slice(world, x, y, z, player);
    true
}

/// `onBlockClicked(world, x, y, z, player)`: eats a slice.
// src: BlockCake.java:72-74
pub fn on_block_clicked(world: &mut World, x: i32, y: i32, z: i32, player: &mut Entity) {
    eat_cake_slice(world, x, y, z, player);
}

/// `eatCakeSlice(world, x, y, z, player)` (private): a player below 20
/// health heals 3 and eats a slice; the sixth slice removes the cake.
// src: BlockCake.java:76-88
fn eat_cake_slice(world: &mut World, x: i32, y: i32, z: i32, var5: &mut Entity) {
    let health = var5
        .kind
        .living()
        .expect("BlockCake.eatCakeSlice: the player is an EntityLiving")
        .health;
    if health < 20 {
        // src: BlockCake.java:78 (EntityPlayer does not override heal)
        crate::entity::living::heal(var5, 3);
        let var6 = world.get_block_metadata(x, y, z) + 1;
        if var6 >= 6 {
            world.set_block_with_notify(x, y, z, 0);
        } else {
            world.set_block_metadata_with_notify(x, y, z, var6);
            world.mark_block_as_needs_update(x, y, z);
        }
    }
}

/// `canPlaceBlockAt(world, x, y, z)`: `Block`'s test, then `canBlockStay`.
// src: BlockCake.java:90-92
pub fn can_place_block_at(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    if !super::dispatch::base_can_place_block_at(world, x, y, z) {
        false
    } else {
        can_block_stay(world, x, y, z)
    }
}

/// `onNeighborBlockChange(world, x, y, z, neighborId)`: without a solid
/// block below, drops (nothing: `quantityDropped` is 0) and vanishes.
// src: BlockCake.java:94-100
pub fn on_neighbor_block_change(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    if !can_block_stay(world, x, y, z) {
        let meta = world.get_block_metadata(x, y, z);
        super::dispatch::drop_block_as_item(world, id, x, y, z, meta);
        world.set_block_with_notify(x, y, z, 0);
    }
}

/// `canBlockStay(world, x, y, z)`: a solid material below.
// src: BlockCake.java:102-104
pub fn can_block_stay(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    world.get_block_material(x, y - 1, z).is_solid()
}
