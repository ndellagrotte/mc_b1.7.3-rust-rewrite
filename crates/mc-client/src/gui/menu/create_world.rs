//! `GuiCreateWorld` (GuiCreateWorld.java): the world name and seed boxes,
//! the folder the world will get ("Will be saved in: …"), Create New World
//! and Cancel.
//!
//! The folder name is the trimmed world name with each of
//! `ChatAllowedCharacters.allowedCharactersArray` (despite its name, the
//! characters a file name may not hold) replaced by `_`, `"World"` when
//! that leaves nothing, then made unused by appending `-` while a world
//! with that folder exists ([`folder_name`]). The seed text is a `long`
//! (0 keeps the random seed), else its `String.hashCode()`
//! ([`parse_seed`]).

use mc_core::java_random::JavaRandom;
use mc_sim::player_controller::PlayerControllerSp;
use mc_sim::save::SaveFormat;

use super::display_parent;
use crate::app::ScreenKind;
use crate::clock;
use crate::gui::batch::GuiGl;
use crate::gui::draw;
use crate::gui::font::java_trim;
use crate::gui::screen::{
    Screen, ScreenBase, ScreenCtx, default_draw_screen, default_mouse_clicked,
    draw_default_background,
};
use crate::gui::widgets::button::GuiButton;
use crate::gui::widgets::textfield::GuiTextField;
use crate::jfmt;

/// `ChatAllowedCharacters.allowedCharactersArray`: the characters a folder
/// name may not contain.
// src: ChatAllowedCharacters.java:8
pub const ILLEGAL_FOLDER_CHARACTERS: [char; 15] = [
    '/', '\n', '\r', '\t', '\0', '\u{c}', '`', '?', '*', '\\', '<', '>', '|', '"', ':',
];

/// The label colour (`10526880`, `0xa0a0a0`).
// src: GuiCreateWorld.java:121
pub const LABEL_COLOR: i32 = 10526880;

/// `calcSaveDirName`'s first half: trimmed, each illegal character
/// replaced by `_`, `"World"` when empty.
// src: GuiCreateWorld.java:35-47
pub fn sanitize_folder_name(world_name: &str) -> String {
    let mut s = java_trim(world_name).to_string();
    for c in ILLEGAL_FOLDER_CHARACTERS {
        s = s.replace(c, "_");
    }
    // MathHelper.stringNullOrLengthZero (MathHelper.java:52-54)
    if s.is_empty() {
        s = "World".to_string();
    }
    s
}

/// `generateUnusedFolderName(saveFormat, name)`: append `-` while
/// `getWorldInfo(name)` finds a world.
// src: GuiCreateWorld.java:52-58
pub fn generate_unused_folder_name(mut exists: impl FnMut(&str) -> bool, var1: &str) -> String {
    let mut var1 = var1.to_string();
    while exists(&var1) {
        var1.push('-');
    }
    var1
}

/// The whole `calcSaveDirName`.
// src: GuiCreateWorld.java:35-50
pub fn folder_name(world_name: &str, exists: impl FnMut(&str) -> bool) -> String {
    generate_unused_folder_name(exists, &sanitize_folder_name(world_name))
}

/// The create button's seed: `random` (`new Random().nextLong()`) unless
/// the text is not empty: then `Long.parseLong(text)` when it parses (a
/// parsed 0 keeps `random`), else `(long) text.hashCode()`.
// src: GuiCreateWorld.java:75-86
pub fn parse_seed(var4: &str, random: i64) -> i64 {
    let mut var2 = random;
    if !var4.is_empty() {
        match jfmt::parse_long(var4) {
            Ok(var5) => {
                if var5 != 0 {
                    var2 = var5;
                }
            }
            Err(_) => var2 = jfmt::string_hash_code(var4) as i64,
        }
    }
    var2
}

/// `GuiCreateWorld`.
pub struct GuiCreateWorld {
    base: ScreenBase,
    /// `parentGuiScreen`.
    parent_gui_screen: Option<Box<dyn Screen>>,
    /// `textboxWorldName`.
    pub textbox_world_name: GuiTextField,
    /// `textboxSeed`.
    pub textbox_seed: GuiTextField,
    /// `folderName`.
    pub folder_name: String,
    /// `createClicked`.
    create_clicked: bool,
}

