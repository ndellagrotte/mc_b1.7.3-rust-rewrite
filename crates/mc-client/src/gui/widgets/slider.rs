//! `GuiSlider` (GuiSlider.java): a 150-pixel button over a float option
//! (music, sound, sensitivity). Its data is
//! [`ButtonKind::Slider`](super::button::ButtonKind::Slider); these are its
//! overrides of `mouseDragged` and `mousePressed`.

use super::button::{ButtonKind, GuiButton};
use crate::events::ClientEvent;
use crate::gui::batch::GuiGl;
use crate::gui::draw;
use crate::gui::screen::ScreenCtx;
use crate::settings::EnumOptions;

/// The value under the pointer: `(x - (xPosition + 4)) / (width - 8)`,
/// clamped to 0..1.
// src: GuiSlider.java:24-31, :45-52
#[allow(clippy::manual_clamp)]
pub fn value_at(b: &GuiButton, var2: i32) -> f32 {
    let mut v = (var2 - (b.x_position + 4)) as f32 / (b.width - 8) as f32;
    if v < 0.0 {
        v = 0.0;
    }
    if v > 1.0 {
        v = 1.0;
    }
    v
}

/// The knob's x: `xPosition + (int)(sliderValue * (width - 8))`.
// src: GuiSlider.java:38
pub fn knob_x(b: &GuiButton, slider_value: f32) -> i32 {
    b.x_position + (slider_value * (b.width - 8) as f32) as i32
}

/// `gameSettings.setOptionFloatValue(option, v)` with its side effect sent
/// on, then the label `gameSettings.getKeyBinding(option)`.
// src: GuiSlider.java:33-34, :54-55
fn apply(b: &mut GuiButton, ctx: &mut ScreenCtx, option: EnumOptions, v: f32) {
    if let Some(e) = ctx.game.game_settings.set_option_float_value(option, v) {
        ctx.game.outbox.events.push(ClientEvent::Settings(e));
    }
    b.display_string = ctx.game.game_settings.get_key_binding(option, ctx.lang);
}

/// `GuiSlider.mouseDragged(mc, x, y)`.
// src: GuiSlider.java:21-41
pub fn mouse_dragged(b: &mut GuiButton, ctx: &mut ScreenCtx, g: &mut GuiGl, var2: i32, _var3: i32) {
    if !b.enabled2 {
        return;
    }
    let ButtonKind::Slider {
        dragging, id_float, ..
    } = b.kind
    else {
        return;
    };
    if dragging {
        let v = value_at(b, var2);
        set_value(b, v);
        apply(b, ctx, id_float, v);
    }
    let ButtonKind::Slider { slider_value, .. } = b.kind else {
        return;
    };
    g.color4f(1.0, 1.0, 1.0, 1.0);
    let x = knob_x(b, slider_value);
    draw::draw_textured_modal_rect(g, b.z_level, x, b.y_position, 0, 66, 4, 20);
    draw::draw_textured_modal_rect(g, b.z_level, x + 4, b.y_position, 196, 66, 4, 20);
}

/// `GuiSlider.mousePressed(mc, x, y)`.
// src: GuiSlider.java:43-61
pub fn mouse_pressed(b: &mut GuiButton, ctx: &mut ScreenCtx, var2: i32, var3: i32) -> bool {
    if !b.button_mouse_pressed(var2, var3) {
        return false;
    }
    let ButtonKind::Slider { id_float, .. } = b.kind else {
        return true;
    };
    let v = value_at(b, var2);
    set_value(b, v);
    apply(b, ctx, id_float, v);
    if let ButtonKind::Slider { dragging, .. } = &mut b.kind {
        *dragging = true;
    }
    true
}

fn set_value(b: &mut GuiButton, v: f32) {
    if let ButtonKind::Slider { slider_value, .. } = &mut b.kind {
        *slider_value = v;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_and_knob() {
        let b = GuiButton::slider(0, 100, 0, EnumOptions::Sensitivity, "s", 0.5);
        assert_eq!(value_at(&b, 104), 0.0);
        assert_eq!(value_at(&b, 50), 0.0);
        assert_eq!(value_at(&b, 246), 1.0);
        assert_eq!(value_at(&b, 400), 1.0);
        assert_eq!(value_at(&b, 175), 71.0 / 142.0);
        assert_eq!(knob_x(&b, 0.5), 171);
        assert_eq!(knob_x(&b, 1.0), 242);
    }
}
