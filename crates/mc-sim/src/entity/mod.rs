//! Entities (`Entity.java` and its subclasses; PLAN_v4 §3.2).
//!
//! # Data layout: one struct per Java class level
//!
//! An [`Entity`] is `{ base: EntityBase, kind: EntityKindData }`. `EntityBase`
//! holds every field `Entity.java` declares. [`EntityKindData`] has one variant
//! per *concrete* entity class the singleplayer game can instantiate, and a
//! variant is a tuple of one data struct per class in its inheritance chain
//! below `Entity`, top-down:
//!
//! ```text
//! EntityPig extends EntityAnimal extends EntityCreature extends EntityLiving extends Entity
//!   => EntityKindData::Pig(LivingData, CreatureData, AnimalData, PigData)
//! ```
//!
//! Every class level gets its struct even when it declares no fields yet
//! (`AnimalData` is empty), so a variant's shape always mirrors the Java chain
//! and adding a field later never changes a pattern. Code that works at one
//! level reaches its struct with an accessor ([`EntityKindData::living`],
//! [`EntityKindData::living_mut`], …) or by matching the variant.
//! The alternative — nesting (`Pig(Animal<Creature<Living<PigData>>>)`) —
//! was rejected: deep generic nesting makes patterns and field paths
//! (`pig.sub.sub.sub`) harder to read than the flat tuple.
//!
//! # Behaviour: one function per class level, plus a dispatcher
//!
//! A Java method `Foo.bar` becomes a function `bar` in the module of class
//! `Foo` ([`base`] for `Entity`, [`living`] for `EntityLiving`, [`item`] for
//! `EntityItem`, [`falling_sand`] for `EntityFallingSand`, …), taking `(e: &mut Entity, world: &mut World, …)`.
//!
//! - `super.bar()` in Java is a direct call to the parent level's function
//!   (`living::on_update` calls `base::on_update`).
//! - A virtual call `this.bar()` (or `other.bar()`) goes through
//!   [`dispatch`]`::bar`, which `match`es on the kind and calls the most
//!   derived level that overrides `bar`. Every override that is not yet
//!   transcribed has its own arm that runs the nearest transcribed ancestor
//!   and records `"Class.method"` in [`World::unimplemented_hits`](crate::world::World::unimplemented_hits), so the
//!   dispatch tables double as a list of the remaining Phase 2 work.
//!
//! # References between entities
//!
//! A Java field holding an entity (`ridingEntity`, `riddenByEntity`,
//! `EntityLiving.currentTarget`, …) holds its `entityId` here (`Option<i32>`).
//! A Java *parameter* of entity type becomes `&Entity` / `&mut Entity`, never
//! an id, because the entity being updated is out of its slot (below) and
//! cannot be found through the world.
//!
//! # The take/put-back driver
//!
//! `World.loadedEntityList` is a `Vec<EntitySlot>` in Java list order; each
//! slot keeps the entity id and an `Option<Box<Entity>>`. To run entity code
//! that needs `&mut World`, the driver takes the entity out of its slot,
//! calls the code with the entity and the world as separate borrows, then puts
//! it back ([`World::update_entities`](crate::world::World::update_entities),
//! [`World::with_entity_mut`](crate::world::World::with_entity_mut)). While an
//! entity is out, world queries do not see it (`World::entity` returns
//! `None`; entity-in-box searches skip it). Vanilla code that would find the
//! updating entity through the world receives it as a parameter instead; a
//! transcriber who meets a query that must include it reports it.
//!
//! # Out of scope
//!
//! `EntityFX` and its subclasses (particles) belong to the client (WP-3.7).
//! `EntityClientPlayerMP` and `EntityOtherPlayerMP` are multiplayer (D3).

pub mod animal;
pub mod arrow;
pub mod base;
pub mod boat;
pub mod creature;
pub mod data_watcher;
pub mod dispatch;
pub mod egg;
pub mod falling_sand;
pub mod fireball;
pub mod fish;
pub mod item;
pub mod lightning_bolt;
pub mod living;
pub mod minecart;
pub mod nbt;
pub mod painting;
pub mod pathfinding;
pub mod pig;
pub mod player;
pub mod snowball;
pub mod tnt_primed;

pub use animal::AnimalData;
pub use arrow::ArrowData;
pub use base::EntityBase;
pub use boat::BoatData;
pub use creature::CreatureData;
pub use egg::EggData;
pub use falling_sand::FallingSandData;
pub use fireball::FireballData;
pub use fish::FishData;
pub use item::ItemData;
pub use lightning_bolt::LightningBoltData;
pub use living::LivingData;
pub use minecart::MinecartData;
pub use painting::{EnumArt, PaintingData};
pub use pig::PigData;
pub use snowball::{SnowballData, ThrowableData};
pub use tnt_primed::TntPrimedData;

