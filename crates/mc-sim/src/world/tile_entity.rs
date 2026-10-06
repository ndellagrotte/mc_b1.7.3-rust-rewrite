//! `TileEntity` base (TileEntity.java) and how the port refers to tile
//! entities.
//!
//! Vanilla shares one `TileEntity` object between `World.loadedTileEntityList`
//! (World.java:19), the pending-add list `field_30900_E` (World.java:20) and
//! `Chunk.chunkTileEntityMap` (Chunk.java:22). The port stores each tile entity
//! once, in [`crate::world::World::tile_entities`], under a stable
//! [`TileEntityRef`] handle; the lists and the chunk map hold handles. Handles
//! are never reused. A tile entity is dropped from the store only where it
//! becomes unreachable from every list in vanilla (`World.updateEntities` and
//! `World.removeBlockTileEntity`).
//!
//! The seven concrete tile entities (furnace, chest, record player, dispenser,
//! sign, mob spawner, note block) are WP-1.8; their data goes in
//! [`TileEntityKindData`]. The moving piston's [`TileEntityPiston`] is piston
//! behaviour and is transcribed here with the pistons (WP-1.6), with
//! [`update_entity`], the per-class `updateEntity` dispatch that
//! `World.updateEntities` calls (WP-1.8 replaces its recorded arm).

use std::collections::BTreeMap;

use mc_nbt::Tag;

use super::World;
use crate::block::{piston_base, piston_moving};

/// A handle to a tile entity in [`crate::world::World::tile_entities`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TileEntityRef(pub u32);

/// Per-class data of a tile entity. The WP-1.8 kinds are still
/// [`TileEntityKindData::Untranscribed`].
#[derive(Clone, Debug, PartialEq)]
pub enum TileEntityKindData {
    /// A `TileEntity` whose subclass is not transcribed yet, with its Java
    /// class name.
    Untranscribed(&'static str),
    /// `TileEntityPiston`, the moving block's tile entity.
    Piston(TileEntityPiston),
}

/// `TileEntity`.
#[derive(Clone, Debug, PartialEq)]
pub struct TileEntity {
    // `worldObj` (TileEntity.java:8): implicit, the owning world.
    /// `xCoord`.
    pub x_coord: i32,
    /// `yCoord`.
    pub y_coord: i32,
    /// `zCoord`.
    pub z_coord: i32,
    /// `field_31007_h`: set by `invalidate()`, read by `isInvalid()`
    /// (TileEntity.java:88-98). Name kept; see `docs/mappings.md`.
    pub field_31007_h: bool,
    /// The subclass data.
    pub kind: TileEntityKindData,
}

impl TileEntity {
    /// `isInvalid()`.
    // src: TileEntity.java:88-90
    pub fn is_invalid(&self) -> bool {
        self.field_31007_h
    }

    /// `invalidate()`.
    // src: TileEntity.java:92-94
    pub fn invalidate(&mut self) {
        self.field_31007_h = true;
    }

    /// `validate()`.
    // src: TileEntity.java:96-98
    pub fn validate(&mut self) {
        self.field_31007_h = false;
    }

