//! The `Block` methods `RenderBlocks` calls, on a [`ChunkSnapshot`] (the
//! `IBlockAccess`) and the job's copy of the singletons' state
//! ([`BlockStatics`]): `getBlockTexture`, `getBlockTextureFromSide(AndMetadata)`,
//! `shouldSideBeRendered`, `getBlockBrightness`, `colorMultiplier`,
//! `getRenderColor`, `setBlockBoundsBasedOnState`, `getRenderType`,
//! `getRenderBlockPass`.
//!
//! Data comes from `mc-sim` (the block table, `block_texture`'s
//! side/metadata textures, `getRenderType`, `getRenderBlockPass`,
//! `isOpaqueCube`). Where `mc-sim`'s version needs the `World`
//! (`getBlockTexture` and `setBlockBoundsBasedOnState` of the kinds that
//! read it) the snapshot carries its result ([`ChunkSnapshot`] docs). The
//! rest — the `shouldSideBeRendered` overrides, `getBlockBrightness`,
//! `colorMultiplier` — are short and read only the `IBlockAccess`, so they
//! are transcribed here on the snapshot (mc-sim's take `&mut World`).

use mc_sim::block::{self, BlockBounds, BlockDef, BlockKind, BlockStatics, Material};

use super::snapshot::ChunkSnapshot;
use crate::textures::colorizer;

/// `Block.blocksList[id]`.
pub fn def(id: i32) -> Option<&'static BlockDef> {
    block::block(id)
}

/// The singleton's current bounds.
pub fn bounds(statics: &BlockStatics, id: i32) -> BlockBounds {
    statics.bounds[(id & 255) as usize]
}

/// `getRenderType()` (-1 for an unknown id).
// src: Block.java:195-197 (mc-sim block::get_render_type)
pub fn get_render_type(id: i32) -> i32 {
    def(id).map_or(-1, |b| block::get_render_type(b.kind))
}

/// `getRenderBlockPass()`.
// src: Block.java:493-495 (mc-sim block::get_render_block_pass)
pub fn get_render_block_pass(id: i32) -> i32 {
    def(id).map_or(0, block::get_render_block_pass)
}

/// `getBlockTextureFromSide(side)`.
// src: Block.java:265-267 (mc-sim block::get_block_texture_from_side)
pub fn get_block_texture_from_side(statics: &BlockStatics, id: i32, side: i32) -> i32 {
    def(id).map_or(0, |b| block::get_block_texture_from_side(b, side, statics))
}

/// `getBlockTextureFromSideAndMetadata(side, meta)`; where the Java throws
/// (an array index out of bounds for a bad metadata), the side's texture
/// without metadata.
// src: Block.java:261-263 (mc-sim block::get_block_texture_from_side_and_metadata)
pub fn get_block_texture_from_side_and_metadata(
    statics: &BlockStatics,
    id: i32,
    side: i32,
    meta: i32,
) -> i32 {
    let Some(b) = def(id) else { return 0 };
    block::get_block_texture_from_side_and_metadata(b, side, meta, statics)
        .unwrap_or_else(|| block::get_block_texture_from_side(b, side, statics))
}

/// `getBlockTexture(blockAccess, x, y, z, side)`: the precomputed value for
/// the kinds that read the world, else
/// `getBlockTextureFromSideAndMetadata(side, metadata)`.
// src: Block.java:257-259
pub fn get_block_texture(
    s: &ChunkSnapshot,
    statics: &BlockStatics,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    side: i32,
) -> i32 {
    match s.block_texture_override(id, x, y, z, side) {
        Some(Some(t)) => t,
        Some(None) => get_block_texture_from_side(statics, id, side),
        None => get_block_texture_from_side_and_metadata(
            statics,
            id,
            side,
            s.get_block_metadata(x, y, z),
        ),
    }
}

/// `setBlockBoundsBasedOnState(blockAccess, x, y, z)`: the precomputed
/// bounds of the kinds that override it; nothing for the rest.
// src: Block.java:522-523
pub fn set_block_bounds_based_on_state(
    s: &ChunkSnapshot,
    statics: &mut BlockStatics,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
) {
    if let Some(b) = s.bounds_override(id, x, y, z) {
        statics.bounds[(id & 255) as usize] = b;
    }
}

