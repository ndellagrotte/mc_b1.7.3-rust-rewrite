//! PLAN_v4 §3.4 rule 5 sweep: which implementation of `Math.sin/cos/atan2/
//! atan/pow/sqrt` reproduces the oracle JVM bit for bit?
//!
//! Replays `tests/golden/math/math-sweep/scenario.json` (edge cases per
//! function, then `count` inputs from one `JavaRandom(seed)` drawing
//! `nextDouble() * 2000 - 1000`, `x` then `y`, README `math_sweep`) and
//! compares three candidates with the oracle's `java.lang.Math` results:
//!
//! - `std`: Rust `f64::sin`, `cos`, `atan2`, `atan`, `powf`, `sqrt` (on Linux,
//!   glibc);
//! - `libm`: the `libm` crate (pure Rust, from FreeBSD msun / fdlibm);
//! - `strict`: the oracle's `StrictMath` results from
//!   `tests/golden/math/math-sweep-strict/expected.jsonl`, when present (what
//!   an exact fdlibm transcription would give).
//!
//! `math_sweep_report` only reports (run with `--nocapture` to see the table)
//! and fails only if `std`'s `sqrt` (correctly rounded by IEEE-754) or the
//! input generation disagrees with the oracle. `golden_math_sweep_strict_eq`
//! is the strict fixture check of `mc_core::jmath`, ignored until the
//! implementation is decided.

use std::collections::BTreeMap;

use mc_core::{JavaRandom, jmath};
use mc_golden::{GoldenError, Scenario, ScenarioKind, TraceLine, parse_jsonl, read_sidecar, run};

const SWEEP: &str = "math-sweep";
const STRICT: &str = "math-sweep-strict";

/// One sweep input in trace order.
#[derive(Clone, Copy, Debug)]
struct Input {
    function: &'static str,
    i: u64,
    x: f64,
    y: Option<f64>,
}

fn is_binary(function: &str) -> bool {
    matches!(function, "atan2" | "pow")
}

fn intern(function: &str) -> &'static str {
    match function {
        "sin" => "sin",
        "cos" => "cos",
        "atan2" => "atan2",
        "atan" => "atan",
        "pow" => "pow",
        "sqrt" => "sqrt",
        other => panic!("unknown math function {other}"),
    }
}

/// The scenario's inputs, generated exactly as the oracle does.
fn inputs(scenario: &Scenario) -> Result<Vec<Input>, GoldenError> {
    let ScenarioKind::MathSweep {
        seed,
        count,
        functions,
        ..
    } = &scenario.kind
    else {
        return Err(GoldenError::other("not a math_sweep scenario"));
    };
    let mut rng = JavaRandom::new(*seed);
    let mut out = Vec::new();
    for function in functions {
        let function = intern(function);
        let two = is_binary(function);
        let edges: Vec<f64> = scenario
            .kind
            .edge_cases_for(function)
            .map_err(GoldenError::other)?
            .into_iter()
            .map(|b| b.to_f64())
            .collect();
        let mut i = 0u64;
        if two {
            if !edges.len().is_multiple_of(2) {
                return Err(GoldenError::other(format!(
                    "{function}: odd edge-case list"
                )));
            }
            for pair in edges.chunks(2) {
                out.push(Input {
                    function,
                    i,
                    x: pair[0],
                    y: Some(pair[1]),
                });
                i += 1;
            }
        } else {
            for &x in &edges {
                out.push(Input {
                    function,
                    i,
                    x,
                    y: None,
                });
                i += 1;
            }
        }
        for _ in 0..*count {
            let x = rng.next_double() * 2000.0 - 1000.0;
            let y = two.then(|| rng.next_double() * 2000.0 - 1000.0);
            out.push(Input { function, i, x, y });
            i += 1;
        }
    }
    Ok(out)
}

type Impl = fn(&str, f64, f64) -> f64;

/// `mc_core::jmath`, the port's shim (currently std plus Java special cases).
fn eval_jmath(function: &str, x: f64, y: f64) -> f64 {
    match function {
        "sin" => jmath::sin(x),
        "cos" => jmath::cos(x),
        "atan2" => jmath::atan2(x, y),
        "atan" => jmath::atan(x),
        "pow" => jmath::pow(x, y),
        "sqrt" => jmath::sqrt(x),
        _ => unreachable!(),
    }
}

