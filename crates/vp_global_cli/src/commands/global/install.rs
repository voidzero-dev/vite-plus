//! Global package installation handling.

use std::{
    collections::{HashMap, HashSet},
    fs::{File, OpenOptions, TryLockError},
    io::{Read, Write},
    process::Stdio,
    time::Duration,
};

use console::style;
use futures::{StreamExt, stream::FuturesUnordered};
use indexmap::IndexMap;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use tokio::process::Command;
use uuid::Uuid;
use vp_js_runtime::NodeProvider;
use vp_shared::{PrependOptions, ToolPathEnv, output};
use vt_path::{AbsolutePath, AbsolutePathBuf, current_dir};

use crate::{
    commands::{
        env::{
            bin_config::{BinConfig, validate_bin_name},
            config::{get_bin_dir, get_node_modules_dir, resolve_version, resolve_version_alias},
            package_metadata::{PackageMetadata, is_legacy_install_id, is_nested_install_id},
        },
        global::{
            LEGACY_PACKAGE_MANAGER_PACKAGES, bin_lock, is_local_package_spec, parse_package_spec,
            update_version_spec,
        },
    },
    error::Error,
};

struct Package<'a> {
    spec: &'a str,
    install: Option<InstalledPackage>,
}

struct InstalledPackage {
    installed_version: Option<String>,
    bin_names: Vec<String>,
    js_bins: HashSet<String>,
    install_id: String,
    install_dir: AbsolutePathBuf,
}

type InstallError = (Option<String>, Box<Error>);

fn package_error(package_name: &str, error: impl Into<Error>) -> InstallError {
    (Some(package_name.to_string()), Box::new(error.into()))
}

/// Absolute `vp` path for package shims. The path is
/// `<DATA>/current/bin/vp`, or `vp.exe` on Windows.
pub(crate) fn package_shim_target() -> AbsolutePathBuf {
    vp_shared::EnvConfig::get()
        .dirs
        .data
        .join("current")
        .join("bin")
        .join(vp_shared::VP_BINARY_NAME)
}

/// Whether `shim_path` is a Vite+ shim for this install's `vp`.
///
/// On Unix, accept a symlink to [`package_shim_target`]. Resolve a relative
/// target from the shim parent. Also accept a working link to a `vp` binary for
/// development and `vp env setup` layouts.
///
/// On Windows, accept a symlink to `vp` or `vp.exe`, or accept `vp-use.cmd`.
/// Also accept a trampoline when its `<name>.shim` records this install's data
/// root. A regular file does not prove ownership. A bin from an explicit
/// override group can contain unrelated executables such as `node.exe` and
/// `npm.exe`.
pub(crate) fn is_vp_shim_target(shim_path: &vt_path::AbsolutePath) -> bool {
    match std::fs::read_link(shim_path.as_path()) {
        Ok(target) => {
            if cfg!(windows) {
                target
                    .file_name()
                    .is_some_and(|name| name == vp_shared::VP_BINARY_NAME || name == "vp")
            } else {
                let expected = package_shim_target();
                let resolved = shim_path.parent().map(|parent| parent.join(&target).clean());
                resolved.as_ref().is_some_and(|path| path.as_path() == expected.as_path())
                    || (target.file_name().is_some_and(|name| name == "vp")
                        && std::fs::exists(shim_path.as_path()).unwrap_or(false))
            }
        }
        Err(_) => cfg!(windows) && windows_regular_file_is_vp_shim(shim_path),
    }
}

/// Windows trampoline / `vp-use.cmd` ownership check.
fn windows_regular_file_is_vp_shim(shim_path: &vt_path::AbsolutePath) -> bool {
    if !shim_path.as_path().is_file() {
        return false;
    }
    if shim_path.as_path().file_name().is_some_and(|name| name == "vp-use.cmd") {
        return true;
    }
    vp_shared::EnvConfig::get().dirs.owns_windows_trampoline(shim_path.as_path())
}

/// Check whether a binary name is a shim Vite+ owns unconditionally: core
/// shims plus the default env shims.
/// Protected shims are never removed or created on behalf of packages.
pub(crate) fn is_protected_shim(bin_name: &str, ignore_case: bool) -> bool {
    let bin_name =
        if cfg!(target_os = "linux") || !ignore_case { bin_name } else { &bin_name.to_lowercase() };
    bin_name == "vp" || crate::shim::DEFAULT_SHIM_TOOLS.contains(&bin_name)
}

/// Options for [`install`].
pub struct InstallOptions<'a> {
    /// Node.js version to install with; resolved from the current directory
    /// when `None`.
    pub node_version: Option<&'a str>,
    /// Auto-uninstall packages whose binaries conflict.
    pub force: bool,
    /// Do not run lifecycle scripts.
    pub ignore_scripts: bool,
    /// Number of packages to install in parallel.
    pub concurrency: usize,
    /// Whether this is a `vp update -g` operation.
    pub update: bool,
}

