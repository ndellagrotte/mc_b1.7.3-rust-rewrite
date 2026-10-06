//! `tests/golden/item/item-table` (Tier A kind `item_table`, WP-1.2): the
//! Rust item table replayed against the oracle's reflection dump of
//! `Item.itemsList`, field by field and data method by data method, plus
//! `EnumToolMaterial`, the block texture lookups behind the item icons, and
//! `ItemStack` value operations (format: tests/golden/README.md
//! "`kind: \"item_table\"`").
//!
//! The Rust side builds each line in the oracle's shape from
//! `mc_sim::item` and `mc_golden::run` compares by parsed JSON value, so a
//! field the table lacks, an extra field, or any differing value fails with
//! the first differing line and key. Java exceptions are compared too:
//! where the oracle recorded `{"exception": "<class>"}`, the Rust method must
//! return the matching `JavaException`.

use mc_golden::{Bits32, GoldenError, Scenario, TraceLine, TraceWriter, fixture_dir};
use mc_sim::block::{self, Material};
use mc_sim::item::block_texture::{
    get_block_texture_from_side, get_block_texture_from_side_and_metadata,
};
use mc_sim::item::{
    EnumToolMaterial, ItemDef, ItemKind, ItemStack, JavaException, item, max_item_id,
};
use serde_json::{Map, Value, json};

fn exception(e: JavaException) -> Value {
    json!({ "exception": e.simple_name() })
}

fn result<T: Into<Value>>(r: Result<T, JavaException>) -> Value {
    match r {
        Ok(v) => v.into(),
        Err(e) => exception(e),
    }
}

fn f32v(v: f32) -> Value {
    Bits32::from(v).into()
}

fn opt_str(s: Option<String>) -> Value {
    s.map_or(Value::Null, Value::String)
}

/// `Material` reduced to the name of the `Material` static field holding it
/// (only doors have a `Material` field).
fn material_name(m: Material) -> &'static str {
    match m {
        Material::Wood => "wood",
        Material::Iron => "iron",
        other => panic!("no Material field name mapped for {other:?}"),
    }
}

/// One class level's declared instance fields, as ItemTable.java reduces them.
fn class_fields(class: &str, d: &ItemDef) -> Value {
    let k = d.kind;
    match class {
        "Item" => json!({
            "bFull3D": d.b_full_3d,
            "containerItem": d.container_item,
            "hasSubtypes": d.has_subtypes,
            "iconIndex": d.icon_index,
            "itemName": d.item_name.map(|n| format!("item.{n}")),
            "maxDamage": d.max_damage,
            "maxStackSize": d.max_stack_size,
            "shiftedIndex": d.shifted_index,
        }),
        "ItemTool" => {
            let t = k.tool().expect("ItemTool kind");
            json!({
                "blocksEffectiveAgainst": t.blocks_effective_against,
                "damageVsEntity": t.damage_vs_entity,
                "efficiencyOnProperMaterial": f32v(t.efficiency_on_proper_material),
                "toolMaterial": t.tool_material.name(),
            })
        }
        "ItemFood" => {
            let f = k.food().expect("ItemFood kind");
            json!({
                "healAmount": f.heal_amount,
                "isWolfsFavoriteMeat": f.is_wolfs_favorite_meat,
            })
        }
        "ItemBlock" => json!({ "blockID": k.item_block_id().expect("ItemBlock kind") }),
        _ => match k {
            ItemKind::Sword { weapon_damage } => json!({ "weaponDamage": weapon_damage }),
            ItemKind::Seeds { block_type } => json!({ "blockType": block_type }),
            ItemKind::Armor {
                armor_level,
                armor_type,
                damage_reduce_amount,
                render_index,
            } => json!({
                "armorLevel": armor_level,
                "armorType": armor_type,
                "damageReduceAmount": damage_reduce_amount,
                "renderIndex": render_index,
            }),
            ItemKind::Door { door_material } => {
                json!({ "doorMaterial": material_name(door_material) })
            }
            ItemKind::Bucket { is_full } => json!({ "isFull": is_full }),
            ItemKind::Minecart { minecart_type } => json!({ "minecartType": minecart_type }),
            ItemKind::Reed { spawn_id } => json!({ "spawnID": spawn_id }),
            ItemKind::Record { record_name } => json!({ "recordName": record_name }),
            _ => json!({}),
        },
    }
}

