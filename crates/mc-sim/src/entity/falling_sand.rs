//! `EntityFallingSand` (EntityFallingSand.java): a falling sand or gravel
//! block. Fully transcribed: both constructors, `canTriggerWalking` (in
//! [`super::dispatch::can_trigger_walking`]), `entityInit` (empty),
//! `canBeCollidedWith`, `onUpdate`, `writeEntityToNBT` / `readEntityFromNBT`.
//! `getShadowSize` (render) and `getWorld` (an accessor) have no port.
//!
//! The class draws no random numbers: neither `Math.random()` nor its
//! `rand` (the drop path's `new EntityItem` draws `Math.random()`, as every
//! item drop does).

use std::collections::BTreeMap;

use mc_core::math::floor_double;
use mc_nbt::Tag;

use super::{Entity, EntityBase, EntityKindData, base, dispatch};
use crate::block::sand;
use crate::world::World;

/// The fields `EntityFallingSand` declares.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FallingSandData {
    /// `blockID`: the block it carries (0 makes it die on its next update).
    // src: EntityFallingSand.java:4
    pub block_id: i32,
    /// `fallTime`: updates so far; at more than 100 in the air it drops as an
    /// item.
    // src: EntityFallingSand.java:5
    pub fall_time: i32,
}

/// `new EntityFallingSand(world)`: the `EntityList` / NBT-load constructor.
/// Only `Entity(World)` runs: default size 0.6 x 1.8, `blockID` 0 until
/// `readEntityFromNBT`.
// src: EntityFallingSand.java:7-9
pub fn new_entity_falling_sand_empty(world: &mut World) -> Entity {
    let b = EntityBase::new(world);
    // entityInit() is empty (EntityFallingSand.java:30-31).
    Entity {
        base: b,
        kind: EntityKindData::FallingSand(FallingSandData::default()),
    }
}

/// `new EntityFallingSand(world, x, y, z, blockId)`: centred on `(x, y, z)`
/// (`yOffset` is half the height), at rest, `prevPos` = `pos`.
// src: EntityFallingSand.java:11-24
pub fn new_entity_falling_sand(world: &mut World, x: f64, y: f64, z: f64, block_id: i32) -> Entity {
    let mut b = EntityBase::new(world);
    // entityInit() is empty (EntityFallingSand.java:30-31). Field initialiser
    // `fallTime = 0` (:5), then `this.blockID = var8` (:13).
    let d = FallingSandData {
        block_id,
        fall_time: 0,
    };
    // src: EntityFallingSand.java:14-23
    b.prevent_entity_spawning = true;
    b.set_size(0.98, 0.98);
    b.y_offset = b.height / 2.0;
    b.set_position(x, y, z);
    b.motion_x = 0.0;
    b.motion_y = 0.0;
    b.motion_z = 0.0;
    b.prev_pos_x = x;
    b.prev_pos_y = y;
    b.prev_pos_z = z;
    Entity {
        base: b,
        kind: EntityKindData::FallingSand(d),
    }
}

fn data(e: &mut Entity) -> &mut FallingSandData {
    e.kind
        .falling_sand_mut()
        .expect("EntityFallingSand method on another kind")
}

/// `EntityFallingSand.canBeCollidedWith()`: while alive.
// src: EntityFallingSand.java:33-35
pub fn can_be_collided_with(e: &Entity) -> bool {
    !e.base.is_dead
}

/// `EntityFallingSand.onUpdate()`.
///
/// No `super.onUpdate()`: `Entity.onEntityUpdate` never runs, so
/// `ticksExisted` stays 0 and there is no fire, water or void handling.
// src: EntityFallingSand.java:37-71
pub fn on_update(e: &mut Entity, world: &mut World) {
    let block_id = data(e).block_id;
    // src: EntityFallingSand.java:38-39
    if block_id == 0 {
        dispatch::set_entity_dead(e, world);
        return;
    }
    // src: EntityFallingSand.java:41-45
    let b = &mut e.base;
    b.prev_pos_x = b.pos_x;
    b.prev_pos_y = b.pos_y;
    b.prev_pos_z = b.pos_z;
    data(e).fall_time += 1;
    let b = &mut e.base;
    b.motion_y -= 0.03999999910593033;
    // src: EntityFallingSand.java:46 (virtual `moveEntity`)
    let (mx, my, mz) = (b.motion_x, b.motion_y, b.motion_z);
    dispatch::move_entity(e, world, mx, my, mz);
    // src: EntityFallingSand.java:47-49
    let b = &mut e.base;
    b.motion_x *= 0.9800000190734863;
    b.motion_y *= 0.9800000190734863;
    b.motion_z *= 0.9800000190734863;
    // src: EntityFallingSand.java:50-52
    let var1 = floor_double(b.pos_x);
    let var2 = floor_double(b.pos_y);
    let var3 = floor_double(b.pos_z);
    // src: EntityFallingSand.java:53-55: removes the block it came from
    if world.get_block_id(var1, var2, var3) == block_id {
        world.set_block_with_notify(var1, var2, var3, 0);
    }

    if e.base.on_ground {
        // src: EntityFallingSand.java:57-61
        let b = &mut e.base;
        b.motion_x *= 0.699999988079071;
        b.motion_z *= 0.699999988079071;
        b.motion_y *= -0.5;
        dispatch::set_entity_dead(e, world);
        // src: EntityFallingSand.java:62-64: `||` short-circuits, so the block
        // is only placed when the two checks pass.
        if (!world.can_block_be_placed_at(block_id, var1, var2, var3, true, 1)
            || sand::can_fall_below(world, var1, var2 - 1, var3)
            || !world.set_block_with_notify(var1, var2, var3, block_id))
            && !world.multiplayer_world
        {
            base::drop_item(e, world, block_id, 1);
        }
    } else if data(e).fall_time > 100 && !world.multiplayer_world {
        // src: EntityFallingSand.java:65-68
        base::drop_item(e, world, block_id, 1);
        dispatch::set_entity_dead(e, world);
    }
}

/// `EntityFallingSand.writeEntityToNBT(compound)`: `Tile` as a byte.
// src: EntityFallingSand.java:73-75
pub fn write_entity_to_nbt(d: &FallingSandData, compound: &mut BTreeMap<String, Tag>) {
    compound.insert("Tile".to_string(), Tag::Byte(d.block_id as i8));
}

/// `EntityFallingSand.readEntityFromNBT(compound)`: `Tile & 255`; a missing
/// tag reads as 0 (`NBTTagCompound.getByte`, NBTTagCompound.java:91-93; Java
/// would throw on a non-byte tag, read here as 0 too).
// src: EntityFallingSand.java:77-79
pub fn read_entity_from_nbt(d: &mut FallingSandData, compound: &BTreeMap<String, Tag>) {
    let v = match compound.get("Tile") {
        Some(Tag::Byte(b)) => *b,
        _ => 0,
    };
    d.block_id = v as i32 & 255;
}