/// `setBlockBounds(x0, y0, z0, x1, y1, z1)` on the singleton.
// src: Block.java:222-229
#[allow(clippy::too_many_arguments)]
pub fn set_block_bounds(
    statics: &mut BlockStatics,
    id: i32,
    x0: f32,
    y0: f32,
    z0: f32,
    x1: f32,
    y1: f32,
    z1: f32,
) {
    statics.bounds[(id & 255) as usize] = BlockBounds::from_floats(x0, y0, z0, x1, y1, z1);
}

/// `setBlockBoundsForItemRender()`: the item shape of the blocks that have
/// one (the piston base's full cube, the pressure plate's, button's,
/// cake's and trapdoor's boxes); nothing for the rest. mc-sim's
/// `block::dispatch::set_block_bounds_for_item_render` writes the world's
/// singletons, so the five constant boxes are repeated here, in the Java's
/// float arithmetic.
// src: Block.java:548-549
pub fn set_block_bounds_for_item_render(statics: &mut BlockStatics, id: i32) {
    let Some(b) = def(id) else { return };
    let bb = match b.kind {
        // src: BlockPistonBase.java:203-205
        BlockKind::PistonBase { .. } => BlockBounds::FULL,
        // src: BlockPressurePlate.java:149-154
        BlockKind::PressurePlate { .. } => {
            let (var1, var2, var3) = (0.5f32, 0.125f32, 0.5f32);
            BlockBounds::from_floats(
                0.5 - var1,
                0.5 - var2,
                0.5 - var3,
                0.5 + var1,
                0.5 + var2,
                0.5 + var3,
            )
        }
        // src: BlockButton.java:250-255
        BlockKind::Button => {
            let (var1, var2, var3) = (0.1875f32, 0.125f32, 0.125f32);
            BlockBounds::from_floats(
                0.5 - var1,
                0.5 - var2,
                0.5 - var3,
                0.5 + var1,
                0.5 + var2,
                0.5 + var3,
            )
        }
        // src: BlockCake.java:19-23
        BlockKind::Cake => {
            let (var1, var2) = (0.0625f32, 0.5f32);
            BlockBounds::from_floats(var1, 0.0, var1, 1.0 - var1, var2, 1.0 - var1)
        }
        // src: BlockTrapDoor.java:42-45
        BlockKind::TrapDoor => {
            let var1 = 0.1875f32;
            BlockBounds::from_floats(0.0, 0.5 - var1 / 2.0, 0.0, 1.0, 0.5 + var1 / 2.0, 1.0)
        }
        _ => return,
    };
    statics.bounds[(id & 255) as usize] = bb;
}

/// `Block.shouldSideBeRendered`: `side`'s face is drawn when the bounds do
/// not reach that face of the cell, else when the neighbour at `(x, y, z)`
/// is not an opaque cube.
// src: Block.java:235-251
#[allow(clippy::if_same_then_else)] // the Java's branches, kept
pub fn base_should_side_be_rendered(
    s: &ChunkSnapshot,
    statics: &BlockStatics,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    side: i32,
) -> bool {
    let bb = bounds(statics, id);
    if side == 0 && bb.min_y > 0.0 {
        true
    } else if side == 1 && bb.max_y < 1.0 {
        true
    } else if side == 2 && bb.min_z > 0.0 {
        true
    } else if side == 3 && bb.max_z < 1.0 {
        true
    } else if side == 4 && bb.min_x > 0.0 {
        true
    } else if side == 5 && bb.max_x < 1.0 {
        true
    } else {
        !s.is_block_opaque_cube(x, y, z)
    }
}

