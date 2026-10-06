//! WP-2.2: `Pathfinder` (Pathfinder.java), `Path` (Path.java), `PathPoint`,
//! `PathEntity` and `ChunkCache` on hand-built worlds, with the expected
//! point sequences derived by hand from the Java (the derivations are in
//! the test comments). No b1.7.3 animal paths to an entity, so the fixtures
//! only reach `getEntityPathToXYZ` through the wander AI
//! (`tests/golden/entity/pig-wander`); these pin the search itself.

use mc_sim::entity::pathfinding::path::Path;
use mc_sim::entity::pathfinding::{ChunkCache, PathBlockAccess, PathEntity, PathPoint};
use mc_sim::entity::{Entity, pig};
use mc_sim::save::NoSave;
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::{World, WorldEnvironment};

fn empty_world() -> World {
    let mut w = World::new(
        Box::new(NoSave),
        "harness",
        WorldProvider::new(Dimension::Surface),
        7,
        ChunkGenerator::Empty(EmptyChunkProvider),
        WorldEnvironment {
            rand_seed: 0,
            field_9437_g: 0,
            lock_timestamp: 0,
        },
    );
    w.make_deterministic(7);
    w
}

fn fill(w: &mut World, (x0, y0, z0): (i32, i32, i32), (x1, y1, z1): (i32, i32, i32), id: i32) {
    for x in x0..=x1 {
        for z in z0..=z1 {
            for y in y0..=y1 {
                w.set_block_and_metadata(x, y, z, id, 0);
            }
        }
    }
}

/// A pig (0.9 × 0.9: a 1 × 1 × 1 cell box for the search) standing at
/// `(x + 0.5, y, z + 0.5)`: its start cell is `(x, y, z)`.
fn pig_at(w: &mut World, x: i32, y: i32, z: i32) -> Entity {
    let mut p = pig::new_entity_pig(w);
    p.base
        .set_location_and_angles(x as f64 + 0.5, y as f64, z as f64 + 0.5, 0.0, 0.0);
    p
}

fn cells(p: &PathEntity) -> Vec<(i32, i32, i32)> {
    p.points()
        .iter()
        .map(|q| (q.x_coord, q.y_coord, q.z_coord))
        .collect()
}

/// The corridor world: solid stone y 55..62 over x, z -8..8, with a 1-wide,
/// 2-high corridor carved at z = 0, x 0..8, y 61..62 (the start cell is
/// (0, 61, 0); x = -1 and x = 9 are stone, the stone above the corridor
/// walls cannot be reached: one step up at most).
fn corridor() -> World {
    let mut w = empty_world();
    fill(&mut w, (-8, 55, -8), (8, 62, 8), 1);
    fill(&mut w, (0, 61, 0), (8, 62, 0), 0);
    w
}

/// Gap and ladder: the floor block (3, 60, 0) removed (stone at y 59 under
/// it), a ladder (id 65, meta 2, on the corridor wall at z = 1) at
/// (5, 61, 0). Target block (6, 61, 0): end cell `floor(6.5 - 0.45)`,
/// `floor(61.5)`, `floor(0.5 - 0.45)` = (6, 61, 0) (Pathfinder.java:17-29).
///
/// Derivation: the corridor admits one route. At (2, 61, 0) the `+x` cell
/// (3, 61, 0) is free (getVerticalOffset 1), and the drop loop of
/// `getSafePoint` (Pathfinder.java:120-131) moves it down to (3, 60, 0)
/// (free) and stops on the stone at (3, 59, 0) (0, not -2): the point is
/// (3, 60, 0). From there the cell above, (3, 61, 0), is free, so
/// `findPathOptions` allows a one-block step up (var7 = 1): `+x` (4, 60, 0)
/// is stone, (4, 61, 0) is free. The ladder is `Material.circuits`, not
/// solid (getVerticalOffset 1): (5, 61, 0) is an ordinary free cell.
#[test]
fn corridor_with_a_gap_and_a_ladder() {
    let mut w = corridor();
    w.set_block_and_metadata(3, 60, 0, 0, 0);
    w.set_block_and_metadata(5, 61, 0, 65, 2);
    let p = pig_at(&mut w, 0, 61, 0);
    let path = w
        .get_entity_path_to_xyz(&p.base, 6, 61, 0, 10.0)
        .expect("a path");
    assert_eq!(
        cells(&path),
        vec![
            (0, 61, 0),
            (1, 61, 0),
            (2, 61, 0),
            (3, 60, 0),
            (4, 61, 0),
            (5, 61, 0),
            (6, 61, 0)
        ]
    );
    assert_eq!(path.path_length, 7);
    assert!(!path.is_finished());
    // getPosition: the cell plus (int)(width + 1) * 0.5 = 0.5 horizontally
    // (PathEntity.java:25-30).
    let v = path.get_position(&p.base);
    assert_eq!((v.x_coord, v.y_coord, v.z_coord), (0.5, 61.0, 0.5));
    assert_eq!(path.func_22328_c().map(|q| q.x_coord), Some(6));
}

