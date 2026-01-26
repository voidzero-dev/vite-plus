use std::{fs::File, time::Duration};

use backon::{ExponentialBuilder, Retryable};
use directories::BaseDirs;
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use tar::Archive;
use tempfile::TempDir;
use tokio::{fs, io::AsyncWriteExt};
use vite_path::{AbsolutePath, AbsolutePathBuf, current_dir};
use vite_str::Str;

use crate::{Error, Platform, node};

/// Supported JavaScript runtime types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsRuntimeType {
    Node,
    // Future: Bun, Deno
}

impl std::fmt::Display for JsRuntimeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Node => write!(f, "node"),
        }
    }
}

/// Represents a downloaded JavaScript runtime
#[derive(Debug)]
pub struct JsRuntime {
    pub runtime_type: JsRuntimeType,
    pub version: Str,
    pub install_dir: AbsolutePathBuf,
}

impl JsRuntime {
    /// Get the path to the runtime binary (e.g., node, bun)
    #[must_use]
    pub fn get_binary_path(&self) -> AbsolutePathBuf {
        match self.runtime_type {
            JsRuntimeType::Node => {
                #[cfg(target_os = "windows")]
                {
                    self.install_dir.join("node.exe")
                }
                #[cfg(not(target_os = "windows"))]
                {
                    self.install_dir.join("bin/node")
                }
            }
        }
    }

    /// Get the bin directory containing the runtime
    #[must_use]
    pub fn get_bin_prefix(&self) -> AbsolutePathBuf {
        match self.runtime_type {
            JsRuntimeType::Node => {
                #[cfg(target_os = "windows")]
                {
                    self.install_dir.clone()
                }
                #[cfg(not(target_os = "windows"))]
                {
                    self.install_dir.join("bin")
                }
            }
        }
    }

    /// Get the runtime type
    #[must_use]
    pub const fn runtime_type(&self) -> JsRuntimeType {
        self.runtime_type
    }

    /// Get the version string
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }
}

/// Download and cache a JavaScript runtime
///
/// # Arguments
/// * `runtime_type` - The type of runtime to download
/// * `version` - The exact version (e.g., "22.13.1")
///
/// # Returns
/// A `JsRuntime` instance with the installation path
///
/// # Errors
/// Returns an error if download, verification, or extraction fails
pub async fn download_runtime(
    runtime_type: JsRuntimeType,
    version: &str,
) -> Result<JsRuntime, Error> {
    match runtime_type {
        JsRuntimeType::Node => download_node(version).await,
    }
}

/// Get the cache directory for JavaScript runtimes
fn get_cache_dir() -> Result<AbsolutePathBuf, Error> {
    let cache_dir = match BaseDirs::new() {
        Some(dirs) => AbsolutePathBuf::new(dirs.cache_dir().to_path_buf()).unwrap(),
        None => current_dir()?.join(".cache"),
    };
    Ok(cache_dir.join("vite/js_runtime"))
}

/// Download and cache Node.js
async fn download_node(version: &str) -> Result<JsRuntime, Error> {
    let platform = Platform::current();
    let cache_dir = get_cache_dir()?;

    // Cache path: $CACHE_DIR/vite/js_runtime/node/{version}/{platform}/
    let install_dir = cache_dir.join(vite_str::format!("node/{version}/{platform}"));

    // Check if already cached
    let binary_path = get_node_binary_path(&install_dir);
    if tokio::fs::try_exists(&binary_path).await.unwrap_or(false) {
        tracing::debug!("Node.js {version} already cached at {install_dir:?}");
        return Ok(JsRuntime {
            runtime_type: JsRuntimeType::Node,
            version: version.into(),
            install_dir,
        });
    }

    tracing::info!("Downloading Node.js {version} for {platform}...");

    // Get download URLs
    let archive_filename = node::get_archive_filename(version, platform);
    let download_url = node::get_download_url(version, platform);
    let shasums_url = node::get_shasums_url(version);

    // Create temp directory for download
    // TempDir::path() always returns an absolute path (e.g., /tmp/xxx)
    let temp_dir = TempDir::new()?;
    let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
    let archive_path = temp_path.join(&archive_filename);

    // Download SHASUMS256.txt and parse expected hash
    let expected_hash = download_and_parse_shasums(&shasums_url, &archive_filename).await?;

    // Download archive
    download_file(&download_url, &archive_path).await?;

    // Verify hash
    verify_file_hash(&archive_path, &expected_hash, &archive_filename).await?;

    // Extract archive
    let extracted_dir_name = node::get_extracted_dir_name(version, platform);
    extract_archive(&archive_path, &temp_path, platform).await?;

    // Move extracted directory to cache location with file-based locking
    let extracted_path = temp_path.join(&extracted_dir_name);
    move_to_cache(&extracted_path, &install_dir, version).await?;

    tracing::info!("Node.js {version} installed at {install_dir:?}");

    Ok(JsRuntime { runtime_type: JsRuntimeType::Node, version: version.into(), install_dir })
}

