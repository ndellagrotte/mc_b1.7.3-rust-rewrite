//! WP-4.6: the simulation gaps Gate 2 and Gate 3 left open that vanilla
//! singleplayer reaches, beside their oracle fixtures (`tick/cake-heal-window`,
//! `entity/pig-lightning-pigman`, `entity/wolf-defends-owner`,
//! `entity/minecart-chest-container`): the cake's heal through
//! `EntityLiving.heal` (BlockCake.java:76-88), `EntityPig.onStruckByLightning`
//! (EntityPig.java:62-69), the tamed-wolf branch of `EntityPlayer.alertWolves`
//! (EntityPlayer.java:402-437) and the chest minecart as the `IInventory` of a
//! `ContainerChest` (EntityMinecart.java:716-761, :811-817). The kill hooks of
//! `EntityLiving.onDeath` are in `stats.rs` (`a_kill_*`).
//!
//! The review fix: an entity removed from `loadedEntityList` that a field
//! still names (a dead skeleton and its arrow in flight) stays the object
//! Java passes on (`World::removed_entity`, fixture
//! `entity/skeleton-arrow-after-death`).

use mc_sim::block::dispatch as bd;
use mc_sim::entity::arrow::{self, new_entity_arrow_shot};
use mc_sim::entity::creature::creature;
use mc_sim::entity::lightning_bolt::new_entity_lightning_bolt;
use mc_sim::entity::minecart::new_entity_minecart;
use mc_sim::entity::player::{self, new_entity_player_sp};
use mc_sim::entity::skeleton::new_entity_skeleton;
use mc_sim::entity::{Entity, EntityKindData, creeper, dispatch, pig, wolf};
use mc_sim::inventory::InventoryRef;
use mc_sim::inventory::container::chest::new_container_chest;
use mc_sim::item::ItemStack;
use mc_sim::save::NoSave;
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::{World, WorldEnvironment};

