//! Read-only activation checks for the environment inherited by this invocation.

use std::{ffi::OsStr, path::Path};

use vp_pm_cli::PackageManagerType;
use vp_shared::{EnvConfig, env_vars, output};
use vt_path::AbsolutePath;

use super::{
    config::{self, Config, ShimMode},
    setup,
};
use crate::{
    cli::{Args, Commands, EnvSubcommands},
    commands::shell::Shell,
};

#[derive(Debug, PartialEq, Eq)]
enum Activation {
    Active,
    Inactive,
    NeedsSetup,
}

/// Compare the entrypoint's directory, not its symlink target: all Unix shims
/// can point at the same vp binary, including links outside the user's bin dir.
fn is_shim_entrypoint(actual: &Path, expected: &Path) -> bool {
    let same_name = if cfg!(windows) {
        actual
            .file_name()
            .zip(expected.file_name())
            .is_some_and(|(a, b)| a.to_string_lossy().eq_ignore_ascii_case(&b.to_string_lossy()))
    } else {
        actual.file_name() == expected.file_name()
    };
    same_name
        && actual
            .parent()
            .zip(expected.parent())
            .is_some_and(|(a, b)| same_file::is_same_file(a, b).unwrap_or(false))
}

fn check(config: &Config, bin: &AbsolutePath, cwd: &AbsolutePath, path: &OsStr) -> Activation {
    let mut inactive = false;
    for tool in crate::shim::DEFAULT_SHIM_TOOLS {
        let mode = if *tool == "node" {
            config.node_shim_mode
        } else if let Some(pm) = PackageManagerType::from_tool(tool) {
            config.package_manager_shim_mode_for(pm)
        } else {
            continue;
        };
        if mode == ShimMode::SystemFirst {
            continue;
        }
        let expected = bin.join(setup::shim_filename(tool));
        // A deleted/broken/non-executable shim cannot be repaired by sourcing env.
        if vp_command::resolve_bin(tool, Some(bin.as_path().as_os_str()), cwd)
            .ok()
            .is_none_or(|found| !is_shim_entrypoint(found.as_path(), expected.as_path()))
        {
            return Activation::NeedsSetup;
        }
        if vp_command::resolve_bin(tool, Some(path), cwd)
            .ok()
            .is_none_or(|found| !is_shim_entrypoint(found.as_path(), expected.as_path()))
        {
            inactive = true;
        }
    }
    if inactive { Activation::Inactive } else { Activation::Active }
}

fn eligible(args: &Args, raw: &[String]) -> bool {
    if args.version
        || raw.iter().any(|arg| {
            matches!(
                arg.as_str(),
                "--quiet"
                    | "-q"
                    | "--silent"
                    | "-s"
                    | "--json"
                    | "--parseable"
                    | "--help"
                    | "-h"
                    | "--format"
                    | "--reporter"
                    | "--reporters"
                    | "--logLevel"
            ) || [
                "--quiet=",
                "--silent=",
                "--json=",
                "--format=",
                "--reporter=",
                "--reporters=",
                "--logLevel=",
            ]
            .iter()
            .any(|prefix| arg.starts_with(prefix))
        })
    {
        return false;
    }
    match &args.command {
        Some(command) if command.is_quiet_or_machine_readable() => false,
        Some(
            Commands::Upgrade { background_check: true, .. }
            | Commands::Implode { .. }
            | Commands::Hooks { .. }
            | Commands::Staged { .. },
        ) => false,
        Some(Commands::Env(args)) => !matches!(
            &args.command,
            Some(
                EnvSubcommands::Setup { .. }
                    | EnvSubcommands::Doctor { .. }
                    | EnvSubcommands::Use { .. }
                    | EnvSubcommands::Print { .. }
                    | EnvSubcommands::Which { .. }
                    | EnvSubcommands::Exec { .. }
                    | EnvSubcommands::Off { .. }
            )
        ),
        _ => true,
    }
}

