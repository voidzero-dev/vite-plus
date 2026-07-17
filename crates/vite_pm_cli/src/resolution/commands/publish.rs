use vite_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct PublishArgs {
    /// Tarball or folder to publish
    #[arg(value_name = "TARBALL|FOLDER")]
    pub(crate) target: Option<String>,

    /// Preview without publishing
    #[arg(long)]
    pub(crate) dry_run: bool,

    /// Publish tag
    #[arg(long)]
    pub(crate) tag: Option<String>,

    /// Access level (public/restricted)
    #[arg(long)]
    pub(crate) access: Option<String>,

    /// One-time password for authentication
    #[arg(long, value_name = "OTP")]
    pub(crate) otp: Option<String>,

    /// Skip git checks
    #[arg(long, not_supported(bun))]
    pub(crate) no_git_checks: bool,

    /// Set the branch name to publish from
    #[arg(long, value_name = "BRANCH", not_supported(npm, yarn, bun))]
    pub(crate) publish_branch: Option<String>,

    /// Save publish summary
    #[arg(long, not_supported(npm, yarn, bun))]
    pub(crate) report_summary: bool,

    /// Publish with provenance
    #[arg(long, not_supported(bun))]
    pub(crate) provenance: bool,

    /// Force publish
    #[arg(long, not_supported(bun))]
    pub(crate) force: bool,

    /// Output in JSON format
    #[arg(long, not_supported(npm, yarn, bun))]
    pub(crate) json: bool,

    /// Publish all workspace packages
    #[arg(short = 'r', long, not_supported(bun))]
    pub(crate) recursive: bool,

    /// Filter packages in monorepo
    #[arg(long, value_name = "PATTERN", not_supported(bun))]
    pub(crate) filter: Option<Vec<String>>,

    /// Additional arguments
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) pass_through_args: Vec<String>,
}

impl Resolve<PublishArgs> for Pnpm {
    fn resolve(&self, args: &PublishArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("pnpm");
        if let Some(filters) = &args.filter {
            cmd.repeated("--filter", filters.iter());
        }
        cmd.arg("publish");
        push_common_publish_args(&mut cmd, args);
        cmd.arg_if("--no-git-checks", args.no_git_checks)
            .option("--publish-branch", args.publish_branch.as_ref())
            .arg_if("--report-summary", args.report_summary)
            .arg_if("--provenance", args.provenance)
            .arg_if("--force", args.force)
            .arg_if("--json", args.json)
            .arg_if("--recursive", args.recursive)
            .extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Npm {
    fn resolve_publish(args: &PublishArgs) -> CommandResolution {
        let mut cmd = CommandBuilder::new("npm");
        cmd.arg("publish").arg_if("--workspaces", args.recursive);
        if let Some(filters) = &args.filter {
            cmd.repeated("--workspace", filters.iter());
        }
        push_common_publish_args(&mut cmd, args);
        cmd.arg_if("--provenance", args.provenance).arg_if("--force", args.force);
        cmd.extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<PublishArgs> for Npm {
    fn resolve(&self, args: &PublishArgs, _diag: &mut Diagnostics) -> CommandResolution {
        Self::resolve_publish(args)
    }
}

impl Resolve<PublishArgs> for Yarn {
    fn resolve(&self, args: &PublishArgs, _diag: &mut Diagnostics) -> CommandResolution {
        Npm::resolve_publish(args)
    }
}

impl Resolve<PublishArgs> for Bun {
    fn resolve(&self, args: &PublishArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("bun");
        cmd.arg("publish");
        push_common_publish_args(&mut cmd, args);
        cmd.extend(args.pass_through_args.iter());
        cmd.into()
    }
}

fn push_common_publish_args(cmd: &mut CommandBuilder, args: &PublishArgs) {
    if let Some(target) = &args.target {
        cmd.arg(target);
    }
    cmd.arg_if("--dry-run", args.dry_run)
        .option("--tag", args.tag.as_ref())
        .option("--access", args.access.as_ref())
        .option("--otp", args.otp.as_ref());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        resolve,
        test_utils::{bun, npm, parse_args, pnpm, yarn},
    };

    #[test]
    fn test_parser_accepts_target_filter_and_pass_through_args() {
        let args = parse_args::<PublishArgs>([
            "pkg.tgz",
            "--filter",
            "app",
            "--",
            "--registry",
            "https://registry.npmjs.org",
        ])
        .unwrap();

        assert_eq!(args.target, Some("pkg.tgz".to_string()));
        assert_eq!(args.filter, Some(vec!["app".to_string()]));
        assert_eq!(
            args.pass_through_args,
            vec!["--registry".to_string(), "https://registry.npmjs.org".to_string()]
        );
    }

    #[test]
    fn test_pnpm_publish() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), PublishArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["publish"]);
    }

