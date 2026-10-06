//! `EntityZombie` (EntityZombie.java) (WP-2.3).
//!
//! | EntityZombie.java | Method | Port |
//! |---|---|---|
//! | 4-9 | constructor | [`new_entity_zombie`], [`zombie_level`] |
//! | 11-20 | `onLivingUpdate` (burns in daylight) | [`on_living_update`] |
//! | 22-32 | `getLivingSound`, `getHurtSound`, `getDeathSound` | [`super::dispatch`] |
//! | 34-36 | `getDropItemId` (feather) | [`super::dispatch::get_drop_item_id`], [`FEATHER`] |

use mc_core::math::floor_double;

use super::{Entity, EntityBase, EntityKindData, LivingData, base, mob};
use crate::world::World;

/// `Item.feather.shiftedIndex` (`256 + 32`).
// src: Item.java:306
pub const FEATHER: i32 = 256 + 32;

/// `EntityZombie` declares no fields.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ZombieData {}

/// `EntityZombie`'s constructor body on the levels above it: the zombie
/// texture, `moveSpeed` 0.5, `attackStrength` 5 (`EntityPigZombie`'s
/// constructor runs it first).
// src: EntityZombie.java:4-9
pub fn zombie_level(l: &mut LivingData, m: &mut mob::MobData) -> ZombieData {
    l.texture = "/mob/zombie.png";
    l.move_speed = 0.5;
    m.attack_strength = 5;
    ZombieData::default()
}

/// `new EntityZombie(world)`.
// src: EntityZombie.java:4-9, EntityMob.java:6-9, Entity.java:75-113, EntityLiving.java:6-68
pub fn new_entity_zombie(world: &mut World) -> Entity {
    let mut b = EntityBase::new(world);
    let mut l = LivingData::new(world, &mut b);
    let (c, mut m) = mob::mob_level(&mut l);
    let z = zombie_level(&mut l, &mut m);
    Entity {
        base: b,
        kind: EntityKindData::Zombie(l, c, m, z),
    }
}

/// The daylight burn shared by `EntityZombie.onLivingUpdate` and
/// `EntitySkeleton.onLivingUpdate` (the same three lines in both): in the
/// day, brighter than 0.5, under open sky, and `rand.nextFloat() * 30 <
/// (brightness - 0.4) * 2` (short-circuit: the draw only when the first
/// two hold) sets `fire` to 300.
// src: EntityZombie.java:12-17, EntitySkeleton.java:24-29
pub fn burn_in_daylight(e: &mut Entity, world: &mut World) {
    if world.is_daytime() {
        let var1 = base::get_entity_brightness(e, world, 1.0);
        if var1 > 0.5
            && world.can_block_see_the_sky(
                floor_double(e.base.pos_x),
                floor_double(e.base.pos_y),
                floor_double(e.base.pos_z),
            )
            && e.base.rand.next_float() * 30.0 < (var1 - 0.4) * 2.0
        {
            e.base.fire = 300;
        }
    }
}

/// `EntityZombie.onLivingUpdate()`: the daylight burn, then `EntityMob`'s.
// src: EntityZombie.java:11-20
pub fn on_living_update(e: &mut Entity, world: &mut World) {
    burn_in_daylight(e, world);
    mob::on_living_update(e, world);
}
