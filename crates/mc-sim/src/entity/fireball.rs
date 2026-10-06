//! `EntityFireball` (EntityFireball.java): the ghast's fireball. It flies
//! along its motion plus a constant acceleration, without gravity, and on
//! any hit makes a flaming explosion of size 1 (`World.newExplosion(null, …,
//! 1.0F, true)`, World.java:1522-1528; [`crate::world::explosion`]).
//!
//! Transcribed: the `(World)` and `(World, EntityLiving, ax, ay, az)`
//! constructors, `onUpdate`, `canBeCollidedWith` (in
//! [`super::dispatch::can_be_collided_with`]), `attackEntityFrom` (a hit
//! with an attacker sends it along the attacker's look), the NBT bodies.
//! Not ported: the `(World, x, y, z, ax, ay, az)` constructor
//! (`NetClientHandler` only, multiplayer, D3), `getCollisionBorderSize`
//! (only `EntityRenderer`'s mouse-over pick, client), `isInRangeToRenderDist`,
//! `getShadowSize` (render).
//!
//! The explosion runs while the fireball is out of its slot for its update;
//! it is passed as `in_hand` to the explosion's entity pass, which vanilla's
//! scan would reach (the fireball is still alive and in its chunk then).
//! The explosion's own `Random` (`ExplosionRNG`, drawn for the fire) is
//! clock-seeded in vanilla and has no harness hook for an explosion made by
//! an entity (tools/oracle/census.md §2); the port seeds it as every
//! explosion's ([`crate::world::explosion`]).

use mc_core::Vec3D;
use mc_core::math;
use mc_nbt::Tag;

use super::arrow::{attack_entity_by_id, closest_entity_hit, entity_hit, update_flight_rotation};
use super::{Entity, EntityBase, EntityKindData, base, dispatch};
use crate::save::SaveError;
use crate::save::nbt::{self as n, Compound};
use crate::world::World;
use crate::world::explosion::Explosion;

/// The fields `EntityFireball` declares (EntityFireball.java:6-17).
#[derive(Clone, Debug, PartialEq)]
pub struct FireballData {
    /// `xTile` (private).
    pub x_tile: i32,
    /// `yTile` (private).
    pub y_tile: i32,
    /// `zTile` (private).
    pub z_tile: i32,
    /// `inTile` (private).
    pub in_tile: i32,
    /// `inGround` (private): never set in flight (any hit explodes it);
    /// NBT only.
    pub in_ground: bool,
    /// `shake`.
    pub shake: i32,
    /// `shootingEntity` (an `EntityLiving`), as its entity id.
    pub shooting_entity: Option<i32>,
    /// `ticksAlive` (private, no initialiser): ticks in the ground.
    pub ticks_alive: i32,
    /// `ticksInAir` (private): the shooter can be hit from 25 on.
    pub ticks_in_air: i32,
    /// `accelerationX`.
    pub acceleration_x: f64,
    /// `accelerationY`.
    pub acceleration_y: f64,
    /// `accelerationZ`.
    pub acceleration_z: f64,
}

impl Default for FireballData {
    /// The field initialisers.
    // src: EntityFireball.java:6-17
    fn default() -> Self {
        FireballData {
            x_tile: -1,
            y_tile: -1,
            z_tile: -1,
            in_tile: 0,
            in_ground: false,
            shake: 0,
            shooting_entity: None,
            ticks_alive: 0,
            ticks_in_air: 0,
            acceleration_x: 0.0,
            acceleration_y: 0.0,
            acceleration_z: 0.0,
        }
    }
}

fn data(e: &mut Entity) -> &mut FireballData {
    match &mut e.kind {
        EntityKindData::Fireball(d) => d,
        _ => panic!("EntityFireball method on another kind"),
    }
}

/// `new EntityFireball(world)`: size 1 x 1, at rest, no acceleration (the
/// harness `spawn` op's).
// src: EntityFireball.java:19-22
pub fn new_entity_fireball_empty(world: &mut World) -> Entity {
    let mut b = EntityBase::new(world);
    // entityInit() is empty (EntityFireball.java:24-25); field initialisers.
    let d = FireballData::default();
    b.set_size(1.0, 1.0);
    Entity {
        base: b,
        kind: EntityKindData::Fireball(d),
    }
}

