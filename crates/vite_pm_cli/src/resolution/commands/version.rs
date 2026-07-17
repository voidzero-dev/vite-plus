use semver::Version;
use vite_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, DiagnosticKind, Diagnostics, Npm,
    PackageManagerDialect, Pnpm, Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct VersionArgs {
    /// Version number or increment strategy
    pub(crate) new_version: Option<String>,

    /// Output in JSON format
    #[arg(long)]
    pub(crate) json: bool,

    /// Arguments to pass to the package manager
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) pass_through_args: Vec<String>,
}

impl Resolve<VersionArgs> for Pnpm {
    fn resolve(&self, args: &VersionArgs, _diag: &mut Diagnostics) -> CommandResolution {
        resolve_native_version("pnpm", args)
    }
}

impl Resolve<VersionArgs> for Npm {
    fn resolve(&self, args: &VersionArgs, _diag: &mut Diagnostics) -> CommandResolution {
        resolve_native_version("npm", args)
    }
}

impl Resolve<VersionArgs> for Yarn {
    fn resolve(&self, args: &VersionArgs, _diag: &mut Diagnostics) -> CommandResolution {
        if self.is_berry() && args.json {
            return unsupported_json("Yarn 2+");
        }

        let mut cmd = CommandBuilder::new("yarn");
        cmd.arg("version");
        if let Some(new_version) = &args.new_version {
            if self.is_berry() {
                cmd.arg(new_version);
            } else if is_yarn_classic_increment(new_version) {
                cmd.arg(vite_str::format!("--{new_version}"));
            } else {
                cmd.arg("--new-version").arg(new_version);
            }
        }
        cmd.arg_if("--json", args.json).extend(args.pass_through_args.iter());
        cmd.into()
    }
}

impl Resolve<VersionArgs> for Bun {
    fn resolve(&self, args: &VersionArgs, diag: &mut Diagnostics) -> CommandResolution {
        if args.json {
            return unsupported_json("Bun");
        }
        if self.version().is_some_and(|version| !bun_supports_version_command(version)) {
            diag.warn(
                DiagnosticKind::BehaviorChange,
                vite_str::format!(
                    "bun {} does not support `bun pm version` (requires bun >= 1.2.18); forwarding anyway",
                    self.version().expect("bun dialect always has a version")
                ),
            );
        }

        let mut cmd = CommandBuilder::new("bun");
        cmd.arg("pm").arg("version");
        append_common_args(&mut cmd, args);
        cmd.into()
    }
}

fn resolve_native_version(program: &str, args: &VersionArgs) -> CommandResolution {
    let mut cmd = CommandBuilder::new(program);
    cmd.arg("version");
    append_common_args(&mut cmd, args);
    cmd.into()
}

fn append_common_args(cmd: &mut CommandBuilder, args: &VersionArgs) {
    if let Some(new_version) = &args.new_version {
        cmd.arg(new_version);
    }
    cmd.arg_if("--json", args.json).extend(args.pass_through_args.iter());
}

fn unsupported_json(manager: &str) -> CommandResolution {
    CommandResolution::InvalidArgument(
        vite_str::format!("Invalid argument: `--json` is not supported by {manager} `version`.")
            .to_string(),
    )
}

fn is_yarn_classic_increment(value: &str) -> bool {
    matches!(
        value,
        "major" | "minor" | "patch" | "premajor" | "preminor" | "prepatch" | "prerelease"
    )
}

fn bun_supports_version_command(version: &Version) -> bool {
    version >= &Version::new(1, 2, 18)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        PackageManagerDialect, resolve,
        test_utils::{bun, npm, parse_args, pnpm, yarn},
    };

    #[test]
    fn parses_native_version_arguments() {
        let args =
            parse_args::<VersionArgs>(["prerelease", "--json", "--", "--preid", "beta"]).unwrap();

        assert_eq!(args.new_version.as_deref(), Some("prerelease"));
        assert!(args.json);
        assert_eq!(args.pass_through_args, ["--preid", "beta"]);
    }

    #[test]
    fn resolves_npm_and_pnpm_version_commands() {
        let args = VersionArgs {
            new_version: Some("prerelease".to_string()),
            pass_through_args: vec!["--preid".to_string(), "beta".to_string()],
            ..Default::default()
        };

        let CommandResolution::Run(command) = resolve(&npm("11.0.0"), args.clone()).outcome else {
            panic!("expected command resolution");
        };
        assert_eq!(command.program, "npm");
        assert_eq!(command.args, ["version", "prerelease", "--preid", "beta"]);

        let CommandResolution::Run(command) = resolve(&pnpm("10.0.0"), args).outcome else {
            panic!("expected command resolution");
        };
        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, ["version", "prerelease", "--preid", "beta"]);
    }

    #[test]
    fn translates_yarn_classic_versions() {
        let patch = resolve(
            &yarn("1.22.22"),
            VersionArgs { new_version: Some("patch".to_string()), ..Default::default() },
        );
        let CommandResolution::Run(patch) = patch.outcome else {
            panic!("expected command resolution");
        };
        assert_eq!(patch.args, ["version", "--patch"]);

        let explicit = resolve(
            &yarn("1.22.22"),
            VersionArgs { new_version: Some("2.0.0".to_string()), ..Default::default() },
        );
        let CommandResolution::Run(explicit) = explicit.outcome else {
            panic!("expected command resolution");
        };
        assert_eq!(explicit.args, ["version", "--new-version", "2.0.0"]);
    }

    #[test]
    fn keeps_yarn_berry_version_positional() {
        let resolution = resolve(
            &yarn("4.0.0"),
            VersionArgs { new_version: Some("patch".to_string()), ..Default::default() },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, ["version", "patch"]);
    }

    #[test]
    fn resolves_bun_pm_version() {
        let resolution = resolve(
            &bun("1.3.11"),
            VersionArgs { new_version: Some("patch".to_string()), ..Default::default() },
        );
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, ["pm", "version", "patch"]);
        assert!(resolution.diagnostics.is_empty());
    }

    #[test]
    fn rejects_unsupported_json_output() {
        for (resolution, manager) in [
            (resolve(&yarn("4.0.0"), VersionArgs { json: true, ..Default::default() }), "Yarn 2+"),
            (resolve(&bun("1.3.11"), VersionArgs { json: true, ..Default::default() }), "Bun"),
        ] {
            let CommandResolution::InvalidArgument(message) = resolution.outcome else {
                panic!("expected invalid argument");
            };
            assert_eq!(
                message,
                vite_str::format!(
                    "Invalid argument: `--json` is not supported by {manager} `version`."
                )
                .to_string()
            );
        }
    }

    #[test]
    fn warns_when_old_bun_may_not_support_version() {
        let dialect = bun("1.2.17");
        assert_eq!(dialect.version(), Some(&Version::new(1, 2, 17)));
        let resolution = resolve(&dialect, VersionArgs::default());
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, ["pm", "version"]);
        assert_eq!(resolution.diagnostics.len(), 1);
        assert_eq!(
            resolution.diagnostics[0].message,
            "bun 1.2.17 does not support `bun pm version` (requires bun >= 1.2.18); forwarding anyway"
        );
    }
}
