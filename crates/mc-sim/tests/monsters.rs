//! WP-2.3 (task 2): unit checks of the hostile mobs that the tick fixtures
//! (`tests/golden/entity/{zombie-chase-player, skeleton-*, spider-leap,
//! creeper-fuse-explode, pigzombie-anger, slime-split, ghast-waypoints}`)
//! do not reach: the constructors (the giant and `EntityList`'s `Monster`
//! have no fixture), the NBT round trip of every class through `EntityList`,
//! `EntityMob.attackEntityFrom`'s target rules, the powered creeper and its
//! record drop, the slime split rules and slime chunks, the pigmen's anger
//! radius, the ghast's course test and the spawn checks.

use mc_core::java_random::JavaRandom;
use mc_sim::entity::data_watcher::WatchValue;
use mc_sim::entity::player::new_entity_player_sp;
use mc_sim::entity::{
    Entity, EntityKindData, creature, creeper, dispatch, ghast, giant_zombie, mob, nbt, pig_zombie,
    skeleton, slime, spider, zombie,
};
use mc_sim::save::NoSave;
use mc_sim::save::entity_list::{FromNbt, create_entity_from_nbt, create_entity_in_world};
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::{World, WorldEnvironment};

fn empty_world(seed: i64) -> World {
    let mut w = World::new(
        Box::new(NoSave),
        "harness",
        WorldProvider::new(Dimension::Surface),
        seed,
        ChunkGenerator::Empty(EmptyChunkProvider),
        WorldEnvironment {
            rand_seed: 0,
            field_9437_g: 0,
            lock_timestamp: 0,
        },
    );
    w.make_deterministic(seed);
    for dz in -3..=3 {
        for dx in -3..=3 {
            w.get_chunk_from_chunk_coords(dx, dz);
        }
    }
    for x in -16..16 {
        for z in -16..16 {
            w.set_block(x, 60, z, 1);
        }
    }
    w
}

fn living(e: &Entity) -> &mc_sim::entity::LivingData {
    e.kind.living().expect("living")
}

fn target(e: &Entity) -> Option<i32> {
    creature::creature(&e.kind).and_then(|c| c.player_to_attack)
}

fn placed(mut e: Entity, x: f64, y: f64, z: f64) -> Entity {
    e.base.set_location_and_angles(x, y, z, 0.0, 0.0);
    e
}

/// The item ids of the `EntityItem`s from list index `from` on.
fn drops_from(w: &World, from: usize) -> Vec<i32> {
    w.loaded_entity_list[from..]
        .iter()
        .filter_map(|s| s.entity.as_ref()?.kind.item()?.item)
        .map(|s| s.item_id)
        .collect()
}

