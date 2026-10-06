//! `EntitySlime` (EntitySlime.java) (WP-2.3).
//!
//! | EntitySlime.java | Method | Port |
//! |---|---|---|
//! | 4-6 | `squishAmount`, `squishFactor`, `slimeJumpDelay` | [`SlimeData`] |
//! | 8-15 | constructor (size and jump delay from `rand`) | [`new_entity_slime`] |
//! | 17-20 | `entityInit` (watcher byte 16: the size) | [`new_entity_slime`] |
//! | 22-31 | `setSlimeSize`, `getSlimeSize` | [`set_slime_size`], [`get_slime_size`] |
//! | 33-41 | `writeEntityToNBT`, `readEntityFromNBT` (`Size`) | [`super::nbt`] |
//! | 43-66 | `onUpdate` (the landing squish) | [`on_update`] |
//! | 68-96 | `updatePlayerActionState` (the jump timer) | [`update_player_action_state`] |
//! | 98-112 | `setEntityDead` (the split) | [`set_entity_dead`] |
//! | 114-120 | `onCollideWithPlayer` (the attack) | [`on_collide_with_player`] |
//! | 122-132 | `getHurtSound`, `getDeathSound`; `getDropItemId` (a slimeball from the smallest) | [`super::dispatch`] |
//! | 134-137 | `getCanSpawnHere` (slime chunks below y 16) | [`get_can_spawn_here`] |
//! | 139-141 | `getSoundVolume` (0.6) | [`super::dispatch::get_sound_volume`] |
//!
//! `EntitySlime implements IMob` and extends `EntityLiving` directly (no
//! path finding).
//!
//! # The constructor's draws
//!
//! `new EntitySlime(world)` draws the size and `slimeJumpDelay` from its
//! own `rand`, which in vanilla is unseeded then. The port's `rand` is
//! seeded `mix(seed, id)` at construction (`EntityBase::new`), and the
//! oracle's `spawn` op replays both draws on that seeded stream right after
//! construction (`TickScenario.seedAfterConstruction`), so a spawned slime
//! matches. The slimes `setEntityDead` makes are a **gap**: the oracle's
//! join hook reseeds their `rand` but does not redraw `slimeJumpDelay`
//! (tools/oracle/census.md §3 lists the size only for the spawner), so a
//! split slime's first jump time is not reproducible (the size is set by
//! the split, the reseed makes everything else match).

use mc_core::math;

use super::data_watcher::WatchValue;
use super::living::{self, EntityView, lv, lv_mut};
use super::{Entity, EntityBase, EntityKindData, LivingData, dispatch};
use crate::world::World;
use mc_core::math::floor_double;

/// `Item.slimeBall.shiftedIndex` (`256 + 85`).
// src: Item.java:359
pub const SLIME_BALL: i32 = 256 + 85;

/// The fields `EntitySlime` declares (EntitySlime.java:4-6).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SlimeData {
    /// `squishAmount` (render: the squash).
    pub squish_amount: f32,
    /// `squishFactor`: last tick's `squishAmount`.
    pub squish_factor: f32,
    /// `slimeJumpDelay` (private): ground ticks until the next jump.
    pub slime_jump_delay: i32,
}

fn sd(e: &Entity) -> &SlimeData {
    match &e.kind {
        EntityKindData::Slime(_, d) => d,
        _ => panic!("EntitySlime method on another kind"),
    }
}

fn sd_mut(e: &mut Entity) -> &mut SlimeData {
    match &mut e.kind {
        EntityKindData::Slime(_, d) => d,
        _ => panic!("EntitySlime method on another kind"),
    }
}

/// `new EntitySlime(world)`: `entityInit` (watcher byte 16 = 1) in
/// `Entity`'s constructor, `EntityLiving`'s initialisers and body, then the
/// slime texture, the size `1 << rand.nextInt(3)` (1, 2 or 4), `yOffset` 0,
/// `slimeJumpDelay = rand.nextInt(20) + 10` and `setSlimeSize(size)` (module
/// docs: the draws).
// src: EntitySlime.java:8-20, Entity.java:75-113, EntityLiving.java:6-68
pub fn new_entity_slime(world: &mut World) -> Entity {
    let mut b = EntityBase::new(world);
    b.data_watcher.add_object(16, WatchValue::Byte(1));
    let mut l = LivingData::new(world, &mut b);
    l.texture = "/mob/slime.png";
    let var2 = 1 << b.rand.next_int_bounded(3);
    b.y_offset = 0.0;
    let slime_jump_delay = b.rand.next_int_bounded(20) + 10;
    let mut e = Entity {
        base: b,
        kind: EntityKindData::Slime(
            l,
            SlimeData {
                slime_jump_delay,
                ..SlimeData::default()
            },
        ),
    };
    set_slime_size(&mut e, var2);
    e
}

