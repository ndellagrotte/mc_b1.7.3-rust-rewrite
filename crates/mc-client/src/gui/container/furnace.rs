//! `GuiFurnace` (GuiFurnace.java): `/gui/furnace.png` with the flame
//! (`getBurnTimeRemainingScaled(12)`) and the arrow
//! (`getCookProgressScaled(24)`) read from the furnace's tile entity.

use mc_sim::world::tile_entity::{TileEntityKindData, TileEntityRef};

use super::LABEL_COLOR;
use crate::gui::batch::GuiGl;
use crate::gui::draw;
use crate::gui::font::FontRenderer;
use crate::gui::screen::ScreenCtx;

/// `/gui/furnace.png`.
// src: GuiFurnace.java:20
pub const FURNACE_PNG: &str = "/gui/furnace.png";

/// `GuiFurnace`'s field.
#[derive(Clone, Debug)]
pub struct FurnaceState {
    /// `furnaceInventory`.
    // src: GuiFurnace.java:6
    pub furnace_inventory: TileEntityRef,
}

/// The flame's and the arrow's `drawTexturedModalRect` arguments relative
/// to the window's corner, `(x, y, u, v, w, h)`: the flame (when burning)
/// grows up from the bottom of its 14×13 box, `scaled + 2` rows tall; the
/// arrow is `scaled + 1` columns wide.
// src: GuiFurnace.java:24-31
pub fn furnace_bars(burning: Option<i32>, cook_scaled: i32) -> Vec<(i32, i32, i32, i32, i32, i32)> {
    let mut out = Vec::new();
    if let Some(var5) = burning {
        out.push((56, 36 + 12 - var5, 176, 12 - var5, 14, var5 + 2));
    }
    let var5 = cook_scaled;
    out.push((79, 34, 176, 14, var5 + 1, 16));
    out
}

/// `drawGuiContainerForegroundLayer()`.
// src: GuiFurnace.java:11-14
pub fn draw_foreground(g: &mut GuiGl, f: &FontRenderer, y_size: i32) {
    f.draw_string(g, "Furnace", 60, 6, LABEL_COLOR);
    f.draw_string(g, "Inventory", 8, y_size - 96 + 2, LABEL_COLOR);
}

impl FurnaceState {
    /// `drawGuiContainerBackgroundLayer(partial)`.
    // src: GuiFurnace.java:16-32
    #[allow(clippy::too_many_arguments)]
    pub fn draw_background(
        &self,
        ctx: &mut ScreenCtx,
        g: &mut GuiGl,
        w: i32,
        h: i32,
        x_size: i32,
        y_size: i32,
    ) {
        g.color4f(1.0, 1.0, 1.0, 1.0);
        g.bind_texture(FURNACE_PNG);
        let var3 = (w - x_size) / 2;
        let var4 = (h - y_size) / 2;
        draw::draw_textured_modal_rect(g, 0.0, var3, var4, 0, 0, x_size, y_size);
        let furnace = ctx
            .game
            .the_world
            .as_mut()
            .and_then(|wd| wd.tile_entities.get_mut(&self.furnace_inventory))
            .and_then(|t| match &mut t.kind {
                TileEntityKindData::Furnace(f) => Some(f),
                _ => None,
            });
        let Some(f) = furnace else {
            return;
        };
        // getBurnTimeRemainingScaled sets currentItemBurnTime to 200 when it
        // is 0 (a getter with a side effect, vanilla; it runs only while
        // burning).
        let burning = f.is_burning().then(|| f.get_burn_time_remaining_scaled(12));
        let cook = f.get_cook_progress_scaled(24);
        for (x, y, u, v, bw, bh) in furnace_bars(burning, cook) {
            draw::draw_textured_modal_rect(g, 0.0, var3 + x, var4 + y, u, v, bw, bh);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn furnace_arrow_scaling() {
        // Half cooked (100 of 200 ticks): 24 * 100 / 200 = 12, drawn 13
        // wide.
        let mut f = mc_sim::tile_entity::furnace::TileEntityFurnace {
            furnace_cook_time: 100,
            ..Default::default()
        };
        assert_eq!(f.get_cook_progress_scaled(24), 12);
        let b = furnace_bars(None, 12);
        assert_eq!(b, vec![(79, 34, 176, 14, 13, 16)]);
        // A coal (1600) with 800 left: 12 * 800 / 1600 = 6: the flame's top
        // 6 rows down, 8 rows drawn.
        f.furnace_burn_time = 800;
        f.current_item_burn_time = 1600;
        assert_eq!(f.get_burn_time_remaining_scaled(12), 6);
        let b = furnace_bars(Some(6), 0);
        assert_eq!(b[0], (56, 42, 176, 6, 14, 8));
        assert_eq!(b[1], (79, 34, 176, 14, 1, 16));
        // The getter's side effect: a burn time of 0 becomes 200.
        f.current_item_burn_time = 0;
        f.furnace_burn_time = 100;
        assert_eq!(f.get_burn_time_remaining_scaled(12), 6);
        assert_eq!(f.current_item_burn_time, 200);
    }
}
