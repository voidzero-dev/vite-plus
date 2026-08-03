use vite_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct PackArgs {
    /// Pack all workspace packages
    #[arg(short = 'r', long, not_supported(yarn < "2", bun))]
    pub(crate) recursive: bool,

    /// Filter packages to pack
    #[arg(long, value_name = "PATTERN", not_supported(yarn < "2", bun))]
    pub(crate) filter: Vec<String>,

    /// Output path for the tarball
    #[arg(long, not_supported(npm))]
    pub(crate) out: Option<String>,

    /// Directory where the tarball will be saved
    #[arg(long, not_supported(yarn))]
    pub(crate) pack_destination: Option<String>,

    /// Gzip compression level (0-9)
    #[arg(long, not_supported(npm, yarn))]
    pub(crate) pack_gzip_level: Option<u8>,

    /// Output in JSON format
    #[arg(long, not_supported(bun))]
    pub(crate) json: bool,

    /// Additional arguments
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) pass_through_args: Vec<String>,
}

impl Resolve<PackArgs> for Pnpm {
    fn resolve(&self, args: &PackArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("pnpm");
        cmd.repeated("--filter", args.filter.iter());
        cmd.arg("pack")
            .arg_if("--recursive", args.recursive)
            .option("--out", args.out.as_ref())
            .option("--pack-destination", args.pack_destination.as_ref())
            .option("--pack-gzip-level", args.pack_gzip_level)
            .arg_if("--json", args.json)
            .extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<PackArgs> for Npm {
    fn resolve(&self, args: &PackArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("npm");
        cmd.arg("pack").arg_if("--workspaces", args.recursive);
        cmd.repeated("--workspace", args.filter.iter());
        if let Some(destination) = &args.pack_destination {
            cmd.create_dir(destination).arg("--pack-destination").arg(destination);
        }
        cmd.arg_if("--json", args.json).extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<PackArgs> for Yarn {
    fn resolve(&self, args: &PackArgs, _diag: &mut Diagnostics) -> CommandResolution {
        if self.is_berry() { Yarn::resolve_berry_pack(args) } else { Yarn::resolve_v1_pack(args) }
    }
}

impl Yarn {
    fn resolve_v1_pack(args: &PackArgs) -> CommandResolution {
        let mut cmd = CommandBuilder::new("yarn");
        cmd.arg("pack");
        if let Some(out) = &args.out {
            cmd.arg("--filename").arg(out);
        }
        cmd.arg_if("--json", args.json).extend(args.pass_through_args.iter());
        cmd.into()
    }

    fn resolve_berry_pack(args: &PackArgs) -> CommandResolution {
        let mut cmd = CommandBuilder::new("yarn");
        if args.recursive || !args.filter.is_empty() {
            cmd.arg("workspaces").arg("foreach").arg("--all");
            cmd.repeated("--include", args.filter.iter());
        }
        cmd.arg("pack");
        if let Some(out) = &args.out {
            cmd.arg("--out").arg(out);
        }
        cmd.arg_if("--json", args.json).extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<PackArgs> for Bun {
    fn resolve(&self, args: &PackArgs, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("bun");
        cmd.arg("pm").arg("pack");
        cmd.option("--filename", args.out.as_ref())
            .option("--destination", args.pack_destination.as_ref())
            .option("--gzip-level", args.pack_gzip_level);
        cmd.extend(args.pass_through_args.iter());
        cmd.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        DiagnosticKind,
        command::PreRunAction,
        resolve,
        test_utils::{bun, npm, parse_args, pnpm, yarn},
    };

    #[test]
    fn test_pnpm_pack_basic() {
        let CommandResolution::Run(command) = resolve(&pnpm("10.0.0"), PackArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["pack"]);
    }

    #[test]
    fn test_pnpm_pack_recursive() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), PackArgs { recursive: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["pack", "--recursive"]);
    }

    #[test]
    fn test_pnpm_pack_with_out() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.0.0"),
            PackArgs { out: Some("./dist/package.tgz".to_string()), ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["pack", "--out", "./dist/package.tgz"]);
    }

    #[test]
    fn test_pnpm_pack_with_destination() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.0.0"),
            PackArgs { pack_destination: Some("./dist".to_string()), ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["pack", "--pack-destination", "./dist"]);
    }

    #[test]
    fn test_pnpm_pack_destination_has_no_pre_run_action() {
        let resolution = resolve(
            &pnpm("10.0.0"),
            PackArgs { pack_destination: Some("./dist".to_string()), ..Default::default() },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert!(command.pre_run.is_empty());
    }

    #[test]
    fn test_pnpm_pack_with_gzip_level() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), PackArgs { pack_gzip_level: Some(9), ..Default::default() })
                .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["pack", "--pack-gzip-level", "9"]);
    }

    #[test]
    fn test_pnpm_pack_json() {
        let CommandResolution::Run(command) =
            resolve(&pnpm("10.0.0"), PackArgs { json: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["pack", "--json"]);
    }

    #[test]
    fn test_pnpm_pack_with_filter() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.0.0"),
            PackArgs { filter: vec!["app".to_string()], ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["--filter", "app", "pack"]);
    }

    #[test]
    fn test_pnpm_pack_with_multiple_filters() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.0.0"),
            PackArgs { filter: vec!["app".to_string(), "web".to_string()], ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["--filter", "app", "--filter", "web", "pack"]);
    }

    #[test]
    fn test_npm_pack_basic() {
        let CommandResolution::Run(command) = resolve(&npm("11.0.0"), PackArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["pack"]);
    }

    #[test]
    fn test_npm_pack_recursive() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), PackArgs { recursive: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["pack", "--workspaces"]);
    }

    #[test]
    fn test_npm_pack_with_destination() {
        let CommandResolution::Run(command) = resolve(
            &npm("11.0.0"),
            PackArgs { pack_destination: Some("./dist".to_string()), ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["pack", "--pack-destination", "./dist"]);
    }

    #[test]
    fn test_npm_pack_destination_creates_directory_before_run() {
        let resolution = resolve(
            &npm("11.0.0"),
            PackArgs { pack_destination: Some("./dist".to_string()), ..Default::default() },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.pre_run, vec![PreRunAction::CreateDir { path: "./dist".to_string() }]);
    }

    #[test]
    fn test_npm_pack_json() {
        let CommandResolution::Run(command) =
            resolve(&npm("11.0.0"), PackArgs { json: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["pack", "--json"]);
    }

    #[test]
    fn test_npm_pack_with_filter() {
        let CommandResolution::Run(command) = resolve(
            &npm("11.0.0"),
            PackArgs { filter: vec!["app".to_string()], ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["pack", "--workspace", "app"]);
    }

    #[test]
    fn test_npm_pack_with_multiple_filters() {
        let CommandResolution::Run(command) = resolve(
            &npm("11.0.0"),
            PackArgs { filter: vec!["app".to_string(), "web".to_string()], ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["pack", "--workspace", "app", "--workspace", "web"]);
    }

    #[test]
    fn test_npm_pack_unsupported_options_warn_and_drop() {
        let resolution = resolve(
            &npm("11.0.0"),
            PackArgs {
                out: Some("./dist/package.tgz".to_string()),
                pack_gzip_level: Some(9),
                ..Default::default()
            },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["pack"]);
        assert_eq!(resolution.diagnostics.len(), 2);
        assert_eq!(resolution.diagnostics[0].kind, DiagnosticKind::UnsupportedOptionDropped);
        assert_eq!(resolution.diagnostics[0].message, "npm does not support --out.");
        assert_eq!(resolution.diagnostics[1].kind, DiagnosticKind::UnsupportedOptionDropped);
        assert_eq!(resolution.diagnostics[1].message, "npm does not support --pack-gzip-level.");
    }

    #[test]
    fn test_yarn1_pack_basic() {
        let CommandResolution::Run(command) = resolve(&yarn("1.22.0"), PackArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["pack"]);
    }

    #[test]
    fn test_yarn1_pack_recursive_ignored() {
        let resolution =
            resolve(&yarn("1.22.0"), PackArgs { recursive: true, ..Default::default() });
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["pack"]);
        assert_eq!(resolution.diagnostics.len(), 1);
    }

    #[test]
    fn test_yarn1_pack_with_out() {
        let CommandResolution::Run(command) = resolve(
            &yarn("1.22.0"),
            PackArgs { out: Some("./dist/package.tgz".to_string()), ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["pack", "--filename", "./dist/package.tgz"]);
    }

    #[test]
    fn test_yarn1_pack_json() {
        let CommandResolution::Run(command) =
            resolve(&yarn("1.22.0"), PackArgs { json: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["pack", "--json"]);
    }

    #[test]
    fn test_yarn1_pack_with_filter_ignored() {
        let resolution = resolve(
            &yarn("1.22.0"),
            PackArgs { filter: vec!["app".to_string()], ..Default::default() },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["pack"]);
        assert_eq!(resolution.diagnostics.len(), 1);
    }

    #[test]
    fn test_yarn2_pack_basic() {
        let CommandResolution::Run(command) = resolve(&yarn("4.0.0"), PackArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["pack"]);
    }

    #[test]
    fn test_yarn2_pack_recursive() {
        let CommandResolution::Run(command) =
            resolve(&yarn("4.0.0"), PackArgs { recursive: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["workspaces", "foreach", "--all", "pack"]);
    }

    #[test]
    fn test_yarn2_pack_with_out() {
        let CommandResolution::Run(command) = resolve(
            &yarn("4.0.0"),
            PackArgs { out: Some("./dist/package.tgz".to_string()), ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["pack", "--out", "./dist/package.tgz"]);
    }

    #[test]
    fn test_yarn2_pack_json() {
        let CommandResolution::Run(command) =
            resolve(&yarn("4.0.0"), PackArgs { json: true, ..Default::default() }).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["pack", "--json"]);
    }

    #[test]
    fn test_yarn2_pack_with_filter() {
        let CommandResolution::Run(command) = resolve(
            &yarn("4.0.0"),
            PackArgs { filter: vec!["app".to_string()], ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(
            command.args,
            vec!["workspaces", "foreach", "--all", "--include", "app", "pack"]
        );
    }

    #[test]
    fn test_yarn2_pack_with_multiple_filters() {
        let CommandResolution::Run(command) = resolve(
            &yarn("4.0.0"),
            PackArgs { filter: vec!["app".to_string(), "web".to_string()], ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(
            command.args,
            vec!["workspaces", "foreach", "--all", "--include", "app", "--include", "web", "pack"]
        );
    }

    #[test]
    fn test_yarn2_pack_with_filter_and_recursive() {
        let CommandResolution::Run(command) = resolve(
            &yarn("4.0.0"),
            PackArgs { recursive: true, filter: vec!["app".to_string()], ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(
            command.args,
            vec!["workspaces", "foreach", "--all", "--include", "app", "pack"]
        );
    }

    #[test]
    fn test_yarn_pack_unsupported_options_warn_and_drop() {
        let resolution = resolve(
            &yarn("4.0.0"),
            PackArgs {
                pack_destination: Some("./dist".to_string()),
                pack_gzip_level: Some(9),
                ..Default::default()
            },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["pack"]);
        assert_eq!(resolution.diagnostics.len(), 2);
        assert_eq!(resolution.diagnostics[0].kind, DiagnosticKind::UnsupportedOptionDropped);
        assert_eq!(resolution.diagnostics[0].message, "yarn does not support --pack-destination.");
        assert_eq!(resolution.diagnostics[1].kind, DiagnosticKind::UnsupportedOptionDropped);
        assert_eq!(resolution.diagnostics[1].message, "yarn does not support --pack-gzip-level.");
    }

    #[test]
    fn test_bun_pack_basic() {
        let CommandResolution::Run(command) = resolve(&bun("1.3.11"), PackArgs::default()).outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["pm", "pack"]);
    }

    #[test]
    fn test_bun_pack_with_out_maps_to_filename() {
        let CommandResolution::Run(command) = resolve(
            &bun("1.3.11"),
            PackArgs { out: Some("./dist/package.tgz".to_string()), ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["pm", "pack", "--filename", "./dist/package.tgz"]);
    }

    #[test]
    fn test_bun_pack_with_destination() {
        let CommandResolution::Run(command) = resolve(
            &bun("1.3.11"),
            PackArgs { pack_destination: Some("./dist".to_string()), ..Default::default() },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["pm", "pack", "--destination", "./dist"]);
    }

    #[test]
    fn test_bun_pack_with_gzip_level() {
        let CommandResolution::Run(command) =
            resolve(&bun("1.3.11"), PackArgs { pack_gzip_level: Some(5), ..Default::default() })
                .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["pm", "pack", "--gzip-level", "5"]);
    }

    #[test]
    fn test_bun_pack_unsupported_options_warn_and_drop() {
        let resolution = resolve(
            &bun("1.3.11"),
            PackArgs {
                recursive: true,
                filter: vec!["app".to_string()],
                json: true,
                ..Default::default()
            },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["pm", "pack"]);
        assert_eq!(resolution.diagnostics.len(), 3);
        assert_eq!(resolution.diagnostics[0].kind, DiagnosticKind::UnsupportedOptionDropped);
        assert_eq!(resolution.diagnostics[0].message, "bun does not support --recursive.");
        assert_eq!(resolution.diagnostics[1].kind, DiagnosticKind::UnsupportedOptionDropped);
        assert_eq!(resolution.diagnostics[1].message, "bun does not support --filter.");
        assert_eq!(resolution.diagnostics[2].kind, DiagnosticKind::UnsupportedOptionDropped);
        assert_eq!(resolution.diagnostics[2].message, "bun does not support --json.");
    }

    #[test]
    fn test_pack_with_pass_through_args() {
        let CommandResolution::Run(command) = resolve(
            &pnpm("10.0.0"),
            PackArgs {
                pass_through_args: vec!["--report-summary".to_string()],
                ..Default::default()
            },
        )
        .outcome
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["pack", "--report-summary"]);
    }

    #[test]
    fn parser_captures_flags_and_pass_through_args() {
        let args = parse_args::<PackArgs>([
            "-r",
            "--filter",
            "app",
            "--out",
            "./dist/package.tgz",
            "--pack-destination",
            "./dist",
            "--pack-gzip-level",
            "9",
            "--json",
            "--",
            "--report-summary",
        ])
        .unwrap();

        assert!(args.recursive);
        assert_eq!(args.filter, vec!["app"]);
        assert_eq!(args.out, Some("./dist/package.tgz".to_string()));
        assert_eq!(args.pack_destination, Some("./dist".to_string()));
        assert_eq!(args.pack_gzip_level, Some(9));
        assert!(args.json);
        assert_eq!(args.pass_through_args, vec!["--report-summary".to_string()]);
    }
}
