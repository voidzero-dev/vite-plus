//! Resolves and executes a parsed package-manager command.
//!
//! Callers must perform any environment setup (PATH adjustments, runtime
//! download) before invoking [`dispatch`].

use std::process::ExitStatus;

use vt_path::AbsolutePath;

use crate::{
    EnvironmentPackageManagerResolution, PackageManager,
    cli::{PackageManagerCommand, PmCommand},
    download_package_manager,
    error::Error,
    helpers::{
        build_package_manager, build_package_manager_or_npm_default, ensure_package_json,
        require_package_json,
    },
    resolution::{DlxArgs, StageCommand, run_resolution},
};

#[derive(Debug)]
pub struct DispatchResult {
    pub status: ExitStatus,
    pub why_hint_packages: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManagerPolicy {
    CreateIfMissing,
    RequireProject,
    AllowNpmFallback,
}

enum ManagerSource<'a> {
    Detect,
    Environment(&'a EnvironmentPackageManagerResolution),
    ResolvedEnvironment(PackageManager, &'a EnvironmentPackageManagerResolution),
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
    package_manager: &EnvironmentPackageManagerResolution,
) -> Result<DispatchResult, Error> {
    dispatch_with_manager(
        cwd,
        command,
        ManagerSource::ResolvedEnvironment(manager, package_manager),
    )
    .await
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
                source => resolve_manager(cwd, source).await?,
            };
            let resolution = PackageManagerCommand::Dlx(args).resolve_for_manager(&manager)?;
            let status = run_resolution(cwd, resolution, render_diagnostics).await?;
            return Ok(DispatchResult { status, why_hint_packages: None });
        }
        command => command,
    };

    let policy = manager_policy(&command);
    match policy {
        ManagerPolicy::CreateIfMissing => ensure_package_json(cwd).await?,
        ManagerPolicy::RequireProject => require_package_json(cwd)?,
        ManagerPolicy::AllowNpmFallback => {}
    }

    let manager = match source {
        ManagerSource::Detect => match policy {
            ManagerPolicy::CreateIfMissing | ManagerPolicy::RequireProject => {
                build_package_manager(cwd).await?
            }
            ManagerPolicy::AllowNpmFallback => build_package_manager_or_npm_default(cwd).await?,
        },
        source => resolve_manager(cwd, source).await?,
    };
    let package_manager = manager.client;
    let why_hint_packages = command.why_hint_packages(package_manager).map(<[String]>::to_vec);
    let resolution = command.resolve_for_manager(&manager)?;
    let status = run_resolution(cwd, resolution, render_diagnostics).await?;
    Ok(DispatchResult { status, why_hint_packages })
}

async fn resolve_manager(
    cwd: &AbsolutePath,
    source: ManagerSource<'_>,
) -> Result<PackageManager, Error> {
    match source {
        ManagerSource::Environment(package_manager) => {
            let manager = build_selected_package_manager(package_manager).await?;
            auto_pin_environment_package_manager(cwd, package_manager, &manager).await?;
            Ok(manager)
        }
        ManagerSource::ResolvedEnvironment(manager, package_manager) => {
            auto_pin_environment_package_manager(cwd, package_manager, &manager).await?;
            Ok(manager)
        }
        ManagerSource::Detect => unreachable!("detected managers are resolved from the cwd"),
    }
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
    Ok(PackageManager {
        client: package_manager.package_manager_type,
        version,
        bin_prefix: install_dir.join("bin"),
    })
}

