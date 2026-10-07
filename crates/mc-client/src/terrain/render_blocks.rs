//! `RenderBlocks` (RenderBlocks.java): the shared state, the dispatch by
//! render type, the standard cube with and without ambient occlusion, the
//! six face functions, `renderBlockUsingTexture` (the crack overlay, the
//! piston) and `renderBlockOnInventory` (the block drawn as an item).
//!
//! # Render types
//!
//! `renderBlockByRenderType` (RenderBlocks.java:92-132) dispatches on
//! `Block.getRenderType()`:
//!
//! | Type | Java method | Blocks | Owner |
//! |---|---|---|---|
//! | 0 | `renderStandardBlock` | cubes and cuboids | WP-3.3a (here) |
//! | 1 | `renderBlockReed` | saplings, flowers, mushrooms, tall grass, dead bush, reeds, web | WP-3.3c ([`super::plants`]) |
//! | 2 | `renderBlockTorch` | torches, redstone torches | WP-3.3c |
//! | 3 | `renderBlockFire` | fire | WP-3.3c |
//! | 4 | `renderBlockFluids` | water, lava | WP-3.3b ([`super::fluids`]) |
//! | 5 | `renderBlockRedstoneWire` | redstone dust | WP-3.3d ([`super::special`]) |
//! | 6 | `renderBlockCrops` | wheat | WP-3.3c |
//! | 7 | `renderBlockDoor` | doors | WP-3.3d |
//! | 8 | `renderBlockLadder` | ladder | WP-3.3d |
//! | 9 | `renderBlockMinecartTrack` | rails | WP-3.3d |
//! | 10 | `renderBlockStairs` | stairs | WP-3.3d |
//! | 11 | `renderBlockFence` | fence | WP-3.3d |
//! | 12 | `renderBlockLever` | lever | WP-3.3d |
//! | 13 | `renderBlockCactus` | cactus | WP-3.3c |
//! | 14 | `renderBlockBed` | bed | WP-3.3d |
//! | 15 | `renderBlockRepeater` | repeater | WP-3.3d |
//! | 16 | `func_31074_b(…, false)` (the piston base) | pistons | WP-3.3d |
//! | 17 | `func_31080_c(…, true)` (the piston head) | piston extension | WP-3.3d |
//!
//! Each arm calls a method of [`RenderBlocks`] defined in the owner's file;
//! until the owner fills it, it draws nothing and logs once
//! ([`stub_drawn`]). Types -1 (sign, moving piston) are not drawn by
//! `RenderBlocks`.
//!
//! # State
//!
//! The Java keeps the per-block state in fields (the AO light values and
//! corner colours, `overrideBlockTexture`, `flipTexture`, `renderAllFaces`,
//! the six `uvRotate*`) and mutates the `Block` singletons' bounds
//! (`setBlockBounds`, `setBlockBoundsBasedOnState`); here the singletons'
//! state is [`RenderBlocks::statics`], the job's own copy (module docs of
//! [`super::snapshot`]). Arithmetic is the Java's: `float` → `f32`,
//! `double` → `f64`, in its order.

use std::sync::atomic::{AtomicBool, Ordering};

use mc_sim::block::{self, BlockStatics};

use super::blocks::{self, bounds};
use super::snapshot::ChunkSnapshot;
use super::tessellator::Tessellator;
use crate::gl_material::anaglyph;

/// `Block.grass.blockID`.
// src: Block.java:594
const GRASS: i32 = 2;
/// The grass side overlay's terrain index (`renderEastFace(…, 38)` &c.).
// src: RenderBlocks.java:1972, :2274
pub const GRASS_SIDE_OVERLAY: i32 = 38;

/// One "not transcribed yet" line per render type.
static STUB_LOGGED: [AtomicBool; 18] = [const { AtomicBool::new(false) }; 18];

/// A render type whose body belongs to WP-3.3b-d and is not filled yet:
/// logs once per type and returns `false` (nothing drawn).
pub fn stub_drawn(render_type: i32, java: &str) -> bool {
    if let Some(f) = STUB_LOGGED.get(render_type as usize)
        && !f.swap(true, Ordering::Relaxed)
    {
        log::info!("terrain: render type {render_type} ({java}) is not drawn yet");
    }
    false
}

/// `RenderBlocks` (module docs).
#[derive(Debug)]
pub struct RenderBlocks<'a> {
    /// `blockAccess`.
    // src: RenderBlocks.java:7
    pub block_access: &'a ChunkSnapshot,
    /// `Tessellator.instance`.
    pub tess: Tessellator,
    /// The `Block` singletons' mutable state (bounds and the leaves) this
    /// renderer reads and writes.
    pub statics: BlockStatics,
    /// `overrideBlockTexture`: a texture forced on every face (-1: none).
    // src: RenderBlocks.java:8
    pub override_block_texture: i32,
    /// `flipTexture`: swaps the u range of the side faces.
    // src: RenderBlocks.java:9
    pub flip_texture: bool,
    /// `renderAllFaces`: draws faces even against opaque neighbours.
    // src: RenderBlocks.java:10
    pub render_all_faces: bool,
    /// `RenderBlocks.fancyGrass` (a static, never changed: `true`).
    // src: RenderBlocks.java:11
    pub fancy_grass: bool,
    /// `field_31088_b`: `renderBlockOnInventory` sets `glColor` from
    /// `getRenderColor(meta)` (later MCP: `useInventoryTint`).
    // src: RenderBlocks.java:12, :3179-3185
    pub use_inventory_tint: bool,
    /// `field_31087_g`: the texture rotation of `renderEastFace`.
    // src: RenderBlocks.java:13, :2851-2882
    pub uv_rotate_east: i32,
    /// `field_31086_h`: the texture rotation of `renderWestFace`.
    // src: RenderBlocks.java:14, :2940-2971
    pub uv_rotate_west: i32,
    /// `field_31085_i`: the texture rotation of `renderSouthFace`.
    // src: RenderBlocks.java:15, :3118-3149
    pub uv_rotate_south: i32,
    /// `field_31084_j`: the texture rotation of `renderNorthFace`.
    // src: RenderBlocks.java:16, :3029-3060
    pub uv_rotate_north: i32,
    /// `field_31083_k`: the texture rotation of `renderTopFace`.
    // src: RenderBlocks.java:17, :2762-2793
    pub uv_rotate_top: i32,
    /// `field_31082_l`: the texture rotation of `renderBottomFace`.
    // src: RenderBlocks.java:18, :2680-2711
    pub uv_rotate_bottom: i32,
    /// `enableAO`: the face functions set the four corner colours.
    // src: RenderBlocks.java:19
    pub enable_ao: bool,
    /// `lightValueOwn`.
    pub light_value_own: f32,
    /// `aoLightValueXNeg`: the brightness at x - 1.
    pub ao_light_value_x_neg: f32,
    /// `aoLightValueYNeg`.
    pub ao_light_value_y_neg: f32,
    /// `aoLightValueZNeg`.
    pub ao_light_value_z_neg: f32,
    /// `aoLightValueXPos`.
    pub ao_light_value_x_pos: f32,
    /// `aoLightValueYPos`.
    pub ao_light_value_y_pos: f32,
    /// `aoLightValueZPos`.
    pub ao_light_value_z_pos: f32,
    /// `field_22377_m`: brightness at (x-1, y-1, z-1).
    pub field_22377_m: f32,
    /// `field_22376_n`: (x-1, y-1, z).
    pub field_22376_n: f32,
    /// `field_22375_o`: (x-1, y-1, z+1).
    pub field_22375_o: f32,
    /// `field_22374_p`: (x, y-1, z-1).
    pub field_22374_p: f32,
    /// `field_22373_q`: (x, y-1, z+1).
    pub field_22373_q: f32,
    /// `field_22372_r`: (x+1, y-1, z-1).
    pub field_22372_r: f32,
    /// `field_22371_s`: (x+1, y-1, z).
    pub field_22371_s: f32,
    /// `field_22370_t`: (x+1, y-1, z+1).
    pub field_22370_t: f32,
    /// `field_22369_u`: (x-1, y+1, z-1).
    pub field_22369_u: f32,
    /// `field_22368_v`: (x-1, y+1, z).
    pub field_22368_v: f32,
    /// `field_22367_w`: (x-1, y+1, z+1).
    pub field_22367_w: f32,
    /// `field_22366_x`: (x, y+1, z-1).
    pub field_22366_x: f32,
    /// `field_22365_y`: (x+1, y+1, z-1).
    pub field_22365_y: f32,
    /// `field_22364_z`: (x+1, y+1, z).
    pub field_22364_z: f32,
    /// `field_22362_A`: (x, y+1, z+1).
    pub field_22362_a: f32,
    /// `field_22360_B`: (x+1, y+1, z+1).
    pub field_22360_b: f32,
    /// `field_22358_C`: (x-1, y, z-1).
    pub field_22358_c: f32,
    /// `field_22356_D`: (x+1, y, z-1).
    pub field_22356_d: f32,
    /// `field_22354_E`: (x-1, y, z+1).
    pub field_22354_e: f32,
    /// `field_22353_F`: (x+1, y, z+1).
    pub field_22353_f: f32,
    /// `field_22352_G`: `1`, never changed; at 0 or below the AO path
    /// would use the face neighbour's brightness for all four corners.
    // src: RenderBlocks.java:47
    pub field_22352_g: i32,
    /// `colorRed/Green/BlueTopLeft`.
    pub color_top_left: [f32; 3],
    /// `colorRed/Green/BlueBottomLeft`.
    pub color_bottom_left: [f32; 3],
    /// `colorRed/Green/BlueBottomRight`.
    pub color_bottom_right: [f32; 3],
    /// `colorRed/Green/BlueTopRight`.
    pub color_top_right: [f32; 3],
    /// `field_22339_T`: `canBlockGrass` at (x, y+1, z-1).
    pub field_22339_t: bool,
    /// `field_22338_U`: (x+1, y+1, z).
    pub field_22338_u: bool,
    /// `field_22337_V`: (x-1, y+1, z).
    pub field_22337_v: bool,
    /// `field_22336_W`: (x, y+1, z+1).
    pub field_22336_w: bool,
    /// `field_22335_X`: (x-1, y, z-1).
    pub field_22335_x: bool,
    /// `field_22334_Y`: (x+1, y, z+1).
    pub field_22334_y: bool,
    /// `field_22333_Z`: (x-1, y, z+1).
    pub field_22333_z: bool,
    /// `field_22363_aa`: (x+1, y, z-1).
    pub field_22363_aa: bool,
    /// `field_22361_ab`: (x, y-1, z-1).
    pub field_22361_ab: bool,
    /// `field_22359_ac`: (x+1, y-1, z).
    pub field_22359_ac: bool,
    /// `field_22357_ad`: (x-1, y-1, z).
    pub field_22357_ad: bool,
    /// `field_22355_ae`: (x, y-1, z+1).
    pub field_22355_ae: bool,
    /// `Minecraft.isAmbientOcclusionEnabled()` (`gameSettings.ambientOcclusion`).
    // src: RenderBlocks.java:1732, Minecraft.java:1539-1541
    pub ambient_occlusion: bool,
    /// `EntityRenderer.anaglyphEnable`.
    // src: RenderBlocks.java:1723
    pub anaglyph: bool,
}

/// A face corner, in the order the face functions set the colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Corner {
    TopLeft,
    BottomLeft,
    BottomRight,
    TopRight,
}

impl<'a> RenderBlocks<'a> {
    /// `new RenderBlocks(blockAccess)` with the singletons' state `statics`
    /// and the two settings it reads.
    // src: RenderBlocks.java:73-75
    pub fn new(
        block_access: &'a ChunkSnapshot,
        statics: BlockStatics,
        ambient_occlusion: bool,
        anaglyph: bool,
    ) -> Self {
        RenderBlocks {
            block_access,
            tess: Tessellator::default(),
            statics,
            override_block_texture: -1,
            flip_texture: false,
            render_all_faces: false,
            fancy_grass: true,
            use_inventory_tint: true,
            uv_rotate_east: 0,
            uv_rotate_west: 0,
            uv_rotate_south: 0,
            uv_rotate_north: 0,
            uv_rotate_top: 0,
            uv_rotate_bottom: 0,
            enable_ao: false,
            light_value_own: 0.0,
            ao_light_value_x_neg: 0.0,
            ao_light_value_y_neg: 0.0,
            ao_light_value_z_neg: 0.0,
            ao_light_value_x_pos: 0.0,
            ao_light_value_y_pos: 0.0,
            ao_light_value_z_pos: 0.0,
            field_22377_m: 0.0,
            field_22376_n: 0.0,
            field_22375_o: 0.0,
            field_22374_p: 0.0,
            field_22373_q: 0.0,
            field_22372_r: 0.0,
            field_22371_s: 0.0,
            field_22370_t: 0.0,
            field_22369_u: 0.0,
            field_22368_v: 0.0,
            field_22367_w: 0.0,
            field_22366_x: 0.0,
            field_22365_y: 0.0,
            field_22364_z: 0.0,
            field_22362_a: 0.0,
            field_22360_b: 0.0,
            field_22358_c: 0.0,
            field_22356_d: 0.0,
            field_22354_e: 0.0,
            field_22353_f: 0.0,
            field_22352_g: 1,
            color_top_left: [0.0; 3],
            color_bottom_left: [0.0; 3],
            color_bottom_right: [0.0; 3],
            color_top_right: [0.0; 3],
            field_22339_t: false,
            field_22338_u: false,
            field_22337_v: false,
            field_22336_w: false,
            field_22335_x: false,
            field_22334_y: false,
            field_22333_z: false,
            field_22363_aa: false,
            field_22361_ab: false,
            field_22359_ac: false,
            field_22357_ad: false,
            field_22355_ae: false,
            ambient_occlusion,
            anaglyph,
        }
    }

