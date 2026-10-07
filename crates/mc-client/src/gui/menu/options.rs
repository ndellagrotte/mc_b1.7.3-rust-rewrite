//! `GuiOptions` (GuiOptions.java): music, sound, invert mouse, sensitivity
//! and difficulty, then Video Settings..., Controls... and Done. Changes go
//! through [`Game::set_option_value`] (`GameSettings.setOptionValue`, which
//! saves `options.txt` and sends the side effects) and the sliders'
//! `setOptionFloatValue` (WP-3.6a's `GuiSlider`).
//!
//! [`Game::set_option_value`]: crate::game::Game::set_option_value

use super::controls::GuiControls;
use super::display_parent;
use super::video_settings::GuiVideoSettings;
use crate::app::ScreenKind;
use crate::gui::batch::GuiGl;
use crate::gui::draw;
use crate::gui::screen::{
    Screen, ScreenBase, ScreenCtx, default_draw_screen, draw_default_background,
};
use crate::gui::widgets::button::{ButtonKind, GuiButton};
use crate::lang::StringTranslate;
use crate::settings::{EnumOptions, GameSettings};

/// `GuiOptions.relevantOptions`.
// src: GuiOptions.java:68-70
pub const RELEVANT_OPTIONS: [EnumOptions; 5] = [
    EnumOptions::Music,
    EnumOptions::Sound,
    EnumOptions::InvertMouse,
    EnumOptions::Sensitivity,
    EnumOptions::Difficulty,
];

/// The options grid both option screens build in `initGui`: two columns
/// 160 apart from `width / 2 - 155`, rows 24 apart from `height / 6`; a
/// `GuiSmallButton` for an ordinary option, a `GuiSlider` for a float one;
/// the button id is the option's ordinal and its label
/// `getKeyBinding(option)`.
// src: GuiOptions.java:17-30, GuiVideoSettings.java:17-30
pub fn option_buttons(
    options: &[EnumOptions],
    w: i32,
    h: i32,
    settings: &GameSettings,
    lang: &StringTranslate,
) -> Vec<GuiButton> {
    let mut out = Vec::new();
    for (var2, &var6) in options.iter().enumerate() {
        let var2 = var2 as i32;
        let x = w / 2 - 155 + var2 % 2 * 160;
        let y = h / 6 + 24 * (var2 >> 1);
        let label = settings.get_key_binding(var6, lang);
        if !var6.get_enum_float() {
            out.push(GuiButton::small_option(
                var6.return_enum_ordinal(),
                x,
                y,
                Some(var6),
                label,
            ));
        } else {
            out.push(GuiButton::slider(
                var6.return_enum_ordinal(),
                x,
                y,
                var6,
                label,
                settings.get_option_float_value(var6),
            ));
        }
    }
    out
}

/// The option buttons' `actionPerformed` half: a `GuiSmallButton` with an
/// id below 100 steps its option (`setOptionValue(option, 1)`) and shows
/// the new label (`getKeyBinding(EnumOptions.getEnumOptions(id))`).
// src: GuiOptions.java:39-42, GuiVideoSettings.java:37-40
pub fn step_option_button(ctx: &mut ScreenCtx, b: &mut GuiButton) {
    if b.id < 100
        && let ButtonKind::Small { enum_options } = b.kind
    {
        if let Some(o) = enum_options {
            ctx.game.set_option_value(o, 1);
        }
        if let Some(o) = EnumOptions::get_enum_options(b.id) {
            b.display_string = ctx.game.game_settings.get_key_binding(o, ctx.lang);
        }
    }
}

/// `GuiOptions`.
pub struct GuiOptions {
    base: ScreenBase,
    /// `parentScreen`.
    parent_screen: Option<Box<dyn Screen>>,
    /// `screenTitle`.
    pub screen_title: String,
}

impl GuiOptions {
    /// `new GuiOptions(parent, mc.gameSettings)` (the settings are
    /// `ctx.game.game_settings`).
    // src: GuiOptions.java:9-12
    pub fn new(parent: Option<Box<dyn Screen>>) -> Self {
        GuiOptions {
            base: ScreenBase::default(),
            parent_screen: parent,
            screen_title: "Options".to_string(),
        }
    }
}

impl Screen for GuiOptions {
    fn base(&self) -> &ScreenBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn kind(&self) -> ScreenKind {
        ScreenKind::Other("GuiOptions".into())
    }

