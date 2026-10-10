//! `WorldRenderer` (WorldRenderer.java) and the terrain half of
//! `RenderGlobal` (RenderGlobal.java): the grid of 16×16×16 sections around
//! the view, which of them need a rebuild and in what order, the frustum
//! flags and the F3 counters; and `WorldRenderer.updateRenderer`'s body
//! as a mesh job ([`update_renderer`]).
//!
//! # The grid
//!
//! `loadRenderers` (RenderGlobal.java:186-252) makes `renderChunksWide ×
//! renderChunksTall × renderChunksDeep` renderers: `64 << (3 -
//! renderDistance)` blocks (at most 400) wide → 26, 17, 9 or 5 sections
//! across for far, normal, short, tiny, and 8 tall. `markRenderersForNewPosition`
//! (:317-384) moves each renderer to the position of its grid column
//! nearest the view, so the grid wraps around the player
//! (`(index of x) = x / 16 mod wide`); a moved renderer is emptied and
//! marked dirty. `func_949_a` (markBlocksForUpdate, :1199-1235) finds a
//! block's renderer by the same modulo, so a mark on a block outside the
//! grid lands on the renderer that will show it later (and a mark below
//! y 0 on the top row: vanilla quirk, transcribed).
//!
//! # Rebuilds
//!
//! `updateRenderers(view, false)` (:924-1066) rebuilds, per call, every
//! dirty renderer within 16 blocks of the view (nearest first) and one
//! more: the best other (`RenderSorter`: in the frustum, then nearest, then
//! lowest `chunkIndex`). Vanilla bug, transcribed: the two-slot "best
//! others" array is meant to hold two, but its shift loop stops at once,
//! so slot 0 is never filled (`--var13` reaches 0 before any move,
//! :980-988). `renderWorld` calls it until the list is empty or, with a
//! frame-rate limit, the frame's time is up (EntityRenderer.java:485-492).
//! Every 10 renderers per `sortAndRender` call are checked for a dirty one
//! missing from the list (:387-393).
//!
//! Here a rebuild is a snapshot of the world ([`super::snapshot`]) taken at
//! the moment vanilla runs `updateRenderer` (loading the chunks around it as
//! `ChunkCache` does) and meshed by [`update_renderer`]: the near ones at
//! once (vanilla's latency: an edit shows the same frame), the far one on
//! Bevy's `AsyncComputeTaskPool`. A renderer keeps drawing its previous
//! mesh until the new one is in; a renderer moved meanwhile drops the late
//! result.
//!
//! # Drawing
//!
//! `sortAndRender(view, pass)` (:386-511) re-sorts the renderers by
//! distance when the view moved more than 4 blocks (`EntitySorter`) and
//! counts the F3 numbers; a section's pass is drawn when it has geometry
//! (`!skipRenderPass[pass]`) and is in the frustum. Occlusion queries
//! (`ARB_occlusion_query`, `advancedOpengl`) are not available in wgpu:
//! `occlusionEnabled` is false, as on a GL without the extension
//! (RenderGlobal.java:66), so `O:` is always 0 and the `cloudOffsetX`
//! stagger (:465) never runs.

use mc_core::Aabb;
use mc_core::math::{bucket_int, floor_double, sqrt_float};

use super::blocks;
use super::frustum::Frustrum;
use super::render_blocks::RenderBlocks;
use super::snapshot::ChunkSnapshot;
use super::tessellator::Tessellator;

/// `WorldRenderer` (module docs).
#[derive(Clone, Debug)]
pub struct WorldRenderer {
    /// `posX`, `posY`, `posZ`: the section's minimum corner.
    pub pos_x: i32,
    /// See [`WorldRenderer::pos_x`].
    pub pos_y: i32,
    /// See [`WorldRenderer::pos_x`].
    pub pos_z: i32,
    /// `sizeWidth`, `sizeHeight`, `sizeDepth` (16).
    pub size: i32,
    /// `posXMinus`, `posYMinus`, `posZMinus`: the 1024-block anchor the
    /// display list is drawn relative to.
    pub pos_minus: [i32; 3],
    /// `posXClip`, `posYClip`, `posZClip`.
    pub pos_clip: [i32; 3],
    /// `posXPlus`, `posYPlus`, `posZPlus`: the centre.
    pub pos_plus: [i32; 3],
    /// `isInFrustum`.
    pub is_in_frustum: bool,
    /// `skipRenderPass`: the pass has no geometry.
    pub skip_render_pass: [bool; 2],
    /// `rendererRadius`.
    pub renderer_radius: f32,
    /// `needsUpdate`.
    pub needs_update: bool,
    /// `rendererBoundingBox`: the section grown by 6 blocks.
    pub renderer_bounding_box: Aabb,
    /// `chunkIndex`: the creation order in `loadRenderers`.
    pub chunk_index: i32,
    /// `isVisible` (occlusion; always true here).
    pub is_visible: bool,
    /// `isWaitingOnOcclusionQuery` (never set here).
    pub is_waiting_on_occlusion_query: bool,
    /// `isChunkLit`: the last rebuild read a sky light above 0.
    pub is_chunk_lit: bool,
    /// `isInitialized`.
    pub is_initialized: bool,
    /// `tileEntityRenderers`: the special-render tile entities in the section.
    pub tile_entity_renderers: Vec<[i32; 3]>,
    /// Bumped by every move ([`WorldRenderer::set_position`]): a mesh job
    /// started before the move is dropped.
    pub generation: u64,
    /// The serial of the last rebuild applied.
    pub applied_serial: u64,
}

impl WorldRenderer {
    /// `new WorldRenderer(world, tileEntities, x, y, z, 16, list)`.
    // src: WorldRenderer.java:42-51
    pub fn new(x: i32, y: i32, z: i32, size: i32) -> Self {
        let mut r = WorldRenderer {
            pos_x: -999,
            pos_y: 0,
            pos_z: 0,
            size,
            pos_minus: [0; 3],
            pos_clip: [0; 3],
            pos_plus: [0; 3],
            is_in_frustum: false,
            skip_render_pass: [false; 2],
            renderer_radius: sqrt_float((size * size + size * size + size * size) as f32) / 2.0,
            needs_update: false,
            renderer_bounding_box: Aabb::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            chunk_index: 0,
            is_visible: true,
            is_waiting_on_occlusion_query: false,
            is_chunk_lit: false,
            is_initialized: false,
            tile_entity_renderers: Vec::new(),
            generation: 0,
            applied_serial: 0,
        };
        r.set_position(x, y, z);
        r.needs_update = false;
        r
    }

