//! `scenario.json`: the one input both engines replay (ADR-007 §5.1,
//! `tests/golden/README.md` "Tier A scenarios").
//!
//! Parsing is two-stage: the common fields (`name`, `tier`, `kind`) first, then
//! the kind-specific fields. An unknown `kind` is an error for tier `A` and
//! becomes [`ScenarioKind::Unsupported`] (raw JSON kept) for tiers B/C/S/D,
//! whose kinds later work packages define. Unknown extra fields are tolerated
//! so the oracle side can add documentation fields without breaking the Rust
//! side.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{Bits64, GoldenError};

/// One fixture's scenario.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Scenario {
    /// Fixture name, normally equal to its directory name.
    pub name: String,
    /// Comparison tier (PLAN_v4 §4.3).
    pub tier: Tier,
    /// What to run; the JSON `"kind"` field selects the variant.
    #[serde(flatten)]
    pub kind: ScenarioKind,
}

/// Stage one of parsing: the common fields, everything else left raw.
#[derive(Deserialize)]
struct RawScenario {
    name: String,
    tier: Tier,
    kind: String,
    #[serde(flatten)]
    rest: serde_json::Map<String, serde_json::Value>,
}

impl<'de> Deserialize<'de> for Scenario {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let RawScenario {
            name,
            tier,
            kind,
            mut rest,
        } = RawScenario::deserialize(d)?;
        rest.insert("kind".into(), kind.clone().into());
        let raw = serde_json::Value::Object(rest);
        let kind = if ScenarioKind::KNOWN.contains(&kind.as_str()) {
            serde_json::from_value(raw).map_err(D::Error::custom)?
        } else if tier == Tier::A {
            return Err(D::Error::custom(format!("unknown Tier A kind `{kind}`")));
        } else {
            ScenarioKind::Unsupported { kind, raw }
        };
        Ok(Self { name, tier, kind })
    }
}

/// PLAN_v4 §4.3 tiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tier {
    /// RNG, sine table, `Math.*`, NBT primitives, terrain, biomes.
    A,
    /// Block-level simulation (lighting, ticks, redstone, ...).
    B,
    /// Entities.
    C,
    /// Saves.
    S,
    /// Human-reviewed rendering/GUI/audio.
    D,
}

/// Kind-specific scenario fields.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScenarioKind {
    /// `java.util.Random` draws on one `new Random(seed)`, in list order.
    JavaRandom {
        /// Seed passed to `new Random(seed)`.
        seed: i64,
        /// Draw groups.
        draws: Vec<Draw>,
    },
    /// `MathHelper.SIN_TABLE` dump.
    SineTable,
    /// JVM `Math.*` bit-pattern sweep.
    MathSweep {
        /// Seed for the shared `new Random(seed)`.
        seed: i64,
        /// Pseudo-random inputs per function, after the edge cases.
        count: u32,
        /// Function names in sweep order.
        functions: Vec<String>,
        /// Edge-case inputs as bit-pattern strings. The oracle side defines
        /// the exact shape; read it through [`ScenarioKind::edge_cases_for`].
        #[serde(default)]
        edge_cases: serde_json::Value,
        /// Call `java.lang.StrictMath` (fdlibm) instead of `Math`.
        #[serde(default)]
        strict: bool,
    },
    /// Chunk generation, population and biomes.
    Terrain {
        /// World seed.
        seed: i64,
        /// Which generator.
        dimension: Dimension,
        /// Chunks in processing order.
        chunks: Vec<ChunkSel>,
    },
    /// Vanilla NBT writer on a typed tree.
    NbtWrite {
        /// The root node (a named compound).
        root: NbtNode,
    },
    /// Vanilla NBT reader on a sidecar file.
    NbtRead {
        /// Sidecar file name, relative to the scenario directory.
        file: String,
        /// `gzip` or `none`.
        compressed: String,
    },
    /// `java.util.HashSet` iteration order.
    JavaHashOrder {
        /// `ChunkCoordIntPair`, `ChunkPosition`, `ChunkCoordinates`,
        /// `Integer` or `String`.
        key_type: String,
        /// Operations in order.
        ops: Vec<HashOp>,
        /// Op counts after which the set is dumped.
        #[serde(default)]
        dump_after: Vec<usize>,
    },
    /// `Block`, `Material` and `MapColor` data after Block.java's static
    /// initialiser (WP-1.1; tests/golden/README.md, `kind: "block_table"`). No fields; the
    /// Rust side is `crates/mc-sim/tests/golden_block_table.rs`.
    BlockTable,
    /// A vanilla world ticked under a script (tiers B, C and S; README
    /// "`kind: \"tick\"`").
    Tick(TickScenario),
    /// Tier S: open a saved world and dump what the vanilla loaders read.
    WorldRead {
        /// Sidecar directory in the scenario directory holding the save
        /// (`level.dat`, `region/`). The README calls this field `dir`.
        dir: String,
        /// Chunks to dump; `None` means every present chunk.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        chunks: Option<Vec<ChunkPos>>,
    },
    /// `Item.itemsList`, `EnumToolMaterial`, block textures and `ItemStack`
    /// samples (WP-1.2; README "`kind: \"item_table\"`"). The kind-specific
    /// fields (`stacks`) are read by `mc-sim/tests/golden_item_table.rs`.
    ItemTable,
    /// A kind of tier B/C/S/D that a later work package defines; never
    /// produced for tier A and never serialized.
    #[serde(skip)]
    Unsupported {
        /// The `kind` string.
        kind: String,
        /// The whole scenario-specific JSON object (including `kind`).
        raw: serde_json::Value,
    },
}

