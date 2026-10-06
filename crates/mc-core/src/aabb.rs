//! `AxisAlignedBB`, as a value type.
//!
//! Vanilla allocates boxes from a static pool (`getBoundingBoxFromPool`,
//! rewound by `clearBoundingBoxPool` every tick; AxisAlignedBB.java:7-35) to
//! avoid garbage. The pool is a memory optimisation only, so here every
//! factory is the plain constructor [`Aabb::new`]: `getBoundingBox` and
//! `getBoundingBoxFromPool` both map to it, and `clearBoundingBoxes` /
//! `clearBoundingBoxPool` have no counterpart. Methods that return a pooled
//! box return a new value; the two that mutate `this` (`setBounds`,
//! `offset`) take `&mut self`, and `setBB` is [`Aabb::set_bb`].
//!
//! Not transcribed: `toString` (Java `Double.toString` formatting; debug
//! output only). Use `Debug`.

use crate::vec3d::Vec3D;

/// A `calculateIntercept` candidate: the crossing point (if the segment
/// reaches that plane), the in-face test, and the side id.
type Candidate = (Option<Vec3D>, fn(&Aabb, &Vec3D) -> bool, i32);

/// `AxisAlignedBB`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    /// `minX`.
    pub min_x: f64,
    /// `minY`.
    pub min_y: f64,
    /// `minZ`.
    pub min_z: f64,
    /// `maxX`.
    pub max_x: f64,
    /// `maxY`.
    pub max_y: f64,
    /// `maxZ`.
    pub max_z: f64,
}

/// Result of [`Aabb::calculate_intercept`]: vanilla returns
/// `new MovingObjectPosition(0, 0, 0, side, hit)` (a `TILE` hit at block
/// 0,0,0); callers fill in the block coordinates. That type belongs to the
/// simulation, so only the two meaningful fields are returned here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AabbIntercept {
    /// `sideHit`: 0 = min Y, 1 = max Y, 2 = min Z, 3 = max Z, 4 = min X,
    /// 5 = max X.
    // src: AxisAlignedBB.java:299-322
    pub side_hit: i32,
    /// `hitVec`.
    pub hit_vec: Vec3D,
}

impl Aabb {
    /// `AxisAlignedBB.getBoundingBox(...)` and
    /// `AxisAlignedBB.getBoundingBoxFromPool(...)`; see the module docs.
    // src: AxisAlignedBB.java:16-18, AxisAlignedBB.java:29-35, AxisAlignedBB.java:37-44
    pub const fn new(
        min_x: f64,
        min_y: f64,
        min_z: f64,
        max_x: f64,
        max_y: f64,
        max_z: f64,
    ) -> Self {
        Self {
            min_x,
            min_y,
            min_z,
            max_x,
            max_y,
            max_z,
        }
    }

    /// `setBounds(...)`: overwrites all six bounds in place.
    // src: AxisAlignedBB.java:46-54
    pub fn set_bounds(
        &mut self,
        min_x: f64,
        min_y: f64,
        min_z: f64,
        max_x: f64,
        max_y: f64,
        max_z: f64,
    ) -> &mut Self {
        *self = Self::new(min_x, min_y, min_z, max_x, max_y, max_z);
        self
    }

    /// `addCoord(x, y, z)`: stretches the box towards the motion vector
    /// (negative components move the min side, positive the max side).
    // src: AxisAlignedBB.java:56-88
    pub fn add_coord(&self, x: f64, y: f64, z: f64) -> Self {
        let mut b = *self;
        if x < 0.0 {
            b.min_x += x;
        }
        if x > 0.0 {
            b.max_x += x;
        }
        if y < 0.0 {
            b.min_y += y;
        }
        if y > 0.0 {
            b.max_y += y;
        }
        if z < 0.0 {
            b.min_z += z;
        }
        if z > 0.0 {
            b.max_z += z;
        }
        b
    }

    /// `expand(x, y, z)`: grows each side by the given amount.
    // src: AxisAlignedBB.java:90-98
    pub fn expand(&self, x: f64, y: f64, z: f64) -> Self {
        Self::new(
            self.min_x - x,
            self.min_y - y,
            self.min_z - z,
            self.max_x + x,
            self.max_y + y,
            self.max_z + z,
        )
    }

    /// `getOffsetBoundingBox(x, y, z)`: a translated copy.
    // src: AxisAlignedBB.java:100-102
    pub fn get_offset_bounding_box(&self, x: f64, y: f64, z: f64) -> Self {
        Self::new(
            self.min_x + x,
            self.min_y + y,
            self.min_z + z,
            self.max_x + x,
            self.max_y + y,
            self.max_z + z,
        )
    }

