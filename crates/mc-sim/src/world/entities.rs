//! The entity side of `World`: the loaded-entity list, spawning, the
//! take/put-back update driver, and entity/box queries.
//!
//! See [`crate::entity`] for the slot model. The invariant every function here
//! keeps: an entity is out of its slot only between a `take` and the matching
//! put-back in the same function, and nothing removes slots from
//! `loaded_entity_list` except [`World::update_entities`] /
//! [`World::update_entity_list`], at the places Java removes them.

use mc_core::Aabb;
use mc_core::math::floor_double;

use super::World;
use super::chunk;
use super::events::WorldEvent;
use crate::block::dispatch as block_dispatch;
use crate::entity::{Entity, EntitySlot, dispatch};

impl World {
    // ---- lookup ---------------------------------------------------------------------

    /// Index of entity `id` in `loadedEntityList`.
    pub fn entity_index(&self, id: i32) -> Option<usize> {
        self.loaded_entity_list
            .iter()
            .position(|s| s.entity_id == id)
    }

    /// The loaded entity `id`, unless absent or currently taken out.
    pub fn entity(&self, id: i32) -> Option<&Entity> {
        self.loaded_entity_list
            .iter()
            .find(|s| s.entity_id == id)
            .and_then(|s| s.entity.as_deref())
    }

    /// The loaded entity `id`, mutably, unless absent or taken out.
    pub fn entity_mut(&mut self, id: i32) -> Option<&mut Entity> {
        self.loaded_entity_list
            .iter_mut()
            .find(|s| s.entity_id == id)
            .and_then(|s| s.entity.as_deref_mut())
    }

    /// Java's `entity.isDead` for an entity held by reference, e.g.
    /// `ridingEntity.isDead`: true if it is dead or no longer in the list
    /// (it can only have left by dying or unloading), false if it is taken out
    /// for an update further up the stack (it is running, so not removed).
    pub fn is_entity_dead_or_gone(&self, id: i32) -> bool {
        match self.loaded_entity_list.iter().find(|s| s.entity_id == id) {
            None => {
                self.note_unimplemented("Entity ref to an entity no longer in loadedEntityList");
                true
            }
            Some(s) => s.entity.as_ref().is_some_and(|e| e.base.is_dead),
        }
    }

    /// Runs `f` on entity `id` taken out of its slot, with the world free for
    /// `f` to use, then puts the entity back. `None` if the entity is absent
    /// or already taken out (it is being updated further up the stack).
    pub fn with_entity_mut<R>(
        &mut self,
        id: i32,
        f: impl FnOnce(&mut World, &mut Entity) -> R,
    ) -> Option<R> {
        let idx = self.entity_index(id)?;
        let mut e = self.loaded_entity_list[idx].entity.take()?;
        let r = f(self, &mut e);
        self.put_back(e);
        Some(r)
    }

    /// Puts a taken entity back into its slot (found again by id: `f` may have
    /// appended entities, never removed any).
    fn put_back(&mut self, e: Box<Entity>) {
        let idx = self
            .entity_index(e.base.entity_id)
            .expect("an entity's slot disappeared while it was taken out");
        self.loaded_entity_list[idx].entity = Some(e);
    }

    // ---- joining and leaving ------------------------------------------------------------

    /// `entityJoinedWorld(entity)` (the b1.7.3 name of "spawn entity in
    /// world"): files the entity in its chunk and appends it to
    /// `loadedEntityList`. Returns false, dropping the entity, when its chunk
    /// is not loaded (players always join).
    // src: World.java:896-918
    pub fn spawn_entity_in_world(&mut self, mut e: Entity) -> bool {
        let var2 = floor_double(e.base.pos_x / 16.0);
        let var3 = floor_double(e.base.pos_z / 16.0);
        let var4 = e.kind.is_player();
        if !var4 && !self.chunk_exists(var2, var3) {
            return false;
        }
        if var4 {
            self.player_entities.push(e.base.entity_id);
            self.skip("World.updateAllPlayersSleepingFlag");
        }
        self.get_chunk_from_chunk_coords(var2, var3)
            .add_entity(&mut e.base);
        let id = e.base.entity_id;
        self.loaded_entity_list.push(EntitySlot {
            entity_id: id,
            entity: Some(Box::new(e)),
        });
        self.obtain_entity_skin(id);
        true
    }

