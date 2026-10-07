//! `GuiTextField` (GuiTextField.java): a one-line text box (the world name
//! and seed of the create-world screen, the rename box). Java's back
//! pointer to the parent screen (for Tab's `selectNextField`) becomes the
//! return value of [`GuiTextField::textbox_key_typed`]; the parent calls
//! its own `select_next_field`. Nothing in the method reads the parent's
//! state after that call, so the order is unchanged.

use crate::gui::batch::GuiGl;
use crate::gui::draw;
use crate::gui::font::{ChatAllowedCharacters, FontRenderer};

/// The box's border colour (`-6250336`, `0xffa0a0a0`).
// src: GuiTextField.java:85
const BORDER: i32 = -6250336;
/// The box's fill (`-16777216`, opaque black).
// src: GuiTextField.java:86
const FILL: i32 = -16777216;
/// The text colour when enabled (`14737632`, `0xe0e0e0`).
// src: GuiTextField.java:89
const TEXT: i32 = 14737632;
/// The text colour when disabled (`7368816`, `0x707070`).
// src: GuiTextField.java:91
const TEXT_DISABLED: i32 = 7368816;
/// LWJGL `KEY_BACK` (backspace).
const KEY_BACK: i32 = 14;

/// `GuiTextField` (module docs).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuiTextField {
    /// `xPos`.
    pub x_pos: i32,
    /// `yPos`.
    pub y_pos: i32,
    /// `width`.
    pub width: i32,
    /// `height`.
    pub height: i32,
    /// `text`.
    text: String,
    /// `maxStringLength` (0: no limit).
    max_string_length: i32,
    /// `cursorCounter`: ticks, for the blink.
    cursor_counter: i32,
    /// `isFocused`.
    pub is_focused: bool,
    /// `isEnabled`.
    pub is_enabled: bool,
}

impl GuiTextField {
    /// `new GuiTextField(parent, font, x, y, width, height, text)`.
    // src: GuiTextField.java:16-24
    pub fn new(x: i32, y: i32, w: i32, h: i32, text: impl Into<String>) -> Self {
        GuiTextField {
            x_pos: x,
            y_pos: y,
            width: w,
            height: h,
            text: text.into(),
            max_string_length: 0,
            cursor_counter: 0,
            is_focused: false,
            is_enabled: true,
        }
    }

    /// `setText(s)`.
    pub fn set_text(&mut self, s: impl Into<String>) {
        self.text = s.into();
    }

    /// `getText()`.
    pub fn get_text(&self) -> &str {
        &self.text
    }

    /// `updateCursorCounter()` (each `updateScreen`).
    // src: GuiTextField.java:34-36
    pub fn update_cursor_counter(&mut self) {
        self.cursor_counter = self.cursor_counter.wrapping_add(1);
    }

    /// `textboxKeyTyped(char, key)`: Tab asks for the next field (returned
    /// true: the caller runs `selectNextField`); Ctrl+V (char 22) pastes;
    /// Backspace deletes the last character; an allowed character is
    /// appended while the text is shorter than the maximum (0: no limit).
    // src: GuiTextField.java:38-69
    pub fn textbox_key_typed(
        &mut self,
        allowed: &ChatAllowedCharacters,
        clipboard: impl FnOnce() -> Option<String>,
        var1: char,
        var2: i32,
    ) -> bool {
        let mut next_field = false;
        if self.is_enabled && self.is_focused {
            if var1 == '\t' {
                next_field = true;
            }
            if var1 == '\u{16}' {
                let var3: Vec<char> = clipboard().unwrap_or_default().chars().collect();
                // vanilla bug: the paste is capped at 32 characters whatever
                // `maxStringLength` is, and is not filtered.
                let mut var4 = 32 - self.len();
                if var4 > var3.len() as i32 {
                    var4 = var3.len() as i32;
                }
                if var4 > 0 {
                    self.text.extend(&var3[..var4 as usize]);
                }
            }
            if var2 == KEY_BACK && self.len() > 0 {
                self.text.pop();
            }
            if allowed.is_allowed(var1)
                && (self.len() < self.max_string_length || self.max_string_length == 0)
            {
                self.text.push(var1);
            }
        }
        next_field
    }

    fn len(&self) -> i32 {
        self.text.chars().count() as i32
    }

    /// `mouseClicked(x, y, button)`: focused when clicked inside.
    // src: GuiTextField.java:71-74
    pub fn mouse_clicked(&mut self, var1: i32, var2: i32, _var3: i32) {
        let var4 = self.is_enabled
            && var1 >= self.x_pos
            && var1 < self.x_pos + self.width
            && var2 >= self.y_pos
            && var2 < self.y_pos + self.height;
        self.set_focused(var4);
    }

