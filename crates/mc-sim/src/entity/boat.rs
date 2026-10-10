//! `EntityBoat` (EntityBoat.java): the boat. It floats by sampling five
//! horizontal slices of its box for water, takes its rider's motion as
//! steering, breaks into 3 planks and 2 sticks when it hits something faster
//! than 0.15 or is hit hard enough, and clears snow layers under it.
//!
//! Transcribed: both constructors, `attackEntityFrom`, `onUpdate`,
//! `updateRiderPosition`, `interact`, the (empty) NBT bodies, and the data
//! overrides `canTriggerWalking`, `getCollisionBox`, `getBoundingBox`,
//! `canBePushed`, `getMountedYOffset`, `canBeCollidedWith` (in
//! [`super::dispatch`]). Transcribed as data for the client:
//! `performHurtAnimation`, `setPositionAndRotation2` and `setVelocity`
//! (multiplayer interpolation; the branch of `onUpdate` they feed is
//! transcribed but unreachable in single player), `getShadowSize` (0).
//!
//! The steering is `riddenByEntity.motionX/Z * 0.2` (EntityBoat.java:195-198),
//! not the rider's `moveForward`: a player's own update sets that motion
//! (`EntityPlayer`, WP-2.5). The splash particles draw the boat's own
//! `rand` (no `Math.random()`).

use mc_core::Aabb;
use mc_core::math::{floor_double, jmath};

use super::{Entity, EntityBase, EntityKindData, base, dispatch};
use crate::block::Material;
use crate::block::ids as block_ids;
use crate::world::World;

/// `Item.stick.shiftedIndex` (`new Item(24)`).
// src: Item.java:298 (shiftedIndex = 256 + id, Item.java:124)
pub const ITEM_STICK: i32 = 256 + 24;

/// The fields `EntityBoat` declares (EntityBoat.java:6-17).
#[derive(Clone, Debug, PartialEq)]
pub struct BoatData {
    /// `boatCurrentDamage`: +10 per damage point, breaks above 40, heals 1
    /// per tick.
    pub boat_current_damage: i32,
    /// `boatTimeSinceHit`: the hurt wobble, 10 after a hit.
    pub boat_time_since_hit: i32,
    /// `boatRockDirection`: the wobble's side, flipped on every hit.
    pub boat_rock_direction: i32,
    /// `boatPosRotationIncrements` (private): multiplayer interpolation
    /// steps left.
    pub boat_pos_rotation_increments: i32,
    /// `boatX` (private): multiplayer target position.
    pub boat_x: f64,
    /// `boatY` (private).
    pub boat_y: f64,
    /// `boatZ` (private).
    pub boat_z: f64,
    /// `boatYaw` (private).
    pub boat_yaw: f64,
    /// `boatPitch` (private).
    pub boat_pitch: f64,
    /// `velocityX` (private): multiplayer velocity, restored by
    /// `setPositionAndRotation2`.
    pub velocity_x: f64,
    /// `velocityY` (private).
    pub velocity_y: f64,
    /// `velocityZ` (private).
    pub velocity_z: f64,
}

impl Default for BoatData {
    /// The `(World)` constructor's assignments.
    // src: EntityBoat.java:19-27
    fn default() -> Self {
        BoatData {
            boat_current_damage: 0,
            boat_time_since_hit: 0,
            boat_rock_direction: 1,
            boat_pos_rotation_increments: 0,
            boat_x: 0.0,
            boat_y: 0.0,
            boat_z: 0.0,
            boat_yaw: 0.0,
            boat_pitch: 0.0,
            velocity_x: 0.0,
            velocity_y: 0.0,
            velocity_z: 0.0,
        }
    }
}

/// The `EntityBoat` level of `e`.
fn data(e: &mut Entity) -> &mut BoatData {
    match &mut e.kind {
        EntityKindData::Boat(d) => d,
        _ => panic!("EntityBoat method on another kind"),
    }
}

