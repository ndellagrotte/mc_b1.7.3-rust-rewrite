//! The world export screens (project-owned, PLAN_v4 D5, WP-4.5), opened by
//! the select-world screen's "Export..." button for the selected world, in
//! the style of the import screen ([`super::import_world`]):
//!
//! - [`GuiExportWorld`]: "Export world", the world's name, a box for the
//!   target folder (default `<data-dir>/exports`; typed or pasted, `~/`
//!   expands to the home directory, the box shows the end of a long path),
//!   Export and Cancel. Export runs [`crate::export::export_from_game`]
//!   inside the click (a large world stalls that frame): the world is
//!   checked, copied to `<folder>/<world folder>` and read back.
//! - [`GuiExportResult`]: drawn as the import refusal dialog (after
//!   `GuiErrorScreen`): on success "World exported" with the path (wrapped
//!   at `/`, shortened by [`elide_path`] to the room above the button) and
//!   what was checked, Done back to the world list; on a refusal or failure the
//!   problem (the first offending id and its chunk, the unreadable chunk,
//!   the folder in the way, …) and "Nothing was exported.", Back to the
//!   export screen.

use std::path::{Path, PathBuf};

use mc_sim::save::import::ImportRefusal;

use super::create_world::LABEL_COLOR;
use super::display_parent;
use super::import_world::{FIELD_WIDTH, resolve_path, visible_tail};
use crate::app::ScreenKind;
use crate::export::{ExportError, ExportReport, default_export_dir, export_from_game};
use crate::gui::batch::GuiGl;
use crate::gui::draw;
use crate::gui::error_screen::GREY;
use crate::gui::font::java_trim;
use crate::gui::screen::{
    Screen, ScreenBase, ScreenCtx, default_draw_screen, default_mouse_clicked,
    draw_default_background,
};
use crate::gui::widgets::button::GuiButton;
use crate::gui::widgets::textfield::GuiTextField;

/// The result dialog's wrap width (scaled pixels), as the import's.
pub const WRAP: i32 = 300;

/// The export screen.
pub struct GuiExportWorld {
    base: ScreenBase,
    /// The select-world screen.
    parent: Option<Box<dyn Screen>>,
    /// The world's folder in `saves/`.
    pub world: String,
    /// The world's name as the list shows it.
    pub display_name: String,
    /// The folder box.
    pub field: GuiTextField,
    /// The box's text across re-inits (`None`: not set yet, the default
    /// folder is filled in by the first `initGui`).
    text: Option<String>,
}

impl GuiExportWorld {
    /// The export screen for `saves/<world>` over `parent`.
    pub fn new(parent: Option<Box<dyn Screen>>, world: String, display_name: String) -> Self {
        GuiExportWorld {
            base: ScreenBase::default(),
            parent,
            world,
            display_name,
            field: GuiTextField::new(0, 0, 0, 0, ""),
            text: None,
        }
    }

    /// Export: the shell's export, then the result dialog.
    fn do_export(&mut self, ctx: &mut ScreenCtx) {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let target = resolve_path(self.field.get_text(), home.as_deref());
        let world = self.world.clone();
        let name = self.display_name.clone();
        match export_from_game(ctx.game, &world, &target) {
            Ok(report) => {
                bevy::log::info!(
                    "exported saves/{world} to {} ({} files, {} chunks checked)",
                    report.path.display(),
                    report.files,
                    report.digest.chunks.len()
                );
                // Done goes back to the world list.
                let parent = self.parent.take();
                ctx.display_gui_screen(Some(Box::new(GuiExportResult::success(
                    parent, &name, &report,
                ))));
            }
            Err(e) => {
                bevy::log::warn!("export of saves/{world} to {}: {e}", target.display());
                ctx.display_wrapping_this(move |this| {
                    Box::new(GuiExportResult::failure(Some(this), &e))
                });
            }
        }
    }
}

impl Screen for GuiExportWorld {
    fn base(&self) -> &ScreenBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn kind(&self) -> ScreenKind {
        ScreenKind::Other("GuiExportWorld".into())
    }

