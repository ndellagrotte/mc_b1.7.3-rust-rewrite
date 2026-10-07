//! The read-only world copy a mesh job draws from: vanilla's `ChunkCache`
//! (ChunkCache.java), the `IBlockAccess` that `WorldRenderer.updateRenderer`
//! hands to `RenderBlocks` (WorldRenderer.java:100-101).
//!
//! # Contents
//!
//! Built on the main thread from the [`World`] ([`ChunkSnapshot::build`]),
//! then moved into a task on Bevy's `AsyncComputeTaskPool`:
//!
//! - block ids, metadata, sky light and block light of a box: the blocks to
//!   draw grown by [`MARGIN`] on every side. `RenderBlocks` reads at most
//!   [`MARGIN`] blocks away from the block it draws (a neighbour, plus the
//!   brightest-neighbour rule of slabs, farmland and stairs, plus
//!   `BlockPortal.shouldSideBeRendered`'s two more, BlockPortal.java:124-127);
//!   cells above y 127 or below 0 behave as `ChunkCache` says (id 0; light
//!   `15 - skylightSubtracted` above, 0 below). A read outside the box is a
//!   bug in a renderer: it returns air and is logged once.
//! - `skylightSubtracted` and the world provider's `lightBrightnessTable`
//!   (ChunkCache.java:54, :100, :109);
//! - the `Block` singletons' mutable state ([`BlockStatics`]: the bounds of
//!   every block id, the leaves' `graphicsLevel` and texture), copied from
//!   the world; `RenderBlocks` mutates its own copy as the Java mutates the
//!   singletons, and the world's stays untouched (the simulation never sees
//!   the renderer's `setBlockBounds`, as in the headless oracle);
//! - for the blocks to draw, what `mc-sim` computes from the world and the
//!   job cannot: `getBlockTexture` of the kinds that override it
//!   ([`TEXTURE_OVERRIDE_KINDS`]: grass, dispenser, chest, furnace, locked
//!   chest, stairs) and the bounds `setBlockBoundsBasedOnState` sets
//!   ([`BOUNDS_OVERRIDE_KINDS`]), each computed by `mc-sim`'s
//!   `block::dispatch` on the world (the world's bounds are restored
//!   afterwards);
//! - the positions of the tile entities with a special renderer (sign, mob
//!   spawner, moving piston), for WP-3.5b (`WorldRenderer.tileEntityRenderers`,
//!   WorldRenderer.java:126-131). They are looked up as pass 0 looks them
//!   up, through `ChunkCache.getBlockTileEntity` →
//!   `Chunk.getChunkBlockTileEntity` (mc-sim
//!   `chunk::get_chunk_block_tile_entity`), with its side effects on the
//!   world: a container block without a tile entity gets one from its
//!   `onBlockAdded`, and an invalid one is removed from the chunk's map
//!   (ChunkCache.java:42-46, Chunk.java:401-421). The cell is re-read from
//!   the world afterwards, as the live `ChunkCache` would see it (a furnace's
//!   or dispenser's `onBlockAdded` sets its metadata). The block events
//!   that change raises mark the section dirty once more; vanilla loses
//!   that mark (`updateRenderers` clears `needsUpdate` after the rebuild,
//!   RenderGlobal.java:1010-1011, :1027-1028), so here the section is rebuilt one extra
//!   time;
//! - a copy of the world's `WorldChunkManager` (biome temperature and
//!   humidity for `colorMultiplier`: grass, leaves, tall grass) and the
//!   colour maps ([`Colorizers`]).
//!
//! `ChunkCache`'s constructor loads (or generates) the 3×3 chunks around the
//! section through `World.getChunkFromChunkCoords` (ChunkCache.java:17-21):
//! in single player, this is how the terrain around a moving player gets
//! loaded. [`ChunkSnapshot::build`] does the same before copying.
//!
//! `Chunk.isLit` (the static set by `Chunk.getBlockLightValue` when the sky
//! light there is above 0, Chunk.java:341-345) is tracked per snapshot
//! ([`ChunkSnapshot::take_is_lit`]): `WorldRenderer.isChunkLit` decides
//! which sections `updateAllRenderers` rebuilds when the sky darkens.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};

use mc_sim::block::{self, BlockBounds, BlockKind, BlockStatics, Material, dispatch};
use mc_sim::world::World;
use mc_sim::world::chunk::{self, block_index};
use mc_sim::world::tile_entity::TileEntityKindData;
use mc_sim::worldgen::chunk_manager::WorldChunkManager;