    /// `obtainEntitySkin(entity)`: notifies the client, and applies the
    /// oracle's join-time reseed of `entity.rand` (`HarnessWorldAccess.obtainEntitySkin`
    /// reseeds with `mix(seed, entityId)`, PLAN_v4 §4.2).
    // src: World.java:920-925 (IWorldAccess.obtainEntitySkin)
    fn obtain_entity_skin(&mut self, id: i32) {
        let seed = self.entity_rand_seed_for(id);
        if let Some(e) = self.entity_mut(id) {
            e.base.rand.set_seed(seed);
        }
        self.events
            .push(WorldEvent::ObtainEntitySkin { entity_id: id });
    }

    /// `releaseEntitySkin(entity)`.
    // src: World.java:927-932 (IWorldAccess.releaseEntitySkin)
    fn release_entity_skin(&mut self, id: i32) {
        self.events
            .push(WorldEvent::ReleaseEntitySkin { entity_id: id });
    }

    /// `addWeatherEffect(entity)`.
    // src: World.java:891-894
    pub fn add_weather_effect(&mut self, e: Entity) -> bool {
        self.weather_effects.push(EntitySlot {
            entity_id: e.base.entity_id,
            entity: Some(Box::new(e)),
        });
        true
    }

    /// `setEntityDead(entity)`: dismounts and marks it dead; the list removal
    /// happens in the next `updateEntities`. `mountEntity(null)` on the rider
    /// is [`crate::entity::base::dismount`].
    // src: World.java:934-949
    pub fn set_entity_dead(&mut self, e: &mut Entity) {
        if let Some(rider) = e.base.ridden_by_entity {
            let mount_id = e.base.entity_id;
            // The mount is in hand (out of its slot): the rider must see it.
            let m = &mut e.base;
            self.with_entity_mut(rider, |w, r| {
                if r.base.riding_entity == Some(mount_id) {
                    crate::entity::base::dismount_from(r, m)
                } else {
                    crate::entity::base::dismount(r, w)
                }
            });
        }
        if e.base.riding_entity.is_some() {
            crate::entity::base::dismount(e, self);
        }
        dispatch::set_entity_dead(e, self);
        if e.kind.is_player() {
            let id = e.base.entity_id;
            if let Some(i) = self.player_entities.iter().position(|p| *p == id) {
                self.player_entities.remove(i);
            }
            self.skip("World.updateAllPlayersSleepingFlag");
        }
    }

    /// `addLoadedEntities(list)`: entities of a chunk being loaded (their data
    /// arrives with the chunk loader, WP-1.9; ids already present are
    /// ignored until then).
    // src: World.java:2077-2084
    pub fn add_loaded_entities(&mut self, ids: &[i32]) {
        for &id in ids {
            if self.entity_index(id).is_none() {
                self.skip("World.addLoadedEntities (chunk-stored entities)");
            }
        }
        for &id in ids {
            self.obtain_entity_skin(id);
        }
    }

    /// `unloadEntities(list)`.
    // src: World.java:2086-2088
    pub fn unload_entities(&mut self, ids: &[i32]) {
        self.unloaded_entity_list.extend_from_slice(ids);
    }

    // ---- the per-tick driver ---------------------------------------------------------------

