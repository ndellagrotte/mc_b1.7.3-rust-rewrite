//! The block dispatch (`mc_sim::block::dispatch`) covers every override of
//! `Block.java` in the vanilla tree, so the block-behaviour tasks cannot leave
//! a silent gap (WP-1.6 prep; module docs of `dispatch`, "Coverage").
//!
//! Input: `tests/data/block_overrides.txt`, generated from the vanilla tree by
//! `tests/data/gen_block_overrides.sh` (the `Block` methods, the class
//! hierarchy, and every `(class, method)` override). For each block id and
//! each `Block` method that has overrides:
//!
//! - a data method ([`dispatch::DATA_METHODS`]) is transcribed in
//!   `mc_sim::block` and pinned per id by `tests/golden/block/block-table`;
//! - every other method has a probe below that calls its entry point. The
//!   override the id's class inherits (the nearest class at or above it that
//!   declares the method) decides what the call must record in
//!   `World::unimplemented_hits` under `*.method`: nothing when the class
//!   inherits `Block`'s body or the pair is in [`dispatch::TRANSCRIBED`],
//!   otherwise exactly `"Class.method"`.

use std::collections::{BTreeMap, BTreeSet};

use mc_core::java_random::JavaRandom;
use mc_core::{Aabb, Vec3D};
use mc_sim::block::{self, dispatch as d};
use mc_sim::entity::Entity;
use mc_sim::entity::player::new_entity_player_sp;
use mc_sim::item::ItemStack;
use mc_sim::save::NoSave;
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::{World, WorldEnvironment};

const OVERRIDES: &str = include_str!("data/block_overrides.txt");

/// Overrides that no instantiated block reaches, not even through `super`:
/// dead code in vanilla, so nothing can record them. `BlockFluid.updateTick`
/// (BlockFluid.java:201) is overridden by both `BlockFlowing` and
/// `BlockStationary`, neither of which calls `super.updateTick`;
/// `BlockLeavesBase.isOpaqueCube` (BlockLeavesBase.java:11) is overridden by
/// its only subclass `BlockLeaves` without `super`.
const UNREACHABLE: &[(&str, &str)] = &[
    ("BlockFluid", "updateTick"),
    ("BlockLeavesBase", "isOpaqueCube"),
];

/// The parsed `block_overrides.txt`.
struct OverrideSet {
    /// `method` rows: name → Block.java line.
    methods: BTreeMap<String, u32>,
    /// `extends` rows: class → parent.
    parent: BTreeMap<String, String>,
    /// `override` rows.
    overrides: BTreeSet<(String, String)>,
}

impl OverrideSet {
    fn parse() -> Self {
        let mut s = OverrideSet {
            methods: BTreeMap::new(),
            parent: BTreeMap::new(),
            overrides: BTreeSet::new(),
        };
        for line in OVERRIDES.lines() {
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            let w: Vec<&str> = line.split(' ').collect();
            match w[0] {
                "method" => {
                    s.methods
                        .insert(w[1].into(), w[2].parse().expect("line number"));
                }
                "extends" => {
                    s.parent.insert(w[1].into(), w[2].into());
                }
                "override" => {
                    s.overrides.insert((w[1].into(), w[2].into()));
                }
                other => panic!("block_overrides.txt: unknown row kind {other:?} in {line:?}"),
            }
        }
        s
    }

    /// The class at or above `class` that declares `method`, below `Block`.
    fn defining_class(&self, class: &str, method: &str) -> Option<String> {
        let mut c = class.to_string();
        while c != "Block" {
            if self.overrides.contains(&(c.clone(), method.to_string())) {
                return Some(c);
            }
            c = self
                .parent
                .get(&c)
                .unwrap_or_else(|| panic!("{c} is not a Block subclass in block_overrides.txt"))
                .clone();
        }
        None
    }

    /// Methods with at least one override.
    fn overridden_methods(&self) -> BTreeSet<String> {
        self.overrides.iter().map(|(_, m)| m.clone()).collect()
    }
}

/// The coordinates every probe passes.
const X: i32 = 0;
const Y: i32 = 61;
const Z: i32 = 0;

type Probe = fn(&mut World, i32, &mut Entity);

