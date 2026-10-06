//! Trace comparison and the first-difference report (ADR-007 §5.2,
//! PLAN_v4 §4.4 item 3).
//!
//! Lines are compared in file order by parsed JSON value: key order does not
//! matter, missing or extra keys do, and `"dbg"` fields (at any depth) are
//! never compared (README "Encoding rules"). The report names the expected
//! file's line number, the line's `t` and `e`, the first differing field path
//! (`state.cargo`, `items[2].count`), both values, floats decoded from their
//! bit patterns, and for hex byte arrays the first differing byte.

use std::fmt;

use serde_json::Value;

use crate::bits::{decode_float, parse_hex_bytes};
use crate::trace::TraceLine;

/// Fields named this are informational only (README "Encoding rules").
const DBG: &str = "dbg";

/// The first difference between two traces.
#[derive(Clone, Debug, PartialEq)]
pub struct Mismatch {
    /// 1-based line number in the expected file (for an extra actual line,
    /// one past its end).
    pub line: usize,
    /// `t` of the line (from the expected side when there is one).
    pub t: i64,
    /// `e` of the line (from the expected side when there is one).
    pub e: String,
    /// What differs.
    pub kind: MismatchKind,
}

/// The kind of difference.
#[derive(Clone, Debug, PartialEq)]
pub enum MismatchKind {
    /// Both sides have the field, with different values.
    Value {
        /// Field path, e.g. `v` or `state.cargo`.
        path: String,
        /// Expected value.
        expected: Value,
        /// Actual value.
        actual: Value,
    },
    /// Arrays of different length whose common prefix is equal.
    Length {
        /// Field path.
        path: String,
        /// Expected length.
        expected: usize,
        /// Actual length.
        actual: usize,
    },
    /// The actual line lacks a field.
    MissingKey {
        /// Field path.
        path: String,
        /// Expected value.
        expected: Value,
    },
    /// The actual line has a field the expected one lacks.
    ExtraKey {
        /// Field path.
        path: String,
        /// Actual value.
        actual: Value,
    },
    /// The actual trace ended before this expected line.
    MissingLine {
        /// The expected line.
        expected: Value,
    },
    /// The actual trace continues past the end of the expected one.
    ExtraLine {
        /// The first extra actual line.
        actual: Value,
    },
}

/// Compares two traces; `Ok` when equal under the rules in the module docs.
pub fn compare(expected: &[TraceLine], actual: &[TraceLine]) -> Result<(), Box<Mismatch>> {
    for (i, exp) in expected.iter().enumerate() {
        let line = if exp.line() == 0 { i + 1 } else { exp.line() };
        let mismatch = |kind| {
            Box::new(Mismatch {
                line,
                t: exp.t(),
                e: exp.e().to_owned(),
                kind,
            })
        };
        let Some(act) = actual.get(i) else {
            return Err(mismatch(MismatchKind::MissingLine {
                expected: Value::Object(exp.fields().clone()),
            }));
        };
        // `t` and `e` first: a wrong tick or event explains everything after.
        for key in ["t", "e"] {
            if let Some(kind) = diff_values(key, &exp.fields()[key], &act.fields()[key]) {
                return Err(mismatch(kind));
            }
        }
        if let Some(kind) = diff_objects("", exp.fields(), act.fields()) {
            return Err(mismatch(kind));
        }
    }
    if let Some(extra) = actual.get(expected.len()) {
        return Err(Box::new(Mismatch {
            line: expected.last().map_or(0, |l| l.line().max(expected.len())) + 1,
            t: extra.t(),
            e: extra.e().to_owned(),
            kind: MismatchKind::ExtraLine {
                actual: Value::Object(extra.fields().clone()),
            },
        }));
    }
    Ok(())
}

fn join(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_owned()
    } else {
        format!("{path}.{key}")
    }
}

fn diff_objects(
    path: &str,
    exp: &serde_json::Map<String, Value>,
    act: &serde_json::Map<String, Value>,
) -> Option<MismatchKind> {
    for (key, ev) in exp.iter().filter(|(k, _)| *k != DBG) {
        let path = join(path, key);
        let Some(av) = act.get(key) else {
            return Some(MismatchKind::MissingKey {
                path,
                expected: ev.clone(),
            });
        };
        if let Some(kind) = diff_values(&path, ev, av) {
            return Some(kind);
        }
    }
    act.iter()
        .find(|(k, _)| *k != DBG && !exp.contains_key(*k))
        .map(|(key, av)| MismatchKind::ExtraKey {
            path: join(path, key),
            actual: av.clone(),
        })
}

