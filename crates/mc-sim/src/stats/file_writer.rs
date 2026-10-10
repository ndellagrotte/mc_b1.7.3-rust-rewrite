//! `StatFileWriter` (StatFileWriter.java), the local file half of
//! `StatsSyncher` (StatsSyncher.java), `AchievementMap` (AchievementMap.java)
//! and `MD5String` (MD5String.java).
//!
//! # The stats file
//!
//! `Minecraft.statFileWriter` keeps two maps from stat to value: the
//! unsent changes ([`StatFileWriter::field_25101_b`]) and the totals
//! ([`StatFileWriter::field_25102_a`], which `hasAchievementUnlocked` and
//! `writeStat` read). The unsent map is what the client saves, as
//! `<mcDataDir>/stats/stats_<username lower case>_unsent.dat` in the format
//! of `func_27185_a` (a JSON-like text with `\r\n` line ends, the stat
//! changes, and an MD5 checksum over `guid,value,` pairs salted with
//! `"local"`), and reads back on start-up (`func_27177_a`, a JSON parse:
//! Argo in Java, `serde_json` here per PLAN_v4 §5's library note).
//!
//! Java iterates the maps' `HashMap` order, keyed by `StatBase` identity
//! hashes, which differs from run to run; the file's order is therefore
//! unspecified and the checksum is computed in whatever order the file has.
//! The port keys the maps by `statId` in a `BTreeMap` and writes ascending
//! ids (§3.4 rule 4: a new case of an iterated Java hash map, reported).
//!
//! # Scope
//!
//! `StatsSyncher` (StatsSyncher.java) is local file I/O, nothing online:
//! b1.7.3's "sync" writes and reads files under `stats/`. Here it runs
//! synchronously through [`Storage`] (D9), where Java starts a thread:
//!
//! - start-up ([`StatFileWriter::open`]): the relocation and renames, the
//!   unsent file's read, and `func_27418_a`'s countdown and receive thread
//!   (`ThreadStatSyncherReceive`: the received copy `stats_<user>.dat`,
//!   kept in [`StatsSyncher::field_27437_b`] for the first tick to merge);
//! - every tick (`Minecraft.runTick` calls `func_27178_d` first,
//!   Minecraft.java:949; [`StatFileWriter::func_27178_d`]): once the
//!   writer is dirty and the 100-tick countdown has run out, the send
//!   (`func_27424_a`, `ThreadStatSyncherSend`: the unsent map written to
//!   `stats_<user>_unsent.dat` through `.tmp` and `.old`), then
//!   `func_27425_c` (the countdowns, and the received map merged into the
//!   totals by `func_27180_b`);
//! - [`StatFileWriter::sync_stats`] (Minecraft.java:420, :460, :1270: a
//!   screen change, a world change and shutdown) writes the same file at
//!   once.
//!
//! The busy flag `field_27438_a` only guards the threads: a synchronous
//! send or receive has finished before anything reads it, so it is false
//! whenever [`StatsSyncher::func_27420_b`] runs. Vanilla bug (transcribed):
//! nothing clears the dirty flag `field_27189_c`, so from the first counted
//! stat on the file is written every 100 ticks, on the title screen too.
//! In singleplayer nothing writes `stats_<user>.dat` (the receive thread
//! writes it only from a map the receive itself sets, and it runs once,
//! before that map exists), but a file can be there: the constructor moves
//! any `stats_*.dat` from the data directory into `stats/`, or one is
//! copied in from another `.minecraft`; it is merged as in vanilla.

use std::collections::BTreeMap;

use mc_core::java_hash::{JavaHash, JavaHashMap};
use serde_json::Value;

use super::stat_base::StatBase;
use super::stat_list::StatList;
use crate::save::{Storage, StorageError};

// ---- AchievementMap ------------------------------------------------------------------

/// `AchievementMap`: `statId` to `statGuid`, read from the jar's
/// `/achievement/map.txt` (one `id,guid` per line). The text is the user's
/// (D8): the client reads it and passes it in.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AchievementMap {
    /// `guidMap` (private).
    guid_map: BTreeMap<i32, String>,
}

impl AchievementMap {
    /// The constructor's parse: each line split at `,` (Java
    /// `String.split`: trailing empty fields dropped), `parseInt` of the
    /// first field, the second as the guid. The first malformed line (no
    /// second field, a bad number) ends the parse; the lines before it are
    /// kept (the exception is caught outside the loop).
    // src: AchievementMap.java:12-28
    pub fn parse(text: &str) -> AchievementMap {
        let mut guid_map = BTreeMap::new();
        for var2 in java_read_lines(text) {
            let mut var3: Vec<&str> = var2.split(',').collect();
            while var3.last().is_some_and(|s| s.is_empty()) {
                var3.pop();
            }
            let (Some(first), Some(second)) = (var3.first(), var3.get(1)) else {
                break;
            };
            let Some(var4) = java_parse_int(first) else {
                break;
            };
            guid_map.insert(var4, second.to_string());
        }
        AchievementMap { guid_map }
    }

    /// `getGuid(id)`: the stat's `statGuid`, or null.
    // src: AchievementMap.java:30-32, StatBase.java:41
    pub fn get_guid(&self, var0: i32) -> Option<&str> {
        self.guid_map.get(&var0).map(String::as_str)
    }
}

