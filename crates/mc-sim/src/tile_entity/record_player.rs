//! `TileEntityRecordPlayer` (TileEntityRecordPlayer.java): the jukebox's
//! record. Behaviour is in [`crate::block::jukebox`].

use std::collections::BTreeMap;

use mc_nbt::Tag;

use super::get_integer;
use crate::item::JavaException;

/// The fields `TileEntityRecordPlayer` declares.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TileEntityRecordPlayer {
    /// `record`: the item id of the record inside, 0 for none.
    // src: TileEntityRecordPlayer.java:4
    pub record: i32,
}

impl TileEntityRecordPlayer {
    /// `readFromNBT(compound)` after `super.readFromNBT`: `Record` (an int).
    // src: TileEntityRecordPlayer.java:6-9
    pub fn read_from_nbt(&mut self, c: &BTreeMap<String, Tag>) -> Result<(), JavaException> {
        self.record = get_integer(c, "Record")?;
        Ok(())
    }

    /// `writeToNBT(compound)` after `super.writeToNBT`: `Record`, only when
    /// a record is inside.
    // src: TileEntityRecordPlayer.java:11-17
    pub fn write_to_nbt(&self, c: &mut BTreeMap<String, Tag>) {
        if self.record > 0 {
            c.insert("Record".to_string(), Tag::Int(self.record));
        }
    }
}
