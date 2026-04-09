//! CLI types and logic for vite-plus using the new Session API from vite-task.
//!
//! This module contains all the CLI-related code.
//! It handles argument parsing, command dispatching, and orchestration of the task execution.

use std::{
    borrow::Cow, env, ffi::OsStr, future::Future, io::IsTerminal, iter, pin::Pin, process::Stdio,
    sync::Arc,
};

use clap::{
    Parser, Subcommand,
    error::{ContextKind, ContextValue, ErrorKind},
};
use cow_utils::CowUtils;
use owo_colors::OwoColorize;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use vite_error::Error;
use vite_path::{AbsolutePath, AbsolutePathBuf};
use vite_shared::{PrependOptions, output, prepend_to_path_env};
use vite_str::Str;
use vite_task::{
    Command, CommandHandler, ExitStatus, HandledCommand, ScriptCommand, Session, SessionConfig,
    config::{
        UserRunConfig,
        user::{
            AutoInput, EnabledCacheConfig, GlobWithBase, InputBase, UserCacheConfig, UserInputEntry,
        },
    },
    loader::UserConfigLoader,
    plan_request::SyntheticPlanRequest,
};

/// Resolved configuration from vite.config.ts
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResolvedUniversalViteConfig {
    #[serde(rename = "configFile")]
    pub config_file: Option<String>,
    pub lint: Option<serde_json::Value>,
    pub fmt: Option<serde_json::Value>,
    pub run: Option<serde_json::Value>,
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
enum CLIArgs {
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
struct ResolvedSubcommand {
    program: Arc<OsStr>,
    args: Arc<[Str]>,
    cache_config: UserCacheConfig,
    envs: Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>>,
}

impl ResolvedSubcommand {
    fn into_synthetic_plan_request(self) -> SyntheticPlanRequest {
        SyntheticPlanRequest {
            program: self.program,
            args: self.args,
            cache_config: self.cache_config,
            envs: self.envs,
        }
    }
}

/// Resolves synthesizable subcommands to concrete programs and arguments.
/// Used by both direct CLI execution and CommandHandler.
pub struct SubcommandResolver {
    cli_options: Option<CliOptions>,
    workspace_path: Arc<AbsolutePath>,
}

impl std::fmt::Debug for SubcommandResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubcommandResolver")
            .field("has_cli_options", &self.cli_options.is_some())
            .field("workspace_path", &self.workspace_path)
            .finish()
    }
}

impl SubcommandResolver {
    pub fn new(workspace_path: Arc<AbsolutePath>) -> Self {
        Self { cli_options: None, workspace_path }
    }

    pub fn with_cli_options(mut self, cli_options: CliOptions) -> Self {
        self.cli_options = Some(cli_options);
        self
    }