// ---- MD5String ---------------------------------------------------------------------

/// `new MD5String(salt).getMD5String(s)`: MD5 of `salt + s`, as
/// `new BigInteger(1, digest).toString(16)` (lower-case hex without
/// leading zeros). Vanilla bug (transcribed): the digest covers
/// `getBytes()` (UTF-8 here) only up to `String.length()` (UTF-16 units),
/// so a non-ASCII string loses its last bytes; the stats checksum is ASCII.
// src: MD5String.java:10-23
pub fn md5_string(salt: Option<&str>, var1: &str) -> String {
    let var2 = format!("{}{var1}", salt.unwrap_or("null"));
    let bytes = var2.as_bytes();
    let n = var2.encode_utf16().count().min(bytes.len());
    let digest = md5(&bytes[..n]);
    let mut hex = String::with_capacity(32);
    for b in digest {
        hex.push_str(&format!("{b:02x}"));
    }
    let trimmed = hex.trim_start_matches('0');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

/// MD5 (RFC 1321), written from the RFC.
fn md5(input: &[u8]) -> [u8; 16] {
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    // K[i] = floor(abs(sin(i + 1)) * 2^32)
    const K: [u32; 64] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613,
        0xfd469501, 0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193,
        0xa679438e, 0x49b40821, 0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d,
        0x02441453, 0xd8a1e681, 0xe7d3fbc8, 0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
        0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a, 0xfffa3942, 0x8771f681, 0x6d9d6122,
        0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70, 0x289b7ec6, 0xeaa127fa,
        0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665, 0xf4292244,
        0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb,
        0xeb86d391,
    ];
    let mut a0: u32 = 0x67452301;
    let mut b0: u32 = 0xefcdab89;
    let mut c0: u32 = 0x98badcfe;
    let mut d0: u32 = 0x10325476;
    let mut msg = input.to_vec();
    let bit_len = (input.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_le_bytes());
    for chunk in msg.as_chunks::<64>().0 {
        let m: Vec<u32> = chunk
            .as_chunks::<4>()
            .0
            .iter()
            .map(|w| u32::from_le_bytes(*w))
            .collect();
        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let f = f.wrapping_add(a).wrapping_add(K[i]).wrapping_add(m[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(f.rotate_left(S[i]));
        }
        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }
    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&a0.to_le_bytes());
    out[4..8].copy_from_slice(&b0.to_le_bytes());
    out[8..12].copy_from_slice(&c0.to_le_bytes());
    out[12..16].copy_from_slice(&d0.to_le_bytes());
    out
}

// ---- Java text helpers ---------------------------------------------------------------

/// `BufferedReader.readLine()` over the whole text: lines end at `\n`,
/// `\r` or `\r\n`; a last line without an end is returned, an empty tail is
/// not.
fn java_read_lines(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let b = text.as_bytes();
    let (mut start, mut i) = (0, 0);
    while i < b.len() {
        match b[i] {
            b'\n' => {
                out.push(&text[start..i]);
                i += 1;
                start = i;
            }
            b'\r' => {
                out.push(&text[start..i]);
                i += if b.get(i + 1) == Some(&b'\n') { 2 } else { 1 };
                start = i;
            }
            _ => i += 1,
        }
    }
    if start < b.len() {
        out.push(&text[start..]);
    }
    out
}

/// `Integer.parseInt(s)` for ASCII input: an optional sign and decimal
/// digits in `i32` range. (Java also accepts non-ASCII Unicode digits.)
fn java_parse_int(s: &str) -> Option<i32> {
    s.parse::<i32>().ok()
}

/// `J_JsonNode.getText()` then `Integer.parseInt`: a string node's text,
/// or a number node's literal (an integer literal; `1.0` or `1e2` do not
/// parse). Anything else throws in Java. `serde_json` keeps no literal: an
/// integer literal is an `i64`/`u64` except `-0`, which it reads as the
/// float `-0.0`; that float is taken as `-0` (so `-0.0` and `-0e0`, which
/// Java rejects, are accepted as 0 here).
fn json_text_int(v: &Value) -> Option<i32> {
    match v {
        Value::String(s) => java_parse_int(s),
        Value::Number(n) => match n.as_i64() {
            Some(x) => i32::try_from(x).ok(),
            None => n
                .as_f64()
                .filter(|f| *f == 0.0 && f.is_sign_negative())
                .map(|_| 0),
        },
        _ => None,
    }
}

// ---- Argo's object field order --------------------------------------------------------

/// Argo's parse tree with each object's fields in document order,
/// duplicates included (`J_JsonObjectNodeBuilder.fieldBuilders` is a
/// `LinkedList`); the other values as `serde_json` reads them.
#[derive(Clone, Debug, PartialEq)]
enum ArgoNode {
    Object(Vec<(String, ArgoNode)>),
    Array(Vec<ArgoNode>),
    Leaf(Value),
}

