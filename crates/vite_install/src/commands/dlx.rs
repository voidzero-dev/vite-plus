use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;
use vite_shared::output;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env,
};

/// Options for the dlx command.
#[derive(Debug, Default)]
pub struct DlxCommandOptions<'a> {
    /// Additional packages to install before running the command
    pub packages: &'a [String],
    /// The package to execute (first positional arg)
    pub package_spec: &'a str,
    /// Arguments to pass to the executed command
    pub args: &'a [String],
    /// Execute in shell mode
    pub shell_mode: bool,
    /// Suppress output
    pub silent: bool,
}

impl PackageManager {
    /// Run the dlx command with the package manager.
    pub async fn run_dlx_command(
        &self,
        options: &DlxCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_dlx_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the dlx command for the detected package manager.
    #[must_use]
    pub fn resolve_dlx_command(&self, options: &DlxCommandOptions) -> ResolveCommandResult {
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);

        match self.client {
            PackageManagerType::Pnpm => self.resolve_pnpm_dlx(options, envs),
            PackageManagerType::Npm => self.resolve_npm_dlx(options, envs),
            PackageManagerType::Yarn => {
                if self.version.starts_with("1.") {
                    // Yarn 1.x doesn't have dlx, fall back to npx
                    self.resolve_npx_fallback(options, envs)
                } else {
                    self.resolve_yarn_dlx(options, envs)
                }
            }
        }
    }

    fn resolve_pnpm_dlx(
        &self,
        options: &DlxCommandOptions,
        envs: HashMap<String, String>,
    ) -> ResolveCommandResult {
        let mut args = Vec::new();

        // Add --package flags before dlx
        for pkg in options.packages {
            args.push("--package".into());
            args.push(pkg.clone());
        }

        args.push("dlx".into());

        // Add shell mode flag
        if options.shell_mode {
            args.push("-c".into());
        }

        // Add silent flag
        if options.silent {
            args.push("--silent".into());
        }

        // Add package spec
        args.push(options.package_spec.into());

        // Add command arguments
        args.extend(options.args.iter().cloned());

        ResolveCommandResult { bin_path: "pnpm".into(), args, envs }
    }

    fn resolve_npm_dlx(
        &self,
        options: &DlxCommandOptions,
        envs: HashMap<String, String>,
    ) -> ResolveCommandResult {
        let mut args = vec!["exec".into()];

        // Add package flags for additional packages
        for pkg in options.packages {
            args.push(format!("--package={}", pkg));
        }

        // When using additional packages or version specifiers, npm exec requires explicit
        // --package flags. For example, `npm exec typescript@5.5.4 -- tsc` doesn't work;
        // we need `npm exec --package=typescript@5.5.4 -- typescript`.
        // Shell mode uses the package_spec as the shell command, so skip this in that case.
        if !options.shell_mode
            && (!options.packages.is_empty() || options.package_spec.contains('@'))
        {
            args.push(format!("--package={}", options.package_spec));
        }

        // Always add --yes to auto-confirm prompts (align with pnpm behavior)
        args.push("--yes".into());

        // Add silent flag
        if options.silent {
            args.push("--loglevel".into());
            args.push("silent".into());
        }

        if options.shell_mode {
            args.push("-c".into());
            args.push(build_shell_command(options.package_spec, options.args));
        } else {
            // Add separator and command
            args.push("--".into());

            // When --package flag was added above (for version specifiers or additional packages),
            // we need to extract just the command name without the version suffix.
            // e.g., "typescript@5.5.4" → command is "typescript" (version is in --package flag)
            // Otherwise, use package_spec directly as the command.
            let command = if options.packages.is_empty() && !options.package_spec.contains('@') {
                options.package_spec.to_string()
            } else {
                extract_command_from_spec(options.package_spec)
            };
            args.push(command);

            // Add command arguments
            args.extend(options.args.iter().cloned());
        }

        ResolveCommandResult { bin_path: "npm".into(), args, envs }
    }

    fn resolve_yarn_dlx(
        &self,
        options: &DlxCommandOptions,
        envs: HashMap<String, String>,
    ) -> ResolveCommandResult {
        let mut args = vec!["dlx".into()];

        // Add package flags
        for pkg in options.packages {
            args.push("-p".into());
            args.push(pkg.clone());
        }

        // Add quiet flag for silent mode
        if options.silent {
            args.push("--quiet".into());
        }

        // Warn about unsupported shell mode
        if options.shell_mode {
            output::warn("yarn dlx does not support shell mode (-c)");
        }

        // Add package spec
        args.push(options.package_spec.into());

        // Add command arguments
        args.extend(options.args.iter().cloned());

        ResolveCommandResult { bin_path: "yarn".into(), args, envs }
    }