/// Install global packages in parallel.
pub async fn install(
    package_specs: &[String],
    options: InstallOptions<'_>,
) -> Result<(), InstallError> {
    let InstallOptions { node_version, force, ignore_scripts, concurrency, update } = options;
    if package_specs.is_empty() {
        return Ok(());
    }

    let operation_progress = if update { "Updating" } else { "Installing" };
    let operation_past = if update { "Updated" } else { "Installed" };

    // 1. Parse package specs and skip legacy globals now provided by Vite+.
    let mut packages = IndexMap::<String, Package>::new();
    for package_spec in package_specs {
        let package_name = match parse_package_spec(package_spec) {
            Ok((package_name, _)) => package_name,
            Err(error) => return Err((Some(package_spec.clone()), Box::new(error))),
        };
        if package_name == "corepack" {
            return Err(package_error(
                &package_name,
                Error::Other("'vp install -g corepack' is no longer supported.".into()),
            ));
        }
        if LEGACY_PACKAGE_MANAGER_PACKAGES.contains(&package_name.as_str()) {
            output::warn(&format!("Vite+ already includes '{package_name}'; skipping."));
            continue;
        }
        packages.insert(package_name, Package { spec: package_spec, install: None });
    }
    if packages.is_empty() {
        return Ok(());
    }

    // 2. Resolve Node.js version
    let node_version = if let Some(v) = node_version {
        let provider = NodeProvider::new();
        match resolve_version_alias(v, &provider).await {
            Ok(version) => version,
            Err(error) => return Err((None, Box::new(error))),
        }
    } else {
        // Resolve from current directory
        let cwd = match current_dir() {
            Ok(cwd) => cwd,
            Err(error) => return Err((None, Box::new(error.into()))),
        };
        let resolution = match resolve_version(&cwd).await {
            Ok(resolution) => resolution,
            Err(error) => return Err((None, Box::new(error))),
        };
        resolution.version
    };

    // 3. Ensure Node.js is installed
    let runtime =
        match vp_js_runtime::download_runtime(vp_js_runtime::JsRuntimeType::Node, &node_version)
            .await
        {
            Ok(runtime) => runtime,
            Err(error) => {
                let error = Error::RuntimeDownload(error);
                return Err((None, Box::new(error)));
            }
        };

    let node_bin_dir = runtime.get_bin_prefix();
    let npm_path =
        if cfg!(windows) { node_bin_dir.join("npm.cmd") } else { node_bin_dir.join("npm") };

    // 4. Install packages in parallel
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
    if vp_shared::is_stderr_terminal() && std::env::var_os("CI").is_none() {
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
                    install_one(
                        package_name,
                        package.spec,
                        &npm_path,
                        &node_bin_dir,
                        ignore_scripts,
                    )
                    .await,
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
                    first_error = Some((Some(package_name), Box::new(error)));
                }
            }
            None => break,
        }
    }
    progress.finish_and_clear();

    // 5. Finalize installed packages.
    for (index, (package_name, Package { spec, install })) in packages.into_iter().enumerate() {
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

        // Re-read ownership under the shared lock and hold it through rollback and cleanup.
        let activation_lock = match bin_lock::acquire().await {
            Ok(lock) => lock,
            Err(error) => {
                let _ = cleanup_failed_install(&install_dir).await;
                if first_error.is_none() {
                    first_error = Some(package_error(&package_name, error));
                }
                continue;
            }
        };

        // Load previous metadata once for stale-bin detection below.
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

        // Drop bin names the package must not own before conflict detection,
        // shim creation, BinConfig ownership, and metadata recording.
        bin_names.retain(|bin| {
            let allowed = !is_protected_shim(bin, true);
            if !allowed {
                output::warn(&format!(
                    "Package '{}' provides '{}' binary, but it conflicts with a built-in shim. \
                     Skipping.",
                    package_name, bin
                ));
            }
            allowed
        });
        js_bins.retain(|bin| !is_protected_shim(bin, true));

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

        // 5.1 Detect binary ownership conflicts before writing metadata.
        for bin_name in &bin_names {
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

        // 5.2 Resolve conflicts, either by force-uninstalling owners or rolling back this install.
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
                    if let Err(error) = uninstall_locked(&pkg, false).await {
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
                        Box::new(Error::BinaryConflict {
                            bin_name: conflicts[0].0.clone(),
                            existing_package: conflicts[0].1.clone(),
                            new_package: package_name.clone(),
                        }),
                    ));
                }
                continue;
            }
        }

        // 5.3 Prepare metadata and remove binaries that the new install no longer provides.
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
        // Reject foreign files at new shim paths before activating the installation.
        if let Some(path) = bin_names.iter().find_map(|bin_name| foreign_bin(&bin_dir, bin_name)) {
            let _ = cleanup_failed_install(&install_dir).await;
            if first_error.is_none() {
                first_error = Some(package_error(
                    &package_name,
                    Error::Other(
                        format!(
                            "{} already exists and is not managed by Vite+",
                            path.as_path().display()
                        )
                        .into(),
                    ),
                ));
            }
            continue;
        }
        let metadata_version = installed_version.as_deref().unwrap_or("unknown");

        let mut metadata = PackageMetadata::new(
            package_name.clone(),
            metadata_version.to_string(),
            node_version.clone(),
            None,
            bin_names.clone(),
            js_bins,
            "npm".to_string(),
        );
        metadata.install_id = install_id.clone();
        metadata.version_spec = update_version_spec(spec);

        let mut rollback = BinRollback::default();
        let result = async {
            for bin_name in &stale_bin_names {
                rollback.capture_bin(&bin_dir, bin_name).await?;
                remove_package_shim(&bin_dir, bin_name).await?;
                BinConfig::delete(bin_name).await?;
            }

            // 5.4 Activate the new installation through metadata.
            metadata.save().await?;

            // 5.5 Expose each binary and record its ownership.
            for bin_name in &bin_names {
                rollback.capture_bin(&bin_dir, bin_name).await?;
                create_package_shim(&bin_dir, bin_name, &package_name).await?;
                BinConfig::new(
                    bin_name.clone(),
                    package_name.clone(),
                    metadata_version.to_string(),
                    node_version.clone(),
                )
                .save()
                .await?;
            }
            Ok::<(), Error>(())
        }
        .await;

        if let Err(error) = result {
            restore_previous_install_state(&package_name, previous_metadata.as_ref(), rollback)
                .await;
            let _ = cleanup_failed_install(&install_dir).await;
            if first_error.is_none() {
                first_error = Some(package_error(&package_name, error));
            }
            continue;
        }

        // 5.6 Remove stale installations for this package.
        cleanup_stale_installations(&package_name, &install_id).await;
        drop(lock_file);
        drop(activation_lock);

        // 5.7 Print success message
        output::raw(&format!(
            "{} {} {} {}{}",
            style("+").green(),
            operation_past,
            style(&package_name).bold(),
            if update { "to " } else { "" },
            style(installed_version.as_deref().unwrap_or("(no version)")).bold()
        ));
        if !bin_names.is_empty() {
            let bins = bin_names
                .iter()
                .map(|bin_name| style(bin_name).bold().to_string())
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
    ignore_scripts: bool,
) -> Result<(InstalledPackage, File), Error> {
    // 1. Create an immutable install directory.
    let install_id = new_install_id();
    let install_dir = PackageMetadata::installation_dir_for(package_name, &install_id)?;
    let lock_file = lock_install_dir(&install_dir)?;
    tokio::fs::create_dir_all(&install_dir).await?;

    // 2. Run npm install with prefix set to the final installation directory.
    //    Pipe stdout/stderr so npm output is hidden on success, shown on failure
    let mut env = ToolPathEnv::from_env();
    env.prepend(node_bin_dir, &["node", "npm", "npx"], PrependOptions::default())?;
    let mut command = Command::new(npm_path.as_path());
    command.args(["install", "-g", "--no-fund", &package_spec]);
    if ignore_scripts {
        command.arg("--ignore-scripts");
    }
    let output = command
        .env("npm_config_prefix", install_dir.as_path())
        .env("npm_config_update_notifier", "false")
        .envs(env.into_envs())
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

    let installed_version = package_json["version"].as_str().map(ToString::to_string);
    let binary_infos = extract_binaries(&package_json, &node_modules_dir);

    let mut bin_names = Vec::new();
    let mut js_bins = HashSet::new();
    for info in &binary_infos {
        bin_names.push(info.name.clone());
        let binary_path = node_modules_dir.join(&info.path);
        match is_javascript_binary(&binary_path).and_then(|is_js| {
            ensure_executable_bits(&binary_path, &node_modules_dir).map(|()| is_js)
        }) {
            Ok(true) => {
                js_bins.insert(info.name.clone());
            }
            Ok(false) => {}
            Err(error) => {
                cleanup_failed_install(&install_dir).await?;
                return Err(Error::Other(
                    format!(
                        "Failed to inspect binary '{}' at {}: {error}",
                        info.name,
                        binary_path.as_path().display()
                    )
                    .into(),
                ));
            }
        }
    }

    Ok((
        InstalledPackage { installed_version, bin_names, js_bins, install_id, install_dir },
        lock_file,
    ))
}

