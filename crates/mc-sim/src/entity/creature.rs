//! `EntityCreature` (EntityCreature.java): the path-following AI of the
//! animals and monsters.
//!
//! | EntityCreature.java | Method | Port |
//! |---|---|---|
//! | 4-6 | fields | [`CreatureData`] |
//! | 8-10 | constructor (`super` only) | the leaf constructors |
//! | 12-14 | `isMovementCeased` | [`dispatch::is_movement_ceased`] |
//! | 16-116 | `updatePlayerActionState` | [`update_player_action_state`] |
//! | 118-143 | `updateWanderPath` | [`update_wander_path`] |
//! | 145-157 | `attackEntity`, `attackBlockedEntity`, `getBlockPathWeight`, `findPlayerToAttack` (empty / 0 / null) | [`dispatch`] |
//! | 159-164 | `getCanSpawnHere` | [`get_can_spawn_here`] |
//! | 166-180 | `hasPath`, `setPathToEntity`, `getTarget`, `setTarget` | [`CreatureData`] fields (no override anywhere) |
//!
//! `playerToAttack` is an entity id ([`crate::entity`] module docs); what the
//! AI reads off it comes from the world ([`EntityView`]).

use mc_core::math::{self, floor_double, jmath};

use super::living::{self, EntityView, lv, lv_mut};
use super::pathfinding::PathEntity;
use super::{Entity, EntityKindData, LivingData, base, dispatch};
use crate::world::World;

/// The fields `EntityCreature` declares (EntityCreature.java:4-6).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CreatureData {
    /// `pathToEntity` (private): the path being followed.
    pub path_to_entity: Option<PathEntity>,
    /// `playerToAttack` (protected), as an entity id: the creature's target
    /// (`getTarget()` / `setTarget()`, EntityCreature.java:174-180).
    pub player_to_attack: Option<i32>,
    /// `hasAttacked` (protected): `isMovementCeased()` of this tick.
    pub has_attacked: bool,
}

/// The `EntityLiving` and `EntityCreature` levels of every kind below
/// `EntityCreature`.
pub(crate) fn levels_mut(
    kind: &mut EntityKindData,
) -> Option<(&mut LivingData, &mut CreatureData)> {
    use EntityKindData as K;
    match kind {
        K::Mob(l, c, ..)
        | K::Creeper(l, c, ..)
        | K::Skeleton(l, c, ..)
        | K::Spider(l, c, ..)
        | K::Zombie(l, c, ..)
        | K::PigZombie(l, c, ..)
        | K::GiantZombie(l, c, ..)
        | K::Pig(l, c, ..)
        | K::Sheep(l, c, ..)
        | K::Cow(l, c, ..)
        | K::Chicken(l, c, ..)
        | K::Wolf(l, c, ..)
        | K::Squid(l, c, ..) => Some((l, c)),
        _ => None,
    }
}

/// The `EntityCreature` level (`instanceof EntityCreature`).
pub fn creature(kind: &EntityKindData) -> Option<&CreatureData> {
    use EntityKindData as K;
    match kind {
        K::Mob(_, c, ..)
        | K::Creeper(_, c, ..)
        | K::Skeleton(_, c, ..)
        | K::Spider(_, c, ..)
        | K::Zombie(_, c, ..)
        | K::PigZombie(_, c, ..)
        | K::GiantZombie(_, c, ..)
        | K::Pig(_, c, ..)
        | K::Sheep(_, c, ..)
        | K::Cow(_, c, ..)
        | K::Chicken(_, c, ..)
        | K::Wolf(_, c, ..)
        | K::Squid(_, c, ..) => Some(c),
        _ => None,
    }
}

/// The `EntityCreature` level, mutably.
pub fn creature_mut(kind: &mut EntityKindData) -> Option<&mut CreatureData> {
    levels_mut(kind).map(|(_, c)| c)
}

fn cr(e: &Entity) -> &CreatureData {
    creature(&e.kind).expect("EntityCreature method on a non-creature")
}

fn cr_mut(e: &mut Entity) -> &mut CreatureData {
    creature_mut(&mut e.kind).expect("EntityCreature method on a non-creature")
}

/// `playerToAttack.isEntityAlive()` for a target held by id (gone or
/// taken out: recorded, not alive).
fn target_alive(world: &World, id: i32) -> bool {
    match world.entity(id) {
        Some(t) => dispatch::is_entity_alive(t),
        None => !world.is_entity_dead_or_gone(id),
    }
}