impl ArgoNode {
    /// `getNode(key)` on an object: `getFields().get(key)`, so a repeated
    /// key gives its last value (`HashMap.put` replaces it); null otherwise.
    // src: J_JsonObject.java:14-16, J_JsonObjectNodeList.java:10-18
    fn get(&self, key: &str) -> Option<&ArgoNode> {
        match self {
            ArgoNode::Object(fields) => fields.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
}

impl<'de> serde::Deserialize<'de> for ArgoNode {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(ArgoVisitor)
    }
}

struct ArgoVisitor;

impl<'de> serde::de::Visitor<'de> for ArgoVisitor {
    type Value = ArgoNode;

    fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("a JSON value")
    }
    fn visit_bool<E>(self, v: bool) -> Result<ArgoNode, E> {
        Ok(ArgoNode::Leaf(Value::Bool(v)))
    }
    fn visit_i64<E>(self, v: i64) -> Result<ArgoNode, E> {
        Ok(ArgoNode::Leaf(Value::from(v)))
    }
    fn visit_u64<E>(self, v: u64) -> Result<ArgoNode, E> {
        Ok(ArgoNode::Leaf(Value::from(v)))
    }
    fn visit_f64<E>(self, v: f64) -> Result<ArgoNode, E> {
        Ok(ArgoNode::Leaf(Value::from(v)))
    }
    fn visit_str<E>(self, v: &str) -> Result<ArgoNode, E> {
        Ok(ArgoNode::Leaf(Value::String(v.to_string())))
    }
    fn visit_string<E>(self, v: String) -> Result<ArgoNode, E> {
        Ok(ArgoNode::Leaf(Value::String(v)))
    }
    fn visit_unit<E>(self) -> Result<ArgoNode, E> {
        Ok(ArgoNode::Leaf(Value::Null))
    }
    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut a: A) -> Result<ArgoNode, A::Error> {
        let mut out = Vec::new();
        while let Some(v) = a.next_element()? {
            out.push(v);
        }
        Ok(ArgoNode::Array(out))
    }
    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut a: A) -> Result<ArgoNode, A::Error> {
        let mut out = Vec::new();
        while let Some((k, v)) = a.next_entry::<String, ArgoNode>()? {
            out.push((k, v));
        }
        Ok(ArgoNode::Object(out))
    }
}

/// The first entry of `J_JsonObject.getFields()`. The fields go into a
/// default `HashMap` in document order (`J_JsonObjectNodeList`, a repeated
/// key keeping its first place and taking the last value), which
/// `new J_JsonObject` copies (`new HashMap(map)`) and `getFields()` copies
/// again: each copy's table is sized for the entry count (below), so for
/// 1 to 6 fields the order differs from the 16-bucket table's. `None` for
/// an empty object (`iterator().next()` throws).
// src: J_JsonObjectNodeList.java:10-18, J_JsonObject.java:10-16, StatFileWriter.java:108-109
fn argo_first_field(fields: &[(String, ArgoNode)]) -> Option<(&str, &ArgoNode)> {
    let mut m0: JavaHashMap<&str, usize> = JavaHashMap::new();
    for (i, (k, _)) in fields.iter().enumerate() {
        m0.insert(k.as_str(), i);
    }
    let order0: Vec<(&str, usize)> = m0.iter().map(|(k, v)| (*k, *v)).collect();
    let order2 = java_hash_map_copy_order(java_hash_map_copy_order(order0));
    let (k, i) = *order2.first()?;
    Some((k, &fields[i].1))
}

/// The iteration order of `new HashMap(map)` filled from `entries` (the
/// source's iteration order, distinct keys), as the oracle JDK (OpenJDK
/// 25) behaves, measured: the table starts at the smallest power of two
/// `>= ceil(n / 0.75)` buckets (2 for one entry, 4 for two or three, 8 for
/// four to six, 16 for seven to twelve) instead of 16, and the entries are
/// inserted in order. In a table of list bins the order is the buckets'
/// (`spread(hash) & (cap - 1)`, `spread(h) = h ^ (h >>> 16)`), each in
/// insertion order; a resize keeps that rule at the new capacity. The table
/// doubles when the size passes `0.75 * cap`, or when a ninth entry is
/// appended to one bucket while `cap < 64`.
///
/// Not modelled: a ninth entry in one bucket of a table of 64 or more
/// buckets, which the JDK turns into a tree bin with its own order; such an
/// object (a hand-made stats file with dozens of fields per entry) is
/// ordered as a list here.
fn java_hash_map_copy_order<K: JavaHash, V>(entries: Vec<(K, V)>) -> Vec<(K, V)> {
    let n = entries.len();
    if n == 0 {
        return entries;
    }
    let spread = |h: i32| h ^ ((h as u32) >> 16) as i32;
    let bucket = |h: i32, cap: usize| (h as u32 as usize) & (cap - 1);
    let recount = |hashes: &[i32], cap: usize| {
        let mut counts = vec![0usize; cap];
        for &h in hashes {
            counts[bucket(h, cap)] += 1;
        }
        counts
    };
    // ceil(n / 0.75) = ceil(4n / 3), then the next power of two.
    let mut cap = (4 * n).div_ceil(3).next_power_of_two().min(1 << 30);
    let mut hashes: Vec<i32> = Vec::with_capacity(n);
    let mut counts = vec![0usize; cap];
    for (k, _) in &entries {
        let h = spread(k.java_hash());
        hashes.push(h);
        counts[bucket(h, cap)] += 1;
        if counts[bucket(h, cap)] > 8 && cap < 64 {
            cap *= 2;
            counts = recount(&hashes, cap);
        }
        if hashes.len() > (cap as f32 * 0.75) as usize {
            cap *= 2;
            counts = recount(&hashes, cap);
        }
    }
    let mut keyed: Vec<(usize, (K, V))> = hashes
        .into_iter()
        .zip(entries)
        .map(|(h, e)| (bucket(h, cap), e))
        .collect();
    keyed.sort_by_key(|(b, _)| *b); // stable: insertion order within a bucket
    keyed.into_iter().map(|(_, e)| e).collect()
}

