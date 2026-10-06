//! WP-2.3 (squid, water mobs, spawn rules): unit checks of the paths the
//! fixtures (`entity/squid-swim`, `tick/natural-spawn-squid`,
//! `tick/mob-spawner-spawn`) do not reach or do not show: the squid's
//! constructor draw, NBT round trip, drops, out-of-water branch and
//! spawn test; `EntityList`'s constructor table (which names construct
//! after this task, for the integrator); `creatureSpecificInit`'s sheep
//! fleece; `performSleepSpawning` from `World.tick` with the monster
//! constructors still missing.

use mc_core::JavaRandom;
use mc_sim::entity::base::mix_seed;
use mc_sim::entity::data_watcher::WatchValue;
use mc_sim::entity::player::new_entity_player_sp;
use mc_sim::entity::{
    AnimalData, CreatureData, Entity, EntityBase, EntityKindData, LivingData, SheepData, dispatch,
    nbt, squid,
};
use mc_sim::save::NoSave;
use mc_sim::save::entity_list::{
    self, CONSTRUCTORS, FromNbt, MAPPINGS, create_entity_from_nbt, create_entity_in_world,
};
use mc_sim::spawner;
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
    w
}

fn squid_data(e: &Entity) -> &squid::SquidData {
    match &e.kind {
        EntityKindData::Squid(.., s) => s,
        _ => panic!("not a squid"),
    }
}

/// `new EntitySquid(world)`: `EntityLiving`'s four `Math.random()` draws, a
/// 0.95 box, the squid texture, health 10, and `rotationVelocity =
/// 1 / (nextFloat() + 1) * 0.2` as the first draw of the `rand` seeded with
/// `mix(seed, id)` (the oracle's join-hook redraw).
#[test]
fn squid_constructor_draws_rotation_velocity() {
    let mut w = world();
    let mut m = w.math_random.clone();
    let mut s = squid::new_entity_squid(&mut w);
    for _ in 0..4 {
        m.next_double();
    }
    assert_eq!(w.math_random.state(), m.state());
    let mut r = JavaRandom::new(mix_seed(w.entity_rand_seed, s.base.entity_id as i64));
    let v = 1.0f32 / (r.next_float() + 1.0) * 0.2;
    assert_eq!(squid_data(&s).rotation_velocity.to_bits(), v.to_bits());
    assert_eq!(s.base.rand.state(), r.state());
    assert_eq!((s.base.width, s.base.height), (0.95, 0.95));
    let l = s.kind.living().unwrap();
    assert_eq!((l.health, l.texture), (10, "/mob/squid.png"));
    assert!(dispatch::can_breathe_underwater(&s, &w));
    assert_eq!(dispatch::get_talk_interval(&s, &w), 120);
    assert_eq!(dispatch::get_living_sound(&mut s, &w), None);
    assert_eq!(dispatch::get_hurt_sound(&s, &w), None);
    assert_eq!(dispatch::get_death_sound(&s, &w), None);
    assert_eq!(dispatch::get_sound_volume(&s, &w), 0.4);
    assert_eq!(dispatch::get_drop_item_id(&s, &w), 0);
    assert!(w.unimplemented_hits.borrow().is_empty());
}

/// `EntitySquid` / `EntityWaterMob` NBT: only `EntityLiving`'s shorts;
/// `EntityList` builds the squid back (a new id, a new `rotationVelocity`).
#[test]
fn squid_nbt_round_trip() {
    let mut w = world();
    w.set_block(0, 60, 0, 1);
    let mut s = squid::new_entity_squid(&mut w);
    s.base.set_location_and_angles(0.5, 61.0, 0.5, 30.0, 0.0);
    s.kind.living_mut().unwrap().health = 6;
    let c = nbt::add_entity_id(&s, &w).unwrap();
    assert_eq!(c.get("id"), Some(&mc_nbt::Tag::String("Squid".into())));
    assert_eq!(c.get("Health"), Some(&mc_nbt::Tag::Short(6)));
    let FromNbt::Created(back) = create_entity_from_nbt(&c, &mut w).unwrap() else {
        panic!("squid not constructed");
    };
    assert_eq!(back.kind.java_class(), "EntitySquid");
    assert_eq!(back.kind.living().unwrap().health, 6);
    assert_eq!(nbt::add_entity_id(&back, &w).unwrap(), c);
    assert!(w.unimplemented_hits.borrow().is_empty());
}

