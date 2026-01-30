use node_semver::{Range, Version};
use tempfile::TempDir;
use vite_path::{AbsolutePath, AbsolutePathBuf};
use vite_str::Str;

use crate::{
    Error, Platform,
    dev_engines::{PackageJson, read_node_version_file, write_node_version_file},
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
    let cache_dir = crate::cache::get_cache_dir()?;

    // Get paths from provider
    let binary_relative_path = provider.binary_relative_path(platform);
    let bin_dir_relative_path = provider.bin_dir_relative_path(platform);

    // Cache path: $CACHE_DIR/vite-plus/js_runtime/{runtime}/{version}/
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

    let download_message = format!("Downloading {} v{version}...", provider.name());
    tracing::info!("{download_message}");

    // Get download info from provider
    let download_info = provider.get_download_info(version, platform);

    // Create temp directory for download under cache_dir to ensure rename works
    // (rename fails with EXDEV if source and target are on different filesystems)
    tokio::fs::create_dir_all(&cache_dir).await?;
    let temp_dir = TempDir::new_in(&cache_dir)?;
    let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
    let archive_path = temp_path.join(&download_info.archive_filename);

    // Verify hash if verification method is provided
    match &download_info.hash_verification {
        HashVerification::ShasumsFile { url } => {
            let shasums_content = download_text(url).await?;
            let expected_hash =
                provider.parse_shasums(&shasums_content, &download_info.archive_filename)?;

            // Download archive
            download_file(&download_info.archive_url, &archive_path, &download_message).await?;

            // Verify hash
            verify_file_hash(&archive_path, &expected_hash, &download_info.archive_filename)
                .await?;
        }
        HashVerification::None => {
            // Download archive without verification
            download_file(&download_info.archive_url, &archive_path, &download_message).await?;
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

/// Represents the source from which a Node.js version was read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionSource {
    /// Version from `.node-version` file (highest priority)
    NodeVersionFile,
    /// Version from `engines.node` in package.json
    EnginesNode,
    /// Version from `devEngines.runtime` in package.json (lowest priority)
    DevEnginesRuntime,
    /// No version source specified, will use latest installed or LTS
    None,
}

impl std::fmt::Display for VersionSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NodeVersionFile => write!(f, ".node-version"),
            Self::EnginesNode => write!(f, "engines.node"),
            Self::DevEnginesRuntime => write!(f, "devEngines.runtime"),
            Self::None => write!(f, "none"),
        }
    }
}

/// Download runtime based on project's version configuration.
///
/// Reads Node.js version from multiple sources with the following priority:
/// 1. `.node-version` file (highest)
/// 2. `engines.node` in package.json
/// 3. `devEngines.runtime` in package.json (lowest)
///
/// If no version source is found, uses the latest installed version from cache,
/// or falls back to the latest LTS version from the network.
///
/// When the resolved version from the highest priority source does NOT satisfy
/// constraints from lower priority sources, a warning is emitted.
///
/// # Arguments
/// * `project_path` - The path to the project directory
///
/// # Returns
/// A `JsRuntime` instance with the installation path
///
/// # Errors
/// Returns an error if version resolution fails or download/extraction fails.
///
/// # Note
/// Currently only supports Node.js runtime.
pub async fn download_runtime_for_project(project_path: &AbsolutePath) -> Result<JsRuntime, Error> {
    let package_json_path = project_path.join("package.json");
    let pkg = read_package_json(&package_json_path).await?;
    let provider = NodeProvider::new();
    let cache_dir = crate::cache::get_cache_dir()?;

    // 1. Read all version sources (with validation)
    let node_version_file = read_node_version_file(project_path)
        .await
        .and_then(|v| normalize_version(&v, ".node-version"));

    let engines_node = pkg
        .as_ref()
        .and_then(|p| p.engines.as_ref())
        .and_then(|e| e.node.clone())
        .and_then(|v| normalize_version(&v, "engines.node"));

    let dev_engines_runtime = pkg
        .as_ref()
        .and_then(|p| p.dev_engines.as_ref())
        .and_then(|de| de.runtime.as_ref())
        .and_then(|rt| rt.find_by_name("node"))
        .map(|r| r.version.clone())
        .filter(|v| !v.is_empty())
        .and_then(|v| normalize_version(&v, "devEngines.runtime"));

    tracing::debug!(
        "Version sources - .node-version: {:?}, engines.node: {:?}, devEngines.runtime: {:?}",
        node_version_file,
        engines_node,
        dev_engines_runtime
    );

    // 2. Select version from highest priority source that exists
    let (version_req, source) = if let Some(ref v) = node_version_file {
        (v.clone(), VersionSource::NodeVersionFile)
    } else if let Some(ref v) = engines_node {
        (v.clone(), VersionSource::EnginesNode)
    } else if let Some(ref v) = dev_engines_runtime {
        (v.clone(), VersionSource::DevEnginesRuntime)
    } else {
        (Str::default(), VersionSource::None)
    };

    tracing::debug!("Selected version source: {source}, version_req: {version_req:?}");

    // 3. Resolve version (if range/partial → exact)
    let (version, should_write_back) =
        resolve_version_for_project(&version_req, source, &provider, &cache_dir).await?;

    // 4. Check compatibility with lower priority sources
    check_version_compatibility(&version, source, &engines_node, &dev_engines_runtime);

    tracing::info!("Resolved Node.js version: {version}");
    let runtime = download_runtime(JsRuntimeType::Node, &version).await?;

    // 5. Write resolved version to .node-version (if resolution occurred)
    if should_write_back {
        if let Err(e) = write_node_version_file(project_path, &version).await {
            tracing::warn!("Failed to write .node-version: {e}");
        } else {
            tracing::info!("Using Node {version} - saved version to .node-version");
        }
    }

    Ok(runtime)
}

