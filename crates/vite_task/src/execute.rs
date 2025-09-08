use std::{
    collections::hash_map::Entry,
    env::{join_paths, split_paths},
    ffi::OsStr,
    iter,
    process::{ExitStatus, Stdio},
    sync::{Arc, Mutex},
};

use anyhow::Context;
use bincode::{Decode, Encode};
use fspy::{AccessMode, Spy, TrackedChild};
use supports_color::{Stream, on};

use futures_util::future::try_join4;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _};
use vite_path::{AbsolutePath, RelativePath, RelativePathBuf};
use vite_str::Str;
use wax::Glob;
use wildmatch::WildMatch;

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
    pub path_reads: HashMap<RelativePathBuf, PathRead>,
    #[expect(dead_code)]
    pub path_writes: HashMap<RelativePathBuf, PathWrite>,
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

/// Checks if a string matches a wildcard pattern using the wildmatch crate.
/// Supports * as a wildcard that matches any number of characters.
fn matches_wildcard_pattern(text: &str, pattern: &str) -> bool {
    // Use WildMatch for pattern matching which supports glob-like patterns
    WildMatch::new(pattern).matches(text)
}

/// Checks if an environment variable should be passed through by default.
/// Based on Turborepo's implementation for commonly needed environment variables.
fn is_default_passthrough_env(name: &str) -> bool {
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
        "FORCE_COLOR",
        // Temporary directories
        "TMP",
        "TEMP",
        // Vercel specific
        "VERCEL",
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
    ];

    // Check exact matches first
    if DEFAULT_PASSTHROUGH_ENVS.contains(&name) {
        return true;
    }

    // Wildcard patterns for common development tools and platforms
    const WILDCARD_PATTERNS: &[&str] =
        &["VSCODE_*", "DOCKER_*", "BUILDKIT_*", "COMPOSE_*", "JB_IDE_*", "VERCEL_*", "NEXT_*"];

    // Check wildcard patterns
    for pattern in WILDCARD_PATTERNS {
        if matches_wildcard_pattern(name, pattern) {
            return true;
        }
    }

    false
}

impl TaskEnvs {
    pub fn resolve(base_dir: &AbsolutePath, task: &ResolvedTaskConfig) -> Result<Self, Error> {
        // All envs that are passed to the task
        let mut all_envs: HashMap<Str, Arc<OsStr>> = std::env::vars_os()
            .filter_map(|(name, value)| {
                let Some(name) = name.to_str() else {
                    return None;
                };

                // Check if this env var should be passed through
                if is_default_passthrough_env(name)
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
                    key: name.clone(),
                    value: value.to_os_string(),
                });
            };
            envs_without_pass_through.insert(name.clone(), value.into());
        }

        let env_path =
            all_envs.entry("PATH".into()).or_insert_with(|| Arc::<OsStr>::from(OsStr::new("")));
        let paths = split_paths(env_path);

        let node_modules_bin_paths = [
            base_dir
                .join(&task.config.cwd)
                .join("node_modules/.bin")
                .into_path_buf(),
            base_dir
                .join(&task.config_dir)
                .join("node_modules/.bin")
                .into_path_buf(),
        ];
        *env_path = join_paths(node_modules_bin_paths.into_iter().chain(paths))?.into();

        // Automatically add FORCE_COLOR environment variable if not already set
        // This enables color output in subprocesses when color is supported
        // TODO: will remove this temporarily until we have a better solution
        if !all_envs.contains_key("FORCE_COLOR") {
            if let Some(support) = on(Stream::Stdout) {
                let force_color_value = if support.has_16m {
                    "3" // True color (16 million colors)
                } else if support.has_256 {
                    "2" // 256 colors
                } else if support.has_basic {
                    "1" // Basic ANSI colors
                } else {
                    "0" // No color support
                };
                all_envs.insert(
                    "FORCE_COLOR".into(),
                    Arc::<OsStr>::from(OsStr::new(force_color_value)),
                );
            }
        }

        Ok(Self { all_envs, envs_without_pass_through })
    }
}

