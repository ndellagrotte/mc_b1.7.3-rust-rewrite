//! `Explosion` (Explosion.java) and its world entry points
//! `World.createExplosion` / `World.newExplosion` (World.java:1518-1528,
//! [`World::create_explosion`], [`World::new_explosion`]).
//!
//! An explosion runs in two passes:
//!
//! - `doExplosionA`: 1352 rays from the centre (the surface of a 16³ grid),
//!   each with `size * (0.7 + 0.6 * World.rand.nextFloat())` power, losing
//!   `(resistance + 0.3) * 0.3` per block and `0.225` per step; every block
//!   a ray reaches with power left goes into `destroyedBlockPositions`, a
//!   Java `HashSet` (iterated, so [`JavaHashSet`]; PLAN_v4 §3.4 rule 4, the
//!   order is pinned by `tests/golden/hash/chunkposition-explosion`). Then
//!   every entity within twice the size is hurt and pushed by its exposure
//!   (`World.getBlockDensity`). A flaming explosion then sets fire on
//!   already-empty destroyed positions above an opaque block, 1 in 3 from
//!   the explosion's own `Random`.
//! - `doExplosionB(true)`: the sound, then for each destroyed position in
//!   reverse set order: particles (drawing `World.rand`), and for a block:
//!   its drops with chance 0.3, air, `onBlockDestroyedByExplosion`.
//!
//! `ExplosionRNG` is `new Random()` per explosion (clock-seeded,
//! Explosion.java:11; tools/oracle/census.md §2). The port seeds it with
//! `mix(seed, -6)` of the world's seeding protocol ([`EXPLOSION_RANDOM_KEY`]),
//! and the oracle's `explode` op seeds the vanilla object the same way
//! (tests/golden/README.md, op table). It only draws for a flaming
//! explosion.
//!
//! Callers in Phase 2: `EntityTNTPrimed.explode` (EntityTNTPrimed.java:68,
//! exploder null), `EntityCreeper` (EntityCreeper.java:101, :103, exploder
//! itself), `EntityFireball` (EntityFireball.java:127, null, flaming), and
//! `BlockBed` in the Nether (BlockBed.java:45, the bed task of WP-1.6). The
//! TNT and the fireball run this from their own update, taken out of their
//! slot, while vanilla's entity scan still finds them: they pass
//! themselves as `in_hand` to [`Explosion::do_explosion_a`] (the world
//! entry points pass none).

use mc_core::java_hash::{ChunkPosition, JavaHashSet};
use mc_core::java_random::JavaRandom;
use mc_core::math::{floor_double, jmath, sqrt_double};
use mc_core::{Aabb, Vec3D};

use super::World;
use crate::block::{self, dispatch as block_dispatch};
use crate::entity::base::mix_seed;
use crate::entity::{Entity, dispatch as entity_dispatch};

/// The `k` of `mix(seed, k)` that seeds `ExplosionRNG` (module docs).
pub const EXPLOSION_RANDOM_KEY: i64 = -6;

/// `Block.fire.blockID`.
// src: Block.java:643
const FIRE_ID: i32 = 51;

/// `Explosion`. The `worldObj` back-reference is the `world` argument of
/// the passes; `exploder` is kept as its entity id (the passes take the
/// entity itself, which may be out of its slot).
#[derive(Clone, Debug)]
pub struct Explosion {
    /// `isFlaming`.
    // src: Explosion.java:10
    pub is_flaming: bool,
    /// `ExplosionRNG` (private; module docs).
    // src: Explosion.java:11
    pub explosion_rng: JavaRandom,
    /// `explosionX`.
    // src: Explosion.java:13
    pub explosion_x: f64,
    /// `explosionY`.
    // src: Explosion.java:14
    pub explosion_y: f64,
    /// `explosionZ`.
    // src: Explosion.java:15
    pub explosion_z: f64,
    /// `exploder`, as its entity id.
    // src: Explosion.java:16
    pub exploder: Option<i32>,
    /// `explosionSize` (doubled during the entity pass of `doExplosionA`).
    // src: Explosion.java:17
    pub explosion_size: f32,
    /// `destroyedBlockPositions`: a `HashSet`, iterated.
    // src: Explosion.java:18
    pub destroyed_block_positions: JavaHashSet<ChunkPosition>,
}

