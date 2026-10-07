//! `BlockWorkbench` (BlockWorkbench.java): the crafting table (id 58).
//!
//! Transcribed: `blockActivated`. `getBlockTextureFromSide` is data
//! ([`crate::block`]).

use crate::entity::{Entity, player};
use crate::world::World;
use crate::world::client_requests::PlayerGui;

/// `blockActivated(world, x, y, z, player)`: opens the crafting GUI; always
/// true. `player.displayWorkbenchGUI(x, y, z)` is client-only: empty in
/// `EntityPlayer` (EntityPlayer.java:343-344), a `GuiCrafting` screen in
/// `EntityPlayerSP` (EntityPlayerSP.java:121-123): [`player::display_gui`],
/// [`PlayerGui::Workbench`].
// src: BlockWorkbench.java:19-26
pub fn block_activated(world: &mut World, x: i32, y: i32, z: i32, player: &Entity) -> bool {
    if world.multiplayer_world {
        return true;
    }
    // src: BlockWorkbench.java:23
    player::display_gui(player, world, PlayerGui::Workbench { x, y, z });
    true
}
