//! Block behaviour entry points (PLAN_v4 §3.2): `Block`'s overridable
//! methods as free functions that look the block up by id and `match` on its
//! [`BlockKind`].
//!
//! # Conventions every transcriber follows
//!
//! - Signature: Java's `(World world, int x, int y, int z, …)` becomes
//!   `(world: &mut World, id: i32, x, y, z, …)`. `id` replaces the `Block`
//!   receiver (`Block.blocksList[id]`); callers pass the id they looked up, as
//!   Java does before calling. Methods that only read the world take
//!   `&World`.
//! - Each function starts from the base `Block` implementation (the `_` arm,
//!   transcribed with its citation). An override gets its own arm, which calls
//!   a function in the module of the overriding class (`block/fire.rs`,
//!   `block/fluid.rs`, …) once transcribed.
//! - **Not yet transcribed** overrides are listed already, grouped by the
//!   class that defines the override (computed from the vanilla tree, so a
//!   subclass that inherits it, like `BlockGravel` from `BlockSand`, is in its
//!   parent's arm). Such an arm runs the base implementation and records
//!   `"Class.method"` with [`World::note_unimplemented`]. No `todo!()`: a
//!   fixture run shows exactly which overrides it reached. A transcriber
//!   replaces the arm; the list is the remaining work.
//! - `Random` parameters: every in-world caller passes `World.rand`
//!   (World.java:1159, :1960, :1988), so behaviour takes a
//!   [`RandomSource`] and draws through [`World::random`]. The two
//!   population callers (WorldGenLiquids.java:57, WorldGenHellLava.java:63)
//!   pass the generator's `Random`; WP-1.7 adds that source.
//!
//! Fully transcribed here already: `isOpaqueCube`, `renderAsNormalBlock`,
//! `tickRate` (constants and field reads, in [`crate::block`]), the base
//! `Block` implementations of everything below, and the overrides of
//! `BlockSand` (and `BlockGravel`, which inherits them; [`super::sand`]).

use mc_core::{Aabb, Vec3D};

use super::{BlockKind, block};
use crate::entity::Entity;
use crate::world::World;

/// Which `java.util.Random` a block callback receives (module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RandomSource {
    /// `World.rand`.
    WorldRand,
}

/// Records an untranscribed override that was reached.
fn untranscribed(world: &World, what: &'static str) {
    world.note_unimplemented(what);
}

/// `tickRate()`; fully transcribed ([`super::tick_rate`]). 10 for an empty id.
// src: Block.java:309-311
pub fn tick_rate(id: i32) -> i32 {
    block(id).map_or(10, super::tick_rate)
}

/// `isOpaqueCube()`; fully transcribed ([`super::is_opaque_cube`]), reading
/// the live `graphicsLevel` of the leaves.
// src: Block.java:285-287
pub fn is_opaque_cube(world: &World, id: i32) -> bool {
    block(id).is_some_and(|b| super::is_opaque_cube(b.kind, world.block_statics.graphics_level))
}

/// `renderAsNormalBlock()`; fully transcribed.
// src: Block.java:191-193
pub fn render_as_normal_block(id: i32) -> bool {
    block(id).is_some_and(|b| super::render_as_normal_block(b.kind))
}

/// `updateTick(world, x, y, z, rand)`. Base: nothing.
// src: Block.java:297-298
pub fn update_tick(world: &mut World, id: i32, x: i32, y: i32, z: i32, rand: RandomSource) {
    let _ = rand;
    let Some(b) = block(id) else { return };
    use BlockKind as K;
    let what = match b.kind {
        // src: BlockSand.java:20-22 (`rand` unused)
        K::Sand | K::Gravel => return super::sand::update_tick(world, id, x, y, z),
        K::Grass => "BlockGrass.updateTick",
        K::Sapling => "BlockSapling.updateTick",
        K::Flowing => "BlockFlowing.updateTick",
        K::Stationary => "BlockStationary.updateTick",
        K::Leaves => "BlockLeaves.updateTick",
        K::Dispenser => "BlockDispenser.updateTick",
        K::DetectorRail => "BlockDetectorRail.updateTick",
        K::TallGrass | K::DeadBush | K::Flower => "BlockFlower.updateTick",
        K::Mushroom => "BlockMushroom.updateTick",
        K::Torch => "BlockTorch.updateTick",
        K::Fire => "BlockFire.updateTick",
        K::Stairs { .. } => "BlockStairs.updateTick",
        K::Crops => "BlockCrops.updateTick",
        K::Farmland => "BlockFarmland.updateTick",
        K::PressurePlate { .. } => "BlockPressurePlate.updateTick",
        K::RedstoneOre { .. } => "BlockRedstoneOre.updateTick",
        K::RedstoneTorch { .. } => "BlockRedstoneTorch.updateTick",
        K::Button => "BlockButton.updateTick",
        K::Snow => "BlockSnow.updateTick",
        K::Ice => "BlockIce.updateTick",
        K::SnowBlock => "BlockSnowBlock.updateTick",
        K::Cactus => "BlockCactus.updateTick",
        K::Reed => "BlockReed.updateTick",
        K::RedstoneRepeater { .. } => "BlockRedstoneRepeater.updateTick",
        K::LockedChest => "BlockLockedChest.updateTick",
        _ => return,
    };
    untranscribed(world, what);
}

