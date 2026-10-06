//! `BlockRedstoneOre` (BlockRedstoneOre.java): redstone ore (id 73) and
//! glowing redstone ore (id 74, the one with random ticks and light).
//! `field_468_a` is [`super::BlockKind::RedstoneOre`]'s `field_468_a` (true
//! for id 74).
//!
//! Fully transcribed: `onBlockClicked`, `onEntityWalking`, `blockActivated`,
//! `func_320_h` (here [`glow`], see `docs/mappings.md`), `updateTick`,
//! `randomDisplayTick`, `sparkle`. Data methods (`tickRate` 30,
//! `idDropped`, `quantityDropped`) are in [`crate::block`].
//!
//! `sparkle` draws from `World.rand` (BlockRedstoneOre.java:67), not from
//! the `Random` it is handed, so the glowing ore's `randomDisplayTick`
//! (client particles) moves `World.rand`: gameplay-visible, transcribed.

use crate::world::World;

/// `Block.oreRedstone.blockID`.
// src: Block.java:665
const ORE_REDSTONE: i32 = 73;
/// `Block.oreRedstoneGlowing.blockID`.
// src: Block.java:666
const ORE_REDSTONE_GLOWING: i32 = 74;

/// `onBlockClicked(world, x, y, z, player)`: glow, then
/// `super.onBlockClicked` (empty).
// src: BlockRedstoneOre.java:21-24
pub fn on_block_clicked(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    glow(world, id, x, y, z);
}

/// `onEntityWalking(world, x, y, z, entity)`: glow, then
/// `super.onEntityWalking` (empty).
// src: BlockRedstoneOre.java:26-29
pub fn on_entity_walking(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    glow(world, id, x, y, z);
}

/// `blockActivated(world, x, y, z, player)`: glow, then
/// `super.blockActivated` (Block's `false`).
// src: BlockRedstoneOre.java:31-34, Block.java:506-508
pub fn block_activated(world: &mut World, id: i32, x: i32, y: i32, z: i32) -> bool {
    glow(world, id, x, y, z);
    false
}

/// `func_320_h(world, x, y, z)` (private): sparkle, and the plain ore turns
/// into the glowing one.
// src: BlockRedstoneOre.java:36-42
fn glow(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    sparkle(world, x, y, z);
    if id == ORE_REDSTONE {
        world.set_block_with_notify(x, y, z, ORE_REDSTONE_GLOWING);
    }
}

/// `updateTick(world, x, y, z, rand)`: the glowing ore (a random tick) turns
/// back into the plain one.
// src: BlockRedstoneOre.java:44-49
pub fn update_tick(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    if id == ORE_REDSTONE_GLOWING {
        world.set_block_with_notify(x, y, z, ORE_REDSTONE);
    }
}

/// `randomDisplayTick(world, x, y, z, rand)`: the glowing ore sparkles
/// (`rand` unused: `sparkle` draws from `World.rand`).
// src: BlockRedstoneOre.java:59-64
pub fn random_display_tick(world: &mut World, x: i32, y: i32, z: i32, field_468_a: bool) {
    if field_468_a {
        sparkle(world, x, y, z);
    }
}

/// `sparkle(world, x, y, z)` (private): six red dust particles, one per
/// face, each with three `World.rand.nextFloat()` draws (x, y, z), moved
/// just outside its face when the neighbour there is not an opaque cube,
/// and spawned only if outside the block. The y test compares with 0, not
/// with `y` (vanilla bug, transcribed: `var11 < 0.0D` where the other axes
/// test against the block's own coordinate).
// src: BlockRedstoneOre.java:66-103
fn sparkle(world: &mut World, x: i32, y: i32, z: i32) {
    let var6 = 0.0625f64;
    for var8 in 0..6 {
        // src: BlockRedstoneOre.java:71-73 (`(double)((float)x + rand.nextFloat())`)
        let mut var9 = (x as f32 + world.rand.next_float()) as f64;
        let mut var11 = (y as f32 + world.rand.next_float()) as f64;
        let mut var13 = (z as f32 + world.rand.next_float()) as f64;
        if var8 == 0 && !world.is_block_opaque_cube(x, y + 1, z) {
            var11 = (y + 1) as f64 + var6;
        }
        if var8 == 1 && !world.is_block_opaque_cube(x, y - 1, z) {
            var11 = y as f64 - var6;
        }
        if var8 == 2 && !world.is_block_opaque_cube(x, y, z + 1) {
            var13 = (z + 1) as f64 + var6;
        }
        if var8 == 3 && !world.is_block_opaque_cube(x, y, z - 1) {
            var13 = z as f64 - var6;
        }
        if var8 == 4 && !world.is_block_opaque_cube(x + 1, y, z) {
            var9 = (x + 1) as f64 + var6;
        }
        if var8 == 5 && !world.is_block_opaque_cube(x - 1, y, z) {
            var9 = x as f64 - var6;
        }
        // vanilla bug: `var11 < 0.0D` (BlockRedstoneOre.java:98) should be `< y`.
        if var9 < x as f64
            || var9 > (x + 1) as f64
            || var11 < 0.0
            || var11 > (y + 1) as f64
            || var13 < z as f64
            || var13 > (z + 1) as f64
        {
            world.spawn_particle("reddust", var9, var11, var13, 0.0, 0.0, 0.0);
        }
    }
}
