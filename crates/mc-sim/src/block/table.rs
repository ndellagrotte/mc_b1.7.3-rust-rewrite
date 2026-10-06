//! Block.java's static initialiser (Block.java:592-706), row by row.
//!
//! Each `const` below is one `public static final Block` field: the subclass
//! constructor it calls (the `fn`s in the first half, each transcribing that
//! class's constructor and the constructors it chains to), then the setter
//! chain in source order. Java evaluates the chain left to right on the same
//! object, so `setHardness` after `setResistance` can only raise the
//! resistance and vice versa (Block.java:186-206).
//!
//! Not in this table: the `Item.itemsList` entries the initialiser also
//! creates (Block.java:689-702; WP-1.2), `StatList.initBreakableStats()`
//! (Block.java:705; WP-2.7). `BlockFire.initializeBlock()`'s burn rates
//! (BlockFire.java:14-24, called via Block.java:700) are data and are here
//! ([`FIRE_BURN_RATES`]); the fire behaviour that reads them is WP-1.6.

use super::material::Material as M;
use super::*;

// ---- subclass constructors --------------------------------------------------

/// `BlockStone(id, tex)` and `BlockObsidian(id, tex)` (which only calls super).
// src: BlockStone.java:6-8, BlockObsidian.java:6-8
const fn block_stone(id: i32, tex: i32, kind: BlockKind) -> BlockDef {
    BlockDef::new_tex(id, tex, kind, M::Rock)
}

/// `BlockGrass(id)`.
// src: BlockGrass.java:6-10
const fn block_grass(id: i32) -> BlockDef {
    BlockDef::new(id, BlockKind::Grass, M::GrassMaterial)
        .texture(3)
        .set_tick_on_load(true)
}

/// `BlockFlower(id, tex)`; also the super constructor of sapling, tall grass,
/// dead bush, mushroom and crops.
// src: BlockFlower.java:6-12
const fn block_flower(id: i32, tex: i32, kind: BlockKind) -> BlockDef {
    let var3: f32 = 0.2;
    BlockDef::new(id, kind, M::Plants)
        .texture(tex)
        .set_tick_on_load(true)
        .set_block_bounds(
            0.5 - var3,
            0.0,
            0.5 - var3,
            0.5 + var3,
            var3 * 3.0,
            0.5 + var3,
        )
}

/// `BlockSapling(id, tex)`.
// src: BlockSapling.java:6-10
const fn block_sapling(id: i32, tex: i32) -> BlockDef {
    let var3: f32 = 0.4;
    block_flower(id, tex, BlockKind::Sapling).set_block_bounds(
        0.5 - var3,
        0.0,
        0.5 - var3,
        0.5 + var3,
        var3 * 2.0,
        0.5 + var3,
    )
}

/// `BlockTallGrass(id, tex)` and `BlockDeadBush(id, tex)` (identical bodies).
// src: BlockTallGrass.java:6-10, BlockDeadBush.java:6-10
const fn block_tall_grass(id: i32, tex: i32, kind: BlockKind) -> BlockDef {
    let var3: f32 = 0.4;
    block_flower(id, tex, kind).set_block_bounds(
        0.5 - var3,
        0.0,
        0.5 - var3,
        0.5 + var3,
        0.8,
        0.5 + var3,
    )
}

/// `BlockMushroom(id, tex)`.
// src: BlockMushroom.java:6-11
const fn block_mushroom(id: i32, tex: i32) -> BlockDef {
    let var3: f32 = 0.2;
    block_flower(id, tex, BlockKind::Mushroom)
        .set_block_bounds(
            0.5 - var3,
            0.0,
            0.5 - var3,
            0.5 + var3,
            var3 * 2.0,
            0.5 + var3,
        )
        .set_tick_on_load(true)
}

/// `BlockCrops(id, tex)`.
// src: BlockCrops.java:6-12
const fn block_crops(id: i32, tex: i32) -> BlockDef {
    let var3: f32 = 0.5;
    block_flower(id, tex, BlockKind::Crops)
        .texture(tex)
        .set_tick_on_load(true)
        .set_block_bounds(0.5 - var3, 0.0, 0.5 - var3, 0.5 + var3, 0.25, 0.5 + var3)
}

/// `BlockFluid(id, material)`: texture `(lava ? 14 : 12) * 16 + 13`.
// src: BlockFluid.java:6-12
const fn block_fluid(id: i32, material: Material, kind: BlockKind) -> BlockDef {
    let tex = (if matches!(material, M::Lava) { 14 } else { 12 }) * 16 + 13;
    let var3: f32 = 0.0;
    let var4: f32 = 0.0;
    BlockDef::new_tex(id, tex, kind, material)
        .set_block_bounds(
            0.0 + var4,
            0.0 + var3,
            0.0 + var4,
            1.0 + var4,
            1.0 + var3,
            1.0 + var4,
        )
        .set_tick_on_load(true)
}

/// `BlockFlowing(id, material)`.
// src: BlockFlowing.java:10-12
const fn block_flowing(id: i32, material: Material) -> BlockDef {
    block_fluid(id, material, BlockKind::Flowing)
}

/// `BlockStationary(id, material)`: random ticks only for lava.
// src: BlockStationary.java:6-13
const fn block_stationary(id: i32, material: Material) -> BlockDef {
    let b = block_fluid(id, material, BlockKind::Stationary).set_tick_on_load(false);
    if matches!(material, M::Lava) {
        b.set_tick_on_load(true)
    } else {
        b
    }
}

/// `BlockSand(id, tex)`; also `BlockGravel(id, tex)`.
// src: BlockSand.java:8-10, BlockGravel.java:6-8
const fn block_sand(id: i32, tex: i32, kind: BlockKind) -> BlockDef {
    BlockDef::new_tex(id, tex, kind, M::Sand)
}

/// `BlockOre(id, tex)`.
// src: BlockOre.java:6-8
const fn block_ore(id: i32, tex: i32) -> BlockDef {
    BlockDef::new_tex(id, tex, BlockKind::Ore, M::Rock)
}

/// `BlockLog(id)`.
// src: BlockLog.java:6-9
const fn block_log(id: i32) -> BlockDef {
    BlockDef::new(id, BlockKind::Log, M::Wood).texture(20)
}

/// `BlockLeaves(id, tex)` via `BlockLeavesBase(id, tex, Material.leaves, false)`.
/// `baseIndexInPNG = tex` (BlockLeaves.java:11) is the texture index again.
// src: BlockLeaves.java:9-13, BlockLeavesBase.java:6-9
const fn block_leaves(id: i32, tex: i32) -> BlockDef {
    BlockDef::new_tex(id, tex, BlockKind::Leaves, M::Leaves).set_tick_on_load(true)
}

