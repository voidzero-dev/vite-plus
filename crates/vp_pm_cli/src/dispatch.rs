//! Resolves and executes a parsed package-manager command.
//!
//! Callers must perform any environment setup (PATH adjustments, runtime
//! download) before invoking [`dispatch`].

use std::process::ExitStatus;

use vt_path::AbsolutePath;

use crate::{
    EnvironmentPackageManagerResolution, PackageManager, PackageManagerType,
    cli::PackageManagerCommand, download_package_manager, error::Error, resolution::run_resolution,
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
    let manager = match source {
        ManagerSource::Detect => {
            PackageManager::builder(cwd)
                .package_manager_type(PackageManagerType::Pnpm)
                .build()
                .await?
        }
        ManagerSource::Environment(package_manager) => {
            build_selected_package_manager(package_manager).await?
        }
        ManagerSource::Resolved(manager) => manager,
    };
    let package_manager = manager.client;
    let why_hint_packages = command.why_hint_packages(package_manager).map(<[String]>::to_vec);
    let resolution = command.resolve_for_manager(&manager)?;
    let status = run_resolution(cwd, resolution, render_diagnostics).await?;
    Ok(DispatchResult { status, why_hint_packages })
}

async fn build_selected_package_manager(
    package_manager: &EnvironmentPackageManagerResolution,
) -> Result<PackageManager, Error> {
    let (install_dir, _, version) = download_package_manager(
        package_manager.package_manager_type,
        &package_manager.version,
        package_manager.hash.as_deref(),
    )
    .await
    .map_err(Error::Install)?;
    Ok(PackageManager::from_install_dir(package_manager.package_manager_type, version, install_dir))
}
