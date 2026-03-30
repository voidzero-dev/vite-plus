//! Implementation of `vp env use` command.
//!
//! Outputs shell-appropriate commands to stdout that set (or unset)
//! the `VP_NODE_VERSION` environment variable. The shell function
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
    let config = vite_shared::EnvConfig::get();
    if config.fish_version.is_some() {
        Shell::Fish
    } else if cfg!(windows) && config.ps_module_path.is_some() {
        Shell::PowerShell
    } else if cfg!(windows) {
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

/// Whether the shell eval wrapper is active.
/// When true, the wrapper will eval our stdout to set env vars — no session file needed.
/// When false (CI, direct invocation), we write a session file so shims can read it.
fn has_eval_wrapper() -> bool {
    vite_shared::EnvConfig::get().env_use_eval_enable
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
        if has_eval_wrapper() {
            println!("{}", format_unset(&shell));
        } else {
            config::delete_session_version().await?;
        }
        eprintln!("Reverted to file-based Node.js version resolution");
        return Ok(ExitStatus::default());
    }

    let provider = vite_js_runtime::NodeProvider::new();

    // Resolve version: explicit argument or from project files
    // When no argument provided, unset session override and resolve from project files
    let (resolved_version, source_desc) = if let Some(ref ver) = version {
        let resolved = config::resolve_version_alias(ver, &provider).await?;
        (resolved, format!("{ver}"))
    } else {
        // No version argument - unset session override first
        if has_eval_wrapper() {
            println!("{}", format_unset(&shell));
        } else {
            config::delete_session_version().await?;
        }
        // Now resolve from project files (not from session override)
        let resolution = config::resolve_version_from_files(&cwd).await?;
        let source = resolution.source.clone();
        (resolution.version, source)
    };

    // Check if already active and suppress output if requested
    if silent_if_unchanged {
        let current_env = vite_shared::EnvConfig::get().node_version.map(|v| v.trim().to_string());
        let current = if !has_eval_wrapper() {
            current_env.or(config::read_session_version().await)
        } else {
            current_env
        };
        if current.as_deref() == Some(&resolved_version) {
            // Already active — idempotent, skip stderr status message
            if has_eval_wrapper() {
                println!("{}", format_export(&shell, &resolved_version));
            } else {
                config::write_session_version(&resolved_version).await?;
            }
            return Ok(ExitStatus::default());
        }
    }

    // Ensure version is installed (unless --no-install)
    if !no_install {
        let home_dir = vite_shared::get_vp_home()
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

    if has_eval_wrapper() {
        // Output the shell command to stdout (consumed by shell wrapper's eval)
        println!("{}", format_export(&shell, &resolved_version));
    } else {
        // No eval wrapper (CI or direct invocation) — write session file so shims can read it
        config::write_session_version(&resolved_version).await?;
    }

    // Status message to stderr (visible to user)
    eprintln!("Using Node.js v{} (resolved from {})", resolved_version, source_desc);

    Ok(ExitStatus::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_shell_posix_even_with_psmodulepath() {
        let _guard = vite_shared::EnvConfig::test_guard(vite_shared::EnvConfig {
            ps_module_path: Some("/some/path".into()),
            ..vite_shared::EnvConfig::for_test()
        });
        let shell = detect_shell();
        #[cfg(not(windows))]
        assert!(matches!(shell, Shell::Posix));
        #[cfg(windows)]
        assert!(matches!(shell, Shell::PowerShell));
    }

    #[test]
    fn test_detect_shell_fish() {
        let _guard = vite_shared::EnvConfig::test_guard(vite_shared::EnvConfig {
            fish_version: Some("3.7.0".into()),
            ..vite_shared::EnvConfig::for_test()
        });
        let shell = detect_shell();
        assert!(matches!(shell, Shell::Fish));
    }

    #[test]
    fn test_detect_shell_posix_default() {
        // All shell detection fields None → defaults
        let _guard = vite_shared::EnvConfig::test_guard(vite_shared::EnvConfig::for_test());
        let shell = detect_shell();
        #[cfg(not(windows))]
        assert!(matches!(shell, Shell::Posix));
        #[cfg(windows)]
        assert!(matches!(shell, Shell::Cmd));
    }

    #[test]
    fn test_format_export_posix() {
        let result = format_export(&Shell::Posix, "20.18.0");
        assert_eq!(result, "export VP_NODE_VERSION=20.18.0");
    }

    #[test]
    fn test_format_export_fish() {
        let result = format_export(&Shell::Fish, "20.18.0");
        assert_eq!(result, "set -gx VP_NODE_VERSION 20.18.0");
    }

    #[test]
    fn test_format_export_powershell() {
        let result = format_export(&Shell::PowerShell, "20.18.0");
        assert_eq!(result, "$env:VP_NODE_VERSION = \"20.18.0\"");
    }

    #[test]
    fn test_format_export_cmd() {
        let result = format_export(&Shell::Cmd, "20.18.0");
        assert_eq!(result, "set VP_NODE_VERSION=20.18.0");
    }

    #[test]
    fn test_format_unset_posix() {
        let result = format_unset(&Shell::Posix);
        assert_eq!(result, "unset VP_NODE_VERSION");
    }

    #[test]
    fn test_format_unset_fish() {
        let result = format_unset(&Shell::Fish);
        assert_eq!(result, "set -e VP_NODE_VERSION");
    }

    #[test]
    fn test_format_unset_powershell() {
        let result = format_unset(&Shell::PowerShell);
        assert_eq!(result, "Remove-Item Env:VP_NODE_VERSION -ErrorAction SilentlyContinue");
    }

    #[test]
    fn test_format_unset_cmd() {
        let result = format_unset(&Shell::Cmd);
        assert_eq!(result, "set VP_NODE_VERSION=");
    }
}