/// The constructors' class-level values (EntityMob.java:4-9,
/// EntityZombie.java:4-9, EntityGiantZombie.java:4-12,
/// EntitySkeleton.java:6-9, EntitySpider.java:4-9, EntityCreeper.java:7-16,
/// EntityPigZombie.java:10-16, EntitySlime.java:8-20, EntityGhast.java:13-23).
#[test]
fn constructors() {
    let mut w = empty_world(3);
    let m = mob::new_entity_mob(&mut w);
    assert_eq!(
        (
            living(&m).health,
            living(&m).texture,
            mob::mob(&m.kind).unwrap().attack_strength
        ),
        (20, "/mob/char.png", 2)
    );
    let z = zombie::new_entity_zombie(&mut w);
    assert_eq!(
        (
            living(&z).texture,
            living(&z).move_speed,
            mob::mob(&z.kind).unwrap().attack_strength
        ),
        ("/mob/zombie.png", 0.5, 5)
    );
    let g = giant_zombie::new_entity_giant_zombie(&mut w);
    assert_eq!(
        (
            living(&g).health,
            mob::mob(&g.kind).unwrap().attack_strength,
            g.base.width,
            g.base.height,
            g.base.y_offset
        ),
        (200, 50, 0.6f32 * 6.0, 1.8f32 * 6.0, 0.0)
    );
    let s = skeleton::new_entity_skeleton(&mut w);
    assert_eq!(
        (living(&s).health, living(&s).texture),
        (20, "/mob/skeleton.png")
    );
    let sp = spider::new_entity_spider(&mut w);
    assert_eq!(
        (sp.base.width, sp.base.height, living(&sp).move_speed),
        (1.4, 0.9, 0.8)
    );
    let c = creeper::new_entity_creeper(&mut w);
    assert_eq!(
        (creeper::get_creeper_state(&c), creeper::get_powered(&c)),
        (-1, false)
    );
    let pz = pig_zombie::new_entity_pig_zombie(&mut w);
    assert!(pz.base.is_immune_to_fire);
    assert_eq!(
        (
            living(&pz).texture,
            mob::mob(&pz.kind).unwrap().attack_strength
        ),
        ("/mob/pigzombie.png", 5)
    );
    let gh = ghast::new_entity_ghast(&mut w);
    assert!(gh.base.is_immune_to_fire);
    assert_eq!((gh.base.width, living(&gh).health), (4.0, 10));

    // The slime's size and jump delay come from its construction-seeded
    // rand: 1 << nextInt(3), then nextInt(20) + 10 (EntitySlime.java:11-14).
    let sl = slime::new_entity_slime(&mut w);
    let mut r = JavaRandom::new(w.entity_rand_seed_for(sl.base.entity_id));
    let size = 1 << r.next_int_bounded(3);
    let delay = r.next_int_bounded(20) + 10;
    assert_eq!(slime::get_slime_size(&sl), size);
    assert_eq!(living(&sl).health, size * size);
    assert_eq!(sl.base.width, 0.6 * size as f32);
    match &sl.kind {
        EntityKindData::Slime(_, d) => assert_eq!(d.slime_jump_delay, delay),
        _ => unreachable!(),
    }

    // EntityList: "Monster" is the concrete EntityMob, "Mob" the abstract
    // EntityLiving.
    assert_eq!(
        create_entity_in_world("Monster", &mut w).map(|e| e.kind.java_class()),
        Some("EntityMob")
    );
    assert!(create_entity_in_world("Mob", &mut w).is_none());
}

/// `writeEntityToNBT` / `readEntityFromNBT` of every class through
/// `EntityList.createEntityFromNBT`: `powered` (EntityCreeper.java:18-29),
/// `Anger` (EntityPigZombie.java:31-39), `Size` (EntitySlime.java:33-41,
/// whose read resets health to size², a vanilla bug kept), and
/// `EntityLiving`'s fields for the others.
#[test]
fn nbt_round_trip_through_entity_list() {
    let mut w = empty_world(4);
    let round = |w: &mut World, e: &Entity| -> Box<Entity> {
        let c = nbt::add_entity_id(e, w).expect("in EntityList");
        match create_entity_from_nbt(&c, w).unwrap() {
            FromNbt::Created(g) => g,
            other => panic!("{} not constructed: {other:?}", e.kind.java_class()),
        }
    };

    let mut c = creeper::new_entity_creeper(&mut w);
    let plain = nbt::write_to_nbt(&c, &w);
    assert!(!plain.contains_key("powered"));
    let bolt = mc_sim::entity::lightning_bolt::new_entity_lightning_bolt(&mut w, 0.0, 61.0, 0.0);
    dispatch::on_struck_by_lightning(&mut c, &mut w, &bolt);
    assert!(creeper::get_powered(&c));
    let g = round(&mut w, &c);
    assert!(creeper::get_powered(&g));
    assert_eq!(creeper::get_creeper_state(&g), -1);

    let mut pz = pig_zombie::new_entity_pig_zombie(&mut w);
    pig_zombie::become_angry_at(&mut pz, 77);
    let anger = match &pz.kind {
        EntityKindData::PigZombie(.., d) => d.anger_level,
        _ => unreachable!(),
    };
    assert!((400..800).contains(&anger));
    let g = round(&mut w, &pz);
    match &g.kind {
        EntityKindData::PigZombie(.., d) => assert_eq!(d.anger_level, anger),
        _ => panic!("not a pig zombie"),
    }

    let mut sl = slime::new_entity_slime(&mut w);
    slime::set_slime_size(&mut sl, 4);
    sl.kind.living_mut().unwrap().health = 3;
    let c = nbt::write_to_nbt(&sl, &w);
    assert_eq!(c.get("Size"), Some(&mc_nbt::Tag::Int(3)));
    let g = round(&mut w, &sl);
    assert_eq!(slime::get_slime_size(&g), 4);
    assert_eq!(living(&g).health, 16, "setSlimeSize resets the health");

    let mut s = skeleton::new_entity_skeleton(&mut w);
    s.kind.living_mut().unwrap().health = 7;
    let mut gh = ghast::new_entity_ghast(&mut w);
    gh.kind.living_mut().unwrap().health = 4;
    let others = [
        mob::new_entity_mob(&mut w),
        zombie::new_entity_zombie(&mut w),
        giant_zombie::new_entity_giant_zombie(&mut w),
        s,
        spider::new_entity_spider(&mut w),
        gh,
    ];
    for e in &others {
        let g = round(&mut w, e);
        assert_eq!(g.kind.java_class(), e.kind.java_class());
        assert_eq!(living(&g).health, living(e).health);
    }
    assert!(
        w.unimplemented_hits
            .borrow()
            .keys()
            .all(|k| !k.contains("NBT"))
    );
}

