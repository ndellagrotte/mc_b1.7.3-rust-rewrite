//! `RenderSheep` (RenderSheep.java): an unsheared sheep's wool
//! (`ModelSheep1`, `/mob/sheep_fur.png`) tinted `brightness ×
//! EntitySheep.fleeceColorTable[colour]`.

use mc_sim::entity::sheep::{FLEECE_COLOR_TABLE, get_fleece_color, get_sheared};
use mc_sim::entity::{Entity, base as ebase};

use super::Ctx;

/// The wool's `glColor3f` for fleece colour `var5` at brightness `var4`.
// src: RenderSheep.java:16
pub fn wool_color(var4: f32, var5: i32) -> [f32; 3] {
    let c = FLEECE_COLOR_TABLE[(var5 & 15) as usize];
    [var4 * c[0], var4 * c[1], var4 * c[2]]
}

/// `setWoolColorAndRender(sheep, pass, partial)`.
// src: RenderSheep.java:11-21
pub fn set_wool_color_and_render(cx: &mut Ctx, var1: &Entity, var2: i32, var3: f32) -> bool {
    if var2 == 0 && !get_sheared(var1) {
        cx.load_texture("/mob/sheep_fur.png");
        let var4 = ebase::get_entity_brightness(var1, cx.world, var3);
        let var5 = get_fleece_color(var1);
        let [r, g, b] = wool_color(var4, var5);
        cx.gl.color3f(r, g, b);
        true
    } else {
        false
    }
}