fn per_damage(f: impl Fn(i32) -> Value) -> Value {
    Value::Array((0..16).map(f).collect())
}

/// 256 entries: `f(block)` for present blocks, null for absent ones.
fn per_block(f: impl Fn(&block::BlockDef) -> Value) -> Value {
    Value::Array(
        (0..256)
            .map(|id| block::block(id).map_or(Value::Null, &f))
            .collect(),
    )
}

fn item_line(id: i32) -> TraceLine {
    let l = TraceLine::new(0, "item").with("id", id);
    let Some(d) = item(id) else {
        return l.with("present", false);
    };
    let mut fields = Map::new();
    for class in d.kind.java_class_chain() {
        fields.insert(class.to_string(), class_fields(class, d));
    }
    let methods = json!({
        "getIconFromDamage": per_damage(|m| d.get_icon_from_damage(m).into()),
        "getItemNameIS": per_damage(|m| match d.get_item_name_is(&ItemStack::new(id, 1, m)) {
            Ok(s) => opt_str(s),
            Err(e) => exception(e),
        }),
        "getPlacedBlockMetadata": per_damage(|m| d.get_placed_block_metadata(m).into()),
        "getColorFromDamage": per_damage(|m| d.get_color_from_damage(m).into()),
        "getItemName": opt_str(d.get_item_name()),
        "getItemStackLimit": d.get_item_stack_limit(),
        "getMaxDamage": d.get_max_damage(),
        "isDamagable": d.is_damagable(),
        "isFull3D": d.is_full_3d(),
        "shouldRotateAroundWhenRendering": d.should_rotate_around_when_rendering(),
        "getHasSubtypes": d.get_has_subtypes(),
        "getContainerItem": d.get_container_item().map(|c| c.shifted_index),
        "hasContainerItem": d.has_container_item(),
        "getDamageVsEntity": d.get_damage_vs_entity(),
        "getStrVsBlock": per_block(|b| f32v(d.get_str_vs_block(b))),
        "canHarvestBlock": per_block(|b| d.can_harvest_block(b).into()),
    });
    l.with("present", true)
        .with("class", d.kind.java_class())
        .with("fields", Value::Object(fields))
        .with("methods", methods)
}

fn tool_material_line(m: EnumToolMaterial) -> TraceLine {
    TraceLine::new(0, "tool_material")
        .with("name", m.name())
        .with("ordinal", m.ordinal())
        .with(
            "fields",
            json!({
                "damageVsEntity": m.get_damage_vs_entity(),
                "efficiencyOnProperMaterial": f32v(m.get_efficiency_on_proper_material()),
                "harvestLevel": m.get_harvest_level(),
                "maxUses": m.get_max_uses(),
            }),
        )
        .with("getMaxUses", m.get_max_uses())
        .with_f32(
            "getEfficiencyOnProperMaterial",
            m.get_efficiency_on_proper_material(),
        )
        .with("getDamageVsEntity", m.get_damage_vs_entity())
        .with("getHarvestLevel", m.get_harvest_level())
}

fn block_texture_line(id: i32) -> TraceLine {
    let side: Vec<Value> = (0..6)
        .map(|s| get_block_texture_from_side(id, s).into())
        .collect();
    let side_meta: Vec<Value> = (0..6)
        .map(|s| {
            Value::Array(
                (0..16)
                    .map(|m| result(get_block_texture_from_side_and_metadata(id, s, m)))
                    .collect(),
            )
        })
        .collect();
    TraceLine::new(0, "block_texture")
        .with("id", id)
        .with("getBlockTextureFromSide", side)
        .with("getBlockTextureFromSideAndMetadata", side_meta)
}

fn stack_json(s: &ItemStack) -> Value {
    json!({
        "id": s.item_id,
        "count": s.stack_size,
        "damage": s.get_item_damage(),
        "animationsToGo": s.animations_to_go,
    })
}

fn int(v: &Value, key: &str) -> i32 {
    let n = v[key]
        .as_i64()
        .unwrap_or_else(|| panic!("stacks: `{key}` is not an integer"));
    i32::try_from(n).expect("stacks: value out of int range")
}

