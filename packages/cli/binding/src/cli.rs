//! CLI types and logic for vite-plus using the new Session API from vite-task.
//!
//! This module contains all the CLI-related code.
//! It handles argument parsing, command dispatching, and orchestration of the task execution.

use std::{
    collections::HashMap, env, ffi::OsStr, future::Future, iter, pin::Pin, process::ExitStatus,
    sync::Arc,
};

use clap::Subcommand;
use monostate::MustBe;
use serde::{Deserialize, Serialize};
use tokio::fs::write;
use vite_error::Error;
use vite_path::{AbsolutePath, AbsolutePathBuf};
use vite_str::Str;
use vite_task::{
    CLIArgs, EnabledCacheConfig, LabeledReporter, Session, SessionCallbacks, TaskSynthesizer,
    UserCacheConfig, UserTaskOptions, plan_request::SyntheticPlanRequest,
};

/// Resolved configuration from vite.config.ts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedUniversalViteConfig {
    pub lint: Option<serde_json::Value>,
    pub fmt: Option<serde_json::Value>,
}

/// Result type for resolved commands from JavaScript
#[derive(Debug, Clone)]
pub struct ResolveCommandResult {
    pub bin_path: Arc<OsStr>,
    pub envs: Vec<(String, String)>,
}

/// These are the custom subcommands that synthesize tasks for vite-plus
/// NOTE: Run command is already provided by vite-task, no need to declare here
#[derive(Debug, Clone, Subcommand)]
pub enum CustomTaskSubcommand {
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
    /// Build application
    #[command(disable_help_flag = true)]
    Build {
        #[clap(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Run test
    #[command(disable_help_flag = true)]
    Test {
        #[clap(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Build library
    #[command(disable_help_flag = true)]
    Lib {
        #[clap(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Run development server
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
    #[command(disable_help_flag = true)]
    Doc {
        #[clap(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Install command.
    /// It will be passed to the package manager's install command currently.
    #[command(disable_help_flag = true, alias = "i")]
    Install {
        #[clap(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
}

/// Non-task subcommands (handled independently without Session)
#[derive(Debug, Clone, Subcommand)]
pub enum NonTaskSubcommand {
    /// Manage the task cache
    Cache {
        #[clap(subcommand)]
        subcmd: CacheSubcommand,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum CacheSubcommand {
    /// Clean up all the cache
    Clean,
    /// View the cache entries in json for debugging purpose
    View,
}

/// Type alias for boxed async resolver function
/// NOTE: Uses anyhow::Error to avoid NAPI type inference issues
pub type BoxedResolverFn =
    Box<dyn Fn() -> Pin<Box<dyn Future<Output = anyhow::Result<ResolveCommandResult>> + 'static>>>;

/// Type alias for boxed vite config resolver function (takes workspace root path, returns JSON string)
pub type BoxedViteConfigResolverFn =
    Box<dyn Fn(String) -> Pin<Box<dyn Future<Output = anyhow::Result<String>> + 'static>>>;

/// CLI options containing JavaScript resolver functions (using boxed futures for simplicity)
pub struct CliOptions {
    pub lint: BoxedResolverFn,
    pub fmt: BoxedResolverFn,
    pub vite: BoxedResolverFn,
    pub test: BoxedResolverFn,
    pub lib: BoxedResolverFn,
    pub doc: BoxedResolverFn,
    pub resolve_universal_vite_config: BoxedViteConfigResolverFn,
}

/// Task synthesizer for vite-plus that uses JavaScript resolver functions
pub struct VitePlusTaskSynthesizer {
    cli_options: Option<CliOptions>,
    workspace_path: Arc<AbsolutePath>,
}

impl std::fmt::Debug for VitePlusTaskSynthesizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VitePlusTaskSynthesizer")
            .field("has_cli_options", &self.cli_options.is_some())
            .field("workspace_path", &self.workspace_path)
            .finish()
    }
}

impl VitePlusTaskSynthesizer {
    pub fn new(workspace_path: Arc<AbsolutePath>) -> Self {
        Self { cli_options: None, workspace_path }
    }

    pub fn with_cli_options(mut self, cli_options: CliOptions) -> Self {
        self.cli_options = Some(cli_options);
        self
    }
}

#[async_trait::async_trait(?Send)]
impl TaskSynthesizer<CustomTaskSubcommand> for VitePlusTaskSynthesizer {
    fn should_synthesize_for_program(&self, program: &str) -> bool {
        program == "vite"
    }

    async fn synthesize_task(
        &mut self,
        subcommand: CustomTaskSubcommand,
        envs: &Arc<HashMap<Arc<OsStr>, Arc<OsStr>>>,
        cwd: &Arc<AbsolutePath>,
    ) -> anyhow::Result<SyntheticPlanRequest> {
        match subcommand {
            CustomTaskSubcommand::Lint { mut args } => {
                let cli_options = self
                    .cli_options
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("CLI options required for lint command"))?;
                let resolved = (cli_options.lint)().await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("lint JS path is not valid UTF-8"))?;

                // Resolve vite config and extract lint config
                let workspace_path_str = self
                    .workspace_path
                    .as_path()
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("workspace path is not valid UTF-8"))?;
                let vite_config_json =
                    (cli_options.resolve_universal_vite_config)(workspace_path_str.to_string())
                        .await?;
                let resolved_vite_config: ResolvedUniversalViteConfig =
                    serde_json::from_str(&vite_config_json).inspect_err(|_| {
                        tracing::error!("Failed to parse vite config: {vite_config_json}");
                    })?;

                // If lint config exists, write to tmp-config and add -c arg
                if let Some(lint_config) = resolved_vite_config.lint {
                    let config_dir = self.workspace_path.join("node_modules/.vite/tmp-config");
                    tokio::fs::create_dir_all(&config_dir).await?;
                    let oxlint_config_path = config_dir.join(".oxlintrc.json");
                    write(&oxlint_config_path, serde_json::to_string(&lint_config)?).await?;
                    let oxlint_config_path_str = oxlint_config_path
                        .as_path()
                        .to_str()
                        .ok_or_else(|| anyhow::anyhow!("oxlint config path is not valid UTF-8"))?;
                    args.insert(0, oxlint_config_path_str.to_string());
                    args.insert(0, "-c".to_string());
                }

                let direct_execution_cache_key: Arc<[Str]> = iter::once(Str::from("lint"))
                    .chain(args.iter().map(|s| Str::from(s.as_str())))
                    .collect();

                // Clone envs and extend with resolved.envs (without overriding existing entries)
                let mut envs = HashMap::clone(envs);
                for (k, v) in resolved.envs {
                    envs.entry(Arc::from(OsStr::new(&k)))
                        .or_insert_with(|| Arc::from(OsStr::new(&v)));
                }

                Ok(SyntheticPlanRequest {
                    program: Arc::from(OsStr::new("node")),
                    args: iter::once(Str::from(js_path_str))
                        .chain(args.into_iter().map(Str::from))
                        .collect(),
                    task_options: UserTaskOptions {
                        cache_config: UserCacheConfig::Enabled {
                            cache: MustBe!(true),
                            enabled_cache_config: EnabledCacheConfig {
                                // Fingerprint OXLINT_TSGOLINT_PATH for type-aware linting cache invalidation
                                envs: Box::new([Str::from("OXLINT_TSGOLINT_PATH")]),
                                pass_through_envs: vec![],
                            },
                        },
                        ..Default::default()
                    },
                    direct_execution_cache_key,
                    envs: Arc::new(envs),
                })
            }
            CustomTaskSubcommand::Fmt { mut args } => {
                let cli_options = self
                    .cli_options
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("CLI options required for fmt command"))?;
                let resolved = (cli_options.fmt)().await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("fmt JS path is not valid UTF-8"))?;

                // Resolve vite config and extract fmt config
                let workspace_path_str = self
                    .workspace_path
                    .as_path()
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("workspace path is not valid UTF-8"))?;
                let vite_config_json =
                    (cli_options.resolve_universal_vite_config)(workspace_path_str.to_string())
                        .await?;
                let resolved_vite_config: ResolvedUniversalViteConfig =
                    serde_json::from_str(&vite_config_json).inspect_err(|_| {
                        tracing::error!("Failed to parse vite config: {vite_config_json}");
                    })?;

                // If fmt config exists, write to tmp-config and add -c arg
                if let Some(fmt_config) = resolved_vite_config.fmt {
                    let config_dir = self.workspace_path.join("node_modules/.vite/tmp-config");
                    tokio::fs::create_dir_all(&config_dir).await?;
                    let oxfmt_config_path = config_dir.join(".oxfmtrc.json");
                    write(&oxfmt_config_path, serde_json::to_string(&fmt_config)?).await?;
                    let oxfmt_config_path_str = oxfmt_config_path
                        .as_path()
                        .to_str()
                        .ok_or_else(|| anyhow::anyhow!("oxfmt config path is not valid UTF-8"))?;
                    args.insert(0, oxfmt_config_path_str.to_string());
                    args.insert(0, "-c".to_string());
                }

                let direct_execution_cache_key: Arc<[Str]> = iter::once(Str::from("fmt"))
                    .chain(args.iter().map(|s| Str::from(s.as_str())))
                    .collect();

                Ok(SyntheticPlanRequest {
                    program: Arc::from(OsStr::new("node")),
                    args: iter::once(Str::from(js_path_str))
                        .chain(args.into_iter().map(Str::from))
                        .collect(),
                    task_options: Default::default(),
                    direct_execution_cache_key,
                    envs: Arc::clone(envs),
                })
            }
            CustomTaskSubcommand::Build { args } => {
                let cli_options = self
                    .cli_options
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("CLI options required for build command"))?;
                let resolved = (cli_options.vite)().await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("vite JS path is not valid UTF-8"))?;

