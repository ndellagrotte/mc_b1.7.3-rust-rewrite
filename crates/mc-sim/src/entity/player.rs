//! `EntityPlayer` / `EntityPlayerSP` construction (EntityPlayer.java:40-57,
//! EntityPlayerSP.java:12-21): the pure part of the constructor chain, so a
//! world can hold a standing player (WP-1.5). Everything the player *does*
//! is Phase 2: its `onUpdate` chain records `EntityPlayer.onUpdate` and
//! `EntityLiving.onUpdate` in [`World::unimplemented_hits`] (see
//! [`super::dispatch::on_update`]), and the world-side effects of a player
//! (`playerEntities`, random ticks around it, mob spawning,
//! `joinEntityInSurroundings`, `randomDisplayUpdates`) come from the world.
//!
//! Also here, because block behaviour and the `use_item` op reach them
//! (WP-1.6 prep): `canHarvestBlock`, `getCurrentPlayerStrVsBlock`
//! (`Block.blockStrength`) and `swingItem` (`Minecraft.clickMouse`).
//!
//! Not transcribed here (WP-2.5): `inventorySlots` / `craftingInventory`
//! (`new ContainerPlayer(inventory, !multiplayerWorld)`,
//! EntityPlayer.java:42-43); EntityPlayerSP's `mc`, `movementInput`,
//! `username` from the session and the stat writer (EntityPlayerSP.java:12-21,
//! client); the remaining `EntityPlayer` fields (score, camera yaw,
//! bed and spawn coordinates, portal timers, fishing hook). The swing fields
//! (`isSwinging`, `swingProgressInt`, EntityPlayer.java:14-15) exist and
//! [`swing_item`] sets them; their per-tick advance in
//! `updatePlayerActionState` (EntityPlayer.java:158-170) is WP-2.5.
//!
//! [`World::unimplemented_hits`]: crate::world::World::unimplemented_hits

use super::data_watcher::WatchValue;
use super::{Entity, EntityKindData, LivingData, PlayerData, PlayerSpData, base};
use crate::block::{BlockDef, Material};
use crate::world::World;

/// The `EntityPlayer` level of `e`; panics where Java's cast to
/// `EntityPlayer` would throw (a caller passed a non-player).
fn player(e: &Entity) -> &PlayerData {
    e.kind
        .player()
        .expect("EntityPlayer method on a non-player (Java ClassCastException)")
}

/// `canHarvestBlock(block)`: `inventory.canHarvestBlock(block)`.
// src: EntityPlayer.java:304-306
pub fn can_harvest_block(e: &Entity, block: &BlockDef) -> bool {
    player(e).inventory.can_harvest_block(block)
}

/// `getCurrentPlayerStrVsBlock(block)`: the held item's speed on `block`,
/// divided by 5 with the eyes in water and again by 5 off the ground.
// src: EntityPlayer.java:291-302
pub fn get_current_player_str_vs_block(e: &Entity, world: &mut World, block: &BlockDef) -> f32 {
    let mut var2 = player(e).inventory.get_str_vs_block(block);
    if base::is_inside_of_material(e, world, Material::Water) {
        var2 /= 5.0;
    }
    if !e.base.on_ground {
        var2 /= 5.0;
    }
    var2
}

/// `swingItem()`: starts the arm swing (EntityPlayerSP does not override it).
// src: EntityPlayer.java:483-486
pub fn swing_item(e: &mut Entity) {
    let p = e
        .kind
        .player_mut()
        .expect("EntityPlayer method on a non-player (Java ClassCastException)");
    p.swing_progress_int = -1;
    p.is_swinging = true;
}

/// `new EntityPlayerSP(world)` without the client half (module docs): the
/// constructor chain `Entity(World)` → `EntityPlayer.entityInit` →
/// `EntityLiving` field initialisers and body (four `Math.random()` draws,
/// [`LivingData::new`]) → `EntityPlayer` field initialisers and body.
///
/// The player starts at the world's spawn point + (0.5, 1, 0.5) with yaw and
/// pitch 0 (`setLocationAndAngles`), so the random yaw `EntityLiving` drew is
/// overwritten. `posY` is that y plus `yOffset` 1.62.
// src: EntityPlayer.java:6-57, Entity.java:75-113, EntityLiving.java:6-68
pub fn new_entity_player_sp(world: &mut World) -> Entity {
    let mut b = base::EntityBase::new(world);
    // entityInit() (virtual, at the end of Entity's constructor):
    // EntityLiving.entityInit is empty (EntityLiving.java:70-71), then
    // EntityPlayer's byte 16 (EntityPlayer.java:54-57).
    b.data_watcher.add_object(16, WatchValue::Byte(0));
    let mut l = LivingData::new(world, &mut b);
    // EntityPlayer field initialisers (EntityPlayer.java:7-38): the inventory
    // (InventoryPlayer.java:4-13) and the sleep state used by World.
    let mut p = PlayerData::default();
    // Constructor body (EntityPlayer.java:41-51).
    // inventorySlots / craftingInventory: ContainerPlayer, WP-2.5.
    b.y_offset = 1.62;
    let var2 = world.get_spawn_point();
    b.set_location_and_angles(
        var2.pos_x as f64 + 0.5,
        (var2.pos_y + 1) as f64,
        var2.pos_z as f64 + 0.5,
        0.0,
        0.0,
    );
    l.health = 20;
    l.entity_type = Some("humanoid");
    l.unused180 = 180.0;
    b.fire_resistance = 20;
    l.texture = "/mob/char.png";
    p.username = None;
    Entity {
        base: b,
        kind: EntityKindData::PlayerSp(l, p, PlayerSpData::default()),
    }
}
