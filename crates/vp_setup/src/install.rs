//! Installation logic shared between `vp upgrade` and `vp-setup.exe`.
//!
//! Handles tarball extraction, dependency installation, symlink swapping,
//! and version cleanup.

use std::{
    env,
    io::{Cursor, Read as _},
    path::Path,
    process::{self, Output},
    time::{SystemTime, UNIX_EPOCH},
};

use flate2::read::GzDecoder;
use tar::Archive;
use vp_js_runtime::{JsRuntimeType, download_runtime};
use vp_pm_cli::{bootstrap_pnpm, npm_user_config_path};
use vp_shared::EnvConfig;
use vt_path::{AbsolutePath, AbsolutePathBuf};

use crate::{DEFAULT_INSTALL_NODE_VERSION, error::Error, lockfile};

/// Validate that a path from a tarball entry is safe (no path traversal).
///
/// Returns `false` if the path contains `..` components or is absolute.
fn is_safe_tar_path(path: &Path) -> bool {
    // Also check for Unix-style absolute paths, since tar archives always use forward
    // slashes and `Path::is_absolute()` on Windows only recognizes `C:\...` style paths.
    let starts_with_slash = path.to_string_lossy().starts_with('/');
    !path.is_absolute()
        && !starts_with_slash
        && !path.components().any(|c| matches!(c, std::path::Component::ParentDir))
}

/// Extract the platform-specific binary and its installation lockfile.
///
/// From the platform tarball, extracts:
/// - The `vp` binary → `{version_dir}/bin/vp`
/// - The `vp-shim.exe` trampoline → `{version_dir}/bin/vp-shim.exe` (Windows only)
/// - The release lockfile → `{version_dir}/bin/vp-install-lock.yaml`
///
/// `.node` files are no longer extracted here — npm installs them
/// via the platform package's optionalDependencies.
pub async fn extract_platform_package(
    tgz_data: &[u8],
    version_dir: &AbsolutePath,
) -> Result<(), Error> {
    let bin_dir = version_dir.join("bin");
    tokio::fs::create_dir_all(&bin_dir).await?;

    let data = tgz_data.to_vec();
    let bin_dir_clone = bin_dir.clone();
    let skip_lockfile = EnvConfig::get().setup_skip_install_lockfile;

    tokio::task::spawn_blocking(move || {
        let cursor = Cursor::new(data);
        let decoder = GzDecoder::new(cursor);
        let mut archive = Archive::new(decoder);
        let mut requires_lockfile = false;
        let mut has_lockfile = false;

        for entry_result in archive.entries()? {
            let mut entry = entry_result?;
            let path = entry.path()?.to_path_buf();

            // Strip the leading `package/` prefix that npm tarballs have
            let relative = path.strip_prefix("package").unwrap_or(&path).to_path_buf();

            // Reject paths with traversal components (security)
            if !is_safe_tar_path(&relative) {
                continue;
            }

            let file_name = relative.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if relative == Path::new("package.json") && entry.header().entry_type().is_file() {
                let manifest: serde_json::Value = serde_json::from_reader(&mut entry)?;
                requires_lockfile = manifest
                    .get("vpInstallLockfile")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
            } else if !skip_lockfile
                && relative == Path::new(lockfile::INSTALL_LOCKFILE)
                && entry.header().entry_type().is_file()
            {
                lockfile::write_atomic(
                    &bin_dir_clone.join(lockfile::INSTALL_LOCKFILE),
                    &mut entry,
                )?;
                has_lockfile = true;
            }

            if file_name == "vp" || file_name == "vp.exe" || file_name == "vp-shim.exe" {
                // Binary goes to bin/
                let target = bin_dir_clone.join(file_name);
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf)?;
                std::fs::write(&target, &buf)?;

                // Set executable permission on Unix
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755))?;
                }
            }
        }

        if requires_lockfile && !has_lockfile && !skip_lockfile {
            return Err(Error::Setup(
                format!("Platform package is missing {}", lockfile::INSTALL_LOCKFILE).into(),
            ));
        }

        Ok::<(), Error>(())
    })
    .await
    .map_err(|e| Error::Setup(format!("Task join error: {e}").into()))??;

    Ok(())
}

/// The pnpm version pinned in the wrapper package.json for global installs.
/// This ensures consistent install behavior regardless of the user's global pnpm version.
const PINNED_PNPM_VERSION: &str = "10.34.9";

/// Integrity of the pinned pnpm npm tarball, in the `packageManager` hash
/// format (`sha512.<hex>`). Setup verifies the downloaded pnpm against this
/// value, which the binary carries.
///
/// A project `.npmrc` may point the registry at any host, so setup must not
/// trust an integrity value that the download source reports for its own
/// tarball. This pin is the `sha512` of the official npm tarball for
/// `PINNED_PNPM_VERSION`. A genuine registry or mirror serves identical bytes
/// and still matches it, so private registries keep working. Update this value
/// together with `PINNED_PNPM_VERSION`.
const PINNED_PNPM_INTEGRITY: &str = "sha512.f49b5a8ea0fdedb44bb3c0c1d6575092a53c2950d95b7ad15df11f17974877a2de3a01c07e934e9b28ce716e51c09a94962c7306d09be2e1e4b18a8c8762fd32";

