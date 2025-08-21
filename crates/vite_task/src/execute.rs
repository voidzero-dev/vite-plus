use std::{
    collections::hash_map::Entry,
    env::{join_paths, split_paths},
    ffi::OsStr,
    iter,
    path::{Component, Path},
    process::{ExitStatus, Stdio},
    sync::{Arc, Mutex},
};

use anyhow::Context;
use bincode::{Decode, Encode};
use fspy::{AccessMode, Spy, TrackedChild};

use futures_util::future::try_join4;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _};
use wax::Glob;
use wildmatch::WildMatch;

use crate::{
    Error,
    collections::{HashMap, HashSet},
    config::{ResolvedTask, ResolvedTaskCommand, ResolvedTaskConfig, TaskCommand},
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
    #[expect(dead_code)]
    pub exit_status: ExitStatus,
    pub path_reads: HashMap<Str, PathRead>,
    #[expect(dead_code)]
    pub path_writes: HashMap<Str, PathWrite>,
}

impl ExecutedTask {
    /// Filter path_reads for install command to optimize cache fingerprint.
    /// This filters out deep node_modules paths while keeping important config files.
    pub fn filter_install_paths(mut self) -> Self {
        let filtered_reads: HashMap<Str, PathRead> = self
            .path_reads
            .into_iter()
            .filter(|(path, path_read)| {
                let path = Path::new(path);
                // For paths ending with node_modules, only keep the top-level directory listing
                if path.ends_with("node_modules") {
                    tracing::trace!(
                        "keep path: {:?}, read_dir_entries: {:?}",
                        path,
                        path_read.read_dir_entries
                    );
                    return true;
                }

                // Ignore other paths that are in node_modules
                if path.components().any(|c| c == Component::Normal(OsStr::new("node_modules"))) {
                    tracing::trace!(
                        "ignore path: {:?}, read_dir_entries: {:?}",
                        path,
                        path_read.read_dir_entries
                    );
                    return false;
                }

                // Keep package.json, lock files, and config files (but only if not in node_modules)
                const CONFIG_FILES: &[&str] = &[
                    "package.json",
                    "package-lock.json",
                    ".npmrc",
                    // pnpm only
                    "pnpm-workspace.yaml",
                    "pnpm-lock.yaml",
                    ".pnpmfile.cjs",
                    // yarn only
                    "yarn.lock",
                    ".yarnrc",
                    ".yarnrc.yml",
                    "yarn.config.cjs",
                ];
                if CONFIG_FILES.iter().any(|file| path.ends_with(file)) {
                    tracing::trace!(
                        "keep path: {:?}, read_dir_entries: {:?}",
                        path,
                        path_read.read_dir_entries
                    );
                    return true;
                }

                // Ignore all other paths that are not in node_modules
                false
            })
            .collect();

        self.path_reads = filtered_reads;
        self
    }
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
    pub fn resolve(base_dir: &Path, task: &ResolvedTaskConfig) -> Result<Self, Error> {
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

    tracing::debug!(
        "read {} paths, wrote {} paths, {}",
        path_reads.len(),
        path_writes.len(),
        exit_status
    );

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
    use crate::collections::HashMap;
    use std::os::unix::process::ExitStatusExt;

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

        // Test edge cases
        assert!(!is_default_passthrough_env("VSCODE")); // Should not match without underscore
        assert!(!is_default_passthrough_env("DOCKER")); // Should not match without underscore
        assert!(!is_default_passthrough_env(""));
        assert!(!is_default_passthrough_env("TEST")); // Should not match any pattern
        assert!(!is_default_passthrough_env("CONFIG")); // Should not match any pattern
    }

    #[test]
    fn test_filter_install_paths_keeps_config_files() {
        let mut path_reads = HashMap::new();
        path_reads.insert("package.json".into(), PathRead { read_dir_entries: false });
        path_reads.insert("apps/web/package.json".into(), PathRead { read_dir_entries: false });
        path_reads.insert("yarn.lock".into(), PathRead { read_dir_entries: false });
        path_reads.insert(".npmrc".into(), PathRead { read_dir_entries: false });
        path_reads.insert(".yarnrc.yml".into(), PathRead { read_dir_entries: false });
        path_reads.insert("pnpm-workspace.yaml".into(), PathRead { read_dir_entries: false });

        let task = ExecutedTask {
            std_outputs: Arc::new([]),
            exit_status: std::process::ExitStatus::from_raw(0),
            path_reads,
            path_writes: HashMap::new(),
        };

        let filtered = task.filter_install_paths();

        // All config files should be kept
        assert!(filtered.path_reads.contains_key(&Str::from("package.json")));
        assert!(filtered.path_reads.contains_key(&Str::from("apps/web/package.json")));
        assert!(filtered.path_reads.contains_key(&Str::from("yarn.lock")));
        assert!(filtered.path_reads.contains_key(&Str::from(".npmrc")));
        assert!(filtered.path_reads.contains_key(&Str::from(".yarnrc.yml")));
        assert!(filtered.path_reads.contains_key(&Str::from("pnpm-workspace.yaml")));
    }