/// `BlockSponge(id)`.
// src: BlockSponge.java:4-7
const fn block_sponge(id: i32) -> BlockDef {
    BlockDef::new(id, BlockKind::Sponge, M::Sponge).texture(48)
}

/// `BlockGlass(id, tex, material, localFlag)` via `BlockBreakable`.
// src: BlockGlass.java:6-8, BlockBreakable.java:6-9
const fn block_glass(id: i32, tex: i32, material: Material, local_flag: bool) -> BlockDef {
    BlockDef::new_tex(id, tex, BlockKind::Glass { local_flag }, material)
}

/// `BlockContainer(id, material)`.
// src: BlockContainer.java:4-7
const fn block_container(id: i32, kind: BlockKind, material: Material) -> BlockDef {
    BlockDef::new(id, kind, material).block_container()
}

/// `BlockContainer(id, tex, material)`.
// src: BlockContainer.java:9-12
const fn block_container_tex(id: i32, tex: i32, kind: BlockKind, material: Material) -> BlockDef {
    BlockDef::new_tex(id, tex, kind, material).block_container()
}

/// `BlockDispenser(id)`.
// src: BlockDispenser.java:8-11
const fn block_dispenser(id: i32) -> BlockDef {
    block_container(id, BlockKind::Dispenser, M::Rock).texture(45)
}

/// `BlockSandStone(id)`.
// src: BlockSandStone.java:4-6
const fn block_sand_stone(id: i32) -> BlockDef {
    BlockDef::new_tex(id, 192, BlockKind::SandStone, M::Rock)
}

/// `BlockNote(id)`.
// src: BlockNote.java:4-6
const fn block_note(id: i32) -> BlockDef {
    block_container_tex(id, 74, BlockKind::Note, M::Wood)
}

/// `BlockBed(id)`; `setBounds()` is `setBlockBounds(0, 0, 0, 1, 0.5625F, 1)`.
// src: BlockBed.java:9-12, BlockBed.java:141-143
const fn block_bed(id: i32) -> BlockDef {
    BlockDef::new_tex(id, 134, BlockKind::Bed, M::Cloth)
        .set_block_bounds(0.0, 0.0, 0.0, 1.0, 0.5625, 1.0)
}

/// `BlockRail(id, tex, isPowered)`; also the super constructor of the
/// detector rail (which passes `true`).
// src: BlockRail.java:17-21
const fn block_rail(id: i32, tex: i32, kind: BlockKind) -> BlockDef {
    BlockDef::new_tex(id, tex, kind, M::Circuits).set_block_bounds(0.0, 0.0, 0.0, 1.0, 0.125, 1.0)
}

/// `BlockDetectorRail(id, tex)`: `super(id, tex, true)`.
// src: BlockDetectorRail.java:7-10
const fn block_detector_rail(id: i32, tex: i32) -> BlockDef {
    block_rail(id, tex, BlockKind::DetectorRail).set_tick_on_load(true)
}

/// `BlockPistonBase(id, tex, isSticky)`.
// src: BlockPistonBase.java:9-14
const fn block_piston_base(id: i32, tex: i32, is_sticky: bool) -> BlockDef {
    BlockDef::new_tex(id, tex, BlockKind::PistonBase { is_sticky }, M::Piston)
        .set_step_sound(SOUND_STONE_FOOTSTEP)
        .set_hardness(0.5)
}

/// `BlockWeb(id, tex)`.
// src: BlockWeb.java:6-8
const fn block_web(id: i32, tex: i32) -> BlockDef {
    BlockDef::new_tex(id, tex, BlockKind::Web, M::Web)
}

/// `BlockPistonExtension(id, tex)`.
// src: BlockPistonExtension.java:9-13
const fn block_piston_extension(id: i32, tex: i32) -> BlockDef {
    BlockDef::new_tex(id, tex, BlockKind::PistonExtension, M::Piston)
        .set_step_sound(SOUND_STONE_FOOTSTEP)
        .set_hardness(0.5)
}

/// `BlockCloth()`: always id 35, texture 64.
// src: BlockCloth.java:4-6
const fn block_cloth() -> BlockDef {
    BlockDef::new_tex(35, 64, BlockKind::Cloth, M::Cloth)
}

/// `BlockPistonMoving(id)`.
// src: BlockPistonMoving.java:6-9
const fn block_piston_moving(id: i32) -> BlockDef {
    block_container(id, BlockKind::PistonMoving, M::Piston).set_hardness(-1.0)
}

/// `BlockOreStorage(id, tex)`.
// src: BlockOreStorage.java:4-7
const fn block_ore_storage(id: i32, tex: i32) -> BlockDef {
    BlockDef::new(id, BlockKind::OreStorage, M::Iron).texture(tex)
}

/// `BlockStep(id, blockType)`.
// src: BlockStep.java:9-17
const fn block_step(id: i32, block_type: bool) -> BlockDef {
    let b = BlockDef::new_tex(id, 6, BlockKind::Step { block_type }, M::Rock);
    let b = if !block_type {
        b.set_block_bounds(0.0, 0.0, 0.0, 1.0, 0.5, 1.0)
    } else {
        b
    };
    b.set_light_opacity(255)
}

/// `BlockTNT(id, tex)`.
// src: BlockTNT.java:6-8
const fn block_tnt(id: i32, tex: i32) -> BlockDef {
    BlockDef::new_tex(id, tex, BlockKind::Tnt, M::Tnt)
}

/// `BlockBookshelf(id, tex)`.
// src: BlockBookshelf.java:6-8
const fn block_bookshelf(id: i32, tex: i32) -> BlockDef {
    BlockDef::new_tex(id, tex, BlockKind::Bookshelf, M::Wood)
}

/// `BlockTorch(id, tex)`; also the super constructor of the redstone torch.
// src: BlockTorch.java:6-9
const fn block_torch(id: i32, tex: i32, kind: BlockKind) -> BlockDef {
    BlockDef::new_tex(id, tex, kind, M::Circuits).set_tick_on_load(true)
}

/// `BlockFire(id, tex)`.
// src: BlockFire.java:9-12
const fn block_fire(id: i32, tex: i32) -> BlockDef {
    BlockDef::new_tex(id, tex, BlockKind::Fire, M::Fire).set_tick_on_load(true)
}

/// `BlockMobSpawner(id, tex)`.
// src: BlockMobSpawner.java:6-8
const fn block_mob_spawner(id: i32, tex: i32) -> BlockDef {
    block_container_tex(id, tex, BlockKind::MobSpawner, M::Rock)
}

