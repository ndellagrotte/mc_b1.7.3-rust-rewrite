//! The terrain (WP-3.3): `RenderBlocks`, `WorldRenderer` and the terrain
//! half of `RenderGlobal`, drawn with Bevy (PLAN_v4 §3.5, D7).
//!
//! | Module | Vanilla | What |
//! |---|---|---|
//! | [`render_blocks`] | `RenderBlocks` | the shared state, the dispatch by render type, standard cubes with and without AO, the faces, `renderBlockUsingTexture`, `renderBlockOnInventory` ([`render_blocks::render_block_as_item`]) |
//! | [`fluids`], [`plants`], [`special`] | `RenderBlocks` | the other render types, one file per owner (WP-3.3b, 3.3c, 3.3d); stubs until filled |
//! | [`blocks`] | `Block` | the block methods the renderer calls, on a snapshot |
//! | [`snapshot`] | `ChunkCache` | the world copy a mesh job reads |
//! | [`tessellator`] | `Tessellator` | colour, normal and translation state, the quads |
//! | [`world_renderer`] | `WorldRenderer`, `RenderGlobal`, `RenderSorter`, `EntitySorter` | sections, rebuild scheduling, sorting, the F3 counters |
//! | [`frustum`] | `ClippingHelper(Impl)`, `Frustrum` | the frustum test |
//! | [`crack`] | `RenderGlobal.drawBlockBreaking` | the block-breaking overlay |
//! | [`material`] | — (GL state) | [`material::TerrainMaterial`] |
//!
//! # Each frame
//!
//! After the shell's frame and the sky's `compute_render_frame` (for
//! `FogState`), [`terrain_frame`] runs `renderWorld`'s terrain steps in its
//! order (EntityRenderer.java:466-545): the world's change and the marks it
//! queued (`IWorldAccess`: `markBlockAndNeighborsNeedsUpdate`,
//! `markBlockRangeNeedsUpdate`, `updateAllRenderers`), `loadRenderers` when
//! the render distance, graphics or AO changed, `clipRenderersByFrustrum`,
//! `updateRenderers` (with the frame-rate limit's time budget; the loop also
//! stops once [`max_pending_jobs`] jobs are in flight, since a far rebuild
//! here costs only a snapshot and a task where vanilla meshed it inside the
//! budget), finished
//! mesh jobs, `sortAndRender` for both passes (the visibility of the
//! section entities, the F3 line `RenderGlobal.getDebugInfoRenders` into
//! [`crate::debug::DebugSources::renders`]). [`draw_block_breaking`] then
//! places the crack overlay; [`update_terrain_materials`] keeps the fog and
//! the atlas current.
//!
//! # Entities
//!
//! A section with geometry in a pass gets one entity ([`SectionMesh`]) per
//! draw: pass 0 in the opaque phase; pass 1 blended in the transparent
//! phase ([`order::TRANSLUCENT`]), preceded with fancy graphics by its
//! depth-only draw ([`order::TRANSLUCENT_DEPTH`]: vanilla draws pass 1 with
//! `glColorMask(false…)` first, then again in colour, so only the nearest
//! translucent surface shows, EntityRenderer.java:520-541). With fast
//! graphics pass 1 is drawn once, blended with depth writes, so the order
//! of the sections decides what shows through what: vanilla calls them
//! nearest first, grouped by `RenderList` anchor
//! ([`world_renderer::RenderGlobalTerrain::render_list_order`]); Bevy sorts
//! the transparent phase back to front. Each fast-graphics pass-1 draw
//! therefore has its own material whose sort bias is its place in
//! vanilla's order ([`order::PASS1_STEP`]), set again whenever that order
//! changes. With fancy graphics the depth-only draw makes the order
//! irrelevant and one material serves every section. The mesh is in
//! the section's coordinates under a `Transform` at the section, scaled by
//! `1.000001` about its centre as the display list is
//! (WorldRenderer.java:118-121). Bevy culls each entity against the camera;
//! vanilla's own frustum flag hides the ones it would not draw
//! ([`frustum`] docs).
//!
//! # For WP-3.5b
//!
//! [`TerrainTileEntities`] is `RenderGlobal.tileEntities` (the sign, mob
//! spawner and moving-piston tile entities of the built sections, which
//! `renderEntities` hands to `TileEntityRenderer`) with
//! `renderEntitiesStartupCounter`; [`order`] has the bands. The entity pass
//! (`renderEntities`, RenderGlobal.java:254-307) is WP-3.5b's.

pub mod blocks;
pub mod crack;
pub mod fluids;
pub mod frustum;
pub mod material;
pub mod plants;
pub mod render_blocks;
pub mod snapshot;
pub mod special;
pub mod tessellator;
pub mod world_renderer;

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures::check_ready};
use mc_sim::world::events::WorldEvent;

