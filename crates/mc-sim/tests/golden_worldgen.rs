//! Tier A terrain fixtures (`tests/golden/worldgen/*`) run through
//! `mc_golden::terrain_driver::drive` and compared with the Java oracle's
//! `expected.jsonl` (PLAN_v4 §4.3, tests/golden/README.md "`kind: "terrain"`").
//!
//! Lines whose stage the port cannot produce yet are left out of the
//! comparison on **both** sides, each stage with the work package that
//! switches it on ([`SKIPPED_STAGES`]): delete the entry and the lines are
//! compared like every other. Everything else (the `generated` chunk_digest /
//! chunk_full lines and the `biome` lines) must match exactly.

use mc_golden::terrain_driver::drive;
use mc_golden::{
    GoldenError, Scenario, ScenarioKind, TraceLine, TraceWriter, compare, fixture_dir, parse_jsonl,
};

/// `(value of "stage", reason)` of the trace lines not compared yet.
const SKIPPED_STAGES: &[(&str, &str)] = &[("populated", "population: WP-1.7")];

/// Why `line` is not compared, if it is not.
fn skip_reason(line: &TraceLine) -> Option<&'static str> {
    let stage = line.get("stage")?.as_str()?;
    SKIPPED_STAGES
        .iter()
        .find(|(s, _)| *s == stage)
        .map(|(_, reason)| *reason)
}

fn expected_lines(name: &str) -> Vec<TraceLine> {
    let path = fixture_dir("worldgen", name).join("expected.jsonl");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    parse_jsonl(&text).unwrap_or_else(|e| panic!("parse {}: {e:?}", path.display()))
}

fn actual_lines(scenario: &Scenario) -> TraceWriter {
    let mut w = TraceWriter::new();
    drive(scenario, &mut w).unwrap_or_else(|e| panic!("{}: {e}", scenario.name));
    w
}

/// Runs one fixture with [`SKIPPED_STAGES`] filtered out of both traces.
fn check(name: &str) {
    let dir = fixture_dir("worldgen", name);
    let scenario = Scenario::load(&dir).expect("load scenario");
    let expected = expected_lines(name);
    let actual = actual_lines(&scenario);

    let mut skipped = 0usize;
    let exp: Vec<TraceLine> = expected
        .into_iter()
        .filter(|l| {
            let s = skip_reason(l).is_some();
            skipped += usize::from(s);
            !s
        })
        .collect();
    let act: Vec<TraceLine> = actual
        .lines()
        .iter()
        .filter(|l| skip_reason(l).is_none())
        .cloned()
        .collect();
    assert!(!exp.is_empty(), "worldgen/{name}: nothing left to compare");
    for (stage, reason) in SKIPPED_STAGES {
        eprintln!("worldgen/{name}: \"stage\":\"{stage}\" lines not compared ({reason})");
    }
    eprintln!(
        "worldgen/{name}: {} lines compared, {skipped} skipped",
        exp.len()
    );
    if let Err(mismatch) = compare(&exp, &act) {
        let e = GoldenError::Mismatch {
            mismatch,
            expected: dir.join("expected.jsonl"),
            actual: "<in memory>".into(),
        };
        panic!("worldgen/{name}: {e}");
    }
}

macro_rules! fixtures {
    ($( $test:ident => $name:literal; )*) => {
        $(
            #[test]
            fn $test() {
                check($name);
            }
        )*
        /// Every fixture name that has a test function above.
        const COVERED: &[&str] = &[$($name),*];
    };
}

fixtures! {
    overworld_seed_123 => "overworld-seed-123";
    overworld_seed_neg_4172144991579068934 => "overworld-seed--4172144991579068934";
    overworld_seed_8675309 => "overworld-seed-8675309";
    nether_seed_123 => "nether-seed-123";
    nether_seed_neg_4172144991579068934 => "nether-seed--4172144991579068934";
    nether_seed_8675309 => "nether-seed-8675309";
}

/// Every `tests/golden/worldgen/<name>` with a `scenario.json` has a test.
#[test]
fn every_worldgen_fixture_has_a_test() {
    let mut on_disk: Vec<String> = mc_golden::all_fixture_dirs()
        .expect("list tests/golden")
        .into_iter()
        .filter_map(|dir| {
            let name = dir.file_name()?.to_str()?.to_string();
            let area = dir.parent()?.file_name()?.to_str()?.to_string();
            (area == "worldgen").then_some(name)
        })
        .collect();
    on_disk.sort();
    let mut covered: Vec<String> = COVERED.iter().map(|n| n.to_string()).collect();
    covered.sort();
    assert_eq!(
        on_disk, covered,
        "tests/golden/worldgen fixtures and the fixtures! list in tests/golden_worldgen.rs differ"
    );
}

/// PLAN_v4 §4.4 item 2 (Rust side): with the world seed's lowest bit flipped
/// the generated chunks must differ from the fixture.
#[test]
fn overworld_with_perturbed_seed_fails() {
    let name = "overworld-seed-123";
    let mut scenario = Scenario::load(fixture_dir("worldgen", name)).expect("load scenario");
    match &mut scenario.kind {
        ScenarioKind::Terrain { seed, .. } => *seed ^= 1,
        other => panic!("{name} is not a terrain scenario: {other:?}"),
    }
    let exp: Vec<TraceLine> = expected_lines(name)
        .into_iter()
        .filter(|l| skip_reason(l).is_none())
        .collect();
    let act: Vec<TraceLine> = actual_lines(&scenario)
        .lines()
        .iter()
        .filter(|l| skip_reason(l).is_none())
        .cloned()
        .collect();
    let mismatch = compare(&exp, &act).expect_err("a one-bit seed change must change the terrain");
    assert_eq!(
        mismatch.line, 1,
        "the first generated chunk already differs"
    );
}
