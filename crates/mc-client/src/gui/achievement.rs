//! `GuiAchievement` (GuiAchievement.java): the achievement popup at the top
//! right — "Achievement get!" with the achievement's name when one is
//! unlocked (`queueTakenAchievement`), or the name and description as a hint
//! (`queueAchievementInformation`, the "Press 'E' to open your inventory."
//! hint `EntityPlayerSP.onLivingUpdate` queues every update until it is
//! unlocked).
//!
//! The requests reach the GUI from the simulation: the hint as
//! [`crate::events::ClientEvent::AchievementHint`], the unlocked ones in
//! the player's [`StatFileWriter::taken_achievements`] (filled by
//! `EntityPlayerSP.addStat`, EntityPlayerSP.java:176-191), drained every
//! frame by [`apply_achievement_requests`]. The window is drawn every frame
//! after the screen and the debug graph ([`draw_achievement_window`]), as
//! `Minecraft.run` calls `updateAchievementWindow` after
//! `updateCameraAndRender` (Minecraft.java:586).
//!
//! Deferred (D2): the unlicensed-copy lines (`hasPaidCheckTime`,
//! GuiAchievement.java:59-72), never shown.
//!
//! [`StatFileWriter::taken_achievements`]: mc_sim::stats::StatFileWriter::taken_achievements

use bevy::prelude::*;
use mc_sim::block::BlockStatics;
use mc_sim::item::ItemStack;
use mc_sim::stats::achievement_list::OPEN_INVENTORY;
use mc_sim::stats::stat;

use super::batch::{GuiGl, GuiLayer, gl};
use super::draw;
use super::font::FontRenderer;
use super::render_item::{ItemGl, render_item_into_gui};
use crate::app::ScaledResolution;
use crate::assets::Lang;
use crate::clock;
use crate::game::Game;
use crate::keys;
use crate::lang::{StringTranslate, java_format};
use crate::settings::KEY_INVENTORY;

/// `/achievement/bg.png`.
// src: GuiAchievement.java:95
pub const ACHIEVEMENT_BG_PNG: &str = "/achievement/bg.png";

/// `GuiAchievement`'s state (module docs). A resource:
/// `Minecraft.guiAchievement`.
#[derive(Resource, Clone, Debug, Default)]
pub struct GuiAchievement {
    /// `achievementWindowWidth`.
    pub achievement_window_width: i32,
    /// `achievementWindowHeight`.
    pub achievement_window_height: i32,
    /// `achievementGetLocalText`.
    pub achievement_get_local_text: String,
    /// `achievementStatName`.
    pub achievement_stat_name: String,
    /// `theAchievement`, as its `statId`.
    pub the_achievement: Option<i32>,
    /// `achievementTime` (milliseconds; 0 = none).
    pub achievement_time: i64,
    /// `haveAchiement` (sic): the window is a hint, shown until replaced.
    pub have_achiement: bool,
    /// The last [`crate::events::ClientEvent::AchievementHint`] not applied
    /// yet.
    pub hint_requested: Option<i32>,
}

/// The achievement's icon (`theItemStack`).
fn icon(id: i32) -> Option<ItemStack> {
    stat(id)?.achievement().map(|a| a.the_item_stack)
}

impl GuiAchievement {
    /// `queueTakenAchievement(achievement)`: "Achievement get!" and the
    /// name, for 3 seconds from now.
    // src: GuiAchievement.java:22-28
    pub fn queue_taken_achievement(&mut self, lang: &StringTranslate, id: i32, now: i64) {
        let Some(s) = stat(id) else {
            return;
        };
        self.achievement_get_local_text = lang.translate_key("achievement.get");
        self.achievement_stat_name = lang.translate_key(&s.stat_name.key);
        self.achievement_time = now;
        self.the_achievement = Some(id);
        self.have_achiement = false;
    }