use crate::GameFrameSet;
use crate::camera::CameraState;
use crate::clock;
use crate::debug::DebugSources;
use crate::events::{ClientEvent, WorldEventMsg};
use crate::fog::FogState;
use crate::game::{FrameView, Game};
use crate::settings::SettingsEffect;
use crate::sky;
use crate::textures::{Colorizers, TERRAIN_PNG, Textures, TexturesSet};

use material::{TerrainMaterial, TerrainMaterialPlugin, TerrainPhase};
use snapshot::{ChunkLoad, ChunkSnapshot, SnapshotTemplate};
use world_renderer::{RenderGlobalTerrain, SectionSettings, update_renderer};

/// Sort biases of the terrain's transparent draws, in the bands of
/// [`crate::gl_material::order`] (README, "Draw order").
pub mod order {
    /// The crack overlay with the eye in water: after the entities and
    /// particles, before the selection box
    /// ([`crate::gl_material::order::SELECTION_IN_WATER`]).
    // src: EntityRenderer.java:507-513
    pub const CRACK_IN_WATER: f32 = -0.9105e6;
    /// Pass 1's depth-only draw (fancy graphics).
    // src: EntityRenderer.java:520-526
    pub const TRANSLUCENT_DEPTH: f32 = -0.8e6;
    /// Pass 1. With fast graphics each section's draw adds its place in
    /// vanilla's call order times [`PASS1_STEP`] (module docs, "Entities").
    // src: EntityRenderer.java:527-545
    pub const TRANSLUCENT: f32 = -0.7e6;
    /// The bias between two consecutive pass-1 sections (fast graphics):
    /// more than any two sections' view-depth difference at Far (Bevy adds
    /// the view depth of the mesh's centre to the bias), so the bias alone
    /// orders them.
    pub const PASS1_STEP: f32 = 1024.0;
    /// The crack overlay otherwise: after pass 1, before the selection box
    /// ([`crate::gl_material::order::SELECTION`]).
    // src: EntityRenderer.java:549-555
    pub const CRACK: f32 = 0.69e6;
}

/// `1.000001F`, the display list's scale about the section centre.
// src: WorldRenderer.java:118-121
pub const SECTION_SCALE: f32 = 1.000001;

/// A section's draw (module docs, "Entities").
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SectionMesh {
    /// The renderer's index in the grid.
    pub renderer: usize,
    /// 0 pass 0, 1 pass 1, 2 pass 1's depth-only draw.
    pub slot: u8,
}

/// `RenderGlobal.tileEntities` and `renderEntitiesStartupCounter` for
/// WP-3.5b (module docs).
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct TerrainTileEntities {
    /// The tile entities' block positions.
    pub tile_entities: Vec<[i32; 3]>,
    /// `renderEntitiesStartupCounter`: `renderEntities` skips its first two
    /// calls after `loadRenderers` (RenderGlobal.java:251, :255-257).
    pub render_entities_startup_counter: i32,
}

/// Counts for the F3 line and the performance baseline.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TerrainStats {
    /// `updateRenderer` calls this frame.
    pub rebuilds_this_frame: u32,
    /// Since the world was loaded.
    pub rebuilds_total: u64,
    /// Mesh jobs on the task pool.
    pub pending_jobs: usize,
    /// Sections drawing pass 0 / pass 1.
    pub drawn: [i32; 2],
    /// Renderers waiting in `worldRenderersToUpdate`.
    pub to_update: usize,
    /// Time spent copying snapshots on the main thread since the world was
    /// loaded, in nanoseconds.
    pub snapshot_nanos_total: u64,
    /// Time spent meshing (`updateRenderer` on a snapshot) since the world
    /// was loaded, on whichever thread ran it, in nanoseconds.
    pub mesh_nanos_total: u64,
    /// [`terrain_frame`]'s own time this frame, in nanoseconds.
    pub frame_nanos: u64,
}

/// A mesh job on the task pool.
struct PendingJob {
    renderer: usize,
    generation: u64,
    serial: u64,
    task: Task<BuiltSection>,
}

/// A rebuilt section, meshed.
struct BuiltSection {
    /// How long the meshing took (nanoseconds).
    mesh_nanos: u64,
    meshes: [Option<Mesh>; 2],
    skip_render_pass: [bool; 2],
    is_chunk_lit: bool,
    tile_entity_renderers: Vec<[i32; 3]>,
}

