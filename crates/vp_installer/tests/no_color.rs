use std::{
    io::Write as _,
    process::{Command, Stdio},
};

fn installer_command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_vp-setup"));
    command
        .env("NO_COLOR", "1")
        .env_remove("CI")
        .env_remove("VP_VERSION")
        .env_remove("VP_BIN_DIR")
        .env_remove("VP_DATA_DIR")
        .env_remove("VP_CACHE_DIR");
    command
}

fn assert_has_no_ansi(bytes: &[u8]) {
    assert!(!bytes.contains(&0x1b), "output contains an ANSI escape: {bytes:?}");
}

fn assert_contains(bytes: &[u8], expected: &[u8]) {
    assert!(
        bytes.windows(expected.len()).any(|window| window == expected),
        "output does not contain {expected:?}: {bytes:?}"
    );
}

#[test]
fn no_color_removes_ansi_from_interactive_output() {
    let temp = tempfile::tempdir().unwrap();
    let mut child = installer_command()
        .args(["--no-node-manager", "--no-modify-path"])
        .env("VP_HOME", temp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"3\n").unwrap();

    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    assert_contains(&output.stdout, b"Welcome to Vite+ Installer!");
    assert_has_no_ansi(&output.stdout);
    assert_has_no_ansi(&output.stderr);
}

#[test]
fn no_color_removes_ansi_from_error_output() {
    let output = installer_command()
        .args(["--yes", "--quiet", "--no-node-manager", "--no-modify-path"])
        .env("VP_HOME", "relative-home")
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert_contains(&output.stderr, b"Set VP_HOME to an absolute path");
    assert_has_no_ansi(&output.stdout);
    assert_has_no_ansi(&output.stderr);
}
