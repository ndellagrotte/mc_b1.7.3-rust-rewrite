//! `Block.getBlockTextureFromSide(side)` and
//! `Block.getBlockTextureFromSideAndMetadata(side, meta)` with every
//! subclass override, in the state after Block.java's static initialiser.
//!
//! The item table needs them: every `ItemBlock` constructor sets its icon to
//! `getBlockTextureFromSide(2)` of its block (ItemBlock.java:9), and the
//! log, sapling, wool, slab and leaves items look their icons up per damage
//! value. They belong to the block module; they live here until the block
//! side has them (reported by WP-1.2), and the `item/item-table` golden
//! fixture checks every block, side and metadata 0–15 against the oracle.
//!
//! Java dispatch: `Block.getBlockTextureFromSideAndMetadata` calls the
//! virtual `getBlockTextureFromSide`, whose default returns
//! `blockIndexInTexture` (Block.java:261-267). A class may override either
//! or both; [`get_block_texture_from_side`] and
//! [`get_block_texture_from_side_and_metadata`] each `match` on the kind the
//! same way.
//!
//! Static-init state used for vanilla's mutable singleton fields:
//! `blockIndexInTexture` from [`crate::block::BLOCKS`] (leaves: 52, before
//! any `setGraphicsLevel`), the piston base's bounds from
//! [`BlockDef::bounds`], and `BlockPistonExtension.headTexture = -1`
//! (BlockPistonExtension.java:7).

use crate::block::{BLOCKS, BlockDef, BlockKind};

use super::JavaException;

/// `PistonBlockTextures.faceToSide`.
// src: PistonBlockTextures.java:4
const FACE_TO_SIDE: [i32; 6] = [1, 0, 3, 2, 5, 4];

/// `ModelBed.bedDirection`.
// src: ModelBed.java:6
const BED_DIRECTION: [[i32; 6]; 4] = [
    [1, 0, 3, 2, 5, 4],
    [1, 0, 5, 4, 2, 3],
    [1, 0, 2, 3, 4, 5],
    [1, 0, 4, 5, 3, 2],
];

// src: Block.java:597, :647
const PLANKS: i32 = 5;
const REDSTONE_WIRE: i32 = 55;
const RAIL_POWERED: i32 = 27; // src: Block.java:619

/// `BlockPistonExtension.headTexture` after construction.
// src: BlockPistonExtension.java:7
const PISTON_EXTENSION_HEAD_TEXTURE: i32 = -1;

/// `Block.blocksList[id]`; panics where Java would throw a
/// `NullPointerException` (callers only pass existing blocks).
const fn def(id: i32) -> &'static BlockDef {
    match &BLOCKS[id as usize] {
        Some(b) => b,
        None => panic!("Block.blocksList[id] is null"),
    }
}

/// `array[i]` with Java's bounds check, for `const` code.
const fn index6(array: &[i32; 6], i: i32) -> Result<i32, JavaException> {
    if i >= 0 && i < 6 {
        Ok(array[i as usize])
    } else {
        Err(JavaException::ArrayIndexOutOfBounds)
    }
}

