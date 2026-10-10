//! WP-2.1: `Entity` base bodies that no transcribed kind reaches in a fixture
//! yet, checked on hand-built entities against values derived from the Java
//! by hand (`entity/item-collision-pair` explains why a fixture cannot reach
//! `applyEntityCollision`: `EntityItem` never calls it).

use mc_core::Vec3D;
use mc_core::math;
use mc_sim::block::Material;
use mc_sim::entity::item::new_entity_item;
use mc_sim::entity::{Entity, base, dispatch};
use mc_sim::item::ItemStack;
use mc_sim::save::NoSave;
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::{World, WorldEnvironment};

fn empty_world() -> World {
    let mut w = World::new(
        Box::new(NoSave),
        "harness",
        WorldProvider::new(Dimension::Surface),
        123,
        ChunkGenerator::Empty(EmptyChunkProvider),
        WorldEnvironment {
            rand_seed: 0,
            field_9437_g: 0,
            lock_timestamp: 0,
        },
    );
    w.make_deterministic(123);
    for dz in -2..=2 {
        for dx in -2..=2 {
            w.get_chunk_from_chunk_coords(dx, dz);
        }
    }
    // A stone floor at y = 60 over (-4..4, -4..4).
    for x in -4..=4 {
        for z in -4..=4 {
            w.set_block(x, 60, z, 1);
        }
    }
    w
}

/// A cobblestone `EntityItem` at `(x, y, z)` (box centre) at rest.
fn item_at(w: &mut World, x: f64, y: f64, z: f64) -> Entity {
    let mut e = new_entity_item(w, x, y, z, ItemStack::new(4, 1, 0));
    e.base.set_velocity(0.0, 0.0, 0.0);
    e
}

// src: Entity.java:709-734
#[test]
fn apply_entity_collision_pushes_apart_by_the_abs_max_distance() {
    let mut w = empty_world();
    let mut a = item_at(&mut w, 0.0, 70.0, 0.0);
    let mut b = item_at(&mut w, 0.5, 70.0, 0.2);
    base::apply_entity_collision(&mut a, &mut w, &mut b);
    // var6 = abs_max(0.5, 0.2) = 0.5, then sqrt as a float; 1/var6 > 1 is capped at 1.
    let var6 = math::sqrt_double(0.5) as f64;
    let push_x = 0.5 / var6 * 1.0 * 0.05000000074505806 * 1.0;
    let push_z = 0.2 / var6 * 1.0 * 0.05000000074505806 * 1.0;
    assert_eq!(a.base.motion_x, -push_x);
    assert_eq!(a.base.motion_z, -push_z);
    assert_eq!(b.base.motion_x, push_x);
    assert_eq!(b.base.motion_z, push_z);
    assert_eq!(a.base.motion_y, 0.0);
    assert_eq!(b.base.motion_y, 0.0);

    // Far apart: 1/var6 < 1 scales the push down; entityCollisionReduction
    // (a float) scales it again.
    let mut c = item_at(&mut w, 0.0, 70.0, 0.0);
    let mut e = item_at(&mut w, 4.0, 70.0, -1.0);
    c.base.entity_collision_reduction = 0.25;
    base::apply_entity_collision(&mut c, &mut w, &mut e);
    let v6 = math::sqrt_double(4.0) as f64;
    let v8 = 1.0 / v6;
    let px = 4.0 / v6 * v8 * 0.05000000074505806 * (1.0f32 - 0.25f32) as f64;
    let pz = -1.0 / v6 * v8 * 0.05000000074505806 * (1.0f32 - 0.25f32) as f64;
    assert_eq!(e.base.motion_x, px);
    assert_eq!(e.base.motion_z, pz);
    assert_eq!(c.base.motion_x, -px);

    // Below 0.01 apart (abs_max), nothing.
    let mut f = item_at(&mut w, 0.0, 70.0, 0.0);
    let mut g = item_at(&mut w, 0.009, 70.0, -0.009);
    base::apply_entity_collision(&mut f, &mut w, &mut g);
    assert_eq!((f.base.motion_x, g.base.motion_x), (0.0, 0.0));

    // A rider and its mount never push each other.
    let mut m = item_at(&mut w, 0.0, 70.0, 0.0);
    let mut r = item_at(&mut w, 0.5, 70.0, 0.0);
    r.base.riding_entity = Some(m.id());
    m.base.ridden_by_entity = Some(r.id());
    base::apply_entity_collision(&mut m, &mut w, &mut r);
    base::apply_entity_collision(&mut r, &mut w, &mut m);
    assert_eq!((m.base.motion_x, r.base.motion_x), (0.0, 0.0));
}

