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
use fspy::{AccessMode, Spy, TrackedChild};
use futures_util::future::try_join4;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _};
use vite_glob::GlobPatternSet;
use vite_str::Str;
use wax::Glob;

use crate::{
    Error,
    collections::{HashMap, HashSet},
    config::{ResolvedTask, ResolvedTaskCommand, ResolvedTaskConfig, TaskCommand},
    maybe_str::MaybeString,
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
    #[expect(dead_code)]
    pub exit_status: ExitStatus,
    pub path_reads: HashMap<Str, PathRead>,
    #[expect(dead_code)]
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

/// Environment variables for task execution.
///
/// # How Environment Variables Affect Caching
///
/// Vite-plus distinguishes between two types of environment variables:
///
/// 1. **Declared envs** (in task config's `envs` array):
///    - Explicitly declared as dependencies of the task
///    - Included in `envs_without_pass_through`
///    - Changes to these invalidate the cache
///    - Example: `NODE_ENV`, `API_URL`, `BUILD_MODE`
///
/// 2. **Pass-through envs** (in task config's `pass_through_envs` or defaults like PATH):
///    - Available to the task but don't affect caching
///    - Only in `all_envs`, NOT in `envs_without_pass_through`
///    - Changes to these don't invalidate cache
///    - Example: PATH, HOME, USER, CI
///
/// ## Cache Key Generation
/// - Only `envs_without_pass_through` is included in the cache key
/// - This ensures tasks are re-run when important envs change
/// - But allows cache reuse when only incidental envs change
///
/// ## Common Issues
/// - If a built-in resolver provides different envs, cache will be polluted
/// - Missing important envs from `envs` array = stale cache on env changes
/// - Including volatile envs in `envs` array = unnecessary cache misses
#[derive(Debug)]
pub struct TaskEnvs {
    /// All environment variables available to the task (declared + pass-through)
    pub all_envs: HashMap<Str, Arc<OsStr>>,
    /// Only declared envs that affect the cache key (excludes pass-through)
    pub envs_without_pass_through: HashMap<Str, Str>,
}

fn resolve_envs_with_patterns(patterns: &[&str]) -> Result<HashMap<Str, Arc<OsStr>>, Error> {
    let patterns = GlobPatternSet::new(patterns.iter().filter(|pattern| {
        if pattern.starts_with("!") {
            // FIXME: use better way to print warning log
            // Or parse and validate TaskConfig in command parsing phase
            tracing::warn!(
                "env pattern starts with '!' is not supported, will be ignored: {}",
                pattern
            );
            false
        } else {
            true
        }
    }))?;
    let envs: HashMap<Str, Arc<OsStr>> = std::env::vars_os()
        .filter_map(|(name, value)| {
            let Some(name) = name.to_str() else {
                return None;
            };

            if patterns.is_match(name) {
                Some((Str::from(name), Arc::<OsStr>::from(value)))
            } else {
                None
            }
        })
        .collect();
    Ok(envs)
}

fn create_all_envs_patterns<'a>(
    pass_through_envs: &'a HashSet<Str>,
    envs: &'a HashSet<Str>,
) -> Vec<&'a str> {
    // Exact matches for common environment variables
    // Referenced from Turborepo's implementation:
    // https://github.com/vercel/turborepo/blob/26d309f073ca3ac054109ba0c29c7e230e7caac3/crates/turborepo-lib/src/task_hash.rs#L439
    const DEFAULT_PASSTHROUGH_ENVS: &[&str] = &[
        // System and shell
        "HOME",
        "USER",
        "TZ",
        "LANG",
        "SHELL",
        "PWD",
        "PATH",
        // CI/CD environments
        "CI",
        // Node.js specific
        "NODE_OPTIONS",
        "COREPACK_HOME",
        "NPM_CONFIG_STORE_DIR",
        "PNPM_HOME",
        // Library paths
        "LD_LIBRARY_PATH",
        "DYLD_FALLBACK_LIBRARY_PATH",
        "LIBPATH",
        // Terminal/display
        "COLORTERM",
        "TERM",
        "TERM_PROGRAM",
        "DISPLAY",
        // Temporary directories
        "TMP",
        "TEMP",
        // Vercel specific
        "VERCEL",
        "VERCEL_*",
        "NEXT_*",
        "USE_OUTPUT_FOR_EDGE_FUNCTIONS",
        "NOW_BUILDER",
        // Windows specific
        "APPDATA",
        "PROGRAMDATA",
        "SYSTEMROOT",
        "SYSTEMDRIVE",
        "USERPROFILE",
        "HOMEDRIVE",
        "HOMEPATH",
        // IDE specific (exact matches)
        "ELECTRON_RUN_AS_NODE",
        "JB_INTERPRETER",
        "_JETBRAINS_TEST_RUNNER_RUN_SCOPE_TYPE",
        "JB_IDE_*",
        // VSCode specific
        "VSCODE_*",
        // Docker specific
        "DOCKER_*",
        "BUILDKIT_*",
        "COMPOSE_*",
    ];

    // All envs that are passed to the task
    let all_patterns: Vec<&str> = DEFAULT_PASSTHROUGH_ENVS
        .iter()
        .copied()
        .chain(pass_through_envs.iter().map(|s| s.as_ref()))
        .chain(envs.iter().map(|s| s.as_ref()))
        .collect();
    all_patterns
}

