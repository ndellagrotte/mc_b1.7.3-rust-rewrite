//! `GuiSlot` (GuiSlot.java): the scrolling list of the world, texture-pack
//! and statistics screens: rows of `posZ` pixels between `top` and
//! `bottom`, a dirt background tinted `0x202020`, the selection box, a
//! scroll bar dragged with the left button (the list itself drags too),
//! the top and bottom dirt bands with 4-pixel shadows, and double clicks
//! (250 ms).
//!
//! Java's abstract methods are [`SlotList`], implemented by the screen's
//! list; [`GuiSlot`] keeps the fields. The screen calls
//! [`GuiSlot::draw_screen`] with its list (split from the slot so both can
//! be borrowed).

use crate::clock;
use crate::gui::batch::{GuiGl, gl};
use crate::gui::screen::{BACKGROUND_PNG, ScreenCtx};
use crate::gui::widgets::button::GuiButton;

/// What a list provides (`GuiSlot`'s abstract and overridable methods).
pub trait SlotList {
    /// `getSize()`.
    fn get_size(&self) -> i32;
    /// `elementClicked(index, doubleClick)`.
    fn element_clicked(&mut self, ctx: &mut ScreenCtx, index: i32, double_click: bool);
    /// `isSelected(index)`.
    fn is_selected(&self, index: i32) -> bool;
    /// `getContentHeight()`: by default `getSize() * posZ + headerPadding`.
    // src: GuiSlot.java:58-60
    fn get_content_height(&self, slot: &GuiSlot) -> i32 {
        self.get_size() * slot.pos_z + slot.header_padding
    }
    /// `drawBackground()`.
    fn draw_background(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl);
    /// `drawSlot(index, x, y, height, tessellator)`.
    fn draw_slot(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, index: i32, x: i32, y: i32, h: i32);
    /// `func_27260_a(x, y, tessellator)`: the list header
    /// ([`GuiSlot::set_has_list_header`]).
    fn draw_list_header(&mut self, _ctx: &mut ScreenCtx, _g: &mut GuiGl, _x: i32, _y: i32) {}
    /// `func_27255_a(x, y)`: a click on the header (relative to it).
    fn header_clicked(&mut self, _ctx: &mut ScreenCtx, _x: i32, _y: i32) {}
    /// `func_27257_b(mouseX, mouseY)`: drawn over the list at the end.
    fn post_draw(&mut self, _ctx: &mut ScreenCtx, _g: &mut GuiGl, _mx: i32, _my: i32) {}
}

/// `GuiSlot`'s fields (module docs).
#[derive(Clone, Debug, PartialEq)]
pub struct GuiSlot {
    /// `width` (the screen's).
    pub width: i32,
    /// `height`.
    pub height: i32,
    /// `top`.
    pub top: i32,
    /// `bottom`.
    pub bottom: i32,
    /// `right` (= `width`).
    pub right: i32,
    /// `left` (0).
    pub left: i32,
    /// `posZ`: the row height.
    pub pos_z: i32,
    /// `scrollUpButtonID`.
    pub scroll_up_button_id: i32,
    /// `scrollDownButtonID`.
    pub scroll_down_button_id: i32,
    /// `initialClickY`: -1 button up, -2 drag ignored, else the last y.
    pub initial_click_y: f32,
    /// `scrollMultiplier`.
    pub scroll_multiplier: f32,
    /// `amountScrolled`.
    pub amount_scrolled: f32,
    /// `selectedElement` (the last clicked).
    pub selected_element: i32,
    /// `lastClicked` (ms).
    pub last_clicked: i64,
    /// `field_25123_p`: draw the selection box.
    pub show_selection_box: bool,
    /// `field_27262_q`: the list has a header.
    pub has_list_header: bool,
    /// `field_27261_r`: the header's height.
    pub header_padding: i32,
}

impl GuiSlot {
    /// `new GuiSlot(mc, width, height, top, bottom, posZ)`.
    // src: GuiSlot.java:28-37
    pub fn new(width: i32, height: i32, top: i32, bottom: i32, pos_z: i32) -> Self {
        GuiSlot {
            width,
            height,
            top,
            bottom,
            right: width,
            left: 0,
            pos_z,
            scroll_up_button_id: 0,
            scroll_down_button_id: 0,
            initial_click_y: -2.0,
            scroll_multiplier: 0.0,
            amount_scrolled: 0.0,
            selected_element: -1,
            last_clicked: 0,
            show_selection_box: true,
            has_list_header: false,
            header_padding: 0,
        }
    }

