//! CLI argument parsing and command routing.
//!
//! This module defines the CLI structure using clap and routes commands
//! to their appropriate handlers.

use std::{collections::HashSet, ffi::OsStr, process::ExitStatus};

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use clap_complete::ArgValueCompleter;
use dialoguer::{Confirm, theme::ColorfulTheme};
use owo_colors::OwoColorize;
use tokio::runtime::Runtime;
use vp_pm_cli::{ManagedGlobalCommand, PackageManagerCommand};
use vp_shared::output;
use vt_path::AbsolutePathBuf;

use crate::{
    commands::{
        self,
        env::{config::resolve_version, package_metadata::PackageMetadata},
        global,
    },
    error::Error,
    help,
};

const DEFAULT_GLOBAL_INSTALL_CONCURRENCY: usize = 5;
const DEFAULT_GLOBAL_VIEW_CONCURRENCY: usize = 3 * DEFAULT_GLOBAL_INSTALL_CONCURRENCY;

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

    /// Run as if vp was started in <DIR> instead of the current working directory
    #[arg(short = 'C', value_name = "DIR")]
    pub chdir: Option<String>,

    #[clap(subcommand)]
    pub command: Option<Commands>,
}

/// Available commands
#[derive(Subcommand, Debug)]
pub enum Commands {
    // =========================================================================
    // Category A: Package Manager Commands
    // (clap-flattened from `vp_pm_cli::PackageManagerCommand` so the
    // global CLI and the local CLI binding share an identical surface.)
    // =========================================================================
    #[command(flatten)]
    PackageManager(PackageManagerCommand),

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

