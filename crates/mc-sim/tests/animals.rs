//! WP-2.3 task 1: unit checks of `EntitySheep`, `EntityCow`,
//! `EntityChicken` and `EntityWolf` paths the tick fixtures
//! (`tests/golden/entity/{sheep-shear, cow-milk, chicken-*, wolf-*}`) do not
//! reach: the constructors' state, the fleece bits and
//! `getRandomFleeceColor`, the NBT round trips, shears breaking, the
//! inventory slot write, the egg timer, and the tamed wolf, which no fixture
//! can make (the oracle's `HarnessPlayer` has a null username, so taming
//! throws there: `entity::wolf` module docs).

use mc_core::java_random::JavaRandom;
use mc_sim::entity::player::new_entity_player_sp;
use mc_sim::entity::{Entity, EntityKindData, chicken, cow, creature, dispatch, nbt, sheep, wolf};
use mc_sim::item::ItemStack;
use mc_sim::save::NoSave;
use mc_sim::save::entity_list::{FromNbt, create_entity_from_nbt, create_entity_in_world};
use mc_sim::stats::StatList;
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::{World, WorldEnvironment};

/// An empty world with a stone floor at y = 60 over -16..16.
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
    for x in -16..16 {
        for z in -16..16 {
            w.set_block(x, 60, z, 1);
        }
    }
    w
}

fn place(mut e: Entity, x: f64, y: f64, z: f64) -> Entity {
    e.base.set_location_and_angles(x, y, z, 0.0, 0.0);
    e
}

/// A player (not in the world) at `(x, 61, z)` holding `held` in slot 0.
fn player(w: &mut World, x: f64, z: f64, name: Option<&str>, held: Option<ItemStack>) -> Entity {
    let mut p = place(new_entity_player_sp(w), x, 61.0, z);
    let d = p.kind.player_mut().unwrap();
    d.username = name.map(str::to_string);
    d.inventory.main_inventory[0] = held;
    p
}

fn held(p: &Entity) -> Option<ItemStack> {
    p.kind.player().unwrap().inventory.main_inventory[0]
}

fn health(e: &Entity) -> i32 {
    e.kind.living().unwrap().health
}

fn target(e: &Entity) -> Option<i32> {
    creature::creature(&e.kind).unwrap().player_to_attack
}

// ---- constructors ---------------------------------------------------------------------

/// Each constructor draws `EntityLiving`'s four `Math.random()` values and
/// sets its class's size, health and data-watcher slots.
#[test]
fn constructors() {
    let mut w = world();
    let mut r = w.math_random.clone();
    let s = sheep::new_entity_sheep(&mut w);
    for _ in 0..4 {
        r.next_double();
    }
    assert_eq!(w.math_random.state(), r.state());
    assert_eq!((s.base.width, s.base.height), (0.9, 1.3));
    assert_eq!(s.base.data_watcher.get_watchable_object_byte(16), 0);
    assert_eq!(s.kind.living().unwrap().texture, "/mob/sheep.png");

    let c = cow::new_entity_cow(&mut w);
    assert_eq!((c.base.width, c.base.height, health(&c)), (0.9, 1.3, 10));
    assert!(!c.base.data_watcher.watched_objects.contains_key(&16));

    let seed = w.entity_rand_seed_for(w.next_entity_id);
    let ch = chicken::new_entity_chicken(&mut w);
    assert_eq!((ch.base.width, ch.base.height, health(&ch)), (0.3, 0.4, 4));
    // The first draw of the seeded rand (module docs of entity::chicken).
    let mut r = JavaRandom::new(seed);
    assert_eq!(
        chicken::time_until_next_egg(&ch),
        r.next_int_bounded(6000) + 6000
    );
    assert_eq!(ch.base.rand.state(), r.state());
    let EntityKindData::Chicken(.., d) = &ch.kind else {
        panic!()
    };
    assert_eq!(
        (d.wing_rot_delta, d.dest_pos, d.field_753_a),
        (1.0, 0.0, false)
    );

    let wf = wolf::new_entity_wolf(&mut w);
    assert_eq!((wf.base.width, wf.base.height, health(&wf)), (0.8, 0.8, 8));
    assert_eq!(wf.kind.living().unwrap().move_speed, 1.1);
    assert_eq!(wf.base.data_watcher.get_watchable_object_byte(16), 0);
    assert_eq!(wolf::get_wolf_owner(&wf), "");
    // int 18 was added while health was still 0 (EntityWolf.java:27).
    assert_eq!(wf.base.data_watcher.get_watchable_object_int(18), 0);
    for (name, class) in [
        ("Sheep", "EntitySheep"),
        ("Cow", "EntityCow"),
        ("Chicken", "EntityChicken"),
        ("Wolf", "EntityWolf"),
    ] {
        let e = create_entity_in_world(name, &mut w).unwrap();
        assert_eq!(e.kind.java_class(), class);
    }
    assert!(w.unimplemented_hits.borrow().is_empty());
}