/// `new EntityBoat(world)`: the `EntityList` / NBT-load constructor and the
/// harness `spawn` op's: 1.5 x 0.6, centred (`yOffset` is half the height),
/// `preventEntitySpawning`.
// src: EntityBoat.java:19-27
pub fn new_entity_boat_empty(world: &mut World) -> Entity {
    let mut b = EntityBase::new(world);
    // entityInit() is empty (EntityBoat.java:33-34).
    let d = BoatData::default();
    b.prevent_entity_spawning = true;
    b.set_size(1.5, 0.6);
    b.y_offset = b.height / 2.0;
    Entity {
        base: b,
        kind: EntityKindData::Boat(d),
    }
}

/// `new EntityBoat(world, x, y, z)` (`ItemBoat.onItemRightClick`): at
/// `(x, y + yOffset, z)`, still, `prevPos` the given (feet) position.
// src: EntityBoat.java:48-57
pub fn new_entity_boat(world: &mut World, x: f64, y: f64, z: f64) -> Entity {
    let mut e = new_entity_boat_empty(world);
    let b = &mut e.base;
    b.set_position(x, y + b.y_offset as f64, z);
    b.motion_x = 0.0;
    b.motion_y = 0.0;
    b.motion_z = 0.0;
    b.prev_pos_x = x;
    b.prev_pos_y = y;
    b.prev_pos_z = z;
    e
}

/// `EntityBoat.getMountedYOffset()`.
// src: EntityBoat.java:59-61
pub fn get_mounted_y_offset(e: &Entity) -> f64 {
    e.base.height as f64 * 0.0 - 0.30000001192092896
}

/// The break drop: 3 planks then 2 sticks, each its own `EntityItem`.
// src: EntityBoat.java:75-81, :251-257
fn drop_wood(e: &Entity, world: &mut World) {
    for _ in 0..3 {
        base::drop_item_with_offset(e, world, block_ids::PLANKS, 1, 0.0);
    }
    for _ in 0..2 {
        base::drop_item_with_offset(e, world, ITEM_STICK, 1, 0.0);
    }
}

/// `EntityBoat.attackEntityFrom(attacker, damage)`: the wobble and `damage *
/// 10` damage; above 40 the rider is put off, the wood is dropped and the
/// boat dies (`setEntityDead` after the drops). Always true.
// src: EntityBoat.java:63-90
pub fn attack_entity_from(
    e: &mut Entity,
    world: &mut World,
    attacker: Option<&Entity>,
    var2: i32,
) -> bool {
    if !world.multiplayer_world && !e.base.is_dead {
        let d = data(e);
        d.boat_rock_direction = -d.boat_rock_direction;
        d.boat_time_since_hit = 10;
        d.boat_current_damage = d.boat_current_damage.wrapping_add(var2.wrapping_mul(10));
        let broken = d.boat_current_damage > 40;
        e.base.set_been_attacked();
        if broken {
            if let Some(rider) = e.base.ridden_by_entity {
                // `this.riddenByEntity.mountEntity(this)`: the rider, already
                // in this boat, gets off on top of it (Entity.java:992-995).
                // A rider in its slot is put off here, also when it is the
                // attacker passed as a copy (its own arrow, EntityArrow.java:163,
                // or the harness's direct attack). A rider out of its slot
                // that is the attacker is a rider punching its own boat, held
                // by the caller: `player_controller::attack_entity` puts it
                // off right after this returns (nothing in between reads
                // either; WP-4.8, entity/boat-broken-by-rider).
                if world
                    .with_entity_mut(rider, |w, r| base::mount_entity(r, w, Some(e)))
                    .is_none()
                    && !attacker.is_some_and(|a| a.base.entity_id == rider)
                {
                    world.note_unimplemented("EntityBoat.attackEntityFrom: rider out of its slot");
                }
            }
            drop_wood(e, world);
            dispatch::set_entity_dead(e, world);
        }
    }
    true
}

/// `EntityBoat.performHurtAnimation()` (client): the wobble.
// src: EntityBoat.java:92-96
pub fn perform_hurt_animation(e: &mut Entity) {
    let d = data(e);
    d.boat_rock_direction = -d.boat_rock_direction;
    d.boat_time_since_hit = 10;
    d.boat_current_damage = d
        .boat_current_damage
        .wrapping_add(d.boat_current_damage.wrapping_mul(10));
}

