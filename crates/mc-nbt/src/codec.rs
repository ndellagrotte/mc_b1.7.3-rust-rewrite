// Adapted from MinecraftOSS third_party/minecraftoss/core/src/nbt.rs @ 4013a68 by chasmlol, Apache-2.0; modified
//! Uncompressed NBT: [`read_named`] and [`write_named`], transcribed from the
//! `readTagContents` / `writeTagContents` methods of the b1.7.3 tag classes.
//! All numbers are big-endian (`DataInput` / `DataOutput`).

use std::collections::BTreeMap;

use crate::{Error, ErrorKind, Result, Tag, TagType, mutf8};

/// Deepest list/compound nesting [`read_named`] accepts. Vanilla has no limit
/// (it recurses until `StackOverflowError`); a bound keeps a hostile file
/// from overflowing the stack here. Real saves nest a handful of levels.
pub const MAX_DEPTH: usize = 512;

/// Reads an uncompressed NBT document: one named root compound
/// (`CompressedStreamTools.read`, CompressedStreamTools.java:38-45, which is
/// `NBTBase.readTag`, NBTBase.java:25-35). Returns the root's name and tree.
///
/// Bytes after the root compound are ignored, as vanilla never reads them.
/// Tag ids 11 and 12 are rejected with [`ErrorKind::PostBetaTagId`].
pub fn read_named(bytes: &[u8]) -> Result<(String, Tag)> {
    let mut r = Reader {
        data: bytes,
        pos: 0,
    };
    let at = r.pos;
    let id = r.u8()?;
    // src: CompressedStreamTools.java:40-43 (checked after reading the whole
    // tag in Java; failing early gives the same outcome).
    if id != TagType::Compound.id() {
        return Err(Error::at(at, ErrorKind::RootNotCompound(id)));
    }
    let name = r.string()?; // src: NBTBase.java:31
    let root = r.payload(TagType::Compound, 0)?;
    Ok((name, root))
}

