//! `tests/golden/block/block-table` (WP-1.1, tests/golden/README.md
//! the `kind: "block_table"` section): the oracle's dump of every `Block.blocksList`
//! entry, the per-id static arrays, every `Material` and `MapColor`, right
//! after Block.java's static initialiser. This test rebuilds every line from
//! the Rust table ([`mc_sim::block`]) and a fresh [`BlockStatics`] and
//! compares the whole trace, floats by bit pattern.
//!
//! The scenario has no Rust driver in `mc-golden` (its kind is defined here),
//! so the expected file is read directly and compared with
//! [`mc_golden::compare`].

use mc_core::JavaRandom;
use mc_golden::{Bits32, Bits64, TraceLine, compare, fixture_dir, parse_jsonl, read_sidecar};
use mc_sim::block::{
    self, BlockDef, BlockKind, BlockStatics, MapColor, Material, StepSound, block,
};
use serde_json::{Map, Value, json};

fn f32v(v: f32) -> Value {
    Bits32::from(v).into()
}

fn f64v(v: f64) -> Value {
    Bits64::from(v).into()
}

/// `StepSound` reduction: class, name, `getVolume()`, `getPitch()`,
/// `stepSoundDir()`, `stepSoundDir2()`.
fn step_sound(s: &StepSound) -> Value {
    json!({
        "class": s.class.java_class(),
        "name": s.step_sound_name,
        "volume": f32v(s.get_volume()),
        "pitch": f32v(s.get_pitch()),
        "stepSoundDir": s.step_sound_dir(),
        "stepSoundDir2": s.step_sound_dir2(),
    })
}

/// Every non-static field of the block instance, by Java name.
fn fields(def: &BlockDef, st: &BlockStatics) -> Value {
    let mut m = Map::new();
    let mut put = |k: &str, v: Value| {
        assert!(m.insert(k.to_string(), v).is_none(), "field {k} twice");
    };
    // Block (Block.java:120-136)
    put("blockIndexInTexture", def.block_index_in_texture.into());
    put("blockID", def.block_id.into());
    put("blockHardness", f32v(def.block_hardness));
    put("blockResistance", f32v(def.block_resistance));
    put(
        "blockConstructorCalled",
        def.block_constructor_called.into(),
    );
    put("enableStats", def.enable_stats.into());
    put("minX", f64v(def.bounds.min_x));
    put("minY", f64v(def.bounds.min_y));
    put("minZ", f64v(def.bounds.min_z));
    put("maxX", f64v(def.bounds.max_x));
    put("maxY", f64v(def.bounds.max_y));
    put("maxZ", f64v(def.bounds.max_z));
    put("stepSound", step_sound(&def.step_sound));
    put("blockParticleGravity", f32v(def.block_particle_gravity));
    put("blockMaterial", def.block_material.java_name().into());
    put("slipperiness", f32v(def.slipperiness));
    put("blockName", def.get_block_name().into());

    // The singleton's live bounds start as the table's.
    let live = &st.bounds[def.block_id as usize];
    assert_eq!(*live, def.bounds, "BlockStatics::bounds[{}]", def.block_id);

    use BlockKind as K;
    if let Some(local_flag) = def.kind.breakable_local_flag() {
        put("localFlag", local_flag.into());
    }
    if let Some(is_powered) = def.kind.rail_is_powered() {
        put("isPowered", is_powered.into());
    }
    match def.kind {
        K::Flowing => {
            let f = st.flowing_scratch(def.block_id).expect("BlockFlowing id");
            put("numAdjacentSources", f.num_adjacent_sources.into());
            put(
                "isOptimalFlowDirection",
                f.is_optimal_flow_direction.to_vec().into(),
            );
            put("flowCost", f.flow_cost.to_vec().into());
        }
        K::Leaves => {
            put("baseIndexInPNG", def.base_index_in_png().into());
            put("graphicsLevel", st.graphics_level.into());
            put(
                "adjacentTreeBlocks",
                st.leaves_adjacent_tree_blocks
                    .as_ref()
                    .map(|a| a.to_vec())
                    .into(),
            );
        }
        // BlockChest.java:6, BlockDispenser.java:6: an unseeded `new Random()`;
        // only the type is dumped (README reduction rules).
        K::Chest | K::Dispenser => put("random", "java.util.Random".into()),
        K::Furnace { is_active } => {
            // BlockFurnace.java:6, as above.
            put("furnaceRand", "java.util.Random".into());
            put("isActive", is_active.into());
        }
        K::PistonBase { is_sticky } => {
            put("isSticky", is_sticky.into());
            put(
                "ignoreUpdates",
                st.piston_ignore_updates(def.block_id)
                    .expect("piston id")
                    .into(),
            );
        }
        K::PistonExtension => put("headTexture", st.piston_extension_head_texture.into()),
        K::Step { block_type } | K::Pumpkin { block_type } => put("blockType", block_type.into()),
        K::Fire => {
            put(
                "chanceToEncourageFire",
                block::CHANCE_TO_ENCOURAGE_FIRE.to_vec().into(),
            );
            put(
                "abilityToCatchFire",
                block::ABILITY_TO_CATCH_FIRE.to_vec().into(),
            );
        }
        K::Stairs { model_block } => put("modelBlock", model_block.into()),
        K::RedstoneWire => {
            put(
                "wiresProvidePower",
                st.redstone_wire_wires_provide_power.into(),
            );
            let set: Vec<Value> = st
                .redstone_wire_blocks_needing_update
                .iter()
                .map(|p| json!([p.x, p.y, p.z]))
                .collect();
            put("blocksNeedingUpdate", set.into());
        }
        K::Sign { is_freestanding } => {
            put("isFreestanding", is_freestanding.into());
            // BlockSign.java:6, Block.java:655/660: always TileEntitySign.class.
            put("signEntityClass", "TileEntitySign".into());
        }
        K::PressurePlate { trigger_mob_type } => {
            put("triggerMobType", trigger_mob_type.java_name().into());
        }
        K::RedstoneOre { field_468_a } => put("field_468_a", field_468_a.into()),
        K::RedstoneTorch { torch_active } => put("torchActive", torch_active.into()),
        K::RedstoneRepeater {
            is_repeater_powered,
        } => put("isRepeaterPowered", is_repeater_powered.into()),
        _ => {}
    }
    Value::Object(m)
}