/// `EntityBoat.setPositionAndRotation2(x, y, z, yaw, pitch, increments)`
/// (multiplayer): the interpolation target, `increments + 4` steps, and the
/// stored velocity restored.
// src: EntityBoat.java:102-112
#[allow(clippy::too_many_arguments)]
pub fn set_position_and_rotation2(
    e: &mut Entity,
    var1: f64,
    var3: f64,
    var5: f64,
    var7: f32,
    var8: f32,
    var9: i32,
) {
    let d = data(e);
    d.boat_x = var1;
    d.boat_y = var3;
    d.boat_z = var5;
    d.boat_yaw = var7 as f64;
    d.boat_pitch = var8 as f64;
    d.boat_pos_rotation_increments = var9 + 4;
    let (vx, vy, vz) = (d.velocity_x, d.velocity_y, d.velocity_z);
    e.base.motion_x = vx;
    e.base.motion_y = vy;
    e.base.motion_z = vz;
}

/// `EntityBoat.setVelocity(x, y, z)` (multiplayer): motion and the stored
/// velocity.
// src: EntityBoat.java:114-118
pub fn set_velocity(e: &mut Entity, var1: f64, var3: f64, var5: f64) {
    let d = data(e);
    d.velocity_x = var1;
    d.velocity_y = var3;
    d.velocity_z = var5;
    e.base.motion_x = var1;
    e.base.motion_y = var3;
    e.base.motion_z = var5;
}

