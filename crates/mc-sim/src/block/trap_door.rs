//! `BlockTrapDoor` (BlockTrapDoor.java): the trapdoor (id 96).
//!
//! Metadata: bits 0-1 the side it hangs on (0: the block at z + 1, 1: z - 1,
//! 2: x + 1, 3: x - 1), bit 2 open.
//!
//! Transcribed: `getSelectedBoundingBoxFromPool`,
//! `getCollisionBoundingBoxFromPool` and `collisionRayTrace` (bounds from the
//! metadata, then `Block`'s body; the dispatch arms call
//! [`set_block_bounds_based_on_state`] and the base helper),
//! `setBlockBoundsBasedOnState`, `setBlockBoundsForItemRender`,
//! `setBlockBoundsForBlockRender`, `onBlockClicked`, `blockActivated`,
//! `onPoweredBlockChange`, `onNeighborBlockChange`, `onBlockPlaced`,
//! `canPlaceBlockOnSide`, `isTrapdoorOpen`. Data methods are in
//! [`crate::block`].

use super::{BlockBounds, Material, dispatch};
use crate::entity::Entity;
use crate::world::World;

/// `isTrapdoorOpen(meta)` (static).
// src: BlockTrapDoor.java:180-182
pub const fn is_trapdoor_open(meta: i32) -> bool {
    (meta & 4) != 0
}

/// Writes the singleton's bounds (`setBlockBounds`).
fn set_block_bounds(world: &mut World, id: i32, b: BlockBounds) {
    world.block_statics.bounds[(id & 255) as usize] = b;
}

/// `setBlockBoundsBasedOnState(world, x, y, z)`.
// src: BlockTrapDoor.java:38-40
pub fn set_block_bounds_based_on_state(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    let meta = world.get_block_metadata(x, y, z);
    set_block_bounds_for_block_render(world, id, meta);
}

/// `setBlockBoundsForItemRender()`: a 3/16 slab centred in the cell.
// src: BlockTrapDoor.java:42-45
pub fn set_block_bounds_for_item_render(world: &mut World, id: i32) {
    let var1 = 0.1875f32;
    set_block_bounds(
        world,
        id,
        BlockBounds::from_floats(0.0, 0.5 - var1 / 2.0, 0.0, 1.0, 0.5 + var1 / 2.0, 1.0),
    );
}

/// `setBlockBoundsForBlockRender(meta)`: closed, a 3/16 slab on the floor
/// of the cell; open, against the side it hangs on.
// src: BlockTrapDoor.java:47-68
pub fn set_block_bounds_for_block_render(world: &mut World, id: i32, var1: i32) {
    let var2 = 0.1875f32;
    set_block_bounds(
        world,
        id,
        BlockBounds::from_floats(0.0, 0.0, 0.0, 1.0, var2, 1.0),
    );
    if is_trapdoor_open(var1) {
        if (var1 & 3) == 0 {
            set_block_bounds(
                world,
                id,
                BlockBounds::from_floats(0.0, 0.0, 1.0 - var2, 1.0, 1.0, 1.0),
            );
        }
        if (var1 & 3) == 1 {
            set_block_bounds(
                world,
                id,
                BlockBounds::from_floats(0.0, 0.0, 0.0, 1.0, 1.0, var2),
            );
        }
        if (var1 & 3) == 2 {
            set_block_bounds(
                world,
                id,
                BlockBounds::from_floats(1.0 - var2, 0.0, 0.0, 1.0, 1.0, 1.0),
            );
        }
        if (var1 & 3) == 3 {
            set_block_bounds(
                world,
                id,
                BlockBounds::from_floats(0.0, 0.0, 0.0, var2, 1.0, 1.0),
            );
        }
    }
}

/// `onBlockClicked(world, x, y, z, player)`: a left click toggles like a
/// right click.
// src: BlockTrapDoor.java:70-72
pub fn on_block_clicked(world: &mut World, id: i32, x: i32, y: i32, z: i32, player: &mut Entity) {
    block_activated(world, id, x, y, z, player);
}

