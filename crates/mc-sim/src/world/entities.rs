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
use crate::entity::{Entity, EntityBase, EntityKindData, EntitySlot, dispatch};

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
    /// `ridingEntity.isDead`: false if it is taken out for an update further
    /// up the stack (it is running, so not removed); for one no longer in the
    /// list, the removed object's flag ([`World::removed_entity`]), or true
    /// (recorded) if the port no longer has it (it can only have left by
    /// dying or unloading).
    pub fn is_entity_dead_or_gone(&self, id: i32) -> bool {
        match self.loaded_entity_list.iter().find(|s| s.entity_id == id) {
            None => match self.removed_entities.get(&id) {
                Some(o) => o.base.is_dead,
                None => {
                    self.note_unimplemented(
                        "Entity ref to an entity no longer in loadedEntityList",
                    );
                    true
                }
            },
            Some(s) => s.entity.as_ref().is_some_and(|e| e.base.is_dead),
        }
    }

    /// The object Java still holds for entity `id` after it left
    /// `loadedEntityList`, as it was when it left (`World::removed_entities`):
    /// kept while a loaded entity's field names it, `None` otherwise or while
    /// `id` is in the list.
    pub fn removed_entity(&self, id: i32) -> Option<&Entity> {
        self.removed_entities.get(&id).map(|b| &**b)
    }

    /// [`World::removed_entity`], mutably: Java's calls on the orphaned
    /// object (e.g. `EntityLiving.onDeath`'s `attacker.onKillEntity`).
    pub fn removed_entity_mut(&mut self, id: i32) -> Option<&mut Entity> {
        self.removed_entities.get_mut(&id).map(|b| &mut **b)
    }

    /// `entity.riddenByEntity = null` on the object a field naming `id`
    /// refers to ([`World::entity_object`]): the loaded entity, else the
    /// removed one.
    fn clear_ridden_by(&mut self, id: i32) {
        if let Some(m) = self.entity_mut(id) {
            m.base.ridden_by_entity = None;
        } else if let Some(m) = self.removed_entity_mut(id) {
            m.base.ridden_by_entity = None;
        }
    }

    /// The object a Java field naming entity `id` refers to: the loaded
    /// entity in its slot, else the removed one ([`World::removed_entity`]).
    /// `None` if it is taken out (being updated further up the stack) or the
    /// port no longer has it.
    pub fn entity_object(&self, id: i32) -> Option<&Entity> {
        self.entity(id).or_else(|| self.removed_entity(id))
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
    /// A player's [`crate::entity::player::KillHook`]s that waited while it
    /// was out of its slot run first.
    fn put_back(&mut self, mut e: Box<Entity>) {
        crate::entity::player::take_pending_kill_hooks(&mut e, self);
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
        let id = e.base.entity_id;
        // An id names one object in the port: a new entity with the id of a
        // removed one (the respawned player keeps the dead player's id,
        // Minecraft.java:1451, :1468) replaces it.
        self.removed_entities.remove(&id);
        if var4 {
            self.player_entities.push(id);
        }
        self.get_chunk_from_chunk_coords(var2, var3)
            .add_entity(&mut e.base);
        self.loaded_entity_list.push(EntitySlot::new(e));
        if var4 {
            // src: World.java:910. Run after the list append (Java runs it
            // before) so the scan finds the new player in its slot; the scan
            // reads only `isPlayerSleeping`, so the order is not observable.
            self.update_all_players_sleeping_flag();
        }
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
        self.weather_effects.push(EntitySlot::new(e));
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
            // src: World.java:946 (the dead player has left playerEntities).
            self.update_all_players_sleeping_flag();
        }
    }

    /// `addLoadedEntities(list)`: the `obtainEntitySkin` half for the
    /// entities of a chunk being loaded. Their `loadedEntityList.addAll`
    /// half has already run: since WP-1.9b the chunk provider files the
    /// loaded entities in the list first (`World::prepare_chunk`, the
    /// comment there), so every id is present, and the `skip` record below
    /// is an invariant check that no fixture or game path reaches (Gate 1
    /// N4).
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
        for slot in removed {
            self.keep_removed_entity(slot);
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
                // `ridingEntity` is the object Java holds: in the list, or
                // the removed one while the rider names it
                // (World::removed_entities; a mount that died earlier in
                // this pass or the last one).
                let mount_ok = match self.entity_object(mount) {
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
                self.clear_ridden_by(mount);
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
                let slot = self.loaded_entity_list.remove(i);
                self.keep_removed_entity(slot);
                self.release_entity_skin(id);
            } else {
                i += 1;
            }
        }
        self.prune_removed_entities();

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

    /// `TileEntity.updateEntity()` dispatch ([`crate::tile_entity::update_entity`]).
    // src: World.java:1239, TileEntity.java:42-43
    fn update_tile_entity(&mut self, r: super::tile_entity::TileEntityRef) {
        crate::tile_entity::update_entity(self, r);
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
                // `ridingEntity` is the object Java holds: in the list, or
                // the removed one while the rider names it
                // (World::removed_entities; a mount that died earlier in
                // this pass or the last one).
                let mount_ok = match self.entity_object(mount) {
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
                self.clear_ridden_by(mount);
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
                let slot = self.loaded_entity_list.remove(i);
                self.keep_removed_entity(slot);
                self.release_entity_skin(id);
            } else {
                i += 1;
            }
        }
        self.prune_removed_entities();
    }

    // ---- removed entities still referenced ---------------------------------------------------

    /// Files an entity that just left `loadedEntityList` in
    /// `World::removed_entities` (kept only while referenced, see
    /// [`World::prune_removed_entities`]).
    fn keep_removed_entity(&mut self, slot: EntitySlot) {
        if let Some(e) = slot.entity {
            self.removed_entities.insert(slot.entity_id, e);
        }
    }

    /// Drops every removed entity that no loaded entity names any more in
    /// an entity field ([`referenced_entity_ids`]) and that is not in
    /// `playerEntities`: Java's object is then unreachable from the world.
    /// A player stays in `playerEntities` when it dies (only
    /// `World.setEntityDead` takes it out, World.java:934-945; the death
    /// runs `Entity.setEntityDead`), so the world keeps the dead player as
    /// vanilla's list does: `saveLevel` writes it to `level.dat`
    /// (World.java:295), `getClosestPlayer`, the random-tick chunks and the
    /// spawn chunks go on reading its position (World.java:1875-1886,
    /// :2196-2210; SpawnerAnimals.java:27-38). Runs after each removal
    /// pass, when every loaded entity is in its slot.
    fn prune_removed_entities(&mut self) {
        if self.removed_entities.is_empty() {
            return;
        }
        let mut refs = self.player_entities.clone();
        for slot in &self.loaded_entity_list {
            if let Some(e) = slot.entity.as_deref() {
                referenced_entity_ids(e, &mut refs);
            }
        }
        self.removed_entities.retain(|id, _| refs.contains(id));
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
    /// skipped (see [`crate::entity`]); one that is not `exclude` is recorded,
    /// because Java would have found it (BlockPressurePlate.java:75 passes a
    /// null `exclude` from inside the colliding entity's `moveEntity`).
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
                                } else {
                                    self.note_unimplemented(
                                        "World.getEntitiesWithinAABBExcludingEntity over a taken-out entity",
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
            // A dead player that left `loadedEntityList` is still in
            // `playerEntities` (World::prune_removed_entities).
            let Some(p) = self.entity_object(pid) else {
                continue;
            };
            let var14 = p.base.get_distance_sq(x, y, z);
            if (max < 0.0 || var14 < max * max) && (var9 == -1.0 || var14 < var9) {
                var9 = var14;
                var11 = Some(pid);
            }
        }
        var11
    }

    /// `getClosestPlayerToEntity(entity, maxDistance)`.
    // src: World.java:2192-2194
    pub fn get_closest_player_to_entity(&self, e: &EntityBase, max: f64) -> Option<i32> {
        self.get_closest_player(e.pos_x, e.pos_y, e.pos_z, max)
    }

    /// `getPlayerEntityByName(name)`: the first player whose `username`
    /// equals `name` (a null username never matches).
    // src: World.java:2212-2220
    pub fn get_player_entity_by_name(&self, name: &str) -> Option<i32> {
        self.player_entities.iter().copied().find(|&pid| {
            self.entity_object(pid)
                .and_then(|p| p.kind.player())
                .and_then(|p| p.username.as_deref())
                == Some(name)
        })
    }

    /// `getEntitiesWithinAABB(class, box)`: ids of the loaded entities that
    /// are instances of `class` ([`is_instance_of`]) and intersect `box`, in
    /// the chunk and section order of
    /// [`World::get_entities_within_aabb_excluding_entity`].
    ///
    /// Java's list still holds an entity that is running its update, and the
    /// query's vanilla callers run inside one (BlockPressurePlate and
    /// BlockDetectorRail reach it from `onEntityCollidedWithBlock` during
    /// `Entity.moveEntity`), so the entity standing on the plate must be found.
    /// The caller passes the entity it holds out of its slot as `in_hand`;
    /// when the scan reaches that id in a chunk section it tests `in_hand`
    /// there, keeping Java's chunk/section/list order. Any other taken-out
    /// entity the scan meets is skipped: silently when its class is not
    /// `class` (Java skips it on the class test,
    /// [`crate::entity::EntitySlot::class_chain`]), else recorded (Java would
    /// test its box; it is an entity further up the stack that the caller
    /// cannot reach).
    // src: World.java:2031-2047, Chunk.java:512-534
    pub fn get_entities_within_aabb(
        &self,
        class: &str,
        bb: &Aabb,
        in_hand: Option<&Entity>,
    ) -> Vec<i32> {
        let var3 = floor_double((bb.min_x - 2.0) / 16.0);
        let var4 = floor_double((bb.max_x + 2.0) / 16.0);
        let var5 = floor_double((bb.min_z - 2.0) / 16.0);
        let var6 = floor_double((bb.max_z + 2.0) / 16.0);
        let mut var7 = Vec::new();
        for var8 in var3..=var4 {
            for var9 in var5..=var6 {
                let Some(c) = self.loaded_chunk(var8, var9) else {
                    continue;
                };
                for section in c.entity_sections(bb.min_y, bb.max_y) {
                    for &id in &c.entities[section] {
                        let held = in_hand.filter(|h| h.base.entity_id == id);
                        let slot = self.loaded_entity_list.iter().find(|s| s.entity_id == id);
                        match held.or_else(|| slot.and_then(|s| s.entity.as_deref())) {
                            Some(o) => {
                                if is_instance_of(&o.kind, class)
                                    && o.base.bounding_box.intersects_with(bb)
                                {
                                    var7.push(id);
                                }
                            }
                            None => match slot {
                                None => self.note_unimplemented(
                                    "Chunk entity list holds an entity not in loadedEntityList",
                                ),
                                // Taken out further up the stack, of another
                                // class: Java's `isAssignableFrom` is false
                                // before the box is read, so it is skipped
                                // as Java skips it (EntitySlot::class_chain).
                                Some(s) if !s.class_chain.contains(&class) => {}
                                Some(_) => self.note_unimplemented(
                                    "World.getEntitiesWithinAABB over a taken-out entity",
                                ),
                            },
                        }
                    }
                }
            }
        }
        var7
    }

    /// `countEntities(class)`: loaded entities that are instances of `class`
    /// ([`is_instance_of`]).
    // src: World.java:2064-2075
    pub fn count_entities(&self, class: &str) -> i32 {
        let mut var2 = 0;
        for slot in &self.loaded_entity_list {
            match slot.entity.as_deref() {
                Some(e) => {
                    if is_instance_of(&e.kind, class) {
                        var2 += 1;
                    }
                }
                None => self.note_unimplemented("World.countEntities over a taken-out entity"),
            }
        }
        var2
    }

    /// `joinEntityInSurroundings(entity)`: loads the 5×5 chunks around the
    /// entity and appends it to `loadedEntityList` unless it is already there
    /// (`List.contains` uses `Entity.equals`, the entity id, Entity.java:121-127).
    /// The client calls it for the player every 30 ticks
    /// (Minecraft.java:1134-1139).
    ///
    /// The entity is named by id: the one in the list, or the removed object
    /// the world still holds ([`World::removed_entity`]), which is how the
    /// client's dead player comes back: it left the list 21 ticks after its
    /// death (EntityLiving.java:154-158, `Entity.setEntityDead`, so it stays
    /// in `playerEntities`), and Java's `thePlayer` is appended again, as
    /// it is (dead, `isDead`, not filed in a chunk, no
    /// `obtainEntitySkin`), and leaves again in the next `updateEntities`
    /// (World.java:1222-1232). An id the port no longer has is recorded.
    // src: World.java:2294-2309
    pub fn join_entity_in_surroundings(&mut self, id: i32) {
        let Some(e) = self.entity_object(id) else {
            self.note_unimplemented(
                "World.joinEntityInSurroundings of an entity the port no longer has",
            );
            return;
        };
        let var2 = floor_double(e.base.pos_x / 16.0);
        let var3 = floor_double(e.base.pos_z / 16.0);
        let var4 = 2;
        for var5 in var2 - var4..=var2 + var4 {
            for var6 in var3 - var4..=var3 + var4 {
                self.get_chunk_from_chunk_coords(var5, var6);
            }
        }
        // src: World.java:2305-2307 (`List.contains` by `Entity.equals`: the id)
        if self.entity_index(id).is_none()
            && let Some(e) = self.removed_entities.remove(&id)
        {
            self.loaded_entity_list.push(EntitySlot {
                entity_id: id,
                entity: Some(e),
            });
        }
    }

    /// `spawnPlayerWithLoadedChunks(player)`: applies the player NBT saved in
    /// `level.dat` (once, [`crate::entity::nbt::read_from_nbt`]), then
    /// `entityJoinedWorld`. The `ChunkProviderLoadOrGenerate` branch
    /// (World.java:264-269) never runs, because no world in the game builds
    /// that provider. The Java body is one `try`: an exception from
    /// `readFromNBT` is printed, the tag is kept and the player does not
    /// join (false here).
    // src: World.java:256-276
    pub fn spawn_player_with_loaded_chunks(&mut self, mut player: Entity) -> bool {
        if let Some(mc_nbt::Tag::Compound(var2)) = self.world_info.player_tag.clone() {
            if crate::entity::nbt::read_from_nbt(&mut player, &var2, self).is_err() {
                return false;
            }
            self.world_info.player_tag = None;
        }
        self.spawn_entity_in_world(player)
    }

    /// `updateAllPlayersSleepingFlag()`: true when there is a player and all
    /// players sleep.
    // src: World.java:2381-2393
    pub fn update_all_players_sleeping_flag(&mut self) {
        let mut flag = !self.player_entities.is_empty();
        for &pid in &self.player_entities {
            match self.entity_object(pid).and_then(|p| p.kind.player()) {
                Some(p) => {
                    if !p.is_player_sleeping() {
                        flag = false;
                        break;
                    }
                }
                None => {
                    self.note_unimplemented(
                        "World.updateAllPlayersSleepingFlag over a taken-out player",
                    );
                    flag = false;
                    break;
                }
            }
        }
        self.all_players_sleeping = flag;
    }
}

