//! Clap definitions for every package-manager subcommand.
//!
//! The top-level [`PackageManagerCommand`] enum is consumed by both the
//! global CLI and the local CLI binding via `#[command(flatten)]`, so any
//! flag added here appears identically in both surfaces.

use clap::Subcommand;
use vite_install::commands::{add::SaveDependencyType, outdated::Format};

/// All package-manager subcommands.
///
/// Variants intentionally mirror the original definitions in
/// `vite_global_cli/src/cli.rs`. Aliases (`i`, `up`, `rm`, `un`, `uninstall`,
/// `explain`, `view`, `show`, `ln`) are preserved so both CLIs accept the
/// same shorthands.
#[derive(Subcommand, Debug, Clone)]
pub enum PackageManagerCommand {
    /// Install all dependencies, or add packages if package names are provided
    #[command(visible_alias = "i")]
    Install {
        /// Do not install devDependencies
        #[arg(short = 'P', long)]
        prod: bool,

        /// Only install devDependencies (install) / Save to devDependencies (add)
        #[arg(short = 'D', long)]
        dev: bool,

        /// Do not install optionalDependencies
        #[arg(long)]
        no_optional: bool,

        /// Fail if lockfile needs to be updated (CI mode)
        #[arg(long, overrides_with = "no_frozen_lockfile")]
        frozen_lockfile: bool,

        /// Allow lockfile updates (opposite of --frozen-lockfile)
        #[arg(long, overrides_with = "frozen_lockfile")]
        no_frozen_lockfile: bool,

        /// Only update lockfile, don't install
        #[arg(long)]
        lockfile_only: bool,

        /// Use cached packages when available
        #[arg(long)]
        prefer_offline: bool,

        /// Only use packages already in cache
        #[arg(long)]
        offline: bool,

        /// Force reinstall all dependencies
        #[arg(short = 'f', long)]
        force: bool,

        /// Do not run lifecycle scripts
        #[arg(long)]
        ignore_scripts: bool,

        /// Don't read or generate lockfile
        #[arg(long)]
        no_lockfile: bool,

        /// Fix broken lockfile entries (pnpm and yarn@2+ only)
        #[arg(long)]
        fix_lockfile: bool,

        /// Create flat `node_modules` (pnpm only)
        #[arg(long)]
        shamefully_hoist: bool,

        /// Re-run resolution for peer dependency analysis (pnpm only)
        #[arg(long)]
        resolution_only: bool,

        /// Suppress output (silent mode)
        #[arg(long)]
        silent: bool,

        /// Filter packages in monorepo (can be used multiple times)
        #[arg(long, value_name = "PATTERN")]
        filter: Option<Vec<String>>,

        /// Install in workspace root only
        #[arg(short = 'w', long)]
        workspace_root: bool,

        /// Save exact version (only when adding packages)
        #[arg(short = 'E', long)]
        save_exact: bool,

        /// Save to peerDependencies (only when adding packages)
        #[arg(long)]
        save_peer: bool,

        /// Save to optionalDependencies (only when adding packages)
        #[arg(short = 'O', long)]
        save_optional: bool,

        /// Save the new dependency to the default catalog (only when adding packages)
        #[arg(long)]
        save_catalog: bool,

        /// Install globally (requires package names)
        #[arg(short = 'g', long, requires = "packages")]
        global: bool,

        /// Node.js version to use for global installation (only with -g)
        #[arg(long, requires = "global")]
        node: Option<String>,

        /// Packages to add (if provided, acts as `vp add`)
        #[arg(required = false)]
        packages: Option<Vec<String>>,

        /// Additional arguments to pass through to the package manager
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// Add packages to dependencies
    Add {
        /// Save to `dependencies` (default)
        #[arg(short = 'P', long)]
        save_prod: bool,

        /// Save to `devDependencies`
        #[arg(short = 'D', long)]
        save_dev: bool,

        /// Save to `peerDependencies` and `devDependencies`
        #[arg(long)]
        save_peer: bool,

        /// Save to `optionalDependencies`
        #[arg(short = 'O', long)]
        save_optional: bool,

        /// Save exact version rather than semver range
        #[arg(short = 'E', long)]
        save_exact: bool,

        /// Save the new dependency to the specified catalog name
        #[arg(long, value_name = "CATALOG_NAME")]
        save_catalog_name: Option<String>,

        /// Save the new dependency to the default catalog
        #[arg(long)]
        save_catalog: bool,

        /// A list of package names allowed to run postinstall
        #[arg(long, value_name = "NAMES")]
        allow_build: Option<String>,

        /// Filter packages in monorepo (can be used multiple times)
        #[arg(long, value_name = "PATTERN")]
        filter: Option<Vec<String>>,

        /// Add to workspace root
        #[arg(short = 'w', long)]
        workspace_root: bool,

        /// Only add if package exists in workspace (pnpm-specific)
        #[arg(long)]
        workspace: bool,

        /// Install globally
        #[arg(short = 'g', long)]
        global: bool,

        /// Node.js version to use for global installation (only with -g)
        #[arg(long, requires = "global")]
        node: Option<String>,

        /// Packages to add
        #[arg(required = true)]
        packages: Vec<String>,

        /// Additional arguments to pass through to the package manager
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// Remove packages from dependencies
    #[command(visible_alias = "rm", visible_alias = "un", visible_alias = "uninstall")]
    Remove {
        /// Only remove from `devDependencies` (pnpm-specific)
        #[arg(short = 'D', long)]
        save_dev: bool,

        /// Only remove from `optionalDependencies` (pnpm-specific)
        #[arg(short = 'O', long)]
        save_optional: bool,

        /// Only remove from `dependencies` (pnpm-specific)
        #[arg(short = 'P', long)]
        save_prod: bool,

        /// Filter packages in monorepo (can be used multiple times)
        #[arg(long, value_name = "PATTERN")]
        filter: Option<Vec<String>>,

        /// Remove from workspace root
        #[arg(short = 'w', long)]
        workspace_root: bool,

        /// Remove recursively from all workspace packages
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Remove global packages
        #[arg(short = 'g', long)]
        global: bool,

        /// Preview what would be removed without actually removing (only with -g)
        #[arg(long, requires = "global")]
        dry_run: bool,

        /// Packages to remove
        #[arg(required = true)]
        packages: Vec<String>,

        /// Additional arguments to pass through to the package manager
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// Update packages to their latest versions
    #[command(visible_alias = "up")]
    Update {
        /// Update to latest version (ignore semver range)
        #[arg(short = 'L', long)]
        latest: bool,

        /// Update global packages
        #[arg(short = 'g', long)]
        global: bool,

        /// Update recursively in all workspace packages
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Filter packages in monorepo (can be used multiple times)
        #[arg(long, value_name = "PATTERN")]
        filter: Option<Vec<String>>,

        /// Include workspace root
        #[arg(short = 'w', long)]
        workspace_root: bool,

        /// Update only devDependencies
        #[arg(short = 'D', long)]
        dev: bool,

        /// Update only dependencies (production)
        #[arg(short = 'P', long)]
        prod: bool,

        /// Interactive mode
        #[arg(short = 'i', long)]
        interactive: bool,

        /// Don't update optionalDependencies
        #[arg(long)]
        no_optional: bool,

        /// Update lockfile only, don't modify package.json
        #[arg(long)]
        no_save: bool,

        /// Only update if package exists in workspace (pnpm-specific)
        #[arg(long)]
        workspace: bool,

        /// Packages to update (optional - updates all if omitted)
        packages: Vec<String>,

        /// Additional arguments to pass through to the package manager
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// Deduplicate dependencies
    Dedupe {
        /// Check if deduplication would make changes
        #[arg(long)]
        check: bool,

        /// Additional arguments to pass through to the package manager
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// Check for outdated packages
    Outdated {
        /// Package name(s) to check
        packages: Vec<String>,

        /// Show extended information
        #[arg(long)]
        long: bool,

        /// Output format: table (default), list, or json
        #[arg(long, value_name = "FORMAT", value_parser = clap::value_parser!(Format))]
        format: Option<Format>,

        /// Check recursively across all workspaces
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Filter packages in monorepo
        #[arg(long, value_name = "PATTERN")]
        filter: Option<Vec<String>>,

        /// Include workspace root
        #[arg(short = 'w', long)]
        workspace_root: bool,

        /// Only production and optional dependencies
        #[arg(short = 'P', long)]
        prod: bool,

        /// Only dev dependencies
        #[arg(short = 'D', long)]
        dev: bool,

        /// Exclude optional dependencies
        #[arg(long)]
        no_optional: bool,

        /// Only show compatible versions
        #[arg(long)]
        compatible: bool,

        /// Sort results by field
        #[arg(long, value_name = "FIELD")]
        sort_by: Option<String>,

        /// Check globally installed packages
        #[arg(short = 'g', long)]
        global: bool,

        /// Additional arguments to pass through to the package manager
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// Show why a package is installed
    #[command(visible_alias = "explain")]
    Why {
        /// Package(s) to check
        #[arg(required = true)]
        packages: Vec<String>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Show extended information
        #[arg(long)]
        long: bool,

        /// Show parseable output
        #[arg(long)]
        parseable: bool,

        /// Check recursively across all workspaces
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Filter packages in monorepo
        #[arg(long, value_name = "PATTERN")]
        filter: Option<Vec<String>>,

        /// Check in workspace root
        #[arg(short = 'w', long)]
        workspace_root: bool,

        /// Only production dependencies
        #[arg(short = 'P', long)]
        prod: bool,

        /// Only dev dependencies
        #[arg(short = 'D', long)]
        dev: bool,

        /// Limit tree depth
        #[arg(long)]
        depth: Option<u32>,

        /// Exclude optional dependencies
        #[arg(long)]
        no_optional: bool,

        /// Check globally installed packages
        #[arg(short = 'g', long)]
        global: bool,

        /// Exclude peer dependencies
        #[arg(long)]
        exclude_peers: bool,

        /// Use a finder function defined in .pnpmfile.cjs
        #[arg(long, value_name = "FINDER_NAME")]
        find_by: Option<String>,

        /// Additional arguments to pass through to the package manager
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// View package information from the registry
    #[command(visible_alias = "view", visible_alias = "show")]
    Info {
        /// Package name with optional version
        #[arg(required = true)]
        package: String,

        /// Specific field to view
        field: Option<String>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Additional arguments to pass through to the package manager
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// Link packages for local development
    #[command(visible_alias = "ln")]
    Link {
        /// Package name or directory to link
        #[arg(value_name = "PACKAGE|DIR")]
        package: Option<String>,

        /// Arguments to pass to package manager
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Unlink packages
    Unlink {
        /// Package name to unlink
        #[arg(value_name = "PACKAGE|DIR")]
        package: Option<String>,

        /// Unlink in every workspace package
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Arguments to pass to package manager
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Execute a package binary without installing it
    Dlx {
        /// Package(s) to install before running
        #[arg(long, short = 'p', value_name = "NAME")]
        package: Vec<String>,

        /// Execute within a shell environment
        #[arg(long = "shell-mode", short = 'c')]
        shell_mode: bool,

        /// Suppress all output except the executed command's output
        #[arg(long, short = 's')]
        silent: bool,

        /// Package to execute and arguments
        #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Forward a command to the package manager
    #[command(subcommand)]
    Pm(PmCommands),
}

impl PackageManagerCommand {
    /// Whether the command was invoked with flags that request quiet or
    /// machine-readable output.
    pub fn is_quiet_or_machine_readable(&self) -> bool {
        match self {
            Self::Install { silent, .. } | Self::Dlx { silent, .. } => *silent,
            Self::Outdated { format, .. } => matches!(format, Some(Format::Json | Format::List)),
            Self::Why { json, parseable, .. } => *json || *parseable,
            Self::Info { json, .. } => *json,
            Self::Pm(sub) => sub.is_quiet_or_machine_readable(),
            _ => false,
        }
    }

    /// Whether this invocation hits the vite-plus-managed-global flow on the
    /// global CLI. The local CLI binding refuses these cases (it has no
    /// managed package store of its own); pass-through `-g` cases like
    /// `outdated -g`, `why -g`, and `pm config get -g` return `false` and
    /// keep working on both CLIs.
    pub fn is_managed_global(&self) -> bool {
        match self {
            Self::Install { global, .. }
            | Self::Add { global, .. }
            | Self::Remove { global, .. }
            | Self::Update { global, .. } => *global,
            Self::Pm(PmCommands::List { global, .. }) => *global,
            _ => false,
        }
    }

    /// Determine the save dependency type from CLI flags shared by `Install` and `Add`.
    pub fn determine_save_dependency_type(
        save_dev: bool,
        save_peer: bool,
        save_optional: bool,
        save_prod: bool,
    ) -> Option<SaveDependencyType> {
        if save_dev {
            Some(SaveDependencyType::Dev)
        } else if save_peer {
            Some(SaveDependencyType::Peer)
        } else if save_optional {
            Some(SaveDependencyType::Optional)
        } else if save_prod {
            Some(SaveDependencyType::Production)
        } else {
            None
        }
    }
}

/// Package manager subcommands (`vp pm <subcommand>`).
#[derive(Subcommand, Debug, Clone)]
pub enum PmCommands {
    /// Remove unnecessary packages
    Prune {
        /// Remove devDependencies
        #[arg(long)]
        prod: bool,

        /// Remove optional dependencies
        #[arg(long)]
        no_optional: bool,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// Create a tarball of the package
    Pack {
        /// Pack all workspace packages
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Filter packages to pack
        #[arg(long, value_name = "PATTERN")]
        filter: Option<Vec<String>>,

        /// Output path for the tarball
        #[arg(long)]
        out: Option<String>,

        /// Directory where the tarball will be saved
        #[arg(long)]
        pack_destination: Option<String>,

        /// Gzip compression level (0-9)
        #[arg(long)]
        pack_gzip_level: Option<u8>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// List installed packages
    #[command(visible_alias = "ls")]
    List {
        /// Package pattern to filter
        pattern: Option<String>,

        /// Maximum depth of dependency tree
        #[arg(long)]
        depth: Option<u32>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Show extended information
        #[arg(long)]
        long: bool,

        /// Parseable output format
        #[arg(long)]
        parseable: bool,

        /// Only production dependencies
        #[arg(short = 'P', long)]
        prod: bool,

        /// Only dev dependencies
        #[arg(short = 'D', long)]
        dev: bool,

        /// Exclude optional dependencies
        #[arg(long)]
        no_optional: bool,

        /// Exclude peer dependencies
        #[arg(long)]
        exclude_peers: bool,

        /// Show only project packages
        #[arg(long)]
        only_projects: bool,

        /// Use a finder function
        #[arg(long, value_name = "FINDER_NAME")]
        find_by: Option<String>,

        /// List across all workspaces
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Filter packages in monorepo
        #[arg(long, value_name = "PATTERN")]
        filter: Vec<String>,

        /// List global packages
        #[arg(short = 'g', long)]
        global: bool,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// View package information from the registry
    #[command(visible_alias = "info", visible_alias = "show")]
    View {
        /// Package name with optional version
        #[arg(required = true)]
        package: String,

        /// Specific field to view
        field: Option<String>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// Publish package to registry
    Publish {
        /// Tarball or folder to publish
        #[arg(value_name = "TARBALL|FOLDER")]
        target: Option<String>,

        /// Preview without publishing
        #[arg(long)]
        dry_run: bool,

        /// Publish tag
        #[arg(long)]
        tag: Option<String>,

        /// Access level (public/restricted)
        #[arg(long)]
        access: Option<String>,

        /// One-time password for authentication
        #[arg(long, value_name = "OTP")]
        otp: Option<String>,

        /// Skip git checks
        #[arg(long)]
        no_git_checks: bool,

        /// Set the branch name to publish from
        #[arg(long, value_name = "BRANCH")]
        publish_branch: Option<String>,

        /// Save publish summary
        #[arg(long)]
        report_summary: bool,

        /// Force publish
        #[arg(long)]
        force: bool,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Publish all workspace packages
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Filter packages in monorepo
        #[arg(long, value_name = "PATTERN")]
        filter: Option<Vec<String>>,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// Manage package owners
    #[command(subcommand, visible_alias = "author")]
    Owner(OwnerCommands),

    /// Manage package cache
    Cache {
        /// Subcommand: dir, path, clean
        #[arg(required = true)]
        subcommand: String,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// Manage package manager configuration
    #[command(subcommand, visible_alias = "c")]
    Config(ConfigCommands),

    /// Log in to a registry
    #[command(visible_alias = "adduser")]
    Login {
        /// Registry URL
        #[arg(long, value_name = "URL")]
        registry: Option<String>,

        /// Scope for the login
        #[arg(long, value_name = "SCOPE")]
        scope: Option<String>,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// Log out from a registry
    Logout {
        /// Registry URL
        #[arg(long, value_name = "URL")]
        registry: Option<String>,

        /// Scope for the logout
        #[arg(long, value_name = "SCOPE")]
        scope: Option<String>,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// Show the current logged-in user
    Whoami {
        /// Registry URL
        #[arg(long, value_name = "URL")]
        registry: Option<String>,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// Manage authentication tokens
    #[command(subcommand)]
    Token(TokenCommands),

    /// Run a security audit
    Audit {
        /// Automatically fix vulnerabilities
        #[arg(long)]
        fix: bool,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Minimum vulnerability level to report
        #[arg(long, value_name = "LEVEL")]
        level: Option<String>,

        /// Only audit production dependencies
        #[arg(long)]
        production: bool,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// Manage distribution tags
    #[command(name = "dist-tag", subcommand)]
    DistTag(DistTagCommands),

    /// Deprecate a package version
    Deprecate {
        /// Package name with version (e.g., "my-pkg@1.0.0")
        package: String,

        /// Deprecation message
        message: String,

        /// One-time password for authentication
        #[arg(long, value_name = "OTP")]
        otp: Option<String>,

        /// Registry URL
        #[arg(long, value_name = "URL")]
        registry: Option<String>,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// Search for packages in the registry
    Search {
        /// Search terms
        #[arg(required = true, num_args = 1..)]
        terms: Vec<String>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Show extended information
        #[arg(long)]
        long: bool,

        /// Registry URL
        #[arg(long, value_name = "URL")]
        registry: Option<String>,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// Rebuild native modules
    #[command(visible_alias = "rb")]
    Rebuild {
        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// Show funding information for installed packages
    Fund {
        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// Ping the registry
    Ping {
        /// Registry URL
        #[arg(long, value_name = "URL")]
        registry: Option<String>,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },
}

impl PmCommands {
    pub fn is_quiet_or_machine_readable(&self) -> bool {
        match self {
            Self::List { json, parseable, .. } => *json || *parseable,
            Self::Pack { json, .. }
            | Self::View { json, .. }
            | Self::Publish { json, .. }
            | Self::Audit { json, .. }
            | Self::Search { json, .. }
            | Self::Fund { json, .. } => *json,
            Self::Config(sub) => sub.is_quiet_or_machine_readable(),
            Self::Token(sub) => sub.is_quiet_or_machine_readable(),
            _ => false,
        }
    }
}

/// Configuration subcommands.
#[derive(Subcommand, Debug, Clone)]
pub enum ConfigCommands {
    /// List all configuration
    List {
        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Use global config
        #[arg(short = 'g', long)]
        global: bool,

        /// Config location: project (default) or global
        #[arg(long, value_name = "LOCATION")]
        location: Option<String>,
    },

    /// Get configuration value
    Get {
        /// Config key
        key: String,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Use global config
        #[arg(short = 'g', long)]
        global: bool,

        /// Config location
        #[arg(long, value_name = "LOCATION")]
        location: Option<String>,
    },

    /// Set configuration value
    Set {
        /// Config key
        key: String,

        /// Config value
        value: String,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Use global config
        #[arg(short = 'g', long)]
        global: bool,

        /// Config location
        #[arg(long, value_name = "LOCATION")]
        location: Option<String>,
    },

    /// Delete configuration key
    Delete {
        /// Config key
        key: String,

        /// Use global config
        #[arg(short = 'g', long)]
        global: bool,

        /// Config location
        #[arg(long, value_name = "LOCATION")]
        location: Option<String>,
    },
}

impl ConfigCommands {
    pub fn is_quiet_or_machine_readable(&self) -> bool {
        match self {
            Self::List { json, .. } | Self::Get { json, .. } | Self::Set { json, .. } => *json,
            _ => false,
        }
    }
}

/// Owner subcommands.
#[derive(Subcommand, Debug, Clone)]
pub enum OwnerCommands {
    /// List package owners
    #[command(visible_alias = "ls")]
    List {
        /// Package name
        package: String,

        /// One-time password for authentication
        #[arg(long, value_name = "OTP")]
        otp: Option<String>,
    },

    /// Add package owner
    Add {
        /// Username
        user: String,
        /// Package name
        package: String,

        /// One-time password for authentication
        #[arg(long, value_name = "OTP")]
        otp: Option<String>,
    },

    /// Remove package owner
    Rm {
        /// Username
        user: String,
        /// Package name
        package: String,

        /// One-time password for authentication
        #[arg(long, value_name = "OTP")]
        otp: Option<String>,
    },
}

/// Token subcommands.
#[derive(Subcommand, Debug, Clone)]
pub enum TokenCommands {
    /// List all known tokens
    #[command(visible_alias = "ls")]
    List {
        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Registry URL
        #[arg(long, value_name = "URL")]
        registry: Option<String>,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// Create a new authentication token
    Create {
        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Registry URL
        #[arg(long, value_name = "URL")]
        registry: Option<String>,

        /// CIDR ranges to restrict the token to
        #[arg(long, value_name = "CIDR")]
        cidr: Option<Vec<String>>,

        /// Create a read-only token
        #[arg(long)]
        readonly: bool,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// Revoke an authentication token
    Revoke {
        /// Token or token ID to revoke
        token: String,

        /// Registry URL
        #[arg(long, value_name = "URL")]
        registry: Option<String>,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },
}

impl TokenCommands {
    pub fn is_quiet_or_machine_readable(&self) -> bool {
        match self {
            Self::List { json, .. } | Self::Create { json, .. } => *json,
            _ => false,
        }
    }
}

/// Distribution tag subcommands.
#[derive(Subcommand, Debug, Clone)]
pub enum DistTagCommands {
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
