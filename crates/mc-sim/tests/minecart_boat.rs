//! WP-2.4 (task 2): unit checks of `EntityMinecart` and `EntityBoat` that the
//! tick fixtures (`tests/golden/entity/minecart-*`, `boat-*`) cannot reach:
//! a chest cart's cargo drop (no harness op fills its inventory), the NBT
//! round trip, the `IInventory` methods, `interact` toggling the rider, the
//! furnace cart's coal, and the boat's rider position and break with a rider.

use mc_core::java_random::JavaRandom;
use mc_core::math::jmath;
use mc_sim::entity::player::new_entity_player_sp;
use mc_sim::entity::{
    BoatData, Entity, EntityKindData, MinecartData, boat, dispatch, minecart, nbt,
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

fn cart(e: &Entity) -> &MinecartData {
    match &e.kind {
        EntityKindData::Minecart(d) => d,
        _ => panic!("not a minecart"),
    }
}

fn cart_mut(e: &mut Entity) -> &mut MinecartData {
    match &mut e.kind {
        EntityKindData::Minecart(d) => d,
        _ => panic!("not a minecart"),
    }
}

fn boat_data(e: &Entity) -> &BoatData {
    match &e.kind {
        EntityKindData::Boat(d) => d,
        _ => panic!("not a boat"),
    }
}

/// `(itemID, stackSize)` of every `EntityItem` in `loadedEntityList`, in
/// list order.
fn items(w: &World) -> Vec<(i32, i32)> {
    w.loaded_entity_list
        .iter()
        .filter_map(|s| s.entity.as_ref()?.kind.item()?.item)
        .map(|s| (s.item_id, s.stack_size))
        .collect()
}

/// A chest cart broken by a hit: `setEntityDead` drops the cargo in stacks of
/// 10-30 (three `rand` floats per slot, `nextInt(21)` and three gaussians per
/// stack), leaving size-0 stacks in the slots; then a minecart; then
/// `attackEntityFrom` walks the slots again, three floats per non-null slot
/// and nothing dropped (EntityMinecart.java:84-113, :136-162); then a chest.
#[test]
fn chest_cart_drops_its_cargo_then_walks_the_emptied_slots_again() {
    let mut w = empty_world(7);
    let mut c = minecart::new_entity_minecart(&mut w, 0.5, 61.0, 0.5, 1);
    cart_mut(&mut c).cargo_items[0] = Some(ItemStack::new(4, 40, 0));
    cart_mut(&mut c).cargo_items[5] = Some(ItemStack::new(280, 3, 0));
    let mut r: JavaRandom = c.base.rand.clone();

    assert!(dispatch::attack_entity_from(&mut c, &mut w, None, 5));
    assert!(c.base.is_dead);

    let mut want = Vec::new();
    for (id, size) in [(4, 40), (280, 3)] {
        for _ in 0..3 {
            r.next_float();
        }
        let mut left = size;
        while left > 0 {
            let n = (r.next_int_bounded(21) + 10).min(left);
            left -= n;
            want.push((id, n));
            for _ in 0..3 {
                r.next_gaussian();
            }
        }
    }
    want.push((minecart::ITEM_MINECART_EMPTY, 1));
    for _ in 0..2 * 3 {
        r.next_float();
    }
    want.push((54, 1));
    assert_eq!(r.state(), c.base.rand.state(), "the second walk's draws");
    assert_eq!(items(&w), want);
    // The emptied stacks stay in their slots at size 0.
    assert_eq!(cart(&c).cargo_items[0].map(|s| s.stack_size), Some(0));
    assert_eq!(cart(&c).cargo_items[5].map(|s| s.stack_size), Some(0));
}

/// Below 41 damage a hit only wobbles the cart; the damage heals one a tick
/// in `onUpdate`.
#[test]
fn cart_hit_wobbles_and_heals() {
    let mut w = empty_world(8);
    let mut c = minecart::new_entity_minecart(&mut w, 0.5, 61.0, 0.5, 0);
    assert!(dispatch::attack_entity_from(&mut c, &mut w, None, 2));
    let d = cart(&c);
    assert_eq!(
        (
            d.minecart_current_damage,
            d.minecart_time_since_hit,
            d.minecart_rock_direction
        ),
        (20, 10, -1)
    );
    assert!(c.base.been_attacked && !c.base.is_dead);
    dispatch::on_update(&mut c, &mut w);
    let d = cart(&c);
    assert_eq!(
        (d.minecart_current_damage, d.minecart_time_since_hit),
        (19, 9)
    );
    assert!(items(&w).is_empty());
}

/// NBT: `Type`; a furnace cart's `PushX`/`PushZ`/`Fuel`; a chest cart's
/// `Items`, read back into 27 slots (a slot past 26 is dropped); a boat
/// saves nothing of its own. `EntityList` constructs both classes.
#[test]
fn nbt_round_trip_of_carts_and_boats() {
    let mut w = empty_world(9);

    let mut f = minecart::new_entity_minecart(&mut w, 1.5, 61.0, 2.5, 2);
    {
        let d = cart_mut(&mut f);
        d.push_x = 0.25;
        d.push_z = -0.5;
        d.fuel = 1199;
    }
    let c = nbt::add_entity_id(&f, &w).expect("Minecart is in EntityList");
    let FromNbt::Created(g) = create_entity_from_nbt(&c, &mut w).unwrap() else {
        panic!("Minecart is constructed")
    };
    let d = cart(&g);
    assert_eq!(
        (d.minecart_type, d.push_x, d.push_z, d.fuel),
        (2, 0.25, -0.5, 1199)
    );
    assert_eq!((g.base.pos_x, g.base.pos_z), (1.5, 2.5));

    let mut ch = minecart::new_entity_minecart(&mut w, 0.5, 61.0, 0.5, 1);
    cart_mut(&mut ch).cargo_items[3] = Some(ItemStack::new(5, 12, 0));
    cart_mut(&mut ch).cargo_items[30] = Some(ItemStack::new(4, 1, 0));
    let c = nbt::add_entity_id(&ch, &w).unwrap();
    let FromNbt::Created(g) = create_entity_from_nbt(&c, &mut w).unwrap() else {
        panic!()
    };
    let d = cart(&g);
    assert_eq!(d.cargo_items.len(), 27);
    assert_eq!(d.cargo_items[3], Some(ItemStack::new(5, 12, 0)));
    assert_eq!(d.cargo_items.iter().flatten().count(), 1);

    let plain = minecart::new_entity_minecart_empty(&mut w);
    let c = nbt::write_to_nbt(&plain, &w);
    assert!(c.contains_key("Type") && !c.contains_key("Items") && !c.contains_key("Fuel"));

    let mut b = boat::new_entity_boat(&mut w, -2.5, 61.0, 3.5);
    b.base.rotation_yaw = 45.0;
    let c = nbt::add_entity_id(&b, &w).expect("Boat is in EntityList");
    let FromNbt::Created(g) = create_entity_from_nbt(&c, &mut w).unwrap() else {
        panic!("Boat is constructed")
    };
    assert_eq!(boat_data(&g), &BoatData::default());
    assert_eq!(
        (
            g.base.pos_x,
            g.base.pos_y,
            g.base.pos_z,
            g.base.rotation_yaw
        ),
        (-2.5, 61.0 + 0.3f32 as f64, 3.5, 45.0)
    );
}

/// `decrStackSize` / `setInventorySlotContents` (EntityMinecart.java:724-750).
#[test]
fn cart_inventory_methods() {
    let mut d = MinecartData::default();
    assert_eq!(d.get_size_inventory(), 27);
    assert_eq!(d.cargo_items.len(), 36);
    d.set_inventory_slot_contents(2, Some(ItemStack::new(4, 70, 0)));
    assert_eq!(d.get_stack_in_slot(2).map(|s| s.stack_size), Some(64));
    assert_eq!(d.decr_stack_size(2, 10), Some(ItemStack::new(4, 10, 0)));
    assert_eq!(d.get_stack_in_slot(2).map(|s| s.stack_size), Some(54));
    assert_eq!(d.decr_stack_size(2, 54), Some(ItemStack::new(4, 54, 0)));
    assert_eq!(d.get_stack_in_slot(2), None);
    assert_eq!(d.decr_stack_size(2, 1), None);
    assert_eq!(d.get_inv_name(), "Minecart");
}

/// `interact` on an empty cart or a boat: the player gets in, and out again
/// on the second click (`mountEntity` on its own mount, Entity.java:992-995).
#[test]
fn interact_toggles_the_rider() {
    let mut w = empty_world(10);
    let mut p = new_entity_player_sp(&mut w);
    p.base.set_location_and_angles(0.5, 61.0, 2.5, 0.0, 0.0);
    for mut v in [
        minecart::new_entity_minecart(&mut w, 0.5, 61.0, 0.5, 0),
        boat::new_entity_boat(&mut w, 0.5, 61.0, 0.5),
    ] {
        assert!(dispatch::interact(&mut v, &mut w, &mut p));
        assert_eq!(p.base.riding_entity, Some(v.base.entity_id));
        assert_eq!(v.base.ridden_by_entity, Some(p.base.entity_id));
        assert!(dispatch::interact(&mut v, &mut w, &mut p));
        assert_eq!(
            (p.base.riding_entity, v.base.ridden_by_entity),
            (None, None)
        );
        // Put off on top of the vehicle's box.
        assert_eq!(
            p.base.pos_y,
            v.base.bounding_box.min_y + v.base.height as f64 + p.base.y_offset as f64
        );
    }
}

/// A furnace cart takes one coal (the last one empties the slot) for 1200
/// fuel; with or without coal it is pushed away from the player.
#[test]
fn furnace_cart_takes_coal_and_a_push() {
    let mut w = empty_world(11);
    let mut p = new_entity_player_sp(&mut w);
    p.base.set_location_and_angles(-2.5, 61.0, 0.5, 0.0, 0.0);
    p.kind.player_mut().unwrap().inventory.main_inventory[0] =
        Some(ItemStack::new(mc_sim::item::ids::COAL, 1, 0));
    let mut f = minecart::new_entity_minecart(&mut w, 0.5, 61.0, 1.5, 2);
    assert!(dispatch::interact(&mut f, &mut w, &mut p));
    assert_eq!(p.kind.player().unwrap().inventory.main_inventory[0], None);
    let d = cart(&f);
    assert_eq!((d.fuel, d.push_x, d.push_z), (1200, 3.0, 1.0));
    assert!(dispatch::interact(&mut f, &mut w, &mut p));
    assert_eq!(cart(&f).fuel, 1200);
    assert_eq!(p.base.riding_entity, None);
}

/// `EntityBoat.updateRiderPosition`: 0.4 ahead along the yaw, at the mounted
/// offset plus the rider's own (`EntityPlayer.getYOffset`: `yOffset - 0.5`,
/// EntityPlayer.java:479-481).
#[test]
fn boat_places_its_rider_ahead() {
    let mut w = empty_world(12);
    let mut b = boat::new_entity_boat(&mut w, 0.5, 61.0, 0.5);
    b.base.rotation_yaw = 30.0;
    let mut p = new_entity_player_sp(&mut w);
    dispatch::update_rider_position(&b, &mut p, &w);
    let a = 30.0f32 as f64 * 3.141592653589793 / 180.0;
    assert_eq!(p.base.pos_x, b.base.pos_x + jmath::cos(a) * 0.4);
    assert_eq!(p.base.pos_z, b.base.pos_z + jmath::sin(a) * 0.4);
    assert_eq!(
        p.base.pos_y,
        b.base.pos_y + boat::get_mounted_y_offset(&b) + (p.base.y_offset - 0.5) as f64
    );
    assert_eq!(boat::get_mounted_y_offset(&b), -0.30000001192092896);
}

/// A boat broken by a hit with a rider: the rider (in the world) is put off,
/// 3 planks and 2 sticks drop, then the boat dies (EntityBoat.java:63-90).
#[test]
fn boat_broken_with_a_rider() {
    let mut w = empty_world(13);
    let p = new_entity_player_sp(&mut w);
    let pid = p.base.entity_id;
    assert!(w.spawn_entity_in_world(p));
    let mut b = boat::new_entity_boat(&mut w, 0.5, 61.0, 0.5);
    w.with_entity_mut(pid, |w, p| dispatch::interact(&mut b, w, p))
        .unwrap();
    assert_eq!(b.base.ridden_by_entity, Some(pid));
    assert!(dispatch::attack_entity_from(&mut b, &mut w, None, 3));
    assert!(!b.base.is_dead);
    assert!(dispatch::attack_entity_from(&mut b, &mut w, None, 2));
    assert!(b.base.is_dead);
    assert_eq!(b.base.ridden_by_entity, None);
    assert_eq!(w.entity(pid).unwrap().base.riding_entity, None);
    assert_eq!(
        items(&w),
        vec![
            (5, 1),
            (5, 1),
            (5, 1),
            (boat::ITEM_STICK, 1),
            (boat::ITEM_STICK, 1)
        ]
    );
}
