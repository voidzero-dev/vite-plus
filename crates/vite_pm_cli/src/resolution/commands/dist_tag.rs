use vite_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, DiagnosticKind, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum DistTagCommand {
    /// List distribution tags for a package
    #[command(visible_alias = "ls")]
    List {
        /// Package name
        package: Option<String>,
    },

    /// Add a distribution tag
    Add {
        /// Package name with version (e.g., "my-pkg@1.0.0")
        package_at_version: String,

        /// Tag name
        tag: String,
    },

    /// Remove a distribution tag
    Rm {
        /// Package name
        package: String,

        /// Tag name
        tag: String,
    },
}

impl Npm {
    fn resolve_dist_tag(args: &DistTagCommand) -> CommandResolution {
        resolve_dist_tag("npm", &["dist-tag"], args)
    }
}

impl Resolve<DistTagCommand> for Npm {
    fn resolve(&self, args: &DistTagCommand, _diag: &mut Diagnostics) -> CommandResolution {
        Self::resolve_dist_tag(args)
    }
}

impl Resolve<DistTagCommand> for Pnpm {
    fn resolve(&self, args: &DistTagCommand, _diag: &mut Diagnostics) -> CommandResolution {
        Npm::resolve_dist_tag(args)
    }
}

impl Resolve<DistTagCommand> for Yarn {
    fn resolve(&self, args: &DistTagCommand, _diag: &mut Diagnostics) -> CommandResolution {
        if self.is_berry() {
            resolve_dist_tag("yarn", &["npm", "tag"], args)
        } else {
            resolve_dist_tag("yarn", &["tag"], args)
        }
    }
}

impl Resolve<DistTagCommand> for Bun {
    fn resolve(&self, args: &DistTagCommand, diag: &mut Diagnostics) -> CommandResolution {
        diag.warn(
            DiagnosticKind::FallbackCommand,
            "bun does not support dist-tag, falling back to npm dist-tag",
        );
        Npm::resolve_dist_tag(args)
    }
}

fn resolve_dist_tag(program: &str, base_args: &[&str], args: &DistTagCommand) -> CommandResolution {
    let mut cmd = CommandBuilder::new(program);
    for arg in base_args {
        cmd.arg(arg);
    }
    match args {
        DistTagCommand::List { package } => {
            cmd.arg("list");
            if let Some(package) = package {
                cmd.arg(package);
            }
        }
        DistTagCommand::Add { package_at_version, tag } => {
            cmd.arg("add").arg(package_at_version).arg(tag);
        }
        DistTagCommand::Rm { package, tag } => {
            cmd.arg("rm").arg(package).arg(tag);
        }
    }
    cmd.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        Resolution, resolve,
        test_utils::{bun, npm, parse_subcommand, pnpm, yarn},
    };

    fn list(package: &str) -> DistTagCommand {
        DistTagCommand::List { package: Some(package.to_string()) }
    }

    #[test]
    fn test_parser_accepts_list_alias() {
        let args = parse_subcommand::<DistTagCommand>(["ls", "my-package"]).unwrap();

        assert_eq!(args, DistTagCommand::List { package: Some("my-package".to_string()) });
    }

    #[test]
    fn test_npm_dist_tag_list() {
        let Resolution { outcome: CommandResolution::Run(command), .. } =
            resolve(&npm("11.0.0"), list("my-package"))
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["dist-tag", "list", "my-package"]);
    }

    #[test]
    fn test_pnpm_dist_tag_list() {
        let Resolution { outcome: CommandResolution::Run(command), .. } =
            resolve(&pnpm("10.0.0"), list("my-package"))
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["dist-tag", "list", "my-package"]);
    }

    #[test]
    fn test_yarn1_dist_tag_list() {
        let Resolution { outcome: CommandResolution::Run(command), .. } =
            resolve(&yarn("1.22.0"), list("my-package"))
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["tag", "list", "my-package"]);
    }

    #[test]
    fn test_yarn2_dist_tag_list() {
        let Resolution { outcome: CommandResolution::Run(command), .. } =
            resolve(&yarn("4.0.0"), list("my-package"))
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["npm", "tag", "list", "my-package"]);
    }

    #[test]
    fn test_bun_dist_tag_list_falls_back_to_npm() {
        let Resolution { outcome: CommandResolution::Run(command), diagnostics } =
            resolve(&bun("1.3.11"), list("my-package"))
        else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["dist-tag", "list", "my-package"]);
        assert_eq!(
            diagnostics[0].message,
            "bun does not support dist-tag, falling back to npm dist-tag"
        );
    }

    #[test]
    fn test_dist_tag_add() {
        let Resolution { outcome: CommandResolution::Run(command), .. } = resolve(
            &npm("11.0.0"),
            DistTagCommand::Add {
                package_at_version: "my-package@1.0.0".into(),
                tag: "beta".into(),
            },
        ) else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["dist-tag", "add", "my-package@1.0.0", "beta"]);
    }

    #[test]
    fn test_dist_tag_rm() {
        let Resolution { outcome: CommandResolution::Run(command), .. } = resolve(
            &npm("11.0.0"),
            DistTagCommand::Rm { package: "my-package".into(), tag: "beta".into() },
        ) else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["dist-tag", "rm", "my-package", "beta"]);
    }
}