/// `new EntityFireball(world, shooter, ax, ay, az)` (the ghast,
/// EntityGhast.java:86): at the shooter's position and angles, at rest,
/// accelerating 0.1 per tick toward `(ax, ay, az)` jittered by three
/// Gaussian draws (times 0.4) on the fireball's `rand`.
// src: EntityFireball.java:44-59
pub fn new_entity_fireball_shot(
    world: &mut World,
    shooter: &Entity,
    mut var3: f64,
    mut var5: f64,
    mut var7: f64,
) -> Entity {
    let mut e = new_entity_fireball_empty(world);
    data(&mut e).shooting_entity = Some(shooter.base.entity_id);
    let b = &mut e.base;
    b.set_size(1.0, 1.0);
    b.set_location_and_angles(
        shooter.base.pos_x,
        shooter.base.pos_y,
        shooter.base.pos_z,
        shooter.base.rotation_yaw,
        shooter.base.rotation_pitch,
    );
    b.set_position(b.pos_x, b.pos_y, b.pos_z);
    b.y_offset = 0.0;
    b.motion_x = 0.0;
    b.motion_y = 0.0;
    b.motion_z = 0.0;
    var3 += b.rand.next_gaussian() * 0.4;
    var5 += b.rand.next_gaussian() * 0.4;
    var7 += b.rand.next_gaussian() * 0.4;
    let var9 = math::sqrt_double(var3 * var3 + var5 * var5 + var7 * var7) as f64;
    let d = data(&mut e);
    d.acceleration_x = var3 / var9 * 0.1;
    d.acceleration_y = var5 / var9 * 0.1;
    d.acceleration_z = var7 / var9 * 0.1;
    e
}

/// `EntityFireball.onUpdate()`.
// src: EntityFireball.java:61-174
pub fn on_update(e: &mut Entity, world: &mut World) {
    base::on_update(e, world);
    // src: EntityFireball.java:63-66
    e.base.fire = 10;
    {
        let d = data(e);
        if d.shake > 0 {
            d.shake -= 1;
        }
    }
    // src: EntityFireball.java:68-87
    if data(e).in_ground {
        let (xt, yt, zt) = {
            let d = data(e);
            (d.x_tile, d.y_tile, d.z_tile)
        };
        let var1 = world.get_block_id(xt, yt, zt);
        let d = data(e);
        if var1 == d.in_tile {
            d.ticks_alive += 1;
            if d.ticks_alive == 1200 {
                dispatch::set_entity_dead(e, world);
            }
            return;
        }
        d.in_ground = false;
        let b = &mut e.base;
        b.motion_x *= (b.rand.next_float() * 0.2) as f64;
        b.motion_y *= (b.rand.next_float() * 0.2) as f64;
        b.motion_z *= (b.rand.next_float() * 0.2) as f64;
        let d = data(e);
        d.ticks_alive = 0;
        d.ticks_in_air = 0;
    } else {
        data(e).ticks_in_air += 1;
    }

    // src: EntityFireball.java:89-96
    let (px, py, pz, mx, my, mz) = {
        let b = &e.base;
        (
            b.pos_x, b.pos_y, b.pos_z, b.motion_x, b.motion_y, b.motion_z,
        )
    };
    let mut var15 = Vec3D::create_vector(px, py, pz);
    let var2 = Vec3D::create_vector(px + mx, py + my, pz + mz);
    let mut var3 = world.ray_trace_blocks(&mut var15, &var2);
    let var15 = Vec3D::create_vector(px, py, pz);
    let mut var2 = Vec3D::create_vector(px + mx, py + my, pz + mz);
    if let Some(h) = &var3 {
        var2 = Vec3D::create_vector(h.hit_vec.x_coord, h.hit_vec.y_coord, h.hit_vec.z_coord);
    }

    // src: EntityFireball.java:98-120 (no multiplayer guard here)
    let (shooter, ticks_in_air) = {
        let d = data(e);
        (d.shooting_entity, d.ticks_in_air)
    };
    if let Some(var4) = closest_entity_hit(e, world, &var15, &var2, shooter, ticks_in_air, 25) {
        var3 = Some(entity_hit(world, var4));
    }

    // src: EntityFireball.java:122-131
    if let Some(var3) = var3 {
        if !world.multiplayer_world {
            if let Some(hit) = var3.entity_hit {
                // `if (… && attackEntityFrom(shootingEntity, 0)) {}`: unused.
                attack_entity_by_id(world, hit, shooter, 0);
            }
            // src: EntityFireball.java:127: worldObj.newExplosion((Entity)null,
            // posX, posY, posZ, 1.0F, true) (World.java:1522-1528), with the
            // fireball in hand (module docs).
            let (x, y, z) = (e.base.pos_x, e.base.pos_y, e.base.pos_z);
            let mut var10 = Explosion::new(world, None, x, y, z, 1.0);
            var10.is_flaming = true;
            var10.do_explosion_a(world, None, Some(e));
            var10.do_explosion_b(world, true);
        }
        dispatch::set_entity_dead(e, world);
    }

    // src: EntityFireball.java:133-173
    let (ax, ay, az) = {
        let d = data(e);
        (d.acceleration_x, d.acceleration_y, d.acceleration_z)
    };
    let b = &mut e.base;
    b.pos_x += b.motion_x;
    b.pos_y += b.motion_y;
    b.pos_z += b.motion_z;
    update_flight_rotation(b);
    let mut var17: f32 = 0.95;
    if b.is_in_water() {
        for _ in 0..4 {
            let var19: f32 = 0.25;
            world.spawn_particle(
                "bubble",
                b.pos_x - b.motion_x * var19 as f64,
                b.pos_y - b.motion_y * var19 as f64,
                b.pos_z - b.motion_z * var19 as f64,
                b.motion_x,
                b.motion_y,
                b.motion_z,
            );
        }
        var17 = 0.8;
    }
    b.motion_x += ax;
    b.motion_y += ay;
    b.motion_z += az;
    b.motion_x *= var17 as f64;
    b.motion_y *= var17 as f64;
    b.motion_z *= var17 as f64;
    world.spawn_particle("smoke", b.pos_x, b.pos_y + 0.5, b.pos_z, 0.0, 0.0, 0.0);
    b.set_position(b.pos_x, b.pos_y, b.pos_z);
}