    pub(crate) async fn resolve_universal_vite_config(
        &self,
    ) -> anyhow::Result<ResolvedUniversalViteConfig> {
        let cli_options = self
            .cli_options
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("CLI options required for vite config resolution"))?;
        let workspace_path_str = self
            .workspace_path
            .as_path()
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("workspace path is not valid UTF-8"))?;
        let vite_config_json =
            (cli_options.resolve_universal_vite_config)(workspace_path_str.to_string()).await?;

        Ok(serde_json::from_str(&vite_config_json).inspect_err(|_| {
            tracing::error!("Failed to parse vite config: {vite_config_json}");
        })?)
    }

    /// Resolve a synthesizable subcommand to a concrete program, args, cache config, and envs.
    async fn resolve(
        &self,
        subcommand: SynthesizableSubcommand,
        resolved_vite_config: Option<&ResolvedUniversalViteConfig>,
        envs: &Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>>,
        cwd: &Arc<AbsolutePath>,
    ) -> anyhow::Result<ResolvedSubcommand> {
        match subcommand {
            SynthesizableSubcommand::Lint { mut args } => {
                let cli_options = self
                    .cli_options
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("CLI options required for lint command"))?;
                let resolved = (cli_options.lint)().await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("lint JS path is not valid UTF-8"))?;
                let owned_resolved_vite_config;
                let resolved_vite_config = if let Some(config) = resolved_vite_config {
                    config
                } else {
                    owned_resolved_vite_config = self.resolve_universal_vite_config().await?;
                    &owned_resolved_vite_config
                };

                if let (Some(_), Some(config_file)) =
                    (&resolved_vite_config.lint, &resolved_vite_config.config_file)
                {
                    args.insert(0, "-c".to_string());
                    args.insert(1, config_file.clone());
                }

                Ok(ResolvedSubcommand {
                    program: Arc::from(OsStr::new("node")),
                    args: iter::once(Str::from("--disable-warning=MODULE_TYPELESS_PACKAGE_JSON"))
                        .chain(iter::once(Str::from(js_path_str)))
                        .chain(args.into_iter().map(Str::from))
                        .collect(),
                    cache_config: UserCacheConfig::with_config(EnabledCacheConfig {
                        env: Some(Box::new([Str::from("OXLINT_TSGOLINT_PATH")])),
                        untracked_env: None,
                        input: None,
                    }),
                    envs: merge_resolved_envs(envs, resolved.envs),
                })
            }
            SynthesizableSubcommand::Fmt { mut args } => {
                let cli_options = self
                    .cli_options
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("CLI options required for fmt command"))?;
                let resolved = (cli_options.fmt)().await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("fmt JS path is not valid UTF-8"))?;
                let owned_resolved_vite_config;
                let resolved_vite_config = if let Some(config) = resolved_vite_config {
                    config
                } else {
                    owned_resolved_vite_config = self.resolve_universal_vite_config().await?;
                    &owned_resolved_vite_config
                };

                if let (Some(_), Some(config_file)) =
                    (&resolved_vite_config.fmt, &resolved_vite_config.config_file)
                {
                    args.insert(0, "-c".to_string());
                    args.insert(1, config_file.clone());
                }

                Ok(ResolvedSubcommand {
                    program: Arc::from(OsStr::new("node")),
                    args: iter::once(Str::from(js_path_str))
                        .chain(args.into_iter().map(Str::from))
                        .collect(),
                    cache_config: UserCacheConfig::with_config(EnabledCacheConfig {
                        env: None,
                        untracked_env: None,
                        input: None,
                    }),
                    envs: merge_resolved_envs(envs, resolved.envs),
                })
            }
            SynthesizableSubcommand::Build { args } => {
                let cli_options = self
                    .cli_options
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("CLI options required for build command"))?;
                let resolved = (cli_options.vite)().await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("vite JS path is not valid UTF-8"))?;

                Ok(ResolvedSubcommand {
                    program: Arc::from(OsStr::new("node")),
                    args: iter::once(Str::from(js_path_str))
                        .chain(iter::once(Str::from("build")))
                        .chain(args.into_iter().map(Str::from))
                        .collect(),
                    cache_config: UserCacheConfig::with_config(EnabledCacheConfig {
                        env: Some(Box::new([Str::from("VITE_*")])),
                        untracked_env: None,
                        input: Some(build_pack_cache_inputs()),
                    }),
                    envs: merge_resolved_envs_with_version(envs, resolved.envs),
                })
            }
            SynthesizableSubcommand::Test { args } => {
                let cli_options = self
                    .cli_options
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("CLI options required for test command"))?;
                let resolved = (cli_options.test)().await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("test JS path is not valid UTF-8"))?;
                let prepend_run = should_prepend_vitest_run(&args);
                let vitest_args: Vec<Str> = if prepend_run {
                    iter::once(Str::from("run")).chain(args.into_iter().map(Str::from)).collect()
                } else {
                    args.into_iter().map(Str::from).collect()
                };

                Ok(ResolvedSubcommand {
                    program: Arc::from(OsStr::new("node")),
                    args: iter::once(Str::from(js_path_str)).chain(vitest_args).collect(),
                    cache_config: UserCacheConfig::with_config(EnabledCacheConfig {
                        env: None,
                        untracked_env: None,
                        input: Some(vec![
                            UserInputEntry::Auto(AutoInput { auto: true }),
                            exclude_glob("!node_modules/.vite-temp/**", InputBase::Package),
                            exclude_glob(
                                "!node_modules/.vite/vitest/**/results.json",
                                InputBase::Package,
                            ),
                        ]),
                    }),
                    envs: merge_resolved_envs_with_version(envs, resolved.envs),
                })
            }
            SynthesizableSubcommand::Pack { args } => {
                let cli_options = self
                    .cli_options
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("CLI options required for pack command"))?;
                let resolved = (cli_options.pack)().await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("pack JS path is not valid UTF-8"))?;

                Ok(ResolvedSubcommand {
                    program: Arc::from(OsStr::new("node")),
                    args: iter::once(Str::from(js_path_str))
                        .chain(args.into_iter().map(Str::from))
                        .collect(),
                    cache_config: UserCacheConfig::with_config(EnabledCacheConfig {
                        env: None,
                        untracked_env: None,
                        input: Some(build_pack_cache_inputs()),
                    }),
                    envs: merge_resolved_envs(envs, resolved.envs),
                })
            }
            SynthesizableSubcommand::Dev { args } => {
                let cli_options = self
                    .cli_options
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("CLI options required for dev command"))?;
                let resolved = (cli_options.vite)().await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("vite JS path is not valid UTF-8"))?;

                Ok(ResolvedSubcommand {
                    program: Arc::from(OsStr::new("node")),
                    args: iter::once(Str::from(js_path_str))
                        .chain(iter::once(Str::from("dev")))
                        .chain(args.into_iter().map(Str::from))
                        .collect(),
                    cache_config: UserCacheConfig::disabled(),
                    envs: merge_resolved_envs_with_version(envs, resolved.envs),
                })
            }
            SynthesizableSubcommand::Preview { args } => {
                let cli_options = self
                    .cli_options
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("CLI options required for preview command"))?;
                let resolved = (cli_options.vite)().await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("vite JS path is not valid UTF-8"))?;

                Ok(ResolvedSubcommand {
                    program: Arc::from(OsStr::new("node")),
                    args: iter::once(Str::from(js_path_str))
                        .chain(iter::once(Str::from("preview")))
                        .chain(args.into_iter().map(Str::from))
                        .collect(),
                    cache_config: UserCacheConfig::disabled(),
                    envs: merge_resolved_envs_with_version(envs, resolved.envs),
                })
            }
            SynthesizableSubcommand::Doc { args } => {
                let cli_options = self
                    .cli_options
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("CLI options required for doc command"))?;
                let resolved = (cli_options.doc)().await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("doc JS path is not valid UTF-8"))?;

                Ok(ResolvedSubcommand {
                    program: Arc::from(OsStr::new("node")),
                    args: iter::once(Str::from(js_path_str))
                        .chain(args.into_iter().map(Str::from))
                        .collect(),
                    cache_config: UserCacheConfig::with_config(EnabledCacheConfig {
                        env: None,
                        untracked_env: None,
                        input: None,
                    }),
                    envs: merge_resolved_envs(envs, resolved.envs),
                })
            }
            SynthesizableSubcommand::Check { .. } => {
                anyhow::bail!(
                    "Check is a composite command and cannot be resolved to a single subcommand"
                );
            }
            SynthesizableSubcommand::Install { args } => {
                let package_manager =
                    vite_install::PackageManager::builder(cwd).build_with_default().await?;
                let resolve_command = package_manager.resolve_install_command(&args);

                let merged_envs = {
                    let mut env_map = FxHashMap::clone(envs);
                    for (k, v) in resolve_command.envs {
                        env_map.insert(Arc::from(OsStr::new(&k)), Arc::from(OsStr::new(&v)));
                    }
                    Arc::new(env_map)
                };

                Ok(ResolvedSubcommand {
                    program: Arc::<OsStr>::from(
                        OsStr::new(&resolve_command.bin_path).to_os_string(),
                    ),
                    args: resolve_command.args.into_iter().map(Str::from).collect(),
                    cache_config: UserCacheConfig::with_config(EnabledCacheConfig {
                        env: None,
                        untracked_env: None,
                        input: None,
                    }),
                    envs: merged_envs,
                })
            }
        }
    }
}

