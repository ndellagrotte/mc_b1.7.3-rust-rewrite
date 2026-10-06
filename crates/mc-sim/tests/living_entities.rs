//! WP-2.2: unit checks of `EntityLiving`, `EntityAnimal` and `EntityPig`
//! paths the tick fixtures (`tests/golden/entity/pig-*`) do not reach: the
//! pig's NBT round trip and `interact`, `getCanSpawnHere`, `heal`,
//! `knockBack`, `faceEntity`'s sign quirk, `handleHealthUpdate`, despawning.

use mc_core::Aabb;
use mc_sim::entity::living::{self, EntityView};
use mc_sim::entity::player::new_entity_player_sp;
use mc_sim::entity::{Entity, dispatch, nbt, pig};
use mc_sim::save::NoSave;
use mc_sim::save::entity_list::{FromNbt, create_entity_from_nbt};
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::{World, WorldEnvironment};

fn world() -> World {
    let mut w = World::new(
        Box::new(NoSave),
        "harness",
        WorldProvider::new(Dimension::Surface),
        3,
        ChunkGenerator::Empty(EmptyChunkProvider),
        WorldEnvironment {
            rand_seed: 0,
            field_9437_g: 0,
            lock_timestamp: 0,
        },
    );
    w.make_deterministic(3);
    for x in -8..8 {
        for z in -8..8 {
            w.set_block(x, 60, z, 1);
        }
    }
    w
}

fn pig_at(w: &mut World, x: f64, y: f64, z: f64) -> Entity {
    let mut p = pig::new_entity_pig(w);
    p.base.set_location_and_angles(x, y, z, 0.0, 0.0);
    p
}

/// `new EntityPig(world)`: four `Math.random()` draws (EntityLiving.java:38,
/// :63, :65, :66), a 0.9 box, watcher byte 16 (the saddle) = 0, health 10.
#[test]
fn pig_constructor() {
    let mut w = world();
    let mut r = w.math_random.clone();
    let p = pig::new_entity_pig(&mut w);
    for _ in 0..4 {
        r.next_double();
    }
    assert_eq!(w.math_random.state(), r.state());
    assert_eq!((p.base.width, p.base.height), (0.9, 0.9));
    assert_eq!(p.base.step_height, 0.5);
    assert!(p.base.prevent_entity_spawning);
    assert!(!pig::get_saddled(&p));
    let l = p.kind.living().unwrap();
    assert_eq!(
        (l.health, l.texture, l.move_speed),
        (10, "/mob/pig.png", 0.7)
    );
}

/// `EntityPig.writeEntityToNBT` / `readEntityFromNBT`: `EntityLiving`'s four
/// shorts and `Saddle`; `EntityList` builds the pig back.
#[test]
fn pig_nbt_round_trip() {
    let mut w = world();
    let mut p = pig_at(&mut w, 1.5, 61.0, 2.5);
    pig::set_saddled(&mut p, true);
    {
        let l = p.kind.living_mut().unwrap();
        l.health = 7;
        l.hurt_time = 3;
        l.death_time = 0;
        l.attack_time = 2;
    }
    let c = nbt::add_entity_id(&p, &w).unwrap();
    assert_eq!(c.get("Saddle"), Some(&mc_nbt::Tag::Byte(1)));
    assert_eq!(c.get("Health"), Some(&mc_nbt::Tag::Short(7)));
    let FromNbt::Created(back) = create_entity_from_nbt(&c, &mut w).unwrap() else {
        panic!("pig not constructed");
    };
    assert_eq!(back.kind.java_class(), "EntityPig");
    assert!(pig::get_saddled(&back));
    let l = back.kind.living().unwrap();
    assert_eq!((l.health, l.hurt_time, l.attack_time), (7, 3, 2));
    assert_eq!(nbt::add_entity_id(&back, &w).unwrap(), c);
    assert!(w.unimplemented_hits.borrow().is_empty());
}

