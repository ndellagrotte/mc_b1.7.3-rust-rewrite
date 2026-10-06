//! The entity dispatch (`mc_sim::entity::dispatch`) covers every override of
//! `Entity.java` in the vanilla tree (WP-2.1; module docs of `dispatch`,
//! "Coverage"), as `block_dispatch_coverage.rs` does for blocks.
//!
//! Input: `tests/data/entity_overrides.txt`, generated from the vanilla tree
//! by `tests/data/gen_entity_overrides.sh` (the `Entity` methods, the class
//! hierarchy of the in-scope subclasses, and every `(class, method)`
//! override). For each entity kind and each `Entity` method that has
//! overrides:
//!
//! - a method in [`dispatch::NOT_DISPATCHED`] has a stated reason and no
//!   entry point;
//! - every other method has a probe below that calls its entry point. The
//!   override the kind's class inherits (the nearest class at or above it
//!   that declares the method) decides what the call must record in
//!   `World::unimplemented_hits` under `*.method`: nothing of its own when
//!   the kind inherits `Entity`'s body or the pair is in
//!   [`dispatch::TRANSCRIBED`], otherwise `"Class.method"`; in both cases it
//!   may also record ancestors of that class that declare the method (an
//!   arm reaches them through `super`), and nothing else. The methods
//!   `EntityLiving` adds are checked by `living_dispatch_coverage.rs`.

use std::collections::{BTreeMap, BTreeSet};

use mc_core::Vec3D;
use mc_sim::entity::item::ItemData;
use mc_sim::entity::player::new_entity_player_sp;
use mc_sim::entity::{
    AnimalData, ArrowData, BoatData, ChickenData, CowData, CreatureData, CreeperData, EggData,
    Entity, EntityBase, EntityKindData, FallingSandData, FireballData, FishData, FlyingData,
    GhastData, GiantZombieData, LightningBoltData, LivingData, MinecartData, MobData, PaintingData,
    PigData, PigZombieData, SheepData, SkeletonData, SlimeData, SnowballData, SpiderData,
    SquidData, TntPrimedData, WaterMobData, WeatherEffectData, WolfData, ZombieData, dispatch as d,
};
use mc_sim::item::ItemStack;
use mc_sim::save::NoSave;
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::{World, WorldEnvironment};

const OVERRIDES: &str = include_str!("data/entity_overrides.txt");

/// Overrides that no kind inherits as its own (every subclass below the
/// class overrides the method again), but that are still reached through
/// `super` calls; none of them is dispatched, so the list is empty today.
const SHADOWED: &[(&str, &str)] = &[];

/// The parsed `entity_overrides.txt`.
struct OverrideSet {
    /// `method` rows: name → Entity.java line.
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
                other => panic!("entity_overrides.txt: unknown row kind {other:?} in {line:?}"),
            }
        }
        s
    }

    /// `class` and its superclasses below `Entity`, bottom-up.
    fn chain(&self, class: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut c = class.to_string();
        while c != "Entity" {
            out.push(c.clone());
            c = self
                .parent
                .get(&c)
                .unwrap_or_else(|| panic!("{c} is not an Entity subclass in entity_overrides.txt"))
                .clone();
        }
        out
    }

    /// The class at or above `class` that declares `method`, below `Entity`.
    fn defining_class(&self, class: &str, method: &str) -> Option<String> {
        self.chain(class)
            .into_iter()
            .find(|c| self.overrides.contains(&(c.clone(), method.to_string())))
    }

    /// Methods with at least one override.
    fn overridden_methods(&self) -> BTreeSet<String> {
        self.overrides.iter().map(|(_, m)| m.clone()).collect()
    }
}

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

