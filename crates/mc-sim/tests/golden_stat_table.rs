//! `tests/golden/stat/stat-table` (Tier A kind `stat_table`, WP-2.7): the
//! Rust `StatList` / `AchievementList` replayed against the oracle's
//! reflection dump of vanilla's registries after the static initialisers,
//! and the three `IStatType` formats of the scenario's `format_values`
//! (format: tests/golden/README.md "`kind: \"stat_table\"`"; oracle:
//! tools/oracle/java/harness/StatTable.java).

use mc_golden::{GoldenError, Scenario, TraceLine, TraceWriter, fixture_dir};
use mc_sim::stats::{StatBase, StatList, StatType};
use serde_json::{Value, json};

fn stat_fields(l: TraceLine, s: &StatBase) -> TraceLine {
    let l = l
        .with("statId", s.stat_id)
        .with("name", s.stat_name.key_form())
        .with("class", s.class_name())
        .with("type", s.stat_type.class_name())
        .with("isIndependent", s.is_independent);
    match s.get_item_id() {
        Some(id) => l.with("itemId", id),
        None => l,
    }
}

fn ids<'a>(it: impl Iterator<Item = &'a StatBase>) -> Value {
    Value::Array(it.map(|s| json!(s.stat_id)).collect())
}

fn drive(scenario: &Scenario, out: &mut TraceWriter) -> Result<(), GoldenError> {
    if !matches!(scenario.kind, mc_golden::ScenarioKind::StatTable) {
        return Err(GoldenError::other("not a stat_table scenario"));
    }
    // The kind-specific field `format_values` is not modelled by mc-golden;
    // read it from the file.
    let path = fixture_dir("stat", &scenario.name).join("scenario.json");
    let text = std::fs::read_to_string(&path).map_err(|e| GoldenError::other(e.to_string()))?;
    let raw: Value = serde_json::from_str(&text).map_err(|e| GoldenError::other(e.to_string()))?;

    let l = StatList::get();
    let listed: Vec<&StatBase> = l.field_25188_a().collect();
    out.push(
        TraceLine::new(0, "stat_count")
            .with("listed", listed.len())
            .with("oneShot", l.one_shot_stats().count())
            .with("generalStats", l.general_stats().count())
            .with("itemStats", l.item_stats().count())
            .with("objectMineStats", l.object_mine_stats().count())
            .with("achievements", l.achievement_list().count()),
    );
    for (i, s) in listed.iter().enumerate() {
        out.push(stat_fields(TraceLine::new(0, "stat").with("i", i), s));
    }
    for s in l.one_shot_stats() {
        if !listed.iter().any(|x| x.stat_id == s.stat_id) {
            out.push(stat_fields(TraceLine::new(0, "unlisted_stat"), s));
        }
    }
    for (name, list) in [
        ("generalStats", ids(l.general_stats())),
        ("itemStats", ids(l.item_stats())),
        ("objectMineStats", ids(l.object_mine_stats())),
    ] {
        out.push(
            TraceLine::new(0, "stat_list")
                .with("list", name)
                .with("ids", list),
        );
    }
    for name in [
        "mineBlockStatArray",
        "objectUseStats",
        "objectBreakStats",
        "objectCraftStats",
    ] {
        let (len, entries) = l.array_entries(name).expect("known array");
        out.push(
            TraceLine::new(0, "stat_array")
                .with("array", name)
                .with("length", len)
                .with(
                    "entries",
                    Value::Array(entries.iter().map(|(k, s)| json!([k, s.stat_id])).collect()),
                ),
        );
    }
    for (i, s) in l.achievement_list().enumerate() {
        let a = s.achievement().expect("an achievement");
        out.push(
            TraceLine::new(0, "achievement")
                .with("i", i)
                .with("statId", s.stat_id)
                .with("name", s.stat_name.key_form())
                .with("column", a.display_column)
                .with("row", a.display_row)
                .with(
                    "parent",
                    a.parent_achievement.map_or(Value::Null, |p| json!(p)),
                )
                .with(
                    "icon",
                    json!({
                        "id": a.the_item_stack.item_id,
                        "damage": a.the_item_stack.get_item_damage(),
                    }),
                )
                .with("isSpecial", a.get_special())
                .with("isIndependent", s.is_independent),
        );
    }
    out.push(
        TraceLine::new(0, "achievement_bounds")
            .with("minDisplayColumn", l.min_display_column)
            .with("minDisplayRow", l.min_display_row)
            .with("maxDisplayColumn", l.max_display_column)
            .with("maxDisplayRow", l.max_display_row),
    );
    if let Some(values) = raw["format_values"].as_array() {
        for ty in [StatType::Simple, StatType::Time, StatType::Distance] {
            for v in values {
                let v = i32::try_from(v.as_i64().expect("an integer")).expect("an int");
                out.push(
                    TraceLine::new(0, "stat_format")
                        .with("type", ty.class_name())
                        .with("v", v)
                        .with("s", ty.format(v)),
                );
            }
        }
    }
    Ok(())
}

#[test]
fn stat_stat_table() {
    if let Err(e) = mc_golden::run(fixture_dir("stat", "stat-table"), drive) {
        panic!("stat/stat-table: {e}");
    }
}
