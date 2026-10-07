//! `RenderItem` (RenderItem.java), the `EntityItem` half: the dropped stack
//! bobbing (`sin((age + partial) / 10 + hoverStart) × 0.1 + 0.1`) and
//! turning (`((age + partial) / 20 + hoverStart)` radians); 1, 2, 3 or 4
//! copies for stacks of 1, 2-5, 6-20 and more, each extra copy moved by
//! three draws of a `Random` seeded 187 per item; a block drawn in 3D
//! (`renderBlockOnInventory`, scaled 0.25, or 0.5 for the shapes that are
//! not full cubes), anything else as a camera-facing quad of its
//! `items.png` / `terrain.png` icon tinted by `getColorFromDamage ×
//! brightness`. `renderItemIntoGUI` and the overlay are WP-3.6b's.

use mc_core::JavaRandom;
use mc_core::math::sin;
use mc_sim::block;
use mc_sim::entity::{Entity, EntityKindData as K};
use mc_sim::item;

use super::Ctx;
use super::base::RenderBase;
use crate::gui::batch::gl;
use crate::terrain::{blocks, render_blocks};
use crate::textures::{ITEMS_PNG, TERRAIN_PNG};

/// `Block.stairSingle`'s id.
// src: Block.java:636
const STAIR_SINGLE: i32 = 44;

/// `RenderItem`'s fields.
#[derive(Clone, Debug)]
pub struct RenderItem {
    /// `shadowSize` 0.15, `field_194_c` 0.75.
    pub base: RenderBase,
    /// `random` (seeded 187 before each item).
    pub random: JavaRandom,
    /// `field_27004_a`: tint by the item's colour (true).
    pub field_27004_a: bool,
}

impl Default for RenderItem {
    // src: RenderItem.java:7-14
    fn default() -> Self {
        RenderItem {
            base: RenderBase {
                shadow_size: 0.15,
                field_194_c: 0.75,
            },
            random: JavaRandom::new(0),
            field_27004_a: true,
        }
    }
}

/// The number of copies drawn for a stack of `n`.
// src: RenderItem.java:22-33
pub fn copies(n: i32) -> i32 {
    let mut var13 = 1;
    if n > 1 {
        var13 = 2;
    }
    if n > 5 {
        var13 = 3;
    }
    if n > 20 {
        var13 = 4;
    }
    var13
}

/// The bob height at `age + partial` and `hoverStart`.
// src: RenderItem.java:20
pub fn bob(age: i32, partial: f32, hover_start: f32) -> f32 {
    sin((age as f32 + partial) / 10.0 + hover_start) * 0.1 + 0.1
}

/// The turn (degrees).
// src: RenderItem.java:21
pub fn spin(age: i32, partial: f32, hover_start: f32) -> f32 {
    ((age as f32 + partial) / 20.0 + hover_start) * 57.295776
}

/// The icon's texture rectangle `(u0, u1, v0, v1)`.
// src: RenderItem.java:72-75
pub fn icon_uv(var14: i32) -> (f32, f32, f32, f32) {
    (
        (var14 % 16 * 16) as f32 / 256.0,
        (var14 % 16 * 16 + 16) as f32 / 256.0,
        (var14 / 16 * 16) as f32 / 256.0,
        (var14 / 16 * 16 + 16) as f32 / 256.0,
    )
}

