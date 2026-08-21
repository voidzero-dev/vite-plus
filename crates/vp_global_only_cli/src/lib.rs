//! Commands that exist only in the global `vp` binary.
//!
//! The global CLI flattens [`GlobalOnlyCommand`] into its top-level command
//! enum. [`is_global_only`] derives the command names from the same enum so
//! other crates can recognise them without keeping a separate list.

#![allow(clippy::allow_attributes, clippy::disallowed_types)]

use clap::{Subcommand, builder::Command};

/// Commands that only the global `vp` binary can run.
#[derive(Subcommand, Debug)]
pub enum GlobalOnlyCommand {
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

        /// npm dist-tag to install (default: "latest", also: "alpha")
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

        /// Refresh the cached update status without producing output
        #[arg(long, hide = true)]
        background_check: bool,
    },

    /// Remove vp and all related data
    Implode {
        /// Skip confirmation prompt
        #[arg(long, short = 'y')]
        yes: bool,
    },
}

impl GlobalOnlyCommand {
    /// Whether the command was invoked with flags that request quiet or
    /// machine-readable output.
    pub fn is_quiet_or_machine_readable(&self) -> bool {
        match self {
            Self::Upgrade { silent, .. } => *silent,
            Self::Env(args) => {
                args.command.as_ref().is_some_and(|sub| sub.is_quiet_or_machine_readable())
            }
            Self::Implode { .. } => false,
        }
    }
}

/// Whether `name` (a subcommand name or alias) is only available in the
/// global `vp` binary. Derived from [`GlobalOnlyCommand`] so there is no
/// separate list to keep in sync.
pub fn is_global_only(name: &str) -> bool {
    GlobalOnlyCommand::augment_subcommands(Command::new("vp"))
        .get_subcommands()
        .any(|cmd| cmd.get_name() == name || cmd.get_all_aliases().any(|alias| alias == name))
}

/// Arguments for the `env` command
#[derive(clap::Args, Debug)]
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
    #[command(after_long_help = "\
Examples:
  vp env default          # Show the current default
  vp env default lts      # Set the default")]
    Default {
        /// Version to set as default (e.g., "20.18.0", "lts", "latest").
        /// If omitted, prints the current default.
        version: Option<String>,
    },

    /// Enable managed mode - shims always use vite-plus managed Node.js
    On,

    /// Enable system-first mode - shims prefer system Node.js, fallback to managed
    Off,

    /// Create or update shims in VP_HOME/bin
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

    /// Pin a Node.js version in the current directory
    /// (updates .node-version or package.json#devEngines.runtime)
    #[command(after_long_help = "\
Examples:
  vp env pin lts                  # Pin to latest LTS
  vp env pin --unpin              # Remove the pin
  vp env pin \"^20.0.0\" --force    # Overwrite existing pin
  vp env pin 24 --target node-version   # Force the .node-version file

The write target follows the compatibility-first rule: an existing .node-version
keeps being updated; otherwise the pin is written to package.json#devEngines.runtime;
.node-version is only created when the directory has no package.json.")]
    Pin {
        /// Version to pin (e.g., "20.18.0", "lts", "latest", "^20.0.0").
        /// If omitted, prints the currently pinned version.
        version: Option<String>,

        /// Remove the pin from the current directory
        #[arg(long)]
        unpin: bool,

        /// Skip pre-downloading the pinned version
        #[arg(long)]
        no_install: bool,

        /// Overwrite an existing pin without confirmation
        #[arg(long)]
        force: bool,

        /// Explicitly choose the write target (overrides the default selection)
        #[arg(long, value_enum)]
        target: Option<PinTarget>,
    },

    /// Remove the Node.js pin from current directory (alias for `pin --unpin`)
    Unpin {
        /// Explicitly choose which pin source to remove
        #[arg(long, value_enum)]
        target: Option<PinTarget>,
    },

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
    #[command(
        visible_alias = "run",
        after_long_help = "\
Examples:
  vp env exec --node lts npm install  # Pin version for this invocation
  vp env exec node -v                 # Shim mode: version auto-resolved"
    )]
    Exec {
        /// Node.js version to use (e.g., "20.18.0", "lts", "^20.0.0").
        /// If omitted and command is node/npm/npx or a global package binary,
        /// version is resolved automatically (same as shim behavior).
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

    /// Remove unused managed runtimes and package manager caches
    Clean,

    /// Install a Node.js version
    #[command(visible_alias = "i")]
    Install {
        /// Version to install (e.g., "20", "20.18.0", "lts", "latest")
        /// If not provided, installs the version from .node-version, package.json, or .nvmrc
        version: Option<String>,
    },

    /// Use a specific Node.js version for this shell session
    #[command(after_long_help = "\
Examples:
  vp env use lts        # Override session with latest LTS
  vp env use --unset    # Clear the session override")]
    Use {
        /// Version to use (e.g., "20", "20.18.0", "lts", "latest").
        /// If omitted, reads from .node-version, package.json, or .nvmrc.
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

impl EnvSubcommands {
    pub fn is_quiet_or_machine_readable(&self) -> bool {
        match self {
            Self::Current { json } | Self::List { json } | Self::ListRemote { json, .. } => *json,
            _ => false,
        }
    }
}

/// Write target for `vp env pin` / `vp env unpin` (see rfcs/dev-engines.md)
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PinTarget {
    /// Pin via the .node-version file
    NodeVersion,
    /// Pin via package.json#devEngines.runtime
    DevEngines,
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

#[cfg(test)]
mod tests {
    use super::is_global_only;

    #[test]
    fn recognises_global_only_command_names() {
        for name in ["env", "upgrade", "implode"] {
            assert!(is_global_only(name), "{name} should be global-only");
        }
    }

    #[test]
    fn rejects_other_command_names() {
        for name in ["create", "dev", "install", "run", ""] {
            assert!(!is_global_only(name), "{name} should not be global-only");
        }
    }
}