    // ---- the Block methods on this renderer's access and singletons ----------------

    /// `block.getBlockBrightness(blockAccess, x, y, z)`.
    pub fn brightness(&self, id: i32, x: i32, y: i32, z: i32) -> f32 {
        blocks::get_block_brightness(self.block_access, id, x, y, z)
    }

    /// `block.getBlockTexture(blockAccess, x, y, z, side)`.
    pub fn block_texture(&self, id: i32, x: i32, y: i32, z: i32, side: i32) -> i32 {
        blocks::get_block_texture(self.block_access, &self.statics, id, x, y, z, side)
    }

    /// `block.shouldSideBeRendered(blockAccess, x, y, z, side)`.
    pub fn should_side(&self, id: i32, x: i32, y: i32, z: i32, side: i32) -> bool {
        blocks::should_side_be_rendered(self.block_access, &self.statics, id, x, y, z, side)
    }

    /// `block.setBlockBounds(…)`.
    #[allow(clippy::too_many_arguments)]
    pub fn set_block_bounds(
        &mut self,
        id: i32,
        x0: f32,
        y0: f32,
        z0: f32,
        x1: f32,
        y1: f32,
        z1: f32,
    ) {
        blocks::set_block_bounds(&mut self.statics, id, x0, y0, z0, x1, y1, z1);
    }

    // ---- the entry points --------------------------------------------------------------

    /// `renderBlockUsingTexture(block, x, y, z, texture)`: the block drawn
    /// with `texture` on every face (the crack overlay).
    // src: RenderBlocks.java:80-84
    pub fn render_block_using_texture(&mut self, id: i32, x: i32, y: i32, z: i32, tex: i32) {
        self.override_block_texture = tex;
        self.render_block_by_render_type(id, x, y, z);
        self.override_block_texture = -1;
    }

    /// `func_31075_a(block, x, y, z)`: the block drawn with every face
    /// (later MCP: `renderBlockAllFaces`; the moving-piston renderer's).
    // src: RenderBlocks.java:86-90
    pub fn func_31075_a(&mut self, id: i32, x: i32, y: i32, z: i32) {
        self.render_all_faces = true;
        self.render_block_by_render_type(id, x, y, z);
        self.render_all_faces = false;
    }

    /// `renderBlockByRenderType(block, x, y, z)`: `setBlockBoundsBasedOnState`,
    /// then the renderer of `getRenderType()` (module docs); true when it
    /// drew something.
    // src: RenderBlocks.java:92-132
    pub fn render_block_by_render_type(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let var5 = blocks::get_render_type(id);
        blocks::set_block_bounds_based_on_state(self.block_access, &mut self.statics, id, x, y, z);
        match var5 {
            0 => self.render_standard_block(id, x, y, z),
            4 => self.render_block_fluids(id, x, y, z),
            13 => self.render_block_cactus(id, x, y, z),
            1 => self.render_block_reed(id, x, y, z),
            6 => self.render_block_crops(id, x, y, z),
            2 => self.render_block_torch(id, x, y, z),
            3 => self.render_block_fire(id, x, y, z),
            5 => self.render_block_redstone_wire(id, x, y, z),
            8 => self.render_block_ladder(id, x, y, z),
            7 => self.render_block_door(id, x, y, z),
            9 => self.render_block_minecart_track(id, x, y, z),
            10 => self.render_block_stairs(id, x, y, z),
            11 => self.render_block_fence(id, x, y, z),
            12 => self.render_block_lever(id, x, y, z),
            14 => self.render_block_bed(id, x, y, z),
            15 => self.render_block_repeater(id, x, y, z),
            16 => self.func_31074_b(id, x, y, z, false),
            17 => self.func_31080_c(id, x, y, z, true),
            _ => false,
        }
    }

    // ---- standard blocks -------------------------------------------------------------

    /// `renderStandardBlock(block, x, y, z)`: the colour multiplier (with the
    /// anaglyph transform), then the AO or the plain path.
    // src: RenderBlocks.java:1718-1733
    pub fn render_standard_block(&mut self, id: i32, x: i32, y: i32, z: i32) -> bool {
        let var5 = blocks::color_multiplier(self.block_access, id, x, y, z);
        let mut var6 = (var5 >> 16 & 255) as f32 / 255.0;
        let mut var7 = (var5 >> 8 & 255) as f32 / 255.0;
        let mut var8 = (var5 & 255) as f32 / 255.0;
        if self.anaglyph {
            [var6, var7, var8] = anaglyph([var6, var7, var8]);
        }
        if self.ambient_occlusion {
            self.render_standard_block_with_ambient_occlusion(id, x, y, z, var6, var7, var8)
        } else {
            self.render_standard_block_with_color_multiplier(id, x, y, z, var6, var7, var8)
        }
    }

    /// The AO corner colours: every corner `base`, then each multiplied by
    /// its light value (`colorRedTopLeft = … = base; colorRedTopLeft *=
    /// var9; …`).
    fn set_ao_colors(&mut self, base: [f32; 3], tl: f32, bl: f32, br: f32, tr: f32) {
        self.color_top_left = base.map(|c| c * tl);
        self.color_bottom_left = base.map(|c| c * bl);
        self.color_bottom_right = base.map(|c| c * br);
        self.color_top_right = base.map(|c| c * tr);
    }

    /// The grass side overlay's tint: every corner colour times the colour
    /// multiplier.
    fn mul_ao_colors(&mut self, m: [f32; 3]) {
        for c in [
            &mut self.color_top_left,
            &mut self.color_bottom_left,
            &mut self.color_bottom_right,
            &mut self.color_top_right,
        ] {
            *c = [c[0] * m[0], c[1] * m[1], c[2] * m[2]];
        }
    }

