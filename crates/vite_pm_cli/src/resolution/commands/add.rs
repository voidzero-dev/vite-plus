use vite_pm_cli_macros::pm_args;

use super::parse_positive_usize;
use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

#[pm_args]
#[derive(clap::Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct AddArgs {
    #[command(flatten)]
    pub(crate) save_dependency: SaveDependencyArgs,

    /// Save exact version rather than semver range
    #[arg(short = 'E', long)]
    pub(crate) save_exact: bool,

    /// Save the new dependency to the specified catalog name
    #[arg(long, value_name = "CATALOG_NAME", not_supported(npm, yarn, bun))]
    pub(crate) save_catalog_name: Option<String>,

    /// Save the new dependency to the default catalog
    #[arg(long, not_supported(npm, yarn, bun))]
    pub(crate) save_catalog: bool,

    /// A list of package names allowed to run postinstall
    #[arg(long, value_name = "NAMES", not_supported(npm, yarn, bun))]
    pub(crate) allow_build: Option<String>,

    /// Filter packages in monorepo (can be used multiple times)
    #[arg(long, value_name = "PATTERN", not_supported(bun))]
    pub(crate) filter: Vec<String>,

    /// Add to workspace root
    #[arg(short = 'w', long, not_supported(bun))]
    pub(crate) workspace_root: bool,

    /// Only add if package exists in workspace (pnpm-specific)
    #[arg(long, not_supported(npm, yarn, bun))]
    pub(crate) workspace: bool,

    /// Install globally
    #[arg(short = 'g', long)]
    pub(crate) global: bool,

    /// Node.js version to use for global installation (only with -g)
    #[arg(long, requires = "global")]
    pub(crate) node: Option<String>,

    /// Number of global package installs to run in parallel (only with -g)
    #[arg(long, requires = "global", value_parser = parse_positive_usize)]
    pub(crate) concurrency: Option<usize>,

    /// Packages to add
    #[arg(required = true)]
    pub(crate) packages: Vec<String>,

    /// Additional arguments to pass through to the package manager
    #[arg(last = true, allow_hyphen_values = true)]
    pub(crate) pass_through_args: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SaveDependencyTarget {
    Production,
    Dev,
    Peer,
    Optional,
}

#[derive(clap::Args, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[group(id = "save_dependency_target", multiple = false)]
pub(crate) struct SaveDependencyArgs {
    /// Save to `dependencies` (default)
    #[arg(short = 'P', long)]
    pub(crate) save_prod: bool,

    /// Save to `devDependencies`
    #[arg(short = 'D', long)]
    pub(crate) save_dev: bool,

    /// Save to `peerDependencies` and `devDependencies`
    #[arg(long)]
    pub(crate) save_peer: bool,

    /// Save to `optionalDependencies`
    #[arg(short = 'O', long)]
    pub(crate) save_optional: bool,
}

impl SaveDependencyArgs {
    pub(crate) fn target(self) -> Option<SaveDependencyTarget> {
        if self.save_dev {
            Some(SaveDependencyTarget::Dev)
        } else if self.save_peer {
            Some(SaveDependencyTarget::Peer)
        } else if self.save_optional {
            Some(SaveDependencyTarget::Optional)
        } else if self.save_prod {
            Some(SaveDependencyTarget::Production)
        } else {
            None
        }
    }

    #[cfg(test)]
    fn dev() -> Self {
        Self { save_dev: true, ..Default::default() }
    }
}