/// `EntityMob.attackEntityFrom` (EntityMob.java:33-47): a counted hit makes
/// the attacker the target, unless it is the mob itself, its rider or its
/// mount; with no attacker and both rider and mount set, the reference
/// comparison clears the target.
#[test]
fn mob_hit_sets_the_target() {
    let mut w = empty_world(5);
    let mut z = placed(zombie::new_entity_zombie(&mut w), 0.5, 61.0, 0.5);
    let p = placed(new_entity_player_sp(&mut w), 3.5, 61.0, 0.5);
    assert!(dispatch::attack_entity_from(&mut z, &mut w, Some(&p), 1));
    assert_eq!(target(&z), Some(p.base.entity_id));

    let mut z2 = placed(zombie::new_entity_zombie(&mut w), 0.5, 61.0, 4.5);
    let me = z2.clone();
    assert!(dispatch::attack_entity_from(&mut z2, &mut w, Some(&me), 1));
    assert_eq!(target(&z2), None, "not itself");

    // A hit inside the resistance window that does not count: no target.
    let mut z3 = placed(zombie::new_entity_zombie(&mut w), 4.5, 61.0, 4.5);
    assert!(dispatch::attack_entity_from(&mut z3, &mut w, None, 5));
    assert!(!dispatch::attack_entity_from(&mut z3, &mut w, Some(&p), 2));
    assert_eq!(target(&z3), None);

    let mut z4 = placed(zombie::new_entity_zombie(&mut w), -4.5, 61.0, 4.5);
    creature::creature_mut(&mut z4.kind)
        .unwrap()
        .player_to_attack = Some(99);
    z4.base.ridden_by_entity = Some(100);
    z4.base.riding_entity = Some(101);
    assert!(dispatch::attack_entity_from(&mut z4, &mut w, None, 1));
    assert_eq!(target(&z4), None, "null attacker != rider and mount");
}

/// `EntityMob.onUpdate` (EntityMob.java:20-26): dead on peaceful; and
/// `EntityGhast.updatePlayerActionState` likewise (EntityGhast.java:32-34).
#[test]
fn peaceful_kills_monsters() {
    let mut w = empty_world(6);
    w.difficulty_setting = 0;
    let mut z = placed(zombie::new_entity_zombie(&mut w), 0.5, 61.0, 0.5);
    dispatch::on_update(&mut z, &mut w);
    assert!(z.base.is_dead);
    let mut g = placed(ghast::new_entity_ghast(&mut w), 0.5, 70.0, 0.5);
    dispatch::update_player_action_state(&mut g, &mut w);
    assert!(g.base.is_dead);
    w.difficulty_setting = 1;
    let mut z = placed(zombie::new_entity_zombie(&mut w), 0.5, 61.0, 0.5);
    dispatch::on_update(&mut z, &mut w);
    assert!(!z.base.is_dead);
}

/// `EntityCreeper.onDeath` (EntityCreeper.java:81-87): a skeleton's kill
/// drops `record13 + rand.nextInt(2)` after `EntityLiving`'s gunpowder;
/// and the flash interpolation (EntityCreeper.java:125-127).
#[test]
fn creeper_record_and_flash() {
    let mut w = empty_world(7);
    let mut c = placed(creeper::new_entity_creeper(&mut w), 0.5, 61.0, 0.5);
    let s = placed(skeleton::new_entity_skeleton(&mut w), 3.5, 61.0, 0.5);
    let mut r = c.base.rand.clone();
    let gunpowder = r.next_int_bounded(3);
    let record = creeper::RECORD_13 + r.next_int_bounded(2);
    let before = w.loaded_entity_list.len();
    dispatch::on_death(&mut c, &mut w, Some(&s));
    let mut want = vec![creeper::GUNPOWDER; gunpowder as usize];
    want.push(record);
    assert_eq!(drops_from(&w, before), want);

    if let EntityKindData::Creeper(.., d) = &mut c.kind {
        d.last_active_time = 10;
        d.time_since_ignited = 12;
    }
    assert_eq!(
        creeper::set_creeper_flash_time(&c, 0.5),
        (10.0 + 2.0 * 0.5) / 28.0
    );
}