fn block_line(id: i32, st: &BlockStatics) -> TraceLine {
    let l = TraceLine::new(0, "block_def").with("id", id);
    let Some(def) = block(id) else {
        return l.with("present", false);
    };
    let ids: Vec<Value> = (0..16)
        .map(|m| block::id_dropped(def, m, &mut JavaRandom::new(0)).into())
        .collect();
    let damage: Vec<Value> = (0..16)
        .map(|m| block::damage_dropped(def, m).into())
        .collect();
    let side: Vec<Value> = (0..6)
        .map(|s| block::get_block_texture_from_side(def, s).into())
        .collect();
    let mut tex = Vec::new();
    for s in 0..6 {
        for m in 0..16 {
            tex.push(
                match block::get_block_texture_from_side_and_metadata(def, s, m, st) {
                    Some(t) => t.into(),
                    None => json!({"throws": "ArrayIndexOutOfBoundsException"}),
                },
            );
        }
    }
    l.with("present", true)
        .with("class", def.kind.java_class())
        .with("fields", fields(def, st))
        .with(
            "isOpaqueCube",
            block::is_opaque_cube(def.kind, st.graphics_level),
        )
        .with(
            "renderAsNormalBlock",
            block::render_as_normal_block(def.kind),
        )
        .with("getRenderType", block::get_render_type(def.kind))
        .with("getRenderBlockPass", block::get_render_block_pass(def))
        .with("isCollidable", block::is_collidable(def))
        .with("getMobilityFlag", def.get_mobility_flag())
        .with("tickRate", block::tick_rate(def))
        .with("getBlockName", def.get_block_name())
        .with("damageDropped", damage)
        .with("idDropped", ids)
        .with(
            "quantityDropped",
            block::quantity_dropped(def, &mut JavaRandom::new(0)),
        )
        .with("getBlockTextureFromSide", side)
        .with("getBlockTextureFromSideAndMetadata", tex)
}

