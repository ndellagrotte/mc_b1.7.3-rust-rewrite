//! `GuiYesNo` (GuiYesNo.java): two messages and two buttons; the answer
//! goes to the parent screen's `deleteWorld(answer, id)` (true for the
//! first button), which usually displays itself again
//! ([`ScreenCtx::display_this`], resolved here: the parent object moves
//! from this screen into the slot).

use crate::app::ScreenKind;
use crate::gui::batch::GuiGl;
use crate::gui::draw;
use crate::gui::screen::{
    Screen, ScreenBase, ScreenCtx, default_draw_screen, draw_default_background,
};
use crate::gui::widgets::button::GuiButton;

/// `GuiYesNo`.
pub struct GuiYesNo {
    base: ScreenBase,
    /// `parentScreen` (moved out when it is displayed again).
    parent_screen: Option<Box<dyn Screen>>,
    /// `message1`.
    pub message1: String,
    /// `message2`.
    pub message2: String,
    /// `buttonText1`.
    pub button_text1: String,
    /// `buttonText2`.
    pub button_text2: String,
    /// `worldNumber`: handed back to the parent.
    pub world_number: i32,
}

impl GuiYesNo {
    /// `new GuiYesNo(parent, message1, message2, yes, no, id)`.
    // src: GuiYesNo.java:11-18
    pub fn new(
        parent: Box<dyn Screen>,
        message1: impl Into<String>,
        message2: impl Into<String>,
        button_text1: impl Into<String>,
        button_text2: impl Into<String>,
        world_number: i32,
    ) -> Self {
        GuiYesNo {
            base: ScreenBase::default(),
            parent_screen: Some(parent),
            message1: message1.into(),
            message2: message2.into(),
            button_text1: button_text1.into(),
            button_text2: button_text2.into(),
            world_number,
        }
    }

    /// The parent, while it is still here.
    pub fn parent(&self) -> Option<&dyn Screen> {
        self.parent_screen.as_deref()
    }
}

impl Screen for GuiYesNo {
    fn base(&self) -> &ScreenBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn kind(&self) -> ScreenKind {
        ScreenKind::Other("GuiYesNo".into())
    }

    // src: GuiYesNo.java:20-23
    #[allow(clippy::identity_op)]
    fn init_gui(&mut self, _ctx: &mut ScreenCtx) {
        let (w, h) = (self.base.width, self.base.height);
        self.base.control_list.push(GuiButton::small(
            0,
            w / 2 - 155 + 0,
            h / 6 + 96,
            self.button_text1.clone(),
        ));
        self.base.control_list.push(GuiButton::small(
            1,
            w / 2 - 155 + 160,
            h / 6 + 96,
            self.button_text2.clone(),
        ));
    }

    // src: GuiYesNo.java:25-27
    fn action_performed(&mut self, ctx: &mut ScreenCtx, button: usize) {
        let id = self.base.control_list[button].id;
        if let Some(p) = self.parent_screen.as_mut() {
            let outer = ctx.take_this_request();
            p.delete_world(ctx, id == 0, self.world_number);
            // The parent's `mc.displayGuiScreen(this)`.
            if let Some(req) = ctx.take_this_request()
                && let Some(p) = self.parent_screen.take()
            {
                ctx.resolve_this(req, p);
            }
            if let Some(o) = outer {
                restore_this_request(ctx, o);
            }
        }
    }

    // src: GuiYesNo.java:29-34
    fn draw_screen(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, mx: i32, my: i32, _p: f32) {
        draw_default_background(&self.base, ctx, g);
        let w = self.base.width;
        draw::draw_centered_string(g, ctx.font, &self.message1, w / 2, 70, 16777215);
        draw::draw_centered_string(g, ctx.font, &self.message2, w / 2, 90, 16777215);
        default_draw_screen(&mut self.base, ctx, g, mx, my);
    }
}

/// Puts back a request of this screen's own that was pending before the
/// parent ran.
fn restore_this_request(ctx: &mut ScreenCtx, req: crate::gui::screen::ThisRequest) {
    match req {
        crate::gui::screen::ThisRequest::Display => ctx.display_this(),
        crate::gui::screen::ThisRequest::Wrap(f) => ctx.display_wrapping_this(f),
    }
}
