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
//! sign, mob spawner, note block, plus the moving piston) are WP-1.8; their data
//! goes in [`TileEntityKindData`].

/// A handle to a tile entity in [`crate::world::World::tile_entities`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TileEntityRef(pub u32);

/// Per-class data of a tile entity. Placeholder until WP-1.8.
#[derive(Clone, Debug, PartialEq)]
pub enum TileEntityKindData {
    /// A `TileEntity` whose subclass is not transcribed yet, with its Java
    /// class name.
    Untranscribed(&'static str),
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