    /// `setShowSelectionBox(b)`.
    pub fn set_show_selection_box(&mut self, b: bool) {
        self.show_selection_box = b;
    }

    /// `func_27259_a(hasHeader, height)`.
    // src: GuiSlot.java:43-50
    pub fn set_has_list_header(&mut self, var1: bool, var2: i32) {
        self.has_list_header = var1;
        self.header_padding = var2;
        if !var1 {
            self.header_padding = 0;
        }
    }

    /// `func_27256_c(x, y)`: the row under `(x, y)`, or -1.
    // src: GuiSlot.java:75-81
    pub fn get_slot_index_from_screen_coords(&self, size: i32, var1: i32, var2: i32) -> i32 {
        let var3 = self.width / 2 - 110;
        let var4 = self.width / 2 + 110;
        let var5 = var2 - self.top - self.header_padding + self.amount_scrolled as i32 - 4;
        let var6 = var5 / self.pos_z;
        if var1 >= var3 && var1 <= var4 && var6 >= 0 && var5 >= 0 && var6 < size {
            var6
        } else {
            -1
        }
    }

    /// `registerScrollButtons(list, up, down)` (the list is not used).
    // src: GuiSlot.java:83-86
    pub fn register_scroll_buttons(&mut self, up: i32, down: i32) {
        self.scroll_up_button_id = up;
        self.scroll_down_button_id = down;
    }

    /// `bindAmountScrolled()`: clamps the scroll to 0 ..= content height −
    /// (bottom − top − 4). A list shorter than its area gets half that
    /// (negative) value, applied after the clamp to 0: the scroll is then
    /// negative and the rows sit centred in the area.
    // src: GuiSlot.java:88-102
    pub fn bind_amount_scrolled(&mut self, content_height: i32) {
        let mut var1 = content_height - (self.bottom - self.top - 4);
        if var1 < 0 {
            var1 /= 2;
        }
        if self.amount_scrolled < 0.0 {
            self.amount_scrolled = 0.0;
        }
        if self.amount_scrolled > var1 as f32 {
            self.amount_scrolled = var1 as f32;
        }
    }

    /// `actionPerformed(button)`: the scroll buttons move two thirds of a
    /// row.
    // src: GuiSlot.java:104-117
    pub fn action_performed(&mut self, list: &dyn SlotList, var1: &GuiButton) {
        if var1.enabled {
            if var1.id == self.scroll_up_button_id {
                self.amount_scrolled -= (self.pos_z * 2 / 3) as f32;
                self.initial_click_y = -2.0;
                let h = list.get_content_height(self);
                self.bind_amount_scrolled(h);
            } else if var1.id == self.scroll_down_button_id {
                self.amount_scrolled += (self.pos_z * 2 / 3) as f32;
                self.initial_click_y = -2.0;
                let h = list.get_content_height(self);
                self.bind_amount_scrolled(h);
            }
        }
    }

    /// The scroll bar's thumb height: `(bottom - top)² / contentHeight`,
    /// within 32 ..= bottom − top − 8. `float_div` is the drag code's float
    /// division (GuiSlot.java:154), the drawing's is integer (:260).
    // src: GuiSlot.java:154-161, :260-267
    pub fn thumb_height(&self, content_height: i32, float_div: bool) -> i32 {
        let span = self.bottom - self.top;
        let mut var13 = if float_div {
            ((span * span) as f32 / content_height as f32) as i32
        } else {
            (span * span) / content_height
        };
        if var13 < 32 {
            var13 = 32;
        }
        if var13 > self.bottom - self.top - 8 {
            var13 = self.bottom - self.top - 8;
        }
        var13
    }

