//! `BlockSnow` (the snow layer, BlockSnow.java), `BlockSnowBlock`
//! (BlockSnowBlock.java) and `BlockIce` (BlockIce.java): melting in block
//! light, the layer's support and shape, and their harvest drops.
//!
//! Fully transcribed: `BlockSnow.getCollisionBoundingBoxFromPool`,
//! `setBlockBoundsBasedOnState`, `canPlaceBlockAt`, `onNeighborBlockChange`,
//! `canSnowStay`, `harvestBlock`, `updateTick`, `shouldSideBeRendered`;
//! `BlockSnowBlock.updateTick`; `BlockIce.shouldSideBeRendered` (with its
//! `BlockBreakable` super body), `harvestBlock`, `updateTick`. Drops, opacity,
//! render pass and mobility are data in [`crate::block`].
//!
//! Where snow and ice come from (the snow roll of
//! `updateBlocksAndPlayCaveSounds`, which asks `canPlaceBlockAt` here) is
//! `crate::tick`.

use mc_core::Aabb;

use super::dispatch;
use super::ids::{WATER_MOVING, WATER_STILL};
use super::leaves::{base_harvest_block, base_should_side_be_rendered};
use crate::entity::Entity;
use crate::entity::item::new_entity_item;
use crate::item::{ItemStack, ids as items};
use crate::world::World;
use crate::world::lighting::EnumSkyBlock;

/// `BlockSnow.getCollisionBoundingBoxFromPool(world, x, y, z)`: none below 3
/// layers (`meta & 7 < 3`), else the block's current bounds in x and z and
/// from its `minY` up to half a block.
// src: BlockSnow.java:12-15
pub fn get_collision_bounding_box_from_pool(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
) -> Option<Aabb> {
    let var5 = world.get_block_metadata(x, y, z) & 7;
    if var5 >= 3 {
        let b = world.block_statics.bounds[(id & 255) as usize];
        Some(Aabb::new(
            x as f64 + b.min_x,
            y as f64 + b.min_y,
            z as f64 + b.min_z,
            x as f64 + b.max_x,
            (y as f32 + 0.5f32) as f64,
            z as f64 + b.max_z,
        ))
    } else {
        None
    }
}

/// `BlockSnow.setBlockBoundsBasedOnState(blockAccess, x, y, z)`: the
/// singleton's height becomes `2 * (1 + (meta & 7)) / 16`.
// src: BlockSnow.java:25-29
pub fn set_block_bounds_based_on_state(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    let var5 = world.get_block_metadata(x, y, z) & 7;
    let var6 = (2 * (1 + var5)) as f32 / 16.0f32;
    world.block_statics.bounds[(id & 255) as usize] =
        super::BlockBounds::from_floats(0.0, 0.0, 0.0, 1.0, var6, 1.0);
}

/// `BlockSnow.canPlaceBlockAt(world, x, y, z)`: on an opaque cube of solid
/// material (`isOpaqueCube` of the block below, so leaves count with fast
/// graphics).
// src: BlockSnow.java:31-34
pub fn can_place_block_at(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    let var5 = world.get_block_id(x, y - 1, z);
    if var5 != 0 && dispatch::is_opaque_cube(world, var5) {
        world.get_block_material(x, y - 1, z).get_is_solid()
    } else {
        false
    }
}

/// `BlockSnow.onNeighborBlockChange(world, x, y, z, neighborId)`: `canSnowStay`.
// src: BlockSnow.java:36-38
pub fn on_neighbor_block_change(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    can_snow_stay(world, id, x, y, z);
}

/// `canSnowStay(world, x, y, z)` (private): drop and remove a layer that
/// lost its support.
// src: BlockSnow.java:40-48
fn can_snow_stay(world: &mut World, id: i32, x: i32, y: i32, z: i32) -> bool {
    if !can_place_block_at(world, x, y, z) {
        let meta = world.get_block_metadata(x, y, z);
        dispatch::drop_block_as_item(world, id, x, y, z, meta);
        world.set_block_with_notify(x, y, z, 0);
        false
    } else {
        true
    }
}

