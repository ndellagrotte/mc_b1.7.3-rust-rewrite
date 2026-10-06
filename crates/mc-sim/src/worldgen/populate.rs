//! `populate` of the two terrain generators: `ChunkProviderGenerate.populate`
//! (ChunkProviderGenerate.java:310-603) and `ChunkProviderHell.populate`
//! (ChunkProviderHell.java:302-359), called by the caching wrapper
//! ([`World::populate_chunk`]) through
//! [`crate::world::chunk_provider::ChunkGenerator::populate`].
//!
//! # The generator's `Random`
//!
//! Every draw is on the generator's own `Random` through
//! [`RandomSource::ChunkProviderRand`], fetched from the world for each
//! draw: the generator stays in the world during population, and a feature
//! that reads outside the loaded chunks makes the wrapper generate one, whose
//! `provideChunk` reseeds that same `Random` (and may populate further
//! chunks, which reseed it again in the overworld). The population then
//! continues from the reseeded state, as in vanilla.
//!
//! - Overworld: `rand` is reseeded at the start from the world seed and the
//!   chunk coordinates (ChunkProviderGenerate.java:315-318).
//! - Nether: `hellRNG` is **never** reseeded (vanilla quirk,
//!   tests/golden/README.md "`kind: "terrain"`"): it continues from the last
//!   `provideChunk` (seeded from the chunk coordinates only) and every
//!   population since, so nether population depends on the chunk load order.
//!
//! # `BlockSand.fallInstantly`
//!
//! Both set the static flag ([`crate::block::BlockStatics::fall_instantly`])
//! at the start and clear it at the end, so sand and gravel placed or
//! uncovered during population fall at once instead of becoming
//! `EntityFallingSand`. Vanilla quirk, transcribed: a nested population (a
//! chunk generated and populated from inside this one) clears the flag when
//! it ends, and the rest of the outer population then runs with it off.

use super::biome::Biome;
use super::features::dungeons::WorldGenDungeons;
use super::features::lakes::WorldGenLakes;
use super::features::liquids::{WorldGenHellLava, WorldGenLiquids};
use super::features::minable::{WorldGenClay, WorldGenMinable};
use super::features::nether::{GlowStoneClass, WorldGenFire, WorldGenGlowStone};
use super::features::plants::{
    WorldGenCactus, WorldGenDeadBush, WorldGenFlowers, WorldGenPumpkin, WorldGenReed,
    WorldGenTallGrass,
};
use super::features::{next_double, next_int};
use super::generate::ChunkProviderGenerate;
use super::trees::WorldGen;
use crate::block::dispatch::RandomSource;
use crate::block::{Material, ids};
use crate::world::World;
use crate::world::chunk_provider::ChunkGenerator;

/// The generator's `Random`, as every feature receives it.
const RAND: RandomSource = RandomSource::ChunkProviderRand;

/// `this` of `ChunkProviderGenerate.populate`: the world's overworld
/// generator (never lent out during population).
fn generator(world: &mut World) -> &mut ChunkProviderGenerate {
    match world.chunk_provider.generator.as_mut() {
        Some(ChunkGenerator::Generate(g)) => g,
        _ => panic!("ChunkProviderGenerate.populate without the overworld generator"),
    }
}

