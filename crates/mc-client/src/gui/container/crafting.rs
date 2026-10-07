//! `GuiCrafting` (GuiCrafting.java): the workbench's 3×3 grid,
//! `/gui/crafting.png`. Its `onGuiClosed` is [`super::GuiContainer`]'s
//! (the second `onCraftGuiClosed` call is discussed there).

use super::LABEL_COLOR;
use crate::gui::batch::GuiGl;
use crate::gui::draw;
use crate::gui::font::FontRenderer;

/// `/gui/crafting.png`.
// src: GuiCrafting.java:19
pub const CRAFTING_PNG: &str = "/gui/crafting.png";

/// `drawGuiContainerForegroundLayer()`.
// src: GuiCrafting.java:13-16
pub fn draw_foreground(g: &mut GuiGl, f: &FontRenderer, y_size: i32) {
    f.draw_string(g, "Crafting", 28, 6, LABEL_COLOR);
    f.draw_string(g, "Inventory", 8, y_size - 96 + 2, LABEL_COLOR);
}

/// `drawGuiContainerBackgroundLayer(partial)`.
// src: GuiCrafting.java:18-25
pub fn draw_background(g: &mut GuiGl, w: i32, h: i32, x_size: i32, y_size: i32) {
    g.color4f(1.0, 1.0, 1.0, 1.0);
    g.bind_texture(CRAFTING_PNG);
    let var3 = (w - x_size) / 2;
    let var4 = (h - y_size) / 2;
    draw::draw_textured_modal_rect(g, 0.0, var3, var4, 0, 0, x_size, y_size);
}
