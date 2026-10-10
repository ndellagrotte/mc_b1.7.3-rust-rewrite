//! `Entity` (Entity.java): the fields every entity has and the class's own
//! method bodies.
//!
//! Methods that only touch the entity's own fields and make no virtual call
//! are `impl EntityBase`. Methods that touch the world or call an overridable
//! method are free functions over `(e: &mut Entity, world: &mut World)`, and
//! make every virtual call through [`super::dispatch`] (module docs of
//! [`crate::entity`]). A method a subclass overrides is called through its
//! [`super::dispatch`] entry point; the function here is `Entity`'s body,
//! which the entry point runs for the kinds that inherit it.
//!
//! # Audit of Entity.java (WP-2.1)
//!
//! Every method of Entity.java, against this module, [`super::dispatch`],
//! [`super::nbt`] and the driver in `world/entities.rs`
//! (`updateEntityWithOptionalForce` and the chunk bookkeeping, World.java:1178-1355,
//! checked line by line: no gap). "WP-0.5" bodies were re-read line by line
//! in WP-2.1 against the Java and pinned by the `entity/item-*` and
//! `entity/falling-sand-on-cactus` fixtures where a transcribed kind reaches
//! them. No method is left a stub.
//!
//! | Entity.java | Method | Status | Port |
//! |---|---|---|---|
//! | 75-113 | constructor | transcribed (WP-0.5); `rand` seeded per PLAN_v4 §4.2 | [`EntityBase::new`] |
//! | 115 | `entityInit` (abstract) | per leaf class | the leaf constructors |
//! | 117-119 | `getDataWatcher` | field | `EntityBase::data_watcher` |
//! | 121-131 | `equals`, `hashCode` | identity is the entity id | [`Entity::id`] |
//! | 133-147 | `preparePlayerToSpawn` | transcribed (WP-2.1) | [`prepare_player_to_spawn`], dispatched |
//! | 149-151 | `setEntityDead` | transcribed | [`EntityBase::set_entity_dead`], dispatched |
//! | 153-161 | `setSize`, `setRotation` | transcribed | [`EntityBase::set_size`], [`EntityBase::set_rotation`] |
//! | 163-170 | `setPosition` | transcribed | [`EntityBase::set_position`] |
//! | 172-187 | `setAngles` | transcribed (WP-2.1) | [`EntityBase::set_angles`] |
//! | 189-191 | `onUpdate` | transcribed | [`on_update`], dispatched |
//! | 193-269 | `onEntityUpdate` | transcribed, verified (WP-2.1) | [`on_entity_update`], dispatched |
//! | 271-281 | `setOnFireFromLava`, `kill` | transcribed | [`set_on_fire_from_lava`], [`kill`] (dispatched) |
//! | 283-291 | `isOffsetPositionInLiquid` | transcribed | [`is_offset_position_in_liquid`] |
//! | 293-540 | `moveEntity` | transcribed, verified (WP-2.1) | [`move_entity`], dispatched |
//! | 542-556 | `canTriggerWalking`, `updateFallState` | transcribed | dispatched; [`update_fall_state`] |
//! | 558-574 | `getBoundingBox`, `dealFireDamage`, `fall` | transcribed | dispatched; [`deal_fire_damage`], [`fall`] |
//! | 576-586 | `isWet`, `isInWater`, `handleWaterMovement` | transcribed | [`is_wet`], [`EntityBase::is_in_water`] and [`handle_water_movement`] (both dispatched) |
//! | 588-609 | `isInsideOfMaterial`, `getEyeHeight`, `handleLavaMovement` | transcribed | [`is_inside_of_material`], dispatched, [`handle_lava_movement`] |
//! | 611-626 | `moveFlying` | transcribed | [`move_flying`] |
//! | 628-643 | `getEntityBrightness` | transcribed (WP-2.1) | [`get_entity_brightness`] |
//! | 645-647 | `setWorld` | no port: an entity belongs to the world that runs it | — |
//! | 649-676 | `setPositionAndRotation`, `setLocationAndAngles` | transcribed | [`EntityBase`] methods |
//! | 678-704 | `getDistanceToEntity`, `getDistanceSq`, `getDistance`, `getDistanceSqToEntity` | transcribed | [`EntityBase`] methods |
//! | 706-707 | `onCollideWithPlayer` | empty body | [`dispatch::on_collide_with_player`] (the Item, Arrow, Snowball and Egg arms: WP-2.4a) |
//! | 709-734 | `applyEntityCollision` | transcribed (WP-2.1) | [`apply_entity_collision`], dispatched |
//! | 736-744 | `addVelocity`, `setBeenAttacked` | transcribed | [`EntityBase::add_velocity`] (dispatched), [`EntityBase::set_been_attacked`] |
//! | 746-757 | `attackEntityFrom`, `canBeCollidedWith`, `canBePushed` | transcribed | [`attack_entity_from`], dispatched |
//! | 759-760 | `addToPlayerScore` | empty body | dispatched |
//! | 762-774 | `isInRangeToRenderVec3D`, `isInRangeToRenderDist` | transcribed (WP-2.1, render data) | [`is_in_range_to_render_vec3d`], [`is_in_range_to_render_dist`]; the overrides are mc-client's (not dispatched, WP-4.8) |
//! | 776-778 | `getEntityTexture` | base `null` | dispatched |
//! | 780-841 | `addEntityID`, `writeToNBT`, `readFromNBT`, `getEntityString`, the abstract NBT pair | transcribed (WP-1.9b) | [`super::nbt`] |
//! | 843-867 | `newDoubleNBTList`, `newFloatNBTList` | transcribed, verified (WP-2.1) | [`super::nbt`] |
//! | 869-871 | `getShadowSize` | transcribed (WP-2.1, render data) | [`get_shadow_size`]; the overrides are mc-client's (not dispatched, WP-4.8) |
//! | 873-886 | `dropItem`, `dropItemWithOffset`, `entityDropItem` | transcribed | [`drop_item`], [`drop_item_with_offset`], [`entity_drop_item`] |
//! | 888-890 | `isEntityAlive` | transcribed | dispatched |
//! | 892-906 | `isEntityInsideOpaqueBlock` | transcribed (WP-2.1) | [`is_entity_inside_opaque_block`], dispatched |
//! | 908-914 | `interact`, `getCollisionBox` | base `false` / `null` | dispatched |
//! | 916-968 | `updateRidden` | transcribed, verified (WP-2.1) | [`update_ridden`], dispatched |
//! | 970-980 | `updateRiderPosition`, `getYOffset`, `getMountedYOffset` | transcribed | [`update_rider_position`] (WP-2.1), [`EntityBase::get_y_offset`], [`get_mounted_y_offset`]; all dispatched |
//! | 982-1008 | `mountEntity` | transcribed: `null` (WP-0.5), general case (WP-2.1) | [`mount_entity`], [`mount_entity_in_world`], [`dismount`], [`dismount_from`] |
//! | 1010-1028 | `setPositionAndRotation2` | transcribed (WP-2.1) | [`set_position_and_rotation2`], dispatched |
//! | 1030-1036 | `getCollisionBorderSize`, `getLookVec` | transcribed (WP-2.1): 0.1, `null` | dispatched |
//! | 1038-1045 | `setInPortal`, `setVelocity` | empty body; transcribed | dispatched; [`EntityBase::set_velocity`] (dispatched) |
//! | 1047-1051 | `handleHealthUpdate`, `performHurtAnimation` | empty bodies | dispatched |
//! | 1053-1054 | `updateCloak` | empty body; `EntityPlayer`'s override builds an online cloak URL (out of scope, D2) | no port |
//! | 1056-1057 | `outfitWithItem` | empty body; only `EntityOtherPlayerMP` overrides it (multiplayer, D3) | no port |
//! | 1059-1083 | `isBurning`, `isRiding`, `isSneaking`, `getEntityFlag`, `setEntityFlag` | transcribed | [`EntityBase`] methods; `isSneaking` dispatched |
//! | 1085-1092 | `onStruckByLightning` | transcribed (WP-2.1) | [`on_struck_by_lightning`], dispatched |
//! | 1094-1095 | `onKillEntity` | empty body | dispatched |
//! | 1097-1170 | `pushOutOfBlocks` | transcribed | [`push_out_of_blocks`], dispatched |
//!
//! `EntityWeatherEffect` (EntityWeatherEffect.java) declares nothing but its
//! constructor: [`new_weather_effect`]. Client-only bodies above
//! (`setAngles`, `getEntityBrightness`, the render-distance and shadow data,
//! `setPositionAndRotation2`) are transcribed as data over the world for
//! `mc-client`; `getEntityBrightness` is also gameplay (EntityMob.java:12,
//! EntitySpider.java:20, :42, EntityZombie.java:13, EntitySkeleton.java:25).

