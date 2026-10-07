//! `EntitySnowball` (EntitySnowball.java), and the thrown-projectile body it
//! shares with `EntityEgg` ([`super::egg`]): the two classes are the same
//! code with renamed fields (`xTileSnowball` / `xTile`, …), except that an
//! egg may hatch chickens where it lands (EntityEgg.java:156-167). One data
//! struct, [`ThrowableData`], serves both; [`throwable_on_update`] is the
//! shared `onUpdate` with the egg's extra step as a hook.
//!
//! Transcribed: the `(World)`, `(World, EntityLiving)` and `(World, x, y,
//! z)` constructors, `setSnowballHeading`, `onUpdate`, the NBT bodies,
//! `onCollideWithPlayer` (the pickup, WP-2.H). Not ported: `setVelocity`
//! (`NetClientHandler` only, multiplayer, D3), `isInRangeToRenderDist`,
//! `getShadowSize` (render).
//!
//! Vanilla quirks, transcribed: a snowball or egg never sets its in-ground
//! flag in flight (any hit kills it), so the in-ground branch and the pickup
//! are only reachable from NBT; and the pickup would give an *arrow*
//! (EntitySnowball.java:223, EntityEgg.java:236). The pickup can never fire
//! in vanilla: `thrower` is only set by the `(World, EntityLiving)`
//! constructor (EntitySnowball.java:32, EntityEgg.java:32) and is not in the
//! NBT (EntitySnowball.java:204-220, EntityEgg.java:217-233), so the
//! NBT-loaded projectile whose in-ground flag is set has a null thrower,
//! which no player equals. It is transcribed anyway; no fixture can reach it.

use mc_core::Vec3D;
use mc_core::math;
use mc_nbt::Tag;

use super::arrow::{
    attack_entity_by_id, closest_entity_hit, entity_hit, set_heading, update_flight_rotation,
};
use super::{Entity, EntityBase, EntityKindData, base, dispatch};
use crate::save::SaveError;
use crate::save::nbt::{self as n, Compound};
use crate::world::World;

/// The fields `EntitySnowball` and `EntityEgg` declare (EntitySnowball.java:6-14,
/// EntityEgg.java:6-14; the egg's names in brackets).
#[derive(Clone, Debug, PartialEq)]
pub struct ThrowableData {
    /// `xTileSnowball` (`xTile`), private.
    pub x_tile: i32,
    /// `yTileSnowball` (`yTile`), private.
    pub y_tile: i32,
    /// `zTileSnowball` (`zTile`), private.
    pub z_tile: i32,
    /// `inTileSnowball` (`inTile`), private.
    pub in_tile: i32,
    /// `inGroundSnowball` (`inGround`), private.
    pub in_ground: bool,
    /// `shakeSnowball` (`shake`).
    pub shake: i32,
    /// `thrower` (an `EntityLiving`, private), as its entity id.
    pub thrower: Option<i32>,
    /// `ticksInGroundSnowball` (`ticksInGround`), private, no initialiser.
    pub ticks_in_ground: i32,
    /// `ticksInAirSnowball` (`ticksinAir`), private.
    pub ticks_in_air: i32,
}

impl Default for ThrowableData {
    /// The field initialisers.
    // src: EntitySnowball.java:6-14, EntityEgg.java:6-14
    fn default() -> Self {
        ThrowableData {
            x_tile: -1,
            y_tile: -1,
            z_tile: -1,
            in_tile: 0,
            in_ground: false,
            shake: 0,
            thrower: None,
            ticks_in_ground: 0,
            ticks_in_air: 0,
        }
    }
}

/// The `EntitySnowball` level.
pub type SnowballData = ThrowableData;

/// `Item.arrow.shiftedIndex` (the pickup quirk, module docs).
// src: Item.java:280
const ARROW_ITEM_ID: i32 = 256 + 6;

/// The snowball's or egg's data.
pub(super) fn thrown(e: &mut Entity) -> &mut ThrowableData {
    match &mut e.kind {
        EntityKindData::Snowball(d) | EntityKindData::Egg(d) => d,
        _ => panic!("EntitySnowball/EntityEgg method on another kind"),
    }
}

/// `new EntitySnowball(world)` / `new EntityEgg(world)` (the `kind`
/// chooses).
// src: EntitySnowball.java:16-19, EntityEgg.java:16-19
pub(super) fn new_thrown_empty(world: &mut World, egg: bool) -> Entity {
    let mut b = EntityBase::new(world);
    // entityInit() is empty (EntitySnowball.java:21-22); field initialisers.
    let d = ThrowableData::default();
    b.set_size(0.25, 0.25);
    Entity {
        base: b,
        kind: if egg {
            EntityKindData::Egg(d)
        } else {
            EntityKindData::Snowball(d)
        },
    }
}

