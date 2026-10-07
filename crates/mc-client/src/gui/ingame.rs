//! `GuiIngame` (GuiIngame.java): the in-game overlay — the vignette, the
//! pumpkin blur, the portal overlay, the hotbar with its items, the
//! crosshair, the armour, hearts and air rows, the sleep fade, the F3 text,
//! the "Now playing" line and the chat — and its tick (`updateTick`), plus
//! `Minecraft.displayDebugInfo`'s frame-time graph (drawn after the screen,
//! as vanilla draws it after `updateCameraAndRender`).
//!
//! [`GuiIngame`] is a resource (`Minecraft.ingameGUI`). It is drawn in
//! [`super::GuiSet::Overlay`] when `renderGameOverlay` runs in vanilla:
//! a world is drawn (`theWorld != null`, not `skipRenderWorld`, no world
//! load in progress) and the GUI is shown or a screen is open
//! (EntityRenderer.java:356-395). `updateTick` runs in
//! [`RunTickHook::IngameGuiUpdateTick`] (Minecraft.java:950).
//!
//! The chat's lines arrive as [`ClientEvent::ChatMessage`] /
//! [`ClientEvent::ChatMessageTranslate`] (`addChatMessage`,
//! `addChatMessageTranslate`), [`ClientEvent::ClearChat`]
//! (`clearChatMessages`) and [`RecordPlayingMessage`]
//! (`setRecordPlayingMessage`), collected by [`collect_gui_messages`] in
//! every tick hook and after the frame (so a line raised inside a tick is
//! there for the next hook, as vanilla adds it at once).
//!
//! Deferred (D2): `Minecraft.hasPaidCheckTime` (the unlicensed-copy check)
//! is never set, so the F3 block is never moved down 32 pixels
//! (GuiIngame.java:168-170).

use bevy::ecs::message::MessageCursor;
use bevy::prelude::*;
use mc_core::java_random::JavaRandom;
use mc_sim::block::{BlockStatics, Material};
use mc_sim::entity::{Entity, base as entity_base};
use mc_sim::world::World;

use super::batch::{GuiGl, GuiLayer, gl};
use super::chat::{ChatLine, java_len, substring};
use super::draw;
use super::font::FontRenderer;
use super::render_item::{ItemGl, render_item_into_gui, render_item_overlay_into_gui};
use crate::app::{ScaledResolution, ScreenKind};
use crate::assets::Lang;
use crate::clock;
use crate::debug::DebugText;
use crate::events::ClientEvent;
use crate::game::Game;
use crate::sound::RecordPlayingMessage;

/// `/gui/gui.png`: the hotbar.
pub const GUI_PNG: &str = "/gui/gui.png";
/// `/gui/icons.png`: crosshair, hearts, armour, air.
pub const ICONS_PNG: &str = "/gui/icons.png";
/// `%blur%/misc/pumpkinblur.png`.
// src: GuiIngame.java:269
pub const PUMPKIN_BLUR_PNG: &str = "%blur%/misc/pumpkinblur.png";
/// `%blur%/misc/vignette.png`.
// src: GuiIngame.java:298
pub const VIGNETTE_PNG: &str = "%blur%/misc/vignette.png";
/// `Block.pumpkin.blockID`.
// src: Block.java:678
pub const PUMPKIN: i32 = 86;
/// `Block.portal.blockIndexInTexture`.
// src: Block.java:682
pub const PORTAL_TEXTURE: i32 = 14;
/// The chat's width limit in pixels.
// src: GuiIngame.java:381, :383
pub const CHAT_WIDTH: i32 = 320;
/// Lines kept.
// src: GuiIngame.java:392
pub const CHAT_MAX_LINES: usize = 50;

/// `GuiIngame`'s state (module docs).
#[derive(Resource, Clone, Debug)]
pub struct GuiIngame {
    /// `chatMessageList`: newest first.
    // src: GuiIngame.java:12
    pub chat_message_list: Vec<ChatLine>,
    /// `rand` (`new Random()`; reseeded every frame before use).
    // src: GuiIngame.java:13
    pub rand: JavaRandom,
    /// `field_933_a`: never set in this version (GuiChat reads it).
    // src: GuiIngame.java:15
    pub field_933_a: Option<String>,
    /// `updateCounter`: ticks.
    // src: GuiIngame.java:16
    pub update_counter: i32,
    /// `recordPlaying`.
    // src: GuiIngame.java:17
    pub record_playing: String,
    /// `recordPlayingUpFor`: ticks left.
    // src: GuiIngame.java:18
    pub record_playing_up_for: i32,
    /// `recordIsPlaying`.
    // src: GuiIngame.java:19
    pub record_is_playing: bool,
    /// `prevVignetteBrightness`.
    // src: GuiIngame.java:21
    pub prev_vignette_brightness: f32,
    /// `Gui.zLevel` (set to −90 by the first frame and kept).
    pub z_level: f32,
}

impl Default for GuiIngame {
    fn default() -> Self {
        GuiIngame {
            chat_message_list: Vec::new(),
            rand: JavaRandom::new(clock::new_random_seed()),
            field_933_a: None,
            update_counter: 0,
            record_playing: String::new(),
            record_playing_up_for: 0,
            record_is_playing: false,
            prev_vignette_brightness: 1.0,
            z_level: 0.0,
        }
    }
}

/// What `renderGameOverlay` reads from `Minecraft` besides the player.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OverlayInputs {
    /// `ScaledResolution.getScaledWidth()`.
    pub width: i32,
    /// `getScaledHeight()`.
    pub height: i32,
    /// `Minecraft.isFancyGraphicsEnabled()`.
    pub fancy: bool,
    /// `gameSettings.thirdPersonView`.
    pub third_person: bool,
    /// `gameSettings.showDebugInfo`.
    pub show_debug_info: bool,
    /// `mc.currentScreen instanceof GuiChat` (`GuiSleepMP` extends it).
    pub chat_open: bool,
    /// `playerController.shouldDrawHUD()` (true in singleplayer).
    pub should_draw_hud: bool,
}

