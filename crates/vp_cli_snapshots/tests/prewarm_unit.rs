#![expect(clippy::disallowed_macros, reason = "standalone test uses std macros")]
#![expect(clippy::disallowed_types, reason = "standalone test uses std types")]

#[path = "cli_snapshots/prewarm.rs"]
mod prewarm;

use prewarm::BunPrewarm;
use serde_json::json;

fn step(prewarm: &mut BunPrewarm, argv: &[&str]) {
    prewarm.step(&argv.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>());
}

#[test]
fn package_changes_update_the_manifest_without_a_second_version_list() {
    let fixture = tempfile::tempdir().unwrap();
    let package = fixture.path().join("package.json");
    std::fs::write(&package, r#"{"packageManager":"bun@1.2.3"}"#).unwrap();
    std::fs::create_dir_all(fixture.path().join("packages/app")).unwrap();
    let mut before = BunPrewarm::default();
    before.workspace(fixture.path(), "packages/app").unwrap();
    assert_eq!(
        before.urls("bun-windows-x64"),
        ["https://registry.npmjs.org/@oven/bun-windows-x64/-/bun-windows-x64-1.2.3.tgz"]
    );

    std::fs::write(
        &package,
        r#"{"devEngines":{"packageManager":{"name":"bun","version":"2.0.0"}}}"#,
    )
    .unwrap();
    let mut after = BunPrewarm::default();
    after.workspace(fixture.path(), "packages/app").unwrap();
    assert_eq!(
        after.urls("bun-windows-x64"),
        ["https://registry.npmjs.org/@oven/bun-windows-x64/-/bun-windows-x64-2.0.0.tgz"]
    );
}

#[test]
fn collects_setup_steps_and_overrides_without_duplicate_downloads() {
    let mut manifest = BunPrewarm::default();
    manifest.package_json(&json!({"packageManager": "bun@1.2.3"}));
    manifest.env([("VP_BUN_VERSION", "1.2.3"), ("VP_PACKAGE_MANAGER", "bun@2.0.0")]);
    step(&mut manifest, &["vpt", "json-edit", "sub/package.json", "packageManager", "bun@3.0.0"]);
    step(
        &mut manifest,
        &[
            "vpt",
            "json-edit",
            "package.json",
            "devEngines.packageManager",
            r#"{"name":"bun","version":"4.0.0"}"#,
        ],
    );
    step(
        &mut manifest,
        &["vpt", "write-file", "package.json", r#"{"packageManager":"bun@5.0.0"}"#],
    );
    step(&mut manifest, &["vp", "env", "default", "bun@6.0.0"]);
    let urls = manifest.urls("bun-windows-x64");
    assert_eq!(urls.len(), 6);
    for (url, version) in urls.iter().zip(["1.2.3", "2.0.0", "3.0.0", "4.0.0", "5.0.0", "6.0.0"]) {
        assert!(url.ends_with(&format!("-{version}.tgz")));
    }
}

#[test]
fn changing_a_step_pin_changes_the_archive_to_prepare() {
    let mut before = BunPrewarm::default();
    let mut after = BunPrewarm::default();
    step(&mut before, &["vpt", "json-edit", "package.json", "packageManager", "bun@1.0.0"]);
    step(&mut after, &["vpt", "json-edit", "package.json", "packageManager", "bun@2.0.0"]);
    assert!(before.urls("bun-windows-x64")[0].ends_with("-1.0.0.tgz"));
    assert!(after.urls("bun-windows-x64")[0].ends_with("-2.0.0.tgz"));
}

#[test]
fn ignores_assertions_other_managers_and_floating_specs() {
    let mut manifest = BunPrewarm::default();
    manifest.package_json(
        &json!({"packageManager": "pnpm@1.0.0", "devDependencies": {"bun":"99.0.0"}}),
    );
    manifest.env([("EXPECTED_BUN_VERSION", "99.0.0"), ("VP_BUN_VERSION", "latest")]);
    step(&mut manifest, &["vpt", "print", "bun@99.0.0"]);
    step(
        &mut manifest,
        &["vpt", "write-file", "example.json", r#"{"packageManager":"bun@99.0.0"}"#],
    );
    step(&mut manifest, &["node", "-e", "console.log('bun@99.0.0')"]);
    for spec in ["bun@latest", "bun@^1.0.0", "bun@>=2", "bun@1.2"] {
        manifest.package_json(&json!({"packageManager": spec}));
    }
    assert!(manifest.urls(&prewarm::platform_package()).is_empty());
}

#[test]
fn supports_dev_engine_arrays_and_integrity_pins() {
    let mut manifest = BunPrewarm::default();
    manifest.package_json(&json!({"devEngines":{"packageManager":[{"name":"pnpm","version":"99.0.0"},{"name":"bun","version":"1.2.3"}]}}));
    manifest.package_json(&json!({"packageManager":"bun@v2.0.0-beta.1+sha512.deadbeef"}));
    let urls = manifest.urls("bun-windows-x64-baseline");
    assert_eq!(urls.len(), 2);
    assert!(urls[0].ends_with("bun-windows-x64-baseline-1.2.3.tgz"));
    assert!(urls[1].ends_with("bun-windows-x64-baseline-2.0.0-beta.1.tgz"));
}