use crate::textures::Colorizers;

/// How far beyond the drawn blocks the snapshot reaches (module docs).
pub const MARGIN: i32 = 3;

/// The kinds whose `getBlockTexture(blockAccess, x, y, z, side)` reads the
/// world (mc-sim `block::dispatch::get_block_texture`): precomputed per
/// block.
// src: BlockGrass.java:12-21, BlockDispenser.java:53-62, BlockStairs.java:109-111,
// BlockChest.java:13-98, BlockFurnace.java:52-65, BlockLockedChest.java:11-40
pub fn is_texture_override_kind(kind: BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::Grass
            | BlockKind::Dispenser
            | BlockKind::Stairs { .. }
            | BlockKind::Chest
            | BlockKind::Furnace { .. }
            | BlockKind::LockedChest
    )
}

/// Documentation copy of [`is_texture_override_kind`]'s list.
pub const TEXTURE_OVERRIDE_KINDS: &str = "Grass, Dispenser, Stairs, Chest, Furnace, LockedChest (mc-sim block::dispatch::get_block_texture)";

/// The kinds that override `setBlockBoundsBasedOnState` (mc-sim
/// `block::dispatch::set_block_bounds_based_on_state`): precomputed per
/// block.
// src: Block.java:522-523 and the overrides cited in mc-sim block/dispatch.rs
pub fn is_bounds_override_kind(kind: BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::Stairs { .. }
            | BlockKind::Bed
            | BlockKind::Button
            | BlockKind::Rail { .. }
            | BlockKind::DetectorRail
            | BlockKind::PistonBase { .. }
            | BlockKind::Snow
            | BlockKind::Door
            | BlockKind::Cake
            | BlockKind::PistonExtension
            | BlockKind::Lever
            | BlockKind::PistonMoving
            | BlockKind::Sign { .. }
            | BlockKind::PressurePlate { .. }
            | BlockKind::Portal
            | BlockKind::TrapDoor
    )
}

/// Documentation copy of [`is_bounds_override_kind`]'s list.
pub const BOUNDS_OVERRIDE_KINDS: &str = "Stairs, Bed, Button, Rail, DetectorRail, PistonBase, Snow, Door, Cake, PistonExtension, Lever, PistonMoving, Sign, PressurePlate, Portal, TrapDoor (mc-sim block::dispatch::set_block_bounds_based_on_state)";

/// The `ChunkCache` stair/farmland ids whose light is their brightest
/// neighbour's: `stairSingle`, `tilledField`, `stairCompactPlanks`,
/// `stairCompactCobblestone`.
// src: ChunkCache.java:71
const NEIGHBOUR_LIGHT_IDS: [i32; 4] = [44, 60, 53, 67];

/// What a snapshot copies besides the blocks: the per-world and per-frame
/// state, kept by the terrain renderer.
#[derive(Clone, Debug)]
pub struct SnapshotTemplate {
    /// The world's `Block` singleton state (bounds, leaves graphics level
    /// and texture, piston head texture) without the simulation scratch.
    pub statics: BlockStatics,
    /// The world's `WorldChunkManager`.
    pub chunk_manager: WorldChunkManager,
    /// The colour maps.
    pub colorizers: Colorizers,
}

impl SnapshotTemplate {
    /// The template of `world` now.
    pub fn of(world: &mut World, colorizers: &Colorizers) -> Self {
        let chunk_manager = world.get_world_chunk_manager().clone();
        SnapshotTemplate {
            statics: render_statics(&world.block_statics),
            chunk_manager,
            colorizers: colorizers.clone(),
        }
    }

    /// Refreshes the block state from `world` (every frame: the bounds and
    /// the leaves change).
    pub fn refresh_statics(&mut self, world: &World) {
        copy_render_statics(&world.block_statics, &mut self.statics);
    }
}

/// A [`BlockStatics`] with the render-relevant fields of `s` (module docs).
pub fn render_statics(s: &BlockStatics) -> BlockStatics {
    let mut out = BlockStatics::default();
    copy_render_statics(s, &mut out);
    out
}

fn copy_render_statics(s: &BlockStatics, out: &mut BlockStatics) {
    *out.bounds = *s.bounds;
    out.graphics_level = s.graphics_level;
    out.leaves_block_index_in_texture = s.leaves_block_index_in_texture;
    out.piston_extension_head_texture = s.piston_extension_head_texture;
}