/// `EntityBoat.onUpdate()` (module docs).
// src: EntityBoat.java:120-316
#[allow(clippy::manual_clamp, clippy::identity_op)] // Java's two ifs and `var4 + 0`, kept
pub fn on_update(e: &mut Entity, world: &mut World) {
    base::on_update(e, world);
    {
        let d = data(e);
        if d.boat_time_since_hit > 0 {
            d.boat_time_since_hit -= 1;
        }
        if d.boat_current_damage > 0 {
            d.boat_current_damage -= 1;
        }
    }
    let b = &mut e.base;
    b.prev_pos_x = b.pos_x;
    b.prev_pos_y = b.pos_y;
    b.prev_pos_z = b.pos_z;

    // src: EntityBoat.java:133-143 (the share of five horizontal slices of
    // the box, lowered by 1/8, that are in water)
    let var1: i8 = 5;
    let mut var2 = 0.0;
    for var4 in 0..var1 as i32 {
        let bb = e.base.bounding_box;
        let var5 = bb.min_y + (bb.max_y - bb.min_y) * (var4 + 0) as f64 / var1 as f64 - 0.125;
        let var7 = bb.min_y + (bb.max_y - bb.min_y) * (var4 + 1) as f64 / var1 as f64 - 0.125;
        let var9 = Aabb::new(bb.min_x, var5, bb.min_z, bb.max_x, var7, bb.max_z);
        if world.is_aabb_in_material(&var9, Material::Water) {
            var2 += 1.0 / var1 as f64;
        }
    }

    if world.multiplayer_world {
        // src: EntityBoat.java:149-182 (multiplayer interpolation)
        let d = data(e).clone();
        let b = &mut e.base;
        if d.boat_pos_rotation_increments > 0 {
            let inc = d.boat_pos_rotation_increments as f64;
            let var21 = b.pos_x + (d.boat_x - b.pos_x) / inc;
            let var6 = b.pos_y + (d.boat_y - b.pos_y) / inc;
            let var8 = b.pos_z + (d.boat_z - b.pos_z) / inc;
            let mut var10 = d.boat_yaw - b.rotation_yaw as f64;
            while var10 < -180.0 {
                var10 += 360.0;
            }
            while var10 >= 180.0 {
                var10 -= 360.0;
            }
            b.rotation_yaw = (b.rotation_yaw as f64 + var10 / inc) as f32;
            b.rotation_pitch =
                (b.rotation_pitch as f64 + (d.boat_pitch - b.rotation_pitch as f64) / inc) as f32;
            b.set_position(var21, var6, var8);
            let (yaw, pitch) = (b.rotation_yaw, b.rotation_pitch);
            b.set_rotation(yaw, pitch);
            data(e).boat_pos_rotation_increments -= 1;
        } else {
            let var21 = b.pos_x + b.motion_x;
            let var6 = b.pos_y + b.motion_y;
            let var8 = b.pos_z + b.motion_z;
            b.set_position(var21, var6, var8);
            if b.on_ground {
                b.motion_x *= 0.5;
                b.motion_y *= 0.5;
                b.motion_z *= 0.5;
            }
            b.motion_x *= 0.9900000095367432;
            b.motion_y *= 0.949999988079071;
            b.motion_z *= 0.9900000095367432;
        }
        return;
    }

    // Buoyancy.
    let b = &mut e.base;
    if var2 < 1.0 {
        let var21 = var2 * 2.0 - 1.0;
        b.motion_y += 0.03999999910593033 * var21;
    } else {
        if b.motion_y < 0.0 {
            b.motion_y /= 2.0;
        }
        b.motion_y += 0.007000000216066837;
    }

    // Steering: the rider's motion (module docs).
    if let Some(r) = e.base.ridden_by_entity {
        match world.entity(r) {
            Some(rider) => {
                let (rx, rz) = (rider.base.motion_x, rider.base.motion_z);
                e.base.motion_x += rx * 0.2;
                e.base.motion_z += rz * 0.2;
            }
            None => world.note_unimplemented("EntityBoat.onUpdate: rider out of its slot"),
        }
    }

    let b = &mut e.base;
    let var21 = 0.4;
    if b.motion_x < -var21 {
        b.motion_x = -var21;
    }
    if b.motion_x > var21 {
        b.motion_x = var21;
    }
    if b.motion_z < -var21 {
        b.motion_z = -var21;
    }
    if b.motion_z > var21 {
        b.motion_z = var21;
    }
    if b.on_ground {
        b.motion_x *= 0.5;
        b.motion_y *= 0.5;
        b.motion_z *= 0.5;
    }
    let (mx, my, mz) = (b.motion_x, b.motion_y, b.motion_z);
    dispatch::move_entity(e, world, mx, my, mz);

    let b = &mut e.base;
    let var6 = jmath::sqrt(b.motion_x * b.motion_x + b.motion_z * b.motion_z);
    if var6 > 0.15 {
        // src: EntityBoat.java:225-244 (splash particles on the boat's rand)
        let var8 = jmath::cos(b.rotation_yaw as f64 * 3.141592653589793 / 180.0);
        let var10 = jmath::sin(b.rotation_yaw as f64 * 3.141592653589793 / 180.0);
        let mut var12 = 0i32;
        while (var12 as f64) < 1.0 + var6 * 60.0 {
            let b = &mut e.base;
            let var13 = (b.rand.next_float() * 2.0 - 1.0) as f64;
            let var15 = (b.rand.next_int_bounded(2) * 2 - 1) as f64 * 0.7;
            let (var17, var19);
            if b.rand.next_boolean() {
                var17 = b.pos_x - var8 * var13 * 0.8 + var10 * var15;
                var19 = b.pos_z - var10 * var13 * 0.8 - var8 * var15;
            } else {
                var17 = b.pos_x + var8 + var10 * var13 * 0.7;
                var19 = b.pos_z + var10 - var8 * var13 * 0.7;
            }
            let (y, mx, my, mz) = (b.pos_y - 0.125, b.motion_x, b.motion_y, b.motion_z);
            world.spawn_particle("splash", var17, y, var19, mx, my, mz);
            var12 += 1;
        }
    }

    if e.base.is_collided_horizontally && var6 > 0.15 {
        if !world.multiplayer_world {
            // `this.setEntityDead()` (Entity's), then the wood; the update
            // goes on.
            dispatch::set_entity_dead(e, world);
            drop_wood(e, world);
        }
    } else {
        let b = &mut e.base;
        b.motion_x *= 0.9900000095367432;
        b.motion_y *= 0.949999988079071;
        b.motion_z *= 0.9900000095367432;
    }

    // src: EntityBoat.java:265-290 (the yaw turns toward the motion, at most
    // 20 degrees a tick)
    let b = &mut e.base;
    b.rotation_pitch = 0.0;
    let mut var8 = b.rotation_yaw as f64;
    let var10 = b.prev_pos_x - b.pos_x;
    let var23 = b.prev_pos_z - b.pos_z;
    if var10 * var10 + var23 * var23 > 0.001 {
        var8 = (jmath::atan2(var23, var10) * 180.0 / 3.141592653589793) as f32 as f64;
    }
    let mut var14 = var8 - b.rotation_yaw as f64;
    while var14 >= 180.0 {
        var14 -= 360.0;
    }
    while var14 < -180.0 {
        var14 += 360.0;
    }
    if var14 > 20.0 {
        var14 = 20.0;
    }
    if var14 < -20.0 {
        var14 = -20.0;
    }
    b.rotation_yaw = (b.rotation_yaw as f64 + var14) as f32;
    let (yaw, pitch) = (b.rotation_yaw, b.rotation_pitch);
    b.set_rotation(yaw, pitch);

    // src: EntityBoat.java:291-300 (other boats touching this one run
    // Entity.applyEntityCollision with this boat in hand)
    let bb = e
        .base
        .bounding_box
        .expand(0.20000000298023224, 0.0, 0.20000000298023224);
    let var16 = world.get_entities_within_aabb_excluding_entity(Some(e.base.entity_id), &bb);
    for id in var16 {
        if Some(id) == e.base.ridden_by_entity {
            continue;
        }
        world.with_entity_mut(id, |w, var18| {
            if dispatch::can_be_pushed(var18, w) && matches!(var18.kind, EntityKindData::Boat(..)) {
                dispatch::apply_entity_collision(var18, w, e);
            }
        });
    }

    // src: EntityBoat.java:302-309 (snow layers at the four corners of a
    // 0.8 square around the boat are removed)
    for var24 in 0..4i32 {
        let b = &e.base;
        let var25 = floor_double(b.pos_x + ((var24 % 2) as f64 - 0.5) * 0.8);
        let var26 = floor_double(b.pos_y);
        let var20 = floor_double(b.pos_z + ((var24 / 2) as f64 - 0.5) * 0.8);
        if world.get_block_id(var25, var26, var20) == block_ids::SNOW {
            world.set_block_with_notify(var25, var26, var20, 0);
        }
    }

    if let Some(r) = e.base.ridden_by_entity
        && world.is_entity_dead_or_gone(r)
    {
        e.base.ridden_by_entity = None;
    }
}