pub(crate) async fn remind(args: &Args, raw: &[String], cwd: &AbsolutePath) {
    if !vp_shared::is_interactive_terminal()
        || !vp_shared::is_stderr_terminal()
        || !eligible(args, raw)
    {
        return;
    }
    // vp run/exec and other delegated commands prepend selected runtime/PM
    // directories. A Node directory also contains npm/npx, so checking only
    // the recorded tool names would still mistake this child environment for
    // an inactive terminal. Sourcing env in the parent cannot fix that PATH.
    if std::env::var_os(env_vars::VP_PATH_INJECTED_TOOLS).is_some_and(|tools| !tools.is_empty()) {
        return;
    }
    let Ok(config) = config::load_config().await else { return };
    let env = EnvConfig::get();
    let path = std::env::var_os("PATH").unwrap_or_default();
    match check(&config, &env.dirs.bin, cwd, &path) {
        Activation::Active => {}
        Activation::NeedsSetup => output::raw_stderr(
            "Vite+ managed tool shims are missing or unusable. Run `vp env setup --refresh` to repair them.",
        ),
        Activation::Inactive => {
            if ["env", "env.fish", "env.nu", "env.ps1"]
                .iter()
                .any(|file| !env.dirs.config.join(file).as_path().is_file())
            {
                output::raw_stderr(
                    "Vite+ shell environment files are missing. Run `vp env setup` to recreate them.",
                );
                return;
            }
            output::raw_stderr(
                "Vite+ managed tools are not first on PATH. Activate in this terminal:",
            );
            for line in
                instructions(&env.dirs.config, env.vp_shell.as_deref().and_then(|s| s.parse().ok()))
            {
                output::raw_stderr(&line);
            }
        }
    }
}

