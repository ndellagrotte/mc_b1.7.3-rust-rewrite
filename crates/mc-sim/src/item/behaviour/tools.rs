//! Tools (WP-2.6): `ItemTool` (ItemTool.java, the spade, pickaxe and axe),
//! `ItemSword` (ItemSword.java), `ItemShears` (ItemShears.java), `ItemHoe`
//! (ItemHoe.java) and `ItemFlintAndSteel` (ItemFlintAndSteel.java).
//!
//! Wear goes through `ItemStack.damageItem(n, entity)`
//! ([`ItemStack::damage_item_by`]): past the maximum the stack loses one
//! item (a size-0 stack is then removed by the caller) and a player gets
//! the break stat.

use super::{ItemStack, offset_by_side, play_place_sound};
use crate::block;
use crate::entity::Entity;
use crate::world::World;

/// `Block.leaves.blockID`.
// src: Block.java:610
const LEAVES: i32 = 18;
/// `Block.web.blockID`.
// src: Block.java:622
const WEB: i32 = 30;
/// `Block.grass.blockID`.
// src: Block.java:594
const GRASS: i32 = 2;
/// `Block.dirt.blockID`.
// src: Block.java:595
const DIRT: i32 = 3;
/// `Block.tilledField.blockID`.
// src: Block.java:652
const TILLED_FIELD: i32 = 60;
/// `Block.fire.blockID`.
// src: Block.java:643
const FIRE: i32 = 51;

/// `ItemTool.hitEntity`: two points of wear; true.
// src: ItemTool.java:29-32
pub fn tool_hit_entity(var1: &mut ItemStack, var3: &mut Entity) -> bool {
    var1.damage_item_by(2, var3);
    true
}

/// `ItemTool.onBlockDestroyed`: one point of wear (whatever the block);
/// true.
// src: ItemTool.java:34-37
pub fn tool_on_block_destroyed(var1: &mut ItemStack, var6: &mut Entity) -> bool {
    var1.damage_item_by(1, var6);
    true
}

/// `ItemSword.hitEntity`: one point of wear; true.
// src: ItemSword.java:17-20
pub fn sword_hit_entity(var1: &mut ItemStack, var3: &mut Entity) -> bool {
    var1.damage_item_by(1, var3);
    true
}

/// `ItemSword.onBlockDestroyed`: two points of wear (whatever the block);
/// true.
// src: ItemSword.java:22-25
pub fn sword_on_block_destroyed(var1: &mut ItemStack, var6: &mut Entity) -> bool {
    var1.damage_item_by(2, var6);
    true
}

/// `ItemShears.onBlockDestroyed`: one point of wear on leaves and webs,
/// then `super` (`Item`'s false: no use stat).
// src: ItemShears.java:10-16
pub fn shears_on_block_destroyed(var1: &mut ItemStack, var2: i32, var6: &mut Entity) -> bool {
    if var2 == LEAVES || var2 == WEB {
        var1.damage_item_by(1, var6);
    }
    // super.onBlockDestroyed: Item.java:201-203
    false
}

/// `ItemHoe.onItemUse`: grass with air above, clicked on any face but the
/// bottom, or dirt in any case, becomes farmland: the farmland's step
/// sound, the block (not on a multiplayer client) and one point of wear.
// src: ItemHoe.java:10-26
#[allow(clippy::too_many_arguments)]
pub fn hoe_on_item_use(
    var1: &mut ItemStack,
    var2: &mut Entity,
    var3: &mut World,
    var4: i32,
    var5: i32,
    var6: i32,
    var7: i32,
) -> bool {
    let var8 = var3.get_block_id(var4, var5, var6);
    let var9 = var3.get_block_id(var4, var5 + 1, var6);
    // vanilla bug: the parenthesisation lets dirt through regardless of the
    // face clicked and the block above (only grass checks them).
    if (var7 == 0 || var9 != 0 || var8 != GRASS) && var8 != DIRT {
        return false;
    }
    let var10 = block::block(TILLED_FIELD).expect("Block.tilledField");
    play_place_sound(var3, var10, var4, var5, var6);
    if var3.multiplayer_world {
        return true;
    }
    var3.set_block_with_notify(var4, var5, var6, TILLED_FIELD);
    var1.damage_item_by(1, var2);
    true
}

/// `ItemFlintAndSteel.onItemUse`: fire in the cell in front of the face if
/// it is air (with the ignite sound, pitch from `Item.itemRand`), and one
/// point of wear either way; always true. The fire block's `onBlockAdded`
/// lights a portal in an obsidian frame.
// src: ItemFlintAndSteel.java:10-43
#[allow(clippy::too_many_arguments)]
pub fn flint_and_steel_on_item_use(
    var1: &mut ItemStack,
    var2: &mut Entity,
    var3: &mut World,
    var4: i32,
    var5: i32,
    var6: i32,
    var7: i32,
) -> bool {
    let (var4, var5, var6) = offset_by_side(var7, var4, var5, var6);
    let var8 = var3.get_block_id(var4, var5, var6);
    if var8 == 0 {
        let pitch = var3.item_rand.next_float() * 0.4 + 0.8;
        var3.play_sound_effect(
            var4 as f64 + 0.5,
            var5 as f64 + 0.5,
            var6 as f64 + 0.5,
            "fire.ignite",
            1.0,
            pitch,
        );
        var3.set_block_with_notify(var4, var5, var6, FIRE);
    }
    var1.damage_item_by(1, var2);
    true
}
