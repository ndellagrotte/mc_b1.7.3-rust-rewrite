//! `EntityItem` (EntityItem.java): a dropped item stack. Fully transcribed
//! except `onCollideWithPlayer` (needs the player inventory, WP-2.5) and NBT.

use mc_core::math::floor_double;

use super::{Entity, EntityKindData, base};
use crate::block::{self, Material};
use crate::item::ItemStack;
use crate::world::World;

/// The fields `EntityItem` declares (EntityItem.java:4-9).
#[derive(Clone, Debug, PartialEq)]
pub struct ItemData {
    /// `item` (null only for an `EntityItem(World)` not yet read from NBT).
    pub item: Option<ItemStack>,
    /// `field_803_e` (private): counts ticks, never read. Name kept.
    pub field_803_e: i32,
    /// `age`: despawns at 6000.
    pub age: i32,
    /// `delayBeforeCanPickup`.
    pub delay_before_can_pickup: i32,
    /// `health` (private): 5, the item burns up at 0.
    pub health: i32,
    /// `hoverStart`: the bobbing phase (render only), drawn from `Math.random()`.
    pub hover_start: f32,
}

/// `new EntityItem(world, x, y, z, stack)`.
///
/// Draw order on the `Math.random()` stream: `hoverStart` (field
/// initialiser, :9), then `rotationYaw` (:17), `motionX` (:18), `motionZ` (:20).
// src: EntityItem.java:3-21
pub fn new_entity_item(world: &mut World, x: f64, y: f64, z: f64, stack: ItemStack) -> Entity {
    let mut b = base::EntityBase::new(world);
    // entityInit() is empty (EntityItem.java:33-34).
    // Field initialisers.
    let hover_start = (base::math_random(world) * 3.141592653589793 * 2.0) as f32;
    let mut d = ItemData {
        item: None,
        field_803_e: 0,
        age: 0,
        delay_before_can_pickup: 0,
        health: 5,
        hover_start,
    };
    // Constructor body.
    b.set_size(0.25, 0.25);
    b.y_offset = b.height / 2.0;
    b.set_position(x, y, z);
    d.item = Some(stack);
    b.rotation_yaw = (base::math_random(world) * 360.0) as f32;
    b.motion_x =
        (base::math_random(world) * 0.20000000298023224 - 0.10000000149011612) as f32 as f64;
    b.motion_y = 0.20000000298023224;
    b.motion_z =
        (base::math_random(world) * 0.20000000298023224 - 0.10000000149011612) as f32 as f64;
    Entity {
        base: b,
        kind: EntityKindData::Item(d),
    }
}

fn data(e: &mut Entity) -> &mut ItemData {
    e.kind
        .item_mut()
        .expect("EntityItem method on another kind")
}

/// `EntityItem.onUpdate()`.
// src: EntityItem.java:36-77
pub fn on_update(e: &mut Entity, world: &mut World) {
    base::on_update(e, world);
    {
        let d = data(e);
        if d.delay_before_can_pickup > 0 {
            d.delay_before_can_pickup -= 1;
        }
    }
    let b = &mut e.base;
    b.prev_pos_x = b.pos_x;
    b.prev_pos_y = b.pos_y;
    b.prev_pos_z = b.pos_z;
    b.motion_y -= 0.03999999910593033;
    if world.get_block_material(
        floor_double(b.pos_x),
        floor_double(b.pos_y),
        floor_double(b.pos_z),
    ) == Material::Lava
    {
        b.motion_y = 0.20000000298023224;
        b.motion_x = ((b.rand.next_float() - b.rand.next_float()) * 0.2) as f64;
        b.motion_z = ((b.rand.next_float() - b.rand.next_float()) * 0.2) as f64;
        let pitch = 2.0 + b.rand.next_float() * 0.4;
        world.play_sound_at_entity(&e.base, "random.fizz", 0.4, pitch);
    }

    let (px, py, pz) = (
        e.base.pos_x,
        (e.base.bounding_box.min_y + e.base.bounding_box.max_y) / 2.0,
        e.base.pos_z,
    );
    base::push_out_of_blocks(e, world, px, py, pz);
    let (mx, my, mz) = (e.base.motion_x, e.base.motion_y, e.base.motion_z);
    super::dispatch::move_entity(e, world, mx, my, mz);
    let mut var1: f32 = 0.98;
    if e.base.on_ground {
        var1 = 0.58800006;
        let var2 = world.get_block_id(
            floor_double(e.base.pos_x),
            floor_double(e.base.bounding_box.min_y) - 1,
            floor_double(e.base.pos_z),
        );
        if var2 > 0
            && let Some(bd) = block::block(var2)
        {
            var1 = bd.slipperiness * 0.98;
        }
    }

    let b = &mut e.base;
    b.motion_x *= var1 as f64;
    b.motion_y *= 0.9800000190734863;
    b.motion_z *= var1 as f64;
    if b.on_ground {
        b.motion_y *= -0.5;
    }

    let d = data(e);
    d.field_803_e += 1;
    d.age += 1;
    if d.age >= 6000 {
        // `this.setEntityDead()`: virtual, but not overridden on this path.
        e.base.set_entity_dead();
    }
}

/// `EntityItem.handleWaterMovement()`: the whole box, not shrunk.
// src: EntityItem.java:79-81
pub fn handle_water_movement(e: &mut Entity, world: &mut World) -> bool {
    let bb = e.base.bounding_box;
    world.handle_material_acceleration(&bb, Material::Water, e)
}

/// `EntityItem.dealFireDamage(amount)`: ignores fire immunity.
// src: EntityItem.java:83-85
pub fn deal_fire_damage(e: &mut Entity, world: &mut World, amount: i32) {
    super::dispatch::attack_entity_from(e, world, None, amount);
}

/// `EntityItem.attackEntityFrom(attacker, damage)`.
// src: EntityItem.java:87-95
pub fn attack_entity_from(
    e: &mut Entity,
    _world: &mut World,
    _attacker: Option<&Entity>,
    damage: i32,
) -> bool {
    e.base.set_been_attacked();
    let d = data(e);
    d.health -= damage;
    if d.health <= 0 {
        e.base.set_entity_dead();
    }
    false
}
