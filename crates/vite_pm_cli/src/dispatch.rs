//! Resolves and executes a parsed package-manager command.
//!
//! Callers must perform any environment setup (PATH adjustments, runtime
//! download) before invoking [`dispatch`].

use std::process::ExitStatus;

use vite_path::AbsolutePath;

use crate::{
    PackageManager,
    cli::{PackageManagerCommand, PmCommand},
    error::Error,
    helpers::{build_package_manager, build_package_manager_or_npm_default, ensure_package_json},
    resolution::{DlxArgs, StageCommand, run_resolution},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManagerPolicy {
    CreateIfMissing,
    RequireProject,
    AllowNpmFallback,
}

pub async fn dispatch(
    cwd: &AbsolutePath,
    command: PackageManagerCommand,
) -> Result<ExitStatus, Error> {
    let render_diagnostics = command.should_render_diagnostics();
    let command = match command {
        PackageManagerCommand::Dlx(args) => {
            return dispatch_dlx(cwd, args, render_diagnostics).await;
        }
        command => command,
    };

    let manager = match manager_policy(&command) {
        ManagerPolicy::CreateIfMissing => {
            ensure_package_json(cwd).await?;
            build_package_manager(cwd).await?
        }
        ManagerPolicy::RequireProject => build_package_manager(cwd).await?,
        ManagerPolicy::AllowNpmFallback => build_package_manager_or_npm_default(cwd).await?,
    };

    let resolution = command.resolve_for_manager(&manager)?;
    run_resolution(cwd, resolution, render_diagnostics).await
}

async fn dispatch_dlx(
    cwd: &AbsolutePath,
    args: DlxArgs,
    render_diagnostics: bool,
) -> Result<ExitStatus, Error> {
    match PackageManager::builder(cwd).build_with_default().await {
        Ok(manager) => {
            let resolution = PackageManagerCommand::Dlx(args).resolve_for_manager(&manager)?;
            run_resolution(cwd, resolution, render_diagnostics).await
        }
        Err(vite_error::Error::WorkspaceError(vite_workspace::Error::PackageJsonNotFound(_))) => {
            run_resolution(cwd, args.resolve_npx_fallback(), render_diagnostics).await
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
        PmCommand::ApproveBuilds(_)
        | PmCommand::Prune(_)
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
