//! Pin command for per-directory Node.js version management.
//!
//! Handles `vp env pin [VERSION]` to pin a Node.js version in the current directory
//! by creating or updating a `.node-version` file.

use std::{io::Write, process::ExitStatus};

use vite_js_runtime::NodeProvider;
use vite_path::AbsolutePathBuf;
use vite_shared::output;

use super::config::{get_config_path, load_config};
use crate::error::Error;

/// Node version file name
const NODE_VERSION_FILE: &str = ".node-version";

/// Execute the pin command.
pub async fn execute(
    cwd: AbsolutePathBuf,
    version: Option<String>,
    unpin: bool,
    no_install: bool,
    force: bool,
) -> Result<ExitStatus, Error> {
    // Handle --unpin flag
    if unpin {
        return do_unpin(&cwd).await;
    }

    match version {
        Some(v) => do_pin(&cwd, &v, no_install, force).await,
        None => show_pinned(&cwd).await,
    }
}

/// Show the current pinned version.
async fn show_pinned(cwd: &AbsolutePathBuf) -> Result<ExitStatus, Error> {
    let node_version_path = cwd.join(NODE_VERSION_FILE);

    // Check if .node-version exists in current directory
    if tokio::fs::try_exists(&node_version_path).await.unwrap_or(false) {
        let content = tokio::fs::read_to_string(&node_version_path).await?;
        let version = content.trim();
        println!("Pinned version: {version}");
        println!("  Source: {}", node_version_path.as_path().display());
        return Ok(ExitStatus::default());
    }

    // Check for inherited version from parent directories
    if let Some((version, source_path)) = find_inherited_version(cwd).await? {
        println!("No version pinned in current directory.");
        println!("  Inherited: {version} from {}", source_path.as_path().display());
        return Ok(ExitStatus::default());
    }

    // No .node-version anywhere - show default
    let config = load_config().await?;
    match config.default_node_version {
        Some(version) => {
            let config_path = get_config_path()?;
            println!("No version pinned.");
            println!("  Using default: {version} (from {})", config_path.as_path().display());
        }
        None => {
            println!("No version pinned.");
            println!("  Run 'vp env pin <version>' to pin a version.");
        }
    }

    Ok(ExitStatus::default())
}

/// Find .node-version in parent directories.
async fn find_inherited_version(
    cwd: &AbsolutePathBuf,
) -> Result<Option<(String, AbsolutePathBuf)>, Error> {
    let mut current: Option<AbsolutePathBuf> = cwd.parent().map(|p| p.to_absolute_path_buf());

    while let Some(dir) = current {
        let node_version_path = dir.join(NODE_VERSION_FILE);
        if tokio::fs::try_exists(&node_version_path).await.unwrap_or(false) {
            let content = tokio::fs::read_to_string(&node_version_path).await?;
            return Ok(Some((content.trim().to_string(), node_version_path)));
        }
        current = dir.parent().map(|p| p.to_absolute_path_buf());
    }

    Ok(None)
}

/// Pin a version to the current directory.
async fn do_pin(
    cwd: &AbsolutePathBuf,
    version: &str,
    no_install: bool,
    force: bool,
) -> Result<ExitStatus, Error> {
    let provider = NodeProvider::new();
    let node_version_path = cwd.join(NODE_VERSION_FILE);

    // Resolve the version (aliases like lts/latest are resolved to exact versions)
    let (resolved_version, was_alias) = resolve_version_for_pin(version, &provider).await?;

    // Check if .node-version already exists
    if !force && tokio::fs::try_exists(&node_version_path).await.unwrap_or(false) {
        let existing_content = tokio::fs::read_to_string(&node_version_path).await?;
        let existing_version = existing_content.trim();

        if existing_version == resolved_version {
            println!("Already pinned to {resolved_version}");
            return Ok(ExitStatus::default());
        }

        // Prompt for confirmation
        print!(".node-version already exists with version {existing_version}");
        println!();
        print!("Overwrite with {resolved_version}? (y/n): ");
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(ExitStatus::default());
        }
    }

    // Write the version to .node-version
    tokio::fs::write(&node_version_path, format!("{resolved_version}\n")).await?;

    // Invalidate resolve cache so the pinned version takes effect immediately
    crate::shim::invalidate_cache();

    // Print success message
    if was_alias {
        output::success(&format!(
            "Pinned Node.js version to {resolved_version} (resolved from {version})"
        ));
    } else {
        output::success(&format!("Pinned Node.js version to {resolved_version}"));
    }
    println!("  Created {} in {}", NODE_VERSION_FILE, cwd.as_path().display());

    // Pre-download the version unless --no-install is specified
    if no_install {
        output::note("Version will be downloaded on first use.");
    } else {
        // Download the runtime
        match vite_js_runtime::download_runtime(
            vite_js_runtime::JsRuntimeType::Node,
            &resolved_version,
        )
        .await
        {
            Ok(_) => {
                output::success(&format!("Node.js {resolved_version} installed"));
            }
            Err(e) => {
                output::warn(&format!("Failed to download Node.js {resolved_version}: {e}"));
                output::note("Version will be downloaded on first use.");
            }
        }
    }

    Ok(ExitStatus::default())
}

