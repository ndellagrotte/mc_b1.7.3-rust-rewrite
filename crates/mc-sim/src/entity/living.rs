//! `EntityLiving` (EntityLiving.java): fields and the class's own method
//! bodies (WP-2.2; the fields and the first overrides were WP-0.5's).
//!
//! # Audit of EntityLiving.java (WP-2.2)
//!
//! | EntityLiving.java | Method | Port |
//! |---|---|---|
//! | 6-58 | fields | [`LivingData`] |
//! | 60-68 | constructor | [`LivingData::new`] |
//! | 70-71 | `entityInit` (empty) | the leaf constructors |
//! | 73-75 | `canEntityBeSeen` | [`can_entity_be_seen`] |
//! | 77-79 | `getEntityTexture` | [`dispatch::get_entity_texture`] |
//! | 81-91 | `canBeCollidedWith`, `canBePushed`, `getEyeHeight` | [`dispatch`] |
//! | 93-95 | `getTalkInterval` | [`dispatch::get_talk_interval`] |
//! | 97-103 | `playLivingSound` | [`play_living_sound`] |
//! | 105-174 | `onEntityUpdate` | [`on_entity_update`] |
//! | 176-185 | `spawnExplosionParticle` | [`spawn_explosion_particle`] |
//! | 187-191 | `updateRidden` | [`update_ridden`] |
//! | 193-201 | `setPositionAndRotation2` | [`set_position_and_rotation2`] |
//! | 203-290 | `onUpdate` | [`on_update`] |
//! | 292-294 | `setSize` (super only) | `EntityBase::set_size` |
//! | 296-305 | `heal` | [`heal`] |
//! | 307-365 | `attackEntityFrom` | [`attack_entity_from`] |
//! | 367-370 | `performHurtAnimation` | [`perform_hurt_animation`] |
//! | 372-390 | `damageEntity`, `getSoundVolume`, `getLivingSound`, `getHurtSound`, `getDeathSound` | [`damage_entity`]; the hooks in [`dispatch`] |
//! | 392-405 | `knockBack` | [`knock_back`] |
//! | 407-422 | `onDeath` | [`on_death`] |
//! | 424-438 | `dropFewItems`, `getDropItemId` | [`drop_few_items`]; [`dispatch::get_drop_item_id`] |
//! | 440-452 | `fall` | [`fall`] |
//! | 454-548 | `moveEntityWithHeading` | [`move_entity_with_heading`] |
//! | 550-555 | `isOnLadder` | [`is_on_ladder`] |
//! | 557-573 | `writeEntityToNBT`, `readEntityFromNBT` | [`super::nbt`] |
//! | 575-581 | `isEntityAlive`, `canBreatheUnderwater` | [`dispatch`] |
//! | 583-653 | `onLivingUpdate` | [`on_living_update`] |
//! | 655-661 | `isMovementBlocked`, `jump` | [`dispatch`]; [`jump`] |
//! | 663-687 | `canDespawn`, `despawnEntity` | [`dispatch::can_despawn`]; [`despawn_entity`] |
//! | 689-730 | `updatePlayerActionState`, `getVerticalFaceSpeed` | [`update_player_action_state`]; [`dispatch`] |
//! | 732-776 | `faceEntity`, `hasCurrentTarget`, `getCurrentTarget`, `updateRotation` | [`face_entity`]; `LivingData::current_target`; [`update_rotation`] |
//! | 778-787 | `onEntityDeath` (empty, no override), `getCanSpawnHere`, `kill` | inline in [`on_entity_update`]; [`get_can_spawn_here`]; [`kill`] |
//! | 789-840 | `getSwingProgress`, `getPosition`, `getLookVec`, `getLook`, `rayTrace` (render and client data) | [`get_swing_progress`], [`get_position`], [`get_look`], [`ray_trace`] |
//! | 842-848 | `getMaxSpawnedInChunk`, `getHeldItem` | [`dispatch`] |
//! | 850-866 | `handleHealthUpdate` | [`handle_health_update`] |
//! | 868-874 | `isPlayerSleeping`, `getItemIcon` | [`dispatch::is_player_sleeping`]; `getItemIcon` is render (WP-3.5) |
//!
//! # Other entities
//!
//! A Java reference to another entity (`currentTarget`, the attacker) is an
//! entity id ([`crate::entity`] module docs). Code that reads fields off it
//! takes an [`EntityView`] of it from the world. The attacker of
//! `attackEntityFrom` is in hand as `&Entity` (a copy for the projectile and
//! harness callers); `onDeath`'s `attacker.addToPlayerScore(this, …)` and
//! `attacker.onKillEntity(this)` mutate only an `EntityPlayer` (both are
//! empty in `Entity`), which a shared reference cannot reach: they record
//! `EntityPlayer.addToPlayerScore` / `EntityPlayer.onKillEntity` as the
//! dispatch would (WP-2.5).

use mc_core::math::{self, floor_double, jmath};
use mc_core::{Aabb, Vec3D};

use super::{Entity, EntityBase, base, dispatch};
use crate::block::{self, Material};
use crate::world::World;
use crate::world::ray_trace::MovingObjectPosition;

/// `Block.ladder.blockID`.
// src: Block.java:657
const LADDER: i32 = 65;

