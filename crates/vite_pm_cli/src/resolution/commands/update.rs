use vite_pm_cli_macros::pm_args;

use super::parse_positive_usize;
use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct UpdateArgs {
    /// Update to latest version (ignore semver range)
    #[arg(short = 'L', long, not_supported(npm))]
    pub(crate) latest: bool,

    /// Update global packages
    #[arg(short = 'g', long)]
    pub(crate) global: bool,

    /// Number of global package updates to run in parallel (only with -g)
    #[arg(long, requires = "global", value_parser = parse_positive_usize)]
    pub(crate) concurrency: Option<usize>,

    /// Reinstall up-to-date global packages installed with a different Node.js version
    #[arg(long, requires = "global")]
    pub(crate) reinstall_node_mismatch: bool,

    /// Skip up-to-date global packages installed with a different Node.js version
    #[arg(long, requires = "global")]
    pub(crate) ignore_node_mismatch: bool,

    /// Update recursively in all workspace packages
    #[arg(short = 'r', long)]
    pub(crate) recursive: bool,

    /// Filter packages in monorepo (can be used multiple times)
    #[arg(long, value_name = "PATTERN")]
    pub(crate) filter: Vec<String>,

    /// Include workspace root
    #[arg(short = 'w', long)]
    pub(crate) workspace_root: bool,

    /// Update only devDependencies
    #[arg(short = 'D', long)]
    pub(crate) dev: bool,

    /// Update only dependencies (production)
    #[arg(short = 'P', long)]
    pub(crate) prod: bool,

    /// Interactive mode
    #[arg(short = 'i', long, not_supported(npm))]
    pub(crate) interactive: bool,

    /// Don't update optionalDependencies
    #[arg(long)]
    pub(crate) no_optional: bool,

    /// Update lockfile only, don't modify package.json
    #[arg(long)]
    pub(crate) no_save: bool,

    /// Only update if package exists in workspace (pnpm-specific)
    #[arg(long)]
    pub(crate) workspace: bool,

    /// Packages to update (optional - updates all if omitted)
    pub(crate) packages: Vec<String>,

    /// Additional arguments to pass through to the package manager
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) pass_through_args: Vec<String>,
}

impl Resolve<UpdateArgs> for Pnpm {
    fn resolve(&self, args: &UpdateArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("pnpm");
        cmd.repeated("--filter", args.filter.iter())
            .arg("update")
            .arg_if("--latest", args.latest)
            .arg_if("--workspace-root", args.workspace_root)
            .arg_if("--recursive", args.recursive)
            .arg_if("--dev", args.dev)
            .arg_if("--prod", args.prod)
            .arg_if("--interactive", args.interactive)
            .arg_if("--no-optional", args.no_optional)
            .arg_if("--no-save", args.no_save)
            .arg_if("--workspace", args.workspace)
            .extend(args.pass_through_args.iter())
            .extend(args.packages.iter());
        cmd.into()
    }
}

impl Resolve<UpdateArgs> for Npm {
    fn resolve(&self, args: &UpdateArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("npm");
        cmd.arg("update").repeated("--workspace", args.filter.iter());
        if args.workspace_root || args.recursive {
            cmd.arg("--include-workspace-root");
        }
        cmd.arg_if("--workspaces", args.recursive)
            .arg_if("--include=dev", args.dev)
            .arg_if("--include=prod", args.prod)
            .arg_if("--no-optional", args.no_optional)
            .arg_if("--no-save", args.no_save)
            .extend(args.pass_through_args.iter())
            .extend(args.packages.iter());
        cmd.into()
    }
}

impl Resolve<UpdateArgs> for Yarn {
    fn resolve(&self, args: &UpdateArgs, _diag: &mut Diagnostics) -> CommandResolution {
        if self.is_berry() {
            Yarn::resolve_berry_update(args)
        } else {
            Yarn::resolve_v1_update(args)
        }
    }
}

impl Yarn {
    fn resolve_berry_update(args: &UpdateArgs) -> CommandResolution {
        let mut cmd = CommandBuilder::new("yarn");
        if !args.filter.is_empty() {
            cmd.arg("workspaces").arg("foreach").arg("--all");
            cmd.repeated("--include", args.filter.iter());
        }
        cmd.arg("up")
            .arg_if("--recursive", args.recursive)
            .arg_if("--interactive", args.interactive)
            .extend(args.pass_through_args.iter())
            .extend(args.packages.iter());
        cmd.into()
    }

    fn resolve_v1_update(args: &UpdateArgs) -> CommandResolution {
        let mut cmd = CommandBuilder::new("yarn");
        if let Some(filter) = args.filter.first() {
            cmd.arg("workspace").arg(filter);
        }
        cmd.arg("upgrade")
            .arg_if("--latest", args.latest)
            .extend(args.pass_through_args.iter())
            .extend(args.packages.iter());
        cmd.into()
    }
}

