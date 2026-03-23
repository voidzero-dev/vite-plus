//! Node.js runtime provider implementation.

use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use node_semver::{Range, Version};
use serde::{Deserialize, Serialize};
use vite_path::{AbsolutePath, AbsolutePathBuf};
use vite_str::Str;

use crate::{
    Error, Platform,
    download::fetch_with_cache_headers,
    platform::Os,
    provider::{ArchiveFormat, DownloadInfo, HashVerification, JsRuntimeProvider},
};

/// Default Node.js distribution base URL
#[cfg(not(target_env = "musl"))]
const DEFAULT_NODE_DIST_URL: &str = "https://nodejs.org/dist";

/// Unofficial builds URL for musl (official nodejs.org only provides glibc binaries)
#[cfg(target_env = "musl")]
const DEFAULT_NODE_DIST_URL: &str = "https://unofficial-builds.nodejs.org/download/release";

/// Environment variable to override the Node.js distribution URL

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

impl NodeVersionEntry {
    /// Check if this version is an LTS release.
    #[must_use]
    pub fn is_lts(&self) -> bool {
        matches!(self.lts, LtsInfo::Codename(_))
    }
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

    /// Check if a version string is an exact version (not a range).
    ///
    /// Returns `true` for exact versions like "20.18.0", "22.13.1".
    /// Returns `false` for ranges like "^20.18.0", "~20.18.0", ">=20 <22", "20.x".
    #[must_use]
    pub fn is_exact_version(version_str: &str) -> bool {
        Version::parse(version_str).is_ok()
    }