// ---- StatFileWriter --------------------------------------------------------------------

/// `StatFileWriter`'s maps (keyed by `statId`) and dirty flag, plus the
/// achievement toasts `EntityPlayerSP.addStat` queues.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StatFileWriter {
    /// `field_25102_a` (private): the totals: `hasAchievementUnlocked`
    /// tests membership, `writeStat` reads a value.
    pub field_25102_a: BTreeMap<i32, i32>,
    /// `field_25101_b` (private): the changes not yet sent, which
    /// `func_27176_a` copies for the stats file.
    pub field_25101_b: BTreeMap<i32, i32>,
    /// `field_27189_c` (private): set when a stat is added or a file is
    /// merged, never cleared (module docs); [`StatFileWriter::func_27178_d`]
    /// sends only once it is set.
    pub field_27189_c: bool,
    /// `statsSyncher` (private): the file side (module docs).
    pub stats_syncher: StatsSyncher,
    /// The achievements `EntityPlayerSP.addStat` passed to
    /// `mc.guiAchievement.queueTakenAchievement` (the client's toast),
    /// as `statId`s in order; the client drains it.
    pub taken_achievements: Vec<i32>,
}

impl StatFileWriter {
    /// `readStat(stat, n)`: adds `n` to both maps and marks the writer
    /// dirty (the name is MCP's; it writes).
    // src: StatFileWriter.java:39-43
    pub fn read_stat(&mut self, var1: &StatBase, var2: i32) {
        write_stat_to_map(&mut self.field_25101_b, var1.stat_id, var2);
        write_stat_to_map(&mut self.field_25102_a, var1.stat_id, var2);
        self.field_27189_c = true;
    }

    /// `func_27176_a()`: a copy of the unsent map.
    // src: StatFileWriter.java:51-53
    pub fn func_27176_a(&self) -> BTreeMap<i32, i32> {
        self.field_25101_b.clone()
    }

    /// `func_27179_a(map)`: merges a map read from the unsent file into
    /// both maps (null: nothing).
    // src: StatFileWriter.java:55-67
    pub fn func_27179_a(&mut self, var1: Option<&BTreeMap<i32, i32>>) {
        if let Some(var1) = var1 {
            self.field_27189_c = true;
            for (&var3, &v) in var1 {
                write_stat_to_map(&mut self.field_25101_b, var3, v);
                write_stat_to_map(&mut self.field_25102_a, var3, v);
            }
        }
    }

    /// `func_27180_b(map)`: the received copy merged into the totals: each
    /// stat's total becomes the received value plus the unsent change
    /// (null: nothing). The dirty flag is not touched.
    // src: StatFileWriter.java:69-81
    pub fn func_27180_b(&mut self, var1: Option<&BTreeMap<i32, i32>>) {
        if let Some(var1) = var1 {
            for (&var3, &v) in var1 {
                let var5 = self.field_25101_b.get(&var3).copied().unwrap_or(0);
                self.field_25102_a.insert(var3, v.wrapping_add(var5));
            }
        }
    }

    /// `func_27187_c(map)`: a map added to the unsent changes only (null:
    /// nothing); marks the writer dirty. Its one caller,
    /// [`StatsSyncher::func_27425_c`], passes `field_27436_c`, which nothing
    /// in the client tree sets.
    // src: StatFileWriter.java:83-94
    pub fn func_27187_c(&mut self, var1: Option<&BTreeMap<i32, i32>>) {
        if let Some(var1) = var1 {
            self.field_27189_c = true;
            for (&var3, &v) in var1 {
                write_stat_to_map(&mut self.field_25101_b, var3, v);
            }
        }
    }

    /// `func_27178_d()`, once per `runTick` (Minecraft.java:949): when the
    /// writer is dirty and the syncher idle (the countdown run out),
    /// `func_27424_a(func_27176_a())` (the countdown back to 100, then the
    /// send thread's write of the unsent file, here at once); then
    /// `func_27425_c`. A failed write is returned after the countdowns
    /// have run (the thread prints its exception and frees the syncher).
    // src: StatFileWriter.java:194-200, StatsSyncher.java:129-137,
    // ThreadStatSyncherSend.java:12-21
    pub fn func_27178_d(
        &mut self,
        storage: &mut dyn Storage,
        mc_data_dir: &str,
        username: &str,
        guids: &AchievementMap,
    ) -> Result<(), StorageError> {
        let mut sent = Ok(());
        if self.field_27189_c && self.stats_syncher.func_27420_b() {
            // func_27424_a: the busy flag is never set here (module docs).
            self.stats_syncher.field_27427_l = 100;
            let map = self.func_27176_a();
            let files = StatsFiles::new(&join(mc_data_dir, "stats"), username);
            sent = func_27410_a(
                storage,
                username,
                &map,
                &files.unsent_dat,
                &files.unsent_tmp,
                &files.unsent_old,
                guids,
            );
        }
        self.func_27425_c();
        sent
    }