    /// `setPosition(x, y, z)`: on a move, empties the section, recomputes
    /// the derived positions and the bounding box, and marks it dirty.
    // src: WorldRenderer.java:53-75
    pub fn set_position(&mut self, var1: i32, var2: i32, var3: i32) {
        if var1 != self.pos_x || var2 != self.pos_y || var3 != self.pos_z {
            self.set_dont_draw();
            self.pos_x = var1;
            self.pos_y = var2;
            self.pos_z = var3;
            self.pos_plus = [
                var1 + self.size / 2,
                var2 + self.size / 2,
                var3 + self.size / 2,
            ];
            self.pos_clip = [var1 & 1023, var2, var3 & 1023];
            self.pos_minus = [
                var1 - self.pos_clip[0],
                var2 - self.pos_clip[1],
                var3 - self.pos_clip[2],
            ];
            let var4 = 6.0f32;
            self.renderer_bounding_box = Aabb::new(
                (var1 as f32 - var4) as f64,
                (var2 as f32 - var4) as f64,
                (var3 as f32 - var4) as f64,
                ((var1 + self.size) as f32 + var4) as f64,
                ((var2 + self.size) as f32 + var4) as f64,
                ((var3 + self.size) as f32 + var4) as f64,
            );
            self.generation += 1;
            self.mark_dirty();
        }
    }

    /// `distanceToEntitySquared(entity)`: from the section's centre, in
    /// float.
    // src: WorldRenderer.java:174-179
    pub fn distance_to_entity_squared(&self, pos: [f64; 3]) -> f32 {
        let var2 = (pos[0] - self.pos_plus[0] as f64) as f32;
        let var3 = (pos[1] - self.pos_plus[1] as f64) as f32;
        let var4 = (pos[2] - self.pos_plus[2] as f64) as f32;
        var2 * var2 + var3 * var3 + var4 * var4
    }

    /// `setDontDraw()`.
    // src: WorldRenderer.java:181-188
    pub fn set_dont_draw(&mut self) {
        self.skip_render_pass = [true; 2];
        self.is_in_frustum = false;
        self.is_initialized = false;
    }

    /// `getGLCallListForPass(pass) >= 0`: in the frustum with geometry.
    // src: WorldRenderer.java:195-201
    pub fn draws_pass(&self, pass: usize) -> bool {
        self.is_in_frustum && !self.skip_render_pass[pass]
    }

    /// `updateInFrustrum(frustum)`.
    // src: WorldRenderer.java:203-205
    pub fn update_in_frustrum(&mut self, f: &Frustrum) {
        self.is_in_frustum = f.is_bounding_box_in_frustum(&self.renderer_bounding_box);
    }

    /// `skipAllRenderPasses()`: initialised and empty in both passes.
    // src: WorldRenderer.java:211-217
    pub fn skip_all_render_passes(&self) -> bool {
        self.is_initialized && self.skip_render_pass[0] && self.skip_render_pass[1]
    }

    /// `markDirty()`.
    // src: WorldRenderer.java:219-221
    pub fn mark_dirty(&mut self) {
        self.needs_update = true;
    }
}

/// `RenderSorter.doCompare(a, b)` for the view at `pos`: 1 when `a` is to
/// be rebuilt before `b` (in the frustum, then nearer, then the lower
/// `chunkIndex`), -1 otherwise (never 0).
// src: RenderSorter.java:12-30
pub fn render_sorter_compare(a: &WorldRenderer, b: &WorldRenderer, pos: [f64; 3]) -> i32 {
    let var3 = a.is_in_frustum;
    let var4 = b.is_in_frustum;
    if var3 && !var4 {
        1
    } else if var4 && !var3 {
        -1
    } else {
        let var5 = a.distance_to_entity_squared(pos) as f64;
        let var7 = b.distance_to_entity_squared(pos) as f64;
        if var5 < var7 {
            1
        } else if var5 > var7 {
            -1
        } else if a.chunk_index < b.chunk_index {
            1
        } else {
            -1
        }
    }
}

/// `EntitySorter`'s key: the squared distance from the section centre to
/// the entity (the comparator returns `(int)((d1 - d2) * 1024)`).
// src: EntitySorter.java:14-22
pub fn entity_sorter_key(r: &WorldRenderer, pos: [f64; 3]) -> f64 {
    let var3 = r.pos_plus[0] as f64 + -pos[0];
    let var5 = r.pos_plus[1] as f64 + -pos[1];
    let var7 = r.pos_plus[2] as f64 + -pos[2];
    var3 * var3 + var5 * var5 + var7 * var7
}

/// `EntitySorter.sortByDistanceToEntity(a, b)`: `(int)((d1 - d2) * 1024)`,
/// so two squared distances less than 1/1024 apart compare equal (and the
/// comparator is not transitive).
// src: EntitySorter.java:16-24
pub fn entity_sorter_compare(a: &WorldRenderer, b: &WorldRenderer, pos: [f64; 3]) -> i32 {
    ((entity_sorter_key(a, pos) - entity_sorter_key(b, pos)) * 1024.0) as i32
}

/// `Arrays.sort(Object[], Comparator)` as the JDK of the time (Java 6)
/// implements it: a stable top-down merge sort with an insertion sort
/// below 7 elements and the "halves already in order" shortcut. Written
/// out rather than `slice::sort_by` because [`entity_sorter_compare`] is
/// not a total order: the result depends on the algorithm, and the
/// standard library's sort may panic on such a comparator.
pub fn java_legacy_sort<T: Copy>(a: &mut [T], mut c: impl FnMut(&T, &T) -> i32) {
    let mut aux = a.to_vec();
    let len = a.len();
    legacy_merge_sort(&mut aux, a, 0, len, 0, &mut c);
}

