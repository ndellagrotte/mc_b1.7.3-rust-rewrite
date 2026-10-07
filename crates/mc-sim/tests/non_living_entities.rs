//! WP-2.4 (task 1): unit checks of the non-living entities that the tick
//! fixtures (`tests/golden/entity/*`) cannot reach: `catchFish` (only the
//! rod's right click calls it, WP-2.6), the NBT round trip of the kinds
//! `EntityList` saves, the lightning bolt's fire at difficulty 2, the
//! `(World, EntityLiving)` arrow, the fireball's deflection, `EnumArt`.

use mc_core::java_random::JavaRandom;
use mc_core::math;
use mc_sim::entity::painting::{self, EnumArt};
use mc_sim::entity::player::new_entity_player_sp;
use mc_sim::entity::{
    Entity, EntityKindData, arrow, dispatch, fireball, fish, item, lightning_bolt, nbt, snowball,
    tnt_primed,
};
use mc_sim::item::ItemStack;
use mc_sim::save::NoSave;
use mc_sim::save::entity_list::{FromNbt, create_entity_from_nbt};
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
    for dz in -1..=1 {
        for dx in -1..=1 {
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

/// A player on the platform at (0.5, 61, 0.5) facing +x, holding a rod.
fn angler(w: &mut World) -> Entity {
    let mut p = new_entity_player_sp(w);
    p.base.set_location_and_angles(0.5, 61.0, 0.5, -90.0, 30.0);
    let pd = p.kind.player_mut().unwrap();
    pd.inventory.main_inventory[0] = Some(ItemStack::new(256 + 90, 1, 0));
    p
}

fn fish_data(e: &Entity) -> &mc_sim::entity::FishData {
    match &e.kind {
        EntityKindData::Fish(d) => d,
        _ => panic!(),
    }
}

/// `catchFish` with a fish on (`ticksCatchable > 0`, no bobber): a raw fish
/// flies from the bobber toward the angler, result 1; the bobber dies and
/// the angler's `fishEntity` is cleared (EntityFish.java:361-382).
#[test]
fn catch_fish_with_a_fish_on_throws_a_raw_fish() {
    let mut w = empty_world(4);
    let mut p = angler(&mut w);
    let mut f = fish::new_entity_fish(&mut w, &mut p);
    assert_eq!(p.kind.player().unwrap().fish_entity, Some(f.base.entity_id));
    f.base.set_position(4.5, 61.0, 0.5);
    if let EntityKindData::Fish(d) = &mut f.kind {
        d.ticks_catchable = 12;
    }
    let item_id = w.next_entity_id;
    let r = fish::catch_fish(&mut f, &mut w, &mut p);
    assert_eq!(r, 1);
    assert!(f.base.is_dead);
    assert_eq!(p.kind.player().unwrap().fish_entity, None);
    let it = w.entity(item_id).expect("the raw fish joined");
    assert_eq!(
        it.kind.item().unwrap().item,
        Some(ItemStack::new(256 + 93, 1, 0))
    );
    let (dx, dy, dz) = (
        p.base.pos_x - f.base.pos_x,
        p.base.pos_y - f.base.pos_y,
        p.base.pos_z - f.base.pos_z,
    );
    let d = math::sqrt_double(dx * dx + dy * dy + dz * dz) as f64;
    assert_eq!(it.base.motion_x, dx * 0.1);
    assert_eq!(
        it.base.motion_y,
        dy * 0.1 + math::sqrt_double(d) as f64 * 0.08
    );
    assert_eq!(it.base.motion_z, dz * 0.1);
    let caught = mc_sim::stats::stat(mc_sim::stats::stat_list::FISH_CAUGHT_STAT).unwrap();
    assert_eq!(
        p.kind.player().unwrap().stat_file_writer.write_stat(caught),
        1
    );
}

/// `catchFish` with a hooked entity pulls it toward the angler (result 3);
/// in the ground the result is 2 whatever came before (EntityFish.java:348-378).
#[test]
fn catch_fish_pulls_the_hooked_entity_and_reports_the_ground() {
    let mut w = empty_world(4);
    let mut p = angler(&mut w);
    let hooked = item::new_entity_item(&mut w, 3.5, 61.0, 0.5, ItemStack::new(4, 1, 0));
    let hid = hooked.base.entity_id;
    let before = hooked.base.motion_y;
    assert!(w.spawn_entity_in_world(hooked));
    let mut f = fish::new_entity_fish(&mut w, &mut p);
    if let EntityKindData::Fish(d) = &mut f.kind {
        d.bobber = Some(hid);
    }
    assert_eq!(fish::catch_fish(&mut f, &mut w, &mut p), 3);
    assert!(w.entity(hid).unwrap().base.motion_y > before);

    let mut f = fish::new_entity_fish(&mut w, &mut p);
    if let EntityKindData::Fish(d) = &mut f.kind {
        d.in_ground = true;
        d.ticks_catchable = 3;
    }
    let n = w.loaded_entity_list.len();
    assert_eq!(fish::catch_fish(&mut f, &mut w, &mut p), 2);
    // The fish was still thrown (result 1 overwritten by 2).
    assert_eq!(w.loaded_entity_list.len(), n + 1);
}

/// The bobber's constructor: at the angler's eyes (posY + 1.62 - yOffset),
/// 0.16 aside, then `calculateVelocity` with three Gaussian draws on its
/// `rand` seeded `mix(seed, id)` (EntityFish.java:48-74, :85-103).
#[test]
fn fish_constructor_heading() {
    let mut w = empty_world(9);
    let mut p = angler(&mut w);
    let id = w.next_entity_id;
    let f = fish::new_entity_fish(&mut w, &mut p);
    assert_eq!(fish_data(&f).angler, Some(p.base.entity_id));
    let mut r = JavaRandom::new(w.entity_rand_seed_for(id));
    let (yaw, pitch) = (-90.0f32, 30.0f32);
    let mx =
        (-math::sin(yaw / 180.0 * 3.1415927) * math::cos(pitch / 180.0 * 3.1415927) * 0.4) as f64;
    let mz =
        (math::cos(yaw / 180.0 * 3.1415927) * math::cos(pitch / 180.0 * 3.1415927) * 0.4) as f64;
    let my = (-math::sin(pitch / 180.0 * 3.1415927) * 0.4) as f64;
    let n = math::sqrt_double(mx * mx + my * my + mz * mz) as f64;
    let mut m = [mx / n, my / n, mz / n];
    for v in &mut m {
        *v += r.next_gaussian() * 0.007499999832361937;
        *v *= 1.5;
    }
    assert_eq!([f.base.motion_x, f.base.motion_y, f.base.motion_z], m);
    assert_eq!(
        f.base.pos_y,
        p.base.pos_y + 1.62 - p.base.y_offset as f64 - 0.10000000149011612
    );
    assert_eq!(f.base.y_offset, 0.0);
}

/// The kinds `EntityList` names come back from their own NBT
/// (`createEntityFromNBT`: the `(World)` constructor, then `readFromNBT`).
#[test]
fn nbt_round_trip_of_the_saved_kinds() {
    let mut w = empty_world(2);
    for x in -4..4 {
        for y in 61..66 {
            w.set_block(x, y, 3, 1);
        }
    }
    let mut tnt = tnt_primed::new_entity_tnt_primed(&mut w, 0.5, 61.5, 0.5);
    if let EntityKindData::TntPrimed(d) = &mut tnt.kind {
        d.fuse = -3; // a byte, read back sign-extended
    }
    let mut arr = arrow::new_entity_arrow_at(&mut w, 1.0, 62.0, 1.0);
    if let EntityKindData::Arrow(d) = &mut arr.kind {
        d.x_tile = 2;
        d.y_tile = 60;
        d.z_tile = -1;
        d.in_tile = 1;
        d.in_data = 0;
        d.in_ground = true;
        d.arrow_shake = 4;
        d.does_arrow_belong_to_player = true;
    }
    let mut sb = snowball::new_entity_snowball_at(&mut w, 1.0, 63.0, 2.0);
    if let EntityKindData::Snowball(d) = &mut sb.kind {
        d.shake = 2;
        d.x_tile = 5;
    }
    let pt = painting::new_entity_painting(&mut w, 0, 63, 3, 0);
    for e in [tnt, arr, sb, pt] {
        let c = nbt::add_entity_id(&e, &w).expect("a named, live entity");
        let FromNbt::Created(back) = create_entity_from_nbt(&c, &mut w).unwrap() else {
            panic!("{} not constructed", e.kind.java_class());
        };
        assert_eq!(back.kind.java_class(), e.kind.java_class());
        let again = nbt::add_entity_id(&back, &w).unwrap();
        assert_eq!(again, c, "{}", e.kind.java_class());
        match (&e.kind, &back.kind) {
            (EntityKindData::TntPrimed(a), EntityKindData::TntPrimed(b)) => assert_eq!(a, b),
            (EntityKindData::Arrow(a), EntityKindData::Arrow(b)) => {
                assert_eq!(
                    (a.x_tile, a.y_tile, a.z_tile),
                    (b.x_tile, b.y_tile, b.z_tile)
                );
                assert_eq!((a.in_ground, a.arrow_shake), (b.in_ground, b.arrow_shake));
                assert!(b.does_arrow_belong_to_player);
            }
            (EntityKindData::Snowball(a), EntityKindData::Snowball(b)) => {
                assert_eq!((a.shake, a.x_tile), (b.shake, b.x_tile));
            }
            (EntityKindData::Painting(a), EntityKindData::Painting(b)) => {
                assert_eq!(a.art, b.art);
                assert_eq!(a.direction, b.direction);
                assert_eq!(e.base.bounding_box, back.base.bounding_box);
                assert_eq!(e.base.pos_x, back.base.pos_x);
            }
            _ => panic!(),
        }
    }
    assert!(w.unimplemented_hits.borrow().is_empty());
}

/// A painting's motive: an unknown title reads back as `Kebab`
/// (EntityPainting.java:238-240); `EnumArt` has 25 motives, the longest
/// title 13 characters.
#[test]
fn painting_motive_fallback_and_enum_art() {
    let mut w = empty_world(2);
    let pt = painting::new_entity_painting(&mut w, 0, 63, 3, 0);
    let mut c = nbt::add_entity_id(&pt, &w).unwrap();
    c.insert(
        "Motive".to_string(),
        mc_nbt::Tag::String("Mona".to_string()),
    );
    let FromNbt::Created(back) = create_entity_from_nbt(&c, &mut w).unwrap() else {
        panic!()
    };
    let EntityKindData::Painting(d) = &back.kind else {
        panic!()
    };
    assert_eq!(d.art, Some(EnumArt::Kebab));
    assert_eq!(EnumArt::VALUES.len(), 25);
    assert_eq!(EnumArt::MAX_ART_TITLE_LENGTH, 13);
    assert_eq!(EnumArt::DonkeyKong.def().size_y, 48);
}

/// At difficulty 2 the bolt's constructor sets fire at the strike and at
/// four blocks jittered by `rand.nextInt(3) - 1` after its two shape draws
/// (EntityLightningBolt.java:14-32); at 1 it places none.
#[test]
fn lightning_constructor_fire_by_difficulty() {
    for difficulty in [1, 2] {
        let mut w = empty_world(6);
        w.difficulty_setting = difficulty;
        let id = w.next_entity_id;
        let bolt = lightning_bolt::new_entity_lightning_bolt(&mut w, 0.5, 61.0, 0.5);
        let mut r = JavaRandom::new(w.entity_rand_seed_for(id));
        let vertex = r.next_long();
        let living = r.next_int_bounded(3) + 1;
        let EntityKindData::LightningBolt(_, d) = &bolt.kind else {
            panic!()
        };
        assert_eq!(
            (d.lightning_state, d.bolt_vertex, d.bolt_living_time),
            (2, vertex, living)
        );
        let mut fires = Vec::new();
        for x in -2..=2 {
            for y in 60..=62 {
                for z in -2..=2 {
                    if w.get_block_id(x, y, z) == 51 {
                        fires.push((x, y, z));
                    }
                }
            }
        }
        if difficulty < 2 {
            assert!(fires.is_empty());
            continue;
        }
        assert!(fires.contains(&(0, 61, 0)));
        let mut want = vec![(0, 61, 0)];
        for _ in 0..4 {
            let p = (
                r.next_int_bounded(3) - 1,
                61 + r.next_int_bounded(3) - 1,
                r.next_int_bounded(3) - 1,
            );
            // Air with fire allowed: on top of the stone platform (y 61).
            if p.1 == 61 && !want.contains(&p) {
                want.push(p);
            }
        }
        want.sort();
        fires.sort();
        assert_eq!(fires, want);
        assert_eq!(bolt.base.rand.state(), r.state());
    }
}

/// `EntityArrow(World, EntityLiving)`: from the shooter's eyes, 0.16 aside
/// and 0.1 down, along its look at speed 1.5 with the heading jitter; owned
/// by the shooter and a player's arrow (EntityArrow.java:30-45).
#[test]
fn arrow_shot_by_a_player() {
    let mut w = empty_world(8);
    let mut p = new_entity_player_sp(&mut w);
    p.base.set_location_and_angles(0.5, 61.0, 0.5, 30.0, -15.0);
    let id = w.next_entity_id;
    let a = arrow::new_entity_arrow_shot(&mut w, &p);
    let EntityKindData::Arrow(d) = &a.kind else {
        panic!()
    };
    assert_eq!(d.owner, Some(p.base.entity_id));
    assert!(d.does_arrow_belong_to_player);
    let eye = dispatch::get_eye_height(&p, &w) as f64;
    let yaw = 30.0f32;
    assert_eq!(
        a.base.pos_x,
        p.base.pos_x - (math::cos(yaw / 180.0 * 3.1415927) * 0.16) as f64
    );
    assert_eq!(a.base.pos_y, p.base.pos_y + eye - 0.10000000149011612);
    let mut r = JavaRandom::new(w.entity_rand_seed_for(id));
    r.next_gaussian();
    r.next_gaussian();
    r.next_gaussian();
    assert_eq!(a.base.rand.state(), r.state());
    let speed =
        (a.base.motion_x.powi(2) + a.base.motion_y.powi(2) + a.base.motion_z.powi(2)).sqrt();
    assert!((speed - 1.5).abs() < 0.05);
    assert_eq!(a.base.rotation_yaw, a.base.prev_rotation_yaw);
}

/// `EntityFireball.attackEntityFrom`: a living attacker sends it along its
/// look (`getLook(1.0F)`) with a tenth of it as acceleration, true; no
/// attacker leaves it, false (EntityFireball.java:202-219).
#[test]
fn fireball_deflected_by_a_living_attacker() {
    let mut w = empty_world(3);
    let mut p = new_entity_player_sp(&mut w);
    p.base.set_location_and_angles(0.5, 61.0, 0.5, 45.0, 10.0);
    let mut fb = fireball::new_entity_fireball_empty(&mut w);
    assert!(!dispatch::attack_entity_from(&mut fb, &mut w, None, 1));
    assert_eq!(fb.base.motion_x, 0.0);
    assert!(dispatch::attack_entity_from(&mut fb, &mut w, Some(&p), 1));
    let (yaw, pitch) = (45.0f32, 10.0f32);
    let v2 = math::cos(-yaw * 0.017453292 - 3.1415927);
    let v3 = math::sin(-yaw * 0.017453292 - 3.1415927);
    let v4 = -math::cos(-pitch * 0.017453292);
    let v5 = math::sin(-pitch * 0.017453292);
    let look = [(v3 * v4) as f64, v5 as f64, (v2 * v4) as f64];
    assert_eq!([fb.base.motion_x, fb.base.motion_y, fb.base.motion_z], look);
    let EntityKindData::Fireball(d) = &fb.kind else {
        panic!()
    };
    assert_eq!(d.acceleration_x, look[0] * 0.1);
    assert!(fb.base.been_attacked);
    assert!(dispatch::can_be_collided_with(&fb, &w));
}
