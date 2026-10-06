//! `tests/golden/math/sine-table`: `MathHelper.SIN_TABLE` (README
//! `sine_table`), digest through `mc_golden::run` plus an element-wise check
//! against the committed sidecar.

use mc_core::math::{SIN_TABLE_LEN, compute_sin_table, sin_table};
use mc_golden::{GoldenError, ScenarioKind, TraceLine, read_sidecar, run};

const SIDECAR: &str = "sine_table.f32le.bin";

fn fixture() -> std::path::PathBuf {
    mc_golden::fixture_dir("math", "sine-table")
}

fn le_bytes(table: &[f32; SIN_TABLE_LEN]) -> Vec<u8> {
    table.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// The oracle's table, from the sidecar.
fn oracle_table() -> Vec<u32> {
    let bytes = read_sidecar(fixture(), SIDECAR).unwrap();
    assert_eq!(bytes.len(), 4 * SIN_TABLE_LEN, "sidecar length");
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b))
        .collect()
}

/// Number of differing entries and the first one, as (index, oracle, port).
fn diff(table: &[f32; SIN_TABLE_LEN]) -> (usize, Option<(usize, u32, u32)>) {
    let oracle = oracle_table();
    let mut first = None;
    let mut count = 0;
    for (i, (&want, got)) in oracle.iter().zip(table.iter()).enumerate() {
        if want != got.to_bits() {
            count += 1;
            first.get_or_insert((i, want, got.to_bits()));
        }
    }
    (count, first)
}

#[test]
fn golden_sine_table() {
    run(fixture(), |scenario, out| {
        if !matches!(scenario.kind, ScenarioKind::SineTable) {
            return Err(GoldenError::other("not a sine_table scenario"));
        }
        let bytes = le_bytes(sin_table());
        out.push(TraceLine::sine_table(SIN_TABLE_LEN as u32, &bytes, SIDECAR));
        Ok(())
    })
    .unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn sine_table_matches_sidecar_elementwise() {
    if let (n, Some((i, want, got))) = diff(sin_table()) {
        panic!(
            "{n} of {SIN_TABLE_LEN} entries differ; first at index {i}: \
             oracle 0x{want:08x} ({}), port 0x{got:08x} ({})",
            f32::from_bits(want),
            f32::from_bits(got)
        );
    }
}

/// Which `Math.sin` candidates reproduce the table (report only).
#[test]
fn sine_table_candidates_report() {
    for (name, f) in [
        ("std f64::sin", f64::sin as fn(f64) -> f64),
        ("libm::sin", libm::sin),
    ] {
        match diff(&compute_sin_table(f)) {
            (0, _) => println!("sine table: {name}: all {SIN_TABLE_LEN} entries match"),
            (n, Some((i, want, got))) => println!(
                "sine table: {name}: {n} of {SIN_TABLE_LEN} differ; first at {i}: \
                 oracle 0x{want:08x}, candidate 0x{got:08x}"
            ),
            (_, None) => unreachable!(),
        }
    }
}