/// A typed NBT node (README `nbt_write`). `value` stays raw JSON; the NBT
/// crate interprets it by `ty`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NbtNode {
    /// `byte`, `short`, `int`, `long`, `float`, `double`, `byte_array`,
    /// `string`, `list` or `compound`.
    #[serde(rename = "type")]
    pub ty: String,
    /// Tag name; present on the root only.
    #[serde(default)]
    pub name: Option<String>,
    /// The payload, shaped by `ty`.
    pub value: serde_json::Value,
}

/// One `java_hash_order` operation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HashOp {
    /// `add`, `remove` or `clear`.
    pub op: String,
    /// Array of ints, an int or a string, by `key_type`; absent for `clear`.
    #[serde(default)]
    pub key: Option<serde_json::Value>,
}

/// One `java_random` draw group.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Draw {
    /// `nextInt`, `nextLong`, `nextFloat`, `nextDouble`, `nextGaussian`,
    /// `nextBoolean`.
    pub call: String,
    /// Bound for `nextInt(bound)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound: Option<i32>,
    /// Number of draws.
    pub n: u32,
}

/// Terrain dimension.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Dimension {
    /// `WorldProviderSurface`.
    Overworld,
    /// `WorldProviderHell`.
    Nether,
}

/// Where a `tick` scenario's chunks come from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorldSource {
    /// The dimension's terrain generator (default).
    #[default]
    Generated,
    /// Every chunk all air (sky light 15 from y = 127 down to 1).
    Empty,
}

/// `mob_spawning` of a `tick` scenario.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MobSpawning {
    /// `setAllowedMobSpawns(difficulty > 0, true)` (Minecraft.java:1163).
    #[default]
    Default,
    /// `setAllowedMobSpawns(false, false)`.
    None,
}

/// A chunk coordinate pair (`{"x":..,"z":..}`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkPos {
    /// Chunk x.
    pub x: i32,
    /// Chunk z.
    pub z: i32,
}

