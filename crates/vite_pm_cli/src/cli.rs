//! Typed clap surface for every package-manager command.
//!
//! [`PackageManagerCommand`] is flattened into both the global CLI and the
//! local NAPI CLI. Each variant owns the same typed argument value that is
//! later diagnosed and resolved for the detected package manager. There is no
//! parser-to-options compatibility layer between clap and dispatch.

use clap::Subcommand;

use crate::{
    Error, PackageManager,
    resolution::{
        AddArgs, ApproveBuildsArgs, AuditArgs, CacheArgs, ConfigCommand, DedupeArgs, DeprecateArgs,
        DistTagCommand, DlxArgs, FundArgs, InstallArgs, LinkArgs, ListArgs, LoginArgs, LogoutArgs,
        OutdatedArgs, OutdatedFormat, OwnerCommand, PackArgs, PingArgs, PruneArgs, PublishArgs,
        RebuildArgs, RemoveArgs, Resolution, SearchArgs, StageCommand, TokenCommand, UnlinkArgs,
        UpdateArgs, VersionArgs, ViewArgs, WhoamiArgs, WhyArgs,
        resolve_for_manager as resolve_args_for_manager,
    },
};

/// A parsed package-manager command.
///
/// The variants intentionally hold the production resolver argument types
/// directly. Aliases match the existing public `vp` command surface.
#[derive(Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum PackageManagerCommand {
    /// Install all dependencies, or add packages if package names are provided
    #[command(visible_alias = "i")]
    Install(InstallArgs),

    /// Add packages to dependencies
    Add(AddArgs),

    /// Remove packages from dependencies
    #[command(visible_alias = "rm", visible_alias = "un", visible_alias = "uninstall")]
    Remove(RemoveArgs),

    /// Update packages to their latest versions
    #[command(visible_alias = "up")]
    Update(UpdateArgs),

    /// Deduplicate dependencies
    Dedupe(DedupeArgs),

    /// Check for outdated packages
    Outdated(OutdatedArgs),

    /// Show why a package is installed
    #[command(visible_alias = "explain")]
    Why(WhyArgs),

    /// View package information from the registry
    #[command(visible_alias = "view", visible_alias = "show")]
    Info(ViewArgs),

    /// Link packages for local development
    #[command(visible_alias = "ln")]
    Link(LinkArgs),

    /// Unlink packages
    Unlink(UnlinkArgs),

    /// Download and execute a package without installing it globally
    Dlx(DlxArgs),

    /// Forward a command to the package manager
    #[command(subcommand)]
    Pm(PmCommand),
}

/// Commands nested below `vp pm`.
#[derive(Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum PmCommand {
    /// Approve dependency lifecycle scripts (install/postinstall) to run
    #[command(name = "approve-builds")]
    ApproveBuilds(ApproveBuildsArgs),

    /// Remove unnecessary packages
    Prune(PruneArgs),

    /// Create a tarball of the package
    Pack(PackArgs),

    /// List installed packages
    #[command(visible_alias = "ls")]
    List(ListArgs),

    /// View package information from the registry
    #[command(visible_alias = "info", visible_alias = "show")]
    View(ViewArgs),

    /// Forward the native package version command
    Version(VersionArgs),

    /// Publish package to registry
    Publish(PublishArgs),

    /// Stage a package for publishing (npm staged publishing workflow)
    #[command(subcommand)]
    Stage(StageCommand),

    /// Manage package owners
    #[command(subcommand, visible_alias = "author")]
    Owner(OwnerCommand),

    /// Manage package cache
    Cache(CacheArgs),

    /// Manage package manager configuration
    #[command(subcommand, visible_alias = "c")]
    Config(ConfigCommand),

    /// Log in to a registry
    #[command(visible_alias = "adduser")]
    Login(LoginArgs),

    /// Log out from a registry
    Logout(LogoutArgs),

    /// Show the current logged-in user
    Whoami(WhoamiArgs),

    /// Manage authentication tokens
    #[command(subcommand)]
    Token(TokenCommand),

    /// Run a security audit
    Audit(AuditArgs),

    /// Manage distribution tags
    #[command(name = "dist-tag", subcommand)]
    DistTag(DistTagCommand),

    /// Deprecate a package version
    Deprecate(DeprecateArgs),

    /// Search for packages in the registry
    Search(SearchArgs),

    /// Rebuild native modules
    #[command(visible_alias = "rb")]
    Rebuild(RebuildArgs),

    /// Show funding information for installed packages
    Fund(FundArgs),

    /// Ping the registry
    Ping(PingArgs),
}

