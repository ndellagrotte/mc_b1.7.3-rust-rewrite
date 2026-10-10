//! `EntityPlayer` (EntityPlayer.java) and the non-client parts of
//! `EntityPlayerSP` (EntityPlayerSP.java), WP-2.5.
//!
//! The port's local player is one kind, [`EntityKindData::PlayerSp`]
//! (`LivingData`, [`PlayerData`] for `EntityPlayer`'s fields,
//! [`PlayerSpData`] for `EntityPlayerSP`'s). Behaviour is one function per
//! Java method below, reached through [`super::dispatch`] for the virtual
//! ones.
//!
//! # `EntityPlayerSP` and the oracle's `HarnessPlayer`
//!
//! The oracle cannot build an `EntityPlayerSP` (it needs a `Minecraft`), so
//! its player is `HarnessPlayer extends EntityPlayer`
//! (tools/oracle/java/harness/HarnessPlayer.java), which adds exactly the
//! `EntityPlayerSP` overrides that need no client: `updatePlayerActionState`
//! applying a [`MovementInput`] (EntityPlayerSP.java:27-32) and
//! `isSneaking` (:144-146). The port has both always. `EntityPlayerSP`'s
//! `onLivingUpdate` body (:34-88: the inventory hint, the portal timer and
//! `Minecraft.usePortal`, the sneak `ySize`, `pushOutOfBlocks` at the four
//! corners) and its other overrides that call into `Minecraft`
//! (`displayGUI*`, `addChatMessage`, `onItemPickup`) run only with
//! [`PlayerSpData::client_additions`] set; the golden driver leaves it off,
//! which is `HarnessPlayer` (tools/oracle/README.md, "`HarnessPlayer`").
//! `EntityPlayerSP.addStat` writes the stat file the player carries
//! ([`add_stat`], [`crate::stats`]); the achievement toast is the
//! client's ([`crate::stats::StatFileWriter::taken_achievements`]).
//!
//! # Calls into `Minecraft` (WP-3.1b)
//!
//! What `EntityPlayerSP` asks of `mc` becomes a [`ClientRequest`] on
//! [`World::client_requests`] ([`crate::world::client_requests`]):
//! [`display_gui`], [`add_chat_message`], [`on_item_pickup`]'s particle, the
//! portal sounds, `usePortal` and the inventory hint
//! ([`sp_on_living_update`]).
//!
//! `mc.usePortal()` (EntityPlayerSP.java:59) changes the world in the middle
//! of the player's update and returns into it: the rest of `onLivingUpdate`
//! (:63 `inPortal = false`, :74-87: the portal cooldown, the sneak `ySize`,
//! `pushOutOfBlocks`, `EntityPlayer.onLivingUpdate` with its movement) runs
//! on the new world. Here [`sp_on_living_update`] runs `usePortal`'s
//! deterministic head ([`use_portal_head`]: the dimension toggle,
//! `setEntityDead` out of `playerEntities`, x and z scaled), so every
//! entity the old world's `updateEntities` reaches after the player sees
//! it gone and moved, as in vanilla; then it raises
//! [`ClientRequest::UsePortal`] and returns. The client runs the rest of
//! `usePortal` and then calls [`sp_on_living_update_after_portal`] on the
//! new world. `usePortal`'s old-world
//! `updateEntityWithOptionalForce(player, false)` (Minecraft.java:1231-1233,
//! :1242-1244: `lastTickPos*`, `prevRotation*`) runs in the client after the
//! old world's `updateEntities` pass instead of inside it; the chunk
//! membership it also fixes is fixed anyway by the pass's own
//! `updateEntityWithOptionalForce(player, true)` tail right after the
//! player's update, so for the rest of the pass only the player's
//! `lastTickPos*`/`prevRotation*` differ, which no other entity reads. What
//! follows `onLivingUpdate` in its callers
//! (`EntityLiving.onUpdate`'s body-yaw tail, EntityLiving.java:206-290;
//! `EntityPlayer.onUpdate`'s tail, the cape position and `minutesPlayed`,
//! EntityPlayer.java:80-124; the old world's `updateEntityWithOptionalForce`
//! bookkeeping) still runs on the old world, before the dimension change
//! instead of after it. Those read the position, which in vanilla is
//! already the new world's; they only feed rendering (body yaw, cape), a
//! stat counter that counts either way, and the old world, which is
//! discarded.
//!
//! `closeScreen`'s `mc.displayGuiScreen(null)` (EntityPlayerSP.java:108-111)
//! is [`close_screen`] ([`ClientRequest::CloseScreen`]); its caller,
//! `EntityPlayer.onUpdate`'s `craftingInventory.isUsableByPlayer` check
//! (EntityPlayer.java:81-84), runs in [`on_update`] on the open GUI's
//! container, which the client's `GuiContainer.initGui` hands to the player
//! ([`PlayerSpData::crafting_inventory`]); the screen's `onGuiClosed`
//! drops that `mc.displayGuiScreen(null)` runs at once happen there too
//! ([`close_screen_dropping`], WP-4.7).
//!
//! Out of scope, client: `setHealth` (multiplayer packets), the
//! `MouseFilter`s.
//!
//! # Audit of EntityPlayer.java
//!
//! | EntityPlayer.java | Method | Port |
//! |---|---|---|
//! | 7-38 | fields | [`PlayerData`] (not `inventorySlots`: no persistent `ContainerPlayer`, WP-2.5b; `craftingInventory` is [`PlayerSpData::crafting_inventory`], WP-4.7; not `playerCloakUrl`, D2) |
//! | 40-57 | constructor, `entityInit` | [`new_entity_player_sp`] |
//! | 59-125 | `onUpdate` | [`on_update`] |
//! | 127-133 | `isMovementBlocked`, `closeScreen` | [`is_movement_blocked`]; [`close_screen`] (with `EntityPlayerSP`'s, EntityPlayerSP.java:108-111) |
//! | 135-138 | `updateCloak` | online cloak URL (D2), not ported |
//! | 140-156 | `updateRidden`, `preparePlayerToSpawn` | [`update_ridden`], [`prepare_player_to_spawn`] |
//! | 158-212 | `updatePlayerActionState`, `onLivingUpdate`, `collideWithPlayer` | [`update_player_action_state`], [`on_living_update`] |
//! | 214-247 | `getScore`, `onDeath`, `addToPlayerScore` | [`PlayerData::get_score`], [`on_death`], [`add_to_player_score`] |
//! | 249-289 | `dropCurrentItem`, `dropPlayerItem`, `dropPlayerItemWithRandomChoice`, `joinEntityItemWithWorld` | [`drop_current_item`], [`drop_player_item`], [`drop_player_item_with_random_choice`], [`join_entity_item_with_world`] |
//! | 291-306 | `getCurrentPlayerStrVsBlock`, `canHarvestBlock` | [`get_current_player_str_vs_block`], [`can_harvest_block`] |
//! | 308-338 | NBT | [`super::nbt`] (the sleeping player's wake-up: [`read_entity_from_nbt_tail`]) |
//! | 340-355 | `displayGUIChest`, `displayWorkbenchGUI`, `onItemPickup` (empty), `getEyeHeight`, `resetHeight` | [`display_gui`] (`EntityPlayerSP`'s); [`on_item_pickup`]; [`EYE_HEIGHT`], [`reset_height`] |
//! | 357-437 | `attackEntityFrom`, `isPVPEnabled`, `alertWolves` | [`attack_entity_from`], [`alert_wolves`] |
//! | 439-446 | `damageEntity` | [`damage_entity`] |
//! | 448-486 | `displayGUI*` (empty; `EntityPlayerSP`'s: [`display_gui`]), `useCurrentItemOnEntity`, `getCurrentEquippedItem`, `destroyCurrentEquippedItem`, `getYOffset`, `swingItem` | [`use_current_item_on_entity`]; [`get_current_equipped_item`]; [`destroy_current_equipped_item`]; [`get_y_offset`]; [`swing_item`] |
//! | 488-514 | `attackTargetEntityWithCurrentItem` | [`attack_target_entity_with_current_item`] |
//! | 516-535 | `respawnPlayer` (empty), `func_6420_o` (abstract), `onItemStackChanged` (empty), `setEntityDead`, `isEntityInsideOpaqueBlock` | [`set_entity_dead`], [`is_entity_inside_opaque_block`] |
//! | 537-651 | `sleepInBedAt`, `setRenderOffsetForSleep`, `wakeUpPlayer`, `isInBed` | [`sleep_in_bed_at`], [`wake_up_player`] |
//! | 653-665 | `func_25060_a` (static) | [`verify_respawn_coordinates`] |
//! | 667-712 | `getBedOrientationInDegrees`, sleep and spawn getters | [`get_bed_orientation_in_degrees`], [`PlayerData`] |
//! | 698-699 | `addChatMessage` (empty) | `EntityPlayerSP`'s: [`add_chat_message`] |
//! | 714-724 | `triggerAchievement`, `addStat` (empty; `EntityPlayerSP`'s writes the stat file), `jump` | [`trigger_achievement`], [`add_stat`], [`jump`] |
//! | 726-793 | `moveEntityWithHeading`, `addMovementStat`, `addMountedMovementStat`, `fall` | [`move_entity_with_heading`], [`fall`] |
//! | 795-817 | `onKillEntity`, `getItemIcon` (render), `setInPortal` | [`on_kill_entity`], [`set_in_portal`] |
//!
//! # The held stack's item calls
//!
//! `ItemStack.hitEntity` / `onDestroyBlock` / `useItemOnEntity` on the held
//! stack are [`item_stack_hit_entity`], [`item_stack_on_destroy_block`] and
//! [`item_stack_use_item_on_entity`]: thin forwards to the
//! [`ItemStack`] methods (item behaviour, WP-2.6, [`crate::item::behaviour`])
//! that move the stack out of its main slot for the call.

use mc_core::Aabb;
use mc_core::java_hash::ChunkCoordinates;
use mc_core::math::{self, floor_double, jmath};

use super::data_watcher::WatchValue;
use super::item::new_entity_item;
use super::living::{self, LivingData, lv, lv_mut};
use super::{Entity, EntityKindData, PlayerData, base, dispatch};
use crate::block::bed;
use crate::block::{BlockDef, Material};
use crate::item::ItemStack;
use crate::stats::achievement_list as al;
use crate::stats::stat_list as sl;
use crate::stats::{StatBase, stat};
use crate::world::World;
use crate::world::client_requests::{ClientRequest, PickupFx, PlayerGui};
use crate::world::is_instance_of;

