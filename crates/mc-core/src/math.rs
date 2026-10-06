//! `MathHelper` and the `java.lang.Math` shims (PLAN_v4 §3.4 rule 5).
//!
//! The free functions transcribe `MathHelper.java` with Java names in
//! snake_case. [`jmath`] is the single entry point the rest of the port uses
//! for the non-exact `java.lang.Math` functions.

use std::sync::OnceLock;

/// Number of entries in `MathHelper.SIN_TABLE`.
// src: MathHelper.java:4
pub const SIN_TABLE_LEN: usize = 65536;

/// The `java.lang.Math` functions vanilla calls whose results are not
/// specified bit-exactly by Java (§3.4 rule 5): `sin`, `cos`, `atan2`, `atan`,
/// `pow`, plus `sqrt` (exact everywhere, kept here so every call site looks
/// the same).
///
/// **Every** port of a `Math.sin/cos/atan2/atan/pow/sqrt` call goes through
/// this module, never through `f64::sin` & co. directly, so the
/// implementation can be swapped in one place.
///
/// Status: the implementation is **not decided yet**; WP-0.3's sweep
/// (`crates/mc-core/tests/math_sweep_report.rs`, against
/// `tests/golden/math/math-sweep`) decides it. These currently delegate to
/// Rust `std` (plus Java's `pow` special cases), which on native targets
/// calls the platform libm (glibc on Linux) and on `wasm32-unknown-unknown`
/// the `libm` crate, so results may differ between the native and browser
/// builds. First sweep result (1040 inputs per function, oracle JDK 25 on
/// x86_64): `std` misses `sin` 4, `cos` 2, `atan2` 170, `atan` 7; the `libm`
/// crate (fdlibm) matches `atan2`/`atan`/`sqrt` exactly but misses `sin` 38,
/// `cos` 24, `pow` 10. HotSpot evaluates `atan2`/`atan` with fdlibm and
/// `sin`/`cos`/`pow` with its own intrinsics.
///
/// Vanilla also calls `Math.abs`, `Math.round`, `Math.ceil`, `Math.max`, which
/// are exact and need no shim.
pub mod jmath {
    /// `Math.sin(a)`.
    #[inline]
    pub fn sin(a: f64) -> f64 {
        a.sin()
    }

    /// `Math.cos(a)`.
    #[inline]
    pub fn cos(a: f64) -> f64 {
        a.cos()
    }

    /// `Math.atan2(y, x)`, Java argument order (first argument is `y`).
    #[inline]
    pub fn atan2(y: f64, x: f64) -> f64 {
        y.atan2(x)
    }

    /// `Math.atan(a)`.
    #[inline]
    pub fn atan(a: f64) -> f64 {
        a.atan()
    }

    /// `Math.pow(a, b)`.
    ///
    /// Java's special cases differ from C99 `pow` (which `powf` follows) in
    /// two places, applied here before delegating: a NaN exponent always
    /// gives NaN (C: `pow(1, NaN) == 1`), and `|a| == 1` with an infinite
    /// exponent gives NaN (C: 1). Both are in the `java.lang.Math.pow`
    /// specification and the oracle sweep's edge cases.
    #[inline]
    pub fn pow(a: f64, b: f64) -> f64 {
        if b.is_nan() || (b.is_infinite() && (a == 1.0 || a == -1.0)) {
            return f64::NAN;
        }
        a.powf(b)
    }

    /// `Math.sqrt(a)`: correctly rounded by IEEE-754, identical everywhere.
    #[inline]
    pub fn sqrt(a: f64) -> f64 {
        a.sqrt()
    }
}

/// Builds `MathHelper.SIN_TABLE` the way its static initialiser does, with
/// `sin` standing in for `Math.sin`.
///
/// Exposed so the golden tests can compare candidate `sin` implementations;
/// the port itself uses [`sin_table`].
pub fn compute_sin_table(sin: fn(f64) -> f64) -> Box<[f32; SIN_TABLE_LEN]> {
    let mut table = Box::new([0.0f32; SIN_TABLE_LEN]);
    for (i, slot) in table.iter_mut().enumerate() {
        // src: MathHelper.java:58 — `(double)i * 3.141592653589793D * 2.0D / 65536.0D`,
        // evaluated left to right.
        let angle = i as f64 * 3.141592653589793 * 2.0 / 65536.0;
        *slot = sin(angle) as f32;
    }
    table
}

/// `MathHelper.SIN_TABLE`, built once on first use.
///
/// Computed, not loaded: `std`'s `f64::sin` (glibc natively, the `libm`
/// crate on wasm) and `libm::sin` both reproduce all 65536 entries of the
/// oracle's table (`tests/golden/math/sine-table`), since the `(float)` cast
/// absorbs their last-bit differences. Unverified on other native libms
/// (macOS, Windows); the golden test catches a difference there.
// src: MathHelper.java:56-61 (static initialiser)
pub fn sin_table() -> &'static [f32; SIN_TABLE_LEN] {
    static TABLE: OnceLock<Box<[f32; SIN_TABLE_LEN]>> = OnceLock::new();
    TABLE.get_or_init(|| compute_sin_table(f64::sin))
}

