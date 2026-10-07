//! The items that place blocks (WP-2.6): `ItemBlock` (ItemBlock.java) with
//! its subclasses, `ItemReed` (ItemReed.java), `ItemDoor` (ItemDoor.java),
//! `ItemBed` (ItemBed.java), `ItemSign` (ItemSign.java), `ItemRedstone`
//! (ItemRedstone.java) and `ItemSeeds` (ItemSeeds.java).
//!
//! `ItemCloth`, `ItemLog`, `ItemSlab`, `ItemSapling`, `ItemLeaves` and
//! `ItemPiston` inherit `ItemBlock.onItemUse` and differ only in
//! `getPlacedBlockMetadata` ([`ItemDef::get_placed_block_metadata`]): the
//! stack's damage, the damage with the no-decay bit for leaves, 7 for the
//! pistons (whose `onBlockPlacedBy` then sets the facing). The block-specific
//! parts of a placement (stairs, slabs merging into a double slab, torches,
//! levers, buttons, ladders, repeaters, …) are the block's `onBlockAdded`,
//! `onBlockPlaced` and `onBlockPlacedBy` in [`crate::block::dispatch`].

use mc_core::math::floor_double;

use super::{ItemDef, ItemStack, can_block_be_placed_at, offset_by_side, play_place_sound};
use crate::block::{self, Material, dispatch as block_dispatch};
use crate::entity::Entity;
use crate::world::World;

/// `Block.snow.blockID` (the snow layer).
// src: Block.java:670
const SNOW: i32 = 78;
/// `Block.doorWood.blockID`.
// src: Block.java:656
const DOOR_WOOD: i32 = 64;
/// `Block.doorSteel.blockID`.
// src: Block.java:663
const DOOR_STEEL: i32 = 71;
/// `Block.blockBed.blockID`.
// src: Block.java:618
const BED: i32 = 26;
/// `Block.signPost.blockID`.
// src: Block.java:655
const SIGN_POST: i32 = 63;
/// `Block.signWall.blockID`.
// src: Block.java:660
const SIGN_WALL: i32 = 68;
/// `Block.redstoneWire.blockID`.
// src: Block.java:647
const REDSTONE_WIRE: i32 = 55;
/// `Block.tilledField.blockID`.
// src: Block.java:652
const TILLED_FIELD: i32 = 60;

/// `ItemBlock.onItemUse` (inherited by `ItemCloth`, `ItemLog`, `ItemSlab`,
/// `ItemSapling`, `ItemLeaves`, `ItemPiston`): on a snow layer the layer
/// itself is the target and the side becomes 0; otherwise the cell in
/// front of the face. Refused for an empty stack, for a solid block at
/// y = 127, and where `canBlockBePlacedAt` says no. Then the block and the
/// metadata from `getPlacedBlockMetadata(damage)` with notification; when
/// that changed the world, `onBlockPlaced(side)`, `onBlockPlacedBy(player)`,
/// the step sound and one item off the stack. True once
/// `canBlockBePlacedAt` agreed, even if nothing changed.
// src: ItemBlock.java:12-58
// Java's branch-per-refusal shape is kept.
#[allow(clippy::too_many_arguments, clippy::if_same_then_else)]
pub fn item_block_on_item_use(
    def: &ItemDef,
    block_id: i32,
    var1: &mut ItemStack,
    var2: &mut Entity,
    var3: &mut World,
    mut var4: i32,
    mut var5: i32,
    mut var6: i32,
    mut var7: i32,
) -> bool {
    if var3.get_block_id(var4, var5, var6) == SNOW {
        var7 = 0;
    } else {
        (var4, var5, var6) = offset_by_side(var7, var4, var5, var6);
    }
    let var8 = block::block(block_id).expect("ItemBlock of an existing block");
    if var1.stack_size == 0 {
        false
    } else if var5 == 127 && var8.block_material.is_solid() {
        false
    } else if can_block_be_placed_at(var3, var2, block_id, var4, var5, var6, false, var7) {
        if var3.set_block_and_metadata_with_notify(
            var4,
            var5,
            var6,
            block_id,
            def.get_placed_block_metadata(var1.get_item_damage()),
        ) {
            block_dispatch::on_block_placed(var3, block_id, var4, var5, var6, var7);
            block_dispatch::on_block_placed_by(var3, block_id, var4, var5, var6, var2);
            play_place_sound(var3, var8, var4, var5, var6);
            var1.stack_size -= 1;
        }
        true
    } else {
        false
    }
}