/// `Block.blockBed.blockID`.
// src: Block.java:618
const BED: i32 = 26;
/// `Item.appleRed.shiftedIndex`.
// src: Item.java:278 (new ItemFood(4, ...): 256 + 4)
const APPLE_RED: i32 = 260;

/// `getEyeHeight()`: `EntityPlayer`'s constant (not `height * 0.85` as for
/// other living entities; `EntityPlayerSP` does not override it).
// src: EntityPlayer.java:349-351
pub const EYE_HEIGHT: f32 = 0.12;

// ---- EnumStatus, MovementInput, PlayerSpData ------------------------------------

/// `EnumStatus`: the result of [`sleep_in_bed_at`].
// src: EnumStatus.java:3-9
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnumStatus {
    /// `OK`.
    Ok,
    /// `NOT_POSSIBLE_HERE` (the nether).
    NotPossibleHere,
    /// `NOT_POSSIBLE_NOW` (daytime).
    NotPossibleNow,
    /// `TOO_FAR_AWAY`.
    TooFarAway,
    /// `OTHER_PROBLEM` (already sleeping, or dead).
    OtherProblem,
}

impl EnumStatus {
    /// `name()`: the Java constant's name.
    pub fn name(self) -> &'static str {
        match self {
            EnumStatus::Ok => "OK",
            EnumStatus::NotPossibleHere => "NOT_POSSIBLE_HERE",
            EnumStatus::NotPossibleNow => "NOT_POSSIBLE_NOW",
            EnumStatus::TooFarAway => "TOO_FAR_AWAY",
            EnumStatus::OtherProblem => "OTHER_PROBLEM",
        }
    }
}

/// `MovementInput` (MovementInput.java:3-8): what the input source hands
/// the player each tick. `EntityPlayerSP.updatePlayerActionState` copies
/// it into `moveStrafing`/`moveForward`/`isJumping`, `isSneaking` reads
/// `sneak`. The client fills it (`MovementInputFromOptions`, key states:
/// [`MovementInput::from_keys`]); the golden harness's `input` op sets it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MovementInput {
    /// `moveStrafe`.
    pub move_strafe: f32,
    /// `moveForward`.
    pub move_forward: f32,
    /// `field_1177_c`: never read in the client tree.
    pub unused_flag: bool,
    /// `jump`.
    pub jump: bool,
    /// `sneak`.
    pub sneak: bool,
}

impl MovementInput {
    /// `MovementInputFromOptions.updatePlayerMoveState`'s mapping as data:
    /// `forward` and `strafe` are the key sums (+1 forward / left, -1 back /
    /// right), scaled by 0.3 (through `double`) while sneaking.
    // src: MovementInputFromOptions.java:50-76
    pub fn from_keys(forward: f32, strafe: f32, jump: bool, sneak: bool) -> Self {
        let mut m = MovementInput {
            move_strafe: strafe,
            move_forward: forward,
            unused_flag: false,
            jump,
            sneak,
        };
        if m.sneak {
            m.move_strafe = (m.move_strafe as f64 * 0.3) as f32;
            m.move_forward = (m.move_forward as f64 * 0.3) as f32;
        }
        m
    }
}

/// `EntityPlayerSP` (EntityPlayerSP.java:6-10): `movementInput`. Not here:
/// `mc` and the three `MouseFilter`s (client). Also `EntityPlayer`'s
/// `craftingInventory` while a GUI has replaced it
/// ([`PlayerSpData::crafting_inventory`]; the port's only player kind is
/// this one).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlayerSpData {
    /// `movementInput` (EntityPlayerSP.java:6).
    pub movement_input: MovementInput,
    /// `EntityPlayer.craftingInventory` (EntityPlayer.java:9) when it is
    /// not `inventorySlots`: the container of the open `GuiContainer`, which
    /// `GuiContainer.initGui` puts here (GuiContainer.java:16-19) and the
    /// screen works on from then on, as Java's screen and player share the
    /// object. `None` is `inventorySlots` (the `ContainerPlayer`, always
    /// usable, ContainerPlayer.java:61-63; the port keeps no persistent
    /// one, WP-2.5b), which `closeScreen` restores (EntityPlayer.java:131-133).
    /// [`on_update`] closes the window when it is no longer usable
    /// (EntityPlayer.java:81-84). The oracle's `HarnessPlayer` never opens a
    /// GUI, so it stays `None` there. Not saved (Java does not save it).
    pub crafting_inventory: Option<crate::inventory::container::Container>,
    /// Whether `EntityPlayerSP.onLivingUpdate`'s own body runs (module docs):
    /// true in the client, false for the golden harness (`HarnessPlayer`).
    pub client_additions: bool,
    /// The last [`sleep_in_bed_at`] result. Not vanilla state: the
    /// observation the harness's `sleep` op reports, as the oracle's
    /// `HarnessPlayer` captures it (vanilla's only consumer is
    /// `BlockBed.blockActivated`, which discards it). Never read by the
    /// simulation.
    pub last_sleep_status: Option<EnumStatus>,
}

// ---- level accessors ---------------------------------------------------------------

/// The `EntityPlayer` level of `e`; panics where Java's cast to
/// `EntityPlayer` would throw (a caller passed a non-player).
fn player(e: &Entity) -> &PlayerData {
    e.kind
        .player()
        .expect("EntityPlayer method on a non-player (Java ClassCastException)")
}

/// The `EntityPlayer` level of `e`, mutably.
fn player_mut(e: &mut Entity) -> &mut PlayerData {
    e.kind
        .player_mut()
        .expect("EntityPlayer method on a non-player (Java ClassCastException)")
}

/// The `EntityPlayerSP` level of `e`.
fn sp(e: &Entity) -> &PlayerSpData {
    match &e.kind {
        EntityKindData::PlayerSp(_, _, s) => s,
        _ => panic!("EntityPlayerSP method on another entity (Java ClassCastException)"),
    }
}

/// The `EntityPlayerSP` level of `e`, mutably.
fn sp_mut(e: &mut Entity) -> &mut PlayerSpData {
    match &mut e.kind {
        EntityKindData::PlayerSp(_, _, s) => s,
        _ => panic!("EntityPlayerSP method on another entity (Java ClassCastException)"),
    }
}

/// `Math.round(float)`: nearest, ties toward positive infinity; NaN is 0,
/// out-of-range values saturate.
fn java_round_f32(a: f32) -> i32 {
    if a.is_nan() {
        return 0;
    }
    let f = a.floor();
    // Exact: below 2^23 `a - floor(a)` has no rounding; above, `a` is whole.
    let r = if a - f >= 0.5 { f + 1.0 } else { f };
    r as i32
}

/// `Math.round(double)` cast to `int`, as `(int)Math.round(d)`.
fn java_round_f64_to_int(a: f64) -> i32 {
    if a.is_nan() {
        return 0;
    }
    let f = a.floor();
    let r = if a - f >= 0.5 { f + 1.0 } else { f };
    (r as i64) as i32
}

// ---- construction ---------------------------------------------------------------

/// `new EntityPlayerSP(world)` without the client half: the constructor
/// chain `Entity(World)` → `EntityPlayer.entityInit` → `EntityLiving` field
/// initialisers and body (four `Math.random()` draws,
/// [`LivingData::new`]) → `EntityPlayer` field initialisers and body
/// (`EntityPlayerSP`'s body sets `dimension` and `username` from the
/// session, client: the golden harness sets the username itself).
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
    // EntityPlayer field initialisers (EntityPlayer.java:7-38).
    let mut p = PlayerData::default();
    // Constructor body (EntityPlayer.java:41-51).
    // inventorySlots / craftingInventory: the ContainerPlayer (none kept,
    // WP-2.5b; PlayerSpData::crafting_inventory None).
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

// ---- update ---------------------------------------------------------------------

/// `EntityPlayer.onUpdate()`: the sleep timer (a sleeper wakes when the bed
/// is gone or at daytime), `EntityLiving.onUpdate`, the cape's lagging
/// position, and the minecart-ride start reset.
// src: EntityPlayer.java:59-125
pub fn on_update(e: &mut Entity, world: &mut World) {
    if player(e).is_player_sleeping() {
        let p = player_mut(e);
        p.sleep_timer = p.sleep_timer.wrapping_add(1);
        if p.sleep_timer > 100 {
            p.sleep_timer = 100;
        }
        if !world.multiplayer_world {
            if !is_in_bed(e, world) {
                wake_up_player(e, world, true, true, false);
            } else if world.is_daytime() {
                wake_up_player(e, world, false, true, true);
            }
        }
    } else if player(e).sleep_timer > 0 {
        let p = player_mut(e);
        p.sleep_timer = p.sleep_timer.wrapping_add(1);
        if p.sleep_timer >= 110 {
            p.sleep_timer = 0;
        }
    }

    living::on_update(e, world);
    // src: EntityPlayer.java:81-84. Without a GUI, craftingInventory is the
    // ContainerPlayer, always usable (PlayerSpData::crafting_inventory).
    if !world.multiplayer_world
        && let Some(c) = sp_mut(e).crafting_inventory.take()
    {
        if c.is_usable_by_player(world, e) {
            sp_mut(e).crafting_inventory = Some(c);
        } else {
            // this.closeScreen(); this.craftingInventory = this.inventorySlots
            // (the container stays taken out: None is inventorySlots).
            close_screen_dropping(e, world, c);
        }
    }

    let (px, py, pz) = (e.base.pos_x, e.base.pos_y, e.base.pos_z);
    let p = player_mut(e);
    p.prev_chasing_pos_x = p.chasing_pos_x;
    p.prev_chasing_pos_y = p.chasing_pos_y;
    p.prev_chasing_pos_z = p.chasing_pos_z;
    let var1 = px - p.chasing_pos_x;
    let var3 = py - p.chasing_pos_y;
    let var5 = pz - p.chasing_pos_z;
    let var7 = 10.0;
    if var1 > var7 {
        p.chasing_pos_x = px;
        p.prev_chasing_pos_x = px;
    }
    if var5 > var7 {
        p.chasing_pos_z = pz;
        p.prev_chasing_pos_z = pz;
    }
    if var3 > var7 {
        p.chasing_pos_y = py;
        p.prev_chasing_pos_y = py;
    }
    if var1 < -var7 {
        p.chasing_pos_x = px;
        p.prev_chasing_pos_x = px;
    }
    if var5 < -var7 {
        p.chasing_pos_z = pz;
        p.prev_chasing_pos_z = pz;
    }
    if var3 < -var7 {
        p.chasing_pos_y = py;
        p.prev_chasing_pos_y = py;
    }
    p.chasing_pos_x += var1 * 0.25;
    p.chasing_pos_z += var5 * 0.25;
    p.chasing_pos_y += var3 * 0.25;
    add_stat(e, stat(sl::MINUTES_PLAYED_STAT), 1);
    if e.base.riding_entity.is_none() {
        player_mut(e).start_minecart_riding_coordinate = None;
    }
}

