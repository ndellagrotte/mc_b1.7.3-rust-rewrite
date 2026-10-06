//! `EntityFish` (EntityFish.java): the fishing bobber. Cast by a player with
//! a fishing rod (`ItemFishingRod.onItemRightClick`, WP-2.6), it flies like
//! a thrown projectile, floats on water, and now and then a fish bites
//! (`ticksCatchable`); the rod's next use calls [`catch_fish`].
//!
//! Transcribed: the `(World)` and `(World, EntityPlayer)` constructors,
//! `calculateVelocity`, `onUpdate`, `catchFish`, the NBT bodies. Not ported:
//! the `(World, x, y, z)` constructor, `setVelocity` and
//! `setPositionAndRotation2` (all `NetClientHandler`, multiplayer, D3; the
//! interpolation branch of `onUpdate` that `setPositionAndRotation2` feeds is
//! transcribed but unreachable in single player), `isInRangeToRenderDist`,
//! `getShadowSize` (render).
//!
//! `angler` (an `EntityPlayer`) and `bobber` (the entity it hooked) are
//! entity ids; the angler's `fishEntity` is [`super::PlayerData::fish_entity`].
//!
//! Vanilla quirk, transcribed: a block hit sets `inGround` but never
//! `xTile`/`yTile`/`zTile`/`inTile` (EntityFish.java:228-230), so the in-ground
//! test reads block (-1, -1, -1), which is air below the world, against
//! `inTile` 0: once in the ground the bobber stays there until it despawns
//! (1200 ticks) or the angler stops holding the rod.
//!
//! Gaps: `catchFish`'s `angler.addStat(StatList.fishCaughtStat, 1)` (WP-2.7,
//! recorded as `"EntityPlayer.addStat"`).

use mc_core::Vec3D;
use mc_core::math::{self, floor_double};
use mc_nbt::Tag;

use super::arrow::{
    attack_entity_by_id, closest_entity_hit, entity_hit, set_heading, update_flight_rotation,
};
use super::item::new_entity_item;
use super::{Entity, EntityBase, EntityKindData, base, dispatch};
use crate::block::Material;
use crate::item::ItemStack;
use crate::save::SaveError;
use crate::save::nbt::{self as n, Compound};
use crate::world::World;
use mc_core::Aabb;

/// The fields `EntityFish` declares (EntityFish.java:6-25).
#[derive(Clone, Debug, PartialEq)]
pub struct FishData {
    /// `xTile` (private): never set from a hit (module docs).
    pub x_tile: i32,
    /// `yTile` (private).
    pub y_tile: i32,
    /// `zTile` (private).
    pub z_tile: i32,
    /// `inTile` (private).
    pub in_tile: i32,
    /// `inGround` (private).
    pub in_ground: bool,
    /// `shake`.
    pub shake: i32,
    /// `angler` (an `EntityPlayer`), as its entity id.
    pub angler: Option<i32>,
    /// `ticksInGround` (private): despawns at 1200.
    pub ticks_in_ground: i32,
    /// `ticksInAir` (private): the angler can be hit from 5 on.
    pub ticks_in_air: i32,
    /// `ticksCatchable` (private): while positive a fish is on the hook.
    pub ticks_catchable: i32,
    /// `bobber`: the entity it hooked, as its entity id.
    pub bobber: Option<i32>,
    /// `fishPosRotationIncrements` (private, multiplayer interpolation).
    pub fish_pos_rotation_increments: i32,
    /// `fishX` (private, multiplayer interpolation).
    pub fish_x: f64,
    /// `fishY` (private).
    pub fish_y: f64,
    /// `fishZ` (private).
    pub fish_z: f64,
    /// `fishYaw` (private).
    pub fish_yaw: f64,
    /// `fishPitch` (private).
    pub fish_pitch: f64,
    /// `velocityX` (private, multiplayer).
    pub velocity_x: f64,
    /// `velocityY` (private).
    pub velocity_y: f64,
    /// `velocityZ` (private).
    pub velocity_z: f64,
}

