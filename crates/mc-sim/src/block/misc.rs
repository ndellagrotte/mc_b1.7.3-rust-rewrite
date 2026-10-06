//! The small block classes of the WP-1.6 "simple remainder" task whose
//! behaviour is more than data: `BlockSponge` and `BlockBreakable`.
//!
//! Audited and complete as data in [`crate::block`] (every override is a
//! constant, a field read or a pure function of `meta`/`rand`, pinned per id
//! by `tests/golden/block/block-table`): `BlockStone` (`idDropped`),
//! `BlockObsidian` (`idDropped`, `quantityDropped`), `BlockOre`
//! (`idDropped`, `quantityDropped`, `damageDropped`), `BlockOreStorage`
//! (`getBlockTextureFromSide`), `BlockCloth` (texture, `damageDropped`; its
//! static `getBlockFromDye` / `getDyeFromBlock` are
//! [`crate::item::block_cloth_get_block_from_dye`]), `BlockGravel`
//! (`idDropped`; `quantityDropped` is `Block`'s 1), `BlockClay`,
//! `BlockGlowStone`, `BlockBookshelf`, `BlockSandStone`, `BlockGlass`
//! (`quantityDropped` 0, `getRenderBlockPass` 0), `BlockMobSpawner`
//! (`idDropped`, `quantityDropped`, `isOpaqueCube`; the tile entity and the
//! `BlockContainer` hooks are WP-1.8) and `BlockNetherrack` (no override).
//! `BlockSand` is [`super::sand`]; `BlockTNT`, `BlockSoulSand`, `BlockWeb`
//! and `BlockPumpkin` have their own modules.

use crate::block::Material;
use crate::world::World;

/// `BlockSponge.onBlockAdded(world, x, y, z)`: scans the 5×5×5 box around
/// the sponge for water and does nothing with it.
// src: BlockSponge.java:9-21
pub fn sponge_on_block_added(world: &mut World, x: i32, y: i32, z: i32) {
    let var5 = 2;
    for var6 in (x - var5)..=(x + var5) {
        for var7 in (y - var5)..=(y + var5) {
            for var8 in (z - var5)..=(z + var5) {
                // vanilla bug: the `if` body is empty (BlockSponge.java:15-16), so a
                // sponge absorbs no water; only the reads remain, and
                // `getBlockMaterial` loads any missing chunk it touches.
                let _is_water = world.get_block_material(var6, var7, var8) == Material::Water;
            }
        }
    }
}

/// `BlockSponge.onBlockRemoval(world, x, y, z)`: notifies the neighbours of
/// every block of the 5×5×5 box, x outer, y middle, z inner, each with that
/// block's own id.
// src: BlockSponge.java:23-34
pub fn sponge_on_block_removal(world: &mut World, x: i32, y: i32, z: i32) {
    let var5 = 2;
    for var6 in (x - var5)..=(x + var5) {
        for var7 in (y - var5)..=(y + var5) {
            for var8 in (z - var5)..=(z + var5) {
                let id = world.get_block_id(var6, var7, var8);
                world.notify_blocks_of_neighbor_change(var6, var7, var8, id);
            }
        }
    }
}

/// `BlockBreakable.shouldSideBeRendered(blockAccess, x, y, z, side)`: a face
/// against the same block is hidden unless `localFlag`; otherwise
/// `super.shouldSideBeRendered`, which the caller passes as `base` (the
/// dispatch's `Block` body; `BlockIce` reaches this through its own `super`
/// call with `1 - side`, BlockIce.java:16-18).
// src: BlockBreakable.java:15-18
pub fn breakable_should_side_be_rendered(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    side: i32,
    base: fn(&mut World, i32, i32, i32, i32, i32) -> bool,
) -> bool {
    let local_flag = super::block(id)
        .and_then(|b| b.kind.breakable_local_flag())
        .expect("BlockBreakable.shouldSideBeRendered on a block that is not a BlockBreakable");
    let var6 = world.get_block_id(x, y, z);
    if !local_flag && var6 == id {
        false
    } else {
        base(world, id, x, y, z, side)
    }
}