/// The hotbar's background and selection rectangles `(x, y, u, v, w, h)`.
// src: GuiIngame.java:52-53
pub fn hotbar_rects(w: i32, h: i32, current_item: i32) -> [(i32, i32, i32, i32, i32, i32); 2] {
    [
        (w / 2 - 91, h - 22, 0, 0, 182, 22),
        (
            w / 2 - 91 - 1 + current_item * 20,
            h - 22 - 1,
            0,
            22,
            24,
            22,
        ),
    ]
}

/// The hotbar slot `i`'s item position (`renderInventorySlot`'s x, y).
// src: GuiIngame.java:142-146
pub fn hotbar_slot_pos(w: i32, h: i32, i: i32) -> (i32, i32) {
    (w / 2 - 90 + i * 20 + 2, h - 16 - 3)
}

/// One `drawTexturedModalRect` of `icons.png`: `(x, y, u, v)` (9×9 for
/// the rows).
pub type IconRect = (i32, i32, i32, i32);

/// The armour and hearts rows (the loop at GuiIngame.java:74-119), in draw
/// order: for each of the ten slots the armour icon (if any armour), the
/// heart outline (white while flashing), the flash's previous-health
/// half/full heart, the full or half heart. `rand` is `GuiIngame.rand`
/// reseeded with `updateCounter * 312871`; below 5 health each heart
/// moves down by `rand.nextInt(2)` (the jitter).
// src: GuiIngame.java:59-119
#[allow(clippy::too_many_arguments)]
pub fn health_rows(
    w: i32,
    h: i32,
    hearts_life: i32,
    health: i32,
    prev_health: i32,
    armor: i32,
    update_counter: i32,
    rand: &mut JavaRandom,
) -> Vec<IconRect> {
    let mut out = Vec::new();
    let mut var12 = hearts_life / 3 % 2 == 1;
    if hearts_life < 10 {
        var12 = false;
    }
    let var13 = health;
    let var14 = prev_health;
    rand.set_seed(update_counter.wrapping_mul(312871) as i64);
    let var15 = armor;
    for var16 in 0..10 {
        let mut var17 = h - 32;
        if var15 > 0 {
            let var18 = w / 2 + 91 - var16 * 8 - 9;
            if var16 * 2 + 1 < var15 {
                out.push((var18, var17, 34, 9));
            }
            if var16 * 2 + 1 == var15 {
                out.push((var18, var17, 25, 9));
            }
            if var16 * 2 + 1 > var15 {
                out.push((var18, var17, 16, 9));
            }
        }
        let var28 = if var12 { 1 } else { 0 };
        let var19 = w / 2 - 91 + var16 * 8;
        if var13 <= 4 {
            var17 += rand.next_int_bounded(2);
        }
        out.push((var19, var17, 16 + var28 * 9, 0));
        if var12 {
            if var16 * 2 + 1 < var14 {
                out.push((var19, var17, 70, 0));
            }
            if var16 * 2 + 1 == var14 {
                out.push((var19, var17, 79, 0));
            }
        }
        if var16 * 2 + 1 < var13 {
            out.push((var19, var17, 52, 0));
        }
        if var16 * 2 + 1 == var13 {
            out.push((var19, var17, 61, 0));
        }
    }
    out
}

/// The air bubbles under water: `(full, popping)` counts,
/// `ceil((air - 2) * 10 / 300)` full ones and
/// `ceil(air * 10 / 300) - full` popping.
// src: GuiIngame.java:122-123
pub fn air_bubbles(air: i32) -> (i32, i32) {
    let var16 = ((air - 2) as f64 * 10.0 / 300.0).ceil() as i32;
    let var17 = (air as f64 * 10.0 / 300.0).ceil() as i32 - var16;
    (var16, var17)
}

/// A chat line's opacity (0–255): fully visible for 180 ticks, then fading
/// quadratically to 0 at 200; 255 whatever its age while the chat is open.
// src: GuiIngame.java:229-245
#[allow(clippy::manual_clamp)] // the Java's two tests, kept
pub fn chat_line_alpha(update_counter: i32, chat_open: bool) -> i32 {
    if !(update_counter < 200 || chat_open) {
        return 0;
    }
    let mut var32 = update_counter as f64 / 200.0;
    var32 = 1.0 - var32;
    var32 *= 10.0;
    if var32 < 0.0 {
        var32 = 0.0;
    }
    if var32 > 1.0 {
        var32 = 1.0;
    }
    var32 *= var32;
    let mut var20 = (255.0 * var32) as i32;
    if chat_open {
        var20 = 255;
    }
    var20
}

/// The "Now playing" line's opacity: `(recordPlayingUpFor - partial) * 256
/// / 20`, at most 255.
// src: GuiIngame.java:193-197
pub fn record_alpha(record_playing_up_for: i32, partial: f32) -> i32 {
    let var25 = record_playing_up_for as f32 - partial;
    let var16 = (var25 * 256.0 / 20.0) as i32;
    var16.min(255)
}

/// The sleep fade's `drawRect` colour: alpha `220 × timer / 100` (falling
/// again past 100), colour `0x101020`.
// src: GuiIngame.java:153-159
pub fn sleep_overlay_color(sleep_timer: i32) -> i32 {
    let var15 = sleep_timer;
    let mut var27 = var15 as f32 / 100.0;
    if var27 > 1.0 {
        var27 = 1.0 - (var15 - 100) as f32 / 10.0;
    }
    ((220.0 * var27) as i32) << 24 | 1052704
}

