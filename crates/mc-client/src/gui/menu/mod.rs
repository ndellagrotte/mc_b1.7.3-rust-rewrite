//! The out-of-world screens and the options, achievements and statistics
//! screens (WP-3.6c), on WP-3.6a's [`Screen`](super::screen::Screen) trait
//! and widgets, plus the project's world import (PLAN_v4 D5, §3.5).
//!
//! | Module | Vanilla |
//! |---|---|
//! | [`main_menu`] | `GuiMainMenu` |
//! | [`ingame_menu`] | `GuiIngameMenu` |
//! | [`options`] | `GuiOptions` |
//! | [`video_settings`] | `GuiVideoSettings` |
//! | [`controls`] | `GuiControls` |
//! | [`select_world`], [`world_slot`] | `GuiSelectWorld`, `GuiWorldSlot` |
//! | [`create_world`] | `GuiCreateWorld` |
//! | [`rename_world`] | `GuiRenameWorld` |
//! | [`achievements`] | `GuiAchievements` (with `Achievement.getDescription`, `StatStringFormatKeyInv`) |
//! | [`stats`] | `GuiStats`, `GuiSlotStats`, `GuiSlotStatsGeneral`, `GuiSlotStatsItem`, `GuiSlotStatsBlock`, `SorterStatsItem`, `SorterStatsBlock` |
//! | [`texture_packs`] | `GuiTexturePacks`, `GuiTexturePackSlot` (the default pack only) |
//! | [`import_world`], [`import_refused`] | — (project-owned: the D5 import and its refusal dialog) |
//! | [`export_world`] | — (project-owned, WP-4.5: the export screen and its result dialog) |
//!
//! The loading screen's drawing (`LoadingScreenRenderer`) is
//! [`super::loading`]; the registrations by class name are
//! [`super::screens_menu`].
//!
//! # Parents
//!
//! Java's `new GuiOptions(this, …)` keeps the opening screen object and
//! displays it again on Done (which re-runs its `initGui`, keeping its other
//! fields). Here the opening screen hands itself over with
//! [`ScreenCtx::display_wrapping_this`] and the child keeps it as
//! `parent: Option<Box<dyn Screen>>`; a screen opened by class name (the
//! `open_screen` dev action) has no parent, which Java's `displayGuiScreen(null)`
//! then turns into the main menu or the game.
//!
//! # Deferred (PLAN_v4 D3)
//!
//! Multiplayer (`GuiMultiplayer`, `GuiConnecting`, `GuiConnectFailed`,
//! `GuiDownloadTerrain`: not transcribed; the title screen's Multiplayer
//! button stays disabled), texture packs beyond the default one
//! (`TexturePackList`, `TexturePackCustom`; the "Open texture pack folder"
//! button is disabled), pre-McRegion conversion (the world list's
//! "conversion" note is drawn, but the shell refuses to open such a world),
//! the stats upload (`StatsSyncher`).
//!
//! [`ScreenCtx::display_wrapping_this`]: super::screen::ScreenCtx::display_wrapping_this

pub mod achievements;
pub mod controls;
pub mod create_world;
pub mod export_world;
pub mod import_refused;
pub mod import_world;
pub mod ingame_menu;
pub mod main_menu;
pub mod options;
pub mod rename_world;
pub mod select_world;
pub mod stats;
pub mod texture_packs;
pub mod video_settings;
pub mod world_slot;

use mc_sim::stats::StatBase;

use super::batch::GuiGl;
use super::screen::{Screen, ScreenCtx};
use crate::lang::StringTranslate;

/// `Minecraft.hideQuitButton`: false when the game runs without an applet
/// (or with `stand-alone=true`), as this client does, so the title screen
/// has its Quit Game button.
// src: Minecraft.java:123, :177-179
pub const HIDE_QUIT_BUTTON: bool = false;

/// A stat's `statName` as Java builds it from the lang table
/// (`StatCollector.translateToLocal`, StatBase.java's constructors).
pub fn stat_name(lang: &StringTranslate, s: &StatBase) -> String {
    s.stat_name
        .translate(&|k: &str| lang.translate_table.get(k).cloned())
}