                let direct_execution_cache_key: Arc<[Str]> = iter::once(Str::from("build"))
                    .chain(args.iter().map(|s| Str::from(s.as_str())))
                    .collect();

                Ok(SyntheticPlanRequest {
                    program: Arc::from(OsStr::new("node")),
                    args: iter::once(Str::from(js_path_str))
                        .chain(iter::once(Str::from("build")))
                        .chain(args.into_iter().map(Str::from))
                        .collect(),
                    task_options: Default::default(),
                    direct_execution_cache_key,
                    envs: Arc::clone(envs),
                })
            }
            CustomTaskSubcommand::Test { args } => {
                let cli_options = self
                    .cli_options
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("CLI options required for test command"))?;
                let resolved = (cli_options.test)().await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("test JS path is not valid UTF-8"))?;

                let direct_execution_cache_key: Arc<[Str]> = iter::once(Str::from("test"))
                    .chain(args.iter().map(|s| Str::from(s.as_str())))
                    .collect();

                Ok(SyntheticPlanRequest {
                    program: Arc::from(OsStr::new("node")),
                    args: iter::once(Str::from(js_path_str))
                        .chain(args.into_iter().map(Str::from))
                        .collect(),
                    task_options: Default::default(),
                    direct_execution_cache_key,
                    envs: Arc::clone(envs),
                })
            }
            CustomTaskSubcommand::Lib { args } => {
                let cli_options = self
                    .cli_options
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("CLI options required for lib command"))?;
                let resolved = (cli_options.lib)().await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("lib JS path is not valid UTF-8"))?;

                let direct_execution_cache_key: Arc<[Str]> = iter::once(Str::from("lib"))
                    .chain(args.iter().map(|s| Str::from(s.as_str())))
                    .collect();

                Ok(SyntheticPlanRequest {
                    program: Arc::from(OsStr::new("node")),
                    args: iter::once(Str::from(js_path_str))
                        .chain(args.into_iter().map(Str::from))
                        .collect(),
                    task_options: Default::default(),
                    direct_execution_cache_key,
                    envs: Arc::clone(envs),
                })
            }
            CustomTaskSubcommand::Dev { args } => {
                let cli_options = self
                    .cli_options
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("CLI options required for dev command"))?;
                let resolved = (cli_options.vite)().await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("vite JS path is not valid UTF-8"))?;

                let direct_execution_cache_key: Arc<[Str]> = iter::once(Str::from("dev"))
                    .chain(args.iter().map(|s| Str::from(s.as_str())))
                    .collect();

                Ok(SyntheticPlanRequest {
                    program: Arc::from(OsStr::new("node")),
                    args: iter::once(Str::from(js_path_str))
                        .chain(iter::once(Str::from("dev")))
                        .chain(args.into_iter().map(Str::from))
                        .collect(),
                    task_options: Default::default(),
                    direct_execution_cache_key,
                    envs: Arc::clone(envs),
                })
            }
            CustomTaskSubcommand::Preview { args } => {
                let cli_options = self
                    .cli_options
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("CLI options required for preview command"))?;
                let resolved = (cli_options.vite)().await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("vite JS path is not valid UTF-8"))?;

                let direct_execution_cache_key: Arc<[Str]> = iter::once(Str::from("preview"))
                    .chain(args.iter().map(|s| Str::from(s.as_str())))
                    .collect();

                Ok(SyntheticPlanRequest {
                    program: Arc::from(OsStr::new("node")),
                    args: iter::once(Str::from(js_path_str))
                        .chain(iter::once(Str::from("preview")))
                        .chain(args.into_iter().map(Str::from))
                        .collect(),
                    task_options: Default::default(),
                    direct_execution_cache_key,
                    envs: Arc::clone(envs),
                })
            }
            CustomTaskSubcommand::Doc { args } => {
                let cli_options = self
                    .cli_options
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("CLI options required for doc command"))?;
                let resolved = (cli_options.doc)().await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("doc JS path is not valid UTF-8"))?;

                let direct_execution_cache_key: Arc<[Str]> = iter::once(Str::from("doc"))
                    .chain(args.iter().map(|s| Str::from(s.as_str())))
                    .collect();

                Ok(SyntheticPlanRequest {
                    program: Arc::from(OsStr::new("node")),
                    args: iter::once(Str::from(js_path_str))
                        .chain(args.into_iter().map(Str::from))
                        .collect(),
                    task_options: Default::default(),
                    direct_execution_cache_key,
                    envs: Arc::clone(envs),
                })
            }
            CustomTaskSubcommand::Install { args } => {
                // Install command uses the package manager
                let package_manager =
                    vite_install::PackageManager::builder(cwd).build_with_default().await?;
                let resolve_command = package_manager.resolve_install_command(&args);

                let direct_execution_cache_key: Arc<[Str]> = iter::once(Str::from("install"))
                    .chain(args.iter().map(|s| Str::from(s.as_str())))
                    .collect();

                Ok(SyntheticPlanRequest {
                    program: Arc::<OsStr>::from(
                        OsStr::new(&resolve_command.bin_path).to_os_string(),
                    ),
                    args: resolve_command.args.into_iter().map(Str::from).collect(),
                    task_options: Default::default(),
                    direct_execution_cache_key,
                    envs: Arc::clone(envs),
                })
            }
        }
    }
}

