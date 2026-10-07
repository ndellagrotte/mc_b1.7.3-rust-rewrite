//! `ScaledResolution` (ScaledResolution.java) — the type itself is the
//! shell's [`crate::app::ScaledResolution`] (WP-3.1), re-exported here —
//! and the two pieces of arithmetic the GUI does with it: the mouse in
//! scaled pixels and the GUI's orthographic projection.

pub use crate::app::ScaledResolution;

/// The mouse in scaled GUI pixels from LWJGL's pointer (origin bottom
/// left, physical pixels): `x * width / displayWidth`,
/// `height - y * height / displayHeight - 1` (integer arithmetic). The
/// same formula serves `GuiScreen.handleMouseInput` (with the screen's
/// `width`/`height`) and `updateCameraAndRender` (with a new
/// `ScaledResolution`'s).
// src: GuiScreen.java:105-109; EntityRenderer.java:362-363
pub fn mouse_to_gui(
    mouse_x: i32,
    mouse_y: i32,
    width: i32,
    height: i32,
    display_width: i32,
    display_height: i32,
) -> (i32, i32) {
    let dw = display_width.max(1);
    let dh = display_height.max(1);
    (
        mouse_x.wrapping_mul(width) / dw,
        height - mouse_y.wrapping_mul(height) / dh - 1,
    )
}

/// The inverse for scripts: an LWJGL pointer position that
/// [`mouse_to_gui`] maps to scaled pixel `(x, y)` (the middle of the range
/// of pointer positions that map there).
pub fn gui_to_mouse(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    display_width: i32,
    display_height: i32,
) -> (i32, i32) {
    // The pointers p with p * n / d == k: ceil(k·d/n) ..= ceil((k+1)·d/n) - 1.
    let mid = |k: i32, n: i32, d: i32| {
        let (k, n, d) = (k as i64, n.max(1) as i64, d.max(1) as i64);
        let lo = (k * d + n - 1).div_euclid(n);
        let hi = ((k + 1) * d + n - 1).div_euclid(n) - 1;
        ((lo + hi.max(lo)) / 2) as i32
    };
    (
        mid(x, width, display_width),
        mid(height - 1 - y, height, display_height),
    )
}

/// `setupOverlayRendering`'s projection: `glOrtho(0, scaledWidthD,
/// scaledHeightD, 0, 1000, 3000)` after `glTranslatef(0, 0, -2000)`: GUI
/// pixels with y down, z between -1000 and 1000. The camera that draws the
/// GUI ([`crate::gui::render`]) sits at z 2000 with near 1000 and far 3000,
/// and the GUI meshes are built with y negated; this returns the area
/// `(width, height)` it covers.
// src: EntityRenderer.java:777-786
pub fn ortho_size(r: &ScaledResolution) -> (f32, f32) {
    (r.scaled_width_d as f32, r.scaled_height_d as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_factors_for_window_sizes_and_gui_scales() {
        // (gui scale, w, h) → (factor, scaled w, scaled h)
        let cases = [
            ((0, 854, 480), (2, 427, 240)),
            ((1, 854, 480), (1, 854, 480)),
            ((2, 854, 480), (2, 427, 240)),
            ((3, 854, 480), (2, 427, 240)),
            ((0, 1920, 1080), (4, 480, 270)),
            ((1, 1920, 1080), (1, 1920, 1080)),
            ((2, 1920, 1080), (2, 960, 540)),
            ((3, 1920, 1080), (3, 640, 360)),
            ((0, 640, 480), (2, 320, 240)),
            ((0, 639, 480), (1, 639, 480)),
            ((0, 320, 240), (1, 320, 240)),
            ((0, 100, 50), (1, 100, 50)),
            ((0, 2560, 1440), (6, 427, 240)),
            ((3, 1001, 721), (3, 334, 241)),
        ];
        for ((gs, w, h), (f, sw, sh)) in cases {
            let r = ScaledResolution::new(gs, w, h);
            assert_eq!(
                (r.scale_factor, r.scaled_width, r.scaled_height),
                (f, sw, sh),
                "guiScale {gs}, {w}x{h}"
            );
            assert_eq!(r.scaled_width_d, w as f64 / f as f64);
        }
    }

    #[test]
    fn mouse_round_trip() {
        // LWJGL (0, 0) is the bottom-left pixel: scaled (0, 239).
        assert_eq!(mouse_to_gui(0, 0, 427, 240, 854, 480), (0, 239));
        assert_eq!(mouse_to_gui(853, 479, 427, 240, 854, 480), (426, 0));
        for (x, y) in [(0, 0), (213, 100), (426, 239)] {
            let (mx, my) = gui_to_mouse(x, y, 427, 240, 854, 480);
            assert_eq!(mouse_to_gui(mx, my, 427, 240, 854, 480), (x, y));
        }
        let r = ScaledResolution::new(3, 1001, 721);
        let (w, h) = (r.scaled_width, r.scaled_height);
        for (x, y) in [(0, 0), (100, 200), (333, 240)] {
            let (mx, my) = gui_to_mouse(x, y, w, h, 1001, 721);
            assert_eq!(mouse_to_gui(mx, my, w, h, 1001, 721), (x, y));
        }
    }
}