/// Writes an uncompressed NBT document: `tag`, which must be a compound, as
/// the root named `name` (`CompressedStreamTools.write`,
/// CompressedStreamTools.java:47-49 → `NBTBase.writeTag`, NBTBase.java:37-43).
///
/// Differences from vanilla's bytes, by design or by Java semantics:
///
/// - compound children are written in `BTreeMap` key order, vanilla's in
///   `HashMap` order (NBTTagCompound.java:14-23); the parsed trees agree;
/// - every NaN is written as `0x7fc00000` / `0x7ff8000000000000`, exactly as
///   vanilla (`DataOutputStream.writeFloat` / `writeDouble` go through
///   `floatToIntBits` / `doubleToLongBits`);
/// - an empty list is written with element type `TAG_Byte` whatever its
///   `elem`, exactly as vanilla (NBTTagList.java:14-18).
///
/// Errors: a non-compound root, a string over 65535 encoded bytes
/// (`writeUTF`'s limit), more than `i32::MAX` entries, or a list item whose
/// type is not the list's `elem` (vanilla would emit a corrupt stream).
pub fn write_named(name: &str, tag: &Tag) -> Result<Vec<u8>> {
    let Tag::Compound(_) = tag else {
        return Err(Error::at(
            0,
            ErrorKind::RootNotCompound(tag.tag_type().id()),
        ));
    };
    let mut w = Writer { out: Vec::new() };
    w.out.push(TagType::Compound.id()); // src: NBTBase.java:38
    w.string(name)?; // src: NBTBase.java:40
    w.payload(tag)?; // src: NBTBase.java:41
    Ok(w.out)
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let available = self.data.len() - self.pos;
        if n > available {
            return Err(Error::at(
                self.pos,
                ErrorKind::UnexpectedEof {
                    needed: n,
                    available,
                },
            ));
        }
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut a = [0; N];
        a.copy_from_slice(self.take(N)?);
        Ok(a)
    }

    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    fn i32(&mut self) -> Result<i32> {
        self.array().map(i32::from_be_bytes)
    }

    /// A tag id byte that must be one of 0–10.
    fn tag_type(&mut self) -> Result<TagType> {
        let at = self.pos;
        let id = self.u8()?;
        TagType::from_id(id).ok_or_else(|| {
            let kind = match id {
                11 | 12 => ErrorKind::PostBetaTagId(id),
                _ => ErrorKind::UnknownTagId(id),
            };
            Error::at(at, kind)
        })
    }

    /// `DataInput.readUTF`: `u16` length, then modified UTF-8.
    fn string(&mut self) -> Result<String> {
        let len = u16::from_be_bytes(self.array()?);
        let start = self.pos;
        let bytes = self.take(usize::from(len))?;
        mutf8::decode(bytes).map_err(|(offset, kind)| Error::at(start + offset, kind))
    }

    /// `readTagContents` of the tag class for `ty`.
    fn payload(&mut self, ty: TagType, depth: usize) -> Result<Tag> {
        Ok(match ty {
            // Only reachable through a list of `TAG_End`, refused in `list`.
            TagType::End => unreachable!("TAG_End has no payload"),
            TagType::Byte => Tag::Byte(i8::from_be_bytes(self.array()?)), // src: NBTTagByte.java:22
            TagType::Short => Tag::Short(i16::from_be_bytes(self.array()?)), // src: NBTTagShort.java:22
            TagType::Int => Tag::Int(self.i32()?), // src: NBTTagInt.java:22
            TagType::Long => Tag::Long(i64::from_be_bytes(self.array()?)), // src: NBTTagLong.java:22
            // `readFloat` = `intBitsToFloat(readInt())`: raw bits kept.
            TagType::Float => Tag::Float(f32::from_bits(u32::from_be_bytes(self.array()?))), // src: NBTTagFloat.java:22
            TagType::Double => Tag::Double(f64::from_bits(u64::from_be_bytes(self.array()?))), // src: NBTTagDouble.java:22
            TagType::ByteArray => {
                // src: NBTTagByteArray.java:23-25
                let at = self.pos;
                let len = self.i32()?;
                let len = usize::try_from(len)
                    .map_err(|_| Error::at(at, ErrorKind::NegativeLength(len)))?;
                Tag::ByteArray(self.take(len)?.to_vec())
            }
            TagType::String => Tag::String(self.string()?), // src: NBTTagString.java:25
            TagType::List => self.list(depth)?,
            TagType::Compound => self.compound(depth)?,
        })
    }

    fn enter(&self, depth: usize) -> Result<()> {
        if depth >= MAX_DEPTH {
            return Err(Error::at(self.pos, ErrorKind::TooDeep));
        }
        Ok(())
    }

    /// src: NBTTagList.java:29-40
    fn list(&mut self, depth: usize) -> Result<Tag> {
        self.enter(depth)?;
        let elem = self.tag_type()?;
        let at = self.pos;
        let count = self.i32()?;
        // Vanilla quirk, transcribed: a negative count runs the loop zero
        // times (NBTTagList.java:34), giving an empty list, not an error.
        let n = usize::try_from(count).unwrap_or(0);
        if n > 0 && elem == TagType::End {
            return Err(Error::at(at, ErrorKind::EndListNotEmpty { count }));
        }
        // Every payload is at least one byte, so `remaining` bounds the count.
        let mut items = Vec::with_capacity(n.min(self.data.len() - self.pos));
        for _ in 0..n {
            items.push(self.payload(elem, depth + 1)?);
        }
        Ok(Tag::List { elem, items })
    }

    /// src: NBTTagCompound.java:25-33 (`readTag` per child, NBTBase.java:25-35)
    fn compound(&mut self, depth: usize) -> Result<Tag> {
        self.enter(depth)?;
        let mut map = BTreeMap::new();
        loop {
            let ty = self.tag_type()?;
            if ty == TagType::End {
                break;
            }
            let name = self.string()?;
            let value = self.payload(ty, depth + 1)?;
            // `HashMap.put`: a repeated name keeps the last value.
            map.insert(name, value);
        }
        Ok(Tag::Compound(map))
    }
}

struct Writer {
    out: Vec<u8>,
}

impl Writer {
    fn error(&self, kind: ErrorKind) -> Error {
        Error::at(self.out.len(), kind)
    }

    /// `DataOutput.writeUTF`.
    fn string(&mut self, s: &str) -> Result<()> {
        let len = mutf8::encoded_len(s);
        if len > mutf8::MAX_LEN {
            return Err(self.error(ErrorKind::StringTooLong(len)));
        }
        self.out.extend_from_slice(&(len as u16).to_be_bytes());
        mutf8::encode_into(s, &mut self.out);
        Ok(())
    }