/// The terrain renderer's state (module docs).
#[derive(Resource, Default)]
pub struct Terrain {
    /// `RenderGlobal`'s terrain half.
    pub global: RenderGlobalTerrain,
    /// The draws of each renderer ([`SectionMesh::slot`]).
    entities: Vec<[Option<Entity>; 3]>,
    /// Jobs in flight.
    pending: Vec<PendingJob>,
    /// The last job serial.
    serial: u64,
    /// The snapshot template of the current world.
    template: Option<SnapshotTemplate>,
    /// `EntityRenderer.renderEndNanoTime`, taken at the end of the frame
    /// ([`mark_render_end`]); left alone while a world loads.
    render_end_nano_time: i64,
    /// The settings the grid was built with: fancy graphics, AO.
    built_with: Option<(bool, bool)>,
    /// Fast graphics: each renderer's own pass-1 material and its sort
    /// bias (module docs, "Entities").
    pass1: Vec<Option<(Handle<TerrainMaterial>, f32)>>,
    /// A `loadRenderers` is due.
    reload: bool,
}

/// The materials of the four draws and the crack's mesh.
#[derive(Resource, Clone, Debug)]
pub struct TerrainHandles {
    /// Pass 0.
    pub opaque: Handle<TerrainMaterial>,
    /// Pass 1.
    pub translucent: Handle<TerrainMaterial>,
    /// Pass 1's depth-only draw.
    pub translucent_depth: Handle<TerrainMaterial>,
    /// The crack overlay.
    pub crack: Handle<TerrainMaterial>,
    /// The crack overlay's mesh.
    pub crack_mesh: Handle<Mesh>,
}

/// The terrain plugin (module docs).
pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TerrainMaterialPlugin)
            .init_resource::<Terrain>()
            .init_resource::<TerrainTileEntities>()
            .init_resource::<TerrainStats>()
            .add_systems(Startup, spawn_terrain_handles)
            .add_systems(
                Last,
                (power_saver_sleep, mark_render_end)
                    .chain()
                    .after(crate::perf::perf_frame_end),
            )
            .add_systems(
                Update,
                (terrain_frame, draw_block_breaking, update_terrain_materials)
                    .chain()
                    .after(GameFrameSet)
                    .after(sky::compute_render_frame)
                    .after(TexturesSet),
            );
    }
}

/// Whether two `Colorizers` share their three colour maps.
fn same_maps(a: &Colorizers, b: &Colorizers) -> bool {
    std::sync::Arc::ptr_eq(&a.grass, &b.grass)
        && std::sync::Arc::ptr_eq(&a.foliage, &b.foliage)
        && std::sync::Arc::ptr_eq(&a.water, &b.water)
}

/// The most mesh jobs in flight before `updateRenderers`' time-budget loop
/// stops early: 16 per task-pool thread (finished jobs are collected once a
/// frame, and a thread meshes about that many sections in a 60 Hz frame).
fn max_pending_jobs() -> usize {
    AsyncComputeTaskPool::get().thread_num().max(1) * 16
}

/// `renderEndNanoTime = System.nanoTime()` (EntityRenderer.java:390):
/// vanilla takes it when `renderWorld` returns (after the Power saver
/// sleep), before the GUI overlay and the next frame's `Display.update` and
/// ticks, and gives the next frame's `updateRenderers` until `1/120` (or
/// `1/40`) s after it (EntityRenderer.java:364-378). Here it is taken at the
/// end of the frame (`Last`, after [`power_saver_sleep`]): the next frame's
/// budget then holds what vanilla's holds, the wait for the GPU (Bevy's
/// extract waits for the previous frame's render, as `Display.update`
/// waits for the swap) and the ticks, but not the frame's own GUI, drawn
/// after the world in both. (Until WP-4.9 it was taken at the next
/// `First`, which kept the vsync wait out of the budget while vsync was
/// the default; with `--vsync on` the wait now uses up the budget and
/// `updateRenderers` runs about once a frame, as vanilla's would with the
/// driver's vsync forced on.)
///
/// Not while a world loads: `func_6255_d` blocks the run loop, so no
/// `updateCameraAndRender` runs and the first frame after the load sees the
/// time taken before it (Minecraft.java:1294, :1346-1378,
/// LoadingScreenRenderer.java:59-122).
// src: EntityRenderer.java:364-391
pub fn mark_render_end(game: NonSend<Game>, mut terrain: ResMut<Terrain>) {
    if game.pending.is_some() {
        return;
    }
    terrain.render_end_nano_time = clock::nano_time();
}

/// The frame-rate limit's sleep: only "Power saver" (`limitFramerate` 2,
/// 40 fps) sleeps, until `1/40` s after the previous `renderEndNanoTime`
/// (and 10 ms more when that time has passed and no world is open, a
/// quirk of the menu branch), if that is between 0 and 500 ms; "Balanced"
/// (the default) and "Max" never sleep, so vanilla runs uncapped there
/// (the window does not wait for the display either: b1.7.3 never turns
/// LWJGL's vsync on). Vanilla sleeps before taking `renderEndNanoTime` and
/// drawing the GUI; here the sleep ends the frame (`Last`), right before
/// [`mark_render_end`] takes the time, which gives the same period. The
/// binary's `--vsync on` additionally waits for the display. No sleep on the
/// loading screen's frames: vanilla's `func_6255_d` blocks the run loop, so
/// `updateCameraAndRender` (and its sleep) does not run while a world loads
/// (Minecraft.java:1294, :1346-1378, LoadingScreenRenderer.java:59-122).
// src: EntityRenderer.java:364-372, :381-391, :403-416
pub fn power_saver_sleep(game: NonSend<Game>, terrain: Res<Terrain>) {
    if game.pending.is_some() {
        return;
    }
    let s = &game.game_settings;
    if let Some(ms) = power_saver_sleep_ms(
        s.limit_framerate,
        game.the_world.is_some(),
        terrain.render_end_nano_time,
        clock::nano_time(),
    ) {
        std::thread::sleep(std::time::Duration::from_millis(ms as u64));
    }
}

