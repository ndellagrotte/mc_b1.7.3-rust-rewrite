//! Golden NBT fixtures (`tests/golden/nbt`, README `nbt_write` / `nbt_read`).
//!
//! These do not go through `mc_golden::run`: the oracle's `nbt_bytes` are in
//! Java `HashMap` order and ours in `BTreeMap` order, so whole-trace
//! comparison would fail by design (PLAN_v4 §4.3 Tier S compares parsed
//! trees). Instead each check rebuilds the relevant trace line from our side
//! and compares it with `mc_golden::compare` where the line is order-free
//! (`nbt_tree`, `nbt_roundtrip`, `nbt_file`), and compares bytes exactly only
//! for the fixtures listed in [`BYTE_EXACT`]; elsewhere it checks that our
//! bytes and the oracle's have the same length and parse to the same tree.

use mc_golden::{
    Bits32, Bits64, NbtNode, Scenario, ScenarioKind, TraceLine, all_fixture_dirs, compare,
    fixture_dir, hex_bytes, parse_hex_bytes, parse_jsonl, read_sidecar,
};
use mc_nbt::{Tag, TagType};
use serde_json::{Map, Value, json};

/// Fixtures whose oracle bytes our sorted-order writer reproduces exactly:
/// only those whose every compound has at most one child, or happens to be
/// in sorted order in the `HashMap`.
const BYTE_EXACT: &[&str] = &["empty-root"];

/// Every fixture under `tests/golden/nbt`, each checked by a test below.
const FIXTURES: &[&str] = &[
    "empty-root",
    "level-dat-shape",
    "nan-payloads",
    "primitives",
    "testworld-level-dat",
];

// ---------------------------------------------------------------------------
// Canonical tree JSON (README "Canonical tree").

fn type_name(ty: TagType) -> &'static str {
    match ty {
        TagType::End => "end",
        TagType::Byte => "byte",
        TagType::Short => "short",
        TagType::Int => "int",
        TagType::Long => "long",
        TagType::Float => "float",
        TagType::Double => "double",
        TagType::ByteArray => "byte_array",
        TagType::String => "string",
        TagType::List => "list",
        TagType::Compound => "compound",
    }
}

fn type_from_name(name: &str) -> TagType {
    TagType::ALL
        .into_iter()
        .find(|&ty| type_name(ty) == name)
        .unwrap_or_else(|| panic!("unknown NBT type name {name:?}"))
}

/// `{"type":T,"name":N,"value":V}` for the root.
fn canonical_root(name: &str, tag: &Tag) -> Value {
    let mut node = canonical_node(tag);
    node["name"] = name.into();
    node
}

/// `{"type":T,"value":V}`.
fn canonical_node(tag: &Tag) -> Value {
    json!({"type": type_name(tag.tag_type()), "value": canonical_value(tag)})
}

/// A bare value: list items are these. Floats are raw bit patterns
/// (`floatToRawIntBits`), byte arrays hex, compound children keyed by name
/// (the `BTreeMap` already gives the sorted order the oracle writes).
fn canonical_value(tag: &Tag) -> Value {
    match tag {
        Tag::Byte(v) => (*v).into(),
        Tag::Short(v) => (*v).into(),
        Tag::Int(v) => (*v).into(),
        Tag::Long(v) => (*v).into(),
        Tag::Float(v) => Bits32::from(*v).into(),
        Tag::Double(v) => Bits64::from(*v).into(),
        Tag::ByteArray(bytes) => hex_bytes(bytes).into(),
        Tag::String(s) => s.as_str().into(),
        Tag::List { elem, items } => json!({
            "elem": type_name(*elem),
            "items": items.iter().map(canonical_value).collect::<Vec<_>>(),
        }),
        Tag::Compound(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), canonical_node(v)))
                .collect::<Map<_, _>>(),
        ),
    }
}

// ---------------------------------------------------------------------------
// Scenario typed tree -> `Tag` (README `nbt_write`).

fn tag_from_node(node: &NbtNode) -> Tag {
    tag_from_value(type_from_name(&node.ty), &node.value)
}