/// Merge resolved environment variables from JS resolver into existing envs.
/// Does not override existing entries.
/// Create a negative glob entry to exclude a pattern from cache fingerprinting.
fn exclude_glob(pattern: &str, base: InputBase) -> UserInputEntry {
    UserInputEntry::GlobWithBase(GlobWithBase { pattern: Str::from(pattern), base })
}

/// Common cache input entries for build/pack commands.
/// Excludes .vite-temp config files and dist output files that are both read and written.
/// TODO: The hardcoded `!dist/**` exclusion is a temporary workaround. It will be replaced
/// by a runner-aware approach that automatically excludes task output directories.
fn build_pack_cache_inputs() -> Vec<UserInputEntry> {
    vec![
        UserInputEntry::Auto(AutoInput { auto: true }),
        exclude_glob("!node_modules/.vite-temp/**", InputBase::Workspace),
        exclude_glob("!node_modules/.vite-temp/**", InputBase::Package),
        exclude_glob("!dist/**", InputBase::Package),
    ]
}

/// Cache input entries for the check command.
/// The vp check subprocess is a full vp CLI process (not resolved to a binary like
/// build/lint/fmt), so it accesses additional directories that must be excluded:
/// - `.vite-temp`: config compilation cache, read+written during vp CLI startup
/// - `.vite/task-cache`: task runner state files that change after each run
fn check_cache_inputs() -> Vec<UserInputEntry> {
    vec![
        UserInputEntry::Auto(AutoInput { auto: true }),
        exclude_glob("!node_modules/.vite-temp/**", InputBase::Workspace),
        exclude_glob("!node_modules/.vite-temp/**", InputBase::Package),
        exclude_glob("!node_modules/.vite/task-cache/**", InputBase::Workspace),
        exclude_glob("!node_modules/.vite/task-cache/**", InputBase::Package),
    ]
}

fn merge_resolved_envs(
    envs: &Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>>,
    resolved_envs: Vec<(String, String)>,
) -> Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>> {
    let mut envs = FxHashMap::clone(envs);
    for (k, v) in resolved_envs {
        envs.entry(Arc::from(OsStr::new(&k))).or_insert_with(|| Arc::from(OsStr::new(&v)));
    }
    Arc::new(envs)
}

/// Merge resolved envs and inject VP_VERSION for rolldown-vite branding.
fn merge_resolved_envs_with_version(
    envs: &Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>>,
    resolved_envs: Vec<(String, String)>,
) -> Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>> {
    let mut merged = merge_resolved_envs(envs, resolved_envs);
    let map = Arc::make_mut(&mut merged);
    map.entry(Arc::from(OsStr::new("VP_VERSION")))
        .or_insert_with(|| Arc::from(OsStr::new(env!("CARGO_PKG_VERSION"))));
    merged
}

/// CommandHandler implementation for vite-plus.
/// Handles `vp` commands in task scripts.
pub struct VitePlusCommandHandler {
    resolver: SubcommandResolver,
}

impl std::fmt::Debug for VitePlusCommandHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VitePlusCommandHandler").finish()
    }
}

impl VitePlusCommandHandler {
    pub fn new(resolver: SubcommandResolver) -> Self {
        Self { resolver }
    }
}