/// `new EntitySnowball(world, thrower)` / `new EntityEgg(world, thrower)`:
/// from the thrower's eyes, along its look at speed 1.5 with the heading
/// jitter (three Gaussian draws on the projectile's `rand`).
// src: EntitySnowball.java:30-45, EntityEgg.java:30-45
pub(super) fn new_thrown_by(world: &mut World, thrower: &Entity, egg: bool) -> Entity {
    let mut e = new_thrown_empty(world, egg);
    thrown(&mut e).thrower = Some(thrower.base.entity_id);
    let eye = dispatch::get_eye_height(thrower, world);
    let b = &mut e.base;
    b.set_location_and_angles(
        thrower.base.pos_x,
        thrower.base.pos_y + eye as f64,
        thrower.base.pos_z,
        thrower.base.rotation_yaw,
        thrower.base.rotation_pitch,
    );
    // src: EntitySnowball.java:35-39
    b.pos_x -= (math::cos(b.rotation_yaw / 180.0 * 3.1415927) * 0.16) as f64;
    b.pos_y -= 0.10000000149011612;
    b.pos_z -= (math::sin(b.rotation_yaw / 180.0 * 3.1415927) * 0.16) as f64;
    b.set_position(b.pos_x, b.pos_y, b.pos_z);
    b.y_offset = 0.0;
    // src: EntitySnowball.java:40-44
    let var3: f32 = 0.4;
    b.motion_x = (-math::sin(b.rotation_yaw / 180.0 * 3.1415927)
        * math::cos(b.rotation_pitch / 180.0 * 3.1415927)
        * var3) as f64;
    b.motion_z = (math::cos(b.rotation_yaw / 180.0 * 3.1415927)
        * math::cos(b.rotation_pitch / 180.0 * 3.1415927)
        * var3) as f64;
    b.motion_y = (-math::sin(b.rotation_pitch / 180.0 * 3.1415927) * var3) as f64;
    let (mx, my, mz) = (b.motion_x, b.motion_y, b.motion_z);
    set_thrown_heading(&mut e, mx, my, mz, 1.5, 1.0);
    e
}

/// `new EntitySnowball(world, x, y, z)` / `new EntityEgg(world, x, y, z)`
/// (the dispenser's).
// src: EntitySnowball.java:47-53, EntityEgg.java:47-53
pub(super) fn new_thrown_at(world: &mut World, x: f64, y: f64, z: f64, egg: bool) -> Entity {
    let mut e = new_thrown_empty(world, egg);
    thrown(&mut e).ticks_in_ground = 0;
    e.base.set_size(0.25, 0.25);
    e.base.set_position(x, y, z);
    e.base.y_offset = 0.0;
    e
}

/// `setSnowballHeading` / `setEggHeading(x, y, z, speed, spread)`.
// src: EntitySnowball.java:55-73, EntityEgg.java:55-73
pub(super) fn set_thrown_heading(e: &mut Entity, x: f64, y: f64, z: f64, speed: f32, spread: f32) {
    set_heading(&mut e.base, x, y, z, speed, spread);
    thrown(e).ticks_in_ground = 0;
}

/// `new EntitySnowball(world)`: the `EntityList` / NBT-load constructor.
// src: EntitySnowball.java:16-19
pub fn new_entity_snowball_empty(world: &mut World) -> Entity {
    new_thrown_empty(world, false)
}

/// `new EntitySnowball(world, thrower)` (ItemSnowball.java:13).
// src: EntitySnowball.java:30-45
pub fn new_entity_snowball_thrown(world: &mut World, thrower: &Entity) -> Entity {
    new_thrown_by(world, thrower, false)
}

/// `new EntitySnowball(world, x, y, z)` (BlockDispenser.java:118).
// src: EntitySnowball.java:47-53
pub fn new_entity_snowball_at(world: &mut World, x: f64, y: f64, z: f64) -> Entity {
    new_thrown_at(world, x, y, z, false)
}

/// `setSnowballHeading(x, y, z, speed, spread)`.
// src: EntitySnowball.java:55-73
pub fn set_snowball_heading(e: &mut Entity, x: f64, y: f64, z: f64, speed: f32, spread: f32) {
    set_thrown_heading(e, x, y, z, speed, spread);
}