/// The fields `EntityLiving` declares (EntityLiving.java:6-58), in order.
/// Field initialisers that draw `Math.random()` are applied by
/// [`LivingData::new`] in Java's order. Renamed `field_NNNN` fields are
/// documented in docs/mappings.md.
#[derive(Clone, Debug, PartialEq)]
pub struct LivingData {
    /// `heartsHalvesLife`: the hurt-resistance window (`heartsLife` is set
    /// to it on a hit, EntityLiving.java:328).
    pub hearts_halves_life: i32,
    /// `field_9365_p`: `Math.random() * 12398` at construction, never read.
    pub random_unused1: f32,
    /// `field_9363_r`: `(Math.random() + 1) * 0.01` at construction, never
    /// read.
    pub random_unused2: f32,
    /// `renderYawOffset`: the body yaw.
    pub render_yaw_offset: f32,
    /// `prevRenderYawOffset`.
    pub prev_render_yaw_offset: f32,
    /// `field_9362_u`: last tick's [`LivingData::on_ground_speed_factor`].
    pub prev_on_ground_speed_factor: f32,
    /// `field_9361_v`: eases toward 1 while walking on the ground, 0
    /// otherwise (EntityLiving.java:227).
    pub on_ground_speed_factor: f32,
    /// `field_9360_w`: accumulated walk distance × 3, signed by facing
    /// (EntityLiving.java:289).
    pub moved_distance: f32,
    /// `field_9359_x`: last tick's [`LivingData::moved_distance`].
    pub prev_moved_distance: f32,
    /// `field_9358_y` (`true`, never read).
    pub unused_flag_y: bool,
    /// `texture`.
    pub texture: &'static str,
    /// `field_9355_A` (`true`, never read).
    pub unused_flag_a: bool,
    /// `unused180`.
    pub unused180: f32,
    /// `entityType`.
    pub entity_type: Option<&'static str>,
    /// `field_9349_D` (`1.0F`, never read).
    pub unused_float_d: f32,
    /// `scoreValue`.
    pub score_value: i32,
    /// `field_9345_F` (`0.0F`, never read).
    pub unused_float_f: f32,
    /// `isMultiplayerEntity`.
    pub is_multiplayer_entity: bool,
    /// `prevSwingProgress`.
    pub prev_swing_progress: f32,
    /// `swingProgress`.
    pub swing_progress: f32,
    /// `health`.
    pub health: i32,
    /// `prevHealth`.
    pub prev_health: i32,
    /// `livingSoundTime` (private).
    pub living_sound_time: i32,
    /// `hurtTime`.
    pub hurt_time: i32,
    /// `maxHurtTime`.
    pub max_hurt_time: i32,
    /// `attackedAtYaw`.
    pub attacked_at_yaw: f32,
    /// `deathTime`.
    pub death_time: i32,
    /// `attackTime`.
    pub attack_time: i32,
    /// `prevCameraPitch`.
    pub prev_camera_pitch: f32,
    /// `cameraPitch`.
    pub camera_pitch: f32,
    /// `unused_flag`: set by `onDeath`, never read.
    pub unused_flag: bool,
    /// `field_9326_T` (`-1`, never read).
    pub unused_int_t: i32,
    /// `field_9325_U`: `Math.random() * 0.9 + 0.1` at construction, never
    /// read.
    pub random_unused3: f32,
    /// `prevLegYaw`.
    pub prev_leg_yaw: f32,
    /// `legYaw`.
    pub leg_yaw: f32,
    /// `legSwing`.
    pub leg_swing: f32,
    /// `newPosRotationIncrements`.
    pub new_pos_rotation_increments: i32,
    /// `newPosX`.
    pub new_pos_x: f64,
    /// `newPosY`.
    pub new_pos_y: f64,
    /// `newPosZ`.
    pub new_pos_z: f64,
    /// `newRotationYaw`.
    pub new_rotation_yaw: f64,
    /// `newRotationPitch`.
    pub new_rotation_pitch: f64,
    /// `field_9348_ae` (`0.0F`, never read).
    pub unused_float_ae: f32,
    /// `field_9346_af`: the damage of the hit that started the current
    /// hurt-resistance window (EntityLiving.java:318-326).
    pub last_damage: i32,
    /// `entityAge`.
    pub entity_age: i32,
    /// `moveStrafing`.
    pub move_strafing: f32,
    /// `moveForward`.
    pub move_forward: f32,
    /// `randomYawVelocity`.
    pub random_yaw_velocity: f32,
    /// `isJumping`.
    pub is_jumping: bool,
    /// `defaultPitch`.
    pub default_pitch: f32,
    /// `moveSpeed`.
    pub move_speed: f32,
    /// `currentTarget` (private), as an entity id.
    pub current_target: Option<i32>,
    /// `numTicksToChaseTarget`.
    pub num_ticks_to_chase_target: i32,
}

impl LivingData {
    /// `EntityLiving`'s field initialisers (EntityLiving.java:6-58) and
    /// constructor body (:60-68), run after `Entity(World)` built `b`.
    ///
    /// `Math.random()` order: `field_9325_U` (field initialiser, :38), then
    /// `field_9363_r` (:63), `field_9365_p` (:65), `rotationYaw` (:66).
    // src: EntityLiving.java:6-68
    pub fn new(world: &mut World, b: &mut EntityBase) -> LivingData {
        let random_unused3 =
            (base::math_random(world) * 0.8999999761581421 + 0.10000000149011612) as f32;
        let mut l = LivingData {
            hearts_halves_life: 20,
            random_unused1: 0.0,
            random_unused2: 0.0,
            render_yaw_offset: 0.0,
            prev_render_yaw_offset: 0.0,
            prev_on_ground_speed_factor: 0.0,
            on_ground_speed_factor: 0.0,
            moved_distance: 0.0,
            prev_moved_distance: 0.0,
            unused_flag_y: true,
            texture: "/mob/char.png",
            unused_flag_a: true,
            unused180: 0.0,
            entity_type: None,
            unused_float_d: 1.0,
            score_value: 0,
            unused_float_f: 0.0,
            is_multiplayer_entity: false,
            prev_swing_progress: 0.0,
            swing_progress: 0.0,
            health: 10,
            prev_health: 0,
            living_sound_time: 0,
            hurt_time: 0,
            max_hurt_time: 0,
            attacked_at_yaw: 0.0,
            death_time: 0,
            attack_time: 0,
            prev_camera_pitch: 0.0,
            camera_pitch: 0.0,
            unused_flag: false,
            unused_int_t: -1,
            random_unused3,
            prev_leg_yaw: 0.0,
            leg_yaw: 0.0,
            leg_swing: 0.0,
            new_pos_rotation_increments: 0,
            new_pos_x: 0.0,
            new_pos_y: 0.0,
            new_pos_z: 0.0,
            new_rotation_yaw: 0.0,
            new_rotation_pitch: 0.0,
            unused_float_ae: 0.0,
            last_damage: 0,
            entity_age: 0,
            move_strafing: 0.0,
            move_forward: 0.0,
            random_yaw_velocity: 0.0,
            is_jumping: false,
            default_pitch: 0.0,
            move_speed: 0.7,
            current_target: None,
            num_ticks_to_chase_target: 0,
        };
        // Constructor body.
        b.prevent_entity_spawning = true;
        l.random_unused2 = (base::math_random(world) + 1.0) as f32 * 0.01;
        b.set_position(b.pos_x, b.pos_y, b.pos_z);
        l.random_unused1 = base::math_random(world) as f32 * 12398.0;
        b.rotation_yaw = (base::math_random(world) * 3.1415927410125732 * 2.0) as f32;
        b.step_height = 0.5;
        l
    }
}

/// The `EntityLiving` level of `e` (callers are `EntityLiving` methods, so
/// the kind is living).
pub(crate) fn lv(e: &Entity) -> &LivingData {
    e.kind
        .living()
        .expect("EntityLiving method on a non-living entity")
}

/// The `EntityLiving` level of `e`, mutably.
pub(crate) fn lv_mut(e: &mut Entity) -> &mut LivingData {
    e.kind
        .living_mut()
        .expect("EntityLiving method on a non-living entity")
}