    /// `getDistanceFrom(x, y, z)` (squared).
    // src: TileEntity.java:77-82
    pub fn get_distance_from(&self, x: f64, y: f64, z: f64) -> f64 {
        let var7 = self.x_coord as f64 + 0.5 - x;
        let var9 = self.y_coord as f64 + 0.5 - y;
        let var11 = self.z_coord as f64 + 0.5 - z;
        var7 * var7 + var9 * var9 + var11 * var11
    }
}

// ---- TileEntityPiston (WP-1.6, pistons) -------------------------------------------
//
// The moving-block tile entity is piston behaviour, so it is transcribed with
// the pistons (`crate::block::piston_moving` creates it and reads it); the
// other kinds are WP-1.8. Field names: `docs/mappings.md`.

/// `TileEntityPiston`: the block a piston is moving, at the cell it moves
/// into, and how far it has moved. `xCoord`… are the [`TileEntity`]'s.
// src: TileEntityPiston.java:7-15
#[derive(Clone, Debug, PartialEq)]
pub struct TileEntityPiston {
    /// `storedBlockID`: the block id placed when the move ends.
    pub stored_block_id: i32,
    /// `storedMetadata`: its metadata.
    pub stored_metadata: i32,
    /// `field_31025_c`: the direction of the move (a `PistonBlockTextures`
    /// side; `getPistonOrientation`, NBT `"facing"`).
    pub stored_orientation: i32,
    /// `field_31024_i`: true while extending (`isExtending`, NBT
    /// `"extending"`).
    pub extending: bool,
    /// `field_31023_j`: the retracting piston base itself (the renderer then
    /// draws the head, TileEntityRendererPiston.java:29-34; `func_31012_k`).
    pub should_head_be_rendered: bool,
    /// `field_31022_k`: progress after this tick, 0 → 1 in steps of 0.5.
    pub progress: f32,
    /// `field_31020_l`: progress at the start of this tick (NBT `"progress"`).
    pub last_progress: f32,
}

impl TileEntityPiston {
    /// `new TileEntityPiston(storedBlockID, storedMetadata, orientation,
    /// extending, shouldHeadBeRendered)`; progress starts at 0.
    // src: TileEntityPiston.java:20-26
    pub fn new(
        stored_block_id: i32,
        stored_metadata: i32,
        stored_orientation: i32,
        extending: bool,
        should_head_be_rendered: bool,
    ) -> Self {
        TileEntityPiston {
            stored_block_id,
            stored_metadata,
            stored_orientation,
            extending,
            should_head_be_rendered,
            progress: 0.0,
            last_progress: 0.0,
        }
    }

    /// `getStoredBlockID()`.
    // src: TileEntityPiston.java:28-30
    pub fn get_stored_block_id(&self) -> i32 {
        self.stored_block_id
    }

    /// `getBlockMetadata()` (overrides `TileEntity.getBlockMetadata`): the
    /// stored metadata, not the world's.
    // src: TileEntityPiston.java:32-34
    pub fn get_block_metadata(&self) -> i32 {
        self.stored_metadata
    }

    /// `isExtending()`.
    // src: TileEntityPiston.java:36-38
    pub fn is_extending(&self) -> bool {
        self.extending
    }

    /// `getPistonOrientation()`.
    // src: TileEntityPiston.java:40-42
    pub fn get_piston_orientation(&self) -> i32 {
        self.stored_orientation
    }

    /// `func_31012_k()`.
    // src: TileEntityPiston.java:44-46
    pub fn should_head_be_rendered(&self) -> bool {
        self.should_head_be_rendered
    }

    /// `getProgress(partialTick)`: `last + (progress - last) * t`, `t`
    /// clamped to at most 1 (not below 0), all `float`.
    // src: TileEntityPiston.java:48-54
    pub fn get_progress(&self, mut var1: f32) -> f32 {
        if var1 > 1.0 {
            var1 = 1.0;
        }
        self.last_progress + (self.progress - self.last_progress) * var1
    }

    /// The renderer's offset along one axis (`func_31017_b` / `func_31014_c`
    /// / `func_31013_d`): `(progress - 1) * offset` while extending,
    /// `(1 - progress) * offset` while retracting.
    // src: TileEntityPiston.java:56-66
    fn offset(&self, var1: f32, axis: &[i32; 6]) -> f32 {
        let o = axis[self.stored_orientation as usize] as f32;
        if self.extending {
            (self.get_progress(var1) - 1.0) * o
        } else {
            (1.0 - self.get_progress(var1)) * o
        }
    }

    /// `func_31017_b(partialTick)`: x offset for the renderer.
    // src: TileEntityPiston.java:56-58
    pub fn get_offset_x(&self, var1: f32) -> f32 {
        self.offset(var1, &piston_base::OFFSETS_X_FOR_SIDE)
    }

    /// `func_31014_c(partialTick)`: y offset for the renderer.
    // src: TileEntityPiston.java:60-62
    pub fn get_offset_y(&self, var1: f32) -> f32 {
        self.offset(var1, &piston_base::OFFSETS_Y_FOR_SIDE)
    }

    /// `func_31013_d(partialTick)`: z offset for the renderer.
    // src: TileEntityPiston.java:64-66
    pub fn get_offset_z(&self, var1: f32) -> f32 {
        self.offset(var1, &piston_base::OFFSETS_Z_FOR_SIDE)
    }

