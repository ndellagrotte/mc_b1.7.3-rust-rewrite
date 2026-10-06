//! Deterministic primitives shared by the simulation crates.
//!
//! Planned contents (PLAN_v4 §3.1, §3.4): `java_random` (a transcription of
//! `java.util.Random`, adapted from MinecraftOSS per D4), `math` (the
//! `Math.sin`/`cos`/`atan2`/`pow` shims and the vanilla sine table), `AABB`,
//! and `JavaHashMap`/`JavaHashSet`, which reproduce Java bucket iteration
//! order for the few places vanilla iterates a hash collection in gameplay.
//!
//! No dependencies. No `std::fs`, threads, clocks, environment access or the
//! `rand` crate (D9); must build for `wasm32-unknown-unknown`.
