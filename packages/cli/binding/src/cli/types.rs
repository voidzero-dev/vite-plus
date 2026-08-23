use std::{ffi::OsStr, future::Future, pin::Pin, sync::Arc};

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use vt::{ExitStatus, config::user::UserCacheConfig, plan_request::SyntheticPlanRequest};
use vt_str::Str;

/// Resolved configuration from vite.config.ts
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct ResolvedUniversalViteConfig {
    #[serde(rename = "configFile")]
    pub(crate) config_file: Option<String>,
    pub(crate) lint: Option<serde_json::Value>,
    pub(crate) fmt: Option<serde_json::Value>,
    pub(crate) check: Option<serde_json::Value>,
    pub(crate) run: Option<serde_json::Value>,
}

/// Result type for resolved commands from JavaScript
#[derive(Debug, Clone)]
pub struct ResolveCommandResult {
    pub bin_path: Arc<OsStr>,
    pub envs: Vec<(String, String)>,
}

/// Built-in subcommands that resolve to a concrete tool (oxlint, vitest, vite, etc.)
#[derive(Debug, Clone)]
pub enum SynthesizableSubcommand {
    /// Lint code
    Lint { args: Vec<String> },
    /// Format code
    Fmt { args: Vec<String> },
    /// Build for production
    Build { args: Vec<String> },
    /// Run tests
    Test { args: Vec<String> },
    /// Build library
    Pack { args: Vec<String> },
    /// Run the development server
    Dev { args: Vec<String> },
    /// Preview production build
    Preview { args: Vec<String> },
    /// Build documentation
    Doc { args: Vec<String> },
    /// Run format, lint, and type checks
    Check {
        /// Auto-fix format and lint issues
        fix: bool,
        /// Skip format check
        no_fmt: bool,
        /// Skip lint rules; type-check still runs when `lint.options.typeCheck` is true
        no_lint: bool,
        /// Do not exit with error when pattern is unmatched
        no_error_on_unmatched_pattern: bool,
        /// File paths to check (passed through to fmt and lint)
        paths: Vec<String>,
    },
}

#[derive(Debug, usage_rs::Args)]
#[usage(args_override_self = false)]
pub struct ToolchainArgs {
    /// Tool or package names to show
    #[usage(value_name = "TOOLS")]
    pub tools: Vec<String>,

    /// Print the graph as JSON
    #[usage(long)]
    pub json: bool,

    /// Use the global Vite+ toolchain
    #[usage(long)]
    pub global: bool,
}

#[derive(Debug, usage_rs::Args)]
#[usage(args_override_self = false, disable_help_flag = true)]
struct PassthroughArgs {
    #[usage(double_dash = "automatic", value_name = "ARGS")]
    args: Vec<String>,
}

#[derive(Debug, usage_rs::Args)]
#[usage(args_override_self = false)]
struct CheckArgs {
    /// Auto-fix format and lint issues
    #[usage(long)]
    fix: bool,
    /// Skip format check
    #[usage(long = "no-fmt")]
    no_fmt: bool,
    /// Skip lint rules; type-check still runs when `lint.options.typeCheck` is true
    #[usage(long = "no-lint")]
    no_lint: bool,
    /// Do not exit with error when pattern is unmatched
    #[usage(long = "no-error-on-unmatched-pattern")]
    no_error_on_unmatched_pattern: bool,
    /// File paths to check (passed through to fmt and lint)
    #[usage(double_dash = "automatic", value_name = "PATH")]
    paths: Vec<String>,
}

#[derive(Debug, usage_rs::Subcommands)]
enum LocalCommand {
    /// Lint code
    Lint(PassthroughArgs),
    /// Format code
    #[usage(visible_alias = "format")]
    Fmt(PassthroughArgs),
    /// Build for production
    Build(PassthroughArgs),
    /// Run tests
    Test(PassthroughArgs),
    /// Build library
    Pack(PassthroughArgs),
    /// Run the development server
    Dev(PassthroughArgs),
    /// Preview production build
    Preview(PassthroughArgs),
    /// Build documentation
    #[usage(hide)]
    Doc(PassthroughArgs),
    /// Run format, lint, and type checks
    Check(CheckArgs),
    /// Execute a command from local node_modules/.bin
    Exec(crate::exec::ExecArgs),
    /// Show active Vite+ tools, versions, and relationships
    Toolchain(ToolchainArgs),
}