impl GuiCreateWorld {
    /// `new GuiCreateWorld(parent)` (the boxes are made by `initGui`).
    // src: GuiCreateWorld.java:13-15
    pub fn new(parent: Option<Box<dyn Screen>>) -> Self {
        GuiCreateWorld {
            base: ScreenBase::default(),
            parent_gui_screen: parent,
            textbox_world_name: GuiTextField::new(0, 0, 0, 0, ""),
            textbox_seed: GuiTextField::new(0, 0, 0, 0, ""),
            folder_name: String::new(),
            create_clicked: false,
        }
    }

    /// `calcSaveDirName()`.
    // src: GuiCreateWorld.java:35-50
    fn calc_save_dir_name(&mut self, ctx: &mut ScreenCtx) {
        let loader = &mut ctx.game.save_loader;
        self.folder_name = folder_name(self.textbox_world_name.get_text(), |n| {
            loader.get_world_info(n).is_some()
        });
    }
}

impl Screen for GuiCreateWorld {
    fn base(&self) -> &ScreenBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn kind(&self) -> ScreenKind {
        ScreenKind::Other("GuiCreateWorld".into())
    }

    // src: GuiCreateWorld.java:17-20
    fn update_screen(&mut self, _ctx: &mut ScreenCtx) {
        self.textbox_world_name.update_cursor_counter();
        self.textbox_seed.update_cursor_counter();
    }

