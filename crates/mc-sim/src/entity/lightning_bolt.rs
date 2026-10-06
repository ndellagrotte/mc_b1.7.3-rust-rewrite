//! `EntityLightningBolt` (EntityLightningBolt.java) < `EntityWeatherEffect`
//! (EntityWeatherEffect.java, no fields or methods of its own): a lightning
//! strike. It lives in `World.weatherEffects`, not in `loadedEntityList`
//! ([`World::add_weather_effect`], ticked first in
//! [`World::update_entities`]); it is never saved (`EntityList` has no name
//! for it, and weather effects are in no chunk).
//!
//! Transcribed: the constructor (bolt shape and lifetime drawn from its
//! `rand`; at difficulty 2 and above fire at and around the strike),
//! `onUpdate` (thunder, re-strikes with fire, `onStruckByLightning` on the
//! entities within 3 blocks, the sky flash `World.field_27172_i`), the empty
//! NBT bodies. Not ported: `isInRangeToRenderVec3D` (render). `boltVertex`
//! only seeds the renderer's shape (RenderLightningBolt.java:17); it is kept.
//!
//! The constructor's `rand` draws run before the bolt is anywhere a harness
//! hook could seed it (weather effects never pass through
//! `obtainEntitySkin`, tools/oracle/census.md §3); the port's `rand` is
//! seeded with `mix(seed, id)` at construction like every entity's. The
//! oracle's `LightningBolt` spawn kind replays the two draws on the seeded
//! stream (tests/golden/README.md).
//!
//! `Entity.onStruckByLightning` (Entity.java:1085-1092) is
//! [`base::on_struck_by_lightning`] (WP-2.1), reached through
//! [`super::dispatch::on_struck_by_lightning`].

use mc_core::Aabb;
use mc_core::math::floor_double;

use super::{Entity, EntityBase, EntityKindData, WeatherEffectData, base, dispatch};
use crate::world::World;

/// `Block.fire.blockID`.
// src: Block.java:643
const FIRE_ID: i32 = 51;

/// The fields `EntityLightningBolt` declares.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LightningBoltData {
    /// `lightningState` (private): 2 at the strike, counts down; below 0 the
    /// bolt waits for a re-strike or dies.
    // src: EntityLightningBolt.java:6
    pub lightning_state: i32,
    /// `boltVertex`: the render shape seed.
    // src: EntityLightningBolt.java:7
    pub bolt_vertex: i64,
    /// `boltLivingTime` (private): re-strikes left, 1 to 3.
    // src: EntityLightningBolt.java:8
    pub bolt_living_time: i32,
}

fn data(e: &mut Entity) -> &mut LightningBoltData {
    match &mut e.kind {
        EntityKindData::LightningBolt(_, d) => d,
        _ => panic!("EntityLightningBolt method on another kind"),
    }
}

/// Fire at `(x, y, z)` if it is air and fire may stand there.
// src: EntityLightningBolt.java:20-22, :28-30, :55-57
fn ignite(world: &mut World, x: i32, y: i32, z: i32) {
    if world.get_block_id(x, y, z) == 0
        && crate::block::dispatch::can_place_block_at(world, FIRE_ID, x, y, z)
    {
        world.set_block_with_notify(x, y, z, FIRE_ID);
    }
}

/// `new EntityLightningBolt(world, x, y, z)` (World.java:1927): at `(x, y,
/// z)` with yaw and pitch 0, `lightningState` 2, `boltVertex =
/// rand.nextLong()`, `boltLivingTime = rand.nextInt(3) + 1`; at difficulty 2
/// or more, when the chunks within 10 blocks exist, fire at the strike block
/// and at four `rand`-jittered blocks around it.
// src: EntityLightningBolt.java:10-34, EntityWeatherEffect.java:4-6
pub fn new_entity_lightning_bolt(world: &mut World, x: f64, y: f64, z: f64) -> Entity {
    let mut b = EntityBase::new(world);
    // entityInit() is empty (EntityLightningBolt.java:76-77).
    let mut d = LightningBoltData::default();
    b.set_location_and_angles(x, y, z, 0.0, 0.0);
    d.lightning_state = 2;
    d.bolt_vertex = b.rand.next_long();
    d.bolt_living_time = b.rand.next_int_bounded(3) + 1;
    // src: EntityLightningBolt.java:16-32
    if world.difficulty_setting >= 2
        && world.do_chunks_near_chunk_exist(floor_double(x), floor_double(y), floor_double(z), 10)
    {
        let var8 = floor_double(x);
        let var9 = floor_double(y);
        let var10 = floor_double(z);
        ignite(world, var8, var9, var10);
        for _ in 0..4 {
            let var9 = floor_double(x) + b.rand.next_int_bounded(3) - 1;
            let var10 = floor_double(y) + b.rand.next_int_bounded(3) - 1;
            let var11 = floor_double(z) + b.rand.next_int_bounded(3) - 1;
            ignite(world, var9, var10, var11);
        }
    }
    Entity {
        base: b,
        kind: EntityKindData::LightningBolt(WeatherEffectData::default(), d),
    }
}

/// `EntityLightningBolt.onUpdate()`.
// src: EntityLightningBolt.java:36-74
pub fn on_update(e: &mut Entity, world: &mut World) {
    base::on_update(e, world);
    // src: EntityLightningBolt.java:38-41
    if data(e).lightning_state == 2 {
        let (x, y, z) = (e.base.pos_x, e.base.pos_y, e.base.pos_z);
        let p1 = 0.8 + e.base.rand.next_float() * 0.2;
        world.play_sound_effect(x, y, z, "ambient.weather.thunder", 10000.0, p1);
        let p2 = 0.5 + e.base.rand.next_float() * 0.2;
        world.play_sound_effect(x, y, z, "random.explode", 2.0, p2);
    }
    // src: EntityLightningBolt.java:43-60
    data(e).lightning_state -= 1;
    if data(e).lightning_state < 0 {
        if data(e).bolt_living_time == 0 {
            dispatch::set_entity_dead(e, world);
        } else {
            let state = data(e).lightning_state;
            if state < -e.base.rand.next_int_bounded(10) {
                let v = e.base.rand.next_long();
                let d = data(e);
                d.bolt_living_time -= 1;
                d.lightning_state = 1;
                d.bolt_vertex = v;
                let (x, y, z) = (
                    floor_double(e.base.pos_x),
                    floor_double(e.base.pos_y),
                    floor_double(e.base.pos_z),
                );
                if world.do_chunks_near_chunk_exist(x, y, z, 10) {
                    ignite(world, x, y, z);
                }
            }
        }
    }
    // src: EntityLightningBolt.java:62-72
    if data(e).lightning_state >= 0 {
        let var6 = 3.0;
        let b = &e.base;
        let bb = Aabb::new(
            b.pos_x - var6,
            b.pos_y - var6,
            b.pos_z - var6,
            b.pos_x + var6,
            b.pos_y + 6.0 + var6,
            b.pos_z + var6,
        );
        let var7 = world.get_entities_within_aabb_excluding_entity(Some(b.entity_id), &bb);
        for id in var7 {
            let bolt: &Entity = e;
            if world
                .with_entity_mut(id, |w, var5| {
                    dispatch::on_struck_by_lightning(var5, w, bolt)
                })
                .is_none()
            {
                world.note_unimplemented(
                    "EntityLightningBolt.onUpdate: struck entity out of its slot",
                );
            }
        }
        world.field_27172_i = 2;
    }
}
