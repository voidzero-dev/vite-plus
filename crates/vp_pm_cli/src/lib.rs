//! Package-manager infrastructure for `vp`.
//!
//! [`PackageManager`] detects and downloads the selected package manager.
//! [`PackageManagerCommand`] provides the shared clap surface, and [`dispatch`]
//! resolves and executes it. Managed Node.js runtimes and managed global
//! packages remain owned by the global CLI.

#![allow(clippy::allow_attributes, clippy::disallowed_types)]

mod cli;
mod config;
mod dispatch;
mod error;
mod helpers;
mod package_manager;
mod request;
pub(crate) mod resolution;
mod shim;

pub use cli::{ManagedGlobalCommand, PackageManagerCommand, PmCommand};
pub use config::npm_registry;
pub use dispatch::{DispatchResult, dispatch, dispatch_with_metadata};
pub use error::Error;
pub use helpers::build_package_manager;
pub use package_manager::{
    PackageManager, PackageManagerBuilder, PackageManagerResolution, PackageManagerSource,
    PackageManagerType, download_package_manager, ensure_package_manager_bin,
    get_package_manager_type_and_version, package_manager_bin_path, package_manager_install_dir,
    resolve_package_manager_from_package_json,
};
pub use request::HttpClient;
pub use resolution::{
    AddArgs, ApproveBuildsArgs, AuditArgs, CacheArgs, ConfigCommand, DedupeArgs, DeprecateArgs,
    DistTagCommand, DlxArgs, FundArgs, InstallArgs, LinkArgs, ListArgs, LoginArgs, LogoutArgs,
    OutdatedArgs, OutdatedFormat, OwnerCommand, PackArgs, PingArgs, PruneArgs, PublishArgs,
    RebuildArgs, RemoveArgs, SearchArgs, StageCommand, TokenCommand, UnlinkArgs, UpdateArgs,
    VersionArgs, ViewArgs, WhoamiArgs, WhyArgs,
};

/// Package-manager-neutral publish intent used by higher-level release flows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PublishRequest {
    pub target: Option<String>,
    pub dry_run: bool,
    pub tag: Option<String>,
    pub access: Option<String>,
    pub otp: Option<String>,
    pub provenance: Option<bool>,
    pub no_git_checks: bool,
    pub publish_branch: Option<String>,
    pub report_summary: bool,
    pub force: bool,
    pub json: bool,
    pub recursive: bool,
    pub filters: Option<Vec<String>>,
    pub pass_through_args: Vec<String>,
}

/// Concrete publish command resolved for a managed package manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPublishCommand {
    pub program: String,
    pub args: Vec<String>,
    pub env: std::collections::BTreeMap<String, String>,
}

fn publish_args(request: &PublishRequest) -> PublishArgs {
    PublishArgs {
        target: request.target.clone(),
        dry_run: request.dry_run,
        tag: request.tag.clone(),
        access: request.access.clone(),
        otp: request.otp.clone(),
        no_git_checks: request.no_git_checks,
        publish_branch: request.publish_branch.clone(),
        report_summary: request.report_summary,
        provenance: request.provenance == Some(true),
        force: request.force,
        json: request.json,
        recursive: request.recursive,
        filter: request.filters.clone(),
        pass_through_args: request.pass_through_args.clone(),
    }
}

fn resolve_publish(
    manager: &PackageManager,
    request: &PublishRequest,
) -> Result<resolution::Resolution, Error> {
    let mut resolution = resolution::resolve_for_manager(manager, publish_args(request))?;
    if let resolution::CommandResolution::Run(command) = &mut resolution.outcome
        && let Some(provenance) = request.provenance
    {
        let key = match manager.package_manager_type() {
            PackageManagerType::Yarn => "YARN_NPM_CONFIG_PROVENANCE",
            PackageManagerType::Npm | PackageManagerType::Pnpm | PackageManagerType::Bun => {
                "NPM_CONFIG_PROVENANCE"
            }
        };
        command.env.insert(key.to_string(), provenance.to_string());
    }
    Ok(resolution)
}

/// Resolves publish intent without spawning a process.
pub fn resolve_publish_command(
    manager: &PackageManager,
    request: &PublishRequest,
) -> Result<ResolvedPublishCommand, Error> {
    let resolution = resolve_publish(manager, request)?;
    match resolution.outcome {
        resolution::CommandResolution::Run(command) => Ok(ResolvedPublishCommand {
            program: command.program,
            args: command.args,
            env: command.env,
        }),
        resolution::CommandResolution::Noop => {
            Err(Error::UserMessage("publish unexpectedly resolved to a no-op".into()))
        }
        resolution::CommandResolution::InvalidArgument(message) => {
            Err(Error::UserMessage(message.into()))
        }
    }
}

/// Runs one publish command using the detected managed package manager.
pub async fn run_publish_command(
    cwd: &vt_path::AbsolutePath,
    manager: &PackageManager,
    request: &PublishRequest,
) -> Result<std::process::ExitStatus, Error> {
    let resolution = resolve_publish(manager, request)?;
    resolution::run_resolution(cwd, resolution, true).await
}

/// Runs package scripts with the detected package manager and managed binary path.
pub async fn run_scripts(
    cwd: &vt_path::AbsolutePath,
    manager: &PackageManager,
    scripts: &[String],
) -> Result<std::process::ExitStatus, Error> {
    let mut args = Vec::with_capacity(scripts.len() + 1);
    args.push("run".to_string());
    args.extend_from_slice(scripts);
    let env = std::collections::HashMap::from([(
        "PATH".to_string(),
        vp_shared::format_path_prepended(manager.get_bin_prefix()),
    )]);
    Ok(vp_command::run_command(&manager.package_manager_type().to_string(), args, &env, cwd)
        .await?)
}

/// npm release line that first includes the registry-side `npm trust` configuration command.
pub const TRUSTED_PUBLISHING_NPM_VERSION: &str = "^11.15.0";

/// Downloads (or reuses) an npm version capable of configuring trusted publishers and returns a
/// managed command handle for it.
pub async fn npm_for_trusted_publishing() -> Result<PackageManager, Error> {
    let (install_dir, _package_name, version) =
        download_package_manager(PackageManagerType::Npm, TRUSTED_PUBLISHING_NPM_VERSION, None)
            .await?;
    Ok(PackageManager::from_install(PackageManagerType::Npm, version, install_dir))
}

/// Runs an arbitrary command through an already managed package-manager installation.
pub async fn run_managed_command(
    cwd: &vt_path::AbsolutePath,
    manager: &PackageManager,
    args: &[String],
) -> Result<std::process::ExitStatus, Error> {
    let env = std::collections::HashMap::from([(
        "PATH".to_string(),
        vp_shared::format_path_prepended(manager.get_bin_prefix()),
    )]);
    Ok(vp_command::run_command(&manager.package_manager_type().to_string(), args, &env, cwd)
        .await?)
}
