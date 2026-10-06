//! IEEE-754 bit-pattern strings and hex byte arrays (ADR-007 §5.2 item 1,
//! `tests/golden/README.md` "Encoding rules").
//!
//! `f32` is `"0x"` + 8 lower-case hex digits, `f64` is `"0x"` + 16. Parsing is
//! strict: wrong width, upper case or a missing prefix is an error, so a
//! fixture can never silently carry a decimal or a truncated pattern.

use std::fmt;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Parses `"0x"` + exactly `width` lower-case hex digits.
fn parse_fixed(s: &str, width: usize) -> Result<u64, String> {
    let digits = s
        .strip_prefix("0x")
        .ok_or_else(|| format!("{s:?}: missing \"0x\" prefix"))?;
    if digits.len() != width {
        return Err(format!(
            "{s:?}: expected {width} hex digits, found {}",
            digits.len()
        ));
    }
    if !digits.bytes().all(is_lower_hex) {
        return Err(format!("{s:?}: not lower-case hex"));
    }
    u64::from_str_radix(digits, 16).map_err(|e| format!("{s:?}: {e}"))
}

fn is_lower_hex(b: u8) -> bool {
    matches!(b, b'0'..=b'9' | b'a'..=b'f')
}

macro_rules! bits_type {
    ($name:ident, $int:ty, $float:ty, $width:literal, $to:ident) => {
        #[doc = concat!("Bit pattern of an `", stringify!($float), "`, written as `\"0x\"` + ", stringify!($width), " lower-case hex digits.")]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub $int);

        impl $name {
            /// Strict parse; see the module docs.
            pub fn parse(s: &str) -> Result<Self, String> {
                // The width check guarantees the value fits.
                parse_fixed(s, $width).map(|v| Self(v as $int))
            }

            #[doc = concat!("The `", stringify!($float), "` with these bits.")]
            pub fn $to(self) -> $float {
                <$float>::from_bits(self.0)
            }
        }

        impl From<$float> for $name {
            fn from(v: $float) -> Self {
                Self(v.to_bits())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "0x{:0width$x}", self.0, width = $width)
            }
        }

        impl From<$name> for serde_json::Value {
            fn from(v: $name) -> Self {
                serde_json::Value::String(v.to_string())
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.collect_str(self)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let s = String::deserialize(d)?;
                Self::parse(&s).map_err(D::Error::custom)
            }
        }
    };
}

bits_type!(Bits32, u32, f32, 8, to_f32);
bits_type!(Bits64, u64, f64, 16, to_f64);

/// Decodes a bit-pattern string for display, choosing `f32`/`f64` by width.
/// Returns `None` for anything that is not a valid [`Bits32`]/[`Bits64`].
pub fn decode_float(s: &str) -> Option<String> {
    match s.len() {
        10 => Bits32::parse(s)
            .ok()
            .map(|b| format!("{:?}f32", b.to_f32())),
        18 => Bits64::parse(s).ok().map(|b| format!("{:?}", b.to_f64())),
        _ => None,
    }
}

/// Lower-case hex of raw bytes, no prefix (the `"full": true` array format).
///
/// ```
/// assert_eq!(mc_golden::hex_bytes(&[0x00, 0xab, 0x7f]), "00ab7f");
/// ```
pub fn hex_bytes(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(DIGITS[usize::from(b >> 4)] as char);
        out.push(DIGITS[usize::from(b & 0xf)] as char);
    }
    out
}

/// Strict inverse of [`hex_bytes`]: even length, lower-case, no prefix.
pub fn parse_hex_bytes(s: &str) -> Result<Vec<u8>, String> {
    if !s.len().is_multiple_of(2) {
        return Err(format!("hex byte string has odd length {}", s.len()));
    }
    let nibble = |b: u8, i: usize| match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        _ => Err(format!(
            "invalid lower-case hex digit {:?} at {i}",
            b as char
        )),
    };
    s.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .enumerate()
        .map(|(i, pair)| Ok(nibble(pair[0], 2 * i)? << 4 | nibble(pair[1], 2 * i + 1)?))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        for v in [
            0.0f32,
            -0.0,
            1.5,
            f32::MIN_POSITIVE,
            f32::INFINITY,
            f32::NAN,
        ] {
            let s = Bits32::from(v).to_string();
            assert_eq!(s.len(), 10);
            assert_eq!(Bits32::parse(&s).unwrap().to_f32().to_bits(), v.to_bits());
        }
        for v in [0.0f64, -0.0, 0.5, -1e300, f64::NEG_INFINITY, f64::NAN] {
            let s = Bits64::from(v).to_string();
            assert_eq!(s.len(), 18);
            assert_eq!(Bits64::parse(&s).unwrap().to_f64().to_bits(), v.to_bits());
        }
        assert_eq!(Bits64::from(0.5).to_string(), "0x3fe0000000000000");
        assert_eq!(Bits32::from(1.0f32).to_string(), "0x3f800000");
        let json = serde_json::to_string(&Bits32(0xdead_beef)).unwrap();
        assert_eq!(json, "\"0xdeadbeef\"");
        assert_eq!(
            serde_json::from_str::<Bits32>(&json).unwrap(),
            Bits32(0xdead_beef)
        );
    }

    #[test]
    fn strict_parsing() {
        for bad in [
            "3f800000",
            "0X3f800000",
            "0x3F800000",
            "0x3f80000",
            "0x3f8000000",
            "0x3f80000g",
            "0x+f800000",
            "",
        ] {
            assert!(Bits32::parse(bad).is_err(), "{bad:?} accepted");
        }
        assert!(Bits64::parse("0x3f800000").is_err());
        assert!(Bits32::parse("0x3fe0000000000000").is_err());
        assert!(serde_json::from_str::<Bits64>("0.5").is_err());
        assert!(serde_json::from_str::<Bits64>("\"0x3FE0000000000000\"").is_err());
    }

    #[test]
    fn decode() {
        assert_eq!(decode_float("0x3fe0000000000000").as_deref(), Some("0.5"));
        assert_eq!(decode_float("0x3f800000").as_deref(), Some("1.0f32"));
        assert_eq!(decode_float("0x3f80000"), None);
        assert_eq!(decode_float("hello"), None);
    }

    #[test]
    fn hex_byte_arrays() {
        let all: Vec<u8> = (0..=255).collect();
        assert_eq!(parse_hex_bytes(&hex_bytes(&all)).unwrap(), all);
        assert_eq!(parse_hex_bytes("").unwrap(), Vec::<u8>::new());
        assert!(parse_hex_bytes("abc").is_err());
        assert!(parse_hex_bytes("AB").is_err());
        assert!(parse_hex_bytes("0x00").is_err());
    }
}
