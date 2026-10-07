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
//! `onLivingUpdate` body (:34-88: the portal timer and `Minecraft.usePortal`,
//! the sneak `ySize`, `pushOutOfBlocks` at the four corners) runs only with
//! [`PlayerSpData::client_additions`] set; the golden driver leaves it off,
//! which is `HarnessPlayer` (tools/oracle/README.md, "`HarnessPlayer`").
//! Out of scope, client: the achievement toast and the stat file
//! (`mc.statFileWriter`, `addStat`: WP-2.7; `EntityPlayer.addStat` is
//! empty, [`add_stat`]), the GUI calls (`displayGUI*`, `closeScreen`'s
//! `displayGuiScreen`), `onItemPickup`'s particle, `addChatMessage`,
//! `setHealth` (multiplayer packets), the `MouseFilter`s.
//!
//! # Audit of EntityPlayer.java
//!
//! | EntityPlayer.java | Method | Port |
//! |---|---|---|
//! | 7-38 | fields | [`PlayerData`] (not `inventorySlots`/`craftingInventory`: the `ContainerPlayer`, WP-2.5b; not `playerCloakUrl`, D2) |
//! | 40-57 | constructor, `entityInit` | [`new_entity_player_sp`] |
//! | 59-125 | `onUpdate` | [`on_update`] |
//! | 127-133 | `isMovementBlocked`, `closeScreen` | [`is_movement_blocked`]; `closeScreen` only resets `craftingInventory` (WP-2.5b) |
//! | 135-138 | `updateCloak` | online cloak URL (D2), not ported |
//! | 140-156 | `updateRidden`, `preparePlayerToSpawn` | [`update_ridden`], [`prepare_player_to_spawn`] |
//! | 158-212 | `updatePlayerActionState`, `onLivingUpdate`, `collideWithPlayer` | [`update_player_action_state`], [`on_living_update`] |
//! | 214-247 | `getScore`, `onDeath`, `addToPlayerScore` | [`PlayerData::get_score`], [`on_death`], [`add_to_player_score`] |
//! | 249-289 | `dropCurrentItem`, `dropPlayerItem`, `dropPlayerItemWithRandomChoice`, `joinEntityItemWithWorld` | [`drop_current_item`], [`drop_player_item`], [`drop_player_item_with_random_choice`], [`join_entity_item_with_world`] |
//! | 291-306 | `getCurrentPlayerStrVsBlock`, `canHarvestBlock` | [`get_current_player_str_vs_block`], [`can_harvest_block`] |
//! | 308-338 | NBT | [`super::nbt`] (the sleeping player's wake-up: [`read_entity_from_nbt_tail`]) |
//! | 340-355 | `displayGUIChest`, `displayWorkbenchGUI`, `onItemPickup` (empty), `getEyeHeight`, `resetHeight` | GUI: client; [`EYE_HEIGHT`], [`reset_height`] |
//! | 357-437 | `attackEntityFrom`, `isPVPEnabled`, `alertWolves` | [`attack_entity_from`], [`alert_wolves`] |
//! | 439-446 | `damageEntity` | [`damage_entity`] |
//! | 448-486 | `displayGUI*` (empty), `useCurrentItemOnEntity`, `getCurrentEquippedItem`, `destroyCurrentEquippedItem`, `getYOffset`, `swingItem` | [`use_current_item_on_entity`]; [`get_current_equipped_item`]; [`destroy_current_equipped_item`]; [`get_y_offset`]; [`swing_item`] |
//! | 488-514 | `attackTargetEntityWithCurrentItem` | [`attack_target_entity_with_current_item`] |
//! | 516-535 | `respawnPlayer` (empty), `func_6420_o` (abstract), `onItemStackChanged` (empty), `setEntityDead`, `isEntityInsideOpaqueBlock` | [`set_entity_dead`], [`is_entity_inside_opaque_block`] |
//! | 537-651 | `sleepInBedAt`, `setRenderOffsetForSleep`, `wakeUpPlayer`, `isInBed` | [`sleep_in_bed_at`], [`wake_up_player`] |
//! | 653-665 | `func_25060_a` (static) | [`verify_respawn_coordinates`] |
//! | 667-712 | `getBedOrientationInDegrees`, sleep and spawn getters | [`get_bed_orientation_in_degrees`], [`PlayerData`] |
//! | 714-724 | `triggerAchievement`, `addStat` (empty), `jump` | [`add_stat`], [`jump`] |
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
use crate::world::World;
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
/// `mc` and the three `MouseFilter`s (client).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlayerSpData {
    /// `movementInput` (EntityPlayerSP.java:6).
    pub movement_input: MovementInput,
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
    // inventorySlots / craftingInventory: ContainerPlayer, WP-2.5b.
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
    // src: EntityPlayer.java:81-84: `craftingInventory.isUsableByPlayer(this)`
    // is ContainerPlayer's `true` (ContainerPlayer.java:61-63) while no GUI
    // replaced it (WP-2.5b), so closeScreen never runs here.

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
    add_stat(e, "StatList.minutesPlayedStat", 1);
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
    add_stat(e, "StatList.deathsStat", 1);
}

