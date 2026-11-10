//! Serialize global bin ownership checks, activation, rollback, and cleanup.

use std::{
    fs::{File, OpenOptions, TryLockError},
    time::{Duration, Instant},
};

use crate::{commands::env::config::get_bin_dir, error::Error};

const WAIT: Duration = Duration::from_mins(5);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

fn open_lock_file() -> Result<File, Error> {
    let bin_dir = get_bin_dir()?;
    std::fs::create_dir_all(&bin_dir)?;
    // Keep the file in place so all writers lock the same inode.
    Ok(OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(bin_dir.join(".vp-global-bin.lock"))?)
}

fn try_lock(file: &File, deadline: Instant) -> Result<bool, Error> {
    match file.try_lock() {
        Ok(()) => Ok(true),
        Err(TryLockError::WouldBlock) if Instant::now() < deadline => Ok(false),
        Err(TryLockError::WouldBlock) => Err(Error::Other(
            "Timed out waiting for another global package operation to finish.".into(),
        )),
        Err(TryLockError::Error(error)) => Err(error.into()),
    }
}

/// Hold the returned file until all shared-state changes and cleanup finish.
pub(crate) async fn acquire() -> Result<File, Error> {
    let file = open_lock_file()?;
    let deadline = Instant::now() + WAIT;
    while !try_lock(&file, deadline)? {
        tokio::time::sleep(POLL_INTERVAL).await;
    }
    Ok(file)
}

/// Synchronous counterpart for npm shim dispatch, outside prompts and npm execution.
pub(crate) fn acquire_sync() -> Result<File, Error> {
    let file = open_lock_file()?;
    let deadline = Instant::now() + WAIT;
    while !try_lock(&file, deadline)? {
        std::thread::sleep(POLL_INTERVAL);
    }
    Ok(file)
}

#[cfg(test)]
mod tests {
    use vp_shared::{EnvConfig, env_vars};

    use super::*;

    #[tokio::test]
    async fn async_wait_does_not_block_and_drop_releases_lock() {
        let temp = tempfile::tempdir().unwrap();
        EnvConfig::with_vars_async([(env_vars::VP_HOME, temp.path())], |_| async {
            let first = acquire_sync().unwrap();
            let second = acquire();
            tokio::pin!(second);
            assert!(tokio::time::timeout(Duration::from_millis(150), &mut second).await.is_err());
            drop(first);
            let second =
                tokio::time::timeout(Duration::from_secs(5), second).await.unwrap().unwrap();
            let contender = open_lock_file().unwrap();
            assert!(try_lock(&contender, Instant::now()).is_err());
            drop(second);
            assert!(try_lock(&contender, Instant::now()).unwrap());
        })
        .await;
    }

    #[test]
    fn lock_child_process() {
        let Some(marker_dir) = std::env::var_os("VP_TEST_GLOBAL_LOCK_MARKERS") else { return };
        let marker_dir = std::path::PathBuf::from(marker_dir);
        std::fs::write(marker_dir.join("started"), b"").unwrap();
        let _lock = acquire_sync().unwrap();
        std::fs::write(marker_dir.join("acquired"), b"").unwrap();
        let mut byte = [0];
        std::io::Read::read(&mut std::io::stdin(), &mut byte).unwrap();
    }

    #[tokio::test]
    async fn serializes_processes_and_releases_lock_after_kill() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        let spawn = |name: &str| {
            let markers = temp.path().join(name);
            std::fs::create_dir_all(&markers).unwrap();
            let child = tokio::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "commands::global::bin_lock::tests::lock_child_process"])
                .env(env_vars::VP_HOME, &home)
                .env("VP_TEST_GLOBAL_LOCK_MARKERS", &markers)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::null())
                .kill_on_drop(true)
                .spawn()
                .unwrap();
            (child, markers)
        };
        let wait_for = |path: std::path::PathBuf| async move {
            tokio::time::timeout(Duration::from_secs(10), async {
                while !path.exists() {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap();
        };
        let (mut first, first_markers) = spawn("first");
        wait_for(first_markers.join("acquired")).await;
        let (mut second, second_markers) = spawn("second");
        wait_for(second_markers.join("started")).await;
        assert!(
            tokio::time::timeout(
                Duration::from_millis(150),
                wait_for(second_markers.join("acquired"))
            )
            .await
            .is_err()
        );
        first.kill().await.unwrap();
        wait_for(second_markers.join("acquired")).await;
        drop(second.stdin.take());
        assert!(
            tokio::time::timeout(Duration::from_secs(5), second.wait())
                .await
                .unwrap()
                .unwrap()
                .success()
        );
    }

    #[test]
    fn contention_times_out_without_taking_lock() {
        let temp = tempfile::tempdir().unwrap();
        EnvConfig::with_vars([(env_vars::VP_HOME, temp.path())], |_| {
            let _first = acquire_sync().unwrap();
            let second = open_lock_file().unwrap();
            let error = try_lock(&second, Instant::now()).unwrap_err();
            assert!(error.to_string().contains("Timed out waiting"));
            assert!(matches!(second.try_lock(), Err(TryLockError::WouldBlock)));
        });
    }
}
