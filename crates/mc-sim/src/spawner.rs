//! `SpawnerAnimals` (SpawnerAnimals.java): the natural mob spawner run by
//! `World.tick`, with the `EnumCreatureType` data (EnumCreatureType.java) and
//! `SpawnListEntry` (the biome lists live in [`crate::worldgen::biome`]).
//!
//! # What is transcribed
//!
//! All of it: [`perform_spawning`] (the eligible chunk set, 17×17 chunks
//! around each player, a Java `HashSet` iterated in bucket order, so
//! [`mc_core::java_hash::JavaHashSet`], held by the world as the static is
//! held by the JVM; the per-type cap; the per-chunk weighted pick;
//! [`get_random_spawning_point_in_chunk`]; the three packs of four jittered
//! positions and [`can_creature_type_spawn_at_location`]; the 24-block
//! player and spawn distance tests; the mob's construction,
//! `setLocationAndAngles`, `getCanSpawnHere`, `entityJoinedWorld`,
//! [`creature_specific_init`] and the `getMaxSpawnedInChunk` pack limit),
//! and [`perform_sleep_spawning`] (a night monster next to a sleeping
//! player, found with the `Pathfinder` over the world).
//!
//! The mob is built by its `(World)` constructor through
//! [`entity_list::new_instance`], which knows every constructor the port
//! has ([`entity_list::CONSTRUCTORS`]); nothing here is per kind except
//! `creatureSpecificInit`'s two cases, as in Java.
//!
//! # Recorded gaps
//!
//! - A mob whose constructor the port does not have yet:
//!   [`entity_list::new_instance`] makes the constructor's world side
//!   effects (the entity id, `EntityLiving`'s four `Math.random()` draws)
//!   and records it; the port then draws the yaw of `setLocationAndAngles`
//!   and continues as if `getCanSpawnHere()` were false, so nothing joins.
//!   **Known divergence** from that point when the oracle's mob would have
//!   joined (its `creatureSpecificInit` and the pack count).
//! - `EntityPlayer.wakeUpPlayer` (WP-2.5) after a sleep spawn.
//!
//! [`World::unimplemented_hits`]: crate::world::World::unimplemented_hits

use mc_core::java_hash::{ChunkCoordIntPair, ChunkCoordinates, ChunkPosition};
use mc_core::math::floor_double;

use crate::block::{Material, bed};
use crate::entity::data_watcher::WatchValue;
use crate::entity::pathfinding::{PathBlockAccess, Pathfinder};
use crate::entity::{Entity, EntityKindData, base, dispatch, living};
use crate::save::entity_list;
use crate::world::{World, in_world_xz};
use crate::worldgen::biome::EnumCreatureType;
use mc_core::JavaRandom;

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
    // src: EnumCreatureType.java:4-6, :20-22
    pub fn get_creature_class(self) -> &'static str {
        match self {
            EnumCreatureType::Monster => "IMob",
            EnumCreatureType::Creature => "EntityAnimal",
            EnumCreatureType::WaterCreature => "EntityWaterMob",
        }
    }

    /// `getMaxNumberOfCreature()`: the cap per 256 eligible chunks.
    // src: EnumCreatureType.java:4-6, :24-26
    pub fn get_max_number_of_creature(self) -> i32 {
        match self {
            EnumCreatureType::Monster => 70,
            EnumCreatureType::Creature => 15,
            EnumCreatureType::WaterCreature => 5,
        }
    }

    /// `getCreatureMaterial()`: the material a pack's centre must be.
    // src: EnumCreatureType.java:4-6, :28-30
    pub fn get_creature_material(self) -> Material {
        match self {
            EnumCreatureType::Monster | EnumCreatureType::Creature => Material::Air,
            EnumCreatureType::WaterCreature => Material::Water,
        }
    }

    /// `getPeacefulCreature()`.
    // src: EnumCreatureType.java:4-6, :32-34
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
        'label130: for var10 in chunks {
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
            let mut var20 = 0i32;
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
                        // src: SpawnerAnimals.java:121-127 (a constructor
                        // that throws returns `var3`; none does in vanilla)
                        let var43 = entity_list::new_instance(var15.entity_class, world);
                        // src: SpawnerAnimals.java:129
                        let yaw = world.rand.next_float() * 360.0;
                        // A mob the port cannot build counts as not
                        // spawnable (module docs, "Recorded gaps").
                        if let Some(mut var43) = var43 {
                            var43.base.set_location_and_angles(
                                var27 as f64,
                                var28 as f64,
                                var29 as f64,
                                yaw,
                                0.0,
                            );
                            // src: SpawnerAnimals.java:130-137
                            if dispatch::get_can_spawn_here(&var43, world) {
                                var20 += 1;
                                let max = join_then(world, var43, |w, e| {
                                    creature_specific_init(e, w, var27, var28, var29);
                                    dispatch::get_max_spawned_in_chunk(e, w)
                                });
                                if var20 >= max {
                                    continue 'label130;
                                }
                            }
                        }
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

