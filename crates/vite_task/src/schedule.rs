use std::{
    collections::HashSet,
    env::{join_paths, split_paths},
    ffi::OsStr,
    io::{self, Read, Write},
    iter,
    ops::DerefMut,
    path::Path,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};

use bincode::{Decode, Encode};

use petgraph::{algo::toposort, stable_graph::StableDiGraph};
use serde::{Deserialize, Serialize};
use wax::Glob;

use crate::{
    cache::{CacheMiss, CachedTask, TaskCache},
    config::{NamedTaskConfig, TaskConfig, Workspace},
    fs::FileSystem,
};

#[derive(Debug)]
pub struct ExecutionPlan {
    steps: Vec<NamedTaskConfig>,
    // node_indices: Vec<NodeIndex>,
    // task_graph: Graph<TaskNode, ()>,
}

impl ExecutionPlan {
    pub fn plan(mut task_graph: StableDiGraph<NamedTaskConfig, ()>) -> anyhow::Result<Self> {
        // TODO: parallel
        let node_indices = match toposort(&task_graph, None) {
            Ok(ok) => ok,
            Err(err) => anyhow::bail!("Circular depedency found in the task graph: {:?}", err),
        };
        let steps = node_indices.into_iter().map(|id| task_graph.remove_node(id).unwrap());
        Ok(ExecutionPlan { steps: steps.collect() })
    }

    pub fn execute(self, workspace: &mut Workspace) -> anyhow::Result<()> {
        for step in self.steps {
            println!("------- {} -------", &step.name);
            let command = step.config.command.clone();
            let (cache_miss, execute_or_replay) = get_cached_or_execute(
                step,
                &mut workspace.task_cache,
                &workspace.fs,
                &workspace.dir,
            )?;
            match cache_miss {
                Some(CacheMiss::NotFound) => {
                    println!("Cache Not Found, executing task");
                    println!("> {}", command);
                }
                Some(CacheMiss::FingerprintMismatch(mismatch)) => {
                    println!("{}, executing task", mismatch);
                    println!("> {}", command);
                }
                None => {
                    println!("Cache hit, replaying previously executed task");
                }
            }
            execute_or_replay()?;
            println!();
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Encode, Decode, Serialize, Deserialize)]
pub enum OutputKind {
    StdOut,
    StdErr,
}

#[derive(Debug, Encode, Decode, Serialize, Deserialize)]
pub struct StdOutput {
    kind: OutputKind,
    content: Vec<u8>,
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

fn get_cached_or_execute<'a>(
    task: NamedTaskConfig,
    cache: &'a mut TaskCache,
    fs: &'a impl FileSystem,
    base_dir: &'a Path,
) -> anyhow::Result<(Option<CacheMiss>, Box<dyn FnOnce() -> anyhow::Result<()> + 'a>)> {
    Ok(match cache.try_hit(&task, fs, base_dir)? {
        Ok(cache_task) => (
            None,
            Box::new({
                // replay
                let std_outputs = Arc::clone(&cache_task.std_outputs);
                move || {
                    let mut stdout = std::io::stdout().lock();
                    let mut stderr = std::io::stderr().lock();
                    for ouput_section in std_outputs.as_ref() {
                        match ouput_section.kind {
                            OutputKind::StdOut => stdout.write_all(&ouput_section.content)?,
                            OutputKind::StdErr => stderr.write_all(&ouput_section.content)?,
                        }
                    }
                    anyhow::Ok(())
                }
            }),
        ),
        Err(cache_miss) => (
            Some(cache_miss),
            Box::new(move || {
                let executed_task = execute_task(&task.config)?;
                let cached_task = CachedTask::create(&task.config, executed_task, fs, base_dir)?;
                cache.update(task.name.clone(), cached_task)?;
                anyhow::Ok(())
            }),
        ),
    })
}

fn execute_task(task: &TaskConfig) -> anyhow::Result<ExecutedTask> {
    let env_path = std::env::var_os("PATH").unwrap_or_default();
    let paths = split_paths(&env_path);
    let node_modules_bin = Path::new(&task.cwd).join("node_modules/.bin");

    let paths = iter::once(node_modules_bin).chain(paths);
    let env_path = join_paths(paths)?;

    let mut child = Command::new("/bin/sh")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .args(["-c", task.command.as_ref()])
        .current_dir(&task.cwd)
        .env("PATH", env_path)
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

    let input_paths = gather_inputs(&task)?;

    Ok(ExecutedTask { std_outputs: outputs.into(), input_paths })
}

fn gather_inputs(task: &TaskConfig) -> anyhow::Result<HashSet<Arc<OsStr>>> {
    let glob = format!("{{{}}}", task.inputs.join(",")); // TODO: handle "," inside globs
    let glob = Glob::new(&glob)?;

    let mut paths: HashSet<Arc<OsStr>> = HashSet::new();
    for entry in glob.walk(task.cwd.as_str()) {
        let entry = entry?;
        paths.insert(entry.into_path().into_os_string().into());
    }
    Ok(paths)
}
