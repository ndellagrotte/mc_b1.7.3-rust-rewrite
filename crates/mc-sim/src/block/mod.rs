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

pub mod dispatch;
pub mod fluid;
pub mod material;
mod table;

pub use material::{MapColor, Material};

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

    /// `getMobilityFlag()`.
    // src: Block.java:588-590
    pub const fn get_mobility_flag(&self) -> i32 {
        self.block_material.get_material_mobility()
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

/// The run-time-mutable state of the `Block` singletons (see the module docs).
///
/// Vanilla blocks are shared singletons whose fields some methods overwrite:
/// `setBlockBoundsBasedOnState` rewrites `minX`…`maxZ` before collision and
/// selection queries (doors, slabs, rails, …), and the renderer sets
/// `BlockLeavesBase.graphicsLevel`. Since the result of later queries depends
/// on the last write, this is world state.
///
/// Fields still to add as their behaviour is transcribed (Phase 1), each a
/// Java static or a mutable singleton field: `BlockSand.fallInstantly`,
/// `BlockFurnace.keepFurnaceInventory`, `BlockRedstoneTorch.torchUpdates`,
/// `BlockRedstoneWire.wiresProvidePower` / `blocksNeedingUpdate`,
/// `BlockPistonBase.ignoreUpdates`, `BlockFlowing`'s scratch fields, and the
/// singleton `Random`s the oracle reseeds (`BlockChest.random`,
/// `BlockFurnace.furnaceRand`, `BlockDispenser.random`).
#[derive(Clone, Debug)]
pub struct BlockStatics {
    /// `minX`…`maxZ` of each block singleton, starting from
    /// [`BlockDef::bounds`]. Index = block id; unused ids hold
    /// [`BlockBounds::FULL`].
    pub bounds: Box<[BlockBounds; 256]>,
    /// `BlockLeavesBase.graphicsLevel` of `Block.leaves` (the only
    /// `BlockLeavesBase` instance). Set by the client's fancy-graphics option
    /// (RenderGlobal); false in the oracle, which has no renderer.
    pub graphics_level: bool,
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