/// `Arrays.mergeSort(src, dest, low, high, off, c)` (JDK 6).
fn legacy_merge_sort<T: Copy>(
    src: &mut [T],
    dest: &mut [T],
    mut low: usize,
    mut high: usize,
    off: isize,
    c: &mut impl FnMut(&T, &T) -> i32,
) {
    let length = high - low;
    if length < 7 {
        for i in low..high {
            let mut j = i;
            while j > low && c(&dest[j - 1], &dest[j]) > 0 {
                dest.swap(j, j - 1);
                j -= 1;
            }
        }
        return;
    }
    let dest_low = low;
    let dest_high = high;
    low = (low as isize + off) as usize;
    high = (high as isize + off) as usize;
    let mid = (low + high) >> 1;
    legacy_merge_sort(dest, src, low, mid, -off, c);
    legacy_merge_sort(dest, src, mid, high, -off, c);
    if c(&src[mid - 1], &src[mid]) <= 0 {
        dest[dest_low..dest_low + length].copy_from_slice(&src[low..high]);
        return;
    }
    let (mut p, mut q) = (low, mid);
    for d in dest.iter_mut().take(dest_high).skip(dest_low) {
        if q >= high || (p < mid && c(&src[p], &src[q]) <= 0) {
            *d = src[p];
            p += 1;
        } else {
            *d = src[q];
            q += 1;
        }
    }
}

/// The terrain half of `RenderGlobal` (module docs).
#[derive(Clone, Debug, Default)]
pub struct RenderGlobalTerrain {
    /// `worldRenderers`, index `(z * tall + y) * wide + x`.
    pub world_renderers: Vec<WorldRenderer>,
    /// `sortedWorldRenderers`, as indices into `world_renderers`.
    pub sorted_world_renderers: Vec<usize>,
    /// `worldRenderersToUpdate` (`None` only inside `updateRenderers`).
    pub world_renderers_to_update: Vec<Option<usize>>,
    /// `renderChunksWide`.
    pub render_chunks_wide: i32,
    /// `renderChunksTall`.
    pub render_chunks_tall: i32,
    /// `renderChunksDeep`.
    pub render_chunks_deep: i32,
    /// `minBlockX`…`maxBlockZ`: the grid's extent.
    pub min_block: [i32; 3],
    /// See [`RenderGlobalTerrain::min_block`].
    pub max_block: [i32; 3],
    /// `renderDistance` the grid was built for (-1: none).
    pub render_distance: i32,
    /// `renderEntitiesStartupCounter`.
    pub render_entities_startup_counter: i32,
    /// `renderersLoaded`.
    pub renderers_loaded: i32,
    /// `renderersBeingClipped`.
    pub renderers_being_clipped: i32,
    /// `renderersBeingOccluded`.
    pub renderers_being_occluded: i32,
    /// `renderersBeingRendered`.
    pub renderers_being_rendered: i32,
    /// `renderersSkippingRenderPass`.
    pub renderers_skipping_render_pass: i32,
    /// `worldRenderersCheckIndex`.
    pub world_renderers_check_index: i32,
    /// `prevSortX`, `prevSortY`, `prevSortZ`.
    pub prev_sort: [f64; 3],
    /// `frustrumCheckOffset`.
    pub frustrum_check_offset: i32,
    /// `tileEntities`: every special-render tile entity of the built
    /// sections (WP-3.5b draws them in `renderEntities`).
    pub tile_entities: Vec<[i32; 3]>,
    /// `occlusionEnabled` (module docs: false).
    pub occlusion_enabled: bool,
}

impl RenderGlobalTerrain {
    /// A renderer with no grid (`renderDistance = -1`, the sort anchors at
    /// -9999).
    // src: RenderGlobal.java:38-39, :55-57
    pub fn new() -> Self {
        RenderGlobalTerrain {
            render_distance: -1,
            render_entities_startup_counter: 2,
            prev_sort: [-9999.0; 3],
            ..Default::default()
        }
    }

    /// The grid width of a render distance: `64 << (3 - renderDistance)`
    /// blocks, at most 400, `/ 16 + 1` sections.
    // src: RenderGlobal.java:196-203
    pub fn chunks_wide(render_distance: i32) -> i32 {
        let mut var1 = 64 << (3 - render_distance);
        if var1 > 400 {
            var1 = 400;
        }
        var1 / 16 + 1
    }

    /// `loadRenderers()` (without the leaves' `setGraphicsLevel`, which the
    /// caller does on the world): a new grid for `render_distance`, every
    /// renderer dirty, positioned and sorted around `view` (the render
    /// view entity's `posX/Y/Z`) if any.
    // src: RenderGlobal.java:186-252
    pub fn load_renderers(&mut self, render_distance: i32, view: Option<[f64; 3]>) {
        self.render_distance = render_distance;
        let w = Self::chunks_wide(render_distance);
        self.render_chunks_wide = w;
        self.render_chunks_tall = 8;
        self.render_chunks_deep = w;
        let n =
            (self.render_chunks_wide * self.render_chunks_tall * self.render_chunks_deep) as usize;
        self.min_block = [0; 3];
        self.max_block = [
            self.render_chunks_wide,
            self.render_chunks_tall,
            self.render_chunks_deep,
        ];
        self.world_renderers_to_update.clear();
        self.tile_entities.clear();
        let mut world_renderers: Vec<Option<WorldRenderer>> = vec![None; n];
        let mut sorted = vec![0usize; n];
        let mut var3 = 0i32;
        for var4 in 0..self.render_chunks_wide {
            for var5 in 0..self.render_chunks_tall {
                for var6 in 0..self.render_chunks_deep {
                    let i = ((var6 * self.render_chunks_tall + var5) * self.render_chunks_wide
                        + var4) as usize;
                    let mut r = WorldRenderer::new(var4 * 16, var5 * 16, var6 * 16, 16);
                    r.is_waiting_on_occlusion_query = false;
                    r.is_visible = true;
                    r.is_in_frustum = true;
                    r.chunk_index = var3;
                    var3 += 1;
                    r.mark_dirty();
                    world_renderers[i] = Some(r);
                    sorted[i] = i;
                    self.world_renderers_to_update.push(Some(i));
                }
            }
        }
        self.world_renderers = world_renderers
            .into_iter()
            .map(|r| r.expect("filled"))
            .collect();
        self.sorted_world_renderers = sorted;
        if let Some(p) = view {
            self.mark_renderers_for_new_position(
                floor_double(p[0]),
                floor_double(p[1]),
                floor_double(p[2]),
            );
            self.sort_by_entity(p);
        }
        self.render_entities_startup_counter = 2;
    }

