//! Block registry and behaviour (`Block.java` and its subclasses, PLAN_v4 §3.2).
//!
//! # Shape
//!
//! - [`BlockDef`]: the data a `Block` instance carries after the static
//!   initialiser has run (Block.java:592-706): id, [`BlockKind`], material,
//!   texture index, hardness, resistance, step sound, bounds, name, plus this
//!   id's entry in each of Block.java's per-id arrays (`tickOnLoad`,
//!   `opaqueCubeLookup`, `isBlockContainer`, `lightOpacity`, `canBlockGrass`,
//!   `lightValue`, `setNeighborNotifyOnMetadataChange`, Block.java:17-23).
//! - [`BLOCKS`] / [`block`]: `Block.blocksList` as an immutable table built at
//!   compile time from `table.rs`, which transcribes every row of the static
//!   initialiser and every subclass constructor it calls.
//! - [`BlockKind`]: one variant per `Block` subclass the initialiser actually
//!   instantiates (69, counting `Block` itself as [`BlockKind::Plain`]). A
//!   variant carries the constructor arguments that make two instances of the
//!   same class behave differently (`BlockStep.blockType`, `BlockRail.isPowered`,
//!   …), so behaviour can `match` on them directly.
//! - Data-only methods as free functions over a row (every override is a
//!   constant, a field read or a call on another row): [`is_opaque_cube`],
//!   [`render_as_normal_block`], [`get_render_type`], [`get_render_block_pass`],
//!   [`is_collidable`], [`BlockDef::get_mobility_flag`], [`tick_rate`],
//!   [`BlockDef::get_block_name`], [`damage_dropped`], [`id_dropped`],
//!   [`quantity_dropped`], [`get_block_texture_from_side`],
//!   [`get_block_texture_from_side_and_metadata`], plus `BlockFire`'s burn
//!   tables [`CHANCE_TO_ENCOURAGE_FIRE`] / [`ABILITY_TO_CATCH_FIRE`]. All of it,
//!   with every instance field and the per-id arrays, is checked against the
//!   oracle by `tests/golden/block/block-table` (`tests/golden_block_table.rs`).
//! - [`dispatch`]: behaviour as free functions `fn(world, x, y, z, …)` that
//!   `match` on the kind. No `Box<dyn Block>`.
//! - [`BlockStatics`]: the parts of the `Block` singletons that vanilla
//!   *mutates* at run time (bounds, `graphicsLevel`, …). They are world state,
//!   so they live in [`crate::world::World::block_statics`], not in the
//!   immutable table.
//!
//! # Per-id arrays and how vanilla fills them
//!
//! The `Block(int, Material)` constructor (Block.java:138-156) sets, for its id,
//! `opaqueCubeLookup = isOpaqueCube()`, `lightOpacity = isOpaqueCube() ? 255 : 0`,
//! `canBlockGrass = !material.getCanBlockGrass()` and `isBlockContainer = false`.
//! `isOpaqueCube()` there is a virtual call made *before* the subclass's field
//! initialisers and constructor body have run, which matters twice (vanilla
//! quirks, transcribed): `BlockStep.isOpaqueCube()` returns the not-yet-assigned
//! `blockType` (false) for both slab ids 43 and 44, and `BlockLeaves.isOpaqueCube()`
//! returns `!graphicsLevel` = true for leaves (BlockStep.java:39-41,
//! BlockLeaves.java:177-179, BlockLeavesBase.java:6-9). Afterwards
//! `setLightOpacity`, `setLightValue` (`(int)(15.0F * v)`), `setTickOnLoad` and
//! `disableNeighborNotifyOnMetadataChange` overwrite single entries,
//! `BlockContainer`'s constructors set `isBlockContainer` (BlockContainer.java:4-12),
//! and finally `canBlockGrass[0] = true` (Block.java:704) for air, which has no
//! row. Ids without a block read as 0 / false.
//!
//! The array named `requiresSelfNotify` in later MCP mappings is
//! `setNeighborNotifyOnMetadataChange` in this tree (Block.java:23); the Java
//! name is kept.

pub mod block_texture;
pub mod button;
pub mod cactus;
pub mod crops;
pub mod dispatch;
pub mod farmland;
pub mod fire;
pub mod flower;
pub mod fluid;
pub mod grass;
pub mod leaves;
pub mod lever;
pub mod material;
pub mod pressure_plate;
pub mod redstone_ore;
pub mod redstone_torch;
pub mod redstone_wire;
pub mod repeater;
pub mod sand;
pub mod sapling;
pub mod snow;
mod table;
pub mod torch;

pub use material::{MapColor, Material};

use mc_core::JavaRandom;
use mc_core::java_hash::{ChunkPosition, JavaHashSet};

/// A block id as stored in `Chunk.blocks` (a Java `byte`, read as `& 255`).
/// Code that compares ids the way Java does uses `i32`.
pub type BlockId = u8;

/// `StepSound` and its two anonymous-ish subclasses (StepSound.java,
/// StepSoundStone.java, StepSoundSand.java). The three `field_` names are
/// renamed; see `docs/mappings.md`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepSound {
    /// `field_1678_a`: the sound name suffix (`"step." + name`).
    pub step_sound_name: &'static str,
    /// `field_1677_b`: returned by `getVolume()`.
    pub step_sound_volume: f32,
    /// `field_1679_c`: returned by `getPitch()`.
    pub step_sound_pitch: f32,
    /// Which class overrides `stepSoundDir()`.
    pub class: StepSoundClass,
}

/// The `StepSound` class an instance belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepSoundClass {
    /// `StepSound`.
    Plain,
    /// `StepSoundStone`: `stepSoundDir()` is `"random.glass"` (StepSoundStone.java:8-10).
    Stone,
    /// `StepSoundSand`: `stepSoundDir()` is `"step.gravel"` (StepSoundSand.java:8-10).
    Sand,
}

impl StepSoundClass {
    /// The Java class name.
    pub const fn java_class(self) -> &'static str {
        match self {
            StepSoundClass::Plain => "StepSound",
            StepSoundClass::Stone => "StepSoundStone",
            StepSoundClass::Sand => "StepSoundSand",
        }
    }
}

impl StepSound {
    const fn new(name: &'static str, volume: f32, pitch: f32, class: StepSoundClass) -> Self {
        StepSound {
            step_sound_name: name,
            step_sound_volume: volume,
            step_sound_pitch: pitch,
            class,
        }
    }

    /// `getVolume()`.
    // src: StepSound.java:14-16
    pub const fn get_volume(&self) -> f32 {
        self.step_sound_volume
    }

    /// `getPitch()`.
    // src: StepSound.java:18-20
    pub const fn get_pitch(&self) -> f32 {
        self.step_sound_pitch
    }

    /// `stepSoundDir()` (the break/place sound).
    // src: StepSound.java:22-24, StepSoundStone.java:8-10, StepSoundSand.java:8-10
    pub fn step_sound_dir(&self) -> String {
        match self.class {
            StepSoundClass::Plain => format!("step.{}", self.step_sound_name),
            StepSoundClass::Stone => "random.glass".to_string(),
            StepSoundClass::Sand => "step.gravel".to_string(),
        }
    }

    /// `stepSoundDir2()` (the footstep sound; never overridden).
    // src: StepSound.java:26-28
    pub fn step_sound_dir2(&self) -> String {
        format!("step.{}", self.step_sound_name)
    }
}

// src: Block.java:7-15
pub const SOUND_POWDER_FOOTSTEP: StepSound =
    StepSound::new("stone", 1.0, 1.0, StepSoundClass::Plain);
pub const SOUND_WOOD_FOOTSTEP: StepSound = StepSound::new("wood", 1.0, 1.0, StepSoundClass::Plain);
pub const SOUND_GRAVEL_FOOTSTEP: StepSound =
    StepSound::new("gravel", 1.0, 1.0, StepSoundClass::Plain);
pub const SOUND_GRASS_FOOTSTEP: StepSound =
    StepSound::new("grass", 1.0, 1.0, StepSoundClass::Plain);
pub const SOUND_STONE_FOOTSTEP: StepSound =
    StepSound::new("stone", 1.0, 1.0, StepSoundClass::Plain);
pub const SOUND_METAL_FOOTSTEP: StepSound =
    StepSound::new("stone", 1.0, 1.5, StepSoundClass::Plain);
pub const SOUND_GLASS_FOOTSTEP: StepSound =
    StepSound::new("stone", 1.0, 1.0, StepSoundClass::Stone);
pub const SOUND_CLOTH_FOOTSTEP: StepSound =
    StepSound::new("cloth", 1.0, 1.0, StepSoundClass::Plain);
pub const SOUND_SAND_FOOTSTEP: StepSound = StepSound::new("sand", 1.0, 1.0, StepSoundClass::Sand);

/// `EnumMobType` (EnumMobType.java): which entities trip a pressure plate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnumMobType {
    /// `everything`.
    Everything,
    /// `mobs`.
    Mobs,
    /// `players`.
    Players,
}

impl EnumMobType {
    /// The Java constant's name.
    // src: EnumMobType.java:3-6
    pub const fn java_name(self) -> &'static str {
        match self {
            EnumMobType::Everything => "everything",
            EnumMobType::Mobs => "mobs",
            EnumMobType::Players => "players",
        }
    }
}