/// `EntityPlayer.isMovementBlocked()`: dead or asleep.
// src: EntityPlayer.java:127-129
pub fn is_movement_blocked(e: &Entity) -> bool {
    lv(e).health <= 0 || player(e).is_player_sleeping()
}

/// `EntityPlayer.updateRidden()`: `EntityLiving`'s, then the camera bob
/// stops and the mounted distance stat.
// src: EntityPlayer.java:140-148
pub fn update_ridden(e: &mut Entity, world: &mut World) {
    let (var1, var3, var5) = (e.base.pos_x, e.base.pos_y, e.base.pos_z);
    living::update_ridden(e, world);
    let p = player_mut(e);
    p.prev_camera_yaw = p.camera_yaw;
    p.camera_yaw = 0.0;
    let (dx, dy, dz) = (
        e.base.pos_x - var1,
        e.base.pos_y - var3,
        e.base.pos_z - var5,
    );
    add_mounted_movement_stat(e, world, dx, dy, dz);
}

/// `EntityPlayer.preparePlayerToSpawn()`: standing size, then `Entity`'s
/// (move up out of blocks), full health.
// src: EntityPlayer.java:150-156
pub fn prepare_player_to_spawn(e: &mut Entity, world: &mut World) {
    e.base.y_offset = 1.62;
    e.base.set_size(0.6, 1.8);
    base::prepare_player_to_spawn(e, world);
    let l = lv_mut(e);
    l.health = 20;
    l.death_time = 0;
}

/// `EntityPlayer.updatePlayerActionState()`: only the arm swing (8 ticks),
/// then `swingProgress`. No `super` call: `EntityLiving`'s idle AI does not
/// run for a player.
// src: EntityPlayer.java:158-170
pub fn update_player_action_state(e: &mut Entity) {
    let p = player_mut(e);
    if p.is_swinging {
        p.swing_progress_int = p.swing_progress_int.wrapping_add(1);
        if p.swing_progress_int >= 8 {
            p.swing_progress_int = 0;
            p.is_swinging = false;
        }
    } else {
        p.swing_progress_int = 0;
    }
    let v = p.swing_progress_int as f32 / 8.0;
    lv_mut(e).swing_progress = v;
}

/// `EntityPlayer.onLivingUpdate()`: the peaceful heal, the inventory's
/// animations, `EntityLiving.onLivingUpdate`, the camera bob and tilt, then
/// `onCollideWithPlayer` of every live entity within 1 block horizontally.
///
/// vanilla bug (transcribed): `ticksExisted % 20 * 12 == 0` parses as
/// `(ticksExisted % 20) * 12 == 0`, so peaceful healing comes every 20
/// ticks, not every 240.
// src: EntityPlayer.java:172-212
pub fn on_living_update(e: &mut Entity, world: &mut World) {
    if world.difficulty_setting == 0
        && lv(e).health < 20
        && (e.base.ticks_existed % 20).wrapping_mul(12) == 0
    {
        living::heal(e, 1);
    }
    crate::inventory::player::decrement_animations(world, e);
    {
        let p = player_mut(e);
        p.prev_camera_yaw = p.camera_yaw;
    }
    living::on_living_update(e, world);
    let b = &e.base;
    let mut var1 = math::sqrt_double(b.motion_x * b.motion_x + b.motion_z * b.motion_z);
    let mut var2 = jmath::atan(-b.motion_y * 0.20000000298023224) as f32 * 15.0;
    if var1 > 0.1 {
        var1 = 0.1;
    }
    let on_ground = b.on_ground;
    let health = lv(e).health;
    if !on_ground || health <= 0 {
        var1 = 0.0;
    }
    if on_ground || health <= 0 {
        var2 = 0.0;
    }
    let p = player_mut(e);
    p.camera_yaw += (var1 - p.camera_yaw) * 0.4;
    let l = lv_mut(e);
    l.camera_pitch += (var2 - l.camera_pitch) * 0.8;
    if health > 0 {
        let bb = e.base.bounding_box.expand(1.0, 0.0, 1.0);
        let var3 = world.get_entities_within_aabb_excluding_entity(Some(e.base.entity_id), &bb);
        for id in var3 {
            // collideWithPlayer(var5) (EntityPlayer.java:210-212)
            world.with_entity_mut(id, |w, var5| {
                if !var5.base.is_dead {
                    dispatch::on_collide_with_player(var5, w, e);
                }
            });
        }
    }
}

// ---- death, score, drops -------------------------------------------------------

/// `EntityPlayer.onDeath(attacker)`: `EntityLiving`'s, then the small dead
/// box, a hop, the whole inventory dropped (`dropAllItems`), a push away from
/// the attacker, `yOffset` 0.1.
///
/// `username.equals("Notch")` dereferences the username: a player without
/// one (a fresh `EntityPlayer`; the client and the golden harness always
/// set it) throws in Java; recorded here and taken as not Notch.
// src: EntityPlayer.java:218-237
pub fn on_death(e: &mut Entity, world: &mut World, var1: Option<&Entity>) {
    living::on_death(e, world, var1);
    e.base.set_size(0.2, 0.2);
    let (x, y, z) = (e.base.pos_x, e.base.pos_y, e.base.pos_z);
    e.base.set_position(x, y, z);
    e.base.motion_y = 0.10000000149011612;
    match player(e).username.as_deref() {
        Some("Notch") => {
            drop_player_item_with_random_choice(
                e,
                world,
                Some(ItemStack::new(APPLE_RED, 1, 0)),
                true,
            );
        }
        Some(_) => {}
        None => world.note_unimplemented(
            "EntityPlayer.onDeath with a null username (Java NullPointerException)",
        ),
    }
    crate::inventory::player::drop_all_items(world, e);
    if var1.is_some() {
        let a = (lv(e).attacked_at_yaw + e.base.rotation_yaw) * 3.1415927 / 180.0;
        e.base.motion_x = (-math::cos(a) * 0.1) as f64;
        e.base.motion_z = (-math::sin(a) * 0.1) as f64;
    } else {
        e.base.motion_x = 0.0;
        e.base.motion_z = 0.0;
    }
    e.base.y_offset = 0.1;
    add_stat(e, stat(sl::DEATHS_STAT), 1);
}

/// `EntityPlayer.addToPlayerScore(victim, score)`. `victim_is_player` is
/// `victim instanceof EntityPlayer`.
// src: EntityPlayer.java:239-247
pub fn add_to_player_score(e: &mut Entity, victim_is_player: bool, var2: i32) {
    let p = player_mut(e);
    p.score = p.score.wrapping_add(var2);
    if victim_is_player {
        add_stat(e, stat(sl::PLAYER_KILLS_STAT), 1);
    } else {
        add_stat(e, stat(sl::MOB_KILLS_STAT), 1);
    }
}

/// The calls `EntityLiving.onDeath(attacker)` makes on its attacker, as
/// data: `attacker.addToPlayerScore(this, scoreValue)` when
/// `scoreValue >= 0`, then `attacker.onKillEntity(this)`. Only an
/// `EntityPlayer` attacker does anything with them (Entity.java:759-760,
/// :1094-1095 are empty), and it reads only two `instanceof` tests of the
/// victim.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KillHook {
    /// The attacker's `entityId`.
    pub attacker: i32,
    /// `scoreValue` when `addToPlayerScore` is called (`>= 0`).
    pub score: Option<i32>,
    /// `victim instanceof EntityPlayer` (EntityPlayer.java:241).
    pub victim_is_player: bool,
    /// `victim instanceof EntityMob` (EntityPlayer.java:796).
    pub victim_is_mob: bool,
}

impl KillHook {
    /// Runs the hook on the attacker `e` (an `EntityPlayer`; other kinds have
    /// the empty `Entity` bodies).
    // src: EntityLiving.java:408-414
    pub fn apply(&self, e: &mut Entity) {
        if !e.kind.is_player() {
            return;
        }
        if let Some(score) = self.score {
            add_to_player_score(e, self.victim_is_player, score);
        }
        on_kill_entity(e, self.victim_is_mob);
    }
}

/// `EntityLiving.onDeath`'s `attacker.addToPlayerScore(this, score)` (for
/// `Some(score)`) and `attacker.onKillEntity(this)`, run on the attacker
/// entity, not on the copy or borrow `attacker` the port holds (an arrow's
/// owner is passed as a copy; the player's own melee hit lends itself).
///
/// A player attacker in its slot is changed there at once, in Java's
/// order. One that is out of its slot is held up the stack (the player
/// attacking with its own hand): the hook waits in
/// `World::pending_kill_hooks` until [`take_pending_kill_hooks`] runs it on
/// the player in hand (right after the hit, in
/// [`attack_target_entity_with_current_item`], or when the world puts the
/// player back in its slot). Nothing else reads the score or the
/// statistics in between. A player that has left `loadedEntityList` (dead
/// for 20 ticks, its arrow still in flight) gets the hook on the removed
/// object the world keeps for `playerEntities`
/// ([`World::removed_entity`]), as Java updates the orphaned object; the
/// client reads the dead player and its `Minecraft.statFileWriter` from
/// there (WP-4.7).
// src: EntityLiving.java:408-414
pub fn kill_hooks(world: &mut World, attacker: &Entity, victim: &Entity, score: Option<i32>) {
    if !attacker.kind.is_player() {
        // src: Entity.java:759-760, :1094-1095 (empty)
        return;
    }
    let h = KillHook {
        attacker: attacker.base.entity_id,
        score,
        victim_is_player: victim.kind.is_player(),
        victim_is_mob: is_instance_of(&victim.kind, "EntityMob"),
    };
    if world
        .with_entity_mut(h.attacker, |_, p| h.apply(p))
        .is_some()
    {
        return;
    }
    if world.entity_index(h.attacker).is_some() {
        world.pending_kill_hooks.push(h);
    } else if let Some(p) = world.removed_entity_mut(h.attacker) {
        h.apply(p);
    } else {
        world.note_unimplemented(
            "EntityLiving.onDeath: the attacker is no longer in loadedEntityList (Java still holds it)",
        );
    }
}