/// Which chunks [`ChunkSnapshot::build`] loads first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChunkLoad {
    /// `new ChunkCache(world, x0 - 1, y0 - 1, z0 - 1, x1 + 1, y1 + 1, z1 +
    /// 1)`: every chunk of the drawn box grown by one, loaded or generated
    /// (x outer, z inner, as the constructor's loops).
    // src: WorldRenderer.java:99-100, ChunkCache.java:9-23
    ChunkCache,
    /// None: read the loaded chunks only (an unloaded one reads as air).
    LoadedOnly,
}

/// The snapshot (module docs).
#[derive(Debug)]
pub struct ChunkSnapshot {
    /// The box's minimum corner.
    min: [i32; 3],
    /// The box's size.
    size: [i32; 3],
    /// Block ids, `((y * sz) + z) * sx + x` relative to `min`.
    ids: Vec<u8>,
    /// Metadata.
    meta: Vec<u8>,
    /// Sky light.
    sky: Vec<u8>,
    /// Block light.
    block_light: Vec<u8>,
    /// The drawn blocks: `[min, max)`.
    pub render_min: [i32; 3],
    /// See [`ChunkSnapshot::render_min`].
    pub render_max: [i32; 3],
    /// `world.skylightSubtracted`.
    pub skylight_subtracted: i32,
    /// `world.worldProvider.lightBrightnessTable`.
    pub light_brightness_table: [f32; 16],
    /// The `Block` singleton state the job starts from.
    pub statics: BlockStatics,
    /// Precomputed `getBlockTexture` per drawn cell (module docs); `None`
    /// sides are where the Java throws.
    textures: BTreeMap<usize, (i32, [Option<i32>; 6])>,
    /// Precomputed `setBlockBoundsBasedOnState` per drawn cell: (id, bounds).
    bounds: BTreeMap<usize, (i32, BlockBounds)>,
    /// Tile entities with a special renderer among the drawn blocks, in the
    /// pass-0 loop's order (y, z, x).
    pub special_tile_entities: Vec<[i32; 3]>,
    /// The world's `WorldChunkManager` (scratch arrays mutated by lookups).
    chunk_manager: RefCell<Option<WorldChunkManager>>,
    /// The colour maps.
    pub colorizers: Colorizers,
    /// `Chunk.isLit` (module docs).
    is_lit: Cell<bool>,
}

/// One log line per kind of bug.
static OUT_OF_BOX_LOGGED: AtomicBool = AtomicBool::new(false);

/// Whether `id` has a `TileEntitySpecialRenderer`: the sign, the mob
/// spawner and the moving piston (`TileEntityRenderer`'s map).
// src: TileEntityRenderer.java:24-27
pub fn has_special_renderer(kind: &TileEntityKindData) -> bool {
    matches!(
        kind,
        TileEntityKindData::Sign(_)
            | TileEntityKindData::MobSpawner(_)
            | TileEntityKindData::Piston(_)
    )
}

impl ChunkSnapshot {
    /// A snapshot with no blocks (every read is air, full sky light): the
    /// `new RenderBlocks()` of the item renderer, which never reads its
    /// `blockAccess` (RenderBlocks.java:77-78).
    pub fn empty(statics: BlockStatics, colorizers: Colorizers) -> Self {
        ChunkSnapshot {
            min: [0; 3],
            size: [0; 3],
            ids: Vec::new(),
            meta: Vec::new(),
            sky: Vec::new(),
            block_light: Vec::new(),
            render_min: [0; 3],
            render_max: [0; 3],
            skylight_subtracted: 0,
            light_brightness_table: [1.0; 16],
            statics,
            textures: BTreeMap::new(),
            bounds: BTreeMap::new(),
            special_tile_entities: Vec::new(),
            chunk_manager: RefCell::new(None),
            colorizers,
            is_lit: Cell::new(false),
        }
    }

