//! The tree generators (`WorldGenerator` and its tree subclasses:
//! WorldGenerator.java, WorldGenTrees.java, WorldGenForest.java,
//! WorldGenTaiga1.java, WorldGenTaiga2.java, WorldGenBigTree.java),
//! transcribed for `BlockSapling.growTree` (WP-1.6) and reused by the
//! population of WP-1.7.
//!
//! # Shape
//!
//! `WorldGenerator` is an abstract class with one abstract method,
//! `generate(world, rand, x, y, z)`, and one hook, `func_517_a(a, b, c)`,
//! which only `WorldGenBigTree` overrides. Here it is the closed enum
//! [`WorldGen`] (one variant per instantiated subclass, as
//! [`crate::block::BlockKind`] is for blocks); the variant holds the
//! instance fields, so a `WorldGen` value is one Java instance and keeps its
//! state between calls (`WorldGenBigTree.field_878_e` survives a second
//! `generate` on the same instance, as in Java). WP-1.7 adds the other
//! `WorldGen*` classes as variants.
//!
//! Every `generate` has the uniform signature
//! `generate(&mut self, world, rand, x, y, z) -> bool`. `rand` is the
//! [`RandomSource`] the Java caller passes, drawn through
//! [`World::random`] (as block callbacks do, `block::dispatch` module docs):
//! `BlockSapling.growTree` passes `World.rand` (BlockSapling.java:20, :51);
//! the population callers pass the provider's `Random`, which WP-1.7 adds as
//! a source.
//!
//! `WorldGenBigTree` keeps the world in a field (`worldObj`) for its helper
//! methods; here the helpers take the world as a parameter instead.
//!
//! Block ids are `Block.x.blockID` (Block.java:593-688); `WorldGenBigTree`
//! uses the literals 17 (wood) and 18 (leaves) itself.

use mc_core::JavaRandom;
use mc_core::math::{floor_double, jmath};

use super::biome::WorldGenTreeKind;
use crate::block::dispatch::RandomSource;
use crate::block::{self, ids};
use crate::world::World;

/// A `WorldGenerator` instance (module docs).
#[derive(Clone, Debug)]
pub enum WorldGen {
    /// `WorldGenTrees`: the small oak.
    Trees,
    /// `WorldGenBigTree`: the large oak, with its instance state.
    BigTree(Box<WorldGenBigTree>),
    /// `WorldGenForest`: birch.
    Forest,
    /// `WorldGenTaiga1`: the tall, thin spruce.
    Taiga1,
    /// `WorldGenTaiga2`: the layered spruce.
    Taiga2,
}

impl WorldGen {
    /// `new WorldGen<kind>()` for the class a biome or sapling picks.
    pub fn new(kind: WorldGenTreeKind) -> Self {
        match kind {
            WorldGenTreeKind::Trees => WorldGen::Trees,
            WorldGenTreeKind::BigTree => WorldGen::BigTree(Box::default()),
            WorldGenTreeKind::Forest => WorldGen::Forest,
            WorldGenTreeKind::Taiga1 => WorldGen::Taiga1,
            WorldGenTreeKind::Taiga2 => WorldGen::Taiga2,
        }
    }

    /// `generate(world, rand, x, y, z)`: true if the feature was placed.
    // src: WorldGenerator.java:6
    pub fn generate(
        &mut self,
        world: &mut World,
        rand: RandomSource,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        match self {
            WorldGen::Trees => generate_trees(world, rand, x, y, z),
            WorldGen::BigTree(t) => t.generate(world, rand, x, y, z),
            WorldGen::Forest => generate_forest(world, rand, x, y, z),
            WorldGen::Taiga1 => generate_taiga1(world, rand, x, y, z),
            WorldGen::Taiga2 => generate_taiga2(world, rand, x, y, z),
        }
    }

    /// `func_517_a(a, b, c)`: the scale hook; empty in `WorldGenerator`,
    /// only `WorldGenBigTree` overrides it (population calls it with
    /// `(1.0, 1.0, 1.0)` before `generate`, ChunkProviderGenerate.java:450).
    // src: WorldGenerator.java:8-9
    pub fn func_517_a(&mut self, var1: f64, var3: f64, var5: f64) {
        if let WorldGen::BigTree(t) = self {
            t.func_517_a(var1, var3, var5);
        }
    }
}