fn eval_std(function: &str, x: f64, y: f64) -> f64 {
    match function {
        "sin" => x.sin(),
        "cos" => x.cos(),
        "atan2" => x.atan2(y),
        "atan" => x.atan(),
        "pow" => x.powf(y),
        "sqrt" => x.sqrt(),
        _ => unreachable!(),
    }
}

fn eval_libm(function: &str, x: f64, y: f64) -> f64 {
    match function {
        "sin" => libm::sin(x),
        "cos" => libm::cos(x),
        "atan2" => libm::atan2(x, y),
        "atan" => libm::atan(x),
        "pow" => libm::pow(x, y),
        "sqrt" => libm::sqrt(x),
        _ => unreachable!(),
    }
}

fn bits(v: f64) -> String {
    format!("0x{:016x}", v.to_bits())
}

fn bits_field(line: &TraceLine, key: &str) -> Option<u64> {
    let s = line.get(key)?.as_str()?;
    mc_golden::Bits64::parse(s).ok().map(|b| b.0)
}

/// `(fn, i) -> (x, y, v)` bit patterns of a `math` trace.
type Results = BTreeMap<(String, u64), (u64, Option<u64>, u64)>;

fn load_results(name: &str) -> Option<Results> {
    let dir = mc_golden::fixture_dir("math", name);
    let text = String::from_utf8(read_sidecar(&dir, "expected.jsonl").ok()?).ok()?;
    let lines = parse_jsonl(&text).unwrap_or_else(|e| panic!("{name}: {e:?}"));
    let mut out = Results::new();
    for line in lines.iter().filter(|l| l.e() == "math") {
        let function = line
            .get("fn")
            .and_then(|v| v.as_str())
            .expect("fn")
            .to_owned();
        let i = line.get("i").and_then(|v| v.as_u64()).expect("i");
        let x = bits_field(line, "x").expect("x");
        let y = bits_field(line, "y");
        let v = bits_field(line, "v").expect("v");
        out.insert((function, i), (x, y, v));
    }
    Some(out)
}

#[derive(Default)]
struct Tally {
    total: usize,
    mismatches: usize,
    first: Option<String>,
    /// Indices of mismatching edge-case inputs.
    edge: Vec<u64>,
}

impl Tally {
    fn record(&mut self, input: &Input, edge: bool, expected: u64, actual: u64) {
        self.total += 1;
        if expected != actual {
            self.mismatches += 1;
            if edge {
                self.edge.push(input.i);
            }
            self.first.get_or_insert_with(|| {
                let y = input
                    .y
                    .map(|y| format!(" y={}", bits(y)))
                    .unwrap_or_default();
                format!(
                    "i={} x={}{y} expected=0x{expected:016x} ({:?}) actual=0x{actual:016x} ({:?})",
                    input.i,
                    bits(input.x),
                    f64::from_bits(expected),
                    f64::from_bits(actual)
                )
            });
        }
    }

    fn cell(&self) -> String {
        format!("{}/{}", self.mismatches, self.total)
    }
}