fn new_install_id() -> String {
    Uuid::new_v4().to_string()
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

/// The regular file at `bin_name`'s shim path when Vite+ does not own it.
fn foreign_bin(bin_dir: &AbsolutePath, bin_name: &str) -> Option<AbsolutePathBuf> {
    if is_protected_shim(bin_name, true) {
        return None;
    }
    let path = if cfg!(windows) {
        bin_dir.join(format!("{bin_name}.exe"))
    } else {
        bin_dir.join(bin_name)
    };
    let metadata = std::fs::symlink_metadata(path.as_path()).ok()?;
    (metadata.is_file() && !is_vp_shim_target(&path)).then_some(path)
}

/// Save only paths an installation is about to change, including their original owners.
#[derive(Default)]
struct BinRollback {
    directory: Option<tempfile::TempDir>,
    files: Vec<(AbsolutePathBuf, Option<AbsolutePathBuf>)>,
}

impl BinRollback {
    async fn capture_bin(&mut self, bin_dir: &AbsolutePath, bin_name: &str) -> Result<(), Error> {
        let config_path = BinConfig::path(bin_name)?;
        let captured_len = self.files.len();
        let result = async {
            #[cfg(unix)]
            let suffixes = [""];
            #[cfg(windows)]
            let suffixes = [".exe", ".shim", ".cmd", ".ps1", ""];
            if !is_protected_shim(bin_name, false) {
                for suffix in suffixes {
                    self.capture(bin_dir.join(format!("{bin_name}{suffix}"))).await?;
                }
            }
            self.capture(config_path).await
        }
        .await;
        if result.is_err() {
            // The caller has not changed these paths when capture fails.
            self.files.truncate(captured_len);
        }
        result
    }

    async fn capture(&mut self, path: AbsolutePathBuf) -> Result<(), Error> {
        if self.files.iter().any(|(captured, _)| *captured == path) {
            return Ok(());
        }
        let backup = match tokio::fs::symlink_metadata(&path).await {
            Ok(metadata) => {
                let directory = match &self.directory {
                    Some(directory) => directory,
                    None => self.directory.insert(tempfile::tempdir()?),
                };
                let backup =
                    AbsolutePathBuf::new(directory.path().join(self.files.len().to_string()))
                        .expect("temporary directory is absolute");
                if metadata.file_type().is_symlink() {
                    let target = tokio::fs::read_link(&path).await?;
                    #[cfg(unix)]
                    tokio::fs::symlink(target, &backup).await?;
                    #[cfg(windows)]
                    tokio::fs::symlink_file(target, &backup).await?;
                } else if metadata.is_file() {
                    tokio::fs::copy(&path, &backup).await?;
                } else {
                    return Err(Error::Other(
                        format!("Cannot replace non-file {}", path.as_path().display()).into(),
                    ));
                }
                Some(backup)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        self.files.push((path, backup));
        Ok(())
    }

    async fn restore(self) {
        let mut failed = false;
        for (path, backup) in self.files.into_iter().rev() {
            let result = async {
                if let Some(backup) = &backup {
                    let metadata = tokio::fs::symlink_metadata(backup).await?;
                    if metadata.file_type().is_symlink()
                        && tokio::fs::read_link(&path).await.ok()
                            == Some(tokio::fs::read_link(backup).await?)
                    {
                        return Ok::<(), std::io::Error>(());
                    }
                }

                #[cfg(windows)]
                {
                    crate::commands::env::remove_or_rename_to_old(&path).await;
                    match tokio::fs::symlink_metadata(&path).await {
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                        Err(error) => return Err(error),
                        Ok(_) => {
                            return Err(std::io::Error::other(
                                "Could not remove replacement binary",
                            ));
                        }
                    }
                }
                #[cfg(unix)]
                match tokio::fs::remove_file(&path).await {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error),
                }
                if let Some(backup) = &backup {
                    if tokio::fs::symlink_metadata(backup).await?.file_type().is_symlink() {
                        let target = tokio::fs::read_link(backup).await?;
                        #[cfg(unix)]
                        tokio::fs::symlink(target, &path).await?;
                        #[cfg(windows)]
                        tokio::fs::symlink_file(target, &path).await?;
                    } else {
                        tokio::fs::copy(backup, &path).await?;
                    }
                }
                Ok(())
            }
            .await;
            if let Err(error) = result {
                failed = true;
                if let Some(backup) = backup {
                    output::warn(&format!(
                        "Failed to restore {} from {}: {error}",
                        path.as_path().display(),
                        backup.as_path().display()
                    ));
                } else {
                    output::warn(&format!(
                        "Failed to remove new binary {}: {error}",
                        path.as_path().display()
                    ));
                }
            }
        }
        if failed && let Some(directory) = self.directory {
            output::warn(&format!(
                "Could not fully restore global binaries. Backups kept at {}",
                directory.keep().display()
            ));
        }
    }
}

async fn restore_previous_install_state(
    package_name: &str,
    previous_metadata: Option<&PackageMetadata>,
    rollback: BinRollback,
) {
    restore_package_metadata(package_name, previous_metadata).await;
    rollback.restore().await;
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

    if !is_nested_install_id(current_install_id) {
        return;
    }

    let result: Result<(), Error> = async {
        let package_dir = PackageMetadata::installation_dir_for(package_name, "")?;
        let package_json = get_node_modules_dir(&package_dir, package_name).join("package.json");
        if !tokio::fs::try_exists(&package_json).await.unwrap_or(false) {
            return Ok(());
        }

        let Some((lock_path, lock_file)) = try_lock_install_dir(&package_dir)? else {
            return Ok(());
        };

        remove_legacy_installation_contents(&package_dir).await?;

        drop(lock_file);
        match tokio::fs::remove_file(&lock_path).await {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
    .await;
    if let Err(error) = result {
        tracing::warn!(
            "Failed to clean legacy global package installation for {package_name}: {error}"
        );
    }
}

async fn remove_legacy_installation_contents(package_dir: &AbsolutePathBuf) -> Result<(), Error> {
    // Preserve nested installs while removing npm contents from the legacy prefix.
    let mut entries = tokio::fs::read_dir(package_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let file_name = entry.file_name();
        let Some(file_name) = file_name.to_str() else { continue };
        if is_nested_install_id(file_name)
            || file_name.strip_suffix(".lock").is_some_and(is_nested_install_id)
        {
            continue;
        }

        if entry.file_type().await?.is_dir() {
            tokio::fs::remove_dir_all(entry.path()).await?;
        } else {
            tokio::fs::remove_file(entry.path()).await?;
        }
    }

    match tokio::fs::remove_dir(package_dir).await {
        Ok(()) => Ok(()),
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
            ) =>
        {
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

async fn stale_installation_dirs(
    package_name: &str,
    current_install_id: &str,
) -> Result<Vec<AbsolutePathBuf>, Error> {
    let package_dir = PackageMetadata::installation_dir_for(package_name, "")?;
    let mut stale_dirs = Vec::new();
    // Current installs are UUID children of the package directory.
    if tokio::fs::try_exists(&package_dir).await.unwrap_or(false) {
        let mut entries = tokio::fs::read_dir(&package_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            if !entry.file_type().await?.is_dir() {
                continue;
            }

            let file_name = entry.file_name();
            let Some(install_id) = file_name.to_str().filter(|name| is_nested_install_id(name))
            else {
                continue;
            };
            if install_id != current_install_id
                && let Some(path) = AbsolutePathBuf::new(entry.path())
            {
                stale_dirs.push(path);
            }
        }
    }

    let Some(parent) = package_dir.as_path().parent() else { return Ok(stale_dirs) };
    let Some(base_name) = package_dir.as_path().file_name().and_then(|name| name.to_str()) else {
        return Ok(stale_dirs);
    };
    if !tokio::fs::try_exists(parent).await.unwrap_or(false) {
        return Ok(stale_dirs);
    }

    // Vite+ 0.2.2 through 0.2.5 stored immutable installs as `<package>#<uuid>` siblings.
    let mut entries = tokio::fs::read_dir(parent).await?;
    while let Some(entry) = entries.next_entry().await? {
        if !entry.file_type().await?.is_dir() {
            continue;
        }

        let file_name = entry.file_name();
        let Some(file_name) = file_name.to_str() else {
            continue;
        };
        let Some(install_id) =
            file_name.strip_prefix(base_name).filter(|install_id| is_legacy_install_id(install_id))
        else {
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
    for bin_name in &previous_bin_names {
        validate_bin_name(bin_name)?;
    }
    previous_bin_names.retain(|bin_name| !current_bin_names.contains(bin_name));

    Ok(previous_bin_names.into_iter().collect())
}

/// Uninstall a global package.
///
/// Uses two-phase uninstall:
/// 1. Try to use PackageMetadata for binary list
/// 2. Fallback to scanning BinConfig files for orphaned binaries
pub async fn uninstall(package_name: &str, dry_run: bool) -> Result<(), Error> {
    let _lock = bin_lock::acquire().await?;
    uninstall_locked(package_name, dry_run).await
}

// Called directly by forced activation, which already holds the global bin lock.
async fn uninstall_locked(package_name: &str, dry_run: bool) -> Result<(), Error> {
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

    for bin_name in &bins {
        validate_bin_name(bin_name)?;
    }

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
        if metadata.as_ref().is_none_or(|metadata| metadata.install_id.is_empty()) {
            remove_legacy_installation_contents(&package_dir).await?;
        } else {
            tokio::fs::remove_dir_all(&package_dir).await?;
        }
    }

    // Remove metadata file
    PackageMetadata::delete(&package_name).await?;

    output::raw(&format!("{} Uninstalled {}", style("-").red(), style(&package_name).bold()));

    Ok(())
}

/// Binary info extracted from package.json.
pub(crate) struct BinaryInfo {
    /// Binary name (the command users will run)
    pub name: String,
    /// Path to the binary file from package root
    pub path: String,
}

/// Extract safe binary names and paths from package.json, skipping invalid entries.
pub(crate) fn extract_binaries(
    package_json: &serde_json::Value,
    package_dir: &AbsolutePath,
) -> Vec<BinaryInfo> {
    let mut bins = Vec::new();

    if let Some(bin) = package_json.get("bin") {
        match bin {
            serde_json::Value::String(path) if !path.is_empty() => {
                if let Some(name) = package_json["name"].as_str() {
                    bins.push(BinaryInfo { name: name.to_string(), path: path.clone() });
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

    let package_root = package_dir.clean();
    bins.retain_mut(|bin| {
        if bin.name.starts_with('@') {
            if let Some((_, name)) = bin.name.split_once('/') {
                bin.name = name.to_string();
            }
        }
        // Like pnpm, check lexical containment without requiring the target to exist.
        validate_bin_name(&bin.name).is_ok()
            && package_dir.join(&bin.path).clean().as_path().starts_with(package_root.as_path())
    });
    // Match npm: later valid entries overwrite earlier targets for the same command.
    bins.into_iter()
        .map(|bin| (bin.name.clone(), bin))
        .collect::<IndexMap<_, _>>()
        .into_values()
        .collect()
}

/// Check if a file is a JavaScript file that should be run with Node.
///
/// Returns true if:
/// - The file has a .js, .mjs, or .cjs extension
/// - The file has a shebang containing "node"
///
/// Reads only the first 256 bytes, tolerating genuinely missing targets.
fn is_javascript_binary(path: &AbsolutePath) -> std::io::Result<bool> {
    // Probe even known extensions so directories and unreadable files fail installation.
    let mut buffer = [0u8; 256];
    let n = match std::fs::File::open(path.as_path()).and_then(|mut file| file.read(&mut buffer)) {
        Ok(n) => n,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // Windows can report NotFound when opening an existing directory with a trailing separator.
            match std::fs::metadata(path.as_path()) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
                Err(error) => return Err(error),
                Ok(metadata) if metadata.is_dir() => {
                    return Err(std::io::ErrorKind::IsADirectory.into());
                }
                Ok(_) => return Err(error),
            }
        }
        Err(error) => return Err(error),
    };

    // Keep extension-based classification for targets not yet created by lifecycle scripts.
    if let Some(ext) = path.as_path().extension() {
        let ext = ext.to_string_lossy().to_lowercase();
        if ext == "js" || ext == "mjs" || ext == "cjs" {
            return Ok(true);
        }
    }

    if buffer[..n].starts_with(b"#!") {
        let end = buffer[..n].iter().position(|&b| b == b'\n').unwrap_or(n);
        if let Ok(shebang) = std::str::from_utf8(&buffer[..end]) {
            return Ok(shebang.contains("node"));
        }
    }

    Ok(false)
}

/// Add missing executable bits to a binary target inside the package, like pnpm.
///
/// npm only fixes the targets selected by its own normalization, so a target Vite+
/// retains instead can stay non-executable. Missing targets are skipped.
fn ensure_executable_bits(path: &AbsolutePath, package_root: &AbsolutePath) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let target = match std::fs::canonicalize(path.as_path()) {
            Ok(target) => target,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        // Never chmod a file that a symlink resolves to outside the package.
        if !target.starts_with(std::fs::canonicalize(package_root.as_path())?) {
            return Ok(());
        }
        let mode = std::fs::metadata(&target)?.permissions().mode();
        // Skip files that are already executable so files owned by other users still work.
        if mode & 0o111 != 0o111 {
            std::fs::set_permissions(&target, std::fs::Permissions::from_mode(mode | 0o111))?;
        }
    }
    #[cfg(not(unix))]
    let _ = (path, package_root);
    Ok(())
}

/// Create a shim for a package binary.
///
/// On Unix: Creates a symlink to `<DATA>/current/bin/vp`
/// On Windows: Creates a trampoline .exe that forwards to vp.exe
pub(crate) async fn create_package_shim(
    bin_dir: &vt_path::AbsolutePath,
    bin_name: &str,
    package_name: &str,
) -> Result<(), Error> {
    validate_bin_name(bin_name)?;
    // Defense in depth: the finalize loop already filters protected bin
    // names; keep the guard here so no other caller can hand one to a package.
    if is_protected_shim(bin_name, true) {
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
        if tokio::fs::read_link(&shim_path).await.is_ok() {
            if is_vp_shim_target(&shim_path) {
                return Ok(());
            }
            // Exists but points elsewhere (e.g., npm-installed direct symlink) — replace it
            tokio::fs::remove_file(&shim_path).await?;
        }

        // Point at the active vp the same way `vp env setup` does.
        let current_exe = std::env::current_exe()?;
        let target = crate::commands::env::setup::resolve_unix_vp_shim_target(&current_exe).await?;
        tokio::fs::symlink(&target, &shim_path).await?;
        tracing::debug!("Created package shim symlink {:?} -> {:?}", shim_path, target);
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
        vp_shared::EnvConfig::get().dirs.write_shim_pointer_beside(shim_path.as_path())?;

        // Remove legacy .cmd and shell script wrappers from previous versions.
        // In Git Bash/MSYS, the extensionless script takes precedence over .exe,
        // so leftover wrappers would bypass the trampoline.
        cleanup_legacy_windows_shim(bin_dir, bin_name).await;

        tracing::debug!("Created package trampoline shim {:?}", shim_path);
    }

    Ok(())
}

/// Remove a shim for a package binary.
async fn remove_package_shim(bin_dir: &vt_path::AbsolutePath, bin_name: &str) -> Result<(), Error> {
    validate_bin_name(bin_name)?;
    // Don't remove protected shims.
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
        for suffix in &[".exe", ".cmd", ".shim", ""] {
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
    use vp_shared::env_vars;

    use super::*;
    use crate::commands::global::is_local_package_spec;

    /// Write a fake `vp-shim.exe` trampoline binary for shim-creation tests.
    /// Pair with a `VP_TRAMPOLINE_PATH` pin via `EnvConfig::with_vars[_async]`.
    fn write_fake_trampoline(dir: &std::path::Path) -> std::path::PathBuf {
        let trampoline = dir.join("vp-shim.exe");
        std::fs::write(&trampoline, b"fake-trampoline").unwrap();
        trampoline
    }

    #[tokio::test]
    async fn test_uninstall_waits_for_global_activation() {
        let temp = tempfile::tempdir().unwrap();
        vp_shared::EnvConfig::with_vars_async([(env_vars::VP_HOME, temp.path())], |_| async {
            let metadata = PackageMetadata::new(
                "test-package".into(),
                "1.0.0".into(),
                "22.18.0".into(),
                None,
                vec![],
                HashSet::new(),
                "npm".into(),
            );
            metadata.save().await.unwrap();
            let bin_dir = get_bin_dir().unwrap();
            std::fs::create_dir_all(&bin_dir).unwrap();
            let lock = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(bin_dir.join(".vp-global-bin.lock"))
                .unwrap();
            lock.lock().unwrap();

            let removal = uninstall("test-package", false);
            tokio::pin!(removal);
            assert!(tokio::time::timeout(Duration::from_millis(150), &mut removal).await.is_err());
            assert!(PackageMetadata::load("test-package").await.unwrap().is_some());
            drop(lock);
            tokio::time::timeout(Duration::from_secs(5), removal).await.unwrap().unwrap();
            assert!(PackageMetadata::load("test-package").await.unwrap().is_none());
        })
        .await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_concurrent_installs_prepare_outside_lock_and_keep_active_installation() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().unwrap();
        vp_shared::EnvConfig::with_vars_async([(env_vars::VP_HOME, temp.path())], |env| async move {
            // A local npm stand-in exercises activation without downloads or lifecycle scripts.
            let node_bin = env.dirs.data.join("js_runtime/node/22.18.0/bin");
            std::fs::create_dir_all(&node_bin).unwrap();
            std::fs::write(node_bin.join("node"), b"").unwrap();
            let npm = node_bin.join("npm");
            std::fs::write(&npm, r#"#!/bin/sh
set -eu
package="$npm_config_prefix/lib/node_modules/test-package"
mkdir -p "$package"
printf '%s' '{"name":"test-package","version":"1.0.0","bin":{"test-cli":"cli"}}' > "$package/package.json"
printf '#!/bin/sh\nprintf working' > "$package/cli"
touch "$npm_config_prefix/prepared"
"#).unwrap();
            std::fs::set_permissions(&npm, std::fs::Permissions::from_mode(0o755)).unwrap();

            let lock = bin_lock::acquire().await.unwrap();
            let specs = vec!["test-package".to_string()];
            let options = || InstallOptions {
                node_version: Some("22.18.0"),
                force: false,
                ignore_scripts: true,
                concurrency: 5,
                update: false,
            };
            let mut installs = Box::pin(async {
                tokio::join!(install(&specs, options()), install(&specs, options()))
            });
            let package_dir = PackageMetadata::installation_dir_for("test-package", "").unwrap();
            let prepared = async {
                loop {
                    let count = std::fs::read_dir(&package_dir).into_iter().flatten().flatten()
                        .filter(|entry| entry.path().join("prepared").exists()).count();
                    if count == 2 {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            };
            tokio::select! {
                _ = &mut installs => panic!("installation finished while activation was locked"),
                result = tokio::time::timeout(Duration::from_secs(10), prepared) => result.unwrap(),
            }
            assert!(tokio::time::timeout(Duration::from_millis(150), &mut installs).await.is_err());
            assert!(PackageMetadata::load("test-package").await.unwrap().is_none());
            drop(lock);
            let (first, second) = tokio::time::timeout(Duration::from_secs(10), installs).await.unwrap();
            first.unwrap();
            second.unwrap();
            let active = PackageMetadata::load("test-package").await.unwrap().unwrap();
            let installation = active.installation_dir().unwrap();
            let binary = get_node_modules_dir(&installation, "test-package").join("cli");
            let output = Command::new(binary.as_path()).output().await.unwrap();
            assert!(output.status.success());
            assert_eq!(output.stdout, b"working");
            let installations: Vec<_> = std::fs::read_dir(&package_dir).unwrap()
                .map(Result::unwrap).filter(|entry| entry.file_type().unwrap().is_dir()).collect();
            assert_eq!(installations.len(), 1);
            assert_eq!(installations[0].path(), installation.as_path());
        }).await;
    }

    #[test]
    fn test_default_shims_are_protected() {
        for shim in crate::shim::DEFAULT_SHIM_TOOLS.iter().chain([&"vp"]) {
            assert!(is_protected_shim(shim, false), "{shim} should be protected");
        }
    }

    #[test]
    fn test_extract_binaries_filters_unsafe_entries() {
        let temp = tempfile::tempdir().unwrap();
        let package_dir = AbsolutePathBuf::new(temp.path().join("package")).unwrap();
        for manifest in [
            serde_json::json!({ "name": "my-tool", "bin": "./dist/cli.js" }),
            serde_json::json!({ "name": "@scope/my-tool", "bin": "./dist/cli.js" }),
            serde_json::json!({ "bin": { "my-tool": "./dist/cli.js" } }),
            serde_json::json!({ "bin": { "@scope/my-tool": "./dist/cli.js" } }),
        ] {
            let bins = extract_binaries(&manifest, &package_dir);
            assert_eq!(bins.len(), 1);
            assert_eq!(bins[0].name, "my-tool");
            assert_eq!(bins[0].path, "./dist/cli.js");
        }
        for manifest in [
            serde_json::json!({ "bin": { "../../outside/victim": "./cli.js" } }),
            serde_json::json!({ "bin": { "@scope/../../victim": "./cli.js" } }),
            serde_json::json!({ "bin": { "my-tool": "../outside/cli.js" } }),
            serde_json::json!({ "name": "my-tool", "bin": "../outside/cli.js" }),
            serde_json::json!({ "name": "my-tool", "bin": "" }),
            serde_json::json!({ "name": "..", "bin": "./cli.js" }),
            serde_json::json!({ "name": r"..\outside", "bin": "./cli.js" }),
            serde_json::json!({ "name": "../my-tool", "bin": "./cli.js" }),
            serde_json::json!({ "bin": { "my-tool": temp.path().join("outside/cli.js") } }),
            serde_json::json!({ "bin": { "my-tool": "dist/../../outside/cli.js" } }),
            serde_json::json!({ "bin": { "my-tool": "../package-sibling/cli.js" } }),
            serde_json::json!({ "name": "my-tool" }),
            serde_json::json!({ "bin": 42 }),
            serde_json::json!({ "bin": { "my-tool": 42 } }),
        ] {
            assert!(extract_binaries(&manifest, &package_dir).is_empty(), "{manifest}");
        }
        let manifest = serde_json::json!({ "bin": {
            "my-tool": "./dist/cli.js",
            "./node": "./dist/cli.js",
            "outside": "../outside/cli.js",
            "with space": "./dist/cli.js",
        } });
        let bins = extract_binaries(&manifest, &package_dir);
        assert_eq!(bins.len(), 1);
        assert_eq!(bins[0].name, "my-tool");
        for path in [
            "dist/../cli.js".to_string(),
            "../package/cli.js".to_string(),
            package_dir.join("cli.js").as_path().display().to_string(),
        ] {
            let manifest = serde_json::json!({"bin": {"my-tool": path}});
            let bins = extract_binaries(&manifest, &package_dir);
            assert_eq!(bins.len(), 1);
            assert_eq!(bins[0].path, path);
        }
        #[cfg(windows)]
        for path in [r"..\outside\cli.js", r"C:\outside\cli.js", r"\\server\share\cli.js"] {
            let manifest = serde_json::json!({"bin": {"my-tool": path}});
            assert!(extract_binaries(&manifest, &package_dir).is_empty(), "{manifest}");
        }
    }

    #[test]
    fn test_extract_binaries_keeps_last_valid_normalized_name() {
        let temp = tempfile::tempdir().unwrap();
        let package_dir = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        for (first_name, last_name) in
            [("@z/foo", "@a/foo"), ("foo", "@scope/foo"), ("@scope/foo", "foo")]
        {
            for (first, last, expected) in [
                ("./native-tool", "./cli.js", "./cli.js"),
                ("./cli.js", "./native-tool", "./native-tool"),
                ("../outside", "./cli.js", "./cli.js"),
                ("./cli.js", "../outside", "./cli.js"),
            ] {
                let manifest = serde_json::json!({ "bin": {
                    (first_name): first,
                    "@b/other": "./other.js",
                    (last_name): last,
                } });
                let bins = extract_binaries(&manifest, &package_dir);
                assert_eq!(bins.len(), 2);
                let foo = bins.iter().find(|bin| bin.name == "foo").unwrap();
                assert_eq!(foo.path, expected);
                assert!(bins.iter().any(|bin| bin.name == "other" && bin.path == "./other.js"));
            }
        }
    }

    #[tokio::test]
    async fn test_invalid_binary_name_in_metadata_preserves_existing_install() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        vp_shared::EnvConfig::with_vars_async([(env_vars::VP_HOME, temp_dir.path())], |_| async {
            let metadata = PackageMetadata::new(
                "test-package".to_string(),
                "1.0.0".to_string(),
                "22.18.0".to_string(),
                None,
                vec!["valid-command".to_string(), "../outside".to_string()],
                HashSet::new(),
                "npm".to_string(),
            );
            metadata.save().await.unwrap();
            let bin_dir = get_bin_dir().unwrap();
            std::fs::create_dir_all(&bin_dir).unwrap();
            std::fs::write(bin_dir.join("valid-command"), b"keep command").unwrap();
            let outside = temp_dir.path().join("outside");
            std::fs::write(&outside, b"keep outside").unwrap();

            assert!(
                stale_bin_names_for_package(Some(&metadata), "test-package", &[]).await.is_err()
            );
            assert!(uninstall("test-package", false).await.is_err());
            assert_eq!(std::fs::read(bin_dir.join("valid-command")).unwrap(), b"keep command");
            assert_eq!(std::fs::read(&outside).unwrap(), b"keep outside");
            assert!(PackageMetadata::load("test-package").await.unwrap().is_some());
        })
        .await;
    }

    #[tokio::test]
    async fn test_invalid_binary_name_cannot_create_capture_or_remove_shim() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        vp_shared::EnvConfig::with_vars_async([(env_vars::VP_HOME, temp_dir.path())], |_| async {
            let bin_dir = get_bin_dir().unwrap();
            tokio::fs::create_dir_all(&bin_dir).await.unwrap();
            let outside = temp_dir.path().join("outside");
            std::fs::write(&outside, b"keep me").unwrap();
            assert!(create_package_shim(&bin_dir, "../new-command", "test-package").await.is_err());
            assert!(std::fs::symlink_metadata(temp_dir.path().join("new-command")).is_err());
            assert!(std::fs::symlink_metadata(temp_dir.path().join("new-command.exe")).is_err());

            let mut rollback = BinRollback::default();
            assert!(rollback.capture_bin(&bin_dir, "../outside").await.is_err());
            assert!(rollback.files.is_empty());
            assert!(remove_package_shim(&bin_dir, "../outside").await.is_err());
            assert_eq!(std::fs::read(&outside).unwrap(), b"keep me");
        })
        .await;
    }

    #[tokio::test]
    async fn test_create_package_shim_creates_bin_dir() {
        use tempfile::TempDir;
        use vt_path::AbsolutePathBuf;

        // Create a temp directory but don't create the bin subdirectory
        let temp_dir = TempDir::new().unwrap();
        let trampoline = write_fake_trampoline(temp_dir.path());
        let bin_dir = temp_dir.path().join("bin");
        let bin_dir = AbsolutePathBuf::new(bin_dir).unwrap();

        vp_shared::EnvConfig::with_vars_async(
            [(env_vars::VP_TRAMPOLINE_PATH, trampoline.as_os_str())],
            |_| async {
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
                    let pointer = bin_dir.join("test-shim.shim");
                    assert!(pointer.as_path().exists(), "per-exe sidecar must be written");
                    let contents = std::fs::read_to_string(pointer.as_path()).unwrap();
                    let dirs = &vp_shared::EnvConfig::get().dirs;
                    assert_eq!(
                        contents,
                        format!(
                            "{}\nlayout={}\ndata={}\ncache={}\nbin={}\n",
                            vp_shared::SHIM_POINTER_HEADER,
                            dirs.layout().as_str(),
                            dirs.data.as_path().display(),
                            dirs.cache.as_path().display(),
                            dirs.bin.as_path().display()
                        )
                    );
                }
            },
        )
        .await;
    }

    #[cfg(windows)]
    #[test]
    fn is_vp_shim_target_requires_sidecar_for_windows_exes() {
        vp_shared::EnvConfig::scoped(|config| {
            let bin = &config.dirs.bin;
            std::fs::create_dir_all(bin).unwrap();
            std::fs::write(bin.join("node.exe").as_path(), b"system-node").unwrap();
            assert!(
                !is_vp_shim_target(&bin.join("node.exe")),
                "unrelated node.exe without a sidecar must not be treated as ours"
            );

            config.dirs.write_shim_pointer("node").unwrap();
            assert!(
                is_vp_shim_target(&bin.join("node.exe")),
                "trampoline with this install's sidecar is owned"
            );

            std::fs::write(bin.join("vp-use.cmd").as_path(), b"@echo off").unwrap();
            assert!(is_vp_shim_target(&bin.join("vp-use.cmd")));
        });
    }

    #[tokio::test]
    async fn test_create_package_shim_skips_core_shims() {
        use tempfile::TempDir;
        use vt_path::AbsolutePathBuf;

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
    async fn test_remove_package_shim_removes_shim() {
        use tempfile::TempDir;
        use vt_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let trampoline = write_fake_trampoline(temp_dir.path());
        let bin_dir = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        vp_shared::EnvConfig::with_vars_async(
            [(env_vars::VP_TRAMPOLINE_PATH, trampoline.as_os_str())],
            |_| async {
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
                    assert!(
                        !bin_dir.join("tsc.shim").as_path().exists(),
                        "per-exe sidecar should be removed"
                    );
                }
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_remove_package_shim_handles_missing_shim() {
        use tempfile::TempDir;
        use vt_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let bin_dir = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Remove a shim that doesn't exist - should not error
        remove_package_shim(&bin_dir, "nonexistent").await.unwrap();
    }

    #[tokio::test]
    async fn test_uninstall_removes_shims_from_metadata() {
        use tempfile::TempDir;
        use vt_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();
        let trampoline = write_fake_trampoline(&temp_path);
        vp_shared::EnvConfig::with_vars_async(
            [
                (env_vars::VP_HOME, temp_path.as_os_str()),
                (env_vars::VP_TRAMPOLINE_PATH, trampoline.as_os_str()),
            ],
            |_| async {
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
                    assert!(
                        bin_dir.join("tsc.exe").as_path().exists(),
                        "tsc.exe shim should exist"
                    );
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
                    assert!(
                        !bin_dir.join("tsc.exe").as_path().exists(),
                        "tsc.exe shim should be removed"
                    );
                    assert!(
                        !bin_dir.join("tsserver.exe").as_path().exists(),
                        "tsserver.exe shim should be removed"
                    );
                }
                assert!(
                    !package_dir.as_path().exists(),
                    "identified package directory should be removed"
                );
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_uninstall_legacy_install_preserves_in_progress_nested_install() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        vp_shared::EnvConfig::with_vars_async([(env_vars::VP_HOME, temp_dir.path())], |_| async {
            let package_name = "test-package";

            let metadata = PackageMetadata::new(
                package_name.to_string(),
                "1.0.0".to_string(),
                "20.0.0".to_string(),
                None,
                Vec::new(),
                HashSet::new(),
                "npm".to_string(),
            );
            metadata.save().await.unwrap();

            let legacy_dir = metadata.installation_dir().unwrap();
            let legacy_package_json =
                get_node_modules_dir(&legacy_dir, package_name).join("package.json");
            tokio::fs::create_dir_all(legacy_package_json.parent().unwrap()).await.unwrap();
            tokio::fs::write(&legacy_package_json, "{}").await.unwrap();

            // Model uninstall starting after npm populated the replacement but before metadata changed.
            let replacement_dir = PackageMetadata::installation_dir_for(
                package_name,
                "123e4567-e89b-42d3-a456-426614174000",
            )
            .unwrap();
            let replacement_package_json =
                get_node_modules_dir(&replacement_dir, package_name).join("package.json");
            let replacement_lock = lock_install_dir(&replacement_dir).unwrap();
            tokio::fs::create_dir_all(replacement_package_json.parent().unwrap()).await.unwrap();
            tokio::fs::write(&replacement_package_json, "{}").await.unwrap();

            uninstall(package_name, false).await.unwrap();

            assert!(!legacy_package_json.as_path().exists());
            assert!(replacement_package_json.as_path().exists());
            assert!(install_dir_lock_path(&replacement_dir).unwrap().as_path().exists());
            drop(replacement_lock);
        })
        .await;
    }

    #[tokio::test]
    async fn test_restore_previous_install_state_removes_partial_new_bins() {
        use tempfile::TempDir;
        use vt_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();
        let trampoline = write_fake_trampoline(&temp_path);
        vp_shared::EnvConfig::with_vars_async(
            [
                (env_vars::VP_HOME, temp_path.as_os_str()),
                (env_vars::VP_TRAMPOLINE_PATH, trampoline.as_os_str()),
            ],
            |_| async {
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
                previous_metadata.save().await.unwrap();
                for bin_name in &previous_metadata.bins {
                    create_package_shim(&bin_dir, bin_name, "test-package").await.unwrap();
                    BinConfig::new(
                        bin_name.clone(),
                        "test-package".into(),
                        "1.0.0".into(),
                        "20.0.0".into(),
                    )
                    .save()
                    .await
                    .unwrap();
                }
                let mut rollback = BinRollback::default();
                rollback.capture_bin(&bin_dir, "drop").await.unwrap();
                remove_package_shim(&bin_dir, "drop").await.unwrap();
                BinConfig::delete("drop").await.unwrap();

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
                    rollback.capture_bin(&bin_dir, bin_name).await.unwrap();
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

                restore_previous_install_state("test-package", Some(&previous_metadata), rollback)
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
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_rollback_preserves_protected_shims_and_restores_bin_config() {
        let temp = tempfile::TempDir::new().unwrap();
        vp_shared::EnvConfig::with_vars_async([(env_vars::VP_HOME, temp.path())], |_| async {
            let bin_dir = get_bin_dir().unwrap();
            std::fs::create_dir_all(&bin_dir).unwrap();
            #[cfg(unix)]
            let suffixes = [""];
            #[cfg(windows)]
            let suffixes = [".exe", ".shim", ".cmd", ".ps1", ""];
            for suffix in suffixes {
                let path = bin_dir.join(format!("node{suffix}"));
                std::fs::write(&path, b"protected shim").unwrap();
                std::fs::hard_link(&path, temp.path().join(format!("original-node{suffix}")))
                    .unwrap();
            }
            BinConfig::new(
                "node".into(),
                "legacy-package".into(),
                "1.0.0".into(),
                "22.18.0".into(),
            )
            .save()
            .await
            .unwrap();
            let config_path = BinConfig::path("node").unwrap();
            let original_config = std::fs::read(&config_path).unwrap();
            let stale_bins =
                stale_bin_names_for_package(None, "legacy-package", &[]).await.unwrap();
            assert_eq!(stale_bins, ["node"]);

            let mut rollback = BinRollback::default();
            for bin_name in stale_bins {
                rollback.capture_bin(&bin_dir, &bin_name).await.unwrap();
                remove_package_shim(&bin_dir, &bin_name).await.unwrap();
                BinConfig::delete(&bin_name).await.unwrap();
            }
            std::fs::create_dir(bin_dir.join(format!("blocked{}", std::env::consts::EXE_SUFFIX)))
                .unwrap();
            assert!(rollback.capture_bin(&bin_dir, "blocked").await.is_err());
            rollback.restore().await;

            assert_eq!(std::fs::read(&config_path).unwrap(), original_config);
            for suffix in suffixes {
                let path = bin_dir.join(format!("node{suffix}"));
                assert_eq!(std::fs::read(&path).unwrap(), b"protected shim");
                assert!(
                    same_file::is_same_file(
                        &path,
                        temp.path().join(format!("original-node{suffix}"))
                    )
                    .unwrap()
                );
            }
        })
        .await;
    }

    #[tokio::test]
    async fn test_failed_capture_preserves_untouched_files_and_prior_backups() {
        let temp = tempfile::TempDir::new().unwrap();
        vp_shared::EnvConfig::with_vars_async([(env_vars::VP_HOME, temp.path())], |_| async {
            let bin_dir = get_bin_dir().unwrap();
            std::fs::create_dir_all(&bin_dir).unwrap();
            let changed = bin_dir.join(format!("changed{}", std::env::consts::EXE_SUFFIX));
            std::fs::write(&changed, b"original contents").unwrap();
            BinConfig::new("changed".into(), "package".into(), "1.0.0".into(), "22.18.0".into())
                .save()
                .await
                .unwrap();
            let config_path = BinConfig::path("changed").unwrap();
            let original_config = std::fs::read(&config_path).unwrap();
            let mut rollback = BinRollback::default();
            rollback.capture_bin(&bin_dir, "changed").await.unwrap();
            std::fs::write(&changed, b"replacement contents").unwrap();
            BinConfig::delete("changed").await.unwrap();

            let untouched = bin_dir.join(format!("untouched{}", std::env::consts::EXE_SUFFIX));
            std::fs::write(&untouched, b"untouched contents").unwrap();
            let original = temp.path().join("original-untouched");
            std::fs::hard_link(&untouched, &original).unwrap();
            std::fs::create_dir(BinConfig::path("untouched").unwrap()).unwrap();
            assert!(rollback.capture_bin(&bin_dir, "untouched").await.is_err());

            rollback.restore().await;

            assert_eq!(std::fs::read(&changed).unwrap(), b"original contents");
            assert_eq!(std::fs::read(&config_path).unwrap(), original_config);
            assert_eq!(std::fs::read(&untouched).unwrap(), b"untouched contents");
            assert!(same_file::is_same_file(&untouched, &original).unwrap());
        })
        .await;
    }

    #[tokio::test]
    async fn test_rollback_restores_regular_wrappers_and_sidecars() {
        let temp = tempfile::TempDir::new().unwrap();
        let bin_dir = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let mut rollback = BinRollback::default();
        for suffix in [".exe", ".shim", ".cmd", ".ps1", ""] {
            let path = bin_dir.join(format!("tool{suffix}"));
            std::fs::write(&path, b"original contents").unwrap();
            rollback.capture(path.clone()).await.unwrap();
            std::fs::write(&path, b"replacement contents").unwrap();
            rollback.capture(path.clone()).await.unwrap();
            std::fs::remove_file(&path).unwrap();
        }
        rollback.restore().await;
        for suffix in [".exe", ".shim", ".cmd", ".ps1", ""] {
            assert_eq!(
                std::fs::read(bin_dir.join(format!("tool{suffix}"))).unwrap(),
                b"original contents"
            );
        }
    }

    #[tokio::test]
    async fn test_rollback_restores_changed_regular_files() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = AbsolutePathBuf::new(temp.path().join("tool")).unwrap();
        std::fs::write(&path, b"original contents").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let permissions = std::fs::metadata(&path).unwrap().permissions();
        let mut rollback = BinRollback::default();
        rollback.capture(path.clone()).await.unwrap();
        std::fs::write(&path, b"replacement contents").unwrap();

        rollback.restore().await;

        assert_eq!(std::fs::read(&path).unwrap(), b"original contents");
        assert_eq!(std::fs::metadata(&path).unwrap().permissions(), permissions);
    }

    #[tokio::test]
    async fn test_rollback_keeps_backup_when_restoration_fails() {
        let temp = tempfile::TempDir::new().unwrap();
        let parent = temp.path().join("parent");
        std::fs::create_dir(&parent).unwrap();
        let path = AbsolutePathBuf::new(parent.join("tool")).unwrap();
        std::fs::write(&path, b"original contents").unwrap();
        let mut rollback = BinRollback::default();
        rollback.capture(path.clone()).await.unwrap();
        let backup = rollback.files[0].1.clone().unwrap();
        let backup_dir = rollback.directory.as_ref().unwrap().path().to_path_buf();
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir(&parent).unwrap();
        std::fs::write(&parent, b"blocks restoration").unwrap();

        rollback.restore().await;

        assert_eq!(std::fs::read(&backup).unwrap(), b"original contents");
        assert_eq!(std::fs::read(&parent).unwrap(), b"blocks restoration");
        std::fs::remove_dir_all(backup_dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn test_foreign_bin_detects_only_unowned_regular_files() {
        let temp = tempfile::TempDir::new().unwrap();
        let bin_dir = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        std::fs::write(bin_dir.join("foreign"), b"original executable").unwrap();
        std::os::unix::fs::symlink(temp.path().join("cli.js"), bin_dir.join("npm-link")).unwrap();
        std::fs::write(bin_dir.join("node"), b"protected shim").unwrap();

        assert_eq!(foreign_bin(&bin_dir, "foreign"), Some(bin_dir.join("foreign")));
        assert_eq!(foreign_bin(&bin_dir, "npm-link"), None);
        assert_eq!(foreign_bin(&bin_dir, "missing"), None);
        assert_eq!(foreign_bin(&bin_dir, "node"), None);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_rollback_restores_replaced_npm_link_and_owner() {
        let temp = tempfile::TempDir::new().unwrap();
        vp_shared::EnvConfig::with_vars_async([(env_vars::VP_HOME, temp.path())], |_| async {
            let bin_dir = get_bin_dir().unwrap();
            std::fs::create_dir_all(&bin_dir).unwrap();
            let old_target = temp.path().join("original-cli.js");
            std::fs::write(&old_target, b"original npm binary").unwrap();
            std::os::unix::fs::symlink(&old_target, bin_dir.join("replaced")).unwrap();
            BinConfig::new_npm("replaced".into(), "test-package".into(), "22.18.0".into())
                .save()
                .await
                .unwrap();
            let config_path = BinConfig::path("replaced").unwrap();
            let old_config = std::fs::read(&config_path).unwrap();

            let mut rollback = BinRollback::default();
            rollback.capture_bin(&bin_dir, "replaced").await.unwrap();
            create_package_shim(&bin_dir, "replaced", "test-package").await.unwrap();
            BinConfig::new(
                "replaced".into(),
                "test-package".into(),
                "2.0.0".into(),
                "22.18.0".into(),
            )
            .save()
            .await
            .unwrap();
            restore_previous_install_state("test-package", None, rollback).await;

            assert_eq!(std::fs::read_link(bin_dir.join("replaced")).unwrap(), old_target);
            assert_eq!(std::fs::read(config_path).unwrap(), old_config);
        })
        .await;
    }

    #[tokio::test]
    async fn test_cleanup_stale_installations_supports_all_layouts() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        vp_shared::EnvConfig::with_vars_async([(env_vars::VP_HOME, temp_dir.path())], |_| async {
            let package_name = "@scope/test-package";
            let current_id = "123e4567-e89b-42d3-a456-426614174000";
            let stale_id = "987e6543-e21b-42d3-a456-426614174000";
            let legacy_id = "#2753e02c-9319-456f-bc0f-23d6d7b6fba5";
            let package_dir = PackageMetadata::installation_dir_for(package_name, "").unwrap();
            let current_dir =
                PackageMetadata::installation_dir_for(package_name, current_id).unwrap();
            let stale_dir = PackageMetadata::installation_dir_for(package_name, stale_id).unwrap();
            let legacy_dir =
                PackageMetadata::installation_dir_for(package_name, legacy_id).unwrap();

            tokio::fs::create_dir_all(&current_dir).await.unwrap();
            tokio::fs::create_dir_all(&stale_dir).await.unwrap();
            tokio::fs::create_dir_all(&legacy_dir).await.unwrap();
            tokio::fs::write(install_dir_lock_path(&current_dir).unwrap(), "").await.unwrap();
            let legacy_package_json =
                get_node_modules_dir(&package_dir, package_name).join("package.json");
            tokio::fs::create_dir_all(legacy_package_json.parent().unwrap()).await.unwrap();
            tokio::fs::write(&legacy_package_json, "{}").await.unwrap();

            cleanup_stale_installations(package_name, current_id).await;

            assert!(current_dir.as_path().is_dir());
            assert!(install_dir_lock_path(&current_dir).unwrap().as_path().is_file());
            assert!(!stale_dir.as_path().exists());
            assert!(!legacy_dir.as_path().exists());
            assert!(!legacy_package_json.as_path().exists());
            assert!(!install_dir_lock_path(&package_dir).unwrap().as_path().exists());
        })
        .await;
    }

    #[test]
    fn test_new_install_id_is_uuid() {
        let install_id = new_install_id();

        assert!(is_nested_install_id(&install_id));
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

    #[cfg(unix)]
    #[test]
    fn test_ensure_executable_bits_only_changes_package_files() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::TempDir::new().unwrap();
        let package_dir = temp.path().join("package");
        std::fs::create_dir(&package_dir).unwrap();
        let mode =
            |path: &std::path::Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
        let write = |path: &std::path::Path, mode: u32| {
            std::fs::write(path, "#!/bin/sh\n").unwrap();
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
        };
        let package_root = AbsolutePathBuf::new(package_dir.clone()).unwrap();

        let native = package_dir.join("native.sh");
        write(&native, 0o640);
        ensure_executable_bits(&AbsolutePathBuf::new(native.clone()).unwrap(), &package_root)
            .unwrap();
        assert_eq!(mode(&native), 0o751);

        let outside = temp.path().join("outside.sh");
        write(&outside, 0o644);
        let link = package_dir.join("link.sh");
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        ensure_executable_bits(&AbsolutePathBuf::new(link).unwrap(), &package_root).unwrap();
        assert_eq!(mode(&outside), 0o644);

        let missing = AbsolutePathBuf::new(package_dir.join("missing.sh")).unwrap();
        ensure_executable_bits(&missing, &package_root).unwrap();
    }

    #[test]
    fn test_is_javascript_binary_with_js_extension() {
        use tempfile::TempDir;
        use vt_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let js_file = temp_dir.path().join("cli.js");
        std::fs::write(&js_file, "console.log('hello')").unwrap();

        let path = AbsolutePathBuf::new(js_file).unwrap();
        assert!(is_javascript_binary(&path).unwrap());
    }

    #[test]
    fn test_is_javascript_binary_with_mjs_extension() {
        use tempfile::TempDir;
        use vt_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let mjs_file = temp_dir.path().join("cli.mjs");
        std::fs::write(&mjs_file, "export default 'hello'").unwrap();

        let path = AbsolutePathBuf::new(mjs_file).unwrap();
        assert!(is_javascript_binary(&path).unwrap());
    }

    #[test]
    fn test_is_javascript_binary_with_cjs_extension() {
        use tempfile::TempDir;
        use vt_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let cjs_file = temp_dir.path().join("cli.cjs");
        std::fs::write(&cjs_file, "module.exports = 'hello'").unwrap();

        let path = AbsolutePathBuf::new(cjs_file).unwrap();
        assert!(is_javascript_binary(&path).unwrap());
    }

    #[test]
    fn test_is_javascript_binary_with_node_shebang() {
        use tempfile::TempDir;
        use vt_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let cli_file = temp_dir.path().join("cli");
        std::fs::write(&cli_file, "#!/usr/bin/env node\nconsole.log('hello')").unwrap();

        let path = AbsolutePathBuf::new(cli_file).unwrap();
        assert!(is_javascript_binary(&path).unwrap());
    }

    #[test]
    fn test_is_javascript_binary_with_direct_node_shebang() {
        use tempfile::TempDir;
        use vt_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let cli_file = temp_dir.path().join("cli");
        std::fs::write(&cli_file, "#!/usr/bin/node\nconsole.log('hello')").unwrap();

        let path = AbsolutePathBuf::new(cli_file).unwrap();
        assert!(is_javascript_binary(&path).unwrap());
    }

    #[test]
    fn test_is_javascript_binary_native_executable() {
        use tempfile::TempDir;
        use vt_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        // Simulate a native binary (ELF header)
        let native_file = temp_dir.path().join("native-cli");
        std::fs::write(&native_file, b"\x7fELF").unwrap();

        let path = AbsolutePathBuf::new(native_file).unwrap();
        assert!(!is_javascript_binary(&path).unwrap());
    }

    #[test]
    fn test_is_javascript_binary_shell_script() {
        use tempfile::TempDir;
        use vt_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let shell_file = temp_dir.path().join("script.sh");
        std::fs::write(&shell_file, "#!/bin/bash\necho hello").unwrap();

        let path = AbsolutePathBuf::new(shell_file).unwrap();
        assert!(!is_javascript_binary(&path).unwrap());
    }

    #[test]
    fn test_is_javascript_binary_python_script() {
        use tempfile::TempDir;
        use vt_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let python_file = temp_dir.path().join("script.py");
        std::fs::write(&python_file, "#!/usr/bin/env python3\nprint('hello')").unwrap();

        let path = AbsolutePathBuf::new(python_file).unwrap();
        assert!(!is_javascript_binary(&path).unwrap());
    }

    #[test]
    fn test_is_javascript_binary_empty_file() {
        use tempfile::TempDir;
        use vt_path::AbsolutePathBuf;

        let temp_dir = TempDir::new().unwrap();
        let empty_file = temp_dir.path().join("empty");
        std::fs::write(&empty_file, "").unwrap();

        let path = AbsolutePathBuf::new(empty_file).unwrap();
        assert!(!is_javascript_binary(&path).unwrap());
    }

    #[test]
    fn test_is_javascript_binary_rejects_directories() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        for name in ["directory", "directory.js", "directory.mjs", "directory.cjs"] {
            let path = AbsolutePathBuf::new(temp_dir.path().join(name)).unwrap();
            std::fs::create_dir(&path).unwrap();
            for target in [path.clone(), path.join(""), path.join(".")] {
                assert!(is_javascript_binary(&target).is_err(), "{target:?}");
            }
        }
    }

    #[test]
    fn test_is_javascript_binary_preserves_missing_target_classification() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        for (name, expected) in
            [("missing", false), ("missing.js", true), ("missing.mjs", true), ("missing.cjs", true)]
        {
            let path = AbsolutePathBuf::new(temp_dir.path().join(name)).unwrap();
            assert_eq!(is_javascript_binary(&path).unwrap(), expected, "{name}");
        }
    }
}
