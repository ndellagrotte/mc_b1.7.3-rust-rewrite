//! `EntityCreeper` (EntityCreeper.java) (WP-2.3).
//!
//! | EntityCreeper.java | Method | Port |
//! |---|---|---|
//! | 4-5 | `timeSinceIgnited`, `lastActiveTime` | [`CreeperData`] |
//! | 7-10 | constructor | [`new_entity_creeper`] |
//! | 12-16 | `entityInit` (watcher 16: state −1, 17: powered 0) | [`new_entity_creeper`] |
//! | 18-29 | `writeEntityToNBT`, `readEntityFromNBT` (`powered`) | [`super::nbt`] |
//! | 31-42 | `attackBlockedEntity` (the fuse winds back) | [`attack_blocked_entity`] |
//! | 44-71 | `onUpdate` | [`on_update`] |
//! | 73-79 | `getHurtSound`, `getDeathSound` | [`super::dispatch`] |
//! | 81-87 | `onDeath` (a record when a skeleton killed it) | [`on_death`] |
//! | 89-119 | `attackEntity` (the fuse and the explosion) | [`attack_entity`] |
//! | 121-123 | `getPowered` | [`get_powered`] |
//! | 125-127 | `setCreeperFlashTime` (render: the fuse flash) | [`set_creeper_flash_time`] |
//! | 129-131 | `getDropItemId` (gunpowder) | [`super::dispatch::get_drop_item_id`], [`GUNPOWDER`] |
//! | 133-139 | `getCreeperState`, `setCreeperState` (private) | [`get_creeper_state`], [`set_creeper_state`] |
//! | 141-144 | `onStruckByLightning` (becomes powered) | [`on_struck_by_lightning`] |
//!
//! The powered creeper exists in b1.7.3: lightning sets watcher byte 17,
//! saved as `powered`, and doubles the explosion (6 instead of 3). The
//! `worldObj.multiplayerWorld` branches (the client following the server's
//! state) are kept as in Java; single player never takes them.

use super::data_watcher::WatchValue;
use super::{Entity, EntityBase, EntityKindData, LivingData, base, creature, dispatch, mob};
use crate::world::World;

/// `Item.gunpowder.shiftedIndex` (`256 + 33`).
// src: Item.java:307
pub const GUNPOWDER: i32 = 256 + 33;
/// `Item.record13.shiftedIndex` (`256 + 2000`); `+ rand.nextInt(2)` is
/// `recordCat` (Item.java:379).
// src: Item.java:378
pub const RECORD_13: i32 = 256 + 2000;

/// The fields `EntityCreeper` declares (EntityCreeper.java:4-5).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CreeperData {
    /// `timeSinceIgnited` (package-private): fuse ticks, 30 explodes.
    pub time_since_ignited: i32,
    /// `lastActiveTime` (package-private): `timeSinceIgnited` at the start of
    /// the tick (render interpolation, [`set_creeper_flash_time`]).
    pub last_active_time: i32,
}

fn cd(e: &Entity) -> &CreeperData {
    match &e.kind {
        EntityKindData::Creeper(_, _, _, d) => d,
        _ => panic!("EntityCreeper method on another kind"),
    }
}

fn cd_mut(e: &mut Entity) -> &mut CreeperData {
    match &mut e.kind {
        EntityKindData::Creeper(_, _, _, d) => d,
        _ => panic!("EntityCreeper method on another kind"),
    }
}

/// `new EntityCreeper(world)`: `entityInit` (`EntityLiving`'s, empty, then
/// watcher byte 16 = −1 and 17 = 0) runs in `Entity`'s constructor, before
/// `EntityLiving`'s field initialisers; then the creeper texture.
// src: EntityCreeper.java:7-16, EntityMob.java:6-9, Entity.java:75-113, EntityLiving.java:6-68
pub fn new_entity_creeper(world: &mut World) -> Entity {
    let mut b = EntityBase::new(world);
    b.data_watcher.add_object(16, WatchValue::Byte(-1));
    b.data_watcher.add_object(17, WatchValue::Byte(0));
    let mut l = LivingData::new(world, &mut b);
    let (c, m) = mob::mob_level(&mut l);
    l.texture = "/mob/creeper.png";
    Entity {
        base: b,
        kind: EntityKindData::Creeper(l, c, m, CreeperData::default()),
    }
}

/// `getCreeperState()` (private): watcher byte 16 (−1 idle, 1 fusing).
// src: EntityCreeper.java:133-135
pub fn get_creeper_state(e: &Entity) -> i32 {
    e.base.data_watcher.get_watchable_object_byte(16) as i32
}

/// `setCreeperState(state)` (private): watcher byte 16 = `(byte) state`.
// src: EntityCreeper.java:137-139
pub fn set_creeper_state(e: &mut Entity, var1: i32) {
    e.base
        .data_watcher
        .update_object(16, WatchValue::Byte(var1 as i8));
}

/// `getPowered()`: watcher byte 17 is 1.
// src: EntityCreeper.java:121-123
pub fn get_powered(e: &Entity) -> bool {
    e.base.data_watcher.get_watchable_object_byte(17) == 1
}

