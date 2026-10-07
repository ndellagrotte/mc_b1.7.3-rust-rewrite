//! `BlockDispenser` (BlockDispenser.java): fires one item when powered.
//!
//! Dispensed arrows, eggs and snowballs are `EntityArrow` / `EntityEgg` /
//! `EntitySnowball` ([`crate::entity`], WP-2.4) with their heading drawn on
//! the projectile's own `rand` (before it joins the world: no harness hook,
//! tools/oracle/census.md §3). Any other item is an `EntityItem`, with
//! velocity from `updateTick`'s `Random` (World.rand). The dispenser singleton's `Random`
//! ([`super::BlockStatics::dispenser_random`]) only scatters items when the
//! block is broken.

use mc_core::math::floor_double;

use super::{block, container, dispatch, opaque_cube_lookup};
use crate::entity::{Entity, EntityKindData, arrow, egg, player, snowball};
use crate::tile_entity::dispenser::get_random_stack_from_inventory;
use crate::world::World;
use crate::world::client_requests::PlayerGui;

// src: Item.java:280 (arrow 6), :362 (egg 88), :350 (snowball 76)
const ARROW: i32 = 256 + 6;
const EGG: i32 = 256 + 88;
const SNOWBALL: i32 = 256 + 76;

/// `blockIndexInTexture` (45, BlockDispenser.java:10).
fn tex(id: i32) -> i32 {
    block(id).map_or(45, |b| b.block_index_in_texture)
}

/// `onBlockAdded(world, x, y, z)`: `BlockContainer.onBlockAdded`, then
/// `setDispenserDefaultDirection`.
// src: BlockDispenser.java:21-24
pub fn on_block_added(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    container::on_block_added(world, id, x, y, z);
    set_dispenser_default_direction(world, x, y, z);
}

/// `setDispenserDefaultDirection(world, x, y, z)` (private): as the
/// furnace's, faces away from an opaque neighbour, with notification.
// src: BlockDispenser.java:26-51
fn set_dispenser_default_direction(world: &mut World, x: i32, y: i32, z: i32) {
    if !world.multiplayer_world {
        let var5 = world.get_block_id(x, y, z - 1);
        let var6 = world.get_block_id(x, y, z + 1);
        let var7 = world.get_block_id(x - 1, y, z);
        let var8 = world.get_block_id(x + 1, y, z);
        let o = opaque_cube_lookup;
        let mut var9 = 3;
        if o(var5) && !o(var6) {
            var9 = 3;
        }
        if o(var6) && !o(var5) {
            var9 = 2;
        }
        if o(var7) && !o(var8) {
            var9 = 5;
        }
        if o(var8) && !o(var7) {
            var9 = 4;
        }
        world.set_block_metadata_with_notify(x, y, z, var9);
    }
}

/// `getBlockTexture(blockAccess, x, y, z, side)`: top and bottom 62, the
/// front (side == metadata) 46, else 45.
// src: BlockDispenser.java:53-62
pub fn get_block_texture(world: &mut World, id: i32, x: i32, y: i32, z: i32, side: i32) -> i32 {
    let bi = tex(id);
    if side == 1 || side == 0 {
        bi + 17
    } else {
        let var6 = world.get_block_metadata(x, y, z);
        if side != var6 { bi } else { bi + 1 }
    }
}

/// `blockActivated(world, x, y, z, player)`: true; off a multiplayer client
/// `player.displayGUIDispenser(tileEntity)` after the tile-entity lookup:
/// the client's `GuiDispenser` ([`player::display_gui`],
/// [`PlayerGui::Dispenser`]).
// src: BlockDispenser.java:74-82
pub fn block_activated(world: &mut World, x: i32, y: i32, z: i32, player: &mut Entity) -> bool {
    if !world.multiplayer_world {
        let var6 = world.get_block_tile_entity(x, y, z);
        player::display_gui(
            player,
            world,
            PlayerGui::Dispenser {
                x,
                y,
                z,
                tile_entity: var6,
            },
        );
    }
    true
}

