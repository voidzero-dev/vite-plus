//! Implementation of `vp env use` command.
//!
//! Outputs shell-appropriate commands to stdout that set (or unset)
//! the Node.js and package-manager environment variables. The shell function
//! wrapper in `<CONFIG>/env` evals this output to modify the current
//! shell session.
//!
//! All user-facing status messages go to stderr so they don't interfere
//! with the eval'd output.

use std::process::ExitStatus;

use vp_pm_cli::{PackageManagerType, download_package_manager, resolve_package_manager_version};
use vt_path::AbsolutePathBuf;

use super::{
    config::{self, PACKAGE_MANAGER_ENV_VAR, VERSION_ENV_VAR},
    exit_status, package_manager,
    spec::{EnvScope, EnvSpecs},
};
use crate::{
    commands::shell::{Shell, detect_shell},
    error::Error,
};

/// Format a shell export command for the detected shell.
fn format_export(shell: &Shell, variable: &str, value: &str) -> String {
    match shell {
        Shell::Posix => format!("export {variable}={value}"),
        Shell::Fish => format!("set -gx {variable} {value}"),
        Shell::PowerShell => format!("$env:{variable} = \"{value}\""),
        Shell::Cmd => format!("set {variable}={value}"),
        Shell::NuShell => format!("$env.{variable} = \"{value}\""),
    }
}

/// Format a shell unset command for the detected shell.
fn format_unset(shell: &Shell, variable: &str) -> String {
    match shell {
        Shell::Posix => format!("unset {variable}"),
        // Fish returns a nonzero status when the variable is already absent.
        // Keep the unset idempotent so wrappers can continue evaluating any
        // following command, such as the project-file export from `vp env use`.
        Shell::Fish => format!("set -e {variable}; or true"),
        Shell::PowerShell => {
            format!("Remove-Item Env:{variable} -ErrorAction SilentlyContinue")
        }
        Shell::Cmd => format!("set {variable}="),
        Shell::NuShell => format!("hide-env {variable}"),
    }
}

/// Whether the shell eval wrapper is active.
/// When true, the wrapper will eval our stdout to set env vars — no session file needed.
/// When false (CI, direct invocation), we write a session file so shims can read it.
fn has_eval_wrapper() -> bool {
    vp_shared::EnvConfig::get().env_use_eval_enable
}

fn can_use_session_file() -> bool {
    cfg!(not(windows)) || vp_shared::EnvConfig::get().is_ci
}

fn print_windows_eval_wrapper_required() {
    let env_ps1 = vp_shared::EnvConfig::get().dirs.config.join("env.ps1");
    eprintln!(
        "vp env use on Windows requires the Vite+ PowerShell wrapper to affect only the current shell session."
    );
    eprintln!("Add this line to your PowerShell $PROFILE:");
    eprintln!("  . \"{}\"", env_ps1.as_path().display());
    eprintln!("Then dot-source it now (or open a new PowerShell session) to load the wrapper.");
}

fn package_manager_spec(
    package_manager: PackageManagerType,
    version: &str,
    hash: Option<&str>,
) -> Result<String, Error> {
    let mut spec = format!("{package_manager}@{version}");
    if let Some(hash) = hash {
        if hash.is_empty()
            || !hash.bytes().all(|byte| {
                byte.is_ascii_alphanumeric()
                    || matches!(byte, b'.' | b'-' | b'_' | b'/' | b'+' | b'=')
            })
        {
            return Err(Error::Other(
                format!("invalid package-manager integrity suffix {hash:?}").into(),
            ));
        }
        spec.push('+');
        spec.push_str(hash);
    }
    Ok(spec)
}