/// `entityJoinedWorld(e)`, then `f` on the entity (Java keeps its
/// reference after the join): on the joined entity in its slot, or on the
/// entity itself when it did not join (its chunk is not loaded,
/// World.java:904-906), as Java goes on with the object either way.
fn join_then<R>(world: &mut World, e: Entity, f: impl FnOnce(&mut World, &mut Entity) -> R) -> R {
    let joins = e.kind.is_player()
        || world.chunk_exists(
            floor_double(e.base.pos_x / 16.0),
            floor_double(e.base.pos_z / 16.0),
        );
    if !joins {
        let mut e = e;
        return f(world, &mut e);
    }
    let id = e.base.entity_id;
    world.spawn_entity_in_world(e);
    world
        .with_entity_mut(id, f)
        .expect("an entity that just joined is in its slot")
}

/// `creatureSpecificInit(mob, world, x, y, z)` (private): a spider gets a
/// skeleton rider one time in 100 (the draw is made only for spiders); a
/// sheep gets a random fleece colour from `World.rand`. `e` is in hand.
// src: SpawnerAnimals.java:161-171
pub fn creature_specific_init(e: &mut Entity, world: &mut World, var2: f32, var3: f32, var4: f32) {
    if matches!(e.kind, EntityKindData::Spider(..)) && world.rand.next_int_bounded(100) == 0 {
        // src: SpawnerAnimals.java:163-166 (`new EntitySkeleton(world)`)
        let Some(mut var5) = entity_list::new_instance("EntitySkeleton", world) else {
            return;
        };
        let yaw = e.base.rotation_yaw;
        var5.base
            .set_location_and_angles(var2 as f64, var3 as f64, var4 as f64, yaw, 0.0);
        join_then(world, var5, |w, s| base::mount_entity(s, w, Some(e)));
    } else if let EntityKindData::Sheep(..) = e.kind {
        // src: SpawnerAnimals.java:167-169
        let c = get_random_fleece_color(&mut world.rand);
        set_fleece_color(e, c);
    }
}

/// `EntitySheep.getRandomFleeceColor(random)` (static): white 82 %, then
/// black, gray, light gray, brown, and pink one time in 500 of the rest.
/// Transcribed here for `creatureSpecificInit`; `EntitySheep` itself is
/// the animals task's (WP-2.3).
// src: EntitySheep.java:102-115
fn get_random_fleece_color(var0: &mut JavaRandom) -> i32 {
    let var1 = var0.next_int_bounded(100);
    if var1 < 5 {
        15
    } else if var1 < 10 {
        7
    } else if var1 < 15 {
        8
    } else if var1 < 18 {
        12
    } else if var0.next_int_bounded(500) == 0 {
        6
    } else {
        0
    }
}

