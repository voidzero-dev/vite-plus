#![expect(clippy::disallowed_macros, reason = "standalone test uses std macros")]
#![expect(clippy::disallowed_types, reason = "standalone test uses std types")]

#[path = "cli_snapshots/schedule.rs"]
mod schedule;

#[test]
fn nextest_reserves_all_workers_for_exact_case_names() {
    let names = ["ctrlc_isolation::explicit_serial", "app_root_listing::picker_cancel::global"];
    let config: toml::Value = toml::from_str(&schedule::nextest_config(names)).unwrap();
    let overrides = config["profile"]["default"]["overrides"].as_array().unwrap();
    assert_eq!(overrides.len(), names.len());
    for (entry, name) in overrides.iter().zip(names) {
        assert_eq!(
            entry["filter"].as_str().unwrap(),
            format!("binary(cli_snapshots) & test(={name})")
        );
        assert_eq!(entry["threads-required"].as_str().unwrap(), "num-test-threads");
        assert_eq!(entry["priority"].as_integer().unwrap(), -100);
    }
}

#[test]
fn no_isolated_cases_does_not_change_nextest_defaults() {
    let config: toml::Value = toml::from_str(&schedule::nextest_config([])).unwrap();
    assert!(config.as_table().unwrap().is_empty());
}
