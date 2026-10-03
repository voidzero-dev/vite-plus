use vp_pm_cli_macros::pm_args;

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
    #[arg(long, value_name = "CATALOG_NAME", not_supported(npm, yarn, bun < "1.4"))]
    pub(crate) save_catalog_name: Option<String>,

    /// Save the new dependency to the default catalog
    #[arg(long, not_supported(npm, yarn, bun < "1.4"))]
    pub(crate) save_catalog: bool,

    /// A list of package names allowed to run postinstall
    #[arg(long, value_name = "NAMES", not_supported(npm, yarn, bun))]
    pub(crate) allow_build: Option<String>,

    /// Do not run lifecycle scripts
    #[arg(long)]
    pub(crate) ignore_scripts: bool,

    /// Do not install optionalDependencies
    #[arg(long, conflicts_with = "global", not_supported(yarn >= "2"))]
    pub(crate) no_optional: bool,

    /// Don't update lockfile
    #[arg(
        long,
        conflicts_with = "global",
        overrides_with = "no_frozen_lockfile",
        not_supported(npm, pnpm, yarn >= "2")
    )]
    pub(crate) frozen_lockfile: bool,

    /// Allow lockfile updates
    #[arg(
        long,
        conflicts_with = "global",
        overrides_with = "frozen_lockfile",
        not_supported(npm, pnpm, yarn)
    )]
    pub(crate) no_frozen_lockfile: bool,

    /// Only update lockfile, don't install
    #[arg(long, conflicts_with = "global", not_supported(yarn < "3"))]
    pub(crate) lockfile_only: bool,

    /// Use cached packages when available
    #[arg(long, conflicts_with = "global", not_supported(yarn >= "2", bun < "1.4.1"))]
    pub(crate) prefer_offline: bool,

    /// Only use packages already in cache
    #[arg(long, conflicts_with = "global", not_supported(yarn >= "2", bun < "1.4.1"))]
    pub(crate) offline: bool,

    /// Force reinstall all dependencies
    #[arg(short = 'f', long, conflicts_with = "global", not_supported(yarn >= "2"))]
    pub(crate) force: bool,

    /// Don't read or generate lockfile
    #[arg(long, conflicts_with = "global", not_supported(yarn >= "2", bun))]
    pub(crate) no_lockfile: bool,

    /// Create flat node_modules (pnpm only)
    #[arg(long, conflicts_with = "global", not_supported(npm, yarn, bun))]
    pub(crate) shamefully_hoist: bool,

    /// Suppress Vite+ output and enable native silent mode where supported
    #[arg(long, conflicts_with = "global", not_supported(yarn >= "2"))]
    pub(crate) silent: bool,

    /// Filter packages in monorepo (can be used multiple times)
    #[arg(long, value_name = "PATTERN", not_supported(yarn < "2", bun < "1.4"))]
    pub(crate) filter: Vec<String>,

    /// Add to workspace root
    #[arg(short = 'w', long, not_supported(yarn >= "2", bun))]
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
                cmd.arg(vt_str::format!("--save-catalog-name={name}"));
            }
        }
        cmd.arg_if("--save-catalog", args.save_catalog);
        if let Some(allow_build) = &args.allow_build {
            cmd.arg(vt_str::format!("--allow-build={allow_build}"));
        }
        cmd.arg_if("--ignore-scripts", args.ignore_scripts)
            .arg_if("--no-optional", args.no_optional)
            .arg_if("--lockfile-only", args.lockfile_only)
            .arg_if("--prefer-offline", args.prefer_offline)
            .arg_if("--offline", args.offline)
            .arg_if("--force", args.force)
            .arg_if("--no-lockfile", args.no_lockfile)
            .arg_if("--shamefully-hoist", args.shamefully_hoist)
            .arg_if("--silent", args.silent)
            .extend(args.pass_through_args.iter())
            .extend(args.packages.iter());
        cmd.into()
    }
}

