//! `EntityMinecart` (EntityMinecart.java): the minecart, in its three types
//! (`minecartType` 0 plain, 1 chest with a 27-slot `IInventory`, 2 furnace
//! with `fuel` and the push vector `pushX`/`pushZ`).
//!
//! Transcribed: both constructors, `attackEntityFrom`, `setEntityDead` (the
//! cargo drop), `onUpdate` (the rail motion over [`MATRIX`], slopes, the
//! powered rail's boost and brake, the furnace push, the derailed motion,
//! the yaw and its reversal, the cart-to-cart pushes), `getPos`,
//! `getPosOffset`, `applyEntityCollision`, `interact`, the `IInventory`
//! methods, `canInteractWith`, the NBT bodies, and the data overrides
//! `canTriggerWalking`, `getCollisionBox`, `getBoundingBox`, `canBePushed`,
//! `getMountedYOffset`, `canBeCollidedWith` (in [`super::dispatch`]).
//! Transcribed as data for the client: `performHurtAnimation` (its
//! `System.out.println("Animating hurt")` is not ported),
//! `setPositionAndRotation2` and `setVelocity` (multiplayer interpolation;
//! the branch of `onUpdate` they feed is transcribed but unreachable in
//! single player), `getShadowSize` (0).
//!
//! `onUpdate` does not call `super.onUpdate()`: `ticksExisted` stays 0 and
//! there is no fire, water or void handling.
//!
//! # Vanilla quirks transcribed
//!
//! - `cargoItems` is `new ItemStack[36]` (EntityMinecart.java:28) while
//!   `getSizeInventory()` is 27; `readEntityFromNBT` of a chest cart
//!   reallocates it with 27 slots (:629). Kept as a `Vec` of that length.
//! - A breaking chest cart drops its cargo twice over: `setEntityDead()`
//!   (:136-162) empties every stack to size 0 (the size-0 stacks stay in
//!   their slots), then `attackEntityFrom` walks the slots again (:89-111)
//!   and draws its three `rand.nextFloat()` offsets for every non-null slot
//!   before its `while` finds nothing left to drop.
//! - `applyEntityCollision` between two carts tests
//!   `(dx * other.motionZ + dz * other.prevPosX)^2 > 5` (:676-680; `// vanilla bug:`
//!   a position where a motion belongs), so far from x = 0 carts pass
//!   through each other unless they meet on the same z.
//!
//! # The chest GUI
//!
//! `interact` of a chest cart opens its GUI (`displayGUIChest`, a client
//! screen and `ContainerChest`): [`super::player::display_gui`] with
//! [`PlayerGui::MinecartChest`]. Only EntityPlayerSP opens the GuiChest
//! (EntityPlayerSP.java:117-119); EntityPlayer's version is empty
//! (EntityPlayer.java:340-341), so the oracle's HarnessPlayer (an
//! EntityPlayer) does nothing here, as the Rust side does for a player
//! without `client_additions`. The request names the cart by id; the
//! client builds `new ContainerChest(player.inventory, cart)` over
//! [`crate::inventory::InventoryRef::Entity`] (WP-4.6), whose slots reach
//! the [`MinecartData`] `IInventory` methods below.

use mc_core::Vec3D;
use mc_core::math::{self, floor_double, jmath};
use mc_nbt::{Tag, TagType};

use super::item::new_entity_item;
use super::{Entity, EntityBase, EntityKindData, base, dispatch};
use crate::block::ids as block_ids;
use crate::block::rail;
use crate::item::ItemStack;
use crate::save::SaveError;
use crate::save::nbt::{self as n, Compound};
use crate::world::World;
use crate::world::client_requests::PlayerGui;

/// `Item.minecartEmpty.shiftedIndex` (`new ItemMinecart(72, 0)`).
// src: Item.java:346 (shiftedIndex = 256 + id, Item.java:124)
pub const ITEM_MINECART_EMPTY: i32 = 256 + 72;

/// `MATRIX` (private static): per rail shape (metadata 0-9), the two ends of
/// the track as `{dx, dy, dz}` cell offsets; `dy` is -1 where the end is a
/// slope's low side.
// src: EntityMinecart.java:15
pub const MATRIX: [[[i32; 3]; 2]; 10] = [
    [[0, 0, -1], [0, 0, 1]],
    [[-1, 0, 0], [1, 0, 0]],
    [[-1, -1, 0], [1, 0, 0]],
    [[-1, 0, 0], [1, -1, 0]],
    [[0, 0, -1], [0, -1, 1]],
    [[0, -1, -1], [0, 0, 1]],
    [[0, 0, 1], [1, 0, 0]],
    [[0, 0, 1], [-1, 0, 0]],
    [[0, 0, -1], [-1, 0, 0]],
    [[0, 0, -1], [1, 0, 0]],
];

/// The fields `EntityMinecart` declares (EntityMinecart.java:6-24).
#[derive(Clone, Debug, PartialEq)]
pub struct MinecartData {
    /// `cargoItems` (private): 36 slots from the constructor, 27 after a
    /// chest cart's `readEntityFromNBT` (module docs).
    pub cargo_items: Vec<Option<ItemStack>>,
    /// `minecartCurrentDamage`: +10 per damage point, breaks above 40,
    /// heals 1 per tick.
    pub minecart_current_damage: i32,
    /// `minecartTimeSinceHit`: the hurt wobble, 10 after a hit.
    pub minecart_time_since_hit: i32,
    /// `minecartRockDirection`: the wobble's side, flipped on every hit.
    pub minecart_rock_direction: i32,
    /// `isInReverse` (private): the yaw is 180 degrees off the motion.
    pub is_in_reverse: bool,
    /// `minecartType`: 0 plain, 1 chest, 2 furnace.
    pub minecart_type: i32,
    /// `fuel`: furnace ticks (1200 per coal), spent one in four smoking
    /// ticks.
    pub fuel: i32,
    /// `pushX`.
    pub push_x: f64,
    /// `pushZ`.
    pub push_z: f64,
    /// `turnProgress` (private): multiplayer interpolation steps left.
    pub turn_progress: i32,
    /// `minecartX` (private): multiplayer target position.
    pub minecart_x: f64,
    /// `minecartY` (private).
    pub minecart_y: f64,
    /// `minecartZ` (private).
    pub minecart_z: f64,
    /// `minecartYaw` (private).
    pub minecart_yaw: f64,
    /// `minecartPitch` (private).
    pub minecart_pitch: f64,
    /// `velocityX` (private): multiplayer velocity, restored by
    /// `setPositionAndRotation2`.
    pub velocity_x: f64,
    /// `velocityY` (private).
    pub velocity_y: f64,
    /// `velocityZ` (private).
    pub velocity_z: f64,
}