/// `MathHelper.sin(f)`: table lookup, 65536 steps per turn.
// src: MathHelper.java:6-8
#[inline]
pub fn sin(f: f32) -> f32 {
    // src: MathHelper.java:7 — `(int)(f * 10430.378F) & 0xffff`; `as i32`
    // saturates like Java's `(int)` (NaN -> 0).
    sin_table()[((f * 10430.378_f32) as i32 & 0xffff) as usize]
}

/// `MathHelper.cos(f)`: the table a quarter turn ahead.
// src: MathHelper.java:10-12
#[inline]
pub fn cos(f: f32) -> f32 {
    // src: MathHelper.java:11 — the `+ 16384.0F` is a float add before the cast.
    sin_table()[((f * 10430.378_f32 + 16384.0_f32) as i32 & 0xffff) as usize]
}

/// `MathHelper.sqrt_float(f)`: `(float)Math.sqrt((double)f)`.
// src: MathHelper.java:14-16
#[inline]
pub fn sqrt_float(f: f32) -> f32 {
    jmath::sqrt(f64::from(f)) as f32
}

/// `MathHelper.sqrt_double(d)`: `(float)Math.sqrt(d)`; returns a `float`.
// src: MathHelper.java:18-20
#[inline]
pub fn sqrt_double(d: f64) -> f32 {
    jmath::sqrt(d) as f32
}

/// `MathHelper.floor_float(f)`.
// src: MathHelper.java:22-25
#[inline]
pub fn floor_float(f: f32) -> i32 {
    let i = f as i32;
    // Java `i - 1` wraps (reached only when `f` saturated to `i32::MIN`).
    if f < i as f32 { i.wrapping_sub(1) } else { i }
}

/// `MathHelper.floor_double(d)`.
// src: MathHelper.java:27-30
#[inline]
pub fn floor_double(d: f64) -> i32 {
    let i = d as i32;
    if d < f64::from(i) {
        i.wrapping_sub(1)
    } else {
        i
    }
}

/// `MathHelper.abs(f)`: `f >= 0.0F ? f : -f` (so `abs(-0.0) == -0.0`, as in
/// vanilla; not `f32::abs`).
// src: MathHelper.java:32-34
#[inline]
pub fn abs(f: f32) -> f32 {
    if f >= 0.0 { f } else { -f }
}

/// `MathHelper.abs_max(a, b)`: the larger magnitude.
// src: MathHelper.java:36-46
#[inline]
pub fn abs_max(mut a: f64, mut b: f64) -> f64 {
    if a < 0.0 {
        a = -a;
    }
    if b < 0.0 {
        b = -b;
    }
    if a > b { a } else { b }
}

/// `MathHelper.bucketInt(i, factor)`: floor division for a positive `factor`.
/// Panics when `factor == 0` (Java: `ArithmeticException`).
// src: MathHelper.java:48-50
#[inline]
pub fn bucket_int(i: i32, factor: i32) -> i32 {
    if i < 0 {
        (i.wrapping_neg().wrapping_sub(1))
            .wrapping_div(factor)
            .wrapping_neg()
            .wrapping_sub(1)
    } else {
        i.wrapping_div(factor)
    }
}

/// `MathHelper.stringNullOrLengthZero(s)`; Java `null` is `None`.
// src: MathHelper.java:52-54
#[inline]
pub fn string_null_or_length_zero(s: Option<&str>) -> bool {
    s.is_none_or(str::is_empty)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floors() {
        assert_eq!(floor_double(-0.5), -1);
        assert_eq!(floor_double(-1.0), -1);
        assert_eq!(floor_double(2.999), 2);
        assert_eq!(floor_double(f64::NAN), 0);
        assert_eq!(floor_double(1e300), i32::MAX);
        // Saturated to MIN, then `MIN - 1` wraps, as in Java.
        assert_eq!(floor_double(-1e300), i32::MAX);
        assert_eq!(floor_float(-0.25), -1);
        assert_eq!(floor_float(3.0), 3);
    }

    #[test]
    fn bucket() {
        assert_eq!(bucket_int(-1, 16), -1);
        assert_eq!(bucket_int(-16, 16), -1);
        assert_eq!(bucket_int(-17, 16), -2);
        assert_eq!(bucket_int(15, 16), 0);
        assert_eq!(bucket_int(i32::MIN, 16), -134_217_728);
    }

    #[test]
    fn abs_quirks() {
        assert_eq!(abs(-0.0).to_bits(), (-0.0f32).to_bits());
        assert_eq!(abs(-2.5), 2.5);
        assert_eq!(abs_max(-3.0, 2.0), 3.0);
        assert_eq!(abs_max(1.0, -4.0), 4.0);
    }

    #[test]
    fn table_lookups() {
        assert_eq!(sin(0.0), 0.0);
        assert_eq!(cos(0.0), 1.0);
        // 1.0 * 10430.378 truncates to index 10430; cos is 16384 entries ahead.
        assert_eq!(sin(1.0), sin_table()[10430]);
        assert_eq!(cos(1.0), sin_table()[10430 + 16384]);
        assert_eq!(sin(-1.0), sin_table()[(-10430i32 & 0xffff) as usize]);
        assert_eq!(sin(f32::NAN), 0.0);
        assert!(string_null_or_length_zero(None));
        assert!(string_null_or_length_zero(Some("")));
        assert!(!string_null_or_length_zero(Some("a")));
    }
}
