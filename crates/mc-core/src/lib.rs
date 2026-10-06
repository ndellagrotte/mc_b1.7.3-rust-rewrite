//! Deterministic primitives shared by the simulation crates (PLAN_v4 §3.1,
//! §3.4).
//!
//! - [`java_random`] — [`JavaRandom`], a bit-exact `java.util.Random`
//!   (adapted from MinecraftOSS per D4). All gameplay randomness goes through
//!   it (§3.4 rule 3).
//! - [`math`] — `MathHelper` (the 65536-entry sine table and its lookups,
//!   floors, `bucketInt`, …) and [`math::jmath`], the one place the port calls
//!   `java.lang.Math.sin/cos/atan2/atan/pow/sqrt` (§3.4 rule 5).
//! - [`strict_math`] — `java.lang.StrictMath` functions transcribed from
//!   fdlibm 5.3 where bit-exactness is required (`log`, for
//!   [`JavaRandom::next_gaussian`]).
//! - [`aabb`] — [`Aabb`], `AxisAlignedBB` as a value type.
//! - [`vec3d`] — [`Vec3D`], vanilla's double vector, as a value type (needed by
//!   [`Aabb::is_vec_inside`] and [`Aabb::calculate_intercept`]).
//!
//! Verified against the Tier A golden fixtures in `tests/golden/rng` and
//! `tests/golden/math` (see `tests/` in this crate).
//!
//! No dependencies. No `std::fs`, threads, clocks, environment access or the
//! `rand` crate (D9); must build for `wasm32-unknown-unknown`. No `unsafe`.

#![forbid(unsafe_code)]

pub mod aabb;
pub mod java_random;
pub mod math;
pub mod strict_math;
pub mod vec3d;

pub mod java_hash;

pub use aabb::{Aabb, AabbIntercept};
pub use java_hash::{JavaHash, JavaHashMap, JavaHashSet};
pub use java_random::JavaRandom;
pub use math::jmath;
pub use vec3d::Vec3D;