    /// The subclass part of `readFromNBT` (`super.readFromNBT`, the
    /// `x`/`y`/`z` of the base, is the chunk loader's, WP-1.8/1.9). A
    /// missing or mistyped key reads as 0 / false (`NBTTagCompound.getInteger`
    /// etc. default to 0 for a missing key; Java would throw on a wrong type).
    // src: TileEntityPiston.java:128-135
    pub fn read_from_nbt(&mut self, c: &BTreeMap<String, Tag>) {
        let int = |k: &str| match c.get(k) {
            Some(Tag::Int(v)) => *v,
            _ => 0,
        };
        self.stored_block_id = int("blockId");
        self.stored_metadata = int("blockData");
        self.stored_orientation = int("facing");
        let p = match c.get("progress") {
            Some(Tag::Float(v)) => *v,
            _ => 0.0,
        };
        self.progress = p;
        self.last_progress = p;
        // `getBoolean` is `getByte(k) != 0` (NBTTagCompound.java).
        self.extending = matches!(c.get("extending"), Some(Tag::Byte(b)) if *b != 0);
    }

    /// The subclass part of `writeToNBT` (after `super.writeToNBT`'s `id`
    /// `"Piston"` and `x`/`y`/`z`, TileEntity.java:30-40, TileEntity.java:108).
    /// `"progress"` is `last_progress` (`field_31020_l`).
    // src: TileEntityPiston.java:137-144
    pub fn write_to_nbt(&self, c: &mut BTreeMap<String, Tag>) {
        c.insert("blockId".to_string(), Tag::Int(self.stored_block_id));
        c.insert("blockData".to_string(), Tag::Int(self.stored_metadata));
        c.insert("facing".to_string(), Tag::Int(self.stored_orientation));
        c.insert("progress".to_string(), Tag::Float(self.last_progress));
        // `setBoolean` writes a byte 1 / 0 (NBTTagCompound.java:83-85).
        c.insert("extending".to_string(), Tag::Byte(i8::from(self.extending)));
    }
}

/// The [`TileEntityPiston`] behind a handle, if the handle is live and is
/// one (`instanceof TileEntityPiston`).
pub fn piston(world: &World, r: TileEntityRef) -> Option<&TileEntityPiston> {
    match &world.tile_entities.get(&r)?.kind {
        TileEntityKindData::Piston(p) => Some(p),
        _ => None,
    }
}

/// Mutable [`piston`].
fn piston_mut(world: &mut World, r: TileEntityRef) -> Option<&mut TileEntityPiston> {
    match &mut world.tile_entities.get_mut(&r)?.kind {
        TileEntityKindData::Piston(p) => Some(p),
        _ => None,
    }
}

/// `xCoord`, `yCoord`, `zCoord` of a live tile entity.
fn coords(world: &World, r: TileEntityRef) -> Option<(i32, i32, i32)> {
    let t = world.tile_entities.get(&r)?;
    Some((t.x_coord, t.y_coord, t.z_coord))
}

/// `this.invalidate()`.
// src: TileEntity.java:92-94
fn invalidate(world: &mut World, r: TileEntityRef) {
    if let Some(t) = world.tile_entities.get_mut(&r) {
        t.invalidate();
    }
}

/// `func_31010_a(progress, step)` (private): pushes every entity in the
/// moving block's box `step` blocks along the move. The box is the stored
/// block's collision box at `progress - 1` (retracting) or `1 - progress`
/// (extending) of a block behind the cell
/// ([`piston_moving::get_axis_aligned_bb`]).
///
/// `field_31018_m` is a static scratch list: `addAll` of the query result,
/// one pass of `moveEntity`, `clear()`. Nothing `moveEntity` reaches calls
/// back into a `TileEntityPiston`, so the list is always empty on entry and
/// a local list is the same; the oracle resets it per scenario (census §6).
// src: TileEntityPiston.java:68-91
fn update_pushed_objects(world: &mut World, r: TileEntityRef, mut var1: f32, var2: f32) {
    let (Some(p), Some((x, y, z))) = (piston(world, r).cloned(), coords(world, r)) else {
        return;
    };
    // src: TileEntityPiston.java:69-73
    if !p.extending {
        var1 -= 1.0;
    } else {
        var1 = 1.0 - var1;
    }
    // src: TileEntityPiston.java:75
    let Some(var3) = piston_moving::get_axis_aligned_bb(
        world,
        x,
        y,
        z,
        p.stored_block_id,
        var1,
        p.stored_orientation,
    ) else {
        return;
    };
    // src: TileEntityPiston.java:77
    let var4 = world.get_entities_within_aabb_excluding_entity(None, &var3);
    if !var4.is_empty() {
        let f = p.stored_orientation as usize;
        // src: TileEntityPiston.java:84: `(double)(var2 * (float)offset)`
        let dx = (var2 * piston_base::OFFSETS_X_FOR_SIDE[f] as f32) as f64;
        let dy = (var2 * piston_base::OFFSETS_Y_FOR_SIDE[f] as f32) as f64;
        let dz = (var2 * piston_base::OFFSETS_Z_FOR_SIDE[f] as f32) as f64;
        // src: TileEntityPiston.java:79-87
        for id in var4 {
            world.with_entity_mut(id, |w, e| {
                crate::entity::dispatch::move_entity(e, w, dx, dy, dz);
            });
        }
    }
}

/// `clearPistonTileEntity()`: ends the move at once (another piston grabs
/// the cell, or the moving block is removed): progress to 1, the tile entity
/// removed and invalidated, and the stored block placed with notification
/// if the cell still holds the moving block.
// src: TileEntityPiston.java:93-103
pub fn clear_piston_tile_entity(world: &mut World, r: TileEntityRef) {
    let Some(p) = piston_mut(world, r) else {
        return;
    };
    if p.last_progress < 1.0 {
        // src: TileEntityPiston.java:95
        p.progress = 1.0;
        p.last_progress = 1.0;
        let (stored_id, stored_meta) = (p.stored_block_id, p.stored_metadata);
        let Some((x, y, z)) = coords(world, r) else {
            return;
        };
        // src: TileEntityPiston.java:96-100
        world.remove_block_tile_entity(x, y, z);
        invalidate(world, r);
        if world.get_block_id(x, y, z) == piston_moving::PISTON_MOVING_ID {
            world.set_block_and_metadata_with_notify(x, y, z, stored_id, stored_meta);
        }
    }
}

/// `TileEntityPiston.updateEntity()`: progress advances by 0.5 a tick
/// (pushing entities along while extending); on the tick after it reaches 1
/// the entities get a last push of 0.25, the tile entity is removed and the
/// stored block replaces the moving block.
// src: TileEntityPiston.java:105-126
pub fn piston_update_entity(world: &mut World, r: TileEntityRef) {
    let Some(p) = piston_mut(world, r) else {
        return;
    };
    // src: TileEntityPiston.java:106
    p.last_progress = p.progress;
    if p.last_progress >= 1.0 {
        // src: TileEntityPiston.java:108
        update_pushed_objects(world, r, 1.0, 0.25);
        let (Some((x, y, z)), Some(p)) = (coords(world, r), piston(world, r)) else {
            return;
        };
        let (stored_id, stored_meta) = (p.stored_block_id, p.stored_metadata);
        // src: TileEntityPiston.java:109-113
        world.remove_block_tile_entity(x, y, z);
        invalidate(world, r);
        if world.get_block_id(x, y, z) == piston_moving::PISTON_MOVING_ID {
            world.set_block_and_metadata_with_notify(x, y, z, stored_id, stored_meta);
        }
    } else {
        // src: TileEntityPiston.java:116-119
        p.progress += 0.5;
        if p.progress >= 1.0 {
            p.progress = 1.0;
        }
        // src: TileEntityPiston.java:121-123
        if p.extending {
            let progress = p.progress;
            let step = p.progress - p.last_progress + 0.0625;
            update_pushed_objects(world, r, progress, step);
        }
    }
}

/// `updateEntity()` of the tile entity behind `r`, by class. The base body
/// is empty (TileEntity.java:42-43); the WP-1.8 kinds record themselves.
pub fn update_entity(world: &mut World, r: TileEntityRef) {
    let Some(t) = world.tile_entities.get(&r) else {
        return;
    };
    match t.kind {
        TileEntityKindData::Untranscribed(_) => {
            world.note_unimplemented("TileEntity*.updateEntity");
        }
        // src: TileEntityPiston.java:105-126
        TileEntityKindData::Piston(_) => piston_update_entity(world, r),
    }
}
