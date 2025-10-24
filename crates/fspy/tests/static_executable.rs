#![cfg(target_os = "linux")]

use std::{
    fs::{self, Permissions},
    os::unix::fs::PermissionsExt as _,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use fspy::PathAccessIterable;
use fspy_shared_unix::is_dynamically_linked_to_libc;

use crate::test_utils::assert_contains;

mod test_utils;

const TEST_BIN_CONTENT: &[u8] = include_bytes!(env!("CARGO_BIN_FILE_FSPY_TEST_BIN"));

fn test_bin_path() -> &'static Path {
    static TEST_BIN_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
        assert_eq!(
            is_dynamically_linked_to_libc(&TEST_BIN_CONTENT),
            Ok(false),
            "Test binary is not a static executable"
        );

        let tmp_dir = env!("CARGO_TARGET_TMPDIR");
        let test_bin_path = PathBuf::from(tmp_dir).join("fspy-test-bin");
        fs::write(&test_bin_path, TEST_BIN_CONTENT).expect("failed to write test binary");
        fs::set_permissions(&test_bin_path, Permissions::from_mode(0o755))
            .expect("failed to set permissions on test binary");

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
