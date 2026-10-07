//! `GuiDispenser` (GuiDispenser.java): the dispenser's 3×3 slots,
//! `/gui/trap.png`.

use super::LABEL_COLOR;
use crate::gui::batch::GuiGl;
use crate::gui::draw;
use crate::gui::font::FontRenderer;

/// `/gui/trap.png`.
// src: GuiDispenser.java:13
pub const TRAP_PNG: &str = "/gui/trap.png";

/// `drawGuiContainerForegroundLayer()`.
// src: GuiDispenser.java:8-11
pub fn draw_foreground(g: &mut GuiGl, f: &FontRenderer, y_size: i32) {
    f.draw_string(g, "Dispenser", 60, 6, LABEL_COLOR);
    f.draw_string(g, "Inventory", 8, y_size - 96 + 2, LABEL_COLOR);
}

/// `drawGuiContainerBackgroundLayer(partial)`.
// src: GuiDispenser.java:13-20
pub fn draw_background(g: &mut GuiGl, w: i32, h: i32, x_size: i32, y_size: i32) {
    g.color4f(1.0, 1.0, 1.0, 1.0);
    g.bind_texture(TRAP_PNG);
    let var3 = (w - x_size) / 2;
    let var4 = (h - y_size) / 2;
    draw::draw_textured_modal_rect(g, 0.0, var3, var4, 0, 0, x_size, y_size);
}
