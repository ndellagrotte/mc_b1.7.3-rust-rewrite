//! `EntityMob` (EntityMob.java): the hostile creatures' shared level
//! (WP-2.3).
//!
//! | EntityMob.java | Method | Port |
//! |---|---|---|
//! | 4 | `attackStrength` (2) | [`MobData`] |
//! | 6-9 | constructor (`health = 20`) | [`mob_level`]; [`new_entity_mob`] for `EntityList`'s `"Monster"` |
//! | 11-18 | `onLivingUpdate` (ages twice as fast in the light) | [`on_living_update`] |
//! | 20-26 | `onUpdate` (dies in peaceful) | [`on_update`] |
//! | 28-31 | `findPlayerToAttack` (closest player within 16, if seen) | [`find_player_to_attack`] |
//! | 33-47 | `attackEntityFrom` (the attacker becomes the target) | [`attack_entity_from`] |
//! | 49-55 | `attackEntity` (melee every 20 ticks) | [`attack_entity`] |
//! | 57-59 | `getBlockPathWeight` (darker is better) | [`get_block_path_weight`] |
//! | 61-67 | `writeEntityToNBT`, `readEntityFromNBT` (`super` only) | [`super::nbt`] |
//! | 69-86 | `getCanSpawnHere` (sky and block light rolls) | [`get_can_spawn_here`] |
//!
//! `EntityMob implements IMob` (IMob.java: a marker interface, read by
//! `SpawnerAnimals` through `EnumCreatureType.monster`, WP-2.3 task 3).
//!
//! `getCanSpawnHere` draws twice from `Entity.rand`. In the oracle that
//! `Random` is still unseeded when `SpawnerAnimals` asks (before
//! `entityJoinedWorld`, tools/oracle/census.md §3), so natural hostile
//! spawning is not reproducible there; the port's `rand` is seeded at
//! construction. The `spawn` op seeds after construction and never calls it.

use mc_core::math::floor_double;

use super::living::{self, EntityView, lv, lv_mut};
use super::{
    CreatureData, Entity, EntityBase, EntityKindData, LivingData, base, creature, dispatch,
};
use crate::world::World;
use crate::world::lighting::EnumSkyBlock;

/// The field `EntityMob` declares (EntityMob.java:4).
#[derive(Clone, Debug, PartialEq)]
pub struct MobData {
    /// `attackStrength` (protected): the melee damage of [`attack_entity`].
    pub attack_strength: i32,
}

impl Default for MobData {
    /// The field initialiser: `attackStrength = 2`.
    // src: EntityMob.java:4
    fn default() -> Self {
        MobData { attack_strength: 2 }
    }
}

/// The `EntityMob` level of every kind below it.
pub fn mob(kind: &EntityKindData) -> Option<&MobData> {
    use EntityKindData as K;
    match kind {
        K::Mob(_, _, m)
        | K::Creeper(_, _, m, ..)
        | K::Skeleton(_, _, m, ..)
        | K::Spider(_, _, m, ..)
        | K::Zombie(_, _, m, ..)
        | K::PigZombie(_, _, m, ..)
        | K::GiantZombie(_, _, m, ..) => Some(m),
        _ => None,
    }
}

/// The `EntityMob` level, mutably.
pub fn mob_mut(kind: &mut EntityKindData) -> Option<&mut MobData> {
    use EntityKindData as K;
    match kind {
        K::Mob(_, _, m)
        | K::Creeper(_, _, m, ..)
        | K::Skeleton(_, _, m, ..)
        | K::Spider(_, _, m, ..)
        | K::Zombie(_, _, m, ..)
        | K::PigZombie(_, _, m, ..)
        | K::GiantZombie(_, _, m, ..) => Some(m),
        _ => None,
    }
}

pub(crate) fn mb(e: &Entity) -> &MobData {
    mob(&e.kind).expect("EntityMob method on a non-mob")
}

/// The `EntityCreature` and `EntityMob` constructors' bodies, run on the
/// `EntityLiving` level the leaf constructor built: `EntityCreature`'s is
/// `super` only (EntityCreature.java:8-10), `EntityMob`'s sets `health` to
/// 20 after its field initialiser (`attackStrength = 2`).
// src: EntityMob.java:4-9, EntityCreature.java:4-10
pub fn mob_level(l: &mut LivingData) -> (CreatureData, MobData) {
    let m = MobData::default();
    l.health = 20;
    (CreatureData::default(), m)
}

/// `new EntityMob(world)`: `EntityList`'s `"Monster"` (EntityList.java:88;
/// `EntityMob` is a concrete class in b1.7.3). `entityInit` is
/// `EntityLiving`'s empty one; the texture stays `/mob/char.png`.
// src: EntityMob.java:6-9, Entity.java:75-113, EntityLiving.java:6-68
pub fn new_entity_mob(world: &mut World) -> Entity {
    let mut b = EntityBase::new(world);
    let mut l = LivingData::new(world, &mut b);
    let (c, m) = mob_level(&mut l);
    Entity {
        base: b,
        kind: EntityKindData::Mob(l, c, m),
    }
}

