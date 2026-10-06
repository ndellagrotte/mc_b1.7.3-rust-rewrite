//! `EntityPigZombie` (EntityPigZombie.java) (WP-2.3).
//!
//! | EntityPigZombie.java | Method | Port |
//! |---|---|---|
//! | 6-8, 93-95 | `angerLevel`, `randomSoundDelay`; `defaultHeldItem` (a gold sword) | [`PigZombieData`]; [`get_held_item`] |
//! | 10-16 | constructor | [`new_entity_pig_zombie`] |
//! | 18-25 | `onUpdate` (runs while it has a target; the angry sound) | [`on_update`] |
//! | 27-29 | `getCanSpawnHere` | [`get_can_spawn_here`] |
//! | 31-39 | `writeEntityToNBT`, `readEntityFromNBT` (`Anger`) | [`super::nbt`] |
//! | 41-43 | `findPlayerToAttack` (only when angry) | [`find_player_to_attack`] |
//! | 45-47 | `onLivingUpdate` (`super` only: `EntityZombie`'s daylight burn) | [`on_living_update`] |
//! | 49-65 | `attackEntityFrom` (a player angers every pigman within 32) | [`attack_entity_from`] |
//! | 67-71 | `becomeAngryAt` (private) | [`become_angry_at`] |
//! | 73-87 | the sounds; `getDropItemId` (cooked porkchop) | [`super::dispatch`] |
//! | 89-91 | `getHeldItem` (render data) | [`get_held_item`] |
//!
//! vanilla bug: b1.7.3 never lowers `angerLevel` (no decrement anywhere in
//! the tree): once angry, a pigman stays angry for good (`Anger` is saved).

use super::{
    Entity, EntityBase, EntityKindData, LivingData, creature, dispatch, living, mob, zombie,
};
use crate::item::ItemStack;
use crate::world::World;

/// `Item.porkCooked.shiftedIndex` (`256 + 64`).
// src: Item.java:338
pub const PORK_COOKED: i32 = 256 + 64;
/// `Item.swordGold.shiftedIndex` (`256 + 27`).
// src: Item.java:301
pub const SWORD_GOLD: i32 = 256 + 27;

/// The fields `EntityPigZombie` declares (EntityPigZombie.java:6-7).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PigZombieData {
    /// `angerLevel` (private): non-zero = hostile (saved as `Anger`).
    pub anger_level: i32,
    /// `randomSoundDelay` (private): ticks until the angry sound.
    pub random_sound_delay: i32,
}

fn pz(e: &Entity) -> &PigZombieData {
    match &e.kind {
        EntityKindData::PigZombie(.., d) => d,
        _ => panic!("EntityPigZombie method on another kind"),
    }
}

fn pz_mut(e: &mut Entity) -> &mut PigZombieData {
    match &mut e.kind {
        EntityKindData::PigZombie(.., d) => d,
        _ => panic!("EntityPigZombie method on another kind"),
    }
}

/// `new EntityPigZombie(world)`: `EntityZombie`'s constructor, then the
/// pigman texture, `moveSpeed` 0.5, `attackStrength` 5 again, and fire
/// immunity.
// src: EntityPigZombie.java:10-16, EntityZombie.java:4-9, EntityMob.java:6-9, Entity.java:75-113, EntityLiving.java:6-68
pub fn new_entity_pig_zombie(world: &mut World) -> Entity {
    let mut b = EntityBase::new(world);
    let mut l = LivingData::new(world, &mut b);
    let (c, mut m) = mob::mob_level(&mut l);
    let z = zombie::zombie_level(&mut l, &mut m);
    l.texture = "/mob/pigzombie.png";
    l.move_speed = 0.5;
    m.attack_strength = 5;
    b.is_immune_to_fire = true;
    Entity {
        base: b,
        kind: EntityKindData::PigZombie(l, c, m, z, PigZombieData::default()),
    }
}

/// `getHeldItem()`: the static `defaultHeldItem`, `new
/// ItemStack(Item.swordGold, 1)` (render data, RenderBiped).
// src: EntityPigZombie.java:89-95
pub fn get_held_item() -> ItemStack {
    ItemStack::new(SWORD_GOLD, 1, 0)
}

