// Adapted from MinecraftOSS third_party/minecraftoss/core/src/nbt.rs @ 4013a68 by chasmlol, Apache-2.0; modified
//! Java modified UTF-8, the string encoding of `DataOutput.writeUTF` /
//! `DataInput.readUTF` (NBTBase.java:31, :40; NBTTagString.java:21, :25).
//!
//! The text is taken as UTF-16 code units; each unit `0x0001..=0x007F` is one
//! byte, `0x0000` and `0x0080..=0x07FF` are two (so U+0000 is `c0 80`), the
//! rest three. A supplementary character is therefore two three-byte
//! surrogates, never a four-byte sequence. The byte length is a big-endian
//! `u16` prefix, written by the callers in `codec`.

use crate::ErrorKind;

/// Largest encoded length `writeUTF` accepts.
pub(crate) const MAX_LEN: usize = 0xFFFF;

/// Encoded length of `s` in bytes (`DataOutputStream.writeUTF`'s first pass).
pub(crate) fn encoded_len(s: &str) -> usize {
    s.encode_utf16()
        .map(|unit| match unit {
            0x0001..=0x007F => 1,
            0x0000 | 0x0080..=0x07FF => 2,
            _ => 3,
        })
        .sum()
}

/// Appends the modified UTF-8 bytes of `s` (no length prefix).
pub(crate) fn encode_into(s: &str, out: &mut Vec<u8>) {
    for unit in s.encode_utf16() {
        match unit {
            0x0001..=0x007F => out.push(unit as u8),
            0x0000 | 0x0080..=0x07FF => {
                out.push(0xC0 | (unit >> 6) as u8);
                out.push(0x80 | (unit & 0x3F) as u8);
            }
            _ => {
                out.push(0xE0 | (unit >> 12) as u8);
                out.push(0x80 | ((unit >> 6) & 0x3F) as u8);
                out.push(0x80 | (unit & 0x3F) as u8);
            }
        }
    }
}

/// Decodes the payload of a `readUTF` string (the bytes after the length).
///
/// Follows `DataInputStream.readUTF` byte for byte: a lead byte `0xxxxxxx`
/// is one unit (a raw `00` is accepted, as Java does), `110xxxxx` and
/// `1110xxxx` take one and two `10xxxxxx` continuation bytes, anything else
/// is malformed, and a sequence cut off by the length is malformed. Overlong
/// forms are accepted, as in Java. The resulting UTF-16 must then be valid
/// (paired surrogates), which Java does not require.
///
/// On error, returns the offset within `bytes` of the offending sequence.
pub(crate) fn decode(bytes: &[u8]) -> Result<String, (usize, ErrorKind)> {
    let mut out = String::with_capacity(bytes.len());
    let mut pending_high: Option<(usize, u16)> = None;
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        let b = bytes[i];
        let unit = match b >> 4 {
            0..=7 => {
                i += 1;
                u16::from(b)
            }
            12 | 13 => {
                let c = continuation(bytes, i + 1).ok_or((start, ErrorKind::MalformedUtf8))?;
                i += 2;
                (u16::from(b & 0x1F) << 6) | u16::from(c & 0x3F)
            }
            14 => {
                let c = continuation(bytes, i + 1).ok_or((start, ErrorKind::MalformedUtf8))?;
                let d = continuation(bytes, i + 2).ok_or((start, ErrorKind::MalformedUtf8))?;
                i += 3;
                (u16::from(b & 0x0F) << 12) | (u16::from(c & 0x3F) << 6) | u16::from(d & 0x3F)
            }
            _ => return Err((start, ErrorKind::MalformedUtf8)),
        };
        match (pending_high.take(), unit) {
            (None, 0xD800..=0xDBFF) => pending_high = Some((start, unit)),
            (Some((_, high)), 0xDC00..=0xDFFF) => {
                let c = 0x10000 + ((u32::from(high) - 0xD800) << 10) + (u32::from(unit) - 0xDC00);
                out.push(char::from_u32(c).ok_or((start, ErrorKind::UnpairedSurrogate(unit)))?);
            }
            (Some((at, high)), _) => return Err((at, ErrorKind::UnpairedSurrogate(high))),
            (None, 0xDC00..=0xDFFF) => return Err((start, ErrorKind::UnpairedSurrogate(unit))),
            (None, _) => out.push(
                char::from_u32(u32::from(unit))
                    .ok_or((start, ErrorKind::UnpairedSurrogate(unit)))?,
            ),
        }
    }
    if let Some((at, high)) = pending_high {
        return Err((at, ErrorKind::UnpairedSurrogate(high)));
    }
    Ok(out)
}