use mc_core::Aabb;
use mc_core::java_random::JavaRandom;
use mc_core::math::{self, floor_double, floor_float};

use super::data_watcher::{DataWatcher, WatchValue};
use super::{Entity, dispatch, item};
use crate::block::{self, Material, dispatch as block_dispatch};
use crate::item::ItemStack;
use crate::world::World;

/// The fields `Entity` declares (Entity.java:7-73), in declaration order.
/// `worldObj` is implicit (an entity always belongs to the world that runs
/// it). The static `nextEntityID` is [`World::next_entity_id`].
#[derive(Clone, Debug)]
pub struct EntityBase {
    /// `entityId`.
    pub entity_id: i32,
    /// `renderDistanceWeight`.
    pub render_distance_weight: f64,
    /// `preventEntitySpawning`.
    pub prevent_entity_spawning: bool,
    /// `riddenByEntity`, as an entity id.
    pub ridden_by_entity: Option<i32>,
    /// `ridingEntity`, as an entity id.
    pub riding_entity: Option<i32>,
    /// `prevPosX`.
    pub prev_pos_x: f64,
    /// `prevPosY`.
    pub prev_pos_y: f64,
    /// `prevPosZ`.
    pub prev_pos_z: f64,
    /// `posX`.
    pub pos_x: f64,
    /// `posY`.
    pub pos_y: f64,
    /// `posZ`.
    pub pos_z: f64,
    /// `motionX`.
    pub motion_x: f64,
    /// `motionY`.
    pub motion_y: f64,
    /// `motionZ`.
    pub motion_z: f64,
    /// `rotationYaw`.
    pub rotation_yaw: f32,
    /// `rotationPitch`.
    pub rotation_pitch: f32,
    /// `prevRotationYaw`.
    pub prev_rotation_yaw: f32,
    /// `prevRotationPitch`.
    pub prev_rotation_pitch: f32,
    /// `boundingBox` (a `final` object mutated in place in Java).
    pub bounding_box: Aabb,
    /// `onGround`.
    pub on_ground: bool,
    /// `isCollidedHorizontally`.
    pub is_collided_horizontally: bool,
    /// `isCollidedVertically`.
    pub is_collided_vertically: bool,
    /// `isCollided`.
    pub is_collided: bool,
    /// `beenAttacked`.
    pub been_attacked: bool,
    /// `isInWeb`.
    pub is_in_web: bool,
    /// `field_9293_aM`: when false, any collision on an axis zeroes the whole
    /// move (Entity.java:348-377). Always true in vanilla; name kept, see
    /// `docs/mappings.md`.
    pub field_9293_a_m: bool,
    /// `isDead`.
    pub is_dead: bool,
    /// `yOffset`.
    pub y_offset: f32,
    /// `width`.
    pub width: f32,
    /// `height`.
    pub height: f32,
    /// `prevDistanceWalkedModified`.
    pub prev_distance_walked_modified: f32,
    /// `distanceWalkedModified`.
    pub distance_walked_modified: f32,
    /// `fallDistance` (protected).
    pub fall_distance: f32,
    /// `nextStepDistance` (private).
    pub next_step_distance: i32,
    /// `lastTickPosX`.
    pub last_tick_pos_x: f64,
    /// `lastTickPosY`.
    pub last_tick_pos_y: f64,
    /// `lastTickPosZ`.
    pub last_tick_pos_z: f64,
    /// `ySize`.
    pub y_size: f32,
    /// `stepHeight`.
    pub step_height: f32,
    /// `noClip`.
    pub no_clip: bool,
    /// `entityCollisionReduction`.
    pub entity_collision_reduction: f32,
    /// `rand` (protected). Seeded per the oracle's protocol (PLAN_v4 §4.2):
    /// see [`World::entity_rand_seed_for`].
    pub rand: JavaRandom,
    /// `ticksExisted`.
    pub ticks_existed: i32,
    /// `fireResistance`.
    pub fire_resistance: i32,
    /// `fire`.
    pub fire: i32,
    /// `maxAir` (protected).
    pub max_air: i32,
    /// `inWater` (protected).
    pub in_water: bool,
    /// `heartsLife`.
    pub hearts_life: i32,
    /// `air`.
    pub air: i32,
    /// `isFirstUpdate` (private).
    pub is_first_update: bool,
    /// `skinUrl` (online skins are out of scope, D2; kept for shape).
    pub skin_url: Option<String>,
    /// `cloakUrl` (as `skinUrl`).
    pub cloak_url: Option<String>,
    /// `isImmuneToFire` (protected).
    pub is_immune_to_fire: bool,
    /// `dataWatcher` (protected).
    pub data_watcher: DataWatcher,
    /// `entityBrightness`.
    pub entity_brightness: f32,
    /// `entityRiderPitchDelta` (private).
    pub entity_rider_pitch_delta: f64,
    /// `entityRiderYawDelta` (private).
    pub entity_rider_yaw_delta: f64,
    /// `addedToChunk`.
    pub added_to_chunk: bool,
    /// `chunkCoordX`.
    pub chunk_coord_x: i32,
    /// `chunkCoordY`.
    pub chunk_coord_y: i32,
    /// `chunkCoordZ`.
    pub chunk_coord_z: i32,
    /// `serverPosX` (multiplayer).
    pub server_pos_x: i32,
    /// `serverPosY` (multiplayer).
    pub server_pos_y: i32,
    /// `serverPosZ` (multiplayer).
    pub server_pos_z: i32,
    /// `ignoreFrustumCheck` (render).
    pub ignore_frustum_check: bool,
}

impl EntityBase {
    /// `Entity(World)`: takes the next entity id, sets the defaults, places the
    /// entity at the origin and registers data-watcher slot 0 (the flags).
    /// The subclass's `entityInit()` runs next, in the leaf constructor.
    ///
    /// `rand` is `new Random()` in vanilla; here it is seeded from the world's
    /// entity seed and the id (PLAN_v4 §4.2), which the oracle replicates.
    // src: Entity.java:75-113
    pub fn new(world: &mut World) -> EntityBase {
        let entity_id = world.next_entity_id;
        world.next_entity_id = world.next_entity_id.wrapping_add(1);
        let mut b = EntityBase {
            entity_id,
            render_distance_weight: 1.0,
            prevent_entity_spawning: false,
            ridden_by_entity: None,
            riding_entity: None,
            prev_pos_x: 0.0,
            prev_pos_y: 0.0,
            prev_pos_z: 0.0,
            pos_x: 0.0,
            pos_y: 0.0,
            pos_z: 0.0,
            motion_x: 0.0,
            motion_y: 0.0,
            motion_z: 0.0,
            rotation_yaw: 0.0,
            rotation_pitch: 0.0,
            prev_rotation_yaw: 0.0,
            prev_rotation_pitch: 0.0,
            bounding_box: Aabb::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            on_ground: false,
            is_collided_horizontally: false,
            is_collided_vertically: false,
            is_collided: false,
            been_attacked: false,
            is_in_web: false,
            field_9293_a_m: true,
            is_dead: false,
            y_offset: 0.0,
            width: 0.6,
            height: 1.8,
            prev_distance_walked_modified: 0.0,
            distance_walked_modified: 0.0,
            fall_distance: 0.0,
            next_step_distance: 1,
            last_tick_pos_x: 0.0,
            last_tick_pos_y: 0.0,
            last_tick_pos_z: 0.0,
            y_size: 0.0,
            step_height: 0.0,
            no_clip: false,
            entity_collision_reduction: 0.0,
            rand: JavaRandom::new(world.entity_rand_seed_for(entity_id)),
            ticks_existed: 0,
            fire_resistance: 1,
            fire: 0,
            max_air: 300,
            in_water: false,
            hearts_life: 0,
            air: 300,
            is_first_update: true,
            skin_url: None,
            cloak_url: None,
            is_immune_to_fire: false,
            data_watcher: DataWatcher::default(),
            entity_brightness: 0.0,
            entity_rider_pitch_delta: 0.0,
            entity_rider_yaw_delta: 0.0,
            added_to_chunk: false,
            chunk_coord_x: 0,
            chunk_coord_y: 0,
            chunk_coord_z: 0,
            server_pos_x: 0,
            server_pos_y: 0,
            server_pos_z: 0,
            ignore_frustum_check: false,
        };
        b.set_position(0.0, 0.0, 0.0);
        b.data_watcher.add_object(0, WatchValue::Byte(0));
        b
    }

