//! `EntityWolf` (EntityWolf.java).
//!
//! | EntityWolf.java | Method | Port |
//! |---|---|---|
//! | 7-13 | fields | [`WolfData`] |
//! | 15-21 | constructor | [`new_entity_wolf`] |
//! | 23-28 | `entityInit` (watcher byte 16: sitting/angry/tamed bits, string 17: the owner, int 18: health) | [`new_entity_wolf`] |
//! | 30-32 | `canTriggerWalking` (false) | [`super::dispatch::can_trigger_walking`] |
//! | 34-40 | `getEntityTexture` | [`get_entity_texture`] |
//! | 42-64 | `writeEntityToNBT`, `readEntityFromNBT` (`Angry`, `Sitting`, `Owner`) | [`super::nbt`] |
//! | 66-68 | `canDespawn` | [`can_despawn`] |
//! | 70-94 | `getLivingSound`, `getHurtSound`, `getDeathSound`, `getSoundVolume` (0.4), `getDropItemId` (-1) | [`get_living_sound`]; [`super::dispatch`] |
//! | 96-123 | `updatePlayerActionState` | [`update_player_action_state`] |
//! | 125-150 | `onLivingUpdate` | [`on_living_update`] |
//! | 152-196 | `onUpdate` (head tilt, shaking) | [`on_update`] |
//! | 198-219 | `getWolfShaking`, `getShadingWhileShaking`, `getShakeAngle`, `getInterestedAngle` (render) | the functions of the same names |
//! | 221-227 | `getEyeHeight`, `getVerticalFaceSpeed` | [`get_eye_height`], [`get_vertical_face_speed`] |
//! | 229-248 | `getPathOrWalkableBlock` (private) | [`get_path_or_walkable_block`] |
//! | 250-252 | `isMovementCeased` | [`is_movement_ceased`] |
//! | 254-298 | `attackEntityFrom` | [`attack_entity_from`] |
//! | 300-302 | `findPlayerToAttack` | [`find_player_to_attack`] |
//! | 304-324 | `attackEntity` | [`attack_entity`] |
//! | 326-378 | `interact` (bones, meat, sitting) | [`interact`] |
//! | 380-393 | `showHeartsOrSmokeFX` | [`show_hearts_or_smoke_fx`] |
//! | 395-408 | `handleHealthUpdate` | [`handle_health_update`] |
//! | 410-416 | `setTailRotation` (render) | [`set_tail_rotation`] |
//! | 418-420 | `getMaxSpawnedInChunk` (8) | [`super::dispatch::get_max_spawned_in_chunk`] |
//! | 422-470 | `getWolfOwner`, `setWolfOwner`, `isWolfSitting`, `setWolfSitting`, `isWolfAngry`, `setWolfAngry`, `isWolfTamed`, `setWolfTamed` | the functions of the same names |
//!
//! b1.7.3 wolves do hunt sheep: an untamed wolf with no target and no path
//! picks a random sheep within 16 blocks on a 1-in-100 roll of the world's
//! `rand` each AI tick (:108-113).
//!
//! # A null username
//!
//! `EntityPlayer.username` is null until `EntityPlayerSP` copies the
//! session's (EntityPlayerSP.java:20); the port's player has none yet
//! (`PlayerData::username`, WP-2.5) and neither has the oracle's
//! `HarnessPlayer`. Taming then stores `null` through
//! `DataWatcher.updateObject` (`var2.equals(...)`, DataWatcher.java:45) and
//! the owner checks call `username.equalsIgnoreCase(...)` (:289, :366): each
//! throws `NullPointerException` in vanilla, and panics here at the same
//! point (after the state changes Java made before it).
//!
//! # Other entities
//!
//! The attacker of [`attack_entity_from`] is in hand (`&Entity`); the other
//! wolves it alerts and the target of [`attack_entity`] are reached through
//! the world by id. A wolf that is out of its slot (the one updating, when
//! it is not the wolf being hit) cannot be reached: recorded.

use mc_core::Aabb;
use mc_core::math::{self, floor_double};

use super::creature::{self, CreatureData};
use super::data_watcher::WatchValue;
use super::living::{self, EntityView, lv, lv_mut};
use super::{AnimalData, Entity, EntityBase, EntityKindData, LivingData, base, dispatch};
use crate::item;
use crate::world::World;

/// `Item.bone.shiftedIndex` (`256 + 96`).
// src: Item.java:370
pub const BONE: i32 = 256 + 96;
/// `Item.porkRaw.shiftedIndex`: `heal(((ItemFood) Item.porkRaw).getHealAmount())`
/// heals by the raw porkchop's amount whatever meat was fed.
// src: Item.java:337, EntityWolf.java:361
const PORK_RAW: i32 = super::pig::PORK_RAW;

