use std::iter;

use clap::{Parser, error::ErrorKind};
use vite_path::AbsolutePath;
use vite_str::Str;
use vite_task::{
    CommandHandler, HandledCommand, ScriptCommand,
    config::user::{EnabledCacheConfig, UserCacheConfig, UserRunConfig},
    loader::UserConfigLoader,
};

use super::{
    resolver::{SubcommandResolver, check_cache_inputs},
    types::{CLIArgs, ResolvedUniversalViteConfig, SynthesizableSubcommand, ViteConfigResolverFn},
};

/// CommandHandler implementation for vite-plus.
/// Handles `vp` commands in task scripts.
pub(super) struct VitePlusCommandHandler {
    resolver: SubcommandResolver,
}

impl std::fmt::Debug for VitePlusCommandHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VitePlusCommandHandler").finish()
    }
}

impl VitePlusCommandHandler {
    pub(super) fn new(resolver: SubcommandResolver) -> Self {
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
                let resolved = self.resolver.resolve(subcmd, None, &command.envs).await?;
                Ok(HandledCommand::Synthesized(resolved.into_synthetic_plan_request()))
            }
            CLIArgs::ViteTask(cmd) => Ok(HandledCommand::ViteTaskCommand(cmd)),
            CLIArgs::PackageManager(_) | CLIArgs::Exec(_) => {
                // PM commands and exec in task scripts run as subprocesses
                // — no caching, no synthesis through the resolver.
                Ok(HandledCommand::Synthesized(
                    command.to_synthetic_plan_request(UserCacheConfig::disabled()),
                ))
            }
        }
    }
}

/// User config loader that resolves vite.config.ts via JavaScript callback
pub(super) struct VitePlusConfigLoader {
    resolve_fn: ViteConfigResolverFn,
}

impl VitePlusConfigLoader {
    pub(super) fn new(resolve_fn: ViteConfigResolverFn) -> Self {
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