/// `onBlockAdded(world, x, y, z)`. Base: nothing.
// src: Block.java:313-314
pub fn on_block_added(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    let Some(b) = block(id) else { return };
    use BlockKind as K;
    let what = match b.kind {
        // src: BlockSand.java:12-14
        K::Sand | K::Gravel => return super::sand::on_block_added(world, id, x, y, z),
        K::Flowing => "BlockFlowing.onBlockAdded",
        K::Stationary => "BlockFluid.onBlockAdded",
        K::Sponge => "BlockSponge.onBlockAdded",
        K::Dispenser => "BlockDispenser.onBlockAdded",
        K::Note | K::MobSpawner | K::Chest | K::Sign { .. } | K::JukeBox => {
            "BlockContainer.onBlockAdded"
        }
        K::Rail { .. } | K::DetectorRail => "BlockRail.onBlockAdded",
        K::PistonBase { .. } => "BlockPistonBase.onBlockAdded",
        K::PistonMoving => "BlockPistonMoving.onBlockAdded",
        K::Step { .. } => "BlockStep.onBlockAdded",
        K::Tnt => "BlockTNT.onBlockAdded",
        K::Torch => "BlockTorch.onBlockAdded",
        K::Fire => "BlockFire.onBlockAdded",
        K::Stairs { .. } => "BlockStairs.onBlockAdded",
        K::RedstoneWire => "BlockRedstoneWire.onBlockAdded",
        K::Furnace { .. } => "BlockFurnace.onBlockAdded",
        K::PressurePlate { .. } => "BlockPressurePlate.onBlockAdded",
        K::RedstoneTorch { .. } => "BlockRedstoneTorch.onBlockAdded",
        K::Pumpkin { .. } => "BlockPumpkin.onBlockAdded",
        K::RedstoneRepeater { .. } => "BlockRedstoneRepeater.onBlockAdded",
        _ => return,
    };
    untranscribed(world, what);
}

/// `onBlockRemoval(world, x, y, z)`. Base: nothing.
// src: Block.java:316-317
pub fn on_block_removal(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    let _ = (x, y, z);
    let Some(b) = block(id) else { return };
    use BlockKind as K;
    let what = match b.kind {
        K::Log => "BlockLog.onBlockRemoval",
        K::Leaves => "BlockLeaves.onBlockRemoval",
        K::Sponge => "BlockSponge.onBlockRemoval",
        K::Dispenser => "BlockDispenser.onBlockRemoval",
        K::Note | K::MobSpawner | K::Sign { .. } => "BlockContainer.onBlockRemoval",
        K::PistonExtension => "BlockPistonExtension.onBlockRemoval",
        K::PistonMoving => "BlockPistonMoving.onBlockRemoval",
        K::Stairs { .. } => "BlockStairs.onBlockRemoval",
        K::Chest => "BlockChest.onBlockRemoval",
        K::RedstoneWire => "BlockRedstoneWire.onBlockRemoval",
        K::Furnace { .. } => "BlockFurnace.onBlockRemoval",
        K::Lever => "BlockLever.onBlockRemoval",
        K::PressurePlate { .. } => "BlockPressurePlate.onBlockRemoval",
        K::RedstoneTorch { .. } => "BlockRedstoneTorch.onBlockRemoval",
        K::Button => "BlockButton.onBlockRemoval",
        K::JukeBox => "BlockJukeBox.onBlockRemoval",
        _ => return,
    };
    untranscribed(world, what);
}

