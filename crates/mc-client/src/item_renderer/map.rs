//! `MapItemRenderer` (MapItemRenderer.java) and the map branch's
//! background (ItemRenderer.java:206-215): what a held map draws in first
//! person, in map pixels (the matrix of
//! [`super::first_person::FirstPersonDraw::Map`]).
//!
//! In order, all with the item lighting still on and the normal
//! `glNormal3f(0, 0, -1)` (no Tessellator normal is set, so GL's current
//! normal applies): `/misc/mapbg.png` over `-7..135`; the map's 128×128
//! pixels at z −0.01, blended (alpha test off; unexplored pixels are a
//! faint checkerboard of alpha 16 and 24); each `playersVisibleOnMap`
//! marker from `/misc/mapicons.png` at z −0.02; the map's name in black at
//! z −0.04 (`FontRenderer.drawString`).

use bevy::math::{Mat4, Vec3};
use mc_sim::block::MapColor;
use mc_sim::world::map_storage::{MAP_COLORS_LEN, MapCoord};

use crate::camera::GlMatrix;
use crate::gl_material::TessBatch;

/// `/misc/mapbg.png`.
// src: ItemRenderer.java:206
pub const MAPBG_PNG: &str = "/misc/mapbg.png";
/// `/misc/mapicons.png`.
// src: MapItemRenderer.java:73
pub const MAPICONS_PNG: &str = "/misc/mapicons.png";

/// `renderMap`'s pixel loop: `MapData.colors` → `intArray` (ARGB), then
/// `func_28150_a`'s upload ([`upload_pixels`]) as RGBA bytes for an
/// `Rgba8Unorm` image. With the anaglyph option on, an explored pixel's
/// colour goes through the anaglyph transform twice, once here and once
/// in the upload.
// src: MapItemRenderer.java:25-57
pub fn map_pixels(colors: &[u8], anaglyph: bool) -> Vec<u8> {
    upload_pixels(&map_int_array(colors, anaglyph), anaglyph)
}

/// `renderMap`'s loop over `MapData.colors`, filling `intArray` (ARGB).
/// A colour index without a `MapColor` (14, 15) or a negative byte would
/// throw in Java (`NullPointerException`, `ArrayIndexOutOfBoundsException`);
/// here such a pixel is 0 (transparent).
// src: MapItemRenderer.java:25-55
pub fn map_int_array(colors: &[u8], anaglyph: bool) -> Vec<i32> {
    let mut out = vec![0i32; MAP_COLORS_LEN];
    for var4 in 0..MAP_COLORS_LEN.min(colors.len()) {
        let var5 = colors[var4] as i8 as i32;
        out[var4] = if var5 / 4 == 0 {
            // (var4 + var4 / 128 & 1) * 8 + 16 << 24
            let v = var4 as i32;
            (((v + v / 128) & 1) * 8 + 16) << 24
        } else {
            let Some(c) = usize::try_from(var5 / 4)
                .ok()
                .and_then(|i| MapColor::MAP_COLOR_ARRAY.get(i).copied().flatten())
            else {
                continue;
            };
            let var6 = c.color_value;
            let var7 = var5 & 3;
            let mut var8: i32 = 220;
            if var7 == 2 {
                var8 = 255;
            }
            if var7 == 0 {
                var8 = 180;
            }
            let mut var9 = (var6 >> 16 & 255) * var8 / 255;
            let mut var10 = (var6 >> 8 & 255) * var8 / 255;
            let mut var11 = (var6 & 255) * var8 / 255;
            if anaglyph {
                // src: MapItemRenderer.java:44-51
                (var9, var10, var11) = crate::textures::texture_fx::anaglyph(var9, var10, var11);
            }
            -16777216i32 | var9 << 16 | var10 << 8 | var11
        };
    }
    out
}

