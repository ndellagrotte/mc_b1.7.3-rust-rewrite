//! The clocks the client passes into the simulation and its own timing (D9:
//! the simulation never reads them itself).
//!
//! - [`current_time_millis`]: `System.currentTimeMillis()`, the system clock.
//! - [`nano_time`]: `System.nanoTime()`, monotonic, from an arbitrary origin
//!   (here: the first call in the process).
//! - [`new_random_seed`]: the seed `new Random()` picks
//!   (`seedUniquifier() ^ System.nanoTime()`, java.util.Random), for every
//!   clock-seeded `Random` the simulation exposes a seed for (the world's
//!   `rand`, the explosion and dispenser `Random`s, the display `Random`, the
//!   entity `rand`s, the teleporter's; [`crate::game`] "Seeding").

#![allow(clippy::disallowed_types)]

use std::sync::OnceLock;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// `System.currentTimeMillis()`.
pub fn current_time_millis() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_millis() as i64,
        Err(e) => -(e.duration().as_millis() as i64),
    }
}

/// `System.nanoTime()`.
pub fn nano_time() -> i64 {
    static ORIGIN: OnceLock<Instant> = OnceLock::new();
    let origin = *ORIGIN.get_or_init(Instant::now);
    origin.elapsed().as_nanos() as i64
}

/// `java.util.Random.seedUniquifier`'s state (`8682522807148012L`, advanced
/// by `* 1181783497276652981L` on every `new Random()`).
static SEED_UNIQUIFIER: AtomicI64 = AtomicI64::new(8682522807148012);

/// The seed of `new Random()`: `seedUniquifier() ^ System.nanoTime()`
/// (the constructor then scrambles it as `setSeed` does, which
/// [`mc_core::java_random::JavaRandom::new`] does too).
pub fn new_random_seed() -> i64 {
    let mut cur = SEED_UNIQUIFIER.load(Ordering::Relaxed);
    loop {
        let next = cur.wrapping_mul(1181783497276652981);
        match SEED_UNIQUIFIER.compare_exchange_weak(cur, next, Ordering::Relaxed, Ordering::Relaxed)
        {
            Ok(_) => return next ^ nano_time(),
            Err(v) => cur = v,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clocks_move_forward_and_seeds_differ() {
        let a = nano_time();
        let b = nano_time();
        assert!(b >= a);
        assert!(current_time_millis() > 1_600_000_000_000);
        assert_ne!(new_random_seed(), new_random_seed());
    }
}
