//! Cross-process file locks for asynchronous installation workflows.

use std::{
    fs::{File, OpenOptions},
    io,
};

use vt_path::AbsolutePath;

/// Wait for an exclusive lock without blocking the async runtime.
///
/// The parent directory must exist. Keep the returned file open until the
/// protected operation finishes, and leave the lock file in place so other
/// processes continue to lock the same inode.
pub async fn lock_exclusive(path: &AbsolutePath) -> io::Result<File> {
    let path = path.to_absolute_path_buf();
    tokio::task::spawn_blocking(move || {
        // Truncation can fail on Windows while another process holds the lock.
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path.as_path())?;
        file.lock()?;
        Ok(file)
    })
    .await
    .map_err(io::Error::other)?
}

#[cfg(test)]
mod tests {
    use std::{fs, time::Duration};

    use vt_path::AbsolutePathBuf;

    use super::*;

    #[tokio::test]
    async fn waits_without_truncating_and_releases_on_drop() {
        let temp = tempfile::tempdir().unwrap();
        let path = AbsolutePathBuf::new(temp.path().join("install.lock")).unwrap();
        fs::write(&path, "keep contents").unwrap();

        let first = lock_exclusive(&path).await.unwrap();
        let second = lock_exclusive(&path);
        tokio::pin!(second);
        assert!(tokio::time::timeout(Duration::from_millis(150), &mut second).await.is_err());
        drop(first);
        let second = tokio::time::timeout(Duration::from_secs(5), second).await.unwrap().unwrap();

        let contender = File::open(&path).unwrap();
        assert!(matches!(contender.try_lock(), Err(fs::TryLockError::WouldBlock)));
        drop(second);
        assert!(contender.try_lock().is_ok());
        drop(contender);
        assert_eq!(fs::read_to_string(&path).unwrap(), "keep contents");
    }

    #[tokio::test]
    async fn different_paths_lock_independently() {
        let temp = tempfile::tempdir().unwrap();
        let root = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let _install = lock_exclusive(&root.join(".install.lock")).await.unwrap();
        let _setup =
            tokio::time::timeout(Duration::from_secs(5), lock_exclusive(&root.join("setup.lock")))
                .await
                .unwrap()
                .unwrap();
    }

    #[tokio::test]
    async fn open_errors_are_returned() {
        let temp = tempfile::tempdir().unwrap();
        let path = AbsolutePathBuf::new(temp.path().join("missing/install.lock")).unwrap();
        let error = lock_exclusive(&path).await.unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }
}