fn diff_values(path: &str, exp: &Value, act: &Value) -> Option<MismatchKind> {
    match (exp, act) {
        (Value::Object(e), Value::Object(a)) => diff_objects(path, e, a),
        (Value::Array(e), Value::Array(a)) => e
            .iter()
            .zip(a)
            .enumerate()
            .find_map(|(i, (ev, av))| diff_values(&format!("{path}[{i}]"), ev, av))
            .or_else(|| {
                (e.len() != a.len()).then(|| MismatchKind::Length {
                    path: path.to_owned(),
                    expected: e.len(),
                    actual: a.len(),
                })
            }),
        _ if exp == act => None,
        _ => Some(MismatchKind::Value {
            path: path.to_owned(),
            expected: exp.clone(),
            actual: act.clone(),
        }),
    }
}

/// A value for the report: truncated when long, floats decoded.
fn show(v: &Value) -> String {
    const MAX: usize = 72;
    let text = v.to_string();
    let mut out = match text.char_indices().nth(MAX) {
        Some((cut, _)) => format!("{}… ({} chars)", &text[..cut], text.chars().count()),
        None => text,
    };
    if let Some(dbg) = v.as_str().and_then(decode_float) {
        out.push_str(&format!(" (dbg {dbg})"));
    }
    out
}

/// For two hex byte arrays longer than a digest, the first differing byte.
fn byte_diff(path: &str, exp: &Value, act: &Value) -> Option<String> {
    let (e, a) = (exp.as_str()?, act.as_str()?);
    if e.len() <= 64 || a.len() <= 64 {
        return None; // digests and bit patterns: the byte index says nothing
    }
    let (e, a) = (parse_hex_bytes(e).ok()?, parse_hex_bytes(a).ok()?);
    let i = (0..e.len().max(a.len())).find(|&i| e.get(i) != a.get(i))?;
    let byte = |b: Option<&u8>| b.map_or("<end>".to_owned(), |b| format!("0x{b:02x}"));
    let mut out = format!(
        "  first differing byte: index {i} (expected {}, actual {}; lengths {} / {})",
        byte(e.get(i)),
        byte(a.get(i)),
        e.len(),
        a.len()
    );
    // `Chunk.blocks` is indexed x << 11 | z << 7 | y (README `chunk_digest`).
    if path.rsplit('.').next() == Some("blocks") && e.len() == 32768 {
        out.push_str(&format!(
            "\n  block: x={} y={} z={}",
            i >> 11,
            i & 0x7f,
            (i >> 7) & 0xf
        ));
    }
    Some(out)
}