/// `EntitySquid.dropFewItems` through `onDeath`: `rand.nextInt(3) + 1` ink
/// sacs (`dyePowder`, damage 0), each its own `EntityItem`.
#[test]
fn squid_drops_one_to_three_ink_sacs() {
    let mut w = world();
    for x in -2..=2 {
        for z in -2..=2 {
            w.set_block(x, 60, z, 1);
        }
    }
    let mut s = squid::new_entity_squid(&mut w);
    s.base.set_location_and_angles(0.5, 61.0, 0.5, 0.0, 0.0);
    let mut r = s.base.rand.clone();
    let n = r.next_int_bounded(3) + 1;
    dispatch::drop_few_items(&mut s, &mut w);
    let items: Vec<_> = w
        .loaded_entity_list
        .iter()
        .filter_map(|slot| slot.entity.as_deref())
        .filter_map(|e| e.kind.item())
        .collect();
    assert_eq!(items.len() as i32, n);
    for d in items {
        let st = d.item.as_ref().unwrap();
        assert_eq!(
            (st.item_id, st.stack_size, st.item_damage),
            (256 + 95, 1, 0)
        );
    }
}

/// Out of water: no horizontal motion, gravity, the pitch eased toward
/// -90 by 2 % a tick (`(float)((double)p + (double)(-90 - p) * 0.02)`),
/// and a new swim direction every tick (`!inWater`).
#[test]
fn squid_on_land_falls_and_tips_over() {
    let mut w = world();
    for x in -2..=2 {
        for z in -2..=2 {
            w.set_block(x, 60, z, 1);
        }
    }
    let mut s = squid::new_entity_squid(&mut w);
    s.base.set_location_and_angles(0.5, 61.0, 0.5, 0.0, 0.0);
    s.base.motion_x = 0.3;
    dispatch::on_living_update(&mut s, &mut w);
    assert_eq!(s.base.motion_x, 0.0);
    assert_eq!(s.base.motion_z, 0.0);
    let d = squid_data(&s);
    assert_eq!(
        d.squid_pitch.to_bits(),
        ((0.0f32 as f64 + (-90.0f32 - 0.0) as f64 * 0.02) as f32).to_bits()
    );
    assert!(d.random_motion_vec_x != 0.0 || d.random_motion_vec_z != 0.0);
    assert!(w.unimplemented_hits.borrow().is_empty());
}

/// `EntityWaterMob.getCanSpawnHere`: only `checkIfAABBIsClear`, so a squid
/// may spawn inside water (where `EntityLiving`'s liquid test would refuse
/// it) and even inside a block.
#[test]
fn water_mob_spawn_test_is_only_the_entity_check() {
    let mut w = world();
    for y in 55..60 {
        w.set_block(0, y, 0, 9);
    }
    w.set_block(3, 57, 3, 1);
    let mut s = squid::new_entity_squid(&mut w);
    s.base.set_location_and_angles(0.5, 56.0, 0.5, 0.0, 0.0);
    assert!(dispatch::get_can_spawn_here(&mut s, &mut w));
    s.base.set_location_and_angles(3.5, 57.0, 3.5, 0.0, 0.0);
    assert!(dispatch::get_can_spawn_here(&mut s, &mut w));
    // A spawn-blocking entity (any living one) in the box refuses it.
    let mut other = squid::new_entity_squid(&mut w);
    other.base.set_location_and_angles(3.5, 57.0, 3.5, 0.0, 0.0);
    assert!(w.spawn_entity_in_world(other));
    assert!(!dispatch::get_can_spawn_here(&mut s, &mut w));
}

/// Which `EntityList` names construct after this task, and which are still
/// gaps. The integrator updated `MISSING` as the parallel mob tasks merged;
/// it is empty since WP-2.3c's merge. Every constructor builds its own class, and
/// `createEntityInWorld` of a missing class makes the constructor's side
/// effects (one id, and `EntityLiving`'s four `Math.random()` draws).
#[test]
fn entity_list_constructors_and_gaps() {
    // Every concrete class constructs since WP-2.3a (the animals), WP-2.3b
    // (the monsters) and WP-2.3c (the squid) merged; the gap branch below
    // stays for a class added to MAPPINGS without a constructor.
    const CONSTRUCTS: &[&str] = &[
        "Arrow",
        "Snowball",
        "Item",
        "Painting",
        "Monster",
        "Creeper",
        "Skeleton",
        "Spider",
        "Giant",
        "Zombie",
        "Slime",
        "Ghast",
        "PigZombie",
        "Pig",
        "Sheep",
        "Cow",
        "Chicken",
        "Squid",
        "Wolf",
        "PrimedTnt",
        "FallingSand",
        "Minecart",
        "Boat",
    ];
    const MISSING: &[&str] = &[];
    assert_eq!(entity_list::missing_constructors(), MISSING);
    let mut w = world();
    for m in MAPPINGS.iter() {
        let id_before = w.next_entity_id;
        let mut r = w.math_random.clone();
        let e = create_entity_in_world(m.name, &mut w);
        match m.name {
            "Mob" => {
                assert!(e.is_none());
                assert_eq!(w.next_entity_id, id_before, "abstract: nothing built");
            }
            n if CONSTRUCTS.contains(&n) => {
                let e = e.unwrap_or_else(|| panic!("{n} should construct"));
                assert_eq!(e.kind.java_class(), m.class);
                assert_eq!(entity_list::get_entity_string(&e.kind), Some(n));
                assert_eq!(w.next_entity_id, id_before + 1);
            }
            n => {
                assert!(MISSING.contains(&n), "{n} in neither list");
                assert!(e.is_none());
                assert_eq!(w.next_entity_id, id_before + 1);
                for _ in 0..4 {
                    r.next_double();
                }
                assert_eq!(
                    w.math_random.state(),
                    r.state(),
                    "{n}: EntityLiving's draws"
                );
                let gap = m.new_instance_gap;
                assert!(gap.contains(&format!("the {}(World) constructor", m.class)));
                assert_eq!(w.unimplemented_hits.borrow().get(gap), Some(&1));
            }
        }
    }
    assert_eq!(CONSTRUCTORS.len(), CONSTRUCTS.len());
    assert!(create_entity_in_world("Nonsense", &mut w).is_none());
}