    /// Find a locally cached version that satisfies the version requirement.
    ///
    /// This checks the local cache directory for installed Node.js versions
    /// and returns a version that satisfies the semver range. Prefers LTS
    /// versions over non-LTS versions.
    ///
    /// # Arguments
    /// * `version_req` - A semver range requirement (e.g., "^20.18.0")
    /// * `cache_dir` - The cache directory path (e.g., `~/.cache/vite-plus/js_runtime`)
    ///
    /// # Returns
    /// The highest LTS cached version that satisfies the requirement, or the
    /// highest non-LTS version if no LTS version matches, or `None` if no
    /// cached version matches.
    ///
    /// # Errors
    /// Returns an error if the version requirement is invalid.
    pub async fn find_cached_version(
        &self,
        version_req: &str,
        cache_dir: &AbsolutePath,
    ) -> Result<Option<Str>, Error> {
        let node_cache = cache_dir.join("node");

        // List directories in cache
        let mut entries = match tokio::fs::read_dir(&node_cache).await {
            Ok(entries) => entries,
            Err(_) => return Ok(None), // Cache dir doesn't exist
        };

        let range = Range::parse(version_req)?;
        let mut matching_versions: Vec<Version> = Vec::new();
        let platform = Platform::current();

        while let Some(entry) = entries.next_entry().await? {
            let name = entry.file_name().to_string_lossy().to_string();
            // Skip non-version entries (index_cache.json, .lock files)
            if let Ok(version) = Version::parse(&name) {
                // Check if binary exists (valid installation)
                let binary_path = node_cache.join(&name).join(self.binary_relative_path(platform));
                if tokio::fs::try_exists(&binary_path).await.unwrap_or(false) {
                    if range.satisfies(&version) {
                        matching_versions.push(version);
                    }
                }
            }
        }

        if matching_versions.is_empty() {
            return Ok(None);
        }

        // Fetch version index to check LTS status
        let version_index = self.fetch_version_index().await?;

        // Build a set of LTS versions for fast lookup
        let lts_versions: std::collections::HashSet<String> = version_index
            .iter()
            .filter(|e| e.is_lts())
            .map(|e| e.version.strip_prefix('v').unwrap_or(&e.version).to_string())
            .collect();

        // Prefer LTS: find highest LTS cached version first
        let lts_max =
            matching_versions.iter().filter(|v| lts_versions.contains(&v.to_string())).max();

        if let Some(version) = lts_max {
            return Ok(Some(version.to_string().into()));
        }

        // Fallback to highest non-LTS
        Ok(matching_versions.into_iter().max().map(|v| v.to_string().into()))
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
    /// Uses ETag-based conditional requests to minimize bandwidth when cache expires.
    /// If a network error occurs and a local cache exists (even if expired), returns
    /// the cached version with a warning log instead of failing.
    ///
    /// # Errors
    ///
    /// Returns an error only if the download fails and no local cache exists.
    pub async fn fetch_version_index(&self) -> Result<Vec<NodeVersionEntry>, Error> {
        let cache_dir = crate::cache::get_cache_dir()?;
        let cache_path = cache_dir.join("node/index_cache.json");

        // Try to load from cache
        let Some(cache) = load_cache(&cache_path).await else {
            // No cache - must fetch
            return self.fetch_and_cache(&cache_path).await;
        };

        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        // If cache is still fresh, use it
        if now < cache.expires_at {
            tracing::debug!("Using cached version index (expires in {}s)", cache.expires_at - now);
            return Ok(cache.versions);
        }

        // Cache expired - try conditional request with ETag if available
        if let Some(ref etag) = cache.etag {
            tracing::debug!("Cache expired, trying conditional request with ETag");
            match self.fetch_with_etag(etag, &cache, &cache_path).await {
                Ok(versions) => return Ok(versions),
                Err(e) => {
                    // Network error with ETag request - return cached version
                    tracing::warn!("Conditional request failed: {e}, using expired cache");
                    return Ok(cache.versions);
                }
            }
        }

        // No ETag - try full fetch, fallback to cache
        tracing::debug!("Cache expired, no ETag available for conditional request");
        match self.fetch_and_cache(&cache_path).await {
            Ok(versions) => Ok(versions),
            Err(e) => {
                tracing::warn!("Failed to fetch version index: {e}, using expired cache");
                Ok(cache.versions)
            }
        }
    }

    /// Try conditional fetch with ETag, returns cached versions if 304
    async fn fetch_with_etag(
        &self,
        etag: &str,
        cache: &VersionIndexCache,
        cache_path: &AbsolutePathBuf,
    ) -> Result<Vec<NodeVersionEntry>, Error> {
        let base_url = get_dist_url();
        let index_url = vite_str::format!("{base_url}/index.json");

        let response = fetch_with_cache_headers(&index_url, Some(etag)).await?;

        if response.not_modified {
            // Server confirmed data hasn't changed, refresh TTL
            tracing::debug!("Server returned 304 Not Modified, refreshing cache TTL");
            let new_cache = VersionIndexCache {
                expires_at: calculate_expires_at(response.max_age),
                etag: cache.etag.clone(),
                versions: cache.versions.clone(),
            };
            save_cache(cache_path, &new_cache).await;
            return Ok(cache.versions.clone());
        }

        // Got new data
        let body = response.body.ok_or_else(|| Error::VersionIndexParseFailed {
            reason: "Empty response body".into(),
        })?;
        let versions: Vec<NodeVersionEntry> = serde_json::from_str(&body)?;

        let new_cache = VersionIndexCache {
            expires_at: calculate_expires_at(response.max_age),
            etag: response.etag,
            versions: versions.clone(),
        };
        save_cache(cache_path, &new_cache).await;

        Ok(versions)
    }

    /// Fetch the version index and cache it.
    async fn fetch_and_cache(
        &self,
        cache_path: &AbsolutePathBuf,
    ) -> Result<Vec<NodeVersionEntry>, Error> {
        let base_url = get_dist_url();
        let index_url = vite_str::format!("{base_url}/index.json");

        tracing::debug!("Fetching version index from {index_url}");
        let response = fetch_with_cache_headers(&index_url, None).await?;

        let body = response.body.ok_or_else(|| Error::VersionIndexParseFailed {
            reason: "Empty response body".into(),
        })?;
        let versions: Vec<NodeVersionEntry> = serde_json::from_str(&body)?;

        let cache = VersionIndexCache {
            expires_at: calculate_expires_at(response.max_age),
            etag: response.etag,
            versions: versions.clone(),
        };
        save_cache(cache_path, &cache).await;

        Ok(versions)
    }

    /// Resolve a version requirement (e.g., "^24.4.0") to an exact version.
    ///
    /// Returns the highest version that satisfies the semver range.
    /// Uses npm-compatible semver range parsing.
    ///
    /// # Errors
    ///
    /// Returns an error if no matching version is found or if the version requirement is invalid.
    pub async fn resolve_version(&self, version_req: &str) -> Result<Str, Error> {
        let versions = self.fetch_version_index().await?;
        resolve_version_from_list(version_req, &versions)
    }

    /// Get the latest LTS version with the highest version number.
    ///
    /// # Errors
    ///
    /// Returns an error if no LTS version is found or the version index cannot be fetched.
    pub async fn resolve_latest_version(&self) -> Result<Str, Error> {
        let versions = self.fetch_version_index().await?;
        find_latest_lts_version(&versions)
    }

    /// Check if a version string is an LTS alias (e.g., `lts/*`, `lts/iron`, `lts/-1`).
    ///
    /// Returns `true` for LTS alias formats:
    /// - `lts/*` - Latest LTS version
    /// - `lts/<codename>` - Specific LTS line (e.g., `lts/iron`, `lts/jod`)
    /// - `lts/-n` - Nth-highest LTS line (e.g., `lts/-1` for second highest)
    #[must_use]
    pub fn is_lts_alias(version: &str) -> bool {
        version.starts_with("lts/")
    }

    /// Check if a version string is a "latest" alias.
    ///
    /// Returns `true` for:
    /// - `latest` - The absolute latest Node.js version (including non-LTS)
    #[must_use]
    pub fn is_latest_alias(version: &str) -> bool {
        version.eq_ignore_ascii_case("latest")
    }

    /// Check if a version string is any kind of alias (lts/* or latest).
    #[must_use]
    pub fn is_version_alias(version: &str) -> bool {
        Self::is_lts_alias(version) || Self::is_latest_alias(version)
    }

    /// Resolve an LTS alias to an exact version.
    ///
    /// # Supported Formats
    ///
    /// - `lts/*` - Returns the latest LTS version
    /// - `lts/<codename>` - Returns the highest version for that LTS line (e.g., `lts/iron` → 20.x)
    /// - `lts/-n` - Returns the nth-highest LTS line (e.g., `lts/-1` → second highest)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The alias format is invalid
    /// - The codename is not recognized
    /// - The offset is too large (not enough LTS lines)
    pub async fn resolve_lts_alias(&self, alias: &str) -> Result<Str, Error> {
        let suffix = alias
            .strip_prefix("lts/")
            .ok_or_else(|| Error::InvalidLtsAlias { alias: alias.into() })?;

        // lts/* - latest LTS
        if suffix == "*" {
            return self.resolve_latest_version().await;
        }

        // lts/-n - nth-highest LTS (e.g., lts/-1 = second highest)
        if suffix.starts_with('-') {
            if let Ok(n) = suffix.parse::<i32>() {
                if n < 0 {
                    return self.resolve_lts_by_offset(n).await;
                }
            }
        }

        // lts/<codename> - specific LTS line
        self.resolve_lts_by_codename(suffix).await
    }

    /// Resolve LTS by codename (e.g., "iron" → 20.x, "jod" → 22.x).
    async fn resolve_lts_by_codename(&self, codename: &str) -> Result<Str, Error> {
        let versions = self.fetch_version_index().await?;
        let target = codename.to_lowercase();

        // Find all versions matching the codename
        let matching: Vec<_> = versions
            .iter()
            .filter(|v| matches!(&v.lts, LtsInfo::Codename(name) if name.to_lowercase() == target))
            .collect();

        if matching.is_empty() {
            return Err(Error::UnknownLtsCodename { codename: codename.into() });
        }

        // Find the highest matching version
        let highest = matching
            .into_iter()
            .filter_map(|entry| {
                let version_str = entry.version.strip_prefix('v').unwrap_or(&entry.version);
                Version::parse(version_str).ok().map(|v| (v, version_str))
            })
            .max_by(|(a, _), (b, _)| a.cmp(b));

        highest
            .map(|(_, version_str)| version_str.into())
            .ok_or_else(|| Error::UnknownLtsCodename { codename: codename.into() })
    }

    /// Resolve LTS by offset (e.g., -1 = second highest LTS line).
    ///
    /// The offset is negative: lts/-1 means "one below the latest LTS line".
    async fn resolve_lts_by_offset(&self, offset: i32) -> Result<Str, Error> {
        let versions = self.fetch_version_index().await?;

        // Get unique LTS codenames ordered by highest version in each line
        let mut lts_lines: Vec<(String, u64)> = Vec::new();

        for entry in &versions {
            if let LtsInfo::Codename(name) = &entry.lts {
                let version_str = entry.version.strip_prefix('v').unwrap_or(&entry.version);
                if let Ok(ver) = Version::parse(version_str) {
                    let key = name.to_lowercase();
                    // Only add if we haven't seen this codename yet (keeping highest version)
                    if !lts_lines.iter().any(|(n, _)| n == &key) {
                        lts_lines.push((key, ver.major));
                    }
                }
            }
        }

        // Sort by major version descending (highest first)
        lts_lines.sort_by(|a, b| b.1.cmp(&a.1));

        // offset is negative, so lts/-1 = index 1 (second highest)
        let index = (-offset) as usize;

        let (codename, _) = lts_lines
            .get(index)
            .ok_or_else(|| Error::InvalidLtsOffset { offset, available: lts_lines.len() })?;

        self.resolve_lts_by_codename(codename).await
    }
}

/// Find the LTS version with the highest version number from a list of versions.
///
/// # Errors
///
/// Returns an error if no LTS version is found in the list.
fn find_latest_lts_version(versions: &[NodeVersionEntry]) -> Result<Str, Error> {
    let latest_lts = versions
        .iter()
        .filter(|entry| entry.is_lts())
        .filter_map(|entry| {
            let version_str = entry.version.strip_prefix('v').unwrap_or(&entry.version);
            Version::parse(version_str).ok().map(|v| (v, version_str))
        })
        .max_by(|(a, _), (b, _)| a.cmp(b));

    latest_lts.map(|(_, version_str)| version_str.into()).ok_or_else(|| {
        Error::VersionIndexParseFailed { reason: "No LTS version found in version index".into() }
    })
}

/// Resolve a version requirement to a matching version from a list.
///
/// Prefers LTS versions over non-LTS versions. Returns the highest LTS version
/// that satisfies the range, or falls back to the highest non-LTS version if
/// no LTS version matches.
///
/// # Errors
///
/// Returns an error if no matching version is found or if the version requirement is invalid.
fn resolve_version_from_list(
    version_req: &str,
    versions: &[NodeVersionEntry],
) -> Result<Str, Error> {
    let range = Range::parse(version_req)?;

    // Collect all matching versions with their LTS status
    let matching_versions: Vec<(Version, &str, bool)> = versions
        .iter()
        .filter_map(|entry| {
            let version_str = entry.version.strip_prefix('v').unwrap_or(&entry.version);
            Version::parse(version_str)
                .ok()
                .filter(|v| range.satisfies(v))
                .map(|v| (v, version_str, entry.is_lts()))
        })
        .collect();

    // Prefer LTS versions: find highest LTS first
    let lts_max = matching_versions
        .iter()
        .filter(|(_, _, is_lts)| *is_lts)
        .max_by(|(a, _, _), (b, _, _)| a.cmp(b));

    if let Some((_, version_str, _)) = lts_max {
        return Ok((*version_str).into());
    }

    // Fallback to highest non-LTS version
    matching_versions
        .into_iter()
        .max_by(|(a, _, _), (b, _, _)| a.cmp(b))
        .map(|(_, version_str, _)| version_str.into())
        .ok_or_else(|| Error::NoMatchingVersion { version_req: version_req.into() })
}

/// Load cache from file.
async fn load_cache(cache_path: &AbsolutePathBuf) -> Option<VersionIndexCache> {
    let content = tokio::fs::read_to_string(cache_path).await.ok()?;
    serde_json::from_str(&content).ok()
}

/// Save cache to file.
async fn save_cache(cache_path: &AbsolutePathBuf, cache: &VersionIndexCache) {
    // Ensure cache directory exists
    if let Some(parent) = cache_path.parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }

