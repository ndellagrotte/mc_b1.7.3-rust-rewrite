//! McRegion container (`save::region`, `save::region_cache`; WP-1.9a).
//!
//! 1. The owner's `testWorld` region (`tests/golden/save/testworld`): chunk
//!    (0, −4) decodes to the oracle's `chunk_nbt` tree, the header parses to its
//!    `region_header` line, and every present chunk decodes.
//! 2. Write/re-read on fresh regions: the sector allocation of RegionFile.java
//!    step by step (grow, first-fit reuse, in-place rewrite, shrink, too large,
//!    the `(len + 5) / 4096 + 1` boundary), `sizeDelta`, timestamps, and after
//!    every write an independent parse of the stored bytes from the Java layout
//!    (no overlaps, file length a sector multiple, every chunk equal to what was
//!    written), plus a reopen from storage.
//! 3. Vanilla behaviour on odd files (the constructor's padding bug, short and
//!    empty files, entries past the end, version bytes), `RegionFileCache`.
//! 4. Cross-read: the vanilla `RegionFile` reads a Rust-written region
//!    (`harness.RegionCheck`), when the oracle is built; skipped otherwise.

use std::collections::BTreeMap;

use mc_core::JavaRandom;
use mc_golden::{Bits32, Bits64, blake3_hex, fixture_dir, hex_bytes, read_sidecar, workspace_root};
use mc_nbt::{Tag, TagType};
use mc_sim::save::{
    MemoryStorage, RegionError, RegionFile, RegionFileCache, RegionWrite, Storage, region_file_path,
};
use serde_json::{Map, Value, json};

const SECTOR: usize = 4096;
const NOW: i64 = 1_791_246_385_099; // ms; timestamp 1_791_246_385

// ---------------------------------------------------------------------------
// Canonical tree JSON (tests/golden/README.md, "Canonical tree"), as
// crates/mc-nbt/tests/golden_nbt.rs builds it (not public there).

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

fn canonical_root(name: &str, tag: &Tag) -> Value {
    let mut node = canonical_node(tag);
    node["name"] = name.into();
    node
}

fn canonical_node(tag: &Tag) -> Value {
    json!({"type": type_name(tag.tag_type()), "value": canonical_value(tag)})
}

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
// Independent reader of the Java layout (RegionFile.java:64-80, :118-150).

#[derive(Debug)]
struct Entry {
    sector: usize,
    count: usize,
    timestamp: i32,
    /// Decompressed payload.
    payload: Vec<u8>,
    /// Compressed length.
    clen: usize,
}

fn be_int(b: &[u8], pos: usize) -> i32 {
    i32::from_be_bytes(b[pos..pos + 4].try_into().unwrap())
}

/// Parses every non-zero offset entry of `bytes`, checking the stored length
/// fits its sectors and the version byte is 2 (zlib, what vanilla writes).
fn parse_region(bytes: &[u8]) -> BTreeMap<usize, Entry> {
    assert!(bytes.len() >= 2 * SECTOR, "header missing");
    let mut out = BTreeMap::new();
    for i in 0..1024 {
        let loc = be_int(bytes, i * 4);
        if loc == 0 {
            continue;
        }
        let (sector, count) = ((loc >> 8) as usize, (loc & 255) as usize);
        let start = sector * SECTOR;
        assert!(
            start + count * SECTOR <= bytes.len(),
            "entry {i} past the end"
        );
        let length = be_int(bytes, start) as usize;
        assert!(
            length <= count * SECTOR,
            "entry {i}: length {length} > {count} sectors"
        );
        assert_eq!(bytes[start + 4], 2, "entry {i}: version");
        let body = &bytes[start + 5..start + 4 + length];
        out.insert(
            i,
            Entry {
                sector,
                count,
                timestamp: be_int(bytes, SECTOR + i * 4),
                payload: mc_nbt::decompress_zlib(body).unwrap(),
                clen: length - 1,
            },
        );
    }
    out
}