#[async_trait::async_trait(?Send)]
impl CommandHandler for VitePlusCommandHandler {
    async fn handle_command(
        &mut self,
        command: &mut ScriptCommand,
    ) -> anyhow::Result<HandledCommand> {
        // Intercept "vp" and "vpr" commands in task scripts so that `vp test`, `vp build`,
        // `vpr build`, etc. are synthesized in-session rather than spawning a new CLI process.
        let program = command.program.as_str();
        if program != "vp" && program != "vpr" {
            return Ok(HandledCommand::Verbatim);
        }
        // "vpr <args>" is shorthand for "vp run <args>", so prepend "run" for parsing.
        let is_vpr = program == "vpr";
        let cli_args = match CLIArgs::try_parse_from(
            iter::once("vp")
                .chain(is_vpr.then_some("run"))
                .chain(command.args.iter().map(Str::as_str)),
        ) {
            Ok(args) => args,
            Err(err) if err.kind() == ErrorKind::InvalidSubcommand => {
                return Ok(HandledCommand::Synthesized(
                    command.to_synthetic_plan_request(UserCacheConfig::disabled()),
                ));
            }
            Err(err) => return Err(err.into()),
        };
        match cli_args {
            CLIArgs::Synthesizable(SynthesizableSubcommand::Check { .. }) => {
                // Check is a composite command (fmt + lint) — run as a subprocess in task scripts
                Ok(HandledCommand::Synthesized(command.to_synthetic_plan_request(
                    UserCacheConfig::with_config(EnabledCacheConfig {
                        env: Some(Box::new([Str::from("OXLINT_TSGOLINT_PATH")])),
                        untracked_env: None,
                        input: Some(check_cache_inputs()),
                    }),
                )))
            }
            CLIArgs::Synthesizable(subcmd) => {
                let resolved =
                    self.resolver.resolve(subcmd, None, &command.envs, &command.cwd).await?;
                Ok(HandledCommand::Synthesized(resolved.into_synthetic_plan_request()))
            }
            CLIArgs::ViteTask(cmd) => Ok(HandledCommand::ViteTaskCommand(cmd)),
            CLIArgs::Exec(_) => {
                // exec in task scripts should run as a subprocess
                Ok(HandledCommand::Synthesized(
                    command.to_synthetic_plan_request(UserCacheConfig::disabled()),
                ))
            }
        }
    }
}

/// User config loader that resolves vite.config.ts via JavaScript callback
pub struct VitePlusConfigLoader {
    resolve_fn: ViteConfigResolverFn,
}

impl VitePlusConfigLoader {
    pub fn new(resolve_fn: ViteConfigResolverFn) -> Self {
        Self { resolve_fn }
    }
}

impl std::fmt::Debug for VitePlusConfigLoader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VitePlusConfigLoader").finish()
    }
}

#[async_trait::async_trait(?Send)]
impl UserConfigLoader for VitePlusConfigLoader {
    async fn load_user_config_file(
        &self,
        package_path: &AbsolutePath,
    ) -> anyhow::Result<Option<UserRunConfig>> {
        // Try static config extraction first (no JS runtime needed)
        let static_fields = vite_static_config::resolve_static_config(package_path);
        match static_fields.get("run") {
            Some(vite_static_config::FieldValue::Json(run_value)) => {
                tracing::debug!(
                    "Using statically extracted run config for {}",
                    package_path.as_path().display()
                );
                let run_config: UserRunConfig = serde_json::from_value(run_value)?;
                return Ok(Some(run_config));
            }
            Some(vite_static_config::FieldValue::NonStatic) => {
                // `run` field exists (or may exist via a spread) — fall back to NAPI
                tracing::debug!(
                    "run config is not statically analyzable for {}, falling back to NAPI",
                    package_path.as_path().display()
                );
            }
            None => {
                // Config was analyzed successfully and `run` field is definitively absent
                return Ok(None);
            }
        }

        // Fall back to NAPI-based config resolution
        let package_path_str = package_path
            .as_path()
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("package path is not valid UTF-8"))?;

        let config_json = (self.resolve_fn)(package_path_str.to_string()).await?;
        let resolved: ResolvedUniversalViteConfig = serde_json::from_str(&config_json)
            .inspect_err(|_| {
                tracing::error!("Failed to parse vite config: {config_json}");
            })?;

        let run_config = match resolved.run {
            Some(run) => serde_json::from_value(run)?,
            None => UserRunConfig::default(),
        };
        Ok(Some(run_config))
    }
}

/// Resolve a subcommand into a prepared `tokio::process::Command`.
async fn resolve_and_build_command(
    resolver: &SubcommandResolver,
    subcommand: SynthesizableSubcommand,
    resolved_vite_config: Option<&ResolvedUniversalViteConfig>,
    envs: &Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>>,
    cwd: &AbsolutePathBuf,
    cwd_arc: &Arc<AbsolutePath>,
) -> Result<tokio::process::Command, Error> {
    let resolved = resolver
        .resolve(subcommand, resolved_vite_config, envs, cwd_arc)
        .await
        .map_err(|e| Error::Anyhow(e))?;

    // Resolve the program path using `which` to handle Windows .cmd/.bat files (PATHEXT)
    let program_path = {
        let paths = resolved.envs.iter().find_map(|(k, v)| {
            let is_path = if cfg!(windows) {
                k.as_ref().eq_ignore_ascii_case("PATH")
            } else {
                k.as_ref() == "PATH"
            };
            if is_path { Some(v.as_ref().to_os_string()) } else { None }
        });
        vite_command::resolve_bin(
            resolved.program.as_ref().to_str().unwrap_or_default(),
            paths.as_deref(),
            cwd,
        )?
    };

    let mut cmd = vite_command::build_command(&program_path, cwd);
    cmd.args(resolved.args.iter().map(|s| s.as_str()))
        .env_clear()
        .envs(resolved.envs.iter().map(|(k, v)| (k.as_ref(), v.as_ref())));
    Ok(cmd)
}

