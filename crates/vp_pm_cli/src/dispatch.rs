//! Resolves and executes a parsed package-manager command.
//!
//! Callers must perform any environment setup (PATH adjustments, runtime
//! download) before invoking [`dispatch`].

use std::process::ExitStatus;

use semver::Version;
use vp_shared::{PrependOptions, ToolPathEnv};
use vt_path::AbsolutePath;
use vt_str::Str;

use crate::{
    EnvironmentPackageManagerResolution, PackageManager, PackageManagerType,
    cli::PackageManagerCommand, download_package_manager_for_cwd, error::Error,
    resolution::run_resolution,
};

#[derive(Debug)]
pub struct DispatchResult {
    pub status: ExitStatus,
    pub why_hint_packages: Option<Vec<String>>,
}

enum ManagerSource<'a> {
    Detect,
    Environment(&'a EnvironmentPackageManagerResolution),
    Resolved(PackageManager),
}

pub async fn dispatch(
    cwd: &AbsolutePath,
    command: PackageManagerCommand,
) -> Result<ExitStatus, Error> {
    Ok(dispatch_with_metadata(cwd, command).await?.status)
}

pub async fn dispatch_with_metadata(
    cwd: &AbsolutePath,
    command: PackageManagerCommand,
) -> Result<DispatchResult, Error> {
    dispatch_with_manager(cwd, command, ManagerSource::Detect).await
}

pub async fn dispatch_with_package_manager(
    cwd: &AbsolutePath,
    command: PackageManagerCommand,
    package_manager: &EnvironmentPackageManagerResolution,
) -> Result<DispatchResult, Error> {
    dispatch_with_manager(cwd, command, ManagerSource::Environment(package_manager)).await
}

pub async fn dispatch_with_resolved_package_manager(
    cwd: &AbsolutePath,
    command: PackageManagerCommand,
    manager: PackageManager,
) -> Result<DispatchResult, Error> {
    dispatch_with_manager(cwd, command, ManagerSource::Resolved(manager)).await
}

async fn dispatch_with_manager(
    cwd: &AbsolutePath,
    command: PackageManagerCommand,
    source: ManagerSource<'_>,
) -> Result<DispatchResult, Error> {
    let render_diagnostics = command.should_render_diagnostics();
    let mut manager = match source {
        ManagerSource::Detect => {
            PackageManager::builder(cwd)
                .package_manager_type(PackageManagerType::Pnpm)
                .build()
                .await?
        }
        ManagerSource::Environment(package_manager) => {
            build_selected_package_manager(cwd, package_manager).await?
        }
        ManagerSource::Resolved(manager) => manager,
    };
    if manager.client == PackageManagerType::Yarn {
        // The selected launcher can delegate through yarnPath before executing a command.
        manager.version = effective_yarn_version(cwd, &manager).await?;
    }
    let package_manager = manager.client;
    let why_hint_packages = command.why_hint_packages(package_manager).map(<[String]>::to_vec);
    let resolution = command.resolve_for_manager(&manager)?;
    let status = run_resolution(cwd, resolution, render_diagnostics).await?;
    Ok(DispatchResult { status, why_hint_packages })
}

async fn effective_yarn_version(
    cwd: &AbsolutePath,
    manager: &PackageManager,
) -> Result<Str, Error> {
    let mut env = ToolPathEnv::from_env();
    env.prepend(manager.get_bin_prefix(), &manager.bin_names(), PrependOptions::default())?;
    let executable =
        vp_command::resolve_bin("yarn", Some(manager.bin_prefix.as_path().as_os_str()), cwd)?;
    let (program, prefix_args) = vp_command::rewrite_cmd_to_powershell(&executable)
        .unwrap_or_else(|| (executable, Vec::new()));
    let output = vp_command::build_command(&program, cwd)
        .args(prefix_args)
        .arg("--version")
        .envs(env.into_envs())
        // User preloads can print to stdout; only the actual command should run them.
        .env_remove("NODE_OPTIONS")
        .output()
        .await?;
    if !output.status.success() {
        return Err(Error::UserMessage(vt_str::format!(
            "Failed to read Yarn version:\n{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        )));
    }
    let version = String::from_utf8_lossy(&output.stdout);
    let version = version.trim();
    let version =
        Version::parse(version).map_err(|source| Error::InvalidPackageManagerVersion {
            manager: manager.client,
            version: version.to_owned(),
            source,
        })?;
    Ok(version.to_string().into())
}

async fn build_selected_package_manager(
    cwd: &AbsolutePath,
    package_manager: &EnvironmentPackageManagerResolution,
) -> Result<PackageManager, Error> {
    let (install_dir, _, version) = download_package_manager_for_cwd(
        cwd,
        package_manager.package_manager_type,
        &package_manager.version,
        package_manager.hash.as_deref(),
    )
    .await
    .map_err(Error::Install)?;
    Ok(PackageManager::from_install_dir(package_manager.package_manager_type, version, install_dir))
}
