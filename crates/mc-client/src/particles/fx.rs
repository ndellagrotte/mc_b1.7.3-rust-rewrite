//! `EntityFX` (EntityFX.java), the base of every particle, with the part of
//! `Entity` it inherits and uses: the constructor's fields, `setSize`,
//! `setPosition`, `moveEntity` (the subset a particle reaches) and
//! `getEntityBrightness`.
//!
//! A particle is never in the world's entity list; it lives in the
//! [`super::EffectRenderer`]'s layers and reads the world (collision boxes,
//! light, materials) with the simulation's own queries.
//!
//! # Randomness
//!
//! Each particle has its own `Entity.rand` (`new Random()` in the `Entity`
//! constructor, Entity.java:96): seeded here from the clock as Java seeds
//! it ([`crate::clock::new_random_seed`]), one `JavaRandom` per particle,
//! so the draws are the same kind of stream vanilla has. `Math.random()` in
//! the constructors and in `EntityRainFX.onUpdate` is the client's own
//! clock-seeded stream ([`FxRandom::math_random`]), never the simulation's
//! (`World::math_random`): vanilla shares one JVM stream between them, which
//! the simulation cannot reproduce in a client anyway (census class C,
//! tools/oracle/census.md). Neither stream reaches the simulation.
//!
//! # What `moveEntity` keeps (Entity.java:293-541)
//!
//! - `noClip` (flame, Entity.java:294-298): the box moves, nothing collides.
//! - The web (`isInWeb`), the collision against
//!   `World.getCollidingBoundingBoxes` (block boxes, then other entities'
//!   `getBoundingBox` — boats; `EntityFX` has no `getCollisionBox`), the y,
//!   x, z order, `onGround`, the motion zeroing.
//! - The block callbacks of `onEntityCollidedWithBlock` that change the
//!   particle: soul sand (motion × 0.4, BlockSoulSand.java:13-16) and the web
//!   (`isInWeb`, BlockWeb.java:10-12). Cactus (`attackEntityFrom` only sets
//!   `beenAttacked`, Entity.java:742-749) and the portal (`Entity.setInPortal`
//!   is empty, Entity.java:1038-1039) change nothing a particle reads. The
//!   pressure plate and the detector rail (BlockPressurePlate.java:61-67,
//!   BlockDetectorRail.java:20-27) would look for entities on them and could
//!   switch the block: they are **not** called from the client, so a
//!   particle never changes the simulation (the oracle has no particles
//!   either).
//! - The fire counter: `isBoundingBoxBurning` → `fire`, and with the
//!   particle wet (`canBlockBeRainedOn`; `inWater` is never set, since
//!   `EntityFX.onUpdate` does not call `onEntityUpdate`) the `random.fizz`
//!   sound at the particle (Entity.java:521-539), queued as a world
//!   `playSound` event as `playSoundAtEntity` does.
//! - Dropped because they cannot apply to a particle: sneaking at an edge
//!   (`isSneaking()` is a data-watcher flag never set), step-up
//!   (`stepHeight` 0), walking (`canTriggerWalking()` is false,
//!   EntityFX.java:52-54), `field_9293_aM` (always true), the fall distance
//!   (`fall` only passes it to a rider).

use bevy::math::DVec3;
use mc_core::math::{floor_double, sqrt_double};
use mc_core::{Aabb, JavaRandom};
use mc_sim::block::{self, BlockKind};
use mc_sim::entity::{Entity, dispatch as entity_dispatch};
use mc_sim::world::World;

use crate::clock;
use crate::gl_material::{TessBatch, tess_color};

/// The random streams particles draw from (module docs).
#[derive(Clone, Debug)]
pub struct FxRandom {
    /// The client's `Math.random()` stream.
    pub math_random: JavaRandom,
    /// Where each particle's `Entity.rand` seed comes from.
    pub entity_seeds: EntitySeeds,
}