/// Runs and removes the waiting [`KillHook`]s of the player `e`, now in
/// hand ([`kill_hooks`]), in the order they were made.
pub fn take_pending_kill_hooks(e: &mut Entity, world: &mut World) {
    if world.pending_kill_hooks.is_empty() {
        return;
    }
    let id = e.base.entity_id;
    let (mine, rest): (Vec<KillHook>, Vec<KillHook>) =
        std::mem::take(&mut world.pending_kill_hooks)
            .into_iter()
            .partition(|h| h.attacker == id);
    world.pending_kill_hooks = rest;
    for h in mine {
        h.apply(e);
    }
}

/// `dropCurrentItem()`: one item of the held slot, thrown forward (the
/// client's drop key, Minecraft.java).
// src: EntityPlayer.java:249-251
pub fn drop_current_item(e: &mut Entity, world: &mut World) {
    let slot = player(e).inventory.current_item;
    let s = player_mut(e).inventory.decr_stack_size(slot, 1);
    drop_player_item_with_random_choice(e, world, s, false);
}

/// `dropPlayerItem(stack)`: thrown forward.
// src: EntityPlayer.java:253-255
pub fn drop_player_item(e: &mut Entity, world: &mut World, var1: Option<ItemStack>) {
    drop_player_item_with_random_choice(e, world, var1, false);
}

/// `dropPlayerItemWithRandomChoice(stack, random)`: an `EntityItem` at the
/// eyes (`posY - 0.3 + getEyeHeight()`), pick-up delay 40; `random` scatters
/// it (two `rand` draws: speed, direction), otherwise it is thrown along the
/// look with a little jitter (four `rand` draws).
// src: EntityPlayer.java:257-285
pub fn drop_player_item_with_random_choice(
    e: &mut Entity,
    world: &mut World,
    var1: Option<ItemStack>,
    var2: bool,
) {
    let Some(stack) = var1 else { return };
    let eye = dispatch::get_eye_height(e, world);
    let mut var3 = new_entity_item(
        world,
        e.base.pos_x,
        e.base.pos_y - 0.30000001192092896 + eye as f64,
        e.base.pos_z,
        stack,
    );
    if let Some(d) = var3.kind.item_mut() {
        d.delay_before_can_pickup = 40;
    }
    let b = &mut e.base;
    let m = &mut var3.base;
    if var2 {
        let var5 = b.rand.next_float() * 0.5;
        let var6 = b.rand.next_float() * 3.1415927 * 2.0;
        m.motion_x = (-math::sin(var6) * var5) as f64;
        m.motion_z = (math::cos(var6) * var5) as f64;
        m.motion_y = 0.20000000298023224;
    } else {
        let mut var4 = 0.3f32;
        let yaw = b.rotation_yaw / 180.0 * 3.1415927;
        let pitch = b.rotation_pitch / 180.0 * 3.1415927;
        m.motion_x = (-math::sin(yaw) * math::cos(pitch) * var4) as f64;
        m.motion_z = (math::cos(yaw) * math::cos(pitch) * var4) as f64;
        m.motion_y = (-math::sin(pitch) * var4 + 0.1) as f64;
        var4 = 0.02;
        let var5 = b.rand.next_float() * 3.1415927 * 2.0;
        var4 *= b.rand.next_float();
        m.motion_x += jmath::cos(var5 as f64) * var4 as f64;
        m.motion_y += ((b.rand.next_float() - b.rand.next_float()) * 0.1) as f64;
        m.motion_z += jmath::sin(var5 as f64) * var4 as f64;
    }
    join_entity_item_with_world(world, var3);
    add_stat(e, stat(sl::DROP_STAT), 1);
}

/// `joinEntityItemWithWorld(item)`: `worldObj.entityJoinedWorld(item)`.
// src: EntityPlayer.java:287-289
pub fn join_entity_item_with_world(world: &mut World, var1: Entity) {
    world.spawn_entity_in_world(var1);
}

// ---- block queries ----------------------------------------------------------------

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

// ---- NBT tail ----------------------------------------------------------------------

/// The part of `EntityPlayer.readEntityFromNBT` that needs the whole
/// entity: a player saved asleep gets `bedChunkCoordinates` at its feet and
/// is woken (`wakeUpPlayer(true, true, false)`), before the spawn
/// coordinates are read ([`super::nbt`] calls this between the two).
// src: EntityPlayer.java:315-318
pub fn read_entity_from_nbt_tail(e: &mut Entity, world: &mut World) {
    if player(e).sleeping {
        let c = ChunkCoordinates::new(
            floor_double(e.base.pos_x),
            floor_double(e.base.pos_y),
            floor_double(e.base.pos_z),
        );
        player_mut(e).bed_chunk_coordinates = Some(c);
        wake_up_player(e, world, true, true, false);
    }
}

// ---- size, damage --------------------------------------------------------------------

/// `onItemPickup(entity, count)`: called by the three pickups
/// (`EntityItem`, `EntityArrow`, `EntitySnowball`/`EntityEgg`
/// `.onCollideWithPlayer`) after the stack went into the inventory.
/// `EntityPlayer`'s body is empty (EntityPlayer.java:346-347), and so is
/// the oracle's `HarnessPlayer`'s. `EntityPlayerSP`'s adds an
/// `EntityPickupFX(theWorld, entity, this, -0.5F)` (the entity flying to
/// the player) to the effect renderer: [`ClientRequest::PickupFx`], with
/// the picked entity as it is now ([`PickupFx`]). No simulation state.
// src: EntityPlayerSP.java:133-135
pub fn on_item_pickup(e: &mut Entity, world: &mut World, picked: &Entity, _count: i32) {
    if is_client_player(e) {
        world
            .client_requests
            .push(ClientRequest::PickupFx(PickupFx {
                picked: Box::new(picked.clone()),
                player_id: e.base.entity_id,
                y_offs: -0.5,
            }));
    }
}

/// `displayGUIChest` / `displayWorkbenchGUI` / `displayGUIFurnace` /
/// `displayGUIDispenser` / `displayGUIEditSign`: empty in `EntityPlayer`
/// (EntityPlayer.java:340-344, :448-456); `EntityPlayerSP` opens the screen
/// (`mc.displayGuiScreen(new Gui…)`, EntityPlayerSP.java:113-131), here
/// [`ClientRequest::DisplayGui`].
// src: EntityPlayerSP.java:113-131
pub fn display_gui(e: &Entity, world: &mut World, gui: PlayerGui) {
    if is_client_player(e) {
        world.client_requests.push(ClientRequest::DisplayGui(gui));
    }
}

/// `closeScreen()`: `EntityPlayer`'s resets `craftingInventory` to
/// `inventorySlots` (EntityPlayer.java:131-133:
/// [`PlayerSpData::crafting_inventory`] to `None`); `EntityPlayerSP`'s then
/// closes the screen (`mc.displayGuiScreen(null)`), here
/// [`ClientRequest::CloseScreen`]. A container still open is closed without
/// its drops (the client's screen makes them, [`close_screen_dropping`] for
/// the simulation's own close).
// src: EntityPlayer.java:131-133, EntityPlayerSP.java:108-111
pub fn close_screen(e: &mut Entity, world: &mut World) {
    sp_mut(e).crafting_inventory = None;
    if is_client_player(e) {
        world.client_requests.push(ClientRequest::CloseScreen);
    }
}

/// [`close_screen`] on the container `c` the player had open (already taken
/// out of [`PlayerSpData::crafting_inventory`]), from inside the player's
/// update (EntityPlayer.java:82): for the client's `EntityPlayerSP`,
/// `mc.displayGuiScreen(null)` runs the screen's `onGuiClosed` at once,
/// whose `onCraftGuiClosed` drops the stack on the cursor and the crafting
/// grid (`PlayerController.func_20086_a` on `craftingInventory`, now the
/// `ContainerPlayer`, then `GuiCrafting`'s own call on the
/// `ContainerWorkbench`, GuiContainer.java:162-166, GuiCrafting.java:10-13:
/// one call on `c` makes the same drops in the same order, see the
/// client's `GuiContainer::on_gui_closed`). So the dropped items join the
/// world during the player's update and are updated later in the same
/// `updateEntities`, as in vanilla; the client's screen then closes with
/// nothing left to drop. `EntityPlayer`'s `closeScreen` (the oracle's
/// `HarnessPlayer`) only resets `craftingInventory`: nothing is dropped.
// src: EntityPlayer.java:81-84, :131-133; EntityPlayerSP.java:108-111
pub fn close_screen_dropping(
    e: &mut Entity,
    world: &mut World,
    mut c: crate::inventory::container::Container,
) {
    if is_client_player(e) {
        c.on_craft_gui_closed(world, e);
    }
    close_screen(e, world);
}

/// [`PlayerSpData::crafting_inventory`]: the open GUI's container, `None`
/// for `inventorySlots`.
pub fn crafting_inventory(e: &Entity) -> Option<&crate::inventory::container::Container> {
    match &e.kind {
        EntityKindData::PlayerSp(_, _, s) => s.crafting_inventory.as_ref(),
        _ => None,
    }
}

/// `player.craftingInventory = container` (`GuiContainer.initGui`,
/// GuiContainer.java:16-19); `None` restores `inventorySlots`. Returns the
/// container it replaces.
// src: GuiContainer.java:16-19, EntityPlayer.java:9
pub fn set_crafting_inventory(
    e: &mut Entity,
    c: Option<crate::inventory::container::Container>,
) -> Option<crate::inventory::container::Container> {
    std::mem::replace(&mut sp_mut(e).crafting_inventory, c)
}

/// `addChatMessage(key)`: empty in `EntityPlayer` (EntityPlayer.java:698-699);
/// `EntityPlayerSP` writes the translated line to the chat
/// (`mc.ingameGUI.addChatMessageTranslate(key)`), here
/// [`ClientRequest::ChatMessage`].
// src: EntityPlayerSP.java:172-174
pub fn add_chat_message(e: &Entity, world: &mut World, key: &'static str) {
    if is_client_player(e) {
        world
            .client_requests
            .push(ClientRequest::ChatMessage { key });
    }
}

/// Whether `e` is the client's `EntityPlayerSP` with its own bodies on
/// ([`PlayerSpData::client_additions`]; module docs), which is where the
/// `Minecraft` calls are.
pub fn is_client_player(e: &Entity) -> bool {
    matches!(&e.kind, EntityKindData::PlayerSp(_, _, s) if s.client_additions)
}

/// `resetHeight()`: `yOffset = 1.62F`.
// src: EntityPlayer.java:353-355
pub fn reset_height(e: &mut Entity) {
    e.base.y_offset = 1.62;
}

