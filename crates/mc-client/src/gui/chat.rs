//! `ChatLine` (ChatLine.java) and `GuiChat` (GuiChat.java).
//!
//! In singleplayer vanilla opens `GuiChat` only for a multiplayer world
//! (the chat key, Minecraft.java:1068-1070), and a line sent from it goes
//! nowhere: `Minecraft.lineIsCommand` answers false for every line
//! (Minecraft.java:1547-1552) and `EntityPlayerSP.sendChatMessage` is empty
//! (EntityPlayerSP.java:141-142). The screen is registered so the dev
//! action `open_screen GuiChat` can show it; the chat lines themselves are
//! [`super::ingame::GuiIngame`]'s.

use crate::app::ScreenKind;
use crate::gui::batch::GuiGl;
use crate::gui::draw;
use crate::gui::screen::{
    Screen, ScreenBase, ScreenCtx, default_draw_screen, default_mouse_clicked,
};
use crate::keys::KEY_ESCAPE;

/// LWJGL `KEY_RETURN`.
pub const KEY_RETURN: i32 = 28;
/// LWJGL `KEY_BACK`.
pub const KEY_BACK: i32 = 14;

/// `ChatLine`: a line of the chat and its age in ticks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatLine {
    /// `message`.
    // src: ChatLine.java:4
    pub message: String,
    /// `updateCounter`: ticks since the line was added
    /// (`GuiIngame.updateTick`).
    // src: ChatLine.java:5
    pub update_counter: i32,
}

impl ChatLine {
    /// `new ChatLine(message)`.
    // src: ChatLine.java:7-10
    pub fn new(var1: impl Into<String>) -> Self {
        ChatLine {
            message: var1.into(),
            update_counter: 0,
        }
    }
}

/// Java's `String.length()` for the chat's strings (the font's characters
/// are all in the BMP: one UTF-16 unit each).
pub fn java_len(s: &str) -> i32 {
    s.chars().count() as i32
}

/// Java's `s.substring(a, b)` in characters.
pub fn substring(s: &str, a: i32, b: i32) -> String {
    s.chars()
        .skip(a.max(0) as usize)
        .take((b - a).max(0) as usize)
        .collect()
}

/// `GuiChat`.
#[derive(Clone, Debug, Default)]
pub struct GuiChat {
    base: ScreenBase,
    /// `message`: the line being typed.
    // src: GuiChat.java:6
    pub message: String,
    /// `updateCounter`: ticks open (the cursor blink).
    // src: GuiChat.java:7
    pub update_counter: i32,
}

impl GuiChat {
    /// `keyTyped`'s editing branch (also `GuiSleepMP`'s `super.keyTyped`):
    /// Backspace drops the last character; an allowed character is
    /// appended while the line is shorter than 100.
    // src: GuiChat.java:36-43
    pub fn edit(&mut self, ctx: &ScreenCtx, var1: char, var2: i32) {
        if var2 == KEY_BACK && !self.message.is_empty() {
            self.message.pop();
        }
        if ctx.font.allowed.is_allowed(var1) && java_len(&self.message) < 100 {
            self.message.push(var1);
        }
    }

    /// The fields (for `GuiSleepMP`, which extends it).
    pub fn base_mut(&mut self) -> &mut ScreenBase {
        &mut self.base
    }

    /// The fields.
    pub fn base(&self) -> &ScreenBase {
        &self.base
    }

    /// `GuiChat.drawScreen`: a translucent black bar at the bottom and
    /// `"> " + message` with a blinking `_`, then the buttons.
    // src: GuiChat.java:47-51
    pub fn draw(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, mx: i32, my: i32) {
        let (w, h) = (self.base.width, self.base.height);
        draw::draw_rect(g, 2, h - 14, w - 2, h - 2, i32::MIN);
        let cursor = if self.update_counter / 6 % 2 == 0 {
            "_"
        } else {
            ""
        };
        draw::draw_string(
            g,
            ctx.font,
            &format!("> {}{}", self.message, cursor),
            4,
            h - 12,
            14737632,
        );
        default_draw_screen(&mut self.base, ctx, g, mx, my);
    }
}

impl Screen for GuiChat {
    fn base(&self) -> &ScreenBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn kind(&self) -> ScreenKind {
        ScreenKind::Chat
    }
    // src: GuiChat.java:10-12
    fn init_gui(&mut self, ctx: &mut ScreenCtx) {
        ctx.enable_repeat_events(true);
    }
    // src: GuiChat.java:14-16
    fn on_gui_closed(&mut self, ctx: &mut ScreenCtx) {
        ctx.enable_repeat_events(false);
    }
    // src: GuiChat.java:18-20
    fn update_screen(&mut self, _ctx: &mut ScreenCtx) {
        self.update_counter = self.update_counter.wrapping_add(1);
    }
    // src: GuiChat.java:22-45
    fn key_typed(&mut self, ctx: &mut ScreenCtx, var1: char, var2: i32) {
        if var2 == KEY_ESCAPE {
            ctx.display_gui_screen(None);
        } else if var2 == KEY_RETURN {
            let var3 = crate::gui::font::java_trim(&self.message);
            if !var3.is_empty() {
                // mc.lineIsCommand(var4) is false for every line and
                // thePlayer.sendChatMessage(var4) is empty in singleplayer
                // (Minecraft.java:1547-1552, EntityPlayerSP.java:141-142):
                // the line goes nowhere.
            }
            ctx.display_gui_screen(None);
        } else {
            self.edit(ctx, var1, var2);
        }
    }
    // src: GuiChat.java:47-51
    fn draw_screen(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, mx: i32, my: i32, _p: f32) {
        self.draw(ctx, g, mx, my);
    }
    // src: GuiChat.java:53-70
    fn mouse_clicked(&mut self, ctx: &mut ScreenCtx, x: i32, y: i32, button: i32) {
        if button == 0 {
            // mc.ingameGUI.field_933_a is never set in this version
            // (GuiIngame.java:15): always the super call.
            default_mouse_clicked(self, ctx, x, y, button);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_strings() {
        assert_eq!(java_len("héllo"), 5);
        assert_eq!(substring("abcdef", 1, 3), "bc");
        assert_eq!(substring("abc", 2, 9), "c");
    }
}