pub async fn execute_task(
    resolved_command: &ResolvedTaskCommand,
    base_dir: &AbsolutePath,
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
        let mut path_reads = HashMap::<RelativePathBuf, PathRead>::new();
        let mut path_writes = HashMap::<RelativePathBuf, PathWrite>::new();
        for access in path_accesses.iter() {
            let path = access.path.to_cow_os_str();
            let Some(path) = AbsolutePath::new(&path) else {
                continue;
            };
            let Some(relative_path) = path.strip_prefix(base_dir)? else {
                // ignore accesses outside the workspace
                continue;
            };

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
fn gather_inputs(
    task: &ResolvedTask,
    base_dir: &AbsolutePath,
) -> Result<HashSet<Arc<OsStr>>, Error> {
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
    fn test_matches_wildcard_pattern() {
        // Test exact matches (no wildcards)
        assert!(matches_wildcard_pattern("PATH", "PATH"));
        assert!(!matches_wildcard_pattern("PATH", "HOME"));

        // Test prefix wildcards (existing behavior)
        assert!(matches_wildcard_pattern("VSCODE_PID", "VSCODE_*"));
        assert!(matches_wildcard_pattern("DOCKER_HOST", "DOCKER_*"));
        assert!(!matches_wildcard_pattern("VSCODE", "VSCODE_*"));

        // Test suffix wildcards
        assert!(matches_wildcard_pattern("MY_CONFIG", "*_CONFIG"));
        assert!(matches_wildcard_pattern("APP_CONFIG", "*_CONFIG"));
        assert!(!matches_wildcard_pattern("CONFIG", "*_CONFIG"));

        // Test middle wildcards (the key new feature)
        assert!(matches_wildcard_pattern("MY_TEST_VAR", "*_TEST_*"));
        assert!(matches_wildcard_pattern("APP_TEST_CONFIG", "*_TEST_*"));
        assert!(matches_wildcard_pattern("SOME_CONFIG_VALUE", "*_CONFIG_*"));
        assert!(!matches_wildcard_pattern("MY_TEST", "*_TEST_*"));
        assert!(!matches_wildcard_pattern("TEST_VAR", "*_TEST_*"));

        // Test multiple wildcards
        assert!(matches_wildcard_pattern("A_B_C_D", "*_B_*_D"));
        assert!(matches_wildcard_pattern("X_B_Y_D", "*_B_*_D"));
        assert!(!matches_wildcard_pattern("A_B_C", "*_B_*_D"));

        // Test edge cases
        assert!(matches_wildcard_pattern("", "*"));
        assert!(matches_wildcard_pattern("anything", "*"));
        assert!(matches_wildcard_pattern("", ""));
        assert!(!matches_wildcard_pattern("something", ""));
    }

    #[test]
    fn test_is_default_passthrough_env() {
        // Test exact matches
        assert!(is_default_passthrough_env("PATH"));
        assert!(is_default_passthrough_env("HOME"));
        assert!(is_default_passthrough_env("USER"));
        assert!(is_default_passthrough_env("CI"));
        assert!(is_default_passthrough_env("NODE_OPTIONS"));
        assert!(is_default_passthrough_env("SHELL"));
        assert!(is_default_passthrough_env("LANG"));
        assert!(is_default_passthrough_env("TZ"));

        // Test existing prefix patterns
        assert!(is_default_passthrough_env("VSCODE_PID"));
        assert!(is_default_passthrough_env("VSCODE_GIT_ASKPASS_MAIN"));
        assert!(is_default_passthrough_env("DOCKER_HOST"));
        assert!(is_default_passthrough_env("DOCKER_CONFIG"));
        assert!(is_default_passthrough_env("BUILDKIT_PROGRESS"));
        assert!(is_default_passthrough_env("COMPOSE_FILE"));
        assert!(is_default_passthrough_env("JB_IDE_PROJECT_DIR"));
        assert!(is_default_passthrough_env("VERCEL_URL"));
        assert!(is_default_passthrough_env("NEXT_PUBLIC_API_URL"));

        // Test patterns that should not match anymore (since we removed the example patterns)
        assert!(!is_default_passthrough_env("MY_TEST_VARIABLE"));
        assert!(!is_default_passthrough_env("APP_CONFIG_FILE"));
        assert!(!is_default_passthrough_env("SOME_DEBUG_FLAG"));

        // Test variables that should NOT be passed through
        assert!(!is_default_passthrough_env("SECRET_KEY"));
        assert!(!is_default_passthrough_env("API_TOKEN"));
        assert!(!is_default_passthrough_env("CUSTOM_VAR"));
        assert!(!is_default_passthrough_env("RANDOM_ENV"));
        assert!(!is_default_passthrough_env("MY_SECRET"));

        // Test FORCE_COLOR is a passthrough env
        assert!(is_default_passthrough_env("FORCE_COLOR"));

        // Test edge cases
        assert!(!is_default_passthrough_env("VSCODE")); // Should not match without underscore
        assert!(!is_default_passthrough_env("DOCKER")); // Should not match without underscore
        assert!(!is_default_passthrough_env(""));
        assert!(!is_default_passthrough_env("TEST")); // Should not match any pattern
        assert!(!is_default_passthrough_env("CONFIG")); // Should not match any pattern
    }

    #[test]
    fn test_task_envs_stable_ordering() {
        use crate::collections::HashSet;
        use crate::config::{ResolvedTaskConfig, TaskCommand, TaskConfig};

        // Create a task config with multiple envs in a HashSet
        let mut envs = HashSet::new();
        envs.insert("ZEBRA_VAR".into());
        envs.insert("ALPHA_VAR".into());
        envs.insert("MIDDLE_VAR".into());
        envs.insert("BETA_VAR".into());

        let task_config = TaskConfig {
            command: TaskCommand::ShellScript("echo test".into()),
            cwd: RelativePathBuf::default(),
            cacheable: true,
            inputs: HashSet::new(),
            envs,
            pass_through_envs: HashSet::new(),
        };

        let resolved_task_config =
            ResolvedTaskConfig { config_dir: RelativePathBuf::default(), config: task_config };

        // Set up environment variables
        unsafe {
            std::env::set_var("ZEBRA_VAR", "zebra_value");
            std::env::set_var("ALPHA_VAR", "alpha_value");
            std::env::set_var("MIDDLE_VAR", "middle_value");
            std::env::set_var("BETA_VAR", "beta_value");
        }

        let base_dir = if cfg!(windows) {
            AbsolutePath::new("C:\\workspace").unwrap()
        } else {
            AbsolutePath::new("/workspace").unwrap()
        };
        // Resolve envs multiple times
        let result1 = TaskEnvs::resolve(base_dir, &resolved_task_config).unwrap();
        let result2 = TaskEnvs::resolve(base_dir, &resolved_task_config).unwrap();
        let result3 = TaskEnvs::resolve(base_dir, &resolved_task_config).unwrap();

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
        assert_eq!(envs1.len(), 4);
        assert!(envs1.iter().any(|(k, _)| k.as_str() == "ALPHA_VAR"));
        assert!(envs1.iter().any(|(k, _)| k.as_str() == "BETA_VAR"));
        assert!(envs1.iter().any(|(k, _)| k.as_str() == "MIDDLE_VAR"));
        assert!(envs1.iter().any(|(k, _)| k.as_str() == "ZEBRA_VAR"));

        // Clean up
        unsafe {
            std::env::remove_var("ZEBRA_VAR");
            std::env::remove_var("ALPHA_VAR");
            std::env::remove_var("MIDDLE_VAR");
            std::env::remove_var("BETA_VAR");
        }
    }

    #[test]
    fn test_force_color_auto_detection() {
        use crate::collections::HashSet;
        use crate::config::{ResolvedTaskConfig, TaskCommand, TaskConfig};
        use std::path::Path;

        let task_config = TaskConfig {
            command: TaskCommand::ShellScript("echo test".into()),
            cwd: ".".into(),
            cacheable: true,
            inputs: HashSet::new(),
            envs: HashSet::new(),
            pass_through_envs: HashSet::new(),
        };

        let resolved_task_config =
            ResolvedTaskConfig { config_dir: ".".into(), config: task_config };

        // Test when FORCE_COLOR is not already set
        unsafe {
            std::env::remove_var("FORCE_COLOR");
        }

        let result = TaskEnvs::resolve(Path::new("."), &resolved_task_config).unwrap();

        // FORCE_COLOR should be automatically added if color is supported
        // Note: This test might vary based on the test environment
        let force_color_present = result.all_envs.contains_key("FORCE_COLOR");
        if force_color_present {
            let force_color_value = result.all_envs.get("FORCE_COLOR").unwrap();
            let force_color_str = force_color_value.to_str().unwrap();
            // Should be a valid FORCE_COLOR level
            assert!(matches!(force_color_str, "0" | "1" | "2" | "3"));
        }

        // Test when FORCE_COLOR is already set - should not be overridden
        unsafe {
            std::env::set_var("FORCE_COLOR", "2");
        }

        let result2 = TaskEnvs::resolve(Path::new("."), &resolved_task_config).unwrap();

        // Should contain the original FORCE_COLOR value
        assert!(result2.all_envs.contains_key("FORCE_COLOR"));
        let force_color_value = result2.all_envs.get("FORCE_COLOR").unwrap();
        assert_eq!(force_color_value.to_str().unwrap(), "2");

        // FORCE_COLOR should not be in envs_without_pass_through since it's a passthrough env
        assert!(!result2.envs_without_pass_through.contains_key("FORCE_COLOR"));

        // Clean up
        unsafe {
            std::env::remove_var("FORCE_COLOR");
        }
    }
}