/// The fields `EntityWolf` declares (EntityWolf.java:7-13), all private,
/// all render state except `looksWithInterest` (which also keeps the
/// player as the look target) and `field_25052_g` (which stops movement).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WolfData {
    /// `looksWithInterest` (`false`): the current target is a player holding
    /// a bone (untamed) or favourite meat (tamed).
    pub looks_with_interest: bool,
    /// `headRotationCourse`: eases toward 1 while interested, else 0.
    pub head_rotation_course: f32,
    /// `headRotationCourseOld`.
    pub head_rotation_course_old: f32,
    /// `isWolfShaking`: got wet; shakes once dry, on the ground and still.
    pub is_wolf_shaking: bool,
    /// `field_25052_g`: the shake animation is running (docs/mappings.md):
    /// it holds the wolf still (`isMovementCeased`) for 40 ticks.
    pub shake_running: bool,
    /// `timeWolfIsShaking`: the shake progress, +0.05 a tick.
    pub time_wolf_is_shaking: f32,
    /// `prevTimeWolfIsShaking`.
    pub prev_time_wolf_is_shaking: f32,
}

fn wd(e: &Entity) -> &WolfData {
    match &e.kind {
        EntityKindData::Wolf(.., d) => d,
        _ => panic!("EntityWolf method on a non-wolf"),
    }
}

fn wd_mut(e: &mut Entity) -> &mut WolfData {
    match &mut e.kind {
        EntityKindData::Wolf(.., d) => d,
        _ => panic!("EntityWolf method on a non-wolf"),
    }
}

fn cr(e: &Entity) -> &CreatureData {
    creature::creature(&e.kind).expect("EntityCreature method on a non-creature")
}

fn cr_mut(e: &mut Entity) -> &mut CreatureData {
    creature::creature_mut(&mut e.kind).expect("EntityCreature method on a non-creature")
}

/// `new EntityWolf(world)`: `Entity(World)` with `EntityWolf.entityInit`
/// (byte 16 = 0, string 17 = `""`, int 18 = `this.health`, which is still 0:
/// `entityInit` runs inside `Entity`'s constructor, before `EntityLiving`'s
/// field initialisers set it to 10), the `EntityLiving` initialisers and
/// body (four `Math.random()` draws), then the texture, a 0.8 × 0.8 box
/// (`setSize` does not move it), `moveSpeed` 1.1 and 8 health.
// src: EntityWolf.java:15-28, Entity.java:75-113, EntityLiving.java:6-68
pub fn new_entity_wolf(world: &mut World) -> Entity {
    let mut b = EntityBase::new(world);
    b.data_watcher.add_object(16, WatchValue::Byte(0));
    b.data_watcher
        .add_object(17, WatchValue::String(String::new()));
    b.data_watcher.add_object(18, WatchValue::Int(0));
    let mut l = LivingData::new(world, &mut b);
    l.texture = "/mob/wolf.png";
    b.set_size(0.8, 0.8);
    l.move_speed = 1.1;
    l.health = 8;
    Entity {
        base: b,
        kind: EntityKindData::Wolf(
            l,
            CreatureData::default(),
            AnimalData::default(),
            WolfData::default(),
        ),
    }
}

// ---- data watcher -----------------------------------------------------------------

/// `getWolfOwner()`: watcher string 17 (`""` when untamed).
// src: EntityWolf.java:422-424, DataWatcher.java:39-41
pub fn get_wolf_owner(e: &Entity) -> &str {
    match e
        .base
        .data_watcher
        .watched_objects
        .get(&17)
        .map(|o| &o.watched_object)
    {
        Some(WatchValue::String(s)) => s,
        other => panic!("DataWatcher 17: expected String, found {other:?}"),
    }
}

/// `setWolfOwner(name)`; `None` is Java's `null`, on which
/// `DataWatcher.updateObject` throws (module docs).
// src: EntityWolf.java:426-428, DataWatcher.java:43-51
pub fn set_wolf_owner(e: &mut Entity, var1: Option<&str>) {
    let Some(name) = var1 else {
        panic!(
            "EntityWolf.setWolfOwner(null): DataWatcher.updateObject throws NullPointerException \
             (a player with a null username, module docs)"
        );
    };
    e.base
        .data_watcher
        .update_object(17, WatchValue::String(name.to_string()));
}

fn flags(e: &Entity) -> i32 {
    e.base.data_watcher.get_watchable_object_byte(16) as i32
}

fn set_flag(e: &mut Entity, bit: i32, on: bool) {
    let var2 = flags(e);
    let v = if on { var2 | bit } else { var2 & !bit };
    e.base
        .data_watcher
        .update_object(16, WatchValue::Byte(v as i8));
}

