//! `GuiRenameWorld` (GuiRenameWorld.java): one box with the world's level
//! name, Rename and Cancel. Rename writes the trimmed text as the level
//! name of `level.dat` (`ISaveFormat.renameWorld`); the folder keeps its
//! name.

use mc_sim::save::SaveFormat;

use super::create_world::LABEL_COLOR;
use super::display_parent;
use crate::app::ScreenKind;
use crate::gui::batch::GuiGl;
use crate::gui::draw;
use crate::gui::font::java_trim;
use crate::gui::screen::{
    Screen, ScreenBase, ScreenCtx, default_draw_screen, default_mouse_clicked,
    draw_default_background,
};
use crate::gui::widgets::button::GuiButton;
use crate::gui::widgets::textfield::GuiTextField;

/// `GuiRenameWorld`.
pub struct GuiRenameWorld {
    base: ScreenBase,
    /// `parentGuiScreen`.
    parent_gui_screen: Option<Box<dyn Screen>>,
    /// `theGuiTextField`.
    pub the_gui_text_field: GuiTextField,
    /// `worldName`: the world's folder.
    pub world_name: String,
}

impl GuiRenameWorld {
    /// `new GuiRenameWorld(parent, folderName)`.
    // src: GuiRenameWorld.java:10-13
    pub fn new(parent: Option<Box<dyn Screen>>, world_name: impl Into<String>) -> Self {
        GuiRenameWorld {
            base: ScreenBase::default(),
            parent_gui_screen: parent,
            the_gui_text_field: GuiTextField::new(0, 0, 0, 0, ""),
            world_name: world_name.into(),
        }
    }
}

impl Screen for GuiRenameWorld {
    fn base(&self) -> &ScreenBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn kind(&self) -> ScreenKind {
        ScreenKind::Other("GuiRenameWorld".into())
    }

    // src: GuiRenameWorld.java:15-17
    fn update_screen(&mut self, _ctx: &mut ScreenCtx) {
        self.the_gui_text_field.update_cursor_counter();
    }

    // src: GuiRenameWorld.java:19-31
    fn init_gui(&mut self, ctx: &mut ScreenCtx) {
        let var1 = ctx.lang;
        ctx.enable_repeat_events(true);
        let (w, h) = (self.base.width, self.base.height);
        let c = &mut self.base.control_list;
        c.clear();
        c.push(GuiButton::new(
            0,
            w / 2 - 100,
            h / 4 + 96 + 12,
            var1.translate_key("selectWorld.renameButton"),
        ));
        c.push(GuiButton::new(
            1,
            w / 2 - 100,
            h / 4 + 120 + 12,
            var1.translate_key("gui.cancel"),
        ));
        // `getWorldInfo(worldName).getWorldName()`: Java would throw on a
        // world without a readable level.dat; the box starts empty instead.
        let var4 = match ctx.game.save_loader.get_world_info(&self.world_name) {
            Some(i) => i.level_name,
            None => {
                bevy::log::warn!("rename: saves/{} has no level.dat", self.world_name);
                String::new()
            }
        };
        self.the_gui_text_field = GuiTextField::new(w / 2 - 100, 60, 200, 20, var4);
        self.the_gui_text_field.is_focused = true;
        self.the_gui_text_field.set_max_string_length(32);
    }

    // src: GuiRenameWorld.java:33-35
    fn on_gui_closed(&mut self, ctx: &mut ScreenCtx) {
        ctx.enable_repeat_events(false);
    }

    // src: GuiRenameWorld.java:37-48
    fn action_performed(&mut self, ctx: &mut ScreenCtx, button: usize) {
        if !self.base.control_list[button].enabled {
            return;
        }
        let id = self.base.control_list[button].id;
        if id == 1 {
            display_parent(ctx, &mut self.parent_gui_screen);
        } else if id == 0 {
            let name = java_trim(self.the_gui_text_field.get_text()).to_string();
            ctx.game.save_loader.rename_world(&self.world_name, &name);
            display_parent(ctx, &mut self.parent_gui_screen);
        }
    }

    // src: GuiRenameWorld.java:50-57
    fn key_typed(&mut self, ctx: &mut ScreenCtx, var1: char, var2: i32) {
        let allowed = &ctx.font.allowed;
        let clip = if var1 == '\u{16}' {
            ctx.get_clipboard_string()
        } else {
            None
        };
        // The only box: Tab's selectNextField is GuiScreen's empty one.
        let _ = self
            .the_gui_text_field
            .textbox_key_typed(allowed, || clip, var1, var2);
        self.base.control_list[0].enabled =
            !java_trim(self.the_gui_text_field.get_text()).is_empty();
        if var1 == '\r' {
            self.action_performed(ctx, 0);
        }
    }

    // src: GuiRenameWorld.java:59-62
    fn mouse_clicked(&mut self, ctx: &mut ScreenCtx, x: i32, y: i32, button: i32) {
        default_mouse_clicked(self, ctx, x, y, button);
        self.the_gui_text_field.mouse_clicked(x, y, button);
    }

    // src: GuiRenameWorld.java:64-71
    fn draw_screen(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, mx: i32, my: i32, _p: f32) {
        let var4 = ctx.lang;
        let f = ctx.font;
        draw_default_background(&self.base, ctx, g);
        let (w, h) = (self.base.width, self.base.height);
        draw::draw_centered_string(
            g,
            f,
            &var4.translate_key("selectWorld.renameTitle"),
            w / 2,
            h / 4 - 60 + 20,
            16777215,
        );
        draw::draw_string(
            g,
            f,
            &var4.translate_key("selectWorld.enterName"),
            w / 2 - 100,
            47,
            LABEL_COLOR,
        );
        self.the_gui_text_field.draw_text_box(g, f);
        default_draw_screen(&mut self.base, ctx, g, mx, my);
    }
}
