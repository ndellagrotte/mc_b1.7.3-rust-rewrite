//! `BlockDoor` (BlockDoor.java): the wooden door (id 64) and the iron door
//! (id 71), each two blocks high.
//!
//! Metadata: bits 0-1 the hinge rotation, bit 2 open, bit 3 the upper half
//! (the upper half mirrors the lower one's bits 0-2 plus 8).
//!
//! Transcribed: `getSelectedBoundingBoxFromPool`,
//! `getCollisionBoundingBoxFromPool` and `collisionRayTrace` (bounds from the
//! state, then `Block`'s body; the dispatch arms call
//! [`set_block_bounds_based_on_state`] and the base helper),
//! `setBlockBoundsBasedOnState`, `setDoorRotation`, `onBlockClicked`,
//! `blockActivated`, `onPoweredBlockChange`, `onNeighborBlockChange`,
//! `getState`, `canPlaceBlockAt`, `isOpen`. Data methods (`isOpaqueCube`,
//! `renderAsNormalBlock`, `getRenderType`, `idDropped`, `getMobilityFlag`,
//! `getBlockTextureFromSideAndMetadata`) are in [`crate::block`].

use super::{BlockBounds, Material, dispatch};
use crate::entity::Entity;
use crate::world::World;

/// `getState(meta)`: the rotation the bounds use: the hinge rotation when
/// open, the one before it (mod 4) when closed.
// src: BlockDoor.java:184-186 (`var1 - 1 & 3` is `(var1 - 1) & 3`)
pub const fn get_state(meta: i32) -> i32 {
    if (meta & 4) == 0 {
        (meta - 1) & 3
    } else {
        meta & 3
    }
}

/// `isOpen(meta)` (static).
// src: BlockDoor.java:196-198
pub const fn is_open(meta: i32) -> bool {
    (meta & 4) != 0
}

/// Writes the singleton's bounds (`setBlockBounds`).
fn set_block_bounds(world: &mut World, id: i32, b: BlockBounds) {
    world.block_statics.bounds[(id & 255) as usize] = b;
}

/// `setBlockBoundsBasedOnState(world, x, y, z)`.
// src: BlockDoor.java:60-62
pub fn set_block_bounds_based_on_state(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    let meta = world.get_block_metadata(x, y, z);
    set_door_rotation(world, id, get_state(meta));
}

/// `setDoorRotation(rotation)`: a 3/16 thick slab along one side of the cell.
// src: BlockDoor.java:64-83
pub fn set_door_rotation(world: &mut World, id: i32, var1: i32) {
    let var2 = 0.1875f32;
    set_block_bounds(
        world,
        id,
        BlockBounds::from_floats(0.0, 0.0, 0.0, 1.0, 2.0, 1.0),
    );
    if var1 == 0 {
        set_block_bounds(
            world,
            id,
            BlockBounds::from_floats(0.0, 0.0, 0.0, 1.0, 1.0, var2),
        );
    }
    if var1 == 1 {
        set_block_bounds(
            world,
            id,
            BlockBounds::from_floats(1.0 - var2, 0.0, 0.0, 1.0, 1.0, 1.0),
        );
    }
    if var1 == 2 {
        set_block_bounds(
            world,
            id,
            BlockBounds::from_floats(0.0, 0.0, 1.0 - var2, 1.0, 1.0, 1.0),
        );
    }
    if var1 == 3 {
        set_block_bounds(
            world,
            id,
            BlockBounds::from_floats(0.0, 0.0, 0.0, var2, 1.0, 1.0),
        );
    }
}

/// `onBlockClicked(world, x, y, z, player)`: a left click toggles like a
/// right click.
// src: BlockDoor.java:85-87
pub fn on_block_clicked(world: &mut World, id: i32, x: i32, y: i32, z: i32, player: &mut Entity) {
    block_activated(world, id, x, y, z, player);
}

/// `blockActivated(world, x, y, z, player)`: the iron door ignores the
/// click; the wooden door's upper half forwards to the lower half, which
/// flips bit 2 on both halves (upper first) and plays sound 1003.
// src: BlockDoor.java:89-111
pub fn block_activated(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    player: &mut Entity,
) -> bool {
    // src: BlockDoor.java:90 (blockMaterial == Material.iron)
    if super::material_of(id) == Material::Iron {
        return true;
    }
    let var6 = world.get_block_metadata(x, y, z);
    if (var6 & 8) != 0 {
        if world.get_block_id(x, y - 1, z) == id {
            block_activated(world, id, x, y - 1, z, player);
        }
        true
    } else {
        if world.get_block_id(x, y + 1, z) == id {
            world.set_block_metadata_with_notify(x, y + 1, z, (var6 ^ 4) + 8);
        }
        world.set_block_metadata_with_notify(x, y, z, var6 ^ 4);
        world.mark_blocks_dirty(x, y - 1, z, x, y, z);
        // src: BlockDoor.java:107 (1003: the door sound, RenderGlobal)
        world.play_aux_sfx_at_entity(Some(player.id()), 1003, x, y, z, 0);
        true
    }
}