impl Default for MinecartData {
    /// The field initialisers and the `(World)` constructor's assignments.
    // src: EntityMinecart.java:6-31
    fn default() -> Self {
        MinecartData {
            cargo_items: vec![None; 36],
            minecart_current_damage: 0,
            minecart_time_since_hit: 0,
            minecart_rock_direction: 1,
            is_in_reverse: false,
            minecart_type: 0,
            fuel: 0,
            push_x: 0.0,
            push_z: 0.0,
            turn_progress: 0,
            minecart_x: 0.0,
            minecart_y: 0.0,
            minecart_z: 0.0,
            minecart_yaw: 0.0,
            minecart_pitch: 0.0,
            velocity_x: 0.0,
            velocity_y: 0.0,
            velocity_z: 0.0,
        }
    }
}

impl MinecartData {
    /// `getSizeInventory()`: 27 (not `cargoItems.length`, module docs).
    // src: EntityMinecart.java:716-718
    pub const fn get_size_inventory(&self) -> i32 {
        27
    }

    /// `getStackInSlot(slot)`. Panics where Java throws
    /// `ArrayIndexOutOfBoundsException`.
    // src: EntityMinecart.java:720-722
    pub fn get_stack_in_slot(&self, slot: i32) -> Option<&ItemStack> {
        self.cargo_items[slot as usize].as_ref()
    }

    /// `decrStackSize(slot, n)`: the whole stack when it has at most `n`
    /// items, otherwise `n` split off it.
    // src: EntityMinecart.java:724-742
    pub fn decr_stack_size(&mut self, var1: i32, var2: i32) -> Option<ItemStack> {
        let slot = &mut self.cargo_items[var1 as usize];
        let s = slot.as_mut()?;
        if s.stack_size <= var2 {
            slot.take()
        } else {
            let var3 = s.split_stack(var2);
            if s.stack_size == 0 {
                *slot = None;
            }
            Some(var3)
        }
    }

    /// `setInventorySlotContents(slot, stack)`: stored, then capped at
    /// `getInventoryStackLimit()` (64).
    // src: EntityMinecart.java:744-750
    pub fn set_inventory_slot_contents(&mut self, var1: i32, var2: Option<ItemStack>) {
        let mut var2 = var2;
        if let Some(s) = var2.as_mut()
            && s.stack_size > self.get_inventory_stack_limit()
        {
            s.stack_size = self.get_inventory_stack_limit();
        }
        self.cargo_items[var1 as usize] = var2;
    }

    /// `getInvName()`.
    // src: EntityMinecart.java:752-754
    pub const fn get_inv_name(&self) -> &'static str {
        "Minecart"
    }

    /// `getInventoryStackLimit()`.
    // src: EntityMinecart.java:756-758
    pub const fn get_inventory_stack_limit(&self) -> i32 {
        64
    }

    /// `onInventoryChanged()`: empty.
    // src: EntityMinecart.java:760-761
    pub fn on_inventory_changed(&mut self) {}
}

/// The `EntityMinecart` level of `e`.
fn data(e: &mut Entity) -> &mut MinecartData {
    match &mut e.kind {
        EntityKindData::Minecart(d) => d,
        _ => panic!("EntityMinecart method on another kind"),
    }
}

fn data_ref(e: &Entity) -> &MinecartData {
    match &e.kind {
        EntityKindData::Minecart(d) => d,
        _ => panic!("EntityMinecart method on another kind"),
    }
}

/// `new EntityMinecart(world)`: the `EntityList` / NBT-load constructor and
/// the harness `spawn` op's: type 0, 0.98 x 0.7, centred (`yOffset` is half
/// the height), `preventEntitySpawning`.
// src: EntityMinecart.java:26-36
pub fn new_entity_minecart_empty(world: &mut World) -> Entity {
    let mut b = EntityBase::new(world);
    // entityInit() is empty (EntityMinecart.java:42-43).
    let d = MinecartData::default();
    b.prevent_entity_spawning = true;
    b.set_size(0.98, 0.7);
    b.y_offset = b.height / 2.0;
    Entity {
        base: b,
        kind: EntityKindData::Minecart(d),
    }
}

/// `new EntityMinecart(world, x, y, z, type)` (`ItemMinecart.onItemUse`):
/// at `(x, y + yOffset, z)`, still, `prevPos` the given (feet) position.
// src: EntityMinecart.java:57-67
pub fn new_entity_minecart(world: &mut World, x: f64, y: f64, z: f64, var8: i32) -> Entity {
    let mut e = new_entity_minecart_empty(world);
    let b = &mut e.base;
    b.set_position(x, y + b.y_offset as f64, z);
    b.motion_x = 0.0;
    b.motion_y = 0.0;
    b.motion_z = 0.0;
    b.prev_pos_x = x;
    b.prev_pos_y = y;
    b.prev_pos_z = z;
    data(&mut e).minecart_type = var8;
    e
}