const STONE: i32 = 1;
const PLANKS: i32 = 5;
const CAKE: i32 = 92;

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
    for dz in -2..=2 {
        for dx in -2..=2 {
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

/// A player named "Player" at `x, 61, z`, joined; its id.
fn join_player(w: &mut World, x: f64, z: f64) -> i32 {
    let mut p = new_entity_player_sp(w);
    p.base.set_location_and_angles(x, 61.0, z, 0.0, 0.0);
    p.kind.player_mut().unwrap().username = Some("Player".into());
    let id = p.base.entity_id;
    assert!(w.spawn_entity_in_world(p));
    id
}

/// A wolf at `x, 61, z`, joined, tamed by `owner` (if any) and sitting as
/// given; its id.
fn join_wolf(w: &mut World, x: f64, z: f64, owner: Option<&str>, sitting: bool) -> i32 {
    let mut e = wolf::new_entity_wolf(w);
    e.base.set_location_and_angles(x, 61.0, z, 0.0, 0.0);
    if let Some(o) = owner {
        wolf::set_wolf_tamed(&mut e, true);
        wolf::set_wolf_owner(&mut e, Some(o));
    }
    wolf::set_wolf_sitting(&mut e, sitting);
    let id = e.base.entity_id;
    assert!(w.spawn_entity_in_world(e));
    id
}

fn target_of(w: &World, id: i32) -> Option<i32> {
    creature(&w.entity(id).unwrap().kind)
        .unwrap()
        .player_to_attack
}

fn joined_pig(w: &mut World, x: f64, z: f64) -> Entity {
    let mut e = pig::new_entity_pig(w);
    e.base.set_location_and_angles(x, 61.0, z, 0.0, 0.0);
    e
}

/// BlockCake.java:76-88 through EntityLiving.java:296-305: a hurt player
/// heals 3 and its resistance window restarts at `heartsHalvesLife / 2`;
/// at full health nothing happens.
#[test]
fn cake_heals_through_entity_living_heal() {
    let mut w = world();
    w.set_block(2, 61, 2, CAKE);
    let mut p = new_entity_player_sp(&mut w);
    p.base.set_location_and_angles(0.5, 61.0, 0.5, 0.0, 0.0);
    p.kind.living_mut().unwrap().health = 14;
    p.base.hearts_life = 17;
    assert!(bd::block_activated(&mut w, CAKE, 2, 61, 2, &mut p));
    assert_eq!(p.kind.living().unwrap().health, 17);
    assert_eq!(p.base.hearts_life, 10);
    assert_eq!(w.get_block_metadata(2, 61, 2), 1);
    p.kind.living_mut().unwrap().health = 20;
    p.base.hearts_life = 3;
    assert!(bd::block_activated(&mut w, CAKE, 2, 61, 2, &mut p));
    assert_eq!(p.base.hearts_life, 3);
    assert_eq!(w.get_block_metadata(2, 61, 2), 1);
    assert!(w.unimplemented_hits.borrow().is_empty());
}

/// EntityPig.java:62-69: a struck pig dies and a zombie pigman with the
/// next entity id joins at its position and angles; no fire damage.
#[test]
fn a_struck_pig_becomes_a_zombie_pigman() {
    let mut w = world();
    let mut e = joined_pig(&mut w, 3.25, 4.75);
    e.base.set_rotation(37.5, -10.0);
    let pid = e.base.entity_id;
    assert!(w.spawn_entity_in_world(e));
    let bolt = new_entity_lightning_bolt(&mut w, 3.0, 61.0, 4.0);
    let next = bolt.base.entity_id + 1;
    w.with_entity_mut(pid, |w, p| dispatch::on_struck_by_lightning(p, w, &bolt))
        .unwrap();
    let pig = w.entity(pid).unwrap();
    assert!(pig.base.is_dead);
    assert_eq!(pig.base.fire, 0);
    assert_eq!(pig.kind.living().unwrap().health, 10);
    let z = w.entity(next).expect("the pigman joined");
    assert!(matches!(z.kind, EntityKindData::PigZombie(..)));
    assert_eq!(
        (
            z.base.pos_x,
            z.base.pos_z,
            z.base.rotation_yaw,
            z.base.rotation_pitch
        ),
        (3.25, 4.75, 37.5, -10.0)
    );
    // setLocationAndAngles adds the pigman's yOffset (0) to the pig's posY.
    assert_eq!(z.base.pos_y, w.entity(pid).unwrap().base.pos_y);
    assert_eq!(
        w.loaded_entity_list.last().map(|s| s.entity_id),
        Some(next),
        "appended behind the pig"
    );
    assert!(w.unimplemented_hits.borrow().is_empty());
}

/// EntityPlayer.java:402-437: the player's tamed wolves without a target
/// stand up and target the attacker; a sitting one stays put on the
/// player's own hit (`skipSitting`); other owners' wolves, untamed wolves
/// and wolves with a target are left alone; a creeper, a ghast, a player
/// and the player's own tamed wolf alert nobody.
#[test]
fn alert_wolves_sends_the_players_tamed_wolves() {
    let mut w = world();
    let pid = join_player(&mut w, 0.5, 0.5);
    let sitting = join_wolf(&mut w, 3.5, 0.5, Some("Player"), true);
    let standing = join_wolf(&mut w, 4.5, 0.5, Some("Player"), false);
    let other = join_wolf(&mut w, 5.5, 0.5, Some("Notch"), false);
    let wild = join_wolf(&mut w, 6.5, 0.5, None, false);
    let far = join_wolf(&mut w, 0.5, 15.5, Some("Player"), false);
    let pig = joined_pig(&mut w, 2.5, 2.5);
    let pig_id = pig.base.entity_id;
    assert!(w.spawn_entity_in_world(pig));
    let pig = w.entity(pig_id).unwrap().clone();
    let p = w.entity(pid).unwrap().clone();

    // The player's own hit: sitting wolves stay.
    player::alert_wolves(&p, &mut w, &pig, true);
    assert_eq!(target_of(&w, sitting), None);
    assert!(wolf::is_wolf_sitting(w.entity(sitting).unwrap()));
    assert_eq!(target_of(&w, standing), Some(pig_id));
    assert_eq!(target_of(&w, other), None);
    assert_eq!(target_of(&w, wild), None);
    // Within 16 + 1 blocks horizontally of the player's corner: z 15.5 is in.
    assert_eq!(target_of(&w, far), Some(pig_id));

    // Hit by the pig: the sitting wolf stands up; a wolf with a target keeps it.
    let pig2 = joined_pig(&mut w, 1.5, 2.5);
    let pig2_id = pig2.base.entity_id;
    assert!(w.spawn_entity_in_world(pig2));
    let pig2 = w.entity(pig2_id).unwrap().clone();
    player::alert_wolves(&p, &mut w, &pig2, false);
    assert_eq!(target_of(&w, sitting), Some(pig2_id));
    assert!(!wolf::is_wolf_sitting(w.entity(sitting).unwrap()));
    assert_eq!(target_of(&w, standing), Some(pig_id));

    // No alert: a creeper, the player's own tamed wolf, a player.
    let fresh = join_wolf(&mut w, 2.5, 0.5, Some("Player"), true);
    let c = creeper::new_entity_creeper(&mut w);
    player::alert_wolves(&p, &mut w, &c, false);
    let own = w.entity(standing).unwrap().clone();
    player::alert_wolves(&p, &mut w, &own, false);
    let other_player = new_entity_player_sp(&mut w);
    player::alert_wolves(&p, &mut w, &other_player, false);
    assert_eq!(target_of(&w, fresh), None);
    assert!(wolf::is_wolf_sitting(w.entity(fresh).unwrap()));
    // Another owner's tamed wolf as the attacker does alert.
    let notchs = w.entity(other).unwrap().clone();
    player::alert_wolves(&p, &mut w, &notchs, false);
    assert_eq!(target_of(&w, fresh), Some(other));
    assert!(
        w.unimplemented_hits.borrow().is_empty(),
        "{:?}",
        w.unimplemented_hits
    );
}

/// A player without a username: `username.equals` throws in Java; the port
/// records it and matches no owner.
#[test]
fn alert_wolves_without_a_username_is_recorded() {
    let mut w = world();
    let pid = join_player(&mut w, 0.5, 0.5);
    let tamed = join_wolf(&mut w, 3.5, 0.5, Some("Player"), false);
    let mut p = w.entity(pid).unwrap().clone();
    p.kind.player_mut().unwrap().username = None;
    let pig = joined_pig(&mut w, 2.5, 2.5);
    player::alert_wolves(&p, &mut w, &pig, false);
    assert_eq!(target_of(&w, tamed), None);
    assert_eq!(
        w.unimplemented_hits
            .borrow()
            .get("EntityPlayer.alertWolves with a null username (Java NullPointerException)"),
        Some(&1)
    );
}

/// EntityMinecart.java:716-761, :811-817 in ContainerChest.java:7-30: a
/// chest cart's window has 3 rows over the cart's `cargoItems`; clicks move
/// stacks in and out; the window is usable within 8 blocks of a live cart.
#[test]
fn a_chest_cart_is_a_container_inventory() {
    let mut w = world();
    let pid = join_player(&mut w, 0.5, 0.5);
    let cart = new_entity_minecart(&mut w, 2.5, 61.0, 0.5, 1);
    let cid = cart.base.entity_id;
    assert!(w.spawn_entity_in_world(cart));
    let inv = InventoryRef::Entity(cid);
    let mut c = new_container_chest(&w, inv.clone());
    assert_eq!(c.inv_get_size_inventory(&w, &inv), 27);
    assert_eq!(c.slots.len(), 27 + 36);
    assert_eq!(c.inv_get_inventory_stack_limit(&inv), 64);
    w.with_entity_mut(pid, |w, p| {
        p.kind.player_mut().unwrap().inventory.main_inventory[0] =
            Some(ItemStack::new(PLANKS, 7, 0));
        // Hotbar slot 0 is container slot 27 + 27 = 54.
        c.slot_click(w, p, 54, 0, false);
        c.slot_click(w, p, 4, 1, false);
        c.slot_click(w, p, 5, 0, false);
        assert!(c.is_usable_by_player(w, p));
        assert_eq!(
            c.inv_get_stack_in_slot(w, p, &inv, 4).map(|s| s.stack_size),
            Some(1)
        );
    })
    .unwrap();
    let EntityKindData::Minecart(d) = &w.entity(cid).unwrap().kind else {
        unreachable!()
    };
    assert_eq!(d.cargo_items[4], Some(ItemStack::new(PLANKS, 1, 0)));
    assert_eq!(d.cargo_items[5], Some(ItemStack::new(PLANKS, 6, 0)));
    // Out of reach: 9 blocks away.
    w.entity_mut(pid)
        .unwrap()
        .base
        .set_position(11.5, 61.0, 0.5);
    let p = w.entity(pid).unwrap().clone();
    assert!(!c.is_usable_by_player(&mut w, &p));
    // A dead cart cannot be used.
    w.entity_mut(pid).unwrap().base.set_position(0.5, 61.0, 0.5);
    let p = w.entity(pid).unwrap().clone();
    w.entity_mut(cid).unwrap().base.is_dead = true;
    assert!(!c.is_usable_by_player(&mut w, &p));
    assert!(w.unimplemented_hits.borrow().is_empty());
}

/// A cart no longer in the world (Java would keep using the object): every
/// call is recorded and the window cannot be used.
#[test]
fn a_chest_cart_gone_from_the_world_is_recorded() {
    let w = world();
    let inv = InventoryRef::Entity(99);
    let c = new_container_chest(&w, inv.clone());
    assert_eq!(c.slots.len(), 36);
    assert_eq!(
        w.unimplemented_hits
            .borrow()
            .get("Container over an entity no longer in loadedEntityList or out of its slot"),
        Some(&1)
    );
}

/// A gone cart's window is not usable (Java's dead cart, `isDead`,
/// EntityMinecart.java:811-817), and the check records nothing: a cart
/// leaves `loadedEntityList` only by dying.
#[test]
fn a_gone_chest_cart_is_not_usable_and_not_recorded() {
    let mut w = world();
    let pid = join_player(&mut w, 0.5, 0.5);
    let p = w.entity(pid).unwrap().clone();
    // Building the window over the gone cart reads its size: recorded.
    let c = new_container_chest(&w, InventoryRef::Entity(99));
    w.unimplemented_hits.borrow_mut().clear();
    assert!(!c.inv_can_interact_with(&mut w, &p, &InventoryRef::Entity(99)));
    assert!(
        w.unimplemented_hits.borrow().is_empty(),
        "{:?}",
        w.unimplemented_hits
    );
}

/// EntityArrow.java:163 with a shooter that has left `loadedEntityList`
/// (EntityLiving.java:154-158: removed 21 ticks after its death): Java's
/// `owner` field still holds the dead skeleton, so the player takes the
/// difficulty-scaled damage of an `EntityMob` (4 / 3 + 1 = 2 at difficulty 1,
/// EntityPlayer.java:366-378), is knocked away from the skeleton
/// (EntityLiving.java:337-345) and alerts its sitting tamed wolf, which
/// targets the dead skeleton (EntityPlayer.java:385-390, :402-437) and drops
/// it on its next AI update (EntityCreature.java:24-25). The removed
/// skeleton is kept while the arrow or the wolf names it, then pruned.
#[test]
fn a_removed_shooter_is_still_the_arrows_attacker() {
    let mut w = world();
    w.difficulty_setting = 1;
    let pid = join_player(&mut w, 0.5, 0.5);
    let wolf_id = join_wolf(&mut w, 3.5, 3.5, Some("Player"), true);
    let mut sk = new_entity_skeleton(&mut w);
    sk.base.set_location_and_angles(-5.5, 61.0, 0.5, 0.0, 0.0);
    let skel = sk.base.entity_id;
    assert!(w.spawn_entity_in_world(sk));
    let sk = w.entity(skel).unwrap().clone();
    // An arrow of the skeleton's, just short of the player, flying at it.
    let bb = w.entity(pid).unwrap().base.bounding_box;
    let y = (bb.min_y + bb.max_y) / 2.0;
    let mut a = new_entity_arrow_shot(&mut w, &sk);
    a.base.set_position(-0.5, y, 0.5);
    a.base.motion_x = 1.0;
    a.base.motion_y = 0.0;
    a.base.motion_z = 0.0;
    let aid = a.base.entity_id;
    assert!(w.spawn_entity_in_world(a));

    // The skeleton dies and leaves the list; its arrow keeps it.
    w.entity_mut(skel).unwrap().base.is_dead = true;
    w.update_entity_list();
    assert!(w.entity_index(skel).is_none());
    assert!(w.removed_entity(skel).is_some_and(|e| e.base.is_dead));

    w.with_entity_mut(aid, |w, a| arrow::on_update(a, w))
        .unwrap();
    assert!(w.entity(aid).unwrap().base.is_dead, "the arrow hit");
    let p = w.entity(pid).unwrap();
    assert_eq!(p.kind.living().unwrap().health, 18);
    assert!(p.base.motion_x > 0.0, "knocked away from the skeleton");
    assert!(!wolf::is_wolf_sitting(w.entity(wolf_id).unwrap()));
    assert_eq!(target_of(&w, wolf_id), Some(skel));
    // Only Gate 2 R5's AABB-query record (the arrow is out of its slot
    // during its own update): nothing about the shooter.
    assert!(
        w.unimplemented_hits
            .borrow()
            .keys()
            .all(|k| *k == "World.getEntitiesWithinAABB over a taken-out entity"),
        "{:?}",
        w.unimplemented_hits
    );

    // The arrow leaves; the wolf still names the skeleton until its AI
    // drops the dead target; then nothing names it.
    w.update_entities();
    assert!(w.entity_index(aid).is_none());
    assert_eq!(target_of(&w, wolf_id), None);
    w.update_entities();
    assert!(w.removed_entity(skel).is_none());
    assert!(
        !w.unimplemented_hits
            .borrow()
            .keys()
            .any(|k| k.contains("loadedEntityList")),
        "{:?}",
        w.unimplemented_hits
    );
}

/// A removed entity is pruned at once when nothing names it, and an
/// entity joining with a removed one's id replaces it (the respawned
/// player keeps the dead player's id, Minecraft.java:1451, :1468).
#[test]
fn removed_entities_are_kept_only_while_named() {
    let mut w = world();
    let pig = joined_pig(&mut w, 2.5, 2.5);
    let pig_id = pig.base.entity_id;
    assert!(w.spawn_entity_in_world(pig));
    w.entity_mut(pig_id).unwrap().base.is_dead = true;
    w.update_entity_list();
    assert!(w.entity_index(pig_id).is_none());
    assert!(w.removed_entity(pig_id).is_none());

    let mut sk = new_entity_skeleton(&mut w);
    sk.base.set_location_and_angles(-5.5, 61.0, 0.5, 0.0, 0.0);
    let skel = sk.base.entity_id;
    assert!(w.spawn_entity_in_world(sk));
    let sk = w.entity(skel).unwrap().clone();
    let mut a = new_entity_arrow_shot(&mut w, &sk);
    a.base.set_position(-5.5, 90.0, 0.5);
    assert!(w.spawn_entity_in_world(a));
    w.entity_mut(skel).unwrap().base.is_dead = true;
    w.update_entity_list();
    assert!(w.removed_entity(skel).is_some());
    assert!(w.entity_object(skel).is_some());
    assert!(w.is_entity_dead_or_gone(skel));
    // A new entity with the same id replaces the removed one.
    let mut again = new_entity_skeleton(&mut w);
    again.base.entity_id = skel;
    again
        .base
        .set_location_and_angles(-4.5, 61.0, 0.5, 0.0, 0.0);
    assert!(w.spawn_entity_in_world(again));
    assert!(w.removed_entity(skel).is_none());
    assert!(!w.entity_object(skel).unwrap().base.is_dead);
}
