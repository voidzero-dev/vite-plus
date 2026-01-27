use directories::BaseDirs;
use tempfile::TempDir;
use vite_path::{AbsolutePathBuf, current_dir};
use vite_str::Str;

use crate::{
    Error, Platform,
    download::{download_file, download_text, extract_archive, move_to_cache, verify_file_hash},
    provider::{HashVerification, JsRuntimeProvider},
    providers::NodeProvider,
};

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
    /// Relative path from `install_dir` to the binary
    binary_relative_path: Str,
    /// Relative path from `install_dir` to the bin directory
    bin_dir_relative_path: Str,
}

impl JsRuntime {
    /// Get the path to the runtime binary (e.g., node, bun)
    #[must_use]
    pub fn get_binary_path(&self) -> AbsolutePathBuf {
        self.install_dir.join(&self.binary_relative_path)
    }

    /// Get the bin directory containing the runtime
    #[must_use]
    pub fn get_bin_prefix(&self) -> AbsolutePathBuf {
        if self.bin_dir_relative_path.is_empty() {
            self.install_dir.clone()
        } else {
            self.install_dir.join(&self.bin_dir_relative_path)
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
        JsRuntimeType::Node => {
            let provider = NodeProvider::new();
            download_runtime_with_provider(&provider, JsRuntimeType::Node, version).await
        }
    }
}

/// Download and cache a JavaScript runtime using a provider
///
/// This is the generic download function that works with any `JsRuntimeProvider`.
///
/// # Errors
///
/// Returns an error if download, verification, or extraction fails.
///
/// # Panics
///
/// Panics if the temp directory path is not absolute (should not happen in practice).
pub async fn download_runtime_with_provider<P: JsRuntimeProvider>(
    provider: &P,
    runtime_type: JsRuntimeType,
    version: &str,
) -> Result<JsRuntime, Error> {
    let platform = Platform::current();
    let cache_dir = get_cache_dir()?;

    // Get paths from provider
    let platform_str = provider.platform_string(platform);
    let binary_relative_path = provider.binary_relative_path(platform);
    let bin_dir_relative_path = provider.bin_dir_relative_path(platform);

    // Cache path: $CACHE_DIR/vite/js_runtime/{runtime}/{version}/
    let install_dir = cache_dir.join(vite_str::format!("{}/{version}", provider.name()));

    // Check if already cached
    let binary_path = install_dir.join(&binary_relative_path);
    if tokio::fs::try_exists(&binary_path).await.unwrap_or(false) {
        tracing::debug!("{} {version} already cached at {install_dir:?}", provider.name());
        return Ok(JsRuntime {
            runtime_type,
            version: version.into(),
            install_dir,
            binary_relative_path,
            bin_dir_relative_path,
        });
    }

    // If install_dir exists but binary doesn't, it's an incomplete installation - clean it up
    if tokio::fs::try_exists(&install_dir).await.unwrap_or(false) {
        tracing::warn!(
            "Incomplete installation detected at {install_dir:?}, removing before re-download"
        );
        tokio::fs::remove_dir_all(&install_dir).await?;
    }

    tracing::info!("Downloading {} {version} for {platform_str}...", provider.name());

    // Get download info from provider
    let download_info = provider.get_download_info(version, platform);

    // Create temp directory for download
    let temp_dir = TempDir::new()?;
    let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
    let archive_path = temp_path.join(&download_info.archive_filename);

    // Verify hash if verification method is provided
    match &download_info.hash_verification {
        HashVerification::ShasumsFile { url } => {
            let shasums_content = download_text(url).await?;
            let expected_hash =
                provider.parse_shasums(&shasums_content, &download_info.archive_filename)?;

            // Download archive
            download_file(&download_info.archive_url, &archive_path).await?;

            // Verify hash
            verify_file_hash(&archive_path, &expected_hash, &download_info.archive_filename)
                .await?;
        }
        HashVerification::None => {
            // Download archive without verification
            download_file(&download_info.archive_url, &archive_path).await?;
        }
    }

    // Extract archive
    extract_archive(&archive_path, &temp_path, download_info.archive_format).await?;

    // Move extracted directory to cache location
    let extracted_path = temp_path.join(&download_info.extracted_dir_name);
    move_to_cache(&extracted_path, &install_dir, version).await?;

    tracing::info!("{} {version} installed at {install_dir:?}", provider.name());

    Ok(JsRuntime {
        runtime_type,
        version: version.into(),
        install_dir,
        binary_relative_path,
        bin_dir_relative_path,
    })
}

/// Get the cache directory for JavaScript runtimes
fn get_cache_dir() -> Result<AbsolutePathBuf, Error> {
    let cache_dir = match BaseDirs::new() {
        Some(dirs) => AbsolutePathBuf::new(dirs.cache_dir().to_path_buf()).unwrap(),
        None => current_dir()?.join(".cache"),
    };
    Ok(cache_dir.join("vite/js_runtime"))
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

    /// Test that incomplete installations are cleaned up and re-downloaded
    #[tokio::test]
    async fn test_incomplete_installation_cleanup() {
        // Use a different version to avoid interference with other tests
        let version = "20.18.1";

        // First, ensure we have a valid cached version
        let runtime = download_runtime(JsRuntimeType::Node, version).await.unwrap();
        let install_dir = runtime.install_dir.clone();
        let binary_path = runtime.get_binary_path();

        // Simulate an incomplete installation by removing the binary but keeping the directory
        tokio::fs::remove_file(&binary_path).await.unwrap();
        assert!(!tokio::fs::try_exists(&binary_path).await.unwrap());
        assert!(tokio::fs::try_exists(&install_dir).await.unwrap());

        // Now download again - it should detect the incomplete installation and re-download
        let runtime2 = download_runtime(JsRuntimeType::Node, version).await.unwrap();

        // Verify the binary exists again
        assert!(tokio::fs::try_exists(&runtime2.get_binary_path()).await.unwrap());

        // Verify binary is executable
        let output = tokio::process::Command::new(runtime2.get_binary_path().as_path())
            .arg("--version")
            .output()
            .await
            .unwrap();
        assert!(output.status.success());
    }

    /// Test concurrent downloads - multiple tasks downloading the same version
    /// should not cause corruption or conflicts due to file-based locking
    #[tokio::test]
    #[ignore]
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
