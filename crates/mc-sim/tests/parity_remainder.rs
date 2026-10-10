//! WP-4.7, the small parity remainder (docs/DEFERRED_REVIEW.md §3.6,
//! docs/PARITY_CHECKLIST.md P4), where the oracle cannot reach it:
//!
//! - `EntityPlayer.onUpdate`'s `craftingInventory.isUsableByPlayer` close
//!   (EntityPlayer.java:80-84) inside the player's update: the oracle's
//!   `HarnessPlayer` never opens a GUI, so its `craftingInventory` is
//!   always the `ContainerPlayer`;
//! - the dead player the world keeps for `playerEntities` (the oracle side is
//!   the `tick/dead-player-save-reload` fixture): `Minecraft.respawn`'s
//!   `World.setEntityDead` on it, `joinEntityInSurroundings`, the queries;
//! - `EntityFireball.getCollisionBorderSize` (client picking only).

use mc_sim::entity::fireball::new_entity_fireball_empty;
use mc_sim::entity::living::EntityView;
use mc_sim::entity::pig::new_entity_pig;
use mc_sim::entity::player::{self, PlayerSpData, new_entity_player_sp};
use mc_sim::entity::{Entity, EntityKindData, dispatch};
use mc_sim::inventory::InventoryRef;
use mc_sim::inventory::container::chest::new_container_chest;
use mc_sim::inventory::container::workbench::new_container_workbench;
use mc_sim::item::ItemStack;
use mc_sim::save::NoSave;
use mc_sim::world::World;
use mc_sim::world::WorldEnvironment;
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::client_requests::ClientRequest;
use mc_sim::world::provider::{Dimension, WorldProvider};

const STONE: i32 = 1;
const COBBLESTONE: i32 = 4;
const PLANKS: i32 = 5;
const CHEST: i32 = 54;
const WORKBENCH: i32 = 58;

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
    for dz in -3..=3 {
        for dx in -3..=3 {
            w.get_chunk_from_chunk_coords(dx, dz);
        }
    }
    for x in -16..16 {
        for z in -16..16 {
            w.set_block(x, 60, z, STONE);
        }
    }
    w
}

fn sp_mut(e: &mut Entity) -> &mut PlayerSpData {
    match &mut e.kind {
        EntityKindData::PlayerSp(_, _, s) => s,
        _ => unreachable!(),
    }
}

/// A player at `x, 61, z` (`client_additions` as given: the client's
/// `EntityPlayerSP`, or the harness's `EntityPlayer`), joined; its id.
fn join_player(w: &mut World, x: f64, z: f64, client: bool) -> i32 {
    let mut p = new_entity_player_sp(w);
    p.base.set_location_and_angles(x, 61.0, z, 0.0, 0.0);
    p.kind.player_mut().unwrap().username = Some("Player".into());
    sp_mut(&mut p).client_additions = client;
    let id = p.base.entity_id;
    assert!(w.spawn_entity_in_world(p));
    id
}

fn cursor(w: &World, pid: i32) -> Option<ItemStack> {
    w.entity_object(pid)
        .unwrap()
        .kind
        .player()
        .unwrap()
        .inventory
        .item_stack
}

fn set_cursor(w: &mut World, pid: i32, s: Option<ItemStack>) {
    w.entity_mut(pid)
        .unwrap()
        .kind
        .player_mut()
        .unwrap()
        .inventory
        .item_stack = s;
}

/// The `EntityItem`s in the list, as (index, stack, ticksExisted).
fn items(w: &World) -> Vec<(usize, ItemStack, i32)> {
    w.loaded_entity_list
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            let e = s.entity.as_deref()?;
            match &e.kind {
                EntityKindData::Item(d) => Some((i, d.item?, e.base.ticks_existed)),
                _ => None,
            }
        })
        .collect()
}

/// Whether the player asked the shell to close the screen
/// (`mc.displayGuiScreen(null)`); drains the requests.
fn closed(w: &mut World) -> bool {
    w.client_requests
        .drain()
        .contains(&ClientRequest::CloseScreen)
}

fn open(w: &mut World, pid: i32, c: mc_sim::inventory::container::Container) {
    let p = w.entity_mut(pid).unwrap();
    assert!(player::set_crafting_inventory(p, Some(c)).is_none());
}

fn open_chest(w: &mut World, pid: i32, x: i32, y: i32, z: i32) {
    w.set_block(x, y, z, CHEST);
    let te = w
        .get_block_tile_entity(x, y, z)
        .expect("a chest tile entity");
    let c = new_container_chest(w, InventoryRef::TileEntity(te));
    open(w, pid, c);
}