/// `kind: "tick"` (tiers B, C, S): a world ticked under a script. Ticks are
/// numbered `0..ticks`; ops and dumps are applied per README "Tick loop".
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TickScenario {
    /// World seed and root of every harness seed.
    pub seed: i64,
    /// Which provider.
    pub dimension: Dimension,
    /// Chunk source; default [`WorldSource::Generated`].
    #[serde(default)]
    pub world: WorldSource,
    /// Number of ticks, at least 1.
    pub ticks: u32,
    /// 0..=3, default 0.
    #[serde(default)]
    pub difficulty: i32,
    /// `none` (default when absent) or `mcregion`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub save_format: Option<String>,
    /// Tier B isolation knob (tests/golden/README.md): `default` (absent)
    /// runs `setAllowedMobSpawns(difficulty > 0, true)` before each
    /// `World.tick`, `none` runs `setAllowedMobSpawns(false, false)`, so
    /// `SpawnerAnimals.performSpawning` returns before any draw.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mob_spawning: Option<MobSpawning>,
    /// Scripted operations, applied in file order within a tick.
    #[serde(default)]
    pub ops: Vec<Op>,
    /// Dumps, run in list order when due.
    #[serde(default)]
    pub dumps: Vec<Dump>,
}

/// One scripted operation at tick `t`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Op {
    /// Tick the op runs at (before the tick's world update).
    pub t: u32,
    /// The op and its fields (`"op"` selects the variant).
    #[serde(flatten)]
    pub kind: OpKind,
}

/// A held item (`spawn`'s `item`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemSpec {
    /// Item id.
    pub id: i32,
    /// Stack size.
    pub count: i32,
    /// Damage / metadata, default 0.
    #[serde(default)]
    pub damage: i32,
}

