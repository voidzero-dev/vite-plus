//! Resolution cache for shim operations.
//!
//! Caches version resolution results to avoid re-resolving on every invocation.
//! Uses mtime-based invalidation to detect changes in version source files.

use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use vite_path::{AbsolutePath, AbsolutePathBuf};

/// Cache format version for upgrade compatibility
/// v2: Added `is_range` field to track range vs exact version for cache expiry
const CACHE_VERSION: u32 = 2;

/// Default maximum cache entries (LRU eviction)
const DEFAULT_MAX_ENTRIES: usize = 4096;

/// A single cache entry for a resolved version.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResolveCacheEntry {
    /// The resolved version string (e.g., "20.18.0")
    pub version: String,
    /// The source of the version (e.g., ".node-version", "engines.node")
    pub source: String,
    /// Project root directory (if applicable)
    pub project_root: Option<String>,
    /// Unix timestamp when this entry was resolved
    pub resolved_at: u64,
    /// Mtime of the version source file (for invalidation)
    pub version_file_mtime: u64,
    /// Path to the version source file
    pub source_path: Option<String>,
    /// Whether the original version spec was a range (e.g., "20", "^20.0.0", "lts/*")
    /// Range versions use time-based expiry (1 hour) instead of mtime-only validation
    #[serde(default)]
    pub is_range: bool,
}

/// Resolution cache stored in VITE_PLUS_HOME/cache/resolve_cache.json.
#[derive(Serialize, Deserialize, Debug)]
pub struct ResolveCache {
    /// Cache format version for upgrade compatibility
    version: u32,
    /// Cache entries keyed by current working directory
    entries: HashMap<String, ResolveCacheEntry>,
}

impl Default for ResolveCache {
    fn default() -> Self {
        Self { version: CACHE_VERSION, entries: HashMap::new() }
    }
}

impl ResolveCache {
    /// Load cache from disk.
    pub fn load(cache_path: &AbsolutePath) -> Self {
        match std::fs::read_to_string(cache_path) {
            Ok(content) => {
                match serde_json::from_str::<Self>(&content) {
                    Ok(cache) if cache.version == CACHE_VERSION => cache,
                    Ok(_) => {
                        // Version mismatch, reset cache
                        tracing::debug!("Cache version mismatch, resetting");
                        Self::default()
                    }
                    Err(e) => {
                        tracing::debug!("Failed to parse cache: {e}");
                        Self::default()
                    }
                }
            }
            Err(_) => Self::default(),
        }
    }

    /// Save cache to disk.
    pub fn save(&self, cache_path: &AbsolutePath) {
        // Ensure parent directory exists
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        if let Ok(content) = serde_json::to_string(self) {
            std::fs::write(cache_path, content).ok();
        }
    }

    /// Get a cache entry if valid.
    pub fn get(&self, cwd: &AbsolutePath) -> Option<&ResolveCacheEntry> {
        let key = cwd.as_path().to_string_lossy().to_string();
        let entry = self.entries.get(&key)?;

        // Validate mtime of source file
        if !self.is_entry_valid(entry) {
            return None;
        }

        Some(entry)
    }

    /// Insert a cache entry.
    pub fn insert(&mut self, cwd: &AbsolutePath, entry: ResolveCacheEntry) {
        let key = cwd.as_path().to_string_lossy().to_string();

        // LRU eviction if needed
        if self.entries.len() >= DEFAULT_MAX_ENTRIES {
            self.evict_oldest();
        }

        self.entries.insert(key, entry);
    }

