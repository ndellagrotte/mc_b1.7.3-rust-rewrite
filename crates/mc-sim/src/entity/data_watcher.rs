//! `DataWatcher` (DataWatcher.java): an entity's small typed key/value store,
//! synchronised to clients in multiplayer and read locally for the entity
//! flags (`Entity.getEntityFlag`, Entity.java:1071-1083).
//!
//! The network encoding (`writeWatchableObjects`, `readWatchableObjects`) is
//! multiplayer-only and not ported (D3). The map is lookup-only, so a
//! `BTreeMap` (PLAN_v4 §3.4 rule 4).

use std::collections::BTreeMap;

use mc_core::java_hash::ChunkCoordinates;

use crate::item::ItemStack;

/// A watched value; the variant is the `dataTypes` id (DataWatcher.java:171-177).
#[derive(Clone, Debug, PartialEq)]
pub enum WatchValue {
    /// `Byte` (type 0).
    Byte(i8),
    /// `Short` (type 1).
    Short(i16),
    /// `Integer` (type 2).
    Int(i32),
    /// `Float` (type 3).
    Float(f32),
    /// `String` (type 4).
    String(String),
    /// `ItemStack` (type 5).
    ItemStack(ItemStack),
    /// `ChunkCoordinates` (type 6).
    ChunkCoordinates(ChunkCoordinates),
}

/// `WatchableObject` (WatchableObject.java), without the redundant id.
#[derive(Clone, Debug, PartialEq)]
pub struct WatchableObject {
    /// `watchedObject`.
    pub watched_object: WatchValue,
    /// `watching`: changed since the last network sync.
    pub watching: bool,
}

/// `DataWatcher`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DataWatcher {
    /// `watchedObjects`.
    pub watched_objects: BTreeMap<i32, WatchableObject>,
    /// `objectChanged`.
    pub object_changed: bool,
}

impl DataWatcher {
    /// `addObject(id, value)`. Vanilla throws on an id above 31 or a
    /// duplicate id; both are programming errors, so this panics likewise.
    // src: DataWatcher.java:17-29
    pub fn add_object(&mut self, id: i32, value: WatchValue) {
        assert!(id <= 31, "Data value id is too big with {id}! (Max is 31)");
        assert!(
            !self.watched_objects.contains_key(&id),
            "Duplicate id value for {id}!"
        );
        self.watched_objects.insert(
            id,
            WatchableObject {
                watched_object: value,
                watching: false,
            },
        );
    }

    /// `getWatchableObjectByte(id)`.
    // src: DataWatcher.java:31-33
    pub fn get_watchable_object_byte(&self, id: i32) -> i8 {
        match self.watched_objects.get(&id).map(|o| &o.watched_object) {
            Some(WatchValue::Byte(b)) => *b,
            other => panic!("DataWatcher {id}: expected Byte, found {other:?}"),
        }
    }

    /// `getWatchableObjectInt(id)`.
    // src: DataWatcher.java:35-37
    pub fn get_watchable_object_int(&self, id: i32) -> i32 {
        match self.watched_objects.get(&id).map(|o| &o.watched_object) {
            Some(WatchValue::Int(v)) => *v,
            other => panic!("DataWatcher {id}: expected Int, found {other:?}"),
        }
    }

    /// `updateObject(id, value)`: marks it changed if the value differs.
    // src: DataWatcher.java:43-51
    pub fn update_object(&mut self, id: i32, value: WatchValue) {
        let o = self
            .watched_objects
            .get_mut(&id)
            .unwrap_or_else(|| panic!("DataWatcher {id}: not added"));
        if o.watched_object != value {
            o.watched_object = value;
            o.watching = true;
            self.object_changed = true;
        }
    }
}