/// `EntityPlayer.addToPlayerScore(victim, score)`.
// src: EntityPlayer.java:239-247
pub fn add_to_player_score(e: &mut Entity, victim: &Entity, var2: i32) {
    let p = player_mut(e);
    p.score = p.score.wrapping_add(var2);
    if victim.kind.is_player() {
        add_stat(e, "StatList.playerKillsStat", 1);
    } else {
        add_stat(e, "StatList.mobKillsStat", 1);
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
    add_stat(e, "StatList.dropStat", 1);
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
            Some(oid) => match world.entity(oid) {
                Some(o) => {
                    if o.kind.living().is_some() {
                        let kind = o.kind.clone();
                        alert_wolves(e, world, &kind, false);
                    }
                }
                None => world.note_unimplemented(
                    "EntityPlayer.attackEntityFrom: the arrow's owner is not in loadedEntityList (Java still holds it)",
                ),
            },
            None => {
                if a.kind.living().is_some() {
                    alert_wolves(e, world, &a.kind, false);
                }
            }
        }
    }
    add_stat(e, "StatList.damageTakenStat", var2);
    living::attack_entity_from(e, world, var1, var2)
}

/// `isPVPEnabled()`: false in singleplayer.
// src: EntityPlayer.java:398-400
pub fn is_pvp_enabled() -> bool {
    false
}

