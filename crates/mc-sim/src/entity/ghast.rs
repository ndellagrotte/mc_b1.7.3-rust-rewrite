//! `EntityGhast` (EntityGhast.java) (WP-2.3).
//!
//! | EntityGhast.java | Method | Port |
//! |---|---|---|
//! | 4-11 | fields | [`GhastData`] |
//! | 13-18 | constructor | [`new_entity_ghast`] |
//! | 20-23 | `entityInit` (watcher byte 16: charging) | [`new_entity_ghast`] |
//! | 25-29 | `onUpdate` (the texture follows byte 16) | [`on_update`] |
//! | 31-113 | `updatePlayerActionState` (waypoints, target, fireballs) | [`update_player_action_state`] |
//! | 115-129 | `isCourseTraversable` (private) | [`is_course_traversable`] |
//! | 131-149 | the sounds, `getDropItemId` (gunpowder), `getSoundVolume` (10) | [`super::dispatch`] |
//! | 151-153 | `getCanSpawnHere` (1 in 20) | [`get_can_spawn_here`] |
//! | 155-157 | `getMaxSpawnedInChunk` (1) | [`super::dispatch::get_max_spawned_in_chunk`] |
//!
//! `EntityGhast implements IMob`. No NBT of its own (`EntityLiving`'s).
//!
//! The fireball is `new EntityFireball(world, ghast, dx, dy, dz)`, whose
//! constructor draws its acceleration jitter from the fireball's `rand`;
//! in the oracle that `Random` is unseeded then (tools/oracle/census.md §3,
//! a **gap**), so a ghast's fireball does not fly reproducibly there. The
//! port draws from the fireball's construction-seeded `rand`.

use mc_core::jmath;
use mc_core::math;

use super::data_watcher::WatchValue;
use super::living::{self, EntityView, lv_mut};
use super::{Entity, EntityBase, EntityKindData, FlyingData, LivingData, dispatch, fireball};
use crate::world::World;

/// The fields `EntityGhast` declares (EntityGhast.java:4-11).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GhastData {
    /// `courseChangeCooldown`: ticks until the next push toward the waypoint.
    pub course_change_cooldown: i32,
    /// `waypointX`.
    pub waypoint_x: f64,
    /// `waypointY`.
    pub waypoint_y: f64,
    /// `waypointZ`.
    pub waypoint_z: f64,
    /// `targetedEntity` (private), as an entity id.
    pub targeted_entity: Option<i32>,
    /// `aggroCooldown` (private): ticks until the target is looked up again.
    pub aggro_cooldown: i32,
    /// `prevAttackCounter` (render: the mouth).
    pub prev_attack_counter: i32,
    /// `attackCounter`: 10 charges (sound), 20 fires, then −40.
    pub attack_counter: i32,
}

fn gd(e: &Entity) -> &GhastData {
    match &e.kind {
        EntityKindData::Ghast(_, _, d) => d,
        _ => panic!("EntityGhast method on another kind"),
    }
}

fn gd_mut(e: &mut Entity) -> &mut GhastData {
    match &mut e.kind {
        EntityKindData::Ghast(_, _, d) => d,
        _ => panic!("EntityGhast method on another kind"),
    }
}

/// `new EntityGhast(world)`: `entityInit` (watcher byte 16 = 0) in
/// `Entity`'s constructor, `EntityLiving`'s initialisers and body (health
/// 10), then the ghast texture, a 4 × 4 box and fire immunity.
// src: EntityGhast.java:13-23, EntityFlying.java:4-6, Entity.java:75-113, EntityLiving.java:6-68
pub fn new_entity_ghast(world: &mut World) -> Entity {
    let mut b = EntityBase::new(world);
    b.data_watcher.add_object(16, WatchValue::Byte(0));
    let mut l = LivingData::new(world, &mut b);
    l.texture = "/mob/ghast.png";
    b.set_size(4.0, 4.0);
    b.is_immune_to_fire = true;
    Entity {
        base: b,
        kind: EntityKindData::Ghast(l, FlyingData::default(), GhastData::default()),
    }
}

/// `EntityGhast.onUpdate()`: `EntityLiving`'s, then the open-mouth texture
/// while watcher byte 16 is 1.
// src: EntityGhast.java:25-29
pub fn on_update(e: &mut Entity, world: &mut World) {
    living::on_update(e, world);
    let var1 = e.base.data_watcher.get_watchable_object_byte(16);
    lv_mut(e).texture = if var1 == 1 {
        "/mob/ghast_fire.png"
    } else {
        "/mob/ghast.png"
    };
}