/// Resolve version requirement to an exact version.
///
/// Returns (resolved_version, should_write_back).
async fn resolve_version_for_project(
    version_req: &str,
    _source: VersionSource,
    provider: &NodeProvider,
    cache_dir: &AbsolutePath,
) -> Result<(Str, bool), Error> {
    if version_req.is_empty() {
        // No source specified - fetch latest LTS from network
        tracing::debug!("No version source specified, fetching latest LTS from network");
        let version = provider.resolve_latest_version().await?;
        return Ok((version, true));
    }

    // Check if it's an exact version
    if NodeProvider::is_exact_version(version_req) {
        let normalized = version_req.strip_prefix('v').unwrap_or(version_req);
        tracing::debug!("Using exact version: {normalized}");
        // Never write back exact versions - user explicitly specified the version
        return Ok((normalized.into(), false));
    }

    // Check local cache first
    if let Some(cached) = provider.find_cached_version(version_req, cache_dir).await? {
        tracing::debug!("Found cached version {cached} satisfying {version_req}");
        // Don't write back - user specified a version requirement
        return Ok((cached, false));
    }

    // Resolve from network
    tracing::debug!("Resolving version requirement from network: {version_req}");
    let version = provider.resolve_version(version_req).await?;

    // Don't write back - user specified a version requirement
    Ok((version, false))
}

/// Check if the resolved version is compatible with lower priority sources.
/// Emit warnings if incompatible.
fn check_version_compatibility(
    resolved_version: &str,
    source: VersionSource,
    engines_node: &Option<Str>,
    dev_engines_runtime: &Option<Str>,
) {
    let parsed = match Version::parse(resolved_version) {
        Ok(v) => v,
        Err(_) => return, // Can't check compatibility without a valid version
    };

    // Check engines.node if it's a lower priority source
    if source != VersionSource::EnginesNode {
        if let Some(req) = engines_node {
            check_constraint(&parsed, req, "engines.node", resolved_version, source);
        }
    }

    // Check devEngines.runtime if it's a lower priority source
    if source != VersionSource::DevEnginesRuntime {
        if let Some(req) = dev_engines_runtime {
            check_constraint(&parsed, req, "devEngines.runtime", resolved_version, source);
        }
    }
}

/// Check if a version satisfies a constraint and warn if not.
fn check_constraint(
    version: &Version,
    constraint: &str,
    constraint_source: &str,
    resolved_version: &str,
    source: VersionSource,
) {
    match Range::parse(constraint) {
        Ok(range) => {
            if !range.satisfies(version) {
                println!(
                    "warning: Node.js version {resolved_version} (from {source}) does not satisfy \
                     {constraint_source} constraint '{constraint}'"
                );
            }
        }
        Err(e) => {
            tracing::debug!("Failed to parse {constraint_source} constraint '{constraint}': {e}");
        }
    }
}

