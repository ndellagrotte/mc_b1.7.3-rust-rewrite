//! `BlockStep` (BlockStep.java): the double slab (id 43, `blockType` true)
//! and the single slab (id 44, `blockType` false). Metadata 0..3: stone,
//! sandstone, wood, cobblestone.
//!
//! Transcribed: `onBlockAdded` (a slab on a single slab of the same kind
//! merges into a double slab) and `shouldSideBeRendered`. The data methods
//! (`isOpaqueCube`, `renderAsNormalBlock`, `idDropped`, `quantityDropped`,
//! `damageDropped`, the texture methods) are in [`crate::block`], and the
//! static `field_22037_a` is [`crate::block::STEP_TYPE_NAMES`].

use super::ids::STAIR_SINGLE;
use crate::world::World;

/// `Block.stairDouble.blockID`.
// src: Block.java:635
const STAIR_DOUBLE: i32 = 43;

/// `onBlockAdded(world, x, y, z)`: when the block below is a single slab
/// with the same metadata, this block is removed and the one below becomes
/// a double slab. Runs for both slab ids: a double slab placed on a single
/// slab of its kind also merges into it (vanilla quirk, transcribed).
// src: BlockStep.java:43-58
pub fn on_block_added(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    // src: BlockStep.java:44-46: `if (this != Block.stairSingle)
    // super.onBlockAdded(...)`, Block's empty body (Block.java:313-314).
    let _ = id;
    let var5 = world.get_block_id(x, y - 1, z);
    let var6 = world.get_block_metadata(x, y, z);
    let var7 = world.get_block_metadata(x, y - 1, z);
    if var6 == var7 && var5 == STAIR_SINGLE {
        world.set_block_with_notify(x, y, z, 0);
        world.set_block_and_metadata_with_notify(x, y - 1, z, STAIR_DOUBLE, var6);
    }
}

/// `shouldSideBeRendered(blockAccess, x, y, z, side)`: `x, y, z` is the
/// neighbour on `side`; `base` is `super.shouldSideBeRendered` (`Block`'s
/// body on this block's bounds). The top face always; otherwise what
/// `Block` says, except that a side face next to the same slab id is hidden.
// src: BlockStep.java:76-90
pub fn should_side_be_rendered(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    side: i32,
    mut base: impl FnMut(&mut World) -> bool,
) -> bool {
    if id != STAIR_SINGLE {
        // src: BlockStep.java:78: the result is discarded (vanilla, transcribed:
        // the call only reads the world).
        base(world);
    }
    if side == 1 {
        true
    } else if !base(world) {
        false
    } else if side == 0 {
        true
    } else {
        world.get_block_id(x, y, z) != id
    }
}
