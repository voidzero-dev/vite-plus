//! Global package installation handling.

use std::{
    collections::{HashMap, HashSet},
    fs::{File, OpenOptions, TryLockError},
    io::{IsTerminal, Read, Write},
    process::Stdio,
    time::Duration,
};

use futures::{StreamExt, stream::FuturesUnordered};
use indexmap::IndexMap;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use owo_colors::OwoColorize;
use tokio::process::Command;
use uuid::Uuid;
use vite_js_runtime::NodeProvider;
use vite_path::{AbsolutePath, AbsolutePathBuf, current_dir};
use vite_shared::{format_path_prepended, output};

#[cfg(test)]
use crate::commands::env::package_metadata::INSTALL_ID_LENGTH;
use crate::{
    commands::{
        env::{
            bin_config::BinConfig,
            config::{get_bin_dir, get_node_modules_dir, resolve_version, resolve_version_alias},
            package_metadata::{INSTALL_ID_PREFIX, PackageMetadata, is_install_id},
        },
        global::{CORE_SHIMS, is_local_package_spec, parse_package_spec},
    },
    error::Error,
};

struct Package<'a> {
    spec: &'a str,
    install: Option<InstalledPackage>,
}

struct InstalledPackage {
    installed_version: String,
    bin_names: Vec<String>,
    js_bins: HashSet<String>,
    install_id: String,
    install_dir: AbsolutePathBuf,
}

fn package_error(package_name: &str, error: impl Into<Error>) -> (Option<String>, Error) {
    (Some(package_name.to_string()), error.into())
}

/// Symlink target used for package shims on Unix (relative to the bin dir).
#[cfg(unix)]
pub(crate) const PACKAGE_SHIM_TARGET: &str = "../current/bin/vp";

/// Check whether a bin symlink target points at the vp binary: the standard
/// relative package-shim target, or a resolvable link to a binary named `vp`
/// (absolute paths in external/dev layouts created by `vp env setup`).
#[cfg(unix)]
pub(crate) fn is_vp_shim_target(
    target: &std::path::Path,
    shim_path: &vite_path::AbsolutePath,
) -> bool {
    target == std::path::Path::new(PACKAGE_SHIM_TARGET)
        || (target.file_name().is_some_and(|file_name| file_name == "vp")
            && std::fs::exists(shim_path.as_path()).unwrap_or(false))
}

/// Check whether a binary name is a shim Vite+ owns unconditionally: core
/// shims plus the default env shims (node, npm, npx, corepack, vpx, vpr).
/// Protected shims are never removed on behalf of packages, and are never
/// created for packages either, with one exception: `vp install -g corepack`
/// may take BinConfig ownership of the corepack shim (see
/// `create_package_shim`).
pub(crate) fn is_protected_shim(bin_name: &str, ignore_case: bool) -> bool {
    let bin_name =
        if cfg!(target_os = "linux") || !ignore_case { bin_name } else { &bin_name.to_lowercase() };
    CORE_SHIMS.contains(&bin_name) || crate::commands::env::setup::SHIM_TOOLS.contains(&bin_name)
}

/// Whether a package may own a bin name. Protected shim names never belong
/// to packages, with one exception: the `corepack` package owning its own
/// `corepack` bin, so an explicit `vp install -g corepack` wins the shim's
/// resolution order. The exemption is scoped to the package name; any other
/// package declaring a `corepack` bin must not take BinConfig ownership.
pub(crate) fn package_may_own_bin(package_name: &str, bin_name: &str) -> bool {
    !is_protected_shim(bin_name, true) || (bin_name == "corepack" && package_name == "corepack")
}