/// `BlockStairs(id, modelBlock)`: copies the model's texture, material,
/// hardness, resistance (`/ 3.0F`, re-multiplied by `setResistance`) and step
/// sound.
// src: BlockStairs.java:9-16
const fn block_stairs(id: i32, model: BlockDef) -> BlockDef {
    BlockDef::new_tex(
        id,
        model.block_index_in_texture,
        BlockKind::Stairs {
            model_block: model.block_id,
        },
        model.block_material,
    )
    .set_hardness(model.block_hardness)
    .set_resistance(model.block_resistance / 3.0)
    .set_step_sound(model.step_sound)
    .set_light_opacity(255)
}

/// `BlockChest(id)`.
// src: BlockChest.java:8-11
const fn block_chest(id: i32) -> BlockDef {
    block_container(id, BlockKind::Chest, M::Wood).texture(26)
}

/// `BlockRedstoneWire(id, tex)`.
// src: BlockRedstoneWire.java:12-15
const fn block_redstone_wire(id: i32, tex: i32) -> BlockDef {
    BlockDef::new_tex(id, tex, BlockKind::RedstoneWire, M::Circuits)
        .set_block_bounds(0.0, 0.0, 0.0, 1.0, 0.0625, 1.0)
}

/// `BlockWorkbench(id)`.
// src: BlockWorkbench.java:4-7
const fn block_workbench(id: i32) -> BlockDef {
    BlockDef::new(id, BlockKind::Workbench, M::Wood).texture(59)
}

/// `BlockFarmland(id)`.
// src: BlockFarmland.java:6-12
const fn block_farmland(id: i32) -> BlockDef {
    BlockDef::new(id, BlockKind::Farmland, M::Ground)
        .texture(87)
        .set_tick_on_load(true)
        .set_block_bounds(0.0, 0.0, 0.0, 1.0, 0.9375, 1.0)
        .set_light_opacity(255)
}

/// `BlockFurnace(id, isActive)`.
// src: BlockFurnace.java:10-14
const fn block_furnace(id: i32, is_active: bool) -> BlockDef {
    block_container(id, BlockKind::Furnace { is_active }, M::Rock).texture(45)
}

/// `BlockSign(id, TileEntitySign.class, isFreestanding)`.
// src: BlockSign.java:9-17
const fn block_sign(id: i32, is_freestanding: bool) -> BlockDef {
    let var4: f32 = 0.25;
    let var5: f32 = 1.0;
    block_container(id, BlockKind::Sign { is_freestanding }, M::Wood)
        .texture(4)
        .set_block_bounds(0.5 - var4, 0.0, 0.5 - var4, 0.5 + var4, var5, 0.5 + var4)
}

/// `BlockDoor(id, material)`: texture 97, 98 for iron.
// src: BlockDoor.java:6-16
const fn block_door(id: i32, material: Material) -> BlockDef {
    let tex = if matches!(material, M::Iron) {
        97 + 1
    } else {
        97
    };
    let var3: f32 = 0.5;
    let var4: f32 = 1.0;
    BlockDef::new(id, BlockKind::Door, material)
        .texture(tex)
        .set_block_bounds(0.5 - var3, 0.0, 0.5 - var3, 0.5 + var3, var4, 0.5 + var3)
}

/// `BlockLadder(id, tex)`.
// src: BlockLadder.java:6-8
const fn block_ladder(id: i32, tex: i32) -> BlockDef {
    BlockDef::new_tex(id, tex, BlockKind::Ladder, M::Circuits)
}

/// `BlockLever(id, tex)`.
// src: BlockLever.java:4-6
const fn block_lever(id: i32, tex: i32) -> BlockDef {
    BlockDef::new_tex(id, tex, BlockKind::Lever, M::Circuits)
}

/// `BlockPressurePlate(id, tex, triggerMobType, material)`.
// src: BlockPressurePlate.java:9-15
const fn block_pressure_plate(
    id: i32,
    tex: i32,
    trigger_mob_type: EnumMobType,
    material: Material,
) -> BlockDef {
    let var5: f32 = 0.0625;
    BlockDef::new_tex(
        id,
        tex,
        BlockKind::PressurePlate { trigger_mob_type },
        material,
    )
    .set_tick_on_load(true)
    .set_block_bounds(var5, 0.0, var5, 1.0 - var5, 0.03125, 1.0 - var5)
}

/// `BlockRedstoneOre(id, tex, glowing)`.
// src: BlockRedstoneOre.java:8-15
const fn block_redstone_ore(id: i32, tex: i32, field_468_a: bool) -> BlockDef {
    let b = BlockDef::new_tex(id, tex, BlockKind::RedstoneOre { field_468_a }, M::Rock);
    if field_468_a {
        b.set_tick_on_load(true)
    } else {
        b
    }
}

/// `BlockRedstoneTorch(id, tex, torchActive)`.
// src: BlockRedstoneTorch.java:35-39
const fn block_redstone_torch(id: i32, tex: i32, torch_active: bool) -> BlockDef {
    block_torch(id, tex, BlockKind::RedstoneTorch { torch_active }).set_tick_on_load(true)
}

/// `BlockButton(id, tex)`.
// src: BlockButton.java:6-9
const fn block_button(id: i32, tex: i32) -> BlockDef {
    BlockDef::new_tex(id, tex, BlockKind::Button, M::Circuits).set_tick_on_load(true)
}

/// `BlockSnow(id, tex)` (the snow layer).
// src: BlockSnow.java:6-10
const fn block_snow(id: i32, tex: i32) -> BlockDef {
    BlockDef::new_tex(id, tex, BlockKind::Snow, M::Snow)
        .set_block_bounds(0.0, 0.0, 0.0, 1.0, 0.125, 1.0)
        .set_tick_on_load(true)
}

/// `BlockIce(id, tex)` via `BlockBreakable(id, tex, Material.ice, false)`.
// src: BlockIce.java:6-10
const fn block_ice(id: i32, tex: i32) -> BlockDef {
    BlockDef::new_tex(id, tex, BlockKind::Ice, M::Ice)
        .slipperiness(0.98)
        .set_tick_on_load(true)
}

/// `BlockSnowBlock(id, tex)`.
// src: BlockSnowBlock.java:6-9
const fn block_snow_block(id: i32, tex: i32) -> BlockDef {
    BlockDef::new_tex(id, tex, BlockKind::SnowBlock, M::BuiltSnow).set_tick_on_load(true)
}