/// Op kinds and fields (README "Ops"). Floats are written as a JSON number
/// or a bit-pattern string; see [`flex`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum OpKind {
    /// `World.setBlockAndMetadataWithNotify`.
    SetBlock {
        /// Block x.
        x: i32,
        /// Block y.
        y: i32,
        /// Block z.
        z: i32,
        /// Block id.
        id: i32,
        /// Metadata, default 0.
        #[serde(default)]
        meta: i32,
    },
    /// `World.setBlockAndMetadata` (no neighbour notification).
    SetBlockSilent {
        /// Block x.
        x: i32,
        /// Block y.
        y: i32,
        /// Block z.
        z: i32,
        /// Block id.
        id: i32,
        /// Metadata, default 0.
        #[serde(default)]
        meta: i32,
    },
    /// `set_block_silent` over an inclusive box, x outer, z middle, y inner.
    Fill {
        /// Corner 0 x.
        x0: i32,
        /// Corner 0 y.
        y0: i32,
        /// Corner 0 z.
        z0: i32,
        /// Corner 1 x.
        x1: i32,
        /// Corner 1 y.
        y1: i32,
        /// Corner 1 z.
        z1: i32,
        /// Block id.
        id: i32,
        /// Metadata, default 0.
        #[serde(default)]
        meta: i32,
    },
    /// `World.setWorldTime`.
    SetTime {
        /// New world time.
        time: i64,
    },
    /// The `WorldInfo` weather setters (`setRainTime`, `setRaining`,
    /// `setThunderTime`, `setThundering`), each only when given.
    SetWeather {
        /// `WorldInfo.setRainTime`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rain_time: Option<i32>,
        /// `WorldInfo.setRaining`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        raining: Option<bool>,
        /// `WorldInfo.setThunderTime`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        thunder_time: Option<i32>,
        /// `WorldInfo.setThundering`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        thundering: Option<bool>,
    },
    /// Load the chunk square of radius `radius` around `(cx, cz)`.
    Preload {
        /// Centre chunk x.
        cx: i32,
        /// Centre chunk z.
        cz: i32,
        /// Radius in chunks.
        radius: i32,
    },
    /// Create the single `HarnessPlayer` at its feet position.
    Player {
        /// Feet x.
        #[serde(with = "flex::f64")]
        x: f64,
        /// Feet y.
        #[serde(with = "flex::f64")]
        y: f64,
        /// Feet z.
        #[serde(with = "flex::f64")]
        z: f64,
        /// Yaw (`f32`), constructor default when absent.
        #[serde(
            default,
            with = "flex::opt_f32",
            skip_serializing_if = "Option::is_none"
        )]
        yaw: Option<f32>,
        /// Pitch (`f32`), constructor default when absent.
        #[serde(
            default,
            with = "flex::opt_f32",
            skip_serializing_if = "Option::is_none"
        )]
        pitch: Option<f32>,
    },
    /// Construct and join an entity by `EntityList` name.
    Spawn {
        /// Entity name (`Pig`, `Item`, `Arrow`, ...).
        kind: String,
        /// Bounding-box bottom position.
        #[serde(with = "flex::vec3")]
        pos: [f64; 3],
        /// Yaw (`f32`).
        #[serde(
            default,
            with = "flex::opt_f32",
            skip_serializing_if = "Option::is_none"
        )]
        yaw: Option<f32>,
        /// Pitch (`f32`).
        #[serde(
            default,
            with = "flex::opt_f32",
            skip_serializing_if = "Option::is_none"
        )]
        pitch: Option<f32>,
        /// Expected entity id.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<i32>,
        /// For `Item`: the stack.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        item: Option<ItemSpec>,
        /// For `Item`: `delayBeforeCanPickup` (default 10).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pickup_delay: Option<i32>,
    },
    /// Assign `motionX/Y/Z`.
    SetMotion {
        /// Entity id.
        id: i32,
        /// New motion.
        #[serde(with = "flex::vec3")]
        motion: [f64; 3],
    },
    /// `Entity.setPosition`.
    SetPos {
        /// Entity id.
        id: i32,
        /// New position.
        #[serde(with = "flex::vec3")]
        pos: [f64; 3],
    },
    /// Assign `rotationYaw` / `rotationPitch` (`f32`).
    SetRotation {
        /// Entity id.
        id: i32,
        /// Yaw.
        #[serde(with = "flex::f32")]
        yaw: f32,
        /// Pitch.
        #[serde(with = "flex::f32")]
        pitch: f32,
    },
    /// Right-click a block face as the player.
    UseItem {
        /// Item id replacing hotbar slot `currentItem`, if given.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        item_id: Option<i32>,
        /// Stack size (default 1).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        count: Option<i32>,
        /// Damage (default 0).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        meta: Option<i32>,
        /// Hit block x.
        x: i32,
        /// Hit block y.
        y: i32,
        /// Hit block z.
        z: i32,
        /// Hit face.
        side: i32,
    },
    /// Attack an entity.
    Attack {
        /// Target entity id.
        target_id: i32,
        /// Attacker entity id (the player's id means a player punch); absent
        /// means a direct hit with no attacker.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attacker_id: Option<i32>,
        /// Damage, required unless the attacker is the player.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        damage: Option<i32>,
    },
    /// `World.createExplosion(null, x, y, z, size)`, or with `flaming` the
    /// body of `World.newExplosion(null, x, y, z, size, true)` with the
    /// explosion's `ExplosionRNG` seeded `mix(seed, -6)` (WP-1.8).
    Explode {
        /// Centre x.
        #[serde(with = "flex::f64")]
        x: f64,
        /// Centre y.
        #[serde(with = "flex::f64")]
        y: f64,
        /// Centre z.
        #[serde(with = "flex::f64")]
        z: f64,
        /// `explosionSize` (`f32`).
        #[serde(with = "flex::f32")]
        size: f32,
        /// `isFlaming`, default false.
        #[serde(default)]
        flaming: bool,
    },
    /// `World.saveWorld(true, null)` then the Tier S dumps (needs
    /// `save_format: "mcregion"`).
    Save {
        /// Chunks to dump; every present chunk when absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        chunks: Option<Vec<ChunkPos>>,
    },
}

/// A dump, due when `(t + 1) % every == 0` or at tick `t`; exactly one of
/// `every` and `t` is set.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Dump {
    /// Period in ticks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub every: Option<u32>,
    /// Single tick.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub t: Option<u32>,
    /// What to dump (`"what"` selects the variant).
    #[serde(flatten)]
    pub what: DumpKind,
}

impl Dump {
    /// Whether this dump is due after tick `t`.
    pub fn due_at(&self, t: u32) -> bool {
        self.t == Some(t) || self.every.is_some_and(|n| (t + 1).is_multiple_of(n))
    }
}

/// Which entities an `entity_state` dump covers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntityWhich {
    /// Every entity in `loadedEntityList` order.
    All,
    /// These ids, in this order.
    Ids(Vec<i32>),
}