fn tag_from_value(ty: TagType, v: &Value) -> Tag {
    let int = || {
        v.as_i64()
            .unwrap_or_else(|| panic!("{v} is not an integer"))
    };
    let text = || v.as_str().unwrap_or_else(|| panic!("{v} is not a string"));
    match ty {
        TagType::End => panic!("`end` is not a value type"),
        TagType::Byte => Tag::Byte(int().try_into().expect("byte range")),
        TagType::Short => Tag::Short(int().try_into().expect("short range")),
        TagType::Int => Tag::Int(int().try_into().expect("int range")),
        TagType::Long => Tag::Long(int()),
        TagType::Float => Tag::Float(Bits32::parse(text()).expect("f32 bits").to_f32()),
        TagType::Double => Tag::Double(Bits64::parse(text()).expect("f64 bits").to_f64()),
        TagType::ByteArray => Tag::ByteArray(parse_hex_bytes(text()).expect("hex")),
        TagType::String => Tag::String(text().to_owned()),
        TagType::List => {
            let elem = type_from_name(v["elem"].as_str().expect("list elem"));
            let items = v["items"].as_array().expect("list items");
            Tag::List {
                elem,
                items: items.iter().map(|i| tag_from_value(elem, i)).collect(),
            }
        }
        TagType::Compound => Tag::Compound(
            v.as_object()
                .expect("compound value")
                .iter()
                .map(|(k, child)| {
                    let ty = type_from_name(child["type"].as_str().expect("child type"));
                    (k.clone(), tag_from_value(ty, &child["value"]))
                })
                .collect(),
        ),
    }
}

/// The in-memory tree as vanilla holds it after a write: `writeTagContents`
/// stores `tagType = 1` in every empty list (NBTTagList.java:17). Our writer
/// takes `&Tag` and emits the same byte without mutating; the oracle's
/// `nbt_tree` line is taken after the write, so mirror the mutation here.
fn after_vanilla_write(tag: &Tag) -> Tag {
    match tag {
        Tag::List { elem, items } => Tag::List {
            elem: if items.is_empty() {
                TagType::Byte
            } else {
                *elem
            },
            items: items.iter().map(after_vanilla_write).collect(),
        },
        Tag::Compound(map) => Tag::Compound(
            map.iter()
                .map(|(k, v)| (k.clone(), after_vanilla_write(v)))
                .collect(),
        ),
        other => other.clone(),
    }
}

// ---------------------------------------------------------------------------
// Helpers.

struct Fixture {
    name: &'static str,
    scenario: Scenario,
    lines: Vec<TraceLine>,
}

fn load(name: &'static str) -> Fixture {
    let dir = fixture_dir("nbt", name);
    let scenario = Scenario::load(&dir).unwrap_or_else(|e| panic!("{name}: {e}"));
    let text = read_sidecar(&dir, "expected.jsonl").unwrap_or_else(|e| panic!("{name}: {e}"));
    let text = String::from_utf8(text).expect("expected.jsonl is UTF-8");
    let lines = parse_jsonl(&text).unwrap_or_else(|e| panic!("{name}: {e:?}"));
    Fixture {
        name,
        scenario,
        lines,
    }
}

impl Fixture {
    /// The expected lines, asserting the documented event order.
    fn expect_events(&self, events: &[&str]) {
        let got: Vec<&str> = self.lines.iter().map(TraceLine::e).collect();
        assert_eq!(got, events, "{}: event order", self.name);
    }

    fn line(&self, e: &str) -> &TraceLine {
        self.lines
            .iter()
            .find(|l| l.e() == e)
            .unwrap_or_else(|| panic!("{}: no `{e}` line", self.name))
    }

    /// `mc_golden::compare` on one expected line; panics with the
    /// first-difference report.
    fn assert_line(&self, actual: TraceLine, what: &str) {
        let expected = self.line(actual.e()).clone();
        if let Err(m) = compare(&[expected], &[actual]) {
            panic!("{}: {what}:\n{m}", self.name);
        }
    }

    /// The oracle's `nbt_bytes`, checked against its own `len`/`blake3`.
    fn oracle_bytes(&self) -> Vec<u8> {
        let line = self.line("nbt_bytes");
        let bytes = parse_hex_bytes(field(line, "bytes").as_str().expect("bytes")).expect("hex");
        assert_eq!(
            TraceLine::nbt_bytes(&bytes),
            *line,
            "{}: nbt_bytes self-consistent",
            self.name
        );
        bytes
    }

    fn byte_exact(&self) -> bool {
        BYTE_EXACT.contains(&self.name)
    }