    /// `Entity.setEntityDead()` (the base body; overrides go through
    /// [`dispatch::set_entity_dead`]).
    // src: Entity.java:149-151
    pub fn set_entity_dead(&mut self) {
        self.is_dead = true;
    }

    /// `setSize(width, height)` (`EntityLiving.setSize` only calls super).
    // src: Entity.java:153-156, EntityLiving.java:292-294
    pub fn set_size(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
    }

    /// `setRotation(yaw, pitch)`: Java float `%` is `fmod`, as Rust's `%`.
    // src: Entity.java:158-161
    pub fn set_rotation(&mut self, yaw: f32, pitch: f32) {
        self.rotation_yaw = yaw % 360.0;
        self.rotation_pitch = pitch % 360.0;
    }

    /// `setAngles(yaw, pitch)`: the mouse-look turn (EntityRenderer.java:354
    /// passes the mouse deltas), pitch clamped to ±90, `prevRotation*`
    /// shifted by the same amount so the turn is not interpolated.
    // src: Entity.java:172-187
    #[allow(clippy::manual_clamp)] // Java's two ifs, kept
    pub fn set_angles(&mut self, var1: f32, var2: f32) {
        let var3 = self.rotation_pitch;
        let var4 = self.rotation_yaw;
        self.rotation_yaw = (self.rotation_yaw as f64 + var1 as f64 * 0.15) as f32;
        self.rotation_pitch = (self.rotation_pitch as f64 - var2 as f64 * 0.15) as f32;
        if self.rotation_pitch < -90.0 {
            self.rotation_pitch = -90.0;
        }
        if self.rotation_pitch > 90.0 {
            self.rotation_pitch = 90.0;
        }
        self.prev_rotation_pitch += self.rotation_pitch - var3;
        self.prev_rotation_yaw += self.rotation_yaw - var4;
    }

    /// `setPosition(x, y, z)`: moves the bounding box with it.
    // src: Entity.java:163-170
    pub fn set_position(&mut self, x: f64, y: f64, z: f64) {
        self.pos_x = x;
        self.pos_y = y;
        self.pos_z = z;
        let var7 = self.width / 2.0;
        let var8 = self.height;
        self.bounding_box.set_bounds(
            x - var7 as f64,
            y - self.y_offset as f64 + self.y_size as f64,
            z - var7 as f64,
            x + var7 as f64,
            y - self.y_offset as f64 + self.y_size as f64 + var8 as f64,
            z + var7 as f64,
        );
    }

    /// `setPositionAndRotation(x, y, z, yaw, pitch)`.
    // src: Entity.java:649-667
    pub fn set_position_and_rotation(&mut self, x: f64, y: f64, z: f64, yaw: f32, pitch: f32) {
        self.pos_x = x;
        self.prev_pos_x = x;
        self.pos_y = y;
        self.prev_pos_y = y;
        self.pos_z = z;
        self.prev_pos_z = z;
        self.rotation_yaw = yaw;
        self.prev_rotation_yaw = yaw;
        self.rotation_pitch = pitch;
        self.prev_rotation_pitch = pitch;
        self.y_size = 0.0;
        let var9 = (self.prev_rotation_yaw - yaw) as f64;
        if var9 < -180.0 {
            self.prev_rotation_yaw += 360.0;
        }
        if var9 >= 180.0 {
            self.prev_rotation_yaw -= 360.0;
        }
        self.set_position(self.pos_x, self.pos_y, self.pos_z);
        self.set_rotation(yaw, pitch);
    }

    /// `setLocationAndAngles(x, y, z, yaw, pitch)`: `y` is the feet, `posY`
    /// adds `yOffset`.
    // src: Entity.java:669-676
    pub fn set_location_and_angles(&mut self, x: f64, y: f64, z: f64, yaw: f32, pitch: f32) {
        self.pos_x = x;
        self.prev_pos_x = x;
        self.last_tick_pos_x = x;
        self.pos_y = y + self.y_offset as f64;
        self.prev_pos_y = self.pos_y;
        self.last_tick_pos_y = self.pos_y;
        self.pos_z = z;
        self.prev_pos_z = z;
        self.last_tick_pos_z = z;
        self.rotation_yaw = yaw;
        self.rotation_pitch = pitch;
        self.set_position(self.pos_x, self.pos_y, self.pos_z);
    }

    /// `getDistanceToEntity(other)`.
    // src: Entity.java:678-683
    pub fn get_distance_to_entity(&self, other: &EntityBase) -> f32 {
        let var2 = (self.pos_x - other.pos_x) as f32;
        let var3 = (self.pos_y - other.pos_y) as f32;
        let var4 = (self.pos_z - other.pos_z) as f32;
        math::sqrt_float(var2 * var2 + var3 * var3 + var4 * var4)
    }

    /// `getDistanceSq(x, y, z)`.
    // src: Entity.java:685-690
    pub fn get_distance_sq(&self, x: f64, y: f64, z: f64) -> f64 {
        let var7 = self.pos_x - x;
        let var9 = self.pos_y - y;
        let var11 = self.pos_z - z;
        var7 * var7 + var9 * var9 + var11 * var11
    }

    /// `getDistance(x, y, z)`: `MathHelper.sqrt_double`, a float.
    // src: Entity.java:692-697
    pub fn get_distance(&self, x: f64, y: f64, z: f64) -> f64 {
        let var7 = self.pos_x - x;
        let var9 = self.pos_y - y;
        let var11 = self.pos_z - z;
        math::sqrt_double(var7 * var7 + var9 * var9 + var11 * var11) as f64
    }

    /// `getDistanceSqToEntity(other)`.
    // src: Entity.java:699-704
    pub fn get_distance_sq_to_entity(&self, other: &EntityBase) -> f64 {
        let var2 = self.pos_x - other.pos_x;
        let var4 = self.pos_y - other.pos_y;
        let var6 = self.pos_z - other.pos_z;
        var2 * var2 + var4 * var4 + var6 * var6
    }

    /// `addVelocity(dx, dy, dz)`.
    // src: Entity.java:736-740
    pub fn add_velocity(&mut self, x: f64, y: f64, z: f64) {
        self.motion_x += x;
        self.motion_y += y;
        self.motion_z += z;
    }

    /// `setBeenAttacked()`.
    // src: Entity.java:742-744
    pub fn set_been_attacked(&mut self) {
        self.been_attacked = true;
    }

    /// `setVelocity(x, y, z)`.
    // src: Entity.java:1041-1045
    pub fn set_velocity(&mut self, x: f64, y: f64, z: f64) {
        self.motion_x = x;
        self.motion_y = y;
        self.motion_z = z;
    }

    /// `getYOffset()` (Entity's body; `EntityPlayer` overrides it).
    // src: Entity.java:974-976
    pub fn get_y_offset(&self) -> f64 {
        self.y_offset as f64
    }

    /// `isInWater()`.
    // src: Entity.java:580-582
    pub fn is_in_water(&self) -> bool {
        self.in_water
    }

    /// `getEntityFlag(bit)`: bit 0 burning, 1 sneaking, 2 riding.
    // src: Entity.java:1071-1073
    pub fn get_entity_flag(&self, bit: i32) -> bool {
        (self.data_watcher.get_watchable_object_byte(0) as i32 & 1 << bit) != 0
    }

    /// `setEntityFlag(bit, value)`.
    // src: Entity.java:1075-1083
    pub fn set_entity_flag(&mut self, bit: i32, value: bool) {
        let var3 = self.data_watcher.get_watchable_object_byte(0);
        if value {
            self.data_watcher
                .update_object(0, WatchValue::Byte((var3 as i32 | 1 << bit) as i8));
        } else {
            self.data_watcher
                .update_object(0, WatchValue::Byte((var3 as i32 & !(1 << bit)) as i8));
        }
    }