impl Serialize for EntityWhich {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::All => s.serialize_str("all"),
            Self::Ids(ids) => ids.serialize(s),
        }
    }
}

impl<'de> Deserialize<'de> for EntityWhich {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Name(String),
            Ids(Vec<i32>),
        }
        match Raw::deserialize(d)? {
            Raw::Name(n) if n == "all" => Ok(Self::All),
            Raw::Name(n) => Err(serde::de::Error::custom(format!(
                "`which` must be \"all\" or a list of ids, got {n:?}"
            ))),
            Raw::Ids(ids) => Ok(Self::Ids(ids)),
        }
    }
}

/// Dump kinds and fields (README "Dumps").
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "what", rename_all = "snake_case")]
pub enum DumpKind {
    /// `chunk_digest` line with `"stage":"tick"` (+ `chunk_full` if `full`).
    ChunkDigest {
        /// Chunk x.
        x: i32,
        /// Chunk z.
        z: i32,
        /// Also emit `chunk_full`.
        #[serde(default)]
        full: bool,
    },
    /// One `block` line.
    Block {
        /// Block x.
        x: i32,
        /// Block y.
        y: i32,
        /// Block z.
        z: i32,
    },
    /// One `blocks_full` line over an inclusive box.
    BlockBox {
        /// Corner 0 x.
        x0: i32,
        /// Corner 0 y.
        y0: i32,
        /// Corner 0 z.
        z0: i32,
        /// Corner 1 x.
        x1: i32,
        /// Corner 1 y.
        y1: i32,
        /// Corner 1 z.
        z1: i32,
    },
    /// `scheduled_ticks` line; with `full` also the entries.
    ScheduledTicks {
        /// Include `entries`.
        #[serde(default)]
        full: bool,
    },
    /// `entity` lines (and `entity_absent` for missing ids).
    EntityState {
        /// Which entities.
        which: EntityWhich,
    },
    /// `world_time` line.
    WorldTime,
    /// `rng_state` line.
    RngState,
    /// `weather` line: the `WorldInfo` timers and flags and the four
    /// rain/thunder strengths.
    Weather,
    /// `ray_trace` line: `World.rayTraceBlocks_do(from, to, stop_on_liquid)`.
    RayTrace {
        /// Start point.
        #[serde(with = "flex::vec3")]
        from: [f64; 3],
        /// End point.
        #[serde(with = "flex::vec3")]
        to: [f64; 3],
        /// `stopOnLiquid` (default false).
        #[serde(default)]
        stop_on_liquid: bool,
    },
    /// `power` line: `isBlockGettingPowered` and
    /// `isBlockIndirectlyGettingPowered` at a block.
    Power {
        /// Block x.
        x: i32,
        /// Block y.
        y: i32,
        /// Block z.
        z: i32,
    },
}

/// Scenario floats: a JSON integer or number, or a bit-pattern string.
pub mod flex {
    use serde::de::{Error, Visitor};
    use serde::{Deserializer, Serializer};
    use std::fmt;

    use crate::{Bits32, Bits64};

    struct FlexVisitor<const WIDE: bool>;