    /// `calculateXOffset(other, dx)`: clips the movement `dx` of `other`
    /// against `self` if they overlap in Y and Z.
    // src: AxisAlignedBB.java:104-129
    pub fn calculate_x_offset(&self, other: &Aabb, mut dx: f64) -> f64 {
        // Nested `if`s in vanilla, merged; same short-circuit order.
        if other.max_y > self.min_y
            && other.min_y < self.max_y
            && other.max_z > self.min_z
            && other.min_z < self.max_z
        {
            if dx > 0.0 && other.max_x <= self.min_x {
                let d = self.min_x - other.max_x;
                if d < dx {
                    dx = d;
                }
            }
            if dx < 0.0 && other.min_x >= self.max_x {
                let d = self.max_x - other.min_x;
                if d > dx {
                    dx = d;
                }
            }
        }
        dx
    }

    /// `calculateYOffset(other, dy)`: as [`Self::calculate_x_offset`] on Y,
    /// requiring overlap in X and Z.
    // src: AxisAlignedBB.java:131-156
    pub fn calculate_y_offset(&self, other: &Aabb, mut dy: f64) -> f64 {
        // Nested `if`s in vanilla, merged; same short-circuit order.
        if other.max_x > self.min_x
            && other.min_x < self.max_x
            && other.max_z > self.min_z
            && other.min_z < self.max_z
        {
            if dy > 0.0 && other.max_y <= self.min_y {
                let d = self.min_y - other.max_y;
                if d < dy {
                    dy = d;
                }
            }
            if dy < 0.0 && other.min_y >= self.max_y {
                let d = self.max_y - other.min_y;
                if d > dy {
                    dy = d;
                }
            }
        }
        dy
    }

    /// `calculateZOffset(other, dz)`: as [`Self::calculate_x_offset`] on Z,
    /// requiring overlap in X and Y.
    // src: AxisAlignedBB.java:158-183
    pub fn calculate_z_offset(&self, other: &Aabb, mut dz: f64) -> f64 {
        // Nested `if`s in vanilla, merged; same short-circuit order.
        if other.max_x > self.min_x
            && other.min_x < self.max_x
            && other.max_y > self.min_y
            && other.min_y < self.max_y
        {
            if dz > 0.0 && other.max_z <= self.min_z {
                let d = self.min_z - other.max_z;
                if d < dz {
                    dz = d;
                }
            }
            if dz < 0.0 && other.min_z >= self.max_z {
                let d = self.max_z - other.min_z;
                if d > dz {
                    dz = d;
                }
            }
        }
        dz
    }

    /// `intersectsWith(other)`: strict overlap on all three axes (touching
    /// faces do not intersect).
    // src: AxisAlignedBB.java:185-195
    pub fn intersects_with(&self, other: &Aabb) -> bool {
        other.max_x > self.min_x
            && other.min_x < self.max_x
            && other.max_y > self.min_y
            && other.min_y < self.max_y
            && other.max_z > self.min_z
            && other.min_z < self.max_z
    }

    /// `offset(x, y, z)`: translates **this** box in place (unlike
    /// [`Self::get_offset_bounding_box`]) and returns it.
    // src: AxisAlignedBB.java:197-205
    pub fn offset(&mut self, x: f64, y: f64, z: f64) -> &mut Self {
        self.min_x += x;
        self.min_y += y;
        self.min_z += z;
        self.max_x += x;
        self.max_y += y;
        self.max_z += z;
        self
    }

    /// `isVecInside(v)`: strictly inside on all three axes.
    // src: AxisAlignedBB.java:207-217
    pub fn is_vec_inside(&self, v: &Vec3D) -> bool {
        v.x_coord > self.min_x
            && v.x_coord < self.max_x
            && v.y_coord > self.min_y
            && v.y_coord < self.max_y
            && v.z_coord > self.min_z
            && v.z_coord < self.max_z
    }

    /// `getAverageEdgeLength()`.
    // src: AxisAlignedBB.java:219-224
    pub fn get_average_edge_length(&self) -> f64 {
        let dx = self.max_x - self.min_x;
        let dy = self.max_y - self.min_y;
        let dz = self.max_z - self.min_z;
        (dx + dy + dz) / 3.0
    }