/// One variant per `Block` subclass instantiated by Block.java's static
/// initialiser (Block.java:593-688), named after the class without its
/// `Block` prefix. Fields are the constructor arguments that vary between
/// instances of the same class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockKind {
    /// `Block` itself (cobblestone, planks, bedrock, lapis block, brick,
    /// mossy cobblestone).
    Plain,
    Stone,
    Grass,
    Dirt,
    Sapling,
    /// `BlockFlowing` (moving water and lava).
    Flowing,
    /// `BlockStationary` (still water and lava).
    Stationary,
    Sand,
    Gravel,
    Ore,
    Log,
    Leaves,
    Sponge,
    Glass {
        /// `BlockBreakable.localFlag` (BlockBreakable.java:4).
        local_flag: bool,
    },
    Dispenser,
    SandStone,
    /// `BlockNote` (note block).
    Note,
    Bed,
    Rail {
        /// `BlockRail.isPowered` (BlockRail.java:19): true for the powered rail.
        is_powered: bool,
    },
    DetectorRail,
    PistonBase {
        /// `BlockPistonBase.isSticky` (BlockPistonBase.java:11).
        is_sticky: bool,
    },
    Web,
    TallGrass,
    DeadBush,
    PistonExtension,
    /// `BlockCloth` (wool).
    Cloth,
    PistonMoving,
    /// `BlockFlower` (dandelion, rose).
    Flower,
    Mushroom,
    /// `BlockOreStorage` (gold, iron, diamond blocks).
    OreStorage,
    /// `BlockStep` (slabs).
    Step {
        /// `BlockStep.blockType` (BlockStep.java:11): true for the double slab.
        block_type: bool,
    },
    Tnt,
    Bookshelf,
    Obsidian,
    Torch,
    Fire,
    MobSpawner,
    Stairs {
        /// `BlockStairs.modelBlock` (BlockStairs.java:11), as its block id.
        model_block: i32,
    },
    Chest,
    RedstoneWire,
    Workbench,
    Crops,
    /// `BlockFarmland` (tilled field).
    Farmland,
    Furnace {
        /// `BlockFurnace.isActive` (BlockFurnace.java:12).
        is_active: bool,
    },
    Sign {
        /// `BlockSign.isFreestanding` (BlockSign.java:11). The tile entity
        /// class (`signEntityClass`) is always `TileEntitySign`.
        is_freestanding: bool,
    },
    Door,
    Ladder,
    Lever,
    PressurePlate {
        /// `BlockPressurePlate.triggerMobType` (BlockPressurePlate.java:11).
        trigger_mob_type: EnumMobType,
    },
    RedstoneOre {
        /// `BlockRedstoneOre.field_468_a` (BlockRedstoneOre.java:14): true for
        /// the glowing ore (id 74). Name kept; see `docs/mappings.md`.
        field_468_a: bool,
    },
    RedstoneTorch {
        /// `BlockRedstoneTorch.torchActive` (BlockRedstoneTorch.java:37).
        torch_active: bool,
    },
    Button,
    /// `BlockSnow` (the snow layer, id 78).
    Snow,
    Ice,
    /// `BlockSnowBlock` (id 80).
    SnowBlock,
    Cactus,
    Clay,
    /// `BlockReed` (sugar cane).
    Reed,
    JukeBox,
    Fence,
    Pumpkin {
        /// `BlockPumpkin.blockType` (BlockPumpkin.java:10): true for the jack-o'-lantern.
        block_type: bool,
    },
    Netherrack,
    SoulSand,
    GlowStone,
    Portal,
    Cake,
    RedstoneRepeater {
        /// `BlockRedstoneRepeater.isRepeaterPowered` (BlockRedstoneRepeater.java:12).
        is_repeater_powered: bool,
    },
    LockedChest,
    TrapDoor,
}

impl BlockKind {
    /// `BlockBreakable.localFlag` (BlockBreakable.java:4) for the three
    /// `BlockBreakable` subclasses; `BlockIce` and `BlockPortal` pass `false`
    /// (BlockIce.java:7, BlockPortal.java:7). `None` for other classes.
    pub const fn breakable_local_flag(self) -> Option<bool> {
        match self {
            BlockKind::Glass { local_flag } => Some(local_flag),
            BlockKind::Ice | BlockKind::Portal => Some(false),
            _ => None,
        }
    }

    /// `BlockRail.isPowered` (BlockRail.java:6); the detector rail passes
    /// `true` (BlockDetectorRail.java:8). `None` for other classes.
    pub const fn rail_is_powered(self) -> Option<bool> {
        match self {
            BlockKind::Rail { is_powered } => Some(is_powered),
            BlockKind::DetectorRail => Some(true),
            _ => None,
        }
    }

    /// The Java class name, for diagnostics (`World::unimplemented_hits`) and
    /// traces.
    pub const fn java_class(self) -> &'static str {
        match self {
            BlockKind::Plain => "Block",
            BlockKind::Stone => "BlockStone",
            BlockKind::Grass => "BlockGrass",
            BlockKind::Dirt => "BlockDirt",
            BlockKind::Sapling => "BlockSapling",
            BlockKind::Flowing => "BlockFlowing",
            BlockKind::Stationary => "BlockStationary",
            BlockKind::Sand => "BlockSand",
            BlockKind::Gravel => "BlockGravel",
            BlockKind::Ore => "BlockOre",
            BlockKind::Log => "BlockLog",
            BlockKind::Leaves => "BlockLeaves",
            BlockKind::Sponge => "BlockSponge",
            BlockKind::Glass { .. } => "BlockGlass",
            BlockKind::Dispenser => "BlockDispenser",
            BlockKind::SandStone => "BlockSandStone",
            BlockKind::Note => "BlockNote",
            BlockKind::Bed => "BlockBed",
            BlockKind::Rail { .. } => "BlockRail",
            BlockKind::DetectorRail => "BlockDetectorRail",
            BlockKind::PistonBase { .. } => "BlockPistonBase",
            BlockKind::Web => "BlockWeb",
            BlockKind::TallGrass => "BlockTallGrass",
            BlockKind::DeadBush => "BlockDeadBush",
            BlockKind::PistonExtension => "BlockPistonExtension",
            BlockKind::Cloth => "BlockCloth",
            BlockKind::PistonMoving => "BlockPistonMoving",
            BlockKind::Flower => "BlockFlower",
            BlockKind::Mushroom => "BlockMushroom",
            BlockKind::OreStorage => "BlockOreStorage",
            BlockKind::Step { .. } => "BlockStep",
            BlockKind::Tnt => "BlockTNT",
            BlockKind::Bookshelf => "BlockBookshelf",
            BlockKind::Obsidian => "BlockObsidian",
            BlockKind::Torch => "BlockTorch",
            BlockKind::Fire => "BlockFire",
            BlockKind::MobSpawner => "BlockMobSpawner",
            BlockKind::Stairs { .. } => "BlockStairs",
            BlockKind::Chest => "BlockChest",
            BlockKind::RedstoneWire => "BlockRedstoneWire",
            BlockKind::Workbench => "BlockWorkbench",
            BlockKind::Crops => "BlockCrops",
            BlockKind::Farmland => "BlockFarmland",
            BlockKind::Furnace { .. } => "BlockFurnace",
            BlockKind::Sign { .. } => "BlockSign",
            BlockKind::Door => "BlockDoor",
            BlockKind::Ladder => "BlockLadder",
            BlockKind::Lever => "BlockLever",
            BlockKind::PressurePlate { .. } => "BlockPressurePlate",
            BlockKind::RedstoneOre { .. } => "BlockRedstoneOre",
            BlockKind::RedstoneTorch { .. } => "BlockRedstoneTorch",
            BlockKind::Button => "BlockButton",
            BlockKind::Snow => "BlockSnow",
            BlockKind::Ice => "BlockIce",
            BlockKind::SnowBlock => "BlockSnowBlock",
            BlockKind::Cactus => "BlockCactus",
            BlockKind::Clay => "BlockClay",
            BlockKind::Reed => "BlockReed",
            BlockKind::JukeBox => "BlockJukeBox",
            BlockKind::Fence => "BlockFence",
            BlockKind::Pumpkin { .. } => "BlockPumpkin",
            BlockKind::Netherrack => "BlockNetherrack",
            BlockKind::SoulSand => "BlockSoulSand",
            BlockKind::GlowStone => "BlockGlowStone",
            BlockKind::Portal => "BlockPortal",
            BlockKind::Cake => "BlockCake",
            BlockKind::RedstoneRepeater { .. } => "BlockRedstoneRepeater",
            BlockKind::LockedChest => "BlockLockedChest",
            BlockKind::TrapDoor => "BlockTrapDoor",
        }
    }
}

/// `minX` … `maxZ` of a `Block` (Block.java:126-131). Java stores doubles set
/// from floats (`setBlockBounds(float…)`, Block.java:222-229).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlockBounds {
    pub min_x: f64,
    pub min_y: f64,
    pub min_z: f64,
    pub max_x: f64,
    pub max_y: f64,
    pub max_z: f64,
}

impl BlockBounds {
    /// `setBlockBounds(float, …)`: each float widened to double.
    // src: Block.java:222-229
    pub const fn from_floats(x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32) -> Self {
        BlockBounds {
            min_x: x0 as f64,
            min_y: y0 as f64,
            min_z: z0 as f64,
            max_x: x1 as f64,
            max_y: y1 as f64,
            max_z: z1 as f64,
        }
    }

