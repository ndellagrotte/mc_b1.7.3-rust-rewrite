//! `BlockTNT` (BlockTNT.java): TNT primes when powered, when a player
//! breaks it with metadata bit 1 set (a flint-and-steel left-click sets it),
//! when fire reaches it (`BlockFire.tryToCatchBlockOnFire` calls
//! `Block.tnt.onBlockDestroyedByPlayer(world, x, y, z, 1)`,
//! BlockFire.java:131) and when an explosion removes it.
//!
//! Data methods (`getBlockTextureFromSide`, `quantityDropped` 0) are in
//! [`crate::block`]. Everything else is transcribed here except the
//! `EntityTNTPrimed` itself: its constructor and update are WP-2.4, so the
//! spawn sites record `"EntityTNTPrimed.<init>"` with
//! [`World::note_unimplemented`] and spawn nothing (see [`spawn_primed`]).

use crate::block::dispatch;
use crate::entity::Entity;
use crate::item::ItemStack;
use crate::world::World;

/// `Block.tnt.blockID`.
// src: Block.java:638
const TNT_ID: i32 = 46;

/// `Item.flintAndSteel.shiftedIndex`: `new ItemFlintAndSteel(3)`, so
/// 256 + 3.
// src: Item.java:277
const FLINT_AND_STEEL_ID: i32 = 256 + 3;

/// `EntityTNTPrimed.fuse` as its position constructor leaves it.
// src: EntityTNTPrimed.java:21
const PRIMED_FUSE: i32 = 80;

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
    // src: BlockTNT.java:40: new EntityTNTPrimed(world, (float)x + 0.5F, …)
    spawn_primed(world, x, y, z);
    // src: BlockTNT.java:41: the fuse draw runs on World.rand after the
    // constructor and before the join; kept so World.rand stays in step.
    let _fuse = world.rand.next_int_bounded(PRIMED_FUSE / 4) + PRIMED_FUSE / 8;
    // src: BlockTNT.java:42: world.entityJoinedWorld(var5): WP-2.4 (spawn_primed)
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
            // src: BlockTNT.java:50-51: construct and join (WP-2.4)
            spawn_primed(world, x, y, z);
            // src: BlockTNT.java:52: playSoundAtEntity(var6, "random.fuse", 1.0F, 1.0F)
            // at the entity's feet, posY - yOffset: setPosition put it at
            // ((float)x + 0.5F, …) and yOffset = height / 2.0F with
            // setSize(0.98F, 0.98F) (EntityTNTPrimed.java:10-11, :15).
            let y_offset = 0.98f32 / 2.0f32;
            world.play_sound_effect(
                (x as f32 + 0.5) as f64,
                (y as f32 + 0.5) as f64 - y_offset as f64,
                (z as f32 + 0.5) as f64,
                "random.fuse",
                1.0,
                1.0,
            );
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

/// `new EntityTNTPrimed(world, (float)x + 0.5F, (float)y + 0.5F,
/// (float)z + 0.5F)` followed by `world.entityJoinedWorld`. The entity
/// (constructor with its `Math.random()` kick, EntityTNTPrimed.java:14-25,
/// and its update) is WP-2.4: recorded, nothing spawned and nothing drawn
/// from the `Math.random()` stream.
// src: BlockTNT.java:40, :42, :50-51
fn spawn_primed(world: &mut World, x: i32, y: i32, z: i32) {
    let _ = (x, y, z);
    world.note_unimplemented("EntityTNTPrimed.<init>");
}