/// `worldObj.getPathToEntity(this, playerToAttack, range)` for a target held
/// by id; `None` (recorded) when the target is not in its slot.
fn path_to_target(e: &Entity, world: &mut World, target: i32, var1: f32) -> Option<PathEntity> {
    match world.entity(target).map(|t| t.base.clone()) {
        Some(tb) => world.get_path_to_entity(&e.base, &tb, var1),
        None => {
            world.note_unimplemented(
                "EntityCreature.updatePlayerActionState: playerToAttack not in its slot",
            );
            None
        }
    }
}

/// `EntityCreature.updatePlayerActionState()`: pick or drop a target (virtual
/// `findPlayerToAttack`), attack it if seen (virtual `attackEntity` /
/// `attackBlockedEntity`), re-path to it 1 tick in 20 or wander (1 in 80),
/// then walk the path (a 1 % roll per tick abandons it for
/// `EntityLiving`'s idle AI): turn at most 30° toward the next point, walk
/// at `moveSpeed`, jump to climb or swim.
// src: EntityCreature.java:16-116
pub fn update_player_action_state(e: &mut Entity, world: &mut World) {
    let ceased = dispatch::is_movement_ceased(e, world);
    cr_mut(e).has_attacked = ceased;
    let var1 = 16.0f32;
    match cr(e).player_to_attack {
        None => {
            let t = dispatch::find_player_to_attack(e, world);
            cr_mut(e).player_to_attack = t;
            if let Some(t) = t {
                let p = path_to_target(e, world, t, var1);
                cr_mut(e).path_to_entity = p;
            }
        }
        Some(t) if !target_alive(world, t) => {
            cr_mut(e).player_to_attack = None;
        }
        Some(t) => {
            if let Some(v) = EntityView::of(world, t) {
                let var2 = v.distance_to(&e.base);
                if living::can_entity_be_seen(e, world, &v) {
                    dispatch::attack_entity(e, world, t, var2);
                } else {
                    dispatch::attack_blocked_entity(e, world, t, var2);
                }
            }
        }
    }

    let (has_attacked, no_target, path_set) = {
        let c = cr(e);
        (
            c.has_attacked,
            c.player_to_attack.is_none(),
            c.path_to_entity.is_some(),
        )
    };
    // `hasAttacked || playerToAttack == null || pathToEntity != null &&
    // rand.nextInt(20) != 0`, short-circuit draws.
    let wander = has_attacked || no_target || (path_set && e.base.rand.next_int_bounded(20) != 0);
    if wander {
        // `!hasAttacked && (pathToEntity == null && rand.nextInt(80) == 0 ||
        // rand.nextInt(80) == 0)`.
        if !has_attacked {
            let no_path = cr(e).path_to_entity.is_none();
            let roll = (no_path && e.base.rand.next_int_bounded(80) == 0)
                || e.base.rand.next_int_bounded(80) == 0;
            if roll {
                update_wander_path(e, world);
            }
        }
    } else if let Some(t) = cr(e).player_to_attack {
        let p = path_to_target(e, world, t, var1);
        cr_mut(e).path_to_entity = p;
    }

    let var21 = floor_double(e.base.bounding_box.min_y + 0.5);
    let var3 = dispatch::is_in_water(e, world);
    let var4 = base::handle_lava_movement(e, world);
    e.base.rotation_pitch = 0.0;
    if cr(e).path_to_entity.is_some() && e.base.rand.next_int_bounded(100) != 0 {
        follow_path(e, world, var21, var3, var4);
    } else {
        living::update_player_action_state(e, world);
        cr_mut(e).path_to_entity = None;
    }
}

