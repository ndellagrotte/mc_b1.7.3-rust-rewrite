//! Dev screens (project-owned, not vanilla) for the `debug_gui` script
//! action: every widget kind on one screen ([`DebugWidgets`]), a
//! `GuiSlot` list laid out as the world list ([`DebugList`]), every colour
//! code and glyph of the font ([`DebugFont`]), and a `GuiYesNo` over the
//! widget screen (`DebugYesNo`). Layouts follow the vanilla screens they
//! imitate (`GuiSelectWorld`'s 36-pixel rows between 32 and `height - 64`,
//! `GuiCreateWorld`'s 200 × 20 text field).

use crate::app::ScreenKind;
use crate::gui::batch::GuiGl;
use crate::gui::draw;
use crate::gui::screen::{
    Screen, ScreenBase, ScreenCtx, ScreenRegistry, default_draw_screen, default_key_typed,
    default_mouse_clicked, draw_default_background,
};
use crate::gui::widgets::button::GuiButton;
use crate::gui::widgets::slot::{GuiSlot, SlotList};
use crate::gui::widgets::textfield::GuiTextField;
use crate::gui::widgets::yesno::GuiYesNo;
use crate::settings::EnumOptions;

/// Registers the dev screens under their names.
pub fn register(r: &mut ScreenRegistry) {
    r.register("DebugWidgets", || Box::new(DebugWidgets::default()));
    r.register("DebugList", || Box::new(DebugList::default()));
    r.register("DebugFont", || Box::new(DebugFont::default()));
    r.register("DebugYesNo", || {
        Box::new(yes_no(Box::new(DebugWidgets::default())))
    });
}

/// The registry name of a `debug_gui` screen (`"widgets"`, `"list"`,
/// `"font"`, `"yesno"`, or a registered name).
pub fn debug_screen_name(s: &str) -> String {
    match s {
        "widgets" => "DebugWidgets",
        "list" => "DebugList",
        "font" => "DebugFont",
        "yesno" => "DebugYesNo",
        other => other,
    }
    .to_string()
}

fn yes_no(parent: Box<dyn Screen>) -> GuiYesNo {
    GuiYesNo::new(
        parent,
        "Are you sure you want to test this?",
        "'GuiYesNo' answers to the screen that opened it.",
        "Yes",
        "No",
        7,
    )
}

/// Buttons (normal, under the pointer, disabled), two small buttons, a
/// slider and a focused text field over the screen background.
#[derive(Default)]
pub struct DebugWidgets {
    base: ScreenBase,
    field: Option<GuiTextField>,
    answer: Option<(bool, i32)>,
}

impl Screen for DebugWidgets {
    fn base(&self) -> &ScreenBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn kind(&self) -> ScreenKind {
        ScreenKind::Other("DebugWidgets".into())
    }
    fn init_gui(&mut self, ctx: &mut ScreenCtx) {
        let (w, h) = (self.base.width, self.base.height);
        let c = &mut self.base.control_list;
        c.push(GuiButton::new(0, w / 2 - 100, h / 4 + 24, "Normal button"));
        c.push(GuiButton::new(1, w / 2 - 100, h / 4 + 48, "Hovered button"));
        let mut d = GuiButton::new(2, w / 2 - 100, h / 4 + 72, "Disabled button");
        d.enabled = false;
        c.push(d);
        c.push(GuiButton::small(3, w / 2 - 155, h / 4 + 96, "Small button"));
        c.push(GuiButton::small(4, w / 2 + 5, h / 4 + 96, "Open GuiYesNo"));
        let s = &ctx.game.game_settings;
        let label = s.get_key_binding(EnumOptions::Sensitivity, ctx.lang);
        let v = s.get_option_float_value(EnumOptions::Sensitivity);
        c.push(GuiButton::slider(
            5,
            w / 2 - 155,
            h / 4 + 120,
            EnumOptions::Sensitivity,
            label,
            v,
        ));
        c.push(GuiButton::small(6, w / 2 + 5, h / 4 + 120, "Open GuiSlot"));
        let mut f = GuiTextField::new(w / 2 - 100, h / 4 - 4, 200, 20, "Hello, world");
        f.set_max_string_length(32);
        f.set_focused(true);
        self.field = Some(f);
    }
    fn update_screen(&mut self, _ctx: &mut ScreenCtx) {
        if let Some(f) = self.field.as_mut() {
            f.update_cursor_counter();
        }
    }
    fn key_typed(&mut self, ctx: &mut ScreenCtx, ch: char, key: i32) {
        if let Some(f) = self.field.as_mut() {
            let allowed = &ctx.font.allowed;
            let mut cb = None;
            if ch == '\u{16}' {
                cb = ctx.get_clipboard_string();
            }
            f.textbox_key_typed(allowed, || cb, ch, key);
        }
        default_key_typed(ctx, key);
    }
    fn mouse_clicked(&mut self, ctx: &mut ScreenCtx, x: i32, y: i32, button: i32) {
        default_mouse_clicked(self, ctx, x, y, button);
        if let Some(f) = self.field.as_mut() {
            f.mouse_clicked(x, y, button);
        }
    }
    fn action_performed(&mut self, ctx: &mut ScreenCtx, button: usize) {
        match self.base.control_list[button].id {
            4 => ctx.display_wrapping_this(|this| Box::new(yes_no(this))),
            6 => ctx.display_gui_screen(Some(Box::new(DebugList::default()))),
            _ => {}
        }
    }
    fn delete_world(&mut self, ctx: &mut ScreenCtx, result: bool, id: i32) {
        self.answer = Some((result, id));
        ctx.display_this();
    }
    fn draw_screen(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, mx: i32, my: i32, _p: f32) {
        draw_default_background(&self.base, ctx, g);
        let w = self.base.width;
        let f = ctx.font;
        draw::draw_centered_string(g, f, "GUI widget test (WP-3.6a)", w / 2, 20, 16777215);
        if let Some((a, id)) = self.answer {
            let s = format!(
                "GuiYesNo answered {} (id {id})",
                if a { "yes" } else { "no" }
            );
            draw::draw_centered_string(g, f, &s, w / 2, 34, 10526880);
        }
        if let Some(t) = self.field.as_ref() {
            t.draw_text_box(g, f);
        }
        default_draw_screen(&mut self.base, ctx, g, mx, my);
    }
}

