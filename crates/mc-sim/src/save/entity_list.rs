//! `EntityList` (EntityList.java): the saved-name ↔ class ↔ network-id table
//! as data, and `createEntityFromNBT` / `createEntityInWorld` for the
//! entity kinds the port can construct.
//!
//! `createEntityFromNBT` instantiates the class saved under `id` with its
//! `(World)` constructor, then `readFromNBT`. The port's `(World)`
//! constructors are the rows of [`CONSTRUCTORS`] (one line per class; every
//! caller — [`create_entity_in_world`], [`create_entity_from_nbt`],
//! [`new_instance`] for `SpawnerAnimals`, the mob spawner, the golden
//! driver's `spawn` op — picks it up). Every concrete class of [`MAPPINGS`]
//! has its row since the WP-2.3 merges ([`missing_constructors`] is empty,
//! which `tests/squid_spawning.rs` and `tests/golden_save.rs` assert); the
//! gap path that kept the stored compound of an unconstructible class was
//! removed as dead code by WP-4.8 (Gate 2 R4, DEFERRED_REVIEW §3.1 D).
//!
//! `"Mob"` maps the abstract `EntityLiving` (EntityList.java:87): its
//! `newInstance` throws `InstantiationException`, caught, so vanilla skips
//! the entity like an unknown id ([`FromNbt::Skipped`], dropped on re-save,
//! no id consumed). The player is not in the table: `EntityPlayer`s are
//! never saved in chunks (`getEntityString` is null, so `addEntityID`
//! returns false, Entity.java:780-789).

use super::SaveError;
use super::nbt::{self as n, Compound};
use crate::entity::arrow::new_entity_arrow_empty;
use crate::entity::boat::new_entity_boat_empty;
use crate::entity::chicken::new_entity_chicken;
use crate::entity::cow::new_entity_cow;
use crate::entity::creeper::new_entity_creeper;
use crate::entity::falling_sand::new_entity_falling_sand_empty;
use crate::entity::ghast::new_entity_ghast;
use crate::entity::giant_zombie::new_entity_giant_zombie;
use crate::entity::item::new_entity_item_empty;
use crate::entity::minecart::new_entity_minecart_empty;
use crate::entity::mob::new_entity_mob;
use crate::entity::painting::new_entity_painting_empty;
use crate::entity::pig::new_entity_pig;
use crate::entity::pig_zombie::new_entity_pig_zombie;
use crate::entity::sheep::new_entity_sheep;
use crate::entity::skeleton::new_entity_skeleton;
use crate::entity::slime::new_entity_slime;
use crate::entity::snowball::new_entity_snowball_empty;
use crate::entity::spider::new_entity_spider;
use crate::entity::squid::new_entity_squid;
use crate::entity::tnt_primed::new_entity_tnt_primed_empty;
use crate::entity::wolf::new_entity_wolf;
use crate::entity::zombie::new_entity_zombie;
use crate::entity::{Entity, EntityKindData, nbt as entity_nbt};
use crate::world::World;

/// One `addMapping(class, name, id)` row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntityMapping {
    /// The Java class's simple name.
    pub class: &'static str,
    /// The saved name (`id` in the entity compound).
    pub name: &'static str,
    /// The network id (`Packet24MobSpawn` etc.; multiplayer, D3).
    pub id: i32,
    /// Whether the class can be instantiated (`EntityLiving` is abstract).
    pub concrete: bool,
    /// Whether it extends `EntityLiving` (its constructor draws
    /// `Math.random()`, EntityLiving.java:38, :63, :65, :66).
    pub living: bool,
}

/// One `addMapping(class, name, id)` row: `row!(class, name, id, living)`
/// for a concrete class.
macro_rules! row {
    ($class:literal, $name:literal, $id:expr, $living:expr) => {
        row!($class, $name, $id, $living, true)
    };
    ($class:literal, $name:literal, $id:expr, $living:expr, $concrete:expr) => {
        EntityMapping {
            class: $class,
            name: $name,
            id: $id,
            concrete: $concrete,
            living: $living,
        }
    };
}