    /// Check if an entry is still valid based on source file mtime and range status.
    ///
    /// For exact versions: Uses mtime-based validation only (cache valid until file changes)
    /// For range versions: Uses both mtime AND time-based expiry (1 hour TTL)
    ///
    /// This ensures range versions like "20" or "^20.0.0" are periodically re-resolved
    /// to pick up new releases, while exact versions like "20.18.0" only re-resolve
    /// when the source file is modified.
    fn is_entry_valid(&self, entry: &ResolveCacheEntry) -> bool {
        // For range versions (including LTS aliases), always apply time-based expiry
        // This ensures we periodically re-resolve to pick up new releases
        if entry.is_range {
            let now =
                SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            if now.saturating_sub(entry.resolved_at) >= 3600 {
                // Range cache expired (> 1 hour)
                return false;
            }
            // Range cache still within TTL, but also check mtime if source_path exists
            if let Some(source_path) = &entry.source_path {
                let path = std::path::Path::new(source_path);
                if let Ok(metadata) = std::fs::metadata(path) {
                    if let Ok(mtime) = metadata.modified() {
                        let mtime_secs =
                            mtime.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                        return mtime_secs == entry.version_file_mtime;
                    }
                }
                return false; // Source file missing or can't read mtime
            }
            return true; // No source file, within TTL
        }

        // For exact versions, check source file
        let Some(source_path) = &entry.source_path else {
            // No source file to validate (e.g., "lts" default)
            // Consider valid if resolved recently (within 1 hour)
            let now =
                SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            return now.saturating_sub(entry.resolved_at) < 3600;
        };

        let path = std::path::Path::new(source_path);
        let Ok(metadata) = std::fs::metadata(path) else {
            return false;
        };

        let Ok(mtime) = metadata.modified() else {
            return false;
        };

        let mtime_secs = mtime.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);

        mtime_secs == entry.version_file_mtime
    }

    /// Evict the oldest entry (by resolved_at timestamp).
    fn evict_oldest(&mut self) {
        if let Some((oldest_key, _)) = self
            .entries
            .iter()
            .min_by_key(|(_, entry)| entry.resolved_at)
            .map(|(k, v)| (k.clone(), v.clone()))
        {
            self.entries.remove(&oldest_key);
        }
    }
}

/// Get the cache file path.
pub fn get_cache_path() -> Option<AbsolutePathBuf> {
    let home = crate::commands::env::config::get_vite_plus_home().ok()?;
    Some(home.join("cache").join("resolve_cache.json"))
}

/// Invalidate the entire resolve cache by deleting the cache file.
/// Called after version configuration changes (e.g., `vp env default`, `vp env pin`, `vp env unpin`).
pub fn invalidate_cache() {
    if let Some(cache_path) = get_cache_path() {
        std::fs::remove_file(cache_path.as_path()).ok();
    }
}

/// Get the mtime of a file as Unix timestamp.
pub fn get_file_mtime(path: &AbsolutePath) -> Option<u64> {
    let metadata = std::fs::metadata(path).ok()?;
    let mtime = metadata.modified().ok()?;
    mtime.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).ok()
}