/// `EntityPig.interact`: only a saddled pig, and only without another
/// rider, is mounted by the player (EntityPig.java:36-43).
#[test]
fn pig_interact_mounts_a_saddled_pig() {
    let mut w = world();
    let mut p = pig_at(&mut w, 0.5, 61.0, 0.5);
    let mut player = new_entity_player_sp(&mut w);
    player
        .base
        .set_location_and_angles(1.5, 61.0, 0.5, 0.0, 0.0);
    assert!(!dispatch::interact(&mut p, &mut w, &mut player));
    assert_eq!(player.base.riding_entity, None);
    pig::set_saddled(&mut p, true);
    assert!(dispatch::interact(&mut p, &mut w, &mut player));
    assert_eq!(player.base.riding_entity, Some(p.base.entity_id));
    assert_eq!(p.base.ridden_by_entity, Some(player.base.entity_id));
    // Another rider: refused.
    p.base.ridden_by_entity = Some(9999);
    assert!(!dispatch::interact(&mut p, &mut w, &mut player));
}

/// `EntityPig.getDropItemId`: cooked while `fire > 0`.
#[test]
fn pig_drop_item_id() {
    let mut w = world();
    let mut p = pig_at(&mut w, 0.5, 61.0, 0.5);
    assert_eq!(dispatch::get_drop_item_id(&p, &w), 256 + 63);
    p.base.fire = 1;
    assert_eq!(dispatch::get_drop_item_id(&p, &w), 256 + 64);
    // Item.java:337-338
    assert_eq!(
        mc_sim::item::item(256 + 63).and_then(|d| d.item_name),
        Some("porkchopRaw")
    );
}

/// `EntityAnimal.getCanSpawnHere`: grass under the feet, full light above 8,
/// then `EntityCreature`'s weight (10 on grass) and `EntityLiving`'s box
/// tests (EntityAnimal.java:20-25, EntityCreature.java:159-164,
/// EntityLiving.java:781-783).
#[test]
fn animal_can_spawn_on_lit_grass_only() {
    let mut w = world();
    w.set_block(2, 60, 2, 2);
    let p = pig_at(&mut w, 2.5, 61.0, 2.5);
    assert!(dispatch::get_can_spawn_here(&p, &mut w));
    let p = pig_at(&mut w, 4.5, 61.0, 4.5);
    assert!(!dispatch::get_can_spawn_here(&p, &mut w), "stone");
    // A block in the box: the collision test fails.
    w.set_block(2, 61, 2, 1);
    let p = pig_at(&mut w, 2.5, 61.0, 2.5);
    assert!(!dispatch::get_can_spawn_here(&p, &mut w));
}

/// `heal`: only while alive, capped at 20, restarts half the window.
#[test]
fn heal() {
    let mut w = world();
    let mut p = pig_at(&mut w, 0.5, 61.0, 0.5);
    living::heal(&mut p, 15);
    assert_eq!(p.kind.living().unwrap().health, 20);
    assert_eq!(p.base.hearts_life, 10);
    p.kind.living_mut().unwrap().health = 0;
    living::heal(&mut p, 5);
    assert_eq!(p.kind.living().unwrap().health, 0);
}

/// `knockBack`: motion halved, 0.4 away from the direction, 0.4 up capped
/// at 0.4 (EntityLiving.java:392-405).
#[test]
fn knock_back() {
    let mut w = world();
    let mut p = pig_at(&mut w, 0.5, 61.0, 0.5);
    let a = pig_at(&mut w, 2.5, 61.0, 0.5);
    p.base.motion_x = 0.2;
    p.base.motion_y = 0.5;
    living::knock_back(&mut p, &a, 1, 2.0, 0.0);
    // sqrt_double(4) = 2.0F; 0.1 - 2 / 2 * 0.4F
    assert_eq!(p.base.motion_x, 0.1 - 2.0 / 2.0f32 as f64 * 0.4f32 as f64);
    assert_eq!(p.base.motion_y, 0.4000000059604645);
    assert_eq!(p.base.motion_z, 0.0);
}