/// Compile-time completeness: a new `EntityKindData` variant must be added to
/// [`all_kinds`] too (this match stops compiling until it is).
fn kind_index(k: &EntityKindData) -> usize {
    use EntityKindData as K;
    match k {
        K::Item(..) => 0,
        K::Arrow(..) => 1,
        K::Snowball(..) => 2,
        K::Egg(..) => 3,
        K::Fireball(..) => 4,
        K::Fish(..) => 5,
        K::Painting(..) => 6,
        K::FallingSand(..) => 7,
        K::TntPrimed(..) => 8,
        K::Minecart(..) => 9,
        K::Boat(..) => 10,
        K::LightningBolt(..) => 11,
        K::Mob(..) => 12,
        K::Creeper(..) => 13,
        K::Skeleton(..) => 14,
        K::Spider(..) => 15,
        K::Zombie(..) => 16,
        K::PigZombie(..) => 17,
        K::GiantZombie(..) => 18,
        K::Slime(..) => 19,
        K::Ghast(..) => 20,
        K::Pig(..) => 21,
        K::Sheep(..) => 22,
        K::Cow(..) => 23,
        K::Chicken(..) => 24,
        K::Wolf(..) => 25,
        K::Squid(..) => 26,
        K::PlayerSp(..) => 27,
    }
}
const KIND_COUNT: usize = 28;

/// One entity of every kind, at (0.5, 70, 0.5) with no motion. Kinds
/// without a transcribed constructor are built from `Entity(World)`, the
/// `EntityLiving` initialisers where they apply, and default data: the probes
/// only need the kind to pick the dispatch arm.
fn all_kinds(w: &mut World) -> Vec<Entity> {
    use EntityKindData as K;
    let mut out = Vec::new();
    let mut push = |w: &mut World, make: &dyn Fn(&mut World, &mut EntityBase) -> EntityKindData| {
        let mut b = EntityBase::new(w);
        let kind = make(w, &mut b);
        b.set_position(0.5, 70.0, 0.5);
        out.push(Entity { base: b, kind });
    };
    push(w, &|_, _| {
        K::Item(ItemData {
            item: Some(ItemStack::new(4, 1, 0)),
            field_803_e: 0,
            age: 0,
            delay_before_can_pickup: 0,
            health: 5,
            hover_start: 0.0,
        })
    });
    push(w, &|_, _| K::Arrow(ArrowData::default()));
    push(w, &|_, _| K::Snowball(SnowballData::default()));
    push(w, &|_, _| K::Egg(EggData::default()));
    push(w, &|_, _| K::Fireball(FireballData::default()));
    push(w, &|_, _| K::Fish(FishData::default()));
    push(w, &|_, _| K::Painting(PaintingData::default()));
    push(w, &|_, _| {
        K::FallingSand(FallingSandData {
            block_id: 12,
            fall_time: 0,
        })
    });
    push(w, &|_, _| K::TntPrimed(TntPrimedData::default()));
    push(w, &|_, _| K::Minecart(MinecartData::default()));
    push(w, &|_, _| K::Boat(BoatData::default()));
    push(w, &|_, _| {
        K::LightningBolt(WeatherEffectData::default(), LightningBoltData::default())
    });
    let c = CreatureData::default;
    push(w, &|w, b| {
        K::Mob(LivingData::new(w, b), c(), MobData::default())
    });
    push(w, &|w, b| {
        K::Creeper(
            LivingData::new(w, b),
            c(),
            MobData::default(),
            CreeperData::default(),
        )
    });
    push(w, &|w, b| {
        K::Skeleton(
            LivingData::new(w, b),
            c(),
            MobData::default(),
            SkeletonData::default(),
        )
    });
    push(w, &|w, b| {
        K::Spider(
            LivingData::new(w, b),
            c(),
            MobData::default(),
            SpiderData::default(),
        )
    });
    push(w, &|w, b| {
        K::Zombie(
            LivingData::new(w, b),
            c(),
            MobData::default(),
            ZombieData::default(),
        )
    });
    push(w, &|w, b| {
        K::PigZombie(
            LivingData::new(w, b),
            c(),
            MobData::default(),
            ZombieData::default(),
            PigZombieData::default(),
        )
    });
    push(w, &|w, b| {
        K::GiantZombie(
            LivingData::new(w, b),
            c(),
            MobData::default(),
            GiantZombieData::default(),
        )
    });
    push(w, &|w, b| {
        K::Slime(LivingData::new(w, b), SlimeData::default())
    });
    push(w, &|w, b| {
        K::Ghast(
            LivingData::new(w, b),
            FlyingData::default(),
            GhastData::default(),
        )
    });
    let a = AnimalData::default;
    push(w, &|w, b| {
        // EntityPig.entityInit (EntityPig.java:10-12): the saddle byte.
        b.data_watcher
            .add_object(16, mc_sim::entity::data_watcher::WatchValue::Byte(0));
        K::Pig(LivingData::new(w, b), c(), a(), PigData::default())
    });
    push(w, &|w, b| {
        K::Sheep(LivingData::new(w, b), c(), a(), SheepData::default())
    });
    push(w, &|w, b| {
        K::Cow(LivingData::new(w, b), c(), a(), CowData::default())
    });
    push(w, &|w, b| {
        K::Chicken(LivingData::new(w, b), c(), a(), ChickenData::default())
    });
    push(w, &|w, b| {
        K::Wolf(LivingData::new(w, b), c(), a(), WolfData::default())
    });
    push(w, &|w, b| {
        K::Squid(
            LivingData::new(w, b),
            c(),
            WaterMobData::default(),
            SquidData::default(),
        )
    });
    let mut p = new_entity_player_sp(w);
    p.base.set_position(0.5, 70.0, 0.5);
    out.push(p);
    assert_eq!(out.len(), KIND_COUNT);
    let idx: BTreeSet<usize> = out.iter().map(|e| kind_index(&e.kind)).collect();
    assert_eq!(idx.len(), KIND_COUNT, "all_kinds builds every kind once");
    out
}