/// Get the Node.js binary path for a given install directory
fn get_node_binary_path(install_dir: &AbsolutePathBuf) -> AbsolutePathBuf {
    #[cfg(target_os = "windows")]
    {
        install_dir.join("node.exe")
    }
    #[cfg(not(target_os = "windows"))]
    {
        install_dir.join("bin/node")
    }
}

/// Download SHASUMS256.txt and parse the expected hash for a filename
async fn download_and_parse_shasums(shasums_url: &str, filename: &str) -> Result<Str, Error> {
    tracing::debug!("Downloading SHASUMS256.txt from {shasums_url}");

    let content = (|| async { reqwest::get(shasums_url).await?.text().await })
        .retry(
            ExponentialBuilder::default()
                .with_jitter()
                .with_min_delay(Duration::from_millis(500))
                .with_max_times(3),
        )
        .await
        .map_err(|e| Error::DownloadFailed {
            url: shasums_url.into(),
            reason: vite_str::format!("{e}"),
        })?;

    node::parse_shasums(&content, filename)
}

/// Download a file with retry logic
async fn download_file(url: &str, target_path: &AbsolutePath) -> Result<(), Error> {
    tracing::debug!("Downloading {url} to {target_path:?}");

    let response = (|| async { reqwest::get(url).await?.error_for_status() })
        .retry(
            ExponentialBuilder::default()
                .with_jitter()
                .with_min_delay(Duration::from_millis(500))
                .with_max_times(3),
        )
        .await
        .map_err(|e| Error::DownloadFailed { url: url.into(), reason: vite_str::format!("{e}") })?;

    // Stream to file
    let mut file = fs::File::create(target_path).await?;
    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        file.write_all(&chunk).await?;
    }

    file.flush().await?;
    tracing::debug!("Download completed: {target_path:?}");

    Ok(())
}

/// Verify file hash against expected SHA256 hash
async fn verify_file_hash(
    file_path: &AbsolutePath,
    expected_hash: &str,
    filename: &str,
) -> Result<(), Error> {
    tracing::debug!("Verifying hash for {filename}");

    let content = fs::read(file_path).await?;

    let mut hasher = Sha256::new();
    hasher.update(&content);
    let actual_hash: Str = hex::encode(hasher.finalize()).into();

    if actual_hash != expected_hash {
        return Err(Error::HashMismatch {
            filename: filename.into(),
            expected: expected_hash.into(),
            actual: actual_hash,
        });
    }

    tracing::debug!("Hash verification successful for {filename}");
    Ok(())
}

/// Extract archive based on platform
async fn extract_archive(
    archive_path: &AbsolutePath,
    target_dir: &AbsolutePath,
    platform: Platform,
) -> Result<(), Error> {
    let archive_path = AbsolutePathBuf::new(archive_path.as_path().to_path_buf()).unwrap();
    let target_dir = AbsolutePathBuf::new(target_dir.as_path().to_path_buf()).unwrap();
    let is_windows = platform.os == crate::platform::Os::Windows;

    tokio::task::spawn_blocking(move || {
        if is_windows {
            extract_zip(&archive_path, &target_dir)
        } else {
            extract_tar_gz(&archive_path, &target_dir)
        }
    })
    .await??;

    Ok(())
}