    /// Our bytes against the oracle's `nbt_bytes`: exact for [`BYTE_EXACT`],
    /// otherwise same length, different order, same parsed tree.
    fn check_bytes(&self, ours: &[u8], oracle: &[u8]) {
        if self.byte_exact() {
            self.assert_line(TraceLine::nbt_bytes(ours), "nbt_bytes (exact)");
        } else {
            assert_eq!(ours.len(), oracle.len(), "{}: nbt_bytes len", self.name);
            assert_ne!(
                ours, oracle,
                "{}: bytes now match exactly; add it to BYTE_EXACT",
                self.name
            );
            assert_eq!(
                mc_nbt::read_named(ours).unwrap(),
                mc_nbt::read_named(oracle).unwrap(),
                "{}: our bytes and the oracle's parse to different trees",
                self.name
            );
        }
    }

    /// Gzip round trip (README line 4 of `nbt_write`): write gzipped, read
    /// back, re-write uncompressed; the result must equal `uncompressed`.
    fn check_gzip(&self, name: &str, tree: &Tag, uncompressed: &[u8]) {
        let gz = mc_nbt::write_gzip(name, tree).unwrap();
        let (n, t) = mc_nbt::read_gzip(&gz).unwrap();
        let again = mc_nbt::write_named(&n, &t).unwrap();
        assert_eq!(again, uncompressed, "{}: gzip round trip", self.name);
        if self.lines.iter().any(|l| l.e() == "nbt_gzip") {
            if self.byte_exact() {
                self.assert_line(TraceLine::nbt_gzip(&again), "nbt_gzip (exact)");
            } else {
                let expected = field(self.line("nbt_gzip"), "len").as_u64();
                assert_eq!(
                    Some(again.len() as u64),
                    expected,
                    "{}: nbt_gzip len",
                    self.name
                );
            }
        }
    }
}

/// A field of a trace line, `Null` when absent.
fn field<'a>(line: &'a TraceLine, key: &str) -> &'a Value {
    line.get(key).unwrap_or(&Value::Null)
}

// ---------------------------------------------------------------------------
// `nbt_write`.

fn check_nbt_write(name: &'static str) {
    let fx = load(name);
    fx.expect_events(&["nbt_bytes", "nbt_tree", "nbt_roundtrip", "nbt_gzip"]);
    let ScenarioKind::NbtWrite { root } = &fx.scenario.kind else {
        panic!("{name}: not nbt_write")
    };
    let root_name = root.name.as_deref().expect("root name");
    let tree = tag_from_node(root);
    let expected_tree = &field(fx.line("nbt_tree"), "root");
    let roundtrip = fx.line("nbt_roundtrip");
    // What the oracle's bytes re-read to: the tree itself, or `reread` when
    // the write changed it (NaN payloads).
    let expected_reread = if field(roundtrip, "equal").as_bool() == Some(true) {
        expected_tree
    } else {
        field(roundtrip, "reread")
    };

    // 1. Reading: the oracle's bytes parse to the oracle's re-read tree.
    let oracle = fx.oracle_bytes();
    let (oracle_name, oracle_tree) = mc_nbt::read_named(&oracle).unwrap();
    assert_eq!(oracle_name, root_name);
    if let Err(m) = compare(
        &[TraceLine::nbt_tree(expected_reread.clone())],
        &[TraceLine::nbt_tree(canonical_root(
            &oracle_name,
            &oracle_tree,
        ))],
    ) {
        panic!("{name}: reading the oracle bytes:\n{m}");
    }

    // 2. The scenario tree, as held after a write, is the oracle's `nbt_tree`.
    let held = after_vanilla_write(&tree);
    fx.assert_line(
        TraceLine::nbt_tree(canonical_root(root_name, &held)),
        "nbt_tree from the scenario",
    );

    // 3. Our write re-read: `nbt_roundtrip`, `equal:false` + `reread` exactly
    //    when the oracle's is (NaN canonicalisation on write).
    let ours = mc_nbt::write_named(root_name, &tree).unwrap();
    let (reread_name, reread) = mc_nbt::read_named(&ours).unwrap();
    assert_eq!(reread_name, root_name);
    let equal = reread == held;
    let mut line = TraceLine::nbt_roundtrip(equal);
    if !equal {
        line = line.with("reread", canonical_root(&reread_name, &reread));
    }
    fx.assert_line(line, "nbt_roundtrip");
    assert_eq!(
        reread, oracle_tree,
        "{name}: our bytes vs oracle bytes, parsed"
    );

    // 4. `nbt_bytes`.
    fx.check_bytes(&ours, &oracle);
    // Writing the tree parsed from the oracle's bytes gives our bytes again,
    // and that write round-trips.
    let from_oracle = mc_nbt::write_named(&oracle_name, &oracle_tree).unwrap();
    assert_eq!(from_oracle, ours, "{name}: rewrite of the oracle tree");

    // 5. `nbt_gzip`.
    fx.check_gzip(root_name, &tree, &ours);
}

