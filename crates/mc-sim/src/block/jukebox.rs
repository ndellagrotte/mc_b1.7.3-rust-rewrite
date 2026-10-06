//! `BlockJukeBox` (BlockJukeBox.java): plays the record in its
//! `TileEntityRecordPlayer` ([`crate::tile_entity::record_player`]);
//! metadata 1 while a record is inside.
//!
//! `ejectRecord` is MCP's name for the method that *inserts* a record
//! (ItemRecord.java:17 calls it; the name is kept), and `func_28038_b_`
//! ejects it ([`func_28038_b_`], name kept, see `docs/mappings.md`).
//!
//! `dropBlockAsItemWithChance` (BlockJukeBox.java:56-60) only adds a
//! multiplayer-client test in front of the base body, which tests it
//! again; it is the jukebox arm of
//! [`super::dispatch::drop_block_as_item_with_chance`].

use crate::entity::Entity;
use crate::item::ItemStack;
use crate::world::World;
use crate::world::tile_entity::{TileEntityKindData, TileEntityRef};

use super::container;

fn record_mut(world: &mut World, r: TileEntityRef) -> Option<&mut i32> {
    match &mut world.tile_entities.get_mut(&r)?.kind {
        TileEntityKindData::RecordPlayer(d) => Some(&mut d.record),
        _ => None,
    }
}

/// `blockActivated(world, x, y, z, player)`: with a record inside
/// (metadata != 0), ejects it; false when empty.
// src: BlockJukeBox.java:12-19
pub fn block_activated(world: &mut World, x: i32, y: i32, z: i32, player: &mut Entity) -> bool {
    let _ = player;
    if world.get_block_metadata(x, y, z) == 0 {
        false
    } else {
        func_28038_b_(world, x, y, z);
        true
    }
}

/// `ejectRecord(world, x, y, z, record)`: puts record `record` in (the
/// caller, `ItemRecord.onItemUse`, plays it), metadata 1.
// src: BlockJukeBox.java:21-28
pub fn eject_record(world: &mut World, x: i32, y: i32, z: i32, record: i32) {
    if !world.multiplayer_world {
        // A missing tile entity would throw in Java (unreachable for a placed block).
        let Some(r) = world.get_block_tile_entity(x, y, z) else {
            return;
        };
        if let Some(rec) = record_mut(world, r) {
            *rec = record;
        }
        crate::tile_entity::on_inventory_changed(world, r);
        world.set_block_metadata_with_notify(x, y, z, 1);
    }
}

/// `func_28038_b_(world, x, y, z)`: ejects the record: aux 1005, stops the
/// music, empties the tile entity, metadata 0, and drops the record as an
/// `EntityItem` above the block (three `World.rand` offsets, pick-up delay
/// 10).
// src: BlockJukeBox.java:30-49
pub fn func_28038_b_(world: &mut World, x: i32, y: i32, z: i32) {
    if !world.multiplayer_world {
        // A missing tile entity would throw in Java (unreachable for a placed block).
        let Some(r) = world.get_block_tile_entity(x, y, z) else {
            return;
        };
        let var6 = record_mut(world, r).map_or(0, |v| *v);
        if var6 != 0 {
            world.play_aux_sfx(1005, x, y, z, 0);
            world.play_record(None, x, y, z);
            if let Some(rec) = record_mut(world, r) {
                *rec = 0;
            }
            crate::tile_entity::on_inventory_changed(world, r);
            world.set_block_metadata_with_notify(x, y, z, 0);
            let var8 = 0.7f32;
            let var9 = (world.rand.next_float() * var8) as f64 + (1.0f32 - var8) as f64 * 0.5;
            let var11 =
                (world.rand.next_float() * var8) as f64 + (1.0f32 - var8) as f64 * 0.2 + 0.6;
            let var13 = (world.rand.next_float() * var8) as f64 + (1.0f32 - var8) as f64 * 0.5;
            let mut var15 = crate::entity::item::new_entity_item(
                world,
                x as f64 + var9,
                y as f64 + var11,
                z as f64 + var13,
                ItemStack::new(var6, 1, 0),
            );
            if let Some(d) = var15.kind.item_mut() {
                d.delay_before_can_pickup = 10;
            }
            world.spawn_entity_in_world(var15);
        }
    }
}

/// `onBlockRemoval(world, x, y, z)`: ejects the record, then
/// `BlockContainer.onBlockRemoval`.
// src: BlockJukeBox.java:51-54
pub fn on_block_removal(world: &mut World, x: i32, y: i32, z: i32) {
    func_28038_b_(world, x, y, z);
    container::on_block_removal(world, x, y, z);
}
