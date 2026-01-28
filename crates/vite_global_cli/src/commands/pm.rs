//! Package manager commands (Category A).
//!
//! These commands wrap existing package managers (pnpm/npm/yarn) and use
//! managed Node.js from `vite_js_runtime` to execute them.

use std::process::ExitStatus;

use vite_install::{
    PackageManager,
    commands::{
        add::{AddCommandOptions, SaveDependencyType},
        cache::CacheCommandOptions,
        config::ConfigCommandOptions,
        dedupe::DedupeCommandOptions,
        dlx::DlxCommandOptions,
        install::InstallCommandOptions,
        link::LinkCommandOptions,
        list::ListCommandOptions,
        outdated::{Format, OutdatedCommandOptions},
        owner::OwnerSubcommand,
        pack::PackCommandOptions,
        prune::PruneCommandOptions,
        publish::PublishCommandOptions,
        remove::RemoveCommandOptions,
        unlink::UnlinkCommandOptions,
        update::UpdateCommandOptions,
        view::ViewCommandOptions,
        why::WhyCommandOptions,
    },
};
use vite_path::AbsolutePathBuf;

use crate::{
    cli::{ConfigCommands, OwnerCommands, PmCommands},
    error::Error,
};

/// Execute the install command.
pub async fn execute_install(
    cwd: AbsolutePathBuf,
    options: &InstallCommandOptions<'_>,
) -> Result<ExitStatus, Error> {
    let package_manager = PackageManager::builder(&cwd).build_with_default().await?;
    Ok(package_manager.run_install_command(options, &cwd).await?)
}

/// Execute the add command.
#[expect(clippy::too_many_arguments)]
pub async fn execute_add(
    cwd: AbsolutePathBuf,
    packages: Vec<String>,
    save_prod: bool,
    save_dev: bool,
    save_peer: bool,
    save_optional: bool,
    save_exact: bool,
    _save_catalog: bool,
    save_catalog_name: Option<String>,
    filter: Option<&[String]>,
    workspace_root: bool,
    workspace_only: bool,
    global: bool,
    allow_build: Option<String>,
    pass_through_args: Option<&[String]>,
) -> Result<ExitStatus, Error> {
    let package_manager = PackageManager::builder(&cwd).build_with_default().await?;

    let save_dependency_type = if save_dev {
        Some(SaveDependencyType::Dev)
    } else if save_peer {
        Some(SaveDependencyType::Peer)
    } else if save_optional {
        Some(SaveDependencyType::Optional)
    } else if save_prod {
        Some(SaveDependencyType::Production)
    } else {
        None
    };

    let options = AddCommandOptions {
        packages: &packages,
        save_dependency_type,
        save_exact,
        save_catalog_name: save_catalog_name.as_deref(),
        allow_build: allow_build.as_deref(),
        filters: filter,
        workspace_root,
        workspace_only,
        global,
        pass_through_args,
    };

    Ok(package_manager.run_add_command(&options, &cwd).await?)
}

/// Execute the remove command.
#[expect(clippy::too_many_arguments)]
pub async fn execute_remove(
    cwd: AbsolutePathBuf,
    packages: Vec<String>,
    save_dev: bool,
    save_optional: bool,
    save_prod: bool,
    filter: Option<&[String]>,
    workspace_root: bool,
    recursive: bool,
    global: bool,
    pass_through_args: Option<&[String]>,
) -> Result<ExitStatus, Error> {
    let package_manager = PackageManager::builder(&cwd).build_with_default().await?;

    let options = RemoveCommandOptions {
        packages: &packages,
        save_dev,
        save_optional,
        save_prod,
        filters: filter,
        workspace_root,
        recursive,
        global,
        pass_through_args,
    };

    Ok(package_manager.run_remove_command(&options, &cwd).await?)
}

/// Execute the update command.
#[expect(clippy::too_many_arguments)]
pub async fn execute_update(
    cwd: AbsolutePathBuf,
    packages: Vec<String>,
    latest: bool,
    global: bool,
    recursive: bool,
    filter: Option<&[String]>,
    workspace_root: bool,
    dev: bool,
    prod: bool,
    interactive: bool,
    no_optional: bool,
    no_save: bool,
    workspace_only: bool,
    pass_through_args: Option<&[String]>,
) -> Result<ExitStatus, Error> {
    let package_manager = PackageManager::builder(&cwd).build_with_default().await?;

    let options = UpdateCommandOptions {
        packages: &packages,
        latest,
        global,
        recursive,
        filters: filter,
        workspace_root,
        dev,
        prod,
        interactive,
        no_optional,
        no_save,
        workspace_only,
        pass_through_args,
    };

    Ok(package_manager.run_update_command(&options, &cwd).await?)
}

/// Execute the dedupe command.
pub async fn execute_dedupe(
    cwd: AbsolutePathBuf,
    check: bool,
    pass_through_args: Option<&[String]>,
) -> Result<ExitStatus, Error> {
    let package_manager = PackageManager::builder(&cwd).build_with_default().await?;

    let options = DedupeCommandOptions { check, pass_through_args };

    Ok(package_manager.run_dedupe_command(&options, &cwd).await?)
}

