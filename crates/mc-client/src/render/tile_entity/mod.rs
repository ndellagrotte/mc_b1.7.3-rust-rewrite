//! `TileEntityRenderer` and `TileEntitySpecialRenderer`
//! (TileEntityRenderer.java, TileEntitySpecialRenderer.java): the special
//! renderers of the sign ([`sign`]), the mob spawner ([`mob_spawner`]) and
//! the moving piston ([`piston`]), called by `renderEntities` for the tile
//! entities of the built sections ([`crate::terrain::TerrainTileEntities`],
//! `RenderGlobal.tileEntities`) within 64 blocks, under
//! `glColor3f(world.getLightBrightness(x, y, z))`, at the block's position
//! relative to the view (`staticPlayerX/Y/Z`).
//!
//! The terrain hands over positions; the tile entity is looked up in its
//! chunk without loading anything (vanilla holds the objects, so one
//! removed from the world since the section was built is still drawn until
//! the rebuild; here it is not).

pub mod mob_spawner;
pub mod piston;
pub mod sign;

use bevy::math::DVec3;
use mc_core::java_hash::ChunkPosition;
use mc_sim::entity::Entity;
use mc_sim::world::World;
use mc_sim::world::tile_entity::{TileEntity, TileEntityKindData as T};

use super::{Ctx, Renderers};

/// `TileEntityRenderer.instance` (module docs).
#[derive(Default)]
pub struct TileEntityRenderer {
    /// `playerX/Y/Z`: the view entity's interpolated position.
    pub player_pos: DVec3,
    /// `playerYaw`, `playerPitch`.
    pub player_yaw_pitch: (f32, f32),
    /// `TileEntitySignRenderer`.
    pub sign: sign::TileEntitySignRenderer,
    /// `TileEntityMobSpawnerRenderer`.
    pub mob_spawner: mob_spawner::TileEntityMobSpawnerRenderer,
    /// `TileEntityRendererPiston`.
    pub piston: piston::TileEntityRendererPiston,
}

/// The tile entity at a block, if its chunk is loaded (no side effects).
pub fn tile_entity_at(world: &World, [x, y, z]: [i32; 3]) -> Option<&TileEntity> {
    let c = world.loaded_chunk(x >> 4, z >> 4)?;
    let r = c
        .chunk_tile_entity_map
        .get(&ChunkPosition::new(x & 15, y, z & 15))?;
    world.tile_entities.get(r)
}

impl TileEntityRenderer {
    /// `func_31072_a(world)`: a new world for the renderers that keep one
    /// (the piston's `RenderBlocks`).
    // src: TileEntityRenderer.java:87-98
    pub fn func_31072_a(&mut self) {
        self.piston.func_31069_a();
    }

    /// `cacheActiveRenderInfo(world, …, viewEntity, partial)`.
    // src: TileEntityRenderer.java:55-68
    pub fn cache_active_render_info(&mut self, _world: &mut World, var4: &Entity, var5: f32) {
        let b = &var4.base;
        self.player_yaw_pitch = (
            b.prev_rotation_yaw + (b.rotation_yaw - b.prev_rotation_yaw) * var5,
            b.prev_rotation_pitch + (b.rotation_pitch - b.prev_rotation_pitch) * var5,
        );
        let p = var5 as f64;
        self.player_pos = DVec3::new(
            b.last_tick_pos_x + (b.pos_x - b.last_tick_pos_x) * p,
            b.last_tick_pos_y + (b.pos_y - b.last_tick_pos_y) * p,
            b.last_tick_pos_z + (b.pos_z - b.last_tick_pos_z) * p,
        );
    }

    /// `renderTileEntity(tileEntity, partial)`.
    // src: TileEntityRenderer.java:70-77
    pub fn render_tile_entity(
        &mut self,
        r: &mut Renderers,
        cx: &mut Ctx,
        pos: [i32; 3],
        var2: f32,
    ) {
        let Some(te) = tile_entity_at(cx.world, pos).cloned() else {
            return;
        };
        let p = self.player_pos;
        if te.get_distance_from(p.x, p.y, p.z) < 4096.0 {
            let var3 = cx
                .world
                .get_light_brightness(te.x_coord, te.y_coord, te.z_coord);
            cx.gl.color3f(var3, var3, var3);
            // staticPlayerX/Y/Z = RenderManager.renderPos (RenderGlobal.java:264-266)
            let s = cx.info.render_pos;
            self.render_tile_entity_at(
                r,
                cx,
                &te,
                te.x_coord as f64 - s.x,
                te.y_coord as f64 - s.y,
                te.z_coord as f64 - s.z,
                var2,
            );
        }
    }

    /// `renderTileEntityAt(tileEntity, x, y, z, partial)`: the class's
    /// special renderer.
    // src: TileEntityRenderer.java:79-85
    #[allow(clippy::too_many_arguments)]
    pub fn render_tile_entity_at(
        &mut self,
        r: &mut Renderers,
        cx: &mut Ctx,
        te: &TileEntity,
        x: f64,
        y: f64,
        z: f64,
        partial: f32,
    ) {
        match &te.kind {
            T::Sign(s) => self.sign.render_tile_entity_sign_at(cx, te, s, x, y, z),
            T::MobSpawner(m) => self
                .mob_spawner
                .render_tile_entity_mob_spawner(r, cx, m, x, y, z, partial),
            T::Piston(p) => self.piston.func_31070_a(cx, te, p, x, y, z, partial),
            _ => {}
        }
    }
}