impl Resolve<AddArgs> for Pnpm {
    fn resolve(&self, args: &AddArgs, _diag: &mut Diagnostics) -> CommandResolution {
        if args.global {
            return Npm::resolve_add(args);
        }
        let mut cmd = CommandBuilder::new("pnpm");
        cmd.repeated("--filter", args.filter.iter());
        cmd.arg("add")
            .arg_if("--workspace-root", args.workspace_root)
            .arg_if("--workspace", args.workspace);
        match args.save_dependency.target() {
            Some(SaveDependencyTarget::Production) => {
                cmd.arg("--save-prod");
            }
            Some(SaveDependencyTarget::Dev) => {
                cmd.arg("--save-dev");
            }
            Some(SaveDependencyTarget::Peer) => {
                cmd.arg("--save-peer");
            }
            Some(SaveDependencyTarget::Optional) => {
                cmd.arg("--save-optional");
            }
            None => {}
        }
        cmd.arg_if("--save-exact", args.save_exact);
        if let Some(name) = &args.save_catalog_name {
            if name.is_empty() {
                cmd.arg("--save-catalog");
            } else {
                cmd.arg(vite_str::format!("--save-catalog-name={name}"));
            }
        }
        cmd.arg_if("--save-catalog", args.save_catalog);
        if let Some(allow_build) = &args.allow_build {
            cmd.arg(vite_str::format!("--allow-build={allow_build}"));
        }
        cmd.extend(args.pass_through_args.iter()).extend(args.packages.iter());
        cmd.into()
    }
}

impl Npm {
    fn resolve_add(args: &AddArgs) -> CommandResolution {
        let mut cmd = CommandBuilder::new("npm");
        if args.global {
            cmd.arg("install")
                .arg("--global")
                .extend(args.pass_through_args.iter())
                .extend(args.packages.iter());
            return cmd.into();
        }

        cmd.arg("install")
            .repeated("--workspace", args.filter.iter())
            .arg_if("--include-workspace-root", args.workspace_root);
        match args.save_dependency.target() {
            Some(SaveDependencyTarget::Production) => {
                cmd.arg("--save");
            }
            Some(SaveDependencyTarget::Dev) => {
                cmd.arg("--save-dev");
            }
            Some(SaveDependencyTarget::Peer) => {
                cmd.arg("--save-peer");
            }
            Some(SaveDependencyTarget::Optional) => {
                cmd.arg("--save-optional");
            }
            None => {}
        }
        cmd.arg_if("--save-exact", args.save_exact)
            .extend(args.pass_through_args.iter())
            .extend(args.packages.iter());
        cmd.into()
    }
}

impl Resolve<AddArgs> for Npm {
    fn resolve(&self, args: &AddArgs, _diag: &mut Diagnostics) -> CommandResolution {
        Self::resolve_add(args)
    }
}

impl Resolve<AddArgs> for Yarn {
    fn resolve(&self, args: &AddArgs, _diag: &mut Diagnostics) -> CommandResolution {
        if args.global {
            return Npm::resolve_add(args);
        }

        let mut cmd = CommandBuilder::new("yarn");
        if !args.filter.is_empty() {
            cmd.arg("workspaces").arg("foreach").arg("--all");
            cmd.repeated("--include", args.filter.iter());
        }
        cmd.arg("add");
        match args.save_dependency.target() {
            Some(SaveDependencyTarget::Dev) => {
                cmd.arg("--dev");
            }
            Some(SaveDependencyTarget::Peer) => {
                cmd.arg("--peer");
            }
            Some(SaveDependencyTarget::Optional) => {
                cmd.arg("--optional");
            }
            Some(SaveDependencyTarget::Production) | None => {}
        }
        cmd.arg_if("--exact", args.save_exact)
            .extend(args.pass_through_args.iter())
            .extend(args.packages.iter());
        cmd.into()
    }
}

