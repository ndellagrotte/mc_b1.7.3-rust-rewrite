//! `Path` (Path.java): the open set of the search, a binary min-heap on
//! `distanceToTarget` that keeps each point's slot in `PathPoint.index`.
//! Java's own heap, transcribed operation by operation (ties and sift
//! order decide which of two equal-cost points is expanded first).

use super::path_point::PathPoint;

/// `Path`. `path_points` holds arena indices (module docs of [`super`]);
/// Java's array starts at 1024 slots and doubles when full
/// (Path.java:11-15), which only affects capacity.
#[derive(Clone, Debug, Default)]
pub struct Path {
    /// `pathPoints[0..count]`.
    path_points: Vec<usize>,
    /// `count`.
    count: usize,
}

impl Path {
    /// `new Path()`.
    pub fn new() -> Path {
        Path {
            path_points: Vec::with_capacity(1024),
            count: 0,
        }
    }

    /// `addPoint(point)`: appends and sifts toward the root. Java throws
    /// `IllegalStateException("OW KNOWS!")` for a point already in a heap;
    /// the search never does that (it tests `isAssigned` first).
    // src: Path.java:7-22
    pub fn add_point(&mut self, pts: &mut [PathPoint], p: usize) -> usize {
        assert!(pts[p].index < 0, "OW KNOWS!");
        if self.path_points.len() == self.count {
            self.path_points.push(p);
        } else {
            self.path_points[self.count] = p;
        }
        pts[p].index = self.count as i32;
        let c = self.count;
        self.count += 1;
        self.sort_back(pts, c);
        p
    }

    /// `clearPath()`: empties the heap (the points keep their `index`).
    // src: Path.java:24-26
    pub fn clear_path(&mut self) {
        self.count = 0;
    }

    /// `dequeue()`: removes the root, moves the last point to the root and
    /// sifts it down; the removed point's `index` becomes -1.
    // src: Path.java:28-38
    pub fn dequeue(&mut self, pts: &mut [PathPoint]) -> usize {
        let var1 = self.path_points[0];
        self.count -= 1;
        self.path_points[0] = self.path_points[self.count];
        if self.count > 0 {
            self.sort_forward(pts, 0);
        }
        pts[var1].index = -1;
        var1
    }

    /// `changeDistance(point, distance)`: sets the key and sifts up if it
    /// decreased, else down.
    // src: Path.java:40-49
    pub fn change_distance(&mut self, pts: &mut [PathPoint], p: usize, var2: f32) {
        let var3 = pts[p].distance_to_target;
        pts[p].distance_to_target = var2;
        let i = pts[p].index as usize;
        if var2 < var3 {
            self.sort_back(pts, i);
        } else {
            self.sort_forward(pts, i);
        }
    }

    /// `sortBack(i)`: sift up while strictly smaller than the parent.
    // src: Path.java:51-68
    fn sort_back(&mut self, pts: &mut [PathPoint], mut var1: usize) {
        let var2 = self.path_points[var1];
        let var3 = pts[var2].distance_to_target;
        while var1 > 0 {
            let var4 = (var1 - 1) >> 1;
            let var5 = self.path_points[var4];
            if var3 >= pts[var5].distance_to_target {
                break;
            }
            self.path_points[var1] = var5;
            pts[var5].index = var1 as i32;
            var1 = var4;
        }
        self.path_points[var1] = var2;
        pts[var2].index = var1 as i32;
    }

    /// `sortForward(i)`: sift down toward the smaller child (the right one on
    /// a tie), while that child is strictly smaller.
    // src: Path.java:70-114
    fn sort_forward(&mut self, pts: &mut [PathPoint], mut var1: usize) {
        let var2 = self.path_points[var1];
        let var3 = pts[var2].distance_to_target;
        loop {
            let var4 = 1 + (var1 << 1);
            let var5 = var4 + 1;
            if var4 >= self.count {
                break;
            }
            let var6 = self.path_points[var4];
            let var7 = pts[var6].distance_to_target;
            let (var8, var9) = if var5 >= self.count {
                (None, f32::INFINITY)
            } else {
                let p = self.path_points[var5];
                (Some(p), pts[p].distance_to_target)
            };
            if var7 < var9 {
                if var7 >= var3 {
                    break;
                }
                self.path_points[var1] = var6;
                pts[var6].index = var1 as i32;
                var1 = var4;
            } else {
                if var9 >= var3 {
                    break;
                }
                // `var9 < var3` is finite here, so the right child exists.
                let var8 = var8.expect("sortForward: right child");
                self.path_points[var1] = var8;
                pts[var8].index = var1 as i32;
                var1 = var5;
            }
        }
        self.path_points[var1] = var2;
        pts[var2].index = var1 as i32;
    }

    /// `isPathEmpty()`.
    // src: Path.java:116-118
    pub fn is_path_empty(&self) -> bool {
        self.count == 0
    }
}
