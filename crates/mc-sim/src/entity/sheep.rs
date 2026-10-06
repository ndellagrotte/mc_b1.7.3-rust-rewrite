//! `EntitySheep` (EntitySheep.java).
//!
//! | EntitySheep.java | Method | Port |
//! |---|---|---|
//! | 6 | `fleeceColorTable` (render data) | [`FLEECE_COLOR_TABLE`] |
//! | 8-12 | constructor | [`new_entity_sheep`] |
//! | 14-17 | `entityInit` (watcher byte 16: colour and sheared bit) | [`new_entity_sheep`] |
//! | 19-21 | `attackEntityFrom` (`super` only) | [`attack_entity_from`] |
//! | 23-28 | `dropFewItems` | [`drop_few_items`] |
//! | 30-32 | `getDropItemId` | [`get_drop_item_id`] |
//! | 34-53 | `interact` (shearing) | [`interact`] |
//! | 55-65 | `writeEntityToNBT`, `readEntityFromNBT` (`Sheared`, `Color`) | [`super::nbt`] |
//! | 67-77 | `getLivingSound`, `getHurtSound`, `getDeathSound` (`"mob.sheep"`) | [`super::dispatch`] |
//! | 79-100 | `getFleeceColor`, `setFleeceColor`, `getSheared`, `setSheared` | the functions of the same names |
//! | 102-115 | `getRandomFleeceColor` (static) | [`get_random_fleece_color`] |
//!
//! b1.7.3 sheep do not eat grass (no such method in EntitySheep.java), and
//! the constructor does not colour the sheep: only `SpawnerAnimals`
//! (SpawnerAnimals.java:167-168, `creatureSpecificInit`) draws
//! [`get_random_fleece_color`] from the world's `rand`; a sheep made any
//! other way (spawner block, `EntityList`, the harness `spawn` op) is white
//! until NBT or a dye (`ItemDye.saddleEntity`, WP-2.6) sets the colour.

use mc_core::java_random::JavaRandom;

use super::data_watcher::WatchValue;
use super::{
    AnimalData, CreatureData, Entity, EntityBase, EntityKindData, LivingData, item, living,
};
use crate::item::ItemStack;
use crate::world::World;

/// `Block.cloth.blockID`.
// src: Block.java:627 (BlockCloth(), id 35: BlockCloth.java:5)
pub const CLOTH: i32 = 35;
/// `Item.shears.shiftedIndex` (`256 + 103`).
// src: Item.java:377
pub const SHEARS: i32 = 256 + 103;

/// `EntitySheep.fleeceColorTable`: the RGB multipliers `RenderSheep` applies
/// to the fleece per colour (RenderSheep.java:16; render data for WP-3.5).
// src: EntitySheep.java:6
pub const FLEECE_COLOR_TABLE: [[f32; 3]; 16] = [
    [1.0, 1.0, 1.0],
    [0.95, 0.7, 0.2],
    [0.9, 0.5, 0.85],
    [0.6, 0.7, 0.95],
    [0.9, 0.9, 0.2],
    [0.5, 0.8, 0.1],
    [0.95, 0.7, 0.8],
    [0.3, 0.3, 0.3],
    [0.6, 0.6, 0.6],
    [0.3, 0.6, 0.7],
    [0.7, 0.4, 0.9],
    [0.2, 0.4, 0.8],
    [0.5, 0.4, 0.3],
    [0.4, 0.5, 0.2],
    [0.8, 0.3, 0.3],
    [0.1, 0.1, 0.1],
];

/// `EntitySheep` declares no instance fields (the colour and the sheared
/// bit are data-watcher byte 16).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SheepData {}

/// `new EntitySheep(world)`: `Entity(World)` with `EntitySheep.entityInit`
/// (`EntityLiving.entityInit` is empty; watcher byte 16 = 0: white, not
/// sheared), the `EntityLiving` initialisers and body (four `Math.random()`
/// draws), the empty `EntityCreature` / `EntityAnimal` levels, then the
/// texture and a 0.9 × 1.3 box (`setSize` does not move the box).
// src: EntitySheep.java:8-17, Entity.java:75-113, EntityLiving.java:6-71
pub fn new_entity_sheep(world: &mut World) -> Entity {
    let mut b = EntityBase::new(world);
    b.data_watcher.add_object(16, WatchValue::Byte(0));
    let mut l = LivingData::new(world, &mut b);
    l.texture = "/mob/sheep.png";
    b.set_size(0.9, 1.3);
    Entity {
        base: b,
        kind: EntityKindData::Sheep(
            l,
            CreatureData::default(),
            AnimalData::default(),
            SheepData::default(),
        ),
    }
}

/// `EntitySheep.attackEntityFrom(attacker, damage)`: `super` only
/// (`EntityLiving`'s; `EntityAnimal` and `EntityCreature` do not override
/// it).
// src: EntitySheep.java:19-21
pub fn attack_entity_from(
    e: &mut Entity,
    world: &mut World,
    var1: Option<&Entity>,
    var2: i32,
) -> bool {
    living::attack_entity_from(e, world, var1, var2)
}

/// `EntitySheep.dropFewItems()`: one wool block of the fleece colour unless
/// sheared (no `rand` draw, unlike `EntityLiving`'s).
// src: EntitySheep.java:23-28
pub fn drop_few_items(e: &mut Entity, world: &mut World) {
    if !get_sheared(e) {
        let c = get_fleece_color(e);
        entity_drop_item(e, world, ItemStack::new(CLOTH, 1, c), 0.0);
    }
}