/// `isWolfSitting()`: bit 0 of watcher byte 16.
// src: EntityWolf.java:430-432
pub fn is_wolf_sitting(e: &Entity) -> bool {
    (flags(e) & 1) != 0
}

/// `setWolfSitting(sitting)` (`var2 | 1` / `var2 & -2`).
// src: EntityWolf.java:434-442
pub fn set_wolf_sitting(e: &mut Entity, var1: bool) {
    set_flag(e, 1, var1);
}

/// `isWolfAngry()`: bit 1 of watcher byte 16.
// src: EntityWolf.java:444-446
pub fn is_wolf_angry(e: &Entity) -> bool {
    (flags(e) & 2) != 0
}

/// `setWolfAngry(angry)` (`var2 | 2` / `var2 & -3`).
// src: EntityWolf.java:448-456
pub fn set_wolf_angry(e: &mut Entity, var1: bool) {
    set_flag(e, 2, var1);
}

/// `isWolfTamed()`: bit 2 of watcher byte 16.
// src: EntityWolf.java:458-460
pub fn is_wolf_tamed(e: &Entity) -> bool {
    (flags(e) & 4) != 0
}

/// `setWolfTamed(tamed)` (`var2 | 4` / `var2 & -5`).
// src: EntityWolf.java:462-470
pub fn set_wolf_tamed(e: &mut Entity, var1: bool) {
    set_flag(e, 4, var1);
}

/// Watcher int 18: the health as of the last AI tick (the tail angle and
/// the whine read it).
// src: EntityWolf.java:27, :120
fn watched_health(e: &Entity) -> i32 {
    e.base.data_watcher.get_watchable_object_int(18)
}

/// Java's `String.equalsIgnoreCase`: same length in UTF-16 units, and each
/// pair of units equal, or equal after `Character.toUpperCase`, or after
/// `toUpperCase` then `toLowerCase` (one-to-one char mappings; a char whose
/// Rust case mapping is not a single char maps to itself, as Java's
/// `Character` methods do for those).
// src: String.equalsIgnoreCase / regionMatches(true, ...) (JDK)
fn java_equals_ignore_case(a: &str, b: &str) -> bool {
    fn one(c: char, upper: bool) -> char {
        let mut m: Vec<char> = if upper {
            c.to_uppercase().collect()
        } else {
            c.to_lowercase().collect()
        };
        if m.len() == 1 { m.remove(0) } else { c }
    }
    let a: Vec<u16> = a.encode_utf16().collect();
    let b: Vec<u16> = b.encode_utf16().collect();
    a.len() == b.len()
        && a.iter().zip(&b).all(|(&x, &y)| {
            if x == y {
                return true;
            }
            match (char::from_u32(x as u32), char::from_u32(y as u32)) {
                (Some(cx), Some(cy)) => {
                    let ux = one(cx, true);
                    let uy = one(cy, true);
                    ux == uy || one(ux, false) == one(uy, false)
                }
                _ => false,
            }
        })
}

/// `player.username.equalsIgnoreCase(getWolfOwner())`; panics where Java
/// throws on a null username (module docs).
fn is_owner(e: &Entity, username: Option<&str>) -> bool {
    match username {
        Some(u) => java_equals_ignore_case(u, get_wolf_owner(e)),
        None => panic!(
            "EntityWolf: null.equalsIgnoreCase(owner) throws NullPointerException (a player with a \
             null username, module docs)"
        ),
    }
}

// ---- overrides ----------------------------------------------------------------------

/// `EntityWolf.getEntityTexture()`.
// src: EntityWolf.java:34-40
pub fn get_entity_texture(e: &Entity) -> &'static str {
    if is_wolf_tamed(e) {
        "/mob/wolf_tame.png"
    } else if is_wolf_angry(e) {
        "/mob/wolf_angry.png"
    } else {
        lv(e).texture
    }
}

/// `EntityWolf.canDespawn()`: only untamed wolves despawn.
// src: EntityWolf.java:66-68
pub fn can_despawn(e: &Entity) -> bool {
    !is_wolf_tamed(e)
}

/// `EntityWolf.getLivingSound()`: a growl when angry; otherwise one
/// `rand.nextInt(3)`: on 0 a whine (tamed, watched health below 10) or
/// panting, else a bark.
// src: EntityWolf.java:70-78
pub fn get_living_sound(e: &mut Entity) -> &'static str {
    if is_wolf_angry(e) {
        "mob.wolf.growl"
    } else if e.base.rand.next_int_bounded(3) == 0 {
        if is_wolf_tamed(e) && watched_health(e) < 10 {
            "mob.wolf.whine"
        } else {
            "mob.wolf.panting"
        }
    } else {
        "mob.wolf.bark"
    }
}

