//! `PlayerController` and `PlayerControllerSP` (PlayerController.java,
//! PlayerControllerSP.java; WP-2.5): what the client's mouse buttons do to
//! the world in singleplayer survival.
//!
//! The Java controllers reach the world and the player through
//! `mc.theWorld` / `mc.thePlayer`; here both are parameters (the player
//! taken out of its slot, as every entity method has it). The client owns
//! the controller and calls it from its input handling
//! (`Minecraft.clickMouse` / `func_6254_a`, Minecraft.java:779-859); the
//! golden harness's `dig` op does the same (tests/golden/README.md).
//!
//! # Audit
//!
//! | Java | Method | Port |
//! |---|---|---|
//! | PlayerController.java:5-11 | fields, constructor | [`PlayerControllerSp`] (`field_1064_b`: [`PlayerControllerSp::is_in_test_mode`]) |
//! | :13-14, :40-41, :62-73 | `func_717_a`, `setPartialTime`, `flipPlayer`, `updateController`, `shouldDrawHUD`, `func_6473_b` | overridden or client: see PlayerControllerSP |
//! | :16-19 | `clickBlock` | overridden ([`PlayerControllerSp::click_block`]) |
//! | :21-32 | `sendBlockRemoved` | [`player_controller_send_block_removed`], called by the override |
//! | :34-38 | `sendBlockRemoving`, `resetBlockRemoving` (empty) | overridden |
//! | :43-45 | `getBlockReachDistance` (5) | overridden (4) |
//! | :47-61, :75-82 | `sendUseItem`, `sendPlaceBlock` | [`send_use_item`], [`send_place_block`] (their `Item` calls are WP-2.6: [`player::item_stack_use_item_right_click`], [`player::item_stack_use_item`]) |
//! | :84-86 | `createPlayer` | [`crate::entity::player::new_entity_player_sp`] (client: the session) |
//! | :88-94 | `interactWithEntity`, `attackEntity` | [`interact_with_entity`], [`attack_entity`] |
//! | :96-103 | `handleMouseClick`, `onCraftGuiClosed` | containers, WP-2.5b |
//! | PlayerControllerSP.java:110-116 | fields | [`PlayerControllerSp`] |
//! | :122-124 | `flipPlayer` | [`flip_player`] |
//! | :126-145 | `sendBlockRemoved` | [`PlayerControllerSp::send_block_removed`] |
//! | :147-158 | `clickBlock` | [`PlayerControllerSp::click_block`] |
//! | :160-199 | `resetBlockRemoving`, `sendBlockRemoving` | [`PlayerControllerSp::reset_block_removing`], [`PlayerControllerSp::send_block_removing`] |
//! | :201-211 | `setPartialTime` | [`PlayerControllerSp::damage_partial_time`] (render) |
//! | :213-224 | `getBlockReachDistance`, `func_717_a`, `updateController` | [`PlayerControllerSp::get_block_reach_distance`]; nothing; [`PlayerControllerSp::update_controller`] |
//!
//! Sounds the controller plays straight through `mc.sndManager` (the dig
//! step sound) are queued in [`PlayerControllerSp::sounds`] for the client;
//! they draw nothing. Item behaviour on a broken block (`onDestroyBlock`) is
//! [`player::item_stack_on_destroy_block`] (module docs of
//! [`crate::entity::player`]).

use crate::block::{self, dispatch as block_dispatch};
use crate::entity::{Entity, player};
use crate::inventory::InventoryPlayer;
use crate::world::World;

/// A sound the controller asks the client to play
/// (`mc.sndManager.playSound(name, x, y, z, volume, pitch)`).
#[derive(Clone, Debug, PartialEq)]
pub struct ControllerSound {
    /// Sound name.
    pub name: String,
    /// Position.
    pub x: f32,
    /// Position.
    pub y: f32,
    /// Position.
    pub z: f32,
    /// Volume.
    pub volume: f32,
    /// Pitch.
    pub pitch: f32,
}

