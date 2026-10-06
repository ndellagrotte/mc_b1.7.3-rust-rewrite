//! `TileEntityNote` (TileEntityNote.java): the note block's pitch. The
//! block is [`crate::block::note`].

use std::collections::BTreeMap;

use mc_nbt::Tag;

use super::get_byte;
use crate::block::Material;
use crate::item::JavaException;
use crate::world::World;
use crate::world::tile_entity::{TileEntityKindData, TileEntityRef};

/// The fields `TileEntityNote` declares.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TileEntityNote {
    /// `note`: the pitch, 0-24.
    // src: TileEntityNote.java:4
    pub note: i8,
    /// `previousRedstoneState`: whether the block was powered at the last
    /// redstone neighbour change (BlockNote.java:12-24).
    // src: TileEntityNote.java:5
    pub previous_redstone_state: bool,
}

impl TileEntityNote {
    /// `readFromNBT(compound)` after `super.readFromNBT`: `note` (a byte),
    /// clamped to 0..=24.
    // src: TileEntityNote.java:12-23
    pub fn read_from_nbt(&mut self, c: &BTreeMap<String, Tag>) -> Result<(), JavaException> {
        // src: TileEntityNote.java:15-21 (`< 0` then `> 24`)
        self.note = get_byte(c, "note")?.clamp(0, 24);
        Ok(())
    }

    /// `writeToNBT(compound)` after `super.writeToNBT`: `note`.
    // src: TileEntityNote.java:7-10
    pub fn write_to_nbt(&self, c: &mut BTreeMap<String, Tag>) {
        c.insert("note".to_string(), Tag::Byte(self.note));
    }
}

fn data(world: &mut World, r: TileEntityRef) -> Option<&mut TileEntityNote> {
    match &mut world.tile_entities.get_mut(&r)?.kind {
        TileEntityKindData::Note(d) => Some(d),
        _ => None,
    }
}

/// `changePitch()`: the next of the 25 notes, then `onInventoryChanged`.
// src: TileEntityNote.java:25-28
pub fn change_pitch(world: &mut World, r: TileEntityRef) {
    if let Some(d) = data(world, r) {
        d.note = ((d.note as i32 + 1) % 25) as i8;
    }
    super::on_inventory_changed(world, r);
}

/// `triggerNote(world, x, y, z)`: with air above, plays the note through
/// `playNoteAt`, the instrument chosen by the material below (1 rock, 2
/// sand, 3 glass, 4 wood, else 0).
// src: TileEntityNote.java:30-52
pub fn trigger_note(world: &mut World, r: TileEntityRef, x: i32, y: i32, z: i32) {
    if world.get_block_material(x, y + 1, z) == Material::Air {
        let var5 = world.get_block_material(x, y - 1, z);
        let mut var6 = 0;
        if var5 == Material::Rock {
            var6 = 1;
        }
        if var5 == Material::Sand {
            var6 = 2;
        }
        if var5 == Material::Glass {
            var6 = 3;
        }
        if var5 == Material::Wood {
            var6 = 4;
        }
        let note = data(world, r).map_or(0, |d| d.note);
        world.play_note_at(x, y, z, var6, note as i32);
    }
}
