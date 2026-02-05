//! Implementation of `vp env use` command.
//!
//! Outputs shell-appropriate commands to stdout that set (or unset)
//! the `VITE_PLUS_NODE_VERSION` environment variable. The shell function
//! wrapper in `~/.vite-plus/env` evals this output to modify the current
//! shell session.
//!
//! All user-facing status messages go to stderr so they don't interfere
//! with the eval'd output.

use std::process::ExitStatus;

use vite_path::AbsolutePathBuf;

use super::config::{self, VERSION_ENV_VAR};
use crate::error::Error;

/// Detected shell type for output formatting.
enum Shell {
    /// POSIX shell (bash, zsh, sh)
    Posix,
    /// Fish shell
    Fish,
    /// PowerShell
    PowerShell,
    /// Windows cmd.exe
    Cmd,
}

/// Detect the current shell from environment variables.
fn detect_shell() -> Shell {
    if std::env::var("FISH_VERSION").is_ok() {
        Shell::Fish
    } else if std::env::var("PSModulePath").is_ok() {
        Shell::PowerShell
    } else if std::env::var("COMSPEC").is_ok() {
        Shell::Cmd
    } else {
        Shell::Posix
    }
}

/// Format a shell export command for the detected shell.
fn format_export(shell: &Shell, value: &str) -> String {
    match shell {
        Shell::Posix => format!("export {VERSION_ENV_VAR}={value}"),
        Shell::Fish => format!("set -gx {VERSION_ENV_VAR} {value}"),
        Shell::PowerShell => format!("$env:{VERSION_ENV_VAR} = \"{value}\""),
        Shell::Cmd => format!("set {VERSION_ENV_VAR}={value}"),
    }
}

/// Format a shell unset command for the detected shell.
fn format_unset(shell: &Shell) -> String {
    match shell {
        Shell::Posix => format!("unset {VERSION_ENV_VAR}"),
        Shell::Fish => format!("set -e {VERSION_ENV_VAR}"),
        Shell::PowerShell => {
            format!("Remove-Item Env:{VERSION_ENV_VAR} -ErrorAction SilentlyContinue")
        }
        Shell::Cmd => format!("set {VERSION_ENV_VAR}="),
    }
}

/// Execute the `vp env use` command.
pub async fn execute(
    cwd: AbsolutePathBuf,
    version: Option<String>,
    unset: bool,
    no_install: bool,
    silent_if_unchanged: bool,
) -> Result<ExitStatus, Error> {
    let shell = detect_shell();

    // Handle --unset: remove session override
    if unset {
        println!("{}", format_unset(&shell));
        eprintln!("Reverted to file-based Node.js version resolution");
        return Ok(ExitStatus::default());
    }

    let provider = vite_js_runtime::NodeProvider::new();

    // Resolve version: explicit argument or from project files
    let (resolved_version, source_desc) = if let Some(ref ver) = version {
        let resolved = config::resolve_version_alias(ver, &provider).await?;
        (resolved, format!("{ver}"))
    } else {
        let resolution = config::resolve_version(&cwd).await?;
        let source = resolution.source.clone();
        (resolution.version, source)
    };

    // Check if already active and suppress output if requested
    if silent_if_unchanged {
        if let Ok(current) = std::env::var(VERSION_ENV_VAR) {
            if current.trim() == resolved_version {
                // Already active, output the export anyway (idempotent) but skip stderr
                println!("{}", format_export(&shell, &resolved_version));
                return Ok(ExitStatus::default());
            }
        }
    }

    // Ensure version is installed (unless --no-install)
    if !no_install {
        let home_dir = vite_shared::get_vite_plus_home()
            .map_err(|e| Error::ConfigError(format!("{e}").into()))?
            .join("js_runtime")
            .join("node")
            .join(&resolved_version);

        #[cfg(windows)]
        let binary_path = home_dir.join("node.exe");
        #[cfg(not(windows))]
        let binary_path = home_dir.join("bin").join("node");

        if !binary_path.as_path().exists() {
            eprintln!("Installing Node.js v{}...", resolved_version);
            vite_js_runtime::download_runtime(
                vite_js_runtime::JsRuntimeType::Node,
                &resolved_version,
            )
            .await?;
        }
    }

    // Output the shell command to stdout (consumed by shell wrapper's eval)
    println!("{}", format_export(&shell, &resolved_version));

    // Status message to stderr (visible to user)
    eprintln!("Using Node.js v{} (resolved from {})", resolved_version, source_desc);

    Ok(ExitStatus::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_export_posix() {
        let result = format_export(&Shell::Posix, "20.18.0");
        assert_eq!(result, "export VITE_PLUS_NODE_VERSION=20.18.0");
    }

    #[test]
    fn test_format_export_fish() {
        let result = format_export(&Shell::Fish, "20.18.0");
        assert_eq!(result, "set -gx VITE_PLUS_NODE_VERSION 20.18.0");
    }

    #[test]
    fn test_format_export_powershell() {
        let result = format_export(&Shell::PowerShell, "20.18.0");
        assert_eq!(result, "$env:VITE_PLUS_NODE_VERSION = \"20.18.0\"");
    }

    #[test]
    fn test_format_export_cmd() {
        let result = format_export(&Shell::Cmd, "20.18.0");
        assert_eq!(result, "set VITE_PLUS_NODE_VERSION=20.18.0");
    }

    #[test]
    fn test_format_unset_posix() {
        let result = format_unset(&Shell::Posix);
        assert_eq!(result, "unset VITE_PLUS_NODE_VERSION");
    }

    #[test]
    fn test_format_unset_fish() {
        let result = format_unset(&Shell::Fish);
        assert_eq!(result, "set -e VITE_PLUS_NODE_VERSION");
    }

    #[test]
    fn test_format_unset_powershell() {
        let result = format_unset(&Shell::PowerShell);
        assert_eq!(result, "Remove-Item Env:VITE_PLUS_NODE_VERSION -ErrorAction SilentlyContinue");
    }

    #[test]
    fn test_format_unset_cmd() {
        let result = format_unset(&Shell::Cmd);
        assert_eq!(result, "set VITE_PLUS_NODE_VERSION=");
    }
}