    /// `isBurning()`.
    // src: Entity.java:1059-1061
    pub fn is_burning(&self) -> bool {
        self.fire > 0 || self.get_entity_flag(0)
    }

    /// `isRiding()`.
    // src: Entity.java:1063-1065
    pub fn is_riding(&self) -> bool {
        self.riding_entity.is_some() || self.get_entity_flag(2)
    }
}

/// `Entity.onUpdate()`: `this.onEntityUpdate()` (virtual).
// src: Entity.java:189-191
pub fn on_update(e: &mut Entity, world: &mut World) {
    dispatch::on_entity_update(e, world);
}

/// `Entity.onEntityUpdate()`: bookkeeping, water/lava/fire, void death,
/// entity flags.
// src: Entity.java:193-269
pub fn on_entity_update(e: &mut Entity, world: &mut World) {
    if let Some(r) = e.base.riding_entity
        && world.is_entity_dead_or_gone(r)
    {
        e.base.riding_entity = None;
    }
    e.base.ticks_existed += 1;
    e.base.prev_distance_walked_modified = e.base.distance_walked_modified;
    e.base.prev_pos_x = e.base.pos_x;
    e.base.prev_pos_y = e.base.pos_y;
    e.base.prev_pos_z = e.base.pos_z;
    e.base.prev_rotation_pitch = e.base.rotation_pitch;
    e.base.prev_rotation_yaw = e.base.rotation_yaw;
    if dispatch::handle_water_movement(e, world) {
        if !e.base.in_water && !e.base.is_first_update {
            let b = &mut e.base;
            let mut var1 = math::sqrt_double(
                b.motion_x * b.motion_x * 0.20000000298023224
                    + b.motion_y * b.motion_y
                    + b.motion_z * b.motion_z * 0.20000000298023224,
            ) * 0.2;
            if var1 > 1.0 {
                var1 = 1.0;
            }
            let pitch = 1.0 + (b.rand.next_float() - b.rand.next_float()) * 0.4;
            world.play_sound_at_entity(&e.base, "random.splash", var1, pitch);
            let b = &mut e.base;
            let var2 = floor_double(b.bounding_box.min_y) as f32;
            let mut var3 = 0;
            while (var3 as f32) < 1.0 + b.width * 20.0 {
                let var4 = (b.rand.next_float() * 2.0 - 1.0) * b.width;
                let var5 = (b.rand.next_float() * 2.0 - 1.0) * b.width;
                let vy = b.motion_y - (b.rand.next_float() * 0.2) as f64;
                world.spawn_particle(
                    "bubble",
                    b.pos_x + var4 as f64,
                    (var2 + 1.0) as f64,
                    b.pos_z + var5 as f64,
                    b.motion_x,
                    vy,
                    b.motion_z,
                );
                var3 += 1;
            }
            var3 = 0;
            while (var3 as f32) < 1.0 + b.width * 20.0 {
                let var4 = (b.rand.next_float() * 2.0 - 1.0) * b.width;
                let var5 = (b.rand.next_float() * 2.0 - 1.0) * b.width;
                world.spawn_particle(
                    "splash",
                    b.pos_x + var4 as f64,
                    (var2 + 1.0) as f64,
                    b.pos_z + var5 as f64,
                    b.motion_x,
                    b.motion_y,
                    b.motion_z,
                );
                var3 += 1;
            }
        }
        e.base.fall_distance = 0.0;
        e.base.in_water = true;
        e.base.fire = 0;
    } else {
        e.base.in_water = false;
    }

    if world.multiplayer_world {
        e.base.fire = 0;
    } else if e.base.fire > 0 {
        if e.base.is_immune_to_fire {
            e.base.fire -= 4;
            if e.base.fire < 0 {
                e.base.fire = 0;
            }
        } else {
            if e.base.fire % 20 == 0 {
                dispatch::attack_entity_from(e, world, None, 1);
            }
            e.base.fire -= 1;
        }
    }

    if handle_lava_movement(e, world) {
        set_on_fire_from_lava(e, world);
    }

    if e.base.pos_y < -64.0 {
        dispatch::kill(e, world);
    }

    if !world.multiplayer_world {
        let burning = e.base.fire > 0;
        e.base.set_entity_flag(0, burning);
        let riding = e.base.riding_entity.is_some();
        e.base.set_entity_flag(2, riding);
    }
    e.base.is_first_update = false;
}

/// `setOnFireFromLava()` (never overridden).
// src: Entity.java:271-277
pub fn set_on_fire_from_lava(e: &mut Entity, world: &mut World) {
    if !e.base.is_immune_to_fire {
        dispatch::attack_entity_from(e, world, None, 4);
        e.base.fire = 600;
    }
}

/// `Entity.kill()`: `setEntityDead()` (virtual).
// src: Entity.java:279-281
pub fn kill(e: &mut Entity, world: &mut World) {
    dispatch::set_entity_dead(e, world);
}

/// `isOffsetPositionInLiquid(dx, dy, dz)`.
// src: Entity.java:283-291
pub fn is_offset_position_in_liquid(e: &Entity, world: &mut World, x: f64, y: f64, z: f64) -> bool {
    let var7 = e.base.bounding_box.get_offset_bounding_box(x, y, z);
    let var8 = world.get_colliding_bounding_boxes(e, &var7);
    if !var8.is_empty() {
        false
    } else {
        !world.get_is_any_liquid(&var7)
    }
}

