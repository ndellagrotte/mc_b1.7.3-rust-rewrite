//! `EntityArrow` (EntityArrow.java): an arrow in flight or stuck in a block.
//! Transcribed: the `(World)`, `(World, x, y, z)` and `(World, EntityLiving)`
//! constructors, `setArrowHeading`, `onUpdate`, the NBT bodies and
//! `onCollideWithPlayer` (the pickup, WP-2.H). Not ported:
//! `setVelocity` (only `NetClientHandler` calls it, multiplayer, D3) and
//! `getShadowSize` (render).
//!
//! The module also holds the flight helpers the other projectiles share
//! line for line ([`closest_entity_hit`], [`update_flight_rotation`],
//! [`set_heading`], [`attack_entity_by_id`]): `EntitySnowball`, `EntityEgg`,
//! `EntityFireball` and `EntityFish` repeat the same code with renamed
//! fields.
//!
//! # References to other entities
//!
//! `owner` (an `EntityLiving` field) is kept as an entity id. Where Java
//! passes it as the attacker of `attackEntityFrom`, the port passes a copy of
//! the owner taken from the world ([`attack_entity_by_id`]): no
//! `attackEntityFrom` body in b1.7.3 writes to its attacker, so the copy
//! reads exactly what the Java object would. An owner that has left
//! `loadedEntityList` (a skeleton removed 20 ticks after its death, its
//! arrow still in flight) is the removed object the world keeps while the
//! arrow names it ([`World::removed_entity`]), as Java's field still holds
//! it.

use mc_core::math::{self, jmath};
use mc_core::{Aabb, Vec3D};

use super::{Entity, EntityBase, EntityKindData, base, dispatch};
use crate::block::dispatch as block_dispatch;
use crate::save::SaveError;
use crate::save::nbt::{self as n, Compound};
use crate::world::World;
use crate::world::ray_trace::MovingObjectPosition;
use mc_nbt::Tag;

/// The fields `EntityArrow` declares (EntityArrow.java:6-16).
#[derive(Clone, Debug, PartialEq)]
pub struct ArrowData {
    /// `xTile` (private): the block it is stuck in.
    // src: EntityArrow.java:6
    pub x_tile: i32,
    /// `yTile` (private).
    // src: EntityArrow.java:7
    pub y_tile: i32,
    /// `zTile` (private).
    // src: EntityArrow.java:8
    pub z_tile: i32,
    /// `inTile` (private): the id of that block when it hit.
    // src: EntityArrow.java:9
    pub in_tile: i32,
    /// `inData` (private): its metadata when it hit.
    // src: EntityArrow.java:10
    pub in_data: i32,
    /// `inGround` (private).
    // src: EntityArrow.java:11
    pub in_ground: bool,
    /// `doesArrowBelongToPlayer`: only such arrows can be picked up.
    // src: EntityArrow.java:12
    pub does_arrow_belong_to_player: bool,
    /// `arrowShake`: 7 after hitting a block, counts down.
    // src: EntityArrow.java:13
    pub arrow_shake: i32,
    /// `owner` (an `EntityLiving`), as its entity id.
    // src: EntityArrow.java:14
    pub owner: Option<i32>,
    /// `ticksInGround` (private, no initialiser): despawns at 1200.
    // src: EntityArrow.java:15
    pub ticks_in_ground: i32,
    /// `ticksInAir` (private).
    // src: EntityArrow.java:16
    pub ticks_in_air: i32,
}

impl Default for ArrowData {
    /// The field initialisers.
    // src: EntityArrow.java:6-16
    fn default() -> Self {
        ArrowData {
            x_tile: -1,
            y_tile: -1,
            z_tile: -1,
            in_tile: 0,
            in_data: 0,
            in_ground: false,
            does_arrow_belong_to_player: false,
            arrow_shake: 0,
            owner: None,
            ticks_in_ground: 0,
            ticks_in_air: 0,
        }
    }
}

/// `Item.arrow.shiftedIndex` (`new Item(6)`, so 256 + 6).
// src: Item.java:280
const ARROW_ITEM_ID: i32 = 256 + 6;

