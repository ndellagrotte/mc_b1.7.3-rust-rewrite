//! The `EntityLiving`-level entity dispatch (`mc_sim::entity::dispatch`,
//! the WP-2.2 section) covers every override of the methods
//! `EntityLiving.java` and `EntityCreature.java` declare beyond `Entity.java`
//! (whose own overrides `entity_dispatch_coverage.rs` checks).
//!
//! Input: `tests/data/living_overrides.txt`, generated from the vanilla
//! tree by `tests/data/gen_living_overrides.sh`. For each living kind and
//! each such method that has overrides and that the kind's class has (an
//! `EntityCreature` method only for the kinds below `EntityCreature`):
//!
//! - a method in [`dispatch::LIVING_NOT_DISPATCHED`] has a stated reason and
//!   no entry point;
//! - every other method has a probe below that calls its entry point. The
//!   override the kind's class inherits decides what the call must record in
//!   `World::unimplemented_hits` under `*.method`: nothing of its own when
//!   the kind inherits the declaring class's body or the pair is in
//!   [`dispatch::LIVING_TRANSCRIBED`], otherwise `"Class.method"`; it may
//!   also record the ancestors of that class that override the method, and
//!   nothing else.

use std::collections::{BTreeMap, BTreeSet};

use mc_sim::entity::data_watcher::WatchValue;
use mc_sim::entity::player::new_entity_player_sp;
use mc_sim::entity::{
    AnimalData, ChickenData, CowData, CreatureData, CreeperData, Entity, EntityBase,
    EntityKindData, FlyingData, GhastData, GiantZombieData, LivingData, MobData, PigData,
    PigZombieData, SheepData, SkeletonData, SlimeData, SpiderData, SquidData, WaterMobData,
    WolfData, ZombieData, dispatch as d,
};
use mc_sim::save::NoSave;
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::{World, WorldEnvironment};

const OVERRIDES: &str = include_str!("data/living_overrides.txt");

/// The parsed `living_overrides.txt`.
struct OverrideSet {
    /// `method` rows: name → declaring class (`EntityLiving` or
    /// `EntityCreature`).
    methods: BTreeMap<String, String>,
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
                    s.methods.insert(w[2].into(), w[1].into());
                }
                "extends" => {
                    s.parent.insert(w[1].into(), w[2].into());
                }
                "override" => {
                    s.overrides.insert((w[1].into(), w[2].into()));
                }
                other => panic!("living_overrides.txt: unknown row kind {other:?} in {line:?}"),
            }
        }
        s
    }

    /// `class` and its superclasses below `EntityLiving`, bottom-up.
    fn chain(&self, class: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut c = class.to_string();
        while c != "EntityLiving" {
            out.push(c.clone());
            c = self
                .parent
                .get(&c)
                .unwrap_or_else(|| panic!("{c} is not an EntityLiving subclass"))
                .clone();
        }
        out
    }

    /// Whether `class` has `method` (declared by `EntityLiving`, or by
    /// `EntityCreature` and `class` is below it).
    fn has_method(&self, class: &str, method: &str) -> bool {
        match self.methods.get(method).map(String::as_str) {
            Some("EntityLiving") => true,
            Some(decl) => self.chain(class).iter().any(|c| c == decl),
            None => false,
        }
    }

    /// The class at or above `class` that overrides `method`, below the
    /// declaring class.
    fn defining_class(&self, class: &str, method: &str) -> Option<String> {
        self.chain(class)
            .into_iter()
            .find(|c| self.overrides.contains(&(c.clone(), method.to_string())))
    }

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