    /// Manage the Vite+ Git hook dispatcher
    #[command(disable_help_flag = true)]
    Hooks {
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
    // Category C: Local CLI Delegation (forwarded to the local vite-plus CLI)
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
    #[command(disable_help_flag = true, visible_alias = "format")]
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
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, add = ArgValueCompleter::new(run_tasks_completions))]
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

    /// Show active Vite+ tools, versions, and relationships
    Toolchain {
        /// Tool or package names to show
        #[arg(value_name = "TOOLS")]
        tools: Vec<String>,

        /// Print the graph as JSON
        #[arg(long)]
        json: bool,

        /// Use the global Vite+ toolchain
        #[arg(long)]
        global: bool,
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

impl Commands {
    /// Whether the command was invoked with flags that request quiet or
    /// machine-readable output (--silent, -s, --json, --parseable, --format json/list).
    pub fn is_quiet_or_machine_readable(&self) -> bool {
        match self {
            Self::PackageManager(pm) => pm.is_quiet_or_machine_readable(),
            Self::Toolchain { json, .. } => *json,
            Self::Upgrade { silent, .. } => *silent,
            Self::Env(args) => {
                args.command.as_ref().is_some_and(|sub| sub.is_quiet_or_machine_readable())
            }
            _ => false,
        }
    }
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
    fn is_quiet_or_machine_readable(&self) -> bool {
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

fn has_flag_before_terminator(args: &[String], flag: &str) -> bool {
    for arg in args {
        if arg == "--" {
            break;
        }
        if arg == flag || arg.starts_with(&format!("{flag}=")) {
            return true;
        }
    }
    false
}

fn should_force_global_delegate(command: &str, args: &[String]) -> bool {
    match command {
        "lint" => has_flag_before_terminator(args, "--init"),
        "fmt" => {
            has_flag_before_terminator(args, "--init")
                || has_flag_before_terminator(args, "--migrate")
        }
        _ => false,
    }
}

/// Whether the Vite+ banner should be suppressed for a lint/fmt invocation.
///
/// IDE extensions invoke `vp lint --lsp`, `vp fmt --lsp`, and
/// `vp fmt --stdin-filepath` and parse the subprocess's stdout as the LSP
/// protocol / formatted source; the cosmetic banner would corrupt that stream.
fn should_suppress_header_for_subcommand(command: &str, args: &[String]) -> bool {
    match command {
        "lint" => has_flag_before_terminator(args, "--lsp"),
        "fmt" => {
            has_flag_before_terminator(args, "--lsp")
                || has_flag_before_terminator(args, "--stdin-filepath")
        }
        _ => false,
    }
}

/// Get available tasks for shell completion.
///
/// Delegates to the local vite-plus CLI to run `vp run` without arguments,
/// which returns a list of available tasks in the format "task_name: description".
fn run_tasks_completions(current: &OsStr) -> Vec<clap_complete::CompletionCandidate> {
    let Ok(cwd) = vt_path::current_dir() else {
        return vec![];
    };

    // Unescape hashtag and trim quotes for better matching
    let current = current
        .to_string_lossy()
        .replace("\\#", "#")
        .trim_matches(|c| c == '"' || c == '\'')
        .to_string();

    let output = tokio::task::block_in_place(|| {
        Runtime::new().ok().and_then(|rt| {
            rt.block_on(async { commands::delegate::execute_output(cwd, "run", &[]).await.ok() })
        })
    });

    output
        .filter(|o| o.status.success())
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|line| line.split_once(": ").map(|(name, _)| name.trim()))
                .filter(|name| !name.is_empty())
                .filter(|name| name.starts_with(&current) || current.is_empty())
                .map(|name| clap_complete::CompletionCandidate::new(name.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// Handle a parsed package-manager command.
///
/// Commands projected by [`PackageManagerCommand::managed_global_command`] are
/// routed through the vite-plus-managed Node.js install store
/// (`commands::global`). Everything else is forwarded to
/// `vp_pm_cli::dispatch`, which executes the underlying package manager
/// (pnpm/npm/yarn/bun).
async fn run_package_manager_command(
    cwd: AbsolutePathBuf,
    command: PackageManagerCommand,
) -> Result<ExitStatus, Error> {
    match command.managed_global_command() {
        Some(ManagedGlobalCommand::Install { packages, node, force, concurrency }) => {
            return managed_install(packages, node, force, concurrency).await;
        }
        Some(ManagedGlobalCommand::Remove { packages, dry_run }) => {
            return managed_uninstall(packages, dry_run).await;
        }
        Some(ManagedGlobalCommand::Update {
            packages,
            latest,
            concurrency,
            reinstall_node_mismatch,
            ignore_node_mismatch,
        }) => {
            if reinstall_node_mismatch && ignore_node_mismatch {
                output::error(
                    "--reinstall-node-mismatch and --ignore-node-mismatch cannot be used together",
                );
                return Ok(exit_status(1));
            }
            return managed_update(
                packages,
                latest,
                concurrency,
                reinstall_node_mismatch,
                ignore_node_mismatch,
            )
            .await;
        }
        Some(ManagedGlobalCommand::Outdated { packages, long, format, concurrency }) => {
            return global::outdated::execute(
                packages,
                long,
                format,
                concurrency.unwrap_or(DEFAULT_GLOBAL_VIEW_CONCURRENCY),
            )
            .await;
        }
        // `pm list -g` lists vite-plus-managed globals, not the underlying PM's.
        Some(ManagedGlobalCommand::List { json, pattern }) => {
            return global::packages::execute(json, pattern).await;
        }
        None => {}
    }

    commands::prepend_js_runtime_to_path_env(&cwd).await?;
    let result = vp_pm_cli::dispatch_with_metadata(&cwd, command).await?;
    if result.status.success()
        && let Some(packages) = result.why_hint_packages.as_deref()
    {
        print_toolchain_why_hint(&cwd, packages);
    }
    Ok(result.status)
}

fn print_toolchain_why_hint(cwd: &vt_path::AbsolutePath, packages: &[String]) {
    let Some(manifest) = active_toolchain_manifest(cwd) else {
        return;
    };
    let Some(hint) = vp_toolchain::why_hint(&manifest, packages) else {
        return;
    };
    output::raw_stderr("");
    output::raw_stderr(&hint);
}

fn active_toolchain_manifest(cwd: &vt_path::AbsolutePath) -> Option<vp_toolchain::Manifest> {
    let manifest_path =
        if let Some(bin_js) = crate::js_executor::JsExecutor::resolve_local_vite_plus(cwd) {
            bin_js.parent()?.join("toolchain.json")
        } else {
            crate::js_executor::JsExecutor::new(None).get_scripts_dir().ok()?.join("toolchain.json")
        };
    vp_toolchain::load_manifest(&manifest_path).ok()
}

async fn managed_install(
    packages: &[String],
    node: Option<&str>,
    force: bool,
    concurrency: Option<usize>,
) -> Result<ExitStatus, Error> {
    if let Err((package_name, error)) = global::install::install(
        packages,
        global::install::InstallOptions {
            node_version: node,
            force,
            concurrency: concurrency.unwrap_or(DEFAULT_GLOBAL_INSTALL_CONCURRENCY),
            update: false,
        },
    )
    .await
    {
        output::error(&format!(
            "Failed to install {}: {error}",
            package_name.as_deref().unwrap_or("global packages")
        ));
        return Ok(exit_status(1));
    }

    Ok(ExitStatus::default())
}

async fn managed_uninstall(packages: &[String], dry_run: bool) -> Result<ExitStatus, Error> {
    for package in packages {
        if let Err(e) = global::install::uninstall(package, dry_run).await {
            vp_shared::output::raw_stderr(&format!("Failed to uninstall {package}: {e}"));
            return Ok(exit_status(1));
        }
    }
    Ok(ExitStatus::default())
}

fn is_same_node_version(installed_version: &str, current_version: &str) -> bool {
    installed_version.trim().trim_start_matches('v')
        == current_version.trim().trim_start_matches('v')
}

fn display_node_version(version: &str) -> String {
    let version = version.trim();
    if version.starts_with('v') { version.to_string() } else { format!("v{version}") }
}

struct NodeMismatchPackage {
    name: String,
    spec: String,
    installed_node: String,
}

async fn managed_update(
    packages: &[String],
    latest: bool,
    concurrency: Option<usize>,
    reinstall_node_mismatch: bool,
    ignore_node_mismatch: bool,
) -> Result<ExitStatus, Error> {
    let concurrency = concurrency.unwrap_or(DEFAULT_GLOBAL_INSTALL_CONCURRENCY);
    let mut to_update: Vec<String> = Vec::new();
    let mut node_mismatches: Vec<NodeMismatchPackage> = Vec::new();
    // Recorded version-spec changes this update implies: `--latest` clears
    // specs, an explicit `pkg@spec` argument replaces the stored one.
    // Reinstalls record the new spec on their own, but packages already at
    // the target version are not reinstalled and must be rewritten here.
    // Entries are `(package name, new spec, registry query spec)`; the query
    // spec ties each rewrite to its lookup so failed resolutions never
    // persist a policy the update could not act on.
    let mut spec_rewrites: Vec<(String, Option<String>, String)> = Vec::new();
    let current_node_version;

    let packages = if packages.is_empty() {
        let all = PackageMetadata::list_all().await?;
        if all.is_empty() {
            vp_shared::output::raw("No global packages installed.");
            return Ok(ExitStatus::default());
        }
        current_node_version = get_current_node_version().await?;

        for metadata in &all {
            if latest && metadata.version_spec.is_some() {
                spec_rewrites.push((metadata.name.clone(), None, metadata.name.clone()));
            }
            if !is_same_node_version(&metadata.platform.node, &current_node_version) {
                node_mismatches.push(NodeMismatchPackage {
                    name: metadata.name.clone(),
                    spec: if latest { metadata.name.clone() } else { metadata.update_spec() },
                    installed_node: metadata.platform.node.clone(),
                });
            }
        }

        None
    } else {
        let mut managed_specs = Vec::new();
        current_node_version = get_current_node_version().await?;

        for package in packages {
            // Always update local packages
            if global::is_local_package_spec(package) {
                to_update.push(package.clone());
                continue;
            }

            // It is not a local package, so `parse_package_spec` there won't return `Err()`
            let (package_name, version_spec) = global::parse_package_spec(package).unwrap();
            if let Some(metadata) = PackageMetadata::load(&package_name).await? {
                if version_spec.is_some() {
                    // An explicit spec replaces the recorded one even when
                    // the installed version already satisfies it.
                    let new_spec = global::update_version_spec(package);
                    if new_spec != metadata.version_spec {
                        spec_rewrites.push((package_name.clone(), new_spec, package.clone()));
                    }
                } else if latest && metadata.version_spec.is_some() {
                    // `--latest` applies to bare names only; explicit specs win.
                    spec_rewrites.push((package_name.clone(), None, package_name.clone()));
                }
                if !is_same_node_version(&metadata.platform.node, &current_node_version) {
                    // Match the spec `get_outdated_packages` resolves for this
                    // package, so the dedup against outdated results holds.
                    let spec = if version_spec.is_some() {
                        package.clone()
                    } else if latest {
                        package_name.clone()
                    } else {
                        metadata.update_spec()
                    };
                    node_mismatches.push(NodeMismatchPackage {
                        name: package_name,
                        spec,
                        installed_node: metadata.platform.node,
                    });
                }
                managed_specs.push(package.clone());
            } else {
                to_update.push(package.clone());
            }
        }

        Some(managed_specs)
    };

    let report = global::outdated::get_outdated_packages(
        &packages.unwrap_or_default(),
        concurrency * 3,
        latest,
        global::outdated::LookupMode::WantedOnly,
    )
    .await?;
    for (_, message) in &report.failures {
        output::warn(&format!("{message}; skipping"));
    }
    // Skipped lookups make the update incomplete; keep going but exit
    // nonzero so scripts can tell.
    let incomplete = !report.failures.is_empty();
    to_update.extend(
        report
            .outdated
            .into_iter()
            // A newer `latest` alone (e.g. a version-pinned package) is not
            // updatable; only a newer wanted version is.
            .filter(|package| package.wanted != package.current)
            .map(|package| package.spec.unwrap_or(package.name)),
    );

    let outdated_specs = to_update.iter().map(String::as_str).collect::<HashSet<_>>();
    node_mismatches.retain(|package| !outdated_specs.contains(package.spec.as_str()));

    if should_reinstall_node_mismatches(
        &node_mismatches,
        &current_node_version,
        reinstall_node_mismatch,
        ignore_node_mismatch,
    ) {
        to_update.extend(node_mismatches.into_iter().map(|package| package.spec));
    }

    // Installs save the new spec only after they succeed.
    let to_update_set = to_update.iter().map(String::as_str).collect::<HashSet<_>>();
    let failed_specs =
        report.failures.iter().map(|(spec, _)| spec.as_str()).collect::<HashSet<_>>();
    for (package_name, new_spec, query_spec) in &spec_rewrites {
        if failed_specs.contains(query_spec.as_str()) || to_update_set.contains(query_spec.as_str())
        {
            continue;
        }
        if let Some(mut metadata) = PackageMetadata::load(package_name).await?
            && metadata.version_spec != *new_spec
        {
            metadata.version_spec = new_spec.clone();
            metadata.save().await?;
        }
    }

    if to_update.is_empty() {
        vp_shared::output::raw("All global packages are up to date.");
        return Ok(if incomplete { exit_status(1) } else { ExitStatus::default() });
    }

    // Call reinstall logic
    if let Err((package_name, error)) = global::install::install(
        &to_update,
        global::install::InstallOptions {
            node_version: Some(&current_node_version),
            force: false,
            concurrency,
            update: true,
        },
    )
    .await
    {
        output::error(&format!(
            "Failed to update {}: {error}",
            package_name.as_deref().unwrap_or("global packages")
        ));
        return Ok(exit_status(1));
    }
    Ok(if incomplete { exit_status(1) } else { ExitStatus::default() })
}

async fn get_current_node_version() -> Result<String, Error> {
    let cwd = vt_path::current_dir()?;
    Ok(resolve_version(&cwd).await?.version)
}

fn should_reinstall_node_mismatches(
    packages: &[NodeMismatchPackage],
    current_node_version: &str,
    reinstall_node_mismatch: bool,
    ignore_node_mismatch: bool,
) -> bool {
    if packages.is_empty() || ignore_node_mismatch {
        return false;
    }

    if reinstall_node_mismatch {
        return true;
    }

    if !vp_shared::is_stdin_terminal() || std::env::var_os("CI").is_some() {
        let package_names =
            packages.iter().map(|package| package.name.as_str()).collect::<Vec<_>>().join(", ");
        output::warn(&format!(
            "Skipping reinstall for global packages installed with a different Node.js version: {package_names}. Use --reinstall-node-mismatch to reinstall them."
        ));
        return false;
    }

    prompt_reinstall_node_mismatches(packages, current_node_version)
}

fn prompt_reinstall_node_mismatches(
    packages: &[NodeMismatchPackage],
    current_node_version: &str,
) -> bool {
    output::info("Some global packages were installed with a different Node.js version.");
    output::raw("");
    output::raw(&format!("Current Node.js: {}", display_node_version(current_node_version).bold()));
    output::raw("");
    output::raw("Affected packages:");
    for package in packages {
        output::raw(&format!(
            "- {} (installed with {})",
            package.name.bold(),
            display_node_version(&package.installed_node).bold()
        ));
    }
    output::raw("");
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Reinstall them with the current Node.js version?")
        .default(false)
        .interact()
        .unwrap_or(false)
}

/// The subcommand as the user wrote it, taken from `argv` before any rewriting.
///
/// Parsing resolves a subcommand to one clap variant, which does not record the
/// spelling used, so it is read straight from the command line instead.
#[must_use]
pub fn raw_subcommand(argv: &[String]) -> Option<&str> {
    argv.iter().skip(1).map(String::as_str).find(|arg| !arg.starts_with('-'))
}

/// Run the CLI command.
pub async fn run_command(
    cwd: AbsolutePathBuf,
    args: Args,
    raw_subcommand: Option<&str>,
    explicit_chdir: bool,
) -> Result<ExitStatus, Error> {
    run_command_with_options(cwd, args, RenderOptions::default(), raw_subcommand, explicit_chdir)
        .await
}

/// Run the CLI command with rendering options.
pub async fn run_command_with_options(
    mut cwd: AbsolutePathBuf,
    args: Args,
    render_options: RenderOptions,
    raw_subcommand: Option<&str>,
    explicit_chdir: bool,
) -> Result<ExitStatus, Error> {
    // Apply the global `-C <dir>` flag before anything reads cwd. main
    // normally consumes a leading `-C` pre-parse; this covers orderings that
    // reach clap (e.g. a second `-C`), with identical semantics: the shared
    // helper changes the process cwd, and delegated children receive PWD from
    // the resolved cwd at spawn time.
    let explicit_chdir = explicit_chdir || args.chdir.is_some();
    if let Some(dir) = &args.chdir {
        cwd =
            crate::apply_chdir(&cwd, dir).map_err(|message| Error::UserMessage(message.into()))?;
    }

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
        // Print the runtime header for `vp install` (when not silent).
        // Then intercept any `--global` paths that need vite-plus-managed
        // global install, falling through to `vp_pm_cli::dispatch` for
        // every project-scoped PM operation.
        Commands::PackageManager(pm_command) => {
            if let Some(silent) = pm_command.install_silent() {
                print_runtime_header(render_options.show_header && !silent);
            }
            run_package_manager_command(cwd, pm_command).await
        }

        // Category B: JS Script Commands
        Commands::Create { args } => commands::create::execute(cwd, &args, raw_subcommand).await,

        Commands::Migrate { args } => commands::migrate::execute(cwd, &args).await,

        Commands::Config { args } => commands::config::execute(cwd, &args, raw_subcommand).await,

        Commands::Hooks { args } => commands::hooks::execute(cwd, &args, raw_subcommand).await,

        Commands::Staged { args } => commands::staged::execute(cwd, &args, raw_subcommand).await,

        // Category C: Local CLI Delegation (forwarded to the local vite-plus CLI)
        Commands::Dev { args } => {
            maybe_print_runtime_header("dev", &args, render_options.show_header);
            commands::delegate::execute_app(cwd, "dev", &args, raw_subcommand, explicit_chdir).await
        }

        Commands::Build { args } => {
            maybe_print_runtime_header("build", &args, render_options.show_header);
            commands::delegate::execute_app(cwd, "build", &args, raw_subcommand, explicit_chdir)
                .await
        }

        Commands::Test { args } => {
            maybe_print_runtime_header("test", &args, render_options.show_header);
            commands::delegate::execute(cwd, "test", &args, raw_subcommand).await
        }

        Commands::Lint { args } => {
            maybe_print_runtime_header("lint", &args, render_options.show_header);
            if should_force_global_delegate("lint", &args) {
                commands::delegate::execute_global(cwd, "lint", &args, raw_subcommand).await
            } else {
                commands::delegate::execute(cwd, "lint", &args, raw_subcommand).await
            }
        }

        Commands::Fmt { args } => {
            maybe_print_runtime_header("fmt", &args, render_options.show_header);
            if should_force_global_delegate("fmt", &args) {
                commands::delegate::execute_global(cwd, "fmt", &args, raw_subcommand).await
            } else {
                commands::delegate::execute(cwd, "fmt", &args, raw_subcommand).await
            }
        }

        Commands::Check { args } => {
            maybe_print_runtime_header("check", &args, render_options.show_header);
            commands::delegate::execute(cwd, "check", &args, raw_subcommand).await
        }

        Commands::Pack { args } => {
            maybe_print_runtime_header("pack", &args, render_options.show_header);
            commands::delegate::execute_app(cwd, "pack", &args, raw_subcommand, explicit_chdir)
                .await
        }

        Commands::Run { args } => {
            maybe_print_runtime_header("run", &args, render_options.show_header);
            commands::delegate::execute(cwd, "run", &args, raw_subcommand).await
        }

        Commands::Exec { args } => {
            maybe_print_runtime_header("exec", &args, render_options.show_header);
            commands::delegate::execute(cwd, "exec", &args, raw_subcommand).await
        }

        Commands::Preview { args } => {
            maybe_print_runtime_header("preview", &args, render_options.show_header);
            commands::delegate::execute_app(cwd, "preview", &args, raw_subcommand, explicit_chdir)
                .await
        }

        Commands::Cache { args } => {
            maybe_print_runtime_header("cache", &args, render_options.show_header);
            commands::delegate::execute(cwd, "cache", &args, raw_subcommand).await
        }

        Commands::Toolchain { tools, json, global } => {
            commands::toolchain::execute(cwd, tools, json, global, raw_subcommand).await
        }

        Commands::Env(args) => commands::env::execute(cwd, args).await,

        // Self-Management
        Commands::Upgrade {
            version,
            tag,
            check,
            rollback,
            force,
            silent,
            registry,
            background_check,
        } => {
            commands::upgrade::execute(commands::upgrade::UpgradeOptions {
                version,
                tag,
                check,
                rollback,
                force,
                silent,
                registry,
                background_check,
            })
            .await
        }
        Commands::Implode { yes } => commands::implode::execute(yes),
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
    vp_shared::header::print_header();
}

fn maybe_print_runtime_header(command: &str, args: &[String], show_header: bool) {
    // Delegated help renders its own header from the project-local CLI. Normal commands do not,
    // so the global launcher prints it before resolving or downloading the managed runtime.
    let delegates_help = match command {
        "run" | "exec" | "cache" => {
            matches!(args, [arg] if matches!(arg.as_str(), "-h" | "--help"))
        }
        _ => help::has_help_flag_before_terminator(args),
    };
    if delegates_help || should_suppress_header_for_subcommand(command, args) {
        return;
    }
    print_runtime_header(show_header);
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
    let header = if render_options.show_header && vp_shared::header::should_print_header() {
        vp_shared::header::vite_plus_header()
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

#[cfg(test)]
mod tests {
    use super::{
        display_node_version, has_flag_before_terminator, is_same_node_version, raw_subcommand,
        should_force_global_delegate, should_suppress_header_for_subcommand,
    };

    fn argv(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| (*arg).to_string()).collect()
    }

    #[test]
    fn raw_subcommand_is_the_token_as_written() {
        assert_eq!(raw_subcommand(&argv(&["vp", "fmt", "src/"])), Some("fmt"));
        assert_eq!(raw_subcommand(&argv(&["vp", "format", "src/"])), Some("format"));
    }

    #[test]
    fn raw_subcommand_skips_leading_flags() {
        assert_eq!(raw_subcommand(&argv(&["vp", "--silent", "install"])), Some("install"));
    }

    #[test]
    fn raw_subcommand_is_none_without_a_subcommand() {
        assert_eq!(raw_subcommand(&argv(&["vp"])), None);
        assert_eq!(raw_subcommand(&argv(&["vp", "--version"])), None);
    }

    #[test]
    fn detects_global_update_node_version_mismatch() {
        assert!(is_same_node_version("21.0.0", "v21.0.0"));
        assert!(!is_same_node_version("21.0.0", "25.0.0"));
    }

    #[test]
    fn displays_node_versions_with_v_prefix() {
        assert_eq!(display_node_version("25.0.0"), "v25.0.0");
        assert_eq!(display_node_version("v25.0.0"), "v25.0.0");
    }

    #[test]
    fn detects_flag_before_option_terminator() {
        assert!(has_flag_before_terminator(
            &["--init".to_string(), "src/index.ts".to_string()],
            "--init"
        ));
    }

    #[test]
    fn ignores_flag_after_option_terminator() {
        assert!(!has_flag_before_terminator(
            &["src/index.ts".to_string(), "--".to_string(), "--init".to_string(),],
            "--init"
        ));
    }

    #[test]
    fn lint_init_forces_global_delegate() {
        assert!(should_force_global_delegate("lint", &["--init".to_string()]));
    }

    #[test]
    fn fmt_migrate_forces_global_delegate() {
        assert!(should_force_global_delegate("fmt", &["--migrate=prettier".to_string()]));
    }

    #[test]
    fn non_init_does_not_force_global_delegate() {
        assert!(!should_force_global_delegate("lint", &["src/index.ts".to_string()]));
        assert!(!should_force_global_delegate("fmt", &["--check".to_string()]));
    }

    #[test]
    fn lint_lsp_suppresses_header() {
        assert!(should_suppress_header_for_subcommand("lint", &["--lsp".to_string()]));
        assert!(should_suppress_header_for_subcommand(
            "lint",
            &["--fix".to_string(), "--lsp".to_string()]
        ));
    }

    #[test]
    fn lint_without_lsp_does_not_suppress_header() {
        assert!(!should_suppress_header_for_subcommand("lint", &[]));
        assert!(!should_suppress_header_for_subcommand("lint", &["src".to_string()]));
        assert!(!should_suppress_header_for_subcommand("lint", &["--fix".to_string()]));
    }

    #[test]
    fn lint_lsp_after_terminator_does_not_suppress_header() {
        assert!(!should_suppress_header_for_subcommand(
            "lint",
            &["--".to_string(), "--lsp".to_string()]
        ));
    }

    #[test]
    fn fmt_lsp_or_stdin_filepath_suppresses_header() {
        assert!(should_suppress_header_for_subcommand("fmt", &["--lsp".to_string()]));
        assert!(should_suppress_header_for_subcommand(
            "fmt",
            &["--stdin-filepath".to_string(), "foo.ts".to_string()]
        ));
        assert!(should_suppress_header_for_subcommand(
            "fmt",
            &["--stdin-filepath=foo.ts".to_string()]
        ));
    }

    #[test]
    fn fmt_without_lsp_or_stdin_does_not_suppress_header() {
        assert!(!should_suppress_header_for_subcommand("fmt", &[]));
        assert!(!should_suppress_header_for_subcommand("fmt", &["src".to_string()]));
        assert!(!should_suppress_header_for_subcommand("fmt", &["--check".to_string()]));
    }

    #[test]
    fn unknown_subcommand_does_not_suppress_header() {
        assert!(!should_suppress_header_for_subcommand("test", &["--lsp".to_string()]));
    }
}