/// `EntitySnowball.onUpdate()`: [`throwable_on_update`] with nothing extra
/// at the impact.
// src: EntitySnowball.java:87-202
pub fn on_update(e: &mut Entity, world: &mut World) {
    throwable_on_update(e, world, |_, _| {});
}

/// The shared `onUpdate` of `EntitySnowball` and `EntityEgg`; `on_impact`
/// runs where the egg's chicken step sits, after the hit entity was attacked
/// and before the 8 `snowballpoof` particles (EntityEgg.java:156-167).
// src: EntitySnowball.java:87-202, EntityEgg.java:87-215
pub(super) fn throwable_on_update(
    e: &mut Entity,
    world: &mut World,
    on_impact: impl FnOnce(&mut Entity, &mut World),
) {
    // src: EntitySnowball.java:88-91
    {
        let b = &mut e.base;
        b.last_tick_pos_x = b.pos_x;
        b.last_tick_pos_y = b.pos_y;
        b.last_tick_pos_z = b.pos_z;
    }
    base::on_update(e, world);
    // src: EntitySnowball.java:92-94
    {
        let d = thrown(e);
        if d.shake > 0 {
            d.shake -= 1;
        }
    }
    // src: EntitySnowball.java:96-115
    if thrown(e).in_ground {
        let (xt, yt, zt) = {
            let d = thrown(e);
            (d.x_tile, d.y_tile, d.z_tile)
        };
        let var1 = world.get_block_id(xt, yt, zt);
        let d = thrown(e);
        if var1 == d.in_tile {
            d.ticks_in_ground += 1;
            if d.ticks_in_ground == 1200 {
                dispatch::set_entity_dead(e, world);
            }
            return;
        }
        d.in_ground = false;
        let b = &mut e.base;
        b.motion_x *= (b.rand.next_float() * 0.2) as f64;
        b.motion_y *= (b.rand.next_float() * 0.2) as f64;
        b.motion_z *= (b.rand.next_float() * 0.2) as f64;
        let d = thrown(e);
        d.ticks_in_ground = 0;
        d.ticks_in_air = 0;
    } else {
        thrown(e).ticks_in_air += 1;
    }

    // src: EntitySnowball.java:117-124
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

    // src: EntitySnowball.java:126-150
    let (thrower, ticks_in_air) = {
        let d = thrown(e);
        (d.thrower, d.ticks_in_air)
    };
    if !world.multiplayer_world
        && let Some(var4) = closest_entity_hit(e, world, &var15, &var2, thrower, ticks_in_air, 5)
    {
        var3 = Some(entity_hit(world, var4));
    }

    // src: EntitySnowball.java:152-161
    if let Some(var3) = var3 {
        if let Some(hit) = var3.entity_hit {
            // `if (… && attackEntityFrom(thrower, 0)) {}`: the result is unused.
            attack_entity_by_id(world, hit, thrower, 0);
        }
        on_impact(e, world);
        for _ in 0..8 {
            world.spawn_particle(
                "snowballpoof",
                e.base.pos_x,
                e.base.pos_y,
                e.base.pos_z,
                0.0,
                0.0,
                0.0,
            );
        }
        dispatch::set_entity_dead(e, world);
    }

    // src: EntitySnowball.java:163-201
    let b = &mut e.base;
    b.pos_x += b.motion_x;
    b.pos_y += b.motion_y;
    b.pos_z += b.motion_z;
    update_flight_rotation(b);
    let mut var18: f32 = 0.99;
    let var19: f32 = 0.03;
    if b.is_in_water() {
        for _ in 0..4 {
            let var20: f32 = 0.25;
            world.spawn_particle(
                "bubble",
                b.pos_x - b.motion_x * var20 as f64,
                b.pos_y - b.motion_y * var20 as f64,
                b.pos_z - b.motion_z * var20 as f64,
                b.motion_x,
                b.motion_y,
                b.motion_z,
            );
        }
        var18 = 0.8;
    }
    b.motion_x *= var18 as f64;
    b.motion_y *= var18 as f64;
    b.motion_z *= var18 as f64;
    b.motion_y -= var19 as f64;
    b.set_position(b.pos_x, b.pos_y, b.pos_z);
}

