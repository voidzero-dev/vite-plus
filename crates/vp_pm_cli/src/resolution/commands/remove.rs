use vp_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct RemoveArgs {
    /// Only remove from `devDependencies` (pnpm-specific)
    #[arg(short = 'D', long, not_supported(yarn, bun))]
    pub(crate) save_dev: bool,

    /// Only remove from `optionalDependencies` (pnpm-specific)
    #[arg(short = 'O', long, not_supported(yarn, bun))]
    pub(crate) save_optional: bool,

    /// Only remove from `dependencies` (pnpm-specific)
    #[arg(short = 'P', long, not_supported(yarn, bun))]
    pub(crate) save_prod: bool,

    /// Filter packages in monorepo (can be used multiple times)
    #[arg(long, value_name = "PATTERN", not_supported(bun < "1.4"))]
    pub(crate) filter: Vec<String>,

    /// Remove from workspace root
    #[arg(short = 'w', long, not_supported(yarn, bun))]
    pub(crate) workspace_root: bool,

    /// Remove recursively from all workspace packages
    #[arg(short = 'r', long, not_supported(yarn < "2", bun))]
    pub(crate) recursive: bool,

    /// Remove global packages
    #[arg(short = 'g', long)]
    pub(crate) global: bool,

    /// Preview what would be removed without actually removing (only with -g)
    #[arg(long, requires = "global")]
    pub(crate) dry_run: bool,

    /// Packages to remove
    #[arg(required = true)]
    pub(crate) packages: Vec<String>,

    /// Additional arguments to pass through to the package manager
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) pass_through_args: Vec<String>,
}

impl Resolve<RemoveArgs> for Pnpm {
    fn resolve(&self, args: &RemoveArgs, _diag: &mut Diagnostics) -> CommandResolution {
        if args.global {
            return Npm::resolve_remove(args);
        }

        let mut cmd = CommandBuilder::new("pnpm");
        cmd.repeated("--filter", args.filter.iter())
            .arg("remove")
            .arg_if("--workspace-root", args.workspace_root)
            .arg_if("--recursive", args.recursive)
            .arg_if("--save-dev", args.save_dev)
            .arg_if("--save-optional", args.save_optional)
            .arg_if("--save-prod", args.save_prod)
            .extend(args.pass_through_args.iter())
            .extend(args.packages.iter());
        cmd.into()
    }
}

impl Npm {
    fn resolve_remove(args: &RemoveArgs) -> CommandResolution {
        let mut cmd = CommandBuilder::new("npm");
        if args.global {
            cmd.arg("uninstall")
                .arg("--global")
                .extend(args.pass_through_args.iter())
                .extend(args.packages.iter());
            return cmd.into();
        }

        cmd.arg("uninstall").repeated("--workspace", args.filter.iter());
        if args.workspace_root || args.recursive {
            cmd.arg("--include-workspace-root");
        }
        cmd.arg_if("--workspaces", args.recursive)
            .arg_if("--save-dev", args.save_dev)
            .arg_if("--save-optional", args.save_optional)
            .arg_if("--save-prod", args.save_prod)
            .extend(args.pass_through_args.iter())
            .extend(args.packages.iter());
        cmd.into()
    }
}

impl Resolve<RemoveArgs> for Npm {
    fn resolve(&self, args: &RemoveArgs, _diag: &mut Diagnostics) -> CommandResolution {
        Self::resolve_remove(args)
    }
}

impl Resolve<RemoveArgs> for Yarn {
    fn resolve(&self, args: &RemoveArgs, _diag: &mut Diagnostics) -> CommandResolution {
        if args.global {
            return Npm::resolve_remove(args);
        }

        let mut cmd = CommandBuilder::new("yarn");
        if !args.filter.is_empty() {
            if self.is_berry() {
                cmd.arg("workspaces").arg("foreach").arg("--all");
                cmd.repeated("--include", args.filter.iter());
            } else {
                // Classic runs one workspace at a time: `yarn workspace <name> remove`.
                // https://classic.yarnpkg.com/en/docs/cli/workspace
                if args.filter.len() > 1 {
                    return CommandResolution::InvalidArgument(
                        "yarn < 2 does not support multiple --filter options.".into(),
                    );
                }
                cmd.arg("workspace").arg(&args.filter[0]);
            }
        }
        cmd.arg("remove")
            .arg_if("--all", args.recursive && args.filter.is_empty())
            .extend(args.pass_through_args.iter())
            .extend(args.packages.iter());
        cmd.into()
    }
}