/// Resolve a single subcommand and execute it, returning its exit status.
async fn resolve_and_execute(
    resolver: &SubcommandResolver,
    subcommand: SynthesizableSubcommand,
    resolved_vite_config: Option<&ResolvedUniversalViteConfig>,
    envs: &Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>>,
    cwd: &AbsolutePathBuf,
    cwd_arc: &Arc<AbsolutePath>,
) -> Result<ExitStatus, Error> {
    let mut cmd =
        resolve_and_build_command(resolver, subcommand, resolved_vite_config, envs, cwd, cwd_arc)
            .await?;
    let mut child = cmd.spawn().map_err(|e| Error::Anyhow(e.into()))?;
    let status = child.wait().await.map_err(|e| Error::Anyhow(e.into()))?;
    Ok(ExitStatus(status.code().unwrap_or(1) as u8))
}

/// Like `resolve_and_execute`, but captures stdout, applies a text filter,
/// and writes the result to real stdout. Stderr remains inherited.
async fn resolve_and_execute_with_stdout_filter(
    resolver: &SubcommandResolver,
    subcommand: SynthesizableSubcommand,
    resolved_vite_config: Option<&ResolvedUniversalViteConfig>,
    envs: &Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>>,
    cwd: &AbsolutePathBuf,
    cwd_arc: &Arc<AbsolutePath>,
    filter: impl Fn(&str) -> Cow<'_, str>,
) -> Result<ExitStatus, Error> {
    let mut cmd =
        resolve_and_build_command(resolver, subcommand, resolved_vite_config, envs, cwd, cwd_arc)
            .await?;
    cmd.stdout(Stdio::piped());

    let child = cmd.spawn().map_err(|e| Error::Anyhow(e.into()))?;
    let output = child.wait_with_output().await.map_err(|e| Error::Anyhow(e.into()))?;

    use std::io::Write;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let filtered = filter(&stdout);
    let _ = std::io::stdout().lock().write_all(filtered.as_bytes());

    Ok(ExitStatus(output.status.code().unwrap_or(1) as u8))
}

/// Like `resolve_and_execute`, but captures stderr, applies a text filter,
/// and writes the result to real stderr. Stdout remains inherited (streaming).
async fn resolve_and_execute_with_stderr_filter(
    resolver: &SubcommandResolver,
    subcommand: SynthesizableSubcommand,
    resolved_vite_config: Option<&ResolvedUniversalViteConfig>,
    envs: &Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>>,
    cwd: &AbsolutePathBuf,
    cwd_arc: &Arc<AbsolutePath>,
    filter: impl Fn(&str) -> Cow<'_, str>,
) -> Result<ExitStatus, Error> {
    let mut cmd =
        resolve_and_build_command(resolver, subcommand, resolved_vite_config, envs, cwd, cwd_arc)
            .await?;
    cmd.stderr(Stdio::piped());

    let child = cmd.spawn().map_err(|e| Error::Anyhow(e.into()))?;
    let output = child.wait_with_output().await.map_err(|e| Error::Anyhow(e.into()))?;

    use std::io::Write;
    let stderr = String::from_utf8_lossy(&output.stderr);
    let filtered = filter(&stderr);
    let _ = std::io::stderr().lock().write_all(filtered.as_bytes());

    Ok(ExitStatus(output.status.code().unwrap_or(1) as u8))
}

