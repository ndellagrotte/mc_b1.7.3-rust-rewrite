//! `BlockPortal` (BlockPortal.java): the nether portal block (id 90), a
//! `BlockBreakable` with `localFlag` false.
//!
//! Transcribed: `getCollisionBoundingBoxFromPool` (always null: the dispatch
//! arm returns `None`), `setBlockBoundsBasedOnState`, `tryToCreatePortal`
//! (called by `BlockFire.onBlockAdded`, BlockFire.java:192, the fire task of
//! WP-1.6), `onNeighborBlockChange`, `shouldSideBeRendered`,
//! `onEntityCollidedWithBlock`, `randomDisplayTick`. Data methods
//! (`isOpaqueCube`, `renderAsNormalBlock`, `quantityDropped`,
//! `getRenderBlockPass`) are in [`crate::block`].
//!
//! `onEntityCollidedWithBlock` calls `entity.setInPortal()`, which is empty
//! for every entity but a player (Entity.java:1038-1039); the player's
//! override (EntityPlayer.java:811-817) writes `timeUntilPortal` / `inPortal`,
//! which `PlayerData` does not have yet (portal travel, Phase 2): recorded as
//! `"EntityPlayer.setInPortal"` with [`World::note_unimplemented`].

use mc_core::java_random::JavaRandom;

use super::BlockBounds;
use crate::entity::Entity;
use crate::world::World;

/// `Block.obsidian.blockID`.
// src: Block.java:641
const OBSIDIAN: i32 = 49;
/// `Block.fire.blockID`.
// src: Block.java:643
const FIRE: i32 = 51;
/// `Block.portal.blockID`.
// src: Block.java:682
const PORTAL: i32 = 90;

/// `setBlockBoundsBasedOnState(world, x, y, z)`: a 1/4 thick pane along x
/// when a portal block is beside it on the x axis, else along z.
// src: BlockPortal.java:14-27
pub fn set_block_bounds_based_on_state(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    let b = if world.get_block_id(x - 1, y, z) != id && world.get_block_id(x + 1, y, z) != id {
        let var5 = 0.125f32;
        let var6 = 0.5f32;
        BlockBounds::from_floats(0.5 - var5, 0.0, 0.5 - var6, 0.5 + var5, 1.0, 0.5 + var6)
    } else {
        let var5 = 0.5f32;
        let var6 = 0.125f32;
        BlockBounds::from_floats(0.5 - var5, 0.0, 0.5 - var6, 0.5 + var5, 1.0, 0.5 + var6)
    };
    world.block_statics.bounds[(id & 255) as usize] = b;
}

/// `tryToCreatePortal(world, x, y, z)`: if `(x, y, z)` is the inside of an
/// obsidian frame 4 wide and 5 high (corners not checked) whose 2×3 inside
/// is air or fire, fills the inside with portal blocks (with
/// `editingBlocks` set, so no neighbour hears of it) and returns true.
// src: BlockPortal.java:37-85
pub fn try_to_create_portal(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    let (mut var2, var3, mut var4) = (x, y, z);
    let mut var5 = 0;
    let mut var6 = 0;
    if world.get_block_id(var2 - 1, var3, var4) == OBSIDIAN
        || world.get_block_id(var2 + 1, var3, var4) == OBSIDIAN
    {
        var5 = 1;
    }
    if world.get_block_id(var2, var3, var4 - 1) == OBSIDIAN
        || world.get_block_id(var2, var3, var4 + 1) == OBSIDIAN
    {
        var6 = 1;
    }
    if var5 == var6 {
        return false;
    }
    if world.get_block_id(var2 - var5, var3, var4 - var6) == 0 {
        var2 -= var5;
        var4 -= var6;
    }
    for var7 in -1..=2 {
        for var8 in -1..=3 {
            let var9 = var7 == -1 || var7 == 2 || var8 == -1 || var8 == 3;
            // the four corners are skipped
            if (var7 != -1 && var7 != 2) || (var8 != -1 && var8 != 3) {
                let var10 = world.get_block_id(var2 + var5 * var7, var3 + var8, var4 + var6 * var7);
                if var9 {
                    if var10 != OBSIDIAN {
                        return false;
                    }
                } else if var10 != 0 && var10 != FIRE {
                    return false;
                }
            }
        }
    }
    world.editing_blocks = true;
    for var7 in 0..2 {
        for var8 in 0..3 {
            world.set_block_with_notify(
                var2 + var5 * var7,
                var3 + var8,
                var4 + var6 * var7,
                PORTAL,
            );
        }
    }
    world.editing_blocks = false;
    true
}

