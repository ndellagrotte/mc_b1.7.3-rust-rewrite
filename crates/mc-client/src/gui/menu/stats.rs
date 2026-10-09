//! `GuiStats` (GuiStats.java) and its three lists: `GuiSlotStatsGeneral`
//! (the general stats, 10-pixel rows), and the two `GuiSlotStats` tables
//! `GuiSlotStatsBlock` (crafted / used / mined per block) and
//! `GuiSlotStatsItem` (depleted / crafted / used per item), 20-pixel rows
//! under a header of column icons. Clicking a column icon sorts by it
//! (descending, then ascending, then back to the id order:
//! `func_27266_c`, `SorterStatsBlock`, `SorterStatsItem`); hovering an icon
//! names the column, hovering a row's item names the item.
//!
//! Values come from `mc.statFileWriter` (`ctx.game.stats_mut()`) in each
//! stat's format (`StatBase.func_27084_a`: grouped integers, times,
//! distances). The block and item lists hold the stats with a non-zero
//! value in any of their three columns, as of `initGui`.
//!
//! The rows' tooltips (`func_27257_b`) are drawn right after the list
//! returns rather than at the end of `GuiSlot.drawScreen`: the same draw
//! order (after the bands and the scroll bar, before the title and the
//! buttons), only the GL state they start from differs (blending is off
//! again; the tooltip's gradient and font set what they draw with).

use mc_sim::block::BlockStatics;
use mc_sim::stats::{StatBase, StatFileWriter, StatList};

use super::{display_parent, stat_name};
use crate::app::ScreenKind;
use crate::gui::batch::{GuiGl, gl};
use crate::gui::draw;
use crate::gui::render_item::{ItemGl, draw_item_into_gui};
use crate::gui::screen::{
    Screen, ScreenBase, ScreenCtx, default_draw_screen, draw_default_background,
};
use crate::gui::widgets::button::GuiButton;
use crate::gui::widgets::slot::{GuiSlot, SlotList};

/// `/gui/slot.png`: the item frame and the column icons.
// src: GuiStats.java:91
pub const SLOT_PNG: &str = "/gui/slot.png";

/// The row colours: even rows white, odd rows `9474192` (`0x909090`).
// src: GuiSlotStatsGeneral.java:34, GuiSlotStats.java:110
fn row_color(var1: i32) -> i32 {
    if var1 % 2 == 0 { 16777215 } else { 9474192 }
}

/// `GuiStats.drawSprite(x, y, u, v)`: an 18 × 18 piece of `/gui/slot.png`
/// (a 128-pixel sheet: `* 0.0078125`).
// src: GuiStats.java:90-101
#[allow(clippy::identity_op)]
pub fn draw_sprite(g: &mut GuiGl, z_level: f32, var1: i32, var2: i32, var3: i32, var4: i32) {
    g.color4f(1.0, 1.0, 1.0, 1.0);
    g.bind_texture(SLOT_PNG);
    let z = z_level as f64;
    let s = 0.0078125f32;
    g.start_drawing_quads();
    g.add_vertex_with_uv(
        (var1 + 0) as f64,
        (var2 + 18) as f64,
        z,
        ((var3 + 0) as f32 * s) as f64,
        ((var4 + 18) as f32 * s) as f64,
    );
    g.add_vertex_with_uv(
        (var1 + 18) as f64,
        (var2 + 18) as f64,
        z,
        ((var3 + 18) as f32 * s) as f64,
        ((var4 + 18) as f32 * s) as f64,
    );
    g.add_vertex_with_uv(
        (var1 + 18) as f64,
        (var2 + 0) as f64,
        z,
        ((var3 + 18) as f32 * s) as f64,
        ((var4 + 0) as f32 * s) as f64,
    );
    g.add_vertex_with_uv(
        (var1 + 0) as f64,
        (var2 + 0) as f64,
        z,
        ((var3 + 0) as f32 * s) as f64,
        ((var4 + 0) as f32 * s) as f64,
    );
    g.draw();
}

