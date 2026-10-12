//! `MathHelper` and the `java.lang.Math` shims (PLAN_v4 §3.4 rule 5).
//!
//! The free functions transcribe `MathHelper.java` with Java names in
//! snake_case. [`jmath`] is the single entry point the rest of the port uses
//! for the non-exact `java.lang.Math` functions.

use std::sync::OnceLock;

/// Number of entries in `MathHelper.SIN_TABLE`.
// src: MathHelper.java:4
pub const SIN_TABLE_LEN: usize = 65536;

/// The `java.lang.Math` replacements for the port (§3.4 rule 5, decided in
/// WP-0.3): `sin`, `cos`, `atan2`, `atan`, `pow`, `log` are `StrictMath`,
/// i.e. the fdlibm 5.3 transcriptions in [`crate::strict_math`], and `sqrt`
/// is IEEE-754 `f64::sqrt` (correctly rounded, identical everywhere).
///
/// **Every** port of a `Math.sin/cos/atan2/atan/pow/sqrt/log` call goes
/// through this module, never through `f64::sin` & co. (the platform's libm,
/// which differs from fdlibm 5.3 in the last bit).
///
/// Why `StrictMath`: the WP-0.3 sweep (`tests/math_sweep_report.rs`) showed
/// HotSpot's x86-64 `sin`/`cos`/`pow` intrinsics match no available Rust
/// implementation, so the oracle runs with those intrinsics disabled
/// (`tools/oracle/env.sh`), which makes its `java.lang.Math` bit-identical to
/// `StrictMath` on every JDK and CPU. These functions reproduce
/// `tests/golden/math/math-sweep` (and `math-sweep-strict`) bit for bit,
/// NaN bit patterns included, on every target. Accepted cost: the real client, with intrinsics on, differs by 1 ulp on a few
/// percent of `sin`/`cos`/`pow` inputs.
///
/// Vanilla also calls `Math.abs`, `Math.round`, `Math.ceil`, `Math.max`, which
/// are exact and need no shim.
pub mod jmath {
    use crate::strict_math;

    /// `Math.sin(a)` = `StrictMath.sin(a)` (fdlibm 5.3).
    #[inline]
    pub fn sin(a: f64) -> f64 {
        strict_math::sin(a)
    }

    /// `Math.cos(a)` = `StrictMath.cos(a)` (fdlibm 5.3).
    #[inline]
    pub fn cos(a: f64) -> f64 {
        strict_math::cos(a)
    }

    /// `Math.atan2(y, x)` = `StrictMath.atan2(y, x)` (fdlibm 5.3), Java
    /// argument order (first argument is `y`).
    #[inline]
    pub fn atan2(y: f64, x: f64) -> f64 {
        strict_math::atan2(y, x)
    }

    /// `Math.atan(a)` = `StrictMath.atan(a)` (fdlibm 5.3).
    #[inline]
    pub fn atan(a: f64) -> f64 {
        strict_math::atan(a)
    }

    /// `Math.pow(a, b)` = `StrictMath.pow(a, b)` (fdlibm 5.3), with Java's
    /// special cases (a NaN exponent gives NaN, `(+-1)**(+-inf)` is NaN),
    /// which fdlibm 5.3 shares and C99 `pow` does not.
    #[inline]
    pub fn pow(a: f64, b: f64) -> f64 {
        strict_math::pow(a, b)
    }

    /// `Math.log(a)` = `StrictMath.log(a)` (fdlibm 5.3).
    #[inline]
    pub fn log(a: f64) -> f64 {
        strict_math::log(a)
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
/// Computed, not loaded, with [`jmath::sin`] like vanilla's `Math.sin`, so
/// the table is the same on every target; it reproduces all 65536 entries of
/// the oracle's table (`tests/golden/math/sine-table`). (`std`'s `f64::sin`
/// and `libm::sin` happen to as well, since the `(float)` cast absorbs their
/// last-bit differences.)
// src: MathHelper.java:56-61 (static initialiser)
pub fn sin_table() -> &'static [f32; SIN_TABLE_LEN] {
    static TABLE: OnceLock<Box<[f32; SIN_TABLE_LEN]>> = OnceLock::new();
    TABLE.get_or_init(|| compute_sin_table(jmath::sin))
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