fn data(e: &mut Entity) -> &mut ArrowData {
    match &mut e.kind {
        EntityKindData::Arrow(d) => d,
        _ => panic!("EntityArrow method on another kind"),
    }
}

/// `new EntityArrow(world)`: the `EntityList` / NBT-load constructor and the
/// harness `spawn` op's.
// src: EntityArrow.java:18-21
pub fn new_entity_arrow_empty(world: &mut World) -> Entity {
    let mut b = EntityBase::new(world);
    // entityInit() is empty (EntityArrow.java:47-48); field initialisers.
    let d = ArrowData::default();
    b.set_size(0.5, 0.5);
    Entity {
        base: b,
        kind: EntityKindData::Arrow(d),
    }
}

/// `new EntityArrow(world, x, y, z)` (the dispenser's): at `(x, y, z)`,
/// `yOffset` 0, at rest.
// src: EntityArrow.java:23-28
pub fn new_entity_arrow_at(world: &mut World, x: f64, y: f64, z: f64) -> Entity {
    let mut e = new_entity_arrow_empty(world);
    e.base.set_position(x, y, z);
    e.base.y_offset = 0.0;
    e
}

/// `new EntityArrow(world, shooter)` (skeletons, EntitySkeleton.java:39; the
/// bow, ItemBow.java:13): from the shooter's eyes, 0.16 to its right and 0.1
/// down, along its look at speed 1.5 with the heading jitter of
/// [`set_arrow_heading`] (three Gaussian draws on the arrow's `rand`).
// src: EntityArrow.java:30-45
pub fn new_entity_arrow_shot(world: &mut World, shooter: &Entity) -> Entity {
    let mut b = EntityBase::new(world);
    let mut d = ArrowData {
        owner: Some(shooter.base.entity_id),
        does_arrow_belong_to_player: shooter.kind.is_player(),
        ..ArrowData::default()
    };
    b.set_size(0.5, 0.5);
    let eye = dispatch::get_eye_height(shooter, world);
    b.set_location_and_angles(
        shooter.base.pos_x,
        shooter.base.pos_y + eye as f64,
        shooter.base.pos_z,
        shooter.base.rotation_yaw,
        shooter.base.rotation_pitch,
    );
    // src: EntityArrow.java:36-39
    b.pos_x -= (math::cos(b.rotation_yaw / 180.0 * 3.1415927) * 0.16) as f64;
    b.pos_y -= 0.10000000149011612;
    b.pos_z -= (math::sin(b.rotation_yaw / 180.0 * 3.1415927) * 0.16) as f64;
    b.set_position(b.pos_x, b.pos_y, b.pos_z);
    b.y_offset = 0.0;
    // src: EntityArrow.java:41-43
    b.motion_x = (-math::sin(b.rotation_yaw / 180.0 * 3.1415927)
        * math::cos(b.rotation_pitch / 180.0 * 3.1415927)) as f64;
    b.motion_z = (math::cos(b.rotation_yaw / 180.0 * 3.1415927)
        * math::cos(b.rotation_pitch / 180.0 * 3.1415927)) as f64;
    b.motion_y = (-math::sin(b.rotation_pitch / 180.0 * 3.1415927)) as f64;
    let (mx, my, mz) = (b.motion_x, b.motion_y, b.motion_z);
    set_heading(&mut b, mx, my, mz, 1.5, 1.0);
    d.ticks_in_ground = 0;
    Entity {
        base: b,
        kind: EntityKindData::Arrow(d),
    }
}

/// `setArrowHeading(x, y, z, speed, spread)`.
// src: EntityArrow.java:50-68
pub fn set_arrow_heading(e: &mut Entity, x: f64, y: f64, z: f64, speed: f32, spread: f32) {
    set_heading(&mut e.base, x, y, z, speed, spread);
    data(e).ticks_in_ground = 0;
}

