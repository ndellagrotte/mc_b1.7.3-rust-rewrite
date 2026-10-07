//! `GuiErrorScreen` (GuiErrorScreen.java): "Out of memory!". Vanilla opens
//! it from the out-of-memory handler, which the shell does not port (no
//! reserve buffer to free); the screen is here for completeness and for
//! `open_screen`.

use crate::app::ScreenKind;
use crate::gui::batch::GuiGl;
use crate::gui::draw;
use crate::gui::screen::{
    Screen, ScreenBase, ScreenCtx, default_draw_screen, draw_default_background,
};

/// The text colour of the explanation (`10526880`, `0xa0a0a0`).
// src: GuiErrorScreen.java:22
pub const GREY: i32 = 10526880;

/// `GuiErrorScreen`.
#[derive(Clone, Debug, Default)]
pub struct GuiErrorScreen {
    base: ScreenBase,
    /// `field_28098_a`: ticks open (counted, never read).
    pub update_counter: i32,
}

impl Screen for GuiErrorScreen {
    fn base(&self) -> &ScreenBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn kind(&self) -> ScreenKind {
        ScreenKind::ErrorScreen
    }
    // src: GuiErrorScreen.java:6-8
    fn update_screen(&mut self, _ctx: &mut ScreenCtx) {
        self.update_counter = self.update_counter.wrapping_add(1);
    }
    // src: GuiErrorScreen.java:10-11
    fn init_gui(&mut self, _ctx: &mut ScreenCtx) {}
    // src: GuiErrorScreen.java:13-14
    fn action_performed(&mut self, _ctx: &mut ScreenCtx, _button: usize) {}
    // src: GuiErrorScreen.java:16-17
    fn key_typed(&mut self, _ctx: &mut ScreenCtx, _ch: char, _key: i32) {}
    // src: GuiErrorScreen.java:19-30
    fn draw_screen(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, mx: i32, my: i32, _p: f32) {
        draw_default_background(&self.base, ctx, g);
        let (w, h) = (self.base.width, self.base.height);
        let f = ctx.font;
        draw::draw_centered_string(g, f, "Out of memory!", w / 2, h / 4 - 60 + 20, 16777215);
        let x = w / 2 - 140;
        let y = h / 4 - 60 + 60;
        let lines: [(&str, i32); 7] = [
            ("Minecraft has run out of memory.", 0),
            ("This could be caused by a bug in the game or by the", 18),
            ("Java Virtual Machine not being allocated enough", 27),
            ("memory. If you are playing in a web browser, try", 36),
            ("downloading the game and playing it offline.", 45),
            (
                "To prevent level corruption, the current game has quit.",
                63,
            ),
            ("Please restart the game.", 81),
        ];
        for (s, dy) in lines {
            draw::draw_string(g, f, s, x, y + dy, GREY);
        }
        default_draw_screen(&mut self.base, ctx, g, mx, my);
    }
}
