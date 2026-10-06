//! Trace lines (`expected.jsonl` / `actual.jsonl`, ADR-007 §5.2).
//!
//! A [`TraceLine`] is a JSON object with an integer `"t"` (tick, `0` for
//! Tier A) and a string `"e"` (event name) plus event fields. The event
//! constructors below produce exactly the shapes documented in
//! `tests/golden/README.md`, so ports emit traces through them rather than by
//! hand.
//!
//! Key order: `serde_json` is used without `preserve_order`, so objects are
//! kept and written with sorted keys. `actual.jsonl` may therefore order keys
//! differently from the oracle's `expected.jsonl`; comparison is by parsed
//! value, where key order does not matter (README "Encoding rules").

use serde::{Serialize, Serializer};
use serde_json::{Map, Value};

use crate::{Bits32, Bits64, blake3_hex, hex_bytes};

/// One trace record.
#[derive(Clone, Debug)]
pub struct TraceLine {
    /// 1-based line number in the file it was parsed from; 0 when built.
    line: usize,
    fields: Map<String, Value>,
}

impl PartialEq for TraceLine {
    /// Equal when the JSON objects are equal; the line number is ignored.
    fn eq(&self, other: &Self) -> bool {
        self.fields == other.fields
    }
}

/// A line of a `.jsonl` file that is not a valid trace record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceParseError {
    /// 1-based line number.
    pub line: usize,
    /// What was wrong.
    pub message: String,
}

impl TraceLine {
    /// A line with only `t` and `e`; add fields with [`TraceLine::with`].
    pub fn new(t: i64, e: &str) -> Self {
        let mut fields = Map::new();
        fields.insert("t".into(), t.into());
        fields.insert("e".into(), e.into());
        Self { line: 0, fields }
    }

    /// Adds (or replaces) a field. Floats must go through [`Bits32`] /
    /// [`Bits64`] (or [`TraceLine::with_f32`] / [`TraceLine::with_f64`]).
    pub fn with(mut self, key: &str, value: impl Into<Value>) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }

    /// Adds an `f32` field as a bit-pattern string.
    pub fn with_f32(self, key: &str, v: f32) -> Self {
        self.with(key, Bits32::from(v))
    }

    /// Adds an `f64` field as a bit-pattern string.
    pub fn with_f64(self, key: &str, v: f64) -> Self {
        self.with(key, Bits64::from(v))
    }

    /// Validates a parsed object: `t` must be an integer, `e` a string.
    pub fn from_map(fields: Map<String, Value>) -> Result<Self, String> {
        if !fields.get("t").is_some_and(Value::is_i64) {
            return Err("missing or non-integer \"t\"".into());
        }
        if !fields.get("e").is_some_and(Value::is_string) {
            return Err("missing or non-string \"e\"".into());
        }
        Ok(Self { line: 0, fields })
    }

    /// The tick.
    pub fn t(&self) -> i64 {
        self.fields["t"].as_i64().unwrap_or_default()
    }

    /// The event name.
    pub fn e(&self) -> &str {
        self.fields["e"].as_str().unwrap_or_default()
    }

    /// A field by name.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.fields.get(key)
    }

    /// The sidecar file this line refers to (README: `"file"` field), if any.
    pub fn file(&self) -> Option<&str> {
        self.fields.get("file").and_then(Value::as_str)
    }

    /// All fields, including `t` and `e`.
    pub fn fields(&self) -> &Map<String, Value> {
        &self.fields
    }

    /// 1-based source line number, or 0 for a line built in code.
    pub fn line(&self) -> usize {
        self.line
    }
}

impl Serialize for TraceLine {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.fields.serialize(s)
    }
}

/// Parses a `.jsonl` trace. Every line must be a trace record; only a final
/// newline may follow the last one (blank lines are rejected so that fixture
/// line numbers stay meaningful).
pub fn parse_jsonl(text: &str) -> Result<Vec<TraceLine>, TraceParseError> {
    let body = text.strip_suffix('\n').unwrap_or(text);
    if body.is_empty() {
        return Ok(Vec::new());
    }
    body.split('\n')
        .enumerate()
        .map(|(i, raw)| {
            let line = i + 1;
            let err = |message: String| TraceParseError { line, message };
            let raw = raw.strip_suffix('\r').unwrap_or(raw);
            let map = serde_json::from_str::<Map<String, Value>>(raw)
                .map_err(|e| err(format!("not a JSON object: {e}")))?;
            let mut parsed = TraceLine::from_map(map).map_err(err)?;
            parsed.line = line;
            Ok(parsed)
        })
        .collect()
}

