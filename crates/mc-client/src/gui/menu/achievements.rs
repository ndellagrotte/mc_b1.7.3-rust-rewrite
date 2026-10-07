//! `GuiAchievements` (GuiAchievements.java): the achievement map — a
//! 224 × 155 window, dragged with the left button, over a background of
//! terrain tiles chosen per column with a seeded `Random` (sand at the top,
//! dirt, stone with coal, iron, diamond and redstone ores further down,
//! bedrock at the bottom), the lines to each achievement's parent, the
//! icons in their frames (lit when taken, blinking when they can be taken,
//! dark otherwise), the `/achievement/bg.png` frame and the tooltip.
//!
//! # The window's clipping
//!
//! Vanilla clips the lines and icons to the tiled area with the depth
//! buffer: the background is written at z 0, the tiles at z −200 with
//! `glDepthFunc(GL_GEQUAL)` (so they pass and push the depth back), and
//! the lines and icons at z −200 with `GL_LEQUAL` pass only over the tiles
//! (GuiAchievements.java:152-200); the frame drawn last covers the tiles'
//! spill past the window. Here the lines, the icon frames and the icons
//! are cut to the tiled rectangle on the CPU instead ([`Clip`]), which
//! gives the same pixels for these axis-aligned quads, and the map is
//! drawn at z 0: WP-3.6a's GUI GL has no `glDepthFunc`, and its batches
//! are ordered by Bevy's transparent sort, which also weighs each batch's
//! distance, so a batch at z −200 would be drawn before the background
//! (reported).
//!
//! # Not ported
//!
//! `RenderHelper.enableStandardItemLighting` / `GL_RESCALE_NORMAL` /
//! `GL_COLOR_MATERIAL` / `GL_CULL_FACE`: the item icons' lighting belongs
//! to the GUI item renderer ([`super::draw_item_into_gui`], WP-3.6b).

use mc_core::java_random::JavaRandom;
use mc_core::jmath;
use mc_core::math::floor_double;
use mc_sim::stats::achievement_list::OPEN_INVENTORY;
use mc_sim::stats::{StatBase, StatFileWriter, StatList, stat};

use super::{Clip, draw_item_into_gui, draw_textured_rect_clipped, stat_name};
use crate::app::ScreenKind;
use crate::clock;
use crate::gui::batch::{GuiGl, gl};
use crate::gui::draw;
use crate::gui::screen::{
    Screen, ScreenBase, ScreenCtx, default_draw_screen, default_key_typed, draw_default_background,
};
use crate::gui::widgets::button::GuiButton;
use crate::lang::{StringTranslate, java_format};
use crate::settings::KEY_INVENTORY;

/// The pane and frame sheet.
// src: GuiAchievements.java:146
pub const BG_PNG: &str = "/achievement/bg.png";
/// `achievementsPaneWidth`.
// src: GuiAchievements.java:12
pub const PANE_WIDTH: i32 = 256;
/// `achievementsPaneHeight`.
// src: GuiAchievements.java:13
pub const PANE_HEIGHT: i32 = 202;
/// The map window's size inside the pane.
// src: GuiAchievements.java:63, :166, :170
pub const WINDOW_W: i32 = 224;
/// Its height.
pub const WINDOW_H: i32 = 155;

/// `Block.blockIndexInTexture` of the background's blocks (terrain.png
/// indices): sand, diamond ore, redstone ore, iron ore, coal ore, stone,
/// dirt, bedrock.
// src: Block.java:604 (sand 18), :648 (oreDiamond 50), :665 (oreRedstone
// 51), :607 (oreIron 33), :608 (oreCoal 34), :593 (stone 1), :595 (dirt
// 2), :599 (bedrock 17)
mod tile {
    pub const SAND: i32 = 18;
    pub const ORE_DIAMOND: i32 = 50;
    pub const ORE_REDSTONE: i32 = 51;
    pub const ORE_IRON: i32 = 33;
    pub const ORE_COAL: i32 = 34;
    pub const STONE: i32 = 1;
    pub const DIRT: i32 = 2;
    pub const BEDROCK: i32 = 17;
}