impl Npm {
    fn resolve_add(args: &AddArgs) -> CommandResolution {
        let mut cmd = CommandBuilder::new("npm");
        if args.global {
            cmd.arg("install")
                .arg("--global")
                .arg_if("--ignore-scripts", args.ignore_scripts)
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
            .arg_if("--ignore-scripts", args.ignore_scripts)
            .arg_if("--omit=optional", args.no_optional)
            .arg_if("--package-lock-only", args.lockfile_only)
            .arg_if("--prefer-offline", args.prefer_offline)
            .arg_if("--offline", args.offline)
            .arg_if("--force", args.force)
            .arg_if("--no-package-lock", args.no_lockfile);
        if args.silent {
            cmd.arg("--loglevel").arg("silent");
        }
        cmd.extend(args.pass_through_args.iter()).extend(args.packages.iter());
        cmd.into()
    }
}

impl Resolve<AddArgs> for Npm {
    fn resolve(&self, args: &AddArgs, _diag: &mut Diagnostics) -> CommandResolution {
        Self::resolve_add(args)
    }
}

impl Resolve<AddArgs> for Yarn {
    fn resolve(&self, args: &AddArgs, diag: &mut Diagnostics) -> CommandResolution {
        if args.global {
            return Npm::resolve_add(args);
        }

        let mut cmd = CommandBuilder::new("yarn");
        if !args.filter.is_empty() {
            cmd.arg("workspaces").arg("foreach").arg("--all");
            cmd.repeated("--include", args.filter.iter());
        }
        cmd.arg("add").arg_if("-W", args.workspace_root && !self.is_berry());
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
        cmd.arg_if("--exact", args.save_exact);
        if self.is_berry() {
            Self::apply_berry_install_mode(&mut cmd, args.lockfile_only, args.ignore_scripts, diag);
        } else {
            cmd.arg_if("--frozen-lockfile", args.frozen_lockfile)
                .arg_if("--ignore-scripts", args.ignore_scripts)
                .arg_if("--ignore-optional", args.no_optional)
                .arg_if("--prefer-offline", args.prefer_offline)
                .arg_if("--offline", args.offline)
                .arg_if("--force", args.force)
                .arg_if("--no-lockfile", args.no_lockfile)
                .arg_if("--silent", args.silent);
        }
        cmd.extend(args.pass_through_args.iter()).extend(args.packages.iter());
        cmd.into()
    }
}

impl Resolve<AddArgs> for Bun {
    fn resolve(&self, args: &AddArgs, _diag: &mut Diagnostics) -> CommandResolution {
        if args.global {
            return Npm::resolve_add(args);
        }
        let mut cmd = CommandBuilder::new("bun");
        cmd.arg("add").repeated("--filter", args.filter.iter());
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
        if let Some(name) = &args.save_catalog_name {
            if name.is_empty() {
                cmd.arg("--catalog");
            } else {
                cmd.arg(vt_str::format!("--catalog={name}"));
            }
        }
        cmd.arg_if("--exact", args.save_exact)
            .arg_if("--catalog", args.save_catalog)
            .arg_if("--ignore-scripts", args.ignore_scripts)
            .arg_if("--lockfile-only", args.lockfile_only)
            .arg_if("--prefer-offline", args.prefer_offline)
            .arg_if("--offline", args.offline)
            .arg_if("--force", args.force)
            .arg_if("--silent", args.silent);
        if args.no_optional {
            cmd.arg("--omit").arg("optional");
        }
        if args.no_frozen_lockfile {
            cmd.arg("--no-frozen-lockfile");
        } else {
            cmd.arg_if("--frozen-lockfile", args.frozen_lockfile);
        }
        cmd.extend(args.pass_through_args.iter()).extend(args.packages.iter());
        cmd.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        DiagnosticKind, resolve,
        test_utils::{bun, expect_run, expect_unsupported, npm, parse_args, pnpm, yarn},
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
        let command = expect_run(resolution.outcome);

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
        let command = expect_run(resolution.outcome);

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
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["--filter", "app", "add", "react"]);
    }