/// A reused target version must run its own setup again before accepting commands.
pub async fn clear_self_setup_marker(version_dir: &AbsolutePath) -> Result<(), Error> {
    match tokio::fs::remove_file(version_dir.join("bin").join(crate::SELF_SETUP_MARKER)).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

/// Generate a wrapper `package.json` that declares `vite-plus` as a dependency.
///
/// The `packageManager` field pins pnpm to a known-good version, ensuring
/// consistent behavior regardless of the user's global pnpm version.
/// pnpm will install `vite-plus` and all its transitive deps via `vp install`.
/// Under `VP_ENV_FORBIDDEN=1` the field is omitted: the system pnpm installs,
/// and a pin would make it download the pinned version.
pub async fn generate_wrapper_package_json(
    version_dir: &AbsolutePath,
    version: &str,
) -> Result<(), Error> {
    let mut json = serde_json::json!({
        "name": "vp-global",
        "version": version,
        "private": true,
        "packageManager": format!("pnpm@{PINNED_PNPM_VERSION}+{PINNED_PNPM_INTEGRITY}"),
        "dependencies": {
            "vite-plus": version
        }
    });
    // Remove after building: inserting would move the field in the default output.
    if EnvConfig::get().env_forbidden
        && let Some(fields) = json.as_object_mut()
    {
        fields.remove("packageManager");
    }
    let content = serde_json::to_string_pretty(&json)? + "\n";
    tokio::fs::write(version_dir.join("package.json"), content).await?;
    Ok(())
}

/// Create a local `.npmrc` in the version directory to bypass pnpm's
/// `minimumReleaseAge` setting that may block installing recently-published packages.
pub async fn write_release_age_overrides(version_dir: &AbsolutePath) -> Result<(), Error> {
    let npmrc_path = version_dir.join(".npmrc");
    tokio::fs::write(&npmrc_path, "minimum-release-age=0\n").await?;
    Ok(())
}

fn format_install_failure_message(exit_code: i32, log_path: Option<&AbsolutePathBuf>) -> String {
    let log_msg = log_path
        .map_or_else(String::new, |p| format!(". See log for details: {}", p.as_path().display()));

    format!("Failed to install production dependencies (exit code: {exit_code}){log_msg}")
}

/// Fixed diagnostics are the only subprocess output that can enter the upgrade log.
/// Do not accept arbitrary `ERR_PNPM_*` values or copy the matching output line:
/// package names, URLs, and even unrecognized error identifiers can contain secrets.
const INSTALL_FAILURE_DIAGNOSTICS: &[(&str, &str)] = &[
    ("ERR_PNPM_FETCH_401", "Check npm registry authentication in your user configuration."),
    ("ERR_PNPM_FETCH_403", "Check your permission to download packages from the npm registry."),
    (
        "ERR_PNPM_OUTDATED_LOCKFILE",
        "The installation lockfile does not match the package manifest. Retry with an updated Vite+ release.",
    ),
    (
        "ERR_PNPM_LOCKFILE_BREAKING_CHANGE",
        "The installation lockfile is incompatible with pnpm. Retry with an updated Vite+ release.",
    ),
    ("ERR_PNPM_TARBALL_INTEGRITY", "A package failed checksum validation. Retry the command."),
    ("ENOSPC", "Free disk space before retrying the command."),
    ("EDQUOT", "Free disk space or increase the storage quota before retrying the command."),
    ("EACCES", "Check write permissions for the Vite+ data and cache directories."),
    ("EPERM", "Check permissions for the Vite+ data and cache directories."),
];

/// Lockfile errors from the system pnpm (`VP_ENV_FORBIDDEN=1`) are not fixed by a
/// newer Vite+ release: that pnpm may not read the pinned pnpm's lockfile.
const SYSTEM_PNPM_LOCKFILE_DIAGNOSTICS: &[&str] =
    &["ERR_PNPM_OUTDATED_LOCKFILE", "ERR_PNPM_LOCKFILE_BREAKING_CHANGE"];

fn install_failure_log(
    exit_code: i32,
    lockfile_bypass: bool,
    stdout: &[u8],
    stderr: &[u8],
) -> String {
    let mut content = format!(
        "Stage: production dependency installation.\npnpm install failed with exit code {exit_code}.\n"
    );
    let mut recognized = false;
    for (code, hint) in INSTALL_FAILURE_DIAGNOSTICS {
        let reported = [stdout, stderr].iter().any(|output| {
            output
                .split(|byte| !byte.is_ascii_alphanumeric() && *byte != b'_')
                .any(|token| token == code.as_bytes())
        });
        if reported {
            recognized = true;
            if lockfile_bypass && SYSTEM_PNPM_LOCKFILE_DIAGNOSTICS.contains(code) {
                content.push_str(&format!(
                    "Reported error: {code}.\nThe system pnpm cannot use the installation lockfile. Retry with {}=1.\n",
                    vp_shared::env_vars::VP_SETUP_SKIP_INSTALL_LOCKFILE
                ));
            } else {
                content.push_str(&format!("Reported error: {code}.\n{hint}\n"));
            }
        }
    }
    if !recognized {
        content.push_str(
            "No recognized error identifier was reported. Check the Node.js/pnpm runtime, disk space, and npm configuration before retrying.\n",
        );
    }
    content.push_str("Registry output is not saved because it can contain credentials.\n");
    content
}

/// Record the failed stage, exit code, and allowlisted diagnostics without raw output.
///
/// The log is written to the **parent** of `version_dir` (i.e. `<DATA>/upgrade.log`)
/// so it survives the cleanup that removes `version_dir` on failure.
/// `lockfile_bypass` points lockfile errors to `VP_SETUP_SKIP_INSTALL_LOCKFILE=1`,
/// for the system pnpm with the installation lockfile.
///
/// Returns the log file path on success, or `None` if writing failed.
pub async fn write_upgrade_log(
    version_dir: &AbsolutePath,
    exit_code: i32,
    lockfile_bypass: bool,
    stdout: &[u8],
    stderr: &[u8],
) -> Option<AbsolutePathBuf> {
    // Write to parent dir so the log survives version_dir cleanup on failure
    let parent = version_dir.as_path().parent()?;
    let log_path = AbsolutePathBuf::new(parent.join("upgrade.log"))?;
    let content = install_failure_log(exit_code, lockfile_bypass, stdout, stderr);
    match tokio::fs::write(&log_path, &content).await {
        Ok(()) => Some(log_path),
        Err(e) => {
            tracing::warn!("Failed to write upgrade log: {}", e);
            None
        }
    }
}

/// Select the runtime for dependency installation.
pub enum NodeVersion<'a> {
    /// Reuse a CI-selected runtime when it supports the pinned pnpm.
    Preferred(&'a str),
    /// Use the target release's exact runtime or fail before activation.
    Required(&'a str),
    /// Use the system `node` and `pnpm` that [`system_install_tools`] checked.
    /// Required under `VP_ENV_FORBIDDEN=1`.
    System(&'a SystemInstallTools),
}

/// The system `node` and `pnpm` that install Vite+ dependencies under
/// `VP_ENV_FORBIDDEN=1`, and the PATH they were selected from.
#[derive(Debug)]
pub struct SystemInstallTools {
    node: AbsolutePathBuf,
    pnpm: AbsolutePathBuf,
    /// The PATH used for the lookup, with relative entries made absolute.
    /// pnpm runs with it, so its `#!/usr/bin/env node` finds the same `node`.
    path: std::ffi::OsString,
}

/// Select the system `node` and `pnpm` for dependency installation under
/// `VP_ENV_FORBIDDEN=1`.
///
/// Call this before touching the installation: it fails unless the first
/// `node` and the first `pnpm` on PATH are system tools, or when `node` does
/// not satisfy `node_range` (the target release's `engines.node`).
///
/// # Errors
///
/// Returns an error if a tool is missing, is not a system tool, or `node`
/// is unsupported.
pub async fn system_install_tools(node_range: Option<&str>) -> Result<SystemInstallTools, Error> {
    let path = env::var_os("PATH").unwrap_or_default();
    let cwd = vt_path::current_dir()?;
    let self_exe = env::current_exe().ok().and_then(|exe| exe.canonicalize().ok());
    find_system_install_tools(&path, &cwd, self_exe.as_deref(), node_range).await
}

async fn find_system_install_tools(
    path: &std::ffi::OsStr,
    cwd: &AbsolutePath,
    self_exe: Option<&Path>,
    node_range: Option<&str>,
) -> Result<SystemInstallTools, Error> {
    // pnpm runs in the version directory, so resolve relative entries here.
    let path = env::join_paths(env::split_paths(path).map(|entry| {
        if entry.is_absolute() || entry.starts_with("~") {
            entry
        } else {
            cwd.as_path().join(entry)
        }
    }))
    .map_err(|error| Error::Setup(format!("Failed to read PATH: {error}").into()))?;
    let node = system_tool("node", &path, self_exe, cwd)?;
    let pnpm = system_tool("pnpm", &path, self_exe, cwd)?;
    if let Some(range) = node_range {
        let output = tokio::process::Command::new(node.as_path()).arg("--version").output().await?;
        let version = String::from_utf8_lossy(&output.stdout);
        let version = version.trim().trim_start_matches('v');
        let supported = output.status.success()
            && js_semver::Range::parse(range).is_ok_and(|range| {
                js_semver::Version::parse(version).is_ok_and(|version| range.satisfies(&version))
            });
        if !supported {
            return Err(Error::Setup(
                format!(
                    "{}=1: Vite+ installs its dependencies with the system Node.js, but {} reports '{version}', which does not satisfy '{range}'",
                    vp_shared::env_vars::VP_ENV_FORBIDDEN,
                    node.as_path().display(),
                )
                .into(),
            ));
        }
    }
    Ok(SystemInstallTools { node, pnpm, path })
}

/// The first `tool` on `path` if it is a system tool. Later matches are never used.
fn system_tool(
    tool: &str,
    path: &std::ffi::OsStr,
    self_exe: Option<&Path>,
    cwd: &AbsolutePath,
) -> Result<AbsolutePathBuf, Error> {
    let var = vp_shared::env_vars::VP_ENV_FORBIDDEN;
    let reason = match vp_command::classify_first_path_match(tool, path, self_exe, cwd) {
        vp_command::FirstPathMatch::System(found) => return Ok(found),
        vp_command::FirstPathMatch::ViteShim(found) => format!(
            "the Vite+ shim at '{}' comes first on PATH; run `vp env off {tool}` to move the shims after the system tools",
            found.as_path().display()
        ),
        vp_command::FirstPathMatch::VpOwned(found) => format!(
            "PATH selects the Vite+ managed install at '{}'; put the system '{tool}' first on PATH",
            found.as_path().display()
        ),
        vp_command::FirstPathMatch::NotFound => {
            format!("no '{tool}' is on PATH; install it with your system package manager")
        }
    };
    Err(Error::Setup(
        format!("{var}=1: Vite+ installs its dependencies with the system '{tool}', but {reason}")
            .into(),
    ))
}

/// Install production dependencies with a compatible managed Node.js and pinned pnpm.
///
/// Spawns: `node <managed-pnpm>/bin/pnpm.cjs install --ignore-workspace`
/// with `CI=true`. With [`NodeVersion::System`], which `VP_ENV_FORBIDDEN=1`
/// requires, runs the system pnpm instead and downloads no Node.js or pnpm.
/// On failure, records safe diagnostics in the parent directory's `upgrade.log`.
pub async fn install_production_deps(
    version_dir: &AbsolutePath,
    registry: Option<&str>,
    show_progress: bool,
    node_version: Option<NodeVersion<'_>>,
) -> Result<(), Error> {
    tracing::debug!("Running pnpm install in {}", version_dir.as_path().display());
    lockfile::invalidate_install(version_dir).await?;
    if EnvConfig::get().setup_skip_install_lockfile {
        vp_shared::output::warn(
            "VP_SETUP_SKIP_INSTALL_LOCKFILE=1: ignoring the installation lockfile; resolving dependencies again.",
        );
    }

    // Keep the bypass local to this Vite+ installation.
    write_release_age_overrides(version_dir).await?;

    let args = dependency_install_args(version_dir, registry).await?;

    let system = match &node_version {
        Some(NodeVersion::System(tools)) => Some(*tools),
        _ if EnvConfig::get().env_forbidden => {
            return Err(Error::Setup(
                format!(
                    "{}=1: Vite+ installs its dependencies only with a checked system node and pnpm",
                    vp_shared::env_vars::VP_ENV_FORBIDDEN
                )
                .into(),
            ));
        }
        _ => None,
    };
    let output = if let Some(tools) = system {
        vp_shared::progress::with_spinner(
            show_progress.then_some("Installing dependencies..."),
            run_system_pnpm_install(version_dir, tools, &args, registry),
        )
        .await?
    } else {
        let (node_runtime, pnpm_entry) = vp_shared::progress::with_spinner(
            show_progress.then_some("Preparing Node.js and pnpm..."),
            prepare_install_runtime(node_version, registry),
        )
        .await?;
        vp_shared::progress::with_spinner(
            show_progress.then_some("Installing dependencies..."),
            run_pnpm_install(version_dir, &node_runtime, &pnpm_entry, &args, registry),
        )
        .await?
    };

    if !output.status.success() {
        let exit_code = vp_shared::exit_code_from_status(output.status);
        // The system pnpm may not read the installation lockfile; the bypass resolves again.
        let lockfile_bypass = system.is_some() && args.contains(&"--frozen-lockfile");
        let log_path = write_upgrade_log(
            version_dir,
            exit_code,
            lockfile_bypass,
            &output.stdout,
            &output.stderr,
        )
        .await;
        let mut message = format_install_failure_message(exit_code, log_path.as_ref());
        if lockfile_bypass {
            message.push_str(&format!(
                ". If the system pnpm cannot use the installation lockfile, retry with {}=1",
                vp_shared::env_vars::VP_SETUP_SKIP_INSTALL_LOCKFILE
            ));
        }
        return Err(Error::Setup(message.into()));
    }

    lockfile::record_install(version_dir).await?;
    if show_progress {
        vp_shared::output::success("Dependencies installed.");
    }
    Ok(())
}

async fn dependency_install_args<'a>(
    version_dir: &AbsolutePath,
    registry: Option<&'a str>,
) -> Result<Vec<&'a str>, Error> {
    // An ancestor workspace must not capture the install or supply its lockfile.
    let mut args = vec!["install", "--ignore-workspace"];
    if EnvConfig::get().setup_skip_install_lockfile {
        // Disable both pnpm's CI default and reading any existing working lockfile.
        args.extend(["--no-frozen-lockfile", "--lockfile=false"]);
    } else if tokio::fs::try_exists(version_dir.join("bin").join(lockfile::INSTALL_LOCKFILE))
        .await?
    {
        lockfile::restore(version_dir).await?;
        // Keep these flags and the numeric setting in run_pnpm_install in sync with
        // packages/cli/generate-install-lockfile.ts.
        args.extend([
            "--frozen-lockfile",
            "--lockfile=true",
            "--config.auto-install-peers=true",
            "--config.exclude-links-from-lockfile=false",
            "--config.dedupe-peers=false",
            "--config.inject-workspace-packages=false",
        ]);
    }
    if let Some(registry_url) = registry {
        args.extend(["--registry", registry_url]);
    }
    Ok(args)
}

async fn prepare_install_runtime(
    node_version: Option<NodeVersion<'_>>,
    registry: Option<&str>,
) -> Result<(vp_js_runtime::JsRuntime, AbsolutePathBuf), Error> {
    let pnpm_dir = bootstrap_pnpm(PINNED_PNPM_VERSION, PINNED_PNPM_INTEGRITY, registry)
        .await
        .map_err(|error| {
            Error::Setup(format!("Failed to install pnpm {PINNED_PNPM_VERSION}: {error}").into())
        })?;
    let pnpm_entry = pnpm_dir.join("bin").join("pnpm.cjs");
    if !tokio::fs::try_exists(&pnpm_entry).await.unwrap_or(false) {
        return Err(Error::Setup(
            format!("pnpm entry not found at {}", pnpm_entry.as_path().display()).into(),
        ));
    }
    let package: serde_json::Value = serde_json::from_slice(
        &tokio::fs::read(pnpm_dir.join("package.json")).await?,
    )
    .map_err(|error| {
        Error::Setup(format!("Failed to read pnpm's Node.js requirement: {error}").into())
    })?;
    let node_version = match node_version {
        Some(NodeVersion::Required(version)) => {
            if !supports_pnpm_node(&package, version) {
                return Err(Error::Setup(
                    format!(
                        "Node.js {version} required by the target Vite+ release is not supported by pnpm {PINNED_PNPM_VERSION}"
                    )
                    .into(),
                ));
            }
            version
        }
        Some(NodeVersion::Preferred(version)) if supports_pnpm_node(&package, version) => version,
        _ => DEFAULT_INSTALL_NODE_VERSION,
    };
    let node_runtime =
        download_runtime(JsRuntimeType::Node, node_version).await.map_err(|error| {
            Error::Setup(format!("Failed to install Node.js {node_version}: {error}").into())
        })?;
    Ok((node_runtime, pnpm_entry))
}

fn supports_pnpm_node(package: &serde_json::Value, version: &str) -> bool {
    let Some(range) = package["engines"]["node"].as_str() else { return false };
    let (Ok(range), Ok(version)) =
        (js_semver::Range::parse(range), js_semver::Version::parse(version))
    else {
        return false;
    };
    range.satisfies(&version)
}

/// Keep pnpm's registry and user config aligned with the native downloader when
/// its working directory differs from the caller's.
fn configure_pnpm_command(
    command: &mut tokio::process::Command,
    registry: Option<&str>,
) -> Result<(), Error> {
    if let Some(registry) = registry {
        command.env(vp_shared::env_vars::NPM_CONFIG_REGISTRY, registry);
        command.env(vp_shared::env_vars::NPM_CONFIG_REGISTRY_UPPER, registry);
    }
    if let Some(path) = npm_user_config_path(None) {
        let path = vt_path::current_dir()?.join(path);
        command.env("npm_config_userconfig", path.as_path());
        command.env("NPM_CONFIG_USERCONFIG", path.as_path());
    }
    Ok(())
}

async fn run_pnpm_install(
    version_dir: &AbsolutePath,
    node_runtime: &vp_js_runtime::JsRuntime,
    pnpm_entry: &AbsolutePath,
    args: &[&str],
    registry: Option<&str>,
) -> Result<Output, Error> {
    let node_bin = node_runtime.get_bin_prefix();
    let pnpm_bin = pnpm_entry.parent().ok_or_else(|| {
        Error::Setup(format!("pnpm entry has no parent: {}", pnpm_entry.as_path().display()).into())
    })?;
    let current_path = env::var_os("PATH").unwrap_or_default();
    let mut path_entries = vec![node_bin.as_path().to_path_buf(), pnpm_bin.as_path().to_path_buf()];
    path_entries.extend(env::split_paths(&current_path));
    let path = env::join_paths(path_entries)
        .map_err(|error| Error::Setup(format!("Failed to build PATH for pnpm: {error}").into()))?;

    let mut cmd = tokio::process::Command::new(node_runtime.get_binary_path().as_path());
    cmd.arg(pnpm_entry.as_path()).args(args).env("PATH", path);
    configure_pnpm_install(&mut cmd, version_dir, args, registry)?;
    Ok(cmd.output().await?)
}

/// Run the system pnpm that [`system_install_tools`] selected, on its PATH.
async fn run_system_pnpm_install(
    version_dir: &AbsolutePath,
    tools: &SystemInstallTools,
    args: &[&str],
    registry: Option<&str>,
) -> Result<Output, Error> {
    tracing::debug!(
        "Installing with system node {} and pnpm {}",
        tools.node.as_path().display(),
        tools.pnpm.as_path().display()
    );
    let mut cmd = tokio::process::Command::new(tools.pnpm.as_path());
    cmd.args(args)
        // A different pnpm version must not download the version it would switch to.
        .arg("--config.manage-package-manager-versions=false")
        // Match the pinned pnpm, which does not run dependency build scripts.
        .arg("--ignore-scripts")
        .env("PATH", &tools.path);
    configure_pnpm_install(&mut cmd, version_dir, args, registry)?;
    Ok(cmd.output().await?)
}

fn configure_pnpm_install(
    cmd: &mut tokio::process::Command,
    version_dir: &AbsolutePath,
    args: &[&str],
    registry: Option<&str>,
) -> Result<(), Error> {
    cmd.current_dir(version_dir)
        // The spinner's raw stdin would be restored by Node when pnpm exits.
        .stdin(process::Stdio::null())
        .env("CI", "true");

    if args.contains(&"--frozen-lockfile") {
        // --config leaves numbers as strings. Environment values get pnpm's numeric parsing.
        // Override both spellings so neither an inherited value nor the user npmrc can win.
        cmd.env("npm_config_peers_suffix_max_length", "1000")
            .env("NPM_CONFIG_PEERS_SUFFIX_MAX_LENGTH", "1000");
    }

    configure_pnpm_command(cmd, registry)
}

/// Save the current version before swapping, for rollback support.
///
/// Reads the `current` symlink target and writes the version to `.previous-version`.
pub async fn save_previous_version(install_dir: &AbsolutePath) -> Result<Option<String>, Error> {
    let version = read_current_version(install_dir).await;

    if let Some(ref v) = version {
        let prev_file = install_dir.join(".previous-version");
        tokio::fs::write(&prev_file, v).await?;
        tracing::debug!("Saved previous version: {}", v);
    }

    Ok(version)
}

/// Atomically swap the `current` symlink to point to a new version.
///
/// On Unix: creates a temp symlink then renames (atomic).
/// On Windows: removes junction and creates a new one.
pub async fn swap_current_link(install_dir: &AbsolutePath, version: &str) -> Result<(), Error> {
    let current_link = install_dir.join("current");
    let version_dir = install_dir.join(version);

    // Verify the version directory exists
    if !tokio::fs::try_exists(&version_dir).await.unwrap_or(false) {
        return Err(Error::Setup(
            format!("Version directory does not exist: {}", version_dir.as_path().display()).into(),
        ));
    }

    #[cfg(unix)]
    {
        // Atomic symlink swap: create temp link, then rename over current
        let temp_link = install_dir.join("current.new");

        // Remove temp link if it exists from a previous failed attempt
        let _ = tokio::fs::remove_file(&temp_link).await;

        tokio::fs::symlink(version, &temp_link).await?;
        tokio::fs::rename(&temp_link, &current_link).await?;
    }

    #[cfg(windows)]
    {
        // Windows: junction swap (not atomic)
        remove_windows_current_link(&current_link)?;

        junction::create(&version_dir, &current_link).map_err(|e| {
            Error::Setup(
                format!(
                    "Failed to create junction at {}: {e}\nTry removing it manually and run again.",
                    current_link.as_path().display()
                )
                .into(),
            )
        })?;
    }

    tracing::debug!("Swapped current → {}", version);
    Ok(())
}

/// Remove the Windows `current` entry without following its target.
#[cfg(windows)]
fn remove_windows_current_link(current_link: &AbsolutePath) -> Result<(), Error> {
    match std::fs::symlink_metadata(current_link) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(Error::Setup(
                format!(
                    "Failed to inspect the current link at {}: {error}",
                    current_link.as_path().display()
                )
                .into(),
            ));
        }
        Ok(_) => {}
    }

    // remove_dir removes a junction or directory symlink without changing its
    // target. It also detects a dangling junction because symlink_metadata did
    // not follow the missing target.
    if let Err(remove_error) = std::fs::remove_dir(current_link) {
        tracing::debug!("remove_dir failed ({remove_error}), trying junction::delete");
        junction::delete(current_link).map_err(|junction_error| {
            Error::Setup(
                format!(
                    "Failed to remove the current link at {}: {remove_error}. Junction cleanup also failed: {junction_error}",
                    current_link.as_path().display()
                )
                .into(),
            )
        })?;

        // junction::delete removes the reparse data. Remove the empty directory
        // that remains, unless the API removed the directory too.
        if let Err(error) = std::fs::remove_dir(current_link)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            return Err(Error::Setup(
                format!(
                    "Failed to remove the current directory at {}: {error}",
                    current_link.as_path().display()
                )
                .into(),
            ));
        }
    }

    Ok(())
}