/// A package-manager command handled by Vite+'s managed global-package store.
///
/// This borrowed projection keeps the clap argument layout private while
/// exposing the small set of values required by the global CLI dispatcher.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManagedGlobalCommand<'a> {
    /// Install packages into the managed global store.
    Install {
        packages: &'a [String],
        node: Option<&'a str>,
        force: bool,
        concurrency: Option<usize>,
    },
    /// Remove packages from the managed global store.
    Remove { packages: &'a [String], dry_run: bool },
    /// Update packages in the managed global store.
    Update {
        packages: &'a [String],
        concurrency: Option<usize>,
        reinstall_node_mismatch: bool,
        ignore_node_mismatch: bool,
    },
    /// Check managed global packages for updates.
    Outdated {
        packages: &'a [String],
        long: bool,
        format: Option<OutdatedFormat>,
        concurrency: Option<usize>,
    },
    /// List packages in the managed global store.
    List { json: bool, pattern: Option<&'a str> },
}

impl PackageManagerCommand {
    /// Build a `dlx` command for callers that do not use the clap parser.
    #[must_use]
    pub fn dlx(package: Vec<String>, shell_mode: bool, silent: bool, args: Vec<String>) -> Self {
        Self::Dlx(DlxArgs { package, shell_mode, silent, args })
    }

    /// Resolve this parsed command for a detected package manager.
    ///
    /// `install <packages>` normalizes directly into [`AddArgs`]. This is the
    /// only command whose typed clap shape selects between two resolvers.
    pub(crate) fn resolve_for_manager(self, manager: &PackageManager) -> Result<Resolution, Error> {
        match self {
            Self::Install(args) if !args.packages.is_empty() => {
                resolve_args_for_manager(manager, args.into_add_args())
            }
            Self::Install(args) => resolve_args_for_manager(manager, args),
            Self::Add(args) => resolve_args_for_manager(manager, args),
            Self::Remove(args) => resolve_args_for_manager(manager, args),
            Self::Update(args) => resolve_args_for_manager(manager, args),
            Self::Dedupe(args) => resolve_args_for_manager(manager, args),
            Self::Outdated(args) => resolve_args_for_manager(manager, args),
            Self::Why(args) => resolve_args_for_manager(manager, args),
            Self::Info(args) => resolve_args_for_manager(manager, args),
            Self::Link(args) => resolve_args_for_manager(manager, args),
            Self::Unlink(args) => resolve_args_for_manager(manager, args),
            Self::Dlx(args) => resolve_args_for_manager(manager, args),
            Self::Pm(command) => command.resolve_for_manager(manager),
        }
    }

    /// Whether this command must bypass normal dispatch and use Vite+'s
    /// managed global-package store.
    #[must_use]
    pub fn is_managed_global(&self) -> bool {
        self.managed_global_command().is_some()
    }

    /// Borrow the values needed by Vite+'s managed global-package dispatcher.
    ///
    /// Returns `None` when normal package-manager dispatch should be used.
    #[must_use]
    pub fn managed_global_command(&self) -> Option<ManagedGlobalCommand<'_>> {
        match self {
            Self::Install(args) if args.global => Some(ManagedGlobalCommand::Install {
                packages: &args.packages,
                node: args.node.as_deref(),
                force: args.force,
                concurrency: args.concurrency,
            }),
            Self::Add(args) if args.global => Some(ManagedGlobalCommand::Install {
                packages: &args.packages,
                node: args.node.as_deref(),
                force: false,
                concurrency: args.concurrency,
            }),
            Self::Remove(args) if args.global => Some(ManagedGlobalCommand::Remove {
                packages: &args.packages,
                dry_run: args.dry_run,
            }),
            Self::Update(args) if args.global => Some(ManagedGlobalCommand::Update {
                packages: &args.packages,
                concurrency: args.concurrency,
                reinstall_node_mismatch: args.reinstall_node_mismatch,
                ignore_node_mismatch: args.ignore_node_mismatch,
            }),
            Self::Outdated(args) if args.global => Some(ManagedGlobalCommand::Outdated {
                packages: &args.packages,
                long: args.long,
                format: args.format,
                concurrency: args.concurrency,
            }),
            Self::Pm(PmCommand::List(args)) if args.global => Some(ManagedGlobalCommand::List {
                json: args.json,
                pattern: args.pattern.as_deref(),
            }),
            _ => None,
        }
    }

    /// Whether normal informational output would corrupt or pollute the
    /// command's requested output format.
    #[must_use]
    pub fn is_quiet_or_machine_readable(&self) -> bool {
        match self {
            Self::Install(args) => args.silent,
            Self::Dlx(args) => args.silent,
            Self::Outdated(args) => {
                matches!(args.format, Some(OutdatedFormat::Json | OutdatedFormat::List))
            }
            Self::Why(args) => args.json || args.parseable,
            Self::Info(args) => args.json,
            Self::Pm(command) => command.is_quiet_or_machine_readable(),
            _ => false,
        }
    }

    /// Whether compatibility diagnostics should be rendered for this command.
    ///
    /// Machine-readable output uses stdout while diagnostics use stderr, so
    /// only explicit silent modes suppress them.
    pub(crate) fn should_render_diagnostics(&self) -> bool {
        match self {
            Self::Install(args) => !args.silent,
            Self::Dlx(args) => !args.silent,
            _ => true,
        }
    }

    /// Return the install command's `--silent` value when this is `install`.
    ///
    /// The global CLI uses this before dispatch to decide whether to print its
    /// managed-runtime header.
    #[must_use]
    pub fn install_silent(&self) -> Option<bool> {
        match self {
            Self::Install(args) => Some(args.silent),
            _ => None,
        }
    }
}

