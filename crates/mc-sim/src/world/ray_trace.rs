//! Block ray tracing (`World.rayTraceBlocks*`, World.java:704-861),
//! `MovingObjectPosition` / `EnumMovingObjectType`, and the explosion helper
//! `World.getBlockDensity` (World.java:1530-1553).
//!
//! # The start vector is mutated (vanilla, transcribed)
//!
//! `rayTraceBlocks_do_do` walks by assigning to its `Vec3D` argument's
//! coordinates (World.java:799-821), so the caller's start vector ends at the
//! last face crossed. The Rust functions take `start: &mut Vec3D` for the
//! same effect; callers that pass a fresh vector (all vanilla callers build
//! one with `Vec3D.createVector` just before the call) can ignore it.

use mc_core::Aabb;
use mc_core::Vec3D;
use mc_core::math::floor_double;

use super::World;
use crate::block::{self, dispatch as block_dispatch};

/// `EnumMovingObjectType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnumMovingObjectType {
    /// `TILE`: a block.
    Tile,
    /// `ENTITY`.
    Entity,
}

/// `MovingObjectPosition`: a ray-trace hit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MovingObjectPosition {
    /// `typeOfHit`.
    pub type_of_hit: EnumMovingObjectType,
    /// `blockX` (0 for an entity hit).
    pub block_x: i32,
    /// `blockY`.
    pub block_y: i32,
    /// `blockZ`.
    pub block_z: i32,
    /// `sideHit`: 0 = -y, 1 = +y, 2 = -z, 3 = +z, 4 = -x, 5 = +x (0 for an
    /// entity hit).
    pub side_hit: i32,
    /// `hitVec`.
    pub hit_vec: Vec3D,
    /// `entityHit`, as an entity id.
    pub entity_hit: Option<i32>,
}

impl MovingObjectPosition {
    /// `new MovingObjectPosition(x, y, z, side, hitVec)`: a `TILE` hit; the
    /// vector is copied with `Vec3D.createVector` (no `-0.0` normalisation).
    // src: MovingObjectPosition.java:12-19
    pub fn tile(x: i32, y: i32, z: i32, side: i32, hit: Vec3D) -> Self {
        MovingObjectPosition {
            type_of_hit: EnumMovingObjectType::Tile,
            block_x: x,
            block_y: y,
            block_z: z,
            side_hit: side,
            hit_vec: Vec3D::create_vector(hit.x_coord, hit.y_coord, hit.z_coord),
            entity_hit: None,
        }
    }

    /// `new MovingObjectPosition(entity)`: an `ENTITY` hit at the entity's
    /// position.
    // src: MovingObjectPosition.java:21-25
    pub fn entity(entity_id: i32, pos_x: f64, pos_y: f64, pos_z: f64) -> Self {
        MovingObjectPosition {
            type_of_hit: EnumMovingObjectType::Entity,
            block_x: 0,
            block_y: 0,
            block_z: 0,
            side_hit: 0,
            hit_vec: Vec3D::create_vector(pos_x, pos_y, pos_z),
            entity_hit: Some(entity_id),
        }
    }
}

impl World {
    /// `rayTraceBlocks(start, end)`: no liquids, blocks without a collision
    /// box included.
    // src: World.java:704-706
    pub fn ray_trace_blocks(
        &mut self,
        start: &mut Vec3D,
        end: &Vec3D,
    ) -> Option<MovingObjectPosition> {
        self.ray_trace_blocks_do_do(start, end, false, false)
    }

    /// `rayTraceBlocks_do(start, end, stopOnLiquid)`.
    // src: World.java:708-710
    pub fn ray_trace_blocks_do(
        &mut self,
        start: &mut Vec3D,
        end: &Vec3D,
        stop_on_liquid: bool,
    ) -> Option<MovingObjectPosition> {
        self.ray_trace_blocks_do_do(start, end, stop_on_liquid, false)
    }

