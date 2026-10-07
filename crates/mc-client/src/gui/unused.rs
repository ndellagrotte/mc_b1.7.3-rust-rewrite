//! `GuiUnused` (GuiUnused.java): a screen nothing opens in single player;
//! `displayGuiScreen` refuses to replace it (the shell's check). Its two
//! messages are never set (`null`): nothing is drawn for them.

use crate::app::ScreenKind;
use crate::gui::batch::GuiGl;
use crate::gui::draw;
use crate::gui::screen::{Screen, ScreenBase, ScreenCtx, default_draw_screen};

/// `GuiUnused`.
#[derive(Clone, Debug, Default)]
pub struct GuiUnused {
    base: ScreenBase,
    /// `message1` (never set: `null`).
    pub message1: Option<String>,
    /// `message2` (never set).
    pub message2: Option<String>,
}

impl Screen for GuiUnused {
    fn base(&self) -> &ScreenBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn kind(&self) -> ScreenKind {
        ScreenKind::Unused
    }
    // src: GuiUnused.java:7-8
    fn init_gui(&mut self, _ctx: &mut ScreenCtx) {}
    // src: GuiUnused.java:10-15
    fn draw_screen(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, mx: i32, my: i32, _p: f32) {
        let (w, h) = (self.base.width, self.base.height);
        draw::draw_gradient_rect(g, 0, 0, w, h, -12574688, -11530224);
        // drawCenteredString(font, null, …): width 0, renderString(null)
        // draws nothing.
        for (m, y) in [(&self.message1, 90), (&self.message2, 110)] {
            if let Some(m) = m {
                draw::draw_centered_string(g, ctx.font, m, w / 2, y, 16777215);
            }
        }
        default_draw_screen(&mut self.base, ctx, g, mx, my);
    }
    // src: GuiUnused.java:17-18
    fn key_typed(&mut self, _ctx: &mut ScreenCtx, _ch: char, _key: i32) {}
}