/// `Block.opaqueCubeLookup[world.getBlockId(x, y, z)]`.
fn opaque_at(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    let id = world.get_block_id(x, y, z);
    block::opaque_cube_lookup(id)
}

/// The clearance test shared by `WorldGenTrees` and `WorldGenForest`
/// (WorldGenTrees.java:14-36, WorldGenForest.java:14-36): every block of
/// the column and crown space is air or leaves and inside y 0..128. The
/// outer loop does not stop early (only the inner two test the flag).
fn small_tree_clear(world: &mut World, x: i32, y: i32, z: i32, var6: i32) -> bool {
    let mut var7 = true;
    let mut var8 = y;
    while var8 <= y + 1 + var6 {
        let mut var9: i32 = 1;
        if var8 == y {
            var9 = 0;
        }
        if var8 >= y + 1 + var6 - 2 {
            var9 = 2;
        }
        let mut var10 = x - var9;
        while var10 <= x + var9 && var7 {
            let mut var11 = z - var9;
            while var11 <= z + var9 && var7 {
                if (0..128).contains(&var8) {
                    let var12 = world.get_block_id(var10, var8, var11);
                    if var12 != 0 && var12 != ids::LEAVES {
                        var7 = false;
                    }
                } else {
                    var7 = false;
                }
                var11 += 1;
            }
            var10 += 1;
        }
        var8 += 1;
    }
    var7
}

/// The body shared by `WorldGenTrees.generate` and `WorldGenForest.generate`
/// after the height draw: they differ only in the height (`nextInt(3) + 4`
/// vs `+ 5`) and in the metadata of the leaves and wood they place
/// (`setBlock` with 0 vs `setBlockAndMetadata` with 2).
fn small_tree(
    world: &mut World,
    rand: RandomSource,
    x: i32,
    y: i32,
    z: i32,
    var6: i32,
    meta: Option<i32>,
) -> bool {
    let set = |world: &mut World, bx: i32, by: i32, bz: i32, id: i32| match meta {
        None => world.set_block(bx, by, bz, id),
        Some(m) => world.set_block_and_metadata(bx, by, bz, id, m),
    };
    // Java: `y >= 1 && y + var6 + 1 <= 128`.
    if !(y >= 1 && y + var6 < 128) {
        return false;
    }
    if !small_tree_clear(world, x, y, z, var6) {
        return false;
    }
    let var8 = world.get_block_id(x, y - 1, z);
    if !((var8 == ids::GRASS || var8 == ids::DIRT) && y < 128 - var6 - 1) {
        return false;
    }
    world.set_block(x, y - 1, z, ids::DIRT);
    let mut var16 = y - 3 + var6;
    while var16 <= y + var6 {
        let var10 = var16 - (y + var6);
        let var11 = 1 - var10 / 2;
        for var12 in (x - var11)..=(x + var11) {
            let var13 = var12 - x;
            for var14 in (z - var11)..=(z + var11) {
                let var15 = var14 - z;
                // Java: (|dx| != r || |dz| != r || rand.nextInt(2) != 0 && dy != 0)
                // && !opaqueCubeLookup[...]: the corner draw only on corners,
                // the block read only when the first part holds.
                if (var13.wrapping_abs() != var11
                    || var15.wrapping_abs() != var11
                    || world.random(rand).next_int_bounded(2) != 0 && var10 != 0)
                    && !opaque_at(world, var12, var16, var14)
                {
                    set(world, var12, var16, var14, ids::LEAVES);
                }
            }
        }
        var16 += 1;
    }
    for var16 in 0..var6 {
        let var10 = world.get_block_id(x, y + var16, z);
        if var10 == 0 || var10 == ids::LEAVES {
            set(world, x, y + var16, z, ids::WOOD);
        }
    }
    true
}

/// `WorldGenTrees.generate`: height `nextInt(3) + 4`, leaves and wood with
/// metadata 0 (`setBlock`).
// src: WorldGenTrees.java:6-77
pub fn generate_trees(world: &mut World, rand: RandomSource, x: i32, y: i32, z: i32) -> bool {
    let var6 = world.random(rand).next_int_bounded(3) + 4;
    small_tree(world, rand, x, y, z, var6, None)
}

