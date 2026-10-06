//! `TileEntityMobSpawner` (TileEntityMobSpawner.java): with a player within
//! 16 blocks, counts `delay` down and then tries to spawn four mobs.
//!
//! The mobs are `EntityList.createEntityInWorld(mobID, world)` (Phase 2:
//! the living-entity constructors are WP-2.3). The port makes every draw
//! of `updateEntity` up to the first construction and records
//! `"EntityList.createEntityInWorld"` there, then returns as Java does for
//! an unknown id; from the first spawn attempt on (the tick after `delay`
//! reaches 0) it therefore diverges from vanilla.

use std::collections::BTreeMap;

use mc_nbt::Tag;

use super::{get_short, get_string};
use crate::item::JavaException;
use crate::world::World;
use crate::world::tile_entity::{TileEntityKindData, TileEntityRef};

/// `EntityList`'s name table (EntityList.java:83-106): the ids
/// `createEntityInWorld` can construct.
// src: EntityList.java:83-106
const ENTITY_LIST_NAMES: [&str; 24] = [
    "Arrow",
    "Snowball",
    "Item",
    "Painting",
    "Mob",
    "Monster",
    "Creeper",
    "Skeleton",
    "Spider",
    "Giant",
    "Zombie",
    "Slime",
    "Ghast",
    "PigZombie",
    "Pig",
    "Sheep",
    "Cow",
    "Chicken",
    "Squid",
    "Wolf",
    "PrimedTnt",
    "FallingSand",
    "Minecart",
    "Boat",
];

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
        // src: TileEntityMobSpawner.java:48-50: four attempts, each
        // constructing the mob first.
        let mob_id = data(world, r).map(|d| d.mob_id.clone()).unwrap_or_default();
        // `var9 == null` (an id EntityList does not know): return.
        if ENTITY_LIST_NAMES.contains(&mob_id.as_str()) {
            // src: TileEntityMobSpawner.java:51 (`EntityList.createEntityInWorld`,
            // then `getEntitiesWithinAABB`, the placement draws, `getCanSpawnHere`,
            // `entityJoinedWorld`, 20 particle pairs, `spawnExplosionParticle`,
            // `updateDelay`, :56-80): the mob constructors are WP-2.3.
            world.note_unimplemented("EntityList.createEntityInWorld");
        }
    }
    // super.updateEntity() (TileEntityMobSpawner.java:84): empty.
}