/// `BlockCactus(id, tex)`.
// src: BlockCactus.java:6-9
const fn block_cactus(id: i32, tex: i32) -> BlockDef {
    BlockDef::new_tex(id, tex, BlockKind::Cactus, M::Cactus).set_tick_on_load(true)
}

/// `BlockClay(id, tex)`.
// src: BlockClay.java:6-8
const fn block_clay(id: i32, tex: i32) -> BlockDef {
    BlockDef::new_tex(id, tex, BlockKind::Clay, M::Clay)
}

/// `BlockReed(id, tex)`.
// src: BlockReed.java:6-12
const fn block_reed(id: i32, tex: i32) -> BlockDef {
    let var3: f32 = 0.375;
    BlockDef::new(id, BlockKind::Reed, M::Plants)
        .texture(tex)
        .set_block_bounds(0.5 - var3, 0.0, 0.5 - var3, 0.5 + var3, 1.0, 0.5 + var3)
        .set_tick_on_load(true)
}

/// `BlockJukeBox(id, tex)`.
// src: BlockJukeBox.java:4-6
const fn block_juke_box(id: i32, tex: i32) -> BlockDef {
    block_container_tex(id, tex, BlockKind::JukeBox, M::Wood)
}

/// `BlockFence(id, tex)`.
// src: BlockFence.java:4-6
const fn block_fence(id: i32, tex: i32) -> BlockDef {
    BlockDef::new_tex(id, tex, BlockKind::Fence, M::Wood)
}

/// `BlockPumpkin(id, tex, blockType)`.
// src: BlockPumpkin.java:6-11
const fn block_pumpkin(id: i32, tex: i32, block_type: bool) -> BlockDef {
    BlockDef::new(id, BlockKind::Pumpkin { block_type }, M::Pumpkin)
        .texture(tex)
        .set_tick_on_load(true)
}

/// `BlockNetherrack(id, tex)`.
// src: BlockNetherrack.java:4-6
const fn block_netherrack(id: i32, tex: i32) -> BlockDef {
    BlockDef::new_tex(id, tex, BlockKind::Netherrack, M::Rock)
}

/// `BlockSoulSand(id, tex)`.
// src: BlockSoulSand.java:4-6
const fn block_soul_sand(id: i32, tex: i32) -> BlockDef {
    BlockDef::new_tex(id, tex, BlockKind::SoulSand, M::Sand)
}

/// `BlockGlowStone(id, tex, material)`.
// src: BlockGlowStone.java:6-8
const fn block_glow_stone(id: i32, tex: i32, material: Material) -> BlockDef {
    BlockDef::new_tex(id, tex, BlockKind::GlowStone, material)
}

/// `BlockPortal(id, tex)` via `BlockBreakable(id, tex, Material.portal, false)`.
// src: BlockPortal.java:6-8
const fn block_portal(id: i32, tex: i32) -> BlockDef {
    BlockDef::new_tex(id, tex, BlockKind::Portal, M::Portal)
}

/// `BlockCake(id, tex)`.
// src: BlockCake.java:6-9
const fn block_cake(id: i32, tex: i32) -> BlockDef {
    BlockDef::new_tex(id, tex, BlockKind::Cake, M::CakeMaterial).set_tick_on_load(true)
}

/// `BlockRedstoneRepeater(id, isRepeaterPowered)`: texture 6.
// src: BlockRedstoneRepeater.java:10-14
const fn block_redstone_repeater(id: i32, is_repeater_powered: bool) -> BlockDef {
    BlockDef::new_tex(
        id,
        6,
        BlockKind::RedstoneRepeater {
            is_repeater_powered,
        },
        M::Circuits,
    )
    .set_block_bounds(0.0, 0.0, 0.0, 1.0, 0.125, 1.0)
}

/// `BlockLockedChest(id)`.
// src: BlockLockedChest.java:6-9
const fn block_locked_chest(id: i32) -> BlockDef {
    BlockDef::new(id, BlockKind::LockedChest, M::Wood).texture(26)
}

/// `BlockTrapDoor(id, material)`: texture 84, 85 for iron.
// src: BlockTrapDoor.java:4-14
const fn block_trap_door(id: i32, material: Material) -> BlockDef {
    let tex = if matches!(material, M::Iron) {
        84 + 1
    } else {
        84
    };
    let var3: f32 = 0.5;
    let var4: f32 = 1.0;
    BlockDef::new(id, BlockKind::TrapDoor, material)
        .texture(tex)
        .set_block_bounds(0.5 - var3, 0.0, 0.5 - var3, 0.5 + var3, var4, 0.5 + var3)
}

// ---- the static initialiser -------------------------------------------------

// src: Block.java:593
const STONE: BlockDef = block_stone(1, 1, BlockKind::Stone)
    .set_hardness(1.5)
    .set_resistance(10.0)
    .set_step_sound(SOUND_STONE_FOOTSTEP)
    .set_block_name("stone");
// src: Block.java:594
const GRASS: BlockDef = block_grass(2)
    .set_hardness(0.6)
    .set_step_sound(SOUND_GRASS_FOOTSTEP)
    .set_block_name("grass");
// src: Block.java:595
const DIRT: BlockDef = BlockDef::new_tex(3, 2, BlockKind::Dirt, M::Ground)
    .set_hardness(0.5)
    .set_step_sound(SOUND_GRAVEL_FOOTSTEP)
    .set_block_name("dirt");
// src: Block.java:596
const COBBLESTONE: BlockDef = BlockDef::new_tex(4, 16, BlockKind::Plain, M::Rock)
    .set_hardness(2.0)
    .set_resistance(10.0)
    .set_step_sound(SOUND_STONE_FOOTSTEP)
    .set_block_name("stonebrick");
// src: Block.java:597
const PLANKS: BlockDef = BlockDef::new_tex(5, 4, BlockKind::Plain, M::Wood)
    .set_hardness(2.0)
    .set_resistance(5.0)
    .set_step_sound(SOUND_WOOD_FOOTSTEP)
    .set_block_name("wood")
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:598
const SAPLING: BlockDef = block_sapling(6, 15)
    .set_hardness(0.0)
    .set_step_sound(SOUND_GRASS_FOOTSTEP)
    .set_block_name("sapling")
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:599
const BEDROCK: BlockDef = BlockDef::new_tex(7, 17, BlockKind::Plain, M::Rock)
    .set_block_unbreakable()
    .set_resistance(6000000.0)
    .set_step_sound(SOUND_STONE_FOOTSTEP)
    .set_block_name("bedrock")
    .disable_stats();
