//! `EntitySquid` (EntitySquid.java).
//!
//! | EntitySquid.java | Method | Port |
//! |---|---|---|
//! | 4-17 | fields | [`SquidData`] |
//! | 19-24 | constructor | [`new_entity_squid`] |
//! | 26-32 | `writeEntityToNBT`, `readEntityFromNBT` (`super` only) | [`super::nbt`] |
//! | 34-48 | `getLivingSound`, `getHurtSound`, `getDeathSound` (`null`), `getSoundVolume` (0.4) | [`super::dispatch`] |
//! | 50-52 | `getDropItemId` (0) | [`super::dispatch::get_drop_item_id`] |
//! | 54-61 | `dropFewItems` | [`drop_few_items`] |
//! | 63-65 | `interact` (`false`) | [`super::dispatch::interact`] |
//! | 67-69 | `isInWater` | [`is_in_water`] |
//! | 71-125 | `onLivingUpdate` | [`on_living_update`] |
//! | 127-129 | `moveEntityWithHeading` | [`move_entity_with_heading`] |
//! | 131-140 | `updatePlayerActionState` | [`update_player_action_state`] |
//!
//! # `rotationVelocity` and the harness
//!
//! The constructor draws `rotationVelocity` from `Entity.rand`, which the
//! vanilla client never seeds. The oracle's join hook reseeds `rand` with
//! `mix(seed, id)`, redraws `rotationVelocity` on it and reseeds
//! (tools/oracle/census.md §3, `HarnessWorld.seedOnJoin`); the port's `rand`
//! is seeded with `mix(seed, id)` at construction ([`EntityBase::new`]) and
//! again when the squid joins, so the constructor's draw here is that
//! redraw. The field is not saved, so a squid read from a chunk draws it
//! afresh in both (vanilla too).

use mc_core::math::{self, jmath};

use super::{
    CreatureData, Entity, EntityBase, EntityKindData, LivingData, WaterMobData, base, dispatch,
    living,
};
use crate::block::Material;
use crate::item::{ItemStack, ids};
use crate::world::World;

/// `3.1415927F` (`(float)Math.PI`), as EntitySquid.java writes it.
const PI_F: f32 = 3.1415927;
/// `6.2831855F` (`(float)(2 * Math.PI)`).
// src: EntitySquid.java:78-79
const TWO_PI_F: f32 = 6.2831855;

/// The fields `EntitySquid` declares (EntitySquid.java:4-17), in order. All
/// start at 0 (`rotationVelocity` is then set by the constructor). The
/// public ones drive `ModelSquid`/`RenderSquid` (client); the private ones
/// are the swim state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SquidData {
    /// `squidPitch`: body pitch (render).
    pub squid_pitch: f32,
    /// `prevSquidPitch`.
    pub prev_squid_pitch: f32,
    /// `squidYaw`: body yaw (render).
    pub squid_yaw: f32,
    /// `prevSquidYaw`.
    pub prev_squid_yaw: f32,
    /// `squidRotation`: the swim-stroke phase, 0 to 2π.
    pub squid_rotation: f32,
    /// `prevSquidRotation`.
    pub prev_squid_rotation: f32,
    /// `tentacleAngle` (render).
    pub tentacle_angle: f32,
    /// `lastTentacleAngle`.
    pub last_tentacle_angle: f32,
    /// `randomMotionSpeed` (private): 1 at the end of a stroke, decaying.
    pub random_motion_speed: f32,
    /// `rotationVelocity` (private): the phase step per tick.
    pub rotation_velocity: f32,
    /// `rotateSpeed` (private).
    pub rotate_speed: f32,
    /// `randomMotionVecX` (private): the swim direction.
    pub random_motion_vec_x: f32,
    /// `randomMotionVecY` (private).
    pub random_motion_vec_y: f32,
    /// `randomMotionVecZ` (private).
    pub random_motion_vec_z: f32,
}

/// The squid level of `e`.
fn sq(e: &Entity) -> &SquidData {
    match &e.kind {
        EntityKindData::Squid(.., s) => s,
        _ => unreachable!("EntitySquid method on another kind"),
    }
}

