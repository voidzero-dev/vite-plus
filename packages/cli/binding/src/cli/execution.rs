use std::{borrow::Cow, process::Stdio, sync::Arc};

use vp_error::Error;
use vt::ExitStatus;
use vt_path::AbsolutePathBuf;

use super::{
    resolver::SubcommandResolver,
    types::{CapturedCommandOutput, EnvMap, SynthesizableSubcommand, exit_status_from},
};

/// Resolve a subcommand into a prepared `tokio::process::Command`.
async fn resolve_and_build_command(
    resolver: &SubcommandResolver,
    subcommand: SynthesizableSubcommand,
    envs: &Arc<EnvMap>,
    cwd: &AbsolutePathBuf,
) -> Result<tokio::process::Command, Error> {
    let resolved = resolver.resolve(subcommand, envs, cwd).await.map_err(Error::Anyhow)?;

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

pub(crate) async fn resolve_and_capture_output(
    resolver: &SubcommandResolver,
    subcommand: SynthesizableSubcommand,
    envs: &Arc<EnvMap>,
    cwd: &AbsolutePathBuf,
    force_color_if_terminal: bool,
) -> Result<CapturedCommandOutput, Error> {
    let mut cmd = resolve_and_build_command(resolver, subcommand, envs, cwd).await?;
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    // Capturing output hides the terminal from the child. Preserve colors only when
    // the parent supports them, without overriding an explicit FORCE_COLOR value.
    if force_color_if_terminal
        && console::colors_enabled()
        && !cmd.as_std().get_envs().any(|(key, _)| key == "FORCE_COLOR")
    {
        cmd.env("FORCE_COLOR", "1");
    }

    let child = cmd.spawn().map_err(|e| Error::Anyhow(e.into()))?;
    let output = child.wait_with_output().await.map_err(|e| Error::Anyhow(e.into()))?;

    Ok(CapturedCommandOutput {
        status: exit_status_from(output.status),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}