/// `EntityMob.onLivingUpdate()`: `entityAge += 2` when the brightness at
/// the entity is above 0.5 (so `despawnEntity`'s age test passes sooner),
/// then `EntityLiving`'s.
// src: EntityMob.java:11-18
pub fn on_living_update(e: &mut Entity, world: &mut World) {
    let var1 = base::get_entity_brightness(e, world, 1.0);
    if var1 > 0.5 {
        lv_mut(e).entity_age += 2;
    }
    living::on_living_update(e, world);
}

/// `EntityMob.onUpdate()`: `EntityLiving`'s, then `setEntityDead()`
/// (virtual) in a single-player world on peaceful (difficulty 0).
// src: EntityMob.java:20-26
pub fn on_update(e: &mut Entity, world: &mut World) {
    living::on_update(e, world);
    if !world.multiplayer_world && world.difficulty_setting == 0 {
        dispatch::set_entity_dead(e, world);
    }
}

/// `EntityMob.findPlayerToAttack()`: the closest player within 16 blocks,
/// if `canEntityBeSeen` (a block ray trace between the eyes).
// src: EntityMob.java:28-31
pub fn find_player_to_attack(e: &mut Entity, world: &mut World) -> Option<i32> {
    let var1 = world.get_closest_player_to_entity(&e.base, 16.0)?;
    let v = EntityView::of(world, var1)?;
    if living::can_entity_be_seen(e, world, &v) {
        Some(var1)
    } else {
        None
    }
}

/// `EntityMob.attackEntityFrom(attacker, damage)`: `EntityLiving`'s; when
/// that counts the hit, the attacker becomes `playerToAttack` unless it is
/// the rider, the mount or the mob itself. Java compares references, so a
/// null attacker equals a null `riddenByEntity` / `ridingEntity`: with
/// both set and no attacker, `playerToAttack` is cleared.
// src: EntityMob.java:33-47
pub fn attack_entity_from(
    e: &mut Entity,
    world: &mut World,
    var1: Option<&Entity>,
    var2: i32,
) -> bool {
    if living::attack_entity_from(e, world, var1, var2) {
        let a = var1.map(|a| a.base.entity_id);
        if e.base.ridden_by_entity != a && e.base.riding_entity != a {
            if a != Some(e.base.entity_id) {
                creature::creature_mut(&mut e.kind)
                    .expect("EntityMob is an EntityCreature")
                    .player_to_attack = a;
            }
            true
        } else {
            true
        }
    } else {
        false
    }
}

/// `EntityMob.attackEntity(target, distance)`: a melee hit of
/// `attackStrength` (`target.attackEntityFrom(this, …)`, virtual) when the
/// attack timer is out, the target is closer than 2 and the two boxes
/// overlap vertically; the timer restarts at 20.
// src: EntityMob.java:49-55
pub fn attack_entity(e: &mut Entity, world: &mut World, target: i32, var2: f32) {
    let Some(tbb) = world.entity(target).map(|t| t.base.bounding_box) else {
        world.note_unimplemented("EntityMob.attackEntity: the target is not in its slot");
        return;
    };
    if lv(e).attack_time <= 0
        && var2 < 2.0
        && tbb.max_y > e.base.bounding_box.min_y
        && tbb.min_y < e.base.bounding_box.max_y
    {
        lv_mut(e).attack_time = 20;
        let strength = mb(e).attack_strength;
        let attacker: &Entity = e;
        world.with_entity_mut(target, |w, t| {
            dispatch::attack_entity_from(t, w, Some(attacker), strength)
        });
    }
}

/// `EntityMob.getBlockPathWeight(x, y, z)`: `0.5 - brightness`, so dark
/// cells win the wander roll.
// src: EntityMob.java:57-59
pub fn get_block_path_weight(world: &mut World, var1: i32, var2: i32, var3: i32) -> f32 {
    0.5 - world.get_light_brightness(var1, var2, var3)
}

/// `EntityMob.getCanSpawnHere()`: sky light at the feet not above
/// `rand.nextInt(32)`, then the full light (with `skylightSubtracted`
/// forced to 10 while thundering) not above `rand.nextInt(8)`, then
/// `EntityCreature`'s test (short-circuit: the second draw only after the
/// first passes).
// src: EntityMob.java:69-86
pub fn get_can_spawn_here(e: &mut Entity, world: &mut World) -> bool {
    let var1 = floor_double(e.base.pos_x);
    let var2 = floor_double(e.base.bounding_box.min_y);
    let var3 = floor_double(e.base.pos_z);
    if world.get_saved_light_value(EnumSkyBlock::Sky, var1, var2, var3)
        > e.base.rand.next_int_bounded(32)
    {
        false
    } else {
        let mut var4 = world.get_block_light_value(var1, var2, var3);
        if world.get_is_thundering() {
            let var5 = world.skylight_subtracted;
            world.skylight_subtracted = 10;
            var4 = world.get_block_light_value(var1, var2, var3);
            world.skylight_subtracted = var5;
        }
        var4 <= e.base.rand.next_int_bounded(8) && creature::get_can_spawn_here(e, world)
    }
}
