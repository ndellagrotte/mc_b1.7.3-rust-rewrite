//! The application states, the current-screen slot (`Minecraft.currentScreen`)
//! and `ScaledResolution` (ScaledResolution.java).
//!
//! [`AppState`] is derived from the game every frame
//! ([`crate::game::sync_app_state`]): `Title` (no world), `Loading` (a
//! world is being opened or built), `InGame`, `Paused`
//! (`Minecraft.isGamePaused`). Systems of later WPs can run in a state with
//! `run_if(in_state(..))`; the game itself is driven by
//! `Minecraft`'s own fields, as vanilla is.
//!
//! [`GuiScreen`] is the slot WP-3.6 fills: the shell knows the screens
//! `Minecraft.java` opens itself and the two properties it reads
//! (`doesGuiPauseGame`, `allowUserInput`); drawing and the screens' own
//! input are WP-3.6's ([`crate::game::RunTickHook::ScreenInput`]).

use bevy::prelude::*;

/// The client's coarse state (module docs).
#[derive(States, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AppState {
    /// No world: the title screen and the menus.
    #[default]
    Title,
    /// `changeWorld` / `respawn` building terrain (the loading screen).
    Loading,
    /// A world, not paused.
    InGame,
    /// A world with a pausing screen open (`isGamePaused`).
    Paused,
}

/// Which screen (module docs). The vanilla classes `Minecraft.java` names;
/// WP-3.6 adds its own through [`ScreenKind::Other`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScreenKind {
    /// `GuiMainMenu`.
    MainMenu,
    /// `GuiIngameMenu` (Escape).
    IngameMenu,
    /// `GuiGameOver`.
    GameOver,
    /// `GuiInventory` (the inventory key).
    Inventory,
    /// `GuiChat` (multiplayer only in vanilla).
    Chat,
    /// `GuiSleepMP` (multiplayer only).
    SleepMp,
    /// `GuiConflictWarning`: the session lock was lost.
    ConflictWarning,
    /// `GuiErrorScreen`: out of memory.
    ErrorScreen,
    /// `GuiUnused`: `displayGuiScreen` refuses to replace it.
    Unused,
    /// Any other screen, by class name.
    Other(String),
}

/// One open screen: its kind, `width`/`height` (set by
/// `setWorldAndResolution`) and the two flags the shell reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuiScreen {
    /// The screen.
    pub kind: ScreenKind,
    /// `width` (scaled pixels).
    pub width: i32,
    /// `height`.
    pub height: i32,
    /// `allowUserInput` (GuiScreen.java:18: false; `GuiInventory` true,
    /// GuiInventory.java:11).
    pub allow_user_input: bool,
    /// `doesGuiPauseGame()` (GuiScreen.java:163-165: true; `GuiGameOver`
    /// and `GuiContainer` false, GuiGameOver.java:45-47,
    /// GuiContainer.java:168-170).
    pub pauses_game: bool,
}

impl GuiScreen {
    /// A screen of `kind` with its class's flags.
    pub fn new(kind: ScreenKind) -> Self {
        let (allow_user_input, pauses_game) = match kind {
            ScreenKind::Inventory => (true, false),
            ScreenKind::GameOver => (false, false),
            _ => (false, true),
        };
        GuiScreen {
            kind,
            width: 0,
            height: 0,
            allow_user_input,
            pauses_game,
        }
    }

    /// `doesGuiPauseGame()`.
    pub fn does_gui_pause_game(&self) -> bool {
        self.pauses_game
    }
}

/// `ScaledResolution`: the GUI's scaled size for a window size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScaledResolution {
    /// `scaledWidth` (private; `getScaledWidth()`).
    pub scaled_width: i32,
    /// `scaledHeight`.
    pub scaled_height: i32,
    /// `scaledWidthD`.
    pub scaled_width_d: f64,
    /// `scaledHeightD`.
    pub scaled_height_d: f64,
    /// `scaleFactor`.
    pub scale_factor: i32,
}

impl ScaledResolution {
    /// `new ScaledResolution(settings, width, height)`: the largest factor
    /// up to `guiScale` (0 = no limit) that keeps at least 320×240 scaled
    /// pixels.
    // src: ScaledResolution.java:10-27
    pub fn new(gui_scale: i32, var2: i32, var3: i32) -> Self {
        let mut scale_factor = 1;
        let mut var4 = gui_scale;
        if var4 == 0 {
            var4 = 1000;
        }
        while scale_factor < var4
            && var2 / (scale_factor + 1) >= 320
            && var3 / (scale_factor + 1) >= 240
        {
            scale_factor += 1;
        }
        let scaled_width_d = var2 as f64 / scale_factor as f64;
        let scaled_height_d = var3 as f64 / scale_factor as f64;
        ScaledResolution {
            scaled_width: scaled_width_d.ceil() as i32,
            scaled_height: scaled_height_d.ceil() as i32,
            scaled_width_d,
            scaled_height_d,
            scale_factor,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaled_resolution() {
        // 854×480 (the default window): factor 2 (427×240).
        let r = ScaledResolution::new(0, 854, 480);
        assert_eq!(r.scale_factor, 2);
        assert_eq!((r.scaled_width, r.scaled_height), (427, 240));
        // 1920×1080 auto: factor 4 (480×270).
        let r = ScaledResolution::new(0, 1920, 1080);
        assert_eq!(r.scale_factor, 4);
        assert_eq!((r.scaled_width, r.scaled_height), (480, 270));
        // Small (1) caps it.
        let r = ScaledResolution::new(1, 1920, 1080);
        assert_eq!(r.scale_factor, 1);
        // Odd sizes round up.
        let r = ScaledResolution::new(0, 1001, 721);
        assert_eq!(r.scale_factor, 3);
        assert_eq!((r.scaled_width, r.scaled_height), (334, 241));
    }

    #[test]
    fn screen_flags() {
        assert!(GuiScreen::new(ScreenKind::IngameMenu).does_gui_pause_game());
        assert!(!GuiScreen::new(ScreenKind::GameOver).does_gui_pause_game());
        let inv = GuiScreen::new(ScreenKind::Inventory);
        assert!(inv.allow_user_input && !inv.pauses_game);
        assert!(!GuiScreen::new(ScreenKind::MainMenu).allow_user_input);
    }
}