/// `onNeighborBlockChange(world, x, y, z, neighborId)`. Base: nothing.
// src: Block.java:306-307
pub fn on_neighbor_block_change(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    neighbor_id: i32,
) {
    let _ = neighbor_id;
    let Some(b) = block(id) else { return };
    use BlockKind as K;
    let what = match b.kind {
        // src: BlockSand.java:16-18
        K::Sand | K::Gravel => {
            return super::sand::on_neighbor_block_change(world, id, x, y, z);
        }
        K::Sapling | K::TallGrass | K::DeadBush | K::Flower | K::Mushroom | K::Crops => {
            "BlockFlower.onNeighborBlockChange"
        }
        K::Flowing => "BlockFluid.onNeighborBlockChange",
        K::Stationary => "BlockStationary.onNeighborBlockChange",
        K::Dispenser => "BlockDispenser.onNeighborBlockChange",
        K::Note => "BlockNote.onNeighborBlockChange",
        K::Bed => "BlockBed.onNeighborBlockChange",
        K::Rail { .. } | K::DetectorRail => "BlockRail.onNeighborBlockChange",
        K::PistonBase { .. } => "BlockPistonBase.onNeighborBlockChange",
        K::PistonExtension => "BlockPistonExtension.onNeighborBlockChange",
        K::PistonMoving => "BlockPistonMoving.onNeighborBlockChange",
        K::Tnt => "BlockTNT.onNeighborBlockChange",
        K::Torch => "BlockTorch.onNeighborBlockChange",
        K::Fire => "BlockFire.onNeighborBlockChange",
        K::RedstoneWire => "BlockRedstoneWire.onNeighborBlockChange",
        K::Farmland => "BlockFarmland.onNeighborBlockChange",
        K::Sign { .. } => "BlockSign.onNeighborBlockChange",
        K::Door => "BlockDoor.onNeighborBlockChange",
        K::Ladder => "BlockLadder.onNeighborBlockChange",
        K::Lever => "BlockLever.onNeighborBlockChange",
        K::PressurePlate { .. } => "BlockPressurePlate.onNeighborBlockChange",
        K::RedstoneTorch { .. } => "BlockRedstoneTorch.onNeighborBlockChange",
        K::Button => "BlockButton.onNeighborBlockChange",
        K::Snow => "BlockSnow.onNeighborBlockChange",
        K::Cactus => "BlockCactus.onNeighborBlockChange",
        K::Reed => "BlockReed.onNeighborBlockChange",
        K::Portal => "BlockPortal.onNeighborBlockChange",
        K::Cake => "BlockCake.onNeighborBlockChange",
        K::RedstoneRepeater { .. } => "BlockRedstoneRepeater.onNeighborBlockChange",
        K::TrapDoor => "BlockTrapDoor.onNeighborBlockChange",
        _ => return,
    };
    untranscribed(world, what);
}

/// The base `getCollisionBoundingBoxFromPool`: the block's current bounds
/// offset to `(x, y, z)`.
// src: Block.java:281-283
fn base_collision_box(world: &World, id: i32, x: i32, y: i32, z: i32) -> Aabb {
    let b = &world.block_statics.bounds[(id & 255) as usize];
    Aabb::new(
        x as f64 + b.min_x,
        y as f64 + b.min_y,
        z as f64 + b.min_z,
        x as f64 + b.max_x,
        y as f64 + b.max_y,
        z as f64 + b.max_z,
    )
}