/// `WorldGenForest.generate`: height `nextInt(3) + 5`, leaves and wood with
/// metadata 2 (birch).
// src: WorldGenForest.java:6-77
pub fn generate_forest(world: &mut World, rand: RandomSource, x: i32, y: i32, z: i32) -> bool {
    let var6 = world.random(rand).next_int_bounded(3) + 5;
    // src: WorldGenForest.java:56, :65
    small_tree(world, rand, x, y, z, var6, Some(2))
}

/// The clearance test of the two spruce generators (WorldGenTaiga1.java:18-38,
/// WorldGenTaiga2.java:17-37): radius 0 below `var7` above the base, else
/// `radius`; every loop stops at the first obstruction.
fn spruce_clear(
    world: &mut World,
    x: i32,
    y: i32,
    z: i32,
    var6: i32,
    var7: i32,
    radius: i32,
) -> bool {
    let mut var10 = true;
    let mut var11 = y;
    while var11 <= y + 1 + var6 && var10 {
        let var18 = if var11 - y < var7 { 0 } else { radius };
        let mut var13 = x - var18;
        while var13 <= x + var18 && var10 {
            let mut var14 = z - var18;
            while var14 <= z + var18 && var10 {
                if (0..128).contains(&var11) {
                    let var15 = world.get_block_id(var13, var11, var14);
                    if var15 != 0 && var15 != ids::LEAVES {
                        var10 = false;
                    }
                } else {
                    var10 = false;
                }
                var14 += 1;
            }
            var13 += 1;
        }
        var11 += 1;
    }
    var10
}

/// `WorldGenTaiga1.generate`: the tall spruce with a short crown, leaves and
/// wood with metadata 1.
// src: WorldGenTaiga1.java:6-82
pub fn generate_taiga1(world: &mut World, rand: RandomSource, x: i32, y: i32, z: i32) -> bool {
    let var6 = world.random(rand).next_int_bounded(5) + 7;
    let var7 = var6 - world.random(rand).next_int_bounded(2) - 3;
    let var8 = var6 - var7;
    let var9 = 1 + world.random(rand).next_int_bounded(var8 + 1);
    // Java: `y >= 1 && y + var6 + 1 <= 128`.
    if !(y >= 1 && y + var6 < 128) {
        return false;
    }
    if !spruce_clear(world, x, y, z, var6, var7, var9) {
        return false;
    }
    let var11 = world.get_block_id(x, y - 1, z);
    if !((var11 == ids::GRASS || var11 == ids::DIRT) && y < 128 - var6 - 1) {
        return false;
    }
    world.set_block(x, y - 1, z, ids::DIRT);
    let mut var18 = 0;
    let mut var13 = y + var6;
    while var13 >= y + var7 {
        for var14 in (x - var18)..=(x + var18) {
            let var15 = var14 - x;
            for var16 in (z - var18)..=(z + var18) {
                let var17 = var16 - z;
                if (var15.wrapping_abs() != var18 || var17.wrapping_abs() != var18 || var18 <= 0)
                    && !opaque_at(world, var14, var13, var16)
                {
                    // src: WorldGenTaiga1.java:55
                    world.set_block_and_metadata(var14, var13, var16, ids::LEAVES, 1);
                }
            }
        }
        // src: WorldGenTaiga1.java:60-64
        if var18 >= 1 && var13 == y + var7 + 1 {
            var18 -= 1;
        } else if var18 < var9 {
            var18 += 1;
        }
        var13 -= 1;
    }
    for var13 in 0..(var6 - 1) {
        let var14 = world.get_block_id(x, y + var13, z);
        if var14 == 0 || var14 == ids::LEAVES {
            // src: WorldGenTaiga1.java:70
            world.set_block_and_metadata(x, y + var13, z, ids::WOOD, 1);
        }
    }
    true
}

