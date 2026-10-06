// Adapted from MinecraftOSS third_party/minecraftoss/core/src/nbt.rs @ 4013a68 by chasmlol, Apache-2.0; modified
//! The in-memory tree: [`Tag`] and [`TagType`].
//!
//! Vanilla keeps one Java class per tag id (`NBTTagByte` ... `NBTTagCompound`);
//! here a tag is a value of the [`Tag`] enum and its id is [`Tag::tag_type`].
//! A tag's *name* is not part of the value: compound children are keyed by
//! name in their map, list items are unnamed (vanilla writes no names for
//! them, NBTTagList.java:23-25), and the root's name travels next to the tree
//! ([`crate::read_named`], [`crate::write_named`]).

use std::collections::BTreeMap;
use std::fmt;

/// Tag ids defined by Beta 1.7.3 (`NBTBase.createTagOfType`,
/// NBTBase.java:45-72). Ids 11 (`TAG_Int_Array`) and 12 (`TAG_Long_Array`)
/// came with later versions and are rejected on read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum TagType {
    /// `NBTTagEnd`, id 0. Ends a compound; never a value. A list read from
    /// the stream may declare it as its element type when empty (later
    /// versions write empty lists that way).
    End = 0,
    /// `NBTTagByte`, id 1.
    Byte = 1,
    /// `NBTTagShort`, id 2.
    Short = 2,
    /// `NBTTagInt`, id 3.
    Int = 3,
    /// `NBTTagLong`, id 4.
    Long = 4,
    /// `NBTTagFloat`, id 5.
    Float = 5,
    /// `NBTTagDouble`, id 6.
    Double = 6,
    /// `NBTTagByteArray`, id 7.
    ByteArray = 7,
    /// `NBTTagString`, id 8.
    String = 8,
    /// `NBTTagList`, id 9.
    List = 9,
    /// `NBTTagCompound`, id 10.
    Compound = 10,
}

impl TagType {
    /// All Beta 1.7.3 tag types, in id order.
    pub const ALL: [Self; 11] = [
        Self::End,
        Self::Byte,
        Self::Short,
        Self::Int,
        Self::Long,
        Self::Float,
        Self::Double,
        Self::ByteArray,
        Self::String,
        Self::List,
        Self::Compound,
    ];

    /// The on-disk id byte.
    pub fn id(self) -> u8 {
        self as u8
    }

    /// The type for an id byte; `None` for anything outside 0–10
    /// (`createTagOfType` returns `null` there, NBTBase.java:69-70).
    pub fn from_id(id: u8) -> Option<Self> {
        Self::ALL.get(usize::from(id)).copied()
    }

    /// Vanilla's display name (`NBTBase.getTagName`, NBTBase.java:74-101).
    pub fn java_name(self) -> &'static str {
        match self {
            Self::End => "TAG_End",
            Self::Byte => "TAG_Byte",
            Self::Short => "TAG_Short",
            Self::Int => "TAG_Int",
            Self::Long => "TAG_Long",
            Self::Float => "TAG_Float",
            Self::Double => "TAG_Double",
            Self::ByteArray => "TAG_Byte_Array",
            Self::String => "TAG_String",
            Self::List => "TAG_List",
            Self::Compound => "TAG_Compound",
        }
    }
}

impl fmt::Display for TagType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.java_name(), self.id())
    }
}

/// One NBT value.
///
/// Equality ([`PartialEq`]/[`Eq`]) is structural with floats compared by
/// **bit pattern**, so `NaN == NaN` when the payloads match and `0.0 != -0.0`.
/// That is the "parsed trees equal" relation of PLAN_v4 §4.3 Tier S.
#[derive(Clone, Debug)]
pub enum Tag {
    /// `NBTTagByte.byteValue`.
    Byte(i8),
    /// `NBTTagShort.shortValue`.
    Short(i16),
    /// `NBTTagInt.intValue`.
    Int(i32),
    /// `NBTTagLong.longValue`.
    Long(i64),
    /// `NBTTagFloat.floatValue`. Any bit pattern is kept in memory; a NaN is
    /// written as `0x7fc00000` (see [`crate::write_named`]).
    Float(f32),
    /// `NBTTagDouble.doubleValue`. A NaN is written as `0x7ff8000000000000`.
    Double(f64),
    /// `NBTTagByteArray.byteArray`, as unsigned bytes.
    ByteArray(Vec<u8>),
    /// `NBTTagString.stringValue`.
    ///
    /// Java strings are UTF-16 and may hold unpaired surrogates; a Rust
    /// `String` cannot, so a stored string whose modified UTF-8 decodes to an
    /// unpaired surrogate is rejected on read
    /// ([`crate::ErrorKind::UnpairedSurrogate`]). U+0000 and supplementary
    /// characters (stored as surrogate pairs) round-trip exactly.
    String(String),
    /// `NBTTagList`: the element type and the items.
    ///
    /// Every item must have type `elem`; writing checks this. `elem` matters
    /// on its own only while the list is empty: a read keeps the type byte
    /// from the stream, while a write always emits `TAG_Byte` for an empty
    /// list (NBTTagList.java:14-18), so a declared type does not survive a
    /// round trip through bytes.
    List {
        /// Declared element type.
        elem: TagType,
        /// The items, in order.
        items: Vec<Tag>,
    },
    /// `NBTTagCompound`: children by name.
    ///
    /// Vanilla stores a `HashMap` and writes it in hash-bucket order
    /// (NBTTagCompound.java:12-23); this port keeps a `BTreeMap` and writes
    /// in key order (`str` byte order). Files therefore differ from vanilla's
    /// in child order only; both readers accept either.
    Compound(BTreeMap<String, Tag>),
}