/// `GuiStats.drawItemSprite(x, y, id)`: the slot frame, then the item's
/// icon (`getIconFromDamage(0)`) 2 pixels in.
// src: GuiStats.java:74-88
pub fn draw_item_sprite(
    g: &mut GuiGl,
    statics: &BlockStatics,
    z_level: f32,
    var1: i32,
    var2: i32,
    var3: i32,
) {
    draw_sprite(g, z_level, var1 + 1, var2 + 1, 0, 0);
    let mut ig = ItemGl::new();
    g.enable(gl::RESCALE_NORMAL);
    ig.rescale_normal = true;
    ig.push_matrix(g);
    ig.rotatef(g, 180.0, 1.0, 0.0, 0.0);
    ig.enable_standard_item_lighting(g);
    ig.pop_matrix(g);
    // GuiStats.renderItem (a static new RenderItem(): field_27004_a true).
    let icon = mc_sim::item::item(var3)
        .map(|i| i.get_icon_from_damage(0))
        .unwrap_or(-1);
    draw_item_into_gui(&mut ig, g, statics, var3, 0, icon, var1 + 2, var2 + 2);
    ig.disable_standard_item_lighting(g);
    g.disable(gl::RESCALE_NORMAL);
}

/// Which `GuiSlotStats` subclass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatsTable {
    /// `GuiSlotStatsBlock`: crafted, used, mined.
    Block,
    /// `GuiSlotStatsItem`: depleted, crafted, used.
    Item,
}

/// The stats of a table row's columns, by its `getItemID()`: the column
/// order of `drawSlot` (the third is the row's own stat).
// src: GuiSlotStatsBlock.java:58-63, GuiSlotStatsItem.java:58-63
fn columns(t: StatsTable, var7: i32) -> [Option<&'static StatBase>; 3] {
    let l = StatList::get();
    match t {
        StatsTable::Block => [
            l.object_craft_stat(var7),
            l.object_use_stat(var7),
            l.mine_block_stat(var7),
        ],
        StatsTable::Item => [
            l.object_break_stat(var7),
            l.object_craft_stat(var7),
            l.object_use_stat(var7),
        ],
    }
}

/// The comparator's stats for sort column `col` (`field_27271_e`):
/// `SorterStatsBlock` reads `mineBlockStatArray` / `objectCraftStats` /
/// `objectUseStats`, `SorterStatsItem` `objectBreakStats` /
/// `objectCraftStats` / `objectUseStats` (the drawn columns).
// src: SorterStatsBlock.java:21-30, SorterStatsItem.java:21-30
fn sort_stat(t: StatsTable, col: i32, id: i32) -> Option<&'static StatBase> {
    let l = StatList::get();
    match (t, col) {
        (StatsTable::Block, 2) => l.mine_block_stat(id),
        (StatsTable::Block, 0) => l.object_craft_stat(id),
        (StatsTable::Block, 1) => l.object_use_stat(id),
        (StatsTable::Item, 0) => l.object_break_stat(id),
        (StatsTable::Item, 1) => l.object_craft_stat(id),
        (StatsTable::Item, 2) => l.object_use_stat(id),
        _ => None,
    }
}

/// `SorterStatsBlock.func_27297_a` / `SorterStatsItem.func_27371_a`:
/// a row without the column's stat goes after one with it; two with it
/// order by value times the direction; otherwise by id.
// src: SorterStatsBlock.java:16-49, SorterStatsItem.java:16-49
pub fn compare_rows(
    t: StatsTable,
    col: i32,
    dir: i32,
    w: &StatFileWriter,
    var1: &StatBase,
    var2: &StatBase,
) -> i32 {
    let var3 = var1.get_item_id().unwrap_or(0);
    let var4 = var2.get_item_id().unwrap_or(0);
    let var5 = sort_stat(t, col, var3);
    let var6 = sort_stat(t, col, var4);
    if var5.is_some() || var6.is_some() {
        let Some(var5) = var5 else {
            return 1;
        };
        let Some(var6) = var6 else {
            return -1;
        };
        let var7 = w.write_stat(var5);
        let var8 = w.write_stat(var6);
        if var7 != var8 {
            return var7.wrapping_sub(var8).wrapping_mul(dir);
        }
    }
    var3.wrapping_sub(var4)
}