/// Get the current Unix timestamp.
pub fn now_timestamp() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_range_version_cache_should_expire_after_ttl() {
        // BUG: Currently, range versions with source_path use mtime-only validation
        // and never expire. They should use time-based expiry like aliases.

        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let cache_file = temp_path.join("cache.json");

        // Create a .node-version file
        let version_file = temp_path.join(".node-version");
        std::fs::write(&version_file, "20\n").unwrap();
        let mtime =
            get_file_mtime(&version_file).expect("Should be able to get mtime of created file");

        let mut cache = ResolveCache::default();

        // Create an entry for a range version (e.g., "20" resolved to "20.20.0")
        // with source_path set (from .node-version file) and resolved 2 hours ago
        let entry = ResolveCacheEntry {
            version: "20.20.0".to_string(),
            source: ".node-version".to_string(),
            project_root: None,
            resolved_at: now_timestamp() - 7200, // 2 hours ago (> 1 hour TTL)
            version_file_mtime: mtime,
            source_path: Some(version_file.as_path().display().to_string()),
            // BUG FIX: need to add is_range field
            is_range: true,
        };

        // Save entry to cache
        cache.insert(&temp_path, entry.clone());
        cache.save(&cache_file);

        // Reload cache
        let loaded_cache = ResolveCache::load(&cache_file);

        // BUG: This entry is still considered valid because mtime hasn't changed
        // but it SHOULD be invalid because it's a range and TTL has expired
        // After fix: is_entry_valid should return false for expired range entries
        let cached_entry = loaded_cache.get(&temp_path);

        // The cache entry should be INVALID (None) because:
        // 1. is_range is true
        // 2. resolved_at is > 1 hour ago
        // Even though the mtime hasn't changed
        assert!(
            cached_entry.is_none(),
            "Range version cache should expire after 1 hour TTL, \
             but mtime-only validation is returning the stale entry"
        );
    }

    #[test]
    fn test_exact_version_cache_uses_mtime_validation() {
        // Exact versions should use mtime-based validation, not time-based expiry
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let cache_file = temp_path.join("cache.json");

        // Create a .node-version file
        let version_file = temp_path.join(".node-version");
        std::fs::write(&version_file, "20.18.0\n").unwrap();
        let mtime = get_file_mtime(&version_file).unwrap();

        let mut cache = ResolveCache::default();

        // Create an entry for an exact version resolved 2 hours ago
        let entry = ResolveCacheEntry {
            version: "20.18.0".to_string(),
            source: ".node-version".to_string(),
            project_root: None,
            resolved_at: now_timestamp() - 7200, // 2 hours ago
            version_file_mtime: mtime,
            source_path: Some(version_file.as_path().display().to_string()),
            is_range: false, // Exact version, not a range
        };

        cache.insert(&temp_path, entry);
        cache.save(&cache_file);

        // Reload cache
        let loaded_cache = ResolveCache::load(&cache_file);
        let cached_entry = loaded_cache.get(&temp_path);

        // Exact version cache should still be valid as long as mtime hasn't changed
        assert!(
            cached_entry.is_some(),
            "Exact version cache should use mtime validation, not time-based expiry"
        );
        assert_eq!(cached_entry.unwrap().version, "20.18.0");
    }

    #[test]
    fn test_range_cache_valid_within_ttl() {
        // Range version cache should be valid within the 1 hour TTL
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let cache_file = temp_path.join("cache.json");

        // Create a .node-version file
        let version_file = temp_path.join(".node-version");
        std::fs::write(&version_file, "20\n").unwrap();
        let mtime = get_file_mtime(&version_file).unwrap();

        let mut cache = ResolveCache::default();

        // Create an entry for a range version resolved recently (30 minutes ago)
        let entry = ResolveCacheEntry {
            version: "20.20.0".to_string(),
            source: ".node-version".to_string(),
            project_root: None,
            resolved_at: now_timestamp() - 1800, // 30 minutes ago (< 1 hour TTL)
            version_file_mtime: mtime,
            source_path: Some(version_file.as_path().display().to_string()),
            is_range: true,
        };

        cache.insert(&temp_path, entry);
        cache.save(&cache_file);

        // Reload cache
        let loaded_cache = ResolveCache::load(&cache_file);
        let cached_entry = loaded_cache.get(&temp_path);

        // Range version cache should still be valid within TTL
        assert!(cached_entry.is_some(), "Range version cache should be valid within TTL");
        assert_eq!(cached_entry.unwrap().version, "20.20.0");
    }

    // Run serially: mutates VITE_PLUS_HOME env var which affects get_cache_path()
    #[test]
    #[serial_test::serial]
    fn test_invalidate_cache_removes_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Set VITE_PLUS_HOME to temp dir so invalidate_cache() targets our test file
        let cache_dir = temp_path.join("cache");
        std::fs::create_dir_all(&cache_dir).unwrap();
        let cache_file = cache_dir.join("resolve_cache.json");

        // Create a cache with an entry and save it
        let mut cache = ResolveCache::default();
        cache.insert(
            &temp_path,
            ResolveCacheEntry {
                version: "20.18.0".to_string(),
                source: ".node-version".to_string(),
                project_root: None,
                resolved_at: now_timestamp(),
                version_file_mtime: 0,
                source_path: None,
                is_range: false,
            },
        );
        cache.save(&cache_file);
        assert!(std::fs::metadata(cache_file.as_path()).is_ok(), "Cache file should exist");

        // Point VITE_PLUS_HOME to our temp dir and call invalidate_cache
        unsafe {
            std::env::set_var(vite_shared::env_vars::VITE_PLUS_HOME, temp_path.as_path());
        }
        invalidate_cache();
        unsafe {
            std::env::remove_var(vite_shared::env_vars::VITE_PLUS_HOME);
        }

        // Cache file should be removed
        assert!(
            std::fs::metadata(cache_file.as_path()).is_err(),
            "Cache file should be removed after invalidation"
        );

        // Loading from removed file should return empty default cache
        let loaded_cache = ResolveCache::load(&cache_file);
        assert!(loaded_cache.get(&temp_path).is_none(), "Cache should be empty after invalidation");
    }
}