/// Collects the lines a drive closure emits (ADR-007 §7).
#[derive(Clone, Debug, Default)]
pub struct TraceWriter {
    lines: Vec<TraceLine>,
}

impl TraceWriter {
    /// An empty trace.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a line.
    pub fn push(&mut self, line: TraceLine) {
        self.lines.push(line);
    }

    /// The lines so far.
    pub fn lines(&self) -> &[TraceLine] {
        &self.lines
    }

    /// The trace as `.jsonl`: one compact object per line, `\n`-terminated.
    pub fn to_jsonl(&self) -> String {
        let mut out = String::new();
        for line in &self.lines {
            out.push_str(&Value::Object(line.fields.clone()).to_string());
            out.push('\n');
        }
        out
    }
}

/// `v` of an `rng` line (README `java_random`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RngValue {
    /// `nextInt` / `nextLong`: a JSON number.
    Int(i64),
    /// `nextFloat`: an `f32` bit pattern.
    F32(f32),
    /// `nextDouble` / `nextGaussian`: an `f64` bit pattern.
    F64(f64),
    /// `nextBoolean`: a JSON boolean.
    Bool(bool),
}

/// `chunk_digest` / `chunk_full` stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Straight out of the generator, before population.
    Generated,
    /// After the 3×3 neighbourhood is loaded and the chunk populated.
    Populated,
}

impl Stage {
    /// The JSON spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Generated => "generated",
            Self::Populated => "populated",
        }
    }
}

/// A chunk's canonical byte arrays (README `chunk_digest`): `Chunk.blocks`
/// (32768 bytes), the `data`, `skylightMap` and `blocklightMap` nibble arrays
/// (16384 bytes each) and `heightMap` (256 bytes), all in Java layout.
#[derive(Clone, Copy, Debug)]
pub struct ChunkArrays<'a> {
    /// Block ids, index `x << 11 | z << 7 | y`.
    pub blocks: &'a [u8],
    /// Block metadata nibbles.
    pub data: &'a [u8],
    /// Sky light nibbles.
    pub skylight: &'a [u8],
    /// Block light nibbles.
    pub blocklight: &'a [u8],
    /// Height map, index `z << 4 | x`.
    pub heightmap: &'a [u8],
}

impl ChunkArrays<'_> {
    fn named(&self) -> [(&'static str, &[u8]); 5] {
        [
            ("blocks", self.blocks),
            ("data", self.data),
            ("skylight", self.skylight),
            ("blocklight", self.blocklight),
            ("heightmap", self.heightmap),
        ]
    }
}

/// Event constructors for the Tier A shapes in `tests/golden/README.md`.
/// All use `t = 0`.
impl TraceLine {
    /// `{"t":0,"e":"rng","i":..,"call":..,("bound":..,)"v":..}`.
    pub fn rng(i: u64, call: &str, bound: Option<i32>, v: RngValue) -> Self {
        let mut line = Self::new(0, "rng").with("i", i).with("call", call);
        if let Some(bound) = bound {
            line = line.with("bound", bound);
        }
        match v {
            RngValue::Int(v) => line.with("v", v),
            RngValue::F32(v) => line.with_f32("v", v),
            RngValue::F64(v) => line.with_f64("v", v),
            RngValue::Bool(v) => line.with("v", v),
        }
    }

    /// `{"t":0,"e":"sine_table","len":..,"blake3":..,"file":..}`; `bytes` is
    /// the table as little-endian `f32`s in index order.
    pub fn sine_table(len: u32, bytes: &[u8], file: &str) -> Self {
        Self::new(0, "sine_table")
            .with("len", len)
            .with("blake3", blake3_hex(bytes))
            .with("file", file)
    }

    /// `{"t":0,"e":"math","fn":..,"i":..,"x":..,("y":..,)"v":..}`, all `f64`.
    pub fn math(function: &str, i: u64, x: f64, y: Option<f64>, v: f64) -> Self {
        let mut line = Self::new(0, "math")
            .with("fn", function)
            .with("i", i)
            .with_f64("x", x);
        if let Some(y) = y {
            line = line.with_f64("y", y);
        }
        line.with_f64("v", v)
    }