/// `X_MIN`, `Y_MIN`, `X_MAX`, `Y_MAX`: the scroll limits from
/// `AchievementList`'s display bounds.
// src: GuiAchievements.java:337-342
pub fn scroll_bounds() -> (i32, i32, i32, i32) {
    let l = StatList::get();
    (
        l.min_display_column * 24 - 112,
        l.min_display_row * 24 - 112,
        l.max_display_column * 24 - 77,
        l.max_display_row * 24 - 77,
    )
}

/// The background tile of map column `column` (`var12 + var24`) and row
/// `row` (`var13 + var22`): `rand.setSeed(1234 + column)`, one `nextInt()`,
/// then `nextInt(1 + row) + row / 2` picks the depth's block.
// src: GuiAchievements.java:171-193
pub fn background_tile(var21: &mut JavaRandom, column: i32, row: i32) -> i32 {
    var21.set_seed(1234i32.wrapping_add(column) as i64);
    var21.next_int();
    let var25 = var21.next_int_bounded(1 + row) + row / 2;
    let mut var26 = tile::SAND;
    if var25 <= 37 && row != 35 {
        if var25 == 22 {
            if var21.next_int_bounded(2) == 0 {
                var26 = tile::ORE_DIAMOND;
            } else {
                var26 = tile::ORE_REDSTONE;
            }
        } else if var25 == 10 {
            var26 = tile::ORE_IRON;
        } else if var25 == 8 {
            var26 = tile::ORE_COAL;
        } else if var25 > 4 {
            var26 = tile::STONE;
        } else if var25 > 0 {
            var26 = tile::DIRT;
        }
    } else {
        var26 = tile::BEDROCK;
    }
    var26
}

/// `Achievement.getDescription()`: the translated description, formatted
/// with the inventory key's name for "Taking Inventory"
/// (`StatStringFormatKeyInv`, set on `openInventory` by `startGame`).
// src: Achievement.java:65-67, StatStringFormatKeyInv.java:14-16,
// Minecraft.java:248
pub fn description(lang: &StringTranslate, s: &StatBase, inventory_key: i32) -> String {
    let Some(a) = s.achievement() else {
        return String::new();
    };
    let d = lang.translate_key(&a.achievement_description);
    if s.stat_id == OPEN_INVENTORY {
        let name = crate::keys::key_name(inventory_key).unwrap_or("null");
        java_format(&d, &[name.to_string()])
    } else {
        d
    }
}

/// `Math.sin((double)(System.currentTimeMillis() % 600L) / 600.0D * π * 2)`:
/// the blink of the achievements that can be taken.
// src: GuiAchievements.java:217, :252
fn blink(now_millis: i64) -> f64 {
    jmath::sin((now_millis % 600) as f64 / 600.0 * 3.141592653589793 * 2.0)
}

/// `GuiAchievements`.
pub struct GuiAchievements {
    base: ScreenBase,
    /// `mouseX`, `mouseY`: the last drag position.
    pub mouse_x: i32,
    /// `mouseY`.
    pub mouse_y: i32,
    /// `xScrollO`: the previous tick's scroll.
    pub x_scroll_o: f64,
    /// `yScrollO`.
    pub y_scroll_o: f64,
    /// `xScrollP`: the scroll.
    pub x_scroll_p: f64,
    /// `yScrollP`.
    pub y_scroll_p: f64,
    /// `xScrollTarget`.
    pub x_scroll_target: f64,
    /// `yScrollTarget`.
    pub y_scroll_target: f64,
    /// `isMouseButtonDown`: 0 up, 1 dragging the map, 2 pressed outside it
    /// (never set in Beta).
    is_mouse_button_down: i32,
}

impl Default for GuiAchievements {
    fn default() -> Self {
        Self::new()
    }
}