/// `ItemReed.onItemUse` (sugar cane, cake, repeater: an item that places
/// `spawnID`): as `ItemBlock`'s but with `setBlockWithNotify` (metadata 0),
/// no y = 127 test, and true for any non-empty stack, placed or not.
// src: ItemReed.java:11-55
#[allow(clippy::too_many_arguments)]
pub fn reed_on_item_use(
    spawn_id: i32,
    var1: &mut ItemStack,
    var2: &mut Entity,
    var3: &mut World,
    mut var4: i32,
    mut var5: i32,
    mut var6: i32,
    mut var7: i32,
) -> bool {
    if var3.get_block_id(var4, var5, var6) == SNOW {
        var7 = 0;
    } else {
        (var4, var5, var6) = offset_by_side(var7, var4, var5, var6);
    }
    if var1.stack_size == 0 {
        return false;
    }
    if can_block_be_placed_at(var3, var2, spawn_id, var4, var5, var6, false, var7) {
        let var8 = block::block(spawn_id).expect("ItemReed's spawnID is a block");
        if var3.set_block_with_notify(var4, var5, var6, spawn_id) {
            block_dispatch::on_block_placed(var3, spawn_id, var4, var5, var6, var7);
            block_dispatch::on_block_placed_by(var3, spawn_id, var4, var5, var6, var2);
            play_place_sound(var3, var8, var4, var5, var6);
            var1.stack_size -= 1;
        }
    }
    true
}

/// `ItemDoor.onItemUse`: only on a top face; the door's
/// `canPlaceBlockAt` at the cell above; the facing from the player's yaw,
/// mirrored (+4, the hinge on the other side) when a door of the same
/// material stands only on the left, or the right side has more normal
/// cubes than the left. Both halves are set with `editingBlocks` on, then
/// the neighbours of both are notified.
// src: ItemDoor.java:12-72
// Java's branch-per-condition shape is kept.
#[allow(clippy::too_many_arguments, clippy::if_same_then_else)]
pub fn door_on_item_use(
    door_material: Material,
    var1: &mut ItemStack,
    var2: &mut Entity,
    var3: &mut World,
    var4: i32,
    mut var5: i32,
    var6: i32,
    var7: i32,
) -> bool {
    if var7 != 1 {
        return false;
    }
    var5 += 1;
    let var8 = if door_material == Material::Wood {
        DOOR_WOOD
    } else {
        DOOR_STEEL
    };
    if !block_dispatch::can_place_block_at(var3, var8, var4, var5, var6) {
        return false;
    }
    let mut var9 = floor_double(((var2.base.rotation_yaw + 180.0) * 4.0 / 360.0) as f64 - 0.5) & 3;
    let mut var10: i32 = 0;
    let mut var11: i32 = 0;
    if var9 == 0 {
        var11 = 1;
    }
    if var9 == 1 {
        var10 = -1;
    }
    if var9 == 2 {
        var11 = -1;
    }
    if var9 == 3 {
        var10 = 1;
    }
    let nc = |w: &mut World, x: i32, y: i32, z: i32| i32::from(w.is_block_normal_cube(x, y, z));
    let var12 =
        nc(var3, var4 - var10, var5, var6 - var11) + nc(var3, var4 - var10, var5 + 1, var6 - var11);
    let var13 =
        nc(var3, var4 + var10, var5, var6 + var11) + nc(var3, var4 + var10, var5 + 1, var6 + var11);
    let var14 = var3.get_block_id(var4 - var10, var5, var6 - var11) == var8
        || var3.get_block_id(var4 - var10, var5 + 1, var6 - var11) == var8;
    let var15 = var3.get_block_id(var4 + var10, var5, var6 + var11) == var8
        || var3.get_block_id(var4 + var10, var5 + 1, var6 + var11) == var8;
    let mut var16 = false;
    if var14 && !var15 {
        var16 = true;
    } else if var13 > var12 {
        var16 = true;
    }
    if var16 {
        var9 = (var9 - 1) & 3;
        var9 += 4;
    }
    var3.editing_blocks = true;
    var3.set_block_and_metadata_with_notify(var4, var5, var6, var8, var9);
    var3.set_block_and_metadata_with_notify(var4, var5 + 1, var6, var8, var9 + 8);
    var3.editing_blocks = false;
    var3.notify_blocks_of_neighbor_change(var4, var5, var6, var8);
    var3.notify_blocks_of_neighbor_change(var4, var5 + 1, var6, var8);
    var1.stack_size -= 1;
    true
}

