use vp_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct AuditArgs {
    /// Automatically fix vulnerabilities
    #[arg(long, not_supported(yarn, bun < "1.4"))]
    pub(crate) fix: bool,

    /// Output in JSON format
    #[arg(long)]
    pub(crate) json: bool,

    /// Minimum vulnerability level to report
    #[arg(long, value_name = "LEVEL")]
    pub(crate) level: Option<String>,

    /// Only audit production dependencies
    // Bun ignores the flag before 1.2.21, when `bun audit --prod` landed.
    // https://bun.sh/blog/bun-v1.2.21
    #[arg(long, not_supported(bun < "1.2.21"))]
    pub(crate) production: bool,

    /// Additional arguments
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) pass_through_args: Vec<String>,
}

impl Resolve<AuditArgs> for Npm {
    fn resolve(&self, args: &AuditArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("npm");
        cmd.arg("audit");
        if args.fix {
            cmd.arg("fix");
        }
        cmd.option("--audit-level", args.level.as_ref())
            .arg_if("--omit=dev", args.production)
            .arg_if("--json", args.json)
            .extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<AuditArgs> for Pnpm {
    fn resolve(&self, args: &AuditArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("pnpm");
        cmd.arg("audit")
            .arg_if("--fix", args.fix)
            .option("--audit-level", args.level.as_ref())
            .arg_if("--prod", args.production)
            .arg_if("--json", args.json)
            .extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<AuditArgs> for Yarn {
    fn resolve(&self, args: &AuditArgs, _diag: &mut Diagnostics) -> CommandResolution {
        if self.is_berry() {
            let mut cmd = CommandBuilder::new("yarn");
            cmd.arg("npm")
                .arg("audit")
                .option("--severity", args.level.as_ref())
                .arg_if("--json", args.json);
            if args.production {
                cmd.arg("--environment").arg("production");
            }
            cmd.extend(args.pass_through_args.iter());
            return cmd.into();
        }

        let mut cmd = CommandBuilder::new("yarn");
        cmd.arg("audit").option("--level", args.level.as_ref()).arg_if("--json", args.json);
        if args.production {
            cmd.arg("--groups").arg("dependencies");
        }
        cmd.extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<AuditArgs> for Bun {
    fn resolve(&self, args: &AuditArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("bun");
        cmd.arg("audit").arg_if("fix", args.fix);
        cmd.option("--audit-level", args.level.as_ref())
            .arg_if("--production", args.production)
            .arg_if("--json", args.json);
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

    #[test]
    fn test_parser_accepts_flags_and_pass_through_args() {
        let args = parse_args::<AuditArgs>([
            "--json",
            "--level",
            "high",
            "--",
            "--registry",
            "https://registry.npmjs.org",
        ])
        .unwrap();

        assert!(args.json);
        assert_eq!(args.level, Some("high".to_string()));
        assert_eq!(
            args.pass_through_args,
            vec!["--registry".to_string(), "https://registry.npmjs.org".to_string()]
        );
    }

    #[test]
    fn test_npm_audit() {
        let resolution = resolve(&npm("11.0.0"), AuditArgs::default());
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["audit"]);
    }

    #[test]
    fn test_npm_audit_fix() {
        let resolution = resolve(&npm("11.0.0"), AuditArgs { fix: true, ..Default::default() });
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["audit", "fix"]);
    }

    #[test]
    fn test_pnpm_audit_fix() {
        let resolution = resolve(&pnpm("10.0.0"), AuditArgs { fix: true, ..Default::default() });
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["audit", "--fix"]);
    }

    #[test]
    fn test_yarn1_audit() {
        let resolution = resolve(&yarn("1.22.0"), AuditArgs::default());
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["audit"]);
    }

    #[test]
    fn test_yarn1_audit_fix_not_supported() {
        let resolution = resolve(&yarn("1.22.0"), AuditArgs { fix: true, ..Default::default() });

        expect_unsupported(resolution, &["yarn does not support --fix."]);
    }

    #[test]
    fn test_yarn2_audit() {
        let resolution = resolve(&yarn("4.0.0"), AuditArgs::default());
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["npm", "audit"]);
    }

    #[test]
    fn test_yarn2_audit_fix_not_supported() {
        for version in ["2.4.2", "3.6.0", "4.18.0"] {
            let resolution = resolve(&yarn(version), AuditArgs { fix: true, ..Default::default() });
            expect_unsupported(resolution, &["yarn does not support --fix."]);
        }
    }

    #[test]
    fn test_audit_with_level_npm() {
        let resolution = resolve(
            &npm("11.0.0"),
            AuditArgs { level: Some("high".to_string()), ..Default::default() },
        );
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["audit", "--audit-level", "high"]);
    }

    #[test]
    fn test_audit_with_level_yarn1() {
        let resolution = resolve(
            &yarn("1.22.0"),
            AuditArgs { level: Some("high".to_string()), ..Default::default() },
        );
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["audit", "--level", "high"]);
    }