    /// `Arrays.sort(sortedWorldRenderers, new EntitySorter(view))`
    /// ([`java_legacy_sort`] with [`entity_sorter_compare`]).
    // src: RenderGlobal.java:247, :418
    pub fn sort_by_entity(&mut self, pos: [f64; 3]) {
        let wr = &self.world_renderers;
        java_legacy_sort(&mut self.sorted_world_renderers, |&a, &b| {
            entity_sorter_compare(&wr[a], &wr[b], pos)
        });
    }

    /// `markRenderersForNewPosition(x, y, z)`: each grid column moves to
    /// the position nearest `(x - 8, z - 8)` with its index modulo the grid
    /// (module docs); a renderer that became dirty joins the update list.
    // src: RenderGlobal.java:317-384
    pub fn mark_renderers_for_new_position(&mut self, mut var1: i32, mut var2: i32, mut var3: i32) {
        var1 -= 8;
        var2 -= 8;
        var3 -= 8;
        let _ = var2;
        self.min_block = [i32::MAX; 3];
        self.max_block = [i32::MIN; 3];
        let var4 = self.render_chunks_wide * 16;
        let var5 = var4 / 2;
        for var6 in 0..self.render_chunks_wide {
            let mut var7 = var6 * 16;
            let mut var8 = var7 + var5 - var1;
            if var8 < 0 {
                var8 -= var4 - 1;
            }
            var8 /= var4;
            var7 -= var8 * var4;
            self.min_block[0] = self.min_block[0].min(var7);
            self.max_block[0] = self.max_block[0].max(var7);
            for var9 in 0..self.render_chunks_deep {
                let mut var10 = var9 * 16;
                let mut var11 = var10 + var5 - var3;
                if var11 < 0 {
                    var11 -= var4 - 1;
                }
                var11 /= var4;
                var10 -= var11 * var4;
                self.min_block[2] = self.min_block[2].min(var10);
                self.max_block[2] = self.max_block[2].max(var10);
                for var12 in 0..self.render_chunks_tall {
                    let var13 = var12 * 16;
                    self.min_block[1] = self.min_block[1].min(var13);
                    self.max_block[1] = self.max_block[1].max(var13);
                    let i = ((var9 * self.render_chunks_tall + var12) * self.render_chunks_wide
                        + var6) as usize;
                    let var14 = &mut self.world_renderers[i];
                    let var15 = var14.needs_update;
                    var14.set_position(var7, var13, var10);
                    if !var15 && var14.needs_update {
                        self.world_renderers_to_update.push(Some(i));
                    }
                }
            }
        }
    }

    /// The renderer index of the section holding block column `(bx, by,
    /// bz)` (`bucketInt(_, 16)` modulo the grid), as `func_949_a` finds it.
    // src: RenderGlobal.java:1200-1225
    pub fn renderer_index(&self, sx: i32, sy: i32, sz: i32) -> usize {
        let mut var14 = sx % self.render_chunks_wide;
        if var14 < 0 {
            var14 += self.render_chunks_wide;
        }
        let mut var16 = sy % self.render_chunks_tall;
        if var16 < 0 {
            var16 += self.render_chunks_tall;
        }
        let mut var18 = sz % self.render_chunks_deep;
        if var18 < 0 {
            var18 += self.render_chunks_deep;
        }
        ((var18 * self.render_chunks_tall + var16) * self.render_chunks_wide + var14) as usize
    }

    /// `func_949_a(x0, y0, z0, x1, y1, z1)` (later MCP:
    /// `markBlocksForUpdate`): every renderer of the sections the box
    /// touches is marked dirty and queued, once.
    // src: RenderGlobal.java:1199-1235
    pub fn func_949_a(&mut self, x0: i32, y0: i32, z0: i32, x1: i32, y1: i32, z1: i32) {
        if self.world_renderers.is_empty() {
            return;
        }
        let var7 = bucket_int(x0, 16);
        let var8 = bucket_int(y0, 16);
        let var9 = bucket_int(z0, 16);
        let var10 = bucket_int(x1, 16);
        let var11 = bucket_int(y1, 16);
        let var12 = bucket_int(z1, 16);
        for var13 in var7..=var10 {
            for var15 in var8..=var11 {
                for var17 in var9..=var12 {
                    let var19 = self.renderer_index(var13, var15, var17);
                    if !self.world_renderers[var19].needs_update {
                        self.world_renderers_to_update.push(Some(var19));
                        self.world_renderers[var19].mark_dirty();
                    }
                }
            }
        }
    }

    /// `markBlockAndNeighborsNeedsUpdate(x, y, z)`.
    // src: RenderGlobal.java:1237-1239
    pub fn mark_block_and_neighbors_needs_update(&mut self, x: i32, y: i32, z: i32) {
        self.func_949_a(x - 1, y - 1, z - 1, x + 1, y + 1, z + 1);
    }

    /// `markBlockRangeNeedsUpdate(x0, y0, z0, x1, y1, z1)`.
    // src: RenderGlobal.java:1241-1243
    pub fn mark_block_range_needs_update(
        &mut self,
        x0: i32,
        y0: i32,
        z0: i32,
        x1: i32,
        y1: i32,
        z1: i32,
    ) {
        self.func_949_a(x0 - 1, y0 - 1, z0 - 1, x1 + 1, y1 + 1, z1 + 1);
    }

    /// `updateAllRenderers()`: the sky light changed; every renderer that
    /// read a sky light above 0 is rebuilt.
    // src: RenderGlobal.java:1341-1349
    pub fn update_all_renderers(&mut self) {
        for i in 0..self.world_renderers.len() {
            let r = &mut self.world_renderers[i];
            if r.is_chunk_lit && !r.needs_update {
                self.world_renderers_to_update.push(Some(i));
                self.world_renderers[i].mark_dirty();
            }
        }
    }