    /// `loadedEntityList.removeAll(unloadedEntityList)` and the chunk
    /// bookkeeping that follows it (shared by `updateEntities` and
    /// `updateEntityList`). `removeAll` uses `Entity.equals` (same
    /// `entityId`) and keeps the order of the rest.
    // src: World.java:1189-1206, World.java:2318-2336
    fn remove_unloaded_entities(&mut self) {
        if self.unloaded_entity_list.is_empty() {
            return;
        }
        let unloaded = std::mem::take(&mut self.unloaded_entity_list);
        let mut removed: Vec<EntitySlot> = Vec::new();
        let mut i = 0;
        while i < self.loaded_entity_list.len() {
            if unloaded.contains(&self.loaded_entity_list[i].entity_id) {
                removed.push(self.loaded_entity_list.remove(i));
            } else {
                i += 1;
            }
        }
        for id in &unloaded {
            if let Some(e) = removed
                .iter()
                .find(|s| s.entity_id == *id)
                .and_then(|s| s.entity.as_deref())
            {
                let (x, y, z) = (
                    e.base.chunk_coord_x,
                    e.base.chunk_coord_y,
                    e.base.chunk_coord_z,
                );
                if e.base.added_to_chunk && self.chunk_exists(x, z) {
                    self.chunk_mut(x, z).remove_entity(*id, y);
                }
            }
        }
        for id in &unloaded {
            self.release_entity_skin(*id);
        }
        // unloadedEntityList.clear(): done by the take above.
    }

    /// `updateEntities()`: weather effects, unloads, every loaded entity in
    /// list order (removing the dead in place), then the tile entities and
    /// the tile entities added meanwhile.
    ///
    /// The take/put-back pattern: each entity is taken out of its slot for its
    /// own update ([`World::update_entity_with_optional_force`]) and is back in
    /// its slot before the next one runs. Java's `remove(var1--)` is
    /// `Vec::remove(i)` without advancing `i`.
    // src: World.java:1178-1276
    pub fn update_entities(&mut self) {
        // Weather effects (World.java:1181-1187).
        let mut i = 0;
        while i < self.weather_effects.len() {
            let mut e = self.weather_effects[i]
                .entity
                .take()
                .expect("weather effect taken twice");
            dispatch::on_update(&mut e, self);
            let dead = e.base.is_dead;
            self.weather_effects[i].entity = Some(e);
            if dead {
                self.weather_effects.remove(i);
            } else {
                i += 1;
            }
        }

        // World.java:1189-1206
        self.remove_unloaded_entities();

        // World.java:1208-1234
        let mut i = 0;
        while i < self.loaded_entity_list.len() {
            let id = self.loaded_entity_list[i].entity_id;
            let riding = self.slot_entity(i).base.riding_entity;
            if let Some(mount) = riding {
                let mount_ok = match self.entity(mount) {
                    Some(m) => !m.base.is_dead && m.base.ridden_by_entity == Some(id),
                    None => {
                        if self.entity_index(mount).is_none() {
                            self.note_unimplemented(
                                "Entity ref to an entity no longer in loadedEntityList",
                            );
                        }
                        false
                    }
                };
                if mount_ok {
                    i += 1;
                    continue;
                }
                if let Some(m) = self.entity_mut(mount) {
                    m.base.ridden_by_entity = None;
                }
                self.slot_entity_mut(i).base.riding_entity = None;
            }

            if !self.slot_entity(i).base.is_dead {
                self.update_entity(id);
            }

            let e = self.slot_entity(i);
            if e.base.is_dead {
                let (x, y, z, added) = (
                    e.base.chunk_coord_x,
                    e.base.chunk_coord_y,
                    e.base.chunk_coord_z,
                    e.base.added_to_chunk,
                );
                if added && self.chunk_exists(x, z) {
                    self.chunk_mut(x, z).remove_entity(id, y);
                }
                self.loaded_entity_list.remove(i);
                self.release_entity_skin(id);
            } else {
                i += 1;
            }
        }

        // Tile entities (World.java:1235-1253).
        self.field_31055_l = true;
        let mut i = 0;
        while i < self.loaded_tile_entity_list.len() {
            let r = self.loaded_tile_entity_list[i];
            if self.tile_entities.get(&r).is_some_and(|t| !t.is_invalid()) {
                self.update_tile_entity(r);
            }
            let invalid = self.tile_entities.get(&r).is_none_or(|t| t.is_invalid());
            if invalid {
                self.loaded_tile_entity_list.remove(i);
                if let Some(t) = self.tile_entities.get(&r) {
                    let (x, y, z) = (t.x_coord, t.y_coord, t.z_coord);
                    // `getChunkFromChunkCoords` (may load) then `removeChunkBlockTileEntity`.
                    self.get_chunk_from_chunk_coords(x >> 4, z >> 4);
                    chunk::remove_chunk_block_tile_entity(self, x >> 4, z >> 4, x & 15, y, z & 15);
                }
                self.tile_entities.remove(&r);
            } else {
                i += 1;
            }
        }
        self.field_31055_l = false;

        // Pending tile entities (World.java:1254-1274).
        if !self.field_30900_e.is_empty() {
            let pending = std::mem::take(&mut self.field_30900_e);
            for r in pending {
                let Some(t) = self.tile_entities.get(&r) else {
                    continue;
                };
                if t.is_invalid() {
                    continue;
                }
                let (x, y, z) = (t.x_coord, t.y_coord, t.z_coord);
                if !self.loaded_tile_entity_list.contains(&r) {
                    self.loaded_tile_entity_list.push(r);
                }
                self.get_chunk_from_chunk_coords(x >> 4, z >> 4);
                chunk::set_chunk_block_tile_entity(self, x >> 4, z >> 4, x & 15, y, z & 15, r);
                self.mark_block_needs_update(x, y, z);
            }
        }
    }