    /// The unit cube every `Block` constructor starts with (Block.java:150).
    pub const FULL: BlockBounds = BlockBounds::from_floats(0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
}

/// A `Block` instance's data after Block.java's static initialiser, plus its
/// id's entries in the per-id static arrays (module docs).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlockDef {
    /// `blockID`.
    pub block_id: i32,
    /// The instantiated subclass.
    pub kind: BlockKind,
    /// `blockMaterial`.
    pub block_material: Material,
    /// `blockIndexInTexture`.
    pub block_index_in_texture: i32,
    /// `blockHardness` (-1 = unbreakable).
    pub block_hardness: f32,
    /// `blockResistance` (already ×3 as `setResistance` stores it).
    pub block_resistance: f32,
    /// `blockConstructorCalled`: set true by the constructor (Block.java:139)
    /// and never read in this tree.
    pub block_constructor_called: bool,
    /// `enableStats`.
    pub enable_stats: bool,
    /// The bounds after construction. Vanilla mutates bounds at run time
    /// (`setBlockBoundsBasedOnState`); the live copy is
    /// [`BlockStatics::bounds`].
    pub bounds: BlockBounds,
    /// `stepSound`.
    pub step_sound: StepSound,
    /// `blockParticleGravity` (1.0 for every vanilla block).
    pub block_particle_gravity: f32,
    /// `slipperiness` (0.6; ice 0.98).
    pub slipperiness: f32,
    /// The `setBlockName` argument; Java's `blockName` is `"tile." + this`
    /// (Block.java:563-566). `None` where vanilla never names the block
    /// (piston extension and moving piston: `blockName` stays null).
    pub block_name: Option<&'static str>,
    /// `tickOnLoad[id]`: receives random ticks.
    pub tick_on_load: bool,
    /// `opaqueCubeLookup[id]`.
    pub opaque_cube_lookup: bool,
    /// `isBlockContainer[id]`.
    pub is_block_container: bool,
    /// `lightOpacity[id]`.
    pub light_opacity: i32,
    /// `canBlockGrass[id]`.
    pub can_block_grass: bool,
    /// `lightValue[id]`.
    pub light_value: i32,
    /// `setNeighborNotifyOnMetadataChange[id]`.
    pub set_neighbor_notify_on_metadata_change: bool,
}

impl BlockDef {
    /// `Block(int id, Material material)`.
    // src: Block.java:138-156
    pub(crate) const fn new(id: i32, kind: BlockKind, material: Material) -> Self {
        let opaque = is_opaque_cube_during_construction(kind);
        BlockDef {
            block_id: id,
            kind,
            block_material: material,
            block_index_in_texture: 0,
            block_hardness: 0.0,
            block_resistance: 0.0,
            block_constructor_called: true,
            enable_stats: true,
            bounds: BlockBounds::FULL,
            step_sound: SOUND_POWDER_FOOTSTEP,
            block_particle_gravity: 1.0,
            slipperiness: 0.6,
            block_name: None,
            tick_on_load: false,
            opaque_cube_lookup: opaque,
            is_block_container: false,
            light_opacity: if opaque { 255 } else { 0 },
            can_block_grass: !material.get_can_block_grass(),
            light_value: 0,
            set_neighbor_notify_on_metadata_change: false,
        }
    }

    /// `Block(int id, int texture, Material material)`.
    // src: Block.java:166-169
    pub(crate) const fn new_tex(id: i32, tex: i32, kind: BlockKind, material: Material) -> Self {
        Self::new(id, kind, material).texture(tex)
    }

    /// A subclass constructor's `this.blockIndexInTexture = tex`.
    pub(crate) const fn texture(mut self, tex: i32) -> Self {
        self.block_index_in_texture = tex;
        self
    }

    /// `setStepSound`.
    // src: Block.java:171-174
    pub(crate) const fn set_step_sound(mut self, s: StepSound) -> Self {
        self.step_sound = s;
        self
    }

    /// `setLightOpacity`.
    // src: Block.java:176-179
    pub(crate) const fn set_light_opacity(mut self, v: i32) -> Self {
        self.light_opacity = v;
        self
    }

    /// `setLightValue`: `(int)(15.0F * v)`.
    // src: Block.java:181-184
    pub(crate) const fn set_light_value(mut self, v: f32) -> Self {
        self.light_value = (15.0f32 * v) as i32;
        self
    }

    /// `setResistance`: stores `v * 3.0F`.
    // src: Block.java:186-189
    pub(crate) const fn set_resistance(mut self, v: f32) -> Self {
        self.block_resistance = v * 3.0;
        self
    }

    /// `setHardness`: also raises the resistance to at least `v * 5.0F`.
    // src: Block.java:199-206
    pub(crate) const fn set_hardness(mut self, v: f32) -> Self {
        self.block_hardness = v;
        if self.block_resistance < v * 5.0 {
            self.block_resistance = v * 5.0;
        }
        self
    }

    /// `setBlockUnbreakable`: `setHardness(-1.0F)`.
    // src: Block.java:208-211
    pub(crate) const fn set_block_unbreakable(self) -> Self {
        self.set_hardness(-1.0)
    }

    /// `setTickOnLoad`.
    // src: Block.java:217-220
    pub(crate) const fn set_tick_on_load(mut self, v: bool) -> Self {
        self.tick_on_load = v;
        self
    }

    /// `setBlockBounds` during construction.
    // src: Block.java:222-229
    pub(crate) const fn set_block_bounds(
        mut self,
        x0: f32,
        y0: f32,
        z0: f32,
        x1: f32,
        y1: f32,
        z1: f32,
    ) -> Self {
        self.bounds = BlockBounds::from_floats(x0, y0, z0, x1, y1, z1);
        self
    }

    /// `setBlockName`.
    // src: Block.java:563-566
    pub(crate) const fn set_block_name(mut self, name: &'static str) -> Self {
        self.block_name = Some(name);
        self
    }

    /// `disableStats`.
    // src: Block.java:583-586
    pub(crate) const fn disable_stats(mut self) -> Self {
        self.enable_stats = false;
        self
    }

    /// `disableNeighborNotifyOnMetadataChange`.
    // src: Block.java:158-161
    pub(crate) const fn disable_neighbor_notify_on_metadata_change(mut self) -> Self {
        self.set_neighbor_notify_on_metadata_change = true;
        self
    }

    /// `BlockContainer`'s constructors: `isBlockContainer[id] = true`.
    // src: BlockContainer.java:4-12
    pub(crate) const fn block_container(mut self) -> Self {
        self.is_block_container = true;
        self
    }

    /// `BlockIce`'s `this.slipperiness = 0.98F`.
    pub(crate) const fn slipperiness(mut self, v: f32) -> Self {
        self.slipperiness = v;
        self
    }

    /// `getBlockName()`: `"tile." + name`, or `None` for vanilla's null.
    // src: Block.java:572-574
    pub fn get_block_name(&self) -> Option<String> {
        self.block_name.map(|n| format!("tile.{n}"))
    }

    /// `getHardness()`.
    // src: Block.java:213-215
    pub const fn get_hardness(&self) -> f32 {
        self.block_hardness
    }

    /// `getExplosionResistance(entity)`: `blockResistance / 5.0F`.
    // src: Block.java:371-373
    pub const fn get_explosion_resistance(&self) -> f32 {
        self.block_resistance / 5.0
    }

    /// `getMobilityFlag()`: the material's mobility unless the class
    /// overrides it.
    // src: Block.java:588-590 and the overrides listed per arm
    pub const fn get_mobility_flag(&self) -> i32 {
        match self.kind {
            // src: BlockBed.java:201-203, BlockDoor.java:200-202,
            // BlockPressurePlate.java:156-158
            BlockKind::Bed | BlockKind::Door | BlockKind::PressurePlate { .. } => 1,
            // src: BlockIce.java:41-43, BlockRail.java:247-249 (+ detector rail)
            BlockKind::Ice | BlockKind::Rail { .. } | BlockKind::DetectorRail => 0,
            _ => self.block_material.get_material_mobility(),
        }
    }

    /// `BlockLeaves.baseIndexInPNG` (BlockLeaves.java:6): the leaves'
    /// constructor stores its texture argument there as well as in
    /// `blockIndexInTexture` (BlockLeaves.java:10-11), so it is the texture
    /// index the row was built with. Never written afterwards, unlike the
    /// singleton's `blockIndexInTexture`, which `setGraphicsLevel` rewrites
    /// (see [`BlockStatics::leaves_block_index_in_texture`]); the row's
    /// `block_index_in_texture` is the constructor value, so this reads it.
    /// `None` for other classes.
    pub const fn base_index_in_png(&self) -> Option<i32> {
        match self.kind {
            BlockKind::Leaves => Some(self.block_index_in_texture),
            _ => None,
        }
    }
}

/// `Block.blocksList` (Block.java:16): the row for each id, `None` where
/// vanilla has no block (0 and 97–255).
pub static BLOCKS: [Option<BlockDef>; 256] = table::build();

/// `Block.blocksList[id]`; `None` for empty ids and ids outside 0..=255
/// (where Java would throw).
pub fn block(id: i32) -> Option<&'static BlockDef> {
    if (0..256).contains(&id) {
        BLOCKS[id as usize].as_ref()
    } else {
        None
    }
}

/// `Block.blocksList[id].blockMaterial`, with `Material.air` for id 0 as
/// `World.getBlockMaterial` does (World.java:400-403).
pub fn material_of(id: i32) -> Material {
    match block(id) {
        Some(b) => b.block_material,
        None => Material::Air,
    }
}

/// `Block.tickOnLoad[id]`.
pub fn tick_on_load(id: i32) -> bool {
    block(id).is_some_and(|b| b.tick_on_load)
}

/// `Block.opaqueCubeLookup[id]`.
pub fn opaque_cube_lookup(id: i32) -> bool {
    block(id).is_some_and(|b| b.opaque_cube_lookup)
}

/// `Block.isBlockContainer[id]`.
pub fn is_block_container(id: i32) -> bool {
    block(id).is_some_and(|b| b.is_block_container)
}

/// `Block.lightOpacity[id]`.
pub fn light_opacity(id: i32) -> i32 {
    block(id).map_or(0, |b| b.light_opacity)
}

/// `Block.canBlockGrass[id]`, including `canBlockGrass[0] = true`.
// src: Block.java:704
pub fn can_block_grass(id: i32) -> bool {
    if id == 0 {
        return true;
    }
    block(id).is_some_and(|b| b.can_block_grass)
}

/// `Block.lightValue[id]`.
pub fn light_value(id: i32) -> i32 {
    block(id).map_or(0, |b| b.light_value)
}

/// `Block.setNeighborNotifyOnMetadataChange[id]`.
pub fn set_neighbor_notify_on_metadata_change(id: i32) -> bool {
    block(id).is_some_and(|b| b.set_neighbor_notify_on_metadata_change)
}