    /// Copies the box around `render_min..render_max` from `world` (module
    /// docs), after loading the chunks `load` asks for.
    pub fn build(
        world: &mut World,
        template: &SnapshotTemplate,
        render_min: [i32; 3],
        render_max: [i32; 3],
        load: ChunkLoad,
    ) -> Self {
        if load == ChunkLoad::ChunkCache {
            // src: ChunkCache.java:11-21
            let (cx0, cz0) = ((render_min[0] - 1) >> 4, (render_min[2] - 1) >> 4);
            let (cx1, cz1) = ((render_max[0] + 1) >> 4, (render_max[2] + 1) >> 4);
            for cx in cx0..=cx1 {
                for cz in cz0..=cz1 {
                    world.get_chunk_from_chunk_coords(cx, cz);
                }
            }
        }
        let min = render_min.map(|v| v - MARGIN);
        let size = [0, 1, 2].map(|i| render_max[i] - render_min[i] + 2 * MARGIN);
        let n = (size[0] * size[1] * size[2]) as usize;
        let mut ids = vec![0u8; n];
        let mut meta = vec![0u8; n];
        let mut sky = vec![0u8; n];
        let mut block_light = vec![0u8; n];
        for dz in 0..size[2] {
            for dx in 0..size[0] {
                let (x, z) = (min[0] + dx, min[2] + dz);
                let Some(c) = world.loaded_chunk(x >> 4, z >> 4) else {
                    continue;
                };
                for dy in 0..size[1] {
                    let y = min[1] + dy;
                    if !(0..128).contains(&y) {
                        continue;
                    }
                    let i = ((dy * size[2] + dz) * size[0] + dx) as usize;
                    let (lx, lz) = (x & 15, z & 15);
                    ids[i] = c.blocks[block_index(lx, y, lz)];
                    meta[i] = c.data.get_nibble(lx, y, lz) as u8;
                    sky[i] = c.skylight_map.get_nibble(lx, y, lz) as u8;
                    block_light[i] = c.blocklight_map.get_nibble(lx, y, lz) as u8;
                }
            }
        }
        let mut s = ChunkSnapshot {
            min,
            size,
            ids,
            meta,
            sky,
            block_light,
            render_min,
            render_max,
            skylight_subtracted: world.skylight_subtracted,
            light_brightness_table: world.world_provider.light_brightness_table,
            statics: template.statics.clone(),
            textures: BTreeMap::new(),
            bounds: BTreeMap::new(),
            special_tile_entities: Vec::new(),
            chunk_manager: RefCell::new(Some(template.chunk_manager.clone())),
            colorizers: template.colorizers.clone(),
            is_lit: Cell::new(false),
        };
        s.precompute(world);
        s
    }

    /// The world-dependent block methods and the tile entities of the drawn
    /// blocks (module docs), in the drawing loop's order (y, z, x).
    fn precompute(&mut self, world: &mut World) {
        let saved_bounds = world.block_statics.bounds.clone();
        for y in self.render_min[1].max(0)..self.render_max[1].min(128) {
            for z in self.render_min[2]..self.render_max[2] {
                for x in self.render_min[0]..self.render_max[0] {
                    let id = self.get_block_id(x, y, z);
                    let Some(def) = block::block(id) else {
                        continue;
                    };
                    let Some(i) = self.index(x, y, z) else {
                        continue;
                    };
                    // Pass 0 asks the ChunkCache for the tile entity of every
                    // container before drawing it, and Chunk.getChunkBlockTileEntity
                    // changes the world: a container without one gets it from
                    // onBlockAdded (which may also set the block's metadata: the
                    // furnace's and dispenser's setDefaultDirection), and an invalid
                    // one is dropped from the chunk's map.
                    // src: WorldRenderer.java:126-131, ChunkCache.java:42-46, Chunk.java:401-421
                    if def.is_block_container && world.loaded_chunk(x >> 4, z >> 4).is_some() {
                        let r = chunk::get_chunk_block_tile_entity(
                            world,
                            x >> 4,
                            z >> 4,
                            x & 15,
                            y,
                            z & 15,
                        );
                        if let Some(c) = world.loaded_chunk(x >> 4, z >> 4) {
                            // the block is drawn from the world as it is now
                            self.ids[i] = c.blocks[block_index(x & 15, y, z & 15)];
                            self.meta[i] = c.data.get_nibble(x & 15, y, z & 15) as u8;
                        }
                        if let Some(r) = r
                            && let Some(te) = world.tile_entities.get(&r)
                            && has_special_renderer(&te.kind)
                        {
                            self.special_tile_entities.push([x, y, z]);
                        }
                    }
                    let id = self.get_block_id(x, y, z);
                    let Some(def) = block::block(id) else {
                        continue;
                    };
                    if is_texture_override_kind(def.kind) {
                        let t = [0, 1, 2, 3, 4, 5]
                            .map(|side| dispatch::get_block_texture(world, id, x, y, z, side));
                        self.textures.insert(i, (id, t));
                    }
                    if is_bounds_override_kind(def.kind) {
                        dispatch::set_block_bounds_based_on_state(world, id, x, y, z);
                        self.bounds
                            .insert(i, (id, world.block_statics.bounds[(id & 255) as usize]));
                    }
                }
            }
        }
        world.block_statics.bounds = saved_bounds;
    }