/// Resolve version for pinning.
///
/// Aliases (lts, latest) are resolved to exact versions.
/// Returns (resolved_version, was_alias).
async fn resolve_version_for_pin(
    version: &str,
    provider: &NodeProvider,
) -> Result<(String, bool), Error> {
    match version.to_lowercase().as_str() {
        "lts" => {
            let resolved = provider.resolve_latest_version().await?;
            Ok((resolved.to_string(), true))
        }
        "latest" => {
            let resolved = provider.resolve_version("*").await?;
            Ok((resolved.to_string(), true))
        }
        _ => {
            // For exact versions, validate they exist
            if NodeProvider::is_exact_version(version) {
                // Validate the version exists by trying to resolve it
                provider.resolve_version(version).await?;
                Ok((version.to_string(), false))
            } else {
                // For ranges/partial versions, resolve to exact version
                let resolved = provider.resolve_version(version).await?;
                Ok((resolved.to_string(), true))
            }
        }
    }
}

/// Remove the .node-version file from current directory.
pub async fn do_unpin(cwd: &AbsolutePathBuf) -> Result<ExitStatus, Error> {
    let node_version_path = cwd.join(NODE_VERSION_FILE);

    if !tokio::fs::try_exists(&node_version_path).await.unwrap_or(false) {
        println!("No {} file in current directory.", NODE_VERSION_FILE);
        return Ok(ExitStatus::default());
    }

    tokio::fs::remove_file(&node_version_path).await?;

    // Invalidate resolve cache so the unpinned version falls back correctly
    crate::shim::invalidate_cache();

    output::success(&format!("Removed {} from {}", NODE_VERSION_FILE, cwd.as_path().display()));

    Ok(ExitStatus::default())
}

#[cfg(test)]
mod tests {
    use serial_test::serial;
    use tempfile::TempDir;
    use vite_path::AbsolutePathBuf;

    use super::*;

    #[tokio::test]
    async fn test_show_pinned_no_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Should not error when no .node-version exists
        let result = show_pinned(&temp_path).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_show_pinned_with_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create .node-version
        tokio::fs::write(temp_path.join(".node-version"), "20.18.0\n").await.unwrap();

        let result = show_pinned(&temp_path).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_find_inherited_version() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create .node-version in parent
        tokio::fs::write(temp_path.join(".node-version"), "20.18.0\n").await.unwrap();

        // Create subdirectory
        let subdir = temp_path.join("subdir");
        tokio::fs::create_dir(&subdir).await.unwrap();