/// `EntityPlayer.attackEntityFrom(attacker, damage)`: wakes a sleeper;
/// damage from a mob or an arrow is scaled by difficulty (0 at peaceful);
/// tamed wolves are told about a living attacker (the arrow's shooter for
/// an arrow); then `EntityLiving`'s.
// src: EntityPlayer.java:357-396
pub fn attack_entity_from(
    e: &mut Entity,
    world: &mut World,
    var1: Option<&Entity>,
    mut var2: i32,
) -> bool {
    lv_mut(e).entity_age = 0;
    if lv(e).health <= 0 {
        return false;
    }
    if player(e).is_player_sleeping() && !world.multiplayer_world {
        wake_up_player(e, world, true, true, false);
    }
    if let Some(a) = var1
        && (is_instance_of(&a.kind, "EntityMob") || is_instance_of(&a.kind, "EntityArrow"))
    {
        if world.difficulty_setting == 0 {
            var2 = 0;
        }
        if world.difficulty_setting == 1 {
            var2 = var2 / 3 + 1;
        }
        if world.difficulty_setting == 3 {
            var2 = var2.wrapping_mul(3) / 2;
        }
    }
    if var2 == 0 {
        return false;
    }
    // var3: the arrow's owner when there is one, else the attacker.
    if let Some(a) = var1 {
        let owner = a.kind.arrow().and_then(|d| d.owner);
        match owner {
            Some(oid) if oid == e.base.entity_id => {
                // The shooter is this player: alertWolves returns at once
                // for a player attacker (isPVPEnabled() is false).
            }
            // The loaded owner, or the removed one Java's field still
            // holds (World::removed_entity).
            Some(oid) => match world.entity_object(oid) {
                Some(o) => {
                    if o.kind.living().is_some() {
                        let o = o.clone();
                        alert_wolves(e, world, &o, false);
                    }
                }
                None => world.note_unimplemented(
                    "EntityPlayer.attackEntityFrom: the arrow's owner is out of its slot or no longer kept",
                ),
            },
            None => {
                if a.kind.living().is_some() {
                    alert_wolves(e, world, a, false);
                }
            }
        }
    }
    add_stat(e, stat(sl::DAMAGE_TAKEN_STAT), var2);
    living::attack_entity_from(e, world, var1, var2)
}

/// `isPVPEnabled()`: false in singleplayer.
// src: EntityPlayer.java:398-400
pub fn is_pvp_enabled() -> bool {
    false
}

/// `alertWolves(attacker, skipSitting)`: unless the attacker is a creeper,
/// a ghast or this player's own tamed wolf, and unless it is a player
/// (`isPVPEnabled()` is false), every tamed wolf of this player within the
/// box 16 blocks around the player (4 vertically) that has no target
/// stands up and targets the attacker; with `skipSitting` (the player's own
/// hit) a sitting wolf stays put.
///
/// `e` is the player (in hand), `var1` the attacker (in hand or a copy).
/// The wolves found are changed in their slots. The attacker may itself be
/// a wolf out of its slot (an angry wolf biting the player); it never
/// passes the tests (a tamed wolf of this player returned at the top, any
/// other is untamed or has another owner), so it is passed to the search as
/// the entity in hand and skipped. A player without a username (Java's
/// `NullPointerException` on `username.equals`) is recorded and matches no
/// owner.
// src: EntityPlayer.java:402-437
pub fn alert_wolves(e: &Entity, world: &mut World, var1: &Entity, var2: bool) {
    if is_instance_of(&var1.kind, "EntityCreeper") || is_instance_of(&var1.kind, "EntityGhast") {
        return;
    }
    let username = player(e).username.clone();
    let is_owner = |world: &World, owner: &str| match username.as_deref() {
        Some(u) => u == owner,
        None => {
            world.note_unimplemented(
                "EntityPlayer.alertWolves with a null username (Java NullPointerException)",
            );
            false
        }
    };
    if let EntityKindData::Wolf(..) = var1.kind {
        // src: EntityPlayer.java:404-409
        if super::wolf::is_wolf_tamed(var1) && is_owner(world, super::wolf::get_wolf_owner(var1)) {
            return;
        }
    }
    if !var1.kind.is_player() || is_pvp_enabled() {
        let b = &e.base;
        let bb = Aabb::new(
            b.pos_x,
            b.pos_y,
            b.pos_z,
            b.pos_x + 1.0,
            b.pos_y + 1.0,
            b.pos_z + 1.0,
        )
        .expand(16.0, 4.0, 16.0);
        let var7 = world.get_entities_within_aabb("EntityWolf", &bb, Some(var1));
        let target = var1.base.entity_id;
        for id in var7 {
            if id == target {
                // The attacking wolf (doc comment): fails the tests below.
                continue;
            }
            let Some(var6) = world.entity(id) else {
                world.note_unimplemented("EntityPlayer.alertWolves: a wolf out of its slot");
                continue;
            };
            // src: EntityPlayer.java:425-430
            if !super::wolf::is_wolf_tamed(var6) {
                continue;
            }
            if super::creature::creature(&var6.kind).is_some_and(|c| c.player_to_attack.is_some()) {
                continue;
            }
            if !is_owner(world, super::wolf::get_wolf_owner(var6)) {
                continue;
            }
            if var2 && super::wolf::is_wolf_sitting(var6) {
                continue;
            }
            // src: EntityPlayer.java:432-433 (setTarget: EntityCreature.java:178-180)
            let var6 = world
                .entity_mut(id)
                .expect("the wolf was in its slot a moment ago");
            super::wolf::set_wolf_sitting(var6, false);
            if let Some(c) = super::creature::creature_mut(&mut var6.kind) {
                c.player_to_attack = Some(target);
            }
        }
    }
}

/// `EntityPlayer.damageEntity(amount)`: armour absorbs
/// `getTotalArmorValue()` 25ths of the damage, the fraction carried over in
/// `damageRemainder`; every armour piece takes the full damage as wear.
// src: EntityPlayer.java:439-446
pub fn damage_entity(e: &mut Entity, world: &mut World, mut var1: i32) {
    let var2 = 25 - player(e).inventory.get_total_armor_value();
    let var3 = var1
        .wrapping_mul(var2)
        .wrapping_add(player(e).damage_remainder);
    crate::inventory::player::damage_armor(world, e, var1);
    var1 = var3 / 25;
    player_mut(e).damage_remainder = var3 % 25;
    living::damage_entity(e, var1);
}

/// `useCurrentItemOnEntity(target)`: the target's `interact(this)`
/// ([`dispatch::interact`]); when that is false, the held stack's
/// `useItemOnEntity` on a living target ([`item_stack_use_item_on_entity`]),
/// and a stack left at size 0 or less leaves the hotbar. Java returns
/// `void`; the port returns `target.interact(this)` so that a caller (the
/// golden `interact` op, which reports it) need not inline the method.
// src: EntityPlayer.java:457-469
pub fn use_current_item_on_entity(e: &mut Entity, world: &mut World, var1: &mut Entity) -> bool {
    let result = dispatch::interact(var1, world, e);
    if !result {
        // var2 = getCurrentEquippedItem(): the stack in hotbar slot currentItem.
        if let Some(slot) = current_item_slot(e)
            && var1.kind.living().is_some()
        {
            item_stack_use_item_on_entity(e, world, slot, var1);
            if player(e).inventory.main_inventory[slot].is_some_and(|s| s.stack_size <= 0) {
                // var2.onItemDestroyedByUse(this): empty (ItemStack.java:167-168)
                destroy_current_equipped_item(e);
            }
        }
    }
    result
}

/// `getCurrentEquippedItem()`: `inventory.getCurrentItem()`.
// src: EntityPlayer.java:471-473
pub fn get_current_equipped_item(e: &Entity) -> Option<&ItemStack> {
    player(e).inventory.get_current_item()
}

/// `destroyCurrentEquippedItem()`: empties the selected hotbar slot.
// src: EntityPlayer.java:475-477
pub fn destroy_current_equipped_item(e: &mut Entity) {
    let inv = &mut player_mut(e).inventory;
    let slot = inv.current_item;
    inv.set_inventory_slot_contents(slot, None);
}

/// `EntityPlayer.getYOffset()`: `yOffset - 0.5`.
// src: EntityPlayer.java:479-481
pub fn get_y_offset(e: &Entity) -> f64 {
    (e.base.y_offset - 0.5) as f64
}

/// `swingItem()`: starts the arm swing (EntityPlayerSP does not override it).
// src: EntityPlayer.java:483-486
pub fn swing_item(e: &mut Entity) {
    let p = player_mut(e);
    p.swing_progress_int = -1;
    p.is_swinging = true;
}

/// `attackTargetEntityWithCurrentItem(target)`: the held item's damage (1
/// bare-handed), +1 while falling; the target's `attackEntityFrom`; the
/// held stack's `hitEntity` (tool wear) on a living target, removing a
/// broken stack; wolves told about a living target that survived. No
/// knock-back of its own: the target's `attackEntityFrom` applies it.
// src: EntityPlayer.java:488-514
pub fn attack_target_entity_with_current_item(
    e: &mut Entity,
    world: &mut World,
    var1: &mut Entity,
) {
    let mut var2 = player(e).inventory.get_damage_vs_entity();
    if var2 > 0 {
        if e.base.motion_y < 0.0 {
            var2 = var2.wrapping_add(1);
        }
        dispatch::attack_entity_from(var1, world, Some(e), var2);
        // A kill ran `onDeath`'s score and kill calls on this player, held
        // here (kill_hooks): they run now, before the item's hitEntity.
        // src: EntityLiving.java:408-414
        take_pending_kill_hooks(e, world);
        // var3 = getCurrentEquippedItem(): the stack in hotbar slot currentItem.
        let held = current_item_slot(e);
        if let Some(slot) = held
            && var1.kind.living().is_some()
        {
            item_stack_hit_entity(e, world, slot, var1);
            if player(e).inventory.main_inventory[slot].is_some_and(|s| s.stack_size <= 0) {
                // var3.onItemDestroyedByUse(this): empty (ItemStack.java:167-168)
                destroy_current_equipped_item(e);
            }
        }
        if var1.kind.living().is_some() {
            if dispatch::is_entity_alive(var1) {
                alert_wolves(e, world, var1, true);
            }
            add_stat(e, stat(sl::DAMAGE_DEALT_STAT), var2);
        }
    }
}

/// `inventory.getCurrentItem()` as the slot holding it (`None` for null).
fn current_item_slot(e: &Entity) -> Option<usize> {
    let inv = &player(e).inventory;
    inv.get_current_item().map(|_| inv.current_item as usize)
}

