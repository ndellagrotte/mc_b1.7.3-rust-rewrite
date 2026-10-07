//! `GuiMainMenu` (GuiMainMenu.java): the title screen — the logo, the
//! rotated, pulsing splash text, Singleplayer / Multiplayer (deferred, D3:
//! always disabled) / Mods and Texture Packs / Options... / Quit Game, the
//! version string and the copyright line.
//!
//! The splash lines come from the jar's `/title/splashes.txt`
//! ([`set_splashes`], read once at start-up by
//! [`crate::gui::screens_menu::MenuPlugin`]); `GuiMainMenu.rand` is one
//! clock-seeded `Random` for the class ([`RAND`]).

use std::sync::{LazyLock, Mutex, OnceLock};

use mc_core::java_random::JavaRandom;
use mc_core::math;

use super::HIDE_QUIT_BUTTON;
use super::options::GuiOptions;
use super::select_world::GuiSelectWorld;
use super::texture_packs::GuiTexturePacks;
use crate::app::ScreenKind;
use crate::clock;
use crate::gui::batch::GuiGl;
use crate::gui::draw;
use crate::gui::font::java_trim;
use crate::gui::screen::{
    Screen, ScreenBase, ScreenCtx, default_draw_screen, draw_default_background,
};
use crate::gui::widgets::button::GuiButton;

/// The title logo's sheet.
// src: GuiMainMenu.java:104
pub const MCLOGO_PNG: &str = "/title/mclogo.png";
/// The splash lines' file in the jar.
// src: GuiMainMenu.java:21
pub const SPLASHES_TXT: &str = "/title/splashes.txt";

/// The version string, bottom left.
// src: GuiMainMenu.java:117
pub const VERSION: &str = "Minecraft Beta 1.7.3";
/// Its colour (`5263440`, `0x505050`).
// src: GuiMainMenu.java:117
pub const VERSION_COLOR: i32 = 5263440;
/// The copyright line, bottom right.
// src: GuiMainMenu.java:118
pub const COPYRIGHT: &str = "Copyright Mojang AB. Do not distribute.";
/// The splash colour (`16776960`, `0xffff00`).
// src: GuiMainMenu.java:115
pub const SPLASH_COLOR: i32 = 16776960;

/// `/title/splashes.txt`'s non-empty trimmed lines, once read.
static SPLASHES: OnceLock<Vec<String>> = OnceLock::new();

/// `GuiMainMenu.rand`: `private static final Random rand = new Random()`.
// src: GuiMainMenu.java:13
pub static RAND: LazyLock<Mutex<JavaRandom>> =
    LazyLock::new(|| Mutex::new(JavaRandom::new(clock::new_random_seed())));

/// The constructor's reader loop over the file's bytes (UTF-8): every line
/// (`readLine` splits at `\n`, `\r` or `\r\n`), trimmed, kept when not
/// empty.
// src: GuiMainMenu.java:19-29
pub fn parse_splashes(bytes: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(bytes);
    let mut out = Vec::new();
    for line in text.split('\n') {
        for piece in line.split('\r') {
            let t = java_trim(piece);
            if !t.is_empty() {
                out.push(t.to_string());
            }
        }
    }
    out
}

/// Stores the jar's splash file (`None`: missing; every title screen then
/// keeps `"missingno"`, as the Java's caught exception does).
pub fn set_splashes(bytes: Option<&[u8]>) {
    let _ = SPLASHES.set(bytes.map(parse_splashes).unwrap_or_default());
}

/// The splash of a new title screen: `var1.get(rand.nextInt(var1.size()))`,
/// or `"missingno"` when the list is empty (`nextInt(0)` throws, caught).
// src: GuiMainMenu.java:15, :31-33
pub fn pick_splash(lines: &[String], rand: &mut JavaRandom) -> String {
    if lines.is_empty() {
        return "missingno".to_string();
    }
    let i = rand.next_int_bounded(lines.len() as i32);
    lines[i as usize].clone()
}