/// Create auto-install synthetic plan request
async fn create_install_synthetic_request(
    cwd: &AbsolutePathBuf,
) -> Result<SyntheticPlanRequest, Error> {
    let package_manager = vite_install::PackageManager::builder(cwd).build_with_default().await?;
    let resolve_command = package_manager.resolve_install_command(&vec![]);

    Ok(SyntheticPlanRequest {
        program: Arc::<OsStr>::from(OsStr::new(&resolve_command.bin_path).to_os_string()),
        args: resolve_command.args.into_iter().map(Str::from).collect(),
        task_options: Default::default(),
        direct_execution_cache_key: vec![Str::from("install")].into(),
        envs: Arc::new(
            std::env::vars_os()
                .map(|(k, v)| (Arc::from(k.as_os_str()), Arc::from(v.as_os_str())))
                .collect(),
        ),
    })
}

/// Check if a command is a package manager command that should skip auto-install
/// We check command line args directly since TaskCLIArgs internals are private
fn is_package_manager_command(args: &[String]) -> bool {
    // Check if "install" or "i" is in the command line args
    args.iter().any(|arg| arg == "install" || arg == "i")
}

/// Handle cache subcommand
async fn handle_cache_command(
    cwd: AbsolutePathBuf,
    subcmd: CacheSubcommand,
) -> Result<ExitStatus, Error> {
    // Get cache path - need to find workspace root first
    let (workspace_root, _) = vite_workspace::find_workspace_root(&cwd)?;
    let cache_path = workspace_root.path.join(".vite-plus");

    match subcmd {
        CacheSubcommand::Clean => {
            if cache_path.as_path().exists() {
                std::fs::remove_dir_all(&cache_path)?;
            }
        }
        CacheSubcommand::View => {
            // TODO: Implement cache view with new API
            eprintln!("Cache view not yet implemented with new Session API");
        }
    }
    Ok(ExitStatus::default())
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
    // Get args from parameter or env::args()
    // When running from NAPI, args should be passed explicitly to skip node/script paths
    let args_vec: Vec<String> = args.unwrap_or_else(|| env::args().collect());

    // Parse CLI args using vite_task::CLIArgs
    // Prepend "vite" as program name for clap
    let args_with_program = std::iter::once("vite".to_string()).chain(args_vec.iter().cloned());
    let cli_args =
        match CLIArgs::<CustomTaskSubcommand, NonTaskSubcommand>::try_parse_from(args_with_program)
        {
            Ok(args) => args,
            Err(err) => {
                err.exit();
            }
        };

    match cli_args {
        CLIArgs::NonTask(non_task) => {
            // Handle non-task subcommands directly (no Session needed)
            match non_task {
                NonTaskSubcommand::Cache { subcmd } => handle_cache_command(cwd, subcmd).await,
            }
        }
        CLIArgs::Task(task_cli_args) => {
            // Get workspace root path first (needed for synthesizer)
            let (workspace_root, _) = vite_workspace::find_workspace_root(&cwd)?;
            let workspace_path: Arc<AbsolutePath> = workspace_root.path.into();

            // Create session callbacks
            let mut task_synthesizer = if let Some(options) = options {
                VitePlusTaskSynthesizer::new(Arc::clone(&workspace_path)).with_cli_options(options)
            } else {
                VitePlusTaskSynthesizer::new(Arc::clone(&workspace_path))
            };
            let mut config_loader = vite_task::loader::JsonUserConfigLoader::default();

            // Create single Session
            let mut session = Session::init(SessionCallbacks {
                task_synthesizer: &mut task_synthesizer,
                user_config_loader: &mut config_loader,
            })?;

            // Auto-install (unless package manager command or disabled)
            if !is_package_manager_command(&args_vec)
                && env::var_os("VITE_DISABLE_AUTO_INSTALL") != Some("1".into())
                && env::var("VITE_TASK_EXECUTION_ENV").ok().as_deref() != Some("1")
            {
                // Use session.plan_synthetic_task for auto-install
                if let Ok(install_request) = create_install_synthetic_request(&cwd).await {
                    if let Ok(plan) = session.plan_synthetic_task(install_request).await {
                        // Use LabeledReporter with hide_summary and silent_if_cache_hit
                        let mut reporter =
                            LabeledReporter::new(std::io::stdout(), session.workspace_path());
                        reporter.set_hide_summary(true);
                        reporter.set_silent_if_cache_hit(true);
                        let _ = session.execute(plan, Box::new(reporter)).await;
                    }
                }

                // Update PATH to include package manager bin directory
                if let Ok(pm) =
                    vite_install::PackageManager::builder(&cwd).build_with_default().await
                {
                    let new_path = vite_install::format_path_env(&pm.get_bin_prefix());
                    // SAFETY: Single-threaded context before task execution
                    unsafe { env::set_var("PATH", new_path) };
                }
            }

            // Plan and execute the main command
            let cwd_arc: Arc<AbsolutePath> = cwd.into();
            let plan = session
                .plan_from_cli(cwd_arc, task_cli_args)
                .await
                .map_err(|e| Error::Anyhow(e.into()))?;
            let reporter = LabeledReporter::new(std::io::stdout(), session.workspace_path());
            session.execute(plan, Box::new(reporter)).await.map_err(|e| Error::Anyhow(e.into()))?;

            Ok(ExitStatus::default())
        }
    }
}

pub fn init_tracing() {
    use std::sync::OnceLock;

    use tracing_subscriber::{
        filter::{LevelFilter, Targets},
        prelude::__tracing_subscriber_SubscriberExt,
        util::SubscriberInitExt,
    };

    static TRACING: OnceLock<()> = OnceLock::new();
    TRACING.get_or_init(|| {
        tracing_subscriber::registry()
            .with(
                std::env::var("VITE_LOG")
                    .map_or_else(
                        |_| Targets::new(),
                        |env_var| {
                            use std::str::FromStr;
                            Targets::from_str(&env_var).unwrap_or_default()
                        },
                    )
                    // disable brush-parser tracing
                    .with_targets([("tokenize", LevelFilter::OFF), ("parse", LevelFilter::OFF)]),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();
    });
}
