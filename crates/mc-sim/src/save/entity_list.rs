//! `EntityList` (EntityList.java): the saved-name ↔ class ↔ network-id table
//! as data, and `createEntityFromNBT` / `createEntityInWorld` for the
//! entity kinds the port can construct.
//!
//! # Kinds the port cannot construct yet (recorded gap)
//!
//! `createEntityFromNBT` instantiates the class saved under `id` with its
//! `(World)` constructor, then `readFromNBT`. The port has those
//! constructors for `EntityItem`, `EntityFallingSand`, `EntityArrow`,
//! `EntitySnowball`, `EntityPainting` and `EntityTNTPrimed` (WP-2.4); the
//! mobs, the minecart and the boat are Phase 2. For the other names of the
//! table, [`create_entity_from_nbt`] returns [`FromNbt::Gap`]:
//!
//! - the side effects the Java constructor has on the world are made: the
//!   entity id is consumed (`Entity.nextEntityID++`, Entity.java:76) and,
//!   for `EntityLiving` subclasses, `EntityLiving`'s four `Math.random()`
//!   draws (EntityLiving.java:38, :63, :65, :66; no subclass constructor
//!   draws `Math.random()`), so ids and the `Math.random()` stream stay
//!   aligned with vanilla;
//! - the gap is recorded in [`World::unimplemented_hits`];
//! - the stored compound itself is kept by the chunk loader and written
//!   back, unchanged, after the chunk's live entities when the chunk is
//!   saved again ([`super::chunk_loader`]), so loading and re-saving a
//!   world never loses a pig. Vanilla would have re-written the pig's
//!   current state in its section's list order; the port writes the stored
//!   state at the end of the list. Nothing ticks the entity meanwhile.
//!
//! `"Mob"` maps the abstract `EntityLiving` (EntityList.java:87): its
//! `newInstance` throws `InstantiationException`, caught, so vanilla skips
//! the entity like an unknown id ([`FromNbt::Skipped`], dropped on re-save,
//! no id consumed). The player is not in the table: `EntityPlayer`s are
//! never saved in chunks (`getEntityString` is null, so `addEntityID`
//! returns false, Entity.java:780-789).

use mc_nbt::Tag;

use super::SaveError;
use super::nbt::{self as n, Compound};
use crate::entity::arrow::new_entity_arrow_empty;
use crate::entity::falling_sand::new_entity_falling_sand_empty;
use crate::entity::item::new_entity_item_empty;
use crate::entity::painting::new_entity_painting_empty;
use crate::entity::snowball::new_entity_snowball_empty;
use crate::entity::tnt_primed::new_entity_tnt_primed_empty;
use crate::entity::{Entity, EntityBase, EntityKindData, LivingData, nbt as entity_nbt};
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
    /// `Math.random()`; module docs).
    pub living: bool,
}

const fn row(class: &'static str, name: &'static str, id: i32, living: bool) -> EntityMapping {
    EntityMapping {
        class,
        name,
        id,
        concrete: true,
        living,
    }
}

/// `addMapping` calls of `EntityList`'s static initialiser, in order.
// src: EntityList.java:82-107
pub const MAPPINGS: [EntityMapping; 24] = [
    row("EntityArrow", "Arrow", 10, false),
    row("EntitySnowball", "Snowball", 11, false),
    row("EntityItem", "Item", 1, false),
    row("EntityPainting", "Painting", 9, false),
    // src: EntityLiving.java:6 (`public abstract class EntityLiving`)
    EntityMapping {
        class: "EntityLiving",
        name: "Mob",
        id: 48,
        concrete: false,
        living: true,
    },
    row("EntityMob", "Monster", 49, true),
    row("EntityCreeper", "Creeper", 50, true),
    row("EntitySkeleton", "Skeleton", 51, true),
    row("EntitySpider", "Spider", 52, true),
    row("EntityGiantZombie", "Giant", 53, true),
    row("EntityZombie", "Zombie", 54, true),
    row("EntitySlime", "Slime", 55, true),
    row("EntityGhast", "Ghast", 56, true),
    row("EntityPigZombie", "PigZombie", 57, true),
    row("EntityPig", "Pig", 90, true),
    row("EntitySheep", "Sheep", 91, true),
    row("EntityCow", "Cow", 92, true),
    row("EntityChicken", "Chicken", 93, true),
    row("EntitySquid", "Squid", 94, true),
    row("EntityWolf", "Wolf", 95, true),
    row("EntityTNTPrimed", "PrimedTnt", 20, false),
    row("EntityFallingSand", "FallingSand", 21, false),
    row("EntityMinecart", "Minecart", 40, false),
    row("EntityBoat", "Boat", 41, false),
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
    /// A class the port cannot construct yet (module docs): its constructor's
    /// world side effects were made; keep the compound.
    Gap {
        /// The Java class's simple name.
        class: &'static str,
    },
    /// No class (unknown id, or the abstract `"Mob"`): vanilla prints
    /// "Skipping Entity with id …" and drops it.
    Skipped,
}