// src: Block.java:600
const WATER_MOVING: BlockDef = block_flowing(8, M::Water)
    .set_hardness(100.0)
    .set_light_opacity(3)
    .set_block_name("water")
    .disable_stats()
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:601
const WATER_STILL: BlockDef = block_stationary(9, M::Water)
    .set_hardness(100.0)
    .set_light_opacity(3)
    .set_block_name("water")
    .disable_stats()
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:602
const LAVA_MOVING: BlockDef = block_flowing(10, M::Lava)
    .set_hardness(0.0)
    .set_light_value(1.0)
    .set_light_opacity(255)
    .set_block_name("lava")
    .disable_stats()
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:603
const LAVA_STILL: BlockDef = block_stationary(11, M::Lava)
    .set_hardness(100.0)
    .set_light_value(1.0)
    .set_light_opacity(255)
    .set_block_name("lava")
    .disable_stats()
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:604
const SAND: BlockDef = block_sand(12, 18, BlockKind::Sand)
    .set_hardness(0.5)
    .set_step_sound(SOUND_SAND_FOOTSTEP)
    .set_block_name("sand");
// src: Block.java:605
const GRAVEL: BlockDef = block_sand(13, 19, BlockKind::Gravel)
    .set_hardness(0.6)
    .set_step_sound(SOUND_GRAVEL_FOOTSTEP)
    .set_block_name("gravel");
// src: Block.java:606
const ORE_GOLD: BlockDef = block_ore(14, 32)
    .set_hardness(3.0)
    .set_resistance(5.0)
    .set_step_sound(SOUND_STONE_FOOTSTEP)
    .set_block_name("oreGold");
// src: Block.java:607
const ORE_IRON: BlockDef = block_ore(15, 33)
    .set_hardness(3.0)
    .set_resistance(5.0)
    .set_step_sound(SOUND_STONE_FOOTSTEP)
    .set_block_name("oreIron");
// src: Block.java:608
const ORE_COAL: BlockDef = block_ore(16, 34)
    .set_hardness(3.0)
    .set_resistance(5.0)
    .set_step_sound(SOUND_STONE_FOOTSTEP)
    .set_block_name("oreCoal");
// src: Block.java:609
const WOOD: BlockDef = block_log(17)
    .set_hardness(2.0)
    .set_step_sound(SOUND_WOOD_FOOTSTEP)
    .set_block_name("log")
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:610
const LEAVES: BlockDef = block_leaves(18, 52)
    .set_hardness(0.2)
    .set_light_opacity(1)
    .set_step_sound(SOUND_GRASS_FOOTSTEP)
    .set_block_name("leaves")
    .disable_stats()
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:611
const SPONGE: BlockDef = block_sponge(19)
    .set_hardness(0.6)
    .set_step_sound(SOUND_GRASS_FOOTSTEP)
    .set_block_name("sponge");
// src: Block.java:612
const GLASS: BlockDef = block_glass(20, 49, M::Glass, false)
    .set_hardness(0.3)
    .set_step_sound(SOUND_GLASS_FOOTSTEP)
    .set_block_name("glass");
// src: Block.java:613
const ORE_LAPIS: BlockDef = block_ore(21, 160)
    .set_hardness(3.0)
    .set_resistance(5.0)
    .set_step_sound(SOUND_STONE_FOOTSTEP)
    .set_block_name("oreLapis");
// src: Block.java:614
const BLOCK_LAPIS: BlockDef = BlockDef::new_tex(22, 144, BlockKind::Plain, M::Rock)
    .set_hardness(3.0)
    .set_resistance(5.0)
    .set_step_sound(SOUND_STONE_FOOTSTEP)
    .set_block_name("blockLapis");
// src: Block.java:615
const DISPENSER: BlockDef = block_dispenser(23)
    .set_hardness(3.5)
    .set_step_sound(SOUND_STONE_FOOTSTEP)
    .set_block_name("dispenser")
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:616
const SAND_STONE: BlockDef = block_sand_stone(24)
    .set_step_sound(SOUND_STONE_FOOTSTEP)
    .set_hardness(0.8)
    .set_block_name("sandStone");
// src: Block.java:617
const MUSIC_BLOCK: BlockDef = block_note(25)
    .set_hardness(0.8)
    .set_block_name("musicBlock")
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:618
const BLOCK_BED: BlockDef = block_bed(26)
    .set_hardness(0.2)
    .set_block_name("bed")
    .disable_stats()
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:619
const RAIL_POWERED: BlockDef = block_rail(27, 179, BlockKind::Rail { is_powered: true })
    .set_hardness(0.7)
    .set_step_sound(SOUND_METAL_FOOTSTEP)
    .set_block_name("goldenRail")
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:620
const RAIL_DETECTOR: BlockDef = block_detector_rail(28, 195)
    .set_hardness(0.7)
    .set_step_sound(SOUND_METAL_FOOTSTEP)
    .set_block_name("detectorRail")
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:621
const PISTON_STICKY_BASE: BlockDef = block_piston_base(29, 106, true)
    .set_block_name("pistonStickyBase")
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:622
const WEB: BlockDef = block_web(30, 11)
    .set_light_opacity(1)
    .set_hardness(4.0)
    .set_block_name("web");
// src: Block.java:623
const TALL_GRASS: BlockDef = block_tall_grass(31, 39, BlockKind::TallGrass)
    .set_hardness(0.0)
    .set_step_sound(SOUND_GRASS_FOOTSTEP)
    .set_block_name("tallgrass");
// src: Block.java:624
const DEAD_BUSH: BlockDef = block_tall_grass(32, 55, BlockKind::DeadBush)
    .set_hardness(0.0)
    .set_step_sound(SOUND_GRASS_FOOTSTEP)
    .set_block_name("deadbush");
// src: Block.java:625
const PISTON_BASE: BlockDef = block_piston_base(33, 107, false)
    .set_block_name("pistonBase")
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:626 (no setBlockName: blockName stays null)
const PISTON_EXTENSION: BlockDef =
    block_piston_extension(34, 107).disable_neighbor_notify_on_metadata_change();
// src: Block.java:627
const CLOTH: BlockDef = block_cloth()
    .set_hardness(0.8)
    .set_step_sound(SOUND_CLOTH_FOOTSTEP)
    .set_block_name("cloth")
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:628 (no setter chain at all)
const PISTON_MOVING: BlockDef = block_piston_moving(36);
// src: Block.java:629
const PLANT_YELLOW: BlockDef = block_flower(37, 13, BlockKind::Flower)
    .set_hardness(0.0)
    .set_step_sound(SOUND_GRASS_FOOTSTEP)
    .set_block_name("flower");
// src: Block.java:630
const PLANT_RED: BlockDef = block_flower(38, 12, BlockKind::Flower)
    .set_hardness(0.0)
    .set_step_sound(SOUND_GRASS_FOOTSTEP)
    .set_block_name("rose");