/// `(world, entity, other)`: `other` is a player (the `player` / `other`
/// parameter of the methods that take one).
type Probe = fn(&mut World, &mut Entity, &mut Entity);

/// One call per entry point, keyed by the Java method name.
const PROBES: &[(&str, Probe)] = &[
    ("addToPlayerScore", |w, e, o| {
        d::add_to_player_score(e, w, o, 5)
    }),
    ("addVelocity", |w, e, _| {
        d::add_velocity(e, w, 0.1, 0.0, 0.1)
    }),
    ("applyEntityCollision", |w, e, o| {
        d::apply_entity_collision(e, w, o)
    }),
    ("attackEntityFrom", |w, e, _| {
        d::attack_entity_from(e, w, None, 1);
    }),
    ("canBeCollidedWith", |w, e, _| {
        d::can_be_collided_with(e, w);
    }),
    ("canBePushed", |w, e, _| {
        d::can_be_pushed(e, w);
    }),
    ("canTriggerWalking", |_, e, _| {
        d::can_trigger_walking(e);
    }),
    ("dealFireDamage", |w, e, _| d::deal_fire_damage(e, w, 1)),
    ("fall", |w, e, _| d::fall(e, w, 5.0)),
    ("getBoundingBox", |w, e, _| {
        d::get_bounding_box(e, w);
    }),
    ("getCollisionBorderSize", |w, e, _| {
        d::get_collision_border_size(e, w);
    }),
    ("getCollisionBox", |w, e, o| {
        d::get_collision_box(e, o, w);
    }),
    ("getEntityTexture", |w, e, _| {
        d::get_entity_texture(e, w);
    }),
    ("getEyeHeight", |w, e, _| {
        d::get_eye_height(e, w);
    }),
    ("getLookVec", |w, e, _| {
        d::get_look_vec(e, w);
    }),
    ("getMountedYOffset", |w, e, _| {
        d::get_mounted_y_offset(e, w);
    }),
    ("getShadowSize", |w, e, _| {
        d::get_shadow_size(e, w);
    }),
    ("getYOffset", |w, e, _| {
        d::get_y_offset(e, w);
    }),
    ("handleHealthUpdate", |w, e, _| {
        d::handle_health_update(e, w, 2)
    }),
    ("handleWaterMovement", |w, e, _| {
        d::handle_water_movement(e, w);
    }),
    ("interact", |w, e, o| {
        d::interact(e, w, o);
    }),
    ("isEntityAlive", |_, e, _| {
        d::is_entity_alive(e);
    }),
    ("isEntityInsideOpaqueBlock", |w, e, _| {
        d::is_entity_inside_opaque_block(e, w);
    }),
    ("isInRangeToRenderDist", |w, e, _| {
        d::is_in_range_to_render_dist(e, w, 100.0);
    }),
    ("isInRangeToRenderVec3D", |w, e, _| {
        d::is_in_range_to_render_vec3d(e, w, &Vec3D::create_vector(0.0, 64.0, 0.0));
    }),
    ("isInWater", |w, e, _| {
        d::is_in_water(e, w);
    }),
    ("isSneaking", |w, e, _| {
        d::is_sneaking(e, w);
    }),
    ("kill", |w, e, _| d::kill(e, w)),
    ("moveEntity", |w, e, _| d::move_entity(e, w, 0.1, -0.1, 0.1)),
    ("onCollideWithPlayer", |w, e, o| {
        d::on_collide_with_player(e, w, o)
    }),
    ("onEntityUpdate", |w, e, _| d::on_entity_update(e, w)),
    ("onKillEntity", |w, e, o| d::on_kill_entity(e, w, o)),
    ("onStruckByLightning", |w, e, o| {
        d::on_struck_by_lightning(e, w, o)
    }),
    ("onUpdate", |w, e, _| d::on_update(e, w)),
    ("performHurtAnimation", |w, e, _| {
        d::perform_hurt_animation(e, w)
    }),
    ("preparePlayerToSpawn", |w, e, _| {
        d::prepare_player_to_spawn(e, w)
    }),
    ("pushOutOfBlocks", |w, e, _| {
        d::push_out_of_blocks(e, w, 0.5, 70.0, 0.5);
    }),
    ("setEntityDead", |w, e, _| d::set_entity_dead(e, w)),
    ("setInPortal", |w, e, _| d::set_in_portal(e, w)),
    ("setPositionAndRotation2", |w, e, _| {
        d::set_position_and_rotation2(e, w, 0.5, 70.0, 0.5, 10.0, 5.0, 3)
    }),
    ("setVelocity", |w, e, _| {
        d::set_velocity(e, w, 0.1, 0.2, 0.3)
    }),
    ("updateRidden", |w, e, _| d::update_ridden(e, w)),
    ("updateRiderPosition", |w, e, o| {
        d::update_rider_position(e, o, w)
    }),
];