/// `Entity.moveEntity(dx, dy, dz)`: collision-resolved movement, step-up,
/// walking sounds, block collision callbacks and fire contact. The physics
/// core of every entity.
// src: Entity.java:293-540
pub fn move_entity(e: &mut Entity, world: &mut World, mut var1: f64, mut var3: f64, mut var5: f64) {
    if e.base.no_clip {
        let b = &mut e.base;
        b.bounding_box.offset(var1, var3, var5);
        b.pos_x = (b.bounding_box.min_x + b.bounding_box.max_x) / 2.0;
        b.pos_y = b.bounding_box.min_y + b.y_offset as f64 - b.y_size as f64;
        b.pos_z = (b.bounding_box.min_z + b.bounding_box.max_z) / 2.0;
        return;
    }
    e.base.y_size *= 0.4;
    let var7 = e.base.pos_x;
    let var9 = e.base.pos_z;
    if e.base.is_in_web {
        e.base.is_in_web = false;
        var1 *= 0.25;
        var3 *= 0.05000000074505806;
        var5 *= 0.25;
        e.base.motion_x = 0.0;
        e.base.motion_y = 0.0;
        e.base.motion_z = 0.0;
    }

    let mut var11 = var1;
    let var13 = var3;
    let mut var15 = var5;
    let var17 = e.base.bounding_box.copy();
    let var18 = e.base.on_ground && dispatch::is_sneaking(e, world);
    if var18 {
        let var19 = 0.05;
        while var1 != 0.0
            && world
                .get_colliding_bounding_boxes(
                    e,
                    &e.base.bounding_box.get_offset_bounding_box(var1, -1.0, 0.0),
                )
                .is_empty()
        {
            if var1 < var19 && var1 >= -var19 {
                var1 = 0.0;
            } else if var1 > 0.0 {
                var1 -= var19;
            } else {
                var1 += var19;
            }
            var11 = var1;
        }
        while var5 != 0.0
            && world
                .get_colliding_bounding_boxes(
                    e,
                    &e.base.bounding_box.get_offset_bounding_box(0.0, -1.0, var5),
                )
                .is_empty()
        {
            if var5 < var19 && var5 >= -var19 {
                var5 = 0.0;
            } else if var5 > 0.0 {
                var5 -= var19;
            } else {
                var5 += var19;
            }
            var15 = var5;
        }
    }

    let mut var35 =
        world.get_colliding_bounding_boxes(e, &e.base.bounding_box.add_coord(var1, var3, var5));
    for bb in &var35 {
        var3 = bb.calculate_y_offset(&e.base.bounding_box, var3);
    }
    e.base.bounding_box.offset(0.0, var3, 0.0);
    if !e.base.field_9293_a_m && var13 != var3 {
        var5 = 0.0;
        var3 = 0.0;
        var1 = 0.0;
    }

    let var36 = e.base.on_ground || var13 != var3 && var13 < 0.0;

    for bb in &var35 {
        var1 = bb.calculate_x_offset(&e.base.bounding_box, var1);
    }
    e.base.bounding_box.offset(var1, 0.0, 0.0);
    if !e.base.field_9293_a_m && var11 != var1 {
        var5 = 0.0;
        var3 = 0.0;
        var1 = 0.0;
    }

    for bb in &var35 {
        var5 = bb.calculate_z_offset(&e.base.bounding_box, var5);
    }
    e.base.bounding_box.offset(0.0, 0.0, var5);
    if !e.base.field_9293_a_m && var15 != var5 {
        var5 = 0.0;
        var3 = 0.0;
        var1 = 0.0;
    }

    if e.base.step_height > 0.0
        && var36
        && (var18 || e.base.y_size < 0.05)
        && (var11 != var1 || var15 != var5)
    {
        let var37 = var1;
        let var23 = var3;
        let var25 = var5;
        var1 = var11;
        var3 = e.base.step_height as f64;
        var5 = var15;
        let var27 = e.base.bounding_box.copy();
        e.base.bounding_box.set_bb(&var17);
        var35 = world
            .get_colliding_bounding_boxes(e, &e.base.bounding_box.add_coord(var11, var3, var15));

        for bb in &var35 {
            var3 = bb.calculate_y_offset(&e.base.bounding_box, var3);
        }
        e.base.bounding_box.offset(0.0, var3, 0.0);
        if !e.base.field_9293_a_m && var13 != var3 {
            var5 = 0.0;
            var3 = 0.0;
            var1 = 0.0;
        }

        for bb in &var35 {
            var1 = bb.calculate_x_offset(&e.base.bounding_box, var1);
        }
        e.base.bounding_box.offset(var1, 0.0, 0.0);
        if !e.base.field_9293_a_m && var11 != var1 {
            var5 = 0.0;
            var3 = 0.0;
            var1 = 0.0;
        }

        for bb in &var35 {
            var5 = bb.calculate_z_offset(&e.base.bounding_box, var5);
        }
        e.base.bounding_box.offset(0.0, 0.0, var5);
        if !e.base.field_9293_a_m && var15 != var5 {
            var5 = 0.0;
            var3 = 0.0;
            var1 = 0.0;
        }

        if !e.base.field_9293_a_m && var13 != var3 {
            var5 = 0.0;
            var3 = 0.0;
            var1 = 0.0;
        } else {
            var3 = -e.base.step_height as f64;
            for bb in &var35 {
                var3 = bb.calculate_y_offset(&e.base.bounding_box, var3);
            }
            e.base.bounding_box.offset(0.0, var3, 0.0);
        }

        if var37 * var37 + var25 * var25 >= var1 * var1 + var5 * var5 {
            var1 = var37;
            var3 = var23;
            var5 = var25;
            e.base.bounding_box.set_bb(&var27);
        } else {
            // (double)(int)minY: Java truncation toward zero, as `as i32`.
            let var41 = e.base.bounding_box.min_y - (e.base.bounding_box.min_y as i32) as f64;
            if var41 > 0.0 {
                e.base.y_size = (e.base.y_size as f64 + var41 + 0.01) as f32;
            }
        }
    }

    {
        let b = &mut e.base;
        b.pos_x = (b.bounding_box.min_x + b.bounding_box.max_x) / 2.0;
        b.pos_y = b.bounding_box.min_y + b.y_offset as f64 - b.y_size as f64;
        b.pos_z = (b.bounding_box.min_z + b.bounding_box.max_z) / 2.0;
        b.is_collided_horizontally = var11 != var1 || var15 != var5;
        b.is_collided_vertically = var13 != var3;
        b.on_ground = var13 != var3 && var13 < 0.0;
        b.is_collided = b.is_collided_horizontally || b.is_collided_vertically;
    }
    let on_ground = e.base.on_ground;
    update_fall_state(e, world, var3, on_ground);
    if var11 != var1 {
        e.base.motion_x = 0.0;
    }
    if var13 != var3 {
        e.base.motion_y = 0.0;
    }
    if var15 != var5 {
        e.base.motion_z = 0.0;
    }

    let var37 = e.base.pos_x - var7;
    let var23 = e.base.pos_z - var9;
    if dispatch::can_trigger_walking(e) && !var18 && e.base.riding_entity.is_none() {
        e.base.distance_walked_modified = (e.base.distance_walked_modified as f64
            + math::sqrt_double(var37 * var37 + var23 * var23) as f64 * 0.6)
            as f32;
        let var38 = floor_double(e.base.pos_x);
        let var26 = floor_double(e.base.pos_y - 0.20000000298023224 - e.base.y_offset as f64);
        let var39 = floor_double(e.base.pos_z);
        let mut var28 = world.get_block_id(var38, var26, var39);
        if world.get_block_id(var38, var26 - 1, var39) == BLOCK_FENCE {
            var28 = world.get_block_id(var38, var26 - 1, var39);
        }
        if e.base.distance_walked_modified > e.base.next_step_distance as f32 && var28 > 0 {
            e.base.next_step_distance += 1;
            let var28_def = block::block(var28).expect("walked-on block id has a Block");
            let mut var29 = var28_def.step_sound;
            if world.get_block_id(var38, var26 + 1, var39) == BLOCK_SNOW {
                var29 = block::block(BLOCK_SNOW).expect("snow").step_sound;
                world.play_sound_at_entity(
                    &e.base,
                    &var29.step_sound_dir2(),
                    var29.get_volume() * 0.15,
                    var29.get_pitch(),
                );
            } else if !var28_def.block_material.get_is_liquid() {
                world.play_sound_at_entity(
                    &e.base,
                    &var29.step_sound_dir2(),
                    var29.get_volume() * 0.15,
                    var29.get_pitch(),
                );
            }
            block_dispatch::on_entity_walking(world, var28, var38, var26, var39, e);
        }
    }

    let var38 = floor_double(e.base.bounding_box.min_x + 0.001);
    let var26 = floor_double(e.base.bounding_box.min_y + 0.001);
    let var39 = floor_double(e.base.bounding_box.min_z + 0.001);
    let var28 = floor_double(e.base.bounding_box.max_x - 0.001);
    let var40 = floor_double(e.base.bounding_box.max_y - 0.001);
    let var30 = floor_double(e.base.bounding_box.max_z - 0.001);
    if world.check_chunks_exist(var38, var26, var39, var28, var40, var30) {
        for var31 in var38..=var28 {
            for var32 in var26..=var40 {
                for var33 in var39..=var30 {
                    let var34 = world.get_block_id(var31, var32, var33);
                    if var34 > 0 {
                        block_dispatch::on_entity_collided_with_block(
                            world, var34, var31, var32, var33, e,
                        );
                    }
                }
            }
        }
    }

    let var42 = is_wet(e, world);
    if world.is_bounding_box_burning(&e.base.bounding_box.contract(0.001, 0.001, 0.001)) {
        dispatch::deal_fire_damage(e, world, 1);
        if !var42 {
            e.base.fire += 1;
            if e.base.fire == 0 {
                e.base.fire = 300;
            }
        }
    } else if e.base.fire <= 0 {
        e.base.fire = -e.base.fire_resistance;
    }

    if var42 && e.base.fire > 0 {
        let pitch = 1.6 + (e.base.rand.next_float() - e.base.rand.next_float()) * 0.4;
        world.play_sound_at_entity(&e.base, "random.fizz", 0.7, pitch);
        e.base.fire = -e.base.fire_resistance;
    }
}

/// `Block.fence.blockID`.
const BLOCK_FENCE: i32 = 85;
/// `Block.snow.blockID`.
const BLOCK_SNOW: i32 = 78;

/// `updateFallState(dy, onGround)` (never overridden).
// src: Entity.java:546-556
pub fn update_fall_state(e: &mut Entity, world: &mut World, var1: f64, var3: bool) {
    if var3 {
        if e.base.fall_distance > 0.0 {
            let d = e.base.fall_distance;
            dispatch::fall(e, world, d);
            e.base.fall_distance = 0.0;
        }
    } else if var1 < 0.0 {
        e.base.fall_distance = (e.base.fall_distance as f64 - var1) as f32;
    }
}