/// `isOpaqueCube()` for a kind, fully transcribed (every override is a
/// constant or a field read). `graphics_level` is
/// `BlockLeavesBase.graphicsLevel` of the leaves singleton
/// ([`BlockStatics::graphics_level`]).
// src: Block.java:285-287 and the overrides listed per arm
pub const fn is_opaque_cube(kind: BlockKind, graphics_level: bool) -> bool {
    match kind {
        // src: BlockLeaves.java:177-179
        BlockKind::Leaves => !graphics_level,
        // src: BlockStep.java:39-41
        BlockKind::Step { block_type } => block_type,
        // src: BlockBreakable.java:11-13 (glass, ice, portal), BlockPortal.java:29-31
        BlockKind::Glass { .. } | BlockKind::Ice | BlockKind::Portal => false,
        // src: BlockBed.java:113, BlockButton.java:19, BlockCactus.java:52, BlockCake.java:63,
        // BlockDoor.java:38, BlockFarmland.java:18, BlockFence.java:20, BlockFire.java:35,
        // BlockFlower.java:47 (sapling, tall grass, dead bush, flower, mushroom, crops),
        // BlockFluid.java:52, BlockLadder.java:54, BlockLever.java:12, BlockMobSpawner.java:22,
        // BlockPistonBase.java:35, BlockPistonExtension.java:58, BlockPistonMoving.java:40,
        // BlockPressurePlate.java:25, BlockRail.java:31 (+ detector), BlockRedstoneRepeater.java:150,
        // BlockRedstoneWire.java:25, BlockReed.java:74, BlockSign.java:64, BlockSnow.java:17,
        // BlockStairs.java:26, BlockTorch.java:15 (+ redstone torch), BlockTrapDoor.java:16,
        // BlockWeb.java:14
        BlockKind::Bed
        | BlockKind::Button
        | BlockKind::Cactus
        | BlockKind::Cake
        | BlockKind::Door
        | BlockKind::Farmland
        | BlockKind::Fence
        | BlockKind::Fire
        | BlockKind::Sapling
        | BlockKind::TallGrass
        | BlockKind::DeadBush
        | BlockKind::Flower
        | BlockKind::Mushroom
        | BlockKind::Crops
        | BlockKind::Flowing
        | BlockKind::Stationary
        | BlockKind::Ladder
        | BlockKind::Lever
        | BlockKind::MobSpawner
        | BlockKind::PistonBase { .. }
        | BlockKind::PistonExtension
        | BlockKind::PistonMoving
        | BlockKind::PressurePlate { .. }
        | BlockKind::Rail { .. }
        | BlockKind::DetectorRail
        | BlockKind::RedstoneRepeater { .. }
        | BlockKind::RedstoneWire
        | BlockKind::Reed
        | BlockKind::Sign { .. }
        | BlockKind::Snow
        | BlockKind::Stairs { .. }
        | BlockKind::Torch
        | BlockKind::RedstoneTorch { .. }
        | BlockKind::TrapDoor
        | BlockKind::Web => false,
        _ => true,
    }
}

/// `isOpaqueCube()` as the `Block` constructor sees it (module docs): the
/// subclass fields are still at their defaults, so `BlockStep.blockType` is
/// false and `BlockLeavesBase.graphicsLevel` is false.
// src: Block.java:151-152
const fn is_opaque_cube_during_construction(kind: BlockKind) -> bool {
    match kind {
        BlockKind::Step { .. } => is_opaque_cube(BlockKind::Step { block_type: false }, false),
        _ => is_opaque_cube(kind, false),
    }
}

/// `renderAsNormalBlock()` for a kind, fully transcribed.
// src: Block.java:191-193 and the overrides listed per arm
pub const fn render_as_normal_block(kind: BlockKind) -> bool {
    match kind {
        // src: BlockStep.java:72-74
        BlockKind::Step { block_type } => block_type,
        // src: BlockBed.java:109, BlockButton.java:23, BlockCactus.java:48, BlockCake.java:59,
        // BlockDoor.java:42, BlockFarmland.java:22, BlockFence.java:24, BlockFire.java:39,
        // BlockFlower.java:51, BlockFluid.java:48, BlockLadder.java:58, BlockLever.java:16,
        // BlockPistonBase.java:212, BlockPistonExtension.java:62, BlockPistonMoving.java:44,
        // BlockPortal.java:33, BlockPressurePlate.java:29, BlockRail.java:62,
        // BlockRedstoneRepeater.java:16, BlockRedstoneWire.java:29, BlockReed.java:78,
        // BlockSign.java:60, BlockSnow.java:21, BlockStairs.java:30, BlockTorch.java:19,
        // BlockTrapDoor.java:20, BlockWeb.java:26
        BlockKind::Bed
        | BlockKind::Button
        | BlockKind::Cactus
        | BlockKind::Cake
        | BlockKind::Door
        | BlockKind::Farmland
        | BlockKind::Fence
        | BlockKind::Fire
        | BlockKind::Sapling
        | BlockKind::TallGrass
        | BlockKind::DeadBush
        | BlockKind::Flower
        | BlockKind::Mushroom
        | BlockKind::Crops
        | BlockKind::Flowing
        | BlockKind::Stationary
        | BlockKind::Ladder
        | BlockKind::Lever
        | BlockKind::PistonBase { .. }
        | BlockKind::PistonExtension
        | BlockKind::PistonMoving
        | BlockKind::Portal
        | BlockKind::PressurePlate { .. }
        | BlockKind::Rail { .. }
        | BlockKind::DetectorRail
        | BlockKind::RedstoneRepeater { .. }
        | BlockKind::RedstoneWire
        | BlockKind::Reed
        | BlockKind::Sign { .. }
        | BlockKind::Snow
        | BlockKind::Stairs { .. }
        | BlockKind::Torch
        | BlockKind::RedstoneTorch { .. }
        | BlockKind::TrapDoor
        | BlockKind::Web => false,
        _ => true,
    }
}

/// `tickRate()` for a block, fully transcribed.
// src: Block.java:309-311 and the overrides listed per arm
pub const fn tick_rate(def: &BlockDef) -> i32 {
    match def.kind {
        // src: BlockButton.java:15-17, BlockDetectorRail.java:12-14, BlockPressurePlate.java:17-19
        BlockKind::Button | BlockKind::DetectorRail | BlockKind::PressurePlate { .. } => 20,
        // src: BlockDispenser.java:13-15
        BlockKind::Dispenser => 4,
        // src: BlockFire.java:51-53
        BlockKind::Fire => 40,
        // src: BlockFluid.java:187-193 (water 5, lava 30, else 0)
        BlockKind::Flowing | BlockKind::Stationary => match def.block_material {
            Material::Water => 5,
            Material::Lava => 30,
            _ => 0,
        },
        // src: BlockRedstoneOre.java:17-19
        BlockKind::RedstoneOre { .. } => 30,
        // src: BlockRedstoneTorch.java:41-43
        BlockKind::RedstoneTorch { .. } => 2,
        // src: BlockSand.java:45-47 (sand and gravel)
        BlockKind::Sand | BlockKind::Gravel => 3,
        // src: BlockStairs.java:113-115: modelBlock.tickRate(); the two model
        // blocks (planks, cobblestone) are plain `Block`s.
        BlockKind::Stairs { .. } => 10,
        _ => 10,
    }
}

/// `getRenderType()` for a kind, fully transcribed (every override is a
/// constant). `-1` means "not drawn by `RenderBlocks`".
// src: Block.java:195-197 and the overrides listed per arm
pub const fn get_render_type(kind: BlockKind) -> i32 {
    match kind {
        // src: BlockFlower.java:55-57 (sapling, tall grass, dead bush, flower,
        // mushroom inherit it), BlockReed.java:82-84, BlockWeb.java:22-24
        BlockKind::Sapling
        | BlockKind::TallGrass
        | BlockKind::DeadBush
        | BlockKind::Flower
        | BlockKind::Mushroom
        | BlockKind::Reed
        | BlockKind::Web => 1,
        // src: BlockTorch.java:23-25 (+ redstone torch)
        BlockKind::Torch | BlockKind::RedstoneTorch { .. } => 2,
        // src: BlockFire.java:43-45
        BlockKind::Fire => 3,
        // src: BlockFluid.java:86-88
        BlockKind::Flowing | BlockKind::Stationary => 4,
        // src: BlockRedstoneWire.java:33-35
        BlockKind::RedstoneWire => 5,
        // src: BlockCrops.java:85-87
        BlockKind::Crops => 6,
        // src: BlockDoor.java:46-48
        BlockKind::Door => 7,
        // src: BlockLadder.java:62-64
        BlockKind::Ladder => 8,
        // src: BlockRail.java:66-68 (+ detector rail)
        BlockKind::Rail { .. } | BlockKind::DetectorRail => 9,
        // src: BlockStairs.java:34-36
        BlockKind::Stairs { .. } => 10,
        // src: BlockFence.java:28-30
        BlockKind::Fence => 11,
        // src: BlockLever.java:20-22
        BlockKind::Lever => 12,
        // src: BlockCactus.java:56-58
        BlockKind::Cactus => 13,
        // src: BlockBed.java:105-107
        BlockKind::Bed => 14,
        // src: BlockRedstoneRepeater.java:57-59
        BlockKind::RedstoneRepeater { .. } => 15,
        // src: BlockPistonBase.java:31-33
        BlockKind::PistonBase { .. } => 16,
        // src: BlockPistonExtension.java:54-56
        BlockKind::PistonExtension => 17,
        // src: BlockPistonMoving.java:36-38, BlockSign.java:56-58
        BlockKind::PistonMoving | BlockKind::Sign { .. } => -1,
        // src: BlockTrapDoor.java:24-26 returns 0 explicitly, as Block does
        _ => 0,
    }
}