/// `GuiSlotStats`' fields and its subclass.
#[derive(Clone, Debug)]
pub struct StatsSlot {
    /// Block or item.
    pub table: StatsTable,
    /// `field_27268_b`: the header column pressed (-1 none).
    pub pressed_column: i32,
    /// `field_27273_c`: the rows.
    pub rows: Vec<&'static StatBase>,
    /// `field_27271_e`: the sort column (-1: id order).
    pub sort_column: i32,
    /// `field_27270_f`: the sort direction (-1 descending, 1 ascending,
    /// 0 none).
    pub sort_direction: i32,
    /// The screen's size (`drawDefaultBackground`).
    width: i32,
    height: i32,
}

impl StatsSlot {
    /// `new GuiSlotStatsBlock(gui)` / `new GuiSlotStatsItem(gui)`: every
    /// mine-block (resp. item-use) stat with a non-zero value in any of the
    /// row's three columns.
    // src: GuiSlotStatsBlock.java:13-36, GuiSlotStatsItem.java:13-36,
    // GuiSlotStats.java:17-25
    pub fn new(table: StatsTable, w: &StatFileWriter, width: i32, height: i32) -> Self {
        let l = StatList::get();
        let source: Vec<&'static StatBase> = match table {
            StatsTable::Block => l.object_mine_stats().collect(),
            StatsTable::Item => l.item_stats().collect(),
        };
        let mut rows = Vec::new();
        for var3 in source {
            let var5 = var3.get_item_id().unwrap_or(0);
            let (b, c) = match table {
                StatsTable::Block => (l.object_use_stat(var5), l.object_craft_stat(var5)),
                StatsTable::Item => (l.object_break_stat(var5), l.object_craft_stat(var5)),
            };
            let var4 = w.write_stat(var3) > 0
                || b.is_some_and(|s| w.write_stat(s) > 0)
                || c.is_some_and(|s| w.write_stat(s) > 0);
            if var4 {
                rows.push(var3);
            }
        }
        StatsSlot {
            table,
            pressed_column: -1,
            rows,
            sort_column: -1,
            sort_direction: 0,
            width,
            height,
        }
    }

    /// `new GuiSlot(…, 32, height - 64, 20)` with the 20-pixel header and
    /// no selection box.
    // src: GuiSlotStats.java:17-25
    pub fn new_slot(width: i32, height: i32) -> GuiSlot {
        let mut s = GuiSlot::new(width, height, 32, height - 64, 20);
        s.set_show_selection_box(false);
        s.set_has_list_header(true, 20);
        s
    }

    /// `func_27263_a(column)`: the column's translation key.
    // src: GuiSlotStatsBlock.java:65-71, GuiSlotStatsItem.java:65-71
    pub fn column_key(&self, var1: i32) -> &'static str {
        match self.table {
            StatsTable::Block => match var1 {
                0 => "stat.crafted",
                1 => "stat.used",
                _ => "stat.mined",
            },
            StatsTable::Item => match var1 {
                1 => "stat.crafted",
                2 => "stat.used",
                _ => "stat.depleted",
            },
        }
    }

    /// `func_27266_c(column)` and the sort.
    // src: GuiSlotStats.java:171-183
    pub fn sort_by(&mut self, var1: i32, w: &StatFileWriter) {
        if var1 != self.sort_column {
            self.sort_column = var1;
            self.sort_direction = -1;
        } else if self.sort_direction == -1 {
            self.sort_direction = 1;
        } else {
            self.sort_column = -1;
            self.sort_direction = 0;
        }
        let (t, col, dir) = (self.table, self.sort_column, self.sort_direction);
        self.rows
            .sort_by(|a, b| compare_rows(t, col, dir, w, a, b).cmp(&0));
    }

    /// `func_27265_a(stat, x, y, even)`: the value right-aligned at `x`, or
    /// `-` for a column the row has no stat for.
    // src: GuiSlotStats.java:106-116
    fn draw_value(
        ctx: &mut ScreenCtx,
        g: &mut GuiGl,
        var1: Option<&StatBase>,
        var2: i32,
        var3: i32,
        var4: bool,
    ) {
        let c = if var4 { 16777215 } else { 9474192 };
        let var5 = match var1 {
            Some(s) => {
                let v = ctx.game.stats_mut().write_stat(s);
                s.func_27084_a(v)
            }
            None => "-".to_string(),
        };
        let f = ctx.font;
        draw::draw_string(g, f, &var5, var2 - f.get_string_width(&var5), var3 + 5, c);
    }

    /// `func_27257_b(mouseX, mouseY)`: the item's name over a row's icon,
    /// the column's name over a header icon.
    // src: GuiSlotStats.java:118-169
    pub fn draw_tooltip(
        &self,
        slot: &GuiSlot,
        ctx: &mut ScreenCtx,
        g: &mut GuiGl,
        var1: i32,
        var2: i32,
    ) {
        if var2 < slot.top || var2 > slot.bottom {
            return;
        }
        let var3 = slot.get_slot_index_from_screen_coords(self.rows.len() as i32, var1, var2);
        let var4 = self.width / 2 - 92 - 16;
        let var5 = if var3 >= 0 {
            if var1 < var4 + 40 || var1 > var4 + 40 + 20 {
                return;
            }
            // func_27267_a: translateNamedKey(item.getItemName()).
            let id = self.rows[var3 as usize].get_item_id().unwrap_or(0);
            let name = mc_sim::item::item(id).and_then(|i| i.get_item_name());
            ctx.lang
                .translate_named_key(name.as_deref().unwrap_or("null"))
        } else {
            let key = if var1 >= var4 + 115 - 18 && var1 <= var4 + 115 {
                self.column_key(0)
            } else if var1 >= var4 + 165 - 18 && var1 <= var4 + 165 {
                self.column_key(1)
            } else {
                if var1 < var4 + 215 - 18 || var1 > var4 + 215 {
                    return;
                }
                self.column_key(2)
            };
            ctx.lang.translate_key(key)
        };
        let var5 = crate::gui::font::java_trim(&var5).to_string();
        if !var5.is_empty() {
            let var6 = var1 + 12;
            let var7 = var2 - 12;
            let f = ctx.font;
            let var8 = f.get_string_width(&var5);
            draw::draw_gradient_rect(
                g,
                var6 - 3,
                var7 - 3,
                var6 + var8 + 3,
                var7 + 8 + 3,
                -1073741824,
                -1073741824,
            );
            f.draw_string_with_shadow(g, &var5, var6, var7, -1);
        }
    }
}

