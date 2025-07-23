use std::{
    collections::hash_map::Entry,
    env::{join_paths, split_paths},
    ffi::OsStr,
    iter,
    path::Path,
    process::{ExitStatus, Stdio},
    sync::{Arc, Mutex},
};

use anyhow::Context;
use bincode::{Decode, Encode};
use fspy::{AccessMode, Spy};

use futures_util::future::try_join4;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _};
use wax::Glob;

use crate::{
    collections::{HashMap, HashSet},
    config::{ResolvedTask, ResolvedTaskConfig, TaskCommand},
    error::Error,
    maybe_str::MaybeString,
    str::Str,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Encode, Decode, Serialize, Deserialize)]
pub enum OutputKind {
    StdOut,
    StdErr,
}

#[derive(Debug, Encode, Decode, Serialize)]
pub struct StdOutput {
    pub kind: OutputKind,
    pub content: MaybeString,
}

#[derive(Debug, Clone, Copy)]
pub struct PathRead {
    pub read_dir_entries: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct PathWrite;

/// Contains info that is available after executing the task
#[derive(Debug)]
pub struct ExecutedTask {
    pub std_outputs: Arc<[StdOutput]>,
    pub exit_status: ExitStatus,
    pub path_reads: HashMap<Str, PathRead>,
    pub path_writes: HashMap<Str, PathWrite>,
}

/// Collects stdout/stderr into `outputs` and at the same time writes them to the real stdout/stderr
async fn collect_std_outputs(
    outputs: &Mutex<Vec<StdOutput>>,
    mut stream: impl AsyncRead + Unpin,
    kind: OutputKind,
) -> Result<(), Error> {
    let mut buf = [0u8; 8192];
    let mut parent_output_handle: Box<dyn AsyncWrite + Unpin + Send> = match kind {
        OutputKind::StdOut => Box::new(tokio::io::stdout()),
        OutputKind::StdErr => Box::new(tokio::io::stderr()),
    };
    loop {
        let n = stream.read(&mut buf).await?;
        if n == 0 {
            return Ok(());
        }
        let content = &buf[..n];
        parent_output_handle.write_all(content).await?;
        let mut outputs = outputs.lock().unwrap();
        if let Some(last) = outputs.last_mut()
            && last.kind == kind
        {
            last.content.extend_from_slice(content);
        } else {
            outputs.push(StdOutput { kind, content: content.to_vec().into() });
        }
    }
}

#[derive(Debug)]
pub struct TaskEnvs {
    pub all_envs: HashMap<Str, Arc<OsStr>>,
    pub envs_without_pass_through: HashMap<Str, Str>,
}

impl TaskEnvs {
    pub fn resolve(base_dir: &Path, task: &ResolvedTaskConfig) -> Result<Self, Error> {
        // All envs that are passed to the task
        let mut all_envs: HashMap<Str, Arc<OsStr>> = std::env::vars_os()
            .filter_map(|(name, value)| {
                let Some(name) = name.to_str() else {
                    return None;
                };
                // TODO: glob
                // TODO: more default passthrough envs: https://github.com/vercel/turborepo/blob/26d309f073ca3ac054109ba0c29c7e230e7caac3/crates/turborepo-lib/src/task_hash.rs#L439
                if name == "PATH"
                    || task.config.envs.contains(name)
                    || task.config.pass_through_envs.contains(name)
                {
                    Some((Str::from(name), Arc::<OsStr>::from(value)))
                } else {
                    None
                }
            })
            .collect();

        let mut envs_without_pass_through = HashMap::<Str, Str>::new();
        for name in &task.config.envs {
            let Some(value) = all_envs.get(name) else {
                continue;
            };
            let Some(value) = value.to_str() else {
                return Err(Error::EnvValueIsNotValidUnicode {
                    key: name.to_string(),
                    value: value.to_os_string(),
                });
            };
            envs_without_pass_through.insert(name.clone(), value.into());
        }

        let env_path =
            all_envs.entry("PATH".into()).or_insert_with(|| Arc::<OsStr>::from(OsStr::new("")));
        let paths = split_paths(env_path);
        let node_modules_bin = base_dir.join(&task.config.cwd).join("node_modules/.bin");
        *env_path = join_paths(
            iter::once(node_modules_bin)
                .chain(iter::once(base_dir.join(&task.config_dir).join("node_modules/.bin")))
                .chain(paths),
        )?
        .into();

        Ok(Self { all_envs, envs_without_pass_through })
    }
}

pub async fn execute_task(task: &ResolvedTask, base_dir: &Path) -> Result<ExecutedTask, Error> {
    let resolved_command = &task.resolved_command;
    let spy = Spy::global()?;

    let mut cmd = match &resolved_command.fingerprint.command {
        TaskCommand::ShellScript(script) => {
            let mut cmd = if cfg!(windows) {
                let mut cmd = spy.new_command("cmd.exe");
                // https://github.com/nodejs/node/blob/dbd24b165128affb7468ca42f69edaf7e0d85a9a/lib/child_process.js#L633
                cmd.args(["/d", "/s", "/c"]);
                cmd
            } else {
                let mut cmd = spy.new_command("sh");
                cmd.args(["-c"]);
                cmd
            };
            cmd.arg(script);
            cmd.envs(&resolved_command.all_envs);
            cmd
        }
        TaskCommand::Parsed(task_parsed_command) => {
            let mut cmd = spy.new_command(&task_parsed_command.program);
            cmd.args(&task_parsed_command.args);
            cmd.envs(&resolved_command.all_envs);
            cmd.envs(&task_parsed_command.envs);
            cmd
        }
    };

    cmd.current_dir(base_dir.join(&resolved_command.fingerprint.cwd))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let (mut child, mut path_accesses) = cmd.spawn().await?;

    let child_stdout = child.stdout.take().unwrap();
    let child_stderr = child.stderr.take().unwrap();

    let outputs = Mutex::new(Vec::<StdOutput>::new());

    let path_accesses_fut = async move {
        let mut path_reads = HashMap::<Str, PathRead>::new();
        let mut path_writes = HashMap::<Str, PathWrite>::new();
        let mut buf = Vec::<u8>::new();
        while let Some(access) = path_accesses.next(&mut buf).await? {
            let path = access.path.to_cow_os_str();
            let path = Path::new(&path);
            let Ok(relative_path) = path.strip_prefix(base_dir) else {
                // ignore accesses outside the workspace
                continue;
            };
            let relative_path = relative_path.to_str().with_context(|| {
                format!("Non-utf8 relative path in the workspace: {:?}", relative_path)
            })?;
            let relative_path = Str::from(relative_path);
            match access.mode {
                AccessMode::Read => {
                    path_reads.entry(relative_path).or_insert(PathRead { read_dir_entries: false });
                }
                AccessMode::Write => {
                    path_writes.insert(relative_path, PathWrite);
                }
                AccessMode::ReadWrite => {
                    path_reads
                        .entry(relative_path.clone())
                        .or_insert(PathRead { read_dir_entries: false });
                    path_writes.insert(relative_path, PathWrite);
                }
                AccessMode::ReadDir => match path_reads.entry(relative_path) {
                    Entry::Occupied(mut occupied) => occupied.get_mut().read_dir_entries = true,
                    Entry::Vacant(vacant) => {
                        vacant.insert(PathRead { read_dir_entries: true });
                    }
                },
            }
        }
        Ok::<_, Error>((path_reads, path_writes))
    };

    let ((), (), (path_reads, path_writes), exit_status) = try_join4(
        collect_std_outputs(&outputs, child_stdout, OutputKind::StdOut),
        collect_std_outputs(&outputs, child_stderr, OutputKind::StdErr),
        path_accesses_fut,
        async move { Ok(child.wait().await?) },
    )
    .await?;

    let outputs = outputs.into_inner().unwrap();

    // let input_paths = gather_inputs(task, base_dir)?;

    Ok(ExecutedTask { std_outputs: outputs.into(), exit_status, path_reads, path_writes })
}

fn gather_inputs(task: &ResolvedTask, base_dir: &Path) -> anyhow::Result<HashSet<Arc<OsStr>>> {
    // Task inferring to be implemented here
    let inputs = &task.resolved_config.config.inputs;
    if inputs.is_empty() {
        return Ok(HashSet::new());
    }
    let glob = format!("{{{}}}", itertools::Itertools::join(&mut inputs.iter(), ",")); // TODO: handle "," inside globs
    let glob = Glob::new(&glob)?;

    let mut paths: HashSet<Arc<OsStr>> = HashSet::new();
    for entry in glob.walk(base_dir.join(&task.resolved_config.config_dir)) {
        let entry = entry?;
        paths.insert(entry.into_path().into_os_string().into());
    }
    Ok(paths)
}
