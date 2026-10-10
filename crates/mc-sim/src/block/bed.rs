//! `BlockBed` (BlockBed.java): the bed (id 26), two blocks long.
//!
//! Metadata: bits 0-1 the direction, bit 2 occupied, bit 3 what MCP calls
//! the "foot" (`isBlockFootOfBed`; it is the half that drops nothing and the
//! one a click is redirected to). `HEAD_BLOCK_TO_FOOT_BLOCK_MAP[direction]`
//! steps from a block without bit 3 to the one with it.
//!
//! Transcribed: `blockActivated` (the block side; see below),
//! `setBlockBoundsBasedOnState` / `setBounds`, `onNeighborBlockChange`,
//! `getDirectionFromMetadata`, `isBlockFootOfBed`, `isBedOccupied`,
//! `setBedOccupied`, `getNearestEmptyChunkCoordinates`,
//! `dropBlockAsItemWithChance` (the dispatch arm runs `Block`'s body unless
//! [`is_block_foot_of_bed`]). Data methods (`idDropped`,
//! `getBlockTextureFromSideAndMetadata`, `getRenderType`, …) are in
//! [`crate::block`].
//!
//! The player side, `EntityPlayer.sleepInBedAt` and `bedChunkCoordinates`,
//! is [`crate::entity::player`] (WP-2.5). The bed exploding outside the
//! overworld is `World.newExplosion` (WP-1.8), run with the activating
//! player in hand ([`Explosion::do_explosion_a`], WP-4.8).
//! `addChatMessage` is a client-only message: `EntityPlayer`'s body is empty
//! (EntityPlayer.java:698-699), `EntityPlayerSP`'s writes to the chat GUI
//! (EntityPlayerSP.java:172-174): [`crate::entity::player::add_chat_message`]
//! ([`crate::world::client_requests::ClientRequest::ChatMessage`]).

use mc_core::java_hash::ChunkCoordinates;

use super::BlockBounds;
use crate::entity::Entity;
use crate::world::World;
use crate::world::explosion::Explosion;

/// `headBlockToFootBlockMap`: `{dx, dz}` per direction.
// src: BlockBed.java:7
pub const HEAD_BLOCK_TO_FOOT_BLOCK_MAP: [[i32; 2]; 4] = [[0, 1], [-1, 0], [0, -1], [1, 0]];

/// `getDirectionFromMetadata(meta)` (static).
// src: BlockBed.java:145-147
pub const fn get_direction_from_metadata(meta: i32) -> i32 {
    meta & 3
}

/// `isBlockFootOfBed(meta)` (static).
// src: BlockBed.java:149-151
pub const fn is_block_foot_of_bed(meta: i32) -> bool {
    (meta & 8) != 0
}

/// `isBedOccupied(meta)` (static).
// src: BlockBed.java:153-155
pub const fn is_bed_occupied(meta: i32) -> bool {
    (meta & 4) != 0
}

/// `headBlockToFootBlockMap[direction]` (direction is `meta & 3`, so in range).
fn step(direction: i32) -> [i32; 2] {
    HEAD_BLOCK_TO_FOOT_BLOCK_MAP[direction as usize]
}

/// `blockActivated(world, x, y, z, player)`: always true. Redirects to the
/// half with bit 3; outside the overworld removes both halves and explodes
/// (WP-1.8); otherwise refuses a bed another sleeping player is in, clears
/// a stale occupied bit and lets the player sleep
/// ([`crate::entity::player::sleep_in_bed_at`]), marking the bed occupied
/// on `OK`.
// src: BlockBed.java:14-83
pub fn block_activated(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    player: &mut Entity,
) -> bool {
    if world.multiplayer_world {
        return true;
    }
    let (mut var2, var3, mut var4) = (x, y, z);
    let mut var6 = world.get_block_metadata(var2, var3, var4);
    if !is_block_foot_of_bed(var6) {
        let var7 = get_direction_from_metadata(var6);
        var2 += step(var7)[0];
        var4 += step(var7)[1];
        if world.get_block_id(var2, var3, var4) != id {
            return true;
        }
        var6 = world.get_block_metadata(var2, var3, var4);
    }
    // src: BlockBed.java:30
    if !world.world_provider.can_respawn_here() {
        let mut var16 = var2 as f64 + 0.5;
        let mut var17 = var3 as f64 + 0.5;
        let mut var11 = var4 as f64 + 0.5;
        world.set_block_with_notify(var2, var3, var4, 0);
        let var13 = get_direction_from_metadata(var6);
        var2 += step(var13)[0];
        var4 += step(var13)[1];
        if world.get_block_id(var2, var3, var4) == id {
            world.set_block_with_notify(var2, var3, var4, 0);
            var16 = (var16 + var2 as f64 + 0.5) / 2.0;
            var17 = (var17 + var3 as f64 + 0.5) / 2.0;
            var11 = (var11 + var4 as f64 + 0.5) / 2.0;
        }
        // vanilla bug: the midpoint (var16, var17, var11) is computed and
        // never used; the explosion is centred on the second block,
        // `(double)((float)var2 + 0.5F)` etc., with power 5.0F and fire.
        let _ = (var16, var17, var11);
        // src: BlockBed.java:45, World.java:1522-1528 (`newExplosion`: the
        // explosion, flaming, both passes). The activating player is out of
        // its slot here (the client's click), but vanilla's entity scan
        // still finds it: it is passed in hand, as the TNT and the fireball
        // pass themselves, so the blast hurts and pushes it (before WP-4.8
        // the scan skipped it: the player next to a nether bed took no
        // damage, where vanilla's usually dies; tick/bed-nether-player-near).
        let mut var10 = Explosion::new(
            world,
            None,
            (var2 as f32 + 0.5) as f64,
            (var3 as f32 + 0.5) as f64,
            (var4 as f32 + 0.5) as f64,
            5.0,
        );
        var10.is_flaming = true;
        var10.do_explosion_a(world, None, Some(player));
        var10.do_explosion_b(world, true);
        return true;
    }
    if is_bed_occupied(var6) {
        // src: BlockBed.java:49-60: the player sleeping in this bed, if any.
        // `playerEntities` holds the activating player too, which is out of
        // its slot here, so it is read from the parameter.
        let mut var14: Option<i32> = None;
        for pid in world.player_entities.clone() {
            let bed_of = |p: &crate::entity::PlayerData| {
                if p.is_player_sleeping() {
                    Some(p.bed_chunk_coordinates)
                } else {
                    None
                }
            };
            let var10 = if pid == player.id() {
                player.kind.player().and_then(bed_of)
            } else {
                world
                    .entity(pid)
                    .and_then(|e| e.kind.player())
                    .and_then(bed_of)
            };
            if let Some(var10) = var10 {
                // A sleeping player always has its bed coordinates (Java would
                // throw on null, EntityPlayer.java:587-589).
                let c = var10.expect("a sleeping player has bedChunkCoordinates");
                if c.pos_x == var2 && c.pos_y == var3 && c.pos_z == var4 {
                    var14 = Some(pid);
                }
            }
        }
        if var14.is_some() {
            // src: BlockBed.java:63
            crate::entity::player::add_chat_message(player, world, "tile.bed.occupied");
            return true;
        }
        set_bed_occupied(world, var2, var3, var4, false);
    }
    // src: BlockBed.java:70-80: OK marks the bed occupied; NOT_POSSIBLE_NOW
    // adds the chat message "tile.bed.noSleep"; true either way.
    let var15 = crate::entity::player::sleep_in_bed_at(player, world, var2, var3, var4);
    if var15 == crate::entity::player::EnumStatus::Ok {
        set_bed_occupied(world, var2, var3, var4, true);
    } else if var15 == crate::entity::player::EnumStatus::NotPossibleNow {
        // src: BlockBed.java:76
        crate::entity::player::add_chat_message(player, world, "tile.bed.noSleep");
    }
    true
}