/// What `EntityLiving` code reads off another entity it holds a reference
/// to (`currentTarget`, `EntityCreature.playerToAttack`), taken from the
/// world by id (module docs).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EntityView {
    /// `entityId`.
    pub id: i32,
    /// `posX`.
    pub pos_x: f64,
    /// `posY`.
    pub pos_y: f64,
    /// `posZ`.
    pub pos_z: f64,
    /// `boundingBox`.
    pub bounding_box: Aabb,
    /// `getEyeHeight()` (virtual).
    pub eye_height: f32,
    /// `instanceof EntityLiving`.
    pub is_living: bool,
    /// `isDead`.
    pub is_dead: bool,
}

impl EntityView {
    /// The view of a loaded entity; `None` (recorded) when it is not in
    /// `loadedEntityList` or is taken out, where Java would still hold the
    /// object.
    pub fn of(world: &World, id: i32) -> Option<EntityView> {
        match world.entity(id) {
            Some(t) => Some(EntityView {
                id,
                pos_x: t.base.pos_x,
                pos_y: t.base.pos_y,
                pos_z: t.base.pos_z,
                bounding_box: t.base.bounding_box,
                eye_height: dispatch::get_eye_height(t, world),
                is_living: t.kind.living().is_some(),
                is_dead: t.base.is_dead,
            }),
            None => {
                world.note_unimplemented(
                    "EntityLiving: a referenced entity is not in its slot (Java still holds it)",
                );
                None
            }
        }
    }

    /// `other.getDistanceSqToEntity(e)` / `e.getDistanceSqToEntity(other)`
    /// (the same bits: only the squares of the differences are summed).
    // src: Entity.java:699-704
    pub fn distance_sq_to(&self, e: &EntityBase) -> f64 {
        let var2 = self.pos_x - e.pos_x;
        let var4 = self.pos_y - e.pos_y;
        let var6 = self.pos_z - e.pos_z;
        var2 * var2 + var4 * var4 + var6 * var6
    }

    /// `other.getDistanceToEntity(e)` (float, as Entity.java computes it
    /// from `other`'s side).
    // src: Entity.java:678-683
    pub fn distance_to(&self, e: &EntityBase) -> f32 {
        let var2 = (self.pos_x - e.pos_x) as f32;
        let var3 = (self.pos_y - e.pos_y) as f32;
        let var4 = (self.pos_z - e.pos_z) as f32;
        math::sqrt_float(var2 * var2 + var3 * var3 + var4 * var4)
    }
}

/// `canEntityBeSeen(other)`: no block between the two eyes
/// (`rayTraceBlocks`, which loads chunks on the way).
// src: EntityLiving.java:73-75
pub fn can_entity_be_seen(e: &Entity, world: &mut World, other: &EntityView) -> bool {
    let eye = dispatch::get_eye_height(e, world);
    let mut from = Vec3D::create_vector(e.base.pos_x, e.base.pos_y + eye as f64, e.base.pos_z);
    let to = Vec3D::create_vector(
        other.pos_x,
        other.pos_y + other.eye_height as f64,
        other.pos_z,
    );
    world.ray_trace_blocks(&mut from, &to).is_none()
}

/// `playLivingSound()`: the living sound (virtual `getLivingSound`), if any,
/// at `getSoundVolume()` and pitch `(r1 - r2) * 0.2 + 1` from `rand`.
// src: EntityLiving.java:97-103
pub fn play_living_sound(e: &mut Entity, world: &mut World) {
    if let Some(var1) = dispatch::get_living_sound(e, world) {
        let volume = dispatch::get_sound_volume(e, world);
        let pitch = (e.base.rand.next_float() - e.base.rand.next_float()) * 0.2 + 1.0;
        world.play_sound_at_entity(&e.base, var1, volume, pitch);
    }
}

/// `EntityLiving.onEntityUpdate()`: `Entity`'s, then the living sound
/// timer, suffocation, drowning, the attack/hurt/resistance timers and
/// the death animation (20 ticks, then `setEntityDead` and 20 particles).
// src: EntityLiving.java:105-174
pub fn on_entity_update(e: &mut Entity, world: &mut World) {
    {
        let l = lv_mut(e);
        l.prev_swing_progress = l.swing_progress;
    }
    base::on_entity_update(e, world);
    // `rand.nextInt(1000) < livingSoundTime++`: the draw, then the old value.
    let r = e.base.rand.next_int_bounded(1000);
    let old = {
        let l = lv_mut(e);
        let old = l.living_sound_time;
        l.living_sound_time = old.wrapping_add(1);
        old
    };
    if r < old {
        let talk = dispatch::get_talk_interval(e, world);
        lv_mut(e).living_sound_time = talk.wrapping_neg();
        play_living_sound(e, world);
    }

    if dispatch::is_entity_alive(e) && dispatch::is_entity_inside_opaque_block(e, world) {
        dispatch::attack_entity_from(e, world, None, 1);
    }

    if e.base.is_immune_to_fire || world.multiplayer_world {
        e.base.fire = 0;
    }

    if dispatch::is_entity_alive(e)
        && base::is_inside_of_material(e, world, Material::Water)
        && !dispatch::can_breathe_underwater(e, world)
    {
        e.base.air -= 1;
        if e.base.air == -20 {
            e.base.air = 0;
            for _ in 0..8 {
                let b = &mut e.base;
                let var2 = b.rand.next_float() - b.rand.next_float();
                let var3 = b.rand.next_float() - b.rand.next_float();
                let var4 = b.rand.next_float() - b.rand.next_float();
                world.spawn_particle(
                    "bubble",
                    b.pos_x + var2 as f64,
                    b.pos_y + var3 as f64,
                    b.pos_z + var4 as f64,
                    b.motion_x,
                    b.motion_y,
                    b.motion_z,
                );
            }
            dispatch::attack_entity_from(e, world, None, 2);
        }
        e.base.fire = 0;
    } else {
        e.base.air = e.base.max_air;
    }

    {
        let l = lv_mut(e);
        l.prev_camera_pitch = l.camera_pitch;
        if l.attack_time > 0 {
            l.attack_time -= 1;
        }
        if l.hurt_time > 0 {
            l.hurt_time -= 1;
        }
    }
    if e.base.hearts_life > 0 {
        e.base.hearts_life -= 1;
    }

    if lv(e).health <= 0 {
        let l = lv_mut(e);
        l.death_time += 1;
        if l.death_time > 20 {
            // onEntityDeath() (EntityLiving.java:778-779): empty, overridden
            // by no class.
            dispatch::set_entity_dead(e, world);
            for _ in 0..20 {
                let b = &mut e.base;
                let var8 = b.rand.next_gaussian() * 0.02;
                let var9 = b.rand.next_gaussian() * 0.02;
                let var6 = b.rand.next_gaussian() * 0.02;
                let x = b.pos_x + (b.rand.next_float() * b.width * 2.0) as f64 - b.width as f64;
                let y = b.pos_y + (b.rand.next_float() * b.height) as f64;
                let z = b.pos_z + (b.rand.next_float() * b.width * 2.0) as f64 - b.width as f64;
                world.spawn_particle("explode", x, y, z, var8, var9, var6);
            }
        }
    }

    let l = lv_mut(e);
    l.prev_moved_distance = l.moved_distance;
    l.prev_render_yaw_offset = l.render_yaw_offset;
    e.base.prev_rotation_yaw = e.base.rotation_yaw;
    e.base.prev_rotation_pitch = e.base.rotation_pitch;
}