impl SlotList for StatsSlot {
    // src: GuiSlotStats.java:96-98
    fn get_size(&self) -> i32 {
        self.rows.len() as i32
    }
    // src: GuiSlotStats.java:27-28
    fn element_clicked(&mut self, _ctx: &mut ScreenCtx, _index: i32, _double: bool) {}
    // src: GuiSlotStats.java:30-32
    fn is_selected(&self, _index: i32) -> bool {
        false
    }
    // src: GuiSlotStats.java:34-36
    fn draw_background(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl) {
        let b = ScreenBase {
            width: self.width,
            height: self.height,
            ..ScreenBase::default()
        };
        draw_default_background(&b, ctx, g);
    }

    // src: GuiSlotStatsBlock.java:56-64, GuiSlotStatsItem.java:56-64
    fn draw_slot(
        &mut self,
        ctx: &mut ScreenCtx,
        g: &mut GuiGl,
        var1: i32,
        var2: i32,
        var3: i32,
        _h: i32,
    ) {
        let var6 = self.rows[var1 as usize];
        let var7 = var6.get_item_id().unwrap_or(0);
        {
            let default_statics = BlockStatics::default();
            let statics = ctx
                .game
                .the_world
                .as_ref()
                .map_or(&default_statics, |w| &w.block_statics);
            draw_item_sprite(g, statics, 0.0, var2 + 40, var3, var7);
        }
        let [a, b, c] = columns(self.table, var7);
        let even = var1 % 2 == 0;
        Self::draw_value(ctx, g, a, var2 + 115, var3, even);
        Self::draw_value(ctx, g, b, var2 + 165, var3, even);
        Self::draw_value(ctx, g, c, var2 + 215, var3, even);
    }