/// `blocksList[id].getBlockTextureFromSide(side)`.
// src: Block.java:265-267 and the overrides cited per arm
pub const fn get_block_texture_from_side(id: i32, side: i32) -> i32 {
    let b = def(id);
    let tex = b.block_index_in_texture;
    match b.kind {
        // src: BlockBookshelf.java:10-12
        BlockKind::Bookshelf => {
            if side <= 1 {
                4
            } else {
                tex
            }
        }
        // src: BlockCactus.java:40-46
        BlockKind::Cactus => {
            if side == 1 {
                tex - 1
            } else if side == 0 {
                tex + 1
            } else {
                tex
            }
        }
        // src: BlockDispenser.java:64-72
        BlockKind::Dispenser => {
            if side == 1 || side == 0 {
                tex + 17
            } else if side == 3 {
                tex + 1
            } else {
                tex
            }
        }
        // src: BlockCake.java:51-57
        BlockKind::Cake => {
            if side == 1 {
                tex
            } else if side == 0 {
                tex + 3
            } else {
                tex + 1
            }
        }
        // src: BlockChest.java:100-108, BlockLockedChest.java:42-50
        BlockKind::Chest | BlockKind::LockedChest => {
            if side == 1 || side == 0 {
                tex - 1
            } else if side == 3 {
                tex + 1
            } else {
                tex
            }
        }
        // src: BlockJukeBox.java:8-10
        BlockKind::JukeBox => tex + if side == 1 { 1 } else { 0 },
        // src: BlockPumpkin.java:36-44
        BlockKind::Pumpkin { .. } => {
            if side == 1 || side == 0 {
                tex
            } else if side == 3 {
                tex + 1 + 16
            } else {
                tex + 16
            }
        }
        // src: BlockFluid.java:27-29 (BlockFlowing and BlockStationary inherit it)
        BlockKind::Flowing | BlockKind::Stationary => {
            if side != 0 && side != 1 {
                tex + 1
            } else {
                tex
            }
        }
        // src: BlockFurnace.java:92-100
        BlockKind::Furnace { .. } => {
            if side == 1 || side == 0 {
                tex + 17
            } else if side == 3 {
                tex - 1
            } else {
                tex
            }
        }
        // src: BlockWorkbench.java:9-17
        BlockKind::Workbench => {
            if side == 1 {
                tex - 16
            } else if side == 0 {
                get_block_texture_from_side(PLANKS, 0)
            } else if side != 2 && side != 4 {
                tex
            } else {
                tex + 1
            }
        }
        // src: BlockStairs.java:105-107
        BlockKind::Stairs { model_block } => get_block_texture_from_side(model_block, side),
        // src: BlockSandStone.java:8-14
        BlockKind::SandStone => {
            if side == 1 {
                tex - 16
            } else if side == 0 {
                tex + 16
            } else {
                tex
            }
        }
        // src: BlockRedstoneRepeater.java:61-63, BlockStep.java:35-37
        BlockKind::RedstoneRepeater { .. } | BlockKind::Step { .. } => {
            match get_block_texture_from_side_and_metadata(id, side, 0) {
                Ok(v) => v,
                Err(_) => panic!("never thrown for metadata 0"),
            }
        }
        // src: BlockTNT.java:10-16
        BlockKind::Tnt => {
            if side == 0 {
                tex + 2
            } else if side == 1 {
                tex + 1
            } else {
                tex
            }
        }
        // BlockOreStorage.java:9-11 and BlockNote.java:8-10 override with the
        // default body.
        _ => tex,
    }
}

