//! `CodecMus` / `MusInputStream` (CodecMus.java, MusInputStream.java): the
//! `.mus` files of `resources/streaming/` are OGG Vorbis streams XORed with a
//! key stream seeded by the file name's `String.hashCode()`. `CodecMus` is
//! paulscode's `CodecJOrbis` reading through `MusInputStream`; here the
//! whole file is decoded to plain OGG bytes before kira sees it.

use crate::jfmt::string_hash_code;

/// The key stream (`MusInputStream`'s `hash`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MusDecoder {
    hash: i32,
}

impl MusDecoder {
    /// `new MusInputStream(codec, url, stream)`: `hash` is the hash code of
    /// the URL path's last segment.
    ///
    /// vanilla bug: the URL comes from `File.toURI().toURL()`, whose path is
    /// percent-encoded, so a file name with a space (`where are we
    /// now.mus`) is hashed as `where%20are%20we%20now.mus` and decodes to
    /// garbage (the files were encoded with the plain name). Pass
    /// [`url_file_name`] of the file name to reproduce it.
    // src: MusInputStream.java:14-21
    pub fn new(url_file_name: &str) -> Self {
        MusDecoder {
            hash: string_hash_code(url_file_name),
        }
    }

    /// `read(buf, off, len)`'s loop over the bytes just read: each byte is
    /// XORed with `hash >> 8`, then `hash = hash * 498729871 + 85731 *
    /// decoded` (the decoded byte signed). Chunking does not matter: the
    /// state carries over between calls.
    ///
    /// (`MusInputStream.read()`, the one-byte read, returns the signed byte,
    /// so a decoded `0xFF` reads as end of stream; JOrbis reads through the
    /// array form only, so that path is not ported.)
    // src: MusInputStream.java:28-41
    pub fn decode(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            // (byte)(var1[i] ^ this.hash >> 8): the low byte of the XOR.
            let var5 = (*b as i8 as i32 ^ (self.hash >> 8)) as i8;
            *b = var5 as u8;
            self.hash = self
                .hash
                .wrapping_mul(498729871)
                .wrapping_add(85731_i32.wrapping_mul(var5 as i32));
        }
    }
}

/// Decodes a whole `.mus` file (`file_name` is the plain file name, e.g.
/// `"13.mus"`; it is URL-encoded as vanilla does before hashing).
pub fn decode_mus(file_name: &str, mut bytes: Vec<u8>) -> Vec<u8> {
    MusDecoder::new(&url_file_name(file_name)).decode(&mut bytes);
    bytes
}

/// The last segment of `new File(dir, name).toURI().toURL().getPath()`:
/// `java.net.URI`'s quoting of a path. ASCII letters, digits and
/// `-_.!~*'()` `:@&=+$,` `;/` stay; every other ASCII character (space,
/// `%`, `"`, `#`, `?`, …) and every non-ASCII space or control character is
/// `%XX`-escaped (UTF-8, upper-case hex); other non-ASCII characters stay.
pub fn url_file_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        let keep = if c.is_ascii() {
            c.is_ascii_alphanumeric() || "-_.!~*'():@&=+$,;/".contains(c)
        } else {
            !(c.is_whitespace() || c.is_control())
        };
        if keep {
            out.push(c);
        } else {
            let mut buf = [0u8; 4];
            for b in c.encode_utf8(&mut buf).bytes() {
                out.push_str(&format!("%{b:02X}"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reverse of `decode` (the encoder Mojang used: the hash advances
    /// on the plain byte).
    fn encode(name: &str, plain: &[u8]) -> Vec<u8> {
        let mut hash = string_hash_code(name);
        plain
            .iter()
            .map(|&p| {
                let e = (p as i8 as i32 ^ (hash >> 8)) as u8;
                hash = hash
                    .wrapping_mul(498729871)
                    .wrapping_add(85731_i32.wrapping_mul(p as i8 as i32));
                e
            })
            .collect()
    }

    #[test]
    fn decodes_a_synthetic_buffer() {
        let plain: Vec<u8> = b"OggS\0\x02 synthetic payload \xff\x80\x7f\x00"
            .iter()
            .copied()
            .chain((0..=255u8).cycle().take(2000))
            .collect();
        let enc = encode("13.mus", &plain);
        assert_ne!(enc, plain);
        assert_eq!(decode_mus("13.mus", enc.clone()), plain);
        // in chunks, as the stream reads it
        let mut d = MusDecoder::new("13.mus");
        let mut buf = enc.clone();
        for chunk in buf.chunks_mut(7) {
            d.decode(chunk);
        }
        assert_eq!(buf, plain);
        // another name is another key
        assert_ne!(decode_mus("cat.mus", enc), plain);
    }

    /// The first bytes worked by hand from the Java expressions:
    /// `"a.mus".hashCode()` = 91_060_414 (0x056D_78BE); `hash >> 8` =
    /// 0x056D78, so the first byte is XORed with 0x78.
    #[test]
    fn first_byte_key() {
        assert_eq!(string_hash_code("a.mus"), 91_060_414);
        let mut b = [0x00u8, 0x00];
        MusDecoder::new("a.mus").decode(&mut b);
        assert_eq!(b[0], 0x78);
        // next: hash = 91060414 * 498729871 + 85731 * (byte)0x78 (= 120)
        let h = 91_060_414_i32
            .wrapping_mul(498_729_871)
            .wrapping_add(85731 * 120);
        assert_eq!(b[1], (h >> 8) as u8);
    }

    #[test]
    fn url_encoding_of_file_names() {
        assert_eq!(url_file_name("13.mus"), "13.mus");
        assert_eq!(
            url_file_name("where are we now.mus"),
            "where%20are%20we%20now.mus"
        );
        assert_eq!(url_file_name("a%b#c?.mus"), "a%25b%23c%3F.mus");
        assert_eq!(url_file_name("ü.mus"), "ü.mus");
    }

    /// Against the user's `resources/` when present (D8): `13.mus` decodes
    /// to an OGG stream; `where are we now.mus` does not (the vanilla bug).
    #[test]
    #[allow(clippy::disallowed_methods)]
    fn real_mus_files_when_present() {
        let dir = crate::paths::default_resources().join("streaming");
        let Ok(b) = std::fs::read(dir.join("13.mus")) else {
            return;
        };
        assert_eq!(&decode_mus("13.mus", b)[..4], b"OggS");
        if let Ok(b) = std::fs::read(dir.join("where are we now.mus")) {
            assert_ne!(&decode_mus("where are we now.mus", b.clone())[..4], b"OggS");
            let mut plain = b;
            MusDecoder::new("where are we now.mus").decode(&mut plain);
            assert_eq!(&plain[..4], b"OggS");
        }
    }
}