/// `getCollisionBoundingBoxFromPool(world, x, y, z)`: the solid box, or
/// `None` for no collision.
// src: Block.java:281-283
pub fn get_collision_bounding_box_from_pool(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
) -> Option<Aabb> {
    let b = block(id)?;
    use BlockKind as K;
    let what = match b.kind {
        K::Sapling | K::TallGrass | K::DeadBush | K::Flower | K::Mushroom | K::Crops => {
            "BlockFlower.getCollisionBoundingBoxFromPool"
        }
        K::Flowing | K::Stationary => "BlockFluid.getCollisionBoundingBoxFromPool",
        K::Rail { .. } | K::DetectorRail => "BlockRail.getCollisionBoundingBoxFromPool",
        K::Web => "BlockWeb.getCollisionBoundingBoxFromPool",
        K::PistonMoving => "BlockPistonMoving.getCollisionBoundingBoxFromPool",
        K::Torch | K::RedstoneTorch { .. } => "BlockTorch.getCollisionBoundingBoxFromPool",
        K::Fire => "BlockFire.getCollisionBoundingBoxFromPool",
        K::Stairs { .. } => "BlockStairs.getCollisionBoundingBoxFromPool",
        K::RedstoneWire => "BlockRedstoneWire.getCollisionBoundingBoxFromPool",
        K::Farmland => "BlockFarmland.getCollisionBoundingBoxFromPool",
        K::Sign { .. } => "BlockSign.getCollisionBoundingBoxFromPool",
        K::Door => "BlockDoor.getCollisionBoundingBoxFromPool",
        K::Ladder => "BlockLadder.getCollisionBoundingBoxFromPool",
        K::Lever => "BlockLever.getCollisionBoundingBoxFromPool",
        K::PressurePlate { .. } => "BlockPressurePlate.getCollisionBoundingBoxFromPool",
        K::Button => "BlockButton.getCollisionBoundingBoxFromPool",
        K::Snow => "BlockSnow.getCollisionBoundingBoxFromPool",
        K::Cactus => "BlockCactus.getCollisionBoundingBoxFromPool",
        K::Reed => "BlockReed.getCollisionBoundingBoxFromPool",
        K::Fence => "BlockFence.getCollisionBoundingBoxFromPool",
        K::SoulSand => "BlockSoulSand.getCollisionBoundingBoxFromPool",
        K::Portal => "BlockPortal.getCollisionBoundingBoxFromPool",
        K::Cake => "BlockCake.getCollisionBoundingBoxFromPool",
        K::TrapDoor => "BlockTrapDoor.getCollisionBoundingBoxFromPool",
        _ => return Some(base_collision_box(world, id, x, y, z)),
    };
    untranscribed(world, what);
    Some(base_collision_box(world, id, x, y, z))
}

/// `getCollidingBoundingBoxes(world, x, y, z, mask, list)`: appends the
/// block's collision boxes that intersect `mask`.
// src: Block.java:273-279
pub fn get_colliding_bounding_boxes(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    mask: &Aabb,
    list: &mut Vec<Aabb>,
) {
    let Some(b) = block(id) else { return };
    use BlockKind as K;
    let what = match b.kind {
        K::PistonBase { .. } => Some("BlockPistonBase.getCollidingBoundingBoxes"),
        K::PistonExtension => Some("BlockPistonExtension.getCollidingBoundingBoxes"),
        K::Stairs { .. } => Some("BlockStairs.getCollidingBoundingBoxes"),
        _ => None,
    };
    if let Some(what) = what {
        untranscribed(world, what);
    }
    if let Some(var7) = get_collision_bounding_box_from_pool(world, id, x, y, z)
        && mask.intersects_with(&var7)
    {
        list.push(var7);
    }
}

