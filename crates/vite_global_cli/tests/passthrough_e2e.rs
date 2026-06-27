//! End-to-end: low-Node project degrades `vpr`/`vp i` to passthrough.
//!
//! Requires VP_PASSTHROUGH_E2E=1 and a pre-cached Node 14 + pnpm runtime,
//! because it exercises real downloads. Skipped otherwise.

use std::process::Command;

fn e2e_enabled() -> bool {
    std::env::var("VP_PASSTHROUGH_E2E").is_ok()
}

#[test]
fn vpr_passthrough_runs_project_script_on_low_node() {
    if !e2e_enabled() {
        eprintln!("skipping (set VP_PASSTHROUGH_E2E=1)");
        return;
    }
    // Fixture: project with .node-version=14.15.0, packageManager=pnpm@<cached>,
    // and a `dev` script that echoes a marker. Assert marker appears in stdout
    // and package.json is NOT mutated with devEngines.
    let project = setup_low_node_project("pnpm");
    let output = Command::new(env!("CARGO_BIN_EXE_vp"))
        .args(["run", "dev"])
        .current_dir(&project)
        .output()
        .expect("vp binary");
    assert!(output.status.success(), "vpr dev should passthrough");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PASSTHROUGH_MARKER"), "script must run");
    let pkg = std::fs::read_to_string(project.join("package.json")).unwrap();
    assert!(!pkg.contains("devEngines"), "must not write devEngines");
}

#[test]
fn vp_install_passthrough_on_low_node() {
    if !e2e_enabled() {
        eprintln!("skipping (set VP_PASSTHROUGH_E2E=1)");
        return;
    }
    let project = setup_low_node_project("pnpm");
    let output = Command::new(env!("CARGO_BIN_EXE_vp"))
        .args(["i"])
        .current_dir(&project)
        .output()
        .expect("vp binary");
    assert!(output.status.success(), "vp i should passthrough");
    let pkg = std::fs::read_to_string(project.join("package.json")).unwrap();
    assert!(!pkg.contains("devEngines"), "must not write devEngines");
}

fn setup_low_node_project(pm: &str) -> std::path::PathBuf {
    let dir = tempfile::tempdir().unwrap().keep();
    std::fs::write(dir.join(".node-version"), "14.15.0\n").unwrap();
    // packageManager pinned to a Node-14-compatible version (caller ensures cached).
    let version = match pm {
        "pnpm" => "7.33.0",
        "yarn" => "1.22.0",
        _ => "6.14.8",
    };
    let pkg_json = format!(
        r#"{{"name":"e2e","scripts":{{"dev":"echo PASSTHROUGH_MARKER"}},"packageManager":"{pm}@{version}"}}"#
    );
    std::fs::write(dir.join("package.json"), pkg_json).unwrap();
    dir
}