    // src: GuiSlotStats.java:38-77, GuiSlotStatsBlock.java:38-54,
    // GuiSlotStatsItem.java:38-54
    fn draw_list_header(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, var1: i32, var2: i32) {
        if !ctx.game.input.is_button_down(0) {
            self.pressed_column = -1;
        }
        for (i, x) in [115, 165, 215].into_iter().enumerate() {
            let v = if self.pressed_column == i as i32 {
                0
            } else {
                18
            };
            draw_sprite(g, 0.0, var1 + x - 18, var2 + 1, 0, v);
        }
        if self.sort_column != -1 {
            let mut var4 = 79;
            let mut var5 = 18;
            if self.sort_column == 1 {
                var4 = 129;
            } else if self.sort_column == 2 {
                var4 = 179;
            }
            if self.sort_direction == 1 {
                var5 = 36;
            }
            draw_sprite(g, 0.0, var1 + var4, var2 + 1, var5, 0);
        }
        let icons: [i32; 3] = match self.table {
            StatsTable::Block => [18, 36, 54],
            StatsTable::Item => [72, 18, 36],
        };
        for (i, (x, u)) in [115, 165, 215].into_iter().zip(icons).enumerate() {
            if self.pressed_column == i as i32 {
                draw_sprite(g, 0.0, var1 + x - 18 + 1, var2 + 1 + 1, u, 18);
            } else {
                draw_sprite(g, 0.0, var1 + x - 18, var2 + 1, u, 18);
            }
        }
    }

    // src: GuiSlotStats.java:79-94
    fn header_clicked(&mut self, ctx: &mut ScreenCtx, var1: i32, _var2: i32) {
        self.pressed_column = -1;
        if (79..115).contains(&var1) {
            self.pressed_column = 0;
        } else if (129..165).contains(&var1) {
            self.pressed_column = 1;
        } else if (179..215).contains(&var1) {
            self.pressed_column = 2;
        }
        if self.pressed_column >= 0 {
            let w: &StatFileWriter = ctx.game.stats_mut();
            self.sort_by(self.pressed_column, w);
            ctx.play_sound_fx("random.click", 1.0, 1.0);
        }
    }
}

/// `GuiSlotStatsGeneral`: the general stats, name left and value right.
#[derive(Clone, Copy, Debug)]
pub struct GeneralSlot {
    width: i32,
    height: i32,
}

impl GeneralSlot {
    /// `new GuiSlot(…, 32, height - 64, 10)`, no selection box.
    // src: GuiSlotStatsGeneral.java:6-10
    pub fn new_slot(width: i32, height: i32) -> GuiSlot {
        let mut s = GuiSlot::new(width, height, 32, height - 64, 10);
        s.set_show_selection_box(false);
        s
    }
}

