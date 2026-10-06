use vp_pm_cli_macros::pm_args;

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
    #[arg(short = 'r', long, not_supported(yarn < "2"))]
    pub(crate) recursive: bool,

    /// Filter packages in monorepo (can be used multiple times)
    #[arg(long, value_name = "PATTERN", not_supported(bun < "1.4", yarn >= "2"))]
    pub(crate) filter: Vec<String>,

    /// Include workspace root
    #[arg(short = 'w', long, not_supported(yarn, bun))]
    pub(crate) workspace_root: bool,

    /// Update devDependencies
    #[arg(short = 'D', long, not_supported(yarn, bun < "1.4"))]
    pub(crate) dev: bool,

    /// Update dependencies (production)
    #[arg(short = 'P', long, not_supported(yarn >= "2"))]
    pub(crate) prod: bool,

    /// Interactive mode
    #[arg(short = 'i', long, not_supported(npm))]
    pub(crate) interactive: bool,

    /// Exclude optionalDependencies
    #[arg(long, not_supported(yarn >= "2"))]
    pub(crate) no_optional: bool,

    /// Update lockfile only, don't modify package.json
    #[arg(long)]
    pub(crate) no_save: bool,

    /// Only update if package exists in workspace (pnpm-specific)
    #[arg(long, not_supported(npm, yarn, bun))]
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
        let recursive_resolutions = self.is_berry() && args.recursive && !args.latest;
        if args.no_save && !recursive_resolutions {
            return CommandResolution::InvalidArgument("yarn does not support --no-save.".into());
        }
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
        cmd.arg("up")
            .arg_if("--recursive", args.recursive && !args.latest)
            .arg_if("--interactive", args.interactive)
            .extend(args.pass_through_args.iter())
            .extend(args.packages.iter());
        // Raw arguments may contain package patterns, so don't widen their selection.
        if args.recursive && args.packages.is_empty() && args.pass_through_args.is_empty() {
            // Recursive mode needs ** to match scoped names; ordinary up treats * as all.
            cmd.arg(if args.latest { "*" } else { "**" });
        }
        cmd.into()
    }

    fn resolve_v1_update(args: &UpdateArgs) -> CommandResolution {
        if args.filter.len() > 1 {
            return CommandResolution::InvalidArgument(
                "yarn < 2 does not support multiple --filter options.".into(),
            );
        }
        let mut cmd = CommandBuilder::new("yarn");
        if let Some(filter) = args.filter.first() {
            cmd.arg("workspace").arg(filter);
        }
        cmd.arg(if args.interactive { "upgrade-interactive" } else { "upgrade" })
            .arg_if("--latest", args.latest)
            .arg_if("--production=true", args.prod)
            .arg_if("--ignore-optional", args.no_optional)
            .extend(args.pass_through_args.iter())
            .extend(args.packages.iter());
        cmd.into()
    }
}

impl Resolve<UpdateArgs> for Bun {
    fn resolve(&self, args: &UpdateArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("bun");
        cmd.arg("update")
            .repeated("--filter", args.filter.iter())
            .arg_if("--latest", args.latest)
            .arg_if("--dev", args.dev)
            .arg_if("--interactive", args.interactive)
            .arg_if("--production", args.prod);
        if args.no_optional {
            if self.supports_v1_4_commands() {
                cmd.arg("--no-optional");
            } else {
                cmd.arg("--omit").arg("optional");
            }
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
        resolve,
        test_utils::{bun, expect_run, expect_unsupported, npm, parse_args, pnpm, yarn},
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
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["update", "react"]);
    }