    /// `chunk_digest`: blake3 of each array.
    pub fn chunk_digest(x: i32, z: i32, stage: Stage, arrays: &ChunkArrays<'_>) -> Self {
        Self::chunk_line("chunk_digest", x, z, stage, arrays, blake3_hex)
    }

    /// `chunk_full`: lower-case hex of each array.
    pub fn chunk_full(x: i32, z: i32, stage: Stage, arrays: &ChunkArrays<'_>) -> Self {
        Self::chunk_line("chunk_full", x, z, stage, arrays, hex_bytes)
    }

    fn chunk_line(
        e: &str,
        x: i32,
        z: i32,
        stage: Stage,
        arrays: &ChunkArrays<'_>,
        encode: fn(&[u8]) -> String,
    ) -> Self {
        let mut line = Self::new(0, e)
            .with("x", x)
            .with("z", z)
            .with("stage", stage.as_str());
        for (name, bytes) in arrays.named() {
            line = line.with(name, encode(bytes));
        }
        line
    }

    /// `biome`: blake3 over the biome names joined with `'\n'` (UTF-8) and
    /// over the temperature and humidity arrays as `f64` little-endian, all in
    /// Java array order. `x`, `z` are chunk coordinates.
    pub fn biome(x: i32, z: i32, names: &[&str], temperature: &[f64], humidity: &[f64]) -> Self {
        let f64_le = |v: &[f64]| v.iter().flat_map(|d| d.to_le_bytes()).collect::<Vec<u8>>();
        Self::new(0, "biome")
            .with("x", x)
            .with("z", z)
            .with("biomes", blake3_hex(names.join("\n").as_bytes()))
            .with("temperature", blake3_hex(&f64_le(temperature)))
            .with("humidity", blake3_hex(&f64_le(humidity)))
    }

    /// `nbt_bytes`: `{"len":..,"blake3":..,"bytes":"<hex>"}`.
    pub fn nbt_bytes(bytes: &[u8]) -> Self {
        Self::new(0, "nbt_bytes")
            .with("len", bytes.len() as u64)
            .with("blake3", blake3_hex(bytes))
            .with("bytes", hex_bytes(bytes))
    }

    /// `nbt_tree`: `{"root":<canonical tree>}`.
    pub fn nbt_tree(root: Value) -> Self {
        Self::new(0, "nbt_tree").with("root", root)
    }

    /// `nbt_roundtrip`: `{"equal":<bool>}`. (The oracle adds `"reread"` when
    /// `false`; use [`TraceLine::with`] for that.)
    pub fn nbt_roundtrip(equal: bool) -> Self {
        Self::new(0, "nbt_roundtrip").with("equal", equal)
    }

    /// `nbt_gzip`: digest and length of the gzip-round-tripped tree written
    /// uncompressed; `uncompressed` is those bytes.
    pub fn nbt_gzip(uncompressed: &[u8]) -> Self {
        Self::new(0, "nbt_gzip")
            .with(
                "blake3_of_uncompressed_after_gunzip",
                blake3_hex(uncompressed),
            )
            .with("len", uncompressed.len() as u64)
    }

    /// `nbt_file`: the file as stored (`raw`) and its decompressed content.
    pub fn nbt_file(file: &str, compressed: &str, raw: &[u8], uncompressed: &[u8]) -> Self {
        Self::new(0, "nbt_file")
            .with("file", file)
            .with("compressed", compressed)
            .with("len", raw.len() as u64)
            .with("blake3", blake3_hex(raw))
            .with("uncompressed_len", uncompressed.len() as u64)
            .with("uncompressed_blake3", blake3_hex(uncompressed))
    }

    /// `hash_codes`: `{"key_type":..,"samples":[{"key":..,"hash":..},..]}`.
    pub fn hash_codes(key_type: &str, samples: Vec<(Value, i32)>) -> Self {
        let samples: Vec<Value> = samples
            .into_iter()
            .map(|(key, hash)| serde_json::json!({"key": key, "hash": hash}))
            .collect();
        Self::new(0, "hash_codes")
            .with("key_type", key_type)
            .with("samples", samples)
    }