#[test]
fn nbt_empty_root() {
    check_nbt_write("empty-root");
}

#[test]
fn nbt_primitives() {
    check_nbt_write("primitives");
}

#[test]
fn nbt_nan_payloads() {
    check_nbt_write("nan-payloads");
}

#[test]
fn nbt_level_dat_shape() {
    check_nbt_write("level-dat-shape");
}

// ---------------------------------------------------------------------------
// `nbt_read`.

#[test]
fn nbt_testworld_level_dat() {
    let fx = load("testworld-level-dat");
    fx.expect_events(&["nbt_file", "nbt_bytes", "nbt_tree", "nbt_roundtrip"]);
    let ScenarioKind::NbtRead { file, compressed } = &fx.scenario.kind else {
        panic!("not nbt_read")
    };
    let dir = fixture_dir("nbt", fx.name);
    let raw = read_sidecar(&dir, file).unwrap();

    // 1. `nbt_file`: the file and its decompressed content.
    let (uncompressed, (name, tree)) = match compressed.as_str() {
        "gzip" => (
            mc_nbt::decompress_gzip(&raw).unwrap(),
            mc_nbt::read_gzip(&raw).unwrap(),
        ),
        "none" => (raw.clone(), mc_nbt::read_named(&raw).unwrap()),
        other => panic!("unknown compression {other:?}"),
    };
    fx.assert_line(
        TraceLine::nbt_file(file, compressed, &raw, &uncompressed),
        "nbt_file",
    );

    // 2. `nbt_tree`: the file parsed.
    fx.assert_line(
        TraceLine::nbt_tree(canonical_root(&name, &tree)),
        "nbt_tree of the file",
    );

    // 3. `nbt_roundtrip`: our rewrite re-reads to the same tree.
    let ours = mc_nbt::write_named(&name, &tree).unwrap();
    let (n2, reread) = mc_nbt::read_named(&ours).unwrap();
    let equal = n2 == name && reread == tree;
    fx.assert_line(TraceLine::nbt_roundtrip(equal), "nbt_roundtrip");

    // 4. `nbt_bytes`: the oracle's rewrite, which equals the gunzipped file
    //    (README); ours has the same length and parses identically.
    let oracle = fx.oracle_bytes();
    assert_eq!(oracle, uncompressed, "oracle rewrite == gunzipped file");
    fx.check_bytes(&ours, &oracle);

    // Gzip framing round trip on our side as well.
    fx.check_gzip(&name, &tree, &ours);
}

/// Every fixture directory under `tests/golden/nbt` is checked above.
#[test]
fn every_nbt_fixture_is_covered() {
    let mut names: Vec<String> = all_fixture_dirs()
        .unwrap()
        .into_iter()
        .filter(|d| d.parent().and_then(|p| p.file_name()) == Some("nbt".as_ref()))
        .map(|d| d.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, FIXTURES);
}

// ---------------------------------------------------------------------------
// The canonical-JSON helpers themselves.

#[test]
fn canonical_shape_matches_readme() {
    let mut inner = Tag::compound();
    inner
        .as_compound_mut()
        .unwrap()
        .insert("id".into(), Tag::Short(1));
    let mut root = Tag::compound();
    let map = root.as_compound_mut().unwrap();
    map.insert("f".into(), Tag::Float(1.0));
    map.insert("b".into(), Tag::ByteArray(vec![0, 255]));
    map.insert(
        "inv".into(),
        Tag::List {
            elem: TagType::Compound,
            items: vec![inner],
        },
    );
    let v = canonical_root("R", &root);
    assert_eq!(
        v,
        json!({"type":"compound","name":"R","value":{
            "b":{"type":"byte_array","value":"00ff"},
            "f":{"type":"float","value":"0x3f800000"},
            "inv":{"type":"list","value":{"elem":"compound","items":[{"id":{"type":"short","value":1}}]}}}})
    );
    // And back through the scenario reader.
    let node: NbtNode = serde_json::from_value(v).unwrap();
    assert_eq!(tag_from_node(&node), root);
}