/// `EntityMinecart.getMountedYOffset()`.
// src: EntityMinecart.java:69-71
pub fn get_mounted_y_offset(e: &Entity) -> f64 {
    e.base.height as f64 * 0.0 - 0.30000001192092896
}

/// `EntityMinecart.attackEntityFrom(attacker, damage)`: the wobble, then
/// `damage * 10` damage; above 40 the rider is put off, the cart dies
/// (`setEntityDead`, the cargo drop) and drops a minecart, the chest cart
/// its cargo again (module docs) and a chest, the furnace cart a furnace.
/// Always true.
// src: EntityMinecart.java:73-123
pub fn attack_entity_from(
    e: &mut Entity,
    world: &mut World,
    attacker: Option<&Entity>,
    var2: i32,
) -> bool {
    if !world.multiplayer_world && !e.base.is_dead {
        {
            let d = data(e);
            d.minecart_rock_direction = -d.minecart_rock_direction;
            d.minecart_time_since_hit = 10;
        }
        e.base.set_been_attacked();
        let d = data(e);
        d.minecart_current_damage = d
            .minecart_current_damage
            .wrapping_add(var2.wrapping_mul(10));
        if d.minecart_current_damage > 40 {
            if let Some(rider) = e.base.ridden_by_entity {
                // `this.riddenByEntity.mountEntity(this)`: the rider, already
                // on this cart, gets off on top of it. A rider punching its
                // own cart is the attacker, held out of its slot by the
                // caller: `player_controller::attack_entity` puts it off
                // right after this returns (the cargo and item drops in
                // between read neither; WP-4.8, entity/minecart-broken-by-rider).
                if attacker.is_some_and(|a| a.base.entity_id == rider) {
                } else if world
                    .with_entity_mut(rider, |w, r| base::mount_entity(r, w, Some(e)))
                    .is_none()
                {
                    world.note_unimplemented(
                        "EntityMinecart.attackEntityFrom: rider out of its slot",
                    );
                }
            }
            dispatch::set_entity_dead(e, world);
            base::drop_item_with_offset(e, world, ITEM_MINECART_EMPTY, 1, 0.0);
            let ty = data_ref(e).minecart_type;
            if ty == 1 {
                // src: EntityMinecart.java:87-111 (the second walk over the
                // emptied slots, module docs)
                drop_cargo(e, world);
                base::drop_item_with_offset(e, world, block_ids::CHEST, 1, 0.0);
            } else if ty == 2 {
                base::drop_item_with_offset(e, world, block_ids::STONE_OVEN_IDLE, 1, 0.0);
            }
        }
    }
    true
}

/// The cargo drop loop shared by `attackEntityFrom` (:89-111) and
/// `setEntityDead` (:137-159): for every non-null slot of the 27, three
/// `rand` offsets, then items of 10-30 until the stack is empty (size 0, left
/// in its slot), each with a `nextGaussian` motion.
// src: EntityMinecart.java:137-159
fn drop_cargo(e: &mut Entity, world: &mut World) {
    let n = data_ref(e).get_size_inventory();
    for var1 in 0..n {
        let Some(&var2) = data_ref(e).get_stack_in_slot(var1) else {
            continue;
        };
        let var3 = e.base.rand.next_float() * 0.8 + 0.1;
        let var4 = e.base.rand.next_float() * 0.8 + 0.1;
        let var5 = e.base.rand.next_float() * 0.8 + 0.1;
        let mut size = var2.stack_size;
        while size > 0 {
            let mut var6 = e.base.rand.next_int_bounded(21) + 10;
            if var6 > size {
                var6 = size;
            }
            size -= var6;
            // `var2.stackSize -= var6` on the slot's own stack.
            if let Some(s) = data(e).cargo_items[var1 as usize].as_mut() {
                s.stack_size = size;
            }
            let mut var7 = new_entity_item(
                world,
                e.base.pos_x + var3 as f64,
                e.base.pos_y + var4 as f64,
                e.base.pos_z + var5 as f64,
                ItemStack::new(var2.item_id, var6, var2.item_damage),
            );
            let var8: f32 = 0.05;
            let r = &mut e.base.rand;
            var7.base.motion_x = (r.next_gaussian() as f32 * var8) as f64;
            var7.base.motion_y = (r.next_gaussian() as f32 * var8 + 0.2) as f64;
            var7.base.motion_z = (r.next_gaussian() as f32 * var8) as f64;
            world.spawn_entity_in_world(var7);
        }
    }
}

/// `EntityMinecart.performHurtAnimation()` (client): the wobble. The
/// `System.out.println("Animating hurt")` is not ported.
// src: EntityMinecart.java:125-130
pub fn perform_hurt_animation(e: &mut Entity) {
    let d = data(e);
    d.minecart_rock_direction = -d.minecart_rock_direction;
    d.minecart_time_since_hit = 10;
    d.minecart_current_damage = d
        .minecart_current_damage
        .wrapping_add(d.minecart_current_damage.wrapping_mul(10));
}

/// `EntityMinecart.setEntityDead()`: drops the cargo (every type walks the
/// slots; only a chest cart has any), then `Entity.setEntityDead`.
// src: EntityMinecart.java:136-162
pub fn set_entity_dead(e: &mut Entity, world: &mut World) {
    drop_cargo(e, world);
    e.base.set_entity_dead();
}

