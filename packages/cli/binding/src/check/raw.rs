//! Client of the internal `vp check --raw` runner (`packages/cli/src/check/raw.ts`).
//!
//! The runner executes the planned steps in one Node.js process, so oxfmt and
//! oxlint share one import of Vite and the config loader. After each step it
//! writes a marker line with the step's exit code to both stdout and stderr,
//! which splits each stream into per-step output.

use std::{collections::BTreeMap, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStderr, ChildStdout},
};
use vp_error::Error;
use vt::ExitStatus;
use vt_path::AbsolutePathBuf;

use crate::cli::{
    CapturedCommandOutput, EnvMap, SubcommandResolver, exit_status_from, spawn_check_raw,
};

// Must match `STEP_MARKER` in packages/cli/src/check/raw.ts.
const STEP_MARKER: &[u8] = b"\0vp-check-raw:";

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum RawCheckTool {
    Fmt,
    Lint,
}

#[derive(Serialize)]
pub(super) struct RawCheckStep {
    tool: RawCheckTool,
    args: Vec<String>,
    env: BTreeMap<&'static str, &'static str>,
}

impl RawCheckStep {
    pub(super) fn fmt(args: Vec<String>) -> Self {
        Self { tool: RawCheckTool::Fmt, args, env: BTreeMap::new() }
    }

    pub(super) fn lint(args: Vec<String>) -> Self {
        let mut env = BTreeMap::new();
        // Capturing output hides the terminal from oxlint. Preserve colors only
        // when the parent supports them; the runner keeps an explicit
        // FORCE_COLOR value.
        if console::colors_enabled() {
            env.insert("FORCE_COLOR", "1");
        }
        Self { tool: RawCheckTool::Lint, args, env }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StepMarker {
    exit_code: u8,
}

pub(super) struct RawCheck {
    child: Child,
    stdout: BufReader<ChildStdout>,
    stderr: BufReader<ChildStderr>,
}

impl RawCheck {
    /// Spawn the runner. It runs `steps` in order and stops after the first
    /// failing one, so callers read steps until one fails.
    pub(super) async fn spawn(
        resolver: &SubcommandResolver,
        steps: &[RawCheckStep],
        envs: &Arc<EnvMap>,
        cwd: &AbsolutePathBuf,
    ) -> Result<Self, Error> {
        let mut child = spawn_check_raw(resolver, envs, cwd)?;
        let (Some(mut stdin), Some(stdout), Some(stderr)) =
            (child.stdin.take(), child.stdout.take(), child.stderr.take())
        else {
            return Err(Error::Anyhow(anyhow::anyhow!("check runner stdio is not piped")));
        };
        // The plan goes through stdin: with paths from lint-staged it can exceed
        // command-line length limits.
        let plan = serde_json::to_vec(steps).map_err(|e| Error::Anyhow(e.into()))?;
        stdin.write_all(&plan).await.map_err(|e| Error::Anyhow(e.into()))?;
        drop(stdin);

        Ok(Self { child, stdout: BufReader::new(stdout), stderr: BufReader::new(stderr) })
    }

    /// Wait for the next step to finish and return its output.
    pub(super) async fn next_step(&mut self) -> Result<CapturedCommandOutput, Error> {
        // Read both streams together so a full stderr pipe cannot block the runner.
        let (stdout, stderr) =
            tokio::try_join!(read_step(&mut self.stdout), read_step(&mut self.stderr))
                .map_err(Error::Anyhow)?;
        let status = match stdout.marker {
            Some(marker) => ExitStatus(marker.exit_code),
            // The runner exited before finishing the step, e.g. a tool crashed the process.
            None => match exit_status_from(
                self.child.wait().await.map_err(|e| Error::Anyhow(e.into()))?,
            ) {
                ExitStatus::SUCCESS => ExitStatus(1),
                status => status,
            },
        };

        Ok(CapturedCommandOutput {
            status,
            stdout: String::from_utf8_lossy(&stdout.output).into_owned(),
            stderr: String::from_utf8_lossy(&stderr.output).into_owned(),
        })
    }

    /// Wait for the runner to exit, then return the status of `vp check`.
    pub(super) async fn finish(mut self, status: ExitStatus) -> Result<ExitStatus, Error> {
        self.child.wait().await.map_err(|e| Error::Anyhow(e.into()))?;
        Ok(status)
    }
}

struct StepOutput {
    output: Vec<u8>,
    marker: Option<StepMarker>,
}

/// Read one step's output up to its marker, or to the end of the stream.
async fn read_step(reader: &mut (impl AsyncBufRead + Unpin)) -> anyhow::Result<StepOutput> {
    let mut output = Vec::new();
    let mut line = Vec::new();
    loop {
        line.clear();
        if reader.read_until(b'\n', &mut line).await? == 0 {
            return Ok(StepOutput { output, marker: None });
        }
        // Tool output may not end with a newline, so the marker can start mid-line.
        if let Some(start) = line.windows(STEP_MARKER.len()).position(|w| w == STEP_MARKER) {
            output.extend_from_slice(&line[..start]);
            let marker = serde_json::from_slice(&line[start + STEP_MARKER.len()..])?;
            return Ok(StepOutput { output, marker: Some(marker) });
        }
        output.extend_from_slice(&line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn read_step_splits_output_at_markers() {
        let mut reader: &[u8] = b"Checking formatting...\n\
            \0vp-check-raw:{\"tool\":\"fmt\",\"exitCode\":0}\n\
            Found 1 error.\0vp-check-raw:{\"tool\":\"lint\",\"exitCode\":1}\n";

        let fmt = read_step(&mut reader).await.unwrap();
        assert_eq!(fmt.output, b"Checking formatting...\n");
        assert_eq!(fmt.marker.unwrap().exit_code, 0);

        let lint = read_step(&mut reader).await.unwrap();
        assert_eq!(lint.output, b"Found 1 error.");
        assert_eq!(lint.marker.unwrap().exit_code, 1);

        let end = read_step(&mut reader).await.unwrap();
        assert!(end.output.is_empty());
        assert!(end.marker.is_none());
    }

    #[tokio::test]
    async fn read_step_keeps_output_of_an_unfinished_step() {
        let mut reader: &[u8] = b"panic: tool crashed\nno trailing newline";

        let step = read_step(&mut reader).await.unwrap();
        assert_eq!(step.output, b"panic: tool crashed\nno trailing newline");
        assert!(step.marker.is_none());
    }

    #[test]
    fn steps_serialize_to_the_runner_plan() {
        let plan = serde_json::to_string(&[RawCheckStep::fmt(vec!["--check".into()])]).unwrap();
        assert_eq!(plan, r#"[{"tool":"fmt","args":["--check"],"env":{}}]"#);
    }
}