impl GuiAchievements {
    /// `new GuiAchievements(mc.statFileWriter)`: centred on "Taking
    /// Inventory" (the writer is read through `ctx.game.stats_mut()`).
    // src: GuiAchievements.java:25-31
    pub fn new() -> Self {
        let (col, row) = stat(OPEN_INVENTORY)
            .and_then(|s| s.achievement())
            .map(|a| (a.display_column, a.display_row))
            .unwrap_or((0, 0));
        let var2: i32 = 141;
        let var3: i32 = 141;
        let x = (col * 24 - var2 / 2 - 12) as f64;
        let y = (row * 24 - var3 / 2) as f64;
        GuiAchievements {
            base: ScreenBase::default(),
            mouse_x: 0,
            mouse_y: 0,
            x_scroll_o: x,
            y_scroll_o: y,
            x_scroll_p: x,
            y_scroll_p: y,
            x_scroll_target: x,
            y_scroll_target: y,
            is_mouse_button_down: 0,
        }
    }

    /// `updateScreen()`: the scroll moves 85 % of the way to its target,
    /// or onto it within 2 pixels.
    // src: GuiAchievements.java:105-118
    pub fn ease(&mut self) {
        self.x_scroll_o = self.x_scroll_p;
        self.y_scroll_o = self.y_scroll_p;
        let var1 = self.x_scroll_target - self.x_scroll_p;
        let var3 = self.y_scroll_target - self.y_scroll_p;
        if var1 * var1 + var3 * var3 < 4.0 {
            self.x_scroll_p += var1;
            self.y_scroll_p += var3;
        } else {
            self.x_scroll_p += var1 * 0.85;
            self.y_scroll_p += var3 * 0.85;
        }
    }

    /// The pane's top-left corner.
    fn pane(&self) -> (i32, i32) {
        (
            (self.base.width - PANE_WIDTH) / 2,
            (self.base.height - PANE_HEIGHT) / 2,
        )
    }

    /// `drawScreen`'s drag half.
    // src: GuiAchievements.java:58-94
    fn drag(&mut self, var1: i32, var2: i32, button_down: bool) {
        let (x_min, y_min, x_max, y_max) = scroll_bounds();
        if button_down {
            let (var4, var5) = self.pane();
            let var6 = var4 + 8;
            let var7 = var5 + 17;
            if (self.is_mouse_button_down == 0 || self.is_mouse_button_down == 1)
                && var1 >= var6
                && var1 < var6 + 224
                && var2 >= var7
                && var2 < var7 + 155
            {
                if self.is_mouse_button_down == 0 {
                    self.is_mouse_button_down = 1;
                } else {
                    self.x_scroll_p -= (var1 - self.mouse_x) as f64;
                    self.y_scroll_p -= (var2 - self.mouse_y) as f64;
                    self.x_scroll_o = self.x_scroll_p;
                    self.x_scroll_target = self.x_scroll_o;
                    self.y_scroll_o = self.y_scroll_p;
                    self.y_scroll_target = self.y_scroll_o;
                }
                self.mouse_x = var1;
                self.mouse_y = var2;
            }
            if self.x_scroll_target < x_min as f64 {
                self.x_scroll_target = x_min as f64;
            }
            if self.y_scroll_target < y_min as f64 {
                self.y_scroll_target = y_min as f64;
            }
            if self.x_scroll_target >= x_max as f64 {
                self.x_scroll_target = (x_max - 1) as f64;
            }
            if self.y_scroll_target >= y_max as f64 {
                self.y_scroll_target = (y_max - 1) as f64;
            }
        } else {
            self.is_mouse_button_down = 0;
        }
    }

    /// The scroll drawn this frame (`var4`, `var5`), within the limits.
    // src: GuiAchievements.java:127-143
    pub fn view(&self, var3: f32) -> (i32, i32) {
        let (x_min, y_min, x_max, y_max) = scroll_bounds();
        let mut var4 =
            floor_double(self.x_scroll_o + (self.x_scroll_p - self.x_scroll_o) * var3 as f64);
        let mut var5 =
            floor_double(self.y_scroll_o + (self.y_scroll_p - self.y_scroll_o) * var3 as f64);
        if var4 < x_min {
            var4 = x_min;
        }
        if var5 < y_min {
            var5 = y_min;
        }
        if var4 >= x_max {
            var4 = x_max - 1;
        }
        if var5 >= y_max {
            var5 = y_max - 1;
        }
        (var4, var5)
    }