impl SlotList for GeneralSlot {
    // src: GuiSlotStatsGeneral.java:12-14
    fn get_size(&self) -> i32 {
        StatList::get().general_stats().count() as i32
    }
    // src: GuiSlotStatsGeneral.java:16-17
    fn element_clicked(&mut self, _ctx: &mut ScreenCtx, _index: i32, _double: bool) {}
    // src: GuiSlotStatsGeneral.java:19-21
    fn is_selected(&self, _index: i32) -> bool {
        false
    }
    // src: GuiSlotStatsGeneral.java:23-25
    fn get_content_height(&self, _slot: &GuiSlot) -> i32 {
        self.get_size() * 10
    }
    // src: GuiSlotStatsGeneral.java:27-29
    fn draw_background(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl) {
        let b = ScreenBase {
            width: self.width,
            height: self.height,
            ..ScreenBase::default()
        };
        draw_default_background(&b, ctx, g);
    }
    // src: GuiSlotStatsGeneral.java:31-36
    fn draw_slot(
        &mut self,
        ctx: &mut ScreenCtx,
        g: &mut GuiGl,
        var1: i32,
        var2: i32,
        var3: i32,
        _h: i32,
    ) {
        let Some(var6) = StatList::get().general_stats().nth(var1 as usize) else {
            return;
        };
        let f = ctx.font;
        let c = row_color(var1);
        draw::draw_string(g, f, &stat_name(ctx.lang, var6), var2 + 2, var3 + 1, c);
        let v = ctx.game.stats_mut().write_stat(var6);
        let var7 = var6.func_27084_a(v);
        draw::draw_string(
            g,
            f,
            &var7,
            var2 + 2 + 213 - f.get_string_width(&var7),
            var3 + 1,
            c,
        );
    }
}

/// `GuiStats.selectedSlot`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    /// `slotGeneral`.
    General,
    /// `slotItem`.
    Item,
    /// `slotBlock`.
    Block,
}

/// `GuiStats`.
pub struct GuiStats {
    base: ScreenBase,
    /// `parentGui`.
    parent_gui: Option<Box<dyn Screen>>,
    /// `statsTitle`.
    pub stats_title: String,
    general: Option<(GuiSlot, GeneralSlot)>,
    item: Option<(GuiSlot, StatsSlot)>,
    block: Option<(GuiSlot, StatsSlot)>,
    /// `selectedSlot`.
    pub selected_slot: Tab,
}

impl GuiStats {
    /// `new GuiStats(parent, mc.statFileWriter)`.
    // src: GuiStats.java:16-19
    pub fn new(parent: Option<Box<dyn Screen>>) -> Self {
        GuiStats {
            base: ScreenBase::default(),
            parent_gui: parent,
            stats_title: "Select world".to_string(),
            general: None,
            item: None,
            block: None,
            selected_slot: Tab::General,
        }
    }

    /// The rows of the item and block tables (tests, dumps).
    pub fn table(&self, t: StatsTable) -> Option<&StatsSlot> {
        match t {
            StatsTable::Item => self.item.as_ref().map(|s| &s.1),
            StatsTable::Block => self.block.as_ref().map(|s| &s.1),
        }
    }

    /// `addHeaderButtons()`.
    // src: GuiStats.java:33-49
    fn add_header_buttons(&mut self, ctx: &mut ScreenCtx) {
        let var1 = ctx.lang;
        let (w, h) = (self.base.width, self.base.height);
        let c = &mut self.base.control_list;
        c.push(GuiButton::with_size(
            0,
            w / 2 + 4,
            h - 28,
            150,
            20,
            var1.translate_key("gui.done"),
        ));
        c.push(GuiButton::with_size(
            1,
            w / 2 - 154,
            h - 52,
            100,
            20,
            var1.translate_key("stat.generalButton"),
        ));
        let mut var2 = GuiButton::with_size(
            2,
            w / 2 - 46,
            h - 52,
            100,
            20,
            var1.translate_key("stat.blocksButton"),
        );
        let mut var3 = GuiButton::with_size(
            3,
            w / 2 + 62,
            h - 52,
            100,
            20,
            var1.translate_key("stat.itemsButton"),
        );
        if self.block.as_ref().is_some_and(|b| b.1.get_size() == 0) {
            var2.enabled = false;
        }
        if self.item.as_ref().is_some_and(|b| b.1.get_size() == 0) {
            var3.enabled = false;
        }
        c.push(var2);
        c.push(var3);
    }
}