/// `EntitySlime.setEntityDead` (EntitySlime.java:98-112): four half-size
/// slimes only for size 2 or more and health exactly 0.
#[test]
fn slime_splits_only_at_zero_health() {
    let mut w = empty_world(8);
    let count = |w: &World| {
        w.loaded_entity_list
            .iter()
            .filter(|s| {
                matches!(
                    s.entity.as_deref().map(|e| &e.kind),
                    Some(EntityKindData::Slime(..))
                )
            })
            .count()
    };
    let mut s = placed(slime::new_entity_slime(&mut w), 0.5, 61.0, 0.5);
    slime::set_slime_size(&mut s, 4);
    s.kind.living_mut().unwrap().health = -1;
    dispatch::set_entity_dead(&mut s, &mut w);
    assert!(s.base.is_dead);
    assert_eq!(count(&w), 0, "overkill: no split");

    let mut s = placed(slime::new_entity_slime(&mut w), 0.5, 61.0, 0.5);
    slime::set_slime_size(&mut s, 4);
    s.kind.living_mut().unwrap().health = 0;
    dispatch::set_entity_dead(&mut s, &mut w);
    assert_eq!(count(&w), 4);
    for slot in &w.loaded_entity_list {
        let k = slot.entity.as_deref().unwrap();
        assert_eq!(slime::get_slime_size(k), 2);
        assert_eq!(k.base.pos_y, 61.5);
        assert_eq!(dispatch::get_drop_item_id(k, &w), 0);
    }

    let mut one = placed(slime::new_entity_slime(&mut w), 0.5, 61.0, 0.5);
    slime::set_slime_size(&mut one, 1);
    one.kind.living_mut().unwrap().health = 0;
    dispatch::set_entity_dead(&mut one, &mut w);
    assert_eq!(count(&w), 4, "size 1 does not split");
    assert_eq!(dispatch::get_drop_item_id(&one, &w), slime::SLIME_BALL);
}

/// `EntitySlime.getCanSpawnHere` (EntitySlime.java:134-137) with the slime
/// chunk test of `Chunk.getRandomWithSeed(987234911)` (Chunk.java:598-600),
/// recomputed here from the formula.
#[test]
fn slime_chunks() {
    let mut w = empty_world(12345);
    let seed = w.get_random_seed();
    let slime_chunk = |x: i32, z: i32| {
        let s = seed
            .wrapping_add((x.wrapping_mul(x).wrapping_mul(4987142)) as i64)
            .wrapping_add(x.wrapping_mul(5947611) as i64)
            .wrapping_add((z.wrapping_mul(z) as i64).wrapping_mul(4392871))
            .wrapping_add(z.wrapping_mul(389711) as i64)
            ^ 987234911;
        JavaRandom::new(s).next_int_bounded(10) == 0
    };
    let mut hits = 0;
    for cx in -3..=3 {
        for cz in -3..=3 {
            let mut s = slime::new_entity_slime(&mut w);
            slime::set_slime_size(&mut s, 1);
            s.base
                .set_position(cx as f64 * 16.0 + 8.0, 10.0, cz as f64 * 16.0 + 8.0);
            let mut r = s.base.rand.clone();
            let want = r.next_int_bounded(10) == 0 && slime_chunk(cx, cz);
            hits += i32::from(slime_chunk(cx, cz));
            assert_eq!(
                dispatch::get_can_spawn_here(&mut s, &mut w),
                want,
                "chunk {cx},{cz}"
            );
            s.base
                .set_position(cx as f64 * 16.0 + 8.0, 20.0, cz as f64 * 16.0 + 8.0);
            assert!(!dispatch::get_can_spawn_here(&mut s, &mut w), "y >= 16");
        }
    }
    assert!(hits > 0, "no slime chunk among the 49");
    // A big slime needs a difficulty above peaceful (no draw otherwise).
    w.difficulty_setting = 0;
    let mut big = slime::new_entity_slime(&mut w);
    slime::set_slime_size(&mut big, 2);
    big.base.set_position(8.0, 10.0, 8.0);
    let r0 = big.base.rand.clone();
    assert!(!dispatch::get_can_spawn_here(&mut big, &mut w));
    assert_eq!(big.base.rand, r0, "short-circuit: no draw");
}