/// `WorldGenTaiga2.generate`: the layered spruce, leaves and wood with
/// metadata 1 (the sapling with metadata 1 grows this one).
// src: WorldGenTaiga2.java:6-94
pub fn generate_taiga2(world: &mut World, rand: RandomSource, x: i32, y: i32, z: i32) -> bool {
    let var6 = world.random(rand).next_int_bounded(4) + 6;
    let var7 = 1 + world.random(rand).next_int_bounded(2);
    let var8 = var6 - var7;
    let var9 = 2 + world.random(rand).next_int_bounded(2);
    // Java: `y >= 1 && y + var6 + 1 <= 128`.
    if !(y >= 1 && y + var6 < 128) {
        return false;
    }
    if !spruce_clear(world, x, y, z, var6, var7, var9) {
        return false;
    }
    let var11 = world.get_block_id(x, y - 1, z);
    if !((var11 == ids::GRASS || var11 == ids::DIRT) && y < 128 - var6 - 1) {
        return false;
    }
    world.set_block(x, y - 1, z, ids::DIRT);
    let mut var21 = world.random(rand).next_int_bounded(2);
    let mut var13 = 1;
    let mut var22 = 0;
    for var15 in 0..=var8 {
        let var16 = y + var6 - var15;
        for var17 in (x - var21)..=(x + var21) {
            let var18 = var17 - x;
            for var19 in (z - var21)..=(z + var21) {
                let var20 = var19 - z;
                if (var18.wrapping_abs() != var21 || var20.wrapping_abs() != var21 || var21 <= 0)
                    && !opaque_at(world, var17, var16, var19)
                {
                    // src: WorldGenTaiga2.java:60
                    world.set_block_and_metadata(var17, var16, var19, ids::LEAVES, 1);
                }
            }
        }
        // src: WorldGenTaiga2.java:65-74
        if var21 >= var13 {
            var21 = var22;
            var22 = 1;
            var13 += 1;
            if var13 > var9 {
                var13 = var9;
            }
        } else {
            var21 += 1;
        }
    }
    let var15 = world.random(rand).next_int_bounded(3);
    for var16 in 0..(var6 - var15) {
        let var17 = world.get_block_id(x, y + var16, z);
        if var17 == 0 || var17 == ids::LEAVES {
            // src: WorldGenTaiga2.java:82
            world.set_block_and_metadata(x, y + var16, z, ids::WOOD, 1);
        }
    }
    true
}

/// `WorldGenBigTree.field_882_a`: for a main axis `a` (0 x, 1 y, 2 z), the
/// two other axes are `[a]` and `[a + 3]`.
// src: WorldGenBigTree.java:6
const FIELD_882_A: [usize; 6] = [2, 0, 0, 1, 2, 1];

/// `Block.wood.blockID` / `Block.leaves.blockID` as `WorldGenBigTree` writes
/// them (literals in the Java: WorldGenBigTree.java:116, :162, :227, :252).
const BIG_TREE_WOOD: i32 = 17;
const BIG_TREE_LEAVES: i32 = 18;

/// A `WorldGenBigTree` instance. Field names are kept (MCP has no names for
/// them); their meaning is in `docs/mappings.md` ("Kept").
#[derive(Clone, Debug)]
pub struct WorldGenBigTree {
    /// `field_881_b`: the tree's own `Random`; `new Random()` (clock seed,
    /// census §2) but always `setSeed(rand.nextLong())` in `generate` before
    /// its first draw (WorldGenBigTree.java:333-334), so the initial seed
    /// never matters.
    // src: WorldGenBigTree.java:7
    pub field_881_b: JavaRandom,
    /// `basePos`.
    // src: WorldGenBigTree.java:9
    pub base_pos: [i32; 3],
    /// `field_878_e`: the total height limit; drawn on the first `generate`
    /// when 0, lowered by `func_519_e` to the first obstruction.
    // src: WorldGenBigTree.java:10
    pub field_878_e: i32,
    /// `height`: the trunk height.
    // src: WorldGenBigTree.java:11
    pub height: i32,
    /// `field_876_g` (0.618): trunk height / height limit.
    // src: WorldGenBigTree.java:12
    pub field_876_g: f64,
    /// `field_875_h` (1.0): never read.
    // src: WorldGenBigTree.java:13
    pub field_875_h: f64,
    /// `field_874_i` (0.381): how far down the trunk a branch starts per
    /// block of horizontal distance.
    // src: WorldGenBigTree.java:14
    pub field_874_i: f64,
    /// `field_873_j` (1.0): branch length scale (`func_517_a`'s second argument).
    // src: WorldGenBigTree.java:15
    pub field_873_j: f64,
    /// `field_872_k` (1.0): leaf-cluster count scale (`func_517_a`'s third argument).
    // src: WorldGenBigTree.java:16
    pub field_872_k: f64,
    /// `field_871_l` (1): trunk width; 2 makes a 2×2 trunk.
    // src: WorldGenBigTree.java:17
    pub field_871_l: i32,
    /// `field_870_m` (12): the bound of the height-limit draw.
    // src: WorldGenBigTree.java:18
    pub field_870_m: i32,
    /// `field_869_n` (4): the height of one leaf cluster.
    // src: WorldGenBigTree.java:19
    pub field_869_n: i32,
    /// `field_868_o`: the leaf nodes `[x, y, z, branch base y]`.
    // src: WorldGenBigTree.java:20
    pub field_868_o: Vec<[i32; 4]>,
}