/// `EntityWolf.updatePlayerActionState()`: `EntityCreature`'s, then a tamed
/// wolf with nothing to do follows its owner beyond 5 blocks
/// ([`get_path_or_walkable_block`]) or sits down when the owner is not in
/// the world (and it is not in water); an untamed wolf with no target and
/// no path picks a random sheep within 16 × 4 × 16 on a 1-in-100 roll of the
/// world's `rand` (a second draw picks the sheep). Water makes it stand up;
/// watcher 18 takes the health.
// src: EntityWolf.java:96-123
pub fn update_player_action_state(e: &mut Entity, world: &mut World) {
    creature::update_player_action_state(e, world);
    let has_attacked = cr(e).has_attacked;
    if !has_attacked && !creature::has_path(e) && is_wolf_tamed(e) && e.base.riding_entity.is_none()
    {
        let owner = get_wolf_owner(e).to_string();
        let var3 = world.get_player_entity_by_name(&owner);
        if let Some(pid) = var3 {
            match EntityView::of(world, pid) {
                Some(v) => {
                    // var3.getDistanceToEntity(this)
                    let var2 = v.distance_to(&e.base);
                    if var2 > 5.0 {
                        get_path_or_walkable_block(e, world, pid, var2);
                    }
                }
                None => world.note_unimplemented(
                    "EntityWolf.updatePlayerActionState: the owner is not in its slot",
                ),
            }
        } else if !dispatch::is_in_water(e, world) {
            set_wolf_sitting(e, true);
        }
    } else if cr(e).player_to_attack.is_none()
        && !creature::has_path(e)
        && !is_wolf_tamed(e)
        && world.rand.next_int_bounded(100) == 0
    {
        let (x, y, z) = (e.base.pos_x, e.base.pos_y, e.base.pos_z);
        let bb = Aabb::new(x, y, z, x + 1.0, y + 1.0, z + 1.0).expand(16.0, 4.0, 16.0);
        let var1 = world.get_entities_within_aabb("EntitySheep", &bb, Some(&*e));
        if !var1.is_empty() {
            let i = world.rand.next_int_bounded(var1.len() as i32);
            // setTarget(entity): EntityCreature.java:178-180
            cr_mut(e).player_to_attack = Some(var1[i as usize]);
        }
    }

    if dispatch::is_in_water(e, world) {
        set_wolf_sitting(e, false);
    }

    if !world.multiplayer_world {
        let h = lv(e).health;
        e.base.data_watcher.update_object(18, WatchValue::Int(h));
    }
}

/// `EntityWolf.onLivingUpdate()`: `EntityLiving`'s, then
/// `looksWithInterest` (the look target, with no path and not angry, is a
/// player holding a bone while untamed or favourite meat while tamed), then
/// the start of the shake animation for a wolf that got wet and is now
/// still on the ground (`setEntityState(8)`: empty in `World`).
// src: EntityWolf.java:125-150
pub fn on_living_update(e: &mut Entity, world: &mut World) {
    living::on_living_update(e, world);
    wd_mut(e).looks_with_interest = false;
    if let Some(var1) = lv(e).current_target
        && !creature::has_path(e)
        && !is_wolf_angry(e)
    {
        let var3 = world
            .entity(var1)
            .and_then(|t| t.kind.player())
            .and_then(|p| p.inventory.get_current_item())
            .map(|s| s.item_id);
        if let Some(id) = var3 {
            if !is_wolf_tamed(e) && id == BONE {
                wd_mut(e).looks_with_interest = true;
            } else if is_wolf_tamed(e)
                && let Some(fav) = item::item(id).and_then(|d| d.get_is_wolfs_favorite_meat())
            {
                wd_mut(e).looks_with_interest = fav;
            }
        }
        if world.entity(var1).is_none() && world.entity_index(var1).is_some() {
            world.note_unimplemented(
                "EntityWolf.onLivingUpdate: the look target is out of its slot",
            );
        }
    }

    let start = {
        let w = wd(e);
        !lv(e).is_multiplayer_entity
            && w.is_wolf_shaking
            && !w.shake_running
            && !creature::has_path(e)
            && e.base.on_ground
    };
    if start {
        let w = wd_mut(e);
        w.shake_running = true;
        w.time_wolf_is_shaking = 0.0;
        w.prev_time_wolf_is_shaking = 0.0;
        world.set_entity_state(e.base.entity_id, 8);
    }
}