/// The row of a vanilla block id that another block's method refers to:
/// `BlockStairs.modelBlock` (BlockStairs.java:7), `Block.dirt` (grass and
/// farmland `idDropped`), `Block.planks` (bed and workbench textures),
/// `Block.redstoneWire` (the redstone torch's top).
fn block_row(id: i32) -> &'static BlockDef {
    block(id).expect("a vanilla block id")
}

/// `getRenderBlockPass()`: 1 for blocks drawn in the translucent pass.
// src: Block.java:493-495 and the overrides listed per arm
pub fn get_render_block_pass(def: &BlockDef) -> i32 {
    match def.kind {
        // src: BlockFluid.java:205-207
        BlockKind::Flowing | BlockKind::Stationary => {
            if matches!(def.block_material, Material::Water) {
                1
            } else {
                0
            }
        }
        // src: BlockIce.java:12-14, BlockPortal.java:146-148
        BlockKind::Ice | BlockKind::Portal => 1,
        // src: BlockStairs.java:89-91
        BlockKind::Stairs { model_block } => get_render_block_pass(block_row(model_block)),
        // src: BlockGlass.java:14-16 returns 0 explicitly
        _ => 0,
    }
}

/// `isCollidable()`: whether the ray trace can select the block.
// src: Block.java:293-295 and the overrides listed per arm
pub fn is_collidable(def: &BlockDef) -> bool {
    match def.kind {
        // src: BlockFire.java:168-170
        BlockKind::Fire => false,
        // src: BlockStairs.java:125-127
        BlockKind::Stairs { model_block } => is_collidable(block_row(model_block)),
        _ => true,
    }
}

/// `damageDropped(meta)`: the damage value of the dropped item stack.
// src: Block.java:367-369 and the overrides listed per arm
pub const fn damage_dropped(def: &BlockDef, meta: i32) -> i32 {
    match def.kind {
        // src: BlockCloth.java:17-19, BlockLog.java:56-58, BlockStep.java:68-70
        BlockKind::Cloth | BlockKind::Log | BlockKind::Step { .. } => meta,
        // src: BlockLeaves.java:173-175, BlockSapling.java:57-59
        BlockKind::Leaves | BlockKind::Sapling => meta & 3,
        // src: BlockOre.java:24-26 (`blockID == Block.oreLapis.blockID ? 4 : 0`)
        BlockKind::Ore if def.block_id == ids::ORE_LAPIS => 4,
        _ => 0,
    }
}

/// Block ids (`Block.<field>.blockID`): the single public id surface, used by
/// the data methods here and by the generators ([`crate::worldgen::ids`]).
pub mod ids {
    // src: Block.java:593-688 (the id argument of each constructor)
    pub const COBBLESTONE: i32 = 4;
    pub const STONE: i32 = 1; // Block.java:593
    pub const GRASS: i32 = 2; // Block.java:594
    pub const DIRT: i32 = 3;
    pub const BEDROCK: i32 = 7; // Block.java:599
    pub const WATER_MOVING: i32 = 8; // Block.java:600
    pub const WATER_STILL: i32 = 9; // Block.java:601
    pub const LAVA_MOVING: i32 = 10; // Block.java:602
    pub const LAVA_STILL: i32 = 11; // Block.java:603
    pub const SAND: i32 = 12; // Block.java:604
    pub const GRAVEL: i32 = 13; // Block.java:605
    pub const PLANKS: i32 = 5;
    pub const LEAVES: i32 = 18;
    pub const SAPLING: i32 = 6;
    pub const ORE_COAL: i32 = 16;
    pub const WOOD: i32 = 17;
    pub const ORE_LAPIS: i32 = 21;
    pub const DISPENSER: i32 = 23;
    pub const SAND_STONE: i32 = 24; // Block.java:616
    pub const RAIL_POWERED: i32 = 27;
    pub const STAIR_SINGLE: i32 = 44;
    pub const OBSIDIAN: i32 = 49;
    pub const ICE: i32 = 79; // Block.java:671
    pub const NETHERRACK: i32 = 87; // Block.java:679
    pub const SLOW_SAND: i32 = 88; // soul sand, Block.java:680
    pub const REDSTONE_WIRE: i32 = 55;
    pub const ORE_DIAMOND: i32 = 56;
    pub const STONE_OVEN_IDLE: i32 = 61;
    pub const TORCH_REDSTONE_ACTIVE: i32 = 76;
}

use crate::item::ids as items;

/// `idDropped(meta, rand)`: the item or block id a broken block drops; 0 or
/// a negative id means nothing (`dropBlockAsItemWithChance` drops only ids
/// `> 0`, Block.java:345-346). `rand` is the caller's `Random` (in the world,
/// `World.rand`, Block.java:341-345); only gravel and tall grass draw from it.
// src: Block.java:323-325 and the overrides listed per arm
pub fn id_dropped(def: &BlockDef, meta: i32, rand: &mut JavaRandom) -> i32 {
    use BlockKind as K;
    match def.kind {
        // src: BlockBed.java:137-139; isBlockFootOfBed = (meta & 8) != 0, BlockBed.java:149-151
        K::Bed => {
            if (meta & 8) != 0 {
                0
            } else {
                items::BED
            }
        }
        // src: BlockCake.java:110-112, BlockFluid.java:90-92, BlockMobSpawner.java:14-16,
        // BlockPistonMoving.java:57-59
        K::Cake | K::Flowing | K::Stationary | K::MobSpawner | K::PistonMoving => 0,
        // src: BlockClay.java:10-12
        K::Clay => items::CLAY,
        // src: BlockCrops.java:107-109
        K::Crops => {
            if meta == 7 {
                items::WHEAT
            } else {
                -1
            }
        }
        // src: BlockDeadBush.java:20-22
        K::DeadBush => -1,
        // src: BlockDispenser.java:17-19
        K::Dispenser => ids::DISPENSER,
        // src: BlockDoor.java:171-177
        K::Door => {
            if (meta & 8) != 0 {
                0
            } else if matches!(def.block_material, Material::Iron) {
                items::DOOR_STEEL
            } else {
                items::DOOR_WOOD
            }
        }
        // src: BlockFarmland.java:94-96, BlockGrass.java:51-53: Block.dirt.idDropped(0, rand)
        K::Farmland | K::Grass => id_dropped(block_row(ids::DIRT), 0, rand),
        // src: BlockFurnace.java:16-18
        K::Furnace { .. } => ids::STONE_OVEN_IDLE,
        // src: BlockGlowStone.java:14-16
        K::GlowStone => items::LIGHT_STONE_DUST,
        // src: BlockGravel.java:10-12
        K::Gravel => {
            if rand.next_int_bounded(10) == 0 {
                items::FLINT
            } else {
                def.block_id
            }
        }
        // src: BlockLeaves.java:159-161
        K::Leaves => ids::SAPLING,
        // src: BlockLog.java:15-17
        K::Log => ids::WOOD,
        // src: BlockObsidian.java:14-16
        K::Obsidian => ids::OBSIDIAN,
        // src: BlockOre.java:10-18
        K::Ore => {
            if def.block_id == ids::ORE_COAL {
                items::COAL
            } else if def.block_id == ids::ORE_DIAMOND {
                items::DIAMOND
            } else if def.block_id == ids::ORE_LAPIS {
                items::DYE_POWDER
            } else {
                def.block_id
            }
        }
        // src: BlockRedstoneOre.java:51-53, BlockRedstoneWire.java:283-285
        K::RedstoneOre { .. } | K::RedstoneWire => items::REDSTONE,
        // src: BlockRedstoneRepeater.java:154-156
        K::RedstoneRepeater { .. } => items::REDSTONE_REPEATER,
        // src: BlockRedstoneTorch.java:143-145
        K::RedstoneTorch { .. } => ids::TORCH_REDSTONE_ACTIVE,
        // src: BlockReed.java:70-72
        K::Reed => items::REED,
        // src: BlockSign.java:76-78
        K::Sign { .. } => items::SIGN,
        // src: BlockSnow.java:63-65, BlockSnowBlock.java:11-13
        K::Snow | K::SnowBlock => items::SNOWBALL,
        // src: BlockStairs.java:93-95
        K::Stairs { model_block } => id_dropped(block_row(model_block), meta, rand),
        // src: BlockStep.java:60-62
        K::Step { .. } => ids::STAIR_SINGLE,
        // src: BlockStone.java:10-12
        K::Stone => ids::COBBLESTONE,
        // src: BlockTallGrass.java:39-41
        K::TallGrass => {
            if rand.next_int_bounded(8) == 0 {
                items::SEEDS
            } else {
                -1
            }
        }
        // src: BlockWeb.java:30-32
        K::Web => items::SILK,
        _ => def.block_id,
    }
}

/// `quantityDropped(rand)`: how many `idDropped` rolls a break makes.
// src: Block.java:319-321 and the overrides listed per arm
pub fn quantity_dropped(def: &BlockDef, rand: &mut JavaRandom) -> i32 {
    use BlockKind as K;
    match def.kind {
        // src: BlockBookshelf.java:14-16, BlockCake.java:106-108, BlockFire.java:47-49,
        // BlockFluid.java:94-96, BlockGlass.java:10-12, BlockIce.java:29-31,
        // BlockMobSpawner.java:18-20, BlockPistonExtension.java:74-76,
        // BlockPortal.java:142-144, BlockSnow.java:67-69, BlockTNT.java:35-37
        K::Bookshelf
        | K::Cake
        | K::Fire
        | K::Flowing
        | K::Stationary
        | K::Glass { .. }
        | K::Ice
        | K::MobSpawner
        | K::PistonExtension
        | K::Portal
        | K::Snow
        | K::Tnt => 0,
        // src: BlockClay.java:14-16, BlockSnowBlock.java:15-17
        K::Clay | K::SnowBlock => 4,
        // src: BlockGlowStone.java:10-12
        K::GlowStone => 2 + rand.next_int_bounded(3),
        // src: BlockLeaves.java:155-157
        K::Leaves => {
            if rand.next_int_bounded(20) == 0 {
                1
            } else {
                0
            }
        }
        // src: BlockOre.java:20-22
        K::Ore => {
            if def.block_id == ids::ORE_LAPIS {
                4 + rand.next_int_bounded(5)
            } else {
                1
            }
        }
        // src: BlockRedstoneOre.java:55-57
        K::RedstoneOre { .. } => 4 + rand.next_int_bounded(2),
        // src: BlockStairs.java:97-99
        K::Stairs { model_block } => quantity_dropped(block_row(model_block), rand),
        // src: BlockStep.java:64-66 (`blockType ? 2 : 1`)
        K::Step { block_type: true } => 2,
        // The single slab (BlockStep.java:65), BlockCrops.java:111-113,
        // BlockLadder.java:126-128, BlockLog.java:11-13, BlockObsidian.java:10-12
        // and BlockRail.java:70-72 return 1, as Block does.
        _ => 1,
    }
}

