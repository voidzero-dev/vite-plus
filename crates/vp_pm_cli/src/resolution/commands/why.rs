use vp_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, DiagnosticKind, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct WhyArgs {
    /// Package(s) to check
    #[arg(required = true)]
    pub(crate) packages: Vec<String>,

    /// Output in JSON format
    #[arg(long, not_supported(bun))]
    pub(crate) json: bool,

    /// Show extended information
    #[arg(long, not_supported(npm, yarn, bun))]
    pub(crate) long: bool,

    /// Show parseable output
    #[arg(long, not_supported(npm, yarn, bun))]
    pub(crate) parseable: bool,

    /// Check recursively across all workspaces
    #[arg(short = 'r', long, not_supported(yarn < "2", bun))]
    pub(crate) recursive: bool,

    /// Filter packages in monorepo
    #[arg(long, value_name = "PATTERN", not_supported(yarn, bun))]
    pub(crate) filter: Vec<String>,

    /// Check in workspace root
    #[arg(short = 'w', long, not_supported(npm, yarn, bun))]
    pub(crate) workspace_root: bool,

    /// Only production dependencies
    #[arg(short = 'P', long, not_supported(npm, yarn, bun))]
    pub(crate) prod: bool,

    /// Only dev dependencies
    #[arg(short = 'D', long, not_supported(npm, yarn, bun))]
    pub(crate) dev: bool,

    /// Limit tree depth
    #[arg(long, not_supported(npm, yarn))]
    pub(crate) depth: Option<u32>,

    /// Exclude optional dependencies
    #[arg(long, not_supported(npm, yarn, bun))]
    pub(crate) no_optional: bool,

    /// Exclude peer dependencies
    #[arg(long, not_supported(npm, yarn < "2", bun))]
    pub(crate) exclude_peers: bool,

    /// Use a finder function defined in .pnpmfile.cjs
    #[arg(long, value_name = "FINDER_NAME", not_supported(npm, yarn, bun))]
    pub(crate) find_by: Option<String>,

    /// Additional arguments to pass through to the package manager
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) pass_through_args: Vec<String>,
}

impl WhyArgs {
    pub(crate) fn is_machine_readable(&self) -> bool {
        self.json
            || self.parseable
            || self.pass_through_args.iter().any(|arg| is_machine_readable_arg(arg))
    }
}

fn is_machine_readable_arg(arg: &str) -> bool {
    if matches!(arg, "--json" | "--parseable") {
        return true;
    }
    let Some((flag, value)) = arg.split_once('=') else {
        return false;
    };
    matches!(flag, "--json" | "--parseable") && !value.eq_ignore_ascii_case("false")
}