impl Tag {
    /// The tag's type (vanilla `getType()`).
    pub fn tag_type(&self) -> TagType {
        match self {
            Self::Byte(_) => TagType::Byte,
            Self::Short(_) => TagType::Short,
            Self::Int(_) => TagType::Int,
            Self::Long(_) => TagType::Long,
            Self::Float(_) => TagType::Float,
            Self::Double(_) => TagType::Double,
            Self::ByteArray(_) => TagType::ByteArray,
            Self::String(_) => TagType::String,
            Self::List { .. } => TagType::List,
            Self::Compound(_) => TagType::Compound,
        }
    }

    /// An empty compound.
    pub fn compound() -> Self {
        Self::Compound(BTreeMap::new())
    }

    /// The child `key` of a compound; `None` for a missing key or a
    /// non-compound.
    pub fn get(&self, key: &str) -> Option<&Tag> {
        self.as_compound().and_then(|map| map.get(key))
    }

    /// The children, if this is a compound.
    pub fn as_compound(&self) -> Option<&BTreeMap<String, Tag>> {
        match self {
            Self::Compound(map) => Some(map),
            _ => None,
        }
    }

    /// The children, mutably, if this is a compound.
    pub fn as_compound_mut(&mut self) -> Option<&mut BTreeMap<String, Tag>> {
        match self {
            Self::Compound(map) => Some(map),
            _ => None,
        }
    }
}

impl PartialEq for Tag {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Byte(a), Self::Byte(b)) => a == b,
            (Self::Short(a), Self::Short(b)) => a == b,
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::Long(a), Self::Long(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => a.to_bits() == b.to_bits(),
            (Self::Double(a), Self::Double(b)) => a.to_bits() == b.to_bits(),
            (Self::ByteArray(a), Self::ByteArray(b)) => a == b,
            (Self::String(a), Self::String(b)) => a == b,
            (
                Self::List {
                    elem: ea,
                    items: ia,
                },
                Self::List {
                    elem: eb,
                    items: ib,
                },
            ) => ea == eb && ia == ib,
            (Self::Compound(a), Self::Compound(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for Tag {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_and_names() {
        for (i, ty) in TagType::ALL.iter().enumerate() {
            assert_eq!(usize::from(ty.id()), i);
            assert_eq!(TagType::from_id(ty.id()), Some(*ty));
        }
        assert_eq!(TagType::from_id(11), None);
        assert_eq!(TagType::from_id(12), None);
        assert_eq!(TagType::from_id(255), None);
        assert_eq!(TagType::ByteArray.java_name(), "TAG_Byte_Array");
        assert_eq!(TagType::List.to_string(), "TAG_List (9)");
    }

    #[test]
    fn equality_is_bitwise_for_floats() {
        assert_eq!(Tag::Float(f32::NAN), Tag::Float(f32::NAN));
        assert_ne!(Tag::Float(0.0), Tag::Float(-0.0));
        assert_ne!(
            Tag::Double(f64::from_bits(0x7ff8_0000_0000_0001)),
            Tag::Double(f64::NAN)
        );
        assert_ne!(Tag::Int(1), Tag::Long(1));
        let a = Tag::List {
            elem: TagType::Int,
            items: vec![],
        };
        let b = Tag::List {
            elem: TagType::Byte,
            items: vec![],
        };
        assert_ne!(a, b);
    }

    #[test]
    fn compound_accessors() {
        let mut t = Tag::compound();
        t.as_compound_mut()
            .unwrap()
            .insert("a".into(), Tag::Byte(1));
        assert_eq!(t.get("a"), Some(&Tag::Byte(1)));
        assert_eq!(t.get("b"), None);
        assert_eq!(Tag::Int(0).get("a"), None);
        assert_eq!(t.tag_type(), TagType::Compound);
    }
}