/// `ModelBed.bedDirection` (ModelBed.java:6): per bed direction, which model
/// face each block side shows.
const BED_DIRECTION: [[i32; 6]; 4] = [
    [1, 0, 3, 2, 5, 4],
    [1, 0, 5, 4, 2, 3],
    [1, 0, 2, 3, 4, 5],
    [1, 0, 4, 5, 3, 2],
];

/// `PistonBlockTextures.faceToSide` (PistonBlockTextures.java:4): the
/// opposite side.
pub const FACE_TO_SIDE: [i32; 6] = [1, 0, 3, 2, 5, 4];

/// `BlockStep.field_22037_a`: the slab type names by metadata, appended to
/// the slab item's name (`super.getItemName() + "." + name`, ItemSlab.java:18-20).
/// A Java static, so outside the `block_table` dump; pinned by the unit test.
// src: BlockStep.java:6
pub const STEP_TYPE_NAMES: [&str; 4] = ["stone", "sand", "wood", "cobble"];

/// `BlockRedstoneRepeater.field_22023_b`: the repeater delay setting by
/// `(meta & 12) >> 2`; the scheduled tick delay is twice the entry
/// (BlockRedstoneRepeater.java:36-37, :93-97). A Java static, so outside the
/// `block_table` dump.
// src: BlockRedstoneRepeater.java:7
pub const REPEATER_DELAYS: [i32; 4] = [1, 2, 3, 4];

/// `BlockRedstoneRepeater.repeaterTorchOffset`: the sliding torch's offset by
/// delay setting, read by the redstone particles (BlockRedstoneRepeater.java:184-193)
/// and the renderer (RenderBlocks.java:323-335). A Java static.
// src: BlockRedstoneRepeater.java:6
pub const REPEATER_TORCH_OFFSET: [f64; 4] = [-0.0625, 0.0625, 0.1875, 0.3125];

/// `BlockRedstoneRepeater.getBlockTextureFromSideAndMetadata` (meta unused).
// src: BlockRedstoneRepeater.java:43-51
const fn repeater_texture(is_repeater_powered: bool, side: i32) -> i32 {
    if side == 0 {
        if is_repeater_powered { 99 } else { 115 }
    } else if side == 1 {
        if is_repeater_powered { 147 } else { 131 }
    } else {
        5
    }
}