/// Displays `parent` again (`mc.displayGuiScreen(this.parentScreen)`):
/// `None` is Java's null parent (the main menu without a world, else the
/// game).
pub fn display_parent(ctx: &mut ScreenCtx, parent: &mut Option<Box<dyn Screen>>) {
    let p = parent.take();
    ctx.display_gui_screen(p);
}

/// A clip rectangle `[x0, x1) × [y0, y1)` in scaled pixels (the
/// achievement map's window; [`achievements`] module docs). The item icons
/// are cut to it by [`super::render_item::draw_item_into_gui_with`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clip {
    /// Left.
    pub x0: i32,
    /// Top.
    pub y0: i32,
    /// Right (exclusive).
    pub x1: i32,
    /// Bottom (exclusive).
    pub y1: i32,
}

impl Clip {
    /// The part of `[x0, x1) × [y0, y1)` inside the clip, if any.
    pub fn intersect(&self, x0: i32, y0: i32, x1: i32, y1: i32) -> Option<(i32, i32, i32, i32)> {
        let r = (
            x0.max(self.x0),
            y0.max(self.y0),
            x1.min(self.x1),
            y1.min(self.y1),
        );
        (r.0 < r.2 && r.1 < r.3).then_some(r)
    }
}

/// `drawTexturedModalRect`'s quad (`w × h` pixels from texel `(u, v)` of a
/// sheet with texel size `texel`, one texel per pixel) cut to `clip`:
/// the texture coordinates follow the cut. `None` draws it whole, as
/// `Gui.drawTexturedModalRect` / `RenderItem.renderTexturedQuad` do.
// src: Gui.java:98-108, RenderItem.java:213-224
#[allow(clippy::too_many_arguments)]
pub fn draw_textured_rect_clipped(
    g: &mut GuiGl,
    z: f32,
    clip: Option<Clip>,
    x: i32,
    y: i32,
    u: i32,
    v: i32,
    w: i32,
    h: i32,
    texel: f32,
) {
    let (x0, y0, x1, y1) = match clip {
        Some(c) => match c.intersect(x, y, x + w, y + h) {
            Some(r) => r,
            None => return,
        },
        None => (x, y, x + w, y + h),
    };
    let (u0, v0) = (u + (x0 - x), v + (y0 - y));
    let (u1, v1) = (u0 + (x1 - x0), v0 + (y1 - y0));
    let t = |a: i32| (a as f32 * texel) as f64;
    let z = z as f64;
    g.start_drawing_quads();
    g.add_vertex_with_uv(x0 as f64, y1 as f64, z, t(u0), t(v1));
    g.add_vertex_with_uv(x1 as f64, y1 as f64, z, t(u1), t(v1));
    g.add_vertex_with_uv(x1 as f64, y0 as f64, z, t(u1), t(v0));
    g.add_vertex_with_uv(x0 as f64, y0 as f64, z, t(u0), t(v0));
    g.draw();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipped_quads_follow_the_texture() {
        let c = Clip {
            x0: 10,
            y0: 10,
            x1: 20,
            y1: 20,
        };
        assert_eq!(c.intersect(0, 0, 5, 5), None);
        assert_eq!(c.intersect(5, 15, 30, 40), Some((10, 15, 20, 20)));
        let mut g = GuiGl::default();
        draw_textured_rect_clipped(&mut g, 0.0, Some(c), 4, 12, 0, 202, 26, 26, 1.0);
        let b = &g.draws()[0].batch;
        // x 10..20 shows texels 6..16; y 12..20 texels 202..210.
        assert_eq!(b.positions[0], [10.0, 20.0, 0.0]);
        assert_eq!(b.uvs[0], [6.0, 210.0]);
        assert_eq!(b.positions[2], [20.0, 12.0, 0.0]);
        assert_eq!(b.uvs[2], [16.0, 202.0]);
    }
}