/// One call per non-data entry point, keyed by the Java method name.
const PROBES: &[(&str, Probe)] = &[
    ("blockActivated", |w, id, p| {
        d::block_activated(w, id, X, Y, Z, p);
    }),
    ("canBlockStay", |w, id, _| {
        d::can_block_stay(w, id, X, Y, Z);
    }),
    ("canCollideCheck", |_, id, _| {
        d::can_collide_check(id, 0, true);
        d::can_collide_check(id, 1, false);
    }),
    ("canPlaceBlockAt", |w, id, _| {
        d::can_place_block_at(w, id, X, Y, Z);
    }),
    ("canPlaceBlockOnSide", |w, id, _| {
        d::can_place_block_on_side(w, id, X, Y, Z, 1);
    }),
    ("canProvidePower", |w, id, _| {
        d::can_provide_power(w, id);
    }),
    ("collisionRayTrace", |w, id, _| {
        let a = Vec3D::create_vector(0.5, 63.0, 0.5);
        let b = Vec3D::create_vector(0.5, 60.0, 0.5);
        d::collision_ray_trace(w, id, X, Y, Z, &a, &b);
    }),
    ("colorMultiplier", |w, id, _| {
        d::color_multiplier(w, id, X, Y, Z);
    }),
    ("dropBlockAsItemWithChance", |w, id, _| {
        d::drop_block_as_item_with_chance(w, id, X, Y, Z, 0, 1.0);
    }),
    ("getBlockBrightness", |w, id, _| {
        d::get_block_brightness(w, id, X, Y, Z);
    }),
    ("getBlockTexture", |w, id, _| {
        d::get_block_texture(w, id, X, Y, Z, 1);
    }),
    ("getCollidingBoundingBoxes", |w, id, _| {
        let mut list = Vec::new();
        let mask = Aabb::new(-1.0, 60.0, -1.0, 2.0, 63.0, 2.0);
        d::get_colliding_bounding_boxes(w, id, X, Y, Z, &mask, &mut list);
    }),
    ("getCollisionBoundingBoxFromPool", |w, id, _| {
        d::get_collision_bounding_box_from_pool(w, id, X, Y, Z);
    }),
    ("getExplosionResistance", |w, id, _| {
        d::get_explosion_resistance(w, id, None);
    }),
    ("getIsBlockSolid", |w, id, _| {
        d::get_is_block_solid(w, id, X, Y, Z, 1);
    }),
    ("getRenderColor", |w, id, _| {
        d::get_render_color(w, id, 0);
    }),
    ("getSelectedBoundingBoxFromPool", |w, id, _| {
        d::get_selected_bounding_box_from_pool(w, id, X, Y, Z);
    }),
    ("harvestBlock", |w, id, p| {
        d::harvest_block(w, id, X, Y, Z, p, 0)
    }),
    ("isIndirectlyPoweringTo", |w, id, _| {
        d::is_indirectly_powering_to(w, id, X, Y, Z, 1);
    }),
    ("isPoweringTo", |w, id, _| {
        d::is_powering_to(w, id, X, Y, Z, 1);
    }),
    ("onBlockAdded", |w, id, _| d::on_block_added(w, id, X, Y, Z)),
    ("onBlockClicked", |w, id, p| {
        d::on_block_clicked(w, id, X, Y, Z, p)
    }),
    ("onBlockDestroyedByExplosion", |w, id, _| {
        d::on_block_destroyed_by_explosion(w, id, X, Y, Z)
    }),
    ("onBlockDestroyedByPlayer", |w, id, _| {
        d::on_block_destroyed_by_player(w, id, X, Y, Z, 0)
    }),
    ("onBlockPlaced", |w, id, _| {
        d::on_block_placed(w, id, X, Y, Z, 1)
    }),
    ("onBlockPlacedBy", |w, id, p| {
        d::on_block_placed_by(w, id, X, Y, Z, p)
    }),
    ("onBlockRemoval", |w, id, _| {
        d::on_block_removal(w, id, X, Y, Z)
    }),
    ("onEntityCollidedWithBlock", |w, id, p| {
        d::on_entity_collided_with_block(w, id, X, Y, Z, p)
    }),
    ("onEntityWalking", |w, id, p| {
        d::on_entity_walking(w, id, X, Y, Z, p)
    }),
    ("onNeighborBlockChange", |w, id, _| {
        d::on_neighbor_block_change(w, id, X, Y, Z, 1)
    }),
    ("playBlock", |w, id, _| d::play_block(w, id, X, Y, Z, 0, 0)),
    ("randomDisplayTick", |w, id, _| {
        let mut r = JavaRandom::new(1);
        d::random_display_tick(w, id, X, Y, Z, &mut r);
    }),
    ("setBlockBoundsBasedOnState", |w, id, _| {
        d::set_block_bounds_based_on_state(w, id, X, Y, Z)
    }),
    ("setBlockBoundsForItemRender", |w, id, _| {
        d::set_block_bounds_for_item_render(w, id)
    }),
    ("shouldSideBeRendered", |w, id, _| {
        d::should_side_be_rendered(w, id, X, Y + 1, Z, 1);
    }),
    ("updateTick", |w, id, _| {
        d::update_tick(w, id, X, Y, Z, d::RandomSource::WorldRand)
    }),
    ("velocityToAddToEntity", |w, id, p| {
        let mut v = Vec3D::create_vector(0.0, 0.0, 0.0);
        d::velocity_to_add_to_entity(w, id, X, Y, Z, p, &mut v);
    }),
];