/// Execute the `vp env use` command.
pub async fn execute(
    cwd: AbsolutePathBuf,
    requests: Vec<String>,
    unset: bool,
    no_install: bool,
    silent_if_unchanged: bool,
) -> Result<ExitStatus, Error> {
    let shell = detect_shell();
    let (scope, specs) = EnvSpecs::parse_requests(&requests)?;
    let uses_project_environment = specs.node.is_none() && specs.package_manager.is_none();

    // Handle --unset: remove session override.
    // Always delete the session file: on Windows it lives under VP_HOME and can
    // leak across shell windows, so even eval mode must clean it up.
    if unset {
        let session_package_manager = config::read_session_package_manager().await;
        let environment_package_manager = vp_shared::EnvConfig::get().package_manager.clone();
        let (delete_session_package_manager, unset_environment_package_manager) = match scope {
            EnvScope::PackageManager(expected) => (
                package_manager_matches(session_package_manager.as_deref(), expected),
                package_manager_matches(environment_package_manager.as_deref(), expected),
            ),
            _ => (scope.includes_package_managers(), scope.includes_package_managers()),
        };
        if scope.includes_node() {
            config::delete_session_version().await?;
        }
        if delete_session_package_manager {
            config::delete_session_package_manager().await?;
        }
        if has_eval_wrapper() {
            if scope.includes_node() {
                println!("{}", format_unset(&shell, VERSION_ENV_VAR));
            }
            if unset_environment_package_manager {
                println!("{}", format_unset(&shell, PACKAGE_MANAGER_ENV_VAR));
            }
        } else if !can_use_session_file() {
            print_windows_eval_wrapper_required();
        }
        eprintln!("Reverted selected components to project environment resolution");
        return Ok(ExitStatus::default());
    }

    let provider = vp_js_runtime::NodeProvider::new();
    let node = if scope.includes_node() {
        let (version, source) = if let Some(selector) = specs.node.as_deref() {
            (config::resolve_version_alias(selector, &provider).await?, selector.to_string())
        } else {
            let resolution = config::resolve_version_from_files(&cwd).await?;
            (resolution.version, resolution.source)
        };
        Some((version, source))
    } else {
        None
    };

    let package_manager = if scope.includes_package_managers() {
        if let Some((kind, selector, hash)) = specs.package_manager {
            let version = resolve_package_manager_version(kind, &selector).await?.to_string();
            package_manager::warn_if_target_differs(&cwd, kind).await;
            Some((kind, version, selector, hash))
        } else if let EnvScope::PackageManager(kind) = scope {
            package_manager::warn_if_target_differs(&cwd, kind).await;
            let resolution =
                package_manager::resolve_from_files_or_fallback_for(&cwd, kind).await?;
            Some((
                resolution.package_manager_type,
                resolution.version.to_string(),
                resolution.source.to_string(),
                resolution.hash.map(|hash| hash.to_string()),
            ))
        } else {
            package_manager::resolve_from_files_for(&cwd, scope.package_manager()).await?.map(
                |resolution| {
                    (
                        resolution.package_manager_type,
                        resolution.version.to_string(),
                        resolution.source.to_string(),
                        resolution.hash.map(|hash| hash.to_string()),
                    )
                },
            )
        }
    } else {
        None
    };

    // Check if already active and suppress output if requested.
    let unchanged = if silent_if_unchanged {
        let node_unchanged = match &node {
            Some((version, _)) => {
                current_override(
                    config::read_session_version().await,
                    vp_shared::EnvConfig::get().node_version.clone(),
                )
                .as_deref()
                    == Some(version)
            }
            None => true,
        };
        let package_manager_unchanged = match &package_manager {
            Some((kind, version, _, hash)) => {
                let spec = package_manager_spec(*kind, version, hash.as_deref())?;
                current_override(
                    config::read_session_package_manager().await,
                    vp_shared::EnvConfig::get().package_manager.clone(),
                )
                .as_deref()
                    == Some(spec.as_str())
            }
            None if uses_project_environment && scope.includes_package_managers() => {
                current_override(
                    config::read_session_package_manager().await,
                    vp_shared::EnvConfig::get().package_manager.clone(),
                )
                .is_none()
            }
            None => true,
        };
        node_unchanged && package_manager_unchanged
    } else {
        false
    };
    if unchanged {
        return Ok(ExitStatus::default());
    }

    if uses_project_environment && !has_eval_wrapper() && !can_use_session_file() {
        if scope.includes_node() {
            config::delete_session_version().await?;
        }
        if scope.includes_package_managers() {
            config::delete_session_package_manager().await?;
        }
        eprintln!("Reverted selected components to project environment resolution");
        print_windows_eval_wrapper_required();
        return Ok(ExitStatus::default());
    }

    if !no_install {
        ensure_components_installed(&node, &package_manager).await?;
    }

    if has_eval_wrapper() {
        if let Some((version, _)) = &node {
            config::delete_session_version().await?;
            println!("{}", format_export(&shell, VERSION_ENV_VAR, version));
        }
        if let Some((kind, version, _, hash)) = &package_manager {
            config::delete_session_package_manager().await?;
            println!(
                "{}",
                format_export(
                    &shell,
                    PACKAGE_MANAGER_ENV_VAR,
                    &package_manager_spec(*kind, version, hash.as_deref())?
                )
            );
        } else if uses_project_environment && scope.includes_package_managers() {
            config::delete_session_package_manager().await?;
            println!("{}", format_unset(&shell, PACKAGE_MANAGER_ENV_VAR));
        }
    } else if !can_use_session_file() {
        print_windows_eval_wrapper_required();
        return Ok(exit_status(1));
    } else {
        // No eval wrapper (CI or direct invocation) — write session file so shims can read it
        if let Some((version, _)) = &node {
            config::write_session_version(version).await?;
        }
        if let Some((kind, version, _, hash)) = &package_manager {
            config::write_session_package_manager(&package_manager_spec(
                *kind,
                version,
                hash.as_deref(),
            )?)
            .await?;
        } else if uses_project_environment && scope.includes_package_managers() {
            config::delete_session_package_manager().await?;
        }
    }

    if let Some((version, source)) = node {
        eprintln!("Using Node.js v{version} (resolved from {source})");
    }
    if let Some((kind, version, source, _)) = package_manager {
        eprintln!("Using {kind} v{version} (resolved from {source})");
    }

    Ok(ExitStatus::default())
}