/// `ItemBed.onItemUse`: only on a top face; the foot at the cell above and
/// the head one step along the player's yaw, both air over normal cubes;
/// the foot gets the direction, the head the direction + 8.
// src: ItemBed.java:8-42
#[allow(clippy::too_many_arguments)]
pub fn bed_on_item_use(
    var1: &mut ItemStack,
    var2: &mut Entity,
    var3: &mut World,
    var4: i32,
    mut var5: i32,
    var6: i32,
    var7: i32,
) -> bool {
    if var7 != 1 {
        return false;
    }
    var5 += 1;
    let var8 = BED;
    let var9 = floor_double((var2.base.rotation_yaw * 4.0 / 360.0) as f64 + 0.5) & 3;
    let mut var10: i32 = 0;
    let mut var11: i32 = 0;
    if var9 == 0 {
        var11 = 1;
    }
    if var9 == 1 {
        var10 = -1;
    }
    if var9 == 2 {
        var11 = -1;
    }
    if var9 == 3 {
        var10 = 1;
    }
    if var3.is_air_block(var4, var5, var6)
        && var3.is_air_block(var4 + var10, var5, var6 + var11)
        && var3.is_block_normal_cube(var4, var5 - 1, var6)
        && var3.is_block_normal_cube(var4 + var10, var5 - 1, var6 + var11)
    {
        var3.set_block_and_metadata_with_notify(var4, var5, var6, var8, var9);
        var3.set_block_and_metadata_with_notify(var4 + var10, var5, var6 + var11, var8, var9 + 8);
        var1.stack_size -= 1;
        true
    } else {
        false
    }
}

/// `ItemSign.onItemUse`: not on a bottom face, only on a solid block; the
/// cell in front of the face must take a sign post (`canPlaceBlockAt`); a
/// top face places a sign post turned to the player's yaw (16 steps), a side
/// face a wall sign with the face as metadata. Then one item off and, with
/// a sign tile entity there, `displayGUIEditSign` (the editor GUI of
/// `EntityPlayerSP`, client; empty in `EntityPlayer`): recorded.
// src: ItemSign.java:9-53
#[allow(clippy::too_many_arguments)]
pub fn sign_on_item_use(
    var1: &mut ItemStack,
    var2: &mut Entity,
    var3: &mut World,
    mut var4: i32,
    mut var5: i32,
    mut var6: i32,
    var7: i32,
) -> bool {
    if var7 == 0 {
        return false;
    }
    if !var3.get_block_material(var4, var5, var6).is_solid() {
        return false;
    }
    // src: ItemSign.java:15-33 (no bottom face here, var7 != 0)
    (var4, var5, var6) = offset_by_side(var7, var4, var5, var6);
    if !block_dispatch::can_place_block_at(var3, SIGN_POST, var4, var5, var6) {
        return false;
    }
    if var7 == 1 {
        let meta =
            floor_double(((var2.base.rotation_yaw + 180.0) * 16.0 / 360.0) as f64 + 0.5) & 15;
        var3.set_block_and_metadata_with_notify(var4, var5, var6, SIGN_POST, meta);
    } else {
        var3.set_block_and_metadata_with_notify(var4, var5, var6, SIGN_WALL, var7);
    }
    var1.stack_size -= 1;
    let var8 = var3.get_block_tile_entity(var4, var5, var6);
    if var8.is_some_and(|r| {
        matches!(
            var3.tile_entities.get(&r).map(|t| &t.kind),
            Some(crate::world::tile_entity::TileEntityKindData::Sign(_))
        )
    }) {
        // var2.displayGUIEditSign(var8): EntityPlayer.java:454-455 is empty;
        // EntityPlayerSP opens GuiEditSign (client GUI, Phase 3).
        var3.note_unimplemented("EntityPlayerSP.displayGUIEditSign");
    }
    true
}

/// `ItemRedstone.onItemUse`: on a snow layer the layer's cell, otherwise
/// the cell in front of the face, which must be air; there, when the wire
/// can be placed, one item off and the wire set. True unless the cell was
/// taken.
// src: ItemRedstone.java:8-45
#[allow(clippy::too_many_arguments)]
pub fn redstone_on_item_use(
    var1: &mut ItemStack,
    var3: &mut World,
    mut var4: i32,
    mut var5: i32,
    mut var6: i32,
    var7: i32,
) -> bool {
    if var3.get_block_id(var4, var5, var6) != SNOW {
        (var4, var5, var6) = offset_by_side(var7, var4, var5, var6);
        if !var3.is_air_block(var4, var5, var6) {
            return false;
        }
    }
    if block_dispatch::can_place_block_at(var3, REDSTONE_WIRE, var4, var5, var6) {
        var1.stack_size -= 1;
        var3.set_block_with_notify(var4, var5, var6, REDSTONE_WIRE);
    }
    true
}

/// `ItemSeeds.onItemUse`: on the top face of farmland with air above, the
/// crop block (`blockType`) above and one item off.
// src: ItemSeeds.java:11-24
pub fn seeds_on_item_use(
    block_type: i32,
    var1: &mut ItemStack,
    var3: &mut World,
    var4: i32,
    var5: i32,
    var6: i32,
    var7: i32,
) -> bool {
    if var7 != 1 {
        return false;
    }
    let var8 = var3.get_block_id(var4, var5, var6);
    if var8 == TILLED_FIELD && var3.is_air_block(var4, var5 + 1, var6) {
        var3.set_block_with_notify(var4, var5 + 1, var6, block_type);
        var1.stack_size -= 1;
        true
    } else {
        false
    }
}