/// `BlockSnow.harvestBlock(world, player, x, y, z, meta)`: one snowball at a
/// random point of the inner 0.7 of the cell (three `World.rand` draws),
/// the block removed, then the statistic. No `super`.
// src: BlockSnow.java:50-61
pub fn harvest_block(world: &mut World, x: i32, y: i32, z: i32, player: &mut Entity) {
    let _ = player;
    let var7 = items::SNOWBALL;
    let var8 = 0.7f32;
    let var9 = (world.rand.next_float() * var8) as f64 + (1.0f32 - var8) as f64 * 0.5;
    let var11 = (world.rand.next_float() * var8) as f64 + (1.0f32 - var8) as f64 * 0.5;
    let var13 = (world.rand.next_float() * var8) as f64 + (1.0f32 - var8) as f64 * 0.5;
    let mut var15 = new_entity_item(
        world,
        x as f64 + var9,
        y as f64 + var11,
        z as f64 + var13,
        ItemStack::new(var7, 1, 0),
    );
    if let Some(d) = var15.kind.item_mut() {
        d.delay_before_can_pickup = 10;
    }
    world.spawn_entity_in_world(var15);
    world.set_block_with_notify(x, y, z, 0);
    // src: BlockSnow.java:60: player.addStat(...) (statistics, WP-2.7).
    world.note_unimplemented("EntityPlayer.addStat");
}

/// `BlockSnow.updateTick` and `BlockSnowBlock.updateTick` (identical):
/// melts (drops, removed) in block light above 11.
// src: BlockSnow.java:71-77, BlockSnowBlock.java:19-25
pub fn update_tick(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    if world.get_saved_light_value(EnumSkyBlock::Block, x, y, z) > 11 {
        let meta = world.get_block_metadata(x, y, z);
        dispatch::drop_block_as_item(world, id, x, y, z, meta);
        world.set_block_with_notify(x, y, z, 0);
    }
}

/// `BlockSnow.shouldSideBeRendered(blockAccess, x, y, z, side)`: the top
/// always, else `super`.
// src: BlockSnow.java:79-81
pub fn should_side_be_rendered(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    side: i32,
) -> bool {
    if side == 1 {
        true
    } else {
        base_should_side_be_rendered(world, id, x, y, z, side)
    }
}

/// `BlockIce.shouldSideBeRendered(blockAccess, x, y, z, side)`:
/// `BlockBreakable.shouldSideBeRendered` with the side flipped as `1 - side`
/// (vanilla, transcribed: only top and bottom swap meaningfully). Ice's
/// `localFlag` is false (BlockIce.java:7), so faces against ice are hidden.
// src: BlockIce.java:16-18, BlockBreakable.java:15-18
pub fn ice_should_side_be_rendered(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    side: i32,
) -> bool {
    let var5 = 1 - side;
    let local_flag = false;
    let var6 = world.get_block_id(x, y, z);
    if !local_flag && var6 == id {
        false
    } else {
        base_should_side_be_rendered(world, id, x, y, z, var5)
    }
}

/// `BlockIce.harvestBlock(world, player, x, y, z, meta)`: `super` (no drops:
/// `quantityDropped` is 0), then flowing water where the ice stood when the
/// block below is solid or liquid.
// src: BlockIce.java:20-27
pub fn ice_harvest_block(world: &mut World, id: i32, x: i32, y: i32, z: i32, meta: i32) {
    base_harvest_block(world, id, x, y, z, meta);
    let var7 = world.get_block_material(x, y - 1, z);
    if var7.get_is_solid() || var7.get_is_liquid() {
        world.set_block_with_notify(x, y, z, WATER_MOVING);
    }
}

/// `BlockIce.updateTick(world, x, y, z, rand)`: melts into still water in
/// block light above `11 - lightOpacity[ice]` (8).
// src: BlockIce.java:33-39
pub fn ice_update_tick(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    if world.get_saved_light_value(EnumSkyBlock::Block, x, y, z) > 11 - super::light_opacity(id) {
        let meta = world.get_block_metadata(x, y, z);
        dispatch::drop_block_as_item(world, id, x, y, z, meta);
        world.set_block_with_notify(x, y, z, WATER_STILL);
    }
}