// src: Entity.java:892-906 (getEyeHeight is 0 for a non-living entity)
#[test]
fn is_entity_inside_opaque_block_tests_eight_points_around_the_eye() {
    let mut w = empty_world();
    let inside = item_at(&mut w, 0.5, 60.5, 0.5);
    assert!(base::is_entity_inside_opaque_block(&inside, &mut w));
    let above = item_at(&mut w, 0.5, 61.1, 0.5);
    assert!(!base::is_entity_inside_opaque_block(&above, &mut w));
    // ±0.05 in y: an eye 0.04 above the stone top still has points in it.
    let skim = item_at(&mut w, 0.5, 61.04, 0.5);
    assert!(base::is_entity_inside_opaque_block(&skim, &mut w));
    // ±0.9 * width / 2 in x and z: a point at x + 0.1125 reaches a lone
    // block beside the column the centre is in.
    w.set_block(6, 64, 0, 1);
    let beside = item_at(&mut w, 5.95, 64.5, 0.5);
    assert!(base::is_entity_inside_opaque_block(&beside, &mut w));
    let clear = item_at(&mut w, 5.85, 64.5, 0.5);
    assert!(!base::is_entity_inside_opaque_block(&clear, &mut w));
}

// src: Entity.java:293-540, the step-up branch (:382-451), which needs
// stepHeight > 0: EntityLiving's 0.5 (EntityLiving.java:67) on an item-sized box.
#[test]
fn move_entity_steps_up_onto_a_half_slab_with_step_height() {
    let mut w = empty_world();
    w.set_block(1, 61, 0, 44); // single stone slab: box y 61..61.5
    let mut e = item_at(&mut w, 0.7, 61.125, 0.5); // box 0.575..0.825, feet on the stone
    e.base.on_ground = true;
    e.base.step_height = 0.5;
    dispatch::move_entity(&mut e, &mut w, 0.3, -0.08, 0.0);
    // The stepped move covers the whole 0.3 and lands on the slab top.
    assert_eq!(e.base.bounding_box.min_y, 61.5);
    assert_eq!(e.base.bounding_box.min_x, 0.575 + 0.3);
    // var41 = minY - (int)minY = 0.5 > 0: ySize = (float)(0.0 + 0.5 + 0.01).
    let y_size = (0.0f64 + 0.5 + 0.01) as f32;
    assert_eq!(e.base.y_size, y_size);
    assert_eq!(e.base.pos_y, 61.5 + 0.125 - y_size as f64);
    assert!(e.base.on_ground);
    assert!(!e.base.is_collided_horizontally);
    assert!(e.base.is_collided_vertically);
    assert_eq!(e.base.motion_y, 0.0);

    // Without step height (an EntityItem's 0), the slab stops it.
    let mut f = item_at(&mut w, 0.7, 61.125, 0.5);
    f.base.on_ground = true;
    dispatch::move_entity(&mut f, &mut w, 0.3, -0.08, 0.0);
    assert_eq!(f.base.bounding_box.max_x, 1.0);
    assert_eq!(f.base.bounding_box.min_y, 61.0);
    assert!(f.base.is_collided_horizontally);
    assert_eq!(f.base.motion_x, 0.0);
    assert_eq!(f.base.y_size, 0.0);
}