/// `EntityMinecart.onUpdate()` (module docs).
// src: EntityMinecart.java:164-495
pub fn on_update(e: &mut Entity, world: &mut World) {
    {
        let d = data(e);
        if d.minecart_time_since_hit > 0 {
            d.minecart_time_since_hit -= 1;
        }
        if d.minecart_current_damage > 0 {
            d.minecart_current_damage -= 1;
        }
    }

    if world.multiplayer_world && data_ref(e).turn_progress > 0 {
        // src: EntityMinecart.java:174-196 (multiplayer interpolation; the
        // inner `else` is unreachable: the outer test is `turnProgress > 0`)
        let d = data_ref(e).clone();
        let b = &mut e.base;
        let tp = d.turn_progress as f64;
        let var46 = b.pos_x + (d.minecart_x - b.pos_x) / tp;
        let var47 = b.pos_y + (d.minecart_y - b.pos_y) / tp;
        let var5 = b.pos_z + (d.minecart_z - b.pos_z) / tp;
        let mut var7 = d.minecart_yaw - b.rotation_yaw as f64;
        while var7 < -180.0 {
            var7 += 360.0;
        }
        while var7 >= 180.0 {
            var7 -= 360.0;
        }
        b.rotation_yaw = (b.rotation_yaw as f64 + var7 / tp) as f32;
        b.rotation_pitch =
            (b.rotation_pitch as f64 + (d.minecart_pitch - b.rotation_pitch as f64) / tp) as f32;
        data(e).turn_progress -= 1;
        let b = &mut e.base;
        b.set_position(var46, var47, var5);
        let (yaw, pitch) = (b.rotation_yaw, b.rotation_pitch);
        b.set_rotation(yaw, pitch);
        return;
    }

    {
        let b = &mut e.base;
        b.prev_pos_x = b.pos_x;
        b.prev_pos_y = b.pos_y;
        b.prev_pos_z = b.pos_z;
        b.motion_y -= 0.03999999910593033;
    }
    let var1 = floor_double(e.base.pos_x);
    let mut var2 = floor_double(e.base.pos_y);
    let var3 = floor_double(e.base.pos_z);
    if rail::is_rail_block_at(world, var1, var2 - 1, var3) {
        var2 -= 1;
    }
    let var4 = 0.4;
    let mut var6 = false;
    let var7 = 0.0078125;
    let var9 = world.get_block_id(var1, var2, var3);
    if rail::is_rail_block(var9) {
        let var10 = get_pos(world, e.base.pos_x, e.base.pos_y, e.base.pos_z);
        let mut var11 = world.get_block_metadata(var1, var2, var3);
        e.base.pos_y = var2 as f64;
        let mut var12 = false;
        let mut var13 = false;
        if var9 == rail::RAIL_POWERED {
            var12 = (var11 & 8) != 0;
            var13 = !var12;
        }
        if rail::get_is_powered(var9) {
            var11 &= 7;
        }
        let b = &mut e.base;
        if (2..=5).contains(&var11) {
            b.pos_y = (var2 + 1) as f64;
        }
        if var11 == 2 {
            b.motion_x -= var7;
        }
        if var11 == 3 {
            b.motion_x += var7;
        }
        if var11 == 4 {
            b.motion_z += var7;
        }
        if var11 == 5 {
            b.motion_z -= var7;
        }

        // Java throws ArrayIndexOutOfBoundsException for a plain rail with
        // metadata above 9; so does the index here.
        let var14 = MATRIX[var11 as usize];
        let mut var15 = (var14[1][0] - var14[0][0]) as f64;
        let mut var17 = (var14[1][2] - var14[0][2]) as f64;
        let var19 = jmath::sqrt(var15 * var15 + var17 * var17);
        let var21 = b.motion_x * var15 + b.motion_z * var17;
        if var21 < 0.0 {
            var15 = -var15;
            var17 = -var17;
        }
        let mut var23 = jmath::sqrt(b.motion_x * b.motion_x + b.motion_z * b.motion_z);
        b.motion_x = var23 * var15 / var19;
        b.motion_z = var23 * var17 / var19;
        if var13 {
            // The brake of an unpowered powered rail.
            let var25 = jmath::sqrt(b.motion_x * b.motion_x + b.motion_z * b.motion_z);
            if var25 < 0.03 {
                b.motion_x *= 0.0;
                b.motion_y *= 0.0;
                b.motion_z *= 0.0;
            } else {
                b.motion_x *= 0.5;
                b.motion_y *= 0.0;
                b.motion_z *= 0.5;
            }
        }

        let var27 = var1 as f64 + 0.5 + var14[0][0] as f64 * 0.5;
        let var29 = var3 as f64 + 0.5 + var14[0][2] as f64 * 0.5;
        let var31 = var1 as f64 + 0.5 + var14[1][0] as f64 * 0.5;
        let var33 = var3 as f64 + 0.5 + var14[1][2] as f64 * 0.5;
        var15 = var31 - var27;
        var17 = var33 - var29;
        // `var25`: the position along the track (Java declares it 0.0 first).
        let var25 = if var15 == 0.0 {
            b.pos_x = var1 as f64 + 0.5;
            b.pos_z - var3 as f64
        } else if var17 == 0.0 {
            b.pos_z = var3 as f64 + 0.5;
            b.pos_x - var1 as f64
        } else {
            let var35 = b.pos_x - var27;
            let var37 = b.pos_z - var29;
            // var39
            (var35 * var15 + var37 * var17) * 2.0
        };
        b.pos_x = var27 + var15 * var25;
        b.pos_z = var29 + var17 * var25;
        let (x, y, z) = (b.pos_x, b.pos_y + b.y_offset as f64, b.pos_z);
        b.set_position(x, y, z);
        let mut var35 = b.motion_x;
        let mut var37 = b.motion_z;
        if b.ridden_by_entity.is_some() {
            var35 *= 0.75;
            var37 *= 0.75;
        }
        if var35 < -var4 {
            var35 = -var4;
        }
        if var35 > var4 {
            var35 = var4;
        }
        if var37 < -var4 {
            var37 = -var4;
        }
        if var37 > var4 {
            var37 = var4;
        }
        dispatch::move_entity(e, world, var35, 0.0, var37);
        let b = &mut e.base;
        if var14[0][1] != 0
            && floor_double(b.pos_x) - var1 == var14[0][0]
            && floor_double(b.pos_z) - var3 == var14[0][2]
        {
            let (x, y, z) = (b.pos_x, b.pos_y + var14[0][1] as f64, b.pos_z);
            b.set_position(x, y, z);
        } else if var14[1][1] != 0
            && floor_double(b.pos_x) - var1 == var14[1][0]
            && floor_double(b.pos_z) - var3 == var14[1][2]
        {
            let (x, y, z) = (b.pos_x, b.pos_y + var14[1][1] as f64, b.pos_z);
            b.set_position(x, y, z);
        }

        if e.base.ridden_by_entity.is_some() {
            let b = &mut e.base;
            b.motion_x *= 0.996999979019165;
            b.motion_y *= 0.0;
            b.motion_z *= 0.996999979019165;
        } else {
            if data_ref(e).minecart_type == 2 {
                let d = data(e);
                let var39 = math::sqrt_double(d.push_x * d.push_x + d.push_z * d.push_z) as f64;
                if var39 > 0.01 {
                    var6 = true;
                    d.push_x /= var39;
                    d.push_z /= var39;
                    let var41 = 0.04;
                    let (px, pz) = (d.push_x, d.push_z);
                    let b = &mut e.base;
                    b.motion_x *= 0.800000011920929;
                    b.motion_y *= 0.0;
                    b.motion_z *= 0.800000011920929;
                    b.motion_x += px * var41;
                    b.motion_z += pz * var41;
                } else {
                    let b = &mut e.base;
                    b.motion_x *= 0.8999999761581421;
                    b.motion_y *= 0.0;
                    b.motion_z *= 0.8999999761581421;
                }
            }
            let b = &mut e.base;
            b.motion_x *= 0.9599999785423279;
            b.motion_y *= 0.0;
            b.motion_z *= 0.9599999785423279;
        }

        let var52 = get_pos(world, e.base.pos_x, e.base.pos_y, e.base.pos_z);
        let b = &mut e.base;
        if let (Some(var52), Some(var10)) = (var52, var10) {
            let var40 = (var10.y_coord - var52.y_coord) * 0.05;
            var23 = jmath::sqrt(b.motion_x * b.motion_x + b.motion_z * b.motion_z);
            if var23 > 0.0 {
                b.motion_x = b.motion_x / var23 * (var23 + var40);
                b.motion_z = b.motion_z / var23 * (var23 + var40);
            }
            let (x, z) = (b.pos_x, b.pos_z);
            b.set_position(x, var52.y_coord, z);
        }

        let var53 = floor_double(b.pos_x);
        let var54 = floor_double(b.pos_z);
        if var53 != var1 || var54 != var3 {
            var23 = jmath::sqrt(b.motion_x * b.motion_x + b.motion_z * b.motion_z);
            b.motion_x = var23 * (var53 - var1) as f64;
            b.motion_z = var23 * (var54 - var3) as f64;
        }

        if data_ref(e).minecart_type == 2 {
            let (mx, mz) = (e.base.motion_x, e.base.motion_z);
            let d = data(e);
            let var42 = math::sqrt_double(d.push_x * d.push_x + d.push_z * d.push_z) as f64;
            if var42 > 0.01 && mx * mx + mz * mz > 0.001 {
                d.push_x /= var42;
                d.push_z /= var42;
                if d.push_x * mx + d.push_z * mz < 0.0 {
                    d.push_x = 0.0;
                    d.push_z = 0.0;
                } else {
                    d.push_x = mx;
                    d.push_z = mz;
                }
            }
        }

        if var12 {
            // The boost of a powered powered rail, or the push off a wall
            // for a cart standing still on a straight one.
            let b = &mut e.base;
            let var42 = jmath::sqrt(b.motion_x * b.motion_x + b.motion_z * b.motion_z);
            if var42 > 0.01 {
                let var44 = 0.06;
                b.motion_x += b.motion_x / var42 * var44;
                b.motion_z += b.motion_z / var42 * var44;
            } else if var11 == 1 {
                if world.is_block_normal_cube(var1 - 1, var2, var3) {
                    e.base.motion_x = 0.02;
                } else if world.is_block_normal_cube(var1 + 1, var2, var3) {
                    e.base.motion_x = -0.02;
                }
            } else if var11 == 0 {
                if world.is_block_normal_cube(var1, var2, var3 - 1) {
                    e.base.motion_z = 0.02;
                } else if world.is_block_normal_cube(var1, var2, var3 + 1) {
                    e.base.motion_z = -0.02;
                }
            }
        }
    } else {
        // Off the rails.
        let b = &mut e.base;
        if b.motion_x < -var4 {
            b.motion_x = -var4;
        }
        if b.motion_x > var4 {
            b.motion_x = var4;
        }
        if b.motion_z < -var4 {
            b.motion_z = -var4;
        }
        if b.motion_z > var4 {
            b.motion_z = var4;
        }
        if b.on_ground {
            b.motion_x *= 0.5;
            b.motion_y *= 0.5;
            b.motion_z *= 0.5;
        }
        let (mx, my, mz) = (b.motion_x, b.motion_y, b.motion_z);
        dispatch::move_entity(e, world, mx, my, mz);
        let b = &mut e.base;
        if !b.on_ground {
            b.motion_x *= 0.949999988079071;
            b.motion_y *= 0.949999988079071;
            b.motion_z *= 0.949999988079071;
        }
    }

    // src: EntityMinecart.java:447-470 (the yaw follows the motion, turned
    // round when it jumps by about 180 degrees)
    let b = &mut e.base;
    b.rotation_pitch = 0.0;
    let var48 = b.prev_pos_x - b.pos_x;
    let var49 = b.prev_pos_z - b.pos_z;
    let reverse = data_ref(e).is_in_reverse;
    let b = &mut e.base;
    if var48 * var48 + var49 * var49 > 0.001 {
        b.rotation_yaw = (jmath::atan2(var49, var48) * 180.0 / 3.141592653589793) as f32;
        if reverse {
            b.rotation_yaw += 180.0;
        }
    }
    let mut var50 = (b.rotation_yaw - b.prev_rotation_yaw) as f64;
    while var50 >= 180.0 {
        var50 -= 360.0;
    }
    while var50 < -180.0 {
        var50 += 360.0;
    }
    if var50 < -170.0 || var50 >= 170.0 {
        b.rotation_yaw += 180.0;
        let d = data(e);
        d.is_in_reverse = !d.is_in_reverse;
    }
    let b = &mut e.base;
    let (yaw, pitch) = (b.rotation_yaw, b.rotation_pitch);
    b.set_rotation(yaw, pitch);

    // src: EntityMinecart.java:471-479 (other carts touching this one run
    // their applyEntityCollision with this cart in hand)
    let bb = e
        .base
        .bounding_box
        .expand(0.20000000298023224, 0.0, 0.20000000298023224);
    let var16 = world.get_entities_within_aabb_excluding_entity(Some(e.base.entity_id), &bb);
    for id in var16 {
        if Some(id) == e.base.ridden_by_entity {
            continue;
        }
        world.with_entity_mut(id, |w, var18| {
            if dispatch::can_be_pushed(var18, w)
                && matches!(var18.kind, EntityKindData::Minecart(..))
            {
                dispatch::apply_entity_collision(var18, w, e);
            }
        });
    }

    if let Some(r) = e.base.ridden_by_entity
        && world.is_entity_dead_or_gone(r)
    {
        e.base.ridden_by_entity = None;
    }

    if var6 && e.base.rand.next_int_bounded(4) == 0 {
        let d = data(e);
        d.fuel -= 1;
        if d.fuel < 0 {
            d.push_x = 0.0;
            d.push_z = 0.0;
        }
        let b = &e.base;
        world.spawn_particle("largesmoke", b.pos_x, b.pos_y + 0.8, b.pos_z, 0.0, 0.0, 0.0);
    }
}

