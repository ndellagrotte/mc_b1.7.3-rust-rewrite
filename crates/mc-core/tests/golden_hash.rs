//! `tests/golden/hash/*`: `java.util.HashSet` iteration order (README
//! `java_hash_order`), replayed on [`JavaHashSet`].

use mc_core::java_hash::{ChunkCoordIntPair, ChunkCoordinates, ChunkPosition};
use mc_core::{JavaHash, JavaHashMap, JavaHashSet};
use mc_golden::{GoldenError, HashOp, Scenario, ScenarioKind, TraceLine, TraceWriter, run};

/// The op's key as `i32`s (`[x, z]` / `[x, y, z]`, or one int).
fn ints<const N: usize>(op: &HashOp) -> Result<[i32; N], GoldenError> {
    let bad = || GoldenError::other(format!("bad key for {N} ints: {:?}", op.key));
    let key = op.key.as_ref().ok_or_else(bad)?;
    let one;
    let items = match key.as_array() {
        Some(items) => items.as_slice(),
        None => {
            one = [key.clone()];
            &one[..]
        }
    };
    if items.len() != N {
        return Err(bad());
    }
    let mut out = [0; N];
    for (o, v) in out.iter_mut().zip(items) {
        *o = v
            .as_i64()
            .and_then(|v| i32::try_from(v).ok())
            .ok_or_else(bad)?;
    }
    Ok(out)
}

fn ccip(op: &HashOp) -> Result<ChunkCoordIntPair, GoldenError> {
    let [x, z] = ints(op)?;
    Ok(ChunkCoordIntPair::new(x, z))
}

fn chunk_position(op: &HashOp) -> Result<ChunkPosition, GoldenError> {
    let [x, y, z] = ints(op)?;
    Ok(ChunkPosition::new(x, y, z))
}

fn chunk_coordinates(op: &HashOp) -> Result<ChunkCoordinates, GoldenError> {
    let [x, y, z] = ints(op)?;
    Ok(ChunkCoordinates::new(x, y, z))
}

fn integer(op: &HashOp) -> Result<i32, GoldenError> {
    let [v] = ints(op)?;
    Ok(v)
}

fn string(op: &HashOp) -> Result<String, GoldenError> {
    op.key
        .as_ref()
        .and_then(|k| k.as_str())
        .map(str::to_owned)
        .ok_or_else(|| GoldenError::other(format!("bad String key: {:?}", op.key)))
}

/// Applies `ops` to one `JavaHashSet<K>` and emits `hash_codes` and the
/// `hash_order` dumps.
fn replay<K: JavaHash + Clone>(
    key_type: &str,
    ops: &[HashOp],
    dump_after: &[usize],
    parse: fn(&HashOp) -> Result<K, GoldenError>,
    out: &mut TraceWriter,
) -> Result<(), GoldenError> {
    // Each distinct key's JSON as written in the scenario, to echo it back.
    let mut json = JavaHashMap::new();
    let mut samples = Vec::new();
    let mut set = JavaHashSet::new();
    let mut dumps = Vec::new();
    for (i, op) in ops.iter().enumerate() {
        match op.op.as_str() {
            "add" | "remove" => {
                let key = parse(op)?;
                if !json.contains_key(&key) {
                    let raw = op.key.clone().expect("parsed key");
                    if samples.len() < 32 {
                        samples.push((raw.clone(), key.java_hash()));
                    }
                    json.insert(key.clone(), raw);
                }
                if op.op == "add" {
                    set.insert(key);
                } else {
                    set.remove(&key);
                }
            }
            "clear" => set.clear(),
            other => return Err(GoldenError::other(format!("unknown op {other}"))),
        }
        let n = i + 1;
        if dump_after.contains(&n) || n == ops.len() {
            let hashes: Vec<i32> = set.iter().map(JavaHash::java_hash).collect();
            let keys: Vec<_> = set
                .iter()
                .map(|k| json.get(k).expect("seen").clone())
                .collect();
            dumps.push(TraceLine::hash_order(n, &hashes, &keys));
        }
    }
    out.push(TraceLine::hash_codes(key_type, samples));
    for line in dumps {
        out.push(line);
    }
    Ok(())
}

fn drive(scenario: &Scenario, out: &mut TraceWriter) -> Result<(), GoldenError> {
    let ScenarioKind::JavaHashOrder {
        key_type,
        ops,
        dump_after,
    } = &scenario.kind
    else {
        return Err(GoldenError::other("not a java_hash_order scenario"));
    };
    match key_type.as_str() {
        "ChunkCoordIntPair" => replay(key_type, ops, dump_after, ccip, out),
        "ChunkPosition" => replay(key_type, ops, dump_after, chunk_position, out),
        "ChunkCoordinates" => replay(key_type, ops, dump_after, chunk_coordinates, out),
        "Integer" => replay(key_type, ops, dump_after, integer, out),
        "String" => replay(key_type, ops, dump_after, string, out),
        other => Err(GoldenError::other(format!("unknown key_type {other}"))),
    }
}

fn check(name: &str) {
    let dir = mc_golden::fixture_dir("hash", name);
    if let Err(e) = run(&dir, drive) {
        panic!("{name}: {e}");
    }
}

#[test]
fn chunkcoordinates() {
    check("chunkcoordinates");
}

#[test]
fn chunkcoordintpair_active_chunks() {
    check("chunkcoordintpair-active-chunks");
}

#[test]
fn chunkcoordintpair_spiral() {
    check("chunkcoordintpair-spiral");
}

#[test]
fn chunkposition_explosion() {
    check("chunkposition-explosion");
}

#[test]
fn integer_chunk_keys() {
    check("integer-chunk-keys");
}

#[test]
fn string_names() {
    check("string-names");
}

/// Every committed hash fixture is covered by a test above.
#[test]
fn all_hash_fixtures_listed() {
    let names: Vec<String> = mc_golden::all_fixture_dirs()
        .unwrap()
        .into_iter()
        .filter(|d| d.parent().and_then(|p| p.file_name()) == Some("hash".as_ref()))
        .map(|d| d.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        names,
        [
            "chunkcoordinates",
            "chunkcoordintpair-active-chunks",
            "chunkcoordintpair-spiral",
            "chunkposition-explosion",
            "integer-chunk-keys",
            "string-names"
        ]
    );
}
