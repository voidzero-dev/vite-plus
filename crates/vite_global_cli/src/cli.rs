//! CLI argument parsing and command routing.
//!
//! This module defines the CLI structure using clap and routes commands
//! to their appropriate handlers.

use std::process::ExitStatus;

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use vite_install::commands::{
    add::SaveDependencyType, install::InstallCommandOptions, outdated::Format,
};
use vite_path::AbsolutePathBuf;

use crate::{
    commands::{
        self, AddCommand, DedupeCommand, DlxCommand, InstallCommand, LinkCommand, OutdatedCommand,
        RemoveCommand, UnlinkCommand, UpdateCommand, WhyCommand,
    },
    error::Error,
    help,
};

#[derive(Clone, Copy, Debug)]
pub struct RenderOptions {
    pub show_header: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self { show_header: true }
    }
}

/// Vite+ Global CLI
#[derive(Parser, Debug)]
#[clap(
    name = "vp",
    bin_name = "vp",
    author,
    about = "Vite+ - A next-generation build tool",
    long_about = None
)]
#[command(disable_help_subcommand = true, disable_version_flag = true)]
pub struct Args {
    /// Print version
    #[arg(short = 'V', long = "version")]
    pub version: bool,

    #[clap(subcommand)]
    pub command: Option<Commands>,
}

/// Available commands
#[derive(Subcommand, Debug)]
pub enum Commands {
    // =========================================================================
    // Category A: Package Manager Commands
    // =========================================================================
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

        /// Install globally (only when adding packages)
        #[arg(short = 'g', long)]
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