/// `getPosOffset(x, y, z, offset)` (render, `RenderMinecart`): the track
/// point `offset` along the rail's direction from `(x, y, z)`, `None` off
/// the rails.
// src: EntityMinecart.java:497-535
pub fn get_pos_offset(
    world: &mut World,
    mut var1: f64,
    mut var3: f64,
    mut var5: f64,
    var7: f64,
) -> Option<Vec3D> {
    let var9 = floor_double(var1);
    let mut var10 = floor_double(var3);
    let var11 = floor_double(var5);
    if rail::is_rail_block_at(world, var9, var10 - 1, var11) {
        var10 -= 1;
    }
    let var12 = world.get_block_id(var9, var10, var11);
    if !rail::is_rail_block(var12) {
        return None;
    }
    let mut var13 = world.get_block_metadata(var9, var10, var11);
    if rail::get_is_powered(var12) {
        var13 &= 7;
    }
    var3 = var10 as f64;
    if (2..=5).contains(&var13) {
        var3 = (var10 + 1) as f64;
    }
    let var14 = MATRIX[var13 as usize];
    let mut var15 = (var14[1][0] - var14[0][0]) as f64;
    let mut var17 = (var14[1][2] - var14[0][2]) as f64;
    let var19 = jmath::sqrt(var15 * var15 + var17 * var17);
    var15 /= var19;
    var17 /= var19;
    var1 += var15 * var7;
    var5 += var17 * var7;
    if var14[0][1] != 0
        && floor_double(var1) - var9 == var14[0][0]
        && floor_double(var5) - var11 == var14[0][2]
    {
        var3 += var14[0][1] as f64;
    } else if var14[1][1] != 0
        && floor_double(var1) - var9 == var14[1][0]
        && floor_double(var5) - var11 == var14[1][2]
    {
        var3 += var14[1][1] as f64;
    }
    get_pos(world, var1, var3, var5)
}

