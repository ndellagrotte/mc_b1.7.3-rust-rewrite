//! `java.lang.StrictMath` functions the port needs bit-exactly.
//!
//! `StrictMath` is specified as fdlibm 5.3 (the JDK ships a line-by-line Java
//! port, `java.lang.FdLibm`). The `libm` crate is *not* a drop-in: its `log`
//! follows FreeBSD's later restructuring of `e_log.c` and differs from fdlibm
//! 5.3 in the last bit for some inputs (found by the `tests/golden/rng`
//! fixtures through `Random.nextGaussian`). So the functions here are
//! transcribed from fdlibm 5.3 itself.
//!
//! fdlibm notice, preserved as its licence requires:
//!
//! > Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
//! >
//! > Developed at SunSoft, a Sun Microsystems, Inc. business.
//! > Permission to use, copy, modify, and distribute this
//! > software is freely granted, provided that this notice
//! > is preserved.

/// High 32 bits of `x` as a signed int (fdlibm `__HI`).
fn hi(x: f64) -> i32 {
    (x.to_bits() >> 32) as i32
}

/// Low 32 bits of `x` (fdlibm `__LO`).
fn lo(x: f64) -> u32 {
    x.to_bits() as u32
}

/// `x` with its high word replaced (fdlibm `__HI(x) = hx`).
fn with_hi(x: f64, hx: i32) -> f64 {
    f64::from_bits((u64::from(hx as u32) << 32) | u64::from(lo(x)))
}

/// `StrictMath.log(x)`: fdlibm 5.3 `__ieee754_log` (`e_log.c`).
///
/// Used by [`crate::JavaRandom::next_gaussian`].
pub fn log(x: f64) -> f64 {
    const LN2_HI: f64 = f64::from_bits(0x3fe6_2e42_fee0_0000); // 6.93147180369123816490e-01
    const LN2_LO: f64 = f64::from_bits(0x3dea_39ef_3579_3c76); // 1.90821492927058770002e-10
    const TWO54: f64 = f64::from_bits(0x4350_0000_0000_0000); // 1.80143985094819840000e+16
    const LG1: f64 = f64::from_bits(0x3fe5_5555_5555_5593); // 6.666666666666735130e-01
    const LG2: f64 = f64::from_bits(0x3fd9_9999_9997_fa04); // 3.999999999940941908e-01
    const LG3: f64 = f64::from_bits(0x3fd2_4924_9422_9359); // 2.857142874366239149e-01
    const LG4: f64 = f64::from_bits(0x3fcc_71c5_1d8e_78af); // 2.222219843214978396e-01
    const LG5: f64 = f64::from_bits(0x3fc7_4664_96cb_03de); // 1.818357216161805012e-01
    const LG6: f64 = f64::from_bits(0x3fc3_9a09_d078_c69f); // 1.531383769920937332e-01
    const LG7: f64 = f64::from_bits(0x3fc2_f112_df3e_5244); // 1.479819860511658591e-01
    const ZERO: f64 = 0.0;

    let mut x = x;
    let mut hx = hi(x);
    let lx = lo(x);

    let mut k: i32 = 0;
    if hx < 0x0010_0000 {
        // x < 2**-1022
        if ((hx & 0x7fff_ffff) as u32 | lx) == 0 {
            return -TWO54 / ZERO; // log(+-0) = -inf
        }
        if hx < 0 {
            // fdlibm's NaN: kept as an expression for the hardware NaN's sign.
            #[allow(clippy::eq_op)]
            return (x - x) / ZERO; // log(-#) = NaN
        }
        k -= 54;
        x *= TWO54; // subnormal number, scale up x
        hx = hi(x);
    }
    if hx >= 0x7ff0_0000 {
        return x + x;
    }
    k += (hx >> 20) - 1023;
    hx &= 0x000f_ffff;
    let i = (hx + 0x95f64) & 0x10_0000;
    x = with_hi(x, hx | (i ^ 0x3ff0_0000)); // normalize x or x/2
    k += i >> 20;
    let f = x - 1.0;
    if (0x000f_ffff & (2 + hx)) < 3 {
        // -2**-20 <= f < 2**-20
        if f == ZERO {
            if k == 0 {
                return ZERO;
            }
            let dk = f64::from(k);
            return dk * LN2_HI + dk * LN2_LO;
        }
        let r = f * f * (0.5 - 0.33333333333333333 * f);
        if k == 0 {
            return f - r;
        }
        let dk = f64::from(k);
        return dk * LN2_HI - ((r - dk * LN2_LO) - f);
    }
    let s = f / (2.0 + f);
    let dk = f64::from(k);
    let z = s * s;
    let mut i = hx - 0x6147a;
    let w = z * z;
    let j = 0x6b851 - hx;
    let t1 = w * (LG2 + w * (LG4 + w * LG6));
    let t2 = z * (LG1 + w * (LG3 + w * (LG5 + w * LG7)));
    i |= j;
    let r = t2 + t1;
    if i > 0 {
        let hfsq = 0.5 * f * f;
        if k == 0 {
            f - (hfsq - s * (hfsq + r))
        } else {
            dk * LN2_HI - ((hfsq - (s * (hfsq + r) + dk * LN2_LO)) - f)
        }
    } else if k == 0 {
        f - s * (f - r)
    } else {
        dk * LN2_HI - ((s * (f - r) - dk * LN2_LO) - f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specials() {
        assert_eq!(log(1.0).to_bits(), 0);
        assert_eq!(log(0.0), f64::NEG_INFINITY);
        assert_eq!(log(-0.0), f64::NEG_INFINITY);
        assert!(log(-1.0).is_nan());
        assert!(log(f64::NAN).is_nan());
        assert_eq!(log(f64::INFINITY), f64::INFINITY);
        assert_eq!(log(core::f64::consts::E), 1.0);
        assert_eq!(log(2.0), core::f64::consts::LN_2);
        assert!((log(f64::from_bits(1)) + 744.4400719213812).abs() < 1e-12);
    }

    #[test]
    fn differs_from_libm_crate_where_the_jdk_does() {
        // `s` from java.util.Random(0)'s 27th nextGaussian pair (rng fixture
        // java-random-0, i = 372): the JDK's StrictMath.log gives ...439,
        // the `libm` crate ...438.
        let s = f64::from_bits(0x3fe9_931e_a60d_1389);
        assert_eq!(log(s).to_bits(), 0xbfcc_b062_a3e4_3439);
    }
}