/// The seed of each particle's `Entity.rand`.
#[derive(Clone, Debug)]
pub enum EntitySeeds {
    /// `new Random()`: `seedUniquifier() ^ System.nanoTime()` (the game).
    Clock,
    /// Seeds drawn (`nextLong`) from one seeded stream (tests).
    Stream(JavaRandom),
}

impl FxRandom {
    /// The game's streams, seeded from the clock.
    pub fn from_clock() -> Self {
        FxRandom {
            math_random: JavaRandom::new(clock::new_random_seed()),
            entity_seeds: EntitySeeds::Clock,
        }
    }

    /// Deterministic streams for tests: `Math.random()` is `new
    /// Random(seed)`, each particle's `rand` is `new Random(s)` with `s` the
    /// next `nextLong()` of `new Random(seed + 1)`.
    pub fn seeded(seed: i64) -> Self {
        FxRandom {
            math_random: JavaRandom::new(seed),
            entity_seeds: EntitySeeds::Stream(JavaRandom::new(seed.wrapping_add(1))),
        }
    }

    /// `Math.random()`.
    pub fn math(&mut self) -> f64 {
        self.math_random.next_double()
    }

    /// A new particle's `rand` (`new Random()`, Entity.java:96).
    pub fn entity_rand(&mut self) -> JavaRandom {
        match &mut self.entity_seeds {
            EntitySeeds::Clock => JavaRandom::new(clock::new_random_seed()),
            EntitySeeds::Stream(r) => JavaRandom::new(r.next_long()),
        }
    }
}

/// The camera-facing basis `renderParticles` computes from the view
/// entity's rotation (EffectRenderer.java:50-54) and passes to every
/// `renderParticle(t, partial, var3, var7, var4, var5, var6)`: here in the
/// parameter order of `renderParticle`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Billboard {
    /// `cos(yaw)` (`renderParticle`'s `var3`).
    pub rot_x: f32,
    /// `cos(pitch)` (`var4`).
    pub rot_xz: f32,
    /// `sin(yaw)` (`var5`).
    pub rot_z: f32,
    /// `-sin(yaw) · sin(pitch)` (`var6`).
    pub rot_yz: f32,
    /// `cos(yaw) · sin(pitch)` (`var7`).
    pub rot_xy: f32,
}

impl Billboard {
    /// From the view entity's `rotationYaw` and `rotationPitch` (degrees;
    /// the current values, not interpolated).
    // src: EffectRenderer.java:50-54
    pub fn from_rotation(yaw: f32, pitch: f32) -> Self {
        use mc_core::math::{cos, sin};
        let var3 = cos(yaw * 3.1415927 / 180.0);
        let var4 = sin(yaw * 3.1415927 / 180.0);
        let var5 = -var4 * sin(pitch * 3.1415927 / 180.0);
        let var6 = var3 * sin(pitch * 3.1415927 / 180.0);
        let var7 = cos(pitch * 3.1415927 / 180.0);
        Billboard {
            rot_x: var3,
            rot_xz: var7,
            rot_z: var4,
            rot_yz: var5,
            rot_xy: var6,
        }
    }
}