impl Default for WorldGenBigTree {
    fn default() -> Self {
        Self::new()
    }
}

impl WorldGenBigTree {
    /// `new WorldGenBigTree()`: the field initialisers.
    // src: WorldGenBigTree.java:6-20
    pub fn new() -> Self {
        WorldGenBigTree {
            field_881_b: JavaRandom::new(0),
            base_pos: [0, 0, 0],
            field_878_e: 0,
            height: 0,
            field_876_g: 0.618,
            field_875_h: 1.0,
            field_874_i: 0.381,
            field_873_j: 1.0,
            field_872_k: 1.0,
            field_871_l: 1,
            field_870_m: 12,
            field_869_n: 4,
            field_868_o: Vec::new(),
        }
    }

    /// `func_521_a()`: the leaf nodes, layer by layer from the top cluster
    /// down; a node is kept if the line to its cluster top and the line from
    /// its branch base on the trunk are both free (`func_524_a == -1`).
    // src: WorldGenBigTree.java:22-88
    fn func_521_a(&mut self, world: &mut World) {
        self.height = (self.field_878_e as f64 * self.field_876_g) as i32;
        if self.height >= self.field_878_e {
            self.height = self.field_878_e - 1;
        }
        let mut var1 =
            (1.382 + jmath::pow(self.field_872_k * self.field_878_e as f64 / 13.0, 2.0)) as i32;
        if var1 < 1 {
            var1 = 1;
        }
        // `new int[var1 * field_878_e][4]`; an index past it would throw in
        // Java as it panics here (it cannot happen: at most
        // `var1 * (field_878_e - field_869_n + 1) + 1` nodes).
        let mut var2 = vec![[0i32; 4]; var1.wrapping_mul(self.field_878_e).max(0) as usize];
        let mut var3 = self.base_pos[1] + self.field_878_e - self.field_869_n;
        let mut var4 = 1usize;
        let var5 = self.base_pos[1] + self.height;
        let mut var6 = var3 - self.base_pos[1];
        var2[0] = [self.base_pos[0], var3, self.base_pos[2], var5];
        var3 -= 1;
        while var6 >= 0 {
            let mut var7 = 0;
            let var8 = self.func_528_a(var6);
            if var8 < 0.0 {
                var3 -= 1;
                var6 -= 1;
            } else {
                let var9 = 0.5;
                while var7 < var1 {
                    // src: WorldGenBigTree.java:53-56
                    let var11 = self.field_873_j
                        * var8 as f64
                        * (self.field_881_b.next_float() as f64 + 0.328);
                    let var13 = self.field_881_b.next_float() as f64 * 2.0 * 3.14159;
                    let var15 =
                        floor_double(var11 * jmath::sin(var13) + self.base_pos[0] as f64 + var9);
                    let var16 =
                        floor_double(var11 * jmath::cos(var13) + self.base_pos[2] as f64 + var9);
                    let var17 = [var15, var3, var16];
                    let var18 = [var15, var3 + self.field_869_n, var16];
                    if self.func_524_a(world, var17, var18) == -1 {
                        let mut var19 = [self.base_pos[0], self.base_pos[1], self.base_pos[2]];
                        // src: WorldGenBigTree.java:61
                        let var20 = jmath::sqrt(
                            jmath::pow(
                                self.base_pos[0].wrapping_sub(var17[0]).wrapping_abs() as f64,
                                2.0,
                            ) + jmath::pow(
                                self.base_pos[2].wrapping_sub(var17[2]).wrapping_abs() as f64,
                                2.0,
                            ),
                        );
                        let var22 = var20 * self.field_874_i;
                        if var17[1] as f64 - var22 > var5 as f64 {
                            var19[1] = var5;
                        } else {
                            var19[1] = (var17[1] as f64 - var22) as i32;
                        }
                        if self.func_524_a(world, var19, var17) == -1 {
                            var2[var4] = [var15, var3, var16, var19[1]];
                            var4 += 1;
                        }
                    }
                    var7 += 1;
                }
                var3 -= 1;
                var6 -= 1;
            }
        }
        var2.truncate(var4);
        self.field_868_o = var2;
    }

