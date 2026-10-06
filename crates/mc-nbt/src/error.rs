//! The crate's error type: what went wrong ([`ErrorKind`]) and where
//! ([`Error::offset`]).

use std::fmt;

use crate::TagType;

/// `Result` with this crate's [`Error`].
pub type Result<T> = core::result::Result<T, Error>;

/// An NBT read, write or (de)compression failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    /// What went wrong.
    pub kind: ErrorKind,
    /// Byte offset of the offending item: into the *uncompressed* NBT stream
    /// for a read (the start of the tag id, length or string at fault), into
    /// the output produced so far for a write. `None` for errors of the
    /// gzip/zlib layer.
    pub offset: Option<usize>,
}

impl Error {
    pub(crate) fn at(offset: usize, kind: ErrorKind) -> Self {
        Self {
            kind,
            offset: Some(offset),
        }
    }

    pub(crate) fn compression(message: String) -> Self {
        Self {
            kind: ErrorKind::Compression(message),
            offset: None,
        }
    }
}

/// The kinds of [`Error`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// The input ended inside an item (Java: `EOFException`).
    UnexpectedEof {
        /// Bytes the item needed.
        needed: usize,
        /// Bytes that were left.
        available: usize,
    },
    /// Tag id 11 (`TAG_Int_Array`) or 12 (`TAG_Long_Array`): defined by later
    /// versions, not by Beta 1.7.3 (PLAN_v4 D4).
    PostBetaTagId(u8),
    /// Any other id outside 0–10 (vanilla's `createTagOfType` returns `null`,
    /// NBTBase.java:69-70, and the read fails with a `NullPointerException`).
    UnknownTagId(u8),
    /// The root tag is not a compound (`CompressedStreamTools.read`,
    /// CompressedStreamTools.java:40-43: "Root tag must be a named compound
    /// tag"). For a read the found id, for a write the given tag's type.
    RootNotCompound(u8),
    /// A byte array with a negative length (Java:
    /// `NegativeArraySizeException`, NBTTagByteArray.java:24).
    NegativeLength(i32),
    /// A list of `TAG_End` with items. Vanilla would build `NBTTagEnd`
    /// placeholders (NBTTagList.java:34-38); this port refuses them.
    EndListNotEmpty {
        /// The declared item count.
        count: i32,
    },
    /// Bytes that are not Java modified UTF-8 (Java:
    /// `UTFDataFormatException` from `DataInputStream.readUTF`).
    MalformedUtf8,
    /// Modified UTF-8 that decodes to an unpaired UTF-16 surrogate. Java
    /// accepts it; a Rust `String` cannot hold it.
    UnpairedSurrogate(u16),
    /// Lists and compounds nested deeper than [`crate::MAX_DEPTH`].
    TooDeep,
    /// A string whose modified UTF-8 form exceeds 65535 bytes (Java:
    /// `UTFDataFormatException`, "encoded string too long").
    StringTooLong(usize),
    /// A byte array or list with more than `i32::MAX` entries.
    LengthTooLarge(usize),
    /// A list item whose type differs from the list's `elem` (vanilla would
    /// write a corrupt stream, NBTTagList.java:14-25).
    ListItemType {
        /// Index of the item.
        index: usize,
        /// The list's declared element type.
        expected: TagType,
        /// The item's type.
        found: TagType,
    },
    /// The gzip or zlib layer failed (Java: `ZipException` and friends).
    Compression(String),
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof { needed, available } => write!(
                f,
                "unexpected end of NBT data: needed {needed} bytes, {available} left"
            ),
            Self::PostBetaTagId(id) => {
                let name = if *id == 11 {
                    "TAG_Int_Array"
                } else {
                    "TAG_Long_Array"
                };
                write!(f, "tag id {id} ({name}) is not defined in Beta 1.7.3")
            }
            Self::UnknownTagId(id) => write!(f, "unknown tag id {id}"),
            Self::RootNotCompound(id) => {
                write!(f, "root tag must be a named compound tag (found id {id})")
            }
            Self::NegativeLength(n) => write!(f, "negative byte array length {n}"),
            Self::EndListNotEmpty { count } => {
                write!(f, "list of TAG_End with {count} items")
            }
            Self::MalformedUtf8 => f.write_str("malformed modified UTF-8"),
            Self::UnpairedSurrogate(unit) => {
                write!(f, "string contains unpaired surrogate U+{unit:04X}")
            }
            Self::TooDeep => write!(f, "NBT nested deeper than {}", crate::MAX_DEPTH),
            Self::StringTooLong(len) => {
                write!(f, "encoded string too long: {len} bytes (max 65535)")
            }
            Self::LengthTooLarge(len) => {
                write!(f, "{len} entries do not fit a Java int length")
            }
            Self::ListItemType {
                index,
                expected,
                found,
            } => write!(
                f,
                "list item {index} is {found}, but the list holds {expected}"
            ),
            Self::Compression(message) => write!(f, "compression: {message}"),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.offset {
            Some(offset) => write!(f, "NBT error at byte {offset}: {}", self.kind),
            None => write!(f, "NBT error: {}", self.kind),
        }
    }
}

impl std::error::Error for Error {}
