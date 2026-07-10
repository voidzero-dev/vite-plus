//! PTY-based snapshot test suite for the vp CLI.
//!
//! Fixtures live in `tests/cli_snapshots/fixtures/<name>/`; each declares
//! cases in `snapshots.toml` (see `rfcs/interactive-snapshot-tests.md`).
//! Every step runs in a real pseudo-terminal backed by a vt100 emulator;
//! interactive steps synchronize on window-title milestones emitted by the child.
//! Snapshots are Markdown files compared with real pass/fail semantics
//! (`UPDATE_SNAPSHOTS=1` accepts changes).
//!
//! The runner deliberately uses std types: it is a dev-only test binary,
//! matching the conventions of vite-task's `e2e_snapshots` runner it is
//! ported from.
#![expect(clippy::disallowed_types, reason = "standalone test runner uses std types")]
#![expect(clippy::disallowed_macros, reason = "standalone test runner uses std macros")]
#![expect(clippy::disallowed_methods, reason = "standalone test runner uses std methods")]

mod flavor;
mod redact;

use std::{
    collections::BTreeMap,
    ffi::OsString,
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

use cp_r::CopyOptions;
use flavor::{Flavor, FlavorRuntime};
use pty_terminal_test::{CommandBuilder, ScreenSize, TestTerminal};
use redact::redact_output;

/// Default per-step timeout. Windows CI needs longer due to process startup
/// overhead and slower I/O. Individual steps override via `timeout` (ms).
const STEP_TIMEOUT: Duration =
    if cfg!(windows) { Duration::from_secs(60) } else { Duration::from_secs(50) };

/// Screen size for the PTY terminal. Large enough to avoid line wrapping.
const SCREEN_SIZE: ScreenSize = ScreenSize { rows: 500, cols: 500 };

const VP_BINARY_NAME: &str = if cfg!(windows) { "vp.exe" } else { "vp" };

/// Raw serde shape for a step: bare argv array or full table.
#[derive(serde::Deserialize, Debug)]
#[serde(untagged)]
enum StepDe {
    /// Shorthand: `["vp", "check"]`
    Simple(Vec<String>),
    /// Detailed: `{ argv = ["vp", "create"], interactions = [...], ... }`
    Detailed(StepTable),
}

fn default_true() -> bool {
    true
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct StepTable {
    argv: Vec<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    comment: Option<String>,
    #[serde(default)]
    envs: Vec<(String, String)>,
    #[serde(default)]
    interactions: Vec<Interaction>,
    #[serde(default, rename = "formatted-snapshot")]
    formatted_snapshot: bool,
    #[serde(default)]
    timeout: Option<u64>,
    #[serde(default = "default_true")]
    snapshot: bool,
    #[serde(default = "default_true")]
    tty: bool,
    #[serde(default, rename = "continue-on-failure")]
    continue_on_failure: bool,
}

/// One executable step, normalized at deserialization: the argv shorthand
/// is a table with every option at its default, so the runner body deals
/// with exactly one shape. Field semantics:
/// - `cwd`: per-step working dir relative to the staged fixture root,
///   defaulting to the case-level `cwd`.
/// - `comment`: rendered under the step heading in the snapshot.
/// - `formatted_snapshot`: render the screen with inline ANSI escapes made
///   visible (`\x1b[…m`) so colour/style attributes are asserted.
/// - `timeout`: per-step override in ms (default `STEP_TIMEOUT`).
/// - `snapshot = false`: omit the screen while the step succeeds; failures
///   always keep their output.
/// - `tty = false`: piped stdio instead of a PTY, for non-TTY assertions;
///   interactions require a PTY.
/// - `continue_on_failure`: on failure, execution skips past the next step
///   marked true (the line boundary in migrated fixtures) and resumes;
///   without one ahead, the case stops (shell-like `&&`).
#[derive(serde::Deserialize, Debug)]
#[serde(from = "StepDe")]
struct Step {
    argv: Vec<String>,
    cwd: Option<String>,
    comment: Option<String>,
    envs: Vec<(String, String)>,
    interactions: Vec<Interaction>,
    formatted_snapshot: bool,
    timeout: Option<u64>,
    snapshot: bool,
    tty: bool,
    continue_on_failure: bool,
}

impl From<StepDe> for Step {
    fn from(de: StepDe) -> Self {
        let table = match de {
            StepDe::Simple(argv) => StepTable {
                argv,
                cwd: None,
                comment: None,
                envs: Vec::new(),
                interactions: Vec::new(),
                formatted_snapshot: false,
                timeout: None,
                snapshot: true,
                tty: true,
                continue_on_failure: false,
            },
            StepDe::Detailed(table) => table,
        };
        Self {
            argv: table.argv,
            cwd: table.cwd,
            comment: table.comment,
            envs: table.envs,
            interactions: table.interactions,
            formatted_snapshot: table.formatted_snapshot,
            timeout: table.timeout,
            snapshot: table.snapshot,
            tty: table.tty,
            continue_on_failure: table.continue_on_failure,
        }
    }
}

impl Step {
    /// Shell-escaped command line including any env-var prefix and non-default
    /// cwd, without the comment (e.g. `cd packages/a && MY_ENV=1 vp check`).
    fn display_command_line(&self, default_cwd: &str) -> String {
        let argv_str = self
            .argv
            .iter()
            .map(|s| {
                if s.contains(|c: char| c.is_whitespace() || c == '"') {
                    shell_escape::escape(s.as_str().into()).into_owned()
                } else {
                    s.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        let mut command = String::new();
        for (k, v) in &self.envs {
            command.push_str(&format!("{k}={v} "));
        }
        command.push_str(&argv_str);

        let cwd = self.cwd.as_deref().unwrap_or(default_cwd);
        if cwd == default_cwd {
            command
        } else {
            let cwd = if cwd.is_empty() { "." } else { cwd };
            format!("cd {} && {command}", shell_escape::escape(cwd.into()))
        }
    }

    fn timeout(&self, default: Duration) -> Duration {
        self.timeout.map_or(default, Duration::from_millis)
    }
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(untagged)]
enum Interaction {
    ExpectMilestone(ExpectMilestoneInteraction),
    Write(WriteInteraction),
    WriteLine(WriteLineInteraction),
    WriteKey(WriteKeyInteraction),
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
struct ExpectMilestoneInteraction {
    #[serde(rename = "expect-milestone")]
    expect_milestone: String,
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
struct WriteInteraction {
    write: String,
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
struct WriteLineInteraction {
    #[serde(rename = "write-line")]
    write_line: String,
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
struct WriteKeyInteraction {
    #[serde(rename = "write-key")]
    write_key: WriteKey,
}

#[derive(serde::Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
enum WriteKey {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Escape,
    Space,
    Tab,
    Backspace,
    CtrlC,
}

impl WriteKey {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::Left => "left",
            Self::Right => "right",
            Self::Enter => "enter",
            Self::Escape => "escape",
            Self::Space => "space",
            Self::Tab => "tab",
            Self::Backspace => "backspace",
            Self::CtrlC => "ctrl-c",
        }
    }

    const fn bytes(self) -> &'static [u8] {
        match self {
            Self::Up => b"\x1b[A",
            Self::Down => b"\x1b[B",
            Self::Left => b"\x1b[D",
            Self::Right => b"\x1b[C",
            Self::Enter => b"\r",
            Self::Escape => b"\x1b",
            Self::Space => b" ",
            Self::Tab => b"\t",
            Self::Backspace => b"\x7f",
            Self::CtrlC => b"\x03",
        }
    }
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(untagged)]
enum FlavorSpec {
    One(Flavor),
    Many(Vec<Flavor>),
}

impl FlavorSpec {
    fn flavors(&self) -> Vec<Flavor> {
        match self {
            Self::One(flavor) => vec![*flavor],
            Self::Many(flavors) => {
                assert!(!flavors.is_empty(), "`vp` must name at least one flavor");
                flavors.clone()
            }
        }
    }

    const fn is_multi(&self) -> bool {
        matches!(self, Self::Many(_))
    }
}

#[derive(serde::Deserialize, Debug)]
#[serde(untagged)]
enum PlatformFilter {
    Os(String),
    Detailed { os: String, libc: Option<String> },
}

impl PlatformFilter {
    fn matches_current(&self) -> bool {
        let (os, libc) = match self {
            Self::Os(os) => (os.as_str(), None),
            Self::Detailed { os, libc } => (os.as_str(), libc.as_deref()),
        };
        let os_matches = match os {
            "windows" => cfg!(windows),
            "linux" => cfg!(target_os = "linux"),
            "macos" => cfg!(target_os = "macos"),
            other => panic!("unknown skip-platforms os '{other}'"),
        };
        let libc_matches = match libc {
            None => true,
            Some("musl") => cfg!(target_env = "musl"),
            Some("glibc") => cfg!(target_os = "linux") && !cfg!(target_env = "musl"),
            Some(other) => panic!("unknown skip-platforms libc '{other}'"),
        };
        os_matches && libc_matches
    }
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct Case {
    name: String,
    /// Which vp flavor(s) run this case: `"local"`, `"global"`, or a list for
    /// parity cases. The list form registers one trial and one snapshot per
    /// flavor.
    vp: FlavorSpec,
    /// Free-form description rendered under the H1 heading of the snapshot.
    #[serde(default)]
    comment: Option<String>,
    #[serde(default)]
    cwd: String,
    /// Exclude-list of platforms this case does not run on.
    #[serde(default, rename = "skip-platforms")]
    skip_platforms: Vec<PlatformFilter>,
    /// Marks the trial `#[ignore]` (runnable with `cargo test -- --ignored`).
    #[serde(default)]
    ignore: bool,
    /// Run this case in isolation: nothing else runs while it does (the suite
    /// otherwise runs cases in parallel). Set for signal-sensitive flows (e.g.
    /// watch modes) that concurrent PTY activity would perturb; ctrl-c cases
    /// are detected automatically (see `case_needs_isolation`) and need not
    /// set this.
    #[serde(default)]
    serial: bool,
    /// Serve the packed checkout packages through the local npm registry.
    #[serde(default, rename = "local-registry")]
    local_registry: bool,
    /// Seed the case's `VP_HOME` with an already-provisioned managed JS
    /// runtime (see `flavor::js_runtime_seed_dir`). Default true; runtime
    /// provisioning tests set false to start from a genuinely empty home.
    #[serde(default = "default_true", rename = "seed-runtime")]
    seed_runtime: bool,
    /// Expose the run-root node_modules as the workspace's parent-dir
    /// node_modules for fixtures that address the linked checkout packages by path (`node
    /// ../node_modules/vite-plus/bin/oxlint`) rather than by specifier
    /// through Node's upward walk.
    #[serde(default, rename = "link-node-modules")]
    link_node_modules: bool,
    /// Case-wide environment additions on top of the runner baseline.
    #[serde(default)]
    env: BTreeMap<String, String>,
    /// Baseline environment variables to remove for this case.
    #[serde(default, rename = "unset-env")]
    unset_env: Vec<String>,
    steps: Vec<Step>,
    /// Cleanup steps: executed after the case, never snapshotted.
    #[serde(default)]
    after: Vec<Step>,
}

#[derive(serde::Deserialize, Default)]
struct SnapshotsFile {
    #[serde(rename = "case", default)]
    cases: Vec<Case>,
}

/// Fixture folder names and `[[case]].name` values must be made of
/// `[A-Za-z0-9_]` only so trial names round-trip through shell filters
/// and snapshot filenames don't carry whitespace or special characters.
fn assert_identifier_like(kind: &str, value: &str) {
    assert!(
        !value.is_empty() && value.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'),
        "{kind} '{value}' must contain only ASCII letters, digits, and '_'"
    );
}

fn load_snapshots_file(fixture_path: &Path) -> SnapshotsFile {
    let cases_toml_path = fixture_path.join("snapshots.toml");
    match std::fs::read_to_string(&cases_toml_path) {
        Ok(content) => toml::from_str(&content)
            .unwrap_or_else(|err| panic!("failed to parse {}: {err}", cases_toml_path.display())),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => SnapshotsFile::default(),
        Err(err) => {
            panic!("failed to read {}: {err}", cases_toml_path.display());
        }
    }
}

enum TerminationState {
    Exited(i64),
    TimedOut,
}

/// Render the byte stream produced by `screen_contents_formatted` into a
/// snapshot-friendly string: newlines stay literal, SGR escapes and other
/// bytes outside printable ASCII come out as `\xNN`, `\t`, etc.
fn render_formatted_screen(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        match b {
            b'\n' => out.push('\n'),
            _ => out.extend(std::ascii::escape_default(b).map(char::from)),
        }
    }
    out
}

/// Append a fenced markdown block containing `body`.
fn push_fenced_block(out: &mut String, body: &str) {
    let trimmed = body.trim_end_matches(['\n', ' ', '\t']);
    out.push_str("```\n");
    if !trimmed.is_empty() {
        out.push_str(trimmed);
        out.push('\n');
    }
    out.push_str("```\n");
}

/// Per-case isolated state directories: `HOME`, `VP_HOME`, and the npm global
/// prefix all live under one disposable root so nothing leaks between cases
/// or into the developer's real environment.
struct CaseHome {
    home: PathBuf,
}

struct CaseInstall {
    path_env: OsString,
    tool_dirs: Vec<PathBuf>,
    vpt: PathBuf,
}

impl CaseInstall {
    /// Resolve a step program from the case-owned Vite+ installation. `vpt`
    /// stays runner-owned so a fixture cannot shadow the assertion helper.
    fn resolve_program(
        &self,
        program: &str,
        case_path: &std::ffi::OsStr,
        cwd: &Path,
    ) -> Result<PathBuf, String> {
        if program == "vpt" {
            return Ok(self.vpt.clone());
        }

        // An explicit `./`-prefixed program runs a file the case itself
        // produced inside the staged workspace (a packed executable); the
        // prefix keeps bare names on the PATH rule below. The prefix is
        // dropped from the joined path: a `/./` component would survive into
        // output that records the executable's own path (`vp env setup`
        // writes shim targets from it).
        if let Some(rel) = program.strip_prefix("./") {
            let candidate = cwd.join(rel);
            if candidate.is_file() {
                return Ok(candidate);
            }
            return Err(format!("`{program}` does not exist relative to the step cwd"));
        }

        let found = which::which_in(program, Some(case_path), cwd)
            .map_err(|e| format!("`{program}` not found on the case PATH: {e}"))?;
        // Git is a fixture dependency in real create/migrate flows, so steps may
        // invoke the system installation directly as the sole PATH exception.
        if program == "git" || self.tool_dirs.iter().any(|dir| found.starts_with(dir)) {
            return Ok(found);
        }
        Err(format!("`{program}` resolved outside the case-owned tool dirs: {}", found.display()))
    }
}

impl CaseHome {
    fn provision(root: &Path, seed_runtime: bool) -> Self {
        let this = Self { home: root.join("home") };
        let vp_home = this.vp_home();
        std::fs::create_dir_all(&vp_home).unwrap();
        std::fs::create_dir_all(root.join("npm-global/lib")).unwrap();
        // Best-effort: if linking fails, the case downloads the runtime.
        if seed_runtime && let Some(seed) = flavor::js_runtime_seed_dir() {
            flavor::link_dir(&seed, &vp_home.join(flavor::JS_RUNTIME_DIR));
        }
        this
    }

    fn provision_vite_plus(
        &self,
        flavor: Flavor,
        runtime: &FlavorRuntime,
    ) -> Result<CaseInstall, String> {
        let current_bin = self.vp_home().join("current").join("bin");
        std::fs::create_dir_all(&current_bin)
            .map_err(|e| format!("failed to create current/bin dir: {e}"))?;

        let vp = current_bin.join(VP_BINARY_NAME);
        flavor::install_file(&vp, &runtime.global_vp, "global vp")?;

        #[cfg(windows)]
        {
            let shim = runtime
                .global_vp
                .parent()
                .ok_or("global vp source has no parent dir")?
                .join("vp-shim.exe");
            if !shim.is_file() {
                return Err(format!(
                    "global vp trampoline template not found at {}; run `cargo build -p vite_trampoline`",
                    shim.display()
                ));
            }
            flavor::install_file(&current_bin.join("vp-shim.exe"), &shim, "global vp-shim.exe")?;
        }

        let package_dir = self.vp_home().join("current").join("node_modules").join("vite-plus");
        Self::install_case_package(&runtime.cli_package_dir, &package_dir)?;
        let local_bin_dir = local_package_bin_dir(&package_dir);
        #[cfg(windows)]
        if flavor == Flavor::Local {
            self.write_local_package_cmd_shims(&package_dir, &local_bin_dir)?;
        }
        self.run_env_setup(&vp)?;

        let vp_bin_dir = self.vp_home().join("bin");
        let mut tool_dirs = match flavor {
            Flavor::Global => vec![vp_bin_dir],
            Flavor::Local => vec![local_bin_dir, vp_bin_dir],
        };
        let mut path_dirs = vec![runtime.runner_bin_dir.clone()];
        path_dirs.extend(tool_dirs.iter().cloned());
        // The whole case root is case-owned for resolution (not PATH): a
        // fixture that runs `vp env setup` against an isolated
        // `VP_HOME=${workspace}/home` then invokes the shims it created
        // through a per-step PATH prefix.
        if let Some(case_root) = self.home.parent() {
            tool_dirs.push(case_root.to_path_buf());
        }

        Ok(CaseInstall {
            path_env: compose_path_env(&path_dirs),
            tool_dirs,
            vpt: runtime.vpt.clone(),
        })
    }

    fn install_case_package(source: &Path, package_dir: &Path) -> Result<(), String> {
        let parent = package_dir.parent().ok_or("case package dir has no parent")?;
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create case package parent: {e}"))?;

        #[cfg(windows)]
        {
            junction::create(source, package_dir)
                .map_err(|e| format!("failed to junction case vite-plus package: {e}"))?;
        }

        #[cfg(not(windows))]
        {
            flavor::link_dir(source, package_dir);
            if package_dir.join("package.json").is_file() {
                return Ok(());
            }
            CopyOptions::new()
                .copy_tree(source, package_dir)
                .map_err(|e| format!("failed to copy case vite-plus package: {e}"))?;
        }
        Ok(())
    }

    #[cfg(windows)]
    fn write_local_package_cmd_shims(
        &self,
        package_dir: &Path,
        shim_dir: &Path,
    ) -> Result<(), String> {
        let bin_dir = package_dir.join("bin");
        std::fs::create_dir_all(shim_dir)
            .map_err(|e| format!("failed to create local package shim dir: {e}"))?;
        for name in ["vp", "vpr", "oxfmt", "oxlint"] {
            let script = bin_dir.join(name);
            if !script.is_file() {
                continue;
            }
            let content = format!(
                "@echo off\r\nnode \"{}\" %*\r\nexit /b %ERRORLEVEL%\r\n",
                script.display()
            );
            std::fs::write(shim_dir.join(format!("{name}.cmd")), content)
                .map_err(|e| format!("failed to write local package {name}.cmd: {e}"))?;
        }
        Ok(())
    }

    fn run_env_setup(&self, vp: &Path) -> Result<(), String> {
        let env = self.base_env(compose_path_env(&[]));
        let output = std::process::Command::new(vp)
            .args(["env", "setup", "--refresh"])
            .env_clear()
            .envs(&env)
            .output()
            .map_err(|e| format!("failed to run `vp env setup`: {e}"))?;
        if output.status.success() {
            return Ok(());
        }

        Err(format!(
            "`vp env setup` failed with status {}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
    }

    fn base_env(&self, path_env: OsString) -> BTreeMap<String, OsString> {
        let mut env = BTreeMap::new();
        env.insert("PATH".into(), path_env);
        // xterm-256color keeps anstream from stripping the OSC 8 milestone
        // sequences used by test steps. It is harmless during env setup.
        env.insert("TERM".into(), "xterm-256color".into());
        env.insert("VP_CLI_TEST".into(), "1".into());
        env.insert("NODE_NO_WARNINGS".into(), "1".into());
        env.insert("VP_HOME".into(), self.vp_home().into_os_string());
        if cfg!(windows) {
            env.insert("USERPROFILE".into(), self.home.clone().into_os_string());
            env.insert(
                "PATHEXT".into(),
                ".COM;.EXE;.BAT;.CMD;.VBS;.VBE;.JS;.JSE;.WSF;.WSH;.MSC".into(),
            );
            for name in [
                "TMP",
                "TEMP",
                "APPDATA",
                "LOCALAPPDATA",
                "PROGRAMDATA",
                "HOMEDRIVE",
                "HOMEPATH",
                "WINDIR",
                "SYSTEMROOT",
                "SYSTEMDRIVE",
                "ProgramFiles",
                "ProgramFiles(x86)",
            ] {
                if let Some(value) = std::env::var_os(name) {
                    env.insert(name.into(), value);
                }
            }
        } else {
            env.insert("HOME".into(), self.home.clone().into_os_string());
        }
        env
    }

    fn vp_home(&self) -> PathBuf {
        self.home.join(flavor::VP_HOME_DIR)
    }

    fn npm_prefix(&self) -> PathBuf {
        self.home.parent().unwrap().join("npm-global")
    }

    /// The machine-specific directories this case owns, paired with their
    /// snapshot labels. Owned by the type that creates the directories so a
    /// new case-owned dir cannot be added without deciding its redaction:
    /// the npm prefix is a sibling of `home`, which the `<home>` pair never
    /// matches, and it leaked raw temp paths into snapshots until it was
    /// paired here.
    fn redaction_paths(&self) -> [(String, &'static str); 3] {
        let root = self.home.parent().unwrap();
        [
            (self.home.to_str().unwrap().to_owned(), "<home>"),
            (self.npm_prefix().to_str().unwrap().to_owned(), "<npm-prefix>"),
            // Last, so the specific dirs above win: fixtures may create
            // siblings of the workspace (`vpt mkdir -p ../test-lib`), whose
            // paths tools then echo back (a yarn portal resolution).
            (root.to_str().unwrap().to_owned(), "<case>"),
        ]
    }
}

fn local_package_bin_dir(package_dir: &Path) -> PathBuf {
    if cfg!(windows) {
        return package_dir.parent().unwrap().join(".vite-plus-bin");
    }
    package_dir.join("bin")
}

fn compose_path_env(tool_dirs: &[PathBuf]) -> OsString {
    let mut entries = tool_dirs.to_vec();
    if cfg!(windows) {
        if let Some(path) = std::env::var_os("PATH") {
            entries.extend(std::env::split_paths(&path));
        }
    } else {
        for dir in ["/usr/bin", "/bin", "/usr/sbin", "/sbin"] {
            entries.push(PathBuf::from(dir));
        }
    }
    std::env::join_paths(entries).unwrap()
}

/// The environment every step starts from. Deliberately small and fully
/// controlled: no ambient variables survive except through the composed PATH.
/// Notably absent: `CI` and `NO_COLOR`; the PTY makes real interactive
/// behaviour the default, and grid rendering strips styling from snapshots.
fn baseline_env(case_home: &CaseHome, install: &CaseInstall) -> BTreeMap<String, OsString> {
    let mut env = case_home.base_env(install.path_env.clone());
    env.insert("VP_EMIT_MILESTONES".into(), "1".into());
    // Legacy-runner parity: `vp migrate` fixtures skip real dependency
    // installs (slow, network-bound). Cases that want real installs unset
    // this via `unset-env`.
    env.insert("VP_SKIP_INSTALL".into(), "1".into());
    env.insert("NPM_CONFIG_PREFIX".into(), case_home.npm_prefix().into_os_string());
    for (key, value) in [
        ("GIT_AUTHOR_NAME", "vite-plus-test"),
        ("GIT_AUTHOR_EMAIL", "test@vite-plus.invalid"),
        ("GIT_COMMITTER_NAME", "vite-plus-test"),
        ("GIT_COMMITTER_EMAIL", "test@vite-plus.invalid"),
    ] {
        env.insert(key.into(), value.into());
    }
    env
}

/// Runs a step with piped stdio (no PTY) for `tty = false` cases. Output is
/// stdout followed by stderr; interleaving is intentionally not modelled.
fn run_step_piped(
    program: &Path,
    args: &[String],
    envs: &BTreeMap<String, OsString>,
    cwd: &Path,
    timeout: Duration,
) -> (TerminationState, String) {
    use std::process::{Command, Stdio};

    let mut cmd = Command::new(program);
    cmd.args(args)
        .env_clear()
        .envs(envs)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    group_leader(&mut cmd);
    let mut child =
        cmd.spawn().unwrap_or_else(|e| panic!("failed to spawn {}: {e}", program.display()));

    let mut stdout_pipe = child.stdout.take().unwrap();
    let mut stderr_pipe = child.stderr.take().unwrap();
    let stdout_thread = std::thread::spawn(move || {
        use std::io::Read as _;
        let mut buf = String::new();
        let _ = stdout_pipe.read_to_string(&mut buf);
        buf
    });
    let stderr_thread = std::thread::spawn(move || {
        use std::io::Read as _;
        let mut buf = String::new();
        let _ = stderr_pipe.read_to_string(&mut buf);
        buf
    });

    let status = wait_with_deadline(&mut child, timeout);

    let mut output = stdout_thread.join().unwrap();
    output.push_str(&stderr_thread.join().unwrap());
    match status {
        Some(status) => (TerminationState::Exited(i64::from(status.code().unwrap_or(-1))), output),
        None => (TerminationState::TimedOut, output),
    }
}

/// Marks the command a process-group leader on Unix so a timeout can kill
/// descendants too (Windows relies on taskkill's tree flag instead).
fn group_leader(cmd: &mut std::process::Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        cmd.process_group(0);
    }
    #[cfg(not(unix))]
    let _ = cmd;
}

/// Polls the child until exit or deadline; on deadline the whole tree is
/// killed so pipe-holding descendants die too. `None` means timed out.
fn wait_with_deadline(
    child: &mut std::process::Child,
    timeout: Duration,
) -> Option<std::process::ExitStatus> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Err(_) => return None,
            Ok(None) if std::time::Instant::now() >= deadline => {
                kill_step_tree(child);
                let _ = child.wait();
                return None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
        }
    }
}

/// Effective env, cwd, and resolved program for one step; shared by the
/// main and cleanup loops so their semantics cannot drift.
/// Expands `${NAME}` references in a step env value. `${workspace}` resolves
/// to the step's working directory and any other name to the case env, so a
/// fixture can express the shell forms `VP_HOME="$(pwd)/home"` and
/// `PATH="$(pwd)/home/bin:$PATH"` without a shell. Unknown names stay
/// verbatim, like vpt's argument expansion.
fn expand_env_value(value: &str, cwd: &Path, case_env: &BTreeMap<String, OsString>) -> OsString {
    let mut out = OsString::new();
    let mut rest = value;
    while let Some(start) = rest.find("${") {
        let (before, from_ref) = rest.split_at(start);
        out.push(before);
        let Some(end) = from_ref.find('}') else {
            rest = from_ref;
            break;
        };
        let name = &from_ref[2..end];
        if name == "workspace" {
            out.push(cwd.as_os_str());
        } else if let Some(resolved) = case_env.get(name) {
            out.push(resolved);
        } else {
            out.push(&from_ref[..=end]);
        }
        rest = &from_ref[end + 1..];
    }
    out.push(rest);
    out
}

fn step_context<'a>(
    step: &Step,
    case_env: &'a BTreeMap<String, OsString>,
    case_path: &OsString,
    stage: &Path,
    case_cwd: &str,
    install: &CaseInstall,
) -> Result<(std::borrow::Cow<'a, BTreeMap<String, OsString>>, PathBuf, PathBuf), String> {
    use std::borrow::Cow;
    assert!(!step.argv.is_empty(), "step argv must not be empty");
    // The empty relative dir maps to the stage itself rather than through
    // `join("")`, whose trailing separator would leak into `${workspace}`
    // expansions (`<stage>//home`).
    let rel_cwd = step.cwd.as_deref().unwrap_or(case_cwd);
    let cwd = if rel_cwd.is_empty() { stage.to_path_buf() } else { stage.join(rel_cwd) };
    // Most steps add no env of their own; borrow the case env then.
    let env: Cow<'a, BTreeMap<String, OsString>> = if step.envs.is_empty() {
        Cow::Borrowed(case_env)
    } else {
        let mut env = case_env.clone();
        for (k, v) in &step.envs {
            // References resolve against the case env, never a sibling step
            // env, so the entries stay order-independent.
            env.insert(k.clone(), expand_env_value(v, &cwd, case_env));
        }
        Cow::Owned(env)
    };
    // Resolution honors a per-step PATH override and runs from the step cwd
    // (relative PATH entries resolve as the child would see them), so shim
    // and custom-prefix steps run exactly the child's tool.
    let path = env.get("PATH").cloned().unwrap_or_else(|| case_path.clone());
    let program = install.resolve_program(&step.argv[0], &path, &cwd)?;
    Ok((env, cwd, program))
}

/// Kills a piped step and every descendant so pipe readers unblock. The
/// child was spawned as a process-group leader on Unix; Windows uses
/// taskkill's tree flag.
fn kill_step_tree(child: &mut std::process::Child) {
    #[cfg(unix)]
    let _ =
        std::process::Command::new("kill").args(["-KILL", &format!("-{}", child.id())]).output();
    #[cfg(windows)]
    let _ = std::process::Command::new("taskkill")
        .args(["/PID", &child.id().to_string(), "/T", "/F"])
        .output();
    let _ = child.kill();
}

/// A per-case local npm registry (`local-npm-registry.ts --serve`). Held for
/// the case's lifetime; dropping it tears the server down and removes the
/// throwaway yarn/bun caches it created.
struct RegistryHandle {
    child: std::process::Child,
    /// The per-run `YARN_GLOBAL_FOLDER` / `BUN_INSTALL_CACHE_DIR` the server
    /// created. Its own SIGTERM handler removes them on Unix, but the Windows
    /// teardown force-kills the process so that handler never runs; removing
    /// them here keeps yarn/bun install cases from leaking caches on every
    /// platform.
    cache_dirs: Vec<PathBuf>,
}

impl Drop for RegistryHandle {
    fn drop(&mut self) {
        // Graceful first: SIGTERM the server so its handler removes the
        // throwaway yarn/bun caches. Keep this scoped to the direct child:
        // process-group SIGTERM can leak past the case boundary on
        // ubuntu-latest PTY runs and interrupt the Actions runner itself.
        #[cfg(unix)]
        {
            let pid = self.child.id();
            let _ = std::process::Command::new("kill").args(["-TERM", &pid.to_string()]).output();
        }
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("taskkill")
                .args(["/PID", &self.child.id().to_string(), "/T", "/F"])
                .output();
        }
        // Drop must never block indefinitely: if the graceful signal did not
        // land (group-kill semantics vary across platforms), force-kill the
        // child directly (guaranteed via the child handle) after a short grace
        // period, then reap. Without this fallback a server that ignored the
        // group SIGTERM would wedge the whole suite on `wait()`.
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) | Err(_) => break,
                Ok(None) if std::time::Instant::now() >= deadline => {
                    let _ = self.child.kill();
                    let _ = self.child.wait();
                    break;
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            }
        }
        // Reclaim the caches the server made (a no-op on Unix, where its own
        // handler already removed them; the only cleanup on Windows).
        for dir in &self.cache_dirs {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

/// Starts `local-npm-registry.ts --serve` for a local-registry case. The server
/// reads the fixture's `mock-manifest.json` / `tarballs/` from `stage` and the
/// packed checkout packages from `pack_dir`, then prints a one-line JSON
/// handshake (`{registry, env}`). The returned `env` (per-package-manager
/// registry settings) is injected into every step so plain `vp`/`npm`/`pnpm`/
/// `yarn`/`bun` commands resolve through it instead of the public registry.
fn start_local_registry(
    node: &Path,
    stage: &Path,
    pack_dir: &Path,
    case_env: &BTreeMap<String, OsString>,
) -> Result<(BTreeMap<String, OsString>, RegistryHandle), String> {
    use std::io::{BufRead as _, Read as _};

    let script = flavor::repo_root().join("packages/tools/src/local-npm-registry.ts");
    let mut cmd = std::process::Command::new(node);
    cmd.arg(&script)
        .arg("--serve")
        // The server discovers the fixture's own packages relative to the
        // staged workspace, so it must run from there.
        .current_dir(stage)
        .env_clear()
        .envs(case_env)
        .env("SNAP_LOCAL_VP_PACKAGES_DIR", pack_dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    // The server proxies to the developer's configured upstream (an internal
    // npm mirror in `~/.npmrc`); it must read the REAL user home to find it,
    // not the isolated case HOME from `baseline_env`, or mirror-only
    // environments fall back to registry.npmjs.org and real-dependency installs
    // fail. The case's own steps keep their isolated HOME — only this
    // upstream-resolving helper sees the real one.
    if let Some(home) = std::env::var_os("HOME") {
        cmd.env("HOME", home);
    }
    if let Some(profile) = std::env::var_os("USERPROFILE") {
        cmd.env("USERPROFILE", profile);
    }
    group_leader(&mut cmd);
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to spawn local registry (`{}`): {e}", node.display()))?;

    let stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();

    // Read the handshake on a thread (so a server that never prints it can't
    // wedge the trial), then keep draining stdout for the case's lifetime so a
    // full pipe never blocks the long-lived server.
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = std::io::BufReader::new(stdout);
        let mut line = String::new();
        let _ = tx.send(reader.read_line(&mut line).map(|_| line));
        let mut sink = String::new();
        let _ = reader.read_to_string(&mut sink);
    });

    let line = match rx.recv_timeout(Duration::from_secs(60)) {
        Ok(Ok(line)) if !line.trim().is_empty() => line,
        other => {
            drop(RegistryHandle { child, cache_dirs: Vec::new() });
            let mut err = String::new();
            let _ = stderr.read_to_string(&mut err);
            return Err(format!(
                "local registry did not start (handshake: {other:?}); stderr:\n{err}"
            ));
        }
    };

    // Wrap the child now so a handshake that parses/validates wrong tears the
    // server down too (an early `?` below would otherwise leak the process and
    // its caches into later trials).
    let mut handle = RegistryHandle { child, cache_dirs: Vec::new() };

    // Drain stderr for the case's lifetime.
    std::thread::spawn(move || {
        let mut sink = String::new();
        let _ = stderr.read_to_string(&mut sink);
    });

    let parsed: serde_json::Value = serde_json::from_str(line.trim())
        .map_err(|e| format!("could not parse local registry handshake `{}`: {e}", line.trim()))?;
    let env_obj = parsed
        .get("env")
        .and_then(serde_json::Value::as_object)
        .ok_or("local registry handshake is missing an `env` object")?;
    let mut registry_env = BTreeMap::new();
    for (key, value) in env_obj {
        if let Some(value) = value.as_str() {
            registry_env.insert(key.clone(), OsString::from(value));
        }
    }

    // Remove these per-run caches on teardown (see RegistryHandle).
    handle.cache_dirs = ["YARN_GLOBAL_FOLDER", "BUN_INSTALL_CACHE_DIR"]
        .into_iter()
        .filter_map(|key| registry_env.get(key).map(PathBuf::from))
        .collect();

    Ok((registry_env, handle))
}

#[expect(
    clippy::too_many_lines,
    reason = "test runner with process management necessarily has many lines"
)]
fn run_case(
    tmpdir: &Path,
    fixture_path: &Path,
    fixture_name: &str,
    case_index: usize,
    case: &Case,
    flavor: Flavor,
    runtime: &FlavorRuntime,
    snapshot_name: &str,
    local_registry_pack: Option<&Path>,
) -> Result<(), String> {
    let snapshots = snapshot_test::Snapshots::new(fixture_path.join("snapshots"));

    // Copy the fixture to a per-case staging directory so the test runs in
    // isolation and workspace-root discovery doesn't walk past the fixture.
    let case_root = tmpdir.join(format!("{fixture_name}_case_{case_index}_{}", flavor.as_str()));
    let stage = case_root.join("workspace");
    std::fs::create_dir_all(&stage).unwrap();
    if case.link_node_modules {
        flavor::link_dir(&tmpdir.join("node_modules"), &case_root.join("node_modules"));
    }
    // The case definition and recorded snapshots are runner metadata, not
    // part of the workspace under test, so they are never copied in.
    CopyOptions::new()
        .filter(|path, _| Ok(path != Path::new("snapshots") && path != Path::new("snapshots.toml")))
        .copy_tree(fixture_path, &stage)
        .unwrap();

    let case_home = CaseHome::provision(&case_root, case.seed_runtime);
    let case_install = case_home.provision_vite_plus(flavor, runtime)?;

    let mut case_env = baseline_env(&case_home, &case_install);
    for key in &case.unset_env {
        case_env.remove(key);
    }
    for (key, value) in &case.env {
        case_env.insert(key.clone(), value.into());
    }

    // Real tools resolve through the case's PATH (may be overridden by the
    // case's own env table).
    let case_path: OsString =
        case_env.get("PATH").cloned().unwrap_or_else(|| case_install.path_env.clone());

    // local-registry cases: stand up a per-case npm registry (serving the
    // packed checkout packages plus this fixture's mock-manifest/tarballs) and
    // fold its registry env into every step. Held to the end of the case so
    // its teardown removes the throwaway package-manager caches. Registry env
    // never sets PATH, so folding it in after `case_path` is derived is safe.
    let _registry = if case.local_registry {
        let pack_dir = local_registry_pack
            .ok_or("internal error: local-registry case reached run_case without a packed dir")?;
        // Prefer the seed runtime's real node binary over the case's node
        // shim: the shim resolves the project's pinned runtime from its cwd,
        // and a fixture pinning an older Node (a `.node-version` under test)
        // must not pick the runtime executing this TypeScript helper.
        let registry_node = flavor::seed_runtime_node()
            .map_or_else(|| case_install.resolve_program("node", &case_path, &stage), Ok)?;
        let (registry_env, handle) =
            start_local_registry(&registry_node, &stage, pack_dir, &case_env)?;
        for (key, value) in registry_env {
            case_env.insert(key, value);
        }
        Some(handle)
    } else {
        None
    };

    // Installs through the local registry are slower than pure vp commands, so
    // local-registry steps get a 120s default (still overridable per step);
    // everything else keeps the standard per-step default.
    let step_default_timeout =
        if case.local_registry { Duration::from_secs(120) } else { STEP_TIMEOUT };

    let stage_str = stage.to_str().unwrap().to_owned();
    let case_dirs = case_home.redaction_paths();
    let repo_root = flavor::repo_root();
    let repo_str = repo_root.to_str().unwrap().to_owned();
    let mut redactions = vec![(stage_str.as_str(), "<workspace>")];
    redactions.extend(case_dirs.iter().map(|(path, label)| (path.as_str(), *label)));
    redactions.push((repo_str.as_str(), "<repo>"));

    let mut doc = String::new();
    doc.push_str(&format!("# {}\n", case.name));
    if let Some(comment) = case.comment.as_deref() {
        // Normalize CRLF → LF: on Windows, git checkouts with autocrlf embed
        // `\r\n` inside TOML multi-line strings.
        let normalized = {
            use cow_utils::CowUtils as _;
            comment.cow_replace("\r\n", "\n").into_owned()
        };
        let trimmed = normalized.trim_matches('\n');
        if !trimmed.is_empty() {
            doc.push('\n');
            doc.push_str(trimmed);
            doc.push('\n');
        }
    }

    let mut timeout_error: Option<String> = None;
    let mut step_index = 0;
    while step_index < case.steps.len() {
        let step = &case.steps[step_index];
        let argv = &step.argv;
        let (step_env, step_cwd, program) =
            step_context(step, &case_env, &case_path, &stage, &case.cwd, &case_install)?;
        let step_env: &BTreeMap<String, OsString> = &step_env;
        let timeout = step.timeout(step_default_timeout);

        let (termination_state, raw_output) = if step.tty {
            'tty: {
                let mut cmd = CommandBuilder::new(&program);
                for arg in &argv[1..] {
                    cmd.arg(arg);
                }
                cmd.env_clear();
                for (k, v) in step_env {
                    cmd.env(k, v);
                }
                cmd.cwd(&step_cwd);

                // Bound the PTY spawn itself. `TestTerminal::spawn` (openpty plus
                // fork/exec of the child into the PTY) can block indefinitely on
                // some CI runners, and it runs before the interaction-phase
                // `recv_timeout` below could catch it. Run it on a helper thread so
                // a wedged spawn becomes a per-step timeout, not a suite-wide hang:
                // the helper thread is abandoned, this case fails, and the rest of
                // the suite proceeds.
                let (spawn_tx, spawn_rx) = mpsc::channel();
                std::thread::spawn(move || {
                    // If the main thread already timed out and dropped the
                    // receiver, the spawn may still complete afterwards: kill
                    // the child so a slow-but-live command can't keep running
                    // (holding a port, mutating the staged workspace) after the
                    // case has already failed.
                    if let Err(mpsc::SendError(Ok(terminal))) =
                        spawn_tx.send(TestTerminal::spawn(SCREEN_SIZE, cmd))
                    {
                        let _ = terminal.child_handle.clone().kill();
                    }
                });
                let terminal = match spawn_rx.recv_timeout(timeout) {
                    Ok(Ok(terminal)) => terminal,
                    Ok(Err(err)) => panic!("failed to spawn PTY terminal: {err}"),
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        break 'tty (
                            TerminationState::TimedOut,
                            "(PTY spawn did not complete)".to_string(),
                        );
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        panic!("PTY spawn thread panicked");
                    }
                };
                let mut killer = terminal.child_handle.clone();
                let interactions = step.interactions.clone();
                let formatted_snapshot = step.formatted_snapshot;
                let output = Arc::new(Mutex::new(String::new()));
                let output_for_thread = Arc::clone(&output);
                let (tx, rx) = mpsc::channel();
                std::thread::spawn(move || {
                    let mut terminal = terminal;

                    for interaction in interactions {
                        match interaction {
                            Interaction::ExpectMilestone(expect) => {
                                output_for_thread.lock().unwrap().push_str(&format!(
                                    "**→ expect-milestone:** `{}`\n\n",
                                    expect.expect_milestone
                                ));
                                let milestone_screen =
                                    terminal.reader.expect_milestone(&expect.expect_milestone);
                                let mut output = output_for_thread.lock().unwrap();
                                push_fenced_block(&mut output, &milestone_screen);
                                output.push('\n');
                            }
                            Interaction::Write(write) => {
                                output_for_thread
                                    .lock()
                                    .unwrap()
                                    .push_str(&format!("**← write:** `{}`\n\n", write.write));
                                terminal.writer.write_all(write.write.as_bytes()).unwrap();
                                terminal.writer.flush().unwrap();
                            }
                            Interaction::WriteLine(write_line) => {
                                output_for_thread.lock().unwrap().push_str(&format!(
                                    "**← write-line:** `{}`\n\n",
                                    write_line.write_line
                                ));
                                terminal
                                    .writer
                                    .write_line(write_line.write_line.as_bytes())
                                    .unwrap();
                            }
                            Interaction::WriteKey(write_key) => {
                                let key_name = write_key.write_key.as_str();
                                output_for_thread
                                    .lock()
                                    .unwrap()
                                    .push_str(&format!("**← write-key:** `{key_name}`\n\n"));
                                terminal.writer.write_all(write_key.write_key.bytes()).unwrap();
                                terminal.writer.flush().unwrap();
                            }
                        }
                    }

                    let status = terminal.reader.wait_for_exit().unwrap();
                    let screen = if formatted_snapshot {
                        render_formatted_screen(&terminal.reader.screen_contents_formatted())
                    } else {
                        terminal.reader.screen_contents()
                    };

                    {
                        let mut output = output_for_thread.lock().unwrap();
                        push_fenced_block(&mut output, &screen);
                    }

                    let _ = tx.send(i64::from(status.exit_code()));
                });

                match rx.recv_timeout(timeout) {
                    Ok(exit_code) => {
                        let output = output.lock().unwrap().clone();
                        (TerminationState::Exited(exit_code), output)
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        let _ = killer.kill();
                        let output = output.lock().unwrap().clone();
                        (TerminationState::TimedOut, output)
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        panic!("terminal thread panicked");
                    }
                }
            }
        } else {
            assert!(
                step.interactions.is_empty(),
                "interactions require a PTY; remove `tty = false` or the interactions"
            );
            let (state, raw) = run_step_piped(&program, &argv[1..], step_env, &step_cwd, timeout);
            let mut block = String::new();
            push_fenced_block(&mut block, &raw);
            (state, block)
        };

        // Blank line separator before every `##`.
        doc.push('\n');
        doc.push_str("## `");
        doc.push_str(&step.display_command_line(&case.cwd));
        doc.push_str("`\n\n");

        if let Some(comment) = step.comment.as_deref() {
            doc.push_str(comment);
            doc.push_str("\n\n");
        }

        // A hung command must fail the trial in both modes: a timeout can
        // never be recorded or blessed as a baseline, not even with
        // UPDATE_SNAPSHOTS=1. The error is deferred (not returned here) so
        // the case's `after` cleanup still runs first.
        if matches!(termination_state, TerminationState::TimedOut) {
            let redacted = redact_output(raw_output, &redactions, !step.formatted_snapshot);
            timeout_error = Some(format!(
                "step `{}` timed out after {timeout:?}; partial output:\n{redacted}",
                step.display_command_line(&case.cwd),
            ));
            break;
        }

        if let TerminationState::Exited(exit_code) = &termination_state {
            if *exit_code != 0 {
                doc.push_str(&format!("**Exit code:** {exit_code}\n\n"));
            }
        }

        // `snapshot = false` suppresses the screen only on success; failures
        // always keep their output for diagnosis.
        let succeeded = matches!(termination_state, TerminationState::Exited(0));
        if step.snapshot || !succeeded {
            let mut redacted = redact_output(raw_output, &redactions, !step.formatted_snapshot);
            // A version-probe step's output is a bare semver that varies by
            // environment (the managed Node's bundled npm or a
            // corepack-resolved pin); mask it. Scoped by argv so
            // fixture-controlled bare versions elsewhere (a printed
            // `.node-version` file) stay assertable.
            let version_probe = matches!(argv.first().map(String::as_str), Some("npm" | "npx"))
                && argv[1..] == ["--version"];
            if version_probe {
                redacted = redact::redact_version_probe_output(redacted);
            }
            doc.push_str(&redacted);
        }

        // Shell-like `&&` semantics with line boundaries: a failing step
        // skips the rest of its line, up to and including the next
        // continue-on-failure step, then the following line resumes.
        // Cases without markers stop here entirely.
        if !succeeded && !step.continue_on_failure {
            match case.steps[step_index + 1..].iter().position(|s| s.continue_on_failure) {
                Some(offset) => {
                    let skipped = offset + 1;
                    doc.push_str(&format!(
                        "\n*(skipped {skipped} step(s) to the next line boundary: step failed)*\n"
                    ));
                    step_index += skipped + 1;
                    continue;
                }
                None => {
                    if step_index + 1 < case.steps.len() {
                        doc.push_str("\n*(remaining steps skipped: step failed)*\n");
                    }
                    break;
                }
            }
        }

        step_index += 1;
    }

    // Cleanup steps: best-effort, never snapshotted. Per-step envs apply
    // here too: cleanup often depends on the same PATH/prefix overrides as
    // the step it tears down.
    for step in &case.after {
        let Ok((after_env, after_cwd, program)) =
            step_context(step, &case_env, &case_path, &stage, &case.cwd, &case_install)
        else {
            continue;
        };
        let mut cmd = std::process::Command::new(program);
        cmd.args(&step.argv[1..])
            .env_clear()
            .envs(after_env.as_ref())
            .current_dir(&after_cwd)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        group_leader(&mut cmd);
        // Cleanup honors the step timeout (output is discarded either way),
        // so a hung teardown can never wedge the whole suite.
        if let Ok(mut child) = cmd.spawn() {
            let _ = wait_with_deadline(&mut child, step.timeout(step_default_timeout));
        }
    }

    // Deferred so the cleanup above always runs, even for hung steps.
    if let Some(error) = timeout_error {
        return Err(error);
    }

    snapshots.check_snapshot(snapshot_name, &doc)
}