    impl<const WIDE: bool> Visitor<'_> for FlexVisitor<WIDE> {
        type Value = f64;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a JSON number or a \"0x\" bit-pattern string")
        }
        fn visit_i64<E: Error>(self, v: i64) -> Result<f64, E> {
            Ok(v as f64)
        }
        fn visit_u64<E: Error>(self, v: u64) -> Result<f64, E> {
            Ok(v as f64)
        }
        fn visit_f64<E: Error>(self, v: f64) -> Result<f64, E> {
            Ok(v)
        }
        fn visit_str<E: Error>(self, v: &str) -> Result<f64, E> {
            if WIDE {
                Bits64::parse(v).map(Bits64::to_f64).map_err(E::custom)
            } else {
                Bits32::parse(v)
                    .map(|b| f64::from(b.to_f32()))
                    .map_err(E::custom)
            }
        }
    }

    /// `f64` as number or 16-digit bit string; written as the bit string.
    pub mod f64 {
        use super::*;
        /// Serde `deserialize_with`.
        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
            d.deserialize_any(FlexVisitor::<true>)
        }
        /// Serde `serialize_with`.
        pub fn serialize<S: Serializer>(v: &f64, s: S) -> Result<S::Ok, S::Error> {
            s.collect_str(&Bits64::from(*v))
        }
    }

    /// `f32` as number or 8-digit bit string; written as the bit string.
    pub mod f32 {
        use super::*;
        /// Serde `deserialize_with`.
        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<f32, D::Error> {
            d.deserialize_any(FlexVisitor::<false>).map(|v| v as f32)
        }
        /// Serde `serialize_with`.
        pub fn serialize<S: Serializer>(v: &f32, s: S) -> Result<S::Ok, S::Error> {
            s.collect_str(&Bits32::from(*v))
        }
    }

    /// `Option<f32>`; pair with `default` and `skip_serializing_if`.
    pub mod opt_f32 {
        use super::*;
        /// Serde `deserialize_with`.
        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<f32>, D::Error> {
            super::f32::deserialize(d).map(Some)
        }
        /// Serde `serialize_with`.
        pub fn serialize<S: Serializer>(v: &Option<f32>, s: S) -> Result<S::Ok, S::Error> {
            match v {
                Some(v) => super::f32::serialize(v, s),
                None => s.serialize_none(),
            }
        }
    }

    /// `[f64; 3]` of numbers or bit strings.
    pub mod vec3 {
        use super::*;
        use serde::{Deserialize, Serialize};

        #[derive(Deserialize)]
        struct One(#[serde(with = "super::f64")] f64);
        #[derive(Serialize)]
        struct OneOut<'a>(#[serde(with = "super::f64")] &'a f64);

        /// Serde `deserialize_with`.
        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[f64; 3], D::Error> {
            let v: [One; 3] = Deserialize::deserialize(d)?;
            Ok([v[0].0, v[1].0, v[2].0])
        }
        /// Serde `serialize_with`.
        pub fn serialize<S: Serializer>(v: &[f64; 3], s: S) -> Result<S::Ok, S::Error> {
            [OneOut(&v[0]), OneOut(&v[1]), OneOut(&v[2])].serialize(s)
        }
    }
}

/// A chunk to generate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkSel {
    /// Chunk x.
    pub x: i32,
    /// Chunk z.
    pub z: i32,
    /// Emit `chunk_full` lines after each `chunk_digest`.
    #[serde(default)]
    pub full: bool,
}

impl Scenario {
    /// Loads `<dir>/scenario.json`.
    pub fn load(dir: impl AsRef<Path>) -> Result<Self, GoldenError> {
        let path = dir.as_ref().join("scenario.json");
        let text = crate::io::read_text(&path)?;
        Self::parse(&text).map_err(|message| GoldenError::Parse {
            path,
            line: None,
            message,
        })
    }

    /// Parses scenario JSON text.
    pub fn parse(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|e| e.to_string())
    }
}

impl ScenarioKind {
    /// Kinds that deserialize into a typed variant.
    const KNOWN: [&'static str; 11] = [
        "java_random",
        "sine_table",
        "math_sweep",
        "terrain",
        "nbt_write",
        "nbt_read",
        "java_hash_order",
        "block_table",
        "tick",
        "world_read",
        "item_table",
    ];

    /// The `math_sweep` edge-case inputs for `function`, as `f64` bit
    /// patterns in document order (for two-argument functions, consecutive
    /// `x`, `y` pairs).
    ///
    /// Lenient about shape: if `edge_cases` is an object with a key equal to
    /// `function`, that entry is used; otherwise the whole value is shared by
    /// every function. Every string found (recursively, arrays in order,
    /// object values in key order) must be a valid [`Bits64`]; anything that
    /// is not a string, array or object is an error. Returns an empty list
    /// for other kinds or when `edge_cases` is absent.
    pub fn edge_cases_for(&self, function: &str) -> Result<Vec<Bits64>, String> {
        let Self::MathSweep { edge_cases, .. } = self else {
            return Ok(Vec::new());
        };
        let selected = match edge_cases {
            serde_json::Value::Object(map) => map.get(function).unwrap_or(edge_cases),
            other => other,
        };
        let mut out = Vec::new();
        collect_f64_bits(selected, &mut out)?;
        Ok(out)
    }
}

