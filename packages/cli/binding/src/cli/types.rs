use std::{ffi::OsStr, future::Future, pin::Pin, sync::Arc};

use clap::{Parser, Subcommand};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use vite_str::Str;
use vite_task::{
    Command, ExitStatus, config::user::UserCacheConfig, plan_request::SyntheticPlanRequest,
};

/// Resolved configuration from vite.config.ts
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct ResolvedUniversalViteConfig {
    #[serde(rename = "configFile")]
    pub(crate) config_file: Option<String>,
    pub(crate) lint: Option<serde_json::Value>,
    pub(crate) fmt: Option<serde_json::Value>,
    pub(crate) run: Option<serde_json::Value>,
}

/// Result type for resolved commands from JavaScript
#[derive(Debug, Clone)]
pub struct ResolveCommandResult {
    pub bin_path: Arc<OsStr>,
    pub envs: Vec<(String, String)>,
}

/// Built-in subcommands that resolve to a concrete tool (oxlint, vitest, vite, etc.)
#[derive(Debug, Clone, Subcommand)]
pub enum SynthesizableSubcommand {
    /// Lint code
    #[command(disable_help_flag = true)]
    Lint {
        #[clap(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Format code
    #[command(disable_help_flag = true)]
    Fmt {
        #[clap(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Build for production
    #[command(disable_help_flag = true)]
    Build {
        #[clap(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Run tests
    #[command(disable_help_flag = true)]
    Test {
        #[clap(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Build library
    #[command(disable_help_flag = true)]
    Pack {
        #[clap(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Run the development server
    #[command(disable_help_flag = true)]
    Dev {
        #[clap(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Preview production build
    #[command(disable_help_flag = true)]
    Preview {
        #[clap(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Build documentation
    #[command(disable_help_flag = true, hide = true)]
    Doc {
        #[clap(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Install command.
    #[command(disable_help_flag = true, alias = "i")]
    Install {
        #[clap(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Run format, lint, and type checks
    Check {
        /// Auto-fix format and lint issues
        #[arg(long)]
        fix: bool,
        /// Skip format check
        #[arg(long = "no-fmt")]
        no_fmt: bool,
        /// Skip lint check
        #[arg(long = "no-lint")]
        no_lint: bool,
        /// File paths to check (passed through to fmt and lint)
        #[arg(trailing_var_arg = true)]
        paths: Vec<String>,
    },
}

/// Top-level CLI argument parser for vite-plus.
#[derive(Debug, Parser)]
#[command(name = "vp", disable_help_subcommand = true)]
pub(super) enum CLIArgs {
    /// vite-task commands (run, cache)
    #[command(flatten)]
    ViteTask(Command),

    /// Built-in subcommands (lint, build, test, etc.)
    #[command(flatten)]
    Synthesizable(SynthesizableSubcommand),

    /// Execute a command from local node_modules/.bin
    Exec(crate::exec::ExecArgs),
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
