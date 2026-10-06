//! The vanilla Beta 1.7.3 singleplayer simulation as plain Rust (PLAN_v4 D1).
//!
//! Everything here is transcribed from
//! `.reference/mc_b1.7.3_release/1.7.3-LTS/src/minecraft` (D2) with
//! `// src: File.java:line` citations. Bevy never appears in this crate; the
//! client drives it from one system and reads its state back.
//!
//! The modules below are the ownership units for agents (§3.1); `worldgen` is
//! the plan's `gen`, renamed because `gen` is a reserved keyword in Rust 2024.
//! All are empty until WP-0.5 and later work packages fill them in.
//!
//! Browser-ready: no `std::fs`, threads, clocks, environment access or `rand`
//! (D9). Storage goes through a `Storage` trait; time is passed in.

pub mod block;
pub mod entity;
pub mod inventory;
pub mod item;
pub mod save;
pub mod world;
pub mod worldgen;