// src: Block.java:631
const MUSHROOM_BROWN: BlockDef = block_mushroom(39, 29)
    .set_hardness(0.0)
    .set_step_sound(SOUND_GRASS_FOOTSTEP)
    .set_light_value(0.125)
    .set_block_name("mushroom");
// src: Block.java:632
const MUSHROOM_RED: BlockDef = block_mushroom(40, 28)
    .set_hardness(0.0)
    .set_step_sound(SOUND_GRASS_FOOTSTEP)
    .set_block_name("mushroom");
// src: Block.java:633
const BLOCK_GOLD: BlockDef = block_ore_storage(41, 23)
    .set_hardness(3.0)
    .set_resistance(10.0)
    .set_step_sound(SOUND_METAL_FOOTSTEP)
    .set_block_name("blockGold");
// src: Block.java:634
const BLOCK_STEEL: BlockDef = block_ore_storage(42, 22)
    .set_hardness(5.0)
    .set_resistance(10.0)
    .set_step_sound(SOUND_METAL_FOOTSTEP)
    .set_block_name("blockIron");
// src: Block.java:635
const STAIR_DOUBLE: BlockDef = block_step(43, true)
    .set_hardness(2.0)
    .set_resistance(10.0)
    .set_step_sound(SOUND_STONE_FOOTSTEP)
    .set_block_name("stoneSlab");
// src: Block.java:636
const STAIR_SINGLE: BlockDef = block_step(44, false)
    .set_hardness(2.0)
    .set_resistance(10.0)
    .set_step_sound(SOUND_STONE_FOOTSTEP)
    .set_block_name("stoneSlab");
// src: Block.java:637
const BRICK: BlockDef = BlockDef::new_tex(45, 7, BlockKind::Plain, M::Rock)
    .set_hardness(2.0)
    .set_resistance(10.0)
    .set_step_sound(SOUND_STONE_FOOTSTEP)
    .set_block_name("brick");
// src: Block.java:638
const TNT: BlockDef = block_tnt(46, 8)
    .set_hardness(0.0)
    .set_step_sound(SOUND_GRASS_FOOTSTEP)
    .set_block_name("tnt");
// src: Block.java:639
const BOOK_SHELF: BlockDef = block_bookshelf(47, 35)
    .set_hardness(1.5)
    .set_step_sound(SOUND_WOOD_FOOTSTEP)
    .set_block_name("bookshelf");
// src: Block.java:640
const COBBLESTONE_MOSSY: BlockDef = BlockDef::new_tex(48, 36, BlockKind::Plain, M::Rock)
    .set_hardness(2.0)
    .set_resistance(10.0)
    .set_step_sound(SOUND_STONE_FOOTSTEP)
    .set_block_name("stoneMoss");
// src: Block.java:641
const OBSIDIAN: BlockDef = block_stone(49, 37, BlockKind::Obsidian)
    .set_hardness(10.0)
    .set_resistance(2000.0)
    .set_step_sound(SOUND_STONE_FOOTSTEP)
    .set_block_name("obsidian");
// src: Block.java:642
const TORCH_WOOD: BlockDef = block_torch(50, 80, BlockKind::Torch)
    .set_hardness(0.0)
    .set_light_value(0.9375)
    .set_step_sound(SOUND_WOOD_FOOTSTEP)
    .set_block_name("torch")
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:643
const FIRE: BlockDef = block_fire(51, 31)
    .set_hardness(0.0)
    .set_light_value(1.0)
    .set_step_sound(SOUND_WOOD_FOOTSTEP)
    .set_block_name("fire")
    .disable_stats()
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:644
const MOB_SPAWNER: BlockDef = block_mob_spawner(52, 65)
    .set_hardness(5.0)
    .set_step_sound(SOUND_METAL_FOOTSTEP)
    .set_block_name("mobSpawner")
    .disable_stats();
// src: Block.java:645
const STAIR_COMPACT_PLANKS: BlockDef = block_stairs(53, PLANKS)
    .set_block_name("stairsWood")
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:646
const CHEST: BlockDef = block_chest(54)
    .set_hardness(2.5)
    .set_step_sound(SOUND_WOOD_FOOTSTEP)
    .set_block_name("chest")
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:647
const REDSTONE_WIRE: BlockDef = block_redstone_wire(55, 164)
    .set_hardness(0.0)
    .set_step_sound(SOUND_POWDER_FOOTSTEP)
    .set_block_name("redstoneDust")
    .disable_stats()
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:648
const ORE_DIAMOND: BlockDef = block_ore(56, 50)
    .set_hardness(3.0)
    .set_resistance(5.0)
    .set_step_sound(SOUND_STONE_FOOTSTEP)
    .set_block_name("oreDiamond");
// src: Block.java:649
const BLOCK_DIAMOND: BlockDef = block_ore_storage(57, 24)
    .set_hardness(5.0)
    .set_resistance(10.0)
    .set_step_sound(SOUND_METAL_FOOTSTEP)
    .set_block_name("blockDiamond");
// src: Block.java:650
const WORKBENCH: BlockDef = block_workbench(58)
    .set_hardness(2.5)
    .set_step_sound(SOUND_WOOD_FOOTSTEP)
    .set_block_name("workbench");
// src: Block.java:651
const CROPS: BlockDef = block_crops(59, 88)
    .set_hardness(0.0)
    .set_step_sound(SOUND_GRASS_FOOTSTEP)
    .set_block_name("crops")
    .disable_stats()
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:652
const TILLED_FIELD: BlockDef = block_farmland(60)
    .set_hardness(0.6)
    .set_step_sound(SOUND_GRAVEL_FOOTSTEP)
    .set_block_name("farmland");