/// `BlockStep.getBlockTextureFromSideAndMetadata`: by slab type (meta).
// src: BlockStep.java:19-33
const fn step_texture(side: i32, meta: i32) -> i32 {
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

/// The live `blockIndexInTexture` of a block singleton: the row's value,
/// except for `Block.leaves`, whose `setGraphicsLevel` rewrites it at run time
/// (BlockLeaves.java:185-188; [`BlockStatics::leaves_block_index_in_texture`]).
/// No other block writes the field after its constructor.
pub fn block_index_in_texture(def: &BlockDef, statics: &BlockStatics) -> i32 {
    match def.kind {
        BlockKind::Leaves => statics.leaves_block_index_in_texture,
        _ => def.block_index_in_texture,
    }
}

/// `getBlockTextureFromSide(side)`: the terrain.png index of a side with no
/// metadata (item icons and the base of
/// [`get_block_texture_from_side_and_metadata`]). `statics` supplies the
/// leaves' live `blockIndexInTexture` ([`block_index_in_texture`]).
// src: Block.java:265-267 and the overrides listed per arm
pub fn get_block_texture_from_side(def: &BlockDef, side: i32, statics: &BlockStatics) -> i32 {
    let t = block_index_in_texture(def, statics);
    use BlockKind as K;
    match def.kind {
        // src: BlockBookshelf.java:10-12
        K::Bookshelf => {
            if side <= 1 {
                4
            } else {
                t
            }
        }
        // src: BlockCactus.java:40-46
        K::Cactus => {
            if side == 1 {
                t - 1
            } else if side == 0 {
                t + 1
            } else {
                t
            }
        }
        // src: BlockCake.java:51-57
        K::Cake => {
            if side == 1 {
                t
            } else if side == 0 {
                t + 3
            } else {
                t + 1
            }
        }
        // src: BlockChest.java:100-108, BlockLockedChest.java:42-50
        K::Chest | K::LockedChest => {
            if side == 1 || side == 0 {
                t - 1
            } else if side == 3 {
                t + 1
            } else {
                t
            }
        }
        // src: BlockDispenser.java:64-72
        K::Dispenser => {
            if side == 1 || side == 0 {
                t + 17
            } else if side == 3 {
                t + 1
            } else {
                t
            }
        }
        // src: BlockFluid.java:27-29
        K::Flowing | K::Stationary => {
            if side != 0 && side != 1 {
                t + 1
            } else {
                t
            }
        }
        // src: BlockFurnace.java:92-100
        K::Furnace { .. } => {
            if side == 1 || side == 0 {
                t + 17
            } else if side == 3 {
                t - 1
            } else {
                t
            }
        }
        // src: BlockJukeBox.java:8-10
        K::JukeBox => t + if side == 1 { 1 } else { 0 },
        // src: BlockPumpkin.java:36-44
        K::Pumpkin { .. } => {
            if side == 1 || side == 0 {
                t
            } else if side == 3 {
                t + 1 + 16
            } else {
                t + 16
            }
        }
        // src: BlockRedstoneRepeater.java:61-63
        K::RedstoneRepeater {
            is_repeater_powered,
        } => repeater_texture(is_repeater_powered, side),
        // src: BlockSandStone.java:8-14
        K::SandStone => {
            if side == 1 {
                t - 16
            } else if side == 0 {
                t + 16
            } else {
                t
            }
        }
        // src: BlockStairs.java:105-107
        K::Stairs { model_block } => {
            get_block_texture_from_side(block_row(model_block), side, statics)
        }
        // src: BlockStep.java:35-37
        K::Step { .. } => step_texture(side, 0),
        // src: BlockTNT.java:10-16
        K::Tnt => {
            if side == 0 {
                t + 2
            } else if side == 1 {
                t + 1
            } else {
                t
            }
        }
        // src: BlockWorkbench.java:9-17
        K::Workbench => {
            if side == 1 {
                t - 16
            } else if side == 0 {
                get_block_texture_from_side(block_row(ids::PLANKS), 0, statics)
            } else if side != 2 && side != 4 {
                t
            } else {
                t + 1
            }
        }
        // BlockNote.java:8-10 and BlockOreStorage.java:9-11 return
        // blockIndexInTexture, as Block does.
        _ => t,
    }
}

/// `getBlockTextureFromSideAndMetadata(side, meta)`: the terrain.png index of
/// a side (a negative index means "mirrored", BlockDoor). `statics` supplies
/// the singleton state two pistons read (bounds, `headTexture`) and the
/// leaves' live `blockIndexInTexture`.
///
/// `None` where vanilla throws `ArrayIndexOutOfBoundsException`: a side
/// outside 0..=5 for the bed, and the piston extension with `meta & 7` of 6
/// or 7 on any side but the front (vanilla bug: `faceToSide` has six
/// entries, BlockPistonExtension.java:50).
// src: Block.java:261-263 and the overrides listed per arm
pub fn get_block_texture_from_side_and_metadata(
    def: &BlockDef,
    side: i32,
    meta: i32,
    statics: &BlockStatics,
) -> Option<i32> {
    let t = block_index_in_texture(def, statics);
    use BlockKind as K;
    Some(match def.kind {
        // src: BlockBed.java:85-103; getDirectionFromMetadata = meta & 3 (:145-147),
        // isBlockFootOfBed = (meta & 8) != 0 (:149-151)
        K::Bed => {
            if side == 0 {
                block_row(ids::PLANKS).block_index_in_texture
            } else {
                let var3 = meta & 3;
                let var4 = *BED_DIRECTION[var3 as usize].get(usize::try_from(side).ok()?)?;
                if (meta & 8) != 0 {
                    if var4 == 2 {
                        t + 2 + 16
                    } else if var4 != 5 && var4 != 4 {
                        t + 1
                    } else {
                        t + 1 + 16
                    }
                } else if var4 == 3 {
                    t - 1 + 16
                } else if var4 != 5 && var4 != 4 {
                    t
                } else {
                    t + 16
                }
            }
        }
        // src: BlockCake.java:41-49
        K::Cake => {
            if side == 1 {
                t
            } else if side == 0 {
                t + 3
            } else if meta > 0 && side == 4 {
                t + 2
            } else {
                t + 1
            }
        }
        // src: BlockCloth.java:8-15
        K::Cloth => {
            if meta == 0 {
                t
            } else {
                let var2 = !(meta & 15);
                113 + ((var2 & 8) >> 3) + (var2 & 7) * 16
            }
        }
        // src: BlockCrops.java:77-83
        K::Crops => t.wrapping_add(if meta < 0 { 7 } else { meta }),
        // src: BlockDeadBush.java:16-18, BlockRedstoneWire.java:17-19
        K::DeadBush | K::RedstoneWire => t,
        // src: BlockDoor.java:18-36; getState, BlockDoor.java:184-186
        K::Door => {
            if side != 0 && side != 1 {
                let var3 = if (meta & 4) == 0 {
                    meta.wrapping_sub(1) & 3
                } else {
                    meta & 3
                };
                if (var3 == 0 || var3 == 2) ^ (side <= 3) {
                    t
                } else {
                    let mut var4 = var3 / 2 + ((side & 1) ^ var3);
                    var4 += (meta & 4) / 4;
                    let mut var5 = t - (meta & 8) * 2;
                    if (var4 & 1) != 0 {
                        var5 = -var5;
                    }
                    var5
                }
            } else {
                t
            }
        }
        // src: BlockFarmland.java:26-32
        K::Farmland => {
            if side == 1 && meta > 0 {
                t - 1
            } else if side == 1 {
                t
            } else {
                2
            }
        }
        // src: BlockLeaves.java:181-183
        K::Leaves => {
            if (meta & 3) == 1 {
                t + 80
            } else {
                t
            }
        }
        // src: BlockLog.java:44-54
        K::Log => {
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
        // src: BlockPistonBase.java:20-29; getOrientation = meta & 7 (:216-218),
        // isPowered = (meta & 8) != 0 (:220-222)
        K::PistonBase { .. } => {
            let var3 = meta & 7;
            if var3 > 5 {
                t
            } else if side == var3 {
                let b = &statics.bounds[(def.block_id & 255) as usize];
                if (meta & 8) == 0
                    && b.min_x <= 0.0
                    && b.min_y <= 0.0
                    && b.min_z <= 0.0
                    && b.max_x >= 1.0
                    && b.max_y >= 1.0
                    && b.max_z >= 1.0
                {
                    t
                } else {
                    110
                }
            } else if side == FACE_TO_SIDE[var3 as usize] {
                109
            } else {
                108
            }
        }
        // src: BlockPistonExtension.java:41-52; getDirectionMeta = meta & 7 (:156-158)
        K::PistonExtension => {
            let var3 = meta & 7;
            if side == var3 {
                if statics.piston_extension_head_texture >= 0 {
                    statics.piston_extension_head_texture
                } else if (meta & 8) != 0 {
                    t - 1
                } else {
                    t
                }
            } else if side == *FACE_TO_SIDE.get(var3 as usize)? {
                // vanilla bug: var3 of 6 or 7 indexes past faceToSide (None above)
                107
            } else {
                108
            }
        }
        // src: BlockPumpkin.java:13-34
        K::Pumpkin { block_type } => {
            if side == 1 || side == 0 {
                t
            } else {
                let mut var3 = t + 1 + 16;
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
                    t + 16
                }
            }
        }
        // src: BlockRail.java:50-60 (+ detector rail, whose isPowered is true but
        // whose id is not railPowered's)
        K::Rail { .. } | K::DetectorRail => {
            if def.kind.rail_is_powered() == Some(true) {
                if def.block_id == ids::RAIL_POWERED && (meta & 8) == 0 {
                    return Some(t - 16);
                }
            } else if meta >= 6 {
                return Some(t - 16);
            }
            t
        }
        // src: BlockRedstoneRepeater.java:43-51
        K::RedstoneRepeater {
            is_repeater_powered,
        } => repeater_texture(is_repeater_powered, side),
        // src: BlockRedstoneTorch.java:11-13; super is Block's (BlockTorch has no
        // override), which is getBlockTextureFromSide
        K::RedstoneTorch { .. } => {
            if side == 1 {
                return get_block_texture_from_side_and_metadata(
                    block_row(ids::REDSTONE_WIRE),
                    side,
                    meta,
                    statics,
                );
            }
            get_block_texture_from_side(def, side, statics)
        }
        // src: BlockSapling.java:27-34; super (BlockFlower has no override) is
        // Block's, getBlockTextureFromSide
        K::Sapling => {
            let var2 = meta & 3;
            if var2 == 1 {
                63
            } else if var2 == 2 {
                79
            } else {
                get_block_texture_from_side(def, side, statics)
            }
        }
        // src: BlockStairs.java:101-103
        K::Stairs { model_block } => {
            return get_block_texture_from_side_and_metadata(
                block_row(model_block),
                side,
                meta,
                statics,
            );
        }
        // src: BlockStep.java:19-33
        K::Step { .. } => step_texture(side, meta),
        // src: BlockTallGrass.java:12-20
        K::TallGrass => {
            if meta == 1 {
                t
            } else if meta == 2 {
                t + 16 + 1
            } else if meta == 0 {
                t + 16
            } else {
                t
            }
        }
        _ => get_block_texture_from_side(def, side, statics),
    })
}

/// `BlockFire.chanceToEncourageFire` (BlockFire.java:6), indexed by block id,
/// as `initializeBlock` leaves it (BlockFire.java:14-24, called from
/// Block.java:700). Never written afterwards; the fire task of WP-1.6 reads it.
pub static CHANCE_TO_ENCOURAGE_FIRE: [i32; 256] = table::FIRE_BURN_RATES.0;

/// `BlockFire.abilityToCatchFire` (BlockFire.java:7), indexed by block id; see
/// [`CHANCE_TO_ENCOURAGE_FIRE`].
pub static ABILITY_TO_CATCH_FIRE: [i32; 256] = table::FIRE_BURN_RATES.1;

/// The run-time-mutable state of the `Block` singletons (see the module docs).
///
/// Vanilla blocks are shared singletons whose fields some methods overwrite:
/// `setBlockBoundsBasedOnState` rewrites `minX`…`maxZ` before collision and
/// selection queries (doors, slabs, rails, …), and the renderer sets
/// `BlockLeavesBase.graphicsLevel`. Since the result of later queries depends
/// on the last write, this is world state.
///
/// Every mutable instance field of a block singleton is here with the value
/// the static initialiser leaves (pinned by `tests/golden/block/block-table`),
/// except the singleton `Random`s the oracle reseeds (`BlockChest.random`,
/// `BlockFurnace.furnaceRand`, `BlockDispenser.random`; census §2), which the
/// tasks that use them add. Still to add with its behaviour (Phase 1), as a
/// Java static: `BlockFurnace.keepFurnaceInventory`.
#[derive(Clone, Debug)]
pub struct BlockStatics {
    /// `minX`…`maxZ` of each block singleton, starting from
    /// [`BlockDef::bounds`]. Index = block id; unused ids hold
    /// [`BlockBounds::FULL`].
    pub bounds: Box<[BlockBounds; 256]>,
    /// `BlockLeavesBase.graphicsLevel` of `Block.leaves` (the only
    /// `BlockLeavesBase` instance). Set by the client's fancy-graphics option
    /// (RenderGlobal); false in the oracle, which has no renderer.
    /// Written together with [`Self::leaves_block_index_in_texture`] by
    /// [`Self::set_leaves_graphics_level`].
    pub graphics_level: bool,
    /// `blockIndexInTexture` of `Block.leaves`: the constructor's 52
    /// (Block.java:610, BlockLeaves.java:9-11) until `setGraphicsLevel`
    /// rewrites it to `baseIndexInPNG + (fancy ? 0 : 1)`. Client-only (the
    /// renderer calls it, RenderGlobal.java:187); the texture methods read it
    /// through [`block_index_in_texture`].
    // src: BlockLeaves.java:185-188
    pub leaves_block_index_in_texture: i32,
    /// `BlockSand.fallInstantly` (a static): sand and gravel move down at
    /// once instead of spawning an `EntityFallingSand`. The terrain
    /// generators set it during population (ChunkProviderGenerate.java:311,
    /// :602); false otherwise.
    // src: BlockSand.java:6
    pub fall_instantly: bool,
    /// `BlockFlowing`'s scratch fields of `Block.waterMoving` (id 8).
    pub water_moving: FlowingScratch,
    /// `BlockFlowing`'s scratch fields of `Block.lavaMoving` (id 10).
    pub lava_moving: FlowingScratch,
    /// `BlockLeaves.adjacentTreeBlocks` of `Block.leaves`: null until the
    /// first decay check allocates it (BlockLeaves.java:7, :65-66).
    pub leaves_adjacent_tree_blocks: Option<Box<[i32]>>,
    /// `BlockPistonBase.ignoreUpdates` of `Block.pistonStickyBase` (id 29).
    // src: BlockPistonBase.java:7
    pub piston_sticky_base_ignore_updates: bool,
    /// `BlockPistonBase.ignoreUpdates` of `Block.pistonBase` (id 33).
    // src: BlockPistonBase.java:7
    pub piston_base_ignore_updates: bool,
    /// `BlockPistonExtension.headTexture` of `Block.pistonExtension`: -1, or
    /// the texture the piston renderer forces on the head
    /// (`setHeadTexture`/`clearHeadTexture`, BlockPistonExtension.java:15-21).
    // src: BlockPistonExtension.java:7
    pub piston_extension_head_texture: i32,
    /// `BlockRedstoneWire.wiresProvidePower` of `Block.redstoneWire`. Per
    /// world here, starting `true`; the oracle replaces it per scenario
    /// (census §6).
    // src: BlockRedstoneWire.java:9
    pub redstone_wire_wires_provide_power: bool,
    /// `BlockRedstoneWire.blocksNeedingUpdate` of `Block.redstoneWire`, a
    /// `HashSet` vanilla iterates (BlockRedstoneWire.java:47), hence
    /// [`JavaHashSet`]. `clear()` keeps the grown table (as `HashSet.clear`
    /// does, :48), so the capacity, which fixes the iteration order, only
    /// grows within a world. Per world here, starting at capacity 16; the
    /// oracle replaces the set per scenario (census §6).
    // src: BlockRedstoneWire.java:10
    pub redstone_wire_blocks_needing_update: JavaHashSet<ChunkPosition>,
    /// `BlockRedstoneTorch.torchUpdates` (a static `ArrayList` of
    /// `RedstoneUpdateInfo`): the burnout history, one entry per turn-off of
    /// a powered torch with the world time, oldest first; entries older than
    /// 100 ticks are dropped from the front by every torch tick
    /// ([`redstone_torch`]). Shared by every world in Java; per world here,
    /// starting empty, as the oracle clears it per scenario (census §6).
    // src: BlockRedstoneTorch.java:9
    pub redstone_torch_updates: Vec<redstone_torch::RedstoneUpdateInfo>,
}

/// The per-instance scratch fields of a `BlockFlowing` singleton.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FlowingScratch {
    /// `numAdjacentSources`.
    pub num_adjacent_sources: i32,
    /// `isOptimalFlowDirection`.
    pub is_optimal_flow_direction: [bool; 4],
    /// `flowCost`.
    pub flow_cost: [i32; 4],
}

