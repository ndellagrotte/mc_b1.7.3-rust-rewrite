//! `ItemBucket` (ItemBucket.java): the empty bucket, the water and lava
//! buckets and the milk bucket (`isFull` 0, 8, 10, -1).

use super::{ItemStack, RightClickResult, player_look_trace};
use crate::block::Material;
use crate::entity::Entity;
use crate::world::World;
use crate::world::ray_trace::EnumMovingObjectType;

/// `Item.bucketEmpty.shiftedIndex`.
// src: Item.java:343 (new ItemBucket(69, 0): 256 + 69)
const BUCKET_EMPTY: i32 = 325;
/// `Item.bucketWater.shiftedIndex`.
// src: Item.java:344
const BUCKET_WATER: i32 = 326;
/// `Item.bucketLava.shiftedIndex`.
// src: Item.java:345
const BUCKET_LAVA: i32 = 327;
/// `Block.waterMoving.blockID`.
// src: Block.java:600
const WATER_MOVING: i32 = 8;

/// `ItemBucket.onItemRightClick`: the eye ray (five blocks; it stops on
/// liquids only for the empty bucket) picks a block the player may mine
/// (`func_6466_a`, always true in single player).
///
/// - Empty: a still source (`getBlockMetadata == 0`) of water or lava is
///   removed and a full bucket returned.
/// - Milk: an empty bucket, whatever the block.
/// - Water or lava: into the cell in front of the face if it is air or
///   not solid, as a source with notification; water in the nether only
///   fizzes (a `World.rand` pitch, eight smoke particles at `Math.random()`
///   offsets) and is gone. An empty bucket is returned.
///
/// Otherwise (no hit, a refused block, a solid target) the stack itself.
/// The entity branch (`instanceof EntityCow` for milk) is dead code: the
/// block ray trace never reports an entity.
// src: ItemBucket.java:12-99
pub fn on_item_right_click(
    is_full: i32,
    var1: &mut ItemStack,
    var2: &mut World,
    var3: &mut Entity,
) -> RightClickResult {
    let _ = var1;
    let ([var7, var9, var11], var24) = player_look_trace(var2, var3, is_full == 0);
    let Some(var24) = var24 else {
        return RightClickResult::Same;
    };
    if var24.type_of_hit == EnumMovingObjectType::Tile {
        let mut var25 = var24.block_x;
        let mut var26 = var24.block_y;
        let mut var27 = var24.block_z;
        if !var2.func_6466_a(var3.base.entity_id, var25, var26, var27) {
            return RightClickResult::Same;
        }
        if is_full == 0 {
            // src: ItemBucket.java:41-49
            if var2.get_block_material(var25, var26, var27) == Material::Water
                && var2.get_block_metadata(var25, var26, var27) == 0
            {
                var2.set_block_with_notify(var25, var26, var27, 0);
                return RightClickResult::Replaced(ItemStack::new(BUCKET_WATER, 1, 0));
            }
            if var2.get_block_material(var25, var26, var27) == Material::Lava
                && var2.get_block_metadata(var25, var26, var27) == 0
            {
                var2.set_block_with_notify(var25, var26, var27, 0);
                return RightClickResult::Replaced(ItemStack::new(BUCKET_LAVA, 1, 0));
            }
        } else {
            if is_full < 0 {
                return RightClickResult::Replaced(ItemStack::new(BUCKET_EMPTY, 1, 0));
            }
            // src: ItemBucket.java:55-77
            let s = var24.side_hit;
            if s == 0 {
                var26 -= 1;
            }
            if s == 1 {
                var26 += 1;
            }
            if s == 2 {
                var27 -= 1;
            }
            if s == 3 {
                var27 += 1;
            }
            if s == 4 {
                var25 -= 1;
            }
            if s == 5 {
                var25 += 1;
            }
            if var2.is_air_block(var25, var26, var27)
                || !var2.get_block_material(var25, var26, var27).is_solid()
            {
                if var2.world_provider.is_hell_world && is_full == WATER_MOVING {
                    // src: ItemBucket.java:81-85. Java evaluates the two
                    // nextFloat() calls left to right.
                    let a = var2.rand.next_float();
                    let b = var2.rand.next_float();
                    var2.play_sound_effect(
                        var7 + 0.5,
                        var9 + 0.5,
                        var11 + 0.5,
                        "random.fizz",
                        0.5,
                        2.6 + (a - b) * 0.8,
                    );
                    for _ in 0..8 {
                        // Math.random() (census §1: ItemBucket.java:84, D)
                        let px = var25 as f64 + var2.math_random.next_double();
                        let py = var26 as f64 + var2.math_random.next_double();
                        let pz = var27 as f64 + var2.math_random.next_double();
                        var2.spawn_particle("largesmoke", px, py, pz, 0.0, 0.0, 0.0);
                    }
                } else {
                    var2.set_block_and_metadata_with_notify(var25, var26, var27, is_full, 0);
                }
                return RightClickResult::Replaced(ItemStack::new(BUCKET_EMPTY, 1, 0));
            }
        }
    }
    // src: ItemBucket.java:93-95: `isFull == 0 && entityHit instanceof
    // EntityCow` is unreachable (rayTraceBlocks_do returns TILE hits only).
    RightClickResult::Same
}
