//! `java.lang.StrictMath`, transcribed from fdlibm 5.3 (PLAN_v4 §3.4 rule 5).
//!
//! `StrictMath` is specified as fdlibm 5.3 (the JDK ships a line-by-line Java
//! port, `java.lang.FdLibm`), and the oracle runs with HotSpot's math
//! intrinsics disabled, so `java.lang.Math` gives the same bits there. The
//! functions here are the netlib fdlibm 5.3 sources (`e_log.c`, `s_sin.c`,
//! `s_cos.c`, `k_sin.c`, `k_cos.c`, `e_rem_pio2.c`, `k_rem_pio2.c`,
//! `s_atan.c`, `e_atan2.c`, `e_pow.c` 1.5 04/04/22, `s_scalbn.c`) transcribed
//! statement by statement: same names, same order of operations, `__HI`/`__LO`
//! as [`f64::to_bits`]/[`f64::from_bits`], wrapping integer arithmetic where
//! C relies on two's complement. [`crate::jmath`] delegates here.
//!
//! The `libm` crate is *not* a drop-in: it follows FreeBSD msun, which
//! restructured several of these after fdlibm 5.3 (`log`, `sin`/`cos`
//! argument reduction, `pow`), and differs in the last bit for some inputs
//! (`tests/math_sweep_report.rs`).
//!
//! **NaN bit patterns.** Where fdlibm produces a NaN through an invalid
//! operation (`x - x` for infinite `x`, `(x - x) / (x - x)`, …) or propagates
//! one (`x + y`), the bits depend on the hardware: x86-64 SSE gives the
//! negative "default NaN" `0xfff8_0000_0000_0000` for an invalid operation and
//! returns the first NaN operand, quieted, otherwise; ARM gives
//! `0x7ff8_0000_0000_0000`, wasm leaves the sign unspecified and LLVM's
//! constant folder picks the positive one. The oracle is x86-64, so those
//! expressions are spelled out with the private `x86_nan` and `x86_add`, which give
//! the x86-64 bits on every target.
//!
//! fdlibm notice, preserved as its licence requires:
//!
//! > Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
//! >
//! > Developed at SunSoft, a Sun Microsystems, Inc. business.
//! > Permission to use, copy, modify, and distribute this
//! > software is freely granted, provided that this notice
//! > is preserved.
//!
//! `e_pow.c`:
//!
//! > Copyright (C) 2004 by Sun Microsystems, Inc. All rights reserved.
//! >
//! > Permission to use, copy, modify, and distribute this
//! > software is freely granted, provided that this notice
//! > is preserved.

// Kept 1:1 with the C so the transcription can be diffed against it.
#![allow(
    clippy::collapsible_if,
    clippy::collapsible_else_if,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::explicit_counter_loop,
    clippy::needless_late_init
)]

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

/// `x` with its low word replaced (fdlibm `__LO(x) = lx`).
fn with_lo(x: f64, lx: u32) -> f64 {
    f64::from_bits((x.to_bits() & 0xffff_ffff_0000_0000) | u64::from(lx))
}

/// The NaN x86-64 SSE returns for an invalid operation (`inf - inf`,
/// `0 / 0`, `sqrt(-1)`): sign set, quiet bit set, zero payload.
const X86_DEFAULT_NAN: f64 = f64::from_bits(0xfff8_0000_0000_0000);

/// `a` with its quiet bit set, as x86-64 returns a NaN operand.
fn quiet(a: f64) -> f64 {
    f64::from_bits(a.to_bits() | 0x0008_0000_0000_0000)
}

/// The value x86-64 gives an fdlibm expression in `a` alone that is NaN for
/// this `a` (`a - a` for infinite or NaN `a`, `(a - a) / 0`,
/// `(a - a) / (a - a)`): `a` itself, quieted, if `a` is NaN, else the default
/// NaN.
fn x86_nan(a: f64) -> f64 {
    if a.is_nan() {
        quiet(a)
    } else {
        X86_DEFAULT_NAN
    }
}

/// `a + b` with x86-64 NaN propagation (the first NaN operand, quieted).
fn x86_add(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        quiet(a)
    } else if b.is_nan() {
        quiet(b)
    } else {
        a + b
    }
}

