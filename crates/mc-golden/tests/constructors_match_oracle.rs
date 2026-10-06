//! The `TraceLine` event constructors rebuild the oracle's checked-in lines
//! exactly (same keys, same encodings), so ports that emit through them
//! produce the documented shapes. Also checks that each `chunk_digest` agrees
//! with the `chunk_full` arrays after it. Skips fixtures that are absent.

use mc_golden::{
    Bits32, Bits64, ChunkArrays, EntityLine, RngValue, ScheduledTick, Stage, TileItem, TraceLine,
    WorldInfoLine, all_fixture_dirs, parse_hex_bytes, parse_jsonl, read_sidecar,
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
/// Bytes of the first `nbt_bytes` line yielded by `it`.
fn first_nbt_bytes<'a>(mut it: impl Iterator<Item = &'a TraceLine>) -> Option<Vec<u8>> {
    let b = it.find(|x| x.e() == "nbt_bytes")?;
    Some(parse_hex_bytes(b.get("bytes")?.as_str()?).unwrap())
}

const ARRAY_KEYS: [&str; 5] = ["blocks", "data", "skylight", "blocklight", "heightmap"];

/// The five arrays of a `chunk_full` line.
fn full_arrays(l: &TraceLine) -> Vec<Vec<u8>> {
    ARRAY_KEYS
        .iter()
        .map(|k| parse_hex_bytes(str_of(l, k)).unwrap())
        .collect()
}

fn as_arrays(a: &[Vec<u8>]) -> ChunkArrays<'_> {
    ChunkArrays {
        blocks: &a[0],
        data: &a[1],
        skylight: &a[2],
        blocklight: &a[3],
        heightmap: &a[4],
    }
}

fn stage_of(l: &TraceLine) -> Stage {
    match str_of(l, "stage") {
        "generated" => Stage::Generated,
        "populated" => Stage::Populated,
        "tick" => Stage::Tick,
        "loaded" => Stage::Loaded,
        s => panic!("stage {s}"),
    }
}

fn hex_of(l: &TraceLine, k: &str) -> Vec<u8> {
    parse_hex_bytes(str_of(l, k)).unwrap()
}

fn i32_of(l: &TraceLine, k: &str) -> i32 {
    int_of(l, k) as i32
}

fn bool_of(l: &TraceLine, k: &str) -> bool {
    l.get(k).and_then(Value::as_bool).unwrap()
}

/// Rebuilds an `entity` line key by key through the builder: the encoding of
/// each value (bit patterns by width, integers, booleans, `state` object)
/// picks the typed helper.
fn rebuild_entity(l: &TraceLine) -> TraceLine {
    let mut b = EntityLine::new(
        l.t(),
        int_of(l, "idx") as u32,
        i32_of(l, "id"),
        str_of(l, "kind"),
    );
    for (k, v) in l.fields() {
        if matches!(k.as_str(), "t" | "e" | "idx" | "id" | "kind") {
            continue;
        }
        b = match v {
            Value::String(s) if s.len() == 18 => b.f64(k, Bits64::parse(s).unwrap().to_f64()),
            Value::String(s) if s.len() == 10 => b.f32(k, Bits32::parse(s).unwrap().to_f32()),
            Value::Bool(v) => b.bool(k, *v),
            Value::Number(n) => b.int(k, n.as_i64().unwrap()),
            Value::Object(_) if k == "state" => b.state(v.clone()),
            other => panic!("entity field {k}: {other:?}"),
        };
    }
    b.build()
}