    /// `TileEntity.updateEntity()` dispatch. The base body is empty; the
    /// concrete tile entities are WP-1.8.
    // src: TileEntity.java:42-43
    fn update_tile_entity(&mut self, r: super::tile_entity::TileEntityRef) {
        if let Some(t) = self.tile_entities.get(&r) {
            match t.kind {
                super::tile_entity::TileEntityKindData::Untranscribed(class) => {
                    let _ = class;
                    self.note_unimplemented("TileEntity*.updateEntity");
                }
            }
        }
    }

    /// The entity in slot `i`; slots outside an update are never empty.
    fn slot_entity(&self, i: usize) -> &Entity {
        self.loaded_entity_list[i]
            .entity
            .as_deref()
            .expect("entity slot empty outside its update")
    }

    fn slot_entity_mut(&mut self, i: usize) -> &mut Entity {
        self.loaded_entity_list[i]
            .entity
            .as_deref_mut()
            .expect("entity slot empty outside its update")
    }

    /// `updateEntity(entity)`: `updateEntityWithOptionalForce(entity, true)`.
    // src: World.java:1287-1289
    pub fn update_entity(&mut self, id: i32) {
        self.update_entity_with_optional_force(id, true);
    }

    /// `updateEntityWithOptionalForce(entity, force)`: one entity's tick.
    ///
    /// Take/put-back: the entity is taken out for everything up to the chunk
    /// move and put back **before** its rider is updated, so the rider (taken
    /// out in turn) finds its mount in the world, as Java's rider code reads
    /// `ridingEntity`'s fields. Nothing runs between those two steps in Java,
    /// so the split is not observable.
    // src: World.java:1291-1355
    pub fn update_entity_with_optional_force(&mut self, id: i32, force: bool) {
        let Some(idx) = self.entity_index(id) else {
            return;
        };
        let Some(mut e) = self.loaded_entity_list[idx].entity.take() else {
            return;
        };
        let var3 = floor_double(e.base.pos_x);
        let var4 = floor_double(e.base.pos_z);
        let var5 = 32;
        if force
            && !self.check_chunks_exist(var3 - var5, 0, var4 - var5, var3 + var5, 128, var4 + var5)
        {
            self.put_back(e);
            return;
        }
        {
            let b = &mut e.base;
            b.last_tick_pos_x = b.pos_x;
            b.last_tick_pos_y = b.pos_y;
            b.last_tick_pos_z = b.pos_z;
            b.prev_rotation_yaw = b.rotation_yaw;
            b.prev_rotation_pitch = b.rotation_pitch;
        }
        if force && e.base.added_to_chunk {
            if e.base.riding_entity.is_some() {
                dispatch::update_ridden(&mut e, self);
            } else {
                dispatch::on_update(&mut e, self);
            }
        }
        {
            let b = &mut e.base;
            if b.pos_x.is_nan() || b.pos_x.is_infinite() {
                b.pos_x = b.last_tick_pos_x;
            }
            if b.pos_y.is_nan() || b.pos_y.is_infinite() {
                b.pos_y = b.last_tick_pos_y;
            }
            if b.pos_z.is_nan() || b.pos_z.is_infinite() {
                b.pos_z = b.last_tick_pos_z;
            }
            if (b.rotation_pitch as f64).is_nan() || (b.rotation_pitch as f64).is_infinite() {
                b.rotation_pitch = b.prev_rotation_pitch;
            }
            if (b.rotation_yaw as f64).is_nan() || (b.rotation_yaw as f64).is_infinite() {
                b.rotation_yaw = b.prev_rotation_yaw;
            }
        }
        let var6 = floor_double(e.base.pos_x / 16.0);
        let var7 = floor_double(e.base.pos_y / 16.0);
        let var8 = floor_double(e.base.pos_z / 16.0);
        let b = &mut e.base;
        if !b.added_to_chunk
            || b.chunk_coord_x != var6
            || b.chunk_coord_y != var7
            || b.chunk_coord_z != var8
        {
            if b.added_to_chunk && self.chunk_exists(b.chunk_coord_x, b.chunk_coord_z) {
                self.chunk_mut(b.chunk_coord_x, b.chunk_coord_z)
                    .remove_entity_at_index(b.entity_id, b.chunk_coord_y);
            }
            if self.chunk_exists(var6, var8) {
                b.added_to_chunk = true;
                self.chunk_mut(var6, var8).add_entity(b);
            } else {
                b.added_to_chunk = false;
            }
        }
        let (added, rider) = (e.base.added_to_chunk, e.base.ridden_by_entity);
        self.put_back(e);

        // World.java:1344-1351
        if force
            && added
            && let Some(rider) = rider
        {
            let rider_ok = self
                .entity(rider)
                .is_some_and(|r| !r.base.is_dead && r.base.riding_entity == Some(id));
            if rider_ok {
                self.update_entity(rider);
            } else {
                if let Some(r) = self.entity_mut(rider) {
                    r.base.riding_entity = None;
                }
                if let Some(m) = self.entity_mut(id) {
                    m.base.ridden_by_entity = None;
                }
            }
        }
    }