/// `EntitySnowball.onCollideWithPlayer(player)` / `EntityEgg`'s: the
/// thrower picks up a stuck, still projectile as an arrow (module docs);
/// then `random.pop` with two `rand` draws, `onItemPickup(this, 1)` and
/// death. Unlike `EntityArrow`'s there is no `multiplayerWorld` test, and
/// `thrower == player` is an identity test (here: the entity id). The `&&`
/// chain tries the inventory only when the other three conditions hold.
// src: EntitySnowball.java:222-229, EntityEgg.java:235-242
pub fn on_collide_with_player(e: &mut Entity, world: &mut World, player: &mut Entity) {
    let pid = player.base.entity_id;
    let d = thrown(e);
    if d.in_ground && d.thrower == Some(pid) && d.shake <= 0 {
        // vanilla bug: an arrow, not a snowball or an egg (copied from EntityArrow).
        // src: EntitySnowball.java:223, EntityEgg.java:236
        let mut stack = crate::item::ItemStack::new(ARROW_ITEM_ID, 1, 0);
        let taken = player
            .kind
            .player_mut()
            .expect("EntityPlayer.inventory on a non-player (Java ClassCastException)")
            .inventory
            .add_item_stack_to_inventory(&mut stack);
        if taken {
            // src: EntitySnowball.java:224, EntityEgg.java:237 (left operand drawn first)
            let pitch = ((e.base.rand.next_float() - e.base.rand.next_float()) * 0.7 + 1.0) * 2.0;
            world.play_sound_at_entity(&e.base, "random.pop", 0.2, pitch);
            // src: EntitySnowball.java:225-226, EntityEgg.java:238-239
            super::player::on_item_pickup(player, world, e, 1);
            e.base.set_entity_dead();
        }
    }
}

/// `writeEntityToNBT(compound)` of `EntitySnowball` and `EntityEgg`.
// src: EntitySnowball.java:204-211, EntityEgg.java:217-224
pub fn write_entity_to_nbt(d: &ThrowableData, c: &mut Compound) {
    n::set(c, "xTile", Tag::Short(d.x_tile as i16));
    n::set(c, "yTile", Tag::Short(d.y_tile as i16));
    n::set(c, "zTile", Tag::Short(d.z_tile as i16));
    n::set(c, "inTile", Tag::Byte(d.in_tile as i8));
    n::set(c, "shake", Tag::Byte(d.shake as i8));
    n::set(c, "inGround", Tag::Byte(if d.in_ground { 1 } else { 0 }));
}

/// `readEntityFromNBT(compound)` of `EntitySnowball` and `EntityEgg`.
// src: EntitySnowball.java:213-220, EntityEgg.java:226-233
pub fn read_entity_from_nbt(d: &mut ThrowableData, c: &Compound) -> Result<(), SaveError> {
    d.x_tile = n::get_short(c, "xTile")? as i32;
    d.y_tile = n::get_short(c, "yTile")? as i32;
    d.z_tile = n::get_short(c, "zTile")? as i32;
    d.in_tile = n::get_byte(c, "inTile")? as i32 & 255;
    d.shake = n::get_byte(c, "shake")? as i32 & 255;
    d.in_ground = n::get_byte(c, "inGround")? == 1;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::player::new_entity_player_sp;
    use crate::item::ItemStack;
    use crate::save::NoSave;
    use crate::world::WorldEnvironment;
    use crate::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
    use crate::world::provider::{Dimension, WorldProvider};

    fn world() -> World {
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
        w
    }

    /// The thrower picks a stuck snowball or egg up as an arrow (vanilla
    /// bug); another player, or a projectile still shaking, is ignored.
    #[test]
    fn the_thrower_picks_up_an_arrow() {
        for egg in [false, true] {
            let mut w = world();
            let mut p = new_entity_player_sp(&mut w);
            let mut s = new_thrown_at(&mut w, 0.5, 61.0, 0.5, egg);
            {
                let d = thrown(&mut s);
                d.in_ground = true;
                d.shake = 0;
                d.thrower = Some(p.base.entity_id + 1);
            }
            on_collide_with_player(&mut s, &mut w, &mut p);
            assert!(!s.base.is_dead, "not the thrower");
            {
                let d = thrown(&mut s);
                d.thrower = Some(p.base.entity_id);
                d.shake = 1;
            }
            on_collide_with_player(&mut s, &mut w, &mut p);
            assert!(!s.base.is_dead, "shaking");
            thrown(&mut s).shake = 0;
            on_collide_with_player(&mut s, &mut w, &mut p);
            assert!(s.base.is_dead);
            let got = p.kind.player().unwrap().inventory.main_inventory[0];
            assert_eq!(
                got.map(|s: ItemStack| (s.item_id, s.stack_size)),
                Some((ARROW_ITEM_ID, 1))
            );
        }
    }
}
