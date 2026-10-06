//! `BiomeGenBase` and its subclasses (BiomeGenBase.java, BiomeGenDesert.java,
//! BiomeGenForest.java, BiomeGenHell.java, BiomeGenRainforest.java,
//! BiomeGenSwamp.java, BiomeGenTaiga.java).
//!
//! # Shape
//!
//! Vanilla's biomes are thirteen singletons created by `BiomeGenBase`'s static
//! initialiser. Here a biome is a [`Biome`] value (copyable, compared by
//! identity like the Java references) and its data is the immutable
//! [`BiomeGenBase`] row in [`BIOMES`], holding the values the fields have
//! *after* the static initialiser ran:
//!
//! 1. each biome is constructed in declaration order (BiomeGenBase.java:9-21);
//!    the base constructor sets grass/dirt, filler metadata 5169201, rain on,
//!    and the default spawn lists (:34-52), then the subclass constructor
//!    edits the lists and the chained setters set colour, name, snow, rain
//!    and filler metadata;
//! 2. `biomeLookupTable = new BiomeGenBase[4096]` (:32);
//! 3. the `static {}` block (:154-156) runs `generateBiomeLookup()`, which
//!    fills the table from [`get_biome`] (:59-64) and **then** sets desert's
//!    and ice desert's top and filler block to sand (:66-67).
//!
//! [`BIOME_LOOKUP_TABLE`] is built at compile time with the same loop and the
//! same `f32` arithmetic (Rust const evaluation is IEEE-754 exact).
//!
//! The sky biome (`BiomeGenBase.sky`, BiomeGenBase.java:21, BiomeGenSky.java)
//! belongs to the sky dimension, which is deferred (PLAN_v4 D3): it is not in
//! the lookup table and is not ported.
//!
//! Spawn lists are data: `(entity class name, weight)` as `SpawnListEntry`
//! holds them (`SpawnerAnimals`, WP-2.3, turns the name into an entity).
//! `getRandomWorldGenForTrees` returns which `WorldGen*` class vanilla would
//! instantiate ([`WorldGenTreeKind`]); the generators themselves are WP-1.7.

use mc_core::JavaRandom;

use super::ids;

/// A biome singleton (`BiomeGenBase.rainforest`, …), in declaration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Biome {
    /// `rainforest`.
    Rainforest,
    /// `swampland`.
    Swampland,
    /// `seasonalForest`.
    SeasonalForest,
    /// `forest`.
    Forest,
    /// `savanna`.
    Savanna,
    /// `shrubland`.
    Shrubland,
    /// `taiga`.
    Taiga,
    /// `desert`.
    Desert,
    /// `plains`.
    Plains,
    /// `iceDesert`.
    IceDesert,
    /// `tundra`.
    Tundra,
    /// `hell`.
    Hell,
}

/// Which `BiomeGenBase` class a biome instantiates (for the overridden
/// methods).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BiomeClass {
    /// `BiomeGenBase` itself.
    Base,
    /// `BiomeGenDesert` (no overrides).
    Desert,
    /// `BiomeGenForest`.
    Forest,
    /// `BiomeGenHell`.
    Hell,
    /// `BiomeGenRainforest`.
    Rainforest,
    /// `BiomeGenSwamp` (no overrides).
    Swamp,
    /// `BiomeGenTaiga`.
    Taiga,
}

/// `SpawnListEntry(entityClass, weight)`; the class as its Java name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpawnListEntry {
    /// `entityClass`, e.g. `"EntitySpider"`.
    pub entity_class: &'static str,
    /// `itemWeight`.
    pub item_weight: i32,
}

const fn spawn(entity_class: &'static str, item_weight: i32) -> SpawnListEntry {
    SpawnListEntry {
        entity_class,
        item_weight,
    }
}

/// `EnumCreatureType`, only as the key of [`BiomeGenBase::get_spawnable_list`];
/// its data (class, cap, material, peaceful flag) belongs to `SpawnerAnimals`
/// (WP-2.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnumCreatureType {
    /// `monster`.
    Monster,
    /// `creature`.
    Creature,
    /// `waterCreature`.
    WaterCreature,
}