/// `Entity.dealFireDamage(amount)`.
// src: Entity.java:562-567
pub fn deal_fire_damage(e: &mut Entity, world: &mut World, amount: i32) {
    if !e.base.is_immune_to_fire {
        dispatch::attack_entity_from(e, world, None, amount);
    }
}

/// `Entity.fall(distance)`: passes the fall on to the rider.
// src: Entity.java:569-574
pub fn fall(e: &mut Entity, world: &mut World, distance: f32) {
    if let Some(rider) = e.base.ridden_by_entity {
        world.with_entity_mut(rider, |w, r| dispatch::fall(r, w, distance));
    }
}

/// `Entity.dropItem(id, count)` → `dropItemWithOffset(id, count, 0.0F)`.
// src: Entity.java:873-875
pub fn drop_item(e: &Entity, world: &mut World, id: i32, count: i32) {
    drop_item_with_offset(e, world, id, count, 0.0);
}

/// `Entity.dropItemWithOffset(id, count, offset)` →
/// `entityDropItem(new ItemStack(id, count, 0), offset)`.
// src: Entity.java:877-879
pub fn drop_item_with_offset(e: &Entity, world: &mut World, id: i32, count: i32, offset: f32) {
    entity_drop_item(e, world, ItemStack::new(id, count, 0), offset);
}

/// `Entity.entityDropItem(stack, offset)`: an `EntityItem` at the entity's
/// position raised by `offset`, pick-up delay 10, joined to the world. (Java
/// returns the `EntityItem`; it is moved into the world here, so nothing is
/// returned.)
// src: Entity.java:881-886
pub fn entity_drop_item(e: &Entity, world: &mut World, stack: ItemStack, offset: f32) {
    let mut var3 = item::new_entity_item(
        world,
        e.base.pos_x,
        e.base.pos_y + offset as f64,
        e.base.pos_z,
        stack,
    );
    if let Some(d) = var3.kind.item_mut() {
        d.delay_before_can_pickup = 10;
    }
    world.spawn_entity_in_world(var3);
}

/// `isWet()`: in water or rained on.
// src: Entity.java:576-578
pub fn is_wet(e: &Entity, world: &mut World) -> bool {
    e.base.in_water
        || world.can_block_be_rained_on(
            floor_double(e.base.pos_x),
            floor_double(e.base.pos_y),
            floor_double(e.base.pos_z),
        )
}

/// `Entity.handleWaterMovement()`.
// src: Entity.java:584-586
pub fn handle_water_movement(e: &mut Entity, world: &mut World) -> bool {
    let bb = e
        .base
        .bounding_box
        .expand(0.0, -0.4000000059604645, 0.0)
        .contract(0.001, 0.001, 0.001);
    world.handle_material_acceleration(&bb, Material::Water, e)
}

/// `isInsideOfMaterial(material)`: is the eye inside it.
// src: Entity.java:588-601
pub fn is_inside_of_material(e: &Entity, world: &mut World, material: Material) -> bool {
    let var2 = e.base.pos_y + dispatch::get_eye_height(e, world) as f64;
    let var4 = floor_double(e.base.pos_x);
    let var5 = floor_float(floor_double(var2) as f32);
    let var6 = floor_double(e.base.pos_z);
    let var7 = world.get_block_id(var4, var5, var6);
    if var7 != 0 && block::material_of(var7) == material {
        let var8 =
            block::fluid::get_percent_air(world.get_block_metadata(var4, var5, var6)) - 0.11111111;
        let var9 = (var5 + 1) as f32 - var8;
        var2 < var9 as f64
    } else {
        false
    }
}

/// `handleLavaMovement()` (never overridden).
// src: Entity.java:607-609
pub fn handle_lava_movement(e: &Entity, world: &mut World) -> bool {
    let bb = e.base.bounding_box.expand(
        -0.10000000149011612,
        -0.4000000059604645,
        -0.10000000149011612,
    );
    world.is_material_in_bb(&bb, Material::Lava)
}

/// `moveFlying(strafe, forward, speed)`: accelerates along the yaw.
// src: Entity.java:611-626
pub fn move_flying(b: &mut EntityBase, mut var1: f32, mut var2: f32, var3: f32) {
    let mut var4 = math::sqrt_float(var1 * var1 + var2 * var2);
    if var4 >= 0.01 {
        if var4 < 1.0 {
            var4 = 1.0;
        }
        var4 = var3 / var4;
        var1 *= var4;
        var2 *= var4;
        let var5 = math::sin(b.rotation_yaw * 3.1415927 / 180.0);
        let var6 = math::cos(b.rotation_yaw * 3.1415927 / 180.0);
        b.motion_x += (var1 * var6 - var2 * var5) as f64;
        b.motion_z += (var2 * var6 + var1 * var5) as f64;
    }
}

/// `Entity.attackEntityFrom(attacker, damage)`: marks it attacked, no damage.
// src: Entity.java:746-749
pub fn attack_entity_from(
    e: &mut Entity,
    _world: &mut World,
    _attacker: Option<&Entity>,
    _damage: i32,
) -> bool {
    e.base.set_been_attacked();
    false
}

/// `Entity.updateRidden()`: the rider's tick while mounted.
// src: Entity.java:916-968
pub fn update_ridden(e: &mut Entity, world: &mut World) {
    let Some(mount_id) = e.base.riding_entity else {
        return;
    };
    if world.is_entity_dead_or_gone(mount_id) {
        e.base.riding_entity = None;
        return;
    }
    e.base.motion_x = 0.0;
    e.base.motion_y = 0.0;
    e.base.motion_z = 0.0;
    dispatch::on_update(e, world);
    if let Some(mount_id) = e.base.riding_entity {
        // `this.ridingEntity.updateRiderPosition()` (virtual): the mount
        // positions its `riddenByEntity`, the rider in hand.
        let Some(m) = world.entity(mount_id) else {
            world.note_unimplemented("Entity.updateRidden: mount out of its slot");
            return;
        };
        dispatch::update_rider_position(m, e, world);
        let (dyaw, dpitch) = (
            (m.base.rotation_yaw - m.base.prev_rotation_yaw) as f64,
            (m.base.rotation_pitch - m.base.prev_rotation_pitch) as f64,
        );
        let b = &mut e.base;
        b.entity_rider_yaw_delta += dyaw;
        b.entity_rider_pitch_delta += dpitch;
        while b.entity_rider_yaw_delta >= 180.0 {
            b.entity_rider_yaw_delta -= 360.0;
        }
        while b.entity_rider_yaw_delta < -180.0 {
            b.entity_rider_yaw_delta += 360.0;
        }
        while b.entity_rider_pitch_delta >= 180.0 {
            b.entity_rider_pitch_delta -= 360.0;
        }
        while b.entity_rider_pitch_delta < -180.0 {
            b.entity_rider_pitch_delta += 360.0;
        }
        let mut var1 = b.entity_rider_yaw_delta * 0.5;
        let mut var3 = b.entity_rider_pitch_delta * 0.5;
        let var5: f32 = 10.0;
        if var1 > var5 as f64 {
            var1 = var5 as f64;
        }
        if var1 < -var5 as f64 {
            var1 = -var5 as f64;
        }
        if var3 > var5 as f64 {
            var3 = var5 as f64;
        }
        if var3 < -var5 as f64 {
            var3 = -var5 as f64;
        }
        b.entity_rider_yaw_delta -= var1;
        b.entity_rider_pitch_delta -= var3;
        b.rotation_yaw = (b.rotation_yaw as f64 + var1) as f32;
        b.rotation_pitch = (b.rotation_pitch as f64 + var3) as f32;
    }
}

/// `Entity.updateRiderPosition()`: puts the rider on top of the mount.
/// Java positions `mount.riddenByEntity`; the caller passes that rider, which
/// is usually out of its slot (the rider in hand in [`update_ridden`]).
// src: Entity.java:970-972
pub fn update_rider_position(mount: &Entity, rider: &mut Entity, world: &World) {
    let y = mount.base.pos_y
        + dispatch::get_mounted_y_offset(mount, world)
        + dispatch::get_y_offset(rider, world);
    rider
        .base
        .set_position(mount.base.pos_x, y, mount.base.pos_z);
}