/// The body every projectile's heading setter shares (`setArrowHeading`,
/// `setSnowballHeading`, `setEggHeading`, `EntityFish.calculateVelocity`):
/// the direction normalised, three Gaussian draws on the entity's `rand`
/// times `0.0075 * spread`, scaled by `speed`; motion and rotation (and the
/// previous rotation) follow. The caller resets its `ticksInGround`.
// src: EntityArrow.java:50-67, EntitySnowball.java:55-71, EntityEgg.java:55-71,
// EntityFish.java:85-101
pub(super) fn set_heading(
    b: &mut EntityBase,
    mut var1: f64,
    mut var3: f64,
    mut var5: f64,
    var7: f32,
    var8: f32,
) {
    let var9 = math::sqrt_double(var1 * var1 + var3 * var3 + var5 * var5);
    var1 /= var9 as f64;
    var3 /= var9 as f64;
    var5 /= var9 as f64;
    var1 += b.rand.next_gaussian() * 0.007499999832361937 * var8 as f64;
    var3 += b.rand.next_gaussian() * 0.007499999832361937 * var8 as f64;
    var5 += b.rand.next_gaussian() * 0.007499999832361937 * var8 as f64;
    var1 *= var7 as f64;
    var3 *= var7 as f64;
    var5 *= var7 as f64;
    b.motion_x = var1;
    b.motion_y = var3;
    b.motion_z = var5;
    let var10 = math::sqrt_double(var1 * var1 + var5 * var5);
    b.rotation_yaw = (jmath::atan2(var1, var5) * 180.0 / 3.1415927410125732) as f32;
    b.prev_rotation_yaw = b.rotation_yaw;
    b.rotation_pitch = (jmath::atan2(var3, var10 as f64) * 180.0 / 3.1415927410125732) as f32;
    b.prev_rotation_pitch = b.rotation_pitch;
}

/// The rotation update after a projectile moved: yaw and pitch from the
/// motion, the previous angles brought within 180 degrees, then 20 % of the
/// way from the previous angles.
// src: EntityArrow.java:196-215, EntitySnowball.java:166-185, EntityEgg.java:179-198,
// EntityFireball.java:136-155, EntityFish.java:235-254
pub(super) fn update_flight_rotation(b: &mut EntityBase) {
    let var19 = math::sqrt_double(b.motion_x * b.motion_x + b.motion_z * b.motion_z);
    b.rotation_yaw = (jmath::atan2(b.motion_x, b.motion_z) * 180.0 / 3.1415927410125732) as f32;
    b.rotation_pitch = (jmath::atan2(b.motion_y, var19 as f64) * 180.0 / 3.1415927410125732) as f32;
    while b.rotation_pitch - b.prev_rotation_pitch < -180.0 {
        b.prev_rotation_pitch -= 360.0;
    }
    while b.rotation_pitch - b.prev_rotation_pitch >= 180.0 {
        b.prev_rotation_pitch += 360.0;
    }
    while b.rotation_yaw - b.prev_rotation_yaw < -180.0 {
        b.prev_rotation_yaw -= 360.0;
    }
    while b.rotation_yaw - b.prev_rotation_yaw >= 180.0 {
        b.prev_rotation_yaw += 360.0;
    }
    b.rotation_pitch = b.prev_rotation_pitch + (b.rotation_pitch - b.prev_rotation_pitch) * 0.2;
    b.rotation_yaw = b.prev_rotation_yaw + (b.rotation_yaw - b.prev_rotation_yaw) * 0.2;
}