/// What the region must hold after every write: `model` maps index
/// `x + z * 32` to the uncompressed payload last stored.
fn validate(
    region: &RegionFile,
    storage: &mut MemoryStorage,
    model: &BTreeMap<usize, Vec<u8>>,
) -> BTreeMap<usize, Entry> {
    let path = region.file_name().to_string();
    let stored = storage.read(&path).unwrap().expect("region file stored");
    assert_eq!(stored, region.bytes(), "storage holds the in-memory file");
    assert_eq!(stored.len() % SECTOR, 0, "file length is a sector multiple");
    let entries = parse_region(&stored);
    assert_eq!(
        entries.keys().copied().collect::<Vec<_>>(),
        model.keys().copied().collect::<Vec<_>>()
    );
    let sectors = stored.len() / SECTOR;
    let mut used = vec![false; sectors];
    used[0] = true;
    used[1] = true;
    for (i, e) in &entries {
        assert_eq!(&e.payload, &model[i], "chunk {i}");
        // src: RegionFile.java:170
        assert_eq!(e.count, (e.clen + 5) / SECTOR + 1, "chunk {i} sector count");
        assert!(e.sector >= 2);
        for (s, u) in used.iter_mut().enumerate().skip(e.sector).take(e.count) {
            assert!(!*u, "sector {s} used twice (chunk {i})");
            *u = true;
        }
        assert_eq!(e.timestamp, (NOW / 1000) as i32);
        let (x, z) = ((i & 31) as i32, (i >> 5) as i32);
        assert_eq!(
            region.get_chunk_data_input_stream(x, z).unwrap().as_ref(),
            Some(&model[i])
        );
        assert!(region.has_chunk(x, z));
    }
    // Timestamps exactly where chunks are.
    for i in 0..1024 {
        assert_eq!(region.chunk_timestamps()[i] != 0, entries.contains_key(&i));
    }
    // The in-memory free list is the complement of the used sectors.
    assert_eq!(
        region.sector_free(),
        used.iter().map(|u| !u).collect::<Vec<_>>()
    );
    // Reopening from storage gives the same tables and free list.
    let mut copy = storage.clone();
    let again = RegionFile::open(&mut copy, &path).unwrap();
    assert_eq!(copy, *storage, "opening an aligned file writes nothing");
    assert_eq!(again.offsets(), region.offsets());
    assert_eq!(again.chunk_timestamps(), region.chunk_timestamps());
    assert_eq!(again.sector_free(), region.sector_free());
    entries
}

/// `n` incompressible bytes: deflate stores them, so the compressed length is
/// `n` plus a few bytes and the sector count is predictable.
fn noise(rng: &mut JavaRandom, n: usize) -> Vec<u8> {
    (0..n).map(|_| rng.next_int() as u8).collect()
}

/// A payload needing exactly `k` sectors once compressed.
fn payload_for(rng: &mut JavaRandom, k: usize) -> Vec<u8> {
    let p = noise(rng, (k - 1) * SECTOR + 1500);
    let clen = mc_nbt::compress_zlib(&p).len();
    assert_eq!((clen + 5) / SECTOR + 1, k, "payload sizing");
    p
}

/// Writes `payload` at local `(x, z)` through `getChunkDataOutputStream`.
fn put(
    region: &mut RegionFile,
    storage: &mut MemoryStorage,
    model: &mut BTreeMap<usize, Vec<u8>>,
    x: i32,
    z: i32,
    payload: Vec<u8>,
) -> RegionWrite {
    let mut stream = region.get_chunk_data_output_stream(x, z).unwrap();
    std::io::Write::write_all(&mut stream, &payload).unwrap();
    let outcome = stream.close(region, storage, NOW).unwrap();
    if outcome != RegionWrite::TooLarge {
        model.insert((x + z * 32) as usize, payload);
    }
    outcome
}

fn loc(region: &RegionFile, x: i32, z: i32) -> (i32, i32) {
    let v = region.offsets()[(x + z * 32) as usize];
    (v >> 8, v & 255)
}

/// A zlib stream with one stored block (RFC 1950/1951), built by hand to hit
/// an exact compressed length: `data.len() + 11` bytes.
fn stored_zlib(data: &[u8]) -> Vec<u8> {
    let n = u16::try_from(data.len()).unwrap();
    let mut out = vec![0x78, 0x01, 0x01];
    out.extend_from_slice(&n.to_le_bytes());
    out.extend_from_slice(&(!n).to_le_bytes());
    out.extend_from_slice(data);
    let (mut a, mut b) = (1u32, 0u32);
    for &c in data {
        a = (a + u32::from(c)) % 65521;
        b = (b + a) % 65521;
    }
    out.extend_from_slice(&((b << 16) | a).to_be_bytes());
    out
}

// ---------------------------------------------------------------------------
// 1. testWorld

fn expected_line(e: &str, pick: impl Fn(&Value) -> bool) -> Value {
    let path = fixture_dir("save", "testworld").join("expected.jsonl");
    let text = std::fs::read_to_string(&path).unwrap();
    text.lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .find(|v| v["e"] == e && pick(v))
        .unwrap_or_else(|| panic!("no {e} line in {}", path.display()))
}

fn testworld_storage() -> (MemoryStorage, Vec<u8>) {
    let dir = fixture_dir("save", "testworld").join("world/region");
    let bytes = read_sidecar(dir, "r.0.-1.mcr").unwrap();
    let mut storage = MemoryStorage::default();
    storage.write("world/region/r.0.-1.mcr", &bytes).unwrap();
    (storage, bytes)
}

#[test]
fn testworld_chunk_matches_oracle_tree() {
    let (mut storage, original) = testworld_storage();
    let mut cache = RegionFileCache::new();
    let nbt = cache
        .get_chunk_input_stream(&mut storage, "world", 0, -4)
        .unwrap()
        .expect("chunk (0,-4) present");
    let (name, tag) = mc_nbt::read_named(&nbt).unwrap();
    let line = expected_line("chunk_nbt", |v| v["x"] == 0 && v["z"] == -4);
    assert!(
        canonical_root(&name, &tag) == line["root"],
        "chunk (0,-4) tree differs from the oracle's"
    );
    // Opening and reading an aligned file changes nothing.
    assert_eq!(
        storage.read("world/region/r.0.-1.mcr").unwrap().unwrap(),
        original
    );
}

