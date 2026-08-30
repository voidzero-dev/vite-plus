//! Environment management commands.
//!
//! This module provides the `vp env` command for managing Node.js environments
//! through shim-based version management.

pub mod bin_config;
mod clean;
pub mod config;
mod current;
mod default;
mod doctor;
mod exec;
mod lifecycle;
mod list;
mod list_remote;
mod off;
mod on;
pub(crate) mod package_manager;
pub mod package_metadata;
mod pin;
pub(crate) mod setup;
mod spec;
mod unpin;
mod r#use;
mod which;

use std::process::ExitStatus;

#[cfg(windows)]
pub(crate) use setup::{cleanup_legacy_windows_shim, get_trampoline_path, remove_or_rename_to_old};
use vt_path::AbsolutePathBuf;

use crate::{
    cli::{EnvArgs, EnvSubcommands, exit_status},
    commands::shell::{Shell, detect_shell},
    error::Error,
};

fn print_env_header() {
    vp_shared::header::print_header();
}

fn print_env_clean_tip() {
    vp_shared::output::raw("");
    vp_shared::output::note(
        "Run `vp env clean` to free disk space from unused managed runtimes and package manager caches.",
    );
}

fn should_print_env_header(subcommand: &EnvSubcommands) -> bool {
    match subcommand {
        EnvSubcommands::Current { json, .. } => !json,
        EnvSubcommands::List { json, .. } => !json,
        EnvSubcommands::ListRemote { json, .. } => !json,
        // Keep these machine-consumable / passthrough commands header-free.
        EnvSubcommands::Use { .. } | EnvSubcommands::Exec { .. } => false,
        _ => true,
    }
}

fn should_print_env_clean_tip(subcommand: &EnvSubcommands) -> bool {
    match subcommand {
        EnvSubcommands::List { json, .. } => !json,
        EnvSubcommands::ListRemote { json, .. } => !json,
        _ => false,
    }
}

/// Execute the env command based on the provided arguments.
pub async fn execute(cwd: AbsolutePathBuf, args: EnvArgs) -> Result<ExitStatus, Error> {
    // Handle subcommands first
    if let Some(subcommand) = args.command {
        if should_print_env_header(&subcommand) {
            print_env_header();
        }
        let should_print_tip = should_print_env_clean_tip(&subcommand);

        let result = match subcommand {
            crate::cli::EnvSubcommands::Current { scope, json } => {
                current::execute(cwd, scope, json).await
            }
            crate::cli::EnvSubcommands::Print { scope } => print_env(cwd, scope).await,
            crate::cli::EnvSubcommands::Default { values, unset } => {
                default::execute(values, unset).await
            }
            crate::cli::EnvSubcommands::On { scope } => on::execute(scope).await,
            crate::cli::EnvSubcommands::Off { scope } => off::execute(scope).await,
            crate::cli::EnvSubcommands::Setup { refresh, env_only } => {
                setup::execute(refresh, env_only).await
            }
            crate::cli::EnvSubcommands::Doctor { scope } => doctor::execute(cwd, scope).await,
            crate::cli::EnvSubcommands::Which { tool } => which::execute(cwd, &tool).await,
            crate::cli::EnvSubcommands::Pin { specs, unpin, no_install, force, target } => {
                pin::execute(cwd, specs, unpin, no_install, force, target).await
            }
            crate::cli::EnvSubcommands::Unpin { scope, target } => {
                unpin::execute(cwd, scope, target).await
            }
            crate::cli::EnvSubcommands::List { scope, json } => {
                list::execute(cwd, scope, json).await
            }
            crate::cli::EnvSubcommands::ListRemote { values, lts, all, json, sort } => {
                list_remote::execute(cwd, values, lts, all, json, sort).await
            }
            crate::cli::EnvSubcommands::Exec { node, npm, package_manager, command } => {
                exec::execute(
                    &cwd,
                    node.as_deref(),
                    npm.as_deref(),
                    package_manager.as_deref(),
                    &command,
                )
                .await
            }
            crate::cli::EnvSubcommands::Uninstall { specs } => lifecycle::uninstall(specs).await,
            crate::cli::EnvSubcommands::Clean { scope } => clean::execute(cwd, scope).await,
            crate::cli::EnvSubcommands::Use {
                requests,
                unset,
                no_install,
                silent_if_unchanged,
            } => r#use::execute(cwd, requests, unset, no_install, silent_if_unchanged).await,
            crate::cli::EnvSubcommands::Install { requests } => {
                lifecycle::install(cwd, requests).await
            }
        };

        if matches!(&result, Ok(status) if status.success()) && should_print_tip {
            print_env_clean_tip();
        }

        return result;
    }

    // No subcommand provided - show unified help to match `vp env --help`.
    if !crate::help::print_unified_clap_help_for_path(&["env"]) {
        // Fallback to clap's built-in help printer if unified rendering fails.
        use clap::CommandFactory;
        vp_shared::header::print_header();
        crate::cli::Args::command()
            .find_subcommand("env")
            .unwrap()
            .clone()
            .disable_help_subcommand(true)
            .print_help()
            .ok();
    }
    Ok(ExitStatus::default())
}

