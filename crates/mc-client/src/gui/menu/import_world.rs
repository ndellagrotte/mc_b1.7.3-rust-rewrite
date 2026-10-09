//! The world import screen (project-owned, PLAN_v4 D5 and §3.5), opened by
//! the select-world screen's "Import..." button: a box for the path of a
//! vanilla Beta save folder (Bevy has no native file dialog; the path is
//! typed or pasted, `~/` expands to the home directory), Import and
//! Cancel. Import runs the shell's import ([`crate::import::import_world`]:
//! the folder is checked block id by block id and item id by item id, then
//! copied into `saves/` under an unused name) and opens the world as the
//! select-world screen opens one; a refusal shows
//! [`GuiImportRefused`](super::import_refused::GuiImportRefused) naming the
//! first offending id and its chunk, and nothing is copied.
//!
//! The copy runs inside the click (a large world stalls that frame).
//! Ctrl+V pastes the whole clipboard here (`GuiTextField`'s paste stops at
//! 32 characters, too short for a path), and the box shows the end of a
//! path too long for it ([`visible_tail`]; `GuiTextField` draws past its
//! edge). Escape returns to the select-world screen (a vanilla screen's
//! Escape closes every menu).

use std::path::PathBuf;

use super::display_parent;
use super::import_refused::GuiImportRefused;
use crate::app::ScreenKind;
use crate::gui::batch::GuiGl;
use crate::gui::draw;
use crate::gui::font::java_trim;
use crate::gui::menu::create_world::LABEL_COLOR;
use crate::gui::screen::{
    Screen, ScreenBase, ScreenCtx, default_draw_screen, default_mouse_clicked,
    draw_default_background,
};
use crate::gui::widgets::button::GuiButton;
use crate::gui::widgets::textfield::GuiTextField;

/// The box's width.
pub const FIELD_WIDTH: i32 = 300;

/// The end of `text` that fits in `width` pixels (`width(s)` measures).
pub fn visible_tail(text: &str, width: i32, w: impl Fn(&str) -> i32) -> &str {
    for (i, _) in text.char_indices() {
        if w(&text[i..]) <= width {
            return &text[i..];
        }
    }
    ""
}

/// The folder a typed path names: trimmed, a leading `~/` (or a lone `~`)
/// replaced by `home`.
pub fn resolve_path(text: &str, home: Option<&std::path::Path>) -> PathBuf {
    let t = java_trim(text);
    if let Some(h) = home {
        if t == "~" {
            return h.to_path_buf();
        }
        if let Some(rest) = t.strip_prefix("~/") {
            return h.join(rest);
        }
    }
    PathBuf::from(t)
}

/// The import screen.
pub struct GuiImportWorld {
    base: ScreenBase,
    /// The select-world screen.
    parent: Option<Box<dyn Screen>>,
    /// The path box.
    pub field: GuiTextField,
    /// The box's text across re-inits (back from the refusal dialog).
    text: String,
    /// The import ran (a second Import does nothing).
    imported: bool,
}

impl GuiImportWorld {
    /// A new import screen over `parent`.
    pub fn new(parent: Option<Box<dyn Screen>>) -> Self {
        GuiImportWorld {
            base: ScreenBase::default(),
            parent,
            field: GuiTextField::new(0, 0, 0, 0, ""),
            text: String::new(),
            imported: false,
        }
    }

    /// Import: the shell's import, then the world (as `selectWorld`), or
    /// the refusal dialog.
    fn do_import(&mut self, ctx: &mut ScreenCtx) {
        if self.imported {
            return;
        }
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let src = resolve_path(self.field.get_text(), home.as_deref());
        let data_dir = ctx.game.paths.data_dir.clone();
        match crate::import::import_world(&data_dir, &src) {
            Ok(name) => {
                self.imported = true;
                bevy::log::info!("imported {} as saves/{name}", src.display());
                // GuiSelectWorld.selectWorld's sequence (Game::open_world:
                // displayGuiScreen(null), a new PlayerControllerSP,
                // startWorld(folder, level name, 0), displayGuiScreen(null)).
                if let Err(e) = ctx.game.open_world(&name, None) {
                    bevy::log::error!("{e}");
                    ctx.game.last_error = Some(e);
                }
                ctx.sync();
            }
            Err(e) => {
                bevy::log::warn!("import of {}: {}", src.display(), e.naming_folder(&src));
                ctx.display_wrapping_this(move |this| {
                    Box::new(GuiImportRefused::new(Some(this), &e))
                });
            }
        }
    }
}

impl Screen for GuiImportWorld {
    fn base(&self) -> &ScreenBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn kind(&self) -> ScreenKind {
        ScreenKind::Other("GuiImportWorld".into())
    }

    fn update_screen(&mut self, _ctx: &mut ScreenCtx) {
        self.field.update_cursor_counter();
    }