use crate::inventory::InventoryPlayer;

/// An entity: `Entity.java`'s fields plus the subclass data.
#[derive(Clone, Debug)]
pub struct Entity {
    /// The fields declared by `Entity`.
    pub base: EntityBase,
    /// The concrete class and the fields of every class below `Entity`.
    pub kind: EntityKindData,
}

impl Entity {
    /// `entityId`.
    pub fn id(&self) -> i32 {
        self.base.entity_id
    }
}

/// A loaded-entity list slot: the entity id (always readable) and the entity
/// itself (`None` while taken out for an update).
#[derive(Clone, Debug)]
pub struct EntitySlot {
    /// `entity.entityId`.
    pub entity_id: i32,
    /// The entity, unless it is currently taken out.
    pub entity: Option<Box<Entity>>,
}

macro_rules! empty_level {
    ($(#[$m:meta] $name:ident),* $(,)?) => {
        $(
            #[$m]
            ///
            /// No fields transcribed yet (Phase 2).
            #[derive(Clone, Debug, Default, PartialEq)]
            pub struct $name {}
        )*
    };
}

empty_level!(
    /// `EntityMob` (EntityMob.java).
    MobData,
    /// `EntityWaterMob` (EntityWaterMob.java).
    WaterMobData,
    /// `EntityFlying` (EntityFlying.java).
    FlyingData,
    /// `EntityWeatherEffect` (EntityWeatherEffect.java).
    WeatherEffectData,
    /// `EntityCreeper` (EntityCreeper.java).
    CreeperData,
    /// `EntitySkeleton` (EntitySkeleton.java).
    SkeletonData,
    /// `EntitySpider` (EntitySpider.java).
    SpiderData,
    /// `EntityZombie` (EntityZombie.java).
    ZombieData,
    /// `EntityPigZombie` (EntityPigZombie.java).
    PigZombieData,
    /// `EntityGiantZombie` (EntityGiantZombie.java).
    GiantZombieData,
    /// `EntitySlime` (EntitySlime.java).
    SlimeData,
    /// `EntityGhast` (EntityGhast.java).
    GhastData,
    /// `EntitySheep` (EntitySheep.java).
    SheepData,
    /// `EntityCow` (EntityCow.java).
    CowData,
    /// `EntityChicken` (EntityChicken.java).
    ChickenData,
    /// `EntityWolf` (EntityWolf.java).
    WolfData,
    /// `EntitySquid` (EntitySquid.java).
    SquidData,
    /// `EntityPlayerSP` (EntityPlayerSP.java).
    PlayerSpData,
);

/// `EntityPlayer` (EntityPlayer.java). Only the fields the world reads so
/// far (construction: [`player::new_entity_player_sp`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlayerData {
    /// `inventory` (EntityPlayer.java:7).
    pub inventory: InventoryPlayer,
    /// `username` (EntityPlayer.java:16): null for a fresh `EntityPlayer`;
    /// `EntityPlayerSP` copies the session's (client, EntityPlayerSP.java:18).
    pub username: Option<String>,
    /// `sleeping` (EntityPlayer.java:25, protected): `isPlayerSleeping()`.
    pub sleeping: bool,
    /// `sleepTimer` (EntityPlayer.java:27, private): `isPlayerFullyAsleep()`
    /// needs 100.
    pub sleep_timer: i32,
    /// `isSwinging` (EntityPlayer.java:14): set by `swingItem`, advanced by
    /// `updatePlayerActionState` (EntityPlayer.java:158-170, WP-2.5).
    pub is_swinging: bool,
    /// `swingProgressInt` (EntityPlayer.java:15).
    pub swing_progress_int: i32,
    /// `score` (EntityPlayer.java:11): saved by `EntityPlayerSP` only
    /// (EntityPlayerSP.java:98-106, [`nbt`]).
    pub score: i32,
    /// `dimension` (EntityPlayer.java:17): saved as `Dimension`
    /// (EntityPlayer.java:312, :329, [`nbt`]).
    pub dimension: i32,
    /// `playerSpawnCoordinate` (EntityPlayer.java:31, private): the bed
    /// spawn, saved as `SpawnX/Y/Z` when set (EntityPlayer.java:320-322,
    /// :332-336, [`nbt`]).
    pub player_spawn_coordinate: Option<mc_core::java_hash::ChunkCoordinates>,
    /// `fishEntity` (EntityPlayer.java:38), as the bobber's entity id: set by
    /// `new EntityFish(world, player)` (EntityFish.java:61), cleared when the
    /// bobber dies (EntityFish.java:148, :381). Read by
    /// `ItemFishingRod.onItemRightClick` (WP-2.6).
    pub fish_entity: Option<i32>,
}

