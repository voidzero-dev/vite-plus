use std::iter;

use clap::{Parser, error::ErrorKind};
use vt::{
    CommandHandler, HandledCommand, ScriptCommand,
    config::user::{UserCacheConfig, UserRunConfig},
    loader::UserConfigLoader,
};
use vt_path::AbsolutePath;
use vt_str::Str;

use super::{
    resolver::{SubcommandResolver, cache_config_for, resolve_vite_config_fields},
    types::{CLIArgs, SynthesizableSubcommand, ViteConfigResolverFn},
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
            #[cfg(windows)]
            rewrite_managed_script_command(command);
            return Ok(HandledCommand::Verbatim);
        }

        // Help must pass through the local JS entrypoint, which owns the prepared documents.
        if command
            .args
            .iter()
            .take_while(|arg| arg.as_str() != "--")
            .any(|arg| matches!(arg.as_str(), "-h" | "--help"))
        {
            return Ok(HandledCommand::Verbatim);
        }

        // "vpr <args>" is shorthand for "vp run <args>", so prepend "run" for parsing.
        let is_vpr = program == "vpr";

        // A leading global `-C <dir>` keeps running in a child vp, which applies the
        // directory, runtime, and toolchain like a direct invocation. A built-in subcommand
        // only adds its cache config, so its tracked envs and inputs apply (#3503).
        if command.args.first().is_some_and(|arg| arg.starts_with("-C")) {
            let subcommand = args_after_leading_chdir(&command.args).and_then(|rest| {
                CLIArgs::try_parse_from(
                    iter::once("vp")
                        .chain(is_vpr.then_some("run"))
                        .chain(rest.iter().map(Str::as_str)),
                )
                .ok()
            });
            return Ok(match subcommand {
                Some(CLIArgs::Synthesizable(subcmd)) => HandledCommand::Synthesized(
                    command.to_synthetic_plan_request(cache_config_for(&subcmd)),
                ),
                _ => HandledCommand::Verbatim,
            });
        }
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
            CLIArgs::Synthesizable(subcmd @ SynthesizableSubcommand::Check { .. }) => {
                // Check is a composite command (fmt + lint) — run as a subprocess in task scripts
                Ok(HandledCommand::Synthesized(
                    command.to_synthetic_plan_request(cache_config_for(&subcmd)),
                ))
            }
            CLIArgs::Synthesizable(subcmd) => {
                // Bare app commands in scripts get the same workspace-root
                // target elicitation as direct invocations: spawn the real
                // binary, which elicits (defaultPackage note, listing +
                // exit 1) identically instead of silently running the root.
                if super::app_target::needs_elicitation(&subcmd, &command.cwd) {
                    return Ok(HandledCommand::Verbatim);
                }
                let resolved = self.resolver.resolve(subcmd, &command.envs, &command.cwd).await?;
                Ok(HandledCommand::Synthesized(resolved.into_synthetic_plan_request()))
            }
            CLIArgs::ViteTask(cmd) => Ok(HandledCommand::ViteTaskCommand(cmd)),
            CLIArgs::PackageManager(_) | CLIArgs::Exec(_) | CLIArgs::Toolchain(_) => {
                // PM commands and exec in task scripts run as subprocesses
                // — no caching, no synthesis through the resolver.
                Ok(HandledCommand::Synthesized(
                    command.to_synthetic_plan_request(UserCacheConfig::disabled()),
                ))
            }
        }
    }
}

fn args_after_leading_chdir(args: &[Str]) -> Option<&[Str]> {
    let rest = args.first()?.as_str().strip_prefix("-C")?;
    let (dir, remaining) = if rest.is_empty() {
        (args.get(1)?.as_str(), &args[2..])
    } else {
        (rest.strip_prefix('=').unwrap_or(rest), &args[1..])
    };
    (!dir.is_empty()).then_some(remaining)
}

/// Vite Task rewrites workspace `.bin` shims. Managed package managers live
/// outside the workspace, so apply Vite+'s launcher policy before the plan is
/// fingerprinted. This also preserves the normal non-TTY and missing-.ps1
/// fallbacks and leaves system tools to the task runner.
#[cfg(windows)]
fn rewrite_managed_script_command(command: &mut ScriptCommand) {
    let path_env = vt::get_path_env(&command.envs).map(|path| path.as_ref());
    let Ok(resolved) = vp_command::resolve_bin(&command.program, path_env, &command.cwd) else {
        return;
    };
    if !resolved.as_path().starts_with(&vp_shared::EnvConfig::get().dirs.data) {
        return;
    }
    let Some((host, prefix_args)) = vp_command::rewrite_cmd_to_powershell(&resolved) else {
        return;
    };
    let Some(program) = host.as_path().to_str() else { return };
    let Some(mut args) =
        prefix_args.iter().map(|arg| arg.to_str().map(Str::from)).collect::<Option<Vec<_>>>()
    else {
        return;
    };
    args.extend(command.args.iter().cloned());
    command.program = program.into();
    command.args = args.into();
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
        // The task graph reads an absent config as the default, so `None` covers both.
        let config = resolve_vite_config_fields(package_path, &["run"], &self.resolve_fn).await?;
        Ok(config.run.map(serde_json::from_value).transpose()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rest(args: &[&str]) -> Option<Vec<String>> {
        let args: Vec<Str> = args.iter().map(|arg| Str::from(*arg)).collect();
        args_after_leading_chdir(&args).map(|rest| rest.iter().map(ToString::to_string).collect())
    }

    #[test]
    fn skips_every_leading_chdir_form() {
        let expected = Some(vec!["lint".to_owned()]);
        assert_eq!(rest(&["-C", "apps/web", "lint"]), expected);
        assert_eq!(rest(&["-C=apps/web", "lint"]), expected);
        assert_eq!(rest(&["-Capps/web", "lint"]), expected);
        assert_eq!(rest(&["-C"]), None);
        assert_eq!(rest(&["-C=", "lint"]), None);
        assert_eq!(rest(&["lint", "-C", "apps/web"]), None);
    }
}