/// `dispenseItem(world, x, y, z, rand)` (private): one random stack's item
/// out of the front face (metadata 2 -z, 3 +z, 5 +x, anything else -x).
/// Empty: aux sound 1001 (click). An item: an `EntityItem` 0.3 below the
/// face centre with `rand` velocity, aux 1000; arrows
/// (`doesArrowBelongToPlayer = true`), eggs and snowballs: the
/// `(World, x, y, z)` constructor at the face, `set*Heading(var9, 0.1, var10,
/// 1.1F, 6.0F)` on the projectile's own `rand`, `entityJoinedWorld`, aux 1002
/// (BlockDispenser.java:107-121). Then smoke, aux 2000. `rand` is the
/// `updateTick`'s, which the world passes as `World.rand` (World.java:1159,
/// :1960) — vanilla, transcribed: not the dispenser's own `random`.
// src: BlockDispenser.java:84-138
fn dispense_item(world: &mut World, x: i32, y: i32, z: i32, rand: dispatch::RandomSource) {
    let var6 = world.get_block_metadata(x, y, z);
    let mut var9: i32 = 0;
    let mut var10: i32 = 0;
    if var6 == 3 {
        var10 = 1;
    } else if var6 == 2 {
        var10 = -1;
    } else if var6 == 5 {
        var9 = 1;
    } else {
        var9 = -1;
    }
    let var11 = world.get_block_tile_entity(x, y, z);
    // `var11.getRandomStackFromInventory()`: a missing tile entity would
    // throw in Java (unreachable for a placed dispenser).
    let Some(r) = var11 else { return };
    let var12 = get_random_stack_from_inventory(world, r);
    let var13 = x as f64 + var9 as f64 * 0.6 + 0.5;
    let var15 = y as f64 + 0.5;
    let var17 = z as f64 + var10 as f64 * 0.6 + 0.5;
    match var12 {
        None => world.play_aux_sfx(1001, x, y, z, 0),
        Some(var12) => {
            if var12.item_id == ARROW {
                // src: BlockDispenser.java:107-111
                let mut var19 = arrow::new_entity_arrow_at(world, var13, var15, var17);
                arrow::set_arrow_heading(
                    &mut var19,
                    var9 as f64,
                    0.10000000149011612,
                    var10 as f64,
                    1.1,
                    6.0,
                );
                if let EntityKindData::Arrow(d) = &mut var19.kind {
                    d.does_arrow_belong_to_player = true;
                }
                world.spawn_entity_in_world(var19);
                world.play_aux_sfx(1002, x, y, z, 0);
            } else if var12.item_id == EGG {
                // src: BlockDispenser.java:113-116
                let mut var22 = egg::new_entity_egg_at(world, var13, var15, var17);
                egg::set_egg_heading(
                    &mut var22,
                    var9 as f64,
                    0.10000000149011612,
                    var10 as f64,
                    1.1,
                    6.0,
                );
                world.spawn_entity_in_world(var22);
                world.play_aux_sfx(1002, x, y, z, 0);
            } else if var12.item_id == SNOWBALL {
                // src: BlockDispenser.java:118-121
                let mut var23 = snowball::new_entity_snowball_at(world, var13, var15, var17);
                snowball::set_snowball_heading(
                    &mut var23,
                    var9 as f64,
                    0.10000000149011612,
                    var10 as f64,
                    1.1,
                    6.0,
                );
                world.spawn_entity_in_world(var23);
                world.play_aux_sfx(1002, x, y, z, 0);
            } else {
                // src: BlockDispenser.java:123-132
                let mut var24 =
                    crate::entity::item::new_entity_item(world, var13, var15 - 0.3, var17, var12);
                let rng = world.random(rand);
                let var20 = rng.next_double() * 0.1 + 0.2;
                var24.base.motion_x = var9 as f64 * var20;
                var24.base.motion_y = 0.20000000298023224;
                var24.base.motion_z = var10 as f64 * var20;
                let rng = world.random(rand);
                var24.base.motion_x += rng.next_gaussian() * 0.007499999832361937 * 6.0;
                var24.base.motion_y += rng.next_gaussian() * 0.007499999832361937 * 6.0;
                var24.base.motion_z += rng.next_gaussian() * 0.007499999832361937 * 6.0;
                world.spawn_entity_in_world(var24);
                world.play_aux_sfx(1000, x, y, z, 0);
            }
            world.play_aux_sfx(2000, x, y, z, var9 + 1 + (var10 + 1) * 3);
        }
    }
}