/// `onPoweredBlockChange(world, x, y, z, powered)`: opens or closes the door
/// to match `powered`; called on the upper half, it forwards to the lower.
// src: BlockDoor.java:113-132
pub fn on_powered_block_change(world: &mut World, id: i32, x: i32, y: i32, z: i32, var5: bool) {
    let var6 = world.get_block_metadata(x, y, z);
    if (var6 & 8) != 0 {
        if world.get_block_id(x, y - 1, z) == id {
            on_powered_block_change(world, id, x, y - 1, z, var5);
        }
    } else {
        let var7 = (world.get_block_metadata(x, y, z) & 4) > 0;
        if var7 != var5 {
            if world.get_block_id(x, y + 1, z) == id {
                world.set_block_metadata_with_notify(x, y + 1, z, (var6 ^ 4) + 8);
            }
            world.set_block_metadata_with_notify(x, y, z, var6 ^ 4);
            world.mark_blocks_dirty(x, y - 1, z, x, y, z);
            world.play_aux_sfx_at_entity(None, 1003, x, y, z, 0);
        }
    }
}

/// `onNeighborBlockChange(world, x, y, z, neighborId)`: an upper half
/// without its lower half vanishes (and forwards a power source's change to
/// the lower half); a lower half without its upper half or without a normal
/// cube below vanishes (taking the upper half with it in the second case)
/// and drops the door; otherwise a power source's change opens or closes it.
// src: BlockDoor.java:134-169
pub fn on_neighbor_block_change(world: &mut World, id: i32, x: i32, y: i32, z: i32, var5: i32) {
    let var6 = world.get_block_metadata(x, y, z);
    if (var6 & 8) != 0 {
        if world.get_block_id(x, y - 1, z) != id {
            world.set_block_with_notify(x, y, z, 0);
        }
        // src: BlockDoor.java:141 (Block.blocksList[var5].canProvidePower())
        if var5 > 0 && dispatch::can_provide_power(world, var5) {
            on_neighbor_block_change(world, id, x, y - 1, z, var5);
        }
    } else {
        let mut var7 = false;
        if world.get_block_id(x, y + 1, z) != id {
            world.set_block_with_notify(x, y, z, 0);
            var7 = true;
        }
        if !world.is_block_normal_cube(x, y - 1, z) {
            world.set_block_with_notify(x, y, z, 0);
            var7 = true;
            if world.get_block_id(x, y + 1, z) == id {
                world.set_block_with_notify(x, y + 1, z, 0);
            }
        }
        if var7 {
            if !world.multiplayer_world {
                dispatch::drop_block_as_item(world, id, x, y, z, var6);
            }
        } else if var5 > 0 && dispatch::can_provide_power(world, var5) {
            let var8 = world.is_block_indirectly_getting_powered(x, y, z)
                || world.is_block_indirectly_getting_powered(x, y + 1, z);
            on_powered_block_change(world, id, x, y, z, var8);
        }
    }
}

/// `canPlaceBlockAt(world, x, y, z)`: below y 127, on a normal cube, with
/// both cells free (`Block`'s test at `y` and `y + 1`).
// src: BlockDoor.java:188-194
pub fn can_place_block_at(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    if y >= 127 {
        false
    } else {
        world.is_block_normal_cube(x, y - 1, z)
            && base_can_place_block_at(world, x, y, z)
            && base_can_place_block_at(world, x, y + 1, z)
    }
}

/// `Block.canPlaceBlockAt` (`super.canPlaceBlockAt`): air or ground cover.
// src: Block.java:501-504
fn base_can_place_block_at(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    let var5 = world.get_block_id(x, y, z);
    var5 == 0 || super::material_of(var5).get_is_ground_cover()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_follows_open_bit() {
        // closed: (meta - 1) & 3; open: meta & 3
        assert_eq!(get_state(0), 3);
        assert_eq!(get_state(1), 0);
        assert_eq!(get_state(3), 2);
        assert_eq!(get_state(4), 0);
        assert_eq!(get_state(7), 3);
        assert_eq!(get_state(8), 3); // upper half, closed, rotation 0
        assert!(is_open(4) && is_open(12) && !is_open(8));
    }
}