    // Write cache file (ignore errors)
    if let Ok(cache_json) = serde_json::to_string(cache) {
        tokio::fs::write(cache_path, cache_json).await.ok();
    }
}

/// Calculate expiration timestamp from max_age or default TTL.
fn calculate_expires_at(max_age: Option<u64>) -> u64 {
    let ttl = max_age.unwrap_or(DEFAULT_CACHE_TTL_SECS);
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + ttl
}

/// Get the Node.js distribution base URL
///
/// Returns the value of `VITE_NODE_DIST_MIRROR` environment variable if set,
/// otherwise returns the default `https://nodejs.org/dist`.
fn get_dist_url() -> Str {
    vite_shared::EnvConfig::get().node_dist_mirror.map_or_else(
        || DEFAULT_NODE_DIST_URL.into(),
        |url| Str::from(url.trim_end_matches('/').to_string()),
    )
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
        // On musl targets, append "-musl" to match unofficial-builds filename pattern
        // e.g. "linux-x64-musl" instead of "linux-x64"
        #[cfg(target_env = "musl")]
        if platform.os == Os::Linux {
            return vite_str::format!("{os}-{arch}-musl");
        }
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
        vite_shared::EnvConfig::test_scope(vite_shared::EnvConfig::for_test(), || {
            assert_eq!(get_dist_url(), "https://nodejs.org/dist");
        });
    }

    #[test]
    fn test_get_dist_url_with_mirror() {
        vite_shared::EnvConfig::test_scope(
            vite_shared::EnvConfig {
                node_dist_mirror: Some("https://nodejs.org/dist".into()),
                ..vite_shared::EnvConfig::for_test()
            },
            || {
                assert_eq!(get_dist_url(), "https://nodejs.org/dist");
            },
        );
    }

    #[test]
    fn test_get_dist_url_trims_trailing_slash() {
        vite_shared::EnvConfig::test_scope(
            vite_shared::EnvConfig {
                node_dist_mirror: Some("https://nodejs.org/dist/".into()),
                ..vite_shared::EnvConfig::for_test()
            },
            || {
                assert_eq!(get_dist_url(), "https://nodejs.org/dist");
            },
        );
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

    #[test]
    fn test_resolve_version_from_list_caret() {
        use super::resolve_version_from_list;

        // Mock version data in random order
        let versions = vec![
            NodeVersionEntry { version: "v20.17.0".into(), lts: LtsInfo::Codename("Iron".into()) },
            NodeVersionEntry { version: "v20.19.0".into(), lts: LtsInfo::Codename("Iron".into()) },
            NodeVersionEntry { version: "v20.18.0".into(), lts: LtsInfo::Codename("Iron".into()) },
            NodeVersionEntry { version: "v21.0.0".into(), lts: LtsInfo::Boolean(false) },
            NodeVersionEntry { version: "v20.20.0".into(), lts: LtsInfo::Codename("Iron".into()) },
        ];

        // ^20.18.0 should match highest 20.x.x >= 20.18.0
        let result = resolve_version_from_list("^20.18.0", &versions).unwrap();
        assert_eq!(result, "20.20.0");
    }

    #[test]
    fn test_resolve_version_from_list_tilde() {
        use super::resolve_version_from_list;

        let versions = vec![
            NodeVersionEntry { version: "v20.18.0".into(), lts: LtsInfo::Codename("Iron".into()) },
            NodeVersionEntry { version: "v20.18.3".into(), lts: LtsInfo::Codename("Iron".into()) },
            NodeVersionEntry { version: "v20.19.0".into(), lts: LtsInfo::Codename("Iron".into()) },
            NodeVersionEntry { version: "v20.18.1".into(), lts: LtsInfo::Codename("Iron".into()) },
            NodeVersionEntry { version: "v20.18.5".into(), lts: LtsInfo::Codename("Iron".into()) },
        ];

        // ~20.18.0 should match highest 20.18.x
        let result = resolve_version_from_list("~20.18.0", &versions).unwrap();
        assert_eq!(result, "20.18.5");
    }

    #[test]
    fn test_resolve_version_from_list_exact() {
        use super::resolve_version_from_list;

        let versions = vec![
            NodeVersionEntry { version: "v20.17.0".into(), lts: LtsInfo::Codename("Iron".into()) },
            NodeVersionEntry { version: "v20.18.0".into(), lts: LtsInfo::Codename("Iron".into()) },
            NodeVersionEntry { version: "v20.19.0".into(), lts: LtsInfo::Codename("Iron".into()) },
        ];

        // Exact version should return that specific version
        let result = resolve_version_from_list("20.18.0", &versions).unwrap();
        assert_eq!(result, "20.18.0");
    }

    #[test]
    fn test_resolve_version_from_list_range() {
        use super::resolve_version_from_list;

        let versions = vec![
            NodeVersionEntry {
                version: "v18.20.0".into(),
                lts: LtsInfo::Codename("Hydrogen".into()),
            },
            NodeVersionEntry { version: "v20.15.0".into(), lts: LtsInfo::Codename("Iron".into()) },
            NodeVersionEntry { version: "v22.5.0".into(), lts: LtsInfo::Codename("Jod".into()) },
            NodeVersionEntry { version: "v20.18.0".into(), lts: LtsInfo::Codename("Iron".into()) },
            NodeVersionEntry { version: "v22.10.0".into(), lts: LtsInfo::Codename("Jod".into()) },
        ];

        // >=20.0.0 <22.0.0 should match highest in range (20.18.0)
        let result = resolve_version_from_list(">=20.0.0 <22.0.0", &versions).unwrap();
        assert_eq!(result, "20.18.0");
    }

    #[test]
    fn test_resolve_version_from_list_no_match() {
        use super::resolve_version_from_list;

        let versions = vec![
            NodeVersionEntry { version: "v20.18.0".into(), lts: LtsInfo::Codename("Iron".into()) },
            NodeVersionEntry { version: "v22.5.0".into(), lts: LtsInfo::Codename("Jod".into()) },
        ];

        // Version that doesn't exist
        let result = resolve_version_from_list("^999.0.0", &versions);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_version_from_list_empty() {
        use super::resolve_version_from_list;

        let versions: Vec<NodeVersionEntry> = vec![];
        let result = resolve_version_from_list("^20.0.0", &versions);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_version_from_list_invalid_range() {
        use super::resolve_version_from_list;

        let versions = vec![NodeVersionEntry {
            version: "v20.18.0".into(),
            lts: LtsInfo::Codename("Iron".into()),
        }];

        // Invalid semver range
        let result = resolve_version_from_list("invalid-range", &versions);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_version_from_list_unordered_finds_max() {
        use super::resolve_version_from_list;

        // Versions in completely random order - the key test case
        let versions = vec![
            NodeVersionEntry { version: "v20.15.0".into(), lts: LtsInfo::Codename("Iron".into()) },
            NodeVersionEntry { version: "v20.20.0".into(), lts: LtsInfo::Codename("Iron".into()) },
            NodeVersionEntry { version: "v20.10.0".into(), lts: LtsInfo::Codename("Iron".into()) },
            NodeVersionEntry { version: "v20.18.0".into(), lts: LtsInfo::Codename("Iron".into()) },
            NodeVersionEntry { version: "v20.12.0".into(), lts: LtsInfo::Codename("Iron".into()) },
        ];

        // Should find the maximum (20.20.0), not the first (20.15.0)
        let result = resolve_version_from_list("^20.0.0", &versions).unwrap();
        assert_eq!(result, "20.20.0");
    }

    #[test]
    fn test_find_latest_lts_version() {
        use super::find_latest_lts_version;

        // Mock version data simulating Node.js index.json structure
        // Note: The index is typically sorted by version descending, but our logic
        // should find the highest LTS version regardless of order
        let versions = vec![
            // Latest non-LTS (Current)
            NodeVersionEntry { version: "v23.5.0".into(), lts: LtsInfo::Boolean(false) },
            NodeVersionEntry { version: "v23.4.0".into(), lts: LtsInfo::Boolean(false) },
            // Latest LTS line (Jod) - v22.x
            NodeVersionEntry { version: "v22.13.0".into(), lts: LtsInfo::Codename("Jod".into()) },
            NodeVersionEntry { version: "v22.12.0".into(), lts: LtsInfo::Codename("Jod".into()) },
            // Older LTS line (Iron) - v20.x
            NodeVersionEntry { version: "v20.18.0".into(), lts: LtsInfo::Codename("Iron".into()) },
            NodeVersionEntry { version: "v20.17.0".into(), lts: LtsInfo::Codename("Iron".into()) },
            // Even older LTS
            NodeVersionEntry {
                version: "v18.20.0".into(),
                lts: LtsInfo::Codename("Hydrogen".into()),
            },
        ];

        let result = find_latest_lts_version(&versions).unwrap();

        // Should return v22.13.0 - the highest version that is LTS
        assert_eq!(result, "22.13.0");
    }

    #[test]
    fn test_find_latest_lts_version_unordered() {
        use super::find_latest_lts_version;

        // Test with versions in random order to ensure we find max, not first
        let versions = vec![
            NodeVersionEntry { version: "v20.18.0".into(), lts: LtsInfo::Codename("Iron".into()) },
            NodeVersionEntry { version: "v23.5.0".into(), lts: LtsInfo::Boolean(false) },
            NodeVersionEntry { version: "v22.12.0".into(), lts: LtsInfo::Codename("Jod".into()) },
            NodeVersionEntry {
                version: "v18.20.0".into(),
                lts: LtsInfo::Codename("Hydrogen".into()),
            },
            NodeVersionEntry { version: "v22.13.0".into(), lts: LtsInfo::Codename("Jod".into()) },
        ];

        let result = find_latest_lts_version(&versions).unwrap();

        // Should still return v22.13.0 - the highest LTS version
        assert_eq!(result, "22.13.0");
    }

    #[test]
    fn test_find_latest_lts_version_no_lts() {
        use super::find_latest_lts_version;

        // Test with no LTS versions
        let versions = vec![
            NodeVersionEntry { version: "v23.5.0".into(), lts: LtsInfo::Boolean(false) },
            NodeVersionEntry { version: "v23.4.0".into(), lts: LtsInfo::Boolean(false) },
            NodeVersionEntry { version: "v23.3.0".into(), lts: LtsInfo::NotLts },
        ];

        let result = find_latest_lts_version(&versions);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_latest_lts_version_empty() {
        use super::find_latest_lts_version;

        let versions: Vec<NodeVersionEntry> = vec![];
        let result = find_latest_lts_version(&versions);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_lts() {
        let lts_entry: NodeVersionEntry =
            serde_json::from_str(r#"{"version": "v22.12.0", "lts": "Jod"}"#).unwrap();
        assert!(lts_entry.is_lts());

        let non_lts_entry: NodeVersionEntry =
            serde_json::from_str(r#"{"version": "v23.0.0", "lts": false}"#).unwrap();
        assert!(!non_lts_entry.is_lts());

        let no_lts_field: NodeVersionEntry =
            serde_json::from_str(r#"{"version": "v23.0.0"}"#).unwrap();
        assert!(!no_lts_field.is_lts());
    }

    #[test]
    fn test_is_exact_version() {
        // Exact versions should return true
        assert!(NodeProvider::is_exact_version("20.18.0"));
        assert!(NodeProvider::is_exact_version("22.13.1"));
        assert!(NodeProvider::is_exact_version("18.20.5"));
        assert!(NodeProvider::is_exact_version("0.0.1"));
        assert!(NodeProvider::is_exact_version("v20.18.0")); // With 'v' prefix is also exact

        // Ranges and partial versions should return false
        assert!(!NodeProvider::is_exact_version("^20.18.0"));
        assert!(!NodeProvider::is_exact_version("~20.18.0"));
        assert!(!NodeProvider::is_exact_version(">=20.0.0"));
        assert!(!NodeProvider::is_exact_version(">=20 <22"));
        assert!(!NodeProvider::is_exact_version("20.x"));
        assert!(!NodeProvider::is_exact_version("20.*"));
        assert!(!NodeProvider::is_exact_version(">20.18.0"));
        assert!(!NodeProvider::is_exact_version("<22.0.0"));
        assert!(!NodeProvider::is_exact_version("20")); // Major only
        assert!(!NodeProvider::is_exact_version("20.18")); // Major.minor only

        // Invalid versions should return false
        assert!(!NodeProvider::is_exact_version("invalid"));
        assert!(!NodeProvider::is_exact_version(""));
    }

    #[tokio::test]
    async fn test_find_cached_version() {
        use tempfile::TempDir;
        use vite_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let cache_dir = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let provider = NodeProvider::new();

        // Initially, no cache exists
        let result = provider.find_cached_version("^20.18.0", &cache_dir).await.unwrap();
        assert!(result.is_none());

        // Create mock cached versions
        let node_cache = cache_dir.join("node");
        tokio::fs::create_dir_all(&node_cache).await.unwrap();

        // Create version directories with mock binary
        let platform = Platform::current();
        let binary_path = provider.binary_relative_path(platform);

        for version in ["20.17.0", "20.18.0", "20.19.0", "21.0.0"] {
            let version_dir = node_cache.join(version);
            let binary_full_path = version_dir.join(&binary_path);
            tokio::fs::create_dir_all(binary_full_path.parent().unwrap()).await.unwrap();
            tokio::fs::write(&binary_full_path, "mock binary").await.unwrap();
        }

        // Create incomplete installation (no binary)
        let incomplete_dir = node_cache.join("20.20.0");
        tokio::fs::create_dir_all(&incomplete_dir).await.unwrap();

        // Test: ^20.18.0 should find highest matching version (20.19.0)
        let result = provider.find_cached_version("^20.18.0", &cache_dir).await.unwrap();
        assert_eq!(result, Some("20.19.0".into()));

        // Test: ~20.18.0 should find highest 20.18.x (only 20.18.0)
        let result = provider.find_cached_version("~20.18.0", &cache_dir).await.unwrap();
        assert_eq!(result, Some("20.18.0".into()));

        // Test: ^21.0.0 should find 21.0.0
        let result = provider.find_cached_version("^21.0.0", &cache_dir).await.unwrap();
        assert_eq!(result, Some("21.0.0".into()));

        // Test: ^22.0.0 should find nothing
        let result = provider.find_cached_version("^22.0.0", &cache_dir).await.unwrap();
        assert!(result.is_none());

        // Test: ^20.20.0 should find nothing (20.20.0 exists but no binary)
        let result = provider.find_cached_version("^20.20.0", &cache_dir).await.unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_version_from_list_prefers_lts() {
        use super::resolve_version_from_list;

        let versions = vec![
            NodeVersionEntry { version: "v25.5.0".into(), lts: LtsInfo::NotLts },
            NodeVersionEntry { version: "v24.5.0".into(), lts: LtsInfo::Codename("Jod".into()) },
            NodeVersionEntry { version: "v22.15.0".into(), lts: LtsInfo::Codename("Jod".into()) },
            NodeVersionEntry { version: "v20.19.0".into(), lts: LtsInfo::Codename("Iron".into()) },
        ];

        // Should prefer highest LTS (v24.5.0) over non-LTS (v25.5.0)
        let result = resolve_version_from_list(">=20.0.0", &versions).unwrap();
        assert_eq!(result, "24.5.0");
    }

    #[test]
    fn test_resolve_version_from_list_falls_back_to_non_lts() {
        use super::resolve_version_from_list;

        let versions = vec![
            NodeVersionEntry { version: "v25.5.0".into(), lts: LtsInfo::NotLts },
            NodeVersionEntry { version: "v25.4.0".into(), lts: LtsInfo::NotLts },
        ];

        // No LTS matches, should return highest non-LTS
        let result = resolve_version_from_list(">24.9999.0", &versions).unwrap();
        assert_eq!(result, "25.5.0");
    }

    #[test]
    fn test_resolve_version_from_list_complex_range_prefers_lts() {
        use super::resolve_version_from_list;

        let versions = vec![
            NodeVersionEntry { version: "v25.5.0".into(), lts: LtsInfo::NotLts },
            NodeVersionEntry { version: "v24.5.0".into(), lts: LtsInfo::Codename("Jod".into()) },
            NodeVersionEntry { version: "v22.15.0".into(), lts: LtsInfo::Codename("Jod".into()) },
            NodeVersionEntry { version: "v20.19.0".into(), lts: LtsInfo::Codename("Iron".into()) },
        ];

        // ^20.19.0 || >=22.12.0 should prefer v24.5.0 (highest LTS) over v25.5.0
        let result = resolve_version_from_list("^20.19.0 || >=22.12.0", &versions).unwrap();
        assert_eq!(result, "24.5.0");
    }

    #[test]
    fn test_resolve_version_from_list_only_matches_in_range_lts() {
        use super::resolve_version_from_list;

        let versions = vec![
            NodeVersionEntry { version: "v25.5.0".into(), lts: LtsInfo::NotLts },
            NodeVersionEntry { version: "v24.5.0".into(), lts: LtsInfo::Codename("Jod".into()) },
            NodeVersionEntry { version: "v20.19.0".into(), lts: LtsInfo::Codename("Iron".into()) },
        ];

        // ^20.18.0 should return 20.19.0 (the only LTS in range)
        let result = resolve_version_from_list("^20.18.0", &versions).unwrap();
        assert_eq!(result, "20.19.0");
    }

    // ========================================================================
    // LTS Alias Tests
    // ========================================================================

    #[test]
    fn test_is_lts_alias() {
        // Valid LTS aliases
        assert!(NodeProvider::is_lts_alias("lts/*"));
        assert!(NodeProvider::is_lts_alias("lts/iron"));
        assert!(NodeProvider::is_lts_alias("lts/jod"));
        assert!(NodeProvider::is_lts_alias("lts/Iron")); // Case-insensitive for codename
        assert!(NodeProvider::is_lts_alias("lts/Jod"));
        assert!(NodeProvider::is_lts_alias("lts/hydrogen"));
        assert!(NodeProvider::is_lts_alias("lts/-1")); // Offset format
        assert!(NodeProvider::is_lts_alias("lts/-2"));

        // Not LTS aliases
        assert!(!NodeProvider::is_lts_alias("20.18.0")); // Exact version
        assert!(!NodeProvider::is_lts_alias("^20.0.0")); // Semver range
        assert!(!NodeProvider::is_lts_alias("20")); // Partial version
        assert!(!NodeProvider::is_lts_alias("iron")); // Codename without lts/ prefix
        assert!(!NodeProvider::is_lts_alias("Lts/*")); // Wrong case for prefix
        assert!(!NodeProvider::is_lts_alias("LTS/*")); // All caps prefix
        assert!(!NodeProvider::is_lts_alias("")); // Empty
        assert!(!NodeProvider::is_lts_alias("latest")); // Different alias
        assert!(!NodeProvider::is_lts_alias("lts")); // No suffix
    }

    #[test]
    fn test_is_latest_alias() {
        // Valid "latest" aliases (case-insensitive)
        assert!(NodeProvider::is_latest_alias("latest"));
        assert!(NodeProvider::is_latest_alias("Latest"));
        assert!(NodeProvider::is_latest_alias("LATEST"));

        // Not "latest" aliases
        assert!(!NodeProvider::is_latest_alias("lts/*"));
        assert!(!NodeProvider::is_latest_alias("20.18.0"));
        assert!(!NodeProvider::is_latest_alias("^20.0.0"));
        assert!(!NodeProvider::is_latest_alias(""));
        assert!(!NodeProvider::is_latest_alias("late"));
        assert!(!NodeProvider::is_latest_alias("latestversion"));
    }

    #[test]
    fn test_is_version_alias() {
        // LTS aliases
        assert!(NodeProvider::is_version_alias("lts/*"));
        assert!(NodeProvider::is_version_alias("lts/iron"));

        // "latest" alias
        assert!(NodeProvider::is_version_alias("latest"));
        assert!(NodeProvider::is_version_alias("LATEST"));

        // Not aliases
        assert!(!NodeProvider::is_version_alias("20.18.0"));
        assert!(!NodeProvider::is_version_alias("^20.0.0"));
        assert!(!NodeProvider::is_version_alias(""));
    }

    #[tokio::test]
    async fn test_resolve_lts_alias_latest() {
        let provider = NodeProvider::new();

        // lts/* should resolve to the latest LTS version
        let version = provider.resolve_lts_alias("lts/*").await.unwrap();

        // Should be a valid semver version
        let parsed = Version::parse(&version).expect("Should parse as semver");

        // As of 2026, latest LTS is at least v24.x (Krypton)
        assert!(parsed.major >= 24, "Latest LTS should be at least v24.x, got {}", version);
    }

    #[tokio::test]
    async fn test_resolve_lts_alias_codename_iron() {
        let provider = NodeProvider::new();

        // lts/iron should resolve to v20.x
        let version = provider.resolve_lts_alias("lts/iron").await.unwrap();
        let parsed = Version::parse(&version).expect("Should parse as semver");
        assert_eq!(parsed.major, 20, "lts/iron should resolve to v20.x, got {}", version);
    }

    #[tokio::test]
    async fn test_resolve_lts_alias_codename_jod() {
        let provider = NodeProvider::new();

        // lts/jod should resolve to v22.x
        let version = provider.resolve_lts_alias("lts/jod").await.unwrap();
        let parsed = Version::parse(&version).expect("Should parse as semver");
        assert_eq!(parsed.major, 22, "lts/jod should resolve to v22.x, got {}", version);
    }

    #[tokio::test]
    async fn test_resolve_lts_alias_codename_case_insensitive() {
        let provider = NodeProvider::new();

        // Should be case-insensitive for codenames
        let version_lower = provider.resolve_lts_alias("lts/iron").await.unwrap();
        let version_mixed = provider.resolve_lts_alias("lts/Iron").await.unwrap();

        assert_eq!(version_lower, version_mixed, "LTS codename should be case-insensitive");
    }

    #[tokio::test]
    async fn test_resolve_lts_alias_offset() {
        let provider = NodeProvider::new();

        // lts/-1 should resolve to the second-highest LTS line
        // As of 2026: lts/* = 24.x (Krypton), lts/-1 = 22.x (Jod)
        let version = provider.resolve_lts_alias("lts/-1").await.unwrap();
        let parsed = Version::parse(&version).expect("Should parse as semver");
        assert_eq!(parsed.major, 22, "lts/-1 should resolve to v22.x (Jod), got {}", version);
    }

    #[tokio::test]
    async fn test_resolve_lts_alias_unknown_codename() {
        let provider = NodeProvider::new();

        // Unknown codename should error
        let result = provider.resolve_lts_alias("lts/unknown").await;
        assert!(result.is_err(), "Unknown LTS codename should return error");
    }

    #[tokio::test]
    async fn test_resolve_lts_alias_invalid_offset() {
        let provider = NodeProvider::new();

        // Too large offset should error (there aren't 100 LTS lines)
        let result = provider.resolve_lts_alias("lts/-100").await;
        assert!(result.is_err(), "Invalid LTS offset should return error");
    }
}