/// `class.isAssignableFrom(entity.getClass())` for the Java class or
/// interface named `class` (`"Entity"`, `"EntityLiving"`, `"EntityAnimal"`,
/// `"IMob"`, `"EntityPlayer"`, …): the kind's class chain from
/// [`EntityKindData::java_class`] up to `Entity`, plus the interfaces it
/// implements (EntityMob.java:3, EntitySlime.java:3, EntityGhast.java:3:
/// `IMob`; EntityMinecart.java:5: `IInventory`).
pub fn is_instance_of(kind: &EntityKindData, class: &str) -> bool {
    java_class_chain(kind).contains(&class)
}

/// The Java classes and interfaces an entity kind is an instance of (see
/// [`is_instance_of`]); the oracle's player is `HarnessPlayer extends
/// EntityPlayer`, the port's `EntityPlayerSP extends EntityPlayer`.
// src: the `extends` / `implements` clauses of each Entity*.java
pub fn java_class_chain(kind: &EntityKindData) -> &'static [&'static str] {
    use EntityKindData as K;
    match kind {
        K::Item(..) => &["EntityItem", "Entity"],
        K::Arrow(..) => &["EntityArrow", "Entity"],
        K::Snowball(..) => &["EntitySnowball", "Entity"],
        K::Egg(..) => &["EntityEgg", "Entity"],
        K::Fireball(..) => &["EntityFireball", "Entity"],
        K::Fish(..) => &["EntityFish", "Entity"],
        K::Painting(..) => &["EntityPainting", "Entity"],
        K::FallingSand(..) => &["EntityFallingSand", "Entity"],
        K::TntPrimed(..) => &["EntityTNTPrimed", "Entity"],
        K::Minecart(..) => &["EntityMinecart", "Entity", "IInventory"],
        K::Boat(..) => &["EntityBoat", "Entity"],
        K::LightningBolt(..) => &["EntityLightningBolt", "EntityWeatherEffect", "Entity"],
        K::Mob(..) => &[
            "EntityMob",
            "EntityCreature",
            "EntityLiving",
            "Entity",
            "IMob",
        ],
        K::Creeper(..) => &[
            "EntityCreeper",
            "EntityMob",
            "EntityCreature",
            "EntityLiving",
            "Entity",
            "IMob",
        ],
        K::Skeleton(..) => &[
            "EntitySkeleton",
            "EntityMob",
            "EntityCreature",
            "EntityLiving",
            "Entity",
            "IMob",
        ],
        K::Spider(..) => &[
            "EntitySpider",
            "EntityMob",
            "EntityCreature",
            "EntityLiving",
            "Entity",
            "IMob",
        ],
        K::Zombie(..) => &[
            "EntityZombie",
            "EntityMob",
            "EntityCreature",
            "EntityLiving",
            "Entity",
            "IMob",
        ],
        K::PigZombie(..) => &[
            "EntityPigZombie",
            "EntityZombie",
            "EntityMob",
            "EntityCreature",
            "EntityLiving",
            "Entity",
            "IMob",
        ],
        K::GiantZombie(..) => &[
            "EntityGiantZombie",
            "EntityMob",
            "EntityCreature",
            "EntityLiving",
            "Entity",
            "IMob",
        ],
        K::Slime(..) => &["EntitySlime", "EntityLiving", "Entity", "IMob"],
        K::Ghast(..) => &[
            "EntityGhast",
            "EntityFlying",
            "EntityLiving",
            "Entity",
            "IMob",
        ],
        K::Pig(..) => &[
            "EntityPig",
            "EntityAnimal",
            "EntityCreature",
            "EntityLiving",
            "Entity",
        ],
        K::Sheep(..) => &[
            "EntitySheep",
            "EntityAnimal",
            "EntityCreature",
            "EntityLiving",
            "Entity",
        ],
        K::Cow(..) => &[
            "EntityCow",
            "EntityAnimal",
            "EntityCreature",
            "EntityLiving",
            "Entity",
        ],
        K::Chicken(..) => &[
            "EntityChicken",
            "EntityAnimal",
            "EntityCreature",
            "EntityLiving",
            "Entity",
        ],
        K::Wolf(..) => &[
            "EntityWolf",
            "EntityAnimal",
            "EntityCreature",
            "EntityLiving",
            "Entity",
        ],
        K::Squid(..) => &[
            "EntitySquid",
            "EntityWaterMob",
            "EntityCreature",
            "EntityLiving",
            "Entity",
        ],
        K::PlayerSp(..) => &["EntityPlayerSP", "EntityPlayer", "EntityLiving", "Entity"],
    }
}

