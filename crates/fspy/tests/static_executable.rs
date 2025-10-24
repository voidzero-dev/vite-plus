#![cfg(target_os = "linux")]

use std::{
    fs::{self, Permissions},
    os::unix::fs::PermissionsExt as _,
    path::{Path, PathBuf},
    str::from_utf8,
    sync::{LazyLock, OnceLock},
};

use bstr::{B, BStr};

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
        let output = std::process::Command::new("ldd")
            .arg(&test_bin_path)
            .output()
            .expect("failed to run ldd");
        let stderr = from_utf8(&output.stderr).unwrap().trim();
        assert_eq!(stderr, "not a dynamic executable");

        test_bin_path
    });
    TEST_BIN_PATH.as_path()
}

#[tokio::test]
async fn static_executable() {
    let mut cmd = fspy::Spy::global().unwrap().new_command(test_bin_path());
    // cmd.envs(std::env::vars_os());
    let mut tracked_child = cmd.spawn().await.unwrap();

    tracked_child.tokio_child.wait().await.unwrap();
}