/// The squid level of `e`, mutably.
fn sq_mut(e: &mut Entity) -> &mut SquidData {
    match &mut e.kind {
        EntityKindData::Squid(.., s) => s,
        _ => unreachable!("EntitySquid method on another kind"),
    }
}

/// `1.0F / (rand.nextFloat() + 1.0F) * 0.2F`: a new `rotationVelocity`,
/// 0.1 to 0.2.
// src: EntitySquid.java:23, :81
fn draw_rotation_velocity(e: &mut Entity) -> f32 {
    1.0 / (e.base.rand.next_float() + 1.0) * 0.2
}

/// `new EntitySquid(world)`: `Entity(World)`, the `EntityLiving`
/// initialisers and body (four `Math.random()` draws), the empty
/// `EntityCreature` / `EntityWaterMob` levels, the squid's zeroed fields,
/// then the texture, a 0.95 × 0.95 box (`setSize` does not move the box)
/// and the `rotationVelocity` draw (module docs).
// src: EntitySquid.java:4-24, Entity.java:75-113, EntityLiving.java:6-68, EntityCreature.java:4-10, EntityWaterMob.java:4-6
pub fn new_entity_squid(world: &mut World) -> Entity {
    let mut b = EntityBase::new(world);
    let mut l = LivingData::new(world, &mut b);
    l.texture = "/mob/squid.png";
    b.set_size(0.95, 0.95);
    let mut e = Entity {
        base: b,
        kind: EntityKindData::Squid(
            l,
            CreatureData::default(),
            WaterMobData::default(),
            SquidData::default(),
        ),
    };
    let v = draw_rotation_velocity(&mut e);
    sq_mut(&mut e).rotation_velocity = v;
    e
}

/// `EntitySquid.dropFewItems()`: one to three ink sacs (`dyePowder` damage
/// 0), each `entityDropItem(stack, 0.0F)`. No `super` call.
// src: EntitySquid.java:54-61
pub fn drop_few_items(e: &mut Entity, world: &mut World) {
    let var1 = e.base.rand.next_int_bounded(3) + 1;
    for _var2 in 0..var1 {
        base::entity_drop_item(e, world, ItemStack::new(ids::DYE_POWDER, 1, 0), 0.0);
    }
}

/// `EntitySquid.isInWater()`: `handleMaterialAcceleration` over the box
/// shrunk by 0.6 at the top and the bottom. Like every
/// `handleMaterialAcceleration` call it also adds the water's flow to the
/// squid's motion (Java has no side-effect-free version here), and it is
/// called twice per tick (`EntityLiving.onLivingUpdate`, then
/// [`on_living_update`]).
// src: EntitySquid.java:67-69
// vanilla bug: a query with a side effect: each call pushes the squid by
// the current again (twice a tick), transcribed.
pub fn is_in_water(e: &mut Entity, world: &mut World) -> bool {
    let bb = e.base.bounding_box.expand(0.0, -0.6000000238418579, 0.0);
    world.handle_material_acceleration(&bb, Material::Water, e)
}