    fn update_screen(&mut self, _ctx: &mut ScreenCtx) {
        self.field.update_cursor_counter();
    }

    fn init_gui(&mut self, ctx: &mut ScreenCtx) {
        ctx.enable_repeat_events(true);
        let text = self
            .text
            .get_or_insert_with(|| {
                default_export_dir(&ctx.game.paths.data_dir)
                    .display()
                    .to_string()
            })
            .clone();
        let (w, h) = (self.base.width, self.base.height);
        let c = &mut self.base.control_list;
        c.clear();
        c.push(GuiButton::new(0, w / 2 - 100, h / 4 + 96 + 12, "Export"));
        c.push(GuiButton::new(
            1,
            w / 2 - 100,
            h / 4 + 120 + 12,
            ctx.lang.translate_key("gui.cancel"),
        ));
        c[0].enabled = !java_trim(&text).is_empty();
        self.field = GuiTextField::new(w / 2 - FIELD_WIDTH / 2, 60, FIELD_WIDTH, 20, text);
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
            0 => self.do_export(ctx),
            1 => display_parent(ctx, &mut self.parent),
            _ => {}
        }
    }

    fn key_typed(&mut self, ctx: &mut ScreenCtx, ch: char, key: i32) {
        if ch == '\u{16}' {
            // The whole clipboard, as the import screen pastes.
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
        let text = self.field.get_text().to_string();
        self.base.control_list[0].enabled = !java_trim(&text).is_empty();
        self.text = Some(text);
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
        draw::draw_centered_string(g, f, "Export world", w / 2, h / 4 - 60 + 20, 16777215);
        let label = format!("Export '{}' to the folder:", self.display_name);
        draw::draw_string(
            g,
            f,
            visible_tail(&label, FIELD_WIDTH, |s| f.get_string_width(s)),
            x,
            47,
            LABEL_COLOR,
        );
        // The import box's look (GuiTextField.drawTextBox, GuiTextField.java:84-94).
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
        let into = format!("The world is checked, copied into <folder>/{}", self.world);
        draw::draw_string(
            g,
            f,
            visible_tail(&into, FIELD_WIDTH, |s| f.get_string_width(s)),
            x,
            88,
            LABEL_COLOR,
        );
        draw::draw_string(
            g,
            f,
            "and read back. A folder that is not empty is",
            x,
            98,
            LABEL_COLOR,
        );
        draw::draw_string(g, f, "never overwritten.", x, 108, LABEL_COLOR);
        default_draw_screen(&mut self.base, ctx, g, mx, my);
    }
}

/// One line of the result dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DialogLine {
    /// Text, word-wrapped to [`WRAP`].
    Text(String),
    /// A path, broken at `/` (or anywhere) to fit [`WRAP`].
    Path(String),
    /// An empty line (9 pixels).
    Gap,
}

/// `text` cut into lines no wider than `width` (`w` measures), each break
/// after the last `/` that fits, else where the width runs out.
pub fn wrap_path(text: &str, width: i32, w: impl Fn(&str) -> i32) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        if w(rest) <= width {
            out.push(rest.to_string());
            break;
        }
        // The longest prefix that fits (at least one character).
        let mut fit = rest.chars().next().map_or(rest.len(), char::len_utf8);
        for (i, ch) in rest.char_indices() {
            let end = i + ch.len_utf8();
            if w(&rest[..end]) > width {
                break;
            }
            fit = end;
        }
        let cut = match rest[..fit].rfind('/') {
            Some(i) if i > 0 => i + 1,
            _ => fit,
        };
        out.push(rest[..cut].to_string());
        rest = &rest[cut..];
    }
    out
}

/// The text's room in the result dialog (scaled pixels): from its first line
/// at `height / 4 - 10` to 4 pixels above the button at `height / 4 + 132`.
pub const TEXT_ROOM: i32 = (120 + 12 - 4) - (-60 + 50);