/// `renderPortalOverlay`'s alpha: below 1 the time is raised to the fourth
/// power, then `× 0.8 + 0.2`.
// src: GuiIngame.java:313-317
pub fn portal_alpha(mut var1: f32) -> f32 {
    if var1 < 1.0 {
        var1 *= var1;
        var1 *= var1;
        var1 = var1 * 0.8 + 0.2;
    }
    var1
}

/// `java.awt.Color.HSBtoRGB(hue, saturation, brightness)`: the hue's
/// fractional part picks one of six sectors; each channel is rounded as
/// `(int)(v * 255 + 0.5)`; the alpha byte is 0xFF.
pub fn hsb_to_rgb(hue: f32, saturation: f32, brightness: f32) -> i32 {
    let q = |v: f32| (v * 255.0 + 0.5) as i32;
    let (r, g, b) = if saturation == 0.0 {
        let v = q(brightness);
        (v, v, v)
    } else {
        let h = (hue - hue.floor()) * 6.0;
        let f = h - h.floor();
        let p = brightness * (1.0 - saturation);
        let qq = brightness * (1.0 - saturation * f);
        let t = brightness * (1.0 - saturation * (1.0 - f));
        match h as i32 {
            0 => (q(brightness), q(t), q(p)),
            1 => (q(qq), q(brightness), q(p)),
            2 => (q(p), q(brightness), q(t)),
            3 => (q(p), q(qq), q(brightness)),
            4 => (q(t), q(p), q(brightness)),
            5 => (q(brightness), q(p), q(qq)),
            _ => (0, 0, 0),
        }
    };
    (0xff000000u32 as i32) | (r << 16) | (g << 8) | b
}

impl GuiIngame {
    /// `updateTick()`: the record line's timer, `updateCounter` and every
    /// chat line's age.
    // src: GuiIngame.java:363-374
    pub fn update_tick(&mut self) {
        if self.record_playing_up_for > 0 {
            self.record_playing_up_for -= 1;
        }
        self.update_counter = self.update_counter.wrapping_add(1);
        for l in &mut self.chat_message_list {
            l.update_counter = l.update_counter.wrapping_add(1);
        }
    }

    /// `clearChatMessages()`.
    // src: GuiIngame.java:376-378
    pub fn clear_chat_messages(&mut self) {
        self.chat_message_list.clear();
    }

    /// `addChatMessage(line)`: a line wider than 320 pixels is cut at the
    /// longest prefix that fits (at least one character), the prefix added
    /// first (recursively); the rest becomes the new line. At most 50 lines
    /// are kept.
    // src: GuiIngame.java:380-396
    pub fn add_chat_message(&mut self, font: &FontRenderer, var1: &str) {
        let mut var1 = var1.to_string();
        while font.get_string_width(&var1) > CHAT_WIDTH {
            let mut var2 = 1;
            while var2 < java_len(&var1)
                && font.get_string_width(&substring(&var1, 0, var2 + 1)) <= CHAT_WIDTH
            {
                var2 += 1;
            }
            self.add_chat_message(font, &substring(&var1, 0, var2));
            var1 = substring(&var1, var2, java_len(&var1));
        }
        self.chat_message_list.insert(0, ChatLine::new(var1));
        self.chat_message_list.truncate(CHAT_MAX_LINES);
    }

    /// `setRecordPlayingMessage(name)`: `"Now playing: " + name` for 60
    /// ticks.
    // src: GuiIngame.java:398-402
    pub fn set_record_playing_message(&mut self, var1: &str) {
        self.record_playing = format!("Now playing: {var1}");
        self.record_playing_up_for = 60;
        self.record_is_playing = true;
    }

    /// `addChatMessageTranslate(key)`.
    // src: GuiIngame.java:404-408
    pub fn add_chat_message_translate(
        &mut self,
        lang: &crate::lang::StringTranslate,
        font: &FontRenderer,
        var1: &str,
    ) {
        let var3 = lang.translate_key(var1);
        self.add_chat_message(font, &var3);
    }

