use vite_pm_cli_macros::pm_args;

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
    #[arg(long, not_supported(yarn, bun))]
    pub(crate) json: bool,

    /// Show extended information
    #[arg(long, not_supported(npm, yarn, bun))]
    pub(crate) long: bool,

    /// Show parseable output
    #[arg(long, not_supported(npm, yarn, bun))]
    pub(crate) parseable: bool,

    /// Check recursively across all workspaces
    #[arg(short = 'r', long, not_supported(bun))]
    pub(crate) recursive: bool,

    /// Filter packages in monorepo
    #[arg(long, value_name = "PATTERN", not_supported(yarn, bun))]
    pub(crate) filter: Vec<String>,

    /// Check in workspace root
    #[arg(short = 'w', long, not_supported(bun))]
    pub(crate) workspace_root: bool,

    /// Only production dependencies
    #[arg(short = 'P', long, not_supported(npm, yarn, bun))]
    pub(crate) prod: bool,

    /// Only dev dependencies
    #[arg(short = 'D', long, not_supported(npm, yarn, bun))]
    pub(crate) dev: bool,

    /// Limit tree depth
    #[arg(long, not_supported(npm))]
    pub(crate) depth: Option<u32>,

    /// Exclude optional dependencies
    #[arg(long, not_supported(bun))]
    pub(crate) no_optional: bool,

    /// Exclude peer dependencies
    #[arg(long, not_supported(bun))]
    pub(crate) exclude_peers: bool,

    /// Use a finder function defined in .pnpmfile.cjs
    #[arg(long, value_name = "FINDER_NAME", not_supported(npm, yarn, bun))]
    pub(crate) find_by: Option<String>,

    /// Additional arguments to pass through to the package manager
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) pass_through_args: Vec<String>,
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
        cmd.arg(&args.packages[0]);
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
        test_utils::{bun, npm, pnpm, yarn},
    };

    fn why_args(packages: &[&str]) -> WhyArgs {
        WhyArgs {
            packages: packages.iter().map(ToString::to_string).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn test_pnpm_why_basic() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), why_args(&["react"])).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["why", "react"]);
    }

    #[test]
    fn test_pnpm_why_multiple_packages() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), why_args(&["react", "lodash"])).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["why", "react", "lodash"]);
    }

    #[test]
    fn test_pnpm_why_json() {
        let mut options = why_args(&["react"]);
        options.json = true;
        let CommandResolution::Run(command) = resolve(&pnpm("10.0.0"), options).outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["why", "--json", "react"]);
    }

    #[test]
    fn test_npm_explain_basic() {
        let CommandResolution::Run(command) = resolve(&npm("11.0.0"), why_args(&["react"])).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["explain", "react"]);
    }

    #[test]
    fn test_npm_explain_multiple_packages() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), why_args(&["react", "lodash"])).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["explain", "react", "lodash"]);
    }

    #[test]
    fn test_npm_explain_with_workspace() {
        let mut options = why_args(&["react"]);
        options.filter = vec!["app".to_string()];
        let CommandResolution::Run(command) = resolve(&npm("11.0.0"), options).outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["explain", "--workspace", "app", "react"]);
    }

    #[test]
    fn test_yarn_why_basic() {
        let CommandResolution::Run(command) = resolve(&yarn("4.0.0"), why_args(&["react"])).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["why", "react", "--peers"]);
    }

    #[test]
    fn test_yarn_why_with_exclude_peers() {
        let mut options = why_args(&["react"]);
        options.exclude_peers = true;
        let CommandResolution::Run(command) = resolve(&yarn("4.0.0"), options).outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["why", "react"]);
    }

    #[test]
    fn test_yarn1_why_no_peers() {
        let CommandResolution::Run(command) =
            resolve(&yarn("1.22.0"), why_args(&["react"])).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["why", "react"]);
    }

    #[test]
    fn test_yarn_why_multiple_packages_warns_and_uses_first_package() {
        let resolution = resolve(&yarn("4.0.0"), why_args(&["react", "lodash"]));
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

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
        let CommandResolution::Run(command) = resolve(&pnpm("10.0.0"), options).outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["--filter", "app", "why", "react"]);
    }

    #[test]
    fn test_pnpm_why_with_depth() {
        let mut options = why_args(&["react"]);
        options.depth = Some(3);
        let CommandResolution::Run(command) = resolve(&pnpm("10.0.0"), options).outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["why", "--depth", "3", "react"]);
    }

    #[test]
    fn test_pnpm_why_with_find_by() {
        let mut options = why_args(&["react"]);
        options.find_by = Some("customFinder".to_string());
        let CommandResolution::Run(command) = resolve(&pnpm("10.0.0"), options).outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["why", "--find-by", "customFinder", "react"]);
    }

    #[test]
    fn test_bun_why_with_depth() {
        let mut options = why_args(&["testnpm2"]);
        options.depth = Some(2);
        let CommandResolution::Run(command) = resolve(&bun("1.3.11"), options).outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["why", "testnpm2", "--depth", "2"]);
    }

    #[test]
    fn unsupported_fields_are_dropped_for_yarn_npm_and_bun() {
        let mut yarn_options = why_args(&["react"]);
        yarn_options.json = true;
        yarn_options.long = true;
        yarn_options.parseable = true;
        yarn_options.filter = vec!["app".to_string()];
        yarn_options.prod = true;
        yarn_options.dev = true;
        yarn_options.find_by = Some("customFinder".to_string());
        let yarn_resolution = resolve(&yarn("1.22.0"), yarn_options);
        let CommandResolution::Run(yarn_command) = yarn_resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(yarn_command.args, vec!["why", "react"]);
        assert_eq!(yarn_resolution.diagnostics.len(), 7);

        let mut npm_options = why_args(&["react"]);
        npm_options.long = true;
        npm_options.parseable = true;
        npm_options.prod = true;
        npm_options.dev = true;
        npm_options.depth = Some(2);
        npm_options.find_by = Some("customFinder".to_string());
        let npm_resolution = resolve(&npm("11.0.0"), npm_options);
        let CommandResolution::Run(npm_command) = npm_resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(npm_command.args, vec!["explain", "react"]);
        assert_eq!(npm_resolution.diagnostics.len(), 6);

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
        let CommandResolution::Run(bun_command) = bun_resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(bun_command.args, vec!["why", "react"]);
        assert_eq!(bun_resolution.diagnostics.len(), 11);
    }
}