/// The entity half of a projectile's hit test: of the entities other than
/// `e` in its box stretched by the motion and grown by 1, those that
/// `canBeCollidedWith` (and are not `owner` until `ticks_in_air` reaches
/// `owner_ticks`) whose box grown by 0.3 the segment `start -> end` crosses,
/// the one whose crossing is nearest `start` (the first on a tie). The
/// distance test `var13 < var6 || var6 == 0.0D` is transcribed as is, so a
/// crossing exactly at `start` (distance 0) is replaced by any later one.
// src: EntityArrow.java:135-154, EntitySnowball.java:127-145, EntityEgg.java:127-145,
// EntityFireball.java:98-116, EntityFish.java:198-217
pub(super) fn closest_entity_hit(
    e: &Entity,
    world: &World,
    start: &Vec3D,
    end: &Vec3D,
    owner: Option<i32>,
    ticks_in_air: i32,
    owner_ticks: i32,
) -> Option<i32> {
    let b = &e.base;
    let bb = b
        .bounding_box
        .add_coord(b.motion_x, b.motion_y, b.motion_z)
        .expand(1.0, 1.0, 1.0);
    let var5 = world.get_entities_within_aabb_excluding_entity(Some(b.entity_id), &bb);
    let mut var4 = None;
    let mut var6 = 0.0;
    for id in var5 {
        let Some(var9) = world.entity(id) else {
            continue;
        };
        if dispatch::can_be_collided_with(var9, world)
            && (Some(id) != owner || ticks_in_air >= owner_ticks)
        {
            let var10: f32 = 0.3;
            let var11: Aabb =
                var9.base
                    .bounding_box
                    .expand(var10 as f64, var10 as f64, var10 as f64);
            if let Some(var12) = var11.calculate_intercept(start, end) {
                let var13 = start.distance_to(&var12.hit_vec);
                if var13 < var6 || var6 == 0.0 {
                    var4 = Some(id);
                    var6 = var13;
                }
            }
        }
    }
    var4
}

/// `new MovingObjectPosition(entity)` for an entity found by id.
// src: MovingObjectPosition.java:21-25
pub(super) fn entity_hit(world: &World, id: i32) -> MovingObjectPosition {
    let (x, y, z) = world
        .entity(id)
        .map(|o| (o.base.pos_x, o.base.pos_y, o.base.pos_z))
        .unwrap_or((0.0, 0.0, 0.0));
    MovingObjectPosition::entity(id, x, y, z)
}

/// `target.attackEntityFrom(attacker, damage)` where Java holds both as
/// objects and the port as ids: the target is taken out of its slot for the
/// call, the attacker is passed as a copy (module docs): of the loaded
/// entity, or of the removed one when it has left `loadedEntityList` (Java's
/// field still holds the dead object: its class, position and `isDead`
/// reach the target, e.g. `EntityPlayer.attackEntityFrom`'s difficulty
/// scaling for an `EntityMob` and `alertWolves`, EntityPlayer.java:366-390).
/// An attacker the world has neither in its slot nor removed is recorded
/// and passed as null. A target that is not in its slot is recorded and
/// counts as `false`.
// src: EntityArrow.java:163, EntityFireball.java:124, EntitySnowball.java:153,
// EntityEgg.java:153, EntityFish.java:225
pub(super) fn attack_entity_by_id(
    world: &mut World,
    target: i32,
    attacker: Option<i32>,
    damage: i32,
) -> bool {
    let attacker_copy = match attacker {
        None => None,
        Some(a) => match world.entity_object(a) {
            Some(o) => Some(o.clone()),
            None => {
                world.note_unimplemented(
                    "Projectile attackEntityFrom: attacker out of its slot or no longer kept",
                );
                None
            }
        },
    };
    match world.with_entity_mut(target, |w, t| {
        dispatch::attack_entity_from(t, w, attacker_copy.as_ref(), damage)
    }) {
        Some(r) => r,
        None => {
            world.note_unimplemented("Projectile attackEntityFrom: target out of its slot");
            false
        }
    }
}

