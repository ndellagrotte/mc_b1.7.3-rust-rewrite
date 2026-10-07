//! `GuiSleepMP` (GuiSleepMP.java): the chat with a "Leave Bed" button,
//! shown while sleeping on a multiplayer world (Minecraft.java:978-983;
//! the shell never opens it in singleplayer, D3). Registered so the shell
//! and the dev action can build it. Leaving the bed sends a packet only
//! for `EntityClientPlayerMP` (deferred): in singleplayer the button and
//! Escape do nothing, as in vanilla.

use crate::app::ScreenKind;
use crate::gui::batch::GuiGl;
use crate::gui::chat::{GuiChat, KEY_RETURN};
use crate::gui::font::java_trim;
use crate::gui::screen::{Screen, ScreenBase, ScreenCtx, default_mouse_clicked};
use crate::gui::widgets::button::GuiButton;
use crate::keys::KEY_ESCAPE;

/// `GuiSleepMP` (a `GuiChat`).
#[derive(Clone, Debug, Default)]
pub struct GuiSleepMp {
    chat: GuiChat,
}

impl GuiSleepMp {
    /// `wakeFromSleep()`: a `Packet19EntityAction(player, 3)` for a
    /// multiplayer player only (deferred, D3); nothing in singleplayer.
    // src: GuiSleepMP.java:46-51
    fn wake_from_sleep(&mut self, _ctx: &mut ScreenCtx) {}
}

impl Screen for GuiSleepMp {
    fn base(&self) -> &ScreenBase {
        self.chat.base()
    }
    fn base_mut(&mut self) -> &mut ScreenBase {
        self.chat.base_mut()
    }
    fn kind(&self) -> ScreenKind {
        ScreenKind::SleepMp
    }
    // src: GuiSleepMP.java:6-10
    fn init_gui(&mut self, ctx: &mut ScreenCtx) {
        ctx.enable_repeat_events(true);
        let (w, h) = (self.chat.base().width, self.chat.base().height);
        let text = ctx.lang.translate_key("multiplayer.stopSleeping");
        self.chat
            .base_mut()
            .control_list
            .push(GuiButton::new(1, w / 2 - 100, h - 40, text));
    }
    // src: GuiSleepMP.java:12-14
    fn on_gui_closed(&mut self, ctx: &mut ScreenCtx) {
        ctx.enable_repeat_events(false);
    }
    // src: GuiChat.java:18-20 (inherited)
    fn update_screen(&mut self, ctx: &mut ScreenCtx) {
        self.chat.update_screen(ctx);
    }
    // src: GuiSleepMP.java:16-30
    fn key_typed(&mut self, ctx: &mut ScreenCtx, var1: char, var2: i32) {
        if var2 == KEY_ESCAPE {
            self.wake_from_sleep(ctx);
        } else if var2 == KEY_RETURN {
            let var3 = java_trim(&self.chat.message);
            if !var3.is_empty() {
                // thePlayer.sendChatMessage: empty in singleplayer
                // (EntityPlayerSP.java:141-142).
            }
            self.chat.message.clear();
        } else {
            // super.keyTyped: GuiChat's, whose Escape and Enter branches
            // were taken above.
            self.chat.edit(ctx, var1, var2);
        }
    }
    // src: GuiSleepMP.java:32-34
    fn draw_screen(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, mx: i32, my: i32, _p: f32) {
        self.chat.draw(ctx, g, mx, my);
    }
    // src: GuiChat.java:53-70 (inherited; field_933_a is never set)
    fn mouse_clicked(&mut self, ctx: &mut ScreenCtx, x: i32, y: i32, button: i32) {
        if button == 0 {
            default_mouse_clicked(self, ctx, x, y, button);
        }
    }
    // src: GuiSleepMP.java:36-44
    fn action_performed(&mut self, ctx: &mut ScreenCtx, button: usize) {
        if self.chat.base().control_list[button].id == 1 {
            self.wake_from_sleep(ctx);
        }
    }
}
