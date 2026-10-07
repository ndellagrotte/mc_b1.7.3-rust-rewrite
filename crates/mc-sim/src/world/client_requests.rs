//! The calls the local player (`EntityPlayerSP`) makes into `Minecraft`, as
//! a queue the client drains (WP-3.1b).
//!
//! Vanilla's `EntityPlayerSP` holds the `Minecraft` object and calls it from
//! inside the simulation: it opens screens (`displayGUI*`), writes the chat
//! (`addChatMessage`), adds a particle (`onItemPickup`), plays the portal
//! sounds and travels (`usePortal`), and shows the inventory hint
//! (`GuiAchievement.queueAchievementInformation`). The simulation never
//! calls the client (PLAN_v4 D1, §3.2), so each such call pushes one
//! [`ClientRequest`] onto [`World::client_requests`](super::World::client_requests)
//! instead, in call order, and returns.
//!
//! These are not `IWorldAccess` calls, so they are not [`super::events::WorldEvent`]s:
//! the world's listeners never see them, and only the local player raises
//! them.
//!
//! Every request is raised only by `EntityPlayerSP`'s own bodies, which run
//! with [`crate::entity::player::PlayerSpData::client_additions`] set; the golden
//! harness's player (`HarnessPlayer`, an `EntityPlayer`, whose versions of
//! these methods are empty) raises none. Nothing in the simulation reads
//! the queue back.
//!
//! | Java call | Request |
//! |---|---|
//! | `EntityPlayerSP.displayGUIChest` (EntityPlayerSP.java:117-119) from `BlockChest.blockActivated` (BlockChest.java:217) / `EntityMinecart.interact` (EntityMinecart.java:774) | [`PlayerGui::Chest`] / [`PlayerGui::MinecartChest`] |
//! | `displayWorkbenchGUI` (:121-123) from `BlockWorkbench.blockActivated` (BlockWorkbench.java:23) | [`PlayerGui::Workbench`] |
//! | `displayGUIFurnace` (:125-127) from `BlockFurnace.blockActivated` (BlockFurnace.java:107) | [`PlayerGui::Furnace`] |
//! | `displayGUIDispenser` (:129-131) from `BlockDispenser.blockActivated` (BlockDispenser.java:79) | [`PlayerGui::Dispenser`] |
//! | `displayGUIEditSign` (:113-115) from `ItemSign.onItemUse` (ItemSign.java:47) | [`PlayerGui::EditSign`] |
//! | `addChatMessage` (:172-174) from `BlockBed.blockActivated` (BlockBed.java:63, :76) | [`ClientRequest::ChatMessage`] |
//! | `onItemPickup` (:133-135): `new EntityPickupFX(theWorld, entity, this, -0.5F)` | [`ClientRequest::PickupFx`] |
//! | `mc.sndManager.playSoundFX("portal.trigger" / "portal.travel", 1.0F, pitch)` (:49-51, :58) | [`ClientRequest::PlaySoundFx`] |
//! | `mc.usePortal()` (:59), after its head ([`crate::entity::player::use_portal_head`]) | [`ClientRequest::UsePortal`] |
//! | `closeScreen` (:108-111) from `EntityPlayer.onUpdate` (EntityPlayer.java:81-84) | [`ClientRequest::CloseScreen`] |
//! | `mc.guiAchievement.queueAchievementInformation(AchievementList.openInventory)` (:35-37) | [`ClientRequest::AchievementHint`] |

use crate::entity::Entity;
use crate::inventory::InventoryRef;
use crate::world::tile_entity::TileEntityRef;