/// `alertWolves(attacker, onlySitting)`: every tamed wolf of this player
/// within 16 blocks (4 vertically) without a target stands up and targets
/// the attacker, unless it is a creeper, a ghast, the player's own wolf or
/// a player.
///
/// `EntityWolf` is not transcribed (WP-2.3) and cannot be constructed yet,
/// so no wolf is ever in a port world: a wolf attacker or a wolf found by
/// the search records `EntityWolf.isWolfTamed` and is taken as untamed
/// (skipped), as any untamed wolf is in Java.
// src: EntityPlayer.java:402-437
pub fn alert_wolves(e: &Entity, world: &mut World, var1: &EntityKindData, var2: bool) {
    let _ = var2;
    if is_instance_of(var1, "EntityCreeper") || is_instance_of(var1, "EntityGhast") {
        return;
    }
    if let EntityKindData::Wolf(..) = var1 {
        // src: EntityPlayer.java:404-409 (a tamed wolf of this player returns)
        world.note_unimplemented("EntityWolf.isWolfTamed (EntityPlayer.alertWolves)");
    }
    if !var1.is_player() || is_pvp_enabled() {
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
        let var7 = world.get_entities_within_aabb("EntityWolf", &bb, None);
        for _ in var7 {
            // src: EntityPlayer.java:427-433: isWolfTamed, getTarget,
            // getWolfOwner, isWolfSitting, setWolfSitting, setTarget (WP-2.3)
            world.note_unimplemented("EntityWolf.isWolfTamed (EntityPlayer.alertWolves)");
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
                alert_wolves(e, world, &var1.kind, true);
            }
            add_stat(e, "StatList.damageDealtStat", var2);
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
/// `craftingInventory` (the same `ContainerPlayer` without a GUI, so twice):
/// `Container.onCraftGuiClosed` drops the stack held by the cursor
/// (`inventory.itemStack`). `ContainerPlayer`'s crafting grid (dropped too,
/// ContainerPlayer.java:48-59) is WP-2.5b; it is empty without a GUI.
// src: EntityPlayer.java:524-531, Container.java:152-159
pub fn set_entity_dead(e: &mut Entity, world: &mut World) {
    e.base.set_entity_dead();
    for _ in 0..2 {
        if let Some(s) = player(e).inventory.item_stack {
            drop_player_item(e, world, Some(s));
            // setItemStack(null): onItemStackChanged is empty (EntityPlayer.java:521-522)
            player_mut(e).inventory.item_stack = None;
        }
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

/// `addStat(stat, amount)` / `triggerAchievement(stat)` (which is
/// `addStat(stat, 1)`, EntityPlayer.java:714-716). `EntityPlayer`'s body is
/// empty, and that is what the oracle's `HarnessPlayer` runs;
/// `EntityPlayerSP`'s writes the client's stat file and queues achievement
/// toasts (EntityPlayerSP.java:176-192, WP-2.7). `stat` names the
/// `StatList` / `AchievementList` field.
// src: EntityPlayer.java:718-719
pub fn add_stat(e: &Entity, stat: &'static str, amount: i32) {
    let _ = (e, stat, amount);
}

/// `EntityPlayer.jump()`: `EntityLiving`'s, then the jump stat.
// src: EntityPlayer.java:721-724
pub fn jump(e: &mut Entity) {
    living::jump(e);
    add_stat(e, "StatList.jumpStat", 1);
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
            add_stat(e, "StatList.distanceDoveStat", var7);
        }
    } else if dispatch::is_in_water(e, world) {
        let var7 = java_round_f32(math::sqrt_double(var1 * var1 + var5 * var5) * 100.0);
        if var7 > 0 {
            add_stat(e, "StatList.distanceSwumStat", var7);
        }
    } else if dispatch::is_on_ladder(e, world) {
        if var3 > 0.0 {
            add_stat(
                e,
                "StatList.distanceClimbedStat",
                java_round_f64_to_int(var3 * 100.0),
            );
        }
    } else if e.base.on_ground {
        let var7 = java_round_f32(math::sqrt_double(var1 * var1 + var5 * var5) * 100.0);
        if var7 > 0 {
            add_stat(e, "StatList.distanceWalkedStat", var7);
        }
    } else {
        let var7 = java_round_f32(math::sqrt_double(var1 * var1 + var5 * var5) * 100.0);
        if var7 > 25 {
            add_stat(e, "StatList.distanceFlownStat", var7);
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
            add_stat(e, "StatList.distanceByMinecartStat", var7);
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
                        add_stat(e, "AchievementList.onARail", 1);
                    }
                }
            }
        }
        "EntityBoat" => add_stat(e, "StatList.distanceByBoatStat", var7),
        "EntityPig" => add_stat(e, "StatList.distanceByPigStat", var7),
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
            "StatList.distanceFallenStat",
            java_round_f64_to_int(var1 as f64 * 100.0),
        );
    }
    living::fall(e, world, var1);
}

/// `EntityPlayer.onKillEntity(victim)`: the `killEnemy` achievement for a
/// mob.
// src: EntityPlayer.java:795-800
pub fn on_kill_entity(e: &mut Entity, victim: &Entity) {
    if is_instance_of(&victim.kind, "EntityMob") {
        add_stat(e, "AchievementList.killEnemy", 1);
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
/// [`PlayerSpData::client_additions`] (module docs), the portal timer
/// (two `rand` draws for the portal sounds' pitch, `Minecraft.usePortal`
/// recorded), the sneak `ySize` dip and `pushOutOfBlocks` at the four
/// corners of the box; then `EntityPlayer`'s.
// src: EntityPlayerSP.java:34-88
pub fn sp_on_living_update(e: &mut Entity, world: &mut World) {
    if sp(e).client_additions {
        // src: EntityPlayerSP.java:35-37: the openInventory achievement toast
        // (client).
        {
            let p = player_mut(e);
            p.prev_time_in_portal = p.time_in_portal;
        }
        if player(e).in_portal {
            if !world.multiplayer_world && e.base.riding_entity.is_some() {
                base::mount_entity(e, world, None);
            }
            // mc.currentScreen closed (client).
            if player(e).time_in_portal == 0.0 {
                // mc.sndManager.playSoundFX("portal.trigger", 1.0F, pitch): the
                // pitch's draw happens; the sound is the client's.
                let _pitch = e.base.rand.next_float() * 0.4 + 0.8;
            }
            let p = player_mut(e);
            p.time_in_portal += 0.0125;
            if p.time_in_portal >= 1.0 {
                p.time_in_portal = 1.0;
                if !world.multiplayer_world {
                    p.time_until_portal = 10;
                    let _pitch = e.base.rand.next_float() * 0.4 + 0.8; // "portal.travel"
                    world.note_unimplemented("Minecraft.usePortal");
                }
            }
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
    }
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