/// The subclass of a particle and its own fields.
#[derive(Clone, Debug)]
pub enum FxKind {
    /// `EntityFX` itself (no call site spawns one in vanilla).
    Base,
    /// `EntityBubbleFX`.
    Bubble,
    /// `EntityDiggingFX`.
    Digging {
        /// `blockInstance`'s id.
        block_id: i32,
        /// `field_32001_o`: the constructor's `var15` (the side hit, or a
        /// `nextInt(6)` for a destroyed block); stored, never read
        /// (EntityDiggingFX.java:5, :14). See `docs/mappings.md`.
        side: i32,
    },
    /// `EntityExplodeFX`.
    Explode,
    /// `EntityFlameFX`.
    Flame {
        /// `flameScale`.
        flame_scale: f32,
    },
    /// `EntityFootStepFX`.
    FootStep {
        /// `footstepAge`.
        footstep_age: i32,
        /// `footstepMaxAge`.
        footstep_max_age: i32,
    },
    /// `EntityHeartFX`.
    Heart {
        /// `particleScaleOverTime`.
        particle_scale_over_time: f32,
    },
    /// `EntityLavaFX`.
    Lava {
        /// `lavaParticleScale`.
        lava_particle_scale: f32,
    },
    /// `EntityNoteFX`.
    Note {
        /// `noteParticleScale`.
        note_particle_scale: f32,
    },
    /// `EntityPickupFX` (the entity is drawn by WP-3.5b; [`super::PickupDraw`]).
    Pickup(Box<PickupData>),
    /// `EntityPortalFX`.
    Portal {
        /// `portalParticleScale`.
        portal_particle_scale: f32,
        /// `portalPosX/Y/Z`.
        portal_pos: DVec3,
    },
    /// `EntityRainFX`.
    Rain,
    /// `EntitySplashFX` (an `EntityRainFX`).
    Splash,
    /// `EntityReddustFX`.
    Reddust {
        /// `reddustParticleScale`.
        reddust_particle_scale: f32,
    },
    /// `EntitySlimeFX` (the snowball and slime ball icons).
    Slime,
    /// `EntitySmokeFX`.
    Smoke {
        /// `smokeParticleScale`.
        smoke_particle_scale: f32,
    },
    /// `EntitySnowShovelFX`.
    SnowShovel {
        /// `snowDigParticleScale`.
        snow_dig_particle_scale: f32,
    },
}

/// `EntityPickupFX`'s own fields.
#[derive(Clone, Debug)]
pub struct PickupData {
    /// `entityToPickUp`: the picked entity as the pickup left it (a copy;
    /// [`mc_sim::world::client_requests::PickupFx`]).
    pub entity_to_pick_up: Box<Entity>,
    /// `entityPickingUp`, by entity id (read live every frame).
    pub entity_picking_up: i32,
    /// `age`.
    pub age: i32,
    /// `maxAge` (3).
    pub max_age: i32,
    /// `yOffs` (-0.5).
    pub y_offs: f32,
}

/// One particle: `Entity`'s fields that a particle uses, `EntityFX`'s, and
/// the subclass ([`FxKind`]).
#[derive(Clone, Debug)]
pub struct EntityFx {
    /// `posX`.
    pub pos_x: f64,
    /// `posY`.
    pub pos_y: f64,
    /// `posZ`.
    pub pos_z: f64,
    /// `prevPosX` (0 until the first `onUpdate`, as in Java).
    pub prev_pos_x: f64,
    /// `prevPosY`.
    pub prev_pos_y: f64,
    /// `prevPosZ`.
    pub prev_pos_z: f64,
    /// `motionX`.
    pub motion_x: f64,
    /// `motionY`.
    pub motion_y: f64,
    /// `motionZ`.
    pub motion_z: f64,
    /// `boundingBox`.
    pub bounding_box: Aabb,
    /// `width`.
    pub width: f32,
    /// `height`.
    pub height: f32,
    /// `yOffset`.
    pub y_offset: f32,
    /// `ySize`.
    pub y_size: f32,
    /// `onGround`.
    pub on_ground: bool,
    /// `noClip`.
    pub no_clip: bool,
    /// `isInWeb`.
    pub is_in_web: bool,
    /// `isDead`.
    pub is_dead: bool,
    /// `fire`.
    pub fire: i32,
    /// `fireResistance` (1).
    pub fire_resistance: i32,
    /// `entityBrightness` (0).
    pub entity_brightness: f32,
    /// `rand` (module docs).
    pub rand: JavaRandom,
    /// `particleTextureIndex`.
    pub particle_texture_index: i32,
    /// `particleTextureJitterX`.
    pub particle_texture_jitter_x: f32,
    /// `particleTextureJitterY`.
    pub particle_texture_jitter_y: f32,
    /// `particleAge`.
    pub particle_age: i32,
    /// `particleMaxAge`.
    pub particle_max_age: i32,
    /// `particleScale`.
    pub particle_scale: f32,
    /// `particleGravity`.
    pub particle_gravity: f32,
    /// `particleRed`.
    pub particle_red: f32,
    /// `particleGreen`.
    pub particle_green: f32,
    /// `particleBlue`.
    pub particle_blue: f32,
    /// The subclass.
    pub kind: FxKind,
}

