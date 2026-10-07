//! What the shell tells the other client parts, as Bevy messages.
//!
//! - [`WorldEventMsg`]: every [`WorldEvent`] the simulation queued
//!   (`IWorldAccess`, PLAN_v4 §3.2), in call order. Drained at each
//!   [`crate::game::RunTickHook`] point and at the end of the frame, so a
//!   hook system (e.g. the effect renderer's `updateEffects`) sees the
//!   particles of the same tick, as `RenderGlobal.spawnParticle` adds them
//!   synchronously in vanilla.
//! - [`ClientSound`]: the sound calls `Minecraft` and its helpers make
//!   directly on `sndManager` (WP-3.8).
//! - [`ClientEvent`]: everything else the shell does that another part must
//!   react to (world changes for the renderers, the effect renderer's
//!   block-hit particles, the chat line, screens, settings side effects).

use bevy::prelude::*;
use mc_sim::world::events::WorldEvent;

use crate::app::GuiScreen;
use crate::settings::SettingsEffect;

/// One simulation event (module docs).
#[derive(Message, Clone, Debug, PartialEq)]
pub struct WorldEventMsg(pub WorldEvent);

/// A `SoundManager` call made by the shell (WP-3.8 plays them).
#[derive(Message, Clone, Debug, PartialEq)]
pub enum ClientSound {
    /// `sndManager.playSound(name, x, y, z, volume, pitch)`: the player
    /// controller's dig-step sounds (`PlayerControllerSp::sounds`).
    PlaySound {
        /// Sound name.
        name: String,
        /// Position.
        x: f32,
        /// Position.
        y: f32,
        /// Position.
        z: f32,
        /// Volume.
        volume: f32,
        /// Pitch.
        pitch: f32,
    },
    /// `sndManager.playSoundFX(name, volume, pitch)` (no position).
    PlaySoundFx {
        /// Sound name.
        name: String,
        /// Volume.
        volume: f32,
        /// Pitch.
        pitch: f32,
    },
    /// `sndManager.playStreaming(null, 0, 0, 0, 0, 0)`: stop the record or
    /// music when the world changes (Minecraft.java:1274).
    StopStreaming,
    /// `sndManager.func_338_a(player, partialTicks)`: the per-frame
    /// listener update (Minecraft.java:550) with the interpolated player
    /// position and yaw; `None` without a player.
    Listener {
        /// `prevPosX + (posX - prevPosX) * partial`, … and the yaw likewise.
        pose: Option<([f64; 3], f32)>,
    },
    /// `forceReload()` (F3+S, Minecraft.java:1180-1185): a new sound
    /// manager with the settings, and the resources installed again.
    ForceReload,
    /// `sndManager.closeMinecraft()` (shutdown, Minecraft.java:484).
    Close,
}

/// A shell event other parts react to (module docs).
#[derive(Message, Clone, Debug, PartialEq)]
pub enum ClientEvent {
    /// `changeWorld` installed a new world (or none): `renderGlobal.changeWorld`,
    /// `effectRenderer.clearEffects` (Minecraft.java:1304-1310).
    WorldChanged {
        /// The new world's dimension (`worldProvider.worldType`), `None`
        /// when leaving the world.
        dimension: Option<i32>,
    },
    /// `effectRenderer.addBlockHitEffects(x, y, z, side)` while digging
    /// (Minecraft.java:791).
    BlockHitEffects {
        /// Block.
        x: i32,
        /// Block.
        y: i32,
        /// Block.
        z: i32,
        /// `sideHit`.
        side: i32,
    },
    /// `ingameGUI.addChatMessage(text)` (the screenshot line,
    /// Minecraft.java:663).
    ChatMessage(String),
    /// `ingameGUI.addChatMessageTranslate(key)` (`EntityPlayerSP.addChatMessage`,
    /// EntityPlayerSP.java:172-174: "tile.bed.notValid" on respawn).
    ChatMessageTranslate(String),
    /// `ingameGUI.clearChatMessages()` (opening the main menu,
    /// Minecraft.java:427-429).
    ClearChat,
    /// `currentScreen.onGuiClosed()` on the screen being replaced.
    ScreenClosed(GuiScreen),
    /// A screen opened, after `setWorldAndResolution(mc, w, h)` (its size
    /// is set).
    ScreenOpened(GuiScreen),
    /// The window size changed while a screen is open (`resize`,
    /// Minecraft.java:900-918): the screen's `setWorldAndResolution` again.
    ScreenResized(GuiScreen),
    /// A settings change asks another subsystem to reload.
    Settings(SettingsEffect),
    /// The equipped item changed by a use (`itemRenderer.resetEquippedProgress`
    /// / `resetEquippedProgress2`, Minecraft.java:841, :849; WP-3.7).
    ResetEquippedProgress {
        /// `true` for `resetEquippedProgress2` (an item use).
        second: bool,
    },
}
