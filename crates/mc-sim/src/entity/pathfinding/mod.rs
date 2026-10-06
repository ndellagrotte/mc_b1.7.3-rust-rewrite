//! Mob pathfinding (WP-2.2): `PathPoint`, `Path` (the open-set binary
//! heap), `PathEntity` (a found path), `Pathfinder` (the A* search) and
//! the block view `World.getPathToEntity` / `getEntityPathToXYZ` build for it
//! (`ChunkCache`).
//!
//! | Java | Port |
//! |---|---|
//! | `PathPoint` (PathPoint.java) | [`path_point::PathPoint`] |
//! | `Path` (Path.java) | [`path::Path`] |
//! | `PathEntity` (PathEntity.java) | [`path_entity::PathEntity`] |
//! | `Pathfinder` (Pathfinder.java) | [`pathfinder::Pathfinder`] |
//! | `MCHash` (`Pathfinder.pointMap`) | a `BTreeMap<i32, usize>` keyed by the point hash: lookup only, never iterated (PLAN_v4 §3.4 rule 4) |
//! | `ChunkCache` (ChunkCache.java), as `IBlockAccess` | [`ChunkCache`] (block id and metadata only: all `Pathfinder` reads) |
//!
//! # Object identity
//!
//! Java's `PathPoint`s are objects that the point map, the heap, the
//! `previous` links and the `pathOptions` scratch array share. Here the
//! search owns them in an arena (`Vec<PathPoint>`) and every one of those
//! references is an index into it; `==` on points is index equality.
//!
//! # The point map keys on the hash alone
//!
//! `MCHash.lookup` compares only the `int` key (MCHash.java:19-29), and the
//! key is `PathPoint.func_22329_a(x, y, z)` (PathPoint.java:19-21), which is
//! not injective: `z` keeps 8 bits (`(z & 32767) << 24`), and the `z < 0`
//! flag `0x8000` shares bit 15 with bit 7 of `x`. `openPoint` therefore
//! returns an existing point whose coordinates differ from the ones asked
//! for when two cells collide (vanilla bug, transcribed: the map is keyed by
//! the hash only).

pub mod path;
pub mod path_entity;
pub mod path_point;
pub mod pathfinder;

pub use path_entity::PathEntity;
pub use path_point::PathPoint;
pub use pathfinder::Pathfinder;

use crate::world::World;
use crate::world::chunk::Chunk;

/// The block reads `Pathfinder` makes through its `IBlockAccess`
/// (Pathfinder.java:359, :375).
pub trait PathBlockAccess {
    /// `getBlockId(x, y, z)`.
    fn get_block_id(&self, x: i32, y: i32, z: i32) -> i32;
    /// `getBlockMetadata(x, y, z)`.
    fn get_block_metadata(&self, x: i32, y: i32, z: i32) -> i32;
}

/// `ChunkCache` (ChunkCache.java): a read-only window on the chunks
/// `(x0 >> 4, z0 >> 4)` to `(x1 >> 4, z1 >> 4)`. The constructor loads (or
/// generates) every chunk of the window through `getChunkFromChunkCoords`,
/// x outer, z inner, as Java's does (a gameplay-visible side effect: chunk
/// loading and population); reads after that never load. Outside the window,
/// and at `y` outside 0..127, the block id is 0.
pub struct ChunkCache<'w> {
    /// `chunkX`.
    chunk_x: i32,
    /// `chunkZ`.
    chunk_z: i32,
    /// `chunkArray.length` (x extent).
    len_x: i32,
    /// `chunkArray[i].length` (z extent).
    len_z: i32,
    world: &'w World,
}

impl<'w> ChunkCache<'w> {
    /// `new ChunkCache(world, x0, y0, z0, x1, y1, z1)`: loads the window's
    /// chunks into `world`, then borrows it read-only. The `y` bounds are
    /// unused, as in Java.
    // src: ChunkCache.java:9-23
    pub fn new(world: &'w mut World, x0: i32, z0: i32, x1: i32, z1: i32) -> ChunkCache<'w> {
        let chunk_x = x0 >> 4;
        let chunk_z = z0 >> 4;
        let var8 = x1 >> 4;
        let var9 = z1 >> 4;
        for var10 in chunk_x..=var8 {
            for var11 in chunk_z..=var9 {
                world.get_chunk_from_chunk_coords(var10, var11);
            }
        }
        ChunkCache {
            chunk_x,
            chunk_z,
            len_x: var8 - chunk_x + 1,
            len_z: var9 - chunk_z + 1,
            world,
        }
    }

    /// `chunkArray[x - chunkX][z - chunkZ]` for chunk coordinates in the
    /// window; `None` outside it.
    fn chunk(&self, cx: i32, cz: i32) -> Option<&Chunk> {
        let var4 = cx - self.chunk_x;
        let var5 = cz - self.chunk_z;
        if var4 >= 0 && var4 < self.len_x && var5 >= 0 && var5 < self.len_z {
            // Every chunk of the window was loaded by the constructor and
            // nothing unloads chunks while a path is searched.
            self.world.loaded_chunk(cx, cz)
        } else {
            None
        }
    }
}

impl PathBlockAccess for ChunkCache<'_> {
    // src: ChunkCache.java:25-40
    fn get_block_id(&self, x: i32, y: i32, z: i32) -> i32 {
        if !(0..128).contains(&y) {
            return 0;
        }
        match self.chunk(x >> 4, z >> 4) {
            Some(c) => c.get_block_id(x & 15, y, z & 15),
            None => 0,
        }
    }

    /// Java indexes `chunkArray` without a bounds check (an
    /// `ArrayIndexOutOfBoundsException` outside the window); `Pathfinder`
    /// asks only for doors whose id it read inside the window, so 0 is
    /// never observed there.
    // src: ChunkCache.java:115-125
    fn get_block_metadata(&self, x: i32, y: i32, z: i32) -> i32 {
        if !(0..128).contains(&y) {
            return 0;
        }
        match self.chunk(x >> 4, z >> 4) {
            Some(c) => c.get_block_metadata(x & 15, y, z & 15),
            None => 0,
        }
    }
}