// ---- sheep ------------------------------------------------------------------------------

/// `getRandomFleeceColor`: one `nextInt(100)` (black < 5, gray < 10, light
/// gray < 15, brown < 18), else a second `nextInt(500)` (pink on 0, white).
#[test]
fn random_fleece_color() {
    let mut seen = [0; 16];
    for s in 0..20_000i64 {
        let mut r = JavaRandom::new(s);
        let mut check = r.clone();
        let c = sheep::get_random_fleece_color(&mut r);
        let v = check.next_int_bounded(100);
        let want = match v {
            0..=4 => 15,
            5..=9 => 7,
            10..=14 => 8,
            15..=17 => 12,
            _ => {
                if check.next_int_bounded(500) == 0 {
                    6
                } else {
                    0
                }
            }
        };
        assert_eq!(c, want, "seed {s}");
        assert_eq!(r.state(), check.state(), "seed {s}: draws");
        seen[c as usize] += 1;
    }
    for c in [0, 6, 7, 8, 12, 15] {
        assert!(seen[c] > 0, "colour {c} never drawn");
    }
    assert_eq!(seen.iter().sum::<i32>(), 20_000);
}

/// The fleece byte: colour in the low nibble, sheared in bit 4
/// (EntitySheep.java:79-100).
#[test]
fn fleece_bits() {
    let mut w = world();
    let mut s = sheep::new_entity_sheep(&mut w);
    sheep::set_fleece_color(&mut s, 14);
    sheep::set_sheared(&mut s, true);
    assert_eq!(s.base.data_watcher.get_watchable_object_byte(16), 0x1e);
    sheep::set_fleece_color(&mut s, 0x23);
    assert_eq!(sheep::get_fleece_color(&s), 3);
    assert!(sheep::get_sheared(&s));
    sheep::set_sheared(&mut s, false);
    assert_eq!(s.base.data_watcher.get_watchable_object_byte(16), 3);
    assert_eq!(sheep::FLEECE_COLOR_TABLE[15], [0.1, 0.1, 0.1]);
}

/// `Sheared` / `Color` through `EntityList` (EntitySheep.java:55-65).
#[test]
fn sheep_nbt_round_trip() {
    let mut w = world();
    let mut s = place(sheep::new_entity_sheep(&mut w), 1.5, 61.0, 2.5);
    sheep::set_fleece_color(&mut s, 11);
    sheep::set_sheared(&mut s, true);
    let c = nbt::add_entity_id(&s, &w).unwrap();
    assert_eq!(c.get("Color"), Some(&mc_nbt::Tag::Byte(11)));
    assert_eq!(c.get("Sheared"), Some(&mc_nbt::Tag::Byte(1)));
    let FromNbt::Created(back) = create_entity_from_nbt(&c, &mut w).unwrap() else {
        panic!("sheep not constructed");
    };
    assert_eq!(sheep::get_fleece_color(&back), 11);
    assert!(sheep::get_sheared(&back));
    assert_eq!(nbt::add_entity_id(&back, &w).unwrap(), c);
    assert!(w.unimplemented_hits.borrow().is_empty());
}