/// `spawnExplosionParticle()`: 20 `"explode"` particles around the box
/// (the mob spawner's effect, TileEntityMobSpawner.java:76).
// src: EntityLiving.java:176-185
pub fn spawn_explosion_particle(e: &mut Entity, world: &mut World) {
    let b = &mut e.base;
    for _ in 0..20 {
        let var2 = b.rand.next_gaussian() * 0.02;
        let var4 = b.rand.next_gaussian() * 0.02;
        let var6 = b.rand.next_gaussian() * 0.02;
        let var8 = 10.0;
        let x =
            b.pos_x + (b.rand.next_float() * b.width * 2.0) as f64 - b.width as f64 - var2 * var8;
        let y = b.pos_y + (b.rand.next_float() * b.height) as f64 - var4 * var8;
        let z =
            b.pos_z + (b.rand.next_float() * b.width * 2.0) as f64 - b.width as f64 - var6 * var8;
        world.spawn_particle("explode", x, y, z, var2, var4, var6);
    }
}

/// `EntityLiving.updateRidden()`.
// src: EntityLiving.java:187-191
pub fn update_ridden(e: &mut Entity, world: &mut World) {
    base::update_ridden(e, world);
    let l = lv_mut(e);
    l.prev_on_ground_speed_factor = l.on_ground_speed_factor;
    l.on_ground_speed_factor = 0.0;
}

/// `EntityLiving.setPositionAndRotation2(x, y, z, yaw, pitch, increments)`:
/// the multiplayer interpolation target, consumed by `onLivingUpdate`.
// src: EntityLiving.java:193-201
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
    e.base.y_offset = 0.0;
    let l = lv_mut(e);
    l.new_pos_x = var1;
    l.new_pos_y = var3;
    l.new_pos_z = var5;
    l.new_rotation_yaw = var7 as f64;
    l.new_rotation_pitch = var8 as f64;
    l.new_pos_rotation_increments = var9;
}

/// `EntityLiving.onUpdate()`: `Entity.onUpdate`, `onLivingUpdate` (virtual),
/// then the body yaw and walk animation state the renderer reads.
// src: EntityLiving.java:203-290
#[allow(clippy::manual_clamp)] // Java's two ifs, kept (NaN behaviour)
pub fn on_update(e: &mut Entity, world: &mut World) {
    base::on_update(e, world);
    dispatch::on_living_update(e, world);
    let b = &e.base;
    let var1 = b.pos_x - b.prev_pos_x;
    let var3 = b.pos_z - b.prev_pos_z;
    let var5 = math::sqrt_double(var1 * var1 + var3 * var3);
    let on_ground = b.on_ground;
    let rotation_yaw = b.rotation_yaw;
    let l = lv_mut(e);
    let mut var6 = l.render_yaw_offset;
    let mut var7 = 0.0f32;
    l.prev_on_ground_speed_factor = l.on_ground_speed_factor;
    let mut var8 = 0.0f32;
    if var5 > 0.05 {
        var8 = 1.0;
        var7 = var5 * 3.0;
        var6 = (jmath::atan2(var3, var1) as f32) * 180.0 / 3.1415927 - 90.0;
    }
    if l.swing_progress > 0.0 {
        var6 = rotation_yaw;
    }
    if !on_ground {
        var8 = 0.0;
    }
    l.on_ground_speed_factor += (var8 - l.on_ground_speed_factor) * 0.3;

    let mut var9 = var6 - l.render_yaw_offset;
    while var9 < -180.0 {
        var9 += 360.0;
    }
    while var9 >= 180.0 {
        var9 -= 360.0;
    }
    l.render_yaw_offset += var9 * 0.3;

    let mut var10 = rotation_yaw - l.render_yaw_offset;
    while var10 < -180.0 {
        var10 += 360.0;
    }
    while var10 >= 180.0 {
        var10 -= 360.0;
    }
    let var11 = var10 < -90.0 || var10 >= 90.0;
    if var10 < -75.0 {
        var10 = -75.0;
    }
    if var10 >= 75.0 {
        var10 = 75.0;
    }
    l.render_yaw_offset = rotation_yaw - var10;
    if var10 * var10 > 2500.0 {
        l.render_yaw_offset += var10 * 0.2;
    }
    if var11 {
        var7 *= -1.0;
    }

    let render_yaw_offset = l.render_yaw_offset;
    let b = &mut e.base;
    while b.rotation_yaw - b.prev_rotation_yaw < -180.0 {
        b.prev_rotation_yaw -= 360.0;
    }
    while b.rotation_yaw - b.prev_rotation_yaw >= 180.0 {
        b.prev_rotation_yaw += 360.0;
    }
    let l = lv_mut(e);
    while render_yaw_offset - l.prev_render_yaw_offset < -180.0 {
        l.prev_render_yaw_offset -= 360.0;
    }
    while render_yaw_offset - l.prev_render_yaw_offset >= 180.0 {
        l.prev_render_yaw_offset += 360.0;
    }
    let b = &mut e.base;
    while b.rotation_pitch - b.prev_rotation_pitch < -180.0 {
        b.prev_rotation_pitch -= 360.0;
    }
    while b.rotation_pitch - b.prev_rotation_pitch >= 180.0 {
        b.prev_rotation_pitch += 360.0;
    }
    lv_mut(e).moved_distance += var7;
}

/// `heal(amount)`: up to 20 health, and restarts half the resistance
/// window; nothing once dead.
// src: EntityLiving.java:296-305
pub fn heal(e: &mut Entity, var1: i32) {
    let l = lv_mut(e);
    if l.health > 0 {
        l.health += var1;
        if l.health > 20 {
            l.health = 20;
        }
        let half = l.hearts_halves_life / 2;
        e.base.hearts_life = half;
    }
}