impl PmCommand {
    fn resolve_for_manager(self, manager: &PackageManager) -> Result<Resolution, Error> {
        match self {
            Self::ApproveBuilds(args) => resolve_args_for_manager(manager, args),
            Self::Prune(args) => resolve_args_for_manager(manager, args),
            Self::Pack(args) => resolve_args_for_manager(manager, args),
            Self::List(args) => resolve_args_for_manager(manager, args),
            Self::View(args) => resolve_args_for_manager(manager, args),
            Self::Version(args) => resolve_args_for_manager(manager, args),
            Self::Publish(args) => resolve_args_for_manager(manager, args),
            Self::Stage(args) => resolve_args_for_manager(manager, args),
            Self::Owner(args) => resolve_args_for_manager(manager, args),
            Self::Cache(args) => resolve_args_for_manager(manager, args),
            Self::Config(args) => resolve_args_for_manager(manager, args),
            Self::Login(args) => resolve_args_for_manager(manager, args),
            Self::Logout(args) => resolve_args_for_manager(manager, args),
            Self::Whoami(args) => resolve_args_for_manager(manager, args),
            Self::Token(args) => resolve_args_for_manager(manager, args),
            Self::Audit(args) => resolve_args_for_manager(manager, args),
            Self::DistTag(args) => resolve_args_for_manager(manager, args),
            Self::Deprecate(args) => resolve_args_for_manager(manager, args),
            Self::Search(args) => resolve_args_for_manager(manager, args),
            Self::Rebuild(args) => resolve_args_for_manager(manager, args),
            Self::Fund(args) => resolve_args_for_manager(manager, args),
            Self::Ping(args) => resolve_args_for_manager(manager, args),
        }
    }