/// `EntityPigZombie.attackEntityFrom` (EntityPigZombie.java:49-65): a
/// player's hit angers every pigman whose box meets this one's grown by 32,
/// and this one; another attacker angers none; `findPlayerToAttack` is null
/// while calm.
#[test]
fn pigmen_anger_radius() {
    let mut w = empty_world(9);
    let near = placed(pig_zombie::new_entity_pig_zombie(&mut w), 10.5, 61.0, 0.5);
    let far = placed(pig_zombie::new_entity_pig_zombie(&mut w), 40.5, 61.0, 0.5);
    let (near_id, far_id) = (near.base.entity_id, far.base.entity_id);
    w.get_chunk_from_chunk_coords(2, 0);
    assert!(w.spawn_entity_in_world(near));
    assert!(w.spawn_entity_in_world(far));
    let p = placed(new_entity_player_sp(&mut w), -3.5, 61.0, 0.5);
    let pid = p.base.entity_id;
    let mut me = placed(pig_zombie::new_entity_pig_zombie(&mut w), 0.5, 61.0, 0.5);
    assert_eq!(dispatch::find_player_to_attack(&mut me, &mut w), None);

    let pig = placed(mc_sim::entity::pig::new_entity_pig(&mut w), 2.5, 61.0, 0.5);
    assert!(dispatch::attack_entity_from(&mut me, &mut w, Some(&pig), 1));
    assert_eq!(
        target(&me),
        Some(pig.base.entity_id),
        "EntityMob's target only"
    );
    assert_eq!(target(w.entity(near_id).unwrap()), None);

    me.base.hearts_life = 0;
    assert!(dispatch::attack_entity_from(&mut me, &mut w, Some(&p), 1));
    assert_eq!(target(&me), Some(pid));
    assert_eq!(target(w.entity(near_id).unwrap()), Some(pid));
    assert_eq!(target(w.entity(far_id).unwrap()), None, "beyond 32");
}

/// `EntityGhast.isCourseTraversable` (EntityGhast.java:115-129): the box
/// stepped toward the waypoint stops at a wall; `EntityFlying.fall` does
/// nothing; the spawn limit, the sound volume and the open-mouth texture.
#[test]
fn ghast_course_and_flight() {
    let mut w = empty_world(10);
    for y in 61..80 {
        for z in -8..8 {
            w.set_block(8, y, z, 1);
        }
    }
    let mut g = placed(ghast::new_entity_ghast(&mut w), 0.5, 65.0, 0.5);
    if let EntityKindData::Ghast(.., d) = &mut g.kind {
        d.waypoint_x = 12.5;
        d.waypoint_y = 65.0;
        d.waypoint_z = 0.5;
    }
    assert!(!ghast::is_course_traversable(
        &g, &mut w, 12.5, 65.0, 0.5, 12.0
    ));
    if let EntityKindData::Ghast(.., d) = &mut g.kind {
        d.waypoint_x = -6.5;
    }
    assert!(ghast::is_course_traversable(
        &g, &mut w, -6.5, 65.0, 0.5, 7.0
    ));

    let hp = living(&g).health;
    dispatch::fall(&mut g, &mut w, 30.0);
    assert_eq!(living(&g).health, hp);
    assert!(!dispatch::is_on_ladder(&g, &mut w));
    assert_eq!(dispatch::get_max_spawned_in_chunk(&g, &w), 1);
    assert_eq!(dispatch::get_sound_volume(&g, &w), 10.0);

    w.difficulty_setting = 1;
    g.base.data_watcher.update_object(16, WatchValue::Byte(1));
    dispatch::on_update(&mut g, &mut w);
    // onUpdate picks the texture after EntityLiving's update, whose
    // updatePlayerActionState (no player: attackCounter stays 0) has written
    // byte 16 back to 0.
    assert_eq!(g.base.data_watcher.get_watchable_object_byte(16), 0);
    assert_eq!(living(&g).texture, "/mob/ghast.png");
}