/// The byte at `i` if it is a `10xxxxxx` continuation byte.
fn continuation(bytes: &[u8], i: usize) -> Option<u8> {
    bytes.get(i).copied().filter(|c| c & 0xC0 == 0x80)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enc(s: &str) -> Vec<u8> {
        let mut v = Vec::new();
        encode_into(s, &mut v);
        assert_eq!(v.len(), encoded_len(s));
        v
    }

    #[test]
    fn nul_is_two_bytes() {
        assert_eq!(enc("a\0b"), [b'a', 0xC0, 0x80, b'b']);
        assert_eq!(decode(&[b'a', 0xC0, 0x80, b'b']).unwrap(), "a\0b");
        // Java's readUTF also accepts a raw zero byte.
        assert_eq!(decode(&[0]).unwrap(), "\0");
    }

    #[test]
    fn supplementary_is_a_surrogate_pair() {
        // U+1F600 = D83D DE00.
        let bytes = enc("\u{1F600}");
        assert_eq!(bytes, [0xED, 0xA0, 0xBD, 0xED, 0xB8, 0x80]);
        assert_eq!(decode(&bytes).unwrap(), "\u{1F600}");
        assert_ne!(bytes, "\u{1F600}".as_bytes(), "not standard UTF-8");
    }

    #[test]
    fn two_and_three_byte_forms() {
        let s = "\u{80}\u{7FF}\u{800}\u{FFFF}ÄÖÜ ☃";
        assert_eq!(decode(&enc(s)).unwrap(), s);
        assert_eq!(enc("Ä"), [0xC3, 0x84]);
        assert_eq!(enc("☃"), [0xE2, 0x98, 0x83]);
    }

    #[test]
    fn overlong_accepted_like_java() {
        assert_eq!(decode(&[0xC1, 0x81]).unwrap(), "A");
        assert_eq!(decode(&[0xE0, 0x80, 0x80]).unwrap(), "\0");
    }

    #[test]
    fn malformed_rejected() {
        let bad = |b: &[u8]| decode(b).unwrap_err();
        assert_eq!(bad(&[b'x', 0x80]), (1, ErrorKind::MalformedUtf8));
        assert_eq!(bad(&[0xF0, 0x9F, 0x98, 0x80]).1, ErrorKind::MalformedUtf8);
        assert_eq!(bad(&[0xC3]).1, ErrorKind::MalformedUtf8);
        assert_eq!(bad(&[0xE2, 0x98]).1, ErrorKind::MalformedUtf8);
        assert_eq!(bad(&[0xC3, 0x41]).1, ErrorKind::MalformedUtf8);
    }

    #[test]
    fn unpaired_surrogates_rejected() {
        // Lone high surrogate D83D at the end.
        assert_eq!(
            decode(&[b'a', 0xED, 0xA0, 0xBD]).unwrap_err(),
            (1, ErrorKind::UnpairedSurrogate(0xD83D))
        );
        // High followed by a non-surrogate.
        assert_eq!(
            decode(&[0xED, 0xA0, 0xBD, b'a']).unwrap_err(),
            (0, ErrorKind::UnpairedSurrogate(0xD83D))
        );
        // Lone low surrogate DE00.
        assert_eq!(
            decode(&[0xED, 0xB8, 0x80]).unwrap_err(),
            (0, ErrorKind::UnpairedSurrogate(0xDE00))
        );
    }
}
