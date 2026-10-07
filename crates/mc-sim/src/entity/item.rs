//! `EntityItem` (EntityItem.java): a dropped item stack. Fully transcribed
//! (audited line by line in WP-2.4; the pickup in `onCollideWithPlayer`
//! completed in WP-2.H). The NBT bodies are in [`super::nbt`].
//! `canTriggerWalking` is in [`super::dispatch::can_trigger_walking`].

use mc_core::math::floor_double;

use super::{Entity, EntityKindData, base};
use crate::block::{self, Material};
use crate::item::ItemStack;
use crate::stats::achievement_list as al;
use crate::stats::stat;
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

/// `new EntityItem(world)`: the `EntityList` / NBT-load constructor. The
/// field initialisers run (the `hoverStart` `Math.random()` draw, :9), then
/// the size; `item` stays null until `readEntityFromNBT`.
// src: EntityItem.java:3-9, :27-31
pub fn new_entity_item_empty(world: &mut World) -> Entity {
    let mut b = base::EntityBase::new(world);
    // entityInit() is empty (EntityItem.java:33-34).
    let hover_start = (base::math_random(world) * 3.141592653589793 * 2.0) as f32;
    let d = ItemData {
        item: None,
        field_803_e: 0,
        age: 0,
        delay_before_can_pickup: 0,
        health: 5,
        hover_start,
    };
    // src: EntityItem.java:29-30
    b.set_size(0.25, 0.25);
    b.y_offset = b.height / 2.0;
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

/// `Block.wood.blockID` (the log).
// src: Block.java:609
const WOOD: i32 = 17;
/// `Item.leather.shiftedIndex`.
// src: Item.java:352 (new Item(78): 256 + 78)
const LEATHER: i32 = 334;

/// `EntityItem.onCollideWithPlayer(player)`: a player touching the item
/// picks it up once `delayBeforeCanPickup` is 0. The stack goes into
/// `player.inventory` (`addItemStackToInventory`, which lowers
/// `item.stackSize` in place); if anything was taken: the `mineWood` /
/// `killCow` achievements for a log / leather, `random.pop` with two `rand`
/// draws, `onItemPickup` (empty for `EntityPlayer`, [`super::player::on_item_pickup`])
/// with the size read *before* the call, and death once the stack is used up.
///
/// `item.stackSize` is read first (:112), so a null `item` (an
/// `EntityItem(World)` never read from NBT) throws there in Java; recorded
/// here and taken as no pickup.
// src: EntityItem.java:110-130
pub fn on_collide_with_player(e: &mut Entity, world: &mut World, player: &mut Entity) {
    if world.multiplayer_world {
        return;
    }
    let Some(mut stack) = data(e).item else {
        world.note_unimplemented(
            "EntityItem.onCollideWithPlayer with a null item (Java NullPointerException)",
        );
        return;
    };
    // src: EntityItem.java:112 (`var2 = item.stackSize`, read before the call)
    let var2 = stack.stack_size;
    if data(e).delay_before_can_pickup != 0 {
        return;
    }
    // src: EntityItem.java:113 (`&&`: the inventory is only tried at delay 0)
    let taken = player
        .kind
        .player_mut()
        .expect("EntityPlayer.inventory on a non-player (Java ClassCastException)")
        .inventory
        .add_item_stack_to_inventory(&mut stack);
    // `item` is the object the inventory call changed in place.
    data(e).item = Some(stack);
    if taken {
        // src: EntityItem.java:114-120
        if stack.item_id == WOOD {
            super::player::trigger_achievement(player, stat(al::MINE_WOOD));
        }
        if stack.item_id == LEATHER {
            super::player::trigger_achievement(player, stat(al::KILL_COW));
        }
        // src: EntityItem.java:122 (left operand drawn first)
        let pitch = ((e.base.rand.next_float() - e.base.rand.next_float()) * 0.7 + 1.0) * 2.0;
        world.play_sound_at_entity(&e.base, "random.pop", 0.2, pitch);
        // src: EntityItem.java:123
        super::player::on_item_pickup(player, world, e, var2);
        // src: EntityItem.java:124-126 (`setEntityDead` is not overridden on this path)
        if stack.stack_size <= 0 {
            e.base.set_entity_dead();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::player::new_entity_player_sp;
    use crate::save::NoSave;
    use crate::world::WorldEnvironment;
    use crate::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
    use crate::world::events::WorldEvent;
    use crate::world::provider::{Dimension, WorldProvider};

    const DIRT: i32 = 3;
    const STONE: i32 = 1;

    fn world() -> World {
        let mut w = World::new(
            Box::new(NoSave),
            "harness",
            WorldProvider::new(Dimension::Surface),
            123,
            ChunkGenerator::Empty(EmptyChunkProvider),
            WorldEnvironment {
                rand_seed: 0,
                field_9437_g: 0,
                lock_timestamp: 0,
            },
        );
        w.make_deterministic(123);
        w
    }

    fn main(p: &Entity) -> &[Option<ItemStack>; 36] {
        &p.kind.player().unwrap().inventory.main_inventory
    }

    fn pops(w: &mut World) -> Vec<f32> {
        w.events
            .drain()
            .into_iter()
            .filter_map(|e| match e {
                WorldEvent::PlaySound {
                    name,
                    volume,
                    pitch,
                    ..
                } if name == "random.pop" => {
                    assert_eq!(volume, 0.2);
                    Some(pitch)
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn pickup_waits_for_the_delay() {
        let mut w = world();
        let mut p = new_entity_player_sp(&mut w);
        let mut it = new_entity_item(&mut w, 0.5, 61.0, 0.5, ItemStack::new(DIRT, 3, 0));
        data(&mut it).delay_before_can_pickup = 1;
        w.events.drain();
        let rand = it.base.rand.clone();
        on_collide_with_player(&mut it, &mut w, &mut p);
        assert!(main(&p).iter().all(Option::is_none));
        assert!(!it.base.is_dead);
        assert_eq!(it.base.rand, rand);
        assert!(pops(&mut w).is_empty());
    }

    #[test]
    fn whole_stack_is_taken_and_the_item_dies() {
        let mut w = world();
        let mut p = new_entity_player_sp(&mut w);
        let mut it = new_entity_item(&mut w, 0.5, 61.0, 0.5, ItemStack::new(DIRT, 3, 0));
        w.events.drain();
        let mut r = it.base.rand.clone();
        on_collide_with_player(&mut it, &mut w, &mut p);
        assert_eq!(
            main(&p)[0].map(|s| (s.item_id, s.stack_size)),
            Some((DIRT, 3))
        );
        assert_eq!(data(&mut it).item.map(|s| s.stack_size), Some(0));
        assert!(it.base.is_dead);
        let expected = ((r.next_float() - r.next_float()) * 0.7 + 1.0) * 2.0;
        assert_eq!(pops(&mut w), vec![expected]);
        assert!(w.unimplemented_hits.borrow().is_empty());
    }

    /// `addItemStackToInventory` loops while a round moves something and
    /// returns `stackSize < var2` of the *last* round (InventoryPlayer.java:144-150),
    /// so it is true only when the whole stack went in: a pickup that leaves
    /// a rest moves items but returns false (no sound, no draw, no
    /// `onItemPickup`), and the rest stays on the ground.
    #[test]
    fn partial_pickup_keeps_the_rest_silently() {
        for extra_slot in [false, true] {
            let mut w = world();
            let mut p = new_entity_player_sp(&mut w);
            {
                let inv = &mut p.kind.player_mut().unwrap().inventory;
                for s in inv.main_inventory.iter_mut() {
                    *s = Some(ItemStack::new(STONE, 64, 0));
                }
                inv.main_inventory[7] = Some(ItemStack::new(DIRT, 63, 0));
                if extra_slot {
                    inv.main_inventory[9] = Some(ItemStack::new(DIRT, 63, 0));
                }
            }
            let mut it = new_entity_item(&mut w, 0.5, 61.0, 0.5, ItemStack::new(DIRT, 3, 0));
            w.events.drain();
            let rand = it.base.rand.clone();
            on_collide_with_player(&mut it, &mut w, &mut p);
            assert_eq!(main(&p)[7].map(|s| s.stack_size), Some(64));
            let rest = if extra_slot { 1 } else { 2 };
            assert_eq!(data(&mut it).item.map(|s| s.stack_size), Some(rest));
            assert!(!it.base.is_dead);
            assert_eq!(it.base.rand, rand);
            assert!(pops(&mut w).is_empty());
        }
    }

    #[test]
    fn a_log_triggers_mine_wood_once_its_parent_is_unlocked() {
        let mut w = world();
        let mut p = new_entity_player_sp(&mut w);
        let mine_wood = stat(al::MINE_WOOD).unwrap();
        let mut it = new_entity_item(&mut w, 0.5, 61.0, 0.5, ItemStack::new(WOOD, 1, 0));
        on_collide_with_player(&mut it, &mut w, &mut p);
        assert!(it.base.is_dead);
        // EntityPlayerSP.addStat: openInventory (the parent) is not unlocked.
        let sfw = &p.kind.player().unwrap().stat_file_writer;
        assert!(!sfw.has_achievement_unlocked(mine_wood));
        super::super::player::trigger_achievement(&mut p, stat(al::OPEN_INVENTORY));
        let mut it = new_entity_item(&mut w, 0.5, 61.0, 0.5, ItemStack::new(WOOD, 1, 0));
        on_collide_with_player(&mut it, &mut w, &mut p);
        let sfw = &p.kind.player().unwrap().stat_file_writer;
        assert!(sfw.has_achievement_unlocked(mine_wood));
        assert!(!sfw.has_achievement_unlocked(stat(al::KILL_COW).unwrap()));
    }

    #[test]
    fn a_null_item_is_recorded() {
        let mut w = world();
        let mut p = new_entity_player_sp(&mut w);
        let mut it = new_entity_item_empty(&mut w);
        on_collide_with_player(&mut it, &mut w, &mut p);
        assert!(w.unimplemented_hits.borrow().contains_key(
            "EntityItem.onCollideWithPlayer with a null item (Java NullPointerException)"
        ));
        assert!(main(&p).iter().all(Option::is_none));
    }
}