    /// `statsSyncher.func_27425_c()` (it calls back into this writer):
    /// counts both countdowns down to 0, and merges a map the threads left
    /// (`field_27436_c` into the unsent changes, `field_27437_b`, the
    /// received copy, into the totals), each once.
    // src: StatsSyncher.java:171-190
    pub fn func_27425_c(&mut self) {
        let s = &mut self.stats_syncher;
        if s.field_27427_l > 0 {
            s.field_27427_l -= 1;
        }
        if s.field_27426_m > 0 {
            s.field_27426_m -= 1;
        }
        if let Some(m) = self.stats_syncher.field_27436_c.take() {
            self.func_27187_c(Some(&m));
        }
        if let Some(m) = self.stats_syncher.field_27437_b.take() {
            self.func_27180_b(Some(&m));
        }
    }

    /// `hasAchievementUnlocked(achievement)`: whether the totals hold it.
    // src: StatFileWriter.java:174-176
    pub fn has_achievement_unlocked(&self, var1: &StatBase) -> bool {
        self.field_25102_a.contains_key(&var1.stat_id)
    }

    /// `canUnlockAchievement(achievement)`: no parent, or the parent
    /// unlocked.
    // src: StatFileWriter.java:178-180
    pub fn can_unlock_achievement(&self, var1: &StatBase) -> bool {
        match var1.achievement().and_then(|a| a.parent_achievement) {
            None => true,
            Some(p) => self.field_25102_a.contains_key(&p),
        }
    }

    /// `writeStat(stat)`: the total, 0 when absent (the name is MCP's; it
    /// reads).
    // src: StatFileWriter.java:182-185
    pub fn write_stat(&self, var1: &StatBase) -> i32 {
        self.field_25102_a.get(&var1.stat_id).copied().unwrap_or(0)
    }

    /// `EntityPlayerSP.addStat(stat, n)`, which writes through
    /// `mc.statFileWriter`: nothing for null; an achievement only when it
    /// has no parent or the parent is unlocked, queueing the toast the
    /// first time; any other stat always.
    // src: EntityPlayerSP.java:176-192
    pub fn entity_player_sp_add_stat(&mut self, var1: Option<&StatBase>, var2: i32) {
        let Some(var1) = var1 else { return };
        if let Some(var3) = var1.achievement() {
            let parent_ok = match var3.parent_achievement {
                None => true,
                Some(p) => self.field_25102_a.contains_key(&p),
            };
            if parent_ok {
                if !self.has_achievement_unlocked(var1) {
                    self.taken_achievements.push(var1.stat_id);
                }
                self.read_stat(var1, var2);
            }
        } else {
            self.read_stat(var1, var2);
        }
    }

    /// `new StatFileWriter(session, mcDataDir)` with
    /// `new StatsSyncher(...)`: moves `stats_*.dat` files from the data
    /// directory into `stats/` (unless already there), renames
    /// mixed-case `stats_<username>*` files to the lower-case names, then
    /// merges the unsent file if it exists, and runs `func_27418_a`: the
    /// countdown set to 100 and the receive thread, here at once
    /// (module docs). `mc_data_dir` is a [`Storage`] path ("" for the
    /// storage root).
    // src: StatFileWriter.java:16-37, StatsSyncher.java:26-49, :119-127,
    // ThreadStatSyncherReceive.java:11-24
    pub fn open(
        storage: &mut dyn Storage,
        mc_data_dir: &str,
        username: &str,
        guids: &AchievementMap,
    ) -> Result<StatFileWriter, StorageError> {
        let mut w = StatFileWriter::default();
        let var3 = join(mc_data_dir, "stats");
        // src: StatFileWriter.java:22-34
        for name in storage.list(mc_data_dir)? {
            if name.starts_with("stats_") && name.ends_with(".dat") {
                let var8 = join(&var3, &name);
                if storage.read(&var8)?.is_none() {
                    // `File.renameTo`'s result is ignored (StatFileWriter.java:31).
                    let _ = storage.rename(&join(mc_data_dir, &name), &var8);
                }
            }
        }
        let files = StatsFiles::new(&var3, username);
        // src: StatsSyncher.java:33-40 (func_28214_a)
        let lower = username.to_lowercase();
        if lower != username {
            for (suffix, target) in files.all() {
                let from = join(&var3, &format!("stats_{username}{suffix}"));
                if storage.read(&from)?.is_some() && storage.read(target)?.is_none() {
                    // `renameTo`'s result is ignored (StatsSyncher.java:54).
                    let _ = storage.rename(&from, target);
                }
            }
        }
        // src: StatsSyncher.java:44-46
        if storage.read(&files.unsent_dat)?.is_some() {
            let m = func_27415_a(
                storage,
                &files.unsent_dat,
                &files.unsent_tmp,
                &files.unsent_old,
                guids,
            )?;
            w.func_27179_a(m.as_ref());
        }
        // src: StatsSyncher.java:48 (func_27418_a): the countdown, then the
        // receive thread. Its first branch (a received map to write back,
        // ThreadStatSyncherReceive.java:13-14) needs field_27437_b, which
        // is null here; the second reads stats_<user>.dat (:15-16).
        w.stats_syncher.field_27427_l = 100;
        if storage.read(&files.dat)?.is_some() {
            w.stats_syncher.field_27437_b =
                func_27415_a(storage, &files.dat, &files.tmp, &files.old, guids)?;
        }
        Ok(w)
    }