/// `blockActivated(world, x, y, z, player)`: flips bit 2 and plays sound
/// 1003, unless the trapdoor is of iron (none is in b1.7.3).
// src: BlockTrapDoor.java:74-83
pub fn block_activated(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    player: &mut Entity,
) -> bool {
    // src: BlockTrapDoor.java:75 (blockMaterial == Material.iron)
    if super::material_of(id) == Material::Iron {
        true
    } else {
        let var6 = world.get_block_metadata(x, y, z);
        world.set_block_metadata_with_notify(x, y, z, var6 ^ 4);
        world.play_aux_sfx_at_entity(Some(player.id()), 1003, x, y, z, 0);
        true
    }
}

/// `onPoweredBlockChange(world, x, y, z, powered)`: opens or closes to
/// match `powered`.
// src: BlockTrapDoor.java:85-92
pub fn on_powered_block_change(world: &mut World, x: i32, y: i32, z: i32, var5: bool) {
    let var6 = world.get_block_metadata(x, y, z);
    let var7 = (var6 & 4) > 0;
    if var7 != var5 {
        world.set_block_metadata_with_notify(x, y, z, var6 ^ 4);
        world.play_aux_sfx_at_entity(None, 1003, x, y, z, 0);
    }
}

/// `onNeighborBlockChange(world, x, y, z, neighborId)`: drops the trapdoor
/// when the block it hangs on is no longer a normal cube; a power source's
/// change opens or closes it.
// src: BlockTrapDoor.java:94-126
pub fn on_neighbor_block_change(world: &mut World, id: i32, x: i32, y: i32, z: i32, var5: i32) {
    if !world.multiplayer_world {
        let var6 = world.get_block_metadata(x, y, z);
        let mut var7 = x;
        let mut var8 = z;
        if (var6 & 3) == 0 {
            var8 = z + 1;
        }
        if (var6 & 3) == 1 {
            var8 -= 1;
        }
        if (var6 & 3) == 2 {
            var7 = x + 1;
        }
        if (var6 & 3) == 3 {
            var7 -= 1;
        }
        if !world.is_block_normal_cube(var7, y, var8) {
            world.set_block_with_notify(x, y, z, 0);
            dispatch::drop_block_as_item(world, id, x, y, z, var6);
        }
        // vanilla bug: no return after the removal above, so a power source's
        // change still runs onPoweredBlockChange on the now empty cell (it
        // reads metadata 0 and may write metadata 4 into the air block).
        // src: BlockTrapDoor.java:120 (Block.blocksList[var5].canProvidePower())
        if var5 > 0 && dispatch::can_provide_power(world, var5) {
            let var9 = world.is_block_indirectly_getting_powered(x, y, z);
            on_powered_block_change(world, x, y, z, var9);
        }
    }
}

/// `onBlockPlaced(world, x, y, z, side)`: the metadata from the face it was
/// placed against (2 → 0, 3 → 1, 4 → 2, 5 → 3; anything else 0).
// src: BlockTrapDoor.java:133-152
pub fn on_block_placed(world: &mut World, x: i32, y: i32, z: i32, var5: i32) {
    let mut var6 = 0;
    if var5 == 2 {
        var6 = 0;
    }
    if var5 == 3 {
        var6 = 1;
    }
    if var5 == 4 {
        var6 = 2;
    }
    if var5 == 5 {
        var6 = 3;
    }
    world.set_block_metadata_with_notify(x, y, z, var6);
}

/// `canPlaceBlockOnSide(world, x, y, z, side)`: only on the side of a
/// normal cube (never on a top or bottom face).
// src: BlockTrapDoor.java:154-178
pub fn can_place_block_on_side(world: &mut World, x: i32, y: i32, z: i32, var5: i32) -> bool {
    if var5 == 0 || var5 == 1 {
        return false;
    }
    let (mut x, mut z) = (x, z);
    if var5 == 2 {
        z += 1;
    }
    if var5 == 3 {
        z -= 1;
    }
    if var5 == 4 {
        x += 1;
    }
    if var5 == 5 {
        x -= 1;
    }
    world.is_block_normal_cube(x, y, z)
}
