//! Named Binary Tag reader and writer for Beta 1.7.3 saves.
//!
//! Transcribed from the vanilla tag classes (`NBTBase`, `NBTTag*`,
//! `CompressedStreamTools`) and adapted from MinecraftOSS `core/src/nbt.rs`
//! (PLAN_v4 D4; adapted files carry the attribution header).
//!
//! - [`Tag`] / [`TagType`]: the tree. Exactly the tag ids b1.7.3 defines,
//!   0 through 10. Compounds are `BTreeMap`s; equality compares floats by bit
//!   pattern.
//! - [`read_named`] / [`write_named`]: one named root compound, uncompressed
//!   (`CompressedStreamTools.read` / `write`).
//! - [`read_gzip`] / [`write_gzip`]: `level.dat` framing
//!   (CompressedStreamTools.java:14, :27). [`read_zlib`] / [`write_zlib`]:
//!   McRegion chunk framing, not yet covered by fixtures. The raw
//!   (de)compressors are public too.
//! - [`Error`] / [`ErrorKind`]: every failure, with the byte offset.
//!
//! Wire format (all `DataInput`/`DataOutput`, big-endian): a named tag is the
//! id byte, the name as a `u16`-length Java modified UTF-8 string, then the
//! payload. Lists are an element id byte, an `i32` count and bare payloads;
//! compounds are named tags closed by a `TAG_End` byte.
//!
//! Vanilla behaviour reproduced, as pinned by `tests/golden/nbt`:
//!
//! - an empty list is written with element type `TAG_Byte`
//!   (NBTTagList.java:17);
//! - every NaN is written as `0x7fc00000` / `0x7ff8000000000000`
//!   (`DataOutputStream.writeFloat`/`writeDouble`), while reading keeps the raw
//!   bits;
//! - strings are modified UTF-8: U+0000 as `c0 80`, supplementary characters as
//!   two three-byte surrogates;
//! - a negative list count reads as an empty list (vanilla quirk, kept);
//! - a repeated compound key keeps the last value; trailing bytes after the
//!   root are ignored.
//!
//! Deliberate differences from vanilla:
//!
//! - compound children are written in key order, not `HashMap` order, so bytes
//!   differ while parsed trees agree (PLAN_v4 §4.3 Tier S compares trees);
//! - tag ids 11 and 12 (later versions' int and long arrays) and any other id
//!   above 10 are errors ([`ErrorKind::PostBetaTagId`],
//!   [`ErrorKind::UnknownTagId`]), as is a non-empty list of `TAG_End`;
//! - strings must decode to valid Unicode ([`ErrorKind::UnpairedSurrogate`]);
//! - nesting is bounded by [`MAX_DEPTH`];
//! - writing refuses a list whose items do not all have its element type.
//!
//! Pure (PLAN_v4 D9): no `std::fs`, threads, clocks or environment
//! access, and no `unsafe`.

#![forbid(unsafe_code)]

mod codec;
mod compress;
mod error;
mod mutf8;
mod tag;

pub use codec::{MAX_DEPTH, read_named, write_named};
pub use compress::{
    compress_gzip, compress_zlib, decompress_gzip, decompress_zlib, read_gzip, read_zlib,
    write_gzip, write_zlib,
};
pub use error::{Error, ErrorKind, Result};
pub use tag::{Tag, TagType};
