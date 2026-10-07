//! Items that place entities (WP-2.6): `ItemBoat` (ItemBoat.java),
//! `ItemMinecart` (ItemMinecart.java) and `ItemPainting` (ItemPainting.java).
//!
//! The painting's constructor draws its art from the painting's own `rand`
//! before it joins the world, which the oracle cannot seed
//! (tools/oracle/census.md §3): the art is only reproducible where a single
//! size of motive fits (entity lines do not carry the art, and every motive
//! of one size gets the same box).

use super::{ItemStack, RightClickResult, player_look_trace};
use crate::block::rail::is_rail_block;
use crate::entity::Entity;
use crate::entity::boat::new_entity_boat;
use crate::entity::minecart::new_entity_minecart;
use crate::entity::painting::{new_entity_painting, on_valid_surface};
use crate::world::World;
use crate::world::ray_trace::EnumMovingObjectType;

/// `Block.snow.blockID` (the snow layer).
// src: Block.java:670
const SNOW: i32 = 78;

/// `ItemBoat.onItemRightClick`: the eye ray (five blocks, stopping on
/// liquids) hits a block: a new `EntityBoat` on top of it (one lower on a
/// snow layer), centred on the block, and one off the stack.
// src: ItemBoat.java:9-46
pub fn boat_on_item_right_click(
    var1: &mut ItemStack,
    var2: &mut World,
    var3: &mut Entity,
) -> RightClickResult {
    let (_, var24) = player_look_trace(var2, var3, true);
    let Some(var24) = var24 else {
        return RightClickResult::Same;
    };
    if var24.type_of_hit == EnumMovingObjectType::Tile {
        let var25 = var24.block_x;
        let mut var26 = var24.block_y;
        let var27 = var24.block_z;
        if !var2.multiplayer_world {
            if var2.get_block_id(var25, var26, var27) == SNOW {
                var26 -= 1;
            }
            let b = new_entity_boat(
                var2,
                (var25 as f32 + 0.5) as f64,
                (var26 as f32 + 1.0) as f64,
                (var27 as f32 + 0.5) as f64,
            );
            var2.spawn_entity_in_world(b);
        }
        var1.stack_size -= 1;
    }
    RightClickResult::Same
}

/// `ItemMinecart.onItemUse`: on any rail, a new `EntityMinecart` of the
/// item's type at the block's centre and one off the stack.
// src: ItemMinecart.java:12-24
pub fn minecart_on_item_use(
    minecart_type: i32,
    var1: &mut ItemStack,
    var3: &mut World,
    var4: i32,
    var5: i32,
    var6: i32,
) -> bool {
    let var8 = var3.get_block_id(var4, var5, var6);
    if is_rail_block(var8) {
        if !var3.multiplayer_world {
            let m = new_entity_minecart(
                var3,
                (var4 as f32 + 0.5) as f64,
                (var5 as f32 + 0.5) as f64,
                (var6 as f32 + 0.5) as f64,
                minecart_type,
            );
            var3.spawn_entity_in_world(m);
        }
        var1.stack_size -= 1;
        true
    } else {
        false
    }
}

/// `ItemPainting.onItemUse`: only on a side face; a new `EntityPainting`
/// hanging on the block, facing out of the face (direction 0 for the −z
/// face, 1 −x, 2 +z, 3 +x); when it has a valid surface it joins the world
/// and one comes off the stack. True on every side face.
// src: ItemPainting.java:8-38
pub fn painting_on_item_use(
    var1: &mut ItemStack,
    var3: &mut World,
    var4: i32,
    var5: i32,
    var6: i32,
    var7: i32,
) -> bool {
    if var7 == 0 || var7 == 1 {
        return false;
    }
    let mut var8 = 0;
    if var7 == 4 {
        var8 = 1;
    }
    if var7 == 3 {
        var8 = 2;
    }
    if var7 == 5 {
        var8 = 3;
    }
    let var9 = new_entity_painting(var3, var4, var5, var6, var8);
    if on_valid_surface(&var9, var3) {
        if !var3.multiplayer_world {
            var3.spawn_entity_in_world(var9);
        }
        var1.stack_size -= 1;
    }
    true
}