/// Normalize and validate a version string as semver (exact version or range).
/// Trims whitespace and returns the normalized version, or None with a warning if invalid.
fn normalize_version(version: &Str, source: &str) -> Option<Str> {
    // Trim leading/trailing whitespace
    let trimmed: Str = version.trim().into();

    if trimmed.is_empty() {
        return None;
    }

    // Try parsing as exact version (strip 'v' prefix for exact version check)
    let without_v = trimmed.strip_prefix('v').unwrap_or(&trimmed);
    if Version::parse(without_v).is_ok() {
        return Some(trimmed);
    }

    // Try parsing as range
    if Range::parse(&trimmed).is_ok() {
        return Some(trimmed);
    }

    // Invalid version
    println!("warning: invalid version '{version}' in {source}, ignoring");
    None
}

/// Read package.json contents.
async fn read_package_json(
    package_json_path: &AbsolutePathBuf,
) -> Result<Option<PackageJson>, Error> {
    if !tokio::fs::try_exists(package_json_path).await.unwrap_or(false) {
        tracing::debug!("package.json not found at {:?}", package_json_path);
        return Ok(None);
    }

    let content = tokio::fs::read_to_string(package_json_path).await?;
    let pkg: PackageJson = serde_json::from_str(&content)?;
    Ok(Some(pkg))
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_js_runtime_type_display() {
        assert_eq!(JsRuntimeType::Node.to_string(), "node");
    }

    #[tokio::test]
    async fn test_download_runtime_for_project_with_dev_engines() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create package.json with devEngines.runtime
        let package_json = r#"{"devEngines":{"runtime":{"name":"node","version":"^20.18.0"}}}"#;
        tokio::fs::write(temp_path.join("package.json"), package_json).await.unwrap();

        let runtime = download_runtime_for_project(&temp_path).await.unwrap();

        assert_eq!(runtime.runtime_type(), JsRuntimeType::Node);
        // Version should be >= 20.18.0 and < 21.0.0
        let version = runtime.version();
        let parsed = node_semver::Version::parse(version).unwrap();
        assert_eq!(parsed.major, 20);
        assert!(parsed.minor >= 18);

        // Verify the binary exists and works
        let binary_path = runtime.get_binary_path();
        assert!(tokio::fs::try_exists(&binary_path).await.unwrap());
    }

    #[tokio::test]
    async fn test_download_runtime_for_project_with_multiple_runtimes() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create package.json with array of runtimes
        let package_json = r#"{
            "devEngines": {
                "runtime": [
                    {"name": "deno", "version": "^2.0.0"},
                    {"name": "node", "version": "^20.18.0"}
                ]
            }
        }"#;
        tokio::fs::write(temp_path.join("package.json"), package_json).await.unwrap();

        let runtime = download_runtime_for_project(&temp_path).await.unwrap();

        // Should use node runtime (deno is not supported yet)
        assert_eq!(runtime.runtime_type(), JsRuntimeType::Node);
        let version = runtime.version();
        let parsed = node_semver::Version::parse(version).unwrap();
        assert_eq!(parsed.major, 20);
    }

    #[tokio::test]
    async fn test_download_runtime_for_project_no_dev_engines() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create package.json without devEngines (minified, will use default 2-space indent)
        let package_json = r#"{"name": "test-project"}"#;
        tokio::fs::write(temp_path.join("package.json"), package_json).await.unwrap();

        let runtime = download_runtime_for_project(&temp_path).await.unwrap();

        // Should download latest Node.js
        assert_eq!(runtime.runtime_type(), JsRuntimeType::Node);

        // Should have a valid version
        let version = runtime.version();
        let parsed = node_semver::Version::parse(version).unwrap();
        assert!(parsed.major >= 20);

        // Should write resolved version to .node-version
        let node_version_content =
            tokio::fs::read_to_string(temp_path.join(".node-version")).await.unwrap();
        assert_eq!(node_version_content, format!("{version}\n"));

        // package.json should remain unchanged
        let pkg_content = tokio::fs::read_to_string(temp_path.join("package.json")).await.unwrap();
        assert_eq!(pkg_content, package_json);
    }

    #[tokio::test]
    async fn test_download_runtime_for_project_writes_back_when_no_version() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create package.json with runtime but no version
        let package_json = r#"{
  "name": "test-project",
  "devEngines": {
    "runtime": {
      "name": "node"
    }
  }
}
"#;
        tokio::fs::write(temp_path.join("package.json"), package_json).await.unwrap();

        let runtime = download_runtime_for_project(&temp_path).await.unwrap();
        let version = runtime.version();

        // Should write resolved version to .node-version
        let node_version_content =
            tokio::fs::read_to_string(temp_path.join(".node-version")).await.unwrap();
        assert_eq!(node_version_content, format!("{version}\n"));

        // package.json should remain unchanged
        let pkg_content = tokio::fs::read_to_string(temp_path.join("package.json")).await.unwrap();
        assert_eq!(pkg_content, package_json);
    }

    #[tokio::test]
    async fn test_download_runtime_for_project_does_not_write_back_when_version_specified() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create package.json with version range
        let package_json = r#"{
  "name": "test-project",
  "devEngines": {
    "runtime": {
      "name": "node",
      "version": "^20.18.0"
    }
  }
}
"#;
        tokio::fs::write(temp_path.join("package.json"), package_json).await.unwrap();

        let runtime = download_runtime_for_project(&temp_path).await.unwrap();
        let version = runtime.version();
        let parsed = node_semver::Version::parse(version).unwrap();
        assert_eq!(parsed.major, 20);

        // Should NOT write .node-version since a version was specified
        assert!(!tokio::fs::try_exists(temp_path.join(".node-version")).await.unwrap());

        // package.json should remain unchanged
        let pkg_content = tokio::fs::read_to_string(temp_path.join("package.json")).await.unwrap();
        assert_eq!(pkg_content, package_json);
    }

    #[tokio::test]
    async fn test_download_runtime_for_project_with_v_prefix_exact_version() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create package.json with exact version including 'v' prefix
        let package_json = r#"{"devEngines":{"runtime":{"name":"node","version":"v20.18.0"}}}"#;
        tokio::fs::write(temp_path.join("package.json"), package_json).await.unwrap();

        let runtime = download_runtime_for_project(&temp_path).await.unwrap();

        assert_eq!(runtime.runtime_type(), JsRuntimeType::Node);
        // Version should be normalized (without 'v' prefix)
        assert_eq!(runtime.version(), "20.18.0");

        // Verify the binary exists and works
        let binary_path = runtime.get_binary_path();
        assert!(tokio::fs::try_exists(&binary_path).await.unwrap());
    }

    #[tokio::test]
    async fn test_download_runtime_for_project_no_package_json() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // No package.json file
        let runtime = download_runtime_for_project(&temp_path).await.unwrap();

        // Should download latest Node.js
        assert_eq!(runtime.runtime_type(), JsRuntimeType::Node);
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
    #[ignore]
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
        let cache_dir = crate::cache::get_cache_dir().unwrap();
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

    // ==========================================
    // Multi-source version reading tests
    // ==========================================

    #[tokio::test]
    async fn test_node_version_file_takes_priority() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create .node-version with exact version
        tokio::fs::write(temp_path.join(".node-version"), "20.18.0\n").await.unwrap();

        // Create package.json with engines.node (should be ignored)
        let package_json = r#"{"engines":{"node":">=22.0.0"}}"#;
        tokio::fs::write(temp_path.join("package.json"), package_json).await.unwrap();

        let runtime = download_runtime_for_project(&temp_path).await.unwrap();
        assert_eq!(runtime.version(), "20.18.0");

        // Should NOT write back since .node-version had exact version
        let node_version_content =
            tokio::fs::read_to_string(temp_path.join(".node-version")).await.unwrap();
        assert_eq!(node_version_content, "20.18.0\n");
    }

    #[tokio::test]
    async fn test_engines_node_takes_priority_over_dev_engines() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create package.json with both engines.node and devEngines.runtime
        let package_json = r#"{
  "engines": {"node": "^20.18.0"},
  "devEngines": {"runtime": {"name": "node", "version": "^22.0.0"}}
}"#;
        tokio::fs::write(temp_path.join("package.json"), package_json).await.unwrap();

        let runtime = download_runtime_for_project(&temp_path).await.unwrap();
        // Should use engines.node (^20.18.0), which will resolve to a 20.x version
        let version = runtime.version();
        let parsed = node_semver::Version::parse(version).unwrap();
        assert_eq!(parsed.major, 20);
    }

    #[tokio::test]
    async fn test_only_engines_node_source() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create package.json with only engines.node
        let package_json = r#"{"engines":{"node":"^20.18.0"}}"#;
        tokio::fs::write(temp_path.join("package.json"), package_json).await.unwrap();

        let runtime = download_runtime_for_project(&temp_path).await.unwrap();
        let version = runtime.version();
        let parsed = node_semver::Version::parse(version).unwrap();
        assert_eq!(parsed.major, 20);

        // Should NOT write .node-version since a version was specified
        assert!(!tokio::fs::try_exists(temp_path.join(".node-version")).await.unwrap());
    }

    #[tokio::test]
    async fn test_node_version_file_partial_version() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create .node-version with partial version (two parts)
        tokio::fs::write(temp_path.join(".node-version"), "20.18\n").await.unwrap();

        let runtime = download_runtime_for_project(&temp_path).await.unwrap();
        let version = runtime.version();
        let parsed = node_semver::Version::parse(version).unwrap();
        // Should resolve to a 20.18.x or higher version in 20.x line
        assert_eq!(parsed.major, 20);
        // Minor version should be at least 18
        assert!(parsed.minor >= 18, "Expected minor >= 18, got {}", parsed.minor);

        // Should NOT write back - .node-version already has a version specified
        let node_version_content =
            tokio::fs::read_to_string(temp_path.join(".node-version")).await.unwrap();
        assert_eq!(node_version_content, "20.18\n");
    }

    #[tokio::test]
    async fn test_node_version_file_single_part_version() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create .node-version with single-part version
        tokio::fs::write(temp_path.join(".node-version"), "20\n").await.unwrap();

        let runtime = download_runtime_for_project(&temp_path).await.unwrap();
        let version = runtime.version();
        let parsed = node_semver::Version::parse(version).unwrap();
        // Should resolve to a 20.x.x version
        assert_eq!(parsed.major, 20);

        // Should NOT write back - .node-version already has a version specified
        let node_version_content =
            tokio::fs::read_to_string(temp_path.join(".node-version")).await.unwrap();
        assert_eq!(node_version_content, "20\n");
    }

    #[test]
    fn test_version_source_display() {
        assert_eq!(VersionSource::NodeVersionFile.to_string(), ".node-version");
        assert_eq!(VersionSource::EnginesNode.to_string(), "engines.node");
        assert_eq!(VersionSource::DevEnginesRuntime.to_string(), "devEngines.runtime");
        assert_eq!(VersionSource::None.to_string(), "none");
    }

    // ==========================================
    // Invalid version validation tests
    // ==========================================

    #[tokio::test]
    async fn test_invalid_node_version_file_is_ignored() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create .node-version with invalid version
        tokio::fs::write(temp_path.join(".node-version"), "invalid\n").await.unwrap();

        // Create package.json without any version
        let package_json = r#"{"name": "test-project"}"#;
        tokio::fs::write(temp_path.join("package.json"), package_json).await.unwrap();

        // Should fall through to fetch latest LTS since .node-version is invalid
        let runtime = download_runtime_for_project(&temp_path).await.unwrap();
        assert_eq!(runtime.runtime_type(), JsRuntimeType::Node);

        // Should have a valid version (latest LTS)
        let version = runtime.version();
        let parsed = node_semver::Version::parse(version).unwrap();
        assert!(parsed.major >= 20);
    }

    #[tokio::test]
    async fn test_invalid_engines_node_is_ignored() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create package.json with invalid engines.node
        let package_json = r#"{"engines":{"node":"invalid"}}"#;
        tokio::fs::write(temp_path.join("package.json"), package_json).await.unwrap();

        // Should fall through to fetch latest LTS since engines.node is invalid
        let runtime = download_runtime_for_project(&temp_path).await.unwrap();
        assert_eq!(runtime.runtime_type(), JsRuntimeType::Node);

        // Should have a valid version (latest LTS)
        let version = runtime.version();
        let parsed = node_semver::Version::parse(version).unwrap();
        assert!(parsed.major >= 20);
    }

    #[tokio::test]
    async fn test_invalid_dev_engines_runtime_is_ignored() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create package.json with invalid devEngines.runtime version
        let package_json = r#"{"devEngines":{"runtime":{"name":"node","version":"invalid"}}}"#;
        tokio::fs::write(temp_path.join("package.json"), package_json).await.unwrap();

        // Should fall through to fetch latest LTS since devEngines.runtime is invalid
        let runtime = download_runtime_for_project(&temp_path).await.unwrap();
        assert_eq!(runtime.runtime_type(), JsRuntimeType::Node);

        // Should have a valid version (latest LTS)
        let version = runtime.version();
        let parsed = node_semver::Version::parse(version).unwrap();
        assert!(parsed.major >= 20);
    }

    #[tokio::test]
    async fn test_invalid_node_version_file_falls_through_to_valid_engines() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create .node-version with invalid version
        tokio::fs::write(temp_path.join(".node-version"), "invalid\n").await.unwrap();

        // Create package.json with valid engines.node
        let package_json = r#"{"engines":{"node":"^20.18.0"}}"#;
        tokio::fs::write(temp_path.join("package.json"), package_json).await.unwrap();

        // Should use engines.node since .node-version is invalid
        let runtime = download_runtime_for_project(&temp_path).await.unwrap();
        let version = runtime.version();
        let parsed = node_semver::Version::parse(version).unwrap();
        assert_eq!(parsed.major, 20);
    }

    #[tokio::test]
    async fn test_invalid_engines_falls_through_to_valid_dev_engines() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create package.json with invalid engines.node but valid devEngines.runtime
        let package_json = r#"{
  "engines": {"node": "invalid"},
  "devEngines": {"runtime": {"name": "node", "version": "^20.18.0"}}
}"#;
        tokio::fs::write(temp_path.join("package.json"), package_json).await.unwrap();

        // Should use devEngines.runtime since engines.node is invalid
        let runtime = download_runtime_for_project(&temp_path).await.unwrap();
        let version = runtime.version();
        let parsed = node_semver::Version::parse(version).unwrap();
        assert_eq!(parsed.major, 20);
    }

    #[test]
    fn test_normalize_version_exact() {
        let version = Str::from("20.18.0");
        assert_eq!(normalize_version(&version, "test"), Some(version.clone()));
    }

    #[test]
    fn test_normalize_version_with_v_prefix() {
        let version = Str::from("v20.18.0");
        assert_eq!(normalize_version(&version, "test"), Some(version.clone()));
    }

    #[test]
    fn test_normalize_version_range() {
        let version = Str::from("^20.18.0");
        assert_eq!(normalize_version(&version, "test"), Some(version.clone()));
    }

    #[test]
    fn test_normalize_version_partial() {
        // Partial versions like "20" or "20.18" should be valid as ranges
        let version = Str::from("20");
        assert_eq!(normalize_version(&version, "test"), Some(version.clone()));

        let version = Str::from("20.18");
        assert_eq!(normalize_version(&version, "test"), Some(version.clone()));
    }

    #[test]
    fn test_normalize_version_invalid() {
        let version = Str::from("invalid");
        assert_eq!(normalize_version(&version, "test"), None);

        let version = Str::from("not-a-version");
        assert_eq!(normalize_version(&version, "test"), None);
    }

    #[test]
    fn test_normalize_version_real_world_ranges() {
        // Test various real-world version range formats
        let valid_ranges = [
            ">=18",
            ">=18 <21",
            "^18.18.0",
            "~20.11.1",
            "18.x",
            "20.*",
            "18 || 20 || >=22",
            ">=16 <=20",
            ">=20.0.0-rc.0",
            "*",
        ];

        for range in valid_ranges {
            let version = Str::from(range);
            assert_eq!(
                normalize_version(&version, "test"),
                Some(version.clone()),
                "Expected '{range}' to be valid"
            );
        }
    }

    #[test]
    fn test_normalize_version_with_negation() {
        // node-semver crate supports negation syntax
        let version = Str::from(">=18 !=19.0.0 <21");
        assert_eq!(
            normalize_version(&version, "test"),
            Some(version.clone()),
            "Expected '>=18 !=19.0.0 <21' to be valid"
        );
    }

    #[test]
    fn test_normalize_version_with_whitespace() {
        // Versions with leading/trailing whitespace are trimmed
        let version = Str::from("   20  ");
        assert_eq!(
            normalize_version(&version, "test"),
            Some(Str::from("20")),
            "Expected '   20  ' to be trimmed to '20'"
        );

        let version = Str::from("  v20.2.0   ");
        assert_eq!(
            normalize_version(&version, "test"),
            Some(Str::from("v20.2.0")),
            "Expected '  v20.2.0   ' to be trimmed to 'v20.2.0'"
        );
    }

    #[test]
    fn test_normalize_version_empty_or_whitespace_only() {
        let version = Str::from("");
        assert_eq!(normalize_version(&version, "test"), None);

        let version = Str::from("   ");
        assert_eq!(normalize_version(&version, "test"), None);
    }
}