    #[test]
    fn test_npm_publish() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), PublishArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["publish"]);
    }

    #[test]
    fn test_yarn1_publish_uses_npm() {
        let CommandResolution::Run(command) =
            resolve(&yarn("1.22.0"), PublishArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["publish"]);
    }

    #[test]
    fn test_yarn2_publish_uses_npm() {
        let CommandResolution::Run(command) =
            resolve(&yarn("4.0.0"), PublishArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["publish"]);
    }

    #[test]
    fn test_yarn_publish_with_tag() {
        let CommandResolution::Run(command) = resolve(
            &yarn("4.0.0"),
            PublishArgs { tag: Some("beta".to_string()), ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["publish", "--tag", "beta"]);
    }

    #[test]
    fn test_pnpm_publish_recursive() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), PublishArgs { recursive: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["publish", "--recursive"]);
    }

    #[test]
    fn test_npm_publish_recursive() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), PublishArgs { recursive: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["publish", "--workspaces"]);
    }

    #[test]
    fn test_pnpm_publish_with_filter() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.0.0"),
            PublishArgs { filter: Some(vec!["app".to_string()]), ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["--filter", "app", "publish"]);
    }

    #[test]
    fn test_npm_publish_with_filter() {
        let CommandResolution::Run(command) = resolve(
            &npm("11.0.0"),
            PublishArgs { filter: Some(vec!["app".to_string()]), ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["publish", "--workspace", "app"]);
    }

    #[test]
    fn test_yarn_publish_with_filter() {
        let CommandResolution::Run(command) = resolve(
            &yarn("1.22.0"),
            PublishArgs { filter: Some(vec!["app".to_string()]), ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["publish", "--workspace", "app"]);
    }

    #[test]
    fn test_pnpm_publish_json() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), PublishArgs { json: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["publish", "--json"]);
    }

    #[test]
    fn test_npm_publish_json_ignored() {
        let resolution = resolve(&npm("11.0.0"), PublishArgs { json: true, ..Default::default() });
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["publish"]);
        assert_eq!(resolution.diagnostics[0].message, "npm does not support --json.");
    }

    #[test]
    fn test_yarn_publish_json_is_checked_against_current_dialect() {
        let resolution = resolve(&yarn("4.0.0"), PublishArgs { json: true, ..Default::default() });
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["publish"]);
        assert_eq!(resolution.diagnostics[0].message, "yarn does not support --json.");
    }

    #[test]
    fn test_pnpm_publish_branch() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.0.0"),
            PublishArgs { publish_branch: Some("main".to_string()), ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["publish", "--publish-branch", "main"]);
    }

    #[test]
    fn test_npm_publish_branch_ignored() {
        let resolution = resolve(
            &npm("11.0.0"),
            PublishArgs { publish_branch: Some("main".to_string()), ..Default::default() },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["publish"]);
        assert_eq!(resolution.diagnostics[0].message, "npm does not support --publish-branch.");
    }

    #[test]
    fn test_pnpm_publish_report_summary() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), PublishArgs { report_summary: true, ..Default::default() })
                .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["publish", "--report-summary"]);
    }

    #[test]
    fn test_npm_publish_report_summary_ignored() {
        let resolution =
            resolve(&npm("11.0.0"), PublishArgs { report_summary: true, ..Default::default() });
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["publish"]);
        assert_eq!(resolution.diagnostics[0].message, "npm does not support --report-summary.");
    }

    #[test]
    fn test_pnpm_publish_provenance() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), PublishArgs { provenance: true, ..Default::default() })
                .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["publish", "--provenance"]);
    }

    #[test]
    fn test_npm_publish_provenance() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), PublishArgs { provenance: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["publish", "--provenance"]);
    }

    #[test]
    fn test_yarn_publish_provenance() {
        let CommandResolution::Run(command) =
            resolve(&yarn("4.0.0"), PublishArgs { provenance: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["publish", "--provenance"]);
    }

    #[test]
    fn test_bun_publish_provenance_ignored() {
        let resolution =
            resolve(&bun("1.2.0"), PublishArgs { provenance: true, ..Default::default() });
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["publish"]);
        assert_eq!(resolution.diagnostics[0].message, "bun does not support --provenance.");
    }

    #[test]
    fn test_pnpm_publish_otp() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.0.0"),
            PublishArgs { otp: Some("123456".to_string()), ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["publish", "--otp", "123456"]);
    }

    #[test]
    fn test_npm_publish_otp() {
        let CommandResolution::Run(command) = resolve(
            &npm("11.0.0"),
            PublishArgs { otp: Some("654321".to_string()), ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["publish", "--otp", "654321"]);
    }

    #[test]
    fn test_yarn_publish_otp() {
        let CommandResolution::Run(command) = resolve(
            &yarn("1.22.0"),
            PublishArgs { otp: Some("999999".to_string()), ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["publish", "--otp", "999999"]);
    }

    #[test]
    fn test_publish_common_options() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.0.0"),
            PublishArgs {
                target: Some("pkg.tgz".to_string()),
                dry_run: true,
                tag: Some("next".to_string()),
                access: Some("public".to_string()),
                force: true,
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
        assert_eq!(
            command.args,
            vec![
                "publish",
                "pkg.tgz",
                "--dry-run",
                "--tag",
                "next",
                "--access",
                "public",
                "--force",
                "--registry",
                "https://registry.npmjs.org"
            ]
        );
    }

    #[test]
    fn test_npm_silently_ignores_no_git_checks() {
        let resolution =
            resolve(&npm("11.0.0"), PublishArgs { no_git_checks: true, ..Default::default() });
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["publish"]);
        assert!(resolution.diagnostics.is_empty());
    }

    #[test]
    fn test_bun_publish_unsupported_flags_warn_and_drop() {
        let resolution = resolve(
            &bun("1.3.11"),
            PublishArgs {
                no_git_checks: true,
                publish_branch: Some("main".to_string()),
                report_summary: true,
                provenance: true,
                force: true,
                json: true,
                recursive: true,
                filter: Some(vec!["app".to_string()]),
                ..Default::default()
            },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["publish"]);
        assert_eq!(resolution.diagnostics.len(), 8);
        assert_eq!(resolution.diagnostics[0].message, "bun does not support --no-git-checks.");
        assert_eq!(resolution.diagnostics[7].message, "bun does not support --filter.");
    }
}