#[test]
fn testworld_header_matches_oracle() {
    let (mut storage, original) = testworld_storage();
    let region = RegionFile::open(&mut storage, "world/region/r.0.-1.mcr").unwrap();
    let line = expected_line("region_header", |v| v["name"] == "r.0.-1.mcr");
    let present: Vec<usize> = (0..1024).filter(|&i| region.offsets()[i] != 0).collect();
    assert_eq!(json!(present), line["present"]);
    assert_eq!(json!(present.len()), line["count"]);
    let timestamps_match =
        (0..1024).all(|i| (region.offsets()[i] != 0) == (region.chunk_timestamps()[i] != 0));
    assert_eq!(json!(timestamps_match), line["timestamps_match"]);
    // `dbg` is not compared for oracle-written regions (deflate-dependent),
    // but for an input file it is a property of the file itself.
    assert_eq!(json!(original.len()), line["dbg"]["len"]);
    assert_eq!(json!(region.offsets().to_vec()), line["dbg"]["locations"]);
    assert_eq!(region.size_delta(), 0);

    // Every present chunk decodes, sits where its entry says, and the file
    // passes the independent reader.
    let entries = parse_region(&original);
    assert_eq!(entries.len(), present.len());
    for &i in &present {
        let (x, z) = ((i & 31) as i32, (i >> 5) as i32);
        let nbt = region.get_chunk_data_input_stream(x, z).unwrap().unwrap();
        assert_eq!(nbt, entries[&i].payload);
        let (_, tag) = mc_nbt::read_named(&nbt).unwrap();
        let level = tag.get("Level").unwrap();
        assert_eq!(level.get("xPos"), Some(&Tag::Int(x)));
        assert_eq!(level.get("zPos"), Some(&Tag::Int(z - 32)));
    }
}

// ---------------------------------------------------------------------------
// 2. Write / re-read

#[test]
fn fresh_region_allocation_steps() {
    let mut storage = MemoryStorage::default();
    let path = "w/region/r.0.0.mcr";
    let mut model = BTreeMap::new();
    let mut rng = JavaRandom::new(1);

    // src: RegionFile.java:36-46: a new file is the empty 8 KiB header.
    let mut region = RegionFile::open(&mut storage, path).unwrap();
    assert_eq!(storage.read(path).unwrap().unwrap(), vec![0u8; 2 * SECTOR]);
    assert_eq!(region.take_size_delta(), 8192);
    assert_eq!(region.take_size_delta(), 0);
    assert_eq!(region.sector_free(), &[false, false]);
    validate(&region, &mut storage, &model);

    // A: 1 sector, appended at 2.
    let a1 = payload_for(&mut rng, 1);
    assert_eq!(
        put(&mut region, &mut storage, &mut model, 0, 0, a1),
        RegionWrite::Grow
    );
    assert_eq!(loc(&region, 0, 0), (2, 1));
    assert_eq!(region.take_size_delta(), 4096);
    validate(&region, &mut storage, &model);

    // B: 1 sector at 3.
    let b1 = payload_for(&mut rng, 1);
    assert_eq!(
        put(&mut region, &mut storage, &mut model, 1, 0, b1),
        RegionWrite::Grow
    );
    assert_eq!(loc(&region, 1, 0), (3, 1));
    validate(&region, &mut storage, &model);

    // A again, same sector count: in place.
    let a1b = payload_for(&mut rng, 1);
    assert_eq!(
        put(&mut region, &mut storage, &mut model, 0, 0, a1b),
        RegionWrite::Rewrite
    );
    assert_eq!(loc(&region, 0, 0), (2, 1));
    assert_eq!(region.take_size_delta(), 4096);
    validate(&region, &mut storage, &model);

    // A grows to 3: sector 2 is freed but too short, so 3 new sectors at 4.
    let a3 = payload_for(&mut rng, 3);
    assert_eq!(
        put(&mut region, &mut storage, &mut model, 0, 0, a3),
        RegionWrite::Grow
    );
    assert_eq!(loc(&region, 0, 0), (4, 3));
    assert_eq!(region.take_size_delta(), 3 * 4096);
    assert!(region.sector_free()[2]);
    validate(&region, &mut storage, &model);

    // C: 1 sector reuses the hole at 2.
    let c1 = payload_for(&mut rng, 1);
    assert_eq!(
        put(&mut region, &mut storage, &mut model, 5, 7, c1),
        RegionWrite::Reuse
    );
    assert_eq!(loc(&region, 5, 7), (2, 1));
    assert_eq!(region.take_size_delta(), 0);
    validate(&region, &mut storage, &model);

    // A shrinks to 1: its sectors are freed first, first fit is 4 again.
    let a1c = payload_for(&mut rng, 1);
    assert_eq!(
        put(&mut region, &mut storage, &mut model, 0, 0, a1c),
        RegionWrite::Reuse
    );
    assert_eq!(loc(&region, 0, 0), (4, 1));
    assert_eq!(&region.sector_free()[5..7], &[true, true]);
    validate(&region, &mut storage, &model);

    // D: 2 sectors fill the run 5..7.
    let d2 = payload_for(&mut rng, 2);
    assert_eq!(
        put(&mut region, &mut storage, &mut model, 31, 31, d2),
        RegionWrite::Reuse
    );
    assert_eq!(loc(&region, 31, 31), (5, 2));
    validate(&region, &mut storage, &model);

    // E: 2 sectors, no hole left: appended at 7.
    let e2 = payload_for(&mut rng, 2);
    assert_eq!(
        put(&mut region, &mut storage, &mut model, 0, 31, e2),
        RegionWrite::Grow
    );
    assert_eq!(loc(&region, 0, 31), (7, 2));
    assert_eq!(region.bytes().len(), 9 * SECTOR);
    validate(&region, &mut storage, &model);

    // B to 2 sectors: frees 3, which alone is too short; the first-fit scan
    // restarts at the next free sector and finds nothing: appended at 9.
    let b2 = payload_for(&mut rng, 2);
    assert_eq!(
        put(&mut region, &mut storage, &mut model, 1, 0, b2),
        RegionWrite::Grow
    );
    assert_eq!(loc(&region, 1, 0), (9, 2));
    validate(&region, &mut storage, &model);

    // Too large (>= 256 sectors): nothing changes, no timestamp.
    let before = (region.clone(), storage.clone());
    let huge = noise(&mut rng, 255 * SECTOR);
    assert_eq!(
        put(&mut region, &mut storage, &mut model, 3, 3, huge),
        RegionWrite::TooLarge
    );
    assert_eq!((region.clone(), storage.clone()), before);
    assert!(!region.has_chunk(3, 3));

    // 255 sectors is the largest that is stored.
    let big = payload_for(&mut rng, 255);
    assert_eq!(
        put(&mut region, &mut storage, &mut model, 3, 3, big),
        RegionWrite::Grow
    );
    assert_eq!(loc(&region, 3, 3), (11, 255));
    assert_eq!(region.take_size_delta(), (2 + 2 + 255) * 4096);
    validate(&region, &mut storage, &model);
}