    fn resolve_npx_fallback(
        &self,
        options: &DlxCommandOptions,
        envs: HashMap<String, String>,
    ) -> ResolveCommandResult {
        output::note("yarn@1 does not have dlx command, falling back to npx");

        let mut args = Vec::new();

        // Add package flags
        for pkg in options.packages {
            args.push("--package".into());
            args.push(pkg.clone());
        }

        // Always add --yes to auto-confirm prompts (align with pnpm behavior)
        args.push("--yes".into());

        // Add quiet flag for silent mode
        if options.silent {
            args.push("--quiet".into());
        }

        if options.shell_mode {
            args.push("-c".into());
            args.push(build_shell_command(options.package_spec, options.args));
        } else {
            // Add package spec
            args.push(options.package_spec.into());

            // Add command arguments
            args.extend(options.args.iter().cloned());
        }

        ResolveCommandResult { bin_path: "npx".into(), args, envs }
    }
}

fn build_shell_command(package_spec: &str, args: &[String]) -> String {
    if args.is_empty() {
        package_spec.to_string()
    } else {
        let mut command = String::from(package_spec);
        for arg in args {
            command.push(' ');
            command.push_str(arg);
        }
        command
    }
}

/// Extract command name from package spec
/// e.g., "create-vue@3.10.0" -> "create-vue"
fn extract_command_from_spec(spec: &str) -> String {
    // Handle scoped packages: @scope/pkg@version -> pkg
    if spec.starts_with('@') {
        // Find the slash that separates scope from package name
        if let Some(slash_pos) = spec.find('/') {
            let after_slash = &spec[slash_pos + 1..];
            // Find the version separator (@ after the package name)
            if let Some(at_pos) = after_slash.find('@') {
                return after_slash[..at_pos].to_string();
            }
            return after_slash.to_string();
        }
    }

    // Non-scoped: pkg@version -> pkg
    if let Some(at_pos) = spec.find('@') {
        return spec[..at_pos].to_string();
    }

    spec.to_string()
}

#[cfg(test)]
mod tests {
    use tempfile::{TempDir, tempdir};
    use vite_path::AbsolutePathBuf;
    use vite_str::Str;

    use super::*;

    fn create_temp_dir() -> TempDir {
        tempdir().expect("Failed to create temp directory")
    }