/// The `WorldGenerator` subclass `getRandomWorldGenForTrees` instantiates.
/// The generators are WP-1.7.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldGenTreeKind {
    /// `WorldGenTrees`.
    Trees,
    /// `WorldGenBigTree`.
    BigTree,
    /// `WorldGenForest` (birch).
    Forest,
    /// `WorldGenTaiga1`.
    Taiga1,
    /// `WorldGenTaiga2`.
    Taiga2,
}

/// The data of one `BiomeGenBase` singleton after static initialisation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BiomeGenBase {
    /// The subclass.
    pub class: BiomeClass,
    /// `biomeName`.
    pub biome_name: &'static str,
    /// `color`.
    pub color: i32,
    /// `topBlock` (a Java `byte`).
    pub top_block: u8,
    /// `fillerBlock` (a Java `byte`).
    pub filler_block: u8,
    /// `fillerBlockMetadata` (a colour; never read by the simulation).
    pub filler_block_metadata: i32,
    /// `spawnableMonsterList`.
    pub spawnable_monster_list: &'static [SpawnListEntry],
    /// `spawnableCreatureList`.
    pub spawnable_creature_list: &'static [SpawnListEntry],
    /// `spawnableWaterCreatureList`.
    pub spawnable_water_creature_list: &'static [SpawnListEntry],
    /// `enableSnow`.
    pub enable_snow: bool,
    /// `enableRain`.
    pub enable_rain: bool,
}

/// The base constructor's monster list.
// src: BiomeGenBase.java:42-46
const BASE_MONSTERS: &[SpawnListEntry] = &[
    spawn("EntitySpider", 10),
    spawn("EntityZombie", 10),
    spawn("EntitySkeleton", 10),
    spawn("EntityCreeper", 10),
    spawn("EntitySlime", 10),
];
/// The base constructor's creature list.
// src: BiomeGenBase.java:47-50
const BASE_CREATURES: &[SpawnListEntry] = &[
    spawn("EntitySheep", 12),
    spawn("EntityPig", 10),
    spawn("EntityChicken", 10),
    spawn("EntityCow", 8),
];
/// The base list plus `EntityWolf` weight 2 (BiomeGenForest, BiomeGenTaiga).
// src: BiomeGenForest.java:7, BiomeGenTaiga.java:7
const WOLF_CREATURES: &[SpawnListEntry] = &[
    spawn("EntitySheep", 12),
    spawn("EntityPig", 10),
    spawn("EntityChicken", 10),
    spawn("EntityCow", 8),
    spawn("EntityWolf", 2),
];
/// The base constructor's water-creature list.
// src: BiomeGenBase.java:51
const BASE_WATER: &[SpawnListEntry] = &[spawn("EntitySquid", 10)];
/// `BiomeGenHell`: all lists cleared, then ghast and pig zombie.
// src: BiomeGenHell.java:5-9
const HELL_MONSTERS: &[SpawnListEntry] = &[spawn("EntityGhast", 10), spawn("EntityPigZombie", 10)];

/// `Block.grass.blockID`, `Block.dirt.blockID` as bytes (BiomeGenBase.java:35-36).
const GRASS: u8 = ids::GRASS;
const DIRT: u8 = ids::DIRT;
const SAND: u8 = ids::SAND;

/// A biome as the base constructor leaves it, before the chained setters.
// src: BiomeGenBase.java:34-52
const fn base(class: BiomeClass) -> BiomeGenBase {
    BiomeGenBase {
        class,
        biome_name: "",
        color: 0,
        top_block: GRASS,
        filler_block: DIRT,
        filler_block_metadata: 5169201,
        spawnable_monster_list: BASE_MONSTERS,
        spawnable_creature_list: BASE_CREATURES,
        spawnable_water_creature_list: BASE_WATER,
        enable_snow: false,
        enable_rain: true,
    }
}