/// `setSlimeSize(size)`: watcher byte 16, a `0.6 * size` cube, health
/// `size²`, and `setPosition` at the current position (re-centres the box).
// src: EntitySlime.java:22-27
pub fn set_slime_size(e: &mut Entity, var1: i32) {
    e.base
        .data_watcher
        .update_object(16, WatchValue::Byte(var1 as i8));
    let s = 0.6 * var1 as f32;
    e.base.set_size(s, s);
    lv_mut(e).health = var1.wrapping_mul(var1);
    let b = &mut e.base;
    b.set_position(b.pos_x, b.pos_y, b.pos_z);
}

/// `getSlimeSize()`: watcher byte 16.
// src: EntitySlime.java:29-31
pub fn get_slime_size(e: &Entity) -> i32 {
    e.base.data_watcher.get_watchable_object_byte(16) as i32
}

/// `EntitySlime.onUpdate()`: `EntityLiving`'s; on landing, `8 × size`
/// slime particles (two `rand` draws each), the landing sound above size 2
/// (two draws) and the squash; the squash eases back by 0.6 per tick.
// src: EntitySlime.java:43-66
pub fn on_update(e: &mut Entity, world: &mut World) {
    let amount = sd(e).squish_amount;
    sd_mut(e).squish_factor = amount;
    let var1 = e.base.on_ground;
    living::on_update(e, world);
    if e.base.on_ground && !var1 {
        let var2 = get_slime_size(e);
        for _ in 0..var2.wrapping_mul(8) {
            let b = &mut e.base;
            let var4 = b.rand.next_float() * 3.1415927 * 2.0;
            let var5 = b.rand.next_float() * 0.5 + 0.5;
            let var6 = math::sin(var4) * var2 as f32 * 0.5 * var5;
            let var7 = math::cos(var4) * var2 as f32 * 0.5 * var5;
            let (x, y, z) = (
                b.pos_x + var6 as f64,
                b.bounding_box.min_y,
                b.pos_z + var7 as f64,
            );
            world.spawn_particle("slime", x, y, z, 0.0, 0.0, 0.0);
        }
        if var2 > 2 {
            let volume = dispatch::get_sound_volume(e, world);
            let pitch = ((e.base.rand.next_float() - e.base.rand.next_float()) * 0.2 + 1.0) / 0.8;
            world.play_sound_at_entity(&e.base, "mob.slime", volume, pitch);
        }
        sd_mut(e).squish_amount = -0.5;
    }
    sd_mut(e).squish_amount *= 0.6;
}

/// `EntitySlime.updatePlayerActionState()` (no `super` call): despawn
/// check, face the closest player within 16, and on the ground count
/// `slimeJumpDelay` down (post-decrement: the jump is on the tick it is
/// already 0 or less); a jump redraws it (10–29, a third of that with a
/// player near), plays the jump sound above size 1 (two draws), squashes,
/// and moves `size` forward with a random strafe; otherwise it stands
/// still on the ground.
// src: EntitySlime.java:68-96
pub fn update_player_action_state(e: &mut Entity, world: &mut World) {
    living::despawn_entity(e, world);
    let var1 = world.get_closest_player_to_entity(&e.base, 16.0);
    if let Some(p) = var1
        && let Some(v) = EntityView::of(world, p)
    {
        living::face_entity(e, world, &v, 10.0, 20.0);
    }

    let jump = e.base.on_ground && {
        let d = sd_mut(e);
        let old = d.slime_jump_delay;
        d.slime_jump_delay = old.wrapping_sub(1);
        old <= 0
    };
    if jump {
        let mut delay = e.base.rand.next_int_bounded(20) + 10;
        if var1.is_some() {
            delay /= 3;
        }
        sd_mut(e).slime_jump_delay = delay;
        lv_mut(e).is_jumping = true;
        if get_slime_size(e) > 1 {
            let volume = dispatch::get_sound_volume(e, world);
            let pitch = ((e.base.rand.next_float() - e.base.rand.next_float()) * 0.2 + 1.0) * 0.8;
            world.play_sound_at_entity(&e.base, "mob.slime", volume, pitch);
        }
        sd_mut(e).squish_amount = 1.0;
        let strafe = 1.0 - e.base.rand.next_float() * 2.0;
        let forward = get_slime_size(e) as f32;
        let l = lv_mut(e);
        l.move_strafing = strafe;
        l.move_forward = forward;
    } else {
        let on_ground = e.base.on_ground;
        let l = lv_mut(e);
        l.is_jumping = false;
        if on_ground {
            l.move_forward = 0.0;
            l.move_strafing = 0.0;
        }
    }
}