fn transcribed() -> BTreeSet<(String, String)> {
    d::TRANSCRIBED
        .iter()
        .flat_map(|(c, ms)| ms.iter().map(move |m| (c.to_string(), m.to_string())))
        .collect()
}

fn not_dispatched() -> BTreeSet<&'static str> {
    d::NOT_DISPATCHED.iter().map(|(m, _)| *m).collect()
}

/// The registries name real methods and overrides, and every overridden
/// method is probed or has a reason not to be dispatched.
#[test]
fn registries_match_the_vanilla_override_set() {
    let s = OverrideSet::parse();
    assert!(
        s.methods.len() > 60 && s.overrides.len() > 150,
        "entity_overrides.txt looks truncated"
    );
    let nd = not_dispatched();
    for (m, why) in d::NOT_DISPATCHED {
        assert!(
            s.methods.contains_key(*m),
            "NOT_DISPATCHED: {m} is not an Entity method"
        );
        assert!(!why.is_empty(), "NOT_DISPATCHED: {m} needs a reason");
    }
    let probed: BTreeSet<&str> = PROBES.iter().map(|(m, _)| *m).collect();
    assert_eq!(probed.len(), PROBES.len(), "a method is probed twice");
    for m in &probed {
        assert!(
            s.methods.contains_key(*m),
            "probe {m} is not an Entity method"
        );
        assert!(!nd.contains(m), "{m} is both probed and NOT_DISPATCHED");
    }
    for (c, m) in transcribed() {
        assert!(
            s.overrides.contains(&(c.clone(), m.clone())),
            "TRANSCRIBED lists {c}.{m}, which is not an override in the vanilla tree"
        );
    }
    for m in s.overridden_methods() {
        assert!(
            nd.contains(m.as_str()) || probed.contains(m.as_str()),
            "Entity.{m} (Entity.java:{}) is overridden but has no dispatch entry point probed in \
             tests/entity_dispatch_coverage.rs (or a NOT_DISPATCHED reason)",
            s.methods[&m]
        );
    }
}