const fn named(mut b: BiomeGenBase, color: i32, name: &'static str) -> BiomeGenBase {
    b.color = color;
    b.biome_name = name;
    b
}

const fn filler_meta(mut b: BiomeGenBase, m: i32) -> BiomeGenBase {
    b.filler_block_metadata = m;
    b
}

const fn snow(mut b: BiomeGenBase) -> BiomeGenBase {
    b.enable_snow = true;
    b
}

const fn no_rain(mut b: BiomeGenBase) -> BiomeGenBase {
    b.enable_rain = false;
    b
}

const fn creatures(mut b: BiomeGenBase, l: &'static [SpawnListEntry]) -> BiomeGenBase {
    b.spawnable_creature_list = l;
    b
}

const fn sand_top(mut b: BiomeGenBase) -> BiomeGenBase {
    b.top_block = SAND;
    b.filler_block = SAND;
    b
}

const fn hell_lists(mut b: BiomeGenBase) -> BiomeGenBase {
    b.spawnable_monster_list = HELL_MONSTERS;
    b.spawnable_creature_list = &[];
    b.spawnable_water_creature_list = &[];
    b
}

/// Every biome's data, indexed by [`Biome`], as after the static initialiser.
pub const BIOMES: [BiomeGenBase; 12] = [
    // src: BiomeGenBase.java:9
    filler_meta(
        named(base(BiomeClass::Rainforest), 588342, "Rainforest"),
        2094168,
    ),
    // src: BiomeGenBase.java:10
    filler_meta(named(base(BiomeClass::Swamp), 522674, "Swampland"), 9154376),
    // src: BiomeGenBase.java:11
    named(base(BiomeClass::Base), 10215459, "Seasonal Forest"),
    // src: BiomeGenBase.java:12, BiomeGenForest.java:6-8
    filler_meta(
        named(
            creatures(base(BiomeClass::Forest), WOLF_CREATURES),
            353825,
            "Forest",
        ),
        5159473,
    ),
    // src: BiomeGenBase.java:13
    named(base(BiomeClass::Desert), 14278691, "Savanna"),
    // src: BiomeGenBase.java:14
    named(base(BiomeClass::Base), 10595616, "Shrubland"),
    // src: BiomeGenBase.java:15, BiomeGenTaiga.java:6-8
    filler_meta(
        snow(named(
            creatures(base(BiomeClass::Taiga), WOLF_CREATURES),
            3060051,
            "Taiga",
        )),
        8107825,
    ),
    // src: BiomeGenBase.java:16; sand from generateBiomeLookup (:66)
    sand_top(no_rain(named(base(BiomeClass::Desert), 16421912, "Desert"))),
    // src: BiomeGenBase.java:17
    named(base(BiomeClass::Desert), 16767248, "Plains"),
    // src: BiomeGenBase.java:18; sand from generateBiomeLookup (:67)
    sand_top(filler_meta(
        no_rain(snow(named(
            base(BiomeClass::Desert),
            16772499,
            "Ice Desert",
        ))),
        12899129,
    )),
    // src: BiomeGenBase.java:19
    filler_meta(
        snow(named(base(BiomeClass::Base), 5762041, "Tundra")),
        12899129,
    ),
    // src: BiomeGenBase.java:20, BiomeGenHell.java:4-10
    no_rain(named(hell_lists(base(BiomeClass::Hell)), 16711680, "Hell")),
];

/// `getBiome(temperature, humidity)`.
// src: BiomeGenBase.java:100-121
pub const fn get_biome(var0: f32, mut var1: f32) -> Biome {
    var1 *= var0;
    if var0 < 0.1 {
        Biome::Tundra
    } else if var1 < 0.2 {
        if var0 < 0.5 {
            Biome::Tundra
        } else if var0 < 0.95 {
            Biome::Savanna
        } else {
            Biome::Desert
        }
    } else if var1 > 0.5 && var0 < 0.7 {
        Biome::Swampland
    } else if var0 < 0.5 {
        Biome::Taiga
    } else if var0 < 0.97 {
        if var1 < 0.35 {
            Biome::Shrubland
        } else {
            Biome::Forest
        }
    } else if var1 < 0.45 {
        Biome::Plains
    } else if var1 < 0.9 {
        Biome::SeasonalForest
    } else {
        Biome::Rainforest
    }
}