/// `new <class>(world)` for the classes the port has, `None` otherwise.
fn construct(class: &str, world: &mut World) -> Option<Entity> {
    match class {
        // src: EntityItem.java:27-31
        "EntityItem" => Some(new_entity_item_empty(world)),
        // src: EntityFallingSand.java:7-9
        "EntityFallingSand" => Some(new_entity_falling_sand_empty(world)),
        // src: EntityArrow.java:18-21
        "EntityArrow" => Some(new_entity_arrow_empty(world)),
        // src: EntitySnowball.java:16-19
        "EntitySnowball" => Some(new_entity_snowball_empty(world)),
        // src: EntityPainting.java:14-20
        "EntityPainting" => Some(new_entity_painting_empty(world)),
        // src: EntityTNTPrimed.java:6-12
        "EntityTNTPrimed" => Some(new_entity_tnt_primed_empty(world)),
        _ => None,
    }
}

/// The world side effects of `new <class>(world)` for a class the port
/// cannot construct (module docs).
fn construct_gap(m: &EntityMapping, world: &mut World) {
    // src: Entity.java:75-113 (the id), EntityLiving.java:6-68 (the draws)
    let mut b = EntityBase::new(world);
    if m.living {
        let _ = LivingData::new(world, &mut b);
    }
    world.note_unimplemented("EntityList.createEntityFromNBT: the entity class's (World) constructor and readFromNBT (Phase 2)");
}

/// `EntityList.createEntityInWorld(name, world)`: a new entity of the class
/// saved as `name`, `None` for an unknown name or a class the port cannot
/// construct yet (recorded).
// src: EntityList.java:19-32
pub fn create_entity_in_world(name: &str, world: &mut World) -> Option<Entity> {
    let m = by_name(name)?;
    if !m.concrete {
        return None;
    }
    let e = construct(m.class, world);
    if e.is_none() {
        construct_gap(m, world);
    }
    e
}

/// `EntityList.createEntityFromNBT(compound, world)`. `Err` where the Java
/// throws out of the method: `id` not a string (the second `getString`,
/// outside the `try`, EntityList.java:49), or `readFromNBT` failing (outside
/// the `try`, :47).
// src: EntityList.java:34-53
pub fn create_entity_from_nbt(c: &Compound, world: &mut World) -> Result<FromNbt, SaveError> {
    let name = n::get_string(c, "id")?;
    let Some(m) = by_name(&name).filter(|m| m.concrete) else {
        // src: EntityList.java:49 (stdout only)
        return Ok(FromNbt::Skipped);
    };
    match construct(m.class, world) {
        Some(mut e) => {
            entity_nbt::read_from_nbt(&mut e, c, world)?; // src: EntityList.java:47
            Ok(FromNbt::Created(Box::new(e)))
        }
        None => {
            construct_gap(m, world);
            Ok(FromNbt::Gap { class: m.class })
        }
    }
}

/// The stored compound of a [`FromNbt::Gap`] entity, as kept for re-saving.
pub fn preserved(c: &Compound) -> Tag {
    Tag::Compound(c.clone())
}