/// `EntityPigZombie.onUpdate()`: `moveSpeed` 0.95 with a target, else 0.5;
/// the angry sound when `randomSoundDelay` counts down to 0 (twice the
/// sound volume, pitch from two `rand` draws); then `EntityMob`'s
/// (`EntityZombie` has no `onUpdate`).
// src: EntityPigZombie.java:18-25
pub fn on_update(e: &mut Entity, world: &mut World) {
    let has_target = creature::creature(&e.kind).is_some_and(|c| c.player_to_attack.is_some());
    living::lv_mut(e).move_speed = if has_target { 0.95 } else { 0.5 };
    if pz(e).random_sound_delay > 0 {
        pz_mut(e).random_sound_delay -= 1;
        if pz(e).random_sound_delay == 0 {
            let volume = dispatch::get_sound_volume(e, world) * 2.0;
            let pitch = ((e.base.rand.next_float() - e.base.rand.next_float()) * 0.2 + 1.0) * 1.8;
            world.play_sound_at_entity(&e.base, "mob.zombiepig.zpigangry", volume, pitch);
        }
    }
    mob::on_update(e, world);
}

/// `EntityPigZombie.getCanSpawnHere()`: not on peaceful, and
/// `EntityLiving`'s test (no light roll: no `rand` draw).
// src: EntityPigZombie.java:27-29
pub fn get_can_spawn_here(e: &Entity, world: &mut World) -> bool {
    let bb = e.base.bounding_box;
    world.difficulty_setting > 0
        && world.check_if_aabb_is_clear(&bb)
        && world.get_colliding_bounding_boxes(e, &bb).is_empty()
        && !world.get_is_any_liquid(&bb)
}

/// `EntityPigZombie.findPlayerToAttack()`: none while `angerLevel` is 0,
/// else `EntityMob`'s.
// src: EntityPigZombie.java:41-43
pub fn find_player_to_attack(e: &mut Entity, world: &mut World) -> Option<i32> {
    if pz(e).anger_level == 0 {
        None
    } else {
        mob::find_player_to_attack(e, world)
    }
}

/// `EntityPigZombie.onLivingUpdate()`: `super` only (`EntityZombie`'s).
// src: EntityPigZombie.java:45-47
pub fn on_living_update(e: &mut Entity, world: &mut World) {
    zombie::on_living_update(e, world);
}

/// `EntityPigZombie.attackEntityFrom(attacker, damage)`: an attacking
/// player angers every pigman whose box meets this one's grown by 32 in
/// each direction (in `getEntitiesWithinAABBExcludingEntity` order), then
/// this one; then `EntityMob`'s.
// src: EntityPigZombie.java:49-65
pub fn attack_entity_from(
    e: &mut Entity,
    world: &mut World,
    var1: Option<&Entity>,
    var2: i32,
) -> bool {
    if let Some(a) = var1.filter(|a| a.kind.is_player()) {
        let aid = a.base.entity_id;
        let bb = e.base.bounding_box.expand(32.0, 32.0, 32.0);
        let var3 = world.get_entities_within_aabb_excluding_entity(Some(e.base.entity_id), &bb);
        for var5 in var3 {
            world.with_entity_mut(var5, |_, o| {
                if let EntityKindData::PigZombie(..) = o.kind {
                    become_angry_at(o, aid);
                }
            });
        }
        become_angry_at(e, aid);
    }
    mob::attack_entity_from(e, world, var1, var2)
}

/// `becomeAngryAt(entity)` (private): the target, `angerLevel` 400–799 and
/// `randomSoundDelay` 0–39 (two `rand` draws).
// src: EntityPigZombie.java:67-71
pub fn become_angry_at(e: &mut Entity, var1: i32) {
    creature::creature_mut(&mut e.kind)
        .expect("EntityPigZombie is an EntityCreature")
        .player_to_attack = Some(var1);
    let anger = 400 + e.base.rand.next_int_bounded(400);
    pz_mut(e).anger_level = anger;
    let delay = e.base.rand.next_int_bounded(40);
    pz_mut(e).random_sound_delay = delay;
}