/// `EntityArrow.onUpdate()`.
// src: EntityArrow.java:86-233
pub fn on_update(e: &mut Entity, world: &mut World) {
    // src: EntityArrow.java:87 (super.onUpdate: Entity.onUpdate)
    base::on_update(e, world);
    // src: EntityArrow.java:88-92
    {
        let b = &mut e.base;
        if b.prev_rotation_pitch == 0.0 && b.prev_rotation_yaw == 0.0 {
            let var1 = math::sqrt_double(b.motion_x * b.motion_x + b.motion_z * b.motion_z);
            b.rotation_yaw =
                (jmath::atan2(b.motion_x, b.motion_z) * 180.0 / 3.1415927410125732) as f32;
            b.prev_rotation_yaw = b.rotation_yaw;
            b.rotation_pitch =
                (jmath::atan2(b.motion_y, var1 as f64) * 180.0 / 3.1415927410125732) as f32;
            b.prev_rotation_pitch = b.rotation_pitch;
        }
    }

    // src: EntityArrow.java:94-101
    let (xt, yt, zt) = {
        let d = data(e);
        (d.x_tile, d.y_tile, d.z_tile)
    };
    let mut var15 = world.get_block_id(xt, yt, zt);
    if var15 > 0 {
        block_dispatch::set_block_bounds_based_on_state(world, var15, xt, yt, zt);
        let var2 = block_dispatch::get_collision_bounding_box_from_pool(world, var15, xt, yt, zt);
        if let Some(var2) = var2
            && var2.is_vec_inside(&Vec3D::create_vector(
                e.base.pos_x,
                e.base.pos_y,
                e.base.pos_z,
            ))
        {
            data(e).in_ground = true;
        }
    }

    // src: EntityArrow.java:103-105
    {
        let d = data(e);
        if d.arrow_shake > 0 {
            d.arrow_shake -= 1;
        }
    }

    if data(e).in_ground {
        // src: EntityArrow.java:107-123
        var15 = world.get_block_id(xt, yt, zt);
        let var18 = world.get_block_metadata(xt, yt, zt);
        let d = data(e);
        if var15 == d.in_tile && var18 == d.in_data {
            d.ticks_in_ground += 1;
            if d.ticks_in_ground == 1200 {
                dispatch::set_entity_dead(e, world);
            }
        } else {
            d.in_ground = false;
            let b = &mut e.base;
            b.motion_x *= (b.rand.next_float() * 0.2) as f64;
            b.motion_y *= (b.rand.next_float() * 0.2) as f64;
            b.motion_z *= (b.rand.next_float() * 0.2) as f64;
            let d = data(e);
            d.ticks_in_ground = 0;
            d.ticks_in_air = 0;
        }
        return;
    }

    // src: EntityArrow.java:125-133
    data(e).ticks_in_air += 1;
    let (px, py, pz, mx, my, mz) = {
        let b = &e.base;
        (
            b.pos_x, b.pos_y, b.pos_z, b.motion_x, b.motion_y, b.motion_z,
        )
    };
    let mut var16 = Vec3D::create_vector(px, py, pz);
    let var17 = Vec3D::create_vector(px + mx, py + my, pz + mz);
    let mut var3 = world.ray_trace_blocks_do_do(&mut var16, &var17, false, true);
    let var16 = Vec3D::create_vector(px, py, pz);
    let mut var17 = Vec3D::create_vector(px + mx, py + my, pz + mz);
    if let Some(h) = &var3 {
        var17 = Vec3D::create_vector(h.hit_vec.x_coord, h.hit_vec.y_coord, h.hit_vec.z_coord);
    }

    // src: EntityArrow.java:135-158
    let (owner, ticks_in_air) = {
        let d = data(e);
        (d.owner, d.ticks_in_air)
    };
    if let Some(var4) = closest_entity_hit(e, world, &var16, &var17, owner, ticks_in_air, 5) {
        var3 = Some(entity_hit(world, var4));
    }

    // src: EntityArrow.java:161-191
    if let Some(var3) = var3 {
        if let Some(hit) = var3.entity_hit {
            if attack_entity_by_id(world, hit, owner, 4) {
                let pitch = 1.2 / (e.base.rand.next_float() * 0.2 + 0.9);
                world.play_sound_at_entity(&e.base, "random.drr", 1.0, pitch);
                dispatch::set_entity_dead(e, world);
            } else {
                // src: EntityArrow.java:167-172
                let b = &mut e.base;
                b.motion_x *= -0.10000000149011612;
                b.motion_y *= -0.10000000149011612;
                b.motion_z *= -0.10000000149011612;
                b.rotation_yaw += 180.0;
                b.prev_rotation_yaw += 180.0;
                data(e).ticks_in_air = 0;
            }
        } else {
            // src: EntityArrow.java:175-189
            let (bx, by, bz) = (var3.block_x, var3.block_y, var3.block_z);
            let in_tile = world.get_block_id(bx, by, bz);
            let in_data = world.get_block_metadata(bx, by, bz);
            {
                let d = data(e);
                d.x_tile = bx;
                d.y_tile = by;
                d.z_tile = bz;
                d.in_tile = in_tile;
                d.in_data = in_data;
            }
            let b = &mut e.base;
            b.motion_x = (var3.hit_vec.x_coord - b.pos_x) as f32 as f64;
            b.motion_y = (var3.hit_vec.y_coord - b.pos_y) as f32 as f64;
            b.motion_z = (var3.hit_vec.z_coord - b.pos_z) as f32 as f64;
            let var19 = math::sqrt_double(
                b.motion_x * b.motion_x + b.motion_y * b.motion_y + b.motion_z * b.motion_z,
            );
            b.pos_x -= b.motion_x / var19 as f64 * 0.05000000074505806;
            b.pos_y -= b.motion_y / var19 as f64 * 0.05000000074505806;
            b.pos_z -= b.motion_z / var19 as f64 * 0.05000000074505806;
            let pitch = 1.2 / (b.rand.next_float() * 0.2 + 0.9);
            world.play_sound_at_entity(&e.base, "random.drr", 1.0, pitch);
            let d = data(e);
            d.in_ground = true;
            d.arrow_shake = 7;
        }
    }

    // src: EntityArrow.java:193-215
    let b = &mut e.base;
    b.pos_x += b.motion_x;
    b.pos_y += b.motion_y;
    b.pos_z += b.motion_z;
    update_flight_rotation(b);
    // src: EntityArrow.java:216-231
    let mut var20: f32 = 0.99;
    let var10: f32 = 0.03;
    if b.is_in_water() {
        for _ in 0..4 {
            let var22: f32 = 0.25;
            world.spawn_particle(
                "bubble",
                b.pos_x - b.motion_x * var22 as f64,
                b.pos_y - b.motion_y * var22 as f64,
                b.pos_z - b.motion_z * var22 as f64,
                b.motion_x,
                b.motion_y,
                b.motion_z,
            );
        }
        var20 = 0.8;
    }
    b.motion_x *= var20 as f64;
    b.motion_y *= var20 as f64;
    b.motion_z *= var20 as f64;
    b.motion_y -= var10 as f64;
    b.set_position(b.pos_x, b.pos_y, b.pos_z);
}