/// `attacker.getLookVec()`: `null` for `Entity`; `EntityLiving` returns
/// `getLook(1.0F)`, the unit vector of its yaw and pitch.
// src: Entity.java:1034-1036, EntityLiving.java:809-823
fn get_look_vec(attacker: &Entity) -> Option<Vec3D> {
    attacker.kind.living()?;
    let b = &attacker.base;
    // src: EntityLiving.java:818-823 (the `var1 == 1.0F` branch)
    let var2 = math::cos(-b.rotation_yaw * 0.017453292 - 3.1415927);
    let var3 = math::sin(-b.rotation_yaw * 0.017453292 - 3.1415927);
    let var4 = -math::cos(-b.rotation_pitch * 0.017453292);
    let var5 = math::sin(-b.rotation_pitch * 0.017453292);
    Some(Vec3D::create_vector(
        (var3 * var4) as f64,
        var5 as f64,
        (var2 * var4) as f64,
    ))
}

/// `EntityFireball.attackEntityFrom(attacker, damage)`: any hit with an
/// attacker that has a look vector (a living one) sends the fireball along
/// that look, accelerating at a tenth of it; true whenever there is an
/// attacker.
// src: EntityFireball.java:202-219
pub fn attack_entity_from(
    e: &mut Entity,
    _world: &mut World,
    attacker: Option<&Entity>,
    _damage: i32,
) -> bool {
    e.base.set_been_attacked();
    let Some(var1) = attacker else {
        return false;
    };
    if let Some(var3) = get_look_vec(var1) {
        let b = &mut e.base;
        b.motion_x = var3.x_coord;
        b.motion_y = var3.y_coord;
        b.motion_z = var3.z_coord;
        let (mx, my, mz) = (b.motion_x, b.motion_y, b.motion_z);
        let d = data(e);
        d.acceleration_x = mx * 0.1;
        d.acceleration_y = my * 0.1;
        d.acceleration_z = mz * 0.1;
    }
    true
}

/// `EntityFireball.writeEntityToNBT(compound)` (unreachable from a chunk
/// save: `EntityList` has no name for the class, so `addEntityID` skips it).
// src: EntityFireball.java:176-183
pub fn write_entity_to_nbt(d: &FireballData, c: &mut Compound) {
    n::set(c, "xTile", Tag::Short(d.x_tile as i16));
    n::set(c, "yTile", Tag::Short(d.y_tile as i16));
    n::set(c, "zTile", Tag::Short(d.z_tile as i16));
    n::set(c, "inTile", Tag::Byte(d.in_tile as i8));
    n::set(c, "shake", Tag::Byte(d.shake as i8));
    n::set(c, "inGround", Tag::Byte(if d.in_ground { 1 } else { 0 }));
}

/// `EntityFireball.readEntityFromNBT(compound)`.
// src: EntityFireball.java:185-192
pub fn read_entity_from_nbt(d: &mut FireballData, c: &Compound) -> Result<(), SaveError> {
    d.x_tile = n::get_short(c, "xTile")? as i32;
    d.y_tile = n::get_short(c, "yTile")? as i32;
    d.z_tile = n::get_short(c, "zTile")? as i32;
    d.in_tile = n::get_byte(c, "inTile")? as i32 & 255;
    d.shake = n::get_byte(c, "shake")? as i32 & 255;
    d.in_ground = n::get_byte(c, "inGround")? == 1;
    Ok(())
}
