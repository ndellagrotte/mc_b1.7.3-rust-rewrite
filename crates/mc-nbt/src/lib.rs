//! Named Binary Tag reader and writer for Beta 1.7.3.
//!
//! Supports exactly the tag ids vanilla b1.7.3 defines (0 through 10); the
//! later `IntArray`/`LongArray` tags are rejected on read and never written.
//! Compounds are `BTreeMap`s. Gzip/zlib framing comes from `flate2`. To be
//! adapted from MinecraftOSS `core/src/nbt.rs` with attribution (PLAN_v4 D4).
//!
//! Browser-ready: no `std::fs`, threads, clocks or environment access (D9).
