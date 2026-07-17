use vite_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, DiagnosticKind, Diagnostics, Npm, Pnpm, Resolution,
    Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct DlxArgs {
    /// Package(s) to install before running
    #[arg(long, short = 'p', value_name = "NAME")]
    pub(crate) package: Vec<String>,

    /// Execute within a shell environment
    #[arg(long = "shell-mode", short = 'c', not_supported(yarn >= "2", bun))]
    pub(crate) shell_mode: bool,

    /// Suppress all output except the executed command's output
    #[arg(long, short = 's')]
    pub(crate) silent: bool,

    /// Package to execute and arguments
    #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
    pub(crate) args: Vec<String>,
}

impl Resolve<DlxArgs> for Pnpm {
    fn resolve(&self, args: &DlxArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let Some((package_spec, command_args)) = args.split_command() else {
            return CommandResolution::Noop;
        };

        let mut cmd = CommandBuilder::new("pnpm");
        cmd.repeated("--package", args.package.iter());
        cmd.arg("dlx")
            .arg_if("-c", args.shell_mode)
            .arg_if("--silent", args.silent)
            .arg(package_spec)
            .extend(command_args);
        cmd.into()
    }
}

impl Resolve<DlxArgs> for Npm {
    fn resolve(&self, args: &DlxArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let Some((package_spec, command_args)) = args.split_command() else {
            return CommandResolution::Noop;
        };

        let mut cmd = CommandBuilder::new("npm");
        cmd.arg("exec");
        for package in &args.package {
            cmd.arg(vite_str::format!("--package={package}"));
        }
        if !args.shell_mode && (!args.package.is_empty() || package_spec.contains('@')) {
            cmd.arg(vite_str::format!("--package={package_spec}"));
        }
        cmd.arg("--yes");
        if args.silent {
            cmd.arg("--loglevel").arg("silent");
        }
        if args.shell_mode {
            cmd.arg("-c").arg(build_shell_command(package_spec, command_args));
        } else {
            let command = if args.package.is_empty() && !package_spec.contains('@') {
                package_spec.to_string()
            } else {
                extract_command_from_spec(package_spec)
            };
            cmd.arg("--").arg(command).extend(command_args);
        }
        cmd.into()
    }
}

impl Resolve<DlxArgs> for Yarn {
    fn resolve(&self, args: &DlxArgs, diag: &mut Diagnostics) -> CommandResolution {
        let Some((package_spec, command_args)) = args.split_command() else {
            return CommandResolution::Noop;
        };

        if self.is_berry() {
            let mut cmd = CommandBuilder::new("yarn");
            cmd.arg("dlx")
                .repeated("-p", args.package.iter())
                .arg_if("--quiet", args.silent)
                .arg(package_spec)
                .extend(command_args);
            return cmd.into();
        }

        diag.note(
            DiagnosticKind::FallbackCommand,
            "yarn@1 does not have dlx command, falling back to npx",
        );

        resolve_npx(args)
    }
}

impl Resolve<DlxArgs> for Bun {
    fn resolve(&self, args: &DlxArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let Some((package_spec, command_args)) = args.split_command() else {
            return CommandResolution::Noop;
        };

        let mut cmd = CommandBuilder::new("bun");
        cmd.arg("x")
            .repeated("--package", args.package.iter())
            .arg(package_spec)
            .extend(command_args);
        cmd.into()
    }
}

impl DlxArgs {
    pub(crate) fn resolve_npx_fallback(&self) -> Resolution {
        Resolution { outcome: resolve_npx(self), diagnostics: Diagnostics::default() }
    }

    fn split_command(&self) -> Option<(&str, &[String])> {
        self.args.split_first().map(|(package_spec, args)| (package_spec.as_str(), args))
    }
}

fn resolve_npx(args: &DlxArgs) -> CommandResolution {
    let Some((package_spec, command_args)) = args.split_command() else {
        return CommandResolution::Noop;
    };

    let mut cmd = CommandBuilder::new("npx");
    cmd.repeated("--package", args.package.iter());
    cmd.arg("--yes").arg_if("--quiet", args.silent);
    if args.shell_mode {
        cmd.arg("-c").arg(build_shell_command(package_spec, command_args));
    } else {
        cmd.arg(package_spec).extend(command_args);
    }
    cmd.into()
}