/// `getMountedYOffset()` (Entity's body).
// src: Entity.java:978-980
pub fn get_mounted_y_offset(e: &Entity) -> f64 {
    e.base.height as f64 * 0.75
}

/// `mountEntity(entity)`: with `None`, [`dismount`]; with the entity
/// already ridden by `e`, dismount from it onto its top; otherwise unlink
/// `e` from its old mount and `target` from its old rider, then link the two.
///
/// `target` is in hand (out of its slot: vanilla's callers are the mount's
/// own `interact`, `attackEntityFrom`, `applyEntityCollision` or
/// `onUpdate` (EntityBoat.java:71, :341, EntityMinecart.java:81, :650,
/// :770, EntityPig.java:40), and the spider jockey of
/// SpawnerAnimals.java:166, whose two entities have not joined yet). The other entities the Java touches through fields (`e`'s old
/// mount, `target`'s old rider) are found in the world by id; one that is
/// out of its slot further up the stack is recorded and left as is.
// src: Entity.java:982-1008
pub fn mount_entity(e: &mut Entity, world: &mut World, target: Option<&mut Entity>) {
    let Some(var1) = target else {
        // src: Entity.java:983-991
        dismount(e, world);
        return;
    };
    e.base.entity_rider_pitch_delta = 0.0;
    e.base.entity_rider_yaw_delta = 0.0;
    let id = e.base.entity_id;
    if e.base.riding_entity == Some(var1.base.entity_id) {
        // src: Entity.java:992-995
        var1.base.ridden_by_entity = None;
        e.base.riding_entity = None;
        let (yaw, pitch) = (e.base.rotation_yaw, e.base.rotation_pitch);
        e.base.set_location_and_angles(
            var1.base.pos_x,
            var1.base.bounding_box.min_y + var1.base.height as f64,
            var1.base.pos_z,
            yaw,
            pitch,
        );
    } else {
        // src: Entity.java:996-1007
        if let Some(old) = e.base.riding_entity {
            match world.entity_mut(old) {
                Some(m) => m.base.ridden_by_entity = None,
                None => world.note_unimplemented("Entity.mountEntity: old mount out of its slot"),
            }
        }
        if let Some(r) = var1.base.ridden_by_entity {
            if r == id {
                // `var1.riddenByEntity` is `e` itself (links out of step).
                e.base.riding_entity = None;
            } else {
                match world.entity_mut(r) {
                    Some(rider) => rider.base.riding_entity = None,
                    None => {
                        world.note_unimplemented("Entity.mountEntity: old rider out of its slot")
                    }
                }
            }
        }
        e.base.riding_entity = Some(var1.base.entity_id);
        var1.base.ridden_by_entity = Some(id);
    }
}

/// `mountEntity(entity)` for a `target` that is in the world (in its slot):
/// takes it out for [`mount_entity`] and puts it back. Recorded, and nothing
/// done, if `target` is absent or out of its slot.
// src: Entity.java:982-1008
pub fn mount_entity_in_world(e: &mut Entity, world: &mut World, target: i32) {
    if world
        .with_entity_mut(target, |w, t| mount_entity(e, w, Some(t)))
        .is_none()
    {
        world.note_unimplemented("Entity.mountEntity: target not in its slot");
    }
}

/// `mountEntity(null)`: dismount (the `null` branch of [`mount_entity`]).
// src: Entity.java:982-991
pub fn dismount(e: &mut Entity, world: &mut World) {
    e.base.entity_rider_pitch_delta = 0.0;
    e.base.entity_rider_yaw_delta = 0.0;
    if let Some(mount_id) = e.base.riding_entity {
        if let Some(m) = world.entity_mut(mount_id) {
            let (x, y, z) = (
                m.base.pos_x,
                m.base.bounding_box.min_y + m.base.height as f64,
                m.base.pos_z,
            );
            m.base.ridden_by_entity = None;
            let (yaw, pitch) = (e.base.rotation_yaw, e.base.rotation_pitch);
            e.base.set_location_and_angles(x, y, z, yaw, pitch);
        } else {
            world.note_unimplemented("Entity.mountEntity(null): mount out of its slot");
        }
    }
    e.base.riding_entity = None;
}

/// `mountEntity(null)` with the mount in hand (it is out of its slot).
// src: Entity.java:982-991
pub fn dismount_from(e: &mut Entity, m: &mut EntityBase) {
    e.base.entity_rider_pitch_delta = 0.0;
    e.base.entity_rider_yaw_delta = 0.0;
    let (yaw, pitch) = (e.base.rotation_yaw, e.base.rotation_pitch);
    e.base.set_location_and_angles(
        m.pos_x,
        m.bounding_box.min_y + m.height as f64,
        m.pos_z,
        yaw,
        pitch,
    );
    m.ridden_by_entity = None;
    e.base.riding_entity = None;
}

/// `pushOutOfBlocks(x, y, z)`: nudges an entity stuck in a normal cube toward
/// the nearest open face.
// src: Entity.java:1097-1170
#[allow(unused_assignments)] // the last `var23 = …` is dead in Java too
pub fn push_out_of_blocks(
    e: &mut Entity,
    world: &mut World,
    var1: f64,
    var3: f64,
    var5: f64,
) -> bool {
    let var7 = floor_double(var1);
    let var8 = floor_double(var3);
    let var9 = floor_double(var5);
    let var10 = var1 - var7 as f64;
    let var12 = var3 - var8 as f64;
    let var14 = var5 - var9 as f64;
    if world.is_block_normal_cube(var7, var8, var9) {
        let var16 = !world.is_block_normal_cube(var7 - 1, var8, var9);
        let var17 = !world.is_block_normal_cube(var7 + 1, var8, var9);
        let var18 = !world.is_block_normal_cube(var7, var8 - 1, var9);
        let var19 = !world.is_block_normal_cube(var7, var8 + 1, var9);
        let var20 = !world.is_block_normal_cube(var7, var8, var9 - 1);
        let var21 = !world.is_block_normal_cube(var7, var8, var9 + 1);
        let mut var22: i8 = -1;
        let mut var23 = 9999.0;
        if var16 && var10 < var23 {
            var23 = var10;
            var22 = 0;
        }
        if var17 && 1.0 - var10 < var23 {
            var23 = 1.0 - var10;
            var22 = 1;
        }
        if var18 && var12 < var23 {
            var23 = var12;
            var22 = 2;
        }
        if var19 && 1.0 - var12 < var23 {
            var23 = 1.0 - var12;
            var22 = 3;
        }
        if var20 && var14 < var23 {
            var23 = var14;
            var22 = 4;
        }
        if var21 && 1.0 - var14 < var23 {
            var23 = 1.0 - var14;
            var22 = 5;
        }
        let var25 = e.base.rand.next_float() * 0.2 + 0.1;
        match var22 {
            0 => e.base.motion_x = -var25 as f64,
            1 => e.base.motion_x = var25 as f64,
            2 => e.base.motion_y = -var25 as f64,
            3 => e.base.motion_y = var25 as f64,
            4 => e.base.motion_z = -var25 as f64,
            5 => e.base.motion_z = var25 as f64,
            _ => {}
        }
    }
    false
}

/// `preparePlayerToSpawn()`: raises the entity a block at a time until its
/// box is free (or `posY` reaches 0), then stops it and levels the pitch.
// src: Entity.java:133-147
pub fn prepare_player_to_spawn(e: &mut Entity, world: &mut World) {
    // `if (this.worldObj != null)`: always set here.
    while e.base.pos_y > 0.0 {
        let (x, y, z) = (e.base.pos_x, e.base.pos_y, e.base.pos_z);
        e.base.set_position(x, y, z);
        let bb = e.base.bounding_box;
        if world.get_colliding_bounding_boxes(e, &bb).is_empty() {
            break;
        }
        e.base.pos_y += 1.0;
    }
    e.base.motion_x = 0.0;
    e.base.motion_y = 0.0;
    e.base.motion_z = 0.0;
    e.base.rotation_pitch = 0.0;
}