    #[test]
    fn test_pnpm_add_with_save_catalog_name() {
        let mut options = add_args(&["react"]);
        options.filter = vec!["app".to_string()];
        options.save_catalog_name = Some("react18".to_string());
        let resolution = resolve(&pnpm("10.0.0"), options);
        let command = expect_run(resolution.outcome);

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
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["--filter", "app", "add", "--save-catalog", "react"]);
    }

    #[test]
    fn test_pnpm_add_with_save_catalog() {
        let mut options = add_args(&["react"]);
        options.filter = vec!["app".to_string()];
        options.save_catalog = true;
        let resolution = resolve(&pnpm("10.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["--filter", "app", "add", "--save-catalog", "react"]);
    }

    #[test]
    fn test_pnpm_add_with_filter_and_workspace_root() {
        let mut options = add_args(&["react"]);
        options.filter = vec!["app".to_string()];
        options.workspace_root = true;
        let resolution = resolve(&pnpm("10.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["--filter", "app", "add", "--workspace-root", "react"]);
    }

    #[test]
    fn test_pnpm_add_workspace_root() {
        let mut options = add_args(&["typescript"]);
        options.save_dependency = SaveDependencyArgs::dev();
        options.workspace_root = true;
        let resolution = resolve(&pnpm("10.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["add", "--workspace-root", "--save-dev", "typescript"]);
    }

    #[test]
    fn test_pnpm_add_workspace_only() {
        let mut options = add_args(&["@myorg/utils"]);
        options.filter = vec!["app".to_string()];
        options.workspace = true;
        let resolution = resolve(&pnpm("10.0.0"), options);
        let command = expect_run(resolution.outcome);

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
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["add", "react"]);
    }

    #[test]
    fn test_yarn_berry_add_with_workspace() {
        let mut options = add_args(&["react"]);
        options.filter = vec!["app".to_string()];
        let resolution = resolve(&yarn("4.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "yarn");
        assert_eq!(
            command.args,
            vec!["workspaces", "foreach", "--all", "--include", "app", "add", "react"]
        );
    }

    #[test]
    fn test_yarn_classic_rejects_filtered_add() {
        for filters in
            [vec!["app".to_string()], vec!["app-*".to_string(), "@scope/web".to_string()]]
        {
            let mut options = add_args(&["react"]);
            options.filter = filters;
            let resolution = resolve(&yarn("1.22.22"), options);

            expect_unsupported(resolution, &["yarn < 2 does not support --filter."]);
        }
    }

    #[test]
    fn test_yarn_classic_add_aggregates_unsupported_options() {
        let args =
            parse_args::<AddArgs>(["react", "--filter", "app", "--save-catalog", "--workspace"])
                .unwrap();
        expect_unsupported(
            resolve(&yarn("1.22.22"), args),
            &[
                "yarn does not support --save-catalog.",
                "yarn < 2 does not support --filter.",
                "yarn does not support --workspace.",
            ],
        );
        let args = parse_args::<AddArgs>(["react", "--", "--filter", "app"]).unwrap();
        let resolution = resolve(&yarn("1.22.22"), args);
        assert!(resolution.diagnostics.is_empty());
        assert_eq!(expect_run(resolution.outcome).args, vec!["add", "--filter", "app", "react"]);
    }

    #[test]
    fn test_yarn_add_workspace_root() {
        let mut options = add_args(&["typescript"]);
        options.save_dependency = SaveDependencyArgs::dev();
        options.workspace_root = true;
        let resolution = resolve(&yarn("1.22.22"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["add", "-W", "--dev", "typescript"]);
        assert!(resolution.diagnostics.is_empty());
    }

    #[test]
    fn test_npm_basic_add() {
        let resolution = resolve(&npm("11.0.0"), add_args(&["react"]));
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["install", "react"]);
    }

    #[test]
    fn test_npm_add_with_workspace() {
        let mut options = add_args(&["react"]);
        options.filter = vec!["app".to_string()];
        let resolution = resolve(&npm("11.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["install", "--workspace", "app", "react"]);
    }

    #[test]
    fn test_npm_add_workspace_root() {
        let mut options = add_args(&["typescript"]);
        options.workspace_root = true;
        let resolution = resolve(&npm("11.0.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["install", "--include-workspace-root", "typescript"]);
    }

    #[test]
    fn test_npm_add_multiple_workspaces() {
        let mut options = add_args(&["lodash"]);
        options.filter = vec!["app".to_string(), "web".to_string()];
        let resolution = resolve(&npm("11.0.0"), options);
        let command = expect_run(resolution.outcome);

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
        let command = expect_run(resolution.outcome);

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
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["add", "--allow-build=react,napi", "react"]);
    }

    #[test]
    fn yarn_add_frozen_lockfile_options_follow_native_support() {
        let args = parse_args::<AddArgs>(["--frozen-lockfile", "react"]).unwrap();
        let resolution = resolve(&yarn("1.22.22"), args.clone());
        assert!(resolution.diagnostics.is_empty());
        assert_eq!(expect_run(resolution.outcome).args, ["add", "--frozen-lockfile", "react"]);
        expect_unsupported(
            resolve(&yarn("2.0.0"), args),
            &["yarn >= 2 does not support --frozen-lockfile."],
        );
        for version in ["1.22.22", "2.0.0"] {
            let args = parse_args::<AddArgs>(["--no-frozen-lockfile", "react"]).unwrap();
            expect_unsupported(
                resolve(&yarn(version), args),
                &["yarn does not support --no-frozen-lockfile."],
            );
        }
    }

    #[test]
    fn yarn_lockfile_only_takes_priority_over_ignore_scripts() {
        let resolution = resolve(
            &yarn("4.0.0"),
            AddArgs { lockfile_only: true, ignore_scripts: true, ..add_args(&["react"]) },
        );
        let command = expect_run(resolution.outcome);
        assert_eq!(command.args, ["add", "--mode", "update-lockfile", "react"]);
        assert_eq!(resolution.diagnostics.len(), 1);
        assert_eq!(resolution.diagnostics[0].kind, DiagnosticKind::BehaviorChange);
    }

    #[test]
    fn add_install_options_do_not_change_managed_global_commands() {
        for flag in [
            "--no-optional",
            "--frozen-lockfile",
            "--no-frozen-lockfile",
            "--lockfile-only",
            "--prefer-offline",
            "--offline",
            "--force",
            "--no-lockfile",
            "--shamefully-hoist",
            "--silent",
        ] {
            let error = parse_args::<AddArgs>(["--global", flag, "react"]).unwrap_err();
            assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict, "{flag}");
        }
    }

    #[test]
    fn add_frozen_lockfile_flags_use_last_value() {
        for (first, last) in [
            ("--frozen-lockfile", "--no-frozen-lockfile"),
            ("--no-frozen-lockfile", "--frozen-lockfile"),
        ] {
            let args = parse_args::<AddArgs>([first, last, "react"]).unwrap();
            let command = expect_run(resolve(&bun("1.3.11"), args).outcome);
            assert_eq!(command.args, ["add", last, "react"]);
        }
    }

    #[test]
    fn test_bun_basic_add() {
        let resolution = resolve(&bun("1.3.11"), add_args(&["react"]));
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["add", "react"]);
    }

    #[test]
    fn yarn_berry_rejects_unsupported_workspace_root() {
        let mut args = add_args(&["react"]);
        args.workspace_root = true;
        let resolution = resolve(&yarn("4.1.0"), args);
        expect_unsupported(resolution, &["yarn >= 2 does not support --workspace-root."]);
    }

    #[test]
    fn bun_rejects_all_unsupported_options() {
        let mut options = add_args(&["react"]);
        options.filter = vec!["app".to_string()];
        options.workspace_root = true;
        options.workspace = true;
        options.save_catalog = true;
        options.allow_build = Some("react".to_string());
        options.prefer_offline = true;
        options.offline = true;
        let resolution = resolve(&bun("1.3.11"), options);
        expect_unsupported(
            resolution,
            &[
                "bun < 1.4 does not support --save-catalog.",
                "bun does not support --allow-build.",
                "bun < 1.4.1 does not support --prefer-offline.",
                "bun < 1.4.1 does not support --offline.",
                "bun < 1.4 does not support --filter.",
                "bun does not support --workspace-root.",
                "bun does not support --workspace.",
            ],
        );
    }

    #[test]
    fn bun_1_4_1_forwards_offline_options() {
        // https://bun.sh/blog/bun-v1.4.1
        let mut options = add_args(&["react"]);
        options.prefer_offline = true;
        options.offline = true;
        let resolution = resolve(&bun("1.4.1"), options);
        assert!(resolution.diagnostics.is_empty());
        assert_eq!(
            expect_run(resolution.outcome).args,
            vec!["add", "--prefer-offline", "--offline", "react"]
        );
    }

    #[test]
    fn bun_1_4_supports_named_catalog() {
        for (name, flag) in [("testing", "--catalog=testing"), ("", "--catalog")] {
            let mut options = add_args(&["react"]);
            options.save_catalog_name = Some(name.to_string());
            let resolution = resolve(&bun("1.4.0"), options);
            assert!(resolution.diagnostics.is_empty());
            assert_eq!(expect_run(resolution.outcome).args, vec!["add", flag, "react"]);
        }
    }

    #[test]
    fn bun_before_1_4_does_not_support_named_catalog() {
        let mut options = add_args(&["react"]);
        options.save_catalog_name = Some("testing".to_string());
        let resolution = resolve(&bun("1.3.14"), options);
        expect_unsupported(resolution, &["bun < 1.4 does not support --save-catalog-name."]);
    }

    #[test]
    fn bun_1_4_supports_filter_and_catalog() {
        let mut options = add_args(&["react"]);
        options.filter = vec!["app".to_string()];
        options.save_catalog = true;
        let resolution = resolve(&bun("1.4.0"), options);
        let command = expect_run(resolution.outcome);

        assert_eq!(command.program, "bun");
        assert_eq!(command.args, vec!["add", "--filter", "app", "--catalog", "react"]);
        assert!(resolution.diagnostics.is_empty());
    }
}