    /// `renderStandardBlockWithAmbientOcclusion(block, x, y, z, r, g, b)`:
    /// per face, each corner's light is the average of the face
    /// neighbour's and three around it (an edge pair counts its corner only
    /// when one of the two edge blocks lets grass grow, `canBlockGrass`,
    /// i.e. is transparent); the face colours are 0.5 (bottom), 1.0 (top),
    /// 0.8 (z faces), 0.6 (x faces) times the colour multiplier (grass:
    /// only the top, and the side overlay 38).
    // src: RenderBlocks.java:1735-2208
    // The four corner values start as `lightValueOwn` (the previous block's)
    // and every face overwrites them before reading: kept as the Java has it.
    #[allow(clippy::too_many_arguments, unused_assignments)]
    pub fn render_standard_block_with_ambient_occlusion(
        &mut self,
        id: i32,
        x: i32,
        y: i32,
        z: i32,
        var5: f32,
        var6: f32,
        var7: f32,
    ) -> bool {
        let (mut var2, mut var3, mut var4) = (x, y, z);
        let s = self.block_access;
        let br = |x, y, z| blocks::get_block_brightness(s, id, x, y, z);
        let cg = |x, y, z| blocks::can_block_grass(s.get_block_id(x, y, z));
        self.enable_ao = true;
        let mut var8 = false;
        let mut var9 = self.light_value_own;
        let mut var10 = self.light_value_own;
        let mut var11 = self.light_value_own;
        let mut var12 = self.light_value_own;
        let mut var13 = true;
        let var14 = true;
        let mut var15 = true;
        let mut var16 = true;
        let mut var17 = true;
        let mut var18 = true;
        self.light_value_own = br(var2, var3, var4);
        self.ao_light_value_x_neg = br(var2 - 1, var3, var4);
        self.ao_light_value_y_neg = br(var2, var3 - 1, var4);
        self.ao_light_value_z_neg = br(var2, var3, var4 - 1);
        self.ao_light_value_x_pos = br(var2 + 1, var3, var4);
        self.ao_light_value_y_pos = br(var2, var3 + 1, var4);
        self.ao_light_value_z_pos = br(var2, var3, var4 + 1);
        self.field_22338_u = cg(var2 + 1, var3 + 1, var4);
        self.field_22359_ac = cg(var2 + 1, var3 - 1, var4);
        self.field_22334_y = cg(var2 + 1, var3, var4 + 1);
        self.field_22363_aa = cg(var2 + 1, var3, var4 - 1);
        self.field_22337_v = cg(var2 - 1, var3 + 1, var4);
        self.field_22357_ad = cg(var2 - 1, var3 - 1, var4);
        self.field_22335_x = cg(var2 - 1, var3, var4 - 1);
        self.field_22333_z = cg(var2 - 1, var3, var4 + 1);
        self.field_22336_w = cg(var2, var3 + 1, var4 + 1);
        self.field_22339_t = cg(var2, var3 + 1, var4 - 1);
        self.field_22355_ae = cg(var2, var3 - 1, var4 + 1);
        self.field_22361_ab = cg(var2, var3 - 1, var4 - 1);
        let index_in_texture =
            blocks::def(id).map_or(-1, |b| block::block_index_in_texture(b, &self.statics));
        if index_in_texture == 3 {
            var18 = false;
            var17 = false;
            var16 = false;
            var15 = false;
            var13 = false;
        }
        if self.override_block_texture >= 0 {
            var18 = false;
            var17 = false;
            var16 = false;
            var15 = false;
            var13 = false;
        }
        let tint = |on: bool| if on { [var5, var6, var7] } else { [1.0; 3] };

        // Bottom (y - 1).
        if self.render_all_faces || self.should_side(id, var2, var3 - 1, var4, 0) {
            if self.field_22352_g <= 0 {
                var9 = self.ao_light_value_y_neg;
                var10 = var9;
                var11 = var9;
                var12 = var9;
            } else {
                var3 -= 1;
                self.field_22376_n = br(var2 - 1, var3, var4);
                self.field_22374_p = br(var2, var3, var4 - 1);
                self.field_22373_q = br(var2, var3, var4 + 1);
                self.field_22371_s = br(var2 + 1, var3, var4);
                self.field_22377_m = if !self.field_22361_ab && !self.field_22357_ad {
                    self.field_22376_n
                } else {
                    br(var2 - 1, var3, var4 - 1)
                };
                self.field_22375_o = if !self.field_22355_ae && !self.field_22357_ad {
                    self.field_22376_n
                } else {
                    br(var2 - 1, var3, var4 + 1)
                };
                self.field_22372_r = if !self.field_22361_ab && !self.field_22359_ac {
                    self.field_22371_s
                } else {
                    br(var2 + 1, var3, var4 - 1)
                };
                self.field_22370_t = if !self.field_22355_ae && !self.field_22359_ac {
                    self.field_22371_s
                } else {
                    br(var2 + 1, var3, var4 + 1)
                };
                var3 += 1;
                var9 = (self.field_22375_o
                    + self.field_22376_n
                    + self.field_22373_q
                    + self.ao_light_value_y_neg)
                    / 4.0;
                var12 = (self.field_22373_q
                    + self.ao_light_value_y_neg
                    + self.field_22370_t
                    + self.field_22371_s)
                    / 4.0;
                var11 = (self.ao_light_value_y_neg
                    + self.field_22374_p
                    + self.field_22371_s
                    + self.field_22372_r)
                    / 4.0;
                var10 = (self.field_22376_n
                    + self.field_22377_m
                    + self.ao_light_value_y_neg
                    + self.field_22374_p)
                    / 4.0;
            }
            let base = tint(var13).map(|c| c * 0.5);
            self.set_ao_colors(base, var9, var10, var11, var12);
            let t = self.block_texture(id, var2, var3, var4, 0);
            self.render_bottom_face(id, var2 as f64, var3 as f64, var4 as f64, t);
            var8 = true;
        }

        // Top (y + 1).
        if self.render_all_faces || self.should_side(id, var2, var3 + 1, var4, 1) {
            if self.field_22352_g <= 0 {
                var9 = self.ao_light_value_y_pos;
                var10 = var9;
                var11 = var9;
                var12 = var9;
            } else {
                var3 += 1;
                self.field_22368_v = br(var2 - 1, var3, var4);
                self.field_22364_z = br(var2 + 1, var3, var4);
                self.field_22366_x = br(var2, var3, var4 - 1);
                self.field_22362_a = br(var2, var3, var4 + 1);
                self.field_22369_u = if !self.field_22339_t && !self.field_22337_v {
                    self.field_22368_v
                } else {
                    br(var2 - 1, var3, var4 - 1)
                };
                self.field_22365_y = if !self.field_22339_t && !self.field_22338_u {
                    self.field_22364_z
                } else {
                    br(var2 + 1, var3, var4 - 1)
                };
                self.field_22367_w = if !self.field_22336_w && !self.field_22337_v {
                    self.field_22368_v
                } else {
                    br(var2 - 1, var3, var4 + 1)
                };
                self.field_22360_b = if !self.field_22336_w && !self.field_22338_u {
                    self.field_22364_z
                } else {
                    br(var2 + 1, var3, var4 + 1)
                };
                var3 -= 1;
                var12 = (self.field_22367_w
                    + self.field_22368_v
                    + self.field_22362_a
                    + self.ao_light_value_y_pos)
                    / 4.0;
                var9 = (self.field_22362_a
                    + self.ao_light_value_y_pos
                    + self.field_22360_b
                    + self.field_22364_z)
                    / 4.0;
                var10 = (self.ao_light_value_y_pos
                    + self.field_22366_x
                    + self.field_22364_z
                    + self.field_22365_y)
                    / 4.0;
                var11 = (self.field_22368_v
                    + self.field_22369_u
                    + self.ao_light_value_y_pos
                    + self.field_22366_x)
                    / 4.0;
            }
            let base = tint(var14);
            self.set_ao_colors(base, var9, var10, var11, var12);
            let t = self.block_texture(id, var2, var3, var4, 1);
            self.render_top_face(id, var2 as f64, var3 as f64, var4 as f64, t);
            var8 = true;
        }

        // East (z - 1).
        if self.render_all_faces || self.should_side(id, var2, var3, var4 - 1, 2) {
            if self.field_22352_g <= 0 {
                var9 = self.ao_light_value_z_neg;
                var10 = var9;
                var11 = var9;
                var12 = var9;
            } else {
                var4 -= 1;
                self.field_22358_c = br(var2 - 1, var3, var4);
                self.field_22374_p = br(var2, var3 - 1, var4);
                self.field_22366_x = br(var2, var3 + 1, var4);
                self.field_22356_d = br(var2 + 1, var3, var4);
                self.field_22377_m = if !self.field_22335_x && !self.field_22361_ab {
                    self.field_22358_c
                } else {
                    br(var2 - 1, var3 - 1, var4)
                };
                self.field_22369_u = if !self.field_22335_x && !self.field_22339_t {
                    self.field_22358_c
                } else {
                    br(var2 - 1, var3 + 1, var4)
                };
                self.field_22372_r = if !self.field_22363_aa && !self.field_22361_ab {
                    self.field_22356_d
                } else {
                    br(var2 + 1, var3 - 1, var4)
                };
                self.field_22365_y = if !self.field_22363_aa && !self.field_22339_t {
                    self.field_22356_d
                } else {
                    br(var2 + 1, var3 + 1, var4)
                };
                var4 += 1;
                var9 = (self.field_22358_c
                    + self.field_22369_u
                    + self.ao_light_value_z_neg
                    + self.field_22366_x)
                    / 4.0;
                var10 = (self.ao_light_value_z_neg
                    + self.field_22366_x
                    + self.field_22356_d
                    + self.field_22365_y)
                    / 4.0;
                var11 = (self.field_22374_p
                    + self.ao_light_value_z_neg
                    + self.field_22372_r
                    + self.field_22356_d)
                    / 4.0;
                var12 = (self.field_22377_m
                    + self.field_22358_c
                    + self.field_22374_p
                    + self.ao_light_value_z_neg)
                    / 4.0;
            }
            let base = tint(var15).map(|c| c * 0.8);
            self.set_ao_colors(base, var9, var10, var11, var12);
            let var19 = self.block_texture(id, var2, var3, var4, 2);
            self.render_east_face(id, var2 as f64, var3 as f64, var4 as f64, var19);
            if self.fancy_grass && var19 == 3 && self.override_block_texture < 0 {
                self.mul_ao_colors([var5, var6, var7]);
                self.render_east_face(
                    id,
                    var2 as f64,
                    var3 as f64,
                    var4 as f64,
                    GRASS_SIDE_OVERLAY,
                );
            }
            var8 = true;
        }

        // West (z + 1).
        if self.render_all_faces || self.should_side(id, var2, var3, var4 + 1, 3) {
            if self.field_22352_g <= 0 {
                var9 = self.ao_light_value_z_pos;
                var10 = var9;
                var11 = var9;
                var12 = var9;
            } else {
                var4 += 1;
                self.field_22354_e = br(var2 - 1, var3, var4);
                self.field_22353_f = br(var2 + 1, var3, var4);
                self.field_22373_q = br(var2, var3 - 1, var4);
                self.field_22362_a = br(var2, var3 + 1, var4);
                self.field_22375_o = if !self.field_22333_z && !self.field_22355_ae {
                    self.field_22354_e
                } else {
                    br(var2 - 1, var3 - 1, var4)
                };
                self.field_22367_w = if !self.field_22333_z && !self.field_22336_w {
                    self.field_22354_e
                } else {
                    br(var2 - 1, var3 + 1, var4)
                };
                self.field_22370_t = if !self.field_22334_y && !self.field_22355_ae {
                    self.field_22353_f
                } else {
                    br(var2 + 1, var3 - 1, var4)
                };
                self.field_22360_b = if !self.field_22334_y && !self.field_22336_w {
                    self.field_22353_f
                } else {
                    br(var2 + 1, var3 + 1, var4)
                };
                var4 -= 1;
                var9 = (self.field_22354_e
                    + self.field_22367_w
                    + self.ao_light_value_z_pos
                    + self.field_22362_a)
                    / 4.0;
                var12 = (self.ao_light_value_z_pos
                    + self.field_22362_a
                    + self.field_22353_f
                    + self.field_22360_b)
                    / 4.0;
                var11 = (self.field_22373_q
                    + self.ao_light_value_z_pos
                    + self.field_22370_t
                    + self.field_22353_f)
                    / 4.0;
                var10 = (self.field_22375_o
                    + self.field_22354_e
                    + self.field_22373_q
                    + self.ao_light_value_z_pos)
                    / 4.0;
            }
            let base = tint(var16).map(|c| c * 0.8);
            self.set_ao_colors(base, var9, var10, var11, var12);
            let var19 = self.block_texture(id, var2, var3, var4, 3);
            let t = self.block_texture(id, var2, var3, var4, 3);
            self.render_west_face(id, var2 as f64, var3 as f64, var4 as f64, t);
            if self.fancy_grass && var19 == 3 && self.override_block_texture < 0 {
                self.mul_ao_colors([var5, var6, var7]);
                self.render_west_face(
                    id,
                    var2 as f64,
                    var3 as f64,
                    var4 as f64,
                    GRASS_SIDE_OVERLAY,
                );
            }
            var8 = true;
        }

        // North (x - 1).
        if self.render_all_faces || self.should_side(id, var2 - 1, var3, var4, 4) {
            if self.field_22352_g <= 0 {
                var9 = self.ao_light_value_x_neg;
                var10 = var9;
                var11 = var9;
                var12 = var9;
            } else {
                var2 -= 1;
                self.field_22376_n = br(var2, var3 - 1, var4);
                self.field_22358_c = br(var2, var3, var4 - 1);
                self.field_22354_e = br(var2, var3, var4 + 1);
                self.field_22368_v = br(var2, var3 + 1, var4);
                self.field_22377_m = if !self.field_22335_x && !self.field_22357_ad {
                    self.field_22358_c
                } else {
                    br(var2, var3 - 1, var4 - 1)
                };
                self.field_22375_o = if !self.field_22333_z && !self.field_22357_ad {
                    self.field_22354_e
                } else {
                    br(var2, var3 - 1, var4 + 1)
                };
                self.field_22369_u = if !self.field_22335_x && !self.field_22337_v {
                    self.field_22358_c
                } else {
                    br(var2, var3 + 1, var4 - 1)
                };
                self.field_22367_w = if !self.field_22333_z && !self.field_22337_v {
                    self.field_22354_e
                } else {
                    br(var2, var3 + 1, var4 + 1)
                };
                var2 += 1;
                var12 = (self.field_22376_n
                    + self.field_22375_o
                    + self.ao_light_value_x_neg
                    + self.field_22354_e)
                    / 4.0;
                var9 = (self.ao_light_value_x_neg
                    + self.field_22354_e
                    + self.field_22368_v
                    + self.field_22367_w)
                    / 4.0;
                var10 = (self.field_22358_c
                    + self.ao_light_value_x_neg
                    + self.field_22369_u
                    + self.field_22368_v)
                    / 4.0;
                var11 = (self.field_22377_m
                    + self.field_22376_n
                    + self.field_22358_c
                    + self.ao_light_value_x_neg)
                    / 4.0;
            }
            let base = tint(var17).map(|c| c * 0.6);
            self.set_ao_colors(base, var9, var10, var11, var12);
            let var19 = self.block_texture(id, var2, var3, var4, 4);
            self.render_north_face(id, var2 as f64, var3 as f64, var4 as f64, var19);
            if self.fancy_grass && var19 == 3 && self.override_block_texture < 0 {
                self.mul_ao_colors([var5, var6, var7]);
                self.render_north_face(
                    id,
                    var2 as f64,
                    var3 as f64,
                    var4 as f64,
                    GRASS_SIDE_OVERLAY,
                );
            }
            var8 = true;
        }

        // South (x + 1).
        if self.render_all_faces || self.should_side(id, var2 + 1, var3, var4, 5) {
            if self.field_22352_g <= 0 {
                var9 = self.ao_light_value_x_pos;
                var10 = var9;
                var11 = var9;
                var12 = var9;
            } else {
                var2 += 1;
                self.field_22371_s = br(var2, var3 - 1, var4);
                self.field_22356_d = br(var2, var3, var4 - 1);
                self.field_22353_f = br(var2, var3, var4 + 1);
                self.field_22364_z = br(var2, var3 + 1, var4);
                self.field_22372_r = if !self.field_22359_ac && !self.field_22363_aa {
                    self.field_22356_d
                } else {
                    br(var2, var3 - 1, var4 - 1)
                };
                self.field_22370_t = if !self.field_22359_ac && !self.field_22334_y {
                    self.field_22353_f
                } else {
                    br(var2, var3 - 1, var4 + 1)
                };
                self.field_22365_y = if !self.field_22338_u && !self.field_22363_aa {
                    self.field_22356_d
                } else {
                    br(var2, var3 + 1, var4 - 1)
                };
                self.field_22360_b = if !self.field_22338_u && !self.field_22334_y {
                    self.field_22353_f
                } else {
                    br(var2, var3 + 1, var4 + 1)
                };
                var2 -= 1;
                var9 = (self.field_22371_s
                    + self.field_22370_t
                    + self.ao_light_value_x_pos
                    + self.field_22353_f)
                    / 4.0;
                var12 = (self.ao_light_value_x_pos
                    + self.field_22353_f
                    + self.field_22364_z
                    + self.field_22360_b)
                    / 4.0;
                var11 = (self.field_22356_d
                    + self.ao_light_value_x_pos
                    + self.field_22365_y
                    + self.field_22364_z)
                    / 4.0;
                var10 = (self.field_22372_r
                    + self.field_22371_s
                    + self.field_22356_d
                    + self.ao_light_value_x_pos)
                    / 4.0;
            }
            let base = tint(var18).map(|c| c * 0.6);
            self.set_ao_colors(base, var9, var10, var11, var12);
            let var19 = self.block_texture(id, var2, var3, var4, 5);
            self.render_south_face(id, var2 as f64, var3 as f64, var4 as f64, var19);
            if self.fancy_grass && var19 == 3 && self.override_block_texture < 0 {
                self.mul_ao_colors([var5, var6, var7]);
                self.render_south_face(
                    id,
                    var2 as f64,
                    var3 as f64,
                    var4 as f64,
                    GRASS_SIDE_OVERLAY,
                );
            }
            var8 = true;
        }

        self.enable_ao = false;
        var8
    }

