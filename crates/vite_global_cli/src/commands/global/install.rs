//! Global package installation handling.

use std::{
    collections::{HashMap, HashSet},
    io::{IsTerminal, Read, Write},
    process::Stdio,
    time::Duration,
};

use futures::{StreamExt, stream::FuturesUnordered};
use indexmap::IndexMap;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use owo_colors::OwoColorize;
use tokio::process::Command;
use vite_js_runtime::NodeProvider;
use vite_path::{AbsolutePath, AbsolutePathBuf, current_dir};
use vite_shared::{format_path_prepended, output};

use crate::{
    commands::{
        env::{
            bin_config::BinConfig,
            config::{
                get_bin_dir, get_node_modules_dir, get_packages_dir, resolve_version,
                resolve_version_alias,
            },
            package_metadata::PackageMetadata,
        },
        global::{
            CORE_SHIMS, is_local_package_spec, npm_view, parse_package_spec,
            read_local_package_json,
        },
    },
    error::Error,
};

struct Package<'a> {
    spec: &'a str,
    bin_names: Vec<String>,
    install: Option<InstalledPackage>,
}

struct InstalledPackage {
    installed_version: String,
    bin_names: Vec<String>,
    js_bins: HashSet<String>,
    had_existing_install: bool,
}

fn package_error(package_name: &str, error: impl Into<Error>) -> (Option<String>, Error) {
    (Some(package_name.to_string()), error.into())
}

