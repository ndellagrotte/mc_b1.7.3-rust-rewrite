//! `EntityTNTPrimed` (EntityTNTPrimed.java): lit TNT. It falls and slides
//! like a falling block, counts its fuse down and at the end explodes with
//! size 4 (`World.createExplosion(null, …, 4.0F)`, World.java:1518-1520;
//! [`crate::world::explosion`]).
//!
//! Transcribed: both constructors (the position one draws one
//! `Math.random()`, the horizontal kick direction, census §1),
//! `canTriggerWalking` (in [`super::dispatch::can_trigger_walking`]),
//! `canBeCollidedWith` (in [`super::dispatch::can_be_collided_with`]),
//! `onUpdate`, `explode`, the NBT bodies. Not ported: `getShadowSize`
//! (render).
//!
//! `onUpdate` does not call `super.onUpdate()`: `ticksExisted` stays 0 and
//! there is no fire, water or void handling. The explosion runs while the
//! TNT (already marked dead, EntityTNTPrimed.java:55) is out of its slot for
//! its update; vanilla's entity scan still finds it in its chunk and pushes
//! it, so it is passed as `in_hand`.

use mc_core::math;
use mc_nbt::Tag;

use super::{Entity, EntityBase, EntityKindData, base, dispatch};
use crate::save::SaveError;
use crate::save::nbt::{self as n, Compound};
use crate::world::World;
use crate::world::explosion::Explosion;

/// The fields `EntityTNTPrimed` declares.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TntPrimedData {
    /// `fuse`: ticks left; 80 from the position constructor, shorter when
    /// an explosion lit it (BlockTNT.java:41).
    // src: EntityTNTPrimed.java:4
    pub fuse: i32,
}

fn data(e: &mut Entity) -> &mut TntPrimedData {
    match &mut e.kind {
        EntityKindData::TntPrimed(d) => d,
        _ => panic!("EntityTNTPrimed method on another kind"),
    }
}

/// `new EntityTNTPrimed(world)`: the `EntityList` / NBT-load constructor and
/// the harness `spawn` op's: fuse 0, size 0.98, centred (`yOffset` is half
/// the height).
// src: EntityTNTPrimed.java:6-12
pub fn new_entity_tnt_primed_empty(world: &mut World) -> Entity {
    let mut b = EntityBase::new(world);
    // entityInit() is empty (EntityTNTPrimed.java:27-28).
    let d = TntPrimedData { fuse: 0 };
    b.prevent_entity_spawning = true;
    b.set_size(0.98, 0.98);
    b.y_offset = b.height / 2.0;
    Entity {
        base: b,
        kind: EntityKindData::TntPrimed(d),
    }
}

/// `new EntityTNTPrimed(world, x, y, z)`: at `(x, y, z)`, a small kick in a
/// `Math.random()` direction, 0.2 up, fuse 80.
// src: EntityTNTPrimed.java:14-25
pub fn new_entity_tnt_primed(world: &mut World, x: f64, y: f64, z: f64) -> Entity {
    let mut e = new_entity_tnt_primed_empty(world);
    let b = &mut e.base;
    b.set_position(x, y, z);
    // src: EntityTNTPrimed.java:17 (census §1, class S)
    let var8 = (base::math_random(world) * 3.1415927410125732 * 2.0) as f32;
    b.motion_x = (-math::sin(var8 * 3.1415927 / 180.0) * 0.02) as f64;
    b.motion_y = 0.20000000298023224;
    b.motion_z = (-math::cos(var8 * 3.1415927 / 180.0) * 0.02) as f64;
    b.prev_pos_x = x;
    b.prev_pos_y = y;
    b.prev_pos_z = z;
    data(&mut e).fuse = 80;
    e
}

/// `EntityTNTPrimed.canBeCollidedWith()`: while alive.
// src: EntityTNTPrimed.java:34-36
pub fn can_be_collided_with(e: &Entity) -> bool {
    !e.base.is_dead
}

/// `EntityTNTPrimed.onUpdate()`.
// src: EntityTNTPrimed.java:38-64
pub fn on_update(e: &mut Entity, world: &mut World) {
    let b = &mut e.base;
    b.prev_pos_x = b.pos_x;
    b.prev_pos_y = b.pos_y;
    b.prev_pos_z = b.pos_z;
    b.motion_y -= 0.03999999910593033;
    let (mx, my, mz) = (b.motion_x, b.motion_y, b.motion_z);
    dispatch::move_entity(e, world, mx, my, mz);
    let b = &mut e.base;
    b.motion_x *= 0.9800000190734863;
    b.motion_y *= 0.9800000190734863;
    b.motion_z *= 0.9800000190734863;
    if b.on_ground {
        b.motion_x *= 0.699999988079071;
        b.motion_z *= 0.699999988079071;
        b.motion_y *= -0.5;
    }
    // src: EntityTNTPrimed.java:53: `this.fuse-- <= 0` (tested before the
    // decrement)
    let d = data(e);
    let fuse = d.fuse;
    d.fuse -= 1;
    if fuse <= 0 {
        dispatch::set_entity_dead(e, world);
        if !world.multiplayer_world {
            explode(e, world);
        }
    } else {
        let b = &e.base;
        world.spawn_particle("smoke", b.pos_x, b.pos_y + 0.5, b.pos_z, 0.0, 0.0, 0.0);
    }
}

/// `explode()` (private): `worldObj.createExplosion((Entity)null, posX,
/// posY, posZ, 4.0F)`, with the TNT in hand (module docs).
// src: EntityTNTPrimed.java:66-69, World.java:1518-1528
fn explode(e: &mut Entity, world: &mut World) {
    let var1: f32 = 4.0;
    let (x, y, z) = (e.base.pos_x, e.base.pos_y, e.base.pos_z);
    let mut var10 = Explosion::new(world, None, x, y, z, var1);
    var10.is_flaming = false;
    var10.do_explosion_a(world, None, Some(e));
    var10.do_explosion_b(world, true);
}

/// `EntityTNTPrimed.writeEntityToNBT(compound)`: `Fuse` as a byte.
// src: EntityTNTPrimed.java:71-73
pub fn write_entity_to_nbt(d: &TntPrimedData, c: &mut Compound) {
    n::set(c, "Fuse", Tag::Byte(d.fuse as i8));
}

/// `EntityTNTPrimed.readEntityFromNBT(compound)`: `Fuse`, sign-extended (no
/// `& 255`).
// src: EntityTNTPrimed.java:75-77
pub fn read_entity_from_nbt(d: &mut TntPrimedData, c: &Compound) -> Result<(), SaveError> {
    d.fuse = n::get_byte(c, "Fuse")? as i32;
    Ok(())
}
