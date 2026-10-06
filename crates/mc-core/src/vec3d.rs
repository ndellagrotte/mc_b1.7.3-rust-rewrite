//! `Vec3D` (vanilla's double-precision vector), as a value type.
//!
//! Vanilla hands out `Vec3D`s from a static pool (`createVector`,
//! `Vec3D.java:26-32`) that is rewound every tick; the pool only exists to
//! avoid allocation, and nothing in its contract survives a rewind, so here a
//! `Vec3D` is a plain `Copy` value. The one behavioural difference between
//! the two Java factories is kept: [`Vec3D::create_vector_helper`] (the
//! private constructor) turns `-0.0` into `0.0`, [`Vec3D::create_vector`]
//! (the pool's `setComponents`) does not.
//!
//! No counterpart: the pool resets `func_28215_a` / `initialize`
//! (Vec3D.java:17-24). Not transcribed: `toString` (Vec3D.java:137-139,
//! `Double.toString` formatting; debug output only). Use `Debug`.

use crate::math;

/// `Vec3D`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3D {
    /// `xCoord`.
    pub x_coord: f64,
    /// `yCoord`.
    pub y_coord: f64,
    /// `zCoord`.
    pub z_coord: f64,
}

/// `getIntermediateWith?Value` rejects segments shorter than this along the
/// axis (squared length).
// src: Vec3D.java:105
const MIN_AXIS_DELTA_SQ: f64 = 1.0000000116860974E-7;

impl Vec3D {
    /// `Vec3D.createVector(x, y, z)`: pooled; no `-0.0` normalisation
    /// (`setComponents`).
    // src: Vec3D.java:26-32, Vec3D.java:52-57
    pub fn create_vector(x: f64, y: f64, z: f64) -> Self {
        Self {
            x_coord: x,
            y_coord: y,
            z_coord: z,
        }
    }

    /// `Vec3D.createVectorHelper(x, y, z)`: a fresh vector through the private
    /// constructor, which replaces `-0.0` with `0.0` per component.
    // src: Vec3D.java:13-15, Vec3D.java:34-50
    pub fn create_vector_helper(x: f64, y: f64, z: f64) -> Self {
        // `v == -0.0D` is also true for `+0.0`; either way the result is `+0.0`.
        let fix = |v: f64| if v == -0.0 { 0.0 } else { v };
        Self::create_vector(fix(x), fix(y), fix(z))
    }

    /// `subtract(v)`: note the operand order, `v - this`.
    // src: Vec3D.java:59-61
    pub fn subtract(&self, v: &Vec3D) -> Self {
        Self::create_vector(
            v.x_coord - self.x_coord,
            v.y_coord - self.y_coord,
            v.z_coord - self.z_coord,
        )
    }

    /// `normalize()`: the length goes through `MathHelper.sqrt_double` (a
    /// `float`); lengths below `1.0E-4` give the zero vector.
    // src: Vec3D.java:63-66
    pub fn normalize(&self) -> Self {
        let len = f64::from(math::sqrt_double(
            self.x_coord * self.x_coord + self.y_coord * self.y_coord + self.z_coord * self.z_coord,
        ));
        // src: Vec3D.java:65
        if len < 1.0E-4 {
            Self::create_vector(0.0, 0.0, 0.0)
        } else {
            Self::create_vector(self.x_coord / len, self.y_coord / len, self.z_coord / len)
        }
    }

    /// `crossProduct(v)`.
    // src: Vec3D.java:68-70
    pub fn cross_product(&self, v: &Vec3D) -> Self {
        Self::create_vector(
            self.y_coord * v.z_coord - self.z_coord * v.y_coord,
            self.z_coord * v.x_coord - self.x_coord * v.z_coord,
            self.x_coord * v.y_coord - self.y_coord * v.x_coord,
        )
    }

    /// `addVector(x, y, z)`.
    // src: Vec3D.java:72-74
    pub fn add_vector(&self, x: f64, y: f64, z: f64) -> Self {
        Self::create_vector(self.x_coord + x, self.y_coord + y, self.z_coord + z)
    }

    /// `distanceTo(v)`, through `MathHelper.sqrt_double` (a `float`).
    // src: Vec3D.java:76-81
    pub fn distance_to(&self, v: &Vec3D) -> f64 {
        f64::from(math::sqrt_double(self.square_distance_to(v)))
    }

    /// `squareDistanceTo(Vec3D)`.
    // src: Vec3D.java:83-88
    pub fn square_distance_to(&self, v: &Vec3D) -> f64 {
        let dx = v.x_coord - self.x_coord;
        let dy = v.y_coord - self.y_coord;
        let dz = v.z_coord - self.z_coord;
        dx * dx + dy * dy + dz * dz
    }