// src: Entity.java:982-1008, :916-972, World.java:1291-1352
#[test]
fn mount_entity_links_relinks_and_dismounts() {
    let mut w = empty_world();
    let a = item_at(&mut w, 0.5, 65.0, 0.5);
    let b = item_at(&mut w, 2.5, 65.0, 0.5);
    // With chunks -2..2 loaded, an entity ticks only at 0 <= x, z < 16 (the
    // 32-block check, World.java:1291-1295).
    let c = item_at(&mut w, 1.5, 65.0, 0.5);
    let (ia, ib, ic) = (a.id(), b.id(), c.id());
    assert!(w.spawn_entity_in_world(a));
    assert!(w.spawn_entity_in_world(b));
    assert!(w.spawn_entity_in_world(c));

    // a rides b.
    w.with_entity_mut(ia, |w, ea| base::mount_entity_in_world(ea, w, ib));
    assert_eq!(w.entity(ia).unwrap().base.riding_entity, Some(ib));
    assert_eq!(w.entity(ib).unwrap().base.ridden_by_entity, Some(ia));

    // c takes b over: a is unseated (Entity.java:1001-1003), c rides b.
    w.with_entity_mut(ic, |w, ec| base::mount_entity_in_world(ec, w, ib));
    assert_eq!(w.entity(ia).unwrap().base.riding_entity, None);
    assert_eq!(w.entity(ic).unwrap().base.riding_entity, Some(ib));
    assert_eq!(w.entity(ib).unwrap().base.ridden_by_entity, Some(ic));

    // c moves to a: b loses its rider (Entity.java:997-999).
    w.with_entity_mut(ic, |w, ec| base::mount_entity_in_world(ec, w, ia));
    assert_eq!(w.entity(ib).unwrap().base.ridden_by_entity, None);
    assert_eq!(w.entity(ia).unwrap().base.ridden_by_entity, Some(ic));
    assert_eq!(w.entity(ic).unwrap().base.entity_rider_yaw_delta, 0.0);

    // One tick: a updates and then its rider c, which updateRidden puts on a's
    // top (posY + height * 0.75 + c's yOffset; Entity.java:970-980).
    w.update_entities();
    let (ax, ay, az) = {
        let ea = w.entity(ia).unwrap();
        (ea.base.pos_x, ea.base.pos_y, ea.base.pos_z)
    };
    let ec = w.entity(ic).unwrap();
    assert_eq!(ec.base.riding_entity, Some(ia));
    assert_eq!(ec.base.pos_x, ax);
    assert_eq!(ec.base.pos_z, az);
    assert_eq!(ec.base.pos_y, ay + 0.25f32 as f64 * 0.75 + 0.125f32 as f64);

    // Mounting the current mount again dismounts onto its top (Entity.java:992-995).
    let top = {
        let ea = w.entity(ia).unwrap();
        ea.base.bounding_box.min_y + ea.base.height as f64
    };
    w.with_entity_mut(ic, |w, ec| base::mount_entity_in_world(ec, w, ia));
    let ec = w.entity(ic).unwrap();
    assert_eq!(ec.base.riding_entity, None);
    assert_eq!(ec.base.pos_y, top + 0.125f32 as f64); // setLocationAndAngles adds yOffset
    assert_eq!(ec.base.prev_pos_y, ec.base.pos_y);
    assert_eq!(w.entity(ia).unwrap().base.ridden_by_entity, None);
}

// src: Entity.java:628-643
#[test]
fn get_entity_brightness_reads_the_light_at_two_thirds_height() {
    let mut w = empty_world();
    let mut e = item_at(&mut w, 0.5, 64.0, 0.5);
    // var5 = floor(posY - yOffset + (maxY - minY) * 0.66).
    let y = math::floor_double(64.0 - 0.125 + 0.25 * 0.66);
    let expected = w.get_light_brightness(0, y, 0);
    assert_eq!(base::get_entity_brightness(&e, &mut w, 1.0), expected);
    e.base.entity_brightness = expected + 0.5;
    assert_eq!(base::get_entity_brightness(&e, &mut w, 1.0), expected + 0.5);
    // A box over a missing chunk: entityBrightness alone.
    let mut far = item_at(&mut w, 200.5, 64.0, 0.5);
    far.base.entity_brightness = 0.25;
    assert_eq!(base::get_entity_brightness(&far, &mut w, 1.0), 0.25);
}

// src: Entity.java:1010-1028, :133-147, :172-187
#[test]
fn position_helpers_lift_out_of_the_floor() {
    let mut w = empty_world();
    // setPositionAndRotation2: placed overlapping the floor's top, lifted
    // onto it (the 1/32 horizontal shrink still overlaps the stone box).
    let mut e = item_at(&mut w, 0.5, 64.0, 0.5);
    base::set_position_and_rotation2(&mut e, &mut w, 0.5, 60.9, 0.5, 370.0, -10.0, 3);
    assert_eq!(e.base.bounding_box.min_y, 61.0);
    assert_eq!(e.base.pos_y, 60.9 + (61.0 - (60.9 - 0.125)));
    assert_eq!(e.base.rotation_yaw, 370.0f32 % 360.0);

    // preparePlayerToSpawn: a whole block at a time until free, then still.
    let mut p = item_at(&mut w, 0.5, 60.5, 0.5);
    p.base.motion_x = 0.3;
    p.base.rotation_pitch = 20.0;
    base::prepare_player_to_spawn(&mut p, &mut w);
    assert_eq!(p.base.pos_y, 61.5);
    assert_eq!((p.base.motion_x, p.base.rotation_pitch), (0.0, 0.0));

    // setAngles: 0.15 per unit, pitch clamped, prevRotation* shifted alike.
    let mut a = item_at(&mut w, 0.5, 64.0, 0.5);
    a.base.rotation_yaw = 10.0;
    a.base.prev_rotation_yaw = 8.0;
    a.base.rotation_pitch = 80.0;
    a.base.prev_rotation_pitch = 79.0;
    a.base.set_angles(20.0, -100.0);
    assert_eq!(a.base.rotation_yaw, (10.0f64 + 20.0 * 0.15) as f32);
    assert_eq!(a.base.rotation_pitch, 90.0);
    assert_eq!(a.base.prev_rotation_pitch, 79.0 + (90.0 - 80.0));
    assert_eq!(
        a.base.prev_rotation_yaw,
        8.0 + ((10.0f64 + 20.0 * 0.15) as f32 - 10.0)
    );
}