/// `(float)(Math.random() * 2.0D - 1.0D) * k` as a double.
fn spread(rnd: &mut FxRandom, k: f32) -> f64 {
    (((rnd.math() * 2.0 - 1.0) as f32) * k) as f64
}

impl EntityFx {
    /// `new EntityFX(world, x, y, z, vx, vy, vz)` (with `Entity(world)`
    /// before it): size 0.2, `yOffset` 0.1, a random motion of length
    /// `(rand + rand + 1) · 0.15 · 0.4` around the given one plus 0.1 up,
    /// texture jitter, scale `(0.5..1) · 2` and age `4 / (0.1..1)`.
    // src: EntityFX.java:18-37, Entity.java:75-113
    pub fn new(x: f64, y: f64, z: f64, vx: f64, vy: f64, vz: f64, rnd: &mut FxRandom) -> Self {
        let rand = rnd.entity_rand();
        let mut e = EntityFx {
            pos_x: 0.0,
            pos_y: 0.0,
            pos_z: 0.0,
            prev_pos_x: 0.0,
            prev_pos_y: 0.0,
            prev_pos_z: 0.0,
            motion_x: 0.0,
            motion_y: 0.0,
            motion_z: 0.0,
            bounding_box: Aabb::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            // src: Entity.java:86-87
            width: 0.6,
            height: 1.8,
            y_offset: 0.0,
            y_size: 0.0,
            on_ground: false,
            no_clip: false,
            is_in_web: false,
            is_dead: false,
            fire: 0,
            // src: Entity.java:98
            fire_resistance: 1,
            entity_brightness: 0.0,
            rand,
            particle_texture_index: 0,
            particle_texture_jitter_x: 0.0,
            particle_texture_jitter_y: 0.0,
            particle_age: 0,
            particle_max_age: 0,
            particle_scale: 0.0,
            particle_gravity: 0.0,
            particle_red: 0.0,
            particle_green: 0.0,
            particle_blue: 0.0,
            kind: FxKind::Base,
        };
        // src: Entity.java:110 (setPosition(0, 0, 0) with the 0.6 × 1.8 size)
        e.set_position(0.0, 0.0, 0.0);
        // src: EntityFX.java:20-22
        e.set_size(0.2, 0.2);
        e.y_offset = e.height / 2.0;
        e.set_position(x, y, z);
        // src: EntityFX.java:23
        e.particle_red = 1.0;
        e.particle_green = 1.0;
        e.particle_blue = 1.0;
        // src: EntityFX.java:24-26
        e.motion_x = vx + spread(rnd, 0.4);
        e.motion_y = vy + spread(rnd, 0.4);
        e.motion_z = vz + spread(rnd, 0.4);
        // src: EntityFX.java:27 (left to right: two draws)
        let a = rnd.math();
        let b = rnd.math();
        let var14 = ((a + b + 1.0) as f32) * 0.15;
        // src: EntityFX.java:28-31
        let var15 = sqrt_double(
            e.motion_x * e.motion_x + e.motion_y * e.motion_y + e.motion_z * e.motion_z,
        );
        e.motion_x = e.motion_x / var15 as f64 * var14 as f64 * 0.4000000059604645;
        e.motion_y =
            e.motion_y / var15 as f64 * var14 as f64 * 0.4000000059604645 + 0.10000000149011612;
        e.motion_z = e.motion_z / var15 as f64 * var14 as f64 * 0.4000000059604645;
        // src: EntityFX.java:32-36
        e.particle_texture_jitter_x = e.rand.next_float() * 3.0;
        e.particle_texture_jitter_y = e.rand.next_float() * 3.0;
        e.particle_scale = (e.rand.next_float() * 0.5 + 0.5) * 2.0;
        e.particle_max_age = (4.0 / (e.rand.next_float() * 0.9 + 0.1)) as i32;
        e.particle_age = 0;
        e
    }