    /// `rayTraceBlocks_do(Vec3D.createVector(from), Vec3D.createVector(to),
    /// stopOnLiquid)`: a trace between two points on fresh vectors (the
    /// shape of every vanilla call site, and of the oracle's `ray_trace`
    /// dump).
    pub fn ray_trace_between(
        &mut self,
        from: [f64; 3],
        to: [f64; 3],
        stop_on_liquid: bool,
    ) -> Option<MovingObjectPosition> {
        let mut start = Vec3D::create_vector(from[0], from[1], from[2]);
        let end = Vec3D::create_vector(to[0], to[1], to[2]);
        self.ray_trace_blocks_do(&mut start, &end, stop_on_liquid)
    }

    /// `rayTraceBlocks_do_do(start, end, stopOnLiquid, ignoreNoCollision)`:
    /// steps block by block from `start` toward `end` (at most 201 steps
    /// after the start block), asking each block it enters for
    /// `collisionRayTrace`. Blocks are loaded on the way (`getBlockId`).
    ///
    /// The start block is tested first with `Block.blocksList[id]` read before
    /// the `id > 0` test: for an air start with `ignoreNoCollision` the
    /// collision box of a null block is skipped by `var13 == null` (Java's
    /// short-circuit), so nothing is dereferenced.
    // src: World.java:712-861
    pub fn ray_trace_blocks_do_do(
        &mut self,
        var1: &mut Vec3D,
        var2: &Vec3D,
        var3: bool,
        var4: bool,
    ) -> Option<MovingObjectPosition> {
        if var1.x_coord.is_nan() || var1.y_coord.is_nan() || var1.z_coord.is_nan() {
            return None;
        }
        if var2.x_coord.is_nan() || var2.y_coord.is_nan() || var2.z_coord.is_nan() {
            return None;
        }
        let var5 = floor_double(var2.x_coord);
        let var6 = floor_double(var2.y_coord);
        let var7 = floor_double(var2.z_coord);
        let mut var8 = floor_double(var1.x_coord);
        let mut var9 = floor_double(var1.y_coord);
        let mut var10 = floor_double(var1.z_coord);
        // src: World.java:721-729
        if let Some(hit) = self.ray_trace_test_block(var8, var9, var10, var1, var2, var3, var4) {
            return Some(hit);
        }

        // src: World.java:731-852 (`while (var11-- >= 0)`: 201 iterations)
        let mut var11 = 200;
        loop {
            let go = var11 >= 0;
            var11 -= 1;
            if !go {
                break;
            }
            if var1.x_coord.is_nan() || var1.y_coord.is_nan() || var1.z_coord.is_nan() {
                return None;
            }
            if var8 == var5 && var9 == var6 && var10 == var7 {
                return None;
            }
            let mut var39 = true;
            let mut var40 = true;
            let mut var41 = true;
            let mut var15 = 999.0;
            let mut var17 = 999.0;
            let mut var19 = 999.0;
            if var5 > var8 {
                var15 = var8 as f64 + 1.0;
            } else if var5 < var8 {
                var15 = var8 as f64 + 0.0;
            } else {
                var39 = false;
            }
            if var6 > var9 {
                var17 = var9 as f64 + 1.0;
            } else if var6 < var9 {
                var17 = var9 as f64 + 0.0;
            } else {
                var40 = false;
            }
            if var7 > var10 {
                var19 = var10 as f64 + 1.0;
            } else if var7 < var10 {
                var19 = var10 as f64 + 0.0;
            } else {
                var41 = false;
            }
            let mut var21 = 999.0;
            let mut var23 = 999.0;
            let mut var25 = 999.0;
            let var27 = var2.x_coord - var1.x_coord;
            let var29 = var2.y_coord - var1.y_coord;
            let var31 = var2.z_coord - var1.z_coord;
            if var39 {
                var21 = (var15 - var1.x_coord) / var27;
            }
            if var40 {
                var23 = (var17 - var1.y_coord) / var29;
            }
            if var41 {
                var25 = (var19 - var1.z_coord) / var31;
            }
            // vanilla bug: the walk moves the caller's start vector (module
            // docs); callers that reuse it see the last face crossed.
            let var42: i32;
            if var21 < var23 && var21 < var25 {
                var42 = if var5 > var8 { 4 } else { 5 };
                var1.x_coord = var15;
                var1.y_coord += var29 * var21;
                var1.z_coord += var31 * var21;
            } else if var23 < var25 {
                var42 = if var6 > var9 { 0 } else { 1 };
                var1.x_coord += var27 * var23;
                var1.y_coord = var17;
                var1.z_coord += var31 * var23;
            } else {
                var42 = if var7 > var10 { 2 } else { 3 };
                var1.x_coord += var27 * var25;
                var1.y_coord += var29 * var25;
                var1.z_coord = var19;
            }
            // src: World.java:824-841. `var34` (a pooled copy) is written and
            // never read again; only the integer coordinates matter.
            var8 = floor_double(var1.x_coord);
            if var42 == 5 {
                var8 -= 1;
            }
            var9 = floor_double(var1.y_coord);
            if var42 == 1 {
                var9 -= 1;
            }
            var10 = floor_double(var1.z_coord);
            if var42 == 3 {
                var10 -= 1;
            }
            // src: World.java:843-851
            if let Some(hit) = self.ray_trace_test_block(var8, var9, var10, var1, var2, var3, var4)
            {
                return Some(hit);
            }
        }
        None
    }

