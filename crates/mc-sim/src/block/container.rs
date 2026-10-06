//! `BlockContainer` (BlockContainer.java): a block with a tile entity.
//!
//! Its constructors set `isBlockContainer[id]` (the table already has it,
//! [`super::is_block_container`]); its `onBlockAdded` creates the tile entity
//! (`getBlockEntity()`, one class per block) and its `onBlockRemoval`
//! removes it. Subclasses that override either call these through `super`.

use crate::tile_entity;
use crate::world::World;
use crate::world::tile_entity::{TileEntity, TileEntityKindData};

use super::{BlockKind, block};

/// `getBlockEntity()` (abstract, one override per subclass): a new tile
/// entity for container block `id`, `None` for an id that is not a
/// container (and for `BlockPistonMoving`, whose override returns null,
/// BlockPistonMoving.java:12-14; the pistons task owns that block).
// src: BlockChest.java:223-225, BlockFurnace.java:128-130, BlockDispenser.java:157-159,
// BlockNote.java:45-47, BlockSign.java:68-74, BlockMobSpawner.java:10-12, BlockJukeBox.java:62-64
pub fn get_block_entity(world: &World, id: i32) -> Option<TileEntityKindData> {
    let class = match block(id)?.kind {
        BlockKind::Chest => "TileEntityChest",
        BlockKind::Furnace { .. } => "TileEntityFurnace",
        BlockKind::Dispenser => "TileEntityDispenser",
        BlockKind::Note => "TileEntityNote",
        // `signEntityClass.newInstance()`: TileEntitySign for both signs (Block.java:655, :660).
        BlockKind::Sign { .. } => "TileEntitySign",
        BlockKind::MobSpawner => "TileEntityMobSpawner",
        BlockKind::JukeBox => "TileEntityRecordPlayer",
        _ => return None,
    };
    tile_entity::new_instance(class, world)
}

/// `BlockContainer.onBlockAdded(world, x, y, z)`: `super.onBlockAdded`
/// (Block's, empty), then `world.setBlockTileEntity(x, y, z,
/// getBlockEntity())`.
// src: BlockContainer.java:14-17
pub fn on_block_added(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    let Some(kind) = get_block_entity(world, id) else {
        return;
    };
    let r = world.new_tile_entity(TileEntity::new(kind));
    world.set_block_tile_entity(x, y, z, r);
}

/// `BlockContainer.onBlockRemoval(world, x, y, z)`: `super.onBlockRemoval`
/// (Block's, empty), then `world.removeBlockTileEntity(x, y, z)`.
// src: BlockContainer.java:19-22
pub fn on_block_removal(world: &mut World, x: i32, y: i32, z: i32) {
    world.remove_block_tile_entity(x, y, z);
}

/// The shared item scatter of `BlockChest`, `BlockFurnace` and
/// `BlockDispenser.onBlockRemoval`: for each non-empty slot of the tile
/// entity at `(x, y, z)`, three offsets from the block's own `Random`, then
/// stacks of `nextInt(21) + 10` (capped by what is left) as `EntityItem`s
/// with Gaussian motion, until the slot's stack is used up. The slot's own
/// stack is counted down to 0 in place (vanilla mutates the shared object
/// it got from `getStackInSlot`; the tile entity is removed right after).
/// `rand` selects the singleton `Random`.
///
/// A missing tile entity would throw in Java (`var5.getSizeInventory()`);
/// unreachable for a placed block, nothing happens here.
// src: BlockChest.java:155-183, BlockFurnace.java:152-182, BlockDispenser.java:181-209
pub(crate) fn scatter_inventory(
    world: &mut World,
    x: i32,
    y: i32,
    z: i32,
    rand: fn(&mut World) -> &mut mc_core::java_random::JavaRandom,
) {
    let Some(r) = world.get_block_tile_entity(x, y, z) else {
        return;
    };
    let Some(size) = world
        .tile_entities
        .get(&r)
        .and_then(|t| tile_entity::get_size_inventory(&t.kind))
    else {
        return;
    };
    for var6 in 0..size {
        let Some(var7) = world
            .tile_entities
            .get(&r)
            .and_then(|t| tile_entity::get_stack_in_slot(&t.kind, var6))
        else {
            continue;
        };
        let var8 = rand(world).next_float() * 0.8 + 0.1;
        let var9 = rand(world).next_float() * 0.8 + 0.1;
        let var10 = rand(world).next_float() * 0.8 + 0.1;
        let mut remaining = var7.stack_size;
        while remaining > 0 {
            let mut var11 = rand(world).next_int_bounded(21) + 10;
            if var11 > remaining {
                var11 = remaining;
            }
            remaining -= var11;
            set_slot_size(world, r, var6, remaining);
            let mut var12 = crate::entity::item::new_entity_item(
                world,
                (x as f32 + var8) as f64,
                (y as f32 + var9) as f64,
                (z as f32 + var10) as f64,
                crate::item::ItemStack::new(var7.item_id, var11, var7.item_damage),
            );
            let var13 = 0.05f32;
            var12.base.motion_x = (rand(world).next_gaussian() as f32 * var13) as f64;
            var12.base.motion_y = (rand(world).next_gaussian() as f32 * var13 + 0.2) as f64;
            var12.base.motion_z = (rand(world).next_gaussian() as f32 * var13) as f64;
            world.spawn_entity_in_world(var12);
        }
    }
}

/// `var7.stackSize -= var11` on the slot's stack (see [`scatter_inventory`]).
fn set_slot_size(
    world: &mut World,
    r: crate::world::tile_entity::TileEntityRef,
    slot: i32,
    n: i32,
) {
    let Some(t) = world.tile_entities.get_mut(&r) else {
        return;
    };
    let s = match &mut t.kind {
        TileEntityKindData::Chest(d) => d.chest_contents.get_mut(slot as usize),
        TileEntityKindData::Furnace(d) => d.furnace_item_stacks.get_mut(slot as usize),
        TileEntityKindData::Dispenser(d) => d.dispenser_contents.get_mut(slot as usize),
        _ => None,
    };
    if let Some(Some(st)) = s {
        st.stack_size = n;
    }
}
