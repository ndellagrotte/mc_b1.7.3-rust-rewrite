//! `EntityWaterMob` (EntityWaterMob.java): the creatures of
//! `EnumCreatureType.waterCreature` (the squid).
//!
//! | EntityWaterMob.java | Method | Port |
//! |---|---|---|
//! | 4-6 | constructor (`super` only) | the leaf constructors |
//! | 8-10 | `canBreatheUnderwater` (`true`) | [`super::dispatch::can_breathe_underwater`] |
//! | 12-18 | `writeEntityToNBT`, `readEntityFromNBT` (`super` only) | [`super::nbt`] |
//! | 20-22 | `getCanSpawnHere` | [`get_can_spawn_here`] |
//! | 24-26 | `getTalkInterval` (120) | [`super::dispatch::get_talk_interval`] |
//!
//! `EntityWaterMob` declares no fields: its level is
//! [`super::WaterMobData`] (empty).

use super::Entity;
use crate::world::World;

/// `EntityWaterMob.getCanSpawnHere()`: only that no spawn-blocking entity
/// is in the box (no `super` call: neither `EntityCreature`'s path weight
/// nor `EntityLiving`'s collision and liquid tests run, so a squid spawns
/// in water).
// src: EntityWaterMob.java:20-22
pub fn get_can_spawn_here(e: &Entity, world: &mut World) -> bool {
    world.check_if_aabb_is_clear(&e.base.bounding_box)
}
