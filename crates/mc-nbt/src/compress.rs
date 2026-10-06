// Adapted from MinecraftOSS third_party/minecraftoss/core/src/nbt.rs @ 4013a68 by chasmlol, Apache-2.0; modified
//! Gzip and zlib framing around [`read_named`] / [`write_named`], via
//! `flate2` (pure-Rust `miniz_oxide` backend, so it builds for the browser).
//!
//! Compressed bytes are never compared with vanilla's (PLAN_v4 §4.3 Tier S):
//! `java.util.zip` and `miniz_oxide` produce different deflate streams for the
//! same input. Only the decompressed content matters.

use std::io::{Read, Write};

use flate2::Compression;
use flate2::read::{GzDecoder, ZlibDecoder};
use flate2::write::{GzEncoder, ZlibEncoder};

use crate::{Error, Result, Tag, read_named, write_named};

/// Reads a gzip-compressed NBT document, as vanilla reads `level.dat`
/// (`CompressedStreamTools.loadGzippedCompoundFromOutputStream`,
/// CompressedStreamTools.java:14-25). Unlike MinecraftOSS's reader, input
/// that is not gzip is an error, not parsed raw (Java: `ZipException`).
pub fn read_gzip(bytes: &[u8]) -> Result<(String, Tag)> {
    read_named(&decompress_gzip(bytes)?)
}

/// Writes a gzip-compressed NBT document
/// (`CompressedStreamTools.writeGzippedCompoundToOutputStream`,
/// CompressedStreamTools.java:27-36). Errors as [`write_named`].
pub fn write_gzip(name: &str, tag: &Tag) -> Result<Vec<u8>> {
    Ok(compress_gzip(&write_named(name, tag)?))
}

/// Reads a zlib-compressed NBT document, the form McRegion stores chunks in
/// (`RegionFile.getChunkDataInputStream` wraps an `InflaterInputStream`).
/// Not exercised by the WP-0.3 fixtures; covered by unit tests only.
pub fn read_zlib(bytes: &[u8]) -> Result<(String, Tag)> {
    read_named(&decompress_zlib(bytes)?)
}

/// Writes a zlib-compressed NBT document (`RegionFile`'s
/// `DeflaterOutputStream`). Not exercised by the WP-0.3 fixtures; covered by
/// unit tests only.
pub fn write_zlib(name: &str, tag: &Tag) -> Result<Vec<u8>> {
    Ok(compress_zlib(&write_named(name, tag)?))
}

/// Gzip-compresses `data` (default level 6, like `GZIPOutputStream`).
pub fn compress_gzip(data: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    // Writing into a `Vec` cannot fail.
    encoder.write_all(data).expect("in-memory write");
    encoder.finish().expect("in-memory write")
}

/// Decompresses one gzip member (CRC and length checked).
pub fn decompress_gzip(data: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    GzDecoder::new(data)
        .read_to_end(&mut out)
        .map_err(|e| Error::compression(format!("gzip: {e}")))?;
    Ok(out)
}

/// Zlib-compresses `data` (default level, like `DeflaterOutputStream`).
pub fn compress_zlib(data: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).expect("in-memory write");
    encoder.finish().expect("in-memory write")
}

/// Decompresses a zlib stream (Adler-32 checked).
pub fn decompress_zlib(data: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    ZlibDecoder::new(data)
        .read_to_end(&mut out)
        .map_err(|e| Error::compression(format!("zlib: {e}")))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ErrorKind, TagType};

    fn sample() -> Tag {
        let mut t = Tag::compound();
        let map = t.as_compound_mut().unwrap();
        map.insert("seed".into(), Tag::Long(-4_172_144_991_579_068_934));
        map.insert(
            "pos".into(),
            Tag::List {
                elem: TagType::Double,
                items: vec![Tag::Double(1.5), Tag::Double(-0.0)],
            },
        );
        t
    }

    #[test]
    fn gzip_round_trip() {
        let bytes = write_gzip("", &sample()).unwrap();
        assert_eq!(bytes[..2], [0x1f, 0x8b]);
        assert_eq!(read_gzip(&bytes).unwrap(), (String::new(), sample()));
        assert_eq!(
            decompress_gzip(&bytes).unwrap(),
            write_named("", &sample()).unwrap()
        );
    }

    #[test]
    fn zlib_round_trip() {
        let bytes = write_zlib("Level", &sample()).unwrap();
        assert_eq!(bytes[0], 0x78);
        assert_eq!(read_zlib(&bytes).unwrap(), ("Level".into(), sample()));
    }

    #[test]
    fn raw_input_is_not_gzip() {
        let raw = write_named("", &sample()).unwrap();
        let err = read_gzip(&raw).unwrap_err();
        assert!(matches!(err.kind, ErrorKind::Compression(_)), "{err}");
        assert_eq!(err.offset, None);
        assert!(read_zlib(&raw).is_err());
        let mut gz = compress_gzip(&raw);
        let n = gz.len();
        gz[n - 8] ^= 1; // corrupt the CRC
        assert!(read_gzip(&gz).is_err());
    }
}
