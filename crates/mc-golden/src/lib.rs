//! Golden-fixture harness (PLAN_v4 §4, ADR-007).
//!
//! Reads `tests/golden/<area>/<name>/scenario.json`, lets the calling test
//! drive the Rust port through the scenario, and compares the produced trace
//! with `expected.jsonl` written by the Java oracle (`tools/oracle/`). The
//! fixture format is defined in `tests/golden/README.md`; this crate implements
//! that contract. Used from tests only: it is a host-side dev-dependency, and
//! its only file access lives in one private module (PLAN_v4 D9 exemption).
//!
//! API overview:
//!
//! - [`run`] — load a fixture, call a drive closure that fills a
//!   [`TraceWriter`], write `target/golden/<area>/<name>/actual.jsonl`, compare.
//! - [`Scenario`] / [`ScenarioKind`] — the parsed `scenario.json`.
//! - [`TraceLine`] and its event constructors ([`TraceLine::rng`],
//!   [`TraceLine::sine_table`], [`TraceLine::math`],
//!   [`TraceLine::chunk_digest`], [`TraceLine::chunk_full`],
//!   [`TraceLine::biome`], [`TraceLine::nbt_bytes`], [`TraceLine::nbt_tree`],
//!   [`TraceLine::nbt_roundtrip`], [`TraceLine::nbt_gzip`],
//!   [`TraceLine::nbt_file`], [`TraceLine::hash_codes`],
//!   [`TraceLine::hash_order`]) — emit exactly the documented line shapes.
//! - [`compare()`] / [`Mismatch`] — first-difference report with floats
//!   decoded (PLAN_v4 §4.4 item 3).
//! - [`Bits32`] / [`Bits64`] / [`hex_bytes`] / [`blake3_hex`] — the encoding
//!   rules (ADR-007 §5.2).
//! - [`fixture_dir`], [`workspace_root`], [`read_sidecar`], [`all_fixture_dirs`].
//!
//! ```no_run
//! use mc_golden::{run, ScenarioKind, TraceLine};
//!
//! run("tests/golden/math/sine-table", |scenario, out| {
//!     assert!(matches!(scenario.kind, ScenarioKind::SineTable));
//!     let bytes = vec![0u8; 4 * 65536]; // the port's table as f32 LE
//!     out.push(TraceLine::sine_table(65536, &bytes, "sine_table.f32le.bin"));
//!     Ok(())
//! })
//! .unwrap();
//! ```

pub mod bits;
pub mod compare;
pub mod digest;
pub mod error;
// Host-only file access for a test-support crate. PLAN_v4 D9 bans `std::fs`
// in the simulation crates; clippy.toml applies the ban workspace-wide, so the
// single module that touches files opts out here, visibly (see `io.rs`).
#[allow(clippy::disallowed_types, clippy::disallowed_methods)]
mod io;
pub mod runner;
pub mod scenario;
pub mod trace;

pub use bits::{Bits32, Bits64, hex_bytes, parse_hex_bytes};
pub use compare::{Mismatch, MismatchKind, compare};
pub use digest::blake3_hex;
pub use error::GoldenError;
pub use runner::{all_fixture_dirs, fixture_dir, read_sidecar, run, target_dir, workspace_root};
pub use scenario::{ChunkSel, Dimension, Draw, HashOp, NbtNode, Scenario, ScenarioKind, Tier};
pub use trace::{
    ChunkArrays, RngValue, Stage, TraceLine, TraceParseError, TraceWriter, parse_jsonl,
};