    /// The mouse half of `drawScreen` (before the drawing): a press starts
    /// a click (row, header or scroll bar), a held button drags.
    // src: GuiSlot.java:119-184
    pub fn handle_mouse(
        &mut self,
        list: &mut dyn SlotList,
        ctx: &mut ScreenCtx,
        var1: i32,
        var2: i32,
        button_down: bool,
        now_millis: impl Fn() -> i64,
    ) {
        let var4 = list.get_size();
        let var5 = self.width / 2 + 124;
        let var6 = var5 + 6;
        if button_down {
            if self.initial_click_y == -1.0 {
                let mut var7 = true;
                if var2 >= self.top && var2 <= self.bottom {
                    let var8 = self.width / 2 - 110;
                    let var9 = self.width / 2 + 110;
                    let var10 =
                        var2 - self.top - self.header_padding + self.amount_scrolled as i32 - 4;
                    let var11 = var10 / self.pos_z;
                    if var1 >= var8 && var1 <= var9 && var11 >= 0 && var10 >= 0 && var11 < var4 {
                        let var12 = var11 == self.selected_element
                            && now_millis() - self.last_clicked < 250;
                        list.element_clicked(ctx, var11, var12);
                        self.selected_element = var11;
                        self.last_clicked = now_millis();
                    } else if var1 >= var8 && var1 <= var9 && var10 < 0 {
                        list.header_clicked(
                            ctx,
                            var1 - var8,
                            var2 - self.top + self.amount_scrolled as i32 - 4,
                        );
                        var7 = false;
                    }
                    if var1 >= var5 && var1 <= var6 {
                        self.scroll_multiplier = -1.0;
                        let mut var19 =
                            list.get_content_height(self) - (self.bottom - self.top - 4);
                        if var19 < 1 {
                            var19 = 1;
                        }
                        let var13 = self.thumb_height(list.get_content_height(self), true);
                        self.scroll_multiplier /=
                            (self.bottom - self.top - var13) as f32 / var19 as f32;
                    } else {
                        self.scroll_multiplier = 1.0;
                    }
                    if var7 {
                        self.initial_click_y = var2 as f32;
                    } else {
                        self.initial_click_y = -2.0;
                    }
                } else {
                    self.initial_click_y = -2.0;
                }
            } else if self.initial_click_y >= 0.0 {
                self.amount_scrolled -=
                    (var2 as f32 - self.initial_click_y) * self.scroll_multiplier;
                self.initial_click_y = var2 as f32;
            }
        } else {
            self.initial_click_y = -1.0;
        }
        let h = list.get_content_height(self);
        self.bind_amount_scrolled(h);
    }