    /// `contract(x, y, z)`: shrinks each side by the given amount.
    // src: AxisAlignedBB.java:226-234
    pub fn contract(&self, x: f64, y: f64, z: f64) -> Self {
        Self::new(
            self.min_x + x,
            self.min_y + y,
            self.min_z + z,
            self.max_x - x,
            self.max_y - y,
            self.max_z - z,
        )
    }

    /// `copy()`.
    // src: AxisAlignedBB.java:236-238
    pub fn copy(&self) -> Self {
        *self
    }

    /// `calculateIntercept(start, end)`: the nearest face crossing of the
    /// segment `start -> end`; Java `null` is `None`.
    ///
    /// Candidates are tested in the order min X, max X, min Y, max Y, min Z,
    /// max Z, and a later one replaces the current best only when strictly
    /// closer to `start`, so ties keep the earlier face. (Vanilla picks the
    /// side by reference identity, `var9 == var3` …; the candidates are
    /// distinct pool objects, so tracking the index is equivalent.)
    // src: AxisAlignedBB.java:240-326
    pub fn calculate_intercept(&self, start: &Vec3D, end: &Vec3D) -> Option<AabbIntercept> {
        // src: AxisAlignedBB.java:241-269 — candidate point, face check, side id.
        let candidates: [Candidate; 6] = [
            (
                start.get_intermediate_with_x_value(end, self.min_x),
                Self::is_vec_in_yz,
                4,
            ),
            (
                start.get_intermediate_with_x_value(end, self.max_x),
                Self::is_vec_in_yz,
                5,
            ),
            (
                start.get_intermediate_with_y_value(end, self.min_y),
                Self::is_vec_in_xz,
                0,
            ),
            (
                start.get_intermediate_with_y_value(end, self.max_y),
                Self::is_vec_in_xz,
                1,
            ),
            (
                start.get_intermediate_with_z_value(end, self.min_z),
                Self::is_vec_in_xy,
                2,
            ),
            (
                start.get_intermediate_with_z_value(end, self.max_z),
                Self::is_vec_in_xy,
                3,
            ),
        ];
        let mut best: Option<(Vec3D, i32)> = None;
        // src: AxisAlignedBB.java:271-294
        for (point, on_face, side) in candidates {
            let Some(point) = point.filter(|p| on_face(self, p)) else {
                continue;
            };
            let closer = match best {
                None => true,
                Some((b, _)) => start.square_distance_to(&point) < start.square_distance_to(&b),
            };
            if closer {
                best = Some((point, side));
            }
        }
        // src: AxisAlignedBB.java:324 — MovingObjectPosition copies the hit via
        // createVector (no -0.0 normalisation).
        best.map(|(hit_vec, side_hit)| AabbIntercept { side_hit, hit_vec })
    }

    /// `isVecInYZ(v)` (private in vanilla): inclusive bounds.
    // src: AxisAlignedBB.java:328-334
    fn is_vec_in_yz(&self, v: &Vec3D) -> bool {
        v.y_coord >= self.min_y
            && v.y_coord <= self.max_y
            && v.z_coord >= self.min_z
            && v.z_coord <= self.max_z
    }

    /// `isVecInXZ(v)` (private in vanilla): inclusive bounds.
    // src: AxisAlignedBB.java:336-342
    fn is_vec_in_xz(&self, v: &Vec3D) -> bool {
        v.x_coord >= self.min_x
            && v.x_coord <= self.max_x
            && v.z_coord >= self.min_z
            && v.z_coord <= self.max_z
    }

    /// `isVecInXY(v)` (private in vanilla): inclusive bounds.
    // src: AxisAlignedBB.java:344-350
    fn is_vec_in_xy(&self, v: &Vec3D) -> bool {
        v.x_coord >= self.min_x
            && v.x_coord <= self.max_x
            && v.y_coord >= self.min_y
            && v.y_coord <= self.max_y
    }