    /// `renderGameOverlay(partialTicks, screenOpen, mouseX, mouseY)` with
    /// the player `p` in `world` (module docs).
    // src: GuiIngame.java:27-261
    #[allow(clippy::too_many_arguments)]
    pub fn render_game_overlay(
        &mut self,
        world: &mut World,
        p: &Entity,
        s: &OverlayInputs,
        g: &mut GuiGl,
        font: &FontRenderer,
        debug: &DebugText,
        var1: f32,
    ) {
        let (var6, var7) = (s.width, s.height);
        let var8 = font;
        let Some(pd) = p.kind.player() else {
            return;
        };
        let living = p.kind.living();
        let mut ig = ItemGl::new();
        g.enable(gl::BLEND);
        if s.fancy {
            let b = entity_base::get_entity_brightness(p, world, var1);
            self.render_vignette(g, b, var6, var7);
        }
        let var9 = pd.inventory.armor_item_in_slot(3);
        if !s.third_person && var9.is_some_and(|i| i.item_id == PUMPKIN) {
            render_pumpkin_blur(g, var6, var7);
        }
        let var10 = pd.prev_time_in_portal + (pd.time_in_portal - pd.prev_time_in_portal) * var1;
        if var10 > 0.0 {
            render_portal_overlay(g, var10, var6, var7);
        }
        g.color4f(1.0, 1.0, 1.0, 1.0);
        g.bind_texture(GUI_PNG);
        let var11 = &pd.inventory;
        self.z_level = -90.0;
        for (x, y, u, v, w, h) in hotbar_rects(var6, var7, var11.current_item) {
            draw::draw_textured_modal_rect(g, self.z_level, x, y, u, v, w, h);
        }
        g.bind_texture(ICONS_PNG);
        g.enable(gl::BLEND);
        g.blend_func(gl::ONE_MINUS_DST_COLOR, gl::ONE_MINUS_SRC_COLOR);
        draw::draw_textured_modal_rect(g, self.z_level, var6 / 2 - 7, var7 / 2 - 7, 0, 0, 16, 16);
        g.disable(gl::BLEND);
        if s.should_draw_hud {
            let (health, prev_health) = living.map_or((0, 0), |l| (l.health, l.prev_health));
            // getPlayerArmorValue(): inventory.getTotalArmorValue()
            // (EntityPlayerSP.java:137-139).
            let armor = var11.get_total_armor_value();
            let rows = health_rows(
                var6,
                var7,
                p.base.hearts_life,
                health,
                prev_health,
                armor,
                self.update_counter,
                &mut self.rand,
            );
            for (x, y, u, v) in rows {
                draw::draw_textured_modal_rect(g, self.z_level, x, y, u, v, 9, 9);
            }
            if entity_base::is_inside_of_material(p, world, Material::Water) {
                let (var16, var17) = air_bubbles(p.base.air);
                for var18 in 0..var16 + var17 {
                    let u = if var18 < var16 { 16 } else { 25 };
                    draw::draw_textured_modal_rect(
                        g,
                        self.z_level,
                        var6 / 2 - 91 + var18 * 8,
                        var7 - 32 - 9,
                        u,
                        18,
                        9,
                        9,
                    );
                }
            }
        }
        g.disable(gl::BLEND);
        ig.rescale_normal = true;
        ig.push_matrix(g);
        ig.rotatef(g, 120.0, 1.0, 0.0, 0.0);
        ig.enable_standard_item_lighting(g);
        ig.pop_matrix(g);
        for var15 in 0..9 {
            let (var16, var17) = hotbar_slot_pos(var6, var7, var15);
            self.render_inventory_slot(
                &mut ig,
                g,
                &world.block_statics,
                font,
                var11.main_inventory[var15 as usize].as_ref(),
                var16,
                var17,
                var1,
            );
        }
        ig.disable_standard_item_lighting(g);
        ig.rescale_normal = false;
        if pd.sleep_timer > 0 {
            g.disable(gl::DEPTH_TEST);
            g.disable(gl::ALPHA_TEST);
            draw::draw_rect(g, 0, 0, var6, var7, sleep_overlay_color(pd.sleep_timer));
            g.enable(gl::ALPHA_TEST);
            g.enable(gl::DEPTH_TEST);
        }
        if s.show_debug_info {
            g.push_matrix();
            // Minecraft.hasPaidCheckTime is never set (module docs).
            for (i, line) in debug.left.iter().enumerate() {
                var8.draw_string_with_shadow(g, line, 2, 2 + 10 * i as i32, 16777215);
            }
            for (i, line) in debug.right.iter().enumerate() {
                draw::draw_string(
                    g,
                    var8,
                    line,
                    var6 - var8.get_string_width(line) - 2,
                    2 + 10 * i as i32,
                    14737632,
                );
            }
            for (i, line) in debug.position.iter().enumerate() {
                draw::draw_string(g, var8, line, 2, 64 + 8 * i as i32, 14737632);
            }
            g.pop_matrix();
        }
        if self.record_playing_up_for > 0 {
            let var25 = self.record_playing_up_for as f32 - var1;
            let var16 = record_alpha(self.record_playing_up_for, var1);
            if var16 > 0 {
                g.push_matrix();
                g.translatef((var6 / 2) as f32, (var7 - 48) as f32, 0.0);
                g.enable(gl::BLEND);
                g.blend_func(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
                let mut var17 = 16777215;
                if self.record_is_playing {
                    var17 = hsb_to_rgb(var25 / 50.0, 0.7, 0.6) & 16777215;
                }
                var8.draw_string(
                    g,
                    &self.record_playing,
                    -var8.get_string_width(&self.record_playing) / 2,
                    -4,
                    var17.wrapping_add(var16 << 24),
                );
                g.disable(gl::BLEND);
                g.pop_matrix();
            }
        }
        let var26 = if s.chat_open { 20 } else { 10 };
        let var31 = s.chat_open;
        g.enable(gl::BLEND);
        g.blend_func(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        g.disable(gl::ALPHA_TEST);
        g.push_matrix();
        g.translatef(0.0, (var7 - 48) as f32, 0.0);
        let n = self.chat_message_list.len().min(var26);
        for var17 in 0..n {
            let line = &self.chat_message_list[var17];
            let var20 = chat_line_alpha(line.update_counter, var31);
            if var20 > 0 {
                let var33 = 2;
                let var22 = -(var17 as i32) * 9;
                draw::draw_rect(
                    g,
                    var33,
                    var22 - 1,
                    var33 + 320,
                    var22 + 8,
                    (var20 / 2) << 24,
                );
                g.enable(gl::BLEND);
                var8.draw_string_with_shadow(
                    g,
                    &line.message,
                    var33,
                    var22,
                    16777215i32.wrapping_add(var20 << 24),
                );
            }
        }
        g.pop_matrix();
        g.enable(gl::ALPHA_TEST);
        g.disable(gl::BLEND);
    }

    /// `renderVignette(brightness, w, h)`: the darkening at the edges with
    /// fancy graphics, following the player's brightness slowly (1 % of the
    /// difference per frame), multiplied into the frame
    /// (`glBlendFunc(GL_ZERO, GL_ONE_MINUS_SRC_COLOR)`).
    // src: GuiIngame.java:283-310
    #[allow(clippy::manual_clamp)] // the Java's two tests, kept
    pub fn render_vignette(&mut self, g: &mut GuiGl, mut var1: f32, var2: i32, var3: i32) {
        var1 = 1.0 - var1;
        if var1 < 0.0 {
            var1 = 0.0;
        }
        if var1 > 1.0 {
            var1 = 1.0;
        }
        self.prev_vignette_brightness = (self.prev_vignette_brightness as f64
            + (var1 - self.prev_vignette_brightness) as f64 * 0.01)
            as f32;
        g.disable(gl::DEPTH_TEST);
        g.depth_mask(false);
        g.blend_func(gl::ZERO, gl::ONE_MINUS_SRC_COLOR);
        let b = self.prev_vignette_brightness;
        g.color4f(b, b, b, 1.0);
        g.bind_texture(VIGNETTE_PNG);
        full_screen_quad(g, var2, var3, [0.0, 0.0, 1.0, 1.0]);
        g.depth_mask(true);
        g.enable(gl::DEPTH_TEST);
        g.color4f(1.0, 1.0, 1.0, 1.0);
        g.blend_func(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
    }

    /// `renderInventorySlot(slot, x, y, partial)`: the hotbar item, squashed
    /// while it bobs after a pickup (`animationsToGo`), and its overlay.
    // src: GuiIngame.java:342-361
    #[allow(clippy::too_many_arguments)]
    pub fn render_inventory_slot(
        &mut self,
        ig: &mut ItemGl,
        g: &mut GuiGl,
        statics: &BlockStatics,
        font: &FontRenderer,
        var5: Option<&mc_sim::item::ItemStack>,
        var2: i32,
        var3: i32,
        var4: f32,
    ) {
        let Some(var5) = var5 else {
            return;
        };
        let var6 = var5.animations_to_go as f32 - var4;
        if var6 > 0.0 {
            ig.push_matrix(g);
            let var7 = 1.0 + var6 / 5.0;
            ig.translatef(g, (var2 + 8) as f32, (var3 + 12) as f32, 0.0);
            ig.scalef(g, 1.0 / var7, (var7 + 1.0) / 2.0, 1.0);
            ig.translatef(g, -(var2 + 8) as f32, -(var3 + 12) as f32, 0.0);
        }
        render_item_into_gui(ig, g, statics, Some(var5), var2, var3);
        if var6 > 0.0 {
            ig.pop_matrix(g);
        }
        render_item_overlay_into_gui(g, font, Some(var5), var2, var3);
    }
}

/// A screen-sized quad at z −90 with the texture rectangle `uv` (`u0, v0,
/// u1, v1`), as the three overlays draw it.
fn full_screen_quad(g: &mut GuiGl, var1: i32, var2: i32, uv: [f64; 4]) {
    let [u0, v0, u1, v1] = uv;
    g.start_drawing_quads();
    g.add_vertex_with_uv(0.0, var2 as f64, -90.0, u0, v1);
    g.add_vertex_with_uv(var1 as f64, var2 as f64, -90.0, u1, v1);
    g.add_vertex_with_uv(var1 as f64, 0.0, -90.0, u1, v0);
    g.add_vertex_with_uv(0.0, 0.0, -90.0, u0, v0);
    g.draw();
}

/// `renderPumpkinBlur(w, h)`: `/misc/pumpkinblur.png` over the screen,
/// blended, no depth test, no alpha test.
// src: GuiIngame.java:263-281
pub fn render_pumpkin_blur(g: &mut GuiGl, var1: i32, var2: i32) {
    g.disable(gl::DEPTH_TEST);
    g.depth_mask(false);
    g.blend_func(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
    g.color4f(1.0, 1.0, 1.0, 1.0);
    g.disable(gl::ALPHA_TEST);
    g.bind_texture(PUMPKIN_BLUR_PNG);
    full_screen_quad(g, var1, var2, [0.0, 0.0, 1.0, 1.0]);
    g.depth_mask(true);
    g.enable(gl::DEPTH_TEST);
    g.enable(gl::ALPHA_TEST);
    g.color4f(1.0, 1.0, 1.0, 1.0);
}

/// `renderPortalOverlay(time, w, h)`: the portal tile of `/terrain.png`
/// over the screen at [`portal_alpha`].
// src: GuiIngame.java:312-340
pub fn render_portal_overlay(g: &mut GuiGl, var1: f32, var2: i32, var3: i32) {
    let var1 = portal_alpha(var1);
    g.disable(gl::ALPHA_TEST);
    g.disable(gl::DEPTH_TEST);
    g.depth_mask(false);
    g.blend_func(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
    g.color4f(1.0, 1.0, 1.0, var1);
    g.bind_texture(super::render_item::TERRAIN_PNG);
    let var4 = (PORTAL_TEXTURE % 16) as f32 / 16.0;
    let var5 = (PORTAL_TEXTURE / 16) as f32 / 16.0;
    let var6 = (PORTAL_TEXTURE % 16 + 1) as f32 / 16.0;
    let var7 = (PORTAL_TEXTURE / 16 + 1) as f32 / 16.0;
    full_screen_quad(
        g,
        var2,
        var3,
        [var4 as f64, var5 as f64, var6 as f64, var7 as f64],
    );
    g.depth_mask(true);
    g.enable(gl::DEPTH_TEST);
    g.enable(gl::ALPHA_TEST);
    g.color4f(1.0, 1.0, 1.0, 1.0);
}

/// Runs `f` with the world and `thePlayer` (in the world, else the copy the
/// game keeps of a dropped player).
pub fn with_player<R>(game: &mut Game, f: impl FnOnce(&mut World, &mut Entity) -> R) -> Option<R> {
    let pid = game.the_player?;
    let world = game.the_world.as_mut()?;
    if world.entity(pid).is_some() {
        return world.with_entity_mut(pid, f);
    }
    let mut p = game.detached_player.clone()?;
    Some(f(world, &mut p))
}

/// [`super::GuiSet::Overlay`]: `ingameGUI.renderGameOverlay` (module docs).
// src: EntityRenderer.java:356-395
pub fn draw_game_overlay(
    mut game: NonSendMut<Game>,
    mut ingame: ResMut<GuiIngame>,
    mut g: ResMut<GuiGl>,
    font: Res<FontRenderer>,
    debug: Res<DebugText>,
) {
    let gm = &mut *game;
    if gm.pending.is_some() || gm.skip_render_world || gm.the_world.is_none() {
        return;
    }
    if gm.game_settings.hide_gui && gm.current_screen.is_none() {
        return;
    }
    let r = ScaledResolution::new(
        gm.game_settings.gui_scale,
        gm.display_width,
        gm.display_height,
    );
    let inputs = OverlayInputs {
        width: r.scaled_width,
        height: r.scaled_height,
        fancy: gm.is_fancy_graphics_enabled(),
        third_person: gm.game_settings.third_person_view,
        show_debug_info: gm.game_settings.show_debug_info,
        chat_open: gm
            .current_screen
            .as_ref()
            .is_some_and(|s| matches!(s.kind, ScreenKind::Chat | ScreenKind::SleepMp)),
        should_draw_hud: true,
    };
    let partial = gm.timer.render_partial_ticks;
    g.set_layer(GuiLayer::Overlay);
    let ing = &mut *ingame;
    let gl_ = &mut *g;
    with_player(gm, |w, p| {
        ing.render_game_overlay(w, p, &inputs, gl_, &font, &debug, partial);
    });
}

/// [`RunTickHook::IngameGuiUpdateTick`]: `ingameGUI.updateTick()`.
///
/// [`RunTickHook::IngameGuiUpdateTick`]: crate::game::RunTickHook::IngameGuiUpdateTick
// src: Minecraft.java:950
pub fn ingame_update_tick(mut ingame: ResMut<GuiIngame>) {
    ingame.update_tick();
}

/// The message cursors of [`collect_gui_messages`] (shared by every
/// schedule it runs in).
#[derive(Resource, Default)]
pub struct GuiMessageCursors {
    client: MessageCursor<ClientEvent>,
    record: MessageCursor<RecordPlayingMessage>,
}

/// The chat lines, the record line, the achievement hints and the
/// screens the player asked for (module docs; the screens are opened by
/// [`super::container::open_player_gui`]).
#[allow(clippy::too_many_arguments)]
pub fn collect_gui_messages(
    mut cursors: ResMut<GuiMessageCursors>,
    client: Option<Res<Messages<ClientEvent>>>,
    record: Option<Res<Messages<RecordPlayingMessage>>>,
    mut ingame: ResMut<GuiIngame>,
    mut achievement: ResMut<super::achievement::GuiAchievement>,
    font: Res<FontRenderer>,
    lang: Res<Lang>,
    mut pending: ResMut<super::container::PendingPlayerGuis>,
) {
    let GuiMessageCursors {
        client: cc,
        record: rc,
    } = &mut *cursors;
    let events: Vec<ClientEvent> = client
        .map(|c| cc.read(&c).cloned().collect())
        .unwrap_or_default();
    for e in &events {
        match e {
            // src: Minecraft.java:663 (the screenshot line)
            ClientEvent::ChatMessage(s) => ingame.add_chat_message(&font, s),
            // src: EntityPlayerSP.java:172-174
            ClientEvent::ChatMessageTranslate(k) => {
                ingame.add_chat_message_translate(&lang.0, &font, k)
            }
            // src: Minecraft.java:427-429
            ClientEvent::ClearChat => ingame.clear_chat_messages(),
            // src: EntityPlayerSP.java:35-37
            ClientEvent::AchievementHint { stat_id } => {
                achievement.hint_requested = Some(*stat_id);
            }
            // src: EntityPlayerSP.java:113-131
            ClientEvent::DisplayGui(gui) => pending.0.push(gui.clone()),
            _ => {}
        }
    }
    if let Some(record) = record {
        for RecordPlayingMessage(s) in rc.read(&record) {
            // src: RenderGlobal.java:1257 (ingameGUI.setRecordPlayingMessage)
            ingame.set_record_playing_message(s);
        }
    }
}

/// `Minecraft.displayDebugInfo`'s graph (the bookkeeping is the shell's,
/// `Game::frame_times`): the 512 last frame times (red past 1/60 s, else
/// green, fading with age) and tick times (grey) as one-pixel columns at
/// the bottom left, over two bars and the average, in window pixels (the
/// GUI's units are `scaleFactor` window pixels). Drawn after the screen,
/// with the depth buffer cleared (no depth test here).
// src: Minecraft.java:579-583, :670-744
pub fn draw_debug_graph(game: NonSend<Game>, mut g: ResMut<GuiGl>) {
    if !game.game_settings.show_debug_info || game.pending.is_some() {
        return;
    }
    let r = ScaledResolution::new(
        game.game_settings.gui_scale,
        game.display_width,
        game.display_height,
    );
    let f = r.scale_factor as f64;
    g.set_layer(GuiLayer::Screen);
    let saved = g.state.clone();
    g.disable(gl::DEPTH_TEST);
    g.disable(gl::BLEND);
    g.disable(gl::TEXTURE_2D);
    for q in debug_graph_quads(
        game.display_height,
        &game.frame_times[..],
        &game.tick_times[..],
        game.num_recorded_frame_times,
    ) {
        g.start_drawing_quads();
        g.set_color_opaque_i(q.color);
        let [x0, y0, x1, y1] = q.rect;
        g.add_vertex(x0 / f, y0 / f, 0.0);
        g.add_vertex(x0 / f, y1 / f, 0.0);
        g.add_vertex(x1 / f, y1 / f, 0.0);
        g.add_vertex(x1 / f, y0 / f, 0.0);
        g.draw();
    }
    g.state = saved;
}

/// One rectangle of the debug graph, in window pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GraphQuad {
    /// `x0, y0, x1, y1`.
    pub rect: [f64; 4],
    /// `setColorOpaque_I`'s colour.
    pub color: i32,
}

/// The debug graph's rectangles (module docs of [`draw_debug_graph`]):
/// the two 60-fps bars and the average as quads, each `GL_LINES` column
/// (from `(x + 0.5, y0 + 0.5)` to `(x + 0.5, y1 + 0.5)`) as the pixels a
/// one-pixel line covers, `x..x + 1` by `y0..y1`.
// src: Minecraft.java:671-744
pub fn debug_graph_quads(
    display_height: i32,
    frame_times: &[i64],
    tick_times: &[i64],
    num_recorded: i32,
) -> Vec<GraphQuad> {
    let var3: i64 = 16666666;
    let h = display_height as f64;
    let len = frame_times.len() as f64;
    let var8 = (var3 / 200000) as f64;
    let mut out = vec![
        GraphQuad {
            rect: [0.0, h - var8, len, h],
            color: 536870912,
        },
        GraphQuad {
            rect: [0.0, h - var8 * 2.0, len, h - var8],
            color: 538968064,
        },
    ];
    let var9: i64 = frame_times.iter().sum();
    let var11 = (var9 / 200000 / frame_times.len() as i64) as f64;
    out.push(GraphQuad {
        rect: [0.0, h - var11, len, h],
        color: 541065216,
    });
    let n = frame_times.len() as i32;
    for var12 in 0..frame_times.len() {
        let var13 = ((var12 as i32).wrapping_sub(num_recorded) & (n - 1)) * 255 / n;
        let mut var14 = var13 * var13 / 255;
        var14 = var14 * var14 / 255;
        let c = if frame_times[var12] > var3 {
            -16777216 + var14 * 65536
        } else {
            -16777216 + var14 * 256
        };
        let var16 = (frame_times[var12] / 200000) as f64;
        let var18 = (tick_times[var12] / 200000) as f64;
        let x = var12 as f64;
        let (top, bottom) = (h - var16, h);
        out.push(GraphQuad {
            rect: [x, top.min(bottom), x + 1.0, top.max(bottom)],
            color: c,
        });
        let (a, b) = (h - var16, h - (var16 - var18));
        out.push(GraphQuad {
            rect: [x, a.min(b), x + 1.0, a.max(b)],
            color: -16777216 + var14 * 65536 + var14 * 256 + var14,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::font::ChatAllowedCharacters;
    use crate::textures::ArgbImage;

    #[test]
    fn hotbar_geometry() {
        // 427×240 (854×480 at scale 2), slot 3 selected.
        let [bg, sel] = hotbar_rects(427, 240, 3);
        assert_eq!(bg, (122, 218, 0, 0, 182, 22));
        assert_eq!(sel, (121 + 60, 217, 0, 22, 24, 22));
        assert_eq!(hotbar_slot_pos(427, 240, 0), (125, 221));
        assert_eq!(hotbar_slot_pos(427, 240, 8), (285, 221));
    }

    #[test]
    fn hearts_full_half_and_armor() {
        let mut r = JavaRandom::new(0);
        // 15 health, no flash, armour 7: 7 full hearts, one half, armour
        // 3 full + 1 half + 6 empty.
        let rows = health_rows(427, 240, 0, 15, 15, 7, 0, &mut r);
        let full: Vec<_> = rows.iter().filter(|q| (q.2, q.3) == (52, 0)).collect();
        let half: Vec<_> = rows.iter().filter(|q| (q.2, q.3) == (61, 0)).collect();
        assert_eq!((full.len(), half.len()), (7, 1));
        assert_eq!(*half[0], (122 + 7 * 8, 208, 61, 0));
        let armor: Vec<_> = rows.iter().filter(|q| q.3 == 9).map(|q| q.2).collect();
        assert_eq!(armor, vec![34, 34, 34, 25, 16, 16, 16, 16, 16, 16]);
        // The armour row starts at the right: x = w/2 + 91 - 9.
        assert_eq!(rows[0], (213 + 91 - 9, 208, 34, 9));
        // Outlines: 10, not flashing (u 16).
        assert_eq!(rows.iter().filter(|q| (q.2, q.3) == (16, 0)).count(), 10);
    }

    #[test]
    fn hearts_flash_and_jitter() {
        // heartsLife 12: 12 / 3 % 2 == 0, no flash; 10: 3 % 2 == 1, flash.
        let mut r = JavaRandom::new(0);
        let rows = health_rows(427, 240, 12, 10, 14, 0, 0, &mut r);
        assert!(rows.iter().all(|q| q.2 != 25 && q.2 != 70 && q.2 != 79));
        let rows = health_rows(427, 240, 10, 10, 14, 0, 0, &mut r);
        assert_eq!(rows.iter().filter(|q| q.2 == 25).count(), 10);
        // Previous health 14: 7 white full hearts (70), no white half.
        assert_eq!(rows.iter().filter(|q| q.2 == 70).count(), 7);
        // heartsLife below 10 never flashes.
        let rows = health_rows(427, 240, 9, 10, 14, 0, 0, &mut r);
        assert!(rows.iter().all(|q| q.2 != 25));
        // At 4 health the hearts jitter by rand.nextInt(2) from the seed
        // updateCounter * 312871, one draw per heart.
        let rows = health_rows(427, 240, 0, 4, 4, 0, 17, &mut r);
        let mut expect = JavaRandom::new(0);
        expect.set_seed((17 * 312871) as i64);
        let outlines: Vec<_> = rows.iter().filter(|q| q.2 == 16).collect();
        for (i, q) in outlines.iter().enumerate() {
            assert_eq!(q.0, 213 - 91 + i as i32 * 8);
            assert_eq!(q.1, 208 + expect.next_int_bounded(2));
        }
        // At 5 no jitter.
        let rows = health_rows(427, 240, 0, 5, 5, 0, 17, &mut r);
        assert!(rows.iter().all(|q| q.1 == 208));
    }

    #[test]
    fn air_bubble_counts() {
        assert_eq!(air_bubbles(300), (10, 0));
        // ceil(298 / 30) = 10, ceil(300 / 30) - 10 = 0.
        assert_eq!(air_bubbles(299), (10, 0));
        // 150: ceil(4.93) = 5 full, ceil(5) - 5 = 0.
        assert_eq!(air_bubbles(150), (5, 0));
        // 151: ceil(4.97) = 5, ceil(5.03) = 6: one popping.
        assert_eq!(air_bubbles(151), (5, 1));
        assert_eq!(air_bubbles(0), (0, 0));
        // Drowning (air -20): nothing (negative counts).
        assert_eq!(air_bubbles(-20), (0, 0));
    }

    #[test]
    fn chat_fade() {
        assert_eq!(chat_line_alpha(0, false), 255);
        // 180: (1 - 0.9) * 10 is 0.9999999999999998 in doubles: 254.
        assert_eq!(chat_line_alpha(180, false), 254);
        assert_eq!(chat_line_alpha(179, false), 255);
        // 190: (1 - 0.95) * 10 = 0.5 → 0.25 → 63.
        assert_eq!(chat_line_alpha(190, false), 63);
        assert_eq!(chat_line_alpha(199, false), 0);
        assert_eq!(chat_line_alpha(200, false), 0);
        // Open chat: every line at full opacity.
        assert_eq!(chat_line_alpha(5000, true), 255);
    }

    fn font() -> FontRenderer {
        // 'a'..'z' all 6 pixels wide (glyph columns 0..4 set).
        let mut sheet = ArgbImage::new(128, 128);
        for g in 0..256u32 {
            for x in 0..4 {
                let px = (g % 16) * 8 + x;
                let py = (g / 16) * 8;
                sheet.pixels[(py * 128 + px) as usize] = -1;
            }
        }
        let chars: Vec<char> = (32u8..127).map(char::from).collect();
        FontRenderer::new(&sheet, false, ChatAllowedCharacters::from_chars(chars))
    }

    #[test]
    fn chat_wraps_at_320_pixels_and_keeps_50() {
        let f = font();
        let w = f.get_string_width("a");
        assert!(w > 0);
        let per_line = (CHAT_WIDTH / w) as usize;
        let mut ing = GuiIngame::default();
        let long: String = "a".repeat(per_line * 2 + 3);
        ing.add_chat_message(&f, &long);
        // Newest first: the tail, then the two full pieces.
        assert_eq!(ing.chat_message_list.len(), 3);
        assert_eq!(ing.chat_message_list[0].message.len(), 3);
        assert_eq!(ing.chat_message_list[1].message.len(), per_line);
        for i in 0..60 {
            ing.add_chat_message(&f, &format!("l{i}"));
        }
        assert_eq!(ing.chat_message_list.len(), CHAT_MAX_LINES);
        assert_eq!(ing.chat_message_list[0].message, "l59");
        ing.update_tick();
        assert_eq!(ing.chat_message_list[0].update_counter, 1);
        assert_eq!(ing.update_counter, 1);
    }

    #[test]
    fn record_line() {
        let mut ing = GuiIngame::default();
        ing.set_record_playing_message("C418 - 13");
        assert_eq!(ing.record_playing, "Now playing: C418 - 13");
        assert_eq!(ing.record_playing_up_for, 60);
        assert_eq!(record_alpha(60, 0.0), 255);
        // 1 tick left at partial 0.5: 0.5 * 256 / 20 = 6.
        assert_eq!(record_alpha(1, 0.5), 6);
        ing.update_tick();
        assert_eq!(ing.record_playing_up_for, 59);
    }

    #[test]
    fn hsb() {
        // Pure hues.
        assert_eq!(hsb_to_rgb(0.0, 1.0, 1.0) & 0xffffff, 0xff0000);
        assert_eq!(hsb_to_rgb(1.0 / 3.0, 1.0, 1.0) & 0xffffff, 0x00ff00);
        assert_eq!(hsb_to_rgb(0.5, 0.0, 0.5) & 0xffffff, 0x808080);
        // The record colour at the start: hue 60/50 = 1.2 → 0.2.
        let c = hsb_to_rgb(1.2, 0.7, 0.6);
        assert_eq!(c >> 24 & 255, 255);
        // sector 1: r = q = 0.6 × (1 - 0.7 × 0.2) → 132, g = v → 153,
        // b = p = 0.18 → 46.
        assert_eq!(c & 0xffffff, (132 << 16) | (153 << 8) | 46);
    }

    #[test]
    fn overlays() {
        // Sleep fade: 50 ticks → alpha 110; 105 → 1 - 0.5 → 110.
        assert_eq!(sleep_overlay_color(50) as u32, (110u32 << 24) | 0x101020);
        assert_eq!(sleep_overlay_color(105) as u32, (110u32 << 24) | 0x101020);
        assert_eq!(portal_alpha(1.0), 1.0);
        assert_eq!(portal_alpha(0.5), 0.0625 * 0.8 + 0.2);
    }

    #[test]
    fn graph_columns() {
        let ft = [16_666_667i64, 1_000_000];
        let tt = [400_000i64, 200_000];
        let q = debug_graph_quads(480, &ft, &tt, 0);
        assert_eq!(q[0].rect, [0.0, 480.0 - 83.0, 2.0, 480.0]);
        // Column 0: over 1/60 s, red; 83 pixels high.
        assert_eq!(q[3].rect, [0.0, 480.0 - 83.0, 1.0, 480.0]);
        assert_eq!(q[3].color & 0xff00, 0);
        // Its tick part: from the top down by 2 pixels.
        assert_eq!(q[4].rect, [0.0, 480.0 - 83.0, 1.0, 480.0 - 81.0]);
        assert_eq!(q[5].rect, [1.0, 475.0, 2.0, 480.0]);
    }
}