/// `canPlaceBlockAt(world, x, y, z)`. Base: the target is air or ground cover.
// src: Block.java:501-504
pub fn can_place_block_at(world: &mut World, id: i32, x: i32, y: i32, z: i32) -> bool {
    let Some(b) = block(id) else { return false };
    use BlockKind as K;
    let what = match b.kind {
        K::Sapling | K::TallGrass | K::DeadBush | K::Flower | K::Mushroom | K::Crops => {
            Some("BlockFlower.canPlaceBlockAt")
        }
        K::Rail { .. } | K::DetectorRail => Some("BlockRail.canPlaceBlockAt"),
        K::PistonExtension => Some("BlockPistonExtension.canPlaceBlockAt"),
        K::PistonMoving => Some("BlockPistonMoving.canPlaceBlockAt"),
        K::Torch | K::RedstoneTorch { .. } => Some("BlockTorch.canPlaceBlockAt"),
        K::Fire => Some("BlockFire.canPlaceBlockAt"),
        K::Stairs { .. } => Some("BlockStairs.canPlaceBlockAt"),
        K::Chest => Some("BlockChest.canPlaceBlockAt"),
        K::RedstoneWire => Some("BlockRedstoneWire.canPlaceBlockAt"),
        K::Door => Some("BlockDoor.canPlaceBlockAt"),
        K::Ladder => Some("BlockLadder.canPlaceBlockAt"),
        K::Lever => Some("BlockLever.canPlaceBlockAt"),
        K::PressurePlate { .. } => Some("BlockPressurePlate.canPlaceBlockAt"),
        K::Button => Some("BlockButton.canPlaceBlockAt"),
        K::Snow => Some("BlockSnow.canPlaceBlockAt"),
        K::Cactus => Some("BlockCactus.canPlaceBlockAt"),
        K::Reed => Some("BlockReed.canPlaceBlockAt"),
        K::Fence => Some("BlockFence.canPlaceBlockAt"),
        K::Pumpkin { .. } => Some("BlockPumpkin.canPlaceBlockAt"),
        K::Cake => Some("BlockCake.canPlaceBlockAt"),
        K::RedstoneRepeater { .. } => Some("BlockRedstoneRepeater.canPlaceBlockAt"),
        K::LockedChest => Some("BlockLockedChest.canPlaceBlockAt"),
        _ => None,
    };
    if let Some(what) = what {
        untranscribed(world, what);
    }
    let var5 = world.get_block_id(x, y, z);
    var5 == 0 || super::material_of(var5).get_is_ground_cover()
}

/// `canPlaceBlockOnSide(world, x, y, z, side)`. Base: `canPlaceBlockAt`.
// src: Block.java:497-499
pub fn can_place_block_on_side(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    side: i32,
) -> bool {
    let _ = side;
    let Some(b) = block(id) else { return false };
    use BlockKind as K;
    let what = match b.kind {
        K::PistonExtension => Some("BlockPistonExtension.canPlaceBlockOnSide"),
        K::PistonMoving => Some("BlockPistonMoving.canPlaceBlockOnSide"),
        K::Lever => Some("BlockLever.canPlaceBlockOnSide"),
        K::Button => Some("BlockButton.canPlaceBlockOnSide"),
        K::TrapDoor => Some("BlockTrapDoor.canPlaceBlockOnSide"),
        _ => None,
    };
    if let Some(what) = what {
        untranscribed(world, what);
    }
    can_place_block_at(world, id, x, y, z)
}

/// `blockActivated(world, x, y, z, player)`: right-click. Base: false.
// src: Block.java:506-508
pub fn block_activated(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    player: &mut Entity,
) -> bool {
    let _ = (x, y, z, player);
    let Some(b) = block(id) else { return false };
    use BlockKind as K;
    let what = match b.kind {
        K::Dispenser => "BlockDispenser.blockActivated",
        K::Note => "BlockNote.blockActivated",
        K::Bed => "BlockBed.blockActivated",
        K::PistonBase { .. } => "BlockPistonBase.blockActivated",
        K::PistonMoving => "BlockPistonMoving.blockActivated",
        K::Tnt => "BlockTNT.blockActivated",
        K::Stairs { .. } => "BlockStairs.blockActivated",
        K::Chest => "BlockChest.blockActivated",
        K::Workbench => "BlockWorkbench.blockActivated",
        K::Furnace { .. } => "BlockFurnace.blockActivated",
        K::Door => "BlockDoor.blockActivated",
        K::Lever => "BlockLever.blockActivated",
        K::RedstoneOre { .. } => "BlockRedstoneOre.blockActivated",
        K::Button => "BlockButton.blockActivated",
        K::JukeBox => "BlockJukeBox.blockActivated",
        K::Cake => "BlockCake.blockActivated",
        K::RedstoneRepeater { .. } => "BlockRedstoneRepeater.blockActivated",
        K::TrapDoor => "BlockTrapDoor.blockActivated",
        _ => return false,
    };
    untranscribed(world, what);
    false
}

/// `onEntityWalking(world, x, y, z, entity)`. Base: nothing.
// src: Block.java:510-511
pub fn on_entity_walking(world: &mut World, id: i32, x: i32, y: i32, z: i32, e: &mut Entity) {
    let _ = (x, y, z, e);
    let Some(b) = block(id) else { return };
    use BlockKind as K;
    let what = match b.kind {
        K::Leaves => "BlockLeaves.onEntityWalking",
        K::Stairs { .. } => "BlockStairs.onEntityWalking",
        K::Farmland => "BlockFarmland.onEntityWalking",
        K::RedstoneOre { .. } => "BlockRedstoneOre.onEntityWalking",
        _ => return,
    };
    untranscribed(world, what);
}

