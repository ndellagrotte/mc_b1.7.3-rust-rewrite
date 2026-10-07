//! `ClippingHelper`, `ClippingHelperImpl`, `Frustrum` (ClippingHelper.java,
//! ClippingHelperImpl.java, Frustrum.java): the six view-frustum planes
//! from the GL projection and modelview matrices, and the box test
//! `RenderGlobal.clipRenderersByFrustrum` sets `WorldRenderer.isInFrustum`
//! with.
//!
//! In vanilla the test decides which display lists are called. Here Bevy
//! culls each section's mesh entity against the camera frustum itself (the
//! natural mapping: one entity per section and pass, with an `Aabb` from
//! its mesh); the transcribed test still sets `isInFrustum`, because
//! vanilla's rebuild priority (`RenderSorter`, `updateRenderers`) and the
//! F3 counters depend on it, and a section outside it is hidden as vanilla
//! skips it (`getGLCallListForPass`, WorldRenderer.java:195-201).
//!
//! The matrices are rebuilt from the camera state of WP-3.1
//! ([`CameraSetup`]): the projection `gluPerspective(fov, aspect, 0.05,
//! farPlaneDistance * 2)` after the anaglyph shift, the modelview
//! `G1 · P · G2` (camera.rs, "Matrices"), both as GL's column-major float
//! arrays.

use bevy::math::Mat4;

use crate::camera::CameraSetup;

/// `ClippingHelper.frustum`: six planes `(a, b, c, d)`, normalised.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ClippingHelper {
    /// `frustum[0..6]`.
    pub frustum: [[f32; 4]; 6],
}

impl ClippingHelper {
    /// `ClippingHelperImpl.init()` from the two matrices (column-major, as
    /// `glGetFloat` returns them): `clippingMatrix = projection ·
    /// modelview`, then the planes right, left, bottom, top, far, near.
    // src: ClippingHelperImpl.java:30-74
    pub fn init(projection: &[f32; 16], modelview: &[f32; 16]) -> Self {
        let p = projection;
        let m = modelview;
        let mut c = [0.0f32; 16];
        for i in 0..4 {
            for j in 0..4 {
                c[i * 4 + j] = m[i * 4] * p[j]
                    + m[i * 4 + 1] * p[4 + j]
                    + m[i * 4 + 2] * p[8 + j]
                    + m[i * 4 + 3] * p[12 + j];
            }
        }
        let plane = |k: usize, sign: f32| {
            [
                c[3] + sign * c[k],
                c[7] + sign * c[4 + k],
                c[11] + sign * c[8 + k],
                c[15] + sign * c[12 + k],
            ]
        };
        let mut f = [
            plane(0, -1.0),
            plane(0, 1.0),
            plane(1, 1.0),
            plane(1, -1.0),
            plane(2, -1.0),
            plane(2, 1.0),
        ];
        for p in &mut f {
            normalize(p);
        }
        ClippingHelper { frustum: f }
    }

    /// `isBoxInFrustum(x0, y0, z0, x1, y1, z1)`: false only when all eight
    /// corners are behind one plane.
    // src: ClippingHelper.java:9-17
    pub fn is_box_in_frustum(&self, x0: f64, y0: f64, z0: f64, x1: f64, y1: f64, z1: f64) -> bool {
        for f in &self.frustum {
            let d = |x: f64, y: f64, z: f64| {
                f[0] as f64 * x + f[1] as f64 * y + f[2] as f64 * z + f[3] as f64 <= 0.0
            };
            if d(x0, y0, z0)
                && d(x1, y0, z0)
                && d(x0, y1, z0)
                && d(x1, y1, z0)
                && d(x0, y0, z1)
                && d(x1, y0, z1)
                && d(x0, y1, z1)
                && d(x1, y1, z1)
            {
                return false;
            }
        }
        true
    }
}

/// `normalize(frustum, i)`: divides the plane by the length of its normal.
// src: ClippingHelperImpl.java:23-29
fn normalize(p: &mut [f32; 4]) {
    let var3 = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
    p[0] /= var3;
    p[1] /= var3;
    p[2] /= var3;
    p[3] /= var3;
}

