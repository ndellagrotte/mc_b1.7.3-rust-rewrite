//! `BlockTNT` (BlockTNT.java): TNT primes when powered, when a player
//! breaks it with metadata bit 1 set (a flint-and-steel left-click sets it),
//! when fire reaches it (`BlockFire.tryToCatchBlockOnFire` calls
//! `Block.tnt.onBlockDestroyedByPlayer(world, x, y, z, 1)`,
//! BlockFire.java:131) and when an explosion removes it.
//!
//! Data methods (`getBlockTextureFromSide`, `quantityDropped` 0) are in
//! [`crate::block`]. Everything else is transcribed here; the primed TNT
//! itself is [`crate::entity::tnt_primed`] (WP-2.4).

use crate::block::dispatch;
use crate::entity::Entity;
use crate::entity::tnt_primed::{TntPrimedData, new_entity_tnt_primed};
use crate::item::ItemStack;
use crate::world::World;

/// `Block.tnt.blockID`.
// src: Block.java:638
const TNT_ID: i32 = 46;

/// `Item.flintAndSteel.shiftedIndex`: `new ItemFlintAndSteel(3)`, so
/// 256 + 3.
// src: Item.java:277
const FLINT_AND_STEEL_ID: i32 = 256 + 3;

/// `BlockTNT.onBlockAdded(world, x, y, z)`: `super.onBlockAdded` (empty),
/// then a TNT placed into power primes at once.
// src: BlockTNT.java:18-25
pub fn on_block_added(world: &mut World, x: i32, y: i32, z: i32) {
    // src: BlockTNT.java:19: super.onBlockAdded is Block's empty body (Block.java:313-314)
    if world.is_block_indirectly_getting_powered(x, y, z) {
        // src: BlockTNT.java:21-22
        on_block_destroyed_by_player(world, x, y, z, 1);
        world.set_block_with_notify(x, y, z, 0);
    }
}

/// `BlockTNT.onNeighborBlockChange(world, x, y, z, neighborId)`: primes when
/// the changed neighbour is a power source and the TNT is now powered.
// src: BlockTNT.java:27-33
pub fn on_neighbor_block_change(world: &mut World, x: i32, y: i32, z: i32, neighbor_id: i32) {
    // src: BlockTNT.java:28: `var5 > 0 && Block.blocksList[var5].canProvidePower() && …`
    if neighbor_id > 0
        && dispatch::can_provide_power(world, neighbor_id)
        && world.is_block_indirectly_getting_powered(x, y, z)
    {
        // src: BlockTNT.java:29-30
        on_block_destroyed_by_player(world, x, y, z, 1);
        world.set_block_with_notify(x, y, z, 0);
    }
}

/// `BlockTNT.onBlockDestroyedByExplosion(world, x, y, z)`: a primed TNT with
/// a shortened fuse, `world.rand.nextInt(fuse / 4) + fuse / 8`.
// src: BlockTNT.java:39-43
pub fn on_block_destroyed_by_explosion(world: &mut World, x: i32, y: i32, z: i32) {
    // src: BlockTNT.java:40
    let mut var5 = new_primed(world, x, y, z);
    // src: BlockTNT.java:41: var5.fuse = world.rand.nextInt(var5.fuse / 4) + var5.fuse / 8
    if let crate::entity::EntityKindData::TntPrimed(d) = &mut var5.kind {
        let fuse = d.fuse;
        *d = TntPrimedData {
            fuse: world.rand.next_int_bounded(fuse / 4) + fuse / 8,
        };
    }
    // src: BlockTNT.java:42
    world.spawn_entity_in_world(var5);
}

/// `BlockTNT.onBlockDestroyedByPlayer(world, x, y, z, meta)`: with bit 1 of
/// `meta` clear the TNT drops as an item (`quantityDropped` is 0, so this is
/// its only item drop); with it set a primed TNT appears and hisses.
/// Nothing on a multiplayer client world.
// src: BlockTNT.java:45-56
pub fn on_block_destroyed_by_player(world: &mut World, x: i32, y: i32, z: i32, meta: i32) {
    if !world.multiplayer_world {
        if (meta & 1) == 0 {
            // src: BlockTNT.java:48
            dispatch::drop_block_as_item_do(world, TNT_ID, x, y, z, ItemStack::new(TNT_ID, 1, 0));
        } else {
            // src: BlockTNT.java:50-51
            let var6 = new_primed(world, x, y, z);
            // src: BlockTNT.java:52: playSoundAtEntity(var6, "random.fuse", 1.0F, 1.0F)
            // at the entity's feet (World.java:863-868: posY - yOffset); joining
            // does not move it.
            let (sx, sy, sz) = (
                var6.base.pos_x,
                var6.base.pos_y - var6.base.y_offset as f64,
                var6.base.pos_z,
            );
            world.spawn_entity_in_world(var6);
            world.play_sound_effect(sx, sy, sz, "random.fuse", 1.0, 1.0);
        }
    }
}

/// `BlockTNT.onBlockClicked(world, x, y, z, player)`: a left-click holding
/// flint and steel sets metadata 1 (no notification), so the break that
/// follows primes the TNT; then `super.onBlockClicked` (empty).
// src: BlockTNT.java:58-64
pub fn on_block_clicked(world: &mut World, x: i32, y: i32, z: i32, player: &mut Entity) {
    // src: BlockTNT.java:59: getCurrentEquippedItem() = inventory.getCurrentItem()
    // (EntityPlayer.java:471-473)
    let holds_flint_and_steel = player
        .kind
        .player()
        .expect("BlockTNT.onBlockClicked on a non-player (Java ClassCastException)")
        .inventory
        .get_current_item()
        .is_some_and(|s| s.item_id == FLINT_AND_STEEL_ID);
    if holds_flint_and_steel {
        // src: BlockTNT.java:60
        world.set_block_metadata(x, y, z, 1);
    }
    // src: BlockTNT.java:63: super.onBlockClicked is Block's empty body (Block.java:516-517)
}

/// `BlockTNT.blockActivated(world, x, y, z, player)`: only
/// `super.blockActivated`, which is `false`.
// src: BlockTNT.java:66-68
pub fn block_activated() -> bool {
    // src: Block.java:506-508
    false
}

/// `new EntityTNTPrimed(world, (double)((float)x + 0.5F), (double)((float)y
/// + 0.5F), (double)((float)z + 0.5F))` (the constructor draws the kick from
/// `Math.random()`, EntityTNTPrimed.java:17).
// src: BlockTNT.java:40, :50
fn new_primed(world: &mut World, x: i32, y: i32, z: i32) -> Entity {
    new_entity_tnt_primed(
        world,
        (x as f32 + 0.5) as f64,
        (y as f32 + 0.5) as f64,
        (z as f32 + 0.5) as f64,
    )
}