fn empty_world() -> World {
    let mut w = World::new(
        Box::new(NoSave),
        "harness",
        WorldProvider::new(Dimension::Surface),
        1,
        ChunkGenerator::Empty(EmptyChunkProvider),
        WorldEnvironment {
            rand_seed: 0,
            field_9437_g: 0,
            lock_timestamp: 0,
        },
    );
    w.make_deterministic(1);
    w
}

fn transcribed() -> BTreeSet<(String, String)> {
    d::TRANSCRIBED
        .iter()
        .flat_map(|(c, ms)| ms.iter().map(move |m| (c.to_string(), m.to_string())))
        .collect()
}

/// The registries name real methods and overrides, and do not overlap.
#[test]
fn registries_match_the_vanilla_override_set() {
    let s = OverrideSet::parse();
    assert!(
        s.methods.len() > 60 && s.overrides.len() > 500,
        "block_overrides.txt looks truncated"
    );
    for m in d::DATA_METHODS {
        assert!(
            s.methods.contains_key(*m),
            "DATA_METHODS: {m} is not a Block method"
        );
    }
    for (m, _) in PROBES {
        assert!(
            s.methods.contains_key(*m),
            "probe {m} is not a Block method"
        );
        assert!(
            !d::DATA_METHODS.contains(m),
            "{m} is both probed and a data method"
        );
    }
    for (c, m) in transcribed() {
        assert!(
            s.overrides.contains(&(c.clone(), m.clone())),
            "TRANSCRIBED lists {c}.{m}, which is not an override in the vanilla tree"
        );
        assert!(
            !d::DATA_METHODS.contains(&m.as_str()),
            "TRANSCRIBED lists {c}.{m}, but {m} is a data method"
        );
    }
    // Every overridden method is either data or has an entry point probed here.
    let probed: BTreeSet<&str> = PROBES.iter().map(|(m, _)| *m).collect();
    for m in s.overridden_methods() {
        assert!(
            d::DATA_METHODS.contains(&m.as_str()) || probed.contains(m.as_str()),
            "Block.{m} (Block.java:{}) is overridden but has no dispatch entry point probed in \
             tests/block_dispatch_coverage.rs",
            s.methods[&m]
        );
    }
}

/// Every `(class, method)` override is reached by some instantiated block, or
/// is one of the known dead overrides.
#[test]
fn every_override_is_reachable_or_known_dead() {
    let s = OverrideSet::parse();
    let mut reached = BTreeSet::new();
    for id in 0..256 {
        let Some(b) = block::block(id) else { continue };
        for m in s.overridden_methods() {
            if let Some(c) = s.defining_class(b.kind.java_class(), &m) {
                reached.insert((c, m));
            }
        }
    }
    let unreached: BTreeSet<(String, String)> = s.overrides.difference(&reached).cloned().collect();
    let known: BTreeSet<(String, String)> = UNREACHABLE
        .iter()
        .map(|(c, m)| (c.to_string(), m.to_string()))
        .collect();
    assert_eq!(
        unreached, known,
        "overrides no block reaches (vs UNREACHABLE)"
    );
}

/// The core check: every entry point, for every block id, records exactly the
/// override the id's class inherits, unless that override is transcribed.
#[test]
fn every_entry_point_records_exactly_the_inherited_override() {
    let s = OverrideSet::parse();
    let done = transcribed();
    let mut world = empty_world();
    let mut player = new_entity_player_sp(&mut world);
    // A held stack, so the player paths that read it have something to read.
    if let Some(p) = player.kind.player_mut() {
        p.inventory.main_inventory[0] = Some(ItemStack::new(1, 1, 0));
    }
    let mut failures = Vec::new();
    let mut recorded_pairs = BTreeSet::new();
    for (method, probe) in PROBES {
        let suffix = format!(".{method}");
        for id in 0..256 {
            let Some(b) = block::block(id) else { continue };
            let class = b.kind.java_class();
            world.unimplemented_hits.borrow_mut().clear();
            probe(&mut world, id, &mut player);
            let hits: BTreeSet<String> = world
                .unimplemented_hits
                .borrow()
                .keys()
                .filter(|k| k.ends_with(&suffix))
                .map(|k| k.to_string())
                .collect();
            let expected: BTreeSet<String> = match s.defining_class(class, method) {
                Some(c) if !done.contains(&(c.clone(), method.to_string())) => {
                    recorded_pairs.insert((c.clone(), method.to_string()));
                    [format!("{c}{suffix}")].into()
                }
                _ => BTreeSet::new(),
            };
            if hits != expected {
                failures.push(format!(
                    "id {id} ({class}).{method}: recorded {hits:?}, expected {expected:?}"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} dispatch gaps:\n{}",
        failures.len(),
        failures.join("\n")
    );
    // The remaining work, for the record (`cargo test -- --nocapture`).
    eprintln!(
        "{} overrides recorded as not transcribed, {} transcribed in dispatch, {} data methods",
        recorded_pairs.len(),
        done.len(),
        d::DATA_METHODS.len()
    );
}