    /// `updateEntityList()`: the removal half of `updateEntities` without the
    /// updates (used by the client while paused).
    // src: World.java:2318-2363
    pub fn update_entity_list(&mut self) {
        self.remove_unloaded_entities();
        let mut i = 0;
        while i < self.loaded_entity_list.len() {
            let id = self.loaded_entity_list[i].entity_id;
            if let Some(mount) = self.slot_entity(i).base.riding_entity {
                let mount_ok = match self.entity(mount) {
                    Some(m) => !m.base.is_dead && m.base.ridden_by_entity == Some(id),
                    None => {
                        if self.entity_index(mount).is_none() {
                            self.note_unimplemented(
                                "Entity ref to an entity no longer in loadedEntityList",
                            );
                        }
                        false
                    }
                };
                if mount_ok {
                    i += 1;
                    continue;
                }
                if let Some(m) = self.entity_mut(mount) {
                    m.base.ridden_by_entity = None;
                }
                self.slot_entity_mut(i).base.riding_entity = None;
            }
            let e = self.slot_entity(i);
            if e.base.is_dead {
                let (x, y, z, added) = (
                    e.base.chunk_coord_x,
                    e.base.chunk_coord_y,
                    e.base.chunk_coord_z,
                    e.base.added_to_chunk,
                );
                if added && self.chunk_exists(x, z) {
                    self.chunk_mut(x, z).remove_entity(id, y);
                }
                self.loaded_entity_list.remove(i);
                self.release_entity_skin(id);
            } else {
                i += 1;
            }
        }
    }

    // ---- box queries ----------------------------------------------------------------------