/// fdlibm `scalbn(x, n)` (`s_scalbn.c`): `x * 2^n` by exponent manipulation.
fn scalbn(x: f64, n: i32) -> f64 {
    const TWO54: f64 = f64::from_bits(0x4350_0000_0000_0000); // 1.80143985094819840000e+16
    const TWOM54: f64 = f64::from_bits(0x3c90_0000_0000_0000); // 5.55111512312578270212e-17
    const HUGE: f64 = 1.0e+300;
    const TINY: f64 = 1.0e-300;

    let mut x = x;
    let mut hx = hi(x);
    let lx = lo(x);
    let mut k = (hx & 0x7ff0_0000) >> 20; // extract exponent
    if k == 0 {
        // 0 or subnormal x
        if (lx | (hx & 0x7fff_ffff) as u32) == 0 {
            return x; // +-0
        }
        x *= TWO54;
        hx = hi(x);
        k = ((hx & 0x7ff0_0000) >> 20) - 54;
        if n < -50000 {
            return TINY * x; // underflow
        }
    }
    if k == 0x7ff {
        return x86_add(x, x); // NaN or Inf
    }
    k = k.wrapping_add(n);
    if k > 0x7fe {
        return HUGE * HUGE.copysign(x); // overflow
    }
    if k > 0 {
        // normal result
        return with_hi(x, (hx & 0x800f_ffff_u32 as i32) | (k << 20));
    }
    if k <= -54 {
        if n > 50000 {
            // in case integer overflow in n+k
            return HUGE * HUGE.copysign(x); // overflow
        } else {
            return TINY * TINY.copysign(x); // underflow
        }
    }
    k += 54; // subnormal result
    x = with_hi(x, (hx & 0x800f_ffff_u32 as i32) | (k << 20));
    x * TWOM54
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
            return x86_nan(x); // C: (x-x)/zero, log(-#) = NaN
        }
        k -= 54;
        x *= TWO54; // subnormal number, scale up x
        hx = hi(x);
    }
    if hx >= 0x7ff0_0000 {
        return x86_add(x, x);
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

/// fdlibm `__kernel_sin(x, y, iy)` (`k_sin.c`): sin on [-pi/4, pi/4]; `y`
/// is the tail of `x`, `iy == 0` means `y` is zero.
fn kernel_sin(x: f64, y: f64, iy: i32) -> f64 {
    const HALF: f64 = f64::from_bits(0x3fe0_0000_0000_0000); // 5.00000000000000000000e-01
    const S1: f64 = f64::from_bits(0xbfc5_5555_5555_5549); // -1.66666666666666324348e-01
    const S2: f64 = f64::from_bits(0x3f81_1111_1110_f8a6); // 8.33333333332248946124e-03
    const S3: f64 = f64::from_bits(0xbf2a_01a0_19c1_61d5); // -1.98412698298579493134e-04
    const S4: f64 = f64::from_bits(0x3ec7_1de3_57b1_fe7d); // 2.75573137070700676789e-06
    const S5: f64 = f64::from_bits(0xbe5a_e5e6_8a2b_9ceb); // -2.50507602534068634195e-08
    const S6: f64 = f64::from_bits(0x3de5_d93a_5acf_d57c); // 1.58969099521155010221e-10

    let ix = hi(x) & 0x7fff_ffff; // high word of x
    if ix < 0x3e40_0000 {
        // |x| < 2**-27
        if (x as i32) == 0 {
            return x; // generate inexact
        }
    }
    let z = x * x;
    let v = z * x;
    let r = S2 + z * (S3 + z * (S4 + z * (S5 + z * S6)));
    if iy == 0 {
        x + v * (S1 + z * r)
    } else {
        x - ((z * (HALF * y - v * r) - y) - v * S1)
    }
}

/// fdlibm `__kernel_cos(x, y)` (`k_cos.c`): cos on [-pi/4, pi/4]; `y` is the
/// tail of `x`.
fn kernel_cos(x: f64, y: f64) -> f64 {
    const ONE: f64 = f64::from_bits(0x3ff0_0000_0000_0000); // 1.00000000000000000000e+00
    const C1: f64 = f64::from_bits(0x3fa5_5555_5555_554c); // 4.16666666666666019037e-02
    const C2: f64 = f64::from_bits(0xbf56_c16c_16c1_5177); // -1.38888888888741095749e-03
    const C3: f64 = f64::from_bits(0x3efa_01a0_19cb_1590); // 2.48015872894767294178e-05
    const C4: f64 = f64::from_bits(0xbe92_7e4f_809c_52ad); // -2.75573143513906633035e-07
    const C5: f64 = f64::from_bits(0x3e21_ee9e_bdb4_b1c4); // 2.08757232129817482790e-09
    const C6: f64 = f64::from_bits(0xbda8_fae9_be88_38d4); // -1.13596475577881948265e-11

    let ix = hi(x) & 0x7fff_ffff; // ix = |x|'s high word
    if ix < 0x3e40_0000 {
        // if x < 2**27
        if (x as i32) == 0 {
            return ONE; // generate inexact
        }
    }
    let z = x * x;
    let r = z * (C1 + z * (C2 + z * (C3 + z * (C4 + z * (C5 + z * C6)))));
    if ix < 0x3fd3_3333 {
        // if |x| < 0.3
        return ONE - (0.5 * z - (z * r - x * y));
    } else {
        let qx = if ix > 0x3fe9_0000 {
            // x > 0.78125
            0.28125
        } else {
            // __HI(qx) = ix-0x00200000; __LO(qx) = 0;  (x/4)
            with_hi(0.0, ix - 0x0020_0000)
        };
        let hz = 0.5 * z - qx;
        let a = ONE - qx;
        return a - (hz - (z * r - x * y));
    }
}

/// Table of constants for 2/pi, 396 hex digits (476 decimal) of 2/pi
/// (`e_rem_pio2.c` `two_over_pi`, 24 bits per entry).
const TWO_OVER_PI: [i32; 66] = [
    0xA2F983, 0x6E4E44, 0x1529FC, 0x2757D1, 0xF534DD, 0xC0DB62, //
    0x95993C, 0x439041, 0xFE5163, 0xABDEBB, 0xC561B7, 0x246E3A, //
    0x424DD2, 0xE00649, 0x2EEA09, 0xD1921C, 0xFE1DEB, 0x1CB129, //
    0xA73EE8, 0x8235F5, 0x2EBB44, 0x84E99C, 0x7026B4, 0x5F7E41, //
    0x3991D6, 0x398353, 0x39F49C, 0x845F8B, 0xBDF928, 0x3B1FF8, //
    0x97FFDE, 0x05980F, 0xEF2F11, 0x8B5A0A, 0x6D1F6D, 0x367ECF, //
    0x27CB09, 0xB74F46, 0x3F669E, 0x5FEA2D, 0x7527BA, 0xC7EBE5, //
    0xF17B3D, 0x0739F7, 0x8A5292, 0xEA6BFB, 0x5FB11F, 0x8D5D08, //
    0x560330, 0x46FC7B, 0x6BABF0, 0xCFBC20, 0x9AF436, 0x1DA9E3, //
    0x91615E, 0xE61B08, 0x659985, 0x5F14A0, 0x68408D, 0xFFD880, //
    0x4D7327, 0x310606, 0x1556CA, 0x73A8C9, 0x60E27B, 0xC08C6B, //
];

/// High words of `n * pi/2` for n = 1..=32 (`e_rem_pio2.c` `npio2_hw`).
const NPIO2_HW: [i32; 32] = [
    0x3FF921FB, 0x400921FB, 0x4012D97C, 0x401921FB, 0x401F6A7A, 0x4022D97C, //
    0x4025FDBB, 0x402921FB, 0x402C463A, 0x402F6A7A, 0x4031475C, 0x4032D97C, //
    0x40346B9C, 0x4035FDBB, 0x40378FDB, 0x403921FB, 0x403AB41B, 0x403C463A, //
    0x403DD85A, 0x403F6A7A, 0x40407E4C, 0x4041475C, 0x4042106C, 0x4042D97C, //
    0x4043A28C, 0x40446B9C, 0x404534AC, 0x4045FDBB, 0x4046C6CB, 0x40478FDB, //
    0x404858EB, 0x404921FB, //
];

/// fdlibm `__ieee754_rem_pio2(x, y)` (`e_rem_pio2.c`): `x - n*pi/2` as
/// `y[0] + y[1]`, returning `n`.
fn rem_pio2(x: f64, y: &mut [f64; 2]) -> i32 {
    const ZERO: f64 = f64::from_bits(0x0000_0000_0000_0000); // 0.00000000000000000000e+00
    const HALF: f64 = f64::from_bits(0x3fe0_0000_0000_0000); // 5.00000000000000000000e-01
    const TWO24: f64 = f64::from_bits(0x4170_0000_0000_0000); // 1.67772160000000000000e+07
    const INVPIO2: f64 = f64::from_bits(0x3fe4_5f30_6dc9_c883); // 6.36619772367581382433e-01, 53 bits of 2/pi
    const PIO2_1: f64 = f64::from_bits(0x3ff9_21fb_5440_0000); // 1.57079632673412561417e+00, first 33 bits of pi/2
    const PIO2_1T: f64 = f64::from_bits(0x3dd0_b461_1a62_6331); // 6.07710050650619224932e-11, pi/2 - pio2_1
    const PIO2_2: f64 = f64::from_bits(0x3dd0_b461_1a60_0000); // 6.07710050630396597660e-11, second 33 bits of pi/2
    const PIO2_2T: f64 = f64::from_bits(0x3ba3_198a_2e03_7073); // 2.02226624879595063154e-21, pi/2 - (pio2_1+pio2_2)
    const PIO2_3: f64 = f64::from_bits(0x3ba3_198a_2e00_0000); // 2.02226624871116645580e-21, third 33 bits of pi/2
    const PIO2_3T: f64 = f64::from_bits(0x397b_839a_2520_49c1); // 8.47842766036889956997e-32, pi/2 - (pio2_1+pio2_2+pio2_3)

    let hx = hi(x); // high word of x
    let ix = hx & 0x7fff_ffff;
    if ix <= 0x3fe9_21fb {
        // |x| ~<= pi/4 , no need for reduction
        y[0] = x;
        y[1] = 0.0;
        return 0;
    }
    if ix < 0x4002_d97c {
        // |x| < 3pi/4, special case with n=+-1
        if hx > 0 {
            let mut z = x - PIO2_1;
            if ix != 0x3ff9_21fb {
                // 33+53 bit pi is good enough
                y[0] = z - PIO2_1T;
                y[1] = (z - y[0]) - PIO2_1T;
            } else {
                // near pi/2, use 33+33+53 bit pi
                z -= PIO2_2;
                y[0] = z - PIO2_2T;
                y[1] = (z - y[0]) - PIO2_2T;
            }
            return 1;
        } else {
            // negative x
            let mut z = x + PIO2_1;
            if ix != 0x3ff9_21fb {
                // 33+53 bit pi is good enough
                y[0] = z + PIO2_1T;
                y[1] = (z - y[0]) + PIO2_1T;
            } else {
                // near pi/2, use 33+33+53 bit pi
                z += PIO2_2;
                y[0] = z + PIO2_2T;
                y[1] = (z - y[0]) + PIO2_2T;
            }
            return -1;
        }
    }
    if ix <= 0x4139_21fb {
        // |x| ~<= 2^19*(pi/2), medium size
        let mut t = x.abs();
        let n = (t * INVPIO2 + HALF) as i32;
        let fn_ = f64::from(n);
        let mut r = t - fn_ * PIO2_1;
        let mut w = fn_ * PIO2_1T; // 1st round good to 85 bit
        if n < 32 && ix != NPIO2_HW[(n - 1) as usize] {
            y[0] = r - w; // quick check no cancellation
        } else {
            let j = ix >> 20;
            y[0] = r - w;
            let mut i = j - ((hi(y[0]) >> 20) & 0x7ff);
            if i > 16 {
                // 2nd iteration needed, good to 118
                t = r;
                w = fn_ * PIO2_2;
                r = t - w;
                w = fn_ * PIO2_2T - ((t - r) - w);
                y[0] = r - w;
                i = j - ((hi(y[0]) >> 20) & 0x7ff);
                if i > 49 {
                    // 3rd iteration need, 151 bits acc
                    t = r; // will cover all possible cases
                    w = fn_ * PIO2_3;
                    r = t - w;
                    w = fn_ * PIO2_3T - ((t - r) - w);
                    y[0] = r - w;
                }
            }
        }
        y[1] = (r - y[0]) - w;
        if hx < 0 {
            y[0] = -y[0];
            y[1] = -y[1];
            return -n;
        } else {
            return n;
        }
    }
    // all other (large) arguments
    if ix >= 0x7ff0_0000 {
        // x is inf or NaN
        y[0] = x86_nan(x); // C: y[0]=y[1]=x-x
        y[1] = y[0];
        return 0;
    }
    // set z = scalbn(|x|,ilogb(x)-23)
    let e0 = (ix >> 20) - 1046; // e0 = ilogb(z)-23;
    // __LO(z) = __LO(x); __HI(z) = ix - (e0<<20);
    let mut z = with_hi(f64::from_bits(u64::from(lo(x))), ix - (e0 << 20));
    let mut tx = [0.0f64; 3];
    for i in 0..2 {
        tx[i] = f64::from(z as i32);
        z = (z - tx[i]) * TWO24;
    }
    tx[2] = z;
    let mut nx = 3;
    while tx[nx - 1] == ZERO {
        nx -= 1; // skip zero term
    }
    let mut yy = [0.0f64; 3];
    let n = kernel_rem_pio2(&tx, &mut yy, e0, nx as i32, 2, &TWO_OVER_PI);
    y[0] = yy[0];
    y[1] = yy[1];
    if hx < 0 {
        y[0] = -y[0];
        y[1] = -y[1];
        return -n;
    }
    n
}

/// fdlibm `__kernel_rem_pio2(x, y, e0, nx, prec, ipio2)` (`k_rem_pio2.c`):
/// the last three bits of `N` with `y = x - N*pi/2`, `|y| < pi/2`, for `x`
/// given as `nx` 24-bit pieces scaled by `2^e0`. `prec` 0..=3 selects
/// 24/53/64/113-bit output in 1, 2, 2 or 3 elements of `y`; `ipio2` holds
/// 2/pi in 24-bit chunks.
fn kernel_rem_pio2(x: &[f64], y: &mut [f64], e0: i32, nx: i32, prec: i32, ipio2: &[i32]) -> i32 {
    const INIT_JK: [i32; 4] = [2, 3, 4, 6]; // initial value for jk
    const PIO2: [f64; 8] = [
        f64::from_bits(0x3ff9_21fb_4000_0000), // 1.57079625129699707031e+00
        f64::from_bits(0x3e74_442d_0000_0000), // 7.54978941586159635335e-08
        f64::from_bits(0x3cf8_4698_8000_0000), // 5.39030252995776476554e-15
        f64::from_bits(0x3b78_cc51_6000_0000), // 3.28200341580791294123e-22
        f64::from_bits(0x39f0_1b83_8000_0000), // 1.27065575308067607349e-29
        f64::from_bits(0x387a_2520_4000_0000), // 1.22933308981111328932e-36
        f64::from_bits(0x36e3_8222_8000_0000), // 2.73370053816464559624e-44
        f64::from_bits(0x3569_f31d_0000_0000), // 2.16741683877804819444e-51
    ];
    const ZERO: f64 = 0.0;
    const ONE: f64 = 1.0;
    const TWO24: f64 = f64::from_bits(0x4170_0000_0000_0000); // 1.67772160000000000000e+07
    const TWON24: f64 = f64::from_bits(0x3e70_0000_0000_0000); // 5.96046447753906250000e-08

    // C indexes with int; `u` converts a (non-negative) index.
    let u = |i: i32| i as usize;

    let mut iq = [0i32; 20];
    let mut f = [0.0f64; 20];
    let mut fq = [0.0f64; 20];
    let mut q = [0.0f64; 20];

    // initialize jk
    let jk = INIT_JK[u(prec)];
    let jp = jk;

    // determine jx,jv,q0, note that 3>q0
    let jx = nx - 1;
    let mut jv = (e0 - 3) / 24;
    if jv < 0 {
        jv = 0;
    }
    let mut q0 = e0 - 24 * (jv + 1);

    // set up f[0] to f[jx+jk] where f[jx+jk] = ipio2[jv+jk]
    let mut j = jv - jx;
    let m = jx + jk;
    for i in 0..=m {
        f[u(i)] = if j < 0 { ZERO } else { f64::from(ipio2[u(j)]) };
        j += 1;
    }

    // compute q[0],q[1],...q[jk]
    for i in 0..=jk {
        let mut fw = 0.0;
        for j in 0..=jx {
            fw += x[u(j)] * f[u(jx + i - j)];
        }
        q[u(i)] = fw;
    }

    let mut jz = jk;
    let mut z;
    let mut n;
    let mut ih;
    loop {
        // recompute:
        // distill q[] into iq[] reversingly
        let mut i = 0;
        let mut j = jz;
        z = q[u(jz)];
        while j > 0 {
            let fw = f64::from((TWON24 * z) as i32);
            iq[u(i)] = (z - TWO24 * fw) as i32;
            z = q[u(j - 1)] + fw;
            i += 1;
            j -= 1;
        }

        // compute n
        z = scalbn(z, q0); // actual value of z
        z -= 8.0 * (z * 0.125).floor(); // trim off integer >= 8
        n = z as i32;
        z -= f64::from(n);
        ih = 0;
        if q0 > 0 {
            // need iq[jz-1] to determine n
            let i = iq[u(jz - 1)] >> (24 - q0);
            n += i;
            iq[u(jz - 1)] -= i << (24 - q0);
            ih = iq[u(jz - 1)] >> (23 - q0);
        } else if q0 == 0 {
            ih = iq[u(jz - 1)] >> 23;
        } else if z >= 0.5 {
            ih = 2;
        }

        if ih > 0 {
            // q > 0.5
            n += 1;
            let mut carry = 0;
            for i in 0..jz {
                // compute 1-q
                let j = iq[u(i)];
                if carry == 0 {
                    if j != 0 {
                        carry = 1;
                        iq[u(i)] = 0x0100_0000 - j;
                    }
                } else {
                    iq[u(i)] = 0x00ff_ffff - j;
                }
            }
            if q0 > 0 {
                // rare case: chance is 1 in 12
                match q0 {
                    1 => iq[u(jz - 1)] &= 0x007f_ffff,
                    2 => iq[u(jz - 1)] &= 0x003f_ffff,
                    _ => {}
                }
            }
            if ih == 2 {
                z = ONE - z;
                if carry != 0 {
                    z -= scalbn(ONE, q0);
                }
            }
        }

        // check if recomputation is needed
        if z == ZERO {
            let mut j = 0;
            let mut i = jz - 1;
            while i >= jk {
                j |= iq[u(i)];
                i -= 1;
            }
            if j == 0 {
                // need recomputation
                let mut k = 1;
                while iq[u(jk - k)] == 0 {
                    k += 1; // k = no. of terms needed
                }

                for i in jz + 1..=jz + k {
                    // add q[jz+1] to q[jz+k]
                    f[u(jx + i)] = f64::from(ipio2[u(jv + i)]);
                    let mut fw = 0.0;
                    for j in 0..=jx {
                        fw += x[u(j)] * f[u(jx + i - j)];
                    }
                    q[u(i)] = fw;
                }
                jz += k;
                continue; // goto recompute
            }
        }
        break;
    }

    // chop off zero terms
    if z == 0.0 {
        jz -= 1;
        q0 -= 24;
        while iq[u(jz)] == 0 {
            jz -= 1;
            q0 -= 24;
        }
    } else {
        // break z into 24-bit if necessary
        z = scalbn(z, -q0);
        if z >= TWO24 {
            let fw = f64::from((TWON24 * z) as i32);
            iq[u(jz)] = (z - TWO24 * fw) as i32;
            jz += 1;
            q0 += 24;
            iq[u(jz)] = fw as i32;
        } else {
            iq[u(jz)] = z as i32;
        }
    }

    // convert integer "bit" chunk to floating-point value
    let mut fw = scalbn(ONE, q0);
    let mut i = jz;
    while i >= 0 {
        q[u(i)] = fw * f64::from(iq[u(i)]);
        fw *= TWON24;
        i -= 1;
    }

    // compute PIo2[0,...,jp]*q[jz,...,0]
    let mut i = jz;
    while i >= 0 {
        let mut fw = 0.0;
        let mut k = 0;
        while k <= jp && k <= jz - i {
            fw += PIO2[u(k)] * q[u(i + k)];
            k += 1;
        }
        fq[u(jz - i)] = fw;
        i -= 1;
    }

    // compress fq[] into y[]
    match prec {
        0 => {
            let mut fw = 0.0;
            let mut i = jz;
            while i >= 0 {
                fw += fq[u(i)];
                i -= 1;
            }
            y[0] = if ih == 0 { fw } else { -fw };
        }
        1 | 2 => {
            let mut fw = 0.0;
            let mut i = jz;
            while i >= 0 {
                fw += fq[u(i)];
                i -= 1;
            }
            y[0] = if ih == 0 { fw } else { -fw };
            fw = fq[0] - fw;
            for i in 1..=jz {
                fw += fq[u(i)];
            }
            y[1] = if ih == 0 { fw } else { -fw };
        }
        _ => {
            // case 3: painful
            let mut i = jz;
            while i > 0 {
                let fw = fq[u(i - 1)] + fq[u(i)];
                fq[u(i)] += fq[u(i - 1)] - fw;
                fq[u(i - 1)] = fw;
                i -= 1;
            }
            let mut i = jz;
            while i > 1 {
                let fw = fq[u(i - 1)] + fq[u(i)];
                fq[u(i)] += fq[u(i - 1)] - fw;
                fq[u(i - 1)] = fw;
                i -= 1;
            }
            let mut fw = 0.0;
            let mut i = jz;
            while i >= 2 {
                fw += fq[u(i)];
                i -= 1;
            }
            if ih == 0 {
                y[0] = fq[0];
                y[1] = fq[1];
                y[2] = fw;
            } else {
                y[0] = -fq[0];
                y[1] = -fq[1];
                y[2] = -fw;
            }
        }
    }
    n & 7
}

/// `StrictMath.sin(x)`: fdlibm 5.3 `sin` (`s_sin.c`).
///
/// `sin(+-inf)` and `sin(NaN)` are NaN; `sin(+-0)` is `+-0`.
pub fn sin(x: f64) -> f64 {
    let mut y = [0.0f64; 2];
    let z = 0.0;

    // High word of x.
    let mut ix = hi(x);

    // |x| ~< pi/4
    ix &= 0x7fff_ffff;
    if ix <= 0x3fe9_21fb {
        return kernel_sin(x, z, 0);
    }
    // sin(Inf or NaN) is NaN
    else if ix >= 0x7ff0_0000 {
        return x86_nan(x); // C: x-x
    }
    // argument reduction needed
    else {
        let n = rem_pio2(x, &mut y);
        match n & 3 {
            0 => kernel_sin(y[0], y[1], 1),
            1 => kernel_cos(y[0], y[1]),
            2 => -kernel_sin(y[0], y[1], 1),
            _ => -kernel_cos(y[0], y[1]),
        }
    }
}

/// `StrictMath.cos(x)`: fdlibm 5.3 `cos` (`s_cos.c`).
///
/// `cos(+-inf)` and `cos(NaN)` are NaN.
pub fn cos(x: f64) -> f64 {
    let mut y = [0.0f64; 2];
    let z = 0.0;

    // High word of x.
    let mut ix = hi(x);

    // |x| ~< pi/4
    ix &= 0x7fff_ffff;
    if ix <= 0x3fe9_21fb {
        return kernel_cos(x, z);
    }
    // cos(Inf or NaN) is NaN
    else if ix >= 0x7ff0_0000 {
        return x86_nan(x); // C: x-x
    }
    // argument reduction needed
    else {
        let n = rem_pio2(x, &mut y);
        match n & 3 {
            0 => kernel_cos(y[0], y[1]),
            1 => -kernel_sin(y[0], y[1], 1),
            2 => -kernel_cos(y[0], y[1]),
            _ => kernel_sin(y[0], y[1], 1),
        }
    }
}

/// `StrictMath.atan(x)`: fdlibm 5.3 `atan` (`s_atan.c`).
pub fn atan(x: f64) -> f64 {
    const ATANHI: [f64; 4] = [
        f64::from_bits(0x3fdd_ac67_0561_bb4f), // 4.63647609000806093515e-01 atan(0.5)hi
        f64::from_bits(0x3fe9_21fb_5444_2d18), // 7.85398163397448278999e-01 atan(1.0)hi
        f64::from_bits(0x3fef_730b_d281_f69b), // 9.82793723247329054082e-01 atan(1.5)hi
        f64::from_bits(0x3ff9_21fb_5444_2d18), // 1.57079632679489655800e+00 atan(inf)hi
    ];
    const ATANLO: [f64; 4] = [
        f64::from_bits(0x3c7a_2b7f_222f_65e2), // 2.26987774529616870924e-17 atan(0.5)lo
        f64::from_bits(0x3c81_a626_3314_5c07), // 3.06161699786838301793e-17 atan(1.0)lo
        f64::from_bits(0x3c70_0788_7af0_cbbd), // 1.39033110312309984516e-17 atan(1.5)lo
        f64::from_bits(0x3c91_a626_3314_5c07), // 6.12323399573676603587e-17 atan(inf)lo
    ];
    const AT: [f64; 11] = [
        f64::from_bits(0x3fd5_5555_5555_550d), // 3.33333333333329318027e-01
        f64::from_bits(0xbfc9_9999_9998_ebc4), // -1.99999999998764832476e-01
        f64::from_bits(0x3fc2_4924_9200_83ff), // 1.42857142725034663711e-01
        f64::from_bits(0xbfbc_71c6_fe23_1671), // -1.11111104054623557880e-01
        f64::from_bits(0x3fb7_45cd_c54c_206e), // 9.09088713343650656196e-02
        f64::from_bits(0xbfb3_b0f2_af74_9a6d), // -7.69187620504482999495e-02
        f64::from_bits(0x3fb1_0d66_a0d0_3d51), // 6.66107313738753120669e-02
        f64::from_bits(0xbfad_de2d_52de_fd9a), // -5.83357013379057348645e-02
        f64::from_bits(0x3fa9_7b4b_2476_0deb), // 4.97687799461593236017e-02
        f64::from_bits(0xbfa2_b444_2c6a_6c2f), // -3.65315727442169155270e-02
        f64::from_bits(0x3f90_ad3a_e322_da11), // 1.62858201153657823623e-02
    ];
    const ONE: f64 = 1.0;
    const HUGE: f64 = 1.0e300;

    let mut x = x;
    let hx = hi(x);
    let ix = hx & 0x7fff_ffff;
    let id: i32;
    if ix >= 0x4410_0000 {
        // if |x| >= 2^66
        if ix > 0x7ff0_0000 || (ix == 0x7ff0_0000 && (lo(x) != 0)) {
            return x86_add(x, x); // NaN
        }
        if hx > 0 {
            return ATANHI[3] + ATANLO[3];
        } else {
            return -ATANHI[3] - ATANLO[3];
        }
    }
    if ix < 0x3fdc_0000 {
        // |x| < 0.4375
        if ix < 0x3e20_0000 {
            // |x| < 2^-29
            if HUGE + x > ONE {
                return x; // raise inexact
            }
        }
        id = -1;
    } else {
        x = x.abs();
        if ix < 0x3ff3_0000 {
            // |x| < 1.1875
            if ix < 0x3fe6_0000 {
                // 7/16 <=|x|<11/16
                id = 0;
                x = (2.0 * x - ONE) / (2.0 + x);
            } else {
                // 11/16<=|x|< 19/16
                id = 1;
                x = (x - ONE) / (x + ONE);
            }
        } else {
            if ix < 0x4003_8000 {
                // |x| < 2.4375
                id = 2;
                x = (x - 1.5) / (ONE + 1.5 * x);
            } else {
                // 2.4375 <= |x| < 2^66
                id = 3;
                x = -1.0 / x;
            }
        }
    }
    // end of argument reduction
    let mut z = x * x;
    let w = z * z;
    // break sum from i=0 to 10 aT[i]z**(i+1) into odd and even poly
    let s1 = z * (AT[0] + w * (AT[2] + w * (AT[4] + w * (AT[6] + w * (AT[8] + w * AT[10])))));
    let s2 = w * (AT[1] + w * (AT[3] + w * (AT[5] + w * (AT[7] + w * AT[9]))));
    if id < 0 {
        return x - x * (s1 + s2);
    } else {
        let id = id as usize;
        z = ATANHI[id] - ((x * (s1 + s2) - ATANLO[id]) - x);
        return if hx < 0 { -z } else { z };
    }
}

/// `StrictMath.atan2(y, x)`: fdlibm 5.3 `__ieee754_atan2` (`e_atan2.c`).
/// Java argument order: the first argument is `y`.
pub fn atan2(y: f64, x: f64) -> f64 {
    const TINY: f64 = 1.0e-300;
    const ZERO: f64 = 0.0;
    const PI_O_4: f64 = f64::from_bits(0x3fe9_21fb_5444_2d18); // 7.8539816339744827900E-01
    const PI_O_2: f64 = f64::from_bits(0x3ff9_21fb_5444_2d18); // 1.5707963267948965580E+00
    const PI: f64 = f64::from_bits(0x4009_21fb_5444_2d18); // 3.1415926535897931160E+00
    const PI_LO: f64 = f64::from_bits(0x3ca1_a626_3314_5c07); // 1.2246467991473531772E-16

    let hx = hi(x);
    let ix = hx & 0x7fff_ffff;
    let lx = lo(x);
    let hy = hi(y);
    let iy = hy & 0x7fff_ffff;
    let ly = lo(y);
    if ((ix as u32 | ((lx | lx.wrapping_neg()) >> 31)) > 0x7ff0_0000)
        || ((iy as u32 | ((ly | ly.wrapping_neg()) >> 31)) > 0x7ff0_0000)
    {
        // x or y is NaN
        return x86_add(x, y);
    }
    if (hx.wrapping_sub(0x3ff0_0000) | lx as i32) == 0 {
        return atan(y); // x=1.0
    }
    let m = ((hy >> 31) & 1) | ((hx >> 30) & 2); // 2*sign(x)+sign(y)

    // when y = 0
    if (iy as u32 | ly) == 0 {
        match m {
            0 | 1 => return y,      // atan(+-0,+anything)=+-0
            2 => return PI + TINY,  // atan(+0,-anything) = pi
            _ => return -PI - TINY, // atan(-0,-anything) =-pi
        }
    }
    // when x = 0
    if (ix as u32 | lx) == 0 {
        return if hy < 0 {
            -PI_O_2 - TINY
        } else {
            PI_O_2 + TINY
        };
    }

    // when x is INF
    if ix == 0x7ff0_0000 {
        if iy == 0x7ff0_0000 {
            match m {
                0 => return PI_O_4 + TINY,        // atan(+INF,+INF)
                1 => return -PI_O_4 - TINY,       // atan(-INF,+INF)
                2 => return 3.0 * PI_O_4 + TINY,  // atan(+INF,-INF)
                _ => return -3.0 * PI_O_4 - TINY, // atan(-INF,-INF)
            }
        } else {
            match m {
                0 => return ZERO,       // atan(+...,+INF)
                1 => return -ZERO,      // atan(-...,+INF)
                2 => return PI + TINY,  // atan(+...,-INF)
                _ => return -PI - TINY, // atan(-...,-INF)
            }
        }
    }
    // when y is INF
    if iy == 0x7ff0_0000 {
        return if hy < 0 {
            -PI_O_2 - TINY
        } else {
            PI_O_2 + TINY
        };
    }

    // compute y/x
    let k = (iy - ix) >> 20;
    let z = if k > 60 {
        PI_O_2 + 0.5 * PI_LO // |y/x| >  2**60
    } else if hx < 0 && k < -60 {
        0.0 // |y|/x < -2**60
    } else {
        atan((y / x).abs()) // safe to do y/x
    };
    match m {
        0 => z,                            // atan(+,+)
        1 => with_hi(z, hi(z) ^ i32::MIN), // atan(-,+): __HI(z) ^= 0x80000000
        2 => PI - (z - PI_LO),             // atan(+,-)
        _ => (z - PI_LO) - PI,             // atan(-,-)
    }
}

/// `StrictMath.pow(x, y)`: fdlibm 5.3 `__ieee754_pow` (`e_pow.c` 1.5
/// 04/04/22).
///
/// Special cases (from the C): `x**0 = 1`; `x**1 = x`; `x**NaN` and
/// `NaN**y` (`y != 0`) are NaN; `(|x|>1)**+inf = +inf`, `(|x|>1)**-inf = +0`,
/// `(|x|<1)**+inf = +0`, `(|x|<1)**-inf = +inf`; `(+-1)**(+-inf)` is NaN;
/// `+0**(y>0) = +0`, `+0**(y<0) = +inf`; `-0**(odd int > 0) = -0`,
/// `-0**(odd int < 0) = -inf`, other `-0**y` as `+0**y`; `+inf**(y>0) = +inf`,
/// `+inf**(y<0) = +0`; `-inf**y = -0**(-y)`; `(-x)**(int) =
/// (-1)**int * x**int`; `(-x)**(non-int)` (finite, non-zero `x`) is NaN.
pub fn pow(x: f64, y: f64) -> f64 {
    const BP: [f64; 2] = [1.0, 1.5];
    const DP_H: [f64; 2] = [0.0, f64::from_bits(0x3fe2_b803_4000_0000)]; // 5.84962487220764160156e-01
    const DP_L: [f64; 2] = [0.0, f64::from_bits(0x3e4c_fdeb_43cf_d006)]; // 1.35003920212974897128e-08
    const ZERO: f64 = 0.0;
    const ONE: f64 = 1.0;
    const TWO: f64 = 2.0;
    const TWO53: f64 = f64::from_bits(0x4340_0000_0000_0000); // 9007199254740992.0
    const HUGE: f64 = 1.0e300;
    const TINY: f64 = 1.0e-300;
    // poly coefs for (3/2)*(log(x)-2s-2/3*s**3
    const L1: f64 = f64::from_bits(0x3fe3_3333_3333_3303); // 5.99999999999994648725e-01
    const L2: f64 = f64::from_bits(0x3fdb_6db6_db6f_abff); // 4.28571428578550184252e-01
    const L3: f64 = f64::from_bits(0x3fd5_5555_518f_264d); // 3.33333329818377432918e-01
    const L4: f64 = f64::from_bits(0x3fd1_7460_a91d_4101); // 2.72728123808534006489e-01
    const L5: f64 = f64::from_bits(0x3fcd_864a_93c9_db65); // 2.30660745775561754067e-01
    const L6: f64 = f64::from_bits(0x3fca_7e28_4a45_4eef); // 2.06975017800338417784e-01
    const P1: f64 = f64::from_bits(0x3fc5_5555_5555_553e); // 1.66666666666666019037e-01
    const P2: f64 = f64::from_bits(0xbf66_c16c_16be_bd93); // -2.77777777770155933842e-03
    const P3: f64 = f64::from_bits(0x3f11_566a_af25_de2c); // 6.61375632143793436117e-05
    const P4: f64 = f64::from_bits(0xbebb_bd41_c5d2_6bf1); // -1.65339022054652515390e-06
    const P5: f64 = f64::from_bits(0x3e66_3769_72be_a4d0); // 4.13813679705723846039e-08
    const LG2: f64 = f64::from_bits(0x3fe6_2e42_fefa_39ef); // 6.93147180559945286227e-01
    const LG2_H: f64 = f64::from_bits(0x3fe6_2e43_0000_0000); // 6.93147182464599609375e-01
    const LG2_L: f64 = f64::from_bits(0xbe20_5c61_0ca8_6c39); // -1.90465429995776804525e-09
    const OVT: f64 = 8.0085662595372944372e-17; // -(1024-log2(ovfl+.5ulp))
    const CP: f64 = f64::from_bits(0x3fee_c709_dc3a_03fd); // 9.61796693925975554329e-01 =2/(3ln2)
    const CP_H: f64 = f64::from_bits(0x3fee_c709_e000_0000); // 9.61796700954437255859e-01 =(float)cp
    const CP_L: f64 = f64::from_bits(0xbe3e_2fe0_145b_01f5); // -7.02846165095275826516e-09 =tail of cp_h
    const IVLN2: f64 = f64::from_bits(0x3ff7_1547_652b_82fe); // 1.44269504088896338700e+00 =1/ln2
    const IVLN2_H: f64 = f64::from_bits(0x3ff7_1547_6000_0000); // 1.44269502162933349609e+00 =24b 1/ln2
    const IVLN2_L: f64 = f64::from_bits(0x3e54_ae0b_f85d_df44); // 1.92596299112661746887e-08 =1/ln2 tail

    // (`i0`/`i1`, the C's endianness probe, are unused by this path.)
    let hx = hi(x);
    let lx = lo(x);
    let hy = hi(y);
    let ly = lo(y);
    let mut ix = hx & 0x7fff_ffff;
    let iy = hy & 0x7fff_ffff;

    // y==zero: x**0 = 1
    if (iy as u32 | ly) == 0 {
        return ONE;
    }

    // +-NaN return x+y
    if ix > 0x7ff0_0000
        || ((ix == 0x7ff0_0000) && (lx != 0))
        || iy > 0x7ff0_0000
        || ((iy == 0x7ff0_0000) && (ly != 0))
    {
        return x86_add(x, y);
    }

    // determine if y is an odd int when x < 0
    // yisint = 0 ... y is not an integer
    // yisint = 1 ... y is an odd int
    // yisint = 2 ... y is an even int
    let mut yisint = 0;
    if hx < 0 {
        if iy >= 0x4340_0000 {
            yisint = 2; // even integer y
        } else if iy >= 0x3ff0_0000 {
            let k = (iy >> 20) - 0x3ff; // exponent
            if k > 20 {
                let j = ly >> (52 - k);
                if (j << (52 - k)) == ly {
                    yisint = 2 - (j & 1) as i32;
                }
            } else if ly == 0 {
                let j = iy >> (20 - k);
                if (j << (20 - k)) == iy {
                    yisint = 2 - (j & 1);
                }
            }
        }
    }

    // special value of y
    if ly == 0 {
        if iy == 0x7ff0_0000 {
            // y is +-inf
            if (ix.wrapping_sub(0x3ff0_0000) as u32 | lx) == 0 {
                return x86_nan(y); // C: y - y, inf**+-1 is NaN
            } else if ix >= 0x3ff0_0000 {
                // (|x|>1)**+-inf = inf,0
                return if hy >= 0 { y } else { ZERO };
            } else {
                // (|x|<1)**-,+inf = inf,0
                return if hy < 0 { -y } else { ZERO };
            }
        }
        if iy == 0x3ff0_0000 {
            // y is  +-1
            if hy < 0 {
                return ONE / x;
            } else {
                return x;
            }
        }
        if hy == 0x4000_0000 {
            return x * x; // y is  2
        }
        if hy == 0x3fe0_0000 {
            // y is  0.5
            if hx >= 0 {
                // x >= +0
                return x.sqrt();
            }
        }
    }

    let mut ax = x.abs();
    // special value of x
    if lx == 0 {
        if ix == 0x7ff0_0000 || ix == 0 || ix == 0x3ff0_0000 {
            let mut z = ax; // x is +-0,+-inf,+-1
            if hy < 0 {
                z = ONE / z; // z = (1/|x|)
            }
            if hx < 0 {
                if ((ix - 0x3ff0_0000) | yisint) == 0 {
                    z = x86_nan(z); // C: (z-z)/(z-z), (-1)**non-int is NaN
                } else if yisint == 1 {
                    z = -z; // (x<0)**odd = -(|x|**odd)
                }
            }
            return z;
        }
    }

    let mut n = (hx >> 31) + 1;

    // (x<0)**(non-int) is NaN
    if (n | yisint) == 0 {
        return x86_nan(x); // C: (x-x)/(x-x)
    }

    let mut s = ONE; // s (sign of result -ve**odd) = -1 else = 1
    if (n | (yisint - 1)) == 0 {
        s = -ONE; // (-ve)**(odd int)
    }

    let t1;
    let t2;
    // |y| is huge
    if iy > 0x41e0_0000 {
        // if |y| > 2**31
        if iy > 0x43f0_0000 {
            // if |y| > 2**64, must o/uflow
            if ix <= 0x3fef_ffff {
                return if hy < 0 { HUGE * HUGE } else { TINY * TINY };
            }
            if ix >= 0x3ff0_0000 {
                return if hy > 0 { HUGE * HUGE } else { TINY * TINY };
            }
        }
        // over/underflow if x is not close to one
        if ix < 0x3fef_ffff {
            return if hy < 0 {
                s * HUGE * HUGE
            } else {
                s * TINY * TINY
            };
        }
        if ix > 0x3ff0_0000 {
            return if hy > 0 {
                s * HUGE * HUGE
            } else {
                s * TINY * TINY
            };
        }
        // now |1-x| is tiny <= 2**-20, suffice to compute
        // log(x) by x-x^2/2+x^3/3-x^4/4
        let t = ax - ONE; // t has 20 trailing zeros
        let w = (t * t) * (0.5 - t * (0.3333333333333333333333 - t * 0.25));
        let u = IVLN2_H * t; // ivln2_h has 21 sig. bits
        let v = t * IVLN2_L - w * IVLN2;
        t1 = with_lo(u + v, 0);
        t2 = v - (t1 - u);
    } else {
        n = 0;
        // take care subnormal number
        if ix < 0x0010_0000 {
            ax *= TWO53;
            n -= 53;
            ix = hi(ax);
        }
        n += (ix >> 20) - 0x3ff;
        let j = ix & 0x000f_ffff;
        // determine interval
        ix = j | 0x3ff0_0000; // normalize ix
        let k: usize;
        if j <= 0x3988E {
            k = 0; // |x|<sqrt(3/2)
        } else if j < 0xBB67A {
            k = 1; // |x|<sqrt(3)
        } else {
            k = 0;
            n += 1;
            ix -= 0x0010_0000;
        }
        ax = with_hi(ax, ix);

        // compute ss = s_h+s_l = (x-1)/(x+1) or (x-1.5)/(x+1.5)
        let mut u = ax - BP[k]; // bp[0]=1.0, bp[1]=1.5
        let mut v = ONE / (ax + BP[k]);
        let ss = u * v;
        let s_h = with_lo(ss, 0);
        // t_h=ax+bp[k] High
        let mut t_h = with_hi(
            ZERO,
            ((ix >> 1) | 0x2000_0000) + 0x0008_0000 + ((k as i32) << 18),
        );
        let mut t_l = ax - (t_h - BP[k]);
        let s_l = v * ((u - s_h * t_h) - s_h * t_l);
        // compute log(ax)
        let mut s2 = ss * ss;
        let mut r = s2 * s2 * (L1 + s2 * (L2 + s2 * (L3 + s2 * (L4 + s2 * (L5 + s2 * L6)))));
        r += s_l * (s_h + ss);
        s2 = s_h * s_h;
        t_h = with_lo(3.0 + s2 + r, 0);
        t_l = r - ((t_h - 3.0) - s2);
        // u+v = ss*(1+...)
        u = s_h * t_h;
        v = s_l * t_h + t_l * ss;
        // 2/(3log2)*(ss+...)
        let p_h = with_lo(u + v, 0);
        let p_l = v - (p_h - u);
        let z_h = CP_H * p_h; // cp_h+cp_l = 2/(3*log2)
        let z_l = CP_L * p_h + p_l * CP + DP_L[k];
        // log2(ax) = (ss+..)*2/(3*log2) = n + dp_h + z_h + z_l
        let t = f64::from(n);
        t1 = with_lo(((z_h + z_l) + DP_H[k]) + t, 0);
        t2 = z_l - (((t1 - t) - DP_H[k]) - z_h);
    }

    // split up y into y1+y2 and compute (y1+y2)*(t1+t2)
    let y1 = with_lo(y, 0);
    let p_l = (y - y1) * t1 + y * t2;
    let mut p_h = y1 * t1;
    let mut z = p_l + p_h;
    let mut j = hi(z);
    let mut i = lo(z) as i32;
    if j >= 0x4090_0000 {
        // z >= 1024
        if ((j - 0x4090_0000) | i) != 0 {
            // if z > 1024
            return s * HUGE * HUGE; // overflow
        } else {
            if p_l + OVT > z - p_h {
                return s * HUGE * HUGE; // overflow
            }
        }
    } else if (j & 0x7fff_ffff) >= 0x4090_cc00 {
        // z <= -1075
        if ((j as u32).wrapping_sub(0xc090_cc00) | i as u32) != 0 {
            // z < -1075
            return s * TINY * TINY; // underflow
        } else {
            if p_l <= z - p_h {
                return s * TINY * TINY; // underflow
            }
        }
    }
    // compute 2**(p_h+p_l)
    i = j & 0x7fff_ffff;
    let mut k = (i >> 20) - 0x3ff;
    n = 0;
    if i > 0x3fe0_0000 {
        // if |z| > 0.5, set n = [z+0.5]
        n = j + (0x0010_0000 >> (k + 1));
        k = ((n & 0x7fff_ffff) >> 20) - 0x3ff; // new k for n
        let t = with_hi(ZERO, n & !(0x000f_ffff >> k));
        n = ((n & 0x000f_ffff) | 0x0010_0000) >> (20 - k);
        if j < 0 {
            n = -n;
        }
        p_h -= t;
    }
    let mut t = with_lo(p_l + p_h, 0);
    let u = t * LG2_H;
    let v = (p_l - (t - p_h)) * LG2 + t * LG2_L;
    z = u + v;
    let w = v - (z - u);
    t = z * z;
    let t1 = z - t * (P1 + t * (P2 + t * (P3 + t * (P4 + t * P5))));
    let r = (z * t1) / (t1 - TWO) - (w + z * w);
    z = ONE - (r - z);
    j = hi(z);
    j = j.wrapping_add(n << 20);
    if (j >> 20) <= 0 {
        z = scalbn(z, n); // subnormal output
    } else {
        z = with_hi(z, hi(z).wrapping_add(n << 20));
    }
    s * z
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

    /// Asserts bit equality (so `-0.0 != 0.0` and NaN payloads count).
    #[track_caller]
    fn same(actual: f64, expected: f64) {
        assert_eq!(
            actual.to_bits(),
            expected.to_bits(),
            "actual {actual:?} (0x{:016x}), expected {expected:?} (0x{:016x})",
            actual.to_bits(),
            expected.to_bits()
        );
    }

    fn b(bits: u64) -> f64 {
        f64::from_bits(bits)
    }

    const INF: f64 = f64::INFINITY;
    const QNAN: f64 = f64::NAN; // 0x7ff8_0000_0000_0000
    /// What the oracle (x86-64) returns for an invalid operation.
    const DNAN: u64 = 0xfff8_0000_0000_0000;
    const PI: f64 = core::f64::consts::PI;

    #[test]
    fn nan_bit_patterns() {
        // Propagated NaNs keep their payload and sign, quieted.
        let payload = b(0x7ff0_0000_0000_0123); // signalling
        same(sin(payload), b(0x7ff8_0000_0000_0123));
        same(cos(-QNAN), -QNAN);
        same(atan(QNAN), QNAN);
        same(atan2(QNAN, 1.0), QNAN);
        same(atan2(1.0, -QNAN), -QNAN);
        same(pow(QNAN, 2.0), QNAN);
        same(pow(1.0, QNAN), QNAN);
        same(log(-QNAN), -QNAN);
        // Invalid operations give the x86-64 default NaN.
        same(sin(INF), b(DNAN));
        same(sin(-INF), b(DNAN));
        same(cos(INF), b(DNAN));
        same(pow(-1.0, INF), b(DNAN));
        same(pow(1.0, -INF), b(DNAN));
        same(pow(-8.0, 1.0 / 3.0), b(DNAN));
        same(pow(-4.0, 0.5), b(DNAN));
        same(log(-1.0), b(DNAN));
        same(log(-INF), b(DNAN));
    }

    #[test]
    fn pow_special_cases() {
        // Java StrictMath.pow special cases, in javadoc order.
        for x in [0.0, -0.0, 1.0, -1.0, 2.5, -2.5, INF, -INF, QNAN] {
            same(pow(x, 0.0), 1.0);
            same(pow(x, -0.0), 1.0);
            if !x.is_nan() {
                same(pow(x, 1.0), x);
            }
        }
        assert!(pow(QNAN, 1.0).is_nan());
        assert!(pow(2.0, QNAN).is_nan());
        assert!(pow(QNAN, 3.0).is_nan());
        // |x| > 1 / |x| < 1 with infinite exponents.
        same(pow(2.0, INF), INF);
        same(pow(-2.0, INF), INF);
        same(pow(2.0, -INF), 0.0);
        same(pow(0.5, INF), 0.0);
        same(pow(-0.5, -INF), INF);
        same(pow(0.5, -INF), INF);
        // +-0 bases.
        same(pow(0.0, 3.0), 0.0);
        same(pow(0.0, 0.5), 0.0);
        same(pow(-0.0, 3.0), -0.0);
        same(pow(-0.0, 2.0), 0.0);
        same(pow(-0.0, 0.5), 0.0);
        same(pow(0.0, -1.0), INF);
        same(pow(0.0, -2.0), INF);
        same(pow(-0.0, -3.0), -INF);
        same(pow(-0.0, -2.0), INF);
        same(pow(-0.0, -0.5), INF);
        // +-inf bases.
        same(pow(INF, 0.5), INF);
        same(pow(INF, -2.0), 0.0);
        same(pow(-INF, 3.0), -INF);
        same(pow(-INF, 2.0), INF);
        same(pow(-INF, 0.5), INF);
        same(pow(-INF, -3.0), -0.0);
        same(pow(-INF, -2.0), 0.0);
        // Negative bases with integer exponents.
        same(pow(-2.0, 3.0), -8.0);
        same(pow(-2.0, 4.0), 16.0);
        same(pow(-1.0, 1e300), 1.0); // |y| >= 2^53 is an even integer
        same(pow(-1.0, 9_007_199_254_740_991.0), -1.0); // 2^53 - 1, odd
        // Exact results.
        same(pow(2.0, 0.5), core::f64::consts::SQRT_2);
        same(pow(4.0, 0.5), 2.0);
        same(pow(2.0, -1.0), 0.5);
        same(pow(10.0, 22.0), 1e22);
        same(pow(3.0, 33.0), 5_559_060_566_555_523.0);
        same(pow(2.0, 1023.0), b(0x7fe0_0000_0000_0000));
        same(pow(-2.0, 1023.0), b(0xffe0_0000_0000_0000));
        // Subnormal results (scalbn path) and the underflow boundary.
        same(pow(2.0, -1074.0), b(1));
        same(pow(-2.0, -1074.0), b(1));
        same(pow(-2.0, -1073.0), -b(2));
        same(pow(0.5, 1074.0), b(1));
        same(pow(2.0, -1075.0), 0.0);
        same(pow(-2.0, -1075.0), -0.0);
        // Overflow.
        same(pow(2.0, 1024.0), INF);
        same(pow(-2.0, 1025.0), -INF);
        same(pow(10.0, 309.0), INF);
        same(pow(10.0, -400.0), 0.0);
        // |y| > 2^64: must over/underflow.
        same(pow(1.5, 1e20), INF);
        same(pow(1.5, -1e20), 0.0);
        same(pow(0.5, 1e20), 0.0);
        same(pow(0.5, -1e20), INF);
        same(pow(-1.5, 1e20), INF);
        // 2^31 < |y| <= 2^64, x not close to 1; 3e9 + 1 is odd.
        same(pow(1.5, 3e9), INF);
        same(pow(-1.5, 3e9 + 1.0), -INF);
        same(pow(-0.5, 3e9 + 1.0), -0.0);
        same(pow(0.5, -3e9), INF);
        // x close to 1 with 2^31 < y: the log(1+t) series path. The
        // correctly rounded (1 + 2^-52)^1e15 = 1.2486270715390861.
        same(pow(1.0 + f64::EPSILON, 1e15), b(0x3ff3_fa60_6152_91ee));
        same(pow(1.0 - f64::EPSILON, 1e300), 0.0);
    }

    #[test]
    fn sin_cos_special_cases() {
        same(sin(0.0), 0.0);
        same(sin(-0.0), -0.0);
        same(cos(0.0), 1.0);
        same(cos(-0.0), 1.0);
        same(sin(b(1)), b(1)); // tiny: sin(x) == x
        same(sin(-b(1)), -b(1));
        same(cos(b(1)), 1.0);
        // Well-known Java values.
        same(sin(PI), 1.2246467991473532e-16);
        same(cos(PI), -1.0);
        same(sin(PI / 2.0), 1.0);
        same(cos(PI / 2.0), 6.123233995736766e-17);
        same(sin(-PI / 2.0), -1.0);
        // Huge arguments (__kernel_rem_pio2); expected values are the
        // correctly rounded results (computed to 800 digits), which fdlibm
        // returns here except where noted, and match the math-sweep fixture
        // where it has them (1e22, MAX).
        same(sin(1e22), b(0xbfeb_453a_b76b_f397)); // -0.8522008497671888
        same(cos(1e22), b(0x3fe0_be2c_ef01_c8f4)); // 0.5232147853951389
        same(sin(-1e22), b(0x3feb_453a_b76b_f397));
        same(sin(b(0x7fe0_0000_0000_0000)), b(0x3fe2_0524_8cbd_b760)); // 2^1023
        // Not correctly rounded (exact: 0.63 ulp above ...be): fdlibm's
        // __kernel_cos error, reproduced by evaluating k_cos.c separately on
        // the reduced argument; glibc gives ...bf.
        same(cos(b(0x7fe0_0000_0000_0000)), b(0xbfea_719f_26c2_32be));
        same(sin(f64::MAX), b(0x3f74_52fc_98b3_4e97));
        same(cos(f64::MAX), b(0xbfef_ffe6_2ecf_ab75));
        same(sin(b(0x4770_0000_0000_0000)), b(0x3fd8_2e34_655e_6fcb)); // 2^120
        same(cos(b(0x4770_0000_0000_0000)), b(0xbfed_a0cd_0b66_d8ce));
        same(sin(1e300), b(0xbfea_2c16_b010_e385));
        same(cos(1e300), b(0xbfe2_6990_22ad_c4c1));
        // Medium arguments (rem_pio2's 2^19*pi/2 boundary and beyond).
        same(sin(1e6), b(0xbfd6_664b_2568_d867));
        same(cos(1e6), b(0x3fed_f9df_9906_d32c));
        same(sin(1e10), b(0xbfdf_334c_7896_a4e3));
        same(cos(1e10), b(0x3feb_f098_901c_931a));
    }

    #[test]
    fn atan_atan2_special_cases() {
        let pi_2 = core::f64::consts::FRAC_PI_2;
        let pi_4 = core::f64::consts::FRAC_PI_4;
        let three_pi_4 = b(0x4002_d97c_7f33_21d2); // 2.356194490192345
        same(atan(0.0), 0.0);
        same(atan(-0.0), -0.0);
        same(atan(1.0), pi_4);
        same(atan(-1.0), -pi_4);
        same(atan(INF), pi_2);
        same(atan(-INF), -pi_2);
        same(atan(1e300), pi_2);
        same(atan(b(1)), b(1));
        // Signed zeros.
        same(atan2(0.0, 0.0), 0.0);
        same(atan2(-0.0, 0.0), -0.0);
        same(atan2(0.0, -0.0), PI);
        same(atan2(-0.0, -0.0), -PI);
        same(atan2(0.0, 1.0), 0.0);
        same(atan2(-0.0, 1.0), -0.0);
        same(atan2(0.0, -1.0), PI);
        same(atan2(-0.0, -1.0), -PI);
        same(atan2(1.0, 0.0), pi_2);
        same(atan2(1.0, -0.0), pi_2);
        same(atan2(-1.0, 0.0), -pi_2);
        // Quadrants.
        same(atan2(1.0, 1.0), pi_4);
        same(atan2(-1.0, 1.0), -pi_4);
        same(atan2(1.0, -1.0), three_pi_4);
        same(atan2(-1.0, -1.0), -three_pi_4);
        // Infinities.
        same(atan2(INF, INF), pi_4);
        same(atan2(-INF, INF), -pi_4);
        same(atan2(INF, -INF), three_pi_4);
        same(atan2(-INF, -INF), -three_pi_4);
        same(atan2(1.0, INF), 0.0);
        same(atan2(-1.0, INF), -0.0);
        same(atan2(1.0, -INF), PI);
        same(atan2(-1.0, -INF), -PI);
        same(atan2(INF, 1.0), pi_2);
        same(atan2(-INF, -1.0), -pi_2);
        // |y/x| beyond 2^60 either way.
        same(atan2(1e300, 1e-300), pi_2);
        same(atan2(1e-300, -1e300), PI);
        same(atan2(-1e-300, -1e300), -PI);
        same(atan2(1e-300, 1e300), 0.0);
        // x == 1 delegates to atan.
        same(atan2(0.5, 1.0), atan(0.5));
    }

    #[test]
    fn scalbn_cases() {
        same(scalbn(1.0, -1074), b(1));
        same(scalbn(1.0, -1075), 0.0);
        same(scalbn(-1.0, 1024), -INF);
        same(scalbn(b(1), 1074), 1.0);
        same(scalbn(3.0, -1073), b(6));
        same(scalbn(-0.0, 5), -0.0);
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