/// `addMapping` calls of `EntityList`'s static initialiser, in order.
// src: EntityList.java:82-107
pub const MAPPINGS: [EntityMapping; 24] = [
    row!("EntityArrow", "Arrow", 10, false),
    row!("EntitySnowball", "Snowball", 11, false),
    row!("EntityItem", "Item", 1, false),
    row!("EntityPainting", "Painting", 9, false),
    // src: EntityLiving.java:6 (`public abstract class EntityLiving`)
    row!("EntityLiving", "Mob", 48, true, false),
    row!("EntityMob", "Monster", 49, true),
    row!("EntityCreeper", "Creeper", 50, true),
    row!("EntitySkeleton", "Skeleton", 51, true),
    row!("EntitySpider", "Spider", 52, true),
    row!("EntityGiantZombie", "Giant", 53, true),
    row!("EntityZombie", "Zombie", 54, true),
    row!("EntitySlime", "Slime", 55, true),
    row!("EntityGhast", "Ghast", 56, true),
    row!("EntityPigZombie", "PigZombie", 57, true),
    row!("EntityPig", "Pig", 90, true),
    row!("EntitySheep", "Sheep", 91, true),
    row!("EntityCow", "Cow", 92, true),
    row!("EntityChicken", "Chicken", 93, true),
    row!("EntitySquid", "Squid", 94, true),
    row!("EntityWolf", "Wolf", 95, true),
    row!("EntityTNTPrimed", "PrimedTnt", 20, false),
    row!("EntityFallingSand", "FallingSand", 21, false),
    row!("EntityMinecart", "Minecart", 40, false),
    row!("EntityBoat", "Boat", 41, false),
];

/// `stringToClassMapping.get(name)`.
// src: EntityList.java:7, :13
pub fn by_name(name: &str) -> Option<&'static EntityMapping> {
    MAPPINGS.iter().find(|m| m.name == name)
}

/// `classToStringMapping.get(entity.getClass())`: `None` for a class
/// without a mapping (the player, the egg, the fireball, the fishing bobber,
/// lightning).
// src: EntityList.java:78-80
pub fn get_entity_string(kind: &EntityKindData) -> Option<&'static str> {
    let class = kind.java_class();
    MAPPINGS.iter().find(|m| m.class == class).map(|m| m.name)
}

/// `getEntityID(entity)`: `None` where the Java unboxes a null `Integer`
/// (`NullPointerException`).
// src: EntityList.java:74-76
pub fn get_entity_id(kind: &EntityKindData) -> Option<i32> {
    let class = kind.java_class();
    MAPPINGS.iter().find(|m| m.class == class).map(|m| m.id)
}

/// What [`create_entity_from_nbt`] made of a compound.
#[derive(Debug)]
pub enum FromNbt {
    /// The entity, read from the compound (not in any list yet).
    Created(Box<Entity>),
    /// No class (unknown id, or the abstract `"Mob"`): vanilla prints
    /// "Skipping Entity with id …" and drops it.
    Skipped,
}

/// A Java `(World)` constructor: `new <class>(world)`.
pub type Constructor = fn(&mut World) -> Entity;

/// The `(World)` constructors the port has, by Java class: what
/// `Class.getConstructor(World.class).newInstance(world)` runs for the
/// classes of [`MAPPINGS`]. One row per class; a task that transcribes a
/// constructor adds its row here and nothing else.
pub const CONSTRUCTORS: &[(&str, Constructor)] = &[
    // src: EntityArrow.java:18-21
    ("EntityArrow", new_entity_arrow_empty),
    // src: EntitySnowball.java:16-19
    ("EntitySnowball", new_entity_snowball_empty),
    // src: EntityItem.java:27-31
    ("EntityItem", new_entity_item_empty),
    // src: EntityPainting.java:14-20
    ("EntityPainting", new_entity_painting_empty),
    // src: EntityPig.java:4-8
    ("EntityPig", new_entity_pig),
    // src: EntitySheep.java:8-12
    ("EntitySheep", new_entity_sheep),
    // src: EntityCow.java:4-8
    ("EntityCow", new_entity_cow),
    // src: EntityChicken.java:12-18
    ("EntityChicken", new_entity_chicken),
    // src: EntityWolf.java:15-21
    ("EntityWolf", new_entity_wolf),
    // src: EntityMob.java:6-9
    ("EntityMob", new_entity_mob),
    // src: EntityCreeper.java:7-10
    ("EntityCreeper", new_entity_creeper),
    // src: EntitySkeleton.java:6-9
    ("EntitySkeleton", new_entity_skeleton),
    // src: EntitySpider.java:4-9
    ("EntitySpider", new_entity_spider),
    // src: EntityGiantZombie.java:4-12
    ("EntityGiantZombie", new_entity_giant_zombie),
    // src: EntityZombie.java:4-9
    ("EntityZombie", new_entity_zombie),
    // src: EntitySlime.java:8-15 (its size and jump delay from rand)
    ("EntitySlime", new_entity_slime),
    // src: EntityGhast.java:13-18
    ("EntityGhast", new_entity_ghast),
    // src: EntityPigZombie.java:10-16
    ("EntityPigZombie", new_entity_pig_zombie),
    // src: EntitySquid.java:19-24
    ("EntitySquid", new_entity_squid),
    // src: EntityTNTPrimed.java:6-12
    ("EntityTNTPrimed", new_entity_tnt_primed_empty),
    // src: EntityFallingSand.java:7-9
    ("EntityFallingSand", new_entity_falling_sand_empty),
    // src: EntityMinecart.java:26-36
    ("EntityMinecart", new_entity_minecart_empty),
    // src: EntityBoat.java:19-27
    ("EntityBoat", new_entity_boat_empty),
];