    /// `setBB(other)`: copies `other`'s bounds into this box.
    // src: AxisAlignedBB.java:352-359
    pub fn set_bb(&mut self, other: &Aabb) {
        *self = *other;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNIT: Aabb = Aabb::new(0.0, 0.0, 0.0, 1.0, 1.0, 1.0);

    #[test]
    fn add_coord_expand_contract() {
        assert_eq!(
            UNIT.add_coord(-0.5, 2.0, 0.0),
            Aabb::new(-0.5, 0.0, 0.0, 1.0, 3.0, 1.0)
        );
        assert_eq!(
            UNIT.expand(0.25, 0.5, 1.0),
            Aabb::new(-0.25, -0.5, -1.0, 1.25, 1.5, 2.0)
        );
        assert_eq!(
            UNIT.contract(0.25, 0.25, 0.25),
            Aabb::new(0.25, 0.25, 0.25, 0.75, 0.75, 0.75)
        );
        assert_eq!(UNIT.get_average_edge_length(), 1.0);
        assert_eq!(
            Aabb::new(0.0, 0.0, 0.0, 1.0, 2.0, 3.0).get_average_edge_length(),
            2.0
        );
    }

    #[test]
    fn offsets_mutate_or_copy() {
        let moved = UNIT.get_offset_bounding_box(1.0, 2.0, 3.0);
        assert_eq!(moved, Aabb::new(1.0, 2.0, 3.0, 2.0, 3.0, 4.0));
        let mut b = UNIT.copy();
        b.offset(1.0, 2.0, 3.0).offset(-1.0, 0.0, 0.0);
        assert_eq!(b, Aabb::new(0.0, 2.0, 3.0, 1.0, 3.0, 4.0));
        b.set_bb(&UNIT);
        assert_eq!(b, UNIT);
    }

    #[test]
    fn collision_offsets() {
        // Mover 0.5 to the left of the wall, overlapping in Y and Z.
        let wall = Aabb::new(2.0, 0.0, 0.0, 3.0, 1.0, 1.0);
        let mover = Aabb::new(0.5, 0.0, 0.0, 1.5, 1.0, 1.0);
        assert_eq!(wall.calculate_x_offset(&mover, 2.0), 0.5);
        assert_eq!(wall.calculate_x_offset(&mover, 0.25), 0.25);
        assert_eq!(wall.calculate_x_offset(&mover, -1.0), -1.0);
        // No overlap in Y: movement untouched.
        let above = mover.get_offset_bounding_box(0.0, 1.0, 0.0);
        assert_eq!(wall.calculate_x_offset(&above, 2.0), 2.0);
        // Falling onto a floor.
        let floor = Aabb::new(0.0, -1.0, 0.0, 1.0, 0.0, 1.0);
        let player = Aabb::new(0.2, 0.3, 0.2, 0.8, 2.1, 0.8);
        assert_eq!(floor.calculate_y_offset(&player, -1.0), -0.3);
        assert_eq!(floor.calculate_z_offset(&player, 1.0), 1.0);
    }

    #[test]
    fn intersection_is_strict() {
        assert!(UNIT.intersects_with(&UNIT.get_offset_bounding_box(0.5, 0.5, 0.5)));
        assert!(!UNIT.intersects_with(&UNIT.get_offset_bounding_box(1.0, 0.0, 0.0)));
        assert!(UNIT.is_vec_inside(&Vec3D::create_vector(0.5, 0.5, 0.5)));
        assert!(!UNIT.is_vec_inside(&Vec3D::create_vector(1.0, 0.5, 0.5)));
    }

    #[test]
    fn intercept() {
        let start = Vec3D::create_vector(-1.0, 0.5, 0.5);
        let end = Vec3D::create_vector(2.0, 0.5, 0.5);
        let hit = UNIT.calculate_intercept(&start, &end).unwrap();
        assert_eq!(hit.side_hit, 4);
        assert_eq!(hit.hit_vec, Vec3D::create_vector(0.0, 0.5, 0.5));
        // From above, straight down: max Y face.
        let hit = UNIT
            .calculate_intercept(
                &Vec3D::create_vector(0.5, 3.0, 0.5),
                &Vec3D::create_vector(0.5, -3.0, 0.5),
            )
            .unwrap();
        assert_eq!(hit.side_hit, 1);
        assert_eq!(hit.hit_vec.y_coord, 1.0);
        // Missing the box.
        assert_eq!(
            UNIT.calculate_intercept(
                &Vec3D::create_vector(-1.0, 5.0, 0.5),
                &Vec3D::create_vector(2.0, 5.0, 0.5)
            ),
            None
        );
        // Exact edge hit: min X (tested first) and min Y are equally close; the
        // earlier candidate wins.
        let hit = UNIT
            .calculate_intercept(
                &Vec3D::create_vector(-1.0, -1.0, 0.5),
                &Vec3D::create_vector(1.0, 1.0, 0.5),
            )
            .unwrap();
        assert_eq!(
            (hit.side_hit, hit.hit_vec),
            (4, Vec3D::create_vector(0.0, 0.0, 0.5))
        );
    }
}