/// `EntityArrow.onCollideWithPlayer(player)`: an arrow a player shot
/// (`doesArrowBelongToPlayer`: any player, not only the shooter), stuck and
/// still, goes into the player's inventory as one arrow; then `random.pop`
/// with two `rand` draws, `onItemPickup(this, 1)` and death. The `&&` chain
/// tries the inventory only when the other three conditions hold.
// src: EntityArrow.java:257-266
pub fn on_collide_with_player(e: &mut Entity, world: &mut World, player: &mut Entity) {
    if !world.multiplayer_world {
        let d = data(e);
        if d.in_ground && d.does_arrow_belong_to_player && d.arrow_shake <= 0 {
            // src: EntityArrow.java:259 (new ItemStack(Item.arrow, 1))
            let mut stack = crate::item::ItemStack::new(ARROW_ITEM_ID, 1, 0);
            let taken = player
                .kind
                .player_mut()
                .expect("EntityPlayer.inventory on a non-player (Java ClassCastException)")
                .inventory
                .add_item_stack_to_inventory(&mut stack);
            if taken {
                // src: EntityArrow.java:260 (left operand drawn first)
                let pitch =
                    ((e.base.rand.next_float() - e.base.rand.next_float()) * 0.7 + 1.0) * 2.0;
                world.play_sound_at_entity(&e.base, "random.pop", 0.2, pitch);
                // src: EntityArrow.java:261-262
                super::player::on_item_pickup(player, world, e, 1);
                e.base.set_entity_dead();
            }
        }
    }
}

/// `EntityArrow.writeEntityToNBT(compound)`.
// src: EntityArrow.java:235-244
pub fn write_entity_to_nbt(d: &ArrowData, c: &mut Compound) {
    n::set(c, "xTile", Tag::Short(d.x_tile as i16));
    n::set(c, "yTile", Tag::Short(d.y_tile as i16));
    n::set(c, "zTile", Tag::Short(d.z_tile as i16));
    n::set(c, "inTile", Tag::Byte(d.in_tile as i8));
    n::set(c, "inData", Tag::Byte(d.in_data as i8));
    n::set(c, "shake", Tag::Byte(d.arrow_shake as i8));
    n::set(c, "inGround", Tag::Byte(if d.in_ground { 1 } else { 0 }));
    n::set_boolean(c, "player", d.does_arrow_belong_to_player);
}