/// Hand off to the newly activated binary using the legacy refresh command.
/// New binaries without a setup marker intercept this invocation as self-setup;
/// legacy rollback targets still execute `env setup --refresh` normally.
pub async fn refresh_shims(install_dir: &AbsolutePath) -> Result<(), Error> {
    let vp_binary = install_dir.join("current").join("bin").join(crate::VP_BINARY_NAME);

    if !tokio::fs::try_exists(&vp_binary).await.unwrap_or(false) {
        tracing::warn!(
            "New binary not found at {}, skipping shim refresh",
            vp_binary.as_path().display()
        );
        return Ok(());
    }

    tracing::debug!("Refreshing shims...");

    let output = tokio::process::Command::new(vp_binary.as_path())
        .args(["env", "setup", "--refresh"])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!(
            "Shim refresh exited with code {}, continuing anyway\n{}",
            vp_shared::exit_code_from_status(output.status),
            stderr.trim()
        );
    }

    Ok(())
}

/// Resolve the directory name to install a target `version` into.
///
/// A forced reinstall of the **currently active** version cannot overwrite the
/// active version directory on Windows, because the running `vp.exe` is locked.
/// In that one case we install into a unique build-metadata directory and
/// repoint `current` only after the install completes; every other case (a
/// normal upgrade, or a forced reinstall of a non-active version) uses the
/// plain `version` directory.
///
/// The forced directory name is deliberately a valid semver build-metadata
/// string (`{version}+force.{pid}.{nanos}`) so [`cleanup_old_versions`] — which
/// only considers entries that parse as semver — still garbage-collects it like
/// any other version directory.
pub fn target_install_dir_name(
    version: &str,
    active_install_dir: Option<&str>,
    force: bool,
) -> String {
    if force && active_install_dir.is_some_and(|active| is_install_dir_for_version(active, version))
    {
        return forced_reinstall_dir_name(version);
    }

    version.to_string()
}