    /// `func_523_a(x, y, z, radius, axis, blockId)`: a disc of `blockId`
    /// perpendicular to `axis` (only air and leaves are replaced).
    // src: WorldGenBigTree.java:90-127
    #[allow(clippy::too_many_arguments)]
    fn func_523_a(
        &self,
        world: &mut World,
        var1: i32,
        var2: i32,
        var3: i32,
        var4: f32,
        var5: usize,
        var6: i32,
    ) {
        let var7 = (var4 as f64 + 0.618) as i32;
        let var8 = FIELD_882_A[var5];
        let var9 = FIELD_882_A[var5 + 3];
        let var10 = [var1, var2, var3];
        let mut var11 = [0i32; 3];
        var11[var5] = var10[var5];
        let mut var12 = -var7;
        while var12 <= var7 {
            var11[var8] = var10[var8] + var12;
            let mut var13 = -var7;
            while var13 <= var7 {
                let var15 = jmath::sqrt(
                    jmath::pow(var12.wrapping_abs() as f64 + 0.5, 2.0)
                        + jmath::pow(var13.wrapping_abs() as f64 + 0.5, 2.0),
                );
                if var15 > var4 as f64 {
                    var13 += 1;
                } else {
                    var11[var9] = var10[var9] + var13;
                    let var14 = world.get_block_id(var11[0], var11[1], var11[2]);
                    if var14 != 0 && var14 != BIG_TREE_LEAVES {
                        var13 += 1;
                    } else {
                        world.set_block(var11[0], var11[1], var11[2], var6);
                        var13 += 1;
                    }
                }
            }
            var12 += 1;
        }
    }

    /// `func_528_a(layer)`: the branch-spread radius of a layer above the
    /// base (a half circle over the height limit), negative below 30 %.
    // src: WorldGenBigTree.java:129-147
    fn func_528_a(&self, var1: i32) -> f32 {
        if (var1 as f64) < (self.field_878_e as f32) as f64 * 0.3 {
            return -1.618;
        }
        let var2 = self.field_878_e as f32 / 2.0;
        let var3 = self.field_878_e as f32 / 2.0 - var1 as f32;
        let mut var4 = if var3 == 0.0 {
            var2
        } else if var3.abs() >= var2 {
            0.0
        } else {
            jmath::sqrt(jmath::pow(var2.abs() as f64, 2.0) - jmath::pow(var3.abs() as f64, 2.0))
                as f32
        };
        var4 *= 0.5;
        var4
    }

    /// `func_526_b(layer)`: the leaf-cluster disc radius at a layer of the
    /// cluster: 2 at the bottom and top, 3 between, -1 outside.
    // src: WorldGenBigTree.java:149-155
    fn func_526_b(&self, var1: i32) -> f32 {
        if var1 >= 0 && var1 < self.field_869_n {
            if var1 != 0 && var1 != self.field_869_n - 1 {
                3.0
            } else {
                2.0
            }
        } else {
            -1.0
        }
    }

    /// `func_520_a(x, y, z)`: one leaf cluster, `field_869_n` discs upward.
    // src: WorldGenBigTree.java:157-165
    fn func_520_a(&self, world: &mut World, var1: i32, var2: i32, var3: i32) {
        let mut var4 = var2;
        let var5 = var2 + self.field_869_n;
        while var4 < var5 {
            let var6 = self.func_526_b(var4 - var2);
            self.func_523_a(world, var1, var4, var3, var6, 1, BIG_TREE_LEAVES);
            var4 += 1;
        }
    }