impl fmt::Display for Mismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let at = format!("line {} (t={}, e={:?})", self.line, self.t, self.e);
        match &self.kind {
            MismatchKind::Value {
                path,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "golden mismatch at {at}, field `{path}`:\n  expected: {}\n  actual:   {}",
                    show(expected),
                    show(actual)
                )?;
                if let Some(bytes) = byte_diff(path, expected, actual) {
                    write!(f, "\n{bytes}")?;
                }
                Ok(())
            }
            MismatchKind::Length {
                path,
                expected,
                actual,
            } => write!(
                f,
                "golden mismatch at {at}, field `{path}`: array length\n  expected: {expected}\n  actual:   {actual}"
            ),
            MismatchKind::MissingKey { path, expected } => write!(
                f,
                "golden mismatch at {at}, field `{path}`: missing from actual\n  expected: {}",
                show(expected)
            ),
            MismatchKind::ExtraKey { path, actual } => write!(
                f,
                "golden mismatch at {at}, field `{path}`: not in expected\n  actual:   {}",
                show(actual)
            ),
            MismatchKind::MissingLine { expected } => write!(
                f,
                "golden mismatch at {at}: actual trace ended before this line\n  expected: {}",
                show(expected)
            ),
            MismatchKind::ExtraLine { actual } => write!(
                f,
                "golden mismatch at {at}: actual trace has more lines than expected\n  actual:   {}",
                show(actual)
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::parse_jsonl;

    const EXPECTED: &str = r#"{"t":0,"e":"rng","i":0,"call":"nextInt","v":5}
{"t":0,"e":"rng","i":1,"call":"nextFloat","v":"0x3f000000"}
{"t":0,"e":"rng","i":2,"call":"nextDouble","v":"0x3fe0000000000000","dbg":"0.5"}
{"t":7,"e":"entity","state":{"cargo":"rideable","list":[1,2]}}
"#;

    fn lines(text: &str) -> Vec<TraceLine> {
        parse_jsonl(text).unwrap()
    }

    fn mismatch(actual: &str) -> Box<Mismatch> {
        compare(&lines(EXPECTED), &lines(actual)).unwrap_err()
    }

    #[test]
    fn identical_and_reordered_pass() {
        compare(&lines(EXPECTED), &lines(EXPECTED)).unwrap();
        let reordered = EXPECTED
            .replace(
                r#"{"t":0,"e":"rng","i":0,"call":"nextInt","v":5}"#,
                r#"{"v":5,"call":"nextInt","i":0,"e":"rng","t":0}"#,
            )
            .replace(r#","dbg":"0.5""#, r#","dbg":"different, ignored""#);
        compare(&lines(EXPECTED), &lines(&reordered)).unwrap();
    }

    #[test]
    fn float_field_difference_on_line_3() {
        let m = mismatch(&EXPECTED.replace("0x3fe0000000000000", "0x3fe0000000000001"));
        assert_eq!((m.line, m.t, m.e.as_str()), (3, 0, "rng"));
        let text = m.to_string();
        assert!(text.contains("line 3"), "{text}");
        assert!(text.contains("field `v`"), "{text}");
        assert!(text.contains(r#""0x3fe0000000000000" (dbg 0.5)"#), "{text}");
        assert!(
            text.contains(r#""0x3fe0000000000001" (dbg 0.5000000000000001)"#),
            "{text}"
        );
        let m = mismatch(&EXPECTED.replace("0x3f000000", "0x3f000001"));
        assert!(m.to_string().contains("(dbg 0.50000006f32)"), "{m}");
    }

    #[test]
    fn nested_path_missing_and_extra() {
        let m = mismatch(&EXPECTED.replace("rideable", "chest"));
        assert_eq!(m.line, 4);
        assert!(m.to_string().contains("field `state.cargo`"), "{m}");
        let m = mismatch(&EXPECTED.replace("[1,2]", "[1,3]"));
        assert!(m.to_string().contains("field `state.list[1]`"), "{m}");
        let m = mismatch(&EXPECTED.replace("[1,2]", "[1]"));
        assert!(matches!(
            m.kind,
            MismatchKind::Length {
                expected: 2,
                actual: 1,
                ..
            }
        ));
        let m = mismatch(&EXPECTED.replace(r#""i":1,"#, ""));
        assert!(matches!(&m.kind, MismatchKind::MissingKey { path, .. } if path == "i"));
        let m = mismatch(&EXPECTED.replace(r#""i":1,"#, r#""i":1,"bound":3,"#));
        assert!(matches!(&m.kind, MismatchKind::ExtraKey { path, .. } if path == "bound"));
        let m = mismatch(&EXPECTED.replace(r#""t":7"#, r#""t":8"#));
        assert!(m.to_string().contains("field `t`"), "{m}");
    }

    #[test]
    fn missing_and_extra_lines() {
        let short: String = EXPECTED.lines().take(2).map(|l| format!("{l}\n")).collect();
        let m = mismatch(&short);
        assert_eq!(m.line, 3);
        assert!(matches!(m.kind, MismatchKind::MissingLine { .. }));
        assert!(m.to_string().contains("ended before this line"), "{m}");
        let long = format!("{EXPECTED}{{\"t\":9,\"e\":\"extra\"}}\n");
        let m = mismatch(&long);
        assert_eq!((m.line, m.t, m.e.as_str()), (5, 9, "extra"));
    }

    #[test]
    fn hex_array_byte_index() {
        let mut blocks = vec![0u8; 32768];
        let exp = format!(
            "{{\"t\":0,\"e\":\"chunk_full\",\"blocks\":\"{}\"}}",
            crate::hex_bytes(&blocks)
        );
        blocks[(3 << 11) | (5 << 7) | 64] = 7;
        let act = format!(
            "{{\"t\":0,\"e\":\"chunk_full\",\"blocks\":\"{}\"}}",
            crate::hex_bytes(&blocks)
        );
        let text = compare(&lines(&exp), &lines(&act)).unwrap_err().to_string();
        let index = (3 << 11) | (5 << 7) | 64;
        assert!(
            text.contains(&format!("index {index} (expected 0x00, actual 0x07")),
            "{text}"
        );
        assert!(text.contains("block: x=3 y=64 z=5"), "{text}");
        assert!(text.len() < 1000, "long values are truncated: {text}");
    }
}
