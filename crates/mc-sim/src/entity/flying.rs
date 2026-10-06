//! `EntityFlying` (EntityFlying.java): the ghast's flight (WP-2.3).
//!
//! | EntityFlying.java | Method | Port |
//! |---|---|---|
//! | 4-6 | constructor (`super` only) | the leaf constructor ([`super::ghast`]) |
//! | 8-9 | `fall` (empty: no fall damage) | [`super::dispatch::fall`] |
//! | 11-61 | `moveEntityWithHeading` (no gravity) | [`move_entity_with_heading`] |
//! | 63-65 | `isOnLadder` (false) | [`super::dispatch::is_on_ladder`] |

use mc_core::math::{self, floor_double};

use super::living::lv_mut;
use super::{Entity, base, dispatch};
use crate::block;
use crate::world::World;

/// `EntityFlying` declares no fields.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FlyingData {}

/// `Block.blocksList[id].slipperiness * 0.91F` of the block under the box,
/// or 0.546 (air: `0.6 * 0.91`) when the id is 0.
// src: EntityFlying.java:27-31, :38-42
fn ground_friction(e: &Entity, world: &mut World) -> f32 {
    let mut var3 = 0.54600006f32;
    let var4 = world.get_block_id(
        floor_double(e.base.pos_x),
        floor_double(e.base.bounding_box.min_y) - 1,
        floor_double(e.base.pos_z),
    );
    if var4 > 0
        && let Some(b) = block::block(var4)
    {
        var3 = b.slipperiness * 0.91;
    }
    var3
}

/// `EntityFlying.moveEntityWithHeading(strafe, forward)` (no `super`
/// call): `EntityLiving`'s movement without gravity, ladders or the liquid
/// climb: drag 0.8 in water, 0.5 in lava, else the air/ground friction on
/// all three axes; then the leg animation.
// src: EntityFlying.java:11-61
pub fn move_entity_with_heading(e: &mut Entity, world: &mut World, var1: f32, var2: f32) {
    if dispatch::is_in_water(e, world) {
        base::move_flying(&mut e.base, var1, var2, 0.02);
        let (mx, my, mz) = (e.base.motion_x, e.base.motion_y, e.base.motion_z);
        dispatch::move_entity(e, world, mx, my, mz);
        let b = &mut e.base;
        b.motion_x *= 0.800000011920929;
        b.motion_y *= 0.800000011920929;
        b.motion_z *= 0.800000011920929;
    } else if base::handle_lava_movement(e, world) {
        base::move_flying(&mut e.base, var1, var2, 0.02);
        let (mx, my, mz) = (e.base.motion_x, e.base.motion_y, e.base.motion_z);
        dispatch::move_entity(e, world, mx, my, mz);
        let b = &mut e.base;
        b.motion_x *= 0.5;
        b.motion_y *= 0.5;
        b.motion_z *= 0.5;
    } else {
        let mut var3 = 0.91f32;
        if e.base.on_ground {
            var3 = ground_friction(e, world);
        }
        let var8 = 0.16277136f32 / (var3 * var3 * var3);
        let speed = if e.base.on_ground { 0.1 * var8 } else { 0.02 };
        base::move_flying(&mut e.base, var1, var2, speed);
        var3 = 0.91;
        if e.base.on_ground {
            var3 = ground_friction(e, world);
        }
        let (mx, my, mz) = (e.base.motion_x, e.base.motion_y, e.base.motion_z);
        dispatch::move_entity(e, world, mx, my, mz);
        let b = &mut e.base;
        b.motion_x *= var3 as f64;
        b.motion_y *= var3 as f64;
        b.motion_z *= var3 as f64;
    }

    let b = &e.base;
    let var10 = b.pos_x - b.prev_pos_x;
    let var9 = b.pos_z - b.prev_pos_z;
    let mut var7 = math::sqrt_double(var10 * var10 + var9 * var9) * 4.0;
    if var7 > 1.0 {
        var7 = 1.0;
    }
    let l = lv_mut(e);
    l.prev_leg_yaw = l.leg_yaw;
    l.leg_yaw += (var7 - l.leg_yaw) * 0.4;
    l.leg_swing += l.leg_yaw;
}