/// `EntityLiving.attackEntityFrom(attacker, damage)`.
///
/// Inside the resistance window (`heartsLife > heartsHalvesLife / 2`) only
/// damage above the window's first hit counts, as the difference, with no
/// knock-back, hurt animation or sound. Otherwise the full damage, the
/// window restarts, `hurtTime` 10, the attacked state, and knock-back away
/// from the attacker (a random direction from `Math.random()` while the two
/// are closer than 0.01) or, with no attacker, a random `attackedAtYaw` of
/// 0 or 180 (one `Math.random()`). Then the death sound and `onDeath`, or
/// the hurt sound.
// src: EntityLiving.java:307-365
pub fn attack_entity_from(
    e: &mut Entity,
    world: &mut World,
    var1: Option<&Entity>,
    var2: i32,
) -> bool {
    if world.multiplayer_world {
        return false;
    }
    lv_mut(e).entity_age = 0;
    if lv(e).health <= 0 {
        return false;
    }
    lv_mut(e).leg_yaw = 1.5;
    let mut var3 = true;
    let window = (e.base.hearts_life as f32) > (lv(e).hearts_halves_life as f32) / 2.0;
    if window {
        let last = lv(e).last_damage;
        if var2 <= last {
            return false;
        }
        dispatch::damage_entity(e, world, var2 - last);
        lv_mut(e).last_damage = var2;
        var3 = false;
    } else {
        let l = lv_mut(e);
        l.last_damage = var2;
        l.prev_health = l.health;
        let window_len = l.hearts_halves_life;
        e.base.hearts_life = window_len;
        dispatch::damage_entity(e, world, var2);
        let l = lv_mut(e);
        l.max_hurt_time = 10;
        l.hurt_time = l.max_hurt_time;
    }

    lv_mut(e).attacked_at_yaw = 0.0;
    if var3 {
        world.set_entity_state(e.base.entity_id, 2);
        e.base.set_been_attacked();
        if let Some(a) = var1 {
            let mut var4 = a.base.pos_x - e.base.pos_x;
            let mut var6 = a.base.pos_z - e.base.pos_z;
            // The `for` body draws var4, its update var6.
            while var4 * var4 + var6 * var6 < 1.0E-4 {
                var4 = (base::math_random(world) - base::math_random(world)) * 0.01;
                var6 = (base::math_random(world) - base::math_random(world)) * 0.01;
            }
            let yaw = e.base.rotation_yaw;
            lv_mut(e).attacked_at_yaw =
                (jmath::atan2(var6, var4) * 180.0 / 3.1415927410125732) as f32 - yaw;
            knock_back(e, a, var2, var4, var6);
        } else {
            lv_mut(e).attacked_at_yaw = (((base::math_random(world) * 2.0) as i32) * 180) as f32;
        }
    }

    if lv(e).health <= 0 {
        if var3 {
            let name = dispatch::get_death_sound(e, world);
            hurt_or_death_sound(e, world, name);
        }
        dispatch::on_death(e, world, var1);
    } else if var3 {
        let name = dispatch::get_hurt_sound(e, world);
        hurt_or_death_sound(e, world, name);
    }
    true
}

/// `worldObj.playSoundAtEntity(this, name, getSoundVolume(), (r1 - r2) *
/// 0.2F + 1.0F)`. Java passes the name on even when it is `null` (a sound
/// listener ignores it); the two `rand` draws happen either way.
fn hurt_or_death_sound(e: &mut Entity, world: &mut World, name: Option<&'static str>) {
    let volume = dispatch::get_sound_volume(e, world);
    let pitch = (e.base.rand.next_float() - e.base.rand.next_float()) * 0.2 + 1.0;
    if let Some(name) = name {
        world.play_sound_at_entity(&e.base, name, volume, pitch);
    }
}

/// `performHurtAnimation()` (client: `Packet`-driven hurt effect).
// src: EntityLiving.java:367-370
pub fn perform_hurt_animation(e: &mut Entity) {
    let l = lv_mut(e);
    l.max_hurt_time = 10;
    l.hurt_time = 10;
    l.attacked_at_yaw = 0.0;
}

/// `EntityLiving.damageEntity(amount)`.
// src: EntityLiving.java:372-374
pub fn damage_entity(e: &mut Entity, var1: i32) {
    let l = lv_mut(e);
    l.health = l.health.wrapping_sub(var1);
}

/// `knockBack(attacker, damage, dx, dz)`: halves the motion, pushes 0.4
/// away from `(dx, dz)` and 0.4 up (capped at 0.4). `attacker` and
/// `damage` are unused, as in Java.
// src: EntityLiving.java:392-405
pub fn knock_back(e: &mut Entity, _var1: &Entity, _var2: i32, var3: f64, var5: f64) {
    let var7 = math::sqrt_double(var3 * var3 + var5 * var5);
    let var8 = 0.4f32;
    let b = &mut e.base;
    b.motion_x /= 2.0;
    b.motion_y /= 2.0;
    b.motion_z /= 2.0;
    b.motion_x -= var3 / var7 as f64 * var8 as f64;
    b.motion_y += 0.4000000059604645;
    b.motion_z -= var5 / var7 as f64 * var8 as f64;
    if b.motion_y > 0.4000000059604645 {
        b.motion_y = 0.4000000059604645;
    }
}

/// `EntityLiving.onDeath(attacker)`: score and kill hooks of the attacker
/// (module docs), the drops, the death state.
// src: EntityLiving.java:407-422
pub fn on_death(e: &mut Entity, world: &mut World, var1: Option<&Entity>) {
    if lv(e).score_value >= 0
        && let Some(a) = var1
        && a.kind.is_player()
    {
        // var1.addToPlayerScore(this, scoreValue): Entity's body is empty
        // (Entity.java:759-760), EntityPlayer's is WP-2.5.
        world.note_unimplemented("EntityPlayer.addToPlayerScore");
    }
    if let Some(a) = var1
        && a.kind.is_player()
    {
        // var1.onKillEntity(this): empty in Entity (Entity.java:1094-1095).
        world.note_unimplemented("EntityPlayer.onKillEntity");
    }
    lv_mut(e).unused_flag = true;
    if !world.multiplayer_world {
        dispatch::drop_few_items(e, world);
    }
    world.set_entity_state(e.base.entity_id, 3);
}

/// `EntityLiving.dropFewItems()`: 0 to 2 of `getDropItemId()` (virtual), one
/// `dropItem` each.
// src: EntityLiving.java:424-434
pub fn drop_few_items(e: &mut Entity, world: &mut World) {
    let var1 = dispatch::get_drop_item_id(e, world);
    if var1 > 0 {
        let var2 = e.base.rand.next_int_bounded(3);
        for _ in 0..var2 {
            base::drop_item(e, world, var1, 1);
        }
    }
}