/// `initGui`'s date override (`Calendar` month is 0-based, hence `+ 1`).
// src: GuiMainMenu.java:45-55
pub fn date_splash(month: u32, day: u32) -> Option<&'static str> {
    if month == 11 && day == 9 {
        Some("Happy birthday, ez!")
    } else if month == 6 && day == 1 {
        Some("Happy birthday, Notch!")
    } else if month == 12 && day == 24 {
        Some("Merry X-mas!")
    } else if month == 1 && day == 1 {
        Some("Happy new year!")
    } else {
        None
    }
}

/// `GuiMainMenu`.
pub struct GuiMainMenu {
    base: ScreenBase,
    /// `updateCounter` (ticks open; counted, never read).
    pub update_counter: f32,
    /// `splashText`.
    pub splash_text: String,
    /// `multiplayerButton`, as its index in the control list.
    multiplayer_button: Option<usize>,
}

impl Default for GuiMainMenu {
    /// `new GuiMainMenu()`.
    // src: GuiMainMenu.java:18-35
    fn default() -> Self {
        let lines = SPLASHES.get().map(Vec::as_slice).unwrap_or_default();
        let splash_text = match RAND.lock() {
            Ok(mut r) => pick_splash(lines, &mut r),
            Err(_) => "missingno".to_string(),
        };
        GuiMainMenu {
            base: ScreenBase::default(),
            update_counter: 0.0,
            splash_text,
            multiplayer_button: None,
        }
    }
}

impl Screen for GuiMainMenu {
    fn base(&self) -> &ScreenBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn kind(&self) -> ScreenKind {
        ScreenKind::MainMenu
    }

    // src: GuiMainMenu.java:37-39
    fn update_screen(&mut self, _ctx: &mut ScreenCtx) {
        self.update_counter += 1.0;
    }

    // src: GuiMainMenu.java:41-42
    fn key_typed(&mut self, _ctx: &mut ScreenCtx, _ch: char, _key: i32) {}

    // src: GuiMainMenu.java:44-73
    fn init_gui(&mut self, ctx: &mut ScreenCtx) {
        let now = crate::screenshot::local_time(clock::current_time_millis());
        if let Some(s) = date_splash(now.month, now.day) {
            self.splash_text = s.to_string();
        }
        let var2 = ctx.lang;
        let (w, h) = (self.base.width, self.base.height);
        let var4 = h / 4 + 48;
        let c = &mut self.base.control_list;
        c.push(GuiButton::new(
            1,
            w / 2 - 100,
            var4,
            var2.translate_key("menu.singleplayer"),
        ));
        c.push(GuiButton::new(
            2,
            w / 2 - 100,
            var4 + 24,
            var2.translate_key("menu.multiplayer"),
        ));
        self.multiplayer_button = Some(c.len() - 1);
        c.push(GuiButton::new(
            3,
            w / 2 - 100,
            var4 + 48,
            var2.translate_key("menu.mods"),
        ));
        if HIDE_QUIT_BUTTON {
            c.push(GuiButton::new(
                0,
                w / 2 - 100,
                var4 + 72,
                var2.translate_key("menu.options"),
            ));
        } else {
            c.push(GuiButton::with_size(
                0,
                w / 2 - 100,
                var4 + 72 + 12,
                98,
                20,
                var2.translate_key("menu.options"),
            ));
            c.push(GuiButton::with_size(
                4,
                w / 2 + 2,
                var4 + 72 + 12,
                98,
                20,
                var2.translate_key("menu.quit"),
            ));
        }
        // `if (mc.session == null) multiplayerButton.enabled = false`: the
        // session is never null here, but multiplayer is deferred (D3), so
        // the button is always disabled.
        if let Some(i) = self.multiplayer_button {
            c[i].enabled = false;
        }
    }

