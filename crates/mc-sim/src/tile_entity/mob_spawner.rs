//! `TileEntityMobSpawner` (TileEntityMobSpawner.java): with a player within
//! 16 blocks, counts `delay` down and then tries to spawn four mobs.
//!
//! The mobs are `EntityList.createEntityInWorld(mobID, world)`
//! ([`entity_list::create_entity_in_world`]). For a mob whose `(World)`
//! constructor the port does not have yet, `createEntityInWorld` makes the
//! constructor's world side effects (entity id, `Math.random()` draws),
//! records the gap and answers `None`, and the update returns as Java does
//! for an unknown id; from that attempt on it diverges from vanilla.

use std::collections::BTreeMap;

use mc_core::Aabb;
use mc_nbt::Tag;

use super::{get_short, get_string};
use crate::entity::{dispatch, living};
use crate::item::JavaException;
use crate::save::entity_list;
use crate::world::World;
use crate::world::tile_entity::{TileEntityKindData, TileEntityRef};

/// The fields `TileEntityMobSpawner` declares.
#[derive(Clone, Debug, PartialEq)]
pub struct TileEntityMobSpawner {
    /// `delay`: ticks until the next spawn attempt; -1 means "draw one".
    // src: TileEntityMobSpawner.java:4
    pub delay: i32,
    /// `mobID` (private): the `EntityList` name to spawn.
    // src: TileEntityMobSpawner.java:5
    pub mob_id: String,
    /// `yaw`: the spinning mob's angle (renderer).
    // src: TileEntityMobSpawner.java:6
    pub yaw: f64,
    /// `yaw2`: the previous tick's `yaw` (renderer interpolation).
    // src: TileEntityMobSpawner.java:7
    pub yaw2: f64,
}

impl Default for TileEntityMobSpawner {
    /// `new TileEntityMobSpawner()`: the initialiser's `delay = -1`, then
    /// the constructor's `delay = 20`.
    // src: TileEntityMobSpawner.java:4-11
    fn default() -> Self {
        TileEntityMobSpawner {
            delay: 20,
            mob_id: "Pig".to_string(),
            yaw: 0.0,
            yaw2: 0.0,
        }
    }
}

impl TileEntityMobSpawner {
    /// `getMobID()`.
    // src: TileEntityMobSpawner.java:13-15
    pub fn get_mob_id(&self) -> &str {
        &self.mob_id
    }

    /// `setMobID(id)` (WorldGenDungeons.java:100 sets the dungeon's mob).
    // src: TileEntityMobSpawner.java:17-19
    pub fn set_mob_id(&mut self, id: String) {
        self.mob_id = id;
    }

    /// `readFromNBT(compound)` after `super.readFromNBT`: `EntityId`
    /// (string), `Delay` (short).
    // src: TileEntityMobSpawner.java:93-97
    pub fn read_from_nbt(&mut self, c: &BTreeMap<String, Tag>) -> Result<(), JavaException> {
        self.mob_id = get_string(c, "EntityId")?;
        self.delay = get_short(c, "Delay")? as i32;
        Ok(())
    }

    /// `writeToNBT(compound)` after `super.writeToNBT`: `EntityId`, `Delay`
    /// (`(short)delay`).
    // src: TileEntityMobSpawner.java:99-103
    pub fn write_to_nbt(&self, c: &mut BTreeMap<String, Tag>) {
        c.insert("EntityId".to_string(), Tag::String(self.mob_id.clone()));
        c.insert("Delay".to_string(), Tag::Short(self.delay as i16));
    }
}

fn data(world: &mut World, r: TileEntityRef) -> Option<&mut TileEntityMobSpawner> {
    match &mut world.tile_entities.get_mut(&r)?.kind {
        TileEntityKindData::MobSpawner(d) => Some(d),
        _ => None,
    }
}

/// `anyPlayerInRange()`: a player within 16 blocks of the block's centre.
// src: TileEntityMobSpawner.java:21-23
pub fn any_player_in_range(world: &World, x: i32, y: i32, z: i32) -> bool {
    world
        .get_closest_player(x as f64 + 0.5, y as f64 + 0.5, z as f64 + 0.5, 16.0)
        .is_some()
}

/// `updateDelay()` (private): 200-799 ticks from `World.rand`.
// src: TileEntityMobSpawner.java:89-91
fn update_delay(world: &mut World, r: TileEntityRef) {
    let v = 200 + world.rand.next_int_bounded(600);
    if let Some(d) = data(world, r) {
        d.delay = v;
    }
}