/// `EntityBoat.updateRiderPosition()`: the rider (in hand) 0.4 ahead along
/// the boat's yaw.
// src: EntityBoat.java:318-324
pub fn update_rider_position(mount: &Entity, rider: &mut Entity, world: &World) {
    // `if (this.riddenByEntity != null)`: the caller passes the rider.
    let m = &mount.base;
    let var1 = jmath::cos(m.rotation_yaw as f64 * 3.141592653589793 / 180.0) * 0.4;
    let var3 = jmath::sin(m.rotation_yaw as f64 * 3.141592653589793 / 180.0) * 0.4;
    let y = m.pos_y + get_mounted_y_offset(mount) + dispatch::get_y_offset(rider, world);
    rider.base.set_position(m.pos_x + var1, y, m.pos_z + var3);
}

/// `EntityBoat.interact(player)` (`player` in hand): the player gets in
/// (or out, `mountEntity` on the boat it rides), unless another player
/// rides it. Always true.
// src: EntityBoat.java:336-346
pub fn interact(e: &mut Entity, world: &mut World, var1: &mut Entity) -> bool {
    if let Some(r) = e.base.ridden_by_entity
        && r != var1.base.entity_id
    {
        let rider_is_player = match world.entity(r) {
            Some(rider) => rider.kind.is_player(),
            None => {
                world.note_unimplemented("EntityBoat.interact: rider out of its slot");
                false
            }
        };
        if rider_is_player {
            return true;
        }
    }
    if !world.multiplayer_world {
        // `var1.mountEntity(this)`: both in hand.
        base::mount_entity(var1, world, Some(e));
    }
    true
}