/// [`power_saver_sleep`]'s arithmetic: the milliseconds to sleep, if any.
// src: EntityRenderer.java:364-372, :381-391, :403-416
pub fn power_saver_sleep_ms(
    limit_framerate: i32,
    world: bool,
    render_end_nano_time: i64,
    now: i64,
) -> Option<i64> {
    if limit_framerate != 2 {
        return None;
    }
    let var7: i64 = 40;
    let mut var8 = (render_end_nano_time + 1_000_000_000 / var7 - now) / 1_000_000;
    if !world && var8 < 0 {
        var8 += 10;
    }
    (var8 > 0 && var8 < 500).then_some(var8)
}

/// Creates the materials and the crack overlay's entity.
fn spawn_terrain_handles(
    mut commands: Commands,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let mut crack = TerrainMaterial::new(TerrainPhase::Crack, None, order::CRACK);
    // glColor4f(1.0F, 1.0F, 1.0F, 0.5F)
    // src: RenderGlobal.java:1080
    crack.params.color = Vec4::new(1.0, 1.0, 1.0, 0.5);
    // A mesh to fill later: one degenerate quad (Bevy's mesh allocator
    // rejects empty meshes).
    let mut placeholder = tessellator::Tessellator::default();
    for _ in 0..4 {
        placeholder.add_vertex_with_uv(0.0, 0.0, 0.0, 0.0, 0.0);
    }
    let crack_mesh = meshes.add(placeholder.to_mesh());
    let handles = TerrainHandles {
        opaque: materials.add(TerrainMaterial::new(TerrainPhase::Opaque, None, 0.0)),
        translucent: materials.add(TerrainMaterial::new(
            TerrainPhase::Translucent,
            None,
            order::TRANSLUCENT,
        )),
        translucent_depth: materials.add(TerrainMaterial::new(
            TerrainPhase::TranslucentDepth,
            None,
            order::TRANSLUCENT_DEPTH,
        )),
        crack: materials.add(crack),
        crack_mesh: crack_mesh.clone(),
    };
    commands.spawn((
        Mesh3d(crack_mesh),
        MeshMaterial3d(handles.crack.clone()),
        Transform::IDENTITY,
        Visibility::Hidden,
        crack::CrackOverlay,
    ));
    commands.insert_resource(handles);
}

impl Terrain {
    /// Despawns every section draw and forgets the jobs.
    fn clear_entities(&mut self, commands: &mut Commands) {
        for slots in self.entities.drain(..) {
            for e in slots.into_iter().flatten() {
                commands.entity(e).despawn();
            }
        }
        self.pass1.clear();
        self.pending.clear();
    }
}

/// The section's transform: at its corner, scaled by [`SECTION_SCALE`]
/// about its centre (`glTranslatef(-8) · glScalef · glTranslatef(8)`).
// src: WorldRenderer.java:117-121
pub fn section_transform(pos: [i32; 3]) -> Transform {
    let s = SECTION_SCALE;
    Transform {
        translation: Vec3::new(pos[0] as f32, pos[1] as f32, pos[2] as f32)
            + Vec3::splat(8.0 * (s - 1.0)),
        rotation: Quat::IDENTITY,
        scale: Vec3::splat(s),
    }
}

/// `updateRenderer()` on a snapshot, meshed: the two passes' meshes (the
/// translucent one flat-shaded when [`SectionSettings::flat_translucent`]).
fn build_section(snap: &ChunkSnapshot, pos: [i32; 3], settings: SectionSettings) -> BuiltSection {
    let t0 = clock::nano_time();
    let m = update_renderer(snap, pos, settings);
    let [p0, p1] = m.passes;
    let mesh = |t: Option<tessellator::Tessellator>, skip: bool, flat: bool| {
        t.filter(|t| !skip && !t.is_empty())
            .map(|t| if flat { t.to_flat_mesh() } else { t.to_mesh() })
    };
    let meshes = [
        mesh(p0, m.skip_render_pass[0], false),
        mesh(p1, m.skip_render_pass[1], settings.flat_translucent),
    ];
    BuiltSection {
        mesh_nanos: (clock::nano_time() - t0).max(0) as u64,
        meshes,
        skip_render_pass: m.skip_render_pass,
        is_chunk_lit: m.is_chunk_lit,
        tile_entity_renderers: m.tile_entity_renderers,
    }
}

