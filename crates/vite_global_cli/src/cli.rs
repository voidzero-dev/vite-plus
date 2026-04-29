//! CLI argument parsing and command routing.
//!
//! This module defines the CLI structure using clap and routes commands
//! to their appropriate handlers.

use std::{ffi::OsStr, process::ExitStatus};

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use clap_complete::ArgValueCompleter;
use tokio::runtime::Runtime;
use vite_path::AbsolutePathBuf;
use vite_pm_cli::PackageManagerCommand;

use crate::{commands, error::Error, help};

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
    Default {
        /// Version to set as default (e.g., "20.18.0", "lts", "latest")
        /// If not provided, shows the current default
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
/// through the vite-plus-managed Node.js install store (`commands::env`).
/// Everything else is forwarded to `vite_pm_cli::dispatch`, which executes
/// the underlying package manager (pnpm/npm/yarn/bun).
async fn run_package_manager_command(
    cwd: AbsolutePathBuf,
    command: PackageManagerCommand,
) -> Result<ExitStatus, Error> {
    match command {
        PackageManagerCommand::Install {
            global: true, packages: Some(pkgs), node, force, ..
        } if !pkgs.is_empty() => managed_install(&pkgs, node.as_deref(), force).await,

        PackageManagerCommand::Add { global: true, ref packages, ref node, .. } => {
            managed_install(packages, node.as_deref(), false).await
        }

        PackageManagerCommand::Remove { global: true, ref packages, dry_run, .. } => {
            managed_uninstall(packages, dry_run).await
        }

        PackageManagerCommand::Update { global: true, ref packages, .. } => {
            managed_update(packages).await
        }

        // `pm list -g` lists vite-plus-managed globals, not the underlying PM's.
        PackageManagerCommand::Pm(vite_pm_cli::cli::PmCommands::List {
            global: true,
            json,
            ref pattern,
            ..
        }) => crate::commands::env::packages::execute(json, pattern.as_deref()).await,

        cmd => {
            commands::prepend_js_runtime_to_path_env(&cwd).await?;
            Ok(vite_pm_cli::dispatch(&cwd, cmd).await?)
        }
    }
}

// snap-test fixtures expect bare lines (no "error:"/"info:" prefix), so
// these helpers use `output::raw_stderr`/`output::raw` rather than the
// prefixed `output::error`/`output::info`.
async fn managed_install(
    packages: &[String],
    node: Option<&str>,
    force: bool,
) -> Result<ExitStatus, Error> {
    for package in packages {
        if let Err(e) = crate::commands::env::global_install::install(package, node, force).await {
            vite_shared::output::raw_stderr(&format!("Failed to install {package}: {e}"));
            return Ok(exit_status(1));
        }
    }
    Ok(ExitStatus::default())
}

async fn managed_uninstall(packages: &[String], dry_run: bool) -> Result<ExitStatus, Error> {
    for package in packages {
        if let Err(e) = crate::commands::env::global_install::uninstall(package, dry_run).await {
            vite_shared::output::raw_stderr(&format!("Failed to uninstall {package}: {e}"));
            return Ok(exit_status(1));
        }
    }
    Ok(ExitStatus::default())
}

async fn managed_update(packages: &[String]) -> Result<ExitStatus, Error> {
    use crate::commands::env::package_metadata::PackageMetadata;

    let to_update: Vec<String> = if packages.is_empty() {
        let all = PackageMetadata::list_all().await?;
        if all.is_empty() {
            vite_shared::output::raw("No global packages installed.");
            return Ok(ExitStatus::default());
        }
        all.iter().map(|p| p.name.clone()).collect()
    } else {
        packages.to_vec()
    };
    for package in &to_update {
        if let Err(e) = crate::commands::env::global_install::install(package, None, false).await {
            vite_shared::output::raw_stderr(&format!("Failed to update {package}: {e}"));
            return Ok(exit_status(1));
        }
    }
    Ok(ExitStatus::default())
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
            print_runtime_header(render_options.show_header);
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
            print_runtime_header(render_options.show_header);
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
    use super::{has_flag_before_terminator, should_force_global_delegate};

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
}
