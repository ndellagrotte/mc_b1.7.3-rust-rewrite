//! `GuiGameOver` (GuiGameOver.java): the red screen after death, with
//! "Respawn" (`thePlayer.respawnPlayer()` = `mc.respawn(false, 0)`,
//! EntityPlayerSP.java:165-167) and "Title menu" (`changeWorld1(null)`,
//! then the main menu).

use crate::app::ScreenKind;
use crate::gui::batch::GuiGl;
use crate::gui::draw;
use crate::gui::screen::{Screen, ScreenBase, ScreenCtx, default_draw_screen};
use crate::gui::widgets::button::GuiButton;

/// The background gradient's top colour, `0x60500000`.
// src: GuiGameOver.java:36
pub const TOP_COLOR: i32 = 1615855616;
/// Its bottom colour, `0xA0803030`.
// src: GuiGameOver.java:36
pub const BOTTOM_COLOR: i32 = -1602211792;

/// `GuiGameOver`.
#[derive(Clone, Debug, Default)]
pub struct GuiGameOver {
    base: ScreenBase,
}

/// The score line, literally (vanilla bug: `&e` where the colour code is
/// `§e`, so the text shows "&e").
// src: GuiGameOver.java:41
pub fn score_line(score: i32) -> String {
    format!("Score: &e{score}")
}

impl Screen for GuiGameOver {
    fn base(&self) -> &ScreenBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn kind(&self) -> ScreenKind {
        ScreenKind::GameOver
    }
    // src: GuiGameOver.java:45-47
    fn does_gui_pause_game(&self) -> bool {
        false
    }
    // src: GuiGameOver.java:6-14 (mc.session is never null here: the
    // "Title menu" button stays enabled)
    fn init_gui(&mut self, _ctx: &mut ScreenCtx) {
        let (w, h) = (self.base.width, self.base.height);
        self.base.control_list.clear();
        self.base
            .control_list
            .push(GuiButton::new(1, w / 2 - 100, h / 4 + 72, "Respawn"));
        self.base
            .control_list
            .push(GuiButton::new(2, w / 2 - 100, h / 4 + 96, "Title menu"));
    }
    // src: GuiGameOver.java:16-17
    fn key_typed(&mut self, _ctx: &mut ScreenCtx, _ch: char, _key: i32) {}
    // src: GuiGameOver.java:19-33
    fn action_performed(&mut self, ctx: &mut ScreenCtx, button: usize) {
        let id = self.base.control_list[button].id;
        if id == 1 {
            ctx.game.respawn(false, 0);
            ctx.display_gui_screen(None);
        }
        if id == 2 {
            ctx.game.change_world1(None);
            let menu = ctx.create_screen(&ScreenKind::MainMenu);
            ctx.display_gui_screen(Some(menu));
        }
    }
    // src: GuiGameOver.java:35-43
    fn draw_screen(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, mx: i32, my: i32, _p: f32) {
        let (w, h) = (self.base.width, self.base.height);
        draw::draw_gradient_rect(g, 0, 0, w, h, TOP_COLOR, BOTTOM_COLOR);
        g.push_matrix();
        g.scalef(2.0, 2.0, 2.0);
        draw::draw_centered_string(g, ctx.font, "Game over!", w / 2 / 2, 30, 16777215);
        g.pop_matrix();
        let score = ctx
            .game
            .player()
            .and_then(|p| p.kind.player())
            .map_or(0, |d| d.score);
        draw::draw_centered_string(g, ctx.font, &score_line(score), w / 2, 100, 16777215);
        default_draw_screen(&mut self.base, ctx, g, mx, my);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_text_keeps_the_ampersand() {
        assert_eq!(score_line(7), "Score: &e7");
        assert_eq!(TOP_COLOR as u32, 0x6050_0000);
        assert_eq!(BOTTOM_COLOR as u32, 0xA080_3030);
    }
}
