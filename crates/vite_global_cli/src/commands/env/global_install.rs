//! Global package installation handling.

use std::{
    collections::HashSet,
    io::{Read, Write},
    process::Stdio,
};

use tokio::process::Command;
use vite_js_runtime::NodeProvider;
use vite_path::{AbsolutePath, current_dir};
use vite_shared::{format_path_prepended, output};

use super::{
    bin_config::BinConfig,
    config::{
        get_bin_dir, get_node_modules_dir, get_packages_dir, get_tmp_dir, resolve_version,
        resolve_version_alias,
    },
    package_metadata::PackageMetadata,
};
use crate::error::Error;

/// Install a global package.
///
/// If `node_version` is provided, uses that version. Otherwise, resolves from current directory.
/// If `force` is true, auto-uninstalls conflicting packages.
pub async fn install(
    package_spec: &str,
    node_version: Option<&str>,
    force: bool,
) -> Result<(), Error> {
    // Parse package spec (e.g., "typescript", "typescript@5.0.0", "@scope/pkg")
    let (package_name, _version_spec) = parse_package_spec(package_spec);

    output::raw(&format!("Installing {} globally...", package_spec));

    // 1. Resolve Node.js version
    let version = if let Some(v) = node_version {
        let provider = NodeProvider::new();
        resolve_version_alias(v, &provider).await?
    } else {
        // Resolve from current directory
        let cwd = current_dir().map_err(|e| {
            Error::ConfigError(format!("Cannot get current directory: {}", e).into())
        })?;
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
    //    Pipe stdout/stderr so npm output is hidden on success, shown on failure
    let output = Command::new(npm_path.as_path())
        .args(["install", "-g", "--no-fund", package_spec])
        .env("npm_config_prefix", staging_dir.as_path())
        .env("PATH", format_path_prepended(node_bin_dir.as_path()))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if !output.status.success() {
        // Clean up staging directory
        let _ = tokio::fs::remove_dir_all(&staging_dir).await;
        // Show captured output to help debug the failure
        let _ = std::io::stdout().write_all(&output.stdout);
        let _ = std::io::stderr().write_all(&output.stderr);
        return Err(Error::ConfigError(
            format!("npm install failed with exit code: {:?}", output.status.code()).into(),
        ));
    }

    // 5. Find installed package and extract metadata
    let node_modules_dir = get_node_modules_dir(&staging_dir, &package_name);
    let package_json_path = node_modules_dir.join("package.json");

    if !tokio::fs::try_exists(&package_json_path).await.unwrap_or(false) {
        let _ = tokio::fs::remove_dir_all(&staging_dir).await;
        return Err(Error::ConfigError(
            format!(
                "Package {} was not installed correctly, package.json not found at {}",
                package_name,
                package_json_path.as_path().display()
            )
            .into(),
        ));
    }

    // Read package.json to get version and binaries
    let package_json_content = tokio::fs::read_to_string(&package_json_path).await?;
    let package_json: serde_json::Value = serde_json::from_str(&package_json_content)
        .map_err(|e| Error::ConfigError(format!("Failed to parse package.json: {}", e).into()))?;

    let installed_version = package_json["version"].as_str().unwrap_or("unknown").to_string();

    let binary_infos = extract_binaries(&package_json);

    // Detect which binaries are JavaScript files
    let mut bin_names = Vec::new();
    let mut js_bins = HashSet::new();
    for info in &binary_infos {
        bin_names.push(info.name.clone());
        let binary_path = node_modules_dir.join(&info.path);
        if is_javascript_binary(&binary_path) {
            js_bins.insert(info.name.clone());
        }
    }

    // 5b. Check for binary conflicts (before moving staging to final location)
    let mut conflicts: Vec<(String, String)> = Vec::new(); // (bin_name, existing_package)

    for bin_name in &bin_names {
        if let Some(config) = BinConfig::load(bin_name).await? {
            // Only conflict if owned by a different package
            if config.package != package_name {
                conflicts.push((bin_name.clone(), config.package.clone()));
            }
        }
    }

    if !conflicts.is_empty() {
        if force {
            // Auto-uninstall conflicting packages
            let packages_to_remove: HashSet<_> =
                conflicts.iter().map(|(_, pkg)| pkg.clone()).collect();
            for pkg in packages_to_remove {
                output::raw(&format!("Uninstalling {} (conflicts with {})...", pkg, package_name));
                // Use Box::pin to avoid recursive async type issues
                Box::pin(uninstall(&pkg, false)).await?;
            }
        } else {
            // Hard fail with clear error
            // Clean up staging directory
            let _ = tokio::fs::remove_dir_all(&staging_dir).await;
            return Err(Error::BinaryConflict {
                bin_name: conflicts[0].0.clone(),
                existing_package: conflicts[0].1.clone(),
                new_package: package_name.clone(),
            });
        }
    }

    // 6. Move staging to final location
    let packages_dir = get_packages_dir()?;
    let final_dir = packages_dir.join(&package_name);

    // Remove existing installation if present
    if tokio::fs::try_exists(&final_dir).await.unwrap_or(false) {
        tokio::fs::remove_dir_all(&final_dir).await?;
    }

    // Create parent directory (handles scoped packages like @scope/pkg)
    if let Some(parent) = final_dir.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::rename(&staging_dir, &final_dir).await?;

    // 7. Save package metadata
    let metadata = PackageMetadata::new(
        package_name.clone(),
        installed_version.clone(),
        version.clone(),
        None, // npm version - could extract from runtime
        bin_names.clone(),
        js_bins,
        "npm".to_string(),
    );
    metadata.save().await?;

    // 8. Create shims for binaries and save per-binary configs
    let bin_dir = get_bin_dir()?;
    for bin_name in &bin_names {
        create_package_shim(&bin_dir, bin_name, &package_name).await?;

        // Write per-binary config
        let bin_config = BinConfig::new(
            bin_name.clone(),
            package_name.clone(),
            installed_version.clone(),
            version.clone(),
        );
        bin_config.save().await?;
    }

    output::raw(&format!("Installed {} v{}", package_name, installed_version));
    if !bin_names.is_empty() {
        output::raw(&format!("Binaries: {}", bin_names.join(", ")));
    }

    Ok(())
}

/// Uninstall a global package.
///
/// Uses two-phase uninstall:
/// 1. Try to use PackageMetadata for binary list
/// 2. Fallback to scanning BinConfig files for orphaned binaries
pub async fn uninstall(package_name: &str, dry_run: bool) -> Result<(), Error> {
    let (package_name, _) = parse_package_spec(package_name);

    // Phase 1: Try to use PackageMetadata for binary list
    let bins = if let Some(metadata) = PackageMetadata::load(&package_name).await? {
        metadata.bins.clone()
    } else {
        // Phase 2: Fallback - scan BinConfig files for orphaned binaries
        let orphan_bins = BinConfig::find_by_package(&package_name).await?;
        if orphan_bins.is_empty() {
            return Err(Error::ConfigError(
                format!("Package {} is not installed", package_name).into(),
            ));
        }
        orphan_bins
    };

    if dry_run {
        let bin_dir = get_bin_dir()?;
        let packages_dir = get_packages_dir()?;
        let package_dir = packages_dir.join(&package_name);
        let metadata_path = PackageMetadata::metadata_path(&package_name)?;

        output::raw(&format!("Would uninstall {}:", package_name));
        for bin_name in &bins {
            output::raw(&format!("  - shim: {}", bin_dir.join(bin_name).as_path().display()));
        }
        output::raw(&format!("  - package dir: {}", package_dir.as_path().display()));
        output::raw(&format!("  - metadata: {}", metadata_path.as_path().display()));
        return Ok(());
    }

    // Remove shims and bin configs
    let bin_dir = get_bin_dir()?;
    for bin_name in &bins {
        remove_package_shim(&bin_dir, bin_name).await?;
        BinConfig::delete(bin_name).await?;
    }

    // Remove package directory
    let packages_dir = get_packages_dir()?;
    let package_dir = packages_dir.join(&package_name);
    if tokio::fs::try_exists(&package_dir).await.unwrap_or(false) {
        tokio::fs::remove_dir_all(&package_dir).await?;
    }

    // Remove metadata file
    PackageMetadata::delete(&package_name).await?;

    output::raw(&format!("Uninstalled {}", package_name));

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

/// Binary info extracted from package.json.
struct BinaryInfo {
    /// Binary name (the command users will run)
    name: String,
    /// Relative path to the binary file from package root
    path: String,
}

/// Extract binary names and paths from package.json.
fn extract_binaries(package_json: &serde_json::Value) -> Vec<BinaryInfo> {
    let mut bins = Vec::new();

    if let Some(bin) = package_json.get("bin") {
        match bin {
            serde_json::Value::String(path) => {
                // Single binary with package name
                if let Some(name) = package_json["name"].as_str() {
                    // Get just the package name without scope
                    let bin_name = name.split('/').last().unwrap_or(name);
                    bins.push(BinaryInfo { name: bin_name.to_string(), path: path.clone() });
                }
            }
            serde_json::Value::Object(map) => {
                // Multiple binaries
                for (name, path) in map {
                    if let serde_json::Value::String(path) = path {
                        bins.push(BinaryInfo { name: name.clone(), path: path.clone() });
                    }
                }
            }
            _ => {}
        }
    }

    bins
}

/// Check if a file is a JavaScript file that should be run with Node.
///
/// Returns true if:
/// - The file has a .js, .mjs, or .cjs extension
/// - The file has a shebang containing "node"
///
/// This function safely reads only the first 256 bytes to check the shebang,
/// avoiding issues with binary files that may not have newlines.
fn is_javascript_binary(path: &AbsolutePath) -> bool {
    // Check extension first (fast path, no file I/O)
    if let Some(ext) = path.as_path().extension() {
        let ext = ext.to_string_lossy().to_lowercase();
        if ext == "js" || ext == "mjs" || ext == "cjs" {
            return true;
        }
    }

    // For extensionless files, read only first 256 bytes to check shebang
    // This is safe even for binary files
    if let Ok(mut file) = std::fs::File::open(path.as_path()) {
        let mut buffer = [0u8; 256];
        if let Ok(n) = file.read(&mut buffer) {
            if n >= 2 && buffer[0] == b'#' && buffer[1] == b'!' {
                // Found shebang, check for "node" in the first line
                // Find newline or use entire buffer
                let end = buffer[..n].iter().position(|&b| b == b'\n').unwrap_or(n);
                if let Ok(shebang) = std::str::from_utf8(&buffer[..end]) {
                    if shebang.contains("node") {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// Core shims that should not be overwritten by package binaries.
pub(crate) const CORE_SHIMS: &[&str] = &["node", "npm", "npx", "vp"];

/// Create a shim for a package binary.
///
/// On Unix: Creates a symlink to ../current/bin/vp
/// On Windows: Creates a trampoline .exe that forwards to vp.exe
async fn create_package_shim(
    bin_dir: &vite_path::AbsolutePath,
    bin_name: &str,
    package_name: &str,
) -> Result<(), Error> {
    // Check for conflicts with core shims
    if CORE_SHIMS.contains(&bin_name) {
        output::warn(&format!(
            "Package '{}' provides '{}' binary, but it conflicts with a core shim. Skipping.",
            package_name, bin_name
        ));
        return Ok(());
    }

    // Ensure bin directory exists
    tokio::fs::create_dir_all(bin_dir).await?;

    #[cfg(unix)]
    {
        let shim_path = bin_dir.join(bin_name);

        // Check if already a managed shim (symlink to ../current/bin/vp)
        if let Ok(target) = tokio::fs::read_link(&shim_path).await {
            if target == std::path::Path::new("../current/bin/vp") {
                return Ok(());
            }
            // Exists but points elsewhere (e.g., npm-installed direct symlink) — replace it
            tokio::fs::remove_file(&shim_path).await?;
        }

        // Create symlink to ../current/bin/vp
        tokio::fs::symlink("../current/bin/vp", &shim_path).await?;
        tracing::debug!("Created package shim symlink {:?} -> ../current/bin/vp", shim_path);
    }

    #[cfg(windows)]
    {
        let shim_path = bin_dir.join(format!("{}.exe", bin_name));

        // Skip if already exists (e.g., re-installing the same package)
        if tokio::fs::try_exists(&shim_path).await.unwrap_or(false) {
            return Ok(());
        }

        // Copy the trampoline binary as <bin_name>.exe.
        // The trampoline detects the tool name from its own filename and sets
        // VP_SHIM_TOOL env var before spawning vp.exe.
        let trampoline_src = super::setup::get_trampoline_path()?;
        tokio::fs::copy(trampoline_src.as_path(), &shim_path).await?;

        // Remove legacy .cmd and shell script wrappers from previous versions.
        // In Git Bash/MSYS, the extensionless script takes precedence over .exe,
        // so leftover wrappers would bypass the trampoline.
        super::setup::cleanup_legacy_windows_shim(bin_dir, bin_name).await;

        tracing::debug!("Created package trampoline shim {:?}", shim_path);
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
        // Use symlink_metadata to detect symlinks (even broken ones)
        if tokio::fs::symlink_metadata(&shim_path).await.is_ok() {
            tokio::fs::remove_file(&shim_path).await?;
        }
    }

    #[cfg(windows)]
    {
        // Remove trampoline .exe shim and legacy .cmd / shell script wrappers.
        // Best-effort: ignore NotFound errors for files that don't exist.
        for suffix in &[".exe", ".cmd", ""] {
            let path = if suffix.is_empty() {
                bin_dir.join(bin_name)
            } else {
                bin_dir.join(format!("{bin_name}{suffix}"))
            };
            let _ = tokio::fs::remove_file(&path).await;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RAII guard that sets `VP_TRAMPOLINE_PATH` to a fake binary on creation
    /// and clears it on drop. Ensures cleanup even on test panics.
    #[cfg(windows)]
    struct FakeTrampolineGuard;

    #[cfg(windows)]
    impl FakeTrampolineGuard {
        fn new(dir: &std::path::Path) -> Self {
            let trampoline = dir.join("vp-shim.exe");
            std::fs::write(&trampoline, b"fake-trampoline").unwrap();
            unsafe {
                std::env::set_var(vite_shared::env_vars::VP_TRAMPOLINE_PATH, &trampoline);
            }
            Self
        }
    }

    #[cfg(windows)]
    impl Drop for FakeTrampolineGuard {
        fn drop(&mut self) {
            unsafe {
                std::env::remove_var(vite_shared::env_vars::VP_TRAMPOLINE_PATH);
            }
        }
    }

    #[tokio::test]
    #[cfg_attr(windows, serial_test::serial)]
    async fn test_create_package_shim_creates_bin_dir() {
        use tempfile::TempDir;
        use vite_path::AbsolutePathBuf;

        // Create a temp directory but don't create the bin subdirectory
        let temp_dir = TempDir::new().unwrap();
        #[cfg(windows)]
        let _guard = FakeTrampolineGuard::new(temp_dir.path());
        let bin_dir = temp_dir.path().join("bin");
        let bin_dir = AbsolutePathBuf::new(bin_dir).unwrap();

        // Verify bin directory doesn't exist
        assert!(!bin_dir.as_path().exists());

        // Create a shim - this should create the bin directory
        create_package_shim(&bin_dir, "test-shim", "test-package").await.unwrap();

        // Verify bin directory was created
        assert!(bin_dir.as_path().exists());

        // Verify shim file was created (on Windows, shims have .exe extension)
        // On Unix, symlinks may be broken (target doesn't exist), so use symlink_metadata
        #[cfg(unix)]
        {
            let shim_path = bin_dir.join("test-shim");
            assert!(
                std::fs::symlink_metadata(shim_path.as_path()).is_ok(),
                "Symlink shim should exist"
            );
        }
        #[cfg(windows)]
        {
            let shim_path = bin_dir.join("test-shim.exe");
            assert!(shim_path.as_path().exists());
        }
    }

    #[tokio::test]
    async fn test_create_package_shim_skips_core_shims() {
        use tempfile::TempDir;
        use vite_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let bin_dir = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Try to create a shim for "node" which is a core shim
        create_package_shim(&bin_dir, "node", "some-package").await.unwrap();

        // Verify the shim was NOT created (core shims should be skipped)
        #[cfg(unix)]
        let shim_path = bin_dir.join("node");
        #[cfg(windows)]
        let shim_path = bin_dir.join("node.exe");
        assert!(!shim_path.as_path().exists());
    }

    #[tokio::test]
    #[cfg_attr(windows, serial_test::serial)]
    async fn test_remove_package_shim_removes_shim() {
        use tempfile::TempDir;
        use vite_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        #[cfg(windows)]
        let _guard = FakeTrampolineGuard::new(temp_dir.path());
        let bin_dir = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create a shim
        create_package_shim(&bin_dir, "tsc", "typescript").await.unwrap();

        // Verify the shim was created
        // On Unix, symlinks may be broken (target doesn't exist), so use symlink_metadata
        #[cfg(unix)]
        {
            let shim_path = bin_dir.join("tsc");
            assert!(
                std::fs::symlink_metadata(shim_path.as_path()).is_ok(),
                "Shim should exist after creation"
            );

            // Remove the shim
            remove_package_shim(&bin_dir, "tsc").await.unwrap();

            // Verify the shim was removed
            assert!(
                std::fs::symlink_metadata(shim_path.as_path()).is_err(),
                "Shim should be removed"
            );
        }
        #[cfg(windows)]
        {
            let shim_path = bin_dir.join("tsc.exe");
            assert!(shim_path.as_path().exists(), "Shim should exist after creation");

            // Remove the shim
            remove_package_shim(&bin_dir, "tsc").await.unwrap();

            // Verify the shim was removed
            assert!(!shim_path.as_path().exists(), "Shim should be removed");
        }
    }

    #[tokio::test]
    async fn test_remove_package_shim_handles_missing_shim() {
        use tempfile::TempDir;
        use vite_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let bin_dir = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Remove a shim that doesn't exist - should not error
        remove_package_shim(&bin_dir, "nonexistent").await.unwrap();
    }

    #[tokio::test]
    #[cfg_attr(windows, serial_test::serial)]
    async fn test_uninstall_removes_shims_from_metadata() {
        use tempfile::TempDir;
        use vite_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();
        #[cfg(windows)]
        let _trampoline_guard = FakeTrampolineGuard::new(&temp_path);
        let _env_guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(&temp_path),
        );

        // Create bin directory
        let bin_dir = AbsolutePathBuf::new(temp_path.join("bin")).unwrap();
        tokio::fs::create_dir_all(&bin_dir).await.unwrap();

        // Create shims for "tsc" and "tsserver"
        create_package_shim(&bin_dir, "tsc", "typescript").await.unwrap();
        create_package_shim(&bin_dir, "tsserver", "typescript").await.unwrap();

        // Verify shims exist
        // On Unix, symlinks may be broken (target doesn't exist), so use symlink_metadata
        #[cfg(unix)]
        {
            assert!(
                std::fs::symlink_metadata(bin_dir.join("tsc").as_path()).is_ok(),
                "tsc shim should exist"
            );
            assert!(
                std::fs::symlink_metadata(bin_dir.join("tsserver").as_path()).is_ok(),
                "tsserver shim should exist"
            );
        }
        #[cfg(windows)]
        {
            assert!(bin_dir.join("tsc.exe").as_path().exists(), "tsc.exe shim should exist");
            assert!(
                bin_dir.join("tsserver.exe").as_path().exists(),
                "tsserver.exe shim should exist"
            );
        }

        // Create metadata with bins
        let metadata = PackageMetadata::new(
            "typescript".to_string(),
            "5.9.3".to_string(),
            "20.18.0".to_string(),
            None,
            vec!["tsc".to_string(), "tsserver".to_string()],
            HashSet::from(["tsc".to_string(), "tsserver".to_string()]),
            "npm".to_string(),
        );
        metadata.save().await.unwrap();

        // Create package directory (needed for uninstall)
        let packages_dir = AbsolutePathBuf::new(temp_path.join("packages")).unwrap();
        let package_dir = packages_dir.join("typescript");
        tokio::fs::create_dir_all(&package_dir).await.unwrap();

        // Verify metadata was saved
        let loaded = PackageMetadata::load("typescript").await.unwrap();
        assert!(loaded.is_some(), "Metadata should be loaded");
        let loaded = loaded.unwrap();
        assert_eq!(loaded.bins, vec!["tsc", "tsserver"], "bins should match");

        // Run uninstall
        uninstall("typescript", false).await.unwrap();

        // Verify shims were removed
        #[cfg(unix)]
        {
            assert!(!bin_dir.join("tsc").as_path().exists(), "tsc shim should be removed");
            assert!(
                !bin_dir.join("tsserver").as_path().exists(),
                "tsserver shim should be removed"
            );
        }
        #[cfg(windows)]
        {
            assert!(!bin_dir.join("tsc.exe").as_path().exists(), "tsc.exe shim should be removed");
            assert!(
                !bin_dir.join("tsserver.exe").as_path().exists(),
                "tsserver.exe shim should be removed"
            );
        }
    }

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

    #[test]
    fn test_is_javascript_binary_with_js_extension() {
        use tempfile::TempDir;
        use vite_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let js_file = temp_dir.path().join("cli.js");
        std::fs::write(&js_file, "console.log('hello')").unwrap();

        let path = AbsolutePathBuf::new(js_file).unwrap();
        assert!(is_javascript_binary(&path));
    }

    #[test]
    fn test_is_javascript_binary_with_mjs_extension() {
        use tempfile::TempDir;
        use vite_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let mjs_file = temp_dir.path().join("cli.mjs");
        std::fs::write(&mjs_file, "export default 'hello'").unwrap();

        let path = AbsolutePathBuf::new(mjs_file).unwrap();
        assert!(is_javascript_binary(&path));
    }

    #[test]
    fn test_is_javascript_binary_with_cjs_extension() {
        use tempfile::TempDir;
        use vite_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let cjs_file = temp_dir.path().join("cli.cjs");
        std::fs::write(&cjs_file, "module.exports = 'hello'").unwrap();

        let path = AbsolutePathBuf::new(cjs_file).unwrap();
        assert!(is_javascript_binary(&path));
    }

    #[test]
    fn test_is_javascript_binary_with_node_shebang() {
        use tempfile::TempDir;
        use vite_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let cli_file = temp_dir.path().join("cli");
        std::fs::write(&cli_file, "#!/usr/bin/env node\nconsole.log('hello')").unwrap();

        let path = AbsolutePathBuf::new(cli_file).unwrap();
        assert!(is_javascript_binary(&path));
    }

    #[test]
    fn test_is_javascript_binary_with_direct_node_shebang() {
        use tempfile::TempDir;
        use vite_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let cli_file = temp_dir.path().join("cli");
        std::fs::write(&cli_file, "#!/usr/bin/node\nconsole.log('hello')").unwrap();

        let path = AbsolutePathBuf::new(cli_file).unwrap();
        assert!(is_javascript_binary(&path));
    }

    #[test]
    fn test_is_javascript_binary_native_executable() {
        use tempfile::TempDir;
        use vite_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        // Simulate a native binary (ELF header)
        let native_file = temp_dir.path().join("native-cli");
        std::fs::write(&native_file, b"\x7fELF").unwrap();

        let path = AbsolutePathBuf::new(native_file).unwrap();
        assert!(!is_javascript_binary(&path));
    }

    #[test]
    fn test_is_javascript_binary_shell_script() {
        use tempfile::TempDir;
        use vite_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let shell_file = temp_dir.path().join("script.sh");
        std::fs::write(&shell_file, "#!/bin/bash\necho hello").unwrap();

        let path = AbsolutePathBuf::new(shell_file).unwrap();
        assert!(!is_javascript_binary(&path));
    }

    #[test]
    fn test_is_javascript_binary_python_script() {
        use tempfile::TempDir;
        use vite_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let python_file = temp_dir.path().join("script.py");
        std::fs::write(&python_file, "#!/usr/bin/env python3\nprint('hello')").unwrap();

        let path = AbsolutePathBuf::new(python_file).unwrap();
        assert!(!is_javascript_binary(&path));
    }

    #[test]
    fn test_is_javascript_binary_empty_file() {
        use tempfile::TempDir;
        use vite_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let empty_file = temp_dir.path().join("empty");
        std::fs::write(&empty_file, "").unwrap();

        let path = AbsolutePathBuf::new(empty_file).unwrap();
        assert!(!is_javascript_binary(&path));
    }
}
