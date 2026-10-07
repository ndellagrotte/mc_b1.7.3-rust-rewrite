//! `TileEntitySignRenderer` (TileEntitySignRenderer.java): `SignModel` with
//! `/item/sign.png` at 2/3 scale — a standing sign turned by its metadata ×
//! 22.5°, a wall sign turned to its wall (2: 180°, 4: 90°, 5: −90°, else
//! 0°) without its stick and pushed against the wall — and its four lines
//! in black, `0.016666668 × 2/3` per pixel, centred, ten pixels apart, with
//! `"> line <"` around the line being edited; depth writes off for the
//! text.

use mc_sim::tile_entity::sign::TileEntitySign;
use mc_sim::world::tile_entity::TileEntity;

use super::super::Ctx;
use super::super::model_sign::SignModel;
use super::super::text3d;

/// `Block.signPost`'s id (the standing sign; the wall sign is 68).
// src: Block.java:655
pub const SIGN_POST: i32 = 63;

/// `TileEntitySignRenderer`'s fields.
#[derive(Clone, Debug, Default)]
pub struct TileEntitySignRenderer {
    /// `signModel`.
    pub sign_model: SignModel,
}

/// The yaw a sign is drawn with (`glRotatef(-angle, 0, 1, 0)`'s angle):
/// a standing sign's `metadata × 360 / 16`, a wall sign's by its facing.
// src: TileEntitySignRenderer.java:13-35
pub fn sign_rotation(standing: bool, meta: i32) -> f32 {
    if standing {
        (meta * 360) as f32 / 16.0
    } else {
        let mut var12 = 0.0f32;
        if meta == 2 {
            var12 = 180.0;
        }
        if meta == 4 {
            var12 = 90.0;
        }
        if meta == 5 {
            var12 = -90.0;
        }
        var12
    }
}

impl TileEntitySignRenderer {
    /// `renderTileEntitySignAt(sign, x, y, z, partial)`.
    // src: TileEntitySignRenderer.java:8-65
    #[allow(clippy::too_many_arguments)]
    pub fn render_tile_entity_sign_at(
        &mut self,
        cx: &mut Ctx,
        te: &TileEntity,
        var1: &TileEntitySign,
        var2: f64,
        var4: f64,
        var6: f64,
    ) {
        let (x, y, z) = (te.x_coord, te.y_coord, te.z_coord);
        let var9 = cx.world.get_block_id(x, y, z);
        let meta = cx.world.get_block_metadata(x, y, z);
        cx.gl.push_matrix();
        let var10 = 0.6666667f32;
        if var9 == SIGN_POST {
            cx.gl.translatef(
                var2 as f32 + 0.5,
                var4 as f32 + 0.75 * var10,
                var6 as f32 + 0.5,
            );
            let var11 = sign_rotation(true, meta);
            cx.gl.rotatef(-var11, 0.0, 1.0, 0.0);
            self.sign_model.sign_stick.show_model = true;
        } else {
            let var12 = sign_rotation(false, meta);
            cx.gl.translatef(
                var2 as f32 + 0.5,
                var4 as f32 + 0.75 * var10,
                var6 as f32 + 0.5,
            );
            cx.gl.rotatef(-var12, 0.0, 1.0, 0.0);
            cx.gl.translatef(0.0, -0.3125, -0.4375);
            self.sign_model.sign_stick.show_model = false;
        }
        cx.gl.bind_texture("/item/sign.png");
        cx.gl.push_matrix();
        cx.gl.scalef(var10, -var10, -var10);
        self.sign_model.func_887_a(cx.gl);
        cx.gl.pop_matrix();
        let var17 = cx.font;
        let var12 = 0.016666668 * var10;
        let g = &mut *cx.gl;
        g.translatef(0.0, 0.5 * var10, 0.07 * var10);
        g.scalef(var12, -var12, var12);
        #[allow(clippy::neg_multiply)] // the Java's `-1.0F * var12`
        g.normal3f(0.0, 0.0, -1.0 * var12);
        g.depth_mask(false);
        let var13 = 0;
        let n = var1.sign_text.len() as i32;
        for (var14, line) in var1.sign_text.iter().enumerate() {
            let var14 = var14 as i32;
            let var15 = if var14 == var1.line_being_edited {
                format!("> {line} <")
            } else {
                line.clone()
            };
            text3d::draw_string(
                g,
                var17,
                &var15,
                -var17.get_string_width(&var15) / 2,
                var14 * 10 - n * 5,
                var13,
            );
        }
        g.depth_mask(true);
        g.color4f(1.0, 1.0, 1.0, 1.0);
        g.pop_matrix();
    }
}