/// `EntityWolf.onUpdate()`: `EntityLiving`'s, then the head tilt (eases 40 %
/// a tick toward 1 while interested, else 0; interest keeps the look target
/// 10 more ticks), then shaking: wet resets it; while the animation runs, a
/// shake sound on its first tick (two `rand` draws), +0.05 a tick, over
/// after `prev >= 2` (40 ticks), and past 0.4 up to 7 splash particles a
/// tick (two `rand` draws each).
// src: EntityWolf.java:152-196
pub fn on_update(e: &mut Entity, world: &mut World) {
    living::on_update(e, world);
    {
        let w = wd_mut(e);
        w.head_rotation_course_old = w.head_rotation_course;
        if w.looks_with_interest {
            w.head_rotation_course += (1.0 - w.head_rotation_course) * 0.4;
        } else {
            w.head_rotation_course += (0.0 - w.head_rotation_course) * 0.4;
        }
    }
    if wd(e).looks_with_interest {
        lv_mut(e).num_ticks_to_chase_target = 10;
    }

    if base::is_wet(e, world) {
        let w = wd_mut(e);
        w.is_wolf_shaking = true;
        w.shake_running = false;
        w.time_wolf_is_shaking = 0.0;
        w.prev_time_wolf_is_shaking = 0.0;
    } else if (wd(e).is_wolf_shaking || wd(e).shake_running) && wd(e).shake_running {
        if wd(e).time_wolf_is_shaking == 0.0 {
            let volume = dispatch::get_sound_volume(e, world);
            let pitch = (e.base.rand.next_float() - e.base.rand.next_float()) * 0.2 + 1.0;
            world.play_sound_at_entity(&e.base, "mob.wolf.shake", volume, pitch);
        }
        {
            let w = wd_mut(e);
            w.prev_time_wolf_is_shaking = w.time_wolf_is_shaking;
            w.time_wolf_is_shaking += 0.05;
            if w.prev_time_wolf_is_shaking >= 2.0 {
                w.is_wolf_shaking = false;
                w.shake_running = false;
                w.prev_time_wolf_is_shaking = 0.0;
                w.time_wolf_is_shaking = 0.0;
            }
        }
        let t = wd(e).time_wolf_is_shaking;
        if t > 0.4 {
            let var1 = e.base.bounding_box.min_y as f32;
            let var2 = (math::sin((t - 0.4) * 3.1415927) * 7.0) as i32;
            for _ in 0..var2 {
                let b = &mut e.base;
                let var4 = (b.rand.next_float() * 2.0 - 1.0) * b.width * 0.5;
                let var5 = (b.rand.next_float() * 2.0 - 1.0) * b.width * 0.5;
                world.spawn_particle(
                    "splash",
                    b.pos_x + var4 as f64,
                    (var1 + 0.8) as f64,
                    b.pos_z + var5 as f64,
                    b.motion_x,
                    b.motion_y,
                    b.motion_z,
                );
            }
        }
    }
}

/// `getWolfShaking()` (render).
// src: EntityWolf.java:198-200
pub fn get_wolf_shaking(e: &Entity) -> bool {
    wd(e).is_wolf_shaking
}

/// `getShadingWhileShaking(partialTick)` (render).
// src: EntityWolf.java:202-204
pub fn get_shading_while_shaking(e: &Entity, var1: f32) -> f32 {
    let w = wd(e);
    0.75 + (w.prev_time_wolf_is_shaking
        + (w.time_wolf_is_shaking - w.prev_time_wolf_is_shaking) * var1)
        / 2.0
        * 0.25
}

/// `getShakeAngle(partialTick, offset)` (render).
// src: EntityWolf.java:206-215
#[allow(clippy::manual_clamp)] // Java's if / else if, kept (NaN behaviour)
pub fn get_shake_angle(e: &Entity, var1: f32, var2: f32) -> f32 {
    let w = wd(e);
    let mut var3 = (w.prev_time_wolf_is_shaking
        + (w.time_wolf_is_shaking - w.prev_time_wolf_is_shaking) * var1
        + var2)
        / 1.8;
    if var3 < 0.0 {
        var3 = 0.0;
    } else if var3 > 1.0 {
        var3 = 1.0;
    }
    math::sin(var3 * 3.1415927) * math::sin(var3 * 3.1415927 * 11.0) * 0.15 * 3.1415927
}

/// `getInterestedAngle(partialTick)` (render).
// src: EntityWolf.java:217-219
pub fn get_interested_angle(e: &Entity, var1: f32) -> f32 {
    let w = wd(e);
    (w.head_rotation_course_old + (w.head_rotation_course - w.head_rotation_course_old) * var1)
        * 0.15
        * 3.1415927
}

/// `EntityWolf.getEyeHeight()`.
// src: EntityWolf.java:221-223
pub fn get_eye_height(e: &Entity) -> f32 {
    e.base.height * 0.8
}

/// `EntityWolf.getVerticalFaceSpeed()`: 20 while sitting, else
/// `EntityLiving`'s 40.
// src: EntityWolf.java:225-227, EntityLiving.java:728-730
pub fn get_vertical_face_speed(e: &Entity) -> i32 {
    if is_wolf_sitting(e) { 20 } else { 40 }
}

