//! `EntityAnimal` (EntityAnimal.java): grass-loving creatures.
//!
//! | EntityAnimal.java | Method | Port |
//! |---|---|---|
//! | 4-6 | constructor (`super` only) | the leaf constructors |
//! | 8-10 | `getBlockPathWeight` | [`get_block_path_weight`] |
//! | 12-18 | `writeEntityToNBT`, `readEntityFromNBT` (`super` only) | [`super::nbt`] |
//! | 20-25 | `getCanSpawnHere` | [`get_can_spawn_here`] |
//! | 27-29 | `getTalkInterval` (120) | [`super::dispatch::get_talk_interval`] |

use mc_core::math::floor_double;

use super::{Entity, creature};
use crate::block::ids;
use crate::world::World;

/// `EntityAnimal` declares no fields.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnimalData {}

/// `EntityAnimal.getBlockPathWeight(x, y, z)`: 10 on grass, else the light
/// brightness there minus 0.5.
// src: EntityAnimal.java:8-10
pub fn get_block_path_weight(world: &mut World, var1: i32, var2: i32, var3: i32) -> f32 {
    if world.get_block_id(var1, var2 - 1, var3) == ids::GRASS {
        10.0
    } else {
        world.get_light_brightness(var1, var2, var3) - 0.5
    }
}

/// `EntityAnimal.getCanSpawnHere()`: grass under the feet, full light above
/// 8, and `EntityCreature`'s test.
// src: EntityAnimal.java:20-25
pub fn get_can_spawn_here(e: &Entity, world: &mut World) -> bool {
    let var1 = floor_double(e.base.pos_x);
    let var2 = floor_double(e.base.bounding_box.min_y);
    let var3 = floor_double(e.base.pos_z);
    world.get_block_id(var1, var2 - 1, var3) == ids::GRASS
        && world.get_full_block_light_value(var1, var2, var3) > 8
        && creature::get_can_spawn_here(e, world)
}