    /// `queueAchievementInformation(achievement)`: the name and
    /// `getDescription()`, shown from its middle (2.5 s in) and held while
    /// `haveAchiement`.
    // src: GuiAchievement.java:30-36
    pub fn queue_achievement_information(
        &mut self,
        lang: &StringTranslate,
        inventory_key: i32,
        id: i32,
        now: i64,
    ) {
        let Some(s) = stat(id) else {
            return;
        };
        self.achievement_get_local_text = lang.translate_key(&s.stat_name.key);
        self.achievement_stat_name = get_description(lang, inventory_key, id);
        self.achievement_time = now - 2500;
        self.the_achievement = Some(id);
        self.have_achiement = true;
    }

    /// `updateAchievementWindowScale()`'s size: the scaled resolution.
    // src: GuiAchievement.java:38-56
    pub fn update_achievement_window_scale(&mut self, r: &ScaledResolution) {
        self.achievement_window_width = r.scaled_width;
        self.achievement_window_height = r.scaled_height;
    }

    /// `updateAchievementWindow()` (module docs): the window slides in from
    /// the top over 0.375 s, stays, and slides out at the end of its 3
    /// seconds ([`window_offset`]); a hint stays in place.
    // src: GuiAchievement.java:58-124
    #[allow(clippy::too_many_arguments)]
    pub fn update_achievement_window(
        &mut self,
        g: &mut GuiGl,
        font: &FontRenderer,
        statics: &BlockStatics,
        r: &ScaledResolution,
        now: i64,
    ) {
        let Some(id) = self.the_achievement else {
            return;
        };
        if self.achievement_time == 0 {
            return;
        }
        let var8 = (now - self.achievement_time) as f64 / 3000.0;
        // vanilla: `haveAchiement || haveAchiement || …` (the same test twice).
        if !(self.have_achiement || (0.0..=1.0).contains(&var8)) {
            self.achievement_time = 0;
            return;
        }
        self.update_achievement_window_scale(r);
        g.disable(gl::DEPTH_TEST);
        g.depth_mask(false);
        let var5 = self.achievement_window_width - 160;
        let var6 = window_offset(var8);
        g.color4f(1.0, 1.0, 1.0, 1.0);
        g.enable(gl::TEXTURE_2D);
        g.bind_texture(ACHIEVEMENT_BG_PNG);
        g.disable(gl::LIGHTING);
        draw::draw_textured_modal_rect(g, 0.0, var5, var6, 96, 202, 160, 32);
        if self.have_achiement {
            font.draw_split_string(g, &self.achievement_stat_name, var5 + 30, var6 + 7, 120, -1);
        } else {
            font.draw_string(
                g,
                &self.achievement_get_local_text,
                var5 + 30,
                var6 + 7,
                -256,
            );
            font.draw_string(g, &self.achievement_stat_name, var5 + 30, var6 + 18, -1);
        }
        let mut ig = ItemGl::new();
        ig.push_matrix(g);
        ig.rotatef(g, 180.0, 1.0, 0.0, 0.0);
        ig.enable_standard_item_lighting(g);
        ig.pop_matrix(g);
        g.disable(gl::LIGHTING);
        ig.rescale_normal = true;
        g.enable(gl::LIGHTING);
        let stack = icon(id);
        render_item_into_gui(&mut ig, g, statics, stack.as_ref(), var5 + 8, var6 + 8);
        g.disable(gl::LIGHTING);
        g.depth_mask(true);
        g.enable(gl::DEPTH_TEST);
    }
}

/// The window's y: `-(1 - min(4 t', 1))⁴ × 36` with `t'` the distance of
/// the time fraction from the nearer end (`var9`).
// src: GuiAchievement.java:80-94
pub fn window_offset(var8: f64) -> i32 {
    let mut var9 = var8 * 2.0;
    if var9 > 1.0 {
        var9 = 2.0 - var9;
    }
    var9 *= 4.0;
    var9 = 1.0 - var9;
    if var9 < 0.0 {
        var9 = 0.0;
    }
    var9 *= var9;
    var9 *= var9;
    -((var9 * 36.0) as i32)
}

