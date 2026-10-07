//! `GuiButton` (GuiButton.java) and `GuiSmallButton` (GuiSmallButton.java);
//! `GuiSlider` is a third kind of button ([`ButtonKind::Slider`], its
//! behaviour in [`super::slider`]). Java's subclass overrides become a
//! `match` on the kind.

use super::slider;
use crate::gui::batch::GuiGl;
use crate::gui::draw;
use crate::gui::screen::ScreenCtx;
use crate::settings::EnumOptions;

/// `/gui/gui.png`: the widgets sheet (buttons at v 46, 66, 86).
// src: GuiButton.java:47
pub const GUI_PNG: &str = "/gui/gui.png";

/// The text colour of a disabled button (`-6250336`, `0xffa0a0a0`).
// src: GuiButton.java:55
pub const COLOR_DISABLED: i32 = -6250336;
/// The text colour under the pointer (`16777120`, `0xffffa0`).
// src: GuiButton.java:57
pub const COLOR_HOVER: i32 = 16777120;
/// The text colour otherwise (`14737632`, `0xe0e0e0`).
// src: GuiButton.java:59
pub const COLOR_NORMAL: i32 = 14737632;

/// Which class a button is.
#[derive(Clone, Debug, PartialEq)]
pub enum ButtonKind {
    /// `GuiButton`.
    Button,
    /// `GuiSmallButton` with its `enumOptions` (`returnEnumOptions()`).
    Small {
        /// The option an options screen attached (or none).
        enum_options: Option<EnumOptions>,
    },
    /// `GuiSlider`.
    Slider {
        /// `sliderValue` (0..1).
        slider_value: f32,
        /// `dragging`.
        dragging: bool,
        /// `idFloat`: the float option it sets.
        id_float: EnumOptions,
    },
}

/// `GuiButton` (module docs).
#[derive(Clone, Debug, PartialEq)]
pub struct GuiButton {
    /// `width`.
    pub width: i32,
    /// `height`.
    pub height: i32,
    /// `xPosition`.
    pub x_position: i32,
    /// `yPosition`.
    pub y_position: i32,
    /// `displayString`.
    pub display_string: String,
    /// `id`.
    pub id: i32,
    /// `enabled`: clickable and drawn normally.
    pub enabled: bool,
    /// `enabled2`: drawn at all.
    pub enabled2: bool,
    /// `zLevel` (Gui.java:6).
    pub z_level: f32,
    /// The class.
    pub kind: ButtonKind,
}

impl GuiButton {
    /// `new GuiButton(id, x, y, text)`: 200 × 20.
    // src: GuiButton.java:16-18
    pub fn new(id: i32, x: i32, y: i32, text: impl Into<String>) -> Self {
        Self::with_size(id, x, y, 200, 20, text)
    }

    /// `new GuiButton(id, x, y, width, height, text)`.
    // src: GuiButton.java:20-31
    pub fn with_size(id: i32, x: i32, y: i32, w: i32, h: i32, text: impl Into<String>) -> Self {
        GuiButton {
            width: w,
            height: h,
            x_position: x,
            y_position: y,
            display_string: text.into(),
            id,
            enabled: true,
            enabled2: true,
            z_level: 0.0,
            kind: ButtonKind::Button,
        }
    }

    /// `new GuiSmallButton(id, x, y, text)`: 150 × 20, no option.
    // src: GuiSmallButton.java:6-8
    pub fn small(id: i32, x: i32, y: i32, text: impl Into<String>) -> Self {
        Self::small_option(id, x, y, None, text)
    }

    /// `new GuiSmallButton(id, x, y, width, height, text)`.
    // src: GuiSmallButton.java:10-13
    pub fn small_sized(id: i32, x: i32, y: i32, w: i32, h: i32, text: impl Into<String>) -> Self {
        GuiButton {
            kind: ButtonKind::Small { enum_options: None },
            ..Self::with_size(id, x, y, w, h, text)
        }
    }

    /// `new GuiSmallButton(id, x, y, option, text)`: 150 × 20.
    // src: GuiSmallButton.java:15-18
    pub fn small_option(
        id: i32,
        x: i32,
        y: i32,
        option: Option<EnumOptions>,
        text: impl Into<String>,
    ) -> Self {
        GuiButton {
            kind: ButtonKind::Small {
                enum_options: option,
            },
            ..Self::with_size(id, x, y, 150, 20, text)
        }
    }

    /// `new GuiSlider(id, x, y, option, text, value)`: 150 × 20.
    // src: GuiSlider.java:11-15
    pub fn slider(
        id: i32,
        x: i32,
        y: i32,
        option: EnumOptions,
        text: impl Into<String>,
        value: f32,
    ) -> Self {
        GuiButton {
            kind: ButtonKind::Slider {
                slider_value: value,
                dragging: false,
                id_float: option,
            },
            ..Self::with_size(id, x, y, 150, 20, text)
        }
    }

    /// `GuiSmallButton.returnEnumOptions()`.
    // src: GuiSmallButton.java:20-22
    pub fn return_enum_options(&self) -> Option<EnumOptions> {
        match self.kind {
            ButtonKind::Small { enum_options } => enum_options,
            _ => None,
        }
    }

