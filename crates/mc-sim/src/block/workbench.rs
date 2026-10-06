//! `BlockWorkbench` (BlockWorkbench.java): the crafting table (id 58).
//!
//! Transcribed: `blockActivated`. `getBlockTextureFromSide` is data
//! ([`crate::block`]).

use crate::world::World;

/// `blockActivated(world, x, y, z, player)`: opens the crafting GUI; always
/// true. `player.displayWorkbenchGUI(x, y, z)` is client-only: empty in
/// `EntityPlayer` (EntityPlayer.java:343-344), a `GuiCrafting` screen in
/// `EntityPlayerSP` (EntityPlayerSP.java:121-123). A no-op here; the client
/// opens the screen when the click returns true on a workbench.
// src: BlockWorkbench.java:19-26
pub fn block_activated(world: &World) -> bool {
    if world.multiplayer_world {
        return true;
    }
    // src: BlockWorkbench.java:23: var5.displayWorkbenchGUI(var2, var3, var4) (client)
    true
}
