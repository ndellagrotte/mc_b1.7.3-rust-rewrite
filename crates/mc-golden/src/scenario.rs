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
    const KNOWN: [&'static str; 7] = [
        "java_random",
        "sine_table",
        "math_sweep",
        "terrain",
        "nbt_write",
        "nbt_read",
        "java_hash_order",
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