const SENSITIVE_PATTERNS: &[&str] = &[
    "*_KEY",
    "*_SECRET",
    "*_TOKEN",
    "*_PASSWORD",
    "*_PASS",
    "*_PWD",
    "*_CREDENTIAL*",
    "*_API_KEY",
    "*_PRIVATE_*",
    "AWS_*",
    "GITHUB_*",
    "NPM_*TOKEN",
    "DATABASE_URL",
    "MONGODB_URI",
    "REDIS_URL",
    "*_CERT*",
    // Exact matches for known sensitive names
    "PASSWORD",
    "SECRET",
    "TOKEN",
    "PRIVATE_KEY",
    "PUBLIC_KEY",
];

impl TaskEnvs {
    pub fn resolve(base_dir: &Path, task: &ResolvedTaskConfig) -> Result<Self, Error> {
        // All envs that are passed to the task
        let all_patterns =
            create_all_envs_patterns(&task.config.pass_through_envs, &task.config.envs);
        let mut all_envs = resolve_envs_with_patterns(&all_patterns)?;

        // envs need to calculate fingerprint
        let mut envs_without_pass_through = HashMap::<Str, Str>::new();
        if !task.config.envs.is_empty() {
            let envs_without_pass_through_patterns =
                GlobPatternSet::new(task.config.envs.iter().filter(|s| !s.starts_with("!")))?;
            let sensitive_patterns = GlobPatternSet::new(SENSITIVE_PATTERNS)?;
            for (name, value) in all_envs.iter() {
                if !envs_without_pass_through_patterns.is_match(name) {
                    continue;
                }
                let Some(value) = value.to_str() else {
                    return Err(Error::EnvValueIsNotValidUnicode {
                        key: name.to_string(),
                        value: value.to_os_string(),
                    });
                };
                let value: Str = if sensitive_patterns.is_match(name) {
                    let mut hasher = Sha256::new();
                    hasher.update(value.as_bytes());
                    format!("sha256:{:x}", hasher.finalize()).into()
                } else {
                    value.into()
                };
                envs_without_pass_through.insert(name.clone(), value);
            }
        }

        // Add node_modules/.bin to PATH
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

pub async fn execute_task(
    resolved_command: &ResolvedTaskCommand,
    base_dir: &Path,
) -> Result<ExecutedTask, Error> {
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
    let TrackedChild { tokio_child: mut child, accesses_future } = cmd.spawn().await?;

    let child_stdout = child.stdout.take().unwrap();
    let child_stderr = child.stderr.take().unwrap();

    let outputs = Mutex::new(Vec::<StdOutput>::new());

    let path_accesses_fut = async move {
        let path_accesses = accesses_future.await?;
        let mut path_reads = HashMap::<Str, PathRead>::new();
        let mut path_writes = HashMap::<Str, PathWrite>::new();
        for access in path_accesses.iter() {
            let path = access.path.to_cow_os_str();
            let path = Path::new(&path);
            let Ok(relative_path) = path.strip_prefix(base_dir) else {
                // ignore accesses outside the workspace
                continue;
            };
            let relative_path = relative_path.to_str().with_context(|| {
                format!("Non-utf8 relative path in the workspace: {relative_path:?}")
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

#[expect(dead_code)]
fn gather_inputs(task: &ResolvedTask, base_dir: &Path) -> Result<HashSet<Arc<OsStr>>, Error> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(unix)]
    fn test_task_envs_stable_ordering() {
        use crate::collections::HashSet;
        use crate::config::{ResolvedTaskConfig, TaskCommand, TaskConfig};
        use std::path::Path;

        // Create a task config with multiple envs in a HashSet
        let mut envs = HashSet::new();
        envs.insert("ZEBRA_VAR".into());
        envs.insert("ALPHA_VAR".into());
        envs.insert("MIDDLE_VAR".into());
        envs.insert("BETA_VAR".into());
        envs.insert("NOT_EXISTS_VAR".into());
        envs.insert("APP?_*".into());
        // will auto ignore ! prefix
        envs.insert("!APP*".into());

        let task_config = TaskConfig {
            command: TaskCommand::ShellScript("echo test".into()),
            cwd: ".".into(),
            cacheable: true,
            inputs: HashSet::new(),
            envs,
            pass_through_envs: HashSet::new(),
        };

        let resolved_task_config =
            ResolvedTaskConfig { config_dir: ".".into(), config: task_config };

        // Set up environment variables
        unsafe {
            std::env::set_var("ZEBRA_VAR", "zebra_value");
            std::env::set_var("ALPHA_VAR", "alpha_value");
            std::env::set_var("MIDDLE_VAR", "middle_value");
            std::env::set_var("BETA_VAR", "beta_value");
            // VSCode specific
            std::env::set_var("VSCODE_VAR", "vscode_value");
            std::env::set_var("APP1_NAME", "app1_value");
            std::env::set_var("APP2_NAME", "app2_value");
            std::env::set_var("APP1_PASSWORD", "app1_password");
        }

        // Resolve envs multiple times
        let result1 = TaskEnvs::resolve(Path::new("."), &resolved_task_config).unwrap();
        let result2 = TaskEnvs::resolve(Path::new("."), &resolved_task_config).unwrap();
        let result3 = TaskEnvs::resolve(Path::new("."), &resolved_task_config).unwrap();

        // Convert to sorted vecs for comparison
        let mut envs1: Vec<_> = result1.envs_without_pass_through.iter().collect();
        let mut envs2: Vec<_> = result2.envs_without_pass_through.iter().collect();
        let mut envs3: Vec<_> = result3.envs_without_pass_through.iter().collect();

        envs1.sort();
        envs2.sort();
        envs3.sort();

        // Verify all resolutions produce the same result
        assert_eq!(envs1, envs2);
        assert_eq!(envs2, envs3);

        // Verify all expected variables are present
        assert_eq!(envs1.len(), 7);
        assert!(envs1.iter().any(|(k, _)| k.as_str() == "ALPHA_VAR"));
        assert!(envs1.iter().any(|(k, _)| k.as_str() == "BETA_VAR"));
        assert!(envs1.iter().any(|(k, _)| k.as_str() == "MIDDLE_VAR"));
        assert!(envs1.iter().any(|(k, _)| k.as_str() == "ZEBRA_VAR"));
        assert!(envs1.iter().any(|(k, _)| k.as_str() == "APP1_NAME"));
        assert!(envs1.iter().any(|(k, _)| k.as_str() == "APP2_NAME"));
        assert!(envs1.iter().any(|(k, _)| k.as_str() == "APP1_PASSWORD"));

        // APP1_PASSWORD should be hashed
        let password = result1.envs_without_pass_through.get("APP1_PASSWORD").unwrap();
        assert_eq!(
            password,
            "sha256:17f1ef795d5663faa129f6fe3e5335e67ac7a701d1a70533a5f4b1635413a1aa"
        );

        // Verify default pass-through envs are present
        let all_envs = result1.all_envs;
        assert!(all_envs.contains_key("VSCODE_VAR"));
        assert!(all_envs.contains_key("PATH"));
        assert!(all_envs.contains_key("HOME"));
        assert!(all_envs.contains_key("APP1_NAME"));
        assert!(all_envs.contains_key("APP2_NAME"));
        assert!(all_envs.contains_key("APP1_PASSWORD"));

        // Clean up
        unsafe {
            std::env::remove_var("ZEBRA_VAR");
            std::env::remove_var("ALPHA_VAR");
            std::env::remove_var("MIDDLE_VAR");
            std::env::remove_var("BETA_VAR");
            std::env::remove_var("VSCODE_VAR");
            std::env::remove_var("APP1_NAME");
            std::env::remove_var("APP2_NAME");
            std::env::remove_var("APP1_PASSWORD");
        }
    }

    #[test]
    #[cfg(not(windows))]
    fn test_unix_env_case_sensitive() {
        use crate::collections::HashSet;
        use crate::config::{ResolvedTaskConfig, TaskCommand, TaskConfig};
        use std::path::Path;

        // Test that Unix environment variable matching is case-sensitive
        // Unix env vars are case-sensitive, so PATH and path are different

        // Create a task config with envs in different cases
        let mut envs = HashSet::new();
        envs.insert("TEST_VAR".into());
        envs.insert("test_var".into()); // Different variable on Unix
        envs.insert("Test_Var".into()); // Different variable on Unix

        let task_config = TaskConfig {
            command: TaskCommand::ShellScript("echo test".into()),
            cwd: ".".into(),
            cacheable: true,
            inputs: HashSet::new(),
            envs,
            pass_through_envs: HashSet::new(),
        };

        let resolved_task_config =
            ResolvedTaskConfig { config_dir: ".".into(), config: task_config };

        // Set up environment variables with different cases
        unsafe {
            std::env::set_var("TEST_VAR", "uppercase");
            std::env::set_var("test_var", "lowercase");
            std::env::set_var("Test_Var", "mixed");
        }

        // Resolve envs
        let result = TaskEnvs::resolve(Path::new("."), &resolved_task_config).unwrap();
        let envs_without_pass_through = result.envs_without_pass_through;

        // On Unix, all three should be treated as separate variables
        assert_eq!(
            envs_without_pass_through.len(),
            3,
            "Unix should treat different cases as different variables"
        );

        assert_eq!(
            envs_without_pass_through.get("TEST_VAR").map(|s| s.as_str()),
            Some("uppercase")
        );
        assert_eq!(
            envs_without_pass_through.get("test_var").map(|s| s.as_str()),
            Some("lowercase")
        );
        assert_eq!(envs_without_pass_through.get("Test_Var").map(|s| s.as_str()), Some("mixed"));

        // Clean up
        unsafe {
            std::env::remove_var("TEST_VAR");
            std::env::remove_var("test_var");
            std::env::remove_var("Test_Var");
        }
    }

    #[test]
    #[cfg(windows)]
    fn test_windows_env_case_insensitive() {
        use crate::collections::HashSet;
        use crate::config::{ResolvedTaskConfig, TaskCommand, TaskConfig};
        use std::path::Path;

        // Create a task config with multiple envs in a HashSet
        let mut envs = HashSet::new();
        envs.insert("ZEBRA_VAR".into());
        envs.insert("ALPHA_VAR".into());
        envs.insert("MIDDLE_VAR".into());
        envs.insert("BETA_VAR".into());
        envs.insert("NOT_EXISTS_VAR".into());
        envs.insert("APP?_*".into());

        let task_config = TaskConfig {
            command: TaskCommand::ShellScript("echo test".into()),
            cwd: ".".into(),
            cacheable: true,
            inputs: HashSet::new(),
            envs,
            pass_through_envs: HashSet::new(),
        };

        let resolved_task_config =
            ResolvedTaskConfig { config_dir: ".".into(), config: task_config };

        // Set up environment variables
        unsafe {
            std::env::set_var("ZEBRA_VAR", "zebra_value");
            std::env::set_var("ALPHA_VAR", "alpha_value");
            std::env::set_var("MIDDLE_VAR", "middle_value");
            std::env::set_var("BETA_VAR", "beta_value");
            // VSCode specific
            std::env::set_var("VSCODE_VAR", "vscode_value");
            std::env::set_var("app1_name", "app1_value");
            std::env::set_var("app2_name", "app2_value");
        }

        // Resolve envs multiple times
        let result1 = TaskEnvs::resolve(Path::new("."), &resolved_task_config).unwrap();
        let result2 = TaskEnvs::resolve(Path::new("."), &resolved_task_config).unwrap();
        let result3 = TaskEnvs::resolve(Path::new("."), &resolved_task_config).unwrap();

        // Convert to sorted vecs for comparison
        let mut envs1: Vec<_> = result1.envs_without_pass_through.iter().collect();
        let mut envs2: Vec<_> = result2.envs_without_pass_through.iter().collect();
        let mut envs3: Vec<_> = result3.envs_without_pass_through.iter().collect();

        envs1.sort();
        envs2.sort();
        envs3.sort();

        // Verify all resolutions produce the same result
        assert_eq!(envs1, envs2);
        assert_eq!(envs2, envs3);

        // Verify all expected variables are present
        assert_eq!(envs1.len(), 6);
        assert!(envs1.iter().any(|(k, _)| k.as_str() == "ALPHA_VAR"));
        assert!(envs1.iter().any(|(k, _)| k.as_str() == "BETA_VAR"));
        assert!(envs1.iter().any(|(k, _)| k.as_str() == "MIDDLE_VAR"));
        assert!(envs1.iter().any(|(k, _)| k.as_str() == "ZEBRA_VAR"));
        assert!(envs1.iter().any(|(k, _)| k.as_str() == "app1_name"));
        assert!(envs1.iter().any(|(k, _)| k.as_str() == "app1_name"));

        // Verify default pass-through envs are present
        let all_envs = result1.all_envs;
        assert!(all_envs.contains_key("VSCODE_VAR"));
        assert!(all_envs.contains_key("Path"));
        assert!(all_envs.contains_key("app1_name"));
        assert!(all_envs.contains_key("app1_name"));

        // Clean up
        unsafe {
            std::env::remove_var("ZEBRA_VAR");
            std::env::remove_var("ALPHA_VAR");
            std::env::remove_var("MIDDLE_VAR");
            std::env::remove_var("BETA_VAR");
            std::env::remove_var("VSCODE_VAR");
            std::env::remove_var("app1_name");
            std::env::remove_var("app1_name");
        }
    }
}