// src: Block.java:653
const STONE_OVEN_IDLE: BlockDef = block_furnace(61, false)
    .set_hardness(3.5)
    .set_step_sound(SOUND_STONE_FOOTSTEP)
    .set_block_name("furnace")
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:654
const STONE_OVEN_ACTIVE: BlockDef = block_furnace(62, true)
    .set_hardness(3.5)
    .set_step_sound(SOUND_STONE_FOOTSTEP)
    .set_light_value(0.875)
    .set_block_name("furnace")
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:655
const SIGN_POST: BlockDef = block_sign(63, true)
    .set_hardness(1.0)
    .set_step_sound(SOUND_WOOD_FOOTSTEP)
    .set_block_name("sign")
    .disable_stats()
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:656
const DOOR_WOOD: BlockDef = block_door(64, M::Wood)
    .set_hardness(3.0)
    .set_step_sound(SOUND_WOOD_FOOTSTEP)
    .set_block_name("doorWood")
    .disable_stats()
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:657
const LADDER: BlockDef = block_ladder(65, 83)
    .set_hardness(0.4)
    .set_step_sound(SOUND_WOOD_FOOTSTEP)
    .set_block_name("ladder")
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:658
const RAIL: BlockDef = block_rail(66, 128, BlockKind::Rail { is_powered: false })
    .set_hardness(0.7)
    .set_step_sound(SOUND_METAL_FOOTSTEP)
    .set_block_name("rail")
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:659
const STAIR_COMPACT_COBBLESTONE: BlockDef = block_stairs(67, COBBLESTONE)
    .set_block_name("stairsStone")
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:660
const SIGN_WALL: BlockDef = block_sign(68, false)
    .set_hardness(1.0)
    .set_step_sound(SOUND_WOOD_FOOTSTEP)
    .set_block_name("sign")
    .disable_stats()
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:661
const LEVER: BlockDef = block_lever(69, 96)
    .set_hardness(0.5)
    .set_step_sound(SOUND_WOOD_FOOTSTEP)
    .set_block_name("lever")
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:662
const PRESSURE_PLATE_STONE: BlockDef =
    block_pressure_plate(70, STONE.block_index_in_texture, EnumMobType::Mobs, M::Rock)
        .set_hardness(0.5)
        .set_step_sound(SOUND_STONE_FOOTSTEP)
        .set_block_name("pressurePlate")
        .disable_neighbor_notify_on_metadata_change();
// src: Block.java:663
const DOOR_STEEL: BlockDef = block_door(71, M::Iron)
    .set_hardness(5.0)
    .set_step_sound(SOUND_METAL_FOOTSTEP)
    .set_block_name("doorIron")
    .disable_stats()
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:664
const PRESSURE_PLATE_PLANKS: BlockDef = block_pressure_plate(
    72,
    PLANKS.block_index_in_texture,
    EnumMobType::Everything,
    M::Wood,
)
.set_hardness(0.5)
.set_step_sound(SOUND_WOOD_FOOTSTEP)
.set_block_name("pressurePlate")
.disable_neighbor_notify_on_metadata_change();
// src: Block.java:665
const ORE_REDSTONE: BlockDef = block_redstone_ore(73, 51, false)
    .set_hardness(3.0)
    .set_resistance(5.0)
    .set_step_sound(SOUND_STONE_FOOTSTEP)
    .set_block_name("oreRedstone")
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:666
const ORE_REDSTONE_GLOWING: BlockDef = block_redstone_ore(74, 51, true)
    .set_light_value(0.625)
    .set_hardness(3.0)
    .set_resistance(5.0)
    .set_step_sound(SOUND_STONE_FOOTSTEP)
    .set_block_name("oreRedstone")
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:667
const TORCH_REDSTONE_IDLE: BlockDef = block_redstone_torch(75, 115, false)
    .set_hardness(0.0)
    .set_step_sound(SOUND_WOOD_FOOTSTEP)
    .set_block_name("notGate")
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:668
const TORCH_REDSTONE_ACTIVE: BlockDef = block_redstone_torch(76, 99, true)
    .set_hardness(0.0)
    .set_light_value(0.5)
    .set_step_sound(SOUND_WOOD_FOOTSTEP)
    .set_block_name("notGate")
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:669
const BUTTON: BlockDef = block_button(77, STONE.block_index_in_texture)
    .set_hardness(0.5)
    .set_step_sound(SOUND_STONE_FOOTSTEP)
    .set_block_name("button")
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:670
const SNOW: BlockDef = block_snow(78, 66)
    .set_hardness(0.1)
    .set_step_sound(SOUND_CLOTH_FOOTSTEP)
    .set_block_name("snow");
// src: Block.java:671
const ICE: BlockDef = block_ice(79, 67)
    .set_hardness(0.5)
    .set_light_opacity(3)
    .set_step_sound(SOUND_GLASS_FOOTSTEP)
    .set_block_name("ice");
// src: Block.java:672
const BLOCK_SNOW: BlockDef = block_snow_block(80, 66)
    .set_hardness(0.2)
    .set_step_sound(SOUND_CLOTH_FOOTSTEP)
    .set_block_name("snow");
// src: Block.java:673
const CACTUS: BlockDef = block_cactus(81, 70)
    .set_hardness(0.4)
    .set_step_sound(SOUND_CLOTH_FOOTSTEP)
    .set_block_name("cactus");
// src: Block.java:674
const BLOCK_CLAY: BlockDef = block_clay(82, 72)
    .set_hardness(0.6)
    .set_step_sound(SOUND_GRAVEL_FOOTSTEP)
    .set_block_name("clay");
// src: Block.java:675
const REED: BlockDef = block_reed(83, 73)
    .set_hardness(0.0)
    .set_step_sound(SOUND_GRASS_FOOTSTEP)
    .set_block_name("reeds")
    .disable_stats();
// src: Block.java:676
const JUKEBOX: BlockDef = block_juke_box(84, 74)
    .set_hardness(2.0)
    .set_resistance(10.0)
    .set_step_sound(SOUND_STONE_FOOTSTEP)
    .set_block_name("jukebox")
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:677
const FENCE: BlockDef = block_fence(85, 4)
    .set_hardness(2.0)
    .set_resistance(5.0)
    .set_step_sound(SOUND_WOOD_FOOTSTEP)
    .set_block_name("fence")
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:678
const PUMPKIN: BlockDef = block_pumpkin(86, 102, false)
    .set_hardness(1.0)
    .set_step_sound(SOUND_WOOD_FOOTSTEP)
    .set_block_name("pumpkin")
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:679
const NETHERRACK: BlockDef = block_netherrack(87, 103)
    .set_hardness(0.4)
    .set_step_sound(SOUND_STONE_FOOTSTEP)
    .set_block_name("hellrock");
// src: Block.java:680
const SLOW_SAND: BlockDef = block_soul_sand(88, 104)
    .set_hardness(0.5)
    .set_step_sound(SOUND_SAND_FOOTSTEP)
    .set_block_name("hellsand");
// src: Block.java:681
const GLOW_STONE: BlockDef = block_glow_stone(89, 105, M::Rock)
    .set_hardness(0.3)
    .set_step_sound(SOUND_GLASS_FOOTSTEP)
    .set_light_value(1.0)
    .set_block_name("lightgem");
// src: Block.java:682
const PORTAL: BlockDef = block_portal(90, 14)
    .set_hardness(-1.0)
    .set_step_sound(SOUND_GLASS_FOOTSTEP)
    .set_light_value(0.75)
    .set_block_name("portal");
