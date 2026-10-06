//! `PathEntity` (PathEntity.java): a found path, followed point by point by
//! `EntityCreature.updatePlayerActionState`.

use mc_core::Vec3D;

use super::path_point::PathPoint;
use crate::entity::EntityBase;

/// `PathEntity`: the points start to end (copies of the search's points;
/// only their coordinates are read after the search).
#[derive(Clone, Debug, PartialEq)]
pub struct PathEntity {
    /// `points` (private, final).
    points: Vec<PathPoint>,
    /// `pathLength` (public, final): `points.length`.
    pub path_length: i32,
    /// `pathIndex` (private): the next point to walk to.
    path_index: i32,
}

impl PathEntity {
    /// `new PathEntity(points)`.
    // src: PathEntity.java:8-11
    pub fn new(points: Vec<PathPoint>) -> PathEntity {
        let path_length = points.len() as i32;
        PathEntity {
            points,
            path_length,
            path_index: 0,
        }
    }

    /// `incrementPathIndex()`.
    // src: PathEntity.java:13-15
    pub fn increment_path_index(&mut self) {
        self.path_index += 1;
    }

    /// `isFinished()`.
    // src: PathEntity.java:17-19
    pub fn is_finished(&self) -> bool {
        self.path_index >= self.points.len() as i32
    }

    /// `func_22328_c()`: the last point (`null` for an empty path). Its only
    /// callers are in the multiplayer server tree; kept for completeness.
    // src: PathEntity.java:21-23
    pub fn func_22328_c(&self) -> Option<&PathPoint> {
        if self.path_length > 0 {
            self.points.get((self.path_length - 1) as usize)
        } else {
            None
        }
    }

    /// `getPosition(entity)`: the current point's cell, centred by
    /// `(int)(width + 1) * 0.5` horizontally (so 0.5 for widths below 1,
    /// 1.0 for widths 1 to 2); `y` is the cell's bottom.
    ///
    /// Java throws `ArrayIndexOutOfBoundsException` past the end; callers
    /// test `isFinished` first.
    // src: PathEntity.java:25-30
    pub fn get_position(&self, e: &EntityBase) -> Vec3D {
        let p = &self.points[self.path_index as usize];
        let var2 = p.x_coord as f64 + ((e.width + 1.0) as i32) as f64 * 0.5;
        let var4 = p.y_coord as f64;
        let var6 = p.z_coord as f64 + ((e.width + 1.0) as i32) as f64 * 0.5;
        Vec3D::create_vector(var2, var4, var6)
    }

    /// The points, start to end (the port's view for tests and debugging).
    pub fn points(&self) -> &[PathPoint] {
        &self.points
    }

    /// `pathIndex`.
    pub fn path_index(&self) -> i32 {
        self.path_index
    }
}