    /// `setSize(w, h)`: only the fields (the box is not rebuilt).
    // src: Entity.java:153-156
    pub fn set_size(&mut self, w: f32, h: f32) {
        self.width = w;
        self.height = h;
    }

    /// `setPosition(x, y, z)`: the position and the box around it.
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

    /// `setEntityDead()`.
    // src: Entity.java:148-150
    pub fn set_entity_dead(&mut self) {
        self.is_dead = true;
    }

    /// `multiplyVelocity(f)`: x and z scaled, y scaled around the 0.1 lift.
    // src: EntityFX.java:39-44
    pub fn multiply_velocity(mut self, var1: f32) -> Self {
        self.motion_x *= var1 as f64;
        self.motion_y = (self.motion_y - 0.10000000149011612) * var1 as f64 + 0.10000000149011612;
        self.motion_z *= var1 as f64;
        self
    }

    /// `multipleParticleScaleBy(f)` (sic).
    // src: EntityFX.java:46-50
    pub fn multiple_particle_scale_by(mut self, var1: f32) -> Self {
        self.set_size(0.2 * var1, 0.2 * var1);
        self.particle_scale *= var1;
        self
    }

    /// `getFXLayer()`: 0 particles.png, 1 terrain.png, 2 items.png, 3 the
    /// lit pass (`renderLitParticles`).
    // src: EntityFX.java:96-98, EntityDiggingFX.java:29-31, EntitySlimeFX.java:12-14,
    // src: EntityFootStepFX.java:54-56, EntityPickupFX.java:51-53
    pub fn get_fx_layer(&self) -> usize {
        match self.kind {
            FxKind::Digging { .. } => 1,
            FxKind::Slime => 2,
            FxKind::FootStep { .. } | FxKind::Pickup(_) => 3,
            _ => 0,
        }
    }

    /// The position interpolated by `partial` (`prevPos + (pos - prevPos) ·
    /// partial`).
    pub fn interp_pos(&self, partial: f32) -> DVec3 {
        let p = partial as f64;
        DVec3::new(
            self.prev_pos_x + (self.pos_x - self.prev_pos_x) * p,
            self.prev_pos_y + (self.pos_y - self.prev_pos_y) * p,
            self.prev_pos_z + (self.pos_z - self.prev_pos_z) * p,
        )
    }

    // ---- Entity.moveEntity ---------------------------------------------------------