/// `PlayerControllerSP`'s state (PlayerController.java:7,
/// PlayerControllerSP.java:110-116).
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerControllerSp {
    /// `field_1064_b` (PlayerController.java:7): true only for the
    /// creative test controller (`PlayerControllerTest`, client), where it
    /// disables block breaking in `Minecraft.func_6254_a`.
    pub is_in_test_mode: bool,
    /// `field_1074_c`: x of the block being broken (-1: none yet).
    pub current_block_x: i32,
    /// `field_1073_d`: y of the block being broken.
    pub current_block_y: i32,
    /// `field_1072_e`: z of the block being broken.
    pub current_block_z: i32,
    /// `curBlockDamage`: breaking progress, the block goes at 1.
    pub cur_block_damage: f32,
    /// `prevBlockDamage`: last tick's progress (render interpolation).
    pub prev_block_damage: f32,
    /// `field_1069_h`: ticks spent on the current block (the step sound
    /// plays every 4th).
    pub block_damage_ticks: f32,
    /// `blockHitWait`: ticks before the next block can be damaged (5 after a
    /// break).
    pub block_hit_wait: i32,
    /// Sounds for the client (module docs), oldest first.
    pub sounds: Vec<ControllerSound>,
}

impl Default for PlayerControllerSp {
    // src: PlayerController.java:7, PlayerControllerSP.java:110-116
    fn default() -> Self {
        PlayerControllerSp {
            is_in_test_mode: false,
            current_block_x: -1,
            current_block_y: -1,
            current_block_z: -1,
            cur_block_damage: 0.0,
            prev_block_damage: 0.0,
            block_damage_ticks: 0.0,
            block_hit_wait: 0,
            sounds: Vec::new(),
        }
    }
}

/// `PlayerControllerSP.flipPlayer(player)`: faces the player north (the
/// client's world start).
// src: PlayerControllerSP.java:122-124
pub fn flip_player(p: &mut Entity) {
    p.base.rotation_yaw = -180.0;
}

/// `PlayerController.sendUseItem(player, world, stack)` (not overridden):
/// `useItemRightClick`; a changed stack (another object, or another size)
/// replaces hotbar slot `currentItem` (cleared at size 0) and counts as
/// used. Java passes the `ItemStack` object; stacks are values here, so
/// `stack` is the `mainInventory` slot holding Java's `var3` (the caller's
/// `getCurrentItem()`), and the stack is moved out of it while the item
/// runs so that whatever `onItemRightClick` does to it lands in the slot.
/// Java's identity test `var5 != var3` is [`player::RightClickResult`].
// src: PlayerController.java:47-61
pub fn send_use_item(world: &mut World, p: &mut Entity, stack: usize) -> bool {
    let mut var3 = player_inventory(p).main_inventory[stack]
        .take()
        .expect("sendUseItem's stack is the slot's stack");
    let var4 = var3.stack_size;
    let r = player::item_stack_use_item_right_click(&mut var3, world, p);
    let inv = player_inventory(p);
    // var3 goes back into its slot first: Java's var3 never left it.
    inv.main_inventory[stack] = Some(var3);
    let var5 = match r {
        player::RightClickResult::Same if var3.stack_size == var4 => return false,
        player::RightClickResult::Same => var3,
        player::RightClickResult::Replaced(s) => s,
    };
    let cur = inv.current_item as usize;
    inv.main_inventory[cur] = Some(var5);
    if var5.stack_size == 0 {
        inv.main_inventory[cur] = None;
    }
    true
}

/// `PlayerController.sendPlaceBlock(player, world, stack, x, y, z, side)`
/// (not overridden): the block's `blockActivated`, else
/// `stack.useItem(...)`. `stack` is the `mainInventory` slot holding Java's
/// `var3` (`None` for `null`; see [`send_use_item`]); no b1.7.3
/// `blockActivated` or `onItemUse` stores another stack into that slot, so
/// the slot keeps standing for the object.
// src: PlayerController.java:75-82
pub fn send_place_block(
    world: &mut World,
    p: &mut Entity,
    stack: Option<usize>,
    x: i32,
    y: i32,
    z: i32,
    side: i32,
) -> bool {
    let var8 = world.get_block_id(x, y, z);
    if var8 > 0 && block_dispatch::block_activated(world, var8, x, y, z, p) {
        true
    } else {
        match stack {
            None => false,
            Some(slot) => {
                // The stack is moved out of its slot while the item runs, so
                // that it and the player can both be borrowed mutably, and
                // put back afterwards: whatever onItemUse does to it lands
                // in the slot, as on Java's shared object.
                let mut s = player_inventory(p).main_inventory[slot]
                    .take()
                    .expect("sendPlaceBlock's stack is the slot's stack");
                let r = player::item_stack_use_item(&mut s, world, p, x, y, z, side);
                player_inventory(p).main_inventory[slot] = Some(s);
                r
            }
        }
    }
}

