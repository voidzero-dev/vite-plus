use std::{borrow::Cow, env, ffi::OsStr, iter, sync::Arc};

use cow_utils::CowUtils;
use rustc_hash::FxHashMap;
use vt::config::user::{
    AutoTracking, EnabledCacheConfig, GlobWithBase, InputBase, UserCacheConfig, UserInputEntry,
};
use vt_path::AbsolutePath;
use vt_str::Str;

use super::{
    help::should_prepend_vitest_run,
    types::{CliOptions, ResolvedSubcommand, ResolvedUniversalViteConfig, SynthesizableSubcommand},
};

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

    fn cli_options(&self) -> anyhow::Result<&CliOptions> {
        self.cli_options
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("CLI options not available (running without NAPI?)"))
    }

    pub(crate) async fn resolve_universal_vite_config(
        &self,
    ) -> anyhow::Result<ResolvedUniversalViteConfig> {
        let cli_options = self.cli_options()?;
        let workspace_path_str = self
            .workspace_path
            .as_path()
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("workspace path is not valid UTF-8"))?;
        if let Some(config) = vp_static_config::resolve_static_metadata(&self.workspace_path)
            && let Some(config_file) = config.config_file.as_path().to_str()
        {
            let mut resolved: ResolvedUniversalViteConfig =
                serde_json::from_value(config.metadata)?;
            resolved.config_file = Some(config_file.cow_replace('\\', "/").into_owned());
            return Ok(resolved);
        }
        let vite_config_json =
            (cli_options.resolve_universal_vite_config)(workspace_path_str.to_string()).await?;

        Ok(serde_json::from_str(&vite_config_json).inspect_err(|_| {
            tracing::error!("Failed to parse vite config: {vite_config_json}");
        })?)
    }

    /// Resolve root settings for package runs unless the user selected another config.
    async fn resolve_workspace_config<'a>(
        &self,
        cwd: &AbsolutePath,
        args: &[String],
        resolved_config: Option<&'a ResolvedUniversalViteConfig>,
    ) -> anyhow::Result<Option<Cow<'a, ResolvedUniversalViteConfig>>> {
        let explicit_config = args
            .iter()
            .take_while(|arg| arg.as_str() != "--")
            .any(|arg| arg.starts_with("-c") || arg == "--config" || arg.starts_with("--config="));
        if cwd == self.workspace_path.as_ref() || explicit_config {
            return Ok(None);
        }
        // Composite commands can reuse their config within this invocation.
        // Do not cache it on the resolver: task runs may observe config changes.
        if let Some(config) = resolved_config {
            return Ok(Some(Cow::Borrowed(config)));
        }
        self.resolve_universal_vite_config().await.map(|config| Some(Cow::Owned(config)))
    }

    /// Resolve a synthesizable subcommand to a concrete program, args, cache config, and envs.
    pub(super) async fn resolve(
        &self,
        subcommand: SynthesizableSubcommand,
        envs: &Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>>,
        cwd: &AbsolutePath,
        resolved_config: Option<&ResolvedUniversalViteConfig>,
    ) -> anyhow::Result<ResolvedSubcommand> {
        match subcommand {
            SynthesizableSubcommand::Lint { mut args } => {
                let cli_options = self.cli_options()?;
                let resolved = (cli_options.lint)(cwd, &args).await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("lint JS path is not valid UTF-8"))?;

                if let Some(config) =
                    self.resolve_workspace_config(cwd, &args, resolved_config).await?
                    && config.lint.is_some()
                    && let Some(config_file) = &config.config_file
                {
                    args.insert(0, "-c".to_string());
                    args.insert(1, config_file.clone());
                }

                Ok(ResolvedSubcommand {
                    program: Arc::clone(&cli_options.node_exec_path),
                    args: [
                        Str::from("--disable-warning=MODULE_TYPELESS_PACKAGE_JSON"),
                        Str::from(js_path_str),
                    ]
                    .into_iter()
                    .chain(args.into_iter().map(Str::from))
                    .collect(),
                    cache_config: UserCacheConfig::with_config(EnabledCacheConfig {
                        env: Some(Box::new([Str::from("OXLINT_TSGOLINT_PATH")])),
                        untracked_env: None,
                        input: None,
                        output: None,
                    }),
                    envs: merge_resolved_envs_with_version(envs, resolved.envs),
                })
            }
            SynthesizableSubcommand::Fmt { mut args } => {
                let cli_options = self.cli_options()?;
                let resolved = (cli_options.fmt)(cwd, &args).await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("fmt JS path is not valid UTF-8"))?;

                if let Some(config) =
                    self.resolve_workspace_config(cwd, &args, resolved_config).await?
                    && config.fmt.is_some()
                    && let Some(config_file) = &config.config_file
                {
                    args.insert(0, "-c".to_string());
                    args.insert(1, config_file.clone());
                }

                Ok(ResolvedSubcommand {
                    program: Arc::clone(&cli_options.node_exec_path),
                    args: iter::once(Str::from(js_path_str))
                        .chain(args.into_iter().map(Str::from))
                        .collect(),
                    cache_config: UserCacheConfig::with_config(EnabledCacheConfig {
                        env: None,
                        untracked_env: None,
                        input: None,
                        output: None,
                    }),
                    envs: merge_resolved_envs_with_version(envs, resolved.envs),
                })
            }
            SynthesizableSubcommand::Build { args } => {
                let cli_options = self.cli_options()?;
                let resolved = (cli_options.vite)(cwd, &args).await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("vite JS path is not valid UTF-8"))?;

                Ok(ResolvedSubcommand {
                    program: Arc::clone(&cli_options.node_exec_path),
                    args: iter::once(Str::from(js_path_str))
                        .chain(iter::once(Str::from("build")))
                        .chain(args.into_iter().map(Str::from))
                        .collect(),
                    // No synthetic cache config: vite reports its inputs/outputs/
                    // envs to the runner via `@voidzero-dev/vite-task-client`.
                    // All fields `None` keep caching enabled with auto input and
                    // auto output inference (the latter drives output restoration);
                    // vite's `ignoreInput`/`ignoreOutput`/`getEnv`/`getEnvs` refine
                    // the fingerprint at runtime.
                    cache_config: UserCacheConfig::with_config(EnabledCacheConfig {
                        env: None,
                        untracked_env: None,
                        input: None,
                        output: None,
                    }),
                    envs: merge_resolved_envs_with_version(envs, resolved.envs),
                })
            }
            SynthesizableSubcommand::Test { args } => {
                let cli_options = self.cli_options()?;
                let resolved = (cli_options.test)(cwd, &args).await?;
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
                    program: Arc::clone(&cli_options.node_exec_path),
                    args: iter::once(Str::from(js_path_str)).chain(vitest_args).collect(),
                    cache_config: UserCacheConfig::with_config(EnabledCacheConfig {
                        env: None,
                        untracked_env: None,
                        input: Some(vec![
                            UserInputEntry::Auto(AutoTracking { auto: true }),
                            exclude_glob(
                                "!node_modules/.vite/vitest/**/results.json",
                                InputBase::Package,
                            ),
                        ]),
                        output: None,
                    }),
                    envs: merge_resolved_envs_with_version(envs, resolved.envs),
                })
            }
            SynthesizableSubcommand::Pack { args } => {
                let cli_options = self.cli_options()?;
                let resolved = (cli_options.pack)(cwd, &args).await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("pack JS path is not valid UTF-8"))?;

                Ok(ResolvedSubcommand {
                    program: Arc::clone(&cli_options.node_exec_path),
                    args: iter::once(Str::from(js_path_str))
                        .chain(args.into_iter().map(Str::from))
                        .collect(),
                    cache_config: UserCacheConfig::with_config(EnabledCacheConfig {
                        env: None,
                        untracked_env: None,
                        input: Some(build_pack_cache_inputs()),
                        output: None,
                    }),
                    envs: merge_resolved_envs(envs, resolved.envs),
                })
            }
            SynthesizableSubcommand::Dev { args } => {
                let cli_options = self.cli_options()?;
                let resolved = (cli_options.vite)(cwd, &args).await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("vite JS path is not valid UTF-8"))?;

                Ok(ResolvedSubcommand {
                    program: Arc::clone(&cli_options.node_exec_path),
                    args: iter::once(Str::from(js_path_str))
                        .chain(iter::once(Str::from("dev")))
                        .chain(args.into_iter().map(Str::from))
                        .collect(),
                    cache_config: UserCacheConfig::disabled(),
                    envs: merge_resolved_envs_with_version(envs, resolved.envs),
                })
            }
            SynthesizableSubcommand::Preview { args } => {
                let cli_options = self.cli_options()?;
                let resolved = (cli_options.vite)(cwd, &args).await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("vite JS path is not valid UTF-8"))?;

                Ok(ResolvedSubcommand {
                    program: Arc::clone(&cli_options.node_exec_path),
                    args: iter::once(Str::from(js_path_str))
                        .chain(iter::once(Str::from("preview")))
                        .chain(args.into_iter().map(Str::from))
                        .collect(),
                    cache_config: UserCacheConfig::disabled(),
                    envs: merge_resolved_envs_with_version(envs, resolved.envs),
                })
            }
            SynthesizableSubcommand::Doc { args } => {
                let cli_options = self.cli_options()?;
                let resolved = (cli_options.doc)(cwd, &args).await?;
                let js_path = resolved.bin_path;
                let js_path_str = js_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("doc JS path is not valid UTF-8"))?;

                Ok(ResolvedSubcommand {
                    program: Arc::clone(&cli_options.node_exec_path),
                    args: iter::once(Str::from(js_path_str))
                        .chain(args.into_iter().map(Str::from))
                        .collect(),
                    cache_config: UserCacheConfig::with_config(EnabledCacheConfig {
                        env: None,
                        untracked_env: None,
                        input: None,
                        output: None,
                    }),
                    envs: merge_resolved_envs(envs, resolved.envs),
                })
            }
            SynthesizableSubcommand::Check { .. } => {
                anyhow::bail!(
                    "Check is a composite command and cannot be resolved to a single subcommand"
                );
            }
        }
    }
}