    fn init_gui(&mut self, ctx: &mut ScreenCtx) {
        ctx.enable_repeat_events(true);
        let (w, h) = (self.base.width, self.base.height);
        let c = &mut self.base.control_list;
        c.clear();
        c.push(GuiButton::new(0, w / 2 - 100, h / 4 + 96 + 12, "Import"));
        c.push(GuiButton::new(
            1,
            w / 2 - 100,
            h / 4 + 120 + 12,
            ctx.lang.translate_key("gui.cancel"),
        ));
        c[0].enabled = !java_trim(&self.text).is_empty();
        self.field = GuiTextField::new(
            w / 2 - FIELD_WIDTH / 2,
            60,
            FIELD_WIDTH,
            20,
            self.text.clone(),
        );
        self.field.is_focused = true;
    }

    fn on_gui_closed(&mut self, ctx: &mut ScreenCtx) {
        ctx.enable_repeat_events(false);
    }

    fn action_performed(&mut self, ctx: &mut ScreenCtx, button: usize) {
        let b = &self.base.control_list[button];
        if !b.enabled {
            return;
        }
        match b.id {
            0 => self.do_import(ctx),
            1 => display_parent(ctx, &mut self.parent),
            _ => {}
        }
    }

    fn key_typed(&mut self, ctx: &mut ScreenCtx, ch: char, key: i32) {
        if ch == '\u{16}' {
            // The whole clipboard (module docs), allowed characters only.
            if let Some(s) = ctx.get_clipboard_string() {
                let allowed = &ctx.font.allowed;
                let mut t = self.field.get_text().to_string();
                t.extend(s.chars().filter(|c| allowed.is_allowed(*c)));
                self.field.set_text(t);
            }
        } else {
            let allowed = &ctx.font.allowed;
            let _ = self.field.textbox_key_typed(allowed, || None, ch, key);
        }
        self.text = self.field.get_text().to_string();
        self.base.control_list[0].enabled = !java_trim(&self.text).is_empty();
        if ch == '\r' {
            self.action_performed(ctx, 0);
        } else if key == crate::keys::KEY_ESCAPE {
            display_parent(ctx, &mut self.parent);
        }
    }

    fn mouse_clicked(&mut self, ctx: &mut ScreenCtx, x: i32, y: i32, button: i32) {
        default_mouse_clicked(self, ctx, x, y, button);
        self.field.mouse_clicked(x, y, button);
    }

    fn draw_screen(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, mx: i32, my: i32, _p: f32) {
        draw_default_background(&self.base, ctx, g);
        let (w, h) = (self.base.width, self.base.height);
        let f = ctx.font;
        let x = w / 2 - FIELD_WIDTH / 2;
        draw::draw_centered_string(g, f, "Import world", w / 2, h / 4 - 60 + 20, 16777215);
        draw::draw_string(g, f, "Folder of a Beta 1.7.3 save:", x, 47, LABEL_COLOR);
        // GuiTextField.drawTextBox's look (GuiTextField.java:84-94), with
        // the text's end kept inside the box.
        let t = &self.field;
        draw::draw_rect(
            g,
            t.x_pos - 1,
            t.y_pos - 1,
            t.x_pos + t.width + 1,
            t.y_pos + t.height + 1,
            -6250336,
        );
        draw::draw_rect(
            g,
            t.x_pos,
            t.y_pos,
            t.x_pos + t.width,
            t.y_pos + t.height,
            -16777216,
        );
        let shown = format!(
            "{}{}",
            t.get_text(),
            if t.cursor_visible() { "_" } else { "" }
        );
        let tail = visible_tail(&shown, t.width - 8, |s| f.get_string_width(s));
        draw::draw_string(
            g,
            f,
            tail,
            t.x_pos + 4,
            t.y_pos + (t.height - 8) / 2,
            14737632,
        );
        draw::draw_string(
            g,
            f,
            "The folder is copied into saves/ if every block",
            x,
            88,
            LABEL_COLOR,
        );
        draw::draw_string(
            g,
            f,
            "and item in it exists in Beta 1.7.3.",
            x,
            98,
            LABEL_COLOR,
        );
        default_draw_screen(&mut self.base, ctx, g, mx, my);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn long_paths_show_their_end() {
        let w = |s: &str| s.chars().count() as i32 * 6;
        assert_eq!(visible_tail("abc", 30, w), "abc");
        assert_eq!(visible_tail("/very/long/path", 30, w), "/path");
        assert_eq!(visible_tail("", 30, w), "");
        assert_eq!(visible_tail("abcdef", 0, w), "");
    }

    #[test]
    fn paths() {
        let home = Path::new("/home/u");
        assert_eq!(resolve_path("  /a/b ", Some(home)), PathBuf::from("/a/b"));
        assert_eq!(
            resolve_path("~/saves/w", Some(home)),
            PathBuf::from("/home/u/saves/w")
        );
        assert_eq!(resolve_path("~", Some(home)), PathBuf::from("/home/u"));
        assert_eq!(resolve_path("~x", Some(home)), PathBuf::from("~x"));
        assert_eq!(resolve_path("~/w", None), PathBuf::from("~/w"));
    }
}