    /// `drawScreen(mouseX, mouseY, partialTicks)`.
    // src: GuiSlot.java:119-302
    pub fn draw_screen(
        &mut self,
        list: &mut dyn SlotList,
        ctx: &mut ScreenCtx,
        g: &mut GuiGl,
        var1: i32,
        var2: i32,
        _var3: f32,
    ) {
        list.draw_background(ctx, g);
        let var4 = list.get_size();
        let var5 = self.width / 2 + 124;
        let var6 = var5 + 6;
        let down = ctx.game.input.is_button_down(0);
        self.handle_mouse(list, ctx, var1, var2, down, clock::current_time_millis);
        g.disable(gl::LIGHTING);
        g.disable(gl::FOG);
        g.bind_texture(BACKGROUND_PNG);
        g.color4f(1.0, 1.0, 1.0, 1.0);
        let var17 = 32.0f32;
        let scrolled = self.amount_scrolled as i32;
        g.start_drawing_quads();
        g.set_color_opaque_i(2105376);
        let (l, r, t, b) = (self.left, self.right, self.top, self.bottom);
        g.add_vertex_with_uv(
            l as f64,
            b as f64,
            0.0,
            (l as f32 / var17) as f64,
            ((b + scrolled) as f32 / var17) as f64,
        );
        g.add_vertex_with_uv(
            r as f64,
            b as f64,
            0.0,
            (r as f32 / var17) as f64,
            ((b + scrolled) as f32 / var17) as f64,
        );
        g.add_vertex_with_uv(
            r as f64,
            t as f64,
            0.0,
            (r as f32 / var17) as f64,
            ((t + scrolled) as f32 / var17) as f64,
        );
        g.add_vertex_with_uv(
            l as f64,
            t as f64,
            0.0,
            (l as f32 / var17) as f64,
            ((t + scrolled) as f32 / var17) as f64,
        );
        g.draw();
        let var9 = self.width / 2 - 92 - 16;
        let var10 = self.top + 4 - scrolled;
        if self.has_list_header {
            list.draw_list_header(ctx, g, var9, var10);
        }
        for var11 in 0..var4 {
            let var19 = var10 + var11 * self.pos_z + self.header_padding;
            let var13 = self.pos_z - 4;
            if var19 <= self.bottom && var19 + var13 >= self.top {
                if self.show_selection_box && list.is_selected(var11) {
                    let var14 = self.width / 2 - 110;
                    let var15 = self.width / 2 + 110;
                    g.color4f(1.0, 1.0, 1.0, 1.0);
                    g.disable(gl::TEXTURE_2D);
                    g.start_drawing_quads();
                    g.set_color_opaque_i(8421504);
                    g.add_vertex_with_uv(var14 as f64, (var19 + var13 + 2) as f64, 0.0, 0.0, 1.0);
                    g.add_vertex_with_uv(var15 as f64, (var19 + var13 + 2) as f64, 0.0, 1.0, 1.0);
                    g.add_vertex_with_uv(var15 as f64, (var19 - 2) as f64, 0.0, 1.0, 0.0);
                    g.add_vertex_with_uv(var14 as f64, (var19 - 2) as f64, 0.0, 0.0, 0.0);
                    g.set_color_opaque_i(0);
                    g.add_vertex_with_uv(
                        (var14 + 1) as f64,
                        (var19 + var13 + 1) as f64,
                        0.0,
                        0.0,
                        1.0,
                    );
                    g.add_vertex_with_uv(
                        (var15 - 1) as f64,
                        (var19 + var13 + 1) as f64,
                        0.0,
                        1.0,
                        1.0,
                    );
                    g.add_vertex_with_uv((var15 - 1) as f64, (var19 - 1) as f64, 0.0, 1.0, 0.0);
                    g.add_vertex_with_uv((var14 + 1) as f64, (var19 - 1) as f64, 0.0, 0.0, 0.0);
                    g.draw();
                    g.enable(gl::TEXTURE_2D);
                }
                list.draw_slot(ctx, g, var11, var9, var19, var13);
            }
        }
        g.disable(gl::DEPTH_TEST);
        let var18 = 4;
        self.overlay_background(g, 0, self.top, 255, 255);
        self.overlay_background(g, self.bottom, self.height, 255, 255);
        g.enable(gl::BLEND);
        g.blend_func(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        g.disable(gl::ALPHA_TEST);
        g.shade_model(gl::SMOOTH);
        g.disable(gl::TEXTURE_2D);
        g.start_drawing_quads();
        g.set_color_rgba_i(0, 0);
        g.add_vertex_with_uv(l as f64, (t + var18) as f64, 0.0, 0.0, 1.0);
        g.add_vertex_with_uv(r as f64, (t + var18) as f64, 0.0, 1.0, 1.0);
        g.set_color_rgba_i(0, 255);
        g.add_vertex_with_uv(r as f64, t as f64, 0.0, 1.0, 0.0);
        g.add_vertex_with_uv(l as f64, t as f64, 0.0, 0.0, 0.0);
        g.draw();
        g.start_drawing_quads();
        g.set_color_rgba_i(0, 255);
        g.add_vertex_with_uv(l as f64, b as f64, 0.0, 0.0, 1.0);
        g.add_vertex_with_uv(r as f64, b as f64, 0.0, 1.0, 1.0);
        g.set_color_rgba_i(0, 0);
        g.add_vertex_with_uv(r as f64, (b - var18) as f64, 0.0, 1.0, 0.0);
        g.add_vertex_with_uv(l as f64, (b - var18) as f64, 0.0, 0.0, 0.0);
        g.draw();
        let content = list.get_content_height(self);
        let var19 = content - (self.bottom - self.top - 4);
        if var19 > 0 {
            let var13 = self.thumb_height(content, false);
            let mut var14 = scrolled * (self.bottom - self.top - var13) / var19 + self.top;
            if var14 < self.top {
                var14 = self.top;
            }
            g.start_drawing_quads();
            g.set_color_rgba_i(0, 255);
            g.add_vertex_with_uv(var5 as f64, b as f64, 0.0, 0.0, 1.0);
            g.add_vertex_with_uv(var6 as f64, b as f64, 0.0, 1.0, 1.0);
            g.add_vertex_with_uv(var6 as f64, t as f64, 0.0, 1.0, 0.0);
            g.add_vertex_with_uv(var5 as f64, t as f64, 0.0, 0.0, 0.0);
            g.draw();
            g.start_drawing_quads();
            g.set_color_rgba_i(8421504, 255);
            g.add_vertex_with_uv(var5 as f64, (var14 + var13) as f64, 0.0, 0.0, 1.0);
            g.add_vertex_with_uv(var6 as f64, (var14 + var13) as f64, 0.0, 1.0, 1.0);
            g.add_vertex_with_uv(var6 as f64, var14 as f64, 0.0, 1.0, 0.0);
            g.add_vertex_with_uv(var5 as f64, var14 as f64, 0.0, 0.0, 0.0);
            g.draw();
            g.start_drawing_quads();
            g.set_color_rgba_i(12632256, 255);
            g.add_vertex_with_uv(var5 as f64, (var14 + var13 - 1) as f64, 0.0, 0.0, 1.0);
            g.add_vertex_with_uv((var6 - 1) as f64, (var14 + var13 - 1) as f64, 0.0, 1.0, 1.0);
            g.add_vertex_with_uv((var6 - 1) as f64, var14 as f64, 0.0, 1.0, 0.0);
            g.add_vertex_with_uv(var5 as f64, var14 as f64, 0.0, 0.0, 0.0);
            g.draw();
        }
        list.post_draw(ctx, g, var1, var2);
        g.enable(gl::TEXTURE_2D);
        g.shade_model(gl::FLAT);
        g.enable(gl::ALPHA_TEST);
        g.disable(gl::BLEND);
    }

    /// `overlayBackground(y1, y2, alpha1, alpha2)`: a dirt band (tint
    /// `0x404040`) over the full width.
    // src: GuiSlot.java:304-317
    fn overlay_background(&self, g: &mut GuiGl, var1: i32, var2: i32, var3: i32, var4: i32) {
        g.bind_texture(BACKGROUND_PNG);
        g.color4f(1.0, 1.0, 1.0, 1.0);
        let var6 = 32.0f32;
        let w = self.width;
        g.start_drawing_quads();
        g.set_color_rgba_i(4210752, var4);
        g.add_vertex_with_uv(0.0, var2 as f64, 0.0, 0.0, (var2 as f32 / var6) as f64);
        g.add_vertex_with_uv(
            w as f64,
            var2 as f64,
            0.0,
            (w as f32 / var6) as f64,
            (var2 as f32 / var6) as f64,
        );
        g.set_color_rgba_i(4210752, var3);
        g.add_vertex_with_uv(
            w as f64,
            var1 as f64,
            0.0,
            (w as f32 / var6) as f64,
            (var1 as f32 / var6) as f64,
        );
        g.add_vertex_with_uv(0.0, var1 as f64, 0.0, 0.0, (var1 as f32 / var6) as f64);
        g.draw();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_clamps() {
        // 240 high screen, list 32..176 (144 px), rows of 36.
        let mut s = GuiSlot::new(427, 240, 32, 176, 36);
        // 10 rows: 360 content, 360 - 140 = 220 of scroll.
        s.amount_scrolled = 500.0;
        s.bind_amount_scrolled(360);
        assert_eq!(s.amount_scrolled, 220.0);
        s.amount_scrolled = -5.0;
        s.bind_amount_scrolled(360);
        assert_eq!(s.amount_scrolled, 0.0);
        // 2 rows (72): (72 - 140) / 2 = -34, checked after the clamp to 0:
        // the short list is centred.
        s.amount_scrolled = 10.0;
        s.bind_amount_scrolled(72);
        assert_eq!(s.amount_scrolled, -34.0);
        s.amount_scrolled = -100.0;
        s.bind_amount_scrolled(72);
        assert_eq!(s.amount_scrolled, -34.0);
    }

    #[test]
    fn thumb_and_rows() {
        let mut s = GuiSlot::new(427, 240, 32, 176, 36);
        // 144² / 360 = 57.6: 57 either way.
        assert_eq!(s.thumb_height(360, false), 57);
        assert_eq!(s.thumb_height(360, true), 57);
        // Short content: at most bottom - top - 8.
        assert_eq!(s.thumb_height(100, false), 136);
        // Very long: at least 32.
        assert_eq!(s.thumb_height(100_000, false), 32);
        // Rows: y = top + 4 + 36·i; x within width/2 ± 110.
        assert_eq!(s.get_slot_index_from_screen_coords(10, 213, 36), 0);
        assert_eq!(s.get_slot_index_from_screen_coords(10, 213, 72), 1);
        assert_eq!(s.get_slot_index_from_screen_coords(10, 213, 35), -1);
        assert_eq!(s.get_slot_index_from_screen_coords(10, 400, 40), -1);
        assert_eq!(s.get_slot_index_from_screen_coords(1, 213, 72), -1);
        s.amount_scrolled = 36.0;
        assert_eq!(s.get_slot_index_from_screen_coords(10, 213, 36), 1);
        s.set_has_list_header(true, 20);
        assert_eq!(s.header_padding, 20);
        s.set_has_list_header(false, 20);
        assert_eq!(s.header_padding, 0);
    }
}