    /// `syncStats()`: `StatsSyncher.syncStatsFileWithMap(func_27176_a())`
    /// without the busy-wait for the threads (none is ever running, module
    /// docs): writes the unsent map to `stats_<user>_unsent.tmp`, deletes
    /// `_unsent.old`, renames the current `_unsent.dat` to `_unsent.old`
    /// and the `.tmp` to `_unsent.dat`. The countdown is not touched.
    // src: StatFileWriter.java:190-192, StatsSyncher.java:99-117, :139-165
    pub fn sync_stats(
        &self,
        storage: &mut dyn Storage,
        mc_data_dir: &str,
        username: &str,
        guids: &AchievementMap,
    ) -> Result<(), StorageError> {
        let files = StatsFiles::new(&join(mc_data_dir, "stats"), username);
        func_27410_a(
            storage,
            username,
            &self.func_27176_a(),
            &files.unsent_dat,
            &files.unsent_tmp,
            &files.unsent_old,
            guids,
        )
    }
}

/// `StatsSyncher`'s state (StatsSyncher.java:12-24) without the files
/// (named from the username by [`StatsFiles`]) and the back references.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StatsSyncher {
    /// `field_27438_a` (private, volatile): a thread is running. Always
    /// false between calls here (module docs).
    pub field_27438_a: bool,
    /// `field_27437_b` (private, volatile): the received copy
    /// (`stats_<user>.dat`) waiting for `func_27425_c` to merge it.
    pub field_27437_b: Option<BTreeMap<i32, i32>>,
    /// `field_27436_c` (private, volatile): read and cleared by
    /// `func_27425_c`, never set in the client tree.
    pub field_27436_c: Option<BTreeMap<i32, i32>>,
    /// `field_27427_l` (private): ticks until the next send may start
    /// (100 after the constructor and after each send).
    pub field_27427_l: i32,
    /// `field_27426_m` (private): counted down, never set (0).
    pub field_27426_m: i32,
}

impl StatsSyncher {
    /// `func_27420_b()`: the countdown has run out, no thread is running and
    /// nothing waits in `field_27436_c`.
    // src: StatsSyncher.java:167-169
    pub fn func_27420_b(&self) -> bool {
        self.field_27427_l <= 0 && !self.field_27438_a && self.field_27436_c.is_none()
    }
}

/// `StatsSyncher.func_27410_a(map, file, tmp, old)` (private): the map in
/// `func_27185_a`'s format (`username`, salt `"local"`) written to `tmp`,
/// `old` deleted, `file` renamed to `old`, `tmp` to `file`.
// src: StatsSyncher.java:99-117
fn func_27410_a(
    storage: &mut dyn Storage,
    username: &str,
    var1: &BTreeMap<i32, i32>,
    var2: &str,
    var3: &str,
    var4: &str,
    guids: &AchievementMap,
) -> Result<(), StorageError> {
    let text = func_27185_a(Some(username), Some("local"), var1, guids);
    storage.write(var3, text.as_bytes())?;
    // `File.delete` and `File.renameTo` report failure by returning
    // false, which vanilla ignores: a failed step does not stop the
    // next one (StatsSyncher.java:108-116).
    if storage.read(var4)?.is_some() {
        let _ = storage.delete(var4);
    }
    if storage.read(var2)?.is_some() {
        let _ = storage.rename(var2, var4);
    }
    let _ = storage.rename(var3, var2);
    Ok(())
}

/// `writeStatToMap(map, stat, n)` (private): adds `n` (Java `int`,
/// wrapping) to the stat's value, 0 when absent.
// src: StatFileWriter.java:45-49
fn write_stat_to_map(var1: &mut BTreeMap<i32, i32>, id: i32, var3: i32) {
    let var5 = var1.get(&id).copied().unwrap_or(0);
    var1.insert(id, var5.wrapping_add(var3));
}

/// `a/b`, or `b` when `a` is the storage root.
fn join(a: &str, b: &str) -> String {
    if a.is_empty() {
        b.to_string()
    } else {
        format!("{a}/{b}")
    }
}