fn rebuild(lines: &[TraceLine], i: usize, dir: &std::path::Path) -> Option<TraceLine> {
    let l = &lines[i];
    let prev = i.checked_sub(1).map(|p| &lines[p]);
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
            let arrays = full_arrays(l);
            let a = as_arrays(&arrays);
            assert_eq!(
                (a.blocks.len(), a.data.len(), a.heightmap.len()),
                (32768, 16384, 256)
            );
            let (x, z) = (i32_of(l, "x"), i32_of(l, "z"));
            let digest = TraceLine::chunk_digest_at(l.t(), x, z, stage_of(l), &a);
            assert_eq!(
                Some(&digest),
                prev,
                "chunk_digest must precede chunk_full and match it"
            );
            TraceLine::chunk_full_at(l.t(), x, z, stage_of(l), &a)
        }
        // Only when a `chunk_full` follows to take the arrays from.
        "chunk_digest" => {
            let full = lines.get(i + 1).filter(|n| n.e() == "chunk_full")?;
            let arrays = full_arrays(full);
            let (x, z) = (i32_of(l, "x"), i32_of(l, "z"));
            let a = as_arrays(&arrays);
            if stage_of(l) == Stage::Tick {
                TraceLine::chunk_digest_tick(l.t(), x, z, &a)
            } else {
                TraceLine::chunk_digest_at(l.t(), x, z, stage_of(l), &a)
            }
        }
        "block" => TraceLine::block(
            l.t(),
            i32_of(l, "x"),
            i32_of(l, "y"),
            i32_of(l, "z"),
            i32_of(l, "id"),
            i32_of(l, "meta"),
            i32_of(l, "skylight"),
            i32_of(l, "blocklight"),
        ),
        "blocks_full" => TraceLine::blocks_full(
            l.t(),
            [i32_of(l, "x0"), i32_of(l, "y0"), i32_of(l, "z0")],
            [i32_of(l, "x1"), i32_of(l, "y1"), i32_of(l, "z1")],
            &hex_of(l, "ids"),
            &hex_of(l, "metas"),
            &hex_of(l, "skylight"),
            &hex_of(l, "blocklight"),
        ),
        "world_time" => TraceLine::world_time(l.t(), int_of(l, "time")),
        "rng_state" => TraceLine::rng_state(
            l.t(),
            int_of(l, "world_rand"),
            i32_of(l, "lcg"),
            int_of(l, "math_random"),
        ),
        // Without `entries` the constructor has nothing to hash.
        "scheduled_ticks" => {
            let rows = l.get("entries")?.as_array()?;
            let entries: Vec<ScheduledTick> = rows
                .iter()
                .map(|r| {
                    let n = |i: usize| r[i].as_i64().unwrap();
                    ScheduledTick {
                        x: n(0) as i32,
                        y: n(1) as i32,
                        z: n(2) as i32,
                        block_id: n(3) as i32,
                        time: n(4),
                    }
                })
                .collect();
            assert_eq!(entries.len() as i64, int_of(l, "count"));
            TraceLine::scheduled_ticks(l.t(), &entries, true)
        }
        "entity" => rebuild_entity(l),
        "entity_absent" => TraceLine::entity_absent(l.t(), i32_of(l, "id")),
        "preload" if l.get("implicit").is_some() => {
            assert!(bool_of(l, "implicit"));
            TraceLine::preload_implicit(int_of(l, "loaded") as u64)
        }
        "level_dat" => {
            let masked: Vec<&str> = l
                .get("masked")?
                .as_array()?
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect();
            TraceLine::level_dat(l.t(), l.get("root")?.clone(), &masked)
        }
        "world_info" => TraceLine::world_info(
            l.t(),
            &WorldInfoLine {
                random_seed: int_of(l, "RandomSeed"),
                spawn: [
                    i32_of(l, "SpawnX"),
                    i32_of(l, "SpawnY"),
                    i32_of(l, "SpawnZ"),
                ],
                time: int_of(l, "Time"),
                level_name: str_of(l, "LevelName"),
                version: i32_of(l, "version"),
                rain_time: i32_of(l, "rainTime"),
                raining: bool_of(l, "raining"),
                thunder_time: i32_of(l, "thunderTime"),
                thundering: bool_of(l, "thundering"),
                dimension: i32_of(l, "dimension"),
                player: bool_of(l, "player"),
            },
        ),
        "region_header" => {
            let present: Vec<u32> = l
                .get("present")?
                .as_array()?
                .iter()
                .map(|v| v.as_u64().unwrap() as u32)
                .collect();
            assert_eq!(present.len() as i64, int_of(l, "count"));
            let dbg = l.get("dbg")?;
            let locations: Vec<i32> = dbg["locations"]
                .as_array()?
                .iter()
                .map(|v| v.as_i64().unwrap() as i32)
                .collect();
            TraceLine::region_header(
                l.t(),
                str_of(l, "name"),
                &present,
                bool_of(l, "timestamps_match"),
                dbg["len"].as_u64()?,
                &locations,
            )
        }
        "chunk_nbt" => TraceLine::chunk_nbt(
            l.t(),
            i32_of(l, "x"),
            i32_of(l, "z"),
            l.get("root")?.clone(),
        ),
        "chunk_absent" => TraceLine::chunk_absent(l.t(), i32_of(l, "x"), i32_of(l, "z")),
        "tile_entity" if !bool_of(l, "known") => {
            TraceLine::tile_entity_unknown(l.t(), int_of(l, "index") as u32, str_of(l, "id"))
        }
        "tile_entity" => {
            let items: Option<Vec<TileItem>> = l.get("items").and_then(Value::as_array).map(|a| {
                a.iter()
                    .map(|i| {
                        let n = |k: &str| i[k].as_i64().unwrap() as i32;
                        TileItem {
                            slot: n("slot"),
                            id: n("id"),
                            count: n("count"),
                            damage: n("damage"),
                        }
                    })
                    .collect()
            });
            let inventory = items.as_deref().map(|i| (int_of(l, "size") as u32, i));
            TraceLine::tile_entity(
                l.t(),
                int_of(l, "index") as u32,
                str_of(l, "id"),
                str_of(l, "class"),
                [i32_of(l, "x"), i32_of(l, "y"), i32_of(l, "z")],
                inventory,
            )
        }
        "nbt_bytes" => TraceLine::nbt_bytes(&parse_hex_bytes(str_of(l, "bytes")).unwrap()),
        "nbt_tree" => TraceLine::nbt_tree(l.get("root")?.clone()),
        "nbt_roundtrip" if l.get("reread").is_none() => {
            TraceLine::nbt_roundtrip(l.get("equal")?.as_bool()?)
        }
        // The re-written bytes are the same bytes as the `nbt_bytes` line.
        "nbt_gzip" => TraceLine::nbt_gzip(&first_nbt_bytes(lines[..i].iter().rev())?),
        "nbt_file" => {
            let raw = read_sidecar(dir, l.file()?).unwrap();
            // README: for the committed fixtures the gunzipped content is
            // what `nbt_bytes` re-writes.
            let unc = first_nbt_bytes(lines[i..].iter())?;
            TraceLine::nbt_file(l.file()?, str_of(l, "compressed"), &raw, &unc)
        }
        "hash_codes" => {
            let samples = l
                .get("samples")?
                .as_array()?
                .iter()
                .map(|s| (s["key"].clone(), s["hash"].as_i64().unwrap() as i32))
                .collect();
            TraceLine::hash_codes(str_of(l, "key_type"), samples)
        }
        "hash_order" => {
            let ints = |k: &str| -> Vec<i32> {
                l.get(k)
                    .unwrap()
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_i64().unwrap() as i32)
                    .collect()
            };
            let keys = l.get("keys")?.as_array()?.clone();
            let hashes = ints("hashes");
            assert_eq!(int_of(l, "size") as usize, hashes.len());
            TraceLine::hash_order(int_of(l, "after") as usize, &hashes, &keys)
        }
        _ => return None,
    })
}

