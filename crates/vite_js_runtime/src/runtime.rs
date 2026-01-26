use std::{fs::File, path::Path, time::Duration};

use backon::{ExponentialBuilder, Retryable};
use directories::BaseDirs;
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use tar::Archive;
use tempfile::TempDir;
use tokio::{fs, io::AsyncWriteExt};
use vite_path::{AbsolutePathBuf, current_dir};
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

/// Parse a runtime specification string (e.g., "node@22.13.1")
///
/// # Arguments
/// * `spec` - The runtime specification string
///
/// # Returns
/// A tuple of (runtime type, version string)
///
/// # Errors
/// Returns an error if the spec format is invalid or the runtime is unsupported
pub fn parse_runtime_spec(spec: &str) -> Result<(JsRuntimeType, String), Error> {
    let parts: Vec<&str> = spec.splitn(2, '@').collect();
    if parts.len() != 2 {
        return Err(Error::InvalidRuntimeSpec { spec: spec.into() });
    }

    let runtime_name = parts[0];
    let version = parts[1];

    if version.is_empty() {
        return Err(Error::InvalidRuntimeSpec { spec: spec.into() });
    }

    let runtime_type = match runtime_name {
        "node" => JsRuntimeType::Node,
        _ => return Err(Error::UnsupportedRuntime { runtime: runtime_name.into() }),
    };

    Ok((runtime_type, version.to_string()))
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
    let install_dir = cache_dir.join(format!("node/{version}/{platform}"));

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
    let temp_dir = TempDir::new()?;
    let archive_path = temp_dir.path().join(&archive_filename);

    // Download SHASUMS256.txt and parse expected hash
    let expected_hash = download_and_parse_shasums(&shasums_url, &archive_filename).await?;

    // Download archive
    download_file(&download_url, &archive_path).await?;

    // Verify hash
    verify_file_hash(&archive_path, &expected_hash, &archive_filename).await?;

    // Extract archive
    let extracted_dir_name = node::get_extracted_dir_name(version, platform);
    extract_archive(&archive_path, temp_dir.path(), platform).await?;

    // Move extracted directory to cache location
    let extracted_path = temp_dir.path().join(&extracted_dir_name);
    move_to_cache(&extracted_path, &install_dir).await?;

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
async fn download_and_parse_shasums(shasums_url: &str, filename: &str) -> Result<String, Error> {
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
            reason: e.to_string().into(),
        })?;

    node::parse_shasums(&content, filename)
}

/// Download a file with retry logic
async fn download_file(url: &str, target_path: &Path) -> Result<(), Error> {
    tracing::debug!("Downloading {url} to {target_path:?}");

    let response = (|| async { reqwest::get(url).await?.error_for_status() })
        .retry(
            ExponentialBuilder::default()
                .with_jitter()
                .with_min_delay(Duration::from_millis(500))
                .with_max_times(3),
        )
        .await
        .map_err(|e| Error::DownloadFailed { url: url.into(), reason: e.to_string().into() })?;

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
    file_path: &Path,
    expected_hash: &str,
    filename: &str,
) -> Result<(), Error> {
    tracing::debug!("Verifying hash for {filename}");

    let content = fs::read(file_path).await?;

    let mut hasher = Sha256::new();
    hasher.update(&content);
    let actual_hash = hex::encode(hasher.finalize());

    if actual_hash != expected_hash {
        return Err(Error::HashMismatch {
            filename: filename.into(),
            expected: expected_hash.into(),
            actual: actual_hash.into(),
        });
    }

    tracing::debug!("Hash verification successful for {filename}");
    Ok(())
}

/// Extract archive based on platform
async fn extract_archive(
    archive_path: &Path,
    target_dir: &Path,
    platform: Platform,
) -> Result<(), Error> {
    let archive_path = archive_path.to_path_buf();
    let target_dir = target_dir.to_path_buf();
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
fn extract_tar_gz(archive_path: &Path, target_dir: &Path) -> Result<(), Error> {
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
fn extract_zip(archive_path: &Path, target_dir: &Path) -> Result<(), Error> {
    tracing::debug!("Extracting zip: {archive_path:?} to {target_dir:?}");

    let file = File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| Error::ExtractionFailed { reason: e.to_string().into() })?;

    archive
        .extract(target_dir)
        .map_err(|e| Error::ExtractionFailed { reason: e.to_string().into() })?;

    tracing::debug!("Extraction completed");
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn extract_zip(_archive_path: &Path, _target_dir: &Path) -> Result<(), Error> {
    // This should never be called on non-Windows platforms
    Err(Error::ExtractionFailed { reason: "Zip extraction not supported on this platform".into() })
}

/// Move extracted directory to cache location with atomic operations
async fn move_to_cache(source: &Path, target: &AbsolutePathBuf) -> Result<(), Error> {
    // Create parent directory
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).await?;
    }

    // If target already exists (race condition), check if it's valid
    if fs::try_exists(target.as_path()).await.unwrap_or(false) {
        tracing::debug!("Target already exists, assuming another process completed: {target:?}");
        return Ok(());
    }

    // Try atomic rename first
    if fs::rename(source, target.as_path()).await.is_ok() {
        return Ok(());
    }

    // If rename fails (cross-device), fall back to copy
    copy_dir_recursive(source, target.as_path()).await?;
    fs::remove_dir_all(source).await?;

    Ok(())
}

/// Recursively copy a directory
async fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), Error> {
    fs::create_dir_all(dst).await?;

    let mut entries = fs::read_dir(src).await?;
    while let Some(entry) = entries.next_entry().await? {
        let src_path = entry.path();
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
    fn test_parse_runtime_spec_valid() {
        let (runtime_type, version) = parse_runtime_spec("node@22.13.1").unwrap();
        assert_eq!(runtime_type, JsRuntimeType::Node);
        assert_eq!(version, "22.13.1");
    }

    #[test]
    fn test_parse_runtime_spec_invalid_no_at() {
        let result = parse_runtime_spec("node22.13.1");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_runtime_spec_invalid_empty_version() {
        let result = parse_runtime_spec("node@");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_runtime_spec_unsupported_runtime() {
        let result = parse_runtime_spec("unknown@1.0.0");
        assert!(result.is_err());
    }

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
}