    // src: GuiOptions.java:14-35
    fn init_gui(&mut self, ctx: &mut ScreenCtx) {
        let var1 = ctx.lang;
        self.screen_title = var1.translate_key("options.title");
        let (w, h) = (self.base.width, self.base.height);
        let buttons = option_buttons(&RELEVANT_OPTIONS, w, h, &ctx.game.game_settings, var1);
        let c = &mut self.base.control_list;
        c.extend(buttons);
        c.push(GuiButton::new(
            101,
            w / 2 - 100,
            h / 6 + 96 + 12,
            var1.translate_key("options.video"),
        ));
        c.push(GuiButton::new(
            100,
            w / 2 - 100,
            h / 6 + 120 + 12,
            var1.translate_key("options.controls"),
        ));
        c.push(GuiButton::new(
            200,
            w / 2 - 100,
            h / 6 + 168,
            var1.translate_key("gui.done"),
        ));
    }

    // src: GuiOptions.java:37-60
    fn action_performed(&mut self, ctx: &mut ScreenCtx, button: usize) {
        if !self.base.control_list[button].enabled {
            return;
        }
        step_option_button(ctx, &mut self.base.control_list[button]);
        let id = self.base.control_list[button].id;
        if id == 101 {
            ctx.game.save_options();
            ctx.display_wrapping_this(|this| Box::new(GuiVideoSettings::new(Some(this))));
        }
        if id == 100 {
            ctx.game.save_options();
            ctx.display_wrapping_this(|this| Box::new(GuiControls::new(Some(this))));
        }
        if id == 200 {
            ctx.game.save_options();
            display_parent(ctx, &mut self.parent_screen);
        }
    }

    // src: GuiOptions.java:62-66
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::menu::video_settings::VIDEO_OPTIONS;

    /// Every `EnumOptions` label at the defaults, with keys translating to
    /// themselves (no jar): `getKeyBinding`'s branches.
    #[test]
    fn labels_for_every_option() {
        let s = GameSettings::default();
        let t = StringTranslate::default();
        let got: Vec<String> = EnumOptions::VALUES
            .iter()
            .map(|&o| s.get_key_binding(o, &t))
            .collect();
        assert_eq!(
            got,
            vec![
                "options.music: 100%",
                "options.sound: 100%",
                "options.invertMouse: options.off",
                "options.sensitivity: 100%",
                "options.renderDistance: options.renderDistance.far",
                "options.viewBobbing: options.on",
                "options.anaglyph: options.off",
                "options.advancedOpengl: options.off",
                "options.framerateLimit: performance.balanced",
                "options.difficulty: options.difficulty.normal",
                "options.graphics: options.graphics.fancy",
                "options.ao: options.on",
                "options.guiScale: options.guiScale.auto",
            ]
        );
    }

    /// The grid: ids are the ordinals, sliders for the float options.
    #[test]
    fn option_grid_layout() {
        let s = GameSettings::default();
        let t = StringTranslate::default();
        let b = option_buttons(&RELEVANT_OPTIONS, 427, 240, &s, &t);
        let got: Vec<(i32, i32, i32, bool)> = b
            .iter()
            .map(|b| {
                (
                    b.id,
                    b.x_position,
                    b.y_position,
                    matches!(b.kind, ButtonKind::Slider { .. }),
                )
            })
            .collect();
        assert_eq!(
            got,
            vec![
                (0, 58, 40, true),
                (1, 218, 40, true),
                (2, 58, 64, false),
                (3, 218, 64, true),
                (9, 58, 88, false),
            ]
        );
        let v = option_buttons(&VIDEO_OPTIONS, 427, 240, &s, &t);
        let ids: Vec<i32> = v.iter().map(|b| b.id).collect();
        assert_eq!(ids, vec![10, 4, 11, 8, 6, 5, 12, 7]);
        assert!(
            v.iter()
                .all(|b| matches!(b.kind, ButtonKind::Small { .. }) && b.width == 150)
        );
        assert_eq!((v[7].x_position, v[7].y_position), (218, 112));
    }

    /// With the user's jar: the English labels vanilla shows.
    #[test]
    fn english_labels_with_the_jar_when_present() {
        let jar = crate::paths::default_jar();
        let Ok(jar) = crate::assets::JarAssets::open(&jar) else {
            return;
        };
        let t = jar.string_translate();
        let s = GameSettings::default();
        assert_eq!(s.get_key_binding(EnumOptions::Music, &t), "Music: 100%");
        assert_eq!(
            s.get_key_binding(EnumOptions::Difficulty, &t),
            "Difficulty: Normal"
        );
        assert_eq!(
            s.get_key_binding(EnumOptions::FramerateLimit, &t),
            "Performance: Balanced"
        );
        assert_eq!(
            s.get_key_binding(EnumOptions::GuiScale, &t),
            "GUI Scale: Auto"
        );
    }
}