    /// The cell index of `(x, y, z)`, `None` outside the box.
    fn index(&self, x: i32, y: i32, z: i32) -> Option<usize> {
        let (dx, dy, dz) = (x - self.min[0], y - self.min[1], z - self.min[2]);
        if dx < 0
            || dy < 0
            || dz < 0
            || dx >= self.size[0]
            || dy >= self.size[1]
            || dz >= self.size[2]
        {
            return None;
        }
        Some(((dy * self.size[2] + dz) * self.size[0] + dx) as usize)
    }

    /// A read outside the box (module docs).
    fn out_of_box(&self, x: i32, y: i32, z: i32) {
        if !self.ids.is_empty() && !OUT_OF_BOX_LOGGED.swap(true, Ordering::Relaxed) {
            log::warn!(
                "terrain: a renderer read ({x}, {y}, {z}) outside its snapshot {:?}+{:?}",
                self.min,
                self.size
            );
        }
    }

    /// Test helper: sets a cell.
    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub fn set(&mut self, x: i32, y: i32, z: i32, id: i32, meta: i32, sky: i32, light: i32) {
        let i = self.index(x, y, z).expect("in the box");
        self.ids[i] = id as u8;
        self.meta[i] = meta as u8;
        self.sky[i] = sky as u8;
        self.block_light[i] = light as u8;
    }

    /// Test helper: an all-air box `[min, max)` drawn whole, sky light 15,
    /// the overworld's brightness table and the given chunk manager.
    #[cfg(test)]
    pub fn synthetic(min: [i32; 3], max: [i32; 3], table: [f32; 16]) -> Self {
        let mut s = ChunkSnapshot::empty(BlockStatics::default(), Colorizers::default());
        s.min = min;
        s.size = [0, 1, 2].map(|i| max[i] - min[i]);
        let n = (s.size[0] * s.size[1] * s.size[2]) as usize;
        s.ids = vec![0; n];
        s.meta = vec![0; n];
        s.sky = vec![15; n];
        s.block_light = vec![0; n];
        s.render_min = min;
        s.render_max = max;
        s.light_brightness_table = table;
        s
    }

    /// Test helper: installs a chunk manager.
    #[cfg(test)]
    pub fn set_chunk_manager(&mut self, m: WorldChunkManager) {
        *self.chunk_manager.borrow_mut() = Some(m);
    }

    // ---- IBlockAccess (ChunkCache) -----------------------------------------------

    /// `getBlockId(x, y, z)`.
    // src: ChunkCache.java:25-40
    pub fn get_block_id(&self, x: i32, y: i32, z: i32) -> i32 {
        if !(0..128).contains(&y) {
            return 0;
        }
        match self.index(x, y, z) {
            Some(i) => self.ids[i] as i32,
            None => {
                self.out_of_box(x, y, z);
                0
            }
        }
    }

    /// `getBlockMetadata(x, y, z)`.
    // src: ChunkCache.java:116-126
    pub fn get_block_metadata(&self, x: i32, y: i32, z: i32) -> i32 {
        if !(0..128).contains(&y) {
            return 0;
        }
        match self.index(x, y, z) {
            Some(i) => self.meta[i] as i32,
            None => {
                self.out_of_box(x, y, z);
                0
            }
        }
    }

    /// `getBrightness(x, y, z, minLight)`: the brightness table at the
    /// light value, at least `min`.
    // src: ChunkCache.java:48-55
    pub fn get_brightness(&self, x: i32, y: i32, z: i32, min: i32) -> f32 {
        let mut var5 = self.get_light_value(x, y, z);
        if var5 < min {
            var5 = min;
        }
        self.light_brightness_table[var5 as usize]
    }

    /// `getLightBrightness(x, y, z)`.
    // src: ChunkCache.java:57-59
    pub fn get_light_brightness(&self, x: i32, y: i32, z: i32) -> f32 {
        self.light_brightness_table[self.get_light_value(x, y, z) as usize]
    }