impl PlayerData {
    /// `isPlayerSleeping()`.
    // src: EntityPlayer.java:686-688
    pub fn is_player_sleeping(&self) -> bool {
        self.sleeping
    }

    /// `isPlayerFullyAsleep()`.
    // src: EntityPlayer.java:690-692
    pub fn is_player_fully_asleep(&self) -> bool {
        self.sleeping && self.sleep_timer >= 100
    }
}

/// One variant per concrete entity class (module docs). The tuple lists the
/// class chain below `Entity`, top-down.
///
/// Variants differ a lot in size (`LivingData` is large, most projectiles
/// are empty); entities are boxed in their slots already, so the variants are
/// not boxed again.
#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum EntityKindData {
    /// `EntityItem`.
    Item(ItemData),
    /// `EntityArrow`.
    Arrow(ArrowData),
    /// `EntitySnowball`.
    Snowball(SnowballData),
    /// `EntityEgg`.
    Egg(EggData),
    /// `EntityFireball`.
    Fireball(FireballData),
    /// `EntityFish` (fishing bobber).
    Fish(FishData),
    /// `EntityPainting`.
    Painting(PaintingData),
    /// `EntityFallingSand`.
    FallingSand(FallingSandData),
    /// `EntityTNTPrimed`.
    TntPrimed(TntPrimedData),
    /// `EntityMinecart`.
    Minecart(MinecartData),
    /// `EntityBoat`.
    Boat(BoatData),
    /// `EntityLightningBolt` < `EntityWeatherEffect`.
    LightningBolt(WeatherEffectData, LightningBoltData),
    /// `EntityMob` ("Monster" in `EntityList`) < `EntityCreature` < `EntityLiving`.
    Mob(LivingData, CreatureData, MobData),
    /// `EntityCreeper` < `EntityMob` < `EntityCreature` < `EntityLiving`.
    Creeper(LivingData, CreatureData, MobData, CreeperData),
    /// `EntitySkeleton` < `EntityMob` < `EntityCreature` < `EntityLiving`.
    Skeleton(LivingData, CreatureData, MobData, SkeletonData),
    /// `EntitySpider` < `EntityMob` < `EntityCreature` < `EntityLiving`.
    Spider(LivingData, CreatureData, MobData, SpiderData),
    /// `EntityZombie` < `EntityMob` < `EntityCreature` < `EntityLiving`.
    Zombie(LivingData, CreatureData, MobData, ZombieData),
    /// `EntityPigZombie` < `EntityZombie` < `EntityMob` < `EntityCreature` < `EntityLiving`.
    PigZombie(LivingData, CreatureData, MobData, ZombieData, PigZombieData),
    /// `EntityGiantZombie` < `EntityMob` < `EntityCreature` < `EntityLiving`.
    GiantZombie(LivingData, CreatureData, MobData, GiantZombieData),
    /// `EntitySlime` < `EntityLiving`.
    Slime(LivingData, SlimeData),
    /// `EntityGhast` < `EntityFlying` < `EntityLiving`.
    Ghast(LivingData, FlyingData, GhastData),
    /// `EntityPig` < `EntityAnimal` < `EntityCreature` < `EntityLiving`.
    Pig(LivingData, CreatureData, AnimalData, PigData),
    /// `EntitySheep` < `EntityAnimal` < `EntityCreature` < `EntityLiving`.
    Sheep(LivingData, CreatureData, AnimalData, SheepData),
    /// `EntityCow` < `EntityAnimal` < `EntityCreature` < `EntityLiving`.
    Cow(LivingData, CreatureData, AnimalData, CowData),
    /// `EntityChicken` < `EntityAnimal` < `EntityCreature` < `EntityLiving`.
    Chicken(LivingData, CreatureData, AnimalData, ChickenData),
    /// `EntityWolf` < `EntityAnimal` < `EntityCreature` < `EntityLiving`.
    Wolf(LivingData, CreatureData, AnimalData, WolfData),
    /// `EntitySquid` < `EntityWaterMob` < `EntityCreature` < `EntityLiving`.
    Squid(LivingData, CreatureData, WaterMobData, SquidData),
    /// `EntityPlayerSP` < `EntityPlayer` < `EntityLiving` (the local player).
    PlayerSp(LivingData, PlayerData, PlayerSpData),
}

