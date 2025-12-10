use std::process::ExitStatus;

use vite_install::{
    commands::{
        cache::CacheCommandOptions, config::ConfigCommandOptions, list::ListCommandOptions,
        owner::OwnerSubcommand, pack::PackCommandOptions, prune::PruneCommandOptions,
        publish::PublishCommandOptions, view::ViewCommandOptions,
    },
    package_manager::PackageManager,
};
use vite_path::AbsolutePathBuf;

use crate::{
    Error,
    cli::{ConfigCommands, OwnerCommands, PmCommands},
};

/// Package manager utilities command.
///
/// This command provides a unified interface to package manager utilities
/// across pnpm, npm, and yarn.
pub struct PmCommand {
    cwd: AbsolutePathBuf,
}

impl PmCommand {
    pub fn new(cwd: AbsolutePathBuf) -> Self {
        Self { cwd }
    }

    pub async fn execute(self, command: PmCommands) -> Result<ExitStatus, Error> {
        // Detect package manager
        let package_manager = PackageManager::builder(&self.cwd).build_with_default().await?;

        match command {
            PmCommands::Prune { prod, no_optional, pass_through_args } => {
                let options = PruneCommandOptions {
                    prod,
                    no_optional,
                    pass_through_args: pass_through_args.as_deref(),
                };
                package_manager.run_prune_command(&options, &self.cwd).await
            }
            PmCommands::Pack {
                recursive,
                filter,
                out,
                pack_destination,
                pack_gzip_level,
                json,
                pass_through_args,
            } => {
                let options = PackCommandOptions {
                    recursive,
                    filters: filter.as_deref(),
                    out: out.as_deref(),
                    pack_destination: pack_destination.as_deref(),
                    pack_gzip_level,
                    json,
                    pass_through_args: pass_through_args.as_deref(),
                };
                package_manager.run_pack_command(&options, &self.cwd).await
            }
            PmCommands::List {
                pattern,
                depth,
                json,
                long,
                parseable,
                prod,
                dev,
                no_optional,
                exclude_peers,
                only_projects,
                find_by,
                recursive,
                filter,
                global,
                pass_through_args,
            } => {
                let options = ListCommandOptions {
                    pattern: pattern.as_deref(),
                    depth,
                    json,
                    long,
                    parseable,
                    prod,
                    dev,
                    no_optional,
                    exclude_peers,
                    only_projects,
                    find_by: find_by.as_deref(),
                    recursive,
                    filters: if filter.is_empty() { None } else { Some(&filter) },
                    global,
                    pass_through_args: pass_through_args.as_deref(),
                };
                package_manager.run_list_command(&options, &self.cwd).await
            }
            PmCommands::View { package, field, json, pass_through_args } => {
                let options = ViewCommandOptions {
                    package: &package,
                    field: field.as_deref(),
                    json,
                    pass_through_args: pass_through_args.as_deref(),
                };
                package_manager.run_view_command(&options, &self.cwd).await
            }
            PmCommands::Publish {
                target,
                dry_run,
                tag,
                access,
                otp,
                no_git_checks,
                publish_branch,
                report_summary,
                force,
                json,
                recursive,
                filter,
                pass_through_args,
            } => {
                let options = PublishCommandOptions {
                    target: target.as_deref(),
                    dry_run,
                    tag: tag.as_deref(),
                    access: access.as_deref(),
                    otp: otp.as_deref(),
                    no_git_checks,
                    publish_branch: publish_branch.as_deref(),
                    report_summary,
                    force,
                    json,
                    recursive,
                    filters: filter.as_deref(),
                    pass_through_args: pass_through_args.as_deref(),
                };
                package_manager.run_publish_command(&options, &self.cwd).await
            }
            PmCommands::Owner(owner_command) => {
                let subcommand = match owner_command {
                    OwnerCommands::List { package, otp } => OwnerSubcommand::List { package, otp },
                    OwnerCommands::Add { user, package, otp } => {
                        OwnerSubcommand::Add { user, package, otp }
                    }
                    OwnerCommands::Rm { user, package, otp } => {
                        OwnerSubcommand::Rm { user, package, otp }
                    }
                };
                package_manager.run_owner_command(&subcommand, &self.cwd).await
            }
            PmCommands::Cache { subcommand, pass_through_args } => {
                let options = CacheCommandOptions {
                    subcommand: &subcommand,
                    pass_through_args: pass_through_args.as_deref(),
                };
                package_manager.run_cache_command(&options, &self.cwd).await
            }
            PmCommands::Config(config_command) => match config_command {
                ConfigCommands::List { json, global, location } => {
                    let options = ConfigCommandOptions {
                        subcommand: "list",
                        key: None,
                        value: None,
                        json,
                        location: if global { Some("global") } else { location.as_deref() },
                        pass_through_args: None,
                    };
                    package_manager.run_config_command(&options, &self.cwd).await
                }
                ConfigCommands::Get { key, json, global, location } => {
                    let options = ConfigCommandOptions {
                        subcommand: "get",
                        key: Some(key.as_str()),
                        value: None,
                        json,
                        location: if global { Some("global") } else { location.as_deref() },
                        pass_through_args: None,
                    };
                    package_manager.run_config_command(&options, &self.cwd).await
                }
                ConfigCommands::Set { key, value, json, global, location } => {
                    let options = ConfigCommandOptions {
                        subcommand: "set",
                        key: Some(key.as_str()),
                        value: Some(value.as_str()),
                        json,
                        location: if global { Some("global") } else { location.as_deref() },
                        pass_through_args: None,
                    };
                    package_manager.run_config_command(&options, &self.cwd).await
                }
                ConfigCommands::Delete { key, global, location } => {
                    let options = ConfigCommandOptions {
                        subcommand: "delete",
                        key: Some(key.as_str()),
                        value: None,
                        json: false,
                        location: if global { Some("global") } else { location.as_deref() },
                        pass_through_args: None,
                    };
                    package_manager.run_config_command(&options, &self.cwd).await
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pm_command_new() {
        let workspace_root = if cfg!(windows) {
            AbsolutePathBuf::new("C:\\test".into()).unwrap()
        } else {
            AbsolutePathBuf::new("/test".into()).unwrap()
        };

        let cmd = PmCommand::new(workspace_root.clone());
        assert_eq!(cmd.cwd, workspace_root);
    }
}