fn statics_line() -> TraceLine {
    let ids = 0..256;
    let arr = |f: fn(i32) -> Value| -> Vec<Value> { ids.clone().map(f).collect() };
    TraceLine::new(0, "block_statics")
        .with("tickOnLoad", arr(|i| block::tick_on_load(i).into()))
        .with(
            "opaqueCubeLookup",
            arr(|i| block::opaque_cube_lookup(i).into()),
        )
        .with(
            "isBlockContainer",
            arr(|i| block::is_block_container(i).into()),
        )
        .with("lightOpacity", arr(|i| block::light_opacity(i).into()))
        .with("canBlockGrass", arr(|i| block::can_block_grass(i).into()))
        .with("lightValue", arr(|i| block::light_value(i).into()))
        .with(
            "setNeighborNotifyOnMetadataChange",
            arr(|i| block::set_neighbor_notify_on_metadata_change(i).into()),
        )
        .with(
            "blockName",
            arr(|i| block(i).and_then(BlockDef::get_block_name).into()),
        )
        .with("fire_encourage", block::CHANCE_TO_ENCOURAGE_FIRE.to_vec())
        .with("fire_catch", block::ABILITY_TO_CATCH_FIRE.to_vec())
}

fn material_line(m: Material) -> TraceLine {
    TraceLine::new(0, "material")
        .with("name", m.java_name())
        .with("class", m.java_class())
        .with(
            "fields",
            json!({
                "canBurn": m.get_burning(),
                "canHarvest": m.get_is_harvestable(),
                "groundCover": m.get_is_ground_cover(),
                "isOpaque": m.is_opaque(),
                "materialMapColor": m.material_map_color().color_index,
                "mobilityFlag": m.get_material_mobility(),
            }),
        )
        .with("getIsLiquid", m.get_is_liquid())
        .with("isSolid", m.is_solid())
        .with("getCanBlockGrass", m.get_can_block_grass())
        .with("getIsSolid", m.get_is_solid())
        .with("getBurning", m.get_burning())
        .with("getIsGroundCover", m.get_is_ground_cover())
        .with("getIsTranslucent", m.get_is_translucent())
        .with("getIsHarvestable", m.get_is_harvestable())
        .with("getMaterialMobility", m.get_material_mobility())
}

fn map_color_line(i: usize, c: Option<MapColor>) -> TraceLine {
    let l = TraceLine::new(0, "map_color").with("index", i);
    match c {
        None => l.with("present", false),
        Some(c) => l.with("present", true).with(
            "fields",
            json!({"colorIndex": c.color_index, "colorValue": c.color_value}),
        ),
    }
}

/// The whole trace, in the oracle's line order (README).
fn rust_trace() -> Vec<TraceLine> {
    let st = BlockStatics::default();
    let mut out: Vec<TraceLine> = (0..256).map(|id| block_line(id, &st)).collect();
    out.push(statics_line());
    let mut materials = Material::ALL;
    materials.sort_by_key(|m| m.java_name());
    out.extend(materials.into_iter().map(material_line));
    out.extend(
        MapColor::MAP_COLOR_ARRAY
            .into_iter()
            .enumerate()
            .map(|(i, c)| map_color_line(i, c)),
    );
    out
}

#[test]
fn block_table_matches_oracle() {
    let dir = fixture_dir("block", "block-table");
    let bytes = read_sidecar(&dir, "expected.jsonl").expect("read expected.jsonl");
    let text = String::from_utf8(bytes).expect("UTF-8");
    let expected = parse_jsonl(&text).expect("parse expected.jsonl");
    let actual = rust_trace();
    if let Err(m) = compare(&expected, &actual) {
        panic!("block/block-table: {m}");
    }
}

/// The comparison is sensitive: one changed table value fails it.
#[test]
fn block_table_comparison_is_sensitive() {
    let dir = fixture_dir("block", "block-table");
    let text = String::from_utf8(read_sidecar(&dir, "expected.jsonl").unwrap()).unwrap();
    let expected = parse_jsonl(&text).unwrap();
    let mut actual = rust_trace();
    actual[1] = actual[1].clone().with("tickRate", 11);
    assert!(compare(&expected, &actual).is_err());
}