    // =========================================================================
    // Category B: JS Script Commands
    // These commands are implemented in JavaScript and executed via managed Node.js
    // =========================================================================
    /// Create a new project from a template (delegates to JS)
    #[command(disable_help_flag = true)]
    Create {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Migrate an existing project to Vite+ (delegates to JS)
    #[command(disable_help_flag = true)]
    Migrate {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// In-repo configuration (hooks, agent integration)
    #[command(disable_help_flag = true)]
    Config {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Run vite-staged on Git staged files
    #[command(disable_help_flag = true, name = "staged")]
    Staged {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    // =========================================================================
    // Category C: Local CLI Delegation (stubs for now)
    // =========================================================================
    /// Run the development server
    #[command(disable_help_flag = true)]
    Dev {
        /// Additional arguments
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Build application
    #[command(disable_help_flag = true)]
    Build {
        /// Additional arguments
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Run tests
    #[command(disable_help_flag = true)]
    Test {
        /// Additional arguments
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Lint code
    #[command(disable_help_flag = true)]
    Lint {
        /// Additional arguments
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Format code
    #[command(disable_help_flag = true)]
    Fmt {
        /// Additional arguments
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Run format, lint, and type checks
    #[command(disable_help_flag = true)]
    Check {
        /// Additional arguments
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Build library
    #[command(disable_help_flag = true)]
    Pack {
        /// Additional arguments
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Run tasks
    #[command(disable_help_flag = true)]
    Run {
        /// Additional arguments
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Execute a command from local node_modules/.bin
    #[command(disable_help_flag = true)]
    Exec {
        /// Additional arguments
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Preview production build
    #[command(disable_help_flag = true)]
    Preview {
        /// Additional arguments
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Manage the task cache
    #[command(disable_help_flag = true)]
    Cache {
        /// Additional arguments
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Manage Node.js versions
    Env(EnvArgs),

    // =========================================================================
    // Self-Management
    // =========================================================================
    /// Update vp itself to the latest version
    #[command(name = "upgrade")]
    Upgrade {
        /// Target version (e.g., "0.2.0"). Defaults to latest.
        version: Option<String>,

        /// npm dist-tag to install (default: "latest", also: "test")
        #[arg(long, default_value = "latest")]
        tag: String,

        /// Check for updates without installing
        #[arg(long)]
        check: bool,

        /// Revert to the previously active version
        #[arg(long)]
        rollback: bool,

        /// Force reinstall even if already on the target version
        #[arg(long)]
        force: bool,

        /// Suppress output
        #[arg(long)]
        silent: bool,

        /// Custom npm registry URL
        #[arg(long)]
        registry: Option<String>,
    },
}

/// Arguments for the `env` command
#[derive(clap::Args, Debug)]
#[command(after_help = "\
Examples:
  Setup:
    vp env setup                  # Create shims for node, npm, npx
    vp env on                     # Use vite-plus managed Node.js
    vp env print                  # Print shell snippet for this session

  Manage:
    vp env pin lts                # Pin to latest LTS version
    vp env install                # Install version from .node-version / package.json
    vp env use 20                 # Use Node.js 20 for this shell session
    vp env use --unset            # Remove session override

  Inspect:
    vp env current                # Show current resolved environment
    vp env current --json         # JSON output for automation
    vp env doctor                 # Check environment configuration
    vp env which node             # Show which node binary will be used
    vp env list-remote --lts      # List only LTS versions

  Execute:
    vp env exec --node lts npm i  # Execute 'npm i' with latest LTS
    vp env exec node -v           # Shim mode (version auto-resolved)

Related Commands:
  vp install -g <package>       # Install a package globally
  vp uninstall -g <package>     # Uninstall a package globally
  vp update -g [package]        # Update global packages
  vp list -g [package]          # List global packages")]
pub struct EnvArgs {
    /// Subcommand (e.g., 'default', 'setup', 'doctor', 'which')
    #[command(subcommand)]
    pub command: Option<EnvSubcommands>,
}

/// Subcommands for the `env` command
#[derive(clap::Subcommand, Debug)]
pub enum EnvSubcommands {
    /// Show current environment information
    Current {
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Print shell snippet to set environment for current session
    Print,

    /// Set or show the global default Node.js version
    Default {
        /// Version to set as default (e.g., "20.18.0", "lts", "latest")
        /// If not provided, shows the current default
        version: Option<String>,
    },

    /// Enable managed mode - shims always use vite-plus managed Node.js
    On,

    /// Enable system-first mode - shims prefer system Node.js, fallback to managed
    Off,

    /// Create or update shims in VITE_PLUS_HOME/bin
    Setup {
        /// Force refresh shims even if they exist
        #[arg(long)]
        refresh: bool,
        /// Only create env files (skip shims and instructions)
        #[arg(long)]
        env_only: bool,
    },

    /// Run diagnostics and show environment status
    Doctor,

    /// Show path to the tool that would be executed
    Which {
        /// Tool name (node, npm, or npx)
        tool: String,
    },

    /// Pin a Node.js version in the current directory (creates .node-version)
    Pin {
        /// Version to pin (e.g., "20.18.0", "lts", "latest", "^20.0.0")
        /// If not provided, shows the current pinned version
        version: Option<String>,

        /// Remove the .node-version file from current directory
        #[arg(long)]
        unpin: bool,

        /// Skip pre-downloading the pinned version
        #[arg(long)]
        no_install: bool,

        /// Overwrite existing .node-version without confirmation
        #[arg(long)]
        force: bool,
    },

    /// Remove the .node-version file from current directory (alias for `pin --unpin`)
    Unpin,

    /// List locally installed Node.js versions
    #[command(visible_alias = "ls")]
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// List available Node.js versions from the registry
    #[command(name = "list-remote", visible_alias = "ls-remote")]
    ListRemote {
        /// Filter versions by pattern (e.g., "20" for 20.x versions)
        pattern: Option<String>,

        /// Show only LTS versions
        #[arg(long)]
        lts: bool,

        /// Show all versions (not just recent)
        #[arg(long)]
        all: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Version sorting order
        #[arg(long, value_enum, default_value_t = SortingMethod::Asc)]
        sort: SortingMethod,
    },

    /// Execute a command with a specific Node.js version
    #[command(visible_alias = "run")]
    Exec {
        /// Node.js version to use (e.g., "20.18.0", "lts", "^20.0.0")
        /// If not provided and command is node/npm/npx or a global package binary,
        /// version is resolved automatically (same as shim behavior)
        #[arg(long)]
        node: Option<String>,

        /// npm version to use (optional, defaults to bundled)
        #[arg(long)]
        npm: Option<String>,

        /// Command and arguments to run
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },

    /// Uninstall a Node.js version
    #[command(visible_alias = "uni")]
    Uninstall {
        /// Version to uninstall (e.g., "20.18.0")
        #[arg(required = true)]
        version: String,
    },

    /// Install a Node.js version
    #[command(visible_alias = "i")]
    Install {
        /// Version to install (e.g., "20", "20.18.0", "lts", "latest")
        /// If not provided, installs the version from .node-version or package.json
        version: Option<String>,
    },

    /// Use a specific Node.js version for this shell session
    Use {
        /// Version to use (e.g., "20", "20.18.0", "lts", "latest")
        /// If not provided, reads from .node-version or package.json
        version: Option<String>,

        /// Remove session override (revert to file-based resolution)
        #[arg(long)]
        unset: bool,

        /// Skip auto-installation if version not present
        #[arg(long)]
        no_install: bool,

        /// Suppress output if version is already active
        #[arg(long)]
        silent_if_unchanged: bool,
    },
}

/// Version sorting order for list-remote command
#[derive(clap::ValueEnum, Clone, Debug, Default)]
pub enum SortingMethod {
    /// Sort versions in ascending order (earliest to latest)
    #[default]
    Asc,
    /// Sort versions in descending order (latest to earliest)
    Desc,
}

/// Package manager subcommands
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

/// Configuration subcommands
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

/// Owner subcommands
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

/// Token subcommands
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

/// Distribution tag subcommands
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

/// Determine the save dependency type from CLI flags.
fn determine_save_dependency_type(
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

/// Run the CLI command.
pub async fn run_command(cwd: AbsolutePathBuf, args: Args) -> Result<ExitStatus, Error> {
    run_command_with_options(cwd, args, RenderOptions::default()).await
}

/// Run the CLI command with rendering options.
pub async fn run_command_with_options(
    cwd: AbsolutePathBuf,
    args: Args,
    render_options: RenderOptions,
) -> Result<ExitStatus, Error> {
    // Handle --version flag (Category B: delegates to JS)
    if args.version {
        return commands::version::execute(cwd).await;
    }

    // If no command provided, show help and exit
    let Some(command) = args.command else {
        // Use custom help formatting to match the JS CLI output
        if render_options.show_header {
            command_with_help().print_help().ok();
        } else {
            command_with_help_with_options(render_options).print_help().ok();
        }
        println!();
        // Return a successful exit status since help was requested implicitly
        return Ok(std::process::ExitStatus::default());
    };

    match command {
        // Category A: Package Manager Commands
        Commands::Install {
            prod,
            dev,
            no_optional,
            frozen_lockfile,
            no_frozen_lockfile,
            lockfile_only,
            prefer_offline,
            offline,
            force,
            ignore_scripts,
            no_lockfile,
            fix_lockfile,
            shamefully_hoist,
            resolution_only,
            silent,
            filter,
            workspace_root,
            save_exact,
            save_peer,
            save_optional,
            save_catalog,
            global,
            node,
            packages,
            pass_through_args,
        } => {
            print_runtime_header(render_options.show_header);
            // If packages are provided, redirect to Add command
            if let Some(pkgs) = packages
                && !pkgs.is_empty()
            {
                // Handle global install via vite-plus managed global install
                if global {
                    use crate::commands::env::global_install;
                    for package in &pkgs {
                        if let Err(e) =
                            global_install::install(package, node.as_deref(), force).await
                        {
                            eprintln!("Failed to install {}: {}", package, e);
                            return Ok(exit_status(1));
                        }
                    }
                    return Ok(ExitStatus::default());
                }

                let save_dependency_type =
                    determine_save_dependency_type(dev, save_peer, save_optional, prod);

                return AddCommand::new(cwd)
                    .execute(
                        &pkgs,
                        save_dependency_type,
                        save_exact,
                        if save_catalog { Some("default") } else { None },
                        filter.as_deref(),
                        workspace_root,
                        false, // workspace_only
                        global,
                        None, // allow_build
                        pass_through_args.as_deref(),
                    )
                    .await;
            }

            // No packages provided, run regular install
            let options = InstallCommandOptions {
                prod,
                dev,
                no_optional,
                frozen_lockfile,
                no_frozen_lockfile,
                lockfile_only,
                prefer_offline,
                offline,
                force,
                ignore_scripts,
                no_lockfile,
                fix_lockfile,
                shamefully_hoist,
                resolution_only,
                silent,
                filters: filter.as_deref(),
                workspace_root,
                pass_through_args: pass_through_args.as_deref(),
            };
            InstallCommand::new(cwd).execute(&options).await
        }

        Commands::Add {
            save_prod,
            save_dev,
            save_peer,
            save_optional,
            save_exact,
            save_catalog_name,
            save_catalog,
            allow_build,
            filter,
            workspace_root,
            workspace,
            global,
            node,
            packages,
            pass_through_args,
        } => {
            // Handle global install via vite-plus managed global install
            if global {
                use crate::commands::env::global_install;
                for package in &packages {
                    if let Err(e) = global_install::install(package, node.as_deref(), false).await {
                        eprintln!("Failed to install {}: {}", package, e);
                        return Ok(exit_status(1));
                    }
                }
                return Ok(ExitStatus::default());
            }

            let save_dependency_type =
                determine_save_dependency_type(save_dev, save_peer, save_optional, save_prod);

            let catalog_name =
                if save_catalog { Some("default") } else { save_catalog_name.as_deref() };

            AddCommand::new(cwd)
                .execute(
                    &packages,
                    save_dependency_type,
                    save_exact,
                    catalog_name,
                    filter.as_deref(),
                    workspace_root,
                    workspace,
                    global,
                    allow_build.as_deref(),
                    pass_through_args.as_deref(),
                )
                .await
        }

        Commands::Remove {
            save_dev,
            save_optional,
            save_prod,
            filter,
            workspace_root,
            recursive,
            global,
            dry_run,
            packages,
            pass_through_args,
        } => {
            // Handle global uninstall via vite-plus managed global install
            if global {
                use crate::commands::env::global_install;
                for package in &packages {
                    if let Err(e) = global_install::uninstall(package, dry_run).await {
                        eprintln!("Failed to uninstall {}: {}", package, e);
                        return Ok(exit_status(1));
                    }
                }
                return Ok(ExitStatus::default());
            }

            RemoveCommand::new(cwd)
                .execute(
                    &packages,
                    save_dev,
                    save_optional,
                    save_prod,
                    filter.as_deref(),
                    workspace_root,
                    recursive,
                    global,
                    pass_through_args.as_deref(),
                )
                .await
        }

        Commands::Update {
            latest,
            global,
            recursive,
            filter,
            workspace_root,
            dev,
            prod,
            interactive,
            no_optional,
            no_save,
            workspace,
            packages,
            pass_through_args,
        } => {
            // Handle global update via vite-plus managed global install
            if global {
                use crate::commands::env::{global_install, package_metadata::PackageMetadata};

                let packages_to_update = if packages.is_empty() {
                    let all = PackageMetadata::list_all().await?;
                    if all.is_empty() {
                        println!("No global packages installed.");
                        return Ok(ExitStatus::default());
                    }
                    all.iter().map(|p| p.name.clone()).collect::<Vec<_>>()
                } else {
                    packages.clone()
                };
                for package in &packages_to_update {
                    if let Err(e) = global_install::install(package, None, false).await {
                        eprintln!("Failed to update {}: {}", package, e);
                        return Ok(exit_status(1));
                    }
                }
                return Ok(ExitStatus::default());
            }

            UpdateCommand::new(cwd)
                .execute(
                    &packages,
                    latest,
                    global,
                    recursive,
                    filter.as_deref(),
                    workspace_root,
                    dev,
                    prod,
                    interactive,
                    no_optional,
                    no_save,
                    workspace,
                    pass_through_args.as_deref(),
                )
                .await
        }

        Commands::Dedupe { check, pass_through_args } => {
            DedupeCommand::new(cwd).execute(check, pass_through_args.as_deref()).await
        }

        Commands::Outdated {
            packages,
            long,
            format,
            recursive,
            filter,
            workspace_root,
            prod,
            dev,
            no_optional,
            compatible,
            sort_by,
            global,
            pass_through_args,
        } => {
            OutdatedCommand::new(cwd)
                .execute(
                    &packages,
                    long,
                    format,
                    recursive,
                    filter.as_deref(),
                    workspace_root,
                    prod,
                    dev,
                    no_optional,
                    compatible,
                    sort_by.as_deref(),
                    global,
                    pass_through_args.as_deref(),
                )
                .await
        }

        Commands::Why {
            packages,
            json,
            long,
            parseable,
            recursive,
            filter,
            workspace_root,
            prod,
            dev,
            depth,
            no_optional,
            global,
            exclude_peers,
            find_by,
            pass_through_args,
        } => {
            WhyCommand::new(cwd)
                .execute(
                    &packages,
                    json,
                    long,
                    parseable,
                    recursive,
                    filter.as_deref(),
                    workspace_root,
                    prod,
                    dev,
                    depth,
                    no_optional,
                    global,
                    exclude_peers,
                    find_by.as_deref(),
                    pass_through_args.as_deref(),
                )
                .await
        }

        Commands::Info { package, field, json, pass_through_args } => {
            commands::pm::execute_info(
                cwd,
                &package,
                field.as_deref(),
                json,
                pass_through_args.as_deref(),
            )
            .await
        }

        Commands::Link { package, args } => {
            let pass_through = if args.is_empty() { None } else { Some(args.as_slice()) };
            LinkCommand::new(cwd).execute(package.as_deref(), pass_through).await
        }

        Commands::Unlink { package, recursive, args } => {
            let pass_through = if args.is_empty() { None } else { Some(args.as_slice()) };
            UnlinkCommand::new(cwd).execute(package.as_deref(), recursive, pass_through).await
        }

        Commands::Dlx { package, shell_mode, silent, args } => {
            DlxCommand::new(cwd).execute(package, shell_mode, silent, args).await
        }

        Commands::Pm(pm_command) => commands::pm::execute_pm_subcommand(cwd, pm_command).await,

        // Category B: JS Script Commands
        Commands::Create { args } => commands::create::execute(cwd, &args).await,

        Commands::Migrate { args } => commands::migrate::execute(cwd, &args).await,

        Commands::Config { args } => commands::config::execute(cwd, &args).await,

        Commands::Staged { args } => commands::staged::execute(cwd, &args).await,

        // Category C: Local CLI Delegation (stubs)
        Commands::Dev { args } => {
            if help::maybe_print_unified_delegate_help("dev", &args, render_options.show_header) {
                return Ok(ExitStatus::default());
            }
            print_runtime_header(render_options.show_header);
            commands::delegate::execute(cwd, "dev", &args).await
        }

        Commands::Build { args } => {
            if help::maybe_print_unified_delegate_help("build", &args, render_options.show_header) {
                return Ok(ExitStatus::default());
            }
            print_runtime_header(render_options.show_header);
            commands::delegate::execute(cwd, "build", &args).await
        }

        Commands::Test { args } => {
            if help::maybe_print_unified_delegate_help("test", &args, render_options.show_header) {
                return Ok(ExitStatus::default());
            }
            print_runtime_header(render_options.show_header);
            commands::delegate::execute(cwd, "test", &args).await
        }

        Commands::Lint { args } => {
            if help::maybe_print_unified_delegate_help("lint", &args, render_options.show_header) {
                return Ok(ExitStatus::default());
            }
            print_runtime_header(render_options.show_header);
            commands::delegate::execute(cwd, "lint", &args).await
        }

        Commands::Fmt { args } => {
            if help::maybe_print_unified_delegate_help("fmt", &args, render_options.show_header) {
                return Ok(ExitStatus::default());
            }
            print_runtime_header(render_options.show_header);
            commands::delegate::execute(cwd, "fmt", &args).await
        }

        Commands::Check { args } => {
            if help::maybe_print_unified_delegate_help("check", &args, render_options.show_header) {
                return Ok(ExitStatus::default());
            }
            print_runtime_header(render_options.show_header);
            commands::delegate::execute(cwd, "check", &args).await
        }

        Commands::Pack { args } => {
            if help::maybe_print_unified_delegate_help("pack", &args, render_options.show_header) {
                return Ok(ExitStatus::default());
            }
            print_runtime_header(render_options.show_header);
            commands::delegate::execute(cwd, "pack", &args).await
        }

        Commands::Run { args } => {
            if help::maybe_print_unified_delegate_help("run", &args, render_options.show_header) {
                return Ok(ExitStatus::default());
            }
            print_runtime_header(render_options.show_header);
            commands::run_or_delegate::execute(cwd, &args).await
        }

        Commands::Exec { args } => {
            if help::maybe_print_unified_delegate_help("exec", &args, render_options.show_header) {
                return Ok(ExitStatus::default());
            }
            print_runtime_header(render_options.show_header);
            commands::delegate::execute(cwd, "exec", &args).await
        }

        Commands::Preview { args } => {
            if help::maybe_print_unified_delegate_help("preview", &args, render_options.show_header)
            {
                return Ok(ExitStatus::default());
            }
            print_runtime_header(render_options.show_header);
            commands::delegate::execute(cwd, "preview", &args).await
        }

        Commands::Cache { args } => {
            if help::maybe_print_unified_delegate_help("cache", &args, render_options.show_header) {
                return Ok(ExitStatus::default());
            }
            print_runtime_header(render_options.show_header);
            commands::delegate::execute(cwd, "cache", &args).await
        }

        Commands::Env(args) => commands::env::execute(cwd, args).await,

        // Self-Management
        Commands::Upgrade { version, tag, check, rollback, force, silent, registry } => {
            commands::upgrade::execute(commands::upgrade::UpgradeOptions {
                version,
                tag,
                check,
                rollback,
                force,
                silent,
                registry,
            })
            .await
        }
    }
}

/// Create an exit status with the given code.
pub(crate) fn exit_status(code: i32) -> ExitStatus {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(code << 8)
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw(code as u32)
    }
}

fn print_runtime_header(show_header: bool) {
    if !show_header {
        return;
    }
    println!("{}", vite_shared::header::vite_plus_header());
    println!();
}

/// Build a clap Command with custom help formatting matching the JS CLI output.
pub fn command_with_help() -> clap::Command {
    command_with_help_with_options(RenderOptions::default())
}

/// Build a clap Command with custom help formatting and rendering options.
pub fn command_with_help_with_options(render_options: RenderOptions) -> clap::Command {
    apply_custom_help(Args::command(), render_options)
}

/// Apply custom help formatting to a clap Command to match the JS CLI output.
fn apply_custom_help(cmd: clap::Command, render_options: RenderOptions) -> clap::Command {
    let after_help = help::render_help_doc(&help::top_level_help_doc());
    let options_heading = help::render_heading("Options");
    let header = if render_options.show_header {
        vite_shared::header::vite_plus_header()
    } else {
        String::new()
    };
    let help_template = format!("{header}{{after-help}}\n{options_heading}\n{{options}}\n");

    cmd.after_help(after_help).help_template(help_template)
}

/// Parse CLI arguments from a custom args iterator with custom help formatting.
/// Returns `Err` with the clap error if parsing fails (e.g., unknown command).
pub fn try_parse_args_from(
    args: impl IntoIterator<Item = String>,
) -> Result<Args, clap::error::Error> {
    try_parse_args_from_with_options(args, RenderOptions::default())
}

/// Parse CLI arguments from a custom args iterator with rendering options.
/// Returns `Err` with the clap error if parsing fails (e.g., unknown command).
pub fn try_parse_args_from_with_options(
    args: impl IntoIterator<Item = String>,
    render_options: RenderOptions,
) -> Result<Args, clap::error::Error> {
    let cmd = apply_custom_help(Args::command(), render_options);
    let matches = cmd.try_get_matches_from(args)?;
    Args::from_arg_matches(&matches).map_err(|e| e.into())
}