/// `generateBiomeLookup()`'s table fill: `table[t + h * 64] =
/// getBiome(t / 63f, h / 63f)`, t outer.
// src: BiomeGenBase.java:59-65
const fn generate_biome_lookup() -> [Biome; 4096] {
    let mut table = [Biome::Tundra; 4096];
    let mut var0 = 0;
    while var0 < 64 {
        let mut var1 = 0;
        while var1 < 64 {
            table[(var0 + var1 * 64) as usize] =
                get_biome(var0 as f32 / 63.0f32, var1 as f32 / 63.0f32);
            var1 += 1;
        }
        var0 += 1;
    }
    table
}

/// `biomeLookupTable`, after the static initialiser.
// src: BiomeGenBase.java:32, :154-156
pub static BIOME_LOOKUP_TABLE: [Biome; 4096] = generate_biome_lookup();

/// `getBiomeFromLookup(temperature, humidity)`.
// src: BiomeGenBase.java:94-98
pub fn get_biome_from_lookup(var0: f64, var2: f64) -> Biome {
    let var4 = (var0 * 63.0) as i32;
    let var5 = (var2 * 63.0) as i32;
    BIOME_LOOKUP_TABLE[(var4 + var5 * 64) as usize]
}

impl Biome {
    /// The singleton's data.
    pub fn data(self) -> &'static BiomeGenBase {
        &BIOMES[self as usize]
    }

    /// `biomeName`.
    pub fn biome_name(self) -> &'static str {
        self.data().biome_name
    }

    /// `getRandomWorldGenForTrees(rand)`.
    // src: BiomeGenBase.java:70-72, BiomeGenForest.java:10-16,
    // BiomeGenRainforest.java:6-8, BiomeGenTaiga.java:10-12
    pub fn get_random_world_gen_for_trees(self, var1: &mut JavaRandom) -> WorldGenTreeKind {
        match self.data().class {
            BiomeClass::Forest => {
                if var1.next_int_bounded(5) == 0 {
                    WorldGenTreeKind::Forest
                } else if var1.next_int_bounded(3) == 0 {
                    WorldGenTreeKind::BigTree
                } else {
                    WorldGenTreeKind::Trees
                }
            }
            BiomeClass::Rainforest => {
                if var1.next_int_bounded(3) == 0 {
                    WorldGenTreeKind::BigTree
                } else {
                    WorldGenTreeKind::Trees
                }
            }
            BiomeClass::Taiga => {
                if var1.next_int_bounded(3) == 0 {
                    WorldGenTreeKind::Taiga1
                } else {
                    WorldGenTreeKind::Taiga2
                }
            }
            BiomeClass::Base | BiomeClass::Desert | BiomeClass::Hell | BiomeClass::Swamp => {
                if var1.next_int_bounded(10) == 0 {
                    WorldGenTreeKind::BigTree
                } else {
                    WorldGenTreeKind::Trees
                }
            }
        }
    }

    /// `getSkyColorByTemp(temperature)`:
    /// `Color.getHSBColor(0.62222224 - t * 0.05, 0.5 + t * 0.1, 1).getRGB()`
    /// with `t = clamp(temperature / 3, -1, 1)`. The result includes the
    /// opaque alpha byte (`0xff000000`), as `Color.getRGB` does.
    // src: BiomeGenBase.java:123-134
    #[allow(clippy::manual_clamp)] // Java's two ifs
    pub fn get_sky_color_by_temp(self, mut var1: f32) -> i32 {
        var1 /= 3.0;
        if var1 < -1.0 {
            var1 = -1.0;
        }
        if var1 > 1.0 {
            var1 = 1.0;
        }
        hsb_to_rgb(0.62222224 - var1 * 0.05, 0.5 + var1 * 0.1, 1.0)
    }

    /// `getSpawnableList(type)`.
    // src: BiomeGenBase.java:136-144
    pub fn get_spawnable_list(self, var1: EnumCreatureType) -> &'static [SpawnListEntry] {
        let d = self.data();
        match var1 {
            EnumCreatureType::Monster => d.spawnable_monster_list,
            EnumCreatureType::Creature => d.spawnable_creature_list,
            EnumCreatureType::WaterCreature => d.spawnable_water_creature_list,
        }
    }

    /// `getEnableSnow()`.
    // src: BiomeGenBase.java:146-148
    pub fn get_enable_snow(self) -> bool {
        self.data().enable_snow
    }

    /// `canSpawnLightningBolt()`: rain and no snow.
    // src: BiomeGenBase.java:150-152
    pub fn can_spawn_lightning_bolt(self) -> bool {
        let d = self.data();
        if d.enable_snow { false } else { d.enable_rain }
    }
}