    fn create_mock_package_manager(pm_type: PackageManagerType, version: &str) -> PackageManager {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let install_dir = temp_dir_path.join("install");

        PackageManager {
            client: pm_type,
            package_name: pm_type.to_string().into(),
            version: Str::from(version),
            hash: None,
            bin_name: pm_type.to_string().into(),
            workspace_root: temp_dir_path.clone(),
            is_monorepo: false,
            install_dir,
        }
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
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let options = DlxCommandOptions {
            packages: &[],
            package_spec: "create-vue",
            args: &["my-app".into()],
            shell_mode: false,
            silent: false,
        };
        let result = pm.resolve_dlx_command(&options);
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["dlx", "create-vue", "my-app"]);
    }

    #[test]
    fn test_pnpm_dlx_with_version() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let options = DlxCommandOptions {
            packages: &[],
            package_spec: "typescript@5.5.4",
            args: &["tsc".into(), "--version".into()],
            shell_mode: false,
            silent: false,
        };
        let result = pm.resolve_dlx_command(&options);
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["dlx", "typescript@5.5.4", "tsc", "--version"]);
    }

    #[test]
    fn test_pnpm_dlx_with_packages() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let options = DlxCommandOptions {
            packages: &["yo".into(), "generator-webapp".into()],
            package_spec: "yo",
            args: &["webapp".into()],
            shell_mode: false,
            silent: false,
        };
        let result = pm.resolve_dlx_command(&options);
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(
            result.args,
            vec!["--package", "yo", "--package", "generator-webapp", "dlx", "yo", "webapp"]
        );
    }

    #[test]
    fn test_pnpm_dlx_with_shell_mode() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let options = DlxCommandOptions {
            packages: &["cowsay".into()],
            package_spec: "echo hello | cowsay",
            args: &[],
            shell_mode: true,
            silent: false,
        };
        let result = pm.resolve_dlx_command(&options);
        assert_eq!(result.bin_path, "pnpm");
        assert!(result.args.contains(&"-c".to_string()));
        assert!(result.args.contains(&"--package".to_string()));
    }

    #[test]
    fn test_pnpm_dlx_with_silent() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let options = DlxCommandOptions {
            packages: &[],
            package_spec: "create-vue",
            args: &["my-app".into()],
            shell_mode: false,
            silent: true,
        };
        let result = pm.resolve_dlx_command(&options);
        assert_eq!(result.bin_path, "pnpm");
        assert!(result.args.contains(&"--silent".to_string()));
    }

    #[test]
    fn test_npm_exec_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let options = DlxCommandOptions {
            packages: &[],
            package_spec: "create-vue",
            args: &["my-app".into()],
            shell_mode: false,
            silent: false,
        };
        let result = pm.resolve_dlx_command(&options);
        assert_eq!(result.bin_path, "npm");
        // --yes is always added to auto-confirm prompts
        assert_eq!(result.args, vec!["exec", "--yes", "--", "create-vue", "my-app"]);
    }

    #[test]
    fn test_npm_exec_with_version() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let options = DlxCommandOptions {
            packages: &[],
            package_spec: "typescript@5.5.4",
            args: &["tsc".into(), "--version".into()],
            shell_mode: false,
            silent: false,
        };
        let result = pm.resolve_dlx_command(&options);
        assert_eq!(result.bin_path, "npm");
        // --yes is always added to auto-confirm prompts
        assert_eq!(
            result.args,
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
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let options = DlxCommandOptions {
            packages: &["yo".into(), "generator-webapp".into()],
            package_spec: "yo",
            args: &["webapp".into()],
            shell_mode: false,
            silent: false,
        };
        let result = pm.resolve_dlx_command(&options);
        assert_eq!(result.bin_path, "npm");
        // --yes is always added to auto-confirm prompts
        assert_eq!(
            result.args,
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
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let options = DlxCommandOptions {
            packages: &[],
            package_spec: "create-vue",
            args: &["my-app".into()],
            shell_mode: false,
            silent: true,
        };
        let result = pm.resolve_dlx_command(&options);
        assert_eq!(result.bin_path, "npm");
        assert!(result.args.contains(&"--loglevel".to_string()));
        assert!(result.args.contains(&"silent".to_string()));
        // --yes is always added to auto-confirm prompts
        assert!(result.args.contains(&"--yes".to_string()));
    }

    #[test]
    fn test_npm_exec_shell_mode_places_command_after_flag() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let options = DlxCommandOptions {
            packages: &["cowsay".into(), "lolcatjs".into()],
            package_spec: "echo hello | cowsay | lolcatjs",
            args: &[],
            shell_mode: true,
            silent: false,
        };
        let result = pm.resolve_dlx_command(&options);
        assert_eq!(
            result.args,
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
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let options = DlxCommandOptions {
            packages: &[],
            package_spec: "echo",
            args: &["hello world".into()],
            shell_mode: true,
            silent: true,
        };
        let result = pm.resolve_dlx_command(&options);
        assert_eq!(
            result.args,
            vec!["exec", "--yes", "--loglevel", "silent", "-c", "echo hello world"]
        );
    }

    #[test]
    fn test_npm_exec_scoped_package_with_version() {
        // Scoped packages with version need --package flag and extracted command name
        // e.g., "@vue/cli@5.0.0" -> --package=@vue/cli@5.0.0 and command "cli"
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let options = DlxCommandOptions {
            packages: &[],
            package_spec: "@vue/cli@5.0.0",
            args: &["create".into(), "my-app".into()],
            shell_mode: false,
            silent: false,
        };
        let result = pm.resolve_dlx_command(&options);
        assert_eq!(result.bin_path, "npm");
        assert_eq!(
            result.args,
            vec!["exec", "--package=@vue/cli@5.0.0", "--yes", "--", "cli", "create", "my-app"]
        );
    }

    #[test]
    fn test_npm_exec_scoped_package_without_version() {
        // Scoped packages contain '@' in their name, so the current logic treats them
        // the same as versioned packages (adds --package flag and extracts command name).
        // e.g., "@vue/cli" -> --package=@vue/cli and command "cli"
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let options = DlxCommandOptions {
            packages: &[],
            package_spec: "@vue/cli",
            args: &["create".into(), "my-app".into()],
            shell_mode: false,
            silent: false,
        };
        let result = pm.resolve_dlx_command(&options);
        assert_eq!(result.bin_path, "npm");
        assert_eq!(
            result.args,
            vec!["exec", "--package=@vue/cli", "--yes", "--", "cli", "create", "my-app"]
        );
    }

    #[test]
    fn test_npm_exec_version_requires_package_flag_and_extracted_command() {
        // This test documents the key behavior: when package_spec contains '@' for version,
        // npm exec needs BOTH:
        // 1. --package=<full-spec> to specify the version
        // 2. The command name (without version) after -- separator
        // Without this, `npm exec create-vue@3.10.0` would fail to find the command
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let options = DlxCommandOptions {
            packages: &[],
            package_spec: "create-vue@3.10.0",
            args: &["my-app".into()],
            shell_mode: false,
            silent: false,
        };
        let result = pm.resolve_dlx_command(&options);

        // Verify --package flag contains full spec with version
        assert!(result.args.contains(&"--package=create-vue@3.10.0".to_string()));

        // Verify command after -- is just the name without version
        let separator_pos = result.args.iter().position(|a| a == "--").unwrap();
        assert_eq!(result.args[separator_pos + 1], "create-vue");
    }

    #[test]
    fn test_yarn_v1_fallback_to_npx() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.19");
        let options = DlxCommandOptions {
            packages: &[],
            package_spec: "create-vue",
            args: &["my-app".into()],
            shell_mode: false,
            silent: false,
        };
        let result = pm.resolve_dlx_command(&options);
        assert_eq!(result.bin_path, "npx");
        // --yes is always added to auto-confirm prompts
        assert_eq!(result.args, vec!["--yes", "create-vue", "my-app"]);
    }

    #[test]
    fn test_yarn_v1_fallback_with_packages() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.19");
        let options = DlxCommandOptions {
            packages: &["yo".into()],
            package_spec: "yo",
            args: &["webapp".into()],
            shell_mode: false,
            silent: false,
        };
        let result = pm.resolve_dlx_command(&options);
        assert_eq!(result.bin_path, "npx");
        // --yes is always added to auto-confirm prompts
        assert_eq!(result.args, vec!["--package", "yo", "--yes", "yo", "webapp"]);
    }

    #[test]
    fn test_yarn_v1_fallback_shell_mode_places_command_after_flag() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.19");
        let options = DlxCommandOptions {
            packages: &["cowsay".into()],
            package_spec: "echo hello | cowsay",
            args: &[],
            shell_mode: true,
            silent: true,
        };
        let result = pm.resolve_dlx_command(&options);
        assert_eq!(
            result.args,
            vec!["--package", "cowsay", "--yes", "--quiet", "-c", "echo hello | cowsay"]
        );
    }

    #[test]
    fn test_yarn_v2_dlx_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let options = DlxCommandOptions {
            packages: &[],
            package_spec: "create-vue",
            args: &["my-app".into()],
            shell_mode: false,
            silent: false,
        };
        let result = pm.resolve_dlx_command(&options);
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["dlx", "create-vue", "my-app"]);
    }

    #[test]
    fn test_yarn_v2_dlx_with_packages() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let options = DlxCommandOptions {
            packages: &["yo".into(), "generator-webapp".into()],
            package_spec: "yo",
            args: &["webapp".into()],
            shell_mode: false,
            silent: false,
        };
        let result = pm.resolve_dlx_command(&options);
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["dlx", "-p", "yo", "-p", "generator-webapp", "yo", "webapp"]);
    }

    #[test]
    fn test_yarn_v2_dlx_with_quiet() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let options = DlxCommandOptions {
            packages: &[],
            package_spec: "create-vue",
            args: &["my-app".into()],
            shell_mode: false,
            silent: true,
        };
        let result = pm.resolve_dlx_command(&options);
        assert_eq!(result.bin_path, "yarn");
        assert!(result.args.contains(&"--quiet".to_string()));
    }

    #[test]
    fn test_yarn_v3_dlx() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "3.6.0");
        let options = DlxCommandOptions {
            packages: &[],
            package_spec: "create-vue",
            args: &["my-app".into()],
            shell_mode: false,
            silent: false,
        };
        let result = pm.resolve_dlx_command(&options);
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["dlx", "create-vue", "my-app"]);
    }

    #[test]
    fn test_yarn_v2_dlx_with_version() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let options = DlxCommandOptions {
            packages: &[],
            package_spec: "typescript@5.5.4",
            args: &["tsc".into(), "--version".into()],
            shell_mode: false,
            silent: false,
        };
        let result = pm.resolve_dlx_command(&options);
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["dlx", "typescript@5.5.4", "tsc", "--version"]);
    }
}