/// Options for [`install`].
pub struct InstallOptions<'a> {
    /// Node.js version to install with; resolved from the current directory
    /// when `None`.
    pub node_version: Option<&'a str>,
    /// Auto-uninstall packages whose binaries conflict.
    pub force: bool,
    /// Number of packages to install in parallel.
    pub concurrency: usize,
    /// `vp update -g` semantics: carries a recorded bin restriction forward.
    pub update: bool,
    /// Only expose these binaries as shims; other bins the package declares
    /// are ignored (used by the corepack shim auto-install, which must not
    /// link corepack's pnpm/yarn launchers).
    pub only_bins: Option<&'a [&'a str]>,
}

/// Install global packages parallelly.
pub async fn install(
    package_specs: &[String],
    options: InstallOptions<'_>,
) -> Result<(), (Option<String>, Error)> {
    let InstallOptions { node_version, force, concurrency, update, only_bins } = options;
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
            Err(error) => return Err((None, error.into())),
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
        packages.insert(package_name, Package { spec: package_spec, install: None });
    }
    let packages_count = packages.len();

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
    let mut install_locks = HashMap::<String, File>::new();
    let mut first_error = None;
    let mut stop_scheduling = false;
    loop {
        while !stop_scheduling && installs.len() < concurrency {
            let Some(package_name) = package_names.next() else { break };
            let package = packages.get(package_name).unwrap();

            installs.push(async {
                (
                    package_name.clone(),
                    install_one(package_name, package.spec, &npm_path, &node_bin_dir).await,
                )
            });
        }

        if installs.is_empty() {
            break;
        }

        match installs.next().await {
            Some((package_name, Ok((installed_package, lock_file)))) => {
                progress.inc(1);
                packages.get_mut(&package_name).unwrap().install = Some(installed_package);
                install_locks.insert(package_name, lock_file);
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
    for (index, (package_name, Package { spec: _, install })) in packages.into_iter().enumerate() {
        let lock_file = install_locks.remove(&package_name);
        let Some(InstalledPackage {
            installed_version,
            mut bin_names,
            mut js_bins,
            install_id,
            install_dir,
        }) = install
        else {
            continue;
        };

        // Previous metadata drives both the inherited bin restriction and
        // stale-bin detection below; load it once.
        let previous_metadata = match PackageMetadata::load(&package_name).await {
            Ok(metadata) => metadata,
            Err(error) => {
                let _ = cleanup_failed_install(&install_dir).await;
                if first_error.is_none() {
                    first_error = Some(package_error(&package_name, error));
                }
                continue;
            }
        };

        // Restrict exposed binaries when requested (e.g., the corepack shim
        // auto-install only links `corepack`, not the pnpm/yarn launchers
        // that `corepack enable` creates on demand). Updates carry a recorded
        // restriction forward so `vp update -g` cannot re-expose the filtered
        // bins; explicit installs (update=false) re-expose the full bin list.
        let restriction: Option<Vec<String>> = match only_bins {
            Some(only) => Some(only.iter().map(ToString::to_string).collect()),
            None if update => previous_metadata
                .as_ref()
                .filter(|previous| previous.bins_restricted)
                .map(|previous| previous.bins.clone()),
            None => None,
        };
        let bins_restricted = restriction.is_some();
        if let Some(only) = &restriction {
            bin_names.retain(|bin| only.contains(bin));
            js_bins.retain(|bin| only.contains(bin));
        }

        // Drop bin names the package must not own before conflict detection,
        // shim creation, BinConfig ownership, and metadata recording.
        bin_names.retain(|bin| {
            let allowed = package_may_own_bin(&package_name, bin);
            if !allowed {
                output::warn(&format!(
                    "Package '{}' provides '{}' binary, but it conflicts with a built-in shim. \
                     Skipping.",
                    package_name, bin
                ));
            }
            allowed
        });
        js_bins.retain(|bin| package_may_own_bin(&package_name, bin));

        let stale_bin_names = match stale_bin_names_for_package(
            previous_metadata.as_ref(),
            &package_name,
            &bin_names,
        )
        .await
        {
            Ok(bin_names) => bin_names,
            Err(error) => {
                let _ = cleanup_failed_install(&install_dir).await;
                if first_error.is_none() {
                    first_error = Some(package_error(&package_name, error));
                }
                continue;
            }
        };

        let mut conflicts = Vec::<(String, String)>::new();
        let mut finalize_blocked = false;

        // 4.1 Detect binary ownership conflicts before writing metadata.
        for bin_name in &bin_names {
            if let Some(owner) = bin_owners.get(bin_name)
                && owner != &package_name
            {
                conflicts.push((bin_name.clone(), owner.clone()));
                continue;
            }

            match BinConfig::load(bin_name).await {
                Ok(Some(config)) => {
                    if config.package != package_name {
                        conflicts.push((bin_name.clone(), config.package.clone()));
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    let _ = cleanup_failed_install(&install_dir).await;
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

        // 4.2 Resolve conflicts, either by force-uninstalling owners or rolling back this install.
        if !conflicts.is_empty() {
            if force {
                let packages_to_remove: HashSet<_> =
                    conflicts.iter().map(|(_, pkg)| pkg.clone()).collect();
                let mut uninstall_failed = false;
                for pkg in packages_to_remove {
                    output::raw(&format!(
                        "Uninstalling {} (conflicts with {})...",
                        pkg, package_name
                    ));
                    if let Err(error) = Box::pin(uninstall(&pkg, false)).await {
                        let _ = cleanup_failed_install(&install_dir).await;
                        if first_error.is_none() {
                            first_error = Some(package_error(&package_name, error));
                        }
                        uninstall_failed = true;
                        break;
                    }
                }
                if uninstall_failed {
                    continue;
                }
            } else {
                let _ = cleanup_failed_install(&install_dir).await;
                if first_error.is_none() {
                    first_error = Some((
                        Some(package_name.clone()),
                        Error::BinaryConflict {
                            bin_name: conflicts[0].0.clone(),
                            existing_package: conflicts[0].1.clone(),
                            new_package: package_name.clone(),
                        },
                    ));
                }
                continue;
            }
        }

        // 4.3 Prepare metadata and remove binaries that the new install no longer provides.
        let bin_dir = match get_bin_dir().map_err(|error| package_error(&package_name, error)) {
            Ok(bin_dir) => bin_dir,
            Err(error) => {
                let _ = cleanup_failed_install(&install_dir).await;
                if first_error.is_none() {
                    first_error = Some(error);
                }
                continue;
            }
        };

        let mut metadata = PackageMetadata::new(
            package_name.clone(),
            installed_version.clone(),
            node_version.clone(),
            None,
            bin_names.clone(),
            js_bins,
            "npm".to_string(),
        );
        metadata.install_id = install_id.clone();
        metadata.bins_restricted = bins_restricted;

        let mut finalized = true;
        for bin_name in &stale_bin_names {
            let result = async {
                remove_package_shim(&bin_dir, bin_name).await?;
                BinConfig::delete(bin_name).await?;
                Ok::<(), Error>(())
            }
            .await;

            if let Err(error) = result.map_err(|error| package_error(&package_name, error)) {
                restore_previous_install_state(
                    &bin_dir,
                    &package_name,
                    previous_metadata.as_ref(),
                    &bin_names,
                )
                .await;
                let _ = cleanup_failed_install(&install_dir).await;
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

        // 4.4 Activate the new installation through metadata.
        if let Err(error) =
            metadata.save().await.map_err(|error| package_error(&package_name, error))
        {
            restore_previous_install_state(
                &bin_dir,
                &package_name,
                previous_metadata.as_ref(),
                &bin_names,
            )
            .await;
            let _ = cleanup_failed_install(&install_dir).await;
            if first_error.is_none() {
                first_error = Some(error);
            }
            continue;
        }

        // 4.5 Expose each binary and record its ownership.
        for bin_name in &bin_names {
            let result = async {
                create_package_shim(&bin_dir, bin_name, &package_name).await?;
                BinConfig::new(
                    bin_name.clone(),
                    package_name.clone(),
                    installed_version.clone(),
                    node_version.clone(),
                )
                .save()
                .await
            }
            .await;

            if let Err(error) = result.map_err(|error| package_error(&package_name, error)) {
                restore_previous_install_state(
                    &bin_dir,
                    &package_name,
                    previous_metadata.as_ref(),
                    &bin_names,
                )
                .await;
                let _ = cleanup_failed_install(&install_dir).await;
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

        for bin_name in &bin_names {
            bin_owners.insert(bin_name.clone(), package_name.clone());
        }

        // 4.6 Remove stale installations for this package.
        cleanup_stale_installations(&package_name, &install_id).await;
        drop(lock_file);

        // 4.7 Print success message
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

/// Install one package into a unique final prefix.
async fn install_one(
    package_name: &str,
    package_spec: &str,
    npm_path: &AbsolutePathBuf,
    node_bin_dir: &AbsolutePathBuf,
) -> Result<(InstalledPackage, File), Error> {
    // 1. Create an immutable install directory.
    let install_id = new_install_id();
    let install_dir = PackageMetadata::installation_dir_for(package_name, &install_id)?;
    let lock_file = lock_install_dir(&install_dir)?;
    tokio::fs::create_dir_all(&install_dir).await?;

    // 2. Run npm install with prefix set to the final installation directory.
    //    Pipe stdout/stderr so npm output is hidden on success, shown on failure
    let output = Command::new(npm_path.as_path())
        .args(["install", "-g", "--no-fund", &package_spec])
        .env("npm_config_prefix", install_dir.as_path())
        .env("PATH", format_path_prepended(node_bin_dir.as_path()))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .output()
        .await?;

    if !output.status.success() {
        // Show captured output to help debug the failure. npm's stdout joins
        // stderr when vp's stdout must stay parseable (shim dispatch).
        if output::user_output_to_stderr() {
            let _ = std::io::stderr().write_all(&output.stdout);
        } else {
            let _ = std::io::stdout().write_all(&output.stdout);
        }
        let _ = std::io::stderr().write_all(&output.stderr);
        cleanup_failed_install(&install_dir).await?;
        return Err(Error::Other(format!("npm install failed with {}", output.status).into()));
    }

    let node_modules_dir = get_node_modules_dir(&install_dir, package_name);
    let package_json_path = node_modules_dir.join("package.json");

    if !tokio::fs::try_exists(&package_json_path).await.unwrap_or(false) {
        cleanup_failed_install(&install_dir).await?;
        return Err(Error::Other(
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
            cleanup_failed_install(&install_dir).await?;
            return Err(error.into());
        }
    };
    let package_json: serde_json::Value = match serde_json::from_str(&package_json_content) {
        Ok(package_json) => package_json,
        Err(error) => {
            cleanup_failed_install(&install_dir).await?;
            return Err(Error::Other(format!("Failed to parse package.json: {error}").into()));
        }
    };

    let installed_version = package_json["version"].as_str().unwrap_or("unknown").to_string();
    let binary_infos = extract_binaries(&package_json);

    let mut bin_names = Vec::new();
    let mut js_bins = HashSet::new();
    for info in &binary_infos {
        bin_names.push(info.name.clone());
        let binary_path = node_modules_dir.join(&info.path);
        if is_javascript_binary(&binary_path) {
            js_bins.insert(info.name.clone());
        }
    }

    Ok((
        InstalledPackage { installed_version, bin_names, js_bins, install_id, install_dir },
        lock_file,
    ))
}

fn new_install_id() -> String {
    format!("{INSTALL_ID_PREFIX}{}", Uuid::new_v4())
}

async fn restore_package_metadata(package_name: &str, previous_metadata: Option<&PackageMetadata>) {
    let result = match previous_metadata {
        Some(metadata) => metadata.save().await,
        None => PackageMetadata::delete(package_name).await,
    };
    if let Err(error) = result {
        tracing::warn!("Failed to restore global package metadata for {package_name}: {error}");
    }
}

async fn restore_previous_install_state(
    bin_dir: &AbsolutePath,
    package_name: &str,
    previous_metadata: Option<&PackageMetadata>,
    current_bin_names: &[String],
) {
    restore_package_metadata(package_name, previous_metadata).await;

    let previous_bin_names = previous_metadata
        .map(|metadata| metadata.bins.iter().cloned().collect::<HashSet<_>>())
        .unwrap_or_default();
    let bin_names =
        current_bin_names.iter().chain(previous_bin_names.iter()).cloned().collect::<HashSet<_>>();

    for bin_name in bin_names {
        let result = if previous_bin_names.contains(&bin_name) {
            async {
                create_package_shim(bin_dir, &bin_name, package_name).await?;
                if let Some(metadata) = previous_metadata {
                    BinConfig::new(
                        bin_name.clone(),
                        package_name.to_string(),
                        metadata.version.clone(),
                        metadata.platform.node.clone(),
                    )
                    .save()
                    .await
                } else {
                    Ok(())
                }
            }
            .await
        } else {
            async {
                remove_package_shim(bin_dir, &bin_name).await?;
                BinConfig::delete(&bin_name).await
            }
            .await
        };

        if let Err(error) = result {
            tracing::warn!(
                "Failed to restore '{}' binary state for global package '{}': {}",
                bin_name,
                package_name,
                error
            );
        }
    }
}

async fn cleanup_failed_install(install_dir: &AbsolutePathBuf) -> Result<(), Error> {
    remove_dir_all_if_exists(install_dir).await
}

async fn remove_dir_all_if_exists(path: &AbsolutePathBuf) -> Result<(), Error> {
    match tokio::fs::remove_dir_all(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn install_dir_lock_path(install_dir: &AbsolutePathBuf) -> Result<AbsolutePathBuf, Error> {
    let parent = install_dir.as_path().parent().ok_or_else(|| {
        Error::ConfigError(
            format!(
                "Global package installation path has no parent: {}",
                install_dir.as_path().display()
            )
            .into(),
        )
    })?;
    let file_name =
        install_dir.as_path().file_name().and_then(|name| name.to_str()).ok_or_else(|| {
            Error::ConfigError(
                format!(
                    "Global package installation path has no file name: {}",
                    install_dir.as_path().display()
                )
                .into(),
            )
        })?;
    AbsolutePathBuf::new(parent.join(format!("{file_name}.lock"))).ok_or_else(|| {
        Error::ConfigError(
            format!(
                "Invalid global package installation lock path for {}",
                install_dir.as_path().display()
            )
            .into(),
        )
    })
}

fn open_install_dir_lock_file(
    install_dir: &AbsolutePathBuf,
) -> Result<(AbsolutePathBuf, File), Error> {
    let lock_path = install_dir_lock_path(install_dir)?;
    if let Some(parent) = lock_path.as_path().parent() {
        std::fs::create_dir_all(parent)?;
    }

    let lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path.as_path())?;

    Ok((lock_path, lock_file))
}

fn lock_install_dir(install_dir: &AbsolutePathBuf) -> Result<File, Error> {
    let (_, lock_file) = open_install_dir_lock_file(install_dir)?;
    lock_file.lock()?;
    Ok(lock_file)
}

fn try_lock_install_dir(
    install_dir: &AbsolutePathBuf,
) -> Result<Option<(AbsolutePathBuf, File)>, Error> {
    let (lock_path, lock_file) = open_install_dir_lock_file(install_dir)?;
    match lock_file.try_lock() {
        Ok(()) => Ok(Some((lock_path, lock_file))),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(error)) => Err(error.into()),
    }
}

async fn cleanup_stale_installations(package_name: &str, current_install_id: &str) {
    let Ok(stale_dirs) = stale_installation_dirs(package_name, current_install_id).await else {
        return;
    };

    for install_dir in stale_dirs {
        let (lock_path, lock_file) = match try_lock_install_dir(&install_dir) {
            Ok(Some(lock)) => lock,
            Ok(None) => continue,
            Err(error) => {
                tracing::warn!(
                    "Failed to lock stale global package installation at {}: {}",
                    install_dir.as_path().display(),
                    error
                );
                continue;
            }
        };

        if let Err(error) = remove_dir_all_if_exists(&install_dir).await {
            tracing::warn!(
                "Failed to remove stale global package installation at {}: {}",
                install_dir.as_path().display(),
                error
            );
            continue;
        }

        drop(lock_file);
        if let Err(error) = tokio::fs::remove_file(&lock_path).await {
            tracing::warn!(
                "Failed to remove stale global package installation lock at {}: {}",
                lock_path.as_path().display(),
                error
            );
        }
    }
}

async fn stale_installation_dirs(
    package_name: &str,
    current_install_id: &str,
) -> Result<Vec<AbsolutePathBuf>, Error> {
    let legacy_install_dir = PackageMetadata::installation_dir_for(package_name, "")?;
    let Some(parent) = legacy_install_dir.as_path().parent() else {
        return Ok(Vec::new());
    };
    if !tokio::fs::try_exists(parent).await.unwrap_or(false) {
        return Ok(Vec::new());
    }

    let Some(base_name) = legacy_install_dir.as_path().file_name().and_then(|name| name.to_str())
    else {
        return Ok(Vec::new());
    };

    let mut stale_dirs = Vec::new();
    let mut entries = tokio::fs::read_dir(parent).await?;
    while let Some(entry) = entries.next_entry().await? {
        if !entry.file_type().await?.is_dir() {
            continue;
        }

        let file_name = entry.file_name();
        let Some(file_name) = file_name.to_str() else {
            continue;
        };
        let Some(install_id) = stale_install_id(base_name, file_name) else {
            continue;
        };
        if install_id == current_install_id {
            continue;
        }
        if let Some(path) = AbsolutePathBuf::new(entry.path()) {
            stale_dirs.push(path);
        }
    }

    Ok(stale_dirs)
}

fn stale_install_id<'a>(base_name: &str, file_name: &'a str) -> Option<&'a str> {
    if file_name == base_name {
        return Some("");
    }

    let install_id = file_name.strip_prefix(base_name)?;
    if is_install_id(install_id) { Some(install_id) } else { None }
}

async fn stale_bin_names_for_package(
    previous_metadata: Option<&PackageMetadata>,
    package_name: &str,
    current_bin_names: &[String],
) -> Result<Vec<String>, Error> {
    let current_bin_names: HashSet<_> = current_bin_names.iter().cloned().collect();
    let mut previous_bin_names = HashSet::new();

    if let Some(metadata) = previous_metadata {
        previous_bin_names.extend(metadata.bins.iter().cloned());
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
        return Err(Error::Other(
            format!(
                "Local path {} can't be resolved, please enter a package name instead",
                package_name
            )
            .into(),
        ));
    }

    let (package_name, _) = parse_package_spec(package_name).unwrap();

    // Phase 1: Try to use PackageMetadata for binary list and installation path.
    let metadata = PackageMetadata::load(&package_name).await?;
    let bins = if let Some(metadata) = &metadata {
        metadata.bins.clone()
    } else {
        // Phase 2: Fallback - scan BinConfig files for orphaned binaries
        let orphan_bins = BinConfig::find_by_package(&package_name).await?;
        if orphan_bins.is_empty() {
            return Err(Error::Other(format!("Package {} is not installed", package_name).into()));
        }
        orphan_bins
    };

    if dry_run {
        let bin_dir = get_bin_dir()?;
        let package_dir = match &metadata {
            Some(metadata) => metadata.installation_dir()?,
            None => PackageMetadata::installation_dir_for(&package_name, "")?,
        };
        let metadata_path = PackageMetadata::metadata_path(&package_name)?;

        output::raw(&format!("Would uninstall {}:", package_name));
        for bin_name in &bins {
            // Protected shims survive the real uninstall; keep dry-run honest.
            if is_protected_shim(bin_name, false) {
                output::raw(&format!(
                    "  - shim: {} (kept: default shim)",
                    bin_dir.join(bin_name).as_path().display()
                ));
            } else {
                output::raw(&format!("  - shim: {}", bin_dir.join(bin_name).as_path().display()));
            }
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
    let package_dir = match &metadata {
        Some(metadata) => metadata.installation_dir()?,
        None => PackageMetadata::installation_dir_for(&package_name, "")?,
    };
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
pub(crate) async fn create_package_shim(
    bin_dir: &vite_path::AbsolutePath,
    bin_name: &str,
    package_name: &str,
) -> Result<(), Error> {
    // Defense in depth: the finalize loop already filters bin names the
    // package must not own (see package_may_own_bin); keep the guard here so
    // no other caller can hand a protected shim to a package.
    if !package_may_own_bin(package_name, bin_name) {
        output::warn(&format!(
            "Package '{}' provides '{}' binary, but it conflicts with a built-in shim. Skipping.",
            package_name, bin_name
        ));
        return Ok(());
    }

    // Ensure bin directory exists
    tokio::fs::create_dir_all(bin_dir).await?;

    #[cfg(unix)]
    {
        let shim_path = bin_dir.join(bin_name);

        // Keep an existing Vite+ shim: replacing an external/dev-layout link
        // with the relative target would dangle when VP_HOME/current is absent.
        if let Ok(target) = tokio::fs::read_link(&shim_path).await {
            if is_vp_shim_target(&target, &shim_path) {
                return Ok(());
            }
            // Exists but points elsewhere (e.g., npm-installed direct symlink) — replace it
            tokio::fs::remove_file(&shim_path).await?;
        }

        // Create symlink to ../current/bin/vp
        tokio::fs::symlink(PACKAGE_SHIM_TARGET, &shim_path).await?;
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
    // Don't remove protected shims (e.g., `vp remove -g corepack` must keep
    // the default corepack shim so it falls back to the Node-bundled or
    // auto-installed corepack).
    if is_protected_shim(bin_name, false) {
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

    #[test]
    fn test_package_may_own_bin_scopes_corepack_to_its_package() {
        // Only the corepack package may own the corepack bin; any other
        // package declaring a `corepack` bin must not take BinConfig
        // ownership (it would win the corepack shim's resolution order).
        assert!(package_may_own_bin("corepack", "corepack"));
        assert!(!package_may_own_bin("some-package", "corepack"));
        assert!(!package_may_own_bin("@scope/corepack", "corepack"));

        // Other protected shims never belong to packages
        assert!(!package_may_own_bin("corepack", "npm"));
        assert!(!package_may_own_bin("some-package", "vpx"));
        assert!(!package_may_own_bin("some-package", "vpr"));

        // Regular bins are unrestricted
        assert!(package_may_own_bin("typescript", "tsc"));

        #[cfg(any(windows, target_os = "macos"))]
        assert!(!package_may_own_bin("some-package", "NPM"));
        #[cfg(any(windows, target_os = "macos"))]
        assert!(!package_may_own_bin("some-package", "Node"));
        #[cfg(any(windows, target_os = "macos"))]
        assert!(!package_may_own_bin("some-package", "VP"));
    }

    #[tokio::test]
    async fn test_create_package_shim_skips_corepack_bin_for_other_packages() {
        use tempfile::TempDir;
        use vite_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let bin_dir = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        create_package_shim(&bin_dir, "corepack", "some-package").await.unwrap();

        #[cfg(unix)]
        let shim_path = bin_dir.join("corepack");
        #[cfg(windows)]
        let shim_path = bin_dir.join("corepack.exe");
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
        let mut metadata = PackageMetadata::new(
            "typescript".to_string(),
            "5.9.3".to_string(),
            "20.18.0".to_string(),
            None,
            vec!["tsc".to_string(), "tsserver".to_string()],
            HashSet::from(["tsc".to_string(), "tsserver".to_string()]),
            "npm".to_string(),
        );
        metadata.install_id = "#123e4567-e89b-42d3-a456-426614174000".to_string();
        metadata.save().await.unwrap();

        // Create identified package directory (needed for uninstall)
        let package_dir = metadata.installation_dir().unwrap();
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
        assert!(!package_dir.as_path().exists(), "identified package directory should be removed");
    }

    #[tokio::test]
    #[cfg_attr(windows, serial_test::serial)]
    async fn test_restore_previous_install_state_removes_partial_new_bins() {
        use tempfile::TempDir;
        use vite_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();
        #[cfg(windows)]
        let _trampoline_guard = FakeTrampolineGuard::new(&temp_path);
        let _env_guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(&temp_path),
        );
        let bin_dir = AbsolutePathBuf::new(temp_path.join("bin")).unwrap();

        let mut previous_metadata = PackageMetadata::new(
            "test-package".to_string(),
            "1.0.0".to_string(),
            "20.0.0".to_string(),
            None,
            vec!["keep".to_string(), "drop".to_string()],
            HashSet::from(["keep".to_string(), "drop".to_string()]),
            "npm".to_string(),
        );
        previous_metadata.install_id = "#123e4567-e89b-42d3-a456-426614174000".to_string();

        let mut new_metadata = PackageMetadata::new(
            "test-package".to_string(),
            "2.0.0".to_string(),
            "22.0.0".to_string(),
            None,
            vec!["keep".to_string(), "new".to_string()],
            HashSet::from(["keep".to_string(), "new".to_string()]),
            "npm".to_string(),
        );
        new_metadata.install_id = "#987e6543-e21b-42d3-a456-426614174000".to_string();
        new_metadata.save().await.unwrap();

        for bin_name in ["keep", "new"] {
            create_package_shim(&bin_dir, bin_name, "test-package").await.unwrap();
            BinConfig::new(
                bin_name.to_string(),
                "test-package".to_string(),
                "2.0.0".to_string(),
                "22.0.0".to_string(),
            )
            .save()
            .await
            .unwrap();
        }

        restore_previous_install_state(
            &bin_dir,
            "test-package",
            Some(&previous_metadata),
            &new_metadata.bins,
        )
        .await;

        let restored = PackageMetadata::load("test-package").await.unwrap().unwrap();
        assert_eq!(restored.install_id, previous_metadata.install_id);
        assert_eq!(BinConfig::load("keep").await.unwrap().unwrap().version, "1.0.0");
        assert_eq!(BinConfig::load("drop").await.unwrap().unwrap().version, "1.0.0");
        assert!(BinConfig::load("new").await.unwrap().is_none());
        #[cfg(unix)]
        {
            assert!(std::fs::symlink_metadata(bin_dir.join("drop").as_path()).is_ok());
            assert!(std::fs::symlink_metadata(bin_dir.join("new").as_path()).is_err());
        }
        #[cfg(windows)]
        {
            assert!(bin_dir.join("drop.exe").as_path().exists());
            assert!(!bin_dir.join("new.exe").as_path().exists());
        }
    }

    #[test]
    fn test_new_install_id_has_fixed_reserved_shape() {
        let install_id = new_install_id();

        assert!(is_install_id(&install_id));
        assert_eq!(install_id.len(), INSTALL_ID_LENGTH);
        assert!(install_id.starts_with(INSTALL_ID_PREFIX));
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
