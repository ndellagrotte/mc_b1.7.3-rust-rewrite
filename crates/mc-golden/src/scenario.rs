//! `scenario.json`: the one input both engines replay (ADR-007 §5.1,
//! `tests/golden/README.md` "Tier A scenarios").
//!
//! Unknown `kind`s are an error; unknown extra fields are tolerated so the
//! oracle side can add documentation fields without breaking the Rust side.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{Bits64, GoldenError};

/// One fixture's scenario.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Scenario {
    /// Fixture name, normally equal to its directory name.
    pub name: String,
    /// Comparison tier (PLAN_v4 §4.3).
    pub tier: Tier,
    /// What to run; the JSON `"kind"` field selects the variant.
    #[serde(flatten)]
    pub kind: ScenarioKind,
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
}