/// `onNeighborBlockChange(world, x, y, z, neighborId)`: when the neighbour
/// is a power source and the dispenser or the block above it is powered,
/// schedules a tick (`tickRate()` 4).
// src: BlockDispenser.java:140-148
pub fn on_neighbor_block_change(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    neighbor_id: i32,
) {
    if neighbor_id > 0 && dispatch::can_provide_power(world, neighbor_id) {
        let var6 = world.is_block_indirectly_getting_powered(x, y, z)
            || world.is_block_indirectly_getting_powered(x, y + 1, z);
        if var6 {
            world.schedule_block_update(x, y, z, id, dispatch::tick_rate(id));
        }
    }
}

/// `updateTick(world, x, y, z, rand)`: dispenses while powered (the
/// dispenser or the block above it).
// src: BlockDispenser.java:150-155
pub fn update_tick(world: &mut World, x: i32, y: i32, z: i32, rand: dispatch::RandomSource) {
    if world.is_block_indirectly_getting_powered(x, y, z)
        || world.is_block_indirectly_getting_powered(x, y + 1, z)
    {
        dispense_item(world, x, y, z, rand);
    }
}

/// `onBlockPlacedBy(world, x, y, z, placer)`: faces the placer, as the
/// furnace.
// src: BlockDispenser.java:161-179
pub fn on_block_placed_by(world: &mut World, x: i32, y: i32, z: i32, placer: &Entity) {
    let var6 = floor_double((placer.base.rotation_yaw * 4.0 / 360.0) as f64 + 0.5) & 3;
    if var6 == 0 {
        world.set_block_metadata_with_notify(x, y, z, 2);
    }
    if var6 == 1 {
        world.set_block_metadata_with_notify(x, y, z, 5);
    }
    if var6 == 2 {
        world.set_block_metadata_with_notify(x, y, z, 3);
    }
    if var6 == 3 {
        world.set_block_metadata_with_notify(x, y, z, 4);
    }
}

/// `onBlockRemoval(world, x, y, z)`: scatters the items with the
/// dispenser's `random`, then `BlockContainer.onBlockRemoval`.
// src: BlockDispenser.java:181-209
pub fn on_block_removal(world: &mut World, x: i32, y: i32, z: i32) {
    container::scatter_inventory(world, x, y, z, |w| &mut w.block_statics.dispenser_random);
    container::on_block_removal(world, x, y, z);
}

#[cfg(test)]
mod tests {
    use super::{ARROW, EGG, SNOWBALL, dispense_item};
    use crate::item::ItemStack;
    use crate::save::NoSave;
    use crate::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
    use crate::world::events::WorldEvent;
    use crate::world::provider::{Dimension, WorldProvider};
    use crate::world::tile_entity::TileEntityKindData;
    use crate::world::{World, WorldEnvironment};

    fn world() -> World {
        let mut w = World::new(
            Box::new(NoSave),
            "harness",
            WorldProvider::new(Dimension::Surface),
            5,
            ChunkGenerator::Empty(EmptyChunkProvider),
            WorldEnvironment {
                rand_seed: 0,
                field_9437_g: 0,
                lock_timestamp: 0,
            },
        );
        w.make_deterministic(5);
        w.get_chunk_from_chunk_coords(0, 0);
        w
    }