impl Screen for GuiStats {
    fn base(&self) -> &ScreenBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn kind(&self) -> ScreenKind {
        ScreenKind::Other("GuiStats".into())
    }

    // The selected tab's list, `selectedSlot` (GuiStats.java:55-61).
    fn gui_slot(&self) -> Option<(&GuiSlot, i32)> {
        match self.selected_slot {
            Tab::General => self.general.as_ref().map(|(s, l)| (s, l.get_size())),
            Tab::Item => self.item.as_ref().map(|(s, l)| (s, l.get_size())),
            Tab::Block => self.block.as_ref().map(|(s, l)| (s, l.get_size())),
        }
    }

    // src: GuiStats.java:21-31
    fn init_gui(&mut self, ctx: &mut ScreenCtx) {
        self.stats_title = ctx.lang.translate_key("gui.stats");
        let (w, h) = (self.base.width, self.base.height);
        let writer: &StatFileWriter = ctx.game.stats_mut();
        let mut g = GeneralSlot::new_slot(w, h);
        g.register_scroll_buttons(1, 1);
        self.general = Some((
            g,
            GeneralSlot {
                width: w,
                height: h,
            },
        ));
        let mut i = StatsSlot::new_slot(w, h);
        i.register_scroll_buttons(1, 1);
        self.item = Some((i, StatsSlot::new(StatsTable::Item, writer, w, h)));
        let mut b = StatsSlot::new_slot(w, h);
        b.register_scroll_buttons(1, 1);
        self.block = Some((b, StatsSlot::new(StatsTable::Block, writer, w, h)));
        self.selected_slot = Tab::General;
        self.add_header_buttons(ctx);
    }

    // src: GuiStats.java:51-66
    fn action_performed(&mut self, ctx: &mut ScreenCtx, button: usize) {
        let b = &self.base.control_list[button];
        if !b.enabled {
            return;
        }
        match b.id {
            0 => display_parent(ctx, &mut self.parent_gui),
            1 => self.selected_slot = Tab::General,
            3 => self.selected_slot = Tab::Item,
            2 => self.selected_slot = Tab::Block,
            _ => {
                let b = b.clone();
                match self.selected_slot {
                    Tab::General => {
                        if let Some((s, l)) = self.general.as_mut() {
                            s.action_performed(l, &b);
                        }
                    }
                    Tab::Item => {
                        if let Some((s, l)) = self.item.as_mut() {
                            s.action_performed(l, &b);
                        }
                    }
                    Tab::Block => {
                        if let Some((s, l)) = self.block.as_mut() {
                            s.action_performed(l, &b);
                        }
                    }
                }
            }
        }
    }