impl FlowingScratch {
    /// The field initialisers.
    // src: BlockFlowing.java:6-8
    pub const NEW: FlowingScratch = FlowingScratch {
        num_adjacent_sources: 0,
        is_optimal_flow_direction: [false; 4],
        flow_cost: [0; 4],
    };
}

impl BlockStatics {
    /// `BlockLeaves.setGraphicsLevel(fancy)` on `Block.leaves`: stores the
    /// flag (read by `isOpaqueCube` and the renderer) and swaps the leaves
    /// texture between the fancy and the fast tile.
    // src: BlockLeaves.java:185-188
    pub fn set_leaves_graphics_level(&mut self, fancy: bool) {
        self.graphics_level = fancy;
        let base = BLOCKS[ids::LEAVES as usize]
            .as_ref()
            .and_then(BlockDef::base_index_in_png)
            .expect("Block.leaves is a BlockLeaves");
        self.leaves_block_index_in_texture = base + if fancy { 0 } else { 1 };
    }

    /// The scratch fields of the `BlockFlowing` singleton with this id.
    pub fn flowing_scratch(&self, id: i32) -> Option<&FlowingScratch> {
        match id {
            8 => Some(&self.water_moving),
            10 => Some(&self.lava_moving),
            _ => None,
        }
    }

    /// `ignoreUpdates` of the `BlockPistonBase` singleton with this id.
    pub fn piston_ignore_updates(&self, id: i32) -> Option<bool> {
        match id {
            29 => Some(self.piston_sticky_base_ignore_updates),
            33 => Some(self.piston_base_ignore_updates),
            _ => None,
        }
    }
}

impl Default for BlockStatics {
    fn default() -> Self {
        let mut bounds = Box::new([BlockBounds::FULL; 256]);
        for (id, row) in BLOCKS.iter().enumerate() {
            if let Some(b) = row {
                bounds[id] = b.bounds;
            }
        }
        BlockStatics {
            bounds,
            graphics_level: false,
            leaves_block_index_in_texture: BLOCKS[ids::LEAVES as usize]
                .as_ref()
                .expect("Block.leaves")
                .block_index_in_texture,
            fall_instantly: false,
            water_moving: FlowingScratch::NEW,
            lava_moving: FlowingScratch::NEW,
            leaves_adjacent_tree_blocks: None,
            piston_sticky_base_ignore_updates: false,
            piston_base_ignore_updates: false,
            // src: BlockPistonExtension.java:7
            piston_extension_head_texture: -1,
            // src: BlockRedstoneWire.java:9
            redstone_wire_wires_provide_power: true,
            redstone_wire_blocks_needing_update: JavaHashSet::new(),
            // src: BlockRedstoneTorch.java:9
            redstone_torch_updates: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stone_row() {
        // src: Block.java:593: setHardness(1.5F).setResistance(10.0F)
        let s = block(1).unwrap();
        assert_eq!(s.kind, BlockKind::Stone);
        assert_eq!(s.block_hardness, 1.5);
        assert_eq!(s.block_resistance, 30.0); // 10 * 3, Block.java:187
        assert_eq!(s.block_index_in_texture, 1);
        assert_eq!(s.block_material, Material::Rock);
        assert_eq!(s.get_block_name().as_deref(), Some("tile.stone"));
        assert!(s.opaque_cube_lookup);
        assert_eq!(s.light_opacity, 255);
    }

    #[test]
    fn hardness_after_resistance_only_raises() {
        // sandStone: setStepSound then setHardness(0.8F): resistance 4.0
        let s = block(24).unwrap();
        assert_eq!(s.block_resistance, 0.8f32 * 5.0);
        // obsidian: setHardness(10).setResistance(2000) -> 6000
        assert_eq!(block(49).unwrap().block_resistance, 6000.0);
        // bedrock: unbreakable, resistance 6000000*3
        let b = block(7).unwrap();
        assert_eq!(b.block_hardness, -1.0);
        assert_eq!(b.block_resistance, 18000000.0);
    }

    #[test]
    fn per_id_arrays() {
        assert!(can_block_grass(0));
        assert!(!opaque_cube_lookup(0));
        assert_eq!(light_opacity(8), 3); // water setLightOpacity(3)
        assert_eq!(light_opacity(18), 1); // leaves
        assert!(opaque_cube_lookup(18)); // construction-time quirk
        assert!(!opaque_cube_lookup(43)); // construction-time quirk
        assert!(is_opaque_cube(BlockKind::Step { block_type: true }, false));
        assert_eq!(light_opacity(43), 255);
        assert_eq!(light_value(50), 14); // torch (int)(15 * 0.9375)
        assert_eq!(light_value(89), 15);
        assert_eq!(light_value(39), 1); // brown mushroom (int)(15 * 0.125)
        assert!(tick_on_load(2)); // grass
        assert!(!tick_on_load(9)); // still water
        assert!(tick_on_load(11)); // still lava
        assert!(tick_on_load(95)); // locked chest setTickOnLoad(true)
        assert!(is_block_container(54));
        assert!(!is_block_container(95));
        assert!(set_neighbor_notify_on_metadata_change(5));
        assert!(!set_neighbor_notify_on_metadata_change(1));
        assert!(block(97).is_none());
        assert_eq!(BLOCKS.iter().filter(|b| b.is_some()).count(), 96);
    }

    /// Inputs the block-table fixture does not reach (it covers sides 0..6).
    #[test]
    fn texture_index_checks() {
        let st = BlockStatics::default();
        let bed = block(26).unwrap();
        // ModelBed.bedDirection[d][side] (BlockBed.java:90) throws past 5.
        assert_eq!(
            get_block_texture_from_side_and_metadata(bed, 6, 0, &st),
            None
        );
        assert_eq!(
            get_block_texture_from_side_and_metadata(bed, -1, 0, &st),
            None
        );
        // The extension's front side never indexes faceToSide (BlockPistonExtension.java:43-48).
        let ext = block(34).unwrap();
        assert_eq!(
            get_block_texture_from_side_and_metadata(ext, 6, 6, &st),
            Some(107)
        );
        assert_eq!(
            get_block_texture_from_side_and_metadata(ext, 0, 6, &st),
            None
        );
        // setGraphicsLevel swaps the leaves tile (BlockLeaves.java:185-188);
        // baseIndexInPNG stays the constructor's 52.
        let leaves = block(18).unwrap();
        let mut fast = st.clone();
        fast.set_leaves_graphics_level(false);
        assert_eq!(get_block_texture_from_side(leaves, 2, &fast), 53);
        assert_eq!(
            get_block_texture_from_side_and_metadata(leaves, 2, 1, &fast),
            Some(53 + 80)
        );
        assert_eq!(leaves.base_index_in_png(), Some(52));
        fast.set_leaves_graphics_level(true);
        assert_eq!(fast.leaves_block_index_in_texture, 52);
        assert!(!is_opaque_cube(leaves.kind, fast.graphics_level));
        // Crops: meta < 0 reads as 7 (BlockCrops.java:78-80).
        assert_eq!(
            get_block_texture_from_side_and_metadata(block(59).unwrap(), 2, -1, &st),
            Some(88 + 7)
        );
    }

    #[test]
    fn subclass_statics() {
        assert_eq!(STEP_TYPE_NAMES, ["stone", "sand", "wood", "cobble"]);
        assert_eq!(REPEATER_DELAYS, [1, 2, 3, 4]);
        // -0.0625D etc. are exact binary fractions.
        assert_eq!(
            REPEATER_TORCH_OFFSET[0].to_bits(),
            (-1.0f64 / 16.0).to_bits()
        );
        assert_eq!(REPEATER_TORCH_OFFSET[3], 5.0 / 16.0);
    }

    #[test]
    fn bounds_and_derived_rows() {
        // flower: 0.5F - 0.2F etc. in float, widened.
        let f = block(37).unwrap();
        assert_eq!(f.bounds.min_x, (0.5f32 - 0.2f32) as f64);
        assert_eq!(f.bounds.max_y, (0.2f32 * 3.0f32) as f64);
        // stairs copy their model block (BlockStairs.java:10-15)
        let st = block(53).unwrap();
        assert_eq!(st.block_hardness, 2.0);
        assert_eq!(st.block_resistance, 15.0);
        assert_eq!(st.block_index_in_texture, 4);
        assert_eq!(st.step_sound, SOUND_WOOD_FOOTSTEP);
        assert_eq!(tick_rate(block(8).unwrap()), 5);
        assert_eq!(tick_rate(block(10).unwrap()), 30);
        assert_eq!(block(79).unwrap().slipperiness, 0.98);
        assert_eq!(block(71).unwrap().block_index_in_texture, 98); // iron door
        assert_eq!(block(70).unwrap().block_index_in_texture, 1); // stone.blockIndexInTexture
    }
}