impl Resolve<AddArgs> for Bun {
    fn resolve(&self, args: &AddArgs, _diag: &mut Diagnostics) -> CommandResolution {
        if args.global {
            return Npm::resolve_add(args);
        }
        let mut cmd = CommandBuilder::new("bun");
        cmd.arg("add");
        match args.save_dependency.target() {
            Some(SaveDependencyTarget::Dev) => {
                cmd.arg("--dev");
            }
            Some(SaveDependencyTarget::Peer) => {
                cmd.arg("--peer");
            }
            Some(SaveDependencyTarget::Optional) => {
                cmd.arg("--optional");
            }
            Some(SaveDependencyTarget::Production) | None => {}
        }
        cmd.arg_if("--exact", args.save_exact)
            .extend(args.pass_through_args.iter())
            .extend(args.packages.iter());
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

    fn add_args(packages: &[&str]) -> AddArgs {
        AddArgs {
            packages: packages.iter().map(ToString::to_string).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn test_pnpm_basic_add() {
        let resolution = resolve(&pnpm("10.0.0"), add_args(&["react"]));
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["add", "react"]);
    }

    #[test]
    fn test_global_add_uses_npm() {
        let mut options = add_args(&["typescript"]);
        options.global = true;
        options.pass_through_args =
            vec!["--registry".to_string(), "https://registry.example".to_string()];
        let resolution = resolve(&pnpm("10.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(
            command.args,
            vec!["install", "--global", "--registry", "https://registry.example", "typescript"]
        );
    }

    #[test]
    fn test_pnpm_add_with_filter() {
        let mut options = add_args(&["react"]);
        options.filter = vec!["app".to_string()];
        let resolution = resolve(&pnpm("10.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["--filter", "app", "add", "react"]);
    }

    #[test]
    fn test_pnpm_add_with_save_catalog_name() {
        let mut options = add_args(&["react"]);
        options.filter = vec!["app".to_string()];
        options.save_catalog_name = Some("react18".to_string());
        let resolution = resolve(&pnpm("10.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(
            command.args,
            vec!["--filter", "app", "add", "--save-catalog-name=react18", "react"]
        );
    }

    #[test]
    fn test_pnpm_add_with_save_catalog_name_and_empty_name() {
        let mut options = add_args(&["react"]);
        options.filter = vec!["app".to_string()];
        options.save_catalog_name = Some(String::new());
        let resolution = resolve(&pnpm("10.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["--filter", "app", "add", "--save-catalog", "react"]);
    }

    #[test]
    fn test_pnpm_add_with_save_catalog() {
        let mut options = add_args(&["react"]);
        options.filter = vec!["app".to_string()];
        options.save_catalog = true;
        let resolution = resolve(&pnpm("10.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["--filter", "app", "add", "--save-catalog", "react"]);
    }

    #[test]
    fn test_pnpm_add_with_filter_and_workspace_root() {
        let mut options = add_args(&["react"]);
        options.filter = vec!["app".to_string()];
        options.workspace_root = true;
        let resolution = resolve(&pnpm("10.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["--filter", "app", "add", "--workspace-root", "react"]);
    }

    #[test]
    fn test_pnpm_add_workspace_root() {
        let mut options = add_args(&["typescript"]);
        options.save_dependency = SaveDependencyArgs::dev();
        options.workspace_root = true;
        let resolution = resolve(&pnpm("10.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["add", "--workspace-root", "--save-dev", "typescript"]);
    }

    #[test]
    fn test_pnpm_add_workspace_only() {
        let mut options = add_args(&["@myorg/utils"]);
        options.filter = vec!["app".to_string()];
        options.workspace = true;
        let resolution = resolve(&pnpm("10.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["--filter", "app", "add", "--workspace", "@myorg/utils"]);
    }

    #[test]
    fn save_dependency_flags_are_mutually_exclusive() {
        let error = parse_args::<AddArgs>(["--save-dev", "--save-optional", "react"]).unwrap_err();

        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn save_dependency_parser_sets_selected_flag() {
        let args = parse_args::<AddArgs>(["--save-peer", "react"]).unwrap();

        assert_eq!(args.save_dependency.target(), Some(SaveDependencyTarget::Peer));
        assert_eq!(args.packages, vec!["react"]);
    }

    #[test]
    fn save_dependency_parser_sets_production_flag() {
        let args = parse_args::<AddArgs>(["--save-prod", "react"]).unwrap();

        assert_eq!(args.save_dependency.target(), Some(SaveDependencyTarget::Production));
        assert_eq!(args.packages, vec!["react"]);
    }

    #[test]
    fn save_dependency_parser_accepts_short_flags() {
        let args = parse_args::<AddArgs>(["-D", "react"]).unwrap();

        assert_eq!(args.save_dependency.target(), Some(SaveDependencyTarget::Dev));
        assert_eq!(args.packages, vec!["react"]);
    }

    #[test]
    fn test_yarn_basic_add() {
        let resolution = resolve(&yarn("1.22.22"), add_args(&["react"]));
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["add", "react"]);
    }

    #[test]
    fn test_yarn_add_with_workspace() {
        let mut options = add_args(&["react"]);
        options.filter = vec!["app".to_string()];
        let resolution = resolve(&yarn("1.22.22"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(
            command.args,
            vec!["workspaces", "foreach", "--all", "--include", "app", "add", "react"]
        );
    }

    #[test]
    fn test_yarn_add_workspace_root() {
        let mut options = add_args(&["typescript"]);
        options.save_dependency = SaveDependencyArgs::dev();
        options.workspace_root = true;
        let resolution = resolve(&yarn("1.22.22"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["add", "--dev", "typescript"]);
        assert!(resolution.diagnostics.is_empty());
    }

    #[test]
    fn test_npm_basic_add() {
        let resolution = resolve(&npm("11.0.0"), add_args(&["react"]));
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["install", "react"]);
    }

    #[test]
    fn test_npm_add_with_workspace() {
        let mut options = add_args(&["react"]);
        options.filter = vec!["app".to_string()];
        let resolution = resolve(&npm("11.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["install", "--workspace", "app", "react"]);
    }

    #[test]
    fn test_npm_add_workspace_root() {
        let mut options = add_args(&["typescript"]);
        options.workspace_root = true;
        let resolution = resolve(&npm("11.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["install", "--include-workspace-root", "typescript"]);
    }

    #[test]
    fn test_npm_add_multiple_workspaces() {
        let mut options = add_args(&["lodash"]);
        options.filter = vec!["app".to_string(), "web".to_string()];
        let resolution = resolve(&npm("11.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(
            command.args,
            vec!["install", "--workspace", "app", "--workspace", "web", "lodash"]
        );
    }

    #[test]
    fn test_npm_add_multiple_workspaces_and_workspace_root() {
        let mut options = add_args(&["lodash"]);
        options.filter = vec!["app".to_string(), "web".to_string()];
        options.workspace_root = true;
        let resolution = resolve(&npm("11.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(
            command.args,
            vec![
                "install",
                "--workspace",
                "app",
                "--workspace",
                "web",
                "--include-workspace-root",
                "lodash"
            ]
        );
    }

    #[test]
    fn test_pnpm_add_with_allow_build() {
        let mut options = add_args(&["react"]);
        options.allow_build = Some("react,napi".to_string());
        let resolution = resolve(&pnpm("10.0.0"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["add", "--allow-build=react,napi", "react"]);
    }

    #[test]
    fn test_bun_basic_add() {
        let resolution = resolve(&bun("1.3.11"), add_args(&["react"]));
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["add", "react"]);
    }

    #[test]
    fn yarn_drops_workspace_root_without_warning() {
        let mut args = add_args(&["react"]);
        args.workspace_root = true;

        let classic = resolve(&yarn("1.22.22"), args.clone());
        let CommandResolution::Run(classic_command) = classic.outcome else {
            panic!("expected command resolution");
        };
        let berry = resolve(&yarn("4.1.0"), args);
        let CommandResolution::Run(berry_command) = berry.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(classic_command.program, "yarn");
        assert_eq!(classic_command.args, vec!["add", "react"]);
        assert_eq!(berry_command.program, "yarn");
        assert_eq!(berry_command.args, vec!["add", "react"]);
        assert!(classic.diagnostics.is_empty());
        assert!(berry.diagnostics.is_empty());
    }

    #[test]
    fn bun_warns_for_unsupported_options() {
        let mut options = add_args(&["react"]);
        options.filter = vec!["app".to_string()];
        options.workspace_root = true;
        options.workspace = true;
        options.save_catalog = true;
        options.allow_build = Some("react".to_string());
        let resolution = resolve(&bun("1.3.11"), options);
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.args, vec!["add", "react"]);
        assert_eq!(
            resolution.diagnostics.iter().map(|entry| entry.message.as_str()).collect::<Vec<_>>(),
            vec![
                "bun does not support --save-catalog.",
                "bun does not support --allow-build.",
                "bun does not support --filter.",
                "bun does not support --workspace-root.",
                "bun does not support --workspace."
            ]
        );
    }
}