/// The pieces of a wrapped path cut to at most `max` lines (at least 3):
/// the first piece, `.../` (the font has no ellipsis), then the last
/// `max - 2` pieces, so a long path keeps its start and the world folder and
/// the text stays above the button.
pub fn elide_path(pieces: Vec<String>, max: usize) -> Vec<String> {
    let max = max.max(3);
    if pieces.len() <= max {
        return pieces;
    }
    let mut out = vec![pieces[0].clone(), ".../".to_string()];
    out.extend_from_slice(&pieces[pieces.len() - (max - 2)..]);
    out
}

/// The dialog's title and lines for a successful export.
pub fn success_text(name: &str, r: &ExportReport) -> (String, Vec<DialogLine>) {
    let mut lines = vec![
        DialogLine::Text(format!("'{name}' was exported to:")),
        DialogLine::Path(r.path.display().to_string()),
        DialogLine::Gap,
        DialogLine::Text(format!(
            "{} files, {} region files and {} chunks were read back and match the world.",
            r.files,
            r.digest.regions.len(),
            r.digest.chunks.len()
        )),
    ];
    if let Some([x, y, z]) = r.digest.player_pos {
        lines.push(DialogLine::Text(format!(
            "The player is at {x:.1}, {y:.1}, {z:.1}."
        )));
    }
    lines.push(DialogLine::Gap);
    lines.push(DialogLine::Text(
        "To play it in Beta 1.7.3, copy the folder into .minecraft/saves.".to_string(),
    ));
    ("World exported".to_string(), lines)
}

/// The dialog's title and lines for a refused or failed export.
pub fn failure_text(e: &ExportError) -> (String, Vec<DialogLine>) {
    let (title, mut lines) = match e {
        ExportError::Refused(r) => (
            "Export refused",
            match r {
                ImportRefusal::UnknownBlock {
                    id,
                    location,
                    detail,
                } => vec![
                    DialogLine::Text(format!("Block id {id} is not a Beta 1.7.3 block.")),
                    DialogLine::Text(format!("First found in {location} ({detail}).")),
                ],
                ImportRefusal::UnknownItem {
                    id,
                    location,
                    detail,
                } => vec![
                    DialogLine::Text(format!("Item id {id} is not a Beta 1.7.3 item.")),
                    DialogLine::Text(format!("First found in {location} ({detail}).")),
                ],
                ImportRefusal::Unreadable { path, reason } => vec![DialogLine::Text(format!(
                    "Cannot read {}: {reason}",
                    crate::export::rel(path)
                ))],
            },
        ),
        ExportError::Corrupt { path, reason } => (
            "Export refused",
            vec![DialogLine::Text(format!(
                "Cannot read {}: {reason}",
                crate::export::rel(path)
            ))],
        ),
        ExportError::TargetNotEmpty(p) => (
            "Export refused",
            vec![
                DialogLine::Text("The folder".to_string()),
                DialogLine::Path(p.display().to_string()),
                DialogLine::Text(
                    "already exists and is not empty (or is inside the world). Choose another folder."
                        .to_string(),
                ),
            ],
        ),
        ExportError::NoWorld(m) => ("Export refused", vec![DialogLine::Text(format!("No such world: {m}"))]),
        ExportError::Save(_)
        | ExportError::Read(_)
        | ExportError::Write(_)
        | ExportError::Mismatch(_) => (
            "Export failed",
            vec![DialogLine::Text(capitalise(&e.to_string()))],
        ),
    };
    lines.push(DialogLine::Gap);
    lines.push(DialogLine::Text("Nothing was exported.".to_string()));
    (title.to_string(), lines)
}

fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

/// The export's result dialog (module docs).
pub struct GuiExportResult {
    base: ScreenBase,
    /// Where the button returns: the world list after a success, the export
    /// screen after a refusal.
    parent: Option<Box<dyn Screen>>,
    /// The title.
    pub title: String,
    /// The lines.
    pub lines: Vec<DialogLine>,
    /// The button's label (`Done` or `Back`).
    pub button: &'static str,
}

