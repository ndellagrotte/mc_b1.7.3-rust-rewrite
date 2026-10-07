//! Thrown and shot things (WP-2.6): `ItemBow` (ItemBow.java), `ItemSnowball`
//! (ItemSnowball.java), `ItemEgg` (ItemEgg.java) and `ItemFishingRod`
//! (ItemFishingRod.java).
//!
//! Each new projectile draws its heading jitter from its own `rand` in the
//! constructor, before it joins the world; the oracle's entity seeding
//! cannot reach those draws (tools/oracle/census.md §3), so a fixture can
//! trace what the item does (the stack, the arrow taken from the
//! inventory, the bobber's catch) but not the projectile's flight.

use super::{ItemStack, RightClickResult};
use crate::entity::arrow::new_entity_arrow_shot;
use crate::entity::egg::new_entity_egg_thrown;
use crate::entity::fish::{catch_fish, new_entity_fish};
use crate::entity::snowball::new_entity_snowball_thrown;
use crate::entity::{Entity, player};
use crate::world::World;

/// `Item.arrow.shiftedIndex`.
// src: Item.java:280 (new Item(6): 256 + 6)
const ARROW: i32 = 262;

/// `ItemBow.onItemRightClick`: when the inventory gives up an arrow, the
/// bow sound (pitch from `Item.itemRand`) and a new `EntityArrow(world,
/// player)` joins the world. The stack is returned unchanged (a bow never
/// wears in b1.7.3).
// src: ItemBow.java:9-18
pub fn bow_on_item_right_click(
    var1: &mut ItemStack,
    var2: &mut World,
    var3: &mut Entity,
) -> RightClickResult {
    let _ = var1;
    let consumed = var3
        .kind
        .player_mut()
        .expect("ItemBow: the player is an EntityPlayer")
        .inventory
        .consume_inventory_item(ARROW);
    if consumed {
        let pitch = 1.0 / (var2.item_rand.next_float() * 0.4 + 0.8);
        var2.play_sound_at_entity(&var3.base, "random.bow", 1.0, pitch);
        if !var2.multiplayer_world {
            let a = new_entity_arrow_shot(var2, var3);
            var2.spawn_entity_in_world(a);
        }
    }
    RightClickResult::Same
}

/// `ItemSnowball.onItemRightClick`: one off the stack, the throw sound
/// (`Item.itemRand`) and a new `EntitySnowball(world, player)`.
// src: ItemSnowball.java:9-17
pub fn snowball_on_item_right_click(
    var1: &mut ItemStack,
    var2: &mut World,
    var3: &mut Entity,
) -> RightClickResult {
    var1.stack_size -= 1;
    let pitch = 0.4 / (var2.item_rand.next_float() * 0.4 + 0.8);
    var2.play_sound_at_entity(&var3.base, "random.bow", 0.5, pitch);
    if !var2.multiplayer_world {
        let s = new_entity_snowball_thrown(var2, var3);
        var2.spawn_entity_in_world(s);
    }
    RightClickResult::Same
}

/// `ItemEgg.onItemRightClick`: as the snowball's, with an `EntityEgg`.
// src: ItemEgg.java:9-17
pub fn egg_on_item_right_click(
    var1: &mut ItemStack,
    var2: &mut World,
    var3: &mut Entity,
) -> RightClickResult {
    var1.stack_size -= 1;
    let pitch = 0.4 / (var2.item_rand.next_float() * 0.4 + 0.8);
    var2.play_sound_at_entity(&var3.base, "random.bow", 0.5, pitch);
    if !var2.multiplayer_world {
        let e = new_entity_egg_thrown(var2, var3);
        var2.spawn_entity_in_world(e);
    }
    RightClickResult::Same
}

/// `ItemFishingRod.onItemRightClick`: with a bobber out
/// (`player.fishEntity`), reel it in (`EntityFish.catchFish`) and wear
/// the rod by the result (1 for a fish, 2 from the ground, 3 for a hooked
/// entity); otherwise the cast sound (`Item.itemRand`) and a new
/// `EntityFish(world, player)`, which sets `fishEntity`. The arm swings
/// either way.
// src: ItemFishingRod.java:18-33
pub fn fishing_rod_on_item_right_click(
    var1: &mut ItemStack,
    var2: &mut World,
    var3: &mut Entity,
) -> RightClickResult {
    let fish = var3
        .kind
        .player()
        .expect("ItemFishingRod: the player is an EntityPlayer")
        .fish_entity;
    if let Some(fish) = fish {
        let var4 = match var2.with_entity_mut(fish, |w, f| catch_fish(f, w, var3)) {
            Some(v) => v,
            None => {
                // Java holds the bobber object even after it left
                // loadedEntityList (an unloaded chunk); the port cannot reach
                // it: recorded, and taken as a catch of nothing.
                var2.note_unimplemented("EntityFish.catchFish: bobber not in loadedEntityList");
                if let Some(p) = var3.kind.player_mut() {
                    p.fish_entity = None;
                }
                0
            }
        };
        var1.damage_item_by(var4, var3);
        player::swing_item(var3);
    } else {
        let pitch = 0.4 / (var2.item_rand.next_float() * 0.4 + 0.8);
        var2.play_sound_at_entity(&var3.base, "random.bow", 0.5, pitch);
        if !var2.multiplayer_world {
            let f = new_entity_fish(var2, var3);
            var2.spawn_entity_in_world(f);
        }
        player::swing_item(var3);
    }
    RightClickResult::Same
}
