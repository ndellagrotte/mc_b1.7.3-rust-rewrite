//! The `TraceLine` event constructors rebuild the oracle's checked-in lines
//! exactly (same keys, same encodings), so ports that emit through them
//! produce the documented shapes. Also checks that each `chunk_digest` agrees
//! with the `chunk_full` arrays after it. Skips fixtures that are absent.

use mc_golden::{
    Bits32, Bits64, ChunkArrays, RngValue, Stage, TraceLine, all_fixture_dirs, parse_hex_bytes,
    parse_jsonl, read_sidecar,
};
use serde_json::Value;

fn read_text(path: &std::path::Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

fn str_of<'a>(l: &'a TraceLine, k: &str) -> &'a str {
    l.get(k)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("{k} in {l:?}"))
}
fn f64_of(l: &TraceLine, k: &str) -> f64 {
    Bits64::parse(str_of(l, k)).unwrap().to_f64()
}
fn int_of(l: &TraceLine, k: &str) -> i64 {
    l.get(k).and_then(Value::as_i64).unwrap()
}

/// Rebuilds a line through the public constructors, or `None` when the line
/// cannot be rebuilt from its own fields.
fn rebuild(l: &TraceLine, prev: Option<&TraceLine>, dir: &std::path::Path) -> Option<TraceLine> {
    Some(match l.e() {
        "rng" => {
            let call = str_of(l, "call");
            let v = match (call, l.get("v")?) {
                (_, Value::Bool(b)) => RngValue::Bool(*b),
                (_, Value::Number(n)) => RngValue::Int(n.as_i64()?),
                ("nextFloat", Value::String(s)) => RngValue::F32(Bits32::parse(s).ok()?.to_f32()),
                (_, Value::String(s)) => RngValue::F64(Bits64::parse(s).ok()?.to_f64()),
                _ => return None,
            };
            let bound = l.get("bound").and_then(Value::as_i64).map(|b| b as i32);
            TraceLine::rng(int_of(l, "i") as u64, call, bound, v)
        }
        "math" => {
            let y = l.get("y").map(|_| f64_of(l, "y"));
            let (f, i) = (str_of(l, "fn"), int_of(l, "i") as u64);
            TraceLine::math(f, i, f64_of(l, "x"), y, f64_of(l, "v"))
        }
        "sine_table" => {
            let bytes = read_sidecar(dir, l.file()?).unwrap();
            TraceLine::sine_table(int_of(l, "len") as u32, &bytes, l.file()?)
        }
        "chunk_full" => {
            let arrays: Vec<Vec<u8>> = ["blocks", "data", "skylight", "blocklight", "heightmap"]
                .iter()
                .map(|k| parse_hex_bytes(str_of(l, k)).unwrap())
                .collect();
            let a = ChunkArrays {
                blocks: &arrays[0],
                data: &arrays[1],
                skylight: &arrays[2],
                blocklight: &arrays[3],
                heightmap: &arrays[4],
            };
            assert_eq!(
                (a.blocks.len(), a.data.len(), a.heightmap.len()),
                (32768, 16384, 256)
            );
            let stage = match str_of(l, "stage") {
                "generated" => Stage::Generated,
                "populated" => Stage::Populated,
                s => panic!("stage {s}"),
            };
            let (x, z) = (int_of(l, "x") as i32, int_of(l, "z") as i32);
            let digest = TraceLine::chunk_digest(x, z, stage, &a);
            assert_eq!(
                Some(&digest),
                prev,
                "chunk_digest must precede chunk_full and match it"
            );
            TraceLine::chunk_full(x, z, stage, &a)
        }
        _ => return None,
    })
}

#[test]
fn constructors_rebuild_oracle_lines() {
    let mut rebuilt = 0usize;
    for dir in all_fixture_dirs().unwrap() {
        let Some(text) = read_text(&dir.join("expected.jsonl")) else {
            continue;
        };
        let lines = parse_jsonl(&text).unwrap();
        for (i, line) in lines.iter().enumerate() {
            let prev = i.checked_sub(1).map(|p| &lines[p]);
            if let Some(ours) = rebuild(line, prev, &dir) {
                assert_eq!(&ours, line, "{}:{}", dir.display(), line.line());
                rebuilt += 1;
            }
            if line.e() == "biome" {
                let keys: Vec<_> = line.fields().keys().map(String::as_str).collect();
                let want = ["biomes", "e", "humidity", "t", "temperature", "x", "z"];
                assert_eq!(keys, want, "{}:{}", dir.display(), line.line());
            }
        }
    }
    println!("rebuilt {rebuilt} oracle lines through the constructors");
}
