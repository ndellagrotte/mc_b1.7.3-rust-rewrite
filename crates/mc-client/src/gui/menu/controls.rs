//! `GuiControls` (GuiControls.java): the ten key bindings as 70-pixel
//! buttons showing the key's LWJGL name, two columns, each with its
//! translated description to the right. Clicking one shows `> KEY <`; the
//! next key press rebinds it (and saves `options.txt`). Beta 1.7.3 marks no
//! conflicts between bindings.

use super::display_parent;
use crate::app::ScreenKind;
use crate::gui::batch::GuiGl;
use crate::gui::draw;
use crate::gui::screen::{
    Screen, ScreenBase, ScreenCtx, default_draw_screen, default_key_typed, draw_default_background,
};
use crate::gui::widgets::button::GuiButton;

/// `GuiControls`.
pub struct GuiControls {
    base: ScreenBase,
    /// `parentScreen`.
    parent_screen: Option<Box<dyn Screen>>,
    /// `screenTitle`.
    pub screen_title: String,
    /// `buttonId`: the binding waiting for a key, or -1.
    pub button_id: i32,
}

impl GuiControls {
    /// `new GuiControls(parent, settings)`.
    // src: GuiControls.java:9-12
    pub fn new(parent: Option<Box<dyn Screen>>) -> Self {
        GuiControls {
            base: ScreenBase::default(),
            parent_screen: parent,
            screen_title: "Controls".to_string(),
            button_id: -1,
        }
    }

    /// `func_20080_j()`: the left column's x.
    // src: GuiControls.java:14-16
    fn left(&self) -> i32 {
        self.base.width / 2 - 155
    }
}

/// The number of key bindings (`options.keyBindings.length`).
const BINDINGS: usize = 10;

impl Screen for GuiControls {
    fn base(&self) -> &ScreenBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn kind(&self) -> ScreenKind {
        ScreenKind::Other("GuiControls".into())
    }

    // src: GuiControls.java:18-28
    fn init_gui(&mut self, ctx: &mut ScreenCtx) {
        let var1 = ctx.lang;
        let var2 = self.left();
        let (w, h) = (self.base.width, self.base.height);
        for var3 in 0..BINDINGS {
            let v = var3 as i32;
            self.base.control_list.push(GuiButton::small_sized(
                v,
                var2 + v % 2 * 160,
                h / 6 + 24 * (v >> 1),
                70,
                20,
                ctx.game.game_settings.get_option_display_string(var3),
            ));
        }
        self.base.control_list.push(GuiButton::new(
            200,
            w / 2 - 100,
            h / 6 + 168,
            var1.translate_key("gui.done"),
        ));
        self.screen_title = var1.translate_key("controls.title");
    }

    // src: GuiControls.java:30-42
    fn action_performed(&mut self, ctx: &mut ScreenCtx, button: usize) {
        for var2 in 0..BINDINGS {
            self.base.control_list[var2].display_string =
                ctx.game.game_settings.get_option_display_string(var2);
        }
        let id = self.base.control_list[button].id;
        if id == 200 {
            display_parent(ctx, &mut self.parent_screen);
        } else {
            self.button_id = id;
            self.base.control_list[button].display_string = format!(
                "> {} <",
                ctx.game
                    .game_settings
                    .get_option_display_string(id as usize)
            );
        }
    }

    // src: GuiControls.java:44-53
    fn key_typed(&mut self, ctx: &mut ScreenCtx, _ch: char, key: i32) {
        if self.button_id >= 0 {
            let i = self.button_id as usize;
            // GameSettings.setKeyBinding: the binding, then saveOptions()
            // (GameSettings.java:95-98).
            ctx.game.game_settings.set_key_binding(i, key);
            ctx.game.save_options();
            self.base.control_list[i].display_string =
                ctx.game.game_settings.get_option_display_string(i);
            self.button_id = -1;
        } else {
            default_key_typed(ctx, key);
        }
    }

    // src: GuiControls.java:55-65
    fn draw_screen(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, mx: i32, my: i32, _p: f32) {
        draw_default_background(&self.base, ctx, g);
        let f = ctx.font;
        draw::draw_centered_string(g, f, &self.screen_title, self.base.width / 2, 20, 16777215);
        let var4 = self.left();
        let h = self.base.height;
        for var5 in 0..BINDINGS {
            let v = var5 as i32;
            let d = ctx
                .game
                .game_settings
                .get_key_binding_description(var5, ctx.lang);
            draw::draw_string(
                g,
                f,
                &d,
                var4 + v % 2 * 160 + 70 + 6,
                h / 6 + 24 * (v >> 1) + 7,
                -1,
            );
        }
        default_draw_screen(&mut self.base, ctx, g, mx, my);
    }
}