        let result = find_inherited_version(&subdir).await.unwrap();
        assert!(result.is_some());
        let (version, _) = result.unwrap();
        assert_eq!(version, "20.18.0");
    }

    #[tokio::test]
    async fn test_do_unpin() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create .node-version
        let node_version_path = temp_path.join(".node-version");
        tokio::fs::write(&node_version_path, "20.18.0\n").await.unwrap();

        // Unpin
        let result = do_unpin(&temp_path).await;
        assert!(result.is_ok());

        // File should be gone
        assert!(!tokio::fs::try_exists(&node_version_path).await.unwrap());
    }

    #[tokio::test]
    // Run serially: mutates VITE_PLUS_HOME env var which affects invalidate_cache()
    #[serial]
    async fn test_do_unpin_invalidates_cache() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Point VITE_PLUS_HOME to temp dir
        unsafe {
            std::env::set_var(vite_shared::env_vars::VITE_PLUS_HOME, temp_path.as_path());
        }

        // Create cache file manually
        let cache_dir = temp_path.join("cache");
        std::fs::create_dir_all(&cache_dir).unwrap();
        let cache_file = cache_dir.join("resolve_cache.json");
        std::fs::write(&cache_file, r#"{"version":2,"entries":{}}"#).unwrap();
        assert!(
            std::fs::metadata(cache_file.as_path()).is_ok(),
            "Cache file should exist before unpin"
        );

        // Create .node-version and unpin
        let node_version_path = temp_path.join(".node-version");
        tokio::fs::write(&node_version_path, "20.18.0\n").await.unwrap();
        let result = do_unpin(&temp_path).await;
        assert!(result.is_ok());

        // Cache file should be removed by invalidate_cache()
        assert!(
            std::fs::metadata(cache_file.as_path()).is_err(),
            "Cache file should be removed after unpin"
        );

        // Cleanup
        unsafe {
            std::env::remove_var(vite_shared::env_vars::VITE_PLUS_HOME);
        }
    }

    // Run serially: mutates VITE_PLUS_HOME env var which affects invalidate_cache()
    #[tokio::test]
    #[serial]
    async fn test_do_pin_invalidates_cache() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Point VITE_PLUS_HOME to temp dir
        unsafe {
            std::env::set_var(vite_shared::env_vars::VITE_PLUS_HOME, temp_path.as_path());
        }

        // Create cache file manually
        let cache_dir = temp_path.join("cache");
        std::fs::create_dir_all(&cache_dir).unwrap();
        let cache_file = cache_dir.join("resolve_cache.json");
        std::fs::write(&cache_file, r#"{"version":2,"entries":{}}"#).unwrap();
        assert!(
            std::fs::metadata(cache_file.as_path()).is_ok(),
            "Cache file should exist before pin"
        );

        // Pin an exact version (no_install=true to skip download, force=true to skip prompt)
        let result = do_pin(&temp_path, "20.18.0", true, true).await;
        assert!(result.is_ok());

        // .node-version should be created
        let node_version_path = temp_path.join(".node-version");
        assert!(tokio::fs::try_exists(&node_version_path).await.unwrap());
        let content = tokio::fs::read_to_string(&node_version_path).await.unwrap();
        assert_eq!(content.trim(), "20.18.0");

        // Cache file should be removed by invalidate_cache()
        assert!(
            std::fs::metadata(cache_file.as_path()).is_err(),
            "Cache file should be removed after pin"
        );

        // Cleanup
        unsafe {
            std::env::remove_var(vite_shared::env_vars::VITE_PLUS_HOME);
        }
    }

    #[tokio::test]
    async fn test_do_unpin_no_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Should not error when no file exists
        let result = do_unpin(&temp_path).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_resolve_version_for_pin_partial_version() {
        let provider = NodeProvider::new();

        // Partial version "20" should resolve to an exact version like "20.x.y"
        let (resolved, was_alias) = resolve_version_for_pin("20", &provider).await.unwrap();
        assert!(was_alias, "partial version should be treated as alias");

        // The resolved version should be a full semver version starting with "20."
        assert!(
            resolved.starts_with("20."),
            "expected resolved version to start with '20.', got: {resolved}"
        );

        // Should be a valid exact version (major.minor.patch)
        let parts: Vec<&str> = resolved.split('.').collect();
        assert_eq!(parts.len(), 3, "expected 3 version parts, got: {resolved}");
        assert!(parts.iter().all(|p| p.parse::<u64>().is_ok()), "all parts should be numeric");
    }

    #[tokio::test]
    async fn test_resolve_version_for_pin_exact_version() {
        let provider = NodeProvider::new();

        // Exact version should be returned as-is
        let (resolved, was_alias) = resolve_version_for_pin("20.18.0", &provider).await.unwrap();
        assert!(!was_alias, "exact version should not be treated as alias");
        assert_eq!(resolved, "20.18.0");
    }
}
