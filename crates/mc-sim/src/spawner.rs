//! `SpawnerAnimals` (SpawnerAnimals.java): the natural mob spawner run by
//! `World.tick`, with the `EnumCreatureType` data (EnumCreatureType.java) and
//! `SpawnListEntry` (the biome lists live in [`crate::worldgen::biome`]).
//!
//! # What is transcribed
//!
//! [`perform_spawning`] in full up to the mob's construction: the eligible
//! chunk set (17×17 chunks around each player, a Java `HashSet` iterated in
//! bucket order, so [`mc_core::java_hash::JavaHashSet`], held by the world as
//! the static is held by the JVM), the per-type cap, the per-chunk weighted
//! pick (`rand.nextInt(totalWeight)`), [`get_random_spawning_point_in_chunk`],
//! the three packs of four jittered positions and
//! [`can_creature_type_spawn_at_location`], the 24-block player and spawn
//! distance tests, and the yaw draw `rand.nextFloat() * 360` of
//! `setLocationAndAngles`. Every `World.rand` draw the Java makes up to that
//! point happens here, in the same order, and the chunk loads
//! (`isBlockNormalCube`/`getBlockMaterial` load what they touch) too.
//!
//! # What is not (recorded gaps)
//!
//! - Constructing the mob (`entityClass.getConstructor(World.class)
//!   .newInstance(world)`, SpawnerAnimals.java:123): the mob constructors are
//!   WP-2.3. The port records `"<Class>.<init> (SpawnerAnimals.performSpawning)"`
//!   in [`World::unimplemented_hits`], still draws the yaw, and continues as if
//!   `getCanSpawnHere()` were false, so nothing joins the world. **Known
//!   divergence:** the Java constructor also takes an entity id and draws four
//!   `Math.random()` values (`EntityLiving`'s field initialiser and
//!   constructor, EntityLiving.java:38, :63, :65, :66), and when
//!   `getCanSpawnHere` holds the mob joins the world and
//!   `creatureSpecificInit` draws from `World.rand` (spider jockey
//!   `nextInt(100)`, sheep fleece colour). So a scenario in which the oracle
//!   reaches a constructor diverges from that point on.
//! - `performSleepSpawning` (SpawnerAnimals.java:173-251) needs the
//!   `Pathfinder` and beds (WP-2.2, WP-2.3, WP-2.5); `World.tick` records it.
//!
//! [`World::unimplemented_hits`]: crate::world::World::unimplemented_hits

use mc_core::java_hash::{ChunkCoordIntPair, ChunkPosition};
use mc_core::math::floor_double;

use crate::block::Material;
use crate::world::World;
use crate::worldgen::biome::EnumCreatureType;

impl EnumCreatureType {
    /// `EnumCreatureType.values()`: declaration order.
    // src: EnumCreatureType.java:4-6
    pub const VALUES: [EnumCreatureType; 3] = [
        EnumCreatureType::Monster,
        EnumCreatureType::Creature,
        EnumCreatureType::WaterCreature,
    ];

    /// `getCreatureClass()`: the class `countEntities` counts, as a Java
    /// class or interface name
    /// ([`crate::world::is_instance_of`]).
    // src: EnumCreatureType.java:4-6, :19-21
    pub fn get_creature_class(self) -> &'static str {
        match self {
            EnumCreatureType::Monster => "IMob",
            EnumCreatureType::Creature => "EntityAnimal",
            EnumCreatureType::WaterCreature => "EntityWaterMob",
        }
    }

    /// `getMaxNumberOfCreature()`: the cap per 256 eligible chunks.
    // src: EnumCreatureType.java:4-6, :23-25
    pub fn get_max_number_of_creature(self) -> i32 {
        match self {
            EnumCreatureType::Monster => 70,
            EnumCreatureType::Creature => 15,
            EnumCreatureType::WaterCreature => 5,
        }
    }

    /// `getCreatureMaterial()`: the material a pack's centre must be.
    // src: EnumCreatureType.java:4-6, :27-29
    pub fn get_creature_material(self) -> Material {
        match self {
            EnumCreatureType::Monster | EnumCreatureType::Creature => Material::Air,
            EnumCreatureType::WaterCreature => Material::Water,
        }
    }

    /// `getPeacefulCreature()`.
    // src: EnumCreatureType.java:4-6, :31-33
    pub fn get_peaceful_creature(self) -> bool {
        match self {
            EnumCreatureType::Monster => false,
            EnumCreatureType::Creature | EnumCreatureType::WaterCreature => true,
        }
    }
}