/// Install global packages parallelly.
///
/// If `node_version` is provided, uses that version. Otherwise, resolves from current directory.
/// If `force` is true, auto-uninstalls conflicting packages.
/// Use `concurrency` to control the number of packages to install in parallel.
pub async fn install(
    package_specs: &[String],
    node_version: Option<&str>,
    force: bool,
    concurrency: usize,
    update: bool,
) -> Result<(), (Option<String>, Error)> {
    if package_specs.is_empty() {
        return Ok(());
    }

    let operation_progress = if update { "Updating" } else { "Installing" };
    let operation_past = if update { "Updated" } else { "Installed" };

    // 1. Resolve Node.js version
    let node_version = if let Some(v) = node_version {
        let provider = NodeProvider::new();
        match resolve_version_alias(v, &provider).await {
            Ok(version) => version,
            Err(error) => return Err((None, error)),
        }
    } else {
        // Resolve from current directory
        let cwd = match current_dir() {
            Ok(cwd) => cwd,
            Err(error) => {
                let error =
                    Error::ConfigError(format!("Cannot get current directory: {}", error).into());
                return Err((None, error));
            }
        };
        let resolution = match resolve_version(&cwd).await {
            Ok(resolution) => resolution,
            Err(error) => return Err((None, error)),
        };
        resolution.version
    };

    // 2. Ensure Node.js is installed
    let runtime = match vite_js_runtime::download_runtime(
        vite_js_runtime::JsRuntimeType::Node,
        &node_version,
    )
    .await
    {
        Ok(runtime) => runtime,
        Err(error) => {
            let error = Error::RuntimeDownload(error);
            return Err((None, error));
        }
    };

    let node_bin_dir = runtime.get_bin_prefix();
    let npm_path =
        if cfg!(windows) { node_bin_dir.join("npm.cmd") } else { node_bin_dir.join("npm") };

    // 3. Install packages in parallel
    let mut packages = IndexMap::<String, Package>::new();
    for package_spec in package_specs {
        // Parse package spec (e.g., "typescript", "typescript@5.0.0", "@scope/pkg")

        let (package_name, _version_spec) = match parse_package_spec(package_spec) {
            Ok(result) => result,
            Err(error) => return Err((Some(package_spec.clone()), error)),
        };
        let bin_names =
            match resolve_package_bin_names(package_spec, &npm_path, &node_bin_dir).await {
                Ok(bin_names) => bin_names,
                Err(error) => return Err((Some(package_name), error)),
            };
        packages.insert(package_name, Package { spec: package_spec, bin_names, install: None });
    }
    let packages_count = packages.len();

    if let Err(error) = resolve_preinstall_conflicts(&packages, force).await {
        return Err(error);
    }

    let concurrency = concurrency.max(1);
    output::info(&format!(
        "{} {} global {} with Node.js {}",
        operation_progress,
        packages_count,
        if packages_count == 1 { "package" } else { "packages" },
        node_version
    ));

    let progress = ProgressBar::new(packages_count as u64);
    if std::io::stderr().is_terminal() && std::env::var_os("CI").is_none() {
        let style = ProgressStyle::with_template("{spinner:.cyan} {msg} ({pos}/{len})")
            .unwrap_or_else(|_| ProgressStyle::default_spinner())
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]);
        progress.set_style(style);
        progress.set_message(format!("{} global packages", operation_progress));
        progress.enable_steady_tick(Duration::from_millis(80));
    } else {
        progress.set_draw_target(ProgressDrawTarget::hidden());
    }

    // We have to clone it because we will modify `packages` to storage package names
    let package_names = packages.keys().cloned().collect::<Vec<_>>();
    let mut package_names = package_names.iter();

    let mut installs = FuturesUnordered::new();
    let mut first_error = None;
    let mut stop_scheduling = false;
    loop {
        while !stop_scheduling && installs.len() < concurrency {
            let Some(package_name) = package_names.next() else { break };
            let package = packages.get(package_name).unwrap();
            let package_name = package_name.clone();
            let package_spec = package.spec.to_string();
            let bin_names = package.bin_names.clone();
            let npm_path = &npm_path;
            let node_bin_dir = &node_bin_dir;

            installs.push(async move {
                let install =
                    install_one(&package_name, &package_spec, &bin_names, npm_path, node_bin_dir)
                        .await;
                (package_name, install)
            });
        }

        if installs.is_empty() {
            break;
        }

        match installs.next().await {
            Some((package_name, Ok(installed_package))) => {
                progress.inc(1);
                packages.get_mut(&package_name).unwrap().install = Some(installed_package)
            }
            Some((package_name, Err(error))) => {
                stop_scheduling = true;
                if first_error.is_none() {
                    first_error = Some((Some(package_name), error));
                }
            }
            None => break,
        }
    }
    progress.finish_and_clear();

    // 4. Finalize installed packages.
    let mut bin_owners = HashMap::<String, String>::new();
    for (index, (package_name, Package { spec: _, bin_names: _, install })) in
        packages.into_iter().enumerate()
    {
        let Some(InstalledPackage { installed_version, bin_names, js_bins, had_existing_install }) =
            install
        else {
            continue;
        };
        let stale_bin_names = match stale_bin_names_for_package(&package_name, &bin_names).await {
            Ok(bin_names) => bin_names,
            Err(error) => {
                let _ = cleanup_failed_install(&package_name, !had_existing_install).await;
                if first_error.is_none() {
                    first_error = Some(package_error(&package_name, error));
                }
                continue;
            }
        };

        let mut finalize_blocked = false;

        // 4.1 Recheck binary ownership in case another process changed metadata after preflight.
        for bin_name in &bin_names {
            if let Some(owner) = bin_owners.get(bin_name)
                && owner != &package_name
            {
                if first_error.is_none() {
                    first_error = Some((
                        Some(package_name.clone()),
                        Error::BinaryConflict {
                            bin_name: bin_name.clone(),
                            existing_package: owner.clone(),
                            new_package: package_name.clone(),
                        },
                    ));
                }
                let _ = cleanup_failed_install(&package_name, !had_existing_install).await;
                finalize_blocked = true;
                continue;
            }

            match BinConfig::load(bin_name).await {
                Ok(Some(config)) => {
                    if config.package != package_name {
                        if first_error.is_none() {
                            first_error = Some((
                                Some(package_name.clone()),
                                Error::BinaryConflict {
                                    bin_name: bin_name.clone(),
                                    existing_package: config.package.clone(),
                                    new_package: package_name.clone(),
                                },
                            ));
                        }
                        let _ = cleanup_failed_install(&package_name, !had_existing_install).await;
                        finalize_blocked = true;
                        break;
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    let _ = cleanup_failed_install(&package_name, !had_existing_install).await;
                    if first_error.is_none() {
                        first_error = Some(package_error(&package_name, error));
                    }
                    finalize_blocked = true;
                    break;
                }
            }
        }
        if finalize_blocked {
            continue;
        }

        // 4.2 Persist package-level metadata for uninstall, list, and dispatch.
        let bin_dir = match get_bin_dir().map_err(|error| package_error(&package_name, error)) {
            Ok(bin_dir) => bin_dir,
            Err(error) => {
                let _ = cleanup_failed_install(&package_name, !had_existing_install).await;
                if first_error.is_none() {
                    first_error = Some(error);
                }
                continue;
            }
        };

        let metadata = PackageMetadata::new(
            package_name.clone(),
            installed_version.clone(),
            node_version.clone(),
            None,
            bin_names.clone(),
            js_bins,
            "npm".to_string(),
        );
        if let Err(error) =
            metadata.save().await.map_err(|error| package_error(&package_name, error))
        {
            let _ = cleanup_failed_install(&package_name, !had_existing_install).await;
            if first_error.is_none() {
                first_error = Some(error);
            }
            continue;
        }

        // 4.3 Expose each binary by creating shims and per-binary ownership config.
        let mut finalized = true;
        for bin_name in &bin_names {
            if let Err(error) = create_package_shim(&bin_dir, bin_name, &package_name)
                .await
                .map_err(|error| package_error(&package_name, error))
            {
                finalized = false;
                if first_error.is_none() {
                    first_error = Some(error);
                }
                break;
            }

            let bin_config = BinConfig::new(
                bin_name.clone(),
                package_name.clone(),
                installed_version.clone(),
                node_version.clone(),
            );
            if let Err(error) =
                bin_config.save().await.map_err(|error| package_error(&package_name, error))
            {
                finalized = false;
                if first_error.is_none() {
                    first_error = Some(error);
                }
                break;
            }
            bin_owners.insert(bin_name.clone(), package_name.clone());
        }

        if !finalized {
            let _ = cleanup_failed_install(&package_name, !had_existing_install).await;
            continue;
        }

        // 4.4 Remove shims for binaries the package used to expose but no longer declares.
        for bin_name in stale_bin_names {
            let result = async {
                remove_package_shim(&bin_dir, &bin_name).await?;
                BinConfig::delete(&bin_name).await?;
                Ok::<(), Error>(())
            }
            .await;

            if let Err(error) = result.map_err(|error| package_error(&package_name, error)) {
                let _ = cleanup_failed_install(&package_name, !had_existing_install).await;
                if first_error.is_none() {
                    first_error = Some(error);
                }
                finalized = false;
                break;
            }
        }

        if !finalized {
            continue;
        }

        // 4.5 Print success message
        output::success(&format!(
            "{} {} {}{}",
            operation_past,
            package_name.bold(),
            if update { "to " } else { "" },
            installed_version.bold()
        ));
        if !bin_names.is_empty() {
            let bins = bin_names
                .iter()
                .map(|bin_name| bin_name.bold().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            output::raw(&format!("  Bins: {}", bins));
        }
        if index + 1 < packages_count {
            output::raw("");
        }
    }

    if let Some(error) = first_error { Err(error) } else { Ok(()) }
}

/// Install one package into its final prefix.
async fn install_one(
    package_name: &str,
    package_spec: &str,
    preflight_bin_names: &[String],
    npm_path: &AbsolutePathBuf,
    node_bin_dir: &AbsolutePathBuf,
) -> Result<InstalledPackage, Error> {
    // 1. Create package directory. npm owns replacement/recovery for existing contents.
    let packages_dir = get_packages_dir()?;
    let package_dir = packages_dir.join(package_name);
    let had_existing_install = PackageMetadata::load(package_name).await?.is_some();
    tokio::fs::create_dir_all(&package_dir).await?;

    // 2. Run npm install with prefix set to the final package directory
    //    Pipe stdout/stderr so npm output is hidden on success, shown on failure
    let output = Command::new(npm_path.as_path())
        .args(["install", "-g", "--no-fund", &package_spec])
        .env("npm_config_prefix", package_dir.as_path())
        .env("PATH", format_path_prepended(node_bin_dir.as_path()))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .output()
        .await?;

    if !output.status.success() {
        // Show captured output to help debug the failure
        let _ = std::io::stdout().write_all(&output.stdout);
        let _ = std::io::stderr().write_all(&output.stderr);
        cleanup_failed_install(package_name, !had_existing_install).await?;
        return Err(Error::ConfigError(
            format!("npm install failed with exit code: {:?}", output.status.code()).into(),
        ));
    }

    let node_modules_dir = get_node_modules_dir(&package_dir, package_name);
    let package_json_path = node_modules_dir.join("package.json");

    if !tokio::fs::try_exists(&package_json_path).await.unwrap_or(false) {
        cleanup_failed_install(package_name, !had_existing_install).await?;
        return Err(Error::ConfigError(
            format!(
                "Package was not installed correctly, package.json not found at {}",
                package_json_path.as_path().display()
            )
            .into(),
        ));
    }

    let package_json_content = match tokio::fs::read_to_string(&package_json_path).await {
        Ok(content) => content,
        Err(error) => {
            cleanup_failed_install(package_name, !had_existing_install).await?;
            return Err(error.into());
        }
    };
    let package_json: serde_json::Value = match serde_json::from_str(&package_json_content) {
        Ok(package_json) => package_json,
        Err(error) => {
            cleanup_failed_install(package_name, !had_existing_install).await?;
            return Err(Error::ConfigError(
                format!("Failed to parse package.json: {error}").into(),
            ));
        }
    };

    let installed_version = package_json["version"].as_str().unwrap_or("unknown").to_string();
    let binary_infos = extract_binaries(&package_json);

    let bin_names = if preflight_bin_names.is_empty() {
        binary_infos.iter().map(|info| info.name.clone()).collect()
    } else {
        preflight_bin_names.to_vec()
    };
    let mut js_bins = HashSet::new();
    for info in binary_infos {
        if !bin_names.contains(&info.name) {
            continue;
        }
        let binary_path = node_modules_dir.join(&info.path);
        if is_javascript_binary(&binary_path) {
            js_bins.insert(info.name);
        }
    }

    Ok(InstalledPackage { installed_version, bin_names, js_bins, had_existing_install })
}

async fn resolve_package_bin_names(
    package_spec: &str,
    npm_path: &AbsolutePathBuf,
    node_bin_dir: &AbsolutePathBuf,
) -> Result<Vec<String>, Error> {
    let (package_name, _) = parse_package_spec(package_spec)?;
    let package_json = if is_local_package_spec(package_spec) {
        Some(read_local_package_json(package_spec)?)
    } else {
        None
    };

    match package_json {
        Some(package_json) => {
            Ok(extract_binaries(&package_json).into_iter().map(|bin| bin.name).collect())
        }
        None => parse_npm_view_bin(
            &package_name,
            &npm_view(npm_path, node_bin_dir, package_spec, "bin").await?,
        ),
    }
}

fn parse_npm_view_bin(package_name: &str, stdout: &[u8]) -> Result<Vec<String>, Error> {
    let raw = String::from_utf8_lossy(stdout);
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let value: serde_json::Value = serde_json::from_str(trimmed)?;
    let default_bin_name = package_name.split('/').last().unwrap_or(package_name).to_string();
    Ok(match value {
        serde_json::Value::String(_) => vec![default_bin_name],
        serde_json::Value::Object(map) => map.keys().cloned().collect(),
        serde_json::Value::Array(values) => values
            .iter()
            .rev()
            .find_map(|value| match value {
                serde_json::Value::Object(map) => Some(map.keys().cloned().collect()),
                serde_json::Value::String(_) => Some(vec![default_bin_name.clone()]),
                _ => None,
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    })
}

async fn resolve_preinstall_conflicts(
    packages: &IndexMap<String, Package<'_>>,
    force: bool,
) -> Result<(), (Option<String>, Error)> {
    let mut bin_owners = HashMap::<String, String>::new();
    let mut conflicts = Vec::<(String, String, String)>::new();

    for (package_name, package) in packages {
        for bin_name in &package.bin_names {
            if let Some(owner) = bin_owners.get(bin_name)
                && owner != package_name
            {
                conflicts.push((bin_name.clone(), owner.clone(), package_name.clone()));
                continue;
            }
            bin_owners.insert(bin_name.clone(), package_name.clone());

            match BinConfig::load(bin_name).await {
                Ok(Some(config)) if config.package != *package_name => {
                    conflicts.push((bin_name.clone(), config.package, package_name.clone()));
                }
                Ok(_) => {}
                Err(error) => return Err(package_error(package_name, error)),
            }
        }
    }

    if conflicts.is_empty() {
        return Ok(());
    }

    if !force {
        let (bin_name, existing_package, new_package) = &conflicts[0];
        return Err((
            Some(new_package.clone()),
            Error::BinaryConflict {
                bin_name: bin_name.clone(),
                existing_package: existing_package.clone(),
                new_package: new_package.clone(),
            },
        ));
    }

    let requested_packages = packages.keys().cloned().collect::<HashSet<_>>();
    let packages_to_remove = conflicts
        .into_iter()
        .filter_map(|(_, existing_package, new_package)| {
            if requested_packages.contains(&existing_package) {
                None
            } else {
                Some((existing_package, new_package))
            }
        })
        .collect::<HashSet<_>>();

    for (existing_package, new_package) in packages_to_remove {
        output::raw(&format!(
            "Uninstalling {} (conflicts with {})...",
            existing_package, new_package
        ));
        if let Err(error) = Box::pin(uninstall(&existing_package, false)).await {
            return Err(package_error(&new_package, error));
        }
    }

    Ok(())
}

async fn cleanup_failed_install(package_name: &str, remove_package: bool) -> Result<(), Error> {
    if remove_package {
        cleanup_installed_package(package_name).await?;
    }
    Ok(())
}

async fn cleanup_installed_package(package_name: &str) -> Result<(), Error> {
    let bin_dir = get_bin_dir()?;
    if let Some(metadata) = PackageMetadata::load(package_name).await? {
        for bin_name in metadata.bins {
            remove_package_shim(&bin_dir, &bin_name).await?;
            BinConfig::delete(&bin_name).await?;
        }
    }

    for bin_name in BinConfig::find_by_package(package_name).await? {
        remove_package_shim(&bin_dir, &bin_name).await?;
        BinConfig::delete(&bin_name).await?;
    }

    let packages_dir = get_packages_dir()?;
    let package_dir = packages_dir.join(package_name);
    if tokio::fs::try_exists(&package_dir).await.unwrap_or(false) {
        tokio::fs::remove_dir_all(&package_dir).await?;
    }
    PackageMetadata::delete(package_name).await?;

    Ok(())
}

async fn stale_bin_names_for_package(
    package_name: &str,
    current_bin_names: &[String],
) -> Result<Vec<String>, Error> {
    let current_bin_names: HashSet<_> = current_bin_names.iter().cloned().collect();
    let mut previous_bin_names = HashSet::new();

    if let Some(metadata) = PackageMetadata::load(package_name).await? {
        previous_bin_names.extend(metadata.bins);
    }

    previous_bin_names.extend(BinConfig::find_by_package(package_name).await?);
    previous_bin_names.retain(|bin_name| !current_bin_names.contains(bin_name));

    Ok(previous_bin_names.into_iter().collect())
}

/// Uninstall a global package.
///
/// Uses two-phase uninstall:
/// 1. Try to use PackageMetadata for binary list
/// 2. Fallback to scanning BinConfig files for orphaned binaries
pub async fn uninstall(package_name: &str, dry_run: bool) -> Result<(), Error> {
    if is_local_package_spec(package_name) {
        // We can't resolve local packages for uninstall, follow npm's behavior
        return Err(Error::ConfigError(
            format!(
                "Local path {} can't be resolved, please enter a package name instead",
                package_name
            )
            .into(),
        ));
    }

    let (package_name, _) = parse_package_spec(package_name).unwrap();

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
        use crate::commands::env::{
            cleanup_legacy_windows_shim, get_trampoline_path, remove_or_rename_to_old,
        };

        let shim_path = bin_dir.join(format!("{}.exe", bin_name));

        // Delete before overwrite; falls back to rename if the exe is locked.
        remove_or_rename_to_old(&shim_path).await;

        // Copy the trampoline binary as <bin_name>.exe.
        // The trampoline detects the tool name from its own filename and sets
        // VP_SHIM_TOOL env var before spawning vp.exe.
        let trampoline_src = get_trampoline_path()?;
        tokio::fs::copy(trampoline_src.as_path(), &shim_path).await?;

        // Remove legacy .cmd and shell script wrappers from previous versions.
        // In Git Bash/MSYS, the extensionless script takes precedence over .exe,
        // so leftover wrappers would bypass the trampoline.
        cleanup_legacy_windows_shim(bin_dir, bin_name).await;

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
    use crate::commands::global::is_local_package_spec;

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
    fn test_is_local_package_spec_relative_paths() {
        assert!(is_local_package_spec("."));
        assert!(is_local_package_spec(".."));
        assert!(is_local_package_spec("./pkg"));
        assert!(is_local_package_spec("../pkg"));
        assert!(is_local_package_spec("file:../pkg"));
    }

    #[test]
    fn test_is_local_package_spec_registry_packages() {
        assert!(!is_local_package_spec("typescript"));
        assert!(!is_local_package_spec("typescript@5.9.3"));
        assert!(!is_local_package_spec("@scope/pkg"));
        assert!(!is_local_package_spec("@scope/pkg@1.0.0"));
    }

    #[test]
    fn test_parse_package_spec_simple() {
        let (name, version) = parse_package_spec("typescript").unwrap();
        assert_eq!(name, "typescript");
        assert_eq!(version, None);
    }

    #[test]
    fn test_parse_package_spec_with_version() {
        let (name, version) = parse_package_spec("typescript@5.0.0").unwrap();
        assert_eq!(name, "typescript");
        assert_eq!(version, Some("5.0.0".to_string()));
    }

    #[test]
    fn test_parse_package_spec_scoped() {
        let (name, version) = parse_package_spec("@types/node").unwrap();
        assert_eq!(name, "@types/node");
        assert_eq!(version, None);
    }

    #[test]
    fn test_parse_package_spec_scoped_with_version() {
        let (name, version) = parse_package_spec("@types/node@20.0.0").unwrap();
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
