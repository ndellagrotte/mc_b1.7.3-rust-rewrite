//! `BlockChest` (BlockChest.java): the chest and its double-chest rules.
//!
//! b1.7.3 has no chest metadata and no neighbour bookkeeping: a double
//! chest is two `TileEntityChest`s side by side, recognised by looking at
//! the neighbours (`getBlockTexture` for the faces, `blockActivated` for the
//! `InventoryLargeChest`), and `canPlaceBlockAt` refuses a third chest next
//! to a pair. Breaking a chest scatters its items with `Block.chest`'s own
//! `Random` ([`super::BlockStatics::chest_random`]).

use super::{block, container, opaque_cube_lookup};
use crate::entity::Entity;
use crate::world::World;

/// `blockIndexInTexture` of the chest (26, BlockChest.java:10).
fn tex(id: i32) -> i32 {
    block(id).map_or(26, |b| b.block_index_in_texture)
}

/// `getBlockTexture(blockAccess, x, y, z, side)`: top and bottom 25; a
/// single chest's front (the side away from an opaque neighbour, south by
/// default) 27, its other sides 26; a double chest's halves pick the front
/// and back textures (+16 / +32, -1 for the left half) from the opaque
/// blocks around the pair.
// src: BlockChest.java:13-98
pub fn get_block_texture(world: &mut World, id: i32, x: i32, y: i32, z: i32, side: i32) -> i32 {
    let bi = tex(id);
    if side == 1 || side == 0 {
        return bi - 1;
    }
    let var6 = world.get_block_id(x, y, z - 1);
    let var7 = world.get_block_id(x, y, z + 1);
    let var8 = world.get_block_id(x - 1, y, z);
    let var9 = world.get_block_id(x + 1, y, z);
    let o = opaque_cube_lookup;
    if var6 != id && var7 != id {
        if var8 != id && var9 != id {
            // src: BlockChest.java:28-45
            let mut var14 = 3;
            if o(var6) && !o(var7) {
                var14 = 3;
            }
            if o(var7) && !o(var6) {
                var14 = 2;
            }
            if o(var8) && !o(var9) {
                var14 = 5;
            }
            if o(var9) && !o(var8) {
                var14 = 4;
            }
            if side == var14 { bi + 1 } else { bi }
        } else if side != 4 && side != 5 {
            // src: BlockChest.java:47-68
            let mut var10 = 0;
            if var8 == id {
                var10 = -1;
            }
            let ox = if var8 == id { x - 1 } else { x + 1 };
            let var11 = world.get_block_id(ox, y, z - 1);
            let var12 = world.get_block_id(ox, y, z + 1);
            if side == 3 {
                var10 = -1 - var10;
            }
            let mut var13 = 3;
            if (o(var6) || o(var11)) && !o(var7) && !o(var12) {
                var13 = 3;
            }
            if (o(var7) || o(var12)) && !o(var6) && !o(var11) {
                var13 = 2;
            }
            (if side == var13 { bi + 16 } else { bi + 32 }) + var10
        } else {
            bi
        }
    } else if side != 2 && side != 3 {
        // src: BlockChest.java:72-93
        let mut var10 = 0;
        if var6 == id {
            var10 = -1;
        }
        let oz = if var6 == id { z - 1 } else { z + 1 };
        let var11 = world.get_block_id(x - 1, y, oz);
        let var12 = world.get_block_id(x + 1, y, oz);
        if side == 4 {
            var10 = -1 - var10;
        }
        let mut var13 = 5;
        if (o(var8) || o(var11)) && !o(var9) && !o(var12) {
            var13 = 5;
        }
        if (o(var9) || o(var12)) && !o(var8) && !o(var11) {
            var13 = 4;
        }
        (if side == var13 { bi + 16 } else { bi + 32 }) + var10
    } else {
        bi
    }
}

