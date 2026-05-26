//! Shared shell detection and profile helpers.

use std::str::FromStr;

use directories::BaseDirs;
use vite_path::{AbsolutePath, AbsolutePathBuf};
use vite_str::Str;

/// Detected shell type for output formatting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shell {
    /// POSIX shell (bash, zsh, sh)
    Posix,
    /// Fish shell
    Fish,
    /// Nushell
    NuShell,
    /// PowerShell
    PowerShell,
    /// Windows cmd.exe
    Cmd,
}

impl FromStr for Shell {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "sh" | "bash" | "zsh" => Ok(Shell::Posix),
            "fish" => Ok(Shell::Fish),
            "nu" | "nushell" => Ok(Shell::NuShell),
            "pwsh" | "powershell" => Ok(Shell::PowerShell),
            "cmd" => Ok(Shell::Cmd),
            _ => Err(()),
        }
    }
}

/// Detect the current shell:
/// 1. `VP_SHELL` environment variable
/// 2. Platform default
#[must_use]
pub fn detect_shell() -> Shell {
    let config = vite_shared::EnvConfig::get();

    // 1. Check VP_SHELL environment variable
    if let Some(vp_shell) = &config.vp_shell {
        if let Ok(shell) = Shell::from_str(vp_shell) {
            return shell;
        }
    }

    // 2. Platform default
    if cfg!(windows) { Shell::Cmd } else { Shell::Posix }
}

/// All shell profile files that interactive terminal sessions may source.
/// This matches the files that `install.sh` writes to and `vp implode` cleans.
#[cfg(not(windows))]
pub const ALL_SHELL_PROFILES: &[ShellProfile] = &[
    ShellProfile {
        root: ShellProfileRoot::Zsh,
        path: ".zshenv",
        env_file: "env",
        kind: ShellProfileKind::Main,
    },
    ShellProfile {
        root: ShellProfileRoot::Zsh,
        path: ".zshrc",
        env_file: "env",
        kind: ShellProfileKind::Main,
    },
    ShellProfile {
        root: ShellProfileRoot::Home,
        path: ".bash_profile",
        env_file: "env",
        kind: ShellProfileKind::Main,
    },
    ShellProfile {
        root: ShellProfileRoot::Home,
        path: ".bashrc",
        env_file: "env",
        kind: ShellProfileKind::Main,
    },
    ShellProfile {
        root: ShellProfileRoot::Home,
        path: ".profile",
        env_file: "env",
        kind: ShellProfileKind::Main,
    },
    ShellProfile {
        root: ShellProfileRoot::Fish,
        path: "fish/config.fish",
        env_file: "env.fish",
        kind: ShellProfileKind::Main,
    },
    ShellProfile {
        root: ShellProfileRoot::Fish,
        path: "fish/conf.d/vite-plus.fish",
        env_file: "env.fish",
        kind: ShellProfileKind::Snippet,
    },
    ShellProfile {
        root: ShellProfileRoot::NushellConfig,
        path: "nushell/config.nu",
        env_file: "env.nu",
        kind: ShellProfileKind::Main,
    },
    ShellProfile {
        root: ShellProfileRoot::NushellConfig,
        path: "nushell/env.nu",
        env_file: "env.nu",
        kind: ShellProfileKind::Main,
    },
    ShellProfile {
        root: ShellProfileRoot::NushellData,
        path: "nushell/vendor/autoload/vite-plus.nu",
        env_file: "env.nu",
        kind: ShellProfileKind::Snippet,
    },
];

#[cfg(windows)]
pub const ALL_SHELL_PROFILES: &[ShellProfile] = &[
    ShellProfile {
        root: ShellProfileRoot::NushellConfig,
        path: "nushell/config.nu",
        env_file: "env.nu",
        kind: ShellProfileKind::Main,
    },
    ShellProfile {
        root: ShellProfileRoot::NushellConfig,
        path: "nushell/env.nu",
        env_file: "env.nu",
        kind: ShellProfileKind::Main,
    },
    ShellProfile {
        root: ShellProfileRoot::NushellData,
        path: "nushell/vendor/autoload/vite-plus.nu",
        env_file: "env.nu",
        kind: ShellProfileKind::Snippet,
    },
];