impl Default for FishData {
    /// What both constructors set (EntityFish.java:29-37, :50-58); the
    /// fields without an assignment are 0.
    fn default() -> Self {
        FishData {
            x_tile: -1,
            y_tile: -1,
            z_tile: -1,
            in_tile: 0,
            in_ground: false,
            shake: 0,
            angler: None,
            ticks_in_ground: 0,
            ticks_in_air: 0,
            ticks_catchable: 0,
            bobber: None,
            fish_pos_rotation_increments: 0,
            fish_x: 0.0,
            fish_y: 0.0,
            fish_z: 0.0,
            fish_yaw: 0.0,
            fish_pitch: 0.0,
            velocity_x: 0.0,
            velocity_y: 0.0,
            velocity_z: 0.0,
        }
    }
}

/// `Item.fishingRod.shiftedIndex` (`new ItemFishingRod(90)`).
// src: Item.java:364
const FISHING_ROD_ID: i32 = 256 + 90;

/// `Item.fishRaw.shiftedIndex` (`new ItemFood(93, …)`).
// src: Item.java:367
const FISH_RAW_ID: i32 = 256 + 93;

fn data(e: &mut Entity) -> &mut FishData {
    match &mut e.kind {
        EntityKindData::Fish(d) => d,
        _ => panic!("EntityFish method on another kind"),
    }
}

/// `new EntityFish(world)`: no angler (`NetClientHandler`'s; an update
/// without an angler throws in Java).
// src: EntityFish.java:27-40
pub fn new_entity_fish_empty(world: &mut World) -> Entity {
    let mut b = EntityBase::new(world);
    // entityInit() is empty (EntityFish.java:76-77).
    let d = FishData::default();
    b.set_size(0.25, 0.25);
    b.ignore_frustum_check = true;
    Entity {
        base: b,
        kind: EntityKindData::Fish(d),
    }
}

/// `new EntityFish(world, angler)` (ItemFishingRod.java:26): sets
/// `angler.fishEntity`, starts at the angler's eyes (posY + 1.62 - yOffset),
/// 0.16 to the right and 0.1 down, and flies along the angler's look at
/// speed 0.4 x 1.5 with the heading jitter of `calculateVelocity` (three
/// Gaussian draws on the bobber's `rand`).
// src: EntityFish.java:48-74
pub fn new_entity_fish(world: &mut World, angler: &mut Entity) -> Entity {
    let mut b = EntityBase::new(world);
    let mut d = FishData::default();
    b.ignore_frustum_check = true;
    d.angler = Some(angler.base.entity_id);
    if let Some(p) = angler.kind.player_mut() {
        // src: EntityFish.java:61
        p.fish_entity = Some(b.entity_id);
    }
    b.set_size(0.25, 0.25);
    let a = &angler.base;
    b.set_location_and_angles(
        a.pos_x,
        a.pos_y + 1.62 - a.y_offset as f64,
        a.pos_z,
        a.rotation_yaw,
        a.rotation_pitch,
    );
    // src: EntityFish.java:64-68
    b.pos_x -= (math::cos(b.rotation_yaw / 180.0 * 3.1415927) * 0.16) as f64;
    b.pos_y -= 0.10000000149011612;
    b.pos_z -= (math::sin(b.rotation_yaw / 180.0 * 3.1415927) * 0.16) as f64;
    b.set_position(b.pos_x, b.pos_y, b.pos_z);
    b.y_offset = 0.0;
    // src: EntityFish.java:69-73
    let var3: f32 = 0.4;
    b.motion_x = (-math::sin(b.rotation_yaw / 180.0 * 3.1415927)
        * math::cos(b.rotation_pitch / 180.0 * 3.1415927)
        * var3) as f64;
    b.motion_z = (math::cos(b.rotation_yaw / 180.0 * 3.1415927)
        * math::cos(b.rotation_pitch / 180.0 * 3.1415927)
        * var3) as f64;
    b.motion_y = (-math::sin(b.rotation_pitch / 180.0 * 3.1415927) * var3) as f64;
    let mut e = Entity {
        base: b,
        kind: EntityKindData::Fish(d),
    };
    let (mx, my, mz) = (e.base.motion_x, e.base.motion_y, e.base.motion_z);
    calculate_velocity(&mut e, mx, my, mz, 1.5, 1.0);
    e
}