    // src: GuiMainMenu.java:75-96
    fn action_performed(&mut self, ctx: &mut ScreenCtx, button: usize) {
        let id = self.base.control_list[button].id;
        if id == 0 {
            ctx.display_wrapping_this(|this| Box::new(GuiOptions::new(Some(this))));
        }
        if id == 1 {
            ctx.display_wrapping_this(|this| Box::new(GuiSelectWorld::new(Some(this))));
        }
        // id 2: `new GuiMultiplayer(this)`: deferred (D3); the button is
        // disabled.
        if id == 3 {
            ctx.display_wrapping_this(|this| Box::new(GuiTexturePacks::new(Some(this))));
        }
        if id == 4 {
            ctx.game.shutdown();
        }
    }

    // src: GuiMainMenu.java:98-121
    fn draw_screen(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, mx: i32, my: i32, _p: f32) {
        draw_default_background(&self.base, ctx, g);
        let (w, h) = (self.base.width, self.base.height);
        let f = ctx.font;
        let var5: i32 = 274;
        let var6 = w / 2 - var5 / 2;
        let var7: i32 = 30;
        g.bind_texture(MCLOGO_PNG);
        g.color4f(1.0, 1.0, 1.0, 1.0);
        let z = self.base.z_level;
        draw::draw_textured_modal_rect(g, z, var6, var7, 0, 0, 155, 44);
        draw::draw_textured_modal_rect(g, z, var6 + 155, var7, 0, 45, 155, 44);
        // `var4.setColorOpaque_I(16777215)` outside a draw: the next
        // `startDrawingQuads` resets the Tessellator's colour, so it has no
        // effect (Tessellator.java:155-170).
        g.push_matrix();
        g.translatef((w / 2 + 90) as f32, 70.0, 0.0);
        g.rotatef(-20.0, 0.0, 0.0, 1.0);
        let t = (clock::current_time_millis() % 1000) as f32 / 1000.0 * 3.1415927 * 2.0;
        let mut var8 = 1.8 - math::abs(math::sin(t) * 0.1);
        var8 = var8 * 100.0 / (f.get_string_width(&self.splash_text) + 32) as f32;
        g.scalef(var8, var8, var8);
        draw::draw_centered_string(g, f, &self.splash_text, 0, -8, SPLASH_COLOR);
        g.pop_matrix();
        draw::draw_string(g, f, VERSION, 2, 2, VERSION_COLOR);
        let var9 = COPYRIGHT;
        draw::draw_string(
            g,
            f,
            var9,
            w - f.get_string_width(var9) - 2,
            h - 10,
            16777215,
        );
        default_draw_screen(&mut self.base, ctx, g, mx, my);
    }
}

/// `new GuiMainMenu()` as a boxed screen (the registry's factory).
pub fn create() -> Box<dyn Screen> {
    Box::new(GuiMainMenu::default())
}

/// `mc.displayGuiScreen(new GuiMainMenu())`.
pub fn display(ctx: &mut ScreenCtx) {
    ctx.display_gui_screen(Some(create()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splash_lines_are_trimmed_and_blank_ones_skipped() {
        let l = parse_splashes(b"  Hello!  \r\n\nAs seen on TV!\r\t \rLast\n");
        assert_eq!(l, vec!["Hello!", "As seen on TV!", "Last"]);
        assert_eq!(pick_splash(&[], &mut JavaRandom::new(1)), "missingno");
        // new Random(0).nextInt(3) == 0 (java.util.Random).
        let three = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let mut r = JavaRandom::new(0);
        let mut check = JavaRandom::new(0);
        let i = check.next_int_bounded(3) as usize;
        assert_eq!(pick_splash(&three, &mut r), three[i]);
    }

    #[test]
    fn date_overrides() {
        assert_eq!(date_splash(11, 9), Some("Happy birthday, ez!"));
        assert_eq!(date_splash(6, 1), Some("Happy birthday, Notch!"));
        assert_eq!(date_splash(12, 24), Some("Merry X-mas!"));
        assert_eq!(date_splash(1, 1), Some("Happy new year!"));
        assert_eq!(date_splash(7, 4), None);
    }
}
