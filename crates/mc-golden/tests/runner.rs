//! End-to-end `mc_golden::run` against a throwaway fixture under the target
//! directory (never under `tests/golden/`).

use std::path::PathBuf;

use mc_golden::{GoldenError, RngValue, ScenarioKind, TraceLine, TraceWriter, run, target_dir};

// Test-only host file writes; clippy.toml's D9 bans exist for the sim crates.
#[allow(clippy::disallowed_methods)]
fn write(path: &PathBuf, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A `java_random` fixture whose expected trace is three hand-written lines.
fn fixture(name: &str) -> PathBuf {
    let dir = target_dir().join("golden-selftest/selftest").join(name);
    write(
        &dir.join("scenario.json"),
        &format!(
            r#"{{"name":"{name}","tier":"A","kind":"java_random","seed":42,
                "draws":[{{"call":"nextInt","n":1}},{{"call":"nextDouble","n":2}}]}}"#
        ),
    );
    write(
        &dir.join("expected.jsonl"),
        concat!(
            r#"{"t":0,"e":"rng","i":0,"call":"nextInt","v":-1170105035}"#,
            "\n",
            r#"{"t":0,"e":"rng","i":1,"call":"nextDouble","v":"0x3fe0000000000000"}"#,
            "\n",
            r#"{"t":0,"e":"rng","i":2,"call":"nextDouble","v":"0x3fd0000000000000"}"#,
            "\n",
        ),
    );
    write(&dir.join("blob.bin"), "abc");
    dir
}

/// A fake engine: replays the expected values, optionally corrupting draw 2.
fn drive(
    corrupt: bool,
) -> impl FnOnce(&mc_golden::Scenario, &mut TraceWriter) -> Result<(), GoldenError> {
    move |scenario, out| {
        let ScenarioKind::JavaRandom { seed, draws } = &scenario.kind else {
            return Err(GoldenError::other("wrong kind"));
        };
        assert_eq!((*seed, draws.len()), (42, 2));
        out.push(TraceLine::rng(
            0,
            "nextInt",
            None,
            RngValue::Int(-1170105035),
        ));
        out.push(TraceLine::rng(1, "nextDouble", None, RngValue::F64(0.5)));
        let last = if corrupt { 0.25000000000000006 } else { 0.25 };
        out.push(TraceLine::rng(2, "nextDouble", None, RngValue::F64(last)));
        Ok(())
    }
}

#[test]
fn passing_run_writes_actual() {
    let dir = fixture("pass");
    run(&dir, drive(false)).unwrap();
    let actual = target_dir().join("golden/selftest/pass/actual.jsonl");
    assert!(actual.is_file(), "{}", actual.display());
    assert_eq!(mc_golden::read_sidecar(&dir, "blob.bin").unwrap(), b"abc");
    assert!(mc_golden::read_sidecar(&dir, "../pass/blob.bin").is_err());
}

#[test]
fn failing_run_reports_line_and_field() {
    let dir = fixture("fail");
    let err = run(&dir, drive(true)).unwrap_err();
    assert!(matches!(err, GoldenError::Mismatch { .. }));
    let text = err.to_string();
    println!("{text}");
    for needle in [
        "line 3 (t=0, e=\"rng\")",
        "field `v`",
        "\"0x3fd0000000000000\" (dbg 0.25)",
        "\"0x3fd0000000000001\" (dbg 0.25000000000000006)",
        "golden/selftest/fail/actual.jsonl",
    ] {
        assert!(text.contains(needle), "missing {needle:?} in:\n{text}");
    }
}

#[test]
fn drive_error_and_missing_fixture() {
    let dir = fixture("drive-error");
    let err = run(&dir, |_, _| Err(GoldenError::other("engine exploded"))).unwrap_err();
    assert_eq!(err.to_string(), "engine exploded");
    let err = run(
        target_dir().join("golden-selftest/selftest/absent"),
        drive(false),
    )
    .unwrap_err();
    assert!(err.to_string().contains("scenario.json"), "{err}");
}

/// `gaps.json` (Gate 2 R7): written next to `actual.jsonl` when the driver
/// records the gap counters, also when the drive fails; removed again by a
/// run whose driver records none; never compared.
#[test]
fn gaps_file_follows_the_driver() {
    let dir = fixture("gaps");
    let gaps = mc_golden::actual_dir_of(&dir).join(mc_golden::GAPS_FILE);
    assert!(
        gaps.ends_with("golden/selftest/gaps/gaps.json"),
        "{}",
        gaps.display()
    );
    let mut g = mc_golden::Gaps::default();
    g.unimplemented_hits.insert("Some.record".into(), 3);
    let set = g.clone();
    let err = run(&dir, move |_, out| {
        out.set_gaps(set);
        Err(GoldenError::other("stopped"))
    })
    .unwrap_err();
    assert_eq!(err.to_string(), "stopped");
    #[allow(clippy::disallowed_methods)]
    let text = std::fs::read_to_string(&gaps).unwrap();
    assert_eq!(text, g.to_file_text());
    assert!(text.contains("\"Some.record\": 3"), "{text}");
    // A run that records nothing leaves no stale file behind.
    run(&dir, drive(false)).unwrap();
    assert!(!gaps.exists());
}

/// The tick driver records the world's counters: a one-tick scenario with no
/// ops or dumps (its trace is the implicit preload line) writes a
/// `gaps.json` with both maps.
#[test]
fn tick_driver_writes_gaps() {
    let dir = target_dir().join("golden-selftest/selftest/tick-gaps");
    write(
        &dir.join("scenario.json"),
        r#"{"name":"tick-gaps","tier":"B","kind":"tick","seed":1,"dimension":"overworld","world":"empty",
            "ticks":1,"mob_spawning":"none","ops":[],"dumps":[]}"#,
    );
    write(
        &dir.join("expected.jsonl"),
        "{\"t\":0,\"e\":\"preload\",\"implicit\":true,\"loaded\":0}\n",
    );
    run(&dir, mc_golden::tick_driver::drive).unwrap();
    let gaps = mc_golden::actual_dir_of(&dir).join(mc_golden::GAPS_FILE);
    #[allow(clippy::disallowed_methods)]
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&gaps).unwrap()).unwrap();
    assert!(v["skipped_systems"].is_object(), "{v}");
    assert!(v["unimplemented_hits"].is_object(), "{v}");
}