impl Explosion {
    /// `new Explosion(world, exploder, x, y, z, size)`.
    // src: Explosion.java:20-27
    pub fn new(
        world: &World,
        exploder: Option<&Entity>,
        x: f64,
        y: f64,
        z: f64,
        size: f32,
    ) -> Explosion {
        Explosion {
            is_flaming: false,
            explosion_rng: JavaRandom::new(mix_seed(world.entity_rand_seed, EXPLOSION_RANDOM_KEY)),
            explosion_x: x,
            explosion_y: y,
            explosion_z: z,
            exploder: exploder.map(|e| e.base.entity_id),
            explosion_size: size,
            destroyed_block_positions: JavaHashSet::new(),
        }
    }

    /// `doExplosionA()`: the rays, the entity pass and, when flaming, the
    /// fire (module docs). `exploder` is the entity this explosion was made
    /// with (excluded from the entity pass, passed to
    /// `getExplosionResistance` and as the attacker); `in_hand` is an entity
    /// the caller holds out of its slot that vanilla's scan would find (the
    /// exploding TNT or fireball), tested where the scan reaches it.
    // src: Explosion.java:29-124
    pub fn do_explosion_a(
        &mut self,
        world: &mut World,
        exploder: Option<&Entity>,
        mut in_hand: Option<&mut Entity>,
    ) {
        // Java's one `exploder` field (Explosion.java:16) feeds both the
        // entity scan and the resistance/attacker roles: the two must agree.
        debug_assert_eq!(exploder.map(|e| e.base.entity_id), self.exploder);
        let var1 = self.explosion_size;
        let var2: i32 = 16;
        // src: Explosion.java:39-75
        for var3 in 0..var2 {
            for var4 in 0..var2 {
                for var5 in 0..var2 {
                    if var3 == 0
                        || var3 == var2 - 1
                        || var4 == 0
                        || var4 == var2 - 1
                        || var5 == 0
                        || var5 == var2 - 1
                    {
                        let mut var6 = (var3 as f32 / (var2 as f32 - 1.0) * 2.0 - 1.0) as f64;
                        let mut var8 = (var4 as f32 / (var2 as f32 - 1.0) * 2.0 - 1.0) as f64;
                        let mut var10 = (var5 as f32 / (var2 as f32 - 1.0) * 2.0 - 1.0) as f64;
                        let var12 = jmath::sqrt(var6 * var6 + var8 * var8 + var10 * var10);
                        var6 /= var12;
                        var8 /= var12;
                        var10 /= var12;
                        let mut var14 = self.explosion_size * (0.7 + world.rand.next_float() * 0.6);
                        let mut var15 = self.explosion_x;
                        let mut var17 = self.explosion_y;
                        let mut var19 = self.explosion_z;
                        let var21 = 0.3f32;
                        while var14 > 0.0 {
                            let var22 = floor_double(var15);
                            let var23 = floor_double(var17);
                            let var24 = floor_double(var19);
                            let var25 = world.get_block_id(var22, var23, var24);
                            if var25 > 0 {
                                var14 -= (block_dispatch::get_explosion_resistance(
                                    world, var25, exploder,
                                ) + 0.3)
                                    * var21;
                            }
                            if var14 > 0.0 {
                                self.destroyed_block_positions
                                    .insert(ChunkPosition::new(var22, var23, var24));
                            }
                            var15 += var6 * var21 as f64;
                            var17 += var8 * var21 as f64;
                            var19 += var10 * var21 as f64;
                            var14 -= var21 * 0.75;
                        }
                    }
                }
            }
        }

        // src: Explosion.java:77-105
        self.explosion_size *= 2.0;
        let size = self.explosion_size as f64;
        let var3 = floor_double(self.explosion_x - size - 1.0);
        let var4 = floor_double(self.explosion_x + size + 1.0);
        let var5 = floor_double(self.explosion_y - size - 1.0);
        let var29 = floor_double(self.explosion_y + size + 1.0);
        let var7 = floor_double(self.explosion_z - size - 1.0);
        let var30 = floor_double(self.explosion_z + size + 1.0);
        let bb = Aabb::new(
            var3 as f64,
            var5 as f64,
            var7 as f64,
            var4 as f64,
            var29 as f64,
            var30 as f64,
        );
        let var9 = entities_within_aabb_excluding(world, self.exploder, &bb, in_hand.as_deref());
        let var31 = Vec3D::create_vector(self.explosion_x, self.explosion_y, self.explosion_z);
        let (ex, ey, ez) = (self.explosion_x, self.explosion_y, self.explosion_z);
        let hit = |world: &mut World, var33: &mut Entity| {
            let var13 = var33.base.get_distance(ex, ey, ez) / size;
            if var13 <= 1.0 {
                let mut var15 = var33.base.pos_x - ex;
                let mut var17 = var33.base.pos_y - ey;
                let mut var19 = var33.base.pos_z - ez;
                let var39 = sqrt_double(var15 * var15 + var17 * var17 + var19 * var19) as f64;
                var15 /= var39;
                var17 /= var39;
                var19 /= var39;
                let bb = var33.base.bounding_box;
                let var40 = world.get_block_density(&var31, &bb) as f64;
                let var41 = (1.0 - var13) * var40;
                let damage = ((var41 * var41 + var41) / 2.0 * 8.0 * size + 1.0) as i32;
                entity_dispatch::attack_entity_from(var33, world, exploder, damage);
                var33.base.motion_x += var15 * var41;
                var33.base.motion_y += var17 * var41;
                var33.base.motion_z += var19 * var41;
            }
        };
        for id in var9 {
            match in_hand.as_deref_mut() {
                Some(h) if h.base.entity_id == id => hit(world, h),
                _ => {
                    world.with_entity_mut(id, hit);
                }
            }
        }
        self.explosion_size = var1;

        // src: Explosion.java:108-122
        // vanilla bug: the fire goes only on positions in destroyedBlockPositions, and
        // doExplosionB (run next by newExplosion) clears every one of them, fire
        // included, so a flaming explosion leaves no fire (only the fires' scheduled
        // ticks; tests/golden/tick/explosion-fire).
        if self.is_flaming {
            let var32: Vec<ChunkPosition> =
                self.destroyed_block_positions.iter().copied().collect();
            for var35 in var32.iter().rev() {
                let (var36, var37, var16) = (var35.x, var35.y, var35.z);
                let var38 = world.get_block_id(var36, var37, var16);
                let var18 = world.get_block_id(var36, var37 - 1, var16);
                if var38 == 0
                    && block::opaque_cube_lookup(var18)
                    && self.explosion_rng.next_int_bounded(3) == 0
                {
                    world.set_block_with_notify(var36, var37, var16, FIRE_ID);
                }
            }
        }
    }