/// `shouldSideBeRendered(blockAccess, x, y, z, side)` with every override;
/// `(x, y, z)` is the neighbour on `side`.
#[allow(clippy::if_same_then_else)] // the Java's branches, kept
pub fn should_side_be_rendered(
    s: &ChunkSnapshot,
    statics: &BlockStatics,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    side: i32,
) -> bool {
    let base = |side| base_should_side_be_rendered(s, statics, id, x, y, z, side);
    let Some(b) = def(id) else {
        return base(side);
    };
    match b.kind {
        // src: BlockFluid.java:71-80
        BlockKind::Flowing | BlockKind::Stationary => {
            let var6 = s.get_block_material(x, y, z);
            if var6 == b.block_material {
                false
            } else if var6 == Material::Ice {
                false
            } else {
                side == 1 || base(side)
            }
        }
        // src: BlockLeavesBase.java:15-18
        BlockKind::Leaves => {
            let var6 = s.get_block_id(x, y, z);
            if !statics.graphics_level && var6 == id {
                false
            } else {
                base(side)
            }
        }
        // src: BlockBreakable.java:15-18
        BlockKind::Glass { local_flag } => {
            let var6 = s.get_block_id(x, y, z);
            if !local_flag && var6 == id {
                false
            } else {
                base(side)
            }
        }
        // src: BlockStep.java:76-90 (the first `super` call's result is
        // discarded: it only reads)
        BlockKind::Step { .. } => {
            if side == 1 {
                true
            } else if !base(side) {
                false
            } else if side == 0 {
                true
            } else {
                s.get_block_id(x, y, z) != id
            }
        }
        // src: BlockSnow.java:79-81
        BlockKind::Snow => side == 1 || base(side),
        // src: BlockIce.java:16-18 → BlockBreakable.java:15-18 (localFlag
        // false). Vanilla bug, transcribed: the side passed on is `1 - side`,
        // so the bounds test looks at the wrong face (no effect for the
        // full-cube ice).
        BlockKind::Ice => {
            let var5 = 1 - side;
            let var6 = s.get_block_id(x, y, z);
            if var6 == id { false } else { base(var5) }
        }
        // src: BlockPortal.java:120-140
        BlockKind::Portal => {
            if s.get_block_id(x, y, z) == id {
                false
            } else {
                let var6 = s.get_block_id(x - 1, y, z) == id && s.get_block_id(x - 2, y, z) != id;
                let var7 = s.get_block_id(x + 1, y, z) == id && s.get_block_id(x + 2, y, z) != id;
                let var8 = s.get_block_id(x, y, z - 1) == id && s.get_block_id(x, y, z - 2) != id;
                let var9 = s.get_block_id(x, y, z + 1) == id && s.get_block_id(x, y, z + 2) != id;
                let var10 = var6 || var7;
                let var11 = var8 || var9;
                if var10 && side == 4 {
                    true
                } else if var10 && side == 5 {
                    true
                } else if var11 && side == 2 {
                    true
                } else {
                    var11 && side == 3
                }
            }
        }
        // src: BlockRedstoneRepeater.java:53-55
        BlockKind::RedstoneRepeater { .. } => side != 0 && side != 1,
        _ => base(side),
    }
}

/// `getBlockBrightness(blockAccess, x, y, z)`: the brightness at the light
/// value, at least the block's own `lightValue`; fluids take the brighter
/// of their cell and the one above; stairs ask their model block.
// src: Block.java:231-233
pub fn get_block_brightness(s: &ChunkSnapshot, id: i32, x: i32, y: i32, z: i32) -> f32 {
    match def(id).map(|b| b.kind) {
        // src: BlockFluid.java:195-199
        Some(BlockKind::Flowing | BlockKind::Stationary) => {
            let var5 = s.get_light_brightness(x, y, z);
            let var6 = s.get_light_brightness(x, y + 1, z);
            if var5 > var6 { var5 } else { var6 }
        }
        // src: BlockStairs.java:81-83
        Some(BlockKind::Stairs { model_block }) => {
            s.get_brightness(x, y, z, block::light_value(model_block))
        }
        _ => s.get_brightness(x, y, z, block::light_value(id)),
    }
}

