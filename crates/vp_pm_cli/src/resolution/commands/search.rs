use vp_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchArgs {
    /// Search terms
    #[arg(required = true, num_args = 1..)]
    pub(crate) terms: Vec<String>,

    /// Output in JSON format
    #[arg(long)]
    pub(crate) json: bool,

    /// Show extended information
    #[arg(long, not_supported(pnpm >= "11"))]
    pub(crate) long: bool,

    /// Registry URL
    #[arg(long, value_name = "URL")]
    pub(crate) registry: Option<String>,

    /// Additional arguments
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) pass_through_args: Vec<String>,
}

impl Npm {
    fn resolve_search(args: &SearchArgs) -> CommandResolution {
        resolve_search("npm", args)
    }
}

fn resolve_search(program: &str, args: &SearchArgs) -> CommandResolution {
    let mut cmd = CommandBuilder::new(program);
    cmd.arg("search")
        .extend(args.terms.iter())
        .arg_if("--json", args.json)
        .arg_if("--long", args.long)
        .option("--registry", args.registry.as_ref())
        .extend(args.pass_through_args.iter());
    cmd.into()
}

impl Resolve<SearchArgs> for Pnpm {
    fn resolve(&self, args: &SearchArgs, _diag: &mut Diagnostics) -> CommandResolution {
        if self.supports_v11_commands() {
            resolve_search("pnpm", args)
        } else {
            Npm::resolve_search(args)
        }
    }
}

impl Resolve<SearchArgs> for Yarn {
    fn resolve(&self, args: &SearchArgs, _diag: &mut Diagnostics) -> CommandResolution {
        Npm::resolve_search(args)
    }
}

impl Resolve<SearchArgs> for Bun {
    fn resolve(&self, args: &SearchArgs, _diag: &mut Diagnostics) -> CommandResolution {
        Npm::resolve_search(args)
    }
}

impl Resolve<SearchArgs> for Npm {
    fn resolve(&self, args: &SearchArgs, _diag: &mut Diagnostics) -> CommandResolution {
        Self::resolve_search(args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        resolve,
        test_utils::{bun, expect_run, expect_unsupported, npm, parse_args, pnpm, yarn},
    };

    #[test]
    fn test_pnpm_native_version_boundary() {
        let args = parse_args::<SearchArgs>([
            "react",
            "hooks",
            "--json",
            "--registry",
            "https://registry.example.com",
            "--",
            "--help",
        ])
        .unwrap();
        for (version, program) in
            [("10.99.0", "npm"), ("11.0.0-rc.1", "npm"), ("11.0.0", "pnpm"), ("12.13.0", "pnpm")]
        {
            let resolution = resolve(&pnpm(version), args.clone());
            assert!(resolution.diagnostics.is_empty());
            let command = expect_run(resolution.outcome);
            assert_eq!(command.program, program, "{version}");
            assert_eq!(
                command.args,
                vec![
                    "search",
                    "react",
                    "hooks",
                    "--json",
                    "--registry",
                    "https://registry.example.com",
                    "--help"
                ]
            );
        }
    }

    #[test]
    fn test_pnpm_long_support_boundary() {
        let args = parse_args::<SearchArgs>(["react", "--long"]).unwrap();
        for version in ["10.99.0", "11.0.0-rc.1"] {
            let command = expect_run(resolve(&pnpm(version), args.clone()).outcome);
            assert_eq!(command.program, "npm");
            assert_eq!(command.args, vec!["search", "react", "--long"]);
        }
        for version in ["11.0.0", "12.13.0"] {
            expect_unsupported(
                resolve(&pnpm(version), args.clone()),
                &["pnpm >= 11 does not support --long."],
            );
            let raw = parse_args::<SearchArgs>(["react", "--", "--long"]).unwrap();
            let command = expect_run(resolve(&pnpm(version), raw).outcome);
            assert_eq!(command.program, "pnpm");
            assert_eq!(command.args, vec!["search", "react", "--long"]);
        }
    }

    #[test]
    fn test_parser_requires_terms_and_accepts_pass_through_args() {
        let args = parse_args::<SearchArgs>(["react", "--", "--searchlimit", "10"]).unwrap();

        assert_eq!(args.terms, vec!["react".to_string()]);
        assert_eq!(args.pass_through_args, vec!["--searchlimit".to_string(), "10".to_string()]);
    }

    #[test]
    fn test_search_basic() {
        let command = expect_run(
            resolve(
                &pnpm("10.0.0"),
                SearchArgs { terms: vec!["react".to_string()], ..Default::default() },
            )
            .outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["search", "react"]);
    }

    #[test]
    fn test_search_with_json() {
        let command = expect_run(
            resolve(
                &npm("11.0.0"),
                SearchArgs { terms: vec!["lodash".to_string()], json: true, ..Default::default() },
            )
            .outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["search", "lodash", "--json"]);
    }

    #[test]
    fn test_search_multiple_terms() {
        let command = expect_run(
            resolve(
                &yarn("4.0.0"),
                SearchArgs {
                    terms: vec!["react".to_string(), "hooks".to_string(), "state".to_string()],
                    long: true,
                    registry: Some("https://registry.npmjs.org".to_string()),
                    ..Default::default()
                },
            )
            .outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(
            command.args,
            vec![
                "search",
                "react",
                "hooks",
                "state",
                "--long",
                "--registry",
                "https://registry.npmjs.org",
            ]
        );
    }

    #[test]
    fn test_bun_search_uses_npm_without_warning() {
        let result = resolve(
            &bun("1.3.11"),
            SearchArgs { terms: vec!["react".to_string()], ..Default::default() },
        );
        let command = expect_run(result.outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["search", "react"]);
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn test_yarn_classic_search_uses_npm() {
        let command = expect_run(
            resolve(
                &yarn("1.22.0"),
                SearchArgs { terms: vec!["react".to_string()], ..Default::default() },
            )
            .outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["search", "react"]);
    }
}