/// IDE-relevant profile files that GUI-launched applications can see.
/// GUI apps don't run through an interactive shell, so only login/environment
/// files reliably affect them.
/// - macOS: `.zshenv` is sourced for all zsh invocations (including IDE env resolution)
/// - Linux: `.profile` is sourced by X11 display managers; `.zshenv` covers Wayland + zsh
#[cfg(target_os = "macos")]
pub const IDE_SHELL_PROFILES: &[ShellProfile] = &[
    ShellProfile {
        root: ShellProfileRoot::Zsh,
        path: ".zshenv",
        env_file: "env",
        kind: ShellProfileKind::Main,
    },
    ShellProfile {
        root: ShellProfileRoot::Home,
        path: ".profile",
        env_file: "env",
        kind: ShellProfileKind::Main,
    },
];

#[cfg(target_os = "linux")]
pub const IDE_SHELL_PROFILES: &[ShellProfile] = &[
    ShellProfile {
        root: ShellProfileRoot::Home,
        path: ".profile",
        env_file: "env",
        kind: ShellProfileKind::Main,
    },
    ShellProfile {
        root: ShellProfileRoot::Zsh,
        path: ".zshenv",
        env_file: "env",
        kind: ShellProfileKind::Main,
    },
];

#[cfg(windows)]
pub const IDE_SHELL_PROFILES: &[ShellProfile] = &[];

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
pub const IDE_SHELL_PROFILES: &[ShellProfile] = &[ShellProfile {
    root: ShellProfileRoot::Home,
    path: ".profile",
    env_file: "env",
    kind: ShellProfileKind::Main,
}];

pub struct ShellProfile {
    pub root: ShellProfileRoot,
    pub path: &'static str,
    pub env_file: &'static str,
    pub kind: ShellProfileKind,
}

#[derive(Clone, Copy)]
pub enum ShellProfileRoot {
    #[cfg_attr(windows, allow(dead_code))]
    Home,
    #[cfg_attr(windows, allow(dead_code))]
    Zsh,
    #[cfg_attr(windows, allow(dead_code))]
    Fish,
    NushellConfig,
    NushellData,
}

#[derive(Clone, Copy)]
pub enum ShellProfileKind {
    Main,
    Snippet,
}

/// Abbreviate a path for display: replace `$HOME` prefix with `~`.
pub(crate) fn abbreviate_home_path(path: &AbsolutePath, user_home: &AbsolutePath) -> Str {
    match path.strip_prefix(user_home) {
        Ok(Some(suffix)) => vite_str::format!("~/{suffix}"),
        _ => Str::from(path.to_string()),
    }
}

