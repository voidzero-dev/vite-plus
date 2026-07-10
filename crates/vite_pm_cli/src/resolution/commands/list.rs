use vite_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, DiagnosticKind, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct ListArgs {
    /// Package pattern to filter
    pub(crate) pattern: Option<String>,

    /// Maximum depth of dependency tree
    #[arg(long, not_supported(bun))]
    pub(crate) depth: Option<u32>,

    /// Output in JSON format
    #[arg(long, not_supported(bun))]
    pub(crate) json: bool,

    /// Show extended information
    #[arg(long, not_supported(yarn, bun))]
    pub(crate) long: bool,

    /// Parseable output format
    #[arg(long, not_supported(yarn, bun))]
    pub(crate) parseable: bool,

    /// Only production dependencies
    #[arg(short = 'P', long, not_supported(yarn, bun))]
    pub(crate) prod: bool,

    /// Only dev dependencies
    #[arg(short = 'D', long, not_supported(yarn, bun))]
    pub(crate) dev: bool,

    /// Exclude optional dependencies
    #[arg(long, not_supported(yarn, bun))]
    pub(crate) no_optional: bool,

    /// Exclude peer dependencies
    #[arg(long, not_supported(yarn, bun))]
    pub(crate) exclude_peers: bool,

    /// Show only project packages
    #[arg(long, not_supported(npm, yarn, bun))]
    pub(crate) only_projects: bool,

    /// Use a finder function
    #[arg(long, value_name = "FINDER_NAME", not_supported(npm, yarn, bun))]
    pub(crate) find_by: Option<String>,

    /// List across all workspaces
    #[arg(short = 'r', long, not_supported(yarn, bun))]
    pub(crate) recursive: bool,

    /// Filter packages in monorepo
    #[arg(long, value_name = "PATTERN", not_supported(yarn, bun))]
    pub(crate) filter: Vec<String>,

    /// List global packages
    #[arg(short = 'g', long)]
    pub(crate) global: bool,

    /// Additional arguments
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) pass_through_args: Vec<String>,
}