    /// `clipRenderersByFrustrum(frustum, partial)`: a renderer in the
    /// frustum is re-tested every 16th frame (staggered by index), one
    /// outside every frame; empty ones never.
    // src: RenderGlobal.java:1245-1253
    pub fn clip_renderers_by_frustrum(&mut self, f: &Frustrum) {
        for (var3, r) in self.world_renderers.iter_mut().enumerate() {
            if !r.skip_all_render_passes()
                && (!r.is_in_frustum
                    || ((var3 as i32).wrapping_add(self.frustrum_check_offset) & 15) == 0)
            {
                r.update_in_frustrum(f);
            }
        }
        self.frustrum_check_offset = self.frustrum_check_offset.wrapping_add(1);
    }

    /// `updateRenderers(view, var2)` (module docs): returns whether the list
    /// was emptied and the renderers to rebuild now, in the Java's call
    /// order (the near ones best first, then the far one). The caller runs
    /// `updateRenderer()` on each and clears its `needsUpdate`.
    // src: RenderGlobal.java:924-1066
    pub fn update_renderers(&mut self, view: [f64; 3], var2: bool) -> (bool, Vec<usize>) {
        const VAR4: usize = 2;
        let mut var6: [Option<usize>; VAR4] = [None; VAR4];
        let mut var7: Option<Vec<usize>> = None;
        let var8 = self.world_renderers_to_update.len();
        let mut var9 = 0usize;
        let mut out = Vec::new();
        'label169: for var10 in 0..var8 {
            let Some(var11) = self.world_renderers_to_update[var10] else {
                continue;
            };
            if !var2 {
                if self.world_renderers[var11].distance_to_entity_squared(view) > 256.0 {
                    let mut var12: i32 = 0;
                    while (var12 as usize) < VAR4
                        && (match var6[var12 as usize] {
                            None => true,
                            Some(s) => {
                                render_sorter_compare(
                                    &self.world_renderers[s],
                                    &self.world_renderers[var11],
                                    view,
                                ) <= 0
                            }
                        })
                    {
                        var12 += 1;
                    }
                    var12 -= 1;
                    if var12 <= 0 {
                        continue;
                    }
                    let mut var13 = var12;
                    loop {
                        var13 -= 1;
                        if var13 == 0 {
                            // vanilla bug (module docs): slot 0 is never filled.
                            var6[var12 as usize] = Some(var11);
                            continue 'label169;
                        }
                        var6[(var13 - 1) as usize] = var6[var13 as usize];
                    }
                }
            } else if !self.world_renderers[var11].is_in_frustum {
                continue;
            }
            var7.get_or_insert_with(Vec::new).push(var11);
            var9 += 1;
            self.world_renderers_to_update[var10] = None;
        }
        if let Some(mut var7) = var7 {
            if var7.len() > 1 {
                let wr = &self.world_renderers;
                var7.sort_by(|&a, &b| {
                    if a == b {
                        std::cmp::Ordering::Equal
                    } else {
                        render_sorter_compare(&wr[a], &wr[b], view).cmp(&0)
                    }
                });
            }
            for var10 in (0..var7.len()).rev() {
                out.push(var7[var10]);
            }
        }
        let mut var10 = 0usize;
        for var21 in (0..VAR4).rev() {
            if let Some(var22) = var6[var21] {
                if !self.world_renderers[var22].is_in_frustum && var21 != VAR4 - 1 {
                    var6[var21] = None;
                    var6[0] = None;
                    break;
                }
                out.push(var22);
                var10 += 1;
            }
        }
        let mut var21 = 0usize;
        let mut var12 = 0usize;
        let var13 = self.world_renderers_to_update.len();
        while var21 != var13 {
            if let Some(var14) = self.world_renderers_to_update[var21] {
                let var15 = var6.contains(&Some(var14));
                if !var15 {
                    if var12 != var21 {
                        self.world_renderers_to_update[var12] = Some(var14);
                    }
                    var12 += 1;
                }
            }
            var21 += 1;
        }
        self.world_renderers_to_update.truncate(var12);
        (var8 == var9 + var10, out)
    }

    /// `sortAndRender(view, pass, partial)`'s bookkeeping, without the
    /// `renderDistance` reload (the caller's): the 10-renderer check, the
    /// counters' reset (pass 0), the re-sort when the view moved more than
    /// 4 blocks (`view` is `posX/Y/Z`), and `renderSortedRenderers`'
    /// counters; returns how many sections draw the pass.
    // src: RenderGlobal.java:386-511, :529-587
    pub fn sort_and_render(&mut self, view: [f64; 3], pass: usize) -> i32 {
        let len = self.world_renderers.len();
        if len == 0 {
            return 0;
        }
        for _ in 0..10 {
            self.world_renderers_check_index = (self.world_renderers_check_index + 1) % len as i32;
            let i = self.world_renderers_check_index as usize;
            if self.world_renderers[i].needs_update
                && !self.world_renderers_to_update.contains(&Some(i))
            {
                self.world_renderers_to_update.push(Some(i));
            }
        }
        if pass == 0 {
            self.renderers_loaded = 0;
            self.renderers_being_clipped = 0;
            self.renderers_being_occluded = 0;
            self.renderers_being_rendered = 0;
            self.renderers_skipping_render_pass = 0;
        }
        let var11 = view[0] - self.prev_sort[0];
        let var13 = view[1] - self.prev_sort[1];
        let var15 = view[2] - self.prev_sort[2];
        if var11 * var11 + var13 * var13 + var15 * var15 > 16.0 {
            self.prev_sort = view;
            self.mark_renderers_for_new_position(
                floor_double(view[0]),
                floor_double(view[1]),
                floor_double(view[2]),
            );
            self.sort_by_entity(view);
        }
        let mut var6 = 0;
        for &i in &self.sorted_world_renderers {
            let r = &self.world_renderers[i];
            if pass == 0 {
                self.renderers_loaded += 1;
                if r.skip_render_pass[pass] {
                    self.renderers_skipping_render_pass += 1;
                } else if !r.is_in_frustum {
                    self.renderers_being_clipped += 1;
                } else if self.occlusion_enabled && !r.is_visible {
                    self.renderers_being_occluded += 1;
                } else {
                    self.renderers_being_rendered += 1;
                }
            }
            if r.draws_pass(pass) && (!self.occlusion_enabled || r.is_visible) {
                var6 += 1;
            }
        }
        var6
    }

    /// The order `sortAndRender(view, pass)` calls the pass's display lists
    /// in (`renderSortedRenderers`): the drawn sections in
    /// `sortedWorldRenderers` order (nearest first), each added to the
    /// `RenderList` of its 1024-block anchor (`posXMinus`…; the last list
    /// with that anchor, else a new one), the lists then called one after
    /// another (`renderAllRenderLists`). Vanilla has four lists and would
    /// throw on a fifth anchor, which no render distance reaches.
    // src: RenderGlobal.java:529-587, RenderList.java:17-56
    pub fn render_list_order(&self, pass: usize) -> Vec<usize> {
        let mut anchors: Vec<[i32; 3]> = Vec::new();
        let mut lists: Vec<Vec<usize>> = Vec::new();
        for &i in &self.sorted_world_renderers {
            let r = &self.world_renderers[i];
            if !(r.draws_pass(pass) && (!self.occlusion_enabled || r.is_visible)) {
                continue;
            }
            let mut var17 = None;
            for (var18, a) in anchors.iter().enumerate() {
                if *a == r.pos_minus {
                    var17 = Some(var18);
                }
            }
            let k = match var17 {
                Some(k) => k,
                None => {
                    anchors.push(r.pos_minus);
                    lists.push(Vec::new());
                    anchors.len() - 1
                }
            };
            lists[k].push(i);
        }
        lists.into_iter().flatten().collect()
    }

    /// `getDebugInfoRenders()`: the F3 line.
    // src: RenderGlobal.java:309-311
    pub fn get_debug_info_renders(&self) -> String {
        format!(
            "C: {}/{}. F: {}, O: {}, E: {}",
            self.renderers_being_rendered,
            self.renderers_loaded,
            self.renderers_being_clipped,
            self.renderers_being_occluded,
            self.renderers_skipping_render_pass
        )
    }

    /// The tile-entity half of `updateRenderer`: the section's new list
    /// replaces its old one, and `tileEntities` gains the new ones and loses
    /// the gone ones (`HashSet` differences).
    // src: WorldRenderer.java:96-98, :163-168
    pub fn set_tile_entity_renderers(&mut self, i: usize, list: Vec<[i32; 3]>) {
        let old = std::mem::replace(&mut self.world_renderers[i].tile_entity_renderers, list);
        let new = &self.world_renderers[i].tile_entity_renderers;
        for p in new {
            if !old.contains(p) && !self.tile_entities.contains(p) {
                self.tile_entities.push(*p);
            }
        }
        for p in &old {
            if !new.contains(p) {
                self.tile_entities.retain(|q| q != p);
            }
        }
    }
}