    /// `drawTitle()`.
    // src: GuiAchievements.java:120-124
    fn draw_title(&self, ctx: &ScreenCtx, g: &mut GuiGl) {
        let (var1, var2) = self.pane();
        ctx.font
            .draw_string(g, "Achievements", var1 + 15, var2 + 5, 4210752);
    }

    /// `drawAchievementScreen(mouseX, mouseY, partialTicks)`.
    // src: GuiAchievements.java:126-331
    #[allow(clippy::too_many_lines)]
    fn draw_achievement_screen(
        &mut self,
        ctx: &mut ScreenCtx,
        g: &mut GuiGl,
        var1: i32,
        var2: i32,
        var3: f32,
    ) {
        let (var4, var5) = self.view(var3);
        let (var8, var9) = self.pane();
        let var10 = var8 + 16;
        let var11 = var9 + 17;
        self.base.z_level = 0.0;
        // glDepthFunc(GL_GEQUAL); glPushMatrix(); glTranslatef(0, 0, -200):
        // the depth clipping, done on the quads here (module docs).
        g.enable(gl::TEXTURE_2D);
        g.disable(gl::LIGHTING);
        g.enable(gl::RESCALE_NORMAL);
        g.bind_texture("/terrain.png");
        let var12 = (var4 + 288) >> 4;
        let var13 = (var5 + 288) >> 4;
        let var14 = (var4 + 288) % 16;
        let var15 = (var5 + 288) % 16;
        let mut var21 = JavaRandom::new(clock::new_random_seed());
        let mut rows = 0;
        let mut cols = 0;
        let mut var22 = 0;
        while var22 * 16 - var15 < WINDOW_H {
            let var23 = 0.6 - (var13 + var22) as f32 / 25.0 * 0.3;
            g.color4f(var23, var23, var23, 1.0);
            let mut var24 = 0;
            while var24 * 16 - var14 < WINDOW_W {
                let var26 = background_tile(&mut var21, var12 + var24, var13 + var22);
                draw::draw_textured_modal_rect(
                    g,
                    self.base.z_level,
                    var10 + var24 * 16 - var14,
                    var11 + var22 * 16 - var15,
                    (var26 % 16) << 4,
                    (var26 >> 4) << 4,
                    16,
                    16,
                );
                var24 += 1;
            }
            cols = var24;
            var22 += 1;
            rows = var22;
        }
        let clip = Clip {
            x0: var10 - var14,
            y0: var11 - var15,
            x1: var10 - var14 + cols * 16,
            y1: var11 - var15 + rows * 16,
        };
        g.enable(gl::DEPTH_TEST);
        g.disable(gl::TEXTURE_2D);

        let now = clock::current_time_millis();
        let list: Vec<&'static StatBase> = StatList::get().achievement_list().collect();
        // statFileWriter.hasAchievementUnlocked / canUnlockAchievement for
        // each achievement, read once through a shared borrow (the reads
        // have no side effects).
        let taken: Vec<Taken> = {
            let stats: &StatFileWriter = ctx.game.stats_mut();
            list.iter().map(|s| Taken::of(stats, s)).collect()
        };
        for (i, var28) in list.iter().enumerate() {
            let Some(a) = var28.achievement() else {
                continue;
            };
            let Some(parent) = a.parent_achievement.and_then(stat) else {
                continue;
            };
            let Some(p) = parent.achievement() else {
                continue;
            };
            let var14 = a.display_column * 24 - var4 + 11 + var10;
            let var15 = a.display_row * 24 - var5 + 11 + var11;
            let var16 = p.display_column * 24 - var4 + 11 + var10;
            let var17 = p.display_row * 24 - var5 + 11 + var11;
            let var19 = taken[i].unlocked;
            let var20 = taken[i].can_unlock;
            let var38: i32 = if blink(now) > 0.6 { 255 } else { 130 };
            let var33 = if var19 {
                -9408400
            } else if var20 {
                0xff00i32.wrapping_add(var38 << 24)
            } else {
                -16777216
            };
            // drawHorizontalLine(var14, var16, var15) and
            // drawVerticalLine(var16, var15, var17) (Gui.java:8-26), cut.
            let (a, b) = (var14.min(var16), var14.max(var16));
            fill_clipped(g, clip, a, var15, b + 1, var15 + 1, var33);
            let (a, b) = (var15.min(var17), var15.max(var17));
            fill_clipped(g, clip, var16, a + 1, var16 + 1, b, var33);
        }

        let mut var27: Option<(&'static StatBase, Taken)> = None;
        // RenderHelper.enableStandardItemLighting() under a 180° X rotation:
        // the item renderer's (module docs).
        g.disable(gl::LIGHTING);
        g.enable(gl::RESCALE_NORMAL);
        for (i, var30) in list.iter().enumerate() {
            let t = taken[i];
            let Some(a) = var30.achievement() else {
                continue;
            };
            let var16 = a.display_column * 24 - var4;
            let var17 = a.display_row * 24 - var5;
            if var16 >= -24 && var17 >= -24 && var16 <= 224 && var17 <= 155 {
                if t.unlocked {
                    let var35 = 1.0f32;
                    g.color4f(var35, var35, var35, 1.0);
                } else if t.can_unlock {
                    let var35 = if blink(now) < 0.6 { 0.6f32 } else { 0.8f32 };
                    g.color4f(var35, var35, var35, 1.0);
                } else {
                    let var35 = 0.3f32;
                    g.color4f(var35, var35, var35, 1.0);
                }
                g.bind_texture(BG_PNG);
                let var33 = var10 + var16;
                let var34 = var11 + var17;
                let u = if a.get_special() { 26 } else { 0 };
                draw_textured_rect_clipped(
                    g,
                    self.base.z_level,
                    Some(clip),
                    var33 - 2,
                    var34 - 2,
                    u,
                    202,
                    26,
                    26,
                    0.00390625,
                );
                // var29.field_27004_a: colorize the icon unless it cannot
                // be taken yet (then it keeps the 0.1 grey).
                let mut colorize = true;
                if !t.can_unlock {
                    let var36 = 0.1f32;
                    g.color4f(var36, var36, var36, 1.0);
                    colorize = false;
                }
                g.enable(gl::LIGHTING);
                g.enable(gl::CULL_FACE);
                let st = &a.the_item_stack;
                let icon = mc_sim::item::item(st.item_id)
                    .map(|i| i.get_icon_index(st))
                    .unwrap_or(-1);
                draw_item_into_gui(
                    g,
                    colorize,
                    Some(clip),
                    st.item_id,
                    st.item_damage,
                    icon,
                    var33 + 3,
                    var34 + 3,
                );
                g.disable(gl::LIGHTING);
                g.color4f(1.0, 1.0, 1.0, 1.0);
                if var1 >= var10
                    && var2 >= var11
                    && var1 < var10 + 224
                    && var2 < var11 + 155
                    && var1 >= var33
                    && var1 <= var33 + 22
                    && var2 >= var34
                    && var2 <= var34 + 22
                {
                    var27 = Some((var30, t));
                }
            }
        }

        g.disable(gl::DEPTH_TEST);
        g.enable(gl::BLEND);
        g.color4f(1.0, 1.0, 1.0, 1.0);
        g.bind_texture(BG_PNG);
        draw::draw_textured_modal_rect(
            g,
            self.base.z_level,
            var8,
            var9,
            0,
            0,
            PANE_WIDTH,
            PANE_HEIGHT,
        );
        // glPopMatrix()
        self.base.z_level = 0.0;
        g.disable(gl::DEPTH_TEST);
        g.enable(gl::TEXTURE_2D);
        default_draw_screen(&mut self.base, ctx, g, var1, var2);
        if let Some((var27, t)) = var27 {
            self.draw_tooltip(ctx, g, t, var27, var1, var2);
        }
        g.enable(gl::DEPTH_TEST);
        g.enable(gl::LIGHTING);
    }

    /// The hovered achievement's tooltip.
    // src: GuiAchievements.java:300-326
    fn draw_tooltip(
        &self,
        ctx: &mut ScreenCtx,
        g: &mut GuiGl,
        t: Taken,
        var27: &StatBase,
        var1: i32,
        var2: i32,
    ) {
        let f = ctx.font;
        let lang = ctx.lang;
        let var31 = stat_name(lang, var27);
        let inv = ctx.game.game_settings.key(KEY_INVENTORY);
        let var32 = description(lang, var27, inv);
        let var17 = var1 + 12;
        let var33 = var2 - 4;
        let special = var27.achievement().is_some_and(|a| a.get_special());
        if t.can_unlock {
            let var34 = f.get_string_width(&var31).max(120);
            let mut var37 = f.split_string_width(&var32, var34);
            if t.unlocked {
                var37 += 12;
            }
            draw::draw_gradient_rect(
                g,
                var17 - 3,
                var33 - 3,
                var17 + var34 + 3,
                var33 + var37 + 3 + 12,
                -1073741824,
                -1073741824,
            );
            f.draw_split_string(g, &var32, var17, var33 + 12, var34, -6250336);
            if t.unlocked {
                f.draw_string_with_shadow(
                    g,
                    &lang.translate_key("achievement.taken"),
                    var17,
                    var33 + var37 + 4,
                    -7302913,
                );
            }
        } else {
            let var34 = f.get_string_width(&var31).max(120);
            let parent = var27
                .achievement()
                .and_then(|a| a.parent_achievement)
                .and_then(stat)
                .map(|p| stat_name(lang, p))
                .unwrap_or_default();
            let var39 = lang.translate_key_format("achievement.requires", &[parent]);
            let var38 = f.split_string_width(&var39, var34);
            draw::draw_gradient_rect(
                g,
                var17 - 3,
                var33 - 3,
                var17 + var34 + 3,
                var33 + var38 + 12 + 3,
                -1073741824,
                -1073741824,
            );
            f.draw_split_string(g, &var39, var17, var33 + 12, var34, -9416624);
        }
        let color = if t.can_unlock {
            if special { -128 } else { -1 }
        } else if special {
            -8355776
        } else {
            -8355712
        };
        f.draw_string_with_shadow(g, &var31, var17, var33, color);
    }
}

/// One achievement's state in `mc.statFileWriter`.
#[derive(Clone, Copy, Debug)]
struct Taken {
    /// `hasAchievementUnlocked`.
    unlocked: bool,
    /// `canUnlockAchievement`.
    can_unlock: bool,
}

impl Taken {
    fn of(stats: &StatFileWriter, s: &StatBase) -> Self {
        Taken {
            unlocked: stats.has_achievement_unlocked(s),
            can_unlock: stats.can_unlock_achievement(s),
        }
    }
}

/// `drawRect(x1, y1, x2, y2, color)` cut to `clip` (the lines' depth
/// clipping, module docs).
fn fill_clipped(g: &mut GuiGl, clip: Clip, x1: i32, y1: i32, x2: i32, y2: i32, color: i32) {
    if let Some((a, b, c, d)) = clip.intersect(x1, y1, x2, y2) {
        draw::draw_rect(g, a, b, c, d, color);
    }
}

impl Screen for GuiAchievements {
    fn base(&self) -> &ScreenBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn kind(&self) -> ScreenKind {
        ScreenKind::Other("GuiAchievements".into())
    }