/// Create a negative glob entry to exclude a pattern from cache fingerprinting.
fn exclude_glob(pattern: &str, base: InputBase) -> UserInputEntry {
    UserInputEntry::GlobWithBase(GlobWithBase { pattern: Str::from(pattern), base })
}

/// Common cache input entries for the pack command.
/// Excludes dist output files that are both read and written.
/// TODO: The hardcoded `!dist/**` exclusion is a temporary workaround. It will be replaced
/// by a runner-aware approach that automatically excludes task output directories.
fn build_pack_cache_inputs() -> Vec<UserInputEntry> {
    vec![
        UserInputEntry::Auto(AutoTracking { auto: true }),
        exclude_glob("!dist/**", InputBase::Package),
    ]
}

/// Cache input entries for the check command.
/// The vp check subprocess is a full vp CLI process (not resolved to a binary like
/// build/lint/fmt), so it accesses additional directories that must be excluded:
/// - `.vite/task-cache`: task runner state files that change after each run
pub(super) fn check_cache_inputs() -> Vec<UserInputEntry> {
    vec![
        UserInputEntry::Auto(AutoTracking { auto: true }),
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

#[cfg(test)]
mod tests {
    use vt_path::AbsolutePathBuf;

    use super::*;
    use crate::cli::types::{BoxedResolverFn, ResolveCommandResult};

    fn tool_resolver() -> BoxedResolverFn {
        Box::new(|_, _| {
            Box::pin(async {
                Ok(ResolveCommandResult {
                    bin_path: Arc::from(OsStr::new("tool.js")),
                    envs: Vec::new(),
                })
            })
        })
    }

    fn cli_options(runtime: Arc<OsStr>) -> CliOptions {
        CliOptions {
            node_exec_path: runtime,
            lint: tool_resolver(),
            fmt: tool_resolver(),
            vite: tool_resolver(),
            test: tool_resolver(),
            pack: tool_resolver(),
            doc: tool_resolver(),
            toolchain_manifest_path: String::new(),
            vite_plus_package_path: String::new(),
            resolve_universal_vite_config: Arc::new(|_| {
                Box::pin(async { anyhow::bail!("config loading is not expected for this command") })
            }),
        }
    }

    #[tokio::test]
    async fn builtins_reuse_the_calling_node_runtime() {
        let temp = tempfile::tempdir().unwrap();
        let cwd = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let runtime: Arc<OsStr> = Arc::from(cwd.join("custom runtime").as_path().as_os_str());
        let resolver = SubcommandResolver::new(cwd.clone().into())
            .with_cli_options(cli_options(Arc::clone(&runtime)));
        let envs = Arc::new(FxHashMap::default());
        for command in [
            SynthesizableSubcommand::Lint { args: vec![] },
            SynthesizableSubcommand::Fmt { args: vec![] },
            SynthesizableSubcommand::Build { args: vec![] },
            SynthesizableSubcommand::Test { args: vec![] },
            SynthesizableSubcommand::Pack { args: vec![] },
            SynthesizableSubcommand::Dev { args: vec![] },
            SynthesizableSubcommand::Preview { args: vec![] },
            SynthesizableSubcommand::Doc { args: vec![] },
        ] {
            let resolved = resolver.resolve(command, &envs, &cwd, None).await.unwrap();
            assert_eq!(resolved.program, runtime);
        }
    }

    #[tokio::test]
    async fn lint_and_fmt_preserve_args_without_loading_config() {
        let temp = tempfile::tempdir().unwrap();
        let cwd = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let resolver = SubcommandResolver::new(cwd.clone().into())
            .with_cli_options(cli_options(Arc::from(OsStr::new("node"))));
        let envs = Arc::new(FxHashMap::default());

        for args in [
            &["src"][..],
            &["-c", "custom.json", "src"],
            &["--config", "custom.json", "src"],
            &["--config=custom.json", "src"],
            &["--disable-nested-config", "src"],
        ] {
            let tool_args: Vec<String> = args.iter().map(|arg| (*arg).to_string()).collect();
            for (command, prefix) in [
                (
                    SynthesizableSubcommand::Lint { args: tool_args.clone() },
                    &["--disable-warning=MODULE_TYPELESS_PACKAGE_JSON", "tool.js"][..],
                ),
                (SynthesizableSubcommand::Fmt { args: tool_args }, &["tool.js"]),
            ] {
                let resolved = resolver.resolve(command, &envs, &cwd, None).await.unwrap();
                let actual_args: Vec<&str> = resolved.args.iter().map(|arg| arg.as_str()).collect();
                let expected_args = [prefix, args].concat();
                assert_eq!(actual_args, expected_args);
            }
        }
    }

    #[tokio::test]
    async fn lint_and_fmt_from_subdirectory_use_the_matching_root_config_block() {
        let temp = tempfile::tempdir().unwrap();
        let root = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let cwd = root.join("packages/app");
        let config_file = root.join("vite config.mts").as_path().to_str().unwrap().to_string();
        let envs = Arc::new(FxHashMap::default());

        for (mut config, has_lint, has_fmt) in [
            (serde_json::json!({}), false, false),
            (serde_json::json!({ "lint": {} }), true, false),
            (serde_json::json!({ "fmt": {} }), false, true),
            (serde_json::json!({ "lint": {}, "fmt": {} }), true, true),
        ] {
            config["configFile"] = serde_json::json!(config_file);
            let config = config.to_string();
            let root_string = root.as_path().to_str().unwrap().to_string();
            let mut options = cli_options(Arc::from(OsStr::new("node")));
            options.resolve_universal_vite_config = Arc::new(move |path| {
                assert_eq!(path, root_string);
                let config = config.clone();
                Box::pin(async move { Ok(config) })
            });
            let resolver = SubcommandResolver::new(root.clone().into()).with_cli_options(options);

            // A config-looking filename after `--` must not suppress root selection.
            for args in [&["index.ts"][..], &["--", "--config"]] {
                let tool_args: Vec<String> = args.iter().map(|arg| (*arg).to_string()).collect();
                for (command, prefix, has_block) in [
                    (
                        SynthesizableSubcommand::Lint { args: tool_args.clone() },
                        &["--disable-warning=MODULE_TYPELESS_PACKAGE_JSON", "tool.js"][..],
                        has_lint,
                    ),
                    (SynthesizableSubcommand::Fmt { args: tool_args }, &["tool.js"], has_fmt),
                ] {
                    let resolved = resolver.resolve(command, &envs, &cwd, None).await.unwrap();
                    let actual_args: Vec<&str> =
                        resolved.args.iter().map(|arg| arg.as_str()).collect();
                    let mut expected_args = prefix.to_vec();
                    if has_block {
                        expected_args.extend(["-c", config_file.as_str()]);
                    }
                    expected_args.extend(args);
                    assert_eq!(actual_args, expected_args);
                }
            }
        }
    }

    #[tokio::test]
    async fn check_phases_reuse_config_without_caching_later_commands() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let temp = tempfile::tempdir().unwrap();
        let root = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let cwd = root.join("packages/app");
        let config_file = root.join("vite.config.ts").as_path().to_str().unwrap().to_string();
        let loads = Arc::new(AtomicUsize::new(0));
        let mut options = cli_options(Arc::from(OsStr::new("node")));
        options.resolve_universal_vite_config = {
            let loads = Arc::clone(&loads);
            let config_file = config_file.clone();
            Arc::new(move |_| {
                // A later command must observe removal of the root tool blocks.
                let config = if loads.fetch_add(1, Ordering::SeqCst) == 0 {
                    serde_json::json!({ "configFile": config_file, "lint": {}, "fmt": {} })
                } else {
                    serde_json::json!({ "configFile": config_file })
                };
                Box::pin(async move { Ok(config.to_string()) })
            })
        };
        let resolver = SubcommandResolver::new(root.into()).with_cli_options(options);
        let envs = Arc::new(FxHashMap::default());
        let config = resolver.resolve_universal_vite_config().await.unwrap();

        for command in [
            SynthesizableSubcommand::Fmt { args: vec![] },
            SynthesizableSubcommand::Lint { args: vec![] },
            SynthesizableSubcommand::Fmt { args: vec![] },
        ] {
            let resolved = resolver.resolve(command, &envs, &cwd, Some(&config)).await.unwrap();
            let args: Vec<&str> = resolved.args.iter().map(|arg| arg.as_str()).collect();
            assert!(args.ends_with(&["-c", &config_file]));
        }
        assert_eq!(loads.load(Ordering::SeqCst), 1);

        let resolved = resolver
            .resolve(SynthesizableSubcommand::Fmt { args: vec![] }, &envs, &cwd, None)
            .await
            .unwrap();
        assert_eq!(loads.load(Ordering::SeqCst), 2);
        assert_eq!(resolved.args.as_ref(), &[Str::from("tool.js")]);
    }

    #[tokio::test]
    async fn literal_metadata_skips_javascript_and_observes_changes() {
        let temp = tempfile::tempdir().unwrap();
        let root = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let file = root.join("vite.config.ts");
        let resolver = SubcommandResolver::new(root.clone().into())
            .with_cli_options(cli_options(Arc::from(OsStr::new("node"))));
        std::fs::write(&file, "export default { lint: {}, fmt: {}, check: { fmt: false } };")
            .unwrap();

        // The callback in cli_options fails if JavaScript resolution is used.
        let config = resolver.resolve_universal_vite_config().await.unwrap();
        assert_eq!(config.lint, Some(serde_json::json!({})));
        assert_eq!(config.fmt, Some(serde_json::json!({})));
        assert_eq!(config.check, Some(serde_json::json!({ "fmt": false })));
        assert_eq!(
            config.config_file.as_deref(),
            Some(file.as_path().to_str().unwrap().cow_replace('\\', "/").as_ref())
        );

        let envs = Arc::new(FxHashMap::default());
        let cwd = root.join("packages/app");
        let resolved = resolver
            .resolve(
                SynthesizableSubcommand::Fmt { args: vec!["src".to_string()] },
                &envs,
                &cwd,
                Some(&config),
            )
            .await
            .unwrap();
        assert!(resolved.args.ends_with(&[
            Str::from("-c"),
            Str::from(config.config_file.unwrap()),
            Str::from("src")
        ]));

        std::fs::write(&file, "export default { check: { lint: false } };").unwrap();
        let config = resolver.resolve_universal_vite_config().await.unwrap();
        assert!(config.lint.is_none());
        assert!(config.fmt.is_none());
        assert_eq!(config.check, Some(serde_json::json!({ "lint": false })));

        std::fs::remove_file(file).unwrap();
        // Removal must return to the existing JavaScript no-config path.
        assert!(resolver.resolve_universal_vite_config().await.is_err());
    }

    #[tokio::test]
    async fn dynamic_metadata_and_vite_options_use_runtime_resolution() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let temp = tempfile::tempdir().unwrap();
        let root = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let file = root.join("vite.config.ts");
        let loads = Arc::new(AtomicUsize::new(0));
        let mut options = cli_options(Arc::from(OsStr::new("node")));
        options.resolve_universal_vite_config = {
            let loads = Arc::clone(&loads);
            Arc::new(move |_| {
                loads.fetch_add(1, Ordering::SeqCst);
                Box::pin(async { Ok(r#"{"check":{"fmt":false}}"#.to_string()) })
            })
        };
        let resolver = SubcommandResolver::new(root.into()).with_cli_options(options);
        for (index, source) in [
            "export default async () => ({ check: { fmt: false } });",
            "export default { plugins: [{ name: 'metadata', config() { return { check: { fmt: false } }; } }] };",
            "import { defineConfig } from 'vite-plus'; export default defineConfig({});",
            "export default {}; console.log('side effect');",
            "export default { envPrefix: '' };",
            "export default { __proto__: { check: { fmt: false } } };",
            "export default {",
        ].iter().enumerate() {
            std::fs::write(&file, source).unwrap();
            let config = resolver.resolve_universal_vite_config().await.unwrap();
            assert_eq!(config.check, Some(serde_json::json!({ "fmt": false })));
            assert_eq!(loads.load(Ordering::SeqCst), index + 1);
        }
    }

    #[tokio::test]
    async fn runtime_errors_are_not_hidden_by_static_extraction() {
        let temp = tempfile::tempdir().unwrap();
        let root = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        std::fs::write(
            root.join("vite.config.ts"),
            "export default {}; throw new Error('bad config');",
        )
        .unwrap();
        let mut options = cli_options(Arc::from(OsStr::new("node")));
        options.resolve_universal_vite_config =
            Arc::new(|_| Box::pin(async { anyhow::bail!("bad config") }));
        let resolver = SubcommandResolver::new(root.into()).with_cli_options(options);
        assert_eq!(
            resolver.resolve_universal_vite_config().await.unwrap_err().to_string(),
            "bad config"
        );
    }

    #[tokio::test]
    async fn explicit_lint_and_fmt_config_from_subdirectory_skips_root_config_loading() {
        let temp = tempfile::tempdir().unwrap();
        let root = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let cwd = root.join("packages/app");
        let resolver = SubcommandResolver::new(root.into())
            .with_cli_options(cli_options(Arc::from(OsStr::new("node"))));
        let envs = Arc::new(FxHashMap::default());

        for args in [
            &["-c", "custom.json", "index.ts"][..],
            &["-c./custom.json", "index.ts"],
            &["-c=custom.json", "index.ts"],
            &["--config", "custom.json", "index.ts"],
            &["--config=custom.json", "index.ts"],
        ] {
            let tool_args: Vec<String> = args.iter().map(|arg| (*arg).to_string()).collect();
            for (command, prefix) in [
                (
                    SynthesizableSubcommand::Lint { args: tool_args.clone() },
                    &["--disable-warning=MODULE_TYPELESS_PACKAGE_JSON", "tool.js"][..],
                ),
                (SynthesizableSubcommand::Fmt { args: tool_args }, &["tool.js"]),
            ] {
                let resolved = resolver.resolve(command, &envs, &cwd, None).await.unwrap();
                let actual_args: Vec<&str> = resolved.args.iter().map(|arg| arg.as_str()).collect();
                let expected_args = [prefix, args].concat();
                assert_eq!(actual_args, expected_args);
            }
        }
    }
}