/// `EntitySheep.setFleeceColor(color)`: the low nibble of watcher byte 16
/// (`EntitySheep.entityInit` adds it).
// src: EntitySheep.java:83-86
fn set_fleece_color(e: &mut Entity, var1: i32) {
    let var2 = e.base.data_watcher.get_watchable_object_byte(16);
    e.base
        .data_watcher
        .update_object(16, WatchValue::Byte((var2 as i32 & 240 | var1 & 15) as i8));
}

/// `SpawnerAnimals.nightSpawnEntities`: the monsters that may wake a
/// sleeping player.
// src: SpawnerAnimals.java:10
pub const NIGHT_SPAWN_ENTITIES: [&str; 3] = ["EntitySpider", "EntityZombie", "EntitySkeleton"];

/// The world as the `IBlockAccess` of `new Pathfinder(world)`
/// (SpawnerAnimals.java:175). `World.getBlockId` loads the chunk it reads;
/// the pathfinder reads within about 64 blocks of a player, where every
/// chunk is loaded (the random-tick pass loads the 19 × 19 chunks around
/// each player every tick, World.java:1868-1885), so this view reads loaded
/// chunks only and records the case it cannot serve.
struct WorldBlocks<'w>(&'w World);

impl PathBlockAccess for WorldBlocks<'_> {
    // src: World.java:311-321
    fn get_block_id(&self, x: i32, y: i32, z: i32) -> i32 {
        if !in_world_xz(x, z) || !(0..128).contains(&y) {
            return 0;
        }
        match self.0.loaded_chunk(x >> 4, z >> 4) {
            Some(c) => c.get_block_id(x & 15, y, z & 15),
            None => {
                self.0.note_unimplemented(
                    "SpawnerAnimals.performSleepSpawning: Pathfinder read of an unloaded chunk (World.getBlockId would load it)",
                );
                0
            }
        }
    }

    // src: World.java:405-420
    fn get_block_metadata(&self, x: i32, y: i32, z: i32) -> i32 {
        if !in_world_xz(x, z) || !(0..128).contains(&y) {
            return 0;
        }
        match self.0.loaded_chunk(x >> 4, z >> 4) {
            Some(c) => c.get_block_metadata(x & 15, y, z & 15),
            None => {
                self.0.note_unimplemented(
                    "SpawnerAnimals.performSleepSpawning: Pathfinder read of an unloaded chunk (World.getBlockId would load it)",
                );
                0
            }
        }
    }
}

/// `performSleepSpawning(world, players)`: for each player, up to 20 tries
/// at a random column within 32 blocks (y within 16) for a night monster
/// that can stand there and whose path to the player ends within 1.5
/// blocks of it; such a monster is moved next to the bed, joins, wakes the
/// player and makes its living sound. True if any spawned (`World.tick`
/// then skips the night).
// src: SpawnerAnimals.java:173-251
#[allow(clippy::manual_clamp)] // Java's two ifs, kept
pub fn perform_sleep_spawning(world: &mut World, var1: &[i32]) -> bool {
    let mut var2 = false;
    for &pid in var1 {
        let var6 = NIGHT_SPAWN_ENTITIES;
        let mut var7 = false;
        let mut var8 = 0;
        while var8 < 20 && !var7 {
            let Some(p) = world.entity(pid) else {
                world.note_unimplemented(
                    "SpawnerAnimals.performSleepSpawning over a taken-out player",
                );
                break;
            };
            let (px, py, pz) = (p.base.pos_x, p.base.pos_y, p.base.pos_z);
            // src: SpawnerAnimals.java:195-204
            let var9 = floor_double(px) + world.rand.next_int_bounded(32)
                - world.rand.next_int_bounded(32);
            let var10 = floor_double(pz) + world.rand.next_int_bounded(32)
                - world.rand.next_int_bounded(32);
            let mut var11 = floor_double(py) + world.rand.next_int_bounded(16)
                - world.rand.next_int_bounded(16);
            if var11 < 1 {
                var11 = 1;
            } else if var11 > 128 {
                var11 = 128;
            }
            let var12 = world.rand.next_int_bounded(var6.len() as i32);
            // src: SpawnerAnimals.java:206-212 (the empty `for` walks down)
            let mut var13 = var11;
            while var13 > 2 && !world.is_block_normal_cube(var9, var13 - 1, var10) {
                var13 -= 1;
            }
            while !can_creature_type_spawn_at_location(
                EnumCreatureType::Monster,
                world,
                var9,
                var13,
                var10,
            ) && var13 < var11 + 16
                && var13 < 128
            {
                var13 += 1;
            }
            if var13 < var11 + 16 && var13 < 128 {
                // src: SpawnerAnimals.java:214-228
                let var14 = var9 as f32 + 0.5;
                let var15 = var13 as f32;
                let var16 = var10 as f32 + 0.5;
                let var17 = entity_list::new_instance(var6[var12 as usize], world);
                let yaw = world.rand.next_float() * 360.0;
                // A monster the port cannot build counts as not spawnable
                // (module docs, "Recorded gaps").
                if let Some(mut var17) = var17 {
                    var17.base.set_location_and_angles(
                        var14 as f64,
                        var15 as f64,
                        var16 as f64,
                        yaw,
                        0.0,
                    );
                    if dispatch::get_can_spawn_here(&var17, world)
                        && sleep_spawn_next_to(world, var17, pid, var9, var13, var10)
                    {
                        var2 = true;
                        var7 = true;
                    }
                }
            }
            var8 += 1;
        }
    }
    var2
}