    /// `getLightValue(x, y, z)`.
    // src: ChunkCache.java:61-63
    pub fn get_light_value(&self, x: i32, y: i32, z: i32) -> i32 {
        self.get_light_value_ext(x, y, z, true)
    }

    /// `getLightValueExt(x, y, z, useNeighbours)`: slabs, farmland and the
    /// two stairs take their brightest neighbour (above and the four
    /// sides). Vanilla quirk, transcribed: the horizontal bound is `z <=
    /// 32000000` but `x < 32000000`.
    // src: ChunkCache.java:65-114
    pub fn get_light_value_ext(&self, x: i32, y: i32, z: i32, var4: bool) -> i32 {
        if x >= -32000000 && z >= -32000000 && x < 32000000 && z <= 32000000 {
            if var4 {
                let var5 = self.get_block_id(x, y, z);
                if NEIGHBOUR_LIGHT_IDS.contains(&var5) {
                    let mut var6 = self.get_light_value_ext(x, y + 1, z, false);
                    let var7 = self.get_light_value_ext(x + 1, y, z, false);
                    let var8 = self.get_light_value_ext(x - 1, y, z, false);
                    let var9 = self.get_light_value_ext(x, y, z + 1, false);
                    let var10 = self.get_light_value_ext(x, y, z - 1, false);
                    for v in [var7, var8, var9, var10] {
                        if v > var6 {
                            var6 = v;
                        }
                    }
                    return var6;
                }
            }
            if y < 0 {
                0
            } else if y >= 128 {
                (15 - self.skylight_subtracted).max(0)
            } else {
                match self.index(x, y, z) {
                    Some(i) => {
                        // Chunk.getBlockLightValue (Chunk.java:341-354).
                        let mut var5 = self.sky[i] as i32;
                        if var5 > 0 {
                            self.is_lit.set(true);
                        }
                        var5 -= self.skylight_subtracted;
                        let var6 = self.block_light[i] as i32;
                        if var6 > var5 {
                            var5 = var6;
                        }
                        var5
                    }
                    None => {
                        self.out_of_box(x, y, z);
                        0
                    }
                }
            }
        } else {
            15
        }
    }

    /// `getBlockMaterial(x, y, z)`.
    // src: ChunkCache.java:128-131
    pub fn get_block_material(&self, x: i32, y: i32, z: i32) -> Material {
        block::material_of(self.get_block_id(x, y, z))
    }

    /// `isBlockOpaqueCube(x, y, z)`.
    // src: ChunkCache.java:137-140
    pub fn is_block_opaque_cube(&self, x: i32, y: i32, z: i32) -> bool {
        block::block(self.get_block_id(x, y, z))
            .is_some_and(|b| block::is_opaque_cube(b.kind, self.statics.graphics_level))
    }

    /// `isBlockNormalCube(x, y, z)`: the material's `getIsSolid()` and
    /// `renderAsNormalBlock()` (unlike `World.isBlockNormalCube`, which
    /// asks `getIsTranslucent`).
    // src: ChunkCache.java:142-149
    pub fn is_block_normal_cube(&self, x: i32, y: i32, z: i32) -> bool {
        block::block(self.get_block_id(x, y, z)).is_some_and(|b| {
            b.block_material.get_is_solid() && block::render_as_normal_block(b.kind)
        })
    }

    /// `getWorldChunkManager().getBiomeGenBaseAt(x, z, 1, 1)`, then
    /// `temperature[0]` and `humidity[0]`; `(0, 0)` without a manager.
    // src: BlockGrass.java:24-26, BlockLeaves.java:30-32, BlockTallGrass.java:32-34
    pub fn biome_temperature_humidity(&self, x: i32, z: i32) -> (f64, f64) {
        let mut m = self.chunk_manager.borrow_mut();
        let Some(m) = m.as_mut() else {
            return (0.0, 0.0);
        };
        m.get_biome_gen_base_at(x, z, 1, 1);
        (m.temperature[0], m.humidity[0])
    }

    // ---- the precomputed block methods ---------------------------------------------

    /// The precomputed `getBlockTexture` of block `id` at `(x, y, z)`, if its
    /// kind reads the world (`Some(None)`: the Java throws).
    pub fn block_texture_override(
        &self,
        id: i32,
        x: i32,
        y: i32,
        z: i32,
        side: i32,
    ) -> Option<Option<i32>> {
        if self.textures.is_empty()
            || !block::block(id).is_some_and(|b| is_texture_override_kind(b.kind))
        {
            return None;
        }
        let i = self.index(x, y, z)?;
        let (tid, t) = self.textures.get(&i)?;
        (*tid == id && (0..6).contains(&side)).then(|| t[side as usize])
    }