/// What a rebuild needs besides the snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SectionSettings {
    /// `Minecraft.isAmbientOcclusionEnabled()`.
    pub ambient_occlusion: bool,
    /// `EntityRenderer.anaglyphEnable`.
    pub anaglyph: bool,
    /// The translucent pass is drawn flat-shaded: with AO on and fast
    /// graphics, `renderWorld` leaves `glShadeModel(GL_FLAT)` from after
    /// pass 0 when it draws pass 1 (EntityRenderer.java:499, :542-543).
    pub flat_translucent: bool,
}

/// The result of `updateRenderer` for one section.
#[derive(Debug)]
pub struct SectionMeshes {
    /// The two passes' draws (`None` where nothing was started).
    pub passes: [Option<Tessellator>; 2],
    /// `skipRenderPass`.
    pub skip_render_pass: [bool; 2],
    /// `isChunkLit`.
    pub is_chunk_lit: bool,
    /// `tileEntityRenderers`.
    pub tile_entity_renderers: Vec<[i32; 3]>,
}

/// `WorldRenderer.updateRenderer()`'s loop on a snapshot of the section at
/// `pos`: pass 0, then pass 1 if a block of it was seen; per block in y, z,
/// x order, `renderBlockByRenderType` when its `getRenderBlockPass()` is
/// the pass. Vertices are relative to `pos` (`setTranslationD(-posX, …)`).
// src: WorldRenderer.java:81-172
pub fn update_renderer(
    s: &ChunkSnapshot,
    pos: [i32; 3],
    settings: SectionSettings,
) -> SectionMeshes {
    let (var1, var2, var3) = (pos[0], pos[1], pos[2]);
    let (var4, var5, var6) = (var1 + 16, var2 + 16, var3 + 16);
    let mut skip = [true; 2];
    s.take_is_lit();
    let mut var10 = RenderBlocks::new(
        s,
        s.statics.clone(),
        settings.ambient_occlusion,
        settings.anaglyph,
    );
    let mut passes: [Option<Tessellator>; 2] = [None, None];
    for var11 in 0..2usize {
        let mut var12 = false;
        let mut var13 = false;
        let mut var14 = false;
        for var15 in var2..var5 {
            for var16 in var3..var6 {
                for var17 in var1..var4 {
                    let var18 = s.get_block_id(var17, var15, var16);
                    if var18 > 0 {
                        if !var14 {
                            var14 = true;
                            var10.tess.start_drawing_quads();
                            var10.tess.set_translation_d(
                                -(var1 as f64),
                                -(var2 as f64),
                                -(var3 as f64),
                            );
                        }
                        if blocks::def(var18).is_none() {
                            // Block.blocksList[id] is null: the Java throws.
                            continue;
                        }
                        let var20 = blocks::get_render_block_pass(var18) as usize;
                        if var20 != var11 {
                            var12 = true;
                        } else {
                            var13 |= var10.render_block_by_render_type(var18, var17, var15, var16);
                        }
                    }
                }
            }
        }
        if var14 {
            passes[var11] = Some(std::mem::take(&mut var10.tess));
            var10.tess.set_translation_d(0.0, 0.0, 0.0);
        } else {
            var13 = false;
        }
        if var13 {
            skip[var11] = false;
        }
        if !var12 {
            break;
        }
    }
    SectionMeshes {
        passes,
        skip_render_pass: skip,
        is_chunk_lit: s.take_is_lit(),
        tile_entity_renderers: s.special_tile_entities.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mc_sim::world::provider::{Dimension, WorldProvider};

    #[test]
    fn grid_sizes_by_render_distance() {
        assert_eq!(RenderGlobalTerrain::chunks_wide(0), 26);
        assert_eq!(RenderGlobalTerrain::chunks_wide(1), 17);
        assert_eq!(RenderGlobalTerrain::chunks_wide(2), 9);
        assert_eq!(RenderGlobalTerrain::chunks_wide(3), 5);
    }

    #[test]
    fn section_index_math_and_wrapping() {
        let mut g = RenderGlobalTerrain::new();
        g.load_renderers(3, None);
        assert_eq!(g.world_renderers.len(), 5 * 8 * 5);
        // index (z * tall + y) * wide + x; chunkIndex in x, y, z loop order
        let r = &g.world_renderers[(2 * 8 + 3) * 5 + 1];
        assert_eq!([r.pos_x, r.pos_y, r.pos_z], [16, 48, 32]);
        assert_eq!(r.chunk_index, 8 * 5 + 3 * 5 + 2); // x 1, y 3, z 2
        assert_eq!(g.renderer_index(1, 3, 2), (2 * 8 + 3) * 5 + 1);
        assert_eq!(g.renderer_index(-1, 3, 2), (2 * 8 + 3) * 5 + 4);
        assert_eq!(g.renderer_index(6, -1, 7), (2 * 8 + 7) * 5 + 1);
        // every renderer starts dirty and listed
        assert_eq!(g.world_renderers_to_update.len(), 200);
        assert!(g.world_renderers.iter().all(|r| r.needs_update));
    }

    #[test]
    fn new_position_wraps_the_grid_around_the_view() {
        let mut g = RenderGlobalTerrain::new();
        g.load_renderers(3, Some([1000.5, 70.0, -500.25]));
        // 80 blocks wide: columns at 1000-8-40 .. +40, i.e. the multiple of
        // 16 of each column index closest to the view
        let xs: std::collections::BTreeSet<i32> =
            g.world_renderers.iter().map(|r| r.pos_x).collect();
        let zs: std::collections::BTreeSet<i32> =
            g.world_renderers.iter().map(|r| r.pos_z).collect();
        assert_eq!(
            xs.into_iter().collect::<Vec<_>>(),
            vec![960, 976, 992, 1008, 1024]
        );
        assert_eq!(
            zs.into_iter().collect::<Vec<_>>(),
            vec![-544, -528, -512, -496, -480]
        );
        // each sits at its index modulo the grid
        for (i, r) in g.world_renderers.iter().enumerate() {
            assert_eq!(
                g.renderer_index(r.pos_x >> 4, r.pos_y >> 4, r.pos_z >> 4),
                i,
                "{:?}",
                [r.pos_x, r.pos_y, r.pos_z]
            );
        }
        // sorted nearest first
        let d: Vec<f64> = g
            .sorted_world_renderers
            .iter()
            .map(|&i| entity_sorter_key(&g.world_renderers[i], [1000.5, 70.0, -500.25]))
            .collect();
        assert!(d.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn legacy_sort_is_stable_and_entity_sorter_quantises() {
        // a stable sort of pairs by key, both below and above the insertion
        // sort threshold
        let mut r = mc_core::java_random::JavaRandom::new(7);
        for n in [0usize, 1, 5, 6, 7, 40, 333] {
            let mut v: Vec<(i32, usize)> = (0..n).map(|i| (r.next_int_bounded(10), i)).collect();
            let mut want = v.clone();
            want.sort_by_key(|p| p.0);
            java_legacy_sort(&mut v, |a, b| a.0 - b.0);
            assert_eq!(v, want, "n {n}");
        }
        // squared distances under 1/1024 apart compare equal
        let a = WorldRenderer::new(0, 0, 0, 16);
        let b = WorldRenderer::new(16, 0, 0, 16);
        // centres 8 and 24 on x: the difference is 32 * (1/65536) = 1/2048
        let p = [16.0 + 1.0 / 65536.0, 8.0, 8.0];
        assert!(entity_sorter_key(&a, p) > entity_sorter_key(&b, p));
        assert_eq!(entity_sorter_compare(&a, &b, p), 0);
        assert!(entity_sorter_compare(&a, &b, [15.0, 8.0, 8.0]) < 0);
        assert!(entity_sorter_compare(&b, &a, [15.0, 8.0, 8.0]) > 0);
    }

    #[test]
    fn marks_find_the_wrapped_renderer_once() {
        let mut g = RenderGlobalTerrain::new();
        g.load_renderers(3, Some([8.0, 64.0, 8.0]));
        g.world_renderers
            .iter_mut()
            .for_each(|r| r.needs_update = false);
        g.world_renderers_to_update.clear();
        g.mark_block_and_neighbors_needs_update(15, 64, 15);
        // x 14..16, y 63..65, z 14..16 → sections x 0..1, y 3..4, z 0..1
        assert_eq!(g.world_renderers_to_update.len(), 8);
        g.mark_block_and_neighbors_needs_update(15, 64, 15);
        assert_eq!(g.world_renderers_to_update.len(), 8);
        // y -1 wraps to the top row (vanilla quirk)
        g.mark_block_and_neighbors_needs_update(40, 0, 40);
        let top = g.renderer_index(2, 7, 2);
        assert!(g.world_renderers_to_update.contains(&Some(top)));
    }

    #[test]
    fn pass_one_lists_nearest_first_grouped_by_anchor() {
        // A view by the x = 1024 anchor line: sections on both sides.
        let view = [1020.0, 70.0, 500.0];
        let mut g = RenderGlobalTerrain::new();
        g.load_renderers(3, Some(view));
        for r in &mut g.world_renderers {
            r.is_in_frustum = true;
            r.skip_render_pass = [false, r.pos_y != 64];
        }
        let order = g.render_list_order(1);
        assert_eq!(order.len(), 25);
        // the first list is the nearest section's anchor, then the other
        let first = g.world_renderers[order[0]].pos_minus;
        let split = order
            .iter()
            .position(|&i| g.world_renderers[i].pos_minus != first)
            .unwrap();
        assert!(
            order[split..]
                .iter()
                .all(|&i| g.world_renderers[i].pos_minus != first)
        );
        // each list keeps sortedWorldRenderers' order
        let pos = |i: usize| {
            g.sorted_world_renderers
                .iter()
                .position(|&j| j == i)
                .unwrap()
        };
        for part in [&order[..split], &order[split..]] {
            assert!(part.windows(2).all(|w| pos(w[0]) < pos(w[1])));
        }
        // a section out of the frustum or without the pass is not drawn
        g.world_renderers[order[0]].is_in_frustum = false;
        assert_eq!(g.render_list_order(1).len(), 24);
        assert!(g.render_list_order(0).len() == 200 - 1);
    }

    #[test]
    fn update_order_near_first_then_one_far() {
        let mut g = RenderGlobalTerrain::new();
        g.load_renderers(3, Some([8.0, 72.0, 8.0]));
        let view = [8.0, 72.0, 8.0];
        // the moved renderers left the frustum (setDontDraw); count all in
        g.world_renderers
            .iter_mut()
            .for_each(|r| r.is_in_frustum = true);
        let total = g.world_renderers_to_update.len();
        let near: Vec<usize> = (0..g.world_renderers.len())
            .filter(|&i| g.world_renderers[i].distance_to_entity_squared(view) <= 256.0)
            .collect();
        let (done, out) = g.update_renderers(view, false);
        assert!(!done);
        // all near ones (nearest first) and exactly one far one
        assert_eq!(out.len(), near.len() + 1);
        let d: Vec<f32> = out[..near.len()]
            .iter()
            .map(|&i| g.world_renderers[i].distance_to_entity_squared(view))
            .collect();
        assert!(d.windows(2).all(|w| w[0] <= w[1]), "{d:?}");
        let far = *out.last().unwrap();
        assert!(g.world_renderers[far].distance_to_entity_squared(view) > 256.0);
        // the far one is the nearest of the far (all are in the frustum)
        let best = (0..g.world_renderers.len())
            .filter(|&i| g.world_renderers[i].distance_to_entity_squared(view) > 256.0)
            .map(|i| g.world_renderers[i].distance_to_entity_squared(view))
            .fold(f32::MAX, f32::min);
        assert_eq!(
            g.world_renderers[far].distance_to_entity_squared(view),
            best
        );
        assert_eq!(g.world_renderers_to_update.len(), total - out.len());
        assert!(g.world_renderers_to_update.iter().all(Option::is_some));
    }

    #[test]
    fn render_sorter_prefers_the_frustum_then_distance_then_index() {
        let mut a = WorldRenderer::new(0, 0, 0, 16);
        let mut b = WorldRenderer::new(32, 0, 0, 16);
        a.chunk_index = 5;
        b.chunk_index = 3;
        let v = [0.0, 0.0, 0.0];
        a.is_in_frustum = false;
        b.is_in_frustum = true;
        assert_eq!(render_sorter_compare(&a, &b, v), -1);
        a.is_in_frustum = true;
        assert_eq!(render_sorter_compare(&a, &b, v), 1);
        let c = WorldRenderer::new(0, 0, 0, 16);
        let mut c = c;
        c.is_in_frustum = true;
        c.chunk_index = 9;
        assert_eq!(render_sorter_compare(&a, &c, v), 1);
        assert_eq!(render_sorter_compare(&c, &a, v), -1);
    }

    #[test]
    fn sections_mesh_by_pass() {
        let t = WorldProvider::new(Dimension::Surface).light_brightness_table;
        let mut s = ChunkSnapshot::synthetic([-3, -3, -3], [35, 35, 35], t);
        s.set(1, 1, 1, 1, 0, 15, 0); // stone
        s.set(5, 1, 1, 79, 0, 15, 0); // ice: pass 1
        let m = update_renderer(
            &s,
            [0, 0, 0],
            SectionSettings {
                ambient_occlusion: false,
                anaglyph: false,
                flat_translucent: false,
            },
        );
        assert_eq!(m.skip_render_pass, [false, false]);
        assert_eq!(m.passes[0].as_ref().unwrap().len(), 24);
        assert_eq!(m.passes[1].as_ref().unwrap().len(), 24);
        assert!(m.is_chunk_lit);
        // stone only: pass 1 is never started
        s.set(5, 1, 1, 0, 0, 15, 0);
        let m = update_renderer(
            &s,
            [0, 0, 0],
            SectionSettings {
                ambient_occlusion: true,
                anaglyph: false,
                flat_translucent: false,
            },
        );
        assert_eq!(m.skip_render_pass, [false, true]);
        assert!(m.passes[1].is_none());
        // an empty section skips both
        s.set(1, 1, 1, 0, 0, 15, 0);
        let m = update_renderer(
            &s,
            [0, 0, 0],
            SectionSettings {
                ambient_occlusion: true,
                anaglyph: false,
                flat_translucent: false,
            },
        );
        assert_eq!(m.skip_render_pass, [true, true]);
        assert!(m.passes[0].is_none());
        // vertices relative to the section
        s.set(17, 17, 17, 1, 0, 15, 0);
        let m = update_renderer(
            &s,
            [16, 16, 16],
            SectionSettings {
                ambient_occlusion: false,
                anaglyph: false,
                flat_translucent: false,
            },
        );
        let p = &m.passes[0].as_ref().unwrap().positions;
        assert!(
            p.iter().all(|v| v.iter().all(|c| (1.0..=2.0).contains(c))),
            "{p:?}"
        );
    }

    #[test]
    fn tile_entity_lists_follow_the_sections() {
        let mut g = RenderGlobalTerrain::new();
        g.load_renderers(3, None);
        g.set_tile_entity_renderers(0, vec![[1, 2, 3], [4, 5, 6]]);
        assert_eq!(g.tile_entities, vec![[1, 2, 3], [4, 5, 6]]);
        g.set_tile_entity_renderers(0, vec![[4, 5, 6], [7, 8, 9]]);
        assert_eq!(g.tile_entities, vec![[4, 5, 6], [7, 8, 9]]);
    }

    #[test]
    fn debug_line() {
        let mut g = RenderGlobalTerrain::new();
        g.load_renderers(3, Some([8.0, 64.0, 8.0]));
        g.sort_and_render([8.0, 64.0, 8.0], 0);
        // nothing built yet: every renderer skips pass 0
        assert_eq!(g.get_debug_info_renders(), "C: 0/200. F: 0, O: 0, E: 200");
    }
}