/// `Achievement.getDescription()`: the translated description, formatted
/// with the inventory key's name for `openInventory`
/// (`StatStringFormatKeyInv`, set in `startGame`, Minecraft.java:248).
// src: Achievement.java:64-66, StatStringFormatKeyInv.java:14-16
pub fn get_description(lang: &StringTranslate, inventory_key: i32, id: i32) -> String {
    let Some(a) = stat(id).and_then(|s| s.achievement()) else {
        return String::new();
    };
    let d = lang.translate_key(&a.achievement_description);
    if id == OPEN_INVENTORY {
        let name = keys::key_name(inventory_key).unwrap_or("null").to_string();
        java_format(&d, &[name])
    } else {
        d
    }
}

/// Applies the hint and the unlocked achievements of this frame (module
/// docs), hints first.
pub fn apply_achievement_requests(
    mut game: NonSendMut<Game>,
    mut a: ResMut<GuiAchievement>,
    lang: Res<Lang>,
) {
    let now = clock::current_time_millis();
    if let Some(id) = a.hint_requested.take() {
        let key = game.game_settings.key(KEY_INVENTORY);
        a.queue_achievement_information(&lang.0, key, id, now);
    }
    let taken: Vec<i32> = std::mem::take(&mut game.stats_mut().taken_achievements);
    for id in taken {
        a.queue_taken_achievement(&lang.0, id, now);
    }
}

/// Draws the window (module docs).
pub fn draw_achievement_window(
    game: NonSend<Game>,
    mut a: ResMut<GuiAchievement>,
    mut g: ResMut<GuiGl>,
    font: Res<FontRenderer>,
) {
    let r = ScaledResolution::new(
        game.game_settings.gui_scale,
        game.display_width,
        game.display_height,
    );
    g.set_layer(GuiLayer::Screen);
    let default = BlockStatics::default();
    let statics = game
        .the_world
        .as_ref()
        .map_or(&default, |w| &w.block_statics);
    a.update_achievement_window(&mut g, &font, statics, &r, clock::current_time_millis());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slide_timing() {
        // Hidden at the ends, sliding in over the first eighth.
        assert_eq!(window_offset(0.0), -36);
        assert_eq!(window_offset(0.0625), -2);
        assert_eq!(window_offset(0.125), 0);
        assert_eq!(window_offset(0.5), 0);
        assert_eq!(window_offset(0.9375), -2);
        assert_eq!(window_offset(1.0), -36);
        // A hint starts at 2.5 s of 3: shown.
        assert_eq!(window_offset(2500.0 / 3000.0), 0);
    }

    #[test]
    fn open_inventory_description() {
        let mut t = StringTranslate::default();
        t.translate_table.insert(
            "achievement.openInventory.desc".into(),
            "Press '%1$s' to open your inventory.".into(),
        );
        t.translate_table.insert(
            "achievement.openInventory".into(),
            "Taking Inventory".into(),
        );
        // E is LWJGL key 18.
        assert_eq!(
            get_description(&t, 18, OPEN_INVENTORY),
            "Press 'E' to open your inventory."
        );
        let mut a = GuiAchievement::default();
        a.queue_achievement_information(&t, 18, OPEN_INVENTORY, 10_000);
        assert_eq!(a.achievement_get_local_text, "Taking Inventory");
        assert_eq!(a.achievement_time, 7_500);
        assert!(a.have_achiement);
        a.queue_taken_achievement(&t, OPEN_INVENTORY, 20_000);
        assert_eq!(a.achievement_get_local_text, "achievement.get");
        assert_eq!(a.achievement_stat_name, "Taking Inventory");
        assert!(!a.have_achiement);
        // The popup ends after 3 s.
        let mut g = GuiGl::default();
        let f = FontRenderer::new(
            &crate::textures::ArgbImage::new(128, 128),
            false,
            Default::default(),
        );
        let r = ScaledResolution::new(0, 854, 480);
        a.update_achievement_window(&mut g, &f, &BlockStatics::default(), &r, 21_000);
        assert!(!g.draws().is_empty());
        assert_eq!(g.draws()[0].batch.positions[3], [427.0 - 160.0, 0.0, 0.0]);
        let mut g = GuiGl::default();
        a.update_achievement_window(&mut g, &f, &BlockStatics::default(), &r, 23_001);
        assert!(g.draws().is_empty());
        assert_eq!(a.achievement_time, 0);
    }
}
