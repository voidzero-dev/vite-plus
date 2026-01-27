//! Node.js runtime provider implementation.

use std::{
    env,
    time::{SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use directories::BaseDirs;
use node_semver::{Range, Version};
use serde::{Deserialize, Serialize};
use vite_path::{AbsolutePathBuf, current_dir};
use vite_str::Str;

use crate::{
    Error, Platform,
    download::download_text,
    platform::Os,
    provider::{ArchiveFormat, DownloadInfo, HashVerification, JsRuntimeProvider},
};

/// Default Node.js distribution base URL
const DEFAULT_NODE_DIST_URL: &str = "https://nodejs.org/dist";

/// Environment variable to override the Node.js distribution URL
const NODE_DIST_MIRROR_ENV: &str = "VITE_NODE_DIST_MIRROR";

/// Default cache TTL in seconds (1 hour)
const DEFAULT_CACHE_TTL_SECS: u64 = 3600;

/// A single entry from the Node.js version index
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct NodeVersionEntry {
    /// Version string (e.g., "v25.5.0")
    pub version: Str,
    /// LTS information
    #[serde(default)]
    pub lts: LtsInfo,
}

/// LTS field can be false or a codename string
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(untagged)]
pub enum LtsInfo {
    /// Not an LTS release
    #[default]
    NotLts,
    /// Boolean false (not LTS)
    Boolean(bool),
    /// LTS codename (e.g., "Jod")
    Codename(Str),
}

/// Cached version index with expiration
#[derive(Deserialize, Serialize, Debug)]
struct VersionIndexCache {
    /// Unix timestamp when cache expires
    expires_at: u64,
    /// ETag from HTTP response (for conditional requests)
    #[serde(default)]
    etag: Option<Str>,
    /// Cached version entries
    versions: Vec<NodeVersionEntry>,
}

/// Node.js runtime provider
#[derive(Debug, Default)]
pub struct NodeProvider;

impl NodeProvider {
    /// Create a new `NodeProvider`
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Get the archive format for a platform
    const fn archive_format(platform: Platform) -> ArchiveFormat {
        match platform.os {
            Os::Windows => ArchiveFormat::Zip,
            Os::Linux | Os::Darwin => ArchiveFormat::TarGz,
        }
    }

    /// Fetch the version index from nodejs.org/dist/index.json with HTTP caching.
    ///
    /// # Errors
    ///
    /// Returns an error if the download fails or the JSON is invalid.
    pub async fn fetch_version_index(&self) -> Result<Vec<NodeVersionEntry>, Error> {
        let cache_dir = get_cache_dir()?;
        let cache_path = cache_dir.join("node/index_cache.json");

        // Try to load from cache
        if let Some(cache) = load_cache(&cache_path).await {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            if now < cache.expires_at {
                tracing::debug!(
                    "Using cached version index (expires in {}s)",
                    cache.expires_at - now
                );
                return Ok(cache.versions);
            }
            tracing::debug!("Version index cache expired, fetching fresh data");
        }

        // Fetch fresh data
        self.fetch_and_cache(&cache_path).await
    }

    /// Fetch the version index and cache it.
    async fn fetch_and_cache(
        &self,
        cache_path: &AbsolutePathBuf,
    ) -> Result<Vec<NodeVersionEntry>, Error> {
        let base_url = get_dist_url();
        let index_url = vite_str::format!("{base_url}/index.json");

        tracing::debug!("Fetching version index from {index_url}");
        let content = download_text(&index_url).await?;

        let versions: Vec<NodeVersionEntry> = serde_json::from_str(&content)?;

        // Save to cache
        let expires_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
            + DEFAULT_CACHE_TTL_SECS;

        let cache = VersionIndexCache { expires_at, etag: None, versions: versions.clone() };

        // Ensure cache directory exists
        if let Some(parent) = cache_path.parent() {
            tokio::fs::create_dir_all(parent).await.ok();
        }

        // Write cache file (ignore errors)
        if let Ok(cache_json) = serde_json::to_string(&cache) {
            tokio::fs::write(cache_path, cache_json).await.ok();
        }

        Ok(versions)
    }

    /// Resolve a version requirement (e.g., "^24.4.0") to an exact version.
    ///
    /// Uses npm-compatible semver range parsing.
    ///
    /// # Errors
    ///
    /// Returns an error if no matching version is found or if the version requirement is invalid.
    pub async fn resolve_version(&self, version_req: &str) -> Result<Str, Error> {
        let range = Range::parse(version_req)?;
        let versions = self.fetch_version_index().await?;

        for entry in versions {
            let version_str = entry.version.strip_prefix('v').unwrap_or(&entry.version);
            if let Ok(version) = Version::parse(version_str) {
                if range.satisfies(&version) {
                    return Ok(version_str.into());
                }
            }
        }

        Err(Error::NoMatchingVersion { version_req: version_req.into() })
    }

    /// Get the latest version (first entry in the index).
    ///
    /// # Errors
    ///
    /// Returns an error if the version index is empty or cannot be fetched.
    pub async fn resolve_latest_version(&self) -> Result<Str, Error> {
        let versions = self.fetch_version_index().await?;

        versions
            .first()
            .map(|entry| entry.version.strip_prefix('v').unwrap_or(&entry.version).into())
            .ok_or_else(|| Error::VersionIndexParseFailed {
                reason: "Version index is empty".into(),
            })
    }
}

/// Load cache from file.
async fn load_cache(cache_path: &AbsolutePathBuf) -> Option<VersionIndexCache> {
    let content = tokio::fs::read_to_string(cache_path).await.ok()?;
    serde_json::from_str(&content).ok()
}

/// Get the cache directory for JavaScript runtimes.
fn get_cache_dir() -> Result<AbsolutePathBuf, Error> {
    let cache_dir = match BaseDirs::new() {
        Some(dirs) => AbsolutePathBuf::new(dirs.cache_dir().to_path_buf()).unwrap(),
        None => current_dir()?.join(".cache"),
    };
    Ok(cache_dir.join("vite/js_runtime"))
}

/// Get the Node.js distribution base URL
///
/// Returns the value of `VITE_NODE_DIST_MIRROR` environment variable if set,
/// otherwise returns the default `https://nodejs.org/dist`.
fn get_dist_url() -> Str {
    env::var(NODE_DIST_MIRROR_ENV)
        .map_or_else(|_| DEFAULT_NODE_DIST_URL.into(), |url| url.trim_end_matches('/').into())
}

#[async_trait]
impl JsRuntimeProvider for NodeProvider {
    fn name(&self) -> &'static str {
        "node"
    }

    fn platform_string(&self, platform: Platform) -> Str {
        let os = match platform.os {
            Os::Linux => "linux",
            Os::Darwin => "darwin",
            Os::Windows => "win",
        };
        let arch = match platform.arch {
            crate::platform::Arch::X64 => "x64",
            crate::platform::Arch::Arm64 => "arm64",
        };
        vite_str::format!("{os}-{arch}")
    }

    fn get_download_info(&self, version: &str, platform: Platform) -> DownloadInfo {
        let base_url = get_dist_url();
        let platform_str = self.platform_string(platform);
        let format = Self::archive_format(platform);
        let ext = format.extension();

        let archive_filename: Str = vite_str::format!("node-v{version}-{platform_str}.{ext}");
        let archive_url = vite_str::format!("{base_url}/v{version}/{archive_filename}");
        let shasums_url = vite_str::format!("{base_url}/v{version}/SHASUMS256.txt");
        let extracted_dir_name = vite_str::format!("node-v{version}-{platform_str}");

        DownloadInfo {
            archive_url,
            archive_filename,
            archive_format: format,
            hash_verification: HashVerification::ShasumsFile { url: shasums_url },
            extracted_dir_name,
        }
    }

    fn binary_relative_path(&self, platform: Platform) -> Str {
        match platform.os {
            Os::Windows => "node.exe".into(),
            Os::Linux | Os::Darwin => "bin/node".into(),
        }
    }

    fn bin_dir_relative_path(&self, platform: Platform) -> Str {
        match platform.os {
            Os::Windows => "".into(),
            Os::Linux | Os::Darwin => "bin".into(),
        }
    }

    fn parse_shasums(&self, shasums_content: &str, filename: &str) -> Result<Str, Error> {
        // Node.js SHASUMS256.txt format: "<hash>  <filename>" (two spaces between)
        for line in shasums_content.lines() {
            let parts: Vec<&str> = line.splitn(2, "  ").collect();
            if parts.len() == 2 {
                let hash = parts[0].trim();
                let file = parts[1].trim();
                if file == filename {
                    return Ok(hash.into());
                }
            }
        }

        Err(Error::HashNotFound { filename: filename.into() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{Arch, Os};

    #[test]
    fn test_platform_string() {
        let provider = NodeProvider::new();

        let cases = [
            (Platform { os: Os::Linux, arch: Arch::X64 }, "linux-x64"),
            (Platform { os: Os::Linux, arch: Arch::Arm64 }, "linux-arm64"),
            (Platform { os: Os::Darwin, arch: Arch::X64 }, "darwin-x64"),
            (Platform { os: Os::Darwin, arch: Arch::Arm64 }, "darwin-arm64"),
            (Platform { os: Os::Windows, arch: Arch::X64 }, "win-x64"),
            (Platform { os: Os::Windows, arch: Arch::Arm64 }, "win-arm64"),
        ];

        for (platform, expected) in cases {
            assert_eq!(provider.platform_string(platform), expected);
        }
    }

    #[test]
    fn test_get_download_info() {
        let provider = NodeProvider::new();
        let platform = Platform { os: Os::Linux, arch: Arch::X64 };

        let info = provider.get_download_info("22.13.1", platform);

        assert_eq!(info.archive_filename, "node-v22.13.1-linux-x64.tar.gz");
        assert_eq!(
            info.archive_url,
            "https://nodejs.org/dist/v22.13.1/node-v22.13.1-linux-x64.tar.gz"
        );
        assert_eq!(info.archive_format, ArchiveFormat::TarGz);
        assert_eq!(info.extracted_dir_name, "node-v22.13.1-linux-x64");

        if let HashVerification::ShasumsFile { url } = &info.hash_verification {
            assert_eq!(url, "https://nodejs.org/dist/v22.13.1/SHASUMS256.txt");
        } else {
            panic!("Expected ShasumsFile verification");
        }
    }

    #[test]
    fn test_get_download_info_windows() {
        let provider = NodeProvider::new();
        let platform = Platform { os: Os::Windows, arch: Arch::X64 };

        let info = provider.get_download_info("22.13.1", platform);

        assert_eq!(info.archive_filename, "node-v22.13.1-win-x64.zip");
        assert_eq!(info.archive_format, ArchiveFormat::Zip);
    }

    #[test]
    fn test_binary_relative_path() {
        let provider = NodeProvider::new();

        assert_eq!(
            provider.binary_relative_path(Platform { os: Os::Linux, arch: Arch::X64 }),
            "bin/node"
        );
        assert_eq!(
            provider.binary_relative_path(Platform { os: Os::Darwin, arch: Arch::Arm64 }),
            "bin/node"
        );
        assert_eq!(
            provider.binary_relative_path(Platform { os: Os::Windows, arch: Arch::X64 }),
            "node.exe"
        );
    }

    #[test]
    fn test_bin_dir_relative_path() {
        let provider = NodeProvider::new();

        assert_eq!(
            provider.bin_dir_relative_path(Platform { os: Os::Linux, arch: Arch::X64 }),
            "bin"
        );
        assert_eq!(
            provider.bin_dir_relative_path(Platform { os: Os::Windows, arch: Arch::X64 }),
            ""
        );
    }

    #[test]
    fn test_parse_shasums() {
        let provider = NodeProvider::new();

        let content = r"abc123def456  node-v22.13.1-linux-x64.tar.gz
789xyz000111  node-v22.13.1-darwin-arm64.tar.gz
fedcba987654  node-v22.13.1-win-x64.zip";

        assert_eq!(
            provider.parse_shasums(content, "node-v22.13.1-linux-x64.tar.gz").unwrap(),
            "abc123def456"
        );
        assert_eq!(
            provider.parse_shasums(content, "node-v22.13.1-darwin-arm64.tar.gz").unwrap(),
            "789xyz000111"
        );
        assert_eq!(
            provider.parse_shasums(content, "node-v22.13.1-win-x64.zip").unwrap(),
            "fedcba987654"
        );

        // Test missing filename
        let result = provider.parse_shasums(content, "nonexistent.tar.gz");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_dist_url_default() {
        // When env var is not set, should return default URL
        unsafe { env::remove_var(NODE_DIST_MIRROR_ENV) };
        assert_eq!(get_dist_url(), "https://nodejs.org/dist");
    }

    #[test]
    fn test_get_dist_url_with_mirror() {
        unsafe { env::set_var(NODE_DIST_MIRROR_ENV, "https://nodejs.org/dist") };
        assert_eq!(get_dist_url(), "https://nodejs.org/dist");
        unsafe { env::remove_var(NODE_DIST_MIRROR_ENV) };
    }

    #[test]
    fn test_get_dist_url_trims_trailing_slash() {
        // Should trim trailing slash from mirror URL
        unsafe { env::set_var(NODE_DIST_MIRROR_ENV, "https://nodejs.org/dist/") };
        assert_eq!(get_dist_url(), "https://nodejs.org/dist");
        unsafe { env::remove_var(NODE_DIST_MIRROR_ENV) };
    }

    #[test]
    fn test_parse_lts_info() {
        // Test parsing different LTS formats
        let json_not_lts = r#"{"version": "v23.0.0", "lts": false}"#;
        let entry: NodeVersionEntry = serde_json::from_str(json_not_lts).unwrap();
        assert!(matches!(entry.lts, LtsInfo::Boolean(false)));

        let json_lts_codename = r#"{"version": "v22.12.0", "lts": "Jod"}"#;
        let entry: NodeVersionEntry = serde_json::from_str(json_lts_codename).unwrap();
        assert!(matches!(entry.lts, LtsInfo::Codename(_)));

        let json_no_lts = r#"{"version": "v23.0.0"}"#;
        let entry: NodeVersionEntry = serde_json::from_str(json_no_lts).unwrap();
        assert!(matches!(entry.lts, LtsInfo::NotLts));
    }

    #[tokio::test]
    async fn test_fetch_version_index() {
        let provider = NodeProvider::new();
        let versions = provider.fetch_version_index().await.unwrap();

        // Should have at least some versions
        assert!(!versions.is_empty());

        // First entry should be the latest version
        let first = &versions[0];
        assert!(first.version.starts_with('v'));

        // Should contain some known versions
        let has_v20 = versions.iter().any(|v| v.version.starts_with("v20."));
        assert!(has_v20, "Should contain Node.js v20.x versions");
    }

    #[tokio::test]
    async fn test_resolve_version() {
        let provider = NodeProvider::new();

        // Test resolving a caret range
        let version = provider.resolve_version("^20.18.0").await.unwrap();
        let parsed = Version::parse(&version).unwrap();
        assert!(parsed.major == 20);
        assert!(parsed.minor >= 18);

        // Test resolving a tilde range
        let version = provider.resolve_version("~20.18.0").await.unwrap();
        let parsed = Version::parse(&version).unwrap();
        assert!(parsed.major == 20);
        assert!(parsed.minor == 18);
    }

    #[tokio::test]
    async fn test_resolve_version_exact() {
        let provider = NodeProvider::new();

        // Test resolving an exact version
        let version = provider.resolve_version("20.18.0").await.unwrap();
        assert_eq!(version, "20.18.0");
    }

    #[tokio::test]
    async fn test_resolve_version_no_match() {
        let provider = NodeProvider::new();

        // Test a version range that doesn't exist
        let result = provider.resolve_version("^999.0.0").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_resolve_latest_version() {
        let provider = NodeProvider::new();

        let version = provider.resolve_latest_version().await.unwrap();

        // Should be a valid semver without 'v' prefix
        assert!(!version.starts_with('v'));
        let parsed = Version::parse(&version).unwrap();
        // Latest version should be fairly recent
        assert!(parsed.major >= 20);
    }
}