/// Global execution gate. Ordinary cases hold a shared (read) lease and run
/// concurrently; isolated cases hold the exclusive (write) lease, so nothing
/// else runs while they do. This replaces the old blanket `--test-threads=1`
/// on Linux: only the few signal-sensitive cases pay for serialization, while
/// the rest parallelize as they already do on macOS and Windows.
///
/// This coordinates threads within a single `cargo test` process, which is the
/// Linux and macOS snapshot jobs and the only place the parallel-PTY
/// signal-routing flakiness occurs. The Windows job runs the suite under
/// `cargo nextest`, which executes each trial in its own process; there the
/// gate is a no-op, but isolation is stronger for free — a signal-sensitive
/// case already has its own process, PTY, and process group, which is exactly
/// what this gate reconstructs for the shared-process case.
static EXECUTION_GATE: std::sync::RwLock<()> = std::sync::RwLock::new(());

/// Held for a case's whole run: either a shared read lease (parallel) or the
/// exclusive write lease (isolated). Poisoning is ignored — a case that
/// panicked already failed, and its neighbours should still run.
enum GateLease {
    Shared(
        #[expect(dead_code, reason = "held for its Drop")] std::sync::RwLockReadGuard<'static, ()>,
    ),
    Exclusive(
        #[expect(dead_code, reason = "held for its Drop")] std::sync::RwLockWriteGuard<'static, ()>,
    ),
}