    fn is_quiet_or_machine_readable(&self) -> bool {
        match self {
            Self::List(args) => args.json || args.parseable,
            Self::Pack(args) => args.json,
            Self::View(args) => args.json,
            Self::Version(args) => args.json,
            Self::Publish(args) => args.json,
            Self::Audit(args) => args.json,
            Self::Search(args) => args.json,
            Self::Fund(args) => args.json,
            Self::Config(args) => match args {
                ConfigCommand::List { json, .. }
                | ConfigCommand::Get { json, .. }
                | ConfigCommand::Set { json, .. } => *json,
                ConfigCommand::Delete { .. } => false,
            },
            Self::Token(args) => match args {
                TokenCommand::List { json, .. } | TokenCommand::Create { json, .. } => *json,
                TokenCommand::Revoke { .. } => false,
            },
            Self::Stage(args) => match args {
                StageCommand::Publish { json, .. }
                | StageCommand::List { json, .. }
                | StageCommand::View { json, .. } => *json,
                StageCommand::Download { .. }
                | StageCommand::Approve { .. }
                | StageCommand::Reject { .. } => false,
            },
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::{FromArgMatches, Subcommand};

    use super::*;
    use crate::{PackageManagerType, resolution::CommandResolution};

    fn parse(args: &[&str]) -> Result<PackageManagerCommand, clap::Error> {
        let command = PackageManagerCommand::augment_subcommands(clap::Command::new("vp"));
        let matches =
            command.try_get_matches_from(std::iter::once("vp").chain(args.iter().copied()))?;
        PackageManagerCommand::from_arg_matches(&matches)
    }

    fn package_manager(client: PackageManagerType, version: &str) -> PackageManager {
        let workspace_root = vite_path::current_dir().unwrap();
        PackageManager {
            client,
            version: version.into(),
            install_dir: workspace_root.join(".test-package-manager"),
        }
    }

    #[test]
    fn parses_top_level_aliases() {
        assert!(matches!(parse(&["i"]).unwrap(), PackageManagerCommand::Install(_)));
        assert!(matches!(parse(&["up"]).unwrap(), PackageManagerCommand::Update(_)));
        for alias in ["rm", "un", "uninstall"] {
            assert!(matches!(parse(&[alias, "react"]).unwrap(), PackageManagerCommand::Remove(_)));
        }
        assert!(matches!(parse(&["explain", "react"]).unwrap(), PackageManagerCommand::Why(_)));
        for alias in ["view", "show"] {
            assert!(matches!(parse(&[alias, "react"]).unwrap(), PackageManagerCommand::Info(_)));
        }
        assert!(matches!(parse(&["ln", "react"]).unwrap(), PackageManagerCommand::Link(_)));
    }

    #[test]
    fn parses_pm_aliases_and_nested_commands() {
        assert!(matches!(
            parse(&["pm", "ls"]).unwrap(),
            PackageManagerCommand::Pm(PmCommand::List(_))
        ));
        for alias in ["info", "show"] {
            assert!(matches!(
                parse(&["pm", alias, "react"]).unwrap(),
                PackageManagerCommand::Pm(PmCommand::View(_))
            ));
        }
        assert!(matches!(
            parse(&["pm", "author", "ls", "react"]).unwrap(),
            PackageManagerCommand::Pm(PmCommand::Owner(OwnerCommand::List { .. }))
        ));
        assert!(matches!(
            parse(&["pm", "c", "list"]).unwrap(),
            PackageManagerCommand::Pm(PmCommand::Config(ConfigCommand::List { .. }))
        ));
        assert!(matches!(
            parse(&["pm", "adduser"]).unwrap(),
            PackageManagerCommand::Pm(PmCommand::Login(_))
        ));
        assert!(matches!(
            parse(&["pm", "rb"]).unwrap(),
            PackageManagerCommand::Pm(PmCommand::Rebuild(_))
        ));
        assert!(matches!(
            parse(&["pm", "token", "ls"]).unwrap(),
            PackageManagerCommand::Pm(PmCommand::Token(TokenCommand::List { .. }))
        ));
        assert!(matches!(
            parse(&["pm", "dist-tag", "ls"]).unwrap(),
            PackageManagerCommand::Pm(PmCommand::DistTag(DistTagCommand::List { .. }))
        ));
        assert!(matches!(
            parse(&["pm", "stage", "ls"]).unwrap(),
            PackageManagerCommand::Pm(PmCommand::Stage(StageCommand::List { .. }))
        ));
    }

    #[test]
    fn install_uses_install_or_add_resolver_from_packages() {
        let manager = package_manager(PackageManagerType::Pnpm, "10.0.0");
        let install = parse(&["install", "--frozen-lockfile"]).unwrap();
        let add = parse(&["install", "-D", "react"]).unwrap();

        let CommandResolution::Run(install) =
            install.resolve_for_manager(&manager).unwrap().outcome
        else {
            panic!("expected install command");
        };
        let CommandResolution::Run(add) = add.resolve_for_manager(&manager).unwrap().outcome else {
            panic!("expected add command");
        };

        assert_eq!(install.args, vec!["install", "--frozen-lockfile"]);
        assert_eq!(add.args, vec!["add", "--save-dev", "react"]);
    }

    #[test]
    fn frozen_lockfile_flags_use_last_value() {
        let PackageManagerCommand::Install(first) =
            parse(&["install", "--frozen-lockfile", "--no-frozen-lockfile"]).unwrap()
        else {
            panic!("expected install command");
        };
        let PackageManagerCommand::Install(second) =
            parse(&["install", "--no-frozen-lockfile", "--frozen-lockfile"]).unwrap()
        else {
            panic!("expected install command");
        };

        assert!(!first.frozen_lockfile);
        assert!(first.no_frozen_lockfile);
        assert!(second.frozen_lockfile);
        assert!(!second.no_frozen_lockfile);
    }

    #[test]
    fn validates_managed_global_options() {
        assert!(parse(&["install", "-g"]).is_err());
        assert!(parse(&["add", "--node", "22", "react"]).is_err());

        let install =
            parse(&["install", "-g", "--node", "22", "--concurrency", "2", "tsx"]).unwrap();
        let add = parse(&["add", "-g", "--node", "22", "--concurrency", "2", "tsx"]).unwrap();

        assert!(install.is_managed_global());
        assert!(add.is_managed_global());
    }

    #[test]
    fn projects_managed_global_commands() {
        let install =
            parse(&["install", "-g", "--node", "22", "--force", "--concurrency", "2", "tsx"])
                .unwrap();
        let Some(ManagedGlobalCommand::Install { packages, node, force, concurrency }) =
            install.managed_global_command()
        else {
            panic!("expected managed install command");
        };
        assert_eq!(packages, &["tsx"]);
        assert_eq!(node, Some("22"));
        assert!(force);
        assert_eq!(concurrency, Some(2));

        let add = parse(&["add", "-g", "tsx"]).unwrap();
        assert!(matches!(
            add.managed_global_command(),
            Some(ManagedGlobalCommand::Install { force: false, .. })
        ));

        let remove = parse(&["remove", "-g", "--dry-run", "tsx"]).unwrap();
        assert!(matches!(
            remove.managed_global_command(),
            Some(ManagedGlobalCommand::Remove { packages, dry_run: true })
                if packages == ["tsx"]
        ));

        let update = parse(&["update", "-g", "--reinstall-node-mismatch", "tsx"]).unwrap();
        assert!(matches!(
            update.managed_global_command(),
            Some(ManagedGlobalCommand::Update {
                packages,
                reinstall_node_mismatch: true,
                ignore_node_mismatch: false,
                ..
            }) if packages == ["tsx"]
        ));

        let outdated = parse(&["outdated", "-g", "--format", "json", "tsx"]).unwrap();
        assert!(matches!(
            outdated.managed_global_command(),
            Some(ManagedGlobalCommand::Outdated {
                packages,
                format: Some(OutdatedFormat::Json),
                ..
            }) if packages == ["tsx"]
        ));

        let list = parse(&["pm", "list", "-g", "--json", "tsx"]).unwrap();
        assert!(matches!(
            list.managed_global_command(),
            Some(ManagedGlobalCommand::List { json: true, pattern: Some("tsx") })
        ));

        assert!(parse(&["install"]).unwrap().managed_global_command().is_none());
    }

    #[test]
    fn save_dependency_targets_are_exclusive() {
        assert!(parse(&["add", "-D", "-O", "react"]).is_err());
    }

    #[test]
    fn classifies_quiet_and_machine_readable_commands() {
        for args in [
            &["install", "--silent"][..],
            &["dlx", "--silent", "tsx"][..],
            &["outdated", "--format", "json"][..],
            &["why", "react", "--parseable"][..],
            &["info", "react", "--json"][..],
            &["pm", "list", "--json"][..],
            &["pm", "version", "patch", "--json"][..],
            &["pm", "config", "list", "--json"][..],
            &["pm", "token", "create", "--json"][..],
            &["pm", "stage", "list", "--json"][..],
        ] {
            assert!(parse(args).unwrap().is_quiet_or_machine_readable(), "{args:?}");
        }
        assert!(!parse(&["install"]).unwrap().is_quiet_or_machine_readable());
    }

    #[test]
    fn suppresses_diagnostics_only_for_explicit_silent_modes() {
        for args in [
            &["outdated", "--format", "json"][..],
            &["why", "react", "--parseable"][..],
            &["info", "react", "--json"][..],
            &["pm", "list", "--json"][..],
        ] {
            assert!(parse(args).unwrap().should_render_diagnostics(), "{args:?}");
        }

        for args in [&["install", "--silent"][..], &["dlx", "--silent", "tsx"][..]] {
            assert!(!parse(args).unwrap().should_render_diagnostics(), "{args:?}");
        }
    }

    #[test]
    fn version_forwards_native_args_and_detects_json() {
        let command =
            parse(&["pm", "version", "prerelease", "--json", "--", "--preid", "beta"]).unwrap();

        assert!(command.is_quiet_or_machine_readable());
        let PackageManagerCommand::Pm(PmCommand::Version(args)) = command else {
            panic!("expected version command");
        };
        assert_eq!(args.new_version.as_deref(), Some("prerelease"));
        assert!(args.json);
        assert_eq!(args.pass_through_args, ["--preid", "beta"]);
    }
}