#[test]
fn sector_count_boundary() {
    // src: RegionFile.java:170: `(len + 5) / 4096 + 1`.
    let mut storage = MemoryStorage::default();
    let mut region = RegionFile::open(&mut storage, "r.0.0.mcr").unwrap();
    let mut model = BTreeMap::new();
    let mut rng = JavaRandom::new(7);
    // 4090 compressed bytes: the entry is 4095 bytes, one sector.
    let p = noise(&mut rng, 4079);
    let z = stored_zlib(&p);
    assert_eq!(z.len(), 4090);
    region.write(&mut storage, 0, 0, &z, NOW).unwrap();
    model.insert(0, p);
    assert_eq!(loc(&region, 0, 0), (2, 1));
    validate(&region, &mut storage, &model);
    // vanilla quirk: 4091 bytes make an entry of exactly 4096, which would
    // fit one sector, but take two.
    let p = noise(&mut rng, 4080);
    let z = stored_zlib(&p);
    assert_eq!(z.len(), 4091);
    assert_eq!(
        region.write(&mut storage, 1, 0, &z, NOW).unwrap(),
        RegionWrite::Grow
    );
    model.insert(1, p);
    assert_eq!(loc(&region, 1, 0), (3, 2));
    validate(&region, &mut storage, &model);
}

#[test]
fn random_write_sequence_round_trips() {
    let mut storage = MemoryStorage::default();
    let mut region = RegionFile::open(&mut storage, "w/region/r.-1.2.mcr").unwrap();
    let mut model = BTreeMap::new();
    let mut rng = JavaRandom::new(-42);
    let mut grown = 0;
    for step in 0..300 {
        let x = rng.next_int_bounded(6);
        let z = rng.next_int_bounded(6) * 5;
        // Mostly small, sometimes many sectors; half compressible (NBT-like
        // runs of zeros), half not.
        let n = match rng.next_int_bounded(10) {
            0 => rng.next_int_bounded(60_000) as usize,
            1..=4 => rng.next_int_bounded(9000) as usize,
            _ => rng.next_int_bounded(500) as usize,
        };
        let payload = if rng.next_boolean() {
            noise(&mut rng, n)
        } else {
            let mut v = vec![0u8; n];
            for b in v.iter_mut().step_by(97) {
                *b = rng.next_int() as u8;
            }
            v
        };
        let outcome = put(&mut region, &mut storage, &mut model, x, z, payload);
        assert_ne!(outcome, RegionWrite::TooLarge);
        if outcome == RegionWrite::Grow {
            grown += 1;
        }
        if step % 10 == 9 {
            validate(&region, &mut storage, &model);
        }
    }
    validate(&region, &mut storage, &model);
    let size_delta = region.take_size_delta();
    assert_eq!(
        size_delta as usize,
        region.bytes().len(),
        "8 KiB header + growth"
    );
    assert!(grown > 0);
}