/// `EntityPlayer.setEntityDead()`: `Entity`'s, then
/// `inventorySlots.onCraftGuiClosed(this)` and the same for
/// `craftingInventory`: `Container.onCraftGuiClosed` drops the stack held
/// by the cursor (`inventory.itemStack`). Without a GUI `craftingInventory`
/// is the same `ContainerPlayer`, so its part runs twice; its crafting grid
/// (dropped too, ContainerPlayer.java:48-59) is empty without a GUI (the
/// port keeps no persistent one, WP-2.5b). With a GUI's container open
/// ([`PlayerSpData::crafting_inventory`]) that container's
/// `onCraftGuiClosed` runs instead (its grid's drops too); it stays the
/// player's `craftingInventory`, as in Java.
// src: EntityPlayer.java:524-531, Container.java:152-159
pub fn set_entity_dead(e: &mut Entity, world: &mut World) {
    e.base.set_entity_dead();
    let drop_cursor = |e: &mut Entity, world: &mut World| {
        if let Some(s) = player(e).inventory.item_stack {
            drop_player_item(e, world, Some(s));
            // setItemStack(null): onItemStackChanged is empty (EntityPlayer.java:521-522)
            player_mut(e).inventory.item_stack = None;
        }
    };
    // src: EntityPlayer.java:526 (inventorySlots)
    drop_cursor(e, world);
    // src: EntityPlayer.java:527-529 (craftingInventory, never null)
    if let Some(mut c) = sp_mut(e).crafting_inventory.take() {
        c.on_craft_gui_closed(world, e);
        sp_mut(e).crafting_inventory = Some(c);
    } else {
        drop_cursor(e, world);
    }
}

/// `EntityPlayer.isEntityInsideOpaqueBlock()`: never while asleep.
// src: EntityPlayer.java:533-535
pub fn is_entity_inside_opaque_block(e: &Entity, world: &mut World) -> bool {
    !player(e).sleeping && base::is_entity_inside_opaque_block(e, world)
}

// ---- sleeping ------------------------------------------------------------------------

/// `sleepInBedAt(x, y, z)`: in the overworld, at night, within 3 blocks
/// (2 vertically) and not already asleep: the 0.2 box lies on the bed
/// (shifted toward its head by the bed's direction), `sleeping`, timer 0,
/// `bedChunkCoordinates`, no motion, and the world's sleeping flag is
/// updated. The result is also kept in
/// [`PlayerSpData::last_sleep_status`] for the harness.
// src: EntityPlayer.java:537-596
pub fn sleep_in_bed_at(e: &mut Entity, world: &mut World, x: i32, y: i32, z: i32) -> EnumStatus {
    let s = sleep_in_bed_at_body(e, world, x, y, z);
    sp_mut(e).last_sleep_status = Some(s);
    s
}

fn sleep_in_bed_at_body(
    e: &mut Entity,
    world: &mut World,
    var1: i32,
    var2: i32,
    var3: i32,
) -> EnumStatus {
    if !world.multiplayer_world {
        if player(e).is_player_sleeping() || !dispatch::is_entity_alive(e) {
            return EnumStatus::OtherProblem;
        }
        if world.world_provider.is_nether {
            return EnumStatus::NotPossibleHere;
        }
        if world.is_daytime() {
            return EnumStatus::NotPossibleNow;
        }
        if !((e.base.pos_x - var1 as f64).abs() <= 3.0
            && (e.base.pos_y - var2 as f64).abs() <= 2.0
            && (e.base.pos_z - var3 as f64).abs() <= 3.0)
        {
            return EnumStatus::TooFarAway;
        }
    }
    e.base.set_size(0.2, 0.2);
    e.base.y_offset = 0.2;
    if world.block_exists(var1, var2, var3) {
        let var4 = world.get_block_metadata(var1, var2, var3);
        let var5 = bed::get_direction_from_metadata(var4);
        let mut var6 = 0.5f32;
        let mut var7 = 0.5f32;
        match var5 {
            0 => var7 = 0.9,
            1 => var6 = 0.1,
            2 => var7 = 0.1,
            3 => var6 = 0.9,
            _ => {}
        }
        set_render_offset_for_sleep(player_mut(e), var5);
        e.base.set_position(
            (var1 as f32 + var6) as f64,
            (var2 as f32 + 0.9375) as f64,
            (var3 as f32 + var7) as f64,
        );
    } else {
        e.base.set_position(
            (var1 as f32 + 0.5) as f64,
            (var2 as f32 + 0.9375) as f64,
            (var3 as f32 + 0.5) as f64,
        );
    }
    let p = player_mut(e);
    p.sleeping = true;
    p.sleep_timer = 0;
    p.bed_chunk_coordinates = Some(ChunkCoordinates::new(var1, var2, var3));
    e.base.motion_x = 0.0;
    e.base.motion_z = 0.0;
    e.base.motion_y = 0.0;
    if !world.multiplayer_world {
        update_all_players_sleeping_flag(world, e);
    }
    EnumStatus::Ok
}

/// `setRenderOffsetForSleep(direction)` (private, render).
// src: EntityPlayer.java:598-615
fn set_render_offset_for_sleep(p: &mut PlayerData, var1: i32) {
    p.render_offset_x = 0.0;
    p.render_offset_z = 0.0;
    match var1 {
        0 => p.render_offset_z = -1.8,
        1 => p.render_offset_x = 1.8,
        2 => p.render_offset_z = 1.8,
        3 => p.render_offset_x = -1.8,
        _ => {}
    }
}

/// `World.updateAllPlayersSleepingFlag()` with the updating player `e` in
/// hand: the world's version cannot see a player taken out of its slot
/// (module docs of [`crate::entity`]), and the players that call it from
/// `sleepInBedAt` / `wakeUpPlayer` are exactly that player.
// src: World.java:2381-2393
fn update_all_players_sleeping_flag(world: &mut World, e: &Entity) {
    let mut flag = !world.player_entities.is_empty();
    for &pid in &world.player_entities {
        let sleeping = if pid == e.base.entity_id {
            Some(player(e).is_player_sleeping())
        } else {
            world
                .entity(pid)
                .and_then(|o| o.kind.player())
                .map(|p| p.is_player_sleeping())
        };
        match sleeping {
            Some(true) => {}
            Some(false) => {
                flag = false;
                break;
            }
            None => {
                world.note_unimplemented(
                    "World.updateAllPlayersSleepingFlag over another taken-out player",
                );
                flag = false;
                break;
            }
        }
    }
    world.all_players_sleeping = flag;
}

/// `wakeUpPlayer(resetTimer, updateFlag, setSpawn)`: standing size and
/// height; if the bed is still there, its occupied bit is cleared and the
/// player is put on the nearest free spot around it (above it if there is
/// none); `sleeping` off; the world's sleeping flag (`updateFlag`); the
/// timer to 0 (`resetTimer`) or 100; the bed as spawn point (`setSpawn`).
// src: EntityPlayer.java:617-647
pub fn wake_up_player(e: &mut Entity, world: &mut World, var1: bool, var2: bool, var3: bool) {
    e.base.set_size(0.6, 1.8);
    reset_height(e);
    let var4 = player(e).bed_chunk_coordinates;
    if let Some(c) = var4
        && world.get_block_id(c.pos_x, c.pos_y, c.pos_z) == BED
    {
        bed::set_bed_occupied(world, c.pos_x, c.pos_y, c.pos_z, false);
        let var5 = bed::get_nearest_empty_chunk_coordinates(world, c.pos_x, c.pos_y, c.pos_z, 0)
            .unwrap_or(ChunkCoordinates::new(c.pos_x, c.pos_y + 1, c.pos_z));
        let y_offset = e.base.y_offset;
        e.base.set_position(
            (var5.pos_x as f32 + 0.5) as f64,
            (var5.pos_y as f32 + y_offset + 0.1) as f64,
            (var5.pos_z as f32 + 0.5) as f64,
        );
    }
    player_mut(e).sleeping = false;
    if !world.multiplayer_world && var2 {
        update_all_players_sleeping_flag(world, e);
    }
    let p = player_mut(e);
    p.sleep_timer = if var1 { 0 } else { 100 };
    if var3 {
        let c = p.bed_chunk_coordinates;
        p.set_player_spawn_coordinate(c);
    }
}

/// `isInBed()` (private): the bed block is still at `bedChunkCoordinates`.
/// Only called while sleeping, when the coordinates are set (Java would
/// throw on null).
// src: EntityPlayer.java:649-651
fn is_in_bed(e: &Entity, world: &mut World) -> bool {
    let c = player(e)
        .bed_chunk_coordinates
        .expect("isInBed with a null bedChunkCoordinates (Java NullPointerException)");
    world.get_block_id(c.pos_x, c.pos_y, c.pos_z) == BED
}

/// `func_25060_a(world, bed)` (static; later `verifyRespawnCoordinates`):
/// loads the four chunks around the bed, then the free spot next to it, or
/// `None` when the bed is gone or boxed in.
// src: EntityPlayer.java:653-665
pub fn verify_respawn_coordinates(
    world: &mut World,
    var1: ChunkCoordinates,
) -> Option<ChunkCoordinates> {
    world.prepare_chunk((var1.pos_x - 3) >> 4, (var1.pos_z - 3) >> 4);
    world.prepare_chunk((var1.pos_x + 3) >> 4, (var1.pos_z - 3) >> 4);
    world.prepare_chunk((var1.pos_x - 3) >> 4, (var1.pos_z + 3) >> 4);
    world.prepare_chunk((var1.pos_x + 3) >> 4, (var1.pos_z + 3) >> 4);
    if world.get_block_id(var1.pos_x, var1.pos_y, var1.pos_z) != BED {
        None
    } else {
        bed::get_nearest_empty_chunk_coordinates(world, var1.pos_x, var1.pos_y, var1.pos_z, 0)
    }
}

/// `getBedOrientationInDegrees()` (render: the sleeping body's yaw).
// src: EntityPlayer.java:667-684
pub fn get_bed_orientation_in_degrees(e: &Entity, world: &mut World) -> f32 {
    if let Some(c) = player(e).bed_chunk_coordinates {
        let var1 = world.get_block_metadata(c.pos_x, c.pos_y, c.pos_z);
        match bed::get_direction_from_metadata(var1) {
            0 => return 90.0,
            1 => return 0.0,
            2 => return 270.0,
            3 => return 180.0,
            _ => {}
        }
    }
    0.0
}

// ---- statistics, movement -------------------------------------------------------------