    /// `renderStandardBlockWithColorMultiplier(block, x, y, z, r, g, b)`:
    /// one colour per face: the face shade (0.5, 1.0, 0.8, 0.6) times the
    /// colour multiplier (grass: only the top; its sides get the overlay 38
    /// tinted) times the brightness of the neighbour on that side (the
    /// block's own for a top below 1 that is not a liquid, and for sides
    /// set in from the cell's faces).
    // src: RenderBlocks.java:2210-2332
    #[allow(clippy::too_many_arguments)]
    pub fn render_standard_block_with_color_multiplier(
        &mut self,
        id: i32,
        x: i32,
        y: i32,
        z: i32,
        var5: f32,
        var6: f32,
        var7: f32,
    ) -> bool {
        let (var2, var3, var4) = (x, y, z);
        self.enable_ao = false;
        let mut var9 = false;
        let var10 = 0.5f32;
        let var11 = 1.0f32;
        let var12 = 0.8f32;
        let var13 = 0.6f32;
        let var14 = var11 * var5;
        let var15 = var11 * var6;
        let var16 = var11 * var7;
        let mut var17 = var10;
        let mut var18 = var12;
        let mut var19 = var13;
        let mut var20 = var10;
        let mut var21 = var12;
        let mut var22 = var13;
        let mut var23 = var10;
        let mut var24 = var12;
        let mut var25 = var13;
        if id != GRASS {
            var17 = var10 * var5;
            var18 = var12 * var5;
            var19 = var13 * var5;
            var20 = var10 * var6;
            var21 = var12 * var6;
            var22 = var13 * var6;
            var23 = var10 * var7;
            var24 = var12 * var7;
            var25 = var13 * var7;
        }
        let bb = bounds(&self.statics, id);
        let is_liquid = blocks::def(id).is_some_and(|b| b.block_material.get_is_liquid());
        let var26 = self.brightness(id, var2, var3, var4);
        if self.render_all_faces || self.should_side(id, var2, var3 - 1, var4, 0) {
            let var27 = self.brightness(id, var2, var3 - 1, var4);
            self.tess
                .set_color_opaque_f(var17 * var27, var20 * var27, var23 * var27);
            let t = self.block_texture(id, var2, var3, var4, 0);
            self.render_bottom_face(id, var2 as f64, var3 as f64, var4 as f64, t);
            var9 = true;
        }
        if self.render_all_faces || self.should_side(id, var2, var3 + 1, var4, 1) {
            let mut var27 = self.brightness(id, var2, var3 + 1, var4);
            if bb.max_y != 1.0 && !is_liquid {
                var27 = var26;
            }
            self.tess
                .set_color_opaque_f(var14 * var27, var15 * var27, var16 * var27);
            let t = self.block_texture(id, var2, var3, var4, 1);
            self.render_top_face(id, var2 as f64, var3 as f64, var4 as f64, t);
            var9 = true;
        }
        if self.render_all_faces || self.should_side(id, var2, var3, var4 - 1, 2) {
            let mut var27 = self.brightness(id, var2, var3, var4 - 1);
            if bb.min_z > 0.0 {
                var27 = var26;
            }
            self.tess
                .set_color_opaque_f(var18 * var27, var21 * var27, var24 * var27);
            let var28 = self.block_texture(id, var2, var3, var4, 2);
            self.render_east_face(id, var2 as f64, var3 as f64, var4 as f64, var28);
            if self.fancy_grass && var28 == 3 && self.override_block_texture < 0 {
                self.tess.set_color_opaque_f(
                    var18 * var27 * var5,
                    var21 * var27 * var6,
                    var24 * var27 * var7,
                );
                self.render_east_face(
                    id,
                    var2 as f64,
                    var3 as f64,
                    var4 as f64,
                    GRASS_SIDE_OVERLAY,
                );
            }
            var9 = true;
        }
        if self.render_all_faces || self.should_side(id, var2, var3, var4 + 1, 3) {
            let mut var27 = self.brightness(id, var2, var3, var4 + 1);
            if bb.max_z < 1.0 {
                var27 = var26;
            }
            self.tess
                .set_color_opaque_f(var18 * var27, var21 * var27, var24 * var27);
            let var28 = self.block_texture(id, var2, var3, var4, 3);
            self.render_west_face(id, var2 as f64, var3 as f64, var4 as f64, var28);
            if self.fancy_grass && var28 == 3 && self.override_block_texture < 0 {
                self.tess.set_color_opaque_f(
                    var18 * var27 * var5,
                    var21 * var27 * var6,
                    var24 * var27 * var7,
                );
                self.render_west_face(
                    id,
                    var2 as f64,
                    var3 as f64,
                    var4 as f64,
                    GRASS_SIDE_OVERLAY,
                );
            }
            var9 = true;
        }
        if self.render_all_faces || self.should_side(id, var2 - 1, var3, var4, 4) {
            let mut var27 = self.brightness(id, var2 - 1, var3, var4);
            if bb.min_x > 0.0 {
                var27 = var26;
            }
            self.tess
                .set_color_opaque_f(var19 * var27, var22 * var27, var25 * var27);
            let var28 = self.block_texture(id, var2, var3, var4, 4);
            self.render_north_face(id, var2 as f64, var3 as f64, var4 as f64, var28);
            if self.fancy_grass && var28 == 3 && self.override_block_texture < 0 {
                self.tess.set_color_opaque_f(
                    var19 * var27 * var5,
                    var22 * var27 * var6,
                    var25 * var27 * var7,
                );
                self.render_north_face(
                    id,
                    var2 as f64,
                    var3 as f64,
                    var4 as f64,
                    GRASS_SIDE_OVERLAY,
                );
            }
            var9 = true;
        }
        if self.render_all_faces || self.should_side(id, var2 + 1, var3, var4, 5) {
            let mut var27 = self.brightness(id, var2 + 1, var3, var4);
            if bb.max_x < 1.0 {
                var27 = var26;
            }
            self.tess
                .set_color_opaque_f(var19 * var27, var22 * var27, var25 * var27);
            let var28 = self.block_texture(id, var2, var3, var4, 5);
            self.render_south_face(id, var2 as f64, var3 as f64, var4 as f64, var28);
            if self.fancy_grass && var28 == 3 && self.override_block_texture < 0 {
                self.tess.set_color_opaque_f(
                    var19 * var27 * var5,
                    var22 * var27 * var6,
                    var25 * var27 * var7,
                );
                self.render_south_face(
                    id,
                    var2 as f64,
                    var3 as f64,
                    var4 as f64,
                    GRASS_SIDE_OVERLAY,
                );
            }
            var9 = true;
        }
        var9
    }

    // ---- the faces -------------------------------------------------------------------

    /// One face vertex: with AO on, the corner's colour first.
    // src: RenderBlocks.java:2718-2731 (and the five others)
    #[allow(clippy::too_many_arguments)]
    fn face_vertex(&mut self, corner: Corner, x: f64, y: f64, z: f64, u: f64, v: f64) {
        if self.enable_ao {
            let c = match corner {
                Corner::TopLeft => self.color_top_left,
                Corner::BottomLeft => self.color_bottom_left,
                Corner::BottomRight => self.color_bottom_right,
                Corner::TopRight => self.color_top_right,
            };
            self.tess.set_color_opaque_f(c[0], c[1], c[2]);
        }
        self.tess.add_vertex_with_uv(x, y, z, u, v);
    }

    /// `renderBottomFace(block, x, y, z, texture)`: the face at `minY`,
    /// textured from the bounds' x and z, rotated by `uv_rotate_bottom`.
    // src: RenderBlocks.java:2654-2734
    pub fn render_bottom_face(&mut self, id: i32, x: f64, y: f64, z: f64, tex: i32) {
        let mut var8 = tex;
        if self.override_block_texture >= 0 {
            var8 = self.override_block_texture;
        }
        let bb = bounds(&self.statics, id);
        let var10 = (var8 & 15) << 4;
        let var11 = var8 & 240;
        let mut var12 = (var10 as f64 + bb.min_x * 16.0) / 256.0;
        let mut var14 = (var10 as f64 + bb.max_x * 16.0 - 0.01) / 256.0;
        let mut var16 = (var11 as f64 + bb.min_z * 16.0) / 256.0;
        let mut var18 = (var11 as f64 + bb.max_z * 16.0 - 0.01) / 256.0;
        if bb.min_x < 0.0 || bb.max_x > 1.0 {
            var12 = ((var10 as f32 + 0.0) / 256.0) as f64;
            var14 = ((var10 as f32 + 15.99) / 256.0) as f64;
        }
        if bb.min_z < 0.0 || bb.max_z > 1.0 {
            var16 = ((var11 as f32 + 0.0) / 256.0) as f64;
            var18 = ((var11 as f32 + 15.99) / 256.0) as f64;
        }
        let mut var20 = var14;
        let mut var22 = var12;
        let mut var24 = var16;
        let mut var26 = var18;
        if self.uv_rotate_bottom == 2 {
            var12 = (var10 as f64 + bb.min_z * 16.0) / 256.0;
            var16 = ((var11 + 16) as f64 - bb.max_x * 16.0) / 256.0;
            var14 = (var10 as f64 + bb.max_z * 16.0) / 256.0;
            var18 = ((var11 + 16) as f64 - bb.min_x * 16.0) / 256.0;
            var24 = var16;
            var26 = var18;
            var20 = var12;
            var22 = var14;
            var16 = var18;
            var18 = var24;
        } else if self.uv_rotate_bottom == 1 {
            var12 = ((var10 + 16) as f64 - bb.max_z * 16.0) / 256.0;
            var16 = (var11 as f64 + bb.min_x * 16.0) / 256.0;
            var14 = ((var10 + 16) as f64 - bb.min_z * 16.0) / 256.0;
            var18 = (var11 as f64 + bb.max_x * 16.0) / 256.0;
            var20 = var14;
            var22 = var12;
            var12 = var14;
            var14 = var22;
            var24 = var18;
            var26 = var16;
        } else if self.uv_rotate_bottom == 3 {
            var12 = ((var10 + 16) as f64 - bb.min_x * 16.0) / 256.0;
            var14 = ((var10 + 16) as f64 - bb.max_x * 16.0 - 0.01) / 256.0;
            var16 = ((var11 + 16) as f64 - bb.min_z * 16.0) / 256.0;
            var18 = ((var11 + 16) as f64 - bb.max_z * 16.0 - 0.01) / 256.0;
            var20 = var14;
            var22 = var12;
            var24 = var16;
            var26 = var18;
        }
        let var28 = x + bb.min_x;
        let var30 = x + bb.max_x;
        let var32 = y + bb.min_y;
        let var34 = z + bb.min_z;
        let var36 = z + bb.max_z;
        self.face_vertex(Corner::TopLeft, var28, var32, var36, var22, var26);
        self.face_vertex(Corner::BottomLeft, var28, var32, var34, var12, var16);
        self.face_vertex(Corner::BottomRight, var30, var32, var34, var20, var24);
        self.face_vertex(Corner::TopRight, var30, var32, var36, var14, var18);
    }

    /// `renderTopFace(block, x, y, z, texture)`: the face at `maxY`,
    /// rotated by `uv_rotate_top`.
    // src: RenderBlocks.java:2736-2816
    pub fn render_top_face(&mut self, id: i32, x: f64, y: f64, z: f64, tex: i32) {
        let mut var8 = tex;
        if self.override_block_texture >= 0 {
            var8 = self.override_block_texture;
        }
        let bb = bounds(&self.statics, id);
        let var10 = (var8 & 15) << 4;
        let var11 = var8 & 240;
        let mut var12 = (var10 as f64 + bb.min_x * 16.0) / 256.0;
        let mut var14 = (var10 as f64 + bb.max_x * 16.0 - 0.01) / 256.0;
        let mut var16 = (var11 as f64 + bb.min_z * 16.0) / 256.0;
        let mut var18 = (var11 as f64 + bb.max_z * 16.0 - 0.01) / 256.0;
        if bb.min_x < 0.0 || bb.max_x > 1.0 {
            var12 = ((var10 as f32 + 0.0) / 256.0) as f64;
            var14 = ((var10 as f32 + 15.99) / 256.0) as f64;
        }
        if bb.min_z < 0.0 || bb.max_z > 1.0 {
            var16 = ((var11 as f32 + 0.0) / 256.0) as f64;
            var18 = ((var11 as f32 + 15.99) / 256.0) as f64;
        }
        let mut var20 = var14;
        let mut var22 = var12;
        let mut var24 = var16;
        let mut var26 = var18;
        if self.uv_rotate_top == 1 {
            var12 = (var10 as f64 + bb.min_z * 16.0) / 256.0;
            var16 = ((var11 + 16) as f64 - bb.max_x * 16.0) / 256.0;
            var14 = (var10 as f64 + bb.max_z * 16.0) / 256.0;
            var18 = ((var11 + 16) as f64 - bb.min_x * 16.0) / 256.0;
            var24 = var16;
            var26 = var18;
            var20 = var12;
            var22 = var14;
            var16 = var18;
            var18 = var24;
        } else if self.uv_rotate_top == 2 {
            var12 = ((var10 + 16) as f64 - bb.max_z * 16.0) / 256.0;
            var16 = (var11 as f64 + bb.min_x * 16.0) / 256.0;
            var14 = ((var10 + 16) as f64 - bb.min_z * 16.0) / 256.0;
            var18 = (var11 as f64 + bb.max_x * 16.0) / 256.0;
            var20 = var14;
            var22 = var12;
            var12 = var14;
            var14 = var22;
            var24 = var18;
            var26 = var16;
        } else if self.uv_rotate_top == 3 {
            var12 = ((var10 + 16) as f64 - bb.min_x * 16.0) / 256.0;
            var14 = ((var10 + 16) as f64 - bb.max_x * 16.0 - 0.01) / 256.0;
            var16 = ((var11 + 16) as f64 - bb.min_z * 16.0) / 256.0;
            var18 = ((var11 + 16) as f64 - bb.max_z * 16.0 - 0.01) / 256.0;
            var20 = var14;
            var22 = var12;
            var24 = var16;
            var26 = var18;
        }
        let var28 = x + bb.min_x;
        let var30 = x + bb.max_x;
        let var32 = y + bb.max_y;
        let var34 = z + bb.min_z;
        let var36 = z + bb.max_z;
        self.face_vertex(Corner::TopLeft, var30, var32, var36, var14, var18);
        self.face_vertex(Corner::BottomLeft, var30, var32, var34, var20, var24);
        self.face_vertex(Corner::BottomRight, var28, var32, var34, var12, var16);
        self.face_vertex(Corner::TopRight, var28, var32, var36, var22, var26);
    }