    #[test]
    fn test_pnpm_update_latest() {
        let mut options = update_args(&["react"]);
        options.latest = true;
        let resolution = resolve(&pnpm("10.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["update", "--latest", "react"]);
    }

    #[test]
    fn test_pnpm_update_all() {
        let resolution = resolve(&pnpm("10.0.0"), UpdateArgs::default());
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["update"]);
    }

    #[test]
    fn test_pnpm_update_with_filter() {
        let mut options = update_args(&["react"]);
        options.filter = vec!["app".to_string()];
        let resolution = resolve(&pnpm("10.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["--filter", "app", "update", "react"]);
    }

    #[test]
    fn test_pnpm_update_recursive() {
        let options = UpdateArgs { recursive: true, ..Default::default() };
        let resolution = resolve(&pnpm("10.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["update", "--recursive"]);
    }

    #[test]
    fn test_pnpm_update_interactive() {
        let options = UpdateArgs { interactive: true, ..Default::default() };
        let resolution = resolve(&pnpm("10.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["update", "--interactive"]);
    }

    #[test]
    fn test_pnpm_update_dev_only() {
        let options = UpdateArgs { dev: true, ..Default::default() };
        let resolution = resolve(&pnpm("10.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["update", "--dev"]);
    }

    #[test]
    fn test_pnpm_update_no_optional() {
        let options = UpdateArgs { no_optional: true, ..Default::default() };
        let resolution = resolve(&pnpm("10.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["update", "--no-optional"]);
    }

    #[test]
    fn test_pnpm_update_no_save() {
        let mut options = update_args(&["react"]);
        options.no_save = true;
        let resolution = resolve(&pnpm("10.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["update", "--no-save", "react"]);
    }

    #[test]
    fn test_pnpm_update_workspace_only() {
        let mut options = update_args(&["@myorg/utils"]);
        options.workspace = true;
        options.filter = vec!["app".to_string()];
        let resolution = resolve(&pnpm("10.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["--filter", "app", "update", "--workspace", "@myorg/utils"]);
    }

    #[test]
    fn test_yarn_v1_basic_update() {
        let resolution = resolve(&yarn("1.22.0"), update_args(&["react"]));
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["upgrade", "react"]);
    }

    #[test]
    fn test_yarn_v1_update_latest() {
        let mut options = update_args(&["react"]);
        options.latest = true;
        let resolution = resolve(&yarn("1.22.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["upgrade", "--latest", "react"]);
    }

    #[test]
    fn test_yarn_v1_update_with_workspace() {
        let mut options = update_args(&["react"]);
        options.filter = vec!["app".to_string()];
        let resolution = resolve(&yarn("1.22.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["workspace", "app", "upgrade", "react"]);
    }

    #[test]
    fn test_update_rejects_workspace_dependency_selection_outside_pnpm() {
        let args = parse_args::<UpdateArgs>(["--workspace", "react"]).unwrap();
        for (resolution, message) in [
            (resolve(&npm("11.16.0"), args.clone()), "npm does not support --workspace."),
            (resolve(&yarn("1.22.22"), args.clone()), "yarn does not support --workspace."),
            (resolve(&yarn("4.16.0"), args.clone()), "yarn does not support --workspace."),
            (resolve(&bun("1.3.14"), args.clone()), "bun does not support --workspace."),
            (resolve(&bun("1.4.0"), args), "bun does not support --workspace."),
        ] {
            expect_unsupported(resolution, &[message]);
        }
    }

    #[test]
    fn test_yarn_classic_rejects_multiple_filters() {
        let args = parse_args::<UpdateArgs>(["--filter", "app", "--filter", "web"]).unwrap();
        expect_unsupported(
            resolve(&yarn("1.22.22"), args),
            &["yarn < 2 does not support multiple --filter options."],
        );
    }

    #[test]
    fn test_yarn_classic_update_interactive() {
        let args = parse_args::<UpdateArgs>([
            "--interactive",
            "--latest",
            "--prod",
            "--no-optional",
            "react",
        ])
        .unwrap();
        let resolution = resolve(&yarn("1.22.22"), args);
        assert!(resolution.diagnostics.is_empty());
        assert_eq!(
            expect_run(resolution.outcome).args,
            vec![
                "upgrade-interactive",
                "--latest",
                "--production=true",
                "--ignore-optional",
                "react"
            ]
        );
    }

    #[test]
    fn test_update_keeps_raw_workspace_options() {
        let args = parse_args::<UpdateArgs>([
            "react",
            "--",
            "--workspace",
            "--recursive",
            "--interactive",
            "--filter",
            "app",
            "--filter",
            "web",
        ])
        .unwrap();
        for (resolution, subcommand) in [
            (resolve(&npm("11.16.0"), args.clone()), "update"),
            (resolve(&yarn("1.22.22"), args.clone()), "upgrade"),
            (resolve(&yarn("4.16.0"), args.clone()), "up"),
            (resolve(&bun("1.3.14"), args), "update"),
        ] {
            assert!(resolution.diagnostics.is_empty());
            assert_eq!(
                expect_run(resolution.outcome).args,
                vec![
                    subcommand,
                    "--workspace",
                    "--recursive",
                    "--interactive",
                    "--filter",
                    "app",
                    "--filter",
                    "web",
                    "react"
                ]
            );
        }
    }

    #[test]
    fn test_yarn_v4_basic_update() {
        let resolution = resolve(&yarn("4.0.0"), update_args(&["react"]));
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["up", "react"]);
    }

    #[test]
    fn test_yarn_update_keeps_raw_pass_through() {
        let args = parse_args::<UpdateArgs>(["react", "--", "--no-save"]).unwrap();
        let resolution = resolve(&yarn("4.10.3"), args);
        assert!(resolution.diagnostics.is_empty());
        assert_eq!(expect_run(resolution.outcome).args, vec!["up", "--no-save", "react"]);
    }

    #[test]
    fn test_yarn_update_rejects_no_save() {
        for version in ["1.22.0", "2.0.0", "3.0.0", "4.0.0"] {
            let args = parse_args::<UpdateArgs>(["react", "--no-save"]).unwrap();
            expect_unsupported(
                resolve(&yarn(version), args),
                &["yarn does not support --no-save."],
            );
        }
    }

    #[test]
    fn test_yarn_v4_update_interactive() {
        let options = UpdateArgs { interactive: true, ..Default::default() };
        let resolution = resolve(&yarn("4.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["up", "--interactive"]);
    }

    #[test]
    fn test_yarn_berry_update_rejects_filter() {
        let mut options = update_args(&["react"]);
        options.filter = vec!["app".to_string()];
        let resolution = resolve(&yarn("4.0.0"), options);

        expect_unsupported(resolution, &["yarn >= 2 does not support --filter."]);
    }

    #[test]
    fn test_yarn_filters_and_selectors_are_rejected_together() {
        let args =
            parse_args::<UpdateArgs>(["--filter", "app", "-D", "-P", "--no-optional"]).unwrap();
        expect_unsupported(
            resolve(&yarn("4.16.0"), args),
            &[
                "yarn >= 2 does not support --filter.",
                "yarn does not support --dev.",
                "yarn >= 2 does not support --prod.",
                "yarn >= 2 does not support --no-optional.",
            ],
        );
    }

    #[test]
    fn test_yarn_v4_update_recursive() {
        let options = UpdateArgs { recursive: true, ..Default::default() };
        let resolution = resolve(&yarn("4.0.0"), options);
        assert!(resolution.diagnostics.is_empty());
        assert_eq!(expect_run(resolution.outcome).args, vec!["up", "--recursive", "**"]);
    }

    #[test]
    fn test_yarn_recursive_latest_uses_project_wide_update() {
        for version in ["2.4.2", "3.0.0", "3.6.0", "4.0.0", "4.16.0"] {
            for (argv, expected) in [
                (vec!["-r", "--latest", "react"], vec!["up", "react"]),
                (vec!["-r", "--latest"], vec!["up", "*"]),
                (
                    vec!["-r", "--latest", "--interactive", "react"],
                    vec!["up", "--interactive", "react"],
                ),
            ] {
                let args = parse_args::<UpdateArgs>(argv).unwrap();
                let resolution = resolve(&yarn(version), args);
                assert!(resolution.diagnostics.is_empty());
                assert_eq!(expect_run(resolution.outcome).args, expected);
            }
        }
    }

    #[test]
    fn test_yarn_recursive_preserves_ranges_and_raw_patterns() {
        for version in ["3.0.0", "3.6.0", "4.0.0", "4.16.0"] {
            for (argv, expected) in [
                (vec!["-r", "react"], vec!["up", "--recursive", "react"]),
                (vec!["-r", "--no-save", "react"], vec!["up", "--recursive", "react"]),
                (vec!["-r"], vec!["up", "--recursive", "**"]),
                (vec!["-r", "--no-save"], vec!["up", "--recursive", "**"]),
                (vec!["-r", "@test/scoped"], vec!["up", "--recursive", "@test/scoped"]),
                (vec!["-r", "--", "@test/*"], vec!["up", "--recursive", "@test/*"]),
                (vec!["-r", "--", "react"], vec!["up", "--recursive", "react"]),
                (vec!["-r", "--latest", "--", "react"], vec!["up", "react"]),
                (
                    vec!["-r", "--", "--mode", "skip-build"],
                    vec!["up", "--recursive", "--mode", "skip-build"],
                ),
            ] {
                let args = parse_args::<UpdateArgs>(argv).unwrap();
                let resolution = resolve(&yarn(version), args);
                assert!(resolution.diagnostics.is_empty());
                assert_eq!(expect_run(resolution.outcome).args, expected);
            }
        }
    }

    #[test]
    fn test_yarn_recursive_latest_rejects_no_save() {
        for version in ["3.0.0", "4.16.0"] {
            let args = parse_args::<UpdateArgs>(["-r", "--latest", "--no-save", "react"]).unwrap();
            expect_unsupported(
                resolve(&yarn(version), args),
                &["yarn does not support --no-save."],
            );
        }
    }

    #[test]
    fn test_npm_basic_update() {
        let resolution = resolve(&npm("11.0.0"), update_args(&["react"]));
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["update", "react"]);
    }

    #[test]
    fn test_npm_update_all() {
        let resolution = resolve(&npm("11.0.0"), UpdateArgs::default());
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["update"]);
    }

    #[test]
    fn test_npm_update_with_workspace() {
        let mut options = update_args(&["react"]);
        options.filter = vec!["app".to_string()];
        let resolution = resolve(&npm("11.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["update", "--workspace", "app", "react"]);
    }

    #[test]
    fn test_npm_update_recursive() {
        let options = UpdateArgs { recursive: true, ..Default::default() };
        let resolution = resolve(&npm("11.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["update", "--include-workspace-root", "--workspaces"]);
    }

    #[test]
    fn test_npm_update_preserves_native_selectors() {
        for (flag, native_flag) in
            [("-D", "--include=dev"), ("-P", "--include=prod"), ("--no-optional", "--no-optional")]
        {
            let args = parse_args::<UpdateArgs>([flag, "react"]).unwrap();
            let resolution = resolve(&npm("11.16.0"), args);
            assert!(resolution.diagnostics.is_empty());
            assert_eq!(expect_run(resolution.outcome).args, ["update", native_flag, "react"]);
        }
    }

    #[test]
    fn test_yarn_rejects_update_selectors() {
        for version in ["1.22.22", "2.4.2", "3.6.0", "4.16.0"] {
            let args = parse_args::<UpdateArgs>(["-D", "-P", "--no-optional"]).unwrap();
            let mut messages = vec!["yarn does not support --dev."];
            if version != "1.22.22" {
                messages.extend([
                    "yarn >= 2 does not support --prod.",
                    "yarn >= 2 does not support --no-optional.",
                ]);
            }
            expect_unsupported(resolve(&yarn(version), args), &messages);
        }
    }

    #[test]
    fn test_update_keeps_raw_selectors() {
        let args =
            parse_args::<UpdateArgs>(["react", "--", "--dev", "--prod", "--no-optional"]).unwrap();
        for (resolution, subcommand) in [
            (resolve(&npm("11.16.0"), args.clone()), "update"),
            (resolve(&yarn("1.22.22"), args.clone()), "upgrade"),
            (resolve(&yarn("4.16.0"), args.clone()), "up"),
            (resolve(&bun("1.3.14"), args), "update"),
        ] {
            assert!(resolution.diagnostics.is_empty());
            assert_eq!(
                expect_run(resolution.outcome).args,
                vec![subcommand, "--dev", "--prod", "--no-optional", "react"]
            );
        }
    }

    #[test]
    fn test_npm_update_no_save() {
        let mut options = update_args(&["react"]);
        options.no_save = true;
        let resolution = resolve(&npm("11.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["update", "--no-save", "react"]);
    }

    #[test]
    fn test_npm_unsupported_options_are_rejected_together() {
        let mut options = update_args(&["react"]);
        options.latest = true;
        options.interactive = true;
        options.dev = true;
        options.prod = true;
        options.no_optional = true;
        let resolution = resolve(&npm("11.0.0"), options);
        expect_unsupported(
            resolution,
            &["npm does not support --latest.", "npm does not support --interactive."],
        );
    }

    #[test]
    fn test_pnpm_update_multiple_packages() {
        let mut options = update_args(&["react", "react-dom", "vite"]);
        options.latest = true;
        let resolution = resolve(&pnpm("10.0.0"), options);
        let command = expect_run(resolution.outcome);

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
        let command = expect_run(resolution.outcome);

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
    fn test_yarn_berry_update_rejects_multiple_filters() {
        for recursive in [false, true] {
            let mut options = update_args(&["lodash"]);
            options.filter = vec!["app".to_string(), "web".to_string()];
            options.recursive = recursive;
            let resolution = resolve(&yarn("4.0.0"), options);

            expect_unsupported(resolution, &["yarn >= 2 does not support --filter."]);
        }
    }

    #[test]
    fn test_bun_basic_update() {
        let resolution = resolve(&bun("1.3.11"), UpdateArgs::default());
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["update"]);
    }

    #[test]
    fn test_bun_update_latest() {
        let options = UpdateArgs { latest: true, ..Default::default() };
        let resolution = resolve(&bun("1.3.11"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["update", "--latest"]);
    }

    #[test]
    fn test_bun_update_dev_only() {
        for version in ["1.4.0", "1.4.2"] {
            for (argv, expected) in [
                (vec!["-D"], vec!["update", "--dev"]),
                (
                    vec!["react", "--dev", "--latest", "--", "--ignore-scripts"],
                    vec!["update", "--latest", "--dev", "--ignore-scripts", "react"],
                ),
            ] {
                let args = parse_args::<UpdateArgs>(argv).unwrap();
                let resolution = resolve(&bun(version), args);
                assert!(resolution.diagnostics.is_empty());
                let command = expect_run(resolution.outcome);
                assert_eq!(command.program, "bun");
                assert_eq!(command.args, expected);
            }
        }
    }

    #[test]
    fn test_bun_update_rejects_dev_before_1_4() {
        let args = parse_args::<UpdateArgs>(["--dev", "react"]).unwrap();
        expect_unsupported(resolve(&bun("1.3.14"), args), &["bun < 1.4 does not support --dev."]);
    }

    #[test]
    fn test_bun_update_rejects_all_unsupported_options() {
        let args = parse_args::<UpdateArgs>([
            "--filter",
            "web",
            "--workspace-root",
            "--dev",
            "--prod",
            "--no-optional",
        ])
        .unwrap();
        expect_unsupported(
            resolve(&bun("1.3.14"), args),
            &[
                "bun < 1.4 does not support --filter.",
                "bun does not support --workspace-root.",
                "bun < 1.4 does not support --dev.",
            ],
        );
    }

    #[test]
    fn test_bun_update_prod_and_optional_selection() {
        for version in ["1.3.14", "1.4.0-beta.1", "1.4.0"] {
            let optional_flags =
                if version == "1.4.0" { vec!["--no-optional"] } else { vec!["--omit", "optional"] };
            for (flags, expected) in [
                (vec!["-P"], vec!["update", "--production"]),
                (vec!["--no-optional"], [vec!["update"], optional_flags.clone()].concat()),
                (
                    vec!["-P", "--no-optional"],
                    [vec!["update", "--production"], optional_flags].concat(),
                ),
            ] {
                let args = parse_args::<UpdateArgs>(flags).unwrap();
                let resolution = resolve(&bun(version), args);
                assert!(resolution.diagnostics.is_empty());
                assert_eq!(expect_run(resolution.outcome).args, expected);
            }
        }
    }

    #[test]
    fn test_bun_update_no_save() {
        let options = UpdateArgs { no_save: true, ..Default::default() };
        let resolution = resolve(&bun("1.3.11"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["update", "--no-save"]);
    }

    #[test]
    fn test_bun_update_recursive() {
        for version in ["1.3.11", "1.4.0"] {
            let options = UpdateArgs { recursive: true, ..Default::default() };
            let resolution = resolve(&bun(version), options);
            assert!(resolution.diagnostics.is_empty());
            let command = expect_run(resolution.outcome);
            assert_eq!(command.program, "bun");
            assert_eq!(command.args, vec!["update", "--recursive"]);
        }
    }

    #[test]
    fn test_bun_update_with_filter() {
        let options = UpdateArgs { filter: vec!["web".to_string()], ..Default::default() };
        let resolution = resolve(&bun("1.4.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["update", "--filter", "web"]);
        assert!(resolution.diagnostics.is_empty());
    }

    #[test]
    fn test_bun_update_rejects_filter_before_1_4() {
        let options = UpdateArgs { filter: vec!["web".to_string()], ..Default::default() };
        let resolution = resolve(&bun("1.3.11"), options);
        expect_unsupported(resolution, &["bun < 1.4 does not support --filter."]);
    }
}
