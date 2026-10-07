//! `GuiWorldSlot` (GuiWorldSlot.java): the world list of the select-world
//! screen, 36-pixel rows between y 32 and `height - 64`. Each row shows the
//! level name (or "World N"), the folder with the last-played date and the
//! size, and the conversion note for a pre-McRegion world.
//!
//! The list is a view over the select-world screen's fields
//! ([`WorldSlotList`], Java's inner-class access to its parent), built for
//! each `drawScreen`.

use mc_sim::save::SaveFormatComparator;

use crate::gui::batch::GuiGl;
use crate::gui::draw;
use crate::gui::screen::{ScreenBase, ScreenCtx, draw_default_background};
use crate::gui::widgets::button::GuiButton;
use crate::gui::widgets::slot::{GuiSlot, SlotList};
use crate::jfmt;
use crate::screenshot::LocalTime;

/// The row height.
// src: GuiWorldSlot.java:9, :34
pub const ROW: i32 = 36;

/// `new GuiWorldSlot(parent)`: `GuiSlot(mc, width, height, 32, height - 64,
/// 36)`.
// src: GuiWorldSlot.java:8-11
pub fn new_slot(width: i32, height: i32) -> GuiSlot {
    GuiSlot::new(width, height, 32, height - 64, ROW)
}

/// `new SimpleDateFormat().format(date)`: the default locale's SHORT date
/// and SHORT time pattern. For `en_US` on the JREs Beta ran on (Java 5/6)
/// that is `"M/d/yy h:mm a"` (newer JDKs' CLDR data add a comma and a
/// narrow space; the Java 6 form is kept).
// src: GuiSelectWorld.java:9, GuiWorldSlot.java:49
pub fn format_date(t: LocalTime) -> String {
    let h12 = match t.hour % 12 {
        0 => 12,
        h => h,
    };
    let ampm = if t.hour < 12 { "AM" } else { "PM" };
    format!(
        "{}/{}/{:02} {}:{:02} {}",
        t.month,
        t.day,
        t.year.rem_euclid(100),
        h12,
        t.minute,
        ampm
    )
}

/// The second line's size: `(float)(size / 1024L * 100L / 1024L) / 100.0F
/// + " MB"` (megabytes with two truncated decimals, printed as a Java
/// `float`).
// src: GuiWorldSlot.java:50-51
pub fn format_size(var9: i64) -> String {
    let mb = (var9 / 1024).wrapping_mul(100) / 1024;
    format!("{} MB", jfmt::float_to_string(mb as f32 / 100.0))
}

/// The second line: `fileName + " (" + date + ", " + size + ")"`.
// src: GuiWorldSlot.java:48-51
pub fn detail_line(s: &SaveFormatComparator, date: LocalTime) -> String {
    format!(
        "{} ({}, {})",
        s.file_name,
        format_date(date),
        format_size(s.size_on_disk)
    )
}

/// The select-world screen's fields the list reads and writes
/// (`GuiSelectWorld`'s synthetic accessors).
pub struct WorldSlotList<'a> {
    /// `saveList`.
    pub save_list: &'a [SaveFormatComparator],
    /// `selectedWorld`.
    pub selected_world: &'a mut i32,
    /// `selected` (a world is being opened).
    pub selected: &'a mut bool,
    /// The screen's buttons.
    pub control_list: &'a mut [GuiButton],
    /// `buttonSelect`, `buttonRename`, `buttonDelete` as indices.
    pub buttons: [usize; 3],
    /// `localizedWorldText`.
    pub localized_world_text: &'a str,
    /// `localizedMustConvertText`.
    pub localized_must_convert_text: &'a str,
    /// The screen's size (`drawDefaultBackground`).
    pub width: i32,
    /// The screen's height.
    pub height: i32,
}

impl SlotList for WorldSlotList<'_> {
    // src: GuiWorldSlot.java:13-15
    fn get_size(&self) -> i32 {
        self.save_list.len() as i32
    }

    // src: GuiWorldSlot.java:17-27
    fn element_clicked(&mut self, ctx: &mut ScreenCtx, var1: i32, var2: bool) {
        *self.selected_world = var1;
        let sel = *self.selected_world;
        let var3 = sel >= 0 && sel < self.get_size();
        for i in self.buttons {
            self.control_list[i].enabled = var3;
        }
        if var2 && var3 {
            super::select_world::select_world(ctx, self.save_list, self.selected, var1);
        }
    }

    // src: GuiWorldSlot.java:29-31
    fn is_selected(&self, var1: i32) -> bool {
        var1 == *self.selected_world
    }

    // src: GuiWorldSlot.java:33-35
    fn get_content_height(&self, _slot: &GuiSlot) -> i32 {
        self.save_list.len() as i32 * 36
    }

    // src: GuiWorldSlot.java:37-39
    fn draw_background(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl) {
        let b = ScreenBase {
            width: self.width,
            height: self.height,
            ..ScreenBase::default()
        };
        draw_default_background(&b, ctx, g);
    }

    // src: GuiWorldSlot.java:41-60
    fn draw_slot(
        &mut self,
        ctx: &mut ScreenCtx,
        g: &mut GuiGl,
        var1: i32,
        var2: i32,
        var3: i32,
        _h: i32,
    ) {
        let var6 = &self.save_list[var1 as usize];
        let mut var7 = var6.display_name.clone();
        if var7.is_empty() {
            var7 = format!("{} {}", self.localized_world_text, var1 + 1);
        }
        let var8 = detail_line(var6, crate::screenshot::local_time(var6.last_time_played));
        let mut var11 = String::new();
        if var6.requires_conversion {
            var11 = format!("{} {}", self.localized_must_convert_text, var11);
        }
        let f = ctx.font;
        draw::draw_string(g, f, &var7, var2 + 2, var3 + 1, 16777215);
        draw::draw_string(g, f, &var8, var2 + 2, var3 + 12, 8421504);
        draw::draw_string(g, f, &var11, var2 + 2, var3 + 12 + 10, 8421504);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> LocalTime {
        LocalTime {
            year,
            month,
            day,
            hour,
            minute,
            second: 0,
        }
    }

    #[test]
    fn simple_date_format_short_short() {
        assert_eq!(format_date(t(2011, 9, 15, 0, 5)), "9/15/11 12:05 AM");
        assert_eq!(format_date(t(2011, 12, 1, 12, 0)), "12/1/11 12:00 PM");
        assert_eq!(format_date(t(2026, 10, 7, 23, 59)), "10/7/26 11:59 PM");
        assert_eq!(format_date(t(2000, 1, 2, 9, 30)), "1/2/00 9:30 AM");
    }

    #[test]
    fn sizes_in_megabytes() {
        assert_eq!(format_size(0), "0.0 MB");
        // 1.5 MB: 1572864 / 1024 * 100 / 1024 = 150.
        assert_eq!(format_size(1572864), "1.5 MB");
        // Truncated, not rounded: 1048575 bytes -> 1023 KB -> 99 -> 0.99.
        assert_eq!(format_size(1048575), "0.99 MB");
        assert_eq!(format_size(123_456_789), "117.73 MB");
    }

    #[test]
    fn the_second_line() {
        let s = SaveFormatComparator::new("New World-", "New World", 0, 2048, false);
        assert_eq!(
            detail_line(&s, crate::screenshot::utc_time(0)),
            "New World- (1/1/70 12:00 AM, 0.0 MB)"
        );
    }
}