/// `creatureSpecificInit` on a sheep: `EntitySheep.getRandomFleeceColor`
/// on `World.rand` into the low nibble of watcher byte 16 (the sheared bit
/// kept); no draw for other kinds.
#[test]
fn creature_specific_init_sheep_fleece() {
    let mut w = world();
    let mut b = EntityBase::new(&mut w);
    b.data_watcher.add_object(16, WatchValue::Byte(16));
    let l = LivingData::new(&mut w, &mut b);
    let mut sheep = Entity {
        base: b,
        kind: EntityKindData::Sheep(
            l,
            CreatureData::default(),
            AnimalData::default(),
            SheepData::default(),
        ),
    };
    let mut r = w.rand.clone();
    let v = r.next_int_bounded(100);
    let colour = match v {
        0..5 => 15,
        5..10 => 7,
        10..15 => 8,
        15..18 => 12,
        _ => {
            if r.next_int_bounded(500) == 0 {
                6
            } else {
                0
            }
        }
    };
    spawner::creature_specific_init(&mut sheep, &mut w, 0.5, 61.0, 0.5);
    assert_eq!(w.rand.state(), r.state());
    assert_eq!(
        sheep.base.data_watcher.get_watchable_object_byte(16) as i32,
        16 | colour
    );
    let before = w.rand.state();
    let mut s = squid::new_entity_squid(&mut w);
    spawner::creature_specific_init(&mut s, &mut w, 0.5, 61.0, 0.5);
    assert_eq!(w.rand.state(), before);
}

/// `World.tick` with every player fully asleep and hostile spawning on runs
/// `performSleepSpawning`: the monsters are built (WP-2.3b) but none passes
/// both `getCanSpawnHere` and the path test in the closed room, so none
/// joins, the night is skipped and the player woken (`wakeUpPlayer`,
/// WP-2.5a).
/// Hostile spawning off: no sleep spawning, the night is skipped too.
#[test]
fn sleep_spawning_from_world_tick() {
    for hostile in [true, false] {
        let mut w = world();
        // A dark stone room around the player (tick/sleep-spawning's).
        for x in -4..=4 {
            for z in -4..=4 {
                for y in 59..=66 {
                    let inside =
                        (-3..=3).contains(&x) && (-3..=3).contains(&z) && (61..=64).contains(&y);
                    w.set_block(x, y, z, if inside { 0 } else { 1 });
                }
            }
        }
        let mut p = new_entity_player_sp(&mut w);
        p.base.set_location_and_angles(0.5, 61.0, 0.5, 0.0, 0.0);
        {
            let d = p.kind.player_mut().unwrap();
            d.sleeping = true;
            d.sleep_timer = 100;
        }
        let pid = p.id();
        assert!(w.spawn_entity_in_world(p));
        w.update_all_players_sleeping_flag();
        w.world_info.world_time = 18000;
        w.difficulty_setting = 1;
        w.set_allowed_mob_spawns(hostile, true);
        assert!(w.is_all_players_fully_asleep());
        let entities = w.loaded_entity_list.len();
        let ids = w.next_entity_id;
        w.tick();
        assert_eq!(w.world_info.world_time, 24001, "night skipped, then +1");
        assert_eq!(w.loaded_entity_list.len(), entities);
        let hits = w.unimplemented_hits.borrow();
        // Every monster of nightSpawnEntities constructs (WP-2.3b): no gap
        // is recorded, and a monster is built (an id consumed) only with
        // hostile spawning; none joins.
        assert!(
            !hits.keys().any(|k| k.starts_with("EntityList: the ")),
            "no EntityList gap: {hits:?}"
        );
        assert_eq!(
            w.next_entity_id > ids,
            hostile,
            "monster constructors reached only with hostile spawning"
        );
        // `wakeUpAllPlayers` wakes the player (EntityPlayer.wakeUpPlayer,
        // WP-2.5a): no gap recorded.
        assert!(
            !hits
                .keys()
                .any(|k| k.starts_with("EntityPlayer.wakeUpPlayer")),
            "no wakeUpPlayer gap: {hits:?}"
        );
        let player = w.entity(pid).and_then(|e| e.kind.player()).unwrap();
        assert!(!player.is_player_sleeping(), "the player is woken");
    }
}
