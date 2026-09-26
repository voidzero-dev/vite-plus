use std::str::FromStr;

use cow_utils::CowUtils as _;
use vp_pm_cli_macros::pm_args;

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
            _ => Err(vt_str::format!("Invalid format '{value}'. Valid formats: table, list, json")
                .to_string()),
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
    #[arg(long, value_name = "FORMAT", value_parser = clap::value_parser!(OutdatedFormat), not_supported(yarn >= "2", bun))]
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
    #[arg(short = 'P', long, not_supported(yarn))]
    pub(crate) prod: bool,

    /// Include dev dependencies
    #[arg(short = 'D', long, not_supported(yarn, bun))]
    pub(crate) dev: bool,

    /// Exclude optional dependencies
    #[arg(long, not_supported(yarn))]
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
        cmd.arg_if("--omit=dev", args.prod)
            .arg_if("--include=dev", args.dev)
            .arg_if("--omit=optional", args.no_optional)
            .extend(args.pass_through_args.iter());
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
                return CommandResolution::InvalidArgument(
                    "Yarn Classic does not support --format list.".into(),
                );
            }
            Some(OutdatedFormat::Table) | None => {}
        }
        cmd.extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<OutdatedArgs> for Bun {
    fn resolve(&self, args: &OutdatedArgs, _diag: &mut Diagnostics) -> CommandResolution {
        if args.global {
            return Npm::resolve_outdated(args);
        }

        let mut cmd = CommandBuilder::new("bun");
        cmd.arg("outdated")
            .repeated("--filter", args.filter.iter())
            .arg_if("--recursive", args.recursive)
            .extend(args.packages.iter());
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
        test_utils::{bun, expect_run, expect_unsupported, npm, parse_args, pnpm, yarn},
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
        let command = expect_run(resolve(&pnpm("10.0.0"), OutdatedArgs::default()).outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["outdated"]);
    }

    #[test]
    fn test_pnpm_outdated_with_packages() {
        let command =
            expect_run(resolve(&pnpm("10.0.0"), outdated_args(&["*babel*", "eslint-*"])).outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["outdated", "*babel*", "eslint-*"]);
    }

    #[test]
    fn test_pnpm_outdated_json() {
        let command = expect_run(
            resolve(
                &pnpm("10.0.0"),
                OutdatedArgs { format: Some(OutdatedFormat::Json), ..Default::default() },
            )
            .outcome,
        );

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["outdated", "--format", "json"]);
    }

    #[test]
    fn test_npm_outdated_basic() {
        let command = expect_run(resolve(&npm("11.0.0"), OutdatedArgs::default()).outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["outdated"]);
    }

    #[test]
    fn test_npm_outdated_json() {
        let command = expect_run(
            resolve(
                &npm("11.0.0"),
                OutdatedArgs { format: Some(OutdatedFormat::Json), ..Default::default() },
            )
            .outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["outdated", "--json"]);
    }

    #[test]
    fn test_yarn_outdated_basic() {
        let command = expect_run(resolve(&yarn("1.22.19"), OutdatedArgs::default()).outcome);

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["outdated"]);
    }

    #[test]
    fn test_pnpm_outdated_with_filter() {
        let command = expect_run(
            resolve(
                &pnpm("10.0.0"),
                OutdatedArgs {
                    filter: vec!["app".to_string()],
                    recursive: true,
                    ..Default::default()
                },
            )
            .outcome,
        );

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["--filter", "app", "outdated", "--recursive"]);
    }

    #[test]
    fn test_pnpm_outdated_prod_only() {
        let command = expect_run(
            resolve(&pnpm("10.0.0"), OutdatedArgs { prod: true, ..Default::default() }).outcome,
        );

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["outdated", "--prod"]);
    }

    #[test]
    fn test_npm_outdated_list_format() {
        let command = expect_run(
            resolve(
                &npm("11.0.0"),
                OutdatedArgs { format: Some(OutdatedFormat::List), ..Default::default() },
            )
            .outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["outdated", "--parseable"]);
    }

    #[test]
    fn test_npm_outdated_recursive() {
        let command = expect_run(
            resolve(&npm("11.0.0"), OutdatedArgs { recursive: true, ..Default::default() }).outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["outdated", "--all"]);
    }

    #[test]
    fn test_npm_outdated_with_workspace() {
        let command = expect_run(
            resolve(
                &npm("11.0.0"),
                OutdatedArgs { filter: vec!["app".to_string()], ..Default::default() },
            )
            .outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["outdated", "--workspace", "app"]);
    }

    #[test]
    fn test_global_outdated() {
        let command = expect_run(
            resolve(&pnpm("10.0.0"), OutdatedArgs { global: true, ..Default::default() }).outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["outdated", "-g"]);
    }

    #[test]
    fn legacy_global_lowering_does_not_bypass_dialect_support_checks() {
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
        expect_unsupported(
            yarn_resolution,
            &[
                "yarn does not support --long.",
                "yarn does not support --recursive.",
                "yarn does not support --filter.",
                "yarn does not support --workspace-root.",
            ],
        );

        let bun_resolution = resolve(&bun("1.3.11"), args);
        expect_unsupported(
            bun_resolution,
            &[
                "bun does not support --long.",
                "bun does not support --format.",
                "bun does not support --workspace-root.",
            ],
        );
    }

    #[test]
    fn test_pnpm_outdated_with_workspace_root() {
        let command = expect_run(
            resolve(&pnpm("10.0.0"), OutdatedArgs { workspace_root: true, ..Default::default() })
                .outcome,
        );

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["outdated", "--workspace-root"]);
    }

    #[test]
    fn test_pnpm_outdated_with_workspace_root_and_recursive() {
        let command = expect_run(
            resolve(
                &pnpm("10.0.0"),
                OutdatedArgs { workspace_root: true, recursive: true, ..Default::default() },
            )
            .outcome,
        );

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["outdated", "--workspace-root", "--recursive"]);
    }

    #[test]
    fn test_pnpm_outdated_with_all_flags() {
        let command = expect_run(
            resolve(
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
            .outcome,
        );

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
        let command = expect_run(
            resolve(&npm("11.0.0"), OutdatedArgs { workspace_root: true, ..Default::default() })
                .outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["outdated", "--include-workspace-root"]);
    }

    #[test]
    fn test_npm_outdated_with_workspace_root_and_workspace() {
        let command = expect_run(
            resolve(
                &npm("11.0.0"),
                OutdatedArgs {
                    filter: vec!["app".to_string()],
                    workspace_root: true,
                    ..Default::default()
                },
            )
            .outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(
            command.args,
            vec!["outdated", "--workspace", "app", "--include-workspace-root"]
        );
    }

    #[test]
    fn yarn_classic_rejects_only_list_format() {
        for version in ["1.22.19", "1.22.22"] {
            let args = parse_args::<OutdatedArgs>(["--format", "list"]).unwrap();
            expect_unsupported(
                resolve(&yarn(version), args),
                &["Yarn Classic does not support --format list."],
            );
        }
    }

    #[test]
    fn yarn_berry_rejects_explicit_formats() {
        for version in ["2.0.0", "2.4.2", "3.6.0", "4.10.3"] {
            for format in ["table", "list", "json"] {
                let args = parse_args::<OutdatedArgs>(["--format", format]).unwrap();
                expect_unsupported(
                    resolve(&yarn(version), args),
                    &["yarn >= 2 does not support --format."],
                );
            }
        }
    }

    #[test]
    fn yarn_berry_reports_format_with_other_unsupported_options() {
        let args = parse_args::<OutdatedArgs>(["--format", "json", "--long"]).unwrap();
        expect_unsupported(
            resolve(&yarn("4.10.3"), args),
            &["yarn does not support --long.", "yarn >= 2 does not support --format."],
        );
    }

    #[test]
    fn yarn_berry_uses_upgrade_interactive_without_format() {
        for version in ["2.0.0", "3.6.0", "4.10.3"] {
            let args = parse_args::<OutdatedArgs>(["--", "--help"]).unwrap();
            let resolution = resolve(&yarn(version), args);
            let command = expect_run(resolution.outcome);

            assert_eq!(command.program, "yarn");
            assert_eq!(command.args, vec!["upgrade-interactive", "--help"]);
            assert_eq!(resolution.diagnostics.len(), 1);
            assert_eq!(
                resolution.diagnostics[0].message,
                "yarn@2+ uses 'yarn upgrade-interactive' for checking outdated packages"
            );
        }
    }

    #[test]
    fn supported_formats_are_preserved_for_other_managers() {
        for format in [OutdatedFormat::Table, OutdatedFormat::List, OutdatedFormat::Json] {
            let args = OutdatedArgs { format: Some(format), ..Default::default() };
            let npm_resolution = resolve(&npm("12.0.2"), args.clone());
            let mut expected = vec!["outdated"];
            match format {
                OutdatedFormat::Json => expected.push("--json"),
                OutdatedFormat::List => expected.push("--parseable"),
                OutdatedFormat::Table => {}
            }
            assert_eq!(expect_run(npm_resolution.outcome).args, expected);
            assert!(npm_resolution.diagnostics.is_empty());

            let pnpm_resolution = resolve(&pnpm("11.3.0"), args.clone());
            assert_eq!(
                expect_run(pnpm_resolution.outcome).args,
                vec!["outdated", "--format", format.as_str()]
            );
            assert!(pnpm_resolution.diagnostics.is_empty());

            if format != OutdatedFormat::List {
                let yarn_resolution = resolve(&yarn("1.22.22"), args);
                assert_eq!(expect_run(yarn_resolution.outcome).args, expected);
                assert!(yarn_resolution.diagnostics.is_empty());
            }
        }
    }

    #[test]
    fn bun_outdated_preserves_default_and_supported_flags() {
        let resolution = resolve(
            &bun("1.3.11"),
            OutdatedArgs {
                packages: vec!["react".to_string()],
                filter: vec!["app".to_string()],
                recursive: true,
                prod: true,
                no_optional: true,
                ..Default::default()
            },
        );
        let command = expect_run(resolution.outcome);

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
        assert!(resolution.diagnostics.is_empty());
    }

    #[test]
    fn bun_outdated_rejects_explicit_formats() {
        for version in ["1.3.11", "1.3.14", "1.4.0"] {
            for format in ["table", "json", "list"] {
                let args = parse_args::<OutdatedArgs>(["--format", format]).unwrap();
                expect_unsupported(
                    resolve(&bun(version), args),
                    &["bun does not support --format."],
                );
            }
        }
    }

    #[test]
    fn bun_outdated_reports_formats_with_other_unsupported_options() {
        for format in ["table", "json", "list"] {
            let args = parse_args::<OutdatedArgs>(["--format", format, "--long"]).unwrap();
            expect_unsupported(
                resolve(&bun("1.4.0"), args),
                &["bun does not support --long.", "bun does not support --format."],
            );
        }
    }

    #[test]
    fn bun_outdated_preserves_raw_args_without_named_format() {
        for raw_args in [vec!["--json"], vec!["--format", "list"]] {
            let args =
                parse_args::<OutdatedArgs>(std::iter::once("--").chain(raw_args.clone())).unwrap();
            let resolution = resolve(&bun("1.4.0"), args);
            assert_eq!(expect_run(resolution.outcome).args, [vec!["outdated"], raw_args].concat());
            assert!(resolution.diagnostics.is_empty());
        }
    }

    #[test]
    fn npm_outdated_maps_dependency_type_filters() {
        for (args, expected) in [
            (
                OutdatedArgs { prod: true, no_optional: true, ..Default::default() },
                vec!["outdated", "--omit=dev", "--omit=optional"],
            ),
            (OutdatedArgs { dev: true, ..Default::default() }, vec!["outdated", "--include=dev"]),
        ] {
            let resolution = resolve(&npm("11.16.0"), args);
            assert!(resolution.diagnostics.is_empty());
            assert_eq!(expect_run(resolution.outcome).args, expected);
        }
    }

    #[test]
    fn unsupported_fields_are_rejected_for_yarn_and_bun() {
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
        expect_unsupported(
            yarn_resolution,
            &[
                "yarn does not support --long.",
                "yarn does not support --recursive.",
                "yarn does not support --filter.",
                "yarn does not support --workspace-root.",
                "yarn does not support --prod.",
                "yarn does not support --dev.",
                "yarn does not support --no-optional.",
                "yarn does not support --compatible.",
                "yarn does not support --sort-by.",
            ],
        );

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
        expect_unsupported(
            bun_resolution,
            &[
                "bun does not support --long.",
                "bun does not support --workspace-root.",
                "bun does not support --dev.",
                "bun does not support --compatible.",
                "bun does not support --sort-by.",
            ],
        );
    }
}
