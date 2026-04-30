use std::{borrow::Cow, ffi::OsStr, io::IsTerminal, process::Stdio, sync::Arc};

use rustc_hash::FxHashMap;
use vite_error::Error;
use vite_path::AbsolutePathBuf;
use vite_task::ExitStatus;

use super::{
    resolver::SubcommandResolver,
    types::{CapturedCommandOutput, ResolvedUniversalViteConfig, SynthesizableSubcommand},
};

/// Resolve a subcommand into a prepared `tokio::process::Command`.
async fn resolve_and_build_command(
    resolver: &SubcommandResolver,
    subcommand: SynthesizableSubcommand,
    resolved_vite_config: Option<&ResolvedUniversalViteConfig>,
    envs: &Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>>,
    cwd: &AbsolutePathBuf,
) -> Result<tokio::process::Command, Error> {
    let resolved = resolver
        .resolve(subcommand, resolved_vite_config, envs)
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
pub(super) async fn resolve_and_execute(
    resolver: &SubcommandResolver,
    subcommand: SynthesizableSubcommand,
    resolved_vite_config: Option<&ResolvedUniversalViteConfig>,
    envs: &Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>>,
    cwd: &AbsolutePathBuf,
) -> Result<ExitStatus, Error> {
    let is_interactive = matches!(
        subcommand,
        SynthesizableSubcommand::Dev { .. } | SynthesizableSubcommand::Preview { .. }
    );

    let mut cmd =
        resolve_and_build_command(resolver, subcommand, resolved_vite_config, envs, cwd).await?;

    // For interactive commands (dev, preview), use terminal guard to restore terminal state on exit
    if is_interactive {
        let status = vite_command::execute_with_terminal_guard(cmd).await?;
        Ok(ExitStatus(status.code().unwrap_or(1) as u8))
    } else {
        let mut child = cmd.spawn().map_err(|e| Error::Anyhow(e.into()))?;
        let status = child.wait().await.map_err(|e| Error::Anyhow(e.into()))?;
        Ok(ExitStatus(status.code().unwrap_or(1) as u8))
    }
}

pub(super) enum FilterStream {
    Stdout,
    Stderr,
}

/// Like `resolve_and_execute`, but captures one stream (stdout or stderr),
/// applies a text filter, and writes the result back. The other stream remains inherited.
pub(super) async fn resolve_and_execute_with_filter(
    resolver: &SubcommandResolver,
    subcommand: SynthesizableSubcommand,
    resolved_vite_config: Option<&ResolvedUniversalViteConfig>,
    envs: &Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>>,
    cwd: &AbsolutePathBuf,
    stream: FilterStream,
    filter: impl Fn(&str) -> Cow<'_, str>,
) -> Result<ExitStatus, Error> {
    let mut cmd =
        resolve_and_build_command(resolver, subcommand, resolved_vite_config, envs, cwd).await?;
    match stream {
        FilterStream::Stdout => cmd.stdout(Stdio::piped()),
        FilterStream::Stderr => cmd.stderr(Stdio::piped()),
    };

    let child = cmd.spawn().map_err(|e| Error::Anyhow(e.into()))?;
    let output = child.wait_with_output().await.map_err(|e| Error::Anyhow(e.into()))?;

    use std::io::Write;
    match stream {
        FilterStream::Stdout => {
            let text = String::from_utf8_lossy(&output.stdout);
            let _ = std::io::stdout().lock().write_all(filter(&text).as_bytes());
        }
        FilterStream::Stderr => {
            let text = String::from_utf8_lossy(&output.stderr);
            let _ = std::io::stderr().lock().write_all(filter(&text).as_bytes());
        }
    }

    Ok(ExitStatus(output.status.code().unwrap_or(1) as u8))
}

pub(crate) async fn resolve_and_capture_output(
    resolver: &SubcommandResolver,
    subcommand: SynthesizableSubcommand,
    resolved_vite_config: Option<&ResolvedUniversalViteConfig>,
    envs: &Arc<FxHashMap<Arc<OsStr>, Arc<OsStr>>>,
    cwd: &AbsolutePathBuf,
    force_color_if_terminal: bool,
) -> Result<CapturedCommandOutput, Error> {
    let mut cmd =
        resolve_and_build_command(resolver, subcommand, resolved_vite_config, envs, cwd).await?;
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
