//! WP-4.8: the behaviour-neutral take-out/removed-entity changes of
//! docs/DEFERRED_REVIEW.md §3.1 E (Gate 2 R5): a class query skips a
//! taken-out entity of another class as Java's `isAssignableFrom` does, with
//! no record; `EntityPigZombie.attackEntityFrom`'s anger query goes through
//! it; and `updateEntities` reads a rider's removed mount through
//! `World::entity_object` (the object Java still holds) instead of recording
//! a gap. A boat or minecart broken by its own rider puts it off on top
//! (Entity.java:992-995) whether the rider is in its slot (its own arrow
//! passes a copy of it) or in the caller's hand (its own click).

use mc_sim::entity::player::new_entity_player_sp;
use mc_sim::entity::{Entity, base, boat, dispatch, minecart, pig, pig_zombie, wolf};
use mc_sim::player_controller;
use mc_sim::save::NoSave;
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::{World, WorldEnvironment};

use mc_core::Aabb;

const ENV: WorldEnvironment = WorldEnvironment {
    rand_seed: 0,
    field_9437_g: 0,
    lock_timestamp: 0,
};

fn world() -> World {
    let mut w = World::new(
        Box::new(NoSave),
        "harness",
        WorldProvider::new(Dimension::Surface),
        7,
        ChunkGenerator::Empty(EmptyChunkProvider),
        ENV,
    );
    w.make_deterministic(7);
    for dz in -2..=2 {
        for dx in -2..=2 {
            w.get_chunk_from_chunk_coords(dx, dz);
        }
    }
    for x in -8..8 {
        for z in -8..8 {
            w.set_block(x, 60, z, 1);
        }
    }
    w
}

fn join(w: &mut World, mut e: Entity, x: f64, z: f64) -> i32 {
    e.base.set_location_and_angles(x, 61.0, z, 0.0, 0.0);
    let id = e.base.entity_id;
    assert!(w.spawn_entity_in_world(e));
    id
}

fn hits(w: &World) -> Vec<(&'static str, u64)> {
    w.unimplemented_hits
        .borrow()
        .iter()
        .map(|(k, v)| (*k, *v))
        .collect()
}

#[test]
fn class_query_skips_a_taken_out_entity_of_another_class() {
    let mut w = world();
    let p = pig::new_entity_pig(&mut w);
    let pid = join(&mut w, p, 0.5, 0.5);
    let f = wolf::new_entity_wolf(&mut w);
    let fid = join(&mut w, f, 2.5, 0.5);
    let bb = Aabb::new(-4.0, 58.0, -4.0, 4.0, 64.0, 4.0);
    // The pig is out of its slot (as the attacker or victim of a nested call).
    let (wolves, pigs, animals) = w
        .with_entity_mut(pid, |w, _| {
            (
                w.get_entities_within_aabb("EntityWolf", &bb, None),
                w.get_entities_within_aabb("EntityPig", &bb, None),
                w.get_entities_within_aabb("EntityAnimal", &bb, None),
            )
        })
        .unwrap();
    assert_eq!(wolves, [fid]);
    // Java would have found the pig in both: the port cannot read its box, so
    // it leaves it out and records it, once per query.
    assert!(pigs.is_empty());
    assert_eq!(animals, [fid]);
    assert_eq!(
        hits(&w),
        [("World.getEntitiesWithinAABB over a taken-out entity", 2)]
    );
}

#[test]
fn pig_zombie_anger_query_tests_the_class_of_the_attacker() {
    let mut w = world();
    let pl = new_entity_player_sp(&mut w);
    let plid = join(&mut w, pl, -2.5, 0.5);
    let a = pig_zombie::new_entity_pig_zombie(&mut w);
    let aid = join(&mut w, a, 0.5, 0.5);
    let b = pig_zombie::new_entity_pig_zombie(&mut w);
    let bid = join(&mut w, b, 2.5, 2.5);
    // The player hits pigman A: the player is out of its slot (the client's
    // click), A is out of its slot (the target), B is in the world.
    w.with_entity_mut(plid, |w, player| {
        w.with_entity_mut(aid, |w, target| {
            dispatch::attack_entity_from(target, w, Some(&*player), 1)
        })
    })
    .unwrap()
    .unwrap();
    for id in [aid, bid] {
        let e = w.entity(id).unwrap();
        let mc_sim::entity::EntityKindData::PigZombie(.., d) = &e.kind else {
            unreachable!()
        };
        assert!(d.anger_level >= 400, "pigman {id} angry");
    }
    // The player (not an EntityPigZombie) is skipped without a record.
    assert_eq!(hits(&w), []);
}