/// `onEntityCollidedWithBlock(world, x, y, z, entity)`. Base: nothing.
// src: Block.java:541-542
pub fn on_entity_collided_with_block(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    e: &mut Entity,
) {
    let _ = (x, y, z, e);
    let Some(b) = block(id) else { return };
    use BlockKind as K;
    let what = match b.kind {
        K::DetectorRail => "BlockDetectorRail.onEntityCollidedWithBlock",
        K::Web => "BlockWeb.onEntityCollidedWithBlock",
        K::PressurePlate { .. } => "BlockPressurePlate.onEntityCollidedWithBlock",
        K::Cactus => "BlockCactus.onEntityCollidedWithBlock",
        K::SoulSand => "BlockSoulSand.onEntityCollidedWithBlock",
        K::Portal => "BlockPortal.onEntityCollidedWithBlock",
        _ => return,
    };
    untranscribed(world, what);
}

/// `velocityToAddToEntity(world, x, y, z, entity, vec)`: fluids push.
/// Base: nothing.
// src: Block.java:519-520
pub fn velocity_to_add_to_entity(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    e: &Entity,
    v: &mut Vec3D,
) {
    let _ = (x, y, z, e, v);
    let Some(b) = block(id) else { return };
    use BlockKind as K;
    let what = match b.kind {
        K::Flowing | K::Stationary => "BlockFluid.velocityToAddToEntity",
        K::Stairs { .. } => "BlockStairs.velocityToAddToEntity",
        _ => return,
    };
    untranscribed(world, what);
}

// ---- entry points added for World's queries (WP-1.5) ----------------------------

/// `canCollideCheck(meta, stopOnLiquid)`: whether a ray trace may select the
/// block. Base: `isCollidable()` ([`super::is_collidable`]).
// src: Block.java:289-291
pub fn can_collide_check(id: i32, meta: i32, stop_on_liquid: bool) -> bool {
    let Some(b) = block(id) else { return false };
    use BlockKind as K;
    match b.kind {
        // src: BlockFluid.java:56-58 (only a source block, and only when asked)
        K::Flowing | K::Stationary => stop_on_liquid && meta == 0,
        // src: BlockStairs.java:129-131: modelBlock.canCollideCheck, and both
        // model blocks (cobblestone, planks) keep the base body.
        K::Stairs { model_block } => block(model_block).is_some_and(super::is_collidable),
        _ => super::is_collidable(b),
    }
}

/// `setBlockBoundsBasedOnState(world, x, y, z)`: overrides rewrite the
/// singleton's bounds ([`crate::block::BlockStatics::bounds`]) from the
/// metadata. Base: nothing.
// src: Block.java:522-523
pub fn set_block_bounds_based_on_state(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    let _ = (x, y, z);
    let Some(b) = block(id) else { return };
    use BlockKind as K;
    let what = match b.kind {
        // src: BlockStairs.java:18-20
        K::Stairs { .. } => {
            world.block_statics.bounds[(id & 255) as usize] = super::BlockBounds::FULL;
            return;
        }
        K::Bed => "BlockBed.setBlockBoundsBasedOnState",
        K::Button => "BlockButton.setBlockBoundsBasedOnState",
        K::Rail { .. } | K::DetectorRail => "BlockRail.setBlockBoundsBasedOnState",
        K::PistonBase { .. } => "BlockPistonBase.setBlockBoundsBasedOnState",
        K::Snow => "BlockSnow.setBlockBoundsBasedOnState",
        K::Door => "BlockDoor.setBlockBoundsBasedOnState",
        K::Cake => "BlockCake.setBlockBoundsBasedOnState",
        K::PistonExtension => "BlockPistonExtension.setBlockBoundsBasedOnState",
        K::Lever => "BlockLever.setBlockBoundsBasedOnState",
        K::PistonMoving => "BlockPistonMoving.setBlockBoundsBasedOnState",
        K::Sign { .. } => "BlockSign.setBlockBoundsBasedOnState",
        K::PressurePlate { .. } => "BlockPressurePlate.setBlockBoundsBasedOnState",
        K::Portal => "BlockPortal.setBlockBoundsBasedOnState",
        K::TrapDoor => "BlockTrapDoor.setBlockBoundsBasedOnState",
        _ => return,
    };
    untranscribed(world, what);
}