/// `updateEntity()` (module docs for the spawn attempt).
// src: TileEntityMobSpawner.java:25-87
pub fn update_entity(world: &mut World, r: TileEntityRef) {
    let Some(t) = world.tile_entities.get(&r) else {
        return;
    };
    let (x, y, z) = (t.x_coord, t.y_coord, t.z_coord);
    if let Some(d) = data(world, r) {
        d.yaw2 = d.yaw;
    }
    if !any_player_in_range(world, x, y, z) {
        return;
    }
    // src: TileEntityMobSpawner.java:28-32 (float sums, widened)
    let var1 = (x as f32 + world.rand.next_float()) as f64;
    let var3 = (y as f32 + world.rand.next_float()) as f64;
    let var5 = (z as f32 + world.rand.next_float()) as f64;
    world.spawn_particle("smoke", var1, var3, var5, 0.0, 0.0, 0.0);
    world.spawn_particle("flame", var1, var3, var5, 0.0, 0.0, 0.0);
    // src: TileEntityMobSpawner.java:34-36: the `for` initialiser adds the
    // step, the body subtracts 360 from `yaw`, the update from `yaw2`.
    if let Some(d) = data(world, r) {
        d.yaw += (1000.0f32 / (d.delay as f32 + 200.0f32)) as f64;
        while d.yaw > 360.0 {
            d.yaw -= 360.0;
            d.yaw2 -= 360.0;
        }
    }
    if !world.multiplayer_world {
        if data(world, r).is_some_and(|d| d.delay == -1) {
            update_delay(world, r);
        }
        if let Some(d) = data(world, r)
            && d.delay > 0
        {
            d.delay -= 1;
            return;
        }
        // src: TileEntityMobSpawner.java:48-82: four attempts, each
        // constructing the mob first.
        let mob_id = data(world, r).map(|d| d.mob_id.clone()).unwrap_or_default();
        for _var8 in 0..4 {
            // `var9 == null`: an id EntityList does not know, or "Mob", the
            // abstract `EntityLiving` (`newInstance` throws
            // `InstantiationException`, caught by createEntityInWorld,
            // EntityList.java:27-29), or a class the port cannot construct
            // yet (recorded by entity_list).
            // src: TileEntityMobSpawner.java:51-54
            let Some(mut var9) = entity_list::create_entity_in_world(&mob_id, world) else {
                return;
            };
            if var9.kind.living().is_none() {
                // The `(EntityLiving)` cast throws `ClassCastException` in
                // vanilla for Arrow, Snowball, Item, Painting, PrimedTnt,
                // FallingSand, Minecart and Boat (out of the tile entity's
                // update, World.updateEntities); unreachable from
                // `WorldGenDungeons.pickMobSpawner` (Skeleton/Zombie/Spider).
                world.note_unimplemented(
                    "TileEntityMobSpawner.updateEntity: ClassCastException (a non-living EntityList id)",
                );
                return;
            }
            // src: TileEntityMobSpawner.java:56-60 (`getClass()`: instances of
            // the mob's own class)
            let bb = Aabb::new(
                x as f64,
                y as f64,
                z as f64,
                (x + 1) as f64,
                (y + 1) as f64,
                (z + 1) as f64,
            )
            .expand(8.0, 4.0, 8.0);
            let var10 = world
                .get_entities_within_aabb(var9.kind.java_class(), &bb, None)
                .len();
            if var10 >= 6 {
                update_delay(world, r);
                return;
            }
            // src: TileEntityMobSpawner.java:62-66
            let var11 = x as f64 + (world.rand.next_double() - world.rand.next_double()) * 4.0;
            let var13 = (y + world.rand.next_int_bounded(3) - 1) as f64;
            let var15 = z as f64 + (world.rand.next_double() - world.rand.next_double()) * 4.0;
            let yaw = world.rand.next_float() * 360.0;
            var9.base
                .set_location_and_angles(var11, var13, var15, yaw, 0.0);
            // src: TileEntityMobSpawner.java:67-80
            if dispatch::get_can_spawn_here(&mut var9, world) {
                let id = var9.base.entity_id;
                world.spawn_entity_in_world(var9);
                for _var17 in 0..20 {
                    let var1 = x as f64 + 0.5 + (world.rand.next_float() as f64 - 0.5) * 2.0;
                    let var3 = y as f64 + 0.5 + (world.rand.next_float() as f64 - 0.5) * 2.0;
                    let var5 = z as f64 + 0.5 + (world.rand.next_float() as f64 - 0.5) * 2.0;
                    world.spawn_particle("smoke", var1, var3, var5, 0.0, 0.0, 0.0);
                    world.spawn_particle("flame", var1, var3, var5, 0.0, 0.0, 0.0);
                }
                // `var9.spawnExplosionParticle()`: on the joined mob (its
                // `rand`, reseeded by the join). A mob whose chunk is not
                // loaded did not join and is dropped here; vanilla still
                // draws its 20 particles from its own `rand`, which nothing
                // reads afterwards.
                if world
                    .with_entity_mut(id, |w, e| living::spawn_explosion_particle(e, w))
                    .is_none()
                {
                    world.note_unimplemented(
                        "TileEntityMobSpawner.updateEntity: spawnExplosionParticle on a mob that did not join",
                    );
                }
                update_delay(world, r);
            }
        }
    }
    // super.updateEntity() (TileEntityMobSpawner.java:85): empty.
}