    // src: GuiAchievements.java:33-36
    fn init_gui(&mut self, ctx: &mut ScreenCtx) {
        self.base.control_list.clear();
        let (w, h) = (self.base.width, self.base.height);
        self.base.control_list.push(GuiButton::small_sized(
            1,
            w / 2 + 24,
            h / 2 + 74,
            80,
            20,
            ctx.lang.translate_key("gui.done"),
        ));
    }

    // src: GuiAchievements.java:38-45
    fn action_performed(&mut self, ctx: &mut ScreenCtx, button: usize) {
        if self.base.control_list[button].id == 1 {
            ctx.display_gui_screen(None);
            ctx.set_ingame_focus();
        }
    }

    // src: GuiAchievements.java:47-55
    fn key_typed(&mut self, ctx: &mut ScreenCtx, _ch: char, key: i32) {
        if key == ctx.game.game_settings.key(KEY_INVENTORY) {
            ctx.display_gui_screen(None);
            ctx.set_ingame_focus();
        } else {
            default_key_typed(ctx, key);
        }
    }

    // src: GuiAchievements.java:57-103
    fn draw_screen(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, mx: i32, my: i32, p: f32) {
        let down = ctx.game.input.is_button_down(0);
        self.drag(mx, my, down);
        draw_default_background(&self.base, ctx, g);
        self.draw_achievement_screen(ctx, g, mx, my, p);
        g.disable(gl::LIGHTING);
        g.disable(gl::DEPTH_TEST);
        self.draw_title(ctx, g);
        g.enable(gl::LIGHTING);
        g.enable(gl::DEPTH_TEST);
    }