    /// `moveEntity(dx, dy, dz)`: the subset a particle reaches (module docs).
    // src: Entity.java:293-541
    pub fn move_entity(&mut self, world: &mut World, mut var1: f64, mut var3: f64, mut var5: f64) {
        if self.no_clip {
            // src: Entity.java:294-298
            self.bounding_box.offset(var1, var3, var5);
            self.pos_x = (self.bounding_box.min_x + self.bounding_box.max_x) / 2.0;
            self.pos_y = self.bounding_box.min_y + self.y_offset as f64 - self.y_size as f64;
            self.pos_z = (self.bounding_box.min_z + self.bounding_box.max_z) / 2.0;
            return;
        }
        self.y_size *= 0.4;
        // src: Entity.java:303-311
        if self.is_in_web {
            self.is_in_web = false;
            var1 *= 0.25;
            var3 *= 0.05000000074505806;
            var5 *= 0.25;
            self.motion_x = 0.0;
            self.motion_y = 0.0;
            self.motion_z = 0.0;
        }
        let var11 = var1;
        let var13 = var3;
        let var15 = var5;
        // src: Entity.java:341-376 (sneaking skipped: never sneaking)
        let var35 = colliding_bounding_boxes(world, &self.bounding_box.add_coord(var1, var3, var5));
        for b in &var35 {
            var3 = b.calculate_y_offset(&self.bounding_box, var3);
        }
        self.bounding_box.offset(0.0, var3, 0.0);
        for b in &var35 {
            var1 = b.calculate_x_offset(&self.bounding_box, var1);
        }
        self.bounding_box.offset(var1, 0.0, 0.0);
        for b in &var35 {
            var5 = b.calculate_z_offset(&self.bounding_box, var5);
        }
        self.bounding_box.offset(0.0, 0.0, var5);
        // src: Entity.java:380-476 (step-up skipped: stepHeight 0; the fall
        // distance only reaches a rider), :453-472
        self.pos_x = (self.bounding_box.min_x + self.bounding_box.max_x) / 2.0;
        self.pos_y = self.bounding_box.min_y + self.y_offset as f64 - self.y_size as f64;
        self.pos_z = (self.bounding_box.min_z + self.bounding_box.max_z) / 2.0;
        self.on_ground = var13 != var3 && var13 < 0.0;
        if var11 != var1 {
            self.motion_x = 0.0;
        }
        if var13 != var3 {
            self.motion_y = 0.0;
        }
        if var15 != var5 {
            self.motion_z = 0.0;
        }
        // src: Entity.java:478-499 skipped: canTriggerWalking() is false
        // src: Entity.java:502-518
        let bb = self.bounding_box;
        let var38 = floor_double(bb.min_x + 0.001);
        let var26 = floor_double(bb.min_y + 0.001);
        let var39 = floor_double(bb.min_z + 0.001);
        let var28 = floor_double(bb.max_x - 0.001);
        let var40 = floor_double(bb.max_y - 0.001);
        let var30 = floor_double(bb.max_z - 0.001);
        if world.check_chunks_exist(var38, var26, var39, var28, var40, var30) {
            for var31 in var38..=var28 {
                for var32 in var26..=var40 {
                    for var33 in var39..=var30 {
                        let var34 = world.get_block_id(var31, var32, var33);
                        if var34 > 0 {
                            self.on_entity_collided_with_block(var34);
                        }
                    }
                }
            }
        }
        // src: Entity.java:521-539
        let var42 = self.is_wet(world);
        if world.is_bounding_box_burning(&self.bounding_box.contract(0.001, 0.001, 0.001)) {
            // dealFireDamage(1) → attackEntityFrom: beenAttacked only
            if !var42 {
                self.fire += 1;
                if self.fire == 0 {
                    self.fire = 300;
                }
            }
        } else if self.fire <= 0 {
            self.fire = -self.fire_resistance;
        }
        if var42 && self.fire > 0 {
            // playSoundAtEntity(this, "random.fizz", 0.7F, 1.6F + (r - r) * 0.4F)
            // src: World.java:863-868 (posY - yOffset)
            let pitch = 1.6 + (self.rand.next_float() - self.rand.next_float()) * 0.4;
            world.play_sound_effect(
                self.pos_x,
                self.pos_y - self.y_offset as f64,
                self.pos_z,
                "random.fizz",
                0.7,
                pitch,
            );
            self.fire = -self.fire_resistance;
        }
    }

    /// `Block.onEntityCollidedWithBlock(world, x, y, z, this)`: the cases
    /// that change a particle (module docs).
    fn on_entity_collided_with_block(&mut self, id: i32) {
        match block::block(id).map(|b| &b.kind) {
            // src: BlockSoulSand.java:13-16
            Some(BlockKind::SoulSand) => {
                self.motion_x *= 0.4;
                self.motion_z *= 0.4;
            }
            // src: BlockWeb.java:10-12
            Some(BlockKind::Web) => self.is_in_web = true,
            _ => {}
        }
    }

    /// `isWet()`: `inWater` (never set for a particle) or rained on.
    // src: Entity.java:576-578
    fn is_wet(&self, world: &mut World) -> bool {
        world.can_block_be_rained_on(
            floor_double(self.pos_x),
            floor_double(self.pos_y),
            floor_double(self.pos_z),
        )
    }

