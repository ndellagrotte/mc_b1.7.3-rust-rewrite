//! WP-4.2 long-run soak (PLAN_v4 §5 Phase 4 row 4.2): the `tests/golden/soak/*`
//! fixtures, tens of thousands of ticks each, run through
//! `mc_golden::tick_driver::drive_in` and compared with the Java oracle's
//! `expected.jsonl` (format and contents: tests/golden/README.md "Soak").
//!
//! Every soak test is `#[ignore]`d so `cargo test` stays fast; run them with
//!
//! ```text
//! cargo test -p mc-golden --test golden_soak -- --ignored --nocapture
//! ```
//!
//! (`--nocapture` shows each fixture's ticks per second). The Rust trace of
//! every run is in `target/golden/soak/<name>/actual.jsonl`. A fixture whose
//! first difference is a port bug stays registered, with that line as its
//! ignore reason in place of the generic one:
//!
//! ```text
//! soak_seed_123 => "seed-123", "port bug: first differing line <t, kind, field>";
//! ```
//!
//! [`every_soak_fixture_has_a_test`] is not ignored: a new fixture directory
//! without a test function fails the ordinary test run.

use mc_golden::tick_driver::drive_in;
use mc_golden::{Scenario, ScenarioKind, fixture_dir};

/// Runs `tests/golden/soak/<name>` and prints its ticks per second.
// Host-side test timing only; clippy.toml's D9 clock ban exists for the
// simulation crates, which this does not reach.
#[allow(clippy::disallowed_types)]
fn check(name: &str) {
    let dir = fixture_dir("soak", name);
    let ticks = match Scenario::load(&dir).map(|s| s.kind) {
        Ok(ScenarioKind::Tick(t)) => t.ticks,
        Ok(other) => panic!("soak/{name}: not a tick scenario: {other:?}"),
        Err(e) => panic!("soak/{name}: {e}"),
    };
    let start = std::time::Instant::now();
    if let Err(e) = mc_golden::run(&dir, drive_in(&dir)) {
        panic!("soak/{name}: {e}");
    }
    let secs = start.elapsed().as_secs_f64();
    eprintln!(
        "soak/{name}: {ticks} ticks in {secs:.1} s ({:.0} ticks/s)",
        f64::from(ticks) / secs
    );
}

/// One ignored soak test: the generic reason, or `$why` (a port bug's first
/// differing line) when given.
macro_rules! soak_test {
    ($(#[$attr:meta])* $test:ident, $name:literal) => {
        soak_test!(
            $(#[$attr])* $test,
            $name,
            "soak: minutes; run with cargo test -p mc-golden --test golden_soak -- --ignored"
        );
    };
    ($(#[$attr:meta])* $test:ident, $name:literal, $why:literal) => {
        #[test]
        #[ignore = $why]
        $(#[$attr])*
        fn $test() {
            check($name);
        }
    };
}

/// `test => "fixture"[, "ignore reason"];` per soak fixture.
macro_rules! soak {
    ($( $(#[$attr:meta])* $test:ident => $name:literal $(, $why:literal)?; )*) => {
        $( soak_test!($(#[$attr])* $test, $name $(, $why)?); )*
        /// Every `tests/golden/soak/<name>` that has a test function above.
        const COVERED: &[&str] = &[$($name),*];
    };
}

soak! {
    soak_nether_8675309 => "nether-8675309";
    soak_seed_123 => "seed-123";
    soak_seed_8675309 => "seed-8675309";
    soak_seed_neg_7612233452384654969 => "seed--7612233452384654969";
    soak_testworld => "testworld";
}

/// Every `tests/golden/soak/*` fixture has a test above (and every name above
/// a fixture directory), so a new soak fixture cannot be forgotten.
#[test]
fn every_soak_fixture_has_a_test() {
    let mut on_disk: Vec<String> = mc_golden::all_fixture_dirs()
        .expect("list tests/golden")
        .into_iter()
        .filter_map(|dir| {
            let name = dir.file_name()?.to_str()?.to_string();
            let area = dir.parent()?.file_name()?.to_str()?.to_string();
            (area == "soak").then_some(name)
        })
        .collect();
    on_disk.sort();
    let mut covered: Vec<String> = COVERED.iter().map(|n| n.to_string()).collect();
    covered.sort();
    assert!(
        !on_disk.is_empty(),
        "no soak fixtures under tests/golden/soak"
    );
    assert_eq!(
        on_disk, covered,
        "tests/golden/soak fixtures and the soak! list in tests/golden_soak.rs differ"
    );
}
