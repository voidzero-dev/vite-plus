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
        // A leading global `-C` keeps the child process (it applies the chdir
        // and reports bad targets), but the command must still carry its
        // subcommand's cache config. Verbatim skipped it, so `vp -C <dir>
        // check` in a cached task never fingerprinted OXLINT_TSGOLINT_PATH.
        if command.args.first().is_some_and(|arg| arg.starts_with("-C")) {
            let Some(cache_config) = chdir_cache_config(command, is_vpr) else {
                return Ok(HandledCommand::Verbatim);
            };
            return Ok(HandledCommand::Synthesized(
                command.to_synthetic_plan_request(cache_config),
            ));
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

/// Index of the first argument after a leading global `-C <dir>` /
/// `-C<dir>` / `-C=<dir>`, matching the global CLI grammar. Returns `None` for
/// a malformed flag or a missing target directory so the caller can fall back
/// to the verbatim child process, which reports the error itself.
fn leading_chdir_end(command: &ScriptCommand) -> Option<usize> {
    let first = command.args.first().map(Str::as_str)?;
    let inline = first.strip_prefix("-C")?;
    let (dir, consumed) = if inline.is_empty() {
        (command.args.get(1).map(Str::as_str)?, 2)
    } else {
        (inline.strip_prefix('=').unwrap_or(inline), 1)
    };
    if dir.is_empty() || !command.cwd.join(dir).as_path().is_dir() {
        return None;
    }
    Some(consumed)
}

/// Cache config for the subcommand behind a leading `-C`. The command itself
/// stays untouched: the child still applies the chdir, and a malformed target
/// keeps the old verbatim fallback.
fn chdir_cache_config(command: &ScriptCommand, is_vpr: bool) -> Option<UserCacheConfig> {
    let consumed = leading_chdir_end(command)?;
    let cli_args = CLIArgs::try_parse_from(
        iter::once("vp")
            .chain(is_vpr.then_some("run"))
            .chain(command.args[consumed..].iter().map(Str::as_str)),
    )
    .ok()?;
    match cli_args {
        CLIArgs::Synthesizable(subcmd) => Some(cache_config_for(&subcmd)),
        _ => None,
    }
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
    use std::sync::Arc;

    use vt::{CommandHandler, HandledCommand, ScriptCommand, config::user::UserCacheConfig};
    use vt_path::AbsolutePathBuf;
    use vt_str::Str;

    use super::{SubcommandResolver, leading_chdir_end};

    fn command(args: &[&str], cwd: &AbsolutePathBuf) -> ScriptCommand {
        ScriptCommand {
            program: Str::from("vp"),
            args: args.iter().map(|arg| Str::from(*arg)).collect::<Vec<_>>().into(),
            envs: Arc::new(Default::default()),
            cwd: Arc::from(cwd.clone()),
        }
    }

    fn cache_env(command: &ScriptCommand) -> Vec<String> {
        let UserCacheConfig::Config(config) =
            super::chdir_cache_config(command, false).expect("expected cache config")
        else {
            panic!("expected enabled cache config");
        };
        config
            .env
            .expect("expected tracked env vars")
            .iter()
            .map(|name| name.as_str().to_owned())
            .collect()
    }

    #[test]
    fn finds_all_leading_chdir_forms() {
        let cwd = AbsolutePathBuf::new(std::env::current_dir().unwrap()).unwrap();

        for (args, expected) in [
            (&["-C", ".", "check", "index.js"][..], 2),
            (&["-C.", "check", "index.js"][..], 1),
            (&["-C=.", "check", "index.js"][..], 1),
        ] {
            assert_eq!(leading_chdir_end(&command(args, &cwd)), Some(expected));
        }
    }

    #[test]
    fn malformed_chdir_falls_back_to_verbatim() {
        let cwd = AbsolutePathBuf::new(std::env::current_dir().unwrap()).unwrap();

        for args in [&["-C"][..], &["-C", ""][..], &["-C", "definitely-missing-dir"][..]] {
            assert_eq!(leading_chdir_end(&command(args, &cwd)), None);
            assert!(super::chdir_cache_config(&command(args, &cwd), false).is_none());
        }
    }

    #[test]
    fn chdir_carries_subcommand_cache_config() {
        let cwd = AbsolutePathBuf::new(std::env::current_dir().unwrap()).unwrap();

        assert_eq!(
            cache_env(&command(&["-C", ".", "check", "index.js"], &cwd)),
            ["OXLINT_TSGOLINT_PATH", "VP_SKIP_STATIC_CONFIG"]
        );
        assert_eq!(
            cache_env(&command(&["-C", ".", "lint", "--type-aware"], &cwd)),
            ["OXLINT_TSGOLINT_PATH"]
        );
    }

    #[tokio::test]
    async fn chdir_keeps_the_original_command() {
        let cwd = AbsolutePathBuf::new(std::env::current_dir().unwrap()).unwrap();
        let resolver = SubcommandResolver::new(Arc::from(cwd.clone()));
        let mut handler = super::VitePlusCommandHandler::new(resolver);
        let mut command = command(&["-C", ".", "check", "index.js"], &cwd);

        let HandledCommand::Synthesized(request) =
            handler.handle_command(&mut command).await.unwrap()
        else {
            panic!("expected check to be synthesized");
        };
        let UserCacheConfig::Config(config) = request.cache_config else {
            panic!("expected check cache config");
        };
        assert!(
            config
                .env
                .expect("check should track env vars")
                .iter()
                .any(|name| { name.as_str() == "OXLINT_TSGOLINT_PATH" })
        );
        assert_eq!(
            request.args.iter().map(Str::as_str).collect::<Vec<_>>(),
            ["-C", ".", "check", "index.js"]
        );
    }
}