/// Collects every bit-pattern string in `value`, depth-first.
pub fn collect_f64_bits(value: &serde_json::Value, out: &mut Vec<Bits64>) -> Result<(), String> {
    use serde_json::Value;
    match value {
        Value::Null => Ok(()),
        Value::String(s) => Bits64::parse(s).map(|b| out.push(b)),
        Value::Array(items) => items.iter().try_for_each(|v| collect_f64_bits(v, out)),
        Value::Object(map) => map.values().try_for_each(|v| collect_f64_bits(v, out)),
        other => Err(format!("edge case {other} is not a bit-pattern string")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readme_examples_parse() {
        let s = Scenario::parse(
            r#"{"name":"java-random-123","tier":"A","kind":"java_random","seed":123,
             "draws":[{"call":"nextInt","n":64},{"call":"nextInt","bound":10,"n":64}]}"#,
        )
        .unwrap();
        assert_eq!(s.tier, Tier::A);
        let ScenarioKind::JavaRandom { seed, draws } = &s.kind else {
            panic!("{s:?}")
        };
        assert_eq!(*seed, 123);
        assert_eq!(draws[1].bound, Some(10));
        assert_eq!(draws[0].bound, None);

        let s = Scenario::parse(r#"{"name":"sine-table","tier":"A","kind":"sine_table"}"#).unwrap();
        assert_eq!(s.kind, ScenarioKind::SineTable);

        let s = Scenario::parse(
            r#"{"name":"o","tier":"A","kind":"terrain","seed":-4172144991579068934,
             "dimension":"nether","chunks":[{"x":0,"z":0,"full":true},{"x":-3,"z":7}]}"#,
        )
        .unwrap();
        let ScenarioKind::Terrain {
            seed,
            dimension,
            chunks,
        } = s.kind
        else {
            panic!()
        };
        assert_eq!(seed, -4172144991579068934);
        assert_eq!(dimension, Dimension::Nether);
        assert!(chunks[0].full && !chunks[1].full);
    }

    #[test]
    fn unknown_kind_and_bad_fields_rejected() {
        assert!(Scenario::parse(r#"{"name":"x","tier":"A","kind":"pistons"}"#).is_err());
        assert!(Scenario::parse(r#"{"name":"x","tier":"B","kind":"terrain"}"#).is_err());
        assert!(Scenario::parse(r#"{"name":"x","tier":"A"}"#).is_err());
        assert!(Scenario::parse(r#"{"name":"x","tier":"Q","kind":"sine_table"}"#).is_err());
        assert!(
            Scenario::parse(
                r#"{"name":"x","tier":"A","kind":"terrain","seed":1,"dimension":"end","chunks":[]}"#
            )
            .is_err()
        );
    }

    #[test]
    fn edge_cases_lenient_shapes() {
        let per_fn = Scenario::parse(
            r#"{"name":"m","tier":"A","kind":"math_sweep","seed":1,"count":2,
             "functions":["sin","atan2"],
             "edge_cases":{"sin":["0x0000000000000000","0x8000000000000000"],
                           "atan2":[["0x3ff0000000000000","0x0000000000000000"]]}}"#,
        )
        .unwrap();
        assert_eq!(per_fn.kind.edge_cases_for("sin").unwrap().len(), 2);
        let pair = per_fn.kind.edge_cases_for("atan2").unwrap();
        assert_eq!(pair[0].to_f64(), 1.0);

        let shared = Scenario::parse(
            r#"{"name":"m","tier":"A","kind":"math_sweep","seed":1,"count":2,
             "functions":["sin"],"edge_cases":["0x7ff8000000000000"]}"#,
        )
        .unwrap();
        assert!(
            shared.kind.edge_cases_for("cos").unwrap()[0]
                .to_f64()
                .is_nan()
        );

        let bad = Scenario::parse(
            r#"{"name":"m","tier":"A","kind":"math_sweep","seed":1,"count":2,
             "functions":["sin"],"edge_cases":[0.5]}"#,
        )
        .unwrap();
        assert!(bad.kind.edge_cases_for("sin").is_err());
    }

    #[test]
    fn tick_scenario_flexible_floats() {
        let s = Scenario::parse(
            r#"{"name":"t","tier":"C","kind":"tick","seed":1,"dimension":"nether","ticks":2,
             "ops":[{"t":0,"op":"spawn","kind":"Pig","pos":["0x3fe0000000000000",70,0.25],"yaw":90},
                    {"t":1,"op":"set_rotation","id":0,"yaw":"0x42b40000","pitch":0}],
             "dumps":[{"every":1,"what":"entity_state","which":"all"},{"t":1,"what":"rng_state"}]}"#,
        )
        .unwrap();
        let ScenarioKind::Tick(t) = &s.kind else {
            panic!()
        };
        assert_eq!(
            (t.world, t.difficulty, t.save_format.is_none()),
            (WorldSource::Generated, 0, true)
        );
        let OpKind::Spawn { pos, yaw, .. } = &t.ops[0].kind else {
            panic!()
        };
        assert_eq!((*pos, *yaw), ([0.5, 70.0, 0.25], Some(90.0)));
        assert!(matches!(&t.ops[1].kind, OpKind::SetRotation { yaw, .. } if *yaw == 90.0));
        assert!(t.dumps[0].due_at(0) && t.dumps[1].due_at(1) && !t.dumps[1].due_at(0));
        assert_eq!(
            Scenario::parse(&serde_json::to_string(&s).unwrap()).unwrap(),
            s
        );
        let w = Scenario::parse(r#"{"name":"w","tier":"S","kind":"world_read","dir":"world"}"#)
            .unwrap();
        assert!(matches!(
            w.kind,
            ScenarioKind::WorldRead { chunks: None, .. }
        ));
        assert!(
            Scenario::parse(
                r#"{"name":"t","tier":"B","kind":"tick","seed":1,"dimension":"overworld","ticks":1,
             "ops":[{"t":0,"op":"explode"}]}"#
            )
            .is_err()
        );
    }

    #[test]
    fn new_kinds_and_unsupported() {
        let s = Scenario::parse(
            r#"{"name":"m","tier":"A","kind":"math_sweep","seed":1,"count":2,
             "functions":["sin"],"edge_cases":{"sin":[]},"strict":true}"#,
        )
        .unwrap();
        assert!(matches!(
            s.kind,
            ScenarioKind::MathSweep { strict: true, .. }
        ));
        let s = Scenario::parse(
            r#"{"name":"n","tier":"A","kind":"nbt_write","root":{"type":"compound","name":"","value":{}}}"#,
        )
        .unwrap();
        let ScenarioKind::NbtWrite { root } = s.kind else {
            panic!()
        };
        assert_eq!(
            (root.ty.as_str(), root.name.as_deref()),
            ("compound", Some(""))
        );
        let s = Scenario::parse(
            r#"{"name":"r","tier":"A","kind":"nbt_read","file":"level.dat","compressed":"gzip"}"#,
        )
        .unwrap();
        assert!(matches!(s.kind, ScenarioKind::NbtRead { .. }));
        let s = Scenario::parse(
            r#"{"name":"h","tier":"A","kind":"java_hash_order","key_type":"Integer",
             "ops":[{"op":"add","key":5},{"op":"clear"}]}"#,
        )
        .unwrap();
        let ScenarioKind::JavaHashOrder {
            ops, dump_after, ..
        } = s.kind
        else {
            panic!()
        };
        assert_eq!((ops[1].key.is_none(), dump_after.len()), (true, 0));
        let s = Scenario::parse(r#"{"name":"p","tier":"B","kind":"pistons","ticks":3}"#).unwrap();
        let ScenarioKind::Unsupported { kind, raw } = s.kind else {
            panic!()
        };
        assert_eq!((kind.as_str(), raw["ticks"].as_i64()), ("pistons", Some(3)));
    }
}