    #[test]
    fn test_bun_audit_basic() {
        let resolution = resolve(&bun("1.3.11"), AuditArgs::default());
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["audit"]);
    }

    #[test]
    fn test_bun_audit_level() {
        let resolution = resolve(
            &bun("1.3.11"),
            AuditArgs { level: Some("high".to_string()), ..Default::default() },
        );
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["audit", "--audit-level", "high"]);
    }

    #[test]
    fn test_bun_audit_fix_not_supported() {
        for version in ["1.3.11", "1.3.14", "1.4.0-rc.0"] {
            let resolution = resolve(&bun(version), AuditArgs { fix: true, ..Default::default() });
            expect_unsupported(resolution, &["bun < 1.4 does not support --fix."]);
        }
    }

    #[test]
    fn test_bun_audit_production_follows_native_support() {
        let args = AuditArgs { production: true, ..Default::default() };
        expect_unsupported(
            resolve(&bun("1.2.20"), args.clone()),
            &["bun < 1.2.21 does not support --production."],
        );
        let command = expect_run(resolve(&bun("1.2.21"), args).outcome);
        assert_eq!(command.args, vec!["audit", "--production"]);
    }

    #[test]
    fn test_bun_audit_rejects_fix_with_supported_production() {
        let args = parse_args::<AuditArgs>(["--fix", "--production", "--", "--help"]).unwrap();
        expect_unsupported(resolve(&bun("1.3.11"), args), &["bun < 1.4 does not support --fix."]);
    }

    #[test]
    fn test_audit_preserves_raw_fix_args() {
        let args = parse_args::<AuditArgs>(["--", "--fix"]).unwrap();
        for (resolution, expected) in [
            (resolve(&yarn("1.22.22"), args.clone()), vec!["audit", "--fix"]),
            (resolve(&yarn("4.18.0"), args.clone()), vec!["npm", "audit", "--fix"]),
            (resolve(&bun("1.3.11"), args), vec!["audit", "--fix"]),
        ] {
            assert_eq!(expect_run(resolution.outcome).args, expected);
            assert!(resolution.diagnostics.is_empty());
        }
    }

    #[test]
    fn test_bun_audit_fix() {
        let resolution =
            resolve(&bun("1.4.0"), AuditArgs { fix: true, production: true, ..Default::default() });
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["audit", "fix", "--production"]);
        assert!(resolution.diagnostics.is_empty());
    }

    #[test]
    fn test_bun_audit_json() {
        let resolution = resolve(&bun("1.3.11"), AuditArgs { json: true, ..Default::default() });
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["audit", "--json"]);
    }

    #[test]
    fn test_audit_with_level_yarn2() {
        let resolution = resolve(
            &yarn("4.0.0"),
            AuditArgs { level: Some("high".to_string()), ..Default::default() },
        );
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["npm", "audit", "--severity", "high"]);
    }

    #[test]
    fn test_production_flag_maps_per_package_manager() {
        let npm_resolution =
            resolve(&npm("11.0.0"), AuditArgs { production: true, ..Default::default() });
        let pnpm_resolution =
            resolve(&pnpm("10.0.0"), AuditArgs { production: true, ..Default::default() });
        let yarn_resolution =
            resolve(&yarn("1.22.0"), AuditArgs { production: true, ..Default::default() });
        let yarn_berry_resolution =
            resolve(&yarn("4.0.0"), AuditArgs { production: true, ..Default::default() });
        let bun_resolution =
            resolve(&bun("1.3.11"), AuditArgs { production: true, ..Default::default() });
        let npm_command = expect_run(npm_resolution.outcome);
        let pnpm_command = expect_run(pnpm_resolution.outcome);
        let yarn_command = expect_run(yarn_resolution.outcome);
        let yarn_berry_command = expect_run(yarn_berry_resolution.outcome);
        let bun_command = expect_run(bun_resolution.outcome);

        assert_eq!(npm_command.program, "npm");
        assert_eq!(npm_command.args, vec!["audit", "--omit=dev"]);
        assert_eq!(pnpm_command.program, "pnpm");
        assert_eq!(pnpm_command.args, vec!["audit", "--prod"]);
        assert_eq!(yarn_command.program, "yarn");
        assert_eq!(yarn_command.args, vec!["audit", "--groups", "dependencies"]);
        assert_eq!(yarn_berry_command.program, "yarn");
        assert_eq!(yarn_berry_command.args, vec!["npm", "audit", "--environment", "production"]);
        assert_eq!(bun_command.args, vec!["audit", "--production"]);
    }
}