    /// `renderEastFace(block, x, y, z, texture)`: the face at `minZ` (vanilla
    /// "east" is −z), textured from x and y, flipped by `flipTexture`,
    /// rotated by `uv_rotate_east`.
    // src: RenderBlocks.java:2818-2905
    pub fn render_east_face(&mut self, id: i32, x: f64, y: f64, z: f64, tex: i32) {
        let mut var8 = tex;
        if self.override_block_texture >= 0 {
            var8 = self.override_block_texture;
        }
        let bb = bounds(&self.statics, id);
        let var10 = (var8 & 15) << 4;
        let var11 = var8 & 240;
        let mut var12 = (var10 as f64 + bb.min_x * 16.0) / 256.0;
        let mut var14 = (var10 as f64 + bb.max_x * 16.0 - 0.01) / 256.0;
        let mut var16 = ((var11 + 16) as f64 - bb.max_y * 16.0) / 256.0;
        let mut var18 = ((var11 + 16) as f64 - bb.min_y * 16.0 - 0.01) / 256.0;
        if self.flip_texture {
            std::mem::swap(&mut var12, &mut var14);
        }
        if bb.min_x < 0.0 || bb.max_x > 1.0 {
            var12 = ((var10 as f32 + 0.0) / 256.0) as f64;
            var14 = ((var10 as f32 + 15.99) / 256.0) as f64;
        }
        if bb.min_y < 0.0 || bb.max_y > 1.0 {
            var16 = ((var11 as f32 + 0.0) / 256.0) as f64;
            var18 = ((var11 as f32 + 15.99) / 256.0) as f64;
        }
        let mut var20 = var14;
        let mut var22 = var12;
        let mut var24 = var16;
        let mut var26 = var18;
        if self.uv_rotate_east == 2 {
            var12 = (var10 as f64 + bb.min_y * 16.0) / 256.0;
            var16 = ((var11 + 16) as f64 - bb.min_x * 16.0) / 256.0;
            var14 = (var10 as f64 + bb.max_y * 16.0) / 256.0;
            var18 = ((var11 + 16) as f64 - bb.max_x * 16.0) / 256.0;
            var24 = var16;
            var26 = var18;
            var20 = var12;
            var22 = var14;
            var16 = var18;
            var18 = var24;
        } else if self.uv_rotate_east == 1 {
            var12 = ((var10 + 16) as f64 - bb.max_y * 16.0) / 256.0;
            var16 = (var11 as f64 + bb.max_x * 16.0) / 256.0;
            var14 = ((var10 + 16) as f64 - bb.min_y * 16.0) / 256.0;
            var18 = (var11 as f64 + bb.min_x * 16.0) / 256.0;
            var20 = var14;
            var22 = var12;
            var12 = var14;
            var14 = var22;
            var24 = var18;
            var26 = var16;
        } else if self.uv_rotate_east == 3 {
            var12 = ((var10 + 16) as f64 - bb.min_x * 16.0) / 256.0;
            var14 = ((var10 + 16) as f64 - bb.max_x * 16.0 - 0.01) / 256.0;
            var16 = (var11 as f64 + bb.max_y * 16.0) / 256.0;
            var18 = (var11 as f64 + bb.min_y * 16.0 - 0.01) / 256.0;
            var20 = var14;
            var22 = var12;
            var24 = var16;
            var26 = var18;
        }
        let var28 = x + bb.min_x;
        let var30 = x + bb.max_x;
        let var32 = y + bb.min_y;
        let var34 = y + bb.max_y;
        let var36 = z + bb.min_z;
        self.face_vertex(Corner::TopLeft, var28, var34, var36, var20, var24);
        self.face_vertex(Corner::BottomLeft, var30, var34, var36, var12, var16);
        self.face_vertex(Corner::BottomRight, var30, var32, var36, var22, var26);
        self.face_vertex(Corner::TopRight, var28, var32, var36, var14, var18);
    }

    /// `renderWestFace(block, x, y, z, texture)`: the face at `maxZ`,
    /// rotated by `uv_rotate_west`.
    // src: RenderBlocks.java:2907-2994
    pub fn render_west_face(&mut self, id: i32, x: f64, y: f64, z: f64, tex: i32) {
        let mut var8 = tex;
        if self.override_block_texture >= 0 {
            var8 = self.override_block_texture;
        }
        let bb = bounds(&self.statics, id);
        let var10 = (var8 & 15) << 4;
        let var11 = var8 & 240;
        let mut var12 = (var10 as f64 + bb.min_x * 16.0) / 256.0;
        let mut var14 = (var10 as f64 + bb.max_x * 16.0 - 0.01) / 256.0;
        let mut var16 = ((var11 + 16) as f64 - bb.max_y * 16.0) / 256.0;
        let mut var18 = ((var11 + 16) as f64 - bb.min_y * 16.0 - 0.01) / 256.0;
        if self.flip_texture {
            std::mem::swap(&mut var12, &mut var14);
        }
        if bb.min_x < 0.0 || bb.max_x > 1.0 {
            var12 = ((var10 as f32 + 0.0) / 256.0) as f64;
            var14 = ((var10 as f32 + 15.99) / 256.0) as f64;
        }
        if bb.min_y < 0.0 || bb.max_y > 1.0 {
            var16 = ((var11 as f32 + 0.0) / 256.0) as f64;
            var18 = ((var11 as f32 + 15.99) / 256.0) as f64;
        }
        let mut var20 = var14;
        let mut var22 = var12;
        let mut var24 = var16;
        let mut var26 = var18;
        if self.uv_rotate_west == 1 {
            var12 = (var10 as f64 + bb.min_y * 16.0) / 256.0;
            var18 = ((var11 + 16) as f64 - bb.min_x * 16.0) / 256.0;
            var14 = (var10 as f64 + bb.max_y * 16.0) / 256.0;
            var16 = ((var11 + 16) as f64 - bb.max_x * 16.0) / 256.0;
            var24 = var16;
            var26 = var18;
            var20 = var12;
            var22 = var14;
            var16 = var18;
            var18 = var24;
        } else if self.uv_rotate_west == 2 {
            var12 = ((var10 + 16) as f64 - bb.max_y * 16.0) / 256.0;
            var16 = (var11 as f64 + bb.min_x * 16.0) / 256.0;
            var14 = ((var10 + 16) as f64 - bb.min_y * 16.0) / 256.0;
            var18 = (var11 as f64 + bb.max_x * 16.0) / 256.0;
            var20 = var14;
            var22 = var12;
            var12 = var14;
            var14 = var22;
            var24 = var18;
            var26 = var16;
        } else if self.uv_rotate_west == 3 {
            var12 = ((var10 + 16) as f64 - bb.min_x * 16.0) / 256.0;
            var14 = ((var10 + 16) as f64 - bb.max_x * 16.0 - 0.01) / 256.0;
            var16 = (var11 as f64 + bb.max_y * 16.0) / 256.0;
            var18 = (var11 as f64 + bb.min_y * 16.0 - 0.01) / 256.0;
            var20 = var14;
            var22 = var12;
            var24 = var16;
            var26 = var18;
        }
        let var28 = x + bb.min_x;
        let var30 = x + bb.max_x;
        let var32 = y + bb.min_y;
        let var34 = y + bb.max_y;
        let var36 = z + bb.max_z;
        self.face_vertex(Corner::TopLeft, var28, var34, var36, var12, var16);
        self.face_vertex(Corner::BottomLeft, var28, var32, var36, var22, var26);
        self.face_vertex(Corner::BottomRight, var30, var32, var36, var14, var18);
        self.face_vertex(Corner::TopRight, var30, var34, var36, var20, var24);
    }

    /// `renderNorthFace(block, x, y, z, texture)`: the face at `minX`,
    /// textured from z and y, rotated by `uv_rotate_north`.
    // src: RenderBlocks.java:2996-3083
    pub fn render_north_face(&mut self, id: i32, x: f64, y: f64, z: f64, tex: i32) {
        let mut var8 = tex;
        if self.override_block_texture >= 0 {
            var8 = self.override_block_texture;
        }
        let bb = bounds(&self.statics, id);
        let var10 = (var8 & 15) << 4;
        let var11 = var8 & 240;
        let mut var12 = (var10 as f64 + bb.min_z * 16.0) / 256.0;
        let mut var14 = (var10 as f64 + bb.max_z * 16.0 - 0.01) / 256.0;
        let mut var16 = ((var11 + 16) as f64 - bb.max_y * 16.0) / 256.0;
        let mut var18 = ((var11 + 16) as f64 - bb.min_y * 16.0 - 0.01) / 256.0;
        if self.flip_texture {
            std::mem::swap(&mut var12, &mut var14);
        }
        if bb.min_z < 0.0 || bb.max_z > 1.0 {
            var12 = ((var10 as f32 + 0.0) / 256.0) as f64;
            var14 = ((var10 as f32 + 15.99) / 256.0) as f64;
        }
        if bb.min_y < 0.0 || bb.max_y > 1.0 {
            var16 = ((var11 as f32 + 0.0) / 256.0) as f64;
            var18 = ((var11 as f32 + 15.99) / 256.0) as f64;
        }
        let mut var20 = var14;
        let mut var22 = var12;
        let mut var24 = var16;
        let mut var26 = var18;
        if self.uv_rotate_north == 1 {
            var12 = (var10 as f64 + bb.min_y * 16.0) / 256.0;
            var16 = ((var11 + 16) as f64 - bb.max_z * 16.0) / 256.0;
            var14 = (var10 as f64 + bb.max_y * 16.0) / 256.0;
            var18 = ((var11 + 16) as f64 - bb.min_z * 16.0) / 256.0;
            var24 = var16;
            var26 = var18;
            var20 = var12;
            var22 = var14;
            var16 = var18;
            var18 = var24;
        } else if self.uv_rotate_north == 2 {
            var12 = ((var10 + 16) as f64 - bb.max_y * 16.0) / 256.0;
            var16 = (var11 as f64 + bb.min_z * 16.0) / 256.0;
            var14 = ((var10 + 16) as f64 - bb.min_y * 16.0) / 256.0;
            var18 = (var11 as f64 + bb.max_z * 16.0) / 256.0;
            var20 = var14;
            var22 = var12;
            var12 = var14;
            var14 = var22;
            var24 = var18;
            var26 = var16;
        } else if self.uv_rotate_north == 3 {
            var12 = ((var10 + 16) as f64 - bb.min_z * 16.0) / 256.0;
            var14 = ((var10 + 16) as f64 - bb.max_z * 16.0 - 0.01) / 256.0;
            var16 = (var11 as f64 + bb.max_y * 16.0) / 256.0;
            var18 = (var11 as f64 + bb.min_y * 16.0 - 0.01) / 256.0;
            var20 = var14;
            var22 = var12;
            var24 = var16;
            var26 = var18;
        }
        let var28 = x + bb.min_x;
        let var30 = y + bb.min_y;
        let var32 = y + bb.max_y;
        let var34 = z + bb.min_z;
        let var36 = z + bb.max_z;
        self.face_vertex(Corner::TopLeft, var28, var32, var36, var20, var24);
        self.face_vertex(Corner::BottomLeft, var28, var32, var34, var12, var16);
        self.face_vertex(Corner::BottomRight, var28, var30, var34, var22, var26);
        self.face_vertex(Corner::TopRight, var28, var30, var36, var14, var18);
    }

    /// `renderSouthFace(block, x, y, z, texture)`: the face at `maxX`,
    /// rotated by `uv_rotate_south`.
    // src: RenderBlocks.java:3085-3172
    pub fn render_south_face(&mut self, id: i32, x: f64, y: f64, z: f64, tex: i32) {
        let mut var8 = tex;
        if self.override_block_texture >= 0 {
            var8 = self.override_block_texture;
        }
        let bb = bounds(&self.statics, id);
        let var10 = (var8 & 15) << 4;
        let var11 = var8 & 240;
        let mut var12 = (var10 as f64 + bb.min_z * 16.0) / 256.0;
        let mut var14 = (var10 as f64 + bb.max_z * 16.0 - 0.01) / 256.0;
        let mut var16 = ((var11 + 16) as f64 - bb.max_y * 16.0) / 256.0;
        let mut var18 = ((var11 + 16) as f64 - bb.min_y * 16.0 - 0.01) / 256.0;
        if self.flip_texture {
            std::mem::swap(&mut var12, &mut var14);
        }
        if bb.min_z < 0.0 || bb.max_z > 1.0 {
            var12 = ((var10 as f32 + 0.0) / 256.0) as f64;
            var14 = ((var10 as f32 + 15.99) / 256.0) as f64;
        }
        if bb.min_y < 0.0 || bb.max_y > 1.0 {
            var16 = ((var11 as f32 + 0.0) / 256.0) as f64;
            var18 = ((var11 as f32 + 15.99) / 256.0) as f64;
        }
        let mut var20 = var14;
        let mut var22 = var12;
        let mut var24 = var16;
        let mut var26 = var18;
        if self.uv_rotate_south == 2 {
            var12 = (var10 as f64 + bb.min_y * 16.0) / 256.0;
            var16 = ((var11 + 16) as f64 - bb.min_z * 16.0) / 256.0;
            var14 = (var10 as f64 + bb.max_y * 16.0) / 256.0;
            var18 = ((var11 + 16) as f64 - bb.max_z * 16.0) / 256.0;
            var24 = var16;
            var26 = var18;
            var20 = var12;
            var22 = var14;
            var16 = var18;
            var18 = var24;
        } else if self.uv_rotate_south == 1 {
            var12 = ((var10 + 16) as f64 - bb.max_y * 16.0) / 256.0;
            var16 = (var11 as f64 + bb.max_z * 16.0) / 256.0;
            var14 = ((var10 + 16) as f64 - bb.min_y * 16.0) / 256.0;
            var18 = (var11 as f64 + bb.min_z * 16.0) / 256.0;
            var20 = var14;
            var22 = var12;
            var12 = var14;
            var14 = var22;
            var24 = var18;
            var26 = var16;
        } else if self.uv_rotate_south == 3 {
            var12 = ((var10 + 16) as f64 - bb.min_z * 16.0) / 256.0;
            var14 = ((var10 + 16) as f64 - bb.max_z * 16.0 - 0.01) / 256.0;
            var16 = (var11 as f64 + bb.max_y * 16.0) / 256.0;
            var18 = (var11 as f64 + bb.min_y * 16.0 - 0.01) / 256.0;
            var20 = var14;
            var22 = var12;
            var24 = var16;
            var26 = var18;
        }
        let var28 = x + bb.max_x;
        let var30 = y + bb.min_y;
        let var32 = y + bb.max_y;
        let var34 = z + bb.min_z;
        let var36 = z + bb.max_z;
        self.face_vertex(Corner::TopLeft, var28, var30, var36, var22, var26);
        self.face_vertex(Corner::BottomLeft, var28, var30, var34, var14, var18);
        self.face_vertex(Corner::BottomRight, var28, var32, var34, var20, var24);
        self.face_vertex(Corner::TopRight, var28, var32, var36, var12, var16);
    }