/// `RenderEngine.func_28150_a(intArray, 128, 128, bufferedImage)`: ARGB →
/// RGBA bytes, applying the anaglyph transform again when the option is on
/// (`options` is the game's settings, RenderEngine.java:36). The arithmetic
/// is `setupTexture`'s, so this is [`crate::textures::argb_to_rgba`]; the
/// hand pass uploads through [`crate::textures::Textures::upload_argb`],
/// which converts the same way with the texture cache's anaglyph flag.
// vanilla bug: with anaglyph on, renderMap has already transformed the
// colours (MapItemRenderer.java:44-51), so the map is transformed twice.
// src: RenderEngine.java:246-293, MapItemRenderer.java:57
pub fn upload_pixels(int_array: &[i32], anaglyph: bool) -> Vec<u8> {
    crate::textures::argb_to_rgba(int_array, anaglyph)
}

/// The background quad (ItemRenderer.java:208-215).
// src: ItemRenderer.java:208-215
pub fn background_quad() -> TessBatch {
    let var20 = 7.0f32;
    let mut b = TessBatch::default();
    let c = [1.0; 4];
    b.vertex([0.0 - var20, 128.0 + var20, 0.0], [0.0, 1.0], c);
    b.vertex([128.0 + var20, 128.0 + var20, 0.0], [1.0, 1.0], c);
    b.vertex([128.0 + var20, 0.0 - var20, 0.0], [1.0, 0.0], c);
    b.vertex([0.0 - var20, 0.0 - var20, 0.0], [0.0, 0.0], c);
    b
}

/// The map image quad (MapItemRenderer.java:65-70).
// src: MapItemRenderer.java:58-70
pub fn image_quad() -> TessBatch {
    let (var15, var16, var18) = (0.0f32, 0.0f32, 0.0f32);
    let z = -0.009999999776482582f64 as f32;
    let mut b = TessBatch::default();
    let c = [1.0; 4];
    b.vertex(
        [var15 + 0.0 + var18, var16 + 128.0 - var18, z],
        [0.0, 1.0],
        c,
    );
    b.vertex(
        [var15 + 128.0 - var18, var16 + 128.0 - var18, z],
        [1.0, 1.0],
        c,
    );
    b.vertex(
        [var15 + 128.0 - var18, var16 + 0.0 + var18, z],
        [1.0, 0.0],
        c,
    );
    b.vertex([var15 + 0.0 + var18, var16 + 0.0 + var18, z], [0.0, 0.0], c);
    b
}

/// The markers (MapItemRenderer.java:74-94), each under its own matrix,
/// in map pixels.
// src: MapItemRenderer.java:74-94
#[allow(clippy::identity_op)] // `iconSize % 4 + 0`, as the Java
pub fn icon_quads(coords: &[MapCoord]) -> TessBatch {
    let (var15, var16) = (0.0f32, 0.0f32);
    let mut b = TessBatch::default();
    let c = [1.0; 4];
    for var20 in coords {
        let mut m = GlMatrix::default();
        m.translate(
            var15 + var20.center_x as f32 / 2.0 + 64.0,
            var16 + var20.center_z as f32 / 2.0 + 64.0,
            -0.02,
        );
        m.rotate(
            (var20.icon_rotation as i32 * 360) as f32 / 16.0,
            0.0,
            0.0,
            1.0,
        );
        m.scale(4.0, 4.0, 3.0);
        m.translate(-0.125, 0.125, 0.0);
        let size = var20.icon_size as i32;
        let var21 = (size % 4 + 0) as f32 / 4.0;
        let var22 = (size / 4 + 0) as f32 / 4.0;
        let var23 = (size % 4 + 1) as f32 / 4.0;
        let var24 = (size / 4 + 1) as f32 / 4.0;
        let p = |x: f32, y: f32| m.0.transform_point3(Vec3::new(x, y, 0.0)).to_array();
        b.vertex(p(-1.0, 1.0), [var21, var22], c);
        b.vertex(p(1.0, 1.0), [var23, var22], c);
        b.vertex(p(1.0, -1.0), [var23, var24], c);
        b.vertex(p(-1.0, -1.0), [var21, var24], c);
    }
    b
}

/// The name's matrix: `glTranslatef(0, 0, -0.04)`, `glScalef(1, 1, 1)`.
// src: MapItemRenderer.java:96-99
pub fn text_matrix() -> Mat4 {
    let mut m = GlMatrix::default();
    m.translate(0.0, 0.0, -0.04);
    m.scale(1.0, 1.0, 1.0);
    m.0
}
