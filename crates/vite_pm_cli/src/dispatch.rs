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
use vite_install::PackageManager;
use vite_path::AbsolutePath;

use crate::{cli::PackageManagerCommand, error::Error, handlers};

pub async fn dispatch(
    cwd: &AbsolutePath,
    command: PackageManagerCommand,
) -> Result<ExitStatus, Error> {
    // Dlx is special-cased: `run_dlx` has a `PackageJsonNotFound → npx` fallback
    // that must be preserved. The `build_for_dispatch` path would propagate
    // `PackageJsonNotFound` via `?`, making the fallback dead code. Call `run_dlx`
    // directly to preserve the original behavior.
    if let PackageManagerCommand::Dlx { package, shell_mode, silent, args } = command {
        return handlers::run_dlx(cwd, package, shell_mode, silent, args).await;
    }
    let pm = handlers::build_for_dispatch(cwd, &command).await?;
    dispatch_with_pm(cwd, command, &pm).await
}

/// Dispatch using an externally-provided [`PackageManager`] (no internal build/pin).
///
/// Used by passthrough mode, where the pm comes from
/// `PackageManager::detect_only`. The match body is identical to the original
/// `dispatch` (zero behavior drift) — only the handler calls take the injected
/// `pm` via the `*_with_pm` variants.
pub async fn dispatch_with_pm(
    cwd: &AbsolutePath,
    command: PackageManagerCommand,
    pm: &PackageManager,
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
            concurrency: _,
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
                return handlers::run_add_with_pm(cwd, &options, pm).await;
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
            handlers::run_install_with_pm(cwd, &options, pm).await
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
            concurrency: _,
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
            handlers::run_add_with_pm(cwd, &options, pm).await
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
            handlers::run_remove_with_pm(cwd, &options, pm).await
        }

        PackageManagerCommand::Update {
            latest,
            global: _,
            concurrency: _,
            reinstall_node_mismatch: _,
            ignore_node_mismatch: _,
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
            handlers::run_update_with_pm(cwd, &options, pm).await
        }

        PackageManagerCommand::Dedupe { check, pass_through_args } => {
            let options =
                DedupeCommandOptions { check, pass_through_args: pass_through_args.as_deref() };
            handlers::run_dedupe_with_pm(cwd, &options, pm).await
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
            concurrency: _,
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
            handlers::run_outdated_with_pm(cwd, &options, pm).await
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
                exclude_peers,
                find_by: find_by.as_deref(),
                pass_through_args: pass_through_args.as_deref(),
            };
            handlers::run_why_with_pm(cwd, &options, pm).await
        }

        PackageManagerCommand::Info { package, field, json, pass_through_args } => {
            let options = ViewCommandOptions {
                package: &package,
                field: field.as_deref(),
                json,
                pass_through_args: pass_through_args.as_deref(),
            };
            handlers::run_info_with_pm(cwd, &options, pm).await
        }

        PackageManagerCommand::Link { package, args } => {
            let options = LinkCommandOptions {
                package: package.as_deref(),
                pass_through_args: pass_through_slice(&args),
            };
            handlers::run_link_with_pm(cwd, &options, pm).await
        }

        PackageManagerCommand::Unlink { package, recursive, args } => {
            let options = UnlinkCommandOptions {
                package: package.as_deref(),
                recursive,
                pass_through_args: pass_through_slice(&args),
            };
            handlers::run_unlink_with_pm(cwd, &options, pm).await
        }

        PackageManagerCommand::Dlx { package, shell_mode, silent, args } => {
            handlers::run_dlx_with_pm(cwd, package, shell_mode, silent, args, pm).await
        }

        PackageManagerCommand::Pm(pm_command) => {
            handlers::run_pm_subcommand_with_pm(cwd, pm_command, pm).await
        }
    }
}

fn catalog_name<'a>(save_catalog: bool, save_catalog_name: Option<&'a str>) -> Option<&'a str> {
    if save_catalog { Some("default") } else { save_catalog_name }
}

fn pass_through_slice(args: &[String]) -> Option<&[String]> {
    if args.is_empty() { None } else { Some(args) }
}