    // src: GuiCreateWorld.java:22-33
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
            var1.translate_key("selectWorld.create"),
        ));
        c.push(GuiButton::new(
            1,
            w / 2 - 100,
            h / 4 + 120 + 12,
            var1.translate_key("gui.cancel"),
        ));
        self.textbox_world_name = GuiTextField::new(
            w / 2 - 100,
            60,
            200,
            20,
            var1.translate_key("selectWorld.newWorld"),
        );
        self.textbox_world_name.is_focused = true;
        self.textbox_world_name.set_max_string_length(32);
        self.textbox_seed = GuiTextField::new(w / 2 - 100, 116, 200, 20, "");
        self.calc_save_dir_name(ctx);
    }

    // src: GuiCreateWorld.java:60-62
    fn on_gui_closed(&mut self, ctx: &mut ScreenCtx) {
        ctx.enable_repeat_events(false);
    }

    // src: GuiCreateWorld.java:64-94
    fn action_performed(&mut self, ctx: &mut ScreenCtx, button: usize) {
        if !self.base.control_list[button].enabled {
            return;
        }
        let id = self.base.control_list[button].id;
        if id == 1 {
            display_parent(ctx, &mut self.parent_gui_screen);
        } else if id == 0 {
            ctx.display_gui_screen(None);
            if self.create_clicked {
                return;
            }
            self.create_clicked = true;
            let random = JavaRandom::new(clock::new_random_seed()).next_long();
            let var2 = parse_seed(self.textbox_seed.get_text(), random);
            ctx.game.player_controller = Some(PlayerControllerSp::new());
            let folder = self.folder_name.clone();
            let name = self.textbox_world_name.get_text().to_string();
            if let Err(e) = ctx.game.start_world(&folder, &name, var2) {
                bevy::log::error!("{e}");
                ctx.game.last_error = Some(e);
            }
            ctx.display_gui_screen(None);
        }
    }

    // src: GuiCreateWorld.java:96-109
    fn key_typed(&mut self, ctx: &mut ScreenCtx, var1: char, var2: i32) {
        let allowed = &ctx.font.allowed;
        let clip = if var1 == '\u{16}' {
            ctx.get_clipboard_string()
        } else {
            None
        };
        let next = if self.textbox_world_name.is_focused {
            self.textbox_world_name
                .textbox_key_typed(allowed, || clip, var1, var2)
        } else {
            self.textbox_seed
                .textbox_key_typed(allowed, || clip, var1, var2)
        };
        if next {
            // GuiTextField's parentGuiScreen.selectNextField() (Tab).
            self.select_next_field(ctx);
        }
        if var1 == '\r' {
            self.action_performed(ctx, 0);
        }
        self.base.control_list[0].enabled = !self.textbox_world_name.get_text().is_empty();
        self.calc_save_dir_name(ctx);
    }

    // src: GuiCreateWorld.java:111-115
    fn mouse_clicked(&mut self, ctx: &mut ScreenCtx, x: i32, y: i32, button: i32) {
        default_mouse_clicked(self, ctx, x, y, button);
        self.textbox_world_name.mouse_clicked(x, y, button);
        self.textbox_seed.mouse_clicked(x, y, button);
    }

    // src: GuiCreateWorld.java:117-128
    fn draw_screen(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, mx: i32, my: i32, _p: f32) {
        let var4 = ctx.lang;
        let f = ctx.font;
        draw_default_background(&self.base, ctx, g);
        let (w, h) = (self.base.width, self.base.height);
        draw::draw_centered_string(
            g,
            f,
            &var4.translate_key("selectWorld.create"),
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
        draw::draw_string(
            g,
            f,
            &format!(
                "{} {}",
                var4.translate_key("selectWorld.resultFolder"),
                self.folder_name
            ),
            w / 2 - 100,
            85,
            LABEL_COLOR,
        );
        draw::draw_string(
            g,
            f,
            &var4.translate_key("selectWorld.enterSeed"),
            w / 2 - 100,
            104,
            LABEL_COLOR,
        );
        draw::draw_string(
            g,
            f,
            &var4.translate_key("selectWorld.seedInfo"),
            w / 2 - 100,
            140,
            LABEL_COLOR,
        );
        self.textbox_world_name.draw_text_box(g, f);
        self.textbox_seed.draw_text_box(g, f);
        default_draw_screen(&mut self.base, ctx, g, mx, my);
    }

    // src: GuiCreateWorld.java:130-139
    fn select_next_field(&mut self, _ctx: &mut ScreenCtx) {
        if self.textbox_world_name.is_focused {
            self.textbox_world_name.set_focused(false);
            self.textbox_seed.set_focused(true);
        } else {
            self.textbox_world_name.set_focused(true);
            self.textbox_seed.set_focused(false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_parsing() {
        // Empty: the random seed.
        assert_eq!(parse_seed("", 42), 42);
        // A long.
        assert_eq!(parse_seed("123", 42), 123);
        assert_eq!(parse_seed("-9223372036854775808", 42), i64::MIN);
        // 0 keeps the random seed.
        assert_eq!(parse_seed("0", 42), 42);
        // Not a long: String.hashCode() ("hello".hashCode() == 99162322).
        assert_eq!(parse_seed("hello", 42), 99162322);
        // Out of range for a long: hashCode too.
        assert_eq!(
            parse_seed("9223372036854775808", 42),
            jfmt::string_hash_code("9223372036854775808") as i64
        );
        // Long.parseLong does not trim.
        assert_eq!(parse_seed(" 1", 42), jfmt::string_hash_code(" 1") as i64);
    }

    #[test]
    fn folder_names() {
        assert_eq!(sanitize_folder_name("New World"), "New World");
        assert_eq!(sanitize_folder_name("  a/b\\c:d*e?f  "), "a_b_c_d_e_f");
        assert_eq!(sanitize_folder_name("<x>|\"y\"`"), "_x___y__");
        assert_eq!(sanitize_folder_name("   "), "World");
        assert_eq!(sanitize_folder_name(""), "World");
        // Dots and spaces inside are kept; reserved names are not special.
        assert_eq!(sanitize_folder_name("CON. x"), "CON. x");
        let taken = ["World", "World-", "New World"];
        assert_eq!(folder_name("", |n| taken.contains(&n)), "World--");
        assert_eq!(
            folder_name("New World", |n| taken.contains(&n)),
            "New World-"
        );
        assert_eq!(folder_name("Fresh", |n| taken.contains(&n)), "Fresh");
    }
}