impl RenderItem {
    /// `doRenderItem(item, x, y, z, yaw, partial)`.
    // src: RenderItem.java:16-115
    pub fn do_render_item(
        &mut self,
        cx: &mut Ctx,
        var1: &Entity,
        var2: f64,
        var4: f64,
        var6: f64,
        var9: f32,
    ) {
        let K::Item(d) = &var1.kind else {
            return;
        };
        let Some(var10) = d.item else {
            return;
        };
        self.random.set_seed(187);
        cx.gl.push_matrix();
        let var11 = bob(d.age, var9, d.hover_start);
        let var12 = spin(d.age, var9, d.hover_start);
        let var13 = copies(var10.stack_size);
        cx.gl
            .translatef(var2 as f32, var4 as f32 + var11, var6 as f32);
        cx.gl.enable(gl::RESCALE_NORMAL);
        if var10.item_id < 256
            && render_blocks::render_item_in_3d(blocks::get_render_type(var10.item_id))
        {
            cx.gl.rotatef(var12, 0.0, 1.0, 0.0);
            cx.load_texture(TERRAIN_PNG);
            let mut var28 = 0.25f32;
            let normal =
                block::block(var10.item_id).is_some_and(|b| block::render_as_normal_block(b.kind));
            if !normal
                && var10.item_id != STAIR_SINGLE
                && blocks::get_render_type(var10.item_id) != 16
            {
                var28 = 0.5;
            }
            cx.gl.scalef(var28, var28, var28);
            for var29 in 0..var13 {
                cx.gl.push_matrix();
                if var29 > 0 {
                    let var16 = (self.random.next_float() * 2.0 - 1.0) * 0.2 / var28;
                    let var17 = (self.random.next_float() * 2.0 - 1.0) * 0.2 / var28;
                    let var18 = (self.random.next_float() * 2.0 - 1.0) * 0.2 / var28;
                    cx.gl.translatef(var16, var17, var18);
                }
                let br = cx.brightness(var1, var9);
                cx.render_block_on_inventory(var10.item_id, var10.get_item_damage(), br);
                cx.gl.pop_matrix();
            }
        } else {
            cx.gl.scalef(0.5, 0.5, 0.5);
            let var14 = var10.get_icon_index();
            if var10.item_id < 256 {
                cx.load_texture(TERRAIN_PNG);
            } else {
                cx.load_texture(ITEMS_PNG);
            }
            let (var16, var17, var18, var19) = icon_uv(var14);
            let var20 = 1.0f32;
            let var21 = 0.5f32;
            let var22 = 0.25f32;
            if self.field_27004_a {
                let var23 = item::item(var10.item_id).map_or(16777215, |i| {
                    i.get_color_from_damage(var10.get_item_damage())
                });
                let var24 = (var23 >> 16 & 255) as f32 / 255.0;
                let var25 = (var23 >> 8 & 255) as f32 / 255.0;
                let var26 = (var23 & 255) as f32 / 255.0;
                let var27 = cx.brightness(var1, var9);
                cx.gl
                    .color4f(var24 * var27, var25 * var27, var26 * var27, 1.0);
            }
            let view_y = cx.info.player_view_y;
            for var23 in 0..var13 {
                cx.gl.push_matrix();
                if var23 > 0 {
                    let var24 = (self.random.next_float() * 2.0 - 1.0) * 0.3;
                    let var25 = (self.random.next_float() * 2.0 - 1.0) * 0.3;
                    let var26 = (self.random.next_float() * 2.0 - 1.0) * 0.3;
                    cx.gl.translatef(var24, var25, var26);
                }
                let g = &mut *cx.gl;
                g.rotatef(180.0 - view_y, 0.0, 1.0, 0.0);
                g.start_drawing_quads();
                g.set_normal(0.0, 1.0, 0.0);
                g.add_vertex_with_uv(
                    (0.0 - var21) as f64,
                    (0.0 - var22) as f64,
                    0.0,
                    var16 as f64,
                    var19 as f64,
                );
                g.add_vertex_with_uv(
                    (var20 - var21) as f64,
                    (0.0 - var22) as f64,
                    0.0,
                    var17 as f64,
                    var19 as f64,
                );
                g.add_vertex_with_uv(
                    (var20 - var21) as f64,
                    (1.0 - var22) as f64,
                    0.0,
                    var17 as f64,
                    var18 as f64,
                );
                g.add_vertex_with_uv(
                    (0.0 - var21) as f64,
                    (1.0 - var22) as f64,
                    0.0,
                    var16 as f64,
                    var18 as f64,
                );
                g.draw();
                g.pop_matrix();
            }
        }
        cx.gl.disable(gl::RESCALE_NORMAL);
        cx.gl.pop_matrix();
    }
}
