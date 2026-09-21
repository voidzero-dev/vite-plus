#![expect(clippy::disallowed_types, reason = "standalone test uses std types")]
#![expect(clippy::disallowed_macros, reason = "standalone test uses std macros")]

#[path = "cli_snapshots/report.rs"]
mod report;

#[test]
fn successful_cases_only_write_timings() {
    let tmp = tempfile::tempdir().unwrap();
    let report = report::Report::new(Some(tmp.path().to_path_buf()), "fixture::case".into());
    {
        let _phase = report.phase("command");
    }
    report.capture("vpt print", "not a failure");
    report.actual("# snapshot\n");
    report.finish(None, None).unwrap();
    assert!(!tmp.path().join("output.txt").exists());
    assert!(!tmp.path().join("actual.md").exists());
    let timing: serde_json::Value =
        serde_json::from_slice(&std::fs::read(tmp.path().join("timing.json")).unwrap()).unwrap();
    assert_eq!(timing["name"], "fixture::case");
    assert_eq!(timing["status"], "passed");
    assert_eq!(timing["phases"][0]["name"], "command");
}

#[test]
fn failures_keep_expected_actual_and_bounded_unicode_output() {
    let tmp = tempfile::tempdir().unwrap();
    let expected = tmp.path().join("baseline.md");
    std::fs::write(&expected, "expected snapshot").unwrap();
    let dir = tmp.path().join("artifacts");
    let report = report::Report::new(Some(dir.clone()), "fixture::case".into());
    report.capture("setup", &"界".repeat(400_000));
    report.capture("vp check", "diagnostic tail");
    report.actual("actual snapshot");
    report.finish(Some("snapshot mismatch"), Some(&expected)).unwrap();
    let output = std::fs::read_to_string(dir.join("output.txt")).unwrap();
    assert!(output.len() <= 1024 * 1024);
    let timing: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("timing.json")).unwrap()).unwrap();
    assert_eq!(timing["output_truncated"], true);
    assert!(output.ends_with("$ vp check\ndiagnostic tail"));
    assert_eq!(std::fs::read_to_string(dir.join("expected.md")).unwrap(), "expected snapshot");
    assert_eq!(std::fs::read_to_string(dir.join("actual.md")).unwrap(), "actual snapshot");
    assert_eq!(std::fs::read_to_string(expected).unwrap(), "expected snapshot");
}

#[test]
fn panics_preserve_phase_timings() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    let result = std::panic::catch_unwind(|| {
        let report = report::Report::new(Some(dir), "panicked".into());
        let _phase = report.phase("spawn");
        panic!("simulated runner failure");
    });
    assert!(result.is_err());
    let timing: serde_json::Value =
        serde_json::from_slice(&std::fs::read(tmp.path().join("timing.json")).unwrap()).unwrap();
    assert_eq!(timing["status"], "failed");
    assert_eq!(timing["phases"][0]["name"], "spawn");
}