// ---------------------------------------------------------------------------
// 3. Odd files

/// A region at `path` holding 1-sector chunks at the given indices.
fn region_with(storage: &mut MemoryStorage, path: &str, indices: &[usize]) -> RegionFile {
    let mut region = RegionFile::open(storage, path).unwrap();
    let mut rng = JavaRandom::new(3);
    let mut model = BTreeMap::new();
    for &i in indices {
        let p = payload_for(&mut rng, 1);
        put(
            &mut region,
            storage,
            &mut model,
            (i & 31) as i32,
            (i >> 5) as i32,
            p,
        );
    }
    region
}

#[test]
fn constructor_padding_bug_zeroes_the_header() {
    let mut storage = MemoryStorage::default();
    let path = "r.0.0.mcr";
    region_with(&mut storage, path, &[0, 30, 100]);
    let mut bytes = storage.read(path).unwrap().unwrap();
    assert_eq!(bytes.len(), 5 * SECTOR);
    bytes.extend_from_slice(&[0xAB; 100]);
    storage.write(path, &bytes).unwrap();

    let mut region = RegionFile::open(&mut storage, path).unwrap();
    // vanilla bug (RegionFile.java:48-52): 100 zero bytes at offset 0, the
    // entries 0..25 are gone, the length stays unaligned.
    let after = storage.read(path).unwrap().unwrap();
    assert_eq!(after.len(), 5 * SECTOR + 100);
    assert!(after[..100].iter().all(|&b| b == 0));
    assert_eq!(after[100..], bytes[100..]);
    assert!(!region.has_chunk(0, 0));
    assert!(region.has_chunk(30, 0));
    assert!(region.has_chunk(4, 3));
    assert_eq!(region.sector_free(), &[false, false, true, false, false]);
    assert!(region.get_chunk_data_input_stream(30, 0).unwrap().is_some());

    // Growth appends after the unaligned end but places the chunk at
    // `sectorFree.size()`, over the 100 trailing bytes.
    let mut rng = JavaRandom::new(9);
    let p = payload_for(&mut rng, 2);
    let mut model = BTreeMap::new();
    // Sector 2 (chunk 0's, now free) is a 1-sector hole: too short.
    assert_eq!(
        put(&mut region, &mut storage, &mut model, 7, 7, p.clone()),
        RegionWrite::Grow
    );
    assert_eq!(loc(&region, 7, 7), (5, 2));
    assert_eq!(region.bytes().len(), 7 * SECTOR + 100);
    assert_eq!(region.get_chunk_data_input_stream(7, 7).unwrap(), Some(p));
    assert_eq!(region.take_size_delta(), 2 * 4096);
}

#[test]
fn short_and_empty_files() {
    let mut storage = MemoryStorage::default();
    // Empty: becomes the 8 KiB header.
    storage.write("a.mcr", &[]).unwrap();
    let mut r = RegionFile::open(&mut storage, "a.mcr").unwrap();
    assert_eq!(storage.read("a.mcr").unwrap().unwrap(), vec![0; 8192]);
    assert_eq!(r.take_size_delta(), 8192);
    // Under 4 KiB: the header overwrites it.
    storage.write("b.mcr", &[0xFF; 100]).unwrap();
    RegionFile::open(&mut storage, "b.mcr").unwrap();
    assert_eq!(storage.read("b.mcr").unwrap().unwrap(), vec![0; 8192]);
    // 4096..8191 bytes: the Java throws IndexOutOfBoundsException from
    // `sectorFree.set(1, false)`, after the padding loop has run.
    storage.write("c.mcr", &[0xFF; 4096]).unwrap();
    assert!(matches!(
        RegionFile::open(&mut storage, "c.mcr"),
        Err(RegionError::Corrupt(_))
    ));
    storage.write("d.mcr", &[0xFF; 5000]).unwrap();
    assert!(matches!(
        RegionFile::open(&mut storage, "d.mcr"),
        Err(RegionError::Corrupt(_))
    ));
    let d = storage.read("d.mcr").unwrap().unwrap();
    assert!(d[..5000 - 4096].iter().all(|&b| b == 0));
    assert!(d[5000 - 4096..].iter().all(|&b| b == 0xFF));
}

/// An 8 KiB header with `entries` (index, value) and `body` from sector 2.
fn raw_region(entries: &[(usize, i32)], body: &[u8]) -> Vec<u8> {
    let mut b = vec![0u8; 2 * SECTOR];
    for &(i, v) in entries {
        b[i * 4..i * 4 + 4].copy_from_slice(&v.to_be_bytes());
        b[SECTOR + i * 4..SECTOR + i * 4 + 4].copy_from_slice(&1i32.to_be_bytes());
    }
    b.extend_from_slice(body);
    let pad = b.len().next_multiple_of(SECTOR);
    b.resize(pad, 0);
    b
}