/// `EntityLiving.fall(distance)`: damage `ceil(distance - 3)` and a landing
/// sound.
// src: EntityLiving.java:440-452
pub fn fall(e: &mut Entity, world: &mut World, distance: f32) {
    base::fall(e, world, distance);
    let var2 = ((distance - 3.0) as f64).ceil() as i32;
    if var2 > 0 {
        dispatch::attack_entity_from(e, world, None, var2);
        let var3 = world.get_block_id(
            floor_double(e.base.pos_x),
            floor_double(e.base.pos_y - 0.20000000298023224 - e.base.y_offset as f64),
            floor_double(e.base.pos_z),
        );
        if var3 > 0
            && let Some(b) = block::block(var3)
        {
            let var4 = b.step_sound;
            world.play_sound_at_entity(
                &e.base,
                &var4.step_sound_dir2(),
                var4.get_volume() * 0.5,
                var4.get_pitch() * 0.75,
            );
        }
    }
}

/// `Block.blocksList[id].slipperiness * 0.91F` of the block under the
/// entity's feet, or 0.546 (air: `0.6 * 0.91`) when the id is 0.
// src: EntityLiving.java:481-485, :491-496
fn ground_friction(e: &Entity, world: &mut World) -> f32 {
    let mut var8 = 0.54600006f32;
    let var4 = world.get_block_id(
        floor_double(e.base.pos_x),
        floor_double(e.base.bounding_box.min_y) - 1,
        floor_double(e.base.pos_z),
    );
    if var4 > 0
        && let Some(b) = block::block(var4)
    {
        var8 = b.slipperiness * 0.91;
    }
    var8
}

/// `EntityLiving.moveEntityWithHeading(strafe, forward)`: one tick of
/// self-propelled movement in water, lava, or air/ground (with ladder
/// clamps), then the leg animation.
// src: EntityLiving.java:454-548
pub fn move_entity_with_heading(e: &mut Entity, world: &mut World, var1: f32, var2: f32) {
    if dispatch::is_in_water(e, world) {
        liquid_move(e, world, var1, var2, 0.800000011920929);
    } else if base::handle_lava_movement(e, world) {
        liquid_move(e, world, var1, var2, 0.5);
    } else {
        let mut var8 = 0.91f32;
        if e.base.on_ground {
            var8 = ground_friction(e, world);
        }
        let var9 = 0.16277136f32 / (var8 * var8 * var8);
        let speed = if e.base.on_ground { 0.1 * var9 } else { 0.02 };
        base::move_flying(&mut e.base, var1, var2, speed);
        var8 = 0.91;
        if e.base.on_ground {
            var8 = ground_friction(e, world);
        }
        if dispatch::is_on_ladder(e, world) {
            let var10 = 0.15f32;
            let b = &mut e.base;
            if b.motion_x < -var10 as f64 {
                b.motion_x = -var10 as f64;
            }
            if b.motion_x > var10 as f64 {
                b.motion_x = var10 as f64;
            }
            if b.motion_z < -var10 as f64 {
                b.motion_z = -var10 as f64;
            }
            if b.motion_z > var10 as f64 {
                b.motion_z = var10 as f64;
            }
            b.fall_distance = 0.0;
            if b.motion_y < -0.15 {
                b.motion_y = -0.15;
            }
            if dispatch::is_sneaking(e, world) && e.base.motion_y < 0.0 {
                e.base.motion_y = 0.0;
            }
        }
        let (mx, my, mz) = (e.base.motion_x, e.base.motion_y, e.base.motion_z);
        dispatch::move_entity(e, world, mx, my, mz);
        if e.base.is_collided_horizontally && dispatch::is_on_ladder(e, world) {
            e.base.motion_y = 0.2;
        }
        let b = &mut e.base;
        b.motion_y -= 0.08;
        b.motion_y *= 0.9800000190734863;
        b.motion_x *= var8 as f64;
        b.motion_z *= var8 as f64;
    }

    let b = &e.base;
    let var3 = b.pos_x - b.prev_pos_x;
    let var11 = b.pos_z - b.prev_pos_z;
    let mut var7 = math::sqrt_double(var3 * var3 + var11 * var11) * 4.0;
    if var7 > 1.0 {
        var7 = 1.0;
    }
    let l = lv_mut(e);
    l.prev_leg_yaw = l.leg_yaw;
    l.leg_yaw += (var7 - l.leg_yaw) * 0.4;
    l.leg_swing += l.leg_yaw;
}

/// The water and lava branches of `moveEntityWithHeading`: they differ only
/// in the drag (`0.800000011920929` in water, `0.5` in lava).
// src: EntityLiving.java:456-477
fn liquid_move(e: &mut Entity, world: &mut World, var1: f32, var2: f32, drag: f64) {
    let var3 = e.base.pos_y;
    base::move_flying(&mut e.base, var1, var2, 0.02);
    let (mx, my, mz) = (e.base.motion_x, e.base.motion_y, e.base.motion_z);
    dispatch::move_entity(e, world, mx, my, mz);
    let b = &mut e.base;
    b.motion_x *= drag;
    b.motion_y *= drag;
    b.motion_z *= drag;
    b.motion_y -= 0.02;
    if e.base.is_collided_horizontally {
        let b = &e.base;
        let dy = b.motion_y + 0.6000000238418579 - b.pos_y + var3;
        let (mx, mz) = (b.motion_x, b.motion_z);
        if base::is_offset_position_in_liquid(e, world, mx, dy, mz) {
            e.base.motion_y = 0.30000001192092896;
        }
    }
}

/// `EntityLiving.isOnLadder()`: the block at the feet is a ladder.
// src: EntityLiving.java:550-555
pub fn is_on_ladder(e: &Entity, world: &mut World) -> bool {
    let var1 = floor_double(e.base.pos_x);
    let var2 = floor_double(e.base.bounding_box.min_y);
    let var3 = floor_double(e.base.pos_z);
    world.get_block_id(var1, var2, var3) == LADDER
}