/// `getPathOrWalkableBlock(target, distance)` (private in Java; `target` by
/// id, public for the tests): a path to the
/// target (range 16); with none beyond 12 blocks, a teleport to the first
/// free cell (solid below, two clear above) on the rim of the 5 × 5 square
/// around the target's feet (x outer, z inner, the inner 3 × 3 skipped).
// src: EntityWolf.java:229-248
pub fn get_path_or_walkable_block(e: &mut Entity, world: &mut World, target: i32, var2: f32) {
    let Some(tb) = world.entity(target).map(|t| t.base.clone()) else {
        world
            .note_unimplemented("EntityWolf.getPathOrWalkableBlock: the target is not in its slot");
        return;
    };
    let var3 = world.get_path_to_entity(&e.base, &tb, 16.0);
    if var3.is_none() && var2 > 12.0 {
        let var4 = floor_double(tb.pos_x) - 2;
        let var5 = floor_double(tb.pos_z) - 2;
        let var6 = floor_double(tb.bounding_box.min_y);
        for var7 in 0..=4 {
            for var8 in 0..=4 {
                if (var7 < 1 || var8 < 1 || var7 > 3 || var8 > 3)
                    && world.is_block_normal_cube(var4 + var7, var6 - 1, var5 + var8)
                    && !world.is_block_normal_cube(var4 + var7, var6, var5 + var8)
                    && !world.is_block_normal_cube(var4 + var7, var6 + 1, var5 + var8)
                {
                    let (yaw, pitch) = (e.base.rotation_yaw, e.base.rotation_pitch);
                    e.base.set_location_and_angles(
                        ((var4 + var7) as f32 + 0.5) as f64,
                        var6 as f64,
                        ((var5 + var8) as f32 + 0.5) as f64,
                        yaw,
                        pitch,
                    );
                    return;
                }
            }
        }
    } else {
        cr_mut(e).path_to_entity = var3;
    }
}

/// `EntityWolf.isMovementCeased()`: sitting, or the shake animation running.
// src: EntityWolf.java:250-252
pub fn is_movement_ceased(e: &Entity) -> bool {
    is_wolf_sitting(e) || wd(e).shake_running
}

/// What `attackEntityFrom` reads off its attacker (`var1`), which may be
/// replaced by an arrow's owner (looked up by id).
struct Attacker {
    id: i32,
    is_player: bool,
    is_living: bool,
    is_arrow: bool,
    arrow_owner: Option<i32>,
    username: Option<String>,
}

impl Attacker {
    fn of(a: &Entity) -> Attacker {
        Attacker {
            id: a.base.entity_id,
            is_player: a.kind.is_player(),
            is_living: a.kind.living().is_some(),
            is_arrow: a.kind.arrow().is_some(),
            arrow_owner: a.kind.arrow().and_then(|d| d.owner),
            username: a.kind.player().and_then(|p| p.username.clone()),
        }
    }
}

/// `EntityWolf.attackEntityFrom(attacker, damage)`.
///
/// Stands up; damage from an attacker other than a player or an arrow is
/// halved (rounded up); then `EntityLiving`'s. On a hit, an untamed calm
/// wolf hit by a player gets angry at it; with a living attacker (an
/// arrow's owner for an arrow) every untamed wolf with no target within
/// 16 × 4 × 16 — this one included — targets it, and gets angry if it is a
/// player. A tamed or angry wolf targets any attacker other than itself,
/// except its owner.
///
/// vanilla bug (transcribed): that last branch takes `var1` before the
/// arrow-to-owner swap of the first branch, so an angry or tamed wolf hit by
/// an arrow would target the arrow itself (unreachable in b1.7.3, where
/// `EntityArrow` passes its owner as the attacker, EntityArrow.java:163).
// src: EntityWolf.java:254-298
pub fn attack_entity_from(
    e: &mut Entity,
    world: &mut World,
    var1: Option<&Entity>,
    mut var2: i32,
) -> bool {
    set_wolf_sitting(e, false);
    if let Some(a) = var1
        && !a.kind.is_player()
        && a.kind.arrow().is_none()
    {
        var2 = var2.wrapping_add(1) / 2;
    }

    if !living::attack_entity_from(e, world, var1, var2) {
        return false;
    }
    let mut att = var1.map(Attacker::of);
    if !is_wolf_tamed(e) && !is_wolf_angry(e) {
        if let Some(a) = &att
            && a.is_player
        {
            set_wolf_angry(e, true);
            cr_mut(e).player_to_attack = Some(a.id);
        }

        if let Some(a) = &att
            && a.is_arrow
            && let Some(owner) = a.arrow_owner
        {
            att = match world.entity(owner) {
                Some(o) => Some(Attacker::of(o)),
                None => {
                    world.note_unimplemented(
                        "EntityWolf.attackEntityFrom: the arrow's owner is not in its slot",
                    );
                    None
                }
            };
        }

        if let Some(a) = att.filter(|a| a.is_living) {
            let (x, y, z) = (e.base.pos_x, e.base.pos_y, e.base.pos_z);
            let bb = Aabb::new(x, y, z, x + 1.0, y + 1.0, z + 1.0).expand(16.0, 4.0, 16.0);
            let var3 = world.get_entities_within_aabb("EntityWolf", &bb, Some(&*e));
            let alert = |var6: &mut Entity| {
                if !is_wolf_tamed(var6) && cr(var6).player_to_attack.is_none() {
                    cr_mut(var6).player_to_attack = Some(a.id);
                    if a.is_player {
                        set_wolf_angry(var6, true);
                    }
                }
            };
            for id in var3 {
                if id == e.base.entity_id {
                    alert(e);
                } else if world.with_entity_mut(id, |_, var6| alert(var6)).is_none() {
                    world.note_unimplemented(
                        "EntityWolf.attackEntityFrom: a wolf in range is out of its slot",
                    );
                }
            }
        }
    } else if let Some(a) = att
        && a.id != e.base.entity_id
    {
        if is_wolf_tamed(e) && a.is_player && is_owner(e, a.username.as_deref()) {
            return true;
        }
        cr_mut(e).player_to_attack = Some(a.id);
    }
    true
}

