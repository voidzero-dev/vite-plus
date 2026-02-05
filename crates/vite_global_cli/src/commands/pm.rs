//! Package manager commands (Category A).
//!
//! This module handles the `pm` subcommand and the `info` command which are
//! routed through helper functions. Other PM commands (add, install, remove, etc.)
//! are implemented as separate command modules with struct-based patterns.

use std::process::ExitStatus;

use vite_install::{
    PackageManager,
    commands::{
        cache::CacheCommandOptions, config::ConfigCommandOptions, list::ListCommandOptions,
        owner::OwnerSubcommand, pack::PackCommandOptions, prune::PruneCommandOptions,
        publish::PublishCommandOptions, view::ViewCommandOptions,
    },
};
use vite_path::AbsolutePathBuf;

use super::prepend_js_runtime_to_path_env;
use crate::{
    cli::{ConfigCommands, OwnerCommands, PmCommands},
    error::Error,
};

/// Execute the info command.
pub async fn execute_info(
    cwd: AbsolutePathBuf,
    package: &str,
    field: Option<&str>,
    json: bool,
    pass_through_args: Option<&[String]>,
) -> Result<ExitStatus, Error> {
    prepend_js_runtime_to_path_env(&cwd).await?;

    let package_manager = PackageManager::builder(&cwd).build_with_default().await?;

    let options = ViewCommandOptions { package, field, json, pass_through_args };

    Ok(package_manager.run_view_command(&options, &cwd).await?)
}

/// Execute a pm subcommand.
pub async fn execute_pm_subcommand(
    cwd: AbsolutePathBuf,
    command: PmCommands,
) -> Result<ExitStatus, Error> {
    // Intercept `pm list -g` to use vite-plus managed global packages listing
    if let PmCommands::List { global: true, json, ref pattern, .. } = command {
        return crate::commands::env::packages::execute(json, pattern.as_deref()).await;
    }

    prepend_js_runtime_to_path_env(&cwd).await?;

    let package_manager = match PackageManager::builder(&cwd).build_with_default().await {
        Ok(pm) => pm,
        Err(e) => {
            // For `list` command, silently succeed when no workspace is found
            // (matches `pnpm list` behavior in dirs without package.json)
            if matches!(&command, PmCommands::List { .. })
                && matches!(
                    &e,
                    vite_error::Error::WorkspaceError(vite_workspace::Error::PackageJsonNotFound(
                        _
                    ))
                )
            {
                return Ok(ExitStatus::default());
            }
            return Err(e.into());
        }
    };

    match command {
        PmCommands::Prune { prod, no_optional, pass_through_args } => {
            let options = PruneCommandOptions {
                prod,
                no_optional,
                pass_through_args: pass_through_args.as_deref(),
            };
            Ok(package_manager.run_prune_command(&options, &cwd).await?)
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
            Ok(package_manager.run_pack_command(&options, &cwd).await?)
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
            Ok(package_manager.run_list_command(&options, &cwd).await?)
        }

        PmCommands::View { package, field, json, pass_through_args } => {
            let options = ViewCommandOptions {
                package: &package,
                field: field.as_deref(),
                json,
                pass_through_args: pass_through_args.as_deref(),
            };
            Ok(package_manager.run_view_command(&options, &cwd).await?)
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
            Ok(package_manager.run_publish_command(&options, &cwd).await?)
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
            Ok(package_manager.run_owner_command(&subcommand, &cwd).await?)
        }

        PmCommands::Cache { subcommand, pass_through_args } => {
            let options = CacheCommandOptions {
                subcommand: &subcommand,
                pass_through_args: pass_through_args.as_deref(),
            };
            Ok(package_manager.run_cache_command(&options, &cwd).await?)
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
                Ok(package_manager.run_config_command(&options, &cwd).await?)
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
                Ok(package_manager.run_config_command(&options, &cwd).await?)
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
                Ok(package_manager.run_config_command(&options, &cwd).await?)
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
                Ok(package_manager.run_config_command(&options, &cwd).await?)
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use vite_install::commands::add::SaveDependencyType;

    #[test]
    fn test_save_dependency_type() {
        assert!(matches!(SaveDependencyType::Dev, SaveDependencyType::Dev));
        assert!(matches!(SaveDependencyType::Production, SaveDependencyType::Production));
    }
}
