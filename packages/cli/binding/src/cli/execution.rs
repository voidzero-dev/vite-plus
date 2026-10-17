use std::{borrow::Cow, process::Stdio, sync::Arc};

use vp_error::Error;
use vt::ExitStatus;
use vt_path::AbsolutePathBuf;

use super::{
    resolver::SubcommandResolver,
    types::{EnvMap, ResolvedSubcommand, SynthesizableSubcommand, exit_status_from},
};

/// Resolve a subcommand into a prepared `tokio::process::Command`.
async fn resolve_and_build_command(
    resolver: &SubcommandResolver,
    subcommand: SynthesizableSubcommand,
    envs: &Arc<EnvMap>,
    cwd: &AbsolutePathBuf,
) -> Result<tokio::process::Command, Error> {
    let resolved = resolver.resolve(subcommand, envs, cwd).await.map_err(Error::Anyhow)?;
    build_command(&resolved, cwd)
}

/// Prepare a `tokio::process::Command` for an already resolved subcommand.
fn build_command(
    resolved: &ResolvedSubcommand,
    cwd: &AbsolutePathBuf,
) -> Result<tokio::process::Command, Error> {
    // Resolve the program path using `which` to handle Windows .cmd/.bat files (PATHEXT)
    let program_path = vp_command::resolve_bin(
        resolved.program.as_ref().to_str().unwrap_or_default(),
        vt::get_path_env(&resolved.envs).map(AsRef::as_ref),
        cwd,
    )?;

    let mut cmd = vp_command::build_command(&program_path, cwd);
    cmd.args(resolved.args.iter().map(|s| s.as_str()))
        .env_clear()
        .envs(resolved.envs.iter().map(|(k, v)| (k.inner(), v)));
    Ok(cmd)
}

/// Spawn the internal `vp check --raw` runner with piped stdio.
pub(crate) fn spawn_check_raw(
    resolver: &SubcommandResolver,
    envs: &Arc<EnvMap>,
    cwd: &AbsolutePathBuf,
) -> Result<tokio::process::Child, Error> {
    let resolved = resolver.resolve_check_raw(envs).map_err(Error::Anyhow)?;
    let mut cmd = build_command(&resolved, cwd)?;
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    cmd.spawn().map_err(|e| Error::Anyhow(e.into()))
}

/// Resolve a single subcommand and execute it, returning its exit status.
pub(super) async fn resolve_and_execute(
    resolver: &SubcommandResolver,
    subcommand: SynthesizableSubcommand,
    envs: &Arc<EnvMap>,
    cwd: &AbsolutePathBuf,
) -> Result<ExitStatus, Error> {
    let is_interactive = matches!(
        subcommand,
        SynthesizableSubcommand::Dev { .. } | SynthesizableSubcommand::Preview { .. }
    );

    let mut cmd = resolve_and_build_command(resolver, subcommand, envs, cwd).await?;

    // For interactive commands (dev, preview), use terminal guard to restore terminal state on exit
    let status = if is_interactive {
        vp_command::execute_with_terminal_guard(cmd).await?
    } else {
        let mut child = cmd.spawn().map_err(|e| Error::Anyhow(e.into()))?;
        child.wait().await.map_err(|e| Error::Anyhow(e.into()))?
    };
    Ok(exit_status_from(status))
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
    envs: &Arc<EnvMap>,
    cwd: &AbsolutePathBuf,
    stream: FilterStream,
    filter: impl Fn(&str) -> Cow<'_, str>,
) -> Result<ExitStatus, Error> {
    let mut cmd = resolve_and_build_command(resolver, subcommand, envs, cwd).await?;
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

    Ok(exit_status_from(output.status))
}
