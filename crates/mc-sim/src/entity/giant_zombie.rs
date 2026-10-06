//! `EntityGiantZombie` (EntityGiantZombie.java) (WP-2.3). `EntityList`'s
//! `"Giant"`; nothing in b1.7.3 spawns it naturally.
//!
//! | EntityGiantZombie.java | Method | Port |
//! |---|---|---|
//! | 4-12 | constructor | [`new_entity_giant_zombie`] |
//! | 14-16 | `getBlockPathWeight` (brighter is better) | [`get_block_path_weight`] |

use super::{Entity, EntityBase, EntityKindData, LivingData, mob};
use crate::world::World;

/// `EntityGiantZombie` declares no fields.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GiantZombieData {}

/// `new EntityGiantZombie(world)`: the zombie texture, `moveSpeed` 0.5,
/// `attackStrength` 50, ten times `EntityMob`'s health (200), `yOffset`
/// times 6 (still 0) and a box six times the default (3.6 × 10.8).
// src: EntityGiantZombie.java:4-12, EntityMob.java:6-9, Entity.java:75-113, EntityLiving.java:6-68
pub fn new_entity_giant_zombie(world: &mut World) -> Entity {
    let mut b = EntityBase::new(world);
    let mut l = LivingData::new(world, &mut b);
    let (c, mut m) = mob::mob_level(&mut l);
    l.texture = "/mob/zombie.png";
    l.move_speed = 0.5;
    m.attack_strength = 50;
    l.health = l.health.wrapping_mul(10);
    b.y_offset *= 6.0;
    let (w, h) = (b.width * 6.0, b.height * 6.0);
    b.set_size(w, h);
    Entity {
        base: b,
        kind: EntityKindData::GiantZombie(l, c, m, GiantZombieData::default()),
    }
}

/// `EntityGiantZombie.getBlockPathWeight(x, y, z)`: `brightness - 0.5`,
/// the opposite of `EntityMob`'s.
// src: EntityGiantZombie.java:14-16
pub fn get_block_path_weight(world: &mut World, var1: i32, var2: i32, var3: i32) -> f32 {
    world.get_light_brightness(var1, var2, var3) - 0.5
}