/// `ChunkProviderGenerate.populate(provider, x, z)`: lakes, dungeons, clay,
/// ores, trees, plants, springs, then snow, over the 16×16 square offset by
/// (8, 8) from the chunk's corner (it straddles the chunk and its +x, +z
/// neighbours, which the wrapper guarantees are loaded).
// src: ChunkProviderGenerate.java:310-603
pub fn populate_generate(world: &mut World, var2: i32, var3: i32) {
    world.block_statics.fall_instantly = true;
    let var4 = var2 * 16;
    let var5 = var3 * 16;
    let var6 = world
        .get_world_chunk_manager()
        .get_biome_gen_at(var4 + 16, var5 + 16);
    // src: ChunkProviderGenerate.java:315-318 (the per-chunk reseed)
    let seed = world.get_random_seed();
    world.random(RAND).set_seed(seed);
    let var7 = (world.random(RAND).next_long() / 2)
        .wrapping_mul(2)
        .wrapping_add(1);
    let var9 = (world.random(RAND).next_long() / 2)
        .wrapping_mul(2)
        .wrapping_add(1);
    world.random(RAND).set_seed(
        ((var2 as i64).wrapping_mul(var7)).wrapping_add((var3 as i64).wrapping_mul(var9)) ^ seed,
    );

    // src: ChunkProviderGenerate.java:323-328 (water lake, 1 in 4)
    if next_int(world, RAND, 4) == 0 {
        let var13 = var4 + next_int(world, RAND, 16) + 8;
        let var14 = next_int(world, RAND, 128);
        let var15 = var5 + next_int(world, RAND, 16) + 8;
        WorldGenLakes::new(ids::WATER_STILL).generate(world, RAND, var13, var14, var15);
    }

    // src: ChunkProviderGenerate.java:330-337 (lava lake, 1 in 8; above
    // y 64 only 1 in 10 of those)
    if next_int(world, RAND, 8) == 0 {
        let var13 = var4 + next_int(world, RAND, 16) + 8;
        let bound = next_int(world, RAND, 120) + 8;
        let var14 = next_int(world, RAND, bound);
        let var15 = var5 + next_int(world, RAND, 16) + 8;
        if var14 < 64 || next_int(world, RAND, 10) == 0 {
            WorldGenLakes::new(ids::LAVA_STILL).generate(world, RAND, var13, var14, var15);
        }
    }

    // src: ChunkProviderGenerate.java:340-345 (8 dungeon attempts)
    for _ in 0..8 {
        let var14 = var4 + next_int(world, RAND, 16) + 8;
        let var15 = next_int(world, RAND, 128);
        let var16 = var5 + next_int(world, RAND, 16) + 8;
        WorldGenDungeons.generate(world, RAND, var14, var15, var16);
    }

    // src: ChunkProviderGenerate.java:347-352 (10 clay patches of 32)
    for _ in 0..10 {
        let var14 = var4 + next_int(world, RAND, 16);
        let var15 = next_int(world, RAND, 128);
        let var16 = var5 + next_int(world, RAND, 16);
        WorldGenClay::new(32).generate(world, RAND, var14, var15, var16);
    }

    // src: ChunkProviderGenerate.java:354-408 (veins: count, block, size,
    // height bound; lapis draws its height as the sum of two nextInt(16))
    let veins: [(i32, i32, i32, i32); 8] = [
        (20, ids::DIRT, 32, 128),
        (10, ids::GRAVEL, 32, 128),
        (20, ids::ORE_COAL, 16, 128),
        (20, ids::ORE_IRON, 8, 64),
        (2, ids::ORE_GOLD, 8, 32),
        (8, ids::ORE_REDSTONE, 7, 16),
        (1, ids::ORE_DIAMOND, 7, 16),
        (1, ids::ORE_LAPIS, 6, 16),
    ];
    for (count, id, size, height) in veins {
        for _ in 0..count {
            let var14 = var4 + next_int(world, RAND, 16);
            let var15 = if id == ids::ORE_LAPIS {
                next_int(world, RAND, 16) + next_int(world, RAND, 16)
            } else {
                next_int(world, RAND, height)
            };
            let var16 = var5 + next_int(world, RAND, 16);
            WorldGenMinable::new(id, size).generate(world, RAND, var14, var15, var16);
        }
    }

    // src: ChunkProviderGenerate.java:410-443 (tree count: the noise first,
    // then the nextDouble; then a 1-in-10 extra tree; biome adjustments)
    let var11 = 0.5f64;
    let noise = generator(world)
        .mob_spawner_noise
        .noise_2d(var4 as f64 * var11, var5 as f64 * var11);
    let var13 = ((noise / 8.0 + next_double(world, RAND) * 4.0 + 4.0) / 3.0) as i32;
    let mut var14 = 0;
    if next_int(world, RAND, 10) == 0 {
        var14 += 1;
    }
    if var6 == Biome::Forest {
        var14 += var13 + 5;
    }
    if var6 == Biome::Rainforest {
        var14 += var13 + 5;
    }
    if var6 == Biome::SeasonalForest {
        var14 += var13 + 2;
    }
    if var6 == Biome::Taiga {
        var14 += var13 + 5;
    }
    if var6 == Biome::Desert {
        var14 -= 20;
    }
    if var6 == Biome::Tundra {
        var14 -= 20;
    }
    if var6 == Biome::Plains {
        var14 -= 20;
    }

    // src: ChunkProviderGenerate.java:446-452 (a fresh generator per tree,
    // `func_517_a(1, 1, 1)`, placed at the height map)
    for _ in 0..var14 {
        let var16 = var4 + next_int(world, RAND, 16) + 8;
        let var17 = var5 + next_int(world, RAND, 16) + 8;
        let kind = var6.get_random_world_gen_for_trees(world.random(RAND));
        let mut var18 = WorldGen::new(kind);
        var18.func_517_a(1.0, 1.0, 1.0);
        let h = world.get_height_value(var16, var17);
        var18.generate(world, RAND, var16, h, var17);
    }

    // src: ChunkProviderGenerate.java:454-478 (dandelions)
    let mut var27 = 0;
    if var6 == Biome::Forest {
        var27 = 2;
    }
    if var6 == Biome::SeasonalForest {
        var27 = 4;
    }
    if var6 == Biome::Taiga {
        var27 = 2;
    }
    if var6 == Biome::Plains {
        var27 = 3;
    }
    for _ in 0..var27 {
        let var17 = var4 + next_int(world, RAND, 16) + 8;
        let var25 = next_int(world, RAND, 128);
        let var19 = var5 + next_int(world, RAND, 16) + 8;
        WorldGenFlowers::new(ids::PLANT_YELLOW).generate(world, RAND, var17, var25, var19);
    }

    // src: ChunkProviderGenerate.java:480-513 (tall grass; the rainforest
    // draws for fern, metadata 2, in two of three patches)
    let mut var28 = 0;
    if var6 == Biome::Forest {
        var28 = 2;
    }
    if var6 == Biome::Rainforest {
        var28 = 10;
    }
    if var6 == Biome::SeasonalForest {
        var28 = 2;
    }
    if var6 == Biome::Taiga {
        var28 = 1;
    }
    if var6 == Biome::Plains {
        var28 = 10;
    }
    for _ in 0..var28 {
        let mut var26 = 1;
        if var6 == Biome::Rainforest && next_int(world, RAND, 3) != 0 {
            var26 = 2;
        }
        let var19 = var4 + next_int(world, RAND, 16) + 8;
        let var20 = next_int(world, RAND, 128);
        let var21 = var5 + next_int(world, RAND, 16) + 8;
        WorldGenTallGrass::new(ids::TALL_GRASS, var26).generate(world, RAND, var19, var20, var21);
    }

    // src: ChunkProviderGenerate.java:515-525 (dead bushes in the desert)
    let mut var28 = 0;
    if var6 == Biome::Desert {
        var28 = 2;
    }
    for _ in 0..var28 {
        let var25 = var4 + next_int(world, RAND, 16) + 8;
        let var19 = next_int(world, RAND, 128);
        let var20 = var5 + next_int(world, RAND, 16) + 8;
        WorldGenDeadBush::new(ids::DEAD_BUSH).generate(world, RAND, var25, var19, var20);
    }

    // src: ChunkProviderGenerate.java:527-546 (rose 1/2, brown mushroom 1/4,
    // red mushroom 1/8)
    for (chance, id) in [
        (2, ids::PLANT_RED),
        (4, ids::MUSHROOM_BROWN),
        (8, ids::MUSHROOM_RED),
    ] {
        if next_int(world, RAND, chance) == 0 {
            let var17 = var4 + next_int(world, RAND, 16) + 8;
            let var25 = next_int(world, RAND, 128);
            let var19 = var5 + next_int(world, RAND, 16) + 8;
            WorldGenFlowers::new(id).generate(world, RAND, var17, var25, var19);
        }
    }

    // src: ChunkProviderGenerate.java:548-553 (10 reed patches)
    for _ in 0..10 {
        let var25 = var4 + next_int(world, RAND, 16) + 8;
        let var19 = next_int(world, RAND, 128);
        let var20 = var5 + next_int(world, RAND, 16) + 8;
        WorldGenReed.generate(world, RAND, var25, var19, var20);
    }

    // src: ChunkProviderGenerate.java:555-560 (pumpkins, 1 in 32)
    if next_int(world, RAND, 32) == 0 {
        let var17 = var4 + next_int(world, RAND, 16) + 8;
        let var25 = next_int(world, RAND, 128);
        let var19 = var5 + next_int(world, RAND, 16) + 8;
        WorldGenPumpkin.generate(world, RAND, var17, var25, var19);
    }

    // src: ChunkProviderGenerate.java:562-572 (cacti in the desert)
    let mut var17 = 0;
    if var6 == Biome::Desert {
        var17 += 10;
    }
    for _ in 0..var17 {
        let var19 = var4 + next_int(world, RAND, 16) + 8;
        let var20 = next_int(world, RAND, 128);
        let var21 = var5 + next_int(world, RAND, 16) + 8;
        WorldGenCactus.generate(world, RAND, var19, var20, var21);
    }

    // src: ChunkProviderGenerate.java:574-579 (50 water springs, height
    // nextInt(nextInt(120) + 8))
    for _ in 0..50 {
        let var19 = var4 + next_int(world, RAND, 16) + 8;
        let bound = next_int(world, RAND, 120) + 8;
        let var20 = next_int(world, RAND, bound);
        let var21 = var5 + next_int(world, RAND, 16) + 8;
        WorldGenLiquids::new(ids::WATER_MOVING).generate(world, RAND, var19, var20, var21);
    }

    // src: ChunkProviderGenerate.java:581-586 (20 lava springs, height
    // nextInt(nextInt(nextInt(112) + 8) + 8): innermost bound first)
    for _ in 0..20 {
        let var19 = var4 + next_int(world, RAND, 16) + 8;
        let b1 = next_int(world, RAND, 112) + 8;
        let b2 = next_int(world, RAND, b1) + 8;
        let var20 = next_int(world, RAND, b2);
        let var21 = var5 + next_int(world, RAND, 16) + 8;
        WorldGenLiquids::new(ids::LAVA_MOVING).generate(world, RAND, var19, var20, var21);
    }

    // src: ChunkProviderGenerate.java:588-600 (snow where the
    // height-adjusted temperature is below 0.5, on a solid block that is not
    // ice). The temperatures are the generator's reused
    // `generatedTemperatures` array, read through the generator each time.
    let temps = std::mem::take(&mut generator(world).generated_temperatures);
    let temps = world
        .get_world_chunk_manager()
        .get_temperatures(temps, var4 + 8, var5 + 8, 16, 16);
    generator(world).generated_temperatures = temps;
    for var25 in (var4 + 8)..(var4 + 8 + 16) {
        for var19 in (var5 + 8)..(var5 + 8 + 16) {
            let var20 = var25 - (var4 + 8);
            let var21 = var19 - (var5 + 8);
            let var22 = world.find_top_solid_block(var25, var19);
            let var23 = generator(world).generated_temperatures[(var20 * 16 + var21) as usize]
                - (var22 - 64) as f64 / 64.0 * 0.3;
            if var23 < 0.5
                && var22 > 0
                && var22 < 128
                && world.is_air_block(var25, var22, var19)
                && world
                    .get_block_material(var25, var22 - 1, var19)
                    .get_is_solid()
                && world.get_block_material(var25, var22 - 1, var19) != Material::Ice
            {
                world.set_block_with_notify(var25, var22, var19, ids::SNOW);
            }
        }
    }

    // vanilla bug: a nested population clears the static flag while the
    // outer one is still running (module docs).
    world.block_statics.fall_instantly = false;
}

