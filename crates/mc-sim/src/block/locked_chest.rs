//! `BlockLockedChest` (BlockLockedChest.java): the April-fools locked chest
//! (id 95). It can be placed anywhere and vanishes on its first random tick
//! (`tickOnLoad` is set, Block.java:687).
//!
//! Transcribed: `getBlockTexture`, `canPlaceBlockAt`, `updateTick`.
//! `getBlockTextureFromSide` is data ([`crate::block`]).

use crate::world::World;

/// `blockIndexInTexture` of the locked chest.
// src: BlockLockedChest.java:8
const BLOCK_INDEX_IN_TEXTURE: i32 = 26;

/// `getBlockTexture(blockAccess, x, y, z, side)`: lid texture on top and
/// bottom; the front faces away from the one opaque cube neighbour (the
/// last matching test wins, default side 3).
// src: BlockLockedChest.java:11-40
pub fn get_block_texture(world: &mut World, x: i32, y: i32, z: i32, var5: i32) -> i32 {
    if var5 == 1 || var5 == 0 {
        return BLOCK_INDEX_IN_TEXTURE - 1;
    }
    let var6 = world.get_block_id(x, y, z - 1);
    let var7 = world.get_block_id(x, y, z + 1);
    let var8 = world.get_block_id(x - 1, y, z);
    let var9 = world.get_block_id(x + 1, y, z);
    let opaque = super::opaque_cube_lookup;
    let mut var10 = 3;
    if opaque(var6) && !opaque(var7) {
        var10 = 3;
    }
    if opaque(var7) && !opaque(var6) {
        var10 = 2;
    }
    if opaque(var8) && !opaque(var9) {
        var10 = 5;
    }
    if opaque(var9) && !opaque(var8) {
        var10 = 4;
    }
    if var5 == var10 {
        BLOCK_INDEX_IN_TEXTURE + 1
    } else {
        BLOCK_INDEX_IN_TEXTURE
    }
}

/// `canPlaceBlockAt(world, x, y, z)`: always.
// src: BlockLockedChest.java:52-54
pub const fn can_place_block_at() -> bool {
    true
}

/// `updateTick(world, x, y, z, rand)`: removes itself (`rand` unused).
// src: BlockLockedChest.java:56-58
pub fn update_tick(world: &mut World, x: i32, y: i32, z: i32) {
    world.set_block_with_notify(x, y, z, 0);
}