/// Extract a tar.gz archive
fn extract_tar_gz(archive_path: &AbsolutePath, target_dir: &AbsolutePath) -> Result<(), Error> {
    tracing::debug!("Extracting tar.gz: {archive_path:?} to {target_dir:?}");

    let file = File::open(archive_path)?;
    let tar_stream = GzDecoder::new(file);
    let mut archive = Archive::new(tar_stream);
    archive.unpack(target_dir)?;

    tracing::debug!("Extraction completed");
    Ok(())
}

/// Extract a zip archive (Windows)
#[cfg(target_os = "windows")]
fn extract_zip(archive_path: &AbsolutePath, target_dir: &AbsolutePath) -> Result<(), Error> {
    tracing::debug!("Extracting zip: {archive_path:?} to {target_dir:?}");

    let file = File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| Error::ExtractionFailed { reason: vite_str::format!("{e}") })?;

    archive
        .extract(target_dir)
        .map_err(|e| Error::ExtractionFailed { reason: vite_str::format!("{e}") })?;

    tracing::debug!("Extraction completed");
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn extract_zip(_archive_path: &AbsolutePath, _target_dir: &AbsolutePath) -> Result<(), Error> {
    // This should never be called on non-Windows platforms
    Err(Error::ExtractionFailed { reason: "Zip extraction not supported on this platform".into() })
}

/// Move extracted directory to cache location with atomic operations and file-based locking
///
/// Uses a file-based lock to ensure atomicity when multiple processes/threads
/// try to install the same runtime version concurrently.
async fn move_to_cache(
    source: &AbsolutePath,
    target: &AbsolutePathBuf,
    version: &str,
) -> Result<(), Error> {
    // Create parent directory
    let parent = target.parent().ok_or_else(|| Error::ExtractionFailed {
        reason: "Target path has no parent directory".into(),
    })?;
    fs::create_dir_all(&parent).await?;

    // Use a file-based lock to ensure atomicity of the move operation.
    // This prevents race conditions when multiple processes/threads
    // try to install the same runtime version concurrently.
    let lock_path = parent.join(vite_str::format!("{version}.lock"));
    tracing::debug!("Acquiring lock file: {lock_path:?}");

    // Acquire file lock in a blocking task to avoid blocking the async runtime.
    // The lock() call blocks until the lock is acquired.
    let lock_path_clone = lock_path.clone();
    tokio::task::spawn_blocking(move || {
        let lock_file = File::create(lock_path_clone.as_path())?;
        // Acquire exclusive lock (blocks until available)
        lock_file.lock()?;
        tracing::debug!("Lock acquired: {lock_path_clone:?}");
        Ok::<_, std::io::Error>(lock_file)
    })
    .await??;
    tracing::debug!("Lock acquired: {lock_path:?}");

    // Check again after acquiring the lock, in case another process completed
    // the installation while we were downloading
    if fs::try_exists(target.as_path()).await.unwrap_or(false) {
        tracing::debug!("Target already exists after lock acquisition, skipping move: {target:?}");
        return Ok(());
    }

    // Try atomic rename first
    if fs::rename(source.as_path(), target.as_path()).await.is_ok() {
        tracing::debug!("Atomic rename successful: {source:?} -> {target:?}");
        return Ok(());
    }

    // If rename fails (cross-device), fall back to copy
    tracing::debug!("Atomic rename failed, falling back to copy: {source:?} -> {target:?}");
    copy_dir_recursive(source, target).await?;
    fs::remove_dir_all(source).await?;

    Ok(())
}