/// Execute the outdated command.
#[expect(clippy::too_many_arguments)]
pub async fn execute_outdated(
    cwd: AbsolutePathBuf,
    packages: Vec<String>,
    long: bool,
    format: Option<Format>,
    recursive: bool,
    filter: Option<&[String]>,
    workspace_root: bool,
    prod: bool,
    dev: bool,
    no_optional: bool,
    compatible: bool,
    sort_by: Option<String>,
    global: bool,
    pass_through_args: Option<&[String]>,
) -> Result<ExitStatus, Error> {
    let package_manager = PackageManager::builder(&cwd).build_with_default().await?;

    let options = OutdatedCommandOptions {
        packages: &packages,
        long,
        format,
        recursive,
        filters: filter,
        workspace_root,
        prod,
        dev,
        no_optional,
        compatible,
        sort_by: sort_by.as_deref(),
        global,
        pass_through_args,
    };

    Ok(package_manager.run_outdated_command(&options, &cwd).await?)
}

/// Execute the why command.
#[expect(clippy::too_many_arguments)]
pub async fn execute_why(
    cwd: AbsolutePathBuf,
    packages: Vec<String>,
    json: bool,
    long: bool,
    parseable: bool,
    recursive: bool,
    filter: Option<&[String]>,
    workspace_root: bool,
    prod: bool,
    dev: bool,
    depth: Option<u32>,
    no_optional: bool,
    global: bool,
    exclude_peers: bool,
    find_by: Option<String>,
    pass_through_args: Option<&[String]>,
) -> Result<ExitStatus, Error> {
    let package_manager = PackageManager::builder(&cwd).build_with_default().await?;

    let options = WhyCommandOptions {
        packages: &packages,
        json,
        long,
        parseable,
        recursive,
        filters: filter,
        workspace_root,
        prod,
        dev,
        depth,
        no_optional,
        global,
        exclude_peers,
        find_by: find_by.as_deref(),
        pass_through_args,
    };

    Ok(package_manager.run_why_command(&options, &cwd).await?)
}

/// Execute the info command.
pub async fn execute_info(
    cwd: AbsolutePathBuf,
    package: &str,
    field: Option<&str>,
    json: bool,
    pass_through_args: Option<&[String]>,
) -> Result<ExitStatus, Error> {
    let package_manager = PackageManager::builder(&cwd).build_with_default().await?;

    let options = ViewCommandOptions { package, field, json, pass_through_args };

    Ok(package_manager.run_view_command(&options, &cwd).await?)
}

/// Execute the link command.
pub async fn execute_link(
    cwd: AbsolutePathBuf,
    package: Option<String>,
    args: &[String],
) -> Result<ExitStatus, Error> {
    let package_manager = PackageManager::builder(&cwd).build_with_default().await?;

    let options = LinkCommandOptions {
        package: package.as_deref(),
        pass_through_args: if args.is_empty() { None } else { Some(args) },
    };

    Ok(package_manager.run_link_command(&options, &cwd).await?)
}

/// Execute the unlink command.
pub async fn execute_unlink(
    cwd: AbsolutePathBuf,
    package: Option<String>,
    recursive: bool,
    args: &[String],
) -> Result<ExitStatus, Error> {
    let package_manager = PackageManager::builder(&cwd).build_with_default().await?;

    let options = UnlinkCommandOptions {
        package: package.as_deref(),
        recursive,
        pass_through_args: if args.is_empty() { None } else { Some(args) },
    };

    Ok(package_manager.run_unlink_command(&options, &cwd).await?)
}

/// Execute the dlx command.
pub async fn execute_dlx(
    cwd: AbsolutePathBuf,
    packages: Vec<String>,
    shell_mode: bool,
    silent: bool,
    args: &[String],
) -> Result<ExitStatus, Error> {
    let package_manager = PackageManager::builder(&cwd).build_with_default().await?;

    // Extract the package spec from args (first positional argument)
    let (package_spec, remaining_args) = if args.is_empty() {
        return Err(Error::Other("dlx requires a package to execute".into()));
    } else {
        (&args[0], &args[1..])
    };

    let options = DlxCommandOptions {
        packages: &packages,
        package_spec,
        args: remaining_args,
        shell_mode,
        silent,
    };

    Ok(package_manager.run_dlx_command(&options, &cwd).await?)
}

/// Execute a pm subcommand.
pub async fn execute_pm_subcommand(
    cwd: AbsolutePathBuf,
    command: PmCommands,
) -> Result<ExitStatus, Error> {
    let package_manager = PackageManager::builder(&cwd).build_with_default().await?;

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
    use super::*;

    #[test]
    fn test_save_dependency_type() {
        assert!(matches!(SaveDependencyType::Dev, SaveDependencyType::Dev));
        assert!(matches!(SaveDependencyType::Production, SaveDependencyType::Production));
    }
}