/// A coloured sheep sheds wool of its colour; worn-out shears drop to size
/// 0 and stay in the slot (ItemStack.damageItem; the break stat is WP-2.7).
#[test]
fn shearing_coloured_sheep_breaks_the_shears() {
    let mut w = world();
    let mut s = place(sheep::new_entity_sheep(&mut w), 0.5, 61.0, 0.5);
    sheep::set_fleece_color(&mut s, 5);
    // Shears: max damage 238 (Item.java:377, ItemShears.java).
    let max = mc_sim::item::item(sheep::SHEARS).unwrap().get_max_damage();
    let mut p = player(
        &mut w,
        3.5,
        0.5,
        None,
        Some(ItemStack::new(sheep::SHEARS, 1, max)),
    );
    let before = w.loaded_entity_list.len();
    assert!(!dispatch::interact(&mut s, &mut w, &mut p));
    assert!(sheep::get_sheared(&s));
    let drops: Vec<_> = w.loaded_entity_list[before..]
        .iter()
        .map(|slot| {
            slot.entity
                .as_ref()
                .unwrap()
                .kind
                .item()
                .unwrap()
                .item
                .unwrap()
        })
        .collect();
    assert!((2..=4).contains(&drops.len()));
    assert!(drops.iter().all(|d| d.item_id == 35 && d.item_damage == 5));
    assert_eq!(held(&p), Some(ItemStack::new(sheep::SHEARS, 0, 0)));
    // ItemStack.java:128-130: the shears' break stat (16973824 + 359).
    let shears_broken = StatList::get().object_break_stat(sheep::SHEARS).unwrap();
    assert_eq!(shears_broken.stat_id, 16973824 + 359);
    assert_eq!(
        p.kind
            .player()
            .unwrap()
            .stat_file_writer
            .write_stat(shears_broken),
        1
    );
    // An unsheared, coloured sheep's death drop.
    let mut s2 = place(sheep::new_entity_sheep(&mut w), 4.5, 61.0, 4.5);
    sheep::set_fleece_color(&mut s2, 9);
    let before = w.loaded_entity_list.len();
    dispatch::drop_few_items(&mut s2, &mut w);
    let it = w.loaded_entity_list[before].entity.as_ref().unwrap();
    assert_eq!(it.kind.item().unwrap().item, Some(ItemStack::new(35, 1, 9)));
}

// ---- cow ----------------------------------------------------------------------------------

/// `InventoryPlayer.setInventorySlotContents`: slots from 36 are armour
/// (InventoryPlayer.java:180-188); the milking writes slot `currentItem`.
#[test]
fn cow_milk_writes_the_current_slot() {
    let mut w = world();
    let mut c = place(cow::new_entity_cow(&mut w), 0.5, 61.0, 0.5);
    let mut p = player(&mut w, 3.5, 0.5, None, None);
    {
        let inv = &mut p.kind.player_mut().unwrap().inventory;
        inv.current_item = 4;
        inv.main_inventory[4] = Some(ItemStack::new(cow::BUCKET_EMPTY, 16, 0));
    }
    assert!(dispatch::interact(&mut c, &mut w, &mut p));
    let inv = &p.kind.player().unwrap().inventory;
    assert_eq!(
        inv.main_inventory[4],
        Some(ItemStack::new(cow::BUCKET_MILK, 1, 0))
    );
    assert_eq!(dispatch::get_drop_item_id(&c, &w), 256 + 78);
    assert_eq!(dispatch::get_sound_volume(&c, &w), 0.4);
}

// ---- chicken ------------------------------------------------------------------------------

/// The egg: at timer 0 an egg item at the feet (two pitch draws first) and a
/// new timer `nextInt(6000) + 6000`; the wings and the slow fall
/// (EntityChicken.java:20-49); no fall damage (:51-52).
#[test]
fn chicken_lays_and_flaps() {
    let mut w = world();
    let mut ch = place(chicken::new_entity_chicken(&mut w), 0.5, 65.0, 0.5);
    ch.base.motion_y = -0.5;
    if let EntityKindData::Chicken(.., d) = &mut ch.kind {
        d.time_until_next_egg = 1;
    }
    let before = w.loaded_entity_list.len();
    chicken::on_living_update(&mut ch, &mut w);
    let it = w.loaded_entity_list[before].entity.as_ref().unwrap();
    assert_eq!(
        it.kind.item().unwrap().item,
        Some(ItemStack::new(chicken::EGG, 1, 0))
    );
    let t = chicken::time_until_next_egg(&ch);
    assert!((6000..12000).contains(&t));
    let EntityKindData::Chicken(.., d) = &ch.kind else {
        panic!()
    };
    // In the air: destPos 0 + 4 * 0.3 clamped to 1; wingRotDelta 1 * 0.9.
    assert_eq!(d.dest_pos, 1.0);
    assert_eq!(d.wing_rot_delta, (1.0f64 * 0.9) as f32);
    assert_eq!(d.wing_rotation, d.wing_rot_delta * 2.0);
    // In the air and falling: motionY scaled by 0.6 after EntityLiving's move.
    assert!(ch.base.motion_y < 0.0);
    // No fall damage, whatever the distance.
    let h = health(&ch);
    dispatch::fall(&mut ch, &mut w, 30.0);
    assert_eq!(health(&ch), h);
    assert_eq!(dispatch::get_drop_item_id(&ch, &w), chicken::FEATHER);
}