    #[test]
    fn test_filter_install_paths_keeps_top_level_node_modules() {
        let mut path_reads = HashMap::new();
        path_reads.insert("node_modules".into(), PathRead { read_dir_entries: true });
        path_reads.insert("node_modules/@types".into(), PathRead { read_dir_entries: true });
        path_reads.insert("node_modules/react".into(), PathRead { read_dir_entries: false });
        path_reads.insert("apps/web/node_modules".into(), PathRead { read_dir_entries: true });
        path_reads
            .insert("apps/web/node_modules/vite".into(), PathRead { read_dir_entries: false });

        let task = ExecutedTask {
            std_outputs: Arc::new([]),
            exit_status: std::process::ExitStatus::from_raw(0),
            path_reads,
            path_writes: HashMap::new(),
        };

        let filtered = task.filter_install_paths();

        // Top-level node_modules entries should be kept
        assert!(filtered.path_reads.contains_key(&Str::from("node_modules")));
        assert!(filtered.path_reads.contains_key(&Str::from("apps/web/node_modules")));

        // Deep node_modules entries should be filtered out
        assert!(!filtered.path_reads.contains_key(&Str::from("apps/web/node_modules/vite")));
        assert!(!filtered.path_reads.contains_key(&Str::from("node_modules/@types")));
        assert!(!filtered.path_reads.contains_key(&Str::from("node_modules/react")));
    }

    #[test]
    fn test_filter_install_paths_removes_deep_node_modules() {
        let mut path_reads = HashMap::new();
        path_reads
            .insert("node_modules/react/package.json".into(), PathRead { read_dir_entries: false });
        path_reads
            .insert("node_modules/react/lib/index.js".into(), PathRead { read_dir_entries: false });
        path_reads.insert(
            "node_modules/@types/node/index.d.ts".into(),
            PathRead { read_dir_entries: false },
        );
        path_reads.insert(
            "apps/web/node_modules/vite/bin/vite.js".into(),
            PathRead { read_dir_entries: false },
        );

        let task = ExecutedTask {
            std_outputs: Arc::new([]),
            exit_status: std::process::ExitStatus::from_raw(0),
            path_reads,
            path_writes: HashMap::new(),
        };

        let filtered = task.filter_install_paths();

        // Deep node_modules paths should be filtered out
        assert!(!filtered.path_reads.contains_key(&Str::from("node_modules/react/package.json")));
        assert!(!filtered.path_reads.contains_key(&Str::from("node_modules/react/lib/index.js")));
        assert!(
            !filtered.path_reads.contains_key(&Str::from("node_modules/@types/node/index.d.ts"))
        );
        assert!(
            !filtered.path_reads.contains_key(&Str::from("apps/web/node_modules/vite/bin/vite.js"))
        );
    }

    #[test]
    fn test_filter_install_paths_mixed_paths() {
        let mut path_reads = HashMap::new();
        // Config files
        path_reads.insert("package.json".into(), PathRead { read_dir_entries: false });
        path_reads.insert("pnpm-lock.yaml".into(), PathRead { read_dir_entries: false });
        // Top-level node_modules
        path_reads.insert("node_modules".into(), PathRead { read_dir_entries: true });
        path_reads.insert("node_modules/typescript".into(), PathRead { read_dir_entries: false });
        // Deep node_modules (should be filtered)
        path_reads.insert(
            "node_modules/typescript/lib/typescript.js".into(),
            PathRead { read_dir_entries: false },
        );
        path_reads.insert(
            "node_modules/typescript/package.json".into(),
            PathRead { read_dir_entries: false },
        );
        // Regular source files
        path_reads.insert("src/main.ts".into(), PathRead { read_dir_entries: false });
        path_reads.insert("tsconfig.json".into(), PathRead { read_dir_entries: false });

        let task = ExecutedTask {
            std_outputs: Arc::new([]),
            exit_status: std::process::ExitStatus::from_raw(0),
            path_reads,
            path_writes: HashMap::new(),
        };

        let filtered = task.filter_install_paths();

        // Check what should be kept
        assert!(filtered.path_reads.contains_key(&Str::from("package.json")));
        assert!(filtered.path_reads.contains_key(&Str::from("pnpm-lock.yaml")));
        assert!(filtered.path_reads.contains_key(&Str::from("node_modules")));

        // Check what should be filtered out
        assert!(
            !filtered
                .path_reads
                .contains_key(&Str::from("node_modules/typescript/lib/typescript.js"))
        );
        assert!(
            !filtered.path_reads.contains_key(&Str::from("node_modules/typescript/package.json"))
        );
        assert!(!filtered.path_reads.contains_key(&Str::from("src/main.ts")));
        assert!(!filtered.path_reads.contains_key(&Str::from("tsconfig.json")));

        // Should have 3 paths after filtering (down from 8)
        assert_eq!(filtered.path_reads.len(), 3);
    }
}