fn stack_line(i: usize, s: &Value) -> TraceLine {
    let (id, count, damage) = (int(s, "id"), int(s, "count"), int(s, "damage"));
    let st = ItemStack::new(id, count, damage);

    let nbt = st.write_to_nbt();
    let c = nbt.as_compound().expect("compound");
    let types: Map<String, Value> = c
        .iter()
        .map(|(k, t)| (k.clone(), t.tag_type().id().into()))
        .collect();
    let num = |key: &str| -> Value {
        match c.get(key) {
            Some(mc_nbt::Tag::Short(v)) => (*v).into(),
            Some(mc_nbt::Tag::Byte(v)) => (*v).into(),
            other => panic!("unexpected tag {other:?}"),
        }
    };
    let read = ItemStack::from_nbt(&nbt).expect("read back");

    let mut d = st.copy();
    let after: Vec<Value> = s["damage_by"]
        .as_array()
        .expect("damage_by")
        .iter()
        .map(|n| {
            d.damage_item(i32::try_from(n.as_i64().expect("int")).expect("int"));
            stack_json(&d)
        })
        .collect();

    let mut sp = st.copy();
    let part = sp.split_stack(int(s, "split"));

    TraceLine::new(0, "item_stack")
        .with("i", i)
        .with("id", id)
        .with("count", count)
        .with("damage", damage)
        .with("nbt_types", Value::Object(types))
        .with(
            "nbt",
            json!({ "id": num("id"), "Count": num("Count"), "Damage": num("Damage") }),
        )
        .with("read", stack_json(&read))
        .with("copy", stack_json(&st.copy()))
        .with("getMaxStackSize", st.get_max_stack_size())
        .with("isStackable", st.is_stackable())
        .with("isItemStackDamageable", st.is_item_stack_damageable())
        .with("isItemDamaged", st.is_item_damaged())
        .with("getHasSubtypes", st.get_has_subtypes())
        .with("getMaxDamage", st.get_max_damage())
        .with(
            "getItemName",
            match st.get_item_name() {
                Ok(n) => opt_str(n),
                Err(e) => exception(e),
            },
        )
        .with("getIconIndex", st.get_icon_index())
        .with("getDamageVsEntity", st.get_damage_vs_entity())
        .with("damageItem", after)
        .with("splitStack", json!([stack_json(&sp), stack_json(&part)]))
}

fn drive(scenario: &Scenario, out: &mut TraceWriter) -> Result<(), GoldenError> {
    if !matches!(scenario.kind, mc_golden::ScenarioKind::ItemTable) {
        return Err(GoldenError::other("not an item_table scenario"));
    }
    // The kind-specific field `stacks` is not modelled by mc-golden; read it
    // from the file.
    let path = fixture_dir("item", &scenario.name).join("scenario.json");
    let text = std::fs::read_to_string(&path).map_err(|e| GoldenError::other(e.to_string()))?;
    let raw: Value = serde_json::from_str(&text).map_err(|e| GoldenError::other(e.to_string()))?;

    for id in 0..=max_item_id() {
        out.push(item_line(id));
    }
    for m in EnumToolMaterial::VALUES {
        out.push(tool_material_line(m));
    }
    for id in 0..256 {
        if block::block(id).is_some() {
            out.push(block_texture_line(id));
        }
    }
    for (i, s) in raw["stacks"].as_array().expect("stacks").iter().enumerate() {
        out.push(stack_line(i, s));
    }
    Ok(())
}

#[test]
fn item_item_table() {
    if let Err(e) = mc_golden::run(fixture_dir("item", "item-table"), drive) {
        panic!("item/item-table: {e}");
    }
}

/// The table's id layout: 96 `ItemBlock`s, 104 items, 2 records.
#[test]
fn item_table_layout() {
    let present: Vec<i32> = (0..32000).filter(|&id| item(id).is_some()).collect();
    assert_eq!(present.len(), 202);
    assert_eq!(max_item_id(), 2257);
    for &id in &present {
        assert_eq!(item(id).unwrap().shifted_index, id, "shiftedIndex of {id}");
    }
    assert_eq!(
        mc_sim::item::all_items()
            .map(|d| d.shifted_index)
            .collect::<Vec<_>>(),
        present
    );
}