/// `setBlockBoundsBasedOnState(world, x, y, z)`: `setBounds()`.
// src: BlockBed.java:117-119
pub fn set_block_bounds_based_on_state(world: &mut World, id: i32) {
    set_bounds(world, id);
}

/// `setBounds()` (private): 9/16 high.
// src: BlockBed.java:141-143
fn set_bounds(world: &mut World, id: i32) {
    world.block_statics.bounds[(id & 255) as usize] =
        BlockBounds::from_floats(0.0, 0.0, 0.0, 1.0, 0.5625, 1.0);
}

/// `onNeighborBlockChange(world, x, y, z, neighborId)`: a half whose other
/// half is gone vanishes; only the half without bit 3 drops the bed item.
// src: BlockBed.java:121-135
pub fn on_neighbor_block_change(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    let var6 = world.get_block_metadata(x, y, z);
    let var7 = get_direction_from_metadata(var6);
    if is_block_foot_of_bed(var6) {
        if world.get_block_id(x - step(var7)[0], y, z - step(var7)[1]) != id {
            world.set_block_with_notify(x, y, z, 0);
        }
    } else if world.get_block_id(x + step(var7)[0], y, z + step(var7)[1]) != id {
        world.set_block_with_notify(x, y, z, 0);
        if !world.multiplayer_world {
            super::dispatch::drop_block_as_item(world, id, x, y, z, var6);
        }
    }
}

/// `setBedOccupied(world, x, y, z, occupied)` (static): sets or clears bit 2.
// src: BlockBed.java:157-166
pub fn set_bed_occupied(world: &mut World, x: i32, y: i32, z: i32, var4: bool) {
    let mut var5 = world.get_block_metadata(x, y, z);
    if var4 {
        var5 |= 4;
    } else {
        var5 &= -5;
    }
    world.set_block_metadata_with_notify(x, y, z, var5);
}

/// `getNearestEmptyChunkCoordinates(world, x, y, z, skip)` (static): the
/// `skip`-th (from 0) spot around the bed with a normal cube below and two
/// air blocks, searching the 3×3 around the block, then the 3×3 shifted one
/// step against the bed's direction; `None` (Java's null) if there is none.
/// Used by `EntityPlayer` to wake the player (WP-2.5).
// src: BlockBed.java:168-192
pub fn get_nearest_empty_chunk_coordinates(
    world: &mut World,
    x: i32,
    y: i32,
    z: i32,
    mut var4: i32,
) -> Option<ChunkCoordinates> {
    let var5 = world.get_block_metadata(x, y, z);
    let var6 = get_direction_from_metadata(var5);
    for var7 in 0..=1 {
        let var8 = x - step(var6)[0] * var7 - 1;
        let var9 = z - step(var6)[1] * var7 - 1;
        let var10 = var8 + 2;
        let var11 = var9 + 2;
        for var12 in var8..=var10 {
            for var13 in var9..=var11 {
                if world.is_block_normal_cube(var12, y - 1, var13)
                    && world.is_air_block(var12, y, var13)
                    && world.is_air_block(var12, y + 1, var13)
                {
                    if var4 <= 0 {
                        return Some(ChunkCoordinates::new(var12, y, var13));
                    }
                    var4 -= 1;
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_helpers() {
        assert_eq!(get_direction_from_metadata(11), 3);
        assert!(is_block_foot_of_bed(8) && !is_block_foot_of_bed(7));
        assert!(is_bed_occupied(4) && !is_bed_occupied(11));
        assert_eq!(HEAD_BLOCK_TO_FOOT_BLOCK_MAP[1], [-1, 0]);
    }
}