/// `EntitySheep.getDropItemId()`: wool (never used by vanilla, whose
/// `dropFewItems` override does not call it).
// src: EntitySheep.java:30-32
pub fn get_drop_item_id() -> i32 {
    CLOTH
}

/// `Entity.entityDropItem(stack, offset)`, returning the new `EntityItem`'s
/// id as Java returns the object (`super::base::entity_drop_item` returns
/// nothing): an item at the entity's position raised by `offset`, pick-up
/// delay 10, joined to the world.
// src: Entity.java:881-886
fn entity_drop_item(e: &Entity, world: &mut World, stack: ItemStack, offset: f32) -> i32 {
    let mut var3 = item::new_entity_item(
        world,
        e.base.pos_x,
        e.base.pos_y + offset as f64,
        e.base.pos_z,
        stack,
    );
    if let Some(d) = var3.kind.item_mut() {
        d.delay_before_can_pickup = 10;
    }
    let id = var3.id();
    world.spawn_entity_in_world(var3);
    id
}

/// `EntitySheep.interact(player)`: with shears in hand and a fleece, the
/// sheep is sheared and drops 2 to 4 wool blocks of its colour, 1 block up,
/// each nudged by `rand` (up 0–0.05, sideways ±0.1); the shears take one
/// point of damage. Always returns false, so `useCurrentItemOnEntity` then
/// tries the held item's `useItemOnEntity` (EntityPlayer.java:457-469).
///
/// The held stack is Java's `inventory.getCurrentItem()` object, damaged in
/// place: here the stack in `mainInventory[currentItem]` (the slot
/// `getCurrentItem` read). A stack that breaks stays in the slot at size 0
/// (Java does not remove it here either).
// src: EntitySheep.java:34-53
pub fn interact(e: &mut Entity, world: &mut World, var1: &mut Entity) -> bool {
    let held = var1
        .kind
        .player()
        .and_then(|p| p.inventory.get_current_item())
        .map(|s| s.item_id);
    if held == Some(SHEARS) && !get_sheared(e) {
        if !world.multiplayer_world {
            set_sheared(e, true);
            let var3 = 2 + e.base.rand.next_int_bounded(3);
            for _ in 0..var3 {
                let c = get_fleece_color(e);
                let var5 = entity_drop_item(e, world, ItemStack::new(CLOTH, 1, c), 1.0);
                let r = &mut e.base.rand;
                let dy = (r.next_float() * 0.05) as f64;
                let dx = ((r.next_float() - r.next_float()) * 0.1) as f64;
                let dz = ((r.next_float() - r.next_float()) * 0.1) as f64;
                // Java adds to the returned object, joined or not; a drop the
                // world refused (unloaded chunk) is gone here.
                if let Some(it) = world.entity_mut(var5) {
                    it.base.motion_y += dy;
                    it.base.motion_x += dx;
                    it.base.motion_z += dz;
                }
            }
        }
        // var2.damageItem(1, var1) (ItemStack.java:124-141).
        let broke = var1.kind.player_mut().is_some_and(|p| {
            let slot = p.inventory.current_item as usize;
            p.inventory.main_inventory[slot]
                .as_mut()
                .is_some_and(|s| s.damage_item(1))
        });
        if broke {
            // ItemStack.java:128-130: addStat(objectBreakStats[itemID], 1),
            // EntityPlayerSP's stat writer (client) / WP-2.7.
            world.note_unimplemented("EntityPlayer.addStat (ItemStack.damageItem)");
        }
    }
    false
}

/// `getFleeceColor()`: the low 4 bits of watcher byte 16.
// src: EntitySheep.java:79-81
pub fn get_fleece_color(e: &Entity) -> i32 {
    (e.base.data_watcher.get_watchable_object_byte(16) & 15) as i32
}

/// `setFleeceColor(colour)`: replaces the low 4 bits of watcher byte 16.
// src: EntitySheep.java:83-86
pub fn set_fleece_color(e: &mut Entity, var1: i32) {
    let var2 = e.base.data_watcher.get_watchable_object_byte(16);
    let v = (var2 as i32 & 240 | var1 & 15) as i8;
    e.base.data_watcher.update_object(16, WatchValue::Byte(v));
}

/// `getSheared()`: bit 4 (16) of watcher byte 16.
// src: EntitySheep.java:88-90
pub fn get_sheared(e: &Entity) -> bool {
    (e.base.data_watcher.get_watchable_object_byte(16) & 16) != 0
}

/// `setSheared(sheared)`: sets or clears bit 4 (16) of watcher byte 16.
// src: EntitySheep.java:92-100
pub fn set_sheared(e: &mut Entity, var1: bool) {
    let var2 = e.base.data_watcher.get_watchable_object_byte(16);
    let v = if var1 {
        (var2 as i32 | 16) as i8
    } else {
        (var2 as i32 & -17) as i8
    };
    e.base.data_watcher.update_object(16, WatchValue::Byte(v));
}

/// `EntitySheep.getRandomFleeceColor(random)`: black 5 %, gray 5 %, light
/// gray 5 %, brown 3 %, else pink with a second draw of 1 in 500, white
/// otherwise (one `nextInt(100)`, then `nextInt(500)` only on the last
/// branch).
// src: EntitySheep.java:102-115
pub fn get_random_fleece_color(var0: &mut JavaRandom) -> i32 {
    let var1 = var0.next_int_bounded(100);
    if var1 < 5 {
        15
    } else if var1 < 10 {
        7
    } else if var1 < 15 {
        8
    } else if var1 < 18 {
        12
    } else if var0.next_int_bounded(500) == 0 {
        6
    } else {
        0
    }
}