// ---- wolf -----------------------------------------------------------------------------

/// The three flags of byte 16, the texture, the tail and the despawn rule.
#[test]
fn wolf_flags_and_render_data() {
    let mut w = world();
    let mut f = wolf::new_entity_wolf(&mut w);
    assert_eq!(dispatch::get_entity_texture(&f, &w), Some("/mob/wolf.png"));
    assert_eq!(wolf::set_tail_rotation(&f), 0.62831855);
    wolf::set_wolf_angry(&mut f, true);
    assert_eq!(
        dispatch::get_entity_texture(&f, &w),
        Some("/mob/wolf_angry.png")
    );
    assert_eq!(wolf::set_tail_rotation(&f), 1.5393804);
    assert_eq!(
        dispatch::get_living_sound(&mut f, &w),
        Some("mob.wolf.growl")
    );
    wolf::set_wolf_sitting(&mut f, true);
    wolf::set_wolf_tamed(&mut f, true);
    assert_eq!(f.base.data_watcher.get_watchable_object_byte(16), 7);
    assert_eq!(
        dispatch::get_entity_texture(&f, &w),
        Some("/mob/wolf_tame.png")
    );
    assert!(!dispatch::can_despawn(&f, &w));
    assert_eq!(dispatch::get_vertical_face_speed(&f, &w), 20);
    assert!(dispatch::is_movement_ceased(&f, &w));
    wolf::set_wolf_angry(&mut f, false);
    wolf::set_wolf_sitting(&mut f, false);
    assert_eq!(f.base.data_watcher.get_watchable_object_byte(16), 4);
    assert_eq!(dispatch::get_vertical_face_speed(&f, &w), 40);
    assert_eq!(dispatch::get_eye_height(&f, &w), 0.8 * 0.8);
    assert_eq!(dispatch::get_max_spawned_in_chunk(&f, &w), 8);
    assert_eq!(dispatch::get_drop_item_id(&f, &w), -1);
    // Tamed, watched health 0: (0.55 - 20 * 0.02) * pi.
    assert_eq!(
        wolf::set_tail_rotation(&f),
        (0.55 - 20.0f32 * 0.02) * 3.1415927
    );
    // handleHealthUpdate(8) starts the shake.
    dispatch::handle_health_update(&mut f, &mut w, 8);
    assert!(wolf::is_movement_ceased(&f));
}

/// `Angry` / `Sitting` / `Owner`; a non-empty owner tames on reading
/// (EntityWolf.java:42-64).
#[test]
fn wolf_nbt_round_trip() {
    let mut w = world();
    let mut f = place(wolf::new_entity_wolf(&mut w), 1.5, 61.0, 1.5);
    wolf::set_wolf_sitting(&mut f, true);
    wolf::set_wolf_owner(&mut f, Some("Notch"));
    let c = nbt::add_entity_id(&f, &w).unwrap();
    assert_eq!(c.get("Owner"), Some(&mc_nbt::Tag::String("Notch".into())));
    assert_eq!(c.get("Sitting"), Some(&mc_nbt::Tag::Byte(1)));
    assert_eq!(c.get("Angry"), Some(&mc_nbt::Tag::Byte(0)));
    let FromNbt::Created(back) = create_entity_from_nbt(&c, &mut w).unwrap() else {
        panic!("wolf not constructed");
    };
    assert!(wolf::is_wolf_tamed(&back));
    assert!(wolf::is_wolf_sitting(&back));
    assert_eq!(wolf::get_wolf_owner(&back), "Notch");
    // An untamed wolf writes "" and reads back untamed.
    let g = place(wolf::new_entity_wolf(&mut w), 2.5, 61.0, 1.5);
    let c = nbt::add_entity_id(&g, &w).unwrap();
    assert_eq!(c.get("Owner"), Some(&mc_nbt::Tag::String(String::new())));
    let FromNbt::Created(back) = create_entity_from_nbt(&c, &mut w).unwrap() else {
        panic!()
    };
    assert!(!wolf::is_wolf_tamed(&back));
}

/// A wolf whose `rand` gives `nextInt(3) == 0` first.
fn wolf_that_tames(w: &mut World) -> Entity {
    let mut f = place(wolf::new_entity_wolf(w), 0.5, 61.0, 0.5);
    let s = (0..)
        .find(|s| JavaRandom::new(*s).next_int_bounded(3) == 0)
        .unwrap();
    f.base.rand.set_seed(s);
    f
}