/// `canPlaceBlockAt(world, x, y, z)`: at most one chest among the four
/// horizontal neighbours, and none of them already part of a double chest.
/// (Not `super.canPlaceBlockAt`: the target's own block is not checked
/// here; the caller, `World.canBlockBePlacedAt`, checks it first.)
// src: BlockChest.java:110-139
pub fn can_place_block_at(world: &mut World, id: i32, x: i32, y: i32, z: i32) -> bool {
    let mut var5 = 0;
    if world.get_block_id(x - 1, y, z) == id {
        var5 += 1;
    }
    if world.get_block_id(x + 1, y, z) == id {
        var5 += 1;
    }
    if world.get_block_id(x, y, z - 1) == id {
        var5 += 1;
    }
    if world.get_block_id(x, y, z + 1) == id {
        var5 += 1;
    }
    // Java's if/else-if chain of `return false`s, in its order.
    var5 <= 1
        && !is_there_a_neighbor_chest(world, id, x - 1, y, z)
        && !is_there_a_neighbor_chest(world, id, x + 1, y, z)
        && !is_there_a_neighbor_chest(world, id, x, y, z - 1)
        && !is_there_a_neighbor_chest(world, id, x, y, z + 1)
}

/// `isThereANeighborChest(world, x, y, z)` (private): a chest at `(x, y, z)`
/// with a chest beside it.
// src: BlockChest.java:141-153
fn is_there_a_neighbor_chest(world: &mut World, id: i32, x: i32, y: i32, z: i32) -> bool {
    if world.get_block_id(x, y, z) != id {
        false
    } else {
        world.get_block_id(x - 1, y, z) == id
            || world.get_block_id(x + 1, y, z) == id
            || world.get_block_id(x, y, z - 1) == id
            || world.get_block_id(x, y, z + 1) == id
    }
}

/// `onBlockRemoval(world, x, y, z)`: scatters the inventory with
/// `Block.chest`'s `Random` ([`container::scatter_inventory`]), then
/// `BlockContainer.onBlockRemoval`.
// src: BlockChest.java:155-183
pub fn on_block_removal(world: &mut World, x: i32, y: i32, z: i32) {
    container::scatter_inventory(world, x, y, z, |w| &mut w.block_statics.chest_random);
    container::on_block_removal(world, x, y, z);
}

/// `blockActivated(world, x, y, z, player)`: true. A normal cube above
/// the chest, or above the other half of a double chest, blocks opening;
/// otherwise the halves' tile entities are looked up in Java's order
/// (`InventoryLargeChest` wrapping, -x, +x, -z, +z) and, off a multiplayer
/// client, `player.displayGUIChest(inventory)`. The GUI and its
/// `ContainerChest` are the client and WP-2.5 (`EntityPlayer`'s own body is
/// empty, EntityPlayer.java:340-341; `EntityPlayerSP` opens `GuiChest`,
/// EntityPlayerSP.java:117-119): recorded as `"EntityPlayerSP.displayGUIChest"`.
/// The lookups are kept because `getBlockTileEntity` can create a missing
/// tile entity (Chunk.java:401-421).
// src: BlockChest.java:185-221
pub fn block_activated(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    player: &mut Entity,
) -> bool {
    let _ = player;
    let _var6 = world.get_block_tile_entity(x, y, z);
    if world.is_block_normal_cube(x, y + 1, z) {
        return true;
    }
    if world.get_block_id(x - 1, y, z) == id && world.is_block_normal_cube(x - 1, y + 1, z) {
        return true;
    }
    if world.get_block_id(x + 1, y, z) == id && world.is_block_normal_cube(x + 1, y + 1, z) {
        return true;
    }
    if world.get_block_id(x, y, z - 1) == id && world.is_block_normal_cube(x, y + 1, z - 1) {
        return true;
    }
    if world.get_block_id(x, y, z + 1) == id && world.is_block_normal_cube(x, y + 1, z + 1) {
        return true;
    }
    // src: BlockChest.java:198-212: `new InventoryLargeChest("Large chest", a, b)`
    // around each neighbouring half (the wrapper itself is WP-2.5 data).
    if world.get_block_id(x - 1, y, z) == id {
        world.get_block_tile_entity(x - 1, y, z);
    }
    if world.get_block_id(x + 1, y, z) == id {
        world.get_block_tile_entity(x + 1, y, z);
    }
    if world.get_block_id(x, y, z - 1) == id {
        world.get_block_tile_entity(x, y, z - 1);
    }
    if world.get_block_id(x, y, z + 1) == id {
        world.get_block_tile_entity(x, y, z + 1);
    }
    if !world.multiplayer_world {
        world.note_unimplemented("EntityPlayerSP.displayGUIChest");
    }
    true
}