    /// `doExplosionB(particles)`: the sound, then each destroyed position in
    /// reverse set order: with `particles`, the explode/smoke particles
    /// (which draw `World.rand` even on a server-less world: they are part
    /// of the simulation's random stream); for a block, its drops with
    /// chance 0.3, air with notification, `onBlockDestroyedByExplosion`.
    // src: Explosion.java:126-164
    pub fn do_explosion_b(&mut self, world: &mut World, var1: bool) {
        let pitch = (1.0f32 + (world.rand.next_float() - world.rand.next_float()) * 0.2) * 0.7;
        world.play_sound_effect(
            self.explosion_x,
            self.explosion_y,
            self.explosion_z,
            "random.explode",
            4.0,
            pitch,
        );
        let var2: Vec<ChunkPosition> = self.destroyed_block_positions.iter().copied().collect();
        for var4 in var2.iter().rev() {
            let (var5, var6, var7) = (var4.x, var4.y, var4.z);
            let var8 = world.get_block_id(var5, var6, var7);
            if var1 {
                // src: Explosion.java:137-155
                let var9 = (var5 as f32 + world.rand.next_float()) as f64;
                let var11 = (var6 as f32 + world.rand.next_float()) as f64;
                let var13 = (var7 as f32 + world.rand.next_float()) as f64;
                let mut var15 = var9 - self.explosion_x;
                let mut var17 = var11 - self.explosion_y;
                let mut var19 = var13 - self.explosion_z;
                let var21 = sqrt_double(var15 * var15 + var17 * var17 + var19 * var19) as f64;
                var15 /= var21;
                var17 /= var21;
                var19 /= var21;
                let mut var23 = 0.5 / (var21 / self.explosion_size as f64 + 0.1);
                var23 *= (world.rand.next_float() * world.rand.next_float() + 0.3) as f64;
                var15 *= var23;
                var17 *= var23;
                var19 *= var23;
                world.spawn_particle(
                    "explode",
                    (var9 + self.explosion_x * 1.0) / 2.0,
                    (var11 + self.explosion_y * 1.0) / 2.0,
                    (var13 + self.explosion_z * 1.0) / 2.0,
                    var15,
                    var17,
                    var19,
                );
                world.spawn_particle("smoke", var9, var11, var13, var15, var17, var19);
            }
            if var8 > 0 {
                // src: Explosion.java:157-161
                let meta = world.get_block_metadata(var5, var6, var7);
                block_dispatch::drop_block_as_item_with_chance(
                    world, var8, var5, var6, var7, meta, 0.3,
                );
                world.set_block_with_notify(var5, var6, var7, 0);
                block_dispatch::on_block_destroyed_by_explosion(world, var8, var5, var6, var7);
            }
        }
    }
}