#[test]
fn constructors_rebuild_oracle_lines() {
    let mut rebuilt = 0usize;
    let mut per_event = std::collections::BTreeMap::<String, usize>::new();
    for dir in all_fixture_dirs().unwrap() {
        let Some(text) = read_text(&dir.join("expected.jsonl")) else {
            continue;
        };
        let lines = parse_jsonl(&text).unwrap();
        for (i, line) in lines.iter().enumerate() {
            if let Some(ours) = rebuild(&lines, i, &dir) {
                assert_eq!(&ours, line, "{}:{}", dir.display(), line.line());
                rebuilt += 1;
                *per_event.entry(line.e().to_string()).or_default() += 1;
            }
            if line.e() == "biome" {
                let keys: Vec<_> = line.fields().keys().map(String::as_str).collect();
                let want = ["biomes", "e", "humidity", "t", "temperature", "x", "z"];
                assert_eq!(keys, want, "{}:{}", dir.display(), line.line());
            }
        }
    }
    println!("rebuilt {rebuilt} oracle lines through the constructors: {per_event:?}");
    // The Tier B/C/S fixtures are committed; every shape must have been
    // exercised at least once (`chunk_digest` from a `chunk_full` and
    // `chunk_absent`/`entity_absent`/unknown tile entities have no fixture).
    for e in [
        "block",
        "blocks_full",
        "world_time",
        "rng_state",
        "scheduled_ticks",
        "entity",
        "level_dat",
        "world_info",
        "region_header",
        "chunk_nbt",
        "tile_entity",
    ] {
        assert!(per_event.contains_key(e), "no `{e}` line rebuilt");
    }
}