/// Whether `install_dir_name` is an install directory for `version` — either the
/// plain `version` directory or a `{version}+force.*` forced-reinstall directory.
///
/// Both `vp upgrade` and `vp-setup` use this so a forced reinstall of the active
/// version is still recognized as that version on subsequent runs.
pub fn is_install_dir_for_version(install_dir_name: &str, version: &str) -> bool {
    install_dir_name == version || install_dir_name.starts_with(&format!("{version}+force."))
}

fn forced_reinstall_dir_name(version: &str) -> String {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    format!("{version}+force.{}.{}", process::id(), nanos)
}

const INSTALL_LOCK_FILE: &str = ".install.lock";

/// Serialize installation changes in `install_dir` across processes.
///
/// Hold the returned file while changing version directories, `current`,
/// `.previous-version`, or cleaning up old versions. In-place self-setup must
/// not reacquire its parent's installation lock.
pub async fn lock_install_dir(install_dir: &AbsolutePath) -> Result<std::fs::File, Error> {
    tokio::fs::create_dir_all(install_dir).await?;
    let path = install_dir.join(INSTALL_LOCK_FILE);
    Ok(vp_shared::file_lock::lock_exclusive(&path).await?)
}

/// Create `version_dir` and return whether this call created it.
///
/// The directory can already exist because a kept version or the rollback
/// target is being installed again. Pass the result to
/// [`discard_failed_version_dir`] so a failed install keeps that directory.
pub async fn create_version_dir(version_dir: &AbsolutePath) -> Result<bool, Error> {
    if let Some(parent) = version_dir.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    match tokio::fs::create_dir(version_dir).await {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(error) => Err(error.into()),
    }
}

/// Remove a failed install only when [`create_version_dir`] created it.
pub async fn discard_failed_version_dir(version_dir: &AbsolutePath, created: bool) {
    if !created {
        tracing::debug!("Keeping existing version directory {}", version_dir.as_path().display());
        return;
    }
    tracing::debug!("Cleaning up failed install at {}", version_dir.as_path().display());
    if let Err(error) = tokio::fs::remove_dir_all(version_dir).await
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!("Failed to remove {}: {error}", version_dir.as_path().display());
    }
}

/// Clean up old version directories, keeping at most `max_keep` versions.
///
/// Sorts by creation time (newest first, matching install.sh behavior) and removes
/// the oldest beyond the limit. Protected versions are never removed, even if they
/// fall outside the keep limit (e.g., the active version after a downgrade).
pub async fn cleanup_old_versions(
    install_dir: &AbsolutePath,
    max_keep: usize,
    protected_versions: &[&str],
) -> Result<(), Error> {
    let mut versions: Vec<(std::time::SystemTime, AbsolutePathBuf)> = Vec::new();

    let mut entries = tokio::fs::read_dir(install_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Only consider entries that parse as semver. Forced-reinstall dirs use
        // semver build metadata (see `forced_reinstall_dir_name`), so they pass
        // this filter and get garbage-collected like any other version dir.
        if js_semver::Version::parse(&name_str).is_ok() {
            let metadata = entry.metadata().await?;
            // Use creation time (birth time), fallback to modified time
            let time = metadata.created().unwrap_or_else(|_| {
                metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            });
            let path = AbsolutePathBuf::new(entry.path()).ok_or_else(|| {
                Error::Setup(format!("Invalid absolute path: {}", entry.path().display()).into())
            })?;
            versions.push((time, path));
        }
    }

    for path in versions_to_remove(versions, max_keep, protected_versions) {
        tracing::debug!("Cleaning up old version: {}", path.as_path().display());
        if let Err(e) = tokio::fs::remove_dir_all(&path).await {
            tracing::warn!("Failed to remove {}: {}", path.as_path().display(), e);
        }
    }

    Ok(())
}

/// Select the version directories that [`cleanup_old_versions`] removes.
///
/// Kept apart from the directory scan so tests can supply exact creation times,
/// which some filesystems record too coarsely (or not at all) to order fresh dirs.
fn versions_to_remove(
    mut versions: Vec<(SystemTime, AbsolutePathBuf)>,
    max_keep: usize,
    protected_versions: &[&str],
) -> Vec<AbsolutePathBuf> {
    // Sort newest first (by creation time, matching install.sh)
    versions.sort_by_key(|b| std::cmp::Reverse(b.0));

    // Remove versions beyond the keep limit, but never remove protected versions
    let mut removable = Vec::new();
    for (_time, path) in versions.into_iter().skip(max_keep) {
        let name = path.as_path().file_name().and_then(|n| n.to_str()).unwrap_or("");
        if protected_versions.contains(&name) {
            tracing::debug!("Skipping protected version: {}", name);
            continue;
        }
        removable.push(path);
    }
    removable
}