    /// `squareDistanceTo(double, double, double)`.
    // src: Vec3D.java:90-95
    pub fn square_distance_to_xyz(&self, x: f64, y: f64, z: f64) -> f64 {
        let dx = x - self.x_coord;
        let dy = y - self.y_coord;
        let dz = z - self.z_coord;
        dx * dx + dy * dy + dz * dz
    }

    /// `lengthVector()`, through `MathHelper.sqrt_double` (a `float`).
    // src: Vec3D.java:97-99
    pub fn length_vector(&self) -> f64 {
        f64::from(math::sqrt_double(
            self.x_coord * self.x_coord + self.y_coord * self.y_coord + self.z_coord * self.z_coord,
        ))
    }

    /// The point on the segment `self -> v` where `axis(point) == value`, if
    /// the segment is long enough along that axis and the point lies on it.
    fn intermediate(&self, v: &Vec3D, d_axis: f64, value_minus_self: f64) -> Option<Self> {
        let dx = v.x_coord - self.x_coord;
        let dy = v.y_coord - self.y_coord;
        let dz = v.z_coord - self.z_coord;
        if d_axis * d_axis < MIN_AXIS_DELTA_SQ {
            return None;
        }
        let t = value_minus_self / d_axis;
        (0.0..=1.0).contains(&t).then(|| {
            Self::create_vector(
                self.x_coord + dx * t,
                self.y_coord + dy * t,
                self.z_coord + dz * t,
            )
        })
    }

    /// `getIntermediateWithXValue(v, x)`; Java `null` is `None`.
    // src: Vec3D.java:101-111
    pub fn get_intermediate_with_x_value(&self, v: &Vec3D, x: f64) -> Option<Self> {
        self.intermediate(v, v.x_coord - self.x_coord, x - self.x_coord)
    }

    /// `getIntermediateWithYValue(v, y)`; Java `null` is `None`.
    // src: Vec3D.java:113-123
    pub fn get_intermediate_with_y_value(&self, v: &Vec3D, y: f64) -> Option<Self> {
        self.intermediate(v, v.y_coord - self.y_coord, y - self.y_coord)
    }

    /// `getIntermediateWithZValue(v, z)`; Java `null` is `None`.
    // src: Vec3D.java:125-135
    pub fn get_intermediate_with_z_value(&self, v: &Vec3D, z: f64) -> Option<Self> {
        self.intermediate(v, v.z_coord - self.z_coord, z - self.z_coord)
    }

    /// `rotateAroundX(angle)`, in place, with the `MathHelper` table sin/cos.
    // src: Vec3D.java:141-150
    pub fn rotate_around_x(&mut self, angle: f32) {
        let c = f64::from(math::cos(angle));
        let s = f64::from(math::sin(angle));
        let y = self.y_coord * c + self.z_coord * s;
        let z = self.z_coord * c - self.y_coord * s;
        self.y_coord = y;
        self.z_coord = z;
    }

    /// `rotateAroundY(angle)`, in place, with the `MathHelper` table sin/cos.
    // src: Vec3D.java:152-161
    pub fn rotate_around_y(&mut self, angle: f32) {
        let c = f64::from(math::cos(angle));
        let s = f64::from(math::sin(angle));
        let x = self.x_coord * c + self.z_coord * s;
        let z = self.z_coord * c - self.x_coord * s;
        self.x_coord = x;
        self.z_coord = z;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_zero_only_normalised_by_helper() {
        let v = Vec3D::create_vector(-0.0, 1.0, -0.0);
        assert!(v.x_coord.is_sign_negative());
        let h = Vec3D::create_vector_helper(-0.0, 1.0, -0.0);
        assert!(h.x_coord.is_sign_positive() && h.z_coord.is_sign_positive());
    }

    #[test]
    fn intermediate_points() {
        let a = Vec3D::create_vector(0.0, 0.0, 0.0);
        let b = Vec3D::create_vector(4.0, 2.0, 0.0);
        assert_eq!(
            a.get_intermediate_with_x_value(&b, 2.0),
            Some(Vec3D::create_vector(2.0, 1.0, 0.0))
        );
        assert_eq!(a.get_intermediate_with_x_value(&b, 5.0), None);
        // No movement along z: rejected by the minimum-delta check.
        assert_eq!(a.get_intermediate_with_z_value(&b, 0.0), None);
        assert_eq!(a.subtract(&b), Vec3D::create_vector(4.0, 2.0, 0.0));
        assert_eq!(a.square_distance_to(&b), 20.0);
    }
}