/// `EntitySlime.setEntityDead()`: a slime of size 2 or more that died with
/// health exactly 0 splits into four of half the size (2 × 2 offsets of a
/// quarter of the size, 0.5 higher, a random yaw from its own `rand`), then
/// `Entity`'s. Overkill (health below 0) or a despawn (health above 0)
/// leaves no children.
// src: EntitySlime.java:98-112
pub fn set_entity_dead(e: &mut Entity, world: &mut World) {
    let var1 = get_slime_size(e);
    if !world.multiplayer_world && var1 > 1 && lv(e).health == 0 {
        for var2 in 0..4 {
            let var3 = ((var2 % 2) as f32 - 0.5) * var1 as f32 / 4.0;
            let var4 = ((var2 / 2) as f32 - 0.5) * var1 as f32 / 4.0;
            let mut var5 = new_entity_slime(world);
            set_slime_size(&mut var5, var1 / 2);
            let yaw = e.base.rand.next_float() * 360.0;
            var5.base.set_location_and_angles(
                e.base.pos_x + var3 as f64,
                e.base.pos_y + 0.5,
                e.base.pos_z + var4 as f64,
                yaw,
                0.0,
            );
            world.spawn_entity_in_world(var5);
        }
    }
    e.base.set_entity_dead();
}

/// `EntitySlime.onCollideWithPlayer(player)`: a slime of size 2 or more
/// that sees the player and is closer than `0.6 × size` hits it for `size`
/// (`player.attackEntityFrom(this, size)`, virtual); a counted hit plays
/// the attack sound (two `rand` draws).
// src: EntitySlime.java:114-120
pub fn on_collide_with_player(e: &mut Entity, world: &mut World, var1: &mut Entity) {
    let var2 = get_slime_size(e);
    if var2 > 1 {
        let v = EntityView {
            id: var1.base.entity_id,
            pos_x: var1.base.pos_x,
            pos_y: var1.base.pos_y,
            pos_z: var1.base.pos_z,
            bounding_box: var1.base.bounding_box,
            eye_height: dispatch::get_eye_height(var1, world),
            is_living: var1.kind.living().is_some(),
            is_dead: var1.base.is_dead,
        };
        if living::can_entity_be_seen(e, world, &v)
            && (e.base.get_distance_to_entity(&var1.base) as f64) < 0.6 * var2 as f64
            && dispatch::attack_entity_from(var1, world, Some(e), var2)
        {
            let pitch = (e.base.rand.next_float() - e.base.rand.next_float()) * 0.2 + 1.0;
            world.play_sound_at_entity(&e.base, "mob.slimeattack", 1.0, pitch);
        }
    }
}

/// `EntitySlime.getDropItemId()`: a slimeball from a size-1 slime, nothing
/// from the bigger ones.
// src: EntitySlime.java:130-132
pub fn get_drop_item_id(e: &Entity) -> i32 {
    if get_slime_size(e) == 1 {
        SLIME_BALL
    } else {
        0
    }
}

/// `EntitySlime.getCanSpawnHere()`: size 1 or not peaceful, a 1-in-10
/// `rand` roll, a slime chunk (`getRandomWithSeed(987234911L).nextInt(10)
/// == 0` of the chunk at the feet, which `getChunkFromBlockCoords` loads)
/// and below y 16. The chunk is fetched before the tests, as in Java.
// src: EntitySlime.java:134-137
pub fn get_can_spawn_here(e: &mut Entity, world: &mut World) -> bool {
    let seed = world.get_random_seed();
    let mut var1 = world
        .get_chunk_from_block_coords(floor_double(e.base.pos_x), floor_double(e.base.pos_z))
        .get_random_with_seed(seed, 987234911);
    (get_slime_size(e) == 1 || world.difficulty_setting > 0)
        && e.base.rand.next_int_bounded(10) == 0
        && var1.next_int_bounded(10) == 0
        && e.base.pos_y < 16.0
}