fn entry_bytes(version: u8, length: i32, body: &[u8]) -> Vec<u8> {
    let mut e = length.to_be_bytes().to_vec();
    e.push(version);
    e.extend_from_slice(body);
    e
}

#[test]
fn reader_null_cases_and_gzip() {
    let nbt = mc_nbt::write_named("", &Tag::compound()).unwrap();
    let gz = mc_nbt::compress_gzip(&nbt);
    let zl = mc_nbt::compress_zlib(&nbt);
    let mut body = Vec::new();
    // sector 2: gzip (version 1), which vanilla reads but never writes.
    body.extend(entry_bytes(1, gz.len() as i32 + 1, &gz));
    body.resize(SECTOR, 0);
    // sector 3: unknown version 3.
    body.extend(entry_bytes(3, zl.len() as i32 + 1, &zl));
    body.resize(2 * SECTOR, 0);
    // sector 4: length larger than its one sector.
    body.extend(entry_bytes(2, 4097, &zl));
    body.resize(3 * SECTOR, 0);
    // sector 5: zero length (the Java's `new byte[-1]`).
    body.extend(entry_bytes(2, 0, &[]));
    body.resize(4 * SECTOR, 0);
    // sector 6: version 1 but not gzip.
    body.extend(entry_bytes(1, zl.len() as i32 + 1, &zl));
    body.resize(5 * SECTOR, 0);
    // sector 7: zlib, corrupt deflate data.
    body.extend(entry_bytes(2, 5, &[0x78, 0x9c, 0xff, 0xff]));
    let bytes = raw_region(
        &[
            (0, 2 << 8 | 1),
            (1, 3 << 8 | 1),
            (2, 4 << 8 | 1),
            (3, 5 << 8 | 1),
            (4, 6 << 8 | 1),
            (5, 7 << 8 | 1),
            (6, 40 << 8 | 1), // past the end
            (7, -256),        // sector -1, 0 sectors: passes the constructor
        ],
        &body,
    );
    let mut storage = MemoryStorage::default();
    storage.write("r.mcr", &bytes).unwrap();
    let region = RegionFile::open(&mut storage, "r.mcr").unwrap();
    assert_eq!(region.get_chunk_data_input_stream(0, 0).unwrap(), Some(nbt));
    assert_eq!(region.get_chunk_data_input_stream(1, 0).unwrap(), None);
    assert_eq!(region.get_chunk_data_input_stream(2, 0).unwrap(), None);
    assert!(matches!(
        region.get_chunk_data_input_stream(3, 0),
        Err(RegionError::Corrupt(_))
    ));
    assert_eq!(region.get_chunk_data_input_stream(4, 0).unwrap(), None);
    assert!(matches!(
        region.get_chunk_data_input_stream(5, 0),
        Err(RegionError::Compression(_))
    ));
    assert_eq!(region.get_chunk_data_input_stream(6, 0).unwrap(), None);
    assert!(region.has_chunk(6, 0));
    // A negative sector: `seek` throws, null.
    assert_eq!(region.get_chunk_data_input_stream(7, 0).unwrap(), None);
    assert_eq!(region.get_chunk_data_input_stream(8, 0).unwrap(), None);
    // Out of bounds (RegionFile.java:247-249).
    for (x, z) in [(32, 0), (-1, 0), (0, 32), (0, -1)] {
        assert_eq!(region.get_chunk_data_input_stream(x, z).unwrap(), None);
        assert!(region.get_chunk_data_output_stream(x, z).is_none());
    }
    // Every sector named by an in-file entry is in use, 40 is ignored.
    assert_eq!(
        region.sector_free(),
        &[false, false, false, false, false, false, false, false]
    );

    // A negative sector with a non-zero count: the constructor's
    // `sectorFree.set(-1, false)` throws (RegionFile.java:70-72).
    let bytes = raw_region(&[(0, -255)], &[]);
    storage.write("n.mcr", &bytes).unwrap();
    assert!(matches!(
        RegionFile::open(&mut storage, "n.mcr"),
        Err(RegionError::Corrupt(_))
    ));
}

#[test]
fn entry_past_the_end_is_rewritten_in_place_then_fails() {
    // An entry of sector 40, one sector, in a 2-sector file.
    let bytes = raw_region(&[(0, 40 << 8 | 1)], &[]);
    let mut storage = MemoryStorage::default();
    storage.write("r.mcr", &bytes).unwrap();
    let mut region = RegionFile::open(&mut storage, "r.mcr").unwrap();
    let z = mc_nbt::compress_zlib(b"abc");
    // Same sector count: rewritten in place at sector 40, past the end; the
    // file is extended to just after the entry (unaligned), the free list is
    // not.
    assert_eq!(
        region.write(&mut storage, 0, 0, &z, NOW).unwrap(),
        RegionWrite::Rewrite
    );
    assert_eq!(region.bytes().len(), 40 * SECTOR + 5 + z.len());
    assert_eq!(region.sector_free().len(), 2);
    // Readers still see an entry past the known sectors: null.
    assert_eq!(region.get_chunk_data_input_stream(0, 0).unwrap(), None);
    // A different size frees sector 40: the Java's IndexOutOfBoundsException.
    let big = mc_nbt::compress_zlib(&noise(&mut JavaRandom::new(1), 5000));
    assert!(matches!(
        region.write(&mut storage, 0, 0, &big, NOW),
        Err(RegionError::Corrupt(_))
    ));
    assert!(matches!(
        region.write(&mut storage, 32, 0, &z, NOW),
        Err(RegionError::OutOfBounds { x: 32, z: 0 })
    ));
}