impl Resolve<WhyArgs> for Pnpm {
    fn resolve(&self, args: &WhyArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("pnpm");
        cmd.repeated("--filter", args.filter.iter())
            .arg("why")
            .arg_if("--json", args.json)
            .arg_if("--long", args.long)
            .arg_if("--parseable", args.parseable)
            .arg_if("--recursive", args.recursive)
            .arg_if("--workspace-root", args.workspace_root)
            .arg_if("--prod", args.prod)
            .arg_if("--dev", args.dev)
            .option("--depth", args.depth)
            .arg_if("--no-optional", args.no_optional)
            .arg_if("--exclude-peers", args.exclude_peers)
            .option("--find-by", args.find_by.as_ref())
            .extend(args.packages.iter())
            .extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<WhyArgs> for Npm {
    fn resolve(&self, args: &WhyArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("npm");
        cmd.arg("explain")
            .arg_if("--workspaces", args.recursive)
            .repeated("--workspace", args.filter.iter())
            .arg_if("--json", args.json)
            .extend(args.packages.iter())
            .extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<WhyArgs> for Yarn {
    fn resolve(&self, args: &WhyArgs, diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("yarn");
        cmd.arg("why");
        if args.packages.len() > 1 {
            diag.warn(
                DiagnosticKind::BehaviorChange,
                "yarn only supports checking one package at a time, using first package",
            );
        }
        cmd.arg(&args.packages[0]).arg_if("--json", args.json);
        if self.is_berry() {
            cmd.arg_if("--recursive", args.recursive).arg_if("--peers", !args.exclude_peers);
        }
        cmd.extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<WhyArgs> for Bun {
    fn resolve(&self, args: &WhyArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("bun");
        cmd.arg("why").extend(args.packages.iter()).option("--depth", args.depth);
        cmd.extend(args.pass_through_args.iter());
        cmd.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        resolve,
        test_utils::{bun, expect_run, expect_unsupported, npm, pnpm, yarn},
    };

    fn why_args(packages: &[&str]) -> WhyArgs {
        WhyArgs {
            packages: packages.iter().map(ToString::to_string).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn test_pnpm_why_basic() {
        let command = expect_run(resolve(&pnpm("10.0.0"), why_args(&["react"])).outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["why", "react"]);
    }

    #[test]
    fn test_pnpm_why_multiple_packages() {
        let command = expect_run(resolve(&pnpm("10.0.0"), why_args(&["react", "lodash"])).outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["why", "react", "lodash"]);
    }

    #[test]
    fn test_pnpm_why_json() {
        let mut options = why_args(&["react"]);
        options.json = true;
        let command = expect_run(resolve(&pnpm("10.0.0"), options).outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["why", "--json", "react"]);
    }

    #[test]
    fn test_npm_explain_basic() {
        let command = expect_run(resolve(&npm("11.0.0"), why_args(&["react"])).outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["explain", "react"]);
    }

    #[test]
    fn test_npm_explain_multiple_packages() {
        let command = expect_run(resolve(&npm("11.0.0"), why_args(&["react", "lodash"])).outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["explain", "react", "lodash"]);
    }

    #[test]
    fn test_npm_explain_with_workspace() {
        let mut options = why_args(&["react"]);
        options.filter = vec!["app".to_string()];
        let command = expect_run(resolve(&npm("11.0.0"), options).outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["explain", "--workspace", "app", "react"]);
    }

    #[test]
    fn test_yarn_why_basic() {
        let command = expect_run(resolve(&yarn("4.0.0"), why_args(&["react"])).outcome);

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["why", "react", "--peers"]);
    }

    #[test]
    fn test_yarn_why_with_exclude_peers() {
        let mut options = why_args(&["react"]);
        options.exclude_peers = true;
        let command = expect_run(resolve(&yarn("4.0.0"), options).outcome);

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["why", "react"]);
    }

    #[test]
    fn test_yarn1_why_no_peers() {
        let command = expect_run(resolve(&yarn("1.22.0"), why_args(&["react"])).outcome);

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["why", "react"]);
    }

    #[test]
    fn test_yarn_why_multiple_packages_warns_and_uses_first_package() {
        let resolution = resolve(&yarn("4.0.0"), why_args(&["react", "lodash"]));
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["why", "react", "--peers"]);
        assert_eq!(
            resolution.diagnostics[0].message,
            "yarn only supports checking one package at a time, using first package"
        );
    }

    #[test]
    fn test_pnpm_why_with_filter() {
        let mut options = why_args(&["react"]);
        options.filter = vec!["app".to_string()];
        let command = expect_run(resolve(&pnpm("10.0.0"), options).outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["--filter", "app", "why", "react"]);
    }

    #[test]
    fn test_pnpm_why_with_depth() {
        let mut options = why_args(&["react"]);
        options.depth = Some(3);
        let command = expect_run(resolve(&pnpm("10.0.0"), options).outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["why", "--depth", "3", "react"]);
    }

    #[test]
    fn test_pnpm_why_with_find_by() {
        let mut options = why_args(&["react"]);
        options.find_by = Some("customFinder".to_string());
        let command = expect_run(resolve(&pnpm("10.0.0"), options).outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["why", "--find-by", "customFinder", "react"]);
    }

    #[test]
    fn test_bun_why_with_depth() {
        let mut options = why_args(&["testnpm2"]);
        options.depth = Some(2);
        let command = expect_run(resolve(&bun("1.3.11"), options).outcome);

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["why", "testnpm2", "--depth", "2"]);
    }

    #[test]
    fn test_yarn_why_json() {
        for version in ["1.22.22", "4.18.0"] {
            let mut options = why_args(&["react"]);
            options.json = true;
            let resolution = resolve(&yarn(version), options);
            assert!(resolution.diagnostics.is_empty());
            let command = expect_run(resolution.outcome);
            assert_eq!(command.program, "yarn");
            if version.starts_with("1.") {
                assert_eq!(command.args, vec!["why", "react", "--json"]);
            } else {
                assert_eq!(command.args, vec!["why", "react", "--json", "--peers"]);
            }
        }
    }

    #[test]
    fn yarn_rejects_depth_and_optional_filtering() {
        let args = WhyArgs { depth: Some(0), no_optional: true, ..why_args(&["react"]) };
        expect_unsupported(
            resolve(&yarn("4.18.0"), args),
            &["yarn does not support --depth.", "yarn does not support --no-optional."],
        );
    }

    #[test]
    fn classic_rejects_recursive_and_peer_options() {
        let args = WhyArgs { recursive: true, exclude_peers: true, ..why_args(&["react"]) };
        expect_unsupported(
            resolve(&yarn("1.22.22"), args),
            &[
                "yarn < 2 does not support --recursive.",
                "yarn < 2 does not support --exclude-peers.",
            ],
        );
    }

    #[test]
    fn berry_preserves_recursive_and_peer_options() {
        let args = WhyArgs { recursive: true, exclude_peers: true, ..why_args(&["react"]) };
        let resolution = resolve(&yarn("4.18.0"), args);
        assert!(resolution.diagnostics.is_empty());
        assert_eq!(expect_run(resolution.outcome).args, vec!["why", "react", "--recursive"]);
    }

    #[test]
    fn classic_preserves_raw_recursive_and_peer_options() {
        let args = WhyArgs {
            pass_through_args: vec!["--recursive".to_string(), "--exclude-peers".to_string()],
            ..why_args(&["react"])
        };
        let resolution = resolve(&yarn("1.22.22"), args);
        assert!(resolution.diagnostics.is_empty());
        assert_eq!(
            expect_run(resolution.outcome).args,
            vec!["why", "react", "--recursive", "--exclude-peers"]
        );
    }

    #[test]
    fn workspace_selectors_preserve_native_and_raw_arguments() {
        let args = WhyArgs { recursive: true, workspace_root: true, ..why_args(&["react"]) };
        let resolution = resolve(&pnpm("11.3.0"), args);
        assert!(resolution.diagnostics.is_empty());
        assert_eq!(
            expect_run(resolution.outcome).args,
            vec!["why", "--recursive", "--workspace-root", "react"]
        );

        let args = WhyArgs { recursive: true, ..why_args(&["react"]) };
        let resolution = resolve(&npm("11.16.0"), args);
        assert!(resolution.diagnostics.is_empty());
        assert_eq!(expect_run(resolution.outcome).args, vec!["explain", "--workspaces", "react"]);

        let args = WhyArgs {
            pass_through_args: [
                "--recursive",
                "--workspace-root",
                "--no-optional",
                "--exclude-peers",
            ]
            .map(str::to_string)
            .to_vec(),
            ..why_args(&["react"])
        };
        let resolution = resolve(&npm("12.0.2"), args);
        assert!(resolution.diagnostics.is_empty());
        assert_eq!(
            expect_run(resolution.outcome).args,
            vec![
                "explain",
                "react",
                "--recursive",
                "--workspace-root",
                "--no-optional",
                "--exclude-peers"
            ]
        );
    }

    #[test]
    fn supported_depth_and_optional_filtering_are_preserved() {
        let args = WhyArgs {
            depth: Some(0),
            no_optional: true,
            exclude_peers: true,
            ..why_args(&["react"])
        };
        let resolution = resolve(&pnpm("11.3.0"), args);
        assert!(resolution.diagnostics.is_empty());
        assert_eq!(
            expect_run(resolution.outcome).args,
            vec!["why", "--depth", "0", "--no-optional", "--exclude-peers", "react"]
        );

        let args = WhyArgs { depth: Some(0), ..why_args(&["react"]) };
        let resolution = resolve(&bun("1.4.0"), args);
        assert!(resolution.diagnostics.is_empty());
        assert_eq!(expect_run(resolution.outcome).args, vec!["why", "react", "--depth", "0"]);
    }

    #[test]
    fn unsupported_fields_are_rejected_for_yarn_npm_and_bun() {
        let mut yarn_options = why_args(&["react"]);
        yarn_options.json = true;
        yarn_options.long = true;
        yarn_options.parseable = true;
        yarn_options.filter = vec!["app".to_string()];
        yarn_options.prod = true;
        yarn_options.dev = true;
        yarn_options.find_by = Some("customFinder".to_string());
        let yarn_resolution = resolve(&yarn("1.22.0"), yarn_options);
        expect_unsupported(
            yarn_resolution,
            &[
                "yarn does not support --long.",
                "yarn does not support --parseable.",
                "yarn does not support --filter.",
                "yarn does not support --prod.",
                "yarn does not support --dev.",
                "yarn does not support --find-by.",
            ],
        );

        let mut npm_options = why_args(&["react"]);
        npm_options.long = true;
        npm_options.parseable = true;
        npm_options.recursive = true;
        npm_options.workspace_root = true;
        npm_options.prod = true;
        npm_options.dev = true;
        npm_options.depth = Some(2);
        npm_options.no_optional = true;
        npm_options.exclude_peers = true;
        npm_options.find_by = Some("customFinder".to_string());
        let npm_resolution = resolve(&npm("11.0.0"), npm_options);
        expect_unsupported(
            npm_resolution,
            &[
                "npm does not support --long.",
                "npm does not support --parseable.",
                "npm does not support --workspace-root.",
                "npm does not support --prod.",
                "npm does not support --dev.",
                "npm does not support --depth.",
                "npm does not support --no-optional.",
                "npm does not support --exclude-peers.",
                "npm does not support --find-by.",
            ],
        );

        let mut bun_options = why_args(&["react"]);
        bun_options.json = true;
        bun_options.long = true;
        bun_options.parseable = true;
        bun_options.recursive = true;
        bun_options.filter = vec!["app".to_string()];
        bun_options.workspace_root = true;
        bun_options.prod = true;
        bun_options.dev = true;
        bun_options.no_optional = true;
        bun_options.exclude_peers = true;
        bun_options.find_by = Some("customFinder".to_string());
        let bun_resolution = resolve(&bun("1.3.11"), bun_options);
        expect_unsupported(
            bun_resolution,
            &[
                "bun does not support --json.",
                "bun does not support --long.",
                "bun does not support --parseable.",
                "bun does not support --recursive.",
                "bun does not support --filter.",
                "bun does not support --workspace-root.",
                "bun does not support --prod.",
                "bun does not support --dev.",
                "bun does not support --no-optional.",
                "bun does not support --exclude-peers.",
                "bun does not support --find-by.",
            ],
        );
    }
}