/// `getPos(x, y, z)`: the point of the track under `(x, y, z)` (the rail in
/// that cell or the one below) nearest along the track, its height raised
/// on slopes; `None` off the rails.
// src: EntityMinecart.java:537-596
#[allow(unused_assignments)] // `var3 = var8` (:548, :554) is dead in Java too
pub fn get_pos(world: &mut World, mut var1: f64, mut var3: f64, mut var5: f64) -> Option<Vec3D> {
    let var7 = floor_double(var1);
    let mut var8 = floor_double(var3);
    let var9 = floor_double(var5);
    if rail::is_rail_block_at(world, var7, var8 - 1, var9) {
        var8 -= 1;
    }
    let var10 = world.get_block_id(var7, var8, var9);
    if !rail::is_rail_block(var10) {
        return None;
    }
    let mut var11 = world.get_block_metadata(var7, var8, var9);
    var3 = var8 as f64;
    if rail::get_is_powered(var10) {
        var11 &= 7;
    }
    if (2..=5).contains(&var11) {
        var3 = (var8 + 1) as f64;
    }
    let var12 = MATRIX[var11 as usize];
    let var15 = var7 as f64 + 0.5 + var12[0][0] as f64 * 0.5;
    let var17 = var8 as f64 + 0.5 + var12[0][1] as f64 * 0.5;
    let var19 = var9 as f64 + 0.5 + var12[0][2] as f64 * 0.5;
    let var21 = var7 as f64 + 0.5 + var12[1][0] as f64 * 0.5;
    let var23 = var8 as f64 + 0.5 + var12[1][1] as f64 * 0.5;
    let var25 = var9 as f64 + 0.5 + var12[1][2] as f64 * 0.5;
    let var27 = var21 - var15;
    let var29 = (var23 - var17) * 2.0;
    let var31 = var25 - var19;
    // `var13`: the position along the track (Java declares it 0.0 first).
    let var13 = if var27 == 0.0 {
        var1 = var7 as f64 + 0.5;
        var5 - var9 as f64
    } else if var31 == 0.0 {
        var5 = var9 as f64 + 0.5;
        var1 - var7 as f64
    } else {
        let var33 = var1 - var15;
        let var35 = var5 - var19;
        // var37
        (var33 * var27 + var35 * var31) * 2.0
    };
    var1 = var15 + var27 * var13;
    var3 = var17 + var29 * var13;
    var5 = var19 + var31 * var13;
    if var29 < 0.0 {
        var3 += 1.0;
    }
    if var29 > 0.0 {
        var3 += 0.5;
    }
    Some(Vec3D::create_vector(var1, var3, var5))
}