/// Use an explicit VP_SHELL when available. Otherwise label the alternatives:
/// SHELL describes the login shell and need not describe this terminal.
pub(crate) fn instructions(env_dir: &AbsolutePath, shell: Option<Shell>) -> Vec<String> {
    let commands = [
        (Shell::Posix, "Bash/Zsh", "env"),
        (Shell::Fish, "Fish", "env.fish"),
        (Shell::NuShell, "Nushell", "env.nu"),
        (Shell::PowerShell, "PowerShell", "env.ps1"),
    ];
    if shell == Some(Shell::Cmd) {
        // cmd has no sourceable environment file. A new terminal inherits the
        // persistent PATH written by setup.
        return vec!["  In cmd.exe, open a new terminal to load the updated PATH.".into()];
    }
    commands
        .into_iter()
        .filter(|(kind, _, _)| shell.is_none_or(|s| s == *kind))
        .map(|(kind, label, file)| {
            let path = env_dir.join(file).to_string();
            let command = match kind {
                Shell::Posix => {
                    format!(". \"{}\"", setup::escape_posix_double_quoted_string(&path))
                }
                Shell::Fish => {
                    format!("source \"{}\"", setup::escape_fish_double_quoted_string(&path))
                }
                Shell::NuShell => {
                    format!("source \"{}\"", setup::escape_nu_double_quoted_string(&path))
                }
                Shell::PowerShell => {
                    format!(". '{}'", setup::escape_powershell_single_quoted_string(&path))
                }
                Shell::Cmd => unreachable!(),
            };
            if shell.is_some() { format!("  {command}") } else { format!("  {label}: {command}") }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use vt_path::AbsolutePathBuf;

    use super::*;

    fn executable(dir: &AbsolutePath, tool: &str) {
        let path = dir.join(setup::shim_filename(tool));
        std::fs::write(&path, b"fake executable").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    #[test]
    fn checks_lookup_order_and_only_managed_tools() {
        let temp = tempfile::tempdir().unwrap();
        let cwd = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let bin = cwd.join("shims");
        let system = cwd.join("system");
        std::fs::create_dir(&bin).unwrap();
        std::fs::create_dir(&system).unwrap();
        let mut config = Config::default();
        config.set_all_package_manager_shim_modes(ShimMode::SystemFirst);
        executable(&bin, "node");
        executable(&system, "node");
        let active = std::env::join_paths([bin.as_path(), system.as_path()]).unwrap();
        let shadowed = std::env::join_paths([system.as_path(), bin.as_path()]).unwrap();
        assert_eq!(check(&config, &bin, &cwd, &active), Activation::Active);
        assert_eq!(check(&config, &bin, &cwd, &shadowed), Activation::Inactive);
        assert_eq!(check(&config, &bin, &cwd, system.as_path().as_os_str()), Activation::Inactive);
        config.node_shim_mode = ShimMode::SystemFirst;
        assert_eq!(check(&config, &bin, &cwd, &shadowed), Activation::Active);
        config.set_package_manager_shim_mode(PackageManagerType::Npm, ShimMode::Managed);
        assert_eq!(check(&config, &bin, &cwd, &active), Activation::NeedsSetup);
        executable(&bin, "npm");
        executable(&bin, "npx");
        assert_eq!(check(&config, &bin, &cwd, &shadowed), Activation::Active);
        executable(&system, "npx");
        assert_eq!(check(&config, &bin, &cwd, &shadowed), Activation::Inactive);
        std::fs::remove_file(bin.join(setup::shim_filename("npm"))).unwrap();
        assert_eq!(check(&config, &bin, &cwd, &shadowed), Activation::NeedsSetup);
    }

    #[cfg(unix)]
    #[test]
    fn directory_aliases_are_active_but_other_links_to_vp_are_not() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let cwd = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let bin = cwd.join("shims");
        let external = cwd.join("brew");
        std::fs::create_dir(&bin).unwrap();
        std::fs::create_dir(&external).unwrap();
        executable(&external, "vp");
        symlink(external.join("vp"), bin.join("node")).unwrap();
        symlink(external.join("vp"), external.join("node")).unwrap();
        let alias = cwd.join("alias");
        symlink(&bin, &alias).unwrap();
        let mut config = Config::default();
        config.set_all_package_manager_shim_modes(ShimMode::SystemFirst);
        for path in [alias.as_path().as_os_str(), OsStr::new("shims/../shims")] {
            assert_eq!(check(&config, &bin, &cwd, path), Activation::Active);
        }
        assert_eq!(
            check(&config, &bin, &cwd, external.as_path().as_os_str()),
            Activation::Inactive
        );
        std::fs::remove_file(external.join("vp")).unwrap();
        assert_eq!(check(&config, &bin, &cwd, bin.as_path().as_os_str()), Activation::NeedsSetup);
    }

    #[test]
    fn excludes_output_protocols_and_quiet_commands() {
        for raw in [
            vec!["env", "current", "--json"],
            vec!["env", "list", "--json"],
            vec!["env", "print"],
            vec!["env", "which", "node"],
            vec!["env", "use", "22", "--no-install"],
            vec!["env", "exec", "node", "-v"],
            vec!["env", "doctor"],
            vec!["env", "setup"],
            vec!["env", "off"],
            vec!["upgrade", "--background-check"],
            vec!["upgrade", "--silent"],
            vec!["install", "--silent"],
            vec!["toolchain", "--json"],
            vec!["sync-versions", "--json"],
            vec!["build", "--quiet"],
            vec!["test", "--reporter=json"],
            vec!["run", "--silent", "build"],
            vec!["--version"],
        ] {
            let strings: Vec<_> = raw.iter().map(|s| (*s).to_string()).collect();
            let args = crate::try_parse_args_from(
                std::iter::once("vp".to_string()).chain(strings.clone()),
            )
            .unwrap();
            assert!(!eligible(&args, &strings), "{raw:?}");
        }
        for raw in [vec!["build"], vec!["env", "current"], vec!["env", "list"], vec!["install"]] {
            let strings: Vec<_> = raw.iter().map(|s| (*s).to_string()).collect();
            let args = crate::try_parse_args_from(
                std::iter::once("vp".to_string()).chain(strings.clone()),
            )
            .unwrap();
            assert!(eligible(&args, &strings), "{raw:?}");
        }
    }
}
