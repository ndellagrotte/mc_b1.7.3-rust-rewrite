//! `BlockNote` (BlockNote.java): the note block. Its pitch lives in
//! `TileEntityNote` ([`crate::tile_entity::note`]).

use mc_core::math::jmath;

use super::dispatch;
use crate::entity::Entity;
use crate::tile_entity::note::{change_pitch, trigger_note};
use crate::world::World;
use crate::world::tile_entity::TileEntityKindData;

/// `onNeighborBlockChange(world, x, y, z, neighborId)`: when a power
/// source changed next to it, plays on the rising edge of
/// `isBlockGettingPowered` and remembers the state in the tile entity.
// src: BlockNote.java:12-25
pub fn on_neighbor_block_change(world: &mut World, x: i32, y: i32, z: i32, neighbor_id: i32) {
    if neighbor_id > 0 && dispatch::can_provide_power(world, neighbor_id) {
        let var6 = world.is_block_getting_powered(x, y, z);
        // A missing tile entity would throw in Java (unreachable for a placed block).
        let Some(r) = world.get_block_tile_entity(x, y, z) else {
            return;
        };
        let prev = match world.tile_entities.get(&r).map(|t| &t.kind) {
            Some(TileEntityKindData::Note(d)) => d.previous_redstone_state,
            _ => return,
        };
        if prev != var6 {
            if var6 {
                trigger_note(world, r, x, y, z);
            }
            if let Some(TileEntityKindData::Note(d)) =
                world.tile_entities.get_mut(&r).map(|t| &mut t.kind)
            {
                d.previous_redstone_state = var6;
            }
        }
    }
}

/// `blockActivated(world, x, y, z, player)`: next pitch, then play it;
/// true.
// src: BlockNote.java:27-36
pub fn block_activated(world: &mut World, x: i32, y: i32, z: i32, player: &mut Entity) -> bool {
    let _ = player;
    if !world.multiplayer_world {
        // A missing tile entity would throw in Java (unreachable for a placed block).
        if let Some(r) = world.get_block_tile_entity(x, y, z) {
            change_pitch(world, r);
            trigger_note(world, r, x, y, z);
        }
    }
    true
}

/// `onBlockClicked(world, x, y, z, player)`: plays the current pitch.
// src: BlockNote.java:38-43
pub fn on_block_clicked(world: &mut World, x: i32, y: i32, z: i32, player: &mut Entity) {
    let _ = player;
    if !world.multiplayer_world
        && let Some(r) = world.get_block_tile_entity(x, y, z)
    {
        trigger_note(world, r, x, y, z);
    }
}

/// `playBlock(world, x, y, z, instrument, note)`: the `note.<instrument>`
/// sound at volume 3 and pitch `2^((note - 12) / 12)` (`Math.pow`, then a
/// `float` cast), and a note particle whose colour is `note / 24`.
// src: BlockNote.java:49-70
pub fn play_block(world: &mut World, x: i32, y: i32, z: i32, var5: i32, var6: i32) {
    let var7 = jmath::pow(2.0, (var6 - 12) as f64 / 12.0) as f32;
    let var8 = match var5 {
        1 => "bd",
        2 => "snare",
        3 => "hat",
        4 => "bassattack",
        _ => "harp",
    };
    world.play_sound_effect(
        x as f64 + 0.5,
        y as f64 + 0.5,
        z as f64 + 0.5,
        &format!("note.{var8}"),
        3.0,
        var7,
    );
    world.spawn_particle(
        "note",
        x as f64 + 0.5,
        y as f64 + 1.2,
        z as f64 + 0.5,
        var6 as f64 / 24.0,
        0.0,
        0.0,
    );
}