/// The rows of [`DebugList`].
struct Rows {
    selected: i32,
    count: i32,
    width: i32,
    height: i32,
}

impl SlotList for Rows {
    fn get_size(&self) -> i32 {
        self.count
    }
    fn element_clicked(&mut self, _ctx: &mut ScreenCtx, index: i32, _double: bool) {
        self.selected = index;
    }
    fn is_selected(&self, index: i32) -> bool {
        index == self.selected
    }
    fn draw_background(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl) {
        let b = ScreenBase {
            width: self.width,
            height: self.height,
            ..ScreenBase::default()
        };
        draw_default_background(&b, ctx, g);
    }
    fn draw_slot(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, i: i32, x: i32, y: i32, _h: i32) {
        // GuiWorldSlot's layout (GuiWorldSlot.java:38-50).
        let f = ctx.font;
        draw::draw_string(
            g,
            f,
            &format!("World number {}", i + 1),
            x + 2,
            y + 1,
            16777215,
        );
        draw::draw_string(
            g,
            f,
            &format!("world{} (a GuiSlot row)", i + 1),
            x + 2,
            y + 12,
            8421504,
        );
        draw::draw_string(g, f, "Survival Mode", x + 2, y + 12 + 10, 8421504);
    }
}

/// A `GuiSlot` of 30 rows, scrolled a little, with a selected row.
pub struct DebugList {
    base: ScreenBase,
    slot: Option<GuiSlot>,
    rows: Rows,
}

impl Default for DebugList {
    fn default() -> Self {
        DebugList {
            base: ScreenBase::default(),
            slot: None,
            rows: Rows {
                selected: 2,
                count: 30,
                width: 0,
                height: 0,
            },
        }
    }
}

impl Screen for DebugList {
    fn base(&self) -> &ScreenBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn kind(&self) -> ScreenKind {
        ScreenKind::Other("DebugList".into())
    }
    fn init_gui(&mut self, _ctx: &mut ScreenCtx) {
        let (w, h) = (self.base.width, self.base.height);
        let mut s = GuiSlot::new(w, h, 32, h - 64, 36);
        s.amount_scrolled = 40.0;
        self.slot = Some(s);
        self.rows.width = w;
        self.rows.height = h;
        self.base.control_list.push(GuiButton::new(
            0,
            w / 2 - 100,
            h - 48,
            "Back to the widget test",
        ));
    }
    fn action_performed(&mut self, ctx: &mut ScreenCtx, button: usize) {
        if self.base.control_list[button].id == 0 {
            ctx.display_gui_screen(Some(Box::new(DebugWidgets::default())));
        }
    }
    fn draw_screen(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, mx: i32, my: i32, p: f32) {
        if let Some(s) = self.slot.as_mut() {
            s.draw_screen(&mut self.rows, ctx, g, mx, my, p);
        }
        let w = self.base.width;
        draw::draw_centered_string(g, ctx.font, "GuiSlot test (WP-3.6a)", w / 2, 20, 16777215);
        default_draw_screen(&mut self.base, ctx, g, mx, my);
    }
}

/// Every colour code with its shadow, and every glyph of the font.
#[derive(Default)]
pub struct DebugFont {
    base: ScreenBase,
}

impl Screen for DebugFont {
    fn base(&self) -> &ScreenBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn kind(&self) -> ScreenKind {
        ScreenKind::Other("DebugFont".into())
    }
    fn draw_screen(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, mx: i32, my: i32, _p: f32) {
        draw_default_background(&self.base, ctx, g);
        let (w, h) = (self.base.width, self.base.height);
        draw::draw_rect(g, 4, 2, w - 4, h - 2, -1610612736);
        let f = ctx.font;
        draw::draw_centered_string(g, f, "FontRenderer test (WP-3.6a)", w / 2, 5, 16777215);
        let hex = "0123456789abcdef";
        for (i, c) in hex.chars().enumerate() {
            let s = format!("\u{a7}{c}\u{a7}{c} {c}: The quick brown fox jumps over the lazy dog.");
            let x = 8;
            let y = 16 + i as i32 * 10;
            draw::draw_string(g, f, &s, x, y, 16777215);
        }
        // The glyphs, 32 to a line.
        let chars: Vec<char> = f.allowed.chars.clone();
        for (n, line) in chars.chunks(32).enumerate() {
            let s: String = line.iter().collect();
            draw::draw_string(g, f, &s, 8, 180 + n as i32 * 10, 14737632);
        }
        default_draw_screen(&mut self.base, ctx, g, mx, my);
    }
}