#[derive(Debug, usage_rs::Cli)]
#[usage(
    bin = "vp",
    unknown_flags = "error",
    args_override_self = false,
    disable_help_subcommand = true
)]
pub(super) struct LocalCli {
    #[usage(subcommand)]
    command: LocalCommand,
}

/// Parsed command from one of the local CLI parser trees.
#[derive(Debug)]
pub(super) enum CLIArgs {
    ViteTask(vt::Command),
    Synthesizable(SynthesizableSubcommand),
    PackageManager(vp_pm_cli::PackageManagerCommand),
    Exec(crate::exec::ExecArgs),
    Toolchain(ToolchainArgs),
}

impl From<LocalCli> for CLIArgs {
    fn from(cli: LocalCli) -> Self {
        let command = match cli.command {
            LocalCommand::Lint(args) => SynthesizableSubcommand::Lint { args: args.args },
            LocalCommand::Fmt(args) => SynthesizableSubcommand::Fmt { args: args.args },
            LocalCommand::Build(args) => SynthesizableSubcommand::Build { args: args.args },
            LocalCommand::Test(args) => SynthesizableSubcommand::Test { args: args.args },
            LocalCommand::Pack(args) => SynthesizableSubcommand::Pack { args: args.args },
            LocalCommand::Dev(args) => SynthesizableSubcommand::Dev { args: args.args },
            LocalCommand::Preview(args) => SynthesizableSubcommand::Preview { args: args.args },
            LocalCommand::Doc(args) => SynthesizableSubcommand::Doc { args: args.args },
            LocalCommand::Check(args) => SynthesizableSubcommand::Check {
                fix: args.fix,
                no_fmt: args.no_fmt,
                no_lint: args.no_lint,
                no_error_on_unmatched_pattern: args.no_error_on_unmatched_pattern,
                paths: args.paths,
            },
            LocalCommand::Exec(args) => return Self::Exec(args),
            LocalCommand::Toolchain(args) => return Self::Toolchain(args),
        };
        Self::Synthesizable(command)
    }
}

/// Type alias for boxed async resolver function
/// NOTE: Uses anyhow::Error to avoid NAPI type inference issues
pub type BoxedResolverFn =
    Box<dyn Fn() -> Pin<Box<dyn Future<Output = anyhow::Result<ResolveCommandResult>> + 'static>>>;

/// Type alias for vite config resolver function (takes package path, returns JSON string)
/// Uses Arc for cloning and Send + Sync for use in UserConfigLoader
pub type ViteConfigResolverFn = Arc<
    dyn Fn(String) -> Pin<Box<dyn Future<Output = anyhow::Result<String>> + Send + 'static>>
        + Send
        + Sync,
>;

/// CLI options containing JavaScript resolver functions (using boxed futures for simplicity)
pub struct CliOptions {
    pub lint: BoxedResolverFn,
    pub fmt: BoxedResolverFn,
    pub vite: BoxedResolverFn,
    pub test: BoxedResolverFn,
    pub pack: BoxedResolverFn,
    pub doc: BoxedResolverFn,
    pub toolchain_manifest_path: String,
    pub vite_plus_package_path: String,
    pub resolve_universal_vite_config: ViteConfigResolverFn,
}

/// A resolved subcommand ready for execution.
pub(super) struct ResolvedSubcommand {
    pub(super) program: Arc<OsStr>,
    pub(super) args: Arc<[Str]>,
    pub(super) cache_config: UserCacheConfig,
    pub(super) envs: Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>>,
}

impl ResolvedSubcommand {
    pub(super) fn into_synthetic_plan_request(self) -> SyntheticPlanRequest {
        SyntheticPlanRequest {
            program: self.program,
            args: self.args,
            cache_config: self.cache_config,
            envs: self.envs,
        }
    }
}

pub(crate) struct CapturedCommandOutput {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

/// Convert a child's exit status to the vite-task `ExitStatus`, preserving
/// the `128 + signal` mapping. A `From` impl is blocked by the orphan rule:
/// both types are foreign here.
pub(crate) fn exit_status_from(status: std::process::ExitStatus) -> ExitStatus {
    ExitStatus(vp_shared::exit_code_from_status(status) as u8)
}