    /// `getHoverState(hovered)`: 0 disabled, 1 normal, 2 under the pointer
    /// (the sheet row `46 + 20·state`); a slider's track is always 0.
    // src: GuiButton.java:33-42; GuiSlider.java:17-19
    pub fn get_hover_state(&self, var1: bool) -> i32 {
        if let ButtonKind::Slider { .. } = self.kind {
            return 0;
        }
        let mut var2 = 1;
        if !self.enabled {
            var2 = 0;
        } else if var1 {
            var2 = 2;
        }
        var2
    }

    /// Whether `(x, y)` is on the button (`drawButton`'s `var5`, half of
    /// `mousePressed`).
    // src: GuiButton.java:49
    pub fn is_under(&self, var2: i32, var3: i32) -> bool {
        var2 >= self.x_position
            && var3 >= self.y_position
            && var2 < self.x_position + self.width
            && var3 < self.y_position + self.height
    }

    /// `drawButton(mc, mouseX, mouseY)`: the two halves of the sheet's
    /// button row (left `width / 2` from the left end, right from the 200
    /// pixel row's right end), `mouseDragged` (the slider's knob), then the
    /// centred label in the state's colour.
    // src: GuiButton.java:44-63
    pub fn draw_button(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, var2: i32, var3: i32) {
        if !self.enabled2 {
            return;
        }
        g.bind_texture(GUI_PNG);
        g.color4f(1.0, 1.0, 1.0, 1.0);
        let var5 = self.is_under(var2, var3);
        let var6 = self.get_hover_state(var5);
        let z = self.z_level;
        draw::draw_textured_modal_rect(
            g,
            z,
            self.x_position,
            self.y_position,
            0,
            46 + var6 * 20,
            self.width / 2,
            self.height,
        );
        draw::draw_textured_modal_rect(
            g,
            z,
            self.x_position + self.width / 2,
            self.y_position,
            200 - self.width / 2,
            46 + var6 * 20,
            self.width / 2,
            self.height,
        );
        self.mouse_dragged(ctx, g, var2, var3);
        let color = if !self.enabled {
            COLOR_DISABLED
        } else if var5 {
            COLOR_HOVER
        } else {
            COLOR_NORMAL
        };
        draw::draw_centered_string(
            g,
            ctx.font,
            &self.display_string,
            self.x_position + self.width / 2,
            self.y_position + (self.height - 8) / 2,
            color,
        );
    }

    /// `mouseDragged(mc, x, y)`: nothing for a button; the slider follows
    /// the pointer while dragged and draws its knob.
    // src: GuiButton.java:65-66; GuiSlider.java:21-41
    fn mouse_dragged(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, var2: i32, var3: i32) {
        if let ButtonKind::Slider { .. } = self.kind {
            slider::mouse_dragged(self, ctx, g, var2, var3);
        }
    }

    /// `mouseReleased(x, y)`: the slider stops dragging.
    // src: GuiButton.java:68-69; GuiSlider.java:63-65
    pub fn mouse_released(&mut self, _x: i32, _y: i32) {
        if let ButtonKind::Slider { dragging, .. } = &mut self.kind {
            *dragging = false;
        }
    }

    /// `mousePressed(mc, x, y)`: enabled and under the pointer (the slider
    /// also jumps to the pointer and starts dragging).
    // src: GuiButton.java:71-73; GuiSlider.java:43-61
    pub fn mouse_pressed(&mut self, ctx: &mut ScreenCtx, var2: i32, var3: i32) -> bool {
        if let ButtonKind::Slider { .. } = self.kind {
            return slider::mouse_pressed(self, ctx, var2, var3);
        }
        self.button_mouse_pressed(var2, var3)
    }

    /// `GuiButton.mousePressed` itself (the slider's `super` call).
    // src: GuiButton.java:71-73
    pub fn button_mouse_pressed(&self, var2: i32, var3: i32) -> bool {
        self.enabled && self.is_under(var2, var3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_states() {
        let mut b = GuiButton::new(0, 10, 20, "x");
        assert_eq!((b.width, b.height), (200, 20));
        assert_eq!(b.get_hover_state(false), 1);
        assert_eq!(b.get_hover_state(true), 2);
        b.enabled = false;
        assert_eq!(b.get_hover_state(true), 0);
        let s = GuiButton::slider(1, 0, 0, EnumOptions::Music, "m", 0.5);
        assert_eq!(s.width, 150);
        assert_eq!(s.get_hover_state(true), 0);
        let sm = GuiButton::small_option(2, 0, 0, Some(EnumOptions::Difficulty), "d");
        assert_eq!(
            (sm.width, sm.return_enum_options()),
            (150, Some(EnumOptions::Difficulty))
        );
        assert_eq!(GuiButton::small(3, 0, 0, "s").return_enum_options(), None);
    }

    #[test]
    fn hit_box_is_half_open() {
        let mut b = GuiButton::with_size(0, 10, 20, 30, 20, "x");
        assert!(b.button_mouse_pressed(10, 20));
        assert!(b.button_mouse_pressed(39, 39));
        assert!(!b.button_mouse_pressed(40, 30));
        assert!(!b.button_mouse_pressed(20, 40));
        assert!(!b.button_mouse_pressed(9, 25));
        b.enabled = false;
        assert!(!b.button_mouse_pressed(15, 25));
        assert!(b.is_under(15, 25));
    }
}