/// `World.getEntitiesWithinAABBExcludingEntity(exploder, box)` with an
/// entity the caller holds out of its slot (`in_hand`) tested where the
/// scan reaches its id, in the chunk/section/list order of
/// [`World::get_entities_within_aabb_excluding_entity`] (which skips
/// taken-out entities). Any other taken-out entity is recorded as there.
// src: World.java:2013-2029, Chunk.java:488-510
fn entities_within_aabb_excluding(
    world: &World,
    exclude: Option<i32>,
    bb: &Aabb,
    in_hand: Option<&Entity>,
) -> Vec<i32> {
    let Some(h) = in_hand else {
        return world.get_entities_within_aabb_excluding_entity(exclude, bb);
    };
    let hid = h.base.entity_id;
    let mut out = Vec::new();
    let var3 = floor_double((bb.min_x - 2.0) / 16.0);
    let var4 = floor_double((bb.max_x + 2.0) / 16.0);
    let var5 = floor_double((bb.min_z - 2.0) / 16.0);
    let var6 = floor_double((bb.max_z + 2.0) / 16.0);
    for var7 in var3..=var4 {
        for var8 in var5..=var6 {
            let Some(c) = world.loaded_chunk(var7, var8) else {
                continue;
            };
            for section in c.entity_sections(bb.min_y, bb.max_y) {
                for &id in &c.entities[section] {
                    if Some(id) == exclude {
                        continue;
                    }
                    let o = if id == hid { Some(h) } else { world.entity(id) };
                    match o {
                        Some(o) => {
                            if o.base.bounding_box.intersects_with(bb) {
                                out.push(id);
                            }
                        }
                        None => world.note_unimplemented(
                            "World.getEntitiesWithinAABBExcludingEntity over a taken-out entity",
                        ),
                    }
                }
            }
        }
    }
    out
}