#[test]
fn math_sweep_report() {
    let dir = mc_golden::fixture_dir("math", SWEEP);
    let scenario = Scenario::load(&dir).unwrap();
    let inputs = inputs(&scenario).unwrap();
    let expected = load_results(SWEEP).expect("math-sweep/expected.jsonl");
    assert_eq!(
        inputs.len(),
        expected.len(),
        "input count vs expected.jsonl"
    );

    // The inputs must be the oracle's, or nothing below means anything.
    for input in &inputs {
        let (x, y, _) = expected[&(input.function.to_owned(), input.i)];
        assert_eq!(
            (x, y),
            (input.x.to_bits(), input.y.map(f64::to_bits)),
            "{} i={}: generated input differs from expected.jsonl",
            input.function,
            input.i
        );
    }

    let strict = load_results(STRICT);
    let ScenarioKind::MathSweep { count, .. } = scenario.kind else {
        unreachable!()
    };
    let candidates: [(&str, Impl); 3] = [
        ("std", eval_std),
        ("libm", eval_libm),
        ("jmath", eval_jmath),
    ];
    let functions: Vec<&'static str> = {
        let mut seen = Vec::new();
        for input in &inputs {
            if !seen.contains(&input.function) {
                seen.push(input.function);
            }
        }
        seen
    };

    // Number of edge cases per function: the random inputs are the last `count`.
    let mut per_fn: BTreeMap<&str, u64> = BTreeMap::new();
    for input in &inputs {
        *per_fn.entry(input.function).or_default() += 1;
    }

    // tallies[function][column]; columns: std, libm, jmath, strict, libm~strict.
    let mut tallies: BTreeMap<&str, [Tally; 5]> = BTreeMap::new();
    for input in &inputs {
        let key = (input.function.to_owned(), input.i);
        let want = expected[&key].2;
        let edge = input.i < per_fn[input.function] - u64::from(count);
        let row = tallies.entry(input.function).or_default();
        let y = input.y.unwrap_or(0.0);
        for (k, (_, f)) in candidates.iter().enumerate() {
            row[k].record(input, edge, want, f(input.function, input.x, y).to_bits());
        }
        if let Some(strict) = &strict
            && let Some(&(sx, sy, sv)) = strict.get(&key)
            && (sx, sy) == (input.x.to_bits(), input.y.map(f64::to_bits))
        {
            row[3].record(input, edge, want, sv);
            let libm = eval_libm(input.function, input.x, y).to_bits();
            row[4].record(input, edge, sv, libm);
        }
    }

    let strict_note = match &strict {
        Some(_) => "strict = oracle StrictMath (math-sweep-strict) vs Math",
        None => "strict: tests/golden/math/math-sweep-strict absent, skipped",
    };
    println!("\n§3.4 rule 5 sweep: mismatching inputs / total, against oracle java.lang.Math");
    println!("({strict_note}; libm~strict = libm crate vs StrictMath)\n");
    println!("(jmath = mc_core::jmath: std plus Java's pow special cases)\n");
    let names = ["std", "libm", "jmath", "strict", "libm~strict"];
    print!("| {:<6} |", "fn");
    for name in names {
        print!(" {name:>11} |");
    }
    println!();
    println!("|--------|{}", "-------------|".repeat(names.len()));
    for function in &functions {
        print!("| {function:<6} |");
        for t in &tallies[function] {
            let cell = if t.total == 0 {
                "n/a".to_owned()
            } else {
                t.cell()
            };
            print!(" {cell:>11} |");
        }
        println!();
    }
    println!("\nFirst mismatch per function and candidate:");
    for function in &functions {
        for (name, tally) in names.iter().zip(&tallies[function]) {
            if let Some(first) = &tally.first {
                println!("  {function:<5} {name:<11} {first}");
            }
        }
    }
    println!("\nMismatching edge cases (indices into the scenario's edge_cases):");
    for function in &functions {
        for (name, tally) in names.iter().zip(&tallies[function]) {
            if !tally.edge.is_empty() {
                println!("  {function:<5} {name:<11} {:?}", tally.edge);
            }
        }
    }

    let sqrt = &tallies["sqrt"][0];
    assert_eq!(
        sqrt.mismatches, 0,
        "std sqrt must be exact: {:?}",
        sqrt.first
    );
}

/// The strict fixture check for `mc_core::jmath`. Enable once §3.4 rule 5's
/// implementation is chosen.
#[test]
#[ignore = "enable once the §3.4 rule 5 implementation is decided (WP-0.3 sweep)"]
fn golden_math_sweep_strict_eq() {
    run(mc_golden::fixture_dir("math", SWEEP), |scenario, out| {
        for input in inputs(scenario)? {
            let v = eval_jmath(input.function, input.x, input.y.unwrap_or(0.0));
            out.push(TraceLine::math(
                input.function,
                input.i,
                input.x,
                input.y,
                v,
            ));
        }
        Ok(())
    })
    .unwrap_or_else(|e| panic!("{e}"));
}