/// `targetedEntity.isDead` for a target held by id.
fn target_dead(world: &World, id: i32) -> bool {
    match world.entity(id) {
        Some(t) => t.base.is_dead,
        None => world.is_entity_dead_or_gone(id),
    }
}

/// `EntityGhast.updatePlayerActionState()` (no `super` call).
///
/// Dies on peaceful; despawn check; a new random waypoint within ±16 when
/// the current one is nearer than 1 or farther than 60 (three `rand`
/// draws); every `courseChangeCooldown` ticks (+2–6, a draw) a push of 0.1
/// toward it if the box can travel there, else the waypoint becomes the
/// current position. The target (closest player within 100) is looked up
/// again every 20 ticks; within 64 the ghast turns to it and, while it sees
/// it, charges (`attackCounter` 10: sound, 20: a fireball 4 blocks ahead
/// along its look, then −40); otherwise it faces its motion and the
/// counter runs down. Watcher byte 16 is 1 above 10.
///
/// vanilla bug (transcribed): the course test and the push use the
/// distance `var7` and offsets `var1`/`var3`/`var5` measured to the *old*
/// waypoint, also on the tick a new waypoint was just drawn: the push heads
/// for the old one, and `isCourseTraversable` steps toward the new one in
/// `var7`-scaled strides (`(waypoint - pos) / var7`).
// src: EntityGhast.java:31-113
pub fn update_player_action_state(e: &mut Entity, world: &mut World) {
    if !world.multiplayer_world && world.difficulty_setting == 0 {
        dispatch::set_entity_dead(e, world);
    }
    living::despawn_entity(e, world);
    {
        let d = gd_mut(e);
        d.prev_attack_counter = d.attack_counter;
    }
    let (var1, var3, var5) = {
        let d = gd(e);
        (
            d.waypoint_x - e.base.pos_x,
            d.waypoint_y - e.base.pos_y,
            d.waypoint_z - e.base.pos_z,
        )
    };
    let var7 = math::sqrt_double(var1 * var1 + var3 * var3 + var5 * var5) as f64;
    if var7 < 1.0 || var7 > 60.0 {
        let b = &mut e.base;
        let wx = b.pos_x + ((b.rand.next_float() * 2.0 - 1.0) * 16.0) as f64;
        let wy = b.pos_y + ((b.rand.next_float() * 2.0 - 1.0) * 16.0) as f64;
        let wz = b.pos_z + ((b.rand.next_float() * 2.0 - 1.0) * 16.0) as f64;
        let d = gd_mut(e);
        d.waypoint_x = wx;
        d.waypoint_y = wy;
        d.waypoint_z = wz;
    }

    let old = gd(e).course_change_cooldown;
    gd_mut(e).course_change_cooldown = old.wrapping_sub(1);
    if old <= 0 {
        let add = e.base.rand.next_int_bounded(5) + 2;
        gd_mut(e).course_change_cooldown += add;
        let (wx, wy, wz) = {
            let d = gd(e);
            (d.waypoint_x, d.waypoint_y, d.waypoint_z)
        };
        if is_course_traversable(e, world, wx, wy, wz, var7) {
            let b = &mut e.base;
            b.motion_x += var1 / var7 * 0.1;
            b.motion_y += var3 / var7 * 0.1;
            b.motion_z += var5 / var7 * 0.1;
        } else {
            let (x, y, z) = (e.base.pos_x, e.base.pos_y, e.base.pos_z);
            let d = gd_mut(e);
            d.waypoint_x = x;
            d.waypoint_y = y;
            d.waypoint_z = z;
        }
    }

    if let Some(t) = gd(e).targeted_entity
        && target_dead(world, t)
    {
        gd_mut(e).targeted_entity = None;
    }

    let look_up = match gd(e).targeted_entity {
        None => true,
        Some(_) => {
            let d = gd_mut(e);
            let old = d.aggro_cooldown;
            d.aggro_cooldown = old.wrapping_sub(1);
            old <= 0
        }
    };
    if look_up {
        let t = world.get_closest_player_to_entity(&e.base, 100.0);
        gd_mut(e).targeted_entity = t;
        if t.is_some() {
            gd_mut(e).aggro_cooldown = 20;
        }
    }

    let var9 = 64.0;
    let target = gd(e).targeted_entity.and_then(|t| {
        world.entity(t).map(|o| (t, o.base.clone())).or_else(|| {
            world.note_unimplemented("EntityGhast: targetedEntity not in its slot");
            None
        })
    });
    let in_range = target
        .as_ref()
        .filter(|(_, tb)| tb.get_distance_sq_to_entity(&e.base) < var9 * var9);
    if let Some((tid, tb)) = in_range {
        let var11 = tb.pos_x - e.base.pos_x;
        let var13 = tb.bounding_box.min_y + (tb.height / 2.0) as f64
            - (e.base.pos_y + (e.base.height / 2.0) as f64);
        let var15 = tb.pos_z - e.base.pos_z;
        let yaw = -(jmath::atan2(var11, var15) as f32) * 180.0 / 3.1415927;
        e.base.rotation_yaw = yaw;
        lv_mut(e).render_yaw_offset = yaw;
        let seen = match EntityView::of(world, *tid) {
            Some(v) => living::can_entity_be_seen(e, world, &v),
            None => false,
        };
        if seen {
            if gd(e).attack_counter == 10 {
                let volume = dispatch::get_sound_volume(e, world);
                let pitch = (e.base.rand.next_float() - e.base.rand.next_float()) * 0.2 + 1.0;
                world.play_sound_at_entity(&e.base, "mob.ghast.charge", volume, pitch);
            }
            gd_mut(e).attack_counter += 1;
            if gd(e).attack_counter == 20 {
                let volume = dispatch::get_sound_volume(e, world);
                let pitch = (e.base.rand.next_float() - e.base.rand.next_float()) * 0.2 + 1.0;
                world.play_sound_at_entity(&e.base, "mob.ghast.fireball", volume, pitch);
                let mut var17 = fireball::new_entity_fireball_shot(world, e, var11, var13, var15);
                let var18 = 4.0;
                let var20 = living::get_look(e, 1.0);
                // Plain field writes: the box stays where the constructor
                // put it (at the ghast).
                var17.base.pos_x = e.base.pos_x + var20.x_coord * var18;
                var17.base.pos_y = e.base.pos_y + (e.base.height / 2.0) as f64 + 0.5;
                var17.base.pos_z = e.base.pos_z + var20.z_coord * var18;
                world.spawn_entity_in_world(var17);
                gd_mut(e).attack_counter = -40;
            }
        } else if gd(e).attack_counter > 0 {
            gd_mut(e).attack_counter -= 1;
        }
    } else {
        let yaw = -(jmath::atan2(e.base.motion_x, e.base.motion_z) as f32) * 180.0 / 3.1415927;
        e.base.rotation_yaw = yaw;
        lv_mut(e).render_yaw_offset = yaw;
        if gd(e).attack_counter > 0 {
            gd_mut(e).attack_counter -= 1;
        }
    }

    if !world.multiplayer_world {
        let var21 = e.base.data_watcher.get_watchable_object_byte(16);
        let var12: i8 = if gd(e).attack_counter > 10 { 1 } else { 0 };
        if var21 != var12 {
            e.base
                .data_watcher
                .update_object(16, WatchValue::Byte(var12));
        }
    }
}