/// One entity of every living kind, at (0.5, 70, 0.5).
fn living_kinds(w: &mut World) -> Vec<Entity> {
    use EntityKindData as K;
    let mut out = Vec::new();
    let mut push = |w: &mut World, make: &dyn Fn(&mut World, &mut EntityBase) -> EntityKindData| {
        let mut b = EntityBase::new(w);
        let kind = make(w, &mut b);
        b.set_position(0.5, 70.0, 0.5);
        out.push(Entity { base: b, kind });
    };
    let c = CreatureData::default;
    let m = MobData::default;
    let a = AnimalData::default;
    push(w, &|w, b| K::Mob(LivingData::new(w, b), c(), m()));
    push(w, &|w, b| {
        K::Creeper(LivingData::new(w, b), c(), m(), CreeperData::default())
    });
    push(w, &|w, b| {
        K::Skeleton(LivingData::new(w, b), c(), m(), SkeletonData::default())
    });
    push(w, &|w, b| {
        K::Spider(LivingData::new(w, b), c(), m(), SpiderData::default())
    });
    push(w, &|w, b| {
        K::Zombie(LivingData::new(w, b), c(), m(), ZombieData::default())
    });
    push(w, &|w, b| {
        K::PigZombie(
            LivingData::new(w, b),
            c(),
            m(),
            ZombieData::default(),
            PigZombieData::default(),
        )
    });
    push(w, &|w, b| {
        K::GiantZombie(LivingData::new(w, b), c(), m(), GiantZombieData::default())
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
    push(w, &|w, b| {
        b.data_watcher.add_object(16, WatchValue::Byte(0));
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
    out
}

type Probe = fn(&mut World, &mut Entity);

/// One call per entry point, keyed by the Java method name.
const PROBES: &[(&str, Probe)] = &[
    ("attackBlockedEntity", |w, e| {
        d::attack_blocked_entity(e, w, 12345, 3.0)
    }),
    ("attackEntity", |w, e| d::attack_entity(e, w, 12345, 3.0)),
    ("canBreatheUnderwater", |w, e| {
        d::can_breathe_underwater(e, w);
    }),
    ("canDespawn", |w, e| {
        d::can_despawn(e, w);
    }),
    ("damageEntity", |w, e| d::damage_entity(e, w, 1)),
    ("dropFewItems", |w, e| d::drop_few_items(e, w)),
    ("findPlayerToAttack", |w, e| {
        d::find_player_to_attack(e, w);
    }),
    ("getBlockPathWeight", |w, e| {
        d::get_block_path_weight(e, w, 0, 61, 0);
    }),
    ("getCanSpawnHere", |w, e| {
        d::get_can_spawn_here(e, w);
    }),
    ("getDeathSound", |w, e| {
        d::get_death_sound(e, w);
    }),
    ("getDropItemId", |w, e| {
        d::get_drop_item_id(e, w);
    }),
    ("getHurtSound", |w, e| {
        d::get_hurt_sound(e, w);
    }),
    ("getLivingSound", |w, e| {
        d::get_living_sound(e, w);
    }),
    ("getMaxSpawnedInChunk", |w, e| {
        d::get_max_spawned_in_chunk(e, w);
    }),
    ("getSoundVolume", |w, e| {
        d::get_sound_volume(e, w);
    }),
    ("getTalkInterval", |w, e| {
        d::get_talk_interval(e, w);
    }),
    ("getVerticalFaceSpeed", |w, e| {
        d::get_vertical_face_speed(e, w);
    }),
    ("isMovementBlocked", |w, e| {
        d::is_movement_blocked(e, w);
    }),
    ("isMovementCeased", |w, e| {
        d::is_movement_ceased(e, w);
    }),
    ("isOnLadder", |w, e| {
        d::is_on_ladder(e, w);
    }),
    ("jump", |w, e| d::jump(e, w)),
    ("moveEntityWithHeading", |w, e| {
        d::move_entity_with_heading(e, w, 0.5, 0.5)
    }),
    ("onDeath", |w, e| d::on_death(e, w, None)),
    ("onLivingUpdate", |w, e| d::on_living_update(e, w)),
    ("updatePlayerActionState", |w, e| {
        d::update_player_action_state(e, w)
    }),
];

fn transcribed() -> BTreeSet<(String, String)> {
    d::LIVING_TRANSCRIBED
        .iter()
        .flat_map(|(c, ms)| ms.iter().map(move |m| (c.to_string(), m.to_string())))
        .collect()
}

fn not_dispatched() -> BTreeSet<&'static str> {
    d::LIVING_NOT_DISPATCHED.iter().map(|(m, _)| *m).collect()
}

/// The registries name real methods and overrides, and every overridden
/// method is probed or has a reason not to be dispatched.
#[test]
fn registries_match_the_vanilla_override_set() {
    let s = OverrideSet::parse();
    assert!(
        s.methods.len() > 40 && s.overrides.len() > 100,
        "living_overrides.txt looks truncated"
    );
    let nd = not_dispatched();
    for (m, why) in d::LIVING_NOT_DISPATCHED {
        assert!(s.methods.contains_key(*m), "LIVING_NOT_DISPATCHED: {m}");
        assert!(!why.is_empty(), "LIVING_NOT_DISPATCHED: {m} needs a reason");
    }
    let probed: BTreeSet<&str> = PROBES.iter().map(|(m, _)| *m).collect();
    assert_eq!(probed.len(), PROBES.len(), "a method is probed twice");
    for m in &probed {
        assert!(s.methods.contains_key(*m), "probe {m} is not a method row");
        assert!(
            !nd.contains(m),
            "{m} is both probed and LIVING_NOT_DISPATCHED"
        );
    }
    for (c, m) in transcribed() {
        assert!(
            s.overrides.contains(&(c.clone(), m.clone())),
            "LIVING_TRANSCRIBED lists {c}.{m}, which is not an override in the vanilla tree"
        );
    }
    for m in s.overridden_methods() {
        assert!(
            nd.contains(m.as_str()) || probed.contains(m.as_str()),
            "{}.{m} is overridden but has no dispatch entry point probed in \
             tests/living_dispatch_coverage.rs (or a LIVING_NOT_DISPATCHED reason)",
            s.methods[&m]
        );
    }
}

/// Every overriding class is a living kind's class or one of its ancestors.
#[test]
fn every_override_is_inherited_by_a_kind() {
    let s = OverrideSet::parse();
    let mut w = empty_world();
    let kinds = living_kinds(&mut w);
    let mut reached = BTreeSet::new();
    for e in &kinds {
        let class = e.kind.java_class();
        for m in s.overridden_methods() {
            if let Some(c) = s.defining_class(class, &m) {
                reached.insert((c, m));
            }
        }
    }
    let all: BTreeSet<(String, String)> = s.overrides.iter().cloned().collect();
    let unreached: BTreeSet<_> = all.difference(&reached).cloned().collect();
    // Overrides every subclass overrides again, reached only through
    // `super` in vanilla: EntityPlayer's onLivingUpdate /
    // updatePlayerActionState under EntityPlayerSP's, and
    // EntityCreature.getCanSpawnHere under EntityMob's, EntityAnimal's,
    // EntityWaterMob's (creature::get_can_spawn_here, transcribed).
    let shadowed: BTreeSet<(String, String)> = [
        ("EntityCreature", "getCanSpawnHere"),
        ("EntityPlayer", "onLivingUpdate"),
        ("EntityPlayer", "updatePlayerActionState"),
    ]
    .iter()
    .map(|(c, m)| (c.to_string(), m.to_string()))
    .collect();
    assert_eq!(unreached, shadowed);
}

/// The core check (module docs).
#[test]
fn every_entry_point_records_exactly_the_inherited_override() {
    let s = OverrideSet::parse();
    let done = transcribed();
    let mut failures = Vec::new();
    let mut recorded_pairs = BTreeSet::new();
    for (method, probe) in PROBES {
        let suffix = format!(".{method}");
        let mut world = empty_world();
        world.get_chunk_from_chunk_coords(0, 0);
        let kinds = living_kinds(&mut world);
        for mut e in kinds {
            let class = e.kind.java_class();
            if !s.has_method(class, method) {
                continue;
            }
            world.unimplemented_hits.borrow_mut().clear();
            probe(&mut world, &mut e);
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
        "{} living overrides recorded as not transcribed, {} transcribed",
        recorded_pairs.len(),
        done.len()
    );
}