/// A closed wooden door (id 64, lower meta 0, upper meta 8: `isOpen` is
/// `meta & 4`, BlockDoor.java:196-198) at x = 4 blocks the corridor
/// (getVerticalOffset 0, Pathfinder.java:174-177, the upper half too, so no
/// step up). The open set empties; the result is the path to the dequeued
/// point closest to the target, (3, 61, 0) at distance 3
/// (Pathfinder.java:45-47, :69-73). Opened (meta 4 / 12) it is passable.
#[test]
fn closed_door_ends_the_path_at_the_closest_point() {
    let mut w = corridor();
    w.set_block_and_metadata(4, 61, 0, 64, 0);
    w.set_block_and_metadata(4, 62, 0, 64, 8);
    let p = pig_at(&mut w, 0, 61, 0);
    let path = w.get_entity_path_to_xyz(&p.base, 6, 61, 0, 10.0).unwrap();
    assert_eq!(
        cells(&path),
        vec![(0, 61, 0), (1, 61, 0), (2, 61, 0), (3, 61, 0)]
    );
    w.set_block_metadata(4, 61, 0, 4);
    w.set_block_metadata(4, 62, 0, 12);
    let path = w.get_entity_path_to_xyz(&p.base, 6, 61, 0, 10.0).unwrap();
    assert_eq!(cells(&path).len(), 7);
    assert_eq!(cells(&path)[4], (4, 61, 0));
}

/// No reachable point closer than the start: `null` (Pathfinder.java:69-70).
/// The corridor blocked 2 high at x = 1 (one block would be stepped onto:
/// the cell above it, (1, 62, 0), is free).
#[test]
fn walled_in_start_gives_no_path() {
    let mut w = corridor();
    fill(&mut w, (1, 61, 0), (1, 62, 0), 1);
    let p = pig_at(&mut w, 0, 61, 0);
    assert!(w.get_entity_path_to_xyz(&p.base, 6, 61, 0, 10.0).is_none());
}

/// The open-field world: stone y 55..60 over x, z -8..8, air above.
fn field() -> World {
    let mut w = empty_world();
    fill(&mut w, (-8, 55, -8), (8, 60, 8), 1);
    w
}

/// Two equally short routes from (0, 61, 0) to (1, 61, 1): the heap order
/// decides. Expanding the start adds `+z` (0, 1) [f = 1 + 1 = 2], `-x`
/// (-1, 0) [1 + √5], `+x` (1, 0) [2], `-z` (0, -1) [1 + √5] in that order
/// (Pathfinder.java:83-101); `sortBack` moves a point up only while it is
/// strictly smaller than its parent (Path.java:51-68), so (0, 1) stays the
/// root and is dequeued first. Its `+x` neighbour is the target (f = 2),
/// which `sortBack` lifts to slot 2 (above (0, -1)), below the root (1, 0)
/// (2 is not < 2). Dequeuing (1, 0) moves (0, -1) to the root and
/// `sortForward` picks the strictly smaller child, the target (Path.java:93-109);
/// (1, 0)'s offer of the target (g = 2) is not shorter than 2, so
/// `previous` stays (0, 1). The target is dequeued next.
#[test]
fn equal_routes_are_decided_by_the_heap() {
    let mut w = field();
    let p = pig_at(&mut w, 0, 61, 0);
    let path = w.get_entity_path_to_xyz(&p.base, 1, 61, 1, 10.0).unwrap();
    assert_eq!(cells(&path), vec![(0, 61, 0), (0, 61, 1), (1, 61, 1)]);
}