fn build_shell_command(package_spec: &str, args: &[String]) -> String {
    if args.is_empty() {
        return package_spec.to_string();
    }

    let mut command = package_spec.to_string();
    for arg in args {
        command.push(' ');
        command.push_str(arg);
    }
    command
}

fn extract_command_from_spec(spec: &str) -> String {
    if spec.starts_with('@')
        && let Some(slash_pos) = spec.find('/')
    {
        let after_slash = &spec[slash_pos + 1..];
        if let Some(at_pos) = after_slash.find('@') {
            return after_slash[..at_pos].to_string();
        }
        return after_slash.to_string();
    }

    if let Some(at_pos) = spec.find('@') {
        return spec[..at_pos].to_string();
    }

    spec.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        resolve,
        test_utils::{bun, npm, parse_args, pnpm, yarn},
    };

    fn dlx_args(package_spec: &str, args: &[&str]) -> DlxArgs {
        let mut command_args = vec![package_spec.to_string()];
        command_args.extend(args.iter().map(ToString::to_string));
        DlxArgs { args: command_args, ..Default::default() }
    }

    #[test]
    fn test_extract_command_from_spec() {
        assert_eq!(extract_command_from_spec("create-vue"), "create-vue");
        assert_eq!(extract_command_from_spec("create-vue@3.10.0"), "create-vue");
        assert_eq!(extract_command_from_spec("typescript@5.5.4"), "typescript");
        assert_eq!(extract_command_from_spec("@vue/cli"), "cli");
        assert_eq!(extract_command_from_spec("@vue/cli@5.0.0"), "cli");
        assert_eq!(extract_command_from_spec("@pnpm/meta-updater"), "meta-updater");
        assert_eq!(extract_command_from_spec("@pnpm/meta-updater@1.0.0"), "meta-updater");
    }

    #[test]
    fn test_pnpm_dlx_basic() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), dlx_args("create-vue", &["my-app"])).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["dlx", "create-vue", "my-app"]);
    }

    #[test]
    fn test_pnpm_dlx_with_version() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), dlx_args("typescript@5.5.4", &["tsc", "--version"])).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["dlx", "typescript@5.5.4", "tsc", "--version"]);
    }

    #[test]
    fn test_pnpm_dlx_with_packages() {
        let mut options = dlx_args("yo", &["webapp"]);
        options.package = vec!["yo".to_string(), "generator-webapp".to_string()];
        let CommandResolution::Run(command) = resolve(&pnpm("10.0.0"), options).outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(
            command.args,
            vec!["--package", "yo", "--package", "generator-webapp", "dlx", "yo", "webapp"]
        );
    }

    #[test]
    fn test_pnpm_dlx_with_shell_mode() {
        let mut options = dlx_args("echo hello | cowsay", &[]);
        options.package = vec!["cowsay".to_string()];
        options.shell_mode = true;
        let CommandResolution::Run(command) = resolve(&pnpm("10.0.0"), options).outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert!(command.args.contains(&"-c".to_string()));
        assert!(command.args.contains(&"--package".to_string()));
    }

    #[test]
    fn test_pnpm_dlx_with_silent() {
        let mut options = dlx_args("create-vue", &["my-app"]);
        options.silent = true;
        let CommandResolution::Run(command) = resolve(&pnpm("10.0.0"), options).outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert!(command.args.contains(&"--silent".to_string()));
    }

    #[test]
    fn test_npm_exec_basic() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), dlx_args("create-vue", &["my-app"])).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["exec", "--yes", "--", "create-vue", "my-app"]);
    }

    #[test]
    fn test_npm_exec_with_version() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), dlx_args("typescript@5.5.4", &["tsc", "--version"])).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(
            command.args,
            vec![
                "exec",
                "--package=typescript@5.5.4",
                "--yes",
                "--",
                "typescript",
                "tsc",
                "--version"
            ]
        );
    }

    #[test]
    fn test_npm_exec_with_packages() {
        let mut options = dlx_args("yo", &["webapp"]);
        options.package = vec!["yo".to_string(), "generator-webapp".to_string()];
        let CommandResolution::Run(command) = resolve(&npm("11.0.0"), options).outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(
            command.args,
            vec![
                "exec",
                "--package=yo",
                "--package=generator-webapp",
                "--package=yo",
                "--yes",
                "--",
                "yo",
                "webapp"
            ]
        );
    }

    #[test]
    fn test_npm_exec_with_silent() {
        let mut options = dlx_args("create-vue", &["my-app"]);
        options.silent = true;
        let CommandResolution::Run(command) = resolve(&npm("11.0.0"), options).outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert!(command.args.contains(&"--loglevel".to_string()));
        assert!(command.args.contains(&"silent".to_string()));
        assert!(command.args.contains(&"--yes".to_string()));
    }

    #[test]
    fn test_npm_exec_shell_mode_places_command_after_flag() {
        let mut options = dlx_args("echo hello | cowsay | lolcatjs", &[]);
        options.package = vec!["cowsay".to_string(), "lolcatjs".to_string()];
        options.shell_mode = true;
        let CommandResolution::Run(command) = resolve(&npm("11.0.0"), options).outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(
            command.args,
            vec![
                "exec",
                "--package=cowsay",
                "--package=lolcatjs",
                "--yes",
                "-c",
                "echo hello | cowsay | lolcatjs"
            ]
        );
    }

    #[test]
    fn test_npm_exec_shell_mode_with_additional_args() {
        let mut options = dlx_args("echo", &["hello world"]);
        options.shell_mode = true;
        options.silent = true;
        let CommandResolution::Run(command) = resolve(&npm("11.0.0"), options).outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(
            command.args,
            vec!["exec", "--yes", "--loglevel", "silent", "-c", "echo hello world"]
        );
    }

    #[test]
    fn test_npm_exec_scoped_package_with_version() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), dlx_args("@vue/cli@5.0.0", &["create", "my-app"])).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(
            command.args,
            vec!["exec", "--package=@vue/cli@5.0.0", "--yes", "--", "cli", "create", "my-app"]
        );
    }

    #[test]
    fn test_npm_exec_scoped_package_without_version() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), dlx_args("@vue/cli", &["create", "my-app"])).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(
            command.args,
            vec!["exec", "--package=@vue/cli", "--yes", "--", "cli", "create", "my-app"]
        );
    }

    #[test]
    fn test_npm_exec_version_requires_package_flag_and_extracted_command() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), dlx_args("create-vue@3.10.0", &["my-app"])).outcome
        else {
            panic!("expected command resolution");
        };

        assert!(command.args.contains(&"--package=create-vue@3.10.0".to_string()));
        let separator_pos = command.args.iter().position(|arg| arg == "--").unwrap();
        assert_eq!(command.args[separator_pos + 1], "create-vue");
    }

    #[test]
    fn test_yarn_v1_fallback_to_npx() {
        let resolution = resolve(&yarn("1.22.19"), dlx_args("create-vue", &["my-app"]));
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npx");
        assert_eq!(command.args, vec!["--yes", "create-vue", "my-app"]);
        assert_eq!(resolution.diagnostics[0].kind, DiagnosticKind::FallbackCommand);
    }

    #[test]
    fn no_project_fallback_uses_npx_without_a_diagnostic() {
        let resolution = dlx_args("create-vue", &["my-app"]).resolve_npx_fallback();
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npx");
        assert_eq!(command.args, vec!["--yes", "create-vue", "my-app"]);
        assert!(resolution.diagnostics.is_empty());
        assert!(command.env.is_empty());
    }

    #[test]
    fn test_yarn_v1_fallback_with_packages() {
        let mut options = dlx_args("yo", &["webapp"]);
        options.package = vec!["yo".to_string()];
        let CommandResolution::Run(command) = resolve(&yarn("1.22.19"), options).outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npx");
        assert_eq!(command.args, vec!["--package", "yo", "--yes", "yo", "webapp"]);
    }

    #[test]
    fn test_yarn_v1_fallback_shell_mode_places_command_after_flag() {
        let mut options = dlx_args("echo hello | cowsay", &[]);
        options.package = vec!["cowsay".to_string()];
        options.shell_mode = true;
        options.silent = true;
        let CommandResolution::Run(command) = resolve(&yarn("1.22.19"), options).outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(
            command.args,
            vec!["--package", "cowsay", "--yes", "--quiet", "-c", "echo hello | cowsay"]
        );
    }

    #[test]
    fn test_yarn_v2_dlx_basic() {
        let CommandResolution::Run(command) =
            resolve(&yarn("4.0.0"), dlx_args("create-vue", &["my-app"])).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["dlx", "create-vue", "my-app"]);
    }

    #[test]
    fn test_yarn_v2_dlx_with_packages() {
        let mut options = dlx_args("yo", &["webapp"]);
        options.package = vec!["yo".to_string(), "generator-webapp".to_string()];
        let CommandResolution::Run(command) = resolve(&yarn("4.0.0"), options).outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["dlx", "-p", "yo", "-p", "generator-webapp", "yo", "webapp"]);
    }

    #[test]
    fn test_yarn_v2_dlx_with_quiet() {
        let mut options = dlx_args("create-vue", &["my-app"]);
        options.silent = true;
        let CommandResolution::Run(command) = resolve(&yarn("4.0.0"), options).outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert!(command.args.contains(&"--quiet".to_string()));
    }

    #[test]
    fn test_yarn_v3_dlx() {
        let CommandResolution::Run(command) =
            resolve(&yarn("3.6.0"), dlx_args("create-vue", &["my-app"])).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["dlx", "create-vue", "my-app"]);
    }

    #[test]
    fn test_yarn_v2_dlx_with_version() {
        let CommandResolution::Run(command) =
            resolve(&yarn("4.0.0"), dlx_args("typescript@5.5.4", &["tsc", "--version"])).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["dlx", "typescript@5.5.4", "tsc", "--version"]);
    }

    #[test]
    fn test_yarn_v2_dlx_shell_mode_warns_and_drops_flag() {
        let mut options = dlx_args("echo", &["hello"]);
        options.shell_mode = true;
        let resolution = resolve(&yarn("4.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["dlx", "echo", "hello"]);
        assert_eq!(resolution.diagnostics.len(), 1);
        assert_eq!(resolution.diagnostics[0].kind, DiagnosticKind::UnsupportedOptionDropped);
        assert!(resolution.diagnostics[0].message.contains("--shell-mode"));
        assert!(resolution.diagnostics[0].message.contains("yarn >=2"));
    }

    #[test]
    fn test_bun_dlx_basic() {
        let CommandResolution::Run(command) =
            resolve(&bun("1.3.11"), dlx_args("create-vue", &["my-app"])).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["x", "create-vue", "my-app"]);
    }

    #[test]
    fn test_bun_dlx_with_packages() {
        let mut options = dlx_args("yo", &["webapp"]);
        options.package = vec!["yo".to_string(), "generator-webapp".to_string()];
        let CommandResolution::Run(command) = resolve(&bun("1.3.11"), options).outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(
            command.args,
            vec!["x", "--package", "yo", "--package", "generator-webapp", "yo", "webapp"]
        );
    }

    #[test]
    fn test_bun_dlx_shell_mode_warns_and_drops_flag() {
        let mut options = dlx_args("echo", &["hello"]);
        options.shell_mode = true;
        let resolution = resolve(&bun("1.3.11"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["x", "echo", "hello"]);
        assert_eq!(resolution.diagnostics.len(), 1);
        assert_eq!(resolution.diagnostics[0].kind, DiagnosticKind::UnsupportedOptionDropped);
        assert!(resolution.diagnostics[0].message.contains("--shell-mode"));
        assert!(resolution.diagnostics[0].message.contains("bun"));
    }

    #[test]
    fn parser_captures_package_flags_and_command_args() {
        let args = parse_args::<DlxArgs>([
            "-p",
            "yo",
            "-s",
            "create-vue",
            "my-app",
            "--template",
            "vue-ts",
        ])
        .unwrap();

        assert_eq!(args.package, vec!["yo"]);
        assert!(args.silent);
        assert_eq!(args.args, vec!["create-vue", "my-app", "--template", "vue-ts"]);
    }

    #[test]
    fn parser_captures_repeated_package_flags_and_shell_mode() {
        let args =
            parse_args::<DlxArgs>(["-p", "yo", "-p", "generator-webapp", "-c", "echo", "hello"])
                .unwrap();

        assert_eq!(args.package, vec!["yo", "generator-webapp"]);
        assert!(args.shell_mode);
        assert_eq!(args.args, vec!["echo", "hello"]);
    }
}