impl Resolve<UpdateArgs> for Bun {
    fn resolve(&self, args: &UpdateArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("bun");
        cmd.arg("update")
            .arg_if("--latest", args.latest)
            .arg_if("--interactive", args.interactive)
            .arg_if("--production", args.prod);
        if args.no_optional {
            cmd.arg("--omit").arg("optional");
        }
        cmd.arg_if("--no-save", args.no_save)
            .arg_if("--recursive", args.recursive)
            .extend(args.pass_through_args.iter())
            .extend(args.packages.iter());
        cmd.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        CommandResolution, resolve,
        test_utils::{bun, npm, parse_args, pnpm, yarn},
    };

    fn update_args(packages: &[&str]) -> UpdateArgs {
        UpdateArgs {
            packages: packages.iter().map(ToString::to_string).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn concurrency_requires_global() {
        let error = parse_args::<UpdateArgs>(["--concurrency", "2", "typescript"]).unwrap_err();

        assert_eq!(error.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn concurrency_rejects_zero() {
        let error = parse_args::<UpdateArgs>(["-g", "--concurrency", "0"]).unwrap_err();

        assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
    }

    #[test]
    fn global_update_args_parse() {
        let args = parse_args::<UpdateArgs>([
            "-g",
            "--concurrency",
            "2",
            "--reinstall-node-mismatch",
            "typescript",
        ])
        .unwrap();

        assert!(args.global);
        assert_eq!(args.concurrency, Some(2));
        assert!(args.reinstall_node_mismatch);
        assert_eq!(args.packages, vec!["typescript"]);
    }

    #[test]
    fn test_pnpm_basic_update() {
        let resolution = resolve(&pnpm("10.0.0"), update_args(&["react"]));
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["update", "react"]);
    }

    #[test]
    fn test_pnpm_update_latest() {
        let mut options = update_args(&["react"]);
        options.latest = true;
        let resolution = resolve(&pnpm("10.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["update", "--latest", "react"]);
    }

    #[test]
    fn test_pnpm_update_all() {
        let resolution = resolve(&pnpm("10.0.0"), UpdateArgs::default());
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["update"]);
    }

    #[test]
    fn test_pnpm_update_with_filter() {
        let mut options = update_args(&["react"]);
        options.filter = vec!["app".to_string()];
        let resolution = resolve(&pnpm("10.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["--filter", "app", "update", "react"]);
    }

    #[test]
    fn test_pnpm_update_recursive() {
        let options = UpdateArgs { recursive: true, ..Default::default() };
        let resolution = resolve(&pnpm("10.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["update", "--recursive"]);
    }

    #[test]
    fn test_pnpm_update_interactive() {
        let options = UpdateArgs { interactive: true, ..Default::default() };
        let resolution = resolve(&pnpm("10.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["update", "--interactive"]);
    }

    #[test]
    fn test_pnpm_update_dev_only() {
        let options = UpdateArgs { dev: true, ..Default::default() };
        let resolution = resolve(&pnpm("10.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["update", "--dev"]);
    }

    #[test]
    fn test_pnpm_update_no_optional() {
        let options = UpdateArgs { no_optional: true, ..Default::default() };
        let resolution = resolve(&pnpm("10.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["update", "--no-optional"]);
    }

    #[test]
    fn test_pnpm_update_no_save() {
        let mut options = update_args(&["react"]);
        options.no_save = true;
        let resolution = resolve(&pnpm("10.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["update", "--no-save", "react"]);
    }

    #[test]
    fn test_pnpm_update_workspace_only() {
        let mut options = update_args(&["@myorg/utils"]);
        options.workspace = true;
        options.filter = vec!["app".to_string()];
        let resolution = resolve(&pnpm("10.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["--filter", "app", "update", "--workspace", "@myorg/utils"]);
    }

    #[test]
    fn test_yarn_v1_basic_update() {
        let resolution = resolve(&yarn("1.22.0"), update_args(&["react"]));
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["upgrade", "react"]);
    }

    #[test]
    fn test_yarn_v1_update_latest() {
        let mut options = update_args(&["react"]);
        options.latest = true;
        let resolution = resolve(&yarn("1.22.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["upgrade", "--latest", "react"]);
    }

    #[test]
    fn test_yarn_v1_update_with_workspace() {
        let mut options = update_args(&["react"]);
        options.filter = vec!["app".to_string()];
        let resolution = resolve(&yarn("1.22.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["workspace", "app", "upgrade", "react"]);
    }

    #[test]
    fn test_yarn_v4_basic_update() {
        let resolution = resolve(&yarn("4.0.0"), update_args(&["react"]));
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["up", "react"]);
    }

    #[test]
    fn test_yarn_v4_update_interactive() {
        let options = UpdateArgs { interactive: true, ..Default::default() };
        let resolution = resolve(&yarn("4.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["up", "--interactive"]);
    }

    #[test]
    fn test_yarn_v4_update_with_filter() {
        let mut options = update_args(&["react"]);
        options.filter = vec!["app".to_string()];
        let resolution = resolve(&yarn("4.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(
            command.args,
            vec!["workspaces", "foreach", "--all", "--include", "app", "up", "react"]
        );
    }

    #[test]
    fn test_yarn_v4_update_recursive() {
        let options = UpdateArgs { recursive: true, ..Default::default() };
        let resolution = resolve(&yarn("4.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["up", "--recursive"]);
    }

    #[test]
    fn test_npm_basic_update() {
        let resolution = resolve(&npm("11.0.0"), update_args(&["react"]));
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["update", "react"]);
    }

    #[test]
    fn test_npm_update_all() {
        let resolution = resolve(&npm("11.0.0"), UpdateArgs::default());
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["update"]);
    }

    #[test]
    fn test_npm_update_with_workspace() {
        let mut options = update_args(&["react"]);
        options.filter = vec!["app".to_string()];
        let resolution = resolve(&npm("11.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["update", "--workspace", "app", "react"]);
    }

    #[test]
    fn test_npm_update_recursive() {
        let options = UpdateArgs { recursive: true, ..Default::default() };
        let resolution = resolve(&npm("11.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["update", "--include-workspace-root", "--workspaces"]);
    }

    #[test]
    fn test_npm_update_dev_only() {
        let options = UpdateArgs { dev: true, ..Default::default() };
        let resolution = resolve(&npm("11.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["update", "--include=dev"]);
    }

    #[test]
    fn test_npm_update_no_optional() {
        let options = UpdateArgs { no_optional: true, ..Default::default() };
        let resolution = resolve(&npm("11.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["update", "--no-optional"]);
    }

    #[test]
    fn test_npm_update_no_save() {
        let mut options = update_args(&["react"]);
        options.no_save = true;
        let resolution = resolve(&npm("11.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["update", "--no-save", "react"]);
    }

    #[test]
    fn test_npm_latest_and_interactive_warn_without_args() {
        let mut options = update_args(&["react"]);
        options.latest = true;
        options.interactive = true;
        let resolution = resolve(&npm("11.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["update", "react"]);
        let messages =
            resolution.diagnostics.iter().map(|entry| entry.message.as_str()).collect::<Vec<_>>();
        assert_eq!(
            messages,
            vec!["npm does not support --latest.", "npm does not support --interactive."]
        );
    }

    #[test]
    fn test_pnpm_update_multiple_packages() {
        let mut options = update_args(&["react", "react-dom", "vite"]);
        options.latest = true;
        let resolution = resolve(&pnpm("10.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["update", "--latest", "react", "react-dom", "vite"]);
    }

    #[test]
    fn test_pnpm_update_complex() {
        let mut options = update_args(&["react"]);
        options.latest = true;
        options.recursive = true;
        options.filter = vec!["app".to_string(), "web".to_string()];
        options.dev = true;
        options.interactive = true;
        let resolution = resolve(&pnpm("10.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(
            command.args,
            vec![
                "--filter",
                "app",
                "--filter",
                "web",
                "update",
                "--latest",
                "--recursive",
                "--dev",
                "--interactive",
                "react"
            ]
        );
    }

    #[test]
    fn test_yarn_v4_update_multiple_filters() {
        let mut options = update_args(&["lodash"]);
        options.filter = vec!["app".to_string(), "web".to_string()];
        let resolution = resolve(&yarn("4.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(
            command.args,
            vec![
                "workspaces",
                "foreach",
                "--all",
                "--include",
                "app",
                "--include",
                "web",
                "up",
                "lodash"
            ]
        );
    }

    #[test]
    fn test_bun_basic_update() {
        let resolution = resolve(&bun("1.3.11"), UpdateArgs::default());
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["update"]);
    }

    #[test]
    fn test_bun_update_latest() {
        let options = UpdateArgs { latest: true, ..Default::default() };
        let resolution = resolve(&bun("1.3.11"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["update", "--latest"]);
    }

    #[test]
    fn test_bun_update_prod() {
        let options = UpdateArgs { prod: true, ..Default::default() };
        let resolution = resolve(&bun("1.3.11"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["update", "--production"]);
    }

    #[test]
    fn test_bun_update_no_optional() {
        let options = UpdateArgs { no_optional: true, ..Default::default() };
        let resolution = resolve(&bun("1.3.11"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["update", "--omit", "optional"]);
    }

    #[test]
    fn test_bun_update_no_save() {
        let options = UpdateArgs { no_save: true, ..Default::default() };
        let resolution = resolve(&bun("1.3.11"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["update", "--no-save"]);
    }

    #[test]
    fn test_bun_update_recursive() {
        let options = UpdateArgs { recursive: true, ..Default::default() };
        let resolution = resolve(&bun("1.3.11"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["update", "--recursive"]);
    }
}
