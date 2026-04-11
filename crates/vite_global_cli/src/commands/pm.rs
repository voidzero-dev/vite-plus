//! Package manager commands (Category A).
//!
//! This module handles the `pm` subcommand and the `info` command which are
//! routed through helper functions. Other PM commands (add, install, remove, etc.)
//! are implemented as separate command modules with struct-based patterns.

use std::process::ExitStatus;

use vite_install::commands::{
    audit::AuditCommandOptions,
    cache::CacheCommandOptions,
    config::ConfigCommandOptions,
    deprecate::DeprecateCommandOptions,
    dist_tag::{DistTagCommandOptions, DistTagSubcommand},
    fund::FundCommandOptions,
    list::ListCommandOptions,
    login::LoginCommandOptions,
    logout::LogoutCommandOptions,
    owner::OwnerSubcommand,
    pack::PackCommandOptions,
    ping::PingCommandOptions,
    prune::PruneCommandOptions,
    publish::PublishCommandOptions,
    rebuild::RebuildCommandOptions,
    search::SearchCommandOptions,
    token::TokenSubcommand,
    view::ViewCommandOptions,
    whoami::WhoamiCommandOptions,
};
use vite_path::AbsolutePathBuf;

use super::{
    build_package_manager, build_package_manager_or_npm_default, prepend_js_runtime_to_path_env,
};
use crate::{
    cli::{ConfigCommands, DistTagCommands, OwnerCommands, PmCommands, TokenCommands},
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

    let package_manager = build_package_manager_or_npm_default(&cwd).await?;

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

    // Project-dependent commands require package.json; standalone ones fall back to npm.
    let needs_project = matches!(
        command,
        PmCommands::Prune { .. }
            | PmCommands::Pack { .. }
            | PmCommands::List { .. }
            | PmCommands::Publish { .. }
            | PmCommands::Rebuild { .. }
            | PmCommands::Fund { .. }
            | PmCommands::Audit { .. }
    );

    let package_manager = if needs_project {
        build_package_manager(&cwd).await?
    } else {
        build_package_manager_or_npm_default(&cwd).await?
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

        PmCommands::Login { registry, scope, pass_through_args } => {
            let options = LoginCommandOptions {
                registry: registry.as_deref(),
                scope: scope.as_deref(),
                pass_through_args: pass_through_args.as_deref(),
            };
            Ok(package_manager.run_login_command(&options, &cwd).await?)
        }

        PmCommands::Logout { registry, scope, pass_through_args } => {
            let options = LogoutCommandOptions {
                registry: registry.as_deref(),
                scope: scope.as_deref(),
                pass_through_args: pass_through_args.as_deref(),
            };
            Ok(package_manager.run_logout_command(&options, &cwd).await?)
        }

        PmCommands::Whoami { registry, pass_through_args } => {
            let options = WhoamiCommandOptions {
                registry: registry.as_deref(),
                pass_through_args: pass_through_args.as_deref(),
            };
            Ok(package_manager.run_whoami_command(&options, &cwd).await?)
        }

        PmCommands::Token(token_command) => {
            let subcommand = match token_command {
                TokenCommands::List { json, registry, pass_through_args } => {
                    TokenSubcommand::List { json, registry, pass_through_args }
                }
                TokenCommands::Create { json, registry, cidr, readonly, pass_through_args } => {
                    TokenSubcommand::Create { json, registry, cidr, readonly, pass_through_args }
                }
                TokenCommands::Revoke { token, registry, pass_through_args } => {
                    TokenSubcommand::Revoke { token, registry, pass_through_args }
                }
            };
            Ok(package_manager.run_token_command(&subcommand, &cwd).await?)
        }

        PmCommands::Audit { fix, json, level, production, pass_through_args } => {
            let options = AuditCommandOptions {
                fix,
                json,
                level: level.as_deref(),
                production,
                pass_through_args: pass_through_args.as_deref(),
            };
            Ok(package_manager.run_audit_command(&options, &cwd).await?)
        }

        PmCommands::DistTag(dist_tag_command) => {
            let subcommand = match dist_tag_command {
                DistTagCommands::List { package } => DistTagSubcommand::List { package },
                DistTagCommands::Add { package_at_version, tag } => {
                    DistTagSubcommand::Add { package_at_version, tag }
                }
                DistTagCommands::Rm { package, tag } => DistTagSubcommand::Rm { package, tag },
            };
            let options = DistTagCommandOptions { subcommand, pass_through_args: None };
            Ok(package_manager.run_dist_tag_command(&options, &cwd).await?)
        }

        PmCommands::Deprecate { package, message, otp, registry, pass_through_args } => {
            let options = DeprecateCommandOptions {
                package: &package,
                message: &message,
                otp: otp.as_deref(),
                registry: registry.as_deref(),
                pass_through_args: pass_through_args.as_deref(),
            };
            Ok(package_manager.run_deprecate_command(&options, &cwd).await?)
        }

        PmCommands::Search { terms, json, long, registry, pass_through_args } => {
            let options = SearchCommandOptions {
                terms: &terms,
                json,
                long,
                registry: registry.as_deref(),
                pass_through_args: pass_through_args.as_deref(),
            };
            Ok(package_manager.run_search_command(&options, &cwd).await?)
        }

        PmCommands::Rebuild { pass_through_args } => {
            let options = RebuildCommandOptions { pass_through_args: pass_through_args.as_deref() };
            Ok(package_manager.run_rebuild_command(&options, &cwd).await?)
        }

        PmCommands::Fund { json, pass_through_args } => {
            let options =
                FundCommandOptions { json, pass_through_args: pass_through_args.as_deref() };
            Ok(package_manager.run_fund_command(&options, &cwd).await?)
        }

        PmCommands::Ping { registry, pass_through_args } => {
            let options = PingCommandOptions {
                registry: registry.as_deref(),
                pass_through_args: pass_through_args.as_deref(),
            };
            Ok(package_manager.run_ping_command(&options, &cwd).await?)
        }
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