/// `colorMultiplier(blockAccess, x, y, z)` (`0xRRGGBB`): grass and oak
/// leaves by biome, spruce and birch leaves fixed, tall grass by the biome
/// at a hashed offset, redstone wire `0x800000`; white otherwise.
// src: Block.java:529-531
pub fn color_multiplier(s: &ChunkSnapshot, id: i32, x: i32, y: i32, z: i32) -> i32 {
    const WHITE: i32 = 16777215;
    let Some(b) = def(id) else { return WHITE };
    match b.kind {
        // src: BlockGrass.java:23-28
        BlockKind::Grass => {
            let (t, h) = s.biome_temperature_humidity(x, z);
            s.colorizers.get_grass_color(t, h)
        }
        // src: BlockFluid.java:14-16
        BlockKind::Flowing | BlockKind::Stationary => WHITE,
        // src: BlockLeaves.java:23-35
        BlockKind::Leaves => {
            let var5 = s.get_block_metadata(x, y, z);
            if (var5 & 1) == 1 {
                colorizer::get_foliage_color_pine()
            } else if (var5 & 2) == 2 {
                colorizer::get_foliage_color_birch()
            } else {
                let (t, h) = s.biome_temperature_humidity(x, z);
                s.colorizers.get_foliage_color(t, h)
            }
        }
        // src: BlockTallGrass.java:22-37
        BlockKind::TallGrass => {
            let var5 = s.get_block_metadata(x, y, z);
            if var5 == 0 {
                WHITE
            } else {
                let mut var6 = x
                    .wrapping_mul(3129871)
                    .wrapping_add(z.wrapping_mul(6129781))
                    .wrapping_add(y) as i64;
                var6 = var6
                    .wrapping_mul(var6)
                    .wrapping_mul(42317861)
                    .wrapping_add(var6.wrapping_mul(11));
                let var2 = (x as i64).wrapping_add(var6 >> 14 & 31) as i32;
                let var4 = (z as i64).wrapping_add(var6 >> 24 & 31) as i32;
                let (t, h) = s.biome_temperature_humidity(var2, var4);
                s.colorizers.get_grass_color(t, h)
            }
        }
        // src: BlockRedstoneWire.java:37-39
        BlockKind::RedstoneWire => 8388608,
        _ => WHITE,
    }
}

/// `getRenderColor(meta)` (`0xRRGGBB`): the item tint, leaves by type.
// src: Block.java:525-527, BlockLeaves.java:15-21
pub fn get_render_color(id: i32, meta: i32) -> i32 {
    match def(id).map(|b| b.kind) {
        Some(BlockKind::Leaves) => block::leaves::get_render_color(meta),
        _ => 16777215,
    }
}

/// `Block.canBlockGrass[id]` (`canBlockGrass[0]` is true).
// src: Block.java:21, :704
pub fn can_block_grass(id: i32) -> bool {
    block::can_block_grass(id)
}