    /// `func_522_a(from, to, blockId)`: a line of `blockId` (stepping along
    /// the longest axis, the others rounded), replacing anything.
    // src: WorldGenBigTree.java:167-202
    fn func_522_a(&self, world: &mut World, var1: [i32; 3], var2: [i32; 3], var3: i32) {
        let mut var4 = [0i32; 3];
        let mut var6 = 0usize;
        for var5 in 0..3 {
            var4[var5] = var2[var5].wrapping_sub(var1[var5]);
            if var4[var5].wrapping_abs() > var4[var6].wrapping_abs() {
                var6 = var5;
            }
        }
        if var4[var6] != 0 {
            let var7 = FIELD_882_A[var6];
            let var8 = FIELD_882_A[var6 + 3];
            let var9 = if var4[var6] > 0 { 1 } else { -1 };
            let var10 = var4[var7] as f64 / var4[var6] as f64;
            let var12 = var4[var8] as f64 / var4[var6] as f64;
            let mut var14 = [0i32; 3];
            let mut var15 = 0;
            let var16 = var4[var6] + var9;
            while var15 != var16 {
                var14[var6] = floor_double((var1[var6] + var15) as f64 + 0.5);
                var14[var7] = floor_double(var1[var7] as f64 + var15 as f64 * var10 + 0.5);
                var14[var8] = floor_double(var1[var8] as f64 + var15 as f64 * var12 + 0.5);
                world.set_block(var14[0], var14[1], var14[2], var3);
                var15 += var9;
            }
        }
    }

    /// `func_518_b()`: a leaf cluster at every node.
    // src: WorldGenBigTree.java:204-214
    fn func_518_b(&self, world: &mut World) {
        for node in &self.field_868_o {
            self.func_520_a(world, node[0], node[1], node[2]);
        }
    }

    /// `func_527_c(height)`: whether a branch base this high gets a branch.
    // src: WorldGenBigTree.java:216-218
    fn func_527_c(&self, var1: i32) -> bool {
        var1 as f64 >= self.field_878_e as f64 * 0.2
    }

    /// `func_529_c()`: the trunk (2×2 with `field_871_l == 2`).
    // src: WorldGenBigTree.java:220-240
    fn func_529_c(&self, world: &mut World) {
        let var1 = self.base_pos[0];
        let var2 = self.base_pos[1];
        let var3 = self.base_pos[1] + self.height;
        let var4 = self.base_pos[2];
        let mut var5 = [var1, var2, var4];
        let mut var6 = [var1, var3, var4];
        self.func_522_a(world, var5, var6, BIG_TREE_WOOD);
        if self.field_871_l == 2 {
            var5[0] += 1;
            var6[0] += 1;
            self.func_522_a(world, var5, var6, BIG_TREE_WOOD);
            var5[2] += 1;
            var6[2] += 1;
            self.func_522_a(world, var5, var6, BIG_TREE_WOOD);
            var5[0] += -1;
            var6[0] += -1;
            self.func_522_a(world, var5, var6, BIG_TREE_WOOD);
        }
    }

    /// `func_525_d()`: the branches, from each node's base on the trunk to
    /// the node, for bases high enough (`func_527_c`).
    // src: WorldGenBigTree.java:242-256
    fn func_525_d(&self, world: &mut World) {
        let mut var3 = [self.base_pos[0], self.base_pos[1], self.base_pos[2]];
        for var4 in &self.field_868_o {
            let var5 = [var4[0], var4[1], var4[2]];
            var3[1] = var4[3];
            let var6 = var3[1] - self.base_pos[1];
            if self.func_527_c(var6) {
                self.func_522_a(world, var3, var5, BIG_TREE_WOOD);
            }
        }
    }