/// `setCreeperFlashTime(partialTick)` (render): the interpolated fuse time
/// over 28.
// src: EntityCreeper.java:125-127
pub fn set_creeper_flash_time(e: &Entity, var1: f32) -> f32 {
    let d = cd(e);
    (d.last_active_time as f32 + (d.time_since_ignited - d.last_active_time) as f32 * var1) / 28.0
}

/// One step back of the fuse: state −1, `timeSinceIgnited - 1`, at least 0
/// (the same three statements in `attackBlockedEntity`, `onUpdate` and
/// `attackEntity`).
// src: EntityCreeper.java:34-38, :64-68, :111-115
fn wind_back(e: &mut Entity) {
    set_creeper_state(e, -1);
    let d = cd_mut(e);
    d.time_since_ignited -= 1;
    if d.time_since_ignited < 0 {
        d.time_since_ignited = 0;
    }
}

/// `EntityCreeper.attackBlockedEntity(target, distance)`: a target it
/// cannot see winds the fuse back.
// src: EntityCreeper.java:31-42
pub fn attack_blocked_entity(e: &mut Entity, world: &mut World) {
    if !world.multiplayer_world && cd(e).time_since_ignited > 0 {
        wind_back(e);
    }
}

/// `EntityCreeper.onUpdate()`: remembers the fuse time, follows the
/// server's state in multiplayer, runs `EntityMob`'s, and winds the fuse
/// back when it has no target.
// src: EntityCreeper.java:44-71
pub fn on_update(e: &mut Entity, world: &mut World) {
    let t = cd(e).time_since_ignited;
    cd_mut(e).last_active_time = t;
    if world.multiplayer_world {
        let var1 = get_creeper_state(e);
        if var1 > 0 && cd(e).time_since_ignited == 0 {
            world.play_sound_at_entity(&e.base, "random.fuse", 1.0, 0.5);
        }
        let d = cd_mut(e);
        d.time_since_ignited += var1;
        #[allow(clippy::manual_clamp)] // Java's two ifs, kept
        if d.time_since_ignited < 0 {
            d.time_since_ignited = 0;
        }
        if d.time_since_ignited >= 30 {
            d.time_since_ignited = 30;
        }
    }
    mob::on_update(e, world);
    let no_target = creature::creature(&e.kind).is_some_and(|c| c.player_to_attack.is_none());
    if no_target && cd(e).time_since_ignited > 0 {
        wind_back(e);
    }
}

/// `EntityCreeper.onDeath(attacker)`: `EntityLiving`'s, then a music disc
/// (`record13 + rand.nextInt(2)`) when the killer is a skeleton.
// src: EntityCreeper.java:81-87
pub fn on_death(e: &mut Entity, world: &mut World, var1: Option<&Entity>) {
    super::living::on_death(e, world, var1);
    if var1.is_some_and(|a| matches!(a.kind, EntityKindData::Skeleton(..))) {
        let id = RECORD_13 + e.base.rand.next_int_bounded(2);
        base::drop_item(e, world, id, 1);
    }
}

/// `EntityCreeper.attackEntity(target, distance)`: closer than 3 (7 once
/// fusing), the fuse burns (the sound on its first tick, state 1,
/// `timeSinceIgnited + 1`); at 30 the creeper explodes (`createExplosion`
/// with itself as the exploder, size 3, or 6 when powered) and dies.
/// `hasAttacked` stops it walking. Out of range the fuse winds back.
// src: EntityCreeper.java:89-119
pub fn attack_entity(e: &mut Entity, world: &mut World, var2: f32) {
    if !world.multiplayer_world {
        let var3 = get_creeper_state(e);
        if (var3 <= 0 && var2 < 3.0) || (var3 > 0 && var2 < 7.0) {
            if cd(e).time_since_ignited == 0 {
                world.play_sound_at_entity(&e.base, "random.fuse", 1.0, 0.5);
            }
            set_creeper_state(e, 1);
            cd_mut(e).time_since_ignited += 1;
            if cd(e).time_since_ignited >= 30 {
                let (x, y, z) = (e.base.pos_x, e.base.pos_y, e.base.pos_z);
                let size = if get_powered(e) { 6.0 } else { 3.0 };
                world.create_explosion(Some(e), x, y, z, size);
                dispatch::set_entity_dead(e, world);
            }
            creature::creature_mut(&mut e.kind)
                .expect("EntityCreeper is an EntityCreature")
                .has_attacked = true;
        } else {
            wind_back(e);
        }
    }
}

/// `EntityCreeper.onStruckByLightning(bolt)`: `Entity`'s (fire damage),
/// then powered (watcher byte 17 = 1).
// src: EntityCreeper.java:141-144
pub fn on_struck_by_lightning(e: &mut Entity, world: &mut World, var1: &Entity) {
    base::on_struck_by_lightning(e, world, var1);
    e.base.data_watcher.update_object(17, WatchValue::Byte(1));
}