/// `StatsSyncher`'s six files (`field_27434_e` ... `field_27431_h`), named
/// with the lower-cased username (Java `toLowerCase()`, default locale).
// src: StatsSyncher.java:27-32
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatsFiles {
    /// `field_27434_e`: `stats_<user>_unsent.dat`, the singleplayer stats.
    pub unsent_dat: String,
    /// `field_27433_f`: `stats_<user>.dat`, the received copy.
    pub dat: String,
    /// `field_27430_i`: `stats_<user>_unsent.old`.
    pub unsent_old: String,
    /// `field_27429_j`: `stats_<user>.old`.
    pub old: String,
    /// `field_27432_g`: `stats_<user>_unsent.tmp`.
    pub unsent_tmp: String,
    /// `field_27431_h`: `stats_<user>.tmp`.
    pub tmp: String,
}

impl StatsFiles {
    /// The six paths inside `stats_dir`.
    pub fn new(stats_dir: &str, username: &str) -> StatsFiles {
        let u = username.to_lowercase();
        let f = |suffix: &str| join(stats_dir, &format!("stats_{u}{suffix}"));
        StatsFiles {
            unsent_dat: f("_unsent.dat"),
            dat: f(".dat"),
            unsent_old: f("_unsent.old"),
            old: f(".old"),
            unsent_tmp: f("_unsent.tmp"),
            tmp: f(".tmp"),
        }
    }

    /// `(suffix, path)` in the order of StatsSyncher.java:34-39.
    fn all(&self) -> [(&'static str, &String); 6] {
        [
            ("_unsent.dat", &self.unsent_dat),
            (".dat", &self.dat),
            ("_unsent.old", &self.unsent_old),
            (".old", &self.old),
            ("_unsent.tmp", &self.unsent_tmp),
            (".tmp", &self.tmp),
        ]
    }
}

/// `StatsSyncher.func_27415_a(file, tmp, old)` (private): reads `file`
/// if it exists, else `old`, else `tmp`, else null.
// src: StatsSyncher.java:59-67
fn func_27415_a(
    storage: &mut dyn Storage,
    var1: &str,
    var2: &str,
    var3: &str,
    guids: &AchievementMap,
) -> Result<Option<BTreeMap<i32, i32>>, StorageError> {
    for path in [var1, var3, var2] {
        if let Some(bytes) = storage.read(path)? {
            return Ok(func_27408_a(&bytes, guids));
        }
    }
    Ok(None)
}

/// `StatsSyncher.func_27408_a(file)` (private): the file's lines joined
/// without their ends (`BufferedReader.readLine`), parsed by
/// `func_27177_a`; null on any exception. The bytes are decoded as UTF-8
/// (Java's `FileReader` uses the platform charset).
// src: StatsSyncher.java:69-97
pub fn func_27408_a(bytes: &[u8], guids: &AchievementMap) -> Option<BTreeMap<i32, i32>> {
    let text = String::from_utf8_lossy(bytes);
    let joined: String = java_read_lines(&text).concat();
    func_27177_a(&joined, guids)
}

/// `StatFileWriter.func_27177_a(text)` (static): parses a stats file. Each
/// element of `"stats-change"` is an object whose first field (Argo's
/// `getFields()` order, [`argo_first_field`]) is `"statId": value`; ids with no stat are skipped
/// (and left out of the checksum); the MD5 of the `guid,value,` pairs
/// salted with `"local"` must equal `"checksum"`, else null ("CHECKSUM
/// MISMATCH"). A syntax error gives an empty map (Argo's
/// `J_InvalidSyntaxException` is caught there); any other failure (a
/// missing node, a non-integer value, an empty object) is an exception that
/// `func_27408_a` turns into null. Argo accepts only an object or array
/// root.
// src: StatFileWriter.java:96-133
pub fn func_27177_a(var0: &str, guids: &AchievementMap) -> Option<BTreeMap<i32, i32>> {
    let mut var1 = BTreeMap::new();
    let var2 = "local";
    let mut var3 = String::new();
    let var4: ArgoNode = match serde_json::from_str(var0) {
        Ok(v @ (ArgoNode::Object(_) | ArgoNode::Array(_))) => v,
        _ => return Some(var1),
    };
    let ArgoNode::Array(var5) = var4.get("stats-change")? else {
        return None;
    };
    let list = StatList::get();
    for var7 in var5 {
        let ArgoNode::Object(var8) = var7 else {
            return None;
        };
        // getFields()'s first entry in Java order (J_JsonStringNode hashes
        // as its String).
        let (k, v) = argo_first_field(var8)?;
        let var10 = java_parse_int(k)?;
        let ArgoNode::Leaf(v) = v else { return None };
        let var11 = json_text_int(v)?;
        match list.func_27361_a(var10) {
            None => {
                // System.out.println(var10 + " is not a valid stat")
            }
            Some(var12) => {
                let guid = guids.get_guid(var12.stat_id).unwrap_or("null");
                var3.push_str(&format!("{guid},{var11},"));
                var1.insert(var12.stat_id, var11);
            }
        }
    }
    let var15 = md5_string(Some(var2), &var3);
    let ArgoNode::Leaf(Value::String(checksum)) = var4.get("checksum")? else {
        return None;
    };
    if &var15 != checksum {
        // System.out.println("CHECKSUM MISMATCH")
        return None;
    }
    Some(var1)
}

/// `StatFileWriter.func_27185_a(username, sessionId, map)` (static): the
/// stats file text, `\r\n` line ends, the `"user"` block only when both
/// strings are non-null, then the changes as one single-field object per
/// stat and the checksum (`MD5String(sessionId)` over `guid,value,` pairs).
/// The strings are written unescaped (vanilla: a `"` in a username makes the
/// file unreadable).
// src: StatFileWriter.java:135-172
pub fn func_27185_a(
    var0: Option<&str>,
    var1: Option<&str>,
    var2: &BTreeMap<i32, i32>,
    guids: &AchievementMap,
) -> String {
    let mut var3 = String::new();
    let mut var4 = String::new();
    let mut var5 = true;
    var3.push_str("{\r\n");
    if let (Some(var0), Some(var1)) = (var0, var1) {
        var3.push_str("  \"user\":{\r\n");
        var3.push_str(&format!("    \"name\":\"{var0}\",\r\n"));
        var3.push_str(&format!("    \"sessionid\":\"{var1}\"\r\n"));
        var3.push_str("  },\r\n");
    }
    var3.push_str("  \"stats-change\":[");
    for (&id, &v) in var2 {
        if !var5 {
            var3.push_str("},");
        } else {
            var5 = false;
        }
        var3.push_str(&format!("\r\n    {{\"{id}\":{v}"));
        var4.push_str(&format!("{},{v},", guids.get_guid(id).unwrap_or("null")));
    }
    if !var5 {
        var3.push('}');
    }
    var3.push_str("\r\n  ],\r\n");
    var3.push_str(&format!(
        "  \"checksum\":\"{}\"\r\n",
        md5_string(var1, &var4)
    ));
    var3.push('}');
    var3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md5_matches_rfc_1321_vectors() {
        let hex = |d: [u8; 16]| d.iter().map(|b| format!("{b:02x}")).collect::<String>();
        assert_eq!(hex(md5(b"")), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(hex(md5(b"abc")), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(
            hex(md5(
                b"12345678901234567890123456789012345678901234567890123456789012345678901234567890"
            )),
            "57edf4a22be3c955ac49da2e2107b67a"
        );
    }

    #[test]
    fn md5_string_drops_leading_zeros() {
        // MD5("local") has no leading zero; BigInteger.toString(16) of a
        // digest starting with 0x0... would be shorter than 32.
        assert_eq!(md5_string(Some("lo"), "cal").len(), 32);
        assert_eq!(md5_string(None, "x"), md5_string(Some("null"), "x"));
    }

    #[test]
    fn read_lines_like_java() {
        assert_eq!(java_read_lines("a\r\nb\rc\n"), vec!["a", "b", "c"]);
        assert_eq!(java_read_lines("a\n\nb"), vec!["a", "", "b"]);
        assert!(java_read_lines("").is_empty());
    }

    #[test]
    fn achievement_map_stops_at_a_bad_line() {
        let m = AchievementMap::parse("1000,aa\r\n1001,bb,\n1002\n1003,dd\n");
        assert_eq!(m.get_guid(1000), Some("aa"));
        assert_eq!(m.get_guid(1001), Some("bb"));
        assert_eq!(m.get_guid(1002), None);
        assert_eq!(m.get_guid(1003), None);
    }

    /// `new HashMap(new HashMap(m0))` with `m0` filled in document order.
    fn argo_order<'a>(doc: &[&'a str]) -> Vec<&'a str> {
        let mut m0: JavaHashMap<&str, ()> = JavaHashMap::new();
        for k in doc {
            m0.insert(*k, ());
        }
        let order0: Vec<(&str, ())> = m0.iter().map(|(k, v)| (*k, *v)).collect();
        java_hash_map_copy_order(java_hash_map_copy_order(order0))
            .into_iter()
            .map(|(k, _)| k)
            .collect()
    }