    /// `Entity.getEntityBrightness(partial)`: the light brightness at the
    /// particle's middle when its box's chunks exist, at least
    /// `entityBrightness`.
    // src: Entity.java:628-643
    pub fn entity_brightness_base(&self, world: &mut World) -> f32 {
        let (var2, var5, var6) = self.brightness_sample_pos();
        let bb = &self.bounding_box;
        if world.check_chunks_exist(
            floor_double(bb.min_x),
            floor_double(bb.min_y),
            floor_double(bb.min_z),
            floor_double(bb.max_x),
            floor_double(bb.max_y),
            floor_double(bb.max_z),
        ) {
            let mut var7 = world.get_light_brightness(var2, var5, var6);
            if var7 < self.entity_brightness {
                var7 = self.entity_brightness;
            }
            var7
        } else {
            self.entity_brightness
        }
    }

    /// The block `getEntityBrightness` reads the light of: `floor(posX)`,
    /// `floor(posY - yOffset + boxHeight · 0.66)`, `floor(posZ)`.
    // src: Entity.java:629-632
    pub fn brightness_sample_pos(&self) -> (i32, i32, i32) {
        let var2 = floor_double(self.pos_x);
        let var3 = (self.bounding_box.max_y - self.bounding_box.min_y) * 0.66;
        let var5 = floor_double(self.pos_y - self.y_offset as f64 + var3);
        let var6 = floor_double(self.pos_z);
        (var2, var5, var6)
    }

    /// `EntityFX.renderParticle`'s quad: the tile `particleTextureIndex` of
    /// a 16 × 16 atlas (a tile is `0.0624375` wide, just under 1/16), the
    /// half size `0.1 · particleScale`, at the interpolated position
    /// relative to `interpPos`, coloured `rgb · brightness`.
    // src: EntityFX.java:79-94
    pub fn render_base_quad(
        &self,
        b: &mut TessBatch,
        partial: f32,
        bb: &Billboard,
        interp: DVec3,
        brightness: f32,
    ) {
        let [var8, var9, var10, var11] = particle_uv(self.particle_texture_index);
        let c = tess_color(
            self.particle_red * brightness,
            self.particle_green * brightness,
            self.particle_blue * brightness,
            1.0,
        );
        self.emit_quad(
            b,
            partial,
            bb,
            interp,
            c,
            [[var9, var11], [var9, var10], [var8, var10], [var8, var11]],
        );
    }

    /// `EntityDiggingFX` / `EntitySlimeFX.renderParticle`'s quad: a quarter
    /// tile (`0.015609375` wide) at the jittered offset inside the tile,
    /// mirrored left to right compared with the base.
    // src: EntityDiggingFX.java:33-48, EntitySlimeFX.java:16-31
    pub fn render_sub_tile_quad(
        &self,
        b: &mut TessBatch,
        partial: f32,
        bb: &Billboard,
        interp: DVec3,
        brightness: f32,
    ) {
        let [var8, var9, var10, var11] = sub_tile_uv(
            self.particle_texture_index,
            self.particle_texture_jitter_x,
            self.particle_texture_jitter_y,
        );
        let c = tess_color(
            brightness * self.particle_red,
            brightness * self.particle_green,
            brightness * self.particle_blue,
            1.0,
        );
        self.emit_quad(
            b,
            partial,
            bb,
            interp,
            c,
            [[var8, var11], [var8, var10], [var9, var10], [var9, var11]],
        );
    }