/// The screen an `EntityPlayerSP.displayGUI*` call opens
/// (`mc.displayGuiScreen(new Gui…(…))`). The client builds the screen and
/// its container.
#[derive(Clone, Debug, PartialEq)]
pub enum PlayerGui {
    /// `displayGUIChest(inventory)` from a chest block: `new
    /// GuiChest(player.inventory, inventory)` (EntityPlayerSP.java:117-119).
    /// `inventory` is the `IInventory` `BlockChest.blockActivated` built: the
    /// clicked chest's tile entity, wrapped in an `InventoryLargeChest` with
    /// each neighbouring chest (BlockChest.java:186, :198-212). `None` where
    /// a looked-up tile entity was null in Java (a chest block without its
    /// tile entity): vanilla would pass the null on and the GUI would throw.
    Chest {
        /// The clicked chest.
        x: i32,
        /// The clicked chest.
        y: i32,
        /// The clicked chest.
        z: i32,
        /// The inventory to show.
        inventory: Option<InventoryRef>,
    },
    /// `displayGUIChest(this)` from a chest minecart's `interact`
    /// (EntityMinecart.java:774): the cart is the `IInventory`.
    MinecartChest {
        /// The cart's `entityId`.
        entity_id: i32,
    },
    /// `displayWorkbenchGUI(x, y, z)`: `new GuiCrafting(player.inventory,
    /// worldObj, x, y, z)` (EntityPlayerSP.java:121-123).
    Workbench {
        /// The workbench.
        x: i32,
        /// The workbench.
        y: i32,
        /// The workbench.
        z: i32,
    },
    /// `displayGUIFurnace(tileEntity)`: `new GuiFurnace(player.inventory,
    /// tileEntity)` (EntityPlayerSP.java:125-127). `tile_entity` is
    /// `(TileEntityFurnace) world.getBlockTileEntity(x, y, z)`
    /// (BlockFurnace.java:106); `None` where it was null.
    Furnace {
        /// The furnace.
        x: i32,
        /// The furnace.
        y: i32,
        /// The furnace.
        z: i32,
        /// Its tile entity.
        tile_entity: Option<TileEntityRef>,
    },
    /// `displayGUIDispenser(tileEntity)`: `new GuiDispenser(player.inventory,
    /// tileEntity)` (EntityPlayerSP.java:129-131), the tile entity as
    /// `BlockDispenser.blockActivated` looked it up (BlockDispenser.java:78).
    Dispenser {
        /// The dispenser.
        x: i32,
        /// The dispenser.
        y: i32,
        /// The dispenser.
        z: i32,
        /// Its tile entity.
        tile_entity: Option<TileEntityRef>,
    },
    /// `displayGUIEditSign(sign)`: `new GuiEditSign(sign)`
    /// (EntityPlayerSP.java:113-115), after `ItemSign` placed the sign.
    EditSign {
        /// The sign.
        x: i32,
        /// The sign.
        y: i32,
        /// The sign.
        z: i32,
        /// Its `TileEntitySign`.
        tile_entity: TileEntityRef,
    },
}

/// `new EntityPickupFX(mc.theWorld, entity, this, -0.5F)`
/// (EntityPlayerSP.java:133-135): the picked-up entity flying to the
/// player for 3 ticks (EntityPickupFX.java:12-18).
///
/// The particle keeps the picked entity's object, which the world drops at
/// once (`setEntityDead` follows `onItemPickup`, EntityItem.java:123-126,
/// EntityArrow.java:261-262): `picked` is that object as it was at the call
/// (an `EntityItem`'s stack already lowered by the pickup), which is all
/// the particle reads afterwards (its position, rotation and what it
/// renders, EntityPickupFX.java:20-41). The player is read live by its id
/// (`lastTickPos*`/`pos*` every frame).
#[derive(Clone, Debug)]
pub struct PickupFx {
    /// `var2`: the entity picked up.
    pub picked: Box<Entity>,
    /// `var3`: the player's `entityId`.
    pub player_id: i32,
    /// `var4`: the target's y offset, `-0.5F`.
    pub y_offs: f32,
}

impl PartialEq for PickupFx {
    /// [`Entity`] has no `PartialEq`: two pickups are equal when they are of
    /// the same entity (by id) to the same player with the same offset.
    fn eq(&self, other: &Self) -> bool {
        self.picked.base.entity_id == other.picked.base.entity_id
            && self.player_id == other.player_id
            && self.y_offs == other.y_offs
    }
}