// src: Entity.java:1085-1092, EntityItem.java:83-95
#[test]
fn struck_by_lightning_deals_fire_damage_and_lights() {
    let mut w = empty_world();
    let mut e = item_at(&mut w, 0.5, 64.0, 0.5);
    let bolt = item_at(&mut w, 0.5, 64.0, 0.5);
    e.base.fire = -1;
    dispatch::on_struck_by_lightning(&mut e, &mut w, &bolt);
    // EntityItem.dealFireDamage(5) takes all its health.
    assert!(e.base.is_dead);
    assert_eq!(e.base.fire, 300);
    let mut f = item_at(&mut w, 0.5, 64.0, 0.5);
    f.base.fire = 40;
    base::on_struck_by_lightning(&mut f, &mut w, &bolt);
    assert_eq!(f.base.fire, 41);
}

// src: Entity.java:762-774, :869-871, :1030-1032
#[test]
fn render_data_bodies() {
    let mut w = empty_world();
    let e = item_at(&mut w, 0.5, 64.0, 0.5);
    // Average edge 0.25 -> range 0.25 * 64 = 16 blocks.
    assert!(base::is_in_range_to_render_dist(&e, 255.9));
    assert!(!base::is_in_range_to_render_dist(&e, 256.0));
    assert!(base::is_in_range_to_render_vec3d(
        &e,
        &Vec3D::create_vector(0.5, 64.0, 15.5)
    ));
    assert!(!base::is_in_range_to_render_vec3d(
        &e,
        &Vec3D::create_vector(0.5, 64.0, 16.5)
    ));
    assert_eq!(base::get_shadow_size(&e), 0.125);
    assert_eq!(base::get_collision_border_size(), 0.1);
    assert_eq!(dispatch::get_look_vec(&e, &w), None);
    assert_eq!(dispatch::get_entity_texture(&e, &w), None);
}

// src: Entity.java:588-601, BlockFluid.java:18-25
#[test]
fn is_inside_of_material_compares_the_eye_with_the_fluid_surface() {
    let mut w = empty_world();
    // A full still-water cell (meta 0): surface at 62 - (1/9 - 0.11111111).
    w.set_block_and_metadata(1, 61, 1, 9, 0);
    // A thin flowing-water cell (meta 7): surface at 62 - (8/9 - 0.11111111),
    // about 61.2222.
    w.set_block_and_metadata(2, 61, 2, 8, 7);
    w.set_block_and_metadata(3, 61, 3, 11, 0);

    // An EntityItem's eye height is Entity's 0, so the eye is posY.
    let e = item_at(&mut w, 1.5, 61.5, 1.5);
    assert!(base::is_inside_of_material(&e, &mut w, Material::Water));
    assert!(!base::is_inside_of_material(&e, &mut w, Material::Lava));
    let e = item_at(&mut w, 1.5, 61.999, 1.5);
    assert!(base::is_inside_of_material(&e, &mut w, Material::Water));

    let e = item_at(&mut w, 2.5, 61.2, 2.5);
    assert!(base::is_inside_of_material(&e, &mut w, Material::Water));
    let e = item_at(&mut w, 2.5, 61.25, 2.5);
    assert!(!base::is_inside_of_material(&e, &mut w, Material::Water));

    let e = item_at(&mut w, 3.5, 61.5, 3.5);
    assert!(base::is_inside_of_material(&e, &mut w, Material::Lava));
    assert!(!base::is_inside_of_material(&e, &mut w, Material::Water));

    // Air (id 0) and stone (not water) are never inside.
    let e = item_at(&mut w, 0.5, 61.5, 0.5);
    assert!(!base::is_inside_of_material(&e, &mut w, Material::Water));
    let e = item_at(&mut w, 0.5, 60.5, 0.5);
    assert!(!base::is_inside_of_material(&e, &mut w, Material::Water));
}