/// `java.awt.Color.HSBtoRGB(h, s, b)` (what `Color.getHSBColor(h, s, b)
/// .getRGB()` returns), written from its documented behaviour: hue sector
/// `(int)((h - floor(h)) * 6)`, channels `(int)(v * 255 + 0.5)`, alpha 255.
/// Not covered by a fixture (client sky colour only).
fn hsb_to_rgb(hue: f32, saturation: f32, brightness: f32) -> i32 {
    let ch = |v: f32| (v * 255.0f32 + 0.5f32) as i32;
    let (r, g, b);
    if saturation == 0.0 {
        let v = ch(brightness);
        (r, g, b) = (v, v, v);
    } else {
        let h = (hue - (hue as f64).floor() as f32) * 6.0f32;
        let f = h - (h as f64).floor() as f32;
        let p = brightness * (1.0f32 - saturation);
        let q = brightness * (1.0f32 - saturation * f);
        let t = brightness * (1.0f32 - saturation * (1.0f32 - f));
        (r, g, b) = match h as i32 {
            0 => (ch(brightness), ch(t), ch(p)),
            1 => (ch(q), ch(brightness), ch(p)),
            2 => (ch(p), ch(brightness), ch(t)),
            3 => (ch(p), ch(q), ch(brightness)),
            4 => (ch(t), ch(p), ch(brightness)),
            5 => (ch(brightness), ch(p), ch(q)),
            _ => (0, 0, 0),
        };
    }
    (0xff000000u32 as i32) | (r << 16) | (g << 8) | b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_corners() {
        // t = 0 column is all tundra; t = h = 1 is rainforest (h*t = 1 >= 0.9).
        assert_eq!(BIOME_LOOKUP_TABLE[0], Biome::Tundra);
        assert_eq!(BIOME_LOOKUP_TABLE[63 + 63 * 64], Biome::Rainforest);
        assert_eq!(get_biome_from_lookup(1.0, 0.0), Biome::Desert);
        assert_eq!(Biome::Desert.data().top_block, ids::SAND);
        assert_eq!(Biome::Plains.data().top_block, ids::GRASS);
        assert_eq!(Biome::IceDesert.data().filler_block, ids::SAND);
    }

    #[test]
    fn biome_order_matches_declarations() {
        let names: Vec<&str> = BIOMES.iter().map(|b| b.biome_name).collect();
        assert_eq!(
            names,
            [
                "Rainforest",
                "Swampland",
                "Seasonal Forest",
                "Forest",
                "Savanna",
                "Shrubland",
                "Taiga",
                "Desert",
                "Plains",
                "Ice Desert",
                "Tundra",
                "Hell"
            ]
        );
    }

    #[test]
    fn hsb_primary_colours() {
        assert_eq!(hsb_to_rgb(0.0, 1.0, 1.0), 0xffff0000u32 as i32);
        assert_eq!(hsb_to_rgb(1.0 / 3.0, 1.0, 1.0), 0xff00ff00u32 as i32);
        assert_eq!(hsb_to_rgb(0.0, 0.0, 1.0), -1);
    }
}
