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
//!   block-hit particles, the chat line, screens, settings side effects),
//!   and the local player's calls into `Minecraft` that the simulation
//!   raised as [`ClientRequest`]s (the screens it opens, its chat lines, the
//!   pickup particle, the inventory hint). The player's `usePortal` is the
//!   shell's own ([`crate::game`]); its sounds become [`ClientSound`]s.
//!
//! [`ClientRequest`]: mc_sim::world::client_requests::ClientRequest

use bevy::prelude::*;
use mc_sim::world::client_requests::{PickupFx, PlayerGui};
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
    /// `sndManager.playSoundFX(name, volume, pitch)` (no position): the
    /// portal sounds `EntityPlayerSP.onLivingUpdate` plays
    /// (EntityPlayerSP.java:50, :58; the pitch drawn by the simulation).
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
    /// `sndManager.playRandomMusicIfReady()`, which
    /// `PlayerControllerSP.updateController()` calls every unpaused tick
    /// (PlayerControllerSP.java:117-120): the music timer.
    PlayRandomMusicIfReady,
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
    /// EntityPlayerSP.java:172-174: "tile.bed.notValid" on respawn,
    /// "tile.bed.occupied" / "tile.bed.noSleep" from a bed, BlockBed.java:63,
    /// :76).
    ChatMessageTranslate(String),
    /// `EntityPlayerSP.displayGUIChest` / `displayWorkbenchGUI` /
    /// `displayGUIFurnace` / `displayGUIDispenser` / `displayGUIEditSign`
    /// (EntityPlayerSP.java:113-131): `mc.displayGuiScreen(new GuiChest(…))`
    /// etc. The screen's owner (WP-3.6) opens it with
    /// [`crate::game::Game::display_gui_screen`]; vanilla does it at once,
    /// inside the click or the update that asked.
    DisplayGui(PlayerGui),
    /// `EntityPlayerSP.onItemPickup`: `effectRenderer.addEffect(new
    /// EntityPickupFX(theWorld, entity, player, -0.5F))`
    /// (EntityPlayerSP.java:133-135; WP-3.7).
    PickupFx(PickupFx),
    /// `guiAchievement.queueAchievementInformation(achievement)`: the
    /// "open your inventory" hint, every player update until the
    /// achievement is unlocked (EntityPlayerSP.java:35-37; WP-3.6).
    AchievementHint {
        /// The achievement's `statId`.
        stat_id: i32,
    },
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