#[test]
fn timestamp_is_seconds_truncated_to_int() {
    let mut storage = MemoryStorage::default();
    let mut region = RegionFile::open(&mut storage, "r.mcr").unwrap();
    let z = mc_nbt::compress_zlib(b"abc");
    region.write(&mut storage, 2, 1, &z, 1_999).unwrap();
    assert_eq!(region.chunk_timestamps()[34], 1);
    // (int)(millis / 1000) wraps past 2038.
    region
        .write(&mut storage, 2, 1, &z, (1i64 << 31) * 1000 + 5_000)
        .unwrap();
    assert_eq!(region.chunk_timestamps()[34], i32::MIN + 5);
    let b = storage.read("r.mcr").unwrap().unwrap();
    assert_eq!(be_int(&b, SECTOR + 34 * 4), i32::MIN + 5);
}

// ---------------------------------------------------------------------------
// RegionFileCache

#[test]
fn cache_paths_reads_and_size_delta() {
    let mut storage = MemoryStorage::default();
    let mut cache = RegionFileCache::new();
    // Reading an absent chunk creates its region file (8 KiB header).
    assert_eq!(
        cache
            .get_chunk_input_stream(&mut storage, "w", -1, -33)
            .unwrap(),
        None
    );
    assert_eq!(
        storage.read("w/region/r.-1.-2.mcr").unwrap().unwrap(),
        vec![0; 8192]
    );
    assert_eq!(cache.len(), 1);

    let mut rng = JavaRandom::new(5);
    let p = payload_for(&mut rng, 3);
    assert_eq!(
        cache
            .write_chunk(&mut storage, "w", -1, -33, &p, NOW)
            .unwrap(),
        RegionWrite::Grow
    );
    let region = cache.get("w/region/r.-1.-2.mcr").unwrap();
    assert!(region.has_chunk(31, 31));
    assert_eq!(
        cache
            .get_chunk_input_stream(&mut storage, "w", -1, -33)
            .unwrap(),
        Some(p.clone())
    );
    // sizeDelta: the header, then the three sectors, then nothing.
    assert_eq!(
        cache.get_size_delta(&mut storage, "w", -1, -33).unwrap(),
        8192 + 3 * 4096
    );
    assert_eq!(cache.get_size_delta(&mut storage, "w", -1, -33).unwrap(), 0);

    // Streams: write, close through the cache.
    let mut s = cache
        .get_chunk_output_stream(&mut storage, "w", 40, 0)
        .unwrap()
        .unwrap();
    assert_eq!(s.region_path(), region_file_path("w", 40, 0));
    assert_eq!(s.chunk(), (8, 0));
    std::io::Write::write_all(&mut s, b"\x0a\x00\x00\x00").unwrap();
    cache
        .close_chunk_output_stream(&mut storage, s, NOW)
        .unwrap();
    assert_eq!(
        cache
            .get_chunk_input_stream(&mut storage, "w", 40, 0)
            .unwrap(),
        Some(b"\x0a\x00\x00\x00".to_vec())
    );

    // After a clear, a pending stream has no region; the data persists.
    let s = cache
        .get_chunk_output_stream(&mut storage, "w", 40, 0)
        .unwrap()
        .unwrap();
    cache.clear_region_file_references();
    assert!(cache.is_empty());
    assert!(matches!(
        cache.close_chunk_output_stream(&mut storage, s, NOW),
        Err(RegionError::StaleStream(_))
    ));
    assert_eq!(
        cache
            .get_chunk_input_stream(&mut storage, "w", -1, -33)
            .unwrap(),
        Some(p)
    );
}

#[test]
fn cache_clears_at_256() {
    let mut storage = MemoryStorage::default();
    let mut cache = RegionFileCache::new();
    for i in 0..256 {
        cache
            .create_or_load_region_file(&mut storage, "w", i * 32, 0)
            .unwrap();
    }
    assert_eq!(cache.len(), 256);
    // Cached: no clear.
    cache
        .create_or_load_region_file(&mut storage, "w", 0, 0)
        .unwrap();
    assert_eq!(cache.len(), 256);
    // The 257th region: everything is closed first (RegionFileCache.java:35-37).
    cache
        .create_or_load_region_file(&mut storage, "w", 0, 32)
        .unwrap();
    assert_eq!(cache.len(), 1);
    assert!(cache.get("w/region/r.0.1.mcr").is_some());
    assert_eq!(storage.list("w/region").unwrap().len(), 257);
}

// ---------------------------------------------------------------------------
// 4. Vanilla reads a Rust-written region

