//! `GuiConflictWarning` (GuiConflictWarning.java): "Level save conflict",
//! opened by the shell when the session lock was lost
//! (`MinecraftException` in `runTick`).

use crate::app::ScreenKind;
use crate::gui::batch::GuiGl;
use crate::gui::draw;
use crate::gui::error_screen::GREY;
use crate::gui::screen::{
    Screen, ScreenBase, ScreenCtx, default_draw_screen, draw_default_background,
};
use crate::gui::widgets::button::GuiButton;

/// `GuiConflictWarning`.
#[derive(Clone, Debug, Default)]
pub struct GuiConflictWarning {
    base: ScreenBase,
    /// `updateCounter`: ticks open (counted, never read).
    pub update_counter: i32,
}

impl Screen for GuiConflictWarning {
    fn base(&self) -> &ScreenBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn kind(&self) -> ScreenKind {
        ScreenKind::ConflictWarning
    }
    // src: GuiConflictWarning.java:6-8
    fn update_screen(&mut self, _ctx: &mut ScreenCtx) {
        self.update_counter = self.update_counter.wrapping_add(1);
    }
    // src: GuiConflictWarning.java:10-13
    fn init_gui(&mut self, _ctx: &mut ScreenCtx) {
        let (w, h) = (self.base.width, self.base.height);
        self.base.control_list.clear();
        self.base.control_list.push(GuiButton::new(
            0,
            w / 2 - 100,
            h / 4 + 120 + 12,
            "Back to title screen",
        ));
    }
    // src: GuiConflictWarning.java:15-22
    fn action_performed(&mut self, ctx: &mut ScreenCtx, button: usize) {
        let b = &self.base.control_list[button];
        if b.enabled && b.id == 0 {
            let menu = ctx.create_screen(&ScreenKind::MainMenu);
            ctx.display_gui_screen(Some(menu));
        }
    }
    // src: GuiConflictWarning.java:24-32
    fn draw_screen(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, mx: i32, my: i32, _p: f32) {
        draw_default_background(&self.base, ctx, g);
        let (w, h) = (self.base.width, self.base.height);
        let f = ctx.font;
        draw::draw_centered_string(
            g,
            f,
            "Level save conflict",
            w / 2,
            h / 4 - 60 + 20,
            16777215,
        );
        let x = w / 2 - 140;
        let y = h / 4 - 60 + 60;
        let lines: [(&str, i32); 4] = [
            ("Minecraft detected a conflict in the level save data.", 0),
            ("This could be caused by two copies of the game", 18),
            ("accessing the same level.", 27),
            (
                "To prevent level corruption, the current game has quit.",
                45,
            ),
        ];
        for (s, dy) in lines {
            draw::draw_string(g, f, s, x, y + dy, GREY);
        }
        default_draw_screen(&mut self.base, ctx, g, mx, my);
    }
}