/// The entity ids `e` holds in its entity fields, i.e. the objects a Java
/// field of it would keep alive: `ridingEntity`, `riddenByEntity`,
/// `EntityLiving.currentTarget`, `EntityCreature.playerToAttack`,
/// `EntityGhast.targetedEntity`, `EntityPlayer.fishEntity`,
/// `EntityArrow.owner`, `EntitySnowball`/`EntityEgg.thrower`,
/// `EntityFireball.shootingEntity`, `EntityFish.angler` and `bobber`.
/// These are every entity-typed field of the simulation's entity classes
/// (the client's `EntityPickupFX` and `EntityRenderer` hold the others).
// src: Entity.java:11-12, EntityLiving.java:57, EntityCreature.java:5,
// EntityGhast.java:8, EntityPlayer.java:38, EntityArrow.java:14,
// EntitySnowball.java:12, EntityEgg.java:12, EntityFireball.java:12,
// EntityFish.java:12, :16
fn referenced_entity_ids(e: &Entity, out: &mut Vec<i32>) {
    use EntityKindData as K;
    out.extend(e.base.riding_entity);
    out.extend(e.base.ridden_by_entity);
    if let Some(l) = e.kind.living() {
        out.extend(l.current_target);
    }
    if let Some(c) = crate::entity::creature::creature(&e.kind) {
        out.extend(c.player_to_attack);
    }
    match &e.kind {
        K::Arrow(d) => out.extend(d.owner),
        K::Snowball(d) | K::Egg(d) => out.extend(d.thrower),
        K::Fireball(d) => out.extend(d.shooting_entity),
        K::Fish(d) => {
            out.extend(d.angler);
            out.extend(d.bobber);
        }
        K::Ghast(_, _, g) => out.extend(g.targeted_entity),
        K::PlayerSp(_, p, _) => out.extend(p.fish_entity),
        _ => {}
    }
}