    /// The projectile branches (BlockDispenser.java:106-121) spawn the
    /// projectile at the face (0.6 out, block centre height) with its heading
    /// `(var9, 0.1, var10)` at speed 1.1 and spread 6 drawn on its own `rand`
    /// (seeded `mix(seed, id)` at construction), take one item from the slot
    /// (the tile entity's `Random`, not `World.rand`), and keep the aux 1002
    /// and the smoke aux 2000 with `var9 + 1 + (var10 + 1) * 3`.
    #[test]
    fn projectiles_spawn_with_their_heading_and_keep_the_effects() {
        use crate::entity::EntityKindData;
        use mc_core::java_random::JavaRandom;
        use mc_core::math::sqrt_double;
        for (item, class) in [
            (ARROW, "EntityArrow"),
            (EGG, "EntityEgg"),
            (SNOWBALL, "EntitySnowball"),
        ] {
            let mut w = world();
            // Meta 3 faces +z: var9 = 0, var10 = 1.
            w.set_block_and_metadata_with_notify(1, 64, 1, 23, 3);
            let r = w.get_block_tile_entity(1, 64, 1).unwrap();
            let TileEntityKindData::Dispenser(d) = &mut w.tile_entities.get_mut(&r).unwrap().kind
            else {
                panic!()
            };
            d.dispenser_contents[4] = Some(ItemStack::new(item, 2, 0));
            let world_rand = w.rand.clone();
            let entities = w.loaded_entity_list.len();
            let id = w.next_entity_id;
            w.events.drain();
            dispense_item(&mut w, 1, 64, 1, super::dispatch::RandomSource::WorldRand);
            assert!(w.unimplemented_hits.borrow().is_empty(), "{class}");
            assert_eq!(w.rand, world_rand, "{class}: no World.rand draw");
            assert_eq!(w.loaded_entity_list.len(), entities + 1, "{class}");
            let e = w.entity(id).unwrap();
            assert_eq!(e.kind.java_class(), class);
            let pos = (
                1.0 + 0.0 * 0.6 + 0.5,
                1.0 * 0.0 + 64.0 + 0.5,
                1.0 + 1.0 * 0.6 + 0.5,
            );
            assert_eq!((e.base.pos_x, e.base.pos_y, e.base.pos_z), pos, "{class}");
            // src: EntityArrow.java:50-63 with (0, 0.1, 1), 1.1F, 6.0F
            let mut rand = JavaRandom::new(w.entity_rand_seed_for(id));
            let var9 = sqrt_double(0.0 + 0.10000000149011612 * 0.10000000149011612 + 1.0) as f64;
            let mut m = [0.0 / var9, 0.10000000149011612 / var9, 1.0 / var9];
            for v in &mut m {
                *v += rand.next_gaussian() * 0.007499999832361937 * 6.0;
                *v *= 1.100000023841858;
            }
            assert_eq!(
                [e.base.motion_x, e.base.motion_y, e.base.motion_z],
                m,
                "{class}"
            );
            if let EntityKindData::Arrow(a) = &e.kind {
                assert!(a.does_arrow_belong_to_player);
            }
            let TileEntityKindData::Dispenser(d) = &w.tile_entities[&r].kind else {
                panic!()
            };
            assert_eq!(d.dispenser_contents[4], Some(ItemStack::new(item, 1, 0)));
            let aux: Vec<(i32, i32)> = w
                .events
                .drain()
                .into_iter()
                .filter_map(|e| match e {
                    WorldEvent::PlayAuxSfx { kind, data, .. } => Some((kind, data)),
                    _ => None,
                })
                .collect();
            assert_eq!(aux, vec![(1002, 0), (2000, 7)], "{class}");
        }
    }
}