/// Every class with an override is a kind's class or one of its ancestors.
#[test]
fn every_kind_class_is_in_the_hierarchy_and_every_override_is_inherited() {
    let s = OverrideSet::parse();
    let mut w = empty_world();
    let kinds = all_kinds(&mut w);
    let mut reached = BTreeSet::new();
    for e in &kinds {
        let class = e.kind.java_class();
        for m in s.overridden_methods() {
            if not_dispatched().contains(m.as_str()) {
                continue;
            }
            if let Some(c) = s.defining_class(class, &m) {
                reached.insert((c, m));
            }
        }
    }
    let dispatched: BTreeSet<(String, String)> = s
        .overrides
        .iter()
        .filter(|(_, m)| !not_dispatched().contains(m.as_str()))
        .cloned()
        .collect();
    let unreached: BTreeSet<(String, String)> = dispatched.difference(&reached).cloned().collect();
    let known: BTreeSet<(String, String)> = SHADOWED
        .iter()
        .map(|(c, m)| (c.to_string(), m.to_string()))
        .collect();
    assert_eq!(
        unreached, known,
        "overrides no kind inherits as its own (vs SHADOWED)"
    );
}

/// The core check: every entry point, for every kind, records exactly the
/// override the kind's class inherits (plus partial ancestors), unless that
/// override is transcribed.
#[test]
fn every_entry_point_records_exactly_the_inherited_override() {
    let s = OverrideSet::parse();
    let done = transcribed();
    let mut failures = Vec::new();
    let mut recorded_pairs = BTreeSet::new();
    for (method, probe) in PROBES {
        let suffix = format!(".{method}");
        // Fresh entities per method: probes change state (death, motion).
        let mut world = empty_world();
        world.get_chunk_from_chunk_coords(0, 0);
        let kinds = all_kinds(&mut world);
        let mut other = new_entity_player_sp(&mut world);
        other.base.set_position(1.5, 70.0, 0.5);
        for mut e in kinds {
            let class = e.kind.java_class();
            world.unimplemented_hits.borrow_mut().clear();
            probe(&mut world, &mut e, &mut other);
            let hits: BTreeSet<String> = world
                .unimplemented_hits
                .borrow()
                .keys()
                .filter(|k| k.ends_with(&suffix))
                .map(|k| k.to_string())
                .collect();
            let def = s.defining_class(class, method);
            let must: Option<String> = match &def {
                Some(c) if !done.contains(&(c.clone(), method.to_string())) => {
                    recorded_pairs.insert((c.clone(), method.to_string()));
                    Some(format!("{c}{suffix}"))
                }
                _ => None,
            };
            // Ancestors of the defining class that declare the method.
            let allowed: BTreeSet<String> = match &def {
                Some(c) => s
                    .chain(c)
                    .into_iter()
                    .skip(1)
                    .filter(|a| s.overrides.contains(&(a.clone(), method.to_string())))
                    .map(|a| format!("{a}{suffix}"))
                    .collect(),
                None => BTreeSet::new(),
            };
            let ok = must.as_ref().is_none_or(|m| hits.contains(m))
                && hits
                    .iter()
                    .all(|h| Some(h) == must.as_ref() || allowed.contains(h));
            if !ok {
                failures.push(format!(
                    "{class}.{method}: recorded {hits:?}, expected {must:?} (+ optionally {allowed:?})"
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
    eprintln!(
        "{} entity overrides recorded as not transcribed, {} transcribed in dispatch",
        recorded_pairs.len(),
        done.len()
    );
}
