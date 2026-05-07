//! Maps a parsed [`PackageManagerCommand`] to the appropriate handler.
//!
//! Callers must perform any environment setup (PATH adjustments, runtime
//! download) before invoking [`dispatch`].

use std::process::ExitStatus;

use vite_install::commands::{
    add::AddCommandOptions, dedupe::DedupeCommandOptions, install::InstallCommandOptions,
    link::LinkCommandOptions, outdated::OutdatedCommandOptions, remove::RemoveCommandOptions,
    unlink::UnlinkCommandOptions, update::UpdateCommandOptions, view::ViewCommandOptions,
    why::WhyCommandOptions,
};
use vite_path::AbsolutePath;

use crate::{cli::PackageManagerCommand, error::Error, handlers};

pub async fn dispatch(
    cwd: &AbsolutePath,
    command: PackageManagerCommand,
) -> Result<ExitStatus, Error> {
    match command {
        PackageManagerCommand::Install {
            prod,
            dev,
            no_optional,
            frozen_lockfile,
            no_frozen_lockfile,
            lockfile_only,
            prefer_offline,
            offline,
            force,
            ignore_scripts,
            no_lockfile,
            fix_lockfile,
            shamefully_hoist,
            resolution_only,
            silent,
            filter,
            workspace_root,
            save_exact,
            save_peer,
            save_optional,
            save_catalog,
            global,
            node: _,
            packages,
            pass_through_args,
        } => {
            // `vp install <packages>` is an alias for `vp add <packages>`.
            if let Some(pkgs) = packages
                && !pkgs.is_empty()
            {
                let save_dependency_type = PackageManagerCommand::determine_save_dependency_type(
                    dev,
                    save_peer,
                    save_optional,
                    prod,
                );
                let options = AddCommandOptions {
                    packages: &pkgs,
                    save_dependency_type,
                    save_exact,
                    save_catalog_name: catalog_name(save_catalog, None),
                    filters: filter.as_deref(),
                    workspace_root,
                    workspace_only: false,
                    global,
                    allow_build: None,
                    pass_through_args: pass_through_args.as_deref(),
                };
                return handlers::run_add(cwd, &options).await;
            }

            let options = InstallCommandOptions {
                prod,
                dev,
                no_optional,
                frozen_lockfile,
                no_frozen_lockfile,
                lockfile_only,
                prefer_offline,
                offline,
                force,
                ignore_scripts,
                no_lockfile,
                fix_lockfile,
                shamefully_hoist,
                resolution_only,
                silent,
                filters: filter.as_deref(),
                workspace_root,
                pass_through_args: pass_through_args.as_deref(),
            };
            handlers::run_install(cwd, &options).await
        }

        PackageManagerCommand::Add {
            save_prod,
            save_dev,
            save_peer,
            save_optional,
            save_exact,
            save_catalog_name,
            save_catalog,
            allow_build,
            filter,
            workspace_root,
            workspace,
            global,
            node: _,
            packages,
            pass_through_args,
        } => {
            let save_dependency_type = PackageManagerCommand::determine_save_dependency_type(
                save_dev,
                save_peer,
                save_optional,
                save_prod,
            );
            let options = AddCommandOptions {
                packages: &packages,
                save_dependency_type,
                save_exact,
                save_catalog_name: catalog_name(save_catalog, save_catalog_name.as_deref()),
                filters: filter.as_deref(),
                workspace_root,
                workspace_only: workspace,
                global,
                allow_build: allow_build.as_deref(),
                pass_through_args: pass_through_args.as_deref(),
            };
            handlers::run_add(cwd, &options).await
        }

        PackageManagerCommand::Remove {
            save_dev,
            save_optional,
            save_prod,
            filter,
            workspace_root,
            recursive,
            global,
            // `--dry-run` is clap-required to coexist with `-g`, and `-g` is
            // either intercepted by the global CLI's `run_package_manager_command`
            // (managed flow) or rejected by the local CLI binding's
            // `execute_pm_command`. Either way, this arm only sees `dry_run: false`.
            dry_run: _,
            packages,
            pass_through_args,
        } => {
            let options = RemoveCommandOptions {
                packages: &packages,
                filters: filter.as_deref(),
                workspace_root,
                recursive,
                global,
                save_dev,
                save_optional,
                save_prod,
                pass_through_args: pass_through_args.as_deref(),
            };
            handlers::run_remove(cwd, &options).await
        }

        PackageManagerCommand::Update {
            latest,
            global: _,
            recursive,
            filter,
            workspace_root,
            dev,
            prod,
            interactive,
            no_optional,
            no_save,
            workspace,
            packages,
            pass_through_args,
        } => {
            let options = UpdateCommandOptions {
                packages: &packages,
                latest,
                recursive,
                filters: filter.as_deref(),
                workspace_root,
                dev,
                prod,
                interactive,
                no_optional,
                no_save,
                workspace_only: workspace,
                pass_through_args: pass_through_args.as_deref(),
            };
            handlers::run_update(cwd, &options).await
        }

        PackageManagerCommand::Dedupe { check, pass_through_args } => {
            let options =
                DedupeCommandOptions { check, pass_through_args: pass_through_args.as_deref() };
            handlers::run_dedupe(cwd, &options).await
        }

        PackageManagerCommand::Outdated {
            packages,
            long,
            format,
            recursive,
            filter,
            workspace_root,
            prod,
            dev,
            no_optional,
            compatible,
            sort_by,
            global,
            pass_through_args,
        } => {
            let options = OutdatedCommandOptions {
                packages: &packages,
                long,
                format,
                recursive,
                filters: filter.as_deref(),
                workspace_root,
                prod,
                dev,
                no_optional,
                compatible,
                sort_by: sort_by.as_deref(),
                global,
                pass_through_args: pass_through_args.as_deref(),
            };
            handlers::run_outdated(cwd, &options).await
        }

        PackageManagerCommand::Why {
            packages,
            json,
            long,
            parseable,
            recursive,
            filter,
            workspace_root,
            prod,
            dev,
            depth,
            no_optional,
            global,
            exclude_peers,
            find_by,
            pass_through_args,
        } => {
            let options = WhyCommandOptions {
                packages: &packages,
                json,
                long,
                parseable,
                recursive,
                filters: filter.as_deref(),
                workspace_root,
                prod,
                dev,
                depth,
                no_optional,
                global,
                exclude_peers,
                find_by: find_by.as_deref(),
                pass_through_args: pass_through_args.as_deref(),
            };
            handlers::run_why(cwd, &options).await
        }

        PackageManagerCommand::Info { package, field, json, pass_through_args } => {
            let options = ViewCommandOptions {
                package: &package,
                field: field.as_deref(),
                json,
                pass_through_args: pass_through_args.as_deref(),
            };
            handlers::run_info(cwd, &options).await
        }

        PackageManagerCommand::Link { package, args } => {
            let options = LinkCommandOptions {
                package: package.as_deref(),
                pass_through_args: pass_through_slice(&args),
            };
            handlers::run_link(cwd, &options).await
        }

        PackageManagerCommand::Unlink { package, recursive, args } => {
            let options = UnlinkCommandOptions {
                package: package.as_deref(),
                recursive,
                pass_through_args: pass_through_slice(&args),
            };
            handlers::run_unlink(cwd, &options).await
        }

        PackageManagerCommand::Dlx { package, shell_mode, silent, args } => {
            handlers::run_dlx(cwd, package, shell_mode, silent, args).await
        }

        PackageManagerCommand::Pm(pm_command) => handlers::run_pm_subcommand(cwd, pm_command).await,
    }
}

fn catalog_name<'a>(save_catalog: bool, save_catalog_name: Option<&'a str>) -> Option<&'a str> {
    if save_catalog { Some("default") } else { save_catalog_name }
}

fn pass_through_slice(args: &[String]) -> Option<&[String]> {
    if args.is_empty() { None } else { Some(args) }
}
