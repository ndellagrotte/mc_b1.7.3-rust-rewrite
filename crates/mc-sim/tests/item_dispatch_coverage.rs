//! The item behaviour dispatch (`mc_sim::item::behaviour`) covers every
//! override of `Item.java` in the vanilla tree (WP-2.6), as
//! `block_dispatch_coverage.rs` does for blocks.
//!
//! Input: `tests/data/item_overrides.txt`, generated from the vanilla tree by
//! `tests/data/gen_item_overrides.sh` (the `Item` methods, the class
//! hierarchy, and every `(class, method)` override). Every override is
//! either of a data method ([`behaviour::DATA_METHODS`], on `ItemDef` since
//! WP-1.2 and pinned per item by `tests/golden/item/item-table`) or of a
//! behaviour method with an entry point probed below, and then listed in
//! [`behaviour::TRANSCRIBED`]. The probes call every entry point for every
//! item and check that none records an item gap.

use std::collections::{BTreeMap, BTreeSet};

use mc_sim::entity::Entity;
use mc_sim::entity::pig::new_entity_pig;
use mc_sim::entity::player::new_entity_player_sp;
use mc_sim::item::{self, ItemStack, behaviour};
use mc_sim::save::NoSave;
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::{World, WorldEnvironment};

const OVERRIDES: &str = include_str!("data/item_overrides.txt");

/// The parsed `item_overrides.txt`.
struct OverrideSet {
    /// `method` rows: name → Item.java line.
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
                other => panic!("item_overrides.txt: unknown row kind {other:?} in {line:?}"),
            }
        }
        s
    }

    /// The class at or above `class` that declares `method`, below `Item`.
    fn defining_class(&self, class: &str, method: &str) -> Option<String> {
        let mut c = class.to_string();
        while c != "Item" {
            if self.overrides.contains(&(c.clone(), method.to_string())) {
                return Some(c);
            }
            c = self
                .parent
                .get(&c)
                .unwrap_or_else(|| panic!("{c} is not an Item subclass in item_overrides.txt"))
                .clone();
        }
        None
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

type Probe = fn(&mut ItemStack, &mut World, &mut Entity, &mut Entity);

/// One call per behaviour entry point, keyed by the Java method name. The
/// player stands at (0.5, 61, 0.5) on stone; the target is a pig.
const PROBES: &[(&str, Probe)] = &[
    ("onItemUse", |s, w, p, _| {
        behaviour::on_item_use(s, p, w, 0, 60, 2, 1);
    }),
    ("onItemRightClick", |s, w, p, _| {
        behaviour::on_item_right_click(s, w, p);
    }),
    ("hitEntity", |s, w, p, t| {
        behaviour::hit_entity(s, t, p, w);
    }),
    ("onBlockDestroyed", |s, w, p, _| {
        behaviour::on_block_destroyed(s, 1, 0, 60, 2, p, w);
    }),
    ("saddleEntity", |s, w, _, t| {
        behaviour::saddle_entity(s, t, w);
    }),
    ("onUpdate", |s, w, p, _| {
        behaviour::on_update(s, w, p, 0, true);
    }),
    ("onCreated", |s, w, p, _| {
        behaviour::on_created(s, w, p);
    }),
];

fn transcribed() -> BTreeSet<(String, String)> {
    behaviour::TRANSCRIBED
        .iter()
        .flat_map(|(c, ms)| ms.iter().map(move |m| (c.to_string(), m.to_string())))
        .collect()
}

/// Every override is data or transcribed, the registries name real methods
/// and overrides, and every behaviour method with overrides is probed.
#[test]
fn registries_match_the_vanilla_override_set() {
    let s = OverrideSet::parse();
    assert!(
        s.methods.len() > 25 && s.overrides.len() > 60,
        "item_overrides.txt looks truncated"
    );
    for m in behaviour::DATA_METHODS {
        assert!(
            s.methods.contains_key(*m),
            "DATA_METHODS: {m} is not an Item method"
        );
    }
    let probed: BTreeSet<&str> = PROBES.iter().map(|(m, _)| *m).collect();
    for m in &probed {
        assert!(
            s.methods.contains_key(*m),
            "probe {m} is not an Item method"
        );
        assert!(
            !behaviour::DATA_METHODS.contains(m),
            "{m} is both probed and data"
        );
    }
    let done = transcribed();
    for (c, m) in &done {
        assert!(
            s.overrides.contains(&(c.clone(), m.clone())),
            "TRANSCRIBED lists {c}.{m}, which is not an override in the vanilla tree"
        );
        assert!(
            probed.contains(m.as_str()),
            "TRANSCRIBED {c}.{m} has no probe"
        );
    }
    for (c, m) in &s.overrides {
        if behaviour::DATA_METHODS.contains(&m.as_str()) {
            continue;
        }
        assert!(
            done.contains(&(c.clone(), m.clone())),
            "{c}.{m} (Item.java:{}) is overridden but not in behaviour::TRANSCRIBED",
            s.methods[m]
        );
    }
}

/// The class chains of `ItemKind` agree with the vanilla hierarchy, and
/// every override is reached by some item.
#[test]
fn every_override_is_reachable_and_chains_match() {
    let s = OverrideSet::parse();
    let mut reached = BTreeSet::new();
    for def in item::all_items() {
        let chain = def.kind.java_class_chain();
        for pair in chain.windows(2) {
            assert_eq!(
                s.parent.get(pair[0]).map(String::as_str),
                Some(pair[1]),
                "ItemKind chain {chain:?} disagrees with item_overrides.txt"
            );
        }
        for (_, m) in &s.overrides {
            if let Some(c) = s.defining_class(chain[0], m) {
                reached.insert((c, m.clone()));
            }
        }
    }
    let unreached: BTreeSet<_> = s.overrides.difference(&reached).collect();
    assert!(
        unreached.is_empty(),
        "overrides no item reaches: {unreached:?}"
    );
}

/// Every entry point, for every item, runs without recording an `Item*`
/// gap. Each call gets a fresh world: a stone platform, the player at
/// (0.5, 61, 0.5) and a pig beside it.
#[test]
fn no_entry_point_records_an_item_gap() {
    let mut other_hits: BTreeSet<String> = BTreeSet::new();
    let mut failures = Vec::new();
    for def in item::all_items() {
        for (method, probe) in PROBES {
            let mut w = empty_world();
            for x in -2..=2 {
                for z in -2..=4 {
                    w.set_block_and_metadata(x, 60, z, 1, 0);
                }
            }
            let mut p = new_entity_player_sp(&mut w);
            p.base
                .set_location_and_angles(0.5, 61.0 + 1.62f32 as f64, 0.5, 0.0, 30.0);
            let mut t = new_entity_pig(&mut w);
            t.base.set_location_and_angles(0.5, 61.0, 2.5, 0.0, 0.0);
            let mut stack = ItemStack::new(def.shifted_index, 1, 0);
            probe(&mut stack, &mut w, &mut p, &mut t);
            for k in w.unimplemented_hits.borrow().keys() {
                if k.starts_with("Item") {
                    failures.push(format!("{} {method}: recorded {k}", def.shifted_index));
                } else {
                    other_hits.insert(k.to_string());
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    eprintln!("other gaps the probes reached: {other_hits:?}");
}