/// `EntityLiving.onLivingUpdate()`: the multiplayer position interpolation,
/// the AI (`updatePlayerActionState`, virtual), swimming up or jumping,
/// input decay, `moveEntityWithHeading` (virtual), then pushing the
/// pushable entities within 0.2 blocks.
// src: EntityLiving.java:583-653
pub fn on_living_update(e: &mut Entity, world: &mut World) {
    if lv(e).new_pos_rotation_increments > 0 {
        let l = lv(e);
        let n = l.new_pos_rotation_increments as f64;
        let var1 = e.base.pos_x + (l.new_pos_x - e.base.pos_x) / n;
        let mut var3 = e.base.pos_y + (l.new_pos_y - e.base.pos_y) / n;
        let var5 = e.base.pos_z + (l.new_pos_z - e.base.pos_z) / n;
        let mut var7 = l.new_rotation_yaw - e.base.rotation_yaw as f64;
        while var7 < -180.0 {
            var7 += 360.0;
        }
        while var7 >= 180.0 {
            var7 -= 360.0;
        }
        let new_pitch = l.new_rotation_pitch;
        let b = &mut e.base;
        b.rotation_yaw = (b.rotation_yaw as f64 + var7 / n) as f32;
        b.rotation_pitch =
            (b.rotation_pitch as f64 + (new_pitch - b.rotation_pitch as f64) / n) as f32;
        lv_mut(e).new_pos_rotation_increments -= 1;
        e.base.set_position(var1, var3, var5);
        e.base
            .set_rotation(e.base.rotation_yaw, e.base.rotation_pitch);
        let bb = e.base.bounding_box.contract(0.03125, 0.0, 0.03125);
        let var9 = world.get_colliding_bounding_boxes(e, &bb);
        if !var9.is_empty() {
            let mut var10 = 0.0;
            for var13 in &var9 {
                if var13.max_y > var10 {
                    var10 = var13.max_y;
                }
            }
            var3 += var10 - e.base.bounding_box.min_y;
            e.base.set_position(var1, var3, var5);
        }
    }

    if dispatch::is_movement_blocked(e, world) {
        let l = lv_mut(e);
        l.is_jumping = false;
        l.move_strafing = 0.0;
        l.move_forward = 0.0;
        l.random_yaw_velocity = 0.0;
    } else if !lv(e).is_multiplayer_entity {
        dispatch::update_player_action_state(e, world);
    }

    let var14 = dispatch::is_in_water(e, world);
    let var2 = base::handle_lava_movement(e, world);
    if lv(e).is_jumping {
        if var14 || var2 {
            e.base.motion_y += 0.03999999910593033;
        } else if e.base.on_ground {
            dispatch::jump(e, world);
        }
    }

    let l = lv_mut(e);
    l.move_strafing *= 0.98;
    l.move_forward *= 0.98;
    l.random_yaw_velocity *= 0.9;
    let (strafe, forward) = (l.move_strafing, l.move_forward);
    dispatch::move_entity_with_heading(e, world, strafe, forward);

    let bb = e
        .base
        .bounding_box
        .expand(0.20000000298023224, 0.0, 0.20000000298023224);
    let var15 = world.get_entities_within_aabb_excluding_entity(Some(e.base.entity_id), &bb);
    for id in var15 {
        world.with_entity_mut(id, |w, var16| {
            if dispatch::can_be_pushed(var16, w) {
                dispatch::apply_entity_collision(var16, w, e);
            }
        });
    }
}

/// `EntityLiving.jump()`.
// src: EntityLiving.java:659-661
pub fn jump(e: &mut Entity) {
    e.base.motion_y = 0.41999998688697815;
}

/// `despawnEntity()`: with a player in the world and `canDespawn()`
/// (virtual): dead beyond 128 blocks; past age 600, a 1-in-800 roll each
/// tick that resets the age within 32 blocks or despawns beyond.
// src: EntityLiving.java:667-687
pub fn despawn_entity(e: &mut Entity, world: &mut World) {
    let var1 = world.get_closest_player_to_entity(&e.base, -1.0);
    if dispatch::can_despawn(e, world)
        && let Some(pid) = var1
        && let Some(p) = world.entity(pid)
    {
        let var2 = p.base.pos_x - e.base.pos_x;
        let var4 = p.base.pos_y - e.base.pos_y;
        let var6 = p.base.pos_z - e.base.pos_z;
        let var8 = var2 * var2 + var4 * var4 + var6 * var6;
        if var8 > 16384.0 {
            dispatch::set_entity_dead(e, world);
        }
        if lv(e).entity_age > 600 && e.base.rand.next_int_bounded(800) == 0 {
            if var8 < 1024.0 {
                lv_mut(e).entity_age = 0;
            } else {
                dispatch::set_entity_dead(e, world);
            }
        }
    }
}

/// `EntityLiving.updatePlayerActionState()`: the idle AI. Ages, despawns,
/// stops, then: a 2 % roll to look at the closest player within 8 blocks
/// for 10–29 ticks (else a new random yaw speed); while looking, turn
/// toward it; otherwise a 5 % roll for a new yaw speed and turn by it.
/// Swims (80 % jump chance) in water or lava.
// src: EntityLiving.java:689-726
pub fn update_player_action_state(e: &mut Entity, world: &mut World) {
    lv_mut(e).entity_age += 1;
    // `var1 = getClosestPlayerToEntity(this, -1.0D)`: no side effect, and
    // overwritten before any read.
    let _var1 = world.get_closest_player_to_entity(&e.base, -1.0);
    despawn_entity(e, world);
    {
        let l = lv_mut(e);
        l.move_strafing = 0.0;
        l.move_forward = 0.0;
    }
    let var2 = 8.0f32;
    if e.base.rand.next_float() < 0.02 {
        let var1 = world.get_closest_player_to_entity(&e.base, var2 as f64);
        match var1 {
            Some(p) => {
                lv_mut(e).current_target = Some(p);
                let n = 10 + e.base.rand.next_int_bounded(20);
                lv_mut(e).num_ticks_to_chase_target = n;
            }
            None => {
                let v = (e.base.rand.next_float() - 0.5) * 20.0;
                lv_mut(e).random_yaw_velocity = v;
            }
        }
    }

    if let Some(target) = lv(e).current_target {
        let view = EntityView::of(world, target);
        if let Some(v) = &view {
            let speed = dispatch::get_vertical_face_speed(e, world) as f32;
            face_entity(e, world, v, 10.0, speed);
        }
        let l = lv_mut(e);
        let ticks = l.num_ticks_to_chase_target;
        l.num_ticks_to_chase_target = ticks.wrapping_sub(1);
        let gone = match &view {
            Some(v) => v.is_dead || v.distance_sq_to(&e.base) > (var2 * var2) as f64,
            None => world.is_entity_dead_or_gone(target),
        };
        if ticks <= 0 || gone {
            lv_mut(e).current_target = None;
        }
    } else {
        if e.base.rand.next_float() < 0.05 {
            let v = (e.base.rand.next_float() - 0.5) * 20.0;
            lv_mut(e).random_yaw_velocity = v;
        }
        let l = lv(e);
        let (vel, pitch) = (l.random_yaw_velocity, l.default_pitch);
        e.base.rotation_yaw += vel;
        e.base.rotation_pitch = pitch;
    }

    let var3 = dispatch::is_in_water(e, world);
    let var4 = base::handle_lava_movement(e, world);
    if var3 || var4 {
        let j = e.base.rand.next_float() < 0.8;
        lv_mut(e).is_jumping = j;
    }
}