impl Resolve<RemoveArgs> for Bun {
    fn resolve(&self, args: &RemoveArgs, _diag: &mut Diagnostics) -> CommandResolution {
        if args.global {
            return Npm::resolve_remove(args);
        }

        let mut cmd = CommandBuilder::new("bun");
        cmd.arg("remove")
            .repeated("--filter", args.filter.iter())
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

    fn remove_args(packages: &[&str]) -> RemoveArgs {
        RemoveArgs {
            packages: packages.iter().map(ToString::to_string).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn dry_run_requires_global() {
        let error = parse_args::<RemoveArgs>(["--dry-run", "lodash"]).unwrap_err();

        assert_eq!(error.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn dry_run_with_global_parses() {
        let args = parse_args::<RemoveArgs>(["--global", "--dry-run", "lodash"]).unwrap();

        assert!(args.global);
        assert!(args.dry_run);
        assert_eq!(args.packages, vec!["lodash"]);
    }

    #[test]
    fn test_pnpm_basic_remove() {
        let resolution = resolve(&pnpm("1.0.0"), remove_args(&["lodash"]));
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["remove", "lodash"]);
    }

    #[test]
    fn test_pnpm_remove_with_filter() {
        let mut options = remove_args(&["lodash"]);
        options.filter = vec!["app".to_string()];
        let resolution = resolve(&pnpm("1.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["--filter", "app", "remove", "lodash"]);
    }

    #[test]
    fn test_pnpm_remove_workspace_root() {
        let mut options = remove_args(&["typescript"]);
        options.workspace_root = true;
        let resolution = resolve(&pnpm("1.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["remove", "--workspace-root", "typescript"]);
    }

    #[test]
    fn test_pnpm_remove_recursive() {
        let mut options = remove_args(&["lodash"]);
        options.recursive = true;
        let resolution = resolve(&pnpm("1.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["remove", "--recursive", "lodash"]);
    }

    #[test]
    fn test_pnpm_remove_multiple_filters() {
        let mut options = remove_args(&["axios"]);
        options.filter = vec!["app".to_string(), "web".to_string()];
        let resolution = resolve(&pnpm("1.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["--filter", "app", "--filter", "web", "remove", "axios"]);
    }

    #[test]
    fn test_yarn_basic_remove() {
        let resolution = resolve(&yarn("1.22.0"), remove_args(&["lodash"]));
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["remove", "lodash"]);
    }

    #[test]
    fn yarn_rejects_unsupported_workspace_root() {
        for version in ["1.22.22", "4.0.0"] {
            let mut options = remove_args(&["lodash"]);
            options.workspace_root = true;
            let resolution = resolve(&yarn(version), options);
            expect_unsupported(resolution, &["yarn does not support --workspace-root."]);
        }
    }

    #[test]
    fn test_yarn_classic_single_filter_uses_workspace_remove() {
        let mut options = remove_args(&["lodash"]);
        options.filter = vec!["app".to_string()];
        let resolution = resolve(&yarn("1.22.22"), options);
        assert!(resolution.diagnostics.is_empty());
        assert_eq!(
            expect_run(resolution.outcome).args,
            vec!["workspace", "app", "remove", "lodash"]
        );
    }

    #[test]
    fn test_yarn_classic_rejects_multiple_filters() {
        let mut options = remove_args(&["lodash"]);
        options.filter = vec!["app".to_string(), "web".to_string()];
        expect_unsupported(
            resolve(&yarn("1.22.22"), options),
            &["yarn < 2 does not support multiple --filter options."],
        );
    }

    #[test]
    fn test_yarn_remove_recursive() {
        let args = parse_args::<RemoveArgs>(["--recursive", "lodash"]).unwrap();
        expect_unsupported(
            resolve(&yarn("1.22.22"), args.clone()),
            &["yarn < 2 does not support --recursive."],
        );
        for version in ["2.4.2", "4.16.0"] {
            let resolution = resolve(&yarn(version), args.clone());
            assert!(resolution.diagnostics.is_empty());
            assert_eq!(expect_run(resolution.outcome).args, vec!["remove", "--all", "lodash"]);
        }
    }

    #[test]
    fn test_classic_remove_aggregates_scope_and_selector_errors() {
        let args = parse_args::<RemoveArgs>(["-D", "--filter", "app", "-r", "lodash"]).unwrap();
        expect_unsupported(
            resolve(&yarn("1.22.22"), args),
            &["yarn does not support --save-dev.", "yarn < 2 does not support --recursive."],
        );
    }

    #[test]
    fn test_bun_remove_rejects_recursive() {
        for version in ["1.3.11", "1.3.14", "1.4.0"] {
            let args = parse_args::<RemoveArgs>(["-r", "lodash"]).unwrap();
            expect_unsupported(
                resolve(&bun(version), args),
                &["bun does not support --recursive."],
            );
        }
    }

    #[test]
    fn test_bun_remove_rejects_recursive_with_filters() {
        for version in ["1.3.11", "1.3.14", "1.4.0"] {
            let args =
                parse_args::<RemoveArgs>(["-r", "--filter", "app", "--filter", "web", "lodash"])
                    .unwrap();
            let mut messages = vec![];
            if version != "1.4.0" {
                messages.push("bun < 1.4 does not support --filter.");
            }
            messages.push("bun does not support --recursive.");
            expect_unsupported(resolve(&bun(version), args), &messages);
        }
    }

    #[test]
    fn test_remove_preserves_raw_workspace_options() {
        let args =
            parse_args::<RemoveArgs>(["lodash", "--", "--recursive", "--filter", "app"]).unwrap();
        for resolution in [resolve(&yarn("1.22.22"), args.clone()), resolve(&bun("1.3.14"), args)] {
            assert!(resolution.diagnostics.is_empty());
            assert_eq!(
                expect_run(resolution.outcome).args,
                vec!["remove", "--recursive", "--filter", "app", "lodash"]
            );
        }
    }

    #[test]
    fn test_global_remove_preserves_npm_fallback() {
        let args = parse_args::<RemoveArgs>(["--global", "typescript"]).unwrap();
        for resolution in [
            resolve(&yarn("1.22.22"), args.clone()),
            resolve(&yarn("4.16.0"), args.clone()),
            resolve(&bun("1.3.14"), args.clone()),
            resolve(&bun("1.4.0"), args),
        ] {
            assert!(resolution.diagnostics.is_empty());
            let command = expect_run(resolution.outcome);
            assert_eq!(command.program, "npm");
            assert_eq!(command.args, vec!["uninstall", "--global", "typescript"]);
        }
    }

    #[test]
    fn test_yarn_berry_remove_with_workspace() {
        let mut options = remove_args(&["lodash"]);
        options.filter = vec!["app".to_string()];
        let resolution = resolve(&yarn("4.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "yarn");
        assert_eq!(
            command.args,
            vec!["workspaces", "foreach", "--all", "--include", "app", "remove", "lodash"]
        );
    }

    #[test]
    fn test_npm_basic_remove() {
        let resolution = resolve(&npm("1.0.0"), remove_args(&["lodash"]));
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["uninstall", "lodash"]);
    }

    #[test]
    fn test_npm_remove_with_workspace() {
        let mut options = remove_args(&["lodash"]);
        options.filter = vec!["app".to_string()];
        let resolution = resolve(&npm("1.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["uninstall", "--workspace", "app", "lodash"]);
    }

    #[test]
    fn test_npm_remove_workspace_root() {
        let mut options = remove_args(&["typescript"]);
        options.workspace_root = true;
        let resolution = resolve(&npm("1.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["uninstall", "--include-workspace-root", "typescript"]);
    }

    #[test]
    fn test_npm_remove_recursive() {
        let mut options = remove_args(&["lodash"]);
        options.recursive = true;
        let resolution = resolve(&npm("1.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(
            command.args,
            vec!["uninstall", "--include-workspace-root", "--workspaces", "lodash"]
        );
    }

    #[test]
    fn test_npm_remove_multiple_workspaces() {
        let mut options = remove_args(&["lodash"]);
        options.filter = vec!["app".to_string(), "web".to_string()];
        let resolution = resolve(&npm("1.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(
            command.args,
            vec!["uninstall", "--workspace", "app", "--workspace", "web", "lodash"]
        );
    }

    #[test]
    fn test_bun_basic_remove() {
        let resolution = resolve(&bun("1.0.0"), remove_args(&["lodash"]));
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["remove", "lodash"]);
    }

    #[test]
    fn test_global_remove() {
        let mut options = remove_args(&["typescript"]);
        options.global = true;
        let resolution = resolve(&pnpm("1.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["uninstall", "--global", "typescript"]);
    }

    #[test]
    fn test_remove_multiple_packages() {
        let resolution = resolve(&pnpm("1.0.0"), remove_args(&["lodash", "axios", "underscore"]));
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["remove", "lodash", "axios", "underscore"]);
    }

    #[test]
    fn test_remove_with_pass_through_args() {
        let mut options = remove_args(&["lodash"]);
        options.pass_through_args = vec!["--use-stderr".to_string()];
        let resolution = resolve(&pnpm("1.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["remove", "--use-stderr", "lodash"]);
    }

    #[test]
    fn test_global_remove_with_pass_through_args() {
        let mut options = remove_args(&["typescript"]);
        options.global = true;
        options.pass_through_args = vec!["--foreground-scripts".to_string()];
        let resolution = resolve(&pnpm("1.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(
            command.args,
            vec!["uninstall", "--global", "--foreground-scripts", "typescript"]
        );
    }

    #[test]
    fn test_pnpm_remove_save_dev() {
        let mut options = remove_args(&["typescript"]);
        options.save_dev = true;
        let resolution = resolve(&pnpm("1.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["remove", "--save-dev", "typescript"]);
    }

    #[test]
    fn test_pnpm_remove_save_optional() {
        let mut options = remove_args(&["sharp"]);
        options.save_optional = true;
        let resolution = resolve(&pnpm("1.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["remove", "--save-optional", "sharp"]);
    }

    #[test]
    fn test_pnpm_remove_save_prod() {
        let mut options = remove_args(&["react"]);
        options.save_prod = true;
        let resolution = resolve(&pnpm("1.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["remove", "--save-prod", "react"]);
    }

    #[test]
    fn test_npm_remove_preserves_native_save_flags() {
        for (flag, native_flag) in
            [("-D", "--save-dev"), ("-O", "--save-optional"), ("-P", "--save-prod")]
        {
            let args = parse_args::<RemoveArgs>([flag, "react"]).unwrap();
            let resolution = resolve(&npm("11.16.0"), args);
            assert!(resolution.diagnostics.is_empty());
            assert_eq!(expect_run(resolution.outcome).args, ["uninstall", native_flag, "react"]);
        }
    }

    #[test]
    fn test_remove_rejects_all_save_flags() {
        let args = parse_args::<RemoveArgs>(["-D", "-O", "-P", "lodash"]).unwrap();
        for (resolution, name) in [
            (resolve(&yarn("1.22.22"), args.clone()), "yarn"),
            (resolve(&yarn("4.16.0"), args.clone()), "yarn"),
            (resolve(&bun("1.3.11"), args.clone()), "bun"),
            (resolve(&bun("1.4.0"), args), "bun"),
        ] {
            expect_unsupported(
                resolution,
                &[
                    &vt_str::format!("{name} does not support --save-dev."),
                    &vt_str::format!("{name} does not support --save-optional."),
                    &vt_str::format!("{name} does not support --save-prod."),
                ],
            );
        }
    }

    #[test]
    fn test_remove_keeps_raw_save_flags() {
        let args = parse_args::<RemoveArgs>([
            "lodash",
            "--",
            "--save-dev",
            "--save-optional",
            "--save-prod",
        ])
        .unwrap();
        for (resolution, subcommand) in [
            (resolve(&npm("11.16.0"), args.clone()), "uninstall"),
            (resolve(&yarn("1.22.22"), args.clone()), "remove"),
            (resolve(&yarn("4.16.0"), args.clone()), "remove"),
            (resolve(&bun("1.4.0"), args), "remove"),
        ] {
            assert!(resolution.diagnostics.is_empty());
            assert_eq!(
                expect_run(resolution.outcome).args,
                vec![subcommand, "--save-dev", "--save-optional", "--save-prod", "lodash"]
            );
        }
    }

    #[test]
    fn test_yarn_berry_remove_with_multiple_filters() {
        let mut options = remove_args(&["lodash"]);
        options.filter = vec!["app".to_string(), "web".to_string()];
        let resolution = resolve(&yarn("4.0.0"), options);
        let command = expect_run(resolution.outcome);

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
                "remove",
                "lodash"
            ]
        );
    }

    #[test]
    fn test_yarn_berry_remove_with_recursive_and_multiple_filters() {
        let mut options = remove_args(&["lodash"]);
        options.filter = vec!["app".to_string(), "web".to_string()];
        options.recursive = true;
        let resolution = resolve(&yarn("4.0.0"), options);
        let command = expect_run(resolution.outcome);

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
                "remove",
                "lodash"
            ]
        );
    }

    #[test]
    fn bun_rejects_unsupported_filter_and_workspace_root() {
        let mut options = remove_args(&["lodash"]);
        options.filter = vec!["app".to_string()];
        options.workspace_root = true;
        let resolution = resolve(&bun("1.0.0"), options);
        expect_unsupported(
            resolution,
            &["bun < 1.4 does not support --filter.", "bun does not support --workspace-root."],
        );
    }

    #[test]
    fn bun_1_4_supports_filter() {
        let mut options = remove_args(&["lodash"]);
        options.filter = vec!["app".to_string()];
        let resolution = resolve(&bun("1.4.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["remove", "--filter", "app", "lodash"]);
        assert!(resolution.diagnostics.is_empty());
    }
}