/// `blocksList[id].getBlockTextureFromSideAndMetadata(side, meta)`. `Err`
/// where Java throws (the piston head with direction 6 or 7, an out-of-range
/// side for the bed).
// src: Block.java:261-263 and the overrides cited per arm
pub const fn get_block_texture_from_side_and_metadata(
    id: i32,
    side: i32,
    meta: i32,
) -> Result<i32, JavaException> {
    let b = def(id);
    let tex = b.block_index_in_texture;
    Ok(match b.kind {
        // src: BlockBed.java:85-103, :145-151
        BlockKind::Bed => {
            if side == 0 {
                def(PLANKS).block_index_in_texture
            } else {
                let var3 = meta & 3; // getDirectionFromMetadata
                let var4 = match index6(&BED_DIRECTION[var3 as usize], side) {
                    Ok(v) => v,
                    Err(e) => return Err(e),
                };
                if (meta & 8) != 0 {
                    // isBlockFootOfBed
                    if var4 == 2 {
                        tex + 2 + 16
                    } else if var4 != 5 && var4 != 4 {
                        tex + 1
                    } else {
                        tex + 1 + 16
                    }
                } else if var4 == 3 {
                    tex - 1 + 16
                } else if var4 != 5 && var4 != 4 {
                    tex
                } else {
                    tex + 16
                }
            }
        }
        // src: BlockFarmland.java:26-32
        BlockKind::Farmland => {
            if side == 1 && meta > 0 {
                tex - 1
            } else if side == 1 {
                tex
            } else {
                2
            }
        }
        // src: BlockCake.java:41-49
        BlockKind::Cake => {
            if side == 1 {
                tex
            } else if side == 0 {
                tex + 3
            } else if meta > 0 && side == 4 {
                tex + 2
            } else {
                tex + 1
            }
        }
        // src: BlockRail.java:50-60 (BlockDetectorRail passes isPowered = true,
        // BlockDetectorRail.java:7)
        BlockKind::Rail { .. } | BlockKind::DetectorRail => {
            let is_powered = match b.kind {
                BlockKind::Rail { is_powered } => is_powered,
                _ => true,
            };
            if is_powered {
                if id == RAIL_POWERED && (meta & 8) == 0 {
                    tex - 16
                } else {
                    tex
                }
            } else if meta >= 6 {
                tex - 16
            } else {
                tex
            }
        }
        // src: BlockCloth.java:8-15
        BlockKind::Cloth => {
            if meta == 0 {
                tex
            } else {
                let m = !(meta & 15);
                113 + ((m & 8) >> 3) + (m & 7) * 16
            }
        }
        // src: BlockRedstoneTorch.java:11-13 (super is Block's default)
        BlockKind::RedstoneTorch { .. } => {
            if side == 1 {
                return get_block_texture_from_side_and_metadata(REDSTONE_WIRE, side, meta);
            } else {
                get_block_texture_from_side(id, side)
            }
        }
        // src: BlockDoor.java:18-36, :184-186
        BlockKind::Door => {
            if side != 0 && side != 1 {
                let var3 = if (meta & 4) == 0 {
                    meta.wrapping_sub(1) & 3
                } else {
                    meta & 3
                }; // getState
                if (var3 == 0 || var3 == 2) ^ (side <= 3) {
                    tex
                } else {
                    let mut var4 = var3 / 2 + ((side & 1) ^ var3);
                    var4 += (meta & 4) / 4;
                    let mut var5 = tex.wrapping_sub((meta & 8) * 2);
                    if (var4 & 1) != 0 {
                        var5 = -var5;
                    }
                    var5
                }
            } else {
                tex
            }
        }
        // src: BlockCrops.java:77-83
        BlockKind::Crops => tex.wrapping_add(if meta < 0 { 7 } else { meta }),
        // src: BlockPistonBase.java:20-29, :216-222
        BlockKind::PistonBase { .. } => {
            let var3 = meta & 7; // getOrientation
            if var3 > 5 {
                tex
            } else if side == var3 {
                let bb = b.bounds;
                let powered = (meta & 8) != 0; // isPowered
                if !powered
                    && bb.min_x <= 0.0
                    && bb.min_y <= 0.0
                    && bb.min_z <= 0.0
                    && bb.max_x >= 1.0
                    && bb.max_y >= 1.0
                    && bb.max_z >= 1.0
                {
                    tex
                } else {
                    110
                }
            } else if side == FACE_TO_SIDE[var3 as usize] {
                109
            } else {
                108
            }
        }
        // src: BlockPumpkin.java:13-34
        BlockKind::Pumpkin { block_type } => {
            if side == 1 || side == 0 {
                tex
            } else {
                let mut var3 = tex + 1 + 16;
                if block_type {
                    var3 += 1;
                }
                if (meta == 2 && side == 2)
                    || (meta == 3 && side == 5)
                    || (meta == 0 && side == 3)
                    || (meta == 1 && side == 4)
                {
                    var3
                } else {
                    tex + 16
                }
            }
        }
        // src: BlockLeaves.java:181-183
        BlockKind::Leaves => {
            if (meta & 3) == 1 {
                tex + 80
            } else {
                tex
            }
        }
        // src: BlockDeadBush.java:16-18, BlockRedstoneWire.java:17-19
        BlockKind::DeadBush | BlockKind::RedstoneWire => tex,
        // src: BlockPistonExtension.java:41-52, :156-158
        BlockKind::PistonExtension => {
            let var3 = meta & 7; // getDirectionMeta
            if side == var3 {
                if PISTON_EXTENSION_HEAD_TEXTURE >= 0 {
                    PISTON_EXTENSION_HEAD_TEXTURE
                } else if (meta & 8) != 0 {
                    tex - 1
                } else {
                    tex
                }
            } else {
                // vanilla bug: faceToSide has 6 entries, so direction 6 or 7
                // (metadata 6, 7, 14, 15) throws on every other side.
                let opposite = match index6(&FACE_TO_SIDE, var3) {
                    Ok(v) => v,
                    Err(e) => return Err(e),
                };
                if side == opposite { 107 } else { 108 }
            }
        }
        // src: BlockTallGrass.java:12-20
        BlockKind::TallGrass => {
            if meta == 1 {
                tex
            } else if meta == 2 {
                tex + 16 + 1
            } else if meta == 0 {
                tex + 16
            } else {
                tex
            }
        }
        // src: BlockLog.java:44-54
        BlockKind::Log => {
            if side == 1 || side == 0 {
                21
            } else if meta == 1 {
                116
            } else if meta == 2 {
                117
            } else {
                20
            }
        }
        // src: BlockSapling.java:27-34 (super: BlockFlower, Block's default)
        BlockKind::Sapling => {
            let m = meta & 3;
            if m == 1 {
                63
            } else if m == 2 {
                79
            } else {
                get_block_texture_from_side(id, side)
            }
        }
        // src: BlockStairs.java:101-103
        BlockKind::Stairs { model_block } => {
            return get_block_texture_from_side_and_metadata(model_block, side, meta);
        }
        // src: BlockRedstoneRepeater.java:43-51
        BlockKind::RedstoneRepeater {
            is_repeater_powered,
        } => {
            if side == 0 {
                if is_repeater_powered { 99 } else { 115 }
            } else if side == 1 {
                if is_repeater_powered { 147 } else { 131 }
            } else {
                5
            }
        }
        // src: BlockStep.java:19-33
        BlockKind::Step { .. } => {
            if meta == 0 {
                if side <= 1 { 6 } else { 5 }
            } else if meta == 1 {
                if side == 0 {
                    208
                } else if side == 1 {
                    176
                } else {
                    192
                }
            } else if meta == 2 {
                4
            } else if meta == 3 {
                16
            } else {
                6
            }
        }
        // src: Block.java:261-263
        _ => get_block_texture_from_side(id, side),
    })
}