    /// `hash_order`: set contents after `after` ops, in iteration order.
    pub fn hash_order(after: usize, hashes: &[i32], keys: &[Value]) -> Self {
        Self::new(0, "hash_order")
            .with("after", after as u64)
            .with("size", hashes.len() as u64)
            .with("hashes", hashes.to_vec())
            .with("keys", keys.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_write_round_trip() {
        let text = "{\"t\":0,\"e\":\"rng\",\"i\":0,\"call\":\"nextInt\",\"v\":-1}\n\
                    {\"e\":\"x\",\"t\":3,\"s\":{\"a\":\"0x3f800000\"}}\n";
        let lines = parse_jsonl(text).unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!((lines[1].line(), lines[1].t(), lines[1].e()), (2, 3, "x"));
        let mut w = TraceWriter::new();
        lines.iter().cloned().for_each(|l| w.push(l));
        assert_eq!(parse_jsonl(&w.to_jsonl()).unwrap(), lines);
    }

    #[test]
    fn parse_rejects_bad_lines() {
        assert_eq!(parse_jsonl("{\"t\":0}\n").unwrap_err().line, 1);
        assert_eq!(
            parse_jsonl("{\"t\":0,\"e\":\"a\"}\n\n").unwrap_err().line,
            2
        );
        assert!(parse_jsonl("{\"t\":0.5,\"e\":\"a\"}").is_err());
        assert!(parse_jsonl("[1]").is_err());
        assert!(parse_jsonl("").unwrap().is_empty());
    }

    #[test]
    fn nbt_and_hash_shapes() {
        let s = |l: TraceLine| Value::Object(l.fields).to_string();
        assert_eq!(
            s(TraceLine::nbt_bytes(&[0, 255])),
            format!(
                r#"{{"blake3":"{}","bytes":"00ff","e":"nbt_bytes","len":2,"t":0}}"#,
                blake3_hex(&[0, 255])
            )
        );
        assert_eq!(
            s(TraceLine::nbt_roundtrip(false)),
            r#"{"e":"nbt_roundtrip","equal":false,"t":0}"#
        );
        assert_eq!(
            s(TraceLine::hash_order(
                2,
                &[1, -2],
                &[serde_json::json!([0, 1]), serde_json::json!([2, 3])]
            )),
            r#"{"after":2,"e":"hash_order","hashes":[1,-2],"keys":[[0,1],[2,3]],"size":2,"t":0}"#
        );
        assert_eq!(
            s(TraceLine::hash_codes("Integer", vec![(5.into(), 5)])),
            r#"{"e":"hash_codes","key_type":"Integer","samples":[{"hash":5,"key":5}],"t":0}"#
        );
    }

    #[test]
    fn event_shapes() {
        let s = |l: TraceLine| Value::Object(l.fields).to_string();
        assert_eq!(
            s(TraceLine::rng(3, "nextInt", Some(10), RngValue::Int(7))),
            r#"{"bound":10,"call":"nextInt","e":"rng","i":3,"t":0,"v":7}"#
        );
        assert_eq!(
            s(TraceLine::rng(0, "nextDouble", None, RngValue::F64(0.5))),
            r#"{"call":"nextDouble","e":"rng","i":0,"t":0,"v":"0x3fe0000000000000"}"#
        );
        assert_eq!(
            s(TraceLine::math("atan2", 1, 1.0, Some(0.0), 0.0)),
            r#"{"e":"math","fn":"atan2","i":1,"t":0,"v":"0x0000000000000000","x":"0x3ff0000000000000","y":"0x0000000000000000"}"#
        );
        let a = ChunkArrays {
            blocks: &[1, 2],
            data: &[],
            skylight: &[],
            blocklight: &[],
            heightmap: &[0xff],
        };
        let full = TraceLine::chunk_full(-1, 2, Stage::Populated, &a);
        assert_eq!(full.get("blocks").unwrap(), "0102");
        assert_eq!(full.get("stage").unwrap(), "populated");
        let digest = TraceLine::chunk_digest(-1, 2, Stage::Generated, &a);
        assert_eq!(
            digest.get("data").unwrap().as_str(),
            Some(&*blake3_hex(&[]))
        );
        let biome = TraceLine::biome(0, 0, &["a", "b"], &[0.5], &[]);
        assert_eq!(
            biome.get("biomes").unwrap().as_str(),
            Some(&*blake3_hex(b"a\nb"))
        );
        assert_eq!(
            biome.get("temperature").unwrap().as_str(),
            Some(&*blake3_hex(&0.5f64.to_le_bytes()))
        );
        let sine = TraceLine::sine_table(1, &[0; 4], "sine_table.f32le.bin");
        assert_eq!(sine.file(), Some("sine_table.f32le.bin"));
    }
}
