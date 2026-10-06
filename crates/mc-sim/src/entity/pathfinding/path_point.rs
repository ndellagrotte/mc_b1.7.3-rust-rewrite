//! `PathPoint` (PathPoint.java): one cell of a path search.

use mc_core::math;

/// `PathPoint`. `previous` is an index into the search's arena (module docs
/// of [`super`]).
#[derive(Clone, Debug, PartialEq)]
pub struct PathPoint {
    /// `xCoord`.
    pub x_coord: i32,
    /// `yCoord`.
    pub y_coord: i32,
    /// `zCoord`.
    pub z_coord: i32,
    /// `hash` (private): [`PathPoint::make_hash`] of the coordinates.
    pub hash: i32,
    /// `index`: the slot in the open-set heap, -1 when not in it.
    pub index: i32,
    /// `totalPathDistance`: the cost from the start (A*'s g).
    pub total_path_distance: f32,
    /// `distanceToNext`: the straight distance to the target (A*'s h).
    pub distance_to_next: f32,
    /// `distanceToTarget`: the heap key (g + h).
    pub distance_to_target: f32,
    /// `previous`, as an arena index.
    pub previous: Option<usize>,
    /// `isFirst`: set when the point is taken off the open set (the closed
    /// set; Pathfinder.java:49).
    pub is_first: bool,
}

impl PathPoint {
    /// `new PathPoint(x, y, z)`.
    // src: PathPoint.java:15-20
    pub fn new(x: i32, y: i32, z: i32) -> PathPoint {
        PathPoint {
            x_coord: x,
            y_coord: y,
            z_coord: z,
            hash: Self::make_hash(x, y, z),
            index: -1,
            total_path_distance: 0.0,
            distance_to_next: 0.0,
            distance_to_target: 0.0,
            previous: None,
            is_first: false,
        }
    }

    /// `func_22329_a(x, y, z)`: the point-map key, `y & 255 | (x & 32767) << 8
    /// | (z & 32767) << 24 | (x < 0 ? 0x80000000 : 0) | (z < 0 ? 0x8000 : 0)`.
    /// Not injective (module docs of [`super`]).
    // src: PathPoint.java:22-24
    pub fn make_hash(x: i32, y: i32, z: i32) -> i32 {
        (y & 255)
            | (x & 32767) << 8
            | (z & 32767).wrapping_shl(24)
            | if x < 0 { i32::MIN } else { 0 }
            | if z < 0 { 0x8000 } else { 0 }
    }

    /// `distanceTo(other)`: Euclidean, in `float`.
    // src: PathPoint.java:26-31
    pub fn distance_to(&self, other: &PathPoint) -> f32 {
        let var2 = (other.x_coord - self.x_coord) as f32;
        let var3 = (other.y_coord - self.y_coord) as f32;
        let var4 = (other.z_coord - self.z_coord) as f32;
        math::sqrt_float(var2 * var2 + var3 * var3 + var4 * var4)
    }

    /// `equals(other)`: same hash and coordinates.
    // src: PathPoint.java:33-40
    pub fn equals(&self, other: &PathPoint) -> bool {
        self.hash == other.hash
            && self.x_coord == other.x_coord
            && self.y_coord == other.y_coord
            && self.z_coord == other.z_coord
    }

    /// `isAssigned()`: in the open-set heap.
    // src: PathPoint.java:46-48
    pub fn is_assigned(&self) -> bool {
        self.index >= 0
    }
}
