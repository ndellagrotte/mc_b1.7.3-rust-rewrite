//! Per-fixture gap reporting (Gate 1 N7, Gate 2 R7; tests/golden/README.md
//! "The gaps file").
//!
//! The simulation counts every untranscribed world system it reaches in
//! [`World::skipped_systems`] and every recorded override, guard or Java
//! exception path in [`World::unimplemented_hits`] (mc-sim's lib docs,
//! "Untranscribed code"). The tick and world-read drivers copy both maps
//! into the [`TraceWriter`] when the world is done ([`TraceWriter::set_gaps`]),
//! and [`crate::run`] writes them to `gaps.json` next to `actual.jsonl`. The
//! file is a diagnostic only: it is never compared, never printed by a
//! passing test, and no `expected.jsonl` changes because of it.

use std::collections::BTreeMap;

use mc_sim::world::World;
use serde_json::{Map, Value, json};

/// The gap counters of one world (or of the chain of worlds a scenario's
/// `reload` ops build: [`World::adopt_statics`] carries the counters over).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Gaps {
    /// [`World::skipped_systems`]: Java name → times reached.
    pub skipped_systems: BTreeMap<String, u64>,
    /// [`World::unimplemented_hits`]: record → times reached.
    pub unimplemented_hits: BTreeMap<String, u64>,
}

impl Gaps {
    /// The counters `world` holds now.
    pub fn of(world: &World) -> Self {
        let copy = |m: &BTreeMap<&'static str, u64>| {
            m.iter()
                .map(|(k, v)| ((*k).to_string(), *v))
                .collect::<BTreeMap<_, _>>()
        };
        Gaps {
            skipped_systems: copy(&world.skipped_systems.borrow()),
            unimplemented_hits: copy(&world.unimplemented_hits.borrow()),
        }
    }

    /// No gap was reached.
    pub fn is_empty(&self) -> bool {
        self.skipped_systems.is_empty() && self.unimplemented_hits.is_empty()
    }

    /// `{"skipped_systems": {name: count, …}, "unimplemented_hits": {…}}`,
    /// keys sorted.
    pub fn to_json(&self) -> Value {
        let obj = |m: &BTreeMap<String, u64>| {
            Value::Object(
                m.iter()
                    .map(|(k, v)| (k.clone(), json!(v)))
                    .collect::<Map<_, _>>(),
            )
        };
        json!({
            "skipped_systems": obj(&self.skipped_systems),
            "unimplemented_hits": obj(&self.unimplemented_hits),
        })
    }

    /// The `gaps.json` text: [`Gaps::to_json`] pretty-printed, `\n`-terminated.
    pub fn to_file_text(&self) -> String {
        let mut s = serde_json::to_string_pretty(&self.to_json()).unwrap_or_default();
        s.push('\n');
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_shape_is_sorted_and_complete() {
        let mut g = Gaps::default();
        assert!(g.is_empty());
        g.unimplemented_hits.insert("b".into(), 2);
        g.unimplemented_hits.insert("a".into(), 1);
        g.skipped_systems.insert("S".into(), 3);
        assert!(!g.is_empty());
        assert_eq!(
            g.to_file_text(),
            "{\n  \"skipped_systems\": {\n    \"S\": 3\n  },\n  \"unimplemented_hits\": {\n    \"a\": 1,\n    \"b\": 2\n  }\n}\n"
        );
        assert_eq!(
            Gaps::default().to_json(),
            json!({"skipped_systems": {}, "unimplemented_hits": {}})
        );
    }
}