/// Print shell snippet for setting environment (`vp env print`)
async fn print_env(cwd: AbsolutePathBuf, scope: Option<String>) -> Result<ExitStatus, Error> {
    let scope = spec::EnvScope::parse(scope.as_deref())?;
    let modes = config::load_config().await?;
    let mut bin_dirs = Vec::new();
    if scope.includes_node() {
        bin_dirs.push(resolve_node_bin_dir(&cwd, &modes).await?.as_path().display().to_string());
    }
    if scope.includes_package_managers() {
        let selected = package_manager::resolve_current_spec(&cwd).await?.filter(|resolution| {
            scope
                .package_manager()
                .is_none_or(|expected| expected == resolution.package_manager_type)
        });
        let selected_type = selected
            .as_ref()
            .map(|resolution| resolution.package_manager_type)
            .or_else(|| scope.package_manager());
        let system_bin_dir = selected_type.and_then(|package_manager| {
            if modes.package_manager_shim_mode_for(package_manager) == config::ShimMode::SystemFirst
            {
                crate::shim::dispatch::find_system_tool(&package_manager.to_string())
                    .and_then(|path| path.parent().map(vt_path::AbsolutePath::to_absolute_path_buf))
            } else {
                None
            }
        });
        if let Some(bin_dir) = system_bin_dir {
            bin_dirs.insert(0, bin_dir.as_path().display().to_string());
        } else {
            let resolution = match scope.package_manager() {
                Some(package_manager) => Some(
                    package_manager::resolve_current_or_fallback_for(&cwd, package_manager).await?,
                ),
                None => package_manager::resolve_current_for(&cwd, None).await?,
            };
            if let Some(resolution) = resolution {
                let (install_dir, _, _) = vp_pm_cli::download_package_manager(
                    resolution.package_manager_type,
                    &resolution.version,
                    resolution.hash.as_deref(),
                )
                .await?;
                bin_dirs.insert(0, install_dir.join("bin").as_path().display().to_string());
            }
        }
    }
    if bin_dirs.is_empty() {
        return Err(Error::Other("no selected environment component could be resolved".into()));
    }
    let snippet = format_path_snippet(detect_shell(), &bin_dirs);

    // Print shell snippet
    println!("# Add to your shell to use this environment for this session:");
    println!("{snippet}");

    Ok(ExitStatus::default())
}

async fn resolve_node_bin_dir(
    cwd: &vt_path::AbsolutePath,
    config: &config::Config,
) -> Result<AbsolutePathBuf, Error> {
    if config.node_shim_mode == config::ShimMode::SystemFirst
        && let Some(path) = crate::shim::dispatch::find_system_tool("node")
        && let Some(bin_dir) = path.parent()
    {
        return Ok(bin_dir.to_absolute_path_buf());
    }

    let resolution = config::resolve_version(cwd).await?;
    let runtime =
        vp_js_runtime::download_runtime(vp_js_runtime::JsRuntimeType::Node, &resolution.version)
            .await?;
    Ok(runtime.get_bin_prefix())
}

fn format_path_snippet(shell: Shell, bin_dirs: &[String]) -> String {
    match shell {
        Shell::Posix => format!(
            "export PATH=\"{}:$PATH\"",
            bin_dirs
                .iter()
                .map(|path| setup::escape_posix_double_quoted_string(path))
                .collect::<Vec<_>>()
                .join(":")
        ),
        Shell::Fish => format!(
            "set -gx PATH {} $PATH",
            bin_dirs
                .iter()
                .map(|path| format!("\"{}\"", setup::escape_fish_double_quoted_string(path)))
                .collect::<Vec<_>>()
                .join(" ")
        ),
        Shell::PowerShell => format!(
            "$env:PATH = '{};' + $env:PATH",
            bin_dirs
                .iter()
                .map(|path| setup::escape_powershell_single_quoted_string(path))
                .collect::<Vec<_>>()
                .join(";")
        ),
        Shell::Cmd => format!(
            "set \"PATH={};%PATH%\"",
            bin_dirs.iter().map(|path| path.replace('%', "%%")).collect::<Vec<_>>().join(";")
        ),
        Shell::NuShell => format!(
            "$env.PATH = ($env.PATH | prepend [{}])",
            bin_dirs
                .iter()
                .map(|path| format!("\"{}\"", setup::escape_nu_double_quoted_string(path)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fish_path_snippet_quotes_each_directory() {
        let snippet = format_path_snippet(
            Shell::Fish,
            &["/Users/Example User/node/bin".into(), "/tmp/pm/bin".into()],
        );

        assert_eq!(snippet, "set -gx PATH \"/Users/Example User/node/bin\" \"/tmp/pm/bin\" $PATH");
    }

    #[test]
    fn path_snippets_escape_shell_metacharacters() {
        assert_eq!(
            format_path_snippet(Shell::Posix, &[r#"/tmp/$USER `tick` \ dir"#.into()]),
            r#"export PATH="/tmp/\$USER \`tick\` \\ dir:$PATH""#
        );
        assert_eq!(
            format_path_snippet(Shell::PowerShell, &[r#"C:\A&B's"#.into()]),
            r#"$env:PATH = 'C:\A&B''s;' + $env:PATH"#
        );
        assert_eq!(
            format_path_snippet(Shell::Cmd, &[r#"C:\%literal%\A&B"#.into()]),
            r#"set "PATH=C:\%%literal%%\A&B;%PATH%""#
        );
        assert_eq!(
            format_path_snippet(Shell::NuShell, &[r#"C:\A "B""#.into()]),
            r#"$env.PATH = ($env.PATH | prepend ["C:\\A \"B\""])"#
        );
    }
}