    // src: GuiStats.java:68-72
    fn draw_screen(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, mx: i32, my: i32, p: f32) {
        match self.selected_slot {
            Tab::General => {
                if let Some((s, l)) = self.general.as_mut() {
                    s.draw_screen(l, ctx, g, mx, my, p);
                }
            }
            Tab::Item => {
                if let Some((s, l)) = self.item.as_mut() {
                    s.draw_screen(l, ctx, g, mx, my, p);
                    l.draw_tooltip(s, ctx, g, mx, my);
                }
            }
            Tab::Block => {
                if let Some((s, l)) = self.block.as_mut() {
                    s.draw_screen(l, ctx, g, mx, my, p);
                    l.draw_tooltip(s, ctx, g, mx, my);
                }
            }
        }
        draw::draw_centered_string(
            g,
            ctx.font,
            &self.stats_title,
            self.base.width / 2,
            20,
            16777215,
        );
        default_draw_screen(&mut self.base, ctx, g, mx, my);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mc_sim::stats::StatFileWriter;

    fn writer(values: &[(i32, i32)]) -> StatFileWriter {
        let mut w = StatFileWriter::default();
        let mut m = std::collections::BTreeMap::new();
        for &(k, v) in values {
            m.insert(k, v);
        }
        w.func_27179_a(Some(&m));
        w
    }

    #[test]
    fn block_table_filters_and_sorts_by_column() {
        let l = StatList::get();
        let stone = l.mine_block_stat(1).unwrap();
        let dirt = l.mine_block_stat(3).unwrap();
        let cobble = l.mine_block_stat(4).unwrap();
        let w = writer(&[(stone.stat_id, 5), (dirt.stat_id, 20), (cobble.stat_id, 5)]);
        let mut t = StatsSlot::new(StatsTable::Block, &w, 427, 240);
        let ids = |t: &StatsSlot| -> Vec<i32> {
            t.rows.iter().map(|s| s.get_item_id().unwrap()).collect()
        };
        // Only the blocks with a value; objectMineStats order (by id).
        assert_eq!(ids(&t), vec![1, 3, 4]);
        // Mined column, first click: descending; ties by id.
        t.sort_by(2, &w);
        assert_eq!((t.sort_column, t.sort_direction), (2, -1));
        assert_eq!(ids(&t), vec![3, 1, 4]);
        // Second click: ascending.
        t.sort_by(2, &w);
        assert_eq!(t.sort_direction, 1);
        assert_eq!(ids(&t), vec![1, 4, 3]);
        // Third: back to the id order.
        t.sort_by(2, &w);
        assert_eq!((t.sort_column, t.sort_direction), (-1, 0));
        assert_eq!(ids(&t), vec![1, 3, 4]);
        // Another column: descending again.
        t.sort_by(0, &w);
        assert_eq!((t.sort_column, t.sort_direction), (0, -1));
        assert_eq!(t.rows.len(), 3);
    }

    #[test]
    fn rows_without_the_stat_sort_last() {
        let l = StatList::get();
        // Planks (5) have a craft stat, dirt (3) none.
        let planks = l.mine_block_stat(5).unwrap();
        let dirt = l.mine_block_stat(3).unwrap();
        let w = StatFileWriter::default();
        assert!(l.object_craft_stat(5).is_some());
        assert!(l.object_craft_stat(3).is_none());
        assert_eq!(compare_rows(StatsTable::Block, 0, -1, &w, dirt, planks), 1);
        assert_eq!(compare_rows(StatsTable::Block, 0, -1, &w, planks, dirt), -1);
        // No column: by id.
        assert_eq!(compare_rows(StatsTable::Block, -1, 0, &w, dirt, planks), -2);
    }

    #[test]
    fn column_keys() {
        let w = StatFileWriter::default();
        let b = StatsSlot::new(StatsTable::Block, &w, 427, 240);
        let i = StatsSlot::new(StatsTable::Item, &w, 427, 240);
        assert!(b.rows.is_empty() && i.rows.is_empty());
        assert_eq!(
            (0..3).map(|c| b.column_key(c)).collect::<Vec<_>>(),
            vec!["stat.crafted", "stat.used", "stat.mined"]
        );
        assert_eq!(
            (0..3).map(|c| i.column_key(c)).collect::<Vec<_>>(),
            vec!["stat.depleted", "stat.crafted", "stat.used"]
        );
    }

    #[test]
    fn value_formats() {
        let l = StatList::get();
        let jump = l
            .general_stats()
            .find(|s| s.stat_name.key == "stat.jump")
            .unwrap();
        assert_eq!(jump.func_27084_a(1234567), "1,234,567");
        let walk = l
            .general_stats()
            .find(|s| s.stat_name.key == "stat.walkOneCm")
            .unwrap();
        assert_eq!(walk.func_27084_a(150000), "1.50 km");
        let play = l
            .general_stats()
            .find(|s| s.stat_name.key == "stat.playOneMinute")
            .unwrap();
        assert_eq!(play.func_27084_a(20 * 90), "1.50 m");
    }
}