/// `getRandomSpawningPointInChunk(world, blockX, blockZ)`: three `World.rand`
/// draws, x then y then z.
// src: SpawnerAnimals.java:12-17
pub fn get_random_spawning_point_in_chunk(
    world: &mut World,
    var1: i32,
    var2: i32,
) -> ChunkPosition {
    let var3 = var1.wrapping_add(world.rand.next_int_bounded(16));
    let var4 = world.rand.next_int_bounded(128);
    let var5 = var2.wrapping_add(world.rand.next_int_bounded(16));
    ChunkPosition::new(var3, var4, var5)
}

/// `performSpawning(world, hostile, peaceful)`: one natural-spawning pass
/// (module docs). Returns Java's running count (`var3`, which adds the pack
/// count once per successful distance test: vanilla, transcribed; the
/// caller ignores it).
// src: SpawnerAnimals.java:19-151
pub fn perform_spawning(world: &mut World, var1: bool, var2: bool) -> i32 {
    if !var1 && !var2 {
        return 0;
    }
    world.eligible_chunks_for_spawning.clear();

    // src: SpawnerAnimals.java:27-38
    let players = world.player_entities.clone();
    for pid in players {
        let Some(p) = world.entity(pid) else {
            world.note_unimplemented("SpawnerAnimals.performSpawning over a taken-out player");
            continue;
        };
        let var5 = floor_double(p.base.pos_x / 16.0);
        let var6 = floor_double(p.base.pos_z / 16.0);
        let var7 = 8;
        for var8 in -var7..=var7 {
            for var9 in -var7..=var7 {
                world
                    .eligible_chunks_for_spawning
                    .insert(ChunkCoordIntPair::new(var8 + var5, var9 + var6));
            }
        }
    }

    let mut var3 = 0i32;
    let var35 = world.get_spawn_point(); // src: SpawnerAnimals.java:41
    let eligible_count = world.eligible_chunks_for_spawning.len() as i32;

    // src: SpawnerAnimals.java:46-147 (label133)
    for var38 in EnumCreatureType::VALUES {
        if !((!var38.get_peaceful_creature() || var2)
            && (var38.get_peaceful_creature() || var1)
            && world.count_entities(var38.get_creature_class())
                <= var38.get_max_number_of_creature() * eligible_count / 256)
        {
            continue;
        }
        // The set is not modified while it is iterated (nothing below
        // touches it), so a snapshot iterates in the same order.
        let chunks: Vec<ChunkCoordIntPair> =
            world.eligible_chunks_for_spawning.iter().copied().collect();
        // src: SpawnerAnimals.java:49-145 (label130: one chunk per iteration)
        for var10 in chunks {
            // src: SpawnerAnimals.java:68-71
            let var11 = world
                .get_world_chunk_manager()
                .get_biome_gen_at_chunk_coord(var10.chunk_x_pos, var10.chunk_z_pos);
            let var12 = var11.get_spawnable_list(var38);
            if var12.is_empty() {
                continue;
            }
            // src: SpawnerAnimals.java:73-90
            let var13: i32 = var12.iter().map(|e| e.item_weight).sum();
            let mut var40 = world.rand.next_int_bounded(var13);
            let mut var15 = var12[0];
            for var17 in var12 {
                var40 -= var17.item_weight;
                if var40 < 0 {
                    var15 = *var17;
                    break;
                }
            }
            // src: SpawnerAnimals.java:92-97
            let var41 = get_random_spawning_point_in_chunk(
                world,
                var10.chunk_x_pos.wrapping_mul(16),
                var10.chunk_z_pos.wrapping_mul(16),
            );
            let (var42, var18, var19) = (var41.x, var41.y, var41.z);
            if world.is_block_normal_cube(var42, var18, var19) {
                continue;
            }
            if world.get_block_material(var42, var18, var19) != var38.get_creature_material() {
                continue;
            }

            // src: SpawnerAnimals.java:99-144
            let var20 = 0i32;
            for _var21 in 0..3 {
                let mut var22 = var42;
                let mut var23 = var18;
                let mut var24 = var19;
                let var25 = 6;
                for _var26 in 0..4 {
                    var22 +=
                        world.rand.next_int_bounded(var25) - world.rand.next_int_bounded(var25);
                    // vanilla bug: nextInt(1) is always 0, so a pack never
                    // changes height, yet both calls still draw (SpawnerAnimals.java:109).
                    var23 += world.rand.next_int_bounded(1) - world.rand.next_int_bounded(1);
                    var24 +=
                        world.rand.next_int_bounded(var25) - world.rand.next_int_bounded(var25);
                    if !can_creature_type_spawn_at_location(var38, world, var22, var23, var24) {
                        continue;
                    }
                    let var27 = var22 as f32 + 0.5;
                    let var28 = var23 as f32;
                    let var29 = var24 as f32 + 0.5;
                    if world
                        .get_closest_player(var27 as f64, var28 as f64, var29 as f64, 24.0)
                        .is_some()
                    {
                        continue;
                    }
                    let var30 = var27 - var35.pos_x as f32;
                    let var31 = var28 - var35.pos_y as f32;
                    let var32 = var29 - var35.pos_z as f32;
                    let var33 = var30 * var30 + var31 * var31 + var32 * var32;
                    if var33 >= 576.0 {
                        // src: SpawnerAnimals.java:121-127: the mob constructor
                        // (module docs: not transcribed, recorded).
                        world.note_unimplemented(constructor_name(var15.entity_class));
                        // src: SpawnerAnimals.java:129 (the yaw of
                        // setLocationAndAngles; the World.rand draw is kept)
                        let _yaw = world.rand.next_float() * 360.0;
                        // src: SpawnerAnimals.java:130-137: getCanSpawnHere()
                        // on the unbuilt mob counts as false here, so nothing
                        // joins, `var20` stays 0 and `var20 >=
                        // getMaxSpawnedInChunk()` (`continue label130`, the
                        // next chunk) is never reached.
                        // vanilla bug: inside the position loop, so the pack
                        // count is added again for every position that passes
                        // the distance tests; the caller ignores the total.
                        var3 += var20; // src: SpawnerAnimals.java:139
                    }
                }
            }
        }
    }
    var3
}