/// `isCourseTraversable(x, y, z, distance)` (private): steps a copy of the
/// box `distance - 1` times by `(waypoint - pos) / distance` (the fields,
/// not the arguments) and fails at the first step that collides.
// src: EntityGhast.java:115-129
pub fn is_course_traversable(
    e: &Entity,
    world: &mut World,
    _var1: f64,
    _var3: f64,
    _var5: f64,
    var7: f64,
) -> bool {
    let d = gd(e);
    let var9 = (d.waypoint_x - e.base.pos_x) / var7;
    let var11 = (d.waypoint_y - e.base.pos_y) / var7;
    let var13 = (d.waypoint_z - e.base.pos_z) / var7;
    let mut var15 = e.base.bounding_box;
    let mut var16 = 1;
    while (var16 as f64) < var7 {
        var15.offset(var9, var11, var13);
        if !world.get_colliding_bounding_boxes(e, &var15).is_empty() {
            return false;
        }
        var16 += 1;
    }
    true
}

/// `EntityGhast.getCanSpawnHere()`: a 1-in-20 `rand` roll, `EntityLiving`'s
/// test, and not peaceful.
// src: EntityGhast.java:151-153
pub fn get_can_spawn_here(e: &mut Entity, world: &mut World) -> bool {
    e.base.rand.next_int_bounded(20) == 0
        && living::get_can_spawn_here(e, world)
        && world.difficulty_setting > 0
}