/// The port's `(World)` constructor of `class`, if it has one.
pub fn constructor(class: &str) -> Option<Constructor> {
    CONSTRUCTORS
        .iter()
        .find(|(c, _)| *c == class)
        .map(|(_, f)| *f)
}

/// The `EntityList` names whose class is concrete but has no row in
/// [`CONSTRUCTORS`], in `MAPPINGS` order: empty (every entity constructor
/// is transcribed); kept as the invariant the tests assert.
pub fn missing_constructors() -> Vec<&'static str> {
    MAPPINGS
        .iter()
        .filter(|m| m.concrete && constructor(m.class).is_none())
        .map(|m| m.name)
        .collect()
}

/// `classToStringMapping`'s key side: the row of the Java class `class`.
// src: EntityList.java:8, :14
pub fn by_class(class: &str) -> Option<&'static EntityMapping> {
    MAPPINGS.iter().find(|m| m.class == class)
}

/// `new <class>(world)` for the classes the port has, `None` otherwise.
fn construct(class: &str, world: &mut World) -> Option<Entity> {
    constructor(class).map(|f| f(world))
}

/// `entityClass.getConstructor(World.class).newInstance(world)` for the
/// concrete class `class` of [`MAPPINGS`] (`SpawnerAnimals` holds classes,
/// not names: SpawnerAnimals.java:231, :329). `None`, recorded, for a class
/// outside `EntityList`'s table or without a constructor row (neither
/// happens: the callers pass table classes, and every row exists).
pub fn new_instance(class: &str, world: &mut World) -> Option<Entity> {
    let e = construct(class, world);
    if e.is_none() {
        world.note_unimplemented("EntityList: a class outside EntityList's table");
    }
    e
}

/// `EntityList.createEntityInWorld(name, world)`: a new entity of the class
/// saved as `name`, `None` for an unknown name or the abstract `"Mob"`
/// (`newInstance` throws `InstantiationException`, caught).
// src: EntityList.java:19-32
pub fn create_entity_in_world(name: &str, world: &mut World) -> Option<Entity> {
    let m = by_name(name)?;
    if !m.concrete {
        return None;
    }
    new_instance(m.class, world)
}

/// `EntityList.createEntityFromNBT(compound, world)`. `Err` where the Java
/// throws out of the method: `id` not a string (the second `getString`,
/// outside the `try`, EntityList.java:49), or `readFromNBT` failing (outside
/// the `try`, :47).
// src: EntityList.java:34-53
pub fn create_entity_from_nbt(c: &Compound, world: &mut World) -> Result<FromNbt, SaveError> {
    let name = n::get_string(c, "id")?;
    // Every concrete class has a constructor ([`missing_constructors`]).
    let Some(mut e) = by_name(&name)
        .filter(|m| m.concrete)
        .and_then(|m| construct(m.class, world))
    else {
        // src: EntityList.java:49 (stdout only)
        return Ok(FromNbt::Skipped);
    };
    entity_nbt::read_from_nbt(&mut e, c, world)?; // src: EntityList.java:47
    Ok(FromNbt::Created(Box::new(e)))
}