/// The base `collisionRayTrace`: the segment `start -> end` against the
/// block's current bounds at `(x, y, z)`. Candidates are tested in Java's
/// order (min x, max x, min y, max y, min z, max z); a later candidate
/// replaces the best only when strictly nearer by `Vec3D.distanceTo` (a
/// `float` square root), and the side is the one of the chosen candidate
/// (Java compares the candidates by identity, so equal points keep the first).
// src: Block.java:375-488
fn base_collision_ray_trace(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    start: &Vec3D,
    end: &Vec3D,
) -> Option<crate::world::ray_trace::MovingObjectPosition> {
    set_block_bounds_based_on_state(world, id, x, y, z);
    let bb = world.block_statics.bounds[(id & 255) as usize];
    let var5 = start.add_vector(-(x as f64), -(y as f64), -(z as f64));
    let var6 = end.add_vector(-(x as f64), -(y as f64), -(z as f64));
    // src: Block.java:466-488 (isVecInside{YZ,XZ,XY}Bounds; inclusive)
    let in_yz = |v: &Vec3D| {
        v.y_coord >= bb.min_y
            && v.y_coord <= bb.max_y
            && v.z_coord >= bb.min_z
            && v.z_coord <= bb.max_z
    };
    let in_xz = |v: &Vec3D| {
        v.x_coord >= bb.min_x
            && v.x_coord <= bb.max_x
            && v.z_coord >= bb.min_z
            && v.z_coord <= bb.max_z
    };
    let in_xy = |v: &Vec3D| {
        v.x_coord >= bb.min_x
            && v.x_coord <= bb.max_x
            && v.y_coord >= bb.min_y
            && v.y_coord <= bb.max_y
    };
    // (candidate, sideHit): var7..var12 and the sides of Block.java:437-460.
    let candidates = [
        (
            var5.get_intermediate_with_x_value(&var6, bb.min_x)
                .filter(in_yz),
            4,
        ),
        (
            var5.get_intermediate_with_x_value(&var6, bb.max_x)
                .filter(in_yz),
            5,
        ),
        (
            var5.get_intermediate_with_y_value(&var6, bb.min_y)
                .filter(in_xz),
            0,
        ),
        (
            var5.get_intermediate_with_y_value(&var6, bb.max_y)
                .filter(in_xz),
            1,
        ),
        (
            var5.get_intermediate_with_z_value(&var6, bb.min_z)
                .filter(in_xy),
            2,
        ),
        (
            var5.get_intermediate_with_z_value(&var6, bb.max_z)
                .filter(in_xy),
            3,
        ),
    ];
    // src: Block.java:409-432
    let mut var13: Option<(Vec3D, i32)> = None;
    for (c, side) in candidates {
        let Some(c) = c else { continue };
        let nearer = match var13 {
            None => true,
            Some((best, _)) => var5.distance_to(&c) < var5.distance_to(&best),
        };
        if nearer {
            var13 = Some((c, side));
        }
    }
    // src: Block.java:434-462
    let (hit, side) = var13?;
    Some(crate::world::ray_trace::MovingObjectPosition::tile(
        x,
        y,
        z,
        side,
        hit.add_vector(x as f64, y as f64, z as f64),
    ))
}

/// `collisionRayTrace(world, x, y, z, start, end)`: where the segment enters
/// the block, or `None`. Base: `base_collision_ray_trace` (Block.java:375-488).
// src: Block.java:375-464
#[allow(clippy::too_many_arguments)]
pub fn collision_ray_trace(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    start: &Vec3D,
    end: &Vec3D,
) -> Option<crate::world::ray_trace::MovingObjectPosition> {
    let b = block(id)?;
    use BlockKind as K;
    let what = match b.kind {
        K::Door => Some("BlockDoor.collisionRayTrace"),
        K::Rail { .. } | K::DetectorRail => Some("BlockRail.collisionRayTrace"),
        K::TrapDoor => Some("BlockTrapDoor.collisionRayTrace"),
        K::Torch | K::RedstoneTorch { .. } => Some("BlockTorch.collisionRayTrace"),
        _ => None,
    };
    if let Some(what) = what {
        untranscribed(world, what);
    }
    base_collision_ray_trace(world, id, x, y, z, start, end)
}

