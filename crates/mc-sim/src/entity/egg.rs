//! `EntityEgg` (EntityEgg.java): a thrown egg. The class is
//! `EntitySnowball` with renamed fields plus the hatching step at the impact
//! (EntityEgg.java:156-167), so its body is [`super::snowball`]'s
//! ([`throwable_on_update`](super::snowball) with [`hatch`] as the impact
//! hook).
//!
//! One egg in 8 hatches one chicken (one in 32 of those, four) where it
//! lands: `new EntityChicken(world)` ([`super::chicken`]),
//! `setLocationAndAngles` at the egg with the egg's yaw and pitch 0,
//! `entityJoinedWorld` (WP-2.3; the chicken's egg timer and the harness:
//! [`super::chicken`] module docs).

use super::Entity;
use super::chicken::new_entity_chicken;
use super::snowball::{self, ThrowableData};
use crate::world::World;

/// The `EntityEgg` level (the fields are `EntitySnowball`'s, renamed).
pub type EggData = ThrowableData;

/// `new EntityEgg(world)`.
// src: EntityEgg.java:16-19
pub fn new_entity_egg_empty(world: &mut World) -> Entity {
    snowball::new_thrown_empty(world, true)
}

/// `new EntityEgg(world, thrower)` (ItemEgg.java:13).
// src: EntityEgg.java:30-45
pub fn new_entity_egg_thrown(world: &mut World, thrower: &Entity) -> Entity {
    snowball::new_thrown_by(world, thrower, true)
}

/// `new EntityEgg(world, x, y, z)` (BlockDispenser.java:113).
// src: EntityEgg.java:47-53
pub fn new_entity_egg_at(world: &mut World, x: f64, y: f64, z: f64) -> Entity {
    snowball::new_thrown_at(world, x, y, z, true)
}

/// `setEggHeading(x, y, z, speed, spread)`.
// src: EntityEgg.java:55-73
pub fn set_egg_heading(e: &mut Entity, x: f64, y: f64, z: f64, speed: f32, spread: f32) {
    snowball::set_thrown_heading(e, x, y, z, speed, spread);
}

/// `EntityEgg.onUpdate()`.
// src: EntityEgg.java:87-215
pub fn on_update(e: &mut Entity, world: &mut World) {
    snowball::throwable_on_update(e, world, hatch);
}

/// The hatching step at the impact (module docs).
// src: EntityEgg.java:156-167
fn hatch(e: &mut Entity, world: &mut World) {
    if !world.multiplayer_world && e.base.rand.next_int_bounded(8) == 0 {
        let mut var16: i8 = 1;
        if e.base.rand.next_int_bounded(32) == 0 {
            var16 = 4;
        }
        for _ in 0..var16 {
            // src: EntityEgg.java:163-165
            let mut var21 = new_entity_chicken(world);
            let b = &e.base;
            var21
                .base
                .set_location_and_angles(b.pos_x, b.pos_y, b.pos_z, b.rotation_yaw, 0.0);
            world.spawn_entity_in_world(var21);
        }
    }
}