/// Taming: the bone is eaten, the wolf sits, gets 20 health and the player
/// as owner (EntityWolf.java:328-351).
#[test]
fn wolf_tames_with_a_bone() {
    let mut w = world();
    let mut f = wolf_that_tames(&mut w);
    let mut p = player(
        &mut w,
        2.5,
        0.5,
        Some("Steve"),
        Some(ItemStack::new(wolf::BONE, 1, 0)),
    );
    assert!(dispatch::interact(&mut f, &mut w, &mut p));
    assert!(wolf::is_wolf_tamed(&f) && wolf::is_wolf_sitting(&f));
    assert_eq!(health(&f), 20);
    assert_eq!(wolf::get_wolf_owner(&f), "Steve");
    assert_eq!(held(&p), None, "the last bone is gone from the slot");
}

/// The oracle's case: a player with a null username tames by throwing.
#[test]
#[should_panic(expected = "NullPointerException")]
fn wolf_taming_by_a_nameless_player_throws() {
    let mut w = world();
    let mut f = wolf_that_tames(&mut w);
    let mut p = player(
        &mut w,
        2.5,
        0.5,
        None,
        Some(ItemStack::new(wolf::BONE, 3, 0)),
    );
    dispatch::interact(&mut f, &mut w, &mut p);
}

/// Tamed: favourite meat heals by 3 below a watched health of 20 (the stack
/// eaten like the bone); other food goes on to the owner check, which is
/// case-insensitive and toggles sitting (EntityWolf.java:352-375).
#[test]
fn tamed_wolf_meat_and_sitting() {
    let mut w = world();
    let mut f = place(wolf::new_entity_wolf(&mut w), 0.5, 61.0, 0.5);
    wolf::set_wolf_tamed(&mut f, true);
    wolf::set_wolf_owner(&mut f, Some("Steve"));
    f.kind.living_mut().unwrap().health = 10;
    // watcher 18 is still 0 (< 20).
    let cooked = ItemStack::new(256 + 64, 2, 0);
    let mut p = player(&mut w, 2.5, 0.5, Some("sTEVE"), Some(cooked));
    assert!(dispatch::interact(&mut f, &mut w, &mut p));
    assert_eq!(health(&f), 13);
    assert_eq!(held(&p), Some(ItemStack::new(256 + 64, 1, 0)));
    assert!(!wolf::is_wolf_sitting(&f));
    // Raw fish is food but not wolf meat: the owner check toggles sitting.
    p.kind.player_mut().unwrap().inventory.main_inventory[0] = Some(ItemStack::new(256 + 93, 1, 0));
    f.kind.living_mut().unwrap().is_jumping = true;
    assert!(dispatch::interact(&mut f, &mut w, &mut p));
    assert!(wolf::is_wolf_sitting(&f));
    assert!(!f.kind.living().unwrap().is_jumping);
    assert!(dispatch::interact(&mut f, &mut w, &mut p));
    assert!(!wolf::is_wolf_sitting(&f));
    // Someone else: false.
    let mut q = player(&mut w, 3.5, 0.5, Some("Alex"), None);
    assert!(!dispatch::interact(&mut f, &mut w, &mut q));
    // Full watched health: meat is not eaten.
    f.base
        .data_watcher
        .update_object(18, mc_sim::entity::data_watcher::WatchValue::Int(20));
    p.kind.player_mut().unwrap().inventory.main_inventory[0] = Some(cooked);
    assert!(dispatch::interact(&mut f, &mut w, &mut p));
    assert_eq!(held(&p), Some(cooked));
}