/// Applies a rebuilt section: the flags, the tile entities, the draws.
#[allow(clippy::too_many_arguments)]
fn apply_section(
    t: &mut Terrain,
    stats: &mut TerrainStats,
    i: usize,
    built: BuiltSection,
    fancy: bool,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    handles: &TerrainHandles,
) {
    stats.mesh_nanos_total += built.mesh_nanos;
    let r = &mut t.global.world_renderers[i];
    r.skip_render_pass = built.skip_render_pass;
    r.is_chunk_lit = built.is_chunk_lit;
    r.is_initialized = true;
    let pos = [r.pos_x, r.pos_y, r.pos_z];
    let in_frustum = r.is_in_frustum;
    t.global
        .set_tile_entity_renderers(i, built.tile_entity_renderers);
    let [m0, m1] = built.meshes;
    for (pass, mesh) in [(0usize, m0), (1usize, m1)] {
        let slots: &[(u8, &Handle<TerrainMaterial>)] = if pass == 0 {
            &[(0, &handles.opaque)]
        } else if fancy {
            &[(1, &handles.translucent), (2, &handles.translucent_depth)]
        } else {
            &[(1, &handles.translucent)]
        };
        match mesh {
            Some(m) => {
                let h = meshes.add(m);
                let vis = if in_frustum && !built.skip_render_pass[pass] {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                for &(slot, mat) in slots {
                    match t.entities[i][slot as usize] {
                        Some(e) => {
                            commands
                                .entity(e)
                                .insert((Mesh3d(h.clone()), section_transform(pos)));
                        }
                        None => {
                            let e = commands
                                .spawn((
                                    Mesh3d(h.clone()),
                                    MeshMaterial3d(mat.clone()),
                                    section_transform(pos),
                                    vis,
                                    SectionMesh { renderer: i, slot },
                                ))
                                .id();
                            t.entities[i][slot as usize] = Some(e);
                        }
                    }
                }
            }
            None => {
                let range: &[usize] = if pass == 0 { &[0] } else { &[1, 2] };
                for &slot in range {
                    if let Some(e) = t.entities[i][slot].take() {
                        commands.entity(e).despawn();
                    }
                }
                if pass == 1
                    && let Some(m) = t.pass1.get_mut(i)
                {
                    *m = None;
                }
            }
        }
    }
}

/// `renderWorld`'s terrain steps (module docs, "Each frame").
#[allow(clippy::too_many_arguments)]
pub fn terrain_frame(
    mut game: NonSendMut<Game>,
    mut terrain: ResMut<Terrain>,
    mut stats: ResMut<TerrainStats>,
    camera: Res<CameraState>,
    colorizers: Res<Colorizers>,
    handles: Option<Res<TerrainHandles>>,
    mut world_events: MessageReader<WorldEventMsg>,
    mut client_events: MessageReader<ClientEvent>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut debug: ResMut<DebugSources>,
    mut tile_entities: ResMut<TerrainTileEntities>,
    mut vis: Query<&mut Visibility, With<SectionMesh>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
) {
    let _perf = crate::perf::time(crate::perf::Phase::Terrain);
    let Some(handles) = handles else { return };
    let frame_t0 = clock::nano_time();
    let t = &mut *terrain;
    let stats = &mut *stats;
    stats.rebuilds_this_frame = 0;
    for e in client_events.read() {
        match e {
            // renderGlobal.changeWorld(world): a new grid (Minecraft.java:1304-1310)
            // src: RenderGlobal.java:168-184
            ClientEvent::WorldChanged { .. } => {
                t.clear_entities(&mut commands);
                t.global = RenderGlobalTerrain::new();
                t.template = None;
                t.reload = true;
                stats.rebuilds_total = 0;
                stats.snapshot_nanos_total = 0;
                stats.mesh_nanos_total = 0;
            }
            ClientEvent::Settings(SettingsEffect::LoadRenderers) => t.reload = true,
            _ => {}
        }
    }
    let g = &mut *game;
    if g.the_world.is_none() {
        if !t.entities.is_empty() {
            t.clear_entities(&mut commands);
            t.global = RenderGlobalTerrain::new();
            t.template = None;
        }
        world_events.clear();
        return;
    }
    // The IWorldAccess calls of this frame's ticks, also while nothing is
    // drawn below (a preload's marks reach vanilla's RenderGlobal at once;
    // Bevy drops unread messages after two frames).
    for WorldEventMsg(e) in world_events.read() {
        match *e {
            WorldEvent::MarkBlockAndNeighborsNeedsUpdate { x, y, z } => {
                t.global.mark_block_and_neighbors_needs_update(x, y, z)
            }
            WorldEvent::MarkBlockRangeNeedsUpdate {
                x0,
                y0,
                z0,
                x1,
                y1,
                z1,
            } => t
                .global
                .mark_block_range_needs_update(x0, y0, z0, x1, y1, z1),
            WorldEvent::UpdateAllRenderers => t.global.update_all_renderers(),
            _ => {}
        }
    }

    // No renderWorld while the loading screen preloads the chunks (vanilla
    // does not render inside func_6255_d).
    if g.pending.is_some() {
        return;
    }
    let Some(view) = sky::gather_view(g) else {
        return;
    };
    let partial = g.timer.render_partial_ticks;
    let s = &g.game_settings;
    let (rd, fancy, ao, anaglyph, limit) = (
        s.render_distance,
        s.fancy_graphics,
        s.ambient_occlusion,
        s.anaglyph,
        s.limit_framerate,
    );
    let view_pos = [view.pos.x, view.pos.y, view.pos.z];
    let world = g.the_world.as_mut().expect("checked");

    // loadRenderers: a new world, an option asked for it, or (in
    // sortAndRender) the render distance changed.
    // src: RenderGlobal.java:186-252, :395-397
    if t.reload || t.global.render_distance != rd || t.built_with != Some((fancy, ao)) {
        t.reload = false;
        // Block.leaves.setGraphicsLevel(fancyGraphics)
        // src: RenderGlobal.java:187
        world.block_statics.set_leaves_graphics_level(fancy);
        t.clear_entities(&mut commands);
        t.global.load_renderers(rd, Some(view_pos));
        t.entities = vec![[None; 3]; t.global.world_renderers.len()];
        t.pass1 = vec![None; t.global.world_renderers.len()];
        t.built_with = Some((fancy, ao));
    }
    match &mut t.template {
        Some(tpl) => {
            tpl.refresh_statics(world);
            // refreshTextures re-reads the colour maps (WP-3.2 replaces the
            // resource): later rebuilds use the new ones.
            if !same_maps(&tpl.colorizers, &colorizers) {
                tpl.colorizers = colorizers.clone();
            }
        }
        None => t.template = Some(SnapshotTemplate::of(world, &colorizers)),
    }

    // clipRenderersByFrustrum(new Frustrum() at the interpolated position)
    // src: EntityRenderer.java:482-484
    if let Some(setup) = camera.setup {
        let p = view.last_tick_interp(partial);
        let f = frustum::frustrum_of(&setup, [p.x, p.y, p.z]);
        t.global.clip_renderers_by_frustrum(&f);
    }

    // updateRenderers until done or out of the frame's time.
    // src: EntityRenderer.java:365-378, :485-492
    let settings = SectionSettings {
        ambient_occlusion: ao,
        anaglyph,
        flat_translucent: ao && !fancy,
    };
    let var7: i64 = match limit {
        1 => 120,
        2 => 40,
        _ => 200,
    };
    let var2 = if limit == 0 {
        0
    } else {
        t.render_end_nano_time + 1_000_000_000 / var7
    };
    let template = t.template.take().expect("set above");
    let max_jobs = max_pending_jobs();
    let mut rebuilt = 0i32;
    loop {
        let (done, list) = t.global.update_renderers(view_pos, false);
        for i in list {
            let r = &mut t.global.world_renderers[i];
            let pos = [r.pos_x, r.pos_y, r.pos_z];
            let near = r.distance_to_entity_squared(view_pos) <= 256.0;
            let generation = r.generation;
            // updateRenderer() does nothing for a clean renderer; the caller
            // clears needsUpdate either way.
            // src: WorldRenderer.java:82, RenderGlobal.java:1010-1011, :1027-1028
            if !r.needs_update {
                continue;
            }
            r.needs_update = false;
            // WorldRenderer.chunksUpdated (the F3 "chunk updates")
            // src: WorldRenderer.java:83
            rebuilt += 1;
            stats.rebuilds_this_frame += 1;
            stats.rebuilds_total += 1;
            let snap_t0 = clock::nano_time();
            let snap = ChunkSnapshot::build(
                world,
                &template,
                pos,
                [pos[0] + 16, pos[1] + 16, pos[2] + 16],
                ChunkLoad::ChunkCache,
            );
            stats.snapshot_nanos_total += (clock::nano_time() - snap_t0).max(0) as u64;
            t.serial += 1;
            let serial = t.serial;
            if near {
                let built = build_section(&snap, pos, settings);
                t.global.world_renderers[i].applied_serial = serial;
                apply_section(
                    t,
                    stats,
                    i,
                    built,
                    fancy,
                    &mut commands,
                    &mut meshes,
                    &handles,
                );
            } else {
                let task = AsyncComputeTaskPool::get()
                    .spawn(async move { build_section(&snap, pos, settings) });
                t.pending.push(PendingJob {
                    renderer: i,
                    generation,
                    serial,
                    task,
                });
            }
        }
        if done || var2 == 0 {
            break;
        }
        // Project choice: vanilla's far rebuild costs its meshing time within
        // the budget; here it is a snapshot and a task, so the loop also stops
        // once enough jobs are in flight (module docs, "Each frame").
        if t.pending.len() >= max_jobs {
            break;
        }
        let var20 = var2 - clock::nano_time();
        if !(0..=1_000_000_000).contains(&var20) {
            break;
        }
    }

    t.template = Some(template);
    g.chunks_updated = g.chunks_updated.wrapping_add(rebuilt);

    // Finished jobs.
    let mut still = Vec::with_capacity(t.pending.len());
    for mut job in std::mem::take(&mut t.pending) {
        match check_ready(&mut job.task) {
            Some(built) => {
                let r = &t.global.world_renderers[job.renderer];
                if r.generation == job.generation && job.serial > r.applied_serial {
                    t.global.world_renderers[job.renderer].applied_serial = job.serial;
                    apply_section(
                        t,
                        stats,
                        job.renderer,
                        built,
                        fancy,
                        &mut commands,
                        &mut meshes,
                        &handles,
                    );
                }
            }
            None => still.push(job),
        }
    }
    t.pending = still;

    // sortAndRender(view, 0) and (view, 1).
    // src: EntityRenderer.java:498, :526/:543
    let d0 = t.global.sort_and_render(view_pos, 0);
    let d1 = t.global.sort_and_render(view_pos, 1);
    for (i, r) in t.global.world_renderers.iter().enumerate() {
        let Some(slots) = t.entities.get(i) else {
            continue;
        };
        for (slot, e) in slots.iter().enumerate() {
            let Some(e) = e else { continue };
            let pass = if slot == 0 { 0 } else { 1 };
            let want = if r.draws_pass(pass) {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if let Ok(mut v) = vis.get_mut(*e)
                && *v != want
            {
                *v = want;
            }
        }
    }
    if !fancy {
        order_pass1(t, &handles, &mut materials, &mut commands);
    }
    let line = t.global.get_debug_info_renders();
    if debug.renders != line {
        debug.renders = line;
    }
    if tile_entities.tile_entities != t.global.tile_entities
        || tile_entities.render_entities_startup_counter != t.global.render_entities_startup_counter
    {
        tile_entities.tile_entities = t.global.tile_entities.clone();
        tile_entities.render_entities_startup_counter = t.global.render_entities_startup_counter;
    }
    stats.pending_jobs = t.pending.len();
    stats.drawn = [d0, d1];
    stats.to_update = t.global.world_renderers_to_update.len();
    stats.frame_nanos = (clock::nano_time() - frame_t0).max(0) as u64;
}

/// Fast graphics: gives each drawn pass-1 section the sort bias of its
/// place in vanilla's call order (module docs, "Entities"), with its own
/// material (a copy of the shared pass-1 material) where it has none yet.
// src: RenderGlobal.java:529-587
fn order_pass1(
    t: &mut Terrain,
    handles: &TerrainHandles,
    materials: &mut Assets<TerrainMaterial>,
    commands: &mut Commands,
) {
    let list = t.global.render_list_order(1);
    // Compressed when the band would overflow into the crack's (never at
    // the four render distances' section counts in practice).
    let room = order::CRACK - order::TRANSLUCENT - order::PASS1_STEP;
    let step = order::PASS1_STEP.min(room / list.len().max(1) as f32);
    for (rank, &i) in list.iter().enumerate() {
        let Some(e) = t.entities.get(i).and_then(|s| s[1]) else {
            continue;
        };
        let bias = order::TRANSLUCENT + rank as f32 * step;
        match t.pass1.get_mut(i) {
            Some(Some((h, b))) => {
                if *b != bias {
                    *b = bias;
                    if let Some(mut m) = materials.get_mut(&*h) {
                        m.sort_bias = bias;
                    }
                }
            }
            Some(slot @ None) => {
                let Some(mut m) = materials.get(&handles.translucent).cloned() else {
                    continue;
                };
                m.sort_bias = bias;
                let h = materials.add(m);
                commands.entity(e).insert(MeshMaterial3d(h.clone()));
                *slot = Some((h, bias));
            }
            None => {}
        }
    }
}

/// `drawBlockBreaking` at the frame's call site ([`crack`]).
// src: EntityRenderer.java:507-513, :549-555
#[allow(clippy::too_many_arguments)]
pub fn draw_block_breaking(
    mut cache: Local<Option<crack::CrackKey>>,
    mut game: NonSendMut<Game>,
    terrain: Res<Terrain>,
    frame: Res<FrameView>,
    handles: Option<Res<TerrainHandles>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    mut q: Query<(&mut Transform, &mut Visibility, &mut Mesh3d), With<crack::CrackOverlay>>,
) {
    let _perf = crate::perf::time(crate::perf::Phase::Terrain);
    let Some(handles) = handles else { return };
    let Ok((mut tr, mut vis, mut mesh3d)) = q.single_mut() else {
        return;
    };
    let g = &mut *game;
    let hidden = |vis: &mut Visibility, cache: &mut Option<crack::CrackKey>| {
        *cache = None;
        if *vis != Visibility::Hidden {
            *vis = Visibility::Hidden;
        }
    };
    let (Some(m), Some(template)) = (frame.object_mouse_over, terrain.template.as_ref()) else {
        hidden(&mut vis, &mut cache);
        return;
    };
    if g.the_world.is_none() || g.pending.is_some() {
        hidden(&mut vis, &mut cache);
        return;
    }
    let Some(view) = sky::gather_view(g) else {
        hidden(&mut vis, &mut cache);
        return;
    };
    // The call after the translucent pass also wants `cameraZoom == 1`
    // (always, outside the high-resolution screenshot code); the one in
    // water does not test it.
    // src: EntityRenderer.java:507, :549
    if !view.is_player || (!view.in_water && g.entity_renderer.camera_zoom != 1.0) {
        hidden(&mut vis, &mut cache);
        return;
    }
    let (ao, anaglyph) = (g.game_settings.ambient_occlusion, g.game_settings.anaglyph);
    let world = g.the_world.as_mut().expect("checked");
    // The quads change only with the stage, the settings or the blocks
    // around: rebuild them then.
    let key = crack::CrackKey::of(world, template, &m, frame.damage_partial_time, ao, anaglyph);
    if *cache != Some(key) {
        let Some(quads) =
            crack::crack_quads(world, template, &m, frame.damage_partial_time, ao, anaglyph)
        else {
            hidden(&mut vis, &mut cache);
            return;
        };
        mesh3d.0 = meshes.add(quads.to_mesh());
        *tr = Transform::from_xyz(m.block_x as f32, m.block_y as f32, m.block_z as f32);
        *cache = Some(key);
    }
    if *vis != Visibility::Inherited {
        *vis = Visibility::Inherited;
    }
    let bias = if view.in_water {
        order::CRACK_IN_WATER
    } else {
        order::CRACK
    };
    if materials
        .get(&handles.crack)
        .is_some_and(|c| c.sort_bias != bias)
        && let Some(mut c) = materials.get_mut(&handles.crack)
    {
        c.sort_bias = bias;
    }
}

/// The terrain materials' fog (`setupFog(0)`, `FogState`) and texture
/// (`/terrain.png` from WP-3.2's cache), when they change.
// src: EntityRenderer.java:494-497
pub fn update_terrain_materials(
    fog: Res<FogState>,
    textures: Res<Textures>,
    handles: Option<Res<TerrainHandles>>,
    terrain: Res<Terrain>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
) {
    let Some(handles) = handles else { return };
    let tex = textures.entry(TERRAIN_PNG).map(|e| e.handle.clone());
    let fog_v = if fog.active {
        fog.world.to_vec4()
    } else {
        Vec4::ZERO
    };
    let fog_c = Vec4::new(fog.color[0], fog.color[1], fog.color[2], 1.0);
    let shared = [
        &handles.opaque,
        &handles.translucent,
        &handles.translucent_depth,
        &handles.crack,
    ];
    let own = terrain.pass1.iter().flatten().map(|(h, _)| h);
    for h in shared.into_iter().chain(own) {
        let changed = materials.get(h).is_some_and(|m| {
            m.params.fog != fog_v || m.params.fog_color != fog_c || m.texture != tex
        });
        if changed && let Some(mut m) = materials.get_mut(h) {
            m.params.fog = fog_v;
            m.params.fog_color = fog_c;
            m.params.textured = u32::from(tex.is_some());
            m.texture = tex.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_power_saver_sleeps() {
        let ms = 1_000_000i64;
        // Balanced (1) and Max (0): never.
        assert_eq!(power_saver_sleep_ms(1, true, 0, 3 * ms), None);
        assert_eq!(power_saver_sleep_ms(0, false, 0, 3 * ms), None);
        // Power saver: until 25 ms after the last renderEndNanoTime.
        assert_eq!(power_saver_sleep_ms(2, true, 0, 3 * ms), Some(22));
        assert_eq!(power_saver_sleep_ms(2, true, 0, 25 * ms), None);
        assert_eq!(power_saver_sleep_ms(2, true, 0, 30 * ms), None);
        // The menu branch adds 10 ms once the time has passed.
        assert_eq!(power_saver_sleep_ms(2, false, 0, 30 * ms), Some(5));
        assert_eq!(power_saver_sleep_ms(2, false, 0, 40 * ms), None);
        // At most 500 ms (a clock jump).
        assert_eq!(power_saver_sleep_ms(2, true, 600 * ms, 0), None);
    }
}
