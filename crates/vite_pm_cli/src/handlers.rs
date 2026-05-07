//! Handlers that wrap `vite_install`'s `PackageManager::run_*_command`
//! family, returning the underlying process exit status.

use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_install::{
    PackageManager,
    commands::{
        add::AddCommandOptions,
        audit::AuditCommandOptions,
        cache::CacheCommandOptions,
        config::ConfigCommandOptions,
        dedupe::DedupeCommandOptions,
        deprecate::DeprecateCommandOptions,
        dist_tag::{DistTagCommandOptions, DistTagSubcommand},
        dlx::{DlxCommandOptions, build_npx_args},
        fund::FundCommandOptions,
        install::InstallCommandOptions,
        link::LinkCommandOptions,
        list::ListCommandOptions,
        login::LoginCommandOptions,
        logout::LogoutCommandOptions,
        outdated::OutdatedCommandOptions,
        owner::OwnerSubcommand,
        pack::PackCommandOptions,
        ping::PingCommandOptions,
        prune::PruneCommandOptions,
        publish::PublishCommandOptions,
        rebuild::RebuildCommandOptions,
        remove::RemoveCommandOptions,
        search::SearchCommandOptions,
        token::TokenSubcommand,
        unlink::UnlinkCommandOptions,
        update::UpdateCommandOptions,
        view::ViewCommandOptions,
        whoami::WhoamiCommandOptions,
        why::WhyCommandOptions,
    },
};
use vite_path::AbsolutePath;

use crate::{
    cli::{ConfigCommands, DistTagCommands, OwnerCommands, PmCommands, TokenCommands},
    error::Error,
    helpers::{build_package_manager, build_package_manager_or_npm_default, ensure_package_json},
};

pub async fn run_add(
    cwd: &AbsolutePath,
    options: &AddCommandOptions<'_>,
) -> Result<ExitStatus, Error> {
    ensure_package_json(cwd).await?;
    let pm = PackageManager::builder(cwd).build_with_default().await?;
    Ok(pm.run_add_command(options, cwd).await?)
}

pub async fn run_install(
    cwd: &AbsolutePath,
    options: &InstallCommandOptions<'_>,
) -> Result<ExitStatus, Error> {
    ensure_package_json(cwd).await?;
    let pm = PackageManager::builder(cwd).build_with_default().await?;
    Ok(pm.run_install_command(options, cwd).await?)
}

pub async fn run_remove(
    cwd: &AbsolutePath,
    options: &RemoveCommandOptions<'_>,
) -> Result<ExitStatus, Error> {
    let pm = build_package_manager(cwd).await?;
    Ok(pm.run_remove_command(options, cwd).await?)
}

pub async fn run_update(
    cwd: &AbsolutePath,
    options: &UpdateCommandOptions<'_>,
) -> Result<ExitStatus, Error> {
    let pm = build_package_manager(cwd).await?;
    Ok(pm.run_update_command(options, cwd).await?)
}

pub async fn run_dedupe(
    cwd: &AbsolutePath,
    options: &DedupeCommandOptions<'_>,
) -> Result<ExitStatus, Error> {
    let pm = build_package_manager(cwd).await?;
    Ok(pm.run_dedupe_command(options, cwd).await?)
}

pub async fn run_outdated(
    cwd: &AbsolutePath,
    options: &OutdatedCommandOptions<'_>,
) -> Result<ExitStatus, Error> {
    let pm = build_package_manager(cwd).await?;
    Ok(pm.run_outdated_command(options, cwd).await?)
}

pub async fn run_why(
    cwd: &AbsolutePath,
    options: &WhyCommandOptions<'_>,
) -> Result<ExitStatus, Error> {
    let pm = build_package_manager(cwd).await?;
    Ok(pm.run_why_command(options, cwd).await?)
}

pub async fn run_info(
    cwd: &AbsolutePath,
    options: &ViewCommandOptions<'_>,
) -> Result<ExitStatus, Error> {
    let pm = build_package_manager_or_npm_default(cwd).await?;
    Ok(pm.run_view_command(options, cwd).await?)
}

pub async fn run_link(
    cwd: &AbsolutePath,
    options: &LinkCommandOptions<'_>,
) -> Result<ExitStatus, Error> {
    let pm = build_package_manager(cwd).await?;
    Ok(pm.run_link_command(options, cwd).await?)
}

pub async fn run_unlink(
    cwd: &AbsolutePath,
    options: &UnlinkCommandOptions<'_>,
) -> Result<ExitStatus, Error> {
    let pm = build_package_manager(cwd).await?;
    Ok(pm.run_unlink_command(options, cwd).await?)
}

pub async fn run_dlx(
    cwd: &AbsolutePath,
    packages: Vec<String>,
    shell_mode: bool,
    silent: bool,
    args: Vec<String>,
) -> Result<ExitStatus, Error> {
    if args.is_empty() {
        return Err(Error::Other("dlx requires a package name".into()));
    }

    let package_spec = &args[0];
    let command_args: Vec<String> = args[1..].to_vec();

    let options = DlxCommandOptions {
        packages: &packages,
        package_spec,
        args: &command_args,
        shell_mode,
        silent,
    };

    match PackageManager::builder(cwd).build_with_default().await {
        Ok(pm) => Ok(pm.run_dlx_command(&options, cwd).await?),
        Err(vite_error::Error::WorkspaceError(vite_workspace::Error::PackageJsonNotFound(_))) => {
            let npx_args = build_npx_args(&options);
            let envs = HashMap::new();
            Ok(run_command("npx", &npx_args, &envs, cwd).await?)
        }
        Err(e) => Err(Error::Install(e)),
    }
}