fn acquire_gate(isolated: bool) -> GateLease {
    use std::sync::PoisonError;
    if isolated {
        GateLease::Exclusive(EXECUTION_GATE.write().unwrap_or_else(PoisonError::into_inner))
    } else {
        GateLease::Shared(EXECUTION_GATE.read().unwrap_or_else(PoisonError::into_inner))
    }
}

/// A case needs isolation if it opts in with `serial` or scripts a ctrl-c
/// keystroke. Parallel PTYs on Linux misroute signals, which is the one
/// documented flakiness source the old blanket serialization guarded against;
/// isolating exactly those cases keeps the guarantee while letting the rest
/// run in parallel.
fn case_needs_isolation(case: &Case) -> bool {
    case.serial
        || case.steps.iter().chain(&case.after).any(|step| {
            step.interactions.iter().any(|interaction| {
                matches!(
                    interaction,
                    Interaction::WriteKey(WriteKeyInteraction { write_key: WriteKey::CtrlC })
                )
            })
        })
}

fn main() {
    let tmp_dir = tempfile::tempdir().unwrap();
    // dunce, not std: std's canonicalize returns a `\\?\` verbatim path on
    // Windows, and CMD.EXE (which runs the local flavor's .cmd shims)
    // rejects verbatim/UNC working directories outright.
    let tmp_dir_path: Arc<Path> = Arc::from(dunce::canonicalize(tmp_dir.path()).unwrap());

    // Bare `vite-plus` / `@voidzero-dev/vite-plus-core` imports in fixture
    // configs resolve to the checkout packages via Node's upward walk (the
    // staged workspaces have no node_modules of their own); the linked
    // packages' own dependencies then resolve at their real location.
    // Anything else a fixture imports must be vendored inside the fixture.
    {
        let repo_root = flavor::repo_root();
        let node_modules = tmp_dir_path.join("node_modules");
        let scoped = node_modules.join("@voidzero-dev");
        std::fs::create_dir_all(&scoped).unwrap();
        flavor::link_dir(&repo_root.join("packages/cli"), &node_modules.join("vite-plus"));
        flavor::link_dir(&repo_root.join("packages/core"), &scoped.join("vite-plus-core"));
        // `vite` resolves to the core package, matching the vite -> core
        // override installed in migrated projects.
        flavor::link_dir(&repo_root.join("packages/core"), &node_modules.join("vite"));
    }

    let fixtures_dir = flavor::manifest_dir().join("tests/cli_snapshots/fixtures");

    let mut fixture_paths = std::fs::read_dir(&fixtures_dir)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", fixtures_dir.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|p| {
            p.is_dir()
                && p.file_name().and_then(|n| n.to_str()).is_some_and(|n| !n.starts_with('.'))
        })
        .collect::<Vec<_>>();
    fixture_paths.sort();

    // Cases run in parallel on every platform. Signal-sensitive cases (ctrl-c,
    // or `serial = true`) instead take the exclusive execution lease so they
    // run in isolation; see `EXECUTION_GATE`. This replaces the old
    // Linux-wide `--test-threads=1`, which serialized the entire suite to
    // protect a handful of ctrl-c cases.
    let args = libtest_mimic::Arguments::from_args();

    // `VP_SNAP_SKIP_FLAVORS=local` (comma-separated) skips registering trials
    // for a flavor entirely; CI legs that don't build the JS CLI use it.
    let skip_flavors: Vec<String> = std::env::var("VP_SNAP_SKIP_FLAVORS")
        .map(|v| v.split(',').map(|s| s.trim().to_owned()).collect())
        .unwrap_or_default();

    // Provisioning is lazy and per-flavor: list phases and filtered runs
    // provision nothing, and nextest's one-trial-per-process model only pays
    // for the flavor that trial actually uses. Individual trials surface the
    // error message when their flavor is unavailable.
    struct LazyRuntimes {
        run_root: Arc<Path>,
        global: std::sync::OnceLock<Result<FlavorRuntime, String>>,
        local: std::sync::OnceLock<Result<FlavorRuntime, String>>,
        /// Packed checkout packages (vite-plus, @voidzero-dev/vite-plus-core),
        /// produced once on the first local-registry case and shared by every
        /// per-case registry via `SNAP_LOCAL_VP_PACKAGES_DIR`.
        local_registry_pack: std::sync::OnceLock<Result<Arc<Path>, String>>,
    }
    impl LazyRuntimes {
        fn get(&self, flavor: Flavor) -> &Result<FlavorRuntime, String> {
            let cell = match flavor {
                Flavor::Global => &self.global,
                Flavor::Local => &self.local,
            };
            cell.get_or_init(|| flavor::provision(flavor, &self.run_root))
        }

        /// Packs the checkout packages once and returns the shared dir. Reuses
        /// `local-npm-registry.ts --pack-to` so the pack logic lives in one
        /// place (the same helper the tool and ecosystem-ci use).
        fn local_registry_pack(&self) -> Result<Arc<Path>, String> {
            self.local_registry_pack
                .get_or_init(|| {
                    let node = which::which("node")
                        .map_err(|e| format!("`node` not found on PATH (needed to pack): {e}"))?;
                    let repo_root = flavor::repo_root();
                    let script = repo_root.join("packages/tools/src/local-npm-registry.ts");
                    let dest = self.run_root.join("local-registry-packages");
                    std::fs::create_dir_all(&dest)
                        .map_err(|e| format!("failed to create pack dir: {e}"))?;
                    // Inherit the runner's environment so the packer finds
                    // `pnpm` and `node` the same way a developer would.
                    let output = std::process::Command::new(&node)
                        .arg(&script)
                        .arg("--pack-to")
                        .arg(&dest)
                        .current_dir(&repo_root)
                        .output()
                        .map_err(|e| format!("failed to run local-registry pack: {e}"))?;
                    if !output.status.success() {
                        return Err(format!(
                            "packing checkout packages failed:\n{}",
                            String::from_utf8_lossy(&output.stderr)
                        ));
                    }
                    Ok(Arc::from(dest.as_path()))
                })
                .clone()
        }
    }
    let runtimes = Arc::new(LazyRuntimes {
        run_root: Arc::clone(&tmp_dir_path),
        global: std::sync::OnceLock::new(),
        local: std::sync::OnceLock::new(),
        local_registry_pack: std::sync::OnceLock::new(),
    });

    // Per-case wall times, printed slowest-first after the run. libtest-mimic
    // (0.8) has no `--report-time`, so the runner records timings itself; the
    // summary shows every case's cost and makes a slow or stuck case obvious.
    let timings: Arc<Mutex<Vec<(String, Duration)>>> = Arc::new(Mutex::new(Vec::new()));

    // local-registry cases pack the checkout, which needs the built JS packages
    // (`packages/cli/dist`). On a Rust-only checkout, such as
    // `snapshot-test-global` without a prior `pnpm build`, that build is absent,
    // so ignore those cases instead of failing the run; the full-build legs
    // cover them.
    let local_build_present = flavor::repo_root().join("packages/cli/dist/bin.js").is_file();

    let mut tests: Vec<libtest_mimic::Trial> = Vec::new();
    for fixture_path in fixture_paths {
        let fixture_path: Arc<Path> = Arc::from(fixture_path.as_path());
        let fixture_name: Arc<str> = Arc::from(fixture_path.file_name().unwrap().to_str().unwrap());
        assert_identifier_like("fixture folder", &fixture_name);
        let cases_file = load_snapshots_file(&fixture_path);
        for (case_index, case) in cases_file.cases.into_iter().enumerate() {
            assert_identifier_like("case name", &case.name);
            if case.skip_platforms.iter().any(PlatformFilter::matches_current) {
                continue;
            }
            let multi = case.vp.is_multi();
            let case = Arc::new(case);
            for flavor in case.vp.flavors() {
                if skip_flavors.iter().any(|s| s == flavor.as_str()) {
                    continue;
                }
                let trial_name = if multi {
                    format!("{fixture_name}::{}::{}", case.name, flavor.as_str())
                } else {
                    format!("{fixture_name}::{}", case.name)
                };
                let snapshot_name = if multi {
                    format!("{}.{}.md", case.name, flavor.as_str())
                } else {
                    format!("{}.md", case.name)
                };
                let runtimes = Arc::clone(&runtimes);
                let fixture_path = Arc::clone(&fixture_path);
                let fixture_name = Arc::clone(&fixture_name);
                let tmp_dir_path = Arc::clone(&tmp_dir_path);
                let case = Arc::clone(&case);
                let ignored = case.ignore || (case.local_registry && !local_build_present);
                let isolated = case_needs_isolation(&case);
                let timings = Arc::clone(&timings);
                let timing_name = trial_name.clone();
                tests.push(
                    libtest_mimic::Trial::test(trial_name, move || {
                        // Hold the execution lease for the whole case: shared
                        // (parallel) unless the case needs isolation. Acquired
                        // before timing so the reported duration is the case's
                        // own work, not time spent waiting for the lease.
                        let _gate = acquire_gate(isolated);
                        let started = std::time::Instant::now();
                        let result = (|| -> Result<(), libtest_mimic::Failed> {
                            let runtime = match runtimes.get(flavor) {
                                Ok(runtime) => runtime,
                                Err(message) => return Err(message.clone().into()),
                            };
                            // Pack the checkout once, lazily, only when a
                            // local-registry case actually runs.
                            let local_registry_pack = if case.local_registry {
                                match runtimes.local_registry_pack() {
                                    Ok(dir) => Some(dir),
                                    Err(message) => return Err(message.into()),
                                }
                            } else {
                                None
                            };
                            run_case(
                                &tmp_dir_path,
                                &fixture_path,
                                &fixture_name,
                                case_index,
                                &case,
                                flavor,
                                runtime,
                                &snapshot_name,
                                local_registry_pack.as_deref(),
                            )
                            .map_err(Into::into)
                        })();
                        timings.lock().unwrap().push((timing_name, started.elapsed()));
                        result
                    })
                    .with_ignored_flag(ignored),
                );
            }
        }
    }

    let conclusion = libtest_mimic::run(&args, tests);

    // Report each case's wall time (slowest first). Skipped for `--list`,
    // which runs nothing.
    if !args.list {
        let mut recorded = std::mem::take(&mut *timings.lock().unwrap());
        if !recorded.is_empty() {
            recorded.sort_by_key(|(_, dur)| std::cmp::Reverse(*dur));
            eprintln!("\nsnapshot case timings (slowest first):");
            for (name, dur) in &recorded {
                eprintln!("  {:>7.2}s  {name}", dur.as_secs_f64());
            }
        }
    }

    // exit() never returns, so the staged run tree must be dropped first or
    // every run would leave its full tempdir behind.
    drop(tmp_dir);
    conclusion.exit();
}