/// A chunk-shaped tree (ChunkLoader.storeChunkInCompound's main fields) with
/// incompressible block data so it spans several sectors.
fn chunk_tree(rng: &mut JavaRandom, x: i32, z: i32, blocks: usize) -> Vec<u8> {
    let mut level = Tag::compound();
    let m = level.as_compound_mut().unwrap();
    m.insert("xPos".into(), Tag::Int(x));
    m.insert("zPos".into(), Tag::Int(z));
    m.insert("LastUpdate".into(), Tag::Long(-1_234_567_890_123));
    m.insert("TerrainPopulated".into(), Tag::Byte(1));
    m.insert("Blocks".into(), Tag::ByteArray(noise(rng, blocks)));
    m.insert("Data".into(), Tag::ByteArray(vec![0; 16384]));
    m.insert("HeightMap".into(), Tag::ByteArray(noise(rng, 256)));
    m.insert(
        "Entities".into(),
        Tag::List {
            elem: TagType::Byte,
            items: vec![],
        },
    );
    let mut te = Tag::compound();
    let t = te.as_compound_mut().unwrap();
    t.insert("id".into(), Tag::String("Sign".into()));
    t.insert("Text1".into(), Tag::String("héllo \u{1F600}\0".into()));
    t.insert("f".into(), Tag::Float(f32::from_bits(0x7fc0_0000)));
    t.insert("d".into(), Tag::Double(-0.0));
    m.insert(
        "TileEntities".into(),
        Tag::List {
            elem: TagType::Compound,
            items: vec![te],
        },
    );
    let mut root = Tag::compound();
    root.as_compound_mut()
        .unwrap()
        .insert("Level".into(), level);
    mc_nbt::write_named("", &root).unwrap()
}

/// Host file for the JVM to read (test-only; D9 binds the simulation, not
/// its tests).
#[allow(clippy::disallowed_methods)]
fn write_host_file(dir: &std::path::Path, name: &str, bytes: &[u8]) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join(name), bytes).unwrap();
}

#[test]
fn vanilla_region_file_reads_rust_region() {
    let root = workspace_root();
    if !root
        .join("build/oracle/harness/RegionCheck.class")
        .is_file()
    {
        eprintln!(
            "skipped: build/oracle/harness/RegionCheck.class missing (run tools/oracle/build.sh)"
        );
        return;
    }
    let mut storage = MemoryStorage::default();
    let mut cache = RegionFileCache::new();
    let mut rng = JavaRandom::new(1234);
    let mut written: BTreeMap<(i32, i32), Vec<u8>> = BTreeMap::new();
    // Various sizes, rewrites that grow and shrink, holes left behind.
    let plan: &[(i32, i32, usize)] = &[
        (0, 0, 32768),
        (1, 0, 100),
        (31, 31, 60000),
        (0, 0, 2000),
        (5, 9, 32768),
        (1, 0, 70000),
        (17, 3, 10),
        (31, 31, 4000),
        (0, 0, 32768),
    ];
    for &(x, z, n) in plan {
        let nbt = chunk_tree(&mut rng, x, z, n);
        cache
            .write_chunk(&mut storage, "w", x, z, &nbt, NOW)
            .unwrap();
        written.insert((x, z), nbt);
    }
    let bytes = storage.read("w/region/r.0.0.mcr").unwrap().unwrap();
    parse_region(&bytes);

    let dir = mc_golden::target_dir().join("region-check");
    write_host_file(&dir, "r.0.0.mcr", &bytes);
    let out = std::process::Command::new("sh")
        .arg("-c")
        .arg(
            ". \"$ROOT/tools/oracle/env.sh\" && \
             java $JAVA_MATH_FLAGS -cp \"$ROOT/build/oracle:$CP\" harness.RegionCheck \"$1\"",
        )
        .arg("sh")
        .arg(dir.join("r.0.0.mcr"))
        .env("ROOT", &root)
        .output();
    let out = match out {
        Ok(o) => o,
        Err(e) => {
            eprintln!("skipped: cannot run sh/java: {e}");
            return;
        }
    };
    assert!(
        out.status.success(),
        "RegionCheck failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let lines: Vec<Value> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    // Index order x + z * 32.
    let mut expect: Vec<(&(i32, i32), &Vec<u8>)> = written.iter().collect();
    expect.sort_by_key(|((x, z), _)| x + z * 32);
    assert_eq!(lines.len(), expect.len(), "{lines:?}");
    for (line, ((x, z), nbt)) in lines.iter().zip(expect) {
        assert_eq!(line["x"], json!(x));
        assert_eq!(line["z"], json!(z));
        assert_eq!(line["len"], json!(nbt.len()), "chunk ({x},{z})");
        assert_eq!(line["blake3"], json!(blake3_hex(nbt)), "chunk ({x},{z})");
        let (name, tag) = mc_nbt::read_named(nbt).unwrap();
        assert!(
            canonical_root(&name, &tag) == line["root"],
            "chunk ({x},{z}): vanilla tree differs"
        );
    }
}