/// `onNeighborBlockChange(world, x, y, z, neighborId)`: removes the block
/// unless it stands in a column of exactly three portal blocks between
/// obsidian below and above, and has, along its axis, obsidian on one side
/// and portal on the other; or when it has portal neighbours on both axes.
// src: BlockPortal.java:87-118
pub fn on_neighbor_block_change(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    let mut var6 = 0;
    let mut var7 = 1;
    if world.get_block_id(x - 1, y, z) == id || world.get_block_id(x + 1, y, z) == id {
        var6 = 1;
        var7 = 0;
    }
    let mut var8 = y;
    while world.get_block_id(x, var8 - 1, z) == id {
        var8 -= 1;
    }
    if world.get_block_id(x, var8 - 1, z) != OBSIDIAN {
        world.set_block_with_notify(x, y, z, 0);
    } else {
        let mut var9 = 1;
        while var9 < 4 && world.get_block_id(x, var8 + var9, z) == id {
            var9 += 1;
        }
        if var9 == 3 && world.get_block_id(x, var8 + var9, z) == OBSIDIAN {
            let var10 =
                world.get_block_id(x - 1, y, z) == id || world.get_block_id(x + 1, y, z) == id;
            let var11 =
                world.get_block_id(x, y, z - 1) == id || world.get_block_id(x, y, z + 1) == id;
            // Java's `if (var10 && var11) remove; else if (...) remove;` as one
            // test, with the same short-circuit order.
            if (var10 && var11)
                || ((world.get_block_id(x + var6, y, z + var7) != OBSIDIAN
                    || world.get_block_id(x - var6, y, z - var7) != id)
                    && (world.get_block_id(x - var6, y, z - var7) != OBSIDIAN
                        || world.get_block_id(x + var6, y, z + var7) != id))
            {
                world.set_block_with_notify(x, y, z, 0);
            }
        } else {
            world.set_block_with_notify(x, y, z, 0);
        }
    }
}

/// `shouldSideBeRendered(blockAccess, x, y, z, side)`: `x, y, z` is the
/// neighbour on `side`. Never towards another portal block; otherwise only
/// the two faces across the pane, for a neighbour cell next to the pane's
/// edge block.
// src: BlockPortal.java:120-140
pub fn should_side_be_rendered(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    side: i32,
) -> bool {
    if world.get_block_id(x, y, z) == id {
        return false;
    }
    let var6 = world.get_block_id(x - 1, y, z) == id && world.get_block_id(x - 2, y, z) != id;
    let var7 = world.get_block_id(x + 1, y, z) == id && world.get_block_id(x + 2, y, z) != id;
    let var8 = world.get_block_id(x, y, z - 1) == id && world.get_block_id(x, y, z - 2) != id;
    let var9 = world.get_block_id(x, y, z + 1) == id && world.get_block_id(x, y, z + 2) != id;
    let var10 = var6 || var7;
    let var11 = var8 || var9;
    // Java's chain of `return true` branches (sides 4, 5, 2, then 3).
    (var10 && side == 4) || (var10 && side == 5) || (var11 && side == 2) || (var11 && side == 3)
}

/// `onEntityCollidedWithBlock(world, x, y, z, entity)`: an entity that
/// neither rides nor is ridden is put "in the portal" (module docs).
// src: BlockPortal.java:150-155
pub fn on_entity_collided_with_block(world: &mut World, e: &mut Entity) {
    if e.base.riding_entity.is_none() && e.base.ridden_by_entity.is_none() {
        // src: BlockPortal.java:152: var5.setInPortal(); Entity's body is empty
        // (Entity.java:1038-1039), EntityPlayer's is Phase 2 (module docs).
        if e.kind.is_player() {
            world.note_unimplemented("EntityPlayer.setInPortal");
        }
    }
}

/// `randomDisplayTick(world, x, y, z, rand)`: a 1-in-100 portal hum and four
/// purple particles drifting across the pane. `rand` is the display
/// `Random` ([`super::dispatch::random_display_tick`]).
// src: BlockPortal.java:157-184
pub fn random_display_tick(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    rand: &mut JavaRandom,
) {
    if rand.next_int_bounded(100) == 0 {
        let pitch = rand.next_float() * 0.4 + 0.8;
        world.play_sound_effect(
            x as f64 + 0.5,
            y as f64 + 0.5,
            z as f64 + 0.5,
            "portal.portal",
            1.0,
            pitch,
        );
    }
    for _var6 in 0..4 {
        let mut var7 = (x as f32 + rand.next_float()) as f64;
        let var9 = (y as f32 + rand.next_float()) as f64;
        let mut var11 = (z as f32 + rand.next_float()) as f64;
        let var19 = rand.next_int_bounded(2) * 2 - 1;
        let mut var13 = (rand.next_float() as f64 - 0.5) * 0.5;
        let var15 = (rand.next_float() as f64 - 0.5) * 0.5;
        let mut var17 = (rand.next_float() as f64 - 0.5) * 0.5;
        if world.get_block_id(x - 1, y, z) != id && world.get_block_id(x + 1, y, z) != id {
            var7 = x as f64 + 0.5 + 0.25 * var19 as f64;
            var13 = (rand.next_float() * 2.0 * var19 as f32) as f64;
        } else {
            var11 = z as f64 + 0.5 + 0.25 * var19 as f64;
            var17 = (rand.next_float() * 2.0 * var19 as f32) as f64;
        }
        world.spawn_particle("portal", var7, var9, var11, var13, var15, var17);
    }
}