impl World {
    /// `createExplosion(exploder, x, y, z, size)`: a non-flaming
    /// [`World::new_explosion`].
    // src: World.java:1518-1520
    pub fn create_explosion(
        &mut self,
        exploder: Option<&Entity>,
        x: f64,
        y: f64,
        z: f64,
        size: f32,
    ) -> Explosion {
        self.new_explosion(exploder, x, y, z, size, false)
    }

    /// `newExplosion(exploder, x, y, z, size, flaming)`: builds the
    /// explosion and runs both passes (`doExplosionB(true)`: with particles).
    // src: World.java:1522-1528
    pub fn new_explosion(
        &mut self,
        exploder: Option<&Entity>,
        x: f64,
        y: f64,
        z: f64,
        size: f32,
        flaming: bool,
    ) -> Explosion {
        let mut var10 = Explosion::new(self, exploder, x, y, z, size);
        var10.is_flaming = flaming;
        var10.do_explosion_a(self, exploder, None);
        var10.do_explosion_b(self, true);
        var10
    }
}

#[cfg(test)]
mod tests {
    use super::Explosion;
    use crate::entity::item::new_entity_item;
    use crate::item::ItemStack;
    use crate::save::NoSave;
    use crate::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
    use crate::world::provider::{Dimension, WorldProvider};
    use crate::world::{World, WorldEnvironment};

    /// A world with three dropped items around (8.5, 65, 8.5), and their ids.
    fn world() -> (World, Vec<i32>) {
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
        for dz in -1..=1 {
            for dx in -1..=1 {
                w.get_chunk_from_chunk_coords(dx, dz);
            }
        }
        // Stone under the blast so the rays break blocks and draw World.rand.
        for x in 4..13 {
            for z in 4..13 {
                w.set_block(x, 63, z, 1);
            }
        }
        let mut ids = Vec::new();
        for (x, y, z) in [(9.5, 65.0, 8.5), (8.5, 65.5, 11.0), (2.5, 66.0, 8.5)] {
            let e = new_entity_item(&mut w, x, y, z, ItemStack::new(4, 1, 0));
            ids.push(e.base.entity_id);
            assert!(w.spawn_entity_in_world(e));
        }
        (w, ids)
    }

    fn state(w: &World, ids: &[i32]) -> String {
        let es: Vec<String> = ids
            .iter()
            .map(|&id| format!("{:?}", w.entity(id).map(|e| &e.base)))
            .collect();
        format!("{es:?} {:?}", w.rand)
    }

    /// `doExplosionA` with an entity held out of its slot (`in_hand`, the
    /// exploding TNT or fireball of Phase 2) damages and pushes it exactly as
    /// when it sits in the world, in the scan's order: the world, the World.rand
    /// state and every entity end up identical.
    #[test]
    fn in_hand_entity_is_hit_as_if_in_the_world() {
        let (mut a, ids) = world();
        let mut ex = Explosion::new(&a, None, 8.5, 65.0, 8.5, 4.0);
        ex.do_explosion_a(&mut a, None, None);
        let want = state(&a, &ids);
        let want_set: Vec<_> = ex.destroyed_block_positions.iter().copied().collect();

        for &held in &ids {
            let (mut b, ids_b) = world();
            assert_eq!(ids, ids_b);
            let mut ex = Explosion::new(&b, None, 8.5, 65.0, 8.5, 4.0);
            b.with_entity_mut(held, |w, e| ex.do_explosion_a(w, None, Some(e)))
                .unwrap();
            assert!(b.unimplemented_hits.borrow().is_empty(), "held {held}");
            assert_eq!(state(&b, &ids), want, "held {held}");
            let got_set: Vec<_> = ex.destroyed_block_positions.iter().copied().collect();
            assert_eq!(got_set, want_set);
        }
        // The two near items were hit (pushed away from the centre).
        let e0 = a.entity(ids[0]).unwrap();
        assert!(e0.base.motion_x > 0.0);
    }
}