#[test]
fn a_rider_reads_its_removed_mount_as_java_holds_it() {
    let mut w = world();
    let m = pig::new_entity_pig(&mut w);
    let mid = join(&mut w, m, 0.5, 0.5);
    // A pig rides the pig (a mob would leave at difficulty 0, EntityMob.java:20-24).
    let r = pig::new_entity_pig(&mut w);
    let rid = join(&mut w, r, 0.5, 0.5);
    w.with_entity_mut(rid, |w, r| base::mount_entity_in_world(r, w, mid))
        .unwrap();
    assert_eq!(w.entity(rid).unwrap().base.riding_entity, Some(mid));
    // The mount dies (Entity.setEntityDead, no dismount): this pass removes
    // it before the rider's turn, and the rider reads the removed object's
    // isDead (World.java:1210-1217), as Java reads ridingEntity.isDead.
    w.entity_mut(mid).unwrap().base.is_dead = true;
    w.update_entities();
    assert!(w.entity_index(mid).is_none());
    assert_eq!(w.entity(rid).unwrap().base.riding_entity, None);
    assert_eq!(hits(&w), []);
    // Nothing names the dead pig any more: it is not kept.
    assert!(w.removed_entity(mid).is_none());
}

/// The player rides `mount` (spawned at (0.5, 61, 0.5)); returns the player id,
/// the mount id and where Java's dismount branch puts the player:
/// `setLocationAndAngles(mount.posX, mount.boundingBox.minY + mount.height,
/// mount.posZ)` (Entity.java:992-995), plus the player's `yOffset`.
fn ride(w: &mut World, mount: Entity) -> (i32, i32, f64) {
    let mid = join(w, mount, 0.5, 0.5);
    let pl = new_entity_player_sp(w);
    let plid = join(w, pl, 0.5, 2.5);
    w.with_entity_mut(plid, |w, p| base::mount_entity_in_world(p, w, mid))
        .unwrap();
    assert_eq!(w.entity(plid).unwrap().base.riding_entity, Some(mid));
    let m = w.entity(mid).unwrap();
    let p = w.entity(plid).unwrap();
    let y = m.base.bounding_box.min_y + m.base.height as f64 + p.base.y_offset as f64;
    (plid, mid, y)
}

fn assert_put_off_on_top(w: &World, plid: i32, mid: i32, y: f64) {
    let p = w.entity(plid).unwrap();
    assert_eq!(p.base.riding_entity, None);
    assert_eq!(p.base.pos_y, y);
    let m = w.entity(mid).unwrap();
    assert!(m.base.is_dead);
    assert_eq!(m.base.ridden_by_entity, None);
    assert_eq!(hits(w), []);
}

/// The rider's own arrow: `attackEntityFrom(this.owner, 4)` (EntityArrow.java:163),
/// the shooter passed as a copy while it stays in its slot. Two hits pass 40.
fn shoot_twice(w: &mut World, plid: i32, mid: i32) {
    let shooter = w.entity(plid).unwrap().clone();
    for _ in 0..2 {
        assert!(
            w.with_entity_mut(mid, |w, m| dispatch::attack_entity_from(
                m,
                w,
                Some(&shooter),
                4
            ))
            .unwrap()
        );
    }
}

/// The rider's own click: `PlayerController.attackEntity` with the player and
/// the mount both out of their slots, until the mount dies.
fn punch_until_broken(w: &mut World, plid: i32, mid: i32) {
    for _ in 0..10 {
        w.with_entity_mut(plid, |w, p| {
            w.with_entity_mut(mid, |w, m| player_controller::attack_entity(w, p, m))
        })
        .unwrap()
        .unwrap();
        if w.entity(mid).unwrap().base.is_dead {
            return;
        }
    }
    panic!("the mount survived ten punches");
}

#[test]
fn a_boat_shot_by_its_rider_puts_it_off_on_top() {
    let mut w = world();
    let b = boat::new_entity_boat(&mut w, 0.5, 61.0, 0.5);
    let (plid, mid, y) = ride(&mut w, b);
    shoot_twice(&mut w, plid, mid);
    assert_put_off_on_top(&w, plid, mid, y);
}

#[test]
fn a_minecart_shot_by_its_rider_puts_it_off_on_top() {
    let mut w = world();
    let c = minecart::new_entity_minecart(&mut w, 0.5, 61.0, 0.5, 0);
    let (plid, mid, y) = ride(&mut w, c);
    shoot_twice(&mut w, plid, mid);
    assert_put_off_on_top(&w, plid, mid, y);
}

#[test]
fn a_boat_punched_by_its_rider_puts_it_off_on_top() {
    let mut w = world();
    let b = boat::new_entity_boat(&mut w, 0.5, 61.0, 0.5);
    let (plid, mid, y) = ride(&mut w, b);
    punch_until_broken(&mut w, plid, mid);
    assert_put_off_on_top(&w, plid, mid, y);
}

#[test]
fn a_minecart_punched_by_its_rider_puts_it_off_on_top() {
    let mut w = world();
    let c = minecart::new_entity_minecart(&mut w, 0.5, 61.0, 0.5, 0);
    let (plid, mid, y) = ride(&mut w, c);
    punch_until_broken(&mut w, plid, mid);
    assert_put_off_on_top(&w, plid, mid, y);
}