    /// Orders printed by OpenJDK 25 for `new HashMap(new HashMap(m0))`:
    /// three keys in a 4-bucket table; twenty keys (a 32-bucket table);
    /// nine keys in one bucket of the 16-bucket copy, which grows to 32
    /// buckets on the ninth instead of making a tree bin.
    #[test]
    fn hash_map_copy_order_matches_the_jdk() {
        assert_eq!(argo_order(&["x1", "x5", "x2"]), ["x1", "x5", "x2"]);
        let doc = [
            "f64236", "f49164", "f29485", "f78044", "f89380", "f66254", "f87968", "f96649",
            "f98850", "f39534", "f31200", "f13712", "f78708", "f88911", "f74495", "f89662",
            "f12961", "f50738", "f71307", "f38279",
        ];
        assert_eq!(
            argo_order(&doc),
            [
                "f74495", "f49164", "f98850", "f78044", "f13712", "f66254", "f87968", "f29485",
                "f64236", "f88911", "f96649", "f50738", "f89662", "f12961", "f71307", "f78708",
                "f89380", "f31200", "f39534", "f38279",
            ]
        );
        let doc = [
            "k17", "k309", "k28", "k408", "k39", "k419", "k150", "k507", "k161",
        ];
        assert_eq!(
            argo_order(&doc),
            [
                "k309", "k408", "k419", "k507", "k17", "k28", "k39", "k150", "k161"
            ]
        );
    }
}