    // src: GuiAchievements.java:105-118
    fn update_screen(&mut self, _ctx: &mut ScreenCtx) {
        self.ease();
    }

    // src: GuiAchievements.java:333-335
    fn does_gui_pause_game(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_limits_and_start() {
        // Columns -1..8, rows -5..6 (AchievementList.java:33-48).
        assert_eq!(scroll_bounds(), (-136, -232, 115, 67));
        let a = GuiAchievements::new();
        // openInventory at (0, 0): 0 - 141 / 2 - 12, 0 - 141 / 2.
        assert_eq!((a.x_scroll_p, a.y_scroll_p), (-82.0, -70.0));
        assert_eq!(a.view(0.0), (-82, -70));
    }

    #[test]
    fn icon_positions_in_the_window() {
        // At the start scroll, "Getting Wood" (2, 1) is drawn at
        // var10 + 2 * 24 + 82, var11 + 1 * 24 + 70 (frame 2 pixels up-left).
        let a = GuiAchievements::new();
        let (vx, vy) = a.view(1.0);
        let wood = stat(mc_sim::stats::achievement_list::MINE_WOOD)
            .and_then(|s| s.achievement())
            .unwrap();
        assert_eq!(
            (wood.display_column * 24 - vx, wood.display_row * 24 - vy),
            (130, 94)
        );
    }

    #[test]
    fn scroll_eases_toward_the_target() {
        let mut a = GuiAchievements::new();
        a.x_scroll_target = -82.0 + 100.0;
        a.ease();
        assert_eq!(a.x_scroll_o, -82.0);
        assert_eq!(a.x_scroll_p, -82.0 + 100.0 * 0.85);
        // Half a tick later, halfway between the two.
        assert_eq!(a.view(0.5).0, floor_double(-82.0 + 85.0 * 0.5));
        a.ease();
        a.ease();
        // Within 2 pixels: onto the target.
        a.ease();
        assert_eq!(a.x_scroll_p, 18.0);
    }

    #[test]
    fn background_tiles_follow_the_seeded_pattern() {
        let mut r = JavaRandom::new(0);
        // Row 35 is always bedrock; rows past 37 + ... too.
        assert_eq!(background_tile(&mut r, 10, 35), tile::BEDROCK);
        // Row 0: nextInt(1) == 0 -> + 0 -> sand.
        assert_eq!(background_tile(&mut r, 3, 0), tile::SAND);
        // The column alone seeds: same column and row, same tile.
        let t1 = background_tile(&mut r, 20, 15);
        let t2 = background_tile(&mut JavaRandom::new(99), 20, 15);
        assert_eq!(t1, t2);
        // Reference: new Random(1234 + 20); nextInt(); nextInt(16) + 7.
        let mut j = JavaRandom::new(1254);
        j.next_int();
        let v = j.next_int_bounded(16) + 7;
        let expect = if v == 22 {
            if j.next_int_bounded(2) == 0 {
                tile::ORE_DIAMOND
            } else {
                tile::ORE_REDSTONE
            }
        } else if v == 10 {
            tile::ORE_IRON
        } else if v == 8 {
            tile::ORE_COAL
        } else {
            tile::STONE
        };
        assert_eq!(t1, expect);
    }

    #[test]
    fn inventory_description_names_the_key() {
        let mut t = StringTranslate::default();
        t.translate_table.insert(
            "achievement.openInventory.desc".into(),
            "Press '%1$s' to open your inventory.".into(),
        );
        let s = stat(OPEN_INVENTORY).unwrap();
        assert_eq!(description(&t, s, 18), "Press 'E' to open your inventory.");
    }
}