/// `EntityMob.getCanSpawnHere` (EntityMob.java:69-86): the sky-light roll
/// draws first; the block-light roll draws only after a passed sky roll.
/// The spider's mount offset and wall climbing; the pigman's spawn test.
#[test]
fn spawn_rolls_and_spider() {
    let mut w = empty_world(11);
    let mut both = 0;
    for _ in 0..40 {
        let mut z = placed(zombie::new_entity_zombie(&mut w), 0.5, 61.0, 0.5);
        let mut r = z.base.rand.clone();
        let sky = r.next_int_bounded(32);
        if sky >= 15 {
            // The sky roll passes: the block-light roll draws (and fails:
            // block light 15 in the day is never <= nextInt(8)).
            r.next_int_bounded(8);
            both += 1;
        }
        assert!(!dispatch::get_can_spawn_here(&mut z, &mut w));
        assert_eq!(z.base.rand, r, "draws: sky {sky}");
    }
    assert!(both > 0);

    let sp = spider::new_entity_spider(&mut w);
    assert_eq!(
        dispatch::get_mounted_y_offset(&sp, &w),
        0.9f32 as f64 * 0.75 - 0.5
    );
    let mut on_wall = sp.clone();
    on_wall.base.is_collided_horizontally = true;
    assert!(dispatch::is_on_ladder(&on_wall, &mut w));
    assert!(!dispatch::is_on_ladder(&sp, &mut w));

    let mut pz = placed(pig_zombie::new_entity_pig_zombie(&mut w), 0.5, 61.0, 0.5);
    w.difficulty_setting = 0;
    assert!(!dispatch::get_can_spawn_here(&mut pz, &mut w));
    w.difficulty_setting = 1;
    assert!(dispatch::get_can_spawn_here(&mut pz, &mut w));
}

/// `EntitySkeleton.dropFewItems` (EntitySkeleton.java:67-81): arrows, then
/// bones, `rand.nextInt(3)` each, no `super` call; the held items (render
/// data).
#[test]
fn skeleton_drops() {
    let mut w = empty_world(13);
    let mut s = placed(skeleton::new_entity_skeleton(&mut w), 0.5, 61.0, 0.5);
    let mut r = s.base.rand.clone();
    let arrows = r.next_int_bounded(3);
    let bones = r.next_int_bounded(3);
    let before = w.loaded_entity_list.len();
    dispatch::drop_few_items(&mut s, &mut w);
    let mut want = vec![skeleton::ARROW; arrows as usize];
    want.extend(vec![skeleton::BONE; bones as usize]);
    assert_eq!(drops_from(&w, before), want);
    assert_eq!(skeleton::get_held_item().item_id, skeleton::BOW);
    assert_eq!(pig_zombie::get_held_item().item_id, pig_zombie::SWORD_GOLD);
}

/// `EntitySlime.onCollideWithPlayer` (EntitySlime.java:114-120): a slime of
/// size 2 or more that sees the player and is closer than `0.6 × size` hits
/// it for `size` (through the player's `attackEntityFrom`, WP-2.5's
/// override recorded); a size-1 slime never does.
#[test]
fn slime_hits_a_touching_player() {
    let mut w = empty_world(14);
    let mut p = placed(new_entity_player_sp(&mut w), 0.5, 61.0, 0.5);
    let hp = living(&p).health;
    let mut s = placed(slime::new_entity_slime(&mut w), 1.0, 62.0, 0.5);
    slime::set_slime_size(&mut s, 1);
    dispatch::on_collide_with_player(&mut s, &mut w, &mut p);
    assert_eq!(living(&p).health, hp, "size 1");
    slime::set_slime_size(&mut s, 2);
    s.base.set_position(1.0, 62.0, 0.5);
    assert!((s.base.get_distance_to_entity(&p.base) as f64) < 1.2);
    dispatch::on_collide_with_player(&mut s, &mut w, &mut p);
    assert_eq!(living(&p).health, hp - 2);
    let mut far = placed(slime::new_entity_slime(&mut w), 3.5, 61.0, 0.5);
    slime::set_slime_size(&mut far, 2);
    p.base.hearts_life = 0;
    dispatch::on_collide_with_player(&mut far, &mut w, &mut p);
    assert_eq!(living(&p).health, hp - 2, "too far");
}