impl GuiExportResult {
    /// The success dialog.
    pub fn success(parent: Option<Box<dyn Screen>>, name: &str, r: &ExportReport) -> Self {
        let (title, lines) = success_text(name, r);
        GuiExportResult {
            base: ScreenBase::default(),
            parent,
            title,
            lines,
            button: "Done",
        }
    }

    /// The refusal dialog.
    pub fn failure(parent: Option<Box<dyn Screen>>, e: &ExportError) -> Self {
        let (title, lines) = failure_text(e);
        GuiExportResult {
            base: ScreenBase::default(),
            parent,
            title,
            lines,
            button: "Back",
        }
    }

    /// The path the success dialog names (tests, the dev action).
    pub fn path(&self) -> Option<&Path> {
        self.lines.iter().find_map(|l| match l {
            DialogLine::Path(p) => Some(Path::new(p.as_str())),
            _ => None,
        })
    }
}

impl Screen for GuiExportResult {
    fn base(&self) -> &ScreenBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn kind(&self) -> ScreenKind {
        ScreenKind::Other("GuiExportResult".into())
    }

    fn init_gui(&mut self, _ctx: &mut ScreenCtx) {
        let (w, h) = (self.base.width, self.base.height);
        self.base.control_list.clear();
        self.base.control_list.push(GuiButton::new(
            0,
            w / 2 - 100,
            h / 4 + 120 + 12,
            self.button,
        ));
    }

    fn action_performed(&mut self, ctx: &mut ScreenCtx, button: usize) {
        if self.base.control_list[button].id == 0 {
            display_parent(ctx, &mut self.parent);
        }
    }

    fn key_typed(&mut self, ctx: &mut ScreenCtx, _ch: char, key: i32) {
        if key == crate::keys::KEY_ESCAPE {
            display_parent(ctx, &mut self.parent);
        }
    }

