//! `EntitySpider` (EntitySpider.java) (WP-2.3).
//!
//! | EntitySpider.java | Method | Port |
//! |---|---|---|
//! | 4-9 | constructor | [`new_entity_spider`] |
//! | 11-13 | `getMountedYOffset` | [`get_mounted_y_offset`] |
//! | 15-17 | `canTriggerWalking` (false) | [`super::dispatch::can_trigger_walking`] |
//! | 19-27 | `findPlayerToAttack` (only in the dark, no sight test) | [`find_player_to_attack`] |
//! | 29-39 | `getLivingSound`, `getHurtSound`, `getDeathSound` | [`super::dispatch`] |
//! | 41-60 | `attackEntity` (loses interest in the light, leaps) | [`attack_entity`] |
//! | 62-68 | `writeEntityToNBT`, `readEntityFromNBT` (`super` only) | [`super::nbt`] |
//! | 70-72 | `getDropItemId` (string) | [`super::dispatch::get_drop_item_id`] |
//! | 74-76 | `isOnLadder` (climbs any wall it walks into) | [`is_on_ladder`] |

use mc_core::math;

use super::{Entity, EntityBase, EntityKindData, LivingData, base, creature, mob};
use crate::world::World;

/// `EntitySpider` declares no fields.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpiderData {}

/// `new EntitySpider(world)`: the spider texture, a 1.4 × 0.9 box,
/// `moveSpeed` 0.8.
// src: EntitySpider.java:4-9, EntityMob.java:6-9, Entity.java:75-113, EntityLiving.java:6-68
pub fn new_entity_spider(world: &mut World) -> Entity {
    let mut b = EntityBase::new(world);
    let mut l = LivingData::new(world, &mut b);
    let (c, m) = mob::mob_level(&mut l);
    l.texture = "/mob/spider.png";
    b.set_size(1.4, 0.9);
    l.move_speed = 0.8;
    Entity {
        base: b,
        kind: EntityKindData::Spider(l, c, m, SpiderData::default()),
    }
}

/// `EntitySpider.getMountedYOffset()`: `height * 0.75 - 0.5`.
// src: EntitySpider.java:11-13
pub fn get_mounted_y_offset(e: &Entity) -> f64 {
    e.base.height as f64 * 0.75 - 0.5
}

/// `EntitySpider.findPlayerToAttack()`: below brightness 0.5, the closest
/// player within 16 blocks, seen or not; none in the light.
// src: EntitySpider.java:19-27
pub fn find_player_to_attack(e: &mut Entity, world: &mut World) -> Option<i32> {
    let var1 = base::get_entity_brightness(e, world, 1.0);
    if var1 < 0.5 {
        let var2 = 16.0;
        world.get_closest_player_to_entity(&e.base, var2)
    } else {
        None
    }
}

/// `EntitySpider.attackEntity(target, distance)`: in the light, a 1 % roll
/// per call drops the target; otherwise, between 2 and 6 blocks a 10 % roll
/// leaps at it when on the ground (horizontal push 0.4 toward it plus a
/// fifth of the motion, `motionY` 0.4), and anything else is `EntityMob`'s
/// melee. The draws short-circuit: the light roll only above 0.5, the leap
/// roll only in the distance band.
// src: EntitySpider.java:41-60
pub fn attack_entity(e: &mut Entity, world: &mut World, target: i32, var2: f32) {
    let var3 = base::get_entity_brightness(e, world, 1.0);
    if var3 > 0.5 && e.base.rand.next_int_bounded(100) == 0 {
        creature::creature_mut(&mut e.kind)
            .expect("EntitySpider is an EntityCreature")
            .player_to_attack = None;
    } else if var2 > 2.0 && var2 < 6.0 && e.base.rand.next_int_bounded(10) == 0 {
        if e.base.on_ground {
            let Some((tx, tz)) = world.entity(target).map(|t| (t.base.pos_x, t.base.pos_z)) else {
                world
                    .note_unimplemented("EntitySpider.attackEntity: the target is not in its slot");
                return;
            };
            let b = &mut e.base;
            let var4 = tx - b.pos_x;
            let var6 = tz - b.pos_z;
            let var8 = math::sqrt_double(var4 * var4 + var6 * var6);
            b.motion_x =
                var4 / var8 as f64 * 0.5 * 0.800000011920929 + b.motion_x * 0.20000000298023224;
            b.motion_z =
                var6 / var8 as f64 * 0.5 * 0.800000011920929 + b.motion_z * 0.20000000298023224;
            b.motion_y = 0.4000000059604645;
        }
    } else {
        mob::attack_entity(e, world, target, var2);
    }
}

/// `EntitySpider.isOnLadder()`: `isCollidedHorizontally`, so a spider
/// walking into a wall climbs it (`EntityLiving.moveEntityWithHeading`'s
/// ladder branch).
// src: EntitySpider.java:74-76
pub fn is_on_ladder(e: &Entity) -> bool {
    e.base.is_collided_horizontally
}
