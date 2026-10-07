//! `GuiIngameMenu` (GuiIngameMenu.java): the pause menu Escape opens — Back
//! to game, Achievements, Statistics, Options..., Save and quit to title —
//! with the "Game menu" title and the blinking "Saving level.." while
//! `quickSaveWorld` has chunks left to write.

use mc_core::math;
use mc_sim::stats::{stat, stat_list};

use super::achievements::GuiAchievements;
use super::options::GuiOptions;
use super::stats::GuiStats;
use crate::app::ScreenKind;
use crate::gui::batch::GuiGl;
use crate::gui::draw;
use crate::gui::screen::{
    Screen, ScreenBase, ScreenCtx, default_draw_screen, draw_default_background,
};
use crate::gui::widgets::button::GuiButton;

/// `GuiIngameMenu`.
#[derive(Default)]
pub struct GuiIngameMenu {
    base: ScreenBase,
    /// `updateCounter2`: the `quickSaveWorld` call count (0 saves the level
    /// too).
    pub update_counter2: i32,
    /// `updateCounter`: ticks open.
    pub update_counter: i32,
}

impl Screen for GuiIngameMenu {
    fn base(&self) -> &ScreenBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn kind(&self) -> ScreenKind {
        ScreenKind::IngameMenu
    }

    // src: GuiIngameMenu.java:7-20
    fn init_gui(&mut self, ctx: &mut ScreenCtx) {
        self.update_counter2 = 0;
        self.base.control_list.clear();
        let var1: i32 = -16;
        let (w, h) = (self.base.width, self.base.height);
        let lang = ctx.lang;
        let c = &mut self.base.control_list;
        c.push(GuiButton::new(
            1,
            w / 2 - 100,
            h / 4 + 120 + var1,
            "Save and quit to title",
        ));
        if ctx.game.is_multiplayer_world() {
            c[0].display_string = "Disconnect".to_string();
        }
        c.push(GuiButton::new(
            4,
            w / 2 - 100,
            h / 4 + 24 + var1,
            "Back to game",
        ));
        c.push(GuiButton::new(
            0,
            w / 2 - 100,
            h / 4 + 96 + var1,
            "Options...",
        ));
        // StatCollector.translateToLocal = StringTranslate.translateKey
        // (StatCollector.java:6-8).
        c.push(GuiButton::with_size(
            5,
            w / 2 - 100,
            h / 4 + 48 + var1,
            98,
            20,
            lang.translate_key("gui.achievements"),
        ));
        c.push(GuiButton::with_size(
            6,
            w / 2 + 2,
            h / 4 + 48 + var1,
            98,
            20,
            lang.translate_key("gui.stats"),
        ));
    }

    // src: GuiIngameMenu.java:22-50
    fn action_performed(&mut self, ctx: &mut ScreenCtx, button: usize) {
        let id = self.base.control_list[button].id;
        if id == 0 {
            ctx.display_wrapping_this(|this| Box::new(GuiOptions::new(Some(this))));
        }
        if id == 1 {
            if let Some(s) = stat(stat_list::LEAVE_GAME_STAT) {
                ctx.game.stats_mut().read_stat(s, 1);
            }
            // `theWorld.sendQuittingDisconnectingPacket()` for a multiplayer
            // world: deferred (D3).
            ctx.game.change_world1(None);
            super::main_menu::display(ctx);
        }
        if id == 4 {
            ctx.display_gui_screen(None);
            ctx.set_ingame_focus();
        }
        if id == 5 {
            ctx.display_gui_screen(Some(Box::new(GuiAchievements::new())));
        }
        if id == 6 {
            ctx.display_wrapping_this(|this| Box::new(GuiStats::new(Some(this))));
        }
    }

    // src: GuiIngameMenu.java:52-56
    fn update_screen(&mut self, _ctx: &mut ScreenCtx) {
        self.update_counter = self.update_counter.wrapping_add(1);
    }

    // src: GuiIngameMenu.java:58-70
    fn draw_screen(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, mx: i32, my: i32, var3: f32) {
        draw_default_background(&self.base, ctx, g);
        let n = self.update_counter2;
        self.update_counter2 = n.wrapping_add(1);
        let var4 = match ctx.game.the_world.as_mut() {
            Some(w) => !w.quick_save_world(n),
            // No world (the menu outlived it): Java would throw; nothing
            // to save.
            None => false,
        };
        let h = self.base.height;
        if var4 || self.update_counter < 20 {
            let mut var5 = ((self.update_counter % 10) as f32 + var3) / 10.0;
            var5 = math::sin(var5 * 3.1415927 * 2.0) * 0.2 + 0.8;
            let var6 = (255.0 * var5) as i32;
            draw::draw_string(
                g,
                ctx.font,
                "Saving level..",
                8,
                h - 16,
                var6 << 16 | var6 << 8 | var6,
            );
        }
        draw::draw_centered_string(g, ctx.font, "Game menu", self.base.width / 2, 40, 16777215);
        default_draw_screen(&mut self.base, ctx, g, mx, my);
    }
}
