//! `EntityCow` (EntityCow.java).
//!
//! | EntityCow.java | Method | Port |
//! |---|---|---|
//! | 4-8 | constructor | [`new_entity_cow`] |
//! | 10-16 | `writeEntityToNBT`, `readEntityFromNBT` (`super` only) | [`super::nbt`] |
//! | 18-32 | `getLivingSound`, `getHurtSound`, `getDeathSound`, `getSoundVolume` (0.4) | [`super::dispatch`] |
//! | 34-36 | `getDropItemId` (leather) | [`get_drop_item_id`] |
//! | 38-46 | `interact` (milking) | [`interact`] |
//!
//! Also here: `InventoryPlayer.setInventorySlotContents`
//! ([`set_inventory_slot_contents`]), which the milking and
//! `EntityWolf.interact` write through; `crate::inventory` has no setter yet
//! (WP-2.5 owns it).

use super::{AnimalData, CreatureData, Entity, EntityBase, EntityKindData, LivingData};
use crate::inventory::InventoryPlayer;
use crate::item::ItemStack;
use crate::world::World;

/// `Item.leather.shiftedIndex` (`256 + 78`).
// src: Item.java:352
pub const LEATHER: i32 = 256 + 78;
/// `Item.bucketEmpty.shiftedIndex` (`256 + 69`).
// src: Item.java:343
pub const BUCKET_EMPTY: i32 = 256 + 69;
/// `Item.bucketMilk.shiftedIndex` (`256 + 79`).
// src: Item.java:353
pub const BUCKET_MILK: i32 = 256 + 79;

/// `EntityCow` declares no fields.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CowData {}

/// `new EntityCow(world)`: `Entity(World)` (`entityInit` is `EntityLiving`'s,
/// empty), the `EntityLiving` initialisers and body (four `Math.random()`
/// draws), the empty `EntityCreature` / `EntityAnimal` levels, then the
/// texture and a 0.9 × 1.3 box (`setSize` does not move the box).
// src: EntityCow.java:4-8, Entity.java:75-113, EntityLiving.java:6-71
pub fn new_entity_cow(world: &mut World) -> Entity {
    let mut b = EntityBase::new(world);
    let mut l = LivingData::new(world, &mut b);
    l.texture = "/mob/cow.png";
    b.set_size(0.9, 1.3);
    Entity {
        base: b,
        kind: EntityKindData::Cow(
            l,
            CreatureData::default(),
            AnimalData::default(),
            CowData::default(),
        ),
    }
}

/// `EntityCow.getDropItemId()`: leather.
// src: EntityCow.java:34-36
pub fn get_drop_item_id() -> i32 {
    LEATHER
}

/// `EntityCow.interact(player)`: an empty bucket in hand becomes a milk
/// bucket (`setInventorySlotContents(currentItem, new
/// ItemStack(Item.bucketMilk))`: a new stack of 1, whatever the size of the
/// held stack of buckets) and returns true; anything else returns false.
// src: EntityCow.java:38-46
pub fn interact(e: &mut Entity, world: &mut World, var1: &mut Entity) -> bool {
    let _ = (e, world);
    let Some(p) = var1.kind.player_mut() else {
        return false;
    };
    let var2 = p.inventory.get_current_item().map(|s| s.item_id);
    if var2 == Some(BUCKET_EMPTY) {
        let slot = p.inventory.current_item;
        // src: ItemStack.java:13-15, :33-38 (new ItemStack(Item): count 1,
        // damage 0)
        set_inventory_slot_contents(
            &mut p.inventory,
            slot,
            Some(ItemStack::new(BUCKET_MILK, 1, 0)),
        );
        true
    } else {
        false
    }
}

/// `InventoryPlayer.setInventorySlotContents(slot, stack)`: slots from 36
/// are the armour slots. Panics where Java throws
/// `ArrayIndexOutOfBoundsException` (a slot outside 0..=39).
// src: InventoryPlayer.java:180-188
pub(crate) fn set_inventory_slot_contents(
    inv: &mut InventoryPlayer,
    var1: i32,
    var2: Option<ItemStack>,
) {
    let main_len = inv.main_inventory.len() as i32;
    let slot = if var1 >= main_len {
        usize::try_from(var1 - main_len)
            .ok()
            .and_then(|i| inv.armor_inventory.get_mut(i))
    } else {
        usize::try_from(var1)
            .ok()
            .and_then(|i| inv.main_inventory.get_mut(i))
    };
    match slot {
        Some(s) => *s = var2,
        None => panic!(
            "InventoryPlayer.setInventorySlotContents({var1}): ArrayIndexOutOfBoundsException"
        ),
    }
}