/// The rest of a sleep-spawn try once the monster can spawn: the path to
/// the player (range 32), and if it has more than one point and ends
/// within 1.5 blocks of the player, the monster moves to the free spot by
/// the bed (or above the column), joins, gets `creatureSpecificInit`, wakes
/// the player and makes its living sound. True once the path test passes
/// (Java sets `var2`/`var7` whether or not `entityJoinedWorld` added the
/// monster).
// src: SpawnerAnimals.java:229-245
fn sleep_spawn_next_to(
    world: &mut World,
    mut var17: Entity,
    pid: i32,
    var9: i32,
    var13: i32,
    var10: i32,
) -> bool {
    let Some(p) = world.entity(pid) else {
        return false;
    };
    let pbase = p.base.clone();
    let var18 = Pathfinder::new(&WorldBlocks(world)).create_entity_path_to_entity(
        &var17.base,
        &pbase,
        32.0,
    );
    let Some(var18) = var18 else {
        return false;
    };
    if var18.points().len() <= 1 {
        return false;
    }
    let Some(var19) = var18.func_22328_c() else {
        return false;
    };
    if !((var19.x_coord as f64 - pbase.pos_x).abs() < 1.5
        && (var19.z_coord as f64 - pbase.pos_z).abs() < 1.5
        && (var19.y_coord as f64 - pbase.pos_y).abs() < 1.5)
    {
        return false;
    }
    // src: SpawnerAnimals.java:233-236
    let var20 = bed::get_nearest_empty_chunk_coordinates(
        world,
        floor_double(pbase.pos_x),
        floor_double(pbase.pos_y),
        floor_double(pbase.pos_z),
        1,
    )
    .unwrap_or(ChunkCoordinates::new(var9, var13 + 1, var10));
    let (x, y, z) = (
        var20.pos_x as f32 + 0.5,
        var20.pos_y as f32,
        var20.pos_z as f32 + 0.5,
    );
    // src: SpawnerAnimals.java:238-242
    var17
        .base
        .set_location_and_angles(x as f64, y as f64, z as f64, 0.0, 0.0);
    join_then(world, var17, |w, e| {
        creature_specific_init(e, w, x, y, z);
        // `var5.wakeUpPlayer(true, false, false)`: EntityPlayer, WP-2.5.
        w.note_unimplemented("EntityPlayer.wakeUpPlayer (SpawnerAnimals.performSleepSpawning)");
        living::play_living_sound(e, w);
    });
    true
}