/// `PlayerController.interactWithEntity(player, target)` (not overridden):
/// `player.useCurrentItemOnEntity(target)`.
// src: PlayerController.java:88-90
pub fn interact_with_entity(world: &mut World, p: &mut Entity, target: &mut Entity) {
    player::use_current_item_on_entity(p, world, target);
}

/// The player's `inventory`.
fn player_inventory(p: &mut Entity) -> &mut InventoryPlayer {
    &mut p
        .kind
        .player_mut()
        .expect("the controller's player is an EntityPlayer")
        .inventory
}

/// `PlayerController.attackEntity(player, target)` (not overridden):
/// `player.attackTargetEntityWithCurrentItem(target)`.
// src: PlayerController.java:92-94
pub fn attack_entity(world: &mut World, p: &mut Entity, target: &mut Entity) {
    player::attack_target_entity_with_current_item(p, world, target);
}

/// `PlayerController.sendBlockRemoved(x, y, z, side)`: the break effect
/// (aux sound 2001 with block id + meta × 256), the block set to air, and
/// on success the block's `onBlockDestroyedByPlayer` with the old meta.
/// Only called on a non-air block (Java dereferences `Block.blocksList[id]`
/// before its null check).
// src: PlayerController.java:21-32
pub fn player_controller_send_block_removed(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    let id = world.get_block_id(x, y, z);
    let meta = world.get_block_metadata(x, y, z);
    world.play_aux_sfx(2001, x, y, z, id + meta * 256);
    let var7 = world.get_block_metadata(x, y, z);
    let var8 = world.set_block_with_notify(x, y, z, 0);
    if block::block(id).is_some() && var8 {
        block_dispatch::on_block_destroyed_by_player(world, id, x, y, z, var7);
    }
    var8
}

impl PlayerControllerSp {
    /// `new PlayerControllerSP(mc)`.
    pub fn new() -> Self {
        Self::default()
    }

    /// `PlayerControllerSP.sendBlockRemoved(x, y, z, side)`: `PlayerController`'s
    /// (the block goes), then the held stack's `onDestroyBlock` (tool wear; a
    /// broken stack leaves the hand) and, when the block went and the player
    /// could harvest it (asked before the wear), the block's `harvestBlock`
    /// (the drops).
    // src: PlayerControllerSP.java:126-145
    pub fn send_block_removed(
        &mut self,
        world: &mut World,
        p: &mut Entity,
        x: i32,
        y: i32,
        z: i32,
        side: i32,
    ) -> bool {
        let _ = side;
        let var5 = world.get_block_id(x, y, z);
        let var6 = world.get_block_metadata(x, y, z);
        let var7 = player_controller_send_block_removed(world, x, y, z);
        let var8 = held_slot(p);
        let var9 = match block::block(var5) {
            Some(b) => player::can_harvest_block(p, b),
            // canHarvestBlock(null) would throw: never reached (var5 > 0).
            None => false,
        };
        if let Some(slot) = var8 {
            player::item_stack_on_destroy_block(p, world, slot, var5);
            if p.kind
                .player()
                .expect("the controller's player")
                .inventory
                .main_inventory[slot]
                .is_some_and(|s| s.stack_size == 0)
            {
                // var8.onItemDestroyedByUse(player): empty (ItemStack.java:167-168)
                player::destroy_current_equipped_item(p);
            }
        }
        if var7 && var9 {
            block_dispatch::harvest_block(world, var5, x, y, z, p, var6);
        }
        var7
    }