    // ---- the block as an item ------------------------------------------------------

    /// One `renderBlockOnInventory` draw: `startDrawingQuads`, `setNormal`,
    /// the face, `draw`. The draws are concatenated in [`RenderBlocks::tess`].
    fn inventory_face(&mut self, normal: [f32; 3], face: impl FnOnce(&mut Self)) {
        self.tess.set_normal(normal[0], normal[1], normal[2]);
        face(self);
    }

    /// `renderBlockOnInventory(block, meta, brightness)`: the block as an
    /// item, in the block's unit cube moved by (−0.5, −0.5, −0.5) (the
    /// `glTranslatef(-0.5F, …)`, baked into the vertices), one draw per face
    /// with its normal. Returns `glColor4f` when [`Self::use_inventory_tint`]
    /// (`getRenderColor(meta)` × `brightness`, alpha 1). Types 0 and 16 (the
    /// piston, with metadata 1) and the face-built 10, 11 and 13 are drawn
    /// here; 1, 2 and 6 call the WP-3.3c helpers.
    // src: RenderBlocks.java:3174-3365
    pub fn render_block_on_inventory(
        &mut self,
        id: i32,
        meta: i32,
        brightness: f32,
    ) -> Option<[f32; 4]> {
        let mut var2 = meta;
        let var3 = brightness;
        self.tess.start_drawing_quads();
        let mut gl_color = None;
        if self.use_inventory_tint {
            let var5 = blocks::get_render_color(id, var2);
            let var6 = (var5 >> 16 & 255) as f32 / 255.0;
            let var7 = (var5 >> 8 & 255) as f32 / 255.0;
            let var8 = (var5 & 255) as f32 / 255.0;
            gl_color = Some([var6 * var3, var7 * var3, var8 * var3, 1.0]);
        }
        let var5 = blocks::get_render_type(id);
        let tex_side = |s: &Self, side| blocks::get_block_texture_from_side(&s.statics, id, side);
        if var5 != 0 && var5 != 16 {
            if var5 == 1 {
                self.tess.set_translation_d(0.0, 0.0, 0.0);
                self.tess.set_normal(0.0, -1.0, 0.0);
                self.render_crossed_squares(id, var2, -0.5, -0.5, -0.5);
            } else if var5 == 13 {
                blocks::set_block_bounds_for_item_render(&mut self.statics, id);
                self.tess.set_translation_d(-0.5, -0.5, -0.5);
                let var6 = 0.0625f32;
                self.inventory_face([0.0, -1.0, 0.0], |s| {
                    let t = tex_side(s, 0);
                    s.render_bottom_face(id, 0.0, 0.0, 0.0, t)
                });
                self.inventory_face([0.0, 1.0, 0.0], |s| {
                    let t = tex_side(s, 1);
                    s.render_top_face(id, 0.0, 0.0, 0.0, t)
                });
                self.inventory_face([0.0, 0.0, -1.0], |s| {
                    s.tess.set_translation_f(0.0, 0.0, var6);
                    let t = tex_side(s, 2);
                    s.render_east_face(id, 0.0, 0.0, 0.0, t);
                    s.tess.set_translation_f(0.0, 0.0, -var6);
                });
                self.inventory_face([0.0, 0.0, 1.0], |s| {
                    s.tess.set_translation_f(0.0, 0.0, -var6);
                    let t = tex_side(s, 3);
                    s.render_west_face(id, 0.0, 0.0, 0.0, t);
                    s.tess.set_translation_f(0.0, 0.0, var6);
                });
                self.inventory_face([-1.0, 0.0, 0.0], |s| {
                    s.tess.set_translation_f(var6, 0.0, 0.0);
                    let t = tex_side(s, 4);
                    s.render_north_face(id, 0.0, 0.0, 0.0, t);
                    s.tess.set_translation_f(-var6, 0.0, 0.0);
                });
                self.inventory_face([1.0, 0.0, 0.0], |s| {
                    s.tess.set_translation_f(-var6, 0.0, 0.0);
                    let t = tex_side(s, 5);
                    s.render_south_face(id, 0.0, 0.0, 0.0, t);
                    s.tess.set_translation_f(var6, 0.0, 0.0);
                });
            } else if var5 == 6 {
                self.tess.set_translation_d(0.0, 0.0, 0.0);
                self.tess.set_normal(0.0, -1.0, 0.0);
                self.func_1245_b(id, var2, -0.5, -0.5, -0.5);
            } else if var5 == 2 {
                self.tess.set_translation_d(0.0, 0.0, 0.0);
                self.tess.set_normal(0.0, -1.0, 0.0);
                self.render_torch_at_angle(id, -0.5, -0.5, -0.5, 0.0, 0.0);
            } else if var5 == 10 {
                for var9 in 0..2 {
                    if var9 == 0 {
                        self.set_block_bounds(id, 0.0, 0.0, 0.0, 1.0, 1.0, 0.5);
                    }
                    if var9 == 1 {
                        self.set_block_bounds(id, 0.0, 0.0, 0.5, 1.0, 0.5, 1.0);
                    }
                    self.inventory_cube(id);
                }
            } else if var5 == 11 {
                for var9 in 0..4 {
                    let mut var7 = 0.125f32;
                    if var9 == 0 {
                        self.set_block_bounds(
                            id,
                            0.5 - var7,
                            0.0,
                            0.0,
                            0.5 + var7,
                            1.0,
                            var7 * 2.0,
                        );
                    }
                    if var9 == 1 {
                        self.set_block_bounds(
                            id,
                            0.5 - var7,
                            0.0,
                            1.0 - var7 * 2.0,
                            0.5 + var7,
                            1.0,
                            1.0,
                        );
                    }
                    var7 = 0.0625;
                    if var9 == 2 {
                        self.set_block_bounds(
                            id,
                            0.5 - var7,
                            1.0 - var7 * 3.0,
                            -var7 * 2.0,
                            0.5 + var7,
                            1.0 - var7,
                            1.0 + var7 * 2.0,
                        );
                    }
                    if var9 == 3 {
                        self.set_block_bounds(
                            id,
                            0.5 - var7,
                            0.5 - var7 * 3.0,
                            -var7 * 2.0,
                            0.5 + var7,
                            0.5 - var7,
                            1.0 + var7 * 2.0,
                        );
                    }
                    self.inventory_cube(id);
                }
                self.set_block_bounds(id, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
            }
        } else {
            if var5 == 16 {
                var2 = 1;
            }
            blocks::set_block_bounds_for_item_render(&mut self.statics, id);
            self.tess.set_translation_d(-0.5, -0.5, -0.5);
            let tex = |s: &Self, side| {
                blocks::get_block_texture_from_side_and_metadata(&s.statics, id, side, var2)
            };
            self.inventory_face([0.0, -1.0, 0.0], |s| {
                let t = tex(s, 0);
                s.render_bottom_face(id, 0.0, 0.0, 0.0, t)
            });
            self.inventory_face([0.0, 1.0, 0.0], |s| {
                let t = tex(s, 1);
                s.render_top_face(id, 0.0, 0.0, 0.0, t)
            });
            self.inventory_face([0.0, 0.0, -1.0], |s| {
                let t = tex(s, 2);
                s.render_east_face(id, 0.0, 0.0, 0.0, t)
            });
            self.inventory_face([0.0, 0.0, 1.0], |s| {
                let t = tex(s, 3);
                s.render_west_face(id, 0.0, 0.0, 0.0, t)
            });
            self.inventory_face([-1.0, 0.0, 0.0], |s| {
                let t = tex(s, 4);
                s.render_north_face(id, 0.0, 0.0, 0.0, t)
            });
            self.inventory_face([1.0, 0.0, 0.0], |s| {
                let t = tex(s, 5);
                s.render_south_face(id, 0.0, 0.0, 0.0, t)
            });
        }
        self.tess.set_translation_d(0.0, 0.0, 0.0);
        gl_color
    }

    /// The six inventory draws of the stairs' and fence's pieces, with
    /// `getBlockTextureFromSide`, around `glTranslatef(∓0.5)`.
    // src: RenderBlocks.java:3253-3278, :3300-3325
    fn inventory_cube(&mut self, id: i32) {
        self.tess.set_translation_d(-0.5, -0.5, -0.5);
        let tex = |s: &Self, side| blocks::get_block_texture_from_side(&s.statics, id, side);
        self.inventory_face([0.0, -1.0, 0.0], |s| {
            let t = tex(s, 0);
            s.render_bottom_face(id, 0.0, 0.0, 0.0, t)
        });
        self.inventory_face([0.0, 1.0, 0.0], |s| {
            let t = tex(s, 1);
            s.render_top_face(id, 0.0, 0.0, 0.0, t)
        });
        self.inventory_face([0.0, 0.0, -1.0], |s| {
            let t = tex(s, 2);
            s.render_east_face(id, 0.0, 0.0, 0.0, t)
        });
        self.inventory_face([0.0, 0.0, 1.0], |s| {
            let t = tex(s, 3);
            s.render_west_face(id, 0.0, 0.0, 0.0, t)
        });
        self.inventory_face([-1.0, 0.0, 0.0], |s| {
            let t = tex(s, 4);
            s.render_north_face(id, 0.0, 0.0, 0.0, t)
        });
        self.inventory_face([1.0, 0.0, 0.0], |s| {
            let t = tex(s, 5);
            s.render_south_face(id, 0.0, 0.0, 0.0, t)
        });
        self.tess.set_translation_d(0.0, 0.0, 0.0);
    }
}

/// `RenderBlocks.renderItemIn3d(renderType)`: whether an item of this block
/// is drawn as a 3D block (types 0, 13, 10, 11, 16).
// src: RenderBlocks.java:3367-3379
#[allow(clippy::if_same_then_else)] // the Java's branches, kept
pub fn render_item_in_3d(var0: i32) -> bool {
    if var0 == 0 {
        true
    } else if var0 == 13 {
        true
    } else if var0 == 10 {
        true
    } else if var0 == 11 {
        true
    } else {
        var0 == 16
    }
}

/// A block drawn as an item ([`render_block_as_item`]).
#[derive(Clone, Debug)]
pub struct ItemBlockDraw {
    /// The quads (positions around the origin, normals per face).
    pub tess: Tessellator,
    /// `glColor4f` for the draw, if set.
    pub gl_color: Option<[f32; 4]>,
}

/// `new RenderBlocks().renderBlockOnInventory(block, meta, brightness)` for
/// the item renderer (WP-3.7b): `statics` is the world's
/// `Block` singleton state (copied, so the item bounds do not leak into the
/// world). `None` when nothing was drawn (no such block, or a render type
/// not drawn as an item).
pub fn render_block_as_item(
    statics: &BlockStatics,
    colorizers: &crate::textures::Colorizers,
    id: i32,
    meta: i32,
    brightness: f32,
) -> Option<ItemBlockDraw> {
    blocks::def(id)?;
    let empty = ChunkSnapshot::empty(super::snapshot::render_statics(statics), colorizers.clone());
    let mut rb = RenderBlocks::new(&empty, empty.statics.clone(), false, false);
    let gl_color = rb.render_block_on_inventory(id, meta, brightness);
    (!rb.tess.is_empty()).then(|| ItemBlockDraw {
        tess: std::mem::take(&mut rb.tess),
        gl_color,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mc_sim::world::provider::{Dimension, WorldProvider};

    fn table() -> [f32; 16] {
        WorldProvider::new(Dimension::Surface).light_brightness_table
    }

    const STONE: i32 = 1;

    /// A renderer over `s` with full bounds and AO off.
    fn rb(s: &ChunkSnapshot) -> RenderBlocks<'_> {
        RenderBlocks::new(s, s.statics.clone(), false, false)
    }

    /// The Java expressions of a full-size face, written out once more.
    fn tile(i: i32) -> (f64, f64, f64, f64) {
        let x = (i & 15) << 4;
        let y = i & 240;
        (
            x as f64 / 256.0,
            (x as f64 + 16.0 - 0.01) / 256.0,
            y as f64 / 256.0,
            (y as f64 + 16.0 - 0.01) / 256.0,
        )
    }

    fn uv(t: &Tessellator) -> Vec<[f32; 2]> {
        t.uvs.clone()
    }

    fn f(v: f64) -> f32 {
        v as f32
    }

    #[test]
    fn faces_of_a_full_cube_without_rotation() {
        let s = ChunkSnapshot::synthetic([0; 3], [1; 3], table());
        let mut r = rb(&s);
        let (u0, u1, v0, v1) = tile(1);
        r.render_bottom_face(STONE, 0.0, 0.0, 0.0, 1);
        assert_eq!(
            r.tess.positions,
            vec![
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 1.0]
            ]
        );
        assert_eq!(
            uv(&r.tess),
            vec![
                [f(u0), f(v1)],
                [f(u0), f(v0)],
                [f(u1), f(v0)],
                [f(u1), f(v1)]
            ]
        );
        r.tess.start_drawing_quads();
        r.render_top_face(STONE, 0.0, 0.0, 0.0, 1);
        assert_eq!(
            r.tess.positions,
            vec![
                [1.0, 1.0, 1.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 1.0, 1.0]
            ]
        );
        assert_eq!(
            uv(&r.tess),
            vec![
                [f(u1), f(v1)],
                [f(u1), f(v0)],
                [f(u0), f(v0)],
                [f(u0), f(v1)]
            ]
        );
        // east (−z): v from the top (16 - maxY*16)
        r.tess.start_drawing_quads();
        r.render_east_face(STONE, 0.0, 0.0, 0.0, 1);
        assert_eq!(
            r.tess.positions,
            vec![
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 0.0, 0.0]
            ]
        );
        assert_eq!(
            uv(&r.tess),
            vec![
                [f(u1), f(v0)],
                [f(u0), f(v0)],
                [f(u0), f(v1)],
                [f(u1), f(v1)]
            ]
        );
        r.tess.start_drawing_quads();
        r.render_west_face(STONE, 0.0, 0.0, 0.0, 1);
        assert_eq!(
            uv(&r.tess),
            vec![
                [f(u0), f(v0)],
                [f(u0), f(v1)],
                [f(u1), f(v1)],
                [f(u1), f(v0)]
            ]
        );
        r.tess.start_drawing_quads();
        r.render_north_face(STONE, 0.0, 0.0, 0.0, 1);
        assert_eq!(
            r.tess.positions,
            vec![
                [0.0, 1.0, 1.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 1.0]
            ]
        );
        assert_eq!(
            uv(&r.tess),
            vec![
                [f(u1), f(v0)],
                [f(u0), f(v0)],
                [f(u0), f(v1)],
                [f(u1), f(v1)]
            ]
        );
        r.tess.start_drawing_quads();
        r.render_south_face(STONE, 0.0, 0.0, 0.0, 1);
        assert_eq!(
            r.tess.positions,
            vec![
                [1.0, 0.0, 1.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [1.0, 1.0, 1.0]
            ]
        );
        assert_eq!(
            uv(&r.tess),
            vec![
                [f(u0), f(v1)],
                [f(u1), f(v1)],
                [f(u1), f(v0)],
                [f(u0), f(v0)]
            ]
        );
    }

    /// Every `uvRotate` case of every face against the Java expressions,
    /// for a cuboid with distinct bounds (x 0.25..0.75, y 0..0.75,
    /// z 0.125..1), texture 77 (x 208, y 64).
    #[test]
    fn uv_rotations_of_every_face() {
        let s = ChunkSnapshot::synthetic([0; 3], [1; 3], table());
        let (x0, y0, z0, x1, y1, z1) = (0.25f64, 0.0f64, 0.125f64, 0.75f64, 0.75f64, 1.0f64);
        let (a, b) = (208.0f64, 64.0f64);
        let d = |v: f64| (v / 256.0) as f32;
        type Face = fn(&mut RenderBlocks<'_>, i32, f64, f64, f64, i32);
        type Setter = fn(&mut RenderBlocks<'_>, i32);
        type Case = (Face, Setter, Vec<(i32, [[f32; 2]; 4])>);
        // (face, setter, [rotation → expected uvs of the 4 vertices])
        let cases: Vec<Case> = vec![
            (
                |r, id, x, y, z, t| r.render_bottom_face(id, x, y, z, t),
                |r, v| r.uv_rotate_bottom = v,
                vec![
                    // 0: (var22,var26) (var12,var16) (var20,var24) (var14,var18)
                    (
                        0,
                        [
                            [d(a + x0 * 16.0), d(b + z1 * 16.0 - 0.01)],
                            [d(a + x0 * 16.0), d(b + z0 * 16.0)],
                            [d(a + x1 * 16.0 - 0.01), d(b + z0 * 16.0)],
                            [d(a + x1 * 16.0 - 0.01), d(b + z1 * 16.0 - 0.01)],
                        ],
                    ),
                    // 2: var12 = a+z0*16, var16' = b+16-x0*16 (var18 old), var14 = a+z1*16,
                    // var18' = var24 = b+16-x1*16; var20 = a+z0*16, var22 = a+z1*16,
                    // var24 = b+16-x1*16, var26 = b+16-x0*16
                    (
                        2,
                        [
                            [d(a + z1 * 16.0), d(b + 16.0 - x0 * 16.0)],
                            [d(a + z0 * 16.0), d(b + 16.0 - x0 * 16.0)],
                            [d(a + z0 * 16.0), d(b + 16.0 - x1 * 16.0)],
                            [d(a + z1 * 16.0), d(b + 16.0 - x1 * 16.0)],
                        ],
                    ),
                    // 1: var12 = a+16-z0*16 (old var14), var14 = a+16-z1*16 (old var12),
                    // var16 = b+x0*16, var18 = b+x1*16, var20 = a+16-z0*16,
                    // var22 = a+16-z1*16, var24 = b+x1*16, var26 = b+x0*16
                    (
                        1,
                        [
                            [d(a + 16.0 - z1 * 16.0), d(b + x0 * 16.0)],
                            [d(a + 16.0 - z0 * 16.0), d(b + x0 * 16.0)],
                            [d(a + 16.0 - z0 * 16.0), d(b + x1 * 16.0)],
                            [d(a + 16.0 - z1 * 16.0), d(b + x1 * 16.0)],
                        ],
                    ),
                    (
                        3,
                        [
                            [d(a + 16.0 - x0 * 16.0), d(b + 16.0 - z1 * 16.0 - 0.01)],
                            [d(a + 16.0 - x0 * 16.0), d(b + 16.0 - z0 * 16.0)],
                            [d(a + 16.0 - x1 * 16.0 - 0.01), d(b + 16.0 - z0 * 16.0)],
                            [
                                d(a + 16.0 - x1 * 16.0 - 0.01),
                                d(b + 16.0 - z1 * 16.0 - 0.01),
                            ],
                        ],
                    ),
                ],
            ),
            (
                |r, id, x, y, z, t| r.render_top_face(id, x, y, z, t),
                |r, v| r.uv_rotate_top = v,
                vec![
                    // top: (var14,var18) (var20,var24) (var12,var16) (var22,var26)
                    (
                        0,
                        [
                            [d(a + x1 * 16.0 - 0.01), d(b + z1 * 16.0 - 0.01)],
                            [d(a + x1 * 16.0 - 0.01), d(b + z0 * 16.0)],
                            [d(a + x0 * 16.0), d(b + z0 * 16.0)],
                            [d(a + x0 * 16.0), d(b + z1 * 16.0 - 0.01)],
                        ],
                    ),
                    (
                        1,
                        [
                            [d(a + z1 * 16.0), d(b + 16.0 - x1 * 16.0)],
                            [d(a + z0 * 16.0), d(b + 16.0 - x1 * 16.0)],
                            [d(a + z0 * 16.0), d(b + 16.0 - x0 * 16.0)],
                            [d(a + z1 * 16.0), d(b + 16.0 - x0 * 16.0)],
                        ],
                    ),
                    (
                        2,
                        [
                            [d(a + 16.0 - z1 * 16.0), d(b + x1 * 16.0)],
                            [d(a + 16.0 - z0 * 16.0), d(b + x1 * 16.0)],
                            [d(a + 16.0 - z0 * 16.0), d(b + x0 * 16.0)],
                            [d(a + 16.0 - z1 * 16.0), d(b + x0 * 16.0)],
                        ],
                    ),
                    (
                        3,
                        [
                            [
                                d(a + 16.0 - x1 * 16.0 - 0.01),
                                d(b + 16.0 - z1 * 16.0 - 0.01),
                            ],
                            [d(a + 16.0 - x1 * 16.0 - 0.01), d(b + 16.0 - z0 * 16.0)],
                            [d(a + 16.0 - x0 * 16.0), d(b + 16.0 - z0 * 16.0)],
                            [d(a + 16.0 - x0 * 16.0), d(b + 16.0 - z1 * 16.0 - 0.01)],
                        ],
                    ),
                ],
            ),
            (
                |r, id, x, y, z, t| r.render_east_face(id, x, y, z, t),
                |r, v| r.uv_rotate_east = v,
                vec![
                    // east: (var20,var24) (var12,var16) (var22,var26) (var14,var18)
                    (
                        0,
                        [
                            [d(a + x1 * 16.0 - 0.01), d(b + 16.0 - y1 * 16.0)],
                            [d(a + x0 * 16.0), d(b + 16.0 - y1 * 16.0)],
                            [d(a + x0 * 16.0), d(b + 16.0 - y0 * 16.0 - 0.01)],
                            [d(a + x1 * 16.0 - 0.01), d(b + 16.0 - y0 * 16.0 - 0.01)],
                        ],
                    ),
                    // 2: var12 = a+y0*16, var16 = b+16-x0*16, var14 = a+y1*16,
                    // var18 = b+16-x1*16; then var24 = var16, var26 = var18,
                    // var20 = var12, var22 = var14, var16 = var18, var18 = var24
                    (
                        2,
                        [
                            [d(a + y0 * 16.0), d(b + 16.0 - x0 * 16.0)],
                            [d(a + y0 * 16.0), d(b + 16.0 - x1 * 16.0)],
                            [d(a + y1 * 16.0), d(b + 16.0 - x1 * 16.0)],
                            [d(a + y1 * 16.0), d(b + 16.0 - x0 * 16.0)],
                        ],
                    ),
                    (
                        1,
                        [
                            [d(a + 16.0 - y0 * 16.0), d(b + x0 * 16.0)],
                            [d(a + 16.0 - y0 * 16.0), d(b + x1 * 16.0)],
                            [d(a + 16.0 - y1 * 16.0), d(b + x1 * 16.0)],
                            [d(a + 16.0 - y1 * 16.0), d(b + x0 * 16.0)],
                        ],
                    ),
                    (
                        3,
                        [
                            [d(a + 16.0 - x1 * 16.0 - 0.01), d(b + y1 * 16.0)],
                            [d(a + 16.0 - x0 * 16.0), d(b + y1 * 16.0)],
                            [d(a + 16.0 - x0 * 16.0), d(b + y0 * 16.0 - 0.01)],
                            [d(a + 16.0 - x1 * 16.0 - 0.01), d(b + y0 * 16.0 - 0.01)],
                        ],
                    ),
                ],
            ),
            (
                |r, id, x, y, z, t| r.render_west_face(id, x, y, z, t),
                |r, v| r.uv_rotate_west = v,
                vec![
                    // west: (var12,var16) (var22,var26) (var14,var18) (var20,var24)
                    (
                        0,
                        [
                            [d(a + x0 * 16.0), d(b + 16.0 - y1 * 16.0)],
                            [d(a + x0 * 16.0), d(b + 16.0 - y0 * 16.0 - 0.01)],
                            [d(a + x1 * 16.0 - 0.01), d(b + 16.0 - y0 * 16.0 - 0.01)],
                            [d(a + x1 * 16.0 - 0.01), d(b + 16.0 - y1 * 16.0)],
                        ],
                    ),
                    // 1: var12 = a+y0*16, var18 = b+16-x0*16, var14 = a+y1*16,
                    // var16 = b+16-x1*16; var24 = var16, var26 = var18,
                    // var20 = var12, var22 = var14, var16 = var18, var18 = var24
                    (
                        1,
                        [
                            [d(a + y0 * 16.0), d(b + 16.0 - x0 * 16.0)],
                            [d(a + y1 * 16.0), d(b + 16.0 - x0 * 16.0)],
                            [d(a + y1 * 16.0), d(b + 16.0 - x1 * 16.0)],
                            [d(a + y0 * 16.0), d(b + 16.0 - x1 * 16.0)],
                        ],
                    ),
                    (
                        2,
                        [
                            [d(a + 16.0 - y0 * 16.0), d(b + x0 * 16.0)],
                            [d(a + 16.0 - y1 * 16.0), d(b + x0 * 16.0)],
                            [d(a + 16.0 - y1 * 16.0), d(b + x1 * 16.0)],
                            [d(a + 16.0 - y0 * 16.0), d(b + x1 * 16.0)],
                        ],
                    ),
                    (
                        3,
                        [
                            [d(a + 16.0 - x0 * 16.0), d(b + y1 * 16.0)],
                            [d(a + 16.0 - x0 * 16.0), d(b + y0 * 16.0 - 0.01)],
                            [d(a + 16.0 - x1 * 16.0 - 0.01), d(b + y0 * 16.0 - 0.01)],
                            [d(a + 16.0 - x1 * 16.0 - 0.01), d(b + y1 * 16.0)],
                        ],
                    ),
                ],
            ),
            (
                |r, id, x, y, z, t| r.render_north_face(id, x, y, z, t),
                |r, v| r.uv_rotate_north = v,
                vec![
                    // north: (var20,var24) (var12,var16) (var22,var26) (var14,var18)
                    (
                        0,
                        [
                            [d(a + z1 * 16.0 - 0.01), d(b + 16.0 - y1 * 16.0)],
                            [d(a + z0 * 16.0), d(b + 16.0 - y1 * 16.0)],
                            [d(a + z0 * 16.0), d(b + 16.0 - y0 * 16.0 - 0.01)],
                            [d(a + z1 * 16.0 - 0.01), d(b + 16.0 - y0 * 16.0 - 0.01)],
                        ],
                    ),
                    (
                        1,
                        [
                            [d(a + y0 * 16.0), d(b + 16.0 - z1 * 16.0)],
                            [d(a + y0 * 16.0), d(b + 16.0 - z0 * 16.0)],
                            [d(a + y1 * 16.0), d(b + 16.0 - z0 * 16.0)],
                            [d(a + y1 * 16.0), d(b + 16.0 - z1 * 16.0)],
                        ],
                    ),
                    (
                        2,
                        [
                            [d(a + 16.0 - y0 * 16.0), d(b + z1 * 16.0)],
                            [d(a + 16.0 - y0 * 16.0), d(b + z0 * 16.0)],
                            [d(a + 16.0 - y1 * 16.0), d(b + z0 * 16.0)],
                            [d(a + 16.0 - y1 * 16.0), d(b + z1 * 16.0)],
                        ],
                    ),
                    (
                        3,
                        [
                            [d(a + 16.0 - z1 * 16.0 - 0.01), d(b + y1 * 16.0)],
                            [d(a + 16.0 - z0 * 16.0), d(b + y1 * 16.0)],
                            [d(a + 16.0 - z0 * 16.0), d(b + y0 * 16.0 - 0.01)],
                            [d(a + 16.0 - z1 * 16.0 - 0.01), d(b + y0 * 16.0 - 0.01)],
                        ],
                    ),
                ],
            ),
            (
                |r, id, x, y, z, t| r.render_south_face(id, x, y, z, t),
                |r, v| r.uv_rotate_south = v,
                vec![
                    // south: (var22,var26) (var14,var18) (var20,var24) (var12,var16)
                    (
                        0,
                        [
                            [d(a + z0 * 16.0), d(b + 16.0 - y0 * 16.0 - 0.01)],
                            [d(a + z1 * 16.0 - 0.01), d(b + 16.0 - y0 * 16.0 - 0.01)],
                            [d(a + z1 * 16.0 - 0.01), d(b + 16.0 - y1 * 16.0)],
                            [d(a + z0 * 16.0), d(b + 16.0 - y1 * 16.0)],
                        ],
                    ),
                    (
                        2,
                        [
                            [d(a + y1 * 16.0), d(b + 16.0 - z1 * 16.0)],
                            [d(a + y1 * 16.0), d(b + 16.0 - z0 * 16.0)],
                            [d(a + y0 * 16.0), d(b + 16.0 - z0 * 16.0)],
                            [d(a + y0 * 16.0), d(b + 16.0 - z1 * 16.0)],
                        ],
                    ),
                    (
                        1,
                        [
                            [d(a + 16.0 - y1 * 16.0), d(b + z1 * 16.0)],
                            [d(a + 16.0 - y1 * 16.0), d(b + z0 * 16.0)],
                            [d(a + 16.0 - y0 * 16.0), d(b + z0 * 16.0)],
                            [d(a + 16.0 - y0 * 16.0), d(b + z1 * 16.0)],
                        ],
                    ),
                    (
                        3,
                        [
                            [d(a + 16.0 - z0 * 16.0), d(b + y0 * 16.0 - 0.01)],
                            [d(a + 16.0 - z1 * 16.0 - 0.01), d(b + y0 * 16.0 - 0.01)],
                            [d(a + 16.0 - z1 * 16.0 - 0.01), d(b + y1 * 16.0)],
                            [d(a + 16.0 - z0 * 16.0), d(b + y1 * 16.0)],
                        ],
                    ),
                ],
            ),
        ];
        for (fi, (face, set, rots)) in cases.into_iter().enumerate() {
            for (rot, want) in rots {
                let mut r = rb(&s);
                r.set_block_bounds(
                    STONE, x0 as f32, y0 as f32, z0 as f32, x1 as f32, y1 as f32, z1 as f32,
                );
                set(&mut r, rot);
                face(&mut r, STONE, 0.0, 0.0, 0.0, 77);
                assert_eq!(r.tess.uvs, want.to_vec(), "face {fi} rotation {rot}");
            }
        }
    }

    #[test]
    fn flip_and_out_of_cell_bounds() {
        let s = ChunkSnapshot::synthetic([0; 3], [1; 3], table());
        let mut r = rb(&s);
        r.flip_texture = true;
        r.render_east_face(STONE, 0.0, 0.0, 0.0, 0);
        // var12 and var14 swapped: the first vertex takes var20 = old var12
        assert_eq!(r.tess.uvs[0][0], 0.0);
        assert_eq!(r.tess.uvs[1][0], f((16.0 - 0.01) / 256.0));
        // bounds beyond the cell take the whole tile, in float
        let mut r = rb(&s);
        r.set_block_bounds(STONE, -0.125, 0.0, 0.0, 1.0, 1.0, 1.0);
        r.render_top_face(STONE, 0.0, 0.0, 0.0, 0);
        assert_eq!(r.tess.uvs[0][0], ((0.0f32 + 15.99) / 256.0));
        assert_eq!(r.tess.uvs[2][0], 0.0);
    }

    /// A stone block in the open with AO off: face shades 0.5, 1.0, 0.8,
    /// 0.8, 0.6, 0.6 times the brightness of the air beside each face.
    #[test]
    fn plain_cube_shading() {
        let mut s = ChunkSnapshot::synthetic([-1, 0, -1], [2, 3, 2], table());
        s.set(0, 1, 0, STONE, 0, 15, 0);
        s.set(0, 0, 0, 0, 0, 3, 0);
        let mut r = rb(&s);
        assert!(r.render_block_by_render_type(STONE, 0, 1, 0));
        assert_eq!(r.tess.len(), 24);
        let shade = |v: f32| ((v * 255.0) as i32).clamp(0, 255) as u8;
        let t = table();
        let want = [
            0.5 * t[3],
            1.0 * t[15],
            0.8 * t[15],
            0.8 * t[15],
            0.6 * t[15],
            0.6 * t[15],
        ];
        for (face, w) in want.iter().enumerate() {
            assert_eq!(r.tess.colors[face * 4][0], shade(*w), "face {face}");
        }
        // a neighbour stone hides the top face
        s.set(0, 2, 0, STONE, 0, 15, 0);
        let mut r = rb(&s);
        r.render_block_by_render_type(STONE, 0, 1, 0);
        assert_eq!(r.tess.len(), 20);
    }

    /// Grass without AO: the top is tinted, the sides are drawn twice (the
    /// side texture 3 untinted, then the overlay 38 tinted).
    #[test]
    fn grass_side_overlay() {
        let mut s = ChunkSnapshot::synthetic([-1, 0, -1], [2, 3, 2], table());
        let mut grass = vec![0i32; 65536];
        grass.iter_mut().for_each(|c| *c = 0x40_80_20);
        s.colorizers.grass = grass.into();
        s.set(0, 1, 0, GRASS, 0, 15, 0);
        s.set(0, 0, 0, STONE, 0, 15, 0);
        let mut r = rb(&s);
        r.render_block_by_render_type(GRASS, 0, 1, 0);
        // top + 4 sides × 2 (bottom hidden by the stone)
        assert_eq!(r.tess.len(), 4 * 9);
        let t15 = table()[15];
        let q = |v: f32| ((v * 255.0) as i32).clamp(0, 255) as u8;
        assert_eq!(
            r.tess.colors[0],
            [
                q(0x40 as f32 / 255.0 * t15),
                q(0x80 as f32 / 255.0 * t15),
                q(0x20 as f32 / 255.0 * t15),
                255
            ]
        );
        // east side: untinted 0.8, then the overlay with the tint
        let side = q(0.8 * t15);
        assert_eq!(r.tess.colors[4], [side, side, side, 255]);
        let (u0, _, v0, _) = tile(38);
        assert_eq!(r.tess.uvs[9], [f(u0), f(v0)]);
        assert_eq!(r.tess.colors[8][1], q(0.8 * t15 * (0x80 as f32 / 255.0)));
    }

    /// AO mixing on a 2×2×2-ish synthetic world: the top face of a stone
    /// block with one stone block diagonally above (+x, +1, +z) and the
    /// rest air of known light. Each corner is the average of four
    /// brightness values, from the Java formula by hand.
    #[test]
    fn ambient_occlusion_light_mixing() {
        let t = table();
        let mut s = ChunkSnapshot::synthetic([-1, -1, -1], [3, 3, 3], t);
        // light: everything sky 15, except the cells above the block
        for x in -1..2 {
            for z in -1..2 {
                s.set(x, 1, z, 0, 0, 6 + 3 * (x + 1) + (z + 1), 0);
            }
        }
        s.set(0, 0, 0, STONE, 0, 0, 0);
        // a stone diagonally above at (+1, +1, +1): it blocks grass, so the
        // corner (x+1, z+1) is read
        s.set(1, 1, 1, STONE, 0, 0, 0);
        let mut r = RenderBlocks::new(&s, s.statics.clone(), true, false);
        r.render_block_by_render_type(STONE, 0, 0, 0);
        // the top face is the second drawn (the bottom is drawn too: air below)
        let top = &r.tess.colors[4..8];
        let b = |x: i32, z: i32| t[(6 + 3 * (x + 1) + (z + 1)) as usize];
        // var9 (TL) = (A + yPos + B + zPos) / 4 with A = (x, z+1) = field_22362_A,
        // B = (x+1, z+1) = field_22360_B (stone there: light 0 → but the
        // block at (1,1,1) is stone with sky 0)
        let y_pos = b(0, 0);
        let a = b(0, 1);
        let v_ = b(-1, 0);
        let z_ = b(1, 0); // field_22364_z (x+1, y+1, z)
        let x_ = b(0, -1); // field_22366_x
        let stone_corner = t[0];
        // field_22360_B: canBlockGrass at (x, y+1, z+1) is air (true) → read (1,1,1) = 0
        let var9 = (a + y_pos + stone_corner + z_) / 4.0;
        // field_22365_y: (x, y+1, z-1) air → read (1, 1, -1)
        let var10 = (y_pos + x_ + z_ + b(1, -1)) / 4.0;
        let var11 = (v_ + b(-1, -1) + y_pos + x_) / 4.0;
        let var12 = (b(-1, 1) + v_ + a + y_pos) / 4.0;
        let q = |v: f32| ((v * 255.0) as i32).clamp(0, 255) as u8;
        assert_eq!(top[0][0], q(var9));
        assert_eq!(top[1][0], q(var10));
        assert_eq!(top[2][0], q(var11));
        assert_eq!(top[3][0], q(var12));
        // the top face's vertex order: (maxX, maxZ), (maxX, minZ), (minX, minZ), (minX, maxZ)
        assert_eq!(r.tess.positions[4], [1.0, 1.0, 1.0]);
    }

    #[test]
    fn ao_corner_uses_the_edge_value_when_both_edges_are_opaque() {
        let t = table();
        let mut s = ChunkSnapshot::synthetic([-1, -1, -1], [3, 3, 3], t);
        s.set(0, 0, 0, STONE, 0, 0, 0);
        // both edge blocks of the (x+1, z+1) top corner are opaque: (1, 1, 0)
        // and (0, 1, 1); the corner (1, 1, 1) is then not read (it is bright)
        s.set(1, 1, 0, STONE, 0, 0, 0);
        s.set(0, 1, 1, STONE, 0, 0, 0);
        s.set(1, 1, 1, 0, 0, 15, 0);
        s.set(0, 1, 0, 0, 0, 15, 0);
        // the west (z + 1) and south (x + 1) faces, drawn after the top,
        // would overwrite field_22360_B: hide them
        s.set(0, 0, 1, STONE, 0, 0, 0);
        s.set(1, 0, 0, STONE, 0, 0, 0);
        let mut r = RenderBlocks::new(&s, s.statics.clone(), true, false);
        r.render_block_by_render_type(STONE, 0, 0, 0);
        // field_22360_B = field_22364_z (the x edge, stone: 0)
        assert_eq!(r.field_22360_b, r.field_22364_z);
        assert_eq!(r.field_22360_b, t[0]);
    }

    #[test]
    fn inventory_cube_geometry() {
        let statics = BlockStatics::default();
        let d = render_block_as_item(
            &statics,
            &crate::textures::Colorizers::default(),
            STONE,
            0,
            1.0,
        )
        .expect("stone draws");
        assert_eq!(d.tess.len(), 24);
        assert_eq!(d.gl_color, Some([1.0, 1.0, 1.0, 1.0]));
        // centred on the origin
        for p in &d.tess.positions {
            assert!(p.iter().all(|c| *c == -0.5 || *c == 0.5), "{p:?}");
        }
        // normals per face, bottom first, as the Tessellator's bytes (the
        // -y and +x overflow: tessellator module docs)
        assert_eq!(d.tess.normals[0], [0, -127, -1]);
        assert_eq!(d.tess.normals[4], [0, 127, 0]);
        assert_eq!(d.tess.normals[16], [-128, -1, -1]);
        assert_eq!(d.tess.normals[20], [-128, -1, -1]);
        // leaves tinted by getRenderColor(meta) × brightness
        let d = render_block_as_item(
            &statics,
            &crate::textures::Colorizers::default(),
            18,
            1,
            0.5,
        )
        .expect("leaves draw");
        let pine = 0x619961;
        assert_eq!(
            d.gl_color,
            Some([
                (pine >> 16 & 255) as f32 / 255.0 * 0.5,
                (pine >> 8 & 255) as f32 / 255.0 * 0.5,
                (pine & 255) as f32 / 255.0 * 0.5,
                1.0
            ])
        );
        // a button item uses setBlockBoundsForItemRender's box
        let d = render_block_as_item(
            &statics,
            &crate::textures::Colorizers::default(),
            77,
            0,
            1.0,
        )
        .expect("button draws");
        assert_eq!(
            d.tess.positions[0],
            [0.3125 - 0.5, 0.375 - 0.5, 0.625 - 0.5]
        );
        assert!(render_item_in_3d(0) && render_item_in_3d(16) && !render_item_in_3d(1));
    }
}