/// Recursively copy a directory
async fn copy_dir_recursive(src: &AbsolutePath, dst: &AbsolutePath) -> Result<(), Error> {
    fs::create_dir_all(dst).await?;

    let mut entries = fs::read_dir(src).await?;
    while let Some(entry) = entries.next_entry().await? {
        // entry.path() returns absolute path when src is absolute
        let src_path = AbsolutePathBuf::new(entry.path()).unwrap();
        let dst_path = dst.join(entry.file_name());

        if entry.file_type().await?.is_dir() {
            Box::pin(copy_dir_recursive(&src_path, &dst_path)).await?;
        } else {
            fs::copy(&src_path, &dst_path).await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_js_runtime_type_display() {
        assert_eq!(JsRuntimeType::Node.to_string(), "node");
    }

    /// Integration test that downloads a real Node.js version
    #[tokio::test]
    async fn test_download_node_integration() {
        // Use a small, old version for faster download
        let version = "20.18.0";

        let runtime = download_runtime(JsRuntimeType::Node, version).await.unwrap();

        assert_eq!(runtime.runtime_type(), JsRuntimeType::Node);
        assert_eq!(runtime.version(), version);

        // Verify the binary exists
        let binary_path = runtime.get_binary_path();
        assert!(tokio::fs::try_exists(&binary_path).await.unwrap());

        // Verify binary is executable by checking version
        let output = tokio::process::Command::new(binary_path.as_path())
            .arg("--version")
            .output()
            .await
            .unwrap();

        assert!(output.status.success());
        let version_output = String::from_utf8_lossy(&output.stdout);
        assert!(version_output.contains(version));
    }

    /// Test cache reuse - second call should be instant
    #[tokio::test]
    async fn test_download_node_cache_reuse() {
        let version = "20.18.0";

        // First download
        let runtime1 = download_runtime(JsRuntimeType::Node, version).await.unwrap();

        // Second download should use cache
        let start = std::time::Instant::now();
        let runtime2 = download_runtime(JsRuntimeType::Node, version).await.unwrap();
        let elapsed = start.elapsed();

        // Cache hit should be very fast (< 100ms)
        assert!(elapsed.as_millis() < 100, "Cache reuse took too long: {elapsed:?}");

        // Should return same install directory
        assert_eq!(runtime1.install_dir, runtime2.install_dir);
    }

    /// Test concurrent downloads - multiple tasks downloading the same version
    /// should not cause corruption or conflicts due to file-based locking
    #[tokio::test]
    async fn test_concurrent_downloads() {
        // Use a different version to avoid conflicts with other tests
        let version = "20.17.0";

        // Clear any existing cache for this version
        let cache_dir = get_cache_dir().unwrap();
        let install_dir = cache_dir.join(vite_str::format!("node/{version}"));
        if tokio::fs::try_exists(&install_dir).await.unwrap_or(false) {
            tokio::fs::remove_dir_all(&install_dir).await.unwrap();
        }

        // Spawn multiple concurrent download tasks
        let num_concurrent = 4;
        let mut handles = Vec::with_capacity(num_concurrent);

        for i in 0..num_concurrent {
            let version = version.to_string();
            handles.push(tokio::spawn(async move {
                tracing::info!("Starting concurrent download task {i}");
                let result = download_runtime(JsRuntimeType::Node, &version).await;
                tracing::info!("Completed concurrent download task {i}");
                result
            }));
        }

        // Wait for all tasks and collect results
        let mut results = Vec::with_capacity(num_concurrent);
        for handle in handles {
            results.push(handle.await.unwrap());
        }

        // All tasks should succeed
        for (i, result) in results.iter().enumerate() {
            assert!(result.is_ok(), "Task {i} failed: {:?}", result.as_ref().err());
        }

        // All tasks should return the same install directory
        let first_install_dir = &results[0].as_ref().unwrap().install_dir;
        for (i, result) in results.iter().enumerate().skip(1) {
            assert_eq!(
                &result.as_ref().unwrap().install_dir,
                first_install_dir,
                "Task {i} has different install_dir"
            );
        }

        // Verify the binary works
        let runtime = results.into_iter().next().unwrap().unwrap();
        let binary_path = runtime.get_binary_path();
        assert!(
            tokio::fs::try_exists(&binary_path).await.unwrap(),
            "Binary should exist at {binary_path:?}"
        );

        let output = tokio::process::Command::new(binary_path.as_path())
            .arg("--version")
            .output()
            .await
            .unwrap();

        assert!(output.status.success(), "Binary should be executable");
        let version_output = String::from_utf8_lossy(&output.stdout);
        assert!(
            version_output.contains(version),
            "Version output should contain {version}, got: {version_output}"
        );
    }
}