/// `Frustrum`: the clipping helper and the camera position the boxes are
/// made relative to.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Frustrum {
    /// `clippingHelper`.
    pub clipping_helper: ClippingHelper,
    /// `xPosition`, `yPosition`, `zPosition`.
    pub position: [f64; 3],
}

impl Frustrum {
    /// `isBoxInFrustum(x0, y0, z0, x1, y1, z1)` in world coordinates.
    // src: Frustrum.java:15-17
    pub fn is_box_in_frustum(&self, x0: f64, y0: f64, z0: f64, x1: f64, y1: f64, z1: f64) -> bool {
        let [x, y, z] = self.position;
        self.clipping_helper
            .is_box_in_frustum(x0 - x, y0 - y, z0 - z, x1 - x, y1 - y, z1 - z)
    }

    /// `isBoundingBoxInFrustum(aabb)`.
    // src: Frustrum.java:19-21
    pub fn is_bounding_box_in_frustum(&self, b: &mc_core::Aabb) -> bool {
        self.is_box_in_frustum(b.min_x, b.min_y, b.min_z, b.max_x, b.max_y, b.max_z)
    }
}

/// `gluPerspective(fovy, aspect, zNear, zFar)` (LWJGL's GLU, float), as a
/// column-major array.
pub fn glu_perspective(fovy: f32, aspect: f32, z_near: f32, z_far: f32) -> [f32; 16] {
    let radians = fovy / 2.0 * std::f32::consts::PI / 180.0;
    let delta_z = z_far - z_near;
    let sine = radians.sin();
    let cotangent = radians.cos() / sine;
    let mut m = [0.0f32; 16];
    m[0] = cotangent / aspect;
    m[5] = cotangent;
    m[10] = -(z_far + z_near) / delta_z;
    m[11] = -1.0;
    m[14] = -2.0 * z_near * z_far / delta_z;
    m
}

/// The frustum of a camera setup at the interpolated position `pos`
/// (`new Frustrum(); setPosition(x, y, z)` in `renderWorld`).
// src: EntityRenderer.java:482-483
pub fn frustrum_of(setup: &CameraSetup, pos: [f64; 3]) -> Frustrum {
    let proj = Mat4::from_translation(bevy::math::Vec3::new(setup.projection_shift_x, 0.0, 0.0))
        * Mat4::from_cols_array(&glu_perspective(
            setup.fov_y_degrees,
            setup.aspect,
            setup.near,
            setup.far,
        ));
    Frustrum {
        clipping_helper: ClippingHelper::init(
            &proj.to_cols_array(),
            &setup.modelview().to_cols_array(),
        ),
        position: pos,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Looking down −z from the origin (identity modelview), 90° wide.
    fn helper() -> ClippingHelper {
        ClippingHelper::init(
            &glu_perspective(90.0, 1.0, 0.05, 100.0),
            &Mat4::IDENTITY.to_cols_array(),
        )
    }

    #[test]
    fn planes_of_a_simple_perspective() {
        let h = helper();
        // every plane has a unit normal
        for p in h.frustum {
            let l = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
            assert!((l - 1.0).abs() < 1e-5, "{p:?}");
        }
        // the near plane faces −z
        assert!(h.frustum[5][2] < 0.0);
    }

    #[test]
    fn boxes_in_and_out() {
        let h = helper();
        assert!(h.is_box_in_frustum(-1.0, -1.0, -10.0, 1.0, 1.0, -9.0));
        // behind the eye
        assert!(!h.is_box_in_frustum(-1.0, -1.0, 5.0, 1.0, 1.0, 6.0));
        // far off to the side
        assert!(!h.is_box_in_frustum(50.0, -1.0, -10.0, 51.0, 1.0, -9.0));
        // beyond the far plane
        assert!(!h.is_box_in_frustum(-1.0, -1.0, -200.0, 1.0, 1.0, -150.0));
        // straddling a side plane counts as in
        assert!(h.is_box_in_frustum(9.0, -1.0, -10.0, 11.0, 1.0, -9.0));
        let f = Frustrum {
            clipping_helper: h,
            position: [100.0, 64.0, 100.0],
        };
        assert!(f.is_box_in_frustum(99.0, 63.0, 90.0, 101.0, 65.0, 91.0));
        assert!(!f.is_box_in_frustum(99.0, 63.0, 105.0, 101.0, 65.0, 106.0));
    }
}