/// `ChunkProviderHell.populate(provider, x, z)`: lava springs, fire,
/// glowstone (two generators), mushrooms; `hellRNG` is not reseeded
/// (module docs).
// src: ChunkProviderHell.java:302-359
pub fn populate_hell(world: &mut World, var2: i32, var3: i32) {
    world.block_statics.fall_instantly = true;
    // vanilla bug: `hellRNG` is not reseeded here (unlike the overworld's
    // `rand.setSeed`), so the draws continue from the last `provideChunk` and
    // every population since: nether population depends on chunk load order.
    let var4 = var2 * 16;
    let var5 = var3 * 16;

    // src: ChunkProviderHell.java:311-316 (8 lava springs)
    for _ in 0..8 {
        let var7 = var4 + next_int(world, RAND, 16) + 8;
        let var8 = next_int(world, RAND, 120) + 4;
        let var9 = var5 + next_int(world, RAND, 16) + 8;
        WorldGenHellLava::new(ids::LAVA_MOVING).generate(world, RAND, var7, var8, var9);
    }

    // src: ChunkProviderHell.java:318-326 (nextInt(nextInt(10) + 1) + 1
    // fire patches)
    let bound = next_int(world, RAND, 10) + 1;
    let var6 = next_int(world, RAND, bound) + 1;
    for _ in 0..var6 {
        let var8 = var4 + next_int(world, RAND, 16) + 8;
        let var9 = next_int(world, RAND, 120) + 4;
        let var10 = var5 + next_int(world, RAND, 16) + 8;
        WorldGenFire.generate(world, RAND, var8, var9, var10);
    }

    // src: ChunkProviderHell.java:328-335 (nextInt(nextInt(10) + 1)
    // WorldGenGlowStone1 clusters)
    let bound = next_int(world, RAND, 10) + 1;
    let var6 = next_int(world, RAND, bound);
    for _ in 0..var6 {
        let var8 = var4 + next_int(world, RAND, 16) + 8;
        let var9 = next_int(world, RAND, 120) + 4;
        let var10 = var5 + next_int(world, RAND, 16) + 8;
        WorldGenGlowStone::new(GlowStoneClass::GlowStone1).generate(world, RAND, var8, var9, var10);
    }

    // src: ChunkProviderHell.java:337-342 (10 WorldGenGlowStone2 clusters)
    for _ in 0..10 {
        let var8 = var4 + next_int(world, RAND, 16) + 8;
        let var9 = next_int(world, RAND, 128);
        let var10 = var5 + next_int(world, RAND, 16) + 8;
        WorldGenGlowStone::new(GlowStoneClass::GlowStone2).generate(world, RAND, var8, var9, var10);
    }

    // src: ChunkProviderHell.java:344-356 (brown then red mushrooms; the
    // `nextInt(1) == 0` tests always pass but still draw)
    for id in [ids::MUSHROOM_BROWN, ids::MUSHROOM_RED] {
        if next_int(world, RAND, 1) == 0 {
            let var7 = var4 + next_int(world, RAND, 16) + 8;
            let var8 = next_int(world, RAND, 128);
            let var9 = var5 + next_int(world, RAND, 16) + 8;
            WorldGenFlowers::new(id).generate(world, RAND, var7, var8, var9);
        }
    }

    // vanilla bug: a population nested inside another (a chunk generated
    // and populated from a feature) clears the static flag here while the
    // outer one is still running (module docs).
    world.block_statics.fall_instantly = false;
}