/// `calculateVelocity(x, y, z, speed, spread)`: the projectile heading
/// ([`set_heading`]).
// src: EntityFish.java:85-103
pub fn calculate_velocity(e: &mut Entity, x: f64, y: f64, z: f64, speed: f32, spread: f32) {
    set_heading(&mut e.base, x, y, z, speed, spread);
    data(e).ticks_in_ground = 0;
}

/// What `onUpdate` reads of the angler: `isDead`, `isEntityAlive()`,
/// whether the current item is a fishing rod, and the squared distance.
fn angler_state(e: &Entity, world: &World, aid: i32) -> Option<(bool, bool, bool, f64)> {
    let a = world.entity(aid)?;
    let holds_rod = a
        .kind
        .player()
        .and_then(|p| p.inventory.get_current_item())
        .is_some_and(|s| s.item_id == FISHING_ROD_ID);
    Some((
        a.base.is_dead,
        dispatch::is_entity_alive(a),
        holds_rod,
        e.base.get_distance_sq_to_entity(&a.base),
    ))
}

/// `angler.fishEntity = null`.
fn clear_angler_fish_entity(world: &mut World, aid: i32) {
    if let Some(p) = world.entity_mut(aid).and_then(|a| a.kind.player_mut()) {
        p.fish_entity = None;
    }
}