/// A cell that drops four blocks is refused (`var8 >= 4`,
/// Pathfinder.java:122-124), so is one whose drop ends in lava (-2,
/// :133-135); one over water is accepted (-1 ends the drop). Target
/// (2, 61, 0) from (0, 61, 0) past the cell (1, 61, 0).
///
/// Refused: the detour, derived by expanding the heap step by step
/// (f = g + h, `+z, -x, +x, -z` order, ties kept in place): S (0,0) → (0,1)
/// [√5 + 1], (0,-1) [same, added later, dequeued second], (1,1)
/// [2 + √2], (1,-1) [2 + √2, dequeued after (1,1)], (-1,0) [4], (2,1) [4,
/// added before (2,-1)]; expanding (2,1) adds the target (2,0) with f = 4
/// and `previous` (2,1); (2,-1) [4] is dequeued before it but offers the
/// target g = 4, not shorter. Path: (0,0) (0,1) (1,1) (2,1) (2,0).
#[test]
fn drops_of_four_and_lava_are_refused_water_is_not() {
    let detour = vec![(0, 61, 0), (0, 61, 1), (1, 61, 1), (2, 61, 1), (2, 61, 0)];
    // A 4-deep hole under (1, 61, 0).
    let mut w = field();
    fill(&mut w, (1, 57, 0), (1, 60, 0), 0);
    let p = pig_at(&mut w, 0, 61, 0);
    let path = w.get_entity_path_to_xyz(&p.base, 2, 61, 0, 10.0).unwrap();
    assert_eq!(cells(&path), detour);

    // A 3-deep hole: three free cells below are fine and the point is the
    // bottom of the drop. To that cell: the start's `+x` option is
    // (1, 58, 0) itself, g = √10, h = 0, below every other f (1 + √11,
    // 1 + √13), so it is dequeued next.
    let mut w = field();
    fill(&mut w, (1, 58, 0), (1, 60, 0), 0);
    let p = pig_at(&mut w, 0, 61, 0);
    let path = w.get_entity_path_to_xyz(&p.base, 1, 58, 0, 10.0).unwrap();
    assert_eq!(cells(&path), vec![(0, 61, 0), (1, 58, 0)]);

    // Lava under a one-block drop.
    let mut w = field();
    w.set_block_and_metadata(1, 60, 0, 0, 0);
    w.set_block_and_metadata(1, 59, 0, 11, 0);
    let p = pig_at(&mut w, 0, 61, 0);
    let path = w.get_entity_path_to_xyz(&p.base, 2, 61, 0, 10.0).unwrap();
    assert_eq!(cells(&path), detour);

    // Water instead of the floor block: the drop stops on it (-1), the
    // point stays (1, 61, 0) and the route is straight (f = 2 each step).
    let mut w = field();
    w.set_block_and_metadata(1, 60, 0, 9, 0);
    let p = pig_at(&mut w, 0, 61, 0);
    let path = w.get_entity_path_to_xyz(&p.base, 2, 61, 0, 10.0).unwrap();
    assert_eq!(cells(&path), vec![(0, 61, 0), (1, 61, 0), (2, 61, 0)]);
}

/// `getPathToEntity` paths to the target's `(posX, boundingBox.minY, posZ)`
/// (Pathfinder.java:13-15): a pig at (3.5, 61, 0.5) is the cell
/// `floor(3.5 - 0.45)` = 3.
#[test]
fn path_to_an_entity() {
    let mut w = corridor();
    let p = pig_at(&mut w, 0, 61, 0);
    let t = pig_at(&mut w, 3, 61, 0);
    let path = w.get_path_to_entity(&p.base, &t.base, 16.0).unwrap();
    assert_eq!(
        cells(&path),
        vec![(0, 61, 0), (1, 61, 0), (2, 61, 0), (3, 61, 0)]
    );
}

/// `range` limits the points to those closer than it to the target
/// (Pathfinder.java:87-101): with range 2 from (0, 61, 0) to (6, 61, 0),
/// no neighbour qualifies and there is no path.
#[test]
fn range_limits_the_search() {
    let mut w = corridor();
    let p = pig_at(&mut w, 0, 61, 0);
    assert!(w.get_entity_path_to_xyz(&p.base, 6, 61, 0, 2.0).is_none());
    // Range 6: (1, 61, 0) at distance 5 qualifies, (0, …) not needed;
    // the end is reached through points at distances 5 … 0.
    let path = w.get_entity_path_to_xyz(&p.base, 6, 61, 0, 6.0).unwrap();
    assert_eq!(cells(&path).len(), 7);
}