/// `EntityMinecart.writeEntityToNBT(compound)`: `Type`; a furnace cart's
/// `PushX`, `PushZ`, `Fuel` (short); a chest cart's `Items` (the non-null
/// slots of `cargoItems`, each with its `Slot` byte).
// src: EntityMinecart.java:598-619
pub fn write_entity_to_nbt(d: &MinecartData, c: &mut Compound) {
    n::set(c, "Type", Tag::Int(d.minecart_type));
    if d.minecart_type == 2 {
        n::set(c, "PushX", Tag::Double(d.push_x));
        n::set(c, "PushZ", Tag::Double(d.push_z));
        n::set(c, "Fuel", Tag::Short(d.fuel as i16));
    } else if d.minecart_type == 1 {
        let mut var2 = Vec::new();
        for (var3, s) in d.cargo_items.iter().enumerate() {
            if let Some(s) = s {
                let mut var4 = Compound::new();
                n::set(&mut var4, "Slot", Tag::Byte(var3 as i8));
                s.write_to_nbt_into(&mut var4);
                var2.push(Tag::Compound(var4));
            }
        }
        n::set(c, "Items", n::list_of(TagType::Compound, var2));
    }
}

/// `EntityMinecart.readEntityFromNBT(compound)`: `Type`, then the furnace or
/// chest fields; a chest cart's `cargoItems` becomes 27 slots, filled from
/// `Items` by `Slot & 255`.
// src: EntityMinecart.java:621-640
pub fn read_entity_from_nbt(d: &mut MinecartData, c: &Compound) -> Result<(), SaveError> {
    d.minecart_type = n::get_integer(c, "Type")?;
    if d.minecart_type == 2 {
        d.push_x = n::get_double(c, "PushX")?;
        d.push_z = n::get_double(c, "PushZ")?;
        d.fuel = n::get_short(c, "Fuel")? as i32;
    } else if d.minecart_type == 1 {
        let var2 = n::get_tag_list(c, "Items")?;
        d.cargo_items = vec![None; d.get_size_inventory() as usize];
        for t in var2 {
            let var4 = n::as_compound(t)?;
            let var5 = n::get_byte(var4, "Slot")? as i32 & 255;
            if var5 >= 0 && (var5 as usize) < d.cargo_items.len() {
                d.cargo_items[var5 as usize] = Some(ItemStack::from_nbt(t)?);
            }
        }
    }
    Ok(())
}

/// `EntityMinecart.applyEntityCollision(other)` (`other` in hand): a moving
/// empty cart picks up a mob it touches; then the two are pushed apart, a
/// cart against a cart by the momentum exchange (the furnace cart pushing a
/// plain one), anything else by a plain push.
// src: EntityMinecart.java:646-714
pub fn apply_entity_collision(e: &mut Entity, world: &mut World, var1: &mut Entity) {
    if world.multiplayer_world || e.base.ridden_by_entity == Some(var1.base.entity_id) {
        return;
    }
    {
        let b = &e.base;
        if var1.kind.living().is_some()
            && !var1.kind.is_player()
            && data_ref(e).minecart_type == 0
            && b.motion_x * b.motion_x + b.motion_z * b.motion_z > 0.01
            && b.ridden_by_entity.is_none()
            && var1.base.riding_entity.is_none()
        {
            // `var1.mountEntity(this)`: both in hand.
            base::mount_entity(var1, world, Some(e));
        }
    }
    let mut var2 = var1.base.pos_x - e.base.pos_x;
    let mut var4 = var1.base.pos_z - e.base.pos_z;
    let mut var6 = var2 * var2 + var4 * var4;
    if var6 < 9.999999747378752E-5 {
        return;
    }
    var6 = math::sqrt_double(var6) as f64;
    var2 /= var6;
    var4 /= var6;
    let mut var8 = 1.0 / var6;
    if var8 > 1.0 {
        var8 = 1.0;
    }
    var2 *= var8;
    var4 *= var8;
    var2 *= 0.10000000149011612;
    var4 *= 0.10000000149011612;
    var2 *= (1.0f32 - e.base.entity_collision_reduction) as f64;
    var4 *= (1.0f32 - e.base.entity_collision_reduction) as f64;
    var2 *= 0.5;
    var4 *= 0.5;
    if let EntityKindData::Minecart(od) = &var1.kind {
        let other_type = od.minecart_type;
        let var10 = var1.base.pos_x - e.base.pos_x;
        let var12 = var1.base.pos_z - e.base.pos_z;
        // vanilla bug: `var12 * var1.prevPosX`, a position where a motion
        // belongs (module docs).
        let mut var14 = var10 * var1.base.motion_z + var12 * var1.base.prev_pos_x;
        var14 *= var14;
        if var14 > 5.0 {
            return;
        }
        let mut var16 = var1.base.motion_x + e.base.motion_x;
        let mut var18 = var1.base.motion_z + e.base.motion_z;
        let ty = data_ref(e).minecart_type;
        if other_type == 2 && ty != 2 {
            e.base.motion_x *= 0.20000000298023224;
            e.base.motion_z *= 0.20000000298023224;
            let (ox, oz) = (var1.base.motion_x, var1.base.motion_z);
            dispatch::add_velocity(e, world, ox - var2, 0.0, oz - var4);
            var1.base.motion_x *= 0.699999988079071;
            var1.base.motion_z *= 0.699999988079071;
        } else if other_type != 2 && ty == 2 {
            var1.base.motion_x *= 0.20000000298023224;
            var1.base.motion_z *= 0.20000000298023224;
            let (mx, mz) = (e.base.motion_x, e.base.motion_z);
            dispatch::add_velocity(var1, world, mx + var2, 0.0, mz + var4);
            e.base.motion_x *= 0.699999988079071;
            e.base.motion_z *= 0.699999988079071;
        } else {
            var16 /= 2.0;
            var18 /= 2.0;
            e.base.motion_x *= 0.20000000298023224;
            e.base.motion_z *= 0.20000000298023224;
            dispatch::add_velocity(e, world, var16 - var2, 0.0, var18 - var4);
            var1.base.motion_x *= 0.20000000298023224;
            var1.base.motion_z *= 0.20000000298023224;
            dispatch::add_velocity(var1, world, var16 + var2, 0.0, var18 + var4);
        }
    } else {
        dispatch::add_velocity(e, world, -var2, 0.0, -var4);
        dispatch::add_velocity(var1, world, var2 / 4.0, 0.0, var4 / 4.0);
    }
}