/// `isPoweringTo(blockAccess, x, y, z, side)`: weak power into the
/// neighbour on `side`. Base: false.
// src: Block.java:533-535
pub fn is_powering_to(world: &mut World, id: i32, x: i32, y: i32, z: i32, side: i32) -> bool {
    let _ = (x, y, z, side);
    let Some(b) = block(id) else { return false };
    use BlockKind as K;
    let what = match b.kind {
        K::Button => "BlockButton.isPoweringTo",
        K::DetectorRail => "BlockDetectorRail.isPoweringTo",
        K::RedstoneRepeater { .. } => "BlockRedstoneRepeater.isPoweringTo",
        K::Lever => "BlockLever.isPoweringTo",
        K::RedstoneWire => "BlockRedstoneWire.isPoweringTo",
        K::RedstoneTorch { .. } => "BlockRedstoneTorch.isPoweringTo",
        K::PressurePlate { .. } => "BlockPressurePlate.isPoweringTo",
        _ => return false,
    };
    untranscribed(world, what);
    false
}

/// `isIndirectlyPoweringTo(world, x, y, z, side)`: strong power through
/// the block on `side`. Base: false.
// src: Block.java:544-546
pub fn is_indirectly_powering_to(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    side: i32,
) -> bool {
    let _ = (x, y, z, side);
    let Some(b) = block(id) else { return false };
    use BlockKind as K;
    let what = match b.kind {
        K::Button => "BlockButton.isIndirectlyPoweringTo",
        K::DetectorRail => "BlockDetectorRail.isIndirectlyPoweringTo",
        K::RedstoneRepeater { .. } => "BlockRedstoneRepeater.isIndirectlyPoweringTo",
        K::Lever => "BlockLever.isIndirectlyPoweringTo",
        K::RedstoneWire => "BlockRedstoneWire.isIndirectlyPoweringTo",
        K::RedstoneTorch { .. } => "BlockRedstoneTorch.isIndirectlyPoweringTo",
        K::PressurePlate { .. } => "BlockPressurePlate.isIndirectlyPoweringTo",
        _ => return false,
    };
    untranscribed(world, what);
    false
}

/// `randomDisplayTick(world, x, y, z, rand)`: client particles and sounds
/// for a block near the player. `rand` is `World.randomDisplayUpdates`'s
/// local `new Random()` (World.java:1999), not `World.rand`. Base: nothing.
// src: Block.java:300-301
pub fn random_display_tick(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    rand: &mut mc_core::java_random::JavaRandom,
) {
    let _ = (x, y, z, rand);
    let Some(b) = block(id) else { return };
    use BlockKind as K;
    let what = match b.kind {
        // src: BlockStairs.java:69-71: modelBlock.randomDisplayTick, and both
        // model blocks keep the base (empty) body.
        K::Stairs { .. } => return,
        K::RedstoneOre { .. } => "BlockRedstoneOre.randomDisplayTick",
        K::Fire => "BlockFire.randomDisplayTick",
        K::RedstoneWire => "BlockRedstoneWire.randomDisplayTick",
        K::RedstoneRepeater { .. } => "BlockRedstoneRepeater.randomDisplayTick",
        K::Flowing | K::Stationary => "BlockFluid.randomDisplayTick",
        K::Furnace { .. } => "BlockFurnace.randomDisplayTick",
        K::Portal => "BlockPortal.randomDisplayTick",
        K::RedstoneTorch { .. } => "BlockRedstoneTorch.randomDisplayTick",
        K::Torch => "BlockTorch.randomDisplayTick",
        _ => return,
    };
    untranscribed(world, what);
}

/// `playBlock(world, x, y, z, a, b)`: a block event (note block notes,
/// piston moves). Base: nothing.
// src: Block.java:576-577
pub fn play_block(world: &mut World, id: i32, x: i32, y: i32, z: i32, a: i32, b2: i32) {
    let _ = (x, y, z, a, b2);
    let Some(b) = block(id) else { return };
    use BlockKind as K;
    let what = match b.kind {
        K::Note => "BlockNote.playBlock",
        K::PistonBase { .. } => "BlockPistonBase.playBlock",
        _ => return,
    };
    untranscribed(world, what);
}
