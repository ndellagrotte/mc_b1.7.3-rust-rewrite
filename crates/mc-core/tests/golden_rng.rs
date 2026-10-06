//! `tests/golden/rng/*`: `java.util.Random` sequences (README `java_random`).

use mc_core::JavaRandom;
use mc_golden::{GoldenError, RngValue, Scenario, ScenarioKind, TraceLine, TraceWriter, run};

/// Replays the scenario's draws on one `JavaRandom`, one trace line per draw.
fn drive(scenario: &Scenario, out: &mut TraceWriter) -> Result<(), GoldenError> {
    let ScenarioKind::JavaRandom { seed, draws } = &scenario.kind else {
        return Err(GoldenError::other("not a java_random scenario"));
    };
    let mut rng = JavaRandom::new(*seed);
    let mut i = 0u64;
    for draw in draws {
        let call = draw.call.as_str();
        if draw.bound.is_some() && call != "nextInt" {
            return Err(GoldenError::other(format!("\"bound\" on {call}")));
        }
        for _ in 0..draw.n {
            let v = match (call, draw.bound) {
                ("nextInt", None) => RngValue::Int(rng.next_int().into()),
                ("nextInt", Some(bound)) => RngValue::Int(rng.next_int_bounded(bound).into()),
                ("nextLong", _) => RngValue::Int(rng.next_long()),
                ("nextFloat", _) => RngValue::F32(rng.next_float()),
                ("nextDouble", _) => RngValue::F64(rng.next_double()),
                ("nextGaussian", _) => RngValue::F64(rng.next_gaussian()),
                ("nextBoolean", _) => RngValue::Bool(rng.next_boolean()),
                _ => return Err(GoldenError::other(format!("unknown call {call}"))),
            };
            out.push(TraceLine::rng(i, call, draw.bound, v));
            i += 1;
        }
    }
    Ok(())
}

fn check(name: &str) {
    let dir = mc_golden::fixture_dir("rng", name);
    if let Err(e) = run(&dir, drive) {
        panic!("{name}: {e}");
    }
}

#[test]
fn java_random_0() {
    check("java-random-0");
}

#[test]
fn java_random_123() {
    check("java-random-123");
}

#[test]
fn java_random_neg_4172144991579068934() {
    check("java-random--4172144991579068934");
}

/// Every committed rng fixture is covered by a test above.
#[test]
fn all_rng_fixtures_listed() {
    let names: Vec<String> = mc_golden::all_fixture_dirs()
        .unwrap()
        .into_iter()
        .filter(|d| d.parent().and_then(|p| p.file_name()) == Some("rng".as_ref()))
        .map(|d| d.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        names,
        [
            "java-random--4172144991579068934",
            "java-random-0",
            "java-random-123"
        ]
    );
}