    /// The per-block test of `rayTraceBlocks_do_do` (World.java:721-729 and
    /// :843-851): `(!ignoreNoCollision || block == null ||
    /// block.getCollisionBoundingBoxFromPool(...) != null) && id > 0 &&
    /// block.canCollideCheck(meta, stopOnLiquid)`, then `collisionRayTrace`.
    #[allow(clippy::too_many_arguments)]
    fn ray_trace_test_block(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        start: &Vec3D,
        end: &Vec3D,
        stop_on_liquid: bool,
        ignore_no_collision: bool,
    ) -> Option<MovingObjectPosition> {
        let id = self.get_block_id(x, y, z);
        let meta = self.get_block_metadata(x, y, z);
        let known = block::block(id).is_some();
        let box_ok = !ignore_no_collision
            || !known
            || block_dispatch::get_collision_bounding_box_from_pool(self, id, x, y, z).is_some();
        if box_ok && id > 0 && block_dispatch::can_collide_check(id, meta, stop_on_liquid) {
            return block_dispatch::collision_ray_trace(self, id, x, y, z, start, end);
        }
        None
    }

    /// `getBlockDensity(vec, box)`: the fraction of sample points of `box`
    /// (a grid with `float` steps of `1 / (2·size + 1)`) from which a ray to
    /// `vec` reaches it unblocked. Used by `Explosion` (WP-1.8).
    ///
    /// The loop counters are `float`s advanced through `double`
    /// (`(float)((double)f + step)`), as in Java.
    // src: World.java:1530-1553
    pub fn get_block_density(&mut self, var1: &Vec3D, var2: &Aabb) -> f32 {
        let var3 = 1.0 / ((var2.max_x - var2.min_x) * 2.0 + 1.0);
        let var5 = 1.0 / ((var2.max_y - var2.min_y) * 2.0 + 1.0);
        let var7 = 1.0 / ((var2.max_z - var2.min_z) * 2.0 + 1.0);
        let mut var9 = 0i32;
        let mut var10 = 0i32;
        let mut var11 = 0.0f32;
        while var11 <= 1.0 {
            let mut var12 = 0.0f32;
            while var12 <= 1.0 {
                let mut var13 = 0.0f32;
                while var13 <= 1.0 {
                    let var14 = var2.min_x + (var2.max_x - var2.min_x) * var11 as f64;
                    let var16 = var2.min_y + (var2.max_y - var2.min_y) * var12 as f64;
                    let var18 = var2.min_z + (var2.max_z - var2.min_z) * var13 as f64;
                    let mut start = Vec3D::create_vector(var14, var16, var18);
                    if self.ray_trace_blocks(&mut start, var1).is_none() {
                        var9 += 1;
                    }
                    var10 += 1;
                    var13 = (var13 as f64 + var7) as f32;
                }
                var12 = (var12 as f64 + var5) as f32;
            }
            var11 = (var11 as f64 + var3) as f32;
        }
        var9 as f32 / var10 as f32
    }
}