    /// The precomputed bounds of block `id` at `(x, y, z)`, if its kind
    /// overrides `setBlockBoundsBasedOnState`.
    pub fn bounds_override(&self, id: i32, x: i32, y: i32, z: i32) -> Option<BlockBounds> {
        if self.bounds.is_empty()
            || !block::block(id).is_some_and(|b| is_bounds_override_kind(b.kind))
        {
            return None;
        }
        let i = self.index(x, y, z)?;
        let (bid, b) = self.bounds.get(&i)?;
        (*bid == id).then_some(*b)
    }

    /// `Chunk.isLit` since the last call, cleared (module docs).
    pub fn take_is_lit(&self) -> bool {
        self.is_lit.replace(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> [f32; 16] {
        mc_sim::world::provider::WorldProvider::new(mc_sim::world::provider::Dimension::Surface)
            .light_brightness_table
    }

    #[test]
    fn light_follows_chunk_cache() {
        let mut s = ChunkSnapshot::synthetic([0, 0, 0], [4, 4, 4], table());
        s.skylight_subtracted = 4;
        s.set(1, 1, 1, 1, 0, 10, 3);
        // sky 10 - 4 = 6 > block 3
        assert_eq!(s.get_light_value(1, 1, 1), 6);
        assert!(s.take_is_lit());
        assert!(!s.take_is_lit());
        s.set(2, 1, 1, 0, 0, 0, 9);
        assert_eq!(s.get_light_value(2, 1, 1), 9);
        assert!(!s.take_is_lit());
        // y outside 0..128
        assert_eq!(s.get_light_value(1, -1, 1), 0);
        assert_eq!(s.get_light_value(1, 128, 1), 11);
        // ±32,000,000: z inclusive, x exclusive (vanilla quirk)
        assert_eq!(s.get_light_value_ext(32000000, 1, 0, false), 15);
        assert_eq!(s.get_light_value_ext(-32000001, 1, 0, false), 15);
        assert_eq!(s.get_brightness(1, 1, 1, 0), table()[6]);
        assert_eq!(s.get_brightness(1, 1, 1, 13), table()[13]);
    }

    #[test]
    fn slabs_take_the_brightest_neighbour() {
        let mut s = ChunkSnapshot::synthetic([0, 0, 0], [5, 5, 5], table());
        for x in 0..5 {
            for y in 0..5 {
                for z in 0..5 {
                    s.set(x, y, z, 0, 0, 0, 0);
                }
            }
        }
        s.set(2, 2, 2, 44, 0, 0, 0);
        s.set(2, 3, 2, 0, 0, 2, 0);
        s.set(3, 2, 2, 0, 0, 0, 7);
        s.set(1, 2, 2, 0, 0, 5, 0);
        // the block below is not a candidate
        s.set(2, 1, 2, 0, 0, 15, 15);
        assert_eq!(s.get_light_value(2, 2, 2), 7);
        // a farmland neighbour of a farmland is read without the rule
        s.set(2, 2, 3, 60, 0, 0, 0);
        assert_eq!(s.get_light_value(2, 2, 2), 7);
    }

    #[test]
    fn blocks_outside_the_world_height_are_air() {
        let s = ChunkSnapshot::synthetic([0, -3, 0], [2, 131, 2], table());
        assert_eq!(s.get_block_id(0, -1, 0), 0);
        assert_eq!(s.get_block_metadata(0, 128, 0), 0);
        assert_eq!(s.get_block_material(0, 5, 0), Material::Air);
        assert!(!s.is_block_opaque_cube(0, 5, 0));
    }

    #[test]
    fn opaque_and_normal_cubes() {
        let mut s = ChunkSnapshot::synthetic([0, 0, 0], [3, 3, 3], table());
        s.set(0, 0, 0, 1, 0, 0, 0); // stone
        s.set(1, 0, 0, 20, 0, 0, 0); // glass
        s.set(2, 0, 0, 18, 0, 0, 0); // leaves
        s.set(0, 1, 0, 44, 0, 0, 0); // slab
        assert!(s.is_block_opaque_cube(0, 0, 0));
        assert!(!s.is_block_opaque_cube(1, 0, 0));
        // leaves are opaque only with fast graphics (graphicsLevel false)
        assert!(s.is_block_opaque_cube(2, 0, 0));
        s.statics.set_leaves_graphics_level(true);
        assert!(!s.is_block_opaque_cube(2, 0, 0));
        assert!(!s.is_block_opaque_cube(0, 1, 0));
        assert!(s.is_block_normal_cube(0, 0, 0));
        assert!(!s.is_block_normal_cube(0, 1, 0));
    }

    fn empty_world() -> World {
        use mc_sim::save::NoSave;
        use mc_sim::world::WorldEnvironment;
        use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
        use mc_sim::world::provider::{Dimension, WorldProvider};
        let mut w = World::new(
            Box::new(NoSave),
            "snapshot",
            WorldProvider::new(Dimension::Surface),
            5,
            ChunkGenerator::Empty(EmptyChunkProvider),
            WorldEnvironment {
                rand_seed: 0,
                field_9437_g: 0,
                lock_timestamp: 0,
            },
        );
        w.get_chunk_from_chunk_coords(0, 0);
        w
    }

    /// Pass 0's `ChunkCache.getBlockTileEntity` (Chunk.java:401-421): a
    /// container block without a tile entity gets one from `onBlockAdded`
    /// (listed when it has a special renderer), and an invalid one is
    /// dropped from the chunk's map.
    #[test]
    fn rebuild_creates_missing_tile_entities_and_drops_invalid_ones() {
        use mc_core::java_hash::ChunkPosition;
        let mut w = empty_world();
        {
            // raw block writes: no onBlockAdded, so no tile entity
            let c = w.get_chunk_from_chunk_coords(0, 0);
            c.blocks[block_index(1, 1, 1)] = 54; // chest
            c.blocks[block_index(3, 1, 1)] = 63; // standing sign
        }
        let chest = ChunkPosition::new(1, 1, 1);
        let sign = ChunkPosition::new(3, 1, 1);
        assert!(
            w.loaded_chunk(0, 0)
                .unwrap()
                .chunk_tile_entity_map
                .is_empty()
        );
        let t = SnapshotTemplate::of(&mut w, &Colorizers::default());
        let s = ChunkSnapshot::build(&mut w, &t, [0, 0, 0], [16, 16, 16], ChunkLoad::LoadedOnly);
        let map = &w.loaded_chunk(0, 0).unwrap().chunk_tile_entity_map;
        let (rc, rs) = (*map.get(&chest).unwrap(), *map.get(&sign).unwrap());
        assert!(matches!(
            w.tile_entities[&rc].kind,
            TileEntityKindData::Chest(_)
        ));
        assert!(matches!(
            w.tile_entities[&rs].kind,
            TileEntityKindData::Sign(_)
        ));
        // only the sign has a special renderer
        assert_eq!(s.special_tile_entities, vec![[3, 1, 1]]);
        // an invalid tile entity is removed from the chunk's map
        w.tile_entities.get_mut(&rs).unwrap().invalidate();
        let s = ChunkSnapshot::build(&mut w, &t, [0, 0, 0], [16, 16, 16], ChunkLoad::LoadedOnly);
        assert!(s.special_tile_entities.is_empty());
        let map = &w.loaded_chunk(0, 0).unwrap().chunk_tile_entity_map;
        assert!(map.get(&sign).is_none());
        assert_eq!(map.get(&chest).copied(), Some(rc));
    }

    /// The biome lookup the snapshot makes per block (`1×1` at the block)
    /// equals the 16×16 batch at the chunk origin, so a cached per-chunk
    /// table would be exact too.
    #[test]
    fn biome_lookup_matches_a_batch() {
        let mut batch = WorldChunkManager::new(123);
        batch.load_block_generator_data(Vec::new(), 32, -16, 16, 16);
        let (bt, bh) = (batch.temperature.clone(), batch.humidity.clone());
        let mut s = ChunkSnapshot::synthetic([0, 0, 0], [1, 1, 1], table());
        s.set_chunk_manager(WorldChunkManager::new(123));
        for (i, j) in [(0, 0), (5, 9), (15, 15), (7, 2)] {
            let (t, h) = s.biome_temperature_humidity(32 + i, -16 + j);
            assert_eq!(t, bt[(i * 16 + j) as usize]);
            assert_eq!(h, bh[(i * 16 + j) as usize]);
        }
    }
}