/// `EntityMinecart.interact(player)` (`player` in hand): a plain cart takes
/// the player on (unless another player rides it), a chest cart opens its
/// GUI (module docs), a furnace cart takes one coal from the
/// player's hand for 1200 fuel and is pushed away from the player. Always
/// true.
// src: EntityMinecart.java:763-791
pub fn interact(e: &mut Entity, world: &mut World, var1: &mut Entity) -> bool {
    let ty = data_ref(e).minecart_type;
    if ty == 0 {
        if let Some(r) = e.base.ridden_by_entity
            && r != var1.base.entity_id
            && rider_is_player(world, r)
        {
            return true;
        }
        if !world.multiplayer_world {
            // `var1.mountEntity(this)`: both in hand.
            base::mount_entity(var1, world, Some(e));
        }
    } else if ty == 1 {
        if !world.multiplayer_world {
            // src: EntityMinecart.java:774 (`var1.displayGUIChest(this)`:
            // EntityPlayerSP opens GuiChest over a ContainerChest,
            // EntityPlayerSP.java:117-119; EntityPlayer's version is
            // empty, EntityPlayer.java:340-341).
            super::player::display_gui(
                var1,
                world,
                PlayerGui::MinecartChest {
                    entity_id: e.base.entity_id,
                },
            );
        }
    } else if ty == 2 {
        if let Some(p) = var1.kind.player_mut() {
            let inv = &mut p.inventory;
            let cur = inv.current_item;
            if inv
                .get_current_item()
                .is_some_and(|s| s.item_id == crate::item::ids::COAL)
            {
                // `getCurrentItem()` returned slot `currentItem` (0-8).
                let s = inv.main_inventory[cur as usize]
                    .as_mut()
                    .expect("getCurrentItem returned this slot's stack");
                s.stack_size -= 1;
                if s.stack_size == 0 {
                    // `inventory.setInventorySlotContents(currentItem, null)`:
                    // a hotbar slot is in mainInventory.
                    // src: InventoryPlayer.java:180-188
                    inv.main_inventory[cur as usize] = None;
                }
                data(e).fuel += 1200;
            }
        }
        let (px, pz) = (var1.base.pos_x, var1.base.pos_z);
        let (x, z) = (e.base.pos_x, e.base.pos_z);
        let d = data(e);
        d.push_x = x - px;
        d.push_z = z - pz;
    }
    true
}

/// `riddenByEntity instanceof EntityPlayer` for a rider in the world.
fn rider_is_player(world: &World, rider: i32) -> bool {
    match world.entity(rider) {
        Some(r) => r.kind.is_player(),
        None => {
            world.note_unimplemented("EntityMinecart.interact: rider out of its slot");
            false
        }
    }
}

/// `EntityMinecart.setPositionAndRotation2(x, y, z, yaw, pitch, increments)`
/// (multiplayer): the interpolation target, `increments + 2` steps, and the
/// stored velocity restored.
// src: EntityMinecart.java:793-803
#[allow(clippy::too_many_arguments)]
pub fn set_position_and_rotation2(
    e: &mut Entity,
    var1: f64,
    var3: f64,
    var5: f64,
    var7: f32,
    var8: f32,
    var9: i32,
) {
    let d = data(e);
    d.minecart_x = var1;
    d.minecart_y = var3;
    d.minecart_z = var5;
    d.minecart_yaw = var7 as f64;
    d.minecart_pitch = var8 as f64;
    d.turn_progress = var9 + 2;
    let (vx, vy, vz) = (d.velocity_x, d.velocity_y, d.velocity_z);
    e.base.motion_x = vx;
    e.base.motion_y = vy;
    e.base.motion_z = vz;
}

/// `EntityMinecart.setVelocity(x, y, z)` (multiplayer): motion and the
/// stored velocity.
// src: EntityMinecart.java:805-809
pub fn set_velocity(e: &mut Entity, var1: f64, var3: f64, var5: f64) {
    let d = data(e);
    d.velocity_x = var1;
    d.velocity_y = var3;
    d.velocity_z = var5;
    e.base.motion_x = var1;
    e.base.motion_y = var3;
    e.base.motion_z = var5;
}

/// `canInteractWith(player)` (`IInventory`, the chest GUI): alive and within
/// 8 blocks.
// src: EntityMinecart.java:811-817
pub fn can_interact_with(e: &Entity, player: &Entity) -> bool {
    if e.base.is_dead {
        false
    } else {
        player.base.get_distance_sq_to_entity(&e.base) <= 64.0
    }
}
