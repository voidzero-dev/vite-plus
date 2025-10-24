#![cfg(target_os = "linux")]

use std::{
    fs::{self, Permissions},
    os::unix::fs::PermissionsExt as _,
    path::{Path, PathBuf},
    str::from_utf8,
    sync::LazyLock,
};

use fspy::PathAccessIterable;

use crate::test_utils::assert_contains;

mod test_utils;

const PRELOAD_CDYLIB_BINARY: &[u8] = include_bytes!(env!("CARGO_BIN_FILE_FSPY_TEST_BIN"));

fn test_bin_path() -> &'static Path {
    static TEST_BIN_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
        let tmp_dir = env!("CARGO_TARGET_TMPDIR");
        let test_bin_path = PathBuf::from(tmp_dir).join("fspy-test-bin");
        fs::write(&test_bin_path, PRELOAD_CDYLIB_BINARY).expect("failed to write test binary");
        fs::set_permissions(&test_bin_path, Permissions::from_mode(0o755))
            .expect("failed to set permissions on test binary");

        // Verify that the test binary is indeed a static executable
        let output = std::process::Command::new("ldd").arg(&test_bin_path).output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(1),
            "ldd should fail on static executables. Stdout: {}. Stderr: {}",
            from_utf8(&output.stdout).unwrap(),
            from_utf8(&output.stderr).unwrap()
        );

        test_bin_path
    });
    TEST_BIN_PATH.as_path()
}

async fn track_test_bin(args: &[&str]) -> PathAccessIterable {
    let mut cmd = fspy::Spy::global().unwrap().new_command(test_bin_path());
    cmd.args(args);
    let mut tracked_child = cmd.spawn().await.unwrap();

    let output = tracked_child.tokio_child.wait().await.unwrap();
    assert!(output.success());

    tracked_child.accesses_future.await.unwrap()
}

#[tokio::test]
async fn open_read() {
    let accesses = track_test_bin(&["open_read", "/hello"]).await;
    assert_contains(&accesses, Path::new("/hello"), fspy::AccessMode::Read);
}