/// `EntityArrow.readEntityFromNBT(compound)`.
// src: EntityArrow.java:246-255
pub fn read_entity_from_nbt(d: &mut ArrowData, c: &Compound) -> Result<(), SaveError> {
    d.x_tile = n::get_short(c, "xTile")? as i32;
    d.y_tile = n::get_short(c, "yTile")? as i32;
    d.z_tile = n::get_short(c, "zTile")? as i32;
    d.in_tile = n::get_byte(c, "inTile")? as i32 & 255;
    d.in_data = n::get_byte(c, "inData")? as i32 & 255;
    d.arrow_shake = n::get_byte(c, "shake")? as i32 & 255;
    d.in_ground = n::get_byte(c, "inGround")? == 1;
    d.does_arrow_belong_to_player = n::get_boolean(c, "player")?;
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
    use crate::world::events::WorldEvent;
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

    fn stuck_arrow(w: &mut World, belongs: bool, shake: i32) -> Entity {
        let mut a = new_entity_arrow_at(w, 0.5, 61.0, 0.5);
        let d = data(&mut a);
        d.in_ground = true;
        d.does_arrow_belong_to_player = belongs;
        d.arrow_shake = shake;
        a
    }

    fn pops(w: &mut World) -> Vec<f32> {
        w.events
            .drain()
            .into_iter()
            .filter_map(|e| match e {
                WorldEvent::PlaySound { name, pitch, .. } if name == "random.pop" => Some(pitch),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_players_still_arrow_is_picked_up() {
        let mut w = world();
        let mut p = new_entity_player_sp(&mut w);
        let mut a = stuck_arrow(&mut w, true, 0);
        w.events.drain();
        let mut r = a.base.rand.clone();
        on_collide_with_player(&mut a, &mut w, &mut p);
        let inv = &p.kind.player().unwrap().inventory;
        let got = inv.main_inventory[0].unwrap();
        assert_eq!((got.item_id, got.stack_size), (ARROW_ITEM_ID, 1));
        // src: InventoryPlayer.java:104 (the new slot's pick-up bob)
        assert_eq!(got.animations_to_go, 5);
        assert!(a.base.is_dead);
        let expected = ((r.next_float() - r.next_float()) * 0.7 + 1.0) * 2.0;
        assert_eq!(pops(&mut w), vec![expected]);
    }

    #[test]
    fn no_pickup_while_shaking_or_not_a_players_or_full() {
        let mut w = world();
        let mut p = new_entity_player_sp(&mut w);
        for (belongs, shake) in [(true, 1), (false, 0)] {
            let mut a = stuck_arrow(&mut w, belongs, shake);
            w.events.drain();
            on_collide_with_player(&mut a, &mut w, &mut p);
            assert!(!a.base.is_dead);
            assert!(pops(&mut w).is_empty());
        }
        let mut a = new_entity_arrow_at(&mut w, 0.5, 61.0, 0.5);
        data(&mut a).does_arrow_belong_to_player = true;
        on_collide_with_player(&mut a, &mut w, &mut p);
        assert!(!a.base.is_dead, "in flight");
        assert!(
            p.kind
                .player()
                .unwrap()
                .inventory
                .main_inventory
                .iter()
                .all(Option::is_none)
        );
        for s in p
            .kind
            .player_mut()
            .unwrap()
            .inventory
            .main_inventory
            .iter_mut()
        {
            *s = Some(ItemStack::new(1, 64, 0));
        }
        let mut a = stuck_arrow(&mut w, true, 0);
        let rand = a.base.rand.clone();
        w.events.drain();
        on_collide_with_player(&mut a, &mut w, &mut p);
        assert!(!a.base.is_dead, "inventory full");
        assert_eq!(a.base.rand, rand);
        assert!(pops(&mut w).is_empty());
    }
}