pub(crate) struct CapturedCommandOutput {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

pub(crate) async fn resolve_and_capture_output(
    resolver: &SubcommandResolver,
    subcommand: SynthesizableSubcommand,
    resolved_vite_config: Option<&ResolvedUniversalViteConfig>,
    envs: &Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>>,
    cwd: &AbsolutePathBuf,
    cwd_arc: &Arc<AbsolutePath>,
    force_color_if_terminal: bool,
) -> Result<CapturedCommandOutput, Error> {
    let mut cmd =
        resolve_and_build_command(resolver, subcommand, resolved_vite_config, envs, cwd, cwd_arc)
            .await?;
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    if force_color_if_terminal && std::io::stdout().is_terminal() {
        cmd.env("FORCE_COLOR", "1");
    }

    let child = cmd.spawn().map_err(|e| Error::Anyhow(e.into()))?;
    let output = child.wait_with_output().await.map_err(|e| Error::Anyhow(e.into()))?;

    Ok(CapturedCommandOutput {
        status: ExitStatus(output.status.code().unwrap_or(1) as u8),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

/// Execute a synthesizable subcommand directly (not through vite-task Session).
/// No caching, no task graph, no dependency resolution.
async fn execute_direct_subcommand(
    subcommand: SynthesizableSubcommand,
    cwd: &AbsolutePathBuf,
    options: Option<CliOptions>,
) -> Result<ExitStatus, Error> {
    let (workspace_root, _) = vite_workspace::find_workspace_root(cwd)?;
    let workspace_path: Arc<AbsolutePath> = workspace_root.path.into();

    let resolver = if let Some(options) = options {
        SubcommandResolver::new(Arc::clone(&workspace_path)).with_cli_options(options)
    } else {
        SubcommandResolver::new(Arc::clone(&workspace_path))
    };

    let envs: Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>> = Arc::new(
        std::env::vars_os()
            .map(|(k, v)| (Arc::from(k.as_os_str()), Arc::from(v.as_os_str())))
            .collect(),
    );
    let cwd_arc: Arc<AbsolutePath> = cwd.clone().into();

    let status = match subcommand {
        SynthesizableSubcommand::Check { fix, no_fmt, no_lint, paths } => {
            return crate::check::execute_check(
                &resolver, fix, no_fmt, no_lint, paths, &envs, cwd, &cwd_arc,
            )
            .await;
        }
        other => {
            if should_suppress_subcommand_stdout(&other) {
                resolve_and_execute_with_stdout_filter(
                    &resolver,
                    other,
                    None,
                    &envs,
                    cwd,
                    &cwd_arc,
                    |_| Cow::Borrowed(""),
                )
                .await?
            } else if matches!(&other, SynthesizableSubcommand::Fmt { .. }) {
                resolve_and_execute_with_stderr_filter(
                    &resolver,
                    other,
                    None,
                    &envs,
                    cwd,
                    &cwd_arc,
                    |s| s.cow_replace("oxfmt --init", "vp fmt --init"),
                )
                .await?
            } else {
                resolve_and_execute(&resolver, other, None, &envs, cwd, &cwd_arc).await?
            }
        }
    };

    Ok(status)
}

/// Execute a vite-task command (run, cache) through Session.
async fn execute_vite_task_command(
    command: Command,
    cwd: AbsolutePathBuf,
    options: Option<CliOptions>,
) -> Result<ExitStatus, Error> {
    let (workspace_root, _) = vite_workspace::find_workspace_root(&cwd)?;
    let workspace_path: Arc<AbsolutePath> = workspace_root.path.into();

    let resolve_vite_config_fn = options
        .as_ref()
        .map(|o| Arc::clone(&o.resolve_universal_vite_config))
        .ok_or_else(|| {
            Error::Anyhow(anyhow::anyhow!(
                "resolve_universal_vite_config is required but not available"
            ))
        })?;

    let resolver = if let Some(options) = options {
        SubcommandResolver::new(Arc::clone(&workspace_path)).with_cli_options(options)
    } else {
        SubcommandResolver::new(Arc::clone(&workspace_path))
    };

    let mut command_handler = VitePlusCommandHandler::new(resolver);
    let mut config_loader = VitePlusConfigLoader::new(resolve_vite_config_fn);

    // Update PATH to include package manager bin directory BEFORE session init
    if let Ok(pm) = vite_install::PackageManager::builder(&cwd).build().await {
        let bin_prefix = pm.get_bin_prefix();
        prepend_to_path_env(&bin_prefix, PrependOptions::default());
    }

    let session = Session::init(SessionConfig {
        command_handler: &mut command_handler,
        user_config_loader: &mut config_loader,
        program_name: Str::from("vp"),
    })?;

    // Main execution (consumes session)
    let result = session.main(command).await.map_err(|e| Error::Anyhow(e));

    result
}

/// Main entry point for vite-plus CLI.
///
/// # Arguments
/// * `cwd` - Current working directory
/// * `options` - Optional CLI options with resolver functions
/// * `args` - Optional CLI arguments. If None, uses env::args(). This allows NAPI bindings
///            to pass process.argv.slice(2) to avoid including node binary and script path.
#[tracing::instrument(skip(options))]
pub async fn main(
    cwd: AbsolutePathBuf,
    options: Option<CliOptions>,
    args: Option<Vec<String>>,
) -> Result<ExitStatus, Error> {
    let args_vec: Vec<String> = args.unwrap_or_else(|| env::args().skip(1).collect());
    let args_vec = normalize_help_args(args_vec);
    if should_print_help(&args_vec) {
        print_help();
        return Ok(ExitStatus::SUCCESS);
    }

    let args_with_program = std::iter::once("vp".to_string()).chain(args_vec.iter().cloned());
    let cli_args = match CLIArgs::try_parse_from(args_with_program) {
        Ok(args) => args,
        Err(err) => return handle_cli_parse_error(err),
    };

    match cli_args {
        CLIArgs::Synthesizable(subcmd) => execute_direct_subcommand(subcmd, &cwd, options).await,
        CLIArgs::ViteTask(command) => execute_vite_task_command(command, cwd, options).await,
        CLIArgs::Exec(exec_args) => crate::exec::execute(exec_args, &cwd).await,
    }
}

fn handle_cli_parse_error(err: clap::Error) -> Result<ExitStatus, Error> {
    if matches!(err.kind(), ErrorKind::InvalidSubcommand) && print_invalid_subcommand_error(&err) {
        return Ok(ExitStatus(err.exit_code() as u8));
    }
    if matches!(err.kind(), ErrorKind::UnknownArgument) && print_unknown_argument_error(&err) {
        return Ok(ExitStatus(err.exit_code() as u8));
    }

    err.print().map_err(|e| Error::Anyhow(e.into()))?;
    Ok(ExitStatus(err.exit_code() as u8))
}

fn normalize_help_args(args: Vec<String>) -> Vec<String> {
    match args.as_slice() {
        [arg] if arg == "help" => vec!["--help".to_string()],
        [first, command, rest @ ..] if first == "help" => {
            let mut normalized = Vec::with_capacity(rest.len() + 2);
            normalized.push(command.to_string());
            normalized.push("--help".to_string());
            normalized.extend(rest.iter().cloned());
            normalized
        }
        _ => args,
    }
}

fn is_vitest_help_flag(arg: &str) -> bool {
    matches!(arg, "-h" | "--help")
}

fn is_vitest_watch_flag(arg: &str) -> bool {
    matches!(arg, "-w" | "--watch")
}

fn is_vitest_test_subcommand(arg: &str) -> bool {
    matches!(arg, "run" | "watch" | "dev" | "related" | "bench" | "init" | "list")
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

fn should_suppress_subcommand_stdout(subcommand: &SynthesizableSubcommand) -> bool {
    match subcommand {
        SynthesizableSubcommand::Lint { args } => has_flag_before_terminator(args, "--init"),
        SynthesizableSubcommand::Fmt { args } => {
            has_flag_before_terminator(args, "--init")
                || has_flag_before_terminator(args, "--migrate")
        }
        _ => false,
    }
}

fn should_prepend_vitest_run(args: &[String]) -> bool {
    let Some(first_arg) = args.first().map(String::as_str) else {
        return true;
    };

    if is_vitest_test_subcommand(first_arg) {
        return false;
    }

    for arg in args.iter().take_while(|arg| arg.as_str() != "--") {
        let arg = arg.as_str();
        if is_vitest_help_flag(arg) || is_vitest_watch_flag(arg) || arg == "--run" {
            return false;
        }
    }

    true
}

fn should_print_help(args: &[String]) -> bool {
    args.is_empty() || matches!(args, [arg] if arg == "-h" || arg == "--help")
}

fn extract_invalid_subcommand_details(error: &clap::Error) -> Option<(String, Option<String>)> {
    let invalid_subcommand = match error.get(ContextKind::InvalidSubcommand) {
        Some(ContextValue::String(value)) => value.as_str(),
        _ => return None,
    };

    let suggestion = match error.get(ContextKind::SuggestedSubcommand) {
        Some(ContextValue::String(value)) => Some(value.to_owned()),
        Some(ContextValue::Strings(values)) => {
            vite_shared::string_similarity::pick_best_suggestion(invalid_subcommand, values)
        }
        _ => None,
    };

    Some((invalid_subcommand.to_owned(), suggestion))
}

fn print_invalid_subcommand_error(error: &clap::Error) -> bool {
    let Some((invalid_subcommand, suggestion)) = extract_invalid_subcommand_details(error) else {
        return false;
    };

    let highlighted_subcommand = invalid_subcommand.bright_blue().to_string();
    output::error(&format!("Command '{highlighted_subcommand}' not found"));

    if let Some(suggestion) = suggestion {
        eprintln!();
        let highlighted_suggestion = format!("`vp {suggestion}`").bright_blue().to_string();
        eprintln!("Did you mean {highlighted_suggestion}?");
    }

    true
}

fn extract_unknown_argument(error: &clap::Error) -> Option<String> {
    match error.get(ContextKind::InvalidArg) {
        Some(ContextValue::String(value)) => Some(value.to_owned()),
        _ => None,
    }
}

fn has_pass_as_value_suggestion(error: &clap::Error) -> bool {
    let contains_pass_as_value = |suggestion: &str| suggestion.contains("as a value");

    match error.get(ContextKind::Suggested) {
        Some(ContextValue::String(suggestion)) => contains_pass_as_value(suggestion),
        Some(ContextValue::Strings(suggestions)) => {
            suggestions.iter().any(|suggestion| contains_pass_as_value(suggestion))
        }
        Some(ContextValue::StyledStr(suggestion)) => {
            contains_pass_as_value(&suggestion.to_string())
        }
        Some(ContextValue::StyledStrs(suggestions)) => {
            suggestions.iter().any(|suggestion| contains_pass_as_value(&suggestion.to_string()))
        }
        _ => false,
    }
}

fn print_unknown_argument_error(error: &clap::Error) -> bool {
    let Some(invalid_argument) = extract_unknown_argument(error) else {
        return false;
    };

    let highlighted_argument = invalid_argument.bright_blue().to_string();
    output::error(&format!("Unexpected argument '{highlighted_argument}'"));

    if has_pass_as_value_suggestion(error) {
        eprintln!();
        let pass_through_argument = format!("-- {invalid_argument}");
        let highlighted_pass_through_argument =
            format!("`{}`", pass_through_argument.bright_blue());
        eprintln!("Use {highlighted_pass_through_argument} to pass the argument as a value");
    }

    true
}

fn print_help() {
    let header = vite_shared::header::vite_plus_header();
    let bold = "\x1b[1m";
    let bold_underline = "\x1b[1;4m";
    let reset = "\x1b[0m";
    println!(
        "{header}

{bold_underline}Usage:{reset} {bold}vp{reset} <COMMAND>

{bold_underline}Core Commands:{reset}
  {bold}dev{reset}            Run the development server
  {bold}build{reset}          Build for production
  {bold}test{reset}           Run tests
  {bold}lint{reset}           Lint code
  {bold}fmt{reset}            Format code
  {bold}check{reset}          Run format, lint, and type checks
  {bold}pack{reset}           Build library
  {bold}run{reset}            Run tasks
  {bold}exec{reset}           Execute a command from local node_modules/.bin
  {bold}preview{reset}        Preview production build
  {bold}cache{reset}          Manage the task cache
  {bold}config{reset}         Configure hooks and agent integration
  {bold}staged{reset}         Run linters on staged files

{bold_underline}Package Manager Commands:{reset}
  {bold}install{reset}    Install all dependencies, or add packages if package names are provided

Options:
  -h, --help  Print help"
    );
}

pub use vite_shared::init_tracing;

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use clap::Parser;
    use vite_task::{Command, config::UserRunConfig};

    use super::{
        CLIArgs, SynthesizableSubcommand, extract_unknown_argument, has_pass_as_value_suggestion,
        should_prepend_vitest_run, should_suppress_subcommand_stdout,
    };

    #[test]
    fn run_config_types_in_sync() {
        // Remove \r for cross-platform consistency
        let ts_type = UserRunConfig::TS_TYPE.replace('\r', "");
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
        let run_config_path = PathBuf::from(manifest_dir).join("../src/run-config.ts");

        if std::env::var("VITE_UPDATE_TASK_TYPES").as_deref() == Ok("1") {
            std::fs::write(&run_config_path, &ts_type).expect("Failed to write run-config.ts");
        } else {
            let current = std::fs::read_to_string(&run_config_path)
                .expect("Failed to read run-config.ts")
                .replace('\r', "");
            pretty_assertions::assert_eq!(
                current,
                ts_type,
                "run-config.ts is out of sync. Run `VITE_UPDATE_TASK_TYPES=1 cargo test -p vite-plus-cli run_config_types_in_sync` to update."
            );
        }
    }

    #[test]
    fn unknown_argument_detected_without_pass_as_value_hint() {
        let error = CLIArgs::try_parse_from(["vp", "--cache"]).expect_err("Expected parse error");
        assert_eq!(extract_unknown_argument(&error).as_deref(), Some("--cache"));
        assert!(!has_pass_as_value_suggestion(&error));
    }

    #[test]
    fn run_accepts_unknown_flags_as_task_args() {
        // After trailing_var_arg change, unknown flags like --yolo are
        // accepted as task arguments instead of producing a parse error.
        let args = CLIArgs::try_parse_from(["vp", "run", "--yolo"]).unwrap();
        let debug = vite_str::format!("{args:?}");
        assert!(debug.contains("\"--yolo\""), "Expected --yolo in task args, got: {debug}",);
        assert!(matches!(args, CLIArgs::ViteTask(Command::Run(_))));
    }

    #[test]
    fn test_without_args_defaults_to_run_mode() {
        assert!(should_prepend_vitest_run(&[]));
    }

    #[test]
    fn test_with_filters_defaults_to_run_mode() {
        assert!(should_prepend_vitest_run(&["src/foo.test.ts".to_string()]));
    }

    #[test]
    fn test_with_options_defaults_to_run_mode() {
        assert!(should_prepend_vitest_run(&["--coverage".to_string()]));
    }

    #[test]
    fn test_with_run_subcommand_does_not_prepend_run() {
        assert!(!should_prepend_vitest_run(&["run".to_string(), "--coverage".to_string()]));
    }

    #[test]
    fn test_with_watch_subcommand_does_not_prepend_run() {
        assert!(!should_prepend_vitest_run(&["watch".to_string()]));
    }

    #[test]
    fn test_with_watch_flag_does_not_prepend_run() {
        assert!(!should_prepend_vitest_run(&["--watch".to_string()]));
        assert!(!should_prepend_vitest_run(&["-w".to_string()]));
    }

    #[test]
    fn test_with_help_flag_does_not_prepend_run() {
        assert!(!should_prepend_vitest_run(&["--help".to_string()]));
        assert!(!should_prepend_vitest_run(&["-h".to_string()]));
    }

    #[test]
    fn test_with_explicit_run_flag_does_not_prepend_run() {
        assert!(!should_prepend_vitest_run(&["--run".to_string(), "--coverage".to_string()]));
    }

    #[test]
    fn test_ignores_flags_after_option_terminator() {
        assert!(should_prepend_vitest_run(&[
            "--".to_string(),
            "--watch".to_string(),
            "src/foo.test.ts".to_string(),
        ]));
    }

    #[test]
    fn lint_init_suppresses_stdout() {
        let subcommand = SynthesizableSubcommand::Lint { args: vec!["--init".to_string()] };
        assert!(should_suppress_subcommand_stdout(&subcommand));
    }

    #[test]
    fn fmt_migrate_suppresses_stdout() {
        let subcommand =
            SynthesizableSubcommand::Fmt { args: vec!["--migrate=prettier".to_string()] };
        assert!(should_suppress_subcommand_stdout(&subcommand));
    }

    #[test]
    fn normal_lint_does_not_suppress_stdout() {
        let subcommand = SynthesizableSubcommand::Lint { args: vec!["src/index.ts".to_string()] };
        assert!(!should_suppress_subcommand_stdout(&subcommand));
    }

    #[test]
    fn global_subcommands_produce_invalid_subcommand_error() {
        use clap::error::ErrorKind;

        for subcommand in ["config", "create", "env", "migrate"] {
            let error = CLIArgs::try_parse_from(["vp", subcommand])
                .expect_err(&format!("expected error for global subcommand '{subcommand}'"));
            assert_eq!(
                error.kind(),
                ErrorKind::InvalidSubcommand,
                "expected InvalidSubcommand for '{subcommand}', got {:?}",
                error.kind()
            );
        }
    }
}