impl EntityKindData {
    /// The concrete Java class name (`getClass().getSimpleName()`), as the
    /// `entity_state` trace reports it.
    pub fn java_class(&self) -> &'static str {
        use EntityKindData as K;
        match self {
            K::Item(..) => "EntityItem",
            K::Arrow(..) => "EntityArrow",
            K::Snowball(..) => "EntitySnowball",
            K::Egg(..) => "EntityEgg",
            K::Fireball(..) => "EntityFireball",
            K::Fish(..) => "EntityFish",
            K::Painting(..) => "EntityPainting",
            K::FallingSand(..) => "EntityFallingSand",
            K::TntPrimed(..) => "EntityTNTPrimed",
            K::Minecart(..) => "EntityMinecart",
            K::Boat(..) => "EntityBoat",
            K::LightningBolt(..) => "EntityLightningBolt",
            K::Mob(..) => "EntityMob",
            K::Creeper(..) => "EntityCreeper",
            K::Skeleton(..) => "EntitySkeleton",
            K::Spider(..) => "EntitySpider",
            K::Zombie(..) => "EntityZombie",
            K::PigZombie(..) => "EntityPigZombie",
            K::GiantZombie(..) => "EntityGiantZombie",
            K::Slime(..) => "EntitySlime",
            K::Ghast(..) => "EntityGhast",
            K::Pig(..) => "EntityPig",
            K::Sheep(..) => "EntitySheep",
            K::Cow(..) => "EntityCow",
            K::Chicken(..) => "EntityChicken",
            K::Wolf(..) => "EntityWolf",
            K::Squid(..) => "EntitySquid",
            K::PlayerSp(..) => "EntityPlayerSP",
        }
    }

    /// The `EntityLiving` level, for every kind that extends it.
    pub fn living(&self) -> Option<&LivingData> {
        use EntityKindData as K;
        match self {
            K::Mob(l, ..)
            | K::Creeper(l, ..)
            | K::Skeleton(l, ..)
            | K::Spider(l, ..)
            | K::Zombie(l, ..)
            | K::PigZombie(l, ..)
            | K::GiantZombie(l, ..)
            | K::Slime(l, ..)
            | K::Ghast(l, ..)
            | K::Pig(l, ..)
            | K::Sheep(l, ..)
            | K::Cow(l, ..)
            | K::Chicken(l, ..)
            | K::Wolf(l, ..)
            | K::Squid(l, ..)
            | K::PlayerSp(l, ..) => Some(l),
            _ => None,
        }
    }

    /// The `EntityLiving` level, mutably.
    pub fn living_mut(&mut self) -> Option<&mut LivingData> {
        use EntityKindData as K;
        match self {
            K::Mob(l, ..)
            | K::Creeper(l, ..)
            | K::Skeleton(l, ..)
            | K::Spider(l, ..)
            | K::Zombie(l, ..)
            | K::PigZombie(l, ..)
            | K::GiantZombie(l, ..)
            | K::Slime(l, ..)
            | K::Ghast(l, ..)
            | K::Pig(l, ..)
            | K::Sheep(l, ..)
            | K::Cow(l, ..)
            | K::Chicken(l, ..)
            | K::Wolf(l, ..)
            | K::Squid(l, ..)
            | K::PlayerSp(l, ..) => Some(l),
            _ => None,
        }
    }

    /// The `EntityPlayer` level (`instanceof EntityPlayer`).
    pub fn player(&self) -> Option<&PlayerData> {
        match self {
            EntityKindData::PlayerSp(_, p, _) => Some(p),
            _ => None,
        }
    }

    /// The `EntityPlayer` level, mutably.
    pub fn player_mut(&mut self) -> Option<&mut PlayerData> {
        match self {
            EntityKindData::PlayerSp(_, p, _) => Some(p),
            _ => None,
        }
    }

    /// `instanceof EntityPlayer`.
    pub fn is_player(&self) -> bool {
        self.player().is_some()
    }

    /// The `EntityItem` level.
    pub fn item(&self) -> Option<&ItemData> {
        match self {
            EntityKindData::Item(d) => Some(d),
            _ => None,
        }
    }

    /// The `EntityItem` level, mutably.
    pub fn item_mut(&mut self) -> Option<&mut ItemData> {
        match self {
            EntityKindData::Item(d) => Some(d),
            _ => None,
        }
    }

    /// The `EntityFallingSand` level.
    pub fn falling_sand(&self) -> Option<&FallingSandData> {
        match self {
            EntityKindData::FallingSand(d) => Some(d),
            _ => None,
        }
    }

    /// The `EntityFallingSand` level, mutably.
    pub fn falling_sand_mut(&mut self) -> Option<&mut FallingSandData> {
        match self {
            EntityKindData::FallingSand(d) => Some(d),
            _ => None,
        }
    }

    /// The `EntityArrow` level.
    pub fn arrow(&self) -> Option<&ArrowData> {
        match self {
            EntityKindData::Arrow(d) => Some(d),
            _ => None,
        }
    }
}