    /// A Java `int` length.
    fn len(&mut self, len: usize) -> Result<()> {
        let n = i32::try_from(len).map_err(|_| self.error(ErrorKind::LengthTooLarge(len)))?;
        self.out.extend_from_slice(&n.to_be_bytes());
        Ok(())
    }

    /// `writeTagContents` of the tag's class.
    fn payload(&mut self, tag: &Tag) -> Result<()> {
        match tag {
            Tag::Byte(v) => self.out.extend_from_slice(&v.to_be_bytes()), // src: NBTTagByte.java:18
            Tag::Short(v) => self.out.extend_from_slice(&v.to_be_bytes()), // src: NBTTagShort.java:18
            Tag::Int(v) => self.out.extend_from_slice(&v.to_be_bytes()),   // src: NBTTagInt.java:18
            Tag::Long(v) => self.out.extend_from_slice(&v.to_be_bytes()), // src: NBTTagLong.java:18
            Tag::Float(v) => {
                // src: NBTTagFloat.java:18; `writeFloat` uses
                // `Float.floatToIntBits`, which canonicalises every NaN.
                let bits = if v.is_nan() { 0x7fc0_0000 } else { v.to_bits() };
                self.out.extend_from_slice(&bits.to_be_bytes());
            }
            Tag::Double(v) => {
                // src: NBTTagDouble.java:18; `Double.doubleToLongBits`.
                let bits = if v.is_nan() {
                    0x7ff8_0000_0000_0000
                } else {
                    v.to_bits()
                };
                self.out.extend_from_slice(&bits.to_be_bytes());
            }
            Tag::ByteArray(bytes) => {
                // src: NBTTagByteArray.java:18-19
                self.len(bytes.len())?;
                self.out.extend_from_slice(bytes);
            }
            Tag::String(s) => self.string(s)?, // src: NBTTagString.java:21
            Tag::List { elem, items } => {
                // src: NBTTagList.java:13-27. Vanilla takes the type of the
                // first item, or 1 (`TAG_Byte`) for an empty list.
                let written = if items.is_empty() {
                    TagType::Byte
                } else {
                    *elem
                };
                if let Some((index, item)) = items
                    .iter()
                    .enumerate()
                    .find(|(_, item)| item.tag_type() != *elem)
                {
                    return Err(self.error(ErrorKind::ListItemType {
                        index,
                        expected: *elem,
                        found: item.tag_type(),
                    }));
                }
                self.out.push(written.id());
                self.len(items.len())?;
                for item in items {
                    self.payload(item)?;
                }
            }
            Tag::Compound(map) => {
                // src: NBTTagCompound.java:14-23 (`writeTag` per child,
                // NBTBase.java:37-43), in key order instead of HashMap order.
                for (name, value) in map {
                    self.out.push(value.tag_type().id());
                    self.string(name)?;
                    self.payload(value)?;
                }
                self.out.push(TagType::End.id());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(children: Vec<(&str, Tag)>) -> Tag {
        Tag::Compound(children.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }

    fn round_trip(tag: &Tag) -> Tag {
        let bytes = write_named("r", tag).unwrap();
        let (name, back) = read_named(&bytes).unwrap();
        assert_eq!(name, "r");
        back
    }

    #[test]
    fn every_primitive_round_trips() {
        let t = root(vec![
            ("b", Tag::Byte(i8::MIN)),
            ("b2", Tag::Byte(i8::MAX)),
            ("s", Tag::Short(i16::MIN)),
            ("i", Tag::Int(i32::MIN)),
            ("l", Tag::Long(i64::MIN)),
            ("f", Tag::Float(-0.0)),
            ("f2", Tag::Float(f32::from_bits(1))),
            ("f3", Tag::Float(f32::INFINITY)),
            ("d", Tag::Double(std::f64::consts::PI)),
            ("d2", Tag::Double(f64::NEG_INFINITY)),
            ("ba", Tag::ByteArray((0..=255).collect())),
            ("ba0", Tag::ByteArray(vec![])),
            ("str", Tag::String("nul\0smp\u{1F600}end ÄÖÜ".into())),
            ("str0", Tag::String(String::new())),
            (
                "list",
                Tag::List {
                    elem: TagType::Short,
                    items: vec![Tag::Short(1), Tag::Short(-1)],
                },
            ),
            ("c", root(vec![("x", root(vec![]))])),
        ]);
        assert_eq!(round_trip(&t), t);
    }

    #[test]
    fn exact_bytes_of_a_small_document() {
        let t = root(vec![
            ("a", Tag::Byte(1)),
            ("s", Tag::String("hi".into())),
            (
                "l",
                Tag::List {
                    elem: TagType::Int,
                    items: vec![Tag::Int(1), Tag::Int(2)],
                },
            ),
        ]);
        let mut expected = vec![10, 0, 1, b'r'];
        expected.extend([1, 0, 1, b'a', 1]);
        expected.extend([9, 0, 1, b'l', 3, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 2]);
        expected.extend([8, 0, 1, b's', 0, 2, b'h', b'i']);
        expected.push(0);
        assert_eq!(write_named("r", &t).unwrap(), expected);
    }

    #[test]
    fn nan_is_canonicalised_on_write_only() {
        let t = root(vec![
            ("f", Tag::Float(f32::from_bits(0xffc0_0001))),
            ("d", Tag::Double(f64::from_bits(0x7ff0_0000_0000_0001))),
        ]);
        let back = round_trip(&t);
        assert_eq!(
            back.get("f"),
            Some(&Tag::Float(f32::from_bits(0x7fc0_0000)))
        );
        assert_eq!(
            back.get("d"),
            Some(&Tag::Double(f64::from_bits(0x7ff8_0000_0000_0000)))
        );
        // Reading keeps raw payloads.
        let mut bytes = write_named("", &root(vec![("f", Tag::Float(0.0))])).unwrap();
        let n = bytes.len();
        bytes[n - 5..n - 1].copy_from_slice(&0x7f80_0001u32.to_be_bytes());
        let (_, raw) = read_named(&bytes).unwrap();
        assert_eq!(
            raw.get("f").map(|t| match t {
                Tag::Float(v) => v.to_bits(),
                _ => 0,
            }),
            Some(0x7f80_0001)
        );
    }

    #[test]
    fn empty_list_is_written_as_byte_list() {
        let t = root(vec![(
            "l",
            Tag::List {
                elem: TagType::Compound,
                items: vec![],
            },
        )]);
        let bytes = write_named("", &t).unwrap();
        assert_eq!(bytes, [10, 0, 0, 9, 0, 1, b'l', 1, 0, 0, 0, 0, 0]);
        assert_eq!(
            read_named(&bytes).unwrap().1.get("l"),
            Some(&Tag::List {
                elem: TagType::Byte,
                items: vec![]
            })
        );
    }

    #[test]
    fn empty_list_of_end_is_read() {
        let bytes = [10, 0, 0, 9, 0, 1, b'l', 0, 0, 0, 0, 0, 0];
        assert_eq!(
            read_named(&bytes).unwrap().1.get("l"),
            Some(&Tag::List {
                elem: TagType::End,
                items: vec![]
            })
        );
        let bytes = [10, 0, 0, 9, 0, 1, b'l', 0, 0, 0, 0, 2, 0];
        assert_eq!(
            read_named(&bytes).unwrap_err(),
            Error::at(8, ErrorKind::EndListNotEmpty { count: 2 })
        );
    }

    #[test]
    fn negative_list_count_reads_as_empty_like_vanilla() {
        let bytes = [10, 0, 0, 9, 0, 1, b'l', 3, 0xff, 0xff, 0xff, 0xff, 0];
        assert_eq!(
            read_named(&bytes).unwrap().1.get("l"),
            Some(&Tag::List {
                elem: TagType::Int,
                items: vec![]
            })
        );
    }

    #[test]
    fn negative_byte_array_length_rejected() {
        let bytes = [10, 0, 0, 7, 0, 1, b'a', 0xff, 0xff, 0xff, 0xfe, 0];
        assert_eq!(
            read_named(&bytes).unwrap_err(),
            Error::at(7, ErrorKind::NegativeLength(-2))
        );
    }

    #[test]
    fn post_beta_tag_ids_rejected() {
        for id in [11u8, 12] {
            // As a compound child.
            let bytes = [10, 0, 0, id, 0, 1, b'a', 0, 0, 0, 0, 0];
            let err = read_named(&bytes).unwrap_err();
            assert_eq!(err, Error::at(3, ErrorKind::PostBetaTagId(id)));
            let text = err.to_string();
            assert!(text.contains(&format!("tag id {id}")), "{text}");
            assert!(text.contains("Beta 1.7.3"), "{text}");
            // As a list element type, even for an empty list.
            let bytes = [10, 0, 0, 9, 0, 1, b'l', id, 0, 0, 0, 0, 0];
            assert_eq!(
                read_named(&bytes).unwrap_err(),
                Error::at(7, ErrorKind::PostBetaTagId(id))
            );
        }
        let bytes = [10, 0, 0, 13, 0, 0];
        assert_eq!(
            read_named(&bytes).unwrap_err(),
            Error::at(3, ErrorKind::UnknownTagId(13))
        );
    }

    #[test]
    fn root_must_be_compound() {
        assert_eq!(
            read_named(&[1, 0, 0, 5]).unwrap_err(),
            Error::at(0, ErrorKind::RootNotCompound(1))
        );
        assert_eq!(
            read_named(&[0]).unwrap_err().kind,
            ErrorKind::RootNotCompound(0)
        );
        assert_eq!(
            write_named("", &Tag::Int(1)).unwrap_err().kind,
            ErrorKind::RootNotCompound(3)
        );
    }

    #[test]
    fn truncated_input_rejected_with_position() {
        let full = write_named(
            "root",
            &root(vec![("i", Tag::Int(7)), ("s", Tag::String("abc".into()))]),
        )
        .unwrap();
        for cut in 0..full.len() {
            let err = read_named(&full[..cut]).unwrap_err();
            assert!(
                matches!(err.kind, ErrorKind::UnexpectedEof { .. }),
                "cut {cut}: {err}"
            );
            assert!(err.offset.unwrap() <= cut, "cut {cut}: {err}");
        }
        assert_eq!(
            read_named(&[10, 0, 0, 3, 0, 1, b'i', 0, 0]).unwrap_err(),
            Error::at(
                7,
                ErrorKind::UnexpectedEof {
                    needed: 4,
                    available: 2
                }
            )
        );
        // Trailing bytes after the root are ignored, as in vanilla.
        let mut more = full.clone();
        more.extend([1, 2, 3]);
        assert_eq!(read_named(&more).unwrap(), read_named(&full).unwrap());
    }

    #[test]
    fn bad_strings_rejected_with_position() {
        // Name "a" then a value string holding a lone surrogate.
        let bytes = [10, 0, 0, 8, 0, 1, b'a', 0, 4, b'x', 0xED, 0xA0, 0xBD, 0];
        assert_eq!(
            read_named(&bytes).unwrap_err(),
            Error::at(10, ErrorKind::UnpairedSurrogate(0xD83D))
        );
        let bytes = [10, 0, 0, 8, 0, 1, b'a', 0, 1, 0xff, 0];
        assert_eq!(
            read_named(&bytes).unwrap_err(),
            Error::at(9, ErrorKind::MalformedUtf8)
        );
    }

    #[test]
    fn duplicate_names_keep_the_last() {
        let bytes = [10, 0, 0, 1, 0, 1, b'a', 1, 1, 0, 1, b'a', 2, 0];
        assert_eq!(read_named(&bytes).unwrap().1.get("a"), Some(&Tag::Byte(2)));
    }

    #[test]
    fn depth_is_bounded() {
        let mut bytes = vec![10, 0, 0];
        for _ in 0..MAX_DEPTH + 1 {
            bytes.extend([10, 0, 0]);
        }
        let err = read_named(&bytes).unwrap_err();
        assert_eq!(err.kind, ErrorKind::TooDeep);
        let mut tree = Tag::compound();
        for _ in 0..MAX_DEPTH - 1 {
            tree = root(vec![("", tree)]);
        }
        assert_eq!(round_trip(&tree), tree);
    }

    #[test]
    fn write_limits() {
        let long = "\u{800}".repeat(21_846); // 65538 encoded bytes
        let err = write_named("", &root(vec![("s", Tag::String(long))])).unwrap_err();
        assert_eq!(err, Error::at(7, ErrorKind::StringTooLong(65_538)));
        let ok = "a".repeat(65_535);
        let t = root(vec![(ok.as_str(), Tag::String(ok.clone()))]);
        assert_eq!(round_trip(&t), t);
        let err = write_named(
            "",
            &root(vec![(
                "l",
                Tag::List {
                    elem: TagType::Int,
                    items: vec![Tag::Int(1), Tag::Byte(2)],
                },
            )]),
        )
        .unwrap_err();
        assert_eq!(
            err.kind,
            ErrorKind::ListItemType {
                index: 1,
                expected: TagType::Int,
                found: TagType::Byte
            }
        );
    }
}
