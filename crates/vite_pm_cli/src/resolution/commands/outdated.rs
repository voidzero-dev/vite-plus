use std::str::FromStr;

use cow_utils::CowUtils as _;
use vite_pm_cli_macros::pm_args;

use super::parse_positive_usize;
use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, DiagnosticKind, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutdatedFormat {
    Table,
    List,
    Json,
}

impl OutdatedFormat {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Table => "table",
            Self::List => "list",
            Self::Json => "json",
        }
    }
}

impl FromStr for OutdatedFormat {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.cow_to_lowercase().as_ref() {
            "table" => Ok(Self::Table),
            "list" => Ok(Self::List),
            "json" => Ok(Self::Json),
            _ => {
                Err(vite_str::format!("Invalid format '{value}'. Valid formats: table, list, json")
                    .to_string())
            }
        }
    }
}

impl std::fmt::Display for OutdatedFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct OutdatedArgs {
    /// Package name(s) to check
    pub(crate) packages: Vec<String>,

    /// Show extended information
    #[arg(long, not_supported(yarn, bun))]
    pub(crate) long: bool,

    /// Output format: table (default), list, or json
    #[arg(long, value_name = "FORMAT", value_parser = clap::value_parser!(OutdatedFormat))]
    pub(crate) format: Option<OutdatedFormat>,

    /// Check recursively across all workspaces
    #[arg(short = 'r', long, not_supported(yarn))]
    pub(crate) recursive: bool,

    /// Filter packages in monorepo
    #[arg(long, value_name = "PATTERN", not_supported(yarn))]
    pub(crate) filter: Vec<String>,

    /// Include workspace root
    #[arg(short = 'w', long, not_supported(yarn, bun))]
    pub(crate) workspace_root: bool,

    /// Only production and optional dependencies
    #[arg(short = 'P', long, not_supported(npm, yarn))]
    pub(crate) prod: bool,

    /// Only dev dependencies
    #[arg(short = 'D', long, not_supported(npm, yarn, bun))]
    pub(crate) dev: bool,

    /// Exclude optional dependencies
    #[arg(long, not_supported(npm, yarn))]
    pub(crate) no_optional: bool,

    /// Only show compatible versions
    #[arg(long, not_supported(npm, yarn, bun))]
    pub(crate) compatible: bool,

    /// Sort results by field
    #[arg(long, value_name = "FIELD", not_supported(npm, yarn, bun))]
    pub(crate) sort_by: Option<String>,

    /// Check globally installed packages
    #[arg(short = 'g', long)]
    pub(crate) global: bool,

    /// Number of global package checks to run in parallel (only with -g)
    #[arg(long, requires = "global", value_parser = parse_positive_usize)]
    pub(crate) concurrency: Option<usize>,

    /// Additional arguments to pass through to the package manager
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) pass_through_args: Vec<String>,
}

