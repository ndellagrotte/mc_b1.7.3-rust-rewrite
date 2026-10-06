//! The Bevy shell around `mc-sim` (PLAN_v4 D1, §3.5).
//!
//! Owns the window, input, rendering, UI, audio and asset loading, and drives
//! the simulation from one system. Reads textures and `lang/` from the user's
//! own `minecraft.jar` and sounds from a `resources/` directory at runtime
//! (D8). Empty until Phase 3.

use bevy::prelude::*;

/// The client plugin. Adds nothing yet.
pub struct ClientPlugin;

impl Plugin for ClientPlugin {
    fn build(&self, _app: &mut App) {}
}
