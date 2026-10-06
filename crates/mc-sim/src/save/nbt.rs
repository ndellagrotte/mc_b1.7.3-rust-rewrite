//! `NBTTagCompound`'s typed getters and setters (NBTTagCompound.java:43-133)
//! over [`mc_nbt::Tag`]: a missing key reads as the type's zero (`0`, `""`,
//! an empty array, a new empty compound or list); a present key of another
//! tag type is the Java cast's `ClassCastException`
//! ([`JavaException::ClassCast`]). `NBTTagList.tagAt` (NBTTagList.java:59-61)
//! is [`tag_at`].
//!
//! The setters build compounds in `BTreeMap` order; vanilla's `HashMap`
//! order only shows in the bytes, which are never compared (PLAN_v4 §4.3).

use std::collections::BTreeMap;

use mc_nbt::{Tag, TagType};

use super::SaveError;
use crate::item::JavaException;

/// A compound's children.
pub type Compound = BTreeMap<String, Tag>;

/// `(NBTTagCompound) tag`: the children, or `ClassCastException`.
pub fn as_compound(t: &Tag) -> Result<&Compound, JavaException> {
    t.as_compound().ok_or(JavaException::ClassCast)
}

/// `hasKey(key)`.
// src: NBTTagCompound.java:87-89
pub fn has_key(c: &Compound, key: &str) -> bool {
    c.contains_key(key)
}

macro_rules! getter {
    ($(#[$m:meta])* $name:ident, $variant:ident, $ty:ty, $zero:expr) => {
        $(#[$m])*
        pub fn $name(c: &Compound, key: &str) -> Result<$ty, JavaException> {
            match c.get(key) {
                None => Ok($zero),
                Some(Tag::$variant(v)) => Ok(*v),
                Some(_) => Err(JavaException::ClassCast),
            }
        }
    };
}

getter!(
    /// `getByte(key)`. src: NBTTagCompound.java:91-93
    get_byte, Byte, i8, 0
);
getter!(
    /// `getShort(key)`. src: NBTTagCompound.java:95-97
    get_short, Short, i16, 0
);
getter!(
    /// `getInteger(key)`. src: NBTTagCompound.java:99-101
    get_integer, Int, i32, 0
);
getter!(
    /// `getLong(key)`. src: NBTTagCompound.java:103-105
    get_long, Long, i64, 0
);
getter!(
    /// `getFloat(key)`. src: NBTTagCompound.java:107-109
    get_float, Float, f32, 0.0
);
getter!(
    /// `getDouble(key)`. src: NBTTagCompound.java:111-113
    get_double, Double, f64, 0.0
);
/// `getString(key)`.
// src: NBTTagCompound.java:115-117
pub fn get_string(c: &Compound, key: &str) -> Result<String, JavaException> {
    match c.get(key) {
        None => Ok(String::new()),
        Some(Tag::String(v)) => Ok(v.clone()),
        Some(_) => Err(JavaException::ClassCast),
    }
}

/// `getByteArray(key)`: `new byte[0]` when absent.
// src: NBTTagCompound.java:119-121
pub fn get_byte_array<'a>(c: &'a Compound, key: &str) -> Result<&'a [u8], JavaException> {
    match c.get(key) {
        None => Ok(&[]),
        Some(Tag::ByteArray(v)) => Ok(v),
        Some(_) => Err(JavaException::ClassCast),
    }
}

/// `getCompoundTag(key)`: a new empty compound when absent.
// src: NBTTagCompound.java:123-125
pub fn get_compound_tag(c: &Compound, key: &str) -> Result<Compound, JavaException> {
    match c.get(key) {
        None => Ok(Compound::new()),
        Some(Tag::Compound(m)) => Ok(m.clone()),
        Some(_) => Err(JavaException::ClassCast),
    }
}

/// `getCompoundTag(key)` without copying: `None` for an absent key (the Java
/// returns a fresh empty compound, which nothing then stores).
// src: NBTTagCompound.java:123-125
pub fn get_compound_tag_ref<'a>(
    c: &'a Compound,
    key: &str,
) -> Result<Option<&'a Compound>, JavaException> {
    match c.get(key) {
        None => Ok(None),
        Some(Tag::Compound(m)) => Ok(Some(m)),
        Some(_) => Err(JavaException::ClassCast),
    }
}

/// `getTagList(key)`: the items, empty when absent.
// src: NBTTagCompound.java:127-129
pub fn get_tag_list<'a>(c: &'a Compound, key: &str) -> Result<&'a [Tag], JavaException> {
    match c.get(key) {
        None => Ok(&[]),
        Some(Tag::List { items, .. }) => Ok(items),
        Some(_) => Err(JavaException::ClassCast),
    }
}

/// `getBoolean(key)`: `getByte(key) != 0`.
// src: NBTTagCompound.java:131-133
pub fn get_boolean(c: &Compound, key: &str) -> Result<bool, JavaException> {
    Ok(get_byte(c, key)? != 0)
}

/// `list.tagAt(i)`: `IndexOutOfBoundsException` past the end.
// src: NBTTagList.java:59-61
pub fn tag_at<'a>(items: &'a [Tag], i: usize, what: &str) -> Result<&'a Tag, SaveError> {
    items.get(i).ok_or_else(|| SaveError::Java {
        exception: "IndexOutOfBoundsException",
        detail: format!("{what}: index {i}, size {}", items.len()),
    })
}

/// `setBoolean(key, v)`: a byte 1 or 0.
// src: NBTTagCompound.java:83-85
pub fn set_boolean(c: &mut Compound, key: &str, v: bool) {
    c.insert(key.to_string(), Tag::Byte(if v { 1 } else { 0 }));
}

/// `c.setXxx(key, tag)`.
pub fn set(c: &mut Compound, key: &str, tag: Tag) {
    c.insert(key.to_string(), tag);
}

/// A `new NBTTagList()` filled with `setTag` calls of one element type: the
/// list's type is the first item's, `TAG_Byte` while empty (the type an
/// empty list is written with, NBTTagList.java:13-18).
// src: NBTTagList.java:50-53
pub fn list_of(elem: TagType, items: Vec<Tag>) -> Tag {
    let elem = if items.is_empty() {
        TagType::Byte
    } else {
        elem
    };
    Tag::List { elem, items }
}
