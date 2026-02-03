//! Global package installation handling.

use std::process::Stdio;

use tokio::process::Command;
use vite_js_runtime::NodeProvider;
use vite_path::AbsolutePathBuf;

use super::{
    config::{get_bin_dir, get_packages_dir, get_tmp_dir, resolve_version},
    package_metadata::PackageMetadata,
};
use crate::error::Error;

/// Install a global package.
///
/// If `node_version` is provided, uses that version. Otherwise, resolves from current directory.
pub async fn install(package_spec: &str, node_version: Option<&str>) -> Result<(), Error> {
    // Parse package spec (e.g., "typescript", "typescript@5.0.0", "@scope/pkg")
    let (package_name, _version_spec) = parse_package_spec(package_spec);

    println!("  Installing {} globally...", package_spec);

    // 1. Resolve Node.js version
    let version = if let Some(v) = node_version {
        // Resolve the provided version to an exact version
        let provider = NodeProvider::new();
        if NodeProvider::is_exact_version(v) {
            v.to_string()
        } else {
            provider.resolve_version(v).await?.to_string()
        }
    } else {
        // Resolve from current directory
        let cwd = std::env::current_dir().map_err(|e| {
            Error::ConfigError(format!("Cannot get current directory: {}", e).into())
        })?;
        let cwd = AbsolutePathBuf::new(cwd)
            .ok_or_else(|| Error::ConfigError("Invalid current directory".into()))?;
        let resolution = resolve_version(&cwd).await?;
        resolution.version
    };

    // 2. Ensure Node.js is installed
    let runtime =
        vite_js_runtime::download_runtime(vite_js_runtime::JsRuntimeType::Node, &version).await?;

    let node_bin_dir = runtime.get_bin_prefix();
    let npm_path =
        if cfg!(windows) { node_bin_dir.join("npm.cmd") } else { node_bin_dir.join("npm") };

    // 3. Create staging directory
    let tmp_dir = get_tmp_dir()?;
    let staging_dir = tmp_dir.join("packages").join(&package_name);

    // Clean up any previous failed install
    if tokio::fs::try_exists(&staging_dir).await.unwrap_or(false) {
        tokio::fs::remove_dir_all(&staging_dir).await?;
    }
    tokio::fs::create_dir_all(&staging_dir).await?;

    // 4. Run npm install with prefix set to staging directory
    println!("  Running npm install...");

    let status = Command::new(npm_path.as_path())
        .args(["install", "-g", package_spec])
        .env("npm_config_prefix", staging_dir.as_path())
        .env(
            "PATH",
            format!(
                "{}:{}",
                node_bin_dir.as_path().display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await?;

    if !status.success() {
        // Clean up staging directory
        let _ = tokio::fs::remove_dir_all(&staging_dir).await;
        return Err(Error::ConfigError(
            format!("npm install failed with exit code: {:?}", status.code()).into(),
        ));
    }

    // 5. Find installed package and extract metadata
    let node_modules_dir = staging_dir.join("lib").join("node_modules").join(&package_name);
    let package_json_path = node_modules_dir.join("package.json");

    if !tokio::fs::try_exists(&package_json_path).await.unwrap_or(false) {
        let _ = tokio::fs::remove_dir_all(&staging_dir).await;
        return Err(Error::ConfigError(
            format!("Package {} was not installed correctly", package_name).into(),
        ));
    }

    // Read package.json to get version and binaries
    let package_json_content = tokio::fs::read_to_string(&package_json_path).await?;
    let package_json: serde_json::Value = serde_json::from_str(&package_json_content)
        .map_err(|e| Error::ConfigError(format!("Failed to parse package.json: {}", e).into()))?;

    let installed_version = package_json["version"].as_str().unwrap_or("unknown").to_string();

    let bins = extract_binaries(&package_json);

    // 6. Move staging to final location
    let packages_dir = get_packages_dir()?;
    let final_dir = packages_dir.join(&package_name);

    // Remove existing installation if present
    if tokio::fs::try_exists(&final_dir).await.unwrap_or(false) {
        tokio::fs::remove_dir_all(&final_dir).await?;
    }

    tokio::fs::create_dir_all(&packages_dir).await?;
    tokio::fs::rename(&staging_dir, &final_dir).await?;

    // 7. Save package metadata
    let metadata = PackageMetadata::new(
        package_name.clone(),
        installed_version.clone(),
        version.clone(),
        None, // npm version - could extract from runtime
        bins.clone(),
        "npm".to_string(),
    );
    metadata.save().await?;

    // 8. Create shims for binaries
    let bin_dir = get_bin_dir()?;
    for bin_name in &bins {
        create_package_shim(&bin_dir, bin_name, &package_name).await?;
    }

    println!("  Installed {} v{}", package_name, installed_version);
    if !bins.is_empty() {
        println!("  Binaries: {}", bins.join(", "));
    }

    Ok(())
}

/// Uninstall a global package.
pub async fn uninstall(package_name: &str) -> Result<(), Error> {
    let (package_name, _) = parse_package_spec(package_name);

    println!("  Uninstalling {}...", package_name);

    // 1. Load metadata to get binary names
    let metadata = PackageMetadata::load(&package_name).await?;

    if metadata.is_none() {
        return Err(Error::ConfigError(
            format!("Package {} is not installed", package_name).into(),
        ));
    }

    let metadata = metadata.unwrap();

    // 2. Remove shims for binaries
    let bin_dir = get_bin_dir()?;
    for bin_name in &metadata.bins {
        remove_package_shim(&bin_dir, bin_name).await?;
    }

    // 3. Remove package directory
    let packages_dir = get_packages_dir()?;
    let package_dir = packages_dir.join(&package_name);
    if tokio::fs::try_exists(&package_dir).await.unwrap_or(false) {
        tokio::fs::remove_dir_all(&package_dir).await?;
    }

    // 4. Remove metadata file
    PackageMetadata::delete(&package_name).await?;

    println!("  Uninstalled {}", package_name);

    Ok(())
}

/// Parse package spec into name and optional version.
fn parse_package_spec(spec: &str) -> (String, Option<String>) {
    // Handle scoped packages: @scope/name@version
    if spec.starts_with('@') {
        // Find the second @ for version
        if let Some(idx) = spec[1..].find('@') {
            let idx = idx + 1; // Adjust for the skipped first char
            return (spec[..idx].to_string(), Some(spec[idx + 1..].to_string()));
        }
        return (spec.to_string(), None);
    }

    // Handle regular packages: name@version
    if let Some(idx) = spec.find('@') {
        return (spec[..idx].to_string(), Some(spec[idx + 1..].to_string()));
    }

    (spec.to_string(), None)
}

/// Extract binary names from package.json.
fn extract_binaries(package_json: &serde_json::Value) -> Vec<String> {
    let mut bins = Vec::new();

    if let Some(bin) = package_json.get("bin") {
        match bin {
            serde_json::Value::String(_) => {
                // Single binary with package name
                if let Some(name) = package_json["name"].as_str() {
                    // Get just the package name without scope
                    let bin_name = name.split('/').last().unwrap_or(name);
                    bins.push(bin_name.to_string());
                }
            }
            serde_json::Value::Object(map) => {
                // Multiple binaries
                for key in map.keys() {
                    bins.push(key.clone());
                }
            }
            _ => {}
        }
    }

    bins
}

/// Core shims that should not be overwritten by package binaries.
const CORE_SHIMS: &[&str] = &["node", "npm", "npx", "vp"];

/// Create a shim for a package binary.
async fn create_package_shim(
    bin_dir: &vite_path::AbsolutePath,
    bin_name: &str,
    package_name: &str,
) -> Result<(), Error> {
    // Check for conflicts with core shims
    if CORE_SHIMS.contains(&bin_name) {
        println!(
            "  Warning: Package '{}' provides '{}' binary, but it conflicts with a core shim. Skipping.",
            package_name, bin_name
        );
        return Ok(());
    }

    // Ensure bin directory exists
    tokio::fs::create_dir_all(bin_dir).await?;

    #[cfg(unix)]
    {
        let current_exe = std::env::current_exe().map_err(|e| {
            Error::ConfigError(format!("Cannot find current executable: {e}").into())
        })?;

        let shim_path = bin_dir.join(bin_name);

        // Skip if already exists (e.g., re-installing the same package)
        if tokio::fs::try_exists(&shim_path).await.unwrap_or(false) {
            return Ok(());
        }

        // Create hardlink
        if tokio::fs::hard_link(&current_exe, &shim_path).await.is_err() {
            // Fallback to copy
            tokio::fs::copy(&current_exe, &shim_path).await?;
        }
    }

    #[cfg(windows)]
    {
        let shim_path = bin_dir.join(format!("{}.cmd", bin_name));

        // Skip if already exists (e.g., re-installing the same package)
        if tokio::fs::try_exists(&shim_path).await.unwrap_or(false) {
            return Ok(());
        }

        // Create .cmd wrapper
        let wrapper_content = format!(
            "@echo off\r\nsetlocal\r\nset \"VITE_PLUS_SHIM_TOOL={}\"\r\n\"%~dp0node.exe\" %*\r\nexit /b %ERRORLEVEL%\r\n",
            bin_name
        );
        tokio::fs::write(&shim_path, wrapper_content).await?;
    }

    Ok(())
}

/// Remove a shim for a package binary.
async fn remove_package_shim(
    bin_dir: &vite_path::AbsolutePath,
    bin_name: &str,
) -> Result<(), Error> {
    // Don't remove core shims
    if CORE_SHIMS.contains(&bin_name) {
        return Ok(());
    }

    #[cfg(unix)]
    {
        let shim_path = bin_dir.join(bin_name);
        if tokio::fs::try_exists(&shim_path).await.unwrap_or(false) {
            tokio::fs::remove_file(&shim_path).await?;
        }
    }

    #[cfg(windows)]
    {
        let shim_path = bin_dir.join(format!("{}.cmd", bin_name));
        if tokio::fs::try_exists(&shim_path).await.unwrap_or(false) {
            tokio::fs::remove_file(&shim_path).await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_package_spec_simple() {
        let (name, version) = parse_package_spec("typescript");
        assert_eq!(name, "typescript");
        assert_eq!(version, None);
    }

    #[test]
    fn test_parse_package_spec_with_version() {
        let (name, version) = parse_package_spec("typescript@5.0.0");
        assert_eq!(name, "typescript");
        assert_eq!(version, Some("5.0.0".to_string()));
    }

    #[test]
    fn test_parse_package_spec_scoped() {
        let (name, version) = parse_package_spec("@types/node");
        assert_eq!(name, "@types/node");
        assert_eq!(version, None);
    }

    #[test]
    fn test_parse_package_spec_scoped_with_version() {
        let (name, version) = parse_package_spec("@types/node@20.0.0");
        assert_eq!(name, "@types/node");
        assert_eq!(version, Some("20.0.0".to_string()));
    }
}
