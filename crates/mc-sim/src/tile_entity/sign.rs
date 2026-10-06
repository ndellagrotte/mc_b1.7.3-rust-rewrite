//! `TileEntitySign` (TileEntitySign.java): four lines of text. Editing is
//! the client's `GuiEditSign`; the block is [`crate::block::sign`].

use std::collections::BTreeMap;

use mc_nbt::Tag;

use super::get_string;
use crate::item::JavaException;

/// The fields `TileEntitySign` declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TileEntitySign {
    /// `signText`: four lines.
    // src: TileEntitySign.java:4
    pub sign_text: [String; 4],
    /// `lineBeingEdited`: -1, or the line `GuiEditSign` is editing
    /// (GuiEditSign.java:100, :104; read by the sign renderer).
    // src: TileEntitySign.java:5
    pub line_being_edited: i32,
    /// `field_25062_c` (private): true when constructed, cleared by
    /// `readFromNBT`; nothing reads it. Name kept (`docs/mappings.md`).
    // src: TileEntitySign.java:6
    pub field_25062_c: bool,
}

impl Default for TileEntitySign {
    // src: TileEntitySign.java:4-6
    fn default() -> Self {
        TileEntitySign {
            sign_text: [String::new(), String::new(), String::new(), String::new()],
            line_being_edited: -1,
            field_25062_c: true,
        }
    }
}

/// `s.length() > 15 ? s.substring(0, 15) : s`, in UTF-16 code units as
/// Java counts them. A cut through a surrogate pair would leave Java an
/// unpaired surrogate, which a Rust `String` cannot hold; that half is
/// dropped (unreachable from vanilla's sign editor, which allows only
/// `ChatAllowedCharacters`, all in the basic plane).
// src: TileEntitySign.java:21-24
fn truncate_15(s: String) -> String {
    let mut units = 0;
    let mut end = s.len();
    for (i, ch) in s.char_indices() {
        let n = ch.len_utf16();
        if units + n > 15 {
            end = i;
            break;
        }
        units += n;
    }
    if units >= 15 || end < s.len() {
        s[..end].to_string()
    } else {
        s
    }
}

impl TileEntitySign {
    /// `readFromNBT(compound)` after `super.readFromNBT` (the caller clears
    /// `field_25062_c` first, TileEntitySign.java:17): `Text1`…`Text4`, each
    /// cut to 15 characters.
    // src: TileEntitySign.java:16-27
    pub fn read_from_nbt(&mut self, c: &BTreeMap<String, Tag>) -> Result<(), JavaException> {
        for var2 in 0..4 {
            let s = get_string(c, &format!("Text{}", var2 + 1))?;
            self.sign_text[var2] = truncate_15(s);
        }
        Ok(())
    }

    /// `writeToNBT(compound)` after `super.writeToNBT`: `Text1`…`Text4`.
    // src: TileEntitySign.java:8-14
    pub fn write_to_nbt(&self, c: &mut BTreeMap<String, Tag>) {
        for (i, line) in self.sign_text.iter().enumerate() {
            c.insert(format!("Text{}", i + 1), Tag::String(line.clone()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::truncate_15;

    #[test]
    fn substring_0_15_in_utf16_units() {
        assert_eq!(truncate_15("short".into()), "short");
        assert_eq!(truncate_15("0123456789abcdef".into()), "0123456789abcde");
        assert_eq!(truncate_15("0123456789abcde".into()), "0123456789abcde");
        // 14 units then a surrogate pair: the pair would straddle unit 15.
        assert_eq!(
            truncate_15("01234567890123\u{1F600}".into()),
            "01234567890123"
        );
        // A pair inside the first 15 units counts as two.
        assert_eq!(
            truncate_15("\u{1F600}0123456789abcdef".into()),
            "\u{1F600}0123456789abc"
        );
    }
}
