//! `EntityChicken` (EntityChicken.java).
//!
//! | EntityChicken.java | Method | Port |
//! |---|---|---|
//! | 4-10 | fields | [`ChickenData`] |
//! | 12-18 | constructor (the first egg timer draw) | [`new_entity_chicken`] |
//! | 20-49 | `onLivingUpdate` (wing flap, slow fall, egg laying) | [`on_living_update`] |
//! | 51-52 | `fall` (empty: no fall damage) | [`super::dispatch::fall`] |
//! | 54-60 | `writeEntityToNBT`, `readEntityFromNBT` (`super` only) | [`super::nbt`] |
//! | 62-72 | `getLivingSound`, `getHurtSound`, `getDeathSound` | [`super::dispatch`] |
//! | 74-76 | `getDropItemId` (feather) | [`get_drop_item_id`] |
//!
//! # The egg timer and the harness
//!
//! The constructor draws `timeUntilNextEgg` from `rand`, which is an
//! unseeded `new Random()` in vanilla (Entity.java:96). The oracle redraws
//! it on the seeded stream when the chicken joins the world
//! (`HarnessWorld.seedOnJoin`: seed `mix(seed, id)`, `nextInt(6000) +
//! 6000`, seed again; tools/oracle/census.md §3). The port's `rand` is
//! seeded with `mix(seed, id)` at construction (`EntityBase::new`) and again
//! at the join (`World::spawn_entity_in_world`), and nothing draws from it
//! before the constructor's draw, so the constructor's value is the
//! oracle's redrawn value and the join leaves the same `rand` state. The
//! timer is not saved in NBT, so a chicken loaded from a save redraws it in
//! vanilla too.

use super::{
    AnimalData, CreatureData, Entity, EntityBase, EntityKindData, LivingData, base, living,
};
use crate::world::World;

/// `Item.feather.shiftedIndex` (`256 + 32`).
// src: Item.java:306
pub const FEATHER: i32 = 256 + 32;
/// `Item.egg.shiftedIndex` (`256 + 88`).
// src: Item.java:362
pub const EGG: i32 = 256 + 88;

/// The fields `EntityChicken` declares (EntityChicken.java:4-10), with their
/// initialisers. All but `timeUntilNextEgg` are the wing animation
/// (`RenderChicken.getWingRotation`, RenderChicken.java:12-16).
#[derive(Clone, Debug, PartialEq)]
pub struct ChickenData {
    /// `field_753_a` (`false`): never read or written anywhere in the client
    /// tree (docs/mappings.md).
    pub field_753_a: bool,
    /// `wingRotation`.
    pub wing_rotation: f32,
    /// `destPos`.
    pub dest_pos: f32,
    /// `oFlapSpeed`.
    pub o_flap_speed: f32,
    /// `oFlap`.
    pub o_flap: f32,
    /// `wingRotDelta` (`1.0F`).
    pub wing_rot_delta: f32,
    /// `timeUntilNextEgg`: ticks until the next egg (constructor and every
    /// egg: `rand.nextInt(6000) + 6000`).
    pub time_until_next_egg: i32,
}

impl Default for ChickenData {
    // src: EntityChicken.java:4-10
    fn default() -> Self {
        ChickenData {
            field_753_a: false,
            wing_rotation: 0.0,
            dest_pos: 0.0,
            o_flap_speed: 0.0,
            o_flap: 0.0,
            wing_rot_delta: 1.0,
            time_until_next_egg: 0,
        }
    }
}

fn ch(e: &Entity) -> &ChickenData {
    match &e.kind {
        EntityKindData::Chicken(.., d) => d,
        _ => panic!("EntityChicken method on a non-chicken"),
    }
}

fn ch_mut(e: &mut Entity) -> &mut ChickenData {
    match &mut e.kind {
        EntityKindData::Chicken(.., d) => d,
        _ => panic!("EntityChicken method on a non-chicken"),
    }
}

/// `new EntityChicken(world)`: `Entity(World)` (`entityInit` is
/// `EntityLiving`'s, empty), the `EntityLiving` initialisers and body (four
/// `Math.random()` draws), the `EntityChicken` field initialisers, then the
/// texture, a 0.3 × 0.4 box (`setSize` does not move it), 4 health and the
/// first egg timer, `rand.nextInt(6000) + 6000` (module docs).
// src: EntityChicken.java:4-18, Entity.java:75-113, EntityLiving.java:6-71
pub fn new_entity_chicken(world: &mut World) -> Entity {
    let mut b = EntityBase::new(world);
    let mut l = LivingData::new(world, &mut b);
    let mut c = ChickenData::default();
    l.texture = "/mob/chicken.png";
    b.set_size(0.3, 0.4);
    l.health = 4;
    c.time_until_next_egg = b.rand.next_int_bounded(6000) + 6000;
    Entity {
        base: b,
        kind: EntityKindData::Chicken(l, CreatureData::default(), AnimalData::default(), c),
    }
}

/// `EntityChicken.onLivingUpdate()`: `EntityLiving`'s, then the wing
/// animation (`destPos` toward 0 on the ground at 0.3 a tick, toward 1 in
/// the air at 1.2, clamped), the slow fall (`motionY *= 0.6` while falling),
/// and the egg timer: at 0 a `"mob.chickenplop"` (two `rand` draws for the
/// pitch), one egg item (`dropItem`, no offset) and a new timer.
// src: EntityChicken.java:20-49
pub fn on_living_update(e: &mut Entity, world: &mut World) {
    living::on_living_update(e, world);
    let on_ground = e.base.on_ground;
    let c = ch_mut(e);
    c.o_flap = c.wing_rotation;
    c.o_flap_speed = c.dest_pos;
    let step: i32 = if on_ground { -1 } else { 4 };
    c.dest_pos = (c.dest_pos as f64 + step as f64 * 0.3) as f32;
    #[allow(clippy::manual_clamp)] // Java's two ifs, kept (NaN behaviour)
    {
        if c.dest_pos < 0.0 {
            c.dest_pos = 0.0;
        }
        if c.dest_pos > 1.0 {
            c.dest_pos = 1.0;
        }
    }
    if !on_ground && c.wing_rot_delta < 1.0 {
        c.wing_rot_delta = 1.0;
    }
    c.wing_rot_delta = (c.wing_rot_delta as f64 * 0.9) as f32;
    if !on_ground && e.base.motion_y < 0.0 {
        e.base.motion_y *= 0.6;
    }
    let c = ch_mut(e);
    c.wing_rotation += c.wing_rot_delta * 2.0;
    if !world.multiplayer_world {
        // `--timeUntilNextEgg <= 0`
        let c = ch_mut(e);
        c.time_until_next_egg = c.time_until_next_egg.wrapping_sub(1);
        if c.time_until_next_egg <= 0 {
            let pitch = (e.base.rand.next_float() - e.base.rand.next_float()) * 0.2 + 1.0;
            world.play_sound_at_entity(&e.base, "mob.chickenplop", 1.0, pitch);
            base::drop_item(e, world, EGG, 1);
            let t = e.base.rand.next_int_bounded(6000) + 6000;
            ch_mut(e).time_until_next_egg = t;
        }
    }
}

/// `EntityChicken.getDropItemId()`: a feather.
// src: EntityChicken.java:74-76
pub fn get_drop_item_id() -> i32 {
    FEATHER
}

/// `timeUntilNextEgg` (for tests and the client's debug views).
pub fn time_until_next_egg(e: &Entity) -> i32 {
    ch(e).time_until_next_egg
}