/// The path-walking branch of `updatePlayerActionState`
/// (EntityCreature.java:47-111).
// src: EntityCreature.java:47-111
fn follow_path(e: &mut Entity, world: &mut World, var21: i32, var3: bool, var4: bool) {
    let mut var5 = {
        let (b, k) = (&e.base, &e.kind);
        creature(k)
            .and_then(|c| c.path_to_entity.as_ref())
            .map(|p| p.get_position(b))
    };
    let var6 = (e.base.width * 2.0) as f64;
    while let Some(v) = var5 {
        if v.square_distance_to_xyz(e.base.pos_x, v.y_coord, e.base.pos_z) >= var6 * var6 {
            break;
        }
        let Entity { base: b, kind } = e;
        let c = creature_mut(kind).expect("EntityCreature method on a non-creature");
        let path = c.path_to_entity.as_mut().expect("followed path");
        path.increment_path_index();
        if path.is_finished() {
            var5 = None;
            c.path_to_entity = None;
        } else {
            var5 = Some(path.get_position(b));
        }
    }

    lv_mut(e).is_jumping = false;
    if let Some(v) = var5 {
        let var8 = v.x_coord - e.base.pos_x;
        let var10 = v.z_coord - e.base.pos_z;
        let var12 = v.y_coord - var21 as f64;
        let var14 = (jmath::atan2(var10, var8) * 180.0 / 3.1415927410125732) as f32 - 90.0;
        let mut var15 = var14 - e.base.rotation_yaw;
        let speed = lv(e).move_speed;
        lv_mut(e).move_forward = speed;
        while var15 < -180.0 {
            var15 += 360.0;
        }
        while var15 >= 180.0 {
            var15 -= 360.0;
        }
        #[allow(clippy::manual_clamp)] // Java's two ifs, kept (NaN behaviour)
        if var15 > 30.0 {
            var15 = 30.0;
        }
        if var15 < -30.0 {
            var15 = -30.0;
        }
        e.base.rotation_yaw += var15;
        if cr(e).has_attacked
            && let Some(t) = cr(e).player_to_attack
            && let Some(tv) = EntityView::of(world, t)
        {
            let var16 = tv.pos_x - e.base.pos_x;
            let var18 = tv.pos_z - e.base.pos_z;
            let var20 = e.base.rotation_yaw;
            e.base.rotation_yaw =
                (jmath::atan2(var18, var16) * 180.0 / 3.1415927410125732) as f32 - 90.0;
            var15 = (var20 - e.base.rotation_yaw + 90.0) * 3.1415927 / 180.0;
            let l = lv_mut(e);
            l.move_strafing = -math::sin(var15) * l.move_forward * 1.0;
            l.move_forward = math::cos(var15) * l.move_forward * 1.0;
        }
        if var12 > 0.0 {
            lv_mut(e).is_jumping = true;
        }
    }

    if let Some(t) = cr(e).player_to_attack
        && let Some(tv) = EntityView::of(world, t)
    {
        living::face_entity(e, world, &tv, 30.0, 30.0);
    }

    if e.base.is_collided_horizontally && !has_path(e) {
        lv_mut(e).is_jumping = true;
    }

    if e.base.rand.next_float() < 0.8 && (var3 || var4) {
        lv_mut(e).is_jumping = true;
    }
}

/// `updateWanderPath()`: ten random cells within ±6, ±3, ±6 blocks; a path
/// (range 10) to the one with the highest `getBlockPathWeight` (virtual),
/// the first on ties.
// src: EntityCreature.java:118-143
pub fn update_wander_path(e: &mut Entity, world: &mut World) {
    let mut var1 = false;
    let mut var2 = -1;
    let mut var3 = -1;
    let mut var4 = -1;
    let mut var5 = -99999.0f32;
    for _ in 0..10 {
        let b = &mut e.base;
        let var7 = floor_double(b.pos_x + b.rand.next_int_bounded(13) as f64 - 6.0);
        let var8 = floor_double(b.pos_y + b.rand.next_int_bounded(7) as f64 - 3.0);
        let var9 = floor_double(b.pos_z + b.rand.next_int_bounded(13) as f64 - 6.0);
        let var10 = dispatch::get_block_path_weight(e, world, var7, var8, var9);
        if var10 > var5 {
            var5 = var10;
            var2 = var7;
            var3 = var8;
            var4 = var9;
            var1 = true;
        }
    }
    if var1 {
        let p = world.get_entity_path_to_xyz(&e.base, var2, var3, var4, 10.0);
        cr_mut(e).path_to_entity = p;
    }
}

/// `EntityCreature.getCanSpawnHere()`: `EntityLiving`'s, and a path weight
/// of at least 0 at the feet (virtual `getBlockPathWeight`).
// src: EntityCreature.java:159-164
pub fn get_can_spawn_here(e: &Entity, world: &mut World) -> bool {
    let var1 = floor_double(e.base.pos_x);
    let var2 = floor_double(e.base.bounding_box.min_y);
    let var3 = floor_double(e.base.pos_z);
    living::get_can_spawn_here(e, world)
        && dispatch::get_block_path_weight(e, world, var1, var2, var3) >= 0.0
}

/// `hasPath()`.
// src: EntityCreature.java:166-168
pub fn has_path(e: &Entity) -> bool {
    cr(e).path_to_entity.is_some()
}