impl Resolve<ListArgs> for Pnpm {
    fn resolve(&self, args: &ListArgs, _diag: &mut Diagnostics) -> CommandResolution {
        if args.global {
            return Npm::resolve_list(args);
        }

        let mut cmd = CommandBuilder::new("pnpm");
        cmd.repeated("--filter", args.filter.iter()).arg("list");
        if let Some(pattern) = &args.pattern {
            cmd.arg(pattern);
        }
        cmd.option("--depth", args.depth).arg_if("--json", args.json);
        cmd.arg_if("--long", args.long)
            .arg_if("--parseable", args.parseable)
            .arg_if("--prod", args.prod)
            .arg_if("--dev", args.dev)
            .arg_if("--no-optional", args.no_optional)
            .arg_if("--exclude-peers", args.exclude_peers)
            .arg_if("--only-projects", args.only_projects)
            .option("--find-by", args.find_by.as_ref())
            .arg_if("--recursive", args.recursive)
            .extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Npm {
    fn resolve_list(args: &ListArgs) -> CommandResolution {
        let mut cmd = CommandBuilder::new("npm");
        cmd.arg("list");
        if let Some(pattern) = &args.pattern {
            cmd.arg(pattern);
        }
        cmd.option("--depth", args.depth).arg_if("--json", args.json);
        cmd.arg_if("--long", args.long).arg_if("--parseable", args.parseable);
        if args.prod {
            cmd.arg("--include").arg("prod").arg("--include").arg("peer");
        }
        if args.dev {
            cmd.arg("--include").arg("dev");
        }
        if args.no_optional {
            cmd.arg("--omit").arg("optional");
        }
        if args.exclude_peers {
            cmd.arg("--omit").arg("peer");
        }
        cmd.arg_if("--workspaces", args.recursive).repeated("--workspace", args.filter.iter());
        if args.global {
            cmd.arg("-g");
        }
        cmd.extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<ListArgs> for Npm {
    fn resolve(&self, args: &ListArgs, _diag: &mut Diagnostics) -> CommandResolution {
        Self::resolve_list(args)
    }
}

impl Resolve<ListArgs> for Yarn {
    fn resolve(&self, args: &ListArgs, diag: &mut Diagnostics) -> CommandResolution {
        if self.is_berry() {
            diag.warn(
                DiagnosticKind::UnsupportedCommandNoop,
                "yarn@2+ does not support 'list' command",
            );
            return CommandResolution::Noop;
        }

        if args.global {
            return Npm::resolve_list(args);
        }

        let mut cmd = CommandBuilder::new("yarn");
        cmd.arg("list");
        if let Some(pattern) = &args.pattern {
            cmd.arg(pattern);
        }
        cmd.option("--depth", args.depth).arg_if("--json", args.json);
        cmd.extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<ListArgs> for Bun {
    fn resolve(&self, args: &ListArgs, _diag: &mut Diagnostics) -> CommandResolution {
        if args.global {
            return Npm::resolve_list(args);
        }

        let mut cmd = CommandBuilder::new("bun");
        cmd.arg("pm").arg("ls");
        if let Some(pattern) = &args.pattern {
            cmd.arg(pattern);
        }
        cmd.extend(args.pass_through_args.iter());
        cmd.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        resolve,
        test_utils::{bun, npm, parse_args, pnpm, yarn},
    };

    fn list_args(pattern: Option<&str>) -> ListArgs {
        ListArgs { pattern: pattern.map(ToString::to_string), ..Default::default() }
    }

    #[test]
    fn test_parser_accepts_short_dev_and_global_flags() {
        let args = parse_args::<ListArgs>(["react", "-D", "-g"]).unwrap();

        assert_eq!(args.pattern, Some("react".to_string()));
        assert!(args.dev);
        assert!(args.global);
    }

    #[test]
    fn test_pnpm_list_basic() {
        let CommandResolution::Run(command) = resolve(&pnpm("10.0.0"), ListArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["list"]);
    }

    #[test]
    fn test_pnpm_list_recursive() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), ListArgs { recursive: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["list", "--recursive"]);
    }

    #[test]
    fn test_npm_list_basic() {
        let CommandResolution::Run(command) = resolve(&npm("11.0.0"), ListArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["list"]);
    }

    #[test]
    fn test_npm_list_recursive() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), ListArgs { recursive: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["list", "--workspaces"]);
    }

    #[test]
    fn test_yarn1_list_basic() {
        let CommandResolution::Run(command) = resolve(&yarn("1.22.0"), ListArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["list"]);
    }

    #[test]
    fn test_yarn1_list_recursive_ignored() {
        let resolution =
            resolve(&yarn("1.22.0"), ListArgs { recursive: true, ..Default::default() });
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["list"]);
        assert_eq!(resolution.diagnostics[0].kind, DiagnosticKind::UnsupportedOptionDropped);
        assert_eq!(resolution.diagnostics[0].message, "yarn does not support --recursive.");
    }

    #[test]
    fn test_yarn2_list_not_supported() {
        let resolution = resolve(&yarn("4.0.0"), ListArgs::default());

        assert_eq!(resolution.outcome, CommandResolution::Noop);
        assert_eq!(resolution.diagnostics[0].kind, DiagnosticKind::UnsupportedCommandNoop);
        assert_eq!(resolution.diagnostics[0].message, "yarn@2+ does not support 'list' command");
    }

    #[test]
    fn test_pnpm_list_global_uses_npm() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), ListArgs { global: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["list", "-g"]);
    }

    #[test]
    fn test_npm_list_global() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), ListArgs { global: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["list", "-g"]);
    }

    #[test]
    fn test_yarn1_list_global_uses_npm() {
        let CommandResolution::Run(command) =
            resolve(&yarn("1.22.0"), ListArgs { global: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["list", "-g"]);
    }

    #[test]
    fn test_yarn2_list_global_is_not_supported() {
        let resolution = resolve(&yarn("4.0.0"), ListArgs { global: true, ..Default::default() });

        assert_eq!(resolution.outcome, CommandResolution::Noop);
    }

    #[test]
    fn test_bun_list_global_uses_npm() {
        let CommandResolution::Run(command) =
            resolve(&bun("1.3.11"), ListArgs { global: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["list", "-g"]);
    }

    #[test]
    fn test_global_list_with_depth() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.0.0"),
            ListArgs { global: true, depth: Some(0), ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["list", "--depth", "0", "-g"]);
    }

    #[test]
    fn test_pnpm_list_with_filter() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.0.0"),
            ListArgs { filter: vec!["app".to_string()], ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["--filter", "app", "list"]);
    }

    #[test]
    fn test_pnpm_list_with_multiple_filters() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.0.0"),
            ListArgs { filter: vec!["app".to_string(), "web".to_string()], ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["--filter", "app", "--filter", "web", "list"]);
    }

    #[test]
    fn test_npm_list_with_filter() {
        let CommandResolution::Run(command) = resolve(
            &npm("11.0.0"),
            ListArgs { filter: vec!["app".to_string()], ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["list", "--workspace", "app"]);
    }

    #[test]
    fn test_yarn1_list_with_filter_ignored() {
        let resolution = resolve(
            &yarn("1.22.0"),
            ListArgs { filter: vec!["app".to_string()], ..Default::default() },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["list"]);
        assert_eq!(resolution.diagnostics[0].message, "yarn does not support --filter.");
    }

    #[test]
    fn test_pnpm_list_prod() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), ListArgs { prod: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["list", "--prod"]);
    }

    #[test]
    fn test_npm_list_prod() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), ListArgs { prod: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["list", "--include", "prod", "--include", "peer"]);
    }

    #[test]
    fn test_yarn1_list_prod_ignored() {
        let CommandResolution::Run(command) =
            resolve(&yarn("1.22.0"), ListArgs { prod: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["list"]);
    }

    #[test]
    fn test_pnpm_list_dev() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), ListArgs { dev: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["list", "--dev"]);
    }

    #[test]
    fn test_npm_list_dev() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), ListArgs { dev: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["list", "--include", "dev"]);
    }

    #[test]
    fn test_yarn1_list_dev_ignored() {
        let CommandResolution::Run(command) =
            resolve(&yarn("1.22.0"), ListArgs { dev: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["list"]);
    }

    #[test]
    fn test_pnpm_list_no_optional() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), ListArgs { no_optional: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["list", "--no-optional"]);
    }

    #[test]
    fn test_npm_list_no_optional() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), ListArgs { no_optional: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["list", "--omit", "optional"]);
    }

    #[test]
    fn test_yarn1_list_no_optional_ignored() {
        let CommandResolution::Run(command) =
            resolve(&yarn("1.22.0"), ListArgs { no_optional: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["list"]);
    }

    #[test]
    fn test_pnpm_list_only_projects() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), ListArgs { only_projects: true, ..Default::default() })
                .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["list", "--only-projects"]);
    }

    #[test]
    fn test_npm_list_only_projects_ignored() {
        let resolution =
            resolve(&npm("11.0.0"), ListArgs { only_projects: true, ..Default::default() });
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["list"]);
        assert_eq!(resolution.diagnostics[0].message, "npm does not support --only-projects.");
    }

    #[test]
    fn test_yarn1_list_only_projects_ignored() {
        let CommandResolution::Run(command) =
            resolve(&yarn("1.22.0"), ListArgs { only_projects: true, ..Default::default() })
                .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["list"]);
    }

    #[test]
    fn test_pnpm_list_exclude_peers() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), ListArgs { exclude_peers: true, ..Default::default() })
                .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["list", "--exclude-peers"]);
    }

    #[test]
    fn test_npm_list_exclude_peers() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), ListArgs { exclude_peers: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["list", "--omit", "peer"]);
    }

    #[test]
    fn test_yarn1_list_exclude_peers_ignored() {
        let CommandResolution::Run(command) =
            resolve(&yarn("1.22.0"), ListArgs { exclude_peers: true, ..Default::default() })
                .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["list"]);
    }

    #[test]
    fn test_pnpm_list_find_by() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.0.0"),
            ListArgs { find_by: Some("customFinder".to_string()), ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["list", "--find-by", "customFinder"]);
    }

    #[test]
    fn test_npm_list_find_by_ignored() {
        let resolution = resolve(
            &npm("11.0.0"),
            ListArgs { find_by: Some("customFinder".to_string()), ..Default::default() },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["list"]);
        assert_eq!(resolution.diagnostics[0].message, "npm does not support --find-by.");
    }

    #[test]
    fn test_yarn1_list_find_by_ignored() {
        let CommandResolution::Run(command) = resolve(
            &yarn("1.22.0"),
            ListArgs { find_by: Some("customFinder".to_string()), ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["list"]);
    }

    #[test]
    fn test_bun_list_basic() {
        let CommandResolution::Run(command) = resolve(&bun("1.3.11"), ListArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["pm", "ls"]);
    }

    #[test]
    fn test_bun_list_unsupported_flags_warn_and_drop() {
        let resolution = resolve(
            &bun("1.3.11"),
            ListArgs {
                depth: Some(1),
                json: true,
                long: true,
                parseable: true,
                prod: true,
                dev: true,
                no_optional: true,
                exclude_peers: true,
                only_projects: true,
                find_by: Some("customFinder".to_string()),
                recursive: true,
                filter: vec!["app".to_string()],
                ..Default::default()
            },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["pm", "ls"]);
        assert_eq!(resolution.diagnostics.len(), 12);
        assert_eq!(resolution.diagnostics[0].message, "bun does not support --depth.");
        assert_eq!(resolution.diagnostics[11].message, "bun does not support --filter.");
    }

    #[test]
    fn test_list_with_pattern_and_pass_through_args() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.0.0"),
            ListArgs {
                pattern: Some("react".to_string()),
                pass_through_args: vec![
                    "--registry".to_string(),
                    "https://registry.npmjs.org".to_string(),
                ],
                ..Default::default()
            },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["list", "react", "--registry", "https://registry.npmjs.org"]);
    }

    #[test]
    fn test_list_with_long_parseable_and_json() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.0.0"),
            ListArgs { json: true, long: true, parseable: true, ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["list", "--json", "--long", "--parseable"]);
    }

    #[test]
    fn list_args_helper_keeps_pattern() {
        assert_eq!(list_args(Some("react")).pattern, Some("react".to_string()));
        assert_eq!(list_args(None).pattern, None);
    }
}