    /// `PlayerControllerSP.clickBlock(x, y, z, side)`: the first click on a
    /// block (and the client's repeat click every 5 ticks while held): puts
    /// out fire in front of the face (`World.onBlockHit`), the block's
    /// `onBlockClicked` when no damage is in progress, and an instant break
    /// for a block the player breaks in one tick.
    // src: PlayerControllerSP.java:147-158
    pub fn click_block(
        &mut self,
        world: &mut World,
        p: &mut Entity,
        x: i32,
        y: i32,
        z: i32,
        side: i32,
    ) -> bool {
        world.on_block_hit(Some(p.base.entity_id), x, y, z, side);
        let var5 = world.get_block_id(x, y, z);
        if var5 > 0 && self.cur_block_damage == 0.0 {
            block_dispatch::on_block_clicked(world, var5, x, y, z, p);
        }
        let mut broken = false;
        if var5 > 0 && block_dispatch::block_strength(world, var5, p) >= 1.0 {
            broken = self.send_block_removed(world, p, x, y, z, side);
        }
        broken
    }

    /// `resetBlockRemoving()`: the button was released.
    // src: PlayerControllerSP.java:160-163
    pub fn reset_block_removing(&mut self) {
        self.cur_block_damage = 0.0;
        self.block_hit_wait = 0;
    }

    /// `sendBlockRemoving(x, y, z, side)`: one tick of holding the button on
    /// a block. After a break it waits 5 ticks. On the same block as last
    /// tick the damage grows by `blockStrength(player)` (a step sound every
    /// 4th tick) and the block breaks at 1; a new block restarts at 0 (no
    /// damage that tick). Returns whether this call broke the block (the
    /// return value of `sendBlockRemoved`).
    // src: PlayerControllerSP.java:165-199
    pub fn send_block_removing(
        &mut self,
        world: &mut World,
        p: &mut Entity,
        x: i32,
        y: i32,
        z: i32,
        side: i32,
    ) -> bool {
        let mut broken = false;
        if self.block_hit_wait > 0 {
            self.block_hit_wait -= 1;
        } else if x == self.current_block_x
            && y == self.current_block_y
            && z == self.current_block_z
        {
            let var5 = world.get_block_id(x, y, z);
            if var5 == 0 {
                return false;
            }
            let Some(var6) = block::block(var5) else {
                return false;
            };
            self.cur_block_damage += block_dispatch::block_strength(world, var5, p);
            if self.block_damage_ticks % 4.0 == 0.0 {
                let s = var6.step_sound;
                self.sounds.push(ControllerSound {
                    name: s.step_sound_dir2(),
                    x: x as f32 + 0.5,
                    y: y as f32 + 0.5,
                    z: z as f32 + 0.5,
                    volume: (s.get_volume() + 1.0) / 8.0,
                    pitch: s.get_pitch() * 0.5,
                });
            }
            self.block_damage_ticks += 1.0;
            if self.cur_block_damage >= 1.0 {
                broken = self.send_block_removed(world, p, x, y, z, side);
                self.cur_block_damage = 0.0;
                self.prev_block_damage = 0.0;
                self.block_damage_ticks = 0.0;
                self.block_hit_wait = 5;
            }
        } else {
            self.cur_block_damage = 0.0;
            self.prev_block_damage = 0.0;
            self.block_damage_ticks = 0.0;
            self.current_block_x = x;
            self.current_block_y = y;
            self.current_block_z = z;
        }
        broken
    }

    /// `setPartialTime(partial)`: the crack overlay's interpolated progress
    /// (`ingameGUI.damageGuiPartialTime`, `renderGlobal.damagePartialTime`);
    /// 0 with no damage.
    // src: PlayerControllerSP.java:201-211
    pub fn damage_partial_time(&self, var1: f32) -> f32 {
        if self.cur_block_damage <= 0.0 {
            0.0
        } else {
            self.prev_block_damage + (self.cur_block_damage - self.prev_block_damage) * var1
        }
    }

    /// `getBlockReachDistance()`: 4 in survival (`PlayerController`'s is 5).
    // src: PlayerControllerSP.java:213-215
    pub fn get_block_reach_distance(&self) -> f32 {
        4.0
    }

    /// `updateController()`: once per client tick (Minecraft.java:962-964):
    /// keeps last tick's progress. The random music it also starts is the
    /// client's (WP-3.8).
    // src: PlayerControllerSP.java:221-224
    pub fn update_controller(&mut self) {
        self.prev_block_damage = self.cur_block_damage;
    }
}

/// `player.getCurrentEquippedItem()` as the hotbar slot holding it.
fn held_slot(p: &Entity) -> Option<usize> {
    let inv = &p.kind.player().expect("the controller's player").inventory;
    inv.get_current_item().map(|_| inv.current_item as usize)
}
