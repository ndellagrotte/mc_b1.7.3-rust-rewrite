//! The import's refusal dialog (project-owned, PLAN_v4 D5): a world
//! holding a block or item id Beta 1.7.3 does not define is refused with a
//! message naming the first offending id and where it is (the chunk, or
//! `level.dat` for the player's inventory); nothing was copied. Drawn as
//! `GuiErrorScreen` draws its message (the dirt or dimmed background, a
//! white title, grey lines), with a Back button to the import screen.

use mc_sim::save::import::ImportRefusal;

use super::display_parent;
use crate::app::ScreenKind;
use crate::gui::batch::GuiGl;
use crate::gui::draw;
use crate::gui::error_screen::GREY;
use crate::gui::screen::{
    Screen, ScreenBase, ScreenCtx, default_draw_screen, draw_default_background,
};
use crate::gui::widgets::button::GuiButton;
use crate::import::ImportError;

/// The message's wrap width (scaled pixels).
pub const WRAP: i32 = 300;

/// The dialog's title and lines for an import error.
pub fn refusal_text(e: &ImportError) -> (String, Vec<String>) {
    match e {
        ImportError::Refused(r) => {
            let mut lines = match r {
                ImportRefusal::UnknownBlock {
                    id,
                    location,
                    detail,
                } => vec![
                    format!("Block id {id} is not a Beta 1.7.3 block."),
                    format!("First found in {location} ({detail})."),
                ],
                ImportRefusal::UnknownItem {
                    id,
                    location,
                    detail,
                } => vec![
                    format!("Item id {id} is not a Beta 1.7.3 item."),
                    format!("First found in {location} ({detail})."),
                ],
                ImportRefusal::Unreadable { path, reason } => {
                    // The scan reads a staged copy under "import/".
                    let p = path.strip_prefix("import/").unwrap_or(path);
                    vec![format!("Cannot read {p}: {reason}")]
                }
            };
            lines.push(String::new());
            lines.push("The world was not imported; nothing was copied.".to_string());
            ("World refused".to_string(), lines)
        }
        ImportError::Read(m) => (
            "Cannot import world".to_string(),
            vec![
                format!("Cannot read the save folder: {m}"),
                String::new(),
                "Nothing was copied.".to_string(),
            ],
        ),
        ImportError::Write(m) => (
            "Cannot import world".to_string(),
            vec![format!("Cannot write the imported world: {m}")],
        ),
    }
}

/// The refusal dialog.
pub struct GuiImportRefused {
    base: ScreenBase,
    /// The import screen (Back returns to it).
    parent: Option<Box<dyn Screen>>,
    /// The title.
    pub title: String,
    /// The message lines (each word-wrapped to [`WRAP`]).
    pub lines: Vec<String>,
}

impl GuiImportRefused {
    /// A dialog for `e` over `parent`.
    pub fn new(parent: Option<Box<dyn Screen>>, e: &ImportError) -> Self {
        let (title, lines) = refusal_text(e);
        GuiImportRefused {
            base: ScreenBase::default(),
            parent,
            title,
            lines,
        }
    }
}

impl Screen for GuiImportRefused {
    fn base(&self) -> &ScreenBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn kind(&self) -> ScreenKind {
        ScreenKind::Other("GuiImportRefused".into())
    }

    fn init_gui(&mut self, _ctx: &mut ScreenCtx) {
        let (w, h) = (self.base.width, self.base.height);
        self.base
            .control_list
            .push(GuiButton::new(0, w / 2 - 100, h / 4 + 120 + 12, "Back"));
    }

    fn action_performed(&mut self, ctx: &mut ScreenCtx, button: usize) {
        if self.base.control_list[button].id == 0 {
            display_parent(ctx, &mut self.parent);
        }
    }

    /// After `GuiErrorScreen.drawScreen` (a centred white title, grey text
    /// below), with this dialog's own geometry: the title centred at
    /// `height / 4 - 40`, the text from `height / 4 - 10` at
    /// `width / 2 - 150`, each line wrapped to [`WRAP`] (300) and an empty
    /// line skipping 9 pixels. (`GuiErrorScreen` itself draws its text at
    /// `width / 2 - 140` from `height / 4`.)
    // src: GuiErrorScreen.java:19-30 (the style followed)
    fn draw_screen(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, mx: i32, my: i32, _p: f32) {
        draw_default_background(&self.base, ctx, g);
        let (w, h) = (self.base.width, self.base.height);
        let f = ctx.font;
        draw::draw_centered_string(g, f, &self.title, w / 2, h / 4 - 60 + 20, 16777215);
        let x = w / 2 - WRAP / 2;
        let mut y = h / 4 - 60 + 50;
        for l in &self.lines {
            if l.is_empty() {
                y += 9;
                continue;
            }
            f.draw_split_string(g, l, x, y, WRAP, GREY);
            y += f.split_string_width(l, WRAP) + 1;
        }
        default_draw_screen(&mut self.base, ctx, g, mx, my);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mc_sim::save::import::Location;

    #[test]
    fn names_the_id_and_the_chunk() {
        let e = ImportError::Refused(ImportRefusal::UnknownBlock {
            id: 213,
            location: Location::Chunk {
                dimension: 0,
                x: -3,
                z: 7,
            },
            detail: "Blocks at (2, 64, 5)".into(),
        });
        let (t, l) = refusal_text(&e);
        assert_eq!(t, "World refused");
        assert_eq!(l[0], "Block id 213 is not a Beta 1.7.3 block.");
        assert_eq!(l[1], "First found in chunk (-3, 7) (Blocks at (2, 64, 5)).");
        assert_eq!(
            l.last().unwrap(),
            "The world was not imported; nothing was copied."
        );
        let e = ImportError::Refused(ImportRefusal::UnknownItem {
            id: 4000,
            location: Location::LevelDat,
            detail: "Player Inventory slot 3".into(),
        });
        let (_, l) = refusal_text(&e);
        assert_eq!(l[0], "Item id 4000 is not a Beta 1.7.3 item.");
        assert_eq!(l[1], "First found in level.dat (Player Inventory slot 3).");
    }
}