/// `EntityWolf.findPlayerToAttack()`: the closest player within 16 while
/// angry.
// src: EntityWolf.java:300-302
pub fn find_player_to_attack(e: &Entity, world: &World) -> Option<i32> {
    if is_wolf_angry(e) {
        world.get_closest_player_to_entity(&e.base, 16.0)
    } else {
        None
    }
}

/// `EntityWolf.attackEntity(target, distance)`: between 2 and 6 blocks a
/// 1-in-10 leap toward the target (from the ground); within 1.5 blocks and
/// overlapping vertically, a bite of 2 (4 when tamed), setting
/// `attackTime` 20.
///
/// vanilla bug (transcribed): nothing checks that `attackTime` (unlike
/// `EntityMob.attackEntity`, EntityMob.java:50), so the wolf bites every AI
/// tick it is in range; only the target's hurt-resistance window spaces the
/// hits.
// src: EntityWolf.java:304-324
pub fn attack_entity(e: &mut Entity, world: &mut World, target: i32, var2: f32) {
    let Some(var1) = EntityView::of(world, target) else {
        // EntityCreature calls this only with a live target in its slot;
        // the Java's nextInt(10) below would be skipped here.
        world.note_unimplemented("EntityWolf.attackEntity: the target is out of its slot");
        return;
    };
    if var2 > 2.0 && var2 < 6.0 && e.base.rand.next_int_bounded(10) == 0 {
        if e.base.on_ground {
            let var8 = var1.pos_x - e.base.pos_x;
            let var5 = var1.pos_z - e.base.pos_z;
            let var7 = math::sqrt_double(var8 * var8 + var5 * var5);
            let b = &mut e.base;
            b.motion_x =
                var8 / var7 as f64 * 0.5 * 0.800000011920929 + b.motion_x * 0.20000000298023224;
            b.motion_z =
                var5 / var7 as f64 * 0.5 * 0.800000011920929 + b.motion_z * 0.20000000298023224;
            b.motion_y = 0.4000000059604645;
        }
    } else if (var2 as f64) < 1.5
        && var1.bounding_box.max_y > e.base.bounding_box.min_y
        && var1.bounding_box.min_y < e.base.bounding_box.max_y
    {
        lv_mut(e).attack_time = 20;
        let mut var3: i8 = 2;
        if is_wolf_tamed(e) {
            var3 = 4;
        }
        let me: &Entity = e;
        let hit = world.with_entity_mut(target, |w, t| {
            dispatch::attack_entity_from(t, w, Some(me), var3 as i32)
        });
        if hit.is_none() {
            world.note_unimplemented("EntityWolf.attackEntity: the target is out of its slot");
        }
    }
}

