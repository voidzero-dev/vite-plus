//! PATH environment variable manipulation utilities.
//!
//! This module provides functions for prepending directories to the PATH
//! environment variable with various deduplication strategies.

use std::{env, ffi::OsString, path::Path};

use vite_path::AbsolutePath;

/// Options for deduplication behavior when prepending to PATH.
#[derive(Debug, Clone, Copy, Default)]
pub struct PrependOptions {
    /// If `false`, only check if the directory is first in PATH (faster).
    /// If `true`, check if the directory exists anywhere in PATH.
    pub dedupe_anywhere: bool,
}

/// Result of a PATH prepend operation.
#[derive(Debug)]
pub enum PrependResult {
    /// The directory was prepended successfully.
    Prepended(OsString),
    /// The directory is already present in PATH (based on dedup strategy).
    AlreadyPresent,
    /// Failed to join paths (invalid characters in path).
    JoinError,
}

/// Format PATH with the given directory prepended.
///
/// This returns a new PATH value without modifying the environment.
/// Use this when you need to set PATH on a `Command` via `cmd.env()`.
///
/// # Arguments
/// * `dir` - The directory to prepend to PATH
/// * `options` - Deduplication options
///
/// # Returns
/// * `PrependResult::Prepended(new_path)` - The new PATH value with directory prepended
/// * `PrependResult::AlreadyPresent` - Directory already exists in PATH (based on options)
/// * `PrependResult::JoinError` - Failed to join paths
pub fn format_path_with_prepend(dir: impl AsRef<Path>, options: PrependOptions) -> PrependResult {
    let dir = dir.as_ref();
    let current_path = env::var_os("PATH").unwrap_or_default();
    let paths: Vec<_> = env::split_paths(&current_path).collect();

    // Check for duplicates based on strategy
    if options.dedupe_anywhere {
        if paths.iter().any(|p| p == dir) {
            return PrependResult::AlreadyPresent;
        }
    } else if let Some(first) = paths.first() {
        if first == dir {
            return PrependResult::AlreadyPresent;
        }
    }

    // Prepend the directory
    let mut new_paths = vec![dir.to_path_buf()];
    new_paths.extend(paths);

    match env::join_paths(new_paths) {
        Ok(new_path) => PrependResult::Prepended(new_path),
        Err(_) => PrependResult::JoinError,
    }
}

/// Prepend a directory to the global PATH environment variable.
///
/// This modifies the process environment using `std::env::set_var`.
///
/// # Safety
/// This function uses `unsafe` to call `std::env::set_var`, which is unsafe
/// in multi-threaded contexts. Only call this before spawning threads or
/// when you're certain no other threads are reading environment variables.
///
/// # Arguments
/// * `dir` - The directory to prepend to PATH
/// * `options` - Deduplication options
///
/// # Returns
/// * `true` if PATH was modified
/// * `false` if the directory was already present or join failed
pub fn prepend_to_path_env(dir: &AbsolutePath, options: PrependOptions) -> bool {
    match format_path_with_prepend(dir.as_path(), options) {
        PrependResult::Prepended(new_path) => {
            // SAFETY: Caller ensures this is safe (single-threaded or before exec)
            unsafe { env::set_var("PATH", new_path) };
            true
        }
        PrependResult::AlreadyPresent | PrependResult::JoinError => false,
    }
}

/// Format PATH with the given directory prepended (simple version).
///
/// This is a simpler version that always prepends without deduplication.
/// Use this for backward compatibility with `format_path_env`.
///
/// # Arguments
/// * `bin_prefix` - The directory to prepend to PATH
///
/// # Returns
/// The new PATH value as a String
pub fn format_path_prepended(bin_prefix: impl AsRef<Path>) -> String {
    let mut paths = env::split_paths(&env::var_os("PATH").unwrap_or_default()).collect::<Vec<_>>();
    paths.insert(0, bin_prefix.as_ref().to_path_buf());
    env::join_paths(paths).unwrap().to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn test_prepend_options_default() {
        let options = PrependOptions::default();
        assert!(!options.dedupe_anywhere);
    }

    #[test]
    fn test_format_path_prepended() {
        let result = format_path_prepended("/test/bin");
        assert!(result.starts_with("/test/bin"));
    }

    #[test]
    fn test_format_path_with_prepend_dedupe_first() {
        // With dedupe_anywhere = false, should check first element only
        let options = PrependOptions { dedupe_anywhere: false };
        let result = format_path_with_prepend(PathBuf::from("/new/path"), options);
        assert!(matches!(result, PrependResult::Prepended(_)));
    }

    #[test]
    fn test_format_path_with_prepend_dedupe_anywhere() {
        let options = PrependOptions { dedupe_anywhere: true };
        let result = format_path_with_prepend(PathBuf::from("/new/path"), options);
        assert!(matches!(result, PrependResult::Prepended(_)));
    }

    #[test]
    #[ignore]
    fn test_format_path_prepended_always_prepends() {
        // Even if the directory exists somewhere in PATH, it should be prepended
        let test_dir = "/test/node/bin";

        // Set PATH to include test_dir in the middle
        // SAFETY: This test runs in isolation
        unsafe {
            std::env::set_var("PATH", format!("/other/bin:{}:/another/bin", test_dir));
        }

        let result = format_path_prepended(test_dir);

        // Should start with test_dir regardless of existing PATH entries
        assert!(
            result.starts_with(test_dir),
            "Directory should always be first in PATH, got: {}",
            result
        );

        // Restore PATH
        unsafe {
            std::env::remove_var("PATH");
        }
    }
}