/// `PathPoint.func_22329_a` keeps 8 bits of z and shares bit 15 between
/// x's bit 7 and the `z < 0` flag (PathPoint.java:21-23), and `MCHash`
/// looks up by that key alone: these cells collide.
#[test]
fn point_hash_collisions() {
    let h = PathPoint::make_hash;
    assert_eq!(h(0, 64, 0), h(0, 64, 256));
    // x = 128 sets bit 15 through `(x & 32767) << 8`; z = -256 keeps no
    // bit of `(z & 32767) << 24` and sets bit 15 through `z < 0`.
    assert_eq!(h(128, 64, 0), h(0, 64, -256));
    assert_eq!(h(0, 64, -1) & 0x8000, 0x8000);
    assert_eq!(h(-1, 0, 0) & i32::MIN, i32::MIN);
    // y keeps 8 bits.
    assert_eq!(h(5, 300, 7), h(5, 44, 7));
    // equals compares coordinates too.
    assert!(!PathPoint::new(0, 64, 0).equals(&PathPoint::new(0, 64, 256)));
}

/// `Path` is Java's heap: ties keep insertion structure (`sortBack` moves
/// only while strictly smaller; `sortForward` takes the right child unless
/// the left is strictly smaller).
#[test]
fn path_heap_order() {
    let mut pts: Vec<PathPoint> = (0..6).map(|i| PathPoint::new(i, 0, 0)).collect();
    for (i, d) in [3.0f32, 1.0, 2.0, 1.0, 5.0, 2.0].iter().enumerate() {
        pts[i].distance_to_target = *d;
    }
    let mut heap = Path::new();
    for i in 0..6 {
        heap.add_point(&mut pts, i);
    }
    // add 0 [3]; add 1 [1]: up over 0 → [1, 0]; add 2 [2]: parent 1 (1) →
    // stays → [1, 0, 2]; add 3 [1]: parent slot 1 = point 0 (3) → up; parent
    // slot 0 = point 1 (1): not smaller → [1, 3, 2, 0]; add 4 [5] → slot 4;
    // add 5 [2]: parent slot 2 = point 2 (2): not smaller → slot 5.
    let mut order = Vec::new();
    while !heap.is_path_empty() {
        order.push(heap.dequeue(&mut pts));
    }
    // dequeue 1 (root); last (5) to root, children 3 [1], 2 [2]: 3 → [3, 5, 2, 0, 4];
    // slot 1 children 0 [3], 4 [5]: 3 < 5 but not < 2: stop.
    // dequeue 3; 4 to root: children 5 [2], 2 [2]: equal → right (2) →
    // [2, 5, 4, 0]; dequeue 2; 0 to root: children 5 [2], 4 [5] → 5 →
    // [5, 0, 4]; dequeue 5, then 0 [3], then 4 [5].
    assert_eq!(order, vec![1, 3, 2, 5, 0, 4]);
    assert!(pts.iter().all(|p| p.index == -1));
}

/// `ChunkCache` reads 0 outside its chunk window and outside y 0..127
/// (ChunkCache.java:25-40), even where the world has blocks.
#[test]
fn chunk_cache_window() {
    let mut w = field();
    let c = ChunkCache::new(&mut w, 0, 0, 15, 15);
    assert_eq!(c.get_block_id(3, 60, 3), 1);
    assert_eq!(c.get_block_id(-1, 60, 3), 0);
    assert_eq!(c.get_block_id(3, 60, -1), 0);
    assert_eq!(c.get_block_id(3, -1, 3), 0);
    assert_eq!(c.get_block_id(3, 128, 3), 0);
}

/// `PathEntity` walking: `incrementPathIndex` / `isFinished`, and the
/// centring offset for wide entities, `(int)(width + 1) * 0.5`.
#[test]
fn path_entity_walk() {
    let mut path = PathEntity::new(vec![PathPoint::new(1, 2, 3), PathPoint::new(4, 5, 6)]);
    let mut w = empty_world();
    let mut p = pig::new_entity_pig(&mut w);
    let v = path.get_position(&p.base);
    assert_eq!((v.x_coord, v.y_coord, v.z_coord), (1.5, 2.0, 3.5));
    p.base.set_size(1.4, 0.9);
    let v = path.get_position(&p.base);
    assert_eq!((v.x_coord, v.z_coord), (2.0, 4.0));
    path.increment_path_index();
    assert!(!path.is_finished());
    path.increment_path_index();
    assert!(path.is_finished());
}