fn package_manager_matches(value: Option<&str>, expected: PackageManagerType) -> bool {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| super::spec::parse_package_manager_spec(value).ok())
        .is_some_and(|(kind, _)| kind == expected)
}

async fn ensure_components_installed(
    node: &Option<(String, String)>,
    package_manager: &Option<(PackageManagerType, String, String, Option<String>)>,
) -> Result<(), Error> {
    if let Some((resolved_version, _)) = node {
        let home_dir = vp_shared::EnvConfig::get()
            .dirs
            .data
            .join("js_runtime")
            .join("node")
            .join(resolved_version);

        #[cfg(windows)]
        let binary_path = home_dir.join("node.exe");
        #[cfg(not(windows))]
        let binary_path = home_dir.join("bin").join("node");

        if !binary_path.as_path().exists() {
            eprintln!("Installing Node.js v{}...", resolved_version);
            vp_js_runtime::download_runtime(vp_js_runtime::JsRuntimeType::Node, resolved_version)
                .await?;
        }
    }
    if let Some((kind, version, _, hash)) = package_manager {
        download_package_manager(*kind, version, hash.as_deref()).await?;
    }
    Ok(())
}

fn current_override(session: Option<String>, environment: Option<String>) -> Option<String> {
    environment.map(|value| value.trim().to_string()).filter(|value| !value.is_empty()).or(session)
}

#[cfg(test)]
mod tests {
    use vp_shared::env_vars;

    use super::*;

    #[test]
    fn test_detect_shell_vp_shell_powershell() {
        vp_shared::EnvConfig::with_vars(
            [
                (env_vars::VP_HOME, std::env::temp_dir().as_os_str()),
                (env_vars::VP_SHELL, std::ffi::OsStr::new("powershell")),
            ],
            |_| {
                let shell = detect_shell();
                assert_eq!(shell, Shell::PowerShell);
            },
        );
    }

    #[test]
    fn empty_environment_override_falls_back_to_session() {
        assert_eq!(
            current_override(Some("pnpm@10.18.0".into()), Some("   ".into())).as_deref(),
            Some("pnpm@10.18.0")
        );
    }

    #[test]
    fn test_detect_shell_vp_shell_fish() {
        vp_shared::EnvConfig::with_vars(
            [
                (env_vars::VP_HOME, std::env::temp_dir().as_os_str()),
                (env_vars::VP_SHELL, std::ffi::OsStr::new("fish")),
            ],
            |_| {
                let shell = detect_shell();
                assert_eq!(shell, Shell::Fish);
            },
        );
    }

    #[test]
    fn test_detect_shell_vp_shell_nu() {
        vp_shared::EnvConfig::with_vars(
            [
                (env_vars::VP_HOME, std::env::temp_dir().as_os_str()),
                (env_vars::VP_SHELL, std::ffi::OsStr::new("nu")),
            ],
            |_| {
                let shell = detect_shell();
                assert_eq!(shell, Shell::NuShell);
            },
        );
    }

    #[test]
    fn test_detect_shell_posix_default() {
        vp_shared::EnvConfig::with_vars([(env_vars::VP_HOME, std::env::temp_dir())], |_| {
            let shell = detect_shell();
            #[cfg(not(windows))]
            assert_eq!(shell, Shell::Posix);
            #[cfg(windows)]
            assert_eq!(shell, Shell::Cmd);
        });
    }