async fn auto_pin_environment_package_manager(
    cwd: &AbsolutePath,
    resolution: &EnvironmentPackageManagerResolution,
    manager: &PackageManager,
) -> Result<(), Error> {
    if matches!(resolution.source.as_str(), "lockfile or config" | "default") {
        let project_root = resolution.project_root.clone().or_else(|| {
            vt_workspace::find_workspace_root(cwd)
                .ok()
                .map(|(workspace, _)| workspace.path.to_absolute_path_buf())
        });
        let Some(project_root) = project_root else {
            return Ok(());
        };
        super::package_manager::set_dev_engines_package_manager_field(
            &project_root.join("package.json"),
            resolution.package_manager_type,
            &manager.version,
        )
        .await?;
    }
    Ok(())
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

fn manager_policy(command: &PackageManagerCommand) -> ManagerPolicy {
    match command {
        PackageManagerCommand::Install(_) | PackageManagerCommand::Add(_) => {
            ManagerPolicy::CreateIfMissing
        }
        PackageManagerCommand::Remove(_)
        | PackageManagerCommand::Update(_)
        | PackageManagerCommand::Dedupe(_)
        | PackageManagerCommand::Outdated(_)
        | PackageManagerCommand::Why(_)
        | PackageManagerCommand::Link(_)
        | PackageManagerCommand::Unlink(_) => ManagerPolicy::RequireProject,
        PackageManagerCommand::Info(_) => ManagerPolicy::AllowNpmFallback,
        PackageManagerCommand::Dlx(_) => {
            unreachable!("dlx commands are dispatched before manager policy selection")
        }
        PackageManagerCommand::Pm(command) => pm_manager_policy(command),
    }
}

fn pm_manager_policy(command: &PmCommand) -> ManagerPolicy {
    match command {
        PmCommand::Ci(_)
        | PmCommand::ApproveBuilds(_)
        | PmCommand::Prune(_)
        | PmCommand::Patch(_)
        | PmCommand::PatchCommit(_)
        | PmCommand::Pack(_)
        | PmCommand::List(_)
        | PmCommand::Version(_)
        | PmCommand::Publish(_)
        | PmCommand::Rebuild(_)
        | PmCommand::Fund(_)
        | PmCommand::Audit(_)
        | PmCommand::Stage(StageCommand::Publish { .. }) => ManagerPolicy::RequireProject,
        PmCommand::View(_)
        | PmCommand::Stage(_)
        | PmCommand::Owner(_)
        | PmCommand::Cache(_)
        | PmCommand::Config(_)
        | PmCommand::Login(_)
        | PmCommand::Logout(_)
        | PmCommand::Whoami(_)
        | PmCommand::Token(_)
        | PmCommand::DistTag(_)
        | PmCommand::Deprecate(_)
        | PmCommand::Search(_)
        | PmCommand::Ping(_) => ManagerPolicy::AllowNpmFallback,
    }
}

#[cfg(test)]
mod tests {
    use clap::{FromArgMatches, Subcommand};

    use super::*;

    fn parse_command(args: &[&str]) -> PackageManagerCommand {
        let mut command = PackageManagerCommand::augment_subcommands(clap::Command::new("vp"));
        let matches = command.try_get_matches_from_mut(args).unwrap();
        PackageManagerCommand::from_arg_matches(&matches).unwrap()
    }

    #[test]
    fn manager_policy_covers_project_creation_and_requirement() {
        assert_eq!(
            manager_policy(&parse_command(&["vp", "install"])),
            ManagerPolicy::CreateIfMissing
        );
        assert_eq!(
            manager_policy(&parse_command(&["vp", "remove", "react"])),
            ManagerPolicy::RequireProject
        );
    }

    #[test]
    fn manager_policy_covers_npm_fallbacks() {
        assert_eq!(
            manager_policy(&parse_command(&["vp", "info", "react"])),
            ManagerPolicy::AllowNpmFallback
        );
    }

    #[test]
    fn only_stage_publish_requires_a_project() {
        assert_eq!(
            manager_policy(&parse_command(&["vp", "pm", "stage", "publish"])),
            ManagerPolicy::RequireProject
        );
        assert_eq!(
            manager_policy(&parse_command(&["vp", "pm", "stage", "list"])),
            ManagerPolicy::AllowNpmFallback
        );
    }
}
