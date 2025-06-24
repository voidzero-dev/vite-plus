use std::{
    collections::{HashMap, HashSet},
    env::{join_paths, split_paths},
    ffi::{OsStr, OsString},
    io::{self, Read, Write},
    iter,
    ops::DerefMut,
    path::Path,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};

use bincode::{Decode, Encode};

use compact_str::CompactStringExt;
use serde::{Deserialize, Serialize};
use wax::Glob;

use crate::{
    config::{ResolvedTask, TaskConfig},
    str::Str,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Encode, Decode, Serialize, Deserialize)]
pub enum OutputKind {
    StdOut,
    StdErr,
}

#[derive(Debug, Encode, Decode, Serialize, Deserialize)]
pub struct StdOutput {
    pub kind: OutputKind,
    pub content: Vec<u8>,
}

#[derive(Debug)]
pub struct ExecutedTask {
    pub std_outputs: Arc<[StdOutput]>,
    pub input_paths: HashSet<Arc<OsStr>>,
}

fn collect_std_outputs(
    outputs: &Mutex<Vec<StdOutput>>,
    mut stream: impl Read,
    kind: OutputKind,
) -> io::Result<()> {
    let mut buf = [0u8; 8192];
    let mut parent_output_handle: Box<dyn Write> = match kind {
        OutputKind::StdOut => Box::new(std::io::stdout().lock()),
        OutputKind::StdErr => Box::new(std::io::stderr().lock()),
    };
    loop {
        let n = stream.read(&mut buf)?;
        if n == 0 {
            return Ok(());
        }
        let content = &buf[..n];
        parent_output_handle.write_all(content)?;
        let mut outputs = outputs.lock().unwrap();
        let outputs = outputs.deref_mut();
        if let Some(last) = outputs.last_mut()
            && last.kind == kind
        {
            last.content.extend_from_slice(content);
        } else {
            outputs.push(StdOutput { kind, content: content.to_vec() });
        }
    }
}

#[derive(Debug)]
pub struct TaskEnvs {
    pub all_envs: HashMap<Str, Arc<OsStr>>,
    pub env_fingerprint: HashMap<Str, Option<Str>>,
}

impl TaskEnvs {
    pub fn resolve(task: &TaskConfig) -> anyhow::Result<Self> {
        // All envs that are passed to the task
        let mut all_envs: HashMap<Str, Arc<OsStr>> = std::env::vars_os()
            .filter_map(|(name, value)| {
                let Some(name) = name.to_str() else {
                    return None;
                };
                // TODO: glob
                // TODO: more default passthrough envs: https://github.com/vercel/turborepo/blob/26d309f073ca3ac054109ba0c29c7e230e7caac3/crates/turborepo-lib/src/task_hash.rs#L439
                if name == "PATH"
                    || task.envs.contains(name)
                    || task.pass_through_envs.contains(name)
                {
                    Some((Str::from(name), Arc::<OsStr>::from(value)))
                } else {
                    None
                }
            })
            .collect();

        let env_path =
            all_envs.entry("PATH".into()).or_insert_with(|| Arc::<OsStr>::from(OsStr::new("")));
        let paths = split_paths(env_path);
        let node_modules_bin = Path::new(&task.cwd).join("node_modules/.bin");
        *env_path = join_paths(iter::once(node_modules_bin).chain(paths))?.into();

        let mut env_fingerprint = HashMap::<Str, Option<Str>>::new();
        for name in &task.envs {
            let value = if let Some(value) = all_envs.get(name) {
                let Some(value) = value.to_str() else {
                    anyhow::bail!(
                        "the value of environment variable '{}' is not valid unicode: {:?}",
                        name,
                        value
                    );
                };
                Some(Str::from(value))
            } else {
                None
            };
            env_fingerprint.insert(name.clone(), value);
        }
        Ok(Self { all_envs, env_fingerprint })
    }
}

pub fn execute_task(task: &ResolvedTask, base_dir: &Path) -> anyhow::Result<ExecutedTask> {
    let mut child = if cfg!(windows) {
        let mut cmd = Command::new("cmd.exe");
        // https://github.com/nodejs/node/blob/dbd24b165128affb7468ca42f69edaf7e0d85a9a/lib/child_process.js#L633
        cmd.args(["/d", "/s", "/c"]);
        cmd
    } else {
        let mut cmd = Command::new("sh");
        cmd.args(["-c"]);
        cmd
    }
    .arg(&task.config.command)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .current_dir(base_dir.join(&task.config.cwd))
    .env_clear()
    .envs(&task.envs.all_envs)
    .spawn()?;

    let child_stdout = child.stdout.take().unwrap();
    let child_stderr = child.stderr.take().unwrap();

    let outputs = Mutex::new(Vec::<StdOutput>::new());

    std::thread::scope(|scope| {
        let stdout_collect_join_handle =
            scope.spawn(|| collect_std_outputs(&outputs, child_stdout, OutputKind::StdOut));
        let stderr_collect_join_handle =
            scope.spawn(|| collect_std_outputs(&outputs, child_stderr, OutputKind::StdErr));

        stdout_collect_join_handle.join().unwrap()?;
        stderr_collect_join_handle.join().unwrap()?;
        io::Result::Ok(())
    })?;

    let outputs = outputs.into_inner().unwrap();

    let input_paths = gather_inputs(&task, base_dir)?;

    Ok(ExecutedTask {
        std_outputs: outputs.into(),
        input_paths,
    })
}

fn gather_inputs(task: &ResolvedTask, base_dir: &Path) -> anyhow::Result<HashSet<Arc<OsStr>>> {
    let glob = format!("{{{}}}", itertools::Itertools::join(&mut task.config.inputs.iter(), ",")); // TODO: handle "," inside globs
    let glob = Glob::new(&glob)?;

    let mut paths: HashSet<Arc<OsStr>> = HashSet::new();
    for entry in glob.walk(base_dir.join(task.config.cwd.as_str())) {
        let entry = entry?;
        paths.insert(entry.into_path().into_os_string().into());
    }
    Ok(paths)
}