    #[test]
    fn test_format_export_posix() {
        let result = format_export(&Shell::Posix, VERSION_ENV_VAR, "20.18.0");
        assert_eq!(result, "export VP_NODE_VERSION=20.18.0");
    }

    #[test]
    fn test_format_export_fish() {
        let result = format_export(&Shell::Fish, VERSION_ENV_VAR, "20.18.0");
        assert_eq!(result, "set -gx VP_NODE_VERSION 20.18.0");
    }

    #[test]
    fn test_format_export_powershell() {
        let result = format_export(&Shell::PowerShell, VERSION_ENV_VAR, "20.18.0");
        assert_eq!(result, "$env:VP_NODE_VERSION = \"20.18.0\"");
    }

    #[test]
    fn test_format_export_cmd() {
        let result = format_export(&Shell::Cmd, VERSION_ENV_VAR, "20.18.0");
        assert_eq!(result, "set VP_NODE_VERSION=20.18.0");
    }

    #[test]
    fn test_format_unset_posix() {
        let result = format_unset(&Shell::Posix, VERSION_ENV_VAR);
        assert_eq!(result, "unset VP_NODE_VERSION");
    }

    #[test]
    fn test_format_unset_fish() {
        let result = format_unset(&Shell::Fish, VERSION_ENV_VAR);
        assert_eq!(result, "set -e VP_NODE_VERSION; or true");
    }

    #[test]
    fn test_format_unset_powershell() {
        let result = format_unset(&Shell::PowerShell, VERSION_ENV_VAR);
        assert_eq!(result, "Remove-Item Env:VP_NODE_VERSION -ErrorAction SilentlyContinue");
    }

    #[test]
    fn test_format_unset_cmd() {
        let result = format_unset(&Shell::Cmd, VERSION_ENV_VAR);
        assert_eq!(result, "set VP_NODE_VERSION=");
    }
    #[test]
    fn test_format_export_nushell() {
        let result = format_export(&Shell::NuShell, VERSION_ENV_VAR, "20.18.0");
        assert_eq!(result, "$env.VP_NODE_VERSION = \"20.18.0\"");
    }

    #[test]
    fn test_format_unset_nushell() {
        let result = format_unset(&Shell::NuShell, VERSION_ENV_VAR);
        assert_eq!(result, "hide-env VP_NODE_VERSION");
    }

    #[test]
    fn package_manager_spec_rejects_shell_metacharacters() {
        let error = package_manager_spec(
            PackageManagerType::Pnpm,
            "10.18.0",
            Some("sha512.valid; touch injected"),
        )
        .unwrap_err();

        assert!(error.to_string().contains("invalid package-manager integrity suffix"));
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn test_windows_direct_use_without_eval_wrapper_does_not_write_session_file() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let cwd = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        // CI runners export `CI` (GitHub Actions always does), and the
        // direct-use guard keys off `is_ci` — the `None` pin unsets it to
        // exercise the non-CI path.
        vp_shared::EnvConfig::with_vars_async(
            [(env_vars::VP_HOME, Some(temp_dir.path())), ("CI", None)],
            |_| async move {
                let status =
                    execute(cwd, vec!["20.18.0".into()], false, true, false).await.unwrap();

                assert_eq!(status.code(), Some(1));
                assert!(config::read_session_version().await.is_none());
            },
        )
        .await;
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn test_windows_ci_direct_use_writes_session_file() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let cwd = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        vp_shared::EnvConfig::with_vars_async(
            [(env_vars::VP_HOME, temp_dir.path().as_os_str()), ("CI", std::ffi::OsStr::new("1"))],
            |_| async {
                let status =
                    execute(cwd, vec!["20.18.0".into()], false, true, false).await.unwrap();

                assert!(status.success());
                assert_eq!(config::read_session_version().await.as_deref(), Some("20.18.0"));
            },
        )
        .await;
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn test_windows_eval_wrapper_cleans_legacy_session_file() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let cwd = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        vp_shared::EnvConfig::with_vars_async(
            [
                (env_vars::VP_HOME, temp_dir.path().as_os_str()),
                (env_vars::VP_ENV_USE_EVAL_ENABLE, std::ffi::OsStr::new("1")),
                (env_vars::VP_SHELL, std::ffi::OsStr::new("powershell")),
            ],
            |_| async {
                config::write_session_version("22.0.0").await.unwrap();

                let status =
                    execute(cwd, vec!["20.18.0".into()], false, true, false).await.unwrap();

                assert!(status.success());
                assert!(config::read_session_version().await.is_none());
            },
        )
        .await;
    }
}
