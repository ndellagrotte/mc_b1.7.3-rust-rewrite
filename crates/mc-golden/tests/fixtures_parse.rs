//! Every checked-in fixture parses under the format contract
//! (`tests/golden/README.md`): scenarios deserialize, every trace line is a
//! record, every `"0x…"` string is a strict bit pattern, every `blake3` field
//! is a 64-digit lower-case digest, and every sidecar exists and matches the
//! `blake3` on its line (lines with both `file` and `blake3`).

use std::path::Path;

use mc_golden::bits::decode_float;
use mc_golden::digest::is_blake3_hex;
use mc_golden::{
    Scenario, ScenarioKind, Tier, TraceLine, all_fixture_dirs, parse_jsonl, read_sidecar,
};
use serde_json::Value;

/// Checks one JSON value; `key` is the field name it sits under.
fn check_value(key: &str, value: &Value, at: &str) {
    match value {
        Value::String(s) => {
            if s.starts_with("0x") {
                assert!(
                    decode_float(s).is_some(),
                    "{at}: `{key}`: bad bit pattern {s:?}"
                );
            }
            if key == "blake3" {
                assert!(is_blake3_hex(s), "{at}: `blake3` is not a digest: {s:?}");
            }
        }
        Value::Array(items) => items.iter().for_each(|v| check_value(key, v, at)),
        Value::Object(map) => map.iter().for_each(|(k, v)| check_value(k, v, at)),
        _ => {}
    }
}

fn read_text(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// The scenario model agrees with the oracle on how many lines it emits;
/// for `math_sweep` this pins the reading of `edge_cases`.
fn check_counts(scenario: &Scenario, lines: &[TraceLine], at: &str) {
    match &scenario.kind {
        ScenarioKind::JavaRandom { draws, .. } => {
            let n: u32 = draws.iter().map(|d| d.n).sum();
            assert_eq!(lines.len(), n as usize, "{at}: rng line count");
        }
        ScenarioKind::MathSweep {
            count, functions, ..
        } => {
            for f in functions {
                let of_fn: Vec<_> = lines
                    .iter()
                    .filter(|l| l.get("fn") == Some(&f.as_str().into()))
                    .collect();
                let arity = if of_fn.first().is_some_and(|l| l.get("y").is_some()) {
                    2
                } else {
                    1
                };
                let edge = scenario
                    .kind
                    .edge_cases_for(f)
                    .unwrap_or_else(|e| panic!("{at}: {e}"));
                assert_eq!(edge.len() % arity, 0, "{at}: {f} edge cases");
                assert_eq!(
                    of_fn.len(),
                    edge.len() / arity + *count as usize,
                    "{at}: {f} line count"
                );
                assert_eq!(
                    of_fn[0].get("x"),
                    Some(&edge[0].into()),
                    "{at}: {f} first input"
                );
            }
        }
        _ => {}
    }
}

/// Tier B/C/S scenarios are typed, ops are inside the tick range and every
/// dump has exactly one of `every` and `t`.
fn check_tick_model(scenario: &Scenario, dir: &Path) {
    let at = dir.display();
    match &scenario.kind {
        ScenarioKind::Tick(t) => {
            assert!(t.ticks >= 1, "{at}: ticks");
            assert!((0..=3).contains(&t.difficulty), "{at}: difficulty");
            for op in &t.ops {
                assert!(op.t < t.ticks, "{at}: op {op:?} at t >= ticks");
            }
            for d in &t.dumps {
                assert!(
                    d.every.is_some() != d.t.is_some(),
                    "{at}: dump {d:?} needs `every` xor `t`"
                );
                assert!(d.every != Some(0), "{at}: every 0");
                if let Some(dt) = d.t {
                    assert!(dt < t.ticks, "{at}: dump {d:?} at t >= ticks");
                }
            }
            // Lines are emitted at ticks, so every line's `t` is < ticks.
            if let Ok(text) = std::fs::read_to_string(dir.join("expected.jsonl")) {
                for l in parse_jsonl(&text).unwrap() {
                    assert!(l.t() >= 0 && l.t() < i64::from(t.ticks), "{at}: {l:?}");
                }
            }
            // Serialising and re-parsing is lossless (floats become bit
            // patterns, which parse to the same values).
            let json = serde_json::to_string(scenario).unwrap();
            assert_eq!(
                &Scenario::parse(&json).unwrap(),
                scenario,
                "{at}: round trip"
            );
        }
        ScenarioKind::WorldRead { dir: sidecar, .. } => {
            assert!(
                dir.join(sidecar).join("level.dat").is_file(),
                "{at}: save dir"
            );
        }
        ScenarioKind::Unsupported { kind, .. } => {
            panic!("{at}: kind `{kind}` is still Unsupported")
        }
        _ => {}
    }
    if matches!(scenario.tier, Tier::B | Tier::C | Tier::S) {
        assert!(
            matches!(
                scenario.kind,
                ScenarioKind::Tick(_) | ScenarioKind::WorldRead { .. }
            ),
            "{at}: tier {:?} scenario is not tick/world_read",
            scenario.tier
        );
    }
}

#[test]
fn all_fixtures_parse() {
    let dirs = all_fixture_dirs().unwrap();
    if dirs.is_empty() {
        println!("note: no tests/golden/*/*/scenario.json yet; nothing to check");
        return;
    }
    for dir in &dirs {
        let scenario = Scenario::load(dir).unwrap_or_else(|e| panic!("{e}"));
        let at = dir.display();
        check_tick_model(&scenario, dir);
        let Some(text) = read_text(&dir.join("expected.jsonl")) else {
            println!("note: {at}: no expected.jsonl yet");
            continue;
        };
        let lines = parse_jsonl(&text).unwrap_or_else(|e| panic!("{at}:{}: {}", e.line, e.message));
        for line in &lines {
            let at = format!("{at}/expected.jsonl:{}", line.line());
            line.fields()
                .iter()
                .for_each(|(k, v)| check_value(k, v, &at));
            if let Some(file) = line.file() {
                let bytes = read_sidecar(dir, file).unwrap_or_else(|e| panic!("{at}: {e}"));
                // `blake3` on a line with `file` is over the sidecar's bytes
                // (for `nbt_file`: the raw file as stored).
                if let Some(Value::String(digest)) = line.get("blake3") {
                    assert_eq!(
                        &mc_golden::blake3_hex(&bytes),
                        digest,
                        "{at}: sidecar {file}"
                    );
                }
            }
        }
        check_counts(&scenario, &lines, &at.to_string());
        println!("ok: {} ({} lines)", scenario.name, lines.len());
    }
}