    /// `setFocused(f)`: gaining the focus restarts the blink.
    // src: GuiTextField.java:76-82
    pub fn set_focused(&mut self, var1: bool) {
        if var1 && !self.is_focused {
            self.cursor_counter = 0;
        }
        self.is_focused = var1;
    }

    /// Whether the `_` cursor shows: focused, and in the first 6 of every
    /// 12 ticks.
    // src: GuiTextField.java:88
    pub fn cursor_visible(&self) -> bool {
        self.is_focused && self.cursor_counter / 6 % 2 == 0
    }

    /// `drawTextBox()`: a grey border, black inside, the text 4 pixels in
    /// (with the cursor).
    // src: GuiTextField.java:84-94
    pub fn draw_text_box(&self, g: &mut GuiGl, font: &FontRenderer) {
        draw::draw_rect(
            g,
            self.x_pos - 1,
            self.y_pos - 1,
            self.x_pos + self.width + 1,
            self.y_pos + self.height + 1,
            BORDER,
        );
        draw::draw_rect(
            g,
            self.x_pos,
            self.y_pos,
            self.x_pos + self.width,
            self.y_pos + self.height,
            FILL,
        );
        let y = self.y_pos + (self.height - 8) / 2;
        if self.is_enabled {
            let var1 = self.cursor_visible();
            let s = format!("{}{}", self.text, if var1 { "_" } else { "" });
            draw::draw_string(g, font, &s, self.x_pos + 4, y, TEXT);
        } else {
            draw::draw_string(g, font, &self.text, self.x_pos + 4, y, TEXT_DISABLED);
        }
    }

    /// `setMaxStringLength(n)`.
    pub fn set_max_string_length(&mut self, n: i32) {
        self.max_string_length = n;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allowed() -> ChatAllowedCharacters {
        ChatAllowedCharacters::from_chars("abc 12".chars().collect())
    }

    #[test]
    fn editing_with_a_maximum() {
        let a = allowed();
        let mut t = GuiTextField::new(0, 0, 100, 20, "");
        t.set_max_string_length(3);
        // Not focused: nothing.
        t.textbox_key_typed(&a, || None, 'a', 30);
        assert_eq!(t.get_text(), "");
        t.set_focused(true);
        for c in "abzc1".chars() {
            t.textbox_key_typed(&a, || None, c, 0);
        }
        assert_eq!(t.get_text(), "abc");
        t.textbox_key_typed(&a, || None, '\0', KEY_BACK);
        assert_eq!(t.get_text(), "ab");
        t.textbox_key_typed(&a, || None, '2', 0);
        assert_eq!(t.get_text(), "ab2");
        // Backspace and a character in one event: both.
        t.textbox_key_typed(&a, || None, 'c', KEY_BACK);
        assert_eq!(t.get_text(), "abc");
        // Tab asks for the next field and types nothing.
        assert!(t.textbox_key_typed(&a, || None, '\t', 15));
        // The paste ignores the maximum (vanilla bug), up to 32.
        t.textbox_key_typed(&a, || Some("zz".into()), '\u{16}', 47);
        assert_eq!(t.get_text(), "abczz");
        t.set_text("");
        t.textbox_key_typed(&a, || Some("x".repeat(40)), '\u{16}', 47);
        assert_eq!(t.get_text().len(), 32);
        // No limit.
        let mut u = GuiTextField::new(0, 0, 100, 20, "");
        u.set_focused(true);
        for _ in 0..50 {
            u.textbox_key_typed(&a, || None, 'a', 30);
        }
        assert_eq!(u.get_text().len(), 50);
        u.is_enabled = false;
        u.textbox_key_typed(&a, || None, 'a', 30);
        assert_eq!(u.get_text().len(), 50);
    }

    #[test]
    fn focus_and_blink() {
        let mut t = GuiTextField::new(10, 10, 50, 20, "x");
        t.mouse_clicked(10, 10, 0);
        assert!(t.is_focused && t.cursor_visible());
        for _ in 0..6 {
            t.update_cursor_counter();
        }
        assert!(!t.cursor_visible());
        for _ in 0..6 {
            t.update_cursor_counter();
        }
        assert!(t.cursor_visible());
        t.mouse_clicked(60, 10, 0);
        assert!(!t.is_focused);
        t.mouse_clicked(59, 29, 0);
        assert!(t.is_focused);
        assert!(t.cursor_visible());
    }
}