/// `addStat(stat, amount)`. `EntityPlayer`'s body is empty, and that is
/// what the oracle's `HarnessPlayer` runs; the port's player is the local
/// `EntityPlayerSP`, whose override writes `mc.statFileWriter`
/// ([`PlayerData::stat_file_writer`],
/// [`crate::stats::StatFileWriter::entity_player_sp_add_stat`]: null
/// stats are ignored, an achievement needs its parent). Stats never reach a
/// golden trace.
// src: EntityPlayer.java:718-719, EntityPlayerSP.java:176-192
pub fn add_stat(e: &mut Entity, stat: Option<&'static StatBase>, amount: i32) {
    if let Some(p) = e.kind.player_mut() {
        p.stat_file_writer.entity_player_sp_add_stat(stat, amount);
    }
}

/// `triggerAchievement(stat)`: `addStat(stat, 1)`.
// src: EntityPlayer.java:714-716
pub fn trigger_achievement(e: &mut Entity, stat: Option<&'static StatBase>) {
    add_stat(e, stat, 1);
}

/// `EntityPlayer.jump()`: `EntityLiving`'s, then the jump stat.
// src: EntityPlayer.java:721-724
pub fn jump(e: &mut Entity) {
    living::jump(e);
    add_stat(e, stat(sl::JUMP_STAT), 1);
}

/// `EntityPlayer.moveEntityWithHeading(strafe, forward)`: `EntityLiving`'s,
/// then the distance stats of the move.
// src: EntityPlayer.java:726-732
pub fn move_entity_with_heading(e: &mut Entity, world: &mut World, var1: f32, var2: f32) {
    let (var3, var5, var7) = (e.base.pos_x, e.base.pos_y, e.base.pos_z);
    living::move_entity_with_heading(e, world, var1, var2);
    let (dx, dy, dz) = (
        e.base.pos_x - var3,
        e.base.pos_y - var5,
        e.base.pos_z - var7,
    );
    add_movement_stat(e, world, dx, dy, dz);
}

/// `addMovementStat(dx, dy, dz)` (private): dove, swum, climbed, walked or
/// flown distance in centimetres. The queries only read blocks; the stats
/// go to [`add_stat`].
// src: EntityPlayer.java:734-764
fn add_movement_stat(e: &mut Entity, world: &mut World, var1: f64, var3: f64, var5: f64) {
    if e.base.riding_entity.is_some() {
        return;
    }
    if base::is_inside_of_material(e, world, Material::Water) {
        let var7 =
            java_round_f32(math::sqrt_double(var1 * var1 + var3 * var3 + var5 * var5) * 100.0);
        if var7 > 0 {
            add_stat(e, stat(sl::DISTANCE_DOVE_STAT), var7);
        }
    } else if dispatch::is_in_water(e, world) {
        let var7 = java_round_f32(math::sqrt_double(var1 * var1 + var5 * var5) * 100.0);
        if var7 > 0 {
            add_stat(e, stat(sl::DISTANCE_SWUM_STAT), var7);
        }
    } else if dispatch::is_on_ladder(e, world) {
        if var3 > 0.0 {
            add_stat(
                e,
                stat(sl::DISTANCE_CLIMBED_STAT),
                java_round_f64_to_int(var3 * 100.0),
            );
        }
    } else if e.base.on_ground {
        let var7 = java_round_f32(math::sqrt_double(var1 * var1 + var5 * var5) * 100.0);
        if var7 > 0 {
            add_stat(e, stat(sl::DISTANCE_WALKED_STAT), var7);
        }
    } else {
        let var7 = java_round_f32(math::sqrt_double(var1 * var1 + var5 * var5) * 100.0);
        if var7 > 25 {
            add_stat(e, stat(sl::DISTANCE_FLOWN_STAT), var7);
        }
    }
}

/// `addMountedMovementStat(dx, dy, dz)` (private): minecart, boat or pig
/// distance; a minecart ride remembers its start block for the `onARail`
/// achievement (1000 blocks away).
// src: EntityPlayer.java:766-785
fn add_mounted_movement_stat(e: &mut Entity, world: &mut World, var1: f64, var3: f64, var5: f64) {
    let Some(mount) = e.base.riding_entity else {
        return;
    };
    let var7 = java_round_f32(math::sqrt_double(var1 * var1 + var3 * var3 + var5 * var5) * 100.0);
    if var7 <= 0 {
        return;
    }
    let kind = match world.entity(mount) {
        Some(m) => m.kind.java_class(),
        None => {
            world.note_unimplemented(
                "EntityPlayer.addMountedMovementStat: the mount is not in its slot",
            );
            return;
        }
    };
    match kind {
        "EntityMinecart" => {
            add_stat(e, stat(sl::DISTANCE_BY_MINECART_STAT), var7);
            let here = (
                floor_double(e.base.pos_x),
                floor_double(e.base.pos_y),
                floor_double(e.base.pos_z),
            );
            let p = player_mut(e);
            match p.start_minecart_riding_coordinate {
                None => {
                    p.start_minecart_riding_coordinate =
                        Some(ChunkCoordinates::new(here.0, here.1, here.2));
                }
                Some(c) => {
                    // ChunkCoordinates.getSqDistanceTo: despite the name, the
                    // distance (Math.sqrt) (ChunkCoordinates.java:44-49).
                    let var4 = c.pos_x.wrapping_sub(here.0);
                    let var5 = c.pos_y.wrapping_sub(here.1);
                    let var6 = c.pos_z.wrapping_sub(here.2);
                    let d = (var4
                        .wrapping_mul(var4)
                        .wrapping_add(var5.wrapping_mul(var5))
                        .wrapping_add(var6.wrapping_mul(var6)) as f64)
                        .sqrt();
                    if d >= 1000.0 {
                        add_stat(e, stat(al::ON_A_RAIL), 1);
                    }
                }
            }
        }
        "EntityBoat" => add_stat(e, stat(sl::DISTANCE_BY_BOAT_STAT), var7),
        "EntityPig" => add_stat(e, stat(sl::DISTANCE_BY_PIG_STAT), var7),
        _ => {}
    }
}

/// `EntityPlayer.fall(distance)`: the fallen-distance stat from 2 blocks,
/// then `EntityLiving`'s (damage).
// src: EntityPlayer.java:787-793
pub fn fall(e: &mut Entity, world: &mut World, var1: f32) {
    if var1 >= 2.0 {
        add_stat(
            e,
            stat(sl::DISTANCE_FALLEN_STAT),
            java_round_f64_to_int(var1 as f64 * 100.0),
        );
    }
    living::fall(e, world, var1);
}

/// `EntityPlayer.onKillEntity(victim)`: the `killEnemy` achievement for a
/// mob. `victim_is_mob` is `victim instanceof EntityMob`.
// src: EntityPlayer.java:795-800
pub fn on_kill_entity(e: &mut Entity, victim_is_mob: bool) {
    if victim_is_mob {
        add_stat(e, stat(al::KILL_ENEMY), 1);
    }
}

/// `EntityPlayer.setInPortal()`: a portal block touched; the portal timer
/// (EntityPlayerSP's `onLivingUpdate`, [`sp_on_living_update`]) counts the
/// travel, which is the client's `Minecraft.usePortal`.
// src: EntityPlayer.java:811-817
pub fn set_in_portal(e: &mut Entity) {
    let p = player_mut(e);
    if p.time_until_portal > 0 {
        p.time_until_portal = 10;
    } else {
        p.in_portal = true;
    }
}

// ---- EntityPlayerSP ------------------------------------------------------------------

/// `EntityPlayerSP.updatePlayerActionState()`: `EntityPlayer`'s (the swing),
/// then the movement input becomes the living entity's steering.
// src: EntityPlayerSP.java:27-32
pub fn sp_update_player_action_state(e: &mut Entity) {
    update_player_action_state(e);
    let mi = sp(e).movement_input;
    let l = lv_mut(e);
    l.move_strafing = mi.move_strafe;
    l.move_forward = mi.move_forward;
    l.is_jumping = mi.jump;
}

/// `EntityPlayerSP.isSneaking()`: the sneak input, unless asleep. Read by
/// `Entity.moveEntity`'s edge guard and the ladder hold.
// src: EntityPlayerSP.java:144-146
pub fn sp_is_sneaking(e: &Entity) -> bool {
    sp(e).movement_input.sneak && !player(e).sleeping
}

/// `EntityPlayerSP.onLivingUpdate()`: with
/// [`PlayerSpData::client_additions`] (module docs), the inventory hint,
/// the portal timer (two `rand` draws for the portal sounds' pitch, both
/// sounds and `Minecraft.usePortal` raised as [`ClientRequest`]s), the
/// sneak `ySize` dip and `pushOutOfBlocks` at the four corners of the box;
/// then `EntityPlayer`'s. When the timer fires `usePortal` this returns
/// right after raising [`ClientRequest::UsePortal`]; the client finishes
/// the update on the new world with [`sp_on_living_update_after_portal`]
/// (module docs, "Calls into `Minecraft`").
// src: EntityPlayerSP.java:34-88
pub fn sp_on_living_update(e: &mut Entity, world: &mut World) {
    if !sp(e).client_additions {
        on_living_update(e, world);
        return;
    }
    // src: EntityPlayerSP.java:35-37
    if let Some(open) = stat(al::OPEN_INVENTORY)
        && !player(e).stat_file_writer.has_achievement_unlocked(open)
    {
        world.client_requests.push(ClientRequest::AchievementHint {
            stat_id: al::OPEN_INVENTORY,
        });
    }
    {
        let p = player_mut(e);
        p.prev_time_in_portal = p.time_in_portal;
    }
    if player(e).in_portal {
        if !world.multiplayer_world && e.base.riding_entity.is_some() {
            base::mount_entity(e, world, None);
        }
        // src: EntityPlayerSP.java:45-47: `mc.currentScreen` closed. The
        // client does it before the tick's updateEntities (mc-client
        // game.rs), where `inPortal` has the value read here.
        if player(e).time_in_portal == 0.0 {
            // src: EntityPlayerSP.java:50
            let pitch = e.base.rand.next_float() * 0.4 + 0.8;
            world.client_requests.push(ClientRequest::PlaySoundFx {
                name: "portal.trigger",
                volume: 1.0,
                pitch,
            });
        }
        let p = player_mut(e);
        p.time_in_portal += 0.0125;
        if p.time_in_portal >= 1.0 {
            p.time_in_portal = 1.0;
            if !world.multiplayer_world {
                p.time_until_portal = 10;
                // src: EntityPlayerSP.java:58
                let pitch = e.base.rand.next_float() * 0.4 + 0.8;
                world.client_requests.push(ClientRequest::PlaySoundFx {
                    name: "portal.travel",
                    volume: 1.0,
                    pitch,
                });
                // src: EntityPlayerSP.java:59: mc.usePortal(). Its head runs
                // here, inside the old world's updateEntities; the client
                // runs the rest, then the update goes on on the new world
                // (sp_on_living_update_after_portal).
                let (x, z) = use_portal_head(e, world);
                world
                    .client_requests
                    .push(ClientRequest::UsePortal { x, z });
                return;
            }
        }
        // src: EntityPlayerSP.java:63
        player_mut(e).in_portal = false;
    } else {
        let p = player_mut(e);
        if p.time_in_portal > 0.0 {
            p.time_in_portal -= 0.05;
        }
        if p.time_in_portal < 0.0 {
            p.time_in_portal = 0.0;
        }
    }
    sp_on_living_update_tail(e, world);
}