/// One call from the local player into `Minecraft` (module docs).
#[derive(Clone, Debug, PartialEq)]
pub enum ClientRequest {
    /// `mc.displayGuiScreen(new Gui…)` from a `displayGUI*` method.
    DisplayGui(PlayerGui),
    /// `addChatMessage(key)`: `mc.ingameGUI.addChatMessageTranslate(key)`
    /// (EntityPlayerSP.java:172-174), a translation key.
    ChatMessage {
        /// The `StringTranslate` key (`"tile.bed.noSleep"`, …).
        key: &'static str,
    },
    /// `onItemPickup`'s particle.
    PickupFx(PickupFx),
    /// `mc.sndManager.playSoundFX(name, volume, pitch)`: a sound at the
    /// listener, not in the world (the portal sounds,
    /// EntityPlayerSP.java:50, :58). The pitch's `rand` draw happened in the
    /// simulation.
    PlaySoundFx {
        /// Sound name.
        name: &'static str,
        /// Volume.
        volume: f32,
        /// Pitch.
        pitch: f32,
    },
    /// `mc.usePortal()` (EntityPlayerSP.java:59). Its head already ran in
    /// the player's update ([`crate::entity::player::use_portal_head`]: the
    /// dimension toggled, the player out of the old world's
    /// `playerEntities`, x and z scaled), so the rest of the old world's
    /// `updateEntities` sees the player gone, as vanilla's does. The
    /// player's update stopped there (`EntityPlayerSP.onLivingUpdate`
    /// returned early): the client runs the rest of `usePortal` (the old
    /// world's `updateEntityWithOptionalForce(player, false)`, the new
    /// world, `changeWorld`, the teleporter), then the rest of the update on
    /// the new world with
    /// [`crate::entity::player::sp_on_living_update_after_portal`].
    UsePortal {
        /// `var1`: the scaled x (Minecraft.java:1228-1229, :1239-1240).
        x: f64,
        /// `var3`: the scaled z.
        z: f64,
    },
    /// `closeScreen()`: `mc.displayGuiScreen(null)` (EntityPlayerSP.java:108-111),
    /// raised by [`crate::entity::player::close_screen`].
    CloseScreen,
    /// `mc.guiAchievement.queueAchievementInformation(achievement)`: the
    /// "open your inventory" hint, raised every update while the
    /// achievement is not unlocked (EntityPlayerSP.java:35-37).
    AchievementHint {
        /// The achievement's `statId` ([`crate::stats::achievement_list`]).
        stat_id: i32,
    },
}

/// The pending [`ClientRequest`]s, in call order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClientRequestQueue(pub Vec<ClientRequest>);

impl ClientRequestQueue {
    /// Appends one request.
    pub fn push(&mut self, r: ClientRequest) {
        self.0.push(r);
    }

    /// Removes and returns every pending request, oldest first.
    pub fn drain(&mut self) -> Vec<ClientRequest> {
        std::mem::take(&mut self.0)
    }

    /// Number of pending requests.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether no request is pending.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The pending requests, oldest first.
    pub fn iter(&self) -> std::slice::Iter<'_, ClientRequest> {
        self.0.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drain_returns_in_order_and_empties() {
        let mut q = ClientRequestQueue::default();
        q.push(ClientRequest::PlaySoundFx {
            name: "portal.travel",
            volume: 1.0,
            pitch: 0.9,
        });
        q.push(ClientRequest::UsePortal { x: 1.0, z: -2.0 });
        assert_eq!(q.len(), 2);
        let v = q.drain();
        assert_eq!(v[1], ClientRequest::UsePortal { x: 1.0, z: -2.0 });
        assert!(matches!(
            v[0],
            ClientRequest::PlaySoundFx {
                name: "portal.travel",
                ..
            }
        ));
        assert!(q.is_empty());
    }
}
