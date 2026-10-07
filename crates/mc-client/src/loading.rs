//! `LoadingScreenRenderer` (LoadingScreenRenderer.java) as state, and
//! `IProgressUpdate` (IProgressUpdate.java): the loading screen's two lines
//! and progress. Drawing them (the dirt background, the bar, the centred
//! text) is WP-3.6's, from [`LoadingScreenRenderer`] in
//! [`crate::game::Game::loading_screen`].
//!
//! The strings the shell sets: `changeWorld`'s title (`"Generating
//! level"`, `"Loading level"`, `"Entering the Nether"`, `"Leaving the
//! Nether"`, `"Respawning"`, or empty when leaving a world), `func_6255_d`'s
//! `"Building terrain"` and `"Simulating world for a bit"`, and
//! `World.saveWorld`'s `"Saving level"` / `"Saving chunks"`
//! (World.java:278-291).
//!
//! `MinecraftError` (thrown when the game stops while loading, to unwind
//! the loading loop) is not needed: the shell's loading is incremental and
//! simply stops.

/// `IProgressUpdate`.
// src: IProgressUpdate.java:3-9
pub trait IProgressUpdate {
    /// `displayProgressMessage(title)`.
    fn display_progress_message(&mut self, var1: &str);
    /// `displayLoadingString(text)`.
    fn display_loading_string(&mut self, var1: &str);
    /// `setLoadingProgress(percent)`; negative hides the bar.
    fn set_loading_progress(&mut self, var1: i32);
}

/// `LoadingScreenRenderer`'s state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LoadingScreenRenderer {
    /// `field_1004_a`: the second line (`displayLoadingString`'s text;
    /// docs/mappings.md).
    // src: LoadingScreenRenderer.java:6
    pub field_1004_a: String,
    /// `currentlyDisplayedText`: the title line.
    // src: LoadingScreenRenderer.java:8
    pub currently_displayed_text: String,
    /// `field_1005_e`: set by `displayProgressMessage`, cleared by
    /// `printText`; whether stopping the game may interrupt (vanilla throws
    /// `MinecraftError` unless set).
    // src: LoadingScreenRenderer.java:10
    pub field_1005_e: bool,
    /// The last `setLoadingProgress(int)` value, -1 for none (the bar is
    /// drawn only for `>= 0`, LoadingScreenRenderer.java:155).
    pub progress: i32,
}

impl LoadingScreenRenderer {
    /// `printText(title)`.
    // src: LoadingScreenRenderer.java:18-21
    pub fn print_text(&mut self, var1: &str) {
        self.field_1005_e = false;
        self.set_loading_title(var1);
    }

    /// `setLoadingProgress(String)` (the overload that sets the title).
    // src: LoadingScreenRenderer.java:28-44
    fn set_loading_title(&mut self, var1: &str) {
        self.currently_displayed_text = var1.to_string();
    }
}

impl IProgressUpdate for LoadingScreenRenderer {
    /// `displayProgressMessage(title)`: vanilla re-sets the *current* title
    /// (its argument is ignored, a vanilla quirk transcribed).
    // src: LoadingScreenRenderer.java:23-26
    fn display_progress_message(&mut self, _var1: &str) {
        self.field_1005_e = true;
        let t = self.currently_displayed_text.clone();
        self.set_loading_title(&t);
    }

    // src: LoadingScreenRenderer.java:46-57
    fn display_loading_string(&mut self, var1: &str) {
        self.field_1004_a = var1.to_string();
        self.set_loading_progress(-1);
    }

    // src: LoadingScreenRenderer.java:59-122
    fn set_loading_progress(&mut self, var1: i32) {
        self.progress = var1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages() {
        let mut l = LoadingScreenRenderer::default();
        l.print_text("Generating level");
        l.display_loading_string("Building terrain");
        assert_eq!(l.currently_displayed_text, "Generating level");
        assert_eq!(l.field_1004_a, "Building terrain");
        assert_eq!(l.progress, -1);
        l.set_loading_progress(42);
        assert_eq!(l.progress, 42);
        l.display_progress_message("Saving level");
        assert!(l.field_1005_e);
        assert_eq!(l.currently_displayed_text, "Generating level");
    }
}
