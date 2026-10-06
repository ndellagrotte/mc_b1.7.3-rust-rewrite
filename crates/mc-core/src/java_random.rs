// Adapted from MinecraftOSS third_party/minecraftoss/core/src/random.rs @ 4013a68 by chasmlol, Apache-2.0; modified
//! `java.util.Random`, bit for bit (PLAN_v4 D4, §3.4 rule 3).
//!
//! Adapted from MinecraftOSS's `LegacyRandom`; the Xoroshiro, worldgen and
//! positional parts are dropped. Changes from the source:
//!
//! - `next_double` multiplies by `DOUBLE_UNIT = 0x1.0p-53` as the JDK does;
//!   MinecraftOSS used modern Minecraft's `1.110223E-16F`, which is not 2^-53.
//! - `next_gaussian` uses `StrictMath.log`/`StrictMath.sqrt` like the JDK:
//!   [`crate::strict_math::log`] (fdlibm 5.3) instead of `f64::ln`.
//! - `next_int_bounded` panics on a non-positive bound (the JDK throws
//!   `IllegalArgumentException`); `next_bytes` is added.
//!
//! The JDK algorithm references below are to `java.util.Random` (stable since
//! JDK 1.2; b1.7.3 ran on Java 5/6, the oracle on JDK 25, same outputs). The
//! class is not part of the vanilla tree, so they are not `// src:` citations.
//! Verified against the `tests/golden/rng/*` fixtures.

use crate::strict_math;

/// `0x5DEECE66DL`, the LCG multiplier.
const MULTIPLIER: i64 = 0x5_DEEC_E66D;
/// `0xBL`, the LCG addend.
const ADDEND: i64 = 0xB;
/// `(1L << 48) - 1`.
const MASK: i64 = (1 << 48) - 1;
/// `0x1.0p-53` (`Random.DOUBLE_UNIT`).
const DOUBLE_UNIT: f64 = 1.0 / (1u64 << 53) as f64;
/// `1 << 24` as a float (`nextFloat` divisor).
const FLOAT_UNIT: f32 = (1u32 << 24) as f32;

/// A `java.util.Random`.
///
/// All arithmetic is wrapping `i64`/`i32` as in Java (§3.4 rule 1).
#[derive(Clone, Debug, PartialEq)]
pub struct JavaRandom {
    /// The 48-bit LCG state (`Random.seed`).
    seed: i64,
    /// `nextNextGaussian`, valid when `haveNextNextGaussian`.
    next_next_gaussian: Option<f64>,
}

impl JavaRandom {
    /// `new Random(seed)`.
    pub fn new(seed: i64) -> Self {
        Self {
            seed: Self::initial_scramble(seed),
            next_next_gaussian: None,
        }
    }

    /// `Random.initialScramble`: `(seed ^ multiplier) & mask`.
    fn initial_scramble(seed: i64) -> i64 {
        (seed ^ MULTIPLIER) & MASK
    }

    /// `setSeed(seed)`; also clears the cached Gaussian, as the JDK does.
    pub fn set_seed(&mut self, seed: i64) {
        self.seed = Self::initial_scramble(seed);
        self.next_next_gaussian = None;
    }

    /// The scrambled 48-bit state (`Random.seed`), for inspection and tests.
    pub fn state(&self) -> i64 {
        self.seed
    }

    /// `next(bits)`: advance the LCG and return its top `bits` bits.
    ///
    /// `bits` must be in `1..=32`. The result is `(int)(seed >>> (48 - bits))`,
    /// so `next(32)` may be negative and smaller widths are non-negative.
    pub fn next(&mut self, bits: u32) -> i32 {
        debug_assert!((1..=32).contains(&bits), "next({bits})");
        self.seed = self.seed.wrapping_mul(MULTIPLIER).wrapping_add(ADDEND) & MASK;
        // `>>>` on a non-negative 48-bit value; `as i32` is Java's `(int)`.
        ((self.seed as u64) >> (48 - bits)) as i32
    }

    /// `nextInt()`.
    pub fn next_int(&mut self) -> i32 {
        self.next(32)
    }

