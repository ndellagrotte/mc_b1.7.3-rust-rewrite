//! `Pathfinder` (Pathfinder.java): A* over block cells for an entity's
//! box, four horizontal neighbours per cell, stepping up one block and
//! dropping down at most three.

use std::collections::BTreeMap;

use mc_core::math::{floor_double, floor_float};

use super::PathBlockAccess;
use super::path::Path;
use super::path_entity::PathEntity;
use super::path_point::PathPoint;
use crate::block::{self, Material};
use crate::entity::EntityBase;

/// `Block.doorWood.blockID`.
// src: Block.java:656
const DOOR_WOOD: i32 = 64;
/// `Block.doorSteel.blockID`.
// src: Block.java:663
const DOOR_STEEL: i32 = 71;

/// `Pathfinder`. One search per instance in vanilla
/// (`new Pathfinder(chunkCache)` at World.java:2128, :2143); the point arena
/// and map are cleared at the start of each search anyway.
pub struct Pathfinder<'a, A: PathBlockAccess> {
    /// `worldMap`.
    world_map: &'a A,
    /// `path`: the open set.
    path: Path,
    /// The `PathPoint` objects of this search (module docs of [`super`]).
    points: Vec<PathPoint>,
    /// `pointMap` (`MCHash`): point hash → arena index, lookup only.
    point_map: BTreeMap<i32, usize>,
    /// `pathOptions` (32 slots, at most 4 used).
    path_options: [usize; 32],
}

impl<'a, A: PathBlockAccess> Pathfinder<'a, A> {
    /// `new Pathfinder(blockAccess)`.
    // src: Pathfinder.java:4-11
    pub fn new(world_map: &'a A) -> Self {
        Pathfinder {
            world_map,
            path: Path::new(),
            points: Vec::new(),
            point_map: BTreeMap::new(),
            path_options: [0; 32],
        }
    }

    /// `createEntityPathTo(entity, target, range)`: to the target's
    /// `(posX, boundingBox.minY, posZ)`.
    // src: Pathfinder.java:13-15
    pub fn create_entity_path_to_entity(
        &mut self,
        e: &EntityBase,
        target: &EntityBase,
        var3: f32,
    ) -> Option<PathEntity> {
        self.create_entity_path_to(
            e,
            target.pos_x,
            target.bounding_box.min_y,
            target.pos_z,
            var3,
        )
    }

    /// `createEntityPathTo(entity, x, y, z, range)` (the `int` overload): to
    /// the centre of the block, `(double)((float)x + 0.5F)`.
    // src: Pathfinder.java:17-19
    pub fn create_entity_path_to_xyz(
        &mut self,
        e: &EntityBase,
        x: i32,
        y: i32,
        z: i32,
        var5: f32,
    ) -> Option<PathEntity> {
        self.create_entity_path_to(
            e,
            (x as f32 + 0.5) as f64,
            (y as f32 + 0.5) as f64,
            (z as f32 + 0.5) as f64,
            var5,
        )
    }

    /// `createEntityPathTo(entity, x, y, z, range)` (the private `double`
    /// overload): start = the cell of the box's min corner, end = the cell
    /// of `(x - width/2, y, z - width/2)`, size = `floor(width + 1)`,
    /// `floor(height + 1)`.
    // src: Pathfinder.java:21-29
    fn create_entity_path_to(
        &mut self,
        e: &EntityBase,
        var2: f64,
        var4: f64,
        var6: f64,
        var8: f32,
    ) -> Option<PathEntity> {
        self.path.clear_path();
        self.point_map.clear();
        self.points.clear();
        let var9 = self.open_point(
            floor_double(e.bounding_box.min_x),
            floor_double(e.bounding_box.min_y),
            floor_double(e.bounding_box.min_z),
        );
        let var10 = self.open_point(
            floor_double(var2 - (e.width / 2.0) as f64),
            floor_double(var4),
            floor_double(var6 - (e.width / 2.0) as f64),
        );
        let var11 = PathPoint::new(
            floor_float(e.width + 1.0),
            floor_float(e.height + 1.0),
            floor_float(e.width + 1.0),
        );
        self.add_to_path(var9, var10, &var11, var8)
    }

    /// `addToPath(entity, start, end, size, range)`: the A* loop. Returns the
    /// path to `end` if it is reached, else to the point closest to `end`,
    /// or `None` if that is the start itself.
    // src: Pathfinder.java:31-74
    fn add_to_path(
        &mut self,
        var2: usize,
        var3: usize,
        var4: &PathPoint,
        var5: f32,
    ) -> Option<PathEntity> {
        let pts = &mut self.points;
        pts[var2].total_path_distance = 0.0;
        pts[var2].distance_to_next = pts[var2].distance_to(&pts[var3]);
        pts[var2].distance_to_target = pts[var2].distance_to_next;
        self.path.clear_path();
        self.path.add_point(&mut self.points, var2);
        let mut var6 = var2;
        while !self.path.is_path_empty() {
            let var7 = self.path.dequeue(&mut self.points);
            if self.points[var7].equals(&self.points[var3]) {
                return Some(self.create_entity_path(var2, var3));
            }
            if self.points[var7].distance_to(&self.points[var3])
                < self.points[var6].distance_to(&self.points[var3])
            {
                var6 = var7;
            }
            self.points[var7].is_first = true;
            let var8 = self.find_path_options(var7, var4, var3, var5);
            for var9 in 0..var8 {
                let var10 = self.path_options[var9];
                let pts = &mut self.points;
                let var11 = pts[var7].total_path_distance + pts[var7].distance_to(&pts[var10]);
                if !pts[var10].is_assigned() || var11 < pts[var10].total_path_distance {
                    pts[var10].previous = Some(var7);
                    pts[var10].total_path_distance = var11;
                    pts[var10].distance_to_next = pts[var10].distance_to(&pts[var3]);
                    if pts[var10].is_assigned() {
                        let d = pts[var10].total_path_distance + pts[var10].distance_to_next;
                        self.path.change_distance(&mut self.points, var10, d);
                    } else {
                        pts[var10].distance_to_target =
                            pts[var10].total_path_distance + pts[var10].distance_to_next;
                        self.path.add_point(&mut self.points, var10);
                    }
                }
            }
        }
        if var6 == var2 {
            None
        } else {
            Some(self.create_entity_path(var2, var6))
        }
    }