    /// The four `addVertexWithUV` calls shared by every billboard.
    // src: EntityFX.java:84-93
    fn emit_quad(
        &self,
        b: &mut TessBatch,
        partial: f32,
        bb: &Billboard,
        interp: DVec3,
        c: [f32; 4],
        uv: [[f32; 2]; 4],
    ) {
        let var12 = 0.1 * self.particle_scale;
        let p = partial as f64;
        let var13 = (self.prev_pos_x + (self.pos_x - self.prev_pos_x) * p - interp.x) as f32;
        let var14 = (self.prev_pos_y + (self.pos_y - self.prev_pos_y) * p - interp.y) as f32;
        let var15 = (self.prev_pos_z + (self.pos_z - self.prev_pos_z) * p - interp.z) as f32;
        let Billboard {
            rot_x: var3,
            rot_xz: var4,
            rot_z: var5,
            rot_yz: var6,
            rot_xy: var7,
        } = *bb;
        b.vertex(
            [
                var13 - var3 * var12 - var6 * var12,
                var14 - var4 * var12,
                var15 - var5 * var12 - var7 * var12,
            ],
            uv[0],
            c,
        );
        b.vertex(
            [
                var13 - var3 * var12 + var6 * var12,
                var14 + var4 * var12,
                var15 - var5 * var12 + var7 * var12,
            ],
            uv[1],
            c,
        );
        b.vertex(
            [
                var13 + var3 * var12 + var6 * var12,
                var14 + var4 * var12,
                var15 + var5 * var12 + var7 * var12,
            ],
            uv[2],
            c,
        );
        b.vertex(
            [
                var13 + var3 * var12 - var6 * var12,
                var14 - var4 * var12,
                var15 + var5 * var12 - var7 * var12,
            ],
            uv[3],
            c,
        );
    }
}

/// `EntityFX.renderParticle`'s texture rectangle of tile `index`:
/// `[u_min, u_max, v_min, v_max]` = `(index % 16) / 16`, `+ 0.0624375`,
/// `(index / 16) / 16`, `+ 0.0624375` (Java's truncating `%` and `/`).
// src: EntityFX.java:80-83
pub fn particle_uv(index: i32) -> [f32; 4] {
    let var8 = (index % 16) as f32 / 16.0;
    let var9 = var8 + 0.0624375;
    let var10 = (index / 16) as f32 / 16.0;
    let var11 = var10 + 0.0624375;
    [var8, var9, var10, var11]
}

/// The digging/slime particles' quarter-tile rectangle:
/// `[u_min, u_max, v_min, v_max]` = `((index % 16) + jitterX / 4) / 16`,
/// `+ 0.015609375`, the same for v.
// src: EntityDiggingFX.java:34-37, EntitySlimeFX.java:17-20
pub fn sub_tile_uv(index: i32, jitter_x: f32, jitter_y: f32) -> [f32; 4] {
    let var8 = ((index % 16) as f32 + jitter_x / 4.0) / 16.0;
    let var9 = var8 + 0.015609375;
    let var10 = ((index / 16) as f32 + jitter_y / 4.0) / 16.0;
    let var11 = var10 + 0.015609375;
    [var8, var9, var10, var11]
}

/// `World.getCollidingBoundingBoxes(particle, box)`: every block's
/// collision boxes in the box's columns (from one block below), then the
/// boxes (`getBoundingBox`) of the world's entities within 0.25 of it. The
/// particle is not in the world, so no entity is excluded; `EntityFX` has
/// no `getCollisionBox(other)` (Entity.java:912-914 returns null).
// src: World.java:959-997
pub fn colliding_bounding_boxes(world: &mut World, bb: &Aabb) -> Vec<Aabb> {
    let mut out = Vec::new();
    let var3 = floor_double(bb.min_x);
    let var4 = floor_double(bb.max_x + 1.0);
    let var5 = floor_double(bb.min_y);
    let var6 = floor_double(bb.max_y + 1.0);
    let var7 = floor_double(bb.min_z);
    let var8 = floor_double(bb.max_z + 1.0);
    for var9 in var3..var4 {
        for var10 in var7..var8 {
            if world.block_exists(var9, 64, var10) {
                for var11 in var5 - 1..var6 {
                    let id = world.get_block_id(var9, var11, var10);
                    if block::block(id).is_some() {
                        block::dispatch::get_colliding_bounding_boxes(
                            world, id, var9, var11, var10, bb, &mut out,
                        );
                    }
                }
            }
        }
    }
    let var14 = 0.25;
    for id in world.get_entities_within_aabb_excluding_entity(None, &bb.expand(var14, var14, var14))
    {
        if let Some(other) = world.entity(id)
            && let Some(var13) = entity_dispatch::get_bounding_box(other, world)
            && var13.intersects_with(bb)
        {
            out.push(var13);
        }
    }
    out
}
