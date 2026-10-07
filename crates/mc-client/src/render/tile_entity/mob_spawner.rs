//! `TileEntityMobSpawnerRenderer` (TileEntityMobSpawnerRenderer.java): a
//! small copy of the spawner's mob (scale 0.4375) turning inside the cage
//! (`yaw` × 10 degrees, tilted −30°), drawn by `RenderManager`.
//!
//! The mob is one entity per mob id, made once and kept
//! (`EntityList.createEntityInWorld(id, null)`). Vanilla's constructor
//! takes the next `Entity.nextEntityID` (a static shared with the world): the
//! first spawner drawn shifts every later entity id by one. Rendering must
//! not touch the simulation here, so the copy is made in a scratch world
//! and the world's ids are not shifted (a deviation, reported).

use std::collections::BTreeMap;

use mc_sim::entity::Entity;
use mc_sim::save::NoSave;
use mc_sim::save::entity_list;
use mc_sim::tile_entity::mob_spawner::TileEntityMobSpawner;
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::{World, WorldEnvironment};

use super::super::{Ctx, Renderers, render_entity_with_pos_yaw};

/// `TileEntityMobSpawnerRenderer`'s fields.
#[derive(Default)]
pub struct TileEntityMobSpawnerRenderer {
    /// `entityHashMap`: the mob by id (`None`: no such entity).
    pub entity_hash_map: BTreeMap<String, Option<Box<Entity>>>,
    /// The scratch world the mobs are made in (module docs).
    scratch: Option<Box<World>>,
}

impl TileEntityMobSpawnerRenderer {
    /// The spawner's mob, made on first use.
    fn entity(&mut self, id: &str) -> Option<&mut Entity> {
        if !self.entity_hash_map.contains_key(id) {
            let w = self.scratch.get_or_insert_with(|| {
                Box::new(World::new(
                    Box::new(NoSave),
                    "spawner",
                    WorldProvider::new(Dimension::Surface),
                    0,
                    ChunkGenerator::Empty(EmptyChunkProvider),
                    WorldEnvironment {
                        rand_seed: 0,
                        field_9437_g: 0,
                        lock_timestamp: 0,
                    },
                ))
            });
            let e = entity_list::create_entity_in_world(id, w).map(Box::new);
            self.entity_hash_map.insert(id.to_string(), e);
        }
        self.entity_hash_map.get_mut(id)?.as_deref_mut()
    }

    /// `renderTileEntityMobSpawner(spawner, x, y, z, partial)`.
    // src: TileEntityMobSpawnerRenderer.java:10-32
    #[allow(clippy::too_many_arguments)]
    pub fn render_tile_entity_mob_spawner(
        &mut self,
        r: &mut Renderers,
        cx: &mut Ctx,
        var1: &TileEntityMobSpawner,
        var2: f64,
        var4: f64,
        var6: f64,
        var8: f32,
    ) {
        cx.gl.push_matrix();
        cx.gl
            .translatef(var2 as f32 + 0.5, var4 as f32, var6 as f32 + 0.5);
        if let Some(var9) = self.entity(var1.get_mob_id()) {
            let var10 = 0.4375f32;
            cx.gl.translatef(0.0, 0.4, 0.0);
            cx.gl.rotatef(
                (var1.yaw2 + (var1.yaw - var1.yaw2) * var8 as f64) as f32 * 10.0,
                0.0,
                1.0,
                0.0,
            );
            cx.gl.rotatef(-30.0, 1.0, 0.0, 0.0);
            cx.gl.translatef(0.0, -0.4, 0.0);
            cx.gl.scalef(var10, var10, var10);
            var9.base
                .set_location_and_angles(var2, var4, var6, 0.0, 0.0);
            let e = var9.clone();
            render_entity_with_pos_yaw(r, cx, &e, 0.0, 0.0, 0.0, 0.0, var8);
        }
        cx.gl.pop_matrix();
    }
}
