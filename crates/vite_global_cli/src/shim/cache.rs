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
const CACHE_VERSION: u32 = 1;

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

    /// Check if an entry is still valid based on source file mtime.
    fn is_entry_valid(&self, entry: &ResolveCacheEntry) -> bool {
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