    /// `getCollidingBoundingBoxes(entity, box)`: the block collision boxes in
    /// and just below the box, then the boxes of nearby entities. Vanilla
    /// returns a shared scratch list; this returns a new `Vec`.
    // src: World.java:959-997
    pub fn get_colliding_bounding_boxes(&mut self, e: &Entity, bb: &Aabb) -> Vec<Aabb> {
        let mut out = Vec::new();
        let var3 = floor_double(bb.min_x);
        let var4 = floor_double(bb.max_x + 1.0);
        let var5 = floor_double(bb.min_y);
        let var6 = floor_double(bb.max_y + 1.0);
        let var7 = floor_double(bb.min_z);
        let var8 = floor_double(bb.max_z + 1.0);
        for var9 in var3..var4 {
            for var10 in var7..var8 {
                if self.block_exists(var9, 64, var10) {
                    for var11 in var5 - 1..var6 {
                        let id = self.get_block_id(var9, var11, var10);
                        if crate::block::block(id).is_some() {
                            block_dispatch::get_colliding_bounding_boxes(
                                self, id, var9, var11, var10, bb, &mut out,
                            );
                        }
                    }
                }
            }
        }
        let var14 = 0.25;
        let var15 = self.get_entities_within_aabb_excluding_entity(
            Some(e.base.entity_id),
            &bb.expand(var14, var14, var14),
        );
        for other_id in var15 {
            let Some(other) = self.entity(other_id) else {
                continue;
            };
            if let Some(var13) = dispatch::get_bounding_box(other, self)
                && var13.intersects_with(bb)
            {
                out.push(var13);
            }
            if let Some(var13) = dispatch::get_collision_box(e, other, self)
                && var13.intersects_with(bb)
            {
                out.push(var13);
            }
        }
        out
    }

    /// `getEntitiesWithinAABBExcludingEntity(entity, box)`: ids of the loaded
    /// entities (other than `exclude`) whose box intersects `box`, scanning
    /// the chunks within 2 blocks, x outer, z inner, then each chunk's
    /// sections bottom-up, in list order. Entities taken out for an update are
    /// skipped (see [`crate::entity`]).
    // src: World.java:2013-2029, Chunk.java:488-510
    pub fn get_entities_within_aabb_excluding_entity(
        &self,
        exclude: Option<i32>,
        bb: &Aabb,
    ) -> Vec<i32> {
        let mut out = Vec::new();
        let var3 = floor_double((bb.min_x - 2.0) / 16.0);
        let var4 = floor_double((bb.max_x + 2.0) / 16.0);
        let var5 = floor_double((bb.min_z - 2.0) / 16.0);
        let var6 = floor_double((bb.max_z + 2.0) / 16.0);
        for var7 in var3..=var4 {
            for var8 in var5..=var6 {
                let Some(c) = self.loaded_chunk(var7, var8) else {
                    continue;
                };
                for section in c.entity_sections(bb.min_y, bb.max_y) {
                    for &id in &c.entities[section] {
                        if Some(id) == exclude {
                            continue;
                        }
                        match self.entity(id) {
                            Some(o) => {
                                if o.base.bounding_box.intersects_with(bb) {
                                    out.push(id);
                                }
                            }
                            None => {
                                if self.entity_index(id).is_none() {
                                    self.note_unimplemented(
                                        "Chunk entity list holds an entity not in loadedEntityList",
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        out
    }

    /// `getClosestPlayer(x, y, z, maxDistance)` (negative = unlimited).
    // src: World.java:2196-2210
    pub fn get_closest_player(&self, x: f64, y: f64, z: f64, max: f64) -> Option<i32> {
        let mut var9 = -1.0;
        let mut var11 = None;
        for &pid in &self.player_entities {
            let Some(p) = self.entity(pid) else { continue };
            let var14 = p.base.get_distance_sq(x, y, z);
            if (max < 0.0 || var14 < max * max) && (var9 == -1.0 || var14 < var9) {
                var9 = var14;
                var11 = Some(pid);
            }
        }
        var11
    }
}
