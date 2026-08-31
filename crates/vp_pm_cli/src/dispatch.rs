//! Resolves and executes a parsed package-manager command.
//!
//! Callers must perform any environment setup (PATH adjustments, runtime
//! download) before invoking [`dispatch`].

use std::process::ExitStatus;

use vt_path::AbsolutePath;

use crate::{
    EnvironmentPackageManagerResolution, PackageManager,
    cli::PackageManagerCommand,
    download_package_manager,
    error::Error,
    helpers::build_package_manager_or_npm_default,
    resolution::{DlxArgs, run_resolution},
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
    let command = match command {
        PackageManagerCommand::Dlx(args) => {
            let manager = match source {
                ManagerSource::Detect => return dispatch_dlx(cwd, args, render_diagnostics).await,
                ManagerSource::Environment(package_manager) => {
                    build_selected_package_manager(package_manager).await?
                }
                ManagerSource::Resolved(manager) => manager,
            };
            let resolution = PackageManagerCommand::Dlx(args).resolve_for_manager(&manager)?;
            let status = run_resolution(cwd, resolution, render_diagnostics).await?;
            return Ok(DispatchResult { status, why_hint_packages: None });
        }
        command => command,
    };

    let manager = match source {
        ManagerSource::Detect => build_package_manager_or_npm_default(cwd).await?,
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

async fn dispatch_dlx(
    cwd: &AbsolutePath,
    args: DlxArgs,
    render_diagnostics: bool,
) -> Result<DispatchResult, Error> {
    match PackageManager::builder(cwd).build_with_default().await {
        Ok(manager) => {
            let resolution = PackageManagerCommand::Dlx(args).resolve_for_manager(&manager)?;
            let status = run_resolution(cwd, resolution, render_diagnostics).await?;
            Ok(DispatchResult { status, why_hint_packages: None })
        }
        Err(vp_error::Error::WorkspaceError(vt_workspace::Error::PackageJsonNotFound(_))) => {
            let status =
                run_resolution(cwd, args.resolve_npx_fallback(), render_diagnostics).await?;
            Ok(DispatchResult { status, why_hint_packages: None })
        }
        Err(error) => Err(Error::Install(error)),
    }
}