/// EntityPlayer.java:80-84 with the client's player: the chest is usable
/// while the player is within 8 blocks; once it walks away the window
/// closes inside the player's update in that same `updateEntities`: the
/// container leaves the player (`craftingInventory = inventorySlots`), the
/// stack on the cursor drops (`GuiContainer.onGuiClosed`, run at once by
/// `mc.displayGuiScreen(null)`), and the dropped `EntityItem`, appended
/// behind the player, is updated later in the same pass (vanilla: the
/// `for` loop reads `size()` each step). The shell gets `closeScreen`.
#[test]
fn a_chest_out_of_reach_closes_in_the_players_update() {
    let mut w = world();
    let pid = join_player(&mut w, 0.5, 0.5, true);
    open_chest(&mut w, pid, 3, 61, 0);
    set_cursor(&mut w, pid, Some(ItemStack::new(COBBLESTONE, 5, 0)));
    w.update_entities();
    assert!(player::crafting_inventory(w.entity(pid).unwrap()).is_some());
    assert!(!closed(&mut w));
    assert!(items(&w).is_empty());

    w.entity_mut(pid)
        .unwrap()
        .base
        .set_position(12.5, 61.0, 0.5);
    w.update_entities();
    assert!(player::crafting_inventory(w.entity(pid).unwrap()).is_none());
    assert_eq!(cursor(&w, pid), None);
    let it = items(&w);
    assert_eq!(it.len(), 1);
    assert_eq!(it[0].1, ItemStack::new(COBBLESTONE, 5, 0));
    let player_idx = w.entity_index(pid).unwrap();
    assert!(it[0].0 > player_idx);
    assert_eq!(
        it[0].2, 1,
        "the drop is updated in the pass that closed the window"
    );
    assert!(closed(&mut w));
}

/// `EntityPlayer.closeScreen` (EntityPlayer.java:131-133) of the oracle's
/// `HarnessPlayer` only restores `inventorySlots`: no drop, no request.
#[test]
fn the_harness_player_closes_without_drops() {
    let mut w = world();
    let pid = join_player(&mut w, 0.5, 0.5, false);
    open_chest(&mut w, pid, 3, 61, 0);
    set_cursor(&mut w, pid, Some(ItemStack::new(COBBLESTONE, 5, 0)));
    w.entity_mut(pid)
        .unwrap()
        .base
        .set_position(12.5, 61.0, 0.5);
    w.update_entities();
    assert!(player::crafting_inventory(w.entity(pid).unwrap()).is_none());
    assert_eq!(cursor(&w, pid), Some(ItemStack::new(COBBLESTONE, 5, 0)));
    assert!(items(&w).is_empty());
    assert!(!closed(&mut w));
}

/// `ContainerWorkbench.isUsableByPlayer` (ContainerWorkbench.java:56-62):
/// the workbench broken under an open window closes it in the player's next
/// update, dropping the 3×3 grid (ContainerWorkbench.java:43-54); the
/// player's own `ContainerPlayer` (GuiInventory) is always usable
/// (ContainerPlayer.java:61-63).
#[test]
fn a_broken_workbench_closes_and_drops_its_grid() {
    let mut w = world();
    let pid = join_player(&mut w, 0.5, 0.5, true);
    w.set_block(2, 61, 0, WORKBENCH);
    let mut c = new_container_workbench(2, 61, 0);
    set_cursor(&mut w, pid, Some(ItemStack::new(PLANKS, 3, 0)));
    w.with_entity_mut(pid, |w, p| {
        // A right click puts one plank into grid slot 1, then slot 2.
        c.slot_click(w, p, 1, 1, false);
        c.slot_click(w, p, 2, 1, false);
    })
    .unwrap();
    open(&mut w, pid, c);
    w.update_entities();
    assert!(items(&w).is_empty());
    w.set_block(2, 61, 0, 0);
    w.update_entities();
    assert!(player::crafting_inventory(w.entity(pid).unwrap()).is_none());
    let stacks: Vec<ItemStack> = items(&w).into_iter().map(|i| i.1).collect();
    // The cursor first, then the grid in slot order.
    assert_eq!(
        stacks,
        vec![
            ItemStack::new(PLANKS, 1, 0),
            ItemStack::new(PLANKS, 1, 0),
            ItemStack::new(PLANKS, 1, 0)
        ]
    );

    let mut w = world();
    let pid = join_player(&mut w, 0.5, 0.5, true);
    let c = mc_sim::inventory::container::player::new_container_player(true);
    open(&mut w, pid, c);
    w.entity_mut(pid)
        .unwrap()
        .base
        .set_position(-12.5, 61.0, 0.5);
    w.update_entities();
    assert!(player::crafting_inventory(w.entity(pid).unwrap()).is_some());
    assert!(!closed(&mut w));
}