/// `updateRotation` takes the shorter way and clamps. `faceEntity` steps
/// from the stored pitch toward the target pitch and stores the result
/// negated (vanilla bug, transcribed): toward a living target above, the
/// first call stores the correct look-up pitch, the next one steps from it
/// (taken as a look-down pitch) and stores another value, so the pitch
/// alternates between two values instead of settling.
#[test]
fn face_entity_pitch_alternates() {
    assert_eq!(living::update_rotation(170.0, -170.0, 30.0), 190.0);
    assert_eq!(living::update_rotation(0.0, 100.0, 30.0), 30.0);
    assert_eq!(living::update_rotation(0.0, -100.0, 30.0), -30.0);
    let mut w = world();
    let mut p = pig_at(&mut w, 0.5, 61.0, 0.5);
    let t = EntityView {
        id: 99,
        pos_x: 4.5,
        pos_y: 65.0,
        pos_z: 0.5,
        bounding_box: Aabb::new(4.2, 65.0, 0.2, 4.8, 66.8, 0.8),
        eye_height: 1.62,
        is_living: true,
        is_dead: false,
    };
    // var6 = (61 + 0.765) - (65 + 1.62) < 0, so var13 = -atan2(var6, 4)
    // in degrees is positive (about 50.5), and the stored pitch its negation.
    let mut pitches = Vec::new();
    for _ in 0..3 {
        living::face_entity(&mut p, &w, &t, 10.0, 90.0);
        pitches.push(p.base.rotation_pitch);
    }
    let var13 = (-(mc_core::math::jmath::atan2(
        (61.0 + (0.9f32 * 0.85) as f64) - (65.0 + 1.62f32 as f64),
        4.0,
    ) * 180.0
        / 3.1415927410125732)) as f32;
    assert_eq!(pitches[0], -var13);
    assert_eq!(pitches[1], -(-var13 + 90.0));
    assert_eq!(pitches[2], pitches[0]);
    // Yaw toward +x is -90; each call turns at most 10 degrees from 0.
    assert_eq!(p.base.rotation_yaw, -30.0);
}

/// `handleHealthUpdate(2)` (hurt): the hurt state, a hurt sound, and
/// `attackEntityFrom(null, 0)` (no damage: 0 is not above the window's
/// first hit); `(3)` (dead): health 0 and `onDeath(null)` with its drops.
#[test]
fn handle_health_update() {
    let mut w = world();
    w.get_chunk_from_chunk_coords(0, 0);
    let mut p = pig_at(&mut w, 0.5, 61.0, 0.5);
    dispatch::handle_health_update(&mut p, &mut w, 2);
    let l = p.kind.living().unwrap();
    assert_eq!((l.hurt_time, l.max_hurt_time, l.leg_yaw), (10, 10, 1.5));
    assert_eq!(l.health, 10);
    assert_eq!(p.base.hearts_life, 20);
    dispatch::handle_health_update(&mut p, &mut w, 3);
    let l = p.kind.living().unwrap();
    assert_eq!(l.health, 0);
    assert!(l.unused_flag);
}

/// `despawnEntity`: dead beyond 128 blocks of the closest player
/// (EntityLiving.java:667-687); no player, no despawning.
#[test]
fn despawn_far_from_the_player() {
    let mut w = world();
    let mut p = pig_at(&mut w, 0.5, 61.0, 0.5);
    living::despawn_entity(&mut p, &mut w);
    assert!(!p.base.is_dead);
    let mut player = new_entity_player_sp(&mut w);
    player
        .base
        .set_location_and_angles(0.5, 61.0, 200.5, 0.0, 0.0);
    w.spawn_entity_in_world(player);
    living::despawn_entity(&mut p, &mut w);
    assert!(p.base.is_dead);
}

/// `EntityLiving.attackEntityFrom` on a pig with no attacker draws one
/// `Math.random()` (`attackedAtYaw` 0 or 180) and two `rand` floats (the
/// hurt sound's pitch); the kill path drops 0–2 raw porkchops.
#[test]
fn attack_without_attacker() {
    let mut w = world();
    w.get_chunk_from_chunk_coords(0, 0);
    let mut p = pig_at(&mut w, 0.5, 61.0, 0.5);
    let mut r = w.math_random.clone();
    let mut r2 = p.base.rand.clone();
    assert!(dispatch::attack_entity_from(&mut p, &mut w, None, 3));
    let yaw = ((r.next_double() * 2.0) as i32 * 180) as f32;
    assert_eq!(w.math_random.state(), r.state());
    let l = p.kind.living().unwrap();
    assert_eq!((l.health, l.attacked_at_yaw, l.last_damage), (7, yaw, 3));
    r2.next_float();
    r2.next_float();
    assert_eq!(p.base.rand.state(), r2.state());
    assert!(p.base.been_attacked);
}
