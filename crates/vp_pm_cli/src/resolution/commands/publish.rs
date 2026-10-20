use vp_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, Diagnostics, Npm, PackageManagerDialect, Pnpm, Resolve,
    Yarn,
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct PublishArgs {
    /// Tarball or folder to publish
    #[arg(value_name = "TARBALL|FOLDER", not_supported(yarn >= "2"))]
    pub(crate) target: Option<String>,

    /// Preview without publishing
    // npm 5.8 ignores this flag and publishes; stable 5.10 supports previews.
    #[arg(long, not_supported(npm < "5.10.0", yarn < "4.9.3"))]
    pub(crate) dry_run: bool,

    /// Publish tag
    #[arg(long)]
    pub(crate) tag: Option<String>,

    /// Access level (public/restricted)
    #[arg(long)]
    pub(crate) access: Option<String>,

    /// One-time password for authentication
    #[arg(long, value_name = "OTP", not_supported(yarn < "1.12.1", yarn >= "2" && < "3.2.0"))]
    pub(crate) otp: Option<String>,

    /// Skip git checks
    #[arg(long, not_supported(npm, yarn, bun))]
    pub(crate) no_git_checks: bool,

    /// Set the branch name to publish from
    #[arg(long, value_name = "BRANCH", not_supported(npm, yarn, bun))]
    pub(crate) publish_branch: Option<String>,

    /// Save publish summary
    #[arg(long, not_supported(npm, yarn, bun))]
    pub(crate) report_summary: bool,

    /// Publish with provenance
    #[arg(long, not_supported(npm < "9.5.0", yarn < "4.9.0", bun))]
    pub(crate) provenance: bool,

    /// Force publish
    #[arg(long, not_supported(yarn, bun))]
    pub(crate) force: bool,

    /// Output in JSON format
    // pnpm 11's native publish dropped JSON output until 11.0.7 restored it.
    // https://github.com/pnpm/pnpm/releases/tag/v11.0.7
    #[arg(long, not_supported(pnpm >= "11.0.0" && < "11.0.7", yarn >= "2" && < "4.9.3", bun))]
    pub(crate) json: bool,

    /// Publish all workspace packages
    #[arg(short = 'r', long, not_supported(npm < "7.13.0", yarn, bun))]
    pub(crate) recursive: bool,

    /// Filter packages in monorepo
    #[arg(long, value_name = "PATTERN", not_supported(npm < "7.13.0", yarn, bun))]
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
        cmd.arg_if("--provenance", args.provenance)
            .arg_if("--force", args.force)
            .arg_if("--json", args.json);
        cmd.extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<PublishArgs> for Npm {
    fn resolve(&self, args: &PublishArgs, _diag: &mut Diagnostics) -> CommandResolution {
        if self.version().is_none() && (args.dry_run || args.provenance) {
            return CommandResolution::InvalidArgument(
                "A known npm version is required for --dry-run and --provenance.".into(),
            );
        }
        Self::resolve_publish(args)
    }
}

impl Resolve<PublishArgs> for Yarn {
    fn resolve(&self, args: &PublishArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("yarn");
        if self.is_berry() {
            cmd.arg("npm");
        }
        cmd.arg("publish");
        push_common_publish_args(&mut cmd, args);
        cmd.arg_if("--provenance", args.provenance)
            .arg_if("--force", args.force)
            .arg_if("--json", args.json)
            .extend(args.pass_through_args.iter());
        cmd.into()
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
        test_utils::{bun, expect_run, expect_unsupported, npm, parse_args, pnpm, yarn},
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
        let command = expect_run(resolve(&pnpm("10.0.0"), PublishArgs::default()).outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["publish"]);
    }

    #[test]
    fn test_npm_publish() {
        let command = expect_run(resolve(&npm("11.0.0"), PublishArgs::default()).outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["publish"]);
    }

    #[test]
    fn test_yarn1_publish_is_native() {
        let command = expect_run(resolve(&yarn("1.22.0"), PublishArgs::default()).outcome);

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["publish"]);
    }

    #[test]
    fn test_yarn2_publish_is_native() {
        let command = expect_run(resolve(&yarn("4.0.0"), PublishArgs::default()).outcome);

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["npm", "publish"]);
    }

    #[test]
    fn test_pnpm_publish_recursive() {
        let command = expect_run(
            resolve(&pnpm("10.0.0"), PublishArgs { recursive: true, ..Default::default() }).outcome,
        );

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["publish", "--recursive"]);
    }

    #[test]
    fn test_npm_publish_recursive() {
        let command = expect_run(
            resolve(&npm("11.0.0"), PublishArgs { recursive: true, ..Default::default() }).outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["publish", "--workspaces"]);
    }

    #[test]
    fn test_pnpm_publish_with_filter() {
        let command = expect_run(
            resolve(
                &pnpm("10.0.0"),
                PublishArgs { filter: Some(vec!["app".to_string()]), ..Default::default() },
            )
            .outcome,
        );

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["--filter", "app", "publish"]);
    }

    #[test]
    fn test_npm_publish_with_filter() {
        let command = expect_run(
            resolve(
                &npm("11.0.0"),
                PublishArgs { filter: Some(vec!["app".to_string()]), ..Default::default() },
            )
            .outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["publish", "--workspace", "app"]);
    }

    #[test]
    fn test_yarn_publish_rejects_workspace_selection() {
        for version in ["1.22.22", "4.9.3"] {
            expect_unsupported(
                resolve(&yarn(version), PublishArgs { recursive: true, ..Default::default() }),
                &["yarn does not support --recursive."],
            );
            expect_unsupported(
                resolve(
                    &yarn(version),
                    PublishArgs { filter: Some(vec!["app".into()]), ..Default::default() },
                ),
                &["yarn does not support --filter."],
            );
        }
    }

    #[test]
    fn test_yarn_publish_target_is_classic_only() {
        for target in ["pkg.tgz", "./packages/app"] {
            let args = PublishArgs { target: Some(target.into()), ..Default::default() };
            let command = expect_run(resolve(&yarn("1.22.22"), args.clone()).outcome);
            assert_eq!(command.program, "yarn");
            assert_eq!(command.args, vec!["publish", target]);
            for version in ["2.0.0", "4.9.3"] {
                expect_unsupported(
                    resolve(&yarn(version), args.clone()),
                    &["yarn >= 2 does not support TARBALL|FOLDER."],
                );
            }
        }
    }

    #[test]
    fn test_yarn_publish_dry_run_version_boundary() {
        let args = parse_args::<PublishArgs>(["--dry-run"]).unwrap();
        for version in ["1.22.22", "2.4.3", "4.9.2", "4.9.3-rc.1"] {
            expect_unsupported(
                resolve(&yarn(version), args.clone()),
                &["yarn < 4.9.3 does not support --dry-run."],
            );
        }
        for version in ["4.9.3", "4.9.4"] {
            let resolution = resolve(&yarn(version), args.clone());
            assert!(resolution.diagnostics.is_empty());
            let command = expect_run(resolution.outcome);
            assert_eq!(command.program, "yarn");
            assert_eq!(command.args, vec!["npm", "publish", "--dry-run"]);
        }
    }

    #[test]
    fn test_pnpm_publish_json() {
        let command = expect_run(
            resolve(&pnpm("10.0.0"), PublishArgs { json: true, ..Default::default() }).outcome,
        );

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["publish", "--json"]);
    }

    #[test]
    fn test_npm_publish_json() {
        for version in ["10.9.4", "11.16.0", "12.0.2"] {
            let resolution =
                resolve(&npm(version), PublishArgs { json: true, ..Default::default() });
            assert!(resolution.diagnostics.is_empty());
            let command = expect_run(resolution.outcome);
            assert_eq!(command.program, "npm");
            assert_eq!(command.args, vec!["publish", "--json"]);
        }
    }

    #[test]
    fn test_yarn_publish_json_version_boundary() {
        let args = parse_args::<PublishArgs>(["--json"]).unwrap();
        for version in ["2.0.0", "4.9.2", "4.9.3-rc.1"] {
            expect_unsupported(
                resolve(&yarn(version), args.clone()),
                &["yarn >= 2 && < 4.9.3 does not support --json."],
            );
        }
        for (version, expected) in [
            ("1.22.22", vec!["publish", "--json"]),
            ("4.9.3", vec!["npm", "publish", "--json"]),
            ("4.10.3", vec!["npm", "publish", "--json"]),
        ] {
            let resolution = resolve(&yarn(version), args.clone());
            assert!(resolution.diagnostics.is_empty());
            let command = expect_run(resolution.outcome);
            assert_eq!(command.program, "yarn");
            assert_eq!(command.args, expected);
        }
    }

    #[test]
    fn test_pnpm_publish_branch() {
        let command = expect_run(
            resolve(
                &pnpm("10.0.0"),
                PublishArgs { publish_branch: Some("main".to_string()), ..Default::default() },
            )
            .outcome,
        );

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["publish", "--publish-branch", "main"]);
    }

    #[test]
    fn test_npm_publish_branch_is_rejected() {
        let resolution = resolve(
            &npm("11.0.0"),
            PublishArgs { publish_branch: Some("main".to_string()), ..Default::default() },
        );
        expect_unsupported(resolution, &["npm does not support --publish-branch."]);
    }

    #[test]
    fn test_pnpm_publish_report_summary() {
        let command = expect_run(
            resolve(&pnpm("10.0.0"), PublishArgs { report_summary: true, ..Default::default() })
                .outcome,
        );

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["publish", "--report-summary"]);
    }

    #[test]
    fn test_npm_publish_report_summary_is_rejected() {
        let resolution =
            resolve(&npm("11.0.0"), PublishArgs { report_summary: true, ..Default::default() });
        expect_unsupported(resolution, &["npm does not support --report-summary."]);
    }

    #[test]
    fn test_pnpm_publish_provenance() {
        let command = expect_run(
            resolve(&pnpm("10.0.0"), PublishArgs { provenance: true, ..Default::default() })
                .outcome,
        );

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["publish", "--provenance"]);
    }

    #[test]
    fn test_npm_publish_provenance() {
        let command = expect_run(
            resolve(&npm("11.0.0"), PublishArgs { provenance: true, ..Default::default() }).outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["publish", "--provenance"]);
    }

    #[test]
    fn test_yarn_publish_preserves_supported_common_flags() {
        let args = PublishArgs {
            tag: Some("beta".into()),
            access: Some("public".into()),
            otp: Some("123456".into()),
            json: true,
            pass_through_args: vec!["--help".into()],
            ..Default::default()
        };
        for (version, prefix) in [("1.22.22", vec!["publish"]), ("4.9.3", vec!["npm", "publish"])] {
            let resolution = resolve(&yarn(version), args.clone());
            assert!(resolution.diagnostics.is_empty());
            let command = expect_run(resolution.outcome);
            assert_eq!(command.program, "yarn");
            assert_eq!(
                command.args,
                [
                    prefix,
                    vec![
                        "--tag", "beta", "--access", "public", "--otp", "123456", "--json",
                        "--help"
                    ]
                ]
                .concat()
            );
        }
    }

    #[test]
    fn test_yarn_publish_provenance_version_boundary() {
        let args = parse_args::<PublishArgs>(["--provenance"]).unwrap();
        for version in ["1.22.22", "2.4.3", "4.8.1", "4.9.0-rc.1"] {
            expect_unsupported(
                resolve(&yarn(version), args.clone()),
                &["yarn < 4.9.0 does not support --provenance."],
            );
        }
        for version in ["4.9.0", "4.9.3"] {
            let resolution = resolve(&yarn(version), args.clone());
            assert!(resolution.diagnostics.is_empty());
            let command = expect_run(resolution.outcome);
            assert_eq!(command.program, "yarn");
            assert_eq!(command.args, vec!["npm", "publish", "--provenance"]);
        }
    }

    #[test]
    fn test_bun_publish_provenance_is_rejected() {
        let resolution =
            resolve(&bun("1.2.0"), PublishArgs { provenance: true, ..Default::default() });
        expect_unsupported(resolution, &["bun does not support --provenance."]);
    }

    #[test]
    fn test_pnpm_publish_otp() {
        let command = expect_run(
            resolve(
                &pnpm("10.0.0"),
                PublishArgs { otp: Some("123456".to_string()), ..Default::default() },
            )
            .outcome,
        );

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["publish", "--otp", "123456"]);
    }

    #[test]
    fn test_npm_publish_otp() {
        let command = expect_run(
            resolve(
                &npm("11.0.0"),
                PublishArgs { otp: Some("654321".to_string()), ..Default::default() },
            )
            .outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["publish", "--otp", "654321"]);
    }

    #[test]
    fn test_yarn_publish_otp_version_boundary() {
        let args = parse_args::<PublishArgs>(["--otp", "999999"]).unwrap();
        for version in ["1.12.0", "1.12.1-rc.1"] {
            expect_unsupported(
                resolve(&yarn(version), args.clone()),
                &["yarn < 1.12.1 does not support --otp."],
            );
        }
        for version in ["2.0.0", "3.1.1", "3.2.0-rc.1"] {
            expect_unsupported(
                resolve(&yarn(version), args.clone()),
                &["yarn >= 2 && < 3.2.0 does not support --otp."],
            );
        }
        for (version, expected) in [
            ("1.12.1", vec!["publish", "--otp", "999999"]),
            ("3.2.0", vec!["npm", "publish", "--otp", "999999"]),
            ("4.9.3", vec!["npm", "publish", "--otp", "999999"]),
        ] {
            let resolution = resolve(&yarn(version), args.clone());
            assert!(resolution.diagnostics.is_empty());
            let command = expect_run(resolution.outcome);
            assert_eq!(command.program, "yarn");
            assert_eq!(command.args, expected);
        }
    }

    #[test]
    fn test_yarn_publish_rejects_force() {
        let args = parse_args::<PublishArgs>(["--force"]).unwrap();
        for version in ["1.22.22", "4.9.3"] {
            expect_unsupported(
                resolve(&yarn(version), args.clone()),
                &["yarn does not support --force."],
            );
        }
    }

    #[test]
    fn test_yarn_publish_preserves_raw_unsupported_flags() {
        let args =
            parse_args::<PublishArgs>(["--", "--dry-run", "--force", "--json", "--otp", "123456"])
                .unwrap();
        for (version, prefix) in [("1.22.22", vec!["publish"]), ("2.4.3", vec!["npm", "publish"])] {
            let resolution = resolve(&yarn(version), args.clone());
            assert!(resolution.diagnostics.is_empty());
            assert_eq!(
                expect_run(resolution.outcome).args,
                [prefix, vec!["--dry-run", "--force", "--json", "--otp", "123456"]].concat()
            );
        }
    }

    #[test]
    fn test_publish_common_options() {
        let command = expect_run(
            resolve(
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
            .outcome,
        );

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
    fn test_npm_publish_rejects_no_git_checks() {
        let resolution =
            resolve(&npm("12.0.2"), PublishArgs { no_git_checks: true, ..Default::default() });
        expect_unsupported(resolution, &["npm does not support --no-git-checks."]);
    }

    #[test]
    fn test_bun_publish_rejects_no_git_checks() {
        for version in ["1.3.11", "1.4.0", "1.4.2"] {
            let resolution =
                resolve(&bun(version), PublishArgs { no_git_checks: true, ..Default::default() });
            expect_unsupported(resolution, &["bun does not support --no-git-checks."]);
        }
    }

    #[test]
    fn test_yarn_publish_rejects_no_git_checks() {
        for version in ["1.22.22", "2.4.2", "3.6.0", "4.18.0"] {
            let resolution =
                resolve(&yarn(version), PublishArgs { no_git_checks: true, ..Default::default() });
            expect_unsupported(resolution, &["yarn does not support --no-git-checks."]);
        }
    }

    #[test]
    fn test_publish_preserves_raw_no_git_checks() {
        let args = parse_args::<PublishArgs>(["--", "--no-git-checks"]).unwrap();
        assert!(!args.no_git_checks);
        assert_eq!(args.pass_through_args, vec!["--no-git-checks"]);
        for (resolution, expected) in [
            (resolve(&npm("12.0.2"), args.clone()), vec!["publish", "--no-git-checks"]),
            (resolve(&yarn("4.18.0"), args.clone()), vec!["npm", "publish", "--no-git-checks"]),
            (resolve(&bun("1.4.2"), args), vec!["publish", "--no-git-checks"]),
        ] {
            assert!(resolution.diagnostics.is_empty());
            assert_eq!(expect_run(resolution.outcome).args, expected);
        }
    }

    #[test]
    fn test_pnpm_publish_explicitly_disables_git_checks() {
        let resolution =
            resolve(&pnpm("11.24.0"), PublishArgs { no_git_checks: true, ..Default::default() });
        assert!(resolution.diagnostics.is_empty());
        assert_eq!(expect_run(resolution.outcome).args, vec!["publish", "--no-git-checks"]);
    }

    #[test]
    fn test_bun_publish_rejects_all_unsupported_flags() {
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
        expect_unsupported(
            resolution,
            &[
                "bun does not support --no-git-checks.",
                "bun does not support --publish-branch.",
                "bun does not support --report-summary.",
                "bun does not support --provenance.",
                "bun does not support --force.",
                "bun does not support --json.",
                "bun does not support --recursive.",
                "bun does not support --filter.",
            ],
        );
    }
}