/// `getEntityBrightness(partialTick)`: the light brightness at 0.66 of the
/// box height above the feet, at least `entityBrightness`; just
/// `entityBrightness` when a chunk under the box is missing. `partialTick`
/// is unused by `Entity`'s body (and no simulation class overrides it).
// src: Entity.java:628-643
pub fn get_entity_brightness(e: &Entity, world: &mut World, _partial_tick: f32) -> f32 {
    let b = &e.base;
    let var2 = floor_double(b.pos_x);
    let var3 = (b.bounding_box.max_y - b.bounding_box.min_y) * 0.66;
    let var5 = floor_double(b.pos_y - b.y_offset as f64 + var3);
    let var6 = floor_double(b.pos_z);
    if world.check_chunks_exist(
        floor_double(b.bounding_box.min_x),
        floor_double(b.bounding_box.min_y),
        floor_double(b.bounding_box.min_z),
        floor_double(b.bounding_box.max_x),
        floor_double(b.bounding_box.max_y),
        floor_double(b.bounding_box.max_z),
    ) {
        let mut var7 = world.get_light_brightness(var2, var5, var6);
        if var7 < b.entity_brightness {
            var7 = b.entity_brightness;
        }
        var7
    } else {
        b.entity_brightness
    }
}

/// `Entity.applyEntityCollision(other)`: pushes the two apart, 0.05 per tick
/// scaled by the horizontal distance (`abs_max`, not Euclidean), unless one
/// rides the other. Both pushes go through the virtual `addVelocity`.
// src: Entity.java:709-734
pub fn apply_entity_collision(e: &mut Entity, world: &mut World, other: &mut Entity) {
    let id = e.base.entity_id;
    if other.base.ridden_by_entity != Some(id) && other.base.riding_entity != Some(id) {
        let mut var2 = other.base.pos_x - e.base.pos_x;
        let mut var4 = other.base.pos_z - e.base.pos_z;
        let mut var6 = math::abs_max(var2, var4);
        if var6 >= 0.009999999776482582 {
            var6 = math::sqrt_double(var6) as f64;
            var2 /= var6;
            var4 /= var6;
            let mut var8 = 1.0 / var6;
            if var8 > 1.0 {
                var8 = 1.0;
            }
            var2 *= var8;
            var4 *= var8;
            var2 *= 0.05000000074505806;
            var4 *= 0.05000000074505806;
            var2 *= (1.0f32 - e.base.entity_collision_reduction) as f64;
            var4 *= (1.0f32 - e.base.entity_collision_reduction) as f64;
            dispatch::add_velocity(e, world, -var2, 0.0, -var4);
            dispatch::add_velocity(other, world, var2, 0.0, var4);
        }
    }
}

/// `Entity.isInRangeToRenderVec3D(v)`: the squared distance to `v` against
/// `isInRangeToRenderDist`, Entity's body for both. The overrides
/// (`EntityLightningBolt`'s, and `isInRangeToRenderDist`'s of the egg,
/// fireball, fish hook and snowball) are render data the client implements
/// (`mc-client` `render::is_in_range_to_render_vec3d`); the simulation never
/// calls these on a tick path ([`dispatch::NOT_DISPATCHED`]).
// src: Entity.java:762-768
pub fn is_in_range_to_render_vec3d(e: &Entity, v: &mc_core::Vec3D) -> bool {
    let var2 = e.base.pos_x - v.x_coord;
    let var4 = e.base.pos_y - v.y_coord;
    let var6 = e.base.pos_z - v.z_coord;
    let var8 = var2 * var2 + var4 * var4 + var6 * var6;
    is_in_range_to_render_dist(e, var8)
}

/// `Entity.isInRangeToRenderDist(distSq)`: within 64 × the box's average edge
/// × `renderDistanceWeight`.
// src: Entity.java:770-774
pub fn is_in_range_to_render_dist(e: &Entity, var1: f64) -> bool {
    let mut var3 = e.base.bounding_box.get_average_edge_length();
    var3 *= 64.0 * e.base.render_distance_weight;
    var1 < var3 * var3
}

/// `Entity.getShadowSize()`: half the height. The overrides are render data
/// the client implements (`mc-client` `render::shadow::get_shadow_size`;
/// [`dispatch::NOT_DISPATCHED`]).
// src: Entity.java:869-871
pub fn get_shadow_size(e: &Entity) -> f32 {
    e.base.height / 2.0
}

/// `Entity.isEntityInsideOpaqueBlock()`: any of eight points around the eye
/// (±0.45 width, ±0.05 height) in a normal cube.
// src: Entity.java:892-906
#[allow(clippy::identity_op)] // `(var1 >> 0) % 2`, as the Java
pub fn is_entity_inside_opaque_block(e: &Entity, world: &mut World) -> bool {
    for var1 in 0..8i32 {
        let var2 = (((var1 >> 0) % 2) as f32 - 0.5) * e.base.width * 0.9;
        let var3 = (((var1 >> 1) % 2) as f32 - 0.5) * 0.1;
        let var4 = (((var1 >> 2) % 2) as f32 - 0.5) * e.base.width * 0.9;
        let var5 = floor_double(e.base.pos_x + var2 as f64);
        let var6 =
            floor_double(e.base.pos_y + dispatch::get_eye_height(e, world) as f64 + var3 as f64);
        let var7 = floor_double(e.base.pos_z + var4 as f64);
        if world.is_block_normal_cube(var5, var6, var7) {
            return true;
        }
    }
    false
}

/// `Entity.setPositionAndRotation2(x, y, z, yaw, pitch, increments)` (the
/// multiplayer position update; `increments` is unused here): placed, then
/// lifted onto the highest box it overlaps after a 1/32 horizontal shrink.
// src: Entity.java:1010-1028
#[allow(clippy::too_many_arguments)]
pub fn set_position_and_rotation2(
    e: &mut Entity,
    world: &mut World,
    var1: f64,
    mut var3: f64,
    var5: f64,
    var7: f32,
    var8: f32,
    _var9: i32,
) {
    e.base.set_position(var1, var3, var5);
    e.base.set_rotation(var7, var8);
    let bb = e.base.bounding_box.contract(0.03125, 0.0, 0.03125);
    let var10 = world.get_colliding_bounding_boxes(e, &bb);
    if !var10.is_empty() {
        let mut var11 = 0.0;
        for var14 in &var10 {
            if var14.max_y > var11 {
                var11 = var14.max_y;
            }
        }
        var3 += var11 - e.base.bounding_box.min_y;
        e.base.set_position(var1, var3, var5);
    }
}

/// `Entity.getCollisionBorderSize()`: the margin the pick ray adds to the
/// box (EntityRenderer.java:111).
// src: Entity.java:1030-1032
pub fn get_collision_border_size() -> f32 {
    0.1
}

/// `Entity.onStruckByLightning(bolt)`: 5 fire damage (virtual) and the fire
/// counter's step (`++fire`, 0 becomes 300), as `moveEntity`'s fire contact.
/// Its one caller is `EntityLightningBolt.onUpdate` (the entities within 3
/// blocks, [`super::lightning_bolt::on_update`]), through
/// [`dispatch::on_struck_by_lightning`].
// src: Entity.java:1085-1092
pub fn on_struck_by_lightning(e: &mut Entity, world: &mut World, _bolt: &Entity) {
    dispatch::deal_fire_damage(e, world, 5);
    e.base.fire += 1;
    if e.base.fire == 0 {
        e.base.fire = 300;
    }
}

/// `new EntityWeatherEffect(world)`: the abstract base of the lightning bolt
/// declares no field and no method, so its constructor is `Entity(World)`'s
/// (the bolt's own data and `entityInit` are `EntityLightningBolt`'s).
// src: EntityWeatherEffect.java:3-7
pub fn new_weather_effect(world: &mut World) -> EntityBase {
    EntityBase::new(world)
}

/// `Math.random()` through the world's single `MathRandom` stream
/// (PLAN_v4 §3.4 rule 3; the oracle's `HarnessMath`).
pub fn math_random(world: &mut World) -> f64 {
    world.math_random.next_double()
}

/// The oracle's entity seed mix (`HarnessWorld.mix`):
/// `seed ^ (k * 0x9E3779B97F4A7C15)` with wrapping 64-bit arithmetic.
pub fn mix_seed(seed: i64, k: i64) -> i64 {
    seed ^ k.wrapping_mul(0x9E3779B97F4A7C15u64 as i64)
}