impl Resolve<OutdatedArgs> for Pnpm {
    fn resolve(&self, args: &OutdatedArgs, _diag: &mut Diagnostics) -> CommandResolution {
        if args.global {
            return Npm::resolve_outdated(args);
        }

        let mut cmd = CommandBuilder::new("pnpm");
        cmd.repeated("--filter", args.filter.iter()).arg("outdated");
        if let Some(format) = args.format {
            cmd.arg("--format").arg(format.as_str());
        }
        cmd.arg_if("--long", args.long)
            .arg_if("--workspace-root", args.workspace_root)
            .arg_if("--recursive", args.recursive)
            .arg_if("--prod", args.prod)
            .arg_if("--dev", args.dev)
            .arg_if("--no-optional", args.no_optional)
            .arg_if("--compatible", args.compatible)
            .option("--sort-by", args.sort_by.as_ref())
            .extend(args.packages.iter())
            .extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Npm {
    fn resolve_outdated(args: &OutdatedArgs) -> CommandResolution {
        let mut cmd = CommandBuilder::new("npm");
        cmd.arg("outdated");
        match args.format {
            Some(OutdatedFormat::Json) => {
                cmd.arg("--json");
            }
            Some(OutdatedFormat::List) => {
                cmd.arg("--parseable");
            }
            Some(OutdatedFormat::Table) | None => {}
        }
        cmd.arg_if("--long", args.long)
            .repeated("--workspace", args.filter.iter())
            .arg_if("--include-workspace-root", args.workspace_root)
            .arg_if("--all", args.recursive)
            .extend(args.packages.iter());
        cmd.extend(args.pass_through_args.iter());
        if args.global {
            cmd.arg("-g");
        }
        cmd.into()
    }
}

impl Resolve<OutdatedArgs> for Npm {
    fn resolve(&self, args: &OutdatedArgs, _diag: &mut Diagnostics) -> CommandResolution {
        Self::resolve_outdated(args)
    }
}

impl Resolve<OutdatedArgs> for Yarn {
    fn resolve(&self, args: &OutdatedArgs, diag: &mut Diagnostics) -> CommandResolution {
        if args.global {
            return Npm::resolve_outdated(args);
        }

        if self.is_berry() {
            if args.format.is_some() {
                diag.warn(
                    DiagnosticKind::UnsupportedOptionDropped,
                    "--format not supported by yarn@2+",
                );
            }
            diag.note(
                DiagnosticKind::BehaviorChange,
                "yarn@2+ uses 'yarn upgrade-interactive' for checking outdated packages",
            );
            let mut cmd = CommandBuilder::new("yarn");
            cmd.arg("upgrade-interactive").extend(args.pass_through_args.iter());
            return cmd.into();
        }

        let mut cmd = CommandBuilder::new("yarn");
        cmd.arg("outdated").extend(args.packages.iter());
        match args.format {
            Some(OutdatedFormat::Json) => {
                cmd.arg("--json");
            }
            Some(OutdatedFormat::List) => {
                diag.warn(
                    DiagnosticKind::UnsupportedOptionDropped,
                    "yarn@1 not support list format",
                );
            }
            Some(OutdatedFormat::Table) | None => {}
        }
        cmd.extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<OutdatedArgs> for Bun {
    fn resolve(&self, args: &OutdatedArgs, diag: &mut Diagnostics) -> CommandResolution {
        if args.global {
            return Npm::resolve_outdated(args);
        }

        let mut cmd = CommandBuilder::new("bun");
        cmd.arg("outdated")
            .repeated("--filter", args.filter.iter())
            .arg_if("--recursive", args.recursive)
            .extend(args.packages.iter());
        if args.format == Some(OutdatedFormat::Json) {
            diag.warn(
                DiagnosticKind::UnsupportedOptionDropped,
                "bun outdated does not support --format json",
            );
        }
        cmd.arg_if("--production", args.prod);
        if args.no_optional {
            cmd.arg("--omit").arg("optional");
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

    fn outdated_args(packages: &[&str]) -> OutdatedArgs {
        OutdatedArgs {
            packages: packages.iter().map(ToString::to_string).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn format_parser_accepts_known_values() {
        let args = parse_args::<OutdatedArgs>(["--format", "json"]).unwrap();

        assert_eq!(args.format, Some(OutdatedFormat::Json));
    }

    #[test]
    fn format_parser_rejects_unknown_value() {
        let error = parse_args::<OutdatedArgs>(["--format", "yaml"]).unwrap_err();

        assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
    }

    #[test]
    fn concurrency_requires_global() {
        let error = parse_args::<OutdatedArgs>(["--concurrency", "2"]).unwrap_err();

        assert_eq!(error.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn concurrency_rejects_zero() {
        let error = parse_args::<OutdatedArgs>(["-g", "--concurrency", "0"]).unwrap_err();

        assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
    }

    #[test]
    fn test_pnpm_outdated_basic() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), OutdatedArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["outdated"]);
    }

    #[test]
    fn test_pnpm_outdated_with_packages() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), outdated_args(&["*babel*", "eslint-*"])).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["outdated", "*babel*", "eslint-*"]);
    }

    #[test]
    fn test_pnpm_outdated_json() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.0.0"),
            OutdatedArgs { format: Some(OutdatedFormat::Json), ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["outdated", "--format", "json"]);
    }

    #[test]
    fn test_npm_outdated_basic() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), OutdatedArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["outdated"]);
    }

    #[test]
    fn test_npm_outdated_json() {
        let CommandResolution::Run(command) = resolve(
            &npm("11.0.0"),
            OutdatedArgs { format: Some(OutdatedFormat::Json), ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["outdated", "--json"]);
    }

    #[test]
    fn test_yarn_outdated_basic() {
        let CommandResolution::Run(command) =
            resolve(&yarn("1.22.19"), OutdatedArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["outdated"]);
    }

    #[test]
    fn test_pnpm_outdated_with_filter() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.0.0"),
            OutdatedArgs { filter: vec!["app".to_string()], recursive: true, ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["--filter", "app", "outdated", "--recursive"]);
    }

    #[test]
    fn test_pnpm_outdated_prod_only() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), OutdatedArgs { prod: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["outdated", "--prod"]);
    }

    #[test]
    fn test_npm_outdated_list_format() {
        let CommandResolution::Run(command) = resolve(
            &npm("11.0.0"),
            OutdatedArgs { format: Some(OutdatedFormat::List), ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["outdated", "--parseable"]);
    }

    #[test]
    fn test_npm_outdated_recursive() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), OutdatedArgs { recursive: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["outdated", "--all"]);
    }

    #[test]
    fn test_npm_outdated_with_workspace() {
        let CommandResolution::Run(command) = resolve(
            &npm("11.0.0"),
            OutdatedArgs { filter: vec!["app".to_string()], ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["outdated", "--workspace", "app"]);
    }

    #[test]
    fn test_global_outdated() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), OutdatedArgs { global: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["outdated", "-g"]);
    }

    #[test]
    fn global_outdated_uses_npm_lowering_after_current_dialect_support_checks() {
        let args = OutdatedArgs {
            packages: vec!["react".to_string()],
            long: true,
            format: Some(OutdatedFormat::List),
            recursive: true,
            filter: vec!["app".to_string()],
            workspace_root: true,
            global: true,
            ..Default::default()
        };

        let yarn_resolution = resolve(&yarn("1.22.19"), args.clone());
        let CommandResolution::Run(yarn_command) = yarn_resolution.outcome else {
            panic!("expected command resolution");
        };
        assert_eq!(yarn_command.program, "npm");
        assert_eq!(yarn_command.args, vec!["outdated", "--parseable", "react", "-g"]);
        assert_eq!(yarn_resolution.diagnostics.len(), 4);

        let bun_resolution = resolve(&bun("1.3.11"), args);
        let CommandResolution::Run(bun_command) = bun_resolution.outcome else {
            panic!("expected command resolution");
        };
        assert_eq!(bun_command.program, "npm");
        assert_eq!(
            bun_command.args,
            vec!["outdated", "--parseable", "--workspace", "app", "--all", "react", "-g"]
        );
        assert_eq!(bun_resolution.diagnostics.len(), 2);
    }

    #[test]
    fn test_pnpm_outdated_with_workspace_root() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), OutdatedArgs { workspace_root: true, ..Default::default() })
                .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["outdated", "--workspace-root"]);
    }

    #[test]
    fn test_pnpm_outdated_with_workspace_root_and_recursive() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.0.0"),
            OutdatedArgs { workspace_root: true, recursive: true, ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["outdated", "--workspace-root", "--recursive"]);
    }

    #[test]
    fn test_pnpm_outdated_with_all_flags() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.0.0"),
            OutdatedArgs {
                packages: vec!["react".to_string()],
                long: true,
                format: Some(OutdatedFormat::Json),
                recursive: true,
                filter: vec!["app".to_string()],
                workspace_root: true,
                prod: true,
                compatible: true,
                sort_by: Some("name".to_string()),
                ..Default::default()
            },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(
            command.args,
            vec![
                "--filter",
                "app",
                "outdated",
                "--format",
                "json",
                "--long",
                "--workspace-root",
                "--recursive",
                "--prod",
                "--compatible",
                "--sort-by",
                "name",
                "react"
            ]
        );
    }

    #[test]
    fn test_npm_outdated_with_workspace_root() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), OutdatedArgs { workspace_root: true, ..Default::default() })
                .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["outdated", "--include-workspace-root"]);
    }

    #[test]
    fn test_npm_outdated_with_workspace_root_and_workspace() {
        let CommandResolution::Run(command) = resolve(
            &npm("11.0.0"),
            OutdatedArgs {
                filter: vec!["app".to_string()],
                workspace_root: true,
                ..Default::default()
            },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(
            command.args,
            vec!["outdated", "--workspace", "app", "--include-workspace-root"]
        );
    }

    #[test]
    fn yarn_classic_supports_json_and_warns_list_format() {
        let list_resolution = resolve(
            &yarn("1.22.19"),
            OutdatedArgs { format: Some(OutdatedFormat::List), ..Default::default() },
        );
        let CommandResolution::Run(list_command) = list_resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(list_command.program, "yarn");
        assert_eq!(list_command.args, vec!["outdated"]);
        assert_eq!(list_resolution.diagnostics[0].message, "yarn@1 not support list format");

        let json_resolution = resolve(
            &yarn("1.22.19"),
            OutdatedArgs { format: Some(OutdatedFormat::Json), ..Default::default() },
        );
        let CommandResolution::Run(json_command) = json_resolution.outcome else {
            panic!("expected command resolution");
        };
        assert_eq!(json_command.args, vec!["outdated", "--json"]);
        assert!(json_resolution.diagnostics.is_empty());
    }

    #[test]
    fn yarn_berry_uses_upgrade_interactive_and_warns_format() {
        let resolution = resolve(
            &yarn("4.0.0"),
            OutdatedArgs { format: Some(OutdatedFormat::Json), ..Default::default() },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["upgrade-interactive"]);
        assert_eq!(resolution.diagnostics[0].message, "--format not supported by yarn@2+");
        assert_eq!(
            resolution.diagnostics[1].message,
            "yarn@2+ uses 'yarn upgrade-interactive' for checking outdated packages"
        );
    }

    #[test]
    fn bun_outdated_supports_subset_and_warns_json_format() {
        let resolution = resolve(
            &bun("1.3.11"),
            OutdatedArgs {
                packages: vec!["react".to_string()],
                format: Some(OutdatedFormat::Json),
                filter: vec!["app".to_string()],
                recursive: true,
                prod: true,
                no_optional: true,
                ..Default::default()
            },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(
            command.args,
            vec![
                "outdated",
                "--filter",
                "app",
                "--recursive",
                "react",
                "--production",
                "--omit",
                "optional"
            ]
        );
        assert_eq!(
            resolution.diagnostics[0].message,
            "bun outdated does not support --format json"
        );
    }

    #[test]
    fn unsupported_fields_are_dropped_for_yarn_and_bun() {
        let yarn_resolution = resolve(
            &yarn("1.22.19"),
            OutdatedArgs {
                long: true,
                recursive: true,
                filter: vec!["app".to_string()],
                workspace_root: true,
                prod: true,
                dev: true,
                no_optional: true,
                compatible: true,
                sort_by: Some("name".to_string()),
                ..Default::default()
            },
        );
        let CommandResolution::Run(yarn_command) = yarn_resolution.outcome else {
            panic!("expected command resolution");
        };
        assert_eq!(yarn_command.args, vec!["outdated"]);
        assert_eq!(yarn_resolution.diagnostics.len(), 9);

        let bun_resolution = resolve(
            &bun("1.3.11"),
            OutdatedArgs {
                long: true,
                workspace_root: true,
                dev: true,
                compatible: true,
                sort_by: Some("name".to_string()),
                ..Default::default()
            },
        );
        let CommandResolution::Run(bun_command) = bun_resolution.outcome else {
            panic!("expected command resolution");
        };
        assert_eq!(bun_command.args, vec!["outdated"]);
        assert_eq!(bun_resolution.diagnostics.len(), 5);
    }
}
