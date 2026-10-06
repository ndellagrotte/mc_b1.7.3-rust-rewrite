//! `EntitySkeleton` (EntitySkeleton.java) (WP-2.3).
//!
//! | EntitySkeleton.java | Method | Port |
//! |---|---|---|
//! | 4, 87-89 | `defaultHeldItem` (a bow) | [`get_held_item`] |
//! | 6-9 | constructor | [`new_entity_skeleton`] |
//! | 11-21 | `getLivingSound`, `getHurtSound`, `getDeathSound` | [`super::dispatch`] |
//! | 23-32 | `onLivingUpdate` (burns in daylight) | [`on_living_update`] |
//! | 34-53 | `attackEntity` (the bow) | [`attack_entity`] |
//! | 55-61 | `writeEntityToNBT`, `readEntityFromNBT` (`super` only) | [`super::nbt`] |
//! | 63-65 | `getDropItemId` (arrow) | [`super::dispatch::get_drop_item_id`], [`ARROW`] |
//! | 67-81 | `dropFewItems` (0-2 arrows, 0-2 bones) | [`drop_few_items`] |
//! | 83-85 | `getHeldItem` (render data) | [`get_held_item`] |
//!
//! The arrow is `new EntityArrow(world, skeleton)`, whose constructor draws
//! its heading jitter from the arrow's own `rand`; in the oracle that
//! `Random` is unseeded then (tools/oracle/census.md §3), but the skeleton
//! calls `setArrowHeading` again after `entityJoinedWorld` (which seeds it),
//! and that call replaces the motion and the angles the first one set, so
//! the arrow's state is reproducible.

use mc_core::jmath;
use mc_core::math;

use super::living::{lv, lv_mut};
use super::{
    Entity, EntityBase, EntityKindData, LivingData, arrow, base, creature, dispatch, mob, zombie,
};
use crate::item::ItemStack;
use crate::world::World;

/// `Item.bow.shiftedIndex` (`256 + 5`).
// src: Item.java:279
pub const BOW: i32 = 256 + 5;
/// `Item.arrow.shiftedIndex` (`256 + 6`).
// src: Item.java:280
pub const ARROW: i32 = 256 + 6;
/// `Item.bone.shiftedIndex` (`256 + 96`).
// src: Item.java:370
pub const BONE: i32 = 256 + 96;

/// `EntitySkeleton` declares no instance fields (`defaultHeldItem` is
/// static: [`get_held_item`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SkeletonData {}

/// `new EntitySkeleton(world)`: `EntityMob`'s, with the skeleton texture.
// src: EntitySkeleton.java:6-9, EntityMob.java:6-9, Entity.java:75-113, EntityLiving.java:6-68
pub fn new_entity_skeleton(world: &mut World) -> Entity {
    let mut b = EntityBase::new(world);
    let mut l = LivingData::new(world, &mut b);
    let (c, m) = mob::mob_level(&mut l);
    l.texture = "/mob/skeleton.png";
    Entity {
        base: b,
        kind: EntityKindData::Skeleton(l, c, m, SkeletonData::default()),
    }
}

/// `getHeldItem()`: the static `defaultHeldItem`, `new ItemStack(Item.bow,
/// 1)` (render data, RenderBiped).
// src: EntitySkeleton.java:83-89
pub fn get_held_item() -> ItemStack {
    ItemStack::new(BOW, 1, 0)
}

/// `EntitySkeleton.onLivingUpdate()`: the daylight burn (as the zombie's),
/// then `EntityMob`'s.
// src: EntitySkeleton.java:23-32
pub fn on_living_update(e: &mut Entity, world: &mut World) {
    zombie::burn_in_daylight(e, world);
    mob::on_living_update(e, world);
}

/// `EntitySkeleton.attackEntity(target, distance)`: within 10 blocks, an
/// arrow every 30 ticks of the attack timer (`attackTime == 0`): `new
/// EntityArrow(world, this)`, one block higher (`++posY`, the box is not
/// moved), the bow sound (one `rand` draw), `entityJoinedWorld`, then
/// `setArrowHeading` toward the target's eyes 0.2 down, raised by a fifth
/// of the horizontal distance, speed 0.6, spread 12. Every call in range
/// faces the target and sets `hasAttacked` (so the creature AI stops
/// walking).
// src: EntitySkeleton.java:34-53
pub fn attack_entity(e: &mut Entity, world: &mut World, target: i32, var2: f32) {
    if var2 < 10.0 {
        let Some(t) = world.entity(target) else {
            world.note_unimplemented("EntitySkeleton.attackEntity: the target is not in its slot");
            return;
        };
        let (tx, ty, tz) = (t.base.pos_x, t.base.pos_y, t.base.pos_z);
        let t_eye = dispatch::get_eye_height(t, world);
        let var3 = tx - e.base.pos_x;
        let var5 = tz - e.base.pos_z;
        if lv(e).attack_time == 0 {
            let mut var7 = arrow::new_entity_arrow_shot(world, e);
            var7.base.pos_y += 1.0;
            let var8 = ty + t_eye as f64 - 0.20000000298023224 - var7.base.pos_y;
            let var10 = math::sqrt_double(var3 * var3 + var5 * var5) * 0.2;
            let pitch = 1.0 / (e.base.rand.next_float() * 0.4 + 0.8);
            world.play_sound_at_entity(&e.base, "random.bow", 1.0, pitch);
            let arrow_id = var7.id();
            world.spawn_entity_in_world(var7);
            // Java calls setArrowHeading on its reference whether or not the
            // arrow joined; one that did not join is gone here, and nothing
            // reads it again.
            world.with_entity_mut(arrow_id, |_, a| {
                arrow::set_arrow_heading(a, var3, var8 + var10 as f64, var5, 0.6, 12.0)
            });
            lv_mut(e).attack_time = 30;
        }
        e.base.rotation_yaw = (jmath::atan2(var5, var3) * 180.0 / 3.1415927410125732) as f32 - 90.0;
        creature::creature_mut(&mut e.kind)
            .expect("EntitySkeleton is an EntityCreature")
            .has_attacked = true;
    }
}

/// `EntitySkeleton.dropFewItems()`: `rand.nextInt(3)` arrows, then
/// `rand.nextInt(3)` bones, one `dropItem` each (no `super` call).
// src: EntitySkeleton.java:67-81
pub fn drop_few_items(e: &mut Entity, world: &mut World) {
    let var1 = e.base.rand.next_int_bounded(3);
    for _ in 0..var1 {
        base::drop_item(e, world, ARROW, 1);
    }
    let var1 = e.base.rand.next_int_bounded(3);
    for _ in 0..var1 {
        base::drop_item(e, world, BONE, 1);
    }
}