/// `EntitySquid.onLivingUpdate()`: `EntityLiving`'s (which runs the AI and
/// the move), then the stroke animation; in water the stroke sets the
/// motion from the swim direction and turns the body along it, out of water
/// the squid falls and its pitch sinks toward −90.
// src: EntitySquid.java:71-125
pub fn on_living_update(e: &mut Entity, world: &mut World) {
    living::on_living_update(e, world);
    {
        let s = sq_mut(e);
        s.prev_squid_pitch = s.squid_pitch;
        s.prev_squid_yaw = s.squid_yaw;
        s.prev_squid_rotation = s.squid_rotation;
        s.last_tentacle_angle = s.tentacle_angle;
        s.squid_rotation += s.rotation_velocity;
    }
    if sq(e).squid_rotation > TWO_PI_F {
        sq_mut(e).squid_rotation -= TWO_PI_F;
        if e.base.rand.next_int_bounded(10) == 0 {
            let v = draw_rotation_velocity(e);
            sq_mut(e).rotation_velocity = v;
        }
    }

    if dispatch::is_in_water(e, world) {
        {
            let s = sq_mut(e);
            if s.squid_rotation < PI_F {
                let var1 = s.squid_rotation / PI_F;
                s.tentacle_angle = math::sin(var1 * var1 * PI_F) * PI_F * 0.25;
                if var1 as f64 > 0.75 {
                    s.random_motion_speed = 1.0;
                    s.rotate_speed = 1.0;
                } else {
                    s.rotate_speed *= 0.8;
                }
            } else {
                s.tentacle_angle = 0.0;
                s.random_motion_speed *= 0.9;
                s.rotate_speed *= 0.99;
            }
        }

        if !living::lv(e).is_multiplayer_entity {
            let s = sq(e);
            let (mx, my, mz) = (
                (s.random_motion_vec_x * s.random_motion_speed) as f64,
                (s.random_motion_vec_y * s.random_motion_speed) as f64,
                (s.random_motion_vec_z * s.random_motion_speed) as f64,
            );
            e.base.motion_x = mx;
            e.base.motion_y = my;
            e.base.motion_z = mz;
        }

        let (mx, my, mz) = (e.base.motion_x, e.base.motion_y, e.base.motion_z);
        let var1 = math::sqrt_double(mx * mx + mz * mz);
        let l = living::lv_mut(e);
        l.render_yaw_offset +=
            (-(jmath::atan2(mx, mz) as f32) * 180.0 / PI_F - l.render_yaw_offset) * 0.1;
        let ryo = l.render_yaw_offset;
        e.base.rotation_yaw = ryo;
        let s = sq_mut(e);
        s.squid_yaw += PI_F * s.rotate_speed * 1.5;
        s.squid_pitch +=
            (-(jmath::atan2(var1 as f64, my) as f32) * 180.0 / PI_F - s.squid_pitch) * 0.1;
    } else {
        let r = sq(e).squid_rotation;
        sq_mut(e).tentacle_angle = math::abs(math::sin(r)) * PI_F * 0.25;
        if !living::lv(e).is_multiplayer_entity {
            e.base.motion_x = 0.0;
            e.base.motion_y -= 0.08;
            e.base.motion_y *= 0.9800000190734863;
            e.base.motion_z = 0.0;
        }
        let s = sq_mut(e);
        s.squid_pitch = (s.squid_pitch as f64 + (-90.0 - s.squid_pitch) as f64 * 0.02) as f32;
    }
}

/// `EntitySquid.moveEntityWithHeading(strafe, forward)`: moves by the
/// motion, ignoring the heading. No `super` call (no friction, no
/// gravity: [`on_living_update`] sets the motion).
// src: EntitySquid.java:127-129
pub fn move_entity_with_heading(e: &mut Entity, world: &mut World) {
    let (mx, my, mz) = (e.base.motion_x, e.base.motion_y, e.base.motion_z);
    dispatch::move_entity(e, world, mx, my, mz);
}

/// `EntitySquid.updatePlayerActionState()`: a 1-in-50 roll (always drawn),
/// or out of water (`inWater`, set by `handleWaterMovement`), or no swim
/// direction yet: a new random direction (horizontal 0.2, vertical −0.1 to
/// 0.1). Then `despawnEntity()`. No `super` call.
// src: EntitySquid.java:131-140
pub fn update_player_action_state(e: &mut Entity, world: &mut World) {
    let roll = e.base.rand.next_int_bounded(50) == 0;
    let s = sq(e);
    if roll
        || !e.base.in_water
        || s.random_motion_vec_x == 0.0
            && s.random_motion_vec_y == 0.0
            && s.random_motion_vec_z == 0.0
    {
        let var1 = e.base.rand.next_float() * PI_F * 2.0;
        let vx = math::cos(var1) * 0.2;
        let vy = -0.1 + e.base.rand.next_float() * 0.2;
        let vz = math::sin(var1) * 0.2;
        let s = sq_mut(e);
        s.random_motion_vec_x = vx;
        s.random_motion_vec_y = vy;
        s.random_motion_vec_z = vz;
    }
    living::despawn_entity(e, world);
}