/// `EntityFish.onUpdate()`.
// src: EntityFish.java:123-322
pub fn on_update(e: &mut Entity, world: &mut World) {
    base::on_update(e, world);
    if data(e).fish_pos_rotation_increments > 0 {
        // src: EntityFish.java:125-142 (multiplayer interpolation)
        let d = data(e).clone();
        let b = &mut e.base;
        let inc = d.fish_pos_rotation_increments as f64;
        let var21 = b.pos_x + (d.fish_x - b.pos_x) / inc;
        let var22 = b.pos_y + (d.fish_y - b.pos_y) / inc;
        let var23 = b.pos_z + (d.fish_z - b.pos_z) / inc;
        let mut var7 = d.fish_yaw - b.rotation_yaw as f64;
        while var7 < -180.0 {
            var7 += 360.0;
        }
        while var7 >= 180.0 {
            var7 -= 360.0;
        }
        b.rotation_yaw = (b.rotation_yaw as f64 + var7 / inc) as f32;
        b.rotation_pitch =
            (b.rotation_pitch as f64 + (d.fish_pitch - b.rotation_pitch as f64) / inc) as f32;
        data(e).fish_pos_rotation_increments -= 1;
        let b = &mut e.base;
        b.set_position(var21, var22, var23);
        let (yaw, pitch) = (b.rotation_yaw, b.rotation_pitch);
        b.set_rotation(yaw, pitch);
        return;
    }

    if !world.multiplayer_world {
        // src: EntityFish.java:145-150
        let Some(aid) = data(e).angler else {
            world.note_unimplemented(
                "EntityFish.onUpdate without an angler (Java NullPointerException)",
            );
            return;
        };
        let gone = match angler_state(e, world, aid) {
            Some((dead, alive, holds_rod, dist)) => dead || !alive || !holds_rod || dist > 1024.0,
            None => {
                if world.entity_index(aid).is_some() {
                    world.note_unimplemented("EntityFish.onUpdate: angler out of its slot");
                    return;
                }
                // Not in the list: it left by dying (Java's object is dead).
                true
            }
        };
        if gone {
            dispatch::set_entity_dead(e, world);
            clear_angler_fish_entity(world, aid);
            return;
        }
        // src: EntityFish.java:152-161
        if let Some(bob) = data(e).bobber {
            if !world.is_entity_dead_or_gone(bob) {
                if let Some(o) = world.entity(bob) {
                    let (x, y, z) = (
                        o.base.pos_x,
                        o.base.bounding_box.min_y + o.base.height as f64 * 0.8,
                        o.base.pos_z,
                    );
                    let b = &mut e.base;
                    b.pos_x = x;
                    b.pos_y = y;
                    b.pos_z = z;
                } else {
                    world.note_unimplemented("EntityFish.onUpdate: bobber out of its slot");
                }
                return;
            }
            data(e).bobber = None;
        }
    }

    // src: EntityFish.java:164-187
    {
        let d = data(e);
        if d.shake > 0 {
            d.shake -= 1;
        }
    }
    if data(e).in_ground {
        let (xt, yt, zt) = {
            let d = data(e);
            (d.x_tile, d.y_tile, d.z_tile)
        };
        let var19 = world.get_block_id(xt, yt, zt);
        let d = data(e);
        if var19 == d.in_tile {
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
        let d = data(e);
        d.ticks_in_ground = 0;
        d.ticks_in_air = 0;
    } else {
        data(e).ticks_in_air += 1;
    }

    // src: EntityFish.java:189-196
    let (px, py, pz, mx, my, mz) = {
        let b = &e.base;
        (
            b.pos_x, b.pos_y, b.pos_z, b.motion_x, b.motion_y, b.motion_z,
        )
    };
    let mut var20 = Vec3D::create_vector(px, py, pz);
    let var2 = Vec3D::create_vector(px + mx, py + my, pz + mz);
    let mut var3 = world.ray_trace_blocks(&mut var20, &var2);
    let var20 = Vec3D::create_vector(px, py, pz);
    let mut var2 = Vec3D::create_vector(px + mx, py + my, pz + mz);
    if let Some(h) = &var3 {
        var2 = Vec3D::create_vector(h.hit_vec.x_coord, h.hit_vec.y_coord, h.hit_vec.z_coord);
    }

    // src: EntityFish.java:198-221
    let (angler, ticks_in_air) = {
        let d = data(e);
        (d.angler, d.ticks_in_air)
    };
    if let Some(var4) = closest_entity_hit(e, world, &var20, &var2, angler, ticks_in_air, 5) {
        var3 = Some(entity_hit(world, var4));
    }

    // src: EntityFish.java:223-231
    if let Some(var3) = var3 {
        if let Some(hit) = var3.entity_hit {
            if attack_entity_by_id(world, hit, angler, 0) {
                data(e).bobber = Some(hit);
            }
        } else {
            // vanilla bug: xTile/yTile/zTile/inTile are not set (module docs).
            data(e).in_ground = true;
        }
    }

    if data(e).in_ground {
        return;
    }
    // src: EntityFish.java:234-254
    let (mx, my, mz) = (e.base.motion_x, e.base.motion_y, e.base.motion_z);
    dispatch::move_entity(e, world, mx, my, mz);
    update_flight_rotation(&mut e.base);
    // src: EntityFish.java:255-270
    let mut var25: f32 = 0.92;
    if e.base.on_ground || e.base.is_collided_horizontally {
        var25 = 0.5;
    }
    let var26: i8 = 5;
    let mut var27 = 0.0;
    // `(double)(var28 + 0)`: the `+ 0` is in the Java.
    for var28 in 0..var26 {
        let bb = e.base.bounding_box;
        let var14 = bb.min_y + (bb.max_y - bb.min_y) * var28 as f64 / var26 as f64 - 0.125 + 0.125;
        let var16 =
            bb.min_y + (bb.max_y - bb.min_y) * (var28 + 1) as f64 / var26 as f64 - 0.125 + 0.125;
        let var18 = Aabb::new(bb.min_x, var14, bb.min_z, bb.max_x, var16, bb.max_z);
        if world.is_aabb_in_material(&var18, Material::Water) {
            var27 += 1.0 / var26 as f64;
        }
    }

    // src: EntityFish.java:272-303
    if var27 > 0.0 {
        if data(e).ticks_catchable > 0 {
            data(e).ticks_catchable -= 1;
        } else {
            let mut var29: i16 = 500;
            if world.can_block_be_rained_on(
                floor_double(e.base.pos_x),
                floor_double(e.base.pos_y) + 1,
                floor_double(e.base.pos_z),
            ) {
                var29 = 300;
            }
            if e.base.rand.next_int_bounded(var29 as i32) == 0 {
                let tc = e.base.rand.next_int_bounded(30) + 10;
                data(e).ticks_catchable = tc;
                e.base.motion_y -= 0.20000000298023224;
                let pitch = 1.0 + (e.base.rand.next_float() - e.base.rand.next_float()) * 0.4;
                world.play_sound_at_entity(&e.base, "random.splash", 0.25, pitch);
                let b = &mut e.base;
                let var30 = floor_double(b.bounding_box.min_y) as f32;
                let mut var15 = 0;
                while (var15 as f32) < 1.0 + b.width * 20.0 {
                    let var31 = (b.rand.next_float() * 2.0 - 1.0) * b.width;
                    let var17 = (b.rand.next_float() * 2.0 - 1.0) * b.width;
                    let vy = b.motion_y - (b.rand.next_float() * 0.2) as f64;
                    world.spawn_particle(
                        "bubble",
                        b.pos_x + var31 as f64,
                        (var30 + 1.0) as f64,
                        b.pos_z + var17 as f64,
                        b.motion_x,
                        vy,
                        b.motion_z,
                    );
                    var15 += 1;
                }
                var15 = 0;
                while (var15 as f32) < 1.0 + b.width * 20.0 {
                    let var31 = (b.rand.next_float() * 2.0 - 1.0) * b.width;
                    let var17 = (b.rand.next_float() * 2.0 - 1.0) * b.width;
                    world.spawn_particle(
                        "splash",
                        b.pos_x + var31 as f64,
                        (var30 + 1.0) as f64,
                        b.pos_z + var17 as f64,
                        b.motion_x,
                        b.motion_y,
                        b.motion_z,
                    );
                    var15 += 1;
                }
            }
        }
    }

    // src: EntityFish.java:305-319
    if data(e).ticks_catchable > 0 {
        let b = &mut e.base;
        b.motion_y -=
            (b.rand.next_float() * b.rand.next_float() * b.rand.next_float()) as f64 * 0.2;
    }
    let var13 = var27 * 2.0 - 1.0;
    let b = &mut e.base;
    b.motion_y += 0.03999999910593033 * var13;
    if var27 > 0.0 {
        var25 = (var25 as f64 * 0.9) as f32;
        b.motion_y *= 0.8;
    }
    b.motion_x *= var25 as f64;
    b.motion_y *= var25 as f64;
    b.motion_z *= var25 as f64;
    b.set_position(b.pos_x, b.pos_y, b.pos_z);
}

/// `catchFish()`: reel in. A hooked entity is pulled toward the angler
/// (result 3); otherwise, with a fish on (`ticksCatchable`), a raw fish item
/// flies to the angler (result 1); a bobber in the ground gives 2 (the
/// rod's damage, `ItemFishingRod.onItemRightClick`, WP-2.6). The bobber dies
/// and the angler's `fishEntity` is cleared.
///
/// `angler` is the player Java reaches through the `angler` field; the
/// caller (the rod's right click) holds it out of its slot.
// src: EntityFish.java:346-383
pub fn catch_fish(e: &mut Entity, world: &mut World, angler: &mut Entity) -> i32 {
    let mut var1: i8 = 0;
    let (bobber, ticks_catchable) = {
        let d = data(e);
        (d.bobber, d.ticks_catchable)
    };
    let (ax, ay, az) = (angler.base.pos_x, angler.base.pos_y, angler.base.pos_z);
    if let Some(bob) = bobber {
        // src: EntityFish.java:349-360
        let var2 = ax - e.base.pos_x;
        let var4 = ay - e.base.pos_y;
        let var6 = az - e.base.pos_z;
        let var8 = math::sqrt_double(var2 * var2 + var4 * var4 + var6 * var6) as f64;
        let var10 = 0.1;
        match world.entity_mut(bob) {
            Some(o) => {
                o.base.motion_x += var2 * var10;
                o.base.motion_y += var4 * var10 + math::sqrt_double(var8) as f64 * 0.08;
                o.base.motion_z += var6 * var10;
            }
            None => world.note_unimplemented(
                "EntityFish.catchFish: bobber no longer in loadedEntityList or out of its slot",
            ),
        }
        var1 = 3;
    } else if ticks_catchable > 0 {
        // src: EntityFish.java:362-373
        let mut var13 = new_entity_item(
            world,
            e.base.pos_x,
            e.base.pos_y,
            e.base.pos_z,
            ItemStack::new(FISH_RAW_ID, 1, 0),
        );
        let var3 = ax - e.base.pos_x;
        let var5 = ay - e.base.pos_y;
        let var7 = az - e.base.pos_z;
        let var9 = math::sqrt_double(var3 * var3 + var5 * var5 + var7 * var7) as f64;
        let var11 = 0.1;
        var13.base.motion_x = var3 * var11;
        var13.base.motion_y = var5 * var11 + math::sqrt_double(var9) as f64 * 0.08;
        var13.base.motion_z = var7 * var11;
        world.spawn_entity_in_world(var13);
        // src: EntityFish.java:372: angler.addStat(StatList.fishCaughtStat, 1) (WP-2.7)
        world.note_unimplemented("EntityPlayer.addStat");
        var1 = 1;
    }
    if data(e).in_ground {
        var1 = 2;
    }
    dispatch::set_entity_dead(e, world);
    if let Some(p) = angler.kind.player_mut() {
        p.fish_entity = None;
    }
    var1 as i32
}

/// `EntityFish.writeEntityToNBT(compound)` (unreachable from a chunk save:
/// `EntityList` has no name for the class).
// src: EntityFish.java:324-331
pub fn write_entity_to_nbt(d: &FishData, c: &mut Compound) {
    n::set(c, "xTile", Tag::Short(d.x_tile as i16));
    n::set(c, "yTile", Tag::Short(d.y_tile as i16));
    n::set(c, "zTile", Tag::Short(d.z_tile as i16));
    n::set(c, "inTile", Tag::Byte(d.in_tile as i8));
    n::set(c, "shake", Tag::Byte(d.shake as i8));
    n::set(c, "inGround", Tag::Byte(if d.in_ground { 1 } else { 0 }));
}

/// `EntityFish.readEntityFromNBT(compound)`.
// src: EntityFish.java:333-340
pub fn read_entity_from_nbt(d: &mut FishData, c: &Compound) -> Result<(), SaveError> {
    d.x_tile = n::get_short(c, "xTile")? as i32;
    d.y_tile = n::get_short(c, "yTile")? as i32;
    d.z_tile = n::get_short(c, "zTile")? as i32;
    d.in_tile = n::get_byte(c, "inTile")? as i32 & 255;
    d.shake = n::get_byte(c, "shake")? as i32 & 255;
    d.in_ground = n::get_byte(c, "inGround")? == 1;
    Ok(())
}