/// The head of `Minecraft.usePortal()`, up to the old world's
/// `updateEntityWithOptionalForce`: the dimension toggled between -1 and 0,
/// `theWorld.setEntityDead(thePlayer)` (out of `playerEntities`, the
/// crafting grid dropped, World.java:934-946) and `isDead` set back, then
/// x and z divided by 8 into the nether or multiplied by 8 out of it and
/// `setLocationAndAngles` there. Returns `(var1, var3)`, the scaled x and z
/// the rest of `usePortal` puts the player at again in the new world
/// (Minecraft.java:1253). Called from [`sp_on_living_update`], where
/// vanilla calls `usePortal`, and by the client for a `usePortal` of its
/// own (`respawn` in the nether), with the player in hand and `world` the
/// world it is in.
// src: Minecraft.java:1215-1230, :1238-1241
pub fn use_portal_head(e: &mut Entity, world: &mut World) -> (f64, f64) {
    let p = player_mut(e);
    p.dimension = if p.dimension == -1 { 0 } else { -1 };
    let dim = p.dimension;
    world.set_entity_dead(e);
    e.base.is_dead = false;
    let mut var1 = e.base.pos_x;
    let mut var3 = e.base.pos_z;
    let var5 = 8.0f64;
    if dim == -1 {
        var1 /= var5;
        var3 /= var5;
    } else {
        var1 *= var5;
        var3 *= var5;
    }
    // vanilla bug: setLocationAndAngles adds yOffset to the y it is given
    // (Entity.java:671), and usePortal passes posY, which already includes
    // it: the player rises 1.62 here (and again at Minecraft.java:1253)
    // until the teleporter places it.
    let (y, yaw, pitch) = (e.base.pos_y, e.base.rotation_yaw, e.base.rotation_pitch);
    e.base.set_location_and_angles(var1, y, var3, yaw, pitch);
    (var1, var3)
}

/// The rest of `EntityPlayerSP.onLivingUpdate()` after `mc.usePortal()`
/// returned (EntityPlayerSP.java:63, :74-87): `inPortal = false`, then the
/// portal cooldown, the sneak `ySize`, `pushOutOfBlocks` and
/// `EntityPlayer.onLivingUpdate`, on the world the player is in now. The
/// client calls it once [`ClientRequest::UsePortal`] has moved the player
/// there, with the player taken out of its slot (module docs). Nothing
/// without [`PlayerSpData::client_additions`].
// src: EntityPlayerSP.java:63-88
pub fn sp_on_living_update_after_portal(e: &mut Entity, world: &mut World) {
    if !sp(e).client_additions {
        return;
    }
    player_mut(e).in_portal = false; // src: EntityPlayerSP.java:63
    sp_on_living_update_tail(e, world);
}

/// `EntityPlayerSP.onLivingUpdate()` from the portal cooldown on.
// src: EntityPlayerSP.java:74-87
fn sp_on_living_update_tail(e: &mut Entity, world: &mut World) {
    let p = player_mut(e);
    if p.time_until_portal > 0 {
        p.time_until_portal -= 1;
    }
    // movementInput.updatePlayerMoveState(this): the client's input
    // source fills movement_input (MovementInput::from_keys).
    if sp(e).movement_input.sneak && e.base.y_size < 0.2 {
        e.base.y_size = 0.2;
    }
    let w = e.base.width as f64 * 0.35;
    let (x, z) = (e.base.pos_x, e.base.pos_z);
    let y = e.base.bounding_box.min_y + 0.5;
    dispatch::push_out_of_blocks(e, world, x - w, y, z + w);
    let (x, z) = (e.base.pos_x, e.base.pos_z);
    let y = e.base.bounding_box.min_y + 0.5;
    dispatch::push_out_of_blocks(e, world, x - w, y, z - w);
    let (x, z) = (e.base.pos_x, e.base.pos_z);
    let y = e.base.bounding_box.min_y + 0.5;
    dispatch::push_out_of_blocks(e, world, x + w, y, z - w);
    let (x, z) = (e.base.pos_x, e.base.pos_z);
    let y = e.base.bounding_box.min_y + 0.5;
    dispatch::push_out_of_blocks(e, world, x + w, y, z + w);
    on_living_update(e, world);
}

/// `EntityPlayerSP.pushOutOfBlocks(x, y, z)`: inside a normal cube (at the
/// feet or the head), set a 0.1 motion toward the nearest open side
/// (`-x`, `+x`, `-z`, `+z`, in that preference on ties). Always returns
/// false.
// src: EntityPlayerSP.java:194-250
pub fn sp_push_out_of_blocks(
    e: &mut Entity,
    world: &mut World,
    var1: f64,
    var3: f64,
    var5: f64,
) -> bool {
    let var7 = floor_double(var1);
    let var8 = floor_double(var3);
    let var9 = floor_double(var5);
    let var10 = var1 - var7 as f64;
    let var12 = var5 - var9 as f64;
    // isBlockTranslucent: World.isBlockNormalCube (EntityPlayerSP.java:194-196)
    let mut t = |x: i32, y: i32, z: i32| world.is_block_normal_cube(x, y, z);
    if t(var7, var8, var9) || t(var7, var8 + 1, var9) {
        let var14 = !t(var7 - 1, var8, var9) && !t(var7 - 1, var8 + 1, var9);
        let var15 = !t(var7 + 1, var8, var9) && !t(var7 + 1, var8 + 1, var9);
        let var16 = !t(var7, var8, var9 - 1) && !t(var7, var8 + 1, var9 - 1);
        let var17 = !t(var7, var8, var9 + 1) && !t(var7, var8 + 1, var9 + 1);
        let mut var18: i8 = -1;
        let mut var19 = 9999.0;
        if var14 && var10 < var19 {
            var19 = var10;
            var18 = 0;
        }
        if var15 && 1.0 - var10 < var19 {
            var19 = 1.0 - var10;
            var18 = 1;
        }
        if var16 && var12 < var19 {
            var19 = var12;
            var18 = 4;
        }
        if var17 && 1.0 - var12 < var19 {
            var18 = 5;
        }
        let var21 = 0.1f32;
        if var18 == 0 {
            e.base.motion_x = -var21 as f64;
        }
        if var18 == 1 {
            e.base.motion_x = var21 as f64;
        }
        if var18 == 4 {
            e.base.motion_z = -var21 as f64;
        }
        if var18 == 5 {
            e.base.motion_z = var21 as f64;
        }
    }
    false
}

// ---- the held stack's item calls (ItemStack methods on a main slot) -------------------

/// Runs `f` on the stack in main slot `slot`, moved out of the slot for the
/// call so that it and the player can both be borrowed mutably, and puts it
/// back: what the item does to it lands in the slot, as on Java's shared
/// object (nothing on these paths stores another stack into the slot).
/// Nothing when the slot is empty.
fn with_main_stack(e: &mut Entity, slot: usize, f: impl FnOnce(&mut ItemStack, &mut Entity)) {
    let Some(mut s) = player_mut(e).inventory.main_inventory[slot].take() else {
        return;
    };
    f(&mut s, e);
    player_mut(e).inventory.main_inventory[slot] = Some(s);
}

/// `ItemStack.hitEntity(target, player)` for the held stack in main slot
/// `slot` ([`ItemStack::hit_entity`]: `Item.hitEntity`, the tools' wear,
/// the saddle on a pig).
// src: ItemStack.java:143-149
pub fn item_stack_hit_entity(e: &mut Entity, world: &mut World, slot: usize, target: &mut Entity) {
    with_main_stack(e, slot, |s, e| s.hit_entity(target, e, world));
}

/// `ItemStack.useItemOnEntity(target)` for the held stack in main slot
/// `slot` ([`ItemStack::use_item_on_entity`]: `Item.saddleEntity`, the
/// saddle on a pig, a dye on a sheep).
// src: ItemStack.java:170-172
pub fn item_stack_use_item_on_entity(
    e: &mut Entity,
    world: &mut World,
    slot: usize,
    target: &mut Entity,
) {
    with_main_stack(e, slot, |s, _| s.use_item_on_entity(target, world));
}

/// `ItemStack.onDestroyBlock(blockId, x, y, z, player)` for the held stack
/// in main slot `slot` ([`ItemStack::on_destroy_block`]:
/// `Item.onBlockDestroyed`, the tools' wear).
// src: ItemStack.java:151-157
#[allow(clippy::too_many_arguments)]
pub fn item_stack_on_destroy_block(
    e: &mut Entity,
    world: &mut World,
    slot: usize,
    var1: i32,
    x: i32,
    y: i32,
    z: i32,
) {
    with_main_stack(e, slot, |s, e| s.on_destroy_block(var1, x, y, z, e, world));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_round_matches_math_round() {
        assert_eq!(java_round_f32(0.5), 1);
        assert_eq!(java_round_f32(-0.5), 0);
        assert_eq!(java_round_f32(2.4999998), 2);
        assert_eq!(java_round_f32(f32::NAN), 0);
        assert_eq!(java_round_f64_to_int(-1.5), -1);
        assert_eq!(java_round_f64_to_int(149.99999), 150);
    }

    #[test]
    fn sneak_scales_the_input_through_double() {
        let m = MovementInput::from_keys(1.0, -1.0, true, true);
        assert_eq!(m.move_forward, (1.0f64 * 0.3) as f32);
        assert_eq!(m.move_strafe, (-0.3f64) as f32);
        assert!(m.jump && m.sneak);
        let m = MovementInput::from_keys(1.0, 0.0, false, false);
        assert_eq!(m.move_forward, 1.0);
    }

    #[test]
    fn enum_status_names() {
        assert_eq!(EnumStatus::NotPossibleNow.name(), "NOT_POSSIBLE_NOW");
        assert_eq!(EnumStatus::Ok.name(), "OK");
    }
}
