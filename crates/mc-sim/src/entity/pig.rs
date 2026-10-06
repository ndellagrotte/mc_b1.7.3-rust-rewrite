//! `EntityPig` (EntityPig.java).
//!
//! | EntityPig.java | Method | Port |
//! |---|---|---|
//! | 4-8 | constructor | [`new_entity_pig`] |
//! | 10-12 | `entityInit` (watcher byte 16: the saddle) | [`new_entity_pig`] |
//! | 14-22 | `writeEntityToNBT`, `readEntityFromNBT` (`Saddle`) | [`super::nbt`] |
//! | 24-34 | `getLivingSound`, `getHurtSound`, `getDeathSound` | [`super::dispatch`] |
//! | 36-43 | `interact` | [`interact`] |
//! | 45-47 | `getDropItemId` | [`get_drop_item_id`] |
//! | 49-60 | `getSaddled`, `setSaddled` | [`get_saddled`], [`set_saddled`] |
//! | 62-69 | `onStruckByLightning` | [`on_struck_by_lightning`] (the `EntityPigZombie` is WP-2.3: recorded) |
//! | 71-77 | `fall` | [`fall`] (the `flyPig` achievement is WP-2.7: recorded) |

use super::data_watcher::WatchValue;
use super::{
    AnimalData, CreatureData, Entity, EntityBase, EntityKindData, LivingData, base, living,
};
use crate::world::World;

/// `Item.porkRaw.shiftedIndex` (`256 + 63`).
// src: Item.java:337
pub const PORK_RAW: i32 = 256 + 63;
/// `Item.porkCooked.shiftedIndex` (`256 + 64`).
// src: Item.java:338
pub const PORK_COOKED: i32 = 256 + 64;

/// `EntityPig` declares no fields (the saddle is data-watcher byte 16).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PigData {}

/// `new EntityPig(world)`: `Entity(World)` with `EntityPig.entityInit`
/// (watcher byte 16 = 0), the `EntityLiving` initialisers and body (four
/// `Math.random()` draws), the empty `EntityCreature` / `EntityAnimal`
/// levels, then the pig texture and a 0.9 × 0.9 box (`setSize` does not
/// move the box: the spawner's `setLocationAndAngles` does).
// src: EntityPig.java:4-12, Entity.java:75-113, EntityLiving.java:6-68, EntityCreature.java:4-10
pub fn new_entity_pig(world: &mut World) -> Entity {
    let mut b = EntityBase::new(world);
    b.data_watcher.add_object(16, WatchValue::Byte(0));
    let mut l = LivingData::new(world, &mut b);
    l.texture = "/mob/pig.png";
    b.set_size(0.9, 0.9);
    Entity {
        base: b,
        kind: EntityKindData::Pig(
            l,
            CreatureData::default(),
            AnimalData::default(),
            PigData::default(),
        ),
    }
}

/// `getSaddled()`: bit 0 of watcher byte 16.
// src: EntityPig.java:49-51
pub fn get_saddled(e: &Entity) -> bool {
    (e.base.data_watcher.get_watchable_object_byte(16) & 1) != 0
}

/// `setSaddled(saddled)`: watcher byte 16 = 1 or 0.
// src: EntityPig.java:53-60
pub fn set_saddled(e: &mut Entity, var1: bool) {
    let v = if var1 { 1 } else { 0 };
    e.base.data_watcher.update_object(16, WatchValue::Byte(v));
}

/// `EntityPig.interact(player)`: a saddled pig with no other rider is
/// mounted by the player (`player.mountEntity(this)`, `this` in hand).
// src: EntityPig.java:36-43
pub fn interact(e: &mut Entity, world: &mut World, var1: &mut Entity) -> bool {
    let other_rider = e
        .base
        .ridden_by_entity
        .is_some_and(|r| r != var1.base.entity_id);
    if !get_saddled(e) || world.multiplayer_world || other_rider {
        false
    } else {
        base::mount_entity(var1, world, Some(e));
        true
    }
}

/// `EntityPig.getDropItemId()`: cooked while burning.
// src: EntityPig.java:45-47
pub fn get_drop_item_id(e: &Entity) -> i32 {
    if e.base.fire > 0 {
        PORK_COOKED
    } else {
        PORK_RAW
    }
}

/// `EntityPig.onStruckByLightning(bolt)`: becomes a zombie pigman. The
/// `new EntityPigZombie(world)` (its constructor, `setLocationAndAngles`,
/// `entityJoinedWorld`) is WP-2.3 and recorded; the pig still dies, so it
/// leaves the world as in vanilla.
// src: EntityPig.java:62-69
pub fn on_struck_by_lightning(e: &mut Entity, world: &mut World) {
    if !world.multiplayer_world {
        world.note_unimplemented("EntityPig.onStruckByLightning");
        super::dispatch::set_entity_dead(e, world);
    }
}

/// `EntityPig.fall(distance)`: `EntityLiving`'s, and the `flyPig`
/// achievement for a player riding it down more than 5 blocks
/// (`triggerAchievement` is `addStat`: WP-2.7, recorded).
// src: EntityPig.java:71-77
pub fn fall(e: &mut Entity, world: &mut World, var1: f32) {
    living::fall(e, world, var1);
    if var1 > 5.0
        && let Some(r) = e.base.ridden_by_entity
        && world.entity(r).is_some_and(|p| p.kind.is_player())
    {
        world.note_unimplemented("EntityPlayer.triggerAchievement");
    }
}