/// Read the previous version from `.previous-version` file.
pub async fn read_previous_version(install_dir: &AbsolutePath) -> Result<Option<String>, Error> {
    let prev_file = install_dir.join(".previous-version");

    if !tokio::fs::try_exists(&prev_file).await.unwrap_or(false) {
        return Ok(None);
    }

    let content = tokio::fs::read_to_string(&prev_file).await?;
    let version = content.trim().to_string();

    if version.is_empty() { Ok(None) } else { Ok(Some(version)) }
}

/// Read the current installed version by following the `current` symlink/junction.
///
/// Returns `None` if no installation exists or the link target cannot be read.
pub async fn read_current_version(install_dir: &AbsolutePath) -> Option<String> {
    let current_link = install_dir.join("current");
    let target = tokio::fs::read_link(&current_link).await.ok()?;
    target.file_name().and_then(|n| n.to_str()).map(String::from)
}

/// Create shell env files by running `vp env setup --env-only`.
///
/// Used when the Node.js manager is disabled — ensures env files exist
/// even without a full shim refresh.
pub async fn create_env_files(install_dir: &AbsolutePath) -> Result<(), Error> {
    let vp_binary = install_dir.join("current").join("bin").join(crate::VP_BINARY_NAME);

    if !tokio::fs::try_exists(&vp_binary).await.unwrap_or(false) {
        return Ok(());
    }

    let output = tokio::process::Command::new(vp_binary.as_path())
        .args(["env", "setup", "--env-only"])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!(
            "env setup --env-only exited with code {}, continuing anyway\n{}",
            vp_shared::exit_code_from_status(output.status),
            stderr.trim()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_userconfig_before_resolving_against_the_caller() {
        let home = tempfile::tempdir().unwrap();
        let relative = vt_path::current_dir().unwrap().join("config/enterprise.npmrc");
        let cases = [
            ("config/enterprise.npmrc", relative.as_path().to_path_buf()),
            ("~/enterprise.npmrc", home.path().join("enterprise.npmrc")),
            ("${HOME}/enterprise.npmrc", home.path().join("enterprise.npmrc")),
        ];
        for (selector, expected) in cases {
            for variable in ["npm_config_userconfig", "NPM_CONFIG_USERCONFIG"] {
                EnvConfig::with_vars(
                    [
                        ("HOME", home.path().to_str()),
                        ("USERPROFILE", home.path().to_str()),
                        ("npm_config_userconfig", None),
                        ("NPM_CONFIG_USERCONFIG", None),
                    ],
                    |_| {
                        // Clear both spellings before setting either: they alias on Windows.
                        EnvConfig::with_vars([(variable, selector)], |_| {
                            let mut command = tokio::process::Command::new("node");
                            configure_pnpm_command(
                                &mut command,
                                Some("https://registry.example.test/"),
                            )
                            .unwrap();
                            let command_env = |name: &str| {
                                command
                                    .as_std()
                                    .get_envs()
                                    .find(|(key, _)| {
                                        if cfg!(windows) {
                                            key.eq_ignore_ascii_case(name)
                                        } else {
                                            *key == name
                                        }
                                    })
                                    .and_then(|(_, value)| value)
                            };
                            for name in ["npm_config_userconfig", "NPM_CONFIG_USERCONFIG"] {
                                assert_eq!(
                                    command_env(name).map(std::path::Path::new),
                                    Some(expected.as_path()),
                                    "{variable}={selector}"
                                );
                            }
                            for name in ["npm_config_registry", "NPM_CONFIG_REGISTRY"] {
                                assert_eq!(
                                    command_env(name),
                                    Some(std::ffi::OsStr::new("https://registry.example.test/"))
                                );
                            }
                            assert_eq!(command.as_std().get_args().len(), 0);
                        });
                    },
                );
            }
        }
    }

    #[tokio::test]
    async fn bootstrap_reuses_compatible_node_and_otherwise_uses_pinned_default() {
        use vp_js_runtime::{JsRuntimeProvider, NodeProvider, Platform};
        use vp_shared::{EnvConfig, env_vars};

        let server = httpmock::MockServer::start_async().await;
        let index = server.mock(|when, then| {
            when.path("/index.json");
            then.status(200).json_body(serde_json::json!([
                {"version":"v26.13.0", "lts":"Newer LTS"},
                {"version":"v22.23.1", "lts":"Jod"}
            ]));
        });
        let root = tempfile::tempdir().unwrap();
        EnvConfig::with_vars_async(
            [
                (env_vars::VP_HOME, root.path().to_str().unwrap()),
                (env_vars::VP_NODE_DIST_MIRROR, server.base_url().as_str()),
            ],
            |env| async move {
                // Key-address the seeded pnpm by its pin, so setup reuses it from
                // the cache instead of fetching it from a registry.
                let cache_key = vp_pm_cli::install_cache_key(Some(PINNED_PNPM_INTEGRITY), None);
                let pnpm = vp_pm_cli::package_manager_install_dir(
                    vp_pm_cli::PackageManagerType::Pnpm,
                    PINNED_PNPM_VERSION,
                    cache_key.as_deref(),
                );
                tokio::fs::create_dir_all(pnpm.join("bin")).await.unwrap();
                for name in ["pnpm", "pnpm.cmd", "pnpm.ps1", "pnpm.cjs"] {
                    tokio::fs::write(pnpm.join("bin").join(name), "").await.unwrap();
                }
                let manifest = pnpm.join("package.json");
                tokio::fs::write(&manifest, r#"{"engines":{"node":">=18.12"}}"#).await.unwrap();
                for version in ["22.23.1", DEFAULT_INSTALL_NODE_VERSION] {
                    let binary = env
                        .dirs
                        .data
                        .join("js_runtime/node")
                        .join(version)
                        .join(NodeProvider::new().binary_relative_path(Platform::current()));
                    tokio::fs::create_dir_all(binary.parent().unwrap()).await.unwrap();
                    tokio::fs::write(binary, "").await.unwrap();
                }

                let (runtime, entry) =
                    prepare_install_runtime(Some(NodeVersion::Preferred("22.23.1")), None)
                        .await
                        .unwrap();
                assert_eq!(runtime.version(), "22.23.1");
                assert_eq!(entry, pnpm.join("bin/pnpm.cjs"));
                index.assert_calls(0);

                let (runtime, _) =
                    prepare_install_runtime(Some(NodeVersion::Required("22.23.1")), None)
                        .await
                        .unwrap();
                assert_eq!(runtime.version(), "22.23.1");
                index.assert_calls(0);

                let (runtime, _) = prepare_install_runtime(None, None).await.unwrap();
                assert_eq!(runtime.version(), DEFAULT_INSTALL_NODE_VERSION);
                index.assert_calls(0);

                tokio::fs::write(manifest, r#"{"engines":{"node":">=24"}}"#).await.unwrap();
                let (runtime, _) =
                    prepare_install_runtime(Some(NodeVersion::Preferred("22.23.1")), None)
                        .await
                        .unwrap();
                assert_eq!(runtime.version(), DEFAULT_INSTALL_NODE_VERSION);
                index.assert_calls(0);

                let error = prepare_install_runtime(Some(NodeVersion::Required("22.23.1")), None)
                    .await
                    .unwrap_err();
                assert!(error.to_string().contains(
                    "Node.js 22.23.1 required by the target Vite+ release is not supported by pnpm"
                ));
                index.assert_calls(0);
            },
        )
        .await;
    }

    #[test]
    fn pnpm_node_compatibility_fails_closed() {
        assert!(!supports_pnpm_node(&serde_json::json!({}), "22.23.1"));
        assert!(!supports_pnpm_node(&serde_json::json!({"engines":{"node":"invalid"}}), "22.23.1"));
        assert!(!supports_pnpm_node(&serde_json::json!({"engines":{"node":">=18.12"}}), "invalid"));
    }

    #[tokio::test]
    async fn release_archives_supply_the_frozen_dependency_tree() {
        EnvConfig::with_vars_async([("VP_SETUP_SKIP_INSTALL_LOCKFILE", None::<&str>)], |_| async {
            let temp = tempfile::tempdir().unwrap();
            let directory = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
            let archive = lockfile::test_archive(true, Some(b"release dependency tree"));
            extract_platform_package(&archive, &directory).await.unwrap();
            let args =
                dependency_install_args(&directory, Some("https://mirror.example")).await.unwrap();
            assert_eq!(
                args,
                [
                    "install",
                    "--ignore-workspace",
                    "--frozen-lockfile",
                    "--lockfile=true",
                    "--config.auto-install-peers=true",
                    "--config.exclude-links-from-lockfile=false",
                    "--config.dedupe-peers=false",
                    "--config.inject-workspace-packages=false",
                    "--registry",
                    "https://mirror.example"
                ]
            );
            assert_eq!(
                tokio::fs::read(directory.join("pnpm-lock.yaml")).await.unwrap(),
                b"release dependency tree"
            );
        })
        .await;
    }

    #[tokio::test]
    async fn incomplete_release_archives_fail_but_old_archives_remain_supported() {
        EnvConfig::with_vars_async([("VP_SETUP_SKIP_INSTALL_LOCKFILE", None::<&str>)], |_| async {
            let temp = tempfile::tempdir().unwrap();
            let directory = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
            let error = extract_platform_package(&lockfile::test_archive(true, None), &directory)
                .await
                .unwrap_err();
            assert!(error.to_string().contains("missing vp-install-lock.yaml"));
            extract_platform_package(&lockfile::test_archive(false, None), &directory)
                .await
                .unwrap();
            assert_eq!(
                dependency_install_args(&directory, None).await.unwrap(),
                ["install", "--ignore-workspace"]
            );
        })
        .await;
    }

    #[tokio::test]
    async fn invalidate_only_the_target_versions_setup() {
        let root = tempfile::tempdir().unwrap();
        let root = AbsolutePathBuf::new(root.path().to_path_buf()).unwrap();
        let old = root.join("1.0.0");
        let target = root.join("1.1.0");
        for version in [&old, &target] {
            tokio::fs::create_dir_all(version.join("bin")).await.unwrap();
            tokio::fs::write(version.join("bin").join(crate::SELF_SETUP_MARKER), b"")
                .await
                .unwrap();
        }
        clear_self_setup_marker(&target).await.unwrap();
        assert!(!target.join("bin").join(crate::SELF_SETUP_MARKER).as_path().exists());
        assert!(old.join("bin").join(crate::SELF_SETUP_MARKER).as_path().is_file());
        clear_self_setup_marker(&target).await.unwrap();
    }

    #[test]
    fn forced_active_version_installs_to_unique_semver_dir() {
        let dir = target_install_dir_name("0.1.23", Some("0.1.23"), true);

        assert!(dir.starts_with("0.1.23+force."));
        assert!(js_semver::Version::parse(&dir).is_ok(), "{dir} should remain semver-compatible");
    }

    #[test]
    fn forced_active_reinstall_dir_installs_to_new_unique_dir() {
        let dir = target_install_dir_name("0.1.23", Some("0.1.23+force.1.2"), true);

        assert!(dir.starts_with("0.1.23+force."));
        assert_ne!(dir, "0.1.23+force.1.2");
    }

    #[test]
    fn non_active_or_non_forced_version_uses_plain_version_dir() {
        assert_eq!(target_install_dir_name("0.1.24", Some("0.1.23"), true), "0.1.24");
        assert_eq!(target_install_dir_name("0.1.23", Some("0.1.23"), false), "0.1.23");
        assert_eq!(target_install_dir_name("0.1.23", None, true), "0.1.23");
    }

    #[test]
    fn active_version_detection_matches_plain_and_forced_dirs() {
        assert!(is_install_dir_for_version("0.1.23", "0.1.23"));
        assert!(is_install_dir_for_version("0.1.23+force.1.2", "0.1.23"));
        assert!(!is_install_dir_for_version("0.1.230", "0.1.23"));
        assert!(!is_install_dir_for_version("0.1.24", "0.1.23"));
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn swap_current_link_replaces_a_dangling_windows_junction() {
        let temp = tempfile::tempdir().unwrap();
        let install_dir = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let old_version = install_dir.join("old-version");
        let new_version = install_dir.join("new-version");
        let current_link = install_dir.join("current");

        std::fs::create_dir(&old_version).unwrap();
        std::fs::create_dir(&new_version).unwrap();
        std::fs::write(new_version.join("active"), b"active").unwrap();
        junction::create(&old_version, &current_link).unwrap();
        std::fs::remove_dir(&old_version).unwrap();

        assert!(!current_link.as_path().exists());
        assert!(std::fs::symlink_metadata(&current_link).is_ok());

        swap_current_link(&install_dir, "new-version").await.unwrap();

        assert!(current_link.join("active").as_path().is_file());
    }

    #[test]
    fn test_is_safe_tar_path_normal() {
        assert!(is_safe_tar_path(Path::new("dist/index.js")));
        assert!(is_safe_tar_path(Path::new("bin/vp")));
        assert!(is_safe_tar_path(Path::new("package.json")));
        assert!(is_safe_tar_path(Path::new("templates/react/index.ts")));
    }

    #[test]
    fn test_is_safe_tar_path_traversal() {
        assert!(!is_safe_tar_path(Path::new("../etc/passwd")));
        assert!(!is_safe_tar_path(Path::new("dist/../../etc/passwd")));
        assert!(!is_safe_tar_path(Path::new("..")));
    }

    #[test]
    fn test_is_safe_tar_path_absolute() {
        assert!(!is_safe_tar_path(Path::new("/etc/passwd")));
        assert!(!is_safe_tar_path(Path::new("/usr/bin/vp")));
    }

    /// Select cleanup candidates from dirs created one second apart, oldest first.
    fn removed_versions(
        created_oldest_first: &[&str],
        max_keep: usize,
        protected_versions: &[&str],
    ) -> Vec<std::ffi::OsString> {
        let install_dir = AbsolutePathBuf::new(std::env::temp_dir()).unwrap();
        let versions = created_oldest_first
            .iter()
            .zip(0..)
            .map(|(name, secs)| {
                (UNIX_EPOCH + std::time::Duration::from_secs(secs), install_dir.join(name))
            })
            .collect();
        versions_to_remove(versions, max_keep, protected_versions)
            .iter()
            .map(|path| path.as_path().file_name().unwrap().to_os_string())
            .collect()
    }

    #[test]
    fn test_cleanup_preserves_active_downgraded_version() {
        // Simulate: installed 0.1-0.7 in order, then rolled back to 0.2.0.
        // Keep the top 5, with 0.2.0 protected (the active version).
        let removed = removed_versions(
            &["0.1.0", "0.2.0", "0.3.0", "0.4.0", "0.5.0", "0.6.0", "0.7.0"],
            5,
            &["0.2.0"],
        );

        // 0.2.0 is the active version — it MUST survive cleanup
        assert_eq!(removed, ["0.1.0"], "Active version 0.2.0 must survive cleanup");
    }

    #[test]
    fn test_cleanup_sorts_by_creation_time_not_semver() {
        // Creation order: 0.5.0 (oldest), 0.1.0, 0.3.0, 0.7.0, 0.2.0, 0.6.0 (newest)
        let removed =
            removed_versions(&["0.5.0", "0.1.0", "0.3.0", "0.7.0", "0.2.0", "0.6.0"], 4, &[]);

        // Keep top 4 by creation time → keep 0.6.0, 0.2.0, 0.7.0, 0.3.0
        // Remove 0.1.0 and 0.5.0 (oldest by creation time)
        assert_eq!(removed, ["0.1.0", "0.5.0"]);
    }

    #[test]
    fn test_cleanup_removes_old_forced_reinstall_dir() {
        // Forced reinstall directories use semver build metadata. They should
        // still be eligible for normal old-version cleanup.
        let old_forced = "0.1.23+force.123.456";
        let removed = removed_versions(
            &[old_forced, "0.1.24", "0.1.25", "0.1.26", "0.1.27", "0.1.28"],
            5,
            &[],
        );

        assert_eq!(removed, [old_forced], "old forced reinstall directory should be removed");
    }

    #[test]
    fn test_cleanup_preserves_protected_forced_reinstall_dir() {
        let protected_forced = "0.1.23+force.123.456";
        let removed = removed_versions(
            &[protected_forced, "0.1.24", "0.1.25", "0.1.26", "0.1.27", "0.1.28"],
            5,
            &[protected_forced],
        );

        assert!(
            removed.is_empty(),
            "active forced reinstall directory should be preserved when protected"
        );
    }

    #[tokio::test]
    async fn test_cleanup_removes_only_unprotected_version_dirs() {
        let temp = tempfile::tempdir().unwrap();
        let install_dir = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();

        // Keeping no versions makes the outcome independent of creation order.
        let forced = "0.1.23+force.123.456";
        for name in [forced, "0.1.24", "0.1.25", "current"] {
            tokio::fs::create_dir(install_dir.join(name)).await.unwrap();
        }

        cleanup_old_versions(&install_dir, 0, &["0.1.25"]).await.unwrap();

        assert!(!tokio::fs::try_exists(install_dir.join(forced)).await.unwrap());
        assert!(!tokio::fs::try_exists(install_dir.join("0.1.24")).await.unwrap());
        assert!(
            tokio::fs::try_exists(install_dir.join("0.1.25")).await.unwrap(),
            "protected version must survive cleanup"
        );
        assert!(
            tokio::fs::try_exists(install_dir.join("current")).await.unwrap(),
            "non-version entries must survive cleanup"
        );
    }

    #[tokio::test]
    async fn test_cleanup_old_versions_with_nonexistent_dir() {
        // Verifies that cleanup_old_versions propagates errors on non-existent dir.
        // In the real flow, such errors from post-swap operations should be non-fatal.
        let non_existent =
            AbsolutePathBuf::new(std::env::temp_dir().join("non-existent-upgrade-test-dir"))
                .unwrap();
        let result = cleanup_old_versions(&non_existent, 5, &[]).await;
        assert!(result.is_err(), "cleanup_old_versions should error on non-existent dir");
    }

    #[tokio::test]
    async fn install_lock_creates_data_dir_and_locks_expected_file() {
        let temp = tempfile::tempdir().unwrap();
        let data = AbsolutePathBuf::new(temp.path().join("data")).unwrap();

        let lock = lock_install_dir(&data).await.unwrap();
        let contender = std::fs::File::open(data.join(INSTALL_LOCK_FILE)).unwrap();
        assert!(matches!(contender.try_lock(), Err(std::fs::TryLockError::WouldBlock)));
        drop(lock);
        assert!(contender.try_lock().is_ok());
    }

    #[tokio::test]
    async fn failed_install_removes_only_a_directory_it_created() {
        let temp = tempfile::tempdir().unwrap();
        let data = AbsolutePathBuf::new(temp.path().join("data")).unwrap();

        let fresh = data.join("1.2.0");
        let created = create_version_dir(&fresh).await.unwrap();
        assert!(created);
        discard_failed_version_dir(&fresh, created).await;
        assert!(!fresh.as_path().exists());

        // A kept version or rollback target survives a failed reinstall.
        let existing = data.join("1.1.0");
        std::fs::create_dir_all(existing.join("bin")).unwrap();
        std::fs::write(existing.join("bin").join("vp"), "kept").unwrap();
        let created = create_version_dir(&existing).await.unwrap();
        assert!(!created);
        discard_failed_version_dir(&existing, created).await;
        assert_eq!(std::fs::read_to_string(existing.join("bin").join("vp")).unwrap(), "kept");
    }

    #[tokio::test]
    async fn test_write_upgrade_log_creates_log_in_parent_dir() {
        let temp = tempfile::tempdir().unwrap();
        // Simulate a `<DATA>/0.1.15/` install structure
        let version_dir = AbsolutePathBuf::new(temp.path().join("0.1.15").to_path_buf()).unwrap();
        tokio::fs::create_dir(&version_dir).await.unwrap();

        let result = write_upgrade_log(&version_dir, 17, false, b"", b"ENOSPC: private path").await;
        assert!(result.is_some(), "write_upgrade_log should return log path");

        let log_path = result.unwrap();
        // Log should be in parent dir, not version_dir
        assert_eq!(
            log_path.as_path().parent().unwrap(),
            temp.path(),
            "upgrade.log should be in parent dir"
        );
        assert!(log_path.as_path().exists(), "upgrade.log should exist");

        let content = tokio::fs::read_to_string(&log_path).await.unwrap();
        assert!(content.contains("exit code 17"));
        assert!(content.contains("Reported error: ENOSPC."));
        assert!(!content.contains("private path"));
        assert!(content.contains("Registry output is not saved"));

        // Log should survive version_dir removal
        tokio::fs::remove_dir_all(&version_dir).await.unwrap();
        assert!(log_path.as_path().exists(), "upgrade.log should survive version_dir cleanup");
    }

    #[test]
    fn install_failure_log_distinguishes_safe_causes_with_the_same_exit_code() {
        let cases = [
            ("ERR_PNPM_FETCH_401", "Check npm registry authentication"),
            ("ERR_PNPM_OUTDATED_LOCKFILE", "The installation lockfile does not match"),
            ("ENOSPC", "Free disk space before retrying"),
        ];
        for (code, hint) in cases {
            let output = format!(
                "{code}: https://private-user:private-password@registry.example.test/private-package?token=private-token\nAuthorization: Bearer private-bearer\n_authToken=private-config-token\n"
            );
            for (stdout, stderr) in [(output.as_bytes(), &[][..]), (&[][..], output.as_bytes())] {
                let content = install_failure_log(1, false, stdout, stderr);
                assert!(content.contains("Stage: production dependency installation."));
                assert!(content.contains("exit code 1"));
                assert!(content.contains(&format!("Reported error: {code}.")));
                assert!(content.contains(hint));
                assert!(!content.contains("private-"));
                assert!(!content.contains("registry.example.test"));
                assert!(!content.contains("Authorization"));
                assert!(!content.contains("_authToken"));
                if code != "ERR_PNPM_FETCH_401" {
                    assert!(!content.contains("authentication"));
                }
            }
        }
    }

    #[test]
    fn install_failure_log_points_the_system_pnpm_to_the_lockfile_bypass() {
        for code in SYSTEM_PNPM_LOCKFILE_DIAGNOSTICS {
            let output = format!("{code}: private detail");
            let managed = install_failure_log(1, false, output.as_bytes(), b"");
            assert!(managed.contains("Retry with an updated Vite+ release."), "{managed}");
            assert!(!managed.contains("VP_SETUP_SKIP_INSTALL_LOCKFILE"), "{managed}");
            let system = install_failure_log(1, true, output.as_bytes(), b"");
            assert!(system.contains(&format!("Reported error: {code}.")), "{system}");
            assert!(system.contains("Retry with VP_SETUP_SKIP_INSTALL_LOCKFILE=1."), "{system}");
            assert!(!system.contains("updated Vite+ release"), "{system}");
            assert!(!system.contains("private"), "{system}");
        }
        // Other hints do not depend on the pnpm.
        let system = install_failure_log(1, true, b"ENOSPC", b"");
        assert!(system.contains("Free disk space before retrying the command."), "{system}");
    }

    #[test]
    fn install_failure_log_deduplicates_known_codes_from_both_streams() {
        let content = install_failure_log(
            1,
            false,
            b"ERR_PNPM_FETCH_401 Unauthorized\nERR_PNPM_FETCH_401 Unauthorized",
            b"ENOSPC: no space\nERR_PNPM_FETCH_401 Unauthorized",
        );
        assert_eq!(content.matches("Reported error: ERR_PNPM_FETCH_401.").count(), 1);
        assert_eq!(content.matches("Reported error: ENOSPC.").count(), 1);
    }

    #[test]
    fn install_failure_log_omits_unknown_output_and_partial_identifiers() {
        for output in [
            &b""[..],
            b"ERR_PNPM_PRIVATE_SECRET=private-token\nhttps://private-user:private-password@example.test/",
            b"PREFIX_ERR_PNPM_FETCH_401 ERR_PNPM_FETCH_401_PRIVATE ENOSPC_PRIVATE",
            b"\xff\xfeprivate-token",
        ] {
            let content = install_failure_log(1, false, output, output);
            assert!(content.contains("No recognized error identifier was reported."));
            assert!(!content.contains("Reported error:"));
            assert!(!content.contains("private"));
            assert!(!content.contains("SECRET"));
            assert!(!content.contains("authentication"));
        }
    }

    #[tokio::test]
    async fn test_write_release_age_overrides_creates_npmrc() {
        let temp = tempfile::tempdir().unwrap();
        let version_dir = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();

        write_release_age_overrides(&version_dir).await.unwrap();

        // .npmrc (pnpm only — packageManager pins pnpm)
        let npmrc = tokio::fs::read_to_string(version_dir.join(".npmrc")).await.unwrap();
        assert!(npmrc.contains("minimum-release-age=0"), ".npmrc should contain pnpm override");

        // No .yarnrc.yml or bunfig.toml (pnpm only)
        assert!(
            !version_dir.join(".yarnrc.yml").as_path().exists(),
            ".yarnrc.yml should not be created"
        );
        assert!(
            !version_dir.join("bunfig.toml").as_path().exists(),
            "bunfig.toml should not be created"
        );
    }

    /// Build a fake managed Node.js runtime that records how `run_pnpm_install`
    /// invokes it: arguments to `invocation.txt`, `$PATH` to `path.txt`, and
    /// `$npm_config_registry` to `registry.txt` (all relative to `version_dir`).
    #[cfg(unix)]
    async fn fake_pnpm_runtime(
        version_dir: &AbsolutePath,
    ) -> (AbsolutePathBuf, AbsolutePathBuf, vp_js_runtime::JsRuntime, AbsolutePathBuf) {
        let node_bin = version_dir.join("node").join("bin");
        let pnpm_bin = version_dir.join("pnpm").join("bin");
        tokio::fs::create_dir_all(&node_bin).await.unwrap();
        tokio::fs::create_dir_all(&pnpm_bin).await.unwrap();

        let node_binary = node_bin.join("node");
        // Reuse a checked-in executable: concurrent test processes can inherit a freshly
        // written script's write descriptor and cause ETXTBSY on Linux.
        tokio::fs::symlink(
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/fake-node.sh"),
            &node_binary,
        )
        .await
        .unwrap();
        let pnpm_entry = pnpm_bin.join("pnpm.cjs");
        tokio::fs::write(&pnpm_entry, "").await.unwrap();
        let node_runtime = vp_js_runtime::JsRuntime::from_system(JsRuntimeType::Node, node_binary);

        (node_bin, pnpm_bin, node_runtime, pnpm_entry)
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn run_pnpm_install_uses_managed_node_directly() {
        let temp = tempfile::tempdir().unwrap();
        let version_dir = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let (node_bin, pnpm_bin, node_runtime, pnpm_entry) = fake_pnpm_runtime(&version_dir).await;

        let output = run_pnpm_install(&version_dir, &node_runtime, &pnpm_entry, &["install"], None)
            .await
            .unwrap();
        assert!(output.status.success());

        let invocation =
            tokio::fs::read_to_string(version_dir.join("invocation.txt")).await.unwrap();
        assert_eq!(invocation, format!("{}\ninstall\n", pnpm_entry.as_path().display()));

        let path = tokio::fs::read_to_string(version_dir.join("path.txt")).await.unwrap();
        let path_entries = env::split_paths(&path).collect::<Vec<_>>();
        assert_eq!(path_entries[0], node_bin.as_path());
        assert_eq!(path_entries[1], pnpm_bin.as_path());

        // Without a custom registry, npm_config_registry must not be set so pnpm
        // falls back to its default registry.
        let registry = tokio::fs::read_to_string(version_dir.join("registry.txt")).await.unwrap();
        assert_eq!(registry, "");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn run_pnpm_install_passes_custom_registry_env() {
        let temp = tempfile::tempdir().unwrap();
        let version_dir = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let (_, _, node_runtime, pnpm_entry) = fake_pnpm_runtime(&version_dir).await;

        let registry_url = "https://registry.example.com/";
        let output = run_pnpm_install(
            &version_dir,
            &node_runtime,
            &pnpm_entry,
            &["install"],
            Some(registry_url),
        )
        .await
        .unwrap();
        assert!(output.status.success());

        let registry = tokio::fs::read_to_string(version_dir.join("registry.txt")).await.unwrap();
        assert_eq!(registry, registry_url);
    }

    /// A bin directory with a system `node` that reports `node_version` and a
    /// `#!/usr/bin/env node` pnpm (see `tests/fixtures/fake-system-node.sh`).
    #[cfg(unix)]
    async fn fake_system_bin(dir: &AbsolutePath, node_version: &str) -> AbsolutePathBuf {
        let bin = dir.join("system-bin");
        tokio::fs::create_dir_all(&bin).await.unwrap();
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        tokio::fs::symlink(fixtures.join("fake-system-node.sh"), bin.join("node")).await.unwrap();
        tokio::fs::symlink(fixtures.join("fake-system-pnpm"), bin.join("pnpm")).await.unwrap();
        tokio::fs::write(bin.join("node-version"), format!("v{node_version}\n")).await.unwrap();
        bin
    }

    #[cfg(unix)]
    fn join_path(entries: &[&AbsolutePath]) -> std::ffi::OsString {
        env::join_paths(entries.iter().map(|entry| entry.as_path())).unwrap()
    }

    fn env_forbidden(home: &Path) -> [(&'static str, Option<&str>); 2] {
        [
            (vp_shared::env_vars::VP_HOME, home.to_str()),
            (vp_shared::env_vars::VP_ENV_FORBIDDEN, Some("1")),
        ]
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn env_forbidden_installs_with_the_system_node_and_pnpm() {
        let temp = tempfile::tempdir().unwrap();
        let root = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let version_dir = root.join("version");
        tokio::fs::create_dir_all(version_dir.join("bin")).await.unwrap();
        let bin = fake_system_bin(&root, "24.22.1").await;
        let home = root.join("home");

        Box::pin(EnvConfig::with_vars_async(env_forbidden(home.as_path()), |env| async move {
            let tools =
                find_system_install_tools(&join_path(&[&bin]), &root, None, Some(">=22.18.0"))
                    .await
                    .unwrap();
            install_production_deps(&version_dir, None, false, Some(NodeVersion::System(&tools)))
                .await
                .unwrap();

            // pnpm's shebang ran the checked system node.
            let invocation =
                tokio::fs::read_to_string(version_dir.join("invocation.txt")).await.unwrap();
            let lines = invocation.lines().collect::<Vec<_>>();
            assert_eq!(lines[0], "SYSTEM_NODE");
            assert_eq!(lines[1], bin.join("pnpm").as_path().to_str().unwrap());
            assert_eq!(lines[2..4], ["install", "--ignore-workspace"]);
            assert!(lines.contains(&"--config.manage-package-manager-versions=false"), "{lines:?}");
            assert!(lines.contains(&"--ignore-scripts"), "{lines:?}");
            let path = tokio::fs::read_to_string(version_dir.join("path.txt")).await.unwrap();
            assert_eq!(path, bin.as_path().to_str().unwrap());
            for managed in ["js_runtime", "package_manager"] {
                assert!(!env.dirs.data.join(managed).as_path().exists(), "{managed} was created");
            }
        }))
        .await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn env_forbidden_rejects_tools_other_than_the_first_system_match() {
        let temp = tempfile::tempdir().unwrap();
        let root = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let bin = fake_system_bin(&root, "24.22.1").await;
        let home = root.join("home");
        let node_only = root.join("node-only");
        tokio::fs::create_dir_all(&node_only).await.unwrap();
        tokio::fs::symlink(bin.join("node"), node_only.join("node")).await.unwrap();
        tokio::fs::copy(bin.join("node-version"), node_only.join("node-version")).await.unwrap();
        // A Unix shim is a link to a file named `vp`.
        let shims = root.join("shims");
        tokio::fs::create_dir_all(&shims).await.unwrap();
        tokio::fs::copy(bin.join("node"), shims.join("vp")).await.unwrap();
        tokio::fs::symlink(shims.join("vp"), shims.join("node")).await.unwrap();
        let managed = home.join("js_runtime/node/24.22.1/bin");
        tokio::fs::create_dir_all(&managed).await.unwrap();
        tokio::fs::copy(bin.join("node"), managed.join("node")).await.unwrap();

        EnvConfig::with_vars_async(env_forbidden(home.as_path()), |_| async move {
            let error = |path: std::ffi::OsString, range: Option<&'static str>| {
                let root = root.clone();
                async move {
                    find_system_install_tools(&path, &root, None, range)
                        .await
                        .unwrap_err()
                        .to_string()
                }
            };
            let empty = root.join("empty");
            assert!(error(join_path(&[&empty]), None).await.contains("no 'node' is on PATH"));
            assert!(error(join_path(&[&node_only]), None).await.contains("no 'pnpm' is on PATH"));
            // A later system match is never used.
            let shim = error(join_path(&[&shims, &bin]), None).await;
            assert!(shim.contains("run `vp env off node`"), "{shim}");
            let owned = error(join_path(&[&managed, &bin]), None).await;
            assert!(owned.contains("the Vite+ managed install"), "{owned}");
            let old = error(join_path(&[&bin]), Some(">=26.0.0")).await;
            assert!(old.contains("reports '24.22.1', which does not satisfy '>=26.0.0'"), "{old}");
        })
        .await;
    }

    #[tokio::test]
    async fn env_forbidden_requires_checked_system_tools() {
        let temp = tempfile::tempdir().unwrap();
        let root = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let home = root.join("home");
        Box::pin(EnvConfig::with_vars_async(env_forbidden(home.as_path()), |env| async move {
            for node_version in [None, Some(NodeVersion::Preferred("24.22.1"))] {
                let error = install_production_deps(&root, None, false, node_version)
                    .await
                    .unwrap_err()
                    .to_string();
                assert!(error.contains("only with a checked system node and pnpm"), "{error}");
            }
            assert!(!env.dirs.data.join("package_manager").as_path().exists());
        }))
        .await;
    }

    #[tokio::test]
    async fn wrapper_omits_the_pnpm_pin_only_under_env_forbidden() {
        let temp = tempfile::tempdir().unwrap();
        let root = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        for (value, pinned) in [(Some("1"), false), (Some("true"), true), (None, true)] {
            EnvConfig::with_vars_async(
                [(vp_shared::env_vars::VP_ENV_FORBIDDEN, value)],
                |_| async {
                    generate_wrapper_package_json(&root, "1.2.3").await.unwrap();
                },
            )
            .await;
            let manifest: serde_json::Value =
                serde_json::from_slice(&tokio::fs::read(root.join("package.json")).await.unwrap())
                    .unwrap();
            assert_eq!(manifest["dependencies"]["vite-plus"], "1.2.3");
            let expected =
                pinned.then(|| format!("pnpm@{PINNED_PNPM_VERSION}+{PINNED_PNPM_INTEGRITY}"));
            assert_eq!(manifest["packageManager"].as_str(), expected.as_deref(), "{value:?}");
        }
    }
}