// src: Block.java:683
const PUMPKIN_LANTERN: BlockDef = block_pumpkin(91, 102, true)
    .set_hardness(1.0)
    .set_step_sound(SOUND_WOOD_FOOTSTEP)
    .set_light_value(1.0)
    .set_block_name("litpumpkin")
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:684
const CAKE: BlockDef = block_cake(92, 121)
    .set_hardness(0.5)
    .set_step_sound(SOUND_CLOTH_FOOTSTEP)
    .set_block_name("cake")
    .disable_stats()
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:685
const REDSTONE_REPEATER_IDLE: BlockDef = block_redstone_repeater(93, false)
    .set_hardness(0.0)
    .set_step_sound(SOUND_WOOD_FOOTSTEP)
    .set_block_name("diode")
    .disable_stats()
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:686
const REDSTONE_REPEATER_ACTIVE: BlockDef = block_redstone_repeater(94, true)
    .set_hardness(0.0)
    .set_light_value(0.625)
    .set_step_sound(SOUND_WOOD_FOOTSTEP)
    .set_block_name("diode")
    .disable_stats()
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:687
const LOCKED_CHEST: BlockDef = block_locked_chest(95)
    .set_hardness(0.0)
    .set_light_value(1.0)
    .set_step_sound(SOUND_WOOD_FOOTSTEP)
    .set_block_name("lockedchest")
    .set_tick_on_load(true)
    .disable_neighbor_notify_on_metadata_change();
// src: Block.java:688
const TRAPDOOR: BlockDef = block_trap_door(96, M::Wood)
    .set_hardness(3.0)
    .set_step_sound(SOUND_WOOD_FOOTSTEP)
    .set_block_name("trapdoor")
    .disable_stats()
    .disable_neighbor_notify_on_metadata_change();

/// `BlockFire.initializeBlock()`'s `setBurnRate(id, encourage, catch)` calls,
/// applied to two zeroed `int[256]`: `(chanceToEncourageFire,
/// abilityToCatchFire)`. Block.java:697-702 calls `initializeBlock` on every
/// block that has no item yet; only `BlockFire` overrides it.
// src: BlockFire.java:14-29
pub(super) const FIRE_BURN_RATES: ([i32; 256], [i32; 256]) = {
    let rates: [(i32, i32, i32); 9] = [
        (PLANKS.block_id, 5, 20),
        (FENCE.block_id, 5, 20),
        (STAIR_COMPACT_PLANKS.block_id, 5, 20),
        (WOOD.block_id, 5, 5),
        (LEAVES.block_id, 30, 60),
        (BOOK_SHELF.block_id, 30, 20),
        (TNT.block_id, 15, 100),
        (TALL_GRASS.block_id, 60, 100),
        (CLOTH.block_id, 30, 60),
    ];
    let mut encourage = [0i32; 256];
    let mut catch = [0i32; 256];
    let mut i = 0;
    while i < rates.len() {
        let (id, e, c) = rates[i];
        encourage[id as usize] = e;
        catch[id as usize] = c;
        i += 1;
    }
    (encourage, catch)
};

/// Every row in initialiser order.
const ROWS: [BlockDef; 96] = [
    STONE,
    GRASS,
    DIRT,
    COBBLESTONE,
    PLANKS,
    SAPLING,
    BEDROCK,
    WATER_MOVING,
    WATER_STILL,
    LAVA_MOVING,
    LAVA_STILL,
    SAND,
    GRAVEL,
    ORE_GOLD,
    ORE_IRON,
    ORE_COAL,
    WOOD,
    LEAVES,
    SPONGE,
    GLASS,
    ORE_LAPIS,
    BLOCK_LAPIS,
    DISPENSER,
    SAND_STONE,
    MUSIC_BLOCK,
    BLOCK_BED,
    RAIL_POWERED,
    RAIL_DETECTOR,
    PISTON_STICKY_BASE,
    WEB,
    TALL_GRASS,
    DEAD_BUSH,
    PISTON_BASE,
    PISTON_EXTENSION,
    CLOTH,
    PISTON_MOVING,
    PLANT_YELLOW,
    PLANT_RED,
    MUSHROOM_BROWN,
    MUSHROOM_RED,
    BLOCK_GOLD,
    BLOCK_STEEL,
    STAIR_DOUBLE,
    STAIR_SINGLE,
    BRICK,
    TNT,
    BOOK_SHELF,
    COBBLESTONE_MOSSY,
    OBSIDIAN,
    TORCH_WOOD,
    FIRE,
    MOB_SPAWNER,
    STAIR_COMPACT_PLANKS,
    CHEST,
    REDSTONE_WIRE,
    ORE_DIAMOND,
    BLOCK_DIAMOND,
    WORKBENCH,
    CROPS,
    TILLED_FIELD,
    STONE_OVEN_IDLE,
    STONE_OVEN_ACTIVE,
    SIGN_POST,
    DOOR_WOOD,
    LADDER,
    RAIL,
    STAIR_COMPACT_COBBLESTONE,
    SIGN_WALL,
    LEVER,
    PRESSURE_PLATE_STONE,
    DOOR_STEEL,
    PRESSURE_PLATE_PLANKS,
    ORE_REDSTONE,
    ORE_REDSTONE_GLOWING,
    TORCH_REDSTONE_IDLE,
    TORCH_REDSTONE_ACTIVE,
    BUTTON,
    SNOW,
    ICE,
    BLOCK_SNOW,
    CACTUS,
    BLOCK_CLAY,
    REED,
    JUKEBOX,
    FENCE,
    PUMPKIN,
    NETHERRACK,
    SLOW_SAND,
    GLOW_STONE,
    PORTAL,
    PUMPKIN_LANTERN,
    CAKE,
    REDSTONE_REPEATER_IDLE,
    REDSTONE_REPEATER_ACTIVE,
    LOCKED_CHEST,
    TRAPDOOR,
];

/// `blocksList`, filled as the `Block` constructor does (`blocksList[id] =
/// this`, Block.java:148). A duplicate id panics at compile time, like the
/// constructor's `IllegalArgumentException` (Block.java:144-145).
pub(super) const fn build() -> [Option<BlockDef>; 256] {
    let mut t: [Option<BlockDef>; 256] = [None; 256];
    let mut i = 0;
    while i < ROWS.len() {
        let row = ROWS[i];
        let id = row.block_id as usize;
        assert!(t[id].is_none(), "Block slot already occupied");
        t[id] = Some(row);
        i += 1;
    }
    t
}
