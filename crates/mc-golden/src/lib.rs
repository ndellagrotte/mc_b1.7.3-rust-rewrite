//! Golden-fixture harness (PLAN_v4 §4, ADR-007).
//!
//! Reads `tests/golden/<area>/<name>/scenario.json`, drives `mc-sim` through
//! the scenario, and compares the result with `expected.jsonl` produced by the
//! Java oracle (`tools/oracle/`). Used from tests only. Empty until WP-0.2.