/// `faceEntity(target, maxYawStep, maxPitchStep)`: turns toward `target`'s
/// eyes (a living target) or box centre (any other).
///
/// vanilla bug (transcribed): the target pitch is stepped toward from the
/// stored pitch and the result stored negated
/// (`rotationPitch = -updateRotation(rotationPitch, …)`), so toward a target
/// off level the pitch alternates between two values instead of settling;
/// and the height difference has opposite signs in the two cases (`this -
/// target` for a living target, `target - this` otherwise), so a non-living
/// target above is first looked at downward.
// src: EntityLiving.java:732-748
pub fn face_entity(e: &mut Entity, world: &World, var1: &EntityView, var2: f32, var3: f32) {
    let var4 = var1.pos_x - e.base.pos_x;
    let var8 = var1.pos_z - e.base.pos_z;
    let eye = dispatch::get_eye_height(e, world);
    let var6 = if var1.is_living {
        e.base.pos_y + eye as f64 - (var1.pos_y + var1.eye_height as f64)
    } else {
        (var1.bounding_box.min_y + var1.bounding_box.max_y) / 2.0 - (e.base.pos_y + eye as f64)
    };
    let var14 = math::sqrt_double(var4 * var4 + var8 * var8) as f64;
    let var12 = (jmath::atan2(var8, var4) * 180.0 / 3.1415927410125732) as f32 - 90.0;
    let var13 = (-(jmath::atan2(var6, var14) * 180.0 / 3.1415927410125732)) as f32;
    e.base.rotation_pitch = -update_rotation(e.base.rotation_pitch, var13, var3);
    e.base.rotation_yaw = update_rotation(e.base.rotation_yaw, var12, var2);
}

/// `updateRotation(current, target, maxStep)` (private): steps `current`
/// toward `target` along the shorter way, at most `maxStep` degrees.
// src: EntityLiving.java:758-776
#[allow(clippy::manual_clamp)] // Java's two ifs, kept (NaN behaviour)
pub fn update_rotation(var1: f32, var2: f32, var3: f32) -> f32 {
    let mut var4 = var2 - var1;
    while var4 < -180.0 {
        var4 += 360.0;
    }
    while var4 >= 180.0 {
        var4 -= 360.0;
    }
    if var4 > var3 {
        var4 = var3;
    }
    if var4 < -var3 {
        var4 = -var3;
    }
    var1 + var4
}

/// `EntityLiving.getCanSpawnHere()`: no spawn-blocking entity in the box,
/// no collision, no liquid.
// src: EntityLiving.java:781-783
pub fn get_can_spawn_here(e: &Entity, world: &mut World) -> bool {
    let bb = e.base.bounding_box;
    world.check_if_aabb_is_clear(&bb)
        && world.get_colliding_bounding_boxes(e, &bb).is_empty()
        && !world.get_is_any_liquid(&bb)
}

/// `EntityLiving.kill()`: `attackEntityFrom(null, 4)`.
// src: EntityLiving.java:785-787
pub fn kill(e: &mut Entity, world: &mut World) {
    dispatch::attack_entity_from(e, world, None, 4);
}

/// `getSwingProgress(partialTick)` (render).
// src: EntityLiving.java:789-796
pub fn get_swing_progress(e: &Entity, var1: f32) -> f32 {
    let l = lv(e);
    let mut var2 = l.swing_progress - l.prev_swing_progress;
    if var2 < 0.0 {
        var2 += 1.0;
    }
    l.prev_swing_progress + var2 * var1
}

/// `getPosition(partialTick)` (client).
// src: EntityLiving.java:798-807
pub fn get_position(e: &Entity, var1: f32) -> Vec3D {
    let b = &e.base;
    if var1 == 1.0 {
        Vec3D::create_vector(b.pos_x, b.pos_y, b.pos_z)
    } else {
        let var2 = b.prev_pos_x + (b.pos_x - b.prev_pos_x) * var1 as f64;
        let var4 = b.prev_pos_y + (b.pos_y - b.prev_pos_y) * var1 as f64;
        let var6 = b.prev_pos_z + (b.pos_z - b.prev_pos_z) * var1 as f64;
        Vec3D::create_vector(var2, var4, var6)
    }
}

/// `getLook(partialTick)`: the unit look vector (`getLookVec()` is
/// `getLook(1.0F)`, EntityLiving.java:809-811).
// src: EntityLiving.java:813-833
pub fn get_look(e: &Entity, var1: f32) -> Vec3D {
    let b = &e.base;
    if var1 == 1.0 {
        let var2 = math::cos(-b.rotation_yaw * 0.017453292 - 3.1415927);
        let var3 = math::sin(-b.rotation_yaw * 0.017453292 - 3.1415927);
        let var4 = -math::cos(-b.rotation_pitch * 0.017453292);
        let var5 = math::sin(-b.rotation_pitch * 0.017453292);
        Vec3D::create_vector((var3 * var4) as f64, var5 as f64, (var2 * var4) as f64)
    } else {
        let var2 = b.prev_rotation_pitch + (b.rotation_pitch - b.prev_rotation_pitch) * var1;
        let var3 = b.prev_rotation_yaw + (b.rotation_yaw - b.prev_rotation_yaw) * var1;
        let var4 = math::cos(-var3 * 0.017453292 - 3.1415927);
        let var5 = math::sin(-var3 * 0.017453292 - 3.1415927);
        let var6 = -math::cos(-var2 * 0.017453292);
        let var7 = math::sin(-var2 * 0.017453292);
        Vec3D::create_vector((var5 * var6) as f64, var7 as f64, (var4 * var6) as f64)
    }
}

/// `rayTrace(distance, partialTick)` (client: the player's block target).
// src: EntityLiving.java:835-840
pub fn ray_trace(
    e: &Entity,
    world: &mut World,
    var1: f64,
    var3: f32,
) -> Option<MovingObjectPosition> {
    let mut var4 = get_position(e, var3);
    let var5 = get_look(e, var3);
    let var6 = var4.add_vector(
        var5.x_coord * var1,
        var5.y_coord * var1,
        var5.z_coord * var1,
    );
    world.ray_trace_blocks(&mut var4, &var6)
}

/// `EntityLiving.handleHealthUpdate(status)` (the multiplayer client's
/// entity status): 2 = hurt, 3 = dead; others go to `Entity`'s empty body.
// src: EntityLiving.java:850-866
pub fn handle_health_update(e: &mut Entity, world: &mut World, var1: i8) {
    if var1 == 2 {
        let l = lv_mut(e);
        l.leg_yaw = 1.5;
        let window = l.hearts_halves_life;
        l.max_hurt_time = 10;
        l.hurt_time = 10;
        l.attacked_at_yaw = 0.0;
        e.base.hearts_life = window;
        let name = dispatch::get_hurt_sound(e, world);
        hurt_or_death_sound(e, world, name);
        dispatch::attack_entity_from(e, world, None, 0);
    } else if var1 == 3 {
        let name = dispatch::get_death_sound(e, world);
        hurt_or_death_sound(e, world, name);
        lv_mut(e).health = 0;
        dispatch::on_death(e, world, None);
    }
    // else super.handleHealthUpdate: empty (Entity.java:1047-1048).
}