pub async fn run_pm_subcommand(
    cwd: &AbsolutePath,
    command: PmCommands,
) -> Result<ExitStatus, Error> {
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

    let pm = if needs_project {
        build_package_manager(cwd).await?
    } else {
        build_package_manager_or_npm_default(cwd).await?
    };

    match command {
        PmCommands::Prune { prod, no_optional, pass_through_args } => {
            let options = PruneCommandOptions {
                prod,
                no_optional,
                pass_through_args: pass_through_args.as_deref(),
            };
            Ok(pm.run_prune_command(&options, cwd).await?)
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
            Ok(pm.run_pack_command(&options, cwd).await?)
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
            Ok(pm.run_list_command(&options, cwd).await?)
        }

        PmCommands::View { package, field, json, pass_through_args } => {
            let options = ViewCommandOptions {
                package: &package,
                field: field.as_deref(),
                json,
                pass_through_args: pass_through_args.as_deref(),
            };
            Ok(pm.run_view_command(&options, cwd).await?)
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
            Ok(pm.run_publish_command(&options, cwd).await?)
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
            Ok(pm.run_owner_command(&subcommand, cwd).await?)
        }

        PmCommands::Cache { subcommand, pass_through_args } => {
            let options = CacheCommandOptions {
                subcommand: &subcommand,
                pass_through_args: pass_through_args.as_deref(),
            };
            Ok(pm.run_cache_command(&options, cwd).await?)
        }

        PmCommands::Config(config_command) => match config_command {
            ConfigCommands::List { json, global, location } => {
                let options = ConfigCommandOptions {
                    subcommand: "list",
                    key: None,
                    value: None,
                    json,
                    location: config_location(global, location.as_deref()),
                    pass_through_args: None,
                };
                Ok(pm.run_config_command(&options, cwd).await?)
            }
            ConfigCommands::Get { key, json, global, location } => {
                let options = ConfigCommandOptions {
                    subcommand: "get",
                    key: Some(key.as_str()),
                    value: None,
                    json,
                    location: config_location(global, location.as_deref()),
                    pass_through_args: None,
                };
                Ok(pm.run_config_command(&options, cwd).await?)
            }
            ConfigCommands::Set { key, value, json, global, location } => {
                let options = ConfigCommandOptions {
                    subcommand: "set",
                    key: Some(key.as_str()),
                    value: Some(value.as_str()),
                    json,
                    location: config_location(global, location.as_deref()),
                    pass_through_args: None,
                };
                Ok(pm.run_config_command(&options, cwd).await?)
            }
            ConfigCommands::Delete { key, global, location } => {
                let options = ConfigCommandOptions {
                    subcommand: "delete",
                    key: Some(key.as_str()),
                    value: None,
                    json: false,
                    location: config_location(global, location.as_deref()),
                    pass_through_args: None,
                };
                Ok(pm.run_config_command(&options, cwd).await?)
            }
        },

        PmCommands::Login { registry, scope, pass_through_args } => {
            let options = LoginCommandOptions {
                registry: registry.as_deref(),
                scope: scope.as_deref(),
                pass_through_args: pass_through_args.as_deref(),
            };
            Ok(pm.run_login_command(&options, cwd).await?)
        }

        PmCommands::Logout { registry, scope, pass_through_args } => {
            let options = LogoutCommandOptions {
                registry: registry.as_deref(),
                scope: scope.as_deref(),
                pass_through_args: pass_through_args.as_deref(),
            };
            Ok(pm.run_logout_command(&options, cwd).await?)
        }

        PmCommands::Whoami { registry, pass_through_args } => {
            let options = WhoamiCommandOptions {
                registry: registry.as_deref(),
                pass_through_args: pass_through_args.as_deref(),
            };
            Ok(pm.run_whoami_command(&options, cwd).await?)
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
            Ok(pm.run_token_command(&subcommand, cwd).await?)
        }

        PmCommands::Audit { fix, json, level, production, pass_through_args } => {
            let options = AuditCommandOptions {
                fix,
                json,
                level: level.as_deref(),
                production,
                pass_through_args: pass_through_args.as_deref(),
            };
            Ok(pm.run_audit_command(&options, cwd).await?)
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
            Ok(pm.run_dist_tag_command(&options, cwd).await?)
        }

        PmCommands::Deprecate { package, message, otp, registry, pass_through_args } => {
            let options = DeprecateCommandOptions {
                package: &package,
                message: &message,
                otp: otp.as_deref(),
                registry: registry.as_deref(),
                pass_through_args: pass_through_args.as_deref(),
            };
            Ok(pm.run_deprecate_command(&options, cwd).await?)
        }

        PmCommands::Search { terms, json, long, registry, pass_through_args } => {
            let options = SearchCommandOptions {
                terms: &terms,
                json,
                long,
                registry: registry.as_deref(),
                pass_through_args: pass_through_args.as_deref(),
            };
            Ok(pm.run_search_command(&options, cwd).await?)
        }

        PmCommands::Rebuild { pass_through_args } => {
            let options = RebuildCommandOptions { pass_through_args: pass_through_args.as_deref() };
            Ok(pm.run_rebuild_command(&options, cwd).await?)
        }

        PmCommands::Fund { json, pass_through_args } => {
            let options =
                FundCommandOptions { json, pass_through_args: pass_through_args.as_deref() };
            Ok(pm.run_fund_command(&options, cwd).await?)
        }

        PmCommands::Ping { registry, pass_through_args } => {
            let options = PingCommandOptions {
                registry: registry.as_deref(),
                pass_through_args: pass_through_args.as_deref(),
            };
            Ok(pm.run_ping_command(&options, cwd).await?)
        }
    }
}

fn config_location(global: bool, location: Option<&str>) -> Option<&str> {
    if global { Some("global") } else { location }
}
