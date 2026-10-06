//! `EntityLiving` (EntityLiving.java): fields and the level's method bodies.
//!
//! WP-0.5 transcribes the fields and the small overrides the base physics
//! reaches (`kill`, `fall`, `updateRidden`, `isEntityAlive`, …, see
//! [`super::dispatch`]). `onUpdate` and `onEntityUpdate` run their `super`
//! call and the first statements, then record the rest as not transcribed
//! (WP-2.2): `onLivingUpdate`, AI, `moveEntityWithHeading`, despawning,
//! breathing, hurt/death timers.

use mc_core::math::floor_double;

use super::{Entity, base, dispatch};
use crate::block;
use crate::world::World;

/// The fields `EntityLiving` declares (EntityLiving.java:6-58), in order.
/// Field initialisers that draw `Math.random()` are applied by
/// [`LivingData::new`] in Java's order.
#[derive(Clone, Debug, PartialEq)]
pub struct LivingData {
    /// `heartsHalvesLife`.
    pub hearts_halves_life: i32,
    /// `field_9365_p`.
    pub field_9365_p: f32,
    /// `field_9363_r`.
    pub field_9363_r: f32,
    /// `renderYawOffset`.
    pub render_yaw_offset: f32,
    /// `prevRenderYawOffset`.
    pub prev_render_yaw_offset: f32,
    /// `field_9362_u`.
    pub field_9362_u: f32,
    /// `field_9361_v`.
    pub field_9361_v: f32,
    /// `field_9360_w`.
    pub field_9360_w: f32,
    /// `field_9359_x`.
    pub field_9359_x: f32,
    /// `field_9358_y`.
    pub field_9358_y: bool,
    /// `texture`.
    pub texture: &'static str,
    /// `field_9355_A`.
    pub field_9355_a: bool,
    /// `unused180`.
    pub unused180: f32,
    /// `entityType`.
    pub entity_type: Option<&'static str>,
    /// `field_9349_D`.
    pub field_9349_d: f32,
    /// `scoreValue`.
    pub score_value: i32,
    /// `field_9345_F`.
    pub field_9345_f: f32,
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
    /// `unused_flag`.
    pub unused_flag: bool,
    /// `field_9326_T`.
    pub field_9326_t: i32,
    /// `field_9325_U`.
    pub field_9325_u: f32,
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
    /// `field_9348_ae`.
    pub field_9348_ae: f32,
    /// `field_9346_af`.
    pub field_9346_af: i32,
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
    pub fn new(world: &mut World, b: &mut base::EntityBase) -> LivingData {
        let field_9325_u =
            (base::math_random(world) * 0.8999999761581421 + 0.10000000149011612) as f32;
        let mut l = LivingData {
            hearts_halves_life: 20,
            field_9365_p: 0.0,
            field_9363_r: 0.0,
            render_yaw_offset: 0.0,
            prev_render_yaw_offset: 0.0,
            field_9362_u: 0.0,
            field_9361_v: 0.0,
            field_9360_w: 0.0,
            field_9359_x: 0.0,
            field_9358_y: true,
            texture: "/mob/char.png",
            field_9355_a: true,
            unused180: 0.0,
            entity_type: None,
            field_9349_d: 1.0,
            score_value: 0,
            field_9345_f: 0.0,
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
            field_9326_t: -1,
            field_9325_u,
            prev_leg_yaw: 0.0,
            leg_yaw: 0.0,
            leg_swing: 0.0,
            new_pos_rotation_increments: 0,
            new_pos_x: 0.0,
            new_pos_y: 0.0,
            new_pos_z: 0.0,
            new_rotation_yaw: 0.0,
            new_rotation_pitch: 0.0,
            field_9348_ae: 0.0,
            field_9346_af: 0,
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
        l.field_9363_r = (base::math_random(world) + 1.0) as f32 * 0.01;
        b.set_position(b.pos_x, b.pos_y, b.pos_z);
        l.field_9365_p = base::math_random(world) as f32 * 12398.0;
        b.rotation_yaw = (base::math_random(world) * 3.1415927410125732 * 2.0) as f32;
        b.step_height = 0.5;
        l
    }
}

fn living_mut(e: &mut Entity) -> &mut LivingData {
    e.kind
        .living_mut()
        .expect("EntityLiving method on a non-living entity")
}

/// `EntityLiving.onUpdate()`. Transcribed: the `super.onUpdate()` call.
/// Recorded as not transcribed: everything after it (WP-2.2).
// src: EntityLiving.java:203-290
pub fn on_update(e: &mut Entity, world: &mut World) {
    base::on_update(e, world);
    world.note_unimplemented("EntityLiving.onUpdate");
}

/// `EntityLiving.onEntityUpdate()`. Transcribed: `prevSwingProgress` and the
/// `super.onEntityUpdate()` call. Recorded as not transcribed: the rest
/// (living sound, suffocation, breathing, hurt/death timers; WP-2.2).
// src: EntityLiving.java:105-174
pub fn on_entity_update(e: &mut Entity, world: &mut World) {
    let l = living_mut(e);
    l.prev_swing_progress = l.swing_progress;
    base::on_entity_update(e, world);
    world.note_unimplemented("EntityLiving.onEntityUpdate");
}

/// `EntityLiving.kill()`: `attackEntityFrom(null, 4)`.
// src: EntityLiving.java:785-787
pub fn kill(e: &mut Entity, world: &mut World) {
    dispatch::attack_entity_from(e, world, None, 4);
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

/// `EntityLiving.updateRidden()`.
// src: EntityLiving.java:187-191
pub fn update_ridden(e: &mut Entity, world: &mut World) {
    base::update_ridden(e, world);
    let l = living_mut(e);
    l.field_9362_u = l.field_9361_v;
    l.field_9361_v = 0.0;
}
