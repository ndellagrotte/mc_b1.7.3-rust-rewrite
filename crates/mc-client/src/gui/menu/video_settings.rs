//! `GuiVideoSettings` (GuiVideoSettings.java): graphics, render distance,
//! smooth lighting, performance, 3D anaglyph, view bobbing, GUI scale,
//! advanced OpenGL, and Done. Every enabled button press ends with
//! `setWorldAndResolution` at the current scaled size, so a GUI scale
//! change lays the screen out again at once.

use super::display_parent;
use super::options::{option_buttons, step_option_button};
use crate::app::{ScaledResolution, ScreenKind};
use crate::gui::batch::GuiGl;
use crate::gui::draw;
use crate::gui::screen::{
    Screen, ScreenBase, ScreenCtx, default_draw_screen, draw_default_background,
    set_world_and_resolution,
};
use crate::gui::widgets::button::GuiButton;
use crate::settings::EnumOptions;

/// `GuiVideoSettings.videoOptions`.
// src: GuiVideoSettings.java:131-133
pub const VIDEO_OPTIONS: [EnumOptions; 8] = [
    EnumOptions::Graphics,
    EnumOptions::RenderDistance,
    EnumOptions::AmbientOcclusion,
    EnumOptions::FramerateLimit,
    EnumOptions::Anaglyph,
    EnumOptions::ViewBobbing,
    EnumOptions::GuiScale,
    EnumOptions::AdvancedOpengl,
];

/// `GuiVideoSettings`.
pub struct GuiVideoSettings {
    base: ScreenBase,
    /// `parentGuiScreen`.
    parent_gui_screen: Option<Box<dyn Screen>>,
    /// `screenTitle`.
    pub screen_title: String,
}

impl GuiVideoSettings {
    /// `new GuiVideoSettings(parent, settings)`.
    // src: GuiVideoSettings.java:9-12
    pub fn new(parent: Option<Box<dyn Screen>>) -> Self {
        GuiVideoSettings {
            base: ScreenBase::default(),
            parent_gui_screen: parent,
            screen_title: "Video Settings".to_string(),
        }
    }
}

impl Screen for GuiVideoSettings {
    fn base(&self) -> &ScreenBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn kind(&self) -> ScreenKind {
        ScreenKind::Other("GuiVideoSettings".into())
    }

    // src: GuiVideoSettings.java:14-32
    fn init_gui(&mut self, ctx: &mut ScreenCtx) {
        let var1 = ctx.lang;
        self.screen_title = var1.translate_key("options.videoTitle");
        let (w, h) = (self.base.width, self.base.height);
        let buttons = option_buttons(&VIDEO_OPTIONS, w, h, &ctx.game.game_settings, var1);
        self.base.control_list.extend(buttons);
        self.base.control_list.push(GuiButton::new(
            200,
            w / 2 - 100,
            h / 6 + 168,
            var1.translate_key("gui.done"),
        ));
    }

    // src: GuiVideoSettings.java:34-51
    fn action_performed(&mut self, ctx: &mut ScreenCtx, button: usize) {
        if !self.base.control_list[button].enabled {
            return;
        }
        step_option_button(ctx, &mut self.base.control_list[button]);
        if self.base.control_list[button].id == 200 {
            ctx.game.save_options();
            display_parent(ctx, &mut self.parent_gui_screen);
        }
        // Also after Done, on this (now closed) screen, as the Java does.
        let var2 = ScaledResolution::new(
            ctx.game.game_settings.gui_scale,
            ctx.game.display_width,
            ctx.game.display_height,
        );
        set_world_and_resolution(self, ctx, var2.scaled_width, var2.scaled_height);
    }

    // src: GuiVideoSettings.java:53-57
    fn draw_screen(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, mx: i32, my: i32, _p: f32) {
        draw_default_background(&self.base, ctx, g);
        draw::draw_centered_string(
            g,
            ctx.font,
            &self.screen_title,
            self.base.width / 2,
            20,
            16777215,
        );
        default_draw_screen(&mut self.base, ctx, g, mx, my);
    }
}