/// `EntityWolf.interact(player)`.
///
/// Untamed and calm: a bone is eaten (`--stackSize`, the slot emptied at 0)
/// and tames on a 1-in-3 roll (stops, sits, 20 health, the player as owner,
/// hearts) or puffs smoke; true. Tamed: favourite meat heals by the raw
/// porkchop's 3 when the watched health is below 20 (eaten as the bone is);
/// otherwise the owner toggles sitting (and stops the jump and the path);
/// true. Anything else: false.
// src: EntityWolf.java:326-378
pub fn interact(e: &mut Entity, world: &mut World, var1: &mut Entity) -> bool {
    let Some(p) = var1.kind.player() else {
        return false;
    };
    let username = p.username.clone();
    let var2 = p.inventory.get_current_item().map(|s| s.item_id);
    if !is_wolf_tamed(e) {
        if var2 == Some(BONE) && !is_wolf_angry(e) {
            eat_held_item(var1);
            if !world.multiplayer_world {
                if e.base.rand.next_int_bounded(3) == 0 {
                    set_wolf_tamed(e, true);
                    cr_mut(e).path_to_entity = None;
                    set_wolf_sitting(e, true);
                    lv_mut(e).health = 20;
                    set_wolf_owner(e, username.as_deref());
                    show_hearts_or_smoke_fx(e, world, true);
                    world.set_entity_state(e.base.entity_id, 7);
                } else {
                    show_hearts_or_smoke_fx(e, world, false);
                    world.set_entity_state(e.base.entity_id, 6);
                }
            }
            return true;
        }
    } else {
        if let Some(id) = var2
            && let Some(fav) = item::item(id).and_then(|d| d.get_is_wolfs_favorite_meat())
            && fav
            && watched_health(e) < 20
        {
            eat_held_item(var1);
            let heal = item::item(PORK_RAW)
                .and_then(|d| d.get_heal_amount())
                .expect("Item.porkRaw is an ItemFood");
            living::heal(e, heal);
            return true;
        }

        if is_owner(e, username.as_deref()) {
            if !world.multiplayer_world {
                let s = !is_wolf_sitting(e);
                set_wolf_sitting(e, s);
                lv_mut(e).is_jumping = false;
                cr_mut(e).path_to_entity = None;
            }
            return true;
        }
    }
    false
}

/// `--var2.stackSize; if (var2.stackSize <= 0)
/// var1.inventory.setInventorySlotContents(var1.inventory.currentItem,
/// null)` on the held stack (the one in `mainInventory[currentItem]`, which
/// `getCurrentItem` returned).
// src: EntityWolf.java:330-333, :356-359
fn eat_held_item(player: &mut Entity) {
    let p = player
        .kind
        .player_mut()
        .expect("EntityWolf.interact with a non-player");
    let slot = p.inventory.current_item;
    let s = p.inventory.main_inventory[slot as usize]
        .as_mut()
        .expect("the held stack is in slot currentItem");
    s.stack_size = s.stack_size.wrapping_sub(1);
    if s.stack_size <= 0 {
        p.inventory.set_inventory_slot_contents(slot, None);
    }
}

/// `showHeartsOrSmokeFX(hearts)`: seven `"heart"` or `"smoke"` particles
/// around the wolf (three `nextGaussian` and three `nextFloat` draws each).
// src: EntityWolf.java:380-393
pub fn show_hearts_or_smoke_fx(e: &mut Entity, world: &mut World, var1: bool) {
    let var2 = if var1 { "heart" } else { "smoke" };
    for _ in 0..7 {
        let b = &mut e.base;
        let var4 = b.rand.next_gaussian() * 0.02;
        let var6 = b.rand.next_gaussian() * 0.02;
        let var8 = b.rand.next_gaussian() * 0.02;
        let x = b.pos_x + (b.rand.next_float() * b.width * 2.0) as f64 - b.width as f64;
        let y = b.pos_y + 0.5 + (b.rand.next_float() * b.height) as f64;
        let z = b.pos_z + (b.rand.next_float() * b.width * 2.0) as f64 - b.width as f64;
        world.spawn_particle(var2, x, y, z, var4, var6, var8);
    }
}

/// `EntityWolf.handleHealthUpdate(status)` (the multiplayer client's entity
/// status; `World.setEntityState` is empty in singleplayer): 7 hearts,
/// 6 smoke, 8 starts the shake animation, others `EntityLiving`'s.
// src: EntityWolf.java:395-408
pub fn handle_health_update(e: &mut Entity, world: &mut World, var1: i8) {
    if var1 == 7 {
        show_hearts_or_smoke_fx(e, world, true);
    } else if var1 == 6 {
        show_hearts_or_smoke_fx(e, world, false);
    } else if var1 == 8 {
        let w = wd_mut(e);
        w.shake_running = true;
        w.time_wolf_is_shaking = 0.0;
        w.prev_time_wolf_is_shaking = 0.0;
    } else {
        living::handle_health_update(e, world, var1);
    }
}

/// `setTailRotation()` (render: the tail pitch, RenderWolf.java:13).
// src: EntityWolf.java:410-416
pub fn set_tail_rotation(e: &Entity) -> f32 {
    if is_wolf_angry(e) {
        1.5393804
    } else if is_wolf_tamed(e) {
        (0.55 - (20 - watched_health(e)) as f32 * 0.02) * 3.1415927
    } else {
        0.62831855
    }
}