pub(crate) fn resolve_profile_path(
    profile: &ShellProfile,
    user_home: &AbsolutePathBuf,
) -> AbsolutePathBuf {
    let base_dirs = BaseDirs::new();
    let root = match profile.root {
        ShellProfileRoot::Home => user_home.clone(),
        ShellProfileRoot::Zsh => std::env::var_os("ZDOTDIR")
            .and_then(|value| AbsolutePathBuf::new(value.into()))
            .unwrap_or_else(|| user_home.clone()),
        ShellProfileRoot::Fish => std::env::var_os("XDG_CONFIG_HOME")
            .and_then(|value| AbsolutePathBuf::new(value.into()))
            .unwrap_or_else(|| user_home.join(".config")),
        ShellProfileRoot::NushellConfig => std::env::var_os("XDG_CONFIG_HOME")
            .and_then(|value| AbsolutePathBuf::new(value.into()))
            .or_else(|| AbsolutePathBuf::new(base_dirs?.config_dir().into()))
            .unwrap_or_else(|| user_home.join(".config")),
        ShellProfileRoot::NushellData => std::env::var_os("XDG_DATA_HOME")
            .and_then(|value| AbsolutePathBuf::new(value.into()))
            .or_else(|| AbsolutePathBuf::new(base_dirs?.data_dir().into()))
            .unwrap_or_else(|| user_home.join(".local/share")),
    };
    root.join(profile.path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_from_str() {
        // POSIX shells
        assert_eq!(Shell::from_str("sh"), Ok(Shell::Posix));
        assert_eq!(Shell::from_str("bash"), Ok(Shell::Posix));
        assert_eq!(Shell::from_str("zsh"), Ok(Shell::Posix));

        // Other shells
        assert_eq!(Shell::from_str("fish"), Ok(Shell::Fish));
        assert_eq!(Shell::from_str("nu"), Ok(Shell::NuShell));
        assert_eq!(Shell::from_str("nushell"), Ok(Shell::NuShell));
        assert_eq!(Shell::from_str("powershell"), Ok(Shell::PowerShell));
        assert_eq!(Shell::from_str("pwsh"), Ok(Shell::PowerShell));
        assert_eq!(Shell::from_str("cmd"), Ok(Shell::Cmd));

        // Case insensitive
        assert_eq!(Shell::from_str("SH"), Ok(Shell::Posix));
        assert_eq!(Shell::from_str("BASH"), Ok(Shell::Posix));
        assert_eq!(Shell::from_str("Fish"), Ok(Shell::Fish));
        assert_eq!(Shell::from_str("POWERSHELL"), Ok(Shell::PowerShell));
        assert_eq!(Shell::from_str("Nu"), Ok(Shell::NuShell));

        // Invalid
        assert!(Shell::from_str("posix").is_err());
        assert!(Shell::from_str("invalid").is_err());
        assert!(Shell::from_str("").is_err());
    }

    #[test]
    fn test_detect_shell_vp_shell_explicit() {
        let _guard = vite_shared::EnvConfig::test_guard(vite_shared::EnvConfig {
            vp_shell: Some("nu".into()),
            ..vite_shared::EnvConfig::for_test()
        });
        let shell = detect_shell();
        assert_eq!(shell, Shell::NuShell);
    }

    #[test]
    fn test_detect_shell_vp_shell_case_insensitive() {
        let _guard = vite_shared::EnvConfig::test_guard(vite_shared::EnvConfig {
            vp_shell: Some("POWERSHELL".into()),
            ..vite_shared::EnvConfig::for_test()
        });
        let shell = detect_shell();
        assert_eq!(shell, Shell::PowerShell);
    }

    #[test]
    fn test_detect_shell_vp_shell_pwsh_alias() {
        let _guard = vite_shared::EnvConfig::test_guard(vite_shared::EnvConfig {
            vp_shell: Some("pwsh".into()),
            ..vite_shared::EnvConfig::for_test()
        });
        let shell = detect_shell();
        assert_eq!(shell, Shell::PowerShell);
    }

    #[test]
    fn test_detect_shell_vp_shell_fish() {
        let _guard = vite_shared::EnvConfig::test_guard(vite_shared::EnvConfig {
            vp_shell: Some("fish".into()),
            ..vite_shared::EnvConfig::for_test()
        });
        let shell = detect_shell();
        assert_eq!(shell, Shell::Fish);
    }

    #[test]
    fn test_detect_shell_defaults_without_vp_shell() {
        let _guard = vite_shared::EnvConfig::test_guard(vite_shared::EnvConfig {
            vp_shell: None,
            ..vite_shared::EnvConfig::for_test()
        });
        let shell = detect_shell();
        if cfg!(windows) {
            assert_eq!(shell, Shell::Cmd);
        } else {
            assert_eq!(shell, Shell::Posix);
        }
    }

    #[test]
    fn test_detect_shell_invalid_vp_shell_falls_back_to_default() {
        let _guard = vite_shared::EnvConfig::test_guard(vite_shared::EnvConfig {
            vp_shell: Some("invalid".into()),
            ..vite_shared::EnvConfig::for_test()
        });
        let shell = detect_shell();
        if cfg!(windows) {
            assert_eq!(shell, Shell::Cmd);
        } else {
            assert_eq!(shell, Shell::Posix);
        }
    }
}