/// `Block.lightValue[id]`.
pub fn light_value(id: i32) -> i32 {
    block::light_value(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mc_sim::world::provider::{Dimension, WorldProvider};

    fn snap() -> ChunkSnapshot {
        ChunkSnapshot::synthetic(
            [-3, 0, -3],
            [4, 6, 4],
            WorldProvider::new(Dimension::Surface).light_brightness_table,
        )
    }

    const STONE: i32 = 1;
    const GLASS: i32 = 20;
    const LEAVES: i32 = 18;
    const SLAB: i32 = 44;
    const ICE: i32 = 79;
    const WATER: i32 = 9;

    /// The face-culling table: opaque neighbours hide a full cube's faces,
    /// transparent ones (air, glass, leaves with fancy graphics) do not;
    /// glass and fast leaves hide faces against their own kind.
    #[test]
    fn culling_against_opaque_and_transparent_neighbours() {
        let mut s = snap();
        let st = s.statics.clone();
        let rendered = |s: &ChunkSnapshot, st: &BlockStatics, id| {
            should_side_be_rendered(s, st, id, 1, 1, 0, 5)
        };
        for (neighbour, stone_face, glass_face) in [
            (0, true, true),
            (STONE, false, false),
            (GLASS, true, false),
            (LEAVES, false, false),
            (WATER, true, true),
        ] {
            s.set(1, 1, 0, neighbour, 0, 15, 0);
            assert_eq!(rendered(&s, &st, STONE), stone_face, "stone|{neighbour}");
            assert_eq!(rendered(&s, &st, GLASS), glass_face, "glass|{neighbour}");
        }
        // leaves: fast graphics hide leaf-leaf faces (and leaves are opaque
        // cubes then), fancy graphics draw them
        s.set(1, 1, 0, LEAVES, 0, 15, 0);
        assert!(!should_side_be_rendered(&s, &st, LEAVES, 1, 1, 0, 5));
        let mut fancy = st.clone();
        fancy.set_leaves_graphics_level(true);
        s.statics.set_leaves_graphics_level(true);
        assert!(should_side_be_rendered(&s, &fancy, LEAVES, 1, 1, 0, 5));
    }

    #[test]
    fn partial_bounds_always_draw_the_inner_faces() {
        let s = snap();
        let mut st = s.statics.clone();
        // a slab's top face is drawn under a stone block
        let mut s2 = snap();
        s2.set(0, 2, 0, STONE, 0, 15, 0);
        assert!(should_side_be_rendered(&s2, &st, SLAB, 0, 2, 0, 1));
        // its side faces are hidden next to another slab, and shown next to air
        s2.set(1, 1, 0, SLAB, 0, 15, 0);
        assert!(!should_side_be_rendered(&s2, &st, SLAB, 1, 1, 0, 5));
        assert!(should_side_be_rendered(&s, &st, SLAB, 1, 1, 0, 5));
        // a stone block with shrunk bounds draws its inner faces regardless
        set_block_bounds(&mut st, STONE, 0.25, 0.0, 0.0, 1.0, 0.5, 1.0);
        s2.set(-1, 1, 0, STONE, 0, 15, 0);
        assert!(should_side_be_rendered(&s2, &st, STONE, -1, 1, 0, 4));
        assert!(should_side_be_rendered(&s2, &st, STONE, 0, 2, 0, 1));
        // a face at the cell's edge is hidden by an opaque neighbour
        s2.set(1, 1, 0, STONE, 0, 15, 0);
        assert!(!should_side_be_rendered(&s2, &st, STONE, 1, 1, 0, 5));
    }

    #[test]
    fn ice_hides_faces_next_to_ice() {
        let mut s = snap();
        let st = s.statics.clone();
        s.set(1, 1, 0, ICE, 0, 15, 0);
        assert!(!should_side_be_rendered(&s, &st, ICE, 1, 1, 0, 5));
        s.set(1, 1, 0, STONE, 0, 15, 0);
        assert!(!should_side_be_rendered(&s, &st, ICE, 1, 1, 0, 5));
        s.set(1, 1, 0, 0, 0, 15, 0);
        assert!(should_side_be_rendered(&s, &st, ICE, 1, 1, 0, 5));
    }

    #[test]
    fn render_passes() {
        assert_eq!(get_render_block_pass(STONE), 0);
        assert_eq!(get_render_block_pass(GLASS), 0);
        assert_eq!(get_render_block_pass(LEAVES), 0);
        assert_eq!(get_render_block_pass(ICE), 1);
        assert_eq!(get_render_block_pass(WATER), 1);
        assert_eq!(get_render_block_pass(8), 1);
        assert_eq!(get_render_block_pass(10), 0);
        assert_eq!(get_render_block_pass(90), 1); // portal
        assert_eq!(get_render_block_pass(53), 0); // wooden stairs
    }

    #[test]
    fn render_types_of_the_dispatch() {
        // src: RenderBlocks.java:92-132 against each class's getRenderType
        for (id, t) in [
            (1, 0),
            (2, 0),
            (18, 0),
            (20, 0),
            (79, 0),
            (6, 1),
            (50, 2),
            (51, 3),
            (9, 4),
            (55, 5),
            (59, 6),
            (64, 7),
            (65, 8),
            (66, 9),
            (53, 10),
            (85, 11),
            (69, 12),
            (81, 13),
            (26, 14),
            (93, 15),
            (33, 16),
            (34, 17),
            (63, -1),
        ] {
            assert_eq!(get_render_type(id), t, "id {id}");
        }
    }

    #[test]
    fn tall_grass_colour_offset_matches_the_java_hash() {
        // (long)(x * 3129871 + z * 6129781 + y) with int overflow, then the
        // long arithmetic, for x = 1000, y = 64, z = -2000.
        let (x, y, z) = (1000i32, 64i32, -2000i32);
        let mut v = x
            .wrapping_mul(3129871)
            .wrapping_add(z.wrapping_mul(6129781))
            .wrapping_add(y) as i64;
        assert_eq!(v, (1000i64 * 3129871 - 2000 * 6129781 + 64) as i32 as i64);
        v = v
            .wrapping_mul(v)
            .wrapping_mul(42317861)
            .wrapping_add(v.wrapping_mul(11));
        let dx = v >> 14 & 31;
        assert!((0..32).contains(&dx));
    }
}