    /// `func_524_a(from, to)`: walks the line like `func_522_a` without
    /// placing; -1 when every block is air or leaves, else the distance
    /// along the main axis to the first obstruction (-1 also for a
    /// zero-length line).
    // src: WorldGenBigTree.java:258-300
    fn func_524_a(&self, world: &mut World, var1: [i32; 3], var2: [i32; 3]) -> i32 {
        let mut var3 = [0i32; 3];
        let mut var5 = 0usize;
        for var4 in 0..3 {
            var3[var4] = var2[var4].wrapping_sub(var1[var4]);
            if var3[var4].wrapping_abs() > var3[var5].wrapping_abs() {
                var5 = var4;
            }
        }
        if var3[var5] == 0 {
            return -1;
        }
        let var6 = FIELD_882_A[var5];
        let var7 = FIELD_882_A[var5 + 3];
        let var8 = if var3[var5] > 0 { 1 } else { -1 };
        let var9 = var3[var6] as f64 / var3[var5] as f64;
        let var11 = var3[var7] as f64 / var3[var5] as f64;
        let mut var13 = [0i32; 3];
        let mut var14 = 0;
        let var15 = var3[var5] + var8;
        while var14 != var15 {
            var13[var5] = var1[var5] + var14;
            var13[var6] = floor_double(var1[var6] as f64 + var14 as f64 * var9);
            var13[var7] = floor_double(var1[var7] as f64 + var14 as f64 * var11);
            let var16 = world.get_block_id(var13[0], var13[1], var13[2]);
            if var16 != 0 && var16 != BIG_TREE_LEAVES {
                break;
            }
            var14 += var8;
        }
        if var14 == var15 {
            -1
        } else {
            var14.wrapping_abs()
        }
    }

    /// `func_519_e()`: grass or dirt below the base, and room for at least 6
    /// blocks of trunk (the height limit drops to the first obstruction).
    // src: WorldGenBigTree.java:302-319
    fn func_519_e(&mut self, world: &mut World) -> bool {
        let var1 = [self.base_pos[0], self.base_pos[1], self.base_pos[2]];
        let var2 = [
            self.base_pos[0],
            self.base_pos[1] + self.field_878_e - 1,
            self.base_pos[2],
        ];
        // src: WorldGenBigTree.java:306 (grass 2, dirt 3)
        let var3 = world.get_block_id(self.base_pos[0], self.base_pos[1] - 1, self.base_pos[2]);
        if var3 != ids::GRASS && var3 != ids::DIRT {
            return false;
        }
        let var4 = self.func_524_a(world, var1, var2);
        if var4 == -1 {
            true
        } else if var4 < 6 {
            false
        } else {
            self.field_878_e = var4;
            true
        }
    }

    /// `func_517_a(a, b, c)`: height-limit bound `(int)(a * 12)` (and leaf
    /// clusters 5 high above `a = 0.5`), branch length `b`, cluster count `c`.
    // src: WorldGenBigTree.java:321-329
    pub fn func_517_a(&mut self, var1: f64, var3: f64, var5: f64) {
        self.field_870_m = (var1 * 12.0) as i32;
        if var1 > 0.5 {
            self.field_869_n = 5;
        }
        self.field_873_j = var3;
        self.field_872_k = var5;
    }

    /// `generate(world, rand, x, y, z)`: reseeds the tree's own `Random` from
    /// `rand.nextLong()`, draws the height limit on first use, then the
    /// nodes, leaves, trunk and branches.
    // src: WorldGenBigTree.java:331-351
    pub fn generate(
        &mut self,
        world: &mut World,
        rand: RandomSource,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        let var6 = world.random(rand).next_long();
        self.field_881_b.set_seed(var6);
        self.base_pos = [x, y, z];
        if self.field_878_e == 0 {
            self.field_878_e = 5 + self.field_881_b.next_int_bounded(self.field_870_m);
        }
        if !self.func_519_e(world) {
            return false;
        }
        self.func_521_a(world);
        self.func_518_b(world);
        self.func_529_c(world);
        self.func_525_d(world);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn big_tree_layer_and_cluster_sizes() {
        let mut t = WorldGenBigTree::new();
        t.field_878_e = 10;
        // 3 < 10 * 0.3
        assert_eq!(t.func_528_a(2), -1.618);
        // var3 == 0: var2 / 2
        assert_eq!(t.func_528_a(5), 2.5);
        // |var3| >= var2: 0
        assert_eq!(t.func_528_a(10), 0.0);
        assert_eq!(t.func_526_b(0), 2.0);
        assert_eq!(t.func_526_b(1), 3.0);
        assert_eq!(t.func_526_b(3), 2.0);
        assert_eq!(t.func_526_b(4), -1.0);
        t.func_517_a(1.0, 1.0, 1.0);
        assert_eq!((t.field_870_m, t.field_869_n), (12, 5));
    }
}