    /// `findPathOptions(entity, point, size, target, range)`: the safe
    /// neighbours `+z, -x, +x, -z` (in that order) that are not closed and
    /// lie within `range` of the target; one step up is allowed when the
    /// cell above `point` is free.
    // src: Pathfinder.java:76-104
    fn find_path_options(
        &mut self,
        var2: usize,
        var3: &PathPoint,
        var4: usize,
        var5: f32,
    ) -> usize {
        let mut var6 = 0;
        let (x, y, z) = {
            let p = &self.points[var2];
            (p.x_coord, p.y_coord, p.z_coord)
        };
        let mut var7 = 0;
        if self.get_vertical_offset(x, y + 1, z, var3) == 1 {
            var7 = 1;
        }
        let var8 = self.get_safe_point(x, y, z + 1, var3, var7);
        let var9 = self.get_safe_point(x - 1, y, z, var3, var7);
        let var10 = self.get_safe_point(x + 1, y, z, var3, var7);
        let var11 = self.get_safe_point(x, y, z - 1, var3, var7);
        for p in [var8, var9, var10, var11].into_iter().flatten() {
            let pt = &self.points[p];
            if !pt.is_first && pt.distance_to(&self.points[var4]) < var5 {
                self.path_options[var6] = p;
                var6 += 1;
            }
        }
        var6
    }

    /// `getSafePoint(entity, x, y, z, size, stepUp)`: the cell at `y` if the
    /// box fits there, else at `y + stepUp`; then down while the cell below
    /// is free, at most three blocks (a fourth free cell refuses the point),
    /// and refused if the drop ends on lava (-2).
    // src: Pathfinder.java:106-139
    fn get_safe_point(
        &mut self,
        var2: i32,
        mut var3: i32,
        var4: i32,
        var5: &PathPoint,
        var6: i32,
    ) -> Option<usize> {
        let mut var7 = None;
        if self.get_vertical_offset(var2, var3, var4, var5) == 1 {
            var7 = Some(self.open_point(var2, var3, var4));
        }
        if var7.is_none()
            && var6 > 0
            && self.get_vertical_offset(var2, var3 + var6, var4, var5) == 1
        {
            var7 = Some(self.open_point(var2, var3 + var6, var4));
            var3 += var6;
        }
        if var7.is_some() {
            let mut var8 = 0;
            let mut var9 = 0;
            while var3 > 0 && {
                var9 = self.get_vertical_offset(var2, var3 - 1, var4, var5);
                var9 == 1
            } {
                var8 += 1;
                if var8 >= 4 {
                    return None;
                }
                var3 -= 1;
                if var3 > 0 {
                    var7 = Some(self.open_point(var2, var3, var4));
                }
            }
            if var9 == -2 {
                return None;
            }
        }
        var7
    }

    /// `openPoint(x, y, z)`: the point with this hash, made on first use.
    /// Keyed by the hash only (module docs of [`super`]).
    // src: Pathfinder.java:141-150
    fn open_point(&mut self, var1: i32, var2: i32, var3: i32) -> usize {
        let var4 = PathPoint::make_hash(var1, var2, var3);
        if let Some(&p) = self.point_map.get(&var4) {
            return p;
        }
        let p = self.points.len();
        self.points.push(PathPoint::new(var1, var2, var3));
        self.point_map.insert(var4, p);
        p
    }

    /// `getVerticalOffset(entity, x, y, z, size)`: over the box of `size`
    /// cells at `(x, y, z)`: 0 at the first solid block or closed door, -1
    /// at the first water, -2 at the first lava (whichever comes first in
    /// x, y, z loop order), else 1 (free). Non-solid blocks (ladders,
    /// plants, torches, …) are free.
    // src: Pathfinder.java:152-183
    fn get_vertical_offset(&self, var2: i32, var3: i32, var4: i32, var5: &PathPoint) -> i32 {
        for var6 in var2..var2 + var5.x_coord {
            for var7 in var3..var3 + var5.y_coord {
                for var8 in var4..var4 + var5.z_coord {
                    let var9 = self.world_map.get_block_id(var6, var7, var8);
                    if var9 > 0 {
                        if var9 != DOOR_STEEL && var9 != DOOR_WOOD {
                            let var11 = block::material_of(var9);
                            if var11.get_is_solid() {
                                return 0;
                            }
                            if var11 == Material::Water {
                                return -1;
                            }
                            if var11 == Material::Lava {
                                return -2;
                            }
                        } else {
                            let var10 = self.world_map.get_block_metadata(var6, var7, var8);
                            if !block::door::is_open(var10) {
                                return 0;
                            }
                        }
                    }
                }
            }
        }
        1
    }

    /// `createEntityPath(start, end)`: follows `previous` from `end` back to
    /// the point without one.
    // src: Pathfinder.java:185-203
    fn create_entity_path(&self, _var1: usize, var2: usize) -> PathEntity {
        let mut var5 = Vec::new();
        let mut var4 = Some(var2);
        while let Some(p) = var4 {
            var5.push(self.points[p].clone());
            var4 = self.points[p].previous;
        }
        var5.reverse();
        PathEntity::new(var5)
    }
}