    /// The import refusal dialog's geometry: the title centred at
    /// `height / 4 - 40`, the text from `height / 4 - 10` at
    /// `width / 2 - 150`, wrapped to [`WRAP`].
    // src: GuiErrorScreen.java:19-30 (the style followed)
    fn draw_screen(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, mx: i32, my: i32, _p: f32) {
        draw_default_background(&self.base, ctx, g);
        let (w, h) = (self.base.width, self.base.height);
        let f = ctx.font;
        draw::draw_centered_string(g, f, &self.title, w / 2, h / 4 - 60 + 20, 16777215);
        let x = w / 2 - WRAP / 2;
        let mut y = h / 4 - 60 + 50;
        // The path gets the lines the other text leaves above the button.
        let other: i32 = self
            .lines
            .iter()
            .map(|l| match l {
                DialogLine::Gap => 9,
                DialogLine::Text(t) => f.split_string_width(t, WRAP) + 1,
                DialogLine::Path(_) => 0,
            })
            .sum();
        let path_lines = ((TEXT_ROOM - other) / 9).max(0) as usize;
        for l in &self.lines {
            match l {
                DialogLine::Gap => y += 9,
                DialogLine::Text(t) => {
                    f.draw_split_string(g, t, x, y, WRAP, GREY);
                    y += f.split_string_width(t, WRAP) + 1;
                }
                DialogLine::Path(p) => {
                    let pieces = wrap_path(p, WRAP, |s| f.get_string_width(s));
                    for piece in elide_path(pieces, path_lines) {
                        draw::draw_string(g, f, &piece, x, y, 16777215);
                        y += 9;
                    }
                }
            }
        }
        default_draw_screen(&mut self.base, ctx, g, mx, my);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::WorldDigest;
    use mc_sim::save::import::Location;

    #[test]
    fn paths_break_after_slashes() {
        let w = |s: &str| s.chars().count() as i32 * 6;
        assert_eq!(wrap_path("/a/b", 60, w), vec!["/a/b"]);
        assert_eq!(
            wrap_path("/home/user/exports/World", 60, w),
            vec!["/home/", "user/", "exports/", "World"]
        );
        // No slash that fits: cut where the width runs out.
        assert_eq!(
            wrap_path("abcdefghijkl", 30, w),
            vec!["abcde", "fghij", "kl"]
        );
        assert_eq!(wrap_path("", 30, w), Vec::<String>::new());
        // A character wider than the box still makes progress.
        assert_eq!(wrap_path("ab", 3, w), vec!["a", "b"]);
    }

    #[test]
    fn long_paths_keep_their_start_and_end() {
        let p = |n: usize| (0..n).map(|i| format!("p{i}/")).collect::<Vec<_>>();
        assert_eq!(elide_path(p(4), 4), p(4));
        assert_eq!(elide_path(p(6), 4), vec!["p0/", ".../", "p4/", "p5/"]);
        // Never fewer than three lines: the start, the ellipsis, the end.
        assert_eq!(elide_path(p(6), 0), vec!["p0/", ".../", "p5/"]);
        assert_eq!(TEXT_ROOM, 138);
    }

    fn report() -> ExportReport {
        ExportReport {
            path: PathBuf::from("/x/exports/W"),
            files: 7,
            bytes: 100,
            digest: WorldDigest {
                level_name: "W".into(),
                seed: 123,
                time: 1,
                player_pos: Some([1.0, 66.62, -2.25]),
                player_dimension: 0,
                level_dat: String::new(),
                files: vec![],
                regions: vec!["region/r.0.0.mcr".into()],
                chunks: vec![],
            },
        }
    }

    #[test]
    fn the_success_dialog_names_the_path() {
        let (t, l) = success_text("My World", &report());
        assert_eq!(t, "World exported");
        assert_eq!(l[0], DialogLine::Text("'My World' was exported to:".into()));
        assert_eq!(l[1], DialogLine::Path("/x/exports/W".into()));
        assert_eq!(
            l[3],
            DialogLine::Text(
                "7 files, 1 region files and 0 chunks were read back and match the world.".into()
            )
        );
        assert_eq!(
            l[4],
            DialogLine::Text("The player is at 1.0, 66.6, -2.2.".into())
        );
        let d = GuiExportResult::success(None, "My World", &report());
        assert_eq!(d.path(), Some(Path::new("/x/exports/W")));
        assert_eq!(d.button, "Done");
    }

    #[test]
    fn refusals_name_the_problem() {
        let e = ExportError::Refused(ImportRefusal::UnknownBlock {
            id: 200,
            location: Location::Chunk {
                dimension: 0,
                x: 0,
                z: -4,
            },
            detail: "Blocks at local x 5, y 70, z 5".into(),
        });
        let (t, l) = failure_text(&e);
        assert_eq!(t, "Export refused");
        assert_eq!(
            l[0],
            DialogLine::Text("Block id 200 is not a Beta 1.7.3 block.".into())
        );
        assert_eq!(
            l[1],
            DialogLine::Text(
                "First found in chunk (0, -4) (Blocks at local x 5, y 70, z 5).".into()
            )
        );
        assert_eq!(
            l.last(),
            Some(&DialogLine::Text("Nothing was exported.".into()))
        );
        let (_, l) = failure_text(&ExportError::Corrupt {
            path: "world/region/r.0.0.mcr".into(),
            reason: "chunk (1, 0): listed in the header but unreadable".into(),
        });
        assert_eq!(
            l[0],
            DialogLine::Text(
                "Cannot read region/r.0.0.mcr: chunk (1, 0): listed in the header but unreadable"
                    .into()
            )
        );
        let (_, l) = failure_text(&ExportError::TargetNotEmpty(PathBuf::from("/x/W")));
        assert_eq!(l[1], DialogLine::Path("/x/W".into()));
        let (t, l) = failure_text(&ExportError::Write("/x: denied".into()));
        assert_eq!(t, "Export failed");
        assert_eq!(
            l[0],
            DialogLine::Text("Cannot write the copy: /x: denied".into())
        );
        let d = GuiExportResult::failure(None, &ExportError::NoWorld("W".into()));
        assert_eq!(d.button, "Back");
        assert_eq!(d.path(), None);
    }
}