    /// `nextInt(bound)`.
    ///
    /// Power-of-two bounds take the high bits (`(bound * (long)next(31)) >> 31`);
    /// other bounds use the JDK rejection loop. Panics when `bound <= 0`
    /// (Java: `IllegalArgumentException`).
    pub fn next_int_bounded(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "bound must be positive, got {bound}");
        // JDK: `(bound & m) == 0` with `m = bound - 1`, i.e. a power of two.
        if bound & (bound - 1) == 0 {
            return ((i64::from(bound) * i64::from(self.next(31))) >> 31) as i32;
        }
        loop {
            let bits = self.next(31);
            let val = bits % bound;
            // `bits - val + (bound - 1) < 0` detects int overflow (rejection).
            if bits.wrapping_sub(val).wrapping_add(bound - 1) >= 0 {
                return val;
            }
        }
    }

    /// `nextLong()`: `((long)next(32) << 32) + next(32)`.
    pub fn next_long(&mut self) -> i64 {
        let hi = i64::from(self.next(32));
        let lo = i64::from(self.next(32));
        (hi << 32).wrapping_add(lo)
    }

    /// `nextBoolean()`: `next(1) != 0`.
    pub fn next_boolean(&mut self) -> bool {
        self.next(1) != 0
    }

    /// `nextFloat()`: `next(24) / (float)(1 << 24)`.
    pub fn next_float(&mut self) -> f32 {
        self.next(24) as f32 / FLOAT_UNIT
    }

    /// `nextDouble()`: `(((long)next(26) << 27) + next(27)) * 0x1.0p-53`.
    pub fn next_double(&mut self) -> f64 {
        let hi = i64::from(self.next(26));
        let lo = i64::from(self.next(27));
        ((hi << 27) + lo) as f64 * DOUBLE_UNIT
    }

    /// `nextGaussian()`: Marsaglia polar method, second value cached.
    ///
    /// The JDK computes the multiplier with `StrictMath.sqrt(-2 *
    /// StrictMath.log(s) / s)`. `log` is [`crate::strict_math::log`], the
    /// fdlibm 5.3 transcription: neither `f64::ln` (glibc) nor `libm::log`
    /// reproduces the `tests/golden/rng` fixtures (both are off by one ulp for
    /// some `s`). `sqrt` is correctly rounded everywhere, so `f64::sqrt`.
    pub fn next_gaussian(&mut self) -> f64 {
        if let Some(saved) = self.next_next_gaussian.take() {
            return saved;
        }
        loop {
            let v1 = 2.0 * self.next_double() - 1.0;
            let v2 = 2.0 * self.next_double() - 1.0;
            let s = v1 * v1 + v2 * v2;
            if s >= 1.0 || s == 0.0 {
                continue;
            }
            let multiplier = (-2.0 * strict_math::log(s) / s).sqrt();
            self.next_next_gaussian = Some(v2 * multiplier);
            return v1 * multiplier;
        }
    }

    /// `nextBytes(bytes)`: one `nextInt()` per 4 bytes, low byte first; a
    /// trailing partial group still consumes a whole `nextInt()`.
    pub fn next_bytes(&mut self, bytes: &mut [u8]) {
        for chunk in bytes.chunks_mut(4) {
            let mut rnd = self.next_int();
            for b in chunk {
                *b = rnd as u8;
                rnd >>= 8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vectors() {
        // java.util.Random(0).nextInt() twice (also in the rng fixture).
        let mut r = JavaRandom::new(0);
        assert_eq!(r.next_int(), -1_155_484_576);
        assert_eq!(r.next_int(), -723_955_400);
    }

    #[test]
    fn set_seed_resets_gaussian_cache() {
        let mut a = JavaRandom::new(7);
        let _ = a.next_gaussian();
        a.set_seed(7);
        let b = JavaRandom::new(7);
        assert_eq!(a, b);
    }

    #[test]
    fn next_bytes_consumes_whole_ints() {
        let mut a = JavaRandom::new(42);
        let mut bytes = [0u8; 6];
        a.next_bytes(&mut bytes);
        let mut b = JavaRandom::new(42);
        let first = b.next_int().to_le_bytes();
        let second = b.next_int().to_le_bytes();
        assert_eq!(bytes[..4], first);
        assert_eq!(bytes[4..], second[..2]);
        assert_eq!(a, b);
    }

    #[test]
    fn double_unit_is_two_to_minus_53() {
        assert_eq!(DOUBLE_UNIT.to_bits(), 0x3ca0_0000_0000_0000);
    }

    #[test]
    #[should_panic(expected = "bound must be positive")]
    fn zero_bound_panics() {
        JavaRandom::new(0).next_int_bounded(0);
    }
}