/// `attackEntityFrom`: halving, anger and the pack (EntityWolf.java:254-298).
#[test]
fn wolf_attack_entity_from() {
    let mut w = world();
    let p = player(&mut w, 6.5, 0.5, Some("Steve"), None);
    let pid = p.id();
    assert!(w.spawn_entity_in_world(p));
    let g = place(wolf::new_entity_wolf(&mut w), 3.5, 61.0, 3.5);
    let gid = g.id();
    w.spawn_entity_in_world(g);
    let f = place(wolf::new_entity_wolf(&mut w), 0.5, 61.0, 0.5);
    let fid = f.id();
    w.spawn_entity_in_world(f);
    let player_copy = w.entity(pid).unwrap().clone();
    // A pig attacker: damage 3 -> 2, no anger, the pack targets the pig.
    let pig = place(mc_sim::entity::pig::new_entity_pig(&mut w), 1.5, 61.0, 0.5);
    let pig_id = pig.id();
    w.with_entity_mut(fid, |w, f| {
        wolf::set_wolf_sitting(f, true);
        assert!(dispatch::attack_entity_from(f, w, Some(&pig), 3));
        assert_eq!(health(f), 6);
        assert!(!wolf::is_wolf_sitting(f));
        assert!(!wolf::is_wolf_angry(f));
        assert_eq!(target(f), Some(pig_id));
    })
    .unwrap();
    assert_eq!(target(w.entity(gid).unwrap()), Some(pig_id));
    // A player attacker on the other wolf: not halved, angry, targets it.
    w.with_entity_mut(gid, |w, g| {
        assert!(dispatch::attack_entity_from(g, w, Some(&player_copy), 3));
        assert_eq!(health(g), 5);
        assert!(wolf::is_wolf_angry(g));
        assert_eq!(target(g), Some(pid));
        assert_eq!(dispatch::find_player_to_attack(g, w), Some(pid));
    })
    .unwrap();
    // Wolf f already had a target: not alerted, not angry.
    assert!(!wolf::is_wolf_angry(w.entity(fid).unwrap()));
    // A tamed wolf hit by its owner keeps no grudge; by another player it does.
    let mut t = place(wolf::new_entity_wolf(&mut w), 9.5, 61.0, 9.5);
    wolf::set_wolf_tamed(&mut t, true);
    wolf::set_wolf_owner(&mut t, Some("steve"));
    assert!(dispatch::attack_entity_from(
        &mut t,
        &mut w,
        Some(&player_copy),
        1
    ));
    assert_eq!(target(&t), None);
    let other = player(&mut w, 8.5, 9.5, Some("Alex"), None);
    t.base.hearts_life = 0;
    assert!(dispatch::attack_entity_from(
        &mut t,
        &mut w,
        Some(&other),
        1
    ));
    assert_eq!(target(&t), Some(other.id()));
}

/// `attackEntity`: a bite within 1.5 blocks (2, or 4 tamed) sets
/// `attackTime` 20 (EntityWolf.java:304-324).
#[test]
fn wolf_bites() {
    let mut w = world();
    let pig = place(mc_sim::entity::pig::new_entity_pig(&mut w), 1.5, 61.0, 0.5);
    let pig_id = pig.id();
    w.spawn_entity_in_world(pig);
    let mut f = place(wolf::new_entity_wolf(&mut w), 0.5, 61.0, 0.5);
    wolf::set_wolf_tamed(&mut f, true);
    dispatch::attack_entity(&mut f, &mut w, pig_id, 1.0);
    assert_eq!(f.kind.living().unwrap().attack_time, 20);
    assert_eq!(health(w.entity(pig_id).unwrap()), 6);
}

/// A tamed wolf follows its owner: a path within reach, a teleport to the
/// rim of the owner's 5 × 5 square when there is none beyond 12 blocks; with
/// the owner gone it sits (EntityWolf.java:96-107, :229-248).
#[test]
fn tamed_wolf_follows_or_teleports() {
    let mut w = world();
    let p = player(&mut w, 0.5, 0.5, Some("Steve"), None);
    let pid = p.id();
    w.spawn_entity_in_world(p);
    // Reachable: a path.
    let mut f = place(wolf::new_entity_wolf(&mut w), 8.5, 61.0, 0.5);
    wolf::get_path_or_walkable_block(&mut f, &mut w, pid, 8.0);
    assert!(creature::has_path(&f));
    // On a stone pillar 20 blocks away, walled off by air: no path.
    for y in 61..70 {
        w.set_block(-20, y, 0, 1);
    }
    let mut g = place(wolf::new_entity_wolf(&mut w), -19.5, 70.0, 0.5);
    let y0 = g.base.pos_y;
    wolf::get_path_or_walkable_block(&mut g, &mut w, pid, 20.0);
    assert!(!creature::has_path(&g));
    assert_ne!(g.base.pos_y, y0, "teleported");
    // The first rim cell, x outer, z inner: (-2, 61, -2) + 0.5.
    assert_eq!((g.base.pos_x, g.base.pos_z), (-1.5, -1.5));
    // No owner in the world: sits down.
    let mut h = place(wolf::new_entity_wolf(&mut w), 3.5, 61.0, 3.5);
    wolf::set_wolf_tamed(&mut h, true);
    wolf::set_wolf_owner(&mut h, Some("Nobody"));
    dispatch::update_player_action_state(&mut h, &mut w);
    assert!(wolf::is_wolf_sitting(&h));
    assert_eq!(h.base.data_watcher.get_watchable_object_int(18), 8);
}