/// `EntityPlayer.setEntityDead` (EntityPlayer.java:524-531) closes
/// `craftingInventory` too: an open workbench's grid drops.
#[test]
fn set_entity_dead_closes_the_open_container() {
    let mut w = world();
    let pid = join_player(&mut w, 0.5, 0.5, true);
    w.set_block(2, 61, 0, WORKBENCH);
    let mut c = new_container_workbench(2, 61, 0);
    set_cursor(&mut w, pid, Some(ItemStack::new(PLANKS, 1, 0)));
    w.with_entity_mut(pid, |w, p| {
        c.slot_click(w, p, 5, 0, false);
    })
    .unwrap();
    assert_eq!(cursor(&w, pid), None);
    open(&mut w, pid, c);
    w.with_entity_mut(pid, |w, p| dispatch::set_entity_dead(p, w))
        .unwrap();
    assert_eq!(
        items(&w).into_iter().map(|i| i.1).collect::<Vec<_>>(),
        vec![ItemStack::new(PLANKS, 1, 0)]
    );
    // It stays the player's craftingInventory, as in Java.
    assert!(player::crafting_inventory(w.entity(pid).unwrap()).is_some());
}

/// Kills the player and runs `updateEntities` until it has left the list
/// (`deathTime > 20`, EntityLiving.java:154-158); the number of passes.
fn die_and_leave(w: &mut World, pid: i32) -> i32 {
    w.entity_mut(pid).unwrap().kind.living_mut().unwrap().health = 0;
    let mut n = 0;
    while w.entity_index(pid).is_some() {
        w.update_entities();
        n += 1;
        assert!(n < 40);
    }
    n
}

/// World.java:934-945: a player that dies (`Entity.setEntityDead`) leaves
/// `loadedEntityList` but not `playerEntities`; the world keeps the object
/// (World::entity_object), which `getClosestPlayer` and
/// `getPlayerEntityByName` go on finding. `joinEntityInSurroundings`
/// (World.java:2294-2309) appends it again, unchanged and without a chunk,
/// and the next `updateEntities` removes it again. `Minecraft.respawn`'s
/// `World.setEntityDead` on it takes it out of `playerEntities`, after
/// which the world drops it.
#[test]
fn the_dead_player_stays_in_player_entities() {
    let mut w = world();
    let pid = join_player(&mut w, 0.5, 0.5, true);
    assert_eq!(die_and_leave(&mut w, pid), 21);
    assert_eq!(w.player_entities, vec![pid]);
    let dead = w.entity_object(pid).expect("kept");
    assert!(dead.base.is_dead);
    assert_eq!(dead.kind.living().unwrap().death_time, 21);
    assert_eq!(w.get_closest_player(3.0, 61.0, 3.0, 8.0), Some(pid));
    assert_eq!(w.get_player_entity_by_name("Player"), Some(pid));
    // Kept across passes (nothing else names it).
    w.update_entities();
    assert!(w.removed_entity(pid).is_some());

    let unimpl = w.unimplemented_hits.borrow().len();
    w.join_entity_in_surroundings(pid);
    assert_eq!(w.loaded_entity_list.last().unwrap().entity_id, pid);
    assert!(w.removed_entity(pid).is_none());
    assert_eq!(w.unimplemented_hits.borrow().len(), unimpl);
    w.update_entities();
    assert!(w.entity_index(pid).is_none());
    // Not updated while back in the list: deathTime is unchanged.
    assert_eq!(
        w.removed_entity(pid)
            .unwrap()
            .kind
            .living()
            .unwrap()
            .death_time,
        21
    );

    w.with_entity_object_mut(pid, |w, p| w.set_entity_dead(p))
        .unwrap();
    assert!(w.player_entities.is_empty());
    assert_eq!(w.get_closest_player(3.0, 61.0, 3.0, 8.0), None);
    w.update_entities();
    assert!(w.entity_object(pid).is_none());
}

/// EntityLiving.java:704-711: `faceEntity(currentTarget)` runs before the
/// `isDead` test, so a removed target is still a view (its last position,
/// `isDead`), not a recorded gap.
#[test]
fn a_removed_look_target_is_still_viewed() {
    let mut w = world();
    let pid = join_player(&mut w, 0.5, 0.5, true);
    let mut pig = new_entity_pig(&mut w);
    pig.base.set_location_and_angles(4.5, 61.0, 0.5, 0.0, 0.0);
    pig.kind.living_mut().unwrap().current_target = Some(pid);
    assert!(w.spawn_entity_in_world(pig));
    die_and_leave(&mut w, pid);
    let v = EntityView::of(&w, pid).expect("the removed player is viewed");
    assert!(v.is_dead && v.is_living);
    assert_eq!(v.pos_x, 0.5);
    assert!(w.unimplemented_hits.borrow().is_empty());
}

/// EntityFireball.java:198-200: the pick border is 1 (Entity's 0.1 for
/// the rest, Entity.java:1030-1032), without a record.
#[test]
fn a_fireballs_collision_border_is_one() {
    let mut w = world();
    let f = new_entity_fireball_empty(&mut w);
    assert_eq!(dispatch::get_collision_border_size(&f, &w), 1.0);
    let p = new_entity_pig(&mut w);
    assert_eq!(dispatch::get_collision_border_size(&p, &w), 0.1);
    assert!(w.unimplemented_hits.borrow().is_empty());
}
