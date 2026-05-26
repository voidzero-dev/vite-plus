//! CLI argument parsing and command routing.
//!
//! This module defines the CLI structure using clap and routes commands
//! to their appropriate handlers.

use std::{collections::HashSet, ffi::OsStr, io::IsTerminal, process::ExitStatus};

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use clap_complete::ArgValueCompleter;
use dialoguer::{Confirm, theme::ColorfulTheme};
use owo_colors::OwoColorize;
use tokio::runtime::Runtime;
use vite_path::AbsolutePathBuf;
use vite_pm_cli::PackageManagerCommand;
use vite_shared::output;

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

    #[clap(subcommand)]
    pub command: Option<Commands>,
}

/// Available commands
#[derive(Subcommand, Debug)]
pub enum Commands {
    // =========================================================================
    // Category A: Package Manager Commands
    // (clap-flattened from `vite_pm_cli::PackageManagerCommand` so the
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

    /// Pin a Node.js version in the current directory (creates .node-version)
    #[command(after_long_help = "\
Examples:
  vp env pin lts                  # Pin to latest LTS
  vp env pin --unpin              # Remove .node-version
  vp env pin \"^20.0.0\" --force    # Overwrite existing pin")]
    Pin {
        /// Version to pin (e.g., "20.18.0", "lts", "latest", "^20.0.0").
        /// If omitted, prints the currently pinned version.
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

    /// Install a Node.js version
    #[command(visible_alias = "i")]
    Install {
        /// Version to install (e.g., "20", "20.18.0", "lts", "latest")
        /// If not provided, installs the version from .node-version or package.json
        version: Option<String>,
    },

    /// Use a specific Node.js version for this shell session
    #[command(after_long_help = "\
Examples:
  vp env use lts        # Override session with latest LTS
  vp env use --unset    # Clear the session override")]
    Use {
        /// Version to use (e.g., "20", "20.18.0", "lts", "latest").
        /// If omitted, reads from .node-version or package.json.
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
    let Ok(cwd) = vite_path::current_dir() else {
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
/// `Install`/`Add`/`Update`/`Remove` invoked with `-g`/`--global` are routed
/// through the vite-plus-managed Node.js install store (`commands::global`).
/// Everything else is forwarded to `vite_pm_cli::dispatch`, which executes
/// the underlying package manager (pnpm/npm/yarn/bun).
async fn run_package_manager_command(
    cwd: AbsolutePathBuf,
    command: PackageManagerCommand,
) -> Result<ExitStatus, Error> {
    match command {
        PackageManagerCommand::Install {
            global: true,
            packages: Some(pkgs),
            node,
            force,
            concurrency,
            ..
        } if !pkgs.is_empty() => managed_install(&pkgs, node.as_deref(), force, concurrency).await,

        PackageManagerCommand::Add {
            global: true, ref packages, ref node, concurrency, ..
        } => managed_install(packages, node.as_deref(), false, concurrency).await,

        PackageManagerCommand::Remove { global: true, ref packages, dry_run, .. } => {
            managed_uninstall(packages, dry_run).await
        }

        PackageManagerCommand::Update {
            global: true,
            ref packages,
            concurrency,
            reinstall_node_mismatch,
            ignore_node_mismatch,
            ..
        } => {
            if reinstall_node_mismatch && ignore_node_mismatch {
                output::error(
                    "--reinstall-node-mismatch and --ignore-node-mismatch cannot be used together",
                );
                return Ok(exit_status(1));
            }
            managed_update(packages, concurrency, reinstall_node_mismatch, ignore_node_mismatch)
                .await
        }

        PackageManagerCommand::Outdated {
            global: true,
            ref packages,
            long,
            format,
            concurrency,
            ..
        } => {
            global::outdated::execute(
                packages,
                long,
                format,
                concurrency.unwrap_or(DEFAULT_GLOBAL_VIEW_CONCURRENCY),
            )
            .await
        }

        // `pm list -g` lists vite-plus-managed globals, not the underlying PM's.
        PackageManagerCommand::Pm(vite_pm_cli::cli::PmCommands::List {
            global: true,
            json,
            ref pattern,
            ..
        }) => global::packages::execute(json, pattern.as_deref()).await,

        cmd => {
            commands::prepend_js_runtime_to_path_env(&cwd).await?;
            Ok(vite_pm_cli::dispatch(&cwd, cmd).await?)
        }
    }
}

async fn managed_install(
    packages: &[String],
    node: Option<&str>,
    force: bool,
    concurrency: Option<usize>,
) -> Result<ExitStatus, Error> {
    if let Err((package_name, error)) = global::install::install(
        packages,
        node,
        force,
        concurrency.unwrap_or(DEFAULT_GLOBAL_INSTALL_CONCURRENCY),
        false,
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
            vite_shared::output::raw_stderr(&format!("Failed to uninstall {package}: {e}"));
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
    concurrency: Option<usize>,
    reinstall_node_mismatch: bool,
    ignore_node_mismatch: bool,
) -> Result<ExitStatus, Error> {
    let concurrency = concurrency.unwrap_or(DEFAULT_GLOBAL_INSTALL_CONCURRENCY);
    let mut to_update: Vec<String> = Vec::new();
    let mut node_mismatches: Vec<NodeMismatchPackage> = Vec::new();
    let current_node_version;

    let packages = if packages.is_empty() {
        let all = PackageMetadata::list_all().await?;
        if all.is_empty() {
            vite_shared::output::raw("No global packages installed.");
            return Ok(ExitStatus::default());
        }
        current_node_version = get_current_node_version().await?;

        for metadata in &all {
            if !is_same_node_version(&metadata.platform.node, &current_node_version) {
                node_mismatches.push(NodeMismatchPackage {
                    name: metadata.name.clone(),
                    spec: metadata.name.clone(),
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

            let (package_name, _) = global::parse_package_spec(package);
            if let Some(metadata) = PackageMetadata::load(&package_name).await? {
                if !is_same_node_version(&metadata.platform.node, &current_node_version) {
                    node_mismatches.push(NodeMismatchPackage {
                        name: package_name,
                        spec: package.clone(),
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

    let outdated = global::outdated::get_outdated_packages(
        &packages.unwrap_or_default(),
        concurrency * 3,
        true,
    )
    .await?;
    to_update.extend(outdated.into_iter().map(|package| package.spec.unwrap_or(package.name)));

    let to_update_set = to_update.iter().map(String::as_str).collect::<HashSet<_>>();
    node_mismatches.retain(|package| !to_update_set.contains(package.spec.as_str()));

    if should_reinstall_node_mismatches(
        &node_mismatches,
        &current_node_version,
        reinstall_node_mismatch,
        ignore_node_mismatch,
    ) {
        to_update.extend(node_mismatches.into_iter().map(|package| package.spec));
    }

    if to_update.is_empty() {
        vite_shared::output::raw("All global packages are up to date.");
        return Ok(ExitStatus::default());
    }

    // Call reinstall logic
    if let Err((package_name, error)) =
        global::install::install(&to_update, Some(&current_node_version), false, concurrency, true)
            .await
    {
        output::error(&format!(
            "Failed to update {}: {error}",
            package_name.as_deref().unwrap_or("global packages")
        ));
        return Ok(exit_status(1));
    }
    Ok(ExitStatus::default())
}

async fn get_current_node_version() -> Result<String, Error> {
    let cwd = vite_path::current_dir().map_err(|error| {
        Error::ConfigError(format!("Cannot get current directory: {error}").into())
    })?;
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

    if !std::io::stdin().is_terminal() || std::env::var_os("CI").is_some() {
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
        // Print the runtime header for `vp install` (when not silent).
        // Then intercept any `--global` paths that need vite-plus-managed
        // global install, falling through to `vite_pm_cli::dispatch` for
        // every project-scoped PM operation.
        Commands::PackageManager(pm_command) => {
            if let PackageManagerCommand::Install { silent, .. } = &pm_command {
                print_runtime_header(render_options.show_header && !*silent);
            }
            run_package_manager_command(cwd, pm_command).await
        }

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
            maybe_print_runtime_header("lint", &args, render_options.show_header);
            if should_force_global_delegate("lint", &args) {
                commands::delegate::execute_global(cwd, "lint", &args).await
            } else {
                commands::delegate::execute(cwd, "lint", &args).await
            }
        }

        Commands::Fmt { args } => {
            if help::maybe_print_unified_delegate_help("fmt", &args, render_options.show_header) {
                return Ok(ExitStatus::default());
            }
            maybe_print_runtime_header("fmt", &args, render_options.show_header);
            if should_force_global_delegate("fmt", &args) {
                commands::delegate::execute_global(cwd, "fmt", &args).await
            } else {
                commands::delegate::execute(cwd, "fmt", &args).await
            }
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
            commands::delegate::execute(cwd, "run", &args).await
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
    vite_shared::header::print_header();
}

fn maybe_print_runtime_header(command: &str, args: &[String], show_header: bool) {
    if should_suppress_header_for_subcommand(command, args) {
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
    let header = if render_options.show_header && vite_shared::header::should_print_header() {
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

#[cfg(test)]
mod tests {
    use super::{
        display_node_version, has_flag_before_terminator, is_same_node_version,
        should_force_global_delegate, should_suppress_header_for_subcommand,
    };

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