/// `canCreatureTypeSpawnAtLocation(type, world, x, y, z)`: water creatures
/// need liquid with no normal cube above; the others a normal cube below,
/// none at or above the feet, and no liquid at the feet.
// src: SpawnerAnimals.java:153-159
pub fn can_creature_type_spawn_at_location(
    var0: EnumCreatureType,
    var1: &mut World,
    var2: i32,
    var3: i32,
    var4: i32,
) -> bool {
    if var0.get_creature_material() == Material::Water {
        var1.get_block_material(var2, var3, var4).get_is_liquid()
            && !var1.is_block_normal_cube(var2, var3 + 1, var4)
    } else {
        var1.is_block_normal_cube(var2, var3 - 1, var4)
            && !var1.is_block_normal_cube(var2, var3, var4)
            && !var1.get_block_material(var2, var3, var4).get_is_liquid()
            && !var1.is_block_normal_cube(var2, var3 + 1, var4)
    }
}

/// The gap name recorded when `performSpawning` reaches a mob constructor.
fn constructor_name(class: &'static str) -> &'static str {
    match class {
        "EntitySpider" => "EntitySpider.<init> (SpawnerAnimals.performSpawning)",
        "EntityZombie" => "EntityZombie.<init> (SpawnerAnimals.performSpawning)",
        "EntitySkeleton" => "EntitySkeleton.<init> (SpawnerAnimals.performSpawning)",
        "EntityCreeper" => "EntityCreeper.<init> (SpawnerAnimals.performSpawning)",
        "EntitySlime" => "EntitySlime.<init> (SpawnerAnimals.performSpawning)",
        "EntityGhast" => "EntityGhast.<init> (SpawnerAnimals.performSpawning)",
        "EntityPigZombie" => "EntityPigZombie.<init> (SpawnerAnimals.performSpawning)",
        "EntitySheep" => "EntitySheep.<init> (SpawnerAnimals.performSpawning)",
        "EntityPig" => "EntityPig.<init> (SpawnerAnimals.performSpawning)",
        "EntityChicken" => "EntityChicken.<init> (SpawnerAnimals.performSpawning)",
        "EntityCow" => "EntityCow.<init> (SpawnerAnimals.performSpawning)",
        "EntityWolf" => "EntityWolf.<init> (SpawnerAnimals.performSpawning)",
        "EntitySquid" => "EntitySquid.<init> (SpawnerAnimals.performSpawning)",
        _ => "Entity*.<init> (SpawnerAnimals.performSpawning)",
    }
}
